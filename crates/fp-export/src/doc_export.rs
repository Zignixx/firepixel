//! Dokument-Export („Exportieren“, `exportFilesStyle`, 08 §1.5): HTML und Bilder, nur Bilder,
//! CSS und Bilder, CSS-Sprites, Ebenen/Status/Seiten in Dateien, Photoshop PSD, PDF, SVG —
//! mit einem [`PageRenderer`] für das flache Rechnen.

use fp_geom::IRect;
use fp_model::{Document, ElementKind, LayerKind, LayerSlot};
use fp_raster::{RgbaImage, Sampler};

use crate::animation::{animated_gif, animated_webp, apng, AnimFrameSrc};
use crate::export::{export_image, EncodeExtras};
use crate::html::{write_css_layers, write_html, CellContent, HtmlOptions, MapArea, Shims};
use crate::options::{ExportFormat, OptimizeSettings};
use crate::pipeline::ExportContext;
use crate::popup::MenuDate;
use crate::render::{page_rect, Background, LayerFilter, PageRenderer, RenderRequest};
use crate::slices::{auto_name, build_grid, collect_hotspots, collect_slices_with, frame_suffix_with, NameDate};
use crate::{ExportError, Result};

/// Exportart (`exportFilesStyle`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilesStyle {
    /// 1: HTML und Bilder.
    HtmlAndImages,
    /// 2: Nur Bilder.
    ImagesOnly,
    /// 5: CSS und Bilder.
    CssAndImages,
    /// 6: CSS-Sprites.
    CssSprites,
    /// 7: Ebenen in Dateien.
    LayersToFiles,
    /// 8: Status in Dateien.
    FramesToFiles,
    /// 9: Seiten in Dateien.
    PagesToFiles,
    /// 15: Photoshop PSD.
    Psd,
    /// 0: PDF.
    Pdf,
    /// SVG (Firepixel-Erweiterung).
    Svg,
}

impl FilesStyle {
    /// Aus `XFSi` (unbekannte Werte ⇒ HTML und Bilder).
    pub fn from_code(v: i32) -> FilesStyle {
        match v {
            0 => FilesStyle::Pdf,
            2 => FilesStyle::ImagesOnly,
            5 => FilesStyle::CssAndImages,
            6 => FilesStyle::CssSprites,
            7 => FilesStyle::LayersToFiles,
            8 => FilesStyle::FramesToFiles,
            9 => FilesStyle::PagesToFiles,
            15 => FilesStyle::Psd,
            _ => FilesStyle::HtmlAndImages,
        }
    }
}

impl FilesStyle {
    /// Wert für `XFSi` (08 §1.5); SVG hat keinen (Firepixel-Erweiterung).
    pub fn code(self) -> Option<i32> {
        Some(match self {
            FilesStyle::Pdf => 0,
            FilesStyle::HtmlAndImages => 1,
            FilesStyle::ImagesOnly => 2,
            FilesStyle::CssAndImages => 5,
            FilesStyle::CssSprites => 6,
            FilesStyle::LayersToFiles => 7,
            FilesStyle::FramesToFiles => 8,
            FilesStyle::PagesToFiles => 9,
            FilesStyle::Psd => 15,
            FilesStyle::Svg => return None,
        })
    }
}

/// „Seiten:“ des Exportdialogs (re\18 §2.1). Bei SVG steht `Selected` für „Ausgewählte Objekte
/// auf aktueller Seite“ (wie FXG, §2.7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PageScope {
    All,
    #[default]
    Current,
    Selected,
}

/// „Segmente:“ des Exportdialogs (re\18 §2.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SliceMode {
    None,
    #[default]
    Export,
    AlongGuides,
}

/// Einstellungen des Unterpaneels im Exportdialog (re\18 §2, Dateien nach §3). Ohne Paneel
/// (`DocExportOptions::panel = None`) gilt das bisherige Verhalten (Befehl `file.export`).
#[derive(Clone, Debug, PartialEq)]
pub struct PanelOptions {
    pub pages: PageScope,
    /// Ausgewählte Seiten (Indizes) für [`PageScope::Selected`].
    pub selected_pages: Vec<usize>,
    pub slices: SliceMode,
    /// „Nur ausgewählte Segmente“: Element-IDs der ausgewählten Segmente.
    pub selected_slices: Option<Vec<fp_model::ElementId>>,
    /// „Nur aktueller Status“ mit dem aktuellen Status `frame` (0-basiert).
    pub current_frame_only: bool,
    pub frame: usize,
    /// „Bereiche ohne Segmente einschließen“.
    pub include_unsliced: bool,
    /// Bildordner relativ zur HTML-Datei, leer oder mit „/“ am Ende.
    pub images_dir: String,
    /// „Bilder zuschneiden“.
    pub trim_images: bool,
    /// HTML-Endung ohne Punkt (HTML-Einrichtung, Vorgabe „htm“).
    pub html_ext: String,
    /// SVG „Ausgewählte Objekte“: Element-IDs.
    pub selected_elements: Vec<fp_model::ElementId>,
    /// Exportoptionen für CSS-Sprites (re\18 §4.2).
    pub sprite: crate::sprites::SpriteOptions,
    /// „Vorgabe für die Optimierung:“ des Sprite-Bilds; `None` = PNG32.
    pub sprite_settings: Option<OptimizeSettings>,
    /// „Exportoptionen - Ebenen/Status/Seiten in Dateien“ (re\18 §4.3): Präfix, Suffix, Dateityp
    /// (`None` = Dokumentoptimierungseinstellungen).
    pub file_prefix: FileAffix,
    pub file_suffix: FileAffix,
    pub file_settings: Option<OptimizeSettings>,
    /// Dokumentname für den Baustein „Dokumentname“.
    pub doc_name: String,
    /// PDF-Exportoptionen (re\18 §4.1).
    pub pdf: crate::vector_out::PdfOptions,
    /// Photoshop „Objekte: Jede Ebene auf Hintergrundebene reduzieren“ (re\18 §4.5).
    pub psd_flatten: bool,
}

/// Präfix/Suffix der Dateinamen bei Ebenen/Status/Seiten in Dateien (re\18 §4.3). Zwischen
/// Baustein und Name steht ein Unterstrich (A, ? 10.7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FileAffix {
    #[default]
    None,
    DocName,
    /// „# (1, 2, 3...)“
    Number,
    /// „# (01, 02, 03...)“
    Number2,
}

impl PanelOptions {
    /// Dateiname mit Präfix/Suffix; `index` ist 0-basiert.
    pub fn affixed(&self, stem: &str, index: usize) -> String {
        let part = |a: FileAffix| match a {
            FileAffix::None => None,
            FileAffix::DocName => Some(safe_name(&self.doc_name)),
            FileAffix::Number => Some(format!("{}", index + 1)),
            FileAffix::Number2 => Some(format!("{:02}", index + 1)),
        };
        let mut s = stem.to_string();
        if let Some(p) = part(self.file_prefix) {
            s = format!("{p}_{s}");
        }
        if let Some(x) = part(self.file_suffix) {
            s = format!("{s}_{x}");
        }
        s
    }
}

/// Hinweise eines Exports für den Dialog.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExportNotes {
    /// CSS-Selektornamen wurden geändert, damit sie eindeutig und gültig sind (re\18 §4.2).
    pub selectors_modified: bool,
}

impl Default for PanelOptions {
    fn default() -> Self {
        PanelOptions {
            pages: PageScope::Current,
            selected_pages: Vec::new(),
            slices: SliceMode::Export,
            selected_slices: None,
            current_frame_only: false,
            frame: 0,
            include_unsliced: true,
            images_dir: String::new(),
            trim_images: true,
            html_ext: "htm".into(),
            selected_elements: Vec::new(),
            sprite: crate::sprites::SpriteOptions::default(),
            sprite_settings: None,
            file_prefix: FileAffix::None,
            file_suffix: FileAffix::None,
            file_settings: None,
            doc_name: String::new(),
            pdf: crate::vector_out::PdfOptions::default(),
            psd_flatten: false,
        }
    }
}

/// Eine erzeugte Datei (Pfad relativ zum Zielordner).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportFile {
    pub path: String,
    pub bytes: Vec<u8>,
}

/// Textbreitenmessung für Pop-up-Menüs: (Text, Schrift, Größe, fett, kursiv) → Breite in px.
pub type MeasureFn = fn(&str, &str, f64, bool, bool) -> f64;

