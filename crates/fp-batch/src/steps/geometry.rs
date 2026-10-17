//! Gruppe „Geometrie“ (SPEC-BATCH §3.3, Paket BATCH-4): `resize`, `crop` (ohne smart), `trim`,
//! `rotate` (mit EXIF-Ausrichtung) und `canvas`.
//!
//! Gemeinsame Regeln aller Schritte dieser Datei (auch von `adjust.rs` benutzt):
//! - Wirken auf Einzelbilder und **jedes** Frame einer Animation gleich (gleiche Zielgröße, gleicher
//!   Ausschnitt); Bildrate, Verzögerungen und Schleifen bleiben unverändert. Dokumente werden vorher
//!   flach gerechnet (wie [`Item::map_images`]).
//! - **Vorschau**: Die Pipeline rechnet auf einem verkleinerten Proxy. Pixelwerte (Breite, Rand,
//!   Ausschnitt, Polster, Radien) werden deshalb mit dem Proxy-Maßstab umgerechnet
//!   ([`preview_scale`], gemerkt in `meta.extra["previewScale"]`), damit die Vorschau wie das
//!   Ergebnis in voller Größe aussieht.
//! - Deterministisch: zeilenweise parallel (rayon), jede Zeile unabhängig → bitgleich zur
//!   Einzel-Thread-Rechnung.
//! - Keine Panik bei 0×0, riesigen Zielgrößen (> 2²⁸ Pixel → Fehler „zu groß“) oder leeren Bildern.

use crate::error::{BatchError, Result};
use crate::executor::CancelToken;
use crate::item::{Item, ItemKind};
use crate::step::{ParamDef, ParamType, Step, StepContext, StepGroup, StepRegistry, parse_color};
use fp_raster::{CLEAR, IRect, Px, RgbaImage};
use rayon::prelude::*;
use serde_json::{Value, json};
use std::sync::Arc;

/// ID „Größe ändern“.
pub const RESIZE: &str = "resize";
/// ID „Zuschneiden“.
pub const CROP: &str = "crop";
/// ID „Ränder abschneiden“.
pub const TRIM: &str = "trim";
/// ID „Drehen/Spiegeln“.
pub const ROTATE: &str = "rotate";
/// ID „Leinwand/Rand“.
pub const CANVAS: &str = "canvas";

/// Schlüssel in `meta.extra`: Maßstab des Vorschau-Proxys gegenüber der vollen Größe (≤ 1).
pub const PREVIEW_SCALE_KEY: &str = "previewScale";
/// Schlüssel in `meta.extra`: Die EXIF-Ausrichtung wurde schon auf die Pixel angewendet (das
/// Metadaten-Paket schreibt dann Ausrichtung 1).
pub const EXIF_APPLIED_KEY: &str = crate::meta::EXTRA_ORIENTATION_APPLIED;

/// Größte Seitenlänge eines Ziels in px (SPEC §3.3).
pub const MAX_SIDE: i64 = 32768;

/// Registriert die Schritte der Gruppe.
pub fn register(r: &mut StepRegistry) {
    r.register(Arc::new(Resize));
    r.register(Arc::new(Crop));
    r.register(Arc::new(Trim));
    r.register(Arc::new(Rotate));
    r.register(Arc::new(Canvas));
}

// =================================================================== gemeinsame Hilfen

/// Textwert eines Parameters (mit Vorgabe).
pub(crate) fn p_str<'a>(p: &'a Value, k: &str, d: &'a str) -> &'a str {
    p.get(k).and_then(Value::as_str).unwrap_or(d)
}

/// Zahl eines Parameters (mit Vorgabe; `null`/fehlend → Vorgabe).
pub(crate) fn p_f64(p: &Value, k: &str, d: f64) -> f64 {
    p.get(k)
        .and_then(Value::as_f64)
        .filter(|v| v.is_finite())
        .unwrap_or(d)
}

/// Ganzzahl eines Parameters (gerundet).
pub(crate) fn p_i64(p: &Value, k: &str, d: i64) -> i64 {
    p.get(k)
        .and_then(|v| {
            v.as_i64().or_else(|| {
                v.as_f64()
                    .filter(|f| f.is_finite())
                    .map(|f| f.round() as i64)
            })
        })
        .unwrap_or(d)
}

/// Wahrheitswert eines Parameters.
pub(crate) fn p_bool(p: &Value, k: &str, d: bool) -> bool {
    p.get(k).and_then(Value::as_bool).unwrap_or(d)
}

/// Optionale Zahl (`null`/fehlend → None).
pub(crate) fn opt_f64(p: &Value, k: &str) -> Option<f64> {
    p.get(k).and_then(Value::as_f64).filter(|v| v.is_finite())
}

/// Farbe eines Parameters (`#rrggbb[aa]`/`transparent`).
pub(crate) fn p_color(p: &Value, k: &str, d: Px) -> Px {
    p.get(k)
        .and_then(Value::as_str)
        .and_then(parse_color)
        .unwrap_or(d)
}

/// Optionaler Zahlenparameter ohne Vorgabe (`null` = „nicht gesetzt“).
pub(crate) fn opt_param(name: &str, ty: ParamType, unit: Option<&str>) -> ParamDef {
    ParamDef {
        unit: unit.map(str::to_string),
        ..ParamDef::new(name, ty, Value::Null)
    }
}

/// Maßstab des Vorschau-Proxys (1,0 außerhalb der Vorschau). Beim ersten Aufruf je Element aus
/// den Maßen der Quelle (`ctx.info`) und des Proxys bestimmt und in `meta.extra` gemerkt; spätere
/// Schritte (auch nach einer Größenänderung) benutzen denselben Wert.
pub fn preview_scale(item: &mut Item, ctx: &StepContext) -> f64 {
    if !ctx.preview {
        return 1.0;
    }
    // BATCH-11a: Maßstab aus der Vorschau-Pipeline (genau, auch nach Größenschritten davor)
    if let Some(s) = ctx.preview_scale() {
        return s;
    }
    if let Some(s) = item
        .meta()
        .and_then(|m| m.extra.get(PREVIEW_SCALE_KEY))
        .and_then(Value::as_f64)
        .filter(|s| *s > 0.0)
    {
        return s;
    }
    let (w, h) = item.dimensions();
    let full = ctx.info.width.max(ctx.info.height) as f64;
    let cur = w.max(h) as f64;
    let s = if full > 0.0 && cur > 0.0 {
        (cur / full).min(1.0)
    } else {
        1.0
    };
    if let Some(m) = item.meta_mut() {
        m.extra.insert(PREVIEW_SCALE_KEY.into(), json!(s));
    }
    s
}

/// Fehler „Bild zu groß“ (übersetzt).
pub(crate) fn too_large(w: i64, h: i64) -> BatchError {
    BatchError::Invalid(fp_i18n::tr!(
        "batch-err-image-too-large",
        width = w,
        height = h
    ))
}

