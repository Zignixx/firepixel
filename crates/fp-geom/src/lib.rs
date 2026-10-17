//! Geometrie-Grundtypen für Firepixel.
//!
//! Verbindliche Grundtypen (von allen Crates genutzt; Signaturen nicht ändern, nur erweitern):
//! [`Point`], [`Rect`], [`Matrix3`]. Koordinaten in Dokument-Pixeln, y nach unten, `f32` wie im
//! Dateiformat. Für Zwischenrechnungen darf intern `f64` verwendet werden, wo die Spezifikation es verlangt.
//!
//! # Überblick
//!
//! | Modul | Inhalt | Spezifikation |
//! |---|---|---|
//! | [`round`] | Original-Rundungsarten (`round_away`, `round_even`, `trunc`, C-Division) | 14 §1.7 |
//! | [`vec2`] | [`Vec2`]: `f64`-Vektor für Bézier- und Offset-Rechnung | – |
//! | [`ops`] | Rechenoperationen für [`Point`] und [`Rect`], ganzzahliges [`IRect`] | 05 §2.5 (floor/ceil) |
//! | [`matrix`] | Matrix-Produkt, Inverse, Konstruktoren, Rechteck→Viereck | 02 `MTXv`, 13 §7.2 |
//! | [`decompose`] | Shoemake-Polarzerlegung, Quaternion-Slerp, Tweening-Interpolation | 13 §7.2 |
//! | [`bezier`] | [`CubicBez`], [`QuadBez`]: Auswertung, Teilung, Bounds, Länge, nächster Punkt | 16 §1.3–1.5 |
//! | [`contour`] | [`BezierNode`], [`BezierContour`], [`Polyline`], Fläche, Windungszahl | 16 §1.1 |
//! | [`flatten`] | Original-Abflachung (= `PCLv`) und die ältere Sehnen-Abflachung des Rasterizers | 16 §1.4, 07 §3 |
//! | [`offset`] | Segment-Offset, Gehrungsregel, Ecken und Kappen | 16 §3.3/3.4 |
//!
//! Matrizen folgen der Zeilenvektor-Konvention von `MTXv`: `p' = p · M`. Das Produkt `a * b`
//! bedeutet „erst `a`, dann `b`“.

pub mod bezier;
pub mod contour;
pub mod decompose;
pub mod flatten;
pub mod matrix;
pub mod offset;
pub mod ops;
pub mod round;
pub mod vec2;

pub use bezier::{Bounds, CubicBez, Nearest, QuadBez};
pub use contour::{BezierContour, BezierNode, Polyline};
pub use decompose::{AffineParts, Quat};
pub use flatten::FlattenOptions;
pub use ops::IRect;
pub use vec2::Vec2;

/// Ellipsen-/Rundungskonstante κ des Originals (`fw.ellipseBCPConst`, Spezifikation 16 §3.11, B).
///
/// Bewusst **nicht** der Lehrbuchwert 0,5522847: Ellipsen und Rundrechtecke weichen sonst um bis zu
/// ≈ 0,0007·r ab.
pub const ELLIPSE_KAPPA: f32 = 0.552_291_87;

/// Punkt bzw. Vektor in Dokumentkoordinaten.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// Achsenparalleles Rechteck (links/oben inklusiv, rechts/unten exklusiv bei Pixelrastern).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Rect {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl Rect {
    pub const fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }
    pub fn width(&self) -> f32 {
        self.right - self.left
    }
    pub fn height(&self) -> f32 {
        self.bottom - self.top
    }
    pub fn is_empty(&self) -> bool {
        self.right <= self.left || self.bottom <= self.top
    }
}

/// 3×3-Matrix in Zeilenvektor-Konvention wie `MTXv`: x' = m00·x + m10·y + m20, y' = m01·x + m11·y + m21.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Matrix3 {
    pub m: [[f32; 3]; 3],
}

impl Default for Matrix3 {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Matrix3 {
    pub const IDENTITY: Self = Self {
        m: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
    };

    /// Wendet die Matrix auf einen Punkt an (affiner Anteil; perspektivische Zeile wird berücksichtigt).
    pub fn apply(&self, p: Point) -> Point {
        let m = &self.m;
        let x = m[0][0] * p.x + m[1][0] * p.y + m[2][0];
        let y = m[0][1] * p.x + m[1][1] * p.y + m[2][1];
        let w = m[0][2] * p.x + m[1][2] * p.y + m[2][2];
        if w != 0.0 && w != 1.0 {
            Point::new(x / w, y / w)
        } else {
            Point::new(x, y)
        }
    }
}
