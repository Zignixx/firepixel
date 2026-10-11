//! Konturverlauf (`FSHi = 3`, `FSXi = 17`; Spezifikation 07 §5.7, B `@007e6b40`, `@007e63d0`).
//!
//! - `R = 0,5·min(Breite, Höhe)` der Pfad-Bounding-Box.
//! - Stopp-Positionen mit `d = |p2 − p1|` umgerechnet: `d < R` → `pos·d/R`, `d > R` →
//!   `pos·R/d + (1 − R/d)`.
//! - Je Pixel innerhalb der Maske: `f = clamp(Abstand(Pixelecke, Pfad)/R, 0, 1)`, Farbe =
//!   Tabelle(1 − f). Der Abstand (`ApuDistanceToPath`) wird hier zur Polylinie gemessen.
//! - Liegt der Mittelpunkt der Bounding-Box nicht auf p1: Vorwärts-Splatting nach
//!   `(x, y) − f·(Mitte − p1)` auf die 4 Nachbarn (hier bilinear gewichtet, A), Lücken über den
//!   3×3-Mittelwert (`@007e72e0`, `@007e5970`).
//! - Laufzeittest T6 (`t6_contour`): 8.665/9.600 Pixel exakt, 9.344 ±8 (die Python-Referenz hat den
//!   Konturverlauf nicht: 3.600). Offen: genaue Gewichtung des Splattings und Abstandsmaß.

use crate::attrs::{Fill, Gradient, GradientStop, VectorPath};
use crate::gradient::GradientTable;
use crate::paint::{fill_pixel, PaintIssue};
use crate::render::{fill_mask, path_bounds};
use fp_geom::{IRect, Vec2};

/// Abtastpunkt relativ zur linken oberen Pixelecke (A: Pixelmitte wie die übrigen Abstandsmaße).
const SAMPLE_OFFSET: f64 = 0.0;

fn remap(stops: &[GradientStop], d: f64, r: f64) -> Vec<GradientStop> {
    stops
        .iter()
        .map(|s| {
            let pos = if d < r && r > 0.0 {
                s.pos * d / r
            } else if d > r && d > 0.0 {
                s.pos * r / d + (1.0 - r / d)
            } else {
                s.pos
            };
            GradientStop { pos, color: s.color }
        })
        .collect()
}

/// Kleinster Abstand des Punkts zu den Kanten aller Konturen (geschlossene inkl. Schlusskante).
pub fn distance_to_polylines(path: &VectorPath, p: Vec2) -> f64 {
    let mut best = f64::INFINITY;
    for c in &path.contours {
        let pts = &c.points;
        let n = pts.len();
        if n == 1 {
            best = best.min(pts[0].distance(p));
        }
        let segs = if c.closed { n } else { n.saturating_sub(1) };
        for i in 0..segs {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            let ab = b - a;
            let l2 = ab.length_sq();
            let t = if l2 > 0.0 { ((p - a).dot(ab) / l2).clamp(0.0, 1.0) } else { 0.0 };
            best = best.min((a + ab * t).distance(p));
        }
    }
    best
}

