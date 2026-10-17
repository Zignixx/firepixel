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
//! Vorstufe (re 30 §2.1): Extrem-Vereinfachung (x, y, mehrstufig) und Refit mit Toleranz =
//! Strichbreite bzw. Abstand, 30°, 16 Proben ([`super::refit::prepare_for_offset`]); die Befehle
//! rufen [`expand_stroke_refit`] und [`inset_path_refit`]. [`expand_stroke`]/[`inset_path`] sind
//! der reine Versatz (Golden-Tests gegen `re\pfade\offset.py`).
//! Abweichung von python.dll (A, wie die Referenz): runde Ecken als echter Kreisbogen (mit
//! [`OffsetOptions::ORIGINAL`] die 0,139-Formel).

use super::boolean::{BoolOp, FillRule, GRID, Region, boolean, merge_collinear};
use super::geom::{Contour, Node, flatten, signed_area};
use fp_geom::offset::{
    CapKind, JoinKind, OffsetOptions, OffsetUse, cap_segments, clamp_miter_limit, join_segments,
    offset_segment,
};
use fp_geom::{CubicBez, Vec2};

/// Einseitiger Offset einer Kontur (links positiv), ungereinigt: Segment-Offsets und Ecken.
pub fn offset_contour(
    c: &Contour,
    d: f64,
    join: JoinKind,
    miter_limit: f64,
    opts: &OffsetOptions,
) -> Option<Vec<CubicBez>> {
    let segs: Vec<CubicBez> = c
        .segments()
        .into_iter()
        .filter(|s| !s.is_degenerate())
        .collect();
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
            let (Some(last), Some(first)) = (pieces[k].last(), pieces[nk].first()) else {
                continue;
            };
            out.extend(join_segments(
                segs[k].p3,
                last.p3,
                segs[k].tangent(1.0),
                first.p0,
                segs[nk].tangent(0.0),
                d,
                join,
                miter_limit,
                opts,
            ));
        }
    }
    Some(out)
}

/// Segmentfolge → geschlossene Kontur.
pub fn segs_to_contour(segs: &[CubicBez]) -> Contour {
    let n = segs.len();
    Contour::new(
        (0..n)
            .map(|k| Node::new(segs[k].p0, segs[(k + n - 1) % n].p2, segs[k].p1))
            .collect(),
        true,
    )
}

fn oriented_positive(c: &Contour) -> Contour {
    let f = flatten(std::slice::from_ref(c), 0.05);
    let a = f.first().map(|(p, _)| signed_area(p)).unwrap_or(0.0);
    if a >= 0.0 { c.clone() } else { c.reversed() }
}

/// Bereinigung mit Regel „Windung > 0“ (`ApuCleanAfterExpand`).
pub fn positive_union(contours: Vec<Contour>) -> Vec<Contour> {
    boolean(
        &Region::new(contours, FillRule::Positive),
        &Region::empty(),
        BoolOp::Union,
        GRID,
    )
}

/// Strich in Fläche umwandeln (`pathExpand`). `width` = Gesamtbreite.
pub fn expand_stroke(
    contours: &[Contour],
    width: f64,
    join: JoinKind,
    miter_limit: f64,
    cap: CapKind,
    opts: &OffsetOptions,
) -> Vec<Contour> {
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
            let (Some(left), Some(right)) = (
                offset_contour(c, h, join, miter, opts),
                offset_contour(&c.reversed(), h, join, miter, opts),
            ) else {
                continue;
            };
            let segs = c.segments();
            let (Some(last), Some(first)) = (segs.last(), segs.first()) else {
                continue;
            };
            let t_end = last.tangent(1.0);
            let t_start = first.tangent(0.0);
            let (Some(l_last), Some(l_first), Some(r_last), Some(r_first)) =
                (left.last(), left.first(), right.last(), right.first())
            else {
                continue;
            };
            let mut ring = left.clone();
            ring.extend(cap_segments(l_last.p3, r_first.p0, t_end, h, cap));
            ring.extend(right.iter().copied());
            ring.extend(cap_segments(
                r_last.p3,
                l_first.p0,
                Vec2::new(-t_start.x, -t_start.y),
                h,
                cap,
            ));
            rings.push(oriented_positive(&segs_to_contour(&ring)));
        }
    }
    merge_collinear(&positive_union(rings), 1e-6)
}

