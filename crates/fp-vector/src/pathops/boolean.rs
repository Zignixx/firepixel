//! Boolesche Operationen auf Bézier-Ebene (Spezifikation 16 §2.3, §2.7; Referenz
//! `re\pfade\boolops.py`, „Arrangement + Klassifikation“).
//!
//! 1. Segmente beider Operanden sammeln (offene Konturen für die Fläche implizit geschlossen).
//! 2. Paarweise Schnittpunkte: Gerade × Gerade analytisch inkl. kollinearer Überlappung, sonst
//!    rekursive Halbierung mit Bounds-Test (Budget gegen deckungsgleiche Kurven); Teilung an allen
//!    Schnittparametern.
//! 3. **Original-Platzierung** neuer Schnittpunkte: Originalknoten innerhalb 0,015 px werden
//!    bevorzugt, sonst Raster 0,005 px; angrenzende Henkel wandern um dasselbe Delta mit (B, python.dll).
//! 4. Knoten innerhalb 10⁻⁶ verschmelzen; deckungsgleiche Teilstücke nur einmal.
//! 5. Klassifikation am **ungerasterten** Teilstück: Testpunkte ±10⁻⁴ senkrecht zur Kurvenmitte,
//!    Windungszahl über exakte Kurven-Strahl-Schnitte, Füllregel je Operand.
//! 6. Kanten mit Innerem links behalten und verketten (stärkste Linksdrehung an Mehrfachknoten).
//!
//! Das Ergebnis besteht aus gleichsinnig orientierten, überlappungsfreien Konturen (Inneres links
//! im mathematischen Sinn); [`super::compound::original_order`] richtet sie wie das Original aus.

use std::collections::{HashMap, HashSet};

use super::geom::{line_line, quad_roots, Contour, Node};
use fp_geom::{CubicBez, Vec2};

/// Knoten-Verschmelzung (Dokumentpixel).
pub const SNAP: f64 = 1e-6;
/// Schnittparameter so nah an 0/1 gelten als Endpunkt.
pub const T_EPS: f64 = 1e-9;
/// Abstand der Klassifikations-Testpunkte.
pub const OFFSET: f64 = 1e-4;
/// Raster neuer Schnittpunkte (python.dll `0,005`).
pub const GRID: f64 = 0.005;
/// Einrasten auf Originalknoten (3ε = 0,015).
pub const NODE_SNAP: f64 = 0.015;

/// Boolesche Verknüpfung.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BoolOp {
    /// A ∪ B.
    Union,
    /// A ∩ B.
    Intersect,
    /// A − B (B stanzt aus A aus).
    Subtract,
    /// A XOR B (nur Referenz, nicht im Original).
    Xor,
}

impl BoolOp {
    fn apply(self, a: bool, b: bool) -> bool {
        match self {
            BoolOp::Union => a || b,
            BoolOp::Intersect => a && b,
            BoolOp::Subtract => a && !b,
            BoolOp::Xor => a != b,
        }
    }
}

/// Innen-Regel einer Region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FillRule {
    /// Gerade/ungerade (`EOFb = 1`).
    EvenOdd,
    /// Windung ≠ 0.
    NonZero,
    /// Windung > 0 (`ApuCleanAfterExpand`).
    Positive,
}

/// Fläche: Konturen und Füllregel (offene Konturen werden implizit geschlossen).
#[derive(Debug, Clone, PartialEq)]
pub struct Region {
    pub contours: Vec<Contour>,
    pub rule: FillRule,
    segs: Vec<CubicBez>,
    boxes: Vec<[f64; 4]>,
}

fn hull(s: &CubicBez) -> [f64; 4] {
    let b = s.hull_bounds();
    [b.min.x, b.min.y, b.max.x, b.max.y]
}

