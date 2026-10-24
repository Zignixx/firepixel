//! FB-32 / FB-33 (RE spec 69 sections 4 and 5, measured): cursor badges of the bitmap selection
//! tools only while a pixel selection exists (Shift "+", Alt "-", Shift+Alt "x" in either order,
//! inside the selection the move cursors), Alt first then Shift intersects; the Pointer's
//! Shift-marquee toggles, its Alt-marquee selects like the Subselection, the Subselection's
//! Shift-marquee adds; marquee selections write no History row. Firepixel extension: the lasso
//! marquee of the Pointer and the Subselection (`Firepixel_PointerLasso`).

use crate::common;
use common::*;
use fp_engine::tools::{Cursor, ToolId};
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

/// Mouse move without a button: the hover cursor.
fn hover(s: &mut Session, ev: ToolEvent) -> Cursor {
    s.tool_event(&ev).map(|r| r.cursor).unwrap_or_else(|e| panic!("{e}"))
}

/// Drag from `a` to `b` with the modifiers of `m` held throughout.
fn drag_m(s: &mut Session, a: (f32, f32), b: (f32, f32), m: fn(ToolEvent) -> ToolEvent) {
    let mid = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
    events(
        s,
        &[
            m(ToolEvent::down(a.0, a.1)),
            m(ToolEvent::drag(mid.0, mid.1)),
            m(ToolEvent::drag(b.0, b.1)),
            m(ToolEvent::up(b.0, b.1)),
        ],
    );
}

fn plain(e: ToolEvent) -> ToolEvent {
    e
}

fn pixel_bounds(s: &Session) -> Option<fp_geom::IRect> {
    s.active()
        .and_then(|d| d.selection.pixels.as_ref())
        .map(|m| m.tight_bounds())
}

/// Bitmap tools: no badge without a pixel selection, badges with one, move cursors inside.
#[test]
fn fb32_bitmap_selection_cursor_badges() {
    let mut s = doc_session(400.0, 300.0);
    bitmap(&mut s, 0, 0, 400, 300, [200, 200, 200, 255]);
    let cases = [
        (ToolId::Marquee, Cursor::Crosshair, [Cursor::CrosshairAdd, Cursor::CrosshairSubtract, Cursor::CrosshairIntersect]),
        (ToolId::Lasso, Cursor::Lasso, [Cursor::LassoAdd, Cursor::LassoSubtract, Cursor::LassoIntersect]),
        (ToolId::MagicWand, Cursor::Wand, [Cursor::WandAdd, Cursor::WandSubtract, Cursor::WandIntersect]),
    ];
    // without a pixel selection: the plain glyph with every modifier
    ex(&mut s, "select.pixels", json!({"shape": "none"}));
    for (t, base, _) in cases {
        tool(&mut s, t);
        assert_eq!(hover(&mut s, ToolEvent::drag(50.0, 50.0)), base, "{t:?} plain");
        assert_eq!(hover(&mut s, ToolEvent::drag(51.0, 50.0).shift()), base, "{t:?} Shift, no selection");
        assert_eq!(hover(&mut s, ToolEvent::drag(52.0, 50.0).alt()), base, "{t:?} Alt, no selection");
        assert_eq!(hover(&mut s, ToolEvent::drag(53.0, 50.0).shift().alt()), base, "{t:?} Shift+Alt, no selection");
    }
    // with a pixel selection 200,150 - 300,250
    ex(&mut s, "select.pixels", json!({"shape": "rect", "rect": [200, 150, 300, 250]}));
    for (t, base, [add, sub, int]) in cases {
        tool(&mut s, t);
        assert_eq!(hover(&mut s, ToolEvent::drag(50.0, 50.0)), base, "{t:?} outside, plain");
        assert_eq!(hover(&mut s, ToolEvent::drag(51.0, 50.0).shift()), add, "{t:?} Shift");
        assert_eq!(hover(&mut s, ToolEvent::drag(52.0, 50.0).alt()), sub, "{t:?} Alt");
        assert_eq!(hover(&mut s, ToolEvent::drag(53.0, 50.0).shift().alt()), int, "{t:?} Shift+Alt");
        assert_eq!(hover(&mut s, ToolEvent::drag(54.0, 50.0).ctrl()), base, "{t:?} Ctrl outside");
        assert_eq!(hover(&mut s, ToolEvent::drag(250.0, 200.0)), Cursor::MoveMarquee, "{t:?} inside");
        assert_eq!(hover(&mut s, ToolEvent::drag(251.0, 200.0).ctrl()), Cursor::MovePixels, "{t:?} inside + Ctrl");
        assert_eq!(hover(&mut s, ToolEvent::drag(252.0, 200.0).shift()), add, "{t:?} inside + Shift adds");
    }
}

