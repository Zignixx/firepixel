//! FB-96 (Firepixel extension): Ctrl (Cmd on macOS) held at the press of a Brush or Pencil stroke
//! over a bitmap paints a new bitmap on top instead of into the bitmap under the point; Ctrl held
//! only later in the stroke changes nothing, the Eraser ignores it. `bitmap.paint` takes
//! `newBitmap: true` to force a new bitmap even when `at` hits one. The hover cursor shows the
//! "+" badge while Ctrl is held over a bitmap.

use crate::common::*;
use fp_engine::tools::{Cursor, ToolId};
use fp_engine::{Session, ToolEvent, docops};
use fp_model::{ElementId, ElementKind};
use serde_json::json;

fn image(s: &Session, id: ElementId) -> fp_raster::RgbaImage {
    match &el(s, id).kind {
        ElementKind::Bitmap(b) => docops::bitmap_image(b).expect("image"),
        _ => panic!("not a bitmap"),
    }
}

fn count(s: &Session) -> usize {
    let d = s.active().expect("doc");
    docops::own_layers(&d.doc, d.page)
        .iter()
        .map(|(_, l)| l.cels.iter().map(|c| c.elements.len()).sum::<usize>())
        .sum()
}

fn sel(s: &Session) -> Vec<ElementId> {
    s.active()
        .map(|d| d.selection.elements.clone())
        .unwrap_or_default()
}

/// The topmost bitmap under `(x, y)`.
fn top_bitmap(s: &Session, x: f32, y: f32) -> Option<ElementId> {
    let d = s.active().expect("doc");
    fp_engine::paint::stroke_target(
        &d.doc,
        d.page,
        d.frame,
        &[],
        Some(fp_geom::Point::new(x, y)),
    )
    .map(|(id, _)| id)
}

/// A blue bitmap A (50,50 100×100), selected.
fn mk() -> (Session, ElementId) {
    fp_i18n::set_language("en");
    let mut s = doc_session(600.0, 400.0);
    let a = bitmap(&mut s, 50, 50, 100, 100, [0, 0, 255, 255]);
    select(&mut s, &[a]);
    (s, a)
}

/// A stroke with the active tool from (80,100) to (120,100); `ctrl_press` / `ctrl_later` hold
/// Ctrl at the press / during the drag.
fn stroke(s: &mut Session, ctrl_press: bool, ctrl_later: bool) {
    let c = |e: ToolEvent, on: bool| if on { e.ctrl() } else { e };
    events(
        s,
        &[
            c(ToolEvent::down(80.0, 100.0), ctrl_press),
            c(ToolEvent::drag(100.0, 100.0), ctrl_later),
            c(ToolEvent::up(120.0, 100.0), ctrl_later),
        ],
    );
}

fn unchanged(s: &Session, a: ElementId) -> bool {
    image(s, a).data().iter().all(|p| *p == [0, 0, 255, 255])
}

#[test]
fn ctrl_at_the_press_paints_a_new_bitmap_on_top_brush_and_pencil() {
    for tool in [ToolId::Brush, ToolId::Pencil] {
        let (mut s, a) = mk();
        s.set_tool(tool).expect("tool");
        let n = count(&s);
        stroke(&mut s, true, false);
        assert_eq!(count(&s), n + 1, "{tool:?}: a new bitmap");
        assert!(unchanged(&s, a), "{tool:?}: A unchanged");
        let new = sel(&s);
        assert_eq!(new.len(), 1, "{tool:?}: the new bitmap is selected");
        assert_ne!(new[0], a);
        assert!(matches!(el(&s, new[0]).kind, ElementKind::Bitmap(_)));
        assert_eq!(
            top_bitmap(&s, 100.0, 100.0),
            Some(new[0]),
            "{tool:?}: on top"
        );
        assert!(
            s.active().expect("doc").bitmap_mode,
            "{tool:?}: bitmap mode on the new bitmap"
        );
    }
}

#[test]
fn without_ctrl_or_with_ctrl_only_later_the_stroke_paints_into_the_bitmap() {
    for tool in [ToolId::Brush, ToolId::Pencil] {
        for later in [false, true] {
            let (mut s, a) = mk();
            s.set_tool(tool).expect("tool");
            let n = count(&s);
            stroke(&mut s, false, later);
            assert_eq!(count(&s), n, "{tool:?} later={later}: no new bitmap");
            assert!(!unchanged(&s, a), "{tool:?} later={later}: A painted");
            assert_eq!(sel(&s), vec![a]);
        }
    }
}

