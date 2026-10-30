//! F148 (TW-283, Firepixel's own switch): View ▸ Gap Measurements. While an object is moved with
//! smart guides on, the gap measurement lines (GuideLine + distance label to the neighbours) only
//! appear with the switch; off by default. The smart guide lines and the snap are the same either
//! way.

use crate::common;
use common::*;
use fp_engine::tools::ToolId;
use fp_engine::{Overlay, Session, ToolEvent};
use serde_json::json;

const CMD: &str = "view.toggleGapMeasurements";

#[test]
fn gap_measurements_toggle_is_off_by_default_and_flips_the_pref() {
    let mut s = doc_session(200.0, 100.0);
    assert_eq!(s.command_state(CMD).1, Some(false));
    assert!(!fp_engine::tools::snap::gap_measurements(&s.prefs));
    ex(&mut s, CMD, json!({}));
    assert_eq!(s.command_state(CMD).1, Some(true));
    assert!(s.prefs.get_bool("GapMeasurements", false));
    ex(&mut s, CMD, json!({}));
    assert_eq!(s.command_state(CMD).1, Some(false));
    assert!(!s.prefs.get_bool("GapMeasurements", true));
}

/// Moves the right rectangle 10 px left and 1 px down (its top snaps back to the left one's top);
/// returns the mid-drag feedback and the final bounds.
fn move_drag(s: &mut Session) -> (Vec<Overlay>, fp_geom::Rect) {
    rect(s, 10.0, 10.0, 50.0, 50.0);
    let b = rect(s, 100.0, 10.0, 140.0, 50.0);
    s.set_tool(ToolId::Pointer).unwrap_or_else(|e| panic!("{e}"));
    events(
        s,
        &[
            ToolEvent::down(120.0, 30.0),
            ToolEvent::drag(116.0, 30.0),
            ToolEvent::drag(113.0, 31.0),
        ],
    );
    let fb = s
        .tool_event(&ToolEvent::drag(110.0, 31.0))
        .unwrap_or_else(|e| panic!("{e}"))
        .overlay;
    events(s, &[ToolEvent::up(110.0, 31.0)]);
    (fb, bounds(s, b))
}

fn gap_lines(fb: &[Overlay]) -> usize {
    fb.iter()
        .filter(|o| matches!(o, Overlay::GuideLine { .. }))
        .count()
}

fn smart(fb: &[Overlay]) -> Vec<Overlay> {
    fb.iter()
        .filter(|o| matches!(o, Overlay::SmartGuide { .. }))
        .cloned()
        .collect()
}

#[test]
fn move_shows_gap_lines_only_with_the_switch() {
    let mut s = doc_session(200.0, 100.0);
    let (off, b_off) = move_drag(&mut s);
    let mut s = doc_session(200.0, 100.0);
    ex(&mut s, CMD, json!({}));
    let (on, b_on) = move_drag(&mut s);
    assert!(
        rect_approx(b_off, fp_geom::Rect::new(90.0, 10.0, 130.0, 50.0)),
        "{b_off:?}"
    );
    assert_eq!(b_off, b_on, "same snap either way");
    assert!(!smart(&off).is_empty(), "smart guides stay: {off:?}");
    assert_eq!(smart(&off), smart(&on));
    assert_eq!(gap_lines(&off), 0, "default off: no gap lines {off:?}");
    assert!(
        !off.iter()
            .any(|o| matches!(o, Overlay::Label { text, .. } if text == "40")),
        "{off:?}"
    );
    assert!(gap_lines(&on) >= 1, "switch on: gap line {on:?}");
    assert!(
        on.iter()
            .any(|o| matches!(o, Overlay::Label { text, .. } if text == "40")),
        "switch on: distance label {on:?}"
    );
}
