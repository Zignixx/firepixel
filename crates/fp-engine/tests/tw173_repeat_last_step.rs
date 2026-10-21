//! TW-173 (re 55 §7.6, measured): Edit ▸ Redo / Ctrl+Y redoes an undone step; with nothing undone
//! it repeats the last history row as a NEW row with the same command ("Repeat: <step>"), and a
//! repeat can be repeated. Not repeatable: right after a redo and after a bitmap stroke ("Can't
//! Redo", nothing happens).

use crate::common;
use common::*;
use fp_engine::tools::ToolId;
use fp_engine::{Session, ToolEvent, ToolKey};
use serde_json::json;

fn undo_count(s: &Session) -> usize {
    s.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

fn last_label(s: &Session) -> String {
    let d = s.active().expect("doc");
    d.history.top().map(|st| s.step_label(st)).unwrap_or_default()
}

fn last_command(s: &Session) -> String {
    let d = s.active().expect("doc");
    d.history.top().map(|st| st.command.clone()).unwrap_or_default()
}

fn redo(s: &mut Session) {
    ex(s, "edit.redo", json!({}));
}

/// (a) Clone, Ctrl+Y → a second clone (2 → 3 elements) as a new row "Clone"; (c) Ctrl+Y again → a
/// third clone; (b) Ctrl+Z, Ctrl+Y = redo (no new row), Ctrl+Y right after the redo does nothing and
/// the entry is disabled; the next new step makes the repeat available again.
#[test]
fn clone_repeats_and_redo_blocks_the_repeat() {
    fp_i18n::set_language("de");
    let mut s = doc_session(400.0, 300.0);
    let r = rect(&mut s, 40.0, 40.0, 140.0, 120.0);
    select(&mut s, &[r]);
    ex(&mut s, "edit.clone", json!({}));
    assert_eq!(top_elements(&s).len(), 2);
    let n = undo_count(&s);
    let clone = last_label(&s);
    assert_eq!(s.undo_redo_labels().1, None, "nothing undone: no redo");
    assert!(s.is_available("edit.redo"), "Repeat: Clone");
    assert_eq!(s.repeat_label().as_deref(), Some(clone.as_str()));
    // (a)
    redo(&mut s);
    assert_eq!(top_elements(&s).len(), 3, "a second clone");
    assert_eq!(undo_count(&s), n + 1, "a new row");
    assert_eq!(last_label(&s), clone);
    assert_eq!(last_command(&s), "edit.clone");
    // (c) a repeat is repeatable
    assert_eq!(s.repeat_label().as_deref(), Some(clone.as_str()));
    redo(&mut s);
    assert_eq!(top_elements(&s).len(), 4, "a third clone");
    assert_eq!(undo_count(&s), n + 2);
    // (b) undo → the entry redoes
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(top_elements(&s).len(), 3);
    assert_eq!(s.undo_redo_labels().1.as_deref(), Some(clone.as_str()), "Redo: Clone");
    assert_eq!(s.repeat_label(), None, "a redo is available: no repeat");
    redo(&mut s);
    assert_eq!(top_elements(&s).len(), 4, "redo");
    assert_eq!(undo_count(&s), n + 2, "redo adds no row");
    // right after the redo: "Can't Redo", nothing happens
    assert_eq!(s.repeat_label(), None);
    assert!(!s.is_available("edit.redo"));
    assert!(s.execute("edit.redo", json!({})).is_err());
    assert_eq!(top_elements(&s).len(), 4);
    assert_eq!(undo_count(&s), n + 2);
    // a new step ends that
    ex(&mut s, "edit.clone", json!({}));
    assert_eq!(top_elements(&s).len(), 5);
    assert!(s.is_available("edit.redo"));
    redo(&mut s);
    assert_eq!(top_elements(&s).len(), 6);
    // the history panel's undo/redo jumps: forward counts as a redo
    ex(&mut s, "edit.undo", json!({}));
    let pos = undo_count(&s) + 1;
    ex(&mut s, "history.goTo", json!({"position": pos}));
    assert!(!s.is_available("edit.redo"), "jumping forward is a redo");
}

/// Duplicate: Ctrl+Y duplicates again (+10/+10 from the last copy).
#[test]
fn duplicate_repeats_with_offset() {
    fp_i18n::set_language("de");
    let mut s = doc_session(400.0, 300.0);
    let r = rect(&mut s, 40.0, 40.0, 140.0, 120.0);
    select(&mut s, &[r]);
    ex(&mut s, "edit.duplicate", json!({}));
    let n = undo_count(&s);
    let sel = |s: &Session| s.active().and_then(|d| d.selection.elements.first().copied()).expect("copy");
    let first = sel(&s);
    assert_ne!(first, r);
    let b0 = bounds(&s, r);
    let b1 = bounds(&s, first);
    assert!(approx(b1.left - b0.left, 10.0), "{b0:?} → {b1:?}");
    redo(&mut s);
    assert_eq!(undo_count(&s), n + 1, "a new row");
    assert_eq!(last_command(&s), "edit.duplicate");
    assert_eq!(top_elements(&s).len(), 3);
    let second = sel(&s);
    assert!(second != first && second != r);
    let b2 = bounds(&s, second);
    assert!(approx(b2.left - b1.left, 10.0) && approx(b2.top - b1.top, 10.0), "{b1:?} → {b2:?}");
}

/// (e) Arrow-key nudge: Ctrl+Y moves again by the same offset as a new row (not merged into the
/// nudge row); a mouse drag with the pointer likewise.
#[test]
fn moves_repeat() {
    fp_i18n::set_language("de");
    let mut s = doc_session(400.0, 300.0);
    let r = rect(&mut s, 40.0, 40.0, 140.0, 120.0);
    select(&mut s, &[r]);
    s.set_tool(ToolId::Pointer).expect("pointer");
    events(&mut s, &[ToolEvent::key(ToolKey::Right)]);
    assert!(approx(bounds(&s, r).left, 41.0));
    let n = undo_count(&s);
    let label = last_label(&s);
    assert_eq!(s.repeat_label().as_deref(), Some(label.as_str()), "Repeat: Move");
    redo(&mut s);
    assert!(approx(bounds(&s, r).left, 42.0), "moved again: {:?}", bounds(&s, r));
    assert_eq!(undo_count(&s), n + 1, "a new row");
    assert_eq!(last_label(&s), label);
    redo(&mut s);
    assert!(approx(bounds(&s, r).left, 43.0));
    assert_eq!(undo_count(&s), n + 2);
    // mouse drag 30/20
    let b0 = bounds(&s, r);
    events(&mut s, &[ToolEvent::down(90.0, 80.0)]);
    for i in 1..=10 {
        let t = i as f32 / 10.0;
        events(&mut s, &[ToolEvent::drag(90.0 + 30.0 * t, 80.0 + 20.0 * t)]);
    }
    events(&mut s, &[ToolEvent::up(120.0, 100.0)]);
    let b1 = bounds(&s, r);
    assert!(approx(b1.left - b0.left, 30.0) && approx(b1.top - b0.top, 20.0), "{b0:?} → {b1:?}");
    let n = undo_count(&s);
    redo(&mut s);
    let b2 = bounds(&s, r);
    assert!(approx(b2.left - b1.left, 30.0) && approx(b2.top - b1.top, 20.0), "{b1:?} → {b2:?}");
    assert_eq!(undo_count(&s), n + 1);
}

/// Transform (rotate 90° CW): Ctrl+Y rotates again.
#[test]
fn rotate_repeats() {
    fp_i18n::set_language("de");
    let mut s = doc_session(400.0, 300.0);
    let r = rect(&mut s, 40.0, 40.0, 140.0, 80.0);
    select(&mut s, &[r]);
    let b0 = bounds(&s, r);
    ex(&mut s, "modify.transform.rotate90DegreeCw", json!({}));
    let b1 = bounds(&s, r);
    assert!(approx(b1.width(), b0.height()));
    let n = undo_count(&s);
    redo(&mut s);
    let b2 = bounds(&s, r);
    assert!(approx(b2.width(), b0.width()) && approx(b2.height(), b0.height()), "rotated again: {b2:?}");
    assert_eq!(undo_count(&s), n + 1);
    assert_eq!(last_command(&s), "modify.transform.rotate90DegreeCw");
}

/// (d) After a bitmap brush stroke the entry reads "Can't Redo" and Ctrl+Y does nothing.
#[test]
fn brush_stroke_is_not_repeatable() {
    fp_i18n::set_language("de");
    let mut s = doc_session(400.0, 300.0);
    s.set_tool(ToolId::Brush).expect("brush");
    events(&mut s, &[ToolEvent::down(50.0, 50.0)]);
    for i in 1..=10 {
        events(&mut s, &[ToolEvent::drag(50.0 + 10.0 * i as f32, 50.0)]);
    }
    events(&mut s, &[ToolEvent::up(150.0, 50.0)]);
    let n = undo_count(&s);
    assert!(n > 0, "the stroke is a row");
    let doc = s.active().map(|d| d.doc.clone()).expect("doc");
    assert_eq!(s.repeat_label(), None);
    assert!(!s.is_available("edit.redo"));
    assert!(s.execute("edit.redo", json!({})).is_err());
    assert_eq!(undo_count(&s), n);
    assert!(std::sync::Arc::ptr_eq(&doc, &s.active().expect("doc").doc));
}

/// Pen point: Ctrl+Y adds the same point again as a new row "Pen" (`appendPointToPath` with the
/// same arguments in the original); drawing goes on from there.
#[test]
fn pen_point_repeats() {
    fp_i18n::set_language("de");
    let mut s = doc_session(400.0, 300.0);
    s.set_tool(ToolId::Pen).expect("pen");
    events(&mut s, &[ToolEvent::down(100.0, 100.0), ToolEvent::up(100.0, 100.0)]);
    events(&mut s, &[ToolEvent::down(320.0, 227.0), ToolEvent::up(320.0, 227.0)]);
    let n = undo_count(&s);
    assert_eq!(s.repeat_label().as_deref(), Some("Stift"));
    redo(&mut s);
    assert_eq!(undo_count(&s), n + 1, "a new row");
    assert_eq!(last_label(&s), "Stift");
    let anchors = |s: &Session| -> Vec<(f32, f32)> {
        let els = top_elements(s);
        match &els.last().expect("path").kind {
            fp_model::ElementKind::Path(p) => p.contours[0].nodes.iter().map(|n| (n.anchor.x, n.anchor.y)).collect(),
            _ => Vec::new(),
        }
    };
    assert_eq!(anchors(&s), [(100.0, 100.0), (320.0, 227.0), (320.0, 227.0)]);
    events(&mut s, &[ToolEvent::down(200.0, 250.0), ToolEvent::up(200.0, 250.0)]);
    assert_eq!(anchors(&s), [(100.0, 100.0), (320.0, 227.0), (320.0, 227.0), (200.0, 250.0)], "drawing goes on");
    assert_eq!(top_elements(&s).len(), 1);
}
