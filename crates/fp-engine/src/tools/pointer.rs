//! Zeiger, Hintere auswählen, Teilauswahl, Skalieren/Neigen/Verzerren und Zuschneiden (10 §2.6).
//!
//! Rückmeldung wie das Original: rote Hervorhebung beim Überfahren (`SelectionMouseOverPreview`),
//! Pfadpunkte ausgewählter Pfade, Vorschau des gezogenen Objekts („Vorschau beim Ziehen“, als
//! [`Overlay::Ghost`]), intelligente Hilfslinien beim Verschieben, Größe beim Skalieren, Winkel beim
//! Drehen, Griff-Cursor je Griff.

use super::snap::Snapper;
use super::{
    Cursor, Invocation, Overlay, PointerKind, ToolContext, ToolEvent, ToolId, ToolInput, ToolKey,
    ToolMachine, ToolResponse, constrain_angle, dist, fmt_num, pt_json, rect_json, size_label,
};
use crate::docops::{self, HitOptions};
use crate::selection::PointRef;
use fp_geom::{Matrix3, Point, Rect};
use fp_model::traverse::ElementPath;
use fp_model::{Contour, Document, Element, ElementId, ElementKind};
use serde_json::json;

/// Die acht Griffe eines Rechtecks: 0 LO, 1 O, 2 RO, 3 R, 4 RU, 5 U, 6 LU, 7 L.
pub fn handles(r: &Rect) -> [Point; 8] {
    let (cx, cy) = ((r.left + r.right) / 2.0, (r.top + r.bottom) / 2.0);
    [
        Point::new(r.left, r.top),
        Point::new(cx, r.top),
        Point::new(r.right, r.top),
        Point::new(r.right, cy),
        Point::new(r.right, r.bottom),
        Point::new(cx, r.bottom),
        Point::new(r.left, r.bottom),
        Point::new(r.left, cy),
    ]
}

fn handle_at(r: &Rect, p: Point, tol: f32) -> Option<usize> {
    handles(r).iter().position(|h| dist(*h, p) <= tol)
}

/// Cursor für einen Griff (0/4 ↖↘, 2/6 ↗↙, 1/5 ↕, 3/7 ↔).
pub fn handle_cursor(h: usize) -> Cursor {
    match h {
        0 | 4 => Cursor::Resize,
        2 | 6 => Cursor::ResizeNeSw,
        1 | 5 => Cursor::ResizeV,
        _ => Cursor::ResizeH,
    }
}

/// Neues Rechteck beim Ziehen eines Griffs: Umschalt proportional (Ecken), Alt um die Mitte.
pub fn scale_by_handle(
    b: Rect,
    handle: usize,
    cur: Point,
    proportional: bool,
    from_center: bool,
) -> Rect {
    let (mut l, mut t, mut r, mut bo) = (b.left, b.top, b.right, b.bottom);
    let (cx, cy) = ((l + r) / 2.0, (t + bo) / 2.0);
    let moves_left = matches!(handle, 0 | 6 | 7);
    let moves_right = matches!(handle, 2..=4);
    let moves_top = matches!(handle, 0..=2);
    let moves_bottom = matches!(handle, 4..=6);
    if moves_left {
        l = cur.x;
    }
    if moves_right {
        r = cur.x;
    }
    if moves_top {
        t = cur.y;
    }
    if moves_bottom {
        bo = cur.y;
    }
    if from_center {
        if moves_left {
            r = 2.0 * cx - l;
        }
        if moves_right {
            l = 2.0 * cx - r;
        }
        if moves_top {
            bo = 2.0 * cy - t;
        }
        if moves_bottom {
            t = 2.0 * cy - bo;
        }
    }
    if proportional && matches!(handle, 0 | 2 | 4 | 6) && b.width() != 0.0 && b.height() != 0.0 {
        let sx = (r - l) / b.width();
        let sy = (bo - t) / b.height();
        let s = if sx.abs() > sy.abs() { sx } else { sy };
        let (w, h) = (
            b.width() * s.abs() * sx.signum(),
            b.height() * s.abs() * sy.signum(),
        );
        if from_center {
            l = cx - w / 2.0;
            r = cx + w / 2.0;
            t = cy - h / 2.0;
            bo = cy + h / 2.0;
        } else {
            if moves_left {
                l = r - w;
            } else {
                r = l + w;
            }
            if moves_top {
                t = bo - h;
            } else {
                bo = t + h;
            }
        }
    }
    Rect::new(l, t, r, bo)
}

/// Skalieren-Werkzeug am Eckgriff wie im Original (PAR 08/10, 08/16 gemessen): proportional mit der
/// **Projektion** des Zeigers auf die Diagonale (R1 50,50–150,120, Ecke → 200,140 ergibt 193,150;
/// B1 → 603,415), Alt von der Mitte aus; Seitengriffe ändern nur ihre Richtung.
pub fn scale_projected(b: Rect, handle: usize, cur: Point, from_center: bool) -> Rect {
    if !matches!(handle, 0 | 2 | 4 | 6) || b.width() == 0.0 || b.height() == 0.0 {
        return scale_by_handle(b, handle, cur, false, from_center);
    }
    let hs = handles(&b);
    let corner = hs[handle];
    let anchor = if from_center {
        b.center()
    } else {
        hs[(handle + 4) % 8]
    };
    let (dx, dy) = ((corner.x - anchor.x) as f64, (corner.y - anchor.y) as f64);
    let (vx, vy) = ((cur.x - anchor.x) as f64, (cur.y - anchor.y) as f64);
    let s = (vx * dx + vy * dy) / (dx * dx + dy * dy).max(1e-12);
    // die gezogene Ecke landet auf ganzen Pixeln (192.95,150.07 → 193,150; B1 603,415)
    let p = Point::new(
        (anchor.x as f64 + s * dx).round() as f32,
        (anchor.y as f64 + s * dy).round() as f32,
    );
    let o = if from_center {
        Point::new(2.0 * anchor.x - p.x, 2.0 * anchor.y - p.y)
    } else {
        anchor
    };
    match handle {
        0 => Rect::new(p.x, p.y, o.x, o.y),
        2 => Rect::new(o.x, p.y, p.x, o.y),
        4 => Rect::new(o.x, o.y, p.x, p.y),
        _ => Rect::new(p.x, o.y, o.x, p.y),
    }
}

fn matrix_json(m: &Matrix3) -> serde_json::Value {
    json!(m.to_affine())
}

// ------------------------------------------------------------------ gemeinsame Hilfen

fn collect_contours(e: &Element, out: &mut Vec<Contour>, depth: u32) {
    if depth > 16 || out.len() > 400 {
        return;
    }
    match &e.kind {
        ElementKind::Path(p) => out.extend(p.contours.iter().cloned()),
        ElementKind::Group(g) => {
            for c in &g.children {
                collect_contours(c, out, depth + 1);
            }
        }
        ElementKind::CompoundShape(c) => match &c.result {
            Some(r) => collect_contours(r, out, depth + 1),
            None => {
                for o in &c.operands {
                    collect_contours(o, out, depth + 1);
                }
            }
        },
        _ => {}
    }
}

/// Hervorhebung eines Elements: Pfadkonturen bzw. Rahmen.
pub(crate) fn outline_overlay(doc: &Document, e: &Element) -> Overlay {
    let mut cs = Vec::new();
    if !matches!(
        e.kind,
        ElementKind::Text(_) | ElementKind::Bitmap(_) | ElementKind::Instance(_)
    ) {
        collect_contours(e, &mut cs, 0);
    }
    if cs.is_empty() {
        Overlay::Highlight {
            contours: Vec::new(),
            rect: docops::element_frame_bounds(doc, e),
        }
    } else {
        Overlay::Highlight {
            contours: cs,
            rect: None,
        }
    }
}

/// Strichbreite eines Elements (für die Vorauswahl über die Grenzen).
fn stroke_margin(e: &Element) -> f32 {
    match &e.kind {
        ElementKind::Path(p) => p
            .attrs
            .brush
            .as_ref()
            .map(|b| b.diameter as f32 / 2.0 + 1.0)
            .unwrap_or(1.0),
        ElementKind::Group(_) | ElementKind::CompoundShape(_) => 32.0,
        _ => 1.0,
    }
}

/// Oberstes getroffenes Element (wie [`docops::hits_at`], aber mit Vorauswahl über die Grenzen,
/// damit das Überfahren auch in großen Dokumenten flüssig bleibt).
pub(crate) fn top_hit(ctx: &ToolContext, p: Point, opt: &HitOptions) -> Option<ElementPath> {
    use fp_model::traverse::PathStep;
    for (slot, l) in docops::own_layers(ctx.doc, ctx.page).into_iter().rev() {
        if opt.skip_hidden && !l.is_visible() {
            continue;
        }
        let ci = if l.shared_across_frames { 0 } else { ctx.frame };
        let Some(cel) = l.cels.get(ci) else { continue };
        if opt.skip_locked && cel.locked {
            continue;
        }
        for (i, e) in cel.elements.iter().enumerate() {
            if (opt.skip_locked && e.common.locked) || (opt.skip_hidden && !e.common.visible) {
                continue;
            }
            let m = stroke_margin(e) + opt.tolerance as f32;
            let near =
                docops::element_bounds(ctx.doc, e).is_none_or(|b| b.inflate(m, m).contains(p));
            if near && docops::hit_element(ctx.doc, e, p, ctx.geometry, opt) {
                // geteilte Ebenen haben eine eigene Wurzel (`RootRef::SharedLayer`)
                return Some(docops::slot_path(
                    ctx.doc,
                    ctx.page,
                    slot,
                    ci,
                    vec![PathStep::Child(i)],
                ));
            }
        }
    }
    None
}

/// Teil eines Pfadknotens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Part {
    Anchor,
    In,
    Out,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PointHit {
    id: ElementId,
    contour: usize,
    node: usize,
    part: Part,
    /// Lage des getroffenen Teils.
    at: Point,
}

/// Pfadpunkt bzw. Henkel ausgewählter Pfade unter `p`. Henkel nur bei unterausgewählten Punkten.
fn point_hit(
    ctx: &ToolContext,
    p: Point,
    allow_primitive: bool,
    handles: bool,
) -> Option<PointHit> {
    let tol = ctx.pick_tolerance() as f32;
    let htol = ctx.handle_tolerance().max(ctx.pick_tolerance() * 0.8) as f32;
    let mut best: Option<(f32, PointHit)> = None;
    for id in &ctx.selection.elements {
        let Some(e) = docops::element_on_page(ctx.doc, ctx.page, *id) else {
            continue;
        };
        let ElementKind::Path(pe) = &e.kind else {
            continue;
        };
        if pe.primitive.is_some() && !allow_primitive {
            continue;
        }
        for (ci, c) in pe.contours.iter().enumerate() {
            for (ni, n) in c.nodes.iter().enumerate() {
                let d = dist(n.anchor, p);
                if d <= tol && best.is_none_or(|(b, _)| d < b) {
                    best = Some((
                        d,
                        PointHit {
                            id: *id,
                            contour: ci,
                            node: ni,
                            part: Part::Anchor,
                            at: n.anchor,
                        },
                    ));
                }
                if handles
                    && ctx.selection.points.contains(&PointRef {
                        element: *id,
                        contour: ci,
                        node: ni,
                    })
                {
                    for (q, part) in [(n.pred, Part::In), (n.succ, Part::Out)] {
                        let d = dist(q, p);
                        if q != n.anchor && d <= htol && best.is_none_or(|(b, _)| d < b) {
                            best = Some((
                                d,
                                PointHit {
                                    id: *id,
                                    contour: ci,
                                    node: ni,
                                    part,
                                    at: q,
                                },
                            ));
                        }
                    }
                }
            }
        }
    }
    best.map(|(_, h)| h)
}