/// Neues transparentes Bild `w × h` am Ursprung, mit Größenprüfung.
pub(crate) fn new_image(w: i64, h: i64) -> Result<RgbaImage> {
    if w <= 0 || h <= 0 {
        return Ok(RgbaImage::empty_at(0, 0));
    }
    if w > MAX_SIDE * 4
        || h > MAX_SIDE * 4
        || (w as u128) * (h as u128) > fp_raster::MAX_PIXELS as u128
    {
        return Err(too_large(w, h));
    }
    RgbaImage::new(IRect::new(0, 0, w as i32, h as i32)).map_err(|_| too_large(w, h))
}

/// Alle Pixelbilder eines Elements (Einzelbild bzw. jedes Frame; Dokumente flach gerechnet).
pub(crate) fn frames_mut(item: &mut Item) -> Result<Vec<&mut RgbaImage>> {
    item.flatten_document()?;
    Ok(match item {
        Item::Still(s) => vec![&mut s.image],
        Item::Animation(a) => a.frames.iter_mut().map(|f| &mut f.image).collect(),
        Item::Document(_) | Item::FileOnly(_) => Vec::new(),
    })
}

/// Nach einer Änderung: Ursprung (0, 0) und Animationsgröße aus dem ersten Frame.
pub(crate) fn finish(item: &mut Item) {
    match item {
        Item::Still(s) => s.image.set_origin(0, 0),
        Item::Animation(a) => {
            for f in &mut a.frames {
                f.image.set_origin(0, 0);
            }
            if let Some(f) = a.frames.first() {
                a.width = f.image.width() as u32;
                a.height = f.image.height() as u32;
            }
        }
        _ => {}
    }
}

/// Transparenz nach einer Änderung neu bestimmen (für die Ausgabe, z. B. JPEG-Hintergrund).
pub(crate) fn refresh_transparency(item: &mut Item) {
    let t = match &*item {
        Item::Still(s) => s.image.data().iter().any(|p| p[3] < 255),
        Item::Animation(a) => a
            .frames
            .iter()
            .any(|f| f.image.data().iter().any(|p| p[3] < 255)),
        _ => return,
    };
    if let Some(m) = item.meta_mut() {
        m.has_transparency = t;
    }
}

/// Die neun Ankerpunkte.
pub const ANCHORS: [&str; 9] = [
    "topLeft",
    "top",
    "topRight",
    "left",
    "center",
    "right",
    "bottomLeft",
    "bottom",
    "bottomRight",
];

/// Anteil (0, ½, 1) je Achse eines Ankers.
pub(crate) fn anchor_factors(a: &str) -> (f64, f64) {
    let fx = match a {
        "topLeft" | "left" | "bottomLeft" => 0.0,
        "topRight" | "right" | "bottomRight" => 1.0,
        _ => 0.5,
    };
    let fy = match a {
        "topLeft" | "top" | "topRight" => 0.0,
        "bottomLeft" | "bottom" | "bottomRight" => 1.0,
        _ => 0.5,
    };
    (fx, fy)
}

/// Seitenverhältnisse (`free` = `ratioWidth : ratioHeight`).
pub const RATIOS: [&str; 8] = ["1:1", "4:3", "3:2", "16:9", "9:16", "3:4", "2:3", "free"];

/// Seitenverhältnis Breite/Höhe aus `ratio` bzw. `ratioWidth`/`ratioHeight`.
pub(crate) fn ratio_of(p: &Value) -> Option<f64> {
    let r = p_str(p, "ratio", "1:1");
    let (a, b) = match r.split_once(':') {
        Some((a, b)) if r != "free" => {
            (a.trim().parse::<f64>().ok()?, b.trim().parse::<f64>().ok()?)
        }
        _ => (p_f64(p, "ratioWidth", 1.0), p_f64(p, "ratioHeight", 1.0)),
    };
    (a > 0.0 && b > 0.0 && a.is_finite() && b.is_finite()).then_some(a / b)
}

/// Mischt `s` über `b` (nicht vormultipliziert, gerundet).
pub(crate) fn over(s: Px, b: Px) -> Px {
    if s[3] == 255 || b[3] == 0 {
        return s;
    }
    if s[3] == 0 {
        return b;
    }
    let sa = s[3] as u32;
    let ba = b[3] as u32 * (255 - sa) / 255;
    let oa = sa + ba;
    let mut o = [0u8; 4];
    for c in 0..3 {
        o[c] = ((s[c] as u32 * sa + b[c] as u32 * ba + oa / 2) / oa).min(255) as u8;
    }
    o[3] = oa.min(255) as u8;
    o
}

/// Neues Bild `w × h` mit Hintergrund `bg`; `img` liegt mit seiner linken oberen Ecke bei
/// (`ox`, `oy`) und wird über den Hintergrund gemischt (Teile außerhalb fallen weg).
pub(crate) fn place(
    img: &RgbaImage,
    w: i64,
    h: i64,
    ox: i64,
    oy: i64,
    bg: Px,
) -> Result<RgbaImage> {
    let mut out = new_image(w, h)?;
    if out.is_empty() {
        return Ok(out);
    }
    let (sw, sh) = (img.width() as i64, img.height() as i64);
    let ow = out.width();
    out.data_mut()
        .par_chunks_mut(ow)
        .enumerate()
        .for_each(|(y, row)| {
            let sy = y as i64 - oy;
            let src = (sy >= 0 && sy < sh).then(|| img.row_local(sy as usize));
            for (x, px) in row.iter_mut().enumerate() {
                let sx = x as i64 - ox;
                *px = match src {
                    Some(r) if sx >= 0 && sx < sw => over(r[sx as usize], bg),
                    _ => bg,
                };
            }
        });
    Ok(out)
}

/// Ausschnitt `rect` (Bildkoordinaten, darf über das Bild hinausragen → transparent bzw. `bg`).
pub(crate) fn cut(img: &RgbaImage, rect: IRect, bg: Px) -> Result<RgbaImage> {
    place(
        img,
        rect.width() as i64,
        rect.height() as i64,
        -(rect.left as i64),
        -(rect.top as i64),
        bg,
    )
}

// =================================================================== Resampling

/// Abtastfilter beim Skalieren.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResizeFilter {
    Nearest,
    Bilinear,
    Bicubic,
    Lanczos3,
}

impl ResizeFilter {
    /// Aus dem Parameterwert (`auto` → None).
    pub fn parse(s: &str) -> Option<ResizeFilter> {
        match s {
            "nearest" => Some(ResizeFilter::Nearest),
            "bilinear" => Some(ResizeFilter::Bilinear),
            "bicubic" => Some(ResizeFilter::Bicubic),
            "lanczos3" => Some(ResizeFilter::Lanczos3),
            _ => None,
        }
    }

