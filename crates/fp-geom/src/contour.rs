//! Konturen: Bézier-Konturen aus Knoten ([`BezierContour`]) und Polylinien ([`Polyline`])
//! (Spezifikation 16 §1.1, 07 §3).
//!
//! Ein Knoten hat Anker `pt`, Eingangshenkel `ctrl_in` (Henkel des vorigen Segments, Datei `XPC/YPC`)
//! und Ausgangshenkel `ctrl_out` (Henkel des nächsten Segments, Datei `XSC/YSC`). Fehlende Henkel
//! liegen auf dem Anker. Segment i → i+1 ist `(pt_i, ctrl_out_i, ctrl_in_{i+1}, pt_{i+1})`; bei
//! geschlossener Kontur gibt es zusätzlich das Segment letzter → erster Knoten.

use crate::bezier::{Bounds, CubicBez, Nearest};
use crate::{Matrix3, Point, Vec2};

/// Bézier-Knoten wie im Dateiformat (`PBTv`): Anker und zwei Henkel, `f32`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BezierNode {
    /// Anker (`XLC/YLC`).
    pub pt: Point,
    /// Eingangshenkel (pred, `XPC/YPC`).
    pub ctrl_in: Point,
    /// Ausgangshenkel (succ, `XSC/YSC`).
    pub ctrl_out: Point,
}

impl BezierNode {
    pub const fn new(pt: Point, ctrl_in: Point, ctrl_out: Point) -> Self {
        Self {
            pt,
            ctrl_in,
            ctrl_out,
        }
    }

    /// Eckpunkt ohne Henkel (beide Henkel auf dem Anker).
    pub const fn corner(pt: Point) -> Self {
        Self {
            pt,
            ctrl_in: pt,
            ctrl_out: pt,
        }
    }

    /// Liegen beide Henkel auf dem Anker?
    pub fn is_corner(&self) -> bool {
        self.ctrl_in == self.pt && self.ctrl_out == self.pt
    }

    /// Ein- und Ausgangshenkel getauscht (für die Richtungsumkehr).
    pub fn swapped(&self) -> BezierNode {
        BezierNode::new(self.pt, self.ctrl_out, self.ctrl_in)
    }

    /// Alle drei Punkte mit einer Matrix transformiert.
    pub fn transformed(&self, m: &Matrix3) -> BezierNode {
        BezierNode::new(
            m.apply(self.pt),
            m.apply(self.ctrl_in),
            m.apply(self.ctrl_out),
        )
    }
}

/// Bézier-Kontur: Knotenliste und geschlossen-Flag (`PBPv`: `ISCb`).
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BezierContour {
    pub nodes: Vec<BezierNode>,
    pub closed: bool,
}

impl BezierContour {
    pub fn new(nodes: Vec<BezierNode>, closed: bool) -> Self {
        Self { nodes, closed }
    }

    /// Anzahl der Segmente (`n` geschlossen, `n − 1` offen, 0 bei weniger als 2 Knoten offen).
    pub fn segment_count(&self) -> usize {
        let n = self.nodes.len();
        if n == 0 {
            0
        } else if self.closed {
            n
        } else {
            n - 1
        }
    }

    /// Segment `i` als `f32`-Kontrollpunkte `[p0, p1, p2, p3]`; `None` außerhalb des Bereichs.
    pub fn segment_points(&self, i: usize) -> Option<[Point; 4]> {
        if i >= self.segment_count() {
            return None;
        }
        let a = &self.nodes[i];
        let b = &self.nodes[(i + 1) % self.nodes.len()];
        Some([a.pt, a.ctrl_out, b.ctrl_in, b.pt])
    }

    /// Segment `i` als [`CubicBez`] (`f64`).
    pub fn segment(&self, i: usize) -> Option<CubicBez> {
        self.segment_points(i)
            .map(|[a, b, c, d]| CubicBez::from_points(a, b, c, d))
    }

