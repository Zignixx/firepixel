//! Vor- und Nachverarbeitung: Bild (RGBA, Dokumentkoordinaten) → Modell-Eingabe → Maske in
//! Dokumentkoordinaten.
//!
//! * Eingabe: Ausschnitt `region` des Bildes, über Weiß gelegt, mit Flächenfilter (wie PIL
//!   „bilinear“ mit Antialiasing) auf die Modellgröße skaliert und ImageNet-normalisiert.
//! * Ausgabe: Logits bzw. Alpha bilinear auf die Ausschnittgröße, mal Deckkraft der Quelle, als
//!   [`Mask`] mit den Grenzen des Ausschnitts.

use crate::cache::{self, Loaded};
use crate::cancel::{Progress, Stage};
use crate::error::{AiError, Result};
use crate::models::{self, ModelId};
use crate::nn;
use crate::sam2::{self, Prompt};
use crate::tuning::{self, Tuning, Variant};
use fp_geom::IRect;
use fp_raster::{Mask, RgbaImage};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

const MEAN: [f32; 3] = [0.485, 0.456, 0.406];
const STD: [f32; 3] = [0.229, 0.224, 0.225];

/// Ein Klickpunkt in Dokumentkoordinaten.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PromptPoint {
    pub x: f32,
    pub y: f32,
    /// `true` = gehört zum Objekt, `false` = Hintergrund.
    pub positive: bool,
}

/// Eingabe der Objektauswahl (Dokumentkoordinaten).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ObjectPrompt {
    pub points: Vec<PromptPoint>,
    /// Rahmen `[l, t, r, b]`.
    pub boxx: Option<[f32; 4]>,
}

impl ObjectPrompt {
    /// Punkt hinzufügen; ein (fast) gleicher Punkt mit gleichem Label wird nicht doppelt
    /// aufgenommen, mit anderem Label ersetzt. Liefert `true`, wenn sich etwas geändert hat.
    pub fn add_point(&mut self, p: PromptPoint) -> bool {
        if let Some(q) = self
            .points
            .iter_mut()
            .find(|q| (q.x - p.x).abs() < 1.0 && (q.y - p.y).abs() < 1.0)
        {
            if q.positive == p.positive {
                return false;
            }
            *q = p;
            return true;
        }
        self.points.push(p);
        true
    }
}

/// Ergebnis der Objektauswahl.
#[derive(Clone, Debug)]
pub struct ObjectResult {
    pub mask: Mask,
    /// Vorhergesagte Güte 0…1.
    pub iou: f32,
    /// Objekt-Logit (< 0: vermutlich kein Objekt unter dem Punkt).
    pub object_score: f32,
    /// War die Bild-Einbettung schon im Speicher?
    pub cached: bool,
}

/// Skaliert eine Ebene `[h, w]` mit Dreiecksfilter (Antialiasing beim Verkleinern).
pub fn resample_plane(src: &[f32], w: usize, h: usize, dw: usize, dh: usize) -> Vec<f32> {
    let taps = |n_in: usize, n_out: usize| -> Vec<(usize, Vec<f32>)> {
        let scale = n_in as f32 / n_out as f32;
        let support = scale.max(1.0);
        (0..n_out)
            .map(|o| {
                let center = (o as f32 + 0.5) * scale;
                let lo = ((center - support).floor().max(0.0)) as usize;
                let hi = ((center + support).ceil() as usize).min(n_in);
                let mut ws: Vec<f32> = (lo..hi)
                    .map(|i| (1.0 - ((i as f32 + 0.5 - center) / support).abs()).max(0.0))
                    .collect();
                let s: f32 = ws.iter().sum();
                if s > 0.0 {
                    for x in &mut ws {
                        *x /= s;
                    }
                } else if !ws.is_empty() {
                    ws[0] = 1.0;
                }
                (lo, ws)
            })
            .collect()
    };
    let tx = taps(w, dw);
    let ty = taps(h, dh);
    // horizontal
    let mut tmp = vec![0.0f32; h * dw];
    tmp.par_chunks_mut(dw).enumerate().for_each(|(y, row)| {
        let s = &src[y * w..(y + 1) * w];
        for (o, (lo, ws)) in tx.iter().enumerate() {
            row[o] = ws.iter().enumerate().map(|(i, k)| s[lo + i] * k).sum();
        }
    });
    let mut out = vec![0.0f32; dh * dw];
    out.par_chunks_mut(dw).enumerate().for_each(|(o, row)| {
        let (lo, ws) = &ty[o];
        for (i, k) in ws.iter().enumerate() {
            let r = &tmp[(lo + i) * dw..(lo + i + 1) * dw];
            for (a, b) in row.iter_mut().zip(r) {
                *a += *b * k;
            }
        }
    });
    out
}