/// Lasso and Magic Wand drag the marquee inside the pixel selection like the Marquee (the move
/// cursor promises it): the selection moves, the pixels stay.
#[test]
fn fb32_lasso_and_wand_move_the_marquee_inside_the_selection() {
    for t in [ToolId::Lasso, ToolId::MagicWand] {
        let mut s = doc_session(400.0, 300.0);
        bitmap(&mut s, 0, 0, 400, 300, [200, 200, 200, 255]);
        ex(&mut s, "select.pixels", json!({"shape": "rect", "rect": [200, 150, 300, 250]}));
        tool(&mut s, t);
        drag_m(&mut s, (250.0, 200.0), (270.0, 210.0), plain);
        assert_eq!(pixel_bounds(&s), Some(fp_geom::IRect::new(220, 160, 320, 260)), "{t:?}");
    }
}

/// RE spec 69 section 4.2: Alt pressed first, then Shift, intersects (like Shift first).
#[test]
fn fb32_marquee_alt_then_shift_intersects() {
    for alt_first in [false, true] {
        let mut s = doc_session(400.0, 300.0);
        bitmap(&mut s, 0, 0, 400, 300, [200, 200, 200, 255]);
        tool(&mut s, ToolId::Marquee);
        drag_m(&mut s, (100.0, 80.0), (200.0, 180.0), plain);
        assert_eq!(pixel_bounds(&s), Some(fp_geom::IRect::new(100, 80, 200, 180)));
        let first = if alt_first { ToolEvent::drag(150.0, 130.0).alt() } else { ToolEvent::drag(150.0, 130.0).shift() };
        let c = hover(&mut s, first);
        assert_eq!(c, if alt_first { Cursor::CrosshairSubtract } else { Cursor::CrosshairAdd });
        assert_eq!(hover(&mut s, ToolEvent::drag(150.0, 131.0).shift().alt()), Cursor::CrosshairIntersect, "x in either order");
        drag_m(&mut s, (150.0, 130.0), (300.0, 250.0), |e| e.shift().alt());
        assert_eq!(pixel_bounds(&s), Some(fp_geom::IRect::new(150, 130, 200, 180)), "intersect, alt first: {alt_first}");
    }
}

