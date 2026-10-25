//! Gruppe „Farbe und Effekte“ (SPEC-BATCH §3.4, Paket BATCH-4): `adjust.auto`, `adjust.basic`,
//! `adjust.levels`, `adjust.curves`, `adjust.grayscale`, `adjust.invert`, `filter.sharpen`,
//! `filter.blur`, `filter.registry`, `effect.live` und `filter.screentone` (Firepixel-Erweiterung
//! FB-38/FB-86).
//!
//! Die Rechenwege kommen aus den vorhandenen Filtern (`fp-raster`, dieselben wie *Filter ▸ …* im
//! Menü) bzw. der Effekt-Engine (`fp-effects`, wie die Live-Effekte im Dokument). Eigene Rechnung nur
//! für Gamma, Farbtemperatur/Tönung, Weißabgleich (Grauwelt), Auto-Kontrast und Graustufen/Sepia/
//! Duoton. Werte mit Vorgabe (0, 1, aus) lassen das Bild **bitgleich** unverändert.
//!
//! Animationen: Automatik-Schritte messen einmal über alle Frames (kein Flackern), alle Frames
//! bekommen dieselbe Korrektur; Filter laufen je Frame mit denselben Parametern.

use crate::error::{BatchError, Result};
use crate::item::{Item, ItemKind};
use crate::step::{Issue, ParamDef, Step, StepContext, StepGroup, StepRegistry, validate_params};
use crate::steps::geometry::{
    finish, fmt_num, frames_mut, p_bool, p_color, p_f64, p_i64, p_str, preview_scale,
    refresh_transparency,
};
use crate::steps::script::{check_command, filter_commands, run_on_item};
use fp_raster::filters::{
    BrightnessContrastParams, CurvePoint, CurvesParams, GaussianBlurParams, HueSaturationParams,
    LevelsChannel, LevelsParams, MotionBlurParams, ParamValue, RadialBlurParams, UnsharpMaskParams,
    ZoomBlurParams, levels_math,
};
use fp_raster::{Filter, PixelFilter, Px, RgbaImage};
use rayon::prelude::*;
use serde_json::{Value, json};
use std::sync::Arc;

/// IDs der Gruppe.
pub const ADJUST_AUTO: &str = "adjust.auto";
pub const ADJUST_BASIC: &str = "adjust.basic";
pub const ADJUST_LEVELS: &str = "adjust.levels";
pub const ADJUST_CURVES: &str = "adjust.curves";
pub const ADJUST_GRAYSCALE: &str = "adjust.grayscale";
pub const ADJUST_INVERT: &str = "adjust.invert";
pub const FILTER_SHARPEN: &str = "filter.sharpen";
pub const FILTER_BLUR: &str = "filter.blur";
pub const FILTER_REGISTRY: &str = "filter.registry";
pub const EFFECT_LIVE: &str = "effect.live";
pub const FILTER_SCREENTONE: &str = "filter.screentone";

/// Registriert die Schritte der Gruppe.
pub fn register(r: &mut StepRegistry) {
    r.register(Arc::new(AdjustAuto));
    r.register(Arc::new(AdjustBasic));
    r.register(Arc::new(AdjustLevels));
    r.register(Arc::new(AdjustCurves));
    r.register(Arc::new(AdjustGrayscale));
    r.register(Arc::new(AdjustInvert));
    r.register(Arc::new(FilterSharpen));
    r.register(Arc::new(FilterBlur));
    r.register(Arc::new(FilterRegistry));
    r.register(Arc::new(EffectLive));
    r.register(Arc::new(FilterScreentone));
}

// =================================================================== Hilfen

/// Wendet einen Filter des Registers auf jedes Bild an; das Ergebnis behält die Bildgröße
/// (Weichzeichner wachsen sonst um ihren Rand).
fn apply_filter(item: &mut Item, ctx: &StepContext, f: &Filter) -> Result<()> {
    for img in frames_mut(item)? {
        ctx.check_cancel()?;
        let b = img.bounds();
        let out = f.apply(img, None);
        *img = if out.bounds() == b {
            out
        } else {
            out.cropped(b)
        };
    }
    finish(item);
    Ok(())
}

/// Wendet je Kanal eine Tabelle an (Alpha bleibt).
fn apply_luts(item: &mut Item, ctx: &StepContext, lut: &[[u8; 256]; 3]) -> Result<()> {
    for img in frames_mut(item)? {
        ctx.check_cancel()?;
        let w = img.width().max(1);
        img.data_mut().par_chunks_mut(w).for_each(|row| {
            for p in row {
                p[0] = lut[0][p[0] as usize];
                p[1] = lut[1][p[1] as usize];
                p[2] = lut[2][p[2] as usize];
            }
        });
    }
    finish(item);
    Ok(())
}

