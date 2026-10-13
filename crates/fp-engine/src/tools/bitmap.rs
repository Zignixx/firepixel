//! Bitmapwerkzeuge: Auswahlrahmen, Lasso, Zauberstab, Pinsel/Bleistift/Radiergummi, Füllwerkzeug,
//! Verlauf (mit Verlaufsgriffen auf Vektoren) und Pipette (10 §2.6).

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
pub(crate) fn bitmap_target(ctx: &ToolContext) -> bool {
    ctx.bitmap_mode
        || ctx.selection.elements.iter().any(|id| docops::element_on_page(ctx.doc, ctx.page, *id).is_some_and(|e| matches!(e.kind, ElementKind::Bitmap(_))))
}

/// Esc in einem Bitmap-Auswahlwerkzeug ohne laufenden Vorgang: wie im Original „Bitmapmodus
/// beenden“ (Verlaufsschritt; Auswahlrahmen weg, Bitmap bleibt ausgewählt, Werkzeug → Zeiger;
/// UX-A L13, `a_321`). Ohne Pixelauswahl: nichts (auch im Bitmapmodus, L27).
pub(crate) fn escape_bitmap_mode(ctx: &ToolContext) -> ToolResponse {
    // Nur mit Auswahlrahmen (z. B. nach dem Zauberstab, L13): Rahmen weg, Bitmapmodus beenden,
    // Zeiger. Ohne Rahmen (Bitmapmodus per Doppelklick, L27) bleibt im Original alles, wie es ist:
    // Werkzeug Auswahlrahmen, Bitmap gewählt (PM-1, gemessen).
    if !ctx.selection.has_pixels() {
        return ToolResponse::default();
    }
    ToolResponse { commands: vec![Invocation::new("modify.exitBitmapMode", json!({}))], switch_tool: Some(ToolId::Pointer), ..ToolResponse::default() }
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
    let commands = if ctx.bitmap_mode || ctx.selection.has_pixels() { vec![Invocation::new("modify.exitBitmapMode", json!({}))] } else { Vec::new() };
    ToolResponse { commands, switch_tool: Some(ToolId::Pointer), ..ToolResponse::default() }
}

