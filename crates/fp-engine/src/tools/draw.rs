//! Vektorwerkzeuge: Rechteck/Ellipse/Polygon, Linie, Stift, Vektorpfad/Pfad nachzeichnen,
//! Hotspots und Segmente (10 §2.6). Alle Punkte laufen vor dem Setzen durch den Fang
//! ([`super::snap::Snapper`]); die Rückmeldung zeigt Form, Größe und Fanglinien.

use super::pointer::contours_json;
use super::snap::Snapper;
use super::{
    constrain_angle, constrain_rect, dist, fmt_num, pt_json, rect_json, size_label, Cursor, Invocation, Overlay, PointerKind, ToolContext, ToolEvent,
    ToolId, ToolInput, ToolKey, ToolMachine, ToolResponse,
};
use crate::docops;
use crate::geometry;
use fp_geom::{Point, Rect};
use fp_model::{Contour, ElementId, ElementKind};
use serde_json::{json, Value};

#[derive(Clone, Debug, PartialEq)]
enum Drag {
    Idle,
    Pressed { start: Point, last: Point },
    Dragging { start: Point, cur: Point, last: Point },
}

/// Gemeinsamer Ziehvorgang mit Schwelle und Leertasten-Verschiebung.
#[derive(Clone, Debug)]
struct DragTracker {
    state: Drag,
}

impl DragTracker {
    fn new() -> DragTracker {
        DragTracker { state: Drag::Idle }
    }

    /// Verarbeitet ein Zeigerereignis; liefert (Start, aktueller Punkt), sobald gezogen wird.
    fn update(&mut self, kind: PointerKind, p: Point, ev: &ToolEvent, thr: f32) -> Option<(Point, Point)> {
        match (kind, self.state.clone()) {
            (PointerKind::Down, _) => {
                self.state = Drag::Pressed { start: p, last: p };
                None
            }
            (PointerKind::Move | PointerKind::Up, Drag::Pressed { start, .. }) => {
                if dist(start, p) >= thr {
                    self.state = Drag::Dragging { start, cur: p, last: p };
                    Some((start, p))
                } else {
                    None
                }
            }
            (PointerKind::Move | PointerKind::Up, Drag::Dragging { mut start, last, .. }) => {
                if ev.mods.space {
                    // Leertaste: Form verschieben (Startpunkt wandert mit, B `@00d5bbc0`)
                    start = Point::new(start.x + p.x - last.x, start.y + p.y - last.y);
                }
                self.state = Drag::Dragging { start, cur: p, last: p };
                Some((start, p))
            }
            _ => None,
        }
    }

    fn reset(&mut self) {
        self.state = Drag::Idle;
    }

    fn dragging(&self) -> Option<(Point, Point)> {
        match self.state {
            Drag::Dragging { start, cur, .. } => Some((start, cur)),
            _ => None,
        }
    }

    fn active(&self) -> bool {
        self.state != Drag::Idle
    }
}

/// Rechteck, Ellipse und Polygon.
#[derive(Clone, Debug)]
pub struct ShapeTool {
    id: ToolId,
    drag: DragTracker,
    /// Eckenradius in px, per Pfeil auf/ab beim Aufziehen geändert.
    corner_px: Option<f32>,
    overlay: Vec<Overlay>,
    guides: Vec<Overlay>,
    /// Automatische Form: gezogener Steuerpunkt.
    control: Option<super::pointer::ControlDrag>,
}

impl ShapeTool {
    /// Neues Werkzeug (Rectangle, Ellipse, Polygon, AutoShape).
    pub fn new(id: ToolId) -> ShapeTool {
        ShapeTool { id, drag: DragTracker::new(), corner_px: None, overlay: Vec::new(), guides: Vec::new(), control: None }
    }

    fn roundness(&self, r: &Rect, ctx: &ToolContext) -> f32 {
        match self.corner_px {
            Some(px) => {
                let m = r.width().abs().min(r.height().abs());
                if m > 0.0 { (2.0 * px / m).clamp(0.0, 1.0) } else { 0.0 }
            }
            None => ctx.options.rect_roundness.clamp(0.0, 1.0),
        }
    }

