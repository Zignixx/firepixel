//! Ausgabe (SPEC-BATCH §2.7, §3.2, §5.2 Punkt 3): Zielformat, Endung, Kodieren über `fp-export`
//! bzw. FW-PNG über `fp-engine`, Schreiben in eine Temporärdatei `.<name>.fptmp` im Zielordner und
//! atomares Umbenennen, Sicherung des Originals in `Original Files`, Pfade über 260 Zeichen.
//!
//! Spätere Pakete können für weitere Formate (SVG über `vector.trace`, PSD, JPEG XL, PDF …) einen
//! Kodierer mit [`register_encoder`] einhängen; er hat Vorrang vor dem eingebauten.

use crate::error::{BatchError, Result};
use crate::item::Item;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};

/// Zielformat der Endkarte „Speichern als“.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum OutputFormat {
    /// Wie die Quelle (wird je Element aufgelöst).
    Same,
    Png,
    Jpeg,
    WebP,
    Avif,
    Gif,
    Apng,
    Tiff,
    Bmp,
    Qoi,
    Ico,
    Wbmp,
    FwPng,
    Tga,
    Psd,
    Jxl,
    Pdf,
    Svg,
}

impl OutputFormat {
    /// Alle Formate (Reihenfolge der Auswahl in der Endkarte).
    pub const ALL: [OutputFormat; 18] = [
        OutputFormat::Same,
        OutputFormat::Png,
        OutputFormat::Jpeg,
        OutputFormat::WebP,
        OutputFormat::Avif,
        OutputFormat::Gif,
        OutputFormat::Apng,
        OutputFormat::Tiff,
        OutputFormat::Bmp,
        OutputFormat::Tga,
        OutputFormat::Qoi,
        OutputFormat::Ico,
        OutputFormat::Wbmp,
        OutputFormat::FwPng,
        OutputFormat::Psd,
        OutputFormat::Jxl,
        OutputFormat::Pdf,
        OutputFormat::Svg,
    ];

    /// Aus dem Rezept-Wert (`same`, `png`, `jpeg`/`jpg`, `tiff`/`tif`, `fwpng`/`fw.png` …).
    pub fn parse(s: &str) -> Option<OutputFormat> {
        Some(
            match s
                .trim()
                .trim_start_matches('.')
                .to_ascii_lowercase()
                .as_str()
            {
                "same" | "" => OutputFormat::Same,
                "png" => OutputFormat::Png,
                "jpeg" | "jpg" => OutputFormat::Jpeg,
                "webp" => OutputFormat::WebP,
                "avif" => OutputFormat::Avif,
                "gif" => OutputFormat::Gif,
                "apng" => OutputFormat::Apng,
                "tiff" | "tif" => OutputFormat::Tiff,
                "bmp" => OutputFormat::Bmp,
                "qoi" => OutputFormat::Qoi,
                "ico" => OutputFormat::Ico,
                "wbmp" => OutputFormat::Wbmp,
                "fwpng" | "fw.png" => OutputFormat::FwPng,
                "tga" => OutputFormat::Tga,
                "psd" => OutputFormat::Psd,
                "jxl" => OutputFormat::Jxl,
                "pdf" => OutputFormat::Pdf,
                "svg" => OutputFormat::Svg,
                _ => return None,
            },
        )
    }

