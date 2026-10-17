//! Curve refit and extrema simplification (re 30 §2.3–§2.5).
//!
//! - [`refit_contour`]: removes nodes and fits the curves anew; the result consists only of nodes
//!   of the (normalised) input. Tolerance, corner angle and sample count as the call site passes
//!   them ("Simplify": amount/30°/16, vector path tool: `10 − precision`/30°/16, expand
//!   stroke/inset path: width/30°/16).
//! - [`simplify_extrema`]: preliminary stage of expand stroke and inset path; splits curves at the
//!   x/y extrema, marks extrema as fixed break points and refits in up to six stages
//!   ([`SIMPLIFY_STAGES`]).
//! - [`push_curve`]: normalisation of every curve when contours are built and when the refit
//!   emits them (degenerate handles ⇒ line, loops/cusps split at the minimum speed).
//!
//! Peculiarities kept on purpose: approximate distance `max + ½·min` ([`adist`]) where the rule
//! names it, handle retraction below `1.3333334·tol`, splitting only at original nodes, mirrored
//! corner tangents, fit target `tol/10` with the slow-progress test `0.89999998`. Computed in `f64`
//! (the original works in `f32`; differences stay below 0.01 px in about 99 % of the cases).
//! Approximated (A): the fit through the middle sample for three samples, the hull test of the
//! loop check (plain triangle test both ways) and the wrap-around of extremum marks on closed
//! contours (the scan starts at the global opposite extreme).

use super::geom::{Contour, Node};
use fp_geom::Vec2;

const EPS: f64 = 0.005;
const PULL: f64 = 1.333_333_4;
const SLOW: f64 = 0.899_999_98;

/// Freehand curve fitting defaults of the original (preferences `BezFitTolerance2`,
/// `BezFitCornerAngle`, `BezFitMaxPctLenDiff`, `BezFitExtraTightTolerance`,
/// `BezFitExtraTightToleranceStep`; read from the preference file in measurement M6 §2c). Only the
/// values are known; their effect when changed is unmeasured, so nothing reads them yet.
pub const BEZ_FIT_TOLERANCE: f64 = 1.0;
pub const BEZ_FIT_CORNER_ANGLE: f64 = 70.0;
pub const BEZ_FIT_MAX_PCT_LEN_DIFF: f64 = 0.02;
pub const BEZ_FIT_EXTRA_TIGHT_TOLERANCE: f64 = 0.001;
pub const BEZ_FIT_EXTRA_TIGHT_TOLERANCE_STEP: f64 = -0.5;

/// Stages of the extrema simplification: (tolerance factor × diagonal, corner angle, samples).
pub const SIMPLIFY_STAGES: [(f64, f64, u32); 6] = [
    (0.01, 30.0, 16),
    (0.012, 30.0, 16),
    (0.025, 30.0, 16),
    (0.05, 45.0, 12),
    (0.1, 60.0, 8),
    (0.2, 60.0, 8),
];

const F_CORNER: u32 = 0x01;
const F_SMOOTH: u32 = 0x02;
const F_LOPSIDED: u32 = 0x04;
const F_NODE: u32 = 0x10;
const F_VERT: u32 = 0x20;
const F_HORZ: u32 = 0x40;
const F_FIXED: u32 = 0x80;

/// Cubic segment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Seg {
    pub p0: Vec2,
    pub c1: Vec2,
    pub c2: Vec2,
    pub p3: Vec2,
}

impl Seg {
    pub fn new(p0: Vec2, c1: Vec2, c2: Vec2, p3: Vec2) -> Seg {
        Seg { p0, c1, c2, p3 }
    }

    pub fn line(a: Vec2, b: Vec2) -> Seg {
        Seg::new(a, a, b, b)
    }

    /// Straight: `c2 = P3` and (`c1 = c2` or `c1 = P0`), exact comparison.
    pub fn is_line(&self) -> bool {
        self.c2 == self.p3 && (self.c1 == self.c2 || self.c1 == self.p0)
    }

    /// Polynomial coefficients `a0 + t(a1 + t(a2 + t·a3))`.
    fn poly(&self) -> [Vec2; 4] {
        let (p0, c1, c2, p3) = (self.p0, self.c1, self.c2, self.p3);
        [
            p0,
            (c1 - p0) * 3.0,
            (p0 - c1 * 2.0 + c2) * 3.0,
            p3 - p0 + (c1 - c2) * 3.0,
        ]
    }

    pub fn eval(&self, t: f64) -> Vec2 {
        let [a0, a1, a2, a3] = self.poly();
        a0 + (a1 + (a2 + a3 * t) * t) * t
    }

    fn d1(&self, t: f64) -> Vec2 {
        let [_, a1, a2, a3] = self.poly();
        a1 + (a2 * 2.0 + a3 * (3.0 * t)) * t
    }

    fn d2(&self, t: f64) -> Vec2 {
        let [_, _, a2, a3] = self.poly();
        a2 * 2.0 + a3 * (6.0 * t)
    }

    /// de Casteljau split.
    pub fn split(&self, t: f64) -> (Seg, Seg) {
        let ab = self.p0.lerp(self.c1, t);
        let bc = self.c1.lerp(self.c2, t);
        let cd = self.c2.lerp(self.p3, t);
        let abc = ab.lerp(bc, t);
        let bcd = bc.lerp(cd, t);
        let m = abc.lerp(bcd, t);
        (Seg::new(self.p0, ab, abc, m), Seg::new(m, bcd, cd, self.p3))
    }
}