/// Gültiger Ausschnitt: `region` ∩ Bildgrenzen (ohne `region` das ganze Bild).
pub fn clip_region(img: &RgbaImage, region: Option<IRect>) -> Result<IRect> {
    let b = img.bounds();
    let r = match region {
        Some(r) => r.intersect(&b),
        None => b,
    };
    if r.is_empty() || r.width() < 2 || r.height() < 2 {
        return Err(AiError::Input(fp_i18n::t("ai-err-d-empty-region")));
    }
    Ok(r)
}

/// Modell-Eingabe `[3, s, s]` aus dem Ausschnitt `r` (über Weiß, normalisiert).
pub fn model_input(img: &RgbaImage, r: IRect, s: usize) -> Vec<f32> {
    let (w, h) = (r.width() as usize, r.height() as usize);
    let mut planes = vec![vec![0.0f32; w * h]; 3];
    let ib = img.bounds();
    let iw = img.width();
    let data = img.data();
    for y in 0..h {
        let sy = (r.top - ib.top) as usize + y;
        for x in 0..w {
            let sx = (r.left - ib.left) as usize + x;
            let px = data[sy * iw + sx];
            let a = px[3] as f32 / 255.0;
            for c in 0..3 {
                planes[c][y * w + x] = (px[c] as f32 / 255.0) * a + (1.0 - a);
            }
        }
    }
    let mut out = Vec::with_capacity(3 * s * s);
    for (c, p) in planes.iter().enumerate() {
        let mut r = resample_plane(p, w, h, s, s);
        for v in &mut r {
            *v = (*v - MEAN[c]) / STD[c];
        }
        out.extend(r);
    }
    out
}

/// Alpha `[s, s]` (0…1) → Maske des Ausschnitts, mal Deckkraft der Quelle.
pub fn alpha_to_mask(alpha: &[f32], s: usize, img: &RgbaImage, r: IRect) -> Result<Mask> {
    values_to_mask(alpha, s, img, r, |v| v)
}

/// SAM-Logits `[s, s]` → Maske: erst bilinear auf die Ausschnittgröße (wie das Original), dann
/// Schwelle 0 mit 2 Logit-Einheiten Übergang (geglättete Kante ohne Schleier um das Objekt).
pub fn logits_to_mask(logits: &[f32], s: usize, img: &RgbaImage, r: IRect) -> Result<Mask> {
    logits_to_mask_at(logits, s, img, r, 0.0)
}

/// Wie [`logits_to_mask`] mit eigener Schwelle (`threshold` in Logit-Einheiten, 0 = Vorgabe).
pub fn logits_to_mask_at(
    logits: &[f32],
    s: usize,
    img: &RgbaImage,
    r: IRect,
    threshold: f32,
) -> Result<Mask> {
    values_to_mask(logits, s, img, r, |l| (l - threshold + 1.0) * 0.5)
}