/// Optionen des Dokument-Exports.
#[derive(Clone, Debug)]
pub struct DocExportOptions {
    pub page: usize,
    pub style: FilesStyle,
    /// Basisname der Dateien (ohne Endung), zugleich Baustein „Dokumentname“ der Slice-Namen.
    pub base_name: String,
    /// Quelldatei (für den `fwtable`-Kommentar).
    pub src_file: String,
    /// Datum für Namensbausteine und Pop-up-Menüs ohne `CDTs`.
    pub date: MenuDate,
    /// Slices verwenden (sonst ganze Seite als ein Bild).
    pub use_slices: bool,
    /// Optimieren-Einstellungen; `None` = die der Seite.
    pub settings: Option<OptimizeSettings>,
    /// Textbreite für Pop-up-Menüs (Text, Schrift, Größe, fett, kursiv); Vorgabe 0,55·Größe je Zeichen.
    pub measure: Option<MeasureFn>,
    /// Unterpaneel des Exportdialogs (Dateinamen wie gemessen, re\18 §3); `None` = bisher.
    pub panel: Option<PanelOptions>,
    /// Programmweite Werte der HTML-Einrichtung (Voreinstellungen, re\18 §6.5).
    pub html_prefs: HtmlPrefs,
    /// Elementgrenzen für Schaltflächen-Segmente ohne aktiven Bereich (TW-19); `None` = Näherung
    /// [`crate::slices::approx_bounds`]. Die Engine übergibt ihre genauen Grenzen.
    pub bounds: Option<crate::slices::BoundsFn>,
}

/// Werte der HTML-Einrichtung, die in den Voreinstellungen stehen (re\18 §6.5): Endung
/// (`HTMLExtension`), Kommentare (`HTMLCommentsOn`), Kleinbuchstaben (`ForceLowercaseFilenames`),
/// leere Zellen (`HTMLEmptyCellColor`, `HTMLEmptyCellUsesCanvasColor`, `HTMLEmptyCellContents`).
#[derive(Clone, Debug, PartialEq)]
pub struct HtmlPrefs {
    /// Endung mit oder ohne Punkt; gilt, wenn das Dokument keine eigene hat.
    pub extension: String,
    pub comments: bool,
    pub lowercase: bool,
    /// Zellfarbe `0xRRGGBB` (nur ohne „Leinwandfarbe verwenden“).
    pub empty_cell_color: u32,
    pub empty_cell_canvas: bool,
    /// 0 Ohne, 1 Platzhalter-Grafik, 2 Geschütztes Leerzeichen.
    pub empty_cell_contents: i32,
}

impl Default for HtmlPrefs {
    fn default() -> Self {
        HtmlPrefs { extension: ".htm".into(), comments: false, lowercase: false, empty_cell_color: 0xFF_FFFF, empty_cell_canvas: true, empty_cell_contents: 1 }
    }
}

/// HTML-Endung ohne Punkt: die des Dokuments, sonst die der Voreinstellungen, sonst „htm“.
pub fn html_extension(html: Option<&fp_model::HtmlSettings>, prefs: &HtmlPrefs) -> String {
    let e = html.and_then(|h| h.file_extension.clone()).filter(|e| !e.trim().is_empty()).unwrap_or_else(|| prefs.extension.clone());
    let e = e.trim().trim_start_matches('.').to_string();
    if e.is_empty() { "htm".into() } else { e }
}

/// CSS für `body` aus den Dokumenteigenschaften der HTML-Einrichtung (re\18 §6.2); leer, wenn
/// alles auf den Vorgaben steht.
pub fn body_css(h: &fp_model::HtmlSettings) -> String {
    let mut css = Vec::new();
    if let Some(u) = h.background_url.as_deref().map(str::trim).filter(|u| !u.is_empty()) {
        css.push(format!("background-image: url(\"{}\");", u.replace('"', "%22")));
        css.push(format!("background-repeat: {};", ["no-repeat", "repeat", "repeat-x", "repeat-y"][h.background_repeat.unwrap_or(0).clamp(0, 3) as usize]));
        css.push(format!("background-attachment: {};", if h.background_attachment == Some(0) { "fixed" } else { "scroll" }));
    }
    match h.page_alignment.unwrap_or(0) {
        1 => css.push("text-align: center;".into()),
        2 => css.push("text-align: right;".into()),
        _ => {}
    }
    css.join(" ")
}

/// Name der externen CSS-Datei („CSS in externe Datei schreiben“, `externalCSS`, Vorgabe an):
/// der gespeicherte Name (`externalCSSFilename`), sonst `<Basisname>.css` neben der HTML-Datei;
/// leer, wenn die Option aus ist (Stile bleiben im Kopf).
pub fn external_css_file(h: &fp_model::HtmlSettings, base: &str) -> String {
    if h.external_css.unwrap_or(1) == 0 {
        return String::new();
    }
    match h.external_css_filename.as_deref().map(str::trim).filter(|f| !f.is_empty()) {
        Some(f) => f.to_string(),
        None => format!("{base}.css"),
    }
}

impl DocExportOptions {
    pub fn new(base_name: &str) -> DocExportOptions {
        DocExportOptions {
            page: 0,
            style: FilesStyle::HtmlAndImages,
            base_name: base_name.to_string(),
            src_file: String::new(),
            date: MenuDate { year: 2026, month: 1, day: 1, ..MenuDate::default() },
            use_slices: true,
            settings: None,
            measure: None,
            panel: None,
            html_prefs: HtmlPrefs::default(),
            bounds: None,
        }
    }
}

fn default_measure(t: &str, _f: &str, size: f64, bold: bool, _i: bool) -> f64 {
    (t.chars().count() as f64 * size.max(1.0) * if bold { 0.62 } else { 0.55 }).round()
}

/// Export-Kontext einer Seite (Leinwandfarbe, Auflösung in dpi).
pub fn page_context(doc: &Document, page: usize) -> ExportContext {
    let mut ctx = ExportContext::default();
    if let Some(p) = doc.pages.get(page) {
        ctx.canvas = p.root.background.0;
        let r = p.canvas.resolution;
        ctx.resolution_dpi = if p.canvas.resolution_unit == 1 { r * 2.54 } else { r };
    }
    ctx
}

/// Optimieren-Einstellungen einer Seite (Matte aus dem HTML-Teil).
pub fn page_settings(doc: &Document, page: usize) -> OptimizeSettings {
    doc.pages
        .get(page)
        .and_then(|p| p.legacy_export.as_ref().or(p.root.export.as_ref()))
        .map(|e| OptimizeSettings::from_model(e, None))
        .unwrap_or_default()
}

fn render(r: &dyn PageRenderer, doc: &Document, page: usize, frame: usize, s: &OptimizeSettings, layers: LayerFilter) -> Result<RgbaImage> {
    let bg = if s.matte.is_some() { Background::Transparent } else { Background::Canvas };
    r.render(doc, &RenderRequest { page, frame, layers, background: bg, area: None }).map_err(ExportError::Render)
}

/// Exportbereich (`crop`, Dokumentpixel) innerhalb eines Bildes `full_w` × `full_h`; ohne bzw. mit
/// leerem Exportbereich das ganze Bild.
pub fn crop_rect(full_w: usize, full_h: usize, s: &OptimizeSettings) -> IRect {
    let full = IRect::new(0, 0, full_w as i32, full_h as i32);
    match s.crop {
        Some([l, t, r, b]) if !IRect::new(l, t, r, b).is_empty() => {
            let c = IRect::new(l, t, r, b).intersect(&full);
            if c.is_empty() { full } else { c }
        }
        _ => full,
    }
}

/// Ausgabegröße nach Exportbereich und Skalierung (re\17-bildvorschau.md §3): Prozent wirken auf
/// den Exportbereich; eine feste Zielgröße (Breite/Höhe) gilt für das ganze Bild, ein Exportbereich
/// wird im selben Verhältnis skaliert. Genau die Größe, die [`apply_crop_scale`] erzeugt.
pub fn crop_scale_size(full_w: usize, full_h: usize, s: &OptimizeSettings) -> (usize, usize) {
    let c = crop_rect(full_w, full_h, s);
    let (cw, ch) = (c.width() as i64, c.height() as i64);
    match s.scale {
        Some(sc) if sc.use_scale && sc.percent > 0 && sc.percent != 100 => ((cw * sc.percent as i64 / 100).max(1) as usize, (ch * sc.percent as i64 / 100).max(1) as usize),
        Some(sc) if sc.use_scale && sc.width > 0 && sc.height > 0 && full_w > 0 && full_h > 0 => {
            let k = |c: i64, target: i32, full: usize| ((c * target as i64 * 2 + full as i64) / (2 * full as i64)).max(1) as usize;
            (k(cw, sc.width, full_w), k(ch, sc.height, full_h))
        }
        _ => (cw as usize, ch as usize),
    }
}

