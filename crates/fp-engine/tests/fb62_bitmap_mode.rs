//! FB-62 / TW-242 (RE spec 71 section 1): when bitmap mode starts and ends, the History rows it
//! writes ("Edit Bitmap", "Exit Bitmap Mode") and paint rows named after their tool.

use crate::common;
use common::*;
use fp_engine::tools::ToolId;
use fp_engine::{Session, ToolEvent, ToolKey};
use serde_json::json;

fn labels(s: &Session) -> Vec<String> {
    s.active().map(|d| d.history.labels()).unwrap_or_default()
}

fn rows(s: &Session) -> usize {
    s.active().expect("doc").history.undo_count()
}

fn mode(s: &Session) -> bool {
    s.active().expect("doc").bitmap_mode
}

fn pixels(s: &Session) -> bool {
    s.active().expect("doc").selection.has_pixels()
}

fn esc(s: &mut Session) {
    s.tool_event(&ToolEvent::key(ToolKey::Escape))
        .expect("Escape");
}

/// Document 400 x 300 with a white bitmap 160,120 80 x 60, selected with the Pointer; History
/// emptied.
fn setup() -> (Session, fp_model::ElementId) {
    fp_i18n::set_language("en");
    let mut s = doc_session(400.0, 300.0);
    s.set_colors(Some(fp_color::Argb(0xff00_0000)), None);
    let b = bitmap(&mut s, 160, 120, 80, 60, [255, 255, 255, 255]);
    select(&mut s, &[b]);
    if let Some(d) = s.active_mut() {
        d.history.clear();
    }
    assert!(!mode(&s));
    (s, b)
}

fn stroke(s: &mut Session, from: (f32, f32), to: (f32, f32)) {
    events(
        s,
        &[
            ToolEvent::down(from.0, from.1),
            ToolEvent::drag((from.0 + to.0) / 2.0, (from.1 + to.1) / 2.0),
            ToolEvent::drag(to.0, to.1),
            ToolEvent::up(to.0, to.1),
        ],
    );
}

fn tail(s: &Session, n: usize) -> Vec<String> {
    let l = labels(s);
    l[l.len().saturating_sub(n)..].to_vec()
}

/// B: choosing a bitmap tool while a bitmap is selected starts bitmap mode at once with one row
/// "Edit Bitmap"; a further bitmap tool adds no second row.
#[test]
fn fb62_bitmap_tool_with_selected_bitmap_starts_mode() {
    for t in [
        ToolId::Marquee,
        ToolId::Lasso,
        ToolId::MagicWand,
        ToolId::Brush,
        ToolId::Pencil,
        ToolId::Eraser,
        ToolId::Blur,
        ToolId::RubberStamp,
    ] {
        let (mut s, b) = setup();
        s.set_tool(t).expect("tool");
        assert!(mode(&s), "{t:?}: bitmap mode on");
        assert_eq!(labels(&s), vec!["Edit Bitmap"], "{t:?}");
        assert_eq!(s.active().expect("doc").selection.elements, vec![b]);
        let other = if t == ToolId::Marquee {
            ToolId::Brush
        } else {
            ToolId::Marquee
        };
        s.set_tool(other).expect("tool");
        assert_eq!(rows(&s), 1, "{t:?}: no second row while in bitmap mode");
    }
    // German row name
    let (mut s, _) = setup();
    fp_i18n::set_language("de");
    s.set_tool(ToolId::Brush).expect("Brush");
    assert_eq!(labels(&s), vec!["Bitmap bearbeiten"]);
    fp_i18n::set_language("en");
}

/// B: without a selected bitmap choosing a bitmap tool changes nothing.
#[test]
fn fb62_bitmap_tool_without_bitmap_keeps_mode_off() {
    let (mut s, _) = setup();
    select(&mut s, &[]);
    s.set_tool(ToolId::Brush).expect("Brush");
    assert!(!mode(&s));
    assert_eq!(rows(&s), 0);
}

/// B + D: a stroke on a bitmap outside bitmap mode writes "Edit Bitmap" and then the stroke row
/// named after the tool; further strokes write only their row.
#[test]
fn fb62_stroke_starts_mode_and_rows_named_after_tool() {
    for (t, name) in [
        (ToolId::Brush, "Brush"),
        (ToolId::Pencil, "Pencil"),
        (ToolId::Eraser, "Eraser"),
    ] {
        let (mut s, b) = setup();
        select(&mut s, &[]);
        s.set_tool(t).expect("tool");
        select(&mut s, &[b]);
        assert!(!mode(&s));
        let n = rows(&s);
        stroke(&mut s, (170.0, 150.0), (220.0, 150.0));
        assert!(mode(&s), "{t:?}: stroke starts bitmap mode");
        assert_eq!(rows(&s), n + 2, "{t:?}: {:?}", labels(&s));
        assert_eq!(
            tail(&s, 2),
            vec!["Edit Bitmap".to_string(), name.to_string()]
        );
        stroke(&mut s, (170.0, 160.0), (220.0, 160.0));
        assert_eq!(rows(&s), n + 3, "{t:?}: only the stroke row");
        assert_eq!(tail(&s, 1), vec![name.to_string()]);
        // undo takes back the second stroke only
        ex(&mut s, "edit.undo", json!({}));
        assert_eq!(tail(&s, 1), vec![name.to_string()]);
    }
    // German names of the rows: the tool names
    let (mut s, b) = setup();
    fp_i18n::set_language("de");
    select(&mut s, &[]);
    s.set_tool(ToolId::Pencil).expect("Pencil");
    select(&mut s, &[b]);
    stroke(&mut s, (170.0, 150.0), (220.0, 150.0));
    assert_eq!(
        tail(&s, 2),
        vec!["Bitmap bearbeiten".to_string(), fp_i18n::t("tool-pencil")]
    );
    fp_i18n::set_language("en");
}

