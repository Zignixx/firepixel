//! FB-01 (re 52 §1): pen clicks and drags on existing anchors. A click on the last anchor of the
//! path being drawn retracts its outgoing handle (convert point), a drag pulls it again as a
//! symmetric curve point; the other anchor modes (interior point, segment, first point, endpoint of
//! another open path) per the mode table.

use crate::common;
use common::*;
use fp_engine::tools::{Cursor, ToolId};
use fp_engine::{PrefValue, Session, ToolEvent, ToolKey};
use fp_geom::Point;
use fp_model::{Contour, ElementId, ElementKind};
use serde_json::json;

fn session() -> Session {
    let mut s = doc_session(800.0, 600.0);
    s.set_tool(ToolId::Pen).unwrap_or_else(|e| panic!("{e}"));
    s
}

fn click(s: &mut Session, x: f32, y: f32) {
    events(s, &[ToolEvent::down(x, y), ToolEvent::up(x, y)]);
}

/// Drag in 10 steps with modifiers.
fn drag_m(s: &mut Session, a: (f32, f32), b: (f32, f32), m: impl Fn(ToolEvent) -> ToolEvent) {
    events(s, &[m(ToolEvent::down(a.0, a.1))]);
    for i in 1..=10 {
        let t = i as f32 / 10.0;
        events(s, &[m(ToolEvent::drag(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t))]);
    }
    events(s, &[m(ToolEvent::up(b.0, b.1))]);
}

fn enter(s: &mut Session) {
    events(s, &[ToolEvent::key(ToolKey::Enter)]);
}

fn contours(s: &Session, id: ElementId) -> Vec<Contour> {
    match &el(s, id).kind {
        ElementKind::Path(p) => p.contours.clone(),
        _ => panic!("not a path"),
    }
}

fn sel(s: &Session) -> Vec<ElementId> {
    s.active().map(|d| d.selection.elements.clone()).unwrap_or_default()
}

fn label(s: &Session) -> Option<String> {
    s.active().and_then(|d| d.history.undo_label().map(|l| l.to_string()))
}

fn pt(p: Point) -> (f32, f32) {
    (p.x, p.y)
}

fn hover_cursor(s: &mut Session, x: f32, y: f32) -> Cursor {
    s.tool_event(&ToolEvent::drag(x, y)).map(|r| r.cursor).unwrap_or_else(|e| panic!("{e}"))
}

/// Tester sequence fb01_1..3: corner point, curve point dragged, click on that point, next point.
#[test]
fn pen_click_last_anchor_retracts_outgoing_handle() {
    let mut s = session();
    click(&mut s, 100.0, 300.0);
    drag_m(&mut s, (250.0, 300.0), (300.0, 250.0), |e| e);
    assert_eq!(hover_cursor(&mut s, 251.0, 301.0), Cursor::PenConvert, "pen with ^ over the last point");
    click(&mut s, 251.0, 301.0);
    click(&mut s, 400.0, 300.0);
    enter(&mut s);
    let id = sel(&s)[0];
    let c = contours(&s, id);
    assert_eq!(c.len(), 1);
    assert!(!c[0].closed, "path stays open");
    assert_eq!(c[0].nodes.len(), 3, "no extra point");
    let n = &c[0].nodes[1];
    assert_eq!(pt(n.anchor), (250.0, 300.0));
    assert_eq!(n.succ, n.anchor, "outgoing handle exactly on the anchor");
    assert_eq!(pt(n.pred), (200.0, 350.0), "incoming handle unchanged");
    assert_eq!(label(&s).as_deref(), Some("Stift"), "one pen step");
}

#[test]
fn pen_drag_on_last_anchor_pulls_symmetric_curve_point() {
    let mut s = session();
    click(&mut s, 100.0, 300.0);
    drag_m(&mut s, (250.0, 300.0), (300.0, 250.0), |e| e);
    // 1 px wobble stays a click (threshold 2 screen px), then a real drag
    drag_m(&mut s, (250.0, 300.0), (250.0, 360.0), |e| e);
    click(&mut s, 400.0, 300.0);
    enter(&mut s);
    let c = contours(&s, sel(&s)[0]);
    assert_eq!(c[0].nodes.len(), 3);
    let n = &c[0].nodes[1];
    assert_eq!(pt(n.succ), (250.0, 360.0), "outgoing handle under the pointer");
    assert_eq!(pt(n.pred), (250.0, 240.0), "incoming mirrored with the same length");
}

#[test]
fn pen_alt_drag_on_last_anchor_decouples() {
    let mut s = session();
    click(&mut s, 100.0, 300.0);
    drag_m(&mut s, (250.0, 300.0), (300.0, 250.0), |e| e);
    drag_m(&mut s, (250.0, 300.0), (250.0, 360.0), |e| e.alt());
    click(&mut s, 400.0, 300.0);
    enter(&mut s);
    let n = contours(&s, sel(&s)[0])[0].nodes[1].clone();
    assert_eq!(pt(n.succ), (250.0, 360.0), "only the outgoing handle follows");
    assert_eq!(pt(n.pred), (200.0, 350.0), "incoming handle stays as it was");
}

#[test]
fn pen_ctrl_and_shift_drag_on_last_anchor() {
    let mut s = session();
    click(&mut s, 100.0, 300.0);
    click(&mut s, 250.0, 300.0);
    drag_m(&mut s, (250.0, 300.0), (270.0, 320.0), |e| e.ctrl());
    drag_m(&mut s, (270.0, 320.0), (330.0, 330.0), |e| e.shift());
    enter(&mut s);
    let c = contours(&s, sel(&s)[0]);
    assert_eq!(c[0].nodes.len(), 2);
    let n = &c[0].nodes[1];
    assert_eq!(pt(n.anchor), (270.0, 320.0), "Ctrl moves the point");
    assert_eq!(pt(n.succ), (330.0, 320.0), "Shift snaps the handle to 45°");
}

