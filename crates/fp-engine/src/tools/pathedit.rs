//! Pfadbearbeitung auf der Leinwand: Messer, Freiform (Schieben/Ziehen) und Bereich umformen
//! (10 §2.6, Formeln 16 §3.7: Unterteilung, Kreisrand-Schieben, Gravitation `1 − (d/R)^p`).
//! Die Vorschau rechnet das Werkzeug selbst; beim Loslassen werden die Konturen übernommen.

use super::pointer::contours_json;
use super::{constrain_angle, dist, pt_json, Cursor, Invocation, Overlay, PointerKind, ToolContext, ToolEvent, ToolId, ToolInput, ToolKey, ToolMachine, ToolResponse};
use crate::docops;
use crate::geometry::{from_bezier, to_bezier};
use fp_geom::{BezierContour, BezierNode, CubicBez, Point, Vec2};
use fp_model::{Contour, ElementId, ElementKind};
use serde_json::{json, Value};

// ------------------------------------------------------------------ Geometrie

/// Segmente einer Kontur in Schritte von höchstens `step` px unterteilen (t-Schritt mindestens
/// 1/128, Teilpunkte bis 0,98; 16 §3.7).
pub fn subdivide(c: &Contour, step: f64) -> Contour {
    let bc = to_bezier(c);
    let segs: Vec<CubicBez> = bc.segments().collect();
    if segs.is_empty() || step <= 0.0 {
        return c.clone();
    }
    let mut pieces: Vec<CubicBez> = Vec::new();
    for s in &segs {
        let parts = (s.length() / step - 1e-9).ceil().max(1.0);
        let dt = (1.0 / parts).max(1.0 / 128.0);
        let mut cur = *s;
        let mut consumed = 0.0;
        let mut t = dt;
        while t <= 0.98 + 1e-12 {
            let (l, r) = cur.split((t - consumed) / (1.0 - consumed));
            pieces.push(l);
            cur = r;
            consumed = t;
            t += dt;
        }
        pieces.push(cur);
    }
    let m = pieces.len();
    let n0 = bc.nodes.len();
    let p = |v: Vec2| v.to_point();
    let mut nodes: Vec<BezierNode> = (0..m)
        .map(|k| {
            let cin = if k > 0 || bc.closed { p(pieces[(k + m - 1) % m].p2) } else { bc.nodes[0].ctrl_in };
            BezierNode::new(p(pieces[k].p0), cin, p(pieces[k].p1))
        })
        .collect();
    if !bc.closed {
        let last = pieces[m - 1];
        nodes.push(BezierNode::new(p(last.p3), p(last.p2), bc.nodes[n0 - 1].ctrl_out));
    }
    from_bezier(&BezierContour::new(nodes, bc.closed))
}

fn move_node(n: &mut fp_model::ContourNode, dx: f32, dy: f32) {
    for q in [&mut n.anchor, &mut n.pred, &mut n.succ] {
        q.x += dx;
        q.y += dy;
    }
}

/// Freiform „Schieben“: Knoten im Kreis (R = Größe/2) werden auf den Kreisrand geschoben.
pub fn push_pull(contours: &[Contour], cursor: Point, size: f64) -> Vec<Contour> {
    let size = size.clamp(1.0, 1000.0);
    let r = (size * 0.5).max(1.0) as f32;
    let step = size.clamp(0.5, 8.0);
    contours
        .iter()
        .map(|c| {
            let mut c = subdivide(c, step);
            for n in &mut c.nodes {
                let mut p = n.anchor;
                if p == cursor {
                    p.x += 0.0001;
                }
                let d = dist(p, cursor);
                if d <= r && d > 0.0 {
                    let np = Point::new(cursor.x - (cursor.x - p.x) * (r / d), cursor.y - (cursor.y - p.y) * (r / d));
                    let (dx, dy) = (np.x - n.anchor.x, np.y - n.anchor.y);
                    if dx.hypot(dy) >= 0.1 {
                        move_node(n, dx, dy);
                    }
                }
            }
            c
        })
        .collect()
}

/// Stärke der Gravitation aus `FreeFormGravPower` (Vorgabe 80 → 2,82).
pub fn gravity_power(pref: f64) -> f64 {
    (pref * 3.4 / 100.0 + 0.1).clamp(0.1, 3.5)
}

