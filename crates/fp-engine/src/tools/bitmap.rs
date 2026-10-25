//! Bitmapwerkzeuge: Auswahlrahmen, Lasso, Zauberstab, Pinsel/Bleistift/Radiergummi, Füllwerkzeug,
//! Verlauf (mit Verlaufsgriffen auf Vektoren) und Pipette (10 §2.6).

use super::snap::Snapper;
use super::{
    Cursor, EdgeMode, Invocation, MarqueeStyle, Overlay, PointerKind, StrokePause, ToolContext,
    ToolEvent, ToolId, ToolInput, ToolKey, ToolMachine, ToolResponse, constrain_angle,
    constrain_rect, dist, pt_json, rect_json, size_label,
};
use crate::docops::{self, HitOptions};
use crate::selection::SelectMode;
use fp_geom::{Point, Rect};
use fp_model::{ElementId, ElementKind, FillHandles};
use serde_json::{Value, json};

fn mode_from(ev: &ToolEvent) -> SelectMode {
    match (ev.mods.shift, ev.mods.alt) {
        (true, true) => SelectMode::Intersect,
        (true, false) => SelectMode::Add,
        (false, true) => SelectMode::Subtract,
        _ => SelectMode::Replace,
    }
}

/// Is `pos` inside the existing pixel selection?
fn inside_pixels(ctx: &ToolContext, pos: Point) -> bool {
    ctx.selection
        .pixels
        .as_ref()
        .is_some_and(|m| m.get(pos.x.floor() as i32, pos.y.floor() as i32) > 0)
}

/// A press of a bitmap selection tool inside the existing pixel selection without Shift/Alt drags
/// the marquee (Ctrl: the pixels, PAR 06/05, 06/16); returns whether the pixels go along.
fn press_moves_marquee(ev: &ToolEvent, ctx: &ToolContext, pos: Point) -> Option<bool> {
    (!ev.mods.shift && !ev.mods.alt && ctx.selection.has_pixels() && inside_pixels(ctx, pos))
        .then_some(ev.mods.ctrl)
}

/// Cursor while the marquee (or with Ctrl the pixels) is moved: arrow with a dashed rectangle or
/// with scissors (RE spec 69 section 4.2).
fn move_cursor(pixels: bool) -> Cursor {
    if pixels {
        Cursor::MovePixels
    } else {
        Cursor::MoveMarquee
    }
}

/// The tool glyph `base` (crosshair, lasso, wand) with the badge of `mode`: "+" add, "-"
/// subtract, "x" intersect, none for replace.
fn badge_cursor(base: Cursor, mode: SelectMode) -> Cursor {
    use Cursor as C;
    match (base, mode) {
        (C::Crosshair, SelectMode::Add) => C::CrosshairAdd,
        (C::Crosshair, SelectMode::Subtract) => C::CrosshairSubtract,
        (C::Crosshair, SelectMode::Intersect) => C::CrosshairIntersect,
        (C::Lasso, SelectMode::Add) => C::LassoAdd,
        (C::Lasso, SelectMode::Subtract) => C::LassoSubtract,
        (C::Lasso, SelectMode::Intersect) => C::LassoIntersect,
        (C::Wand, SelectMode::Add) => C::WandAdd,
        (C::Wand, SelectMode::Subtract) => C::WandSubtract,
        (C::Wand, SelectMode::Intersect) => C::WandIntersect,
        _ => base,
    }
}

/// Hover cursor of Marquee, Lasso and Magic Wand (RE spec 69 section 4.3, measured): only while a
/// pixel selection exists Shift shows "+", Alt "-" and Shift+Alt (either order) "x"; inside the
/// selection without Shift/Alt the move cursor (Ctrl: move the pixels). Without a pixel selection
/// always the plain glyph. `can_move`: a press inside would move the marquee. Like every hover
/// cursor it follows the next mouse move.
pub(crate) fn selection_hover_cursor(
    base: Cursor,
    ev: &ToolEvent,
    ctx: &ToolContext,
    pos: Point,
    can_move: bool,
) -> Cursor {
    if !ctx.selection.has_pixels() {
        return base;
    }
    if can_move && let Some(pixels) = press_moves_marquee(ev, ctx, pos) {
        return move_cursor(pixels);
    }
    badge_cursor(base, mode_from(ev))
}

/// Drag of the marquee inside the pixel selection: dashed outline of the moved selection bounds;
/// with Ctrl (`pixels`) the live preview of the moved pixels like the Pointer (FB-73).
fn marquee_move_overlay(ctx: &ToolContext, from: Point, to: Point, pixels: bool) -> Vec<Overlay> {
    let (dx, dy) = ((to.x - from.x).round(), (to.y - from.y).round());
    if pixels && let Some(g) = super::pointer::pixel_ghost(ctx, dx, dy) {
        return vec![g];
    }
    ctx.selection
        .pixels
        .as_ref()
        .map(|m| {
            let b = m.tight_bounds().to_rect();
            vec![Overlay::Rect {
                rect: Rect::new(b.left + dx, b.top + dy, b.right + dx, b.bottom + dy),
                dashed: true,
            }]
        })
        .unwrap_or_default()
}

/// Release of a marquee drag: `select.moveMarquee` (or `select.movePixels`), nothing without a move.
fn marquee_move_release(from: Point, to: Point, pixels: bool) -> ToolResponse {
    let (dx, dy) = ((to.x - from.x).round(), (to.y - from.y).round());
    if dx == 0.0 && dy == 0.0 {
        return ToolResponse::default();
    }
    let name = if pixels {
        "select.movePixels"
    } else {
        "select.moveMarquee"
    };
    ToolResponse::commit(vec![Invocation::new(name, json!({"dx": dx, "dy": dy}))])
}

fn edge_params(e: EdgeMode) -> (bool, u32) {
    match e {
        EdgeMode::Hard => (false, 0),
        EdgeMode::AntiAlias => (true, 0),
        EdgeMode::Feather(f) => (true, f),
    }
}

/// Bitmap als Ziel (Bitmapmodus oder genau eine ausgewählte Bitmap)?
pub(crate) fn bitmap_target(ctx: &ToolContext) -> bool {
    ctx.bitmap_mode
        || ctx.selection.elements.iter().any(|id| {
            docops::element_on_page(ctx.doc, ctx.page, *id)
                .is_some_and(|e| matches!(e.kind, ElementKind::Bitmap(_)))
        })
}

/// Esc in einem Bitmap-Auswahlwerkzeug ohne laufenden Vorgang: wie im Original „Bitmapmodus
/// beenden“ (Verlaufsschritt; Auswahlrahmen weg, Bitmap bleibt ausgewählt, Werkzeug → Zeiger;
/// UX-A L13, `a_321`). Ohne Pixelauswahl: nichts (auch im Bitmapmodus, L27).
pub(crate) fn escape_bitmap_mode(ctx: &ToolContext) -> ToolResponse {
    // Nur mit Auswahlrahmen (z. B. nach dem Zauberstab, L13): Rahmen weg, Bitmapmodus beenden,
    // Zeiger. Ohne Rahmen (Bitmapmodus per Doppelklick, L27) bleibt im Original alles, wie es ist:
    // Werkzeug Auswahlrahmen, Bitmap gewählt (PM-1, gemessen).
    // re 28 L3: wurde im Bitmap-Modus gearbeitet (Rahmen gelöscht, Pixel gelöscht, radiert), beendet
    // Esc ihn auch ohne Rahmen (Original: Strg+D, Esc → Zeiger, Bitmap beschnitten)
    if !ctx.selection.has_pixels() && !ctx.bitmap_worked {
        return ToolResponse::default();
    }
    ToolResponse {
        commands: vec![Invocation::new("modify.exitBitmapMode", json!({}))],
        switch_tool: Some(ToolId::Pointer),
        ..ToolResponse::default()
    }
}

