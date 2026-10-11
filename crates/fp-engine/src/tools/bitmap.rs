//! Bitmapwerkzeuge: Auswahlrahmen, Lasso, Zauberstab, Pinsel/Bleistift/Radiergummi, Füllwerkzeug,
//! Verlauf (mit Verlaufsgriffen auf Vektoren) und Pipette (10 §2.6).

use super::draw::freehand_contour_json;
use super::snap::Snapper;
use super::{
    constrain_angle, constrain_rect, dist, pt_json, rect_json, size_label, Cursor, EdgeMode, Invocation, MarqueeStyle, Overlay, PointerKind, ToolContext,
    ToolEvent, ToolId, ToolInput, ToolKey, ToolMachine, ToolResponse,
};
use crate::docops::{self, HitOptions};
use crate::selection::SelectMode;
use fp_geom::{Point, Rect};
use fp_model::{ElementId, ElementKind, FillHandles};
use serde_json::{json, Value};

fn mode_from(ev: &ToolEvent) -> SelectMode {
    match (ev.mods.shift, ev.mods.alt) {
        (true, true) => SelectMode::Intersect,
        (true, false) => SelectMode::Add,
        (false, true) => SelectMode::Subtract,
        _ => SelectMode::Replace,
    }
}

fn edge_params(e: EdgeMode) -> (bool, u32) {
    match e {
        EdgeMode::Hard => (false, 0),
        EdgeMode::AntiAlias => (true, 0),
        EdgeMode::Feather(f) => (true, f),
    }
}

/// Bitmap als Ziel (Bitmapmodus oder genau eine ausgewählte Bitmap)?
fn bitmap_target(ctx: &ToolContext) -> bool {
    ctx.bitmap_mode
        || ctx.selection.elements.iter().any(|id| docops::element_on_page(ctx.doc, ctx.page, *id).is_some_and(|e| matches!(e.kind, ElementKind::Bitmap(_))))
}

/// Fang für Pixelauswahl: Hilfslinien, Raster, ganze Pixel (keine intelligenten Hilfslinien).
fn pixel_snap(ctx: &ToolContext, p: Point) -> Point {
    Snapper::new(ctx, &[]).without_smart().point(p).point
}

/// Auswahlrahmen und ovaler Auswahlrahmen.
#[derive(Clone, Debug)]
pub struct MarqueeTool {
    id: ToolId,
    start: Option<(Point, SelectMode)>,
    cur: Option<Point>,
    overlay: Vec<Overlay>,
}

impl MarqueeTool {
    /// Neues Werkzeug (Marquee, OvalMarquee).
    pub fn new(id: ToolId) -> MarqueeTool {
        MarqueeTool { id, start: None, cur: None, overlay: Vec::new() }
    }

    fn rect(&self, start: Point, cur: Point, mode: SelectMode, ev: &ToolEvent, ctx: &ToolContext) -> Rect {
        match ctx.options.marquee_style {
            MarqueeStyle::FixedSize(w, h) => Rect::new(cur.x, cur.y, cur.x + w, cur.y + h),
            MarqueeStyle::FixedRatio(rx, ry) if rx > 0.0 => {
                let w = cur.x - start.x;
                let h = w.abs() * ry / rx * if cur.y < start.y { -1.0 } else { 1.0 };
                Rect::new(start.x, start.y, start.x + w, start.y + h).normalized()
            }
            _ => constrain_rect(start, cur, ev.mods.shift && mode == SelectMode::Replace, ev.mods.alt && mode == SelectMode::Replace),
        }
    }

    fn shape(&self, r: Rect) -> Overlay {
        if self.id == ToolId::OvalMarquee { Overlay::Ellipse { rect: r, dashed: true } } else { Overlay::Rect { rect: r, dashed: true } }
    }
}

impl ToolMachine for MarqueeTool {
    fn id(&self) -> ToolId {
        self.id
    }

