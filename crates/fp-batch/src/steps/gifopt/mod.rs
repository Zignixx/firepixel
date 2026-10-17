//! GIF-Optimierer „kleine GIFs, sehr gute Qualität“ (SPEC-BATCH §3.10, §5.6), eigene Umsetzung
//! ohne GPL/AGPL-Bausteine:
//!
//! - [`frames`]: Vorverarbeitung (fast gleiche Frames zusammenlegen, zeitliche Glättung, Bildrate),
//!   Bilddifferenz → Transparenz, Zuschnitt auf das Rechteck der Änderungen
//! - [`palette`]: gewichtete Farbstatistik, Median-Cut + k-Means in Oklab; global / je Frame / auto
//! - [`dither`]: none / ordered / blueNoise / floydSteinberg / temporal (feste Blue-Noise-Matrix)
//! - [`lzw_lossy`]: LZW mit verlustbehafteter Verlängerung der Wörterbuch-Einträge
//! - [`encoder`]: Planung je Frame, Wahl der Entsorgung nach kleinerer Größe, GIF89a-Datei
//! - [`size`]: Zielgröße in KB (Qualität → Bildrate → Größe, höchstens 8 Kodierungen)
//! - [`metrics`]: SSIM (Oklab-L) und PSNR für Messung und Tests
//!
//! Eingehängt als Kodierer für „Speichern als: GIF“ ([`crate::output::register_encoder`]); Optionen
//! kommen aus `recipe.output.options` (Schema: Endkarte `output.gif`). Dazu der Schritt
//! `anim.stabilize` („Ruhiger Hintergrund“, zeitliche Glättung als eigener Schritt).

pub mod color;
pub mod dither;
pub mod encoder;
pub mod frames;
pub mod lzw_lossy;
pub mod metrics;
pub mod palette;
pub mod size;

use crate::error::{BatchError, Result};
use crate::item::{Item, ItemKind};
use crate::output::OutputFormat;
use crate::step::{ParamDef, ParamType, Step, StepContext, StepGroup, StepKind, StepRegistry};
use dither::DitherMode;
pub use encoder::{FrameStat, GifSettings, GifStats, PaletteMode};
use fp_raster::{IRect, RgbaImage};
use serde_json::{Map, Value};
use std::sync::Arc;

/// ID der Endkarte mit den GIF-Optionen (nur Schema; die Werte stehen in `recipe.output.options`).
pub const OUTPUT_GIF: &str = "output.gif";
/// ID des Schritts „Ruhiger Hintergrund“.
pub const STABILIZE: &str = "anim.stabilize";

/// Registriert Kodierer und Schritte.
pub fn register(r: &mut StepRegistry) {
    crate::output::register_encoder(OutputFormat::Gif, encode_item);
    r.register(Arc::new(GifOutput));
    r.register(Arc::new(Stabilize));
}

/// Quelle eines Kodiervorgangs: Leinwand, Frames (deckungsgleich mit der Leinwand), Dauer, Schleife.
#[derive(Clone)]
pub struct Source {
    pub width: usize,
    pub height: usize,
    pub frames: Vec<(RgbaImage, u32)>,
    pub loop_count: i32,
}

impl Source {
    /// Aus einem Element (Dokumente werden flach gerechnet).
    pub fn from_item(item: &Item) -> Result<Source> {
        match item {
            Item::Still(s) => Ok(Source {
                width: s.image.width(),
                height: s.image.height(),
                frames: vec![(s.image.clone(), 0)],
                loop_count: 0,
            }),
            Item::Animation(a) => {
                if a.frames.is_empty() {
                    return Err(BatchError::Decode(fp_i18n::t("batch-err-no-frames")));
                }
                let (w, h) = (a.width.max(1) as usize, a.height.max(1) as usize);
                Ok(Source {
                    width: w,
                    height: h,
                    frames: a
                        .frames
                        .iter()
                        .map(|f| (f.image.clone(), f.delay_ms))
                        .collect(),
                    loop_count: a.loop_count,
                })
            }
            Item::Document(_) => {
                let mut c = item.clone();
                c.flatten_document()?;
                Source::from_item(&c)
            }
            Item::FileOnly(p) => Err(BatchError::Unsupported(fp_i18n::tr!(
                "batch-err-not-image",
                path = p.display().to_string()
            ))),
        }
    }

