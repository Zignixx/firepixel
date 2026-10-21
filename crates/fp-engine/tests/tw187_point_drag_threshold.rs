//! TW-187 (re 55 §7.3): a Subselection point drag starts at the same per-zoom threshold as a
//! Pointer move (TW-176): travel > 3·√zoom screen px. Measured at 50 %: an anchor drag of (1,1)
//! screen px writes no row, (−2,−2) writes "Move Point" with a move of −4/−4 document px (free
//! drag at 1/zoom, View ▸ Snap to Pixels off). The measurement says nothing about a point drag
//! that returns to its start, so that keeps "no row" (unlike the Pointer's zero move).

use crate::common;
use common::*;
use fp_engine::tools::ToolId;
use fp_engine::{Session, ToolEvent};
use fp_model::{ElementId, ElementKind};
use serde_json::json;

/// Closed triangle (x0/30, 30/8, 50/30) drawn with the Pen; returns the path and switches to
/// the Subselection tool at `zoom` with the first anchor clicked (sub-selected).
fn setup(x0: f32, zoom: f64) -> (Session, ElementId) {
    let mut s = doc_session(200.0, 150.0);
    s.set_tool(ToolId::Pen).unwrap_or_else(|e| panic!("{e}"));
    let mut evs = Vec::new();
    for (x, y) in [(x0, 30.0), (30.0, 8.0), (50.0, 30.0), (x0, 30.0)] {
        evs.push(ToolEvent::down(x, y));
        evs.push(ToolEvent::up(x, y));
    }
    events(&mut s, &evs);
    let id = top_elements(&s)[0].id;
    s.set_tool(ToolId::Subselection)
        .unwrap_or_else(|e| panic!("{e}"));
    ex(&mut s, "view.setZoom", json!({"zoom": zoom}));
    events(
        &mut s,
        &[ToolEvent::down(x0, 30.0), ToolEvent::up(x0, 30.0)],
    );
    assert_eq!(anchor(&s, id), (x0, 30.0));
    (s, id)
}

fn anchor(s: &Session, id: ElementId) -> (f32, f32) {
    match &el(s, id).kind {
        ElementKind::Path(p) => {
            let a = p.contours[0].nodes[0].anchor;
            (a.x, a.y)
        }
        _ => panic!("not a path"),
    }
}

fn rows(s: &Session) -> usize {
    s.active().map(|d| d.history.steps().len()).unwrap_or(0)
}

/// Drags from `a` through `steps` intermediate events to `b` (document px), optionally via
/// `via` first; returns the number of history rows written.
fn drag(
    s: &mut Session,
    a: (f32, f32),
    b: (f32, f32),
    via: Option<(f32, f32)>,
    steps: usize,
) -> usize {
    let n0 = rows(s);
    let mut evs = vec![ToolEvent::down(a.0, a.1)];
    let mut from = a;
    for to in via.into_iter().chain([b]) {
        for i in 1..=steps {
            let t = i as f32 / steps as f32;
            evs.push(ToolEvent::drag(
                from.0 + (to.0 - from.0) * t,
                from.1 + (to.1 - from.1) * t,
            ));
        }
        from = to;
    }
    evs.push(ToolEvent::up(b.0, b.1));
    events(s, &evs);
    rows(s) - n0
}

fn assert_move_point_row(s: &Session) {
    let label = s.undo_redo_labels().0;
    assert!(
        matches!(label.as_deref(), Some("Punkt verschieben" | "Move Point")),
        "{label:?}"
    );
    let d = s.active().expect("doc");
    let st = d.history.steps();
    let t = st.last().expect("row");
    assert_eq!(t.command, "path.setContours");
    assert_eq!(t.params["step"], "move-point");
}

#[test]
fn anchor_drag_at_50_percent_follows_the_measurement() {
    for steps in [1, 2, 20] {
        let (mut s, id) = setup(10.0, 0.5);
        // (1,1) screen px = (2,2) document px: √2 ≈ 1.41 ≤ 2.12 → no row, no move
        assert_eq!(
            drag(&mut s, (10.0, 30.0), (12.0, 32.0), None, steps),
            0,
            "{steps}"
        );
        assert_eq!(anchor(&s, id), (10.0, 30.0));
        // (−2,−2) screen px: 2.83 > 2.12 → one row, the anchor moves by −4/−4 document px
        assert_eq!(
            drag(&mut s, (10.0, 30.0), (6.0, 26.0), None, steps),
            1,
            "{steps}"
        );
        assert_move_point_row(&s);
        assert_eq!(anchor(&s, id), (6.0, 26.0));
        s.undo().expect("undo");
        assert_eq!(anchor(&s, id), (10.0, 30.0));
    }
}

#[test]
fn threshold_per_zoom_for_anchor_drags() {
    // (zoom, largest travel without a row, smallest travel with a row) in screen px
    for (zoom, no, yes) in [
        (1.0, 3.0, 4.0),
        (2.0, 4.0, 5.0),
        (4.0, 6.0, 7.0),
        (16.0, 12.0, 13.0),
    ] {
        let (mut s, id) = setup(10.0, zoom);
        let z = zoom as f32;
        assert_eq!(
            drag(&mut s, (10.0, 30.0), (10.0 + no / z, 30.0), None, 4),
            0,
            "{zoom} {no}"
        );
        assert_eq!(anchor(&s, id), (10.0, 30.0));
        assert_eq!(
            drag(&mut s, (10.0, 30.0), (10.0 + yes / z, 30.0), None, 4),
            1,
            "{zoom} {yes}"
        );
        assert_move_point_row(&s);
        let a = anchor(&s, id);
        assert!(
            approx(a.0, 10.0 + yes / z) && approx(a.1, 30.0),
            "{zoom}: {a:?}"
        );
    }
}

/// 1600 %: the measured 15.9375 → 17.0 (+17 screen px) still drags the anchor at 16/30, by the
/// exact mouse path (1/16 steps, no whole-pixel rounding with the switch off).
#[test]
fn anchor_drag_at_1600_percent_still_works() {
    let (mut s, id) = setup(16.0, 16.0);
    assert_eq!(drag(&mut s, (15.9375, 30.0), (17.0, 30.0), None, 20), 1);
    assert_move_point_row(&s);
    let a = anchor(&s, id);
    assert!(approx(a.0, 17.0625) && approx(a.1, 30.0), "{a:?}");
}

/// A point drag past the threshold that returns to its start writes no row (not measured for
/// anchors; the Pointer's zero-move row of TW-176 is not carried over).
#[test]
fn zero_point_move_writes_no_row() {
    for zoom in [0.5, 1.0, 16.0] {
        let (mut s, id) = setup(10.0, zoom);
        let far = (10.0 + 20.0 / zoom as f32, 30.0);
        assert_eq!(
            drag(&mut s, (10.0, 30.0), (10.0, 30.0), Some(far), 5),
            0,
            "{zoom}"
        );
        assert_eq!(anchor(&s, id), (10.0, 30.0));
    }
}
