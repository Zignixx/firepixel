//! FB-97 / TW-272: Window ▸ Duplicate Window splits the document window into two panes, each with
//! its own page, state, zoom, scroll and selection; both show the same document. The focused pane's
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
    // a second Duplicate Window keeps the split as it is
    ex(&mut s, "window.duplicateWindow", json!({}));
    assert_eq!(s.active().map(|d| d.live_pane()), Some(0));
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

#[test]
fn close_duplicate_keeps_the_focused_pane() {
    let mut s = two_pages();
    assert!(s.execute("window.closeDuplicate", json!({})).is_err());
    assert!(s.execute("window.focusPane", json!({"pane": 1})).is_err());
    ex(&mut s, "window.duplicateWindow", json!({}));
    ex(&mut s, "page.setCurrent", json!({"index": 1}));
    let v = ex(&mut s, "window.closeDuplicate", json!({}));
    assert_eq!(v["closed"], true, "{v}");
    let d = s.active().expect("doc");
    assert!(!d.is_split());
    assert_eq!((d.page, d.live_pane()), (1, 0));
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