/// Bereich umformen: Knoten mit Abstand d < R zum Zentrum wandern um Δ·(1 − (d/R)^p).
pub fn reshape_area(contours: &[Contour], center: Point, delta: (f32, f32), diameter: f64, power: f64) -> Vec<Contour> {
    let r = (diameter.clamp(1.0, 1000.0) * 0.5).max(1.0);
    let step = (0.2 * r).clamp(0.5, 8.0);
    contours
        .iter()
        .map(|c| {
            let mut c = subdivide(c, step);
            for n in &mut c.nodes {
                let d = dist(n.anchor, center) as f64;
                if d < r {
                    let w = (1.0 - (d / r).powf(power)) as f32;
                    move_node(n, delta.0 * w, delta.1 * w);
                }
            }
            c
        })
        .collect()
}

/// Freiform „Ziehen“: Knoten (der ursprünglichen Kontur) nahe am Greifpunkt folgen dem Zug mit
/// weichem Abfall über die Länge `len`.
pub fn pull(contours: &[Contour], grab: Point, delta: (f32, f32), len: f64) -> Vec<Contour> {
    let r = (len.clamp(1.0, 1000.0) * 0.5).max(1.0);
    let step = (0.1 * r).clamp(0.5, 8.0);
    contours
        .iter()
        .map(|c| {
            let mut c = subdivide(c, step);
            for n in &mut c.nodes {
                let d = dist(n.anchor, grab) as f64;
                if d < r {
                    let x = d / r;
                    let w = ((1.0 - x * x) * (1.0 - x * x)) as f32;
                    move_node(n, delta.0 * w, delta.1 * w);
                }
            }
            c
        })
        .collect()
}

/// Schnittparameter (Segment, t) einer Kontur mit einer Messerlinie (Polylinie), aufsteigend.
pub fn knife_params(c: &Contour, line: &[Point]) -> Vec<(usize, f64)> {
    let bc = to_bezier(c);
    let mut out: Vec<(usize, f64)> = Vec::new();
    for (si, seg) in bc.segments().enumerate() {
        for w in line.windows(2) {
            let (a, b) = (Vec2::from(w[0]), Vec2::from(w[1]));
            let dir = Vec2::new(b.x - a.x, b.y - a.y);
            let len2 = dir.length_sq();
            if len2 <= 1e-12 {
                continue;
            }
            let f = |t: f64| {
                let q = seg.eval(t);
                dir.cross(Vec2::new(q.x - a.x, q.y - a.y))
            };
            let n = 48;
            let mut t0 = 0.0;
            let mut f0 = f(0.0);
            for k in 1..=n {
                let t1 = k as f64 / n as f64;
                let f1 = f(t1);
                if (f0 <= 0.0 && f1 > 0.0) || (f0 >= 0.0 && f1 < 0.0) {
                    // Halbierung
                    let (mut lo, mut hi, mut flo) = (t0, t1, f0);
                    for _ in 0..40 {
                        let mid = 0.5 * (lo + hi);
                        let fm = f(mid);
                        if (flo <= 0.0 && fm > 0.0) || (flo >= 0.0 && fm < 0.0) {
                            hi = mid;
                        } else {
                            lo = mid;
                            flo = fm;
                        }
                    }
                    let t = 0.5 * (lo + hi);
                    let q = seg.eval(t);
                    let u = ((q.x - a.x) * dir.x + (q.y - a.y) * dir.y) / len2;
                    if (-1e-6..=1.0 + 1e-6).contains(&u) && out.iter().all(|(s, tt)| *s != si || (tt - t).abs() > 1e-4) {
                        out.push((si, t));
                    }
                }
                t0 = t1;
                f0 = f1;
            }
        }
    }
    out.sort_by(|x, y| (x.0, x.1).partial_cmp(&(y.0, y.1)).unwrap_or(std::cmp::Ordering::Equal));
    out
}

