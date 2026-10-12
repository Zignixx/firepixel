//! Spiegelsymmetrie erkennen und angleichen (Schalter `symmetry`).
//!
//! Erkannt wird eine senkrechte und/oder waagerechte Achse durch die Mitte des Motivs (alles außer
//! der Hintergrundfarbe am Bildrand; Achse ±½ px probiert). Stimmen mindestens 97 % der
//! Motivpixel mit ihrem Spiegelbild überein, wird die rechte (untere) Hälfte aus der linken
//! (oberen) gespiegelt. Nach der Anpassung übernimmt jede Kette der rechten Hälfte die gespiegelte
//! Kurve ihres linken Gegenstücks – beide Seiten sind dann exakt gleich.

use super::fit::{fit_chain, reverse, FitParams, FitStats, Seg, V};
use super::outline::{Chain, EdgeKey};
use super::NONE;
use std::collections::HashMap;

/// Achse in Pixelecken-Koordinaten ×2 (`a2`): Spiegelbild der Ecke x ist `a2 - x`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Axes {
    pub vertical: Option<i32>,
    pub horizontal: Option<i32>,
}

const MIN_AGREE: f64 = 0.97;

pub(crate) fn detect_and_apply(label: &mut [u32], w: usize, h: usize) -> Axes {
    // Hintergrund = häufigste Farbe am Rand
    let mut border: HashMap<u32, usize> = HashMap::new();
    for x in 0..w {
        *border.entry(label[x]).or_default() += 1;
        *border.entry(label[(h - 1) * w + x]).or_default() += 1;
    }
    for y in 0..h {
        *border.entry(label[y * w]).or_default() += 1;
        *border.entry(label[y * w + w - 1]).or_default() += 1;
    }
    let bg = border.into_iter().max_by_key(|&(l, n)| (n, u32::MAX - l)).map(|v| v.0).unwrap_or(NONE);
    let (mut x0, mut x1, mut y0, mut y1) = (usize::MAX, 0, usize::MAX, 0);
    for y in 0..h {
        for x in 0..w {
            if label[y * w + x] != bg {
                x0 = x0.min(x);
                x1 = x1.max(x);
                y0 = y0.min(y);
                y1 = y1.max(y);
            }
        }
    }
    if x0 == usize::MAX || x1 - x0 < 8 || y1 - y0 < 8 {
        return Axes::default();
    }
    let mut axes = Axes::default();
    // senkrecht: Pixel x ↔ a2 - 1 - x
    let base = (x0 + x1 + 1) as i32;
    let mut best: Option<(f64, i32)> = None;
    for a2 in [base - 1, base, base + 1] {
        let (mut eq, mut tot) = (0usize, 0usize);
        for y in y0..=y1 {
            for x in x0..=x1 {
                let xm = a2 - 1 - x as i32;
                if xm < 0 || xm >= w as i32 {
                    continue;
                }
                let (l, lm) = (label[y * w + x], label[y * w + xm as usize]);
                if l == bg && lm == bg {
                    continue;
                }
                tot += 1;
                eq += usize::from(l == lm);
            }
        }
        if tot > 0 {
            let r = eq as f64 / tot as f64;
            if best.is_none_or(|b| r > b.0) {
                best = Some((r, a2));
            }
        }
    }
    if let Some((r, a2)) = best
        && r >= MIN_AGREE
    {
        for y in 0..h {
            for x in 0..w {
                let xm = a2 - 1 - x as i32;
                if xm > x as i32 && xm < w as i32 {
                    label[y * w + xm as usize] = label[y * w + x];
                }
            }
        }
        axes.vertical = Some(a2);
    }
    // waagerecht: Pixel y ↔ a2 - 1 - y
    let base = (y0 + y1 + 1) as i32;
    let mut best: Option<(f64, i32)> = None;
    for a2 in [base - 1, base, base + 1] {
        let (mut eq, mut tot) = (0usize, 0usize);
        for y in y0..=y1 {
            let ym = a2 - 1 - y as i32;
            if ym < 0 || ym >= h as i32 {
                continue;
            }
            for x in x0..=x1 {
                let (l, lm) = (label[y * w + x], label[ym as usize * w + x]);
                if l == bg && lm == bg {
                    continue;
                }
                tot += 1;
                eq += usize::from(l == lm);
            }
        }
        if tot > 0 {
            let r = eq as f64 / tot as f64;
            if best.is_none_or(|b| r > b.0) {
                best = Some((r, a2));
            }
        }
    }
    if let Some((r, a2)) = best
        && r >= MIN_AGREE
    {
        for y in 0..h {
            let ym = a2 - 1 - y as i32;
            if ym > y as i32 && ym < h as i32 {
                let (src, dst) = (y * w, ym as usize * w);
                for x in 0..w {
                    label[dst + x] = label[src + x];
                }
            }
        }
        axes.horizontal = Some(a2);
    }
    axes
}

