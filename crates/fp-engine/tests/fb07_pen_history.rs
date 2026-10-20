//! FB-07 / FB-08 (re 55 §1, §2): every pen point is its own history step "Pen"; Ctrl+Z while
//! drawing removes only the last point and drawing goes on from the new last point; finishing adds
//! no step. Vector Path ignores Ctrl+Z while the stroke is drawn. Redo is redo only: without an
//! undone step it does nothing and the Edit menu reads "Can't Redo".

use crate::common;
use common::*;
use fp_engine::tools::ToolId;
use fp_engine::{Session, ToolEvent, ToolKey};
use fp_geom::Point;
use fp_model::{Contour, ElementId, ElementKind};
use serde_json::json;

fn session() -> (Session, ElementId) {
    fp_i18n::set_language("de");
    let mut s = doc_session(400.0, 300.0);
    let r = rect(&mut s, 24.0, 32.0, 115.0, 110.0);
    s.set_tool(ToolId::Pen).unwrap_or_else(|e| panic!("{e}"));
    (s, r)
}

fn click(s: &mut Session, x: f32, y: f32) {
    events(s, &[ToolEvent::down(x, y), ToolEvent::up(x, y)]);
}

fn drag(s: &mut Session, a: (f32, f32), b: (f32, f32)) {
    events(s, &[ToolEvent::down(a.0, a.1)]);
    for i in 1..=10 {
        let t = i as f32 / 10.0;
        events(s, &[ToolEvent::drag(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)]);
    }
    events(s, &[ToolEvent::up(b.0, b.1)]);
}

fn undo(s: &mut Session) {
    ex(s, "edit.undo", json!({}));
}

