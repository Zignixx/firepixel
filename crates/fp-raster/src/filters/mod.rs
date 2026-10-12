//! Die eingebauten Bitmap-Filter des Originals (Spezifikation 06, Referenzen
//! `re\filter\fwfilters.py`/`fweffects.py`).
//!
//! # Aufbau
//!
//! Jeder Filter hat ein Parameter-Struct (serde-fähig, Standardwerte wie im Original, Dialog-
//! bereiche als Konstanten bzw. `dialog_clamped`) und eine Funktion
//! `apply(&RgbaImage, &Params, Option<&Mask>) -> RgbaImage` im jeweiligen Modul. Gemeinsam ist der
//! Rahmen des Originals (`@00b2f380`, 06 §2.4, §8), umgesetzt in [`PixelFilter::apply`]:
//!
//! 1. Ausgabegrenzen bestimmen ([`PixelFilter::output_bounds`]; Gauß, Bewegung und Radial
//!    vergrößern, alle anderen nicht) und die Eingabe **transparent** darauf erweitern.
//! 2. Ohne Maske: den ganzen Puffer rechnen ([`PixelFilter::process`]).
//! 3. Mit Maske (Pixelauswahl): Rechenbereich auf die Maskengrenzen beschneiden, rechnen und das
//!    Ergebnis **durch die Maske** per skalarem Original-„over“ über das Original mischen
//!    (06 §8, Laufzeit T7: 10.000/10.000). Ergebnisgrenzen: Eingabe ∪ Rechenbereich (A).
//!
//! Filter, die vom ganzen Bereich abhängen (Helligkeit/Kontrast: Mittelwert; Auto-Tonwert:
//! Histogramm; Gauß > 200: Mittelwertfarbe; Rauschen: Tabellengröße), werten genau den
//! Rechenbereich aus.
//!
//! Fehlerfall (Puffer zu groß, siehe [`crate::MAX_PIXELS`]): Die Eingabe wird unverändert
//! zurückgegeben; Filter erzeugen nie eine Panik.
//!
//! Live-Filter aus Dateien (EffectMoaID + Parameter-Strings) bildet [`Filter::from_effect`] auf
//! die Typen ab (wie `fweffects.apply_effect`).

pub mod blur;
pub mod color;
pub mod convolve;
pub mod curve_edit;
pub mod gauss;
pub mod noise;
pub mod table;
pub mod levels_math;

use std::collections::BTreeMap;

use fp_color::blend::{blend_pixel, OverVariant};
use fp_color::{BlendMode, Rgba8};
use fp_geom::IRect;

use crate::image::RgbaImage;
use crate::mask::Mask;

pub use blur::{MotionBlurParams, RadialBlurParams, ZoomBlurParams};
pub use color::{BrightnessContrastParams, ColorFillParams, ConvertToAlpha, HueSaturationParams};
pub use convolve::{Blur, BlurMore, FindEdges, SharpenParams, SimpleBlurParams};
pub use gauss::{GaussPrecision, GaussianBlurParams, UnsharpMaskParams};
pub use noise::AddNoiseParams;
pub use table::{AutoLevels, CurvePoint, CurvesParams, Invert, LevelsChannel, LevelsParams};

/// Gemeinsame Schnittstelle aller Pixelfilter (im Original `CMbPixelFilter`).
pub trait PixelFilter {
    /// Ausgabegrenzen aus Eingabegrenzen (Vtable-Slot 5; Standard: unverändert).
    fn output_bounds(&self, input: IRect) -> IRect {
        input
    }

    /// Rechnet den Filter (Arbeitsfunktion). `src` ist bereits auf [`Self::output_bounds`]
    /// erweitert; das Ergebnis hat dieselben Grenzen wie `src`. Gerechnet wird nur `region`
    /// (⊆ Grenzen von `src`); Nachbarschaftsfilter lesen auch außerhalb davon. Pixel außerhalb von
    /// `region` sind Kopien von `src`.
    fn process(&self, src: &RgbaImage, region: IRect) -> RgbaImage;

    /// Anwenden mit dem Original-Rahmen (siehe Moduldoku), optional durch eine Auswahlmaske.
    fn apply(&self, img: &RgbaImage, mask: Option<&Mask>) -> RgbaImage {
        apply_framework(self, img, mask)
    }
}