fn values_to_mask(
    vals: &[f32],
    s: usize,
    img: &RgbaImage,
    r: IRect,
    map: impl Fn(f32) -> f32 + Sync,
) -> Result<Mask> {
    let (w, h) = (r.width() as usize, r.height() as usize);
    let up = resample_plane(vals, s, s, w, h);
    let ib = img.bounds();
    let iw = img.width();
    let data = img.data();
    let mut m = Mask::new(r).map_err(|e| AiError::Input(e.to_string()))?;
    let md = m.data_mut();
    md.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
        let sy = (r.top - ib.top) as usize + y;
        for (x, v) in row.iter_mut().enumerate() {
            let sx = (r.left - ib.left) as usize + x;
            let a = data[sy * iw + sx][3] as f32 / 255.0;
            let val = (map(up[y * w + x]).clamp(0.0, 1.0) * a * 255.0).round();
            *v = if val < 2.0 { 0 } else { val as u8 };
        }
    });
    Ok(m)
}

/// Schlüssel eines Bildausschnitts für den Zwischenspeicher.
pub fn image_key(img: &RgbaImage, r: IRect, model: ModelId) -> u64 {
    cache::hash_bytes(
        img.as_bytes(),
        &[
            r.left as u64,
            r.top as u64,
            r.right as u64,
            r.bottom as u64,
            img.bounds().left as u64,
            img.bounds().top as u64,
            img.width() as u64,
            model as u64,
        ],
    )
}

/// Modell aus dem Zwischenspeicher bzw. von der Platte; meldet die Stufe „Modell laden“, wenn es
/// wirklich geladen werden muss, danach „Bild analysieren“.
fn load_model(id: ModelId, prog: &Progress) -> Result<std::sync::Arc<Loaded>> {
    if !cache::is_loaded(id) {
        prog.set_stage(Stage::LoadModel);
    }
    let m = cache::model(id)?;
    prog.cancel.check()?;
    prog.set_stage(Stage::Analyze);
    Ok(m)
}

fn sam_model(id: ModelId) -> Result<std::sync::Arc<Loaded>> {
    let m = cache::model(id)?;
    if !matches!(*m, Loaded::Sam2(_)) {
        return Err(AiError::Input(fp_i18n::t("ai-err-d-no-sam")));
    }
    Ok(m)
}

/// Ist die Einbettung dieses Ausschnitts schon berechnet (Klick reagiert sofort)?
pub fn object_ready(img: &RgbaImage, region: Option<IRect>, model: ModelId) -> bool {
    clip_region(img, region)
        .map(|r| cache::embedding_cached(image_key(img, r, model)).is_some())
        .unwrap_or(false)
}

/// Berechnet nur die Bild-Einbettung (Vorwärmen nach dem ersten Klick bzw. beim Werkzeugwechsel).
pub fn object_prepare(
    img: &RgbaImage,
    region: Option<IRect>,
    model: ModelId,
    prog: &Progress,
) -> Result<bool> {
    let r = clip_region(img, region)?;
    let key = image_key(img, r, model);
    if cache::embedding_cached(key).is_some() {
        return Ok(true);
    }
    let m = load_model(model, prog)?;
    let Loaded::Sam2(sam) = &*m else {
        return Err(AiError::Input(fp_i18n::t("ai-err-d-no-sam")));
    };
    nn::pool().install(|| {
        cache::embedding(key, || {
            let x = model_input(img, r, sam2::SIZE);
            sam.encode(&x, prog)
        })
    })?;
    Ok(false)
}

/// Rohausgabe der Objektauswahl: alle vier Masken-Logits des Decoders für einen Ausschnitt. Aus
/// ihr lassen sich Toleranz, Maskenvariante und Kante ohne neues Modell-Laufen ändern
/// ([`object_from_raw`]).
#[derive(Clone, Debug)]
pub struct ObjectRaw {
    /// `[4, LOW, LOW]` Logits (0 = Einzelmaske, 1…3 = Mehrfachmasken).
    pub masks: std::sync::Arc<Vec<f32>>,
    /// Güte je Maske.
    pub ious: [f32; 4],
    /// Wahl des Modells (Variante „Automatisch“).
    pub pick: usize,
    /// Objekt-Logit.
    pub object_score: f32,
    /// Ausschnitt in Dokumentkoordinaten.
    pub region: IRect,
    /// War die Bild-Einbettung schon im Speicher?
    pub cached: bool,
}