impl Region {
    /// Region aus Konturen (Konturen mit < 2 Knoten entfallen).
    pub fn new(contours: Vec<Contour>, rule: FillRule) -> Self {
        let contours: Vec<Contour> = contours.into_iter().filter(|c| c.nodes.len() >= 2).collect();
        let mut segs = Vec::new();
        for c in &contours {
            segs.extend(c.segments());
            if !c.closed && c.nodes.len() >= 2 {
                let (a, b) = (c.nodes[c.nodes.len() - 1].anchor, c.nodes[0].anchor);
                segs.push(CubicBez::new(a, a, b, b));
            }
        }
        let boxes = segs.iter().map(hull).collect();
        Self { contours, rule, segs, boxes }
    }

    /// Region mit gerade/ungerade bzw. Windungsregel (`EOFb`).
    pub fn with_eof(contours: Vec<Contour>, even_odd: bool) -> Self {
        Self::new(contours, if even_odd { FillRule::EvenOdd } else { FillRule::NonZero })
    }

    /// Leere Region.
    pub fn empty() -> Self {
        Self::new(Vec::new(), FillRule::EvenOdd)
    }

    /// Windungszahl über exakte Kurven-Strahl-Schnitte.
    pub fn winding(&self, p: Vec2) -> i32 {
        let mut w = 0;
        for (s, b) in self.segs.iter().zip(&self.boxes) {
            if p.y < b[1] || p.y > b[3] || b[2] < p.x {
                continue;
            }
            w += ray_crossings(s, p.x, p.y);
        }
        w
    }

    /// Liegt der Punkt innen?
    pub fn contains(&self, p: Vec2) -> bool {
        let w = self.winding(p);
        match self.rule {
            FillRule::EvenOdd => w & 1 == 1,
            FillRule::NonZero => w != 0,
            FillRule::Positive => w > 0,
        }
    }
}

/// Vorzeichenbehaftete Zahl der Schnitte des Strahls (x,y) → (+∞,y) mit dem Segment
/// (halboffene Regel `[ymin, ymax)`).
fn ray_crossings(seg: &CubicBez, x: f64, y: f64) -> i32 {
    if seg.is_line() {
        let (a, b) = (seg.p0, seg.p3);
        if (a.y <= y && y < b.y) || (b.y <= y && y < a.y) {
            let xi = a.x + (y - a.y) * (b.x - a.x) / (b.y - a.y);
            if xi > x {
                return if b.y > a.y { 1 } else { -1 };
            }
        }
        return 0;
    }
    let (p0, p1, p2, p3) = (seg.p0, seg.p1, seg.p2, seg.p3);
    let a = -p0.y + 3.0 * p1.y - 3.0 * p2.y + p3.y;
    let b = 2.0 * (p0.y - 2.0 * p1.y + p2.y);
    let c = p1.y - p0.y;
    let mut ts = vec![0.0];
    ts.extend(quad_roots(a, b, c));
    ts.push(1.0);
    ts.sort_by(f64::total_cmp);
    ts.dedup();
    let mut w = 0;
    for k in 0..ts.len().saturating_sub(1) {
        let (t0, t1) = (ts[k], ts[k + 1]);
        let ya = seg.eval(t0).y;
        let yb = seg.eval(t1).y;
        if !((ya <= y && y < yb) || (yb <= y && y < ya)) {
            continue;
        }
        let (mut lo, mut hi) = (t0, t1);
        let up = yb > ya;
        for _ in 0..60 {
            let m = (lo + hi) / 2.0;
            let ym = seg.eval(m).y;
            if (ym < y) == up {
                lo = m;
            } else {
                hi = m;
            }
        }
        let xi = seg.eval((lo + hi) / 2.0).x;
        if xi > x {
            w += if up { 1 } else { -1 };
        }
    }
    w
}