    /// RGBA-Pixel der Frames auf der (ggf. skalierten) Leinwand.
    fn canvas_frames(&self, scale: f32) -> (usize, usize, CanvasFrames) {
        let scale = scale.clamp(0.01, 1.0);
        let (w, h) = if scale < 1.0 {
            (
                ((self.width as f32 * scale).round() as usize).max(1),
                ((self.height as f32 * scale).round() as usize).max(1),
            )
        } else {
            (self.width.max(1), self.height.max(1))
        };
        let frames = self
            .frames
            .iter()
            .map(|(img, d)| {
                let mut img = img.clone();
                img.set_origin(0, 0);
                if img.width() != self.width || img.height() != self.height {
                    img = fit_canvas(&img, self.width, self.height);
                }
                if (w, h) != (self.width, self.height) {
                    img = fp_raster::resample::resize(
                        &img,
                        w as u32,
                        h as u32,
                        fp_raster::Sampler::Bicubic,
                    );
                    img.set_origin(0, 0);
                }
                let mut px = img.data().to_vec();
                px.resize(w * h, [0; 4]);
                (px, *d)
            })
            .collect();
        (w, h, frames)
    }
}

/// RGBA-Pixel je Frame mit Dauer in ms.
type CanvasFrames = Vec<(Vec<[u8; 4]>, u32)>;

/// Bild auf die Leinwand legen (oben links, Überstand abgeschnitten, Rest transparent).
fn fit_canvas(img: &RgbaImage, w: usize, h: usize) -> RgbaImage {
    let mut out =
        RgbaImage::new(IRect::new(0, 0, w as i32, h as i32)).unwrap_or_else(|_| img.clone());
    let iw = img.width();
    let data = img.data();
    let od = out.data_mut();
    for y in 0..h.min(img.height()) {
        for x in 0..w.min(iw) {
            od[y * w + x] = data[y * iw + x];
        }
    }
    out
}

/// Ergebnis eines Kodiervorgangs.
#[derive(Clone, Debug)]
pub struct Optimized {
    pub bytes: Vec<u8>,
    pub stats: GifStats,
    /// Verwendete Qualität.
    pub quality: u8,
    pub scale: f32,
    pub frame_step: usize,
    /// Zielgröße erreicht (ohne Zielgröße immer `true`).
    pub reached: bool,
    /// Zahl der Kodierungen.
    pub encodes: usize,
}

/// Kodiert eine Quelle mit festen Einstellungen.
pub fn encode_source(src: &Source, s: &GifSettings, quality: u8) -> Optimized {
    let (w, h, frames) = src.canvas_frames(s.scale);
    let prep = frames::PrepSettings {
        transparency: s.transparency,
        matte: s.matte,
        frame_step: s.frame_step,
        merge_similar: s.merge_similar,
        threshold: s.diff_threshold,
    };
    let pf = frames::prepare(w, h, frames, &prep);
    let mut s2 = s.clone();
    s2.loop_count = src.loop_count;
    let (bytes, stats) = encoder::encode(w, h, &pf, &s2);
    Optimized {
        bytes,
        stats,
        quality,
        scale: s.scale,
        frame_step: s.frame_step,
        reached: true,
        encodes: 1,
    }
}

/// Optionen aus dem Rezept (`recipe.output.options`); nicht gesetzte Werte folgen `preset`/`quality`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GifOptions {
    pub preset: Option<String>,
    pub quality: Option<u8>,
    pub max_kb: Option<u32>,
    pub colors: Option<usize>,
    pub palette: Option<PaletteMode>,
    pub dither: Option<DitherMode>,
    pub dither_amount: Option<u32>,
    pub lossy: Option<u32>,
    pub diff_transparency: Option<bool>,
    pub diff_threshold: Option<u32>,
    pub crop_changed: Option<bool>,
    pub merge_similar: Option<bool>,
    pub disposal_auto: Option<bool>,
    pub transparency: Option<bool>,
    pub matte: Option<[u8; 3]>,
    pub split_frames: Option<bool>,
}

fn num(o: &Map<String, Value>, k: &str) -> Option<f64> {
    o.get(k).and_then(|v| {
        v.as_f64()
            .or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))
    })
}