    /// Wert im Rezept.
    pub fn name(self) -> &'static str {
        match self {
            OutputFormat::Same => "same",
            OutputFormat::Png => "png",
            OutputFormat::Jpeg => "jpeg",
            OutputFormat::WebP => "webp",
            OutputFormat::Avif => "avif",
            OutputFormat::Gif => "gif",
            OutputFormat::Apng => "apng",
            OutputFormat::Tiff => "tiff",
            OutputFormat::Bmp => "bmp",
            OutputFormat::Qoi => "qoi",
            OutputFormat::Ico => "ico",
            OutputFormat::Wbmp => "wbmp",
            OutputFormat::FwPng => "fwpng",
            OutputFormat::Tga => "tga",
            OutputFormat::Psd => "psd",
            OutputFormat::Jxl => "jxl",
            OutputFormat::Pdf => "pdf",
            OutputFormat::Svg => "svg",
        }
    }

    /// Endung ohne Punkt (klein). `jpeg_as_jpg`: `jpg` statt `jpeg`.
    pub fn extension(self, jpeg_as_jpg: bool) -> &'static str {
        match self {
            OutputFormat::Jpeg if jpeg_as_jpg => "jpg",
            OutputFormat::Jpeg => "jpeg",
            OutputFormat::Apng => "png",
            OutputFormat::Tiff => "tif",
            OutputFormat::FwPng => "fw.png",
            OutputFormat::Same => "",
            f => f.name(),
        }
    }

    /// Kann der Kern (oder ein eingehängter Kodierer) dieses Format schreiben?
    pub fn can_encode(self) -> bool {
        matches!(
            self,
            OutputFormat::Png
                | OutputFormat::Jpeg
                | OutputFormat::WebP
                | OutputFormat::Avif
                | OutputFormat::Gif
                | OutputFormat::Apng
                | OutputFormat::Tiff
                | OutputFormat::Bmp
                | OutputFormat::Qoi
                | OutputFormat::Ico
                | OutputFormat::Wbmp
                | OutputFormat::FwPng
        ) || encoders().read().is_ok_and(|m| m.contains_key(&self))
    }

    /// Formatname für Anzeigen (Eigenname, nicht übersetzt).
    pub fn display_name(self) -> &'static str {
        match self {
            OutputFormat::Same => "",
            OutputFormat::Png => "PNG",
            OutputFormat::Jpeg => "JPEG",
            OutputFormat::WebP => "WebP",
            OutputFormat::Avif => "AVIF",
            OutputFormat::Gif => "GIF",
            OutputFormat::Apng => "APNG",
            OutputFormat::Tiff => "TIFF",
            OutputFormat::Bmp => "BMP",
            OutputFormat::Qoi => "QOI",
            OutputFormat::Ico => "ICO",
            OutputFormat::Wbmp => "WBMP",
            OutputFormat::FwPng => "FW-PNG",
            OutputFormat::Tga => "TGA",
            OutputFormat::Psd => "PSD",
            OutputFormat::Jxl => "JPEG XL",
            OutputFormat::Pdf => "PDF",
            OutputFormat::Svg => "SVG",
        }
    }
}

/// Qualität, die ohne Angabe im Rezept geschrieben wird (JPEG 85, WebP 82, AVIF 60). Die
/// Oberfläche zeigt denselben Wert im Kartenkopf „Speichern als“ (CT-C-08: Kopf 85, Feld/Datei 82).
pub fn default_quality(fmt: OutputFormat) -> Option<i64> {
    match fmt {
        OutputFormat::Jpeg => Some(85),
        OutputFormat::WebP => Some(82),
        OutputFormat::Avif => Some(60),
        _ => None,
    }
}

/// Löst „wie Quelle“ auf: Format der Quelle, wenn schreibbar, sonst PNG mit Hinweis (übersetzt).
pub fn resolve_format(fmt: OutputFormat, source_format: &str) -> (OutputFormat, Option<String>) {
    if fmt != OutputFormat::Same {
        return (fmt, None);
    }
    match OutputFormat::parse(source_format) {
        Some(f) if f != OutputFormat::Same && f.can_encode() => (f, None),
        _ => (
            OutputFormat::Png,
            Some(fp_i18n::tr!(
                "batch-hint-same-fallback-png",
                format = source_format.to_string()
            )),
        ),
    }
}

/// Eingehängter Kodierer: Element + Formatoptionen → Dateiinhalt.
pub type Encoder = fn(&Item, &Map<String, Value>) -> Result<Vec<u8>>;

fn encoders() -> &'static RwLock<HashMap<OutputFormat, Encoder>> {
    static E: OnceLock<RwLock<HashMap<OutputFormat, Encoder>>> = OnceLock::new();
    E.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Hängt einen Kodierer für ein Format ein (ersetzt einen vorhandenen).
pub fn register_encoder(fmt: OutputFormat, f: Encoder) {
    if let Ok(mut m) = encoders().write() {
        m.insert(fmt, f);
    }
}

