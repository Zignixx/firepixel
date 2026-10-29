//! FB-97 / TW-272: Window ▸ Duplicate Window splits the document window into two panes, each with
//! its own page, state, zoom, scroll and selection; both show the same document. F137 / TW-275: up
//! to four panes. The focused pane's
//! state is the live one (`d.view`, `d.page` …), so commands, panels and MCP address it unchanged.

use crate::common;
use common::*;
use fp_engine::Session;
use serde_json::json;

/// Two pages, a rectangle on each; page 0 current.
fn two_pages() -> Session {
    fp_i18n::set_language("en");
    let mut s = doc_session(400.0, 300.0);
    rect(&mut s, 10.0, 10.0, 60.0, 60.0);
    ex(&mut s, "page.add", json!({}));
    ex(&mut s, "page.setCurrent", json!({"index": 1}));
    rect(&mut s, 100.0, 100.0, 150.0, 150.0);
    ex(&mut s, "page.setCurrent", json!({"index": 0}));
    s
}

#[test]
fn duplicate_window_splits_with_an_independent_page_and_zoom() {
    let mut s = two_pages();
    s.active_mut().expect("doc").view.zoom = 2.0;
    let v = ex(&mut s, "window.duplicateWindow", json!({}));
    assert_eq!(v["panes"], 2, "{v}");
    // the new right pane has the focus and starts as a copy of the left one
    let d = s.active().expect("doc");
    assert!(d.is_split());
    assert_eq!(d.live_pane(), 1);
    assert_eq!((d.page, d.view.zoom), (0, 2.0));
    // the right pane switches its page and zooms; the left pane keeps page 0 and 200 %
    ex(&mut s, "page.setCurrent", json!({"index": 1}));
    ex(&mut s, "view.zoomIn", json!({}));
    let d = s.active().expect("doc");
    assert_eq!(d.page, 1);
    let right_zoom = d.view.zoom;
    assert!(right_zoom > 2.0);
    let left = d.other_pane().expect("left pane");
    assert_eq!((left.page, left.view.zoom), (0, 2.0));
    // focus back to the left pane: its page and zoom are live again
    let v = ex(&mut s, "window.focusPane", json!({"pane": 0}));
    assert_eq!((v["pane"].as_u64(), v["page"].as_u64()), (Some(0), Some(0)));
    let d = s.active().expect("doc");
    assert_eq!((d.page, d.view.zoom), (0, 2.0));
    assert_eq!(d.other_pane().map(|p| p.page), Some(1));
    assert_eq!(d.other_pane().map(|p| p.view.zoom), Some(right_zoom));
    // F137: a second Duplicate Window splits the focused pane again: a third pane after it
    let v = ex(&mut s, "window.duplicateWindow", json!({}));
    assert_eq!((v["panes"].as_u64(), v["pane"].as_u64()), (Some(3), Some(1)));
    let d = s.active().expect("doc");
    assert_eq!((d.page, d.view.zoom), (0, 2.0), "a copy of the focused pane");
    assert_eq!(d.parked_pane(2).map(|p| p.page), Some(1), "the old right pane");
}

#[test]
fn an_edit_in_one_pane_shows_in_the_other() {
    let mut s = two_pages();
    ex(&mut s, "window.duplicateWindow", json!({}));
    // both panes on page 0: a rectangle drawn in the right pane is in the document the left shows
    let id = rect(&mut s, 200.0, 20.0, 260.0, 80.0);
    ex(&mut s, "window.focusPane", json!({"pane": 0}));
    let d = s.active().expect("doc");
    assert!(fp_engine::docops::find_on_page(&d.doc, d.page, id).is_some());
    // the right pane deletes page 1 while the left pane is on it: the left pane falls back to the
    // last page when it gets the focus again
    ex(&mut s, "page.setCurrent", json!({"index": 1}));
    ex(&mut s, "window.focusPane", json!({"pane": 1}));
    ex(&mut s, "page.setCurrent", json!({"index": 1}));
    ex(&mut s, "page.delete", json!({}));
    ex(&mut s, "window.focusPane", json!({"pane": 0}));
    let d = s.active().expect("doc");
    assert_eq!(d.doc.pages.len(), 1);
    assert_eq!(d.page, 0);
    assert!(d.layer.is_some());
}