    /// Alle Segmente in Reihenfolge.
    pub fn segments(&self) -> impl Iterator<Item = CubicBez> + '_ {
        (0..self.segment_count()).filter_map(move |i| self.segment(i))
    }

    /// Richtung umkehren (wie `ApuReversePath`): Knotenreihenfolge umdrehen, Henkel tauschen.
    /// Bei geschlossenen Konturen bleibt der Startknoten erhalten.
    pub fn reversed(&self) -> BezierContour {
        let mut nodes: Vec<BezierNode> = self.nodes.iter().rev().map(BezierNode::swapped).collect();
        if self.closed && !nodes.is_empty() {
            nodes.rotate_right(1);
        }
        BezierContour::new(nodes, self.closed)
    }

    /// Punkt einfügen: Segment `seg_index` bei `t` per de Casteljau teilen (16 §3.6). Die Form
    /// bleibt erhalten, die Nachbarhenkel werden gekürzt. `None` bei ungültigem Index.
    pub fn insert_node(&self, seg_index: usize, t: f64) -> Option<BezierContour> {
        let seg = self.segment(seg_index)?;
        let n = self.nodes.len();
        let (i, j) = (seg_index, (seg_index + 1) % n);
        let (l, r) = seg.split(t);
        let mut nodes = self.nodes.clone();
        nodes[i].ctrl_out = l.p1.to_point();
        let new = BezierNode::new(l.p3.to_point(), l.p2.to_point(), r.p1.to_point());
        nodes[j].ctrl_in = r.p2.to_point();
        nodes.insert(i + 1, new);
        Some(BezierContour::new(nodes, self.closed))
    }

    /// Punkt entfernen: Knoten löschen, die Nachbarn behalten ihre Henkel (16 §3.6, A).
    pub fn remove_node(&self, index: usize) -> BezierContour {
        let nodes: Vec<BezierNode> = self
            .nodes
            .iter()
            .enumerate()
            .filter(|(k, _)| *k != index)
            .map(|(_, n)| *n)
            .collect();
        let closed = self.closed && nodes.len() > 2;
        BezierContour::new(nodes, closed)
    }

    /// Grenzen: `exact = true` exakte Kurvengrenzen, sonst Hülle aller Anker und Henkel.
    /// `None` bei leerer Kontur.
    pub fn bounds(&self, exact: bool) -> Option<Bounds> {
        let mut out: Option<Bounds> = None;
        let mut add = |b: Bounds| {
            out = Some(match out {
                Some(o) => o.union(&b),
                None => b,
            })
        };
        if self.segment_count() == 0
            && let Some(n) = self.nodes.first()
        {
            add(Bounds::from_point(n.pt.into()));
        }
        for s in self.segments() {
            add(if exact { s.bounds() } else { s.hull_bounds() });
        }
        out
    }

    /// Nächster Punkt auf der Kontur: `(segment_index, Nearest)`.
    pub fn nearest(&self, p: Vec2) -> Option<(usize, Nearest)> {
        let mut best: Option<(usize, Nearest)> = None;
        for (si, s) in self.segments().enumerate() {
            let bx = s.bounds();
            let dx = (bx.min.x - p.x).max(0.0).max(p.x - bx.max.x);
            let dy = (bx.min.y - p.y).max(0.0).max(p.y - bx.max.y);
            if let Some((_, b)) = &best
                && dx.hypot(dy) > b.distance
            {
                continue;
            }
            let n = s.nearest(p, 16);
            if best.as_ref().is_none_or(|(_, b)| n.distance < b.distance) {
                best = Some((si, n));
            }
        }
        best
    }

    /// Alle Knoten mit einer Matrix transformiert.
    pub fn transformed(&self, m: &Matrix3) -> BezierContour {
        BezierContour::new(
            self.nodes.iter().map(|n| n.transformed(m)).collect(),
            self.closed,
        )
    }
}

/// Abgeflachte Kontur (Polylinie, Datei `PPLv`/`PPTv`).
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Polyline {
    pub points: Vec<Point>,
    pub closed: bool,
}

impl Polyline {
    pub fn new(points: Vec<Point>, closed: bool) -> Self {
        Self { points, closed }
    }

    /// Vorzeichenbehaftete Fläche nach Gauß (Kontur implizit geschlossen); siehe [`signed_area`].
    pub fn signed_area(&self) -> f64 {
        signed_area(&self.points)
    }

