//! Zielgröße in KB (Schritt `optimize.targetSize`, SPEC-BATCH §3.12, §5.6 Punkt 6): Bisektion über
//! eine „Leiter“ von Einstellungen, die von der besten Qualität zur kleinsten Datei führt –
//! JPEG/WebP/AVIF: Qualität (von der eingestellten bis `minQuality`); GIF: Farben 256 → 2; PNG:
//! verlustfrei optimiert, dann Farben 256 → 16 (eigener Quantisierer). Reicht das nicht und ist
//! „Qualität + Größe“ gewählt, wird das Bild schrittweise verkleinert (bis `minScale`). Höchstens
//! [`MAX_ENCODES`] Kodierungen je Datei; nicht erreichbar → Hinweis mit erreichter Größe.

use super::{encode_once, quantize, PlainEncoder, EXTRA_PNG_OPT};
use crate::error::Result;
use crate::item::Item;
use crate::output::OutputFormat;
use serde_json::{json, Map, Value};

/// Höchstzahl der Kodierungen je Datei.
pub const MAX_ENCODES: usize = 8;

/// Parameter der Suche.
#[derive(Clone, Debug, PartialEq)]
pub struct TargetSpec {
    pub max_bytes: u64,
    /// Verkleinern erlaubt (`strategy: qualityResize`).
    pub resize: bool,
    pub min_quality: i64,
    /// Kleinster Maßstab 0,1…1.
    pub min_scale: f64,
}

impl TargetSpec {
    /// Aus den Schritt-Parametern (`maxKB`, `strategy`, `minQuality`, `minScale` in %).
    pub fn from_params(v: &Value) -> TargetSpec {
        let kb = v.get("maxKB").and_then(Value::as_f64).unwrap_or(200.0).max(1.0);
        TargetSpec {
            max_bytes: (kb * 1024.0) as u64,
            resize: v.get("strategy").and_then(Value::as_str) == Some("qualityResize"),
            min_quality: v.get("minQuality").and_then(Value::as_i64).unwrap_or(40).clamp(1, 100),
            min_scale: (v.get("minScale").and_then(Value::as_f64).unwrap_or(50.0) / 100.0).clamp(0.1, 1.0),
        }
    }
}