fn collinear_overlap(s1: &CubicBez, s2: &CubicBez) -> (Vec<f64>, Vec<f64>) {
    let (a, b, c, d) = (s1.p0, s1.p3, s2.p0, s2.p3);
    let (rx, ry) = (b.x - a.x, b.y - a.y);
    let (sx, sy) = (d.x - c.x, d.y - c.y);
    let l1 = rx * rx + ry * ry;
    let l2 = sx * sx + sy * sy;
    if l1 < 1e-18 || l2 < 1e-18 {
        return (Vec::new(), Vec::new());
    }
    for p in [c, d] {
        if ((p.x - a.x) * ry - (p.y - a.y) * rx).abs() / l1.sqrt() > SNAP {
            return (Vec::new(), Vec::new());
        }
    }
    let u: Vec<f64> = [c, d].iter().map(|p| ((p.x - a.x) * rx + (p.y - a.y) * ry) / l1).filter(|t| T_EPS < *t && *t < 1.0 - T_EPS).collect();
    let v: Vec<f64> = [a, b].iter().map(|p| ((p.x - c.x) * sx + (p.y - c.y) * sy) / l2).filter(|t| T_EPS < *t && *t < 1.0 - T_EPS).collect();
    (u, v)
}

struct Budget(i64);

#[allow(clippy::too_many_arguments)]
fn curve_x(s1: &CubicBez, s2: &CubicBez, t1: (f64, f64), t2: (f64, f64), out: &mut Vec<(f64, f64)>, depth: u32, budget: &mut Budget) -> Result<(), ()> {
    budget.0 -= 1;
    if budget.0 < 0 {
        return Err(());
    }
    let (b1, b2) = (hull(s1), hull(s2));
    let tol = 1e-9;
    if b1[0] > b2[2] + tol || b2[0] > b1[2] + tol || b1[1] > b2[3] + tol || b2[1] > b1[3] + tol {
        return Ok(());
    }
    let (l1, l2) = (s1.is_line(), s2.is_line());
    if l1 && l2 {
        if let Some((u, v)) = line_line(s1.p0, s1.p3, s2.p0, s2.p3) {
            out.push((t1.0 + (t1.1 - t1.0) * u, t2.0 + (t2.1 - t2.0) * v));
        }
        return Ok(());
    }
    let size1 = (b1[2] - b1[0]).max(b1[3] - b1[1]);
    let size2 = (b2[2] - b2[0]).max(b2[3] - b2[1]);
    let small1 = !l1 && size1 < 1e-7;
    let small2 = !l2 && size2 < 1e-7;
    if ((small1 || l1) && (small2 || l2)) || depth > 60 {
        let param = |s: &CubicBez, t: (f64, f64), small: bool, other: Vec2| -> Option<f64> {
            if !small && s.is_line() {
                let (a, b) = (s.p0, s.p3);
                let (dx, dy) = (b.x - a.x, b.y - a.y);
                let mut l2 = dx * dx + dy * dy;
                if l2 == 0.0 {
                    l2 = 1.0;
                }
                let u = ((other.x - a.x) * dx + (other.y - a.y) * dy) / l2;
                if !(-1e-9..=1.0 + 1e-9).contains(&u) {
                    return None;
                }
                return Some(t.0 + (t.1 - t.0) * u.clamp(0.0, 1.0));
            }
            Some((t.0 + t.1) / 2.0)
        };
        let p1 = if !l1 { Some(s1.eval(0.5)) } else { None };
        let p2 = if !l2 { Some(s2.eval(0.5)) } else { None };
        let Some(rf) = p1.or(p2) else { return Ok(()) };
        if let (Some(q), None) = (p1, p2)
            && super::geom::dline(q, s2.p0, s2.p3) > 1e-6
        {
            return Ok(());
        }
        if let (None, Some(q)) = (p1, p2)
            && super::geom::dline(q, s1.p0, s1.p3) > 1e-6
        {
            return Ok(());
        }
        if let (Some(u), Some(v)) = (param(s1, t1, small1, rf), param(s2, t2, small2, rf)) {
            out.push((u, v));
        }
        return Ok(());
    }
    if !l1 && (l2 || size1 >= size2) {
        let (a, b) = s1.split(0.5);
        let m = (t1.0 + t1.1) / 2.0;
        curve_x(&a, s2, (t1.0, m), t2, out, depth + 1, budget)?;
        curve_x(&b, s2, (m, t1.1), t2, out, depth + 1, budget)?;
    } else {
        let (a, b) = s2.split(0.5);
        let m = (t2.0 + t2.1) / 2.0;
        curve_x(s1, &a, t1, (t2.0, m), out, depth + 1, budget)?;
        curve_x(s1, &b, t1, (m, t2.1), out, depth + 1, budget)?;
    }
    Ok(())
}

