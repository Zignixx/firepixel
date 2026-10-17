//! Rechenoperationen für [`Point`] und [`Rect`] sowie das ganzzahlige Pixelrechteck [`IRect`].
//!
//! Rundung von Gleitkomma-Rechtecken auf Pixel: Das Original rundet Effekt- und Puffergrenzen nach
//! außen (`floor` links/oben, `ceil` rechts/unten; `@00442ea0`/`@00442ff0`, Spezifikation 05 §2.5,
//! 06 §2.4) → [`Rect::round_out`]. Die Regel von „An Pixeln ausrichten“ ist laut 14 offene Frage 7
//! noch nicht geklärt und deshalb hier nicht nachgebaut.

use crate::round::round_away;
use crate::{Point, Rect, Vec2};
use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

// ------------------------------------------------------------------ Point

impl Point {
    /// Ursprung (0, 0).
    pub const ZERO: Point = Point { x: 0.0, y: 0.0 };

    /// Skalarprodukt.
    #[inline]
    pub fn dot(self, o: Point) -> f32 {
        self.x * o.x + self.y * o.y
    }

    /// Kreuzprodukt (z-Komponente).
    #[inline]
    pub fn cross(self, o: Point) -> f32 {
        self.x * o.y - self.y * o.x
    }

    /// Länge (in `f64` gerechnet).
    #[inline]
    pub fn length(self) -> f32 {
        (self.x as f64).hypot(self.y as f64) as f32
    }

    /// Abstand zu `o` (in `f64` gerechnet).
    #[inline]
    pub fn distance(self, o: Point) -> f32 {
        Vec2::from(self).distance(Vec2::from(o)) as f32
    }

    /// Lineare Interpolation.
    #[inline]
    pub fn lerp(self, o: Point, t: f32) -> Point {
        Point::new(self.x + (o.x - self.x) * t, self.y + (o.y - self.y) * t)
    }

    /// Umwandlung in [`Vec2`] (`f64`).
    #[inline]
    pub fn to_vec2(self) -> Vec2 {
        Vec2::from(self)
    }

    /// Beide Koordinaten endlich?
    #[inline]
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

impl Add for Point {
    type Output = Point;
    #[inline]
    fn add(self, o: Point) -> Point {
        Point::new(self.x + o.x, self.y + o.y)
    }
}

impl AddAssign for Point {
    #[inline]
    fn add_assign(&mut self, o: Point) {
        self.x += o.x;
        self.y += o.y;
    }
}

impl Sub for Point {
    type Output = Point;
    #[inline]
    fn sub(self, o: Point) -> Point {
        Point::new(self.x - o.x, self.y - o.y)
    }
}

impl SubAssign for Point {
    #[inline]
    fn sub_assign(&mut self, o: Point) {
        self.x -= o.x;
        self.y -= o.y;
    }
}

impl Mul<f32> for Point {
    type Output = Point;
    #[inline]
    fn mul(self, s: f32) -> Point {
        Point::new(self.x * s, self.y * s)
    }
}

impl Div<f32> for Point {
    type Output = Point;
    #[inline]
    fn div(self, s: f32) -> Point {
        Point::new(self.x / s, self.y / s)
    }
}

impl Neg for Point {
    type Output = Point;
    #[inline]
    fn neg(self) -> Point {
        Point::new(-self.x, -self.y)
    }
}

// ------------------------------------------------------------------ Rect

impl Rect {
    /// Rechteck aus Ursprung und Größe.
    pub fn from_xywh(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::new(x, y, x + w, y + h)
    }

    /// Kleinstes Rechteck um alle Punkte; `None` bei leerer Eingabe.
    ///
    /// Ein einzelner Punkt oder eine waagerechte Linie ergibt ein entartetes Rechteck
    /// (Breite bzw. Höhe 0), das [`Rect::is_empty`] meldet, aber gültige Koordinaten hat.
    pub fn from_points<I: IntoIterator<Item = Point>>(pts: I) -> Option<Rect> {
        let mut it = pts.into_iter();
        let first = it.next()?;
        let mut r = Rect::new(first.x, first.y, first.x, first.y);
        for p in it {
            r.include_point(p);
        }
        Some(r)
    }

