//! Bézier-Konturen in `f64` für die Pfadoperationen (Referenz `re\pfade\bezier.py`).
//!
//! Die Pfadoperationen rechnen durchgehend in `f64` (16 §6: python.dll speichert `f32`, Newton und
//! Löser rechnen `f64`; für eigene Operationen genügt `f64`). Ein- und Ausgabe in das Dokumentmodell
//! über [`Contour::from_bezier`] / [`Contour::to_bezier`] (`f32`).

use fp_geom::{BezierContour, BezierNode, CubicBez, Point, Vec2};

/// Knoten: Anker, Eingangs- und Ausgangshenkel (wie `PBTv`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Node {
    pub anchor: Vec2,
    /// Eingangshenkel (Henkel des vorigen Segments, `XPC/YPC`).
    pub ctrl_in: Vec2,
    /// Ausgangshenkel (Henkel des nächsten Segments, `XSC/YSC`).
    pub ctrl_out: Vec2,
}

impl Node {
    pub const fn new(anchor: Vec2, ctrl_in: Vec2, ctrl_out: Vec2) -> Self {
        Self { anchor, ctrl_in, ctrl_out }
    }

    /// Eckpunkt (Henkel auf dem Anker).
    pub const fn corner(p: Vec2) -> Self {
        Self { anchor: p, ctrl_in: p, ctrl_out: p }
    }
}

/// Kontur aus Knoten mit geschlossen-Flag.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Contour {
    pub nodes: Vec<Node>,
    pub closed: bool,
}

impl Contour {
    pub fn new(nodes: Vec<Node>, closed: bool) -> Self {
        Self { nodes, closed }
    }

    /// Polygon aus Eckpunkten.
    pub fn polygon(pts: &[Vec2], closed: bool) -> Self {
        Self::new(pts.iter().map(|p| Node::corner(*p)).collect(), closed)
    }

    /// Segmente `(A_i, Out_i, In_{i+1}, A_{i+1})`; geschlossen auch letzter → erster.
    pub fn segments(&self) -> Vec<CubicBez> {
        let n = self.nodes.len();
        let m = if self.closed { n } else { n.saturating_sub(1) };
        (0..m)
            .map(|i| {
                let (a, b) = (&self.nodes[i], &self.nodes[(i + 1) % n]);
                CubicBez::new(a.anchor, a.ctrl_out, b.ctrl_in, b.anchor)
            })
            .collect()
    }

    /// Richtung umkehren (Reihenfolge umdrehen, Henkel tauschen; geschlossen: Startknoten bleibt).
    pub fn reversed(&self) -> Contour {
        let mut nodes: Vec<Node> = self.nodes.iter().rev().map(|n| Node::new(n.anchor, n.ctrl_out, n.ctrl_in)).collect();
        if self.closed && !nodes.is_empty() {
            nodes.rotate_right(1);
        }
        Contour::new(nodes, self.closed)
    }

    /// Ankerpunkte.
    pub fn anchors(&self) -> Vec<Vec2> {
        self.nodes.iter().map(|n| n.anchor).collect()
    }

    /// Aus einer `f32`-Kontur des Modells.
    pub fn from_bezier(c: &BezierContour) -> Self {
        Self::new(c.nodes.iter().map(|n| Node::new(n.pt.into(), n.ctrl_in.into(), n.ctrl_out.into())).collect(), c.closed)
    }

    /// In eine `f32`-Kontur des Modells.
    pub fn to_bezier(&self) -> BezierContour {
        let p = |v: Vec2| Point::new(v.x as f32, v.y as f32);
        BezierContour::new(self.nodes.iter().map(|n| BezierNode::new(p(n.anchor), p(n.ctrl_in), p(n.ctrl_out))).collect(), self.closed)
    }

    /// Mit einer affinen Abbildung `(a, b, c, d, tx, ty)` (Zeilenvektor wie `MTXv`).
    pub fn transformed(&self, m: [f64; 6]) -> Contour {
        let f = |p: Vec2| Vec2::new(m[0] * p.x + m[2] * p.y + m[4], m[1] * p.x + m[3] * p.y + m[5]);
        Contour::new(self.nodes.iter().map(|n| Node::new(f(n.anchor), f(n.ctrl_in), f(n.ctrl_out))).collect(), self.closed)
    }
}

/// Abstand Punkt–Gerade durch `a`, `b` (bei `|ab| < 1e−12` Abstand zu `a`).
pub fn dline(p: Vec2, a: Vec2, b: Vec2) -> f64 {
    let l = (b.x - a.x).hypot(b.y - a.y);
    if l < 1e-12 {
        return (p.x - a.x).hypot(p.y - a.y);
    }
    ((b.x - a.x) * (a.y - p.y) - (a.x - p.x) * (b.y - a.y)).abs() / l
}