/// Strich erweitern mit der Vorstufe des Originals (re 30 §2.1): Vereinfachen an Extrema, Refit
/// mit Toleranz = Breite, dann [`expand_stroke`].
pub fn expand_stroke_refit(
    contours: &[Contour],
    width: f64,
    join: JoinKind,
    miter_limit: f64,
    cap: CapKind,
    opts: &OffsetOptions,
) -> Vec<Contour> {
    expand_stroke(
        &super::refit::prepare_for_offset(contours, width.abs()),
        width,
        join,
        miter_limit,
        cap,
        opts,
    )
}

/// Signed area of a closed contour (flattened at 0.05 px).
fn contour_area(c: &Contour) -> f64 {
    flatten(std::slice::from_ref(c), 0.05)
        .first()
        .map(|(p, _)| signed_area(p))
        .unwrap_or(0.0)
}

/// Step 0 of inset path (re 34 §1.2): every contour is closed; a closing line is appended when the
/// end differs from the start, otherwise the end node is merged into the start node.
fn close_contour(c: &Contour) -> Option<Contour> {
    let mut nodes = c.nodes.clone();
    if nodes.len() < 2 {
        return None;
    }
    if !c.closed {
        let (first, last) = (nodes[0], nodes[nodes.len() - 1]);
        if first.anchor == last.anchor {
            nodes.pop();
            nodes[0].ctrl_in = last.ctrl_in;
        } else {
            let k = nodes.len() - 1;
            nodes[k].ctrl_out = last.anchor;
            nodes[0].ctrl_in = first.anchor;
        }
    }
    (nodes.len() >= 2).then(|| Contour::new(nodes, true))
}

/// Offset of a closed segment ring like [`offset_contour`], but inner corners are cut as in the
/// original (re 16 §3.3 "Innenecke: Offset-Kurven schneiden und beschneiden", re 34 §1.5 c): the
/// last offset piece of a segment and the first one of the next are trimmed at their intersection
/// instead of being joined through the source vertex; the cut point is snapped to the 0.005 grid
/// (P1 inset 1: 304.16/217.92 already in stage 5). Ends closer than two grid steps meet in their
/// middle (P1 inset 1 at the cusp 290/200: 289.4172/200.5721 instead of a pair 0.0015 px apart).
/// Without an intersection the join stays as before (K-4, checked against `re\re34\daten\stufen.txt`).
fn offset_contour_trimmed(
    segs: &[CubicBez],
    d: f64,
    join: JoinKind,
    miter: f64,
    opts: &OffsetOptions,
) -> Option<Vec<CubicBez>> {
    use super::boolean::intersections;
    if segs.is_empty() {
        return None;
    }
    let mut pieces: Vec<Vec<CubicBez>> = segs.iter().map(|s| offset_segment_inset(s, d)).collect();
    let n = segs.len();
    let mut joins: Vec<Vec<CubicBez>> = vec![Vec::new(); n];
    for k in 0..n {
        let nk = (k + 1) % n;
        let (Some(&prev), Some(&next)) = (pieces[k].last(), pieces[nk].first()) else {
            continue;
        };
        let (t_prev, t_next) = (segs[k].tangent(1.0), segs[nk].tangent(0.0));
        let cross = t_prev.x * t_next.y - t_prev.y * t_next.x;
        let inner = cross * d >= 0.0;
        let gap = prev.p3.distance(next.p0);
        if inner && gap < 2.0 * GRID {
            // ends within the grid: both meet in the middle (P1 inset 1: 289.4172/200.5721)
            let p = (prev.p3 + next.p0) * 0.5;
            let last = pieces[k].len() - 1;
            pieces[k][last].p3 = p;
            pieces[nk][0].p0 = p;
            continue;
        }
        if inner && !(n == 1 && pieces[k].len() == 1) {
            let cut = intersections(&prev, &next)
                .into_iter()
                .filter_map(|(u, v)| Some((u?, v?)))
                .filter(|&(u, v)| u > 1e-9 && v < 1.0 - 1e-9)
                .max_by(|a, b| a.0.total_cmp(&b.0));
            if let Some((u, v)) = cut {
                // the cut point lies on the 0.005 grid like the kernel's intersections
                let q = prev.eval(u);
                let p = Vec2::new((q.x / GRID).round() * GRID, (q.y / GRID).round() * GRID);
                let mut a = prev.subsegment(0.0, u);
                let mut b = next.subsegment(v, 1.0);
                a.p3 = p;
                b.p0 = p;
                let last = pieces[k].len() - 1;
                pieces[k][last] = a;
                pieces[nk][0] = b;
                continue;
            }
        }
        joins[k] = join_segments(
            segs[k].p3, prev.p3, t_prev, next.p0, t_next, d, join, miter, opts,
        );
    }
    let mut out = Vec::new();
    for k in 0..n {
        out.extend_from_slice(&pieces[k]);
        out.extend_from_slice(&joins[k]);
    }
    Some(out)
}

