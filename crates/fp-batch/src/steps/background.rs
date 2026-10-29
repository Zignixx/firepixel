//! Gruppe „Hintergrund“ (SPEC-BATCH §3.5, Paket BATCH-6):
//!
//! - `background.remove` – Hintergrund entfernen mit KI (BiRefNet über `fp-ai`, Feineinstellungen
//!   wie `select.aiSubject`: Toleranz, Kante, Ausdehnen, Glätten, Kante verfeinern); Ausgabe
//!   transparent, nur Maske oder Maske als Alpha; Kantenfarben entmischen.
//! - `background.removeColor` – Farbe entfernen ohne KI (Flutfüllung von den Rändern oder überall,
//!   Toleranz, weiche Kante, Entmischen mit der Schlüsselfarbe).
//! - `background.flatten` – Transparenz → Farbe/Verlauf/Bild/unscharfe Kopie, mit
//!   Kantenbehandlung (normal mischen, Matte-Farbe entmischen, harte Kante) und optional in linearem
//!   Licht.
//! - `background.replace` – Hintergrund entfernen (KI oder Farbe) und ersetzen in einem Schritt.
//! - `background.shadow` – Schatten (Schlagschatten bzw. Bodenschatten für Produktbilder).
//!
//! Die Pixelhilfen (Fließkomma-Bild, Weichzeichnen, Skalieren, Drehen, Mischen, Bild-Parameter)
//! nutzt auch `overlay.rs`.

use crate::error::{BatchError, Result};
use crate::executor::CancelToken;
use crate::item::{Item, ItemKind};
use crate::step::{
    Issue, ParamDef, Severity, Step, StepContext, StepGroup, StepRegistry, parse_color,
};
use fp_ai::{AiError, ModelId};
use fp_raster::{IRect, RgbaImage};
use rayon::prelude::*;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

/// ID „Hintergrund entfernen (KI)“.
pub const REMOVE: &str = "background.remove";
/// ID „Farbe entfernen (ohne KI)“.
pub const REMOVE_COLOR: &str = "background.removeColor";
/// ID „Transparenz → Farbe“.
pub const FLATTEN: &str = "background.flatten";
/// ID „Hintergrund ersetzen“.
pub const REPLACE: &str = "background.replace";
/// ID „Schatten hinzufügen“.
pub const SHADOW: &str = "background.shadow";

/// Registriert die Schritte der Gruppe.
pub fn register(r: &mut StepRegistry) {
    r.register(Arc::new(Remove));
    r.register(Arc::new(RemoveColor));
    r.register(Arc::new(Flatten));
    r.register(Arc::new(Replace));
    r.register(Arc::new(Shadow));
}

// ------------------------------------------------------------------ Parameter lesen

pub(crate) fn p_str<'a>(p: &'a Value, k: &str, d: &'a str) -> &'a str {
    p.get(k).and_then(Value::as_str).unwrap_or(d)
}
pub(crate) fn p_f64(p: &Value, k: &str, d: f64) -> f64 {
    p.get(k)
        .and_then(Value::as_f64)
        .filter(|v| v.is_finite())
        .unwrap_or(d)
}
pub(crate) fn p_bool(p: &Value, k: &str, d: bool) -> bool {
    p.get(k).and_then(Value::as_bool).unwrap_or(d)
}
pub(crate) fn p_color(p: &Value, k: &str, d: [u8; 4]) -> [u8; 4] {
    p.get(k)
        .and_then(Value::as_str)
        .and_then(parse_color)
        .unwrap_or(d)
}

/// Maßstab der Vorschau: Pixelangaben (Rahmenbreite, Schattenabstand …) werden im verkleinerten
/// Proxy-Bild mitverkleinert, damit die Vorschau wie das Ergebnis aussieht.
pub(crate) fn preview_scale(ctx: &StepContext, w: usize, h: usize) -> f64 {
    if let Some(s) = ctx.preview_scale() {
        return s.clamp(0.01, 1.0); // BATCH-11a: Maßstab aus der Vorschau-Pipeline
    }
    let src = ctx.info.width.max(ctx.info.height) as f64;
    if !ctx.preview || src <= 0.0 {
        return 1.0;
    }
    ((w.max(h) as f64) / src).clamp(0.01, 1.0)
}

// ------------------------------------------------------------------ Fließkomma-Bild (vormultipliziert)

/// Bild mit vormultiplizierten Kanälen 0…1 (für Weichzeichnen, Skalieren, Drehen).
#[derive(Clone, Debug)]
pub(crate) struct Pm {
    pub w: usize,
    pub h: usize,
    pub px: Vec<[f32; 4]>,
}

impl Pm {
    pub fn from_rgba(img: &RgbaImage) -> Pm {
        let px = img
            .data()
            .iter()
            .map(|p| {
                let a = p[3] as f32 / 255.0;
                [
                    p[0] as f32 / 255.0 * a,
                    p[1] as f32 / 255.0 * a,
                    p[2] as f32 / 255.0 * a,
                    a,
                ]
            })
            .collect();
        Pm {
            w: img.width(),
            h: img.height(),
            px,
        }
    }

    pub fn to_rgba(&self) -> RgbaImage {
        let data: Vec<[u8; 4]> = self.px.iter().map(|p| to_u8(unpremul(*p))).collect();
        RgbaImage::from_raw(IRect::new(0, 0, self.w as i32, self.h as i32), data)
            .unwrap_or_else(|_| RgbaImage::empty_at(0, 0))
    }

    /// Bilinear an (x, y) (Pixelmitten bei ganzzahligen Koordinaten); außerhalb transparent bzw.
    /// am Rand fortgesetzt (`clamp`).
    pub fn sample(&self, x: f32, y: f32, clamp: bool) -> [f32; 4] {
        if self.w == 0 || self.h == 0 {
            return [0.0; 4];
        }
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = (x - x0, y - y0);
        let (x0, y0) = (x0 as i64, y0 as i64);
        let get = |xi: i64, yi: i64| -> [f32; 4] {
            if clamp {
                let xi = xi.clamp(0, self.w as i64 - 1) as usize;
                let yi = yi.clamp(0, self.h as i64 - 1) as usize;
                self.px[yi * self.w + xi]
            } else if xi < 0 || yi < 0 || xi >= self.w as i64 || yi >= self.h as i64 {
                [0.0; 4]
            } else {
                self.px[yi as usize * self.w + xi as usize]
            }
        };
        let (a, b, c, d) = (
            get(x0, y0),
            get(x0 + 1, y0),
            get(x0, y0 + 1),
            get(x0 + 1, y0 + 1),
        );
        let mut o = [0.0f32; 4];
        for i in 0..4 {
            let top = a[i] + (b[i] - a[i]) * fx;
            let bot = c[i] + (d[i] - c[i]) * fx;
            o[i] = top + (bot - top) * fy;
        }
        o
    }
}

pub(crate) fn unpremul(p: [f32; 4]) -> [f32; 4] {
    if p[3] <= 1e-6 {
        [0.0; 4]
    } else {
        [p[0] / p[3], p[1] / p[3], p[2] / p[3], p[3]]
    }
}

pub(crate) fn to_u8(p: [f32; 4]) -> [u8; 4] {
    let c = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    let a = c(p[3]);
    if a == 0 {
        [0, 0, 0, 0]
    } else {
        [c(p[0]), c(p[1]), c(p[2]), a]
    }
}

pub(crate) fn to_f(p: [u8; 4]) -> [f32; 4] {
    [
        p[0] as f32 / 255.0,
        p[1] as f32 / 255.0,
        p[2] as f32 / 255.0,
        p[3] as f32 / 255.0,
    ]
}

fn srgb_to_linear(v: f32) -> f32 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(v: f32) -> f32 {
    if v <= 0.003_130_8 {
        v * 12.92
    } else {
        1.055 * v.max(0.0).powf(1.0 / 2.4) - 0.055
    }
}

