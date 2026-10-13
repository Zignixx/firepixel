//! Grundformen erkennen: geschlossene Randketten, die (innerhalb der Formtoleranz) Kreis, Ellipse,
//! Rechteck oder abgerundetes Rechteck sind, werden durch die exakte Form ersetzt.
//!
//! - **Ellipse/Kreis:** Fläche, Schwerpunkt und zweite Momente des Pixelumrisses (exakt über die
//!   Eckpunkte) ergeben Mittelpunkt, Halbachsen und Drehung; fast gleiche Achsen → Kreis mit dem
//!   Radius aus der Fläche, fast achsparallel → exakt achsparallel. Ausgabe: 4 Knoten
//!   (Henkel κ = 0,5523 × Halbachse).
//! - **Rechteck/abgerundetes Rechteck:** kleinstes umschließendes Rechteck der Kantenmitten
//!   (Drehschieber über die konvexe Hülle), Eckradius durch Minimieren der mittleren Abweichung.
//!   Ausgabe: 4 Ecken bzw. 8 Knoten (je Ecke ein Viertelbogen).
//!
//! Angenommen wird eine Form nur, wenn **jede** Kantenmitte höchstens `tol` (+0,4 % der Größe)
//! von ihr entfernt liegt und die Fläche auf 3 % stimmt.

use super::fit::{Seg, V};

/// Erkannte Grundform.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Shape {
    Circle,
    Ellipse,
    Rect,
    RoundedRect,
}

/// κ für Viertelkreise aus einer kubischen Kurve.
const KAPPA: f64 = 0.552_284_749_830_793_4;

/// Auf 0,5 px runden, wenn höchstens 0,12 px entfernt (saubere Zahlen bei Pixelkanten).
fn tidy(v: f64) -> f64 {
    let r = (v * 2.0).round() / 2.0;
    if (v - r).abs() <= 0.12 { r } else { v }
}

/// Fläche (mit Vorzeichen), Schwerpunkt und zentrale zweite Momente eines Polygons.
fn moments(c: &[(i32, i32)]) -> (f64, V, f64, f64, f64) {
    let n = c.len();
    let (mut a, mut cx, mut cy, mut ixx, mut iyy, mut ixy) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    for i in 0..n {
        let (x0, y0) = (c[i].0 as f64, c[i].1 as f64);
        let (x1, y1) = (c[(i + 1) % n].0 as f64, c[(i + 1) % n].1 as f64);
        let cr = x0 * y1 - x1 * y0;
        a += cr;
        cx += (x0 + x1) * cr;
        cy += (y0 + y1) * cr;
        ixx += (x0 * x0 + x0 * x1 + x1 * x1) * cr;
        iyy += (y0 * y0 + y0 * y1 + y1 * y1) * cr;
        ixy += (x0 * y1 + 2.0 * x0 * y0 + 2.0 * x1 * y1 + x1 * y0) * cr;
    }
    a *= 0.5;
    if a.abs() < 1e-9 {
        return (0.0, V::default(), 0.0, 0.0, 0.0);
    }
    let (cx, cy) = (cx / (6.0 * a), cy / (6.0 * a));
    let cxx = ixx / (12.0 * a) - cx * cx;
    let cyy = iyy / (12.0 * a) - cy * cy;
    let cxy = ixy / (24.0 * a) - cx * cy;
    (a, V::new(cx, cy), cxx, cyy, cxy)
}

fn signed_area(segs: &[Seg]) -> f64 {
    segs.iter().map(|s| s.p0.cross(s.p3)).sum::<f64>()
}

fn orient(mut segs: Vec<Seg>, positive: bool) -> Vec<Seg> {
    if (signed_area(&segs) > 0.0) != positive {
        segs = super::fit::reverse(&segs);
    }
    segs
}

/// Erkennt eine Grundform. `corners`: Pixelumriss (Eckpunkte), `samples`: Kantenmitten.
pub(crate) fn detect(corners: &[(i32, i32)], samples: &[V], tol: f64, snap_deg: f64, allow_curves: bool) -> Option<(Shape, Vec<Seg>)> {
    let (area, c, cxx, cyy, cxy) = moments(corners);
    let positive = area > 0.0;
    let area = area.abs();
    if area < 40.0 || samples.len() < 16 {
        return None;
    }
    if allow_curves && let Some(r) = ellipse(area, c, cxx, cyy, cxy, samples, tol, snap_deg) {
        return Some((r.0, orient(r.1, positive)));
    }
    rect(area, samples, tol, snap_deg, allow_curves).map(|r| (r.0, orient(r.1, positive)))
}