/// Pfadpunkt-Anzeige der ausgewählten Pfade.
fn path_points_feedback(ctx: &ToolContext, subselect: bool) -> Vec<Overlay> {
    let mut out = Vec::new();
    for id in ctx.selection.elements.iter().take(64) {
        let Some(e) = docops::element_on_page(ctx.doc, ctx.page, *id) else {
            continue;
        };
        let ElementKind::Path(pe) = &e.kind else {
            continue;
        };
        if let Some(pr) = pe.primitive.as_ref().filter(|_| !subselect) {
            // Rechteck-Grundform: nur die vier Eckpunkte (Ziehen = Rundung)
            let q = pr
                .rect
                .normalized()
                .corners()
                .map(|c| pr.transform.apply(c));
            out.push(Overlay::PathPoints {
                contours: vec![crate::geometry::polygon_contour(&q, true)],
                selected: Vec::new(),
                handles: false,
            });
            continue;
        }
        let selected: Vec<(usize, usize)> = if subselect {
            ctx.selection
                .points
                .iter()
                .filter(|r| r.element == *id)
                .map(|r| (r.contour, r.node))
                .collect()
        } else {
            Vec::new()
        };
        out.push(Overlay::PathPoints {
            contours: pe.contours.clone(),
            selected,
            handles: subselect,
        });
    }
    out
}

fn ids_json(ids: &[ElementId]) -> serde_json::Value {
    json!(ids.iter().map(|i| i.0).collect::<Vec<_>>())
}

/// Vorschau für das Abbilden der Auswahl von `rect` auf `quad`.
fn ghost(ctx: &ToolContext, rect: Rect, quad: [Point; 4], copy: bool) -> Overlay {
    Overlay::Ghost {
        ids: ctx.selection.elements.iter().map(|i| i.0).collect(),
        rect,
        quad,
        copy,
    }
}

// ------------------------------------------------------------------ Zeiger

#[derive(Clone, Debug, PartialEq)]
enum PState {
    Idle,
    /// Gedrückt auf einem (dann ausgewählten) Element; `toggle`: beim Loslassen ohne Ziehen
    /// abwählen (Umschalt auf gewähltem Objekt); `fresh`: erst mit diesem Druck gewählt.
    PressedObject {
        start: Point,
        copy: bool,
        toggle: Option<ElementId>,
        fresh: bool,
    },
    PressedEmpty {
        start: Point,
        add: bool,
    },
    PressedHandle {
        start: Point,
        handle: usize,
        bounds: Rect,
    },
    PressedPoint {
        start: Point,
        hit: PointHit,
        pull: bool,
    },
    Moving {
        start: Point,
        cur: Point,
        copy: bool,
        bounds: Option<Rect>,
        guides: Vec<Overlay>,
        fresh: bool,
    },
    Rubber {
        start: Point,
        cur: Point,
        add: bool,
    },
    Scaling {
        handle: usize,
        bounds: Rect,
        cur: Rect,
    },
    DraggingPoint {
        start: Point,
        cur: Point,
        hit: PointHit,
        pull: bool,
        guides: Vec<Overlay>,
    },
    /// Steuerpunkt einer automatischen Form wird gezogen.
    Control(ControlDrag),
    /// Verlaufs-/Mustergriff des gewählten Objekts wird gezogen (BA-043).
    FillHandle {
        id: ElementId,
        handles: fp_model::FillHandles,
        which: FillGrip,
        from: Point,
        cur: Point,
    },
}

/// Griff einer Verlaufs- bzw. Musterfüllung, den der Zeiger zieht (BA-043).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum FillGrip {
    /// Anfang (runder Griff)
    Start,
    /// Ende (eckiger Griff)
    End,
    /// zweite Achse (Ellipse, Rechteck, Muster)
    Second,
}

/// Verlaufs-/Mustergriff des einzigen gewählten Pfads bzw. Texts unter `p` – dieselben Griffe, die
/// die Leinwand beim Zeiger zeichnet (`fill_handles`, PAR 18/07).
pub(crate) fn fill_grip_at(
    ctx: &ToolContext,
    p: Point,
    tol: f32,
) -> Option<(ElementId, fp_model::FillHandles, FillGrip)> {
    let [id] = ctx.selection.elements.as_slice() else {
        return None;
    };
    let e = docops::element_on_page(ctx.doc, ctx.page, *id)?;
    if !matches!(e.kind, ElementKind::Path(_) | ElementKind::Text(_)) {
        return None;
    }
    let attrs = docops::path_attrs_of(&e)?;
    let f = attrs.fill.as_ref()?;
    let kind = crate::commands::panel::fill_kind(Some(f));
    if kind != 2 && kind != 3 {
        return None;
    }
    let h = crate::commands::panel::handles_of(&e).filter(|h| h.valid)?;
    let second = kind == 3 || matches!(f.shape, 9 | 10);
    let tol = tol.max(4.0 / ctx.zoom.max(1e-6) as f32);
    let mut grips = vec![(FillGrip::Start, h.start), (FillGrip::End, h.end)];
    if second {
        grips.push((FillGrip::Second, h.second));
    }
    grips
        .into_iter()
        .filter(|g| dist(g.1, p) <= tol)
        .min_by(|a, b| dist(a.1, p).total_cmp(&dist(b.1, p)))
        .map(|g| (*id, h, g.0))
}

/// Griffe nach dem Ziehen des Griffs `which` um `from` → `to` (nur dieser Griff bewegt sich, wie
/// beim Füll- und Verlaufswerkzeug).
pub(crate) fn dragged_fill_handles(
    h: fp_model::FillHandles,
    which: FillGrip,
    from: Point,
    to: Point,
) -> fp_model::FillHandles {
    let (dx, dy) = (to.x - from.x, to.y - from.y);
    let mv = |q: Point| Point::new(q.x + dx, q.y + dy);
    let mut n = h;
    match which {
        FillGrip::Start => n.start = mv(h.start),
        FillGrip::End => n.end = mv(h.end),
        FillGrip::Second => n.second = mv(h.second),
    }
    n
}

/// Sichtbare Steuerpunkte der ausgewählten automatischen Formen: (Form, Index, Lage, Tooltip).
pub(crate) fn control_points(ctx: &ToolContext) -> Vec<(ElementId, usize, Point, Option<String>)> {
    let mut out = Vec::new();
    for id in ctx.selection.elements.iter().take(64) {
        let Some(e) = docops::element_on_page(ctx.doc, ctx.page, *id) else {
            continue;
        };
        let ElementKind::Group(g) = &e.kind else {
            continue;
        };
        let Some(ss) = &g.smart_shape else { continue };
        for (i, cp) in ss.control_points.iter().flatten().enumerate() {
            if cp.visible {
                out.push((
                    *id,
                    i,
                    cp.position,
                    cp.tooltip.as_deref().and_then(tooltip_text),
                ));
            }
        }
    }
    out
}

/// Anzeigetext eines Steuerpunkt-Tooltips: leer → keiner; die Texte der Original-Formen in der
/// Sprache der Oberfläche (im Dokument steht wie im Skript der deutsche Text; TW-90).
fn tooltip_text(tip: &str) -> Option<String> {
    let key = match tip {
        "" => return None,
        "Spirale öffnen/schließen" => "autoshape-tip-spiral-toggle",
        "Klicken, um zwischen den Ecken umzuschalten" => "autoshape-tip-corner-toggle",
        _ => return Some(tip.to_string()),
    };
    Some(fp_i18n::t(key))
}

/// Rückmeldung beim Überfahren: alle Steuerpunkte, der unter `p` hervorgehoben mit Tooltip (TW-90,
/// das Original zeigt den Tooltip schon beim Darüberfahren).
pub(crate) fn control_points_hover(ctx: &ToolContext, p: Point, tol: f32) -> Option<Vec<Overlay>> {
    control_point_at(ctx, p, tol).map(|h| control_points_feedback(ctx, Some(h)))
}

/// Steuerpunkt einer ausgewählten automatischen Form unter `p`.
pub(crate) fn control_point_at(
    ctx: &ToolContext,
    p: Point,
    tol: f32,
) -> Option<(ElementId, usize, Point)> {
    control_points(ctx)
        .into_iter()
        .filter(|c| dist(c.2, p) <= tol.max(4.0 / ctx.zoom.max(1e-6) as f32))
        .min_by(|a, b| dist(a.2, p).total_cmp(&dist(b.2, p)))
        .map(|c| (c.0, c.1, c.2))
}

/// Rückmeldung: Steuerpunkte als Rauten (der gezogene an der Zeigerposition).
pub(crate) fn control_points_feedback(
    ctx: &ToolContext,
    dragging: Option<(ElementId, usize, Point)>,
) -> Vec<Overlay> {
    control_points(ctx)
        .into_iter()
        .map(|(id, i, at, tooltip)| match dragging {
            Some((did, di, cur)) if did == id && di == i => Overlay::ControlPoint {
                at: cur,
                tooltip,
                hot: true,
            },
            _ => Overlay::ControlPoint {
                at,
                tooltip,
                hot: false,
            },
        })
        .collect()
}

/// Laufendes Ziehen eines Steuerpunkts (Zeiger und Werkzeug „Automatische Form“).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ControlDrag {
    pub id: ElementId,
    pub index: usize,
    pub from: Point,
    pub points: Vec<Point>,
}

impl ControlDrag {
    /// Befehl beim Loslassen.
    pub fn finish(&self, to: Point, ev: &ToolEvent) -> Option<Invocation> {
        if dist(self.from, to) < 1e-3 && self.points.len() <= 1 {
            return None;
        }
        let pts: Vec<serde_json::Value> = self.points.iter().map(|p| json!([p.x, p.y])).collect();
        Some(Invocation::new(
            "autoShape.dragControlPoint",
            json!({"id": self.id.0, "index": self.index, "from": [self.from.x, self.from.y], "to": [to.x, to.y], "points": pts, "shift": ev.mods.shift, "alt": ev.mods.alt, "ctrl": ev.mods.ctrl}),
        ))
    }
}

/// Auswahl durch das Werkzeug (kein Verlaufsschritt, `tool.pick`).
fn pick(command: &str, params: serde_json::Value) -> Invocation {
    Invocation::new("tool.pick", json!({"command": command, "params": params}))
}

/// Zeiger, Hintere auswählen und Teilauswahl.
#[derive(Clone, Debug)]
pub struct PointerTool {
    id: ToolId,
    state: PState,
    overlay: Vec<Overlay>,
    /// Hervorhebung beim Überfahren.
    hover: Option<Overlay>,
    hover_cursor: Cursor,
}

impl PointerTool {
    /// Neues Werkzeug (Pointer, SelectBehind oder Subselection).
    pub fn new(id: ToolId) -> PointerTool {
        PointerTool {
            id,
            state: PState::Idle,
            overlay: Vec::new(),
            hover: None,
            hover_cursor: Cursor::Arrow,
        }
    }

    fn idle_cursor(&self) -> Cursor {
        match self.id {
            ToolId::Subselection => Cursor::Subselect,
            ToolId::SelectBehind => Cursor::SelectBehind,
            _ => Cursor::Arrow,
        }
    }

    fn selection_bounds(ctx: &ToolContext) -> Option<Rect> {
        docops::ids_frame_bounds(ctx.doc, ctx.page, &ctx.selection.elements)
    }

    /// Ist unter der Auswahl ein gewöhnlicher Pfad (keine Rechteck-Grundform)?
    fn has_plain_path(ctx: &ToolContext) -> bool {
        ctx.selection.elements.iter().any(|id| {
            docops::element_on_page(ctx.doc, ctx.page, *id)
                .is_some_and(|e| matches!(&e.kind, ElementKind::Path(p) if p.primitive.is_none()))
        })
    }

    /// Ist das Element eine Rechteck-Grundform?
    fn is_rect_primitive(ctx: &ToolContext, id: ElementId) -> bool {
        docops::element_on_page(ctx.doc, ctx.page, id)
            .is_some_and(|e| matches!(&e.kind, ElementKind::Path(p) if p.primitive.is_some()))
    }