/// Pointer hover: Alt = hollow Subselection arrow, Shift and Ctrl no badge; any Alt drag (clone or
/// Alt-marquee) the arrow with "+"; the Subselection's Alt-marquee too, its Shift-marquee nothing.
#[test]
fn fb32_pointer_and_subselection_cursors() {
    let mut s = doc_session(600.0, 400.0);
    let a = rect(&mut s, 100.0, 80.0, 200.0, 160.0);
    tool(&mut s, ToolId::Pointer);
    ex(&mut s, "select.set", json!({"ids": [], "mode": "replace"}));
    assert_eq!(hover(&mut s, ToolEvent::drag(400.0, 300.0)), Cursor::Arrow);
    assert_eq!(hover(&mut s, ToolEvent::drag(401.0, 300.0).alt()), Cursor::Subselect, "Alt over empty canvas");
    assert_eq!(hover(&mut s, ToolEvent::drag(150.0, 120.0).alt()), Cursor::Subselect, "Alt over an object");
    assert_eq!(hover(&mut s, ToolEvent::drag(151.0, 120.0).shift()), Cursor::Arrow, "Shift: no badge");
    assert_eq!(hover(&mut s, ToolEvent::drag(152.0, 120.0).ctrl()), Cursor::Arrow, "Ctrl: plain arrow");
    assert_eq!(hover(&mut s, ToolEvent::drag(153.0, 120.0).shift().alt()), Cursor::Subselect);
    // Alt-marquee on empty canvas: arrow with "+"
    let mid = s.tool_event(&ToolEvent::down(300.0, 250.0).alt()).map(|r| r.cursor).ok();
    assert!(mid.is_some());
    let c = s.tool_event(&ToolEvent::drag(350.0, 300.0).alt()).map(|r| r.cursor).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(c, Cursor::Copy, "Pointer Alt-marquee");
    events(&mut s, &[ToolEvent::up(350.0, 300.0).alt()]);
    // Shift-marquee: no badge
    events(&mut s, &[ToolEvent::down(300.0, 250.0).shift()]);
    let c = s.tool_event(&ToolEvent::drag(350.0, 300.0).shift()).map(|r| r.cursor).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(c, Cursor::Arrow, "Pointer Shift-marquee");
    events(&mut s, &[ToolEvent::up(350.0, 300.0).shift()]);
    // Alt-drag on an object (clone): arrow with "+"
    select(&mut s, &[a]);
    events(&mut s, &[ToolEvent::down(150.0, 120.0).alt()]);
    let c = s.tool_event(&ToolEvent::drag(180.0, 150.0).alt()).map(|r| r.cursor).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(c, Cursor::Copy, "Pointer Alt-drag clone");
    events(&mut s, &[ToolEvent::up(180.0, 150.0).alt()]);
    // Subselection
    tool(&mut s, ToolId::Subselection);
    ex(&mut s, "select.set", json!({"ids": [], "mode": "replace"}));
    let base = hover(&mut s, ToolEvent::drag(400.0, 300.0));
    assert_eq!(hover(&mut s, ToolEvent::drag(401.0, 300.0).alt()), base, "Subselection Alt hover: no badge");
    assert_eq!(hover(&mut s, ToolEvent::drag(402.0, 300.0).shift()), base, "Subselection Shift hover: no badge");
    events(&mut s, &[ToolEvent::down(300.0, 250.0).alt()]);
    let c = s.tool_event(&ToolEvent::drag(350.0, 300.0).alt()).map(|r| r.cursor).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(c, Cursor::Copy, "Subselection Alt-marquee");
    events(&mut s, &[ToolEvent::up(350.0, 300.0).alt()]);
    events(&mut s, &[ToolEvent::down(300.0, 250.0).shift()]);
    let c = s.tool_event(&ToolEvent::drag(350.0, 300.0).shift()).map(|r| r.cursor).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(c, base, "Subselection Shift-marquee: no badge");
    events(&mut s, &[ToolEvent::up(350.0, 300.0).shift()]);
}

/// Three named rectangles like the measurement S5(): A, B on top, C lower right.
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
const AROUND_C: ((f32, f32), (f32, f32)) = ((380.0, 190.0), (520.0, 320.0));

/// RE spec 69 section 5.2 (`5a`–`5i`): the Pointer marquee and its modifiers, no History row.
#[test]
fn fb33_pointer_marquee_modifiers() {
    let (mut s, a, b, c) = s5();
    tool(&mut s, ToolId::Pointer);
    let h0 = rows(&s);
    // 5c: plain marquee around A, B
    drag_m(&mut s, AROUND_AB.0, AROUND_AB.1, plain);
    assert_eq!(sel(&s), sorted(vec![a, b]), "5c");
    // 5h: only A selected, Shift-marquee around A, B toggles: only B
    select(&mut s, &[a]);
    let h1 = rows(&s);
    drag_m(&mut s, AROUND_AB.0, AROUND_AB.1, |e| e.shift());
    assert_eq!(sel(&s), vec![b], "5h: Shift toggles");
    // 5e: A, B, C selected, Shift-marquee around A, B: only C
    select(&mut s, &[a, b, c]);
    drag_m(&mut s, AROUND_AB.0, AROUND_AB.1, |e| e.shift());
    assert_eq!(sel(&s), vec![c], "5e");
    // 5d: A, B selected, Shift-marquee around C: A, B, C
    select(&mut s, &[a, b]);
    drag_m(&mut s, AROUND_C.0, AROUND_C.1, |e| e.shift());
    assert_eq!(sel(&s), sorted(vec![a, b, c]), "5d");
    // 5f: C selected, Shift+Alt-marquee around A: C, A (= Shift)
    select(&mut s, &[c]);
    drag_m(&mut s, AROUND_A.0, AROUND_A.1, |e| e.shift().alt());
    assert_eq!(sel(&s), sorted(vec![a, c]), "5f");
    // 5g: Ctrl-marquee = normal
    ex(&mut s, "select.set", json!({"ids": [], "mode": "replace"}));
    drag_m(&mut s, AROUND_AB.0, AROUND_AB.1, |e| e.ctrl());
    assert_eq!(sel(&s), sorted(vec![a, b]), "5g");
    // 5a: Alt-marquee around A, B: A, B selected, no clone
    ex(&mut s, "select.set", json!({"ids": [], "mode": "replace"}));
    let n = top_elements(&s).len();
    drag_m(&mut s, AROUND_AB.0, AROUND_AB.1, |e| e.alt());
    assert_eq!(sel(&s), sorted(vec![a, b]), "5a");
    assert_eq!(top_elements(&s).len(), n, "5a: no clone");
    // 5b / 5i: Alt-marquee around A replaces
    select(&mut s, &[a, b, c]);
    drag_m(&mut s, AROUND_A.0, AROUND_A.1, |e| e.alt());
    assert_eq!(sel(&s), vec![a], "5i");
    // marquee selections write no row (select() above writes its own rows through select.set)
    let _ = h1;
    assert!(rows(&s) >= h0);
    let before = rows(&s);
    drag_m(&mut s, AROUND_AB.0, AROUND_AB.1, |e| e.shift());
    drag_m(&mut s, AROUND_AB.0, AROUND_AB.1, |e| e.alt());
    drag_m(&mut s, AROUND_AB.0, AROUND_AB.1, plain);
    assert_eq!(rows(&s), before, "marquee selections write no History row");
    // 5j: Alt-drag on A clones (unchanged)
    select(&mut s, &[a]);
    let n = top_elements(&s).len();
    drag_m(&mut s, (150.0, 120.0), (180.0, 150.0), |e| e.alt());
    assert_eq!(top_elements(&s).len(), n + 1, "5j: Alt-drag clones");
}