/// Der gemeinsame Rahmen `@00b2f380`/`@00b2f140` (06 §2.4, §8).
pub fn apply_framework<F: PixelFilter + ?Sized>(f: &F, img: &RgbaImage, mask: Option<&Mask>) -> RgbaImage {
    let ob = f.output_bounds(img.bounds());
    let src = match img.extended_to(ob) {
        Ok(s) => s,
        Err(_) => return img.clone(),
    };
    let Some(m) = mask else {
        return f.process(&src, src.bounds());
    };
    let region = m.tight_bounds().intersect(&src.bounds());
    if region.is_empty() {
        return img.clone();
    }
    let filtered = f.process(&src, region);
    let mut out = src;
    let (ob, w) = (out.bounds(), out.width());
    let data = out.data_mut();
    for y in region.top..region.bottom {
        let frow = filtered.row(y).unwrap_or(&[]);
        for x in region.left..region.right {
            let mv = m.get(x, y);
            if mv == 0 {
                continue;
            }
            let i = (y - ob.top) as usize * w + (x - ob.left) as usize;
            let fp = frow.get((x - ob.left) as usize).copied().unwrap_or(data[i]);
            data[i] = blend_pixel(Rgba8::from_array(data[i]), Rgba8::from_array(fp), BlendMode(0), 255, mv, OverVariant::Scalar)
                .to_array();
        }
    }
    out.cropped(img.bounds().union(&region))
}

/// Zuschnitt nach dem Menübefehl „Filter“ an einer Bitmap: Der vergrößerte Ausgabepuffer wird auf
/// die Pixel mit Alpha > 0 zugeschnitten, die Eingabegrenzen bleiben immer erhalten.
///
/// Gemessen am Original (PM-6, PARITAET_B D47–D49, B1 = 320,200,420,300 deckend): Radial 10/20
/// → 317,197,424,304 (Puffer 299,179,441,321), Bewegung 90/20 → 320,200,420,319, Gauß 2 →
/// 315,195,425,305. Wie die gespeicherten Caches (06 §2.4). Dass Eingabegrenzen mit
/// durchsichtigem Rand erhalten bleiben, ist angenommen (A).
pub fn trim_growth(input: IRect, out: &RgbaImage) -> RgbaImage {
    let ob = out.bounds();
    let r = out.alpha_bounds().union(&input).intersect(&ob);
    if r == ob || r.is_empty() {
        return out.clone();
    }
    out.cropped(r)
}

/// Startet das Ergebnis eines Filters als Kopie der Eingabe (das Original füllt `dst` vorab mit
/// `src`, 06 §2.4).
pub(crate) fn start(src: &RgbaImage) -> RgbaImage {
    src.clone()
}

/// Parameterwert eines Live-Filter-Eintrags (`EPSv`/`DCKs`): fast immer ein String, bei Kurven
/// eine Punktliste (`GPLv{GPTv{XLCf YLCf}}`).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum ParamValue {
    /// Wert als Text, wie in der Datei.
    Text(String),
    /// Punktliste (x, y) als `f32`.
    Points(Vec<(f32, f32)>),
}

/// Parameter eines Live-Filter-Eintrags (Schlüssel → Wert).
pub type EffectParams = BTreeMap<String, ParamValue>;

fn p_text<'a>(p: &'a EffectParams, k: &str) -> Option<&'a str> {
    match p.get(k) {
        Some(ParamValue::Text(s)) => Some(s.trim()),
        _ => None,
    }
}

/// Zahl wie `fweffects._num`: `float(text)`, sonst `default`.
fn p_num(p: &EffectParams, k: &str, default: f64) -> f64 {
    p_text(p, k).and_then(|s| s.parse::<f64>().ok()).unwrap_or(default)
}

/// Ganzzahl wie `int(float(text))` (abschneiden, gesättigt).
fn p_int(p: &EffectParams, k: &str, default: i32) -> i32 {
    let v = p_num(p, k, default as f64);
    if v.is_finite() { v.trunc().clamp(i32::MIN as f64, i32::MAX as f64) as i32 } else { default }
}

fn p_bool(p: &EffectParams, k: &str) -> bool {
    p_text(p, k) == Some("true")
}

fn p_points(p: &EffectParams, k: &str) -> Vec<CurvePoint> {
    match p.get(k) {
        Some(ParamValue::Points(v)) => v.iter().map(|&(x, y)| CurvePoint { x, y }).collect(),
        _ => Vec::new(),
    }
}

