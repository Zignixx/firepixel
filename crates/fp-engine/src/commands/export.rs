//! Export-Befehle (Menü „Datei“ und „Bearbeiten > HTML-Code kopieren“) über fp-export:
//! Exportieren, Exportbereich, Bildvorschau, Exportassistent, HTML aktualisieren, HTML-Einrichtung,
//! HTML-Code kopieren, selektives JPEG, Vorschau im Browser, Als E-Mail senden, Batch-Verarbeitung,
//! Seite einrichten und Drucken.
//!
//! Gerendert wird mit dem Renderer der Sitzung über [`crate::export_backend::ExportRenderer`].
//! Befehle, die ein Fremdprogramm starten (Browser, E-Mail-Programm, Drucken), tun das nur mit
//! `open: true` (Vorgabe) und wenn die Umgebungsvariable `FP_NO_LAUNCH` nicht gesetzt ist; die
//! erzeugten Dateien und ihr Pfad stehen immer im Ergebnis.

use super::{with_doc, DOC};
use crate::command::{cmd, CommandOutput, CommandRegistry, ParamKind, Params};
use crate::error::{EngineError, Result};
use crate::export_backend::ExportRenderer;
use crate::session::Session;
use fp_export::doc_export::{export_document, export_whole, page_context, page_settings, DocExportOptions, ExportFile, FilesStyle};
use fp_export::export::{export_image, EncodeExtras};
use fp_export::options::{builtin_presets, ColorMode, Dither, ExportFormat, OptimizeSettings, PaletteKind, Transparency};
use fp_export::render::{Background, LayerFilter, PageRenderer as _, RenderRequest};
use fp_model::{Document, ExportSettings};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;

// ------------------------------------------------------------------ Einstellungen als JSON

/// Name eines Formats für Parameter/Ergebnisse.
pub fn format_name(s: &OptimizeSettings) -> &'static str {
    match (s.format, s.color_mode) {
        (ExportFormat::Gif, _) => "gif",
        (ExportFormat::AnimatedGif, _) => "animatedGif",
        (ExportFormat::Jpeg, _) => "jpeg",
        (ExportFormat::Png, ColorMode::Indexed) => "png8",
        (ExportFormat::Png, ColorMode::Rgb24) => "png24",
        (ExportFormat::Png, ColorMode::Rgba32) => "png32",
        (ExportFormat::Tiff, _) => "tiff",
        (ExportFormat::Bmp, _) => "bmp",
        (ExportFormat::Wbmp, _) => "wbmp",
        (ExportFormat::Pict, _) => "pict",
        (ExportFormat::WebP, _) => "webp",
        (ExportFormat::WebPLossless, _) => "webpLossless",
        (ExportFormat::AnimatedWebP, _) => "animatedWebp",
        (ExportFormat::Avif, _) => "avif",
        (ExportFormat::Apng, _) => "apng",
        (ExportFormat::Qoi, _) => "qoi",
        (ExportFormat::Ico, _) => "ico",
        (ExportFormat::Custom(_), _) => "custom",
    }
}

/// Formatnamen für das Parameterschema.
pub const FORMAT_NAMES: &[&str] = &[
    "gif", "animatedGif", "jpeg", "png8", "png24", "png32", "tiff", "bmp", "wbmp", "webp", "webpLossless", "animatedWebp", "avif", "apng", "qoi", "ico",
];

fn bad(name: &str, msg: impl Into<String>) -> EngineError {
    EngineError::InvalidParams { name: name.into(), message: msg.into() }
}

/// Wendet JSON-Angaben (`preset`, `format`, `colors`, `quality`, `dither`, `transparency`, `matte`,
/// `loop`, `progressive`, `smoothing`, `lossy`, `interlaced`, `palette`, `webSnap`, `crop`, `scale`)
/// auf Einstellungen an.
pub fn apply_settings_json(mut s: OptimizeSettings, v: &Value, cmd_name: &str) -> Result<OptimizeSettings> {
    let Some(o) = v.as_object() else {
        if v.is_null() {
            return Ok(s);
        }
        return Err(bad(cmd_name, fp_i18n::t("err-param-settings")));
    };
    if let Some(p) = o.get("preset").and_then(|x| x.as_str()) {
        s = builtin_presets().into_iter().find(|x| x.name.eq_ignore_ascii_case(p)).map(|x| x.settings).ok_or_else(|| bad(cmd_name, fp_i18n::tr!("err-unknown-preset", name = p)))?;
    }
    if let Some(f) = o.get("format").and_then(|x| x.as_str()) {
        let (fmt, cm) = match f.to_ascii_lowercase().as_str() {
            "gif" => (ExportFormat::Gif, ColorMode::Indexed),
            "animatedgif" => (ExportFormat::AnimatedGif, ColorMode::Indexed),
            "jpeg" | "jpg" => (ExportFormat::Jpeg, ColorMode::Rgb24),
            "png8" => (ExportFormat::Png, ColorMode::Indexed),
            "png24" => (ExportFormat::Png, ColorMode::Rgb24),
            "png" | "png32" => (ExportFormat::Png, ColorMode::Rgba32),
            "tiff" | "tif" => (ExportFormat::Tiff, ColorMode::Rgba32),
            "bmp" => (ExportFormat::Bmp, ColorMode::Rgb24),
            "wbmp" => (ExportFormat::Wbmp, ColorMode::Indexed),
            "webp" => (ExportFormat::WebP, ColorMode::Rgba32),
            "webplossless" => (ExportFormat::WebPLossless, ColorMode::Rgba32),
            "animatedwebp" => (ExportFormat::AnimatedWebP, ColorMode::Rgba32),
            "avif" => (ExportFormat::Avif, ColorMode::Rgba32),
            "apng" => (ExportFormat::Apng, ColorMode::Rgba32),
            "qoi" => (ExportFormat::Qoi, ColorMode::Rgba32),
            "ico" => (ExportFormat::Ico, ColorMode::Rgba32),
            _ => return Err(bad(cmd_name, fp_i18n::tr!("err-unknown-format", name = f))),
        };
        s.format = fmt;
        s.color_mode = cm;
        if fmt == ExportFormat::WebPLossless {
            s.webp.lossless = true;
        }
    }
    let int = |k: &str| o.get(k).and_then(|x| x.as_f64()).map(|x| x.round() as i32);
    if let Some(c) = int("colors") {
        s.max_colors = c.clamp(2, 256);
    }
    if let Some(q) = int("quality") {
        let q = q.clamp(1, 100);
        s.jpeg.quality = q;
        s.webp.quality = q;
        s.avif.quality = q;
    }
    if let Some(v) = int("smoothing") {
        s.jpeg.smoothing = v.clamp(0, 8);
    }
    if let Some(v) = int("lossy") {
        s.lossy = v.clamp(0, 100);
    }
    if let Some(v) = int("loop") {
        s.anim.loop_count = v;
    }
    if let Some(b) = o.get("progressive").and_then(|x| x.as_bool()) {
        s.jpeg.progressive = b;
    }
    if let Some(b) = o.get("interlaced").and_then(|x| x.as_bool()) {
        s.interlaced = b;
    }
    if let Some(b) = o.get("webSnap").and_then(|x| x.as_bool()) {
        s.web_adaptive = b;
    }
    if let Some(d) = o.get("dither") {
        s.dither = match d {
            Value::Bool(false) | Value::Null => Dither::None,
            Value::Bool(true) => Dither::Diffusion(100),
            Value::Number(n) => {
                let p = n.as_f64().unwrap_or(0.0).round() as i32;
                if p <= 0 { Dither::None } else { Dither::Diffusion(p.min(100)) }
            }
            Value::String(t) if t == "2x2" => Dither::Pattern2x2,
            _ => return Err(bad(cmd_name, fp_i18n::t("err-param-dither"))),
        };
    }
    if let Some(t) = o.get("transparency").and_then(|x| x.as_str()) {
        s.transparency = match t {
            "none" => Transparency::None,
            "index" => Transparency::Index,
            "alpha" => Transparency::IndexAlpha,
            "rgba" => Transparency::Rgba,
            _ => return Err(bad(cmd_name, fp_i18n::t("err-param-transparency"))),
        };
    }
    if let Some(p) = o.get("palette").and_then(|x| x.as_str()) {
        s.palette_kind = match p {
            "adaptive" => PaletteKind::Adaptive,
            "webAdaptive" => {
                s.web_adaptive = true;
                PaletteKind::Adaptive
            }
            "web216" => PaletteKind::Web216,
            "exact" => PaletteKind::Exact,
            "uniform" => PaletteKind::Uniform,
            "grayscale" => PaletteKind::Grayscale,
            "blackWhite" => {
                s.max_colors = 2;
                PaletteKind::Grayscale
            }
            "mac" => PaletteKind::Mac,
            "windows" => PaletteKind::Windows,
            _ => return Err(bad(cmd_name, fp_i18n::tr!("err-unknown-palette", name = p))),
        };
    }
    match o.get("matte") {
        Some(Value::String(m)) if m == "none" => s.matte = None,
        Some(Value::String(m)) => s.matte = Some(fp_color::Rgba8::parse_hex(m).map(|c| c.to_argb().0).ok_or_else(|| bad(cmd_name, fp_i18n::t("err-param-matte")))?),
        _ => {}
    }
    if let Some(c) = o.get("crop") {
        s.crop = match c {
            Value::Null => None,
            v => {
                let r = crate::command::parse_rect(v).ok_or_else(|| bad(cmd_name, fp_i18n::t("err-param-crop")))?;
                Some([r.left.round() as i32, r.top.round() as i32, r.right.round() as i32, r.bottom.round() as i32])
            }
        };
    }
    if let Some(p) = int("scale") {
        s.scale = Some(fp_model::ExportScale { use_scale: true, constrain: true, percent: p.clamp(1, 10000), width: 0, height: 0 });
    }
    if let (Some(w), Some(h)) = (int("width"), int("height")) {
        s.scale = Some(fp_model::ExportScale { use_scale: true, constrain: false, percent: 100, width: w.max(1), height: h.max(1) });
    }
    Ok(s)
}