#[allow(clippy::too_many_arguments)]
fn ellipse(area: f64, c: V, cxx: f64, cyy: f64, cxy: f64, samples: &[V], tol: f64, snap_deg: f64) -> Option<(Shape, Vec<Seg>)> {
    let tr = 0.5 * (cxx + cyy);
    let dis = (0.25 * (cxx - cyy).powi(2) + cxy * cxy).sqrt();
    let (l1, l2) = (tr + dis, tr - dis);
    if l2 <= 0.0 {
        return None;
    }
    let mut ang = 0.5 * (2.0 * cxy).atan2(cxx - cyy);
    let (mut a, mut b) = (2.0 * l1.sqrt(), 2.0 * l2.sqrt());
    let tol_eff = tol + 0.004 * a;
    let circle = (a - b).abs() <= (0.5f64).max(0.012 * a);
    let mut center = c;
    if circle {
        let r = tidy((area / std::f64::consts::PI).sqrt());
        a = r;
        b = r;
        ang = 0.0;
        center = V::new(tidy(c.x), tidy(c.y));
    } else {
        // nahe an den Achsen → exakt achsparallel
        let q = (ang / std::f64::consts::FRAC_PI_2).round();
        if snap_deg > 0.0 && (ang - q * std::f64::consts::FRAC_PI_2).abs().to_degrees() <= snap_deg {
            let (sa, sb) = (2.0 * cxx.sqrt(), 2.0 * cyy.sqrt());
            ang = 0.0;
            a = sa;
            b = sb;
            center = V::new(tidy(c.x), tidy(c.y));
        }
        // Fläche angleichen
        let k = (area / (std::f64::consts::PI * a * b)).sqrt();
        a = tidy(a * k);
        b = tidy(b * k);
    }
    let (s, co) = ang.sin_cos();
    let (u, v) = (V::new(co, s), V::new(-s, co));
    let mut worst = 0.0f64;
    for &p in samples {
        let q = p - center;
        let (x, y) = (q.dot(u), q.dot(v));
        let f = (x / a).powi(2) + (y / b).powi(2) - 1.0;
        let g = V::new(2.0 * x / (a * a), 2.0 * y / (b * b)).len();
        let d = if g > 1e-12 { (f / g).abs() } else { a };
        worst = worst.max(d);
        if worst > tol_eff {
            return None;
        }
    }
    if ((std::f64::consts::PI * a * b) - area).abs() > 0.03 * area {
        return None;
    }
    let pt = |t: f64| center + u * (a * t.cos()) + v * (b * t.sin());
    let dv = |t: f64| u * (-a * t.sin()) + v * (b * t.cos());
    let segs = (0..4)
        .map(|k| {
            let (t0, t1) = (k as f64 * std::f64::consts::FRAC_PI_2, (k + 1) as f64 * std::f64::consts::FRAC_PI_2);
            Seg { p0: pt(t0), c1: pt(t0) + dv(t0) * KAPPA, c2: pt(t1) - dv(t1) * KAPPA, p3: pt(t1) }
        })
        .map(|s| s.map(clean))
        .collect();
    Some((if circle || (a - b).abs() < 1e-9 { Shape::Circle } else { Shape::Ellipse }, segs))
}

/// Rundungsreste von sin/cos entfernen (1e-9).
fn clean(p: V) -> V {
    let r = |v: f64| {
        let k = (v * 1e6).round() / 1e6;
        if (v - k).abs() < 1e-9 { k } else { v }
    };
    V::new(r(p.x), r(p.y))
}

