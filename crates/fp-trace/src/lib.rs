//! # fp-trace – Bitmap nachzeichnen
//!
//! Wandelt ein Rasterbild ([`fp_raster::RgbaImage`]) in gefüllte, bearbeitbare Bézier-Pfade um
//! (Menü „Modifizieren ▸ Bitmap nachzeichnen…“). Grundlage ist die Clusterbildung und
//! Kurvenanpassung von `visioncortex` (MIT OR Apache-2.0); der Ablauf folgt dem Muster von vtracer
//! (ebenfalls MIT OR Apache-2.0), ist hier aber eigenständig umgesetzt.
//!
//! ## Ablauf
//! 1. **Vorbereitung:** zu große Bilder (`max_pixels`) bilinear herunterskalieren (oder Fehler,
//!    wenn `downscale = false`); transparente Pixel (`alpha < alpha_threshold`) bei
//!    `ignore_transparent` ausblenden (Schlüsselfarbe, ergibt keine Pfade), sonst auf Weiß
//!    verrechnen; optional Farbreduktion auf `colors` Farben (Median-Cut).
//! 2. **Farbe:** hierarchische Farbcluster (`color_precision` signifikante Bits je Kanal,
//!    `layer_difference` Farbabstand zwischen Stapelebenen, Flecken unter
//!    `filter_speckle`² Pixeln werden mit dem Nachbarn verschmolzen). „Stapeln“ legt größere
//!    Flächen ohne Löcher unter kleinere, „Ausschneiden“ erzeugt überlappungsfreie Flächen mit
//!    Löchern (zweiter Durchlauf). **Schwarzweiß:** Helligkeit < `threshold` = schwarz,
//!    Zusammenhangskomponenten → schwarze Pfade.
//! 3. **Kontur** je Fläche als Polygon (Eckpunkte) oder Spline (kubische Bézierkurven;
//!    `corner_threshold`, `length_threshold`, `max_iterations`, `splice_threshold`).
//! 4. **Koordinaten** in Dokumentkoordinaten (Bitmap-Ursprung, Rückskalierung), gerundet auf
//!    `path_precision` Nachkommastellen.
//!
//! Ergebnis: [`TraceResult::paths`] in **Zeichenreihenfolge von unten nach oben**, je Pfad
//! geschlossene [`BezierContour`]s (Füllregel gerade/ungerade, Löcher als eigene Konturen) und
//! eine deckende Füllfarbe. [`TraceResult::render`] rastert das Ergebnis (Vorschau, Tests).
//!
//! **Logo-Nachbearbeitung** ([`TraceOptions::refine`], Vorgabe „Logo“: an): eigener Ablauf für
//! flächige Motive – Farben auf saubere Originalfarben einrasten, gemeinsame Kanten benachbarter
//! Flächen (keine Haarrisse), exakte Grundformen (Kreis, Ellipse, Rechteck, abgerundetes
//! Rechteck), eingerastete Geraden (0°/45°/90°, rechte Winkel), wenige Bézier-Knoten, optional
//! Spiegelsymmetrie. Kennzahlen liefert [`trace_with_report`] ([`RefineReport`]).
//!
//! Abbruch und Fortschritt (für die Live-Vorschau): [`trace_with`] nimmt ein `&AtomicBool` und
//! einen Rückruf mit [`Progress`]. Kein `unsafe`; Panics der Bibliothek werden abgefangen und als
//! [`TraceError::Internal`] gemeldet.

mod quantize;
mod refine;
mod render;

pub use refine::{RefineOptions, RefineReport, Symmetry, FAST_PREVIEW_PIXELS};

use fp_color::Rgba8;
use fp_geom::{BezierContour, BezierNode, IRect, Point};
use fp_raster::RgbaImage;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::sync::atomic::{AtomicBool, Ordering};
use visioncortex::color_clusters::{KeyingAction, Runner, RunnerConfig, HIERARCHICAL_MAX};
use visioncortex::{BinaryImage, Color, ColorImage, CompoundPath, CompoundPathElement, PathSimplifyMode};

/// Absolute Obergrenze der nachgezeichneten Pixel (Speicher, 32-Bit-Indizes der Clusterbildung).
pub const HARD_PIXEL_LIMIT: u64 = 64_000_000;