    fn preview(&self, start: Point, cur: Point, ev: &ToolEvent, ctx: &ToolContext) -> (Vec<Overlay>, Option<Invocation>) {
        match self.id {
            ToolId::Polygon => {
                let corner = if ev.mods.shift { constrain_angle(start, cur, 15.0) } else { cur };
                let o = ctx.options;
                let c = geometry::polygon_tool_contour(start, corner, o.polygon_sides as usize, o.polygon_star, o.polygon_spikiness.map(|v| v as f64));
                let mut ov = c.map(|c| vec![Overlay::Path { contours: vec![c] }]).unwrap_or_default();
                let r = dist(start, corner);
                let a = -((corner.y - start.y) as f64).atan2((corner.x - start.x) as f64).to_degrees();
                ov.push(Overlay::Label { at: corner, text: format!("r {} · {}°", fmt_num(r), fmt_num(a as f32)) });
                let inv = Invocation::new(
                    "draw.polygon",
                    json!({"center": pt_json(start), "corner": pt_json(corner), "sides": o.polygon_sides, "star": o.polygon_star,
                           "spikiness": o.polygon_spikiness}),
                );
                (ov, Some(inv))
            }
            ToolId::AutoShape => {
                // Automatische Form: Rahmen aufziehen, die Form selbst erzeugt das Skript der Form
                let r = constrain_rect(start, cur, ev.mods.shift, ev.mods.alt);
                (
                    vec![Overlay::Rect { rect: r, dashed: false }, size_label(r)],
                    Some(Invocation::new("autoShape.insert", json!({"shape": ctx.options.auto_shape, "rect": rect_json(r)}))),
                )
            }
            ToolId::Ellipse => {
                let r = constrain_rect(start, cur, ev.mods.shift, ev.mods.alt);
                (vec![Overlay::Path { contours: vec![geometry::ellipse_contour(r)] }, size_label(r)], Some(Invocation::new("draw.ellipse", json!({"rect": rect_json(r)}))))
            }
            _ => {
                let r = constrain_rect(start, cur, ev.mods.shift, ev.mods.alt);
                let rd = self.roundness(&r, ctx);
                (
                    vec![Overlay::Path { contours: vec![geometry::rect_contour(r, rd as f64)] }, size_label(r)],
                    Some(Invocation::new("draw.rectangle", json!({"rect": rect_json(r), "roundness": rd}))),
                )
            }
        }
    }
}

impl ToolMachine for ShapeTool {
    fn id(&self) -> ToolId {
        self.id
    }