/// Approximate distance `max(|dx|, |dy|) + ½·min(|dx|, |dy|)`.
pub fn adist(a: Vec2, b: Vec2) -> f64 {
    let (dx, dy) = ((a.x - b.x).abs(), (a.y - b.y).abs());
    dx.max(dy) + 0.5 * dx.min(dy)
}

fn near(a: Vec2, b: Vec2) -> bool {
    (a.x - b.x).abs() < EPS && (a.y - b.y).abs() < EPS
}

fn unit(v: Vec2) -> Vec2 {
    let l = v.length();
    if l > 0.0 { v / l } else { Vec2::ZERO }
}

/// Distance of `p` from the line through `a`, `b` (distance to `a` when they coincide).
fn chord_dist(p: Vec2, a: Vec2, b: Vec2) -> f64 {
    let l = a.distance(b);
    if l <= 0.0 {
        p.distance(a)
    } else {
        (b - a).cross(p - a).abs() / l
    }
}

// ---------------------------------------------------------------------------------------------
// Normalisation (§2.5)

fn side(p: Vec2, a: Vec2, b: Vec2) -> f64 {
    let c = (b - a).cross(p - a);
    if c > 0.0 {
        1.0
    } else if c < 0.0 {
        -1.0
    } else {
        0.0
    }
}

/// Area measure with the original's peculiarity: only the second product is halved.
fn area_odd(p1: Vec2, p2: Vec2, p3: Vec2) -> f64 {
    ((p3.x - p1.x) * (p2.y - p1.y) - (p2.x - p1.x) * (p3.y - p1.y) / 2.0).abs()
}

const IN_TRI: [u8; 14] = [1, 1, 0, 1, 1, 0, 0, 0, 0, 1, 1, 0, 1, 1];

fn in_triangle(p: Vec2, a: Vec2, b: Vec2, c: Vec2) -> bool {
    let s = |u: Vec2, v: Vec2| -> i32 { side(p, u, v) as i32 };
    let i = (9 * s(a, b) + 3 * s(b, c) + s(c, a)).unsigned_abs() as usize;
    IN_TRI.get(i).copied().unwrap_or(0) == 1
}

/// Parameter of the minimum speed `|B′(t)|²` in (0, 1), else 0.5.
fn min_speed_t(s: &Seg) -> f64 {
    // zero of g = B′·B″ (derivative of |B′|²/2), bracketed on a grid and bisected
    let g = |t: f64| s.d1(t).dot(s.d2(t));
    let steps = 256;
    let mut best: Option<(f64, f64)> = None;
    let mut t0 = 0.0;
    let mut g0 = g(0.0);
    for k in 1..=steps {
        let t1 = k as f64 / steps as f64;
        let g1 = g(t1);
        if g0 == 0.0 || (g0 < 0.0 && g1 > 0.0) || (g0 > 0.0 && g1 < 0.0) {
            let (mut lo, mut hi, mut glo) = (t0, t1, g0);
            if g0 != 0.0 {
                for _ in 0..60 {
                    let m = 0.5 * (lo + hi);
                    let gm = g(m);
                    if (gm < 0.0) == (glo < 0.0) {
                        lo = m;
                        glo = gm;
                    } else {
                        hi = m;
                    }
                }
            }
            let r = if g0 == 0.0 { t0 } else { 0.5 * (lo + hi) };
            if r > 0.0 && r < 1.0 {
                let sp = s.d1(r).length_sq();
                if best.is_none_or(|(_, b)| sp < b) {
                    best = Some((r, sp));
                }
            }
        }
        t0 = t1;
        g0 = g1;
    }
    best.map(|b| b.0).unwrap_or(0.5)
}

fn loops(s: &Seg) -> bool {
    let (p0, c1, c2, p3) = (s.p0, s.c1, s.c2, s.p3);
    let crossed =
        side(p0, p3, c2) * side(c1, p3, c2) < 0.0 && side(p3, p0, c1) * side(c2, p0, c1) < 0.0;
    if crossed {
        let f = area_odd(p3, p0, c1);
        let g = (area_odd(p3, c2, c1).sqrt() + EPS).powi(2);
        f <= g
    } else {
        in_triangle(p3, p0, c1, c2) || in_triangle(p0, p3, c2, c1)
    }
}

/// Appends a curve with the normalisation of §2.5 (line for degenerate handles, split at loops).
pub fn push_curve(out: &mut Vec<Seg>, s: Seg) {
    push_curve_depth(out, s, 0);
}

fn push_curve_depth(out: &mut Vec<Seg>, s: Seg, depth: u32) {
    if s.p0 == s.p3 && s.c1 == s.p0 && s.c2 == s.p3 {
        return;
    }
    if (s.c1 == s.p0 && s.c2 == s.p3)
        || (chord_dist(s.c1, s.p0, s.p3) <= EPS
            && chord_dist(s.c2, s.p0, s.p3) <= EPS
            && s.p0 != s.p3)
    {
        out.push(Seg::line(s.p0, s.p3));
        return;
    }
    let l1 = |a: Vec2, b: Vec2| (a.x - b.x).abs() + (a.y - b.y).abs();
    if depth < 2 && l1(s.c1, s.p0) >= EPS && l1(s.c2, s.p3) >= EPS && loops(&s) {
        let t = min_speed_t(&s);
        if t > 0.002 && t < 0.998 {
            let (a, b) = s.split(t);
            push_curve_depth(out, a, depth + 1);
            push_curve_depth(out, b, depth + 1);
            return;
        }
    }
    out.push(s);
}

