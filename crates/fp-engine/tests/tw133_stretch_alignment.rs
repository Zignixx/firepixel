//! TW-133 (re 50 section 1): "stretched" (alignment code 4) cannot be stored by the text engine
//! and becomes left on every text, point or fixed-width; "justified" (3) is stored on both. Every
//! alignment command still writes `Text_Justification` (also 4), creates a "Text change" step,
//! marks the document dirty and lays the text out again.

use crate::common::{doc_session, el, ex, select};
use fp_engine::Session;
use fp_model::{ElementId, ElementKind};
use serde_json::json;

fn setup() -> Session {
    fp_i18n::set_language("de");
    doc_session(400.0, 300.0)
}

fn point_text(s: &mut Session) -> ElementId {
    let v = ex(s, "draw.text", json!({"text": "Abc 123", "anchor": [10, 50], "size": 24}));
    ElementId(v["id"].as_u64().unwrap_or(0))
}

fn frame_text(s: &mut Session) -> ElementId {
    let v = ex(
        s,
        "draw.text",
        json!({"text": "Rahmen", "rect": [10, 100, 200, 140], "size": 14}),
    );
    ElementId(v["id"].as_u64().unwrap_or(0))
}

/// Alignment of every run (and the initial attributes) of text `id`.
fn alignments(s: &Session, id: ElementId) -> Vec<Option<i32>> {
    match &el(s, id).kind {
        ElementKind::Text(t) => {
            let mut v = vec![t.initial.alignment];
            v.extend(t.resolved_runs().iter().map(|(a, _)| a.alignment));
            v
        }
        _ => panic!("no text"),
    }
}

fn mark_saved(s: &mut Session) {
    let d = s.active_mut().expect("doc");
    d.saved_generation = d.generation;
}

fn last_row(s: &Session) -> String {
    let d = s.active().expect("doc");
    d.history
        .steps()
        .last()
        .map(|st| s.step_label(st))
        .unwrap_or_default()
}

fn relaid(s: &Session, id: ElementId) -> bool {
    matches!(&el(s, id).kind, ElementKind::Text(t) if t.relaid)
}

#[test]
fn stretched_becomes_left_on_point_and_frame_text() {
    let mut s = setup();
    for id in [point_text(&mut s), frame_text(&mut s)] {
        select(&mut s, &[id]);
        ex(&mut s, "text.align.right", json!({}));
        assert!(alignments(&s, id).iter().all(|a| *a == Some(2)));
        ex(&mut s, "text.align.stretched", json!({}));
        assert!(
            alignments(&s, id).iter().all(|a| *a == Some(0)),
            "stretched stored as left: {:?}",
            alignments(&s, id)
        );
        assert_eq!(s.prefs.get_int("Text_Justification", 0), 4, "pref keeps 4");
        // the menu shows "left" checked for the text afterwards
        let left = s.registry().get("text.align.left").and_then(|c| c.checked(&s));
        assert_eq!(left, Some(true), "left checked");
    }
}

#[test]
fn justified_is_stored_on_point_and_frame_text() {
    let mut s = setup();
    for id in [point_text(&mut s), frame_text(&mut s)] {
        select(&mut s, &[id]);
        ex(&mut s, "text.align.justified", json!({}));
        assert!(alignments(&s, id).iter().all(|a| *a == Some(3)));
        assert_eq!(s.prefs.get_int("Text_Justification", 0), 3);
    }
}

#[test]
fn script_and_inspector_alignment_4_store_left() {
    let mut s = setup();
    let id = point_text(&mut s);
    select(&mut s, &[id]);
    // JSF `setTextAlignment("stretch")` runs `text.setAttributes {alignment: 4}`
    ex(&mut s, "text.setAttributes", json!({"alignment": 4}));
    assert!(alignments(&s, id).iter().all(|a| *a == Some(0)));
    ex(&mut s, "text.align.right", json!({}));
    // the inspector button stores the default as given, the text gets left
    ex(&mut s, "text.applyAttributes", json!({"alignment": 4}));
    assert!(alignments(&s, id).iter().all(|a| *a == Some(0)));
    assert_eq!(s.prefs.get_int("Text_Justification", 0), 4);
}

#[test]
fn alignment_command_writes_pref_history_dirty_and_relayout() {
    let mut s = setup();
    let id = point_text(&mut s);
    select(&mut s, &[id]);
    mark_saved(&mut s);
    let n = s.active().expect("doc").history.len();
    // already left: "stretched" changes nothing visible, yet it is a step like in the original
    ex(&mut s, "text.align.stretched", json!({}));
    let d = s.active().expect("doc");
    assert_eq!(d.history.len(), n + 1, "one history step");
    assert!(d.is_dirty(), "document dirty");
    assert!(relaid(&s, id), "text laid out again");
    assert_eq!(last_row(&s), "Textänderung");
    assert_eq!(s.prefs.get_int("Text_Justification", 0), 4);
    fp_i18n::set_language("en");
    assert_eq!(last_row(&s), "Text Change");
    fp_i18n::set_language("de");
    ex(&mut s, "text.align.centeredHorizontally", json!({}));
    assert_eq!(s.prefs.get_int("Text_Justification", 0), 1);
    assert_eq!(s.active().expect("doc").history.len(), n + 2);
}

#[test]
fn without_text_only_the_defaults_change() {
    let mut s = setup();
    let n = s.active().expect("doc").history.len();
    ex(&mut s, "text.align.stretched", json!({}));
    assert_eq!(s.prefs.get_int("Text_Justification", 0), 4);
    assert_eq!(s.active().expect("doc").history.len(), n, "no history step");
    // new text reads the default and becomes left
    let id = point_text(&mut s);
    assert!(
        alignments(&s, id).iter().all(|a| a.unwrap_or(0) == 0),
        "{:?}",
        alignments(&s, id)
    );
    ex(&mut s, "text.align.justified", json!({}));
    ex(&mut s, "select.none", json!({}));
    let id = frame_text(&mut s);
    assert!(alignments(&s, id).iter().all(|a| *a == Some(3)));
}