    fn handle(&mut self, ev: &ToolEvent, ctx: &ToolContext) -> ToolResponse {
        if !ctx.has_document {
            return ToolResponse::default();
        }
        match &ev.input {
            ToolInput::Pointer { kind, pos } => {
                // Automatische Form: Steuerpunkte der ausgewählten Formen ziehen
                if self.id == ToolId::AutoShape {
                    use super::pointer::{control_point_at, control_points_feedback, ControlDrag};
                    match (kind, self.control.take()) {
                        (PointerKind::Down, _) => {
                            if let Some((id, index, at)) = control_point_at(ctx, *pos, ctx.pick_tolerance() as f32) {
                                self.control = Some(ControlDrag { id, index, from: at, points: vec![at] });
                                return ToolResponse::with_overlay(control_points_feedback(ctx, Some((id, index, at))), Cursor::PointerPoint);
                            }
                        }
                        (PointerKind::Move, Some(mut c)) => {
                            c.points.push(*pos);
                            let o = control_points_feedback(ctx, Some((c.id, c.index, *pos)));
                            self.control = Some(c);
                            return ToolResponse::with_overlay(o, Cursor::PointerPoint);
                        }
                        (PointerKind::Up, Some(mut c)) => {
                            c.points.push(*pos);
                            return match c.finish(*pos, ev) {
                                Some(inv) => ToolResponse::commit(vec![inv]),
                                None => ToolResponse::default(),
                            };
                        }
                        (_, c) => self.control = c,
                    }
                    if *kind == PointerKind::Move && !self.drag.active() && control_point_at(ctx, *pos, ctx.pick_tolerance() as f32).is_some() {
                        return ToolResponse::with_overlay(control_points_feedback(ctx, None), Cursor::PointerPoint);
                    }
                }
                if *kind == PointerKind::Down {
                    self.corner_px = None;
                }
                let sp = Snapper::new(ctx, &[]).point(*pos);
                if !self.drag.active() && *kind != PointerKind::Down {
                    // Überfahren: nur Fanglinien zeigen
                    self.guides = sp.guides;
                    return ToolResponse::with_overlay(self.guides.clone(), Cursor::Crosshair);
                }
                let pressed_at = match self.drag.state {
                    Drag::Pressed { start, .. } => Some(start),
                    _ => None,
                };
                let d = self.drag.update(*kind, sp.point, ev, ctx.drag_threshold());
                if *kind == PointerKind::Up && d.is_none() && self.id == ToolId::AutoShape
                    && let Some(at) = pressed_at
                {
                    // Klick ohne Ziehen: Form in Vorgabegröße an dieser Stelle
                    self.drag.reset();
                    return ToolResponse::commit(vec![Invocation::new("autoShape.insert", json!({"shape": ctx.options.auto_shape, "at": pt_json(at)}))]);
                }
                if *kind == PointerKind::Up {
                    self.drag.reset();
                    self.overlay.clear();
                    self.guides.clear();
                    return match d {
                        Some((s, c)) => ToolResponse::commit(self.preview(s, c, ev, ctx).1.into_iter().map(|i| super::styled(i, ctx.options)).collect()),
                        None => ToolResponse::default(),
                    };
                }
                self.guides = sp.guides;
                if let Some((s, c)) = d {
                    self.overlay = self.preview(s, c, ev, ctx).0;
                }
                ToolResponse::with_overlay(self.overlay(), Cursor::Crosshair)
            }
            ToolInput::Key(k @ (ToolKey::Up | ToolKey::Down)) if self.id == ToolId::Rectangle => {
                if let Some((s, c)) = self.drag.dragging() {
                    let r = constrain_rect(s, c, ev.mods.shift, ev.mods.alt);
                    let max = r.width().abs().min(r.height().abs()) / 2.0;
                    let cur = self.corner_px.unwrap_or(ctx.options.rect_roundness * max);
                    let step = if *k == ToolKey::Up { 1.0 } else { -1.0 };
                    self.corner_px = Some((cur + step).clamp(0.0, max));
                    self.overlay = self.preview(s, c, ev, ctx).0;
                }
                ToolResponse::with_overlay(self.overlay(), Cursor::Crosshair)
            }
            ToolInput::Key(ToolKey::Escape) => {
                self.drag.reset();
                self.control = None;
                self.overlay.clear();
                self.guides.clear();
                ToolResponse::default()
            }
            _ => ToolResponse::default(),
        }
    }

    fn overlay(&self) -> Vec<Overlay> {
        let mut v = self.overlay.clone();
        v.extend(self.guides.iter().cloned());
        v
    }

    fn feedback(&self, ctx: &ToolContext) -> Vec<Overlay> {
        let mut v = self.overlay();
        if self.id == ToolId::AutoShape {
            let drag = self.control.as_ref().map(|c| (c.id, c.index, *c.points.last().unwrap_or(&c.from)));
            v.extend(super::pointer::control_points_feedback(ctx, drag));
        }
        v
    }
}

/// Linie: Umschalt rastet auf 45°.
#[derive(Clone, Debug)]
pub struct LineTool {
    drag: DragTracker,
    overlay: Vec<Overlay>,
}

impl Default for LineTool {
    fn default() -> Self {
        LineTool { drag: DragTracker::new(), overlay: Vec::new() }
    }
}

impl ToolMachine for LineTool {
    fn id(&self) -> ToolId {
        ToolId::Line
    }