fn opt_i(o: &Map<String, Value>, k: &str) -> Option<i64> {
    o.get(k)
        .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f.round() as i64)))
}
fn opt_b(o: &Map<String, Value>, k: &str) -> Option<bool> {
    o.get(k).and_then(Value::as_bool)
}
fn opt_s<'a>(o: &'a Map<String, Value>, k: &str) -> Option<&'a str> {
    o.get(k).and_then(Value::as_str)
}

/// Dokument eines Elements für den Export.
pub fn item_document(item: &Item) -> Result<fp_model::Document> {
    match item {
        Item::Still(s) => {
            let r = fp_io::RasterImage {
                image: s.image.clone(),
                dpi: s.meta.dpi,
                palette: None,
                has_transparency: s.meta.has_transparency,
                layer_name: None,
            };
            Ok(fp_io::build::document_from_raster(
                r,
                fp_io::FileFormat::Png,
            ))
        }
        Item::Animation(a) => {
            let frames = a
                .frames
                .iter()
                .map(|f| fp_io::AnimationFrame {
                    image: f.image.clone(),
                    x: 0,
                    y: 0,
                    delay_cs: f.delay_ms.div_ceil(10),
                    disposal: 1,
                    replace: true,
                })
                .collect();
            let fa = fp_io::Animation {
                width: a.width,
                height: a.height,
                frames,
                loop_count: a.loop_count,
                has_transparency: a.meta.has_transparency,
                palette: None,
            };
            Ok(fp_io::build::document_from_animation(
                fa,
                fp_io::FileFormat::Apng,
                &fp_io::ImportOptions::default(),
            ))
        }
        Item::Document(d) => Ok((*d.doc).clone()),
        Item::FileOnly(p) => Err(BatchError::Unsupported(fp_i18n::tr!(
            "batch-err-not-image",
            path = p.display().to_string()
        ))),
    }
}

/// Kodiert ein Element. Liefert Dateiinhalt und Warnungen (übersetzt). `fmt` darf nicht `Same` sein.
/// Mit Nachbearbeitung von BATCH-5 (`meta::finish_encode`: Metadaten, PNG-Optimierung, Zielgröße, TIFF).
pub fn encode(
    item: &Item,
    fmt: OutputFormat,
    options: &Map<String, Value>,
) -> Result<(Vec<u8>, Vec<String>)> {
    crate::meta::finish_encode(item, fmt, options, encode_plain)
}

