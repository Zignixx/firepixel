//! Operationen für [`Matrix3`] (Zeilenvektor-Konvention wie `MTXv`).
//!
//! Konvention: `p' = p · M`, also `x' = m00·x + m10·y + m20`, `y' = m01·x + m11·y + m21`,
//! `w = m02·x + m12·y + m22`. Das Produkt `a * b` heißt „erst `a`, dann `b`“. Die affine
//! Schreibweise `(a, b, c, d, tx, ty)` der Python-Referenzen entspricht
//! `x' = a·x + c·y + tx`, `y' = b·x + d·y + ty`, also `m00 = a`, `m01 = b`, `m10 = c`, `m11 = d`.
//!
//! Alle Rechnungen laufen intern in `f64` und werden erst beim Speichern auf `f32` gerundet.

#![allow(clippy::needless_range_loop)] // Matrix-/Kanalindizes sind hier lesbarer

use crate::{Matrix3, Point, Rect, Vec2};
use std::ops::Mul;

/// Interne `f64`-Matrix.
pub(crate) type M64 = [[f64; 3]; 3];

pub(crate) fn to64(m: &Matrix3) -> M64 {
    let mut r = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            r[i][j] = m.m[i][j] as f64;
        }
    }
    r
}

pub(crate) fn from64(m: &M64) -> Matrix3 {
    let mut r = [[0.0f32; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            r[i][j] = m[i][j] as f32;
        }
    }
    Matrix3 { m: r }
}

pub(crate) fn mul64(a: &M64, b: &M64) -> M64 {
    let mut r = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            r[i][j] = a[i][0] * b[0][j] + a[i][1] * b[1][j] + a[i][2] * b[2][j];
        }
    }
    r
}

fn det64(m: &M64) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]) - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

fn inv64(m: &M64) -> Option<M64> {
    let det = det64(m);
    if det == 0.0 || !det.is_finite() {
        return None;
    }
    let inv = 1.0 / det;
    let r = [
        [
            (m[1][1] * m[2][2] - m[1][2] * m[2][1]) * inv,
            (m[0][2] * m[2][1] - m[0][1] * m[2][2]) * inv,
            (m[0][1] * m[1][2] - m[0][2] * m[1][1]) * inv,
        ],
        [
            (m[1][2] * m[2][0] - m[1][0] * m[2][2]) * inv,
            (m[0][0] * m[2][2] - m[0][2] * m[2][0]) * inv,
            (m[0][2] * m[1][0] - m[0][0] * m[1][2]) * inv,
        ],
        [
            (m[1][0] * m[2][1] - m[1][1] * m[2][0]) * inv,
            (m[0][1] * m[2][0] - m[0][0] * m[2][1]) * inv,
            (m[0][0] * m[1][1] - m[0][1] * m[1][0]) * inv,
        ],
    ];
    if r.iter().flatten().all(|v| v.is_finite()) { Some(r) } else { None }
}

impl Matrix3 {
    /// Matrix aus den neun Einträgen (Zeilen `m[0]`, `m[1]`, `m[2]`).
    pub const fn from_rows(m: [[f32; 3]; 3]) -> Self {
        Self { m }
    }

    /// Affine Matrix aus `(a, b, c, d, tx, ty)`: `x' = a·x + c·y + tx`, `y' = b·x + d·y + ty`.
    pub const fn from_affine(a: f32, b: f32, c: f32, d: f32, tx: f32, ty: f32) -> Self {
        Self { m: [[a, b, 0.0], [c, d, 0.0], [tx, ty, 1.0]] }
    }

    /// Affiner Anteil als `(a, b, c, d, tx, ty)` (perspektivische Spalte wird ignoriert).
    pub fn to_affine(&self) -> [f32; 6] {
        let m = &self.m;
        [m[0][0], m[0][1], m[1][0], m[1][1], m[2][0], m[2][1]]
    }

    /// Verschiebung.
    pub const fn translation(tx: f32, ty: f32) -> Self {
        Self::from_affine(1.0, 0.0, 0.0, 1.0, tx, ty)
    }

    /// Skalierung um den Ursprung.
    pub const fn scaling(sx: f32, sy: f32) -> Self {
        Self::from_affine(sx, 0.0, 0.0, sy, 0.0, 0.0)
    }

    /// Drehung um den Ursprung um `radians`. Bei y nach unten dreht ein positiver Winkel im
    /// Uhrzeigersinn auf dem Bildschirm (+x Richtung +y): `x' = x·cos − y·sin`, `y' = x·sin + y·cos`.
    pub fn rotation(radians: f64) -> Self {
        let (s, c) = radians.sin_cos();
        Self::from_affine(c as f32, s as f32, -s as f32, c as f32, 0.0, 0.0)
    }

    /// Drehung in Grad (siehe [`Matrix3::rotation`]).
    pub fn rotation_degrees(degrees: f64) -> Self {
        Self::rotation(degrees.to_radians())
    }