    fn hit_opts(ctx: &ToolContext) -> HitOptions {
        HitOptions {
            tolerance: ctx.pick_tolerance(),
            skip_locked: true,
            skip_hidden: true,
        }
    }

    fn paint_handles(&self, ctx: &ToolContext) -> bool {
        self.id == ToolId::Pointer && ctx.prefs.get_bool("PointerToolPaintHandles", true)
    }

    /// Überfahren ohne gedrückte Taste: Hervorhebung und Cursor.
    fn hover(&mut self, p: Point, ctx: &ToolContext) -> ToolResponse {
        self.hover = None;
        self.hover_cursor = self.idle_cursor();
        let tol = ctx.pick_tolerance() as f32;
        if self.id != ToolId::SelectBehind
            && let Some(o) = control_points_hover(ctx, p, tol)
        {
            self.hover_cursor = Cursor::PointerPoint;
            // nur die hervorgehobene Raute mit Tooltip; die übrigen zeichnet `feedback`
            self.hover = o
                .into_iter()
                .find(|x| matches!(x, Overlay::ControlPoint { hot: true, .. }));
            return ToolResponse {
                overlay: self.hover.iter().cloned().collect(),
                cursor: self.hover_cursor,
                ..ToolResponse::default()
            };
        }
        if self.id != ToolId::Subselection && fill_grip_at(ctx, p, tol).is_some() {
            self.hover_cursor = Cursor::Move;
            return ToolResponse {
                cursor: self.hover_cursor,
                ..ToolResponse::default()
            };
        }
        // Über den Griffen der Auswahl zeigt das Original einen Pfeil ohne Zusatz: über den Ecken
        // weiß mit schwarzem Rand (K2, wie die Teilauswahl), über den Kanten schwarz (K3)
        if self.paint_handles(ctx)
            && let Some(b) = Self::selection_bounds(ctx)
            && let Some(h) = handle_at(&b, p, tol)
            && !(h % 2 == 0 && Self::has_plain_path(ctx))
        {
            if h % 2 == 0 && self.hover_cursor == Cursor::Arrow {
                self.hover_cursor = Cursor::Subselect;
            }
            return ToolResponse {
                cursor: self.hover_cursor,
                ..ToolResponse::default()
            };
        }
        if self.id != ToolId::SelectBehind
            && point_hit(ctx, p, false, self.id == ToolId::Subselection).is_some()
        {
            self.hover_cursor = Cursor::PointerPoint;
            return ToolResponse {
                cursor: self.hover_cursor,
                ..ToolResponse::default()
            };
        }
        let opt = Self::hit_opts(ctx);
        if let Some(path) = top_hit(ctx, p, &opt) {
            let path = if self.id == ToolId::Subselection {
                docops::deep_hit(ctx.doc, &path, p, ctx.geometry, &opt)
            } else {
                path
            };
            if let Some(e) = ctx.doc.element_at(&path) {
                let selected = ctx.selection.contains(e.id);
                if !selected && ctx.prefs.get_bool("SelectionMouseOverPreview", true) {
                    self.hover = Some(outline_overlay(ctx.doc, e));
                }
                if self.id == ToolId::Subselection
                    && matches!(e.kind, ElementKind::Path(_))
                    && selected
                {
                    self.hover_cursor = Cursor::PointerSegment;
                }
            }
        }
        ToolResponse {
            overlay: self.hover.iter().cloned().collect(),
            cursor: self.hover_cursor,
            ..ToolResponse::default()
        }
    }

    fn down(&mut self, p: Point, ev: &ToolEvent, ctx: &ToolContext) -> ToolResponse {
        self.hover = None;
        let tol = ctx.pick_tolerance() as f32;
        // Steuerpunkte automatischer Formen (gelbe Rauten)
        if self.id != ToolId::SelectBehind
            && let Some((id, index, at)) = control_point_at(ctx, p, tol)
        {
            self.state = PState::Control(ControlDrag {
                id,
                index,
                from: at,
                points: vec![at],
            });
            return ToolResponse {
                overlay: control_points_feedback(ctx, Some((id, index, at))),
                cursor: Cursor::PointerPoint,
                ..ToolResponse::default()
            };
        }
        // Verlaufs-/Mustergriffe des gewählten Objekts (BA-043; die Leinwand zeichnet sie bei Zeiger
        // und Hintere auswählen)
        if self.id != ToolId::Subselection
            && let Some((id, handles, which)) = fill_grip_at(ctx, p, tol)
        {
            self.state = PState::FillHandle {
                id,
                handles,
                which,
                from: p,
                cur: p,
            };
            return ToolResponse {
                cursor: Cursor::Move,
                ..ToolResponse::default()
            };
        }
        // Griffe der Auswahl (nur Zeiger, PointerToolPaintHandles); auch die Eckgriffe eines
        // Rechtecks skalieren – im Original live gemessen, die Rundung bleibt (PARITAET_A L26)
        // Am Eckpunkt eines gewöhnlichen Pfads verschiebt der Zeiger dagegen das Objekt (PAR 08/04)
        if self.paint_handles(ctx)
            && let Some(b) = Self::selection_bounds(ctx)
            && let Some(h) = handle_at(&b, p, tol)
            && !(h % 2 == 0 && Self::has_plain_path(ctx))
        {
            self.state = PState::PressedHandle {
                start: p,
                handle: h,
                bounds: b,
            };
            return ToolResponse {
                cursor: handle_cursor(h),
                ..ToolResponse::default()
            };
        }
        // Pfadpunkte ausgewählter Pfade (Zeiger: Anker; Teilauswahl: Anker und Henkel). Die
        // Eckpunkte einer Rechteck-Grundform verschieben mit der Teilauswahl das ganze Rechteck
        // (PAR 10/06), sie sind hier also keine Pfadpunkte.
        if self.id != ToolId::SelectBehind
            && let Some(hit) = point_hit(ctx, p, false, self.id == ToolId::Subselection)
        {
            let pull = self.id == ToolId::Subselection && ev.mods.alt && hit.part == Part::Anchor;
            self.state = PState::PressedPoint {
                start: p,
                hit,
                pull,
            };
            let mut cmds = Vec::new();
            if self.id == ToolId::Subselection && hit.part == Part::Anchor {
                let r = PointRef {
                    element: hit.id,
                    contour: hit.contour,
                    node: hit.node,
                };
                let already = ctx.selection.points.contains(&r);
                if ev.mods.shift || !already {
                    let mode = match (ev.mods.shift, already) {
                        (true, true) => "subtract",
                        (true, false) => "add",
                        _ => "replace",
                    };
                    cmds.push(pick("select.points", json!({"points": [{"element": hit.id.0, "contour": hit.contour, "node": hit.node}], "mode": mode})));
                }
            }
            return ToolResponse {
                commands: cmds,
                cursor: Cursor::PointerPoint,
                ..ToolResponse::default()
            };
        }
        let opt = Self::hit_opts(ctx);
        let mut target = if self.id == ToolId::SelectBehind {
            let hits = docops::hits_at(ctx.doc, ctx.page, ctx.frame, p, ctx.geometry, &opt);
            let mut t = hits.first().cloned();
            if hits.len() > 1
                && let Some(i) = hits.iter().position(|h| {
                    ctx.doc
                        .element_at(h)
                        .is_some_and(|e| ctx.selection.contains(e.id))
                })
            {
                // wiederholter Klick wählt das jeweils darunterliegende Objekt
                t = hits.get((i + 1) % hits.len()).cloned();
            }
            t
        } else {
            top_hit(ctx, p, &opt)
        };
        if self.id == ToolId::Subselection
            && let Some(t) = &target
        {
            target = Some(docops::deep_hit(ctx.doc, t, p, ctx.geometry, &opt));
        }
        match target.and_then(|t| ctx.doc.element_at(&t).map(|e| e.id)) {
            Some(id) => {
                let already = ctx.selection.contains(id);
                // Umschalt auf einem gewählten Objekt: Abwählen erst beim Loslassen ohne Ziehen –
                // Umschalt-Ziehen verschiebt waagrecht/45° (PAR 08/03)
                let toggle_on_up = ev.mods.shift && already;
                self.state = PState::PressedObject {
                    start: p,
                    copy: ev.mods.alt,
                    toggle: toggle_on_up.then_some(id),
                    fresh: !already,
                };
                let cmds = if toggle_on_up {
                    vec![]
                } else if ev.mods.shift {
                    vec![pick("select.set", json!({"ids": [id.0], "mode": "toggle"}))]
                } else if !already {
                    vec![pick(
                        "select.set",
                        json!({"ids": [id.0], "mode": "replace"}),
                    )]
                } else {
                    vec![]
                };
                ToolResponse {
                    commands: cmds,
                    cursor: if ev.mods.alt {
                        Cursor::Copy
                    } else {
                        Cursor::Move
                    },
                    ..ToolResponse::default()
                }
            }
            None => {
                self.state = PState::PressedEmpty {
                    start: p,
                    add: ev.mods.shift,
                };
                ToolResponse {
                    cursor: self.idle_cursor(),
                    ..ToolResponse::default()
                }
            }
        }
    }