/// Contour → normalised segments (zero-length straight pieces dropped).
pub fn contour_segs(c: &Contour) -> Vec<Seg> {
    let mut out = Vec::new();
    for s in c.segments() {
        push_curve(&mut out, Seg::new(s.p0, s.p1, s.p2, s.p3));
    }
    out
}

/// Segments → contour.
pub fn segs_contour(segs: &[Seg], closed: bool) -> Contour {
    let m = segs.len();
    if m == 0 {
        return Contour::new(Vec::new(), closed);
    }
    let mut nodes = Vec::with_capacity(m + 1);
    for k in 0..m {
        let cin = if k > 0 {
            segs[k - 1].c2
        } else if closed {
            segs[m - 1].c2
        } else {
            segs[0].p0
        };
        nodes.push(Node::new(segs[k].p0, cin, segs[k].c1));
    }
    if !closed {
        nodes.push(Node::new(segs[m - 1].p3, segs[m - 1].c2, segs[m - 1].p3));
    }
    Contour::new(nodes, closed)
}

fn control_diag(segs: &[Seg]) -> f64 {
    let (mut x0, mut y0, mut x1, mut y1) = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    for s in segs {
        for p in [s.p0, s.c1, s.c2, s.p3] {
            x0 = x0.min(p.x);
            y0 = y0.min(p.y);
            x1 = x1.max(p.x);
            y1 = y1.max(p.y);
        }
    }
    if x0.is_finite() {
        (x1 - x0).hypot(y1 - y0)
    } else {
        0.0
    }
}

// ---------------------------------------------------------------------------------------------
// Refit (§2.3)

#[derive(Debug, Clone, Copy)]
struct Smp {
    p: Vec2,
    flag: u32,
}

/// Extra behaviour of the simplification (§2.4): split at extrema (x, y) and mark extrema (x, y).
#[derive(Debug, Clone, Copy, Default)]
struct Extrema {
    split_x: bool,
    split_y: bool,
    mark_x: bool,
    mark_y: bool,
}

struct Fit<'a> {
    s: &'a [Smp],
    closed: bool,
    tol: f64,
    diag: f64,
    mask: u32,
    out: Vec<Seg>,
    split: bool,
}

/// Node class (§2.3 d). `tin`/`tout`: 1 straight, 2 curve.
fn classify(p: Vec2, prev: Vec2, next: Vec2, tin: u8, tout: u8, thr: f64, tol: f64) -> u32 {
    let (va, vb) = (prev - p, p - next);
    let (la, lb) = (va.length(), vb.length());
    if la < EPS || lb < EPS {
        return F_CORNER;
    }
    let (a, b) = (va / la, vb / lb);
    let smooth = (a - b).length() < thr;
    let short = la < tol && lb < tol;
    let mut f = F_CORNER;
    match (tin, tout) {
        (1, 1) => {
            if smooth {
                f = if la > 4.0 * lb || lb > 4.0 * la {
                    F_LOPSIDED
                } else {
                    F_SMOOTH
                };
            } else if short {
                f = F_SMOOTH;
            }
        }
        (2, 2) => {
            if smooth {
                f = F_SMOOTH;
            }
        }
        (2, 1) => {
            if smooth {
                f = if lb > 4.0 * la { F_LOPSIDED } else { F_SMOOTH };
            } else if short {
                f = F_SMOOTH;
            }
        }
        _ => {
            if smooth {
                f = if la > 4.0 * lb { F_LOPSIDED } else { F_SMOOTH };
            } else if short {
                f = F_SMOOTH;
            }
        }
    }
    if f == F_SMOOTH {
        let s = a + b;
        if s.x.abs() * 200.0 < s.y.abs() {
            f |= F_VERT;
        } else if s.x.abs() > 200.0 * s.y.abs() {
            f |= F_HORZ;
        }
    }
    f | F_NODE
}

/// Quadratic roots of a derivative component as the extremum split takes them.
fn extremum_t(a: f64, b: f64, c: f64) -> Option<f64> {
    let t = if a.abs() < 1e-12 {
        if b.abs() < 1e-12 {
            return None;
        }
        -c / b
    } else {
        let disc = b * b - 4.0 * a * c;
        if disc < 0.0 {
            return None;
        }
        let s = disc.sqrt();
        let (mut r1, mut r2) = ((-b + s) / (2.0 * a), (-b - s) / (2.0 * a));
        if r1 < 0.006 {
            r1 = 1.0;
        }
        if r2 < 0.006 {
            r2 = 1.0;
        }
        r1.min(r2)
    };
    (0.006..=0.994).contains(&t).then_some(t)
}

