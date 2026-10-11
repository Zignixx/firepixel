//! KI-Auswahlwerkzeuge (Flyout des Zauberstabs): Objektauswahl, Motiv auswählen, Kante verfeinern.
//!
//! Die Werkzeuge rechnen nicht selbst. Sie übersetzen Klicks und Modifikatoren in einen Aufruf von
//! `select.aiObject` / `select.aiSubject` / `select.aiRefineEdge` und reichen ihn als
//! [`ToolResponse::ui_request`] an die Oberfläche: Die rechnet das Bild-Encoding bzw. die
//! Freistellung im Hintergrund (mit Fortschritt und Abbrechen) und führt danach den Befehl aus. Ohne
//! Oberfläche (MCP, CLI) benutzen Agenten die Befehle direkt.
//!
//! Bedienung wie der klassische Zauberstab (`WandTool`): Klick = neue Auswahl (ersetzt die alte),
//! Umschalt+Klick = hinzufügen, Alt+Klick = abziehen, Umschalt+Alt+Klick = Schnittmenge; jeder Klick
//! wählt das angeklickte Objekt bzw. Motiv unabhängig vom vorigen. Esc und Eingabe gehen wie beim
//! Zauberstab an die Menükürzel (das Werkzeug selbst hat keinen offenen Vorgang). Zusätzlich kann
//! bei der Objektauswahl ein Rahmen um das Objekt aufgezogen werden (gleiche Modifikatoren).

use super::{dist, Cursor, Invocation, Overlay, ToolContext, ToolEvent, ToolId, ToolInput, ToolKey, ToolMachine, ToolResponse};
use crate::selection::SelectMode;
use fp_geom::{Point, Rect};
use serde_json::{json, Value};

fn view_rect(ctx: &ToolContext) -> Value {
    let r = ctx.view.visible_rect();
    json!([r.left, r.top, r.right, r.bottom])
}

fn request(cmd: &str, params: Value) -> ToolResponse {
    ToolResponse { ui_request: Some(Invocation::new(cmd, params)), cursor: Cursor::Wand, ..ToolResponse::default() }
}

/// Verknüpfung wie beim Zauberstab: Umschalt = hinzufügen, Alt = abziehen, beide = Schnittmenge.
fn mode_from(ev: &ToolEvent) -> SelectMode {
    match (ev.mods.shift, ev.mods.alt) {
        (true, true) => SelectMode::Intersect,
        (true, false) => SelectMode::Add,
        (false, true) => SelectMode::Subtract,
        _ => SelectMode::Replace,
    }
}

/// Objektauswahl (SAM 2.1).
#[derive(Default)]
pub struct AiObjectTool {
    down: Option<Point>,
    cur: Point,
    dragging: bool,
    /// Verknüpfung beim Mausdruck (wie der Zauberstab, der beim Drücken auslöst).
    mode: SelectMode,
}

impl AiObjectTool {
    fn box_rect(&self) -> Option<Rect> {
        self.down.map(|d| Rect::new(d.x, d.y, self.cur.x, self.cur.y).normalized())
    }
}

impl ToolMachine for AiObjectTool {
    fn id(&self) -> ToolId {
        ToolId::AiObject
    }

    fn handle(&mut self, ev: &ToolEvent, ctx: &ToolContext) -> ToolResponse {
        match &ev.input {
            ToolInput::Pointer { kind, pos } => match kind {
                super::PointerKind::Down => {
                    self.down = Some(*pos);
                    self.cur = *pos;
                    self.dragging = false;
                    self.mode = mode_from(ev);
                    ToolResponse::with_overlay(Vec::new(), Cursor::Wand)
                }
                super::PointerKind::Move => {
                    self.cur = *pos;
                    if let Some(d) = self.down {
                        if !self.dragging && dist(d, *pos) >= ctx.drag_threshold() {
                            self.dragging = true;
                        }
                        if self.dragging {
                            let r = self.box_rect().unwrap_or_default();
                            return ToolResponse::with_overlay(vec![Overlay::Rect { rect: r, dashed: true }], Cursor::Crosshair);
                        }
                    }
                    ToolResponse::with_overlay(Vec::new(), Cursor::Wand)
                }
                super::PointerKind::Up | super::PointerKind::DoubleClick => {
                    let Some(start) = self.down.take() else { return ToolResponse::default() };
                    self.cur = *pos;
                    if !ctx.has_document {
                        return ToolResponse::default();
                    }
                    // jeder Klick ist eine eigene Objektauswahl (keine Fortsetzung des vorigen Objekts)
                    let mut p = json!({"mode": self.mode.name(), "view": view_rect(ctx)});
                    if self.dragging {
                        let r = Rect::new(start.x, start.y, pos.x, pos.y).normalized();
                        p["box"] = json!([r.left, r.top, r.right, r.bottom]);
                    } else {
                        p["points"] = json!([[pos.x, pos.y, 1]]);
                    }
                    self.dragging = false;
                    request("select.aiObject", p)
                }
            },
            // Esc nur während des Aufziehens (Rahmen verwerfen); sonst wie beim Zauberstab an die Kürzel
            ToolInput::Key(ToolKey::Escape) if self.down.is_some() => {
                self.down = None;
                self.dragging = false;
                ToolResponse::with_overlay(Vec::new(), Cursor::Wand)
            }
            _ => ToolResponse::default(),
        }
    }

    fn finish(&mut self, _ctx: &ToolContext) -> ToolResponse {
        self.down = None;
        self.dragging = false;
        ToolResponse::default()
    }

    fn overlay(&self) -> Vec<Overlay> {
        if self.dragging {
            self.box_rect().map(|r| vec![Overlay::Rect { rect: r, dashed: true }]).unwrap_or_default()
        } else {
            Vec::new()
        }
    }
}

/// Motiv auswählen (BiRefNet) bzw. Kante verfeinern (BiRefNet-Matting): ein Klick genügt.
pub struct AiClickTool {
    id: ToolId,
}

impl AiClickTool {
    /// Werkzeug `AiSubject` oder `AiRefineEdge`.
    pub fn new(id: ToolId) -> AiClickTool {
        AiClickTool { id }
    }
}

impl ToolMachine for AiClickTool {
    fn id(&self) -> ToolId {
        self.id
    }

    fn handle(&mut self, ev: &ToolEvent, ctx: &ToolContext) -> ToolResponse {
        let ToolInput::Pointer { kind, pos } = &ev.input else { return ToolResponse::default() };
        match kind {
            // wie der Zauberstab: der Mausdruck löst aus
            super::PointerKind::Down if ctx.has_document => {
                if self.id == ToolId::AiSubject {
                    request("select.aiSubject", json!({"mode": mode_from(ev).name(), "point": [pos.x, pos.y]}))
                } else {
                    if !ctx.selection.has_pixels() {
                        return ToolResponse { status: Some("Kante verfeinern: zuerst eine Pixelauswahl erstellen (z. B. mit „Motiv auswählen“ oder „Objektauswahl“).".into()), cursor: Cursor::Wand, ..ToolResponse::default() };
                    }
                    request("select.aiRefineEdge", json!({}))
                }
            }
            _ => ToolResponse::with_overlay(Vec::new(), Cursor::Wand),
        }
    }
}