/// Vorgabe für [`TraceOptions::max_pixels`]: 4 Megapixel.
pub const DEFAULT_MAX_PIXELS: u64 = 4_000_000;

// ------------------------------------------------------------------ Optionen

/// Vorgaben (Dialog „Vorgabe“, Befehlsparameter `preset`).
///
/// | Vorgabe | Farbe | Farben | Präzision | Ebenenabstand | Flecken | Kurven | Ecke | Stapel |
/// |---|---|---|---|---|---|---|---|---|
/// | `Logo` | Farbe | 16 | 6 Bit | 16 | 4 px | Spline | 60° | stapeln |
/// | `Schwarzweiss` | Schwarzweiß (Schwelle 128) | – | – | – | 4 px | Spline | 60° | – |
/// | `Illustration` | Farbe | 32 | 7 Bit | 16 | 4 px | Spline | 60° | stapeln |
/// | `Foto` | Farbe | 64 | 6 Bit | 16 | 12 px | Spline (Segment 8) | 180° | stapeln |
///
/// Logo-Nachbearbeitung ([`RefineOptions`]) nur bei `Logo` an (Schalter wie [`RefineOptions::logo`]).
///
/// Foto (VT-4): feinere Farbstufen (Ebenenabstand 16 statt 48) gegen den Plakat-Eindruck, 6 Bit
/// Farbgenauigkeit und größere Flecken/Segmente gegen unruhige Konturen bei Bildrauschen.
///
/// Gemeinsam: Segmentlänge 4, 10 Glättungsdurchläufe, Spleißwinkel 45°, 2 Nachkommastellen,
/// Transparenz ignorieren (Alpha < 128), höchstens 4 Megapixel (größere herunterskalieren).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Preset {
    /// Wenige flache Farben, saubere Kanten (Logos, Symbole, Schrift in Farbe).
    #[default]
    #[serde(rename = "logo")]
    Logo,
    /// Schwarze Flächen auf hellem Grund (Strichzeichnungen, Scans, Schrift).
    #[serde(rename = "blackWhite", alias = "schwarzweiss", alias = "bw")]
    Schwarzweiss,
    /// Viele Farben, Stapelebenen (Grafiken, Poster).
    #[serde(rename = "illustration")]
    Illustration,
    /// Fotos: feine Farbstufen (Verläufe als viele Farbflächen), gröbere Flecken, weiche Kurven.
    #[serde(rename = "photo", alias = "foto")]
    Foto,
}

impl Preset {
    /// Alle Vorgaben in Dialogreihenfolge.
    pub const ALL: [Preset; 4] = [Preset::Logo, Preset::Schwarzweiss, Preset::Illustration, Preset::Foto];

    /// Name im JSON (Befehlsparameter).
    pub fn name(self) -> &'static str {
        match self {
            Preset::Logo => "logo",
            Preset::Schwarzweiss => "blackWhite",
            Preset::Illustration => "illustration",
            Preset::Foto => "photo",
        }
    }

    /// Vorgabe zu einem JSON-Namen (auch die deutschen Aliasnamen).
    pub fn from_name(s: &str) -> Option<Preset> {
        match s {
            "logo" => Some(Preset::Logo),
            "blackWhite" | "schwarzweiss" | "bw" => Some(Preset::Schwarzweiss),
            "illustration" => Some(Preset::Illustration),
            "photo" | "foto" => Some(Preset::Foto),
            _ => None,
        }
    }
}

/// Farbmodus.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ColorMode {
    /// Farbige Flächen.
    #[default]
    Color,
    /// Nur schwarze Flächen (Schwellwert).
    BlackWhite,
}

/// Art der Konturen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CurveMode {
    /// Gerade Segmente (Eckpunkte ohne Henkel).
    Polygon,
    /// Kubische Bézierkurven (Knoten mit Henkeln).
    #[default]
    Spline,
}

/// Anordnung der Farbflächen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Hierarchy {
    /// Stapeln: größere Flächen ohne Löcher liegen unter den kleineren.
    #[default]
    Stacked,
    /// Ausschneiden: Flächen überlappen nicht, Löcher bleiben offen.
    Cutout,
}