/// EffectMoaIDs der Filter (06 §2.2), kleingeschrieben ohne Klammern.
pub mod moa {
    pub const BRIGHTNESS_CONTRAST: &str = "3439b08c-1921-11d3-9bde00e02910d580";
    pub const HUE_SATURATION: &str = "3439b08d-1922-11d3-9bde00e02910d580";
    pub const CURVES: &str = "3439b08e-1923-11d3-9bde00e02910d580";
    pub const LEVELS: &str = "d04ef8c2-71b5-11d1-8c8200a024cdc039";
    pub const AUTO_LEVELS: &str = "d04ef8c1-71b4-11d1-8c8200a024cdc039";
    pub const INVERT: &str = "d2541291-70d6-11d1-8c8000a024cdc039";
    pub const GAUSSIAN_BLUR: &str = "d04ef8c0-71b3-11d1-8c8200a024cdc039";
    pub const BLUR: &str = "f1cfce41-718e-11d1-8c8200a024cdc039";
    pub const BLUR_MORE: &str = "f1cfce42-718e-11d1-8c8200a024cdc039";
    pub const MOTION_BLUR: &str = "538016b6-ffd6-418f-a10c590e6ee841ae";
    pub const RADIAL_BLUR: &str = "d1c33142-2ad8-4215-a39fc5d934c8fc0c";
    pub const ZOOM_BLUR: &str = "94dbd663-9b2e-44bf-8064ab6efdd2d327";
    pub const SHARPEN: &str = "c20952b1-fc76-11d0-8be700a024cdc039";
    pub const SHARPEN_MORE: &str = "1f2f2591-9db7-11d1-8cad00a024cdc039";
    pub const UNSHARP_MASK: &str = "f1cfce44-718e-11d1-8c8200a024cdc039";
    pub const ADD_NOISE: &str = "e4c0f4bc-c0a3-4cb3-b3513822027e4d9f";
    pub const FIND_EDGES: &str = "fc7093f1-f95c-11d0-8be200a024cdc039";
    pub const CONVERT_TO_ALPHA: &str = "2932d5a2-ca48-11d1-8561000502701850";
    pub const SIMPLE_BLUR: &str = "a2dea491-fd45-11d0-8be900a024cdc039";
    pub const COLOR_FILL: &str = "dd54adc0-a279-11d3-b92a000502f3fdbe";
}

/// Ein Filter mit Parametern (alle eingebauten Filter plus Farbfüllung).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "filter", content = "params", rename_all = "snake_case")]
pub enum Filter {
    BrightnessContrast(BrightnessContrastParams),
    HueSaturation(HueSaturationParams),
    Curves(CurvesParams),
    Levels(LevelsParams),
    AutoLevels,
    Invert,
    GaussianBlur(GaussianBlurParams),
    Blur,
    BlurMore,
    SimpleBlur(SimpleBlurParams),
    Sharpen(SharpenParams),
    SharpenMore(SharpenParams),
    UnsharpMask(UnsharpMaskParams),
    MotionBlur(MotionBlurParams),
    RadialBlur(RadialBlurParams),
    ZoomBlur(ZoomBlurParams),
    AddNoise(AddNoiseParams),
    FindEdges,
    ConvertToAlpha,
    ColorFill(ColorFillParams),
}