/// Wendet eine Pixelfunktion an (Alpha bleibt, wenn `f` ihn nicht ändert).
fn map_px(item: &mut Item, ctx: &StepContext, f: impl Fn(Px) -> Px + Sync) -> Result<()> {
    for img in frames_mut(item)? {
        ctx.check_cancel()?;
        let w = img.width().max(1);
        img.data_mut().par_chunks_mut(w).for_each(|row| {
            for p in row {
                *p = f(*p);
            }
        });
    }
    finish(item);
    Ok(())
}

/// Alle Bilder des Elements (nur lesen; Dokumente flach gerechnet).
fn images(item: &mut Item) -> Result<Vec<RgbaImage>> {
    Ok(frames_mut(item)?.into_iter().map(|i| i.clone()).collect())
}

/// Helligkeit nach Rec. 601 (ganzzahlig gerundet).
fn luma(p: Px) -> u8 {
    ((299 * p[0] as u32 + 587 * p[1] as u32 + 114 * p[2] as u32 + 500) / 1000) as u8
}

// =================================================================== adjust.auto

/// `adjust.auto`: automatisch verbessern (Tonwert, Kontrast, Weißabgleich).
struct AdjustAuto;

impl Step for AdjustAuto {
    fn id(&self) -> &str {
        ADJUST_AUTO
    }
    fn group(&self) -> StepGroup {
        StepGroup::Color
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::bool("levels", true).beginner(),
            ParamDef::bool("contrast", true).beginner(),
            ParamDef::bool("whiteBalance", false).beginner(),
        ]
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        // 1. Weißabgleich (Grauwelt): Kanalmittel angleichen
        if p_bool(params, "whiteBalance", false) {
            let imgs = images(item)?;
            let mut sum = [0f64; 3];
            let mut n = 0f64;
            for img in &imgs {
                for p in img.data().iter().filter(|p| p[3] > 0) {
                    for c in 0..3 {
                        sum[c] += p[c] as f64;
                    }
                    n += 1.0;
                }
            }
            if n > 0.0 && sum.iter().all(|s| *s > 0.0) {
                let gray = (sum[0] + sum[1] + sum[2]) / 3.0;
                let lut: [[u8; 256]; 3] = std::array::from_fn(|c| {
                    let g = (gray / sum[c]).clamp(0.5, 2.0);
                    std::array::from_fn(|i| (i as f64 * g + 0.5).floor().clamp(0.0, 255.0) as u8)
                });
                apply_luts(item, ctx, &lut)?;
            }
        }
        // 2. Auto-Tonwert je Kanal (wie *Filter ▸ Farbe anpassen ▸ Auto-Tonwertkorrektur*), aber mit
        //    einem gemeinsamen Histogramm über alle Frames
        if p_bool(params, "levels", true) {
            let imgs = images(item)?;
            let mut h = [[0u64; 256]; 3];
            for img in &imgs {
                levels_math::add_channel_histograms(&mut h, img);
            }
            let mut lp = LevelsParams::default();
            levels_math::auto_levels_from(&mut lp, &h);
            if lp != LevelsParams::default() {
                apply_filter(item, ctx, &Filter::Levels(lp))?;
            }
        }
        // 3. Auto-Kontrast: Helligkeit strecken (0,5 % abgeschnitten), gleich für alle Kanäle
        if p_bool(params, "contrast", true) {
            let imgs = images(item)?;
            let mut hist = [0u64; 256];
            for img in &imgs {
                for p in img.data().iter().filter(|p| p[3] > 0) {
                    hist[luma(*p) as usize] += 1;
                }
            }
            let total: u64 = hist.iter().sum();
            if total > 0 {
                let clip = total / 200;
                let mut acc = 0;
                let lo = hist.iter().position(|v| {
                    acc += v;
                    acc > clip
                });
                acc = 0;
                let hi = hist.iter().rposition(|v| {
                    acc += v;
                    acc > clip
                });
                if let (Some(lo), Some(hi)) = (lo, hi)
                    && hi > lo
                    && (lo > 0 || hi < 255)
                {
                    let span = (hi - lo) as f64;
                    let l: [u8; 256] = std::array::from_fn(|i| {
                        ((i as f64 - lo as f64) * 255.0 / span + 0.5)
                            .floor()
                            .clamp(0.0, 255.0) as u8
                    });
                    apply_luts(item, ctx, &[l, l, l])?;
                }
            }
        }
        Ok(())
    }
}