#[test]
fn each_pane_keeps_its_selection_and_stale_ids_are_dropped() {
    let mut s = two_pages();
    let a = rect(&mut s, 200.0, 20.0, 260.0, 80.0);
    ex(&mut s, "window.duplicateWindow", json!({}));
    ex(&mut s, "select.none", json!({}));
    ex(&mut s, "window.focusPane", json!({"pane": 0}));
    // the left pane kept the rectangle selected
    assert_eq!(
        s.active().map(|d| d.selection.elements.clone()),
        Some(vec![a])
    );
    ex(&mut s, "window.focusPane", json!({"pane": 1}));
    select(&mut s, &[a]);
    ex(&mut s, "edit.clear", json!({}));
    // deleted in the right pane: the left pane's selection no longer names it
    ex(&mut s, "window.focusPane", json!({"pane": 0}));
    assert!(s.active().is_some_and(|d| d.selection.elements.is_empty()));
}

/// F137: Close Duplicate Window closes the focused pane; the remaining pane fills the window.
#[test]
fn close_duplicate_closes_the_focused_pane() {
    let mut s = two_pages();
    assert!(s.execute("window.closeDuplicate", json!({})).is_err());
    assert!(s.execute("window.focusPane", json!({"pane": 1})).is_err());
    ex(&mut s, "window.duplicateWindow", json!({}));
    ex(&mut s, "page.setCurrent", json!({"index": 1}));
    let v = ex(&mut s, "window.closeDuplicate", json!({}));
    assert_eq!(v["closed"], true, "{v}");
    let d = s.active().expect("doc");
    assert!(!d.is_split());
    assert_eq!((d.page, d.live_pane()), (0, 0), "the left pane on page 0 stays");
    // with `pane` another pane closes and the focused one stays
    ex(&mut s, "window.duplicateWindow", json!({}));
    ex(&mut s, "page.setCurrent", json!({"index": 1}));
    let v = ex(&mut s, "window.closeDuplicate", json!({"pane": 0}));
    assert_eq!((v["panes"].as_u64(), v["page"].as_u64()), (Some(1), Some(1)));
    assert!(s.active().is_some_and(|d| !d.is_split()));
    assert!(s.execute("window.closeDuplicate", json!({"pane": 0})).is_err());
}

/// F137: up to four panes; each keeps its own page and zoom, Next Pane cycles all of them, a
/// closed pane in the middle gives its space to the pane before it and the focus stays where it
/// is; at four panes Duplicate Window is unavailable.
#[test]
fn up_to_four_panes_each_with_its_own_view() {
    let mut s = two_pages();
    ex(&mut s, "window.duplicateWindow", json!({}));
    ex(&mut s, "window.duplicateWindow", json!({}));
    ex(&mut s, "window.duplicateWindow", json!({}));
    let d = s.active().expect("doc");
    assert_eq!((d.pane_count(), d.live_pane()), (4, 3));
    assert!(!s.command_state("window.duplicateWindow").0, "four is the most");
    assert!(s.execute("window.duplicateWindow", json!({})).is_err());
    let d = s.active().expect("doc");
    // the splitters: four panes, increasing, every pane at least 10 %
    let spans = d.split_layout.spans(4);
    assert_eq!(spans.len(), 4);
    assert!(spans.iter().all(|(a, b)| b - a >= 0.099), "{spans:?}");
    assert_eq!(d.split_layout.bounds.len(), 3);
    // give every pane its own page and zoom
    for p in 0..4usize {
        ex(&mut s, "window.focusPane", json!({"pane": p}));
        ex(&mut s, "page.setCurrent", json!({"index": p % 2}));
        s.active_mut().expect("doc").view.zoom = 1.0 + p as f64;
    }
    for p in 0..4usize {
        let d = s.active().expect("doc");
        assert_eq!(
            d.pane_summary(p).map(|(page, _, z)| (page, z)),
            Some((p % 2, 1.0 + p as f64)),
            "pane {p}"
        );
    }
    // Next Pane cycles 3 -> 0 -> 1 -> 2 -> 3
    let mut seen = Vec::new();
    for _ in 0..4 {
        let v = ex(&mut s, "window.nextPane", json!({}));
        seen.push(v["pane"].as_u64().unwrap_or(9));
    }
    assert_eq!(seen, vec![0, 1, 2, 3]);
    assert!(s.execute("window.focusPane", json!({"pane": 4})).is_err());
    // close pane 1 (not focused): pane 3 keeps the focus, now as pane 2
    let bounds = s
        .active()
        .map(|d| d.split_layout.bounds.clone())
        .expect("doc");
    let v = ex(&mut s, "window.closeDuplicate", json!({"pane": 1}));
    assert_eq!((v["panes"].as_u64(), v["pane"].as_u64()), (Some(3), Some(2)));
    let d = s.active().expect("doc");
    assert_eq!(d.view.zoom, 4.0);
    assert_eq!(d.pane_summary(1).map(|(_, _, z)| z), Some(3.0));
    assert_eq!(d.split_layout.bounds, vec![bounds[1], bounds[2]], "pane 0 grew");
    // the focused pane closes: the focus goes to the pane before it
    let v = ex(&mut s, "window.closeDuplicate", json!({}));
    assert_eq!((v["panes"].as_u64(), v["pane"].as_u64()), (Some(2), Some(1)));
    assert_eq!(s.active().map(|d| d.view.zoom), Some(3.0));
    assert!(s.command_state("window.duplicateWindow").0);
}