/// „Über“-Mischung zweier nicht vormultiplizierter Pixel (0…1); `linear` mischt in linearem Licht.
pub(crate) fn over(top: [f32; 4], bottom: [f32; 4], linear: bool) -> [f32; 4] {
    let (ta, ba) = (top[3], bottom[3]);
    let oa = ta + ba * (1.0 - ta);
    if oa <= 1e-6 {
        return [0.0; 4];
    }
    let mut o = [0.0f32; 4];
    for i in 0..3 {
        let (t, b) = if linear {
            (srgb_to_linear(top[i]), srgb_to_linear(bottom[i]))
        } else {
            (top[i], bottom[i])
        };
        let v = (t * ta + b * ba * (1.0 - ta)) / oa;
        o[i] = if linear { linear_to_srgb(v) } else { v };
    }
    o[3] = oa;
    o
}

/// Separierbarer Kastenfilter (Radius `r`) über `c` Kanäle; außerhalb 0 bzw. Rand fortgesetzt.
fn box_pass(buf: &mut [f32], w: usize, h: usize, c: usize, r: usize, clamp: bool) {
    if r == 0 || w == 0 || h == 0 {
        return;
    }
    let norm = 1.0 / (2 * r + 1) as f32;
    buf.par_chunks_mut(w * c).take(h).for_each(|row| {
        let src = row.to_vec();
        let get = |i: i64, ch: usize| -> f32 {
            if i < 0 || i >= w as i64 {
                if clamp {
                    src[(i.clamp(0, w as i64 - 1) as usize) * c + ch]
                } else {
                    0.0
                }
            } else {
                src[i as usize * c + ch]
            }
        };
        for ch in 0..c {
            let mut sum = 0.0f32;
            for i in -(r as i64)..=(r as i64) {
                sum += get(i, ch);
            }
            for x in 0..w {
                row[x * c + ch] = sum * norm;
                sum += get(x as i64 + r as i64 + 1, ch) - get(x as i64 - r as i64, ch);
            }
        }
    });
}

fn transpose(buf: &[f32], w: usize, h: usize, c: usize) -> Vec<f32> {
    let mut out = vec![0.0f32; buf.len()];
    out.par_chunks_mut(h * c).enumerate().for_each(|(x, col)| {
        for y in 0..h {
            col[y * c..y * c + c].copy_from_slice(&buf[(y * w + x) * c..(y * w + x) * c + c]);
        }
    });
    out
}

/// Gauß-ähnliches Weichzeichnen (drei Kastenfilter) mit „Radius“ ≈ 2 σ.
pub(crate) fn gauss(buf: &mut Vec<f32>, w: usize, h: usize, c: usize, radius: f32, clamp: bool) {
    if radius < 0.5 || w == 0 || h == 0 {
        return;
    }
    let sigma = radius / 2.0;
    let r = (((4.0 * sigma * sigma + 1.0).sqrt() - 1.0) / 2.0)
        .round()
        .max(1.0) as usize;
    for _ in 0..3 {
        box_pass(buf, w, h, c, r, clamp);
    }
    let mut t = transpose(buf, w, h, c);
    for _ in 0..3 {
        box_pass(&mut t, h, w, c, r, clamp);
    }
    *buf = transpose(&t, h, w, c);
}

/// Weichzeichnen eines Bildes (vormultipliziert, Rand fortgesetzt).
pub(crate) fn blur_image(img: &RgbaImage, radius: f32) -> RgbaImage {
    let pm = Pm::from_rgba(img);
    let mut flat: Vec<f32> = pm.px.iter().flat_map(|p| p.iter().copied()).collect();
    gauss(&mut flat, pm.w, pm.h, 4, radius, true);
    let px = flat.as_chunks::<4>().0.to_vec();
    Pm {
        w: pm.w,
        h: pm.h,
        px,
    }
    .to_rgba()
}

/// Skaliert auf `w × h` (vormultipliziert; Verkleinern mit Flächenmittel, sonst bilinear).
pub(crate) fn resize(img: &RgbaImage, w: u32, h: u32) -> RgbaImage {
    let (w, h) = (w.max(1) as usize, h.max(1) as usize);
    let mut pm = Pm::from_rgba(img);
    if pm.w == 0 || pm.h == 0 {
        return RgbaImage::empty_at(0, 0);
    }
    // ganzzahlige Vorverkleinerung (Flächenmittel), damit nichts flimmert
    let fx = (pm.w / w).max(1);
    let fy = (pm.h / h).max(1);
    if fx > 1 || fy > 1 {
        let (nw, nh) = (pm.w / fx, pm.h / fy);
        let mut px = vec![[0.0f32; 4]; nw * nh];
        px.par_chunks_mut(nw).enumerate().for_each(|(y, row)| {
            for (x, o) in row.iter_mut().enumerate() {
                let mut s = [0.0f32; 4];
                for yy in 0..fy {
                    for xx in 0..fx {
                        let p = pm.px[(y * fy + yy) * pm.w + x * fx + xx];
                        for i in 0..4 {
                            s[i] += p[i];
                        }
                    }
                }
                let n = (fx * fy) as f32;
                *o = [s[0] / n, s[1] / n, s[2] / n, s[3] / n];
            }
        });
        pm = Pm { w: nw, h: nh, px };
    }
    let (sx, sy) = (pm.w as f32 / w as f32, pm.h as f32 / h as f32);
    let mut px = vec![[0.0f32; 4]; w * h];
    px.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
        for (x, o) in row.iter_mut().enumerate() {
            *o = pm.sample(
                (x as f32 + 0.5) * sx - 0.5,
                (y as f32 + 0.5) * sy - 0.5,
                true,
            );
        }
    });
    Pm { w, h, px }.to_rgba()
}

/// Dreht um `deg` Grad (im Uhrzeigersinn, Bild wächst auf die Hülle, außen transparent).
pub(crate) fn rotate(img: &RgbaImage, deg: f64) -> RgbaImage {
    let d = deg.rem_euclid(360.0);
    if d.abs() < 1e-6 || (360.0 - d).abs() < 1e-6 || img.is_empty() {
        return img.clone();
    }
    let pm = Pm::from_rgba(img);
    let (s, c) = d.to_radians().sin_cos();
    let (w, h) = (pm.w as f64, pm.h as f64);
    let nw = (w * c.abs() + h * s.abs() - 1e-6).ceil().max(1.0) as usize;
    let nh = (w * s.abs() + h * c.abs() - 1e-6).ceil().max(1.0) as usize;
    let (cx, cy) = (w / 2.0, h / 2.0);
    let (ncx, ncy) = (nw as f64 / 2.0, nh as f64 / 2.0);
    let mut px = vec![[0.0f32; 4]; nw * nh];
    px.par_chunks_mut(nw).enumerate().for_each(|(y, row)| {
        for (x, o) in row.iter_mut().enumerate() {
            let (dx, dy) = (x as f64 + 0.5 - ncx, y as f64 + 0.5 - ncy);
            // Rückwärtsabbildung (Drehung um −deg)
            let sx = dx * c + dy * s + cx - 0.5;
            let sy = -dx * s + dy * c + cy - 0.5;
            *o = pm.sample(sx as f32, sy as f32, false);
        }
    });
    Pm { w: nw, h: nh, px }.to_rgba()
}

/// Mischmodi für Überlagerungen (Teilmenge des Registers, nach W3C-Compositing).
pub const BLEND_MODES: [&str; 10] = [
    "normal",
    "multiply",
    "screen",
    "overlay",
    "softLight",
    "hardLight",
    "darken",
    "lighten",
    "difference",
    "exclusion",
];