/// Teilt eine Kontur an Schnittparametern in offene Stücke (geschlossen mit k Schnitten → k
/// Stücke, offen → k + 1).
pub fn split_at(c: &Contour, cuts: &[(usize, f64)]) -> Vec<Contour> {
    let bc = to_bezier(c);
    let segs: Vec<CubicBez> = bc.segments().collect();
    if cuts.is_empty() || segs.is_empty() {
        return vec![c.clone()];
    }
    // Teilsegmente mit Markierung „nach diesem Stück wird geschnitten“
    let mut subs: Vec<(CubicBez, bool)> = Vec::new();
    for (si, seg) in segs.iter().enumerate() {
        let ts: Vec<f64> = cuts.iter().filter(|(s, _)| *s == si).map(|(_, t)| *t).collect();
        let mut cur = *seg;
        let mut consumed = 0.0;
        for t in ts {
            let local = ((t - consumed) / (1.0 - consumed)).clamp(0.0, 1.0);
            let (l, r) = cur.split(local);
            subs.push((l, true));
            cur = r;
            consumed = t;
        }
        subs.push((cur, false));
    }
    let to_contour = |group: &[CubicBez]| -> Contour {
        let p = |v: Vec2| v.to_point();
        let mut nodes = vec![BezierNode::new(p(group[0].p0), p(group[0].p0), p(group[0].p1))];
        for k in 0..group.len() {
            let out = if k + 1 < group.len() { p(group[k + 1].p1) } else { p(group[k].p3) };
            nodes.push(BezierNode::new(p(group[k].p3), p(group[k].p2), out));
        }
        from_bezier(&BezierContour::new(nodes, false))
    };
    let mut groups: Vec<Vec<CubicBez>> = Vec::new();
    let mut cur: Vec<CubicBez> = Vec::new();
    for (s, cut) in &subs {
        cur.push(*s);
        if *cut {
            groups.push(std::mem::take(&mut cur));
        }
    }
    if bc.closed {
        // das Reststück am Ende gehört vor das erste Stück
        if let Some(first) = groups.first_mut() {
            let mut tail = std::mem::take(&mut cur);
            tail.append(first);
            *first = tail;
        }
        // geschlossene Kontur mit einem Schnitt: ein Stück, das am Schnitt beginnt und endet
        if groups.len() > 1 {
            let first = groups.remove(0);
            groups.push(first);
        }
    } else if !cur.is_empty() {
        groups.push(cur);
    }
    groups.iter().filter(|g| !g.is_empty() && g.iter().map(|s| s.length()).sum::<f64>() > 1e-3).map(|g| to_contour(g)).collect()
}

// ------------------------------------------------------------------ 9-Slice

/// 9-Slice auf einer Achse: Bereich `[a0, a1]` mit festen Rändern `i0`/`i1` auf `[b0, b1]`.
/// Die Ränder behalten ihre Größe (bei zu kleinem Ziel anteilig verkleinert), die Mitte wird
/// gestreckt.
pub fn nine_slice_axis(v: f32, a0: f32, a1: f32, i0: f32, i1: f32, b0: f32, b1: f32) -> f32 {
    let (i0, i1) = (i0.max(0.0), i1.max(0.0));
    let dst = (b1 - b0).abs();
    let k = if i0 + i1 > 0.0 && dst < i0 + i1 { dst / (i0 + i1) } else { 1.0 };
    let (m0, m1) = (a0 + i0, a1 - i1);
    if v <= m0 {
        b0 + (v - a0) * k
    } else if v >= m1 {
        b1 - (a1 - v) * k
    } else {
        let (n0, n1) = (b0 + i0 * k, b1 - i1 * k);
        if m1 - m0 <= 1e-6 { n0 } else { n0 + (v - m0) / (m1 - m0) * (n1 - n0) }
    }
}

/// 9-Slice eines Punkts: `from` → `to`, `insets` = feste Ränder (links, oben, rechts, unten).
pub fn nine_slice_point(p: Point, from: &fp_geom::Rect, insets: [f32; 4], to: &fp_geom::Rect) -> Point {
    Point::new(
        nine_slice_axis(p.x, from.left, from.right, insets[0], insets[2], to.left, to.right),
        nine_slice_axis(p.y, from.top, from.bottom, insets[1], insets[3], to.top, to.bottom),
    )
}