/// RE spec 69 section 5.2 `5m`–`5p`: in Group(A, B) the plain marquee selects the group, the
/// Alt-marquee (and the Subselection's marquee) the child A alone.
#[test]
fn fb33_alt_marquee_selects_like_the_subselection() {
    let (mut s, a, b, _c) = s5();
    select(&mut s, &[a, b]);
    let v = ex(&mut s, "modify.group", json!({}));
    let g = ElementId(v["id"].as_u64().or_else(|| v["ids"][0].as_u64()).unwrap_or(0));
    ex(&mut s, "select.set", json!({"ids": [], "mode": "replace"}));
    tool(&mut s, ToolId::Pointer);
    drag_m(&mut s, AROUND_A.0, AROUND_A.1, plain);
    assert_eq!(sel(&s), vec![g], "5m: the group");
    ex(&mut s, "select.set", json!({"ids": [], "mode": "replace"}));
    drag_m(&mut s, AROUND_A.0, AROUND_A.1, |e| e.alt());
    assert_eq!(sel(&s), vec![a], "5n: A alone");
    ex(&mut s, "select.set", json!({"ids": [], "mode": "replace"}));
    drag_m(&mut s, AROUND_AB.0, AROUND_AB.1, plain);
    assert_eq!(sel(&s), vec![g], "5o");
    tool(&mut s, ToolId::Subselection);
    ex(&mut s, "select.set", json!({"ids": [], "mode": "replace"}));
    drag_m(&mut s, AROUND_A.0, AROUND_A.1, plain);
    assert_eq!(sel(&s), vec![a], "5p: Subselection, A");
}

/// RE spec 69 section 5.2 `5q`, `5r`: the Subselection's Shift-marquee adds (no toggle), its
/// Alt-marquee is a normal one; no History row.
#[test]
fn fb33_subselection_shift_marquee_adds() {
    let (mut s, a, b, _c) = s5();
    tool(&mut s, ToolId::Subselection);
    drag_m(&mut s, AROUND_A.0, AROUND_A.1, plain);
    assert_eq!(sel(&s), vec![a]);
    let h = rows(&s);
    drag_m(&mut s, AROUND_AB.0, AROUND_AB.1, |e| e.shift());
    assert_eq!(sel(&s), sorted(vec![a, b]), "5q: A stays, B added");
    drag_m(&mut s, AROUND_AB.0, AROUND_AB.1, |e| e.shift());
    assert_eq!(sel(&s), sorted(vec![a, b]), "a second Shift-marquee does not toggle");
    ex(&mut s, "select.set", json!({"ids": [], "mode": "replace"}));
    let h2 = rows(&s);
    drag_m(&mut s, AROUND_A.0, AROUND_A.1, |e| e.alt());
    assert_eq!(sel(&s), vec![a], "5r: Alt-marquee normal");
    assert_eq!(rows(&s), h2, "no History row");
    let _ = h;
}