/// Schnittparameter `(u, v)` zweier Segmente; `None` = an diesem Segment kein Parameter
/// (Teilung nur am anderen, bei deckungsgleichen Stücken).
pub fn intersections(s1: &CubicBez, s2: &CubicBez) -> Vec<(Option<f64>, Option<f64>)> {
    if s1.is_line() && s2.is_line() {
        let (u, v) = collinear_overlap(s1, s2);
        if !u.is_empty() || !v.is_empty() {
            return u.into_iter().map(|t| (Some(t), None)).chain(v.into_iter().map(|t| (None, Some(t)))).collect();
        }
    }
    let mut out = Vec::new();
    let mut budget = Budget(20000);
    if curve_x(s1, s2, (0.0, 1.0), (0.0, 1.0), &mut out, 0, &mut budget).is_err() {
        let mut res = Vec::new();
        for p in [s2.p0, s2.p3] {
            let n = s1.nearest(p, 16);
            if n.distance < 1e-5 {
                res.push((Some(n.t), None));
            }
        }
        for p in [s1.p0, s1.p3] {
            let n = s2.nearest(p, 16);
            if n.distance < 1e-5 {
                res.push((None, Some(n.t)));
            }
        }
        return res;
    }
    out.into_iter().map(|(u, v)| (Some(u), Some(v))).collect()
}

fn cluster(ts: &mut [f64]) -> Vec<f64> {
    ts.sort_by(f64::total_cmp);
    let mut out: Vec<f64> = Vec::new();
    for &t in ts.iter() {
        if out.last().is_none_or(|l| t - l > 1e-7) {
            out.push(t);
        }
    }
    out
}

#[inline]
fn py_round(v: f64) -> i64 {
    let r = v.round_ties_even();
    if r.is_finite() { r.clamp(-9e15, 9e15) as i64 } else { 0 }
}

#[inline]
fn py_floor(v: f64) -> i64 {
    let r = v.floor();
    if r.is_finite() { r.clamp(-9e15, 9e15) as i64 } else { 0 }
}

/// Punkt auf das Raster setzen: `floor(p/grid + 0,5)·grid`.
pub fn snap_to_grid(p: Vec2, grid: f64) -> Vec2 {
    Vec2::new(py_floor(p.x / grid + 0.5) as f64 * grid, py_floor(p.y / grid + 0.5) as f64 * grid)
}

fn is_degenerate(s: &CubicBez) -> bool {
    s.is_degenerate()
}

/// Boolesche Operation `A op B` (Raster `grid`, 0 = ohne Raster). Liefert geschlossene Konturen
/// mit Innerem links (mathematische Orientierung).
pub fn boolean(a: &Region, b: &Region, op: BoolOp, grid: f64) -> Vec<Contour> {
    boolean_with(a, b, &|x, y| op.apply(x, y), grid)
}

