//! Bézier-Kurven: kubisch ([`CubicBez`]) und quadratisch ([`QuadBez`]) in `f64`
//! (Spezifikation 16 §1.3–§1.5, Referenz `re\pfade\bezier.py`).
//!
//! Eigenheit wie im Original und in der Referenz: Ein kubisches Segment, dessen Henkel auf den
//! Ankern liegen (`p1 == p0` und `p2 == p3`), ist eine **Gerade** und wird **linear**
//! parametrisiert (Auswertung, Ableitung, Teilung). Die kubische Form hätte dort eine
//! ungleichmäßige Parametrisierung, und Schnitt-Parameter von Geraden passten sonst nicht.
//!
//! Die Formeln folgen in Reihenfolge und Klammerung der Python-Referenz, damit die Ergebnisse
//! bitgleich sind (Golden-Tests unter `tests/`).

use crate::{Point, Rect, Vec2};

/// Achsenparallele `f64`-Grenzen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    pub min: Vec2,
    pub max: Vec2,
}

impl Bounds {
    /// Grenzen eines einzelnen Punkts.
    pub fn from_point(p: Vec2) -> Bounds {
        Bounds { min: p, max: p }
    }

    /// Erweitert um einen Punkt.
    pub fn include(&mut self, p: Vec2) {
        self.min.x = self.min.x.min(p.x);
        self.min.y = self.min.y.min(p.y);
        self.max.x = self.max.x.max(p.x);
        self.max.y = self.max.y.max(p.y);
    }

    /// Vereinigung.
    pub fn union(&self, o: &Bounds) -> Bounds {
        let mut b = *self;
        b.include(o.min);
        b.include(o.max);
        b
    }

    /// Umwandlung in ein `f32`-[`Rect`] (gerundet auf `f32`).
    pub fn to_rect(&self) -> Rect {
        Rect::new(
            self.min.x as f32,
            self.min.y as f32,
            self.max.x as f32,
            self.max.y as f32,
        )
    }
}

/// Ergebnis einer Suche nach dem nächsten Kurvenpunkt.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Nearest {
    /// Parameter auf der Kurve (0..=1).
    pub t: f64,
    /// Abstand zum Suchpunkt.
    pub distance: f64,
    /// Kurvenpunkt.
    pub point: Vec2,
}

/// Kubisches Bézier-Segment `p0, p1, p2, p3` (Anker, Ausgangshenkel, Eingangshenkel, Anker).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CubicBez {
    pub p0: Vec2,
    pub p1: Vec2,
    pub p2: Vec2,
    pub p3: Vec2,
}

/// Gauß-Legendre-Stützstellen und Gewichte (5 Punkte).
const GL5: [(f64, f64); 5] = [
    (-0.906_179_845_938_664, 0.236_926_885_056_189_1),
    (-0.538_469_310_105_683_1, 0.478_628_670_499_366_5),
    (0.0, 0.568_888_888_888_888_9),
    (0.538_469_310_105_683_1, 0.478_628_670_499_366_5),
    (0.906_179_845_938_664, 0.236_926_885_056_189_1),
];

/// Schwelle für verschwindende Ableitungen ([`CubicBez::tangent`]).
const EPS: f64 = 1e-9;

impl CubicBez {
    pub const fn new(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2) -> Self {
        Self { p0, p1, p2, p3 }
    }

    /// Segment aus `f32`-Punkten.
    pub fn from_points(p0: Point, p1: Point, p2: Point, p3: Point) -> Self {
        Self::new(p0.into(), p1.into(), p2.into(), p3.into())
    }

    /// Gerade von `a` nach `b` (Henkel auf den Ankern).
    pub fn line(a: Vec2, b: Vec2) -> Self {
        Self::new(a, a, b, b)
    }

    /// Gerade (beide Henkel auf ihren Ankern)?
    #[inline]
    pub fn is_line(&self) -> bool {
        self.p1 == self.p0 && self.p2 == self.p3
    }