// =================================================================== adjust.basic

/// `adjust.basic`: Helligkeit, Kontrast, Sättigung, Farbton, Gamma, Farbtemperatur, Tönung.
struct AdjustBasic;

impl Step for AdjustBasic {
    fn id(&self) -> &str {
        ADJUST_BASIC
    }
    fn group(&self) -> StepGroup {
        StepGroup::Color
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::int("brightness", -100, 100, 0).beginner(),
            ParamDef::int("contrast", -100, 100, 0).beginner(),
            ParamDef::int("saturation", -100, 100, 0).beginner(),
            ParamDef::int("hue", -180, 180, 0).unit("°"),
            ParamDef::number("gamma", 0.1, 5.0, 1.0),
            ParamDef::int("temperature", -100, 100, 0),
            ParamDef::int("tint", -100, 100, 0),
        ]
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let b = p_i64(params, "brightness", 0).clamp(-100, 100) as i32;
        let c = p_i64(params, "contrast", 0).clamp(-100, 100) as i32;
        if b != 0 || c != 0 {
            apply_filter(
                item,
                ctx,
                &Filter::BrightnessContrast(BrightnessContrastParams {
                    brightness: b,
                    contrast: c,
                }),
            )?;
        }
        let s = p_i64(params, "saturation", 0).clamp(-100, 100) as i32;
        let h = p_i64(params, "hue", 0).clamp(-180, 180) as i32;
        if s != 0 || h != 0 {
            apply_filter(
                item,
                ctx,
                &Filter::HueSaturation(HueSaturationParams {
                    hue: h,
                    saturation: s,
                    lightness: 0,
                    colorize: false,
                }),
            )?;
        }
        let g = p_f64(params, "gamma", 1.0).clamp(0.1, 5.0);
        let t = p_i64(params, "temperature", 0).clamp(-100, 100) as f64;
        let tint = p_i64(params, "tint", 0).clamp(-100, 100) as f64;
        if (g - 1.0).abs() > 1e-9 || t != 0.0 || tint != 0.0 {
            // Gamma, dann Temperatur (warm: Rot +, Blau −) und Tönung (+ = Magenta: Grün −), je bis ±40 Stufen
            let curve = |i: usize, shift: f64| -> u8 {
                let v = 255.0 * (i as f64 / 255.0).powf(1.0 / g) + shift;
                (v + 0.5).floor().clamp(0.0, 255.0) as u8
            };
            let lut: [[u8; 256]; 3] = [
                std::array::from_fn(|i| curve(i, t * 0.4)),
                std::array::from_fn(|i| curve(i, -tint * 0.4)),
                std::array::from_fn(|i| curve(i, -t * 0.4)),
            ];
            apply_luts(item, ctx, &lut)?;
        }
        Ok(())
    }
    fn summary(&self, p: &Value) -> String {
        fp_i18n::tr!(
            "batch-summary-adjust-basic",
            brightness = p_i64(p, "brightness", 0),
            contrast = p_i64(p, "contrast", 0),
            saturation = p_i64(p, "saturation", 0)
        )
    }
}

// =================================================================== adjust.levels

/// `adjust.levels`: Tonwertkorrektur (wie der Dialog; RGB-Regler oder alle Kanäle als JSON).
struct AdjustLevels;

/// Ganzzahlige Gleitkommawerte (`26.0`) als Ganzzahl, damit sie in `i32`-Felder passen.
fn whole_numbers(v: &mut Value) {
    match v {
        Value::Number(n) if !n.is_i64() && !n.is_u64() => {
            if let Some(f) = n.as_f64().filter(|f| f.fract() == 0.0 && f.abs() < 9.0e15) {
                *v = Value::from(f as i64);
            }
        }
        Value::Array(a) => a.iter_mut().for_each(whole_numbers),
        Value::Object(o) => o.values_mut().for_each(whole_numbers),
        _ => {}
    }
}

fn levels_of(p: &Value) -> std::result::Result<LevelsParams, String> {
    if let Some(ch) = p.get("channels").filter(|v| !v.is_null()) {
        let mut v = ch.clone();
        whole_numbers(&mut v);
        return serde_json::from_value::<LevelsParams>(v).map_err(|e| e.to_string());
    }
    let i = |k: &str, d: i64| p_i64(p, k, d).clamp(0, 255) as i32;
    Ok(LevelsParams {
        rgb: LevelsChannel {
            source_low: i("inputLow", 0),
            source_high: i("inputHigh", 255),
            dest_low: i("outputLow", 0),
            dest_high: i("outputHigh", 255),
            gamma: p_f64(p, "gamma", 1.0).clamp(0.1, 9.99),
        },
        ..LevelsParams::default()
    })
}