impl ObjectRaw {
    fn plane(&self, i: usize) -> &[f32] {
        let n = sam2::LOW * sam2::LOW;
        &self.masks[i * n..(i + 1) * n]
    }

    /// Index der Maske für eine Variante: Klein/Mittel/Groß = Mehrfachmasken 1…3 nach Fläche
    /// (Logit > 0).
    pub fn index_for(&self, v: Variant) -> usize {
        let mut multi: Vec<(usize, usize)> = (1..4)
            .map(|i| (self.plane(i).iter().filter(|l| **l > 0.0).count(), i))
            .collect();
        multi.sort();
        match v {
            Variant::Auto => self.pick,
            Variant::Small => multi[0].1,
            Variant::Medium => multi[1].1,
            Variant::Large => multi[2].1,
        }
    }
}

/// Objektauswahl bis zur Rohausgabe (Encoder aus dem Zwischenspeicher, dann der Decoder).
pub fn object_raw(
    img: &RgbaImage,
    region: Option<IRect>,
    prompt: &ObjectPrompt,
    model: ModelId,
    prog: &Progress,
) -> Result<ObjectRaw> {
    if prompt.points.is_empty() && prompt.boxx.is_none() {
        return Err(AiError::Input(fp_i18n::t("ai-err-d-point-or-box")));
    }
    let r = clip_region(img, region)?;
    let key = image_key(img, r, model);
    let cached = cache::embedding_cached(key).is_some();
    let m = if cached {
        sam_model(model)?
    } else {
        load_model(model, prog)?
    };
    let Loaded::Sam2(sam) = &*m else {
        return Err(AiError::Input(fp_i18n::t("ai-err-d-no-sam")));
    };
    let (w, h) = (r.width() as f32, r.height() as f32);
    let sx = sam2::SIZE as f32 / w;
    let sy = sam2::SIZE as f32 / h;
    let to_model = |x: f32, y: f32| ((x - r.left as f32) * sx, (y - r.top as f32) * sy);
    nn::pool().install(|| {
        let emb = cache::embedding(key, || {
            let x = model_input(img, r, sam2::SIZE);
            sam.encode(&x, prog)
        })?;
        prog.set_stage(Stage::Mask);
        let mut p = Prompt {
            multimask: prompt.points.len() == 1 && prompt.boxx.is_none(),
            ..Prompt::default()
        };
        for pt in &prompt.points {
            let (x, y) = to_model(pt.x, pt.y);
            p.points.push((x, y, if pt.positive { 1 } else { 0 }));
        }
        if let Some(b) = prompt.boxx {
            let (x0, y0) = to_model(b[0].min(b[2]), b[1].min(b[3]));
            let (x1, y1) = to_model(b[0].max(b[2]), b[1].max(b[3]));
            p.boxx = Some([x0, y0, x1, y1]);
        }
        let d = sam.decode(&emb, &p)?;
        Ok(ObjectRaw {
            masks: std::sync::Arc::new(d.all),
            ious: d.ious,
            pick: d.pick,
            object_score: d.object_score,
            region: r,
            cached,
        })
    })
}

/// Maske aus der Rohausgabe mit Feineinstellungen (schnell, kein Modell).
pub fn object_from_raw(raw: &ObjectRaw, img: &RgbaImage, t: &Tuning) -> Result<ObjectResult> {
    let t = t.clamped();
    let i = raw.index_for(t.variant);
    let mask = nn::pool().install(|| {
        logits_to_mask_at(raw.plane(i), sam2::LOW, img, raw.region, t.sam_threshold())
    })?;
    let mask = tuning::postprocess(mask, &t)?;
    Ok(ObjectResult {
        mask,
        iou: raw.ious[i],
        object_score: raw.object_score,
        cached: raw.cached,
    })
}

