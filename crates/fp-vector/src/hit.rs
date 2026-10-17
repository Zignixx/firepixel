//! Trefferprüfung für Werkzeuge (Spezifikation 16 §1.5; Referenz `bezier.hit_test`).
//!
//! Das Original prüft Treffer mit der Präferenz `PickDistance` (Vorgabe 5 Bildschirmpixel); die
//! Toleranz in Dokumentpixeln ist `PickDistance / Zoom` (A für die Division, B für das Messer).
//!
//! - **Strich**: Abstand zur Polylinie ≤ wirksame Strichbreite + Toleranz (mittig `D/2`,
//!   innen/außen `D` auf der jeweiligen Seite).
//! - **Füllung**: Punkt innen nach der Füllregel des Pfads.
//! - **Kontur**: Abstand zur Polylinie ≤ Toleranz (ungefüllte Pfade, Unterauswahl).
//!
//! Für pixelgenaue Prüfung (z. B. Zauberstab auf Vektoren) dient [`hit_test_pixel`].

use crate::attrs::{Feedback, StrokePlacement, VectorPath};
use crate::contour_gradient::distance_to_polylines;
use crate::stroke::effective_placement;
use fp_geom::{IRect, Vec2};

/// Getroffener Teil eines Pfads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HitPart {
    /// Auf dem Strich.
    Stroke,
    /// In der Füllung.
    Fill,
    /// Nahe der Kontur (ohne Füllung/Strich an dieser Stelle).
    Outline,
}

/// Windungszahl bezüglich der (implizit geschlossenen) Polylinien, halboffene Kantenregel.
pub fn winding_number(path: &VectorPath, p: Vec2) -> i32 {
    let mut w = 0;
    for c in &path.contours {
        let pts = &c.points;
        let n = pts.len();
        for i in 0..n {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            if (a.y <= p.y && p.y < b.y) || (b.y <= p.y && p.y < a.y) {
                let xi = a.x + (p.y - a.y) * (b.x - a.x) / (b.y - a.y);
                if xi > p.x {
                    w += if b.y > a.y { 1 } else { -1 };
                }
            }
        }
    }
    w
}

/// Liegt der Punkt innen (Füllregel des Pfads)?
pub fn point_inside(path: &VectorPath, p: Vec2) -> bool {
    let w = winding_number(path, p);
    if path.even_odd { w & 1 == 1 } else { w != 0 }
}

/// Trefferprüfung mit Toleranz `tolerance` in Dokumentpixeln. Reihenfolge: Strich vor Füllung
/// vor Kontur (bei `FOTb = 1` Füllung vor Strich).
pub fn hit_test(path: &VectorPath, p: Vec2, tolerance: f64) -> Option<HitPart> {
    let dist = distance_to_polylines(path, p);
    let inside = point_inside(path, p);
    let stroke_hit = path.stroke.as_ref().is_some_and(|s| {
        if s.brush.feedback == Feedback::Background {
            return false;
        }
        let d = s.brush.diameter as f64;
        match effective_placement(path, s) {
            StrokePlacement::Center => dist <= d / 2.0 + tolerance,
            StrokePlacement::Inside => dist <= d + tolerance && (inside || dist <= tolerance),
            StrokePlacement::Outside => dist <= d + tolerance && (!inside || dist <= tolerance),
        }
    });
    let fill_hit = path.fill.is_some() && inside;
    match (stroke_hit, fill_hit) {
        (true, true) if path.fill_over_stroke => Some(HitPart::Fill),
        (true, _) => Some(HitPart::Stroke),
        (false, true) => Some(HitPart::Fill),
        _ if dist <= tolerance => Some(HitPart::Outline),
        _ => None,
    }
}

/// Pixelgenaue Prüfung: hat der gerenderte Pfad im Pixel `(x, y)` Alpha > 0?
pub fn hit_test_pixel(path: &VectorPath, x: i32, y: i32) -> bool {
    let (px, _) = crate::render::render_path_pixels(path, IRect::new(x, y, x + 1, y + 1));
    px.first().is_some_and(|p| p[3] > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attrs::{BrushParams, Fill, PathContour, Stroke};
    use fp_color::Argb;

    #[test]
    fn hits() {
        let sq = PathContour::new(
            vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(10.0, 0.0),
                Vec2::new(10.0, 10.0),
                Vec2::new(0.0, 10.0),
            ],
            true,
        );
        let mut p = VectorPath::new(vec![sq]);
        assert_eq!(hit_test(&p, Vec2::new(5.0, 5.0), 1.0), None);
        assert_eq!(
            hit_test(&p, Vec2::new(5.0, 0.5), 1.0),
            Some(HitPart::Outline)
        );
        p.fill = Some(Fill::solid(Argb(0xFF000000)));
        assert_eq!(hit_test(&p, Vec2::new(5.0, 5.0), 1.0), Some(HitPart::Fill));
        p.stroke = Some(Stroke::new(
            BrushParams {
                diameter: 4,
                ..Default::default()
            },
            Argb(0xFF000000),
        ));
        assert_eq!(
            hit_test(&p, Vec2::new(5.0, 1.5), 0.0),
            Some(HitPart::Stroke)
        );
        assert_eq!(hit_test(&p, Vec2::new(5.0, -3.0), 0.0), None);
        assert!(hit_test_pixel(&p, 5, 5));
        assert!(!hit_test_pixel(&p, 20, 20));
    }
}