/// Arrow keys in a bitmap selection tool with a pixel selection (re 62 §1.8): only the marquee
/// moves, 1 px (Shift 10 px), as a key nudge, so consecutive presses are one history row "Move"
/// and one undo restores the marquee. Ctrl/Alt arrows and arrows without a pixel selection are
/// left to the menu shortcuts.
pub(crate) fn nudge_marquee(ev: &ToolEvent, ctx: &ToolContext) -> Option<ToolResponse> {
    let ToolInput::Key(k) = &ev.input else {
        return None;
    };
    if ev.mods.ctrl || ev.mods.alt || !ctx.selection.has_pixels() {
        return None;
    }
    let s = if ev.mods.shift { 10.0 } else { 1.0 };
    let (dx, dy) = match k {
        ToolKey::Left => (-s, 0.0),
        ToolKey::Right => (s, 0.0),
        ToolKey::Up => (0.0, -s),
        ToolKey::Down => (0.0, s),
        _ => return None,
    };
    Some(ToolResponse::commit(vec![Invocation::new(
        "edit.move",
        json!({"dx": dx, "dy": dy, "nudge": true}),
    )]))
}

/// Escape while a new marquee or (non-polygon) lasso is dragged (re 62 §1.5, measured with the real key):
/// the drag is dropped and bitmap mode ends at once, an older pixel selection included; the Pointer
/// becomes active (the release then does nothing) and the bitmap stays selected. One row "Exit
/// Bitmap Mode"; when this very press opened bitmap mode (a bitmap selected, no mode, no pixel
/// selection) the reference writes that row twice. Without a bitmap or pixel selection only the
/// drag is dropped.
pub(crate) fn escape_selection_drag(ctx: &ToolContext) -> ToolResponse {
    let exit = || Invocation::new("modify.exitBitmapMode", json!({"row": true}));
    let commands = if ctx.selection.has_pixels() || ctx.bitmap_mode || ctx.bitmap_worked {
        vec![exit()]
    } else if bitmap_target(ctx) {
        let edit = || Invocation::new("modify.editBitmap", json!({}));
        vec![edit(), exit(), edit(), exit()]
    } else {
        return ToolResponse::default();
    };
    ToolResponse {
        commands,
        switch_tool: Some(ToolId::Pointer),
        ..ToolResponse::default()
    }
}

/// Esc in einem Malwerkzeug (Pinsel, Bleistift, Radiergummi, Weichzeichner & Co., Stempel …) ohne
/// laufenden Strich, gemessen am Original (FV-11): Ist eine Bitmap das Ziel (ausgewählt oder
/// Bitmapmodus), endet der Bitmapmodus und das Werkzeug wechselt zum Zeiger; die Bitmap bleibt
/// ausgewählt – auch ohne vorheriges Malen. Ohne Bitmap (nichts oder ein Vektor gewählt) bleibt
/// alles, wie es ist.
pub(crate) fn escape_paint_tool(ctx: &ToolContext) -> ToolResponse {
    if !bitmap_target(ctx) {
        return ToolResponse::default();
    }
    let commands = if ctx.bitmap_mode || ctx.selection.has_pixels() || ctx.bitmap_worked {
        vec![Invocation::new("modify.exitBitmapMode", json!({}))]
    } else {
        Vec::new()
    };
    ToolResponse {
        commands,
        switch_tool: Some(ToolId::Pointer),
        ..ToolResponse::default()
    }
}

/// Esc nach einem Malzug (Pinsel 01/14, Buntstift/Radiergummi 03/16, Retusche): Zeiger und Schritt
/// „Bitmapmodus beenden“, die Bitmap bleibt ausgewählt (gemessen am Original).
pub(crate) fn escape_after_stroke(ctx: &ToolContext) -> Option<ToolResponse> {
    (bitmap_target(ctx) && !ctx.bitmap_mode && !ctx.selection.has_pixels()).then(|| ToolResponse {
        commands: vec![
            Invocation::new("modify.editBitmap", json!({})),
            Invocation::new("modify.exitBitmapMode", json!({"step": true})),
        ],
        switch_tool: Some(ToolId::Pointer),
        ..ToolResponse::default()
    })
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
    /// Ziehen im vorhandenen Rahmen: Startpunkt und ob die Pixel mitgehen (Strg) – sonst nur der Rahmen.
    moving: Option<(Point, bool)>,
}

impl MarqueeTool {
    /// Neues Werkzeug (Marquee, OvalMarquee).
    pub fn new(id: ToolId) -> MarqueeTool {
        MarqueeTool {
            id,
            start: None,
            cur: None,
            overlay: Vec::new(),
            moving: None,
        }
    }

    /// Verknüpfung beim Drücken wie im Original (PAR 06/03): ohne vorhandenen Rahmen ersetzt jeder
    /// Zug (Umschalt = Quadrat, Alt = von der Mitte); mit Rahmen Umschalt addiert, Alt zieht ab,
    /// both intersect in either order (RE spec 69 section 4.2: Alt first, then Shift, also writes
    /// `setSelectionMask(…, "intersect")`).
    fn press_mode(&self, ev: &ToolEvent, ctx: &ToolContext) -> SelectMode {
        if !ctx.selection.has_pixels() {
            return SelectMode::Replace;
        }
        mode_from(ev)
    }

    fn rect(
        &self,
        start: Point,
        cur: Point,
        mode: SelectMode,
        ev: &ToolEvent,
        ctx: &ToolContext,
    ) -> Rect {
        match ctx.options.marquee_style {
            MarqueeStyle::FixedSize(w, h) => Rect::new(cur.x, cur.y, cur.x + w, cur.y + h),
            MarqueeStyle::FixedRatio(rx, ry) if rx > 0.0 => {
                let w = cur.x - start.x;
                let h = w.abs() * ry / rx * if cur.y < start.y { -1.0 } else { 1.0 };
                Rect::new(start.x, start.y, start.x + w, start.y + h).normalized()
            }
            _ => constrain_rect(
                start,
                cur,
                ev.mods.shift && mode == SelectMode::Replace,
                ev.mods.alt && mode == SelectMode::Replace,
            ),
        }
    }