    fn handle(&mut self, ev: &ToolEvent, ctx: &ToolContext) -> ToolResponse {
        if !ctx.has_document {
            return ToolResponse::default();
        }
        let ToolInput::Pointer { kind, pos } = &ev.input else {
            if ev.input == ToolInput::Key(ToolKey::Escape) {
                self.start = None;
                self.overlay.clear();
            }
            return ToolResponse::default();
        };
        let ellipse = self.id == ToolId::OvalMarquee;
        let p = pixel_snap(ctx, *pos);
        match kind {
            PointerKind::Down => {
                self.start = Some((p, mode_from(ev)));
                self.cur = None;
                ToolResponse { cursor: Cursor::Crosshair, ..ToolResponse::default() }
            }
            PointerKind::Move => {
                // Leertaste beim Aufziehen: Rahmen verschieben (Startpunkt wandert mit)
                if ev.mods.space
                    && let (Some((s, m)), Some(c)) = (self.start, self.cur)
                {
                    self.start = Some((Point::new(s.x + p.x - c.x, s.y + p.y - c.y), m));
                }
                if let Some((s, m)) = self.start
                    && (self.cur.is_some() || dist(s, *pos) >= ctx.drag_threshold())
                {
                    self.cur = Some(p);
                    let r = self.rect(s, p, m, ev, ctx);
                    self.overlay = vec![self.shape(r), size_label(r)];
                } else if self.start.is_none()
                    && let MarqueeStyle::FixedSize(..) = ctx.options.marquee_style
                {
                    // feste Größe: Rahmen hängt am Cursor
                    let r = self.rect(p, p, SelectMode::Replace, ev, ctx);
                    self.overlay = vec![self.shape(r)];
                }
                ToolResponse::with_overlay(self.overlay.clone(), Cursor::Crosshair)
            }
            PointerKind::Up => {
                self.overlay.clear();
                let Some((s, m)) = self.start.take() else { return ToolResponse::default() };
                let fixed = matches!(ctx.options.marquee_style, MarqueeStyle::FixedSize(..));
                if self.cur.take().is_none() && !fixed && dist(s, *pos) < ctx.drag_threshold() {
                    return if m == SelectMode::Replace && ctx.selection.has_pixels() {
                        ToolResponse::commit(vec![Invocation::new("select.pixels", json!({"shape": "none"}))])
                    } else {
                        ToolResponse::default()
                    };
                }
                let r = self.rect(s, p, m, ev, ctx);
                let (aa, fe) = edge_params(ctx.options.marquee_edge);
                ToolResponse::commit(vec![Invocation::new(
                    "select.pixels",
                    json!({"shape": if ellipse { "ellipse" } else { "rect" }, "rect": rect_json(r), "mode": m.name(), "antialias": aa, "feather": fe}),
                )])
            }
            PointerKind::DoubleClick => ToolResponse::default(),
        }
    }

    fn overlay(&self) -> Vec<Overlay> {
        self.overlay.clone()
    }
}

/// Lasso (Freihand) und Polygon-Lasso (Klickpunkte, Doppelklick bzw. Klick auf den Start schließt).
#[derive(Clone, Debug)]
pub struct LassoTool {
    id: ToolId,
    points: Vec<Point>,
    mode: SelectMode,
    active: bool,
    hover: Option<Point>,
}

impl LassoTool {
    /// Neues Werkzeug (Lasso, PolygonLasso).
    pub fn new(id: ToolId) -> LassoTool {
        LassoTool { id, points: Vec::new(), mode: SelectMode::Replace, active: false, hover: None }
    }

    fn commit(&mut self, ctx: &ToolContext) -> ToolResponse {
        let pts = std::mem::take(&mut self.points);
        self.active = false;
        self.hover = None;
        if pts.len() < 3 {
            return ToolResponse::default();
        }
        let (aa, fe) = edge_params(ctx.options.lasso_edge);
        ToolResponse::commit(vec![Invocation::new(
            "select.pixels",
            json!({"shape": "polygon", "points": pts.iter().map(|p| pt_json(*p)).collect::<Vec<Value>>(), "mode": self.mode.name(), "antialias": aa, "feather": fe}),
        )])
    }

    fn cursor(&self) -> Cursor {
        Cursor::Lasso
    }
}

impl ToolMachine for LassoTool {
    fn id(&self) -> ToolId {
        self.id
    }

