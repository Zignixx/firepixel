//! FB-19 (TW-202, RE spec 60 section 1): what the pen shows and which cursor it sets. While a path
//! is drawn the pen's overlay carries the path (`PenPath` = no fill, no box in the UI), its outline
//! and anchors (earlier ones hollow, the last placed one filled) and the rubber band, which is
//! hidden over a segment or an inner corner point. Without a path in progress the pen shows "x"
//! (new path), "/" over the end point of an unselected open path, "+"/"−" on a selected path.
//! Right after closing the first anchor is filled. Activating the pen on a selected open path
//! resumes drawing at its end point.

use crate::common;
use common::*;
use fp_engine::tools::{Cursor, ToolId};
use fp_engine::{Overlay, Session, ToolEvent};
use fp_model::{ElementId, ElementKind};
use serde_json::json;

fn session() -> (Session, ElementId) {
    fp_i18n::set_language("de");
    let mut s = doc_session(600.0, 400.0);
    let r = rect(&mut s, 350.0, 250.0, 500.0, 350.0);
    s.set_tool(ToolId::Pen).unwrap_or_else(|e| panic!("{e}"));
    (s, r)
}

fn click(s: &mut Session, x: f32, y: f32) {
    events(s, &[ToolEvent::down(x, y), ToolEvent::up(x, y)]);
}

/// Hover without a button: the cursor of the response.
fn hover(s: &mut Session, x: f32, y: f32) -> Cursor {
    s.tool_event(&ToolEvent::drag(x, y))
        .map(|r| r.cursor)
        .unwrap_or_else(|e| panic!("{e}"))
}

fn has_band(fb: &[Overlay]) -> bool {
    fb.iter().any(|o| matches!(o, Overlay::Line { .. }))
}

fn pen_path(fb: &[Overlay]) -> Option<u64> {
    fb.iter().find_map(|o| match o {
        Overlay::PenPath { target } => Some(*target),
        _ => None,
    })
}

/// Selected points, `handles` and node count of one `PathPoints` overlay.
type Points = (Vec<(usize, usize)>, bool, usize);

/// (selected, handles, node count) of every `PathPoints` overlay.
fn points(fb: &[Overlay]) -> Vec<Points> {
    fb.iter()
        .filter_map(|o| match o {
            Overlay::PathPoints {
                contours,
                selected,
                handles,
            } => Some((
                selected.clone(),
                *handles,
                contours.iter().map(|c| c.nodes.len()).sum(),
            )),
            _ => None,
        })
        .collect()
}

fn selected_path(s: &Session) -> ElementId {
    s.active().expect("doc").selection.elements[0]
}

