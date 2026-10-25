//! FB-68 (RE spec 71 §4): multi-selection in the Pages panel. Shift selects the range from the
//! current page (the anchor, it stays current), Ctrl adds or removes a page; every click is a
//! History row "Change Page Selection". Delete removes all selected pages in one row (all
//! selected: the first stays, or the page named by `keep`; the only page becomes an empty
//! "Page 1"); Duplicate copies each selected page directly after itself in one row; Rename Page
//! and Set as Master Page are disabled while several pages are selected.

use crate::common;
use common::*;
use fp_engine::Session;
use serde_json::json;

/// Document with the pages P1…P4 (measured set-up `P4()`), each with a rectangle; P1 current.
fn four_pages() -> Session {
    fp_i18n::set_language("en");
    let mut s = doc_session(400.0, 300.0);
    for _ in 0..3 {
        ex(&mut s, "page.add", json!({}));
    }
    for i in 0..4 {
        ex(&mut s, "page.setCurrent", json!({"index": i}));
        ex(
            &mut s,
            "page.rename",
            json!({"index": i, "name": format!("P{}", i + 1)}),
        );
        let x = 10.0 + 20.0 * i as f32;
        rect(&mut s, x, 10.0, x + 40.0, 60.0);
    }
    ex(&mut s, "page.setCurrent", json!({"index": 0}));
    s
}

fn names(s: &Session) -> Vec<String> {
    s.active()
        .expect("doc")
        .doc
        .pages
        .iter()
        .map(|p| p.display_name().to_string())
        .collect()
}

fn selected(s: &Session) -> Vec<usize> {
    s.active().expect("doc").selected_pages()
}

fn current(s: &Session) -> usize {
    s.active().expect("doc").page
}