    /// Alle vier Punkte (nahezu) gleich? (|Δ| < 1e-12 je Koordinate.)
    pub fn is_degenerate(&self) -> bool {
        let p = self.p0;
        [self.p0, self.p1, self.p2, self.p3]
            .iter()
            .all(|q| (q.x - p.x).abs() < 1e-12 && (q.y - p.y).abs() < 1e-12)
    }

    /// Kurvenpunkt bei `t` (Geraden linear).
    pub fn eval(&self, t: f64) -> Vec2 {
        let (p0, p1, p2, p3) = (self.p0, self.p1, self.p2, self.p3);
        if self.is_line() {
            return Vec2::new(p0.x + (p3.x - p0.x) * t, p0.y + (p3.y - p0.y) * t);
        }
        let mt = 1.0 - t;
        let a = mt * mt * mt;
        let b = 3.0 * mt * mt * t;
        let c = 3.0 * mt * t * t;
        let d = t * t * t;
        Vec2::new(
            a * p0.x + b * p1.x + c * p2.x + d * p3.x,
            a * p0.y + b * p1.y + c * p2.y + d * p3.y,
        )
    }

    /// Erste Ableitung nach `t` (Geraden: konstant `p3 − p0`).
    pub fn deriv(&self, t: f64) -> Vec2 {
        let (p0, p1, p2, p3) = (self.p0, self.p1, self.p2, self.p3);
        if self.is_line() {
            return p3 - p0;
        }
        let mt = 1.0 - t;
        let a = 3.0 * mt * mt;
        let b = 6.0 * mt * t;
        let c = 3.0 * t * t;
        Vec2::new(
            a * (p1.x - p0.x) + b * (p2.x - p1.x) + c * (p3.x - p2.x),
            a * (p1.y - p0.y) + b * (p2.y - p1.y) + c * (p3.y - p2.y),
        )
    }

    /// Zweite Ableitung nach `t` (kubische Form, auch für Geraden).
    pub fn deriv2(&self, t: f64) -> Vec2 {
        let (p0, p1, p2, p3) = (self.p0, self.p1, self.p2, self.p3);
        let f = |a: f64, b: f64, c: f64, d: f64| {
            6.0 * (1.0 - t) * (c - 2.0 * b + a) + 6.0 * t * (d - 2.0 * c + b)
        };
        Vec2::new(f(p0.x, p1.x, p2.x, p3.x), f(p0.y, p1.y, p2.y, p3.y))
    }

    /// Einheitstangente bei `t`. Verschwindet die Ableitung (Henkel auf dem Anker), wird eine
    /// Ersatzrichtung zum nächsten verschiedenen Kontrollpunkt genommen.
    pub fn tangent(&self, t: f64) -> Vec2 {
        let (p0, p1, p2, p3) = (self.p0, self.p1, self.p2, self.p3);
        let mut d = self.deriv(t);
        let mut l = d.x.hypot(d.y);
        if l < EPS {
            if t < 0.5 {
                let q = if p1 == p0 { p2 } else { p1 };
                d = if q != p0 { q - p0 } else { p3 - p0 };
            } else {
                let q = if p2 == p3 { p1 } else { p2 };
                d = if q != p3 { p3 - q } else { p3 - p0 };
            }
            l = d.x.hypot(d.y);
            if l == 0.0 {
                l = 1.0;
            }
        }
        Vec2::new(d.x / l, d.y / l)
    }

    /// Teilt bei `t` per de Casteljau (Geraden linear, Henkel bleiben eingezogen).
    pub fn split(&self, t: f64) -> (CubicBez, CubicBez) {
        let (p0, p1, p2, p3) = (self.p0, self.p1, self.p2, self.p3);
        if self.is_line() {
            let m = self.eval(t);
            return (CubicBez::new(p0, p0, m, m), CubicBez::new(m, m, p3, p3));
        }
        let a = p0.lerp(p1, t);
        let b = p1.lerp(p2, t);
        let c = p2.lerp(p3, t);
        let d = a.lerp(b, t);
        let e = b.lerp(c, t);
        let f = d.lerp(e, t);
        (CubicBez::new(p0, a, d, f), CubicBez::new(f, e, c, p3))
    }