fn split_extrema(segs: &mut Vec<Seg>, ex: Extrema, tol: f64) {
    let mut k = 0;
    while k < segs.len() {
        let s = segs[k];
        if !s.is_line() {
            let mut best: Option<f64> = None;
            for (on, pick) in [(ex.split_x, 0usize), (ex.split_y, 1usize)] {
                if !on {
                    continue;
                }
                let g = |v: Vec2| if pick == 0 { v.x } else { v.y };
                let (p0, c1, c2, p3) = (g(s.p0), g(s.c1), g(s.c2), g(s.p3));
                let a = 3.0 * (-p0 + 3.0 * c1 - 3.0 * c2 + p3);
                let b = 6.0 * (p0 - 2.0 * c1 + c2);
                let c = 3.0 * (c1 - p0);
                if let Some(t) = extremum_t(a, b, c) {
                    let q = s.eval(t);
                    if adist(q, s.p0).min(adist(q, s.p3)) > 2.0 * tol && best.is_none_or(|b| t < b)
                    {
                        best = Some(t);
                    }
                }
            }
            if let Some(t) = best {
                let (l, r) = s.split(t);
                segs[k] = l;
                segs.insert(k + 1, r);
            }
        }
        k += 1;
    }
}

/// Marks extrema of one coordinate (§2.4, flags 4/8).
fn mark_extrema(s: &mut [Smp], closed: bool, axis_x: bool, tol: f64) {
    let n = s.len();
    if n < 3 {
        return;
    }
    let coord = |p: Vec2| if axis_x { p.x } else { p.y };
    let (mut lo, mut hi) = (
        (f64::INFINITY, f64::INFINITY),
        (f64::NEG_INFINITY, f64::NEG_INFINITY),
    );
    for x in s.iter() {
        lo = (lo.0.min(x.p.x), lo.1.min(x.p.y));
        hi = (hi.0.max(x.p.x), hi.1.max(x.p.y));
    }
    // plateau width compared with 0.1·height (x) or 0.1·width (y)
    let extent = if axis_x { hi.1 - lo.1 } else { hi.0 - lo.0 };
    let mark = if axis_x {
        F_FIXED | F_VERT
    } else {
        F_FIXED | F_HORZ
    };
    let noise = (tol / 16.0).max(EPS);
    for sign in [1.0f64, -1.0] {
        let w = |i: usize| sign * coord(s[i % n].p);
        let start = if closed {
            (0..n).max_by(|&a, &b| w(a).total_cmp(&w(b))).unwrap_or(0)
        } else {
            0
        };
        let len = if closed { n + 1 } else { n };
        let idx = |pos: usize| (start + pos) % n;
        let mut last = w(idx(0));
        let mut prev_d = 0.0f64;
        let mut plateau = false;
        let mut cand: Option<(usize, usize, f64)> = None;
        let mut marks: Vec<(usize, usize)> = Vec::new();
        for pos in 1..len {
            let v = w(idx(pos));
            let mut d = v - last;
            if d.abs() < EPS / 2.0 {
                d = 0.0;
            }
            if prev_d < 0.0 && d >= 0.0 {
                cand = Some((pos - 1, if d == 0.0 { pos } else { pos - 1 }, last));
                plateau = d == 0.0;
            } else if d == 0.0 {
                if plateau && let Some(c) = cand.as_mut() {
                    c.1 = pos;
                }
            } else {
                plateau = false;
            }
            if d != 0.0 {
                prev_d = d;
                last = v;
            } else if prev_d < 0.0 {
                prev_d = 0.0;
            }
            if let Some(c) = cand {
                if v - c.2 > tol {
                    marks.push((c.0, c.1));
                    cand = None;
                    plateau = false;
                } else if c.2 - v > noise {
                    cand = None;
                    plateau = false;
                }
            }
        }
        for (a, b) in marks {
            let (ia, ib) = (idx(a), idx(b));
            if a == b {
                s[ia].flag |= mark;
            } else if adist(s[ia].p, s[ib].p) < 0.1 * extent {
                let mid = (a + b) / 2;
                let pick = (0..=(b - a))
                    .flat_map(|k| [mid.checked_sub(k), Some(mid + k)])
                    .flatten()
                    .filter(|&p| p >= a && p <= b)
                    .find(|&p| s[idx(p)].flag & F_NODE != 0);
                s[idx(pick.unwrap_or(mid))].flag |= mark;
            } else {
                s[ia].flag |= mark;
                s[ib].flag |= mark;
            }
        }
    }
}

