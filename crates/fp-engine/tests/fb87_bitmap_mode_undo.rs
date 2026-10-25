//! FB-87: Undo ends bitmap mode and Redo enters it again (TW-249), and the History rows of
//! choosing a bitmap tool (TW-265), both after RE spec 72 (§1, §2).

use crate::common;
use common::*;
use fp_engine::Session;
use fp_engine::ToolEvent;
use fp_engine::tools::ToolId;
use serde_json::json;

fn labels(s: &Session) -> Vec<String> {
    s.active().map(|d| d.history.labels()).unwrap_or_default()
}

fn done(s: &Session) -> usize {
    s.active().expect("doc").history.undo_count()
}

fn mode(s: &Session) -> bool {
    s.active().expect("doc").bitmap_mode
}

fn selection(s: &Session) -> Vec<fp_model::ElementId> {
    s.active().expect("doc").selection.elements.clone()
}

/// Document 400 x 300 with a white bitmap 160,120 80 x 60, selected; History emptied.
fn setup() -> (Session, fp_model::ElementId) {
    fp_i18n::set_language("en");
    let mut s = doc_session(400.0, 300.0);
    s.set_colors(Some(fp_color::Argb(0xff00_0000)), None);
    let b = bitmap(&mut s, 160, 120, 80, 60, [255, 255, 255, 255]);
    select(&mut s, &[b]);
    if let Some(d) = s.active_mut() {
        d.history.clear();
    }
    (s, b)
}

fn stroke(s: &mut Session) {
    events(
        s,
        &[
            ToolEvent::down(170.0, 140.0),
            ToolEvent::drag(190.0, 140.0),
            ToolEvent::drag(210.0, 140.0),
            ToolEvent::up(210.0, 140.0),
        ],
    );
}

fn is_stroke_row(l: &str) -> bool {
    matches!(l, "Brush Stroke" | "Stroke Color")
}

/// Choosing the Marquee with a bitmap selected writes "Edit Bitmap"; undoing it ends bitmap
/// mode, Redo enters it again.
#[test]
fn fb87_undo_edit_bitmap_row_ends_mode_redo_enters() {
    let (mut s, b) = setup();
    s.set_tool(ToolId::Marquee).expect("Marquee");
    assert!(mode(&s));
    assert_eq!(labels(&s), vec!["Edit Bitmap"]);
    s.undo().expect("undo");
    assert!(!mode(&s), "undo of Edit Bitmap ends bitmap mode");
    assert_eq!(selection(&s), vec![b], "bitmap selected as an object");
    s.redo().expect("redo");
    assert!(mode(&s), "redo enters bitmap mode again");
    assert_eq!(s.tools.active, ToolId::Marquee, "the tool never changes");
}

/// RE spec 72 §2.2 (u1): Brush chosen with the bitmap selected ("Edit Bitmap" + stroke row), a
/// stroke ("Brush"). Undo: the stroke goes, the mode stays; Undo: "Edit Bitmap" goes together with
/// its stroke row, the mode ends; Redo: both come back, the mode is on; Redo: the stroke.
#[test]
fn fb87_undo_stroke_rows_step_out_of_bitmap_mode() {
    let (mut s, b) = setup();
    s.set_tool(ToolId::Brush).expect("Brush");
    let l = labels(&s);
    assert_eq!(l.len(), 2, "{l:?}");
    assert_eq!(l[0], "Edit Bitmap");
    assert!(is_stroke_row(&l[1]), "{l:?}");
    stroke(&mut s);
    assert_eq!(labels(&s).last().map(String::as_str), Some("Brush"));
    assert_eq!(done(&s), 3);
    s.undo().expect("undo stroke");
    assert!(mode(&s), "mode stays after undoing the stroke");
    assert_eq!(done(&s), 2);
    assert_eq!(
        s.active().expect("doc").history.undo_label(),
        Some("Edit Bitmap"),
        "the Edit menu names the real step"
    );
    s.undo().expect("undo Edit Bitmap");
    assert_eq!(done(&s), 0, "the stroke row goes back with Edit Bitmap");
    assert!(!mode(&s), "mode ends with Edit Bitmap");
    assert_eq!(selection(&s), vec![b]);
    s.redo().expect("redo Edit Bitmap");
    assert_eq!(done(&s), 2, "the stroke row comes back with Edit Bitmap");
    assert!(mode(&s));
    s.redo().expect("redo stroke");
    assert_eq!(done(&s), 3);
    assert!(mode(&s));
    assert_eq!(s.tools.active, ToolId::Brush);
}

/// u3: nothing selected, Brush, stroke on the bitmap: "Edit Bitmap" goes in below the stroke;
/// Undo keeps the mode, the second Undo ends it.
#[test]
fn fb87_undo_after_stroke_on_unselected_bitmap() {
    let (mut s, b) = setup();
    select(&mut s, &[]);
    s.set_tool(ToolId::Brush).expect("Brush");
    select(&mut s, &[b]);
    stroke(&mut s);
    let l = labels(&s);
    assert_eq!(l[l.len() - 2..], ["Edit Bitmap", "Brush"]);
    s.undo().expect("undo stroke");
    assert!(mode(&s));
    s.undo().expect("undo Edit Bitmap");
    assert!(!mode(&s));
    s.redo().expect("redo");
    assert!(mode(&s));
}