    /// Erweitert das Rechteck so, dass `p` enthalten ist (Grenzen inklusiv).
    pub fn include_point(&mut self, p: Point) {
        self.left = self.left.min(p.x);
        self.top = self.top.min(p.y);
        self.right = self.right.max(p.x);
        self.bottom = self.bottom.max(p.y);
    }

    /// Vereinigung: min/max der Koordinaten, **ohne** Sonderbehandlung leerer Rechtecke
    /// (Hüllen-Semantik, damit entartete Grenzen von Punkten/Linien erhalten bleiben).
    /// Für „leer ist neutral“ siehe [`Rect::union_non_empty`].
    pub fn union(&self, o: &Rect) -> Rect {
        Rect::new(
            self.left.min(o.left),
            self.top.min(o.top),
            self.right.max(o.right),
            self.bottom.max(o.bottom),
        )
    }

    /// Vereinigung, bei der leere Rechtecke ([`Rect::is_empty`]) ignoriert werden.
    pub fn union_non_empty(&self, o: &Rect) -> Rect {
        match (self.is_empty(), o.is_empty()) {
            (true, true) => Rect::default(),
            (true, false) => *o,
            (false, true) => *self,
            (false, false) => self.union(o),
        }
    }

    /// Schnittmenge; kann leer sein (dann [`Rect::is_empty`] wahr, Koordinaten nicht normiert).
    pub fn intersect(&self, o: &Rect) -> Rect {
        Rect::new(
            self.left.max(o.left),
            self.top.max(o.top),
            self.right.min(o.right),
            self.bottom.min(o.bottom),
        )
    }

    /// Haben beide Rechtecke eine Fläche gemeinsam?
    pub fn intersects(&self, o: &Rect) -> bool {
        !self.intersect(o).is_empty()
    }

    /// Vergrößert um `dx` links und rechts sowie `dy` oben und unten (negativ = verkleinern).
    pub fn inflate(&self, dx: f32, dy: f32) -> Rect {
        Rect::new(
            self.left - dx,
            self.top - dy,
            self.right + dx,
            self.bottom + dy,
        )
    }

    /// Verschiebt um (`dx`, `dy`).
    pub fn translate(&self, dx: f32, dy: f32) -> Rect {
        Rect::new(
            self.left + dx,
            self.top + dy,
            self.right + dx,
            self.bottom + dy,
        )
    }

    /// Punkt im halboffenen Rechteck `[left, right) × [top, bottom)`?
    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.left && p.x < self.right && p.y >= self.top && p.y < self.bottom
    }

    /// Liegt `o` vollständig in diesem Rechteck?
    pub fn contains_rect(&self, o: &Rect) -> bool {
        o.left >= self.left && o.top >= self.top && o.right <= self.right && o.bottom <= self.bottom
    }

    /// Mittelpunkt.
    pub fn center(&self) -> Point {
        Point::new(
            (self.left + self.right) * 0.5,
            (self.top + self.bottom) * 0.5,
        )
    }

    /// Die vier Ecken im Uhrzeigersinn auf dem Bildschirm: links oben, rechts oben, rechts unten, links unten.
    pub fn corners(&self) -> [Point; 4] {
        [
            Point::new(self.left, self.top),
            Point::new(self.right, self.top),
            Point::new(self.right, self.bottom),
            Point::new(self.left, self.bottom),
        ]
    }

    /// Vertauscht verkehrt herum liegende Kanten, sodass `left ≤ right` und `top ≤ bottom`.
    pub fn normalized(&self) -> Rect {
        Rect::new(
            self.left.min(self.right),
            self.top.min(self.bottom),
            self.left.max(self.right),
            self.top.max(self.bottom),
        )
    }