/// Wie [`boolean`] mit beliebiger Verknüpfung `f(innen_A, innen_B)`.
pub fn boolean_with(a: &Region, b: &Region, f: &dyn Fn(bool, bool) -> bool, grid: f64) -> Vec<Contour> {
    let segs: Vec<CubicBez> = a.segs.iter().chain(&b.segs).filter(|s| !is_degenerate(s)).copied().collect();
    let n = segs.len();
    let mut cuts: Vec<Vec<f64>> = vec![Vec::new(); n];
    let boxes: Vec<[f64; 4]> = segs.iter().map(hull).collect();
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&i, &j| boxes[i][0].total_cmp(&boxes[j][0]));
    for ii in 0..n {
        let i = order[ii];
        let bi = boxes[i];
        for &j in &order[ii + 1..] {
            let bj = boxes[j];
            if bj[0] > bi[2] + SNAP {
                break;
            }
            if bj[1] > bi[3] + SNAP || bi[1] > bj[3] + SNAP {
                continue;
            }
            for (u, v) in intersections(&segs[i], &segs[j]) {
                if let Some(u) = u
                    && T_EPS < u
                    && u < 1.0 - T_EPS
                {
                    cuts[i].push(u);
                }
                if let Some(v) = v
                    && T_EPS < v
                    && v < 1.0 - T_EPS
                {
                    cuts[j].push(v);
                }
            }
        }
    }
    // Originalknoten (Endpunkte) für das Einrasten
    let mut origs: HashMap<(i64, i64), Vec<Vec2>> = HashMap::new();
    for s in &segs {
        for p in [s.p0, s.p3] {
            origs.entry((py_round(p.x / NODE_SNAP), py_round(p.y / NODE_SNAP))).or_default().push(p);
        }
    }
    let place = |p: Vec2| -> Vec2 {
        let (kx, ky) = (py_round(p.x / NODE_SNAP), py_round(p.y / NODE_SNAP));
        let mut best: Option<(f64, Vec2)> = None;
        for dx in [-1, 0, 1] {
            for dy in [-1, 0, 1] {
                if let Some(list) = origs.get(&(kx + dx, ky + dy)) {
                    for q in list {
                        if (q.x - p.x).abs() <= NODE_SNAP && (q.y - p.y).abs() <= NODE_SNAP {
                            let d = (q.x - p.x).abs() + (q.y - p.y).abs();
                            if best.is_none_or(|(bd, _)| d < bd) {
                                best = Some((d, *q));
                            }
                        }
                    }
                }
            }
        }
        match best {
            Some((_, q)) => q,
            None => snap_to_grid(p, grid),
        }
    };
    let mut pieces: Vec<(CubicBez, CubicBez)> = Vec::new();
    for (k, s) in segs.iter().enumerate() {
        let mut ts = vec![0.0];
        ts.extend(cluster(&mut cuts[k]));
        ts.push(1.0);
        for w in ts.windows(2) {
            let (t0, t1) = (w[0], w[1]);
            if t1 - t0 < 1e-12 {
                continue;
            }
            let q = s.subsegment(t0, t1);
            let q_orig = q;
            let mut q2 = q;
            if grid != 0.0 {
                let (mut p0, mut p1, mut p2, mut p3) = (q.p0, q.p1, q.p2, q.p3);
                if t0 > 0.0 {
                    let n0 = place(p0);
                    let d = n0 - p0;
                    p1 = if p1 == p0 { n0 } else { p1 + d };
                    p0 = n0;
                }
                if t1 < 1.0 {
                    let n3 = place(p3);
                    let d = n3 - p3;
                    p2 = if p2 == p3 { n3 } else { p2 + d };
                    p3 = n3;
                }
                q2 = CubicBez::new(p0, p1, p2, p3);
            }
            pieces.push((q2, q_orig));
        }
    }
    // Knoten verschmelzen
    let mut verts: Vec<Vec2> = Vec::new();
    let mut vgrid: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
    let mut vid = |p: Vec2, verts: &mut Vec<Vec2>| -> usize {
        let (kx, ky) = (py_floor(p.x / (SNAP * 4.0)), py_floor(p.y / (SNAP * 4.0)));
        for dx in [-1, 0, 1] {
            for dy in [-1, 0, 1] {
                if let Some(list) = vgrid.get(&(kx + dx, ky + dy)) {
                    for &v in list {
                        let q = verts[v];
                        if (q.x - p.x).abs() <= SNAP && (q.y - p.y).abs() <= SNAP {
                            return v;
                        }
                    }
                }
            }
        }
        verts.push(p);
        vgrid.entry((kx, ky)).or_default().push(verts.len() - 1);
        verts.len() - 1
    };
    let mut edges: Vec<(usize, usize, CubicBez, CubicBez)> = Vec::new();
    let mut seen: HashSet<(usize, usize, i64, i64)> = HashSet::new();
    for (s, s_orig) in &pieces {
        let a = vid(s.p0, &mut verts);
        let bb = vid(s.p3, &mut verts);
        let m = s_orig.eval(0.5);
        if a == bb && s.length() < 10.0 * SNAP {
            continue;
        }
        let key = (a.min(bb), a.max(bb), py_round(m.x / (SNAP * 10.0)), py_round(m.y / (SNAP * 10.0)));
        if !seen.insert(key) {
            continue;
        }
        let s2 = CubicBez::new(verts[a], if s.p1 != s.p0 { s.p1 } else { verts[a] }, if s.p2 != s.p3 { s.p2 } else { verts[bb] }, verts[bb]);
        edges.push((a, bb, s2, *s_orig));
    }
    // Klassifizieren am ungerasterten Teilstück
    let mut kept: Vec<(usize, usize, CubicBez)> = Vec::new();
    for (ea, eb, s, s_orig) in &edges {
        let m = s_orig.eval(0.5);
        let t = s_orig.tangent(0.5);
        let nrm = Vec2::new(-t.y, t.x);
        let l = Vec2::new(m.x + nrm.x * OFFSET, m.y + nrm.y * OFFSET);
        let r = Vec2::new(m.x - nrm.x * OFFSET, m.y - nrm.y * OFFSET);
        let in_l = f(a.contains(l), b.contains(l));
        let in_r = f(a.contains(r), b.contains(r));
        if in_l == in_r {
            continue;
        }
        if in_l {
            kept.push((*ea, *eb, *s));
        } else {
            kept.push((*eb, *ea, s.reversed()));
        }
    }
    chain(&kept)
}

