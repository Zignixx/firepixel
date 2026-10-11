//! Schmale Geometrie-Schnittstelle für Befehle und Werkzeuge.
//!
//! Befehle brauchen Pfadoperationen (Vereinigen, Strich erweitern …), Treffertests und
//! Deckungsmasken. Damit fp-engine auch baut, wenn sich die API von fp-vector noch bewegt, laufen
//! diese Aufrufe über das Trait [`GeometryOps`]:
//!
//! - [`BasicGeometry`]: eingebaut (nur fp-geom): Grenzen, Treffertest, Deckung, Grundformen;
//!   boolesche Operationen/Offsets/Vereinfachen liefern [`EngineError::BackendMissing`].
//! - `VectorGeometry` (Feature `vector`, Vorgabe): Pfadoperationen über fp-vector (16).
//!
//! Konturen sind die des Dokumentmodells ([`fp_model::Contour`], `f32`).

use crate::error::{EngineError, Result};
use fp_geom::{BezierContour, BezierNode, IRect, Point, Rect, Vec2};
use fp_model::{Contour, ContourNode, Extra};
use fp_raster::Mask;
use std::sync::Arc;

/// Destruktive Pfadkombination (Modifizieren ▸ Pfade kombinieren).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathBoolOp {
    /// Ein Pfad aus allen Flächen.
    Union,
    /// Schnittmenge aller Pfade.
    Intersect,
    /// Oberster Pfad stanzt aus jedem weiteren aus.
    Punch,
    /// Oberster Pfad schneidet jedes weitere zu.
    Crop,
}

/// Eine Fläche: Konturen und Füllregel (`EOFb`).
#[derive(Clone, Debug, PartialEq)]
pub struct PathShape {
    pub contours: Vec<Contour>,
    pub even_odd: bool,
}

/// Geometrie-Operationen (siehe Moduldoku).
pub trait GeometryOps: Send + Sync {
    /// Name der Umsetzung (für Diagnose).
    fn name(&self) -> &'static str;

    /// Pfade kombinieren; `paths` oberster zuerst. Vereinigen/Überschneiden liefern ein Ergebnis,
    /// Ausstanzen/Zuschneiden eines je Ziel (`paths[1..]`).
    fn combine(&self, op: PathBoolOp, paths: &[PathShape]) -> Result<Vec<Vec<Contour>>>;

    /// Strich erweitern (`width` = Strichbreite; Ecken 0 Bevel/1 Rund/2 Gehrung; Kappen 0/1/2).
    fn expand_stroke(&self, contours: &[Contour], width: f64, join: u32, cap: u32, miter: f64) -> Result<Vec<Contour>>;

    /// Pfad einlassen (`expand = false`) bzw. erweitern um `distance` px.
    fn inset(&self, contours: &[Contour], distance: f64, expand: bool, join: u32, miter: f64) -> Result<Vec<Contour>>;

    /// Vereinfachen mit Stärke `amount`.
    fn simplify(&self, contours: &[Contour], amount: f64) -> Result<Vec<Contour>>;

    /// Treffertest: liegt `p` in der Füllung (falls `filled`) oder innerhalb `stroke_width/2 +
    /// tolerance` an der Kontur?
    fn hit_test(&self, shape: &PathShape, p: Point, stroke_width: f64, filled: bool, tolerance: f64) -> bool {
        basic_hit_test(shape, p, stroke_width, filled, tolerance)
    }

    /// Deckungsmaske der Fläche innerhalb `bounds` (geglättet mit 4×4-Abtastung oder hart).
    fn coverage(&self, shape: &PathShape, bounds: IRect, antialias: bool) -> Mask {
        basic_coverage(shape, bounds, antialias)
    }
}

/// Eingebaute Grundgeometrie ohne Pfadoperationen.
#[derive(Clone, Copy, Debug, Default)]
pub struct BasicGeometry;

impl GeometryOps for BasicGeometry {
    fn name(&self) -> &'static str {
        "basic"
    }
    fn combine(&self, _op: PathBoolOp, _paths: &[PathShape]) -> Result<Vec<Vec<Contour>>> {
        Err(EngineError::BackendMissing("err-backend-pathops"))
    }
    fn expand_stroke(&self, _c: &[Contour], _w: f64, _j: u32, _cap: u32, _m: f64) -> Result<Vec<Contour>> {
        Err(EngineError::BackendMissing("err-backend-pathops"))
    }
    fn inset(&self, _c: &[Contour], _d: f64, _e: bool, _j: u32, _m: f64) -> Result<Vec<Contour>> {
        Err(EngineError::BackendMissing("err-backend-pathops"))
    }
    fn simplify(&self, _c: &[Contour], _a: f64) -> Result<Vec<Contour>> {
        Err(EngineError::BackendMissing("err-backend-pathops"))
    }
}