impl Step for AdjustLevels {
    fn id(&self) -> &str {
        ADJUST_LEVELS
    }
    fn group(&self) -> StepGroup {
        StepGroup::Color
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::int("inputLow", 0, 255, 0).beginner(),
            ParamDef::number("gamma", 0.1, 9.99, 1.0).beginner(),
            ParamDef::int("inputHigh", 0, 255, 255).beginner(),
            ParamDef::int("outputLow", 0, 255, 0),
            ParamDef::int("outputHigh", 0, 255, 255),
            ParamDef::json("channels", Value::Null),
        ]
    }
    fn validate(&self, params: &Value) -> Vec<Issue> {
        let mut out = validate_params(&self.params(), params);
        if let Err(e) = levels_of(params) {
            out.push(Issue::param(
                "channels",
                fp_i18n::tr!("batch-issue-json-invalid", error = e),
            ));
        }
        out
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let lp = levels_of(params).map_err(|e| {
            BatchError::Invalid(fp_i18n::tr!("batch-issue-json-invalid", error = e))
        })?;
        if lp == LevelsParams::default() {
            return Ok(());
        }
        apply_filter(item, ctx, &Filter::Levels(lp))
    }
}

// =================================================================== adjust.curves

/// `adjust.curves`: Gradationskurven (Punkte der RGB-Kurve oder alle Kanäle als JSON).
struct AdjustCurves;

fn points_of(v: &Value) -> Option<Vec<CurvePoint>> {
    let a = v.as_array()?;
    let mut out = Vec::with_capacity(a.len());
    for p in a {
        let (x, y) = match p {
            Value::Array(xy) if xy.len() == 2 => (xy[0].as_f64()?, xy[1].as_f64()?),
            Value::Object(o) => (o.get("x")?.as_f64()?, o.get("y")?.as_f64()?),
            _ => return None,
        };
        out.push(CurvePoint {
            x: x.clamp(0.0, 255.0) as f32,
            y: y.clamp(0.0, 255.0) as f32,
        });
    }
    (out.len() >= 2).then_some(out)
}

fn curves_of(p: &Value) -> std::result::Result<CurvesParams, String> {
    if let Some(ch) = p.get("channels").filter(|v| !v.is_null()) {
        return serde_json::from_value::<CurvesParams>(ch.clone()).map_err(|e| e.to_string());
    }
    let pts = match p.get("points") {
        None | Some(Value::Null) => return Ok(CurvesParams::default()),
        Some(v) => points_of(v).ok_or_else(|| fp_i18n::t("batch-issue-curve-points"))?,
    };
    Ok(CurvesParams {
        rgb_points: pts,
        ..CurvesParams::default()
    })
}

impl Step for AdjustCurves {
    fn id(&self) -> &str {
        ADJUST_CURVES
    }
    fn group(&self) -> StepGroup {
        StepGroup::Color
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::json("points", json!([[0, 0], [255, 255]])).beginner(),
            ParamDef::json("channels", Value::Null),
        ]
    }
    fn validate(&self, params: &Value) -> Vec<Issue> {
        let mut out = validate_params(&self.params(), params);
        if let Err(e) = curves_of(params) {
            out.push(Issue::param(
                "points",
                fp_i18n::tr!("batch-issue-json-invalid", error = e),
            ));
        }
        out
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let cp = curves_of(params).map_err(|e| {
            BatchError::Invalid(fp_i18n::tr!("batch-issue-json-invalid", error = e))
        })?;
        if cp == CurvesParams::default() {
            return Ok(());
        }
        apply_filter(item, ctx, &Filter::Curves(cp))
    }
}

// =================================================================== adjust.grayscale

/// `adjust.grayscale`: Schwarzweiß, Sepia oder Duoton.
struct AdjustGrayscale;

