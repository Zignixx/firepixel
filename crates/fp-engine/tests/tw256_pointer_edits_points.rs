//! TW-256 (Firepixel extension, contributor PR #3): View ▸ Pointer Edits Points, preference
//! `PointerEditsPoints`, default off. Off, the measured rules stay (corner handle scales a
//! rectangle, PARITAET_A L26; a path point drag moves the object, PAR 08/04; see `canvas_tools`,
//! `parx_tools3`, `tools`). On, the Pointer selects anchors (click; Ctrl/Shift add or remove),
//! a drag moves only the selected points (a rectangle becomes a path), Delete removes them and
//! the arrow keys nudge them. Also: Ctrl adds points with the Subselection (TW-256 review).

use crate::common;
use common::*;
use fp_engine::tools::{Cursor, ToolId, ToolKey};
use fp_engine::{Session, ToolEvent};
use fp_geom::Rect;
use fp_model::{ElementId, ElementKind};
use serde_json::json;

fn tool(s: &mut Session, t: ToolId) {
    s.set_tool(t).unwrap_or_else(|e| panic!("{e}"));
}

fn drag(s: &mut Session, a: (f32, f32), b: (f32, f32)) {
    events(s, &[ToolEvent::down(a.0, a.1)]);
    for i in 1..=10 {
        let t = i as f32 / 10.0;
        events(
            s,
            &[ToolEvent::drag(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)],
        );
    }
    events(s, &[ToolEvent::up(b.0, b.1)]);
}

fn label(s: &Session) -> Option<String> {
    s.active()
        .and_then(|d| d.history.undo_label().map(|l| l.to_string()))
}

fn anchors(s: &Session, id: ElementId) -> Vec<(f32, f32)> {
    match &el(s, id).kind {
        ElementKind::Path(p) => p.contours[0]
            .nodes
            .iter()
            .map(|n| (n.anchor.x, n.anchor.y))
            .collect(),
        _ => panic!("not a path"),
    }
}

fn is_primitive(s: &Session, id: ElementId) -> bool {
    matches!(&el(s, id).kind, ElementKind::Path(p) if p.primitive.is_some())
}

fn selected_points(s: &Session) -> usize {
    s.active().map(|d| d.selection.points.len()).unwrap_or(0)
}

fn on(s: &mut Session) {
    ex(s, "view.pointerEditsPoints", json!({}));
    assert_eq!(s.command_state("view.pointerEditsPoints").1, Some(true));
}

fn triangle(s: &mut Session) -> ElementId {
    tool(s, ToolId::Pen);
    let mut evs = Vec::new();
    for (x, y) in [(10.0, 60.0), (60.0, 10.0), (110.0, 60.0), (10.0, 60.0)] {
        evs.push(ToolEvent::down(x, y));
        evs.push(ToolEvent::up(x, y));
    }
    events(s, &evs);
    top_elements(s)[0].id
}

#[test]
fn toggle_is_off_by_default_and_flips_the_pref() {
    let mut s = doc_session(300.0, 200.0);
    assert_eq!(s.command_state("view.pointerEditsPoints").1, Some(false));
    assert!(!fp_engine::tools::pointer_edits_points(&s.prefs));
    on(&mut s);
    assert!(s.prefs.get_bool("PointerEditsPoints", false));
    ex(&mut s, "view.pointerEditsPoints", json!({}));
    assert_eq!(s.command_state("view.pointerEditsPoints").1, Some(false));
}

#[test]
fn off_corner_handle_still_scales_the_rectangle() {
    let mut s = doc_session(300.0, 200.0);
    let a = rect(&mut s, 10.0, 20.0, 110.0, 80.0);
    select(&mut s, &[a]);
    tool(&mut s, ToolId::Pointer);
    drag(&mut s, (110.0, 80.0), (140.0, 100.0));
    assert!(is_primitive(&s, a), "stays a rectangle");
    assert!(
        rect_approx(bounds(&s, a), Rect::new(10.0, 20.0, 140.0, 100.0)),
        "{:?}",
        bounds(&s, a)
    );
}

