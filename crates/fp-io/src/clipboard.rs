//! Zwischenablage (12 §6): Bildformate dekodieren und die Auswertungsreihenfolge nach
//! `PasteImportOrder` bestimmen. Der Zugriff auf die Windows-Zwischenablage selbst liegt in der
//! Oberfläche; hier kommen nur (Formatname, Bytes)-Paare an.
//!
//! Unterstützt: `PNG`/`kMoaCfFormat_PNG`/`image/png` (FW-PNG wird erkannt), `CF_DIB` (8),
//! `CF_DIBV5` (17), `kMoaCfFormat_BMP`, `kMoaCfFormat_GIF`, `kMoaCfFormat_JPEG`, `kMoaCfFormat_TIFF`,
//! `image/svg+xml` sowie Dateilisten (Pfade, über [`crate::open_path`]).

use crate::build::{document_from_raster, import_format, ImportResult, ImportedContent};
use crate::detect::{detect_format, FileFormat};
use crate::{ImportError, ImportOptions, Result};

/// Bekannte Formate der Zwischenablage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipFormat {
    /// PNG-Datenstrom (auch FW-PNG).
    Png,
    /// `CF_DIB` (BITMAPINFOHEADER + Pixel).
    Dib,
    /// `CF_DIBV5` (BITMAPV5HEADER + Pixel, mit Alpha).
    DibV5,
    /// Vollständige Bilddatei (GIF, JPEG, TIFF, BMP …).
    FileData,
    /// SVG-Text.
    Svg,
    /// Unbekannt.
    Other,
}

/// Formatname → Art (Windows-Namen wie das Original sie registriert).
pub fn classify(name: &str) -> ClipFormat {
    match name {
        "PNG" | "kMoaCfFormat_PNG" | "image/png" => ClipFormat::Png,
        "CF_DIB" | "8" | "#8" | "kMoaCfFormat_BMP" => ClipFormat::Dib,
        "CF_DIBV5" | "17" | "#17" => ClipFormat::DibV5,
        "kMoaCfFormat_GIF" | "kMoaCfFormat_JPEG" | "kMoaCfFormat_TIFF" | "kMoaCfFormat_TGA" | "kMoaCfFormat_WBMP" | "image/gif" | "image/jpeg" | "image/webp" => ClipFormat::FileData,
        "image/svg+xml" => ClipFormat::Svg,
        _ => ClipFormat::Other,
    }
}

/// Inhalt eines Zwischenablageformats importieren.
pub fn import_clipboard(format: &str, data: &[u8], opts: &ImportOptions) -> Result<ImportResult> {
    let mut o = opts.clone();
    o.importing = true;
    match classify(format) {
        ClipFormat::Dib | ClipFormat::DibV5 => {
            // CF_DIB kann auch mit „BM“-Kopf kommen
            let img = if data.starts_with(b"BM") { crate::bmp::decode(data)? } else { crate::bmp::decode_dib(data, None, true)? };
            Ok(ImportResult { document: document_from_raster(img, FileFormat::Bmp), format: FileFormat::Bmp, content: ImportedContent::Bitmap, warnings: vec![] })
        }
        ClipFormat::Png | ClipFormat::FileData | ClipFormat::Svg => {
            let fmt = detect_format(data, if classify(format) == ClipFormat::Svg { "clip.svg" } else { "" });
            import_format(fmt, format, data, &o)
        }
        ClipFormat::Other => Err(ImportError::Unsupported(fp_i18n::tr!("err-import-clipboard-format", name = format.to_string()))),
    }
}

/// Wählt aus den angebotenen Formaten das erste nach `PasteImportOrder` (Tokens wie
/// [`crate::detect::paste_order`]; `#ID` oder Name). Ohne Treffer: erstes unterstütztes Format.
pub fn choose_format<'a>(available: &[&'a str], order: &[String]) -> Option<&'a str> {
    for tok in order {
        let t = tok.trim_start_matches('#');
        if let Some(f) = available.iter().find(|f| **f == t || **f == tok.as_str())
            && classify(f) != ClipFormat::Other {
                return Some(f);
            }
    }
    let rank = |f: &str| match classify(f) {
        ClipFormat::Png => 0,
        ClipFormat::DibV5 => 1,
        ClipFormat::Dib => 2,
        ClipFormat::FileData => 3,
        ClipFormat::Svg => 4,
        ClipFormat::Other => 9,
    };
    available.iter().filter(|f| classify(f) != ClipFormat::Other).min_by_key(|f| rank(f)).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn choose() {
        let order = crate::detect::paste_order("", &["kMoaCfFormat_PNG", "CF_DIB"]);
        assert_eq!(choose_format(&["CF_DIB", "kMoaCfFormat_PNG"], &order), Some("kMoaCfFormat_PNG"));
        assert_eq!(choose_format(&["CF_DIB", "Text"], &[]), Some("CF_DIB"));
        assert_eq!(choose_format(&["Text"], &[]), None);
        assert!(import_clipboard("Text", b"x", &ImportOptions::default()).is_err());
    }
}