/// One-sided offset of one closed contour in the orientation of the original (outer contours with
/// negative area, `d < 0` = inwards): drops shrinking contours whose bounds are smaller than
/// `|2ε + 2d|` (re 34 §1.5 b) and starts the result at the end of the first offset piece (§1.5 c).
fn expand_closed_contour(
    c: &Contour,
    d: f64,
    join: JoinKind,
    miter: f64,
    opts: &OffsetOptions,
) -> Option<Contour> {
    if c.nodes.len() < 2 {
        return None;
    }
    let a = contour_area(c);
    if (a < 0.0 && d < 0.0) || (a > 0.0 && d > 0.0) {
        let (mut lo, mut hi) = (Vec2::new(f64::MAX, f64::MAX), Vec2::new(f64::MIN, f64::MIN));
        for n in &c.nodes {
            for p in [n.anchor, n.ctrl_in, n.ctrl_out] {
                lo = Vec2::new(lo.x.min(p.x), lo.y.min(p.y));
                hi = Vec2::new(hi.x.max(p.x), hi.y.max(p.y));
            }
        }
        let lim = (2.0 * GRID + 2.0 * d).abs();
        if hi.x - lo.x < lim || hi.y - lo.y < lim {
            return None;
        }
    }
    let segs: Vec<CubicBez> = c
        .segments()
        .into_iter()
        .filter(|s| !s.is_degenerate())
        .collect();
    let (first, last) = (segs.first()?, segs.last()?);
    let n_first = offset_segment_inset(first, d).len();
    let mut out = offset_contour_trimmed(&segs, d, join, miter, opts)?;
    // Glatter Konturstart: Beginn am Ende des ersten Versatzstücks. An einer Ecke beginnt die
    // Kontur am (beschnittenen) Anfang des ersten Stücks; den Beschnitt erledigt Schritt 6 (A, an
    // P1 geprüft: Stufe 5 des Originals beginnt am Eckschnittpunkt).
    let (t_in, t_out) = (last.tangent(1.0), first.tangent(0.0));
    if (t_in - t_out).length() < 1e-6 {
        let k = n_first.min(out.len());
        out.rotate_left(k);
    }
    // Winzige Eckstücke (fast tangentiale Ecken, Abstand unter dem Raster) entfallen; das folgende
    // Stück beginnt dann am Anfang des entfallenen.
    let tiny = |s: &CubicBez| {
        [s.p1, s.p2, s.p3]
            .iter()
            .all(|p| (*p - s.p0).length() < 2.0 * GRID)
    };
    let mut kept: Vec<CubicBez> = Vec::with_capacity(out.len());
    let mut carry: Option<Vec2> = None;
    for s in out {
        if tiny(&s) {
            carry.get_or_insert(s.p0);
            continue;
        }
        let mut s = s;
        if let Some(p) = carry.take() {
            let dlt = p - s.p0;
            s.p1 = if s.p1 == s.p0 { p } else { s.p1 + dlt };
            s.p0 = p;
        }
        kept.push(s);
    }
    if let (Some(p), Some(f)) = (carry, kept.first().copied()) {
        let dlt = p - f.p0;
        let k = kept.len() - 1;
        kept[k].p3 += dlt;
        kept[k].p2 += dlt;
        let _ = f;
    }
    if kept.len() < 2 {
        return None;
    }
    Some(segs_to_contour(&kept))
}