impl Step for AdjustGrayscale {
    fn id(&self) -> &str {
        ADJUST_GRAYSCALE
    }
    fn group(&self) -> StepGroup {
        StepGroup::Color
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::choice("style", &["gray", "sepia", "duotone"], "gray").beginner(),
            ParamDef::color("dark", "#1b2a4a").when("style", &["duotone"]),
            ParamDef::color("light", "#f5d9a8").when("style", &["duotone"]),
        ]
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        match p_str(params, "style", "gray") {
            "sepia" => map_px(item, ctx, |p| {
                let (r, g, b) = (p[0] as f64, p[1] as f64, p[2] as f64);
                let ch = |v: f64| (v + 0.5).floor().clamp(0.0, 255.0) as u8;
                [
                    ch(0.393 * r + 0.769 * g + 0.189 * b),
                    ch(0.349 * r + 0.686 * g + 0.168 * b),
                    ch(0.272 * r + 0.534 * g + 0.131 * b),
                    p[3],
                ]
            }),
            "duotone" => {
                let d = p_color(params, "dark", [0x1b, 0x2a, 0x4a, 255]);
                let l = p_color(params, "light", [0xf5, 0xd9, 0xa8, 255]);
                map_px(item, ctx, move |p| {
                    let t = luma(p) as u32;
                    let mix =
                        |a: u8, b: u8| ((a as u32 * (255 - t) + b as u32 * t + 127) / 255) as u8;
                    [mix(d[0], l[0]), mix(d[1], l[1]), mix(d[2], l[2]), p[3]]
                })
            }
            _ => map_px(item, ctx, |p| {
                let y = luma(p);
                [y, y, y, p[3]]
            }),
        }
    }
    fn summary(&self, p: &Value) -> String {
        fp_i18n::t(&format!(
            "batch-value-adjust-grayscale-style-{}",
            p_str(p, "style", "gray")
        ))
    }
}

// =================================================================== adjust.invert

/// `adjust.invert`: Farben umkehren (Alpha bleibt).
struct AdjustInvert;

impl Step for AdjustInvert {
    fn id(&self) -> &str {
        ADJUST_INVERT
    }
    fn group(&self) -> StepGroup {
        StepGroup::Color
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, _params: &Value, ctx: &mut StepContext) -> Result<()> {
        apply_filter(item, ctx, &Filter::Invert)
    }
}

// =================================================================== filter.sharpen

/// `filter.sharpen`: Unscharf maskieren.
struct FilterSharpen;

impl Step for FilterSharpen {
    fn id(&self) -> &str {
        FILTER_SHARPEN
    }
    fn group(&self) -> StepGroup {
        StepGroup::Color
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::int("amount", 0, 500, 60).beginner().unit("%"),
            ParamDef::number("radius", 0.1, 10.0, 1.0).unit("px"),
            ParamDef::int("threshold", 0, 255, 0),
        ]
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let amount = p_i64(params, "amount", 60).clamp(0, 500) as i32;
        if amount == 0 {
            return Ok(());
        }
        let scale = preview_scale(item, ctx);
        let radius = (p_f64(params, "radius", 1.0).clamp(0.1, 10.0) * scale).max(0.1);
        let threshold = p_i64(params, "threshold", 0).clamp(0, 255) as i32;
        apply_filter(
            item,
            ctx,
            &Filter::UnsharpMask(UnsharpMaskParams {
                radius,
                amount,
                threshold,
                ..UnsharpMaskParams::default()
            }),
        )
    }
    fn summary(&self, p: &Value) -> String {
        fp_i18n::tr!(
            "batch-summary-filter-sharpen",
            amount = p_i64(p, "amount", 60)
        )
    }
}

// =================================================================== filter.blur

/// `filter.blur`: Weichzeichnen (Gauß, Bewegung, radial, Zoom).
struct FilterBlur;

