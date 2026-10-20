//! FB-11 (re 55 §5): Subselection point drags are free (resolution 1/zoom, no whole-pixel
//! rounding); Pointer moves and Pen points stay whole-pixel unless View ▸ Snap to Pixels is off.

use crate::common;
use common::*;
use fp_engine::tools::ToolId;
use fp_engine::{Session, ToolEvent, ToolKey};
use fp_model::{ElementId, ElementKind};
use serde_json::json;

fn tool(s: &mut Session, t: ToolId) {
    s.set_tool(t).unwrap_or_else(|e| panic!("{e}"));
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

/// Closed triangle (10/30, 30/8, 50/30) drawn with the Pen like in the measurement.
fn triangle(s: &mut Session) -> ElementId {
    tool(s, ToolId::Pen);
    let mut evs = Vec::new();
    for (x, y) in [(10.0, 30.0), (30.0, 8.0), (50.0, 30.0), (10.0, 30.0)] {
        evs.push(ToolEvent::down(x, y));
        evs.push(ToolEvent::up(x, y));
    }
    events(s, &evs);
    top_elements(s)[0].id
}

fn zoom(s: &mut Session, z: f64) {
    ex(s, "view.setZoom", json!({"zoom": z}));
}

fn drag(s: &mut Session, a: (f32, f32), b: (f32, f32)) {
    events(
        s,
        &[
            ToolEvent::down(a.0, a.1),
            ToolEvent::drag((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0),
            ToolEvent::drag(b.0, b.1),
            ToolEvent::up(b.0, b.1),
        ],
    );
}

#[test]
fn subselection_anchor_drag_is_fractional_at_zoom_16() {
    let mut s = doc_session(60.0, 40.0);
    let id = triangle(&mut s);
    tool(&mut s, ToolId::Subselection);
    zoom(&mut s, 16.0);
    // re 55 §5.2: moveSelectedBezierPointsBy({x:1.6875, y:-1.25}) → 11.6875 / 28.75
    drag(&mut s, (10.0, 30.0), (11.6875, 28.75));
    let a = anchors(&s, id);
    assert!(
        approx(a[0].0, 11.6875) && approx(a[0].1, 28.75),
        "anchor {a:?}"
    );
    assert_eq!((a[1], a[2]), ((30.0, 8.0), (50.0, 30.0)));
    // one history step "Move Point"
    let label = s.undo_redo_labels().0;
    assert!(
        matches!(label.as_deref(), Some("Punkt verschieben" | "Move Point")),
        "{label:?}"
    );
    s.undo().expect("undo");
    assert_eq!(anchors(&s, id)[0], (10.0, 30.0));
}

#[test]
fn subselection_drag_stays_free_with_snap_to_pixels_on_or_off() {
    let mut s = doc_session(60.0, 40.0);
    let id = triangle(&mut s);
    tool(&mut s, ToolId::Subselection);
    zoom(&mut s, 16.0);
    ex(&mut s, "view.snapToPixels", json!({}));
    ex(&mut s, "view.snapToPixels", json!({}));
    assert_eq!(s.command_state("view.snapToPixels").1, Some(true));
    drag(&mut s, (50.0, 30.0), (50.125, 31.3125));
    let a = anchors(&s, id);
    assert!(approx(a[2].0, 50.125) && approx(a[2].1, 31.3125), "{a:?}");
}

#[test]
fn pointer_move_and_pen_whole_pixels_by_default() {
    let mut s = doc_session(300.0, 200.0);
    zoom(&mut s, 16.0);
    assert_eq!(s.command_state("view.snapToPixels").1, Some(true));
    let r = rect(&mut s, 10.0, 10.0, 20.0, 20.0);
    tool(&mut s, ToolId::Pointer);
    select(&mut s, &[r]);
    let b0 = bounds(&s, r);
    drag(&mut s, (15.2, 15.2), (16.75, 14.47));
    let b = bounds(&s, r);
    assert!(
        approx(b.left - b0.left, 1.0) && approx(b.top - b0.top, -1.0),
        "{b0:?} -> {b:?}"
    );
    // Pen clicks round to whole pixels (53.66/34.6 → 54/35)
    tool(&mut s, ToolId::Pen);
    events(
        &mut s,
        &[
            ToolEvent::down(53.66, 34.6),
            ToolEvent::up(53.66, 34.6),
            ToolEvent::down(57.2, 36.3),
            ToolEvent::up(57.2, 36.3),
            ToolEvent::key(ToolKey::Enter),
        ],
    );
    let id = top_elements(&s)[0].id;
    assert_eq!(anchors(&s, id), vec![(54.0, 35.0), (57.0, 36.0)]);
}

#[test]
fn snap_to_pixels_off_gives_fractional_pointer_moves() {
    let mut s = doc_session(300.0, 200.0);
    zoom(&mut s, 16.0);
    ex(&mut s, "view.snapToPixels", json!({}));
    assert_eq!(s.command_state("view.snapToPixels").1, Some(false));
    assert!(!s.prefs.get_bool("SnapToPixelGrid", true));
    let r = rect(&mut s, 10.0, 10.0, 20.0, 20.0);
    tool(&mut s, ToolId::Pointer);
    select(&mut s, &[r]);
    let b0 = bounds(&s, r);
    drag(&mut s, (15.2, 15.2), (16.75, 14.47));
    let b = bounds(&s, r);
    assert!(
        approx(b.left - b0.left, 1.55) && approx(b.top - b0.top, -0.73),
        "{b0:?} -> {b:?}"
    );
    // switching back restores the default
    ex(&mut s, "view.snapToPixels", json!({}));
    assert!(s.prefs.get_bool("SnapToPixelGrid", false));
}