/// Pfad einlassen (`pathInset`) wie das Original in sieben Schritten (re 34 §1.2): Konturen
/// schließen, Umlaufsinn nach Even-Odd und Bereinigung, Vereinfachen an den Extrema, Refit mit der
/// vorzeichenbehafteten Breite (innen Toleranz 0 = Zerlegung in Probenstücke), einseitiger Versatz
/// je Kontur, Bereinigung „Windung > 0“, Nachlauf-Refit (2 px, 30°, 16). `d` ist der Betrag,
/// `expand` die Richtung (außen).
pub fn inset_path_refit(
    contours: &[Contour],
    d: f64,
    join: JoinKind,
    miter_limit: f64,
    expand: bool,
    opts: &OffsetOptions,
) -> Vec<Contour> {
    let [.., last] = inset_path_stages(contours, d, join, miter_limit, expand, opts);
    last
}

/// The seven intermediate results of [`inset_path_refit`] (steps 1 + 2 together as the first two
/// entries: closed input and Even-Odd cleaning; then simplify, refit, offset, cleaning, final
/// refit), for comparisons with the stages of the original.
pub fn inset_path_stages(
    contours: &[Contour],
    d: f64,
    join: JoinKind,
    miter_limit: f64,
    expand: bool,
    opts: &OffsetOptions,
) -> [Vec<Contour>; 7] {
    use super::refit::{refit_contour, simplify_extrema};
    let width = if expand { d.abs() } else { -d.abs() };
    let miter = clamp_miter_limit(miter_limit, OffsetUse::InsetPath);
    // 0: schließen
    let closed: Vec<Contour> = contours.iter().filter_map(close_contour).collect();
    // 1 + 2: Even-Odd-Bereinigung, Außenkonturen mit negativer Fläche (Orientierung des Originals)
    let mut clean = boolean(
        &Region::new(closed.clone(), FillRule::EvenOdd),
        &Region::empty(),
        BoolOp::Union,
        GRID,
    );
    let outer_positive = clean
        .iter()
        .map(contour_area)
        .max_by(|a, b| a.abs().total_cmp(&b.abs()))
        .is_some_and(|a| a > 0.0);
    if outer_positive {
        clean = clean.iter().map(Contour::reversed).collect();
    }
    // 3: Extrema (1, 1, 1)
    let simple: Vec<Contour> = clean.iter().map(simplify_extrema).collect();
    // 4: Refit mit vorzeichenbehafteter Breite (≤ 0 ⇒ Toleranz 0 = Zerlegung)
    let fitted: Vec<Contour> = simple
        .iter()
        .map(|c| refit_contour(c, width.max(0.0), 30.0, 16))
        .collect();
    // 5: einseitiger Versatz
    let rings: Vec<Contour> = fitted
        .iter()
        .filter_map(|c| expand_closed_contour(c, width, join, miter, opts))
        .collect();
    // 6: Windung > 0 (die eigene Regel zählt positive Fläche, daher gedreht hin und zurück)
    // zu kleine Teilschleifen (Restschleifen an Ecken) entfallen (§1.5 d)
    let merged: Vec<Contour> = positive_union(rings.iter().map(Contour::reversed).collect())
        .iter()
        .filter(|c| c.nodes.len() >= 2 && contour_area(c).abs() > GRID)
        .map(Contour::reversed)
        .collect();
    // 7: Nachlauf-Refit
    let last: Vec<Contour> = merged
        .iter()
        .map(|c| refit_contour(c, 2.0, 30.0, 16))
        .collect();
    [closed, clean, simple, fitted, rings, merged, last]
}

