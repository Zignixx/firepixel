//! Rechenregeln des Kurvendialogs (re\19-kurven-tonwert.md §2.9): „Auto“ und die drei Pipetten
//! setzen Kurvenpunkte, die Pixelarbeit macht danach der Tabellenfilter ([`super::table`]).
//!
//! Alle Funktionen sind rein (keine Oberfläche), damit sie sich gegen die Zahlenbeispiele der
//! Spezifikation prüfen lassen.

use super::table::{histogram_bounds, CurvePoint, CurvesParams};

/// Die drei Pipetten der Kurven- und Tonwertdialoge, links → rechts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pipette {
    /// Schwarzpunkt.
    Shadow,
    /// Graupunkt.
    Midtone,
    /// Weißpunkt.
    Highlight,
}

impl Pipette {
    /// Alle drei in der Reihenfolge der Knöpfe.
    pub const ALL: [Pipette; 3] = [Pipette::Shadow, Pipette::Midtone, Pipette::Highlight];
}

/// Kaufmännisch runden wie das Original (`floor(v + 0,5)`).
#[inline]
pub fn round_half_up(v: f32) -> f32 {
    (v + 0.5).floor()
}

/// Identitätskurve (0,0)–(255,255).
pub fn identity_points() -> Vec<CurvePoint> {
    vec![CurvePoint { x: 0.0, y: 0.0 }, CurvePoint { x: 255.0, y: 255.0 }]
}

fn pt(x: f32, y: f32) -> CurvePoint {
    CurvePoint { x, y }
}

/// Punktliste eines Kanals (0 = RGB, 1 = Rot, 2 = Grün, 3 = Blau).
pub fn channel_points(p: &CurvesParams, ch: usize) -> &Vec<CurvePoint> {
    match ch {
        1 => &p.red_points,
        2 => &p.green_points,
        3 => &p.blue_points,
        _ => &p.rgb_points,
    }
}

/// Veränderbare Punktliste eines Kanals (0 = RGB, 1 = Rot, 2 = Grün, 3 = Blau).
pub fn channel_points_mut(p: &mut CurvesParams, ch: usize) -> &mut Vec<CurvePoint> {
    match ch {
        1 => &mut p.red_points,
        2 => &mut p.green_points,
        3 => &mut p.blue_points,
        _ => &mut p.rgb_points,
    }
}

/// Erster und letzter Punkt einer Liste (leere bzw. einpunktige Listen gelten als Identität).
fn ends(list: &[CurvePoint]) -> (CurvePoint, CurvePoint) {
    match (list.first(), list.last()) {
        (Some(a), Some(b)) if list.len() >= 2 => (*a, *b),
        _ => (pt(0.0, 0.0), pt(255.0, 255.0)),
    }
}

/// „Auto“ im Kurvendialog (§2.9): `hist` = Histogramme R, G, B des Originalbilds (nur Pixel mit
/// Alpha > 0). RGB-Kurve wird Identität, jeder Kanal eine Strecke (lo,0)–(hi,255) mit den
/// 0,5-%-Grenzen wie Auto-Tonwert; innere Punkte entfallen.
pub fn auto_points(hist: &[[u64; 256]; 3]) -> CurvesParams {
    let ch = |h: &[u64; 256]| {
        let (lower, upper) = histogram_bounds(h);
        let lo = lower.clamp(0, 251);
        let hi = upper.max(lo + 4).min(255);
        vec![pt(lo as f32, 0.0), pt(hi as f32, 255.0)]
    };
    CurvesParams { rgb_points: identity_points(), red_points: ch(&hist[0]), green_points: ch(&hist[1]), blue_points: ch(&hist[2]) }
}