/// Kodiert ein Element ohne Nachbearbeitung.
pub fn encode_plain(
    item: &Item,
    fmt: OutputFormat,
    options: &Map<String, Value>,
) -> Result<(Vec<u8>, Vec<String>)> {
    let mut warnings = Vec::new();
    if let Some(f) = encoders().read().ok().and_then(|m| m.get(&fmt).copied()) {
        let bytes = f(item, options)?;
        // GIF mit Zielgröße: nicht erreicht → Warnung im Bericht und in der Vorschau (BATCH-11b)
        if fmt == OutputFormat::Gif
            && let Some(kb) = opt_i(options, "maxKB").filter(|k| *k > 0)
            && bytes.len() as u64 > kb as u64 * 1024
        {
            warnings.push(fp_i18n::tr!(
                "batch-warn-gif-target-missed",
                target = fp_i18n::fmt_size(kb as u64 * 1024),
                reached = fp_i18n::fmt_size(bytes.len() as u64)
            ));
        }
        return Ok((bytes, warnings));
    }
    if fmt == OutputFormat::FwPng {
        let dir = std::env::temp_dir().join("firepixel-batch");
        std::fs::create_dir_all(&dir).map_err(|e| BatchError::Io(e.to_string()))?;
        let tmp = dir.join(
            format!(
                "{}-{:?}.fptmp",
                std::process::id(),
                std::thread::current().id()
            )
            .replace(['(', ')'], ""),
        );
        write_fwpng(item, &tmp)?;
        let bytes = std::fs::read(&tmp).map_err(|e| BatchError::Io(e.to_string()));
        let _ = std::fs::remove_file(&tmp);
        return Ok((bytes?, warnings));
    }
    if !fmt.can_encode() || fmt == OutputFormat::Same {
        return Err(BatchError::Unsupported(fp_i18n::tr!(
            "batch-err-format-unavailable",
            format = fmt.display_name()
        )));
    }
    let doc = item_document(item)?;
    let animated = matches!(item, Item::Animation(a) if a.frames.len() > 1);
    let has_alpha = item.meta().is_some_and(|m| m.has_transparency);
    use fp_export::{ColorMode, Dither, ExportFormat, Transparency};
    let mut s = fp_export::doc_export::page_settings(&doc, 0);
    s.crop = None;
    s.scale = None;
    let quality = opt_i(options, "quality");
    s.format = match fmt {
        OutputFormat::Png => ExportFormat::Png,
        OutputFormat::Jpeg => ExportFormat::Jpeg,
        OutputFormat::WebP if animated => ExportFormat::AnimatedWebP,
        OutputFormat::WebP if opt_b(options, "lossless").unwrap_or(false) => {
            ExportFormat::WebPLossless
        }
        OutputFormat::WebP => ExportFormat::WebP,
        OutputFormat::Avif => ExportFormat::Avif,
        OutputFormat::Gif if animated => ExportFormat::AnimatedGif,
        OutputFormat::Gif => ExportFormat::Gif,
        OutputFormat::Apng => ExportFormat::Apng,
        OutputFormat::Tiff => ExportFormat::Tiff,
        OutputFormat::Bmp => ExportFormat::Bmp,
        OutputFormat::Qoi => ExportFormat::Qoi,
        OutputFormat::Ico => ExportFormat::Ico,
        OutputFormat::Wbmp => ExportFormat::Wbmp,
        _ => {
            return Err(BatchError::Unsupported(fp_i18n::tr!(
                "batch-err-format-unavailable",
                format = fmt.display_name()
            )));
        }
    };
    if animated && !s.format.is_animated() {
        warnings.push(fp_i18n::tr!(
            "batch-warn-first-frame-only",
            format = fmt.display_name()
        ));
    }
    s.color_mode = if has_alpha {
        ColorMode::Rgba32
    } else {
        ColorMode::Rgb24
    };
    match fmt {
        OutputFormat::Png => {
            s.color_mode = match opt_i(options, "bitDepth") {
                Some(8) => ColorMode::Indexed,
                Some(24) => ColorMode::Rgb24,
                Some(32) => ColorMode::Rgba32,
                _ => s.color_mode,
            };
            if s.color_mode == ColorMode::Indexed {
                s.max_colors = opt_i(options, "palette")
                    .or_else(|| opt_i(options, "colors"))
                    .unwrap_or(256)
                    .clamp(2, 256) as i32;
                s.transparency = if has_alpha {
                    Transparency::Rgba
                } else {
                    Transparency::None
                };
            }
            s.interlaced = opt_b(options, "interlace").unwrap_or(false);
        }
        OutputFormat::Jpeg => {
            let q = quality
                .unwrap_or(default_quality(OutputFormat::Jpeg).unwrap_or(85))
                .clamp(1, 100) as i32;
            s.color_mode = ColorMode::Rgb24;
            s.jpeg.quality = q;
            s.jpeg.progressive = opt_b(options, "progressive").unwrap_or(true);
            s.jpeg.subsampling = match opt_s(options, "subsampling") {
                Some("4:4:4") => 0,
                Some("4:2:0" | "4:2:2") => 1,
                _ if q >= 90 => 0,
                _ => 1,
            };
            let bg = opt_s(options, "background")
                .and_then(crate::step::parse_color)
                .unwrap_or([255, 255, 255, 255]);
            s.matte = Some(0xff00_0000 | (bg[0] as u32) << 16 | (bg[1] as u32) << 8 | bg[2] as u32);
        }
        OutputFormat::WebP => {
            s.color_mode = ColorMode::Rgba32;
            s.webp.lossless = opt_b(options, "lossless").unwrap_or(false);
            s.webp.quality = quality
                .unwrap_or(default_quality(OutputFormat::WebP).unwrap_or(82))
                .clamp(0, 100) as i32;
            // BA-108: alpha quality and encoder effort reach the writer
            s.webp.alpha_quality = opt_i(options, "alphaQuality")
                .unwrap_or(i64::from(
                    fp_export::options::WebpSettings::DEFAULT_ALPHA_QUALITY,
                ))
                .clamp(0, 100) as i32;
            s.webp.method = opt_i(options, "method")
                .unwrap_or(i64::from(fp_export::options::WebpSettings::DEFAULT_METHOD))
                .clamp(0, 6) as i32;
        }
        OutputFormat::Avif => {
            s.color_mode = ColorMode::Rgba32;
            s.avif.quality = quality
                .unwrap_or(default_quality(OutputFormat::Avif).unwrap_or(60))
                .clamp(1, 100) as i32;
            s.avif.speed = opt_i(options, "speed").unwrap_or(6).clamp(1, 10) as u8;
            s.avif.alpha_quality = opt_i(options, "alphaQuality").map(|a| a.clamp(1, 100) as i32); // BA-108
        }
        OutputFormat::Gif => {
            s.color_mode = ColorMode::Indexed;
            s.max_colors = opt_i(options, "colors").unwrap_or(256).clamp(2, 256) as i32;
            s.dither = match opt_s(options, "dither") {
                Some("none") => Dither::None,
                _ => Dither::Diffusion(
                    opt_i(options, "ditherAmount").unwrap_or(80).clamp(0, 100) as i32
                ),
            };
            s.transparency = if has_alpha && opt_b(options, "transparency").unwrap_or(true) {
                Transparency::Index
            } else {
                Transparency::None
            };
        }
        OutputFormat::Apng => s.color_mode = ColorMode::Rgba32,
        OutputFormat::Wbmp => s.color_mode = ColorMode::Indexed,
        _ => {}
    }
    if let Item::Animation(a) = item {
        s.anim.loop_count = a.loop_count;
    }
    let opaque = fp_render::Renderer::default();
    let transparent = fp_render::Renderer::new(fp_render::RenderOptions {
        background: fp_render::BackgroundMode::Transparent,
        ..fp_render::RenderOptions::default()
    });
    let render = |d: &fp_model::Document,
                  req: &fp_export::render::RenderRequest|
     -> std::result::Result<fp_raster::RgbaImage, String> {
        let r = match req.background {
            fp_export::render::Background::Transparent => &transparent,
            _ => &opaque,
        };
        let area = req
            .area
            .or_else(|| fp_render::Renderer::page_rect(d, req.page))
            .ok_or_else(|| fp_i18n::t("batch-err-empty-page"))?;
        Ok(r.render_rect(d, req.page, req.frame, area).image)
    };
    let bytes = fp_export::doc_export::export_whole(
        &render,
        &doc,
        0,
        &s,
        &fp_export::export::EncodeExtras::standalone(),
    )
    .map_err(|e| BatchError::Encode(e.to_string()))?;
    Ok((bytes, warnings))
}