    fn handle(&mut self, ev: &ToolEvent, ctx: &ToolContext) -> ToolResponse {
        if !ctx.has_document {
            return ToolResponse::default();
        }
        if ev.input == ToolInput::Key(ToolKey::Escape) {
            self.drag.reset();
            self.overlay.clear();
            return ToolResponse::default();
        }
        let ToolInput::Pointer { kind, pos } = &ev.input else { return ToolResponse::default() };
        let sp = Snapper::new(ctx, &[]).point(*pos);
        if !self.drag.active() && *kind != PointerKind::Down {
            self.overlay = sp.guides;
            return ToolResponse::with_overlay(self.overlay.clone(), Cursor::Crosshair);
        }
        let d = self.drag.update(*kind, sp.point, ev, ctx.drag_threshold());
        let line = d.map(|(s, c)| (s, if ev.mods.shift { constrain_angle(s, c, 45.0) } else { c }));
        if *kind == PointerKind::Up {
            self.drag.reset();
            self.overlay.clear();
            return match line {
                Some((a, b)) => ToolResponse::commit(vec![super::styled(Invocation::new("draw.line", json!({"from": pt_json(a), "to": pt_json(b)})), ctx.options)]),
                None => ToolResponse::default(),
            };
        }
        if let Some((a, b)) = line {
            let ang = -((b.y - a.y) as f64).atan2((b.x - a.x) as f64).to_degrees();
            self.overlay = vec![Overlay::Line { from: a, to: b }, Overlay::Label { at: b, text: format!("{} px · {}°", fmt_num(dist(a, b)), fmt_num(ang as f32)) }];
            self.overlay.extend(sp.guides);
        }
        ToolResponse::with_overlay(self.overlay.clone(), Cursor::Crosshair)
    }

    fn overlay(&self) -> Vec<Overlay> {
        self.overlay.clone()
    }
}

/// Knoten des Stifts: (Anker, Eingang, Ausgang).
type PenNode = (Point, Point, Point);

/// Fortgesetzter Pfad: Element, Konturindex, Originalkonturen.
#[derive(Clone, Debug)]
struct PenTarget {
    id: ElementId,
    contour: usize,
    contours: Vec<Contour>,
}

/// Stift (Bézier): Klick = Eckpunkt, Ziehen = Kurvenpunkt (Tangenten symmetrisch, Alt bricht),
/// Umschalt 45°, Klick auf den Startpunkt schließt, Doppelklick/Eingabe/Werkzeugwechsel beendet offen.
/// Bei ausgewähltem Pfad: Klick auf einen Endpunkt setzt den Pfad fort, Klick auf einen Punkt
/// entfernt ihn, Klick auf ein Segment fügt einen Punkt ein.
#[derive(Clone, Debug, Default)]
pub struct PenTool {
    nodes: Vec<PenNode>,
    button: bool,
    closing: bool,
    hover: Option<Point>,
    target: Option<PenTarget>,
    cursor: Cursor,
    guides: Vec<Overlay>,
}

/// Was unter dem Stift liegt (nur bei genau einem ausgewählten Pfad und ohne laufenden Pfad).
enum PenHit {
    /// Endpunkt einer offenen Kontur (am Anfang = `true`).
    End { id: ElementId, contour: usize, at_start: bool },
    Point { id: ElementId, contour: usize, node: usize },
    Segment { id: ElementId, contour: usize, segment: usize, t: f64 },
}

impl PenTool {
    fn contour(&self, closed: bool) -> Contour {
        geometry::contour_from_nodes(&self.nodes, closed)
    }

    fn pen_hit(ctx: &ToolContext, p: Point) -> Option<PenHit> {
        if ctx.selection.elements.len() != 1 {
            return None;
        }
        let id = ctx.selection.elements[0];
        let e = docops::element_on_page(ctx.doc, ctx.page, id)?;
        let ElementKind::Path(pe) = &e.kind else { return None };
        if pe.primitive.is_some() {
            return None;
        }
        let tol = ctx.pick_tolerance() as f32;
        for (ci, c) in pe.contours.iter().enumerate() {
            let n = c.nodes.len();
            if n == 0 {
                continue;
            }
            if !c.closed {
                if dist(c.nodes[n - 1].anchor, p) <= tol {
                    return Some(PenHit::End { id, contour: ci, at_start: false });
                }
                if dist(c.nodes[0].anchor, p) <= tol {
                    return Some(PenHit::End { id, contour: ci, at_start: true });
                }
            }
            for (ni, k) in c.nodes.iter().enumerate() {
                if dist(k.anchor, p) <= tol {
                    return Some(PenHit::Point { id, contour: ci, node: ni });
                }
            }
        }
        for (ci, c) in pe.contours.iter().enumerate() {
            let bc = geometry::to_bezier(c);
            if let Some((seg, near)) = bc.nearest(fp_geom::Vec2::from(p))
                && near.distance <= tol as f64
                && near.t > 0.001
                && near.t < 0.999
            {
                return Some(PenHit::Segment { id, contour: ci, segment: seg, t: near.t });
            }
        }
        None
    }

