//! FB-63 (TW-239): View ▸ Play/Stop toggles `ui.playing`; the playback's state switch
//! (`Session::show_state`) behaves like `state.setCurrent` but writes no History row.

use crate::common;
use common::*;
use serde_json::json;

#[test]
fn toggle_playback_flips_playing() {
    let mut s = doc_session(100.0, 100.0);
    assert!(!s.ui.playing);
    ex(&mut s, "view.togglePlayback", json!({}));
    assert!(s.ui.playing);
    ex(&mut s, "view.togglePlayback", json!({}));
    assert!(!s.ui.playing);
}

#[test]
fn show_state_switches_without_history_row() {
    let mut s = doc_session(100.0, 100.0);
    ex(
        &mut s,
        "panel.states.add",
        json!({"count": 2, "where": "end"}),
    );
    ex(&mut s, "state.setCurrent", json!({"index": 0}));
    rect(&mut s, 10.0, 10.0, 40.0, 40.0);
    ex(&mut s, "select.all", json!({}));
    let rows = |s: &fp_engine::Session| {
        let d = s.active().expect("doc");
        d.history.len() + d.history.dropped()
    };
    let before = rows(&s);
    assert!(s.show_state(1));
    assert!(s.show_state(2));
    assert!(!s.show_state(2), "already current");
    assert!(!s.show_state(3), "out of range");
    assert!(s.show_state(0));
    let d = s.active().expect("doc");
    assert_eq!(d.frame, 0);
    assert!(
        d.selection.is_empty(),
        "selection cleared like state.setCurrent"
    );
    assert_eq!(rows(&s), before, "no History rows");
}