#[test]
fn pen_click_interior_curve_point_while_drawing_makes_corner_then_deletes() {
    let mut s = session();
    click(&mut s, 100.0, 300.0);
    drag_m(&mut s, (200.0, 300.0), (230.0, 300.0), |e| e);
    click(&mut s, 300.0, 200.0);
    assert_eq!(hover_cursor(&mut s, 200.0, 300.0), Cursor::PenConvert);
    click(&mut s, 200.0, 300.0);
    assert_eq!(hover_cursor(&mut s, 200.0, 300.0), Cursor::PenRemove);
    click(&mut s, 400.0, 200.0);
    enter(&mut s);
    let c = contours(&s, sel(&s)[0]);
    assert_eq!(c[0].nodes.len(), 4);
    let n = &c[0].nodes[1];
    assert!(n.pred == n.anchor && n.succ == n.anchor, "curve point became a corner: {n:?}");
    // a second click on the (now corner) point deletes it
    let mut s = session();
    click(&mut s, 100.0, 300.0);
    click(&mut s, 200.0, 300.0);
    click(&mut s, 300.0, 200.0);
    click(&mut s, 200.0, 300.0);
    click(&mut s, 400.0, 200.0);
    enter(&mut s);
    let a: Vec<(f32, f32)> = contours(&s, sel(&s)[0])[0].nodes.iter().map(|n| pt(n.anchor)).collect();
    assert_eq!(a, vec![(100.0, 300.0), (300.0, 200.0), (400.0, 200.0)]);
}

#[test]
fn pen_click_segment_while_drawing_inserts_point() {
    let mut s = session();
    click(&mut s, 100.0, 300.0);
    click(&mut s, 300.0, 300.0);
    click(&mut s, 300.0, 100.0);
    assert_eq!(hover_cursor(&mut s, 200.0, 301.0), Cursor::PenAdd);
    click(&mut s, 200.0, 301.0);
    enter(&mut s);
    let a: Vec<(f32, f32)> = contours(&s, sel(&s)[0])[0].nodes.iter().map(|n| pt(n.anchor)).collect();
    assert_eq!(a.len(), 4);
    assert_eq!(a[1], (200.0, 300.0), "inserted on the segment");
}

#[test]
fn pen_click_first_point_closes() {
    let mut s = session();
    click(&mut s, 100.0, 300.0);
    click(&mut s, 300.0, 300.0);
    click(&mut s, 300.0, 100.0);
    assert_eq!(hover_cursor(&mut s, 101.0, 299.0), Cursor::PenClose);
    click(&mut s, 101.0, 299.0);
    let c = contours(&s, sel(&s)[0]);
    assert!(c[0].closed);
    assert_eq!(c[0].nodes.len(), 3);
}

#[test]
fn pen_click_endpoint_of_other_open_path_joins() {
    let mut s = session();
    let v = ex(&mut s, "draw.path", json!({"contours": [{"closed": false, "nodes": [[400, 100], [500, 100]]}], "fill": false}));
    let other = ElementId(v["id"].as_u64().expect("id"));
    ex(&mut s, "select.none", json!({}));
    click(&mut s, 100.0, 100.0);
    click(&mut s, 200.0, 100.0);
    assert_eq!(hover_cursor(&mut s, 401.0, 100.0), Cursor::PenJoin);
    click(&mut s, 401.0, 100.0);
    assert_eq!(top_elements(&s).len(), 1, "joined into one path");
    let a: Vec<(f32, f32)> = contours(&s, other)[0].nodes.iter().map(|n| pt(n.anchor)).collect();
    assert_eq!(a, vec![(100.0, 100.0), (200.0, 100.0), (400.0, 100.0), (500.0, 100.0)]);
    assert_eq!(sel(&s), vec![other]);
    assert_eq!(label(&s).as_deref(), Some("Stift"));
    // drawing ended: the next click starts a new path
    click(&mut s, 100.0, 400.0);
    click(&mut s, 200.0, 400.0);
    enter(&mut s);
    assert_eq!(top_elements(&s).len(), 2);
}

#[test]
fn pen_click_endpoint_of_open_path_with_nothing_drawn_continues() {
    let mut s = session();
    let v = ex(&mut s, "draw.path", json!({"contours": [{"closed": false, "nodes": [[400, 100], [500, 100]]}], "fill": false}));
    let p = ElementId(v["id"].as_u64().expect("id"));
    ex(&mut s, "select.none", json!({}));
    assert_eq!(hover_cursor(&mut s, 500.0, 101.0), Cursor::PenContinue);
    click(&mut s, 500.0, 101.0);
    click(&mut s, 600.0, 200.0);
    enter(&mut s);
    assert_eq!(top_elements(&s).len(), 1);
    assert_eq!(contours(&s, p)[0].nodes.len(), 3);
}

#[test]
fn pen_hit_radius_follows_snap_distance() {
    let mut s = session();
    click(&mut s, 100.0, 300.0);
    click(&mut s, 250.0, 300.0);
    // 6 px away: outside the default 5 px, a new point
    assert_eq!(hover_cursor(&mut s, 256.0, 300.0), Cursor::Pen);
    s.prefs.set("SnapDistance", PrefValue::Int(8));
    assert_eq!(hover_cursor(&mut s, 256.0, 300.0), Cursor::PenConvert);
}