    fn support(self) -> f64 {
        match self {
            ResizeFilter::Nearest => 0.5,
            ResizeFilter::Bilinear => 1.0,
            ResizeFilter::Bicubic => 2.0,
            ResizeFilter::Lanczos3 => 3.0,
        }
    }

    fn weight(self, x: f64) -> f64 {
        let x = x.abs();
        match self {
            ResizeFilter::Nearest => {
                if x < 0.5 {
                    1.0
                } else {
                    0.0
                }
            }
            ResizeFilter::Bilinear => (1.0 - x).max(0.0),
            // Catmull-Rom (a = −0,5): scharf, ohne starkes Überschwingen
            ResizeFilter::Bicubic => {
                if x < 1.0 {
                    1.5 * x * x * x - 2.5 * x * x + 1.0
                } else if x < 2.0 {
                    -0.5 * x * x * x + 2.5 * x * x - 4.0 * x + 2.0
                } else {
                    0.0
                }
            }
            ResizeFilter::Lanczos3 => {
                if x < 3.0 {
                    sinc(x) * sinc(x / 3.0)
                } else {
                    0.0
                }
            }
        }
    }
}

fn sinc(x: f64) -> f64 {
    if x.abs() < 1e-12 {
        1.0
    } else {
        let a = std::f64::consts::PI * x;
        a.sin() / a
    }
}

/// Gewichte eines Zielpixels: erstes Quellpixel und normierte Gewichte.
struct Contrib {
    start: usize,
    weights: Vec<f32>,
}

fn contributions(src: usize, dst: usize, f: ResizeFilter) -> Vec<Contrib> {
    let scale = src as f64 / dst as f64;
    let fs = scale.max(1.0);
    let support = f.support() * fs;
    (0..dst)
        .map(|i| {
            let center = (i as f64 + 0.5) * scale;
            let left = ((center - support).floor().max(0.0)) as usize;
            let right = ((center + support).ceil() as usize).min(src).max(left + 1);
            let mut w: Vec<f64> = (left..right)
                .map(|j| f.weight((j as f64 + 0.5 - center) / fs))
                .collect();
            let sum: f64 = w.iter().sum();
            if sum.abs() < 1e-12 {
                // kein Beitrag (sehr schmaler Kern): nächstes Pixel
                let j = (center.floor() as usize).min(src - 1);
                return Contrib {
                    start: j,
                    weights: vec![1.0],
                };
            }
            for x in &mut w {
                *x /= sum;
            }
            Contrib {
                start: left,
                weights: w.into_iter().map(|x| x as f32).collect(),
            }
        })
        .collect()
}

/// Skaliert `src` auf `nw × nh` (Ursprung 0, 0). Getrennt in Spalten und Zeilen, vormultipliziertes
/// Alpha (keine dunklen Säume an transparenten Kanten), beim Verkleinern mit verbreitertem Kern
/// (Flächenmittel statt Punktabtastung). Zeilenweise parallel, bitgleich zur Einzel-Thread-Rechnung.
pub fn resample(
    src: &RgbaImage,
    nw: u32,
    nh: u32,
    f: ResizeFilter,
    cancel: &CancelToken,
) -> Result<RgbaImage> {
    let (w, h) = (src.width(), src.height());
    let mut out = new_image(nw as i64, nh as i64)?;
    if out.is_empty() || w == 0 || h == 0 {
        return Ok(out);
    }
    if nw as usize == w && nh as usize == h {
        let mut c = src.clone();
        c.set_origin(0, 0);
        return Ok(c);
    }
    let nwu = nw as usize;
    if f == ResizeFilter::Nearest {
        out.data_mut()
            .par_chunks_mut(nwu)
            .enumerate()
            .for_each(|(y, row)| {
                let sy = ((2 * y + 1) * h / (2 * nh as usize)).min(h - 1);
                let srow = src.row_local(sy);
                for (x, px) in row.iter_mut().enumerate() {
                    let sx = ((2 * x + 1) * w / (2 * nwu)).min(w - 1);
                    *px = srow[sx];
                }
            });
        cancel.check()?;
        return Ok(out);
    }
    let xc = contributions(w, nwu, f);
    let yc = contributions(h, nh as usize, f);
    out.data_mut()
        .par_chunks_mut(nwu)
        .enumerate()
        .for_each(|(y, row)| {
            if cancel.is_cancelled() {
                return;
            }
            // senkrecht: gewichtete Summe der Quellzeilen (vormultipliziert)
            let cy = &yc[y];
            let mut buf = vec![[0f32; 4]; w];
            for (k, &wy) in cy.weights.iter().enumerate() {
                let srow = src.row_local(cy.start + k);
                for (b, p) in buf.iter_mut().zip(srow) {
                    let a = p[3] as f32;
                    let m = wy * a / 255.0;
                    b[0] += p[0] as f32 * m;
                    b[1] += p[1] as f32 * m;
                    b[2] += p[2] as f32 * m;
                    b[3] += wy * a;
                }
            }
            // waagerecht
            for (x, px) in row.iter_mut().enumerate() {
                let cx = &xc[x];
                let mut s = [0f32; 4];
                for (k, &wx) in cx.weights.iter().enumerate() {
                    let b = &buf[cx.start + k];
                    s[0] += b[0] * wx;
                    s[1] += b[1] * wx;
                    s[2] += b[2] * wx;
                    s[3] += b[3] * wx;
                }
                let a = s[3].clamp(0.0, 255.0);
                if a < 0.5 {
                    *px = CLEAR;
                    continue;
                }
                let k = 255.0 / a;
                let ch = |v: f32| (v * k + 0.5).floor().clamp(0.0, 255.0) as u8;
                *px = [ch(s[0]), ch(s[1]), ch(s[2]), (a + 0.5).floor() as u8];
            }
        });
    cancel.check()?;
    Ok(out)
}

/// Sieht das Bild nach Pixelgrafik aus (klein, höchstens 256 Farben)? Dann skaliert `auto` mit
/// nächstem Nachbarn, damit die Kanten hart bleiben.
pub fn looks_like_pixel_art(img: &RgbaImage) -> bool {
    if img.width().max(img.height()) > 512 || img.is_empty() {
        return false;
    }
    let mut seen = std::collections::HashSet::new();
    for p in img.data() {
        seen.insert(*p);
        if seen.len() > 256 {
            return false;
        }
    }
    true
}

// =================================================================== resize

/// `resize`: Größe ändern.
struct Resize;

/// Modi von `resize`.
pub const RESIZE_MODES: [&str; 9] = [
    "percent",
    "width",
    "height",
    "longEdge",
    "shortEdge",
    "fit",
    "fill",
    "exact",
    "none",
];

