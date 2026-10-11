//! Offset-Grundlagen für „Strich erweitern“ und „Pfad einlassen“ (Spezifikation 16 §3.3/§3.4,
//! Referenz `re\pfade\offset.py`). Nur die reine Geometrie; Bereinigung (Vereinigung mit Regel
//! „Windung > 0“) und Refit gehören zu den Pfadoperationen in `fp-vector`.
//!
//! - **Segment-Offset** ([`offset_segment`]): Endpunkte entlang der Normalen um `d` versetzen
//!   (links = positiv im mathematischen Sinn), Henkelrichtungen = Endtangenten, Henkellängen so,
//!   dass die Offset-Kubik durch den versetzten Kurvenmittelpunkt läuft (2×2-System, B). Ist das
//!   nicht lösbar, Rückfall auf Henkel im Sehnenverhältnis. Ist der Fehler bei t = ¼, ¾ größer als
//!   max(0,025; |d|/10), wird das Quellsegment bei t = 0,5 geteilt.
//! - **Teilungsabbruch** (B, Lückenschluss): geteilt wird nur, solange die Sehne |P0P3| des
//!   Quellstücks > |d|/2 ist ([`OffsetOptions::min_chord_factor`] = 0,5). Die Python-Referenz
//!   verwendet abweichend |d|/2·0,01 (Faktor 0,005); die Golden-Tests rechnen mit diesem Wert.
//! - **Gehrung** ([`miter_join`], B): Spitze = Schnitt der Offset-Tangenten; Gehrung nur, solange
//!   der Abstand **unversetzter Eckpunkt → Spitze** kleiner als |d|·Grenze ist, sonst Bevel.
//! - **Runde Ecke** im Original: eine Kubik mit Henkellänge c·(4f² + f)·0,139, c = Abstand der
//!   Offset-Enden, f = c/|2d| ([`round_join_original`], B); die Referenz nimmt einen echten
//!   Kreisbogen ([`arc_segments`]).
//! - **Kappen** ([`cap_segments`]): Butt = Gerade, Square = um h verlängert, Round = eine Kubik mit
//!   beiden Henkeln 4/3·h in Tangentenrichtung (B).
//! - Gehrungsgrenzen: Strich erweitern [1; 10], Pfad einlassen [4; 10] ([`clamp_miter_limit`]).

use crate::bezier::CubicBez;
use crate::contour::BezierContour;
use crate::Vec2;

/// Eckenart (`ExpandJoinType`/`InsetJoinType`; Tabelle `@01748610`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum JoinKind {
    /// 0 = abgeflacht.
    #[default]
    Bevel = 0,
    /// 1 = rund.
    Round = 1,
    /// 2 = Gehrung.
    Miter = 2,
}

impl JoinKind {
    /// Aus dem Zahlenwert; unbekannte Werte → Bevel.
    pub fn from_u32(v: u32) -> JoinKind {
        match v {
            1 => JoinKind::Round,
            2 => JoinKind::Miter,
            _ => JoinKind::Bevel,
        }
    }
}

/// Endkappe (`ExpandCapType`; Tabelle `@017486b0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum CapKind {
    /// 0 = ohne.
    #[default]
    Butt = 0,
    /// 1 = eckig (um h verlängert).
    Square = 1,
    /// 2 = rund.
    Round = 2,
}

impl CapKind {
    /// Aus dem Zahlenwert; unbekannte Werte → Butt.
    pub fn from_u32(v: u32) -> CapKind {
        match v {
            1 => CapKind::Square,
            2 => CapKind::Round,
            _ => CapKind::Butt,
        }
    }
}

/// Anwendungsfall für die Begrenzung der Gehrungsgrenze.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffsetUse {
    /// Strich erweitern (`ApuExpandStroke`): Grenze in [1; 10].
    ExpandStroke,
    /// Pfad einlassen (`ApuExpandPath`): Grenze in [4; 10].
    InsetPath,
}

/// Begrenzt die Gehrungsgrenze wie `python.dll`.
pub fn clamp_miter_limit(limit: f64, use_: OffsetUse) -> f64 {
    let lo = match use_ {
        OffsetUse::ExpandStroke => 1.0,
        OffsetUse::InsetPath => 4.0,
    };
    if limit.is_nan() { lo } else { limit.clamp(lo, 10.0) }
}

/// Einstellungen des Segment-Offsets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OffsetOptions {
    /// Geteilt wird nur, solange |P0P3| > `min_chord_factor`·|d| ist (Original 0,5; Referenz 0,005).
    pub min_chord_factor: f64,
    /// Höchste Teilungstiefe (Referenz 8).
    pub max_depth: u32,
    /// Runde Ecken mit der Original-Formel (0,139) statt mit einem echten Kreisbogen.
    pub original_round_join: bool,
}