/// Objektauswahl: Punkte/Rahmen (Dokumentkoordinaten) → Maske des Ausschnitts (Vorgabe-Einstellungen).
pub fn object_mask(
    img: &RgbaImage,
    region: Option<IRect>,
    prompt: &ObjectPrompt,
    model: ModelId,
    prog: &Progress,
) -> Result<ObjectResult> {
    let raw = object_raw(img, region, prompt, model, prog)?;
    let i = raw.pick;
    let mask = nn::pool().install(|| logits_to_mask(raw.plane(i), sam2::LOW, img, raw.region))?;
    Ok(ObjectResult {
        mask,
        iou: raw.ious[i],
        object_score: raw.object_score,
        cached: raw.cached,
    })
}

fn alpha_key(img: &RgbaImage, r: IRect, model: ModelId, size: usize) -> u64 {
    image_key(img, r, model) ^ (size as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)
}

fn birefnet_alpha(
    img: &RgbaImage,
    r: IRect,
    model: ModelId,
    size: usize,
    prog: &Progress,
) -> Result<std::sync::Arc<Vec<f32>>> {
    let key = alpha_key(img, r, model, size);
    if let Some(a) = cache::alpha_cached(key) {
        return Ok(a);
    }
    let m = load_model(model, prog)?;
    let Loaded::BiRefNet(net) = &*m else {
        return Err(AiError::Input(fp_i18n::t("ai-err-d-no-birefnet")));
    };
    nn::pool().install(|| {
        cache::alpha(key, || {
            let x = model_input(img, r, size);
            let mut out = net.forward(&x, size, prog)?;
            nn::map_inplace(&mut out, nn::sigmoid);
            Ok(out)
        })
    })
}

/// Modellauflösung einer BiRefNet-Variante. HR-Matting ist für 2048 trainiert, läuft hier aber mit
/// 1024 (gemessen: 2048 → 35 s, 1024 → 11 s auf 24 Kernen; Haarsträhnen bei 1024 nur wenig weicher).
pub fn birefnet_size(model: ModelId) -> usize {
    let _ = model;
    1024
}

/// Ist das Motiv dieses Ausschnitts schon berechnet?
pub fn subject_ready(img: &RgbaImage, region: Option<IRect>, model: ModelId) -> bool {
    clip_region(img, region)
        .map(|r| cache::alpha_cached(alpha_key(img, r, model, birefnet_size(model))).is_some())
        .unwrap_or(false)
}

/// Rohausgabe von „Motiv auswählen“: weiche Maske des Netzes (`size × size`, 0…1) für einen
/// Ausschnitt; Toleranz und Kante lassen sich daraus ohne neues Modell-Laufen ändern.
#[derive(Clone, Debug)]
pub struct SubjectRaw {
    /// Weiche Maske `[size, size]`.
    pub alpha: std::sync::Arc<Vec<f32>>,
    /// Modellauflösung.
    pub size: usize,
    /// Ausschnitt in Dokumentkoordinaten.
    pub region: IRect,
}

/// Motiv bis zur Rohausgabe.
pub fn subject_raw(
    img: &RgbaImage,
    region: Option<IRect>,
    model: ModelId,
    prog: &Progress,
) -> Result<SubjectRaw> {
    let r = clip_region(img, region)?;
    let s = birefnet_size(model);
    let alpha = birefnet_alpha(img, r, model, s, prog)?;
    Ok(SubjectRaw {
        alpha,
        size: s,
        region: r,
    })
}

/// Motiv-Maske aus der Rohausgabe mit Feineinstellungen (eine Maskenvariante gibt es hier nicht).
pub fn subject_from_raw(raw: &SubjectRaw, img: &RgbaImage, t: &Tuning) -> Result<Mask> {
    let t = t.clamped();
    let mask = nn::pool()
        .install(|| values_to_mask(&raw.alpha, raw.size, img, raw.region, |a| t.soft_level(a)))?;
    tuning::postprocess(mask, &t)
}