fn undo_count(s: &Session) -> usize {
    s.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

fn labels(s: &Session) -> Vec<String> {
    let d = s.active().expect("doc");
    d.history.steps()[..d.history.undo_count()].iter().map(|st| s.step_label(st)).collect()
}

/// The pen paths on the page (everything except the rectangle).
fn paths(s: &Session, r: ElementId) -> Vec<Vec<Contour>> {
    top_elements(s)
        .iter()
        .filter(|e| e.id != r)
        .filter_map(|e| match &e.kind {
            ElementKind::Path(p) => Some(p.contours.clone()),
            _ => None,
        })
        .collect()
}

fn anchors(c: &Contour) -> Vec<(f32, f32)> {
    c.nodes.iter().map(|n| (n.anchor.x, n.anchor.y)).collect()
}

fn pt(p: Point) -> (f32, f32) {
    (p.x, p.y)
}

/// re 55 §1.2: rectangle, three pen clicks → Rectangle, Pen, Pen, Pen; the path exists from the
/// first click and is selected.
#[test]
fn every_pen_point_is_one_step() {
    let (mut s, r) = session();
    let base = undo_count(&s);
    click(&mut s, 180.0, 45.0);
    let p = paths(&s, r);
    assert_eq!(p.len(), 1, "the first point creates the path");
    assert_eq!(anchors(&p[0][0]), [(180.0, 45.0)]);
    click(&mut s, 310.0, 58.0);
    click(&mut s, 284.0, 214.0);
    assert_eq!(undo_count(&s), base + 3);
    assert_eq!(labels(&s)[base..], ["Stift", "Stift", "Stift"]);
    let p = paths(&s, r);
    assert_eq!(p.len(), 1, "one path");
    assert_eq!(anchors(&p[0][0]), [(180.0, 45.0), (310.0, 58.0), (284.0, 214.0)]);
    let sel = s.active().map(|d| d.selection.elements.clone()).unwrap_or_default();
    assert_eq!(sel.len(), 1, "the path being drawn is selected");
}

/// re 55 §1.2: Ctrl+Z while drawing removes only the last point; the next click appends to the new
/// last point (drawing went on), the undone rows are dropped; undoing the first point removes the
/// path and the next Ctrl+Z undoes the rectangle.
#[test]
fn undo_while_drawing_removes_last_point_and_drawing_goes_on() {
    let (mut s, r) = session();
    click(&mut s, 180.0, 45.0);
    click(&mut s, 310.0, 58.0);
    click(&mut s, 284.0, 214.0);
    undo(&mut s);
    assert_eq!(anchors(&paths(&s, r)[0][0]), [(180.0, 45.0), (310.0, 58.0)]);
    undo(&mut s);
    assert_eq!(anchors(&paths(&s, r)[0][0]), [(180.0, 45.0)], "only the first point");
    assert_eq!(s.tools.active, ToolId::Pen);
    click(&mut s, 250.0, 150.0);
    let p = paths(&s, r);
    assert_eq!(p.len(), 1, "no second path: the click appended");
    assert_eq!(anchors(&p[0][0]), [(180.0, 45.0), (250.0, 150.0)]);
    assert!(!s.active().is_some_and(|d| d.history.can_redo()), "redo rows dropped");
    undo(&mut s);
    undo(&mut s);
    assert!(paths(&s, r).is_empty(), "first point undone: path removed");
    assert!(top_elements(&s).iter().any(|e| e.id == r), "rectangle still there");
    // no running path any more: the next click starts a new path, Ctrl+Z before undoes the rectangle
    undo(&mut s);
    assert!(top_elements(&s).is_empty(), "the next Ctrl+Z undoes the rectangle");
}

/// The handles of a curve point survive the undo of the following point: drawing goes on with
/// the curve's outgoing handle.
#[test]
fn undo_keeps_curve_handles_of_the_new_last_point() {
    let (mut s, r) = session();
    click(&mut s, 100.0, 200.0);
    drag(&mut s, (200.0, 200.0), (240.0, 160.0));
    click(&mut s, 300.0, 250.0);
    undo(&mut s);
    let c = paths(&s, r)[0][0].clone();
    assert_eq!(c.nodes.len(), 2);
    assert_eq!(pt(c.nodes[1].succ), (240.0, 160.0), "outgoing handle of the curve point");
    assert_eq!(pt(c.nodes[1].pred), (160.0, 240.0), "incoming handle of the curve point");
    // the rubber band starts at the curve point with its handle
    let ov = s.tool_event(&ToolEvent::drag(320.0, 220.0)).expect("hover").overlay;
    assert!(!ov.is_empty(), "drawing preview shown");
    click(&mut s, 320.0, 220.0);
    let c = paths(&s, r)[0][0].clone();
    assert_eq!(anchors(&c), [(100.0, 200.0), (200.0, 200.0), (320.0, 220.0)]);
    assert_eq!(pt(c.nodes[1].succ), (240.0, 160.0), "handle kept for the new segment");
}

/// Finishing (Enter, Escape, double-click, tool change) adds no step; Ctrl+Z afterwards removes just
/// the last point.
#[test]
fn finishing_adds_no_step() {
    for how in ["enter", "escape", "double", "tool"] {
        let (mut s, r) = session();
        click(&mut s, 180.0, 45.0);
        click(&mut s, 310.0, 58.0);
        click(&mut s, 284.0, 214.0);
        let n = undo_count(&s);
        match how {
            "enter" => events(&mut s, &[ToolEvent::key(ToolKey::Enter)]),
            "escape" => events(&mut s, &[ToolEvent::key(ToolKey::Escape)]),
            "double" => {
                events(
                    &mut s,
                    &[ToolEvent::down(284.0, 214.0), ToolEvent::up(284.0, 214.0), ToolEvent::double(284.0, 214.0)],
                );
            }
            _ => {
                s.set_tool(ToolId::Pointer).expect("tool");
            }
        }
        assert_eq!(undo_count(&s), n, "{how}: no extra step");
        assert_eq!(anchors(&paths(&s, r)[0][0]).len(), 3, "{how}");
        undo(&mut s);
        assert_eq!(anchors(&paths(&s, r)[0][0]), [(180.0, 45.0), (310.0, 58.0)], "{how}: Ctrl+Z removes the last point");
    }
}

/// Closing on the start point is one step "Pen"; the closed path gets the fill of a closed shape
/// and undoing it reopens the path.
#[test]
fn closing_is_one_step() {
    let (mut s, r) = session();
    click(&mut s, 150.0, 50.0);
    click(&mut s, 300.0, 60.0);
    click(&mut s, 250.0, 200.0);
    let n = undo_count(&s);
    click(&mut s, 150.0, 50.0);
    assert_eq!(undo_count(&s), n + 1, "closing click: one step");
    assert_eq!(labels(&s).last().map(String::as_str), Some("Stift"));
    let p = paths(&s, r);
    assert_eq!(p.len(), 1);
    assert!(p[0][0].closed);
    assert_eq!(p[0][0].nodes.len(), 3);
    undo(&mut s);
    let p = paths(&s, r);
    assert_eq!(p.len(), 1);
    assert!(!p[0][0].closed, "undo reopens the path");
    assert_eq!(p[0][0].nodes.len(), 3);
}

/// Line: one step on release. Vector Path: one step "Brush" on release; Ctrl+Z while the stroke is
/// drawn is ignored.
#[test]
fn line_and_vector_path_one_step_on_release() {
    let (mut s, r) = session();
    s.set_tool(ToolId::Line).expect("line");
    let n = undo_count(&s);
    drag(&mut s, (50.0, 200.0), (150.0, 250.0));
    assert_eq!(undo_count(&s), n + 1);
    s.set_tool(ToolId::VectorPath).expect("vector path");
    events(&mut s, &[ToolEvent::down(200.0, 150.0)]);
    for i in 1..=10 {
        events(&mut s, &[ToolEvent::drag(200.0 + 10.0 * i as f32, 150.0 + 3.0 * i as f32)]);
    }
    let before = s.active().map(|d| d.doc.clone()).expect("doc");
    ex(&mut s, "edit.undo", json!({}));
    assert!(
        std::sync::Arc::ptr_eq(&before, &s.active().expect("doc").doc),
        "Ctrl+Z while dragging is ignored"
    );
    assert_eq!(undo_count(&s), n + 1);
    events(&mut s, &[ToolEvent::up(300.0, 180.0)]);
    assert_eq!(undo_count(&s), n + 2, "one step on release");
    assert_eq!(labels(&s).last().map(String::as_str), Some("Pinsel"));
    assert_eq!(top_elements(&s).iter().filter(|e| e.id != r).count(), 2);
}

/// re 55 §2: Redo without an undone step does nothing (it does not repeat the last command); the
/// menu entry is disabled and reads "Can't Redo"; with an undone step "Redo: <step>".
#[test]
fn redo_without_undone_step_does_nothing() {
    fp_i18n::set_language("de");
    let mut s = doc_session(400.0, 300.0);
    rect(&mut s, 10.0, 10.0, 60.0, 60.0);
    let n = undo_count(&s);
    let count = top_elements(&s).len();
    assert!(!s.is_available("edit.redo"), "Redo disabled without an undone step");
    assert!(s.execute("edit.redo", json!({})).is_err(), "nothing to redo");
    assert_eq!(undo_count(&s), n, "no new step");
    assert_eq!(top_elements(&s).len(), count, "last command not repeated");
    let (undo_l, redo_l) = s.undo_redo_labels();
    assert!(undo_l.is_some());
    assert_eq!(redo_l, None, "no redo label: \"Can't Redo\"");
    ex(&mut s, "edit.undo", json!({}));
    assert!(s.is_available("edit.redo"));
    assert_eq!(s.undo_redo_labels().1, undo_l, "Redo: <undone step>");
    ex(&mut s, "edit.redo", json!({}));
    assert_eq!(top_elements(&s).len(), count);
    assert!(!s.is_available("edit.redo"));
    assert_eq!(fp_i18n::t("menu-edit-redoImpossible").replace('&', ""), "Wiederholen nicht möglich");
    assert_eq!(fp_i18n::t("menu-edit-undoImpossible").replace('&', ""), "Rückgängig nicht möglich");
    fp_i18n::set_language("en");
    assert_eq!(fp_i18n::t("menu-edit-redoImpossible").replace('&', ""), "Can't Redo");
    fp_i18n::set_language("de");
}
