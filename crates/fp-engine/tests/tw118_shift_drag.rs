//! TW-118: Shift on an already selected object never moves it with the pointer tool; a Shift-click
//! without drag deselects it, Shift on an unselected object adds it and moves the selection.

use crate::common;
use common::*;
use fp_engine::tools::ToolId;
use fp_engine::{Session, ToolEvent};
use fp_geom::Rect;
use fp_model::ElementId;

fn sel(s: &Session) -> Vec<ElementId> {
    s.active().map(|d| d.selection.elements.clone()).unwrap_or_default()
}

fn same(a: Rect, b: Rect) -> bool {
    (a.left - b.left).abs() < 0.01
        && (a.top - b.top).abs() < 0.01
        && (a.right - b.right).abs() < 0.01
        && (a.bottom - b.bottom).abs() < 0.01
}

fn setup() -> (Session, ElementId) {
    let mut s = doc_session(300.0, 200.0);
    let r1 = rect(&mut s, 50.0, 50.0, 150.0, 120.0);
    s.set_tool(ToolId::Pointer).expect("pointer");
    select(&mut s, &[r1]);
    (s, r1)
}

#[test]
fn shift_drag_of_selected_object_does_not_move_it() {
    let (mut s, r1) = setup();
    events(
        &mut s,
        &[
            ToolEvent::down(70.0, 100.0).shift(),
            ToolEvent::drag(100.0, 105.0).shift(),
            ToolEvent::drag(130.0, 110.0).shift(),
            ToolEvent::up(130.0, 110.0).shift(),
        ],
    );
    assert!(same(bounds(&s, r1), Rect::new(50.0, 50.0, 150.0, 120.0)), "{:?}", bounds(&s, r1));
    assert_eq!(sel(&s), vec![r1]);
}

#[test]
fn shift_drag_of_selected_object_is_additive_marquee() {
    let mut s = doc_session(300.0, 200.0);
    let r1 = rect(&mut s, 50.0, 50.0, 150.0, 120.0);
    let r2 = rect(&mut s, 160.0, 130.0, 180.0, 150.0);
    s.set_tool(ToolId::Pointer).expect("pointer");
    select(&mut s, &[r1]);
    events(
        &mut s,
        &[
            ToolEvent::down(140.0, 110.0).shift(),
            ToolEvent::drag(170.0, 140.0).shift(),
            ToolEvent::drag(190.0, 160.0).shift(),
            ToolEvent::up(190.0, 160.0).shift(),
        ],
    );
    assert!(same(bounds(&s, r1), Rect::new(50.0, 50.0, 150.0, 120.0)));
    assert!(same(bounds(&s, r2), Rect::new(160.0, 130.0, 180.0, 150.0)));
    let mut got = sel(&s);
    got.sort_by_key(|i| i.0);
    let mut want = vec![r1, r2];
    want.sort_by_key(|i| i.0);
    assert_eq!(got, want);
}

#[test]
fn shift_click_on_selected_without_drag_deselects() {
    let (mut s, _r1) = setup();
    // below the 3 px threshold counts as a click
    events(
        &mut s,
        &[
            ToolEvent::down(70.0, 100.0).shift(),
            ToolEvent::drag(71.5, 101.0).shift(),
            ToolEvent::up(71.5, 101.0).shift(),
        ],
    );
    assert!(sel(&s).is_empty(), "{:?}", sel(&s));
}

#[test]
fn shift_on_unselected_adds_and_moves() {
    let mut s = doc_session(300.0, 200.0);
    let r1 = rect(&mut s, 50.0, 50.0, 150.0, 120.0);
    let r2 = rect(&mut s, 200.0, 50.0, 250.0, 100.0);
    s.set_tool(ToolId::Pointer).expect("pointer");
    select(&mut s, &[r1]);
    events(
        &mut s,
        &[
            ToolEvent::down(220.0, 70.0).shift(),
            ToolEvent::drag(240.0, 75.0).shift(),
            ToolEvent::drag(250.0, 80.0).shift(),
            ToolEvent::up(250.0, 80.0).shift(),
        ],
    );
    let mut got = sel(&s);
    got.sort_by_key(|i| i.0);
    let mut want = vec![r1, r2];
    want.sort_by_key(|i| i.0);
    assert_eq!(got, want);
    // Shift held since the press does not constrain to an axis: both move by the same delta
    // (+30 horizontally, vertical part kept; object snapping may round it)
    let (b1, b2) = (bounds(&s, r1), bounds(&s, r2));
    let (dx, dy) = (b1.left - 50.0, b1.top - 50.0);
    assert!((dx - 30.0).abs() < 0.01 && dy >= 5.0, "{b1:?}");
    assert!(same(b2, Rect::new(200.0 + dx, 50.0 + dy, 250.0 + dx, 100.0 + dy)), "{b2:?}");
}

#[test]
fn alt_drag_moves_a_copy() {
    let (mut s, r1) = setup();
    let n = top_elements(&s).len();
    events(
        &mut s,
        &[
            ToolEvent::down(70.0, 100.0).alt(),
            ToolEvent::drag(100.0, 105.0).alt(),
            ToolEvent::drag(130.0, 110.0).alt(),
            ToolEvent::up(130.0, 110.0).alt(),
        ],
    );
    assert_eq!(top_elements(&s).len(), n + 1);
    assert!(same(bounds(&s, r1), Rect::new(50.0, 50.0, 150.0, 120.0)));
}