/// Einstellungen als JSON (Gegenstück zu [`apply_settings_json`]).
pub fn settings_json(s: &OptimizeSettings) -> Value {
    json!({
        "format": format_name(s),
        "colors": s.max_colors,
        "quality": match s.format { ExportFormat::WebP | ExportFormat::WebPLossless | ExportFormat::AnimatedWebP => s.webp.quality, ExportFormat::Avif => s.avif.quality, _ => s.jpeg.quality },
        "smoothing": s.jpeg.smoothing,
        "progressive": s.jpeg.progressive,
        "lossy": s.lossy,
        "interlaced": s.interlaced,
        "webSnap": s.web_adaptive,
        "dither": match s.dither { Dither::None => json!(0), Dither::Diffusion(p) => json!(p), Dither::Pattern2x2 => json!("2x2") },
        "transparency": match s.transparency { Transparency::None => "none", Transparency::Index => "index", Transparency::IndexAlpha => "alpha", Transparency::Rgba => "rgba" },
        "matte": s.matte.map(|m| super::argb_hex(fp_color::Argb(m))),
        "loop": s.anim.loop_count,
        "selectiveJpeg": {"enabled": s.jpeg.selective.enabled, "quality": s.jpeg.selective.quality, "preserveText": s.jpeg.selective.preserve_text, "preserveButtons": s.jpeg.selective.preserve_buttons},
        "crop": s.crop,
    })
}

/// Speichert Optimieren-Einstellungen in die Seite (auch in die alte Einstellung, falls vorhanden,
/// weil `page_settings` diese zuerst liest).
pub fn store_page_settings(doc: &mut Document, page: usize, s: &OptimizeSettings) -> Result<()> {
    let p = doc.page_mut(page).ok_or_else(|| EngineError::NotFound(fp_i18n::tr!("err-nf-page-n", n = (page + 1).to_string())))?;
    let base: ExportSettings = p.root.export.as_deref().cloned().unwrap_or_default();
    p.root.export = Some(Box::new(s.to_model(&base)));
    if let Some(l) = p.legacy_export.as_mut() {
        **l = s.to_model(l);
    }
    Ok(())
}

/// Einstellungen eines Aufrufs: Seiteneinstellungen + `settings`-Parameter.
fn settings_of(doc: &Document, page: usize, p: &Params) -> Result<OptimizeSettings> {
    let base = page_settings(doc, page);
    match p.get("settings") {
        Some(v) => apply_settings_json(base, v, &p.command),
        None => Ok(base),
    }
}

// ------------------------------------------------------------------ Dateien

fn write_files(dir: &Path, files: &[ExportFile]) -> Result<Vec<String>> {
    let mut out = Vec::new();
    for f in files {
        let path = dir.join(&f.path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, &f.bytes)?;
        out.push(path.to_string_lossy().to_string());
    }
    Ok(out)
}

/// Ordner für temporäre Ausgaben (Vorschau, E-Mail, Druck).
pub fn temp_dir(kind: &str) -> PathBuf {
    let d = std::env::temp_dir().join("Firepixel").join(kind);
    let _ = std::fs::create_dir_all(&d);
    d
}

fn launch_allowed(p: &Params) -> Result<bool> {
    Ok(p.bool_or("open", true)? && crate::shell::allowed())
}

/// Öffnet eine Datei mit dem zugeordneten Programm (bzw. mit `program`) über
/// [`crate::shell::shell_open`] (abschaltbar mit `FP_NO_SHELL_OPEN`).
pub fn open_with_system(path: &Path, program: Option<&str>) -> Result<()> {
    crate::shell::shell_open(path, program).map(|_| ())
}

fn base_name_of(s: &Session) -> Result<String> {
    let d = s.doc()?;
    let t = d.path.as_ref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| d.title.clone());
    let t = t.strip_suffix(".png").unwrap_or(&t).to_string();
    Ok(t.strip_suffix(".fw").unwrap_or(&t).to_string())
}

fn style_of(name: &str) -> Option<FilesStyle> {
    Some(match name {
        "htmlAndImages" | "html" => FilesStyle::HtmlAndImages,
        "imagesOnly" | "images" => FilesStyle::ImagesOnly,
        "cssAndImages" | "css" => FilesStyle::CssAndImages,
        "cssSprites" | "sprites" => FilesStyle::CssSprites,
        "layersToFiles" | "layers" => FilesStyle::LayersToFiles,
        "framesToFiles" | "statesToFiles" | "states" => FilesStyle::FramesToFiles,
        "pagesToFiles" | "pages" => FilesStyle::PagesToFiles,
        "psd" => FilesStyle::Psd,
        "pdf" => FilesStyle::Pdf,
        "svg" => FilesStyle::Svg,
        _ => return None,
    })
}

const STYLE_NAMES: &[&str] = &["imagesOnly", "htmlAndImages", "cssAndImages", "cssSprites", "layersToFiles", "framesToFiles", "pagesToFiles", "psd", "pdf", "svg"];

fn today() -> fp_export::popup::MenuDate {
    // Tage seit 1970 → Datum (bürgerlicher Kalender, Algorithmus von H. Hinnant)
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) as i64;
    let z = secs.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    fp_export::popup::MenuDate { year: y as i32, month: m as u32, day: d as u32, ..Default::default() }
}

