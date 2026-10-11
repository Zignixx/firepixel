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
        &["fw.png", "png", "apng", "psd", "gif", "jpg", "jpeg", "bmp", "tif", "tiff", "tga", "wbmp", "webp", "jxl", "qoi", "ico", "svg", "svgz", "ai", "eps"]
    } else {
        &["fw.png", "png"]
    }
}

/// Endungen, in die flach exportiert werden kann.
pub fn export_extensions() -> &'static [&'static str] {
    if cfg!(feature = "formats") {
        &["png", "gif", "jpg", "jpeg", "webp", "avif", "bmp", "tif", "tiff", "qoi", "ico", "apng"]
    } else {
        &["png"]
    }
}

fn ext_of(path: &Path) -> String {
    let l = path.to_string_lossy().to_lowercase();
    if l.ends_with(".fw.png") {
        return "fw.png".into();
    }
    path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default()
}

/// Öffnet eine Datei als neues aktives Dokument. FW-PNG und PNG über fp-engine (`file.open`),
/// andere Formate über fp-io (als unbenanntes Dokument, wie das Original beim Öffnen fremder Formate).
pub fn open_document(s: &mut Session, path: &Path) -> Result<DocId, String> {
    let ext = ext_of(path);
    if ext == "fw.png" || ext == "png" || !cfg!(feature = "formats") {
        let v = s.execute("file.open", serde_json::json!({"path": path.to_string_lossy()})).map_err(|e| e.to_string())?;
        return v.value.get("doc").and_then(|d| d.as_u64()).map(DocId).ok_or_else(|| "Öffnen fehlgeschlagen".into());
    }
    #[cfg(feature = "formats")]
    {
        let r = fp_io::open_path(path, &fp_io::ImportOptions::default()).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(s.add_document(r.document, None))
    }
    #[cfg(not(feature = "formats"))]
    {
        Err(format!("Format .{ext} wird ohne fp-io nicht gelesen"))
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
        let r = fp_io::open_bytes(&path.to_string_lossy(), &bytes, &fp_io::ImportOptions::default()).map_err(|e| format!("{}: {e}", path.display()))?;
        let rnd = fp_render::Renderer::new(fp_render::RenderOptions { background: fp_render::BackgroundMode::Transparent, ..fp_render::RenderOptions::default() });
        let img = rnd.render_page(&r.document, 0, 0).image;
        Ok((img.width() as u32, img.height() as u32, img.as_bytes().to_vec()))
    }
    #[cfg(not(feature = "formats"))]
    {
        Err(format!("{}: nur PNG ohne fp-io", path.display()))
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
        let num = |k: &str| opts.get(k).and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))).map(|x| x as i32);
        ExportSpec { ext, png_bits, quality: num("jpegQuality"), colors: num("numPaletteColors").or_else(|| num("numColors")) }
    }
}

/// Flaches Bild der Seite: PNG32 direkt, andere Formate über fp-export.
pub fn export_page(s: &Session, doc: DocId, page: Option<usize>, frame: Option<usize>, path: &Path, opts: &Value) -> Result<usize, String> {
    let d = s.document(doc).ok_or("Dokument ist geschlossen")?;
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

fn png32(img: &RgbaImage) -> Result<Vec<u8>, String> {
    let (w, h) = (img.width() as u32, img.height() as u32);
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, w.max(1), h.max(1));
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut wr = enc.write_header().map_err(|e| e.to_string())?;
        wr.write_image_data(img.as_bytes()).map_err(|e| e.to_string())?;
        wr.finish().map_err(|e| e.to_string())?;
    }
    Ok(out)
}

/// Kodiert eine Seite im gewünschten Format.
pub fn encode(doc: &fp_model::Document, page: usize, frame: usize, spec: &ExportSpec) -> Result<Vec<u8>, String> {
    let renderer = fp_render::Renderer::default();
    if spec.ext == "png" && spec.png_bits == 32 {
        let img = renderer.render_page(doc, page, frame).image;
        if img.is_empty() {
            return Err("Seite ist leer oder fehlt".into());
        }
        return png32(&img);
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
            other => return Err(format!("Exportformat .{other} wird nicht unterstützt")),
        };
        s.color_mode = match (s.format, spec.png_bits) {
            (ExportFormat::Png, 8) | (ExportFormat::Gif | ExportFormat::AnimatedGif, _) => ColorMode::Indexed,
            (ExportFormat::Png, 24) | (ExportFormat::Jpeg, _) => ColorMode::Rgb24,
            _ => ColorMode::Rgba32,
        };
        if let Some(q) = spec.quality {
            s.jpeg.quality = q.clamp(1, 100);
            s.webp.quality = q.clamp(0, 100);
            s.avif.quality = q.clamp(1, 100);
        }
        if let Some(c) = spec.colors {
            s.max_colors = c.clamp(2, 256);
        }
        let opaque = fp_render::Renderer::default();
        let transparent = fp_render::Renderer::new(fp_render::RenderOptions { background: fp_render::BackgroundMode::Transparent, ..fp_render::RenderOptions::default() });
        let render = |d: &fp_model::Document, req: &fp_export::render::RenderRequest| -> Result<RgbaImage, String> {
            let r = match req.background {
                fp_export::render::Background::Transparent => &transparent,
                _ => &opaque,
            };
            let area = req.area.or_else(|| fp_render::Renderer::page_rect(d, req.page)).ok_or("Seite fehlt")?;
            Ok(r.render_rect(d, req.page, req.frame, area).image)
        };
        let _ = frame;
        fp_export::doc_export::export_whole(&render, doc, page, &s, &fp_export::export::EncodeExtras::standalone()).map_err(|e| e.to_string())
    }
    #[cfg(not(feature = "formats"))]
    {
        let _ = (renderer, frame);
        Err(format!("Exportformat .{} braucht fp-export (Feature „formats“)", spec.ext))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_from_options() {
        let s = ExportSpec::from(Path::new("a.png"), &serde_json::json!({"exportFormat": "GIF", "numPaletteColors": 16}));
        assert_eq!(s.ext, "gif");
        assert_eq!(s.colors, Some(16));
        assert_eq!(ExportSpec::from(Path::new("b.fw.png"), &Value::Null).ext, "png");
        assert_eq!(ExportSpec::from(Path::new("c.png"), &serde_json::json!({"colorMode": "8 bit"})).png_bits, 8);
    }
}
