//! Einfache Rasterung des Ergebnisses (ohne Kantenglättung, Abtastung in der Pixelmitte,
//! gerade/ungerade je Pfad) für Vorschau und Prüfungen.

use crate::TraceResult;
use fp_geom::{BezierContour, Point};
use fp_raster::RgbaImage;

/// Kontur → Polylinie (jedes Bézier-Segment je nach Länge in 1–64 Stücke geteilt).
fn flatten(c: &BezierContour) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    let n = c.nodes.len();
    for i in 0..n {
        let a = &c.nodes[i];
        let b = &c.nodes[(i + 1) % n];
        let (p0, p1, p2, p3) = (a.pt, a.ctrl_out, b.ctrl_in, b.pt);
        let curved = p1 != p0 || p2 != p3;
        let d = |u: Point, v: Point| ((u.x - v.x) as f64).hypot((u.y - v.y) as f64);
        let steps = if curved {
            ((d(p0, p1) + d(p1, p2) + d(p2, p3)) / 1.5)
                .ceil()
                .clamp(1.0, 64.0) as usize
        } else {
            1
        };
        for k in 0..steps {
            let t = k as f64 / steps as f64;
            let mt = 1.0 - t;
            let f = |a: f32, b: f32, c: f32, d: f32| {
                mt * mt * mt * a as f64
                    + 3.0 * mt * mt * t * b as f64
                    + 3.0 * mt * t * t * c as f64
                    + t * t * t * d as f64
            };
            out.push((f(p0.x, p1.x, p2.x, p3.x), f(p0.y, p1.y, p2.y, p3.y)));
        }
    }
    out
}

pub(crate) fn render(r: &TraceResult) -> RgbaImage {
    let b = r.bounds;
    let mut img = match RgbaImage::new(b) {
        Ok(i) => i,
        Err(_) => return RgbaImage::empty_at(b.left, b.top),
    };
    let w = img.width();
    let h = img.height();
    let mut xs: Vec<f64> = Vec::new();
    for path in &r.paths {
        let polys: Vec<Vec<(f64, f64)>> = path
            .contours
            .iter()
            .map(flatten)
            .filter(|p| p.len() >= 2)
            .collect();
        if polys.is_empty() {
            continue;
        }
        let (mut ymin, mut ymax) = (f64::MAX, f64::MIN);
        for p in &polys {
            for &(_, y) in p {
                ymin = ymin.min(y);
                ymax = ymax.max(y);
            }
        }
        let row0 = ((ymin - b.top as f64 - 0.5).floor() as i64).clamp(0, h as i64) as usize;
        let row1 = ((ymax - b.top as f64 + 0.5).ceil() as i64).clamp(0, h as i64) as usize;
        let color = [path.color.r, path.color.g, path.color.b, path.color.a];
        for row in row0..row1 {
            let y = b.top as f64 + row as f64 + 0.5;
            xs.clear();
            for p in &polys {
                let m = p.len();
                for i in 0..m {
                    let (x0, y0) = p[i];
                    let (x1, y1) = p[(i + 1) % m];
                    if (y0 <= y) != (y1 <= y) {
                        xs.push(x0 + (y - y0) * (x1 - x0) / (y1 - y0));
                    }
                }
            }
            xs.sort_by(|a, b| a.total_cmp(b));
            let data = img.data_mut();
            for pair in xs.as_chunks::<2>().0 {
                // Pixel, deren Mitte x + 0,5 in [x0, x1) liegt.
                let c0 =
                    ((pair[0] - b.left as f64 - 0.5).ceil() as i64).clamp(0, w as i64) as usize;
                let c1 =
                    ((pair[1] - b.left as f64 - 0.5).ceil() as i64).clamp(0, w as i64) as usize;
                for px in &mut data[row * w + c0..row * w + c1.max(c0)] {
                    *px = color;
                }
            }
        }
    }
    img
}