/// Zielgröße (Breite, Höhe) und bei `fill` der Ausschnitt danach (Breite, Höhe).
type ResizeTarget = (f64, f64, Option<(f64, f64)>);

/// Zielgröße in voller Auflösung (None = nicht ändern).
fn resize_target(w: f64, h: f64, p: &Value) -> Option<ResizeTarget> {
    if w <= 0.0 || h <= 0.0 {
        return None;
    }
    let only_shrink = p_bool(p, "onlyShrink", true);
    let tw = p_f64(p, "width", 1600.0).max(1.0);
    let th = p_f64(p, "height", 1600.0).max(1.0);
    let size = p_f64(p, "size", 1600.0).max(1.0);
    let clamp = |s: f64| if only_shrink { s.min(1.0) } else { s };
    let uniform = |s: f64| Some((w * clamp(s), h * clamp(s), None));
    match p_str(p, "mode", "longEdge") {
        "percent" => uniform(p_f64(p, "percent", 50.0) / 100.0),
        "width" => uniform(tw / w),
        "height" => uniform(th / h),
        "longEdge" => uniform(size / w.max(h)),
        "shortEdge" => uniform(size / w.min(h)),
        "fit" => uniform((tw / w).min(th / h)),
        "fill" => {
            let s = clamp((tw / w).max(th / h));
            let (sw, sh) = (w * s, h * s);
            // nicht vergrößern: auf das Seitenverhältnis des Rahmens zuschneiden
            let k = (sw / tw).min(sh / th).min(1.0);
            Some((sw, sh, Some((tw * k, th * k))))
        }
        "exact" => {
            if only_shrink {
                Some((tw.min(w), th.min(h), None))
            } else {
                Some((tw, th, None))
            }
        }
        _ => None,
    }
}

fn px_round(v: f64) -> i64 {
    (v.round() as i64).max(1)
}

impl Step for Resize {
    fn id(&self) -> &str {
        RESIZE
    }
    fn group(&self) -> StepGroup {
        StepGroup::Geometry
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::choice("mode", &RESIZE_MODES, "longEdge").beginner(),
            ParamDef::int("size", 1, MAX_SIDE, 1600)
                .beginner()
                .unit("px")
                .when("mode", &["longEdge", "shortEdge"]),
            ParamDef::int("width", 1, MAX_SIDE, 1600)
                .beginner()
                .unit("px")
                .when("mode", &["width", "fit", "fill", "exact"]),
            ParamDef::int("height", 1, MAX_SIDE, 1600)
                .beginner()
                .unit("px")
                .when("mode", &["height", "fit", "fill", "exact"]),
            ParamDef::number("percent", 1.0, 10000.0, 50.0)
                .beginner()
                .unit("%")
                .when("mode", &["percent"]),
            ParamDef::bool("onlyShrink", true)
                .beginner()
                .when("mode", &RESIZE_MODES[..8]),
            ParamDef::choice(
                "filter",
                &["auto", "nearest", "bilinear", "bicubic", "lanczos3"],
                "auto",
            )
            .when("mode", &RESIZE_MODES[..8]),
            opt_param(
                "dpi",
                ParamType::Number {
                    min: 1.0,
                    max: 10000.0,
                },
                Some("dpi"),
            ),
        ]
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let scale = preview_scale(item, ctx);
        if let Some(d) = opt_f64(params, "dpi")
            && let Some(m) = item.meta_mut()
        {
            m.dpi = Some(d as f32);
        }
        let (cw, ch) = item.dimensions();
        if cw == 0 || ch == 0 {
            return Ok(());
        }
        let Some((tw, th, fill)) = resize_target(cw as f64 / scale, ch as f64 / scale, params)
        else {
            return Ok(());
        };
        let (nw, nh) = (px_round(tw * scale), px_round(th * scale));
        if nw > MAX_SIDE * 4
            || nh > MAX_SIDE * 4
            || (nw as u128) * (nh as u128) > fp_raster::MAX_PIXELS as u128
        {
            return Err(too_large(px_round(tw), px_round(th)));
        }
        let crop =
            fill.map(|(fw, fh)| (px_round(fw * scale).min(nw), px_round(fh * scale).min(nh)));
        if nw == cw as i64 && nh == ch as i64 && crop.is_none_or(|c| c == (nw, nh)) {
            return Ok(());
        }
        let filter_name = p_str(params, "filter", "auto");
        let fixed = ResizeFilter::parse(filter_name);
        let cancel = ctx.cancel;
        let mut auto: Option<ResizeFilter> = None;
        for img in frames_mut(item)? {
            cancel.check()?;
            let f = match fixed {
                Some(f) => f,
                // „auto“ entscheidet einmal je Element (erstes Bild), damit alle Frames gleich skalieren
                None => *auto.get_or_insert_with(|| {
                    if looks_like_pixel_art(img) {
                        ResizeFilter::Nearest
                    } else {
                        ResizeFilter::Lanczos3
                    }
                }),
            };
            let mut r = resample(img, nw as u32, nh as u32, f, cancel)?;
            if let Some((fw, fh)) = crop
                && (fw, fh) != (nw, nh)
            {
                let x = (nw - fw) / 2;
                let y = (nh - fh) / 2;
                r = cut(
                    &r,
                    IRect::new(x as i32, y as i32, (x + fw) as i32, (y + fh) as i32),
                    CLEAR,
                )?;
            }
            *img = r;
        }
        finish(item);
        Ok(())
    }
    fn summary(&self, p: &Value) -> String {
        let mode = p_str(p, "mode", "longEdge");
        let key = format!("batch-summary-resize-{mode}");
        if !fp_i18n::has(&key) {
            return String::new();
        }
        fp_i18n::tr!(
            &key,
            size = p_i64(p, "size", 1600),
            width = p_i64(p, "width", 1600),
            height = p_i64(p, "height", 1600),
            percent = fmt_num(p_f64(p, "percent", 50.0))
        )
    }
}

/// Zahl ohne überflüssige Nachkommastellen, Dezimalzeichen der Sprache.
pub(crate) fn fmt_num(v: f64) -> String {
    fp_i18n::fmt_number(v, 0, 2)
}

// =================================================================== crop

/// `crop`: Zuschneiden (Seitenverhältnis, Rechteck oder Ränder; ohne „smart“, das kommt in P2).
struct Crop;

/// Modi von `crop`.
pub const CROP_MODES: [&str; 3] = ["aspect", "rect", "edges"];