/// F137: the splitters keep their distance (10 % per pane) and are remembered for the document;
/// splitting a pane halves it.
#[test]
fn splitter_positions_are_clamped_and_remembered() {
    use fp_engine::view_pane::SplitLayout;
    let mut l = SplitLayout::default();
    assert_eq!(l.fitted_bounds(2), vec![0.5]);
    assert!(l.set_bound(2, 0, 0.7));
    assert_eq!((l.bounds.clone(), l.ratio), (vec![0.7], 0.7));
    assert!(l.set_bound(2, 0, 0.99));
    assert!((l.bounds[0] - 0.9).abs() < 1e-6);
    assert!(!l.set_bound(2, 1, 0.5), "no second splitter");
    assert_eq!(l.fitted_bounds(3).len(), 2, "stored bounds for another count: even");
    let mut s = two_pages();
    ex(&mut s, "window.duplicateWindow", json!({}));
    s.active_mut()
        .expect("doc")
        .split_layout
        .set_bound(2, 0, 0.6);
    // the right pane (0.6 .. 1.0) is split in the middle
    ex(&mut s, "window.duplicateWindow", json!({}));
    let b = s
        .active()
        .map(|d| d.split_layout.bounds.clone())
        .expect("doc");
    assert_eq!(b.len(), 2);
    assert!((b[0] - 0.6).abs() < 1e-6 && (b[1] - 0.8).abs() < 1e-6, "{b:?}");
    // the middle splitter cannot pass its neighbours
    let d = s.active_mut().expect("doc");
    d.split_layout.set_bound(3, 1, 0.1);
    assert!((d.split_layout.bounds[1] - 0.7).abs() < 1e-6);
    // closed down to one pane and split again: two panes at the last two-pane position
    ex(&mut s, "window.closeDuplicate", json!({}));
    ex(&mut s, "window.closeDuplicate", json!({}));
    let d = s.active().expect("doc");
    assert!(!d.is_split());
    let ratio = d.split_layout.ratio;
    ex(&mut s, "window.duplicateWindow", json!({}));
    assert_eq!(
        s.active().map(|d| d.split_layout.bounds.clone()),
        Some(vec![ratio])
    );
}

#[test]
fn swap_pane_exchanges_the_live_state_as_it_is() {
    let mut s = two_pages();
    ex(&mut s, "window.duplicateWindow", json!({}));
    ex(&mut s, "page.setCurrent", json!({"index": 1}));
    let d = s.active_mut().expect("doc");
    d.swap_pane();
    assert_eq!((d.live_pane(), d.page), (0, 0));
    d.swap_pane();
    assert_eq!((d.live_pane(), d.page), (1, 1));
}

/// F132: `window.splitOrientation` arranges the panes one above the other; both keep their page,
/// zoom and focus, and the orientation stays with the document after the split is closed.
#[test]
fn orientation_switch_keeps_the_panes_and_is_remembered() {
    use fp_engine::view_pane::SplitOrientation;
    let mut s = two_pages();
    ex(&mut s, "window.duplicateWindow", json!({}));
    ex(&mut s, "page.setCurrent", json!({"index": 1}));
    ex(&mut s, "view.zoomIn", json!({}));
    let (page, zoom) = s.active().map(|d| (d.page, d.view.zoom)).expect("doc");
    let other = s.active().and_then(|d| d.other_pane().cloned());
    assert_eq!(
        s.active().map(|d| d.split_layout.orientation),
        Some(SplitOrientation::Vertical),
        "Duplicate Window starts side by side"
    );
    let v = ex(
        &mut s,
        "window.splitOrientation",
        json!({"orientation": "horizontal"}),
    );
    assert_eq!(
        (v["orientation"].as_str(), v["changed"].as_bool()),
        (Some("horizontal"), Some(true))
    );
    let d = s.active().expect("doc");
    assert_eq!(d.split_layout.orientation, SplitOrientation::Horizontal);
    assert_eq!((d.live_pane(), d.page, d.view.zoom), (1, page, zoom));
    assert_eq!(d.other_pane().cloned(), other);
    // closed and split again: still one above the other
    ex(&mut s, "window.closeDuplicate", json!({}));
    ex(&mut s, "window.duplicateWindow", json!({}));
    assert_eq!(
        s.active().map(|d| d.split_layout.orientation),
        Some(SplitOrientation::Horizontal)
    );
    assert!(
        s.execute(
            "window.splitOrientation",
            json!({"orientation": "diagonal"})
        )
        .is_err()
    );
    ex(
        &mut s,
        "window.splitOrientation",
        json!({"orientation": "vertical"}),
    );
    assert_eq!(
        s.active().map(|d| d.split_layout.orientation),
        Some(SplitOrientation::Vertical)
    );
}

