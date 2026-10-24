//! FB-56 (Firepixel extension, preference `Firepixel_AltMarqueeDeselects`, off by default): an
//! Alt-marquee of the Pointer or the Subselection started on empty canvas removes the objects
//! inside from the selection (objects outside stay selected, no History row), also with the lasso
//! marquee; the cursor shows the arrow with "−"; Alt-drag on an object still clones. Off, the
//! reference behaviour is unchanged (RE spec 69 section 5).

use crate::common;
use common::*;
use fp_engine::prefs::PrefValue;
use fp_engine::tools::{ALT_MARQUEE_DESELECT_PREF, Cursor, ToolId};
use fp_engine::{Session, ToolEvent};
use fp_model::ElementId;
use serde_json::json;

fn sel(s: &Session) -> Vec<ElementId> {
    let mut v = s.active().map(|d| d.selection.elements.clone()).unwrap_or_default();
    v.sort_by_key(|i| i.0);
    v
}

fn sorted(mut v: Vec<ElementId>) -> Vec<ElementId> {
    v.sort_by_key(|i| i.0);
    v
}

fn rows(s: &Session) -> usize {
    s.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

fn tool(s: &mut Session, t: ToolId) {
    s.set_tool(t).unwrap_or_else(|e| panic!("{e}"));
}

fn deselect_pref(s: &mut Session, on: bool) {
    s.prefs.set(ALT_MARQUEE_DESELECT_PREF, PrefValue::Bool(on));
}

/// Drag from `a` to `b` with the modifiers of `m` held throughout; returns the cursor while dragging.
fn drag_m(s: &mut Session, a: (f32, f32), b: (f32, f32), m: fn(ToolEvent) -> ToolEvent) -> Cursor {
    let mid = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
    events(s, &[m(ToolEvent::down(a.0, a.1)), m(ToolEvent::drag(mid.0, mid.1))]);
    let c = s.tool_event(&m(ToolEvent::drag(b.0, b.1))).map(|r| r.cursor).unwrap_or_else(|e| panic!("{e}"));
    events(s, &[m(ToolEvent::up(b.0, b.1))]);
    c
}

fn plain(e: ToolEvent) -> ToolEvent {
    e
}

/// Three rectangles: A, B on top, C lower right (like the measurement S5).
fn s5() -> (Session, ElementId, ElementId, ElementId) {
    let mut s = doc_session(600.0, 400.0);
    let a = rect(&mut s, 100.0, 80.0, 200.0, 160.0);
    let b = rect(&mut s, 250.0, 80.0, 350.0, 160.0);
    let c = rect(&mut s, 400.0, 200.0, 500.0, 300.0);
    ex(&mut s, "select.set", json!({"ids": [], "mode": "replace"}));
    (s, a, b, c)
}

const AROUND_AB: ((f32, f32), (f32, f32)) = ((80.0, 60.0), (370.0, 180.0));
const AROUND_A: ((f32, f32), (f32, f32)) = ((80.0, 60.0), (220.0, 180.0));

/// Preference off (default): the Alt-marquee works as in the reference.
#[test]
fn fb56_off_keeps_the_reference_alt_marquee() {
    let (mut s, a, b, c) = s5();
    assert!(!s.prefs.get_bool(ALT_MARQUEE_DESELECT_PREF, false), "default off");
    tool(&mut s, ToolId::Pointer);
    select(&mut s, &[a, b, c]);
    let cur = drag_m(&mut s, AROUND_A.0, AROUND_A.1, |e| e.alt());
    assert_eq!(sel(&s), vec![a], "Pointer Alt-marquee replaces");
    assert_eq!(cur, Cursor::Copy, "arrow with +");
    tool(&mut s, ToolId::Subselection);
    select(&mut s, &[a, b, c]);
    let cur = drag_m(&mut s, AROUND_A.0, AROUND_A.1, |e| e.alt());
    assert_eq!(sel(&s), vec![a], "Subselection Alt-marquee is a normal marquee");
    assert_eq!(cur, Cursor::Copy);
}

/// Preference on: the Pointer's Alt-marquee removes the objects inside, keeps the others, writes
/// no History row and shows the arrow with "−"; an Alt-click keeps the selection; Shift+Alt still
/// toggles, Shift and plain marquees are unchanged.
#[test]
fn fb56_pointer_alt_marquee_deselects() {
    let (mut s, a, b, c) = s5();
    deselect_pref(&mut s, true);
    tool(&mut s, ToolId::Pointer);
    select(&mut s, &[a, b, c]);
    let h = rows(&s);
    let n = top_elements(&s).len();
    let cur = drag_m(&mut s, AROUND_AB.0, AROUND_AB.1, |e| e.alt());
    assert_eq!(sel(&s), vec![c], "A and B removed, C stays");
    assert_eq!(cur, Cursor::Deselect, "arrow with -");
    assert_eq!(rows(&s), h, "no History row");
    assert_eq!(top_elements(&s).len(), n, "no clone");
    // a marquee around unselected objects changes nothing
    drag_m(&mut s, AROUND_A.0, AROUND_A.1, |e| e.alt());
    assert_eq!(sel(&s), vec![c]);
    // Alt-click on empty canvas keeps the selection
    events(&mut s, &[ToolEvent::down(560.0, 360.0).alt(), ToolEvent::up(560.0, 360.0).alt()]);
    assert_eq!(sel(&s), vec![c], "Alt-click keeps the selection");
    // Shift+Alt toggles as before
    drag_m(&mut s, AROUND_A.0, AROUND_A.1, |e| e.shift().alt());
    assert_eq!(sel(&s), sorted(vec![a, c]), "Shift+Alt toggles");
    // Shift alone toggles, plain replaces
    drag_m(&mut s, AROUND_A.0, AROUND_A.1, |e| e.shift());
    assert_eq!(sel(&s), vec![c]);
    drag_m(&mut s, AROUND_AB.0, AROUND_AB.1, plain);
    assert_eq!(sel(&s), sorted(vec![a, b]));
    assert_eq!(rows(&s), h, "marquee selections write no History row");
}

/// Preference on: a selected group touched by the marquee is removed, and so is a selected child
/// of a group (picked earlier one by one).
#[test]
fn fb56_pointer_deselect_groups_and_children() {
    let (mut s, a, b, c) = s5();
    select(&mut s, &[a, b]);
    let v = ex(&mut s, "modify.group", json!({}));
    let g = ElementId(v["id"].as_u64().or_else(|| v["ids"][0].as_u64()).unwrap_or(0));
    deselect_pref(&mut s, true);
    tool(&mut s, ToolId::Pointer);
    select(&mut s, &[g, c]);
    drag_m(&mut s, AROUND_A.0, AROUND_A.1, |e| e.alt());
    assert_eq!(sel(&s), vec![c], "the group removed");
    ex(&mut s, "select.set", json!({"ids": [a.0, b.0, c.0], "mode": "replace"}));
    drag_m(&mut s, AROUND_A.0, AROUND_A.1, |e| e.alt());
    assert_eq!(sel(&s), sorted(vec![b, c]), "the child A removed, B and C stay");
}

/// Preference on: the Subselection's Alt-marquee removes the subselected objects inside.
#[test]
fn fb56_subselection_alt_marquee_deselects() {
    let (mut s, a, b, c) = s5();
    deselect_pref(&mut s, true);
    tool(&mut s, ToolId::Subselection);
    drag_m(&mut s, AROUND_AB.0, AROUND_AB.1, plain);
    assert_eq!(sel(&s), sorted(vec![a, b]));
    let h = rows(&s);
    let cur = drag_m(&mut s, AROUND_A.0, AROUND_A.1, |e| e.alt());
    assert_eq!(sel(&s), vec![b], "A removed, B stays");
    assert_eq!(cur, Cursor::Deselect);
    assert_eq!(rows(&s), h, "no History row");
    // Shift-marquee still adds
    drag_m(&mut s, AROUND_A.0, AROUND_A.1, |e| e.shift());
    assert_eq!(sel(&s), sorted(vec![a, b]));
    let _ = c;
}

/// Preference on with the lasso marquee: the Alt-lasso removes the objects it encloses.
#[test]
fn fb56_lasso_alt_marquee_deselects() {
    let (mut s, a, b, c) = s5();
    deselect_pref(&mut s, true);
    ex(&mut s, "tool.pointerLasso", json!({"on": true}));
    tool(&mut s, ToolId::Pointer);
    select(&mut s, &[a, b, c]);
    let h = rows(&s);
    // an L-shaped outline around A and C, leaving out B
    let outline = [(80.0, 60.0), (220.0, 60.0), (220.0, 180.0), (520.0, 180.0), (520.0, 320.0), (80.0, 320.0)];
    let mut evs = vec![ToolEvent::down(70.0, 50.0).alt()];
    evs.extend(outline.iter().map(|(x, y)| ToolEvent::drag(*x, *y).alt()));
    let mut cur = Cursor::Arrow;
    for e in &evs {
        cur = s.tool_event(e).map(|r| r.cursor).unwrap_or_else(|e| panic!("{e}"));
    }
    events(&mut s, &[ToolEvent::up(70.0, 330.0).alt()]);
    assert_eq!(sel(&s), vec![b], "A and C removed, B stays");
    assert_eq!(cur, Cursor::Deselect);
    assert_eq!(rows(&s), h, "no History row");
    // the Subselection's Alt-lasso too
    tool(&mut s, ToolId::Subselection);
    select(&mut s, &[a, b, c]);
    let h = rows(&s);
    let mut evs = vec![ToolEvent::down(70.0, 50.0).alt()];
    evs.extend(outline.iter().map(|(x, y)| ToolEvent::drag(*x, *y).alt()));
    evs.push(ToolEvent::up(70.0, 330.0).alt());
    events(&mut s, &evs);
    assert_eq!(sel(&s), vec![b], "Subselection: A and C removed");
    assert_eq!(rows(&s), h);
}

/// Preference on: Alt-drag starting on an object still clones it.
#[test]
fn fb56_alt_drag_on_an_object_still_clones() {
    let (mut s, a, _b, _c) = s5();
    deselect_pref(&mut s, true);
    tool(&mut s, ToolId::Pointer);
    select(&mut s, &[a]);
    let n = top_elements(&s).len();
    let cur = drag_m(&mut s, (150.0, 120.0), (180.0, 150.0), |e| e.alt());
    assert_eq!(top_elements(&s).len(), n + 1, "Alt-drag clones");
    assert_eq!(cur, Cursor::Copy, "clone cursor with +");
}