    fn commit(&mut self, closed: bool, opts: &super::ToolOptions) -> ToolResponse {
        let nodes = std::mem::take(&mut self.nodes);
        let target = self.target.take();
        self.closing = false;
        self.button = false;
        self.hover = None;
        self.guides.clear();
        if let Some(t) = target {
            if nodes.len() < 2 && !closed {
                return ToolResponse::default();
            }
            let mut cs = t.contours.clone();
            if let Some(c) = cs.get_mut(t.contour) {
                *c = geometry::contour_from_nodes(&nodes, closed);
            }
            return ToolResponse::commit(vec![Invocation::new("path.setContours", json!({"id": t.id.0, "contours": contours_json(&cs)}))]);
        }
        if nodes.len() < 2 {
            return ToolResponse::default();
        }
        let c = json!({"closed": closed, "nodes": nodes.iter().map(|(a, i, o)| json!({"anchor": pt_json(*a), "in": pt_json(*i), "out": pt_json(*o)})).collect::<Vec<Value>>()});
        ToolResponse::commit(vec![super::styled(Invocation::new("draw.path", json!({"contours": [c]})), opts)])
    }

    /// Fortsetzen: Knoten der Kontur übernehmen (am Anfang begonnen → umgedreht).
    fn start_continue(&mut self, ctx: &ToolContext, id: ElementId, contour: usize, at_start: bool) -> bool {
        let Some(e) = docops::element_on_page(ctx.doc, ctx.page, id) else { return false };
        let ElementKind::Path(pe) = &e.kind else { return false };
        let Some(c) = pe.contours.get(contour) else { return false };
        let mut nodes: Vec<PenNode> = c.nodes.iter().map(|n| (n.anchor, n.pred, n.succ)).collect();
        if at_start {
            nodes.reverse();
            for n in &mut nodes {
                *n = (n.0, n.2, n.1);
            }
        }
        self.nodes = nodes;
        self.target = Some(PenTarget { id, contour, contours: pe.contours.clone() });
        true
    }

    fn hover_cursor(&self, ctx: &ToolContext, p: Point) -> Cursor {
        if !self.nodes.is_empty() {
            let close = self.nodes.len() >= 2 && dist(p, self.nodes[0].0) <= ctx.pick_tolerance() as f32;
            return if close { Cursor::PenClose } else { Cursor::Pen };
        }
        match Self::pen_hit(ctx, p) {
            Some(PenHit::End { .. }) => Cursor::PenContinue,
            Some(PenHit::Point { .. }) => Cursor::PenRemove,
            Some(PenHit::Segment { .. }) => Cursor::PenAdd,
            None => Cursor::Pen,
        }
    }
}

impl ToolMachine for PenTool {
    fn id(&self) -> ToolId {
        ToolId::Pen
    }