    /// Teilstück `[t0, t1]` (im Original `@005cf1b0`: t′ = (t1−t0)/(1−t0)). Geraden bleiben Geraden.
    pub fn subsegment(&self, t0: f64, t1: f64) -> CubicBez {
        if self.is_line() {
            let a = self.eval(t0);
            let b = self.eval(t1);
            return CubicBez::line(a, b);
        }
        let mut seg = *self;
        let mut t1 = t1;
        if t0 > 0.0 {
            seg = seg.split(t0).1;
            t1 = if t0 < 1.0 {
                (t1 - t0) / (1.0 - t0)
            } else {
                1.0
            };
        }
        if t1 < 1.0 {
            seg = seg.split(t1).0;
        }
        seg
    }

    /// Richtung umgekehrt (p3, p2, p1, p0).
    pub fn reversed(&self) -> CubicBez {
        CubicBez::new(self.p3, self.p2, self.p1, self.p0)
    }

    /// Parameter der Extremstellen je Achse in (0, 1) (Nullstellen von B′(t)); zuerst x, dann y.
    pub fn extrema(&self) -> Vec<f64> {
        let mut ts = Vec::with_capacity(4);
        let pts = [self.p0, self.p1, self.p2, self.p3];
        for k in 0..2 {
            let c = |p: Vec2| if k == 0 { p.x } else { p.y };
            let (q0, q1, q2, q3) = (c(pts[0]), c(pts[1]), c(pts[2]), c(pts[3]));
            let a = -q0 + 3.0 * q1 - 3.0 * q2 + q3;
            let b = 2.0 * (q0 - 2.0 * q1 + q2);
            let cc = q1 - q0;
            quad_roots_01(a, b, cc, &mut ts);
        }
        ts
    }

    /// Exakte Grenzen (Endpunkte und Kurvenpunkte an den Extremstellen).
    pub fn bounds(&self) -> Bounds {
        let mut b = Bounds::from_point(self.p0);
        b.include(self.p3);
        for t in self.extrema() {
            b.include(self.eval(t));
        }
        b
    }

    /// Kontrollpunkt-Hülle (Anker + Henkel), wie `python.dll` sie für die CPath-BBox benutzt.
    pub fn hull_bounds(&self) -> Bounds {
        let mut b = Bounds::from_point(self.p0);
        b.include(self.p1);
        b.include(self.p2);
        b.include(self.p3);
        b
    }

    fn gl_length(&self, a: f64, b: f64) -> f64 {
        let h = (b - a) / 2.0;
        let m = (a + b) / 2.0;
        let mut s = 0.0;
        for &(x, w) in GL5.iter() {
            let d = self.deriv(m + h * x);
            s += w * d.x.hypot(d.y);
        }
        s * h
    }

    fn length_rec(&self, t0: f64, t1: f64, depth: u32) -> f64 {
        let whole = self.gl_length(t0, t1);
        let mid = (t0 + t1) / 2.0;
        let halves = self.gl_length(t0, mid) + self.gl_length(mid, t1);
        if depth > 8 || (whole - halves).abs() < 1e-7 {
            return halves;
        }
        self.length_rec(t0, mid, depth + 1) + self.length_rec(mid, t1, depth + 1)
    }

    /// Bogenlänge zwischen `t0` und `t1` (Gauß-Legendre mit 5 Stützstellen, adaptiv halbiert).
    pub fn length_range(&self, t0: f64, t1: f64) -> f64 {
        self.length_rec(t0, t1, 0)
    }

    /// Bogenlänge des ganzen Segments.
    pub fn length(&self) -> f64 {
        self.length_range(0.0, 1.0)
    }

    /// Parameter, an dem die Bogenlänge `s` erreicht ist (Bisektion, 50 Schritte).
    pub fn t_at_length(&self, s: f64) -> f64 {
        let l = self.length();
        if s <= 0.0 {
            return 0.0;
        }
        if s >= l {
            return 1.0;
        }
        let (mut lo, mut hi) = (0.0, 1.0);
        for _ in 0..50 {
            let m = (lo + hi) / 2.0;
            if self.length_range(0.0, m) < s {
                lo = m;
            } else {
                hi = m;
            }
        }
        (lo + hi) / 2.0
    }

