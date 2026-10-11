//! Hand und Zoom (10 §2.6): Hand verschiebt die Ansicht; Zoom = Klick vergrößert, Alt+Klick
//! verkleinert, Ziehen zoomt auf den Bereich.

use super::{dist, pt_json, rect_json, Cursor, Invocation, Overlay, PointerKind, ToolContext, ToolEvent, ToolId, ToolInput, ToolMachine, ToolResponse};
use fp_geom::{Point, Rect};
use serde_json::json;

/// Hand: Ziehen verschiebt die Ansicht (Befehl `view.pan`, Δ in Bildschirmpixeln).
#[derive(Clone, Debug, Default)]
pub struct HandTool {
    last: Option<Point>,
}

impl ToolMachine for HandTool {
    fn id(&self) -> ToolId {
        ToolId::Hand
    }

    fn handle(&mut self, ev: &ToolEvent, ctx: &ToolContext) -> ToolResponse {
        let ToolInput::Pointer { kind, pos } = &ev.input else { return ToolResponse::default() };
        match kind {
            PointerKind::Down => {
                self.last = Some(*pos);
                ToolResponse { cursor: Cursor::Grabbing, ..ToolResponse::default() }
            }
            PointerKind::Move | PointerKind::Up => {
                let Some(l) = self.last else { return ToolResponse { cursor: Cursor::Hand, ..ToolResponse::default() } };
                // Der gegriffene Dokumentpunkt bleibt unter dem Zeiger: nach jedem Verschieben liegt
                // die neue Zeigerposition (Dokumentkoordinaten) wieder auf dem Griffpunkt, Δ = Rest.
                let z = ctx.zoom as f32;
                let (dx, dy) = ((pos.x - l.x) * z, (pos.y - l.y) * z);
                if *kind == PointerKind::Up {
                    self.last = None;
                }
                if dx == 0.0 && dy == 0.0 {
                    return ToolResponse { cursor: Cursor::Grabbing, ..ToolResponse::default() };
                }
                ToolResponse { commands: vec![Invocation::new("view.pan", json!({"dx": dx, "dy": dy}))], cursor: Cursor::Grabbing, ..ToolResponse::default() }
            }
            PointerKind::DoubleClick => ToolResponse::commit(vec![Invocation::new("view.fitAll", json!({}))]),
        }
    }
}

/// Zoom.
#[derive(Clone, Debug, Default)]
pub struct ZoomTool {
    start: Option<Point>,
    cur: Option<Point>,
}

impl ToolMachine for ZoomTool {
    fn id(&self) -> ToolId {
        ToolId::Zoom
    }

    fn handle(&mut self, ev: &ToolEvent, ctx: &ToolContext) -> ToolResponse {
        let ToolInput::Pointer { kind, pos } = &ev.input else { return ToolResponse::default() };
        let cursor = if ev.mods.alt { Cursor::ZoomOut } else { Cursor::ZoomIn };
        match kind {
            PointerKind::Down => {
                self.start = Some(*pos);
                self.cur = None;
                ToolResponse { cursor, ..ToolResponse::default() }
            }
            PointerKind::Move => {
                // Leertaste beim Aufziehen: Zoomrahmen verschieben
                if ev.mods.space
                    && let (Some(s), Some(c)) = (self.start, self.cur)
                {
                    self.start = Some(Point::new(s.x + pos.x - c.x, s.y + pos.y - c.y));
                }
                if let Some(s) = self.start
                    && (self.cur.is_some() || dist(s, *pos) >= ctx.drag_threshold())
                {
                    self.cur = Some(*pos);
                }
                ToolResponse::with_overlay(self.overlay(), cursor)
            }
            PointerKind::Up => {
                let Some(s) = self.start.take() else { return ToolResponse::default() };
                let dragged = self.cur.take().is_some();
                if dragged && !ev.mods.alt {
                    let r = Rect::new(s.x, s.y, pos.x, pos.y).normalized();
                    return ToolResponse::commit(vec![Invocation::new("view.zoomToRect", json!({"rect": rect_json(r)}))]);
                }
                let name = if ev.mods.alt { "view.zoomOut" } else { "view.zoomIn" };
                ToolResponse::commit(vec![Invocation::new(name, json!({"at": pt_json(*pos)}))])
            }
            PointerKind::DoubleClick => ToolResponse::default(),
        }
    }

    fn overlay(&self) -> Vec<Overlay> {
        match (self.start, self.cur) {
            (Some(a), Some(b)) => vec![Overlay::Rect { rect: Rect::new(a.x, a.y, b.x, b.y).normalized(), dashed: true }],
            _ => Vec::new(),
        }
    }
}