/// Exportbereich und Skalierung anwenden (dieselbe Kette für Export, Bildvorschau und „Optimieren
/// auf Größe“). Die Ausgabegröße ist [`crop_scale_size`].
pub fn apply_crop_scale(img: RgbaImage, s: &OptimizeSettings) -> RgbaImage {
    let (full_w, full_h) = (img.width(), img.height());
    let mut img = img;
    if s.crop.is_some() {
        let b = img.bounds();
        let r = crop_rect(full_w, full_h, s);
        let r = IRect::new(r.left + b.left, r.top + b.top, r.right + b.left, r.bottom + b.top);
        if r != b {
            img = img.cropped(r);
        }
    }
    let (w, h) = crop_scale_size(full_w, full_h, s);
    if w != img.width() || h != img.height() {
        img = fp_raster::resample::resize(&img, w as u32, h as u32, Sampler::Bicubic);
    }
    img
}

fn frame_count(doc: &Document, page: usize) -> usize {
    doc.pages.get(page).map(|p| p.root.frame_count.max(1) as usize).unwrap_or(1)
}

fn frames_of(r: &dyn PageRenderer, doc: &Document, page: usize, s: &OptimizeSettings) -> Result<(usize, usize, Vec<AnimFrameSrc>)> {
    let n = frame_count(doc, page);
    let mut out = Vec::with_capacity(n);
    let (mut w, mut h) = (0, 0);
    for f in 0..n {
        let img = apply_crop_scale(render(r, doc, page, f, s, LayerFilter::All)?, s);
        w = img.width();
        h = img.height();
        out.push(AnimFrameSrc { pixels: crate::pipeline::image_pixels(&img), info: s.anim.frames.get(f).cloned().unwrap_or_default() });
    }
    Ok((w, h, out))
}

/// Exportiert ein ganzes Bild bzw. eine Animation.
pub fn export_whole(r: &dyn PageRenderer, doc: &Document, page: usize, s: &OptimizeSettings, ex: &EncodeExtras) -> Result<Vec<u8>> {
    let ctx = page_context(doc, page);
    if s.format.is_animated() {
        return export_animation(r, doc, page, s, &ctx);
    }
    Ok(export_still(r, doc, page, 0, s, ex, &ctx)?.bytes)
}

/// Animation (alle Status) wie [`export_whole`].
fn export_animation(r: &dyn PageRenderer, doc: &Document, page: usize, s: &OptimizeSettings, ctx: &ExportContext) -> Result<Vec<u8>> {
    let (w, h, fr) = frames_of(r, doc, page, s)?;
    match s.format {
        ExportFormat::AnimatedGif => animated_gif(&fr, w, h, s, ctx),
        ExportFormat::Apng => apng(&fr, w, h, ctx, s.anim.loop_count),
        _ => animated_webp(&fr, w, h, s.webp.lossless, s.webp.quality, s.anim.loop_count),
    }
}

/// Ein Status: rendern, Exportbereich/Skalierung, optimieren, kodieren – die Kette von
/// [`export_whole`] und [`preview_whole`].
fn export_still(r: &dyn PageRenderer, doc: &Document, page: usize, frame: usize, s: &OptimizeSettings, ex: &EncodeExtras, ctx: &ExportContext) -> Result<crate::export::ExportedImage> {
    let img = apply_crop_scale(render(r, doc, page, frame, s, LayerFilter::All)?, s);
    export_image(&img, s, ctx, ex)
}

/// Ergebnis der Bildvorschau eines Feldes (re\17-bildvorschau.md §5.7, §9).
#[derive(Clone, Debug, PartialEq)]
pub struct WholePreview {
    /// Dateigröße in Byte, genau wie [`export_whole`] mit denselben Einstellungen und Kodierern
    /// (bei Animationen die ganze Datei mit allen Status).
    pub bytes: usize,
    /// Anzahl Farben der Palette (indiziert) bzw. 0.
    pub colors: usize,
    /// Palette (indiziert, `0xAARRGGBB`), sonst leer.
    pub palette: Vec<u32>,
    /// Pixelzähler je Paletteneintrag (indiziert), sonst leer.
    pub counts: Vec<u64>,
    /// Transparenzindex des Indexbilds (kann hinter der Palette liegen, Index-Alpha).
    pub transparent_index: Option<usize>,
    /// Indizes des angezeigten Bildes zeilenweise (indiziert; gleiche Größe wie `image`), sonst
    /// leer – für Pipetten und Farbtabelle der Bildvorschau (Pixel → Paletteneintrag).
    pub indices: Vec<u8>,
    /// Angezeigtes Bild: Indexbild durch die Palette aufgelöst, JPEG echt dekodiert.
    pub image: RgbaImage,
}

/// Ergebnis von [`optimize_whole_to_size`].
#[derive(Clone, Debug, PartialEq)]
pub enum SizeOutcome {
    /// Gefundene Einstellung (zuletzt berechnete der Bisektion, kann das Ziel überschreiten).
    Found(crate::size::SizeResult),
    /// Ziel kleiner als die kleinstmögliche Datei: die ursprünglichen Werte bleiben (Meldung
    /// „Die resultierende Datei wäre zu klein …“, re\17-bildvorschau.md §2.8). `floor` = kleinste
    /// erreichbare Größe in Byte.
    TooSmall { floor: usize },
    /// Format ohne Suche (PNG 24/32, TIFF, BMP 24 …, re\17 §9): Einstellung unverändert.
    NotApplicable,
}

/// „Optimieren auf Größe“ (Assistent für Dateigrößenoptimierung, re\17-bildvorschau.md §2.8,
/// 08 §7) für ein ganzes Bild: dieselbe Kette wie [`export_whole`] (Status `frame`, Exportbereich,
/// Skalierung, gleiche Kodierer), dann Bisektion über Farbanzahl bzw. JPEG-Qualität. Vorab wird
/// die kleinstmögliche Größe bestimmt (2 Farben bzw. Qualität 2); liegt das Ziel darunter, kommt
/// [`SizeOutcome::TooSmall`]. `target` in Byte.
pub fn optimize_whole_to_size(r: &dyn PageRenderer, doc: &Document, page: usize, frame: usize, s: &OptimizeSettings, target: usize, ex: &EncodeExtras) -> Result<SizeOutcome> {
    let ctx = page_context(doc, page);
    let img = apply_crop_scale(render(r, doc, page, frame.min(frame_count(doc, page) - 1), s, LayerFilter::All)?, s);
    let Some(floor) = crate::size::size_floor(&img, s, &ctx, ex)? else {
        return Ok(SizeOutcome::NotApplicable);
    };
    if target < floor {
        return Ok(SizeOutcome::TooSmall { floor });
    }
    let res = crate::size::optimize_to_size(&img, s, target, &ctx, ex)?;
    Ok(if res.changed { SizeOutcome::Found(res) } else { SizeOutcome::NotApplicable })
}

/// Bildvorschau eines Feldes: dieselbe Kette wie [`export_whole`] (gleiche Kodierer und
/// Parameter, damit die Größe stimmt), dazu das Bild des Status `frame` für die Anzeige.
/// `cancelled` wird zwischen den Schritten gefragt; `Ok(None)` = abgebrochen.
///
/// Bei Einzelbildern ist die Größe die des Status `frame` (beim Export Status 1 = `frame` 0); bei
/// animierten Formaten die der ganzen Animation, das Bild zeigt den Status `frame`.
pub fn preview_whole(r: &dyn PageRenderer, doc: &Document, page: usize, frame: usize, s: &OptimizeSettings, ex: &EncodeExtras, cancelled: &dyn Fn() -> bool) -> Result<Option<WholePreview>> {
    let ctx = page_context(doc, page);
    let still = export_still(r, doc, page, frame.min(frame_count(doc, page) - 1), s, ex, &ctx)?;
    if cancelled() {
        return Ok(None);
    }
    let bytes = if s.format.is_animated() {
        let b = export_animation(r, doc, page, s, &ctx)?.len();
        if cancelled() {
            return Ok(None);
        }
        b
    } else {
        still.bytes.len()
    };
    let (palette, counts, transparent_index, indices) = match &still.optimized {
        crate::Optimized::Indexed(ix) => (ix.palette.clone(), ix.counts.clone(), ix.transparent_index, ix.indices.clone()),
        _ => (Vec::new(), Vec::new(), None, Vec::new()),
    };
    let image = crate::export::preview(&still, s, (0, 0));
    let indices = if indices.len() == image.width() * image.height() { indices } else { Vec::new() };
    Ok(Some(WholePreview { bytes, colors: still.colors, palette, counts, transparent_index, indices, image }))
}

fn safe_name(s: &str) -> String {
    let v: String = s.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '_' }).collect();
    if v.is_empty() { "unbenannt".into() } else { v }
}