    fn moved(&mut self, p: Point, ev: &ToolEvent, ctx: &ToolContext) -> ToolResponse {
        let thr = ctx.drag_threshold();
        let st = self.state.clone();
        match st {
            PState::Idle => self.hover(p, ctx),
            PState::PressedObject {
                start, copy, fresh, ..
            } if dist(start, p) >= thr => {
                self.state = PState::Moving {
                    start,
                    cur: p,
                    copy: copy || ev.mods.alt,
                    bounds: Self::selection_bounds(ctx),
                    guides: Vec::new(),
                    fresh,
                };
                self.moved(p, ev, ctx)
            }
            // Zeiger am Punkt eines gewählten Pfads: Ziehen verschiebt das ganze Objekt (PAR 08/04)
            PState::PressedPoint { start, .. }
                if self.id == ToolId::Pointer && dist(start, p) >= thr =>
            {
                self.state = PState::Moving {
                    start,
                    cur: p,
                    copy: ev.mods.alt,
                    bounds: Self::selection_bounds(ctx),
                    guides: Vec::new(),
                    fresh: false,
                };
                self.moved(p, ev, ctx)
            }
            PState::PressedEmpty { start, add } if dist(start, p) >= thr => {
                self.state = PState::Rubber { start, cur: p, add };
                self.moved(p, ev, ctx)
            }
            PState::PressedHandle {
                start,
                handle,
                bounds,
            } if dist(start, p) >= thr => {
                self.state = PState::Scaling {
                    handle,
                    bounds,
                    cur: bounds,
                };
                self.moved(p, ev, ctx)
            }
            PState::FillHandle {
                id,
                handles,
                which,
                from,
                ..
            } => {
                // re 40 §1: mit dem Zeiger ändert sich die Füllung live, die Griffslinie folgt dem
                // Zeiger, keine zusätzliche Vorschaulinie (anders als das Füllwerkzeug)
                let n = dragged_fill_handles(handles, which, from, p);
                self.overlay = vec![Overlay::FillLive {
                    target: id.0,
                    handles: n,
                }];
                self.state = PState::FillHandle {
                    id,
                    handles,
                    which,
                    from,
                    cur: p,
                };
                ToolResponse::with_overlay(self.overlay.clone(), Cursor::Move)
            }
            PState::Control(mut c) => {
                c.points.push(p);
                let tip = control_points(ctx)
                    .into_iter()
                    .find(|x| x.0 == c.id && x.1 == c.index)
                    .and_then(|x| x.3);
                self.overlay = control_points_feedback(ctx, Some((c.id, c.index, p)));
                if let Some(t) = tip {
                    self.overlay.push(Overlay::Label { at: p, text: t });
                }
                self.state = PState::Control(c);
                ToolResponse::with_overlay(self.overlay.clone(), Cursor::PointerPoint)
            }
            PState::PressedPoint { start, hit, pull } if dist(start, p) >= thr => {
                self.state = PState::DraggingPoint {
                    start,
                    cur: p,
                    hit,
                    pull,
                    guides: Vec::new(),
                };
                self.moved(p, ev, ctx)
            }
            PState::Moving {
                start,
                copy,
                bounds,
                fresh,
                ..
            } => {
                let copy = copy || ev.mods.alt;
                let mut cur = if ev.mods.shift {
                    constrain_angle(start, p, 45.0)
                } else {
                    p
                };
                let mut guides = Vec::new();
                if let Some(b) = bounds
                    && !ev.mods.shift
                {
                    let snap = Snapper::new(ctx, &ctx.selection.elements);
                    // re 41 §1.2: Zeigerpunkte mit floor auf ganze Pixel, Fang über den Gesamtzug
                    let (rx, ry) = snap.move_delta(start, cur);
                    let (dx, dy, g) = snap.move_rect(b.translate(rx, ry), rx, ry);
                    cur = Point::new(start.x + rx + dx, start.y + ry + dy);
                    guides = g;
                }
                self.state = PState::Moving {
                    start,
                    cur,
                    copy,
                    bounds,
                    guides: guides.clone(),
                    fresh,
                };
                self.overlay = Vec::new();
                if let Some(b) = bounds {
                    let q = b.translate(cur.x - start.x, cur.y - start.y);
                    self.overlay.push(ghost(ctx, b, q.corners(), copy));
                }
                self.overlay.extend(guides);
                ToolResponse::with_overlay(
                    self.overlay.clone(),
                    if copy { Cursor::Copy } else { Cursor::Move },
                )
            }
            PState::Rubber { start, add, .. } => {
                self.state = PState::Rubber { start, cur: p, add };
                self.overlay = vec![Overlay::Rect {
                    rect: Rect::new(start.x, start.y, p.x, p.y).normalized(),
                    dashed: true,
                }];
                ToolResponse::with_overlay(self.overlay.clone(), self.idle_cursor())
            }
            PState::Scaling { handle, bounds, .. } => {
                let sp = Snapper::new(ctx, &ctx.selection.elements).point(p);
                let r = scale_by_handle(bounds, handle, sp.point, ev.mods.shift, ev.mods.alt);
                self.state = PState::Scaling {
                    handle,
                    bounds,
                    cur: r,
                };
                let rn = r.normalized();
                self.overlay = vec![ghost(
                    ctx,
                    bounds,
                    [
                        Point::new(r.left, r.top),
                        Point::new(r.right, r.top),
                        Point::new(r.right, r.bottom),
                        Point::new(r.left, r.bottom),
                    ],
                    false,
                )];
                self.overlay.push(Overlay::Rect {
                    rect: rn,
                    dashed: false,
                });
                self.overlay.push(Overlay::Handles {
                    points: handles(&rn).to_vec(),
                });
                self.overlay.push(size_label(rn));
                self.overlay.extend(sp.guides);
                ToolResponse::with_overlay(self.overlay.clone(), handle_cursor(handle))
            }
            PState::DraggingPoint {
                start, hit, pull, ..
            } => {
                let sp = Snapper::new(ctx, &[hit.id]).point(Point::new(
                    hit.at.x + p.x - start.x,
                    hit.at.y + p.y - start.y,
                ));
                let target = if ev.mods.shift && hit.part != Part::Anchor {
                    // Henkel auf 45° zum Anker einrasten
                    self.anchor_of(ctx, &hit)
                        .map(|a| constrain_angle(a, sp.point, 45.0))
                        .unwrap_or(sp.point)
                } else {
                    sp.point
                };
                let cur = Point::new(start.x + target.x - hit.at.x, start.y + target.y - hit.at.y);
                self.state = PState::DraggingPoint {
                    start,
                    cur,
                    hit,
                    pull,
                    guides: sp.guides.clone(),
                };
                self.overlay = self.point_drag_overlay(
                    ctx,
                    &hit,
                    cur.x - start.x,
                    cur.y - start.y,
                    pull,
                    ev.mods.alt,
                );
                self.overlay.extend(sp.guides);
                ToolResponse::with_overlay(self.overlay.clone(), Cursor::PointerPoint)
            }
            _ => ToolResponse::default(),
        }
    }

    fn anchor_of(&self, ctx: &ToolContext, hit: &PointHit) -> Option<Point> {
        let e = docops::element_on_page(ctx.doc, ctx.page, hit.id)?;
        let ElementKind::Path(pe) = &e.kind else {
            return None;
        };
        pe.contours
            .get(hit.contour)?
            .nodes
            .get(hit.node)
            .map(|n| n.anchor)
    }

    /// Konturen des gezogenen Pfads nach dem Ziehen eines Punkts bzw. Henkels.
    fn dragged_contours(
        &self,
        ctx: &ToolContext,
        hit: &PointHit,
        dx: f32,
        dy: f32,
        pull: bool,
        alt: bool,
    ) -> Option<Vec<Contour>> {
        let e = docops::element_on_page(ctx.doc, ctx.page, hit.id)?;
        let ElementKind::Path(pe) = &e.kind else {
            return None;
        };
        let mut cs = pe.contours.clone();
        // mehrere unterausgewählte Punkte desselben Pfads wandern gemeinsam
        let multi: Vec<PointRef> =
            if self.id == ToolId::Subselection && hit.part == Part::Anchor && !pull {
                ctx.selection
                    .points
                    .iter()
                    .filter(|r| r.element == hit.id)
                    .copied()
                    .collect()
            } else {
                Vec::new()
            };
        let mv = |q: &mut Point| {
            q.x += dx;
            q.y += dy;
        };
        if multi.len() > 1
            && multi.contains(&PointRef {
                element: hit.id,
                contour: hit.contour,
                node: hit.node,
            })
        {
            for r in multi {
                if let Some(n) = cs.get_mut(r.contour).and_then(|c| c.nodes.get_mut(r.node)) {
                    mv(&mut n.anchor);
                    mv(&mut n.pred);
                    mv(&mut n.succ);
                }
            }
            return Some(cs);
        }
        let n = cs.get_mut(hit.contour)?.nodes.get_mut(hit.node)?;
        match (hit.part, pull) {
            (Part::Anchor, true) => {
                // Alt-Ziehen am Anker zieht wie im Original nur den Ausgangshenkel heraus; der
                // Eingangshenkel bleibt, wo er war (PARITAET_A L33: Eckpunkt 320,140 → 350,140 ergibt
                // pred 320,140 | succ 350,140, nicht symmetrisch)
                n.succ = Point::new(n.anchor.x + dx, n.anchor.y + dy);
                n.curve = true;
            }
            (Part::Anchor, false) => {
                mv(&mut n.anchor);
                mv(&mut n.pred);
                mv(&mut n.succ);
            }
            (part, _) => {
                let a = n.anchor;
                let (moved, other) = if part == Part::Out {
                    (&mut n.succ, n.pred)
                } else {
                    (&mut n.pred, n.succ)
                };
                let old = *moved;
                moved.x += dx;
                moved.y += dy;
                let newp = *moved;
                // glatte Knoten bleiben glatt (Gegenhenkel auf der Gegenrichtung, Länge bleibt); Alt bricht
                let smooth = other != a && {
                    let (u, v) = (
                        Point::new(old.x - a.x, old.y - a.y),
                        Point::new(other.x - a.x, other.y - a.y),
                    );
                    let c = u.x * v.y - u.y * v.x;
                    let lu = u.x.hypot(u.y) * v.x.hypot(v.y);
                    lu > 0.0 && (c / lu).abs() < 0.02 && (u.x * v.x + u.y * v.y) < 0.0
                };
                if smooth && !alt {
                    let len = (other.x - a.x).hypot(other.y - a.y);
                    let d = (newp.x - a.x).hypot(newp.y - a.y).max(1e-6);
                    let o = Point::new(
                        a.x - (newp.x - a.x) / d * len,
                        a.y - (newp.y - a.y) / d * len,
                    );
                    if part == Part::Out {
                        n.pred = o;
                    } else {
                        n.succ = o;
                    }
                }
                n.curve = true;
            }
        }
        Some(cs)
    }

    fn point_drag_overlay(
        &self,
        ctx: &ToolContext,
        hit: &PointHit,
        dx: f32,
        dy: f32,
        pull: bool,
        alt: bool,
    ) -> Vec<Overlay> {
        let Some(cs) = self.dragged_contours(ctx, hit, dx, dy, pull, alt) else {
            return Vec::new();
        };
        vec![
            Overlay::Path {
                contours: cs.clone(),
            },
            Overlay::PathPoints {
                contours: cs,
                selected: vec![(hit.contour, hit.node)],
                handles: self.id == ToolId::Subselection || pull,
            },
        ]
    }