/// Eine Stufe der Leiter.
#[derive(Clone, Debug, PartialEq)]
enum Rung {
    /// Formatoption setzen (`quality`, `colors`).
    Option(&'static str, i64),
    /// PNG verlustfrei optimiert.
    PngLossless,
    /// PNG mit höchstens n Farben (+ verlustfrei optimiert).
    PngColors(usize),
}

fn ladder(fmt: OutputFormat, opts: &Map<String, Value>, spec: &TargetSpec) -> Vec<Rung> {
    let q = |d: i64| opts.get("quality").and_then(Value::as_i64).unwrap_or(d);
    let quality = |hi: i64| -> Vec<Rung> {
        let lo = spec.min_quality.min(hi);
        (lo..=hi).rev().map(|v| Rung::Option("quality", v)).collect()
    };
    match fmt {
        OutputFormat::Jpeg => quality(q(85).clamp(1, 100)),
        OutputFormat::WebP => quality(q(82).clamp(0, 100)),
        OutputFormat::Avif => quality(q(60).clamp(1, 100)),
        OutputFormat::Gif => {
            let start = opts.get("colors").and_then(Value::as_i64).unwrap_or(256).clamp(2, 256);
            [256, 192, 128, 96, 64, 48, 32, 16, 8, 4, 2].into_iter().filter(|&c| c <= start).map(|c| Rung::Option("colors", c)).collect()
        }
        OutputFormat::Png => {
            let mut v = vec![Rung::PngLossless];
            v.extend([256usize, 192, 128, 96, 64, 48, 32, 24, 16].into_iter().map(Rung::PngColors));
            v
        }
        _ => Vec::new(),
    }
}

fn with_png_opt(item: &mut Item) {
    if let Some(m) = item.meta_mut() {
        m.extra.entry(EXTRA_PNG_OPT.to_string()).or_insert_with(|| json!({"level": 2, "alphaClean": true}));
    }
}

/// Element für eine Stufe vorbereiten und kodieren.
fn encode_rung(item: &Item, fmt: OutputFormat, opts: &Map<String, Value>, rung: Option<&Rung>, plain: PlainEncoder) -> Result<(Vec<u8>, Vec<String>)> {
    match rung {
        None => encode_once(item, fmt, opts, plain),
        Some(Rung::Option(k, v)) => {
            let mut o = opts.clone();
            o.insert((*k).to_string(), json!(v));
            if fmt == OutputFormat::WebP {
                o.insert("lossless".into(), json!(false));
            }
            encode_once(item, fmt, &o, plain)
        }
        Some(Rung::PngLossless) => {
            let mut it = item.clone();
            with_png_opt(&mut it);
            encode_once(&it, fmt, opts, plain)
        }
        Some(Rung::PngColors(n)) => {
            let mut it = item.clone();
            it.map_images(|img| {
                let w = img.width();
                quantize::quantize(img.data_mut(), w, *n, quantize::Dither::FloydSteinberg, 0.8);
                Ok(())
            })?;
            with_png_opt(&mut it);
            encode_once(&it, fmt, opts, plain)
        }
    }
}

/// Verkleinert alle Bilder des Elements auf `scale` (bikubisch).
fn scaled(item: &Item, scale: f64) -> Result<Item> {
    let mut it = item.clone();
    it.map_images(|img| {
        let (w, h) = (img.width() as f64, img.height() as f64);
        let (nw, nh) = (((w * scale).round() as u32).max(1), ((h * scale).round() as u32).max(1));
        *img = fp_raster::resample::resize(img, nw, nh, fp_raster::Sampler::Bicubic);
        Ok(())
    })?;
    Ok(it)
}

fn kb(b: u64) -> String {
    let v = b as f64 / 1024.0;
    if fp_i18n::current_language() == "de" { format!("{v:.1}").replace('.', ",") } else { format!("{v:.1}") }
}

/// Sucht die beste Einstellung, deren Datei höchstens `spec.max_bytes` groß ist.
pub fn search(item: &Item, fmt: OutputFormat, opts: &Map<String, Value>, spec: &TargetSpec, plain: PlainEncoder) -> Result<(Vec<u8>, Vec<String>)> {
    let mut opts = opts.clone();
    let mut notes = Vec::new();
    if fmt == OutputFormat::WebP && opts.get("lossless").and_then(Value::as_bool) == Some(true) {
        opts.insert("lossless".into(), json!(false));
        notes.push(fp_i18n::t("batch-meta-info-target-lossy"));
    }
    let rungs = ladder(fmt, &opts, spec);
    let used = std::cell::Cell::new(0usize);
    let enc = |it: &Item, r: Option<&Rung>| -> Result<(Vec<u8>, Vec<String>)> {
        used.set(used.get() + 1);
        encode_rung(it, fmt, &opts, r, plain)
    };
    let fits = |b: &[u8]| b.len() as u64 <= spec.max_bytes;
    // beste Stufe zuerst
    let first = enc(item, rungs.first())?;
    if fits(&first.0) {
        return Ok(with_notes(first, notes));
    }
    let mut smallest = first;
    let mut last_rung = rungs.first();
    if rungs.len() > 1 {
        let lo = enc(item, rungs.last())?;
        last_rung = rungs.last();
        if fits(&lo.0) {
            // Bisektion: kleinster Index k mit Datei ≤ Ziel (beste Qualität, die passt)
            let (mut bad, mut good) = (0usize, rungs.len() - 1);
            let mut best = lo;
            while good - bad > 1 && used.get() < MAX_ENCODES {
                let mid = (bad + good) / 2;
                let r = enc(item, rungs.get(mid))?;
                if fits(&r.0) {
                    good = mid;
                    best = r;
                } else {
                    bad = mid;
                }
            }
            return Ok(with_notes(best, notes));
        }
        if lo.0.len() < smallest.0.len() {
            smallest = lo;
        }
    }
    // Verkleinern (nur „Qualität + Größe“)
    if spec.resize {
        let mut scale = 1.0f64;
        let mut size = smallest.0.len() as f64;
        while used.get() < MAX_ENCODES && scale > spec.min_scale + 1e-9 {
            scale = (scale * (spec.max_bytes as f64 / size).sqrt() * 0.95).min(scale * 0.97).max(spec.min_scale);
            let it = scaled(item, scale)?;
            let r = enc(&it, last_rung)?;
            size = r.0.len() as f64;
            let ok = fits(&r.0);
            if r.0.len() < smallest.0.len() || ok {
                smallest = r;
            }
            if ok {
                notes.push(fp_i18n::tr!("batch-meta-info-target-resized", percent = (scale * 100.0).round() as i64));
                return Ok(with_notes(smallest, notes));
            }
        }
    }
    notes.push(fp_i18n::tr!("batch-meta-warn-target-missed", target = kb(spec.max_bytes), size = kb(smallest.0.len() as u64)));
    Ok(with_notes(smallest, notes))
}

fn with_notes(mut r: (Vec<u8>, Vec<String>), notes: Vec<String>) -> (Vec<u8>, Vec<String>) {
    r.1.extend(notes);
    r
}