/// Photoshop-Datei einer Seite (Ebenen einzeln, Hilfslinien).
fn psd_bytes(doc: &Document, r: &dyn PageRenderer, page: usize, flatten: bool) -> Result<Vec<u8>> {
    let p = doc.pages.get(page).ok_or_else(|| ExportError::Encode(fp_i18n::tr!("err-export-d-page-missing", page = page.to_string())))?;
    let ctx = page_context(doc, page);
    let rect = page_rect(doc, page).unwrap_or_default();
    let mut layers = Vec::new();
    for slot in &p.root.layers {
        let LayerSlot::Own(l) = slot else { continue };
        if l.kind == LayerKind::Web {
            continue;
        }
        let img = r
            .render(doc, &RenderRequest { page, frame: 0, layers: LayerFilter::Only(vec![l.id]), background: Background::Transparent, area: Some(rect) })
            .map_err(ExportError::Render)?;
        let ab = img.alpha_bounds();
        let img = if ab.is_empty() { RgbaImage::empty_at(rect.left, rect.top) } else { img.cropped(ab) };
        let img = img.translated(-rect.left, -rect.top);
        layers.push(crate::psd_out::PsdLayer { name: l.name.clone(), image: img, opacity: l.opacity.0, blend: l.blend.0, visible: l.is_visible() });
    }
    let comp = r.render(doc, &RenderRequest { page, frame: 0, ..Default::default() }).map_err(ExportError::Render)?.translated(-rect.left, -rect.top);
    if flatten {
        // „Jede Ebene auf Hintergrundebene reduzieren“: eine Ebene mit dem flachen Seitenbild
        layers = vec![crate::psd_out::PsdLayer { name: fp_i18n::t("err-export-d-psd-background"), image: comp.clone(), opacity: 255, blend: 0, visible: true }];
    }
    let guides: Vec<crate::psd_out::PsdGuide> = p
        .root
        .guides
        .as_ref()
        .map(|g| g.vertical.iter().map(|&v| crate::psd_out::PsdGuide { pos: v - p.canvas.origin.x, vertical: true }).chain(g.horizontal.iter().map(|&v| crate::psd_out::PsdGuide { pos: v - p.canvas.origin.y, vertical: false })).collect())
        .unwrap_or_default();
    crate::psd_out::write_psd(rect.width() as usize, rect.height() as usize, &layers, &comp, ctx.resolution_dpi, &guides)
}

/// PDF mit je einer Seite für `pages`.
fn pdf_bytes(doc: &Document, r: &dyn PageRenderer, pages: &[usize], title: &str, opt: &crate::vector_out::PdfOptions) -> Result<Vec<u8>> {
    let mut imgs = Vec::new();
    let mut sizes = Vec::new();
    for &i in pages {
        let img = r.render(doc, &RenderRequest { page: i, ..Default::default() }).map_err(ExportError::Render)?;
        let c = page_context(doc, i);
        let k = 72.0 / c.resolution_dpi.max(1.0) as f64;
        sizes.push((img.width() as f64 * k, img.height() as f64 * k));
        imgs.push(img);
    }
    let pages: Vec<crate::vector_out::PdfPage> = imgs.iter().zip(&sizes).map(|(i, &(w, h))| crate::vector_out::PdfPage { image: i, width_pt: w, height_pt: h }).collect();
    crate::vector_out::write_pdf_with(&pages, title, opt)
}

/// Segmente entlang der Hilfslinien (re\18 §2.1 „Segmente entlang Hilfslinien erstellen“).
fn guide_slices(doc: &Document, page: usize) -> Vec<crate::slices::SliceSpec> {
    let Some(rect) = page_rect(doc, page) else { return Vec::new() };
    let (l, t, r, b) = (rect.left as f32, rect.top as f32, rect.right as f32, rect.bottom as f32);
    let g = doc.pages.get(page).and_then(|p| p.root.guides.clone()).unwrap_or_default();
    let mut xs: Vec<f32> = std::iter::once(l).chain(g.vertical.iter().copied().filter(|&v| v > l && v < r)).chain(std::iter::once(r)).collect();
    let mut ys: Vec<f32> = std::iter::once(t).chain(g.horizontal.iter().copied().filter(|&v| v > t && v < b)).chain(std::iter::once(b)).collect();
    for v in [&mut xs, &mut ys] {
        v.sort_by(|a, b| a.total_cmp(b));
        v.dedup();
    }
    let mut out = Vec::new();
    for y in ys.windows(2) {
        for x in xs.windows(2) {
            out.push(crate::slices::SliceSpec {
                rect: [x[0], y[0], x[1], y[1]],
                id: out.len() as u32 + 1,
                poly: false,
                base_name: None,
                url: String::new(),
                alt: String::new(),
                target: String::new(),
                kind: 0,
                html: String::new(),
                polygon: Vec::new(),
                behaviors: Vec::new(),
            });
        }
    }
    out
}

/// Kennungen der Segmente zu Element-IDs (gleiche Zählung wie [`collect_slices`]).
fn slice_ids_of(doc: &Document, page: usize, elements: &[fp_model::ElementId]) -> std::collections::HashSet<u32> {
    let mut out = std::collections::HashSet::new();
    let Some(p) = doc.pages.get(page) else { return out };
    let mut next = 1u32;
    for slot in p.root.layers.iter().rev() {
        let LayerSlot::Own(l) = slot else { continue };
        if l.kind != LayerKind::Web {
            continue;
        }
        let Some(cel) = l.cels.first() else { continue };
        for e in &cel.elements {
            let ElementKind::WebObject(w) = &e.kind else { continue };
            let Some(sp) = &w.slice else { continue };
            if !e.common.visible {
                continue;
            }
            let id = sp.slice_id.unwrap_or_else(|| {
                next += 1;
                next - 1
            });
            if elements.contains(&e.id) {
                out.insert(id);
            }
        }
    }
    out
}

/// Eindeutiger Dateiname: Doppelte bekommen „_1“, „_2“ … (gemessen bei Ebenen in Dateien).
fn unique(name: String, used: &mut std::collections::HashSet<String>) -> String {
    if used.insert(name.to_lowercase()) {
        return name;
    }
    let mut n = 1;
    loop {
        let c = format!("{name}_{n}");
        if used.insert(c.to_lowercase()) {
            return c;
        }
        n += 1;
    }
}

/// Seiten nach „Seiten:“ (ohne Masterseite).
fn scope_pages(doc: &Document, page: usize, pn: &PanelOptions) -> Vec<usize> {
    let all: Vec<usize> = (0..doc.pages.len()).filter(|&i| !doc.pages[i].master()).collect();
    match pn.pages {
        PageScope::All => all,
        PageScope::Current => vec![page],
        PageScope::Selected => {
            let v: Vec<usize> = pn.selected_pages.iter().copied().filter(|i| all.contains(i)).collect();
            if v.is_empty() { vec![page] } else { v }
        }
    }
}

/// Dateiname einer Seite (Seitenname, bereinigt).
fn page_stem(doc: &Document, i: usize) -> String {
    safe_name(doc.pages.get(i).and_then(|p| p.name.clone()).filter(|n| !n.trim().is_empty()).as_deref().unwrap_or(&format!("{}", i + 1)))
}

/// Einzelbild-Einstellung (animierte Formate ⇒ Einzelbild desselben Formats).
fn still(s: &OptimizeSettings) -> OptimizeSettings {
    let mut s = s.clone();
    if s.format.is_animated() {
        s.format = match s.format {
            ExportFormat::AnimatedGif => ExportFormat::Gif,
            ExportFormat::Apng => ExportFormat::Png,
            _ => ExportFormat::WebPLossless,
        };
    }
    s
}

/// Ein Status/Ebenenbild, mit „Bilder zuschneiden“ auf den deckenden Inhalt beschnitten.
fn layer_image(r: &dyn PageRenderer, doc: &Document, page: usize, frame: usize, s: &OptimizeSettings, layers: LayerFilter, trim: bool) -> Result<RgbaImage> {
    let img = render(r, doc, page, frame, s, layers.clone())?;
    if !trim {
        return Ok(img);
    }
    let t = r.render(doc, &RenderRequest { page, frame, layers, background: Background::Transparent, area: None }).map_err(ExportError::Render)?;
    let ab = t.alpha_bounds();
    Ok(if ab.is_empty() { img } else { img.cropped(ab) })
}