    fn up(&mut self, p: Point, ev: &ToolEvent, ctx: &ToolContext) -> ToolResponse {
        let _ = self.moved(p, ev, ctx);
        let st = std::mem::replace(&mut self.state, PState::Idle);
        self.overlay.clear();
        match st {
            // ein Verlaufsschritt; ein Klick ohne Ziehen ändert nichts
            PState::FillHandle {
                id,
                handles,
                which,
                from,
                cur,
            } => {
                if dist(from, cur) < ctx.drag_threshold() {
                    return ToolResponse::default();
                }
                let n = dragged_fill_handles(handles, which, from, cur);
                ToolResponse::commit(vec![Invocation::new(
                    "element.setFillHandles",
                    json!({"ids": [id.0], "start": pt_json(n.start), "end": pt_json(n.end), "second": pt_json(n.second)}),
                )])
            }
            PState::Control(c) => match c.finish(p, ev) {
                Some(inv) => ToolResponse::commit(vec![inv]),
                None => ToolResponse::default(),
            },
            PState::Moving {
                start, cur, copy, ..
            } => {
                let (dx, dy) = (cur.x - start.x, cur.y - start.y);
                // Umschalt-Klick hat das Objekt gerade abgewählt: nichts verschieben
                if ctx.selection.elements.is_empty() || (dx == 0.0 && dy == 0.0 && !copy) {
                    return ToolResponse::default();
                }
                // Ziehen im Auswahlrahmen einer Bitmap verschiebt die ausgewählten Pixel (PAR 06/16)
                if !copy
                    && ctx.selection.has_pixels()
                    && ctx
                        .selection
                        .pixels
                        .as_ref()
                        .is_some_and(|m| m.get(start.x.floor() as i32, start.y.floor() as i32) > 0)
                {
                    return ToolResponse::commit(vec![Invocation::new(
                        "select.movePixels",
                        json!({"dx": dx.round(), "dy": dy.round()}),
                    )]);
                }
                // Alt-Ziehen heißt im Original „Duplizieren“ (PAR 08/03)
                let params = if copy {
                    json!({"dx": dx, "dy": dy, "copy": true, "step": "duplicate"})
                } else {
                    json!({"dx": dx, "dy": dy, "copy": false})
                };
                ToolResponse::commit(vec![Invocation::new("edit.move", params)])
            }
            PState::PressedObject {
                toggle: Some(id), ..
            } => ToolResponse::commit(vec![pick(
                "select.set",
                json!({"ids": [id.0], "mode": "toggle"}),
            )]),
            PState::Rubber { start, cur, add } => {
                let r = Rect::new(start.x, start.y, cur.x, cur.y).normalized();
                if self.id == ToolId::Subselection {
                    // Teilauswahl: Punkte der (getroffenen) Pfade im Rechteck
                    let pts = points_in_rect(ctx, r);
                    if !pts.is_empty() {
                        let pj = |l: &[&(ElementId, usize, usize)]| -> Vec<serde_json::Value> {
                            l.iter()
                                .map(|(e, c, n)| json!({"element": e.0, "contour": c, "node": n}))
                                .collect()
                        };
                        let ids: Vec<ElementId> = {
                            let mut v: Vec<ElementId> = pts.iter().map(|(e, _, _)| *e).collect();
                            v.dedup();
                            v
                        };
                        let all: Vec<&(ElementId, usize, usize)> = pts.iter().collect();
                        if !add {
                            return ToolResponse::commit(vec![
                                pick(
                                    "select.set",
                                    json!({"ids": ids_json(&ids), "mode": "replace"}),
                                ),
                                pick(
                                    "select.points",
                                    json!({"points": pj(&all), "mode": "replace"}),
                                ),
                            ]);
                        }
                        // Umschalt: Punkte umschalten (ausgewählte abwählen, übrige hinzufügen)
                        type Pts<'a> = Vec<&'a (ElementId, usize, usize)>;
                        let (on, off): (Pts, Pts) = pts.iter().partition(|(e, c, n)| {
                            ctx.selection.points.contains(&PointRef {
                                element: *e,
                                contour: *c,
                                node: *n,
                            })
                        });
                        let mut cmds = vec![pick(
                            "select.set",
                            json!({"ids": ids_json(&ids), "mode": "add"}),
                        )];
                        if !off.is_empty() {
                            cmds.push(pick(
                                "select.points",
                                json!({"points": pj(&off), "mode": "add"}),
                            ));
                        }
                        if !on.is_empty() {
                            cmds.push(pick(
                                "select.points",
                                json!({"points": pj(&on), "mode": "subtract"}),
                            ));
                        }
                        return ToolResponse::commit(cmds);
                    }
                }
                ToolResponse::commit(vec![pick(
                    "select.inRect",
                    json!({"rect": rect_json(r), "mode": if add { "add" } else { "replace" }}),
                )])
            }
            PState::Scaling { bounds, cur, .. } => {
                if (cur.left - bounds.left).abs()
                    + (cur.right - bounds.right).abs()
                    + (cur.top - bounds.top).abs()
                    + (cur.bottom - bounds.bottom).abs()
                    < 1e-4
                {
                    return ToolResponse::default();
                }
                let m = docops::rect_to_rect(bounds, cur);
                // Eckpunkt einer Rechteck-Grundform: „Rechteck modifizieren“ (PAR 10/06)
                let rect_only = ctx.selection.elements.len() == 1
                    && Self::is_rect_primitive(ctx, ctx.selection.elements[0]);
                let params = if rect_only {
                    json!({"matrix": matrix_json(&m), "step": "modify-rect"})
                } else {
                    json!({"matrix": matrix_json(&m)})
                };
                ToolResponse::commit(vec![Invocation::new("transform.matrix", params)])
            }
            PState::DraggingPoint {
                start,
                cur,
                hit,
                pull,
                ..
            } => {
                let Some(cs) = self.dragged_contours(
                    ctx,
                    &hit,
                    cur.x - start.x,
                    cur.y - start.y,
                    pull,
                    ev.mods.alt,
                ) else {
                    return ToolResponse::default();
                };
                // Teilauswahl: Punkt bzw. Henkel ziehen heißt im Original „Punkt verschieben“ (PAR 08/06)
                ToolResponse::commit(vec![Invocation::new(
                    "path.setContours",
                    json!({"id": hit.id.0, "contours": contours_json(&cs), "step": "move-point"}),
                )])
            }
            PState::PressedPoint { hit, .. } if self.id == ToolId::Pointer => {
                // Klick auf einen Punkt mit dem Zeiger: Punkt unterauswählen (zum Löschen mit Entf)
                ToolResponse::commit(vec![pick(
                    "select.points",
                    json!({"points": [{"element": hit.id.0, "contour": hit.contour, "node": hit.node}], "mode": "replace"}),
                )])
            }
            // Klick ins Leere hebt die Auswahl auf – im Original kein Verlaufsschritt (nur der
            // Menübefehl Strg+D ist einer)
            PState::PressedEmpty { add: false, .. } if !ctx.selection.is_empty() => {
                ToolResponse::commit(vec![pick(
                    "select.set",
                    json!({"ids": [], "mode": "replace"}),
                )])
            }
            _ => ToolResponse::default(),
        }
    }

    fn double(&mut self, p: Point, ctx: &ToolContext) -> ToolResponse {
        self.state = PState::Idle;
        let opt = Self::hit_opts(ctx);
        let Some(top) = top_hit(ctx, p, &opt) else {
            return ToolResponse::default();
        };
        let Some(e) = ctx.doc.element_at(&top) else {
            return ToolResponse::default();
        };
        match &e.kind {
            ElementKind::Group(_) => {
                let deep = docops::deep_hit(ctx.doc, &top, p, ctx.geometry, &opt);
                match ctx.doc.element_at(&deep) {
                    Some(c) if c.id != e.id => {
                        if let ElementKind::Text(_) = &c.kind {
                            return text_edit_response(c.id, p);
                        }
                        ToolResponse::commit(vec![pick(
                            "select.set",
                            json!({"ids": [c.id.0], "mode": "replace"}),
                        )])
                    }
                    _ => ToolResponse::default(),
                }
            }
            // Bitmap: Bitmapmodus, das Werkzeug wechselt zum Auswahlrahmen (PARITAET_A L27)
            ElementKind::Bitmap(_) => ToolResponse {
                switch_tool: Some(ToolId::Marquee),
                ..ToolResponse::commit(vec![
                    pick("select.set", json!({"ids": [e.id.0], "mode": "replace"})),
                    Invocation::new("modify.editBitmap", json!({})),
                ])
            },
            ElementKind::Text(_) => text_edit_response(e.id, p),
            // Symbolinstanz: An Position bearbeiten (re 28 S2, RT-3; FP4-03): übrige Objekte mit
            // 50 % Deckkraft an ihrer Lage, Inhalt am Instanzort
            ElementKind::Instance(_) => ToolResponse::commit(vec![
                pick("select.set", json!({"ids": [e.id.0], "mode": "replace"})),
                Invocation::new("modify.symbol.editInPlace", json!({})),
            ]),
            ElementKind::Path(_) if self.id == ToolId::Pointer => {
                // Doppelklick auf einen Pfad: zur Teilauswahl (Punkte bearbeiten), wie in CS6
                ToolResponse {
                    commands: vec![pick(
                        "select.set",
                        json!({"ids": [e.id.0], "mode": "replace"}),
                    )],
                    switch_tool: Some(ToolId::Subselection),
                    ..ToolResponse::default()
                }
            }
            _ => ToolResponse::default(),
        }
    }
}

/// Doppelklick auf Text: auswählen, zum Textwerkzeug wechseln und an derselben Stelle klicken.
fn text_edit_response(id: ElementId, p: Point) -> ToolResponse {
    ToolResponse {
        commands: vec![pick(
            "select.set",
            json!({"ids": [id.0], "mode": "replace"}),
        )],
        switch_tool: Some(ToolId::Text),
        replay: vec![ToolEvent::down(p.x, p.y), ToolEvent::up(p.x, p.y)],
        ..ToolResponse::default()
    }
}

/// Konturen als JSON für `path.setContours`/`draw.path`.
pub(crate) fn contours_json(cs: &[Contour]) -> serde_json::Value {
    json!(cs
        .iter()
        .map(|c| json!({"closed": c.closed, "nodes": c.nodes.iter().map(|n| json!({"anchor": pt_json(n.anchor), "in": pt_json(n.pred), "out": pt_json(n.succ)})).collect::<Vec<_>>()}))
        .collect::<Vec<_>>())
}

