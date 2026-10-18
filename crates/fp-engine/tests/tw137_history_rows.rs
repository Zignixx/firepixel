//! TW-137: one history row per finished undo session. Copy ("Kopie"/"Copy") and Cut
//! ("Ausschneiden"/"Cut") are rows of their own, as are Select All and Deselect; a step without a
//! name reads "Befehlsskript"/"Command script"; `MaxUndos` (default 20) drops exactly the oldest
//! step. Copy is an undoable no-op: it neither changes the document nor marks it as changed.

use crate::common::{doc_session, ex, rect, select, top_elements};
use fp_engine::Session;
use serde_json::json;
use std::sync::Arc;

fn rows(s: &Session) -> Vec<String> {
    let d = s.active().expect("doc");
    d.history.steps().iter().map(|st| s.step_label(st)).collect()
}

fn mark_saved(s: &mut Session) {
    let d = s.active_mut().expect("doc");
    d.saved_generation = d.generation;
}

fn setup() -> Session {
    fp_i18n::set_language("de");
    let mut s = doc_session(300.0, 200.0);
    let r = rect(&mut s, 10.0, 10.0, 60.0, 60.0);
    select(&mut s, &[r]);
    mark_saved(&mut s);
    s
}

#[test]
fn copy_is_an_undoable_no_op_row() {
    let mut s = setup();
    let n = s.active().unwrap().history.len();
    let doc = Arc::clone(&s.active().unwrap().doc);
    ex(&mut s, "edit.copy", json!({}));
    let d = s.active().unwrap();
    assert_eq!(d.history.len(), n + 1, "Copy is a row");
    assert_eq!(rows(&s).last().map(String::as_str), Some("Kopie"));
    assert!(Arc::ptr_eq(&doc, &s.active().unwrap().doc), "document untouched");
    assert!(!s.active().unwrap().is_dirty(), "Copy does not mark the document");
    assert_eq!(s.clipboard.elements.len(), 1);
    ex(&mut s, "edit.undo", json!({}));
    let d = s.active().unwrap();
    assert_eq!(d.history.len(), n);
    assert!(Arc::ptr_eq(&doc, &d.doc), "undo leaves the document alone");
    assert!(!d.is_dirty(), "undoing Copy does not mark the document");
    assert_eq!(s.clipboard.elements.len(), 1, "clipboard is not undone");
    ex(&mut s, "edit.redo", json!({}));
    assert!(!s.active().unwrap().is_dirty());
    assert_eq!(rows(&s).last().map(String::as_str), Some("Kopie"));
}

#[test]
fn cut_is_one_row() {
    let mut s = setup();
    let n = s.active().unwrap().history.len();
    ex(&mut s, "edit.cut", json!({}));
    assert_eq!(s.active().unwrap().history.len(), n + 1);
    assert_eq!(rows(&s).last().map(String::as_str), Some("Ausschneiden"));
    assert!(top_elements(&s).is_empty(), "removed");
    assert_eq!(s.clipboard.elements.len(), 1, "copied");
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(top_elements(&s).len(), 1, "undo brings it back");
}

#[test]
fn select_all_and_deselect_are_rows() {
    let mut s = setup();
    ex(&mut s, "select.none", json!({}));
    ex(&mut s, "select.all", json!({}));
    // also when everything is selected already: the command is a session of its own
    ex(&mut s, "select.all", json!({}));
    let r = rows(&s);
    assert_eq!(
        r[r.len() - 3..],
        ["Auswahl aufheben", "Alles auswählen", "Alles auswählen"]
    );
    assert!(!s.active().unwrap().is_dirty(), "selection rows do not mark");
}

#[test]
fn row_names_follow_the_language() {
    let mut s = setup();
    ex(&mut s, "edit.copy", json!({}));
    ex(&mut s, "select.none", json!({}));
    ex(&mut s, "select.all", json!({}));
    ex(&mut s, "edit.cut", json!({}));
    let tail = |s: &Session| {
        let r = rows(s);
        r[r.len() - 4..].to_vec()
    };
    assert_eq!(
        tail(&s),
        ["Kopie", "Auswahl aufheben", "Alles auswählen", "Ausschneiden"]
    );
    fp_i18n::set_language("en");
    assert_eq!(tail(&s), ["Copy", "Deselect", "Select All", "Cut"]);
    assert_eq!(fp_i18n::t("msg-step-copy"), "Copy");
    assert_eq!(fp_i18n::t("msg-step-cut"), "Cut");
    assert_eq!(fp_i18n::t("msg-step-command-script"), "Command script");
    fp_i18n::set_language("de");
    assert_eq!(fp_i18n::t("msg-step-copy"), "Kopie");
    assert_eq!(fp_i18n::t("msg-step-cut"), "Ausschneiden");
    assert_eq!(fp_i18n::t("msg-step-command-script"), "Befehlsskript");
}

#[test]
fn step_without_name_reads_command_script() {
    let mut s = setup();
    s.run_group("", |s| {
        s.execute("draw.rectangle", json!({"rect": [80, 80, 120, 120]}))?;
        Ok(())
    })
    .expect("group");
    assert_eq!(rows(&s).last().map(String::as_str), Some("Befehlsskript"));
    fp_i18n::set_language("en");
    assert_eq!(rows(&s).last().map(String::as_str), Some("Command script"));
    fp_i18n::set_language("de");
}

#[test]
fn max_undos_default_20_drops_exactly_the_oldest() {
    let mut s = setup();
    assert_eq!(s.prefs.max_undos(), 20);
    assert_eq!(s.active().unwrap().history.max_undos(), 20);
    s.active_mut().unwrap().history.clear();
    let dropped0 = s.active().unwrap().history.dropped();
    ex(&mut s, "select.all", json!({}));
    for _ in 0..19 {
        ex(&mut s, "edit.copy", json!({}));
    }
    let d = s.active().unwrap();
    assert_eq!(d.history.len(), 20);
    assert_eq!(d.history.dropped(), dropped0);
    assert_eq!(rows(&s)[0], "Alles auswählen");
    ex(&mut s, "edit.copy", json!({}));
    let d = s.active().unwrap();
    assert_eq!(d.history.len(), 20, "never more than MaxUndos");
    assert_eq!(d.history.dropped(), dropped0 + 1, "exactly one step dropped");
    assert!(rows(&s).iter().all(|r| r == "Kopie"), "the oldest one went");
}
