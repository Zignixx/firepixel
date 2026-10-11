//! Strich erweitern und Pfad einlassen (Spezifikation 16 §3.3, §3.4; Referenz `re\pfade\offset.py`).
//!
//! - **Strich erweitern** (`ApuExpandStroke`): geschlossene Kontur → Ring aus Offset `+h` und `−h`
//!   (Innenseite umgedreht); offene Kontur → `+h`-Seite, Kappe, `−h`-Seite umgedreht, Kappe. Danach
//!   Bereinigung mit Regel „Windung > 0“ (`ApuCleanAfterExpand`) und Zusammenfassen kollinearer
//!   Geradenstücke (`ApuCoalescePathCurves`, Laufzeittest T16: 3/3 Konturen gleiche Knotenfolge).
//!   Ecken: 0 Bevel, 1 Round, 2 Miter; Kappen: 0 Butt, 1 Square, 2 Round. Gehrungsgrenze [1; 10].
//! - **Pfad einlassen** (`ApuExpandPath`): Pfad normalisieren (Außen positiv), einseitiger Offset
//!   (positiv = innen/verkleinern), Bereinigung „Windung > 0“. Gehrungsgrenze [4; 10].
//!
//! Attribute: Strich erweitern tauscht **nicht** Strich und Füllung (B Laufzeit T16); das Ergebnis
//! behält Füllung, Strich, Effekte und `POPi` der Quelle (Sache des Aufrufers).
//!
//! Abweichungen von python.dll (A, wie die Referenz): vorgeschaltetes `ApuSimplifyPath`/`ApuRefitPath`
//! entfällt; runde Ecken als echter Kreisbogen (mit [`OffsetOptions::ORIGINAL`] die
//! 0,139-Formel).

use super::boolean::{boolean, merge_collinear, BoolOp, FillRule, Region, GRID};
use super::geom::{flatten, signed_area, Contour, Node};
use fp_geom::offset::{cap_segments, clamp_miter_limit, join_segments, offset_segment, CapKind, JoinKind, OffsetOptions, OffsetUse};
use fp_geom::{CubicBez, Vec2};

/// Einseitiger Offset einer Kontur (links positiv), ungereinigt: Segment-Offsets und Ecken.
pub fn offset_contour(c: &Contour, d: f64, join: JoinKind, miter_limit: f64, opts: &OffsetOptions) -> Option<Vec<CubicBez>> {
    let segs: Vec<CubicBez> = c.segments().into_iter().filter(|s| !s.is_degenerate()).collect();
    if segs.is_empty() {
        return None;
    }
    let pieces: Vec<Vec<CubicBez>> = segs.iter().map(|s| offset_segment(s, d, opts)).collect();
    let n = segs.len();
    let mut out = Vec::new();
    for k in 0..n {
        out.extend_from_slice(&pieces[k]);
        if k < n - 1 || c.closed {
            let nk = (k + 1) % n;
            let (Some(last), Some(first)) = (pieces[k].last(), pieces[nk].first()) else { continue };
            out.extend(join_segments(segs[k].p3, last.p3, segs[k].tangent(1.0), first.p0, segs[nk].tangent(0.0), d, join, miter_limit, opts));
        }
    }
    Some(out)
}

/// Segmentfolge → geschlossene Kontur.
pub fn segs_to_contour(segs: &[CubicBez]) -> Contour {
    let n = segs.len();
    Contour::new((0..n).map(|k| Node::new(segs[k].p0, segs[(k + n - 1) % n].p2, segs[k].p1)).collect(), true)
}

fn oriented_positive(c: &Contour) -> Contour {
    let f = flatten(std::slice::from_ref(c), 0.05);
    let a = f.first().map(|(p, _)| signed_area(p)).unwrap_or(0.0);
    if a >= 0.0 { c.clone() } else { c.reversed() }
}

/// Bereinigung mit Regel „Windung > 0“ (`ApuCleanAfterExpand`).
pub fn positive_union(contours: Vec<Contour>) -> Vec<Contour> {
    boolean(&Region::new(contours, FillRule::Positive), &Region::empty(), BoolOp::Union, GRID)
}

