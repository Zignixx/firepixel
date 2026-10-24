//! FB-44 rest (RE spec 69 §3): fill handles with the Pointer. A drag on the line h1–h2 or h1–h3
//! turns both arms rigidly about h1 (lengths kept, the dragged arm points at the mouse, one row
//! `element.rotateFillHandles` like the reference's "rotate1" / "rotate2"); Shift limits the drag
//! to the dominant axis; Alt on a handle clones the object instead of dragging the handle; the
//! cursor is a hollow arrow over a handle and a rotate arrow over a handle line. Measured values
//! of the reference: pattern h1 (250,200), h2 (344,200), h3 (250,112); linear gradient h1 (200,200),
//! h2 (320,200), h3 (200,80).

use crate::common;
use common::*;
use fp_engine::tools::{Cursor, Overlay, ToolId};
use fp_engine::{Session, ToolEvent};
use fp_geom::Point;
use fp_model::{ElementId, ElementKind, FillHandles};
use serde_json::json;

/// Test pattern 16 × 12 in the test profile's pattern folder, written once per test program.
fn write_pattern() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let _ = doc_session(10.0, 10.0); // test profile
        let user = fp_i18n::profile::roaming_dir().expect("test profile");
        let path = user.join("Patterns").join("Fb44RotMuster.png");
        let mut s = doc_session(16.0, 12.0);
        let _ = rect(&mut s, 0.0, 0.0, 8.0, 12.0);
        std::fs::create_dir_all(path.parent().expect("folder")).expect("create folder");
        ex(
            &mut s,
            "file.exportArea",
            json!({"rect": [0, 0, 16, 12], "path": path.to_string_lossy(), "settings": {"format": "png32"}}),
        );
        assert!(path.is_file(), "{path:?}");
    });
}

fn handles(s: &Session, id: ElementId) -> FillHandles {
    match &el(s, id).kind {
        ElementKind::Path(p) => p.fill_handles,
        _ => panic!("path"),
    }
}

fn near(p: Point, x: f32, y: f32) -> bool {
    (p.x - x).abs() < 0.01 && (p.y - y).abs() < 0.01
}

fn len(a: Point, b: Point) -> f32 {
    ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt()
}

fn undo_count(s: &Session) -> usize {
    s.active().expect("doc").history.undo_count()
}

/// Command of the last done History step.
fn last_command(s: &Session) -> String {
    let d = s.active().expect("doc");
    let n = d.history.undo_count();
    d.history.steps()[n - 1].command.clone()
}

fn object_count(s: &Session) -> usize {
    top_elements(s).len()
}

/// Drag in 20 steps like a real mouse; `m` adds the modifiers to every event.
fn drag_with(s: &mut Session, a: (f32, f32), b: (f32, f32), m: fn(ToolEvent) -> ToolEvent) {
    events(s, &[m(ToolEvent::down(a.0, a.1))]);
    for i in 1..=20 {
        let t = i as f32 / 20.0;
        events(
            s,
            &[m(ToolEvent::drag(
                a.0 + (b.0 - a.0) * t,
                a.1 + (b.1 - a.1) * t,
            ))],
        );
    }
    events(s, &[m(ToolEvent::up(b.0, b.1))]);
}

fn drag(s: &mut Session, a: (f32, f32), b: (f32, f32)) {
    drag_with(s, a, b, |e| e);
}

/// Pattern rectangle 100,60–400,340 with the measured handles, selected, Pointer active.
fn pattern_rect(s: &mut Session) -> ElementId {
    write_pattern();
    let r = rect(s, 100.0, 60.0, 400.0, 340.0);
    select(s, &[r]);
    ex(s, "pi.fill", json!({"pattern": "Fb44RotMuster"}));
    ex(
        s,
        "element.setFillHandles",
        json!({"ids": [r.0], "start": [250, 200], "end": [344, 200], "second": [250, 112]}),
    );
    s.set_tool(ToolId::Pointer).expect("Pointer");
    r
}

/// Linear gradient h1 (200,200) → h2 (320,200); h3 becomes (200,80).
fn gradient_rect(s: &mut Session) -> ElementId {
    let r = rect(s, 100.0, 60.0, 400.0, 340.0);
    select(s, &[r]);
    ex(
        s,
        "element.setGradient",
        json!({"from": [200, 200], "to": [320, 200]}),
    );
    s.set_tool(ToolId::Pointer).expect("Pointer");
    r
}