/// 9-Slice von Konturen (Anker und Henkel).
pub fn nine_slice_contours(cs: &[Contour], from: &fp_geom::Rect, insets: [f32; 4], to: &fp_geom::Rect) -> Vec<Contour> {
    cs.iter()
        .map(|c| {
            let mut c = c.clone();
            for n in &mut c.nodes {
                n.anchor = nine_slice_point(n.anchor, from, insets, to);
                n.pred = nine_slice_point(n.pred, from, insets, to);
                n.succ = nine_slice_point(n.succ, from, insets, to);
            }
            c
        })
        .collect()
}

/// Vorgabe-Ränder des 9-Slice-Werkzeugs: je ein Drittel (A).
pub fn default_insets(b: &fp_geom::Rect) -> [f32; 4] {
    let (w, h) = (b.width().abs() / 3.0, b.height().abs() / 3.0);
    [w, h, w, h]
}

// ------------------------------------------------------------------ Pfadausbesserung

/// Stützstellen je Druckprofil.
const SCRUB_SAMPLES: usize = 64;

/// Pfadausbesserung: ändert die Druckwerte (`BZLv/BAZv PRSf`) der Pfadpunkte entlang des Strichs
/// um `amount` (+ mehr Druck, − weniger), gewichtet mit `1 − d/r`. Fehlende Profile gelten als
/// voller Druck 1,0 (A). Liefert, ob der Strich den Pfad berührt (auch wenn der Druck schon am
/// Anschlag war und sich nichts ändert).
pub fn scrub_pressure(contours: &mut [Contour], stroke: &[Point], radius: f32, amount: f32) -> bool {
    let r = radius.max(0.5);
    let mut hit = false;
    for c in contours.iter_mut() {
        let bc = to_bezier(c);
        let segs: Vec<CubicBez> = bc.segments().collect();
        for (i, seg) in segs.iter().enumerate() {
            let Some(n) = c.nodes.get_mut(i) else { continue };
            let mut samples: Vec<f32> = match &n.pressure {
                Some(pp) if pp.samples.len() == SCRUB_SAMPLES => pp.samples.iter().map(|s| s.pressure.unwrap_or(1.0)).collect(),
                Some(pp) if !pp.samples.is_empty() => vec![pp.samples[0].pressure.unwrap_or(1.0); SCRUB_SAMPLES],
                _ => vec![1.0; SCRUB_SAMPLES],
            };
            let mut touched = false;
            for (k, s) in samples.iter_mut().enumerate() {
                let q = seg.eval(k as f64 / (SCRUB_SAMPLES - 1) as f64).to_point();
                let w = stroke.iter().map(|p| 1.0 - dist(*p, q) / r).fold(0.0f32, f32::max);
                if w > 0.0 {
                    hit = true;
                    let nv = (*s + amount * w).clamp(0.0, 1.0);
                    if (nv - *s).abs() > 1e-6 {
                        *s = nv;
                        touched = true;
                    }
                }
            }
            if touched {
                let old_vel: Vec<Option<f32>> = n.pressure.as_ref().map(|pp| pp.samples.iter().map(|s| s.velocity).collect()).unwrap_or_default();
                n.pressure = Some(fp_model::element::PressureProfile {
                    count: Some(SCRUB_SAMPLES as u32),
                    samples: samples
                        .iter()
                        .enumerate()
                        .map(|(k, p)| fp_model::element::PressureSample { pressure: Some(*p), velocity: old_vel.get(k).copied().flatten(), extra: Default::default() })
                        .collect(),
                    extra: Default::default(),
                });
            }
        }
    }
    hit
}

/// Pfadausbesserung additiv/subtraktiv: über ausgewählte Pfade ziehen erhöht bzw. senkt den Druck
/// (Strichbreite bei druckempfindlichen Pinseln). Umschalt = gerade Linie.
#[derive(Clone, Debug)]
pub struct ScrubTool {
    id: ToolId,
    points: Vec<Point>,
    active: bool,
    hover: Option<Point>,
}

impl ScrubTool {
    /// Neues Werkzeug (PathScrubAdd, PathScrubSubtract).
    pub fn new(id: ToolId) -> ScrubTool {
        ScrubTool { id, points: Vec::new(), active: false, hover: None }
    }