/// Vorgabe-Geometrie: mit Feature `vector` fp-vector, sonst die Grundgeometrie.
pub fn default_geometry() -> Arc<dyn GeometryOps> {
    #[cfg(feature = "vector")]
    {
        Arc::new(crate::vector_backend::VectorGeometry)
    }
    #[cfg(not(feature = "vector"))]
    {
        Arc::new(BasicGeometry)
    }
}

// ------------------------------------------------------------------ Umwandlungen

/// Modellkontur → fp-geom-Kontur.
pub fn to_bezier(c: &Contour) -> BezierContour {
    BezierContour::new(c.nodes.iter().map(|n| BezierNode::new(n.anchor, n.pred, n.succ)).collect(), c.closed)
}

/// fp-geom-Kontur → Modellkontur (`CRVb` bei Knoten mit Henkeln).
pub fn from_bezier(c: &BezierContour) -> Contour {
    Contour {
        closed: c.closed,
        bsl: None,
        nodes: c
            .nodes
            .iter()
            .map(|n| {
                let mut k = ContourNode::corner(n.pt);
                k.pred = n.ctrl_in;
                k.succ = n.ctrl_out;
                k.curve = n.ctrl_in != n.pt || n.ctrl_out != n.pt;
                k
            })
            .collect(),
        extra: Extra::default(),
    }
}

/// Neue Modellkontur aus Knoten (Anker, Eingang, Ausgang).
pub fn contour_from_nodes(nodes: &[(Point, Point, Point)], closed: bool) -> Contour {
    Contour {
        closed,
        bsl: None,
        nodes: nodes
            .iter()
            .map(|&(a, i, o)| {
                let mut k = ContourNode::corner(a);
                k.pred = i;
                k.succ = o;
                k.curve = i != a || o != a;
                k
            })
            .collect(),
        extra: Extra::default(),
    }
}

/// Polygon als Modellkontur.
pub fn polygon_contour(pts: &[Point], closed: bool) -> Contour {
    Contour { closed, bsl: None, nodes: pts.iter().map(|&p| ContourNode::corner(p)).collect(), extra: Extra::default() }
}

/// Abgeflachte Kontur (Sehnenverfahren, Toleranz 0,1 px).
pub fn flatten(c: &Contour) -> Vec<Point> {
    let bc = to_bezier(c);
    if bc.nodes.len() == 1 {
        return vec![bc.nodes[0].pt];
    }
    fp_geom::flatten::flatten_contour_chord(&bc, 0.1, 12).into_iter().map(|v| v.to_point()).collect()
}

/// Exakte Grenzen mehrerer Konturen (`None` ohne Knoten).
pub fn contours_bounds(contours: &[Contour]) -> Option<Rect> {
    let mut out: Option<fp_geom::Bounds> = None;
    for c in contours {
        if let Some(b) = to_bezier(c).bounds(true) {
            out = Some(match out {
                Some(o) => o.union(&b),
                None => b,
            });
        }
    }
    out.map(|b| Rect::new(b.min.x as f32, b.min.y as f32, b.max.x as f32, b.max.y as f32))
}

fn dist_point_segment(p: Vec2, a: Vec2, b: Vec2) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let l2 = dx * dx + dy * dy;
    let t = if l2 > 0.0 { (((p.x - a.x) * dx + (p.y - a.y) * dy) / l2).clamp(0.0, 1.0) } else { 0.0 };
    let (qx, qy) = (a.x + t * dx, a.y + t * dy);
    (p.x - qx).hypot(p.y - qy)
}

/// Kleinster Abstand eines Punkts zu den (abgeflachten) Konturen.
pub fn distance_to_contours(contours: &[Contour], p: Point) -> f64 {
    let pv = Vec2::from(p);
    let mut best = f64::INFINITY;
    for c in contours {
        let pts = flatten(c);
        if pts.len() == 1 {
            best = best.min(pv.distance(Vec2::from(pts[0])));
        }
        let n = pts.len();
        let segs = if c.closed { n } else { n.saturating_sub(1) };
        for i in 0..segs {
            best = best.min(dist_point_segment(pv, Vec2::from(pts[i]), Vec2::from(pts[(i + 1) % n])));
        }
    }
    best
}