    fn handle(&mut self, ev: &ToolEvent, ctx: &ToolContext) -> ToolResponse {
        if !ctx.has_document {
            return ToolResponse::default();
        }
        let tol = ctx.pick_tolerance() as f32;
        match &ev.input {
            ToolInput::Pointer { kind: PointerKind::Down, pos } => {
                self.button = true;
                if self.nodes.len() >= 2 && dist(*pos, self.nodes[0].0) <= tol {
                    self.closing = true;
                    return ToolResponse { cursor: Cursor::PenClose, ..ToolResponse::default() };
                }
                if self.nodes.is_empty() {
                    match Self::pen_hit(ctx, *pos) {
                        Some(PenHit::End { id, contour, at_start }) => {
                            if self.start_continue(ctx, id, contour, at_start) {
                                return ToolResponse::with_overlay(self.overlay(), Cursor::PenContinue);
                            }
                        }
                        Some(PenHit::Point { id, contour, node }) => {
                            self.button = false;
                            return ToolResponse::commit(vec![Invocation::new("path.removeNode", json!({"id": id.0, "contour": contour, "node": node}))]);
                        }
                        Some(PenHit::Segment { id, contour, segment, t }) => {
                            self.button = false;
                            return ToolResponse::commit(vec![Invocation::new("path.insertNode", json!({"id": id.0, "contour": contour, "segment": segment, "t": t}))]);
                        }
                        None => {}
                    }
                }
                let snapped = Snapper::new(ctx, &[]).point(*pos);
                let p = match self.nodes.last() {
                    Some(last) if ev.mods.shift => constrain_angle(last.0, *pos, 45.0),
                    _ => snapped.point,
                };
                self.nodes.push((p, p, p));
                self.guides = snapped.guides;
                ToolResponse::with_overlay(self.overlay(), Cursor::Pen)
            }
            ToolInput::Pointer { kind: PointerKind::Move, pos } => {
                if self.button {
                    let idx = if self.closing { 0 } else { self.nodes.len().saturating_sub(1) };
                    if let Some(n) = self.nodes.get_mut(idx) {
                        let a = n.0;
                        let q = if ev.mods.shift { constrain_angle(a, *pos, 45.0) } else { *pos };
                        if self.closing {
                            // Schließen mit Ziehen: Eingangshenkel des Startpunkts (gespiegelt)
                            n.1 = Point::new(2.0 * a.x - q.x, 2.0 * a.y - q.y);
                            if !ev.mods.alt {
                                n.2 = q;
                            }
                        } else {
                            n.2 = q;
                            if !ev.mods.alt {
                                n.1 = Point::new(2.0 * a.x - q.x, 2.0 * a.y - q.y);
                            }
                        }
                    }
                    self.guides.clear();
                    self.cursor = if self.closing { Cursor::PenClose } else { Cursor::Pen };
                } else {
                    let sp = Snapper::new(ctx, &[]).point(*pos);
                    let h = match self.nodes.last() {
                        Some(last) if ev.mods.shift => constrain_angle(last.0, *pos, 45.0),
                        _ => sp.point,
                    };
                    self.hover = Some(h);
                    self.guides = sp.guides;
                    self.cursor = self.hover_cursor(ctx, *pos);
                }
                ToolResponse::with_overlay(self.overlay(), self.cursor)
            }
            ToolInput::Pointer { kind: PointerKind::Up, .. } => {
                self.button = false;
                if self.closing {
                    return self.commit(true, ctx.options);
                }
                ToolResponse::with_overlay(self.overlay(), Cursor::Pen)
            }
            ToolInput::Pointer { kind: PointerKind::DoubleClick, .. } => {
                // der Doppelklick hat bereits einen Punkt gesetzt (Down) – doppelten Endpunkt entfernen
                if self.nodes.len() >= 2 {
                    let n = self.nodes.len();
                    if dist(self.nodes[n - 1].0, self.nodes[n - 2].0) <= tol {
                        self.nodes.pop();
                    }
                }
                self.commit(false, ctx.options)
            }
            ToolInput::Key(ToolKey::Enter) | ToolInput::Key(ToolKey::Escape) => self.commit(false, ctx.options),
            ToolInput::Key(ToolKey::Backspace) | ToolInput::Key(ToolKey::Delete) if !self.nodes.is_empty() => {
                // letzten gesetzten Punkt zurücknehmen
                self.nodes.pop();
                if self.nodes.is_empty() {
                    self.target = None;
                }
                ToolResponse::with_overlay(self.overlay(), Cursor::Pen)
            }
            _ => ToolResponse::default(),
        }
    }

    fn finish(&mut self, ctx: &ToolContext) -> ToolResponse {
        self.commit(false, ctx.options)
    }

    fn overlay(&self) -> Vec<Overlay> {
        let mut v = Vec::new();
        if !self.nodes.is_empty() {
            v.push(Overlay::Path { contours: vec![self.contour(false)] });
            let last = self.nodes.len() - 1;
            let mut pts = self.contour(false);
            if self.closing && pts.nodes.len() > 1 {
                pts.closed = true;
            }
            v.push(Overlay::PathPoints { contours: vec![pts], selected: vec![(0, if self.closing { 0 } else { last })], handles: true });
            if let (Some(h), Some(l)) = (self.hover, self.nodes.last())
                && !self.button
            {
                if l.2 == l.0 {
                    v.push(Overlay::Line { from: l.0, to: h });
                } else {
                    // Vorschau des nächsten Segments (`ShowPenPreview`): Kurve aus dem Ausgangshenkel
                    v.push(Overlay::Path { contours: vec![geometry::contour_from_nodes(&[*l, (h, h, h)], false)] });
                }
            }
        }
        v.extend(self.guides.iter().cloned());
        v
    }
}

