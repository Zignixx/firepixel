//! FB-75 (preference `Firepixel_DuplicateWithCtrlDrag`, off by default): with it on, Ctrl-drag of
//! the selection with the Pointer drags a copy (cursor "+") and Alt-drag moves without copying;
//! Alt keeps its other roles (Alt-marquee deselect, Alt-click picks a group's child, Alt+arrow
//! duplicate nudge). Off, the reference rules stay (Alt-drag duplicates, Ctrl-drag moves like a
//! plain drag).
//!
//! FB-76 (preference `Firepixel_ShiftMarqueeAddsOnly`, off by default): with it on, the Pointer's
//! Shift-marquee only adds (cursor "+") and the Alt-marquee removes (Alt-marquee deselect implied
//! while on); off, the Shift-marquee toggles each object inside (TW-223).

use crate::common;
use common::*;
use fp_engine::prefs::PrefValue;
use fp_engine::tools::{
    ALT_MARQUEE_DESELECT_PREF, Cursor, DUPLICATE_WITH_CTRL_DRAG_PREF, SHIFT_MARQUEE_ADDS_ONLY_PREF,
    ToolId, ToolKey,
};
use fp_engine::{Session, ToolEvent};
use fp_model::ElementId;
use serde_json::json;

fn sel(s: &Session) -> Vec<ElementId> {
    let mut v = s
        .active()
        .map(|d| d.selection.elements.clone())
        .unwrap_or_default();
    v.sort_by_key(|i| i.0);
    v
}

fn sorted(mut v: Vec<ElementId>) -> Vec<ElementId> {
    v.sort_by_key(|i| i.0);
    v
}

fn pref(s: &mut Session, key: &str, on: bool) {
    s.prefs.set(key, PrefValue::Bool(on));
}

/// Drag from `a` to `b` with the modifiers of `m` held throughout; returns the cursor while dragging.
fn drag_m(s: &mut Session, a: (f32, f32), b: (f32, f32), m: fn(ToolEvent) -> ToolEvent) -> Cursor {
    let mid = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
    events(
        s,
        &[
            m(ToolEvent::down(a.0, a.1)),
            m(ToolEvent::drag(mid.0, mid.1)),
        ],
    );
    let c = s
        .tool_event(&m(ToolEvent::drag(b.0, b.1)))
        .map(|r| r.cursor)
        .unwrap_or_else(|e| panic!("{e}"));
    events(s, &[m(ToolEvent::up(b.0, b.1))]);
    c
}

/// Three rectangles: A, B on top, C lower right; Pointer active, nothing selected.
fn s5() -> (Session, ElementId, ElementId, ElementId) {
    let mut s = doc_session(600.0, 400.0);
    let a = rect(&mut s, 100.0, 80.0, 200.0, 160.0);
    let b = rect(&mut s, 250.0, 80.0, 350.0, 160.0);
    let c = rect(&mut s, 400.0, 200.0, 500.0, 300.0);
    ex(&mut s, "select.set", json!({"ids": [], "mode": "replace"}));
    s.set_tool(ToolId::Pointer)
        .unwrap_or_else(|e| panic!("{e}"));
    (s, a, b, c)
}

const AROUND_AB: ((f32, f32), (f32, f32)) = ((80.0, 60.0), (370.0, 180.0));
const AROUND_A: ((f32, f32), (f32, f32)) = ((80.0, 60.0), (220.0, 180.0));