/// Liegt der Punkt in der Fläche (Füllregel der Form)?
pub fn point_in_shape(shape: &PathShape, p: Point) -> bool {
    let polys: Vec<Vec<Point>> = shape.contours.iter().map(flatten).filter(|v| v.len() > 2).collect();
    fp_geom::contour::point_inside(polys.iter().map(|v| v.as_slice()), Vec2::from(p), shape.even_odd)
}

/// Treffertest der Grundgeometrie.
pub fn basic_hit_test(shape: &PathShape, p: Point, stroke_width: f64, filled: bool, tolerance: f64) -> bool {
    if filled && point_in_shape(shape, p) {
        return true;
    }
    distance_to_contours(&shape.contours, p) <= stroke_width.max(0.0) / 2.0 + tolerance
}

/// Deckung der Grundgeometrie (Pixelmitte bzw. 4×4-Abtastung).
pub fn basic_coverage(shape: &PathShape, bounds: IRect, antialias: bool) -> Mask {
    let Some(sb) = contours_bounds(&shape.contours) else { return Mask::empty_at(bounds.left, bounds.top) };
    let r = bounds.intersect(&sb.round_out());
    let Ok(mut m) = Mask::new(r) else { return Mask::empty_at(bounds.left, bounds.top) };
    if r.is_empty() {
        return m;
    }
    let polys: Vec<Vec<Point>> = shape.contours.iter().map(flatten).filter(|v| v.len() > 2).collect();
    let inside = |x: f64, y: f64| fp_geom::contour::point_inside(polys.iter().map(|v| v.as_slice()), Vec2::new(x, y), shape.even_odd);
    for y in r.top..r.bottom {
        for x in r.left..r.right {
            let v = if antialias {
                let mut n = 0u32;
                for sy in 0..4 {
                    for sx in 0..4 {
                        if inside(x as f64 + (sx as f64 + 0.5) / 4.0, y as f64 + (sy as f64 + 0.5) / 4.0) {
                            n += 1;
                        }
                    }
                }
                ((n * 255 + 8) / 16) as u8
            } else if inside(x as f64 + 0.5, y as f64 + 0.5) {
                255
            } else {
                0
            };
            m.set(x, y, v);
        }
    }
    m
}

// ------------------------------------------------------------------ Grundformen (16 §3.10–§3.12)

/// κ der Ellipsen und Rundungen (`fw.ellipseBCPConst`).
pub const KAPPA: f64 = 0.552_291_870_12;

fn p(x: f64, y: f64) -> Point {
    Point::new(x as f32, y as f32)
}

/// Ellipse im Rechteck `r`: 4 Kurvenknoten links, oben, rechts, unten.
pub fn ellipse_contour(r: Rect) -> Contour {
    let (cx, cy) = (((r.left + r.right) / 2.0) as f64, ((r.top + r.bottom) / 2.0) as f64);
    let (rx, ry) = ((r.width() / 2.0).abs() as f64, (r.height() / 2.0).abs() as f64);
    let (kx, ky) = (rx * KAPPA, ry * KAPPA);
    contour_from_nodes(
        &[
            (p(cx - rx, cy), p(cx - rx, cy + ky), p(cx - rx, cy - ky)),
            (p(cx, cy - ry), p(cx - kx, cy - ry), p(cx + kx, cy - ry)),
            (p(cx + rx, cy), p(cx + rx, cy - ky), p(cx + rx, cy + ky)),
            (p(cx, cy + ry), p(cx + kx, cy + ry), p(cx - kx, cy + ry)),
        ],
        true,
    )
}