#[test]
fn rotate_command_turns_both_arms_about_h1() {
    let mut s = doc_session(600.0, 400.0);
    let r = pattern_rect(&mut s);
    let n0 = undo_count(&s);
    ex(
        &mut s,
        "element.rotateFillHandles",
        json!({"ids": [r.0], "axis": 1, "toward": [47, 40]}),
    );
    let h = handles(&s, r);
    assert!(near(h.start, 250.0, 200.0), "{h:?}");
    assert!(near(h.end, 321.585, 260.923), "{h:?}");
    assert!(near(h.second, 307.034, 132.985), "{h:?}");
    assert_eq!(undo_count(&s), n0 + 1, "one step");
    ex(&mut s, "edit.undo", json!({}));
    assert!(near(handles(&s, r).end, 344.0, 200.0));
    // axis 2: the h3 arm points at the mouse, both lengths stay
    ex(
        &mut s,
        "element.rotateFillHandles",
        json!({"axis": 2, "toward": [40, -44]}),
    );
    let h = handles(&s, r);
    let a = (h.second.y - h.start.y).atan2(h.second.x - h.start.x);
    assert!((a - (-44f32).atan2(40.0)).abs() < 1e-4, "{h:?}");
    assert!((len(h.start, h.end) - 94.0).abs() < 0.01);
    assert!((len(h.start, h.second) - 88.0).abs() < 0.01);
}

#[test]
fn line_drag_rotates_pattern_handles_in_one_row() {
    fp_i18n::set_language("de");
    let mut s = doc_session(600.0, 400.0);
    let r = pattern_rect(&mut s);
    let n0 = undo_count(&s);
    let b0 = bounds(&s, r);
    // p8: middle of the line h1–h2, 0/+40
    events(&mut s, &[ToolEvent::down(297.0, 200.0)]);
    events(&mut s, &[ToolEvent::drag(297.0, 220.0)]);
    let fb = s.tool_feedback();
    assert!(
        fb.iter().any(|o| matches!(o, Overlay::FillLive { .. })),
        "the fill follows live: {fb:?}"
    );
    for i in 1..=10 {
        events(&mut s, &[ToolEvent::drag(297.0, 220.0 + 2.0 * i as f32)]);
    }
    events(&mut s, &[ToolEvent::up(297.0, 240.0)]);
    let h = handles(&s, r);
    assert!(near(h.start, 250.0, 200.0), "h1 stays: {h:?}");
    assert!(near(h.end, 321.585, 260.923), "{h:?}");
    assert!(near(h.second, 307.034, 132.985), "{h:?}");
    assert_eq!(undo_count(&s), n0 + 1, "one row");
    assert_eq!(last_command(&s), "element.rotateFillHandles");
    assert_eq!(
        s.active()
            .and_then(|d| d.history.undo_label().map(str::to_string)),
        Some("Verlaufsgriffe drehen".to_string())
    );
    assert_eq!(bounds(&s, r), b0, "the object stays");
    ex(&mut s, "edit.undo", json!({}));
    assert!(
        near(handles(&s, r).end, 344.0, 200.0),
        "undo puts the handles back"
    );
    // p9: the line h1–h3, +40/0: the h3 arm points at the mouse, h2 turns with it
    drag(&mut s, (250.0, 156.0), (290.0, 156.0));
    let h = handles(&s, r);
    let a3 = (h.second.y - h.start.y).atan2(h.second.x - h.start.x);
    assert!((a3 - (-44f32).atan2(40.0)).abs() < 1e-3, "{h:?}");
    let a2 = (h.end.y - h.start.y).atan2(h.end.x - h.start.x);
    assert!(
        (a2 - (90f32 - 47.726).to_radians()).abs() < 1e-3,
        "h2 turned by the same angle: {h:?}"
    );
    assert!((len(h.start, h.end) - 94.0).abs() < 0.01, "lengths stay");
    assert!((len(h.start, h.second) - 88.0).abs() < 0.01, "lengths stay");
    assert_eq!(last_command(&s), "element.rotateFillHandles");
}

#[test]
fn line_drag_rotates_a_linear_gradient() {
    let mut s = doc_session(600.0, 400.0);
    let r = gradient_rect(&mut s);
    let h0 = handles(&s, r);
    assert!(near(h0.second, 200.0, 80.0), "{h0:?}");
    // g: line h1–h2 0/+40 from its middle
    drag(&mut s, (260.0, 200.0), (260.0, 240.0));
    let h = handles(&s, r);
    assert!(near(h.start, 200.0, 200.0), "{h:?}");
    assert!(near(h.end, 299.846, 266.564), "{h:?}");
    assert!(near(h.second, 266.564, 100.154), "{h:?}");
}