#[test]
fn ctrl_press_over_an_unselected_bitmap_and_on_empty_canvas() {
    // nothing selected, Ctrl over A: new bitmap, A unchanged
    let (mut s, a) = mk();
    ex(&mut s, "select.none", json!({}));
    s.set_tool(ToolId::Pencil).expect("pencil");
    let n = count(&s);
    stroke(&mut s, true, false);
    assert_eq!(count(&s), n + 1);
    assert!(unchanged(&s, a));
    // A selected, Ctrl on empty canvas next to it: Ctrl changes nothing (A grows as without Ctrl)
    let (mut s, a) = mk();
    s.set_tool(ToolId::Pencil).expect("pencil");
    let n = count(&s);
    events(
        &mut s,
        &[
            ToolEvent::down(160.0, 100.0).ctrl(),
            ToolEvent::up(170.0, 100.0).ctrl(),
        ],
    );
    assert_eq!(count(&s), n, "no new bitmap away from a bitmap");
    assert!(image(&s, a).bounds().right > 150, "A grew");
}

#[test]
fn the_eraser_ignores_ctrl() {
    let (mut s, a) = mk();
    s.set_tool(ToolId::Eraser).expect("eraser");
    let n = count(&s);
    stroke(&mut s, true, false);
    assert_eq!(count(&s), n, "no new bitmap");
    assert!(!unchanged(&s, a), "A erased");
}

#[test]
fn bitmap_paint_new_bitmap_parameter_wins_over_a_hit() {
    let br = serde_json::to_value(fp_model::Brush::basic(4)).expect("brush");
    let params = |nb: bool| {
        json!({"tool": "pencil", "brush": br, "color": "#C04040", "trimEnd": false,
               "points": [[80.0, 100.0], [120.0, 100.0]], "at": [80.0, 100.0], "newBitmap": nb})
    };
    let (mut s, a) = mk();
    ex(&mut s, "select.none", json!({}));
    let n = count(&s);
    let v = ex(&mut s, "bitmap.paint", params(true));
    assert_eq!(
        count(&s),
        n + 1,
        "newBitmap: a new bitmap although `at` hits A"
    );
    assert_ne!(v["target"].as_u64(), Some(a.0));
    assert!(unchanged(&s, a));
    let (mut s, a) = mk();
    ex(&mut s, "select.none", json!({}));
    let n = count(&s);
    let v = ex(&mut s, "bitmap.paint", params(false));
    assert_eq!(count(&s), n);
    assert_eq!(v["target"].as_u64(), Some(a.0));
}

#[test]
fn the_hover_cursor_shows_the_new_badge_with_ctrl_over_a_bitmap() {
    let cur = |s: &mut Session, e: ToolEvent| {
        s.tool_event(&e)
            .map(|r| r.cursor)
            .unwrap_or_else(|e| panic!("{e}"))
    };
    for (tool, plain, badged) in [
        (ToolId::Brush, Cursor::Brush, Cursor::BrushNew),
        (ToolId::Pencil, Cursor::Pencil, Cursor::PencilNew),
    ] {
        let (mut s, _) = mk();
        s.set_tool(tool).expect("tool");
        assert_eq!(cur(&mut s, ToolEvent::drag(100.0, 100.0)), plain);
        assert_eq!(cur(&mut s, ToolEvent::drag(100.0, 100.0).ctrl()), badged);
        // away from a bitmap: no badge
        assert_eq!(cur(&mut s, ToolEvent::drag(300.0, 300.0).ctrl()), plain);
        // while a stroke runs: no badge
        cur(&mut s, ToolEvent::down(100.0, 100.0));
        assert_eq!(cur(&mut s, ToolEvent::drag(110.0, 100.0).ctrl()), plain);
        cur(&mut s, ToolEvent::up(110.0, 100.0));
    }
    let (mut s, _) = mk();
    s.set_tool(ToolId::Eraser).expect("eraser");
    assert_eq!(
        cur(&mut s, ToolEvent::drag(100.0, 100.0).ctrl()),
        Cursor::Eraser
    );
}