fn chain(kept: &[(usize, usize, CubicBez)]) -> Vec<Contour> {
    let mut out_of: HashMap<usize, Vec<usize>> = HashMap::new();
    for (k, (a, _, _)) in kept.iter().enumerate() {
        out_of.entry(*a).or_default().push(k);
    }
    let mut used = vec![false; kept.len()];
    let mut contours = Vec::new();
    for start in 0..kept.len() {
        if used[start] {
            continue;
        }
        let mut ring: Vec<CubicBez> = Vec::new();
        let mut k = start;
        loop {
            used[k] = true;
            ring.push(kept[k].2);
            let (_, b, s) = kept[k];
            if b == kept[start].0 {
                break;
            }
            let cand: Vec<usize> = out_of.get(&b).map(|v| v.iter().copied().filter(|c| !used[*c]).collect()).unwrap_or_default();
            if cand.is_empty() {
                break;
            }
            if cand.len() == 1 {
                k = cand[0];
            } else {
                let din = s.tangent(1.0);
                let mut best: Option<(f64, usize)> = None;
                for c in cand {
                    let dout = kept[c].2.tangent(0.0);
                    let ang = (din.x * dout.y - din.y * dout.x).atan2(din.x * dout.x + din.y * dout.y);
                    if best.is_none_or(|(ba, _)| ang > ba) {
                        best = Some((ang, c));
                    }
                }
                if let Some((_, c)) = best {
                    k = c;
                }
            }
        }
        if ring.len() >= 2 || (ring.len() == 1 && !ring[0].is_line()) {
            contours.push(ring_to_contour(&ring));
        }
    }
    contours
}

fn ring_to_contour(ring: &[CubicBez]) -> Contour {
    let n = ring.len();
    let nodes = (0..n)
        .map(|k| {
            let prev = &ring[(k + n - 1) % n];
            let cur = &ring[k];
            Node::new(cur.p0, prev.p2, cur.p1)
        })
        .collect();
    Contour::new(nodes, true)
}

/// Aufeinanderfolgende kollineare Geradensegmente zusammenfassen (Referenz `merge_collinear`,
/// entspricht `ApuCoalescePathCurves`; das Original fasst bei booleschen Operationen **nicht** zusammen).
pub fn merge_collinear(contours: &[Contour], tol: f64) -> Vec<Contour> {
    let mut out = Vec::new();
    for c in contours {
        let mut nodes = c.nodes.clone();
        let mut changed = true;
        while changed && nodes.len() > 3 {
            changed = false;
            let n = nodes.len();
            for k in 0..n {
                let (prv, cur, nxt) = (nodes[(k + n - 1) % n], nodes[k], nodes[(k + 1) % n]);
                if cur.ctrl_in == cur.anchor
                    && cur.ctrl_out == cur.anchor
                    && prv.ctrl_out == prv.anchor
                    && nxt.ctrl_in == nxt.anchor
                    && super::geom::dline(cur.anchor, prv.anchor, nxt.anchor) < tol
                {
                    let d1 = cur.anchor - prv.anchor;
                    let d2 = nxt.anchor - cur.anchor;
                    if d1.x * d2.x + d1.y * d2.y >= 0.0 {
                        nodes.remove(k);
                        changed = true;
                        break;
                    }
                }
            }
        }
        out.push(Contour::new(nodes, c.closed));
    }
    out
}

