//! Live-Effekte des Originals (Spezifikation `re\05-live-effekte.md` inkl. §11
//! Laufzeitergebnisse; Referenzen `re\effekte\fx.py`, `bevel.py`, `psfx.py`).
//!
//! Hängt nur von `fp-geom`, `fp-color` und `fp-raster` ab. Ein Effekt bekommt das **Objektbild**
//! (RGBA8, nicht vormultipliziert, mit Lage in Dokumentkoordinaten, [`fp_raster::RgbaImage`]) und
//! liefert das Ergebnisbild mit eigener Lage (meist größer). Objekt-Deckkraft und Mischmodus wendet
//! erst der Compositor beim Einsetzen an (05 §2.6).
//!
//! # Überblick
//!
//! | Modul | Effekt | Stand |
//! |---|---|---|
//! | [`shadow`] | Schlagschatten (Rand-Ring, Original-over MMX, Aussparen, letzte Zeile `screen(v,v)`), Innerer Schatten | B, Laufzeit T1/T1b exakt |
//! | [`bevel`] | Abschrägung innen/außen, Prägung erhaben/vertieft (Profile, Schaltflächenstatus), Glühen | B, Laufzeit T2 |
//! | [`inner_glow`] | Inneres Glühen (Morphologie mit `SelectionAdjustTolerance`) | B |
//! | [`solid`] | Massiver Schatten | B, Laufzeit T3 |
//! | [`color_fill`] | Farbfüllung | B (Normal) |
//! | [`screentone`] | Screentone / halftone screen (Firepixel extension, FB-38) | – |
//! | [`photoshop`] | Photoshop-Live-Effekte: Parser aller Datensätze, Schlagschatten, Farbüberlagerung | B/A |
//! | [`chain`] | [`LiveEffect`], Effektliste in Reihenfolge, Bitmap-Filter als Live-Filter | – |
//! | [`params`] | Lesen der Datei-Parameter | – |
//!
//! # Beispiel
//!
//! ```
//! use fp_effects::{apply_chain, EffectContext, EffectEntry, LiveEffect, params::params_from_pairs};
//! use fp_raster::{IRect, RgbaImage};
//! use fp_color::Rgba8;
//!
//! let obj = RgbaImage::filled(IRect::new(10, 10, 30, 30), Rgba8::rgb(255, 0, 0)).unwrap();
//! let p = params_from_pairs([("ShadowColor", "#000000ff"), ("ShadowBlur", "4"), ("ShadowDistance", "0")]);
//! let fx = LiveEffect::parse("{a7944db8-6ce2-11d1-8c76000502701850}", &p);
//! let out = apply_chain(&obj, &[EffectEntry { effect: fx, visible: true }], &EffectContext::default());
//! assert_eq!(out.image.alpha_bounds(), IRect::new(6, 6, 34, 34));
//! ```
//!
//! # Abweichungen von der Python-Referenz
//!
//! - Der Dreiecksfilter begrenzt seinen Radius wie das Original auf die Maskengröße (05 §2.2, B);
//!   die Referenz tut das nicht (nur bei sehr kleinen Objekten mit großem Weichzeichnen sichtbar).
//! - `EdgeThreshold` gilt nur beim Glühen (Leser `@00779ce0`, 05 offene Frage 5d); die Referenz
//!   liest den Wert für alle Typen (in Dateien bei Typ 0–3 immer 0).

// Index-Schleifen bilden die Rechenwege der Referenz 1:1 ab.
#![allow(clippy::needless_range_loop)]

pub mod bevel;
pub mod chain;
pub mod color_fill;
pub mod inner_glow;
pub mod params;
pub mod photoshop;
mod ps_relief;
pub mod screentone;
pub mod shadow;
pub mod solid;
mod util;
mod x87;

pub use bevel::{BevelParams, BevelType, ButtonState};
pub use chain::{ChainResult, EffectEntry, LiveEffect, apply_chain, chain_growth};
pub use color_fill::ColorFillParams;
pub use inner_glow::InnerGlowParams;
pub use photoshop::PsEffects;
pub use screentone::{ScreentoneKind, ScreentoneParams};
pub use shadow::ShadowParams;
pub use solid::SolidShadowParams;

/// Laufzeit-Einstellungen, die das Rechenergebnis beeinflussen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectContext {
    /// Voreinstellung `SelectionAdjustTolerance` (Schwelle der Maskenmorphologie, Standard 1).
    pub selection_adjust_tolerance: u8,
    /// Objekt auf Effektebene mit der MMX-Rundung mischen (das Original auf MMX-Prozessoren, Standard).
    pub mmx: bool,
    /// Darstellung wie die CS5-Bitmap-Paare (ohne Rand-Ring, exakt gerundet, ,5 ab); nur für Altdaten.
    pub legacy_cs5: bool,
    /// Eigenheit „letzte Zeile = screen(v,v)“ beim Schlagschatten (Laufzeit T1/T1b, Standard an).
    pub shadow_last_row_screen: bool,
}

impl Default for EffectContext {
    fn default() -> Self {
        EffectContext {
            selection_adjust_tolerance: 1,
            mmx: true,
            legacy_cs5: false,
            shadow_last_row_screen: true,
        }
    }
}