/// Esc nach einem Malzug (Pinsel 01/14, Buntstift/Radiergummi 03/16, Retusche): Zeiger und Schritt
/// „Bitmapmodus beenden“, die Bitmap bleibt ausgewählt (gemessen am Original).
pub(crate) fn escape_after_stroke(ctx: &ToolContext) -> Option<ToolResponse> {
    (bitmap_target(ctx) && !ctx.bitmap_mode && !ctx.selection.has_pixels()).then(|| ToolResponse {
        commands: vec![Invocation::new("modify.editBitmap", json!({})), Invocation::new("modify.exitBitmapMode", json!({"step": true}))],
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
                if self.start.is_none() {
                    return escape_bitmap_mode(ctx);
                }
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
                if !self.active {
                    return escape_bitmap_mode(ctx);
                }
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
            ToolInput::Key(ToolKey::Escape) if ctx.has_document => escape_bitmap_mode(ctx),
            _ => ToolResponse { cursor: Cursor::Wand, ..ToolResponse::default() },
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
    live: Option<Box<crate::paint::LivePaint>>,
    /// Versatz der Punkte gegenüber der Maus (Pinsel mit gerader Größe: +1/+1 wie im Original).
    offset: f32,
    /// Pinsel: Zug beginnt außerhalb der gewählten Bitmap mit „Transparenz beibehalten“ – nichts
    /// malen, beim Loslassen die Auswahl aufheben.
    deselect_only: bool,
    /// Pinsel: der letzte Zug wurde gemalt (Esc danach beendet den Bitmapmodus mit Verlaufsschritt).
    painted: bool,
}

/// Abstand (px) des Zugbeginns von der gewählten Bitmap, ab dem der Pinsel ein neues Bitmap malt
/// (Annahme zwischen den Messungen 19 px → Bitmap wächst und 100 px → neues Bitmap).
pub const OUTSIDE_NEW_BITMAP: f32 = 32.0;

/// Strichtextur des Pinsels, solange im Inspektor keine gewählt ist (Liste des Originals zeigt „Ader“).
pub const DEFAULT_STROKE_TEXTURE: &str = "Vein";

/// Startwert eines neuen Pinselzugs (je Zug verschieden wie im Original).
fn stroke_seed() -> u32 {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos() ^ d.as_secs() as u32).unwrap_or(0);
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
    let unit = |v: f32| if v > 0.5 { 1.0 } else if v < -0.5 { -1.0 } else { 0.0 };
    Point::new(a.x + unit(c) * m, a.y + unit(s) * m)
}

impl BrushTool {
    /// Neues Werkzeug (Brush, Pencil, Eraser).
    pub fn new(id: ToolId) -> BrushTool {
        BrushTool { id, active: false, hover: None, params: Value::Null, points: Vec::new(), live: None, offset: 0.0, deselect_only: false, painted: false }
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
            .find_map(|p| ctx.doc.element_at(&p).filter(|e| !matches!(e.kind, ElementKind::Bitmap(_))).cloned())?;
        let a = docops::path_attrs_of(&e)?;
        Some(a.brush.clone().unwrap_or_else(|| crate::commands::panel::tool_brush_from(Some(&json!({"preset": "Bleistift/1-Pixel hart", "width": 1})), None)))
    }

    fn paint_tool(&self) -> crate::paint::PaintTool {
        match self.id {
            ToolId::Eraser => crate::paint::PaintTool::Eraser,
            ToolId::Pencil => crate::paint::PaintTool::Pencil,
            _ => crate::paint::PaintTool::Brush,
        }
    }

    fn command(&self) -> &'static str {
        if self.id == ToolId::Eraser { "bitmap.erase" } else { "bitmap.paint" }
    }

    /// Auftrag aus den Werkzeugwerten (Inspektor) beim Drücken; `start_px` = Pixel der Zielbitmap
    /// unter dem Startpunkt (Bleistift „Automatisch löschen“).
    fn job_params(&self, ctx: &ToolContext, start_px: Option<[u8; 4]>) -> Value {
        let o = ctx.options;
        let hex = |c: fp_color::Argb| c.to_rgba8().to_hex_rgba();
        match self.id {
            ToolId::Eraser => json!({"size": o.eraser_size, "round": o.eraser_round, "opacity": o.eraser_opacity, "edge": o.eraser_edge}),
            ToolId::Pencil => {
                let mut b = fp_model::Brush::basic(1);
                b.antialiased = o.pencil_antialias;
                let s = ctx.stroke_color;
                let same = start_px.is_some_and(|p| p[3] > 0 && p[0] == s.r() && p[1] == s.g() && p[2] == s.b());
                let color = if o.auto_erase && same { ctx.fill_color } else { s };
                json!({"tool": "pencil", "brush": b, "color": hex(color), "opacity": o.opacity, "blend": o.paint_blend, "preserveTransparency": o.keep_transparency, "trimEnd": false})
            }
            _ => {
                let st = o.new_stroke.as_ref();
                let mut b = match Self::vector_brush(ctx) {
                    Some(v) => v,
                    None => {
                        let mut b = crate::commands::panel::tool_brush(o);
                        if let Some(a) = st.and_then(|s| s.get("textureAmount")).and_then(|v| v.as_i64()) {
                            b.texture_blend = (a.clamp(0, 100) * 10) as i32;
                        }
                        b
                    }
                };
                b.diameter = b.diameter.max(1);
                // echte Züge malen bis einschließlich Endpunkt (r03: 60→500 = 441 px), eigener
                // Zufalls-Startwert je Zug (natürliche Pinsel)
                let mut p = json!({"brush": b, "color": hex(ctx.stroke_color), "opacity": o.opacity, "blend": o.paint_blend, "preserveTransparency": o.keep_transparency, "trimEnd": false, "seed": stroke_seed()});
                // Textur der Strichvorgabe; ohne Wahl wie im Original „Ader“ (Vein, Vorgabe der Liste),
                // damit texturierte Arten (Graphit, Kohlestift …) ihre Textur zeigen
                let tex = st.and_then(|s| s.get("texture")).and_then(|v| v.as_str()).filter(|t| !t.is_empty()).unwrap_or(DEFAULT_STROKE_TEXTURE);
                p["texture"] = json!(tex);
                p
            }
        }
    }

    /// Beginnt den Zug: Auftrag festlegen, Zielbitmap lesen, ersten Abdruck rechnen.
    fn begin(&mut self, ctx: &ToolContext, pos: Point) {
        self.active = true;
        self.deselect_only = false;
        self.painted = false;
        self.offset = 0.0;
        let mut target = crate::paint::target_bitmap(ctx.doc, ctx.page, &ctx.selection.elements);
        let mut base = target.as_ref().and_then(|(_, e)| match &e.kind {
            ElementKind::Bitmap(b) => docops::bitmap_image(b),
            _ => None,
        });
        let start_px = base.as_ref().and_then(|b| b.get(pos.x.floor() as i32, pos.y.floor() as i32));
        self.params = self.job_params(ctx, start_px);
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
        let mut p = self.params.clone();
        p["points"] = json!([[pos.x, pos.y]]);
        let size = if self.id == ToolId::Eraser { ctx.options.eraser_size } else { ctx.options.brush_size };
        let job = crate::paint::PaintJob::from_params(self.paint_tool(), &p, ctx.stroke_color, size);
        let sel = ctx.selection.pixels.clone().filter(|_| ctx.selection.has_pixels());
        let backdrop = ctx.doc.pages.get(ctx.page).map(|pg| pg.root.background).unwrap_or(fp_color::Argb(0xffff_ffff));
        self.live = Some(Box::new(crate::paint::LivePaint::new(job, base, sel, target.map(|t| t.0), backdrop)));
    }

    /// Punkt anhängen bzw. (Umschalt) gerade Linie vom Startpunkt.
    fn extend(&mut self, pos: Point, line: bool) {
        if self.deselect_only {
            return;
        }
        let pos = Point::new(pos.x + self.offset, pos.y + self.offset);
        if line {
            let first = self.points[0];
            let pos = if self.id == ToolId::Brush { snap_brush_line(first, pos) } else { pos };
            self.points = vec![first, pos];
            if let Some(l) = self.live.as_mut() {
                l.set_points(self.points.clone());
            }
        } else if self.points.last().is_none_or(|l| *l != pos) {
            self.points.push(pos);
            if let Some(l) = self.live.as_mut() {
                l.push(pos);
            }
        }
    }

    fn size(&self, ctx: &ToolContext) -> f32 {
        match self.id {
            ToolId::Pencil => 1.0,
            ToolId::Eraser => ctx.options.eraser_size,
            _ => crate::commands::panel::tool_brush(ctx.options).diameter.max(1) as f32,
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
            if let Some(l) = &self.live {
                v.push(Overlay::Paint(l.preview()));
            }
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
            if self.active && self.id == ToolId::Brush {
                // Pinsel: Esc mitten im Zug bricht nicht ab (gemessen r14_esc_mid = ganzer Strich)
                return ToolResponse::with_overlay(self.overlay_for(ctx), self.cursor());
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
        let ToolInput::Pointer { kind, pos } = &ev.input else { return ToolResponse::default() };
        self.hover = Some(*pos);
        match kind {
            PointerKind::Down if self.id == ToolId::Eraser && !bitmap_target(ctx) => {
                // Radiergummi ohne Bitmap: das Original hebt nur die Auswahl auf (gemessen, R1 gewählt)
                self.cancel();
                if !ctx.selection.elements.is_empty() {
                    return ToolResponse {
                        commands: vec![Invocation::new("tool.pick", json!({"command": "select.set", "params": {"ids": []}}))],
                        cursor: self.cursor(),
                        ..ToolResponse::default()
                    };
                }
            }
            PointerKind::Down => self.begin(ctx, *pos),
            PointerKind::Move if self.active => self.extend(*pos, ev.mods.shift),
            PointerKind::Up if self.active && self.deselect_only => {
                self.cancel();
                if !ctx.selection.elements.is_empty() {
                    return ToolResponse {
                        commands: vec![Invocation::new("tool.pick", json!({"command": "select.set", "params": {"ids": []}}))],
                        cursor: self.cursor(),
                        ..ToolResponse::default()
                    };
                }
            }
            PointerKind::Up if self.active => {
                self.extend(*pos, ev.mods.shift);
                let mut params = std::mem::take(&mut self.params);
                params["points"] = Value::Array(self.points.iter().map(|p| pt_json(*p)).collect());
                self.cancel();
                // Pinsel (01/14), Buntstift und Radiergummi (03/16): Esc danach → „Bitmapmodus beenden“
                self.painted = true;
                let cmd = Invocation::new(self.command(), params);
                return ToolResponse { commands: vec![cmd], cursor: self.cursor(), overlay: self.overlay_for(ctx), ..ToolResponse::default() };
            }
            _ => {}
        }
        ToolResponse::with_overlay(self.overlay_for(ctx), self.cursor())
    }

    fn finish(&mut self, _ctx: &ToolContext) -> ToolResponse {
        // Werkzeugwechsel oder Menübefehl mitten im Zug: Zug verwerfen
        self.cancel();
        ToolResponse::default()
    }

    fn overlay(&self) -> Vec<Overlay> {
        self.live.as_ref().map(|l| vec![Overlay::Paint(l.preview())]).unwrap_or_default()
    }

    fn feedback(&self, ctx: &ToolContext) -> Vec<Overlay> {
        self.overlay_for(ctx)
    }
}

impl BrushTool {
    fn cancel(&mut self) {
        self.active = false;
        self.points.clear();
        self.live = None;
        self.deselect_only = false;
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
}

impl FillTool {
    /// Neues Werkzeug (PaintBucket, Gradient).
    pub fn new(id: ToolId) -> FillTool {
        FillTool { id, start: None, cur: None, handle: None }
    }

    fn cursor(&self) -> Cursor {
        if self.id == ToolId::Gradient { Cursor::Gradient } else { Cursor::Bucket }
    }

    /// Gibt es unter `p` etwas zu füllen (Bitmapmodus/Pixelauswahl oder ein Objekt)?
    fn has_target(&self, p: Point, ctx: &ToolContext) -> bool {
        if bitmap_target(ctx) || ctx.selection.has_pixels() {
            return true;
        }
        let opt = HitOptions { tolerance: ctx.pick_tolerance(), ..HitOptions::default() };
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
                let cursor = if hover_handle && self.start.is_none() {
                    Cursor::Move
                } else if self.id == ToolId::PaintBucket && self.start.is_none() && !self.has_target(*pos, ctx) {
                    // Füllwerkzeug über leerer Fläche ohne Objekt: Verbotszeichen (PARITAET_A K8)
                    Cursor::NotAllowed
                } else {
                    self.cursor()
                };
                ToolResponse::with_overlay(self.overlay(), cursor)
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
                // eigene Farben des Verlauf-Werkzeugs (Werksvorgabe Schwarz → Weiß), nicht Füll- → Strichfarbe
                let colors = json!(ctx.options.gradient_tool_colors().into_iter().map(crate::commands::argb_hex).collect::<Vec<_>>());
                if self.id == ToolId::Gradient && dragged {
                    if bitmap_target(ctx) || ctx.selection.has_pixels() {
                        let (tol, fill_sel) = (ctx.options.gradient_tolerance, ctx.options.gradient_fill_selection);
                        return ToolResponse::commit(vec![Invocation::new("bitmap.gradient", json!({"from": pt_json(s), "to": pt_json(to), "colors": colors, "tolerance": tol, "fillSelection": fill_sel}))]);
                    }
                    if !ctx.selection.elements.is_empty() {
                        return ToolResponse::commit(vec![Invocation::new("element.setGradient", json!({"from": pt_json(s), "to": pt_json(to), "colors": colors}))]);
                    }
                    return ToolResponse::default();
                }
                if bitmap_target(ctx) {
                    // Rand aus dem Füllbereich des Inspektors (Werksvorgabe Anti-Alias, weiche Kante 10)
                    let nf = ctx.options.new_fill.as_ref();
                    let edge = nf.and_then(|v| v.get("edge")).and_then(Value::as_i64).unwrap_or(1);
                    let feather = nf.and_then(|v| v.get("feather")).and_then(Value::as_i64).filter(|f| *f > 0).unwrap_or(10);
                    return ToolResponse::commit(vec![Invocation::new(
                        "bitmap.floodFill",
                        json!({"point": pt_json(*pos), "tolerance": ctx.options.tolerance_of(self.id), "fillSelection": ctx.options.fills_selection(self.id), "edge": edge, "feather": feather}),
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