    /// Grenzen der Punkte.
    pub fn bounds(&self) -> Option<crate::Rect> {
        crate::Rect::from_points(self.points.iter().copied())
    }
}

/// Fläche nach Gauß: positiv = gegen den Uhrzeigersinn im mathematischen (y-oben-)System, also
/// **im Uhrzeigersinn auf dem Bildschirm** (y nach unten). Rechnung in `f64`.
pub fn signed_area(pts: &[Point]) -> f64 {
    let n = pts.len();
    let mut s = 0.0;
    for i in 0..n {
        let a = pts[i];
        let b = pts[(i + 1) % n];
        s += a.x as f64 * b.y as f64 - b.x as f64 * a.y as f64;
    }
    s / 2.0
}

/// Windungszahl eines Punkts bezüglich mehrerer (implizit geschlossener) Polylinien.
/// Halboffene Kantenregel `y0 ≤ y < y1`, Zählung für Schnittpunkte rechts vom Punkt.
pub fn winding_number<'a, I>(polys: I, p: Vec2) -> i32
where
    I: IntoIterator<Item = &'a [Point]>,
{
    let mut w = 0;
    for pts in polys {
        let n = pts.len();
        for i in 0..n {
            let (a, b) = (Vec2::from(pts[i]), Vec2::from(pts[(i + 1) % n]));
            if (a.y <= p.y && p.y < b.y) || (b.y <= p.y && p.y < a.y) {
                let xi = a.x + (p.y - a.y) * (b.x - a.x) / (b.y - a.y);
                if xi > p.x {
                    w += if b.y > a.y { 1 } else { -1 };
                }
            }
        }
    }
    w
}

/// Liegt der Punkt innen? `even_odd` = Gerade/ungerade-Regel (`EOFb = 1`), sonst Non-Zero.
pub fn point_inside<'a, I>(polys: I, p: Vec2, even_odd: bool) -> bool
where
    I: IntoIterator<Item = &'a [Point]>,
{
    let w = winding_number(polys, p);
    if even_odd { w & 1 == 1 } else { w != 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> BezierContour {
        let p = |x, y| BezierNode::corner(Point::new(x, y));
        BezierContour::new(
            vec![p(0.0, 0.0), p(10.0, 0.0), p(10.0, 10.0), p(0.0, 10.0)],
            true,
        )
    }

    #[test]
    fn segments_and_reverse() {
        let c = square();
        assert_eq!(c.segment_count(), 4);
        assert_eq!(c.segment_points(3).unwrap()[3], Point::new(0.0, 0.0));
        let r = c.reversed();
        assert_eq!(r.nodes[0].pt, Point::new(0.0, 0.0));
        assert_eq!(r.nodes[1].pt, Point::new(0.0, 10.0));
        let open = BezierContour::new(c.nodes.clone(), false);
        assert_eq!(open.segment_count(), 3);
        assert_eq!(open.reversed().nodes[0].pt, Point::new(0.0, 10.0));
    }

    #[test]
    fn area_and_winding() {
        let pts = [
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(10.0, 10.0),
            Point::new(0.0, 10.0),
        ];
        assert_eq!(signed_area(&pts), 100.0);
        assert_eq!(winding_number([&pts[..]], Vec2::new(5.0, 5.0)), 1);
        assert!(point_inside([&pts[..]], Vec2::new(5.0, 5.0), true));
        assert!(!point_inside([&pts[..]], Vec2::new(15.0, 5.0), false));
    }

    #[test]
    fn insert_remove() {
        let c = square();
        let c2 = c.insert_node(0, 0.5).unwrap();
        assert_eq!(c2.nodes.len(), 5);
        assert_eq!(c2.nodes[1].pt, Point::new(5.0, 0.0));
        assert_eq!(c2.remove_node(1), c);
        assert!(c.insert_node(9, 0.5).is_none());
        let b = c.bounds(true).unwrap();
        assert_eq!(b.max, Vec2::new(10.0, 10.0));
        let (si, n) = c.nearest(Vec2::new(5.0, -3.0)).unwrap();
        assert_eq!(si, 0);
        assert!((n.distance - 3.0).abs() < 1e-12);
    }
}