/// FB-75 off (default): Ctrl-drag on a selected object moves it like a plain drag (no copy, move
/// cursor); Alt-drag duplicates with the "+" cursor.
#[test]
fn fb75_off_alt_drag_duplicates_ctrl_drag_moves() {
    let (mut s, a, _b, _c) = s5();
    assert!(
        !s.prefs.get_bool(DUPLICATE_WITH_CTRL_DRAG_PREF, false),
        "default off"
    );
    select(&mut s, &[a]);
    let n = top_elements(&s).len();
    let before = bounds(&s, a);
    let cur = drag_m(&mut s, (150.0, 120.0), (180.0, 150.0), |e| e.ctrl());
    assert_eq!(top_elements(&s).len(), n, "Ctrl-drag makes no copy");
    assert_eq!(cur, Cursor::Move);
    assert!(
        approx(bounds(&s, a).left, before.left + 30.0),
        "Ctrl-drag moved A"
    );
    let cur = drag_m(&mut s, (180.0, 150.0), (210.0, 170.0), |e| e.alt());
    assert_eq!(top_elements(&s).len(), n + 1, "Alt-drag duplicates");
    assert_eq!(cur, Cursor::Copy);
}

/// FB-75 on: Ctrl-drag duplicates (cursor "+"), Alt-drag moves without a copy, Ctrl pressed
/// during a plain move turns it into a copy, Alt+arrow still duplicates, and the Alt-marquee
/// deselect keeps working.
#[test]
fn fb75_on_ctrl_drag_duplicates_alt_drag_moves() {
    let (mut s, a, b, c) = s5();
    pref(&mut s, DUPLICATE_WITH_CTRL_DRAG_PREF, true);
    select(&mut s, &[a]);
    let n = top_elements(&s).len();
    let cur = drag_m(&mut s, (150.0, 120.0), (180.0, 150.0), |e| e.ctrl());
    assert_eq!(top_elements(&s).len(), n + 1, "Ctrl-drag duplicates");
    assert_eq!(cur, Cursor::Copy, "copy cursor with +");
    select(&mut s, &[a]);
    let before = bounds(&s, a);
    let cur = drag_m(&mut s, (110.0, 90.0), (110.0, 140.0), |e| e.alt());
    assert_eq!(top_elements(&s).len(), n + 1, "Alt-drag makes no copy");
    assert_eq!(cur, Cursor::Move, "move cursor without +");
    assert!(
        approx(bounds(&s, a).top, before.top + 50.0),
        "Alt-drag moved A"
    );
    // Ctrl pressed after the move started turns it into a copy
    select(&mut s, &[b]);
    events(
        &mut s,
        &[ToolEvent::down(300.0, 120.0), ToolEvent::drag(320.0, 130.0)],
    );
    let cur = s
        .tool_event(&ToolEvent::drag(340.0, 140.0).ctrl())
        .map(|r| r.cursor)
        .unwrap_or_else(|e| panic!("{e}"));
    events(&mut s, &[ToolEvent::up(340.0, 140.0).ctrl()]);
    assert_eq!(cur, Cursor::Copy);
    assert_eq!(
        top_elements(&s).len(),
        n + 2,
        "Ctrl during the drag duplicates"
    );
    // Alt+arrow keeps duplicating (measured key, not tied to the preference)
    select(&mut s, &[c]);
    events(&mut s, &[ToolEvent::key(ToolKey::Right).alt()]);
    assert_eq!(top_elements(&s).len(), n + 3, "Alt+arrow duplicates");
    // Alt-marquee deselect (FB-56) still works with Alt
    pref(&mut s, ALT_MARQUEE_DESELECT_PREF, true);
    select(&mut s, &[a, c]);
    let cur = drag_m(&mut s, AROUND_A.0, (215.0, 235.0), |e| e.alt());
    assert_eq!(sel(&s), vec![c], "Alt-marquee removed A");
    assert_eq!(cur, Cursor::Deselect);
}