/// Boolesche Operation auf Polygonen (Punktlisten); kollineare Punkte werden zusammengefasst.
pub fn polygon_boolean(a: &[Vec<Vec2>], b: &[Vec<Vec2>], op: BoolOp, eo_a: bool, eo_b: bool) -> Vec<Vec<Vec2>> {
    let to = |ps: &[Vec<Vec2>]| ps.iter().filter(|p| p.len() >= 3).map(|p| Contour::polygon(p, true)).collect::<Vec<_>>();
    let ra = Region::with_eof(to(a), eo_a);
    let rb = Region::with_eof(to(b), eo_b);
    merge_collinear(&boolean(&ra, &rb, op, GRID), 1e-6).iter().map(|c| c.anchors()).collect()
}

#[cfg(test)]
mod tests {
    use super::super::geom::area;
    use super::*;

    fn sq(x0: f64, y0: f64, x1: f64, y1: f64) -> Contour {
        Contour::polygon(&[Vec2::new(x0, y0), Vec2::new(x1, y0), Vec2::new(x1, y1), Vec2::new(x0, y1)], true)
    }

    #[test]
    fn squares() {
        let a = Region::with_eof(vec![sq(0.0, 0.0, 10.0, 10.0)], true);
        let b = Region::with_eof(vec![sq(5.0, 5.0, 15.0, 15.0)], true);
        let u = boolean(&a, &b, BoolOp::Union, GRID);
        assert_eq!(u.len(), 1);
        assert!((area(&u, 0.01) - 175.0).abs() < 1e-9);
        let i = boolean(&a, &b, BoolOp::Intersect, GRID);
        assert!((area(&i, 0.01) - 25.0).abs() < 1e-9);
        let s = boolean(&a, &b, BoolOp::Subtract, GRID);
        assert!((area(&s, 0.01) - 75.0).abs() < 1e-9);
        let x = boolean(&a, &b, BoolOp::Xor, GRID);
        assert!((area(&x, 0.01) - 150.0).abs() < 1e-9);
        // gemeinsame Kante → ein Rechteck mit 6 Knoten (kollineare Originalknoten bleiben)
        let c = Region::with_eof(vec![sq(10.0, 0.0, 20.0, 10.0)], true);
        let u2 = boolean(&a, &c, BoolOp::Union, GRID);
        assert_eq!(u2.len(), 1);
        assert_eq!(merge_collinear(&u2, 1e-6)[0].nodes.len(), 4);
        assert!((area(&u2, 0.01) - 200.0).abs() < 1e-9);
    }

    #[test]
    fn hole_and_rules() {
        let outer = sq(0.0, 0.0, 10.0, 10.0);
        let inner = sq(3.0, 3.0, 7.0, 7.0);
        let eo = Region::with_eof(vec![outer.clone(), inner.clone()], true);
        let r = boolean(&eo, &Region::empty(), BoolOp::Union, GRID);
        assert!((area(&r, 0.01) - 84.0).abs() < 1e-9);
        let nz = Region::with_eof(vec![outer, inner], false);
        let r = boolean(&nz, &Region::empty(), BoolOp::Union, GRID);
        assert!((area(&r, 0.01) - 100.0).abs() < 1e-9);
        let g = snap_to_grid(Vec2::new(-2.487488, 3.986328), GRID);
        assert!((g.x + 2.485).abs() < 1e-12 && (g.y - 3.985).abs() < 1e-12);
    }
}
