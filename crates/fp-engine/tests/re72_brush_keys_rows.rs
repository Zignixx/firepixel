//! RE spec 72 rests (F124): B toggles the Pencil and the Brush, Y is the Knife only with a vector
//! selected (§1.2); a Brush/Pencil stroke that makes a new bitmap writes "Insert Empty Bitmap"
//! before "Edit Bitmap" and the stroke row (RE spec 71 §1.3, §2.2); a direct switch between the
//! Brush and the Pencil keeps the stroke and the chosen tool remembers it (§1.4).

use crate::common;
use common::*;
use fp_engine::tools::ToolId;
use fp_engine::{Session, Shortcut, ToolEvent};
use serde_json::json;

fn key(c: u8) -> Shortcut {
    Shortcut::key(c as u16)
}

fn press(s: &mut Session, c: u8) -> Option<ToolId> {
    s.tool_shortcut(&key(c)).ok().flatten()
}

fn labels(s: &Session) -> Vec<String> {
    s.active().map(|d| d.history.labels()).unwrap_or_default()
}

fn selection(s: &Session) -> Vec<fp_model::ElementId> {
    s.active().expect("doc").selection.elements.clone()
}

fn elements(s: &Session) -> usize {
    top_elements(s).len()
}

/// The default set: B selects the Brush first (neither used yet), each further press toggles;
/// from another tool B selects the one used last. E = Eraser, M toggles the marquees.
#[test]
fn re72_b_toggles_pencil_and_brush() {
    let mut s = doc_session(400.0, 300.0);
    assert_eq!(press(&mut s, b'B'), Some(ToolId::Brush));
    assert_eq!(press(&mut s, b'B'), Some(ToolId::Pencil));
    assert_eq!(press(&mut s, b'B'), Some(ToolId::Brush));
    assert_eq!(press(&mut s, b'B'), Some(ToolId::Pencil));
    s.set_tool(ToolId::Pointer).expect("Pointer");
    assert_eq!(
        press(&mut s, b'B'),
        Some(ToolId::Pencil),
        "the first press selects the one used last"
    );
    assert_eq!(press(&mut s, b'E'), Some(ToolId::Eraser));
    assert_eq!(press(&mut s, b'M'), Some(ToolId::Marquee));
    assert_eq!(press(&mut s, b'M'), Some(ToolId::OvalMarquee));
    assert_eq!(press(&mut s, b'M'), Some(ToolId::Marquee));
}

/// Y never selects the Pencil: with a vector selected it is the Knife, otherwise nothing happens.
#[test]
fn re72_y_is_the_knife_only_with_a_vector() {
    let mut s = doc_session(400.0, 300.0);
    s.set_tool(ToolId::Pointer).expect("Pointer");
    assert_eq!(press(&mut s, b'Y'), None);
    assert_eq!(s.tools.active, ToolId::Pointer);
    let b = bitmap(&mut s, 160, 120, 80, 60, [255, 255, 255, 255]);
    select(&mut s, &[b]);
    assert_eq!(press(&mut s, b'Y'), None, "a bitmap is no vector");
    assert_eq!(s.tools.active, ToolId::Pointer);
    let r = rect(&mut s, 20.0, 20.0, 100.0, 80.0);
    select(&mut s, &[r]);
    assert_eq!(press(&mut s, b'Y'), Some(ToolId::Knife));
    assert_eq!(s.tools.active, ToolId::Knife);
    assert_eq!(selection(&s), vec![r], "the selection stays");
}

fn stroke(s: &mut Session) {
    events(
        s,
        &[
            ToolEvent::down(40.0, 40.0),
            ToolEvent::drag(65.0, 40.0),
            ToolEvent::drag(90.0, 40.0),
            ToolEvent::up(90.0, 40.0),
        ],
    );
}

/// Does the selected bitmap have a visible pixel?
fn painted(s: &Session) -> bool {
    let d = s.active().expect("doc");
    let id = d.selection.elements[0];
    let p = fp_engine::docops::find_on_page(&d.doc, d.page, id).expect("bitmap");
    match &d.doc.element_at(&p).expect("element").kind {
        fp_model::ElementKind::Bitmap(b) => fp_engine::docops::bitmap_image(b)
            .expect("image")
            .data()
            .iter()
            .any(|px| px[3] > 0),
        _ => panic!("no bitmap"),
    }
}