fn hull(pts: &[V]) -> Vec<V> {
    let mut p: Vec<V> = pts.to_vec();
    p.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    p.dedup();
    if p.len() < 3 {
        return p;
    }
    let mut lower: Vec<V> = Vec::new();
    for &q in &p {
        while lower.len() >= 2 && (lower[lower.len() - 1] - lower[lower.len() - 2]).cross(q - lower[lower.len() - 2]) <= 0.0 {
            lower.pop();
        }
        lower.push(q);
    }
    let mut upper: Vec<V> = Vec::new();
    for &q in p.iter().rev() {
        while upper.len() >= 2 && (upper[upper.len() - 1] - upper[upper.len() - 2]).cross(q - upper[upper.len() - 2]) <= 0.0 {
            upper.pop();
        }
        upper.push(q);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

/// Vorzeichenbehafteter Abstand zu einem abgerundeten Rechteck (lokale Koordinaten, Mitte 0).
fn sdf(x: f64, y: f64, hw: f64, hh: f64, r: f64) -> f64 {
    let qx = x.abs() - (hw - r);
    let qy = y.abs() - (hh - r);
    V::new(qx.max(0.0), qy.max(0.0)).len() + qx.max(qy).min(0.0) - r
}

fn rect(area: f64, samples: &[V], tol: f64, snap_deg: f64, allow_curves: bool) -> Option<(Shape, Vec<Seg>)> {
    let h = hull(samples);
    if h.len() < 3 {
        return None;
    }
    // kleinstes umschließendes Rechteck (Richtungen der Hüllkanten)
    let mut best: Option<(f64, f64)> = None; // (Fläche, Winkel)
    for i in 0..h.len() {
        let e = h[(i + 1) % h.len()] - h[i];
        if e.len() < 1e-9 {
            continue;
        }
        let ang = e.y.atan2(e.x);
        let (u, v) = (e.norm(), V::new(-e.norm().y, e.norm().x));
        let (mut a0, mut a1, mut b0, mut b1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
        for &q in &h {
            let (x, y) = (q.dot(u), q.dot(v));
            a0 = a0.min(x);
            a1 = a1.max(x);
            b0 = b0.min(y);
            b1 = b1.max(y);
        }
        let ar = (a1 - a0) * (b1 - b0);
        if best.is_none_or(|b| ar < b.0) {
            best = Some((ar, ang));
        }
    }
    let (_, mut ang) = best?;
    // Winkel nach [-45°, 45°)
    let q = (ang / std::f64::consts::FRAC_PI_2).round();
    ang -= q * std::f64::consts::FRAC_PI_2;
    if snap_deg > 0.0 && ang.abs().to_degrees() <= snap_deg {
        ang = 0.0;
    }
    let (s, co) = ang.sin_cos();
    let (u, v) = (V::new(co, s), V::new(-s, co));
    let (mut a0, mut a1, mut b0, mut b1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
    for &q in samples {
        let (x, y) = (q.dot(u), q.dot(v));
        a0 = a0.min(x);
        a1 = a1.max(x);
        b0 = b0.min(y);
        b1 = b1.max(y);
    }
    if ang == 0.0 {
        (a0, a1, b0, b1) = (tidy(a0), tidy(a1), tidy(b0), tidy(b1));
    }
    let (hw, hh) = (0.5 * (a1 - a0), 0.5 * (b1 - b0));
    if hw < 2.0 || hh < 2.0 {
        return None;
    }
    let center = u * (0.5 * (a0 + a1)) + v * (0.5 * (b0 + b1));
    let local: Vec<(f64, f64)> = samples.iter().map(|&p| ((p - center).dot(u), (p - center).dot(v))).collect();
    let step = (local.len() / 400).max(1);
    let mean_err = |r: f64| {
        let (mut s, mut n) = (0.0, 0.0);
        for &(x, y) in local.iter().step_by(step) {
            s += sdf(x, y, hw, hh, r).abs();
            n += 1.0;
        }
        s / n
    };
    let rmax = hw.min(hh);
    let mut r = 0.0;
    if allow_curves {
        let (mut lo, mut hi) = (0.0, rmax);
        for _ in 0..40 {
            let (m1, m2) = (lo + (hi - lo) / 3.0, hi - (hi - lo) / 3.0);
            if mean_err(m1) <= mean_err(m2) {
                hi = m2;
            } else {
                lo = m1;
            }
        }
        let rc = 0.5 * (lo + hi);
        r = if mean_err(0.0) <= mean_err(rc) + 0.05 || rc < 1.0 { 0.0 } else { tidy(rc).min(rmax) };
        if rmax - r < 0.3 {
            r = rmax;
        }
    }
    let tol_eff = tol + 0.004 * hw.max(hh);
    if local.iter().any(|&(x, y)| sdf(x, y, hw, hh, r).abs() > tol_eff) {
        return None;
    }
    let shape_area = 4.0 * hw * hh - (4.0 - std::f64::consts::PI) * r * r;
    if (shape_area - area).abs() > 0.03 * area {
        return None;
    }
    let g = |x: f64, y: f64| clean(center + u * x + v * y);
    let k = KAPPA * r;
    let mut segs: Vec<Seg> = Vec::new();
    let line = |a: V, b: V, segs: &mut Vec<Seg>| {
        if a.dist(b) > 1e-9 {
            segs.push(Seg::line(a, b));
        }
    };
    if r == 0.0 {
        let c = [g(-hw, -hh), g(hw, -hh), g(hw, hh), g(-hw, hh)];
        for i in 0..4 {
            line(c[i], c[(i + 1) % 4], &mut segs);
        }
        return Some((Shape::Rect, segs));
    }
    // oben → rechts → unten → links, je Ecke ein Viertelbogen
    line(g(-hw + r, -hh), g(hw - r, -hh), &mut segs);
    segs.push(Seg { p0: g(hw - r, -hh), c1: g(hw - r + k, -hh), c2: g(hw, -hh + r - k), p3: g(hw, -hh + r) });
    line(g(hw, -hh + r), g(hw, hh - r), &mut segs);
    segs.push(Seg { p0: g(hw, hh - r), c1: g(hw, hh - r + k), c2: g(hw - r + k, hh), p3: g(hw - r, hh) });
    line(g(hw - r, hh), g(-hw + r, hh), &mut segs);
    segs.push(Seg { p0: g(-hw + r, hh), c1: g(-hw + r - k, hh), c2: g(-hw, hh - r + k), p3: g(-hw, hh - r) });
    line(g(-hw, hh - r), g(-hw, -hh + r), &mut segs);
    segs.push(Seg { p0: g(-hw, -hh + r), c1: g(-hw, -hh + r - k), c2: g(-hw + r - k, -hh), p3: g(-hw + r, -hh) });
    Some((Shape::RoundedRect, segs))
}