/// Programmweite Werte der HTML-Einrichtung aus den Voreinstellungen (re\18 §6.5). Die Liste
/// „Inhalt:“ leerer Zellen steht 1-basiert in `HTMLEmptyCellContents` (Vorgabe 2 = Platzhalter-Grafik).
pub fn html_prefs(prefs: &crate::prefs::Preferences) -> fp_export::doc_export::HtmlPrefs {
    fp_export::doc_export::HtmlPrefs {
        extension: prefs.get_str("HTMLExtension", ".htm"),
        comments: prefs.get_bool("HTMLCommentsOn", false),
        lowercase: prefs.get_bool("ForceLowercaseFilenames", false),
        empty_cell_color: prefs.get("HTMLEmptyCellColor").and_then(|v| v.as_color()).map(|c| c.0 & 0xFF_FFFF).unwrap_or(0xFF_FFFF),
        empty_cell_canvas: prefs.get_bool("HTMLEmptyCellUsesCanvasColor", true),
        empty_cell_contents: (prefs.get_int("HTMLEmptyCellContents", 2) - 1).clamp(0, 2) as i32,
    }
}

/// HTML-Endung (ohne Punkt) des aktiven Dokuments: Dokument, sonst Voreinstellung (re\18 §6.5).
pub fn html_ext_of(s: &Session) -> Result<String> {
    let d = s.doc()?;
    let h = d.doc.pages.get(d.page).and_then(|p| p.root.export.as_ref()).and_then(|e| e.html.as_deref());
    Ok(fp_export::doc_export::html_extension(h, &html_prefs(&s.prefs)))
}

/// Ersetzt in einer vorhandenen HTML-Seite die von Firepixel geschriebene Tabelle (erkannt am
/// Kommentar `<!-- fwtable …`) durch die neue und ergänzt fehlende Skripte im Kopf; der übrige
/// Inhalt bleibt. Ohne solche Tabelle wird sie vor `</body>` bzw. `</html>` eingefügt; `None`,
/// wenn die Datei auch das nicht enthält (keine HTML-Seite).
pub fn replace_html_table(old: &str, new: &str) -> Option<String> {
    fn table_span(html: &str) -> Option<(usize, usize)> {
        let lower = html.to_ascii_lowercase();
        let mark = lower.find("<!-- fwtable")?;
        let start = lower[..mark].rfind("<table")?;
        // passendes </table> (verschachtelte Tabellen mitzählen)
        let mut depth = 0usize;
        let mut i = start;
        while i < lower.len() {
            let open = lower[i..].find("<table").map(|k| i + k);
            let close = lower[i..].find("</table>").map(|k| i + k)?;
            match open {
                Some(o) if o < close => {
                    depth += 1;
                    i = o + 6;
                }
                _ => {
                    depth = depth.saturating_sub(1);
                    i = close + 8;
                    if depth == 0 {
                        return Some((start, i));
                    }
                }
            }
        }
        None
    }
    let (ns, ne) = table_span(new)?;
    let mut out = match table_span(old) {
        Some((os, oe)) => format!("{}{}{}", &old[..os], &new[ns..ne], &old[oe..]),
        None => {
            // noch keine Firepixel-Tabelle: am Ende des Body (bzw. vor </html>) einfügen
            let lower = old.to_ascii_lowercase();
            let at = lower.rfind("</body>").or_else(|| lower.rfind("</html>"))?;
            format!("{}{}\n{}", &old[..at], &new[ns..ne], &old[at..])
        }
    };
    // Skripte des Kopfes (Rollover, Pop-up-Menüs) übernehmen, wenn die Seite noch keine hat
    let lower_new = new.to_ascii_lowercase();
    let scripts = lower_new.find("</head>").and_then(|h| Some((lower_new[..h].find("<script")?, lower_new[..h].rfind("</script>")? + 9)));
    if let Some((a, e)) = scripts.filter(|(a, e)| a < e)
        && !out.to_ascii_lowercase().contains("<script")
        && let Some(h) = out.to_ascii_lowercase().find("</head>")
    {
        out.insert_str(h, &format!("{}\n", &new[a..e]));
    }
    Some(out)
}

/// Exportiert das aktive Dokument (Grundlage von „Exportieren“, „HTML aktualisieren“, Vorschau …).
pub fn export_active(s: &Session, style: FilesStyle, base: &str, page: usize, settings: Option<OptimizeSettings>, use_slices: bool) -> Result<Vec<ExportFile>> {
    let d = s.doc()?;
    let r = ExportRenderer::new(s.renderer());
    let mut o = DocExportOptions::new(base);
    o.page = page;
    o.style = style;
    o.use_slices = use_slices;
    o.settings = settings;
    o.date = today();
    o.src_file = d.path.as_ref().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    o.html_prefs = html_prefs(&s.prefs);
    export_document(&d.doc, &r, &o, &EncodeExtras::default()).map_err(|e| EngineError::Failed(e.to_string()))
}

fn page_param(s: &Session, p: &Params) -> Result<usize> {
    let d = s.doc()?;
    let page = match p.opt_i64("page")? {
        Some(i) => usize::try_from(i).map_err(|_| bad(&p.command, "page"))?,
        None => d.page,
    };
    if page >= d.doc.pages.len() {
        return Err(EngineError::NotFound(fp_i18n::tr!("err-nf-page-n", n = (page + 1).to_string())));
    }
    Ok(page)
}

fn has_slices(doc: &Document, page: usize) -> bool {
    !fp_export::slices::collect_slices(doc, page).is_empty()
}

/// Zählt die Farben eines Bildes (bis `limit + 1`) und prüft auf Transparenz.
fn color_stats(img: &fp_raster::RgbaImage, limit: usize) -> (usize, bool) {
    let mut set = std::collections::HashSet::new();
    let mut alpha = false;
    for px in img.data() {
        if px[3] < 255 {
            alpha = true;
        }
        if set.len() <= limit {
            set.insert(*px);
        }
    }
    (set.len(), alpha)
}

fn render_page_image(s: &Session, page: usize, frame: usize, transparent: bool) -> Result<fp_raster::RgbaImage> {
    let d = s.doc()?;
    let r = ExportRenderer::new(s.renderer());
    r.render(&d.doc, &RenderRequest { page, frame, layers: LayerFilter::All, background: if transparent { Background::Transparent } else { Background::Canvas }, area: None })
        .map_err(EngineError::Failed)
}

/// Erstellungsdatum des Dokuments für `Creation Time` des reduzierten PNG: der Text aus der
/// geöffneten Datei, sonst heute (neu angelegtes Dokument).
pub fn creation_time_of(doc: &Document) -> String {
    doc.file_meta
        .text_chunks
        .iter()
        .find(|(k, v)| k == "Creation Time" && !v.trim().is_empty())
        .map(|(_, v)| v.clone())
        .unwrap_or_else(fp_export::flat_png::creation_time_today)
}

/// „Reduziertes PNG“ (re\20 §5.3): aktuelle Seite im aktuellen Status, gerendert wie beim Export,
/// mit den Optimierungseinstellungen der Seite, Format immer PNG.
pub(super) fn flattened_png_bytes(s: &Session) -> Result<Vec<u8>> {
    let d = s.doc()?;
    let (page, frame) = (d.page, d.frame);
    let doc = Arc::clone(&d.doc);
    let settings = page_settings(&doc, page);
    let flat = fp_export::flat_png::flattened_png_settings(&settings);
    let img = render_page_image(s, page, frame, flat.matte.is_some())?;
    let ctx = page_context(&doc, page);
    fp_export::flat_png::write_flattened_png(&img, &settings, &ctx, &creation_time_of(&doc)).map_err(|e| EngineError::Failed(e.to_string()))
}

// ------------------------------------------------------------------ Druck

/// Seiteneinrichtung für „Drucken“ (das Original speichert sie als `printer_data` der Seite; Firepixel
/// hält sie in der Sitzung).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageSetup {
    /// Hochformat (`portrait`) oder Querformat (`landscape`).
    pub orientation: String,
    /// Skalierung in Prozent (bei `fit = false`).
    pub scale: f64,
    /// Auf Seite einpassen.
    pub fit: bool,
    /// Papiergröße in mm (Breite, Höhe; Vorgabe A4).
    pub paper_mm: [f64; 2],
    /// Ränder in mm (links, oben, rechts, unten).
    pub margins_mm: [f64; 4],
    /// Druckauflösung in dpi.
    pub dpi: f64,
    /// Druckername (leer = Standarddrucker).
    pub printer: String,
}