/// "Exit Bitmap Mode" undone: bitmap mode is back; redone: off again; the tool stays.
#[test]
fn fb87_undo_exit_bitmap_mode_reenters() {
    let (mut s, _) = setup();
    s.set_tool(ToolId::Brush).expect("Brush");
    assert!(mode(&s));
    s.execute("modify.exitBitmapMode", json!({"row": true}))
        .expect("exit");
    assert!(!mode(&s));
    assert_eq!(
        labels(&s).last().map(String::as_str),
        Some("Exit Bitmap Mode")
    );
    s.undo().expect("undo");
    assert!(mode(&s), "undo of Exit Bitmap Mode re-enters the mode");
    s.redo().expect("redo");
    assert!(!mode(&s));
    assert_eq!(s.tools.active, ToolId::Brush);
}

fn brush_of(s: &Session, id: fp_model::ElementId) -> Option<fp_model::Brush> {
    let d = s.active().expect("doc");
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, id).expect("element");
    fp_engine::docops::path_attrs_of(&e).and_then(|a| a.brush.clone())
}

/// RE spec 72 §1.2: a bitmap tool chosen with a vector selected clears the selection and leaves
/// the vector unchanged; Brush and Pencil write one row ("Brush Stroke" for a vector without a
/// stroke), Eraser and Marquee none. The row goes back with the step before it.
#[test]
fn fb87_bitmap_tool_with_vector_deselects_and_keeps_it() {
    for (t, row) in [
        (ToolId::Brush, Some("Brush Stroke")),
        (ToolId::Pencil, Some("Brush Stroke")),
        (ToolId::Eraser, None),
        (ToolId::Marquee, None),
    ] {
        fp_i18n::set_language("en");
        let mut s = doc_session(400.0, 300.0);
        let r = rect(&mut s, 20.0, 20.0, 100.0, 80.0);
        select(&mut s, &[r]);
        ex(&mut s, "element.setStroke", json!({"width": 0}));
        assert_eq!(brush_of(&s, r), None, "no stroke");
        let doc_before = s.active().expect("doc").doc.clone();
        let n0 = labels(&s).len();
        s.set_tool(t).expect("tool");
        assert!(selection(&s).is_empty(), "{t:?}: vector deselected");
        assert!(
            std::sync::Arc::ptr_eq(&doc_before, &s.active().expect("doc").doc),
            "{t:?}: document unchanged"
        );
        let rows = labels(&s)[n0..].to_vec();
        assert_eq!(
            rows,
            row.map(|r| vec![r.to_string()]).unwrap_or_default(),
            "{t:?}"
        );
        assert!(!mode(&s));
        if row.is_some() {
            // the row has no undo step of its own: Undo takes the step before it along
            let before = done(&s);
            s.undo().expect("undo");
            assert_eq!(done(&s), before - 2, "{t:?}");
        }
    }
    // German row names
    fp_i18n::set_language("de");
    let mut s = doc_session(400.0, 300.0);
    s.set_tool(ToolId::Pencil).expect("Pencil");
    assert_eq!(labels(&s).last().map(String::as_str), Some("Pinselstrich"));
    fp_i18n::set_language("en");
}

/// RE spec 72 §1.3 (h1, h8), nothing selected: the Brush's stroke equals the current one ->
/// "Stroke Color"; another "Stroke Color" right after it is not written; the Pencil's own stroke
/// differs -> "Brush Stroke", then the Brush again -> "Brush Stroke"; Eraser and Marquee write
/// nothing.
#[test]
fn fb87_brush_pencil_choice_rows_without_selection() {
    fp_i18n::set_language("en");
    let mut s = doc_session(400.0, 300.0);
    let n0 = labels(&s).len();
    let tail = |s: &Session| labels(s)[n0..].to_vec();
    s.set_tool(ToolId::Brush).expect("Brush");
    assert_eq!(tail(&s), ["Stroke Color"]);
    for t in [
        ToolId::Pointer,
        ToolId::Eraser,
        ToolId::Pointer,
        ToolId::Brush,
    ] {
        s.set_tool(t).expect("tool");
    }
    assert_eq!(
        tail(&s),
        ["Stroke Color"],
        "no second Stroke Color in a row"
    );
    s.set_tool(ToolId::Pointer).expect("Pointer");
    s.set_tool(ToolId::Pencil).expect("Pencil");
    assert_eq!(tail(&s), ["Stroke Color", "Brush Stroke"]);
    s.set_tool(ToolId::Pointer).expect("Pointer");
    s.set_tool(ToolId::Brush).expect("Brush");
    assert_eq!(tail(&s), ["Stroke Color", "Brush Stroke", "Brush Stroke"]);
    s.set_tool(ToolId::Marquee).expect("Marquee");
    assert_eq!(tail(&s).len(), 3, "Marquee writes no row");
    assert!(!mode(&s));
}