/// Pfadpunkte aller sichtbaren, nicht gesperrten Pfade (auch in Gruppen) im Rechteck.
fn points_in_rect(ctx: &ToolContext, r: Rect) -> Vec<(ElementId, usize, usize)> {
    fn walk(e: &Element, r: &Rect, out: &mut Vec<(ElementId, usize, usize)>, depth: u32) {
        if depth > 16 || !e.common.visible || e.common.locked {
            return;
        }
        match &e.kind {
            ElementKind::Path(p) => {
                for (ci, c) in p.contours.iter().enumerate() {
                    for (ni, n) in c.nodes.iter().enumerate() {
                        if r.contains(n.anchor) {
                            out.push((e.id, ci, ni));
                        }
                    }
                }
            }
            ElementKind::Group(g) => {
                for c in &g.children {
                    walk(c, r, out, depth + 1);
                }
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    for (_, l) in docops::own_layers(ctx.doc, ctx.page) {
        if !l.is_visible() {
            continue;
        }
        let ci = if l.shared_across_frames { 0 } else { ctx.frame };
        let Some(cel) = l.cels.get(ci) else { continue };
        if cel.locked {
            continue;
        }
        for e in &cel.elements {
            walk(e, &r, &mut out, 0);
        }
    }
    out
}

impl ToolMachine for PointerTool {
    fn id(&self) -> ToolId {
        self.id
    }

    fn press_hit(&self) -> Option<&'static str> {
        Some(match &self.state {
            PState::Idle => "idle",
            PState::PressedObject { .. } | PState::Moving { .. } => "object",
            PState::PressedHandle { .. } | PState::Scaling { .. } => "handle",
            PState::PressedPoint { .. } | PState::DraggingPoint { .. } | PState::Control(_) => {
                "point"
            }
            PState::FillHandle { .. } => "fill_handle",
            PState::PressedEmpty { .. } | PState::Rubber { .. } => "empty",
        })
    }

    fn handle(&mut self, ev: &ToolEvent, ctx: &ToolContext) -> ToolResponse {
        if !ctx.has_document {
            return ToolResponse::default();
        }
        match &ev.input {
            ToolInput::Pointer { kind, pos } => {
                match kind {
                    PointerKind::Down => {
                        let mut r = self.down(*pos, ev, ctx);
                        // re 28 L3: Zeigerklick nach einer Bearbeitung im Bitmap-Modus (außerhalb eines
                        // Auswahlrahmens) beendet den Modus; die Bitmap wird dabei beschnitten
                        if ctx.bitmap_worked
                            && !ctx.selection.pixels.as_ref().is_some_and(|m| {
                                m.get(pos.x.floor() as i32, pos.y.floor() as i32) > 0
                            })
                        {
                            r.commands
                                .insert(0, Invocation::new("modify.exitBitmapMode", json!({})));
                        }
                        r
                    }
                    PointerKind::Move => self.moved(*pos, ev, ctx),
                    PointerKind::Up => self.up(*pos, ev, ctx),
                    PointerKind::DoubleClick => self.double(*pos, ctx),
                }
            }
            ToolInput::Key(k) => {
                let big = ev.mods.shift;
                let dir = match k {
                    ToolKey::Left => Some("left"),
                    ToolKey::Right => Some("right"),
                    ToolKey::Up => Some("up"),
                    ToolKey::Down => Some("down"),
                    _ => None,
                };
                if let Some(d) = dir {
                    if ctx.selection.elements.is_empty() {
                        return ToolResponse::default();
                    }
                    let s = if big { 10.0 } else { 1.0 };
                    let (dx, dy) = match d {
                        "left" => (-s, 0.0),
                        "right" => (s, 0.0),
                        "up" => (0.0, -s),
                        _ => (0.0, s),
                    };
                    if self.id == ToolId::Subselection
                        && !ctx.selection.points.is_empty()
                        && !ev.mods.alt
                    {
                        return ToolResponse::commit(vec![Invocation::new(
                            "path.movePoints",
                            json!({"dx": dx, "dy": dy}),
                        )]);
                    }
                    if ev.mods.alt {
                        return ToolResponse::commit(vec![Invocation::new(
                            "edit.move",
                            json!({"dx": dx, "dy": dy, "copy": true}),
                        )]);
                    }
                    let name = if big {
                        format!("edit.nudge.big{}{}", d[..1].to_uppercase(), &d[1..])
                    } else {
                        format!("edit.nudge.{d}")
                    };
                    return ToolResponse::commit(vec![Invocation::new(&name, json!({}))]);
                }
                match k {
                    ToolKey::Delete | ToolKey::Backspace
                        if !ctx.selection.points.is_empty() && self.id != ToolId::SelectBehind =>
                    {
                        ToolResponse::commit(vec![Invocation::new("path.deletePoints", json!({}))])
                    }
                    ToolKey::Delete | ToolKey::Backspace if !ctx.selection.is_empty() => {
                        ToolResponse::commit(vec![Invocation::new("edit.clear", json!({}))])
                    }
                    ToolKey::Escape => {
                        let was_busy = self.state != PState::Idle;
                        self.state = PState::Idle;
                        self.overlay.clear();
                        // eine gewählte Bitmap (z. B. nach „Bitmapmodus beenden“ per Esc) bleibt wie im
                        // Original gewählt (PAR 06/18: zweites Esc ändert nichts)
                        let single_bitmap = ctx.selection.elements.len() == 1
                            && docops::element_on_page(
                                ctx.doc,
                                ctx.page,
                                ctx.selection.elements[0],
                            )
                            .is_some_and(|e| matches!(e.kind, fp_model::ElementKind::Bitmap(_)));
                        if was_busy || ctx.selection.is_empty() || single_bitmap {
                            ToolResponse::default()
                        } else {
                            ToolResponse::commit(vec![Invocation::new("select.none", json!({}))])
                        }
                    }
                    // Enter auf der Leinwand hebt die Auswahl auf wie im Original (Lauf 10
                    // `xs-shortcuts/22`: gewählte Gruppe + Enter → `fw.selection` leer); eine einzelne
                    // Bitmap bleibt wie bei Esc gewählt (Bitmapmodus, `xs-shortcuts/12`).
                    ToolKey::Enter if self.state == PState::Idle => {
                        let single_bitmap = ctx.selection.elements.len() == 1
                            && docops::element_on_page(
                                ctx.doc,
                                ctx.page,
                                ctx.selection.elements[0],
                            )
                            .is_some_and(|e| matches!(e.kind, fp_model::ElementKind::Bitmap(_)));
                        if ctx.selection.is_empty() || single_bitmap {
                            ToolResponse::default()
                        } else {
                            ToolResponse::commit(vec![Invocation::new("select.none", json!({}))])
                        }
                    }
                    _ => ToolResponse::default(),
                }
            }
            _ => ToolResponse::default(),
        }
    }

    fn overlay(&self) -> Vec<Overlay> {
        let mut v = self.overlay.clone();
        if let Some(h) = &self.hover {
            v.push(h.clone());
        }
        v
    }

    fn inspector_hold(&self) -> bool {
        matches!(
            self.state,
            PState::Moving { fresh: true, .. } | PState::PressedObject { fresh: true, .. }
        )
    }

    fn feedback(&self, ctx: &ToolContext) -> Vec<Overlay> {
        let mut v = Vec::new();
        let dragging_points = matches!(self.state, PState::DraggingPoint { .. });
        let moving = matches!(self.state, PState::Moving { .. } | PState::Scaling { .. });
        if !dragging_points && !moving && self.id != ToolId::SelectBehind {
            v.extend(path_points_feedback(ctx, self.id == ToolId::Subselection));
            if !matches!(self.state, PState::Control(_)) {
                v.extend(control_points_feedback(ctx, None));
            }
        }
        v.extend(self.overlay());
        v
    }
}

// ------------------------------------------------------------------ Transformieren

#[derive(Clone, Copy, Debug, PartialEq)]
enum TDrag {
    Corner(usize),
    Edge(usize),
    Rotate,
    Move,
    Pivot,
    /// 9-Slice-Hilfslinie (0 links, 1 oben, 2 rechts, 3 unten).
    Guide(usize),
}

/// Skalieren, Neigen und Verzerren: Rahmen mit 8 Griffen und Drehpunkt; Ecke ziehen skaliert
/// (Umschalt proportional, Alt um die Mitte), außerhalb ziehen dreht um den Drehpunkt (Umschalt
/// 15°), Drehpunkt ziehen verschiebt ihn, Neigen über die Kantengriffe, Verzerren bewegt Ecken
/// einzeln. Jeder Zug wird als Befehl übernommen; Eingabe/Doppelklick beendet, Esc verwirft den Zug.
#[derive(Clone, Debug)]
pub struct TransformTool {
    id: ToolId,
    bounds: Option<Rect>,
    /// Vom Benutzer verschobener Drehpunkt (sonst die Mitte).
    pivot: Option<Point>,
    drag: Option<(TDrag, Point, Point)>,
    overlay: Vec<Overlay>,
    cursor: Cursor,
    /// 9-Slice: feste Ränder (links, oben, rechts, unten) in px, vom Benutzer verschoben.
    insets: Option<([f32; 4], Rect)>,
    /// Noch nicht übernommene Transformation (wie im Original eine Vorschau bis Eingabe,
    /// Doppelklick oder Werkzeugwechsel; Esc verwirft): Ausgangsrahmen, Ziel-Viereck, Auswahl.
    pending: Option<(Rect, [Point; 4], Vec<ElementId>)>,
    /// Ein Bildschirmpixel in Dokumentpixeln (letzter Zoom).
    px: f32,
    /// In der laufenden Vorschau wurde gedreht: Bitmaps werden nach dem Übernehmen beschnitten
    /// (`TransformToolsAutoTrim`, re 28 L5).
    rotated: bool,
}

impl TransformTool {
    /// Neues Werkzeug (Scale, Skew oder Distort).
    pub fn new(id: ToolId) -> TransformTool {
        TransformTool {
            id,
            bounds: None,
            pivot: None,
            drag: None,
            overlay: Vec::new(),
            cursor: Cursor::Arrow,
            insets: None,
            pending: None,
            px: 1.0,
            rotated: false,
        }
    }

    fn pivot_of(&self, b: &Rect) -> Point {
        self.pivot.unwrap_or_else(|| b.center())
    }

    fn nine(&self) -> bool {
        self.id == ToolId::NineSliceScale
    }

    /// Zeigerlage für einen Zug: beim Skalieren folgt der Griff dem eingerasteten Zeiger (wie die
    /// Griffe des Zeigers), sonst die rohe Lage.
    fn handle_pos(&self, ctx: &ToolContext, k: TDrag, p: Point) -> Point {
        if self.id == ToolId::Scale && matches!(k, TDrag::Corner(_) | TDrag::Edge(_)) {
            Snapper::new(ctx, &ctx.selection.elements).point(p).point
        } else {
            p
        }
    }

    fn insets_of(&self, b: &Rect) -> [f32; 4] {
        match self.insets {
            Some((i, r))
                if (r.left - b.left).abs()
                    + (r.top - b.top).abs()
                    + (r.right - b.right).abs()
                    + (r.bottom - b.bottom).abs()
                    < 2.0 =>
            {
                i
            }
            _ => super::pathedit::default_insets(b),
        }
    }

    /// Lage der vier 9-Slice-Hilfslinien (x links, y oben, x rechts, y unten).
    fn guide_lines(&self, b: &Rect) -> [f32; 4] {
        let i = self.insets_of(b);
        [b.left + i[0], b.top + i[1], b.right - i[2], b.bottom - i[3]]
    }

    fn kind_at(&self, b: &Rect, p: Point, tol: f32) -> TDrag {
        if self.nine() {
            if let Some(h) = handle_at(b, p, tol) {
                return if h % 2 == 0 {
                    TDrag::Corner(h)
                } else {
                    TDrag::Edge(h)
                };
            }
            let g = self.guide_lines(b);
            let inside_x = p.x >= b.left - tol && p.x <= b.right + tol;
            let inside_y = p.y >= b.top - tol && p.y <= b.bottom + tol;
            for (i, v) in g.iter().enumerate() {
                let near = if i % 2 == 0 {
                    (p.x - v).abs() <= tol && inside_y
                } else {
                    (p.y - v).abs() <= tol && inside_x
                };
                if near {
                    return TDrag::Guide(i);
                }
            }
            return if b.contains(p) {
                TDrag::Move
            } else {
                TDrag::Rotate
            };
        }
        // Drehpunkt nur direkt auf dem kleinen Kreis greifen; daneben im Rahmen verschiebt
        // (PAR 08/10: Ziehen innen = Verschieben)
        if dist(self.pivot_of(b), p) <= tol.min(4.0 * self.px) {
            return TDrag::Pivot;
        }
        match handle_at(b, p, tol) {
            Some(h) if h % 2 == 0 => TDrag::Corner(h),
            Some(h) => TDrag::Edge(h),
            None if b.contains(p) => TDrag::Move,
            None => TDrag::Rotate,
        }
    }

    fn cursor_for(&self, k: TDrag) -> Cursor {
        match (self.id, k) {
            (_, TDrag::Guide(i)) => {
                if i % 2 == 0 {
                    Cursor::GuideV
                } else {
                    Cursor::GuideH
                }
            }
            (ToolId::NineSliceScale, TDrag::Rotate) => Cursor::Arrow,
            (_, TDrag::Pivot) => Cursor::Pivot,
            (_, TDrag::Rotate) => Cursor::Rotate,
            (_, TDrag::Move) => Cursor::Move,
            (ToolId::Distort, TDrag::Corner(_)) => Cursor::Distort,
            (ToolId::Skew, TDrag::Edge(_) | TDrag::Corner(_)) => Cursor::Skew,
            (_, TDrag::Corner(h) | TDrag::Edge(h)) => handle_cursor(h),
        }
    }

    fn quad_for(
        &self,
        b: Rect,
        kind: TDrag,
        start: Point,
        cur: Point,
        ev: &ToolEvent,
    ) -> [Point; 4] {
        let corners = b.corners();
        match (self.id, kind) {
            (ToolId::Distort, TDrag::Corner(h)) => {
                let ci = match h {
                    0 => 0,
                    2 => 1,
                    4 => 2,
                    _ => 3,
                };
                let mut q = corners;
                q[ci] = Point::new(q[ci].x + cur.x - start.x, q[ci].y + cur.y - start.y);
                q
            }
            _ => {
                let m = self.matrix_for(b, kind, start, cur, ev);
                corners.map(|p| m.apply(p))
            }
        }
    }

    fn angle(&self, b: &Rect, start: Point, cur: Point, ev: &ToolEvent) -> f64 {
        let c = self.pivot_of(b);
        let a0 = ((start.y - c.y) as f64).atan2((start.x - c.x) as f64);
        let a1 = ((cur.y - c.y) as f64).atan2((cur.x - c.x) as f64);
        let mut d = (a1 - a0).to_degrees();
        if d > 180.0 {
            d -= 360.0;
        } else if d < -180.0 {
            d += 360.0;
        }
        if ev.mods.shift {
            d = (d / 15.0).round() * 15.0;
        }
        d
    }

    fn matrix_for(
        &self,
        b: Rect,
        kind: TDrag,
        start: Point,
        cur: Point,
        ev: &ToolEvent,
    ) -> Matrix3 {
        let c = b.center();
        match kind {
            TDrag::Move => Matrix3::translation(cur.x - start.x, cur.y - start.y),
            TDrag::Pivot | TDrag::Guide(_) => Matrix3::IDENTITY,
            TDrag::Rotate if self.nine() => Matrix3::IDENTITY,
            TDrag::Rotate => docops::about(
                self.pivot_of(&b),
                &Matrix3::rotation_degrees(self.angle(&b, start, cur, ev)),
            ),
            TDrag::Edge(h) | TDrag::Corner(h) if self.id == ToolId::Skew => {
                // Neigen am Seitengriff wie im Original (PAR 08/13): nur die gegriffene Kante wandert,
                // die gegenüberliegende bleibt (Mittelgriff oben +30 → 80,50/180,50, unten fest).
                // Ecken bilden ein Trapez (siehe `drag_quad`).
                let (w, hgt) = (b.width().max(1e-6), b.height().max(1e-6));
                let _ = c;
                if matches!(h, 3 | 7) {
                    let dy = cur.y - start.y;
                    let (k, ax) = if h == 7 {
                        (-dy / w, b.right)
                    } else {
                        (dy / w, b.left)
                    };
                    docops::about(Point::new(ax, b.top), &Matrix3::shear(0.0, k))
                } else {
                    let dx = cur.x - start.x;
                    let (k, ay) = if h <= 2 {
                        (-dx / hgt, b.bottom)
                    } else {
                        (dx / hgt, b.top)
                    };
                    docops::about(Point::new(b.left, ay), &Matrix3::shear(k, 0.0))
                }
            }
            TDrag::Corner(h) | TDrag::Edge(h) if self.id == ToolId::Scale => {
                // Skalieren wie im Original (PAR 08/10, 08/16 gemessen): der Griff folgt dem
                // (eingerasteten) Zeiger, Ecken skalieren proportional mit der Projektion auf die
                // Diagonale, Umschalt ändert nichts, Alt skaliert von der Mitte aus
                docops::rect_to_rect(b, scale_projected(b, h, cur, ev.mods.alt))
            }
            TDrag::Corner(h) | TDrag::Edge(h) => {
                let hp = handles(&b)[h];
                let target = Point::new(hp.x + cur.x - start.x, hp.y + cur.y - start.y);
                docops::rect_to_rect(b, scale_by_handle(b, h, target, ev.mods.shift, ev.mods.alt))
            }
        }
    }

    fn idle_overlay(&self, b: &Rect) -> Vec<Overlay> {
        if self.nine() {
            let g = self.guide_lines(b);
            return vec![
                Overlay::TransformBox {
                    quad: b.corners(),
                    center: b.center(),
                },
                Overlay::Handles {
                    points: handles(b).to_vec(),
                },
                Overlay::SmartGuide {
                    from: Point::new(g[0], b.top),
                    to: Point::new(g[0], b.bottom),
                },
                Overlay::SmartGuide {
                    from: Point::new(b.left, g[1]),
                    to: Point::new(b.right, g[1]),
                },
                Overlay::SmartGuide {
                    from: Point::new(g[2], b.top),
                    to: Point::new(g[2], b.bottom),
                },
                Overlay::SmartGuide {
                    from: Point::new(b.left, g[3]),
                    to: Point::new(b.right, g[3]),
                },
            ];
        }
        vec![
            Overlay::TransformBox {
                quad: b.corners(),
                center: self.pivot_of(b),
            },
            Overlay::Handles {
                points: handles(b).to_vec(),
            },
            Overlay::Pivot {
                at: self.pivot_of(b),
            },
        ]
    }
}

impl TransformTool {
    /// Übernehmen der Vorschau als **ein** Befehl (Matrix, Verzerren bzw. 9-Slice).
    fn commit_pending(&mut self) -> Option<Invocation> {
        let (orig, q, _) = self.pending.take()?;
        let rotated = std::mem::take(&mut self.rotated);
        self.bounds = None;
        self.overlay.clear();
        if self.nine() {
            let to = Rect::from_points(q).unwrap_or(orig).normalized();
            let ins = self.insets_of(&orig);
            // Hilfslinien behalten ihren Pixelabstand zum Rand
            let k = (to.width() / (ins[0] + ins[2]).max(1e-6))
                .min(to.height() / (ins[1] + ins[3]).max(1e-6))
                .min(1.0);
            self.insets = Some((ins.map(|v| v * k), to));
            return Some(Invocation::new(
                "transform.nineSlice",
                json!({"rect": rect_json(orig), "to": rect_json(to), "insets": ins}),
            ));
        }
        match Matrix3::rect_to_quad(&orig, &q) {
            Some(m) if m.is_affine() => {
                if m.is_identity() {
                    return None;
                }
                // nach einer Drehung beschneidet das Original Bitmaps auf die deckenden Pixel (re 28 L5)
                Some(Invocation::new(
                    "transform.matrix",
                    json!({"matrix": matrix_json(&m), "step": "transform", "autoTrim": rotated}),
                ))
            }
            // Skalieren, Drehen, Neigen und Verzerren heißen im Original immer „Transformieren“ (PAR 08/11)
            _ => Some(Invocation::new(
                "transform.distort",
                json!({"rect": rect_json(orig), "quad": q.iter().map(|p| pt_json(*p)).collect::<Vec<_>>(), "step": "transform"}),
            )),
        }
    }

    /// Rahmen und Viereck der laufenden Vorschau (bzw. der Auswahl ohne Vorschau).
    fn current(&self, ctx: &ToolContext) -> Option<(Rect, [Point; 4])> {
        match &self.pending {
            Some((_, q, _)) => Some((Rect::from_points(*q)?.normalized(), *q)),
            // re 28 L5: Griffe und Mittelpunkt am gespeicherten Objektrechteck (bei Bitmaps nicht am
            // Rechteck der deckenden Pixel; gemessen 0/400 × 0/300 bei Pixeln 100–200)
            None => docops::ids_bounds(ctx.doc, ctx.page, &ctx.selection.elements)
                .map(|b| (b, b.corners())),
        }
    }

    fn pending_overlay(&self, ctx: &ToolContext, q: [Point; 4]) -> Vec<Overlay> {
        let mut v = Vec::new();
        if let Some((orig, _, _)) = &self.pending {
            v.push(ghost(ctx, *orig, q, false));
        }
        let b = Rect::from_points(q).unwrap_or_default().normalized();
        v.push(Overlay::TransformBox {
            quad: q,
            center: self.pivot_of(&b),
        });
        v.push(Overlay::Handles {
            points: handles(&b).to_vec(),
        });
        if !self.nine() {
            v.push(Overlay::Pivot {
                at: self.pivot_of(&b),
            });
        }
        v
    }
}

impl ToolMachine for TransformTool {
    fn id(&self) -> ToolId {
        self.id
    }

    fn handle(&mut self, ev: &ToolEvent, ctx: &ToolContext) -> ToolResponse {
        if !ctx.has_document {
            return ToolResponse::default();
        }
        // Auswahl hat sich geändert (z. B. über ein Bedienfeld): Vorschau verwerfen
        if self
            .pending
            .as_ref()
            .is_some_and(|(_, _, ids)| *ids != ctx.selection.elements)
        {
            self.pending = None;
            self.bounds = None;
        }
        if self.bounds.is_none() || self.drag.is_none() {
            self.bounds = self.current(ctx).map(|c| c.0);
        }
        let Some(b) = self.bounds else {
            return ToolResponse {
                cursor: Cursor::NotAllowed,
                ..ToolResponse::default()
            };
        };
        self.px = (1.0 / ctx.zoom.max(1e-6)) as f32;
        let q_cur = self
            .current(ctx)
            .map(|c| c.1)
            .unwrap_or_else(|| b.corners());
        let orig = self.pending.as_ref().map(|p| p.0).unwrap_or(b);
        let tol = ctx.pick_tolerance() as f32;
        match &ev.input {
            ToolInput::Pointer {
                kind: PointerKind::Down,
                pos,
            } => {
                let p = *pos;
                let kind = self.kind_at(&b, p, tol);
                self.drag = Some((kind, p, p));
                self.cursor = self.cursor_for(kind);
                ToolResponse {
                    overlay: self.pending_overlay(ctx, q_cur),
                    cursor: self.cursor,
                    ..ToolResponse::default()
                }
            }
            ToolInput::Pointer {
                kind: PointerKind::Move,
                pos,
            } => {
                if let Some((k, s, _)) = self.drag {
                    let cur = if k == TDrag::Move && ev.mods.shift {
                        constrain_angle(s, *pos, 45.0)
                    } else {
                        self.handle_pos(ctx, k, *pos)
                    };
                    self.drag = Some((k, s, cur));
                    if let TDrag::Guide(i) = k {
                        // 9-Slice-Hilfslinie verschieben (innerhalb des Rahmens, Linien kreuzen sich nicht)
                        let mut ins = self.insets_of(&b);
                        let (w, h) = (b.width(), b.height());
                        match i {
                            0 => ins[0] = (cur.x - b.left).clamp(0.0, w - ins[2]),
                            1 => ins[1] = (cur.y - b.top).clamp(0.0, h - ins[3]),
                            2 => ins[2] = (b.right - cur.x).clamp(0.0, w - ins[0]),
                            _ => ins[3] = (b.bottom - cur.y).clamp(0.0, h - ins[1]),
                        }
                        self.insets = Some((ins, b));
                        self.overlay = self.idle_overlay(&b);
                        self.overlay.push(Overlay::Label {
                            at: cur,
                            text: format!("{} px", fmt_num(ins[i])),
                        });
                    } else if self.nine()
                        && matches!(k, TDrag::Corner(_) | TDrag::Edge(_) | TDrag::Move)
                    {
                        let to = Rect::from_points(self.quad_for(b, k, s, cur, ev))
                            .unwrap_or(b)
                            .normalized();
                        let ins = self.insets_of(&orig);
                        self.overlay = Vec::new();
                        for id in &ctx.selection.elements {
                            let Some(e) = docops::element_on_page(ctx.doc, ctx.page, *id) else {
                                continue;
                            };
                            let mut cs = Vec::new();
                            collect_contours(&e, &mut cs, 0);
                            self.overlay.push(Overlay::Path {
                                contours: super::pathedit::nine_slice_contours(
                                    &cs, &orig, ins, &to,
                                ),
                            });
                        }
                        self.overlay.push(Overlay::TransformBox {
                            quad: to.corners(),
                            center: to.center(),
                        });
                        self.overlay.push(size_label(to));
                    } else if k == TDrag::Pivot {
                        let sp = Snapper::new(ctx, &ctx.selection.elements).point(cur);
                        self.pivot = Some(sp.point);
                        self.overlay = self.pending_overlay(ctx, q_cur);
                        self.overlay.extend(sp.guides);
                    } else {
                        let q = self.drag_quad(b, q_cur, k, s, cur, ev);
                        let m = self.matrix_for(b, k, s, cur, ev);
                        let piv = m.apply(self.pivot_of(&b));
                        self.overlay = vec![
                            ghost(ctx, orig, q, false),
                            Overlay::TransformBox {
                                quad: q,
                                center: piv,
                            },
                            Overlay::Pivot { at: piv },
                        ];
                        match k {
                            TDrag::Rotate => self.overlay.push(Overlay::Label {
                                at: cur,
                                text: format!("{}°", fmt_num(self.angle(&b, s, cur, ev) as f32)),
                            }),
                            TDrag::Corner(_) | TDrag::Edge(_) if self.id == ToolId::Scale => {
                                let r = Rect::from_points(q).unwrap_or(b);
                                self.overlay.push(size_label(r));
                            }
                            _ => {}
                        }
                    }
                } else {
                    self.cursor = self.cursor_for(self.kind_at(&b, *pos, tol));
                    self.overlay = if self.pending.is_some() {
                        self.pending_overlay(ctx, q_cur)
                    } else {
                        self.idle_overlay(&b)
                    };
                }
                ToolResponse::with_overlay(self.overlay.clone(), self.cursor)
            }
            ToolInput::Pointer {
                kind: PointerKind::Up,
                pos,
            } => {
                let Some((k, s, cur)) = self.drag.take() else {
                    return ToolResponse::default();
                };
                let cur = if dist(cur, *pos) > 0.0 && !(k == TDrag::Move && ev.mods.shift) {
                    self.handle_pos(ctx, k, *pos)
                } else {
                    cur
                };
                if k == TDrag::Pivot
                    || matches!(k, TDrag::Guide(_))
                    || (self.nine() && k == TDrag::Rotate)
                    || dist(s, cur) < ctx.drag_threshold()
                {
                    self.overlay = if self.pending.is_some() {
                        self.pending_overlay(ctx, q_cur)
                    } else {
                        self.idle_overlay(&b)
                    };
                    return ToolResponse::with_overlay(self.overlay.clone(), self.cursor);
                }
                // Vorschau fortschreiben, übernommen wird erst mit Eingabe/Doppelklick/Werkzeugwechsel
                let q = if self.nine() {
                    Rect::from_points(self.quad_for(b, k, s, cur, ev))
                        .unwrap_or(b)
                        .normalized()
                        .corners()
                } else {
                    let m = self.matrix_for(b, k, s, cur, ev);
                    if let Some(pv) = self.pivot
                        && !(self.id == ToolId::Distort
                            && matches!(k, TDrag::Corner(_) | TDrag::Edge(_)))
                        && !(self.id == ToolId::Skew && matches!(k, TDrag::Corner(_)))
                    {
                        self.pivot = Some(m.apply(pv));
                    }
                    self.drag_quad(b, q_cur, k, s, cur, ev)
                };
                if k == TDrag::Rotate && !self.nine() {
                    self.rotated = true;
                }
                self.pending = Some((orig, q, ctx.selection.elements.clone()));
                self.bounds = Rect::from_points(q).map(|r| r.normalized());
                self.overlay = self.pending_overlay(ctx, q);
                ToolResponse::with_overlay(self.overlay.clone(), self.cursor)
            }
            ToolInput::Pointer {
                kind: PointerKind::DoubleClick,
                ..
            }
            | ToolInput::Key(ToolKey::Enter) => {
                self.drag = None;
                let cmds: Vec<Invocation> = self.commit_pending().into_iter().collect();
                self.bounds = None;
                self.overlay.clear();
                ToolResponse {
                    commands: cmds,
                    switch_tool: Some(ToolId::Pointer),
                    ..ToolResponse::default()
                }
            }
            ToolInput::Key(ToolKey::Escape) if self.drag.is_none() && !self.nine() => {
                // Esc beendet den Transformationsmodus und **verwirft** die Vorschau wie im Original
                // (PAR 08/15: Objekt unverändert, kein Verlaufsschritt); danach ist der Zeiger aktiv
                self.pending = None;
                self.pivot = None;
                self.bounds = None;
                self.overlay.clear();
                ToolResponse {
                    switch_tool: Some(ToolId::Pointer),
                    ..ToolResponse::default()
                }
            }
            ToolInput::Key(ToolKey::Escape) => {
                if self.drag.take().is_none() {
                    // 9-Slice: Esc ohne laufenden Zug verwirft die Vorschau, Ausgangslage bleibt
                    self.pending = None;
                    self.pivot = None;
                }
                self.bounds = None;
                self.overlay = match self.current(ctx) {
                    Some((b, q)) if self.pending.is_some() => {
                        let _ = b;
                        self.pending_overlay(ctx, q)
                    }
                    Some((b, _)) => self.idle_overlay(&b),
                    None => Vec::new(),
                };
                ToolResponse::with_overlay(self.overlay.clone(), self.cursor)
            }
            _ => ToolResponse::default(),
        }
    }

    fn finish(&mut self, _ctx: &ToolContext) -> ToolResponse {
        // Werkzeugwechsel bzw. Menübefehl übernimmt die Vorschau
        self.drag = None;
        ToolResponse::commit(self.commit_pending().into_iter().collect())
    }

    fn overlay(&self) -> Vec<Overlay> {
        self.overlay.clone()
    }

    fn feedback(&self, ctx: &ToolContext) -> Vec<Overlay> {
        if self.drag.is_some() && !self.overlay.is_empty() {
            return self.overlay.clone();
        }
        match self.current(ctx) {
            Some((_, q)) if self.pending.is_some() => self.pending_overlay(ctx, q),
            Some((b, _)) => self.idle_overlay(&b),
            None => Vec::new(),
        }
    }
}

impl TransformTool {
    /// Ziel-Viereck eines Zuges: Verzerren bewegt die gegriffene Ecke des aktuellen Vierecks,
    /// sonst wird das aktuelle Viereck mit der Zugmatrix abgebildet.
    fn drag_quad(
        &self,
        b: Rect,
        q_cur: [Point; 4],
        k: TDrag,
        s: Point,
        cur: Point,
        ev: &ToolEvent,
    ) -> [Point; 4] {
        let (dx, dy) = (cur.x - s.x, cur.y - s.y);
        // Ecke des aktuellen Vierecks zum Griff `h` (Reihenfolge LO, RO, RU, LU)
        let corner_of = |h: usize| {
            let hp = handles(&b)[h];
            (0..4)
                .min_by(|a, c| dist(q_cur[*a], hp).total_cmp(&dist(q_cur[*c], hp)))
                .unwrap_or(0)
        };
        if self.id == ToolId::Distort
            && let TDrag::Corner(h) = k
        {
            let ci = corner_of(h);
            let mut q = q_cur;
            q[ci] = Point::new(q[ci].x + dx, q[ci].y + dy);
            return q;
        }
        if self.id == ToolId::Distort
            && let TDrag::Edge(h) = k
        {
            // Verzerren am Seitengriff (PAR 08/14): beide Ecken der Kante wandern mit dem Zeiger
            // (oben +20,−10 → 70,40/170,40)
            let mut q = q_cur;
            for c in [corner_of((h + 7) % 8), corner_of((h + 1) % 8)] {
                q[c] = Point::new(q[c].x + dx, q[c].y + dy);
            }
            return q;
        }
        if self.id == ToolId::Skew
            && let TDrag::Corner(h) = k
        {
            // Neigen an der Ecke (PAR 08/13): Trapez – die Ecke wandert entlang ihrer Kante, die
            // andere Ecke derselben Kante spiegelbildlich (senkrecht −20 → 150,30/150,140;
            // waagrecht +20 → 30,50/170,50)
            let ci = corner_of(h);
            let mut q = q_cur;
            if dx.abs() >= dy.abs() {
                let pi = ci ^ 1;
                q[ci].x += dx;
                q[pi].x -= dx;
            } else {
                let pi = 3 - ci;
                q[ci].y += dy;
                q[pi].y -= dy;
            }
            return q;
        }
        let m = self.matrix_for(b, k, s, cur, ev);
        q_cur.map(|p| m.apply(p))
    }
}

// ------------------------------------------------------------------ Zuschneiden / Exportbereich

/// Zuschneiden und Exportbereich: Rahmen aufziehen, mit Griffen anpassen, Eingabe oder Doppelklick
/// übernimmt (Zuschneiden → `edit.cropDocument`, Exportbereich → `file.exportArea`), Esc verwirft.
/// Außerhalb des Rahmens wird abgedunkelt.
#[derive(Clone, Debug)]
pub struct CropTool {
    id: ToolId,
    rect: Option<Rect>,
    drag: Option<(Option<usize>, Point, Rect)>,
    cursor: Cursor,
}

impl CropTool {
    /// Neues Werkzeug (Crop oder ExportArea).
    pub fn new(id: ToolId) -> CropTool {
        CropTool {
            id,
            rect: None,
            drag: None,
            cursor: Self::idle(id),
        }
    }

    /// Zeiger ohne Griff: Zuschneiden zeigt das Zuschneide-Symbol (PARITAET_A K9), Exportbereich
    /// das Fadenkreuz.
    fn idle(id: ToolId) -> Cursor {
        if id == ToolId::Crop {
            Cursor::Crop
        } else {
            Cursor::Crosshair
        }
    }

    fn commit(&mut self) -> ToolResponse {
        let Some(r) = self.rect.take() else {
            return ToolResponse::default();
        };
        self.drag = None;
        let inv = |name: &str| Invocation::new(name, json!({"rect": rect_json(r.normalized())}));
        if self.id == ToolId::Crop {
            // Rahmen auf die Leinwand begrenzen, Objekte außerhalb löschen, danach alles gewählt (PAR 08/18)
            ToolResponse::commit(vec![Invocation::new(
                "edit.cropDocument",
                json!({"rect": rect_json(r.normalized()), "tool": true}),
            )])
        } else {
            // Exportbereich: wie im Original öffnet die Oberfläche den Export-Dialog für den Bereich
            ToolResponse {
                ui_request: Some(inv("file.exportArea")),
                ..ToolResponse::default()
            }
        }
    }
}

impl ToolMachine for CropTool {
    fn id(&self) -> ToolId {
        self.id
    }

    fn handle(&mut self, ev: &ToolEvent, ctx: &ToolContext) -> ToolResponse {
        if !ctx.has_document {
            return ToolResponse::default();
        }
        let tol = ctx.pick_tolerance() as f32;
        match &ev.input {
            ToolInput::Pointer {
                kind: PointerKind::Down,
                pos,
            } => {
                let h = self.rect.and_then(|r| handle_at(&r, *pos, tol));
                let inside = h.is_none() && self.rect.is_some_and(|r| r.contains(*pos));
                let base = match (h, self.rect) {
                    (Some(_), Some(r)) => r,
                    _ if inside => self.rect.unwrap_or_default(),
                    _ => {
                        let p = Snapper::new(ctx, &[]).without_smart().point(*pos).point;
                        Rect::new(p.x, p.y, p.x, p.y)
                    }
                };
                // Innen ziehen verschiebt den Rahmen (Griff 8)
                self.drag = Some((if inside { Some(8) } else { h }, *pos, base));
                self.cursor = match h {
                    Some(h) => handle_cursor(h),
                    None if inside => Cursor::Move,
                    None => Self::idle(self.id),
                };
                ToolResponse {
                    overlay: self.overlay(),
                    cursor: self.cursor,
                    ..ToolResponse::default()
                }
            }
            ToolInput::Pointer {
                kind: PointerKind::Move,
                pos,
            }
            | ToolInput::Pointer {
                kind: PointerKind::Up,
                pos,
            } => {
                let mut extra = Vec::new();
                if let Some((h, start, base)) = self.drag {
                    // Zuschneiden/Exportbereich rasten nicht an Objekten (PAR 08/20: 50,50→300,200
                    // ergibt genau 250 × 150)
                    let sp = Snapper::new(ctx, &[]).without_smart().point(*pos);
                    extra = sp.guides;
                    let r = match h {
                        Some(8) => base.translate(pos.x - start.x, pos.y - start.y),
                        Some(h) => scale_by_handle(base, h, sp.point, ev.mods.shift, ev.mods.alt),
                        None => super::constrain_rect(
                            Point::new(base.left, base.top),
                            sp.point,
                            ev.mods.shift,
                            ev.mods.alt,
                        ),
                    };
                    if dist(start, *pos) >= ctx.drag_threshold() || h.is_some() {
                        self.rect = Some(r.normalized());
                    }
                    if ev.kind() == Some(PointerKind::Up) {
                        self.drag = None;
                        extra.clear();
                    }
                } else {
                    self.cursor = match self.rect {
                        Some(r) => match handle_at(&r, *pos, tol) {
                            Some(h) => handle_cursor(h),
                            None if r.contains(*pos) => Cursor::Move,
                            None => Self::idle(self.id),
                        },
                        None => Self::idle(self.id),
                    };
                }
                let mut o = self.overlay();
                if let Some(r) = self.rect
                    && self.drag.is_some()
                {
                    o.push(size_label(r));
                }
                o.extend(extra);
                ToolResponse::with_overlay(o, self.cursor)
            }
            ToolInput::Pointer {
                kind: PointerKind::DoubleClick,
                ..
            }
            | ToolInput::Key(ToolKey::Enter) => self.commit(),
            ToolInput::Key(ToolKey::Escape) => {
                let had = self.rect.is_some();
                self.rect = None;
                self.drag = None;
                // Zuschneiden: Esc verwirft den Rahmen, das Werkzeug wechselt zum Zeiger (Original
                // live gemessen, PARITAET_A L28); Exportbereich nicht gemessen, bleibt
                let switch_tool = (had && self.id == ToolId::Crop).then_some(ToolId::Pointer);
                ToolResponse {
                    switch_tool,
                    ..ToolResponse::default()
                }
            }
            _ => ToolResponse::default(),
        }
    }

    fn finish(&mut self, _ctx: &ToolContext) -> ToolResponse {
        self.rect = None;
        self.drag = None;
        ToolResponse::default()
    }

    fn overlay(&self) -> Vec<Overlay> {
        match self.rect {
            Some(r) => vec![
                Overlay::Shade { rect: r },
                Overlay::Rect {
                    rect: r,
                    dashed: false,
                },
                Overlay::Handles {
                    points: handles(&r).to_vec(),
                },
            ],
            None => Vec::new(),
        }
    }

    fn live_rect(&self) -> Option<Rect> {
        self.rect
    }
}