#[test]
fn on_corner_drag_moves_only_that_point_and_converts_the_rectangle() {
    let mut s = doc_session(300.0, 200.0);
    on(&mut s);
    let a = rect(&mut s, 10.0, 20.0, 110.0, 80.0);
    select(&mut s, &[a]);
    tool(&mut s, ToolId::Pointer);
    // over a corner: the point cursor, not the measured K2 arrow
    let r = s.tool_event(&ToolEvent::drag(110.0, 20.0)).expect("hover");
    assert_eq!(r.cursor, Cursor::PointerPoint);
    drag(&mut s, (110.0, 20.0), (140.0, 10.0));
    assert!(!is_primitive(&s, a), "the point drag makes it a path");
    assert_eq!(
        anchors(&s, a),
        vec![(10.0, 20.0), (140.0, 10.0), (110.0, 80.0), (10.0, 80.0)]
    );
    assert_eq!(label(&s).as_deref(), Some("Punkt verschieben"));
    // Ctrl+click adds the bottom-right corner, dragging it moves both selected points
    events(
        &mut s,
        &[
            ToolEvent::down(110.0, 80.0).ctrl(),
            ToolEvent::up(110.0, 80.0).ctrl(),
        ],
    );
    assert_eq!(selected_points(&s), 2);
    drag(&mut s, (110.0, 80.0), (110.0, 100.0));
    assert_eq!(
        anchors(&s, a),
        vec![(10.0, 20.0), (140.0, 30.0), (110.0, 100.0), (10.0, 80.0)]
    );
    // a plain click on a selected point narrows the selection to it
    events(
        &mut s,
        &[ToolEvent::down(110.0, 100.0), ToolEvent::up(110.0, 100.0)],
    );
    assert_eq!(selected_points(&s), 1);
    // Shift+click on it removes it again
    events(
        &mut s,
        &[
            ToolEvent::down(110.0, 100.0).shift(),
            ToolEvent::up(110.0, 100.0).shift(),
        ],
    );
    assert_eq!(selected_points(&s), 0);
}

#[test]
fn on_arrow_keys_nudge_and_delete_removes_the_selected_points() {
    let mut s = doc_session(300.0, 200.0);
    on(&mut s);
    let a = triangle(&mut s);
    select(&mut s, &[a]);
    tool(&mut s, ToolId::Pointer);
    events(
        &mut s,
        &[ToolEvent::down(60.0, 10.0), ToolEvent::up(60.0, 10.0)],
    );
    assert_eq!(selected_points(&s), 1);
    events(&mut s, &[ToolEvent::key(ToolKey::Right)]);
    let pts = anchors(&s, a);
    assert!(pts.contains(&(61.0, 10.0)), "{pts:?}");
    assert!(pts.contains(&(10.0, 60.0)), "the other points stay: {pts:?}");
    let n = pts.len();
    events(&mut s, &[ToolEvent::key(ToolKey::Delete)]);
    assert_eq!(anchors(&s, a).len(), n - 1, "Delete removes the point");
}

#[test]
fn on_edge_handles_of_several_objects_still_resize() {
    let mut s = doc_session(300.0, 200.0);
    on(&mut s);
    let a = rect(&mut s, 10.0, 20.0, 60.0, 80.0);
    let b = rect(&mut s, 100.0, 20.0, 150.0, 80.0);
    select(&mut s, &[a, b]);
    tool(&mut s, ToolId::Pointer);
    // right edge handle of the selection frame: resize cursor
    let r = s.tool_event(&ToolEvent::drag(150.0, 50.0)).expect("hover");
    assert_eq!(r.cursor, Cursor::ResizeH);
    drag(&mut s, (150.0, 50.0), (170.0, 50.0));
    assert!(
        bounds(&s, b).right > 160.0,
        "the frame was stretched: {:?}",
        bounds(&s, b)
    );
}

#[test]
fn subselection_ctrl_click_adds_points() {
    let mut s = doc_session(300.0, 200.0);
    let a = triangle(&mut s);
    select(&mut s, &[a]);
    tool(&mut s, ToolId::Subselection);
    events(
        &mut s,
        &[ToolEvent::down(60.0, 10.0), ToolEvent::up(60.0, 10.0)],
    );
    assert_eq!(selected_points(&s), 1);
    events(
        &mut s,
        &[
            ToolEvent::down(110.0, 60.0).ctrl(),
            ToolEvent::up(110.0, 60.0).ctrl(),
        ],
    );
    assert_eq!(selected_points(&s), 2, "Ctrl adds like Shift");
    events(
        &mut s,
        &[
            ToolEvent::down(110.0, 60.0).ctrl(),
            ToolEvent::up(110.0, 60.0).ctrl(),
        ],
    );
    assert_eq!(selected_points(&s), 1, "Ctrl on a selected point removes it");
}