impl Filter {
    /// Live-Filter aus EffectMoaID (mit oder ohne Klammern) und Parametern der Datei
    /// (wie `fweffects.apply_effect`). `None` für unbekannte IDs (Wolken, Photoshop-Plugins,
    /// Live-Effekte anderer Art).
    ///
    /// Schärfen/Stark schärfen rechnen als Live-Filter mit `sharpen_amount` aus der Datei
    /// (fehlt → 0 → Stärke 1, 06 §4.9). Der Zoom-Mittelpunkt gilt nur, wenn beide Schlüssel
    /// vorhanden sind.
    pub fn from_effect(moa_id: &str, p: &EffectParams) -> Option<Filter> {
        let id = moa_id.trim().trim_start_matches('{').trim_end_matches('}').to_ascii_lowercase();
        let f = match id.as_str() {
            moa::BRIGHTNESS_CONTRAST => Filter::BrightnessContrast(BrightnessContrastParams {
                brightness: p_int(p, "brightness_amount", 0),
                contrast: p_int(p, "contrast_amount", 0),
            }),
            moa::HUE_SATURATION => Filter::HueSaturation(HueSaturationParams {
                hue: p_int(p, "hue_amount", 0),
                saturation: p_int(p, "saturation_amount", 0),
                lightness: p_int(p, "lightness_amount", 0),
                colorize: p_bool(p, "hls_colorize"),
            }),
            moa::CURVES => Filter::Curves(CurvesParams {
                rgb_points: p_points(p, "rgb_points"),
                red_points: p_points(p, "red_points"),
                green_points: p_points(p, "green_points"),
                blue_points: p_points(p, "blue_points"),
            }),
            moa::LEVELS => {
                let ch = |n: &str| LevelsChannel {
                    source_low: p_int(p, &format!("source_low_{n}"), 0),
                    source_high: p_int(p, &format!("source_high_{n}"), 255),
                    dest_low: p_int(p, &format!("dest_low_{n}"), 0),
                    dest_high: p_int(p, &format!("dest_high_{n}"), 255),
                    gamma: p_num(p, &format!("gamma_{n}"), 1.0),
                };
                Filter::Levels(LevelsParams { rgb: ch("rgb"), red: ch("red"), green: ch("green"), blue: ch("blue") })
            }
            moa::AUTO_LEVELS => Filter::AutoLevels,
            moa::INVERT => Filter::Invert,
            moa::GAUSSIAN_BLUR => Filter::GaussianBlur(GaussianBlurParams {
                radius: p_num(p, "gaussian_blur_radius", 4.1),
                ..Default::default()
            }),
            moa::BLUR => Filter::Blur,
            moa::BLUR_MORE => Filter::BlurMore,
            moa::SIMPLE_BLUR => Filter::SimpleBlur(SimpleBlurParams { amount: p_int(p, "simple_blur_amount", 0) }),
            moa::SHARPEN => Filter::Sharpen(SharpenParams { amount: p_int(p, "sharpen_amount", 0) }),
            moa::SHARPEN_MORE => Filter::SharpenMore(SharpenParams { amount: p_int(p, "sharpen_amount", 0) }),
            moa::UNSHARP_MASK => Filter::UnsharpMask(UnsharpMaskParams {
                radius: p_num(p, "unsharp_mask_radius", 4.1),
                amount: p_int(p, "unsharp_mask_amount", 50),
                threshold: p_int(p, "unsharp_mask_threshold", 0),
                ..Default::default()
            }),
            moa::MOTION_BLUR => Filter::MotionBlur(MotionBlurParams {
                angle: p_int(p, "motion_blur_angle", 90),
                distance: p_int(p, "motion_blur_distance", 10),
            }),
            moa::RADIAL_BLUR => Filter::RadialBlur(RadialBlurParams {
                amount: p_int(p, "radial_blur_amount", 30),
                quality: p_int(p, "radial_blur_quality", 20),
            }),
            moa::ZOOM_BLUR => {
                let has = p.contains_key("zoom_blur_center_x") && p.contains_key("zoom_blur_center_y");
                Filter::ZoomBlur(ZoomBlurParams {
                    amount: p_int(p, "zoom_blur_amount", 30),
                    quality: p_int(p, "zoom_blur_quality", 20),
                    center: if has {
                        Some((p_int(p, "zoom_blur_center_x", 0), p_int(p, "zoom_blur_center_y", 0)))
                    } else {
                        None
                    },
                })
            }
            moa::ADD_NOISE => Filter::AddNoise(AddNoiseParams {
                amount: p_int(p, "add_noise_amount", 50),
                use_color: p_bool(p, "add_noise_use_color"),
            }),
            moa::FIND_EDGES => Filter::FindEdges,
            moa::CONVERT_TO_ALPHA => Filter::ConvertToAlpha,
            moa::COLOR_FILL => Filter::ColorFill(ColorFillParams {
                color: p_text(p, "Color").and_then(Rgba8::parse_hex).unwrap_or(Rgba8::rgb(255, 0, 0)),
                opacity: p_int(p, "Opacity", 100),
                blend_mode: BlendMode::from_file(Some(p_int(p, "Blendmode", 0) as i64)),
            }),
            _ => return None,
        };
        Some(f)
    }