/// Strich in Fläche umwandeln (`pathExpand`). `width` = Gesamtbreite.
pub fn expand_stroke(contours: &[Contour], width: f64, join: JoinKind, miter_limit: f64, cap: CapKind, opts: &OffsetOptions) -> Vec<Contour> {
    let h = width.abs() / 2.0;
    let miter = clamp_miter_limit(miter_limit, OffsetUse::ExpandStroke);
    let mut rings = Vec::new();
    for c in contours {
        if c.closed {
            let c = oriented_positive(c);
            if let Some(outer) = offset_contour(&c, -h, join, miter, opts) {
                rings.push(segs_to_contour(&outer));
            }
            if let Some(inner) = offset_contour(&c, h, join, miter, opts) {
                rings.push(segs_to_contour(&inner).reversed());
            }
        } else {
            let (Some(left), Some(right)) = (offset_contour(c, h, join, miter, opts), offset_contour(&c.reversed(), h, join, miter, opts)) else {
                continue;
            };
            let segs = c.segments();
            let (Some(last), Some(first)) = (segs.last(), segs.first()) else { continue };
            let t_end = last.tangent(1.0);
            let t_start = first.tangent(0.0);
            let (Some(l_last), Some(l_first), Some(r_last), Some(r_first)) = (left.last(), left.first(), right.last(), right.first()) else { continue };
            let mut ring = left.clone();
            ring.extend(cap_segments(l_last.p3, r_first.p0, t_end, h, cap));
            ring.extend(right.iter().copied());
            ring.extend(cap_segments(r_last.p3, l_first.p0, Vec2::new(-t_start.x, -t_start.y), h, cap));
            rings.push(oriented_positive(&segs_to_contour(&ring)));
        }
    }
    merge_collinear(&positive_union(rings), 1e-6)
}

/// Pfad verkleinern (`expand = false`) bzw. erweitern um `d` Pixel (`pathInset`).
pub fn inset_path(contours: &[Contour], d: f64, join: JoinKind, miter_limit: f64, expand: bool, opts: &OffsetOptions) -> Vec<Contour> {
    let miter = clamp_miter_limit(miter_limit, OffsetUse::InsetPath);
    let region = boolean(&Region::new(contours.to_vec(), FillRule::EvenOdd), &Region::empty(), BoolOp::Union, GRID);
    let sgn = if expand { -1.0 } else { 1.0 };
    let rings: Vec<Contour> = region.iter().filter_map(|c| offset_contour(c, sgn * d.abs(), join, miter, opts)).map(|o| segs_to_contour(&o)).collect();
    positive_union(rings)
}

#[cfg(test)]
mod tests {
    use super::super::geom::area;
    use super::*;

    fn sq(x0: f64, y0: f64, x1: f64, y1: f64) -> Contour {
        Contour::polygon(&[Vec2::new(x0, y0), Vec2::new(x1, y0), Vec2::new(x1, y1), Vec2::new(x0, y1)], true)
    }

    #[test]
    fn expand_and_inset() {
        let line = Contour::polygon(&[Vec2::new(40.0, 100.0), Vec2::new(160.0, 100.0)], false);
        let r = expand_stroke(&[line], 10.0, JoinKind::Miter, 10.0, CapKind::Butt, &OffsetOptions::REFERENCE);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].nodes.len(), 4);
        assert!((area(&r, 0.01).abs() - 1200.0).abs() < 1e-6);
        let ring = expand_stroke(&[sq(40.0, 130.0, 160.0, 170.0)], 10.0, JoinKind::Miter, 10.0, CapKind::Butt, &OffsetOptions::REFERENCE);
        assert_eq!(ring.len(), 2);
        let a: f64 = ring.iter().map(|c| area(std::slice::from_ref(c), 0.01).abs()).sum();
        assert!((a - (6500.0 + 3300.0)).abs() < 1e-6, "{a}");
        let s = inset_path(&[sq(0.0, 0.0, 20.0, 20.0)], 5.0, JoinKind::Miter, 4.0, false, &OffsetOptions::REFERENCE);
        assert!((area(&s, 0.01).abs() - 100.0).abs() < 1e-6);
        let g = inset_path(&[sq(0.0, 0.0, 20.0, 20.0)], 5.0, JoinKind::Miter, 4.0, true, &OffsetOptions::REFERENCE);
        assert!((area(&g, 0.01).abs() - 900.0).abs() < 1e-6);
        let gone = inset_path(&[sq(0.0, 0.0, 20.0, 20.0)], 15.0, JoinKind::Miter, 4.0, false, &OffsetOptions::REFERENCE);
        assert!(area(&gone, 0.01).abs() < 1e-6);
    }
}