impl Fit<'_> {
    fn n(&self) -> usize {
        self.s.len()
    }

    fn flag(&self, i: usize) -> u32 {
        self.s[i % self.n()].flag
    }

    fn pt(&self, i: usize) -> Vec2 {
        self.s[i % self.n()].p
    }

    /// Forward and backward differences to the neighbouring samples with flag ≠ 0.
    fn fr(&self, i: usize) -> (Vec2, Vec2) {
        let n = self.n();
        let i = i % n;
        let step = |dir: i64| -> Option<usize> {
            let mut j = i as i64;
            for _ in 0..n {
                j += dir;
                if !self.closed && (j < 0 || j >= n as i64) {
                    return None;
                }
                let k = j.rem_euclid(n as i64) as usize;
                if k == i {
                    return None;
                }
                if self.s[k].flag != 0 {
                    return Some(k);
                }
            }
            None
        };
        let p = self.s[i].p;
        let f = step(1).map(|k| self.s[k].p - p);
        let r = step(-1).map(|k| p - self.s[k].p);
        let (mut f, mut r) = (f.or(r).unwrap_or(Vec2::ZERO), r.or(f).unwrap_or(Vec2::ZERO));
        let fl = self.s[i].flag;
        if fl & F_SMOOTH != 0 {
            if fl & F_HORZ != 0 {
                f.y = 0.0;
                r.y = 0.0;
            }
            if fl & F_VERT != 0 {
                f.x = 0.0;
                r.x = 0.0;
            }
        }
        (f, r)
    }

    fn t_start(&self, i: usize) -> Vec2 {
        let (f, r) = self.fr(i);
        let fl = self.flag(i);
        unit(if fl & F_SMOOTH != 0 {
            f + r
        } else if fl & F_LOPSIDED != 0 {
            if f.length() > r.length() { f } else { r }
        } else {
            f
        })
    }

    fn t_end(&self, i: usize) -> Vec2 {
        let (f, r) = self.fr(i);
        let fl = self.flag(i);
        let v = if fl & F_SMOOTH != 0 {
            r + f
        } else if fl & F_LOPSIDED != 0 {
            if f.length() > r.length() { f } else { r }
        } else {
            r
        };
        unit(-v)
    }

    /// Next sample with the split mask, searching alternately left and right of `p`.
    fn knot(&self, p: usize, a: usize, b: usize) -> usize {
        if self.flag(p) & self.mask != 0 {
            return p;
        }
        let mut k = 1;
        loop {
            let l = p.checked_sub(k).filter(|&l| l > a);
            let r = Some(p + k).filter(|&r| r < b);
            if let Some(l) = l
                && self.flag(l) & self.mask != 0
            {
                return l;
            }
            if let Some(r) = r
                && self.flag(r) & self.mask != 0
            {
                return r;
            }
            if l.is_none() && r.is_none() {
                return p;
            }
            k += 1;
        }
    }

    fn emit(&mut self, s: Seg, k: f64) {
        if chord_dist(s.c1, s.p0, s.p3) <= k && chord_dist(s.c2, s.p0, s.p3) < k {
            if s.p0 != s.p3 {
                self.out.push(Seg::line(s.p0, s.p3));
            }
        } else {
            push_curve(&mut self.out, s);
        }
    }

    fn range(&mut self, a: usize, b: usize, depth: u32) {
        let n = self.n();
        let full = self.closed && b - a == n;
        if depth < 64 && ((full && self.flag(a) & F_SMOOTH != 0) || b - a + 1 > 1000) {
            let s = self.knot((a + 1 + b) >> 1, a, b);
            if s > a && s < b {
                self.split = true;
                self.range(a, s, depth + 1);
                self.range(s, b, depth + 1);
                return;
            }
        }
        match self.fit(a, b) {
            Ok(()) => {}
            Err(e) => {
                let s = self.knot(e, a, b);
                if depth < 64 && s > a && s < b {
                    self.split = true;
                    self.range(a, s, depth + 1);
                    self.range(s, b, depth + 1);
                } else {
                    let (p0, p3) = (self.pt(a), self.pt(b));
                    if p0 != p3 {
                        self.out.push(Seg::line(p0, p3));
                    }
                }
            }
        }
    }

    fn handle(p: Vec2, t: Vec2, a: f64) -> Vec2 {
        if t.length() <= EPS { p } else { p + t * a }
    }

    /// Fits one cubic to the samples `a..=b`; `Err(index of the largest error)` on failure.
    fn fit(&mut self, a: usize, b: usize) -> Result<(), usize> {
        let tol = self.tol;
        let k_line = (tol / 10.0).max(EPS);
        let pts: Vec<Vec2> = (a..=b).map(|i| self.pt(i)).collect();
        let m = pts.len();
        let (p0, p3) = (pts[0], pts[m - 1]);
        let (fa, fb) = (self.flag(a), self.flag(b));
        let mut t1 = self.t_start(a);
        let mut t2 = self.t_end(b);
        let reflect = |axis: Vec2, n: Vec2| unit(axis * (2.0 * axis.dot(n)) - n);
        if m == 2 {
            if fa & F_SMOOTH == 0 && fb & F_SMOOTH == 0 {
                if p0 != p3 {
                    self.out.push(Seg::line(p0, p3));
                }
                return Ok(());
            }
            if fa & F_CORNER != 0 && fb & F_SMOOTH != 0 {
                t1 = reflect(t1, -t2);
            } else if fb & F_CORNER != 0 && fa & F_SMOOTH != 0 {
                t2 = reflect(t2, -t1);
            }
            let d = 0.39 * p0.distance(p3);
            let s = Seg::new(p0, Self::handle(p0, t1, d), Self::handle(p3, t2, d), p3);
            self.emit(s, k_line);
            return Ok(());
        }
        if m < 5 && fa & F_CORNER != 0 && fb & F_CORNER != 0 {
            if pts.iter().all(|&p| chord_dist(p, p0, p3) <= tol / 2.0) {
                if p0 != p3 {
                    self.out.push(Seg::line(p0, p3));
                }
                return Ok(());
            }
            if m == 3 {
                let q = pts[1] * 2.0 - (p0 + p3) * 0.5;
                let s = Seg::new(
                    p0,
                    p0 + (q - p0) * (2.0 / 3.0),
                    p3 + (q - p3) * (2.0 / 3.0),
                    p3,
                );
                self.emit(s, k_line);
                return Ok(());
            }
        }
        if let Some(j) = (a + 1..=b).find(|&i| self.flag(i) & 7 != 0)
            && j != b
        {
            if fa & F_CORNER != 0 {
                t1 = reflect(t1, -self.t_end(j));
            }
            if fb & F_CORNER != 0
                && let Some(k) = (a..b).rev().find(|&i| self.flag(i) & 7 != 0)
            {
                t2 = reflect(t2, -self.t_start(k));
            }
        }
        // curve fit (h)
        let mut u = vec![0.0f64; m];
        for i in 1..m {
            u[i] = u[i - 1] + pts[i].distance(pts[i - 1]);
        }
        let len = u[m - 1];
        if len <= 0.0 {
            return Ok(());
        }
        for x in u.iter_mut() {
            *x /= len;
        }
        let amax = len.min(2.0 * self.diag);
        let mut lim = (len / 8.0).max(2.0 * tol);
        // tol = 0 (inset path, re 34 §1.4): only an exact fit passes; in f64 an error below 1e-9
        // counts as 0 (the original computes in f32 and reaches exactly 0)
        let small = if tol > 0.0 { tol / 10.0 } else { 1e-9 };
        let mut prev = lim;
        let mut worst = m >> 1;
        let mut seg = Seg::line(p0, p3);
        for it in 0..15 {
            if it >= 1 {
                for i in 1..m - 1 {
                    let t = u[i];
                    let q = seg.eval(t) - pts[i];
                    let d1 = seg.d1(t);
                    let f = d1.dot(q);
                    let fp = d1.dot(d1) + seg.d2(t).dot(q);
                    if fp.abs() <= f.abs() {
                        return Err(a + worst);
                    }
                    let nu = t - f / fp;
                    if !(-0.002..=1.002).contains(&nu) {
                        return Err(a + worst);
                    }
                    u[i] = nu.clamp(0.0, 1.0);
                }
            }
            // handle lengths by least squares
            let (mut c11, mut c12, mut c22, mut x1, mut x2) = (0.0, 0.0, 0.0, 0.0, 0.0);
            for i in 1..m - 1 {
                let t = u[i];
                let mt = 1.0 - t;
                let (b0, b1, b2, b3) =
                    (mt * mt * mt, 3.0 * t * mt * mt, 3.0 * t * t * mt, t * t * t);
                let a1 = t1 * b1;
                let a2 = t2 * b2;
                let tmp = pts[i] - (p0 * (b0 + b1) + p3 * (b2 + b3));
                c11 += a1.dot(a1);
                c12 += a1.dot(a2);
                c22 += a2.dot(a2);
                x1 += a1.dot(tmp);
                x2 += a2.dot(tmp);
            }
            let det = c11 * c22 - c12 * c12;
            let (mut al1, mut al2) = if det.abs() >= 1e-11 {
                ((x1 * c22 - x2 * c12) / det, (x2 * c11 - x1 * c12) / det)
            } else {
                (amax, amax)
            };
            for al in [&mut al1, &mut al2] {
                if al.abs() < EPS {
                    *al = 0.0;
                } else if *al > amax {
                    *al = amax;
                }
            }
            if al1 < 0.0 || al2 < 0.0 {
                al1 = p0.distance(p3) / 3.0;
                al2 = al1;
            }
            seg = Seg::new(p0, Self::handle(p0, t1, al1), Self::handle(p3, t2, al2), p3);
            let mut err = 0.0;
            for i in 1..m - 1 {
                let e = seg.eval(u[i]).distance(pts[i]);
                if e > err {
                    err = e;
                    worst = i;
                }
            }
            if err <= small {
                self.emit(seg, k_line);
                return Ok(());
            }
            if err >= SLOW * prev || it >= 14 {
                if err <= tol {
                    self.emit(seg, k_line);
                    return Ok(());
                }
                if err > lim {
                    return Err(a + worst);
                }
                lim = ((lim + 4.0 * tol) / 3.0).min(err);
            }
            prev = err;
        }
        Err(a + worst)
    }
}