/// Motiv auswählen: weiche Maske des Hauptmotivs im Ausschnitt (Vorgabe-Einstellungen).
pub fn subject_mask(
    img: &RgbaImage,
    region: Option<IRect>,
    model: ModelId,
    prog: &Progress,
) -> Result<Mask> {
    let raw = subject_raw(img, region, model, prog)?;
    prog.set_stage(Stage::Mask);
    alpha_to_mask(&raw.alpha, raw.size, img, raw.region)
}

/// Exakte euklidische Distanztransformation (Felzenszwalb/Huttenlocher): Abstand jedes Pixels
/// zum nächsten Pixel mit `feature = true` (Quadrat der Distanz).
fn edt_sq(feature: &[bool], w: usize, h: usize) -> Vec<f32> {
    const INF: f32 = 1e20;
    let dt1 = |f: &[f32], n: usize| -> Vec<f32> {
        if n == 0 {
            return Vec::new();
        }
        let f64v: Vec<f64> = f.iter().map(|x| *x as f64).collect();
        let mut d = vec![0.0f32; n];
        let mut v = vec![0usize; n];
        let mut z = vec![0.0f64; n + 1];
        let mut k = 0usize;
        z[0] = f64::NEG_INFINITY;
        z[1] = f64::INFINITY;
        let inter = |q: usize, p: usize| {
            ((f64v[q] + (q * q) as f64) - (f64v[p] + (p * p) as f64))
                / (2.0 * q as f64 - 2.0 * p as f64)
        };
        for q in 1..n {
            let mut s = inter(q, v[k]);
            while s <= z[k] {
                k -= 1;
                s = inter(q, v[k]);
            }
            k += 1;
            v[k] = q;
            z[k] = s;
            z[k + 1] = f64::INFINITY;
        }
        k = 0;
        for (q, dq) in d.iter_mut().enumerate() {
            while z[k + 1] < q as f64 {
                k += 1;
            }
            let dd = q as f64 - v[k] as f64;
            *dq = (dd * dd + f64v[v[k]]) as f32;
        }
        d
    };
    let mut g = vec![0.0f32; w * h];
    // Spalten
    let cols: Vec<Vec<f32>> = (0..w)
        .into_par_iter()
        .map(|x| {
            let f: Vec<f32> = (0..h)
                .map(|y| if feature[y * w + x] { 0.0 } else { INF })
                .collect();
            dt1(&f, h)
        })
        .collect();
    for (x, c) in cols.iter().enumerate() {
        for y in 0..h {
            g[y * w + x] = c[y];
        }
    }
    g.par_chunks_mut(w).for_each(|row| {
        let r = dt1(row, w);
        row.copy_from_slice(&r);
    });
    g
}

/// Zuschnitt und wirksamer Radius für „Kante verfeinern“.
fn refine_region(img: &RgbaImage, sel: &Mask, radius: f32) -> Result<(IRect, f32)> {
    let tb = sel.tight_bounds();
    if tb.is_empty() {
        return Err(AiError::Input(fp_i18n::t("ai-err-d-no-pixel-selection")));
    }
    let diag = ((tb.width() as f32).powi(2) + (tb.height() as f32).powi(2)).sqrt();
    let radius = if radius > 0.0 {
        radius
    } else {
        (diag * 0.025).clamp(4.0, 48.0)
    };
    let margin = (radius * 2.0).max(diag * 0.08).ceil() as i32 + 4;
    Ok((clip_region(img, Some(tb.inflate(margin, margin)))?, radius))
}

/// Ist das Matting für diese Auswahl schon berechnet?
pub fn refine_ready(img: &RgbaImage, sel: &Mask, radius: f32, model: ModelId) -> bool {
    refine_region(img, sel, radius)
        .map(|(r, _)| cache::alpha_cached(alpha_key(img, r, model, birefnet_size(model))).is_some())
        .unwrap_or(false)
}

