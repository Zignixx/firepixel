//! Vektorwerkzeuge: Rechteck/Ellipse/Polygon, Linie, Stift, Vektorpfad/Pfad nachzeichnen,
//! Hotspots und Segmente (10 §2.6). Alle Punkte laufen vor dem Setzen durch den Fang
//! ([`super::snap::Snapper`]); die Rückmeldung zeigt Form, Größe und Fanglinien.

use super::pointer::contours_json;
use super::snap::Snapper;
use super::{
    Cursor, Invocation, Overlay, PointerKind, ToolContext, ToolEvent, ToolId, ToolInput, ToolKey,
    ToolMachine, ToolResponse, constrain_angle, constrain_rect, dist, fmt_num, pt_json, rect_json,
    size_label,
};
use crate::docops;
use crate::geometry;
use fp_geom::{Point, Rect};
use fp_model::{Contour, ElementId, ElementKind};
use serde_json::{Value, json};

#[derive(Clone, Debug, PartialEq)]
enum Drag {
    Idle,
    Pressed {
        start: Point,
        last: Point,
    },
    Dragging {
        start: Point,
        cur: Point,
        last: Point,
    },
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
    fn update(
        &mut self,
        kind: PointerKind,
        p: Point,
        ev: &ToolEvent,
        thr: f32,
    ) -> Option<(Point, Point)> {
        match (kind, self.state.clone()) {
            (PointerKind::Down, _) => {
                self.state = Drag::Pressed { start: p, last: p };
                None
            }
            (PointerKind::Move | PointerKind::Up, Drag::Pressed { start, .. }) => {
                if dist(start, p) >= thr {
                    self.state = Drag::Dragging {
                        start,
                        cur: p,
                        last: p,
                    };
                    Some((start, p))
                } else {
                    None
                }
            }
            (
                PointerKind::Move | PointerKind::Up,
                Drag::Dragging {
                    mut start, last, ..
                },
            ) => {
                if ev.mods.space {
                    // Leertaste: Form verschieben (Startpunkt wandert mit, B `@00d5bbc0`)
                    start = Point::new(start.x + p.x - last.x, start.y + p.y - last.y);
                }
                self.state = Drag::Dragging {
                    start,
                    cur: p,
                    last: p,
                };
                Some((start, p))
            }
            _ => None,
        }
    }

    fn reset(&mut self) {
        self.state = Drag::Idle;
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
    overlay: Vec<Overlay>,
    guides: Vec<Overlay>,
    /// Automatische Form: gezogener Steuerpunkt.
    control: Option<super::pointer::ControlDrag>,
    /// Rohe Drückposition (ihr Fang gilt für den Startpunkt, auch wenn die Leertaste ihn verschiebt).
    press: Option<Point>,
    /// Rundung, die ↑/↓ während des Aufziehens hinzugefügt haben (PAR 10/05).
    round_add: f32,
    /// Letzte ↑/↓-Taste mit Zeit (gehaltene Taste: Wiederholungen addieren nach der Haltedauer).
    last_key: Option<(ToolKey, u64)>,
    /// Letzter Zug (Start, aktueller Punkt, Umschalttasten) für die Vorschau nach einer Taste.
    last: Option<(Point, Point, super::Modifiers)>,
    /// Aufgezogener Rahmen (Inspektor live, PAR 10/15).
    live: Option<Rect>,
}

/// Rundung je Sekunde gehaltener ↑-Taste (Original: 1,5 s → 0,97, PAR 10/05).
const ROUND_RATE: f32 = 0.65;
/// Rundung je kurzem Tastendruck (Original: drei Tipper → 0,01).
const ROUND_TAP: f32 = 0.004;

impl ShapeTool {
    /// Neues Werkzeug (Rectangle, Ellipse, Polygon, AutoShape).
    pub fn new(id: ToolId) -> ShapeTool {
        ShapeTool {
            id,
            drag: DragTracker::new(),
            overlay: Vec::new(),
            guides: Vec::new(),
            control: None,
            press: None,
            round_add: 0.0,
            last_key: None,
            last: None,
            live: None,
        }
    }

    fn roundness(&self, _r: &Rect, ctx: &ToolContext) -> f32 {
        (ctx.options.rect_roundness + self.round_add).clamp(0.0, 1.0)
    }

    /// Objektname neuer Formen wie im Original: Ellipse „Ellipse“, Polygon/Stern „Polygon“;
    /// Rechteck-Grundformen bleiben ohne Namen (PAR 10/08).
    fn object_name(&self) -> Option<String> {
        match self.id {
            ToolId::Ellipse => Some(fp_i18n::t("tool-ellipse")),
            ToolId::Polygon => Some(fp_i18n::t("tool-polygon")),
            _ => None,
        }
    }

    /// Befehl beim Loslassen: Strich 1 px aus der Strichfarbe wie im Original (PAR 10/12) und der
    /// Objektname (PAR 10/08).
    fn finalize(&self, mut inv: Invocation) -> Invocation {
        if matches!(
            inv.command.as_str(),
            "draw.rectangle" | "draw.ellipse" | "draw.polygon"
        ) && let Some(o) = inv.params.as_object_mut()
        {
            o.entry("strokeWidth").or_insert(json!(1));
            if let Some(n) = self.object_name() {
                o.insert("name".into(), json!(n));
            }
        }
        inv
    }

