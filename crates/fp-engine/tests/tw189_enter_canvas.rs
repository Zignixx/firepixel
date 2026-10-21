//! TW-189 (run 31, `xs-shortcuts/22_enter_canvas`, measured with the real Enter key): Enter on the
//! canvas with the Pointer tool idle changes nothing. A selected group stays selected, a single
//! selected object stays selected, and no History row is added.

use crate::common::{bitmap, doc_session, events, ex, rect, select};
use fp_engine::tools::ToolId;
use fp_engine::{Session, ToolEvent, ToolKey};
use fp_model::ElementId;
use serde_json::json;

fn selection(s: &Session) -> Vec<ElementId> {
    s.active().map(|d| d.selection.elements.clone()).unwrap_or_default()
}

fn undo_count(s: &Session) -> usize {
    s.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

fn press_enter_and_check(s: &mut Session, expected: &[ElementId]) {
    s.set_tool(ToolId::Pointer).expect("pointer");
    assert_eq!(selection(s), expected, "selection before Enter");
    let rows = undo_count(s);
    let r = s.tool_event(&ToolEvent::key(ToolKey::Enter)).expect("Enter");
    assert!(r.commands.is_empty(), "Enter runs no command: {:?}", r.commands);
    assert_eq!(selection(s), expected, "Enter keeps the selection");
    assert_eq!(undo_count(s), rows, "Enter adds no History row");
}

#[test]
fn enter_on_canvas_keeps_selected_group() {
    let mut s = doc_session(300.0, 200.0);
    let a = rect(&mut s, 10.0, 10.0, 60.0, 60.0);
    let b = rect(&mut s, 80.0, 10.0, 140.0, 60.0);
    select(&mut s, &[a, b]);
    ex(&mut s, "modify.group", json!({}));
    let g = selection(&s);
    assert_eq!(g.len(), 1, "one group selected");
    press_enter_and_check(&mut s, &g);
    // a second Enter changes nothing either
    events(&mut s, &[ToolEvent::key(ToolKey::Enter)]);
    assert_eq!(selection(&s), g);
}

#[test]
fn enter_on_canvas_keeps_single_selected_object() {
    let mut s = doc_session(300.0, 200.0);
    let a = rect(&mut s, 10.0, 10.0, 60.0, 60.0);
    select(&mut s, &[a]);
    press_enter_and_check(&mut s, &[a]);
}

#[test]
fn enter_on_canvas_keeps_two_selected_objects_and_bitmap() {
    let mut s = doc_session(300.0, 200.0);
    let a = rect(&mut s, 10.0, 10.0, 60.0, 60.0);
    let b = rect(&mut s, 80.0, 10.0, 140.0, 60.0);
    select(&mut s, &[a, b]);
    press_enter_and_check(&mut s, &[a, b]);
    let bm = bitmap(&mut s, 150, 100, 40, 30, [200, 0, 0, 255]);
    select(&mut s, &[bm]);
    press_enter_and_check(&mut s, &[bm]);
}

#[test]
fn enter_on_canvas_without_selection_does_nothing() {
    let mut s = doc_session(300.0, 200.0);
    rect(&mut s, 10.0, 10.0, 60.0, 60.0);
    ex(&mut s, "select.none", json!({}));
    press_enter_and_check(&mut s, &[]);
}