/// Firepixel extension: with `Firepixel_PointerLasso` the drag draws a free-form outline; it
/// selects the objects (Pointer) whose centre it encloses, with the same modifiers; the
/// Subselection selects the anchors inside.
#[test]
fn lasso_marquee_option_for_pointer_and_subselection() {
    let (mut s, a, b, c) = s5();
    assert!(!s.prefs.get_bool(fp_engine::tools::POINTER_LASSO_PREF, false), "default: rectangle");
    let v = ex(&mut s, "tool.pointerLasso", json!({}));
    assert_eq!(v["on"], json!(true));
    assert!(s.prefs.get_bool(fp_engine::tools::POINTER_LASSO_PREF, false));
    tool(&mut s, ToolId::Pointer);
    // an L-shaped outline around A and C, leaving out B
    let outline = [(80.0, 60.0), (220.0, 60.0), (220.0, 180.0), (520.0, 180.0), (520.0, 320.0), (80.0, 320.0)];
    let lasso = |s: &mut Session, m: fn(ToolEvent) -> ToolEvent| {
        let mut evs = vec![m(ToolEvent::down(70.0, 50.0))];
        for (x, y) in outline {
            evs.push(m(ToolEvent::drag(x, y)));
        }
        evs.push(m(ToolEvent::up(70.0, 330.0)));
        let mut last = Vec::new();
        for e in &evs {
            last = s.tool_event(e).map(|r| r.overlay).unwrap_or_else(|e| panic!("{e}"));
        }
        last
    };
    let h = rows(&s);
    lasso(&mut s, plain);
    assert_eq!(sel(&s), sorted(vec![a, c]), "the outline selects A and C, not B");
    assert_eq!(rows(&s), h, "no History row");
    // the overlay while dragging is the free-form outline
    let mut evs = vec![ToolEvent::down(70.0, 50.0)];
    evs.extend(outline.iter().map(|(x, y)| ToolEvent::drag(*x, *y)));
    let mut ov = Vec::new();
    for e in &evs {
        ov = s.tool_event(e).map(|r| r.overlay).unwrap_or_else(|e| panic!("{e}"));
    }
    assert!(
        matches!(ov.first(), Some(fp_engine::tools::Overlay::Polyline { points, closed: true }) if points.len() >= 6),
        "{ov:?}"
    );
    events(&mut s, &[ToolEvent::up(70.0, 330.0)]);
    // Shift toggles like the rectangle: A, C selected -> both off
    lasso(&mut s, |e| e.shift());
    assert!(sel(&s).is_empty(), "Shift-lasso toggles: {:?}", sel(&s));
    let _ = b;
    // Subselection: the anchors inside the outline
    tool(&mut s, ToolId::Pen);
    events(&mut s, &[ToolEvent::down(100.0, 350.0), ToolEvent::up(100.0, 350.0), ToolEvent::down(300.0, 350.0), ToolEvent::up(300.0, 350.0), ToolEvent::down(500.0, 350.0), ToolEvent::up(500.0, 350.0), ToolEvent::key(fp_engine::tools::ToolKey::Enter)]);
    tool(&mut s, ToolId::Subselection);
    ex(&mut s, "select.set", json!({"ids": [], "mode": "replace"}));
    let pts = [(90.0, 340.0), (310.0, 340.0), (310.0, 360.0), (90.0, 360.0)];
    let mut evs = vec![ToolEvent::down(80.0, 335.0)];
    evs.extend(pts.iter().map(|(x, y)| ToolEvent::drag(*x, *y)));
    evs.push(ToolEvent::up(80.0, 365.0));
    events(&mut s, &evs);
    let mut nodes: Vec<usize> = s.active().map(|d| d.selection.points.iter().map(|p| p.node).collect()).unwrap_or_default();
    nodes.sort();
    assert_eq!(nodes, vec![0, 1], "two of three anchors inside the outline");
    // switch back: rectangle
    ex(&mut s, "tool.pointerLasso", json!({"on": false}));
    assert!(!s.prefs.get_bool(fp_engine::tools::POINTER_LASSO_PREF, false));
}