/// B: a Brush stroke on empty space with nothing selected makes a new bitmap and starts bitmap
/// mode ("Edit Bitmap" before the stroke row).
#[test]
fn fb62_brush_on_empty_space_starts_mode() {
    let (mut s, _) = setup();
    select(&mut s, &[]);
    s.set_tool(ToolId::Brush).expect("Brush");
    stroke(&mut s, (40.0, 40.0), (90.0, 40.0));
    assert!(mode(&s));
    assert_eq!(tail(&s, 2), vec!["Edit Bitmap", "Brush"]);
}

/// B: a Pointer double-click on a bitmap writes two rows "Edit Bitmap" and switches to the
/// Marquee.
#[test]
fn fb62_pointer_double_click_on_bitmap() {
    let (mut s, b) = setup();
    select(&mut s, &[]);
    events(&mut s, &[ToolEvent::double(200.0, 150.0)]);
    assert!(mode(&s));
    assert_eq!(s.tools.active, ToolId::Marquee);
    assert_eq!(labels(&s), vec!["Edit Bitmap", "Edit Bitmap"]);
    assert_eq!(s.active().expect("doc").selection.elements, vec![b]);
}

/// C: the red button ends bitmap mode with one row; the tool stays and the bitmap stays selected.
#[test]
fn fb62_button_ends_mode_tool_stays() {
    for t in [ToolId::Brush, ToolId::Marquee] {
        let (mut s, b) = setup();
        s.set_tool(t).expect("tool");
        assert!(s.is_available("modify.exitBitmapMode"));
        ex(&mut s, "modify.exitBitmapMode", json!({"row": true}));
        assert!(!mode(&s));
        assert_eq!(labels(&s), vec!["Edit Bitmap", "Exit Bitmap Mode"], "{t:?}");
        assert_eq!(s.tools.active, t, "tool stays");
        assert_eq!(s.active().expect("doc").selection.elements, vec![b]);
    }
}

/// C: choosing the Pointer ends bitmap mode (one row) without a pixel selection and keeps it with
/// one.
#[test]
fn fb62_pointer_choice_ends_mode_only_without_pixel_selection() {
    let (mut s, b) = setup();
    s.set_tool(ToolId::Brush).expect("Brush");
    stroke(&mut s, (170.0, 150.0), (220.0, 150.0));
    s.set_tool(ToolId::Pointer).expect("Pointer");
    assert!(!mode(&s));
    assert_eq!(labels(&s), vec!["Edit Bitmap", "Brush", "Exit Bitmap Mode"]);
    assert_eq!(s.active().expect("doc").selection.elements, vec![b]);

    let (mut s, _) = setup();
    s.set_tool(ToolId::Marquee).expect("Marquee");
    stroke(&mut s, (170.0, 130.0), (210.0, 170.0));
    assert!(pixels(&s), "pixel selection");
    let n = rows(&s);
    s.set_tool(ToolId::Pointer).expect("Pointer");
    assert!(mode(&s), "bitmap mode stays with marching ants");
    assert!(pixels(&s));
    assert_eq!(rows(&s), n, "no row");

    // without work: Marquee chosen and the Pointer again
    let (mut s, _) = setup();
    s.set_tool(ToolId::Marquee).expect("Marquee");
    s.set_tool(ToolId::Pointer).expect("Pointer");
    assert!(!mode(&s));
    assert_eq!(labels(&s), vec!["Edit Bitmap", "Exit Bitmap Mode"]);
}

/// C: Escape in bitmap mode switches to the Pointer; two rows "Exit Bitmap Mode" without a pixel
/// selection, one with a pixel selection.
#[test]
fn fb62_escape_ends_mode() {
    // after a brush stroke
    let (mut s, b) = setup();
    s.set_tool(ToolId::Brush).expect("Brush");
    stroke(&mut s, (170.0, 150.0), (220.0, 150.0));
    esc(&mut s);
    assert!(!mode(&s));
    assert_eq!(s.tools.active, ToolId::Pointer);
    assert_eq!(
        labels(&s),
        vec![
            "Edit Bitmap",
            "Brush",
            "Exit Bitmap Mode",
            "Exit Bitmap Mode"
        ]
    );
    assert_eq!(s.active().expect("doc").selection.elements, vec![b]);
    // Marquee without a pixel selection
    let (mut s, _) = setup();
    s.set_tool(ToolId::Marquee).expect("Marquee");
    esc(&mut s);
    assert!(!mode(&s));
    assert_eq!(s.tools.active, ToolId::Pointer);
    assert_eq!(
        labels(&s),
        vec!["Edit Bitmap", "Exit Bitmap Mode", "Exit Bitmap Mode"]
    );
    // Marquee with a pixel selection: one row
    let (mut s, b) = setup();
    s.set_tool(ToolId::Marquee).expect("Marquee");
    stroke(&mut s, (170.0, 130.0), (210.0, 170.0));
    assert!(pixels(&s));
    let n = rows(&s);
    esc(&mut s);
    assert!(!mode(&s) && !pixels(&s));
    assert_eq!(s.tools.active, ToolId::Pointer);
    assert_eq!(rows(&s), n + 1);
    assert_eq!(tail(&s, 1), vec!["Exit Bitmap Mode"]);
    assert_eq!(s.active().expect("doc").selection.elements, vec![b]);
}

/// A: outside bitmap mode the exit command is unavailable once nothing was worked on.
#[test]
fn fb62_exit_unavailable_outside_mode() {
    let (s, _) = setup();
    assert!(!s.is_available("modify.exitBitmapMode"));
}