impl Default for OffsetOptions {
    fn default() -> Self {
        Self::ORIGINAL
    }
}

impl OffsetOptions {
    /// Werte laut Spezifikation 16 (B).
    pub const ORIGINAL: OffsetOptions = OffsetOptions { min_chord_factor: 0.5, max_depth: 8, original_round_join: true };
    /// Werte der Python-Referenz `offset.py` (für Golden-Tests).
    pub const REFERENCE: OffsetOptions = OffsetOptions { min_chord_factor: 0.005, max_depth: 8, original_round_join: false };
}

/// Fehlertoleranz des Segment-Offsets: max(0,025; |d|/10) (A).
pub fn offset_tolerance(d: f64) -> f64 {
    0.025f64.max(d.abs() / 10.0)
}

fn off(p: Vec2, t: Vec2, d: f64) -> Vec2 {
    p + t.perp() * d
}

/// Offset eines Quellsegments um `d` (links positiv) als Liste kubischer Segmente.
pub fn offset_segment(seg: &CubicBez, d: f64, opts: &OffsetOptions) -> Vec<CubicBez> {
    let mut out = Vec::new();
    offset_segment_rec(seg, d, offset_tolerance(d), 0, opts, &mut out);
    out
}

fn offset_segment_rec(seg: &CubicBez, d: f64, tol: f64, depth: u32, opts: &OffsetOptions, out: &mut Vec<CubicBez>) {
    let (p0, p1, p2, p3) = (seg.p0, seg.p1, seg.p2, seg.p3);
    let t0 = seg.tangent(0.0);
    let t1 = seg.tangent(1.0);
    let q0 = off(p0, t0, d);
    let q3 = off(p3, t1, d);
    if seg.is_line() {
        out.push(CubicBez::line(q0, q3));
        return;
    }
    let tm = seg.tangent(0.5);
    let m = off(seg.eval(0.5), tm, d);
    let rx = (m.x - (q0.x + q3.x) / 2.0) * 8.0 / 3.0;
    let ry = (m.y - (q0.y + q3.y) / 2.0) * 8.0 / 3.0;
    // alpha·t0 − beta·t1 = r
    let det = t0.x * (-t1.y) - t0.y * (-t1.x);
    let mut ok = det.abs() > 1e-3 * (t0.x.abs() + t0.y.abs() + t1.x.abs() + t1.y.abs());
    let (mut alpha, mut beta) = (0.0, 0.0);
    if ok {
        alpha = (rx * (-t1.y) - ry * (-t1.x)) / det;
        beta = (t0.x * ry - t0.y * rx) / det;
        ok = alpha >= 0.0 && beta >= 0.0;
    }
    if !ok {
        let mut c0 = p0.distance(p3);
        if c0 == 0.0 {
            c0 = 1.0;
        }
        let s = q0.distance(q3) / c0;
        alpha = p0.distance(p1) * s;
        beta = p2.distance(p3) * s;
    }
    let cand = CubicBez::new(q0, q0 + t0 * alpha, q3 + t1 * (-beta), q3);
    let mut err: f64 = 0.0;
    for t in [0.25, 0.75] {
        let truth = off(seg.eval(t), seg.tangent(t), d);
        err = err.max(cand.nearest(truth, 8).distance);
    }
    if err > tol && depth < opts.max_depth && p0.distance(p3) > d.abs() * opts.min_chord_factor {
        let (a, b) = seg.split(0.5);
        offset_segment_rec(&a, d, tol, depth + 1, opts, out);
        offset_segment_rec(&b, d, tol, depth + 1, opts, out);
        return;
    }
    out.push(cand);
}

/// Schnitt der beiden Offset-Tangenten: Spitze `end_prev + u·t_prev`; `None` bei parallelen
/// Tangenten. Rückgabe `(Spitze, u)`.
pub fn miter_tip(end_prev: Vec2, t_prev: Vec2, start_next: Vec2, t_next: Vec2) -> Option<(Vec2, f64)> {
    let den = t_prev.x * t_next.y - t_prev.y * t_next.x;
    if den.abs() <= 1e-12 {
        return None;
    }
    let qx = start_next.x - end_prev.x;
    let qy = start_next.y - end_prev.y;
    let u = (qx * t_next.y - qy * t_next.x) / den;
    Some((end_prev + t_prev * u, u))
}

