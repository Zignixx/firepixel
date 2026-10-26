//! FB-100 (TW-267, RE spec 72 §2.4): rows bound to the step before them ("Stroke Color",
//! "Brush Stroke") do not count against `MaxUndos` and are dropped together with their step.
//! TW-265 (RE spec 72 §1.4): a stroke changed in the inspector while the Brush or Pencil is
//! active becomes the stroke that tool remembers.

use crate::common;
use common::*;
use fp_engine::tools::ToolId;
use fp_engine::{PrefValue, Preferences, Session};
use serde_json::json;

fn session(max_undos: i64) -> Session {
    let mut p = Preferences::new();
    p.set("MaxUndos", PrefValue::Int(max_undos));
    let mut s = Session::new(p);
    ex(&mut s, "file.new", json!({"width": 400, "height": 300}));
    s
}

fn labels(s: &Session) -> Vec<String> {
    s.active().map(|d| d.history.labels()).unwrap_or_default()
}

/// One real step (a rectangle) followed by one attached row (choosing the Brush), back to the
/// Pointer (no row).
fn real_and_attached(s: &mut Session, i: usize) {
    let x = (i * 15) as f32;
    rect(s, x, 0.0, x + 10.0, 10.0);
    s.set_tool(ToolId::Brush).expect("Brush");
    s.set_tool(ToolId::Pointer).expect("Pointer");
}

/// 20 real steps with an attached row after each: all 20 stay undoable with `MaxUndos` 20.
#[test]
fn fb100_attached_rows_do_not_count_against_max_undos() {
    fp_i18n::set_language("en");
    let mut s = session(20);
    for i in 0..20 {
        real_and_attached(&mut s, i);
    }
    let d = s.active().expect("doc");
    assert_eq!(d.history.len(), 40, "rows: {:?}", labels(&s));
    assert_eq!(d.history.real_undo_count(), 20);
    assert_eq!(d.history.dropped(), 0);
    assert_eq!(top_elements(&s).len(), 20);
    let mut undone = 0;
    while s.active().is_some_and(|d| d.history.can_undo()) {
        s.execute("edit.undo", json!({})).expect("undo");
        undone += 1;
    }
    assert_eq!(undone, 20, "every real step is undoable");
    assert!(top_elements(&s).is_empty());
    for _ in 0..20 {
        s.execute("edit.redo", json!({})).expect("redo");
    }
    assert_eq!(top_elements(&s).len(), 20);
    assert_eq!(s.active().expect("doc").history.len(), 40);
}

/// The oldest real step goes with its attached rows: the leading ones (before the first real
/// step) and the one directly after it; Edit menu labels, the panel position and jumping still
/// work.
#[test]
fn fb100_oldest_step_drops_with_its_attached_rows() {
    fp_i18n::set_language("en");
    let mut s = session(3);
    // a leading attached row before the first real step
    s.set_tool(ToolId::Brush).expect("Brush");
    s.set_tool(ToolId::Pointer).expect("Pointer");
    assert_eq!(labels(&s).len(), 1);
    for i in 0..3 {
        real_and_attached(&mut s, i);
    }
    assert_eq!(labels(&s).len(), 7, "{:?}", labels(&s));
    assert_eq!(s.active().expect("doc").history.dropped(), 0);
    rect(&mut s, 100.0, 100.0, 120.0, 120.0);
    let d = s.active().expect("doc");
    // dropped: the leading row, the first rectangle and its row
    assert_eq!(d.history.dropped(), 3);
    assert_eq!(d.history.real_undo_count(), 3);
    let rows = labels(&s);
    assert_eq!(rows.len(), 5, "{rows:?}");
    assert!(
        !d.history.steps()[0].attached(),
        "the panel starts with a real step"
    );
    assert_eq!(d.history.undo_label(), Some(rows[4].as_str()));
    // one more real step drops exactly one step and its attached row
    rect(&mut s, 130.0, 100.0, 150.0, 120.0);
    let d = s.active().expect("doc");
    assert_eq!(d.history.dropped(), 5);
    assert_eq!(d.history.len(), 4);
    assert_eq!(d.history.real_undo_count(), 3);
    // the Edit menu names the real step below an attached row
    s.set_tool(ToolId::Brush).expect("Brush");
    s.set_tool(ToolId::Pointer).expect("Pointer");
    let d = s.active().expect("doc");
    assert!(d.history.top().is_some_and(|t| t.attached()));
    assert_eq!(d.history.real_undo_count(), 3, "attached rows never trim");
    let real = d.history.steps()[3].label.clone();
    assert_eq!(d.history.undo_label(), Some(real.as_str()));
    s.execute("edit.undo", json!({})).expect("undo");
    assert_eq!(
        s.active().expect("doc").history.redo_label(),
        Some(real.as_str())
    );
    s.execute("edit.redo", json!({})).expect("redo");
    // jumping to the start of the panel keeps the two oldest rectangles
    let n = top_elements(&s).len();
    s.history_go_to(0).expect("go to");
    assert_eq!(top_elements(&s).len(), n - 3);
    s.history_go_to(5).expect("go to");
    assert_eq!(top_elements(&s).len(), n);
}