/// Kante verfeinern: Matting im Bereich um die Auswahlkante.
///
/// `radius` = Breite des Kantenbereichs in Pixeln (0 = automatisch aus der Auswahlgröße). Innen
/// und außen bleibt die bisherige Auswahl, im Kantenbereich gilt das Alpha des Matting-Modells,
/// am Rand des Bereichs weich übergeblendet.
pub fn refine_edge(
    img: &RgbaImage,
    sel: &Mask,
    radius: f32,
    model: ModelId,
    prog: &Progress,
) -> Result<Mask> {
    let (r, radius) = refine_region(img, sel, radius)?;
    let s = birefnet_size(model);
    let a = birefnet_alpha(img, r, model, s, prog)?;
    prog.set_stage(Stage::Mask);
    let (w, h) = (r.width() as usize, r.height() as usize);
    let am = resample_plane(&a, s, s, w, h);
    // Auswahl im Ausschnitt
    let mut sv = vec![0u8; w * h];
    for y in 0..h {
        for x in 0..w {
            sv[y * w + x] = sel.get(r.left + x as i32, r.top + y as i32);
        }
    }
    let inside: Vec<bool> = sv.iter().map(|v| *v >= 128).collect();
    let outside: Vec<bool> = inside.iter().map(|b| !b).collect();
    let d_to_in = edt_sq(&inside, w, h);
    let d_to_out = edt_sq(&outside, w, h);
    let ib = img.bounds();
    let iw = img.width();
    let data = img.data();
    let fade = (radius * 0.35).max(1.5);
    let mut out = sel
        .copy_rect(sel.bounds().union(&r))
        .map_err(|e| AiError::Input(e.to_string()))?;
    let ob = out.bounds();
    let ow = out.width();
    let od = out.data_mut();
    for y in 0..h {
        let sy = (r.top - ib.top) as usize + y;
        for x in 0..w {
            let i = y * w + x;
            let d = if inside[i] {
                d_to_out[i].sqrt()
            } else {
                d_to_in[i].sqrt()
            };
            let t = ((radius - d) / fade).clamp(0.0, 1.0);
            let t = t * t * (3.0 - 2.0 * t);
            let src_a = data[sy * iw + (r.left - ib.left) as usize + x][3] as f32 / 255.0;
            let v = sv[i] as f32 * (1.0 - t) + am[i].clamp(0.0, 1.0) * 255.0 * src_a * t;
            let o = (r.top - ob.top) as usize + y;
            od[o * ow + (r.left - ob.left) as usize + x] = v.round().clamp(0.0, 255.0) as u8;
        }
    }
    let _ = models::info(model);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resample_identity_and_mean() {
        let src: Vec<f32> = (0..16).map(|i| i as f32).collect();
        assert_eq!(resample_plane(&src, 4, 4, 4, 4), src);
        // Dreiecksfilter mit doppelter Breite wie PIL: Gewichte 0,75/0,75/0,25 → 3,571
        let half = resample_plane(&src, 4, 4, 2, 2);
        assert!((half[0] - 25.0 / 7.0).abs() < 1e-4, "{}", half[0]);
        let flat = vec![3.0f32; 100];
        assert!(
            resample_plane(&flat, 10, 10, 37, 23)
                .iter()
                .all(|v| (v - 3.0).abs() < 1e-5)
        );
    }

    #[test]
    fn edt_exact() {
        let (w, h) = (7, 5);
        let mut f = vec![false; w * h];
        f[2 * w + 3] = true;
        let d = edt_sq(&f, w, h);
        for y in 0..h {
            for x in 0..w {
                let e = (x as f32 - 3.0).powi(2) + (y as f32 - 2.0).powi(2);
                assert!((d[y * w + x] - e).abs() < 1e-4, "{x},{y}");
            }
        }
    }

    #[test]
    fn prompt_dedup() {
        let mut p = ObjectPrompt::default();
        assert!(p.add_point(PromptPoint {
            x: 10.0,
            y: 10.0,
            positive: true
        }));
        assert!(!p.add_point(PromptPoint {
            x: 10.2,
            y: 9.9,
            positive: true
        }));
        assert!(p.add_point(PromptPoint {
            x: 10.0,
            y: 10.0,
            positive: false
        }));
        assert_eq!(p.points.len(), 1);
        assert!(!p.points[0].positive);
    }
}