/// Pfad verkleinern (`expand = false`) bzw. erweitern um `d` Pixel (`pathInset`).
pub fn inset_path(
    contours: &[Contour],
    d: f64,
    join: JoinKind,
    miter_limit: f64,
    expand: bool,
    opts: &OffsetOptions,
) -> Vec<Contour> {
    let miter = clamp_miter_limit(miter_limit, OffsetUse::InsetPath);
    let region = boolean(
        &Region::new(contours.to_vec(), FillRule::EvenOdd),
        &Region::empty(),
        BoolOp::Union,
        GRID,
    );
    let sgn = if expand { -1.0 } else { 1.0 };
    let rings: Vec<Contour> = region
        .iter()
        .filter_map(|c| offset_contour(c, sgn * d.abs(), join, miter, opts))
        .map(|o| segs_to_contour(&o))
        .collect();
    positive_union(rings)
}

/// Sign of `cross(u, v) = v_x·u_y − u_x·v_y` (re 39 §1.2, exact comparison with 0).
fn cross_sign(u: Vec2, v: Vec2) -> i32 {
    let c = v.x * u.y - u.x * v.y;
    if c > 0.0 {
        1
    } else if c < 0.0 {
        -1
    } else {
        0
    }
}

/// "Almost straight" test of a source piece (re 39 §1.2 step 1): main axis monotonic and both
/// handles inside a wedge of ±(chord/8) around the chord.
fn almost_straight(seg: &CubicBez) -> bool {
    let mut p = [seg.p0, seg.p1, seg.p2, seg.p3];
    if (p[3].y - p[0].y).abs() < (p[3].x - p[0].x).abs() {
        for q in &mut p {
            *q = Vec2::new(q.y, q.x);
        }
    }
    if p[3].y < p[0].y {
        for q in &mut p {
            *q = Vec2::new(-q.x, -q.y);
        }
    }
    let [a, b, c, e] = p;
    if !(a.y <= b.y && b.y <= c.y && c.y <= e.y) {
        return false;
    }
    let f = -(e.y - a.y) / 8.0;
    let s1 = cross_sign(Vec2::new(e.x + f - a.x, e.y - a.y), b - a);
    let s2 = cross_sign(Vec2::new(e.x - f - a.x, e.y - a.y), b - a);
    let s3 = cross_sign(Vec2::new(a.x + f - e.x, a.y - e.y), c - e);
    let s4 = cross_sign(Vec2::new(a.x - f - e.x, a.y - e.y), c - e);
    s1 != -1 && s2 != 1 && s3 != 1 && s4 != -1
}

/// Normal `N(t) = (−t_y, t_x)/|t|` (zero for a zero vector).
fn unit_normal(t: Vec2) -> Vec2 {
    let l = t.length();
    if l > 0.0 {
        Vec2::new(-t.y / l, t.x / l)
    } else {
        Vec2::new(0.0, 0.0)
    }
}

/// Convex control polygon: among the four turn signs never both +1 and −1.
fn convex_polygon(q: [Vec2; 4]) -> bool {
    let mut pos = false;
    let mut neg = false;
    for i in 0..4 {
        let e0 = q[(i + 1) % 4] - q[i];
        let e1 = q[(i + 2) % 4] - q[(i + 1) % 4];
        match cross_sign(e0, e1) {
            1 => pos = true,
            -1 => neg = true,
            _ => {}
        }
    }
    !(pos && neg)
}

/// Offset of one source piece for inset path (re 39 §1.2 steps 1–4). Returns the piece and
/// whether it is "inexact" (nearly parallel end tangents, midpoint fit rejected or not convex).
/// Almost straight pieces are always exact.
pub fn offset_piece(seg: &CubicBez, d: f64) -> (CubicBez, bool) {
    let (p0, p1, p2, p3) = (seg.p0, seg.p1, seg.p2, seg.p3);
    let b = if p1 != p0 { p1 - p0 } else { p2 - p0 };
    let a = if p3 != p2 { p3 - p2 } else { p3 - p1 };
    let q0 = p0 + unit_normal(b) * d;
    let q3 = p3 + unit_normal(a) * d;
    // simple offset: end points along the normals, handle directions kept, lengths scaled
    let simple = || {
        let c0 = p0.distance(p3);
        let s = if c0 > 0.0 { q0.distance(q3) / c0 } else { 1.0 };
        CubicBez::new(q0, q0 + (p1 - p0) * s, q3 + (p2 - p3) * s, q3)
    };
    if seg.is_line() || almost_straight(seg) {
        return (simple(), false);
    }
    let cr = a.y * b.x - a.x * b.y;
    if cr.abs() <= (a.x.abs() + a.y.abs() + b.x.abs() + b.y.abs()) / 1000.0 {
        return (simple(), true);
    }
    let m = seg.eval(0.5);
    let dm = seg.deriv(0.5);
    let mp = m + unit_normal(dm) * d;
    let f = (mp - (q0 + q3) * 0.5) * (8.0 / 3.0);
    let alpha = (-a.y * f.x + a.x * f.y) / -cr;
    let beta = (b.x * f.y - b.y * f.x) / -cr;
    let accept = alpha >= 0.0
        && beta >= 0.0
        && (alpha <= 1.05 || beta >= 0.95)
        && (alpha >= 0.95 || beta <= 1.05);
    if accept {
        let cand = [q0, q0 + b * alpha, q3 - a * beta, q3];
        if convex_polygon(cand) {
            return (CubicBez::new(cand[0], cand[1], cand[2], cand[3]), false);
        }
    }
    (simple(), true)
}

