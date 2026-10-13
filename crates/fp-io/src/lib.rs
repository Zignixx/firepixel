//! Import fremder Formate wie das Original (Spezifikation 12) und moderne Formate (ARCHITEKTUR §5).
//!
//! # Überblick
//!
//! | Modul | Inhalt | Spezifikation / Referenz |
//! |---|---|---|
//! | [`detect`] | Formaterkennung am Inhalt (Rückfall Endung), Leser-Registry, Dateifilter, `PasteImportOrder` | 12 §1, `detect.py` |
//! | [`raster`] | Zwischenformat Bild/Animation | – |
//! | [`build`] | Dokumente aus Bildern/Animationen (Ebene „Hintergrund“, Status, Exporteinstellungen) | 12 §5 |
//! | [`gif`] | GIF inkl. Animation (Verzögerung, Löschmethode, Schleife, Erweitern) | 12 §5.1, `gif.py` |
//! | [`png_in`] | PNG und APNG (`.fw.png` geht an `fp-fwpng`) | 12 §5.3 |
//! | [`jpeg_in`] | JPEG (CMYK naiv wie das Original), Auflösung aus JFIF/EXIF | 12 §5.2, 7a T7 |
//! | [`bmp`], [`tga`], [`wbmp`], [`tiff_in`] | Plug-in-Formate (TIFF-CMYK über die Original-CMYK-Tabelle) | 12 §5.4, 7a T7 |
//! | [`psd`] | Photoshop: Ebenen, Gruppen, Masken, Schnittmasken, Einstellungsebenen, Effekte | 12 §3, `psd.py`, `psd_map.py` |
//! | [`vector`] | SVG (usvg → Pfade), Illustrator ≤ 8 / EPS (soweit spezifiziert) | 12 §4, ARCHITEKTUR §5 |
//! | [`modern`] | WebP (auch animiert), JPEG XL, QOI, ICO (alle Größen) | ARCHITEKTUR §5 |
//! | [`avif_in`], [`heic_in`], [`isobmff`] | AVIF (rav1d, auch Raster/Alpha/Animation), HEIC/HEIF (heic-decoder), HEIF-Container | ARCHITEKTUR §5 |
//! | [`clipboard`] | Zwischenablage: PNG, DIB/DIBV5, Dateiinhalte | 12 §6 |
//! | [`pe`] | Ressourcen einer vorhandenen Installation des Originals zur Laufzeit (CMYK-Tabelle), nichts eingebettet | 7a T7 |
//!
//! Einstieg: [`open_bytes`] (Datei öffnen ⇒ neues Dokument) und [`import_bytes`] (Inhalt für den Import in
//! ein bestehendes Dokument). Beschädigte Dateien liefern [`ImportError`] statt einer Panik.

pub mod avif_in;
pub mod bmp;
pub mod build;
pub mod clipboard;
pub mod detect;
pub mod gif;
pub mod heic_in;
pub mod isobmff;
pub mod jpeg_ijg;
pub mod jpeg_in;
pub mod modern;
pub mod pe;
pub mod png_in;
pub mod psd;
pub mod raster;
pub mod tga;
pub mod tiff_in;
pub mod vector;
pub mod wbmp;

pub use build::{ImportResult, ImportedContent};
pub use detect::{detect_format, FileFormat};
pub use raster::{Animation, AnimationFrame, RasterImage};

use std::sync::Arc;