    /// Scherung: `x' = x + shx·y`, `y' = shy·x + y`.
    pub const fn shear(shx: f32, shy: f32) -> Self {
        Self::from_affine(1.0, shy, shx, 1.0, 0.0, 0.0)
    }

    /// Scherung über Winkel (Grad): `shx = tan(ax)`, `shy = tan(ay)`.
    pub fn skew_degrees(ax: f64, ay: f64) -> Self {
        Self::shear(ax.to_radians().tan() as f32, ay.to_radians().tan() as f32)
    }

    /// Produkt `self · o` („erst `self`, dann `o`“), in `f64` gerechnet.
    pub fn then(&self, o: &Matrix3) -> Matrix3 {
        from64(&mul64(&to64(self), &to64(o)))
    }

    /// Determinante der vollen 3×3-Matrix.
    pub fn determinant(&self) -> f64 {
        det64(&to64(self))
    }

    /// Inverse (allgemein, auch perspektivisch); `None`, wenn singulär oder nicht endlich.
    pub fn inverse(&self) -> Option<Matrix3> {
        inv64(&to64(self)).map(|m| from64(&m))
    }

    /// Ist die perspektivische Spalte `(0, 0, 1)`?
    pub fn is_affine(&self) -> bool {
        self.m[0][2] == 0.0 && self.m[1][2] == 0.0 && self.m[2][2] == 1.0
    }

    /// Exakt die Einheitsmatrix?
    pub fn is_identity(&self) -> bool {
        *self == Matrix3::IDENTITY
    }

    /// Ist die Matrix bis auf `eps` eine reine Verschiebung (affin, linearer Anteil ≈ Einheit)?
    /// Das Original verwendet beim Tweening `eps = 0,003` (13 §7.2).
    pub fn is_translation(&self, eps: f32) -> bool {
        let m = &self.m;
        (m[0][0] - 1.0).abs() <= eps
            && m[0][1].abs() <= eps
            && m[1][0].abs() <= eps
            && (m[1][1] - 1.0).abs() <= eps
            && m[0][2].abs() <= eps
            && m[1][2].abs() <= eps
            && (m[2][2] - 1.0).abs() <= eps
    }

    /// Punkt in `f64` transformieren (mit perspektivischer Division, wenn `w ∉ {0, 1}`).
    pub fn apply_vec2(&self, p: Vec2) -> Vec2 {
        let m = to64(self);
        let x = m[0][0] * p.x + m[1][0] * p.y + m[2][0];
        let y = m[0][1] * p.x + m[1][1] * p.y + m[2][1];
        let w = m[0][2] * p.x + m[1][2] * p.y + m[2][2];
        if w != 0.0 && w != 1.0 { Vec2::new(x / w, y / w) } else { Vec2::new(x, y) }
    }

    /// Richtungsvektor transformieren (nur linearer Anteil, ohne Verschiebung).
    pub fn apply_vector(&self, v: Point) -> Point {
        let m = &self.m;
        Point::new(m[0][0] * v.x + m[1][0] * v.y, m[0][1] * v.x + m[1][1] * v.y)
    }

    /// Die vier transformierten Ecken eines Rechtecks (Reihenfolge wie [`Rect::corners`]).
    pub fn transform_quad(&self, r: &Rect) -> [Point; 4] {
        r.corners().map(|p| self.apply_vec2(Vec2::from(p)).to_point())
    }

    /// Achsenparallele Hülle eines transformierten Rechtecks (Bounding-Box der vier Ecken).
    pub fn transform_rect_bounds(&self, r: &Rect) -> Rect {
        let q = r.corners().map(|p| self.apply_vec2(Vec2::from(p)));
        let (mut l, mut t, mut rr, mut b) = (q[0].x, q[0].y, q[0].x, q[0].y);
        for p in &q[1..] {
            l = l.min(p.x);
            t = t.min(p.y);
            rr = rr.max(p.x);
            b = b.max(p.y);
        }
        Rect::new(l as f32, t as f32, rr as f32, b as f32)
    }

    /// Projektive Abbildung des Rechtecks `r` auf das Viereck `quad`
    /// (Ecken in der Reihenfolge von [`Rect::corners`]: links oben, rechts oben, rechts unten,
    /// links unten). Ist das Viereck ein Parallelogramm, ist das Ergebnis affin.
    /// `None` bei leerem Rechteck oder entartetem Viereck.
    ///
    /// Das Original nutzt eine solche Abbildung u. a. beim Tweening (13 §7.2, Rechteck → Viereck).
    pub fn rect_to_quad(r: &Rect, quad: &[Point; 4]) -> Option<Matrix3> {
        let w = (r.right - r.left) as f64;
        let h = (r.bottom - r.top) as f64;
        if w == 0.0 || h == 0.0 || !w.is_finite() || !h.is_finite() {
            return None;
        }
        let q: [Vec2; 4] = quad.map(Vec2::from);
        let s = square_to_quad(&q)?;
        let to_unit: M64 = [[1.0 / w, 0.0, 0.0], [0.0, 1.0 / h, 0.0], [-(r.left as f64) / w, -(r.top as f64) / h, 1.0]];
        Some(from64(&mul64(&to_unit, &s)))
    }
}