impl Step for FilterBlur {
    fn id(&self) -> &str {
        FILTER_BLUR
    }
    fn group(&self) -> StepGroup {
        StepGroup::Color
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::choice(
                "kind",
                &["gaussian", "motion", "radial", "zoom"],
                "gaussian",
            )
            .beginner(),
            ParamDef::number("radius", 0.0, 250.0, 2.0)
                .beginner()
                .unit("px")
                .when("kind", &["gaussian", "motion"]),
            ParamDef::number("angle", 0.0, 360.0, 0.0)
                .unit("°")
                .when("kind", &["motion"]),
            ParamDef::int("amount", 1, 100, 30).when("kind", &["radial", "zoom"]),
            ParamDef::int("quality", 1, 100, 20).when("kind", &["radial", "zoom"]),
        ]
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let scale = preview_scale(item, ctx);
        let radius = p_f64(params, "radius", 2.0).clamp(0.0, 250.0) * scale;
        let amount = p_i64(params, "amount", 30).clamp(1, 100) as i32;
        let quality = p_i64(params, "quality", 20).clamp(1, 100) as i32;
        let f = match p_str(params, "kind", "gaussian") {
            "motion" => {
                let d = radius.round() as i32;
                if d <= 0 {
                    return Ok(());
                }
                Filter::MotionBlur(MotionBlurParams {
                    angle: p_f64(params, "angle", 0.0).round() as i32,
                    distance: d,
                })
            }
            "radial" => Filter::RadialBlur(RadialBlurParams { amount, quality }),
            "zoom" => Filter::ZoomBlur(ZoomBlurParams {
                amount,
                quality,
                center: None,
            }),
            _ => {
                if radius <= 0.0 {
                    return Ok(());
                }
                Filter::GaussianBlur(GaussianBlurParams {
                    radius,
                    ..GaussianBlurParams::default()
                })
            }
        };
        apply_filter(item, ctx, &f)
    }
    fn summary(&self, p: &Value) -> String {
        let kind = fp_i18n::t(&format!(
            "batch-value-filter-blur-kind-{}",
            p_str(p, "kind", "gaussian")
        ));
        fp_i18n::tr!(
            "batch-summary-filter-blur",
            kind = kind,
            radius = fmt_num(p_f64(p, "radius", 2.0))
        )
    }
}

// =================================================================== filter.registry

/// `filter.registry`: beliebiger `filter.*`-Befehl des Registers mit seinen Parametern (läuft wie
/// *Filter ▸ …* auf der ausgewählten Bitmap in einer eigenen Sitzung).
struct FilterRegistry;

impl Step for FilterRegistry {
    fn id(&self) -> &str {
        FILTER_REGISTRY
    }
    fn group(&self) -> StepGroup {
        StepGroup::Color
    }
    fn params(&self) -> Vec<ParamDef> {
        let list: Vec<&str> = filter_commands().iter().map(String::as_str).collect();
        let default = if list.contains(&"filter.blur.gaussian") {
            "filter.blur.gaussian"
        } else {
            list.first().copied().unwrap_or("")
        };
        vec![
            ParamDef::choice("command", &list, default).beginner(),
            ParamDef::json("params", json!({})).beginner(),
        ]
    }
    fn validate(&self, params: &Value) -> Vec<Issue> {
        let mut out = validate_params(&self.params(), params);
        if let Some(c) = params.get("command").and_then(Value::as_str)
            && let Some(m) = check_command(c)
        {
            out.push(Issue::param("command", m));
        }
        if params
            .get("params")
            .is_some_and(|p| !(p.is_object() || p.is_null()))
        {
            out.push(Issue::param(
                "params",
                fp_i18n::t("batch-issue-params-not-object"),
            ));
        }
        out
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let cmd = p_str(params, "command", "filter.blur.gaussian").to_string();
        if !cmd.starts_with("filter.") {
            return Err(BatchError::Invalid(fp_i18n::tr!(
                "batch-issue-script-unknown",
                command = cmd
            )));
        }
        if let Some(m) = check_command(&cmd) {
            return Err(BatchError::Invalid(m));
        }
        let p = params
            .get("params")
            .filter(|v| v.is_object())
            .cloned()
            .unwrap_or(json!({}));
        run_on_item(item, ctx, false, &|s| {
            s.execute("select.all", json!({}))
                .map_err(|e| BatchError::Invalid(e.to_string()))?;
            s.execute(&cmd, p.clone())
                .map_err(|e| BatchError::Invalid(e.to_string()))?;
            Ok(())
        })
    }
    fn summary(&self, p: &Value) -> String {
        let c = p_str(p, "command", "");
        let k = fp_i18n::command_key(c);
        if fp_i18n::has(&k) {
            fp_i18n::t(&k)
        } else {
            c.to_string()
        }
    }
}

// =================================================================== effect.live

/// `effect.live`: Firepixel-Live-Effekt auf das Bild als Element, danach flach gerechnet.
struct EffectLive;

/// Effekte von `effect.live` (Namen wie bei `element.addEffect`).
pub const LIVE_EFFECTS: [&str; 10] = [
    "dropShadow",
    "innerShadow",
    "glow",
    "innerGlow",
    "bevel",
    "outerBevel",
    "emboss",
    "insetEmboss",
    "solidShadow",
    "colorFill",
];

