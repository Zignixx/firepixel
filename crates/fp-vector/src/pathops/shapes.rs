//! Grundformen als Bézier-Konturen (Spezifikation 16 §3.10–§3.12; Referenz `re\pfade\shapes.py`).
//!
//! - Ellipse: 4 Kurvenknoten links, oben, rechts, unten (auf dem Bildschirm im Uhrzeigersinn),
//!   Henkel `κ·r` mit **κ = 0,55229187012** (`fw.ellipseBCPConst`, nicht der Lehrbuchwert).
//! - Rechteck/Rundrechteck: Radius `RDSf·min(|B|,|H|)/2`; 8 Knoten, Start oben links nach der
//!   Rundung, im Uhrzeigersinn; Henkel `κ·r`.
//! - Polygon/Stern (`@00d8b680`): erste Ecke am Ziehpunkt, Seiten 3..360, Spitzigkeit 0..1,
//!   automatische Spitzigkeit `R·cos(2π/n′)/cos(π/n′)` mit `n′ = max(n, 5)` (A).

use super::geom::{Contour, Node};
use fp_geom::Vec2;

/// κ der Ellipsen und Rundungen (16 §3.11, B).
pub const KAPPA: f64 = 0.552_291_870_12;

/// Startknoten einer Ellipse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EllipseStart {
    #[default]
    Left,
    Top,
    Right,
    Bottom,
}

/// Ellipse um `(cx, cy)` mit Radien `rx`, `ry`.
pub fn ellipse(cx: f64, cy: f64, rx: f64, ry: f64, start: EllipseStart) -> Contour {
    let (kx, ky) = (rx * KAPPA, ry * KAPPA);
    let v = Vec2::new;
    let l = Node::new(v(cx - rx, cy), v(cx - rx, cy + ky), v(cx - rx, cy - ky));
    let t = Node::new(v(cx, cy - ry), v(cx - kx, cy - ry), v(cx + kx, cy - ry));
    let r = Node::new(v(cx + rx, cy), v(cx + rx, cy - ky), v(cx + rx, cy + ky));
    let b = Node::new(v(cx, cy + ry), v(cx + kx, cy + ry), v(cx - kx, cy + ry));
    let mut nodes = vec![l, t, r, b];
    nodes.rotate_left(start as usize);
    Contour::new(nodes, true)
}

/// Rechteck `(l, t)`–`(r, b)` (4 Eckknoten, Start oben links, im Uhrzeigersinn).
pub fn rect(l: f64, t: f64, r: f64, b: f64) -> Contour {
    Contour::polygon(&[Vec2::new(l, t), Vec2::new(r, t), Vec2::new(r, b), Vec2::new(l, b)], true)
}

/// Rundrechteck mit Eckradius in Pixeln (auf die halbe kurze Seite begrenzt).
pub fn rounded_rect(l: f64, t: f64, r: f64, b: f64, radius: f64) -> Contour {
    let (w, h) = (r - l, b - t);
    let rad = radius.min(w.abs().min(h.abs()) / 2.0).max(0.0);
    if rad <= 0.0 {
        return rect(l, t, r, b);
    }
    let k = rad * (1.0 - KAPPA);
    let v = Vec2::new;
    let n = vec![
        Node::new(v(l + rad, t), v(l + k, t), v(l + rad, t)),
        Node::new(v(r - rad, t), v(r - rad, t), v(r - k, t)),
        Node::new(v(r, t + rad), v(r, t + k), v(r, t + rad)),
        Node::new(v(r, b - rad), v(r, b - rad), v(r, b - k)),
        Node::new(v(r - rad, b), v(r - k, b), v(r - rad, b)),
        Node::new(v(l + rad, b), v(l + rad, b), v(l + k, b)),
        Node::new(v(l, b - rad), v(l, b - k), v(l, b - rad)),
        Node::new(v(l, t + rad), v(l, t + rad), v(l, t + k)),
    ];
    Contour::new(n, true)
}

/// Eckradius aus `PRIv.RDSf` (immer Bruchteil): `RDSf·min(|B|, |H|)/2`.
pub fn pri_radius(rds: f64, l: f64, t: f64, r: f64, b: f64) -> f64 {
    rds * (r - l).abs().min((b - t).abs()) / 2.0
}

/// Regelmäßiges Vieleck, erste Ecke bei `angle_deg`.
pub fn polygon(cx: f64, cy: f64, radius: f64, sides: usize, angle_deg: f64) -> Contour {
    let pts: Vec<Vec2> = (0..sides)
        .map(|i| {
            let a = angle_deg.to_radians() + 2.0 * std::f64::consts::PI * i as f64 / sides as f64;
            Vec2::new(cx + radius * a.cos(), cy + radius * a.sin())
        })
        .collect();
    Contour::polygon(&pts, true)
}

/// Stern mit `points` Spitzen (abwechselnd Außen- und Innenradius).
pub fn star(cx: f64, cy: f64, r_outer: f64, r_inner: f64, points: usize, angle_deg: f64) -> Contour {
    let pts: Vec<Vec2> = (0..2 * points)
        .map(|i| {
            let a = angle_deg.to_radians() + std::f64::consts::PI * i as f64 / points as f64;
            let rr = if i % 2 == 0 { r_outer } else { r_inner };
            Vec2::new(cx + rr * a.cos(), cy + rr * a.sin())
        })
        .collect();
    Contour::polygon(&pts, true)
}

/// Automatische Spitzigkeit (A): `n′ = max(n, 5)`, Innenradius `R·cos(2π/n′)/cos(π/n′)`.
pub fn star_inner_radius_automatic(r_outer: f64, points: usize) -> f64 {
    let n = points.max(5) as f64;
    r_outer * (2.0 * std::f64::consts::PI / n).cos() / (std::f64::consts::PI / n).cos()
}

/// Polygon-Werkzeug (`@00d8b680`, B): erste Ecke am Ziehpunkt `outside`; `spikiness` `None` =
/// automatisch. `None` bei Radius 0.
pub fn polygon_tool(center: Vec2, outside: Vec2, sides: usize, star_shape: bool, spikiness: Option<f64>) -> Option<Contour> {
    let sides = sides.clamp(3, 360);
    let r = center.distance(outside);
    if r <= 0.0 || !r.is_finite() {
        return None;
    }
    let ang = (outside.y - center.y).atan2(outside.x - center.x).to_degrees();
    if !star_shape {
        return Some(polygon(center.x, center.y, r, sides, ang));
    }
    let ri = match spikiness {
        Some(s) if s >= 0.0 => s.min(1.0) * r,
        _ => star_inner_radius_automatic(r, sides),
    };
    Some(star(center.x, center.y, r, ri, sides, ang))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shapes() {
        let e = ellipse(50.0, 50.0, 20.0, 10.0, EllipseStart::Left);
        assert_eq!(e.nodes[0].anchor, Vec2::new(30.0, 50.0));
        assert!((e.nodes[0].ctrl_out.y - (50.0 - 10.0 * KAPPA)).abs() < 1e-12);
        assert_eq!(rounded_rect(0.0, 0.0, 10.0, 10.0, 2.0).nodes.len(), 8);
        assert_eq!(rounded_rect(0.0, 0.0, 10.0, 10.0, 0.0).nodes.len(), 4);
        let s = polygon_tool(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0), 5, true, None).unwrap();
        assert_eq!(s.nodes.len(), 10);
        assert!((s.nodes[0].anchor.x - 10.0).abs() < 1e-12);
        assert!((star_inner_radius_automatic(1.0, 5) - 0.381_966).abs() < 1e-5);
    }
}
