//! Datei-Ein-/Ausgabe für Skripte, MCP und CLI: Öffnen/Importieren über fp-io, flacher Export über
//! fp-export (Feature `formats`, Vorgabe an). Ohne das Feature gehen nur FW-PNG und PNG.
//!
//! Gemeinsam genutzt von fp-script (`openDocument`, `importFile`, `exportTo`), fp-mcp (`open`,
//! `export`) und firepixel-cli (`convert`, `run --out`, `batch`).

use fp_engine::{DocId, Session};
use fp_raster::RgbaImage;
use serde_json::Value;
use std::path::Path;

/// Endungen, die gelesen werden können.
pub fn import_extensions() -> &'static [&'static str] {
    if cfg!(feature = "formats") {
        &[
            "fw.png", "png", "apng", "psd", "gif", "jpg", "jpeg", "bmp", "tif", "tiff", "tga",
            "wbmp", "webp", "jxl", "qoi", "ico", "svg", "svgz", "ai", "eps",
        ]
    } else {
        &["fw.png", "png"]
    }
}

/// Endungen, in die flach exportiert werden kann.
pub fn export_extensions() -> &'static [&'static str] {
    if cfg!(feature = "formats") {
        &[
            "png", "gif", "jpg", "jpeg", "webp", "avif", "bmp", "tif", "tiff", "qoi", "ico", "apng",
        ]
    } else {
        &["png"]
    }
}

fn ext_of(path: &Path) -> String {
    let l = path.to_string_lossy().to_lowercase();
    if l.ends_with(".fw.png") {
        return "fw.png".into();
    }
    path.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

/// Öffnet eine Datei als neues aktives Dokument. FW-PNG und PNG über fp-engine (`file.open`),
/// andere Formate über fp-io (als unbenanntes Dokument, wie das Original beim Öffnen fremder Formate).
pub fn open_document(s: &mut Session, path: &Path) -> Result<DocId, String> {
    let ext = ext_of(path);
    if ext == "fw.png" || ext == "png" || !cfg!(feature = "formats") {
        let v = s
            .execute(
                "file.open",
                serde_json::json!({"path": path.to_string_lossy()}),
            )
            .map_err(|e| e.to_string())?;
        return v
            .value
            .get("doc")
            .and_then(|d| d.as_u64())
            .map(DocId)
            .ok_or_else(|| fp_i18n::t("err-script-open-failed"));
    }
    #[cfg(feature = "formats")]
    {
        let r = fp_io::open_path(path, &fp_io::ImportOptions::default())
            .map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(s.add_document(r.document, None))
    }
    #[cfg(not(feature = "formats"))]
    {
        Err(fp_i18n::tr!(
            "err-script-format-needs-io",
            ext = ext.to_string()
        ))
    }
}

/// Liest eine Bilddatei als Pixel (für Import in ein Dokument): PNG direkt, sonst über fp-io
/// (erste Ebene des importierten Dokuments flach gerechnet).
pub fn read_pixels(path: &Path) -> Result<(u32, u32, Vec<u8>), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if let Ok(chunks) = fp_fwpng::png::read_chunks(&bytes)
        && let Ok(img) = fp_fwpng::png::decode_image(&chunks)
    {
        return Ok((img.width, img.height, img.rgba));
    }
    #[cfg(feature = "formats")]
    {
        let r = fp_io::open_bytes(
            &path.to_string_lossy(),
            &bytes,
            &fp_io::ImportOptions::default(),
        )
        .map_err(|e| format!("{}: {e}", path.display()))?;
        let rnd = fp_render::Renderer::new(fp_render::RenderOptions {
            background: fp_render::BackgroundMode::Transparent,
            ..fp_render::RenderOptions::default()
        });
        let img = rnd.render_page(&r.document, 0, 0).image;
        Ok((
            img.width() as u32,
            img.height() as u32,
            img.as_bytes().to_vec(),
        ))
    }
    #[cfg(not(feature = "formats"))]
    {
        Err(fp_i18n::tr!(
            "err-script-png-only",
            path = path.display().to_string()
        ))
    }
}

/// Exportformat aus Endung bzw. Original-Optionen (`exportFormat`, `colorMode`, `jpegQuality` …).
#[derive(Clone, Debug, PartialEq)]
pub struct ExportSpec {
    /// Endung (png, gif, jpg, webp, avif, bmp, tiff, qoi, ico, apng).
    pub ext: String,
    /// Farbtiefe für PNG: 8, 24 oder 32.
    pub png_bits: u8,
    /// JPEG-/WebP-/AVIF-Qualität.
    pub quality: Option<i32>,
    /// Farbzahl für GIF/PNG8.
    pub colors: Option<i32>,
    /// F27 (TW-102/103): ohne `exportFormat`/`colorMode` gelten die Exporteinstellungen des Dokuments
    /// (Optimieren), wenn sie zur Endung passen – wie `exportDocumentAs(doc, url, null)` im Original.
    pub doc_opts: bool,
}