/// Rechteck des Ausschnitts in Bildpixeln (None = unverändert).
fn crop_rect(w: i64, h: i64, p: &Value, scale: f64) -> Option<IRect> {
    if w <= 0 || h <= 0 {
        return None;
    }
    let percent = p_str(p, "unit", "px") == "percent";
    // Länge in Bildpixeln (px-Werte im Proxy-Maßstab, Prozent vom jeweiligen Maß)
    let len = |k: &str, d: f64, full: i64| -> f64 {
        let v = p_f64(p, k, d).max(0.0);
        if percent {
            v / 100.0 * full as f64
        } else {
            v * scale
        }
    };
    let (fx, fy) = anchor_factors(p_str(p, "anchor", "center"));
    let r = match p_str(p, "mode", "aspect") {
        "aspect" => {
            let ratio = ratio_of(p)?;
            let (mut cw, mut ch) = (w, h);
            if (w as f64 / h as f64) > ratio {
                cw = px_round(h as f64 * ratio).min(w);
            } else {
                ch = px_round(w as f64 / ratio).min(h);
            }
            let x = ((w - cw) as f64 * fx).round() as i64;
            let y = ((h - ch) as f64 * fy).round() as i64;
            IRect::new(x as i32, y as i32, (x + cw) as i32, (y + ch) as i32)
        }
        "rect" => {
            let x = len("x", 0.0, w).round() as i64;
            let y = len("y", 0.0, h).round() as i64;
            let cw = len("width", 1000.0, w).round() as i64;
            let ch = len("height", 1000.0, h).round() as i64;
            IRect::new(
                x.min(i32::MAX as i64) as i32,
                y.min(i32::MAX as i64) as i32,
                (x + cw).min(i32::MAX as i64) as i32,
                (y + ch).min(i32::MAX as i64) as i32,
            )
        }
        _ => {
            let t = len("top", 0.0, h).round() as i64;
            let b = len("bottom", 0.0, h).round() as i64;
            let l = len("left", 0.0, w).round() as i64;
            let rr = len("right", 0.0, w).round() as i64;
            IRect::new(
                l.min(w) as i32,
                t.min(h) as i32,
                (w - rr).max(0) as i32,
                (h - b).max(0) as i32,
            )
        }
    };
    Some(r.intersect(&IRect::new(0, 0, w as i32, h as i32)))
}

fn anchor_param() -> ParamDef {
    ParamDef::choice("anchor", &ANCHORS, "center").beginner()
}

fn ratio_params(beginner: bool, ratio_modes: &[&str]) -> Vec<ParamDef> {
    let r = ParamDef::choice("ratio", &RATIOS, "1:1").when("mode", ratio_modes);
    vec![
        if beginner { r.beginner() } else { r },
        ParamDef::number("ratioWidth", 0.01, 10000.0, 1.0).when("ratio", &["free"]),
        ParamDef::number("ratioHeight", 0.01, 10000.0, 1.0).when("ratio", &["free"]),
    ]
}

impl Step for Crop {
    fn id(&self) -> &str {
        CROP
    }
    fn group(&self) -> StepGroup {
        StepGroup::Geometry
    }
    fn params(&self) -> Vec<ParamDef> {
        let mut v = vec![ParamDef::choice("mode", &CROP_MODES, "aspect").beginner()];
        v.extend(ratio_params(true, &["aspect"]));
        v.extend([
            anchor_param().when("mode", &["aspect"]),
            ParamDef::choice("unit", &["px", "percent"], "px")
                .beginner()
                .when("mode", &["rect", "edges"]),
            ParamDef::number("x", 0.0, 100000.0, 0.0).when("mode", &["rect"]),
            ParamDef::number("y", 0.0, 100000.0, 0.0).when("mode", &["rect"]),
            ParamDef::number("width", 0.0, 100000.0, 1000.0).when("mode", &["rect"]),
            ParamDef::number("height", 0.0, 100000.0, 1000.0).when("mode", &["rect"]),
            ParamDef::number("top", 0.0, 100000.0, 0.0).when("mode", &["edges"]),
            ParamDef::number("right", 0.0, 100000.0, 0.0).when("mode", &["edges"]),
            ParamDef::number("bottom", 0.0, 100000.0, 0.0).when("mode", &["edges"]),
            ParamDef::number("left", 0.0, 100000.0, 0.0).when("mode", &["edges"]),
        ]);
        v
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let scale = preview_scale(item, ctx);
        item.flatten_document()?;
        let (w, h) = item.dimensions();
        let Some(r) = crop_rect(w as i64, h as i64, params, scale) else {
            return Ok(());
        };
        if r.is_empty() {
            ctx.warn(fp_i18n::t("batch-warn-crop-empty"));
            return Ok(());
        }
        if r == IRect::new(0, 0, w as i32, h as i32) {
            return Ok(());
        }
        for img in frames_mut(item)? {
            ctx.cancel.check()?;
            *img = cut(img, r, CLEAR)?;
        }
        finish(item);
        Ok(())
    }
    fn summary(&self, p: &Value) -> String {
        match p_str(p, "mode", "aspect") {
            "aspect" => {
                let r = p_str(p, "ratio", "1:1");
                let r = if r == "free" {
                    format!(
                        "{}:{}",
                        fmt_num(p_f64(p, "ratioWidth", 1.0)),
                        fmt_num(p_f64(p, "ratioHeight", 1.0))
                    )
                } else {
                    r.to_string()
                };
                fp_i18n::tr!("batch-summary-crop-aspect", ratio = r)
            }
            "rect" => fp_i18n::t("batch-summary-crop-rect"),
            _ => fp_i18n::t("batch-summary-crop-edges"),
        }
    }
}

// =================================================================== trim

/// `trim`: Ränder abschneiden (transparent, Farbe der Ecke oben links oder beides).
struct Trim;

/// Inhaltsrechteck eines Bildes (leer, wenn alles Hintergrund ist).
fn content_bounds(img: &RgbaImage, what: &str, tol: u8, corner: Px) -> IRect {
    let (w, h) = (img.width(), img.height());
    let is_bg = |p: &Px| -> bool {
        let transparent = p[3] <= tol;
        let color = (0..4).all(|c| (p[c] as i16 - corner[c] as i16).unsigned_abs() <= tol as u16);
        match what {
            "color" => color,
            "both" => transparent || color,
            _ => transparent,
        }
    };
    let rows: Vec<Option<(usize, usize)>> = (0..h)
        .into_par_iter()
        .map(|y| {
            let row = img.row_local(y);
            let first = row.iter().position(|p| !is_bg(p))?;
            let last = row.iter().rposition(|p| !is_bg(p)).unwrap_or(first);
            Some((first, last + 1))
        })
        .collect();
    let (mut x0, mut x1, mut y0, mut y1) = (usize::MAX, 0usize, usize::MAX, 0usize);
    for (y, r) in rows.iter().enumerate() {
        if let Some((a, b)) = r {
            x0 = x0.min(*a);
            x1 = x1.max(*b);
            y0 = y0.min(y);
            y1 = y + 1;
        }
    }
    if x0 == usize::MAX || w == 0 {
        return IRect::new(0, 0, 0, 0);
    }
    IRect::new(x0 as i32, y0 as i32, x1 as i32, y1 as i32)
}