impl Default for PageSetup {
    fn default() -> Self {
        PageSetup { orientation: "portrait".into(), scale: 100.0, fit: false, paper_mm: [210.0, 297.0], margins_mm: [10.0; 4], dpi: 300.0, printer: String::new() }
    }
}

/// Erzeugt das Druckbild: Papier weiß, Seite nach Einrichtung skaliert und mittig in den Rändern.
pub fn print_image(page_img: &fp_raster::RgbaImage, doc_dpi: f64, ps: &PageSetup) -> fp_raster::RgbaImage {
    let land = ps.orientation == "landscape";
    let (pw, ph) = if land { (ps.paper_mm[1], ps.paper_mm[0]) } else { (ps.paper_mm[0], ps.paper_mm[1]) };
    let px = |mm: f64| (mm / 25.4 * ps.dpi.clamp(36.0, 1200.0)).round().max(1.0);
    let (w, h) = (px(pw) as i32, px(ph) as i32);
    let (ml, mt, mr, mb) = (px(ps.margins_mm[0]), px(ps.margins_mm[1]), px(ps.margins_mm[2]), px(ps.margins_mm[3]));
    let (aw, ah) = ((w as f64 - ml - mr).max(1.0), (h as f64 - mt - mb).max(1.0));
    let (iw, ih) = (page_img.width().max(1) as f64, page_img.height().max(1) as f64);
    let k = if ps.fit { (aw / iw).min(ah / ih) } else { ps.scale.max(1.0) / 100.0 * ps.dpi / doc_dpi.max(1.0) };
    let (tw, th) = ((iw * k).round().clamp(1.0, 20000.0) as u32, (ih * k).round().clamp(1.0, 20000.0) as u32);
    let scaled = fp_raster::resample::resize(page_img, tw, th, fp_raster::Sampler::Bicubic);
    let mut paper = fp_raster::RgbaImage::filled(fp_geom::IRect::new(0, 0, w, h), fp_color::Rgba8::new(255, 255, 255, 255)).unwrap_or_else(|_| fp_raster::RgbaImage::empty_at(0, 0));
    let x = (ml + (aw - tw as f64) / 2.0).round() as i32;
    let y = (mt + (ah - th as f64) / 2.0).round() as i32;
    let placed = scaled.translated(x - scaled.bounds().left, y - scaled.bounds().top);
    fp_raster::composite(&mut paper, &placed, None, &fp_raster::CompositeOptions::default());
    paper
}

fn png_bytes(img: &fp_raster::RgbaImage) -> Result<Vec<u8>> {
    let mut s = OptimizeSettings { format: ExportFormat::Png, color_mode: ColorMode::Rgba32, ..OptimizeSettings::default() };
    s.matte = None;
    Ok(export_image(img, &s, &Default::default(), &EncodeExtras::standalone()).map_err(|e| EngineError::Failed(e.to_string()))?.bytes)
}

/// Startet den Windows-Druck einer Bilddatei (Paint `/pt`, sonst das Druckverb der Shell).
fn system_print(path: &Path, printer: &str) -> Result<&'static str> {
    #[cfg(windows)]
    {
        let windir = std::env::var_os("WINDIR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
        let paint = windir.join("System32").join("mspaint.exe");
        if paint.exists() {
            let mut c = std::process::Command::new(paint);
            c.arg("/pt").arg(path);
            if !printer.is_empty() {
                c.arg(printer);
            }
            return Ok(if crate::shell::spawn(c)? { "mspaint" } else { "abgeschaltet" });
        }
        let script = format!("Start-Process -FilePath '{}' -Verb Print", path.to_string_lossy().replace('\'', "''"));
        let mut c = std::process::Command::new("powershell.exe");
        c.args(["-NoProfile", "-NonInteractive", "-Command", &script]);
        Ok(if crate::shell::spawn(c)? { "shell" } else { "abgeschaltet" })
    }
    #[cfg(not(windows))]
    {
        let mut c = std::process::Command::new("lp");
        if !printer.is_empty() {
            c.args(["-d", printer]);
        }
        c.arg(path);
        Ok(if crate::shell::spawn(c)? { "lp" } else { "abgeschaltet" })
    }
}

// ------------------------------------------------------------------ E-Mail

fn base64(data: &[u8]) -> String {
    let b = fp_export::vector_out::base64(data);
    let mut out = String::with_capacity(b.len() + b.len() / 76 * 2);
    for (i, ch) in b.chars().enumerate() {
        if i > 0 && i % 76 == 0 {
            out.push_str("\r\n");
        }
        out.push(ch);
    }
    out
}

/// Baut eine ungesendete E-Mail (`.eml`, `X-Unsent: 1` – Outlook/Mail öffnen sie als Entwurf) mit Anhang.
pub fn build_eml(to: &str, subject: &str, body: &str, file_name: &str, mime: &str, data: &[u8]) -> String {
    let boundary = "----=_Firepixel_Grenze_0001";
    format!(
        "X-Unsent: 1\r\nTo: {to}\r\nSubject: {subject}\r\nMIME-Version: 1.0\r\nContent-Type: multipart/mixed; boundary=\"{boundary}\"\r\n\r\n--{boundary}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Transfer-Encoding: 8bit\r\n\r\n{body}\r\n\r\n--{boundary}\r\nContent-Type: {mime}; name=\"{file_name}\"\r\nContent-Transfer-Encoding: base64\r\nContent-Disposition: attachment; filename=\"{file_name}\"\r\n\r\n{}\r\n--{boundary}--\r\n",
        base64(data)
    )
}

fn mime_of(ext: &str) -> &'static str {
    match ext {
        "gif" => "image/gif",
        "jpg" => "image/jpeg",
        "png" => "image/png",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "bmp" => "image/bmp",
        "tif" => "image/tiff",
        "ico" => "image/x-icon",
        _ => "application/octet-stream",
    }
}

// ------------------------------------------------------------------ Registrierung