    fn size(ctx: &ToolContext) -> f32 {
        ctx.prefs.get_int("PathScrubToolSize", 30).clamp(1, 500) as f32
    }

    fn amount(&self, ctx: &ToolContext) -> f32 {
        let rate = ctx.prefs.get_int("PathScrubToolRate", 50).clamp(1, 100) as f32 / 100.0;
        if self.id == ToolId::PathScrubAdd { rate * 0.5 } else { -rate * 0.5 }
    }
}

impl ToolMachine for ScrubTool {
    fn id(&self) -> ToolId {
        self.id
    }

    fn handle(&mut self, ev: &ToolEvent, ctx: &ToolContext) -> ToolResponse {
        if !ctx.has_document {
            return ToolResponse::default();
        }
        if ev.input == ToolInput::Key(ToolKey::Escape) {
            self.points.clear();
            self.active = false;
            return ToolResponse::default();
        }
        let ToolInput::Pointer { kind, pos } = &ev.input else { return ToolResponse::default() };
        self.hover = Some(*pos);
        match kind {
            PointerKind::Down => {
                if !ctx.selection.elements.iter().filter_map(|id| docops::element_on_page(ctx.doc, ctx.page, *id)).any(|e| matches!(e.kind, ElementKind::Path(_))) {
                    return ToolResponse { cursor: Cursor::NotAllowed, status: Some("Pfadausbesserung: zuerst einen Pfad auswählen.".into()), ..ToolResponse::default() };
                }
                self.points = vec![*pos];
                self.active = true;
            }
            PointerKind::Move if self.active => {
                if ev.mods.shift {
                    let s = self.points[0];
                    self.points = vec![s, *pos];
                } else if self.points.last().is_none_or(|l| dist(*l, *pos) >= 1.0) {
                    self.points.push(*pos);
                }
            }
            PointerKind::Up if self.active => {
                self.active = false;
                self.points.push(*pos);
                let pts = std::mem::take(&mut self.points);
                return ToolResponse::commit(vec![Invocation::new(
                    "path.scrub",
                    json!({"points": pts.iter().map(|p| pt_json(*p)).collect::<Vec<Value>>(), "amount": self.amount(ctx), "size": Self::size(ctx)}),
                )]);
            }
            _ => {}
        }
        ToolResponse::with_overlay(self.feedback(ctx), Cursor::Brush)
    }

    fn overlay(&self) -> Vec<Overlay> {
        if self.points.len() < 2 { Vec::new() } else { vec![Overlay::Polyline { points: self.points.clone(), closed: false }] }
    }

    fn feedback(&self, ctx: &ToolContext) -> Vec<Overlay> {
        let mut o = self.overlay();
        if let Some(h) = self.hover {
            o.push(Overlay::BrushTip { at: h, diameter: Self::size(ctx), round: true });
        }
        o
    }
}

// ------------------------------------------------------------------ Messer

/// Messer: über ausgewählte Pfade ziehen schneidet sie an den Schnittpunkten (Umschalt = gerade
/// Linie in 45°-Schritten). Ergebnis: getrennte Pfade (`knifeElementsFromPoints`).
#[derive(Clone, Debug, Default)]
pub struct KnifeTool {
    points: Vec<Point>,
    active: bool,
}

impl ToolMachine for KnifeTool {
    fn id(&self) -> ToolId {
        ToolId::Knife
    }