impl GifOptions {
    /// Liest die Optionen (unbekannte Werte werden ignoriert, Zahlen begrenzt).
    pub fn from_map(o: &Map<String, Value>) -> GifOptions {
        let b = |k: &str| o.get(k).and_then(Value::as_bool);
        let s = |k: &str| o.get(k).and_then(Value::as_str);
        GifOptions {
            preset: s("preset")
                .filter(|p| matches!(*p, "smallest" | "balanced" | "best"))
                .map(str::to_string),
            quality: num(o, "quality").map(|q| q.round().clamp(1.0, 100.0) as u8),
            max_kb: num(o, "maxKB")
                .map(|v| v.round().clamp(0.0, 100_000.0) as u32)
                .filter(|v| *v > 0),
            colors: num(o, "colors").map(|v| v.round().clamp(2.0, 256.0) as usize),
            palette: s("palette").and_then(|p| match p {
                "global" => Some(PaletteMode::Global),
                "perFrame" => Some(PaletteMode::PerFrame),
                "auto" => Some(PaletteMode::Auto),
                _ => None,
            }),
            dither: s("dither").and_then(DitherMode::parse),
            dither_amount: num(o, "ditherAmount").map(|v| v.round().clamp(0.0, 100.0) as u32),
            lossy: num(o, "lossy").map(|v| v.round().clamp(0.0, 200.0) as u32),
            diff_transparency: b("diffTransparency"),
            diff_threshold: num(o, "diffThreshold").map(|v| v.round().clamp(0.0, 32.0) as u32),
            crop_changed: b("cropChanged"),
            merge_similar: b("mergeSimilar"),
            disposal_auto: s("disposal").map(|d| d != "keep"),
            transparency: b("transparency"),
            matte: s("background")
                .and_then(crate::step::parse_color)
                .map(|c| [c[0], c[1], c[2]]),
            split_frames: b("splitFrames"),
        }
    }

    /// Qualität aus `quality` bzw. `preset` (smallest 60, balanced 80, best 100).
    pub fn base_quality(&self) -> u8 {
        self.quality.unwrap_or(match self.preset.as_deref() {
            Some("smallest") => 60,
            Some("best") => 100,
            _ => 80,
        })
    }

    /// Einstellungen für eine Qualität. Liegt `q` unter der Ausgangsqualität (Zielgröße), werden auch
    /// ausdrücklich gesetzte Werte entsprechend verschärft (mehr Verlust, weniger Farben/Dithering).
    ///
    /// Ohne ausdrückliche Werte hängen Palettengröße und Verlust vom Inhalt ab (BATCH-8b, siehe
    /// [`quality_colors`], [`quality_lossy`]): Grafiken (wenige Farben, Flächen, Verläufe) kommen mit
    /// wenigen Farben aus und gewinnen kaum durch Verlust; Fotos/Videos brauchen mehr Farben (sonst
    /// Farbflecken) und werden über verlustbehaftetes LZW im feinen Korn klein.
    pub fn resolve(&self, q: u8) -> GifSettings {
        let q = q.clamp(1, 100);
        let base = self.base_quality();
        let below = base.saturating_sub(q) as f32;
        let qf = q as f32;
        let auto_colors = [quality_colors(qf, false), quality_colors(qf, true)];
        let auto_lossy = [quality_lossy(qf, false), quality_lossy(qf, true)];
        let (colors, colors_auto) = match self.colors {
            // ausdrücklich: genau so viele, unter der Ausgangsqualität höchstens so viele wie automatisch
            Some(c) => (c, (below > 0.0).then_some(auto_colors)),
            None => (256, Some(auto_colors)),
        };
        let (lossy, lossy_auto) = match self.lossy {
            Some(l) => ((l as f32 + 2.0 * below).round() as u32, None),
            None => (auto_lossy[0], Some(auto_lossy)),
        };
        // unter Qualität 50 weniger, ab 10 kein Dithering (wenige Farben + Dithering wären größer statt
        // kleiner – die Zielgröße braucht eine mit der Qualität fallende Größe, BATCH-8b)
        let amount =
            self.dither_amount.unwrap_or(70) as f32 / 100.0 * ((qf - 10.0) / 40.0).clamp(0.0, 1.0);
        let thr =
            self.diff_threshold.unwrap_or(3) as f32 / 100.0 * ((130.0 - qf) / 70.0).clamp(0.2, 2.0);
        GifSettings {
            colors: colors.clamp(2, 256),
            colors_auto,
            palette: self.palette.unwrap_or(PaletteMode::Auto),
            dither: self.dither.unwrap_or(DitherMode::Temporal),
            dither_amount: amount.clamp(0.0, 1.0),
            lossy: lossy.min(200),
            lossy_auto,
            diff_transparency: self.diff_transparency.unwrap_or(true),
            diff_threshold: thr,
            crop_changed: self.crop_changed.unwrap_or(true),
            merge_similar: self.merge_similar.unwrap_or(true),
            disposal_auto: self.disposal_auto.unwrap_or(true),
            transparency: self.transparency.unwrap_or(true),
            matte: self.matte.unwrap_or([255, 255, 255]),
            loop_count: 0,
            scale: 1.0,
            frame_step: 1,
            split_frames: self.split_frames.unwrap_or(q < 95),
        }
    }
}