/// Einstellungen des Nachzeichnens. Ausgangspunkt ist immer eine Vorgabe
/// ([`TraceOptions::from_preset`]); [`TraceOptions::validated`] begrenzt alle Werte.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TraceOptions {
    /// Vorgabe, aus der die übrigen Werte stammen (nur zur Anzeige; die Felder gelten).
    pub preset: Preset,
    /// Farbe oder Schwarzweiß.
    pub color_mode: ColorMode,
    /// Höchstzahl der Farben (Median-Cut vor der Clusterbildung), 0 = keine Reduktion, sonst 2–256.
    pub colors: u32,
    /// Signifikante Bits je Farbkanal beim Vergleich (1–8; kleiner = weniger Farbflächen).
    pub color_precision: u8,
    /// Farbabstand zwischen Stapelebenen (0–255; 0 verbindet auch diagonale Nachbarn).
    pub layer_difference: u8,
    /// Flecken kleiner als n × n Pixel entfernen (0–128).
    pub filter_speckle: u32,
    /// Knoten mit einem Richtungswechsel über diesem Winkel bleiben Ecken (Grad, 0–180).
    pub corner_threshold: f64,
    /// Mindestlänge der Segmente vor der Kurvenanpassung (3,5–10 Pixel).
    pub length_threshold: f64,
    /// Höchstzahl der Glättungsdurchläufe (1–50).
    pub max_iterations: usize,
    /// Splines werden an Richtungswechseln über diesem Winkel geteilt (Grad, 0–180).
    pub splice_threshold: f64,
    /// Nachkommastellen der Koordinaten (0–8).
    pub path_precision: u32,
    /// Polygon oder Spline.
    pub mode: CurveMode,
    /// Stapeln oder Ausschneiden (nur Farbe).
    pub hierarchical: Hierarchy,
    /// Transparente Pixel ergeben keine Flächen (sonst auf Weiß verrechnet).
    pub ignore_transparent: bool,
    /// Pixel mit kleinerem Alpha gelten als transparent (1–255).
    pub alpha_threshold: u8,
    /// Schwarzweiß: Helligkeit unter diesem Wert = schwarz (0–255).
    pub threshold: u8,
    /// Höchstzahl der Pixel (0 = nur [`HARD_PIXEL_LIMIT`]).
    pub max_pixels: u64,
    /// Größere Bilder herunterskalieren (sonst [`TraceError::TooLarge`]).
    pub downscale: bool,
    /// Logo-Nachbearbeitung (fehlt der Eintrag im JSON: aus).
    #[serde(default)]
    pub refine: RefineOptions,
}

impl Default for TraceOptions {
    fn default() -> Self {
        TraceOptions::from_preset(Preset::Logo)
    }
}

impl TraceOptions {
    /// Werte einer Vorgabe (Tabelle bei [`Preset`]).
    pub fn from_preset(p: Preset) -> TraceOptions {
        let base = TraceOptions {
            preset: p,
            color_mode: ColorMode::Color,
            colors: 0,
            color_precision: 7,
            layer_difference: 16,
            filter_speckle: 4,
            corner_threshold: 60.0,
            length_threshold: 4.0,
            max_iterations: 10,
            splice_threshold: 45.0,
            path_precision: 2,
            mode: CurveMode::Spline,
            hierarchical: Hierarchy::Stacked,
            ignore_transparent: true,
            alpha_threshold: 128,
            threshold: 128,
            max_pixels: DEFAULT_MAX_PIXELS,
            downscale: true,
            refine: RefineOptions::default(),
        };
        match p {
            Preset::Logo => TraceOptions { colors: 16, color_precision: 6, refine: RefineOptions::logo(), ..base },
            Preset::Schwarzweiss => TraceOptions { color_mode: ColorMode::BlackWhite, ..base },
            Preset::Illustration => TraceOptions { colors: 32, ..base },
            Preset::Foto => TraceOptions { colors: 64, color_precision: 6, layer_difference: 16, filter_speckle: 12, corner_threshold: 180.0, length_threshold: 8.0, ..base },
        }
    }