/// MOA-ID und erzwungener `BevelType` zu einem Effektnamen.
fn effect_moa(name: &str) -> Option<(&'static str, Option<&'static str>)> {
    use fp_model::EffectKind as K;
    let (kind, bevel) = match name {
        "dropShadow" => (K::DropShadow, None),
        "innerShadow" => (K::InnerShadow, None),
        "innerGlow" => (K::InnerGlow, None),
        "solidShadow" => (K::SolidShadow, None),
        "colorFill" => (K::ColorFill, None),
        "bevel" => (K::BevelFamily, Some("0")),
        "outerBevel" => (K::BevelFamily, Some("1")),
        "emboss" => (K::BevelFamily, Some("2")),
        "insetEmboss" => (K::BevelFamily, Some("3")),
        "glow" => (K::BevelFamily, Some("4")),
        _ => return None,
    };
    Some((kind.moa_id()?, bevel))
}

/// Effekt-Parameter (JSON-Objekt im Dateiformat: `{"ShadowBlur": "4", …}`) → Effektparameter.
fn effect_params(p: &Value) -> fp_raster::filters::EffectParams {
    let mut out = fp_raster::filters::EffectParams::new();
    if let Some(o) = p.as_object() {
        for (k, v) in o {
            let pv = match v {
                Value::String(s) => ParamValue::Text(s.clone()),
                Value::Array(a) => match points_of(&Value::Array(a.clone())) {
                    Some(pts) => ParamValue::Points(pts.iter().map(|q| (q.x, q.y)).collect()),
                    None => ParamValue::Text(v.to_string()),
                },
                Value::Null => continue,
                other => ParamValue::Text(other.to_string()),
            };
            out.insert(k.clone(), pv);
        }
    }
    out
}

/// Pixelangaben eines Live-Effekts (Abstand, Weichheit, Breite) mit dem Vorschau-Maßstab
/// verkleinern, damit der Effekt auf dem Proxy so groß wirkt wie im Ergebnis.
pub(crate) fn scale_live_effect(e: &mut fp_effects::LiveEffect, s: f64) {
    use fp_effects::LiveEffect as L;
    let si = |v: i64| ((v as f64) * s).round() as i64;
    let su = |v: u32| ((v as f64) * s).round().max(0.0) as u32;
    match e {
        L::DropShadow(p) | L::InnerShadow(p) => {
            p.blur = su(p.blur);
            p.distance *= s;
        }
        L::Bevel(p) => {
            p.width = si(p.width).max(1);
            p.angle_softness = si(p.angle_softness);
            p.mask_softness = si(p.mask_softness);
            p.light_distance = si(p.light_distance);
        }
        L::InnerGlow(p) => {
            p.softness = su(p.softness);
            p.width = su(p.width).max(1);
            p.start = su(p.start);
        }
        L::SolidShadow(p) => p.distance *= s,
        _ => {}
    }
}

impl Step for EffectLive {
    fn id(&self) -> &str {
        EFFECT_LIVE
    }
    fn group(&self) -> StepGroup {
        StepGroup::Color
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::choice("effect", &LIVE_EFFECTS, "dropShadow").beginner(),
            ParamDef::json("params", json!({})),
            ParamDef::bool("expandCanvas", true).beginner(),
        ]
    }
    fn validate(&self, params: &Value) -> Vec<Issue> {
        let mut out = validate_params(&self.params(), params);
        if params
            .get("params")
            .is_some_and(|p| !(p.is_object() || p.is_null()))
        {
            out.push(Issue::param(
                "params",
                fp_i18n::t("batch-issue-params-not-object"),
            ));
        }
        out
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let name = p_str(params, "effect", "dropShadow");
        let (moa, bevel) = effect_moa(name).ok_or_else(|| {
            BatchError::Invalid(fp_i18n::tr!(
                "batch-issue-param-enum",
                param = "effect",
                values = LIVE_EFFECTS.join(", ")
            ))
        })?;
        let mut ep = effect_params(params.get("params").unwrap_or(&Value::Null));
        if let Some(b) = bevel {
            ep.insert("BevelType".into(), ParamValue::Text(b.into()));
        }
        let mut effect = fp_effects::LiveEffect::parse(moa, &ep);
        // Vorschau auf dem Proxy: Pixelangaben des Effekts mitverkleinern (BATCH-11a, Befund BATCH-4)
        let scale = preview_scale(item, ctx);
        if scale < 1.0 {
            scale_live_effect(&mut effect, scale);
        }
        let entry = fp_effects::EffectEntry {
            effect,
            visible: true,
        };
        let fx_ctx = fp_effects::EffectContext::default();
        let expand = p_bool(params, "expandCanvas", true);
        let mut frames = frames_mut(item)?;
        // erst alle Frames rechnen, dann gemeinsame Grenzen (Animation: alle Frames gleich groß)
        let mut results = Vec::with_capacity(frames.len());
        let mut issues = Vec::new();
        let mut target: Option<fp_raster::IRect> = None;
        for img in frames.iter() {
            ctx.check_cancel()?;
            let orig = img.bounds();
            let r = fp_effects::apply_chain(img, std::slice::from_ref(&entry), &fx_ctx);
            issues.extend(r.issues);
            let b = if expand {
                let ab = r.image.alpha_bounds();
                if ab.is_empty() { orig } else { ab.union(&orig) }
            } else {
                orig
            };
            target = Some(target.map_or(b, |t| t.union(&b)));
            results.push(r.image);
        }
        let Some(target) = target else {
            return Ok(());
        };
        if (target.width() as u64) * (target.height() as u64) > fp_raster::MAX_PIXELS as u64 {
            return Err(crate::steps::geometry::too_large(
                target.width() as i64,
                target.height() as i64,
            ));
        }
        for (img, r) in frames.iter_mut().zip(results) {
            **img = r
                .copy_rect(target)
                .map_err(|e| BatchError::Invalid(e.to_string()))?;
        }
        drop(frames);
        if !issues.is_empty() {
            ctx.warn(fp_i18n::tr!(
                "batch-warn-effect-unsupported",
                effect = name.to_string()
            ));
        }
        finish(item);
        refresh_transparency(item);
        Ok(())
    }
    fn summary(&self, p: &Value) -> String {
        fp_i18n::t(&format!(
            "batch-value-effect-live-effect-{}",
            p_str(p, "effect", "dropShadow")
        ))
    }
}