    fn handle(&mut self, ev: &ToolEvent, ctx: &ToolContext) -> ToolResponse {
        if !ctx.has_document {
            return ToolResponse::default();
        }
        let poly = self.id == ToolId::PolygonLasso;
        match &ev.input {
            ToolInput::Pointer { kind: PointerKind::Down, pos } => {
                if !self.active {
                    self.mode = mode_from(ev);
                    self.active = true;
                    self.points.clear();
                }
                if poly && self.points.len() >= 3 && dist(self.points[0], *pos) <= ctx.pick_tolerance() as f32 {
                    return self.commit(ctx);
                }
                let p = match self.points.last() {
                    Some(l) if poly && ev.mods.shift => constrain_angle(*l, *pos, 45.0),
                    _ => *pos,
                };
                self.points.push(p);
                ToolResponse::with_overlay(self.overlay(), self.cursor())
            }
            ToolInput::Pointer { kind: PointerKind::Move, pos } if self.active && !poly => {
                if self.points.last().is_none_or(|l| dist(*l, *pos) >= 0.5) {
                    self.points.push(*pos);
                }
                ToolResponse::with_overlay(self.overlay(), self.cursor())
            }
            ToolInput::Pointer { kind: PointerKind::Move, pos } if self.active && poly => {
                self.hover = Some(match self.points.last() {
                    Some(l) if ev.mods.shift => constrain_angle(*l, *pos, 45.0),
                    _ => *pos,
                });
                ToolResponse::with_overlay(self.overlay(), self.cursor())
            }
            ToolInput::Pointer { kind: PointerKind::Up, pos } if self.active && !poly => {
                self.points.push(*pos);
                self.commit(ctx)
            }
            ToolInput::Pointer { kind: PointerKind::DoubleClick, pos } if poly && self.active => {
                // Doppelklick an einer neuen Stelle setzt dort noch den letzten Punkt
                if self.points.last().is_none_or(|l| dist(*l, *pos) > ctx.pick_tolerance() as f32) {
                    self.points.push(*pos);
                }
                self.commit(ctx)
            }
            ToolInput::Key(ToolKey::Enter) if poly && self.active => self.commit(ctx),
            ToolInput::Key(ToolKey::Escape) => {
                self.points.clear();
                self.active = false;
                self.hover = None;
                ToolResponse::default()
            }
            _ => ToolResponse::with_overlay(self.overlay(), self.cursor()),
        }
    }

    fn overlay(&self) -> Vec<Overlay> {
        let mut pts = self.points.clone();
        if let Some(h) = self.hover {
            pts.push(h);
        }
        if pts.is_empty() { Vec::new() } else { vec![Overlay::Polyline { points: pts, closed: true }] }
    }
}

/// Zauberstab: Klick wählt ähnliche, zusammenhängende Pixel der Bitmap (`WandToolTolerance`).
#[derive(Clone, Debug, Default)]
pub struct WandTool;

impl ToolMachine for WandTool {
    fn id(&self) -> ToolId {
        ToolId::MagicWand
    }

    fn handle(&mut self, ev: &ToolEvent, ctx: &ToolContext) -> ToolResponse {
        match &ev.input {
            ToolInput::Pointer { kind: PointerKind::Down, pos } if ctx.has_document => {
                let (aa, fe) = edge_params(ctx.options.wand_edge);
                ToolResponse {
                    commands: vec![Invocation::new(
                        "select.magicWand",
                        json!({"point": pt_json(*pos), "tolerance": ctx.options.wand_tolerance, "mode": mode_from(ev).name(), "antialias": aa, "feather": fe}),
                    )],
                    cursor: Cursor::Wand,
                    ..ToolResponse::default()
                }
            }
            _ => ToolResponse { cursor: Cursor::Wand, ..ToolResponse::default() },
        }
    }
}

/// Pinsel (Bitmap- oder Vektormodus), Bleistift (1 px) und Radiergummi.
#[derive(Clone, Debug)]
pub struct BrushTool {
    id: ToolId,
    points: Vec<Point>,
    active: bool,
    hover: Option<Point>,
}

impl BrushTool {
    /// Neues Werkzeug (Brush, Pencil, Eraser).
    pub fn new(id: ToolId) -> BrushTool {
        BrushTool { id, points: Vec::new(), active: false, hover: None }
    }