/// Schreibt ein Element als FW-PNG (bearbeitbar) nach `path` (über `fp-engine`, wie „Speichern“).
pub fn write_fwpng(item: &Item, path: &Path) -> Result<()> {
    let doc = item_document(item)?;
    thread_local! {
        static SESSION: fp_engine::Session = fp_engine::Session::default();
    }
    SESSION
        .with(|s| fp_engine::commands::save_document(s, &doc, 0, path))
        .map_err(|e| BatchError::Encode(e.to_string()))
}

// ------------------------------------------------------------------ Schreiben

/// Pfad mit `\\?\`-Präfix, wenn er unter Windows länger als 259 Zeichen ist (absolut).
pub fn long_path(p: &Path) -> PathBuf {
    if cfg!(windows) {
        let s = p.to_string_lossy();
        if s.chars().count() > 259 && p.is_absolute() && !s.starts_with(r"\\?\") {
            if let Some(unc) = s.strip_prefix(r"\\") {
                return PathBuf::from(format!(r"\\?\UNC\{unc}"));
            }
            return PathBuf::from(format!(r"\\?\{}", s.replace('/', "\\")));
        }
    }
    p.to_path_buf()
}

/// Temporärdatei für ein Ziel: `.<name>.fptmp` im selben Ordner.
pub fn temp_path(target: &Path) -> PathBuf {
    let name = target
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    target.with_file_name(format!(".{name}.fptmp"))
}

/// Endung der Temporärdateien (für Aufräumen und Tests).
pub const TEMP_EXTENSION: &str = "fptmp";