/// Arten mit eigenem Ablauf im Exportdialog (Seitenwahl, gemessene Dateinamen, re\18 §3).
/// `None` = weiter wie ohne Paneel für die eine Seite.
fn export_with_panel(doc: &Document, r: &dyn PageRenderer, o: &DocExportOptions, pn: &PanelOptions, ex: &EncodeExtras, notes: &mut ExportNotes) -> Result<Option<Vec<ExportFile>>> {
    let page = o.page;
    let base = safe_name(&o.base_name);
    let s = o.settings.clone().unwrap_or_else(|| page_settings(doc, page));
    let ctx = page_context(doc, page);
    let mut files = Vec::new();
    let mut used = std::collections::HashSet::new();
    match o.style {
        FilesStyle::FramesToFiles => {
            // gemessen: s01.png, s02.png … (zweistellig)
            let s = pn.file_settings.clone().unwrap_or(s);
            let ss = still(&s);
            for f in 0..frame_count(doc, page) {
                let img = apply_crop_scale(layer_image(r, doc, page, f, &s, LayerFilter::All, pn.trim_images)?, &s);
                files.push(ExportFile { path: format!("{}.{}", pn.affixed(&format!("s{:02}", f + 1), f), ss.format.extension()), bytes: export_image(&img, &ss, &ctx, ex)?.bytes });
            }
        }
        FilesStyle::LayersToFiles => {
            // gemessen: Dateiname = Ebenenname (Leerzeichen ⇒ „_“), Doppelte „_1“, Web-Ebene übersprungen
            let p = &doc.pages[page];
            let s = pn.file_settings.clone().unwrap_or(s);
            let ss = still(&s);
            let frame = pn.frame.min(frame_count(doc, page) - 1);
            let mut k = 0;
            for slot in &p.root.layers {
                let LayerSlot::Own(l) = slot else { continue };
                if l.kind == LayerKind::Web {
                    continue;
                }
                let img = apply_crop_scale(layer_image(r, doc, page, frame, &s, LayerFilter::Only(vec![l.id]), pn.trim_images)?, &s);
                let name = unique(pn.affixed(&safe_name(&l.name), k), &mut used);
                k += 1;
                files.push(ExportFile { path: format!("{name}.{}", ss.format.extension()), bytes: export_image(&img, &ss, &ctx, ex)?.bytes });
            }
        }
        FilesStyle::PagesToFiles => {
            // gemessen: „Aktuelle Seite“ ⇒ eingegebener Name; mehrere Seiten ⇒ Seitennamen (A)
            for (k, i) in scope_pages(doc, page, pn).into_iter().enumerate() {
                let sp = pn.file_settings.clone().or_else(|| o.settings.clone()).unwrap_or_else(|| page_settings(doc, i));
                let stem = if pn.pages == PageScope::Current { pn.affixed(&base, k) } else { unique(pn.affixed(&page_stem(doc, i), k), &mut used) };
                files.push(ExportFile { path: format!("{stem}.{}", sp.format.extension()), bytes: export_whole(r, doc, i, &sp, ex)? });
            }
        }
        FilesStyle::Pdf => {
            files.push(ExportFile { path: format!("{base}.pdf"), bytes: pdf_bytes(doc, r, &scope_pages(doc, page, pn), &o.base_name, &pn.pdf)? });
        }
        FilesStyle::Svg if pn.pages == PageScope::Selected => {
            let d = only_elements(doc, page, &pn.selected_elements);
            let frame = pn.frame.min(frame_count(doc, page) - 1);
            files.push(ExportFile { path: format!("{base}.svg"), bytes: crate::vector_out::write_svg(&d, page, frame, None)?.into_bytes() });
        }
        _ if pn.pages == PageScope::Current => return Ok(None),
        _ => {
            // HTML-Arten, PSD, SVG über mehrere Seiten: je Seite mit dem Seitennamen (A, re\18 §3)
            let mut seen = std::collections::HashSet::new();
            for i in scope_pages(doc, page, pn) {
                let mut sub = PanelOptions { pages: PageScope::Current, ..pn.clone() };
                sub.selected_slices = None;
                let o2 = DocExportOptions { page: i, base_name: unique(page_stem(doc, i), &mut used), settings: None, panel: Some(sub), ..o.clone() };
                for f in export_document_noted(doc, r, &o2, ex, notes)? {
                    if seen.insert(f.path.to_lowercase()) {
                        files.push(f);
                    }
                }
            }
        }
    }
    Ok(Some(files))
}

/// Kopie des Dokuments, in der auf Seite `page` nur die Elemente `ids` stehen (SVG „Ausgewählte
/// Objekte auf aktueller Seite“).
fn only_elements(doc: &Document, page: usize, ids: &[fp_model::ElementId]) -> Document {
    let mut d = doc.clone();
    if let Some(p) = d.page_mut(page) {
        for slot in p.root.layers.iter_mut() {
            let LayerSlot::Own(l) = slot else { continue };
            let l = std::sync::Arc::make_mut(l);
            for cel in l.cels.iter_mut() {
                std::sync::Arc::make_mut(cel).elements.retain(|e| ids.contains(&e.id));
            }
        }
    }
    d
}

/// Dokument exportieren.
pub fn export_document(doc: &Document, r: &dyn PageRenderer, o: &DocExportOptions, ex: &EncodeExtras) -> Result<Vec<ExportFile>> {
    export_document_noted(doc, r, o, ex, &mut ExportNotes::default())
}