    /// EffectMoaID ohne Klammern.
    pub fn moa_id(&self) -> &'static str {
        match self {
            Filter::BrightnessContrast(_) => moa::BRIGHTNESS_CONTRAST,
            Filter::HueSaturation(_) => moa::HUE_SATURATION,
            Filter::Curves(_) => moa::CURVES,
            Filter::Levels(_) => moa::LEVELS,
            Filter::AutoLevels => moa::AUTO_LEVELS,
            Filter::Invert => moa::INVERT,
            Filter::GaussianBlur(_) => moa::GAUSSIAN_BLUR,
            Filter::Blur => moa::BLUR,
            Filter::BlurMore => moa::BLUR_MORE,
            Filter::SimpleBlur(_) => moa::SIMPLE_BLUR,
            Filter::Sharpen(_) => moa::SHARPEN,
            Filter::SharpenMore(_) => moa::SHARPEN_MORE,
            Filter::UnsharpMask(_) => moa::UNSHARP_MASK,
            Filter::MotionBlur(_) => moa::MOTION_BLUR,
            Filter::RadialBlur(_) => moa::RADIAL_BLUR,
            Filter::ZoomBlur(_) => moa::ZOOM_BLUR,
            Filter::AddNoise(_) => moa::ADD_NOISE,
            Filter::FindEdges => moa::FIND_EDGES,
            Filter::ConvertToAlpha => moa::CONVERT_TO_ALPHA,
            Filter::ColorFill(_) => moa::COLOR_FILL,
        }
    }

    /// Menübefehl „Filter › …“: Schärfen/Stark schärfen nutzen dort das Prototyp-Objekt mit
    /// Stärke 4 bzw. 8 (06 §4.9, B Laufzeit T3); alle anderen Filter rechnen wie live.
    pub fn menu_sharpen() -> Filter {
        Filter::Sharpen(SharpenParams::MENU)
    }

    /// Menübefehl „Stark scharfzeichnen“ (Stärke 8).
    pub fn menu_sharpen_more() -> Filter {
        Filter::SharpenMore(SharpenParams::MENU_MORE)
    }

    fn as_pixel_filter(&self) -> &dyn PixelFilter {
        match self {
            Filter::BrightnessContrast(p) => p,
            Filter::HueSaturation(p) => p,
            Filter::Curves(p) => p,
            Filter::Levels(p) => p,
            Filter::AutoLevels => &AutoLevels,
            Filter::Invert => &Invert,
            Filter::GaussianBlur(p) => p,
            Filter::Blur => &Blur,
            Filter::BlurMore => &BlurMore,
            Filter::SimpleBlur(p) => p,
            Filter::Sharpen(p) | Filter::SharpenMore(p) => p,
            Filter::UnsharpMask(p) => p,
            Filter::MotionBlur(p) => p,
            Filter::RadialBlur(p) => p,
            Filter::ZoomBlur(p) => p,
            Filter::AddNoise(p) => p,
            Filter::FindEdges => &FindEdges,
            Filter::ConvertToAlpha => &ConvertToAlpha,
            Filter::ColorFill(p) => p,
        }
    }
}

impl PixelFilter for Filter {
    fn output_bounds(&self, input: IRect) -> IRect {
        self.as_pixel_filter().output_bounds(input)
    }
    fn process(&self, src: &RgbaImage, region: IRect) -> RgbaImage {
        self.as_pixel_filter().process(src, region)
    }
}

/// Wendet eine Live-Filter-Liste in Listenreihenfolge an (erster Eintrag zuerst, 06 §2.4); die
/// Grenzen wachsen mit jedem Schritt.
pub fn apply_chain(img: &RgbaImage, filters: &[Filter]) -> RgbaImage {
    let mut cur = img.clone();
    for f in filters {
        cur = f.apply(&cur, None);
    }
    cur
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_from_effect() {
        let mut p = EffectParams::new();
        p.insert("gaussian_blur_radius".into(), ParamValue::Text("2.5".into()));
        match Filter::from_effect("{D04EF8C0-71B3-11d1-8c8200a024cdc039}", &p) {
            Some(Filter::GaussianBlur(g)) => assert_eq!(g.radius, 2.5),
            other => panic!("{other:?}"),
        }
        let s = Filter::from_effect(moa::SHARPEN_MORE, &EffectParams::new());
        assert_eq!(s, Some(Filter::SharpenMore(SharpenParams { amount: 0 })));
        assert!(Filter::from_effect("ff84d00c-4494-11d8-a072000a9582f7d4", &p).is_none());
        let json = serde_json::to_string(&Filter::Invert).unwrap();
        assert_eq!(serde_json::from_str::<Filter>(&json).unwrap(), Filter::Invert);
    }

    #[test]
    fn mask_blends_over_original() {
        let img = RgbaImage::filled(IRect::new(0, 0, 4, 4), Rgba8::rgb(10, 20, 30)).unwrap();
        let mut m = Mask::new(IRect::new(0, 0, 4, 4)).unwrap();
        m.set(1, 1, 255);
        m.set(2, 1, 128);
        let out = Filter::Invert.apply(&img, Some(&m));
        assert_eq!(out.bounds(), img.bounds());
        assert_eq!(out.pixel(1, 1), Rgba8::rgb(245, 235, 225));
        assert_eq!(out.pixel(0, 0), Rgba8::rgb(10, 20, 30));
        let half = out.pixel(2, 1);
        assert!(half.r > 100 && half.r < 150);
    }
}