    fn size(&self, ctx: &ToolContext) -> f32 {
        match self.id {
            ToolId::Pencil => 1.0,
            ToolId::Eraser => ctx.options.eraser_size,
            _ => ctx.options.brush_size,
        }
    }

    fn cursor(&self) -> Cursor {
        match self.id {
            ToolId::Pencil => Cursor::Pencil,
            ToolId::Eraser => Cursor::Eraser,
            _ => Cursor::Brush,
        }
    }

    fn overlay_for(&self, ctx: &ToolContext) -> Vec<Overlay> {
        let mut v = Vec::new();
        if !self.points.is_empty() {
            v.push(Overlay::Stroke { points: self.points.clone(), diameter: self.size(ctx) });
        }
        if let Some(h) = self.hover
            && ctx.prefs.get_bool("UseBrushSizeCursors", true)
            && self.id != ToolId::Pencil
        {
            v.push(Overlay::BrushTip { at: h, diameter: self.size(ctx), round: self.id != ToolId::Eraser || ctx.options.eraser_round });
        }
        v
    }
}

impl ToolMachine for BrushTool {
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
                self.points = vec![*pos];
                self.active = true;
            }
            PointerKind::Move if self.active => {
                if ev.mods.shift {
                    self.points.truncate(1);
                }
                self.points.push(*pos);
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
                let pj: Vec<Value> = pts.iter().map(|p| pt_json(*p)).collect();
                let size = self.size(ctx);
                let cmd = match self.id {
                    ToolId::Eraser => Invocation::new(
                        "bitmap.erase",
                        json!({"points": pj, "size": size, "round": ctx.options.eraser_round, "opacity": ctx.options.eraser_opacity}),
                    ),
                    ToolId::Brush if !bitmap_target(ctx) => {
                        let Some(c) = freehand_contour_json(&pts, ctx.options.path_precision, false) else { return ToolResponse::default() };
                        super::styled(Invocation::new("draw.path", json!({"contours": [c], "fill": false, "strokeWidth": size})), ctx.options)
                    }
                    _ => Invocation::new(
                        "bitmap.paint",
                        json!({"points": pj, "size": size, "antialias": self.id == ToolId::Brush || ctx.options.pencil_antialias}),
                    ),
                };
                return ToolResponse { commands: vec![cmd], cursor: self.cursor(), overlay: self.overlay_for(ctx), ..ToolResponse::default() };
            }
            _ => {}
        }
        ToolResponse::with_overlay(self.overlay_for(ctx), self.cursor())
    }

    fn overlay(&self) -> Vec<Overlay> {
        if self.points.is_empty() { Vec::new() } else { vec![Overlay::Polyline { points: self.points.clone(), closed: false }] }
    }

    fn feedback(&self, ctx: &ToolContext) -> Vec<Overlay> {
        self.overlay_for(ctx)
    }
}