/// Vektorpfad und Pfad nachzeichnen (Freihand): Punktfolge beim Ziehen, beim Loslassen
/// vereinfacht und geglättet (`PathToolPrecision`); Umschalt = gerade Linie.
#[derive(Clone, Debug)]
pub struct FreehandTool {
    id: ToolId,
    points: Vec<Point>,
    active: bool,
}

impl FreehandTool {
    /// Neues Werkzeug (VectorPath oder RedrawPath).
    pub fn new(id: ToolId) -> FreehandTool {
        FreehandTool { id, points: Vec::new(), active: false }
    }
}

/// Freihand-Punkte → Konturparameter für `draw.path`.
pub fn freehand_contour_json(points: &[Point], precision: u32, closed: bool) -> Option<Value> {
    if points.len() < 2 {
        return None;
    }
    let tol = (11 - precision.clamp(1, 10)) as f64 * 0.4;
    let simple = geometry::simplify_polyline(points, tol);
    let c = geometry::smooth_contour(&simple, closed);
    Some(json!({"closed": closed, "nodes": c.nodes.iter().map(|n| json!({"anchor": pt_json(n.anchor), "in": pt_json(n.pred), "out": pt_json(n.succ)})).collect::<Vec<Value>>()}))
}

impl ToolMachine for FreehandTool {
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
        match kind {
            PointerKind::Down => {
                self.points = vec![*pos];
                self.active = true;
            }
            PointerKind::Move if self.active => {
                if ev.mods.shift && !self.points.is_empty() {
                    self.points.truncate(1);
                }
                if self.points.last().is_none_or(|l| dist(*l, *pos) >= 0.5) {
                    self.points.push(*pos);
                }
            }
            PointerKind::Up if self.active => {
                self.active = false;
                if ev.mods.shift {
                    self.points.truncate(1);
                }
                if self.points.last().is_none_or(|l| dist(*l, *pos) > 0.0) {
                    self.points.push(*pos);
                }
                let pts = std::mem::take(&mut self.points);
                let Some(c) = freehand_contour_json(&pts, ctx.options.path_precision, false) else { return ToolResponse::default() };
                // Pfad nachzeichnen: genau ein ausgewählter Pfad → dessen Konturen ersetzen
                if self.id == ToolId::RedrawPath
                    && ctx.selection.elements.len() == 1
                    && docops::element_on_page(ctx.doc, ctx.page, ctx.selection.elements[0]).is_some_and(|e| matches!(e.kind, ElementKind::Path(_)))
                {
                    return ToolResponse::commit(vec![Invocation::new("path.setContours", json!({"contours": [c]}))]);
                }
                return ToolResponse::commit(vec![super::styled(Invocation::new("draw.path", json!({"contours": [c], "fill": false})), ctx.options)]);
            }
            _ => {}
        }
        ToolResponse::with_overlay(self.overlay(), Cursor::Pen)
    }

    fn overlay(&self) -> Vec<Overlay> {
        if self.points.is_empty() { Vec::new() } else { vec![Overlay::Polyline { points: self.points.clone(), closed: false }] }
    }
}

/// Hotspot-Rechteck/-Kreis/-Polygon, Segment und Polygon-Segment.
#[derive(Clone, Debug)]
pub struct WebShapeTool {
    id: ToolId,
    drag: DragTracker,
    points: Vec<Point>,
    hover: Option<Point>,
    overlay: Vec<Overlay>,
}

impl WebShapeTool {
    /// Neues Werkzeug.
    pub fn new(id: ToolId) -> WebShapeTool {
        WebShapeTool { id, drag: DragTracker::new(), points: Vec::new(), hover: None, overlay: Vec::new() }
    }

    fn is_polygon(&self) -> bool {
        matches!(self.id, ToolId::HotspotPolygon | ToolId::PolygonSlice)
    }