impl Step for Trim {
    fn id(&self) -> &str {
        TRIM
    }
    fn group(&self) -> StepGroup {
        StepGroup::Geometry
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::choice("what", &["transparent", "color", "both"], "transparent").beginner(),
            ParamDef::int("tolerance", 0, 255, 8).when("what", &["color", "both"]),
            ParamDef::int("padding", 0, 500, 0).unit("px"),
        ]
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let scale = preview_scale(item, ctx);
        let what = p_str(params, "what", "transparent").to_string();
        let tol = p_i64(params, "tolerance", 8).clamp(0, 255) as u8;
        let pad = (p_i64(params, "padding", 0).clamp(0, 500) as f64 * scale).round() as i32;
        let mut frames = frames_mut(item)?;
        let Some(first) = frames.first() else {
            return Ok(());
        };
        if first.is_empty() {
            return Ok(());
        }
        let corner = first.row_local(0)[0];
        let full = first.bounds();
        // Animation: Vereinigung über alle Frames, damit alle Frames gleich groß bleiben
        let mut union = IRect::new(0, 0, 0, 0);
        for f in frames.iter() {
            ctx.cancel.check()?;
            let b = content_bounds(f, &what, tol, corner);
            if !b.is_empty() {
                union = if union.is_empty() { b } else { union.union(&b) };
            }
        }
        if union.is_empty() {
            ctx.warn(fp_i18n::t("batch-warn-trim-empty"));
            return Ok(());
        }
        let target = union.inflate(pad, pad);
        if target == full {
            return Ok(());
        }
        let bg = if what == "color" { corner } else { CLEAR };
        for f in frames.iter_mut() {
            ctx.cancel.check()?;
            **f = cut(f, target, bg)?;
        }
        drop(frames);
        finish(item);
        Ok(())
    }
    fn summary(&self, p: &Value) -> String {
        fp_i18n::t(&format!(
            "batch-summary-trim-{}",
            p_str(p, "what", "transparent")
        ))
    }
}

// =================================================================== rotate

/// `rotate`: Drehen und Spiegeln, optional nach EXIF-Ausrichtung.
struct Rotate;

/// Ordnet Pixel um: Ziel `nw × nh`, Quelle je Zielpixel über `f(x, y) → (sx, sy)`.
fn remap(
    img: &RgbaImage,
    nw: usize,
    nh: usize,
    f: impl Fn(usize, usize) -> (usize, usize) + Sync,
) -> Result<RgbaImage> {
    let mut out = new_image(nw as i64, nh as i64)?;
    if out.is_empty() {
        return Ok(out);
    }
    let data = img.data();
    let w = img.width();
    out.data_mut()
        .par_chunks_mut(nw)
        .enumerate()
        .for_each(|(y, row)| {
            for (x, px) in row.iter_mut().enumerate() {
                let (sx, sy) = f(x, y);
                *px = data[sy * w + sx];
            }
        });
    Ok(out)
}

/// Verlustfreie Grundoperationen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orient {
    Rot90,
    Rot180,
    Rot270,
    FlipH,
    FlipV,
    /// Spiegeln an der Hauptdiagonale (EXIF 5).
    Transpose,
    /// Spiegeln an der Nebendiagonale (EXIF 7).
    Transverse,
}

/// Wendet eine verlustfreie Operation an (Drehungen im Uhrzeigersinn).
pub fn orient(img: &RgbaImage, o: Orient) -> Result<RgbaImage> {
    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 {
        return Ok(img.clone());
    }
    match o {
        Orient::Rot90 => remap(img, h, w, |x, y| (y, h - 1 - x)),
        Orient::Rot180 => remap(img, w, h, |x, y| (w - 1 - x, h - 1 - y)),
        Orient::Rot270 => remap(img, h, w, |x, y| (w - 1 - y, x)),
        Orient::FlipH => remap(img, w, h, |x, y| (w - 1 - x, y)),
        Orient::FlipV => remap(img, w, h, |x, y| (x, h - 1 - y)),
        Orient::Transpose => remap(img, h, w, |x, y| (y, x)),
        Orient::Transverse => remap(img, h, w, |x, y| (w - 1 - y, h - 1 - x)),
    }
}

/// Operation zum EXIF-Ausrichtungswert (1 = keine).
pub fn exif_orient(v: u16) -> Option<Orient> {
    match v {
        2 => Some(Orient::FlipH),
        3 => Some(Orient::Rot180),
        4 => Some(Orient::FlipV),
        5 => Some(Orient::Transpose),
        6 => Some(Orient::Rot90),
        7 => Some(Orient::Transverse),
        8 => Some(Orient::Rot270),
        _ => None,
    }
}

/// Lage des Ausrichtungswerts (Tag 0x0112) in TIFF-Daten: (Position, big endian).
fn tiff_orientation_pos(t: &[u8]) -> Option<(usize, bool)> {
    let be = match t.get(0..4)? {
        [b'I', b'I', 42, 0] => false,
        [b'M', b'M', 0, 42] => true,
        _ => return None,
    };
    let u16_at = |i: usize| -> Option<u16> {
        let b = t.get(i..i + 2)?;
        Some(if be {
            u16::from_be_bytes([b[0], b[1]])
        } else {
            u16::from_le_bytes([b[0], b[1]])
        })
    };
    let u32_at = |i: usize| -> Option<u32> {
        let b = t.get(i..i + 4)?;
        Some(if be {
            u32::from_be_bytes([b[0], b[1], b[2], b[3]])
        } else {
            u32::from_le_bytes([b[0], b[1], b[2], b[3]])
        })
    };
    let ifd = u32_at(4)? as usize;
    let n = u16_at(ifd)? as usize;
    for i in 0..n.min(512) {
        let e = ifd + 2 + i * 12;
        if u16_at(e)? == 0x0112 && u16_at(e + 2)? == 3 {
            return Some((e + 8, be));
        }
    }
    None
}