/// Verlaufsgriffe der einzigen ausgewählten Vektorfüllung (Verlauf/Muster mit gültigen Griffen).
fn gradient_target(ctx: &ToolContext) -> Option<(ElementId, FillHandles)> {
    if ctx.selection.elements.len() != 1 {
        return None;
    }
    let id = ctx.selection.elements[0];
    let e = docops::element_on_page(ctx.doc, ctx.page, id)?;
    let (attrs, h) = match &e.kind {
        ElementKind::Path(p) => (&p.attrs, p.fill_handles),
        ElementKind::Text(t) => (&t.attrs, t.fill_handles),
        _ => return None,
    };
    let f = attrs.fill.as_ref()?;
    ((2..=15).contains(&f.shape) && h.valid).then_some((id, h))
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum GradDrag {
    Start,
    End,
    Both,
}

/// Füllwerkzeug (Klick) und Verlauf (Ziehen). Auf einer ausgewählten Vektorfüllung mit Verlauf
/// zeigen beide Werkzeuge die Verlaufsgriffe (Kreis = Anfang, Quadrat = Ende); Ziehen eines Griffs
/// verschiebt ihn, Ziehen der Linie verschiebt beide (`moveFillVectorHandleBy`).
#[derive(Clone, Debug)]
pub struct FillTool {
    id: ToolId,
    start: Option<Point>,
    cur: Option<Point>,
    handle: Option<(ElementId, FillHandles, GradDrag, Point)>,
}

impl FillTool {
    /// Neues Werkzeug (PaintBucket, Gradient).
    pub fn new(id: ToolId) -> FillTool {
        FillTool { id, start: None, cur: None, handle: None }
    }

    fn cursor(&self) -> Cursor {
        if self.id == ToolId::Gradient { Cursor::Gradient } else { Cursor::Bucket }
    }

    fn dragged_handles(h: FillHandles, which: GradDrag, start: Point, cur: Point) -> FillHandles {
        let (dx, dy) = (cur.x - start.x, cur.y - start.y);
        let mv = |p: Point| Point::new(p.x + dx, p.y + dy);
        let mut n = h;
        match which {
            GradDrag::Start => n.start = mv(h.start),
            GradDrag::End => n.end = mv(h.end),
            GradDrag::Both => {
                n.start = mv(h.start);
                n.end = mv(h.end);
                n.second = mv(h.second);
            }
        }
        n
    }
}

/// Abstand eines Punkts zu einer Strecke.
fn dist_segment(p: Point, a: Point, b: Point) -> f32 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let l2 = dx * dx + dy * dy;
    let t = if l2 > 0.0 { (((p.x - a.x) * dx + (p.y - a.y) * dy) / l2).clamp(0.0, 1.0) } else { 0.0 };
    dist(p, Point::new(a.x + t * dx, a.y + t * dy))
}

impl ToolMachine for FillTool {
    fn id(&self) -> ToolId {
        self.id
    }

    fn handle(&mut self, ev: &ToolEvent, ctx: &ToolContext) -> ToolResponse {
        if !ctx.has_document {
            return ToolResponse::default();
        }
        let ToolInput::Pointer { kind, pos } = &ev.input else {
            if ev.input == ToolInput::Key(ToolKey::Escape) {
                self.start = None;
                self.cur = None;
                self.handle = None;
            }
            return ToolResponse::default();
        };
        let tol = ctx.pick_tolerance() as f32;
        match kind {
            PointerKind::Down => {
                // Verlaufsgriffe greifen
                if let Some((id, h)) = gradient_target(ctx) {
                    let which = if dist(*pos, h.start) <= tol {
                        Some(GradDrag::Start)
                    } else if dist(*pos, h.end) <= tol {
                        Some(GradDrag::End)
                    } else if dist_segment(*pos, h.start, h.end) <= tol {
                        Some(GradDrag::Both)
                    } else {
                        None
                    };
                    if let Some(w) = which {
                        self.handle = Some((id, h, w, *pos));
                        return ToolResponse { cursor: Cursor::Move, ..ToolResponse::default() };
                    }
                }
                self.start = Some(*pos);
                self.cur = None;
                ToolResponse { cursor: self.cursor(), ..ToolResponse::default() }
            }
            PointerKind::Move => {
                if let Some((_, h, w, s)) = self.handle {
                    let n = Self::dragged_handles(h, w, s, *pos);
                    return ToolResponse::with_overlay(vec![Overlay::GradientHandles { start: n.start, end: n.end }], Cursor::Move);
                }
                if let Some(s) = self.start
                    && self.id == ToolId::Gradient
                    && dist(s, *pos) >= ctx.drag_threshold()
                {
                    self.cur = Some(if ev.mods.shift { constrain_angle(s, *pos, 45.0) } else { *pos });
                }
                let hover_handle = gradient_target(ctx).is_some_and(|(_, h)| dist(*pos, h.start) <= tol || dist(*pos, h.end) <= tol);
                ToolResponse::with_overlay(self.overlay(), if hover_handle && self.start.is_none() { Cursor::Move } else { self.cursor() })
            }
            PointerKind::Up => {
                if let Some((id, h, w, s)) = self.handle.take() {
                    if dist(s, *pos) < ctx.drag_threshold() {
                        return ToolResponse::default();
                    }
                    let n = Self::dragged_handles(h, w, s, *pos);
                    return ToolResponse::commit(vec![Invocation::new(
                        "element.setFillHandles",
                        json!({"ids": [id.0], "start": pt_json(n.start), "end": pt_json(n.end), "second": pt_json(n.second)}),
                    )]);
                }
                let Some(s) = self.start.take() else { return ToolResponse::default() };
                let to = self.cur.take();
                let dragged = to.is_some() || dist(s, *pos) >= ctx.drag_threshold();
                let to = to.unwrap_or(*pos);
                let colors = json!([crate::commands::argb_hex(ctx.fill_color), crate::commands::argb_hex(ctx.stroke_color)]);
                if self.id == ToolId::Gradient && dragged {
                    if bitmap_target(ctx) || ctx.selection.has_pixels() {
                        return ToolResponse::commit(vec![Invocation::new("bitmap.gradient", json!({"from": pt_json(s), "to": pt_json(to), "colors": colors}))]);
                    }
                    if !ctx.selection.elements.is_empty() {
                        return ToolResponse::commit(vec![Invocation::new("element.setGradient", json!({"from": pt_json(s), "to": pt_json(to), "colors": colors}))]);
                    }
                    return ToolResponse::default();
                }
                if bitmap_target(ctx) {
                    return ToolResponse::commit(vec![Invocation::new(
                        "bitmap.floodFill",
                        json!({"point": pt_json(*pos), "tolerance": ctx.options.fill_tolerance, "fillSelection": ctx.options.fill_selection}),
                    )]);
                }
                let opt = HitOptions { tolerance: ctx.pick_tolerance(), ..HitOptions::default() };
                let hit = super::pointer::top_hit(ctx, *pos, &opt).and_then(|p| ctx.doc.element_at(&p).map(|e| e.id));
                match hit {
                    Some(id) if self.id == ToolId::Gradient => {
                        // Klick mit dem Verlaufswerkzeug: Verlauf quer über das Objekt
                        let b = docops::ids_bounds(ctx.doc, ctx.page, &[id]).unwrap_or_default();
                        let (a, c) = (Point::new(b.left, (b.top + b.bottom) / 2.0), Point::new(b.right, (b.top + b.bottom) / 2.0));
                        ToolResponse::commit(vec![Invocation::new("element.setGradient", json!({"ids": [id.0], "from": pt_json(a), "to": pt_json(c), "colors": colors}))])
                    }
                    Some(id) => ToolResponse::commit(vec![Invocation::new("element.setFill", json!({"ids": [id.0], "color": crate::commands::argb_hex(ctx.fill_color)}))]),
                    None => ToolResponse::default(),
                }
            }
            PointerKind::DoubleClick => ToolResponse::default(),
        }
    }

