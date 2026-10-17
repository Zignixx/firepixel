//! CF-FP4 (`docs\funktionstest\fp4_2026-10-17.md`): Doppelklick auf eine Symbolinstanz bearbeitet
//! an Position (FP4-03, re 28 S2), Füllwerkzeug zeigt beim Griffzug die Vorschaulinie zum Zeiger
//! (FP4-01, K-6/T-9, re 40 §1).

use crate::common;
use common::*;
use fp_engine::tools::{Overlay, ToolId};
use fp_engine::ToolEvent;
use fp_geom::Rect;
use serde_json::json;

/// FP4-03: Rechteck 0,0–50,50 → Symbol, Instanz nach 200,150; Doppelklick mit dem Zeiger öffnet
/// „An Position bearbeiten“ (Inhalt an der Instanzlage), keine Platzhaltermeldung.
#[test]
fn fp4_03_double_click_instance_edits_in_place() {
    let mut s = doc_session(400.0, 300.0);
    rect(&mut s, 300.0, 0.0, 350.0, 50.0);
    let b = rect(&mut s, 0.0, 0.0, 50.0, 50.0);
    select(&mut s, &[b]);
    ex(&mut s, "modify.symbol.convertToSymbol", json!({"name": "FP4"}));
    ex(&mut s, "edit.move", json!({"dx": 200, "dy": 150}));
    let inst = s.active().expect("d").selection.elements[0];
    assert_eq!(bounds(&s, inst), Rect::new(200.0, 150.0, 250.0, 200.0));
    let parent = s.active_id().expect("Dokument");
    ex(&mut s, "select.set", json!({"ids": [], "mode": "replace"}));
    s.set_tool(ToolId::Pointer).expect("Zeiger");
    events(&mut s, &[ToolEvent::down(225.0, 175.0), ToolEvent::up(225.0, 175.0), ToolEvent::down(225.0, 175.0), ToolEvent::up(225.0, 175.0)]);
    let r = s.tool_event(&ToolEvent::double(225.0, 175.0)).expect("Doppelklick");
    assert!(r.status.is_none(), "keine Platzhaltermeldung: {:?}", r.status);
    assert_ne!(s.active_id(), Some(parent), "Symbolbearbeitung geöffnet");
    let d = s.active().expect("Symbol");
    let link = d.symbol_edit.as_ref().expect("Symbolbearbeitung");
    assert!(link.in_place, "an Position");
    assert_eq!(link.instance, Some(inst));
    let c = &d.doc.pages[d.page].canvas;
    let cb = bounds(&s, top_elements(&s)[0].id);
    assert_eq!((cb.left - c.origin.x, cb.top - c.origin.y), (200.0, 150.0), "Inhalt an der Instanzlage");
    ex(&mut s, "modify.symbol.exitEdit", json!({}));
    assert_eq!(s.active_id(), Some(parent));
    assert_eq!(bounds(&s, inst), Rect::new(200.0, 150.0, 250.0, 200.0), "Instanz unverändert");
}

/// FP4-01: Füllwerkzeug, Endgriff ziehen: alte Linie bleibt, dazu die Vorschaulinie vom festen
/// Anfangsgriff zum Zeiger; Füllung erst beim Loslassen.
#[test]
fn fp4_01_fill_tool_handle_drag_shows_preview_line_to_pointer() {
    let mut s = doc_session(400.0, 300.0);
    let r = rect(&mut s, 50.0, 50.0, 150.0, 120.0);
    select(&mut s, &[r]);
    ex(&mut s, "element.setGradient", json!({"from": [60, 85], "to": [140, 100]}));
    s.set_tool(ToolId::Gradient).expect("Verlauf");
    events(&mut s, &[ToolEvent::down(140.0, 100.0)]);
    let lines = |s: &mut fp_engine::Session| -> Vec<((f32, f32), (f32, f32))> {
        s.tool_feedback()
            .iter()
            .filter_map(|o| match o {
                Overlay::GradientHandles { start, end } => Some(((start.x, start.y), (end.x, end.y))),
                _ => None,
            })
            .collect()
    };
    assert_eq!(lines(&mut s), vec![((60.0, 85.0), (140.0, 100.0))], "vor dem Zug nur die alte Linie");
    events(&mut s, &[ToolEvent::drag(135.0, 85.0), ToolEvent::drag(130.0, 70.0)]);
    let l = lines(&mut s);
    assert_eq!(l.len(), 2, "alte Linie und Vorschaulinie: {l:?}");
    assert_eq!(l[0], ((60.0, 85.0), (140.0, 100.0)), "alte Linie bleibt");
    assert_eq!(l[1], ((60.0, 85.0), (130.0, 70.0)), "Vorschaulinie vom festen Griff zum Zeiger");
    events(&mut s, &[ToolEvent::up(130.0, 70.0)]);
    assert_eq!(lines(&mut s), vec![((60.0, 85.0), (130.0, 70.0))], "nach dem Loslassen die neuen Griffe");
}
