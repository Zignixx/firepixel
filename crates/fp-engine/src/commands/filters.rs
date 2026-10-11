//! Menü „Filter“ (Spezifikation 06): alle eingebauten Bitmap-Filter über fp-raster.
//!
//! Die Filterliste baut das Original zur Laufzeit (IDs 43501–44000, 10 §4.1 Nr. 6); Firepixel
//! registriert sie fest. Ausgewählte Bitmaps werden destruktiv gefiltert (mit Pixelauswahl nur
//! innerhalb der Maske, 06 §8). Andere Objekte erhalten den Filter als Live-Effekt (`EFLv`, A).
//! „Filter wiederholen“ wiederholt den letzten Filteraufruf mit denselben Parametern.

use super::edit::selected_bitmaps;
use super::{selected_elements, with_doc, SEL};
use crate::command::{cmd, CommandOutput, CommandRegistry, ParamKind, Params, Requirement};
use crate::docops::{self, bitmap_image, set_bitmap_image};
use crate::error::{EngineError, Result};
use crate::session::Session;
use fp_model::effects::Effect;
use fp_model::element::ElementKind;
use fp_raster::filters::{
    AddNoiseParams, BrightnessContrastParams, CurvesParams, GaussianBlurParams, HueSaturationParams, LevelsParams, MotionBlurParams, RadialBlurParams,
    UnsharpMaskParams, ZoomBlurParams,
};
use fp_raster::{Filter, PixelFilter};
use serde_json::Value;
use std::sync::Arc;

/// Filterbefehle: (Name, Beschriftung, Menüpfad).
pub const FILTERS: &[(&str, &str, &[&str])] = &[
    ("filter.adjustColor.autoLevels", "Auto-Tonwertkorrektur", &["Filter", "Farbe anpassen", "Auto-Tonwertkorrektur"]),
    ("filter.adjustColor.brightnessContrast", "Helligkeit/Kontrast", &["Filter", "Farbe anpassen", "Helligkeit/Kontrast..."]),
    ("filter.adjustColor.colorFill", "Farbfüllung", &["Filter", "Farbe anpassen", "Farbfüllung..."]),
    ("filter.adjustColor.curves", "Gradationskurven", &["Filter", "Farbe anpassen", "Gradationskurven..."]),
    ("filter.adjustColor.hueSaturation", "Farbton/Sättigung", &["Filter", "Farbe anpassen", "Farbton/Sättigung..."]),
    ("filter.adjustColor.invert", "Umkehren", &["Filter", "Farbe anpassen", "Umkehren"]),
    ("filter.adjustColor.levels", "Tonwertkorrektur", &["Filter", "Farbe anpassen", "Tonwertkorrektur..."]),
    ("filter.blur.blur", "Weichzeichnen", &["Filter", "Weichzeichnen", "Weichzeichnen"]),
    ("filter.blur.blurMore", "Stark weichzeichnen", &["Filter", "Weichzeichnen", "Stark weichzeichnen"]),
    ("filter.blur.gaussian", "Gaußscher Weichzeichner", &["Filter", "Weichzeichnen", "Gaußscher Weichzeichner..."]),
    ("filter.blur.motion", "Bewegungsunschärfe", &["Filter", "Weichzeichnen", "Bewegungsunschärfe..."]),
    ("filter.blur.radial", "Radialer Weichzeichner", &["Filter", "Weichzeichnen", "Radialer Weichzeichner..."]),
    ("filter.blur.zoom", "Zoom-Weichzeichner", &["Filter", "Weichzeichnen", "Zoom-Weichzeichner..."]),
    ("filter.noise.addNoise", "Rauschen hinzufügen", &["Filter", "Rauschen", "Rauschen hinzufügen..."]),
    ("filter.other.convertToAlpha", "In Alpha konvertieren", &["Filter", "Andere", "In Alpha konvertieren"]),
    ("filter.other.findEdges", "Kanten suchen", &["Filter", "Andere", "Kanten suchen"]),
    ("filter.sharpen.sharpen", "Scharfzeichnen", &["Filter", "Scharfzeichnen", "Scharfzeichnen"]),
    ("filter.sharpen.sharpenMore", "Stark scharfzeichnen", &["Filter", "Scharfzeichnen", "Stark scharfzeichnen"]),
    ("filter.sharpen.unsharpMask", "Unscharf maskieren", &["Filter", "Scharfzeichnen", "Unscharf maskieren..."]),
];