/// Result of one refit pass.
struct Pass {
    segs: Vec<Seg>,
    closed: bool,
    split: bool,
}

/// One refit pass over the working segments (preprocessing changes them in place).
fn refit_pass(segs: &mut Vec<Seg>, tol: f64, angle: f64, n: u32, ex: Extrema) -> Pass {
    let diag = control_diag(segs);
    let tol = if tol <= 0.0 {
        0.0
    } else {
        tol.min(0.25 * diag)
    };
    let n = n.clamp(1, 32);
    let thr = 2.0 * (angle.clamp(0.0, 180.0).to_radians() / 2.0).sin();
    // (b) retract short handles
    for s in segs.iter_mut() {
        let d = adist(s.p0, s.c1);
        if d > 0.0 && d < tol * PULL {
            s.c1 = s.p0;
        }
        let d = adist(s.c2, s.p3);
        if d > 0.0 && d < tol * PULL {
            s.c2 = s.p3;
        }
    }
    if ex.split_x || ex.split_y {
        split_extrema(segs, ex, tol);
    }
    let seg_len = |s: &Seg| {
        0.5 * (adist(s.p0, s.c1) + adist(s.c1, s.c2) + adist(s.c2, s.p3) + adist(s.p3, s.p0))
    };
    let curves: Vec<&Seg> = segs.iter().filter(|s| !s.is_line()).collect();
    let has_curves = !curves.is_empty();
    let total: f64 = curves.iter().map(|s| seg_len(s)).sum();
    let density = if has_curves && total > 0.0 {
        n as f64 / (total / curves.len() as f64)
    } else {
        0.0
    };
    // (c) sampling
    let mut s: Vec<Smp> = vec![Smp {
        p: segs[0].p0,
        flag: F_CORNER,
    }];
    let (mut prev_pt, mut tin) = (segs[0].p0, 2u8);
    let (mut first_next, mut first_tout) = (segs[0].p3, 2u8);
    for (k, seg) in segs.iter().enumerate() {
        let line = seg.is_line();
        let tout = if line { 1 } else { 2 };
        let next = if line {
            seg.p3
        } else if near(seg.p0, seg.c1) {
            seg.c2
        } else {
            seg.c1
        };
        if k > 0 {
            let i = s.len() - 1;
            s[i].flag = classify(seg.p0, prev_pt, next, tin, tout, thr, tol);
        } else {
            first_next = next;
            first_tout = tout;
        }
        if line {
            s.push(Smp {
                p: seg.eval(1.0 / 3.0),
                flag: 0,
            });
            s.push(Smp {
                p: seg.eval(2.0 / 3.0),
                flag: 0,
            });
            prev_pt = seg.p0;
        } else {
            // tol = 0 (inset path): a product of exactly n rounds below n in the original's f32
            // arithmetic in the observed cases (E1: 15 instead of 16 samples, re 34 §1.4; A)
            let bias = if tol > 0.0 { 0.0 } else { 1e-6 };
            let kk = ((seg_len(seg) * density - bias) as i64)
                .clamp(n.min(3) as i64, 2 * n as i64 - 1)
                .max(1) as usize;
            let mut t = 0.0;
            for _ in 1..kk {
                t += 1.0 / kk as f64;
                s.push(Smp {
                    p: seg.eval(t),
                    flag: F_SMOOTH,
                });
            }
            prev_pt = if near(seg.c2, seg.p3) { seg.c1 } else { seg.c2 };
        }
        s.push(Smp { p: seg.p3, flag: 0 });
        tin = tout;
    }
    let closed = s.len() > 2 && s[s.len() - 1].p == s[0].p;
    if closed {
        s.pop();
        s[0].flag = classify(s[0].p, prev_pt, first_next, tin, first_tout, thr, tol);
    } else {
        s[0].flag = F_CORNER | F_NODE;
        let l = s.len() - 1;
        s[l].flag = F_CORNER | F_NODE;
    }
    let ns = s.len();
    for i in 0..ns {
        if s[i].flag & 7 == F_LOPSIDED {
            let nb = |j: Option<usize>| j.map(|j| s[j].flag & F_SMOOTH != 0).unwrap_or(false);
            let prev = if i > 0 {
                Some(i - 1)
            } else if closed {
                Some(ns - 1)
            } else {
                None
            };
            let next = if i + 1 < ns {
                Some(i + 1)
            } else if closed {
                Some(0)
            } else {
                None
            };
            if nb(prev) && nb(next) {
                s[i].flag = (s[i].flag & !F_LOPSIDED) | F_SMOOTH;
            }
        }
    }
    if ex.mark_x {
        mark_extrema(&mut s, closed, true, tol);
    }
    if ex.mark_y {
        mark_extrema(&mut s, closed, false, tol);
    }
    let mut fit = Fit {
        s: &s,
        closed,
        tol,
        diag,
        mask: if has_curves { F_NODE } else { 0x85 },
        out: Vec::new(),
        split: false,
    };
    let breaks: Vec<usize> = (0..ns)
        .filter(|&i| s[i].flag & (F_FIXED | F_CORNER) != 0)
        .collect();
    if closed {
        if breaks.is_empty() {
            fit.range(0, ns, 0);
        } else {
            for k in 0..breaks.len() {
                let a = breaks[k];
                let b = if k + 1 < breaks.len() {
                    breaks[k + 1]
                } else {
                    breaks[0] + ns
                };
                fit.range(a, b, 0);
            }
        }
    } else {
        for w in breaks.windows(2) {
            fit.range(w[0], w[1], 0);
        }
    }
    let split = fit.split;
    Pass {
        segs: fit.out,
        closed,
        split,
    }
}