/// Log-lineare Interpolation über Stützstellen `(Qualität, Wert)`.
fn interp_log(q: f32, pts: &[(f32, f32)]) -> f32 {
    let q = q.clamp(pts[0].0, pts[pts.len() - 1].0);
    for w in pts.windows(2) {
        let ((q0, v0), (q1, v1)) = (w[0], w[1]);
        if q <= q1 {
            let t = if q1 > q0 { (q - q0) / (q1 - q0) } else { 0.0 };
            return (v0.ln() + (v1.ln() - v0.ln()) * t).exp();
        }
    }
    pts[pts.len() - 1].1
}

/// Palettengröße (mit Transparenz-Index) je Qualität für Grafik bzw. Foto (BATCH-8b). Gemessen an
/// den Testclips (Bericht `docs\berichte\batch8b.md`): „balanced“ (80) = 44 Farben für Grafik, 92 für
/// Foto; „best“ (100) = 256.
pub fn quality_colors(q: f32, photo: bool) -> usize {
    let pts: &[(f32, f32)] = if photo {
        &[
            (1.0, 8.0),
            (40.0, 32.0),
            (60.0, 60.0),
            (80.0, 92.0),
            (90.0, 160.0),
            (100.0, 256.0),
        ]
    } else {
        &[
            (1.0, 6.0),
            (40.0, 16.0),
            (60.0, 24.0),
            (80.0, 44.0),
            (90.0, 96.0),
            (100.0, 256.0),
        ]
    };
    (interp_log(q, pts).round() as usize).clamp(2, 256)
}

/// Verlust (0…200) je Qualität für Grafik bzw. Foto (BATCH-8b): Grafik (100 − q)·2, Foto (100 − q)·5.
pub fn quality_lossy(q: f32, photo: bool) -> u32 {
    (((100.0 - q.clamp(1.0, 100.0)) * if photo { 5.0 } else { 2.0 }).round() as u32).min(200)
}

/// Optimiert ein Element (mit Zielgröße, wenn `maxKB` gesetzt ist).
pub fn optimize(item: &Item, options: &Map<String, Value>) -> Result<Optimized> {
    let src = Source::from_item(item)?;
    let opts = GifOptions::from_map(options);
    Ok(match opts.max_kb {
        Some(kb) => size::encode_to_target(&src, &opts, kb as usize * 1024),
        None => {
            let q = opts.base_quality();
            encode_source(&src, &opts.resolve(q), q)
        }
    })
}

/// Kodierer für „Speichern als: GIF“.
pub fn encode_item(item: &Item, options: &Map<String, Value>) -> Result<Vec<u8>> {
    Ok(optimize(item, options)?.bytes)
}

/// Endkarte „GIF-Optionen“ (Schema der Optionen in `recipe.output.options` bei Format GIF).
struct GifOutput;

impl Step for GifOutput {
    fn id(&self) -> &str {
        OUTPUT_GIF
    }
    fn group(&self) -> StepGroup {
        StepGroup::Format
    }
    fn kind(&self) -> StepKind {
        StepKind::Final
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::choice("preset", &["smallest", "balanced", "best"], "balanced").beginner(),
            ParamDef::int("quality", 1, 100, 80).beginner(),
            ParamDef::int("maxKB", 0, 100_000, 0).beginner().unit("KB"),
            // ohne Wert: nach Qualität und Inhalt (Grafik/Foto) gewählt → „Automatisch“ (BATCH-11b)
            ParamDef::new("colors", ParamType::Int { min: 2, max: 256 }, Value::Null),
            ParamDef::choice("palette", &["global", "perFrame", "auto"], "auto"),
            ParamDef::choice(
                "dither",
                &["none", "ordered", "blueNoise", "floydSteinberg", "temporal"],
                "temporal",
            ),
            ParamDef::int("ditherAmount", 0, 100, 70).unit("%"),
            ParamDef::new("lossy", ParamType::Int { min: 0, max: 200 }, Value::Null),
            ParamDef::bool("diffTransparency", true),
            ParamDef::int("diffThreshold", 0, 32, 3),
            ParamDef::bool("cropChanged", true),
            ParamDef::bool("mergeSimilar", true),
            ParamDef::choice("disposal", &["auto", "keep"], "auto"),
            ParamDef::bool("transparency", true),
            ParamDef::new("splitFrames", ParamType::Bool, Value::Null),
            // Füllfarbe für halbtransparente Pixel ohne GIF-Transparenz (BATCH-11b: eigenes Feld)
            ParamDef::color("background", "#ffffff").when("transparency", &["false"]),
        ]
    }
    fn preview_ok(&self) -> bool {
        false
    }
    fn apply(&self, _item: &mut Item, _params: &Value, _ctx: &mut StepContext) -> Result<()> {
        Ok(())
    }
}