/// Rechteck bzw. Rundrechteck (`roundness` 0..1 wie `RDSf`: Radius = Rundung · kurze Seite / 2).
pub fn rect_contour(r: Rect, roundness: f64) -> Contour {
    let (l, t, rr, b) = (r.left as f64, r.top as f64, r.right as f64, r.bottom as f64);
    let rad = (roundness.clamp(0.0, 1.0) * (rr - l).abs().min((b - t).abs()) / 2.0).max(0.0);
    if rad <= 0.0 {
        return polygon_contour(&[p(l, t), p(rr, t), p(rr, b), p(l, b)], true);
    }
    let k = rad * (1.0 - KAPPA);
    contour_from_nodes(
        &[
            (p(l + rad, t), p(l + k, t), p(l + rad, t)),
            (p(rr - rad, t), p(rr - rad, t), p(rr - k, t)),
            (p(rr, t + rad), p(rr, t + k), p(rr, t + rad)),
            (p(rr, b - rad), p(rr, b - rad), p(rr, b - k)),
            (p(rr - rad, b), p(rr - k, b), p(rr - rad, b)),
            (p(l + rad, b), p(l + rad, b), p(l + k, b)),
            (p(l, b - rad), p(l, b - k), p(l, b - rad)),
            (p(l, t + rad), p(l, t + rad), p(l, t + k)),
        ],
        true,
    )
}

/// Polygon-Werkzeug (`@00d8b680`): erste Ecke am Ziehpunkt `corner`, Seiten 3..360, Stern mit
/// Spitzigkeit 0..1 (`None` = automatisch `R·cos(2π/n′)/cos(π/n′)`, `n′ = max(n, 5)`).
pub fn polygon_tool_contour(center: Point, corner: Point, sides: usize, star: bool, spikiness: Option<f64>) -> Option<Contour> {
    let sides = sides.clamp(3, 360);
    let (cx, cy) = (center.x as f64, center.y as f64);
    let r = (corner.x as f64 - cx).hypot(corner.y as f64 - cy);
    if r <= 0.0 || !r.is_finite() {
        return None;
    }
    let ang = (corner.y as f64 - cy).atan2(corner.x as f64 - cx);
    let pts: Vec<Point> = if !star {
        (0..sides)
            .map(|i| {
                let a = ang + 2.0 * std::f64::consts::PI * i as f64 / sides as f64;
                p(cx + r * a.cos(), cy + r * a.sin())
            })
            .collect()
    } else {
        let ri = match spikiness {
            Some(s) if s >= 0.0 => s.min(1.0) * r,
            _ => {
                let n = sides.max(5) as f64;
                r * (2.0 * std::f64::consts::PI / n).cos() / (std::f64::consts::PI / n).cos()
            }
        };
        (0..2 * sides)
            .map(|i| {
                let a = ang + std::f64::consts::PI * i as f64 / sides as f64;
                let rr = if i % 2 == 0 { r } else { ri };
                p(cx + rr * a.cos(), cy + rr * a.sin())
            })
            .collect()
    };
    Some(polygon_contour(&pts, true))
}

/// Ramer-Douglas-Peucker-Vereinfachung einer Punktfolge (Freihandwerkzeuge).
pub fn simplify_polyline(pts: &[Point], tolerance: f64) -> Vec<Point> {
    if pts.len() < 3 {
        return pts.to_vec();
    }
    let mut keep = vec![false; pts.len()];
    keep[0] = true;
    keep[pts.len() - 1] = true;
    let mut stack = vec![(0usize, pts.len() - 1)];
    while let Some((a, b)) = stack.pop() {
        if b <= a + 1 {
            continue;
        }
        let (pa, pb) = (Vec2::from(pts[a]), Vec2::from(pts[b]));
        let mut best = (0.0, a);
        for (i, q) in pts.iter().enumerate().take(b).skip(a + 1) {
            let d = dist_point_segment(Vec2::from(*q), pa, pb);
            if d > best.0 {
                best = (d, i);
            }
        }
        if best.0 > tolerance {
            keep[best.1] = true;
            stack.push((a, best.1));
            stack.push((best.1, b));
        }
    }
    pts.iter().zip(keep).filter(|(_, k)| *k).map(|(p, _)| *p).collect()
}

/// Glatte Kontur durch Punkte (Catmull-Rom → Bézier), für Freihand-Vektorpfade.
pub fn smooth_contour(pts: &[Point], closed: bool) -> Contour {
    let n = pts.len();
    if n < 3 {
        return polygon_contour(pts, closed);
    }
    let get = |i: isize| -> Vec2 {
        let k = if closed { i.rem_euclid(n as isize) as usize } else { i.clamp(0, n as isize - 1) as usize };
        Vec2::from(pts[k])
    };
    let nodes: Vec<(Point, Point, Point)> = (0..n as isize)
        .map(|i| {
            let (a, b, c) = (get(i - 1), get(i), get(i + 1));
            let t = Vec2::new((c.x - a.x) / 6.0, (c.y - a.y) / 6.0);
            let inn = Vec2::new(b.x - t.x, b.y - t.y);
            let out = Vec2::new(b.x + t.x, b.y + t.y);
            if !closed && (i == 0 || i == n as isize - 1) {
                (b.to_point(), b.to_point(), b.to_point())
            } else {
                (b.to_point(), inn.to_point(), out.to_point())
            }
        })
        .collect();
    contour_from_nodes(&nodes, closed)
}