    /// Schnelle Längenschätzung wie `ApuQuickCurveLength` (16 §1.4, B):
    /// (|P0P1| + |P1P2| + |P2P3| + |P3P0|) / 2 mit der Schnellnorm max + min/2 (Fehler bis ≈ 12 %).
    pub fn quick_length(&self) -> f64 {
        let n = |a: Vec2, b: Vec2| {
            let dx = (b.x - a.x).abs();
            let dy = (b.y - a.y).abs();
            dx.max(dy) + dx.min(dy) / 2.0
        };
        (n(self.p0, self.p1) + n(self.p1, self.p2) + n(self.p2, self.p3) + n(self.p3, self.p0))
            / 2.0
    }

    /// Nächster Kurvenpunkt zu `p`: Abtastung mit `samples + 1` Stellen, Newton-Verfeinerung
    /// (höchstens 8 Schritte), danach Vergleich mit den Endpunkten (Referenz `nearest_on_seg`).
    pub fn nearest(&self, p: Vec2, samples: u32) -> Nearest {
        let samples = samples.max(1);
        let mut best_t = 0.0;
        let mut best_d = f64::INFINITY;
        for i in 0..=samples {
            let t = i as f64 / samples as f64;
            let q = self.eval(t);
            let d = (q.x - p.x).powi(2) + (q.y - p.y).powi(2);
            if d < best_d || i == 0 {
                best_t = t;
                best_d = d;
            }
        }
        let mut t = best_t;
        for _ in 0..8 {
            let q = self.eval(t);
            let d1 = self.deriv(t);
            let d2 = self.deriv2(t);
            let fx = q.x - p.x;
            let fy = q.y - p.y;
            let num = fx * d1.x + fy * d1.y;
            let den = d1.x * d1.x + d1.y * d1.y + fx * d2.x + fy * d2.y;
            if den.abs() < 1e-12 {
                break;
            }
            let nt = (t - num / den).clamp(0.0, 1.0);
            if (nt - t).abs() < 1e-10 {
                t = nt;
                break;
            }
            t = nt;
        }
        let mut q = self.eval(t);
        for te in [0.0, 1.0] {
            let qe = self.eval(te);
            if (qe.x - p.x).hypot(qe.y - p.y) < (q.x - p.x).hypot(q.y - p.y) {
                t = te;
                q = qe;
            }
        }
        Nearest {
            t,
            distance: (q.x - p.x).hypot(q.y - p.y),
            point: q,
        }
    }
}

/// Nullstellen von `a·t² + b·t + c` im offenen Intervall (0, 1), angehängt an `out`.
fn quad_roots_01(a: f64, b: f64, c: f64, out: &mut Vec<f64>) {
    let mut cand = [f64::NAN; 2];
    if a.abs() < 1e-12 {
        if b.abs() > 1e-12 {
            cand[0] = -c / b;
        }
    } else {
        let disc = b * b - 4.0 * a * c;
        if disc >= 0.0 {
            let s = disc.sqrt();
            cand[0] = (-b + s) / (2.0 * a);
            cand[1] = (-b - s) / (2.0 * a);
        }
    }
    for t in cand {
        if t > 0.0 && t < 1.0 {
            out.push(t);
        }
    }
}

/// Quadratisches Bézier-Segment `p0, p1, p2`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct QuadBez {
    pub p0: Vec2,
    pub p1: Vec2,
    pub p2: Vec2,
}

impl QuadBez {
    pub const fn new(p0: Vec2, p1: Vec2, p2: Vec2) -> Self {
        Self { p0, p1, p2 }
    }