/// TIFF-Teil von EXIF-Daten (mit oder ohne `Exif\0\0`, auch aus JPEG-APP1 bzw. PNG-`eXIf`).
fn tiff_slice(d: &[u8]) -> Option<(usize, &[u8])> {
    if d.starts_with(b"Exif\0\0") {
        return Some((6, &d[6..]));
    }
    if d.starts_with(b"II*\0") || d.starts_with(b"MM\0*") {
        return Some((0, d));
    }
    if d.starts_with(&[0xFF, 0xD8]) {
        // JPEG: Segmente bis SOS durchgehen, APP1 mit „Exif“
        let mut i = 2;
        while i + 4 <= d.len() && d[i] == 0xFF {
            let marker = d[i + 1];
            if marker == 0xDA || marker == 0xD9 {
                break;
            }
            let len = u16::from_be_bytes([d[i + 2], d[i + 3]]) as usize;
            if marker == 0xE1 && d.get(i + 4..i + 10) == Some(b"Exif\0\0") {
                let end = (i + 2 + len).min(d.len());
                return Some((i + 10, &d[i + 10..end]));
            }
            i += 2 + len;
        }
        return None;
    }
    if d.starts_with(&[0x89, b'P', b'N', b'G']) {
        let mut i = 8;
        while i + 8 <= d.len() {
            let len = u32::from_be_bytes([d[i], d[i + 1], d[i + 2], d[i + 3]]) as usize;
            let kind = &d[i + 4..i + 8];
            if kind == b"eXIf" {
                let end = (i + 8 + len).min(d.len());
                return Some((i + 8, &d[i + 8..end]));
            }
            if kind == b"IDAT" || kind == b"IEND" {
                break;
            }
            i += 12 + len;
        }
    }
    None
}

/// EXIF-Ausrichtung (1…8) aus EXIF-Daten, TIFF-Daten oder dem Anfang einer JPEG-/PNG-Datei.
pub fn exif_orientation(data: &[u8]) -> Option<u16> {
    let (_, t) = tiff_slice(data)?;
    let (pos, be) = tiff_orientation_pos(t)?;
    let b = t.get(pos..pos + 2)?;
    let v = if be {
        u16::from_be_bytes([b[0], b[1]])
    } else {
        u16::from_le_bytes([b[0], b[1]])
    };
    (1..=8).contains(&v).then_some(v)
}

/// Setzt die EXIF-Ausrichtung in den Daten auf `v` (falls vorhanden). Liefert, ob etwas geändert wurde.
pub fn set_exif_orientation(data: &mut [u8], v: u16) -> bool {
    let Some((off, t)) = tiff_slice(data) else {
        return false;
    };
    let Some((pos, be)) = tiff_orientation_pos(t) else {
        return false;
    };
    let bytes = if be { v.to_be_bytes() } else { v.to_le_bytes() };
    match data.get_mut(off + pos..off + pos + 2) {
        Some(s) => {
            s.copy_from_slice(&bytes);
            true
        }
        None => false,
    }
}

/// Freie Drehung um `deg` Grad im Uhrzeigersinn; die Fläche wächst auf das umschließende Rechteck,
/// leere Ecken bekommen `bg`.
fn rotate_free(
    img: &RgbaImage,
    deg: f64,
    sampler: fp_raster::Sampler,
    bg: Px,
) -> Result<RgbaImage> {
    let (w, h) = (img.width() as f64, img.height() as f64);
    if w == 0.0 || h == 0.0 {
        return Ok(img.clone());
    }
    let t = deg.to_radians();
    let (s, c) = t.sin_cos();
    let nw = (w * c.abs() + h * s.abs() - 1e-6).ceil().max(1.0);
    let nh = (w * s.abs() + h * c.abs() - 1e-6).ceil().max(1.0);
    let mut out = new_image(nw as i64, nh as i64)?;
    let mut src = img.clone();
    src.set_origin(0, 0);
    let (cx, cy, ncx, ncy) = (w / 2.0, h / 2.0, nw / 2.0, nh / 2.0);
    let ow = out.width();
    out.data_mut()
        .par_chunks_mut(ow)
        .enumerate()
        .for_each(|(y, row)| {
            let dy = y as f64 + 0.5 - ncy;
            for (x, px) in row.iter_mut().enumerate() {
                let dx = x as f64 + 0.5 - ncx;
                // Rückdrehung (Ziel → Quelle)
                let sx = dx * c + dy * s + cx;
                let sy = -dx * s + dy * c + cy;
                let p = fp_raster::resample::sample(
                    &src,
                    sx,
                    sy,
                    sampler,
                    fp_raster::Edge::Transparent,
                );
                *px = over(p, bg);
            }
        });
    Ok(out)
}

impl Step for Rotate {
    fn id(&self) -> &str {
        ROTATE
    }
    fn group(&self) -> StepGroup {
        StepGroup::Geometry
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::number("angle", -360.0, 360.0, 90.0)
                .beginner()
                .unit("°"),
            ParamDef::choice("flip", &["none", "horizontal", "vertical", "both"], "none")
                .beginner(),
            ParamDef::bool("autoExif", true),
            ParamDef::color("background", "transparent"),
            ParamDef::choice("filter", &["bicubic", "bilinear", "nearest"], "bicubic"),
        ]
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        item.flatten_document()?;
        let mut ops: Vec<Orient> = Vec::new();
        // 1. EXIF-Ausrichtung: dieselbe Rechnung wie `metadata.orientation` (BATCH-5). Beim Laden ist sie
        //    meist schon angewendet (Ausrichtung danach 1) – dann ohne Wirkung, nie doppelt.
        if p_bool(params, "autoExif", true) {
            crate::meta::apply_orientation(item);
        }
        // 2. Winkel (Vielfache von 90° verlustfrei)
        let angle = p_f64(params, "angle", 90.0).rem_euclid(360.0);
        let quarter = (angle / 90.0).round();
        let lossless = (angle - quarter * 90.0).abs() < 1e-9;
        if lossless {
            match quarter as i64 % 4 {
                1 => ops.push(Orient::Rot90),
                2 => ops.push(Orient::Rot180),
                3 => ops.push(Orient::Rot270),
                _ => {}
            }
        }
        let flip = p_str(params, "flip", "none");
        let bg = p_color(params, "background", CLEAR);
        let sampler = match p_str(params, "filter", "bicubic") {
            "bilinear" => fp_raster::Sampler::Bilinear,
            "nearest" => fp_raster::Sampler::Nearest,
            _ => fp_raster::Sampler::Bicubic,
        };
        let free = (!lossless).then_some(angle);
        if ops.is_empty() && free.is_none() && flip == "none" {
            return Ok(());
        }
        for img in frames_mut(item)? {
            ctx.cancel.check()?;
            let mut cur = std::mem::take(img);
            for op in &ops {
                cur = orient(&cur, *op)?;
            }
            if let Some(a) = free {
                cur = rotate_free(&cur, a, sampler, bg)?;
            }
            if flip == "horizontal" || flip == "both" {
                cur = orient(&cur, Orient::FlipH)?;
            }
            if flip == "vertical" || flip == "both" {
                cur = orient(&cur, Orient::FlipV)?;
            }
            *img = cur;
        }
        finish(item);
        if free.is_some() {
            refresh_transparency(item);
        }
        Ok(())
    }
    fn summary(&self, p: &Value) -> String {
        let flip = p_str(p, "flip", "none");
        let angle = fmt_num(p_f64(p, "angle", 90.0));
        if flip == "none" {
            fp_i18n::tr!("batch-summary-rotate", angle = angle)
        } else {
            fp_i18n::tr!("batch-summary-rotate-flip", angle = angle)
        }
    }
}

