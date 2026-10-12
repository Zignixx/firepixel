//! „Reduziertes PNG“ aus *Datei ▸ Speichern unter* (`re\20-ai8swf.md` §5): die aktuelle Seite als
//! normales PNG mit den Optimierungseinstellungen des Dokuments, das Dateiformat aber immer PNG.
//!
//! - PNG 8/24/32 bleibt wie eingestellt; GIF und animiertes GIF werden PNG 8 mit derselben
//!   Paletteneinstellung (Farbanzahl, Palette, Dither, Transparenz); alle übrigen Formate PNG 32
//!   (Vorschlag der Spezifikation, nicht beobachtet).
//! - Chunks wie beim PNG-Export (`IHDR sBIT [PLTE] [tRNS] pHYs tEXt tEXt IDAT IEND`), `tEXt` in der
//!   Reihenfolge `Creation Time` (Erstellungsdatum des Dokuments), `Software`.
//! - Keine privaten Chunks des FW-PNG und kein XMP.

use fp_raster::RgbaImage;

use crate::options::{ColorMode, ExportFormat, OptimizeSettings};
use crate::pipeline::{image_pixels, optimize_pixels, ExportContext, Optimized};
use crate::png_out::{write_png8, write_png_truecolor, PngOptions};
use crate::{ExportError, Result};

/// Name im `Software`-Text.
pub const SOFTWARE: &str = "Firepixel";
/// PNG-Schlüsselwort des Erstellungsdatums.
pub const KEY_CREATION_TIME: &str = "Creation Time";
/// PNG-Schlüsselwort des Programmnamens.
pub const KEY_SOFTWARE: &str = "Software";

/// Optimierungseinstellungen des Dokuments → Einstellungen des reduzierten PNG (Format immer PNG).
pub fn flattened_png_settings(doc: &OptimizeSettings) -> OptimizeSettings {
    let mut s = doc.clone();
    match doc.format {
        ExportFormat::Png => {}
        ExportFormat::Gif | ExportFormat::AnimatedGif => {
            s.format = ExportFormat::Png;
            s.color_mode = ColorMode::Indexed;
            // GIF-Zeilensprung ist kein PNG-Zeilensprung
            s.interlaced = false;
        }
        ExportFormat::Apng => s.format = ExportFormat::Png,
        _ => {
            s.format = ExportFormat::Png;
            s.color_mode = ColorMode::Rgba32;
            s.interlaced = false;
        }
    }
    s
}

/// Kurzname der Abbildung für Meldungen und Tests: `"png8"`, `"png24"` oder `"png32"`.
pub fn flattened_png_kind(doc: &OptimizeSettings) -> &'static str {
    match flattened_png_settings(doc).color_mode {
        ColorMode::Indexed => "png8",
        ColorMode::Rgb24 => "png24",
        ColorMode::Rgba32 => "png32",
    }
}

/// Erstellungsdatum im Format des `Creation Time`-Texts (`MM/DD/YY`, wie das Original im
/// Systemformat; Firepixel schreibt es fest so, weil es ein technischer Text ist).
pub fn creation_time_text(year: i64, month: u32, day: u32) -> String {
    format!("{month:02}/{day:02}/{:02}", year.rem_euclid(100))
}

/// Heutiges Datum (UTC) als `Creation Time`-Text.
pub fn creation_time_today() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) as i64;
    let days = secs.div_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    creation_time_text(y, m, d)
}

/// Schreibt ein gerendertes Seitenbild als reduziertes PNG. `doc_settings` sind die
/// Optimierungseinstellungen des Dokuments, `creation_time` der Text für `Creation Time`.
pub fn write_flattened_png(img: &RgbaImage, doc_settings: &OptimizeSettings, ctx: &ExportContext, creation_time: &str) -> Result<Vec<u8>> {
    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 {
        return Err(ExportError::BadSize(w, h));
    }
    let s = flattened_png_settings(doc_settings);
    let optimized = optimize_pixels(&image_pixels(img), w, h, &s, ctx);
    // technische PNG-Schlüsselwörter (nicht übersetzt), Reihenfolge wie im Original
    let chunks = vec![(KEY_CREATION_TIME.to_string(), creation_time.to_string()), (KEY_SOFTWARE.to_string(), SOFTWARE.to_string())];
    let opts = PngOptions { interlaced: s.interlaced || ctx.png_adam7, dpi: Some(ctx.resolution_dpi), text: chunks, level: 6 };
    match &optimized {
        Optimized::Indexed(ix) => write_png8(ix, s.effective_matte(ctx.canvas), &opts),
        Optimized::Truecolor { width, height, pixels, alpha } => write_png_truecolor(pixels, *width, *height, *alpha, &opts),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn datum_wie_im_original() {
        assert_eq!(creation_time_text(2026, 10, 12), "10/12/26");
        assert_eq!(creation_time_text(2006, 10, 17), "10/17/06");
        assert_eq!(creation_time_today().len(), 8);
    }
}