    /// Kurvenpunkt bei `t`.
    pub fn eval(&self, t: f64) -> Vec2 {
        let mt = 1.0 - t;
        let a = mt * mt;
        let b = 2.0 * mt * t;
        let c = t * t;
        Vec2::new(
            a * self.p0.x + b * self.p1.x + c * self.p2.x,
            a * self.p0.y + b * self.p1.y + c * self.p2.y,
        )
    }

    /// Erste Ableitung.
    pub fn deriv(&self, t: f64) -> Vec2 {
        let mt = 1.0 - t;
        (self.p1 - self.p0) * (2.0 * mt) + (self.p2 - self.p1) * (2.0 * t)
    }

    /// Teilt bei `t` (de Casteljau).
    pub fn split(&self, t: f64) -> (QuadBez, QuadBez) {
        let a = self.p0.lerp(self.p1, t);
        let b = self.p1.lerp(self.p2, t);
        let m = a.lerp(b, t);
        (QuadBez::new(self.p0, a, m), QuadBez::new(m, b, self.p2))
    }

    /// Exakte Erhöhung auf eine kubische Kurve (Henkel bei 2/3).
    pub fn to_cubic(&self) -> CubicBez {
        let c1 = self.p0 + (self.p1 - self.p0) * (2.0 / 3.0);
        let c2 = self.p2 + (self.p1 - self.p2) * (2.0 / 3.0);
        CubicBez::new(self.p0, c1, c2, self.p2)
    }

    /// Exakte Grenzen.
    pub fn bounds(&self) -> Bounds {
        let mut b = Bounds::from_point(self.p0);
        b.include(self.p2);
        for k in 0..2 {
            let c = |p: Vec2| if k == 0 { p.x } else { p.y };
            let den = c(self.p0) - 2.0 * c(self.p1) + c(self.p2);
            if den.abs() > 1e-12 {
                let t = (c(self.p0) - c(self.p1)) / den;
                if t > 0.0 && t < 1.0 {
                    b.include(self.eval(t));
                }
            }
        }
        b
    }

    /// Bogenlänge (über die exakte kubische Form).
    pub fn length(&self) -> f64 {
        self.to_cubic().length()
    }

    /// Nächster Kurvenpunkt (über die exakte kubische Form).
    pub fn nearest(&self, p: Vec2, samples: u32) -> Nearest {
        self.to_cubic().nearest(p, samples)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: f64, y: f64) -> Vec2 {
        Vec2::new(x, y)
    }

    #[test]
    fn line_is_linear() {
        let l = CubicBez::line(v(0.0, 0.0), v(10.0, 0.0));
        assert_eq!(l.eval(0.25), v(2.5, 0.0));
        assert_eq!(l.split(0.3).0.p3, v(3.0, 0.0));
        assert!((l.length() - 10.0).abs() < 1e-12);
        assert_eq!(l.quick_length(), 10.0);
    }

    #[test]
    fn split_and_bounds() {
        let c = CubicBez::new(v(0.0, 0.0), v(0.0, 10.0), v(10.0, 10.0), v(10.0, 0.0));
        let (a, b) = c.split(0.5);
        assert_eq!(a.p3, c.eval(0.5));
        assert_eq!(b.p0, a.p3);
        let bb = c.bounds();
        assert!((bb.max.y - 7.5).abs() < 1e-12);
        assert_eq!(c.hull_bounds().max.y, 10.0);
        let s = c.subsegment(0.25, 0.75);
        assert!((s.p0.x - c.eval(0.25).x).abs() < 1e-12 && (s.p3.x - c.eval(0.75).x).abs() < 1e-12);
        let n = c.nearest(v(5.0, 20.0), 16);
        assert!((n.t - 0.5).abs() < 1e-9 && (n.distance - 12.5).abs() < 1e-9);
    }

    #[test]
    fn quad_basics() {
        let q = QuadBez::new(v(0.0, 0.0), v(5.0, 10.0), v(10.0, 0.0));
        let c = q.to_cubic();
        for t in [0.0, 0.3, 0.5, 1.0] {
            assert!((q.eval(t).y - c.eval(t).y).abs() < 1e-12);
        }
        assert!((q.bounds().max.y - 5.0).abs() < 1e-12);
    }
}