    /// Nach außen auf ganze Pixel runden: `floor` links/oben, `ceil` rechts/unten
    /// (im Original `@00442ea0`/`@00442ff0`, Spezifikation 05 §2.5).
    pub fn round_out(&self) -> IRect {
        IRect::new(
            sat_i32(self.left.floor() as f64),
            sat_i32(self.top.floor() as f64),
            sat_i32(self.right.ceil() as f64),
            sat_i32(self.bottom.ceil() as f64),
        )
    }

    /// Alle vier Kanten mit `floor` runden (Ausgabegrenzen der Weichzeichner, Spezifikation 06 §2.4:
    /// „Gleitkomma-Grenzen ± ceil(2,5·r), dann auf allen vier Seiten abgerundet“).
    pub fn round_floor(&self) -> IRect {
        IRect::new(
            sat_i32(self.left.floor() as f64),
            sat_i32(self.top.floor() as f64),
            sat_i32(self.right.floor() as f64),
            sat_i32(self.bottom.floor() as f64),
        )
    }

    /// Nach innen auf ganze Pixel runden: `ceil` links/oben, `floor` rechts/unten.
    pub fn round_in(&self) -> IRect {
        IRect::new(
            sat_i32(self.left.ceil() as f64),
            sat_i32(self.top.ceil() as f64),
            sat_i32(self.right.floor() as f64),
            sat_i32(self.bottom.floor() as f64),
        )
    }

    /// Jede Kante einzeln mit [`round_away`] runden (halbe Werte weg von null).
    pub fn round_nearest(&self) -> IRect {
        IRect::new(
            sat_i32(round_away(self.left as f64)),
            sat_i32(round_away(self.top as f64)),
            sat_i32(round_away(self.right as f64)),
            sat_i32(round_away(self.bottom as f64)),
        )
    }

    /// Alle Koordinaten endlich?
    pub fn is_finite(&self) -> bool {
        self.left.is_finite()
            && self.top.is_finite()
            && self.right.is_finite()
            && self.bottom.is_finite()
    }
}

#[inline]
fn sat_i32(v: f64) -> i32 {
    if v.is_nan() {
        0
    } else {
        v.clamp(i32::MIN as f64, i32::MAX as f64) as i32
    }
}

// ------------------------------------------------------------------ IRect