/// Offset of one source segment for inset path with the split loop of the original
/// (re 39 §1.2 step 5): while the piece is inexact and its chord is longer than |d|/2, it is split
/// at t = 0.5, the first half is offset again and the second half follows as the next piece.
/// The part tolerance max(0.025; |d|/10) does not enter this decision.
pub fn offset_segment_inset(seg: &CubicBez, d: f64) -> Vec<CubicBez> {
    let mut out = Vec::new();
    let mut queue: Vec<(CubicBez, u32)> = vec![(*seg, 0)];
    while let Some((s, depth)) = queue.pop() {
        let (piece, inexact) = offset_piece(&s, d);
        if inexact && depth < 24 && s.p0.distance(s.p3) > d.abs() * 0.5 {
            let (first, second) = s.split(0.5);
            queue.push((second, depth + 1));
            queue.push((first, depth + 1));
            continue;
        }
        out.push(piece);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::geom::area;
    use super::*;

    fn sq(x0: f64, y0: f64, x1: f64, y1: f64) -> Contour {
        Contour::polygon(
            &[
                Vec2::new(x0, y0),
                Vec2::new(x1, y0),
                Vec2::new(x1, y1),
                Vec2::new(x0, y1),
            ],
            true,
        )
    }

    #[test]
    fn expand_and_inset() {
        let line = Contour::polygon(&[Vec2::new(40.0, 100.0), Vec2::new(160.0, 100.0)], false);
        let r = expand_stroke(
            &[line],
            10.0,
            JoinKind::Miter,
            10.0,
            CapKind::Butt,
            &OffsetOptions::REFERENCE,
        );
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].nodes.len(), 4);
        assert!((area(&r, 0.01).abs() - 1200.0).abs() < 1e-6);
        let ring = expand_stroke(
            &[sq(40.0, 130.0, 160.0, 170.0)],
            10.0,
            JoinKind::Miter,
            10.0,
            CapKind::Butt,
            &OffsetOptions::REFERENCE,
        );
        assert_eq!(ring.len(), 2);
        let a: f64 = ring
            .iter()
            .map(|c| area(std::slice::from_ref(c), 0.01).abs())
            .sum();
        assert!((a - (6500.0 + 3300.0)).abs() < 1e-6, "{a}");
        let s = inset_path(
            &[sq(0.0, 0.0, 20.0, 20.0)],
            5.0,
            JoinKind::Miter,
            4.0,
            false,
            &OffsetOptions::REFERENCE,
        );
        assert!((area(&s, 0.01).abs() - 100.0).abs() < 1e-6);
        let g = inset_path(
            &[sq(0.0, 0.0, 20.0, 20.0)],
            5.0,
            JoinKind::Miter,
            4.0,
            true,
            &OffsetOptions::REFERENCE,
        );
        assert!((area(&g, 0.01).abs() - 900.0).abs() < 1e-6);
        let gone = inset_path(
            &[sq(0.0, 0.0, 20.0, 20.0)],
            15.0,
            JoinKind::Miter,
            4.0,
            false,
            &OffsetOptions::REFERENCE,
        );
        assert!(area(&gone, 0.01).abs() < 1e-6);
    }
}
