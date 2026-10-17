//! Farben für Firepixel.
//!
//! Verbindliche Grundtypen (Signaturen nicht ändern, nur erweitern): [`Argb`] (Dateiformat, nicht
//! vormultipliziert), [`Rgba8`] (Pixel, nicht vormultipliziert), [`Opacity`], [`BlendMode`].
//!
//! # Überblick
//!
//! | Modul | Inhalt | Spezifikation / Referenz |
//! |---|---|---|
//! | [`round`] | Rundungsarten, `mul255`, Divisionstabelle [`round::DIV`] | 14 §1.7, 07 §7.2 |
//! | [`convert`] | Hex-Notation, Vormultiplikation, Hilfen für [`Argb`]/[`Rgba8`] | 14 §1.12 |
//! | [`models`] | Farbmischer (RGB/Hex/CMY/HSB/Grau), Byte-HSB, Info-Bedienfeld, Gamma-Vorschau, 8↔16 Bit | 14 §1.5–1.9, `re\farbe\farbmodelle.py` |
//! | [`cmyk`] | CMYK → RGB bitgenau (9⁴-Gitter, Kaskade C→M→Y→K mit `>> 13`) | 14 §1.6 |
//! | [`install`] | Ressourcenordner des Originals zur Laufzeit (Voreinstellung, `FP_ORIGINAL_DIR`, Suche am Inhalt), PE-Ressourcen | 14 §1.6 |
//! | [`blend`] | 46 Mischmodi (`BLDi`), Original-„over“ skalar/MMX, Deckkraft, Überlappungstabelle | 07 §7, 05 §2.5, `re\vektor\blend.py` |
//! | [`swatches`] | Farbfelddateien ACT, ASE, ACO, GIF-Palette, `Color List.txt` | 14 §2.2, `re\farbe\farbfelddateien.py` |
//! | [`palette`] | [`Palette`] mit Transparenzindex, Farbabstand, nächste Farbe, Farbsortierung | 08 §2.4, 14 §2.4 |
//!
//! Es gibt **kein** Farbmanagement im Pixelpfad (14 §1.1): Dokumentfarben sind Geräte-RGB.

pub mod blend;
pub mod cmyk;
pub mod convert;
pub mod install;
pub mod models;
pub mod palette;
pub mod round;
pub mod swatches;

pub use palette::Palette;

/// Farbe wie im Dateiformat: 0xAARRGGBB, nicht vormultipliziert.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Argb(pub u32);

impl Argb {
    pub const fn a(self) -> u8 {
        (self.0 >> 24) as u8
    }
    pub const fn r(self) -> u8 {
        (self.0 >> 16) as u8
    }
    pub const fn g(self) -> u8 {
        (self.0 >> 8) as u8
    }
    pub const fn b(self) -> u8 {
        self.0 as u8
    }
    pub const fn to_rgba8(self) -> Rgba8 {
        Rgba8 {
            r: self.r(),
            g: self.g(),
            b: self.b(),
            a: self.a(),
        }
    }
}

/// Pixel RGBA mit 8 Bit je Kanal, nicht vormultipliziert (Byte-Reihenfolge R, G, B, A).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C)]
pub struct Rgba8 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba8 {
    pub const TRANSPARENT: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
    pub const fn to_argb(self) -> Argb {
        Argb(
            ((self.a as u32) << 24)
                | ((self.r as u32) << 16)
                | ((self.g as u32) << 8)
                | self.b as u32,
        )
    }
}

/// Deckkraft wie das Original sie intern hält: ein Byte 0..=255.
/// Datei: `OPAi` = trunc(byte · 1000 / 255) (Spezifikation 04).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Opacity(pub u8);

impl Default for Opacity {
    fn default() -> Self {
        Opacity(255)
    }
}

impl Opacity {
    /// Promille (`OPAi`) → Byte: `floor(OPAi/1000·255 + 0,5)` (07 §7.2, `@00937900`).
    /// Für alle 0..=1000 gleich der Python-Referenz `blend.opacity_byte` (Golden-Test).
    pub fn from_permille(p: u32) -> Opacity {
        Opacity(((p.min(1000) as f64) * 255.0 / 1000.0).round() as u8)
    }
    /// Byte → Promille: `⌊byte·1000/255⌋` (so schreibt das Original `OPAi`).
    pub fn to_permille(self) -> u32 {
        (self.0 as u32 * 1000) / 255
    }
    /// Prozent (0..100) → Byte wie im Original: `round(p·255/100)`, 30 % → 77 (= 30,196 %, re 29 §7).
    /// Deckkraft wird nur 8-bittig gespeichert; Anzeige und Skript runden auf ganze Prozent zurück.
    pub fn from_percent(p: f64) -> Opacity {
        let p = if p.is_finite() {
            p.clamp(0.0, 100.0)
        } else {
            100.0
        };
        Opacity((p * 255.0 / 100.0).round() as u8)
    }
    /// JS-API `opacity` (0..100).
    pub fn to_percent(self) -> f64 {
        self.0 as f64 * 100.0 / 255.0
    }
}

/// Mischmodus als Rohwert von `BLDi` (fehlt = 0 = Normal). Die Zuordnung Zahl → Formel steht in
/// Spezifikation 07 (§7.1) und wird in diesem Crate als Funktion umgesetzt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BlendMode(pub u8);