/// Wie [`export_document`], dazu die Hinweise für den Dialog (geänderte Selektornamen).
pub fn export_document_noted(doc: &Document, r: &dyn PageRenderer, o: &DocExportOptions, ex: &EncodeExtras, notes: &mut ExportNotes) -> Result<Vec<ExportFile>> {
    if let Some(pn) = &o.panel
        && let Some(files) = export_with_panel(doc, r, o, pn, ex, notes)?
    {
        return Ok(files);
    }
    let page = o.page;
    let p = doc.pages.get(page).ok_or_else(|| ExportError::Encode(fp_i18n::tr!("err-export-d-page-missing", page = page.to_string())))?;
    let s = o.settings.clone().unwrap_or_else(|| page_settings(doc, page));
    let ext = s.format.extension();
    let ctx = page_context(doc, page);
    let base = safe_name(&o.base_name);
    let mut files = Vec::new();
    match o.style {
        FilesStyle::FramesToFiles => {
            for f in 0..frame_count(doc, page) {
                let img = apply_crop_scale(render(r, doc, page, f, &s, LayerFilter::All)?, &s);
                files.push(ExportFile { path: format!("{base}_f{}.{ext}", f + 1), bytes: export_image(&img, &s, &ctx, ex)?.bytes });
            }
        }
        FilesStyle::PagesToFiles => {
            for (i, pg) in doc.pages.iter().enumerate() {
                if pg.master() {
                    continue;
                }
                let sp = o.settings.clone().unwrap_or_else(|| page_settings(doc, i));
                let name = safe_name(pg.name.as_deref().unwrap_or(&format!("{base}_{}", i + 1)));
                files.push(ExportFile { path: format!("{name}.{}", sp.format.extension()), bytes: export_whole(r, doc, i, &sp, ex)? });
            }
        }
        FilesStyle::LayersToFiles => {
            for slot in &p.root.layers {
                let LayerSlot::Own(l) = slot else { continue };
                if l.kind == LayerKind::Web {
                    continue;
                }
                let img = apply_crop_scale(render(r, doc, page, 0, &s, LayerFilter::Only(vec![l.id]))?, &s);
                files.push(ExportFile { path: format!("{}.{ext}", safe_name(&l.name)), bytes: export_image(&img, &s, &ctx, ex)?.bytes });
            }
        }
        FilesStyle::Psd => {
            files.push(ExportFile { path: format!("{base}.psd"), bytes: psd_bytes(doc, r, page, o.panel.as_ref().is_some_and(|p| p.psd_flatten))? });
        }
        FilesStyle::Pdf => {
            let pages: Vec<usize> = (0..doc.pages.len()).filter(|&i| !doc.pages[i].master()).collect();
            files.push(ExportFile { path: format!("{base}.pdf"), bytes: pdf_bytes(doc, r, &pages, &o.base_name, &crate::vector_out::PdfOptions::default())? });
        }
        FilesStyle::Svg => {
            let frame = o.panel.as_ref().map(|pn| pn.frame.min(frame_count(doc, page) - 1)).unwrap_or(0);
            files.push(ExportFile { path: format!("{base}.svg"), bytes: crate::vector_out::write_svg(doc, page, frame, None)?.into_bytes() });
        }
        FilesStyle::HtmlAndImages | FilesStyle::ImagesOnly | FilesStyle::CssAndImages | FilesStyle::CssSprites => {
            let pn = o.panel.as_ref();
            // „Dateiname in Kleinbuchstaben“ (HTML-Einrichtung, re\18 §6.2)
            let base = if o.html_prefs.lowercase { base.to_lowercase() } else { base };
            let slices = match pn.map(|x| x.slices) {
                None if o.use_slices => collect_slices_with(doc, page, o.bounds.unwrap_or(crate::slices::approx_bounds)),
                Some(SliceMode::Export) => collect_slices_with(doc, page, o.bounds.unwrap_or(crate::slices::approx_bounds)),
                Some(SliceMode::AlongGuides) => guide_slices(doc, page),
                _ => Vec::new(),
            };
            // „Nur ausgewählte Segmente“ (Kennungen der Segmente)
            let only: Option<std::collections::HashSet<u32>> = pn.and_then(|x| x.selected_slices.as_ref()).map(|ids| slice_ids_of(doc, page, ids));
            if pn.is_some() && (only.as_ref().is_some_and(|s| s.is_empty()) || (o.style == FilesStyle::CssSprites && slices.is_empty())) {
                return Err(ExportError::Encode(fp_i18n::t("err-export-d-no-slices")));
            }
            if slices.is_empty() && o.style == FilesStyle::ImagesOnly {
                files.push(ExportFile { path: format!("{base}.{ext}"), bytes: export_whole(r, doc, page, &s, ex)? });
                return Ok(files);
            }
            let rect = page_rect(doc, page).unwrap_or_default();
            let grid = build_grid([rect.left as f32, rect.top as f32, rect.right as f32, rect.bottom as f32], &slices);
            let html_set = p.root.export.as_ref().and_then(|e| e.html.as_deref()).cloned().unwrap_or_else(fp_model::HtmlSettings::cs6_default);
            let tokens = html_set.slice_naming.clone().unwrap_or_else(|| vec![6, 15, 1, 0, 0, 0]);
            let ftokens = html_set.frame_naming.clone().unwrap_or_else(|| vec![5, 1]);
            let state_names = html_set.state_names.clone().unwrap_or_else(|| [String::new(), "_o".into(), "_d".into(), "_od".into()]);
            let date = NameDate { year: o.date.year, month: o.date.month, day: o.date.day };
            let images_dir = match pn {
                Some(x) => x.images_dir.clone(),
                None => html_set.images_dir.clone().unwrap_or_default(),
            };
            let images_dir = if images_dir.is_empty() || images_dir.ends_with('/') { images_dir } else { format!("{images_dir}/") };
            let discard = pn.map(|x| !x.include_unsliced).unwrap_or_else(|| html_set.discard_unspecified_slices.unwrap_or(false));
            let total = grid.cells.iter().flatten().filter(|c| !c.skip).count();
            // Statusteil der Rollover-Bilder wie gemessen „_s2“, auch ohne Exportdialog (re\18 §3)
            let letter = 's';
            let one_frame = pn.is_some_and(|x| x.current_frame_only);
            let base_frame = pn.filter(|x| x.current_frame_only).map(|x| x.frame.min(frame_count(doc, page) - 1)).unwrap_or(0);
            // Status-Bilder rendern (Grundbild immer, weitere nach Bedarf)
            let mut frame_imgs: Vec<Option<RgbaImage>> = vec![None; frame_count(doc, page)];
            frame_imgs[0] = Some(render(r, doc, page, base_frame, &s, LayerFilter::All)?);
            let mut cells: Vec<Vec<CellContent>> = Vec::new();
            let mut seq = 0;
            let mut cell_of: std::collections::HashMap<Option<u32>, (usize, usize, String)> = Default::default();
            for (ri, row) in grid.cells.iter().enumerate() {
                let mut out_row = Vec::new();
                for (ci, cell) in row.iter().enumerate() {
                    if cell.skip {
                        out_row.push(CellContent::default());
                        continue;
                    }
                    seq += 1;
                    let spec = cell.slice().and_then(|id| slices.iter().find(|x| x.id == id));
                    let name = spec.and_then(|x| x.base_name.clone()).unwrap_or_else(|| auto_name(&tokens, ri + 1, ci + 1, &base, seq, total, date));
                    let name = if o.html_prefs.lowercase { name.to_lowercase() } else { name };
                    let mut cc = CellContent { name: safe_name(&name), image: None, html: None, slice: cell.slice() };
                    if let Some(x) = spec
                        && x.kind == 3
                    {
                        cc.html = Some(x.html.clone());
                    } else if !(cell.undefined && discard) {
                        cc.image = Some(format!("{images_dir}{}.{}", cc.name, ext));
                    }
                    if let Some(id) = cell.slice() {
                        cell_of.entry(Some(id)).or_insert((ri, ci, cc.name.clone()));
                    }
                    out_row.push(cc);
                }
                cells.push(out_row);
            }
            let ext_s = ext.to_string();
            let frame_name = |slice: Option<u32>, frame: i32| -> Option<String> {
                let (_, _, n) = cell_of.get(&slice)?;
                if frame <= 0 {
                    Some(format!("{images_dir}{n}.{ext_s}"))
                } else {
                    Some(format!("{images_dir}{n}{}.{ext_s}", frame_suffix_with(&ftokens, frame as usize + 1, &state_names, letter)))
                }
            };
            let areas: Vec<MapArea> = collect_hotspots(doc, page)
                .into_iter()
                .map(|(shape, rc, poly, url, alt, target, behaviors)| MapArea { shape, rect: [rc.left, rc.top, rc.right, rc.bottom], polygon: poly, url, alt, target, behaviors })
                .collect();
            let mut ho = HtmlOptions {
                shims: Shims::from_code(html_set.shim_generation.unwrap_or(1)),
                xhtml: html_set.xhtml.unwrap_or(1) == 1,
                charset: ["iso-8859-1", "utf-8", "iso-8859-1", "Shift_JIS", "iso-2022-jp", "EUC-JP", "big5", "gb2312", "euc-kr", "iso-8859-2", "x-mac-ce", "windows-1250", "iso-8859-5", "KOI8-R", "x-mac-cyrillic", "windows-1251", "iso-8859-7", "x-mac-greek", "windows-1253", "windows-1252"]
                    .get(html_set.encoding.unwrap_or(1).clamp(0, 19) as usize)
                    .copied()
                    .unwrap_or("utf-8")
                    .to_string(),
                title: format!("{base}.{ext}"),
                src_file: o.src_file.clone(),
                page_name: p.name.clone().unwrap_or_default(),
                base_name: format!("{base}.{ext}"),
                style_name: html_set.template_name.clone().filter(|t| !t.is_empty()).unwrap_or_else(|| "Firepixel".into()),
                doc_id: p.doc_random.unwrap_or(0),
                bgcolor: format!("#{:06x}", if p.root.background.a() == 0 { 0xFF_FFFF } else { p.root.background.0 & 0xFF_FFFF }),
                css_popups: html_set.css_popups.unwrap_or(true),
                images_dir: images_dir.clone(),
                table_align: html_set.table_alignment.unwrap_or(0),
                now: o.date,
                comments: o.html_prefs.comments,
                empty_cells: crate::html::EmptyCells {
                    color: (!o.html_prefs.empty_cell_canvas).then(|| format!("#{:06x}", o.html_prefs.empty_cell_color & 0xFF_FFFF)),
                    contents: match o.html_prefs.empty_cell_contents {
                        0 => crate::html::EmptyContents::None,
                        2 => crate::html::EmptyContents::Nbsp,
                        _ => crate::html::EmptyContents::Spacer,
                    },
                },
                default_alt: html_set.default_alt.clone().unwrap_or_default(),
                body_css: body_css(&html_set),
                external_css: external_css_file(&html_set, &base),
                ..HtmlOptions::default()
            };
            if o.html_prefs.empty_cell_canvas {
                ho.empty_cells.color = Some(ho.bgcolor.clone());
            }
            ho.spacer = format!("{images_dir}spacer.gif");
            ho.arrow = "arrows.gif".into();
            let measure = o.measure.unwrap_or(default_measure);
            let out = write_html(&grid, &cells, &slices, &areas, frame_count(doc, page), &frame_name, &measure, &ho);
            // Bilder je Zelle (Grundbild) und benötigte Status

            let mut sprite_parts: Vec<(String, RgbaImage)> = Vec::new();
            for (ri, row) in grid.cells.iter().enumerate() {
                for (ci, cell) in row.iter().enumerate() {
                    let cc = &cells[ri][ci];
                    if cell.skip || cc.image.is_none() {
                        continue;
                    }
                    if only.as_ref().is_some_and(|set| !cell.slice().is_some_and(|id| set.contains(&id))) {
                        continue;
                    }
                    if pn.is_some() && o.style == FilesStyle::CssSprites && cell.slice().is_none() {
                        continue;
                    }
                    let (w, h) = grid.span_size(ri, ci);
                    let crect = IRect::new(grid.xs[ci], grid.ys[ri], grid.xs[ci] + w, grid.ys[ri] + h);
                    let frames: Vec<i32> = if one_frame {
                        vec![0]
                    } else if pn.is_some() && matches!(o.style, FilesStyle::ImagesOnly | FilesStyle::CssSprites) {
                        // gemessen: je weiterem Status ein Bild „_s<Nr>“ (re\18 §3)
                        (0..frame_imgs.len() as i32).collect()
                    } else {
                        std::iter::once(0).chain(out.needed_frames.iter().filter(|(sl, _)| *sl == cell.slice() && cell.slice().is_some()).map(|(_, f)| *f)).collect()
                    };
                    let spec = cell.slice().and_then(|id| slices.iter().find(|x| x.id == id));
                    let own = spec.and_then(|x| {
                        doc.pages[page].root.layers.iter().find_map(|slot| match slot {
                            LayerSlot::Own(l) if l.kind == LayerKind::Web => l.cels.first().and_then(|c| {
                                c.elements.iter().find_map(|e| match &e.kind {
                                    ElementKind::WebObject(w) => w.slice.as_ref().filter(|sp| sp.slice_id == Some(x.id)).and_then(|sp| sp.export.as_ref()).map(|e| OptimizeSettings::from_model(e, s.matte)),
                                    _ => None,
                                })
                            }),
                            _ => None,
                        })
                    });
                    let cs = own.unwrap_or_else(|| s.clone());
                    for f in frames {
                        let fi = f.max(0) as usize;
                        if fi >= frame_imgs.len() {
                            continue;
                        }
                        if frame_imgs[fi].is_none() {
                            frame_imgs[fi] = Some(render(r, doc, page, fi, &s, LayerFilter::All)?);
                        }
                        let Some(full) = &frame_imgs[fi] else { continue };
                        let part = full.cropped(crect);
                        let path = if f == 0 {
                            cc.image.clone().unwrap_or_default()
                        } else if pn.is_some() {
                            format!("{images_dir}{}{}.{ext}", cc.name, frame_suffix_with(&ftokens, f as usize + 1, &state_names, letter))
                        } else {
                            frame_name(cell.slice(), f).unwrap_or_default()
                        };
                        if o.style == FilesStyle::CssSprites && pn.is_some() {
                            // Selektor je Segment und Status: Status 2, 3 … ⇒ „_1“, „_2“ (gemessen)
                            let n = if f == 0 { cc.name.clone() } else { format!("{}_{f}", cc.name) };
                            sprite_parts.push((n, part.clone()));
                            continue;
                        }
                        if o.style == FilesStyle::CssSprites && f == 0 {
                            sprite_parts.push((cc.name.clone(), part.clone()));
                            continue;
                        }
                        let mut cs2 = cs.clone();
                        if cs2.format.is_animated() {
                            cs2.format = match cs2.format {
                                ExportFormat::AnimatedGif => ExportFormat::Gif,
                                ExportFormat::Apng => ExportFormat::Png,
                                _ => ExportFormat::WebPLossless,
                            };
                        }
                        files.push(ExportFile { path, bytes: export_image(&part, &cs2, &ctx, ex)?.bytes });
                    }
                }
            }
            // Grafik-Menübilder der Pop-up-Menüs (mmmenu<n>_<B>x<H>_up/_over)
            if matches!(o.style, FilesStyle::HtmlAndImages | FilesStyle::CssAndImages) {
                for (path, bytes) in crate::popup_images::menu_image_files(r, &out.menu_images, &ctx, ex)? {
                    files.push(ExportFile { path, bytes });
                }
            }
            let html_ext = pn.map(|x| x.html_ext.trim_start_matches('.').to_string()).filter(|e| !e.is_empty()).unwrap_or_else(|| html_extension(Some(&html_set), &o.html_prefs));
            match o.style {
                FilesStyle::CssSprites if pn.is_some() => {
                    let so = pn.map(|x| x.sprite.clone()).unwrap_or_default();
                    let preset = pn.and_then(|x| x.sprite_settings.as_ref()).map(still);
                    let sext = preset.as_ref().map(|p| p.format.extension()).unwrap_or("png");
                    let (img, css, modified) = crate::sprites::pack_sprites_with(&sprite_parts, &base, &format!("{base}.{sext}"), &so);
                    notes.selectors_modified |= modified;
                    let bytes = match &preset {
                        Some(ps) => export_image(&img, ps, &ctx, ex)?.bytes,
                        None => {
                            let px = crate::pipeline::image_pixels(&img);
                            crate::png_out::write_png_truecolor(&px, img.width().max(1), img.height().max(1), true, &crate::png_out::PngOptions::from_context(&ctx, false))?
                        }
                    };
                    files.push(ExportFile { path: format!("{base}.{sext}"), bytes });
                    files.push(ExportFile { path: format!("{base}.css"), bytes: css.into_bytes() });
                }
                FilesStyle::HtmlAndImages => {
                    files.push(ExportFile { path: format!("{base}.{html_ext}"), bytes: out.html.into_bytes() });
                    for (n, b) in out.support_files {
                        files.push(ExportFile { path: n, bytes: b });
                    }
                }
                FilesStyle::CssAndImages => {
                    files.push(ExportFile { path: format!("{base}.{html_ext}"), bytes: write_css_layers(&grid, &cells, &ho).into_bytes() });
                }
                FilesStyle::CssSprites => {
                    let (img, css) = crate::sprites::pack_sprites(&sprite_parts, &format!("{images_dir}{base}_sprite.png"));
                    let px = crate::pipeline::image_pixels(&img);
                    let png = crate::png_out::write_png_truecolor(&px, img.width().max(1), img.height().max(1), true, &crate::png_out::PngOptions::from_context(&ctx, false))?;
                    files.push(ExportFile { path: format!("{images_dir}{base}_sprite.png"), bytes: png });
                    files.push(ExportFile { path: format!("{base}.css"), bytes: css.into_bytes() });
                }
                _ => {}
            }
        }
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::SimpleRenderer;
    use fp_model::{Element, Point, Rect};
    use std::sync::Arc;

    fn doc() -> Document {
        let mut d = Document::new(40.0, 20.0, fp_color::Argb(0xFFFFFFFF));
        let mut ids = std::mem::take(&mut d.ids);
        let mut tid = 0;
        let bmp = Element::new_bitmap(&mut ids, Point::new(0.0, 0.0), 20, 20, &[255, 0, 0, 255].repeat(400), &mut tid);
        let sl = Element::new_slice(&mut ids, Rect::new(0.0, 0.0, 20.0, 20.0));
        d.ids = ids;
        let pg = d.page_mut(0).unwrap();
        if let LayerSlot::Own(l) = &mut pg.root.layers[0] {
            Arc::make_mut(&mut Arc::make_mut(l).cels[0]).elements.push(Arc::new(bmp));
        }
        if let LayerSlot::Own(l) = &mut pg.root.layers[1] {
            Arc::make_mut(&mut Arc::make_mut(l).cels[0]).elements.push(Arc::new(sl));
        }
        d
    }

    /// Bildvorschau: dieselbe Byte-Zahl wie der echte Export (re\17 §9 „Größe = echte Kodierung“,
    /// P4.3: gleiche `EncodeExtras`), für jedes Format; Abbruch liefert `None`.
    #[test]
    fn preview_size_equals_real_export_for_every_format() {
        use crate::options::{ColorMode, Dither};
        let d = doc();
        let ex = EncodeExtras::default();
        let mut cases: Vec<OptimizeSettings> = Vec::new();
        for (f, m) in [
            (ExportFormat::Gif, ColorMode::Indexed),
            (ExportFormat::AnimatedGif, ColorMode::Indexed),
            (ExportFormat::Jpeg, ColorMode::Rgb24),
            (ExportFormat::Png, ColorMode::Indexed),
            (ExportFormat::Png, ColorMode::Rgb24),
            (ExportFormat::Png, ColorMode::Rgba32),
            (ExportFormat::Tiff, ColorMode::Rgba32),
            (ExportFormat::Bmp, ColorMode::Rgb24),
            (ExportFormat::Wbmp, ColorMode::Indexed),
            (ExportFormat::WebP, ColorMode::Rgba32),
            (ExportFormat::Apng, ColorMode::Rgba32),
            (ExportFormat::Qoi, ColorMode::Rgba32),
        ] {
            cases.push(OptimizeSettings { format: f, color_mode: m, ..OptimizeSettings::default() });
        }
        let mut jpeg = cases[2].clone();
        jpeg.jpeg.quality = 37;
        jpeg.jpeg.smoothing = 3;
        jpeg.jpeg.subsampling = 1;
        cases.push(jpeg);
        let mut gif = cases[0].clone();
        gif.max_colors = 7;
        gif.dither = Dither::Diffusion(100);
        cases.push(gif);
        for s in &cases {
            let real = export_whole(&SimpleRenderer, &d, 0, s, &ex).unwrap();
            let p = preview_whole(&SimpleRenderer, &d, 0, 0, s, &ex, &|| false).unwrap().expect("nicht abgebrochen");
            assert_eq!(p.bytes, real.len(), "{:?} {:?}", s.format, s.color_mode);
            assert_eq!((p.image.width(), p.image.height()), (40, 20), "{:?}", s.format);
            assert_eq!(p.colors > 0, s.format == ExportFormat::Gif || s.format == ExportFormat::AnimatedGif || s.color_mode == ColorMode::Indexed, "{:?}", s.format);
            assert_eq!(p.colors, p.palette.len());
        }
        // andere Tabellen (IJG-Ersatz) ändern die JPEG-Größe nicht zwingend, die Vorschau nimmt aber
        // genau die übergebenen: dieselben Extras → dieselbe Größe auch mit `standalone()`
        let sa = EncodeExtras::standalone();
        let real = export_whole(&SimpleRenderer, &d, 0, &cases[2], &sa).unwrap();
        assert_eq!(preview_whole(&SimpleRenderer, &d, 0, 0, &cases[2], &sa, &|| false).unwrap().map(|p| p.bytes), Some(real.len()));
        // Abbruch
        assert!(preview_whole(&SimpleRenderer, &d, 0, 0, &cases[0], &ex, &|| true).unwrap().is_none());
        // Status außerhalb: letzter Status statt Panik
        assert!(preview_whole(&SimpleRenderer, &d, 0, 9, &cases[0], &ex, &|| false).unwrap().is_some());
    }

    /// Bildvorschau P2: Indizes, Zähler und Transparenzindex kommen aus demselben Lauf; Pixel →
    /// Paletteneintrag stimmt mit dem angezeigten Bild überein. „Optimieren auf Größe“ meldet ein
    /// unerreichbares Ziel und Formate ohne Suche.
    #[test]
    fn preview_carries_indices_and_size_outcomes() {
        let d = doc();
        let ex = EncodeExtras::default();
        let s = OptimizeSettings::default();
        let p = preview_whole(&SimpleRenderer, &d, 0, 0, &s, &ex, &|| false).unwrap().expect("Ergebnis");
        assert_eq!(p.indices.len(), 40 * 20);
        assert_eq!(p.counts.len(), p.palette.len());
        assert!(p.counts.iter().sum::<u64>() > 0);
        for (i, &ix) in p.indices.iter().enumerate() {
            let c = p.palette[ix as usize];
            let px = p.image.data()[i];
            assert_eq!([(c >> 16) as u8, (c >> 8) as u8, c as u8], [px[0], px[1], px[2]], "Pixel {i}");
        }
        let truecolor = OptimizeSettings { format: ExportFormat::Png, color_mode: crate::options::ColorMode::Rgba32, ..OptimizeSettings::default() };
        let t = preview_whole(&SimpleRenderer, &d, 0, 0, &truecolor, &ex, &|| false).unwrap().expect("Ergebnis");
        assert!(t.indices.is_empty() && t.palette.is_empty() && t.transparent_index.is_none());
        // Indextransparenz mit markierter Farbe: der Eintrag wird transparent und ist der Transparenzindex
        let mut tr = s.clone();
        crate::palette_edit::set_only_transparent(&mut tr, 0xFFFF_0000);
        let pt = preview_whole(&SimpleRenderer, &d, 0, 0, &tr, &ex, &|| false).unwrap().expect("Ergebnis");
        let k = pt.palette.iter().position(|c| c & 0xFF_FFFF == 0xFF_0000).expect("Rot in der Palette");
        assert_eq!(pt.transparent_index, Some(k));
        assert_eq!(pt.image.data()[0][3], 0, "rotes Pixel transparent");
        // Optimieren auf Größe
        assert!(matches!(optimize_whole_to_size(&SimpleRenderer, &d, 0, 0, &s, 1, &ex).unwrap(), SizeOutcome::TooSmall { floor } if floor > 1));
        assert_eq!(optimize_whole_to_size(&SimpleRenderer, &d, 0, 0, &truecolor, 100, &ex).unwrap(), SizeOutcome::NotApplicable);
        match optimize_whole_to_size(&SimpleRenderer, &d, 0, 0, &s, p.bytes, &ex).unwrap() {
            SizeOutcome::Found(r) => assert!(r.changed && r.bytes > 0),
            o => panic!("{o:?}"),
        }
    }

    #[test]
    fn html_and_images_with_one_slice() {
        let d = doc();
        let o = DocExportOptions::new("test");
        let files = export_document(&d, &SimpleRenderer, &o, &EncodeExtras::standalone()).unwrap();
        let names: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
        assert!(names.contains(&"test.htm"), "{names:?}");
        assert!(names.contains(&"test_r1_c1.png"), "neue Seite: PNG 32 (BA-033): {names:?}");
        assert!(names.contains(&"test_r1_c2.png"), "neue Seite: PNG 32 (BA-033): {names:?}");
        // CS6-Vorgabe LSSi = 3: verschachtelte Tabellen ohne Abstandhalter
        let html = String::from_utf8_lossy(&files.iter().find(|f| f.path == "test.htm").unwrap().bytes).into_owned();
        assert!(html.contains("fwnested=\"1\"") && !names.contains(&"spacer.gif"));
        for style in [FilesStyle::ImagesOnly, FilesStyle::CssAndImages, FilesStyle::CssSprites, FilesStyle::LayersToFiles, FilesStyle::FramesToFiles, FilesStyle::PagesToFiles, FilesStyle::Psd, FilesStyle::Pdf, FilesStyle::Svg] {
            let o = DocExportOptions { style, ..DocExportOptions::new("t") };
            let f = export_document(&d, &SimpleRenderer, &o, &EncodeExtras::standalone());
            assert!(f.as_ref().is_ok_and(|v| !v.is_empty()), "{style:?}: {:?}", f.err());
        }
    }

    /// HTML-Export mit CSS-Pop-up-Menü: „CSS in externe Datei schreiben“ (Vorgabe an) legt
    /// `<Basisname>.css` an und importiert sie; aus = Stile im Kopf, keine Datei (TW-03).
    #[test]
    fn external_css_file_in_html_export() {
        let mut d = doc();
        let pg = d.page_mut(0).unwrap();
        if let LayerSlot::Own(l) = &mut pg.root.layers[1] {
            let el = Arc::make_mut(&mut Arc::make_mut(&mut Arc::make_mut(l).cels[0]).elements[0]);
            if let fp_model::ElementKind::WebObject(w) = &mut el.kind {
                let mut b = fp_model::Behavior::new(fp_model::BehaviorAction::PopupMenu, 1);
                b.popup = Some(Box::new(fp_model::PopupMenu::default()));
                w.behaviors = Some(fp_model::Behaviors { items: vec![b], ..Default::default() });
            }
        }
        let run = |d: &Document| export_document(d, &SimpleRenderer, &DocExportOptions::new("test"), &EncodeExtras::standalone()).unwrap();
        let files = run(&d);
        let css = files.iter().find(|f| f.path == "test.css").expect("test.css");
        assert!(String::from_utf8_lossy(&css.bytes).contains("position:absolute"));
        let html = String::from_utf8_lossy(&files.iter().find(|f| f.path == "test.htm").unwrap().bytes).into_owned();
        assert!(html.contains("@import url(\"test.css\");"), "{html}");
        // Option aus: keine Datei, Regeln im Kopf
        let pg = d.page_mut(0).unwrap();
        let e = pg.root.export.get_or_insert_with(|| Box::new(fp_model::ExportSettings::default()));
        e.html.get_or_insert_with(|| Box::new(fp_model::HtmlSettings::cs6_default())).external_css = Some(0);
        let files = run(&d);
        assert!(!files.iter().any(|f| f.path.ends_with(".css")));
        let html = String::from_utf8_lossy(&files.iter().find(|f| f.path == "test.htm").unwrap().bytes).into_owned();
        assert!(!html.contains("@import") && html.contains("position:absolute"), "{html}");
    }

    /// `CBAi`: 0 fixed, 1 scroll (Vorgabe neuer Dokumente 1 = scroll, TW-06).
    #[test]
    fn background_attachment_codes() {
        let mut h = fp_model::HtmlSettings::cs6_default();
        h.background_url = Some("bg.png".into());
        assert!(body_css(&h).contains("background-attachment: scroll;"));
        h.background_attachment = Some(0);
        assert!(body_css(&h).contains("background-attachment: fixed;"));
    }

    #[test]
    fn crop_scale_size_matches_apply_crop_scale() {
        let img = RgbaImage::filled(IRect::new(0, 0, 480, 320), fp_color::Rgba8::new(10, 20, 30, 255)).expect("Bild");
        let mut s = OptimizeSettings::default();
        let sc = |percent, width, height| Some(fp_model::ExportScale { use_scale: true, constrain: true, percent, width, height });
        for (crop, scale, want) in [
            (None, sc(50, 0, 0), (240, 160)),
            (Some([40, 0, 240, 320]), sc(50, 240, 160), (100, 160)),
            (Some([0, 0, 240, 160]), sc(100, 120, 80), (60, 40)),
            (None, sc(100, 120, 80), (120, 80)),
            (Some([400, 300, 600, 400]), None, (80, 20)),
        ] {
            s.crop = crop;
            s.scale = scale;
            assert_eq!(crop_scale_size(480, 320, &s), want, "{crop:?} {scale:?}");
            let out = apply_crop_scale(img.clone(), &s);
            assert_eq!((out.width(), out.height()), want, "{crop:?} {scale:?}");
        }
    }
}