    /// Alle Werte in ihren Bereichen (NaN → Vorgabewert der Vorgabe).
    pub fn validated(&self) -> TraceOptions {
        let d = TraceOptions::from_preset(self.preset);
        let f = |v: f64, lo: f64, hi: f64, def: f64| if v.is_finite() { v.clamp(lo, hi) } else { def };
        TraceOptions {
            colors: if self.colors < 2 { 0 } else { self.colors.min(256) },
            color_precision: self.color_precision.clamp(1, 8),
            filter_speckle: self.filter_speckle.min(128),
            corner_threshold: f(self.corner_threshold, 0.0, 180.0, d.corner_threshold),
            length_threshold: f(self.length_threshold, 3.5, 10.0, d.length_threshold),
            max_iterations: self.max_iterations.clamp(1, 50),
            splice_threshold: f(self.splice_threshold, 0.0, 180.0, d.splice_threshold),
            path_precision: self.path_precision.min(8),
            alpha_threshold: self.alpha_threshold.max(1),
            refine: self.refine.validated(),
            ..self.clone()
        }
    }

    fn simplify_mode(&self) -> PathSimplifyMode {
        match self.mode {
            CurveMode::Polygon => PathSimplifyMode::Polygon,
            CurveMode::Spline => PathSimplifyMode::Spline,
        }
    }
}

// ------------------------------------------------------------------ Ergebnis

/// Ein nachgezeichneter, gefüllter Pfad.
#[derive(Clone, Debug, PartialEq)]
pub struct TracedPath {
    /// Geschlossene Konturen in Dokumentkoordinaten; Löcher sind eigene Konturen
    /// (Füllregel gerade/ungerade).
    pub contours: Vec<BezierContour>,
    /// Deckende Füllfarbe (Mittelwert der Fläche; Schwarzweiß: Schwarz).
    pub color: Rgba8,
    /// Fläche in Pixeln des nachgezeichneten (ggf. verkleinerten) Bildes.
    pub pixels: usize,
}

impl TracedPath {
    /// Anzahl der Knoten über alle Konturen.
    pub fn node_count(&self) -> usize {
        self.contours.iter().map(|c| c.nodes.len()).sum()
    }
}

/// Ergebnis des Nachzeichnens.
#[derive(Clone, Debug, PartialEq)]
pub struct TraceResult {
    /// Pfade in Zeichenreihenfolge **von unten nach oben** (der erste liegt zuunterst).
    pub paths: Vec<TracedPath>,
    /// Grenzen des Quellbildes in Dokumentkoordinaten.
    pub bounds: IRect,
    /// Größe des tatsächlich nachgezeichneten Bildes (kleiner als `bounds` nach dem Herunterskalieren).
    pub traced_width: u32,
    pub traced_height: u32,
}

impl TraceResult {
    /// Wurde das Bild verkleinert?
    pub fn downscaled(&self) -> bool {
        self.traced_width != self.bounds.width() || self.traced_height != self.bounds.height()
    }

    /// Anzahl der Knoten über alle Pfade.
    pub fn node_count(&self) -> usize {
        self.paths.iter().map(TracedPath::node_count).sum()
    }

    /// Anzahl der Konturen über alle Pfade.
    pub fn contour_count(&self) -> usize {
        self.paths.iter().map(|p| p.contours.len()).sum()
    }

    /// Pfade von oben nach unten (Reihenfolge der Kinder einer Gruppe im Dokumentmodell).
    pub fn top_to_bottom(&self) -> impl Iterator<Item = &TracedPath> {
        self.paths.iter().rev()
    }

    /// Rastert das Ergebnis ohne Kantenglättung (Pixelmitte, gerade/ungerade) auf transparentem
    /// Grund in den Grenzen des Quellbildes – für Vorschau und Prüfungen.
    pub fn render(&self) -> RgbaImage {
        render::render(self)
    }
}

/// Fortschrittsabschnitt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// Skalieren, Transparenz, Farbreduktion.
    Prepare,
    /// Farbcluster bzw. Schwellwert.
    Cluster,
    /// Konturen und Kurvenanpassung.
    Paths,
    /// Fertig.
    Done,
}

/// Fortschritt für die Live-Vorschau.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Progress {
    pub stage: Stage,
    /// Gesamtfortschritt 0–1.
    pub fraction: f32,
}

/// Fehler beim Nachzeichnen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TraceError {
    /// Das Bild hat keine Pixel.
    Empty,
    /// Das Bild ist zu groß und Herunterskalieren ist aus.
    TooLarge { width: u32, height: u32 },
    /// Über das Abbruch-Flag abgebrochen.
    Cancelled,
    /// Interner Fehler der Vektorisierung.
    Internal(String),
}