/// Einheitsquadrat (0,0),(1,0),(1,1),(0,1) → Viereck q0..q3 (Heckbert), Zeilenvektor-Form.
fn square_to_quad(q: &[Vec2; 4]) -> Option<M64> {
    let (x0, y0, x1, y1, x2, y2, x3, y3) = (q[0].x, q[0].y, q[1].x, q[1].y, q[2].x, q[2].y, q[3].x, q[3].y);
    let sx = x0 - x1 + x2 - x3;
    let sy = y0 - y1 + y2 - y3;
    let (a, b, c, d, e, f, g, hh);
    if sx == 0.0 && sy == 0.0 {
        a = x1 - x0;
        b = x3 - x0;
        c = x0;
        d = y1 - y0;
        e = y3 - y0;
        f = y0;
        g = 0.0;
        hh = 0.0;
    } else {
        let dx1 = x1 - x2;
        let dx2 = x3 - x2;
        let dy1 = y1 - y2;
        let dy2 = y3 - y2;
        let den = dx1 * dy2 - dx2 * dy1;
        if den == 0.0 {
            return None;
        }
        g = (sx * dy2 - dx2 * sy) / den;
        hh = (dx1 * sy - sx * dy1) / den;
        a = x1 - x0 + g * x1;
        b = x3 - x0 + hh * x3;
        c = x0;
        d = y1 - y0 + g * y1;
        e = y3 - y0 + hh * y3;
        f = y0;
    }
    let m = [[a, d, g], [b, e, hh], [c, f, 1.0]];
    if det64(&m) == 0.0 || !m.iter().flatten().all(|v| v.is_finite()) {
        return None;
    }
    Some(m)
}

impl Mul for Matrix3 {
    type Output = Matrix3;
    /// `a * b` = „erst `a`, dann `b`“ (siehe [`Matrix3::then`]).
    fn mul(self, o: Matrix3) -> Matrix3 {
        self.then(&o)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Point, b: Point) -> bool {
        (a.x - b.x).abs() < 1e-4 && (a.y - b.y).abs() < 1e-4
    }

    #[test]
    fn compose_order() {
        let t = Matrix3::translation(10.0, 0.0);
        let s = Matrix3::scaling(2.0, 2.0);
        let p = Point::new(1.0, 1.0);
        assert_eq!((t * s).apply(p), Point::new(22.0, 2.0)); // erst verschieben, dann skalieren
        assert_eq!((s * t).apply(p), Point::new(12.0, 2.0));
        let r = Matrix3::rotation_degrees(90.0);
        assert!(close(r.apply(Point::new(1.0, 0.0)), Point::new(0.0, 1.0)));
    }

    #[test]
    fn inverse_roundtrip() {
        let m = Matrix3::from_affine(1.5, 0.3, -0.7, 2.0, 12.0, -4.0) * Matrix3::rotation(0.4);
        let inv = m.inverse().unwrap();
        let p = Point::new(3.25, -7.5);
        assert!(close(inv.apply(m.apply(p)), p));
        assert!(Matrix3::scaling(0.0, 1.0).inverse().is_none());
        let id = m * inv;
        assert!(id.is_translation(1e-5));
    }

    #[test]
    fn rect_bounds_and_quad() {
        let r = Rect::new(0.0, 0.0, 10.0, 20.0);
        let m = Matrix3::rotation_degrees(90.0);
        let b = m.transform_rect_bounds(&r);
        assert!((b.left + 20.0).abs() < 1e-4 && b.top.abs() < 1e-4 && b.right.abs() < 1e-4 && (b.bottom - 10.0).abs() < 1e-4);
        // affines Viereck
        let quad = m.transform_quad(&r);
        let back = Matrix3::rect_to_quad(&r, &quad).unwrap();
        for p in [Point::new(3.0, 4.0), Point::new(10.0, 0.0)] {
            assert!(close(back.apply(p), m.apply(p)));
        }
        // perspektivisch: Ecken müssen exakt getroffen werden
        let quad = [Point::new(0.0, 0.0), Point::new(100.0, 10.0), Point::new(90.0, 80.0), Point::new(5.0, 60.0)];
        let pm = Matrix3::rect_to_quad(&r, &quad).unwrap();
        assert!(!pm.is_affine());
        for (c, q) in r.corners().iter().zip(quad.iter()) {
            assert!(close(pm.apply(*c), *q), "{:?} -> {:?} statt {:?}", c, pm.apply(*c), q);
        }
    }
}