    fn preview(
        &self,
        start: Point,
        cur: Point,
        ev: &ToolEvent,
        ctx: &ToolContext,
    ) -> (Vec<Overlay>, Option<Invocation>) {
        match self.id {
            ToolId::Polygon => {
                let corner = if ev.mods.shift {
                    constrain_angle(start, cur, 15.0)
                } else {
                    cur
                };
                let o = ctx.options;
                let c = geometry::polygon_tool_contour(
                    start,
                    corner,
                    o.polygon_sides as usize,
                    o.polygon_star,
                    o.polygon_spikiness.map(|v| v as f64),
                );
                let mut ov = c
                    .map(|c| vec![Overlay::Path { contours: vec![c] }])
                    .unwrap_or_default();
                let r = dist(start, corner);
                let a = -((corner.y - start.y) as f64)
                    .atan2((corner.x - start.x) as f64)
                    .to_degrees();
                ov.push(Overlay::Label {
                    at: corner,
                    text: format!("r {} · {}°", fmt_num(r), fmt_num(a as f32)),
                });
                let inv = Invocation::new(
                    "draw.polygon",
                    json!({"center": pt_json(start), "corner": pt_json(corner), "sides": o.polygon_sides, "star": o.polygon_star,
                           "spikiness": o.polygon_spikiness}),
                );
                (ov, Some(inv))
            }
            ToolId::AutoShape => {
                // Automatische Form: Rahmen aufziehen, die Form selbst erzeugt das Skript der Form.
                // Umschalt macht den Zugrahmen zum Quadrat (längere Seite, in Zugrichtung; TW-88,
                // Lauf 9); gesperrte Formen (`lockAspect` im Skript) sind ohnehin quadratisch.
                // Die Pfeillinie quadriert nicht, sie rastet in 45°-Schritten ein (M10, re 43 §2).
                let r = autoshape_rect(
                    &ctx.options.auto_shape,
                    start,
                    cur,
                    ev.mods.shift,
                    ev.mods.alt,
                );
                (
                    vec![
                        Overlay::Rect {
                            rect: r,
                            dashed: false,
                        },
                        size_label(r),
                    ],
                    Some(Invocation::new(
                        "autoShape.insert",
                        json!({"shape": ctx.options.auto_shape, "rect": rect_json(r), "from": pt_json(start), "shift": ev.mods.shift}),
                    )),
                )
            }
            ToolId::Ellipse => {
                let r = constrain_rect(start, cur, ev.mods.shift, ev.mods.alt);
                (
                    vec![
                        Overlay::Path {
                            contours: vec![geometry::ellipse_contour(r)],
                        },
                        size_label(r),
                    ],
                    Some(Invocation::new(
                        "draw.ellipse",
                        json!({"rect": rect_json(r)}),
                    )),
                )
            }
            _ => {
                let r = constrain_rect(start, cur, ev.mods.shift, ev.mods.alt);
                let rd = self.roundness(&r, ctx);
                (
                    vec![
                        Overlay::Path {
                            contours: vec![geometry::rect_contour(r, rd as f64)],
                        },
                        size_label(r),
                    ],
                    Some(Invocation::new(
                        "draw.rectangle",
                        json!({"rect": rect_json(r), "roundness": rd}),
                    )),
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
                    use super::pointer::{ControlDrag, control_point_at, control_points_feedback};
                    match (kind, self.control.take()) {
                        (PointerKind::Down, _) => {
                            if let Some((id, index, at)) =
                                control_point_at(ctx, *pos, ctx.pick_tolerance() as f32)
                            {
                                self.control = Some(ControlDrag {
                                    id,
                                    index,
                                    from: at,
                                    points: vec![at],
                                });
                                return ToolResponse::with_overlay(
                                    control_points_feedback(ctx, Some((id, index, at))),
                                    Cursor::PointerPoint,
                                );
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
                    if *kind == PointerKind::Move
                        && !self.drag.active()
                        && let Some(o) = super::pointer::control_points_hover(
                            ctx,
                            *pos,
                            ctx.pick_tolerance() as f32,
                        )
                    {
                        return ToolResponse::with_overlay(o, Cursor::PointerPoint);
                    }
                }
                // Fang wie im Original (PARITAET_A L16, gemessen): Start- und Endpunkt rasten je
                // Ereignis getrennt ein (bzw. mit Alt der Mittelpunkt); mit Umschalt rastet keiner
                // an intelligenten Hilfslinien. Der Automat führt daher die rohen Zeigerpunkte.
                let snapper = Snapper::new(ctx, &[]);
                // Polygon: Mittelpunkt und Ecke folgen genau der Maus (PAR 10/10: 600,150→650,200
                // ergibt die Ecke 650,200, kein Einrasten an Punkten anderer Formen)
                let snapper = if ev.mods.shift || self.id == ToolId::Polygon {
                    snapper.without_smart()
                } else {
                    snapper
                };
                let sp = snapper.point(*pos);
                if !self.drag.active() && *kind != PointerKind::Down {
                    // Überfahren: nur Fanglinien zeigen
                    self.guides = sp.guides;
                    return ToolResponse::with_overlay(self.guides.clone(), Cursor::Crosshair);
                }
                let pressed_at = match self.drag.state {
                    Drag::Pressed { start, .. } => Some(snapper.point(start).point),
                    _ => None,
                };
                if *kind == PointerKind::Down {
                    self.press = Some(*pos);
                    self.round_add = 0.0;
                    self.last_key = None;
                    self.last = None;
                    self.live = None;
                }
                let press = self.press.unwrap_or(*pos);
                let fix = snapper.point(press).point;
                let (fx, fy) = (fix.x - press.x, fix.y - press.y);
                let d = self
                    .drag
                    .update(*kind, *pos, ev, ctx.drag_threshold())
                    .map(|(s, _)| (Point::new(s.x + fx, s.y + fy), sp.point));
                if *kind == PointerKind::Up
                    && d.is_none()
                    && self.id == ToolId::AutoShape
                    && let Some(at) = pressed_at
                {
                    // Klick ohne Ziehen: Form in Vorgabegröße an dieser Stelle
                    self.drag.reset();
                    return ToolResponse::commit(vec![Invocation::new(
                        "autoShape.insert",
                        json!({"shape": ctx.options.auto_shape, "at": pt_json(at)}),
                    )]);
                }
                if *kind == PointerKind::Up {
                    self.drag.reset();
                    self.overlay.clear();
                    self.guides.clear();
                    self.live = None;
                    self.last = None;
                    let resp = match d {
                        Some((s, c)) => ToolResponse::commit(
                            self.preview(s, c, ev, ctx)
                                .1
                                .into_iter()
                                .map(|i| super::styled(self.finalize(i), ctx.options))
                                .collect(),
                        ),
                        None => ToolResponse::default(),
                    };
                    self.round_add = 0.0;
                    return resp;
                }
                self.guides = sp.guides;
                if let Some((s, c)) = d {
                    self.overlay = self.preview(s, c, ev, ctx).0;
                    self.last = Some((s, c, ev.mods));
                    self.live = match self.id {
                        ToolId::Polygon => None,
                        ToolId::AutoShape => Some(
                            autoshape_rect(
                                &ctx.options.auto_shape,
                                s,
                                c,
                                ev.mods.shift,
                                ev.mods.alt,
                            )
                            .normalized(),
                        ),
                        _ => Some(constrain_rect(s, c, ev.mods.shift, ev.mods.alt).normalized()),
                    };
                }
                ToolResponse::with_overlay(self.overlay(), Cursor::Crosshair)
            }
            // ↑/↓ während des Aufziehens ändern die Eckrundung des Rechtecks, solange gehalten
            // (PAR 10/05: 1,5 s → 0,97; kurzer Tipper ≈ 0–1 %); ← → ohne Wirkung (kein Verschieben)
            ToolInput::Key(k @ (ToolKey::Up | ToolKey::Down | ToolKey::Left | ToolKey::Right))
                if self.drag.active() =>
            {
                if self.id == ToolId::Rectangle && matches!(k, ToolKey::Up | ToolKey::Down) {
                    let t = ev.time_ms;
                    let step = match self.last_key {
                        Some((lk, lt)) if lk == *k && t >= lt && t - lt <= 250 => {
                            ROUND_RATE * (t - lt) as f32 / 1000.0
                        }
                        _ => ROUND_TAP,
                    };
                    self.last_key = Some((*k, t));
                    let base = ctx.options.rect_roundness.clamp(0.0, 1.0);
                    let sign = if *k == ToolKey::Up { 1.0 } else { -1.0 };
                    self.round_add = (base + self.round_add + sign * step).clamp(0.0, 1.0) - base;
                    if let Some((s, c, m)) = self.last {
                        let mut e2 = ev.clone();
                        e2.mods = m;
                        self.overlay = self.preview(s, c, &e2, ctx).0;
                    }
                }
                let mut o = self.overlay();
                if o.is_empty() {
                    // Taste verbraucht (sonst verschöbe das Menükürzel die Auswahl)
                    o.push(Overlay::Label {
                        at: self.press.unwrap_or_default(),
                        text: String::new(),
                    });
                }
                ToolResponse::with_overlay(o, Cursor::Crosshair)
            }
            // Esc während des Aufziehens wird im Original ignoriert, die Form
            // entsteht beim Loslassen (PARITAET_A L19, gemessen am Rechteck).
            ToolInput::Key(ToolKey::Escape)
                if self.id == ToolId::Rectangle && self.drag.active() =>
            {
                ToolResponse::with_overlay(self.overlay(), Cursor::Crosshair)
            }
            ToolInput::Key(ToolKey::Escape) => {
                self.drag.reset();
                self.control = None;
                self.overlay.clear();
                self.guides.clear();
                self.live = None;
                self.last = None;
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

    fn live_rect(&self) -> Option<Rect> {
        self.live
    }

    fn feedback(&self, ctx: &ToolContext) -> Vec<Overlay> {
        let mut v = self.overlay();
        if self.id == ToolId::AutoShape {
            let drag = self
                .control
                .as_ref()
                .map(|c| (c.id, c.index, *c.points.last().unwrap_or(&c.from)));
            v.extend(super::pointer::control_points_feedback(ctx, drag));
        }
        v
    }
}

/// Mitte des Pixels unter `p` (Linie, Vektorpfad: Knoten auf ,5 wie im Original).
pub(crate) fn pixel_center(p: Point) -> Point {
    Point::new(p.x.floor() + 0.5, p.y.floor() + 0.5)
}

/// Umschalt-Linie wie im Original (PAR 09/01): waagrecht/senkrecht, schräg genau 45° mit der
/// **größeren** Komponente als Kantenlänge (100,300→200,390 ergibt 200,400).
pub(crate) fn constrain_line(s: Point, c: Point) -> Point {
    let (dx, dy) = (c.x - s.x, c.y - s.y);
    let a = (dy as f64).atan2(dx as f64).to_degrees();
    let k = ((a / 45.0).round() as i64).rem_euclid(8);
    match k {
        0 | 4 => Point::new(c.x, s.y),
        2 | 6 => Point::new(s.x, c.y),
        _ => {
            let m = dx.abs().max(dy.abs());
            let (sx, sy) = if k == 1 {
                (1.0, 1.0)
            } else if k == 3 {
                (-1.0, 1.0)
            } else if k == 5 {
                (-1.0, -1.0)
            } else {
                (1.0, -1.0)
            };
            Point::new(s.x + sx * m, s.y + sy * m)
        }
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
        LineTool {
            drag: DragTracker::new(),
            overlay: Vec::new(),
        }
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
        let ToolInput::Pointer { kind, pos } = &ev.input else {
            return ToolResponse::default();
        };
        let sp = Snapper::new(ctx, &[]).point(*pos);
        if !self.drag.active() && *kind != PointerKind::Down {
            self.overlay = sp.guides;
            return ToolResponse::with_overlay(self.overlay.clone(), Cursor::Crosshair);
        }
        let d = self.drag.update(*kind, sp.point, ev, ctx.drag_threshold());
        let line = d.map(|(s, c)| {
            (
                s,
                if ev.mods.shift {
                    constrain_line(s, c)
                } else {
                    c
                },
            )
        });
        if *kind == PointerKind::Up {
            self.drag.reset();
            self.overlay.clear();
            return match line {
                Some((a, b)) => {
                    // wie im Original: Knoten auf Pixelmitten (100.5,100.5), Name „Linie“ (PAR 09/01)
                    let (a, b) = (pixel_center(a), pixel_center(b));
                    ToolResponse::commit(vec![super::styled(
                        Invocation::new(
                            "draw.line",
                            json!({"from": pt_json(a), "to": pt_json(b), "name": fp_i18n::t("tool-line")}),
                        ),
                        ctx.options,
                    )])
                }
                None => ToolResponse::default(),
            };
        }
        if let Some((a, b)) = line {
            let ang = -((b.y - a.y) as f64).atan2((b.x - a.x) as f64).to_degrees();
            self.overlay = vec![
                Overlay::Line { from: a, to: b },
                Overlay::Label {
                    at: b,
                    text: format!("{} px · {}°", fmt_num(dist(a, b)), fmt_num(ang as f32)),
                },
            ];
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
    /// Nach Esc/Eingabe bleibt der offene Pfad gewählt; der nächste Klick setzt ihn am Endpunkt
    /// fort (Konturindex; PAR 09/07). Doppelklick, Schließen oder Abwählen beenden das.
    resume: Option<usize>,
    /// Klick/Ziehen auf den letzten Punkt des laufenden Pfads: Punkt umwandeln (FB-01, re 52 §1.4).
    convert: Option<ConvertLast>,
    /// Alt-Ziehen am End- bzw. Anfangspunkt eines gewählten Pfads (Fortsetzen): beide Henkel
    /// symmetrisch wie ein glatter Punkt (B-2b BA-003: 400,200 → 430,200 ergibt 370 · 430).
    smooth_drag: bool,
    /// Alt-Ziehen an einem mittleren Punkt des gewählten Pfads: zieht dort einen glatten Punkt auf,
    /// statt ihn zu löschen (B-2b BA-003: 300,160 → 330,160 ergibt 270 · 330).
    alt_point: Option<AltPoint>,
}

/// Press on the last anchor of the path being drawn (convert point, re 52 §1.4/§1.5): the outgoing
/// handle is retracted on press; a drag past 2 screen px pulls the point again as a symmetric curve
/// point (Alt: only the outgoing handle follows, the incoming one keeps `pred0`; Ctrl: moves it).
#[derive(Clone, Copy, Debug)]
struct ConvertLast {
    /// Incoming handle before the press.
    pred0: Point,
    down: Point,
    last: Point,
    dragged: bool,
}

/// What the pen hits on the path being drawn (re 52 §1.3).
enum DrawHit {
    /// First point: close.
    Close,
    /// Last point: convert (retract the outgoing handle).
    Last,
    /// Interior point.
    Interior(usize),
    /// Segment (index, t): insert a point.
    Segment(usize, f64),
    /// Endpoint of another open path: join.
    Join {
        id: ElementId,
        contour: usize,
        at_start: bool,
    },
}

/// Anchor hit radius of the pen: `SnapDistance` screen pixels (default 5, re 52 §1.3).
fn pen_tol(ctx: &ToolContext) -> f32 {
    (ctx.prefs.snap_distance() / ctx.zoom.max(1e-6)) as f32
}

/// Laufendes Alt-Ziehen an einem vorhandenen Punkt (BA-003).
#[derive(Clone, Debug)]
struct AltPoint {
    id: ElementId,
    contour: usize,
    node: usize,
    contours: Vec<Contour>,
    moved: bool,
}

/// Biegt ein Henkel des Knotens das Segment (liegt nicht auf der Geraden zum Nachbarpunkt)?
/// Ein auf einer Geraden eingefügter Punkt ist so ein Eckpunkt, ein Bogenpunkt wie 370,200 von P1
/// ein Kurvenpunkt.
fn is_curve_node(c: &Contour, i: usize) -> bool {
    let n = c.nodes.len();
    let Some(k) = c.nodes.get(i) else {
        return false;
    };
    let bends = |h: Point, other: Option<usize>| -> bool {
        if dist(h, k.anchor) < 1e-3 {
            return false;
        }
        let Some(o) = other.and_then(|j| c.nodes.get(j)) else {
            return true;
        };
        let (ux, uy) = (h.x - k.anchor.x, h.y - k.anchor.y);
        let (vx, vy) = (o.anchor.x - k.anchor.x, o.anchor.y - k.anchor.y);
        let cross = (ux * vy - uy * vx).abs();
        cross > 1e-3 * (ux.hypot(uy) * vx.hypot(vy)).max(1e-6)
    };
    let prev = if i > 0 {
        Some(i - 1)
    } else if c.closed && n > 1 {
        Some(n - 1)
    } else {
        None
    };
    let next = if i + 1 < n {
        Some(i + 1)
    } else if c.closed && n > 1 {
        Some(0)
    } else {
        None
    };
    bends(k.pred, prev) || bends(k.succ, next)
}

/// `tool.pick` (Auswahl ohne Verlaufsschritt).
fn pick_select(id: ElementId) -> Invocation {
    Invocation::new(
        "tool.pick",
        json!({"command": "select.set", "params": {"ids": [id.0], "mode": "replace"}}),
    )
}

/// Was unter dem Stift liegt (nur bei genau einem ausgewählten Pfad und ohne laufenden Pfad).
enum PenHit {
    /// Endpunkt einer offenen Kontur (am Anfang = `true`).
    End {
        id: ElementId,
        contour: usize,
        at_start: bool,
    },
    Point {
        id: ElementId,
        contour: usize,
        node: usize,
    },
    Segment {
        id: ElementId,
        contour: usize,
        segment: usize,
        t: f64,
    },
}

impl PenTool {
    fn contour(&self, closed: bool) -> Contour {
        geometry::contour_from_nodes(&self.nodes, closed)
    }

    fn pen_hit(ctx: &ToolContext, p: Point) -> Option<PenHit> {
        Self::pen_hit_selected(ctx, p).or_else(|| Self::open_end_any(ctx, p))
    }

    /// Endpunkt eines offenen, **nicht** gewählten Pfads unter `p` (oberstes zuerst): der Stift setzt
    /// ihn fort wie im Original (PAR 09/08).
    fn open_end_any(ctx: &ToolContext, p: Point) -> Option<PenHit> {
        Self::open_end_except(ctx, p, None)
    }

    /// Like [`Self::open_end_any`], skipping one contour (the path being continued).
    fn open_end_except(
        ctx: &ToolContext,
        p: Point,
        skip: Option<(ElementId, usize)>,
    ) -> Option<PenHit> {
        let tol = pen_tol(ctx);
        for id in docops::selectable_ids(ctx.doc, ctx.page, ctx.frame) {
            let Some(e) = docops::element_on_page(ctx.doc, ctx.page, id) else {
                continue;
            };
            let ElementKind::Path(pe) = &e.kind else {
                continue;
            };
            if pe.primitive.is_some() {
                continue;
            }
            for (ci, c) in pe.contours.iter().enumerate() {
                let n = c.nodes.len();
                if c.closed || n < 2 || skip == Some((id, ci)) {
                    continue;
                }
                if dist(c.nodes[n - 1].anchor, p) <= tol {
                    return Some(PenHit::End {
                        id,
                        contour: ci,
                        at_start: false,
                    });
                }
                if dist(c.nodes[0].anchor, p) <= tol {
                    return Some(PenHit::End {
                        id,
                        contour: ci,
                        at_start: true,
                    });
                }
            }
        }
        None
    }

    fn pen_hit_selected(ctx: &ToolContext, p: Point) -> Option<PenHit> {
        if ctx.selection.elements.len() != 1 {
            return None;
        }
        let id = ctx.selection.elements[0];
        let e = docops::element_on_page(ctx.doc, ctx.page, id)?;
        let ElementKind::Path(pe) = &e.kind else {
            return None;
        };
        if pe.primitive.is_some() {
            return None;
        }
        let tol = pen_tol(ctx);
        for (ci, c) in pe.contours.iter().enumerate() {
            let n = c.nodes.len();
            if n == 0 {
                continue;
            }
            if !c.closed {
                if dist(c.nodes[n - 1].anchor, p) <= tol {
                    return Some(PenHit::End {
                        id,
                        contour: ci,
                        at_start: false,
                    });
                }
                if dist(c.nodes[0].anchor, p) <= tol {
                    return Some(PenHit::End {
                        id,
                        contour: ci,
                        at_start: true,
                    });
                }
            }
            for (ni, k) in c.nodes.iter().enumerate() {
                if dist(k.anchor, p) <= tol {
                    return Some(PenHit::Point {
                        id,
                        contour: ci,
                        node: ni,
                    });
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
                return Some(PenHit::Segment {
                    id,
                    contour: ci,
                    segment: seg,
                    t: near.t,
                });
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
            // Verlaufsschritt „Stift“ wie im Original (PAR 09/05); der Pfad wird gewählt
            return ToolResponse::commit(vec![
                pick_select(t.id),
                Invocation::new(
                    "path.setContours",
                    json!({"id": t.id.0, "contours": contours_json(&cs), "step": "pen"}),
                ),
            ]);
        }
        if nodes.len() < 2 {
            return ToolResponse::default();
        }
        let c = json!({"closed": closed, "nodes": nodes.iter().map(|(a, i, o)| json!({"anchor": pt_json(*a), "in": pt_json(*i), "out": pt_json(*o)})).collect::<Vec<Value>>()});
        ToolResponse::commit(vec![super::styled(
            Invocation::new("draw.path", json!({"contours": [c], "step": "pen"})),
            opts,
        )])
    }

    /// Esc/Eingabe: Pfad beenden, aber zum Fortsetzen merken (PAR 09/07).
    fn commit_resumable(&mut self, opts: &super::ToolOptions) -> ToolResponse {
        let contour = self.target.as_ref().map(|t| t.contour).unwrap_or(0);
        let had = self.nodes.len() >= 2;
        let r = self.commit(false, opts);
        if had {
            self.resume = Some(contour);
        }
        r
    }

    /// Is the node of a document path a curve point (a click retracts its handles)?
    fn is_curve_point(ctx: &ToolContext, id: ElementId, contour: usize, node: usize) -> bool {
        docops::element_on_page(ctx.doc, ctx.page, id)
            .and_then(|e| match &e.kind {
                ElementKind::Path(pe) => pe.contours.get(contour).map(|c| is_curve_node(c, node)),
                _ => None,
            })
            .unwrap_or(false)
    }

    /// Classifies `p` against the path being drawn (re 52 §1.3): first point closes, last point
    /// converts, interior points convert/delete, segments insert, endpoints of other open paths
    /// join. Shift suppresses interior, segment and join.
    fn draw_hit(&self, ctx: &ToolContext, p: Point, shift: bool) -> Option<DrawHit> {
        let n = self.nodes.len();
        if n == 0 {
            return None;
        }
        let tol = pen_tol(ctx);
        if n >= 2 && dist(p, self.nodes[0].0) <= tol {
            return Some(DrawHit::Close);
        }
        if dist(p, self.nodes[n - 1].0) <= tol {
            return Some(DrawHit::Last);
        }
        if shift {
            return None;
        }
        if let Some(i) = (1..n - 1).find(|&i| dist(p, self.nodes[i].0) <= tol) {
            return Some(DrawHit::Interior(i));
        }
        if n >= 2 {
            let bc = geometry::to_bezier(&self.contour(false));
            if let Some((seg, near)) = bc.nearest(fp_geom::Vec2::from(p))
                && near.distance <= tol as f64
                && near.t > 0.001
                && near.t < 0.999
            {
                return Some(DrawHit::Segment(seg, near.t));
            }
        }
        let skip = self.target.as_ref().map(|t| (t.id, t.contour));
        match Self::open_end_except(ctx, p, skip) {
            Some(PenHit::End {
                id,
                contour,
                at_start,
            }) => {
                // joining with another element than the one being continued is not supported:
                // the press appends a point instead
                if self.target.as_ref().is_some_and(|t| t.id != id) {
                    return None;
                }
                Some(DrawHit::Join {
                    id,
                    contour,
                    at_start,
                })
            }
            _ => None,
        }
    }

    /// Joins the path being drawn with the open contour `contour` of `id` at the hit endpoint
    /// (re 52 §1.2 mode 8) and ends drawing. The joined contour keeps the attributes of the hit
    /// path.
    fn join(
        &mut self,
        ctx: &ToolContext,
        id: ElementId,
        contour: usize,
        at_start: bool,
    ) -> Option<ToolResponse> {
        let e = docops::element_on_page(ctx.doc, ctx.page, id)?;
        let ElementKind::Path(pe) = &e.kind else {
            return None;
        };
        let c = pe.contours.get(contour)?;
        let mut other: Vec<PenNode> = c.nodes.iter().map(|n| (n.anchor, n.pred, n.succ)).collect();
        if !at_start {
            other.reverse();
            for n in &mut other {
                *n = (n.0, n.2, n.1);
            }
        }
        let mut merged = self.nodes.clone();
        merged.extend(other);
        let joined = geometry::contour_from_nodes(&merged, false);
        let cs = match &self.target {
            None => {
                let mut cs = pe.contours.clone();
                cs[contour] = joined;
                cs
            }
            Some(t) if t.id == id && t.contour != contour => {
                let mut cs = t.contours.clone();
                *cs.get_mut(t.contour)? = joined;
                cs.remove(contour);
                cs
            }
            Some(_) => return None,
        };
        self.nodes.clear();
        self.target = None;
        self.closing = false;
        self.button = false;
        self.hover = None;
        self.guides.clear();
        self.resume = None;
        Some(ToolResponse::commit(vec![
            pick_select(id),
            Invocation::new(
                "path.setContours",
                json!({"id": id.0, "contours": contours_json(&cs), "step": "pen"}),
            ),
        ]))
    }

    /// Konturen des Pfads mit eingezogenen Henkeln am Knoten (Kurvenpunkt → Eckpunkt).
    fn corner_contours(
        ctx: &ToolContext,
        id: ElementId,
        contour: usize,
        node: usize,
    ) -> Option<Vec<Contour>> {
        let e = docops::element_on_page(ctx.doc, ctx.page, id)?;
        let ElementKind::Path(pe) = &e.kind else {
            return None;
        };
        let mut cs = pe.contours.clone();
        let n = cs.get_mut(contour)?.nodes.get_mut(node)?;
        n.pred = n.anchor;
        n.succ = n.anchor;
        n.curve = false;
        Some(cs)
    }

    /// Fortsetzen: Knoten der Kontur übernehmen (am Anfang begonnen → umgedreht).
    fn start_continue(
        &mut self,
        ctx: &ToolContext,
        id: ElementId,
        contour: usize,
        at_start: bool,
    ) -> bool {
        let Some(e) = docops::element_on_page(ctx.doc, ctx.page, id) else {
            return false;
        };
        let ElementKind::Path(pe) = &e.kind else {
            return false;
        };
        let Some(c) = pe.contours.get(contour) else {
            return false;
        };
        let mut nodes: Vec<PenNode> = c.nodes.iter().map(|n| (n.anchor, n.pred, n.succ)).collect();
        if at_start {
            nodes.reverse();
            for n in &mut nodes {
                *n = (n.0, n.2, n.1);
            }
        }
        self.nodes = nodes;
        self.target = Some(PenTarget {
            id,
            contour,
            contours: pe.contours.clone(),
        });
        true
    }

    /// Zuletzt mit der Teilauswahl gewählter Endpunkt der offenen Kontur (`true` = Anfang), `None`
    /// ohne Teilauswahl-Eingriff (BA-001, re 29 §6 L1).
    fn active_end(ctx: &ToolContext, id: ElementId, contour: usize) -> Option<bool> {
        let e = docops::element_on_page(ctx.doc, ctx.page, id)?;
        let ElementKind::Path(pe) = &e.kind else {
            return None;
        };
        let c = pe.contours.get(contour)?;
        let n = c.nodes.len();
        if c.closed || n < 2 {
            return None;
        }
        ctx.selection
            .points
            .iter()
            .rev()
            .filter(|r| r.element == id && r.contour == contour)
            .find_map(|r| {
                if r.node == 0 {
                    Some(true)
                } else if r.node == n - 1 {
                    Some(false)
                } else {
                    None
                }
            })
    }

    fn hover_cursor(&self, ctx: &ToolContext, p: Point, shift: bool) -> Cursor {
        if !self.nodes.is_empty() {
            return match self.draw_hit(ctx, p, shift) {
                Some(DrawHit::Close) => Cursor::PenClose,
                Some(DrawHit::Last) => Cursor::PenConvert,
                Some(DrawHit::Interior(i)) => {
                    let (a, pr, su) = self.nodes[i];
                    if pr == a && su == a {
                        Cursor::PenRemove
                    } else {
                        Cursor::PenConvert
                    }
                }
                Some(DrawHit::Segment(..)) => Cursor::PenAdd,
                Some(DrawHit::Join { .. }) => Cursor::PenJoin,
                None => Cursor::Pen,
            };
        }
        match Self::pen_hit(ctx, p) {
            Some(PenHit::End { .. }) => Cursor::PenContinue,
            Some(PenHit::Point { id, contour, node }) => {
                if Self::is_curve_point(ctx, id, contour, node) {
                    Cursor::PenConvert
                } else {
                    Cursor::PenRemove
                }
            }
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
        let tol = pen_tol(ctx);
        match &ev.input {
            ToolInput::Pointer {
                kind: PointerKind::Down,
                pos,
            } => {
                self.button = true;
                self.convert = None;
                self.smooth_drag = false;
                self.alt_point = None;
                if self.nodes.len() >= 2 && dist(*pos, self.nodes[0].0) <= tol {
                    self.closing = true;
                    return ToolResponse {
                        cursor: Cursor::PenClose,
                        ..ToolResponse::default()
                    };
                }
                match self.draw_hit(ctx, *pos, ev.mods.shift) {
                    Some(DrawHit::Last) => {
                        // convert point: retract the outgoing handle, the incoming one stays; the
                        // path stays in progress (FB-01, re 52 §1.4)
                        if let Some(n) = self.nodes.last_mut() {
                            self.convert = Some(ConvertLast {
                                pred0: n.1,
                                down: *pos,
                                last: *pos,
                                dragged: false,
                            });
                            n.2 = n.0;
                        }
                        self.hover = None;
                        self.guides.clear();
                        return ToolResponse::with_overlay(self.overlay(), Cursor::PenConvert);
                    }
                    Some(DrawHit::Interior(i)) => {
                        // curve point -> corner (both handles retracted), corner point -> delete
                        self.button = false;
                        let (a, pr, su) = self.nodes[i];
                        if pr == a && su == a {
                            self.nodes.remove(i);
                        } else {
                            self.nodes[i] = (a, a, a);
                        }
                        return ToolResponse::with_overlay(self.overlay(), Cursor::Pen);
                    }
                    Some(DrawHit::Segment(seg, t)) => {
                        self.button = false;
                        if let Some(bc) =
                            geometry::to_bezier(&self.contour(false)).insert_node(seg, t)
                        {
                            let c = geometry::from_bezier(&bc);
                            self.nodes =
                                c.nodes.iter().map(|n| (n.anchor, n.pred, n.succ)).collect();
                        }
                        return ToolResponse::with_overlay(self.overlay(), Cursor::PenAdd);
                    }
                    Some(DrawHit::Join {
                        id,
                        contour,
                        at_start,
                    }) => {
                        if let Some(r) = self.join(ctx, id, contour, at_start) {
                            return r;
                        }
                    }
                    _ => {}
                }
                if self.nodes.is_empty() {
                    let resume = self.resume.take();
                    match Self::pen_hit(ctx, *pos) {
                        Some(PenHit::End {
                            id,
                            contour,
                            at_start,
                        }) => {
                            // BA-001 (re 29 §6 L1, re 33 §6): aktiv ist der zuletzt per Teilauswahl
                            // angefasste Endpunkt des gewählten Pfads, ohne Teilauswahl-Eingriff der
                            // letzte Punkt. Ein Klick auf den **anderen** Endpunkt schließt den Pfad
                            // (Knotenzahl bleibt), der nächste Klick beginnt einen neuen Pfad.
                            // FP2-02 (re 37 §2): ein Klick auf den **Anfangspunkt** schließt immer,
                            // auch wenn er per Teilauswahl aktiv ist; nur der Endpunkt setzt fort
                            let active = Self::active_end(ctx, id, contour).unwrap_or(false);
                            if ctx.selection.elements == [id]
                                && (at_start || active != at_start)
                                && self.start_continue(ctx, id, contour, active && !at_start)
                                && self.nodes.len() >= 2
                            {
                                self.closing = true;
                                return ToolResponse {
                                    cursor: Cursor::PenClose,
                                    ..ToolResponse::default()
                                };
                            }
                            if self.start_continue(ctx, id, contour, at_start) {
                                // Alt-Ziehen am Endpunkt: glatter Punkt, beide Henkel (BA-003)
                                self.smooth_drag = ev.mods.alt;
                                let cmds = if ctx.selection.elements == [id] {
                                    Vec::new()
                                } else {
                                    vec![pick_select(id)]
                                };
                                return ToolResponse {
                                    commands: cmds,
                                    overlay: self.overlay(),
                                    cursor: Cursor::PenContinue,
                                    ..ToolResponse::default()
                                };
                            }
                        }
                        Some(PenHit::Point { id, contour, node }) if ev.mods.alt => {
                            // Alt-Ziehen am mittleren Punkt: glatten Punkt aufziehen; erst das
                            // Loslassen entscheidet (ohne Bewegung wie ein Klick, BA-003)
                            if let Some(e) = docops::element_on_page(ctx.doc, ctx.page, id)
                                && let ElementKind::Path(pe) = &e.kind
                            {
                                self.alt_point = Some(AltPoint {
                                    id,
                                    contour,
                                    node,
                                    contours: pe.contours.clone(),
                                    moved: false,
                                });
                                return ToolResponse::with_overlay(Vec::new(), Cursor::Pen);
                            }
                        }
                        Some(PenHit::Point { id, contour, node }) => {
                            self.button = false;
                            // Kurvenpunkt → Eckpunkt (Henkel eingezogen), Eckpunkt → entfernen; Schritt
                            // „Stift“ (PAR 09/10)
                            let curve = docops::element_on_page(ctx.doc, ctx.page, id)
                                .and_then(|e| match &e.kind {
                                    ElementKind::Path(pe) => {
                                        pe.contours.get(contour).map(|c| is_curve_node(c, node))
                                    }
                                    _ => None,
                                })
                                .unwrap_or(false);
                            if curve && let Some(cs) = Self::corner_contours(ctx, id, contour, node)
                            {
                                return ToolResponse::commit(vec![Invocation::new(
                                    "path.setContours",
                                    json!({"id": id.0, "contours": contours_json(&cs), "step": "pen"}),
                                )]);
                            }
                            return ToolResponse::commit(vec![Invocation::new(
                                "path.removeNode",
                                json!({"id": id.0, "contour": contour, "node": node, "step": "pen"}),
                            )]);
                        }
                        Some(PenHit::Segment {
                            id,
                            contour,
                            segment,
                            t,
                        }) => {
                            self.button = false;
                            return ToolResponse::commit(vec![Invocation::new(
                                "path.insertNode",
                                json!({"id": id.0, "contour": contour, "segment": segment, "t": t, "step": "pen"}),
                            )]);
                        }
                        _ => {}
                    }
                    // Esc/Eingabe davor: der gewählte offene Pfad wird am Endpunkt fortgesetzt (PAR 09/07)
                    if let Some(ci) = resume
                        && ctx.selection.elements.len() == 1
                        && let Some(e) =
                            docops::element_on_page(ctx.doc, ctx.page, ctx.selection.elements[0])
                        && let ElementKind::Path(pe) = &e.kind
                        && pe.primitive.is_none()
                        && pe
                            .contours
                            .get(ci)
                            .is_some_and(|c| !c.closed && !c.nodes.is_empty())
                    {
                        let id = e.id;
                        // am zuletzt per Teilauswahl angefassten Endpunkt, sonst am letzten Punkt (BA-001)
                        let at_start = Self::active_end(ctx, id, ci).unwrap_or(false);
                        self.start_continue(ctx, id, ci, at_start);
                    }
                }
                // Stiftpunkte rasten nicht an Objekten (PAR 09/11: 102,207 und 152,122 bleiben)
                let snapped = Snapper::new(ctx, &[]).without_smart().point(*pos);
                let p = match self.nodes.last() {
                    Some(last) if ev.mods.shift => constrain_angle(last.0, *pos, 45.0),
                    _ => snapped.point,
                };
                self.nodes.push((p, p, p));
                self.guides = snapped.guides;
                ToolResponse::with_overlay(self.overlay(), Cursor::Pen)
            }
            ToolInput::Pointer {
                kind: PointerKind::Move,
                pos,
            } => {
                if self.button
                    && let Some(ap) = self.alt_point.as_mut()
                {
                    let Some(n) = ap
                        .contours
                        .get_mut(ap.contour)
                        .and_then(|c| c.nodes.get_mut(ap.node))
                    else {
                        return ToolResponse::default();
                    };
                    let a = n.anchor;
                    let q = if ev.mods.shift {
                        constrain_angle(a, *pos, 45.0)
                    } else {
                        *pos
                    };
                    if dist(q, a) > 0.5 {
                        ap.moved = true;
                    }
                    if ap.moved {
                        n.succ = q;
                        n.pred = Point::new(2.0 * a.x - q.x, 2.0 * a.y - q.y);
                        n.curve = true;
                    }
                    let c = ap.contours[ap.contour].clone();
                    let ov = vec![
                        Overlay::Path {
                            contours: vec![c.clone()],
                        },
                        Overlay::PathPoints {
                            contours: vec![c],
                            selected: vec![(0, ap.node)],
                            handles: true,
                        },
                    ];
                    return ToolResponse::with_overlay(ov, Cursor::Pen);
                }
                if self.button
                    && let Some(cv) = self.convert.as_mut()
                {
                    // drag threshold 2 screen px (re 52 §1.1)
                    let thr = (2.0 / ctx.zoom.max(1e-6)) as f32;
                    if !cv.dragged {
                        if (pos.x - cv.down.x).abs() < thr && (pos.y - cv.down.y).abs() < thr {
                            return ToolResponse::with_overlay(self.overlay(), Cursor::PenConvert);
                        }
                        // first drag step: both handles back onto the anchor, pulled again
                        cv.dragged = true;
                        if let Some(n) = self.nodes.last_mut() {
                            n.1 = n.0;
                            n.2 = n.0;
                        }
                    }
                    let (dx, dy) = (pos.x - cv.last.x, pos.y - cv.last.y);
                    cv.last = *pos;
                    let pred0 = cv.pred0;
                    if let Some(n) = self.nodes.last_mut() {
                        if ev.mods.ctrl {
                            // Ctrl moves the point with its handles
                            for q in [&mut n.0, &mut n.1, &mut n.2] {
                                *q = Point::new(q.x + dx, q.y + dy);
                            }
                        } else {
                            let a = n.0;
                            let q = if ev.mods.shift {
                                constrain_angle(a, *pos, 45.0)
                            } else {
                                *pos
                            };
                            n.2 = q;
                            // Alt decouples: the incoming handle keeps its previous position
                            n.1 = if ev.mods.alt {
                                pred0
                            } else {
                                Point::new(2.0 * a.x - q.x, 2.0 * a.y - q.y)
                            };
                        }
                    }
                    self.guides.clear();
                    self.cursor = Cursor::Pen;
                    return ToolResponse::with_overlay(self.overlay(), self.cursor);
                }
                if self.button {
                    let idx = if self.closing {
                        0
                    } else {
                        self.nodes.len().saturating_sub(1)
                    };
                    if let Some(n) = self.nodes.get_mut(idx) {
                        let a = n.0;
                        let q = if ev.mods.shift {
                            constrain_angle(a, *pos, 45.0)
                        } else {
                            *pos
                        };
                        if self.smooth_drag {
                            // Alt am Endpunkt eines gewählten Pfads: glatter Punkt (BA-003)
                            n.2 = q;
                            n.1 = Point::new(2.0 * a.x - q.x, 2.0 * a.y - q.y);
                        } else if self.closing {
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
                    self.cursor = if self.closing {
                        Cursor::PenClose
                    } else {
                        Cursor::Pen
                    };
                } else {
                    let sp = Snapper::new(ctx, &[]).without_smart().point(*pos);
                    let h = match self.nodes.last() {
                        Some(last) if ev.mods.shift => constrain_angle(last.0, *pos, 45.0),
                        _ => sp.point,
                    };
                    self.hover = Some(h);
                    self.guides = sp.guides;
                    self.cursor = self.hover_cursor(ctx, *pos, ev.mods.shift);
                }
                ToolResponse::with_overlay(self.overlay(), self.cursor)
            }
            ToolInput::Pointer {
                kind: PointerKind::Up,
                ..
            } => {
                self.button = false;
                self.convert = None;
                self.smooth_drag = false;
                if let Some(ap) = self.alt_point.take() {
                    if ap.moved {
                        return ToolResponse::commit(vec![Invocation::new(
                            "path.setContours",
                            json!({"id": ap.id.0, "contours": contours_json(&ap.contours), "step": "pen"}),
                        )]);
                    }
                    // ohne Bewegung wie ein Klick: Kurvenpunkt → Eckpunkt, Eckpunkt entfernen
                    let curve = ap
                        .contours
                        .get(ap.contour)
                        .is_some_and(|c| is_curve_node(c, ap.node));
                    if curve
                        && let Some(cs) = Self::corner_contours(ctx, ap.id, ap.contour, ap.node)
                    {
                        return ToolResponse::commit(vec![Invocation::new(
                            "path.setContours",
                            json!({"id": ap.id.0, "contours": contours_json(&cs), "step": "pen"}),
                        )]);
                    }
                    return ToolResponse::commit(vec![Invocation::new(
                        "path.removeNode",
                        json!({"id": ap.id.0, "contour": ap.contour, "node": ap.node, "step": "pen"}),
                    )]);
                }
                if self.closing {
                    self.resume = None;
                    return self.commit(true, ctx.options);
                }
                ToolResponse::with_overlay(self.overlay(), Cursor::Pen)
            }
            ToolInput::Pointer {
                kind: PointerKind::DoubleClick,
                ..
            } => {
                // der Doppelklick hat bereits einen Punkt gesetzt (Down) – doppelten Endpunkt entfernen
                if self.nodes.len() >= 2 {
                    let n = self.nodes.len();
                    if dist(self.nodes[n - 1].0, self.nodes[n - 2].0) <= tol {
                        self.nodes.pop();
                    }
                }
                // Doppelklick beendet endgültig: nächster Klick beginnt einen neuen Pfad (PAR 09/06)
                self.resume = None;
                self.commit(false, ctx.options)
            }
            ToolInput::Key(ToolKey::Enter) | ToolInput::Key(ToolKey::Escape) => {
                self.commit_resumable(ctx.options)
            }
            ToolInput::Key(ToolKey::Backspace) | ToolInput::Key(ToolKey::Delete)
                if !self.nodes.is_empty() =>
            {
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
            v.push(Overlay::Path {
                contours: vec![self.contour(false)],
            });
            let last = self.nodes.len() - 1;
            let mut pts = self.contour(false);
            if self.closing && pts.nodes.len() > 1 {
                pts.closed = true;
            }
            v.push(Overlay::PathPoints {
                contours: vec![pts],
                selected: vec![(0, if self.closing { 0 } else { last })],
                handles: true,
            });
            if let (Some(h), Some(l)) = (self.hover, self.nodes.last())
                && !self.button
            {
                if l.2 == l.0 {
                    v.push(Overlay::Line { from: l.0, to: h });
                } else {
                    // Vorschau des nächsten Segments (`ShowPenPreview`): Kurve aus dem Ausgangshenkel
                    v.push(Overlay::Path {
                        contours: vec![geometry::contour_from_nodes(&[*l, (h, h, h)], false)],
                    });
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
        FreehandTool {
            id,
            points: Vec::new(),
            active: false,
        }
    }
}

/// Freihand-Kontur wie im Original (PAR 09/12): vereinfachte Punktfolge mit Knoten auf
/// Pixelmitten; an scharfen Knicken (ab 50°) Eckpunkte ohne Henkel, sonst glatte Kurvenpunkte.
pub fn freehand_contour(points: &[Point], precision: u32, closed: bool) -> Option<Contour> {
    if points.len() < 2 {
        return None;
    }
    let tol = (11 - precision.clamp(1, 10)) as f64 * 0.4;
    let mut simple: Vec<Point> = geometry::simplify_polyline(points, tol)
        .into_iter()
        .map(pixel_center)
        .collect();
    simple.dedup_by(|a, b| dist(*a, *b) < 0.5);
    if simple.len() < 2 {
        return None;
    }
    let n = simple.len();
    let smooth = geometry::smooth_contour(&simple, closed);
    let nodes: Vec<PenNode> = (0..n)
        .map(|i| {
            let a = simple[i];
            let inner = closed || (i > 0 && i + 1 < n);
            if !inner {
                return (a, a, a);
            }
            let (p, q) = (simple[(i + n - 1) % n], simple[(i + 1) % n]);
            let (ux, uy) = ((a.x - p.x) as f64, (a.y - p.y) as f64);
            let (vx, vy) = ((q.x - a.x) as f64, (q.y - a.y) as f64);
            let turn = (ux * vy - uy * vx)
                .atan2(ux * vx + uy * vy)
                .abs()
                .to_degrees();
            match smooth.nodes.get(i) {
                Some(s) if turn < 50.0 => (a, s.pred, s.succ),
                _ => (a, a, a),
            }
        })
        .collect();
    Some(geometry::contour_from_nodes(&nodes, closed))
}

/// Kontur → Parameter für `draw.path` bzw. `path.setContours`.
fn contour_json(c: &Contour) -> Value {
    json!({"closed": c.closed, "nodes": c.nodes.iter().map(|n| json!({"anchor": pt_json(n.anchor), "in": pt_json(n.pred), "out": pt_json(n.succ)})).collect::<Vec<Value>>()})
}

/// Anpassung des Vektorpfad-Zugs nach dem Loslassen wie im Original (re 27 P3): Schneider-Anpassung
/// (dasselbe Verfahren wie „Vereinfachen“) mit Toleranz `10 − PathToolPrecision` Pixel, Eckwinkel
/// 30°, 16 Stichproben; bei Präzision 10 keine Anpassung. Ecken (Knick > 30°) bleiben feste Knoten,
/// angepasst werden die Abschnitte dazwischen; ein Abschnitt aus nur einem Segment bleibt, wie er
/// ist (die Anpassung bildet ihn nur nach). Ohne Pfad-Backend bleibt die Kontur unverändert.
pub fn refit_freehand(c: Contour, precision: u32, geo: &dyn geometry::GeometryOps) -> Contour {
    let n = c.nodes.len();
    if precision >= 10 || c.closed || n < 3 {
        return c;
    }
    let tol = (10 - precision.clamp(1, 10)) as f64;
    let dir = |a: Point, b: Point| -> Option<(f64, f64)> {
        let (x, y) = ((b.x - a.x) as f64, (b.y - a.y) as f64);
        let l = x.hypot(y);
        (l > 1e-9).then(|| (x / l, y / l))
    };
    let corner = |i: usize| -> bool {
        let (p, a, q) = (&c.nodes[i - 1], &c.nodes[i], &c.nodes[i + 1]);
        let tin = [a.pred, p.succ, p.anchor]
            .into_iter()
            .find_map(|f| dir(f, a.anchor));
        let tout = [a.succ, q.pred, q.anchor]
            .into_iter()
            .find_map(|t| dir(a.anchor, t));
        match (tin, tout) {
            (Some(u), Some(v)) => {
                (u.0 * v.0 + u.1 * v.1).clamp(-1.0, 1.0).acos().to_degrees() > 30.0
            }
            _ => true,
        }
    };
    let mut out: Vec<fp_model::ContourNode> = Vec::with_capacity(n);
    let mut start = 0;
    for i in 1..n {
        if i < n - 1 && !corner(i) {
            continue;
        }
        let run = &c.nodes[start..=i];
        let mut nodes = if run.len() > 2 {
            let rc = Contour {
                closed: false,
                nodes: run.to_vec(),
                ..c.clone()
            };
            geo.simplify(&[rc], tol)
                .ok()
                .and_then(|v| v.into_iter().next())
                .filter(|r| r.nodes.len() >= 2)
                .map(|r| r.nodes)
                .unwrap_or_else(|| run.to_vec())
        } else {
            run.to_vec()
        };
        if !out.is_empty() && !nodes.is_empty() {
            // Nahtstelle an der Ecke: Eingang vom vorigen, Ausgang vom neuen Abschnitt
            let first = nodes.remove(0);
            if let Some(l) = out.last_mut() {
                l.succ = first.succ;
            }
        }
        out.extend(nodes);
        start = i;
    }
    Contour { nodes: out, ..c }
}

/// Pfad nachzeichnen wie im Original (PAR 09/14): beginnt der Zug am gewählten Pfad, wird nur der
/// Abschnitt zwischen Anfangs- und Endpunkt des Zugs ersetzt (endet der Zug abseits, ab dem
/// Anfangspunkt bis zum Konturende). `None`: Zug beginnt nicht an einer offenen Kontur.
fn redraw_section(
    contours: &[Contour],
    new: &Contour,
    start: Point,
    end: Point,
    tol: f64,
) -> Option<Vec<Contour>> {
    let near = |c: &Contour, p: Point| -> Option<(usize, f64, f64)> {
        let bc = geometry::to_bezier(c);
        bc.nearest(fp_geom::Vec2::from(p))
            .map(|(seg, nr)| (seg, nr.t, nr.distance))
    };
    let (ci, (s_seg, s_t, _)) = contours
        .iter()
        .enumerate()
        .filter(|(_, c)| !c.closed && c.nodes.len() >= 2)
        .filter_map(|(i, c)| near(c, start).filter(|h| h.2 <= tol).map(|h| (i, h)))
        .min_by(|a, b| a.1.2.total_cmp(&b.1.2))?;
    let c = &contours[ci];
    let end_hit = near(c, end).filter(|h| h.2 <= tol);
    let s_par = s_seg as f64 + s_t;
    let mut new_nodes = new.nodes.clone();
    let (lo, hi, reversed) = match end_hit {
        Some((e_seg, e_t, _)) => {
            let e_par = e_seg as f64 + e_t;
            if e_par >= s_par {
                (s_par, e_par, false)
            } else {
                (e_par, s_par, true)
            }
        }
        None => (s_par, (c.nodes.len() - 1) as f64, false),
    };
    if reversed {
        new_nodes.reverse();
        for k in &mut new_nodes {
            std::mem::swap(&mut k.pred, &mut k.succ);
        }
    }
    let split = |par: f64| -> (usize, f64) {
        let seg = (par.floor() as usize).min(c.nodes.len().saturating_sub(2));
        (seg, (par - seg as f64).clamp(0.0, 1.0))
    };
    let last = (c.nodes.len() - 1) as f64;
    // Stück vor dem Anfang bzw. nach dem Ende (an den Nahtstellen Eckpunkte)
    let mut nodes = if lo > 1e-6 {
        super::pathedit::split_at(c, &[split(lo)])
            .first()
            .map(|h| h.nodes.clone())
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    if let Some(l) = nodes.last_mut() {
        l.succ = l.anchor;
    }
    nodes.extend(new_nodes);
    if end_hit.is_some() && hi < last - 1e-6 {
        let mut t = super::pathedit::split_at(c, &[split(hi)])
            .last()
            .map(|t| t.nodes.clone())
            .unwrap_or_default();
        if let Some(f) = t.first_mut() {
            f.pred = f.anchor;
        }
        nodes.extend(t);
    }
    let mut out = contours.to_vec();
    out[ci] = Contour {
        closed: false,
        nodes,
        ..c.clone()
    };
    Some(out)
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
        let ToolInput::Pointer { kind, pos } = &ev.input else {
            return ToolResponse::default();
        };
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
                let Some(contour) = freehand_contour(&pts, ctx.options.path_precision, false)
                else {
                    return ToolResponse::default();
                };
                // nur das Vektorpfad-Werkzeug passt nach dem Zug neu an (re 27 P3)
                let contour = if self.id == ToolId::VectorPath {
                    refit_freehand(contour, ctx.options.path_precision, ctx.geometry)
                } else {
                    contour
                };
                let c = contour_json(&contour);
                // Pfad nachzeichnen: genau ein ausgewählter Pfad → nur den nachgezeichneten Abschnitt
                // ersetzen (PAR 09/14); beginnt der Zug abseits, wie bisher die ganze Kontur
                if self.id == ToolId::RedrawPath
                    && ctx.selection.elements.len() == 1
                    && let Some(e) =
                        docops::element_on_page(ctx.doc, ctx.page, ctx.selection.elements[0])
                    && let ElementKind::Path(pe) = &e.kind
                {
                    let tol = (ctx.pick_tolerance() * 1.5).max(3.0);
                    if let (Some(a), Some(b)) = (pts.first(), pts.last())
                        && let Some(cs) = redraw_section(&pe.contours, &contour, *a, *b, tol)
                    {
                        return ToolResponse::commit(vec![Invocation::new(
                            "path.setContours",
                            json!({"id": e.id.0, "contours": contours_json(&cs)}),
                        )]);
                    }
                    return ToolResponse::commit(vec![Invocation::new(
                        "path.setContours",
                        json!({"contours": [c]}),
                    )]);
                }
                // Vektorpfad: Verlaufsschritt „Pinsel“ wie im Original (PAR 09/12)
                return ToolResponse::commit(vec![super::styled(
                    Invocation::new(
                        "draw.path",
                        json!({"contours": [c], "fill": false, "step": "brush"}),
                    ),
                    ctx.options,
                )]);
            }
            _ => {}
        }
        ToolResponse::with_overlay(self.overlay(), Cursor::Pen)
    }

    fn overlay(&self) -> Vec<Overlay> {
        if self.points.is_empty() {
            Vec::new()
        } else {
            vec![Overlay::Polyline {
                points: self.points.clone(),
                closed: false,
            }]
        }
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
        WebShapeTool {
            id,
            drag: DragTracker::new(),
            points: Vec::new(),
            hover: None,
            overlay: Vec::new(),
        }
    }

    fn is_polygon(&self) -> bool {
        matches!(self.id, ToolId::HotspotPolygon | ToolId::PolygonSlice)
    }

    fn command(&self) -> &'static str {
        if matches!(self.id, ToolId::Slice | ToolId::PolygonSlice) {
            "web.addSlice"
        } else {
            "web.addHotspot"
        }
    }

    fn finish_polygon(&mut self) -> ToolResponse {
        let pts = std::mem::take(&mut self.points);
        self.overlay.clear();
        self.hover = None;
        if pts.len() < 3 {
            return ToolResponse::default();
        }
        ToolResponse::commit(vec![Invocation::new(
            self.command(),
            json!({"shape": "polygon", "points": pts.iter().map(|p| pt_json(*p)).collect::<Vec<_>>()}),
        )])
    }

    fn poly_overlay(&self) -> Vec<Overlay> {
        let mut pts = self.points.clone();
        if let Some(h) = self.hover {
            pts.push(h);
        }
        if pts.is_empty() {
            return Vec::new();
        }
        vec![
            Overlay::Polyline {
                points: pts,
                closed: true,
            },
            Overlay::Handles {
                points: self.points.clone(),
            },
        ]
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
                ToolInput::Pointer {
                    kind: PointerKind::Down,
                    pos,
                } => {
                    let p = Snapper::new(ctx, &[]).point(*pos).point;
                    if self.points.len() >= 3
                        && dist(self.points[0], p) <= ctx.pick_tolerance() as f32
                    {
                        return self.finish_polygon();
                    }
                    if self
                        .points
                        .last()
                        .is_none_or(|l| dist(*l, p) > ctx.pick_tolerance() as f32)
                    {
                        self.points.push(p);
                    }
                    self.overlay = self.poly_overlay();
                    ToolResponse::with_overlay(self.overlay.clone(), Cursor::Crosshair)
                }
                ToolInput::Pointer {
                    kind: PointerKind::Move,
                    pos,
                } => {
                    if !self.points.is_empty() {
                        self.hover = Some(Snapper::new(ctx, &[]).point(*pos).point);
                        self.overlay = self.poly_overlay();
                    }
                    ToolResponse::with_overlay(self.overlay.clone(), Cursor::Crosshair)
                }
                ToolInput::Pointer {
                    kind: PointerKind::DoubleClick,
                    ..
                }
                | ToolInput::Key(ToolKey::Enter) => self.finish_polygon(),
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
                    let shape = if self.id == ToolId::HotspotCircle {
                        "circle"
                    } else {
                        "rect"
                    };
                    ToolResponse::commit(vec![Invocation::new(
                        self.command(),
                        json!({"shape": shape, "rect": rect_json(r)}),
                    )])
                }
                None => ToolResponse::default(),
            };
        }
        if let Some(r) = r {
            self.overlay = vec![
                if self.id == ToolId::HotspotCircle {
                    Overlay::Ellipse {
                        rect: r,
                        dashed: false,
                    }
                } else {
                    Overlay::Rect {
                        rect: r,
                        dashed: false,
                    }
                },
                size_label(r),
            ];
            self.overlay.extend(sp.guides);
        }
        ToolResponse::with_overlay(self.overlay.clone(), Cursor::Crosshair)
    }

    fn finish(&mut self, _ctx: &ToolContext) -> ToolResponse {
        if self.is_polygon() {
            self.finish_polygon()
        } else {
            ToolResponse::default()
        }
    }

    fn overlay(&self) -> Vec<Overlay> {
        self.overlay.clone()
    }
}

/// Zugrahmen einer automatischen Form: Umschalt quadriert (längere Seite, Zugrichtung; TW-88),
/// außer bei der Pfeillinie – sie rastet die Richtung in 45°-Schritten ein (M10, re 43 §2):
/// waagerecht/senkrecht mit der Länge der Hauptachse, bei 45° max(|dx|, |dy|) für beide Achsen.
pub(crate) fn autoshape_rect(
    shape: &str,
    start: Point,
    cur: Point,
    shift: bool,
    alt: bool,
) -> Rect {
    if shift && shape == "Pfeillinie" {
        return constrain_rect(start, arrow_line_snap(start, cur), false, alt);
    }
    constrain_rect(start, cur, shift, alt)
}

/// Endpunkt der Pfeillinie mit Umschalt (45°-Raster ohne Quadrieren, M10 re 43 §2).
pub(crate) fn arrow_line_snap(start: Point, cur: Point) -> Point {
    let (dx, dy) = ((cur.x - start.x) as f64, (cur.y - start.y) as f64);
    if dx == 0.0 && dy == 0.0 {
        return cur;
    }
    let k = (dy.atan2(dx) / std::f64::consts::FRAC_PI_4).round() * std::f64::consts::FRAC_PI_4;
    let (ux, uy) = (k.cos().round(), k.sin().round());
    if uy == 0.0 {
        return Point::new(cur.x, start.y);
    }
    if ux == 0.0 {
        return Point::new(start.x, cur.y);
    }
    let m = dx.abs().max(dy.abs());
    Point::new(
        (start.x as f64 + ux * m) as f32,
        (start.y as f64 + uy * m) as f32,
    )
}

#[cfg(test)]
mod t11_tests {
    use super::*;

    /// M10 (re 43 §2): Pfeillinie mit Umschalt – Messfälle des Originals.
    #[test]
    fn t11_arrow_line_shift_snaps_45_without_squaring() {
        let p = |x: f32, y: f32| Point::new(x, y);
        assert_eq!(
            arrow_line_snap(p(300.0, 300.0), p(500.0, 360.0)),
            p(500.0, 300.0)
        );
        assert_eq!(
            arrow_line_snap(p(400.0, 50.0), p(440.0, 190.0)),
            p(400.0, 190.0)
        );
        assert_eq!(
            arrow_line_snap(p(50.0, 50.0), p(150.0, 130.0)),
            p(150.0, 150.0)
        );
        assert_eq!(
            arrow_line_snap(p(50.0, 200.0), p(150.0, 230.0)),
            p(150.0, 200.0)
        );
        // L-Form quadriert weiter auf die längere Seite
        let r = autoshape_rect("L-Form", p(50.0, 330.0), p(250.0, 430.0), true, false);
        assert_eq!((r.width(), r.height()), (200.0, 200.0));
        let r = autoshape_rect("Pfeillinie", p(300.0, 300.0), p(500.0, 360.0), true, false);
        assert_eq!((r.width(), r.height()), (200.0, 0.0));
    }
}