/// Wirkung einer Pipette auf die Kurven (§2.9). `c` = aufgenommene, **angezeigte** Farbe (bei
/// eingeschalteter Vorschau also schon korrigiert).
pub fn apply_pipette(p: &mut CurvesParams, which: Pipette, c: [u8; 3]) {
    if which != Pipette::Midtone {
        p.rgb_points = identity_points();
    }
    for k in 0..3 {
        let list = channel_points_mut(p, k + 1);
        let (mut lo, mut hi) = ends(list);
        let ck = c[k] as f32;
        match which {
            Pipette::Shadow => lo = pt(ck, 0.0),
            Pipette::Highlight => hi = pt(ck, 255.0),
            Pipette::Midtone => {}
        }
        lo.x = lo.x.clamp(0.0, 251.0);
        hi.x = hi.x.clamp(4.0, 255.0);
        *list = match which {
            Pipette::Shadow => {
                hi.x = hi.x.max(lo.x + 4.0).min(255.0);
                vec![lo, hi]
            }
            Pipette::Highlight => {
                lo.x = lo.x.max(0.0).min(hi.x - 4.0);
                vec![lo, hi]
            }
            Pipette::Midtone => {
                // clamp(c, lo.x + 2, hi.x − 2) ohne Panik bei vertauschten Grenzen
                let mx = round_half_up(ck.max(lo.x + 2.0).min(hi.x - 2.0));
                vec![lo, pt(mx, (lo.y + hi.y) / 2.0), hi]
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filters::table::curve_table;

    fn pts(v: &[(f32, f32)]) -> Vec<CurvePoint> {
        v.iter().map(|&(x, y)| pt(x, y)).collect()
    }

    #[test]
    fn shadow_example_of_the_spec() {
        // §2.9: Schatten auf (200,100,50) → Rot (200,0)–(255,255), Grün (100,0)–…, Blau (50,0)–…
        let mut p = CurvesParams::default();
        apply_pipette(&mut p, Pipette::Shadow, [200, 100, 50]);
        assert_eq!(p.red_points, pts(&[(200.0, 0.0), (255.0, 255.0)]));
        assert_eq!(p.green_points, pts(&[(100.0, 0.0), (255.0, 255.0)]));
        assert_eq!(p.blue_points, pts(&[(50.0, 0.0), (255.0, 255.0)]));
        assert_eq!(p.rgb_points, identity_points());
    }

    #[test]
    fn midtone_after_shadow_uses_displayed_color() {
        // §1.3: nach der Schatten-Korrektur erscheint (40,120,220) als (0,33,211)
        let mut p = CurvesParams::default();
        apply_pipette(&mut p, Pipette::Shadow, [200, 100, 50]);
        let t = p.tables();
        let shown = [t.rgb[t.red[40] as usize], t.rgb[t.green[120] as usize], t.rgb[t.blue[220] as usize]];
        assert_eq!(shown, [0, 33, 211]);
        apply_pipette(&mut p, Pipette::Midtone, shown);
        assert_eq!(p.red_points, pts(&[(200.0, 0.0), (202.0, 127.5), (255.0, 255.0)]));
        assert_eq!(p.green_points, pts(&[(100.0, 0.0), (102.0, 127.5), (255.0, 255.0)]));
        assert_eq!(p.blue_points, pts(&[(50.0, 0.0), (211.0, 127.5), (255.0, 255.0)]));
    }

    #[test]
    fn midtone_keeps_rgb_and_end_y() {
        let mut p = CurvesParams { rgb_points: pts(&[(0.0, 0.0), (64.0, 124.0), (255.0, 255.0)]), ..Default::default() };
        p.red_points = pts(&[(10.0, 30.0), (100.0, 50.0), (250.0, 200.0)]);
        apply_pipette(&mut p, Pipette::Midtone, [128, 128, 128]);
        assert_eq!(p.rgb_points, pts(&[(0.0, 0.0), (64.0, 124.0), (255.0, 255.0)]));
        assert_eq!(p.red_points, pts(&[(10.0, 30.0), (128.0, 115.0), (250.0, 200.0)]));
    }

    #[test]
    fn highlight_and_limits() {
        let mut p = CurvesParams::default();
        apply_pipette(&mut p, Pipette::Shadow, [250, 0, 0]);
        // Schatten 250: Grenze lo ≤ 251, hi ≥ lo + 4
        assert_eq!(p.red_points, pts(&[(250.0, 0.0), (255.0, 255.0)]));
        apply_pipette(&mut p, Pipette::Highlight, [2, 200, 0]);
        // Lichter 2: hi.x ≥ 4, lo.x ≤ hi.x − 4
        assert_eq!(p.red_points, pts(&[(0.0, 0.0), (4.0, 255.0)]));
        assert_eq!(p.green_points, pts(&[(0.0, 0.0), (200.0, 255.0)]));
        assert_eq!(p.blue_points, pts(&[(0.0, 0.0), (4.0, 255.0)]));
        // Mittelton mit engem Bereich: m.x = clamp(c, lo+2, hi−2)
        apply_pipette(&mut p, Pipette::Midtone, [255, 0, 3]);
        assert_eq!(p.red_points, pts(&[(0.0, 0.0), (2.0, 127.5), (4.0, 255.0)]));
        assert_eq!(p.green_points[1], pt(2.0, 127.5));
    }

    #[test]
    fn auto_on_range_20_230() {
        // `kt_auto`: Verlauf 20…230 → Kanalkurven (20,0)–(230,255); Kurven-Auto rundet (32 → 15)
        let mut h = [[0u64; 256]; 3];
        for c in &mut h {
            for v in 20..=230 {
                c[v] = 100;
            }
            // Ränder wie beim Verlauf mit Farbfeldern: genug Pixel an den Enden
            c[20] = 1000;
            c[230] = 1000;
        }
        let p = auto_points(&h);
        assert_eq!(p.red_points, pts(&[(20.0, 0.0), (230.0, 255.0)]));
        assert_eq!(p.rgb_points, identity_points());
        let t = curve_table(&p.red_points);
        assert_eq!(t[32], 15);
        assert_eq!((t[20], t[230], t[0], t[255]), (0, 255, 0, 255));
        // Grenzen: alles bei 254 → lo 251, hi 255
        let mut h2 = [[0u64; 256]; 3];
        h2[0][254] = 10;
        h2[1][3] = 10;
        let q = auto_points(&h2);
        assert_eq!(q.red_points, pts(&[(251.0, 0.0), (255.0, 255.0)]));
        assert_eq!(q.green_points, pts(&[(3.0, 0.0), (7.0, 255.0)]));
    }

    #[test]
    fn spline_overshoot_example() {
        // §2.6: Punkte (0,0) (18,20) (25,26) (128,201) (192,194) (255,255) → Bogen bis 208
        let t = curve_table(&pts(&[(0.0, 0.0), (18.0, 20.0), (25.0, 26.0), (128.0, 201.0), (192.0, 194.0), (255.0, 255.0)]));
        assert_eq!(t[128..=192].iter().copied().max(), Some(208));
    }

    #[test]
    fn rounding_half_up() {
        assert_eq!(round_half_up(63.14), 63.0);
        assert_eq!(round_half_up(128.71), 129.0);
        assert_eq!(round_half_up(127.5), 128.0);
    }
}