/// Fewer than three points (a closed contour counts its start point twice).
fn too_small(c: &Contour) -> bool {
    c.nodes.len() + usize::from(c.closed) <= 2
}

fn node_count(p: &Pass) -> usize {
    if p.closed {
        p.segs.len()
    } else {
        p.segs.len() + 1
    }
}

/// Refit of one contour (`tol` in pixels, corner angle in degrees, samples per curve 1..32).
/// Contours with two points or fewer stay as they are; the original counts the repeated start
/// point of a closed contour (K-4, re 34 stages 3/4: the closed two-node middle face of P1 is
/// split at its extremum and decomposed).
pub fn refit_contour(c: &Contour, tol: f64, angle: f64, n: u32) -> Contour {
    if too_small(c) {
        return c.clone();
    }
    let mut segs = contour_segs(c);
    if segs.is_empty() {
        return c.clone();
    }
    let p = refit_pass(&mut segs, tol, angle, n, Extrema::default());
    if p.segs.is_empty() {
        return c.clone();
    }
    segs_contour(&p.segs, p.closed)
}

/// Extrema simplification with x and y and several stages (the call before expand stroke and
/// inset path, §2.4).
pub fn simplify_extrema(c: &Contour) -> Contour {
    let n_nodes = c.nodes.len();
    if too_small(c) {
        return c.clone();
    }
    let mut segs = contour_segs(c);
    if segs.is_empty() {
        return c.clone();
    }
    let diag = control_diag(&segs);
    let ex = Extrema {
        split_x: true,
        split_y: true,
        mark_x: true,
        mark_y: true,
    };
    let mut last: Option<Pass> = None;
    for &(fac, angle, n) in SIMPLIFY_STAGES.iter() {
        let tol = (fac * diag).max(0.05);
        let p = refit_pass(&mut segs, tol, angle, n, ex);
        let removed = n_nodes as i64 - node_count(&p) as i64;
        let more = p.split && tol < 0.25 * diag && removed < ((n_nodes - 1) / 5) as i64;
        last = Some(p);
        if !more {
            break;
        }
    }
    match last {
        Some(p) if !p.segs.is_empty() => segs_contour(&p.segs, p.closed),
        _ => c.clone(),
    }
}

