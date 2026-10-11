//! Vereinfachen / Refit (Spezifikation 16 §3.2; Referenz `re\pfade\simplify.py`).
//!
//! Das Original ruft `ApuRefitPath(pfad, Menge, 30,0, 16, 0, 0)` (Menge 1..25, Vorgabe 10, Toleranz
//! in Pixeln unskaliert). Verfahren nach Schneider („An Algorithm for Automatically Fitting
//! Digitized Curves“): Abtasten (Geraden bei ⅓, ⅔; Kurven mit N Stichproben, N ∈ 1..32), Ecken mit
//! Knick > Winkel als feste Bruchstellen, Kubik per kleinsten Quadraten (Rückfall Sehne/3), Fehler =
//! größter Abstand, Newton-Reparametrisierung (≤ 15), Teilen am Punkt größten Fehlers; Toleranz
//! mindestens 0,05. Wie die Referenz nicht byte-genau zu python.dll (Abtastdichte, Abbruch).
//! Dieselbe Anpassung nutzt das Vektorpfad-Werkzeug (`ApuRefitPath(p, 10 − Präferenz, …)`).

use super::geom::{Contour, Node};
use fp_geom::{CubicBez, Vec2};

/// Vorgabe der Präferenz `SimplifyAmount`.
pub const DEFAULT_AMOUNT: f64 = 10.0;

fn unit(v: Vec2) -> Vec2 {
    let l = v.x.hypot(v.y);
    if l > 1e-12 { Vec2::new(v.x / l, v.y / l) } else { Vec2::ZERO }
}

/// Punkte entlang der Kontur und Indizes der Originalknoten.
pub fn sample_contour(c: &Contour, n: u32) -> (Vec<Vec2>, Vec<usize>) {
    let n = n.clamp(1, 32);
    let mut pts = vec![c.nodes[0].anchor];
    let mut idx = vec![0usize];
    for s in c.segments() {
        if s.is_line() {
            pts.push(s.eval(1.0 / 3.0));
            pts.push(s.eval(2.0 / 3.0));
            pts.push(s.p3);
        } else {
            for i in 1..=n {
                pts.push(s.eval(i as f64 / n as f64));
            }
        }
        idx.push(pts.len() - 1);
    }
    (pts, idx)
}

fn is_corner(c: &Contour, segs: &[CubicBez], k: usize, angle_deg: f64) -> bool {
    let n = c.nodes.len();
    if !c.closed && (k == 0 || k == n - 1) {
        return true;
    }
    let m = segs.len();
    if m == 0 {
        return true;
    }
    let sin_ = &segs[(k + m - 1) % m];
    let sout = &segs[k % m];
    let a = sin_.tangent(1.0);
    let b = sout.tangent(0.0);
    (a.x * b.x + a.y * b.y).clamp(-1.0, 1.0).acos().to_degrees() > angle_deg
}

fn chord_params(p: &[Vec2]) -> Vec<f64> {
    let mut u = vec![0.0];
    for i in 1..p.len() {
        let last = u[u.len() - 1];
        u.push(last + p[i].distance(p[i - 1]));
    }
    let mut l = u[u.len() - 1];
    if l == 0.0 {
        l = 1.0;
    }
    u.iter().map(|x| x / l).collect()
}

fn generate(p: &[Vec2], u: &[f64], t1: Vec2, t2: Vec2) -> CubicBez {
    let (p0, p3) = (p[0], p[p.len() - 1]);
    let mut c = [[0.0f64; 2]; 2];
    let mut x = [0.0f64; 2];
    for (pt, &t) in p.iter().zip(u) {
        let mt = 1.0 - t;
        let (b0, b1, b2, b3) = (mt * mt * mt, 3.0 * t * mt * mt, 3.0 * t * t * mt, t * t * t);
        let a1 = t1 * b1;
        let a2 = t2 * b2;
        c[0][0] += a1.dot(a1);
        c[0][1] += a1.dot(a2);
        c[1][1] += a2.dot(a2);
        let tmp = *pt - (p0 * (b0 + b1) + p3 * (b2 + b3));
        x[0] += a1.dot(tmp);
        x[1] += a2.dot(tmp);
    }
    c[1][0] = c[0][1];
    let det = c[0][0] * c[1][1] - c[0][1] * c[1][0];
    let seg_len = p0.distance(p3);
    let eps = 1e-6 * seg_len;
    let (mut a1, mut a2) = if det.abs() > 1e-11 {
        ((x[0] * c[1][1] - x[1] * c[0][1]) / det, (c[0][0] * x[1] - c[1][0] * x[0]) / det)
    } else {
        (-1.0, -1.0)
    };
    if a1 < eps || a2 < eps {
        a1 = seg_len / 3.0;
        a2 = a1;
    }
    CubicBez::new(p0, p0 + t1 * a1, p3 + t2 * a2, p3)
}

fn max_error(bez: &CubicBez, p: &[Vec2], u: &[f64]) -> (f64, usize) {
    let (mut best, mut idx) = (0.0, p.len() / 2);
    for i in 1..p.len().saturating_sub(1) {
        let d = bez.eval(u[i]).distance(p[i]);
        if d > best {
            best = d;
            idx = i;
        }
    }
    (best, idx)
}