impl std::fmt::Display for TraceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TraceError::Empty => f.write_str(&fp_i18n::t("trace-err-empty")),
            TraceError::TooLarge { width, height } => f.write_str(&fp_i18n::tr!("trace-err-too-large", width = *width as f64, height = *height as f64)),
            TraceError::Cancelled => f.write_str(&fp_i18n::t("trace-err-cancelled")),
            TraceError::Internal(m) => f.write_str(&fp_i18n::tr!("trace-err-internal", detail = m.as_str())),
        }
    }
}

impl std::error::Error for TraceError {}

// ------------------------------------------------------------------ Einstieg

/// Zeichnet ein Bild nach (ohne Abbruch und Fortschritt).
pub fn trace(image: &RgbaImage, opts: &TraceOptions) -> Result<TraceResult, TraceError> {
    trace_with(image, opts, &AtomicBool::new(false), &mut |_| {})
}

/// Zeichnet ein Bild nach. `cancel` wird regelmäßig geprüft (→ [`TraceError::Cancelled`]),
/// `progress` erhält den Fortschritt (monoton steigend, zuletzt [`Stage::Done`] mit 1,0).
pub fn trace_with(image: &RgbaImage, opts: &TraceOptions, cancel: &AtomicBool, progress: &mut dyn FnMut(Progress)) -> Result<TraceResult, TraceError> {
    trace_with_report(image, opts, cancel, progress).map(|(r, _)| r)
}

/// Wie [`trace_with`], zusätzlich die Kennzahlen der Logo-Nachbearbeitung (`None`, wenn
/// [`RefineOptions::enabled`] aus ist).
pub fn trace_with_report(image: &RgbaImage, opts: &TraceOptions, cancel: &AtomicBool, progress: &mut dyn FnMut(Progress)) -> Result<(TraceResult, Option<RefineReport>), TraceError> {
    let o = opts.validated();
    let bounds = image.bounds();
    let (w, h) = (image.width() as u64, image.height() as u64);
    if w == 0 || h == 0 {
        return Err(TraceError::Empty);
    }
    progress(Progress { stage: Stage::Prepare, fraction: 0.0 });
    let total = w * h;
    let mut limit = if o.max_pixels == 0 { HARD_PIXEL_LIMIT } else { o.max_pixels.min(HARD_PIXEL_LIMIT) };
    if o.refine.enabled && o.refine.fast {
        limit = limit.min(FAST_PREVIEW_PIXELS);
    }
    let work: Cow<RgbaImage> = if total > limit {
        if !o.downscale {
            return Err(TraceError::TooLarge { width: w as u32, height: h as u32 });
        }
        let f = (limit as f64 / total as f64).sqrt();
        let tw = ((w as f64 * f).floor() as u32).max(1);
        let th = ((h as f64 * f).floor() as u32).max(1);
        Cow::Owned(fp_raster::resample::resize(image, tw, th, fp_raster::Sampler::Bilinear))
    } else {
        Cow::Borrowed(image)
    };
    let (tw, th) = (work.width(), work.height());
    if tw == 0 || th == 0 {
        return Err(TraceError::Empty);
    }
    let map = Mapper {
        left: bounds.left as f64,
        top: bounds.top as f64,
        sx: w as f64 / tw as f64,
        sy: h as f64 / th as f64,
        factor: 10f64.powi(o.path_precision as i32),
        w: tw as f64,
        h: th as f64,
    };
    check(cancel)?;
    let run = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if o.refine.enabled {
            return refine::trace(&work, &o, &map, cancel, progress).map(|(p, r)| (p, Some(r)));
        }
        match o.color_mode {
            ColorMode::Color => trace_color(&work, &o, &map, cancel, progress),
            ColorMode::BlackWhite => trace_binary(&work, &o, &map, cancel, progress),
        }
        .map(|p| (p, None))
    }));
    let (paths, report) = match run {
        Ok(r) => r?,
        Err(e) => {
            let msg = e.downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| e.downcast_ref::<String>().cloned()).unwrap_or_default();
            return Err(TraceError::Internal(msg));
        }
    };
    progress(Progress { stage: Stage::Done, fraction: 1.0 });
    Ok((TraceResult { paths, bounds, traced_width: tw as u32, traced_height: th as u32 }, report))
}

fn check(cancel: &AtomicBool) -> Result<(), TraceError> {
    if cancel.load(Ordering::Relaxed) {
        Err(TraceError::Cancelled)
    } else {
        Ok(())
    }
}