fn params<T: serde::de::DeserializeOwned + Default>(p: &Params) -> Result<T> {
    if p.value.is_null() {
        return Ok(T::default());
    }
    let mut v = p.value.clone();
    if let Value::Object(m) = &mut v {
        m.remove("live");
    }
    // Zahlenfelder der Dialoge liefern auch ganzzahlige Werte als Gleitkomma (`26.0`)
    whole_numbers_as_ints(&mut v);
    serde_json::from_value(v).map_err(|e| EngineError::InvalidParams { name: p.command.clone(), message: e.to_string() })
}

/// Ersetzt ganzzahlige Gleitkommawerte (`26.0`) rekursiv durch Ganzzahlen, damit sie in
/// `i32`-Felder passen (Gleitkommafelder nehmen Ganzzahlen ohnehin an).
pub fn whole_numbers_as_ints(v: &mut Value) {
    match v {
        Value::Number(n) => {
            if !n.is_i64()
                && !n.is_u64()
                && let Some(f) = n.as_f64()
                && f.fract() == 0.0
                && f.abs() < 9.0e15
            {
                *v = Value::from(f as i64);
            }
        }
        Value::Array(a) => a.iter_mut().for_each(whole_numbers_as_ints),
        Value::Object(o) => o.values_mut().for_each(whole_numbers_as_ints),
        _ => {}
    }
}

/// Filter zu Befehl und Parametern.
pub fn filter_for(name: &str, p: &Params) -> Result<Filter> {
    Ok(match name {
        "filter.adjustColor.autoLevels" => Filter::AutoLevels,
        "filter.adjustColor.brightnessContrast" => Filter::BrightnessContrast(params::<BrightnessContrastParams>(p)?),
        "filter.adjustColor.colorFill" => Filter::ColorFill(fp_raster::filters::ColorFillParams {
            color: p.opt_color("color")?.map(|c| c.to_rgba8()).unwrap_or(fp_color::Rgba8::rgb(255, 0, 0)),
            opacity: p.i64_or("opacity", 100)?.clamp(0, 100) as i32,
            blend_mode: fp_color::BlendMode::from_file(Some(p.i64_or("blendMode", 0)?)),
        }),
        "filter.adjustColor.curves" => Filter::Curves(params::<CurvesParams>(p)?),
        "filter.adjustColor.hueSaturation" => Filter::HueSaturation(params::<HueSaturationParams>(p)?),
        "filter.adjustColor.invert" => Filter::Invert,
        "filter.adjustColor.levels" => Filter::Levels(params::<LevelsParams>(p)?),
        "filter.blur.blur" => Filter::Blur,
        "filter.blur.blurMore" => Filter::BlurMore,
        "filter.blur.gaussian" => Filter::GaussianBlur(params::<GaussianBlurParams>(p)?),
        "filter.blur.motion" => Filter::MotionBlur(params::<MotionBlurParams>(p)?),
        "filter.blur.radial" => Filter::RadialBlur(params::<RadialBlurParams>(p)?),
        "filter.blur.zoom" => Filter::ZoomBlur(params::<ZoomBlurParams>(p)?),
        "filter.noise.addNoise" => Filter::AddNoise(params::<AddNoiseParams>(p)?),
        "filter.other.convertToAlpha" => Filter::ConvertToAlpha,
        "filter.other.findEdges" => Filter::FindEdges,
        "filter.sharpen.sharpen" => Filter::menu_sharpen(),
        "filter.sharpen.sharpenMore" => Filter::menu_sharpen_more(),
        "filter.sharpen.unsharpMask" => Filter::UnsharpMask(params::<UnsharpMaskParams>(p)?),
        other => return Err(EngineError::UnknownCommand(other.into())),
    })
}