    fn handle(&mut self, ev: &ToolEvent, ctx: &ToolContext) -> ToolResponse {
        if !ctx.has_document {
            return ToolResponse::default();
        }
        if ev.input == ToolInput::Key(ToolKey::Escape) {
            self.points.clear();
            self.active = false;
            return ToolResponse::default();
        }
        let ToolInput::Pointer { kind, pos } = &ev.input else { return ToolResponse::default() };
        match kind {
            PointerKind::Down => {
                if ctx.selection.elements.is_empty() {
                    return ToolResponse { cursor: Cursor::NotAllowed, status: Some("Messer: zuerst Pfade auswählen.".into()), ..ToolResponse::default() };
                }
                self.points = vec![*pos];
                self.active = true;
            }
            PointerKind::Move if self.active => {
                if ev.mods.shift {
                    let s = self.points[0];
                    self.points = vec![s, constrain_angle(s, *pos, 45.0)];
                } else if self.points.last().is_none_or(|l| dist(*l, *pos) >= 1.0) {
                    self.points.push(*pos);
                }
            }
            PointerKind::Up if self.active => {
                self.active = false;
                if ev.mods.shift {
                    let s = self.points[0];
                    self.points = vec![s, constrain_angle(s, *pos, 45.0)];
                } else {
                    self.points.push(*pos);
                }
                let pts = std::mem::take(&mut self.points);
                if pts.len() < 2 || dist(pts[0], *pts.last().unwrap_or(&pts[0])) < ctx.drag_threshold() && pts.len() < 3 {
                    return ToolResponse::default();
                }
                return ToolResponse::commit(vec![Invocation::new("path.knife", json!({"points": pts.iter().map(|p| pt_json(*p)).collect::<Vec<Value>>()}))]);
            }
            _ => {}
        }
        ToolResponse::with_overlay(self.overlay(), Cursor::Knife)
    }

    fn overlay(&self) -> Vec<Overlay> {
        if self.points.len() < 2 { Vec::new() } else { vec![Overlay::Polyline { points: self.points.clone(), closed: false }] }
    }
}

// ------------------------------------------------------------------ Freiform / Bereich umformen

#[derive(Clone, Debug, PartialEq)]
enum FMode {
    Push,
    Pull { grab: Point },
    Reshape,
}

/// Freiform (Schieben außerhalb, Ziehen am Pfad) und Bereich umformen auf ausgewählten Pfaden.
#[derive(Clone, Debug)]
pub struct FreeformTool {
    id: ToolId,
    /// (Element, ursprüngliche Konturen, Arbeitsstand).
    work: Vec<(ElementId, Vec<Contour>, Vec<Contour>)>,
    mode: Option<FMode>,
    last: Option<Point>,
    hover: Option<Point>,
}

impl FreeformTool {
    /// Neues Werkzeug (Freeform, Reshape).
    pub fn new(id: ToolId) -> FreeformTool {
        FreeformTool { id, work: Vec::new(), mode: None, last: None, hover: None }
    }

    fn size(&self, ctx: &ToolContext) -> f64 {
        if self.id == ToolId::Reshape { ctx.prefs.get_int("FreeFormGravDiameter", 72) as f64 } else { ctx.prefs.get_int("FreeFormPushNPullLen", 72) as f64 }
    }

    fn selected_paths(ctx: &ToolContext) -> Vec<(ElementId, Vec<Contour>)> {
        ctx.selection
            .elements
            .iter()
            .filter_map(|id| docops::element_on_page(ctx.doc, ctx.page, *id))
            .filter_map(|e| match &e.kind {
                ElementKind::Path(p) => Some((e.id, p.contours.clone())),
                _ => None,
            })
            .collect()
    }

    fn cursor(&self) -> Cursor {
        if self.id == ToolId::Reshape { Cursor::Reshape } else { Cursor::Freeform }
    }
}

impl ToolMachine for FreeformTool {
    fn id(&self) -> ToolId {
        self.id
    }