/// Bildpixel → Dokumentkoordinaten.
struct Mapper {
    left: f64,
    top: f64,
    sx: f64,
    sy: f64,
    factor: f64,
    /// Größe des nachgezeichneten Bildes: Punkte werden auf [0, w] × [0, h] begrenzt, damit
    /// geglättete Kurven (z. B. Vorgabe Foto an den Bildecken) nicht über die Bitmap hinausragen.
    w: f64,
    h: f64,
}

impl Mapper {
    fn map(&self, x: f64, y: f64) -> Point {
        let r = |v: f64| (v * self.factor).round() / self.factor;
        let (x, y) = (x.clamp(0.0, self.w), y.clamp(0.0, self.h));
        Point::new(r(self.left + x * self.sx) as f32, r(self.top + y * self.sy) as f32)
    }
}

/// Helligkeit (ITU-R 601, ganzzahlig).
fn luma(r: u8, g: u8, b: u8) -> u32 {
    (299 * r as u32 + 587 * g as u32 + 114 * b as u32) / 1000
}

/// Pixel auf Weiß verrechnet.
fn over_white(p: &mut [u8]) {
    let a = p[3] as u32;
    for c in &mut p[..3] {
        *c = ((*c as u32 * a + 255 * (255 - a) + 127) / 255) as u8;
    }
    p[3] = 255;
}

// ------------------------------------------------------------------ Farbe

fn trace_color(img: &RgbaImage, o: &TraceOptions, map: &Mapper, cancel: &AtomicBool, progress: &mut dyn FnMut(Progress)) -> Result<Vec<TracedPath>, TraceError> {
    let (w, h) = (img.width(), img.height());
    let mut px = img.as_bytes().to_vec();
    // Transparenz: Alpha 0 markiert „ausblenden“, alle übrigen Pixel werden deckend.
    let mut transparent = vec![false; w * h];
    let mut any_transparent = false;
    for (i, p) in px.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        if o.ignore_transparent && p[3] < o.alpha_threshold {
            transparent[i] = true;
            any_transparent = true;
        } else if o.ignore_transparent {
            p[3] = 255;
        } else {
            over_white(p);
        }
    }
    if any_transparent && transparent.iter().all(|&t| t) {
        return Ok(Vec::new());
    }
    if o.colors >= 2 {
        quantize::reduce(&mut px, &transparent, o.colors as usize);
    }
    check(cancel)?;
    let shift = 8 - o.color_precision as i32;
    let key = if any_transparent { Some(quantize::unused_color(&px, &transparent, shift)) } else { None };
    if let Some(k) = key {
        for (p, &t) in px.as_chunks_mut::<4>().0.iter_mut().zip(&transparent) {
            if t {
                p.copy_from_slice(&[k.r, k.g, k.b, 255]);
            }
        }
    }
    progress(Progress { stage: Stage::Cluster, fraction: 0.05 });
    let key_color = key.unwrap_or_default();
    let cutout = o.hierarchical == Hierarchy::Cutout;
    let config = RunnerConfig {
        diagonal: o.layer_difference == 0,
        hierarchical: HIERARCHICAL_MAX,
        batch_size: 25600,
        good_min_area: (o.filter_speckle * o.filter_speckle) as usize,
        good_max_area: w * h,
        is_same_color_a: shift,
        is_same_color_b: 1,
        deepen_diff: o.layer_difference as i32,
        hollow_neighbours: 1,
        key_color,
        keying_action: KeyingAction::Discard,
    };
    let span = if cutout { (0.05, 0.40) } else { (0.05, 0.60) };
    let mut clusters = run_clusters(config, ColorImage { pixels: px, width: w, height: h }, span, cancel, progress)?;
    if cutout {
        // Zweiter Durchlauf auf dem gestapelten Ergebnisbild: exakte Farbflächen ohne Überlappung.
        let mut flat = clusters.view().to_color_image();
        let fallback = key.unwrap_or_else(|| quantize::unused_color(&flat.pixels, &vec![false; w * h], 0));
        let mut keyed = key.is_some();
        for (i, p) in flat.pixels.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            if transparent[i] || p[3] == 0 {
                p.copy_from_slice(&[fallback.r, fallback.g, fallback.b, 255]);
                keyed = true;
            }
        }
        let config2 = RunnerConfig {
            diagonal: false,
            hierarchical: 64,
            batch_size: 25600,
            good_min_area: 0,
            good_max_area: w * h,
            is_same_color_a: 0,
            is_same_color_b: 1,
            deepen_diff: 0,
            hollow_neighbours: 0,
            key_color: if keyed { fallback } else { Color::default() },
            keying_action: KeyingAction::Discard,
        };
        clusters = run_clusters(config2, flat, (0.40, 0.60), cancel, progress)?;
    }
    let view = clusters.view();
    let order: Vec<_> = view.clusters_output.iter().rev().copied().collect();
    let n = order.len().max(1);
    let mode = o.simplify_mode();
    let mut out = Vec::new();
    for (k, idx) in order.into_iter().enumerate() {
        if k % 16 == 0 {
            check(cancel)?;
            progress(Progress { stage: Stage::Paths, fraction: 0.60 + 0.40 * k as f32 / n as f32 });
        }
        let cluster = view.get_cluster(idx);
        let c = cluster.residue_color();
        if key.is_some_and(|kc| kc.r == c.r && kc.g == c.g && kc.b == c.b) {
            continue;
        }
        let cp = cluster.to_compound_path(&view, false, mode, o.corner_threshold.to_radians(), o.length_threshold, o.max_iterations, o.splice_threshold.to_radians());
        let contours = convert(&cp, map);
        if contours.is_empty() {
            continue;
        }
        out.push(TracedPath { contours, color: Rgba8::new(c.r, c.g, c.b, 255), pixels: cluster.area() });
    }
    if !cutout && !any_transparent {
        keep_image_corners(&mut out, map);
    }
    Ok(out)
}