/// Fehler beim Import.
#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    /// Format nicht erkannt.
    #[error("{}", m0("err-import-unknown-format"))]
    UnknownFormat,
    /// Format erkannt, aber nicht unterstützt (mit Begründung).
    #[error("{}", m1("err-import-unsupported", "detail", .0))]
    Unsupported(String),
    /// Datei beschädigt bzw. unvollständig.
    #[error("{}", m1("err-import-corrupt", "detail", .0))]
    Corrupt(String),
    /// Bild zu groß.
    #[error("{}", m2("err-import-too-large", "w", .0, "h", .1))]
    TooLarge(u32, u32),
    /// Fehler eines Fremd-Dekoders.
    #[error("{}", m2("err-import-decode", "codec", .0, "detail", .1))]
    Decode(&'static str, String),
    /// FW-PNG-Fehler.
    #[error("{}", m1("err-import-fwpng", "detail", .0))]
    FwPng(#[from] fp_fwpng::FwError),
    /// Ein-/Ausgabe.
    #[error("{}", m1("err-import-io", "detail", .0))]
    Io(#[from] std::io::Error),
}

// ---- Mehrsprachige Meldungen (fp-i18n, `messages.ftl`) ----
fn m0(key: &str) -> String {
    fp_i18n::t(key)
}
/// Strukturteil einer Datei in Fehlermeldungen der Dekoder: Schlüssel (`err-import-part-*`) → Text
/// in der aktuellen Sprache, sonst unverändert.
pub(crate) fn part(s: &str) -> String {
    if s.starts_with("err-") { fp_i18n::t(s) } else { s.to_string() }
}

fn m1(key: &str, a: &str, va: &dyn std::fmt::Display) -> String {
    fp_i18n::t_args(key, &[(a, fp_i18n::Value::from(va.to_string()))])
}
fn m2(key: &str, a: &str, va: &dyn std::fmt::Display, b: &str, vb: &dyn std::fmt::Display) -> String {
    fp_i18n::t_args(key, &[(a, fp_i18n::Value::from(va.to_string())), (b, fp_i18n::Value::from(vb.to_string()))])
}

/// Ergebnis-Typ der Import-Funktionen.
pub type Result<T> = std::result::Result<T, ImportError>;

/// Obergrenze der Pixelzahl (wie `fp-raster`).
pub const MAX_PIXELS: u64 = 1 << 28;

/// Prüft eine Bildgröße.
pub fn check_size(w: u32, h: u32) -> Result<()> {
    if w == 0 || h == 0 {
        return Err(ImportError::Corrupt(fp_i18n::tr!("err-import-image-size", w = w.to_string(), h = h.to_string())));
    }
    if w as u64 * h as u64 > MAX_PIXELS {
        return Err(ImportError::TooLarge(w, h));
    }
    Ok(())
}

/// Importoptionen (Voreinstellungen aus 12 §3.2, §5.1 und Firepixel-Erweiterungen).
#[derive(Clone, Debug)]
pub struct ImportOptions {
    /// Öffnen (neues Dokument) oder Importieren (Inhalt in ein bestehendes Dokument).
    pub importing: bool,
    /// Photoshop-Optionen.
    pub psd: psd::PsdOptions,
    /// `ExpandAnimatedGifFrames` (Vorgabe aus).
    pub expand_gif_frames: bool,
    /// CMYK-Tabelle des Originals (Ressource `CMYK/1001`), zur Laufzeit geladen; ohne Tabelle `255 − C`.
    pub cmyk_table: Option<Arc<fp_color::cmyk::CmykTable>>,
    /// Bei ICO: jede Größe als eigene Seite (sonst nur die größte).
    pub ico_all_sizes: bool,
}

impl Default for ImportOptions {
    fn default() -> Self {
        ImportOptions { importing: false, psd: psd::PsdOptions::default(), expand_gif_frames: false, cmyk_table: None, ico_all_sizes: true }
    }
}

impl ImportOptions {
    /// Wie [`Default`], aber mit der CMYK-Tabelle aus [`pe::load_cmyk_table_default`].
    pub fn with_installation() -> ImportOptions {
        ImportOptions { cmyk_table: pe::load_cmyk_table_default().map(Arc::new), ..ImportOptions::default() }
    }
}

/// Datei öffnen: erkennt das Format am Inhalt (Rückfall Endung) und liefert ein neues Dokument.
pub fn open_bytes(name: &str, data: &[u8], opts: &ImportOptions) -> Result<ImportResult> {
    let fmt = detect_format(data, name);
    let mut o = opts.clone();
    o.importing = false;
    build::import_format(fmt, name, data, &o)
}

/// Datei importieren: wie [`open_bytes`], aber nach den Import-Regeln (z. B. PSD ohne
/// Hintergrundebene, sofern nicht eingeschlossen; animiertes GIF als Animationssymbol).
pub fn import_bytes(name: &str, data: &[u8], opts: &ImportOptions) -> Result<ImportResult> {
    let fmt = detect_format(data, name);
    let mut o = opts.clone();
    o.importing = true;
    build::import_format(fmt, name, data, &o)
}

/// Datei vom Datenträger öffnen.
pub fn open_path(path: &std::path::Path, opts: &ImportOptions) -> Result<ImportResult> {
    let data = std::fs::read(path)?;
    open_bytes(&path.to_string_lossy(), &data, opts)
}