impl ExportSpec {
    /// Aus Zielpfad und optionalen Original-Exportoptionen.
    pub fn from(path: &Path, opts: &Value) -> ExportSpec {
        let mut ext = ext_of(path);
        if ext == "fw.png" {
            ext = "png".into();
        }
        if let Some(f) = opts.get("exportFormat").and_then(|v| v.as_str()) {
            let f = f.to_ascii_uppercase();
            ext = match f.as_str() {
                "GIF" => "gif",
                "GIF ANIMATION" | "ANIMATED GIF" => "agif",
                "JPEG" => "jpg",
                "PNG" => "png",
                "BMP" => "bmp",
                "TIFF" => "tiff",
                "WEBP" => "webp",
                _ => ext.as_str(),
            }
            .to_string();
        }
        let png_bits = match opts.get("colorMode").and_then(|v| v.as_str()) {
            Some(m) if m.starts_with('8') || m.contains("indexed") => 8,
            Some(m) if m.starts_with("24") => 24,
            _ => 32,
        };
        let num = |k: &str| {
            opts.get(k)
                .and_then(|v| {
                    v.as_f64()
                        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
                })
                .map(|x| x as i32)
        };
        let doc_opts = opts.get("exportFormat").is_none() && opts.get("colorMode").is_none();
        ExportSpec {
            ext,
            png_bits,
            quality: num("jpegQuality"),
            colors: num("numPaletteColors").or_else(|| num("numColors")),
            doc_opts,
        }
    }
}