pub(super) fn register(r: &mut CommandRegistry) {
    let sp = || ParamKind::Json;
    r.add(
        cmd("file.export")
            .requires(DOC)
            .param("path", ParamKind::Path, "Zielordner oder Zieldatei (mit Endung: Basisname und Ordner daraus)")
            .opt_default("style", ParamKind::Enum { values: STYLE_NAMES.iter().map(|s| s.to_string()).collect() }, json!("imagesOnly"), "Exportart (exportFilesStyle)")
            .opt("settings", sp(), "Optimieren-Einstellungen {format, quality, colors, preset, matte …}; Vorgabe die der Seite")
            .opt("page", ParamKind::Integer { min: Some(0), max: None }, "Seite (Vorgabe aktuelle)")
            .opt("useSlices", ParamKind::Bool, "Segmente verwenden (Vorgabe: wenn vorhanden)")
            .opt("baseName", ParamKind::String, "Basisname der Dateien")
            .js("exportDocumentAs")
            .run(|s, p| {
                let path = PathBuf::from(p.str("path")?);
                let style = match p.opt_str("style")? {
                    Some(n) => style_of(&n).ok_or_else(|| bad("file.export", fp_i18n::tr!("err-unknown-export-kind", name = n.as_str())))?,
                    None => FilesStyle::ImagesOnly,
                };
                let page = page_param(s, &p.clone())?;
                let doc = Arc::clone(&s.doc()?.doc);
                let settings = settings_of(&doc, page, p)?;
                let (dir, base) = if path.extension().is_some() && !path.is_dir() {
                    let stem = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                    let stem = stem.split('.').next().unwrap_or("export").to_string();
                    (path.parent().map(Path::to_path_buf).unwrap_or_default(), stem)
                } else {
                    (path.clone(), base_name_of(s)?)
                };
                let base = p.opt_str("baseName")?.unwrap_or(base);
                // Zieldatei mit Endung = ein Bild; Segmente nur beim Export in einen Ordner (oder ausdrücklich)
                let single = path.extension().is_some() && !path.is_dir();
                let use_slices = p.opt_bool("useSlices")?.unwrap_or_else(|| !single && has_slices(&doc, page));
                let mut files = export_active(s, style, &base, page, Some(settings), use_slices)?;
                // Einzeldatei mit eigener Endung: Name übernehmen
                if files.len() == 1 && path.extension().is_some() && !path.is_dir() && matches!(style, FilesStyle::ImagesOnly | FilesStyle::Psd | FilesStyle::Pdf | FilesStyle::Svg) {
                    files[0].path = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| files[0].path.clone());
                }
                let written = write_files(&dir, &files)?;
                Ok(CommandOutput::value(json!({"files": written, "count": written.len()})))
            }),
    );
    r.add(
        cmd("file.exportArea")
            .label("Exportbereich exportieren")
            .requires(DOC)
            .param("rect", ParamKind::Rect, "Bereich in Dokumentpixeln")
            .param("path", ParamKind::Path, "Zieldatei")
            .opt("settings", sp(), "Optimieren-Einstellungen")
            .run(|s, p| {
                let rect = p.rect("rect")?;
                let ir = fp_geom::IRect::new(rect.left.floor() as i32, rect.top.floor() as i32, rect.right.ceil() as i32, rect.bottom.ceil() as i32);
                if ir.is_empty() {
                    return Err(bad("file.exportArea", fp_i18n::t("err-empty-area")));
                }
                let d = s.doc()?;
                let page = d.page;
                let frame = d.frame;
                let settings = settings_of(&d.doc, page, p)?;
                let r = ExportRenderer::new(s.renderer());
                let bg = if settings.matte.is_some() { Background::Transparent } else { Background::Canvas };
                let img = r.render(&d.doc, &RenderRequest { page, frame, layers: LayerFilter::All, background: bg, area: Some(ir) }).map_err(EngineError::Failed)?;
                let ctx = page_context(&d.doc, page);
                let bytes = export_image(&img, &settings, &ctx, &EncodeExtras::default()).map_err(|e| EngineError::Failed(e.to_string()))?.bytes;
                let path = PathBuf::from(p.str("path")?);
                if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
                    std::fs::create_dir_all(dir)?;
                }
                std::fs::write(&path, &bytes)?;
                Ok(CommandOutput::value(json!({"path": path.to_string_lossy(), "bytes": bytes.len(), "width": img.width(), "height": img.height()})))
            }),
    );
    r.add(
        cmd("file.imagePreview")
            .requires(DOC)
            .opt("settings", sp(), "Optimieren-Einstellungen (Vorgabe die der Seite)")
            .opt_default("apply", ParamKind::Bool, json!(false), "Einstellungen in die Seite übernehmen (Verlaufsschritt)")
            .opt("path", ParamKind::Path, "Bild zusätzlich exportieren")
            .opt("preview", ParamKind::Path, "Vorschaubild (wie es nach dem Export aussieht) als PNG schreiben")
            .opt("targetSize", ParamKind::Integer { min: Some(1), max: None }, "Auf Größe optimieren (Bytes)")
            .js("exportDocumentAs")
            .run(|s, p| {
                let d = s.doc()?;
                let page = d.page;
                let doc = Arc::clone(&d.doc);
                let mut settings = settings_of(&doc, page, p)?;
                let img = render_page_image(s, page, 0, settings.matte.is_some())?;
                let ctx = page_context(&doc, page);
                let mut fitted = None;
                if let Some(t) = p.opt_i64("targetSize")? {
                    let res = fp_export::size::optimize_to_size(&img, &settings, t.max(1) as usize, &ctx, &EncodeExtras::default()).map_err(|e| EngineError::Failed(e.to_string()))?;
                    fitted = Some(res.bytes);
                    settings = res.settings;
                }
                let bytes = if settings.format.is_animated() {
                    export_whole(&ExportRenderer::new(s.renderer()), &doc, page, &settings, &EncodeExtras::default()).map_err(|e| EngineError::Failed(e.to_string()))?
                } else {
                    let ex = export_image(&img, &settings, &ctx, &EncodeExtras::default()).map_err(|e| EngineError::Failed(e.to_string()))?;
                    if let Some(pv) = p.opt_str("preview")? {
                        let pimg = fp_export::export::preview(&ex, &settings, (img.bounds().left, img.bounds().top));
                        std::fs::write(pv, png_bytes(&pimg)?)?;
                    }
                    ex.bytes
                };
                let colors = if settings.color_mode == ColorMode::Indexed || matches!(settings.format, ExportFormat::Gif | ExportFormat::AnimatedGif) { settings.max_colors.max(0) as usize } else { 0 };
                let info = fp_export::preview::PreviewInfo::new(settings.format.display_name(), bytes.len(), colors, 0);
                let times: Vec<Value> = fp_export::preview::CONNECTION_SPEEDS.iter().map(|(n, bps)| json!({"connection": n, "seconds": fp_export::preview::download_seconds(bytes.len(), *bps)})).collect();
                if let Some(path) = p.opt_str("path")? {
                    std::fs::write(&path, &bytes)?;
                }
                let value = json!({"settings": settings_json(&settings), "bytes": bytes.len(), "size": fp_export::preview::format_size(bytes.len()), "text": info.text(), "download": times, "fittedBytes": fitted});
                if p.bool_or("apply", false)? {
                    with_doc(s, |d| store_page_settings(d.doc_mut(), page, &settings))?;
                    return Ok(CommandOutput::doc_with(value).labeled(fp_i18n::t("msg-step-export-settings")));
                }
                Ok(CommandOutput::value(value))
            }),
    );
    r.add(
        cmd("file.exportWizard")
            .requires(DOC)
            .opt_default("destination", ParamKind::Enum { values: vec!["web".into(), "image".into(), "dreamweaver".into()] }, json!("web"), "Ziel (Web, Bildbearbeitung, Dreamweaver)")
            .opt("targetSize", ParamKind::Integer { min: Some(1), max: None }, "Zielgröße in Bytes (Auf Größe optimieren)")
            .opt_default("apply", ParamKind::Bool, json!(true), "Empfehlung in die Seite übernehmen")
            .opt("path", ParamKind::Path, "gleich exportieren (Zieldatei)")
            .run(|s, p| {
                let d = s.doc()?;
                let page = d.page;
                let doc = Arc::clone(&d.doc);
                let img = render_page_image(s, page, 0, false)?;
                let timg = render_page_image(s, page, 0, true)?;
                let (colors, _) = color_stats(&img, 256);
                let transparent_canvas = doc.pages[page].root.background.a() < 255;
                let (_, alpha) = color_stats(&timg, 0);
                let frames = doc.pages[page].root.frame_count.max(1);
                let mut st = page_settings(&doc, page);
                let dest = p.opt_str("destination")?.unwrap_or_else(|| "web".into());
                let reason;
                if frames > 1 {
                    st = apply_settings_json(st, &json!({"format": "animatedGif", "webSnap": true, "colors": 128}), "file.exportWizard")?;
                    reason = "msg-wizard-animated-gif";
                } else if dest == "image" {
                    st = apply_settings_json(st, &json!({"format": "png32"}), "file.exportWizard")?;
                    reason = "msg-wizard-png32-edit";
                } else if colors <= 256 {
                    let tr = if transparent_canvas && alpha { "index" } else { "none" };
                    st = apply_settings_json(st, &json!({"format": "png8", "palette": "adaptive", "colors": colors.clamp(2, 256), "transparency": tr}), "file.exportWizard")?;
                    reason = "msg-wizard-png8";
                } else if transparent_canvas && alpha {
                    st = apply_settings_json(st, &json!({"format": "png32"}), "file.exportWizard")?;
                    reason = "msg-wizard-png32-alpha";
                } else {
                    st = apply_settings_json(st, &json!({"format": "jpeg", "quality": 80, "smoothing": 0}), "file.exportWizard")?;
                    reason = "msg-wizard-jpeg";
                }
                let ctx = page_context(&doc, page);
                if let Some(t) = p.opt_i64("targetSize")? {
                    st = fp_export::size::optimize_to_size(&img, &st, t.max(1) as usize, &ctx, &EncodeExtras::default()).map_err(|e| EngineError::Failed(e.to_string()))?.settings;
                }
                let bytes = if st.format.is_animated() {
                    export_whole(&ExportRenderer::new(s.renderer()), &doc, page, &st, &EncodeExtras::default())
                } else {
                    export_image(&img, &st, &ctx, &EncodeExtras::default()).map(|e| e.bytes)
                }
                .map_err(|e| EngineError::Failed(e.to_string()))?;
                if let Some(path) = p.opt_str("path")? {
                    std::fs::write(path, &bytes)?;
                }
                let value = json!({"settings": settings_json(&st), "reason": fp_i18n::t(reason), "reasonKey": reason, "colors": colors, "bytes": bytes.len()});
                if p.bool_or("apply", true)? {
                    // Original: kein Verlaufsschritt „Exportassistent“ (PM-5 D11)
                    let changed = with_doc(s, |d| {
                        let before = Arc::clone(&d.doc);
                        store_page_settings(d.doc_mut(), page, &st)?;
                        let changed = d.doc.pages.get(page).map(|p| &p.root.export) != before.pages.get(page).map(|p| &p.root.export);
                        if !changed {
                            d.doc = before;
                        }
                        Ok(changed)
                    })?;
                    return Ok(super::view::doc_without_step(s, changed, value));
                }
                Ok(CommandOutput::value(value))
            }),
    );
    r.add(
        cmd("file.updateHtml")
            .requires(DOC)
            .param("path", ParamKind::Path, "vorhandene HTML-Datei (Bilder und Tabellen werden neu geschrieben)")
            .run(|s, p| {
                let path = PathBuf::from(p.str("path")?);
                if !path.is_file() {
                    return Err(EngineError::NotFound(fp_i18n::tr!("err-nf-html-file", path = path.display().to_string())));
                }
                let base = path.file_stem().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
                let page = s.doc()?.page;
                let old = String::from_utf8_lossy(&std::fs::read(&path)?).into_owned();
                let mut files = export_active(s, FilesStyle::HtmlAndImages, &base, page, None, true)?;
                let html_name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                let ext = html_ext_of(s)?;
                let inserted = !old.to_ascii_lowercase().contains("<!-- fwtable");
                for f in files.iter_mut() {
                    if !f.path.contains('/') && f.path.eq_ignore_ascii_case(&format!("{base}.{ext}")) {
                        // nur die Firepixel-Tabelle austauschen, der übrige Inhalt der Seite bleibt
                        let new = String::from_utf8_lossy(&f.bytes).into_owned();
                        let merged = replace_html_table(&old, &new).ok_or_else(|| bad("file.updateHtml", fp_i18n::tr!("err-update-html-no-table", path = path.display().to_string())))?;
                        f.bytes = merged.into_bytes();
                        f.path = html_name.clone();
                    }
                }
                let written = write_files(&dir, &files)?;
                Ok(CommandOutput::value(json!({"files": written, "inserted": inserted})))
            }),
    );
    r.add(
        cmd("file.htmlSetup")
            .requires(DOC)
            .opt("settings", sp(), "Felder der HTML-Einstellungen (fp_model::HtmlSettings, snake_case) zum Setzen; ohne = lesen")
            .run(|s, p| {
                let d = s.doc()?;
                let page = d.page;
                let cur = d.doc.pages[page].root.export.as_ref().and_then(|e| e.html.as_deref()).cloned().unwrap_or_else(fp_model::HtmlSettings::cs6_default);
                let Some(Value::Object(upd)) = p.get("settings").cloned() else {
                    return Ok(CommandOutput::value(serde_json::to_value(&cur).unwrap_or(Value::Null)));
                };
                let mut v = serde_json::to_value(&cur).map_err(|e| EngineError::Failed(e.to_string()))?;
                if let Value::Object(m) = &mut v {
                    for (k, x) in upd {
                        m.insert(k, x);
                    }
                }
                let mut new: fp_model::HtmlSettings = serde_json::from_value(v).map_err(|e| bad("file.htmlSetup", e.to_string()))?;
                new.not_yet_set = Some(false);
                let out = serde_json::to_value(&new).unwrap_or(Value::Null);
                // Original: kein Verlaufsschritt „HTML-Einrichtung“ (PM-5 D13); unverändert = keine Änderung
                let same = d.doc.pages[page].root.export.as_ref().and_then(|e| e.html.as_deref()) == Some(&new);
                if !same {
                    with_doc(s, |d| {
                        let pg = d.doc_mut().page_mut(page).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-page")))?;
                        let e = pg.root.export.get_or_insert_with(|| Box::new(ExportSettings::default()));
                        e.html = Some(Box::new(new));
                        Ok(())
                    })?;
                }
                Ok(super::view::doc_without_step(s, !same, out))
            }),
    );
    r.add(cmd("edit.copyHtmlCode").requires(DOC).opt_default("style", ParamKind::Enum { values: vec!["htmlAndImages".into(), "cssAndImages".into()] }, json!("htmlAndImages"), "HTML-Art").opt("dir", ParamKind::Path, "Ordner für die Bilder (ohne: nur der Code)").run(|s, p| {
        let style = style_of(&p.opt_str("style")?.unwrap_or_else(|| "htmlAndImages".into())).unwrap_or(FilesStyle::HtmlAndImages);
        let base = base_name_of(s)?;
        let page = s.doc()?.page;
        let files = export_active(s, style, &base, page, None, true)?;
        let ext = format!(".{}", html_ext_of(s)?.to_ascii_lowercase());
        let is_html = |f: &ExportFile| !f.path.contains('/') && f.path.to_ascii_lowercase().ends_with(&ext);
        let html = files.iter().find(|f| is_html(f)).map(|f| String::from_utf8_lossy(&f.bytes).into_owned()).ok_or_else(|| EngineError::Failed(fp_i18n::t("err-no-html")))?;
        // mit Zielordner: die Bilder dorthin schreiben, damit der kopierte Code sie findet
        let others: Vec<ExportFile> = files.iter().filter(|f| !is_html(f)).cloned().collect();
        let written = match p.opt_str("dir")? {
            Some(dir) => write_files(Path::new(&dir), &others)?,
            None => Vec::new(),
        };
        s.clipboard.text = Some(html.clone());
        Ok(CommandOutput::session_with(json!({"html": html, "images": others.iter().map(|f| f.path.clone()).collect::<Vec<_>>(), "written": written})))
    }));
    r.add(
        cmd("modify.selectiveJpeg.settings")
            .requires(DOC)
            .opt("enabled", ParamKind::Bool, "selektive Qualität aktivieren")
            .opt("quality", ParamKind::Integer { min: Some(1), max: Some(100) }, "Qualität der Maske")
            .opt("preserveText", ParamKind::Bool, "Textqualität beibehalten")
            .opt("preserveButtons", ParamKind::Bool, "Schaltflächenqualität beibehalten")
            .run(|s, p| {
                let d = s.doc()?;
                let page = d.page;
                let mut st = page_settings(&d.doc, page);
                let sel = &mut st.jpeg.selective;
                let changed = p.has("enabled") || p.has("quality") || p.has("preserveText") || p.has("preserveButtons");
                if let Some(b) = p.opt_bool("enabled")? {
                    sel.enabled = b;
                }
                if let Some(q) = p.opt_i64("quality")? {
                    sel.quality = q.clamp(1, 100) as i32;
                }
                if let Some(b) = p.opt_bool("preserveText")? {
                    sel.preserve_text = b;
                }
                if let Some(b) = p.opt_bool("preserveButtons")? {
                    sel.preserve_buttons = b;
                }
                let v = json!({"enabled": sel.enabled, "quality": sel.quality, "preserveText": sel.preserve_text, "preserveButtons": sel.preserve_buttons});
                if !changed {
                    return Ok(CommandOutput::value(v));
                }
                with_doc(s, |d| store_page_settings(d.doc_mut(), page, &st))?;
                Ok(CommandOutput::doc_with(v))
            }),
    );
    for (name, browser, all) in [
        ("file.previewInBrowser.previewInPrimaryBrowser", 0usize, false),
        ("file.previewInBrowser.previewInSecondaryBrowser", 1, false),
        ("file.previewInBrowser.previewAllPagesInPrimaryBrowser", 0, true),
    ] {
        // Sekundärbrowser: im Menü wie im Original nur, wenn einer festgelegt ist
        r.add(cmd(name).requires(DOC).menu_when(move |s| browser == 0 || s.ui.browsers[browser].is_some()).opt_default("open", ParamKind::Bool, json!(true), "Browser starten").run(move |s, p| {
            let base = base_name_of(s)?;
            let dir = temp_dir("Preview").join(format!("doc{}", s.doc()?.id.0));
            let _ = std::fs::remove_dir_all(&dir);
            let pages: Vec<usize> = if all { (0..s.doc()?.doc.pages.len()).filter(|&i| !s.doc().map(|d| d.doc.pages[i].master()).unwrap_or(false)).collect() } else { vec![s.doc()?.page] };
            let mut written = Vec::new();
            let mut first = None;
            for (n, &pg) in pages.iter().enumerate() {
                let b = if all { format!("{base}_s{}", n + 1) } else { base.clone() };
                let files = export_active(s, FilesStyle::HtmlAndImages, &b, pg, None, true)?;
                written.extend(write_files(&dir, &files)?);
                first.get_or_insert(dir.join(format!("{b}.htm")));
            }
            let html = first.ok_or_else(|| EngineError::Failed(fp_i18n::t("err-no-page")))?;
            if launch_allowed(p)? {
                let prog = s.ui.browsers[browser].clone();
                open_with_system(&html, prog.as_deref())?;
            }
            Ok(CommandOutput::value(json!({"html": html.to_string_lossy(), "files": written})))
        }));
    }
    for (fw_id, kind) in [(35587u32, 0u8), (35588, 1), (35420, 2)] {
        // Name aus dem Katalog (über die Befehls-ID, damit die Namensgebung dort maßgeblich bleibt)
        let Some(name) = crate::catalog::entries().iter().find(|e| e.fw_id == Some(fw_id)).map(|e| e.name.clone()) else { continue };
        r.add(
            cmd(&name)
                .requires(DOC)
                .opt("to", ParamKind::String, "Empfänger")
                .opt_default("open", ParamKind::Bool, json!(true), "E-Mail-Programm mit dem Entwurf öffnen")
                .run(move |s, p| {
                    let base = base_name_of(s)?;
                    let d = s.doc()?;
                    let page = d.page;
                    let (file_name, data) = match kind {
                        0 => {
                            let doc = Arc::clone(&d.doc);
                            let tmp = temp_dir("Email").join(format!("{base}.fw.png"));
                            super::file::save_document(s, &doc, page, &tmp)?;
                            (format!("{base}.fw.png"), std::fs::read(&tmp)?)
                        }
                        1 => {
                            let img = render_page_image(s, page, 0, false)?;
                            let st = apply_settings_json(OptimizeSettings::default(), &json!({"format": "jpeg", "quality": 80}), &p.command)?;
                            let b = export_image(&img, &st, &page_context(&d.doc, page), &EncodeExtras::default()).map_err(|e| EngineError::Failed(e.to_string()))?.bytes;
                            (format!("{base}.jpg"), b)
                        }
                        _ => {
                            let st = page_settings(&d.doc, page);
                            let b = export_whole(&ExportRenderer::new(s.renderer()), &d.doc, page, &st, &EncodeExtras::default()).map_err(|e| EngineError::Failed(e.to_string()))?;
                            (format!("{base}.{}", st.format.extension()), b)
                        }
                    };
                    let ext = file_name.rsplit('.').next().unwrap_or("");
                    let eml = build_eml(&p.opt_str("to")?.unwrap_or_default(), &file_name, &format!("Anhang: {file_name}"), &file_name, mime_of(ext), &data);
                    let path = temp_dir("Email").join(format!("{base}.eml"));
                    std::fs::write(&path, eml)?;
                    if launch_allowed(p)? {
                        open_with_system(&path, None)?;
                    }
                    Ok(CommandOutput::value(json!({"eml": path.to_string_lossy(), "attachment": file_name, "bytes": data.len()})))
                }),
        );
    }
    r.add(
        cmd("file.pageSetup")
            .opt("orientation", ParamKind::Enum { values: vec!["portrait".into(), "landscape".into()] }, "Ausrichtung")
            .opt("scale", ParamKind::Number { min: Some(1.0), max: Some(1000.0) }, "Skalierung in Prozent")
            .opt("fit", ParamKind::Bool, "auf Seite einpassen")
            .opt("paper", ParamKind::Json, "Papiergröße in mm [Breite, Höhe] oder \"A4\"/\"A3\"/\"Letter\"")
            .opt("margins", ParamKind::Json, "Ränder in mm [links, oben, rechts, unten] oder Zahl")
            .opt("dpi", ParamKind::Number { min: Some(36.0), max: Some(1200.0) }, "Druckauflösung")
            .opt("printer", ParamKind::String, "Drucker (leer = Standard)")
            .run(|s, p| {
                let mut ps = s.ui.page_setup.clone();
                if let Some(o) = p.opt_str("orientation")? {
                    ps.orientation = o;
                }
                if let Some(v) = p.opt_f64("scale")? {
                    ps.scale = v;
                }
                if let Some(b) = p.opt_bool("fit")? {
                    ps.fit = b;
                }
                if let Some(v) = p.opt_f64("dpi")? {
                    ps.dpi = v;
                }
                if let Some(v) = p.opt_str("printer")? {
                    ps.printer = v;
                }
                match p.get("paper") {
                    Some(Value::String(n)) => {
                        ps.paper_mm = match n.to_ascii_lowercase().as_str() {
                            "a3" => [297.0, 420.0],
                            "a4" => [210.0, 297.0],
                            "a5" => [148.0, 210.0],
                            "letter" => [215.9, 279.4],
                            "legal" => [215.9, 355.6],
                            _ => return Err(bad("file.pageSetup", fp_i18n::t("err-param-paper"))),
                        }
                    }
                    Some(Value::Array(a)) if a.len() == 2 => ps.paper_mm = [a[0].as_f64().unwrap_or(210.0).max(10.0), a[1].as_f64().unwrap_or(297.0).max(10.0)],
                    Some(_) => return Err(bad("file.pageSetup", "paper")),
                    None => {}
                }
                match p.get("margins") {
                    Some(Value::Number(n)) => ps.margins_mm = [n.as_f64().unwrap_or(10.0).max(0.0); 4],
                    Some(Value::Array(a)) if a.len() == 4 => {
                        for (i, x) in a.iter().enumerate() {
                            ps.margins_mm[i] = x.as_f64().unwrap_or(10.0).max(0.0);
                        }
                    }
                    Some(_) => return Err(bad("file.pageSetup", "margins")),
                    None => {}
                }
                s.ui.page_setup = ps.clone();
                Ok(CommandOutput::session_with(serde_json::to_value(&ps).unwrap_or(Value::Null)))
            }),
    );
    r.add(
        cmd("file.print")
            .requires(DOC)
            .opt("printer", ParamKind::String, "Drucker (Vorgabe aus „Seite einrichten“)")
            .opt("pages", ParamKind::Json, "Seitenindizes (Vorgabe aktuelle Seite) oder \"all\"")
            .opt_default("open", ParamKind::Bool, json!(true), "an den Drucker senden (false: nur Druckbilder erzeugen)")
            .opt("outDir", ParamKind::Path, "Ordner für die Druckbilder (Vorgabe temporär)")
            .run(|s, p| {
                let d = s.doc()?;
                let pages: Vec<usize> = match p.get("pages") {
                    Some(Value::String(a)) if a == "all" => (0..d.doc.pages.len()).filter(|&i| !d.doc.pages[i].master()).collect(),
                    Some(Value::Array(a)) => a.iter().filter_map(|x| x.as_u64().map(|v| v as usize)).filter(|&i| i < d.doc.pages.len()).collect(),
                    Some(_) => return Err(bad("file.print", "pages")),
                    None => vec![d.page],
                };
                if pages.is_empty() {
                    return Err(bad("file.print", fp_i18n::t("err-no-valid-page")));
                }
                let ps = s.ui.page_setup.clone();
                let printer = p.opt_str("printer")?.unwrap_or_else(|| ps.printer.clone());
                let dir = p.opt_str("outDir")?.map(PathBuf::from).unwrap_or_else(|| temp_dir("Print"));
                std::fs::create_dir_all(&dir)?;
                let base = base_name_of(s)?;
                let mut out = Vec::new();
                let mut method = Value::Null;
                for pg in pages {
                    let img = render_page_image(s, pg, 0, false)?;
                    let dpi = page_context(&s.doc()?.doc, pg).resolution_dpi as f64;
                    let paper = print_image(&img, dpi, &ps);
                    let path = dir.join(format!("{base}_print_{}.png", pg + 1));
                    std::fs::write(&path, png_bytes(&paper)?)?;
                    if launch_allowed(p)? {
                        method = json!(system_print(&path, &printer)?);
                    }
                    out.push(json!({"page": pg, "file": path.to_string_lossy(), "width": paper.width(), "height": paper.height()}));
                }
                Ok(CommandOutput::value(json!({"printed": out, "method": method, "printer": printer})))
            }),
    );
    r.add(
        cmd("file.batchProcess")
            .opt("files", ParamKind::Json, "Liste der Eingabedateien")
            .opt("outDir", ParamKind::Path, "Zielordner")
            // BATCH-2 (SPEC-BATCH §5.1): mit `recipe` an `batch.run` weiterleiten (neuer Batch-Modus)
            .opt("recipe", ParamKind::Json, "Rezept (Objekt) oder Pfad einer Rezept-Datei")
            .opt("commands", ParamKind::Json, "Befehle je Datei [{name, params}] (z. B. modify.canvas.imageSize)")
            .opt_default("output", ParamKind::Enum { values: vec!["export".into(), "fwpng".into()] }, json!("export"), "Ergebnis exportieren (Optimieren-Einstellungen) oder als FW-PNG speichern")
            .opt("settings", sp(), "Optimieren-Einstellungen für den Export (Vorgabe: die der Datei)")
            .opt("prefix", ParamKind::String, "Präfix für Dateinamen")
            .opt("suffix", ParamKind::String, "Suffix für Dateinamen")
            .opt("replace", ParamKind::Json, "[suchen, ersetzen] im Dateinamen")
            .opt_default("backup", ParamKind::Bool, json!(false), "vorhandene Zieldateien als „Original Files“ sichern")
            .run(|s, p| {
                if p.get("recipe").is_some_and(|v| !v.is_null()) {
                    let mut v = if p.value.is_object() { p.value.clone() } else { json!({}) };
                    if v.get("out").is_none()
                        && let Some(o) = v.get("outDir").cloned()
                    {
                        v["out"] = o;
                    }
                    let host = super::batch::batch_host().ok_or(EngineError::BackendMissing("err-backend-batch-runtime"))?;
                    return host.handle(s, super::batch::BatchOp::Run, &v).map(CommandOutput::value).map_err(EngineError::Failed);
                }
                let files: Vec<String> = p.parse("files")?.unwrap_or_default();
                let cmds: Vec<crate::history::RecordedCommand> = match p.get("commands") {
                    Some(v) => serde_json::from_value(v.clone()).or_else(|_| {
                        serde_json::from_value::<Vec<Value>>(v.clone()).map(|a| {
                            a.into_iter().filter_map(|x| Some(crate::history::RecordedCommand { command: x.get("name")?.as_str()?.to_string(), params: x.get("params").cloned().unwrap_or(Value::Null) })).collect()
                        })
                    }).map_err(|e| bad("file.batchProcess", e.to_string()))?,
                    None => Vec::new(),
                };
                let out_dir = PathBuf::from(p.str("outDir")?);
                std::fs::create_dir_all(&out_dir)?;
                let to_fw = p.opt_str("output")?.as_deref() == Some("fwpng");
                let prefix = p.opt_str("prefix")?.unwrap_or_default();
                let suffix = p.opt_str("suffix")?.unwrap_or_default();
                let replace: Option<(String, String)> = p.parse("replace")?;
                let backup = p.bool_or("backup", false)?;
                let settings_v = p.get("settings").cloned();
                let mut results = Vec::new();
                let mut errors = Vec::new();
                for f in &files {
                    let mut sub = Session::with_registry(s.prefs.clone(), Arc::clone(s.registry()));
                    sub.set_renderer(s.renderer().cloned());
                    sub.set_script_host(s.script_host());
                    let res: Result<String> = (|| {
                        sub.execute("file.open", json!({"path": f}))?;
                        for c in &cmds {
                            sub.execute(&c.command, c.params.clone())?;
                        }
                        let stem = Path::new(f).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                        let mut stem = stem.split('.').next().unwrap_or("datei").to_string();
                        if let Some((a, b)) = &replace {
                            stem = stem.replace(a.as_str(), b);
                        }
                        let name = format!("{prefix}{stem}{suffix}");
                        let target = if to_fw {
                            out_dir.join(format!("{name}.fw.png"))
                        } else {
                            let d = sub.doc()?;
                            let st = apply_settings_json(page_settings(&d.doc, d.page), settings_v.as_ref().unwrap_or(&Value::Null), "file.batchProcess")?;
                            out_dir.join(format!("{name}.{}", st.format.extension()))
                        };
                        if backup && target.exists() {
                            let bdir = out_dir.join("Original Files");
                            std::fs::create_dir_all(&bdir)?;
                            if let Some(n) = target.file_name() {
                                std::fs::copy(&target, bdir.join(n))?;
                            }
                        }
                        if to_fw {
                            let d = sub.doc()?;
                            super::file::save_document(&sub, &d.doc, d.page, &target)?;
                        } else {
                            let mut params = json!({"path": target.to_string_lossy()});
                            if let Some(v) = &settings_v {
                                params["settings"] = v.clone();
                            }
                            sub.execute("file.export", params)?;
                        }
                        Ok(target.to_string_lossy().to_string())
                    })();
                    match res {
                        Ok(t) => results.push(json!({"input": f, "output": t})),
                        Err(e) => errors.push(json!({"input": f, "error": e.to_string()})),
                    }
                }
                Ok(CommandOutput::value(json!({"processed": results, "errors": errors})))
            }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_json_roundtrip() {
        let s = apply_settings_json(OptimizeSettings::default(), &json!({"format": "jpeg", "quality": 55, "matte": "#ff0000"}), "t").unwrap();
        assert_eq!(s.format, ExportFormat::Jpeg);
        assert_eq!(s.jpeg.quality, 55);
        assert_eq!(s.matte, Some(0xFFFF_0000));
        let v = settings_json(&s);
        assert_eq!(v["format"], "jpeg");
        let s2 = apply_settings_json(OptimizeSettings::default(), &v, "t").unwrap();
        assert_eq!(s2.format, s.format);
        assert_eq!(s2.jpeg.quality, 55);
    }

    #[test]
    fn eml_has_attachment() {
        let e = build_eml("a@b", "x.png", "body", "x.png", "image/png", &[1, 2, 3]);
        assert!(e.starts_with("X-Unsent: 1"));
        assert!(e.contains("filename=\"x.png\"") && e.contains("AQID"));
    }
}