/// Umrandet eine Maske (Schwelle 128) mit achsenparallelen Polygonen (Auswahlrahmen → Pfad).
/// Jede Kontur umläuft eine zusammenhängende Kante; Füllregel gerade/ungerade.
pub fn trace_mask(mask: &Mask) -> Vec<Contour> {
    let b = mask.bounds();
    if b.is_empty() {
        return Vec::new();
    }
    let on = |x: i32, y: i32| mask.get(x, y) >= 128;
    // Gerichtete Kanten (Pixelgrenzen) so, dass das Innere rechts liegt (Bildschirm-Uhrzeigersinn).
    use std::collections::HashMap;
    let mut next: HashMap<(i32, i32), Vec<(i32, i32)>> = HashMap::new();
    let mut add = |a: (i32, i32), c: (i32, i32)| next.entry(a).or_default().push(c);
    for y in b.top..b.bottom {
        for x in b.left..b.right {
            if !on(x, y) {
                continue;
            }
            if !on(x, y - 1) {
                add((x, y), (x + 1, y));
            }
            if !on(x + 1, y) {
                add((x + 1, y), (x + 1, y + 1));
            }
            if !on(x, y + 1) {
                add((x + 1, y + 1), (x, y + 1));
            }
            if !on(x - 1, y) {
                add((x, y + 1), (x, y));
            }
        }
    }
    let mut out = Vec::new();
    let mut starts: Vec<(i32, i32)> = next.keys().copied().collect();
    starts.sort();
    for s in starts {
        while next.get(&s).is_some_and(|v| !v.is_empty()) {
            let mut pts = vec![s];
            let mut cur = s;
            let mut guard = 0usize;
            while let Some(n) = next.get_mut(&cur).and_then(|l| l.pop()) {
                cur = n;
                if cur == s {
                    break;
                }
                pts.push(cur);
                guard += 1;
                if guard > 50_000_000 {
                    break;
                }
            }
            // kollineare Punkte entfernen
            let m = pts.len();
            let simplified: Vec<Point> = (0..m)
                .filter(|&i| {
                    let (a, c, d) = (pts[(i + m - 1) % m], pts[i], pts[(i + 1) % m]);
                    (c.0 - a.0) * (d.1 - c.1) != (c.1 - a.1) * (d.0 - c.0)
                })
                .map(|i| Point::new(pts[i].0 as f32, pts[i].1 as f32))
                .collect();
            if simplified.len() >= 3 {
                out.push(polygon_contour(&simplified, true));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shapes_and_hits() {
        let r = rect_contour(Rect::new(0.0, 0.0, 10.0, 10.0), 0.0);
        let s = PathShape { contours: vec![r], even_odd: true };
        assert!(basic_hit_test(&s, Point::new(5.0, 5.0), 0.0, true, 0.0));
        assert!(!basic_hit_test(&s, Point::new(15.0, 5.0), 1.0, true, 1.0));
        assert!(basic_hit_test(&s, Point::new(10.4, 5.0), 1.0, false, 0.0));
        let m = basic_coverage(&s, IRect::new(-5, -5, 20, 20), false);
        assert_eq!(m.tight_bounds(), IRect::new(0, 0, 10, 10));
        let traced = trace_mask(&m);
        assert_eq!(traced.len(), 1);
        assert_eq!(traced[0].nodes.len(), 4);
        let e = ellipse_contour(Rect::new(0.0, 0.0, 20.0, 10.0));
        let b = contours_bounds(&[e]).unwrap_or_default();
        assert!((b.right - 20.0).abs() < 1e-3 && (b.bottom - 10.0).abs() < 1e-3);
        let poly = polygon_tool_contour(Point::new(0.0, 0.0), Point::new(10.0, 0.0), 5, false, None);
        assert_eq!(poly.map(|c| c.nodes.len()), Some(5));
    }
}