/// Ganzzahliges Pixelrechteck, links/oben inklusiv, rechts/unten exklusiv.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl IRect {
    pub const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    /// Rechteck aus Ursprung und Größe.
    pub const fn from_xywh(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self::new(x, y, x.saturating_add(w), y.saturating_add(h))
    }

    /// Breite (0, wenn leer).
    pub fn width(&self) -> u32 {
        (self.right as i64 - self.left as i64).max(0) as u32
    }

    /// Höhe (0, wenn leer).
    pub fn height(&self) -> u32 {
        (self.bottom as i64 - self.top as i64).max(0) as u32
    }

    /// Fläche in Pixeln.
    pub fn area(&self) -> u64 {
        self.width() as u64 * self.height() as u64
    }

    pub fn is_empty(&self) -> bool {
        self.right <= self.left || self.bottom <= self.top
    }

    /// Vereinigung; leere Rechtecke sind neutral.
    pub fn union(&self, o: &IRect) -> IRect {
        if self.is_empty() {
            return *o;
        }
        if o.is_empty() {
            return *self;
        }
        IRect::new(
            self.left.min(o.left),
            self.top.min(o.top),
            self.right.max(o.right),
            self.bottom.max(o.bottom),
        )
    }

    /// Schnittmenge; leer ergibt `IRect::default()`.
    pub fn intersect(&self, o: &IRect) -> IRect {
        let r = IRect::new(
            self.left.max(o.left),
            self.top.max(o.top),
            self.right.min(o.right),
            self.bottom.min(o.bottom),
        );
        if r.is_empty() { IRect::default() } else { r }
    }

    /// Vergrößert um `d` Pixel auf jeder Seite (negativ = verkleinern).
    pub fn inflate(&self, dx: i32, dy: i32) -> IRect {
        IRect::new(
            self.left.saturating_sub(dx),
            self.top.saturating_sub(dy),
            self.right.saturating_add(dx),
            self.bottom.saturating_add(dy),
        )
    }

    /// Verschiebt um (`dx`, `dy`).
    pub fn translate(&self, dx: i32, dy: i32) -> IRect {
        IRect::new(
            self.left.saturating_add(dx),
            self.top.saturating_add(dy),
            self.right.saturating_add(dx),
            self.bottom.saturating_add(dy),
        )
    }

    /// Pixel (x, y) enthalten?
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.left && x < self.right && y >= self.top && y < self.bottom
    }

    /// Umwandlung in ein Gleitkomma-Rechteck.
    pub fn to_rect(&self) -> Rect {
        Rect::new(
            self.left as f32,
            self.top as f32,
            self.right as f32,
            self.bottom as f32,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_ops() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        let b = Rect::new(5.0, -5.0, 20.0, 5.0);
        assert_eq!(a.union(&b), Rect::new(0.0, -5.0, 20.0, 10.0));
        assert_eq!(a.intersect(&b), Rect::new(5.0, 0.0, 10.0, 5.0));
        assert!(a.intersects(&b));
        assert!(!a.intersects(&Rect::new(10.0, 0.0, 12.0, 1.0)));
        assert_eq!(a.inflate(1.0, 2.0), Rect::new(-1.0, -2.0, 11.0, 12.0));
        assert!(a.contains(Point::new(0.0, 9.9)));
        assert!(!a.contains(Point::new(10.0, 5.0)));
        let line = Rect::from_points([Point::new(1.0, 2.0), Point::new(5.0, 2.0)]).unwrap();
        assert!(line.is_empty());
        assert_eq!(line.union(&a), Rect::new(0.0, 0.0, 10.0, 10.0));
        assert_eq!(Rect::new(1.0, 1.0, 1.0, 1.0).union_non_empty(&b), b);
    }

    #[test]
    fn rect_rounding() {
        let r = Rect::new(-16.097, 1.0, 28.653, 35.0).inflate(5.0, 5.0);
        // Beispiel aus 06 §2.4: Location Pin, ± ceil(2,5·2) = 5, alle Seiten floor → x −22…33, y −4…40
        assert_eq!(r.round_floor(), IRect::new(-22, -4, 33, 40));
        assert_eq!(r.round_out(), IRect::new(-22, -4, 34, 40));
        assert_eq!(
            Rect::new(0.5, 0.5, 2.5, 2.5).round_in(),
            IRect::new(1, 1, 2, 2)
        );
        assert_eq!(
            Rect::new(-0.5, 0.5, 2.5, 2.4).round_nearest(),
            IRect::new(-1, 1, 3, 2)
        );
        assert_eq!(Rect::new(f32::NAN, 0.0, 1.0, 1.0).round_out().left, 0);
    }

    #[test]
    fn irect_ops() {
        let a = IRect::new(0, 0, 10, 10);
        assert_eq!(
            a.intersect(&IRect::new(5, 5, 15, 15)),
            IRect::new(5, 5, 10, 10)
        );
        assert_eq!(a.intersect(&IRect::new(10, 10, 15, 15)), IRect::default());
        assert_eq!(a.union(&IRect::default()), a);
        assert_eq!(a.width(), 10);
        assert_eq!(IRect::new(5, 5, 0, 0).width(), 0);
        assert!(a.contains(9, 0));
        assert!(!a.contains(10, 0));
    }

    #[test]
    fn point_ops() {
        let p = Point::new(3.0, 4.0);
        assert_eq!(p.length(), 5.0);
        assert_eq!(p + Point::new(1.0, 1.0), Point::new(4.0, 5.0));
        assert_eq!(p * 2.0, Point::new(6.0, 8.0));
        assert_eq!(p.cross(Point::new(1.0, 0.0)), -4.0);
    }
}