#[test]
fn fb19_pen_drawing_overlay_and_cursors() {
    let (mut s, r) = session();
    // no path in progress: "x" over empty space and over the rectangle's edge
    assert_eq!(hover(&mut s, 50.0, 50.0), Cursor::PenNew);
    assert_eq!(
        hover(&mut s, 350.0, 300.0),
        Cursor::PenNew,
        "rectangle edge: new path"
    );
    // first point: one filled anchor, the rubber band to the mouse
    click(&mut s, 100.0, 100.0);
    assert_eq!(
        hover(&mut s, 300.0, 200.0),
        Cursor::Pen,
        "plain pen while drawing"
    );
    click(&mut s, 250.0, 80.0);
    let fb = s.tool_feedback();
    let id = selected_path(&s);
    assert_ne!(id, r);
    assert_eq!(
        pen_path(&fb),
        Some(id.0),
        "the path being drawn: no fill, no box"
    );
    assert_eq!(
        points(&fb),
        vec![(vec![(0, 1)], true, 2)],
        "first hollow, last filled"
    );
    assert_eq!(hover(&mut s, 400.0, 150.0), Cursor::Pen);
    assert!(has_band(&s.tool_feedback()), "rubber band to the mouse");
    // over the first point "o", over the last point "^"
    assert_eq!(hover(&mut s, 100.0, 100.0), Cursor::PenClose);
    assert_eq!(hover(&mut s, 250.0, 80.0), Cursor::PenConvert);
    click(&mut s, 200.0, 250.0);
    assert_eq!(
        points(&s.tool_feedback()),
        vec![(vec![(0, 2)], true, 3)],
        "two hollow, the last filled"
    );
    // over an inner corner point "−", over a segment "+": no rubber band
    assert_eq!(hover(&mut s, 250.0, 80.0), Cursor::PenRemove);
    assert!(
        !has_band(&s.tool_feedback()),
        "rubber band hidden over a point to delete"
    );
    assert_eq!(hover(&mut s, 175.0, 90.0), Cursor::PenAdd);
    assert!(
        !has_band(&s.tool_feedback()),
        "rubber band hidden over a segment"
    );
    assert_eq!(hover(&mut s, 450.0, 100.0), Cursor::Pen);
    assert!(
        has_band(&s.tool_feedback()),
        "rubber band back over empty space"
    );
    // close: the first anchor filled, the others hollow, the fill shows (no `PenPath`)
    click(&mut s, 100.0, 100.0);
    let id = selected_path(&s);
    match &el(&s, id).kind {
        ElementKind::Path(p) => assert!(p.contours[0].closed),
        _ => panic!("not a path"),
    }
    let fb = s.tool_feedback();
    assert_eq!(pen_path(&fb), None, "closed: drawn normally");
    assert_eq!(
        points(&fb),
        vec![(vec![(0, 0)], true, 3)],
        "first anchor filled"
    );
    // the selected closed path: "+" on a segment, "−" on an anchor (also the first one)
    assert_eq!(hover(&mut s, 175.0, 90.0), Cursor::PenAdd);
    assert_eq!(hover(&mut s, 100.0, 100.0), Cursor::PenRemove);
    assert_eq!(hover(&mut s, 50.0, 300.0), Cursor::PenNew);
    // the next press ends the "just closed" display: the anchors are hollow again
    click(&mut s, 50.0, 300.0);
    events(&mut s, &[ToolEvent::double(50.0, 300.0)]);
    ex(&mut s, "select.set", json!({"ids": [id.0]}));
    assert_eq!(
        points(&s.tool_feedback()),
        vec![(vec![], true, 3)],
        "hollow anchors"
    );
}

#[test]
fn fb19_pen_on_selected_paths_and_unselected_ends() {
    let (mut s, r) = session();
    let v = ex(
        &mut s,
        "draw.path",
        json!({"contours": [{"closed": false, "nodes": [
            {"anchor": [100.0, 300.0], "in": [100.0, 300.0], "out": [100.0, 300.0]},
            {"anchor": [250.0, 290.0], "in": [250.0, 290.0], "out": [250.0, 290.0]},
        ]}]}),
    );
    let open = ElementId(
        v["id"]
            .as_u64()
            .or_else(|| v["ids"][0].as_u64())
            .expect("id"),
    );
    // the rectangle primitive selected: its four corners filled (no point handles)
    ex(&mut s, "select.set", json!({"ids": [r.0]}));
    assert_eq!(points(&s.tool_feedback()), vec![(vec![], false, 4)]);
    // the open path not selected: "/" over its end point, "x" over its segment
    assert_eq!(hover(&mut s, 250.0, 290.0), Cursor::PenContinue);
    assert_eq!(hover(&mut s, 175.0, 295.0), Cursor::PenNew);
    // activating the pen on the selected open path resumes drawing at its end point
    ex(&mut s, "select.set", json!({"ids": [open.0]}));
    s.set_tool(ToolId::Pointer).expect("pointer");
    s.set_tool(ToolId::Pen).expect("pen");
    let fb = s.tool_feedback();
    assert_eq!(pen_path(&fb), Some(open.0), "drawing resumed");
    assert_eq!(points(&fb), vec![(vec![(0, 1)], true, 2)]);
    assert_eq!(hover(&mut s, 300.0, 200.0), Cursor::Pen);
    let band_from_end = s.tool_feedback().iter().any(|o| {
        matches!(o, Overlay::Line { from, .. } if (from.x - 250.0).abs() < 0.01 && (from.y - 290.0).abs() < 0.01)
    });
    assert!(band_from_end, "rubber band from the end point");
    assert_eq!(hover(&mut s, 100.0, 300.0), Cursor::PenClose);
    assert_eq!(hover(&mut s, 250.0, 290.0), Cursor::PenConvert);
    click(&mut s, 300.0, 200.0);
    assert_eq!(selected_path(&s), open, "the same path goes on");
    match &el(&s, open).kind {
        ElementKind::Path(p) => assert_eq!(p.contours[0].nodes.len(), 3),
        _ => panic!("not a path"),
    }
}