/// FB-75 on: an Alt-click (no drag) on a group still selects the clicked child like the
/// Subselection; a Ctrl-click selects the group and makes no copy.
#[test]
fn fb75_on_alt_click_still_picks_the_child() {
    let (mut s, a, b, _c) = s5();
    select(&mut s, &[a, b]);
    let v = ex(&mut s, "modify.group", json!({}));
    let g = ElementId(
        v["id"]
            .as_u64()
            .or_else(|| v["ids"][0].as_u64())
            .unwrap_or(0),
    );
    pref(&mut s, DUPLICATE_WITH_CTRL_DRAG_PREF, true);
    ex(&mut s, "select.set", json!({"ids": [], "mode": "replace"}));
    events(
        &mut s,
        &[
            ToolEvent::down(150.0, 120.0).alt(),
            ToolEvent::up(150.0, 120.0).alt(),
        ],
    );
    assert_eq!(sel(&s), vec![a], "Alt-click picks the child A");
    let n = top_elements(&s).len();
    ex(&mut s, "select.set", json!({"ids": [], "mode": "replace"}));
    events(
        &mut s,
        &[
            ToolEvent::down(150.0, 120.0).ctrl(),
            ToolEvent::up(150.0, 120.0).ctrl(),
        ],
    );
    assert_eq!(sel(&s), vec![g], "Ctrl-click selects the group");
    assert_eq!(
        top_elements(&s).len(),
        n,
        "a Ctrl-click without a drag makes no copy"
    );
}

/// FB-76 off (default): the Pointer's Shift-marquee toggles each object inside, no badge.
#[test]
fn fb76_off_shift_marquee_toggles() {
    let (mut s, a, b, c) = s5();
    assert!(
        !s.prefs.get_bool(SHIFT_MARQUEE_ADDS_ONLY_PREF, false),
        "default off"
    );
    select(&mut s, &[a, c]);
    let cur = drag_m(&mut s, AROUND_AB.0, AROUND_AB.1, |e| e.shift());
    assert_eq!(sel(&s), sorted(vec![b, c]), "A toggled off, B toggled on");
    assert_eq!(cur, Cursor::Arrow);
    // Alt-marquee without FB-56 replaces (reference)
    drag_m(&mut s, AROUND_A.0, AROUND_A.1, |e| e.alt());
    assert_eq!(sel(&s), vec![a]);
}

/// FB-76 on: the Pointer's Shift-marquee (also Shift+Alt) only adds and shows "+"; the
/// Alt-marquee removes even with the FB-56 option off; the Subselection's Shift-marquee adds and
/// its Alt-marquee removes too.
#[test]
fn fb76_on_shift_marquee_adds_alt_marquee_removes() {
    let (mut s, a, b, c) = s5();
    pref(&mut s, SHIFT_MARQUEE_ADDS_ONLY_PREF, true);
    assert!(!s.prefs.get_bool(ALT_MARQUEE_DESELECT_PREF, false));
    select(&mut s, &[a, c]);
    let cur = drag_m(&mut s, AROUND_AB.0, AROUND_AB.1, |e| e.shift());
    assert_eq!(sel(&s), sorted(vec![a, b, c]), "A stays, B added");
    assert_eq!(cur, Cursor::Copy, "arrow with +");
    drag_m(&mut s, AROUND_AB.0, AROUND_AB.1, |e| e.shift().alt());
    assert_eq!(sel(&s), sorted(vec![a, b, c]), "Shift+Alt adds too");
    let cur = drag_m(&mut s, AROUND_A.0, AROUND_A.1, |e| e.alt());
    assert_eq!(sel(&s), sorted(vec![b, c]), "Alt-marquee removes A");
    assert_eq!(cur, Cursor::Deselect, "arrow with -");
    // Subselection
    s.set_tool(ToolId::Subselection)
        .unwrap_or_else(|e| panic!("{e}"));
    select(&mut s, &[c]);
    drag_m(&mut s, AROUND_AB.0, AROUND_AB.1, |e| e.shift());
    assert_eq!(sel(&s), sorted(vec![a, b, c]), "Subselection Shift adds");
    let cur = drag_m(&mut s, AROUND_A.0, AROUND_A.1, |e| e.alt());
    assert_eq!(sel(&s), sorted(vec![b, c]), "Subselection Alt removes");
    assert_eq!(cur, Cursor::Deselect);
}