fn io_err(p: &Path, e: std::io::Error) -> BatchError {
    BatchError::Io(format!("{}: {e}", p.display()))
}

/// Schreibt Bytes atomar: Temporärdatei im Zielordner, dann umbenennen (ersetzt ein vorhandenes Ziel).
pub fn write_bytes_atomic(target: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(dir) = target.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(long_path(dir)).map_err(|e| io_err(dir, e))?;
    }
    let tmp = temp_path(target);
    std::fs::write(long_path(&tmp), bytes).map_err(|e| io_err(&tmp, e))?;
    commit_temp(&tmp, target)
}

/// Temporärdatei an ihr Ziel umbenennen (bei Fehler wird sie gelöscht).
pub fn commit_temp(tmp: &Path, target: &Path) -> Result<()> {
    std::fs::rename(long_path(tmp), long_path(target)).map_err(|e| {
        let _ = std::fs::remove_file(long_path(tmp));
        io_err(target, e)
    })
}

/// Name des Sicherungsordners (technisch englisch, wie die Vorgabe des Originals).
pub const BACKUP_FOLDER: &str = "Original Files";

/// Verschiebt das Original in `Original Files` neben der Quelle. `incremental`: vorhandene
/// Sicherungen bleiben, die neue heißt `name-1.ext`, `name-2.ext` …; sonst wird die alte Sicherung
/// ersetzt. Liefert den Pfad der Sicherung.
pub fn backup_original(source: &Path, incremental: bool) -> Result<PathBuf> {
    let dir = source.parent().unwrap_or(Path::new("")).join(BACKUP_FOLDER);
    std::fs::create_dir_all(long_path(&dir)).map_err(|e| io_err(&dir, e))?;
    let name = source
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let mut dest = dir.join(&name);
    if incremental && dest.exists() {
        let (base, ext) = crate::naming::split_extension(&name);
        let mut k = 1;
        loop {
            let cand = dir.join(if ext.is_empty() {
                format!("{base}-{k}")
            } else {
                format!("{base}-{k}.{ext}")
            });
            if !cand.exists() {
                dest = cand;
                break;
            }
            k += 1;
        }
    }
    if dest.exists() && !incremental {
        std::fs::remove_file(long_path(&dest)).map_err(|e| io_err(&dest, e))?;
    }
    std::fs::rename(long_path(source), long_path(&dest)).map_err(|e| io_err(source, e))?;
    Ok(dest)
}

/// Kopiert eine Datei atomar an ihr Ziel (ohne Dekodieren).
pub fn copy_atomic(source: &Path, target: &Path) -> Result<u64> {
    if let Some(dir) = target.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(long_path(dir)).map_err(|e| io_err(dir, e))?;
    }
    let tmp = temp_path(target);
    let n = std::fs::copy(long_path(source), long_path(&tmp)).map_err(|e| {
        let _ = std::fs::remove_file(long_path(&tmp));
        io_err(source, e)
    })?;
    commit_temp(&tmp, target)?;
    Ok(n)
}

/// Verschiebt eine Datei (Rückfall über Laufwerke: kopieren, dann Quelle löschen).
pub fn move_file(source: &Path, target: &Path) -> Result<u64> {
    if let Some(dir) = target.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(long_path(dir)).map_err(|e| io_err(dir, e))?;
    }
    let n = std::fs::metadata(long_path(source))
        .map(|m| m.len())
        .unwrap_or(0);
    if std::fs::rename(long_path(source), long_path(target)).is_ok() {
        return Ok(n);
    }
    copy_atomic(source, target)?;
    std::fs::remove_file(long_path(source)).map_err(|e| io_err(source, e))?;
    Ok(n)
}

/// Übernimmt das Änderungsdatum der Quelle.
pub fn copy_modified_time(source: &Path, target: &Path) -> Result<()> {
    let t = std::fs::metadata(long_path(source))
        .and_then(|m| m.modified())
        .map_err(|e| io_err(source, e))?;
    let f = std::fs::OpenOptions::new()
        .write(true)
        .open(long_path(target))
        .map_err(|e| io_err(target, e))?;
    f.set_modified(t).map_err(|e| io_err(target, e))
}