/// Kette, die eine Achse kreuzt (ihr eigenes Spiegelbild): nur die linke/obere Hälfte anpassen
/// (bis zum Achsenpunkt, dort glatt senkrecht zur Achse, wenn die Kurve es zulässt) und spiegeln.
/// `None`, wenn die Kette keine Achse kreuzt oder nicht genau symmetrisch geteilt werden kann.
pub(crate) fn fit_mirrored(ch: &Chain, axes: &Axes, fp: &FitParams, st: &mut FitStats) -> Option<Vec<Seg>> {
    for (vertical, a2) in [(true, axes.vertical), (false, axes.horizontal)] {
        let Some(a2) = a2 else { continue };
        let side = |p: (i32, i32)| if vertical { 2 * p.0 - a2 } else { 2 * p.1 - a2 };
        if !(ch.pts.iter().any(|&p| side(p) < 0) && ch.pts.iter().any(|&p| side(p) > 0)) {
            continue;
        }
        let pv = |q: (i32, i32)| V::new(q.0 as f64, q.1 as f64);
        let coord = |v: V| if vertical { v.x } else { v.y };
        let on = |v: V| (2.0 * coord(v) - a2 as f64).abs() < 1e-9;
        let mirv = |p: V| if vertical { V::new(a2 as f64 - p.x, p.y) } else { V::new(p.x, a2 as f64 - p.y) };
        let n = ch.pts.len();
        let mut s: Vec<V> = Vec::with_capacity(n + 4);
        if ch.closed {
            for i in 0..n {
                let p = pv(ch.pts[i]);
                if on(p) {
                    s.push(p);
                }
                s.push(p.mid(pv(ch.pts[(i + 1) % n])));
            }
        } else {
            s.push(pv(ch.pts[0]));
            for i in 0..n - 1 {
                let p = pv(ch.pts[i]);
                if i > 0 && on(p) {
                    s.push(p);
                }
                s.push(p.mid(pv(ch.pts[i + 1])));
            }
            s.push(pv(ch.pts[n - 1]));
        }
        let axis: Vec<usize> = (0..s.len()).filter(|&i| on(s[i])).collect();
        let hint = if vertical { V::new(1.0, 0.0) } else { V::new(0.0, 1.0) };
        let left = |v: V| 2.0 * coord(v) < a2 as f64;
        let mirrored = |segs: &[Seg]| reverse(&segs.iter().map(|x| x.map(mirv)).collect::<Vec<_>>());
        if ch.closed {
            if axis.len() != 2 {
                return None;
            }
            let (i0, i1) = (axis[0], axis[1]);
            let arc1: Vec<V> = s[i0..=i1].to_vec();
            let mut arc2: Vec<V> = s[i1..].to_vec();
            arc2.extend_from_slice(&s[..=i0]);
            return Some(if left(arc1[arc1.len() / 2]) {
                let segs = fit_chain(&arc1, false, fp, st, [Some(hint), Some(hint)]);
                [segs.clone(), mirrored(&segs)].concat()
            } else {
                let segs = fit_chain(&arc2, false, fp, st, [Some(hint), Some(hint)]);
                [mirrored(&segs), segs].concat()
            });
        }
        if axis.len() != 1 || pv(ch.pts[0]) != mirv(pv(ch.pts[n - 1])) {
            return None;
        }
        let k = axis[0];
        return Some(if left(s[0]) {
            let segs = fit_chain(&s[..=k], false, fp, st, [None, Some(hint)]);
            [segs.clone(), mirrored(&segs)].concat()
        } else {
            let segs = fit_chain(&s[k..], false, fp, st, [Some(hint), None]);
            [mirrored(&segs), segs].concat()
        });
    }
    None
}

/// Rechte/untere Ketten übernehmen die gespiegelte Anpassung ihres Gegenstücks.
pub(crate) fn equalize(chains: &[Chain], fitted: &mut [Vec<Seg>], axes: &Axes) {
    for (vertical, a2) in [(true, axes.vertical), (false, axes.horizontal)] {
        let Some(a2) = a2 else { continue };
        let mir = |p: (i32, i32)| if vertical { (a2 - p.0, p.1) } else { (p.0, a2 - p.1) };
        let mirv = |p: V| if vertical { V::new(a2 as f64 - p.x, p.y) } else { V::new(p.x, a2 as f64 - p.y) };
        let side = |p: (i32, i32)| if vertical { 2 * p.0 - a2 } else { 2 * p.1 - a2 };
        // Schlüssel: gerichtete erste Kante (offen) bzw. Begrenzungsrechteck + Länge (Ring)
        let mut open: HashMap<EdgeKey, (usize, bool)> = HashMap::new();
        let mut rings: HashMap<(i32, i32, i32, i32, usize), usize> = HashMap::new();
        let bbox = |pts: &[(i32, i32)]| pts.iter().fold((i32::MAX, i32::MAX, i32::MIN, i32::MIN), |b, p| (b.0.min(p.0), b.1.min(p.1), b.2.max(p.0), b.3.max(p.1)));
        for (id, c) in chains.iter().enumerate() {
            let m = c.pts.len();
            if c.closed {
                let b = bbox(&c.pts);
                rings.insert((b.0, b.1, b.2, b.3, m), id);
            } else if m >= 2 {
                open.insert((c.pts[0], c.pts[1]), (id, false));
                open.insert((c.pts[m - 1], c.pts[m - 2]), (id, true));
            }
        }
        for (id, c) in chains.iter().enumerate() {
            // nur Ketten ganz auf der linken/oberen Seite (Achse erlaubt)
            if c.pts.iter().any(|&p| side(p) > 0) || c.pts.iter().all(|&p| side(p) == 0) {
                continue;
            }
            let target = if c.closed {
                let b = bbox(&c.pts.iter().map(|&p| mir(p)).collect::<Vec<_>>());
                rings.get(&(b.0, b.1, b.2, b.3, c.pts.len())).map(|&t| (t, false))
            } else {
                open.get(&(mir(c.pts[0]), mir(c.pts[1]))).copied()
            };
            let Some((t, rev)) = target else { continue };
            if t == id {
                continue;
            }
            let mirrored: Vec<Seg> = fitted[id].iter().map(|s| s.map(mirv)).collect();
            fitted[t] = if rev { reverse(&mirrored) } else { mirrored };
        }
    }
}