/// F132: Window ▸ Next Pane moves the focus to the other pane (and back); without a split it is
/// unavailable.
#[test]
fn next_pane_cycles_the_focus() {
    let mut s = two_pages();
    assert!(s.execute("window.nextPane", json!({})).is_err());
    assert!(!s.command_state("window.nextPane").0);
    ex(&mut s, "window.duplicateWindow", json!({}));
    assert!(s.command_state("window.nextPane").0);
    ex(&mut s, "page.setCurrent", json!({"index": 1}));
    let v = ex(&mut s, "window.nextPane", json!({}));
    assert_eq!((v["pane"].as_u64(), v["page"].as_u64()), (Some(0), Some(0)));
    let v = ex(&mut s, "window.nextPane", json!({}));
    assert_eq!((v["pane"].as_u64(), v["page"].as_u64()), (Some(1), Some(1)));
}

/// F132: the hover probe for the pane without the focus reports the tool's hover feedback and
/// cursor without running anything and leaves the active tool machine alone.
#[test]
fn hover_probe_runs_nothing() {
    use fp_engine::tools::{Modifiers, PointerKind, ToolInput};
    let mut s = two_pages();
    ex(&mut s, "window.duplicateWindow", json!({}));
    ex(&mut s, "select.none", json!({}));
    let _ = s.set_tool(fp_engine::ToolId::Pointer);
    let before = s
        .active()
        .map(|d| (d.history.undo_count(), d.selection.clone()));
    let ev = fp_engine::ToolEvent {
        input: ToolInput::Pointer {
            kind: PointerKind::Move,
            pos: fp_geom::Point::new(30.0, 30.0),
        },
        mods: Modifiers::default(),
        pressure: 1.0,
        time_ms: 0,
    };
    let (feedback, _) = s.tool_hover_probe(Some(&ev));
    assert!(
        feedback
            .iter()
            .any(|o| matches!(o, fp_engine::Overlay::Highlight { .. })),
        "hover highlight over the rectangle: {feedback:?}"
    );
    assert_eq!(
        s.active()
            .map(|d| (d.history.undo_count(), d.selection.clone())),
        before
    );
    // the active machine got no move: its own feedback has no hover highlight
    assert!(
        !s.tool_feedback()
            .iter()
            .any(|o| matches!(o, fp_engine::Overlay::Highlight { .. }))
    );
}

/// F137: every pane keeps a stable key while other panes open and close (the UI keeps the
/// pane's canvas cache under it); the remaining pane of a closed split keeps its key.
#[test]
fn pane_keys_are_stable() {
    let mut s = two_pages();
    assert_eq!(s.active().map(|d| d.pane_key(0)), Some(0));
    ex(&mut s, "window.duplicateWindow", json!({}));
    ex(&mut s, "window.duplicateWindow", json!({}));
    let keys = |s: &Session| {
        let d = s.active().expect("doc");
        (0..d.pane_count()).map(|p| d.pane_key(p)).collect::<Vec<_>>()
    };
    let k = keys(&s);
    assert_eq!(k.len(), 3);
    assert_eq!(k[0], 0, "the first pane keeps the key of the unsplit view");
    assert!(k[0] != k[1] && k[1] != k[2] && k[0] != k[2], "{k:?}");
    ex(&mut s, "window.closeDuplicate", json!({"pane": 1}));
    assert_eq!(keys(&s), vec![k[0], k[2]]);
    ex(&mut s, "window.closeDuplicate", json!({"pane": 0}));
    let d = s.active().expect("doc");
    assert!(!d.is_split());
    assert_eq!(d.pane_key(0), k[2], "the remaining pane keeps its key");
    ex(&mut s, "window.duplicateWindow", json!({}));
    let k2 = keys(&s);
    assert_eq!(k2[0], k[2]);
    assert!(!k.contains(&k2[1]), "a new key: {k2:?}");
}