    fn overlay(&self) -> Vec<Overlay> {
        match (self.start, self.cur) {
            (Some(a), Some(b)) => vec![Overlay::Line { from: a, to: b }, Overlay::GradientHandles { start: a, end: b }],
            _ => Vec::new(),
        }
    }

    fn feedback(&self, ctx: &ToolContext) -> Vec<Overlay> {
        let mut v = self.overlay();
        if self.handle.is_none()
            && self.cur.is_none()
            && let Some((_, h)) = gradient_target(ctx)
        {
            v.push(Overlay::GradientHandles { start: h.start, end: h.end });
        }
        v
    }
}

/// Pipette: Klick übernimmt die Farbe in die Füllfarbe (Alt: Strichfarbe); 1, 3×3 oder 5×5 Pixel.
#[derive(Clone, Debug, Default)]
pub struct EyedropperTool;

impl ToolMachine for EyedropperTool {
    fn id(&self) -> ToolId {
        ToolId::Eyedropper
    }

    fn handle(&mut self, ev: &ToolEvent, ctx: &ToolContext) -> ToolResponse {
        match &ev.input {
            ToolInput::Pointer { kind: PointerKind::Up, pos } if ctx.has_document => ToolResponse {
                commands: vec![Invocation::new(
                    "color.sample",
                    json!({"point": pt_json(*pos), "target": if ev.mods.alt { "stroke" } else { "fill" }, "samples": ctx.options.eyedropper_samples}),
                )],
                cursor: Cursor::Eyedropper,
                ..ToolResponse::default()
            },
            _ => ToolResponse { cursor: Cursor::Eyedropper, ..ToolResponse::default() },
        }
    }
}
