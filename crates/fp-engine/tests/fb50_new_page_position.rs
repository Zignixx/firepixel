//! FB-50 (RE spec 70 §2): New Page (panel button / panel menu / context menu = `edit.insert.page`,
//! `panel.pages.new`; script `page.add`) inserts directly after the current page and makes the new
//! page current, one History row "Add Page"; the default name is "Page N" with the smallest free
//! N ≥ 1. Duplicate Page (`page.duplicate`, `panel.pages.duplicate`) inserts the copy directly
//! after the source page, the copy becomes current, one History row "Duplicate Page"; names
//! "<name> Copy", "Copy 1", "Copy 2" … with an existing copy suffix stripped first. Changing the
//! page is its own History row "Change Page Selection".

use crate::common;
use common::*;
use fp_engine::Session;
use serde_json::json;

/// Document 400 × 300 with the pages P1, P2, P3 (measured set-up), page `cur` current.
fn three_pages(cur: usize) -> Session {
    fp_i18n::set_language("de");
    let mut s = doc_session(400.0, 300.0);
    ex(&mut s, "page.add", json!({}));
    ex(&mut s, "page.add", json!({}));
    for (i, n) in ["P1", "P2", "P3"].iter().enumerate() {
        ex(&mut s, "page.rename", json!({"index": i, "name": n}));
    }
    ex(&mut s, "page.setCurrent", json!({"index": cur}));
    s
}

fn names(s: &Session) -> Vec<String> {
    s.active().expect("doc").doc.pages.iter().map(|p| p.display_name().to_string()).collect()
}

fn current(s: &Session) -> usize {
    s.active().expect("doc").page
}

fn current_name(s: &Session) -> String {
    let d = s.active().expect("doc");
    d.doc.pages[d.page].display_name().to_string()
}