/// Flaches Bild der Seite: PNG32 direkt, andere Formate über fp-export.
pub fn export_page(
    s: &Session,
    doc: DocId,
    page: Option<usize>,
    frame: Option<usize>,
    path: &Path,
    opts: &Value,
) -> Result<usize, String> {
    let d = s
        .document(doc)
        .ok_or_else(|| fp_i18n::t("err-script-doc-closed"))?;
    let page = page.unwrap_or(d.page);
    let frame = frame.unwrap_or(if page == d.page { d.frame } else { 0 });
    let spec = ExportSpec::from(path, opts);
    let bytes = encode(&d.doc, page, frame, &spec)?;
    if let Some(dir) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(path, &bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(bytes.len())
}

/// PNG32 wie der Export des Originals: nach `IHDR` folgen `sBIT` und `pHYs` aus der Auflösung der Seite
/// (`floor(trunc(dpi) / 0,0254)`, BA-035; `exportTo`/`exportDocumentAs` per JSF schreiben sie wie der Export).
fn png32_with_phys(img: &RgbaImage, page: Option<&fp_model::Page>) -> Result<Vec<u8>, String> {
    use fp_fwpng::png as pngc;
    let extra = match page {
        Some(p) => vec![
            pngc::Chunk::new(b"sBIT", vec![8, 8, 8, 8]),
            pngc::make_phys(p.canvas.resolution, p.canvas.resolution_unit),
        ],
        None => Vec::new(),
    };
    // ein Kodierer für alle PNG-Wege: Zeilenfilter wie libpng, zlib 6 (Q-37)
    pngc::encode_rgba8(
        img.width() as u32,
        img.height() as u32,
        img.as_bytes(),
        &extra,
    )
    .map_err(|e| e.to_string())
}

/// Kodiert eine Seite im gewünschten Format.
pub fn encode(
    doc: &fp_model::Document,
    page: usize,
    frame: usize,
    spec: &ExportSpec,
) -> Result<Vec<u8>, String> {
    let renderer = fp_render::Renderer::default();
    if spec.ext == "png" && spec.png_bits == 32 && doc_override(doc, page, spec).is_none() {
        let img = renderer.render_page(doc, page, frame).image;
        if img.is_empty() {
            return Err(fp_i18n::t("err-script-page-empty"));
        }
        return png32_with_phys(&img, doc.pages.get(page).map(|p| &**p));
    }
    #[cfg(feature = "formats")]
    {
        use fp_export::{ColorMode, ExportFormat};
        let mut s = fp_export::doc_export::page_settings(doc, page);
        s.format = match spec.ext.as_str() {
            "gif" => ExportFormat::Gif,
            "agif" => ExportFormat::AnimatedGif,
            "jpg" | "jpeg" => ExportFormat::Jpeg,
            "png" => ExportFormat::Png,
            "webp" => ExportFormat::WebP,
            "avif" => ExportFormat::Avif,
            "bmp" => ExportFormat::Bmp,
            "tif" | "tiff" => ExportFormat::Tiff,
            "qoi" => ExportFormat::Qoi,
            "ico" => ExportFormat::Ico,
            "apng" => ExportFormat::Apng,
            other => {
                return Err(fp_i18n::tr!(
                    "err-script-export-unsupported",
                    ext = other.to_string()
                ));
            }
        };
        s.color_mode = match (s.format, spec.png_bits) {
            (ExportFormat::Png, 8) | (ExportFormat::Gif | ExportFormat::AnimatedGif, _) => {
                ColorMode::Indexed
            }
            (ExportFormat::Png, 24) | (ExportFormat::Jpeg, _) => ColorMode::Rgb24,
            _ => ColorMode::Rgba32,
        };
        if let Some((f, m)) = doc_override(doc, page, spec) {
            s.format = f;
            s.color_mode = m;
        }
        if let Some(q) = spec.quality {
            s.jpeg.quality = q.clamp(1, 100);
            s.webp.quality = q.clamp(0, 100);
            s.avif.quality = q.clamp(1, 100);
        }
        if let Some(c) = spec.colors {
            s.max_colors = c.clamp(2, 256);
        }
        let opaque = fp_render::Renderer::default();
        let transparent = fp_render::Renderer::new(fp_render::RenderOptions {
            background: fp_render::BackgroundMode::Transparent,
            ..fp_render::RenderOptions::default()
        });
        let render = |d: &fp_model::Document,
                      req: &fp_export::render::RenderRequest|
         -> Result<RgbaImage, String> {
            let r = match req.background {
                fp_export::render::Background::Transparent => &transparent,
                _ => &opaque,
            };
            let area = req
                .area
                .or_else(|| fp_render::Renderer::page_rect(d, req.page))
                .ok_or_else(|| fp_i18n::t("err-script-page-missing"))?;
            Ok(r.render_rect(d, req.page, req.frame, area).image)
        };
        let _ = frame;
        fp_export::doc_export::export_whole(
            &render,
            doc,
            page,
            &s,
            &fp_export::export::EncodeExtras::standalone(),
        )
        .map_err(|e| e.to_string())
    }
    #[cfg(not(feature = "formats"))]
    {
        let _ = (renderer, frame);
        Err(fp_i18n::tr!(
            "err-script-export-needs-feature",
            ext = spec.ext.to_string()
        ))
    }
}

/// F27 (TW-102/103): Format und Farbtiefe aus den Exporteinstellungen der Seite, wenn keine
/// Optionen übergeben wurden und die Einstellung zur Endung passt (PNG 8/24/32 bei `.png`, GIF oder
/// animiertes GIF bei `.gif`). Sonst `None` (Endung entscheidet wie bisher).
/// Ob ein Export ohne Optionen nach `path` die Exporteinstellungen der Seite nimmt (F27).
pub fn uses_doc_settings(doc: &fp_model::Document, page: usize, path: &Path) -> bool {
    doc_override(doc, page, &ExportSpec::from(path, &Value::Null)).is_some()
}

#[cfg(feature = "formats")]
fn doc_override(
    doc: &fp_model::Document,
    page: usize,
    spec: &ExportSpec,
) -> Option<(fp_export::ExportFormat, fp_export::ColorMode)> {
    use fp_export::{ColorMode, ExportFormat};
    if !spec.doc_opts {
        return None;
    }
    let s = fp_export::doc_export::page_settings(doc, page);
    match (spec.ext.as_str(), s.format) {
        ("png", ExportFormat::Png) if s.color_mode != ColorMode::Rgba32 => {
            Some((s.format, s.color_mode))
        }
        ("gif", ExportFormat::AnimatedGif) => Some((s.format, ColorMode::Indexed)),
        _ => None,
    }
}

#[cfg(not(feature = "formats"))]
fn doc_override(_: &fp_model::Document, _: usize, _: &ExportSpec) -> Option<()> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_from_options() {
        let s = ExportSpec::from(
            Path::new("a.png"),
            &serde_json::json!({"exportFormat": "GIF", "numPaletteColors": 16}),
        );
        assert_eq!(s.ext, "gif");
        assert_eq!(s.colors, Some(16));
        assert_eq!(
            ExportSpec::from(Path::new("b.fw.png"), &Value::Null).ext,
            "png"
        );
        assert_eq!(
            ExportSpec::from(
                Path::new("c.png"),
                &serde_json::json!({"colorMode": "8 bit"})
            )
            .png_bits,
            8
        );
    }
}