/// Live-Effekt-Eintrag (`EPSv`) zu einem Filter, Parameternamen wie im Dateiformat (06 §2.2).
pub fn live_effect(f: &Filter) -> Effect {
    let mut e = Effect::new(&format!("{{{}}}", f.moa_id()), true);
    let mut set = |k: &str, v: String| e.set(k, v);
    match f {
        Filter::BrightnessContrast(p) => {
            set("brightness_amount", p.brightness.to_string());
            set("contrast_amount", p.contrast.to_string());
        }
        Filter::HueSaturation(p) => {
            set("hue_amount", p.hue.to_string());
            set("saturation_amount", p.saturation.to_string());
            set("lightness_amount", p.lightness.to_string());
            set("hls_colorize", p.colorize.to_string());
        }
        Filter::GaussianBlur(p) => set("gaussian_blur_radius", p.radius.to_string()),
        Filter::MotionBlur(p) => {
            set("motion_blur_angle", p.angle.to_string());
            set("motion_blur_distance", p.distance.to_string());
        }
        Filter::RadialBlur(p) => {
            set("radial_blur_amount", p.amount.to_string());
            set("radial_blur_quality", p.quality.to_string());
        }
        Filter::ZoomBlur(p) => {
            set("zoom_blur_amount", p.amount.to_string());
            set("zoom_blur_quality", p.quality.to_string());
            if let Some((x, y)) = p.center {
                set("zoom_blur_center_x", x.to_string());
                set("zoom_blur_center_y", y.to_string());
            }
        }
        Filter::AddNoise(p) => {
            set("add_noise_amount", p.amount.to_string());
            set("add_noise_use_color", p.use_color.to_string());
        }
        Filter::Sharpen(p) | Filter::SharpenMore(p) => set("sharpen_amount", p.amount.to_string()),
        Filter::UnsharpMask(p) => {
            set("unsharp_mask_radius", p.radius.to_string());
            set("unsharp_mask_amount", p.amount.to_string());
            set("unsharp_mask_threshold", p.threshold.to_string());
        }
        Filter::SimpleBlur(p) => set("simple_blur_amount", p.amount.to_string()),
        Filter::ColorFill(p) => {
            set("Color", format!("#{:02X}{:02X}{:02X}", p.color.r, p.color.g, p.color.b));
            set("Opacity", p.opacity.to_string());
            set("Blendmode", p.blend_mode.0.to_string());
        }
        _ => {}
    }
    e
}

/// Wendet einen Filter auf die Auswahl an.
pub fn apply_filter(s: &mut Session, f: &Filter, live: bool) -> Result<CommandOutput> {
    with_doc(s, |d| {
        let page = d.page;
        let mask = d.selection.pixels.clone().filter(|_| d.selection.has_pixels());
        let bitmaps: Vec<_> = if live { Vec::new() } else { selected_bitmaps(d) };
        let others: Vec<_> = selected_elements(d).into_iter().filter(|(_, e)| live || !matches!(e.kind, ElementKind::Bitmap(_))).collect();
        if bitmaps.is_empty() && others.is_empty() {
            return Err(super::sel_err("nichts zum Filtern ausgewählt"));
        }
        let doc = Arc::make_mut(&mut d.doc);
        for (path, _) in bitmaps {
            docops::edit_at(doc, &path, |e| {
                if let ElementKind::Bitmap(b) = &mut e.kind
                    && let Some(img) = bitmap_image(b)
                {
                    let out = f.apply(&img, mask.as_ref());
                    set_bitmap_image(b, &out);
                }
            });
        }
        let eff = live_effect(f);
        for (_, e) in others {
            docops::edit_by_id(doc, page, e.id, |x| {
                let mut l = x.common.effects.clone().unwrap_or_default();
                l.effects.push(eff.clone());
                super::draw::set_effects(x, Some(l));
            });
        }
        Ok(CommandOutput::doc())
    })
}