    fn handle(&mut self, ev: &ToolEvent, ctx: &ToolContext) -> ToolResponse {
        if !ctx.has_document {
            return ToolResponse::default();
        }
        if ev.input == ToolInput::Key(ToolKey::Escape) {
            self.work.clear();
            self.mode = None;
            return ToolResponse::default();
        }
        let ToolInput::Pointer { kind, pos } = &ev.input else { return ToolResponse::default() };
        self.hover = Some(*pos);
        let size = self.size(ctx);
        match kind {
            PointerKind::Down => {
                let paths = Self::selected_paths(ctx);
                if paths.is_empty() {
                    return ToolResponse { cursor: Cursor::NotAllowed, status: Some("Zuerst einen Pfad auswählen.".into()), ..ToolResponse::default() };
                }
                let near = paths.iter().any(|(_, cs)| crate::geometry::distance_to_contours(cs, *pos) <= ctx.pick_tolerance());
                self.mode = Some(match self.id {
                    ToolId::Reshape => FMode::Reshape,
                    _ if near => FMode::Pull { grab: *pos },
                    _ => FMode::Push,
                });
                self.work = paths.into_iter().map(|(id, cs)| (id, cs.clone(), cs)).collect();
                self.last = Some(*pos);
                if self.mode == Some(FMode::Push) {
                    for w in &mut self.work {
                        w.2 = push_pull(&w.2, *pos, size);
                    }
                }
            }
            PointerKind::Move if self.mode.is_some() => {
                let last = self.last.unwrap_or(*pos);
                match self.mode.clone() {
                    Some(FMode::Push) => {
                        for w in &mut self.work {
                            w.2 = push_pull(&w.2, *pos, size);
                        }
                    }
                    Some(FMode::Pull { grab }) => {
                        for w in &mut self.work {
                            w.2 = pull(&w.1, grab, (pos.x - grab.x, pos.y - grab.y), size);
                        }
                    }
                    Some(FMode::Reshape) => {
                        let power = gravity_power(ctx.prefs.get_int("FreeFormGravPower", 80) as f64);
                        for w in &mut self.work {
                            w.2 = reshape_area(&w.2, last, (pos.x - last.x, pos.y - last.y), size, power);
                        }
                    }
                    None => {}
                }
                self.last = Some(*pos);
            }
            PointerKind::Up if self.mode.is_some() => {
                self.mode = None;
                let work = std::mem::take(&mut self.work);
                let cmds: Vec<Invocation> = work
                    .into_iter()
                    .filter(|(_, a, b)| a != b)
                    .map(|(id, _, cs)| Invocation::new("path.setContours", json!({"id": id.0, "contours": contours_json(&cs)})))
                    .collect();
                return ToolResponse { commands: cmds, cursor: self.cursor(), ..ToolResponse::default() };
            }
            _ => {}
        }
        let mut o = self.overlay();
        if matches!(self.mode, None | Some(FMode::Push) | Some(FMode::Reshape)) {
            o.push(Overlay::BrushTip { at: *pos, diameter: size as f32, round: true });
        }
        ToolResponse::with_overlay(o, self.cursor())
    }

    fn overlay(&self) -> Vec<Overlay> {
        self.work.iter().map(|(_, _, cs)| Overlay::Path { contours: cs.clone() }).collect()
    }

    fn feedback(&self, ctx: &ToolContext) -> Vec<Overlay> {
        let mut o = self.overlay();
        if let Some(h) = self.hover
            && !matches!(self.mode, Some(FMode::Pull { .. }))
        {
            o.push(Overlay::BrushTip { at: h, diameter: self.size(ctx) as f32, round: true });
        }
        o
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::polygon_contour;

    #[test]
    fn knife_splits_closed_square_into_two() {
        let sq = polygon_contour(&[Point::new(0.0, 0.0), Point::new(10.0, 0.0), Point::new(10.0, 10.0), Point::new(0.0, 10.0)], true);
        let cuts = knife_params(&sq, &[Point::new(5.0, -5.0), Point::new(5.0, 15.0)]);
        assert_eq!(cuts.len(), 2);
        let parts = split_at(&sq, &cuts);
        assert_eq!(parts.len(), 2);
        assert!(parts.iter().all(|p| !p.closed));
        let line = polygon_contour(&[Point::new(0.0, 5.0), Point::new(10.0, 5.0)], false);
        let parts = split_at(&line, &knife_params(&line, &[Point::new(5.0, 0.0), Point::new(5.0, 10.0)]));
        assert_eq!(parts.len(), 2);
        assert!((parts[0].nodes.last().map(|n| n.anchor.x).unwrap_or(0.0) - 5.0).abs() < 1e-3);
    }

    #[test]
    fn push_moves_nodes_to_circle_edge() {
        let line = polygon_contour(&[Point::new(0.0, 0.0), Point::new(100.0, 0.0)], false);
        let out = push_pull(&[line], Point::new(50.0, 2.0), 20.0);
        let min_d = out[0].nodes.iter().map(|n| dist(n.anchor, Point::new(50.0, 2.0))).fold(f32::INFINITY, f32::min);
        assert!(min_d >= 9.9, "{min_d}");
    }
}
