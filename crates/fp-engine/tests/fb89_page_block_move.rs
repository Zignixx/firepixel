//! FB-89 (Firepixel extension): `page.move` with `pages` moves several pages as one block to the
//! target index (index of the first moved page in the new order), keeping their order, in one
//! History row "Move Pages"; the current page stays current, the moved pages stay selected. The
//! single form (`page`) keeps moving only that page.

use crate::common;
use common::*;
use fp_engine::Session;
use serde_json::json;

/// Pages P1…P6, P1 current.
fn six_pages() -> Session {
    fp_i18n::set_language("en");
    let mut s = doc_session(200.0, 100.0);
    for _ in 0..5 {
        ex(&mut s, "page.add", json!({}));
    }
    for i in 0..6 {
        ex(
            &mut s,
            "page.rename",
            json!({"index": i, "name": format!("P{}", i + 1)}),
        );
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

fn current(s: &Session) -> usize {
    s.active().expect("doc").page
}

fn current_name(s: &Session) -> String {
    let d = s.active().expect("doc");
    d.doc.pages[d.page].display_name().to_string()
}

fn selected(s: &Session) -> Vec<usize> {
    s.active().expect("doc").selected_pages()
}

fn undo_count(s: &Session) -> usize {
    s.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

fn last_label(s: &Session) -> String {
    let d = s.active().expect("doc");
    s.step_label(d.history.steps()[d.history.undo_count() - 1])
}

#[test]
fn block_moves_down_in_order_in_one_row() {
    let mut s = six_pages();
    ex(&mut s, "page.setCurrent", json!({"index": 1}));
    ex(&mut s, "page.select", json!({"index": 3, "mode": "toggle"}));
    let rows = undo_count(&s);
    // P2 and P4 (not adjacent) after P5: first of them at index 3 of the new order
    ex(&mut s, "page.move", json!({"pages": [3, 1], "index": 3}));
    assert_eq!(names(&s), ["P1", "P3", "P5", "P2", "P4", "P6"]);
    assert_eq!(undo_count(&s), rows + 1, "one History row");
    assert_eq!(last_label(&s), "Move Pages");
    assert_eq!(current_name(&s), "P2", "the current page stays current");
    assert_eq!(selected(&s), [3, 4], "the moved pages stay selected");
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(names(&s), ["P1", "P2", "P3", "P4", "P5", "P6"]);
    assert_eq!(current(&s), 1);
    ex(&mut s, "edit.redo", json!({}));
    assert_eq!(names(&s), ["P1", "P3", "P5", "P2", "P4", "P6"]);
}

#[test]
fn block_moves_up_and_target_is_clamped() {
    let mut s = six_pages();
    ex(&mut s, "page.setCurrent", json!({"index": 5}));
    ex(&mut s, "page.select", json!({"index": 3, "mode": "range"}));
    let rows = undo_count(&s);
    ex(&mut s, "page.move", json!({"pages": [3, 4, 5], "index": 0}));
    assert_eq!(names(&s), ["P4", "P5", "P6", "P1", "P2", "P3"]);
    assert_eq!(undo_count(&s), rows + 1);
    assert_eq!(current_name(&s), "P6");
    assert_eq!(selected(&s), [0, 1, 2]);
    // beyond the end: the block ends at the last page
    ex(&mut s, "page.move", json!({"pages": [0, 2], "index": 99}));
    assert_eq!(names(&s), ["P5", "P1", "P2", "P3", "P4", "P6"]);
    ex(&mut s, "edit.undo", json!({}));
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(names(&s), ["P1", "P2", "P3", "P4", "P5", "P6"]);
}

#[test]
fn a_current_page_outside_the_block_stays_current() {
    let mut s = six_pages();
    ex(&mut s, "page.setCurrent", json!({"index": 2}));
    ex(&mut s, "page.move", json!({"pages": [4, 5], "index": 0}));
    assert_eq!(names(&s), ["P5", "P6", "P1", "P2", "P3", "P4"]);
    assert_eq!(current_name(&s), "P3");
}

#[test]
fn single_forms_and_errors() {
    let mut s = six_pages();
    // a list of one page = the single form (row "Move Page")
    ex(&mut s, "page.move", json!({"pages": [0], "index": 2}));
    assert_eq!(names(&s), ["P2", "P3", "P1", "P4", "P5", "P6"]);
    assert_eq!(last_label(&s), "Move Page");
    ex(&mut s, "page.move", json!({"page": 2, "index": 0}));
    assert_eq!(names(&s), ["P1", "P2", "P3", "P4", "P5", "P6"]);
    assert!(
        s.execute("page.move", json!({"pages": [1, 9], "index": 0}))
            .is_err()
    );
    assert!(
        s.execute("page.move", json!({"pages": [], "index": 0}))
            .is_err()
    );
    assert_eq!(names(&s), ["P1", "P2", "P3", "P4", "P5", "P6"]);
}