    fn command(&self) -> &'static str {
        if matches!(self.id, ToolId::Slice | ToolId::PolygonSlice) { "web.addSlice" } else { "web.addHotspot" }
    }

    fn finish_polygon(&mut self) -> ToolResponse {
        let pts = std::mem::take(&mut self.points);
        self.overlay.clear();
        self.hover = None;
        if pts.len() < 3 {
            return ToolResponse::default();
        }
        ToolResponse::commit(vec![Invocation::new(self.command(), json!({"shape": "polygon", "points": pts.iter().map(|p| pt_json(*p)).collect::<Vec<_>>()}))])
    }

    fn poly_overlay(&self) -> Vec<Overlay> {
        let mut pts = self.points.clone();
        if let Some(h) = self.hover {
            pts.push(h);
        }
        if pts.is_empty() {
            return Vec::new();
        }
        vec![Overlay::Polyline { points: pts, closed: true }, Overlay::Handles { points: self.points.clone() }]
    }
}

impl ToolMachine for WebShapeTool {
    fn id(&self) -> ToolId {
        self.id
    }

    fn handle(&mut self, ev: &ToolEvent, ctx: &ToolContext) -> ToolResponse {
        if !ctx.has_document {
            return ToolResponse::default();
        }
        if self.is_polygon() {
            return match &ev.input {
                ToolInput::Pointer { kind: PointerKind::Down, pos } => {
                    let p = Snapper::new(ctx, &[]).point(*pos).point;
                    if self.points.len() >= 3 && dist(self.points[0], p) <= ctx.pick_tolerance() as f32 {
                        return self.finish_polygon();
                    }
                    if self.points.last().is_none_or(|l| dist(*l, p) > ctx.pick_tolerance() as f32) {
                        self.points.push(p);
                    }
                    self.overlay = self.poly_overlay();
                    ToolResponse::with_overlay(self.overlay.clone(), Cursor::Crosshair)
                }
                ToolInput::Pointer { kind: PointerKind::Move, pos } => {
                    if !self.points.is_empty() {
                        self.hover = Some(Snapper::new(ctx, &[]).point(*pos).point);
                        self.overlay = self.poly_overlay();
                    }
                    ToolResponse::with_overlay(self.overlay.clone(), Cursor::Crosshair)
                }
                ToolInput::Pointer { kind: PointerKind::DoubleClick, .. } | ToolInput::Key(ToolKey::Enter) => self.finish_polygon(),
                ToolInput::Key(ToolKey::Escape) => {
                    self.points.clear();
                    self.hover = None;
                    self.overlay.clear();
                    ToolResponse::default()
                }
                _ => ToolResponse::with_overlay(self.overlay.clone(), Cursor::Crosshair),
            };
        }
        let ToolInput::Pointer { kind, pos } = &ev.input else {
            if ev.input == ToolInput::Key(ToolKey::Escape) {
                self.drag.reset();
                self.overlay.clear();
            }
            return ToolResponse::default();
        };
        let sp = Snapper::new(ctx, &[]).point(*pos);
        if !self.drag.active() && *kind != PointerKind::Down {
            self.overlay = sp.guides;
            return ToolResponse::with_overlay(self.overlay.clone(), Cursor::Crosshair);
        }
        let d = self.drag.update(*kind, sp.point, ev, ctx.drag_threshold());
        let r = d.map(|(s, c)| constrain_rect(s, c, ev.mods.shift, ev.mods.alt));
        if *kind == PointerKind::Up {
            self.drag.reset();
            self.overlay.clear();
            return match r {
                Some(r) => {
                    let shape = if self.id == ToolId::HotspotCircle { "circle" } else { "rect" };
                    ToolResponse::commit(vec![Invocation::new(self.command(), json!({"shape": shape, "rect": rect_json(r)}))])
                }
                None => ToolResponse::default(),
            };
        }
        if let Some(r) = r {
            self.overlay = vec![
                if self.id == ToolId::HotspotCircle { Overlay::Ellipse { rect: r, dashed: false } } else { Overlay::Rect { rect: r, dashed: false } },
                size_label(r),
            ];
            self.overlay.extend(sp.guides);
        }
        ToolResponse::with_overlay(self.overlay.clone(), Cursor::Crosshair)
    }

    fn finish(&mut self, _ctx: &ToolContext) -> ToolResponse {
        if self.is_polygon() { self.finish_polygon() } else { ToolResponse::default() }
    }

    fn overlay(&self) -> Vec<Overlay> {
        self.overlay.clone()
    }
}
