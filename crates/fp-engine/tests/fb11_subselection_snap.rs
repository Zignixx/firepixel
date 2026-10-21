//! FB-11/FB-12 (re 55 §5, §7.3): Subselection point drags are free (resolution 1/zoom) while
//! View ▸ Snap to Pixels is off (the default) and round to whole document pixels when it is on;
//! no smart-guide snap for anchors (TW-175). Pointer moves and Pen points stay whole-pixel
//! regardless of the switch.

use crate::common;
use common::*;
use fp_engine::tools::ToolId;
use fp_engine::{Overlay, Session, ToolEvent, ToolKey};
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

/// Handles and anchor of the first node: (pred, anchor, succ).
type Node3 = ((f32, f32), (f32, f32), (f32, f32));

fn first_node(s: &Session, id: ElementId) -> Node3 {
    match &el(s, id).kind {
        ElementKind::Path(p) => {
            let n = &p.contours[0].nodes[0];
            (
                (n.pred.x, n.pred.y),
                (n.anchor.x, n.anchor.y),
                (n.succ.x, n.succ.y),
            )
        }
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

/// Open path whose first node is a curve point: a Pen drag at 10/30 pulls its handles out.
fn curve(s: &mut Session) -> ElementId {
    tool(s, ToolId::Pen);
    events(
        s,
        &[
            ToolEvent::down(10.0, 30.0),
            ToolEvent::drag(15.0, 30.0),
            ToolEvent::drag(20.0, 30.0),
            ToolEvent::up(20.0, 30.0),
            ToolEvent::down(50.0, 10.0),
            ToolEvent::up(50.0, 10.0),
            ToolEvent::key(ToolKey::Enter),
        ],
    );
    top_elements(s)[0].id
}

fn zoom(s: &mut Session, z: f64) {
    ex(s, "view.setZoom", json!({"zoom": z}));
}

fn drag(s: &mut Session, a: (f32, f32), b: (f32, f32)) {
    let _ = drag_feedback(s, a, b);
}

/// Like [`drag`]; returns the overlay shown at the last drag position (mouse still down).
fn drag_feedback(s: &mut Session, a: (f32, f32), b: (f32, f32)) -> Vec<Overlay> {
    events(
        s,
        &[
            ToolEvent::down(a.0, a.1),
            ToolEvent::drag((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0),
        ],
    );
    let fb = s
        .tool_event(&ToolEvent::drag(b.0, b.1))
        .unwrap_or_else(|e| panic!("{e}"))
        .overlay;
    events(s, &[ToolEvent::up(b.0, b.1)]);
    fb
}

fn snap_to_pixels(s: &Session) -> Option<bool> {
    s.command_state("view.snapToPixels").1
}

fn smart_guides(fb: &[Overlay]) -> usize {
    fb.iter()
        .filter(|o| matches!(o, Overlay::SmartGuide { .. }))
        .count()
}

#[test]
fn snap_to_pixels_is_off_by_default() {
    let s = doc_session(60.0, 40.0);
    assert_eq!(snap_to_pixels(&s), Some(false));
    assert!(!fp_engine::tools::snap::snap_to_pixels(&s.prefs));
    assert!(!s.prefs.get_bool("SnapToPixelGrid", false));
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
fn subselection_drag_free_with_switch_off_whole_pixels_with_switch_on() {
    let mut s = doc_session(60.0, 40.0);
    let id = triangle(&mut s);
    tool(&mut s, ToolId::Subselection);
    zoom(&mut s, 16.0);
    // off (default): 1/16 steps
    assert_eq!(snap_to_pixels(&s), Some(false));
    drag(&mut s, (50.0, 30.0), (50.125, 31.3125));
    let a = anchors(&s, id);
    assert!(approx(a[2].0, 50.125) && approx(a[2].1, 31.3125), "{a:?}");
    s.undo().expect("undo");
    // on: the dragged point lands on whole document pixels
    ex(&mut s, "view.snapToPixels", json!({}));
    assert_eq!(snap_to_pixels(&s), Some(true));
    assert!(s.prefs.get_bool("SnapToPixelGrid", false));
    drag(&mut s, (50.0, 30.0), (50.125, 31.3125));
    assert_eq!(anchors(&s, id)[2], (50.0, 31.0));
    drag(&mut s, (50.0, 31.0), (51.6875, 29.4375));
    assert_eq!(anchors(&s, id)[2], (52.0, 29.0));
    assert_eq!(anchors(&s, id)[0], (10.0, 30.0));
    // off again: free once more
    ex(&mut s, "view.snapToPixels", json!({}));
    assert_eq!(snap_to_pixels(&s), Some(false));
    drag(&mut s, (52.0, 29.0), (52.3125, 29.0625));
    let a = anchors(&s, id);
    assert!(approx(a[2].0, 52.3125) && approx(a[2].1, 29.0625), "{a:?}");
}

#[test]
fn subselection_handle_drag_follows_snap_to_pixels() {
    let mut s = doc_session(60.0, 40.0);
    let id = curve(&mut s);
    let (pred, anchor, succ) = first_node(&s, id);
    assert_eq!(anchor, (10.0, 30.0));
    assert_ne!(pred, succ, "Pen drag made handles");
    assert_ne!(succ, anchor, "Pen drag made handles");
    tool(&mut s, ToolId::Subselection);
    zoom(&mut s, 16.0);
    // select the anchor so its handles can be grabbed
    events(
        &mut s,
        &[ToolEvent::down(10.0, 30.0), ToolEvent::up(10.0, 30.0)],
    );
    let to = (succ.0 + 0.4375, succ.1 - 1.1875);
    // off: the handle moves in 1/16 steps
    drag(&mut s, succ, to);
    let n = first_node(&s, id);
    assert!(approx(n.2.0, to.0) && approx(n.2.1, to.1), "{n:?}");
    assert_eq!(n.1, anchor);
    s.undo().expect("undo");
    assert_eq!(first_node(&s, id).2, succ);
    // on: the handle lands on whole pixels
    ex(&mut s, "view.snapToPixels", json!({}));
    drag(&mut s, succ, to);
    let n = first_node(&s, id);
    assert_eq!(n.2, (to.0.round(), to.1.round()), "{n:?}");
    assert_eq!(n.1, anchor);
}

/// re 55 §7.3 (1600 %): rectangle 36/10–50/24; anchor targets 2 screen px from its bottom edge,
/// its centre and its left edge stay exactly where the mouse put them, no guide line.
#[test]
fn subselection_anchor_ignores_smart_guides_at_zoom_16() {
    let mut s = doc_session(60.0, 40.0);
    rect(&mut s, 36.0, 10.0, 50.0, 24.0);
    let id = triangle(&mut s);
    assert!(
        s.active()
            .is_some_and(|d| d.view.snap_to_smart_guides && d.view.show_smart_guides)
    );
    tool(&mut s, ToolId::Subselection);
    zoom(&mut s, 16.0);
    let mut from = (10.0, 30.0);
    for to in [(25.3125, 24.125), (25.3125, 17.125), (35.875, 12.3125)] {
        let fb = drag_feedback(&mut s, from, to);
        let a = anchors(&s, id)[0];
        assert!(approx(a.0, to.0) && approx(a.1, to.1), "{to:?} -> {a:?}");
        assert_eq!(smart_guides(&fb), 0, "{fb:?}");
        from = to;
    }
}

/// re 55 §7.3 (100 %): no snap to the canvas centre or another object's edge either; the
/// Pointer still snaps (counter-check).
#[test]
fn subselection_anchor_ignores_smart_guides_at_zoom_1() {
    let mut s = doc_session(200.0, 150.0);
    let r = rect(&mut s, 120.0, 20.0, 160.0, 60.0);
    let id = triangle(&mut s);
    tool(&mut s, ToolId::Subselection);
    zoom(&mut s, 1.0);
    let mut from = (10.0, 30.0);
    for to in [(80.0, 62.0), (80.0, 77.0), (117.0, 85.0)] {
        let fb = drag_feedback(&mut s, from, to);
        assert_eq!(anchors(&s, id)[0], to);
        assert_eq!(smart_guides(&fb), 0, "{fb:?}");
        from = to;
    }
    // counter-check: the Pointer snaps the rectangle's bottom edge to the canvas centre (y 75)
    tool(&mut s, ToolId::Pointer);
    select(&mut s, &[r]);
    let fb = drag_feedback(&mut s, (140.0, 40.0), (140.0, 52.0));
    assert!(approx(bounds(&s, r).bottom, 75.0), "{:?}", bounds(&s, r));
    assert!(smart_guides(&fb) > 0, "{fb:?}");
}

#[test]
fn pointer_move_and_pen_whole_pixels_by_default() {
    let mut s = doc_session(300.0, 200.0);
    zoom(&mut s, 16.0);
    assert_eq!(snap_to_pixels(&s), Some(false));
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

/// The switch only drives point drags: Pointer moves keep floor(end) − floor(start) in document
/// pixels (re 55 §7.4) and Pen clicks stay whole-pixel, with the switch on or off.
#[test]
fn pointer_move_and_pen_unchanged_by_snap_to_pixels() {
    for on in [false, true] {
        let mut s = doc_session(300.0, 200.0);
        zoom(&mut s, 16.0);
        if on {
            ex(&mut s, "view.snapToPixels", json!({}));
        }
        assert_eq!(snap_to_pixels(&s), Some(on));
        let r = rect(&mut s, 10.0, 10.0, 20.0, 20.0);
        tool(&mut s, ToolId::Pointer);
        select(&mut s, &[r]);
        let b0 = bounds(&s, r);
        drag(&mut s, (15.2, 15.2), (16.75, 14.47));
        let b = bounds(&s, r);
        assert!(
            approx(b.left - b0.left, 1.0) && approx(b.top - b0.top, -1.0),
            "on={on}: {b0:?} -> {b:?}"
        );
        // floor rule, not rounding of the mouse path: 15.9375 → 17.0 (+17 screen px, above the
        // 12 px threshold at 1600 %) moves by +2, not by round(1.0625) = 1
        drag(&mut s, (15.9375, 15.9), (17.0, 15.9));
        assert!(approx(bounds(&s, r).left - b0.left, 3.0), "on={on}");
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
        assert_eq!(anchors(&s, id), vec![(54.0, 35.0), (57.0, 36.0)], "on={on}");
    }
}