fn undo_count(s: &Session) -> usize {
    s.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

fn last_label(s: &Session) -> String {
    let d = s.active().expect("doc");
    s.step_label(d.history.steps()[d.history.undo_count() - 1])
}

const NEW_ROUTES: [&str; 3] = ["edit.insert.page", "panel.pages.new", "page.add"];
const DUP_ROUTES: [&str; 2] = ["page.duplicate", "panel.pages.duplicate"];

#[test]
fn new_page_goes_directly_after_the_current_page_on_every_route() {
    // RE spec 70 §2.2: P2 current → index 2; P1 (first) → index 1; P3 (last) → the end
    let want: [(usize, [&str; 4]); 3] = [
        (0, ["P1", "Seite 1", "P2", "P3"]),
        (1, ["P1", "P2", "Seite 1", "P3"]),
        (2, ["P1", "P2", "P3", "Seite 1"]),
    ];
    for route in NEW_ROUTES {
        for (cur, list) in &want {
            let mut s = three_pages(*cur);
            let before = undo_count(&s);
            ex(&mut s, route, json!({}));
            assert_eq!(names(&s), list, "{route}, page {cur} current");
            assert_eq!(current(&s), cur + 1, "{route}: the new page becomes current");
            assert_eq!(current_name(&s), "Seite 1");
            assert_eq!(undo_count(&s), before + 1, "{route}: one History row");
            assert_eq!(last_label(&s), "Seite hinzufügen", "{route}");
            ex(&mut s, "edit.undo", json!({}));
            assert_eq!(names(&s), ["P1", "P2", "P3"], "{route}: Undo removes the page");
            assert_eq!(current(&s), *cur, "{route}: Undo restores the current page");
        }
    }
}

#[test]
fn new_page_after_the_last_page_repeatedly() {
    // follow-up runs of the measurement: P3 current, button twice → P1 P2 P3 Seite 1 Seite 2
    let mut s = three_pages(2);
    ex(&mut s, "edit.insert.page", json!({}));
    ex(&mut s, "edit.insert.page", json!({}));
    assert_eq!(names(&s), ["P1", "P2", "P3", "Seite 1", "Seite 2"]);
    assert_eq!(current(&s), 4);
    // first page current again: the next one goes to index 1
    ex(&mut s, "page.setCurrent", json!({"index": 0}));
    ex(&mut s, "edit.insert.page", json!({}));
    assert_eq!(names(&s), ["P1", "Seite 3", "P2", "P3", "Seite 1", "Seite 2"]);
    assert_eq!(current(&s), 1);
}

#[test]
fn default_name_takes_the_smallest_free_number() {
    // fresh document (only "Seite 1") → "Seite 2"
    fp_i18n::set_language("de");
    let mut s = doc_session(400.0, 300.0);
    ex(&mut s, "edit.insert.page", json!({}));
    assert_eq!(names(&s), ["Seite 1", "Seite 2"]);
    // P1/P2/P3 → Seite 1, 2, 3, 4 (not the page count + 1)
    let mut s = three_pages(2);
    for _ in 0..4 {
        ex(&mut s, "edit.insert.page", json!({}));
    }
    assert_eq!(names(&s), ["P1", "P2", "P3", "Seite 1", "Seite 2", "Seite 3", "Seite 4"]);
    // a gap after deleting "Seite 2" is filled again
    ex(&mut s, "page.delete", json!({"index": 4}));
    ex(&mut s, "page.setCurrent", json!({"index": 5}));
    ex(&mut s, "edit.insert.page", json!({}));
    assert_eq!(current_name(&s), "Seite 2");
    // P1 renamed "Seite 9", Seite 1–4 and 9 exist → "Seite 5"
    ex(&mut s, "page.rename", json!({"index": 0, "name": "Seite 9"}));
    ex(&mut s, "edit.insert.page", json!({}));
    assert_eq!(current_name(&s), "Seite 5");
    // an explicit name is kept
    ex(&mut s, "page.add", json!({"name": "Eigen"}));
    assert_eq!(current_name(&s), "Eigen");
}

#[test]
fn duplicate_goes_directly_after_the_source_on_every_route() {
    let want: [(usize, [&str; 4]); 3] = [
        (0, ["P1", "P1 Kopie", "P2", "P3"]),
        (1, ["P1", "P2", "P2 Kopie", "P3"]),
        (2, ["P1", "P2", "P3", "P3 Kopie"]),
    ];
    for route in DUP_ROUTES {
        for (cur, list) in &want {
            let mut s = three_pages(*cur);
            let before = undo_count(&s);
            ex(&mut s, route, json!({}));
            assert_eq!(names(&s), list, "{route}, page {cur} current");
            assert_eq!(current(&s), cur + 1, "{route}: the copy becomes current");
            assert_eq!(undo_count(&s), before + 1, "{route}: one History row");
            assert_eq!(last_label(&s), "Seite duplizieren", "{route}");
        }
    }
    // explicit source (script `duplicatePage(i)`, row dragged onto the button): after that page
    let mut s = three_pages(1);
    ex(&mut s, "page.duplicate", json!({"index": 2}));
    assert_eq!(names(&s), ["P1", "P2", "P3", "P3 Kopie"]);
    assert_eq!(current(&s), 3);
}

#[test]
fn duplicate_names_count_up_and_strip_an_existing_copy_suffix() {
    let mut s = three_pages(1);
    ex(&mut s, "page.duplicate", json!({"index": 1}));
    assert_eq!(current_name(&s), "P2 Kopie");
    ex(&mut s, "page.duplicate", json!({"index": 1}));
    assert_eq!(current_name(&s), "P2 Kopie 1");
    // duplicate of "P2 Kopie 1" is "P2 Kopie 2", not "P2 Kopie 1 Kopie"
    ex(&mut s, "page.duplicate", json!({}));
    assert_eq!(current_name(&s), "P2 Kopie 2");
    // duplicate of "P2 Kopie" → next free variant
    let i = names(&s).iter().position(|n| n == "P2 Kopie").expect("P2 Kopie");
    ex(&mut s, "page.duplicate", json!({"index": i}));
    assert_eq!(current_name(&s), "P2 Kopie 3");
    assert_eq!(names(&s), ["P1", "P2", "P2 Kopie 1", "P2 Kopie 2", "P2 Kopie", "P2 Kopie 3", "P3"]);
    // a gap is filled; a name merely ending in digits keeps them
    ex(&mut s, "page.rename", json!({"index": 2, "name": "Frei"}));
    ex(&mut s, "page.duplicate", json!({"index": 1}));
    assert_eq!(current_name(&s), "P2 Kopie 1");
    ex(&mut s, "page.rename", json!({"index": 0, "name": "Seite 7"}));
    ex(&mut s, "page.duplicate", json!({"index": 0}));
    assert_eq!(current_name(&s), "Seite 7 Kopie");
}

#[test]
fn page_change_is_a_history_row() {
    let mut s = three_pages(0);
    let before = undo_count(&s);
    ex(&mut s, "page.setCurrent", json!({"index": 2}));
    assert_eq!(undo_count(&s), before + 1);
    assert_eq!(last_label(&s), "Seitenauswahl ändern");
}

#[test]
fn english_names_and_history_rows() {
    let mut s = three_pages(1);
    fp_i18n::set_language("en");
    ex(&mut s, "edit.insert.page", json!({}));
    assert_eq!(current_name(&s), "Page 1");
    assert_eq!(last_label(&s), "Add Page");
    ex(&mut s, "edit.insert.page", json!({}));
    assert_eq!(current_name(&s), "Page 2");
    ex(&mut s, "page.duplicate", json!({"index": 1}));
    assert_eq!(current_name(&s), "P2 copy");
    assert_eq!(last_label(&s), "Duplicate Page");
    ex(&mut s, "page.duplicate", json!({"index": 1}));
    assert_eq!(current_name(&s), "P2 copy 1");
    ex(&mut s, "page.duplicate", json!({}));
    assert_eq!(current_name(&s), "P2 copy 2");
    ex(&mut s, "page.setCurrent", json!({"index": 0}));
    assert_eq!(last_label(&s), "Change Page Selection");
    assert_eq!(names(&s), ["P1", "P2", "P2 copy 1", "P2 copy 2", "P2 copy", "Page 1", "Page 2", "P3"]);
    fp_i18n::set_language("de");
}

#[test]
fn saved_file_keeps_the_page_order() {
    let mut s = three_pages(2);
    // the model's page of the stored root copy is P3; inserting in front of it keeps it on P3
    if let Some(d) = s.active_mut() {
        std::sync::Arc::make_mut(&mut d.doc).current_page = 2;
    }
    ex(&mut s, "page.setCurrent", json!({"index": 0}));
    ex(&mut s, "edit.insert.page", json!({}));
    ex(&mut s, "page.duplicate", json!({"index": 0}));
    let d = s.active().expect("doc");
    assert_eq!(d.doc.pages[d.doc.current_page].display_name(), "P3");
    let order = names(&s);
    assert_eq!(order, ["P1", "P1 Kopie", "Seite 1", "P2", "P3"]);
    let dir = std::env::temp_dir().join(format!("fp-engine-fb50-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("pages.fw.png");
    ex(&mut s, "file.saveAs", json!({"path": path.to_string_lossy()}));
    let back = fp_fwpng::read(&std::fs::read(&path).unwrap_or_default()).unwrap_or_else(|e| panic!("{e}"));
    let back_names: Vec<String> = back.pages.iter().map(|p| p.display_name().to_string()).collect();
    assert_eq!(back_names, order, "the file keeps the page order");
    let _ = std::fs::remove_dir_all(&dir);
}