/// A Brush or Pencil stroke on empty canvas with nothing selected: "Insert Empty Bitmap", "Edit
/// Bitmap", then the stroke row. Undo takes back the stroke (the empty bitmap stays, mode on),
/// then "Edit Bitmap" (mode off, the bitmap selected as an object), then the bitmap; Redo brings
/// all back.
#[test]
fn re72_new_bitmap_rows_and_undo() {
    for tool in [ToolId::Brush, ToolId::Pencil] {
        fp_i18n::set_language("en");
        let mut s = doc_session(400.0, 300.0);
        s.set_colors(Some(fp_color::Argb(0xff00_0000)), None);
        s.set_tool(tool).expect("tool");
        let n0 = labels(&s).len();
        let e0 = elements(&s);
        stroke(&mut s);
        let row = if tool == ToolId::Brush {
            "Brush"
        } else {
            "Pencil"
        };
        assert_eq!(
            labels(&s)[n0..],
            ["Insert Empty Bitmap", "Edit Bitmap", row],
            "{tool:?}"
        );
        assert_eq!(elements(&s), e0 + 1);
        assert!(painted(&s));
        s.execute("edit.undo", json!({})).expect("undo");
        assert!(
            s.active().expect("doc").bitmap_mode,
            "undoing the stroke keeps bitmap mode"
        );
        assert_eq!(elements(&s), e0 + 1, "the empty bitmap stays");
        assert!(!painted(&s), "the stroke is gone");
        s.execute("edit.undo", json!({})).expect("undo");
        assert!(
            !s.active().expect("doc").bitmap_mode,
            "undoing Edit Bitmap ends the mode"
        );
        assert_eq!(selection(&s).len(), 1, "the empty bitmap is selected");
        s.execute("edit.undo", json!({})).expect("undo");
        assert_eq!(elements(&s), e0, "the new bitmap is gone");
        for _ in 0..3 {
            s.execute("edit.redo", json!({})).expect("redo");
        }
        assert_eq!(elements(&s), e0 + 1);
        assert!(painted(&s));
        assert!(s.active().expect("doc").bitmap_mode);
    }
}

/// A further stroke into the new bitmap writes no second "Insert Empty Bitmap".
#[test]
fn re72_no_insert_row_without_new_bitmap() {
    fp_i18n::set_language("en");
    let mut s = doc_session(400.0, 300.0);
    s.set_colors(Some(fp_color::Argb(0xff00_0000)), None);
    s.set_tool(ToolId::Brush).expect("Brush");
    stroke(&mut s);
    let n = labels(&s).len();
    stroke(&mut s);
    assert_eq!(labels(&s)[n..], ["Brush"]);
}

/// Measured sequence h8 (fresh program, nothing selected): Pencil -> "Brush Stroke", Pointer,
/// Brush -> "Brush Stroke", directly Pencil -> "Stroke Color" (the Brush stroke stays), directly
/// Brush -> no new row, Pointer, Pencil -> no new row (the Pencil remembers the Brush stroke).
#[test]
fn re72_direct_switch_keeps_the_stroke() {
    fp_i18n::set_language("en");
    let mut s = doc_session(400.0, 300.0);
    let n0 = labels(&s).len();
    let tail = |s: &Session| labels(s)[n0..].to_vec();
    s.set_tool(ToolId::Pencil).expect("Pencil");
    assert_eq!(tail(&s), ["Brush Stroke"]);
    s.set_tool(ToolId::Pointer).expect("Pointer");
    s.set_tool(ToolId::Brush).expect("Brush");
    assert_eq!(tail(&s), ["Brush Stroke", "Brush Stroke"]);
    s.set_tool(ToolId::Pencil).expect("Pencil");
    assert_eq!(tail(&s), ["Brush Stroke", "Brush Stroke", "Stroke Color"]);
    s.set_tool(ToolId::Brush).expect("Brush");
    assert_eq!(tail(&s).len(), 3, "no second Stroke Color");
    s.set_tool(ToolId::Pointer).expect("Pointer");
    s.set_tool(ToolId::Pencil).expect("Pencil");
    assert_eq!(tail(&s).len(), 3, "the Pencil remembers the Brush stroke");
}

/// The same with the key: B, B from the Pointer is a direct switch, so it writes no second
/// "Brush Stroke".
#[test]
fn re72_direct_switch_by_key() {
    fp_i18n::set_language("en");
    let mut s = doc_session(400.0, 300.0);
    let n0 = labels(&s).len();
    assert_eq!(press(&mut s, b'B'), Some(ToolId::Brush));
    assert_eq!(press(&mut s, b'B'), Some(ToolId::Pencil));
    assert_eq!(labels(&s)[n0..], ["Stroke Color"]);
}