/// Preliminary stage of expand stroke and inset path (§2.1): extrema simplification, then refit
/// with tolerance `tol` (stroke width), 30°, 16 samples.
pub fn prepare_for_offset(contours: &[Contour], tol: f64) -> Vec<Contour> {
    contours
        .iter()
        .map(|c| refit_contour(&simplify_extrema(c), tol, 30.0, 16))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: f64, y: f64) -> Vec2 {
        Vec2::new(x, y)
    }

    /// P1 of re 30 §2.2 as an open contour.
    pub fn p1() -> Contour {
        Contour::new(
            vec![
                Node::new(v(200.0, 200.0), v(200.0, 200.0), v(240.0, 160.0)),
                Node::new(v(290.0, 200.0), v(260.0, 160.0), v(320.0, 240.0)),
                Node::new(v(370.0, 200.0), v(340.0, 240.0), v(380.0, 180.0)),
                Node::new(v(420.0, 240.0), v(400.0, 200.0), v(420.0, 240.0)),
            ],
            false,
        )
    }

    fn close(a: Vec2, b: Vec2, eps: f64) -> bool {
        (a.x - b.x).abs() < eps && (a.y - b.y).abs() < eps
    }

    #[test]
    fn p1_simplify_splits_at_y_extrema() {
        let s = simplify_extrema(&p1());
        let a = s.anchors();
        let want = [
            v(200.0, 200.0),
            v(248.75, 170.0),
            v(330.0, 230.0),
            v(380.0, 192.1539),
            v(420.0, 240.0),
        ];
        assert_eq!(a.len(), want.len(), "{a:?}");
        for (g, w) in a.iter().zip(want) {
            assert!(close(*g, w, 0.001), "{a:?}");
        }
    }

    #[test]
    fn p1_refit_width_10() {
        let r = prepare_for_offset(&[p1()], 10.0);
        let c = &r[0];
        assert_eq!(c.nodes.len(), 3, "{c:?}");
        assert!(close(c.nodes[0].anchor, v(200.0, 200.0), 0.001));
        assert!(
            close(c.nodes[0].ctrl_out, v(300.0987, 99.1803), 0.01),
            "{c:?}"
        );
        assert!(
            close(c.nodes[1].ctrl_in, v(297.3627, 317.4612), 0.01),
            "{c:?}"
        );
        assert!(close(c.nodes[1].anchor, v(380.0, 192.1539), 0.001), "{c:?}");
        assert!(
            close(c.nodes[1].ctrl_out, v(396.6549, 192.5791), 0.01),
            "{c:?}"
        );
        assert!(
            close(c.nodes[2].ctrl_in, v(413.4716, 226.8658), 0.01),
            "{c:?}"
        );
        assert!(close(c.nodes[2].anchor, v(420.0, 240.0), 0.001));
    }

    #[test]
    fn refit_keeps_rectangle_and_reduces_circle_polygon() {
        let pts: Vec<Vec2> = (0..72)
            .map(|i| {
                let a = i as f64 * std::f64::consts::TAU / 72.0;
                v(50.0 + 30.0 * a.cos(), 50.0 + 30.0 * a.sin())
            })
            .collect();
        let r = refit_contour(&Contour::polygon(&pts, true), 2.0, 30.0, 16);
        assert!(r.closed && r.nodes.len() <= 8, "{}", r.nodes.len());
        let rect = Contour::polygon(
            &[v(0.0, 0.0), v(10.0, 0.0), v(10.0, 5.0), v(0.0, 5.0)],
            true,
        );
        let r = refit_contour(&rect, 10.0, 30.0, 16);
        assert_eq!(r.anchors(), rect.anchors());
    }

    #[test]
    fn normalisation_turns_flat_curves_into_lines() {
        let mut out = Vec::new();
        push_curve(
            &mut out,
            Seg::new(v(0.0, 0.0), v(3.0, 0.001), v(6.0, -0.001), v(9.0, 0.0)),
        );
        assert!(out.len() == 1 && out[0].is_line());
        // loop: end inside the handle triangle → split
        let mut out = Vec::new();
        push_curve(
            &mut out,
            Seg::new(v(0.0, 0.0), v(100.0, 0.0), v(100.0, 100.0), v(50.0, 20.0)),
        );
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn adist_is_max_plus_half_min() {
        assert_eq!(adist(v(0.0, 0.0), v(3.0, -4.0)), 5.5);
    }
}