/// A rectangle (one real step), deselected so that a following tool choice compares with the
/// current stroke, not with the rectangle's.
fn real_step(s: &mut Session, x: f32) {
    rect(s, x, 10.0, x + 20.0, 30.0);
    select(s, &[]);
}

/// The row the tool choice `id` writes (`None` = none).
fn choose(s: &mut Session, id: ToolId) -> Option<String> {
    let n = labels(s).len();
    s.set_tool(id).expect("tool");
    let rows = labels(s);
    assert!(rows.len() <= n + 1, "{rows:?}");
    (rows.len() > n).then(|| rows[n].clone())
}

/// TW-265: changing the Brush stroke while the Brush is active, then a switch to the Pencil and
/// back over the Pointer: the rows follow the changed stroke ("Stroke Color", no "Brush Stroke").
#[test]
fn tw265_inspector_stroke_updates_the_tool_memory() {
    fp_i18n::set_language("en");
    let mut s = doc_session(400.0, 300.0);
    let sc = Some("Stroke Color".to_string());
    assert_eq!(choose(&mut s, ToolId::Brush), sc);
    ex(&mut s, "pi.tool.brush", json!({"width": 12}));
    assert_eq!(choose(&mut s, ToolId::Pointer), None);
    real_step(&mut s, 10.0);
    assert_eq!(
        choose(&mut s, ToolId::Brush),
        sc,
        "the Brush remembers the stroke changed while it was active"
    );
    real_step(&mut s, 40.0);
    ex(&mut s, "pi.tool.brush", json!({"width": 20}));
    assert_eq!(choose(&mut s, ToolId::Pencil), sc, "direct switch");
    assert_eq!(choose(&mut s, ToolId::Pointer), None);
    real_step(&mut s, 70.0);
    assert_eq!(
        choose(&mut s, ToolId::Pencil),
        sc,
        "the Pencil keeps the 20 px stroke"
    );
    assert_eq!(choose(&mut s, ToolId::Pointer), None);
    real_step(&mut s, 100.0);
    assert_eq!(
        choose(&mut s, ToolId::Brush),
        sc,
        "the Brush keeps the 20 px stroke"
    );
}

/// A stroke changed while another tool is active leaves the remembered stroke alone: choosing
/// the Brush then writes "Brush Stroke".
#[test]
fn tw265_stroke_changed_under_another_tool_writes_brush_stroke() {
    fp_i18n::set_language("en");
    let mut s = doc_session(400.0, 300.0);
    choose(&mut s, ToolId::Brush);
    choose(&mut s, ToolId::Pointer);
    ex(&mut s, "pi.tool.brush", json!({"width": 12}));
    real_step(&mut s, 10.0);
    assert_eq!(
        choose(&mut s, ToolId::Brush),
        Some("Brush Stroke".to_string())
    );
}