/// Bildkante (Stapeln, deckendes Bild): Die unterste Fläche reicht bis an die Bildkante, ihre Kurve
/// rundet die Bildecken aber ab (bei „Foto“ gibt es wegen Ecke 180° gar keine Ecken), sodass dort
/// Lücken entstehen. Die unterste Fläche wird deshalb zum exakten Bildrechteck (alle anderen
/// Flächen liegen darüber, das Bild ist vollständig gedeckt); reicht sie nicht an alle vier Kanten,
/// kommt ein Rechteck in ihrer Farbe darunter.
fn keep_image_corners(out: &mut Vec<TracedPath>, map: &Mapper) {
    let Some(bottom) = out.first() else { return };
    let rect = BezierContour::new(vec![BezierNode::corner(map.map(0.0, 0.0)), BezierNode::corner(map.map(map.w, 0.0)), BezierNode::corner(map.map(map.w, map.h)), BezierNode::corner(map.map(0.0, map.h))], true);
    let (lo, hi) = (map.map(0.0, 0.0), map.map(map.w, map.h));
    let (mut l, mut t, mut r, mut b) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for n in bottom.contours.iter().flat_map(|c| c.nodes.iter()) {
        l = l.min(n.pt.x);
        t = t.min(n.pt.y);
        r = r.max(n.pt.x);
        b = b.max(n.pt.y);
    }
    // „reicht an die Kante“: höchstens 2 Bildpixel Abstand
    let (tx, ty) = (2.0 * map.sx as f32, 2.0 * map.sy as f32);
    let spans = l <= lo.x + tx && t <= lo.y + ty && r >= hi.x - tx && b >= hi.y - ty;
    if spans {
        out[0].contours = vec![rect];
    } else {
        let color = bottom.color;
        out.insert(0, TracedPath { contours: vec![rect], color, pixels: 0 });
    }
}

/// Clusterbildung Schritt für Schritt mit Abbruch und Fortschritt im Bereich `span`.
fn run_clusters(config: RunnerConfig, image: ColorImage, span: (f32, f32), cancel: &AtomicBool, progress: &mut dyn FnMut(Progress)) -> Result<visioncortex::color_clusters::Clusters, TraceError> {
    let mut b = Runner::new(config, image).start();
    let mut ticks = 0u32;
    while !b.tick() {
        ticks += 1;
        if ticks.is_multiple_of(8) {
            check(cancel)?;
            let p = b.progress().min(100) as f32 / 100.0;
            progress(Progress { stage: Stage::Cluster, fraction: span.0 + (span.1 - span.0) * p });
        }
    }
    check(cancel)?;
    Ok(b.result())
}

