//! [`Vec2`]: zweidimensionaler `f64`-Vektor für genaue Zwischenrechnungen.
//!
//! Dokumentkoordinaten sind `f32` ([`Point`]); Bézier-Auswertung, Längen, Offsets und Löser
//! rechnen in `f64` (wie die Python-Referenzen und wie x87-Zwischenwerte des Originals).

use crate::Point;
use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

/// Punkt bzw. Vektor mit `f64`-Koordinaten.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Vec2 {
    /// Nullvektor.
    pub const ZERO: Vec2 = Vec2 { x: 0.0, y: 0.0 };

    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Skalarprodukt.
    #[inline]
    pub fn dot(self, o: Vec2) -> f64 {
        self.x * o.x + self.y * o.y
    }

    /// Kreuzprodukt (z-Komponente): `self.x·o.y − self.y·o.x`.
    #[inline]
    pub fn cross(self, o: Vec2) -> f64 {
        self.x * o.y - self.y * o.x
    }

    /// Euklidische Länge (`hypot`).
    #[inline]
    pub fn length(self) -> f64 {
        self.x.hypot(self.y)
    }

    /// Quadrat der Länge.
    #[inline]
    pub fn length_sq(self) -> f64 {
        self.x * self.x + self.y * self.y
    }

    /// Abstand zu `o`.
    #[inline]
    pub fn distance(self, o: Vec2) -> f64 {
        (self.x - o.x).hypot(self.y - o.y)
    }

    /// Lineare Interpolation `self + (o − self)·t` (Formel wie in den Python-Referenzen).
    #[inline]
    pub fn lerp(self, o: Vec2, t: f64) -> Vec2 {
        Vec2::new(self.x + (o.x - self.x) * t, self.y + (o.y - self.y) * t)
    }

    /// Einheitsvektor; Länge ≤ 1e-12 ergibt den Nullvektor.
    pub fn normalize(self) -> Vec2 {
        let l = self.length();
        if l > 1e-12 {
            Vec2::new(self.x / l, self.y / l)
        } else {
            Vec2::ZERO
        }
    }

    /// Senkrechte „links“ im mathematischen Sinn: `(−y, x)`.
    #[inline]
    pub fn perp(self) -> Vec2 {
        Vec2::new(-self.y, self.x)
    }

    /// Umwandlung in einen `f32`-Punkt.
    #[inline]
    pub fn to_point(self) -> Point {
        Point::new(self.x as f32, self.y as f32)
    }

    /// Beide Koordinaten endlich?
    #[inline]
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

impl From<Point> for Vec2 {
    #[inline]
    fn from(p: Point) -> Self {
        Vec2::new(p.x as f64, p.y as f64)
    }
}

impl From<Vec2> for Point {
    #[inline]
    fn from(v: Vec2) -> Self {
        v.to_point()
    }
}

impl From<(f64, f64)> for Vec2 {
    #[inline]
    fn from((x, y): (f64, f64)) -> Self {
        Vec2::new(x, y)
    }
}

impl Add for Vec2 {
    type Output = Vec2;
    #[inline]
    fn add(self, o: Vec2) -> Vec2 {
        Vec2::new(self.x + o.x, self.y + o.y)
    }
}

impl AddAssign for Vec2 {
    #[inline]
    fn add_assign(&mut self, o: Vec2) {
        self.x += o.x;
        self.y += o.y;
    }
}

impl Sub for Vec2 {
    type Output = Vec2;
    #[inline]
    fn sub(self, o: Vec2) -> Vec2 {
        Vec2::new(self.x - o.x, self.y - o.y)
    }
}

impl SubAssign for Vec2 {
    #[inline]
    fn sub_assign(&mut self, o: Vec2) {
        self.x -= o.x;
        self.y -= o.y;
    }
}

impl Mul<f64> for Vec2 {
    type Output = Vec2;
    #[inline]
    fn mul(self, s: f64) -> Vec2 {
        Vec2::new(self.x * s, self.y * s)
    }
}

impl Div<f64> for Vec2 {
    type Output = Vec2;
    #[inline]
    fn div(self, s: f64) -> Vec2 {
        Vec2::new(self.x / s, self.y / s)
    }
}

impl Neg for Vec2 {
    type Output = Vec2;
    #[inline]
    fn neg(self) -> Vec2 {
        Vec2::new(-self.x, -self.y)
    }
}
