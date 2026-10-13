//! Gruppe „Optimieren“ (SPEC-BATCH §3.12): `optimize.png` (verlustfrei, eigene Umsetzung),
//! `optimize.quantize` (Farben reduzieren, Dithering) und `optimize.targetSize` (Zielgröße in KB).
//!
//! `optimize.png` und `optimize.targetSize` ändern keine Pixel, sondern legen ihre Parameter in
//! `Meta::extra` ab; gerechnet wird beim Kodieren (`meta::finish_encode`), damit Vorschau-Größe und
//! Lauf dieselbe Datei messen. Mehrfaches Anwenden ersetzt die Werte (idempotent).

use crate::error::Result;
use crate::item::{Item, ItemKind};
use crate::meta::{quantize, EXTRA_COMMENT, EXTRA_IPTC, EXTRA_OUTPUT_OPTIONS, EXTRA_PNG_OPT, EXTRA_TARGET};
use crate::output::OutputFormat;
use crate::step::{ParamDef, Step, StepContext, StepGroup, StepRegistry};
use serde_json::{json, Value};
use std::sync::Arc;

pub const PNG: &str = "optimize.png";
pub const QUANTIZE: &str = "optimize.quantize";
pub const TARGET_SIZE: &str = "optimize.targetSize";

/// Registriert die Schritte der Gruppe.
pub fn register(r: &mut StepRegistry) {
    r.register(Arc::new(Png));
    r.register(Arc::new(Quantize));
    r.register(Arc::new(TargetSize));
}

fn int(p: &Value, k: &str, d: i64) -> i64 {
    p.get(k).and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f.round() as i64))).unwrap_or(d)
}

/// `optimize.png`.
struct Png;

impl Step for Png {
    fn id(&self) -> &str {
        PNG
    }
    fn group(&self) -> StepGroup {
        StepGroup::Optimize
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![ParamDef::int("level", 1, 6, 2).beginner(), ParamDef::choice("strip", &["none", "safe", "all"], "safe"), ParamDef::bool("alphaClean", true)]
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        if ctx.output_format != OutputFormat::Png {
            ctx.warn(fp_i18n::tr!("batch-meta-warn-png-only", format = ctx.output_format.display_name()));
            return Ok(());
        }
        let strip = params.get("strip").and_then(Value::as_str).unwrap_or("safe");
        let Some(m) = item.meta_mut() else { return Ok(()) };
        if strip != "none" {
            m.exif = None;
            m.xmp = None;
            m.extra.remove(EXTRA_IPTC);
            m.extra.remove(EXTRA_COMMENT);
            if strip == "all" {
                m.icc = None;
            }
        }
        m.extra.insert(EXTRA_PNG_OPT.into(), json!({"level": int(params, "level", 2).clamp(1, 6), "alphaClean": params.get("alphaClean").and_then(Value::as_bool).unwrap_or(true)}));
        Ok(())
    }
    fn summary(&self, params: &Value) -> String {
        fp_i18n::tr!("batch-meta-summary-png", level = int(params, "level", 2))
    }
}

/// `optimize.quantize`.
struct Quantize;

impl Step for Quantize {
    fn id(&self) -> &str {
        QUANTIZE
    }
    fn group(&self) -> StepGroup {
        StepGroup::Optimize
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::int("colors", 2, 256, 256).beginner(),
            ParamDef::choice("dither", &["none", "ordered", "floydSteinberg"], "floydSteinberg").beginner(),
            ParamDef::int("ditherAmount", 0, 100, 80).unit("%").when("dither", &["ordered", "floydSteinberg"]),
        ]
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let n = int(params, "colors", 256).clamp(2, 256) as usize;
        let dither = quantize::Dither::parse(params.get("dither").and_then(Value::as_str).unwrap_or("floydSteinberg"));
        let amount = int(params, "ditherAmount", 80).clamp(0, 100) as f32 / 100.0;
        let cancel = ctx.cancel.clone();
        item.map_images(|img| {
            cancel.check()?;
            let w = img.width();
            quantize::quantize(img.data_mut(), w, n, dither, amount);
            Ok(())
        })?;
        let fmt = ctx.output_format;
        if let Some(m) = item.meta_mut() {
            m.has_transparency = false;
            match fmt {
                // Palette-PNG verlustfrei aus den reduzierten Farben (kein zweites Quantisieren)
                OutputFormat::Png => {
                    m.extra.entry(EXTRA_PNG_OPT.to_string()).or_insert_with(|| json!({"level": 2, "alphaClean": false}));
                }
                // GIF: Farbzahl übernehmen, nicht noch einmal dithern
                OutputFormat::Gif => {
                    let mut o = m.extra.get(EXTRA_OUTPUT_OPTIONS).and_then(Value::as_object).cloned().unwrap_or_default();
                    o.insert("colors".into(), json!(n));
                    o.insert("dither".into(), json!("none"));
                    m.extra.insert(EXTRA_OUTPUT_OPTIONS.into(), Value::Object(o));
                }
                _ => {}
            }
        }
        if let Item::Still(s) = item {
            s.meta.has_transparency = s.image.data().iter().any(|p| p[3] < 255);
        } else if let Item::Animation(a) = item {
            a.meta.has_transparency = a.frames.iter().any(|f| f.image.data().iter().any(|p| p[3] < 255));
        }
        Ok(())
    }
    fn summary(&self, params: &Value) -> String {
        let d = params.get("dither").and_then(Value::as_str).unwrap_or("floydSteinberg");
        fp_i18n::tr!("batch-meta-summary-quantize", colors = int(params, "colors", 256), dither = fp_i18n::t(&format!("batch-meta-dither-{d}")))
    }
}

/// `optimize.targetSize`.
struct TargetSize;

impl Step for TargetSize {
    fn id(&self) -> &str {
        TARGET_SIZE
    }
    fn group(&self) -> StepGroup {
        StepGroup::Optimize
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::int("maxKB", 1, 100_000, 200).beginner().unit("KB"),
            ParamDef::choice("strategy", &["quality", "qualityResize"], "quality").beginner(),
            ParamDef::int("minQuality", 10, 95, 40),
            ParamDef::int("minScale", 10, 100, 50).unit("%").when("strategy", &["qualityResize"]),
        ]
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let fmt = ctx.output_format;
        let resize = params.get("strategy").and_then(Value::as_str) == Some("qualityResize");
        let searchable = matches!(fmt, OutputFormat::Jpeg | OutputFormat::WebP | OutputFormat::Avif | OutputFormat::Gif | OutputFormat::Png);
        if fmt == OutputFormat::FwPng || (!searchable && !resize) {
            ctx.warn(fp_i18n::tr!("batch-meta-warn-target-format", format = fmt.display_name()));
            if fmt == OutputFormat::FwPng {
                return Ok(());
            }
        }
        if let Some(m) = item.meta_mut() {
            m.extra.insert(EXTRA_TARGET.into(), params.clone());
        }
        Ok(())
    }
    fn summary(&self, params: &Value) -> String {
        fp_i18n::tr!("batch-meta-summary-target", kb = int(params, "maxKB", 200))
    }
}