pub(super) fn register(r: &mut CommandRegistry) {
    for &(name, label, path) in FILTERS {
        let mut b = cmd(name).label(label).requires(SEL).opt("live", ParamKind::Bool, "als Live-Effekt anwenden statt destruktiv");
        b = match name {
            "filter.adjustColor.brightnessContrast" => b
                .opt("brightness", ParamKind::Integer { min: Some(-100), max: Some(100) }, "Helligkeit")
                .opt("contrast", ParamKind::Integer { min: Some(-100), max: Some(100) }, "Kontrast"),
            "filter.adjustColor.hueSaturation" => b
                .opt("hue", ParamKind::Integer { min: Some(-180), max: Some(360) }, "Farbton")
                .opt("saturation", ParamKind::Integer { min: Some(-100), max: Some(100) }, "Sättigung")
                .opt("lightness", ParamKind::Integer { min: Some(-100), max: Some(100) }, "Helligkeit")
                .opt("colorize", ParamKind::Bool, "Kolorieren"),
            "filter.adjustColor.curves" => b
                .opt("rgb_points", ParamKind::Json, "Kurvenpunkte RGB [{x, y}, …]")
                .opt("red_points", ParamKind::Json, "Rot")
                .opt("green_points", ParamKind::Json, "Grün")
                .opt("blue_points", ParamKind::Json, "Blau"),
            "filter.adjustColor.levels" => b.opt("rgb", ParamKind::Json, "{source_low, source_high, dest_low, dest_high, gamma}").opt("red", ParamKind::Json, "Rot").opt("green", ParamKind::Json, "Grün").opt("blue", ParamKind::Json, "Blau"),
            "filter.adjustColor.colorFill" => b
                .opt("color", ParamKind::Color, "Füllfarbe")
                .opt("opacity", ParamKind::Integer { min: Some(0), max: Some(100) }, "Deckkraft in %")
                .opt("blendMode", ParamKind::Integer { min: Some(0), max: Some(255) }, "Mischmodus (BLDi)"),
            "filter.blur.gaussian" => b.opt("radius", ParamKind::Number { min: Some(0.1), max: Some(250.0) }, "Radius (Vorgabe 4,1)"),
            "filter.blur.motion" => b.opt("angle", ParamKind::Integer { min: Some(0), max: Some(360) }, "Winkel").opt("distance", ParamKind::Integer { min: Some(1), max: Some(100) }, "Distanz"),
            "filter.blur.radial" | "filter.blur.zoom" => b.opt("amount", ParamKind::Integer { min: Some(1), max: Some(100) }, "Stärke").opt("quality", ParamKind::Integer { min: Some(1), max: Some(100) }, "Qualität"),
            "filter.noise.addNoise" => b.opt("amount", ParamKind::Integer { min: Some(1), max: Some(400) }, "Stärke in %").opt("use_color", ParamKind::Bool, "farbig"),
            "filter.sharpen.unsharpMask" => b
                .opt("radius", ParamKind::Number { min: Some(0.1), max: Some(250.0) }, "Radius")
                .opt("amount", ParamKind::Integer { min: Some(1), max: Some(500) }, "Stärke in %")
                .opt("threshold", ParamKind::Integer { min: Some(0), max: Some(255) }, "Schwellenwert"),
            _ => b,
        };
        // Menüpfad selbst setzen (dynamische Filtereinträge stehen nicht im Katalog)
        r.add(b.menu(path).run(move |s, p| {
            let f = filter_for(name, p)?;
            let live = p.bool_or("live", false)?;
            apply_filter(s, &f, live)
        }));
    }
    r.add(cmd("filter.repeat").requires(&[Requirement::Document, Requirement::Selection]).available(|s| s.last_filter.is_some()).run(|s, _| {
        let last = s.last_filter.clone().ok_or_else(|| EngineError::Failed("noch kein Filter angewandt".into()))?;
        let p = Params::new(&last.command, last.params.clone());
        let f = filter_for(&last.command, &p)?;
        let live = p.bool_or("live", false)?;
        apply_filter(s, &f, live).map(|o| o.labeled("Filter wiederholen"))
    }));
}