fn blend_channel(mode: &str, b: f32, s: f32) -> f32 {
    let hard = |b: f32, s: f32| {
        if s <= 0.5 {
            b * 2.0 * s
        } else {
            let t = 2.0 * s - 1.0;
            b + t - b * t
        }
    };
    match mode {
        "multiply" => b * s,
        "screen" => b + s - b * s,
        "overlay" => hard(s, b),
        "hardLight" => hard(b, s),
        "softLight" => {
            if s <= 0.5 {
                b - (1.0 - 2.0 * s) * b * (1.0 - b)
            } else {
                let d = if b <= 0.25 {
                    ((16.0 * b - 12.0) * b + 4.0) * b
                } else {
                    b.sqrt()
                };
                b + (2.0 * s - 1.0) * (d - b)
            }
        }
        "darken" => b.min(s),
        "lighten" => b.max(s),
        "difference" => (b - s).abs(),
        "exclusion" => b + s - 2.0 * b * s,
        _ => s,
    }
}

/// Mischt `stamp` an (ox, oy) auf `dst` mit Deckkraft 0…1 und Mischmodus.
pub(crate) fn blend_onto(
    dst: &mut RgbaImage,
    stamp: &RgbaImage,
    ox: i32,
    oy: i32,
    opacity: f32,
    mode: &str,
) {
    if opacity <= 0.0 || stamp.is_empty() || dst.is_empty() {
        return;
    }
    let (dw, dh) = (dst.width() as i32, dst.height() as i32);
    let (sw, sh) = (stamp.width() as i32, stamp.height() as i32);
    let x0 = ox.max(0);
    let x1 = (ox + sw).min(dw);
    let y0 = oy.max(0);
    let y1 = (oy + sh).min(dh);
    if x0 >= x1 || y0 >= y1 {
        return;
    }
    let sdata = stamp.data();
    let op = opacity.clamp(0.0, 1.0);
    let rows = &mut dst.data_mut()[(y0 * dw) as usize..(y1 * dw) as usize];
    rows.par_chunks_mut(dw as usize)
        .enumerate()
        .for_each(|(i, row)| {
            let y = y0 + i as i32;
            let sy = (y - oy) as usize;
            for x in x0..x1 {
                let s = sdata[sy * sw as usize + (x - ox) as usize];
                if s[3] == 0 {
                    continue;
                }
                let mut sf = to_f(s);
                sf[3] *= op;
                let d = &mut row[x as usize];
                let bf = to_f(*d);
                if mode != "normal" && bf[3] > 0.0 {
                    for c in 0..3 {
                        sf[c] = (1.0 - bf[3]) * sf[c] + bf[3] * blend_channel(mode, bf[c], sf[c]);
                    }
                }
                *d = to_u8(over(sf, bf, false));
            }
        });
}

// ------------------------------------------------------------------ Bilder aus Parametern

/// Base64 (Standardalphabet, Auffüllung und Leerraum erlaubt).
pub fn base64_decode(s: &str) -> Option<Vec<u8>> {
    let s = s.split_once("base64,").map(|(_, b)| b).unwrap_or(s);
    let val = |c: u8| -> Option<u32> {
        Some(match c {
            b'A'..=b'Z' => (c - b'A') as u32,
            b'a'..=b'z' => (c - b'a' + 26) as u32,
            b'0'..=b'9' => (c - b'0' + 52) as u32,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            _ => return None,
        })
    };
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0u32);
    for c in s.bytes() {
        if c == b'=' {
            break;
        }
        if c.is_ascii_whitespace() {
            continue;
        }
        acc = (acc << 6) | val(c)?;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    Some(out)
}

fn image_cache() -> &'static Mutex<HashMap<u64, Arc<RgbaImage>>> {
    static C: OnceLock<Mutex<HashMap<u64, Arc<RgbaImage>>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(HashMap::new()))
}