// ------------------------------------------------------------------ Schwarzweiß

fn trace_binary(img: &RgbaImage, o: &TraceOptions, map: &Mapper, cancel: &AtomicBool, progress: &mut dyn FnMut(Progress)) -> Result<Vec<TracedPath>, TraceError> {
    let (w, h) = (img.width(), img.height());
    let mut bin = BinaryImage::new_w_h(w, h);
    for (i, p) in img.as_bytes().as_chunks::<4>().0.iter().enumerate() {
        let mut q = [p[0], p[1], p[2], p[3]];
        if o.ignore_transparent && q[3] < o.alpha_threshold {
            continue;
        }
        if !o.ignore_transparent {
            over_white(&mut q);
        }
        if luma(q[0], q[1], q[2]) < o.threshold as u32 {
            bin.set_pixel(i % w, i / w, true);
        }
    }
    check(cancel)?;
    progress(Progress { stage: Stage::Cluster, fraction: 0.2 });
    let clusters = bin.to_clusters(false);
    check(cancel)?;
    let min_area = (o.filter_speckle * o.filter_speckle) as usize;
    let n = clusters.len().max(1);
    let mode = o.simplify_mode();
    let mut out = Vec::new();
    for (k, cluster) in clusters.iter().enumerate() {
        if k % 16 == 0 {
            check(cancel)?;
            progress(Progress { stage: Stage::Paths, fraction: 0.2 + 0.8 * k as f32 / n as f32 });
        }
        if cluster.size() < min_area.max(1) {
            continue;
        }
        let cp = cluster.to_compound_path(mode, o.corner_threshold.to_radians(), o.length_threshold, o.max_iterations, o.splice_threshold.to_radians());
        let contours = convert(&cp, map);
        if !contours.is_empty() {
            out.push(TracedPath { contours, color: Rgba8::new(0, 0, 0, 255), pixels: cluster.size() });
        }
    }
    Ok(out)
}

// ------------------------------------------------------------------ Umwandlung

/// Pfade/Splines der Bibliothek → geschlossene Bézier-Konturen in Dokumentkoordinaten.
fn convert(cp: &CompoundPath, map: &Mapper) -> Vec<BezierContour> {
    let mut out = Vec::new();
    for el in &cp.paths {
        let nodes = match el {
            CompoundPathElement::PathI32(p) => polygon(p.path.iter().map(|q| (q.x as f64, q.y as f64)), map),
            CompoundPathElement::PathF64(p) => polygon(p.path.iter().map(|q| (q.x, q.y)), map),
            CompoundPathElement::Spline(s) => spline(&s.points.iter().map(|q| (q.x, q.y)).collect::<Vec<_>>(), map),
        };
        if nodes.len() >= 2 {
            out.push(BezierContour::new(nodes, true));
        }
    }
    out
}

fn polygon(pts: impl Iterator<Item = (f64, f64)>, map: &Mapper) -> Vec<BezierNode> {
    let mut v: Vec<Point> = Vec::new();
    for (x, y) in pts {
        let p = map.map(x, y);
        if v.last() != Some(&p) {
            v.push(p);
        }
    }
    while v.len() > 1 && v.first() == v.last() {
        v.pop();
    }
    if v.len() < 3 {
        return Vec::new();
    }
    v.into_iter().map(BezierNode::corner).collect()
}

/// Spline: Punkte `p0, (c1, c2, p1), (c1, c2, p2), …`; die letzte Ankerposition wiederholt den Anfang.
fn spline(pts: &[(f64, f64)], map: &Mapper) -> Vec<BezierNode> {
    let n = pts.len().saturating_sub(1) / 3;
    if n == 0 {
        return Vec::new();
    }
    let m = |i: usize| map.map(pts[i].0, pts[i].1);
    let mut nodes: Vec<BezierNode> = Vec::with_capacity(n + 1);
    for i in 0..n {
        let anchor = m(3 * i);
        let ctrl_in = if i == 0 { anchor } else { m(3 * i - 1) };
        nodes.push(BezierNode::new(anchor, ctrl_in, m(3 * i + 1)));
    }
    let last = m(3 * n);
    let into_last = m(3 * n - 1);
    if last == nodes[0].pt {
        nodes[0].ctrl_in = into_last;
    } else {
        nodes.push(BezierNode::new(last, into_last, last));
    }
    nodes
}