fn undo_count(s: &Session) -> usize {
    s.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

fn last_label(s: &Session) -> String {
    let d = s.active().expect("doc");
    s.step_label(d.history.steps()[d.history.undo_count() - 1])
}

fn elements_on(s: &Session, page: usize) -> usize {
    let d = s.active().expect("doc");
    fp_engine::docops::own_layers(&d.doc, page)
        .into_iter()
        .map(|(_, l)| l.cels.first().map_or(0, |c| c.elements.len()))
        .sum()
}

fn select(s: &mut Session, i: usize, mode: &str) {
    ex(s, "page.select", json!({"index": i, "mode": mode}));
}

#[test]
fn shift_selects_a_range_and_ctrl_toggles_with_the_current_page_as_anchor() {
    let mut s = four_pages();
    let rows = undo_count(&s);
    // measured: [0] → Shift P3 → [0,1,2] → Ctrl P2 → [0,2] → Ctrl P4 → [0,2,3]
    select(&mut s, 2, "range");
    assert_eq!(selected(&s), [0, 1, 2]);
    assert_eq!(current(&s), 0, "the anchor stays current");
    select(&mut s, 1, "toggle");
    assert_eq!(selected(&s), [0, 2]);
    select(&mut s, 3, "toggle");
    assert_eq!(selected(&s), [0, 2, 3]);
    assert_eq!(current(&s), 0);
    assert_eq!(undo_count(&s), rows + 3, "every click is a History row");
    assert_eq!(last_label(&s), "Change Page Selection");
    // the anchor cannot be toggled off
    select(&mut s, 0, "toggle");
    assert_eq!(selected(&s), [0, 2, 3]);
    // Undo brings the previous selection back
    ex(&mut s, "edit.undo", json!({}));
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(selected(&s), [0, 2]);
    // a plain click on the current page collapses the selection (its own row)
    let rows = undo_count(&s);
    select(&mut s, 0, "single");
    assert_eq!(selected(&s), [0]);
    assert_eq!(undo_count(&s), rows + 1);
    // Shift range from a later anchor goes backwards
    ex(&mut s, "page.setCurrent", json!({"index": 3}));
    select(&mut s, 1, "range");
    assert_eq!(selected(&s), [1, 2, 3]);
    assert_eq!(current(&s), 3);
    // a page change by script ends the multi-selection
    ex(&mut s, "page.setCurrent", json!({"index": 2}));
    assert_eq!(selected(&s), [2]);
}

#[test]
fn delete_removes_all_selected_pages_in_one_row_and_undo_restores_them() {
    let mut s = four_pages();
    select(&mut s, 2, "range");
    let rows = undo_count(&s);
    ex(&mut s, "panel.pages.delete", json!({}));
    assert_eq!(names(&s), ["P4"]);
    assert_eq!(undo_count(&s), rows + 1, "one History row");
    assert_eq!(last_label(&s), "Delete Page");
    assert_eq!(selected(&s), [0]);
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(names(&s), ["P1", "P2", "P3", "P4"]);
    assert_eq!(selected(&s), [0, 1, 2], "Undo restores the selection");
    ex(&mut s, "edit.redo", json!({}));
    assert_eq!(names(&s), ["P4"]);
}

#[test]
fn delete_with_a_page_list_and_a_non_current_page() {
    let mut s = four_pages();
    ex(&mut s, "page.setCurrent", json!({"index": 3}));
    ex(&mut s, "page.delete", json!({"pages": [0, 2]}));
    assert_eq!(names(&s), ["P2", "P4"]);
    assert_eq!(current(&s), 1, "the current page P4 stays current");
    // the single-page form keeps working
    ex(&mut s, "page.delete", json!({"index": 0}));
    assert_eq!(names(&s), ["P4"]);
}

#[test]
fn deleting_all_pages_keeps_the_first_or_the_chosen_page_with_its_content() {
    let mut s = four_pages();
    select(&mut s, 3, "range");
    ex(&mut s, "page.delete", json!({}));
    assert_eq!(names(&s), ["P1"], "reference: the first page stays");
    assert_eq!(elements_on(&s, 0), 1, "with its content");
    ex(&mut s, "edit.undo", json!({}));
    // Firepixel extension: `keep` names the page that stays
    let rows = undo_count(&s);
    ex(
        &mut s,
        "page.delete",
        json!({"pages": [0, 1, 2, 3], "keep": 2}),
    );
    assert_eq!(names(&s), ["P3"]);
    assert_eq!(elements_on(&s, 0), 1);
    assert_eq!(undo_count(&s), rows + 1, "one History row");
}

#[test]
fn deleting_the_only_page_leaves_an_empty_page_1() {
    fp_i18n::set_language("en");
    let mut s = doc_session(200.0, 100.0);
    rect(&mut s, 10.0, 10.0, 50.0, 50.0);
    ex(&mut s, "page.rename", json!({"name": "Only"}));
    ex(&mut s, "page.delete", json!({}));
    assert_eq!(names(&s), ["Page 1"]);
    assert_eq!(elements_on(&s, 0), 0);
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(names(&s), ["Only"]);
    assert_eq!(elements_on(&s, 0), 1);
}

#[test]
fn duplicate_copies_each_selected_page_directly_after_itself_in_one_row() {
    let mut s = four_pages();
    // measured: [1,2] → P1, P2, P2 Copy, P3, P3 Copy, P4; then "P2 copy" current and alone
    ex(&mut s, "page.setCurrent", json!({"index": 1}));
    select(&mut s, 2, "toggle");
    let rows = undo_count(&s);
    ex(&mut s, "panel.pages.duplicate", json!({}));
    assert_eq!(names(&s), ["P1", "P2", "P2 copy", "P3", "P3 copy", "P4"]);
    assert_eq!(undo_count(&s), rows + 1, "one History row");
    assert_eq!(current(&s), 2);
    assert_eq!(selected(&s), [2]);
    assert_eq!(elements_on(&s, 4), 1, "the copy has the content");
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(names(&s), ["P1", "P2", "P3", "P4"]);
    // the single-page form keeps working
    ex(&mut s, "page.duplicate", json!({"index": 3}));
    assert_eq!(names(&s), ["P1", "P2", "P3", "P4", "P4 copy"]);
}

#[test]
fn dragging_moves_only_the_dragged_page() {
    let mut s = four_pages();
    select(&mut s, 1, "range");
    ex(&mut s, "page.move", json!({"page": 0, "index": 3}));
    assert_eq!(names(&s), ["P2", "P3", "P4", "P1"]);
    assert_eq!(current(&s), 3);
    assert_eq!(selected(&s), [3]);
}

#[test]
fn rename_and_set_master_are_disabled_with_several_pages_selected() {
    let mut s = four_pages();
    let enabled = |s: &Session, c: &str| s.command_state(c).0;
    for c in ["panel.pages.rename", "panel.pages.setMaster"] {
        assert!(enabled(&s, c), "{c} with one page");
    }
    select(&mut s, 1, "range");
    for c in ["panel.pages.rename", "panel.pages.setMaster"] {
        assert!(!enabled(&s, c), "{c} with several pages");
    }
    for c in [
        "panel.pages.new",
        "panel.pages.duplicate",
        "panel.pages.delete",
        "panel.pages.export",
    ] {
        assert!(enabled(&s, c), "{c} stays enabled");
    }
    select(&mut s, 0, "single");
    assert!(enabled(&s, "panel.pages.rename"));
}