fn fnv(parts: &[&[u8]]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for p in parts {
        for b in *p {
            h ^= *b as u64;
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
        h = h.rotate_left(7);
    }
    h
}

/// Bild aus den Parametern `path_key` (Datei) bzw. `data_key` (im Rezept eingebettet, Base64);
/// einmal je Datei/Inhalt dekodiert und zwischengespeichert. `None` = kein Bild angegeben.
pub(crate) fn param_image(
    p: &Value,
    path_key: &str,
    data_key: &str,
) -> Result<Option<Arc<RgbaImage>>> {
    let data = p
        .get(data_key)
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or("");
    let path = p
        .get(path_key)
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or("");
    let (key, name, bytes): (u64, String, Option<Vec<u8>>) = if !data.is_empty() {
        (
            fnv(&[b"data", data.as_bytes()]),
            "embedded.png".into(),
            None,
        )
    } else if !path.is_empty() {
        let md = std::fs::metadata(path).map_err(|e| {
            BatchError::Invalid(fp_i18n::tr!(
                "batch-bg-err-image-read",
                path = path.to_string(),
                error = e.to_string()
            ))
        })?;
        let stamp = md
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        (
            fnv(&[
                b"path",
                path.as_bytes(),
                &stamp.to_le_bytes(),
                &md.len().to_le_bytes(),
            ]),
            path.to_string(),
            None,
        )
    } else {
        return Ok(None);
    };
    if let Ok(c) = image_cache().lock()
        && let Some(img) = c.get(&key)
    {
        return Ok(Some(img.clone()));
    }
    let bytes = match bytes {
        Some(b) => b,
        None if !data.is_empty() => base64_decode(data)
            .ok_or_else(|| BatchError::Invalid(fp_i18n::t("batch-bg-err-image-data")))?,
        None => std::fs::read(path).map_err(|e| {
            BatchError::Invalid(fp_i18n::tr!(
                "batch-bg-err-image-read",
                path = path.to_string(),
                error = e.to_string()
            ))
        })?,
    };
    let item = crate::item::decode_bytes(&name, &bytes)?;
    let mut img = item.preview_image()?;
    img.set_origin(0, 0);
    let img = Arc::new(img);
    if let Ok(mut c) = image_cache().lock() {
        if c.len() >= 32 {
            c.clear();
        }
        c.insert(key, img.clone());
    }
    Ok(Some(img))
}

// ------------------------------------------------------------------ KI-Maske

fn model_id(name: &str) -> ModelId {
    if name == "birefnet" {
        ModelId::BiRefNet
    } else {
        ModelId::BiRefNetLite
    }
}

pub(crate) fn model_missing(step: &str, model: ModelId) -> BatchError {
    BatchError::step(
        step,
        fp_i18n::tr!(
            "batch-bg-err-model-missing",
            model = model.display_name(),
            folder = fp_ai::models::download_folder().display().to_string()
        ),
    )
}

/// Feineinstellungen wie bei `select.aiSubject` aus den Parametern.
pub fn tuning_from(p: &Value) -> fp_ai::Tuning {
    fp_ai::Tuning {
        tolerance: p_f64(p, "tolerance", 50.0) as f32,
        variant: fp_ai::Variant::Auto,
        edge: fp_ai::Edge::from_name(p_str(p, "edge", "antiAlias")).unwrap_or_default(),
        feather: p_f64(p, "feather", 4.0).max(0.0) as u32,
        expand: p_f64(p, "expand", 0.0) as i32,
        smooth: p_f64(p, "smooth", 0.0).max(0.0) as u32,
    }
    .clamped()
}

/// Motiv-Maske (0…1 je Pixel, Bildgröße) über BiRefNet. Fehlt das Modell → Fehler „KI-Modell
/// fehlt“ (es wird nie etwas heruntergeladen). Das Netz rechnet je Bild einmal; geänderte
/// Feineinstellungen nutzen die zwischengespeicherte Ausgabe von `fp-ai`.
pub fn ai_subject_mask(
    img: &RgbaImage,
    p: &Value,
    cancel: &CancelToken,
    warnings: &mut Vec<String>,
    step: &str,
) -> Result<Vec<f32>> {
    let model = model_id(p_str(p, "model", "birefnetLite"));
    if !fp_ai::models::available(model) {
        return Err(model_missing(step, model));
    }
    let t = tuning_from(p);
    let matting = ModelId::BiRefNetLiteMatting;
    let mut refine = p_bool(p, "refineEdge", false);
    if refine && !fp_ai::models::available(matting) {
        warnings.push(fp_i18n::tr!(
            "batch-bg-warn-matting-missing",
            model = matting.display_name()
        ));
        refine = false;
    }
    cancel.check()?;
    let ai_cancel = fp_ai::Cancel::new();
    let prog = fp_ai::Progress::with_cancel(ai_cancel.clone());
    // CT-C-01: Das Netz rechnet in einem eigenen, gewöhnlichen Thread. Ruft ein Arbeits-Thread des
    // Batch-Pools `fp_ai::nn::pool().install` direkt auf, holt rayon ihm während des Wartens andere
    // Elemente des Batch-Pools auf denselben Stapel (Fremdpool-Warten). Die blockierten dann im
    // Speicherbudget bzw. an der Ladesperre des Modells, die der wartende Rahmen darunter hält –
    // der Lauf stand (auch „Abbrechen“ kam nie an). Dieser Thread wartet nur und reicht den Abbruch weiter.
    let lang = fp_i18n::current_language();
    let res = std::thread::scope(|s| {
        let worker = s.spawn(|| {
            fp_i18n::with_language(
                &lang,
                || -> std::result::Result<fp_raster::Mask, AiError> {
                    let raw = fp_ai::pipeline::subject_raw(img, None, model, &prog)?;
                    let m = fp_ai::pipeline::subject_from_raw(&raw, img, &t)?;
                    if refine {
                        fp_ai::pipeline::refine_edge(img, &m, 0.0, matting, &prog)
                    } else {
                        Ok(m)
                    }
                },
            )
        });
        while !worker.is_finished() {
            if cancel.is_cancelled() {
                ai_cancel.cancel();
            }
            std::thread::sleep(std::time::Duration::from_millis(15));
        }
        worker
            .join()
            .unwrap_or_else(|_| Err(AiError::Input(fp_i18n::t("batch-err-worker-panic"))))
    });
    let m = res.map_err(|e| match e {
        AiError::Cancelled => BatchError::cancelled(),
        AiError::ModelMissing { model, .. } => model_missing(step, model),
        e => BatchError::step(step, e.to_string()),
    })?;
    let (w, h) = (img.width(), img.height());
    let mut out = vec![0.0f32; w * h];
    out.par_chunks_mut(w.max(1))
        .enumerate()
        .for_each(|(y, row)| {
            for (x, v) in row.iter_mut().enumerate() {
                *v = m.get(x as i32, y as i32) as f32 / 255.0;
            }
        });
    Ok(out)
}

/// Radius (px) of the window for the local background and subject colours of an edge pixel.
const LOCAL_RADIUS: usize = 16;

/// Alpha-weighted local mean colours from a summed-area table `(w + 1) × (h + 1)` of
/// `[a·r, a·g, a·b, a]` over the pixels a predicate on the mask picks.
struct LocalMean {
    w: usize,
    h: usize,
    sat: Vec<[f64; 4]>,
}

impl LocalMean {
    fn new(img: &RgbaImage, mask: &[f32], pick: impl Fn(f32) -> bool) -> LocalMean {
        let (w, h) = (img.width(), img.height());
        let mut sat = vec![[0.0f64; 4]; (w + 1) * (h + 1)];
        let data = img.data();
        for y in 0..h {
            let mut row = [0.0f64; 4];
            for x in 0..w {
                let i = y * w + x;
                let p = data[i];
                if pick(mask[i]) && p[3] > 0 {
                    let a = p[3] as f64 / 255.0;
                    for c in 0..3 {
                        row[c] += p[c] as f64 / 255.0 * a;
                    }
                    row[3] += a;
                }
                let up = sat[y * (w + 1) + x + 1];
                sat[(y + 1) * (w + 1) + x + 1] = std::array::from_fn(|k| up[k] + row[k]);
            }
        }
        LocalMean { w, h, sat }
    }

    fn mean(v: [f64; 4]) -> Option<[f32; 3]> {
        (v[3] >= 0.5).then(|| std::array::from_fn(|k| (v[k] / v[3]) as f32))
    }

    /// Mean over all picked pixels.
    fn global(&self) -> Option<[f32; 3]> {
        LocalMean::mean(self.sat[(self.h + 1) * (self.w + 1) - 1])
    }

    /// Mean in the window of [`LOCAL_RADIUS`] around pixel `i`.
    fn at(&self, i: usize) -> Option<[f32; 3]> {
        let (x, y) = (i % self.w, i / self.w);
        let r = LOCAL_RADIUS;
        let (x0, x1) = (x.saturating_sub(r), (x + r + 1).min(self.w));
        let (y0, y1) = (y.saturating_sub(r), (y + r + 1).min(self.h));
        let s = |xx: usize, yy: usize| self.sat[yy * (self.w + 1) + xx];
        let (a, b, c, d) = (s(x1, y1), s(x0, y1), s(x1, y0), s(x0, y0));
        LocalMean::mean(std::array::from_fn(|k| a[k] - b[k] - c[k] + d[k]))
    }
}

/// Colours for unmixing the edge (F143). Measured with the real BiRefNet on the test photos: one
/// global background mean made edges in front of a differently coloured part of the background
/// unmix into saturated fringes (cat photo: 794 edge pixels whose channel spread grew by more
/// than 100 levels, 143 of them at least 12 % opaque; hair photo 15 812 / 4 310). Now the
/// background colour is the local mean of the background pixels (mask < 5 %) around the pixel,
/// and the unmixed colour is blended with the local mean of the subject pixels (mask > 95 %) by
/// the mask value: `F = m·unmix(c, m, B) + (1 − m)·S`. Faint edge pixels, where unmixing amplifies
/// every error of the mask by `(1 − m)/m`, take the colour of the subject next to them.
struct EdgeColors {
    bg: LocalMean,
    fg: LocalMean,
    global_bg: [f32; 3],
}

impl EdgeColors {
    fn new(img: &RgbaImage, mask: &[f32]) -> Option<EdgeColors> {
        let bg = LocalMean::new(img, mask, |m| m < 0.05);
        let global_bg = bg.global()?;
        Some(EdgeColors {
            fg: LocalMean::new(img, mask, |m| m > 0.95),
            bg,
            global_bg,
        })
    }

    /// Subject colour of edge pixel `i` with colour `c` and mask `m`.
    fn unmix(&self, i: usize, c: [f32; 4], m: f32) -> [f32; 4] {
        let f = unmix(c, m, self.bg.at(i).unwrap_or(self.global_bg));
        match self.fg.at(i) {
            Some(s) => [
                m * f[0] + (1.0 - m) * s[0],
                m * f[1] + (1.0 - m) * s[1],
                m * f[2] + (1.0 - m) * s[2],
                c[3],
            ],
            None => f,
        }
    }
}

/// Entmischt eine Kantenfarbe: `c = m·F + (1−m)·B` → `F` (begrenzt).
fn unmix(c: [f32; 4], m: f32, b: [f32; 3]) -> [f32; 4] {
    let m = m.max(0.08);
    [
        ((c[0] - (1.0 - m) * b[0]) / m).clamp(0.0, 1.0),
        ((c[1] - (1.0 - m) * b[1]) / m).clamp(0.0, 1.0),
        ((c[2] - (1.0 - m) * b[2]) / m).clamp(0.0, 1.0),
        c[3],
    ]
}

/// Wendet eine Motiv-Maske (0…1) an. `output`: `transparent` (Alpha × Maske, Kantenfarben
/// optional entmischt mit der örtlichen Hintergrundfarbe), `mask` (Maske als deckendes Graubild),
/// `alpha` (Farben unverändert, Maske ersetzt das Alpha).
pub fn apply_subject_mask(img: &mut RgbaImage, mask: &[f32], output: &str, decontaminate: bool) {
    if mask.len() != img.data().len() {
        return;
    }
    let bg = if output == "transparent" && decontaminate {
        EdgeColors::new(img, mask)
    } else {
        None
    };
    img.data_mut()
        .par_iter_mut()
        .zip(mask.par_iter())
        .enumerate()
        .for_each(|(i, (p, m))| {
            let m = m.clamp(0.0, 1.0);
            match output {
                "mask" => {
                    let v = (m * 255.0).round() as u8;
                    *p = [v, v, v, 255];
                }
                "alpha" => {
                    p[3] = (m * 255.0).round() as u8;
                }
                _ => {
                    let mut f = to_f(*p);
                    if let Some(b) = &bg
                        && m > 0.0
                        && m < 0.98
                    {
                        f = b.unmix(i, f, m);
                    }
                    f[3] *= m;
                    *p = to_u8(f);
                }
            }
        });
}

// ------------------------------------------------------------------ Farbe entfernen

/// Schlüsselfarbe: Ecke (`topLeft` …) oder `custom` (= `color`).
fn key_color(img: &RgbaImage, p: &Value) -> [u8; 4] {
    let (w, h) = (img.width() as i32, img.height() as i32);
    let at = |x: i32, y: i32| img.get(x, y).unwrap_or([255, 255, 255, 255]);
    match p_str(p, "pick", "topLeft") {
        "topRight" => at(w - 1, 0),
        "bottomLeft" => at(0, h - 1),
        "bottomRight" => at(w - 1, h - 1),
        "custom" => p_color(p, "color", [255, 255, 255, 255]),
        _ => at(0, 0),
    }
}

fn color_dist(a: [u8; 4], k: [u8; 4]) -> u8 {
    if a[3] == 0 {
        return 0;
    }
    (0..3)
        .map(|i| (a[i] as i16 - k[i] as i16).unsigned_abs() as u8)
        .max()
        .unwrap_or(0)
}

/// Maske der zu entfernenden Pixel (1 = entfernen): Abstand (größte Kanaldifferenz) ≤ `tol`;
/// `contiguous` = nur von den Bildrändern aus zusammenhängend (Flutfüllung, 4er-Nachbarschaft).
/// Voll transparente Pixel gelten als passend.
pub fn color_key_mask(img: &RgbaImage, key: [u8; 4], tol: u8, contiguous: bool) -> Vec<f32> {
    let (w, h) = (img.width(), img.height());
    let data = img.data();
    let hit: Vec<bool> = data.iter().map(|p| color_dist(*p, key) <= tol).collect();
    if !contiguous {
        return hit.iter().map(|b| if *b { 1.0 } else { 0.0 }).collect();
    }
    let mut out = vec![0.0f32; w * h];
    let mut stack: Vec<usize> = Vec::new();
    let push = |i: usize, out: &mut Vec<f32>, stack: &mut Vec<usize>| {
        if hit[i] && out[i] == 0.0 {
            out[i] = 1.0;
            stack.push(i);
        }
    };
    for x in 0..w {
        push(x, &mut out, &mut stack);
        if h > 0 {
            push((h - 1) * w + x, &mut out, &mut stack);
        }
    }
    for y in 0..h {
        push(y * w, &mut out, &mut stack);
        if w > 0 {
            push(y * w + w - 1, &mut out, &mut stack);
        }
    }
    while let Some(i) = stack.pop() {
        let (x, y) = (i % w, i / w);
        if x > 0 {
            push(i - 1, &mut out, &mut stack);
        }
        if x + 1 < w {
            push(i + 1, &mut out, &mut stack);
        }
        if y > 0 {
            push(i - w, &mut out, &mut stack);
        }
        if y + 1 < h {
            push(i + w, &mut out, &mut stack);
        }
    }
    out
}

/// Entfernt die Schlüsselfarbe: Maske → weiche Kante (`feather` px, nur nach innen) → Alpha.
///
/// `decontaminate` behandelt den Kantensaum (weiche Kante und alle Pixel neben entfernten): Die
/// Motivfarbe `F` wird aus dem Inneren nach außen fortgesetzt, der Motivanteil eines Saumpixels
/// `C` ist die Projektion `t = (C−K)·(F−K) / |F−K|²` (K = Schlüsselfarbe). Das Pixel bekommt die
/// Farbe `F` und Alpha × min(weiche Kante, t) – so verschwindet ein heller/dunkler Rand, und
/// geglättete Kanten bleiben glatt.
pub fn remove_color(
    img: &mut RgbaImage,
    key: [u8; 4],
    tol: u8,
    contiguous: bool,
    feather: f32,
    decontaminate: bool,
) {
    let (w, h) = (img.width(), img.height());
    let rem = color_key_mask(img, key, tol, contiguous);
    let mut soft = rem.clone();
    if feather > 0.0 {
        gauss(&mut soft, w, h, 1, feather * 2.0, true);
        for (s, r) in soft.iter_mut().zip(&rem) {
            *s = s.max(*r);
        }
    }
    let n = w * h;
    // Saum: nicht ganz entfernt, aber weich oder neben einem entfernten Pixel
    let mut band = vec![false; n];
    let mut fg: Vec<Option<[f32; 3]>> = vec![None; n];
    if decontaminate {
        let data = img.data();
        for i in 0..n {
            if rem[i] >= 0.5 {
                continue;
            }
            let (x, y) = ((i % w) as i64, (i / w) as i64);
            let mut near = soft[i] > 0.0;
            for dy in -1..=1i64 {
                for dx in -1..=1i64 {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx >= 0
                        && ny >= 0
                        && nx < w as i64
                        && ny < h as i64
                        && rem[(ny as usize) * w + nx as usize] >= 0.5
                    {
                        near = true;
                    }
                }
            }
            band[i] = near;
            if !near && data[i][3] > 0 {
                let p = data[i];
                fg[i] = Some([
                    p[0] as f32 / 255.0,
                    p[1] as f32 / 255.0,
                    p[2] as f32 / 255.0,
                ]);
            }
        }
        // Motivfarbe ins Saumgebiet fortsetzen (Mittel der schon bekannten Nachbarn)
        let passes = (feather.ceil() as usize + 3).min(64);
        for _ in 0..passes {
            let prev = fg.clone();
            let mut changed = false;
            for i in 0..n {
                if !band[i] || prev[i].is_some() {
                    continue;
                }
                let (x, y) = ((i % w) as i64, (i / w) as i64);
                let (mut s, mut c) = ([0.0f32; 3], 0.0f32);
                for dy in -1..=1i64 {
                    for dx in -1..=1i64 {
                        let (nx, ny) = (x + dx, y + dy);
                        if nx >= 0
                            && ny >= 0
                            && nx < w as i64
                            && ny < h as i64
                            && let Some(f) = prev[(ny as usize) * w + nx as usize]
                        {
                            for k in 0..3 {
                                s[k] += f[k];
                            }
                            c += 1.0;
                        }
                    }
                }
                if c > 0.0 {
                    fg[i] = Some([s[0] / c, s[1] / c, s[2] / c]);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
    }
    let k = [
        key[0] as f32 / 255.0,
        key[1] as f32 / 255.0,
        key[2] as f32 / 255.0,
    ];
    img.data_mut()
        .par_iter_mut()
        .enumerate()
        .for_each(|(i, p)| {
            let mut keep = (1.0 - soft[i]).clamp(0.0, 1.0);
            let mut f = to_f(*p);
            if band[i]
                && let Some(fc) = fg[i]
            {
                let d = [fc[0] - k[0], fc[1] - k[1], fc[2] - k[2]];
                let len2 = d[0] * d[0] + d[1] * d[1] + d[2] * d[2];
                if len2 > 1e-4 {
                    let t =
                        ((f[0] - k[0]) * d[0] + (f[1] - k[1]) * d[1] + (f[2] - k[2]) * d[2]) / len2;
                    keep = keep.min(t.clamp(0.0, 1.0));
                    f = [fc[0], fc[1], fc[2], f[3]];
                }
            }
            if keep >= 1.0 && !band[i] {
                return;
            }
            f[3] *= keep;
            *p = to_u8(f);
        });
}

// ------------------------------------------------------------------ Hintergrund füllen

/// Hintergrundbild in Bildgröße nach `kind` (color, gradient, image, blur).
fn background_layer(src: &RgbaImage, p: &Value) -> Result<Vec<[f32; 4]>> {
    let (w, h) = (src.width(), src.height());
    let base = to_f(p_color(p, "color", [255, 255, 255, 255]));
    let mut bg = vec![base; w * h];
    match p_str(p, "kind", "color") {
        "gradient" => {
            let c2 = to_f(p_color(p, "color2", [0, 0, 0, 255]));
            let radial = p_str(p, "gradientShape", "linear") == "radial";
            let ang = p_f64(p, "angle", 90.0).to_radians();
            let (dx, dy) = (ang.cos() as f32, ang.sin() as f32);
            // Ausdehnung des Bildes entlang der Richtung
            let (fw, fh) = (w as f32, h as f32);
            let ext = (fw * dx.abs() + fh * dy.abs()).max(1.0);
            let half_diag = ((fw * fw + fh * fh).sqrt() / 2.0).max(1.0);
            bg.par_chunks_mut(w.max(1))
                .enumerate()
                .for_each(|(y, row)| {
                    for (x, o) in row.iter_mut().enumerate() {
                        let (px, py) = (x as f32 + 0.5 - fw / 2.0, y as f32 + 0.5 - fh / 2.0);
                        let t = if radial {
                            (px * px + py * py).sqrt() / half_diag
                        } else {
                            (px * dx + py * dy) / ext + 0.5
                        };
                        let t = t.clamp(0.0, 1.0);
                        let mut c = [0.0f32; 4];
                        for i in 0..4 {
                            c[i] = base[i] + (c2[i] - base[i]) * t;
                        }
                        *o = c;
                    }
                });
        }
        "image" => {
            let Some(im) = param_image(p, "image", "imageData")? else {
                return Err(BatchError::Invalid(fp_i18n::t("batch-bg-err-no-image")));
            };
            let (iw, ih) = (im.width() as f64, im.height() as f64);
            let fit = p_str(p, "fit", "cover");
            let (mut ox, mut oy) = (0i64, 0i64);
            let layer = if fit == "stretch" {
                resize(&im, w as u32, h as u32)
            } else if fit == "tile" || iw <= 0.0 || ih <= 0.0 {
                (*im).clone()
            } else {
                // cover (Vorgabe) bzw. contain, mittig
                let s = if fit == "contain" {
                    (w as f64 / iw).min(h as f64 / ih)
                } else {
                    (w as f64 / iw).max(h as f64 / ih)
                };
                let (nw, nh) = (
                    (iw * s).round().max(1.0) as u32,
                    (ih * s).round().max(1.0) as u32,
                );
                ox = (w as i64 - nw as i64) / 2;
                oy = (h as i64 - nh as i64) / 2;
                resize(&im, nw, nh)
            };
            let (lw, lh) = (layer.width() as i64, layer.height() as i64);
            if lw > 0 && lh > 0 {
                let ld = layer.data();
                let tile = fit == "tile";
                bg.par_chunks_mut(w.max(1))
                    .enumerate()
                    .for_each(|(y, row)| {
                        for (x, o) in row.iter_mut().enumerate() {
                            let (mut lx, mut ly) = (x as i64 - ox, y as i64 - oy);
                            if tile {
                                lx = lx.rem_euclid(lw);
                                ly = ly.rem_euclid(lh);
                            }
                            if lx < 0 || ly < 0 || lx >= lw || ly >= lh {
                                continue;
                            }
                            *o = over(to_f(ld[(ly * lw + lx) as usize]), *o, false);
                        }
                    });
            }
        }
        "blur" => {
            let r = p_f64(p, "blur", 30.0).clamp(0.0, 250.0) as f32;
            let b = blur_image(src, r);
            bg.par_iter_mut()
                .zip(b.data().par_iter())
                .for_each(|(o, s)| *o = over(to_f(*s), *o, false));
        }
        _ => {}
    }
    Ok(bg)
}

/// Kantenbehandlung vor dem Füllen: `blend` (normal), `unmix` (Kantenfarben mit der Matte-Farbe
/// entmischen), `hard` (Alpha ≥ Schwelle → deckend, sonst transparent).
fn prepare_edge(f: [f32; 4], edge: &str, matte: [f32; 3], threshold: f32) -> [f32; 4] {
    match edge {
        "unmix" if f[3] > 0.0 && f[3] < 1.0 => unmix(f, f[3], matte),
        "hard" => {
            let mut g = f;
            g[3] = if f[3] >= threshold { 1.0 } else { 0.0 };
            g
        }
        _ => f,
    }
}

/// Füllt transparente Bereiche nach den Parametern von `background.flatten`.
pub fn flatten_image(img: &mut RgbaImage, p: &Value) -> Result<()> {
    if p_bool(p, "onlyIfTransparent", true) && img.data().iter().all(|px| px[3] == 255) {
        return Ok(());
    }
    let bg = background_layer(img, p)?;
    let edge = p_str(p, "edge", "blend").to_string();
    let m = p_color(p, "matte", [255, 255, 255, 255]);
    let matte = [
        m[0] as f32 / 255.0,
        m[1] as f32 / 255.0,
        m[2] as f32 / 255.0,
    ];
    let threshold = (p_f64(p, "threshold", 128.0).clamp(1.0, 255.0) / 255.0) as f32;
    let linear = p_bool(p, "linearLight", false);
    img.data_mut()
        .par_iter_mut()
        .zip(bg.par_iter())
        .for_each(|(px, b)| {
            let f = prepare_edge(to_f(*px), &edge, matte, threshold);
            *px = to_u8(over(f, *b, linear));
        });
    Ok(())
}

// ------------------------------------------------------------------ Schatten

/// Schatten unter das Bild legen (Alpha des Bildes als Form); wächst mit `expandCanvas`.
pub fn shadow_image(img: &RgbaImage, p: &Value, scale: f64) -> RgbaImage {
    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 {
        return img.clone();
    }
    let color = to_f(p_color(p, "color", [0, 0, 0, 255]));
    let opacity = (p_f64(p, "opacity", 50.0) / 100.0).clamp(0.0, 1.0) as f32;
    let blur = (p_f64(p, "blur", 8.0).clamp(0.0, 250.0) * scale) as f32;
    let dist = p_f64(p, "distance", 10.0).clamp(0.0, 1000.0) * scale;
    let ang = p_f64(p, "angle", 315.0).to_radians();
    let below = p_str(p, "style", "drop") == "below";
    let expand = p_bool(p, "expandCanvas", true);
    let (dx, dy) = if below {
        (0i64, 0i64)
    } else {
        (
            (dist * ang.cos()).round() as i64,
            (-dist * ang.sin()).round() as i64,
        )
    };
    let reach = (blur * 1.5).ceil() as i64 + 1;
    // Form des Schattens im Bildraum
    let alpha: Vec<f32> = img.data().iter().map(|q| q[3] as f32 / 255.0).collect();
    let mut shape = alpha.clone();
    let mut extra_bottom = 0i64;
    if below {
        // Bodenschatten: flache Ellipse unter dem Motiv
        let mut b = (w, h, 0usize, 0usize);
        for (i, a) in alpha.iter().enumerate() {
            if *a > 0.05 {
                let (x, y) = (i % w, i / w);
                b = (b.0.min(x), b.1.min(y), b.2.max(x), b.3.max(y));
            }
        }
        shape.iter_mut().for_each(|v| *v = 0.0);
        if b.2 >= b.0 && b.3 >= b.1 {
            let cx = (b.0 + b.2) as f64 / 2.0 + 0.5;
            let rx = ((b.2 - b.0 + 1) as f64 * 0.45).max(1.0);
            let ry = ((b.3 - b.1 + 1) as f64 * 0.05 + dist * 0.2).max(1.0);
            let cy = b.3 as f64 + 0.5;
            extra_bottom = ry.ceil() as i64;
            let h2 = h + extra_bottom.max(0) as usize;
            shape = vec![0.0; w * h2];
            for y in 0..h2 {
                for x in 0..w {
                    let (u, v) = ((x as f64 + 0.5 - cx) / rx, (y as f64 + 0.5 - cy) / ry);
                    let d = (u * u + v * v).sqrt();
                    shape[y * w + x] = (1.0 - d).clamp(0.0, 1.0).powf(0.7) as f32;
                }
            }
        }
    }
    let sh = shape.len() / w;
    let (l, t, r, bm) = if expand {
        (
            (reach - dx).max(0),
            (reach - dy).max(0),
            (reach + dx).max(0),
            (reach + dy).max(0) + extra_bottom,
        )
    } else {
        (0, 0, 0, 0)
    };
    let (nw, nh) = (w + (l + r) as usize, h + (t + bm) as usize);
    let mut plane = vec![0.0f32; nw * nh];
    for y in 0..sh {
        let ty = y as i64 + t + dy;
        if ty < 0 || ty >= nh as i64 {
            continue;
        }
        for x in 0..w {
            let tx = x as i64 + l + dx;
            if tx < 0 || tx >= nw as i64 {
                continue;
            }
            plane[ty as usize * nw + tx as usize] = shape[y * w + x];
        }
    }
    gauss(&mut plane, nw, nh, 1, blur, false);
    let mut out = vec![[0u8; 4]; nw * nh];
    out.par_chunks_mut(nw).enumerate().for_each(|(y, row)| {
        for (x, o) in row.iter_mut().enumerate() {
            let sa = plane[y * nw + x] * opacity * color[3];
            let under = [color[0], color[1], color[2], sa.clamp(0.0, 1.0)];
            let (ix, iy) = (x as i64 - l, y as i64 - t);
            let top = if ix >= 0 && iy >= 0 && (ix as usize) < w && (iy as usize) < h {
                to_f(img.data()[iy as usize * w + ix as usize])
            } else {
                [0.0; 4]
            };
            *o = to_u8(over(top, under, false));
        }
    });
    RgbaImage::from_raw(IRect::new(0, 0, nw as i32, nh as i32), out).unwrap_or_else(|_| img.clone())
}

// ------------------------------------------------------------------ Schritte

fn ai_params(beginner_model: bool) -> Vec<ParamDef> {
    let model = ParamDef::choice("model", &["birefnetLite", "birefnet"], "birefnetLite");
    vec![
        if beginner_model {
            model.beginner()
        } else {
            model
        },
        ParamDef::choice("edge", &["hard", "antiAlias", "feather"], "antiAlias").beginner(),
        ParamDef::int("feather", 0, 250, 4)
            .unit("px")
            .when("edge", &["feather"]),
        ParamDef::int("tolerance", 0, 100, 50),
        ParamDef::int("expand", -100, 100, 0).unit("px"),
        ParamDef::int("smooth", 0, 100, 0).unit("px"),
        ParamDef::bool("refineEdge", false),
        ParamDef::bool("decontaminate", true),
    ]
}

fn model_issue(p: &Value) -> Option<Issue> {
    let model = model_id(p_str(p, "model", "birefnetLite"));
    (!fp_ai::models::available(model)).then(|| {
        Issue::global(
            Severity::Warning,
            "aiModelMissing",
            fp_i18n::tr!(
                "batch-bg-err-model-missing",
                model = model.display_name(),
                folder = fp_ai::models::download_folder().display().to_string()
            ),
        )
    })
}

/// `background.remove`.
/// Gleichzeitig laufende Elemente mit KI-Schritt: das Modell rechnet selbst auf allen Kernen und
/// braucht je Bild mehrere 100 MB – mehr Elemente zugleich machen den Lauf nicht schneller, nur die
/// Restzeit unruhig und den Speicher knapp (BATCH-T: 24 Fotos liefen alle sofort „läuft“).
pub const AI_PARALLEL: usize = 2;

struct Remove;

impl Step for Remove {
    fn id(&self) -> &str {
        REMOVE
    }
    fn group(&self) -> StepGroup {
        StepGroup::Background
    }
    fn params(&self) -> Vec<ParamDef> {
        let mut v = ai_params(true);
        v.push(ParamDef::choice(
            "output",
            &["transparent", "mask", "alpha"],
            "transparent",
        ));
        v
    }
    fn validate(&self, params: &Value) -> Vec<Issue> {
        let mut v = crate::step::validate_params(&self.params(), params);
        v.extend(model_issue(params));
        v
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn max_parallel(&self, _params: &Value) -> Option<usize> {
        Some(AI_PARALLEL)
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let output = p_str(params, "output", "transparent").to_string();
        let deco = p_bool(params, "decontaminate", true);
        let cancel = ctx.cancel;
        let mut warnings = Vec::new();
        item.map_images(|img| {
            let mask = ai_subject_mask(img, params, cancel, &mut warnings, REMOVE)?;
            apply_subject_mask(img, &mask, &output, deco);
            Ok(())
        })?;
        warnings.dedup();
        ctx.warnings.extend(warnings);
        if let Some(m) = item.meta_mut() {
            m.has_transparency = output != "mask";
        }
        Ok(())
    }
    fn summary(&self, params: &Value) -> String {
        fp_i18n::t(&format!(
            "batch-bg-summary-output-{}",
            p_str(params, "output", "transparent")
        ))
    }
}

/// `background.removeColor`.
struct RemoveColor;

impl Step for RemoveColor {
    fn id(&self) -> &str {
        REMOVE_COLOR
    }
    fn group(&self) -> StepGroup {
        StepGroup::Background
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::choice(
                "pick",
                &["topLeft", "topRight", "bottomLeft", "bottomRight", "custom"],
                "topLeft",
            )
            .beginner(),
            ParamDef::color("color", "#ffffff")
                .beginner()
                .when("pick", &["custom"]),
            ParamDef::int("tolerance", 0, 255, 24).beginner(),
            ParamDef::bool("contiguous", true),
            ParamDef::number("feather", 0.0, 10.0, 0.0).unit("px"),
            ParamDef::bool("decontaminate", true),
        ]
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let tol = p_f64(params, "tolerance", 24.0).clamp(0.0, 255.0) as u8;
        let contiguous = p_bool(params, "contiguous", true);
        let deco = p_bool(params, "decontaminate", true);
        let feather = p_f64(params, "feather", 0.0).clamp(0.0, 10.0);
        let cancel = ctx.cancel;
        let info_long = ctx.info.width.max(ctx.info.height) as f64;
        let preview = ctx.preview;
        item.map_images(|img| {
            cancel.check()?;
            let key = key_color(img, params);
            let s = if preview && info_long > 0.0 {
                (img.width().max(img.height()) as f64 / info_long).min(1.0)
            } else {
                1.0
            };
            remove_color(img, key, tol, contiguous, (feather * s) as f32, deco);
            Ok(())
        })?;
        if let Some(m) = item.meta_mut() {
            m.has_transparency = true;
        }
        Ok(())
    }
}

fn fill_params() -> Vec<ParamDef> {
    vec![
        ParamDef::choice("kind", &["color", "gradient", "image", "blur"], "color").beginner(),
        ParamDef::color("color", "#ffffff")
            .beginner()
            .when("kind", &["color", "gradient"]),
        ParamDef::color("color2", "#000000").when("kind", &["gradient"]),
        ParamDef::choice("gradientShape", &["linear", "radial"], "linear")
            .when("kind", &["gradient"]),
        ParamDef::number("angle", -360.0, 360.0, 90.0)
            .unit("°")
            .when("gradientShape", &["linear"]),
        ParamDef::path("image").when("kind", &["image"]),
        ParamDef::text("imageData", "").when("kind", &["image"]),
        ParamDef::choice("fit", &["cover", "contain", "stretch", "tile"], "cover")
            .when("kind", &["image"]),
        ParamDef::number("blur", 0.0, 250.0, 30.0)
            .unit("px")
            .when("kind", &["blur"]),
    ]
}

fn check_fill(p: &Value) -> Vec<Issue> {
    let mut v = Vec::new();
    if p_str(p, "kind", "color") == "image"
        && p_str(p, "image", "").trim().is_empty()
        && p_str(p, "imageData", "").trim().is_empty()
    {
        let mut i = Issue::param("image", fp_i18n::t("batch-bg-err-no-image"));
        i.code = "missingImage".into();
        v.push(i);
    }
    v
}

/// `background.flatten`.
struct Flatten;

impl Step for Flatten {
    fn id(&self) -> &str {
        FLATTEN
    }
    fn group(&self) -> StepGroup {
        StepGroup::Background
    }
    fn params(&self) -> Vec<ParamDef> {
        let mut v = fill_params();
        v.push(ParamDef::bool("onlyIfTransparent", true));
        v.push(ParamDef::choice("edge", &["blend", "unmix", "hard"], "blend").beginner());
        v.push(ParamDef::color("matte", "#ffffff").when("edge", &["unmix"]));
        v.push(ParamDef::int("threshold", 1, 255, 128).when("edge", &["hard"]));
        v.push(ParamDef::bool("linearLight", false));
        v
    }
    fn validate(&self, params: &Value) -> Vec<Issue> {
        let mut v = crate::step::validate_params(&self.params(), params);
        v.extend(check_fill(params));
        v
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let cancel = ctx.cancel;
        let scale = item
            .preview_image()
            .map(|i| preview_scale(ctx, i.width(), i.height()))
            .unwrap_or(1.0);
        let mut p = params.clone();
        if let Some(b) = p.get("blur").and_then(Value::as_f64) {
            p["blur"] = Value::from(b * scale);
        }
        item.map_images(|img| {
            cancel.check()?;
            flatten_image(img, &p)
        })?;
        if let Some(m) = item.meta_mut() {
            m.has_transparency = item_has_alpha_hint(params);
        }
        Ok(())
    }
    fn summary(&self, params: &Value) -> String {
        fp_i18n::t(&format!(
            "batch-bg-summary-kind-{}",
            p_str(params, "kind", "color")
        ))
    }
}

fn item_has_alpha_hint(p: &Value) -> bool {
    p_color(p, "color", [255, 255, 255, 255])[3] < 255
}

/// `background.replace`: entfernen (KI oder Farbe) und füllen.
struct Replace;

impl Step for Replace {
    fn id(&self) -> &str {
        REPLACE
    }
    fn group(&self) -> StepGroup {
        StepGroup::Background
    }
    fn params(&self) -> Vec<ParamDef> {
        let mut v = vec![ParamDef::choice("method", &["ai", "color"], "ai").beginner()];
        // KI-Werte nur bei `method` = ai, Farbschlüssel nur bei `method` = color (BATCH-11a)
        v.extend(ai_params(false).into_iter().map(|d| {
            if d.visible_when.is_none() {
                d.when("method", &["ai"])
            } else {
                d
            }
        }));
        v.push(
            ParamDef::choice(
                "pick",
                &["topLeft", "topRight", "bottomLeft", "bottomRight", "custom"],
                "topLeft",
            )
            .when("method", &["color"]),
        );
        v.push(ParamDef::color("keyColor", "#ffffff").when("pick", &["custom"]));
        v.push(ParamDef::int("keyTolerance", 0, 255, 24).when("method", &["color"]));
        v.push(ParamDef::bool("contiguous", true).when("method", &["color"]));
        v.extend(fill_params());
        v.push(ParamDef::bool("linearLight", false));
        v
    }
    fn validate(&self, params: &Value) -> Vec<Issue> {
        let mut v = crate::step::validate_params(&self.params(), params);
        v.extend(check_fill(params));
        if p_str(params, "method", "ai") == "ai" {
            v.extend(model_issue(params));
        }
        v
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn max_parallel(&self, params: &Value) -> Option<usize> {
        (p_str(params, "method", "ai") == "ai").then_some(AI_PARALLEL)
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let deco = p_bool(params, "decontaminate", true);
        let ai = p_str(params, "method", "ai") == "ai";
        let cancel = ctx.cancel;
        let scale = item
            .preview_image()
            .map(|i| preview_scale(ctx, i.width(), i.height()))
            .unwrap_or(1.0);
        let mut fill = params.clone();
        fill["onlyIfTransparent"] = Value::Bool(false);
        fill["edge"] = Value::from("blend");
        if let Some(b) = fill.get("blur").and_then(Value::as_f64) {
            fill["blur"] = Value::from(b * scale);
        }
        let mut warnings = Vec::new();
        item.map_images(|img| {
            cancel.check()?;
            // Für „unscharfe Kopie“ zählt das Bild vor dem Entfernen.
            let original = img.clone();
            if ai {
                let mask = ai_subject_mask(img, params, cancel, &mut warnings, REPLACE)?;
                apply_subject_mask(img, &mask, "transparent", deco);
            } else {
                let mut kp = params.clone();
                kp["color"] = params
                    .get("keyColor")
                    .cloned()
                    .unwrap_or(Value::from("#ffffff"));
                let key = key_color(img, &kp);
                let tol = p_f64(params, "keyTolerance", 24.0).clamp(0.0, 255.0) as u8;
                remove_color(img, key, tol, p_bool(params, "contiguous", true), 0.0, deco);
            }
            if p_str(&fill, "kind", "color") == "blur" {
                let bg = background_layer(&original, &fill)?;
                let linear = p_bool(&fill, "linearLight", false);
                img.data_mut()
                    .par_iter_mut()
                    .zip(bg.par_iter())
                    .for_each(|(px, b)| *px = to_u8(over(to_f(*px), *b, linear)));
                Ok(())
            } else {
                flatten_image(img, &fill)
            }
        })?;
        warnings.dedup();
        ctx.warnings.extend(warnings);
        if let Some(m) = item.meta_mut() {
            m.has_transparency = item_has_alpha_hint(params);
        }
        Ok(())
    }
    fn summary(&self, params: &Value) -> String {
        fp_i18n::t(&format!(
            "batch-bg-summary-kind-{}",
            p_str(params, "kind", "color")
        ))
    }
}

/// `background.shadow`.
struct Shadow;

impl Step for Shadow {
    fn id(&self) -> &str {
        SHADOW
    }
    fn group(&self) -> StepGroup {
        StepGroup::Background
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::choice("style", &["drop", "below"], "drop").beginner(),
            ParamDef::color("color", "#000000"),
            ParamDef::int("opacity", 0, 100, 50).unit("%").beginner(),
            ParamDef::number("distance", 0.0, 1000.0, 10.0).unit("px"),
            ParamDef::number("angle", -360.0, 360.0, 315.0).unit("°"),
            ParamDef::number("blur", 0.0, 250.0, 8.0)
                .unit("px")
                .beginner(),
            ParamDef::bool("expandCanvas", true),
        ]
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let cancel = ctx.cancel;
        let scale = item
            .preview_image()
            .map(|i| preview_scale(ctx, i.width(), i.height()))
            .unwrap_or(1.0);
        item.map_images(|img| {
            cancel.check()?;
            *img = shadow_image(img, params, scale);
            Ok(())
        })?;
        if let Some(m) = item.meta_mut() {
            m.has_transparency = true;
        }
        Ok(())
    }
}