    fn shape(&self, r: Rect) -> Overlay {
        if self.id == ToolId::OvalMarquee {
            Overlay::Ellipse {
                rect: r,
                dashed: true,
            }
        } else {
            Overlay::Rect {
                rect: r,
                dashed: true,
            }
        }
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
            if self.start.is_none()
                && self.moving.is_none()
                && let Some(r) = nudge_marquee(ev, ctx)
            {
                return r;
            }
            if ev.input == ToolInput::Key(ToolKey::Escape) {
                if self.start.is_none() {
                    // Esc after dragging a marquee: "Exit bitmap mode", pixel selection gone,
                    // Pointer, the bitmap STAYS selected like Lasso and Magic Wand (re 58 §6,
                    // measured with the real Escape key; the earlier TW-04 rule came from a key
                    // path that sent Escape as a letter)
                    return escape_bitmap_mode(ctx);
                }
                self.start = None;
                self.cur = None;
                self.overlay.clear();
                return escape_selection_drag(ctx);
            }
            return ToolResponse::default();
        };
        let ellipse = self.id == ToolId::OvalMarquee;
        let p = pixel_snap(ctx, *pos);
        let fixed = matches!(ctx.options.marquee_style, MarqueeStyle::FixedSize(..));
        match kind {
            PointerKind::Down => {
                // im vorhandenen Rahmen: Ziehen verschiebt den Rahmen, Strg+Ziehen die Pixel (PAR 06/05, 06/16)
                if !fixed && let Some(pixels) = press_moves_marquee(ev, ctx, *pos) {
                    self.moving = Some((p, pixels));
                    return ToolResponse {
                        cursor: move_cursor(pixels),
                        ..ToolResponse::default()
                    };
                }
                let m = self.press_mode(ev, ctx);
                self.start = Some((p, m));
                self.cur = None;
                ToolResponse {
                    cursor: badge_cursor(Cursor::Crosshair, m),
                    ..ToolResponse::default()
                }
            }
            PointerKind::Move if self.moving.is_some() => {
                let (s, pixels) = self.moving.unwrap_or((p, false));
                self.overlay = marquee_move_overlay(ctx, s, p, pixels);
                ToolResponse::with_overlay(self.overlay.clone(), move_cursor(pixels))
            }
            PointerKind::Up if self.moving.is_some() => {
                self.overlay.clear();
                let (s, pixels) = self.moving.take().unwrap_or((p, false));
                marquee_move_release(s, p, pixels)
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
                let cursor = match self.start {
                    Some((_, m)) => badge_cursor(Cursor::Crosshair, m),
                    None => selection_hover_cursor(Cursor::Crosshair, ev, ctx, *pos, !fixed),
                };
                ToolResponse::with_overlay(self.overlay.clone(), cursor)
            }
            PointerKind::Up => {
                self.overlay.clear();
                let Some((s, m)) = self.start.take() else {
                    return ToolResponse::default();
                };
                if self.cur.take().is_none() && !fixed && dist(s, *pos) < ctx.drag_threshold() {
                    return if m == SelectMode::Replace && ctx.selection.has_pixels() {
                        ToolResponse::commit(vec![Invocation::new(
                            "select.pixels",
                            json!({"shape": "none"}),
                        )])
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
    /// Drag inside the pixel selection: start and whether the pixels go along (Ctrl).
    moving: Option<(Point, bool)>,
    /// Cursor while the outline is drawn (the badge only with a pixel selection).
    draw_cursor: Cursor,
}

impl LassoTool {
    /// Neues Werkzeug (Lasso, PolygonLasso).
    pub fn new(id: ToolId) -> LassoTool {
        LassoTool {
            id,
            points: Vec::new(),
            mode: SelectMode::Replace,
            active: false,
            hover: None,
            moving: None,
            draw_cursor: Cursor::Lasso,
        }
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

    /// Cursor while the outline is drawn: the lasso, with a pixel selection with the badge of
    /// its combine mode.
    fn cursor(&self) -> Cursor {
        self.draw_cursor
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
        if let ToolInput::Pointer { kind, pos } = &ev.input
            && let Some((from, pixels)) = self.moving
        {
            let p = pixel_snap(ctx, *pos);
            return match kind {
                PointerKind::Up => {
                    self.moving = None;
                    marquee_move_release(from, p, pixels)
                }
                _ => ToolResponse::with_overlay(
                    marquee_move_overlay(ctx, from, p, pixels),
                    move_cursor(pixels),
                ),
            };
        }
        match &ev.input {
            ToolInput::Pointer {
                kind: PointerKind::Down,
                pos,
            } => {
                // inside the pixel selection: drag the marquee (Ctrl: the pixels), RE spec 69 §4.3;
                // the Polygon Lasso (not measured) keeps starting its outline there
                if !self.active
                    && !poly
                    && let Some(pixels) = press_moves_marquee(ev, ctx, *pos)
                {
                    self.moving = Some((pixel_snap(ctx, *pos), pixels));
                    return ToolResponse {
                        cursor: move_cursor(pixels),
                        ..ToolResponse::default()
                    };
                }
                if !self.active {
                    self.draw_cursor = selection_hover_cursor(Cursor::Lasso, ev, ctx, *pos, false);
                    self.mode = mode_from(ev);
                    self.active = true;
                    self.points.clear();
                }
                if poly
                    && self.points.len() >= 3
                    && dist(self.points[0], *pos) <= ctx.pick_tolerance() as f32
                {
                    return self.commit(ctx);
                }
                let p = match self.points.last() {
                    Some(l) if poly && ev.mods.shift => constrain_angle(*l, *pos, 45.0),
                    _ => *pos,
                };
                self.points.push(p);
                ToolResponse::with_overlay(self.overlay(), self.cursor())
            }
            ToolInput::Pointer {
                kind: PointerKind::Move,
                pos,
            } if self.active && !poly => {
                if self.points.last().is_none_or(|l| dist(*l, *pos) >= 0.5) {
                    self.points.push(*pos);
                }
                ToolResponse::with_overlay(self.overlay(), self.cursor())
            }
            ToolInput::Pointer {
                kind: PointerKind::Move,
                pos,
            } if self.active && poly => {
                self.hover = Some(match self.points.last() {
                    Some(l) if ev.mods.shift => constrain_angle(*l, *pos, 45.0),
                    _ => *pos,
                });
                ToolResponse::with_overlay(self.overlay(), self.cursor())
            }
            ToolInput::Pointer {
                kind: PointerKind::Up,
                pos,
            } if self.active && !poly => {
                self.points.push(*pos);
                self.commit(ctx)
            }
            ToolInput::Pointer {
                kind: PointerKind::DoubleClick,
                pos,
            } if poly && self.active => {
                // Doppelklick an einer neuen Stelle setzt dort noch den letzten Punkt
                if self
                    .points
                    .last()
                    .is_none_or(|l| dist(*l, *pos) > ctx.pick_tolerance() as f32)
                {
                    self.points.push(*pos);
                }
                self.commit(ctx)
            }
            ToolInput::Key(ToolKey::Enter) if poly && self.active => self.commit(ctx),
            ToolInput::Pointer {
                kind: PointerKind::Move,
                pos,
            } if !self.active => ToolResponse::with_overlay(
                Vec::new(),
                selection_hover_cursor(Cursor::Lasso, ev, ctx, *pos, !poly),
            ),
            ToolInput::Key(_) if !self.active && nudge_marquee(ev, ctx).is_some() => {
                nudge_marquee(ev, ctx).unwrap_or_default()
            }
            ToolInput::Key(ToolKey::Escape) => {
                if !self.active {
                    return escape_bitmap_mode(ctx);
                }
                self.points.clear();
                self.active = false;
                self.hover = None;
                if !poly {
                    return escape_selection_drag(ctx);
                }
                if bitmap_target(ctx) {
                    // offenes Polygon-Lasso: verwerfen und wie im Original den Bitmapmodus beenden,
                    // Werkzeug → Zeiger, die Bitmap bleibt gewählt (PAR 06/08)
                    let commands = if ctx.bitmap_mode || ctx.selection.has_pixels() {
                        vec![Invocation::new("modify.exitBitmapMode", json!({}))]
                    } else {
                        Vec::new()
                    };
                    return ToolResponse {
                        commands,
                        switch_tool: Some(ToolId::Pointer),
                        ..ToolResponse::default()
                    };
                }
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
        if pts.is_empty() {
            Vec::new()
        } else {
            vec![Overlay::Polyline {
                points: pts,
                closed: true,
            }]
        }
    }
}

/// Zauberstab: Klick wählt ähnliche, zusammenhängende Pixel der Bitmap (`WandToolTolerance`).
#[derive(Clone, Debug, Default)]
pub struct WandTool {
    /// Drag inside the pixel selection: start and whether the pixels go along (Ctrl).
    moving: Option<(Point, bool)>,
}

impl ToolMachine for WandTool {
    fn id(&self) -> ToolId {
        ToolId::MagicWand
    }

    fn handle(&mut self, ev: &ToolEvent, ctx: &ToolContext) -> ToolResponse {
        if let ToolInput::Pointer { kind, pos } = &ev.input
            && let Some((from, pixels)) = self.moving
        {
            let p = pixel_snap(ctx, *pos);
            return match kind {
                PointerKind::Up => {
                    self.moving = None;
                    marquee_move_release(from, p, pixels)
                }
                _ => ToolResponse::with_overlay(
                    marquee_move_overlay(ctx, from, p, pixels),
                    move_cursor(pixels),
                ),
            };
        }
        match &ev.input {
            // inside the pixel selection: drag the marquee (Ctrl: the pixels), RE spec 69 §4.3
            ToolInput::Pointer {
                kind: PointerKind::Down,
                pos,
            } if ctx.has_document && press_moves_marquee(ev, ctx, *pos).is_some() => {
                let pixels = ev.mods.ctrl;
                self.moving = Some((pixel_snap(ctx, *pos), pixels));
                ToolResponse {
                    cursor: move_cursor(pixels),
                    ..ToolResponse::default()
                }
            }
            ToolInput::Pointer {
                kind: PointerKind::Move,
                pos,
            } if ctx.has_document => ToolResponse {
                cursor: selection_hover_cursor(Cursor::Wand, ev, ctx, *pos, true),
                ..ToolResponse::default()
            },
            ToolInput::Pointer {
                kind: PointerKind::Down,
                pos,
            } if ctx.has_document => {
                let (aa, fe) = edge_params(ctx.options.wand_edge);
                ToolResponse {
                    commands: vec![Invocation::new(
                        "select.magicWand",
                        json!({"point": pt_json(*pos), "tolerance": ctx.options.wand_tolerance, "mode": mode_from(ev).name(), "antialias": aa, "feather": fe}),
                    )],
                    cursor: selection_hover_cursor(Cursor::Wand, ev, ctx, *pos, false),
                    ..ToolResponse::default()
                }
            }
            ToolInput::Key(ToolKey::Escape) if ctx.has_document => escape_bitmap_mode(ctx),
            _ => ToolResponse {
                cursor: Cursor::Wand,
                ..ToolResponse::default()
            },
        }
    }
}

/// Pinsel, Bleistift (1 px) und Radiergummi: malen **live** (PINSEL-1, N31). Beim Drücken legt das
/// Werkzeug den vollständigen Auftrag fest (Pinsel, Farbe, Deckkraft, Mischmodus, „Transparenz
/// beibehalten“ bzw. Radiergummi-Werte) und rechnet ab dann je Bewegung nur die betroffenen Kacheln
/// ([`crate::paint::LivePaint`], Vorschau [`Overlay::Paint`]). Loslassen schickt denselben Auftrag
/// als **einen** Befehl (`bitmap.paint` bzw. `bitmap.erase`); Esc verwirft ihn (Bleistift,
/// Radiergummi). Der Pinsel folgt den echten Zügen des Originals (PAR-X-BRUSH-1, `real_strokes.txt`):
/// Esc mitten im Zug bricht nicht ab, gerade Spitzengrößen liegen um +1/+1 versetzt, Umschalt rastet
/// auf 45°, ein Zug außerhalb der gewählten Bitmap malt ein neues Bitmap bzw. (Transparenz
/// beibehalten) nichts und hebt die Auswahl auf.
#[derive(Clone, Debug)]
pub struct BrushTool {
    id: ToolId,
    active: bool,
    hover: Option<Point>,
    /// Parameter des abschließenden Befehls (ohne Punkte), beim Drücken festgelegt.
    params: Value,
    /// Punkte des laufenden Zugs (auch ohne Vorschau, z. B. gesperrte Bitmap).
    points: Vec<Point>,
    /// Zeitstempel (ms) der Mauspunkte (Geschwindigkeit, 55-px-Fenster nach re §2.2).
    times: Vec<u64>,
    /// Mauspunkte ohne Versatz/Umschalt-Raster (Weg für die Geschwindigkeit).
    raw: Vec<Point>,
    /// Geschwindigkeit (`VELf`) je Punkt von `points`; leer = Zug ohne Zeitstempel (Vorgabe).
    vels: Vec<f32>,
    /// Ansichtszoom beim Drücken (Weg in Bildschirmpixeln).
    zoom: f64,
    live: Option<Box<crate::paint::LivePaint>>,
    /// Versatz der Punkte gegenüber der Maus (Pinsel mit gerader Größe: +1/+1 wie im Original).
    offset: f32,
    /// Pinsel: Zug beginnt außerhalb der gewählten Bitmap mit „Transparenz beibehalten“ – nichts
    /// malen, beim Loslassen die Auswahl aufheben.
    deselect_only: bool,
    /// Pinsel: der letzte Zug wurde gemalt (Esc danach beendet den Bitmapmodus mit Verlaufsschritt).
    painted: bool,
    /// Pinsel: Escape kam mitten im Zug (re 62 §3.4). Der Zug läuft weiter und wird beim Loslassen
    /// ganz gemalt, die Vorschau bleibt aber auf dem Stand beim Escape stehen; der Zeiger ist dann
    /// schon aktiv (die Sitzung gibt diesem Werkzeug nur noch Bewegungen und das Loslassen).
    escaped: bool,
    /// Eingefrorene Vorschau seit dem Escape.
    frozen: Option<Overlay>,
    /// FB-31 (Firepixel extension): the stroke is paused (P); the button may be released, the next
    /// press resumes the same stroke as a new segment.
    paused: bool,
    /// FB-31: indices into `points` where the stroke was resumed after a pause.
    breaks: Vec<usize>,
    /// FB-71: index in `points` the Shift line starts from: the segment start when Shift was held
    /// at the press (or the resume), the point reached when Shift went down mid-stroke (the part
    /// painted before stays); `None` while Shift is up.
    line_anchor: Option<usize>,
    /// FB-69 (Firepixel extension): subtraction passes of the stroke (Alt), for the command.
    subtract: Vec<crate::paint::SubtractPass>,
    /// FB-69: a subtraction pass is running (Alt held with the button down).
    subtracting: bool,
    /// FB-69: Alt was held at the press that began the stroke (the Brush has no Alt function at
    /// the press, so it paints); it starts subtracting only after Alt was released once.
    alt_latch: bool,
}

/// Distance (px) of the stroke start from the selected bitmap from which the brush paints a new
/// bitmap: assumed between the measurements 60 px -> the selected bitmap grows to include the
/// detached stroke (RE spec 70 section 1.8) and 100 px -> new bitmap (T03).
pub const OUTSIDE_NEW_BITMAP: f32 = 80.0;

/// Strichtextur des Pinsels, solange im Inspektor keine gewählt ist (Liste des Originals zeigt „Ader“).
pub const DEFAULT_STROKE_TEXTURE: &str = "Vein";

/// Startwert eines neuen Pinselzugs (je Zug verschieden wie im Original).
fn stroke_seed() -> u32 {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() ^ d.as_secs() as u32)
        .unwrap_or(0);
    crate::paint::nonzero_seed(t.wrapping_add(N.fetch_add(0x9e37_79b9, Ordering::Relaxed)))
}

/// Umschalt beim Pinsel (gemessen, `real_strokes.txt` r12): Richtung rastet in 45°-Schritten, die
/// Länge je Achse ist die größere Komponente (60,150 → 300,260 ergibt 300,390).
pub fn snap_brush_line(a: Point, b: Point) -> Point {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let m = dx.abs().max(dy.abs());
    if m == 0.0 {
        return a;
    }
    let k = (dy.atan2(dx) / std::f32::consts::FRAC_PI_4).round();
    let ang = k * std::f32::consts::FRAC_PI_4;
    let (s, c) = ang.sin_cos();
    let unit = |v: f32| {
        if v > 0.5 {
            1.0
        } else if v < -0.5 {
            -1.0
        } else {
            0.0
        }
    };
    Point::new(a.x + unit(c) * m, a.y + unit(s) * m)
}

impl BrushTool {
    /// Neues Werkzeug (Brush, Pencil, Eraser).
    pub fn new(id: ToolId) -> BrushTool {
        BrushTool {
            id,
            active: false,
            hover: None,
            params: Value::Null,
            points: Vec::new(),
            times: Vec::new(),
            raw: Vec::new(),
            vels: Vec::new(),
            zoom: 1.0,
            live: None,
            offset: 0.0,
            deselect_only: false,
            painted: false,
            escaped: false,
            frozen: None,
            paused: false,
            breaks: Vec::new(),
            line_anchor: None,
            subtract: Vec::new(),
            subtracting: false,
            alt_latch: false,
        }
    }

    /// Pinsel bei ausgewähltem Vektor (ohne Bitmap als Ziel): Das Original zeigt im Inspektor den
    /// Strich des Vektors und malt damit (gemessen T04/R1 ohne Strich: 1 px, Annahme
    /// „Inspektorstrich“); ohne Strich gilt „1-Pixel hart“.
    fn vector_brush(ctx: &ToolContext) -> Option<fp_model::Brush> {
        if crate::paint::target_bitmap(ctx.doc, ctx.page, &ctx.selection.elements).is_some() {
            return None;
        }
        let e = docops::paths_sorted(ctx.doc, ctx.page, &ctx.selection.elements)
            .into_iter()
            .filter(|p| !docops::is_locked(ctx.doc, ctx.page, p))
            .find_map(|p| {
                ctx.doc
                    .element_at(&p)
                    .filter(|e| !matches!(e.kind, ElementKind::Bitmap(_)))
                    .cloned()
            })?;
        let a = docops::path_attrs_of(&e)?;
        Some(a.brush.clone().unwrap_or_else(|| {
            crate::commands::panel::tool_brush_from(
                Some(&json!({"preset": "Bleistift/1-Pixel hart", "width": 1})),
                None,
            )
        }))
    }

    fn paint_tool(&self) -> crate::paint::PaintTool {
        match self.id {
            ToolId::Eraser => crate::paint::PaintTool::Eraser,
            ToolId::Pencil => crate::paint::PaintTool::Pencil,
            _ => crate::paint::PaintTool::Brush,
        }
    }

    fn command(&self) -> &'static str {
        if self.id == ToolId::Eraser {
            "bitmap.erase"
        } else {
            "bitmap.paint"
        }
    }

    /// Auftrag aus den Werkzeugwerten (Inspektor) beim Drücken; `start_px` = Pixel der Zielbitmap
    /// unter dem Startpunkt (Bleistift „Automatisch löschen“).
    fn job_params(&self, ctx: &ToolContext, start_px: Option<[u8; 4]>) -> Value {
        let o = ctx.options;
        let hex = |c: fp_color::Argb| c.to_rgba8().to_hex_rgba();
        match self.id {
            ToolId::Eraser => {
                json!({"size": o.eraser_size, "round": o.eraser_round, "opacity": o.eraser_opacity, "edge": o.eraser_edge})
            }
            ToolId::Pencil => {
                let mut b = fp_model::Brush::basic(1);
                b.antialiased = o.pencil_antialias;
                let s = ctx.stroke_color;
                let same = start_px
                    .is_some_and(|p| p[3] > 0 && p[0] == s.r() && p[1] == s.g() && p[2] == s.b());
                let color = if o.auto_erase && same {
                    ctx.fill_color
                } else {
                    s
                };
                // Firepixel option "Confine to boundaries" (FB-52)
                let confine = ctx.prefs.get_bool("PencilToolConfineToBounds", false);
                json!({"tool": "pencil", "brush": b, "color": hex(color), "opacity": o.opacity, "blend": o.paint_blend, "preserveTransparency": o.keep_transparency, "trimEnd": false, "confineToBounds": confine})
            }
            _ => {
                let st = o.new_stroke.as_ref();
                let mut b = match Self::vector_brush(ctx) {
                    Some(v) => v,
                    None => {
                        let mut b = crate::commands::panel::tool_brush(o);
                        if let Some(a) = st
                            .and_then(|s| s.get("textureAmount"))
                            .and_then(|v| v.as_i64())
                        {
                            b.texture_blend = (a.clamp(0, 100) * 10) as i32;
                        }
                        b
                    }
                };
                b.diameter = b.diameter.max(1);
                // echte Züge malen bis einschließlich Endpunkt (r03: 60→500 = 441 px), eigener
                // Zufalls-Startwert je Zug (natürliche Pinsel)
                // Firepixel option "Confine to boundaries" (FB-52)
                let confine = ctx.prefs.get_bool("BrushToolConfineToBounds", false);
                let mut p = json!({"brush": b, "color": hex(ctx.stroke_color), "opacity": o.opacity, "blend": o.paint_blend, "preserveTransparency": o.keep_transparency, "trimEnd": false, "seed": stroke_seed(), "confineToBounds": confine});
                // Textur der Strichvorgabe; ohne Wahl wie im Original „Ader“ (Vein, Vorgabe der Liste),
                // damit texturierte Arten (Graphit, Kohlestift …) ihre Textur zeigen
                let tex = st
                    .and_then(|s| s.get("texture"))
                    .and_then(|v| v.as_str())
                    .filter(|t| !t.is_empty())
                    .unwrap_or(DEFAULT_STROKE_TEXTURE);
                p["texture"] = json!(tex);
                p
            }
        }
    }

    /// Beginnt den Zug: Auftrag festlegen, Zielbitmap lesen, ersten Abdruck rechnen.
    fn begin(&mut self, ctx: &ToolContext, pos: Point, time_ms: u64) {
        self.active = true;
        self.zoom = ctx.zoom;
        self.raw = vec![pos];
        self.times = vec![time_ms];
        // Zug mit Zeitstempeln (echte Maus): Geschwindigkeit aus der Bewegung, am Start 0;
        // ohne (Skripte, Tests): Vorgabe wie erzeugte Pfade
        self.vels = if time_ms > 0 && self.id == ToolId::Brush {
            vec![0.0]
        } else {
            Vec::new()
        };
        self.deselect_only = false;
        self.painted = false;
        self.offset = 0.0;
        // FB-65: a Brush or Pencil stroke that starts on a bitmap paints into it
        let at = (self.id != ToolId::Eraser).then_some(pos);
        let mut target =
            crate::paint::stroke_target(ctx.doc, ctx.page, ctx.frame, &ctx.selection.elements, at);
        let mut base = target.as_ref().and_then(|(_, e)| match &e.kind {
            ElementKind::Bitmap(b) => docops::bitmap_image(b),
            _ => None,
        });
        let start_px = base
            .as_ref()
            .and_then(|b| b.get(pos.x.floor() as i32, pos.y.floor() as i32));
        self.params = self.job_params(ctx, start_px);
        if let Some(a) = at {
            self.params["at"] = json!([a.x, a.y]);
        }
        self.live = None;
        if self.id == ToolId::Brush {
            // gerade Größen +1/+1 (echte Züge r02/r13/r16: Größe 10 → 56,146–506,156 für 60→500)
            let d = self.params["brush"]["diameter"].as_i64().unwrap_or(1);
            self.offset = if d % 2 == 0 { 1.0 } else { 0.0 };
            // Start deutlich außerhalb der gewählten Bitmap (gemessen T03, Start 100,250 = 100 px
            // neben den deckenden Pixeln): mit „Transparenz beibehalten“ nichts malen und die Auswahl
            // aufheben, sonst ein neues Bitmap. Knapp daneben (FV-11: 10 bzw. 19 px) wächst die
            // gewählte Bitmap wie bisher – Grenze dazwischen angenommen (OUTSIDE_NEW_BITMAP).
            let outside = base.as_ref().is_some_and(|b| {
                let r = b.bounds();
                let dx = (r.left as f32 - pos.x).max(pos.x - r.right as f32).max(0.0);
                let dy = (r.top as f32 - pos.y).max(pos.y - r.bottom as f32).max(0.0);
                dx.max(dy) > OUTSIDE_NEW_BITMAP
            });
            if outside {
                if ctx.options.keep_transparency {
                    self.deselect_only = true;
                    self.points = vec![pos];
                    return;
                }
                target = None;
                base = None;
                self.params["newBitmap"] = json!(true);
            }
        }
        let pos = Point::new(pos.x + self.offset, pos.y + self.offset);
        self.points = vec![pos];
        // Radiergummi ohne (ungesperrte) Bitmap: nichts zu radieren, der Befehl meldet es
        if self.id == ToolId::Eraser && target.is_none() {
            return;
        }
        // FB-64: no visible unlocked layer for a new bitmap → no live stroke (the command refuses)
        if target.is_none()
            && docops::paint_target_layer(ctx.doc, ctx.page, ctx.frame, ctx.layer).is_err()
        {
            return;
        }
        let mut p = self.params.clone();
        p["points"] = json!([[pos.x, pos.y]]);
        if !self.vels.is_empty() {
            p["velocities"] = json!(self.vels);
        }
        let size = if self.id == ToolId::Eraser {
            ctx.options.eraser_size
        } else {
            ctx.options.brush_size
        };
        let job =
            crate::paint::PaintJob::from_params(self.paint_tool(), &p, ctx.stroke_color, size);
        let sel = ctx
            .selection
            .pixels
            .clone()
            .filter(|_| ctx.selection.has_pixels());
        let backdrop = ctx
            .doc
            .pages
            .get(ctx.page)
            .map(|pg| pg.root.background)
            .unwrap_or(fp_color::Argb(0xffff_ffff));
        self.live = Some(Box::new(crate::paint::LivePaint::new(
            job,
            base,
            sel,
            target.map(|t| t.0),
            backdrop,
        )));
    }

    /// Punkt anhängen bzw. (Umschalt) gerade Linie vom Startpunkt.
    fn extend(&mut self, pos: Point, line: bool, time_ms: u64) {
        if self.deselect_only {
            return;
        }
        // Zeigergeschwindigkeit des neuen Mauspunkts (nur bei Zügen mit Zeitstempeln)
        let vel = if self.vels.is_empty() || self.raw.last() == Some(&pos) {
            None
        } else {
            self.raw.push(pos);
            self.times.push(time_ms);
            Some(
                crate::paint::pointer_velocity(&self.raw, &self.times, self.zoom)
                    .unwrap_or_else(|| *self.vels.last().unwrap_or(&0.0)),
            )
        };
        let pos = Point::new(pos.x + self.offset, pos.y + self.offset);
        if line {
            // straight line from the anchor: the first point of the running segment when Shift was
            // held at the press (FB-31: of the resumed part), otherwise the point reached when Shift
            // went down; everything painted before it stays (FB-71)
            let start = self.breaks.last().copied().unwrap_or(0);
            let seg = *self
                .line_anchor
                .get_or_insert(self.points.len().saturating_sub(1));
            let seg = seg.max(start);
            let Some(&first) = self.points.get(seg) else {
                return;
            };
            let pos = if self.id == ToolId::Brush {
                snap_brush_line(first, pos)
            } else {
                pos
            };
            self.points.truncate(seg + 1);
            self.points.push(pos);
            if let Some(v) = vel {
                self.vels.truncate(seg + 1);
                self.vels.push(v);
            }
            if let Some(l) = self.live.as_mut() {
                if vel.is_some() {
                    l.job.velocities = self.vels.clone();
                }
                l.set_points(self.points.clone());
            }
        } else if self.points.last().is_none_or(|l| *l != pos) {
            self.line_anchor = None;
            self.points.push(pos);
            if let Some(v) = vel {
                self.vels.push(v);
            }
            if let Some(l) = self.live.as_mut() {
                l.push_with_velocity(pos, vel);
            }
        }
    }

    /// FB-31: resumes the paused stroke at `pos` – a new segment of the same stroke, not joined to
    /// the last point by a line.
    fn resume(&mut self, pos: Point, time_ms: u64, shift: bool) {
        self.paused = false;
        self.subtracting = false;
        let p = Point::new(pos.x + self.offset, pos.y + self.offset);
        // the speed starts at 0 like at a press (no jump from the last point)
        let vel = (!self.vels.is_empty()).then_some(0.0);
        self.raw = vec![pos];
        self.times = vec![time_ms];
        if let Some(v) = vel {
            // one speed per point, so the new segment's speeds line up with its points
            let fill = self.vels.last().copied().unwrap_or(0.0);
            self.vels.resize(self.points.len(), fill);
            self.vels.push(v);
        }
        self.breaks.push(self.points.len());
        // Shift held at the resume: the line starts at the new segment's first point
        self.line_anchor = shift.then_some(self.points.len());
        self.points.push(p);
        if let Some(l) = self.live.as_mut() {
            l.start_segment(p, vel);
        }
    }

    /// FB-69: can the running stroke subtract its own paint (Brush and Pencil; not a stroke that
    /// paints nothing, not after Escape)?
    fn can_subtract(&self) -> bool {
        matches!(self.id, ToolId::Brush | ToolId::Pencil)
            && self.active
            && !self.deselect_only
            && !self.escaped
            && !self.points.is_empty()
    }

    /// FB-69: one subtraction step at `pos` (Alt held): starts a pass or extends the running one.
    fn subtract_at(&mut self, pos: Point) {
        let p = Point::new(pos.x + self.offset, pos.y + self.offset);
        if !self.subtracting {
            let (size, round) = match self.live.as_ref() {
                Some(l) => (l.job.brush.diameter.max(1) as f32, l.job.brush.shape != 0),
                None => (1.0, true),
            };
            self.subtracting = true;
            self.line_anchor = None;
            self.subtract.push(crate::paint::SubtractPass {
                at: self.points.len(),
                size,
                round,
                points: vec![p],
            });
            if let Some(l) = self.live.as_mut() {
                l.start_subtract(p, size, round);
            }
        } else if let Some(pass) = self.subtract.last_mut()
            && pass.points.last() != Some(&p)
        {
            pass.points.push(p);
            if let Some(l) = self.live.as_mut() {
                l.push_subtract(p);
            }
        }
    }

    /// Applies the running stroke: the command with all points (and the segments of a resumed
    /// stroke, FB-31) as one history row.
    fn commit(&mut self, ctx: &ToolContext) -> ToolResponse {
        let mut params = std::mem::take(&mut self.params);
        params["points"] = Value::Array(self.points.iter().map(|p| pt_json(*p)).collect());
        if !self.vels.is_empty() {
            params["velocities"] = json!(self.vels);
        }
        if !self.breaks.is_empty() {
            params["breaks"] = json!(self.breaks);
        }
        if !self.subtract.is_empty() {
            params["subtract"] = Value::Array(self.subtract.iter().map(|p| p.to_json()).collect());
        }
        let escaped = self.escaped;
        self.cancel();
        // Pinsel (01/14), Buntstift und Radiergummi (03/16): Esc danach → „Bitmapmodus beenden“
        self.painted = true;
        let mut commands = vec![Invocation::new(self.command(), params)];
        if escaped && bitmap_target(ctx) {
            // Esc im Zug: der Zeiger ist schon aktiv, der Bitmapmodus bleibt bis zum nächsten
            // Esc offen (dann „Bitmapmodus beenden“, re 62 §1.4)
            commands.push(Invocation::new(
                "modify.editBitmap",
                json!({"worked": true}),
            ));
        }
        ToolResponse {
            commands,
            cursor: self.cursor(),
            overlay: self.overlay_for(ctx),
            ..ToolResponse::default()
        }
    }

    fn size(&self, ctx: &ToolContext) -> f32 {
        match self.id {
            ToolId::Pencil => 1.0,
            ToolId::Eraser => ctx.options.eraser_size,
            _ => crate::commands::panel::tool_brush(ctx.options)
                .diameter
                .max(1) as f32,
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
        if let Some(f) = &self.frozen {
            v.push(f.clone());
        } else if !self.points.is_empty()
            && let Some(l) = &self.live
        {
            v.push(Overlay::Paint(l.preview()));
        }
        if let Some(h) = self.hover
            && !self.escaped
            && ctx.prefs.get_bool("UseBrushSizeCursors", true)
            && self.id != ToolId::Pencil
        {
            v.push(Overlay::BrushTip {
                at: h,
                diameter: self.size(ctx),
                round: self.id != ToolId::Eraser || ctx.options.eraser_round,
            });
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
        // FB-31 (Firepixel extension): P pauses the running stroke; while paused, Enter applies
        // it and so does Escape (safe default, nothing is lost)
        match ev.input {
            ToolInput::Key(ToolKey::Pause) => {
                if self.stroke_pause() == StrokePause::Painting {
                    self.paused = true;
                    return ToolResponse::with_overlay(self.overlay_for(ctx), self.cursor());
                }
                return ToolResponse::default();
            }
            ToolInput::Key(ToolKey::Enter | ToolKey::Escape) if self.paused => {
                return self.commit(ctx);
            }
            _ => {}
        }
        if ev.input == ToolInput::Key(ToolKey::Escape) {
            if self.active && self.id == ToolId::Brush {
                // Pinsel: Esc mitten im Zug bricht nicht ab (gemessen r14_esc_mid = ganzer Strich),
                // aber das Werkzeug wechselt sofort zum Zeiger und die Vorschau friert ein; der Zug
                // wird beim Loslassen trotzdem ganz gemalt (re 62 §3.4)
                if !self.escaped {
                    self.escaped = true;
                    self.frozen = self
                        .live
                        .take()
                        .filter(|_| !self.points.is_empty())
                        .map(|l| Overlay::Paint(l.preview()));
                }
                return ToolResponse {
                    overlay: self.overlay_for(ctx),
                    switch_tool: Some(ToolId::Pointer),
                    keep_gesture: true,
                    ..ToolResponse::default()
                };
            }
            if self.active {
                // Zug verwerfen: Vorschau weg, kein Befehl, kein Verlaufsschritt
                self.cancel();
                return ToolResponse::with_overlay(self.overlay_for(ctx), self.cursor());
            }
            if std::mem::take(&mut self.painted)
                && let Some(r) = escape_after_stroke(ctx)
            {
                return r;
            }
            return escape_paint_tool(ctx);
        }
        let ToolInput::Pointer { kind, pos } = &ev.input else {
            return ToolResponse::default();
        };
        self.hover = Some(*pos);
        if !ev.mods.alt {
            self.alt_latch = false;
        }
        // FB-69 (Firepixel extension): Alt while painting or while paused subtracts: the brush tip
        // erases only what this stroke has painted; releasing Alt paints again (a new segment)
        let alt = ev.mods.alt && !self.alt_latch && !ev.mods.space && self.can_subtract();
        match kind {
            PointerKind::Down if self.paused && alt => self.subtract_at(*pos),
            PointerKind::Move if self.paused && self.subtracting => {
                if alt {
                    self.subtract_at(*pos);
                } else {
                    // Alt released with the button held: painting resumes here
                    self.resume(*pos, ev.time_ms, ev.mods.shift);
                }
            }
            PointerKind::Up if self.paused && self.subtracting => {
                if alt {
                    self.subtract_at(*pos);
                }
                // the stroke stays paused
                self.subtracting = false;
            }
            PointerKind::Move | PointerKind::Up if self.active && !self.paused && alt => {
                self.subtract_at(*pos);
                if *kind == PointerKind::Up {
                    return self.commit(ctx);
                }
            }
            PointerKind::Move if self.active && !self.paused && self.subtracting => {
                // Alt released: painting goes on from here as a new segment of the stroke
                self.resume(*pos, ev.time_ms, ev.mods.shift);
            }
            PointerKind::Up if self.active && !self.paused && self.subtracting => {
                return self.commit(ctx);
            }
            // FB-31: a paused stroke resumes with the next press; moves and the release only move
            // the brush outline, nothing is painted (a press with Space held pans the view)
            PointerKind::Down if self.paused && !ev.mods.space => {
                self.resume(*pos, ev.time_ms, ev.mods.shift)
            }
            _ if self.paused => {}
            PointerKind::Down if self.id == ToolId::Eraser && !bitmap_target(ctx) => {
                // Radiergummi ohne Bitmap: das Original hebt nur die Auswahl auf (gemessen, R1 gewählt)
                self.cancel();
                if !ctx.selection.elements.is_empty() {
                    return ToolResponse {
                        commands: vec![Invocation::new(
                            "tool.pick",
                            json!({"command": "select.set", "params": {"ids": []}}),
                        )],
                        cursor: self.cursor(),
                        ..ToolResponse::default()
                    };
                }
            }
            PointerKind::Down => {
                self.begin(ctx, *pos, ev.time_ms);
                // FB-71: Shift held at the press draws the line from the press point
                self.line_anchor = ev.mods.shift.then_some(0);
                self.alt_latch = ev.mods.alt;
            }
            PointerKind::Move if self.active => self.extend(*pos, ev.mods.shift, ev.time_ms),
            PointerKind::Up if self.active && self.deselect_only => {
                self.cancel();
                if !ctx.selection.elements.is_empty() {
                    return ToolResponse {
                        commands: vec![Invocation::new(
                            "tool.pick",
                            json!({"command": "select.set", "params": {"ids": []}}),
                        )],
                        cursor: self.cursor(),
                        ..ToolResponse::default()
                    };
                }
            }
            PointerKind::Up if self.active => {
                self.extend(*pos, ev.mods.shift, ev.time_ms);
                return self.commit(ctx);
            }
            _ => {}
        }
        ToolResponse::with_overlay(self.overlay_for(ctx), self.cursor())
    }

    fn finish(&mut self, ctx: &ToolContext) -> ToolResponse {
        // FB-31: a paused stroke is applied first (tool change, menu command, undo)
        if self.paused {
            return self.commit(ctx);
        }
        // Werkzeugwechsel oder Menübefehl mitten im Zug: Zug verwerfen
        self.cancel();
        ToolResponse::default()
    }

    fn takes_history(&self) -> bool {
        self.active
    }

    fn history_key(&mut self, redo: bool, ctx: &ToolContext) -> ToolResponse {
        // FB-70: with the button held Undo and Redo are ignored, the stroke goes on (measured,
        // RE spec 71 section 5); while paused (Firepixel extension) Undo discards the stroke (no
        // history row) and Redo applies it like Enter
        if !self.active || !self.paused {
            return ToolResponse::with_overlay(self.overlay_for(ctx), self.cursor());
        }
        if redo {
            return self.commit(ctx);
        }
        self.cancel();
        ToolResponse::with_overlay(self.overlay_for(ctx), self.cursor())
    }

    fn stroke_subtracting(&self) -> bool {
        self.subtracting
    }

    fn stroke_pause(&self) -> StrokePause {
        if !self.active || self.deselect_only || self.escaped {
            StrokePause::Idle
        } else if self.paused {
            StrokePause::Paused
        } else {
            StrokePause::Painting
        }
    }

    fn overlay(&self) -> Vec<Overlay> {
        self.live
            .as_ref()
            .map(|l| vec![Overlay::Paint(l.preview())])
            .unwrap_or_default()
    }

    fn feedback(&self, ctx: &ToolContext) -> Vec<Overlay> {
        self.overlay_for(ctx)
    }
}

impl BrushTool {
    fn cancel(&mut self) {
        self.active = false;
        self.points.clear();
        self.raw.clear();
        self.times.clear();
        self.vels.clear();
        self.live = None;
        self.deselect_only = false;
        self.escaped = false;
        self.frozen = None;
        self.paused = false;
        self.breaks.clear();
        self.line_anchor = None;
        self.subtract.clear();
        self.subtracting = false;
        self.alt_latch = false;
    }

    /// Laufender Malvorgang (Tests, Vorschau).
    pub fn live(&self) -> Option<&crate::paint::LivePaint> {
        self.live.as_deref()
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
    /// Zeigerlage beim Griffzug (FP4-01: Vorschaulinie vom festen Griff zum Zeiger).
    handle_cur: Option<Point>,
}

impl FillTool {
    /// Neues Werkzeug (PaintBucket, Gradient).
    pub fn new(id: ToolId) -> FillTool {
        FillTool {
            id,
            start: None,
            cur: None,
            handle: None,
            handle_cur: None,
        }
    }

    fn cursor(&self) -> Cursor {
        if self.id == ToolId::Gradient {
            Cursor::Gradient
        } else {
            Cursor::Bucket
        }
    }

    /// Gibt es unter `p` etwas zu füllen (Bitmapmodus/Pixelauswahl oder ein Objekt)?
    fn has_target(&self, p: Point, ctx: &ToolContext) -> bool {
        if bitmap_target(ctx) || ctx.selection.has_pixels() {
            return true;
        }
        let opt = HitOptions {
            tolerance: ctx.pick_tolerance(),
            ..HitOptions::default()
        };
        super::pointer::top_hit(ctx, p, &opt).is_some()
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
    let t = if l2 > 0.0 {
        (((p.x - a.x) * dx + (p.y - a.y) * dy) / l2).clamp(0.0, 1.0)
    } else {
        0.0
    };
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
                self.handle_cur = None;
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
                        self.handle_cur = None;
                        return ToolResponse {
                            cursor: Cursor::Move,
                            ..ToolResponse::default()
                        };
                    }
                }
                self.start = Some(*pos);
                self.cur = None;
                ToolResponse {
                    cursor: self.cursor(),
                    ..ToolResponse::default()
                }
            }
            PointerKind::Move => {
                if let Some((_, h, w, s)) = self.handle {
                    self.handle_cur = Some(*pos);
                    let n = Self::dragged_handles(h, w, s, *pos);
                    return ToolResponse::with_overlay(
                        vec![Overlay::GradientHandles {
                            start: n.start,
                            end: n.end,
                        }],
                        Cursor::Move,
                    );
                }
                if let Some(s) = self.start
                    && self.id == ToolId::Gradient
                    && dist(s, *pos) >= ctx.drag_threshold()
                {
                    self.cur = Some(if ev.mods.shift {
                        constrain_angle(s, *pos, 45.0)
                    } else {
                        *pos
                    });
                }
                let hover_handle = gradient_target(ctx)
                    .is_some_and(|(_, h)| dist(*pos, h.start) <= tol || dist(*pos, h.end) <= tol);
                let cursor = if hover_handle && self.start.is_none() {
                    Cursor::Move
                } else if self.id == ToolId::PaintBucket
                    && self.start.is_none()
                    && !self.has_target(*pos, ctx)
                {
                    // Füllwerkzeug über leerer Fläche ohne Objekt: Verbotszeichen (PARITAET_A K8)
                    Cursor::NotAllowed
                } else {
                    self.cursor()
                };
                ToolResponse::with_overlay(self.overlay(), cursor)
            }
            PointerKind::Up => {
                if let Some((id, h, w, s)) = self.handle.take() {
                    self.handle_cur = None;
                    if dist(s, *pos) < ctx.drag_threshold() {
                        return ToolResponse::default();
                    }
                    let n = Self::dragged_handles(h, w, s, *pos);
                    return ToolResponse::commit(vec![Invocation::new(
                        "element.setFillHandles",
                        json!({"ids": [id.0], "start": pt_json(n.start), "end": pt_json(n.end), "second": pt_json(n.second)}),
                    )]);
                }
                let Some(s) = self.start.take() else {
                    return ToolResponse::default();
                };
                let to = self.cur.take();
                let dragged = to.is_some() || dist(s, *pos) >= ctx.drag_threshold();
                let to = to.unwrap_or(*pos);
                // eigene Farben des Verlauf-Werkzeugs (Werksvorgabe Schwarz → Weiß), nicht Füll- → Strichfarbe
                let colors = json!(
                    ctx.options
                        .gradient_tool_colors()
                        .into_iter()
                        .map(crate::commands::argb_hex)
                        .collect::<Vec<_>>()
                );
                if self.id == ToolId::Gradient && dragged {
                    if bitmap_target(ctx) || ctx.selection.has_pixels() {
                        let (tol, fill_sel) = (
                            ctx.options.gradient_tolerance,
                            ctx.options.gradient_fill_selection,
                        );
                        return ToolResponse::commit(vec![Invocation::new(
                            "bitmap.gradient",
                            json!({"from": pt_json(s), "to": pt_json(to), "colors": colors, "tolerance": tol, "fillSelection": fill_sel}),
                        )]);
                    }
                    if !ctx.selection.elements.is_empty() {
                        return ToolResponse::commit(vec![Invocation::new(
                            "element.setGradient",
                            json!({"from": pt_json(s), "to": pt_json(to), "colors": colors}),
                        )]);
                    }
                    return ToolResponse::default();
                }
                if bitmap_target(ctx) {
                    // Rand aus dem Füllbereich des Inspektors (Werksvorgabe Anti-Alias, weiche Kante 10)
                    let nf = ctx.options.new_fill.as_ref();
                    let edge = nf
                        .and_then(|v| v.get("edge"))
                        .and_then(Value::as_i64)
                        .unwrap_or(1);
                    let feather = nf
                        .and_then(|v| v.get("feather"))
                        .and_then(Value::as_i64)
                        .filter(|f| *f > 0)
                        .unwrap_or(10);
                    return ToolResponse::commit(vec![Invocation::new(
                        "bitmap.floodFill",
                        json!({"point": pt_json(*pos), "tolerance": ctx.options.tolerance_of(self.id), "fillSelection": ctx.options.fills_selection(self.id), "edge": edge, "feather": feather}),
                    )]);
                }
                let opt = HitOptions {
                    tolerance: ctx.pick_tolerance(),
                    ..HitOptions::default()
                };
                let hit = super::pointer::top_hit(ctx, *pos, &opt)
                    .and_then(|p| ctx.doc.element_at(&p).map(|e| e.id));
                match hit {
                    Some(id) if self.id == ToolId::Gradient => {
                        // Klick mit dem Verlaufswerkzeug: Verlauf quer über das Objekt
                        let b = docops::ids_bounds(ctx.doc, ctx.page, &[id]).unwrap_or_default();
                        let (a, c) = (
                            Point::new(b.left, (b.top + b.bottom) / 2.0),
                            Point::new(b.right, (b.top + b.bottom) / 2.0),
                        );
                        ToolResponse::commit(vec![Invocation::new(
                            "element.setGradient",
                            json!({"ids": [id.0], "from": pt_json(a), "to": pt_json(c), "colors": colors}),
                        )])
                    }
                    // Farbeimer auf ein Objekt: Schritt „Füllen“ wie im Original (PAR 07/16)
                    // samt Textur aus dem Füllbereich des Inspektors (CT-2: Textur „Ader“ 50 wirkt mit)
                    Some(id) => {
                        let mut p = json!({"ids": [id.0], "category": "solid", "color": crate::commands::argb_hex(ctx.fill_color)});
                        if let Some(nf) = ctx.options.new_fill.as_ref()
                            && nf
                                .get("texture")
                                .and_then(Value::as_str)
                                .is_some_and(|t| !t.is_empty())
                        {
                            for k in ["texture", "textureAmount", "textureTransparent"] {
                                if let Some(v) = nf.get(k) {
                                    p[k] = v.clone();
                                }
                            }
                        }
                        ToolResponse::commit(vec![Invocation::new("pi.fill", p)])
                    }
                    None => ToolResponse::default(),
                }
            }
            PointerKind::DoubleClick => ToolResponse::default(),
        }
    }

    fn overlay(&self) -> Vec<Overlay> {
        match (self.start, self.cur) {
            (Some(a), Some(b)) => vec![
                Overlay::Line { from: a, to: b },
                Overlay::GradientHandles { start: a, end: b },
            ],
            _ => Vec::new(),
        }
    }

    fn feedback(&self, ctx: &ToolContext) -> Vec<Overlay> {
        let mut v = self.overlay();
        // re 37 §3: beim Ziehen eines Griffs bleiben die alten Griffe bis zum Loslassen stehen
        // FP4-01 (K-6, T-9): dazu die Vorschaulinie vom festen Griff zum Zeiger
        if let Some((_, h, w, s)) = self.handle {
            v.insert(
                0,
                Overlay::GradientHandles {
                    start: h.start,
                    end: h.end,
                },
            );
            if let Some(cur) = self.handle_cur.filter(|c| dist(*c, s) > 0.0) {
                let n = Self::dragged_handles(h, w, s, cur);
                v.push(Overlay::GradientHandles {
                    start: n.start,
                    end: n.end,
                });
            }
        }
        if self.handle.is_none()
            && self.cur.is_none()
            && let Some((_, h)) = gradient_target(ctx)
        {
            v.push(Overlay::GradientHandles {
                start: h.start,
                end: h.end,
            });
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
            ToolInput::Pointer {
                kind: PointerKind::Up,
                pos,
            } if ctx.has_document => {
                // neben der Leinwand: Farbe bleibt (PAR 07/13)
                let canvas = crate::commands::canvas_rect(ctx.doc, ctx.page);
                if pos.x < canvas.left
                    || pos.y < canvas.top
                    || pos.x >= canvas.right
                    || pos.y >= canvas.bottom
                {
                    return ToolResponse {
                        cursor: Cursor::Eyedropper,
                        ..ToolResponse::default()
                    };
                }
                // Ziel = aktives Farbfeld des Werkzeugbedienfelds (Alt: das andere); gewählte Vektoren
                // bekommen die Farbe (PAR 07/11)
                let stroke = ctx.options.stroke_well != ev.mods.alt;
                ToolResponse {
                    commands: vec![Invocation::new(
                        "color.sample",
                        json!({"point": pt_json(*pos), "target": if stroke { "stroke" } else { "fill" }, "samples": ctx.options.eyedropper_samples, "applyToSelection": true}),
                    )],
                    cursor: Cursor::Eyedropper,
                    ..ToolResponse::default()
                }
            }
            _ => ToolResponse {
                cursor: Cursor::Eyedropper,
                ..ToolResponse::default()
            },
        }
    }
}
