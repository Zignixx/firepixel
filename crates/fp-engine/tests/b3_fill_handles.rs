//! B3 (BA-043): Verlaufs-/Mustergriffe lassen sich mit dem Zeiger auf der Leinwand ziehen – ein
//! Verlaufsschritt, nur der gezogene Griff bewegt sich; ein Klick auf den Griff ändert nichts, Ziehen
//! neben den Griffen verschiebt weiter das Objekt.

use crate::common;
use common::*;
use fp_engine::tools::ToolId;
use fp_engine::{Session, ToolEvent};
use fp_model::{ElementId, ElementKind, FillHandles};
use serde_json::json;

fn handles(s: &Session, id: ElementId) -> FillHandles {
    let ElementKind::Path(p) = &el(s, id).kind else { panic!("Pfad") };
    p.fill_handles
}

fn drag(s: &mut Session, a: (f32, f32), b: (f32, f32)) {
    events(s, &[ToolEvent::down(a.0, a.1)]);
    for i in 1..=10 {
        let t = i as f32 / 10.0;
        events(s, &[ToolEvent::drag(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)]);
    }
    events(s, &[ToolEvent::up(b.0, b.1)]);
}

fn undo_count(s: &Session) -> usize {
    s.active().expect("Dokument").history.undo_count()
}

fn gradient_rect(s: &mut Session) -> ElementId {
    let r = rect(s, 50.0, 50.0, 150.0, 120.0);
    select(s, &[r]);
    ex(s, "element.setGradient", json!({"from": [60, 85], "to": [140, 100]}));
    s.set_tool(ToolId::Pointer).expect("Zeiger");
    r
}

#[test]
fn pointer_drags_gradient_end_handle_in_one_step() {
    let mut s = doc_session(400.0, 300.0);
    let r = gradient_rect(&mut s);
    let h0 = handles(&s, r);
    let n0 = undo_count(&s);
    drag(&mut s, (140.0, 100.0), (130.0, 70.0));
    let h = handles(&s, r);
    assert_eq!(undo_count(&s), n0 + 1, "ein Schritt");
    assert_eq!((h.end.x, h.end.y), (130.0, 70.0), "Ende folgt dem Zeiger");
    assert_eq!((h.start, h.second), (h0.start, h0.second), "übrige Griffe bleiben");
    let b = bounds(&s, r);
    assert_eq!((b.left, b.top), (50.0, 50.0), "Objekt bleibt liegen");
    // Rückgängig: Griffe wie vorher
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(handles(&s, r).end, h0.end);
}

#[test]
fn pointer_drags_gradient_start_handle() {
    let mut s = doc_session(400.0, 300.0);
    let r = gradient_rect(&mut s);
    let h0 = handles(&s, r);
    drag(&mut s, (60.0, 85.0), (70.0, 95.0));
    let h = handles(&s, r);
    assert_eq!((h.start.x, h.start.y), (70.0, 95.0));
    assert_eq!(h.end, h0.end);
}

#[test]
fn click_on_handle_changes_nothing_and_drag_elsewhere_moves_object() {
    let mut s = doc_session(400.0, 300.0);
    let r = gradient_rect(&mut s);
    let h0 = handles(&s, r);
    let n0 = undo_count(&s);
    events(&mut s, &[ToolEvent::down(140.0, 100.0)]);
    events(&mut s, &[ToolEvent::up(140.0, 100.0)]);
    assert_eq!(undo_count(&s), n0, "Klick: kein Schritt");
    assert_eq!(handles(&s, r), h0);
    // Ziehen im Objekt abseits der Griffe verschiebt das Objekt samt Griffen
    drag(&mut s, (100.0, 60.0), (110.0, 60.0));
    let h = handles(&s, r);
    assert_eq!((h.start.x, h.end.x), (h0.start.x + 10.0, h0.end.x + 10.0));
}

#[test]
fn solid_fill_has_no_draggable_handles() {
    let mut s = doc_session(400.0, 300.0);
    let r = rect(&mut s, 50.0, 50.0, 150.0, 120.0);
    select(&mut s, &[r]);
    s.set_tool(ToolId::Pointer).expect("Zeiger");
    let h0 = handles(&s, r);
    drag(&mut s, (60.0, 85.0), (70.0, 95.0));
    // ohne Verlauf: normales Verschieben des Objekts
    assert_eq!(handles(&s, r).valid, h0.valid);
    let b = bounds(&s, r);
    assert_eq!((b.left, b.top), (60.0, 60.0));
}

/// T-9 (re 40 §1): Mit dem Zeiger ändert sich die Füllung während des Zugs live (Rückmeldung
/// `FillLive` mit den aktuellen Griffen, keine Vorschaulinie); mit dem Füllwerkzeug Vorschaulinie
/// und Füllung erst beim Loslassen. Beides ein Schritt mit demselben Ergebnis.
#[test]
fn t9_pointer_live_fill_tool_preview() {
    use fp_engine::tools::Overlay;
    let mut s = doc_session(400.0, 300.0);
    let r = gradient_rect(&mut s);
    let h0 = handles(&s, r);
    let n0 = undo_count(&s);
    events(&mut s, &[ToolEvent::down(140.0, 100.0)]);
    events(&mut s, &[ToolEvent::drag(135.0, 85.0)]);
    events(&mut s, &[ToolEvent::drag(130.0, 70.0)]);
    let fb = s.tool_feedback();
    let live = fb.iter().find_map(|o| match o {
        Overlay::FillLive { target, handles } => Some((*target, *handles)),
        _ => None,
    });
    let (t, h) = live.expect("Zeiger: Füllung live");
    assert_eq!(t, r.0);
    assert_eq!((h.end.x, h.end.y), (130.0, 70.0));
    assert!(!fb.iter().any(|o| matches!(o, Overlay::GradientHandles { .. })), "keine Vorschaulinie");
    assert_eq!(handles(&s, r), h0, "Dokument erst beim Loslassen");
    events(&mut s, &[ToolEvent::up(130.0, 70.0)]);
    assert_eq!(undo_count(&s), n0 + 1);
    let after_pointer = handles(&s, r);
    ex(&mut s, "edit.undo", json!({}));
    // Füllwerkzeug: Vorschaulinie, kein Live
    s.set_tool(ToolId::Gradient).expect("Verlauf");
    events(&mut s, &[ToolEvent::down(140.0, 100.0)]);
    events(&mut s, &[ToolEvent::drag(135.0, 85.0)]);
    events(&mut s, &[ToolEvent::drag(130.0, 70.0)]);
    let fb = s.tool_feedback();
    assert!(fb.iter().any(|o| matches!(o, Overlay::GradientHandles { .. })), "Vorschaulinie");
    assert!(!fb.iter().any(|o| matches!(o, Overlay::FillLive { .. })), "Füllung erst beim Loslassen");
    events(&mut s, &[ToolEvent::up(130.0, 70.0)]);
    assert_eq!(handles(&s, r), after_pointer, "gleiches Ergebnis");
    assert_eq!(undo_count(&s), n0 + 1);
}