/// Füllebene eines Konturverlaufs.
pub fn contour_fill_layer(path: &VectorPath, fill: &Fill, g: &Gradient, rect: IRect) -> Result<Vec<[u8; 4]>, PaintIssue> {
    if g.color_stops.is_empty() {
        return Err(PaintIssue::NoStops);
    }
    let w = rect.width() as usize;
    let m = fill_mask(path, fill.edge, rect);
    let mut out = vec![[0u8; 4]; m.data.len()];
    let Some(bb) = path_bounds(path) else { return Ok(out) };
    let r = 0.5 * (bb.width().min(bb.height()) as f64);
    let h = &fill.handles;
    let d = h.p1.distance(h.p2);
    let table = GradientTable::new(&remap(&g.color_stops, d, r), &remap(&g.opacity_stops, d, r));
    if w == 0 {
        return Ok(out);
    }
    let h = m.data.len() / w;
    let center = Vec2::new((bb.left as f64 + bb.right as f64) / 2.0, (bb.top as f64 + bb.bottom as f64) / 2.0);
    let off = center - fill.handles.p1;
    let shifted = off.x.abs() > 1e-9 || off.y.abs() > 1e-9;
    // Farbe und Abstandsfaktor je Pixel innerhalb der Maske
    let mut colors = vec![0u32; m.data.len()];
    let mut factors = vec![0.0f64; m.data.len()];
    for (k, &c) in m.data.iter().enumerate() {
        if c == 0 {
            continue;
        }
        let x = rect.left as f64 + (k % w) as f64 + SAMPLE_OFFSET;
        let y = rect.top as f64 + (k / w) as f64 + SAMPLE_OFFSET;
        let dist = distance_to_polylines(path, Vec2::new(x, y));
        let f = if r > 0.0 { (dist / r).clamp(0.0, 1.0) } else { 1.0 };
        colors[k] = table.lookup(1.0 - f).0;
        factors[k] = f;
    }
    if shifted {
        // Vorwärts-Splatting nach (x, y) − f·(Mitte − p1) auf die 4 Nachbarn, Lücken über 3×3-Mittel (A)
        let mut acc = vec![[0.0f64; 4]; m.data.len()];
        let mut wsum = vec![0.0f64; m.data.len()];
        for k in 0..m.data.len() {
            if m.data[k] == 0 {
                continue;
            }
            let (px, py) = ((k % w) as f64, (k / w) as f64);
            let tx = px - factors[k] * off.x;
            let ty = py - factors[k] * off.y;
            let (ix, iy) = (tx.floor(), ty.floor());
            let (fx, fy) = (tx - ix, ty - iy);
            let c = colors[k];
            let ch = [(c >> 24) & 255, (c >> 16) & 255, (c >> 8) & 255, c & 255].map(|v| v as f64);
            for (dx, dy, wt) in [(0.0, 0.0, (1.0 - fx) * (1.0 - fy)), (1.0, 0.0, fx * (1.0 - fy)), (0.0, 1.0, (1.0 - fx) * fy), (1.0, 1.0, fx * fy)] {
                let (qx, qy) = (ix + dx, iy + dy);
                if wt <= 0.0 || qx < 0.0 || qy < 0.0 || qx >= w as f64 || qy >= h as f64 {
                    continue;
                }
                let q = qy as usize * w + qx as usize;
                for (a, v) in acc[q].iter_mut().zip(ch) {
                    *a += v * wt;
                }
                wsum[q] += wt;
            }
        }
        let pack = |v: [f64; 4]| -> u32 { v.iter().fold(0u32, |a, x| (a << 8) | (x.round().clamp(0.0, 255.0) as u32)) };
        let mut splat = vec![None; m.data.len()];
        for k in 0..m.data.len() {
            if wsum[k] > 0.0 {
                splat[k] = Some(acc[k].map(|v| v / wsum[k]));
            }
        }
        for k in 0..m.data.len() {
            if m.data[k] == 0 {
                continue;
            }
            colors[k] = match splat[k] {
                Some(v) => pack(v),
                None => {
                    let (px, py) = ((k % w) as i64, (k / w) as i64);
                    let (mut s, mut n) = ([0.0f64; 4], 0.0);
                    for dy in -1..=1 {
                        for dx in -1..=1 {
                            let (qx, qy) = (px + dx, py + dy);
                            if qx < 0 || qy < 0 || qx >= w as i64 || qy >= h as i64 {
                                continue;
                            }
                            if let Some(v) = splat[qy as usize * w + qx as usize] {
                                for (a, b) in s.iter_mut().zip(v) {
                                    *a += b;
                                }
                                n += 1.0;
                            }
                        }
                    }
                    if n > 0.0 { pack(s.map(|v| v / n)) } else { colors[k] }
                }
            };
        }
    }
    for (k, (o, &c)) in out.iter_mut().zip(&m.data).enumerate() {
        if c != 0 {
            *o = fill_pixel(colors[k], c);
        }
    }
    Ok(out)
}