// =================================================================== filter.screentone

/// `filter.screentone`: the Screentone live filter (Firepixel extension, FB-38) on the pixels of
/// every image, with the same parameters as its dialog; `antiAlias` (FB-86) smooths the dot edges.
struct FilterScreentone;

/// Patterns of `filter.screentone` (file values 0, 1, 2 of `screentone_type`).
pub const SCREENTONE_PATTERNS: [&str; 3] = ["dots", "lines", "crossHatch"];

/// Screentone parameters of a `filter.screentone` step (`scale` < 1 on the preview proxy).
pub fn screentone_params(p: &Value, scale: f64) -> fp_effects::ScreentoneParams {
    let d = fp_effects::ScreentoneParams::default();
    let kind = SCREENTONE_PATTERNS
        .iter()
        .position(|k| *k == p_str(p, "pattern", "dots"))
        .unwrap_or(0);
    let (lo, hi) = fp_effects::ScreentoneParams::SIZE_RANGE;
    let c = p_color(p, "color", [0, 0, 0, 255]);
    fp_effects::ScreentoneParams {
        kind: fp_effects::ScreentoneKind::from_i64(kind as i64),
        size: (p_f64(p, "size", d.size).clamp(lo, hi) * scale).max(lo),
        angle: p_f64(p, "angle", d.angle).clamp(0.0, 180.0),
        density: p_f64(p, "density", d.density).clamp(0.0, 100.0),
        from_luminance: p_bool(p, "fromLuminance", d.from_luminance),
        color: fp_color::Rgba8::new(c[0], c[1], c[2], c[3]),
        keep_color: p_bool(p, "keepColor", d.keep_color),
        anti_alias: p_bool(p, "antiAlias", d.anti_alias),
    }
}

impl Step for FilterScreentone {
    fn id(&self) -> &str {
        FILTER_SCREENTONE
    }
    fn group(&self) -> StepGroup {
        StepGroup::Color
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::choice("pattern", &SCREENTONE_PATTERNS, "dots").beginner(),
            ParamDef::int("size", 2, 32, 8).unit("px").beginner(),
            ParamDef::int("angle", 0, 180, 45).unit("°"),
            ParamDef::bool("fromLuminance", true),
            ParamDef::int("density", 0, 100, 50).unit("%"),
            ParamDef::color("color", "#000000"),
            ParamDef::bool("keepColor", false),
            ParamDef::bool("antiAlias", false).beginner(),
        ]
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let scale = preview_scale(item, ctx);
        let p = screentone_params(params, scale.min(1.0));
        for img in frames_mut(item)? {
            ctx.check_cancel()?;
            *img = fp_effects::screentone::screentone(img, &p);
        }
        finish(item);
        refresh_transparency(item);
        Ok(())
    }
}
