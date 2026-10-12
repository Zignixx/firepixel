//! Pixelpuffer, Masken, Compositing, Resampling und Bitmap-Filter für Firepixel.
//!
//! Grundlage: Spezifikation 06 (Bitmap-Filter) mit den Referenzen `re\filter\fwfilters.py` und
//! `fweffects.py`, dazu 05 §2.2 (Dreiecksfilter, Morphologie), 13 §7.7 (Abtaster, 9-Slice),
//! 14 §3.4 (`*TransformMode`) und 02 §14 (Miniaturbild-Abtastung). Die Mischformeln kommen aus
//! `fp_color::blend`.
//!
//! # Überblick
//!
//! | Modul | Inhalt | stabil |
//! |---|---|---|
//! | [`image`] | [`RgbaImage`]: RGBA8, nicht vormultipliziert, mit Lage in Dokumentkoordinaten; Ausschnitt, Erweitern, Zuschneiden, Vormultiplizieren | ja (Grund-API) |
//! | [`tiles`] | [`TiledImage`]: 128×128-Kacheln mit Copy-on-Write (`Arc`) für Undo | – |
//! | [`mask`] | [`Mask`]: 8-Bit-Maske; Dreiecksfilter, weiche Kante, Erweitern/Verkleinern/Rand/Glätten, Umkehren | ja (Grund-API) |
//! | [`composite`] | [`composite()`]/[`composite_color`]: Bild bzw. Farbe mit Maske, Mischmodus, Deckkraft und Clip-Rechteck auf ein Ziel | ja |
//! | [`resample`] | nächster Nachbar, bilinear, bikubisch (a = −1), affine Transformation, 9-Slice-Skalierung | – |
//! | [`filters`] | alle eingebauten Filter, Rahmen mit Ausgabegrenzen und Auswahlmaske, Verteiler nach EffectMoaID | – |
//!
//! # Konventionen
//!
//! - Pixel `[r, g, b, a]` ([`Px`]), nicht vormultipliziert; die Farbe transparenter Pixel bleibt
//!   erhalten (mehrere Filter rechnen damit).
//! - Rechtecke ([`IRect`]) links/oben inklusiv, rechts/unten exklusiv, in Dokumentpixeln.
//! - Kein `unsafe`, keine Panik auf Eingaben: zu große Puffer ergeben [`RasterError::TooLarge`]
//!   bzw. bei Filtern die unveränderte Eingabe.
//! - Parallelisierung mit rayon nur über unabhängige Zeilen/Pixel: Ergebnisse sind bitgleich zur
//!   Einzel-Thread-Rechnung.

// Index-Schleifen bilden die Rechenwege der Spezifikation 1:1 ab; NaN-sichere negierte
// Vergleiche (wie im Original) sind Absicht.
#![allow(clippy::needless_range_loop, clippy::manual_checked_ops, clippy::neg_cmp_op_on_partial_ord)]

pub mod composite;
pub mod filters;
pub mod image;
pub mod mask;
mod par;
pub mod resample;
pub mod tiles;

pub use composite::{composite, composite_color, composite_extend, CompositeOptions};
pub use filters::{Filter, PixelFilter};
pub use fp_geom::IRect;
pub use image::{CompareResult, Px, RgbaImage, CLEAR};
pub use mask::{AlphaMask, Mask, MorphMode, SoftEdge};
pub use resample::{Edge, Sampler};
pub use tiles::{Tile, TiledImage, TILE_SIZE};

/// Größte Pixelzahl eines Puffers (2²⁸ ≈ 268 Mio. Pixel, 1 GiB RGBA).
pub const MAX_PIXELS: usize = 1 << 28;

/// Meldung aus fp-i18n (`meldungen.ftl`) mit zwei Werten.
fn raster_msg(key: &str, a: &str, va: &dyn std::fmt::Display, b: &str, vb: &dyn std::fmt::Display) -> String {
    fp_i18n::t_args(key, &[(a, fp_i18n::Value::from(va.to_string())), (b, fp_i18n::Value::from(vb.to_string()))])
}

/// Fehler von fp-raster.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RasterError {
    /// Puffer größer als [`MAX_PIXELS`].
    #[error("{}", raster_msg("err-raster-too-large", "w", .width, "h", .height))]
    TooLarge { width: u32, height: u32 },
    /// Datenlänge passt nicht zur Größe.
    #[error("{}", raster_msg("err-raster-size-mismatch", "got", .got, "expected", .expected))]
    SizeMismatch { expected: usize, got: usize },
}
