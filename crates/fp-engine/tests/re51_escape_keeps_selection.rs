//! Escape on the canvas never clears the object selection (re 51 §1). It only commits an open text
//! editor, cancels Crop/Transform or leaves bitmap edit mode.

use crate::common;
use common::*;
use fp_engine::tools::ToolId;
use fp_engine::{Session, ToolEvent, ToolKey};
use fp_model::{Document, ElementId};
use serde_json::json;

fn tool(s: &mut Session, t: ToolId) {
    s.set_tool(t).unwrap_or_else(|e| panic!("{e}"));
}

fn doc(s: &Session) -> Document {
    (*s.active().expect("document").doc).clone()
}

fn esc(s: &mut Session) {
    events(s, &[ToolEvent::key(ToolKey::Escape)]);
}

fn sel(s: &Session) -> Vec<ElementId> {
    s.active().map(|d| d.selection.elements.clone()).unwrap_or_default()
}

fn rows(s: &Session) -> usize {
    s.active().map(|d| d.history.len()).unwrap_or_default()
}

#[test]
fn escape_keeps_selection_with_selection_tools() {
    for t in [ToolId::Pointer, ToolId::Subselection, ToolId::Text, ToolId::Pen] {
        let mut s = doc_session(300.0, 200.0);
        let a = rect(&mut s, 10.0, 10.0, 50.0, 50.0);
        let b = rect(&mut s, 100.0, 10.0, 140.0, 50.0);
        select(&mut s, &[a, b]);
        tool(&mut s, t);
        let d0 = doc(&s);
        let h0 = rows(&s);
        esc(&mut s);
        esc(&mut s);
        assert_eq!(sel(&s), vec![a, b], "{t:?}: selection stays");
        assert_eq!(doc(&s), d0, "{t:?}: document unchanged");
        assert_eq!(rows(&s), h0, "{t:?}: no history row");
    }
}

#[test]
fn escape_after_pointer_rubber_band_keeps_selection() {
    let mut s = doc_session(300.0, 200.0);
    rect(&mut s, 10.0, 10.0, 50.0, 50.0);
    rect(&mut s, 100.0, 10.0, 140.0, 50.0);
    tool(&mut s, ToolId::Pointer);
    events(&mut s, &[ToolEvent::down(0.0, 0.0), ToolEvent::drag(80.0, 40.0), ToolEvent::drag(160.0, 70.0), ToolEvent::up(160.0, 70.0)]);
    assert_eq!(sel(&s).len(), 2);
    esc(&mut s);
    assert_eq!(sel(&s).len(), 2);
}

#[test]
fn escape_cancels_crop_without_applying() {
    let mut s = doc_session(300.0, 200.0);
    let a = rect(&mut s, 10.0, 10.0, 50.0, 50.0);
    select(&mut s, &[a]);
    tool(&mut s, ToolId::Crop);
    let d0 = doc(&s);
    let h0 = rows(&s);
    events(&mut s, &[ToolEvent::down(20.0, 20.0), ToolEvent::drag(70.0, 60.0), ToolEvent::drag(120.0, 90.0), ToolEvent::up(120.0, 90.0)]);
    esc(&mut s);
    assert_eq!(doc(&s), d0, "canvas unchanged");
    assert_eq!(rows(&s), h0);
    assert_eq!(s.tools.active, ToolId::Pointer);
    assert!(!s.tool_feedback().iter().any(|o| matches!(o, fp_engine::tools::Overlay::Shade { .. })), "crop frame gone");
}

#[test]
fn escape_cancels_free_transform_and_keeps_selection() {
    let mut s = doc_session(600.0, 400.0);
    let r1 = rect(&mut s, 50.0, 50.0, 150.0, 120.0);
    select(&mut s, &[r1]);
    ex(&mut s, "modify.transform.freeTransform", json!({}));
    assert_eq!(s.tools.active, ToolId::Scale);
    let d0 = doc(&s);
    esc(&mut s);
    assert_eq!(doc(&s), d0);
    assert_eq!(s.tools.active, ToolId::Pointer);
    assert_eq!(sel(&s), vec![r1]);
}

#[test]
fn escape_leaves_bitmap_mode_and_switches_bitmap_tool_to_pointer() {
    let mut s = doc_session(300.0, 200.0);
    let b = bitmap(&mut s, 20, 20, 40, 40, [255, 0, 0, 255]);
    select(&mut s, &[b]);
    ex(&mut s, "modify.editBitmap", json!({}));
    assert!(s.active().is_some_and(|d| d.bitmap_mode));
    tool(&mut s, ToolId::Brush);
    esc(&mut s);
    assert!(s.active().is_some_and(|d| !d.bitmap_mode), "bitmap mode left");
    assert_eq!(s.tools.active, ToolId::Pointer);
    assert_eq!(sel(&s), vec![b], "bitmap stays selected");
    // a second Escape with the pointer changes nothing
    esc(&mut s);
    assert_eq!(sel(&s), vec![b]);
}

#[test]
fn escape_commits_open_text_editor() {
    let mut s = doc_session(300.0, 200.0);
    tool(&mut s, ToolId::Text);
    events(&mut s, &[ToolEvent::down(10.0, 100.0), ToolEvent::up(10.0, 100.0), ToolEvent::text("Hallo")]);
    assert!(s.tool_is_editing_text());
    esc(&mut s);
    assert!(!s.tool_is_editing_text());
    assert_eq!(top_elements(&s).len(), 1, "text committed");
    assert_eq!(sel(&s).len(), 1, "text object stays selected");
}