/// Nullstellen von `a·t² + b·t + c` in (0, 1) in der Reihenfolge der Referenz.
pub fn quad_roots(a: f64, b: f64, c: f64) -> Vec<f64> {
    let mut out = Vec::with_capacity(2);
    if a.abs() < 1e-12 {
        if b.abs() > 1e-12 {
            out.push(-c / b);
        }
    } else {
        let disc = b * b - 4.0 * a * c;
        if disc >= 0.0 {
            let s = disc.sqrt();
            out.push((-b + s) / (2.0 * a));
            out.push((-b - s) / (2.0 * a));
        }
    }
    out.retain(|t| *t > 0.0 && *t < 1.0);
    out
}

/// Schnittparameter `(u, v)` der Strecken a–b und c–d; `None` bei parallel/außerhalb.
pub fn line_line(a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> Option<(f64, f64)> {
    let (rx, ry) = (b.x - a.x, b.y - a.y);
    let (sx, sy) = (d.x - c.x, d.y - c.y);
    let den = rx * sy - ry * sx;
    if den.abs() < 1e-14 {
        return None;
    }
    let (qx, qy) = (c.x - a.x, c.y - a.y);
    let u = (qx * sy - qy * sx) / den;
    let v = (qx * ry - qy * rx) / den;
    if (-1e-12..=1.0 + 1e-12).contains(&u) && (-1e-12..=1.0 + 1e-12).contains(&v) {
        Some((u.clamp(0.0, 1.0), v.clamp(0.0, 1.0)))
    } else {
        None
    }
}

/// Rekursive Halbierung bei t = 0,5, bis beide Henkel höchstens `tol` von der Sehne entfernt sind
/// (Referenz `flatten_seg`, Tiefe ≤ `maxdepth`); Punkte ohne Startpunkt.
pub fn flatten_seg(seg: &CubicBez, tol: f64, maxdepth: u32, depth: u32, out: &mut Vec<Vec2>) {
    if seg.is_line() || depth >= maxdepth || dline(seg.p1, seg.p0, seg.p3).max(dline(seg.p2, seg.p0, seg.p3)) <= tol {
        out.push(seg.p3);
        return;
    }
    let (a, b) = seg.split(0.5);
    flatten_seg(&a, tol, maxdepth, depth + 1, out);
    flatten_seg(&b, tol, maxdepth, depth + 1, out);
}

/// Konturen → Polylinien `(Punkte, geschlossen)` (Referenz `flatten`).
pub fn flatten(contours: &[Contour], tol: f64) -> Vec<(Vec<Vec2>, bool)> {
    let mut out = Vec::new();
    for c in contours {
        let Some(first) = c.nodes.first() else { continue };
        let mut pts = vec![first.anchor];
        for s in c.segments() {
            flatten_seg(&s, tol, 10, 0, &mut pts);
        }
        if c.closed && pts.len() > 1 && pts.last() == pts.first() {
            pts.pop();
        }
        out.push((pts, c.closed));
    }
    out
}

/// Fläche nach Gauß (positiv = gegen den Uhrzeigersinn im y-oben-System).
pub fn signed_area(pts: &[Vec2]) -> f64 {
    let n = pts.len();
    let mut s = 0.0;
    for i in 0..n {
        let (a, b) = (pts[i], pts[(i + 1) % n]);
        s += a.x * b.y - b.x * a.y;
    }
    s / 2.0
}

/// Windungszahl bezüglich Polylinien (alle als geschlossen behandelt).
pub fn winding_polys(polys: &[(Vec<Vec2>, bool)], p: Vec2) -> i32 {
    let mut w = 0;
    for (pts, _) in polys {
        let n = pts.len();
        for i in 0..n {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
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

/// Vorzeichenbehaftete Gesamtfläche der abgeflachten Konturen (Referenz `boolops.area`).
pub fn area(contours: &[Contour], tol: f64) -> f64 {
    flatten(contours, tol).iter().map(|(p, _)| signed_area(p)).sum()
}

/// Exakte Grenzen eines Pfads (`exact`) bzw. Kontrollpunkt-Hülle; `None` ohne Knoten.
pub fn bounds(contours: &[Contour], exact: bool) -> Option<(Vec2, Vec2)> {
    let mut b: Option<(Vec2, Vec2)> = None;
    let mut add = |lo: Vec2, hi: Vec2| {
        b = Some(match b {
            None => (lo, hi),
            Some((l, h)) => (Vec2::new(l.x.min(lo.x), l.y.min(lo.y)), Vec2::new(h.x.max(hi.x), h.y.max(hi.y))),
        });
    };
    for c in contours {
        let segs = c.segments();
        if segs.is_empty() {
            if let Some(n) = c.nodes.first() {
                add(n.anchor, n.anchor);
            }
            continue;
        }
        for s in segs {
            let bb = if exact { s.bounds() } else { s.hull_bounds() };
            add(bb.min, bb.max);
        }
    }
    b
}