/// Gehrungsregel (16 §3.3, B): Spitze, wenn sie vor dem Offset-Ende liegt (`u ≥ 0`) und der Abstand
/// des **unversetzten** Eckpunkts `vertex` zur Spitze kleiner als |d|·`limit` ist; sonst `None`
/// (Bevel).
pub fn miter_join(
    vertex: Vec2,
    end_prev: Vec2,
    t_prev: Vec2,
    start_next: Vec2,
    t_next: Vec2,
    d: f64,
    limit: f64,
) -> Option<Vec2> {
    let (tip, u) = miter_tip(end_prev, t_prev, start_next, t_next)?;
    if u >= 0.0 && tip.distance(vertex) < limit * d.abs() { Some(tip) } else { None }
}

/// Runde Ecke wie das Original (`python.dll@100026b0`, B): eine Kubik von `end_prev` nach
/// `start_next`, Henkel in Richtung `t_prev` bzw. gegen `t_next`, Länge c·(4f² + f)·0,139 mit
/// c = |start_next − end_prev| und f = c/|2d|.
pub fn round_join_original(end_prev: Vec2, t_prev: Vec2, start_next: Vec2, t_next: Vec2, d: f64) -> CubicBez {
    let c = end_prev.distance(start_next);
    let f = if d != 0.0 { c / (2.0 * d).abs() } else { 0.0 };
    let h = c * (4.0 * f * f + f) * 0.139;
    CubicBez::new(end_prev, end_prev + t_prev * h, start_next - t_next * h, start_next)
}

/// Kreisbogen von `a` nach `b` um `center` mit Radius `r` in Stücken ≤ 90° (echter Bogen,
/// Henkel 4/3·tan(θ/4)·r). `ccw` = positiver Drehsinn (mathematisch).
pub fn arc_segments(center: Vec2, a: Vec2, b: Vec2, r: f64, ccw: bool) -> Vec<CubicBez> {
    use std::f64::consts::{FRAC_PI_2, PI};
    let a0 = (a.y - center.y).atan2(a.x - center.x);
    let a1 = (b.y - center.y).atan2(b.x - center.x);
    let mut sweep = a1 - a0;
    if ccw && sweep < 0.0 {
        sweep += 2.0 * PI;
    }
    if !ccw && sweep > 0.0 {
        sweep -= 2.0 * PI;
    }
    let n = ((sweep.abs() / FRAC_PI_2 - 1e-9).ceil() as i64).max(1) as usize;
    let step = sweep / n as f64;
    let k = 4.0 / 3.0 * (step / 4.0).tan() * r;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let s0 = a0 + step * i as f64;
        let s1 = a0 + step * (i + 1) as f64;
        let p0 = Vec2::new(center.x + r * s0.cos(), center.y + r * s0.sin());
        let p3 = Vec2::new(center.x + r * s1.cos(), center.y + r * s1.sin());
        let t0 = Vec2::new(-s0.sin(), s0.cos());
        let t1 = Vec2::new(-s1.sin(), s1.cos());
        out.push(CubicBez::new(p0, p0 + t0 * k, p3 + t1 * (-k), p3));
    }
    if let Some(f) = out.first_mut() {
        f.p0 = a;
    }
    if let Some(l) = out.last_mut() {
        l.p3 = b;
    }
    out
}

/// Verbindung zwischen dem Offset-Ende des vorigen und dem Offset-Anfang des nächsten Segments
/// (Referenz `_join`). Innenecken laufen als Schlaufe über den Originalknoten (die spätere
/// Bereinigung entfernt sie); Außenecken je nach `join`.
#[allow(clippy::too_many_arguments)]
pub fn join_segments(
    vertex: Vec2,
    end_prev: Vec2,
    t_prev: Vec2,
    start_next: Vec2,
    t_next: Vec2,
    d: f64,
    join: JoinKind,
    miter_limit: f64,
    opts: &OffsetOptions,
) -> Vec<CubicBez> {
    if end_prev.distance(start_next) < offset_tolerance(d) * 0.01 {
        return Vec::new();
    }
    let cross = t_prev.x * t_next.y - t_prev.y * t_next.x;
    let outer = cross * d < 0.0;
    if !outer {
        return vec![CubicBez::line(end_prev, vertex), CubicBez::line(vertex, start_next)];
    }
    match join {
        JoinKind::Round => {
            if opts.original_round_join {
                vec![round_join_original(end_prev, t_prev, start_next, t_next, d)]
            } else {
                arc_segments(vertex, end_prev, start_next, d.abs(), cross > 0.0)
            }
        }
        JoinKind::Miter => match miter_join(vertex, end_prev, t_prev, start_next, t_next, d, miter_limit) {
            Some(tip) => vec![CubicBez::line(end_prev, tip), CubicBez::line(tip, start_next)],
            None => vec![CubicBez::line(end_prev, start_next)],
        },
        JoinKind::Bevel => vec![CubicBez::line(end_prev, start_next)],
    }
}