#[test]
fn shift_limits_handle_drags_to_the_dominant_axis() {
    let mut s = doc_session(600.0, 400.0);
    let r = pattern_rect(&mut s);
    // p4: Shift + h2 +10/+40: only y
    drag_with(&mut s, (344.0, 200.0), (354.0, 240.0), ToolEvent::shift);
    let h = handles(&s, r);
    assert!(near(h.end, 344.0, 240.0), "{h:?}");
    assert!(
        near(h.start, 250.0, 200.0) && near(h.second, 250.0, 112.0),
        "{h:?}"
    );
    ex(&mut s, "edit.undo", json!({}));
    // p6: Shift + h1 +40/+10: all three +40/0
    drag_with(&mut s, (250.0, 200.0), (290.0, 210.0), ToolEvent::shift);
    let h = handles(&s, r);
    assert!(near(h.start, 290.0, 200.0), "{h:?}");
    assert!(near(h.end, 384.0, 200.0), "{h:?}");
    assert!(near(h.second, 290.0, 112.0), "{h:?}");
    ex(&mut s, "edit.undo", json!({}));
    // the gradient: Shift + h1 +40/+10 → +40/0
    let g = gradient_rect(&mut s);
    drag_with(&mut s, (200.0, 200.0), (240.0, 210.0), ToolEvent::shift);
    assert!(near(handles(&s, g).start, 240.0, 200.0));
    assert!(near(handles(&s, g).end, 360.0, 200.0));
}

#[test]
fn alt_on_a_handle_clones_the_object() {
    let mut s = doc_session(600.0, 400.0);
    let r = pattern_rect(&mut s);
    let h0 = handles(&s, r);
    let n0 = object_count(&s);
    // p5: Alt + h2 0/+60
    drag_with(&mut s, (344.0, 200.0), (344.0, 260.0), ToolEvent::alt);
    assert_eq!(object_count(&s), n0 + 1, "a copy");
    assert_eq!(handles(&s, r), h0, "the original's handles stay");
    let copy = s
        .active()
        .and_then(|d| d.selection.elements.first().copied())
        .expect("copy selected");
    assert_ne!(copy, r);
    let hc = handles(&s, copy);
    assert!(
        near(hc.start, 250.0, 260.0) && near(hc.end, 344.0, 260.0),
        "{hc:?}"
    );
    assert!(
        (bounds(&s, copy).top - 120.0).abs() < 1e-3,
        "copy moved 0/+60"
    );
    // p7: Alt + h1 of the copy +40/+30 makes another copy
    drag_with(&mut s, (250.0, 260.0), (290.0, 290.0), ToolEvent::alt);
    assert_eq!(object_count(&s), n0 + 2);
    assert_eq!(handles(&s, copy), hc, "the dragged copy's handles stay");
}

#[test]
fn hover_and_drag_cursors_over_handles_and_lines() {
    let mut s = doc_session(600.0, 400.0);
    let _r = pattern_rect(&mut s);
    let hover = |s: &mut Session, x: f32, y: f32| {
        s.tool_event(&ToolEvent::drag(x, y)).expect("hover").cursor
    };
    assert_eq!(hover(&mut s, 344.0, 200.0), Cursor::FillHandle, "h2");
    assert_eq!(hover(&mut s, 250.0, 200.0), Cursor::FillHandle, "h1");
    assert_eq!(hover(&mut s, 250.0, 113.0), Cursor::FillHandle, "h3");
    assert_eq!(
        hover(&mut s, 297.0, 201.0),
        Cursor::FillRotate,
        "line h1–h2"
    );
    assert_eq!(
        hover(&mut s, 251.0, 156.0),
        Cursor::FillRotate,
        "line h1–h3"
    );
    assert_ne!(
        hover(&mut s, 150.0, 300.0),
        Cursor::FillRotate,
        "elsewhere in the object"
    );
    // while dragging: the same cursors; Alt on a handle drags a copy
    let c = s
        .tool_event(&ToolEvent::down(297.0, 200.0))
        .expect("down")
        .cursor;
    assert_eq!(c, Cursor::FillRotate);
    let c = s
        .tool_event(&ToolEvent::drag(297.0, 230.0))
        .expect("drag")
        .cursor;
    assert_eq!(c, Cursor::FillRotate);
    let _ = s.tool_event(&ToolEvent::up(297.0, 230.0));
    let c = s
        .tool_event(&ToolEvent::down(250.0, 200.0).alt())
        .expect("down")
        .cursor;
    assert_eq!(c, Cursor::Copy, "Alt on a handle: copy cursor");
    let _ = s.tool_event(&ToolEvent::up(250.0, 200.0).alt());
}