/// „Ruhiger Hintergrund“: zeitliche Glättung (SPEC-BATCH §3.9 `anim.stabilize`, §5.6 Punkt 1).
struct Stabilize;

impl Step for Stabilize {
    fn id(&self) -> &str {
        STABILIZE
    }
    fn group(&self) -> StepGroup {
        StepGroup::Animation
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![ParamDef::int("threshold", 0, 32, 3).beginner()]
    }
    fn applies_to(&self) -> &[ItemKind] {
        &[ItemKind::Animation]
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let Item::Animation(a) = item else {
            return Ok(());
        };
        let thr = params
            .get("threshold")
            .and_then(Value::as_f64)
            .unwrap_or(3.0)
            .clamp(0.0, 32.0) as f32
            / 100.0;
        if thr <= 0.0 || a.frames.len() < 2 {
            return Ok(());
        }
        ctx.check_cancel()?;
        let mut px: Vec<Vec<[u8; 4]>> = a.frames.iter().map(|f| f.image.data().to_vec()).collect();
        frames::stabilize(&mut px, thr);
        for (f, p) in a.frames.iter_mut().zip(px) {
            if f.image.data().len() == p.len() {
                f.image.data_mut().copy_from_slice(&p);
            }
        }
        Ok(())
    }
    fn summary(&self, params: &Value) -> String {
        let t = params.get("threshold").and_then(Value::as_i64).unwrap_or(3);
        fp_i18n::tr!("batch-summary-anim-stabilize", threshold = t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Voreinstellungen (BATCH-8b): Grafik braucht weniger Farben als Foto, Fotos mehr Verlust; beide
    /// steigen bzw. fallen mit der Qualität; „best“ = 256 Farben ohne Verlust.
    #[test]
    fn quality_mapping_is_monotonic_and_content_aware() {
        assert_eq!(
            (quality_colors(80.0, false), quality_colors(80.0, true)),
            (44, 92)
        );
        assert_eq!(
            (quality_colors(100.0, false), quality_colors(100.0, true)),
            (256, 256)
        );
        assert_eq!(
            (quality_lossy(100.0, false), quality_lossy(100.0, true)),
            (0, 0)
        );
        assert_eq!(
            (quality_lossy(80.0, false), quality_lossy(80.0, true)),
            (40, 100)
        );
        for photo in [false, true] {
            for q in 1..100 {
                assert!(quality_colors(q as f32, photo) <= quality_colors(q as f32 + 1.0, photo));
                assert!(quality_lossy(q as f32, photo) >= quality_lossy(q as f32 + 1.0, photo));
                assert!(quality_colors(q as f32, false) <= quality_colors(q as f32, true));
            }
        }
        // ausdrückliche Farben gelten genau (ab der Ausgangsqualität), automatisch sonst
        let o = GifOptions::from_map(
            &serde_json::json!({"colors": 128})
                .as_object()
                .cloned()
                .unwrap_or_default(),
        );
        let s = o.resolve(80);
        assert_eq!(
            (s.colors, s.colors_auto, s.for_content(1.0).0),
            (128, None, 128)
        );
        let s = o.resolve(40);
        assert_eq!(s.for_content(0.0).0, quality_colors(40.0, false));
        let auto = GifOptions::default().resolve(80);
        assert_eq!(
            (auto.for_content(0.0), auto.for_content(1.0)),
            ((44, 40), (92, 100))
        );
        // Teilbilder nur unter „best“
        assert!(auto.split_frames && !GifOptions::default().resolve(100).split_frames);
    }
}