/// Kappe vom Ende `a` der linken Seite zum Anfang `b` der rechten Seite (`t_end` = Endtangente,
/// `h` = halbe Breite).
pub fn cap_segments(a: Vec2, b: Vec2, t_end: Vec2, h: f64, cap: CapKind) -> Vec<CubicBez> {
    match cap {
        CapKind::Square => {
            let a2 = a + t_end * h;
            let b2 = b + t_end * h;
            vec![CubicBez::line(a, a2), CubicBez::line(a2, b2), CubicBez::line(b2, b)]
        }
        CapKind::Round => {
            let k = 4.0 / 3.0 * h;
            vec![CubicBez::new(a, a + t_end * k, b + t_end * k, b)]
        }
        CapKind::Butt => vec![CubicBez::line(a, b)],
    }
}

/// Einseitiger Offset einer Kontur (links positiv), ungereinigt: Segment-Offsets und Ecken in
/// Reihenfolge. Entartete Segmente werden übersprungen; `None`, wenn keines übrig bleibt.
pub fn offset_contour(c: &BezierContour, d: f64, join: JoinKind, miter_limit: f64, opts: &OffsetOptions) -> Option<Vec<CubicBez>> {
    let segs: Vec<CubicBez> = c.segments().filter(|s| !s.is_degenerate()).collect();
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

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: f64, y: f64) -> Vec2 {
        Vec2::new(x, y)
    }

    #[test]
    fn line_offset() {
        let s = CubicBez::line(v(0.0, 0.0), v(10.0, 0.0));
        let o = offset_segment(&s, 2.0, &OffsetOptions::ORIGINAL);
        assert_eq!(o, vec![CubicBez::line(v(0.0, 2.0), v(10.0, 2.0))]);
    }

    #[test]
    fn circle_offset_is_accurate() {
        // Viertelkreis Radius 10 mit κ, Offset nach außen (rechts vom Lauf) um 5
        let k = crate::ELLIPSE_KAPPA as f64 * 10.0;
        let s = CubicBez::new(v(10.0, 0.0), v(10.0, k), v(k, 10.0), v(0.0, 10.0));
        for opts in [OffsetOptions::ORIGINAL, OffsetOptions::REFERENCE] {
            let o = offset_segment(&s, -5.0, &opts);
            for seg in &o {
                for i in 0..=8 {
                    let p = seg.eval(i as f64 / 8.0);
                    assert!((p.length() - 15.0).abs() < 0.05, "{}", p.length());
                }
            }
        }
    }

    #[test]
    fn miter_rule() {
        // rechter Winkel, d = 1: Abstand Ecke → Spitze = √2 ≈ 1,414
        // Lauf nach rechts, dann nach unten (+y); Offset d = −1 liegt außen
        let vtx = v(10.0, 0.0);
        let (ep, tp) = (v(10.0, -1.0), v(1.0, 0.0));
        let (sn, tn) = (v(11.0, 0.0), v(0.0, 1.0));
        assert_eq!(miter_join(vtx, ep, tp, sn, tn, -1.0, 2.0), Some(v(11.0, -1.0)));
        assert_eq!(miter_join(vtx, ep, tp, sn, tn, -1.0, 1.4), None);
        let j = join_segments(vtx, ep, tp, sn, tn, -1.0, JoinKind::Miter, 2.0, &OffsetOptions::ORIGINAL);
        assert_eq!(j.len(), 2);
        let j = join_segments(vtx, ep, tp, sn, tn, 1.0, JoinKind::Miter, 2.0, &OffsetOptions::ORIGINAL);
        assert_eq!(j[0].p3, vtx); // Innenecke: Schlaufe über den Knoten
        assert_eq!(clamp_miter_limit(20.0, OffsetUse::ExpandStroke), 10.0);
        assert_eq!(clamp_miter_limit(2.0, OffsetUse::InsetPath), 4.0);
    }

    #[test]
    fn caps_and_round_join() {
        let c = cap_segments(v(0.0, 1.0), v(0.0, -1.0), v(1.0, 0.0), 1.0, CapKind::Square);
        assert_eq!(c.len(), 3);
        assert_eq!(c[1].p0, v(1.0, 1.0));
        let r = round_join_original(v(0.0, 1.0), v(1.0, 0.0), v(1.0, 0.0), v(0.0, -1.0), 1.0);
        // c = √2, f = √2/2 → h = √2·(2 + 0,7071)·0,139
        let h = 2f64.sqrt() * (4.0 * 0.5 + 2f64.sqrt() / 2.0) * 0.139;
        assert!((r.p1.x - h).abs() < 1e-12);
        let arc = arc_segments(v(0.0, 0.0), v(1.0, 0.0), v(0.0, 1.0), 1.0, true);
        assert_eq!(arc.len(), 1);
        assert!((arc[0].eval(0.5).length() - 1.0).abs() < 1e-3);
    }
}