fn reparam(bez: &CubicBez, p: &[Vec2], u: &[f64]) -> Vec<f64> {
    p.iter()
        .zip(u)
        .map(|(pt, &t)| {
            let q = bez.eval(t);
            let d1 = bez.deriv(t);
            let d2 = bez.deriv2(t);
            let qp = q - *pt;
            let num = qp.dot(d1);
            let den = d1.dot(d1) + qp.dot(d2);
            let nt = if den.abs() > 1e-12 && num.abs() <= den.abs() { t - num / den } else { t };
            nt.clamp(0.0, 1.0)
        })
        .collect()
}

/// Kubiken durch die Punkte `p` mit Endtangenten `t1` (vorwärts) und `t2` (rückwärts).
pub fn fit_cubic(p: &[Vec2], t1: Vec2, t2: Vec2, tol: f64, depth: u32) -> Vec<CubicBez> {
    if p.len() == 2 {
        let d = p[0].distance(p[1]) / 3.0;
        return vec![CubicBez::new(p[0], p[0] + t1 * d, p[1] + t2 * d, p[1])];
    }
    let mut u = chord_params(p);
    let mut bez = generate(p, &u, t1, t2);
    let (mut err, mut split) = max_error(&bez, p, &u);
    if err <= tol {
        return vec![bez];
    }
    if err <= 4.0 * tol {
        for _ in 0..15 {
            u = reparam(&bez, p, &u);
            bez = generate(p, &u, t1, t2);
            (err, split) = max_error(&bez, p, &u);
            if err <= tol {
                return vec![bez];
            }
        }
    }
    if depth > 30 || split == 0 || split + 1 >= p.len() {
        return vec![bez];
    }
    let tc = unit(p[split - 1] - p[split + 1]);
    let mut left = fit_cubic(&p[..split + 1], t1, tc, tol, depth + 1);
    let right = fit_cubic(&p[split..], Vec2::new(-tc.x, -tc.y), t2, tol, depth + 1);
    left.extend(right);
    left
}

/// Kontur neu anpassen; Ecken (Knick > `corner_angle`) bleiben feste Knoten.
pub fn refit_contour(c: &Contour, tol: f64, corner_angle: f64, n: u32) -> Contour {
    if c.nodes.len() <= 2 {
        return c.clone();
    }
    let tol = tol.max(0.05);
    let (pts, node_idx) = sample_contour(c, n);
    let segs = c.segments();
    let mut corners: Vec<usize> = (0..c.nodes.len()).filter(|&k| is_corner(c, &segs, k, corner_angle)).collect();
    if c.closed && corners.is_empty() {
        corners.push(0);
    }
    let (breaks, ext): (Vec<usize>, Vec<Vec2>) = if c.closed {
        let mut b: Vec<usize> = corners.iter().map(|&k| node_idx[k]).collect();
        b.push(node_idx[corners[0]] + pts.len() - 1);
        let body = &pts[..pts.len() - 1];
        let mut e = body.to_vec();
        e.extend_from_slice(body);
        e.push(pts[0]);
        (b, e)
    } else {
        (corners.iter().map(|&k| node_idx[k]).collect(), pts.clone())
    };
    let mut out_segs = Vec::new();
    for w in breaks.windows(2) {
        let (a, b) = (w[0], w[1]);
        if a > b || b >= ext.len() {
            continue;
        }
        let p = &ext[a..=b];
        let mut q = vec![p[0]];
        for i in 1..p.len() {
            if p[i].distance(p[i - 1]) > 1e-9 {
                q.push(p[i]);
            }
        }
        if q.len() < 2 {
            continue;
        }
        let t1 = unit(q[1] - q[0]);
        let t2 = unit(q[q.len() - 2] - q[q.len() - 1]);
        out_segs.extend(fit_cubic(&q, t1, t2, tol, 0));
    }
    if out_segs.is_empty() {
        return c.clone();
    }
    let m = out_segs.len();
    let mut nodes = Vec::with_capacity(m + 1);
    for k in 0..m {
        let prev = if c.closed || k > 0 { Some(&out_segs[(k + m - 1) % m]) } else { None };
        nodes.push(Node::new(out_segs[k].p0, prev.map(|p| p.p2).unwrap_or(out_segs[k].p0), out_segs[k].p1));
    }
    if !c.closed {
        let last = out_segs[m - 1];
        nodes.push(Node::new(last.p3, last.p2, last.p3));
    }
    Contour::new(nodes, c.closed)
}

/// „Ändern → Pfad ändern → Vereinfachen“: `amount` = Dialogwert 1..25, Toleranz in Pixeln.
pub fn simplify(contours: &[Contour], amount: f64) -> Vec<Contour> {
    contours.iter().map(|c| refit_contour(c, amount, 30.0, 16)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn circle_polygon_becomes_few_nodes() {
        let pts: Vec<Vec2> = (0..72).map(|i| {
            let a = i as f64 * std::f64::consts::TAU / 72.0;
            Vec2::new(50.0 + 30.0 * a.cos(), 50.0 + 30.0 * a.sin())
        }).collect();
        let c = Contour::polygon(&pts, true);
        let r = simplify(&[c], 2.0);
        assert!(r[0].nodes.len() <= 4, "{}", r[0].nodes.len());
        let rect = Contour::polygon(&[Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0), Vec2::new(10.0, 5.0), Vec2::new(0.0, 5.0)], true);
        let r = simplify(std::slice::from_ref(&rect), 10.0);
        assert_eq!(r[0].anchors(), rect.anchors());
    }
}
