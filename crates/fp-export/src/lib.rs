//! Export von Firepixel wie das Original (Spezifikationen 08 Export und 15 Verhalten/Pop-up).
//!
//! # Überblick
//!
//! | Modul | Inhalt | Spezifikation / Referenz |
//! |---|---|---|
//! | [`options`] | Optimieren-Einstellungen (typisierte Sicht auf `EXPv`), Vorgaben | 08 §1 |
//! | [`palettes`] | feste Paletten: Einheitlich/Web 216, Graustufen, Mac, Windows, Exakt | 08 §2.2 |
//! | [`quant`] | Variance-Quantisierer, adaptive Palette, Zellensuche, Diffusion/2×2/LZW-Glättung | 08 §2.3–3, `fw_palette.py` |
//! | [`rgba`] | RGBA-Palette (PNG8 mit Alpha), Sortierung, bearbeitete/unbenutzte Farben | 08 §2.6–2.7, `fw_rgba.py` |
//! | [`pipeline`] | Optimieren als reine Funktion (Bild → Indexbild bzw. Truecolor) | 08 §2.1 |
//! | [`gif`] | GIF89a-Schreiber im Original-Aufbau, LZW, verlustbehaftetes GIF | 08 §5.2, `fw_gif.py` |
//! | [`png_out`] | PNG8 (mit `tRNS`), PNG24/32, Adam7, APNG | 08 §5.1 |
//! | [`jpeg`], [`jpeg_tables`] | eigener JPEG-Kodierer (IJG-Rechenweg, Original-Viertelstufen, selektives JPEG, progressiv); Tabellen zur Laufzeit aus Haupt-EXE | 08 §4 |
//! | [`formats`] | TIFF, BMP, WBMP, QOI, ICO | 08 §5.4 |
//! | [`modern`] | WebP (verlustbehaftet/-frei, animiert), AVIF | ARCHITEKTUR §5 |
//! | [`animation`] | Status → animiertes GIF (Auto-Crop, Differenzbilder), APNG, animiertes WebP | 08 §5.3 |
//! | [`export`] | `export_image`/`encode`: optimiertes Bild in Bytes, Vorschau | 08 §2–5, §7 |
//! | [`preview`] | Vorschau-Daten: Dateigröße, Ladezeit je Verbindung, JPEG-Rückdekodierung | 08 §7 |
//! | [`size`] | Auf Größe optimieren | 08 §7 |
//! | [`render`] | schmale Renderer-Schnittstelle [`render::PageRenderer`] (fp-render/fp-engine implementieren sie) | – |
//! | [`slices`] | Slice-Raster, Polygon-Hüllen, automatische Namen, Hotspots | 08 §6.1–6.2, 15 §1 |
//! | [`html`] | HTML-Tabellen (Abstandhalter/verschachtelt), Image-Maps, CSS-Ebenen, eigene Vorlagen | 08 §6.3 |
//! | [`behavior`], [`popup`] | Verhalten (Rollover, Austauschbild, Statusleiste, Pop-up) und Pop-up-Menüs (CSS/HTML/JS), eigene Laufzeit-Skripte | 15 |
//! | [`doc_export`] | Dokument-Export: je Seite/Status/Ebene, Slices + HTML, Dateinamen | 08 §1.5, §6.4 |
//! | [`psd_out`], [`vector_out`] | PSD (Ebenen), PDF, SVG | 08 §5.4 |
//! | [`sprites`] | CSS-Sprites | ARCHITEKTUR §5 |
//! | [`px`] | ARGB-Hilfen (Matte, Websnap) | – |
//!
//! Einstieg: [`export::export_image`] (ein Bild) bzw. [`doc_export::export_document`] (Dokument mit
//! Slices/HTML). Alle Funktionen sind frei von `unsafe` und geben bei ungültigen Eingaben Fehler statt
//! Paniken zurück. Keine Vorlagen oder Bilder des Originals: HTML/JS/CSS stammen aus eigenen Vorlagen.

pub mod animation;
pub mod behavior;
pub mod doc_export;
pub mod export;
pub mod formats;
pub mod gif;
pub mod html;
pub mod jpeg;
pub mod jpeg_tables;
pub mod modern;
pub mod options;
pub mod palette_edit;
pub mod palettes;
pub mod pipeline;
pub mod popup;
pub mod popup_images;
pub mod png_out;
pub mod preview;
pub mod psd_out;
pub mod px;
pub mod quant;
pub mod render;
pub mod rgba;
pub mod size;
pub mod slices;
pub mod sprites;
mod tables_data;
pub mod vector_out;

pub use options::{ColorMode, Dither, ExportFormat, OptimizeSettings, PaletteKind, Sorting, Transparency};
pub use pipeline::{optimize, ExportContext, IndexedImage, Optimized};

/// Fehler beim Export.
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    /// Bild zu groß oder leer für das Format.
    #[error("{}", m2("err-export-bad-size", "w", .0, "h", .1))]
    BadSize(usize, usize),
    /// Format wird nicht geschrieben.
    #[error("{}", m1("err-export-unsupported", "detail", .0))]
    Unsupported(String),
    /// Kodierfehler eines Fremd-Kodierers.
    #[error("{}", m1("err-export-encode", "detail", .0))]
    Encode(String),
    /// Renderer fehlgeschlagen.
    #[error("{}", m1("err-export-render", "detail", .0))]
    Render(String),
    /// Ein-/Ausgabefehler.
    #[error("{}", m1("err-export-io", "detail", .0))]
    Io(#[from] std::io::Error),
}

// ---- Mehrsprachige Meldungen (fp-i18n, `meldungen.ftl`) ----
fn m1(key: &str, a: &str, va: &dyn std::fmt::Display) -> String {
    fp_i18n::t_args(key, &[(a, fp_i18n::Value::from(va.to_string()))])
}
fn m2(key: &str, a: &str, va: &dyn std::fmt::Display, b: &str, vb: &dyn std::fmt::Display) -> String {
    fp_i18n::t_args(key, &[(a, fp_i18n::Value::from(va.to_string())), (b, fp_i18n::Value::from(vb.to_string()))])
}

/// Ergebnis-Typ der Export-Funktionen.
pub type Result<T> = std::result::Result<T, ExportError>;