// =================================================================== canvas

/// `canvas`: Leinwand vergrößern bzw. Rand hinzufügen.
struct Canvas;

/// Modi von `canvas`.
pub const CANVAS_MODES: [&str; 4] = ["addBorder", "toSize", "toAspect", "square"];

/// Neue Leinwand (Breite, Höhe, Versatz x, Versatz y) in Bildpixeln.
fn canvas_geometry(w: i64, h: i64, p: &Value, scale: f64) -> Option<(i64, i64, i64, i64)> {
    let (fx, fy) = anchor_factors(p_str(p, "anchor", "center"));
    let placed = |nw: i64, nh: i64| {
        Some((
            nw,
            nh,
            ((nw - w) as f64 * fx).round() as i64,
            ((nh - h) as f64 * fy).round() as i64,
        ))
    };
    match p_str(p, "mode", "addBorder") {
        "addBorder" => {
            let all = p_f64(p, "border", 20.0).max(0.0);
            let side =
                |k: &str| (opt_f64(p, k).unwrap_or(all).clamp(0.0, 10000.0) * scale).round() as i64;
            let (t, r, b, l) = (side("top"), side("right"), side("bottom"), side("left"));
            Some((w + l + r, h + t + b, l, t))
        }
        "toSize" => {
            let nw = ((p_f64(p, "width", 1600.0).max(1.0)) * scale)
                .round()
                .max(1.0) as i64;
            let nh = ((p_f64(p, "height", 1600.0).max(1.0)) * scale)
                .round()
                .max(1.0) as i64;
            placed(nw, nh)
        }
        "toAspect" => {
            let ratio = ratio_of(p)?;
            let (mut nw, mut nh) = (w, h);
            if (w as f64 / h.max(1) as f64) > ratio {
                nh = (w as f64 / ratio).round() as i64;
            } else {
                nw = (h as f64 * ratio).round() as i64;
            }
            placed(nw.max(w), nh.max(h))
        }
        _ => {
            let s = w.max(h);
            placed(s, s)
        }
    }
}

impl Step for Canvas {
    fn id(&self) -> &str {
        CANVAS
    }
    fn group(&self) -> StepGroup {
        StepGroup::Geometry
    }
    fn params(&self) -> Vec<ParamDef> {
        let mut v = vec![
            ParamDef::choice("mode", &CANVAS_MODES, "addBorder").beginner(),
            ParamDef::int("border", 0, 10000, 20)
                .beginner()
                .unit("px")
                .when("mode", &["addBorder"]),
            opt_param("top", ParamType::Int { min: 0, max: 10000 }, Some("px"))
                .when("mode", &["addBorder"]),
            opt_param("right", ParamType::Int { min: 0, max: 10000 }, Some("px"))
                .when("mode", &["addBorder"]),
            opt_param("bottom", ParamType::Int { min: 0, max: 10000 }, Some("px"))
                .when("mode", &["addBorder"]),
            opt_param("left", ParamType::Int { min: 0, max: 10000 }, Some("px"))
                .when("mode", &["addBorder"]),
            ParamDef::int("width", 1, MAX_SIDE, 1600)
                .beginner()
                .unit("px")
                .when("mode", &["toSize"]),
            ParamDef::int("height", 1, MAX_SIDE, 1600)
                .beginner()
                .unit("px")
                .when("mode", &["toSize"]),
        ];
        v.extend(ratio_params(false, &["toAspect"]));
        v.extend([
            ParamDef::color("color", "transparent").beginner(),
            anchor_param().when("mode", &["toSize", "toAspect", "square"]),
        ]);
        v
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let scale = preview_scale(item, ctx);
        item.flatten_document()?;
        let (w, h) = item.dimensions();
        if w == 0 || h == 0 {
            return Ok(());
        }
        let Some((nw, nh, ox, oy)) = canvas_geometry(w as i64, h as i64, params, scale) else {
            return Ok(());
        };
        let bg = p_color(params, "color", CLEAR);
        if (nw, nh, ox, oy) == (w as i64, h as i64, 0, 0) && bg[3] == 0 {
            return Ok(());
        }
        if nw > MAX_SIDE * 4
            || nh > MAX_SIDE * 4
            || (nw as u128) * (nh as u128) > fp_raster::MAX_PIXELS as u128
        {
            return Err(too_large(nw, nh));
        }
        for img in frames_mut(item)? {
            ctx.cancel.check()?;
            *img = place(img, nw, nh, ox, oy, bg)?;
        }
        finish(item);
        refresh_transparency(item);
        Ok(())
    }
    fn summary(&self, p: &Value) -> String {
        let mode = p_str(p, "mode", "addBorder");
        fp_i18n::tr!(
            &format!("batch-summary-canvas-{mode}"),
            border = p_i64(p, "border", 20),
            width = p_i64(p, "width", 1600),
            height = p_i64(p, "height", 1600)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exif_roundtrip_le_be() {
        // minimale TIFF-Daten mit einem Eintrag 0x0112 = 6
        for be in [false, true] {
            let mut t: Vec<u8> = if be {
                b"MM\0*".to_vec()
            } else {
                b"II*\0".to_vec()
            };
            let w16 = |v: u16| if be { v.to_be_bytes() } else { v.to_le_bytes() };
            let w32 = |v: u32| if be { v.to_be_bytes() } else { v.to_le_bytes() };
            t.extend(w32(8));
            t.extend(w16(1));
            t.extend(w16(0x0112));
            t.extend(w16(3));
            t.extend(w32(1));
            t.extend(w16(6));
            t.extend([0, 0]);
            t.extend(w32(0));
            let mut exif = b"Exif\0\0".to_vec();
            exif.extend(&t);
            assert_eq!(exif_orientation(&exif), Some(6));
            assert!(set_exif_orientation(&mut exif, 1));
            assert_eq!(exif_orientation(&exif), Some(1));
        }
        assert_eq!(exif_orientation(b"nothing"), None);
        assert_eq!(exif_orientation(&[]), None);
    }

    #[test]
    fn contributions_sum_to_one() {
        for (s, d) in [(10usize, 3usize), (3, 10), (1, 7), (7, 1), (1000, 999)] {
            for f in [
                ResizeFilter::Bilinear,
                ResizeFilter::Bicubic,
                ResizeFilter::Lanczos3,
            ] {
                for c in contributions(s, d, f) {
                    let sum: f32 = c.weights.iter().sum();
                    assert!((sum - 1.0).abs() < 1e-4, "{s}->{d} {f:?}: {sum}");
                    assert!(c.start + c.weights.len() <= s);
                }
            }
        }
    }
}
