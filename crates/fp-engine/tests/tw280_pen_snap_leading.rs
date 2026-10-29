//! TW-280 (RE spec 77): Pen anchor hits as a square of `max(SnapDistance, PickDistance)/zoom`
//! (close, join, continue, also on unselected paths); Enter / Escape keep drawing, double-click
//! and closing end it with the measured selection states; Pen points snap to ruler guides and the
//! grid but not to smart guides or the page edge; Subselection anchor drags snap to guides and
//! grid; Pointer moves snap to the bounding boxes of other objects (not to anchors inside a box)
//! with the measured guide reference values; leading px <-> % with the size and result clamps and
//! one "Text change" row per unit switch.

use crate::common::*;
use fp_engine::prefs::PrefValue;
use fp_engine::tools::ToolId;
use fp_engine::{Overlay, Session, ToolEvent, ToolKey};
use fp_model::{Contour, ElementId, ElementKind};
use serde_json::json;

fn session() -> Session {
    fp_i18n::set_language("de");
    let mut s = doc_session(400.0, 300.0);
    s.set_tool(ToolId::Pen).unwrap_or_else(|e| panic!("{e}"));
    s
}

fn click(s: &mut Session, x: f32, y: f32) {
    events(s, &[ToolEvent::down(x, y), ToolEvent::up(x, y)]);
}

fn undo_count(s: &Session) -> usize {
    s.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

fn paths(s: &Session) -> Vec<(ElementId, Vec<Contour>)> {
    top_elements(s)
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Path(p) if p.primitive.is_none() => Some((e.id, p.contours.clone())),
            _ => None,
        })
        .collect()
}

fn anchors(c: &Contour) -> Vec<(f32, f32)> {
    c.nodes.iter().map(|n| (n.anchor.x, n.anchor.y)).collect()
}

fn selection(s: &Session) -> Vec<ElementId> {
    s.active().map(|d| d.selection.elements.clone()).unwrap_or_default()
}

fn zoom(s: &mut Session, z: f64) {
    ex(s, "view.setZoom", json!({"zoom": z}));
}

/// Selected anchors of the first `PathPoints` overlay the Pen shows.
fn selected_points(s: &mut Session) -> Vec<(usize, usize)> {
    s.tool_feedback()
        .iter()
        .find_map(|o| match o {
            Overlay::PathPoints { selected, .. } => Some(selected.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

fn smart_guides(fb: &[Overlay]) -> Vec<(f32, f32, f32, f32)> {
    fb.iter()
        .filter_map(|o| match o {
            Overlay::SmartGuide { from, to } => Some((from.x, from.y, to.x, to.y)),
            _ => None,
        })
        .collect()
}

/// The measured three-point path 50/50, 150/60, 120/150 of `c1_close.json`.
fn three_points(s: &mut Session) {
    click(s, 50.0, 50.0);
    click(s, 150.0, 60.0);
    click(s, 120.0, 150.0);
}

// ------------------------------------------------------------------ anchor square

/// RE spec 77 section 2.1 at 100 %: (0,0), (5,0), (5,5), (-5,-5), (-5,4) beside the start point
/// close the path; (6,0) inserts on the segment, (6,6) and (0,-6) append a point.
#[test]
fn close_tolerance_is_a_square_at_zoom_1() {
    for (dx, dy) in [(0.0, 0.0), (5.0, 0.0), (5.0, 5.0), (-5.0, -5.0), (-5.0, 4.0)] {
        let mut s = session();
        three_points(&mut s);
        let n = undo_count(&s);
        click(&mut s, 50.0 + dx, 50.0 + dy);
        assert_eq!(undo_count(&s), n + 1, "({dx},{dy}): one row");
        let p = paths(&s);
        assert_eq!(p.len(), 1);
        assert!(p[0].1[0].closed, "({dx},{dy}) closes");
        assert_eq!(p[0].1[0].nodes.len(), 3, "({dx},{dy}): node count stays 3");
    }
    for (dx, dy, nodes) in [(6.0, 0.0, 4), (6.0, 6.0, 4), (0.0, -6.0, 4)] {
        let mut s = session();
        three_points(&mut s);
        click(&mut s, 50.0 + dx, 50.0 + dy);
        let p = paths(&s);
        assert_eq!(p.len(), 1);
        assert!(!p[0].1[0].closed, "({dx},{dy}) does not close");
        assert_eq!(p[0].1[0].nodes.len(), nodes, "({dx},{dy})");
    }
    // (6,0) is a segment hit: the point is inserted after the start point
    let mut s = session();
    three_points(&mut s);
    click(&mut s, 56.0, 50.0);
    let a = anchors(&paths(&s)[0].1[0]);
    assert_eq!(a[0], (50.0, 50.0));
    assert!((a[1].0 - 56.0).abs() < 0.5, "inserted on the first segment: {a:?}");
    assert_eq!(a[3], (120.0, 150.0), "the end point stays last");
}

/// At 200 % the tolerance is 5 screen px = 2.5 document px: 50/47.5 closes, 50/47 does not.
#[test]
fn close_tolerance_at_zoom_2() {
    let mut s = session();
    zoom(&mut s, 2.0);
    three_points(&mut s);
    click(&mut s, 50.0, 47.5);
    assert!(paths(&s)[0].1[0].closed, "5 screen px closes");
    let mut s = session();
    zoom(&mut s, 2.0);
    three_points(&mut s);
    click(&mut s, 50.0, 47.0);
    assert!(!paths(&s)[0].1[0].closed, "6 screen px does not close");
}

/// The tolerance is `max(SnapDistance, PickDistance)`: a larger PickDistance widens it.
#[test]
fn close_tolerance_uses_the_larger_of_snap_and_pick_distance() {
    let mut s = session();
    s.prefs.set("PickDistance", PrefValue::Int(8));
    three_points(&mut s);
    click(&mut s, 58.0, 42.0);
    assert!(paths(&s)[0].1[0].closed, "8 px with PickDistance 8");
    let mut s = session();
    s.prefs.set("SnapDistance", PrefValue::Int(7));
    s.prefs.set("PickDistance", PrefValue::Int(3));
    three_points(&mut s);
    click(&mut s, 57.0, 57.0);
    assert!(paths(&s)[0].1[0].closed, "7 px with SnapDistance 7");
}

/// The open path Q (300,50)-(350,80)-(330,120) of `mkpen.jsf`, not selected.
fn path_q(s: &mut Session) -> ElementId {
    click(s, 300.0, 50.0);
    click(s, 350.0, 80.0);
    events(
        s,
        &[
            ToolEvent::down(330.0, 120.0),
            ToolEvent::up(330.0, 120.0),
            ToolEvent::double(330.0, 120.0),
        ],
    );
    let id = paths(s)[0].0;
    ex(s, "select.none", json!({}));
    id
}

/// Join with the end point of another, unselected open path within the square (the corner
/// (5,5) lies outside a circle of 5).
#[test]
fn join_uses_the_square_on_an_unselected_path() {
    let mut s = session();
    let _q = path_q(&mut s);
    click(&mut s, 250.0, 20.0);
    click(&mut s, 305.0, 55.0);
    let p = paths(&s);
    assert_eq!(p.len(), 1, "joined into one path: {p:?}");
    assert_eq!(
        anchors(&p[0].1[0]),
        [(250.0, 20.0), (300.0, 50.0), (350.0, 80.0), (330.0, 120.0)]
    );
}

/// Without a path in progress a click within the square of the start point of an unselected
/// open path continues it from there.
#[test]
fn continue_uses_the_square_on_an_unselected_path() {
    let mut s = session();
    let q = path_q(&mut s);
    click(&mut s, 303.0, 45.0);
    assert_eq!(selection(&s), [q], "the path is selected for continuing");
    click(&mut s, 280.0, 30.0);
    let p = paths(&s);
    assert_eq!(p.len(), 1, "no new path");
    let a = anchors(&p[0].1[0]);
    assert_eq!(a.len(), 4, "{a:?}");
    assert!(a.contains(&(280.0, 30.0)), "{a:?}");
}

// ------------------------------------------------------------------ ending the path

/// Escape / Enter: no row, path open and selected, the last anchor selected; the next click
/// appends to the same path.
#[test]
fn enter_and_escape_keep_the_last_anchor_selected() {
    for key in [ToolKey::Escape, ToolKey::Enter] {
        let mut s = session();
        three_points(&mut s);
        let n = undo_count(&s);
        events(&mut s, &[ToolEvent::key(key)]);
        assert_eq!(undo_count(&s), n, "{key:?}");
        assert_eq!(selected_points(&mut s), [(0, 2)], "{key:?}: last anchor selected");
        let p = paths(&s);
        assert!(!p[0].1[0].closed);
        assert_eq!(selection(&s), [p[0].0]);
        click(&mut s, 220.0, 117.0);
        assert_eq!(paths(&s).len(), 1, "{key:?}: appended");
        assert_eq!(paths(&s)[0].1[0].nodes.len(), 4);
    }
}

/// Double-click: one row for the point, the path stays selected without a selected anchor; the
/// next click starts a new path and selects only that.
#[test]
fn double_click_ends_with_no_anchor_selected() {
    let mut s = session();
    click(&mut s, 50.0, 50.0);
    click(&mut s, 150.0, 60.0);
    let n = undo_count(&s);
    events(
        &mut s,
        &[
            ToolEvent::down(120.0, 150.0),
            ToolEvent::up(120.0, 150.0),
            ToolEvent::double(120.0, 150.0),
        ],
    );
    assert_eq!(undo_count(&s), n + 1);
    let old = paths(&s)[0].0;
    assert_eq!(selection(&s), [old], "the path stays selected");
    assert!(selected_points(&mut s).is_empty(), "no anchor selected");
    click(&mut s, 220.0, 117.0);
    assert_eq!(paths(&s).len(), 2, "a new path");
    let sel = selection(&s);
    assert_eq!(sel.len(), 1);
    assert_ne!(sel[0], old, "the old path is deselected");
}

/// Closing on the start point: one row, the path closed, anchor 0 selected; the next click starts
/// a new path.
#[test]
fn closing_selects_anchor_zero_and_ends_the_path() {
    let mut s = session();
    three_points(&mut s);
    let n = undo_count(&s);
    click(&mut s, 50.0, 50.0);
    assert_eq!(undo_count(&s), n + 1);
    let p = paths(&s);
    assert!(p[0].1[0].closed);
    assert_eq!(selection(&s), [p[0].0]);
    assert_eq!(selected_points(&mut s), [(0, 0)], "anchor 0 selected");
    click(&mut s, 220.0, 117.0);
    assert_eq!(paths(&s).len(), 2, "a new path");
    assert_ne!(selection(&s), [p[0].0]);
}

// ------------------------------------------------------------------ Pen point snapping

fn set_view(s: &mut Session, guides: bool, grid: bool) {
    if let Some(d) = s.active_mut() {
        d.view.show_guides = true;
        d.view.snap_to_guides = guides;
        d.view.snap_to_grid = grid;
        d.view.snap_to_smart_guides = true;
        d.view.show_smart_guides = true;
    }
}

/// `mkpen.jsf`: rectangle R 100/100-200/160, guides x 250 and y 230.
fn pen_doc(grid: bool) -> Session {
    let mut s = session();
    let _r = rect(&mut s, 100.0, 100.0, 200.0, 160.0);
    ex(&mut s, "select.none", json!({}));
    ex(&mut s, "guide.add", json!({"orientation": "vertical", "position": 250}));
    ex(&mut s, "guide.add", json!({"orientation": "horizontal", "position": 230}));
    set_view(&mut s, true, grid);
    if grid {
        ex(&mut s, "view.grid.editGrid", json!({"width": 36, "height": 36, "origin": [0, 0]}));
    }
    s.set_tool(ToolId::Pen).unwrap_or_else(|e| panic!("{e}"));
    s
}

/// Hovers at `(x, y)` (returns the overlay), clicks there and returns the point placed.
fn pen_point(s: &mut Session, x: f32, y: f32) -> ((f32, f32), Vec<Overlay>) {
    let fb = s.tool_event(&ToolEvent::drag(x, y)).expect("hover").overlay;
    click(s, x, y);
    let p = paths(s);
    let a = anchors(&p[0].1[0]);
    (a[0], fb)
}

/// RE spec 77 section 2.2: Pen points snap to ruler guides (<= 5 px) but not to edges, centres
/// or anchors of other objects (no smart guide line either), not to the canvas centre and not
/// to the page edge.
#[test]
fn pen_point_snaps_to_guides_not_to_smart_guides() {
    for ((x, y), want) in [
        ((204.0, 200.0), (204.0, 200.0)),
        ((203.0, 130.0), (203.0, 130.0)),
        ((152.0, 200.0), (152.0, 200.0)),
        ((60.0, 163.0), (60.0, 163.0)),
        ((203.0, 20.0), (203.0, 20.0)),
        ((253.0, 200.0), (250.0, 200.0)),
        ((255.0, 200.0), (250.0, 200.0)),
        ((256.0, 200.0), (256.0, 200.0)),
        ((60.0, 234.0), (60.0, 230.0)),
        ((403.0, 150.0), (403.0, 150.0)),
        ((60.0, 297.0), (60.0, 297.0)),
    ] {
        let mut s = pen_doc(false);
        let (got, fb) = pen_point(&mut s, x, y);
        assert_eq!(got, want, "click {x}/{y}");
        assert!(smart_guides(&fb).is_empty(), "no smart guide at {x}/{y}: {fb:?}");
    }
}

/// With the grid on (36 px): 254 -> 252, 257/219 -> 252/216, 258 stays.
#[test]
fn pen_point_snaps_to_the_grid() {
    for ((x, y), want) in [
        ((254.0, 200.0), (252.0, 200.0)),
        ((257.0, 219.0), (252.0, 216.0)),
        ((258.0, 200.0), (258.0, 200.0)),
    ] {
        let mut s = pen_doc(true);
        if let Some(d) = s.active_mut() {
            // no ruler guides near x 254..258 for this case
            d.view.snap_to_guides = false;
        }
        let (got, _) = pen_point(&mut s, x, y);
        assert_eq!(got, want, "click {x}/{y}");
    }
}

// ------------------------------------------------------------------ Subselection anchor drags

/// RE spec 77 section 3: a Subselection anchor drag snaps to ruler guides and the grid
/// (<= 5 px), without a smart guide line.
#[test]
fn subselection_anchor_snaps_to_guides_and_grid() {
    for (grid, to, want) in [
        (false, (253.0, 50.0), (250.0, 50.0)),
        (false, (256.0, 50.0), (256.0, 50.0)),
        (false, (300.0, 233.0), (300.0, 230.0)),
        (true, (254.0, 50.0), (252.0, 50.0)),
        (true, (257.0, 50.0), (252.0, 50.0)),
        (true, (258.0, 50.0), (258.0, 50.0)),
    ] {
        let mut s = pen_doc(grid);
        if grid && let Some(d) = s.active_mut() {
            d.view.snap_to_guides = false;
        }
        let q = path_q(&mut s);
        s.set_tool(ToolId::Subselection).expect("tool");
        click(&mut s, 325.0, 65.0);
        assert_eq!(selection(&s), [q]);
        let mut evs = vec![ToolEvent::down(300.0, 50.0)];
        for i in 1..=20 {
            let t = i as f32 / 20.0;
            evs.push(ToolEvent::drag(300.0 + (to.0 - 300.0) * t, 50.0 + (to.1 - 50.0) * t));
        }
        events(&mut s, &evs);
        let fb = s.tool_event(&ToolEvent::drag(to.0, to.1)).expect("drag").overlay;
        assert!(smart_guides(&fb).is_empty(), "{fb:?}");
        events(&mut s, &[ToolEvent::up(to.0, to.1)]);
        let p = paths(&s);
        let c = &p.iter().find(|(id, _)| *id == q).expect("Q").1[0];
        assert_eq!(c.nodes[0].anchor.x, want.0, "grid {grid}, to {to:?}");
        assert_eq!(c.nodes[0].anchor.y, want.1, "grid {grid}, to {to:?}");
    }
}

// ------------------------------------------------------------------ Pointer smart guides

/// `mkptr.jsf`: R 100/100-200/160 (a rectangle), Q the open path above (bounds 300/50-350/120,
/// an inner anchor at 330/120 and one at 350/80); R selected with the Pointer.
fn pointer_doc() -> (Session, ElementId) {
    let mut s = session();
    let _q = path_q(&mut s);
    let r = rect(&mut s, 100.0, 100.0, 200.0, 160.0);
    set_view(&mut s, false, false);
    s.set_tool(ToolId::Pointer).expect("tool");
    select(&mut s, &[r]);
    (s, r)
}

/// Drags R from 150/130 by `(dx, dy)` and returns the overlay with the mouse held and R's
/// bounds after the release.
fn move_r(s: &mut Session, r: ElementId, dx: f32, dy: f32) -> (Vec<Overlay>, fp_geom::Rect) {
    let mut evs = vec![ToolEvent::down(150.0, 130.0)];
    for i in 1..=10 {
        let t = i as f32 / 10.0;
        evs.push(ToolEvent::drag(150.0 + dx * t, 130.0 + dy * t));
    }
    events(s, &evs);
    let fb = s
        .tool_event(&ToolEvent::drag(150.0 + dx, 130.0 + dy))
        .expect("drag")
        .overlay;
    events(s, &[ToolEvent::up(150.0 + dx, 130.0 + dy)]);
    (fb, bounds(s, r))
}

/// RE spec 77 section 4: the right edge of R snaps to the left edge of Q at a gap of 3 and 5,
/// not 6; anchors inside Q's box (x 330, y 80) do not count; centre to centre snaps with the
/// line spanning the two centres; an edge line spans the tops and bottoms of both boxes.
#[test]
fn pointer_snaps_to_bounding_boxes_only() {
    let (mut s, r) = pointer_doc();
    let b0 = bounds(&s, r);
    let cases: [(f32, f32, f32, f32, bool); 6] = [
        (97.0, 0.0, 100.0, 0.0, true),
        (95.0, 0.0, 100.0, 0.0, true),
        (94.0, 0.0, 94.0, 0.0, false),
        (227.0, 0.0, 227.0, 0.0, false),
        (0.0, -23.0, 0.0, -23.0, false),
        (172.0, 0.0, 175.0, 0.0, true),
    ];
    for (dx, dy, mx, my, line) in cases {
        let (fb, b) = move_r(&mut s, r, dx, dy);
        assert_eq!(
            (b.left - b0.left, b.top - b0.top),
            (mx, my),
            "drag ({dx},{dy})"
        );
        let g = smart_guides(&fb);
        assert_eq!(!g.is_empty(), line, "drag ({dx},{dy}): {g:?}");
        if dx == 97.0 {
            assert!(
                g.contains(&(300.0, 50.0, 300.0, b0.bottom)),
                "edge line spans Q top to R bottom: {g:?}"
            );
        }
        if dx == 172.0 {
            let (qc, rc) = (85.0, (b0.top + b0.bottom) / 2.0);
            assert!(
                g.contains(&(325.0, qc, 325.0, rc)),
                "centre line spans the two centres: {g:?}"
            );
        }
        ex(&mut s, "edit.undo", json!({}));
        assert_eq!(bounds(&s, r), b0);
    }
}

// ------------------------------------------------------------------ leading px <-> %

/// RE spec 77 section 5.2/5.3: the measured chain 120 % -> 36 px -> 120 % -> 37 px -> 1.2333
/// (shown 123) -> 36.9 px; 125 % -> 37.5 px -> 1.2667 (37.5 shown 38).
#[test]
fn leading_conversion_chain() {
    use fp_engine::commands::{convert_leading, leading_field_value};
    let near = |a: f32, b: f64| (a as f64 - b).abs() < 1e-5;
    assert!(near(convert_leading(1.2, 30.0, false, true), 36.0));
    assert!(near(convert_leading(36.0, 30.0, true, false), 1.2));
    let f = convert_leading(37.0, 30.0, true, false);
    assert!(near(f, 37.0 / 30.0), "37 -> 1.2333");
    assert_eq!(leading_field_value(f, false), 123.0);
    assert!(near(convert_leading(f, 30.0, false, true), 36.9), "1.2333 -> 36.9");
    let px = convert_leading(1.25, 30.0, false, true);
    assert!(near(px, 37.5));
    assert_eq!(leading_field_value(px, true), 38.0);
    assert!(near(convert_leading(px, 30.0, true, false), 38.0 / 30.0), "37.5 -> 1.2667");
}

/// Size clamped to [4, 1000], px -> % clamped to [0, 100], % -> px clamped to [0, 10000]; the
/// typed field values clamp as well.
#[test]
fn leading_conversion_clamps() {
    use fp_engine::commands::{convert_leading, leading_from_field};
    let near = |a: f32, b: f64| (a as f64 - b).abs() < 1e-4;
    assert!(near(convert_leading(10.0, 2.0, true, false), 2.5), "size 2 counts as 4");
    assert!(near(convert_leading(500.0, 3030.0, true, false), 0.5), "size 3030 counts as 1000");
    assert!(near(convert_leading(1.0, 1.0, false, true), 4.0), "size 1 counts as 4");
    assert!(near(convert_leading(9000.0, 4.0, true, false), 100.0), "factor clamped to 100");
    assert!(near(convert_leading(100.0, 1000.0, false, true), 10000.0), "px clamped to 10000");
    assert!(near(convert_leading(-3.0, 30.0, true, false), 0.0), "no negative factor");
    assert!(near(convert_leading(-1.0, 30.0, false, true), 0.0), "no negative px");
    assert_eq!(leading_from_field(20000.0, true), 10000.0);
    assert_eq!(leading_from_field(-5.0, true), 0.0);
    assert_eq!(leading_from_field(123.0, false), 1.23);
    assert_eq!(leading_from_field(20000.0, false), 100.0);
}

/// With a text object selected every unit switch is one "Text change" row that sets the object
/// and the defaults `Text_Leading` / `Text_LeadingMode` at once.
#[test]
fn leading_unit_switch_is_one_text_change_row() {
    use fp_engine::commands::convert_leading;
    let mut s = session();
    let v = ex(
        &mut s,
        "draw.text",
        json!({"text": "Zeile eins", "anchor": [50, 80], "size": 30, "color": "#000000"}),
    );
    let id = ElementId(v["id"].as_u64().unwrap_or(0));
    s.set_tool(ToolId::Pointer).expect("tool");
    select(&mut s, &[id]);
    ex(&mut s, "text.applyAttributes", json!({"leading": 1.2, "leadingMode": 0}));
    let mut last = (1.2_f32, false);
    for to_px in [true, false, true, false] {
        let n = undo_count(&s);
        let w = convert_leading(last.0, 30.0, last.1, to_px);
        ex(
            &mut s,
            "text.applyAttributes",
            json!({"leading": w, "leadingMode": to_px as i64}),
        );
        assert_eq!(undo_count(&s), n + 1, "one row");
        let d = s.active().expect("doc");
        let label = s.step_label(d.history.steps()[d.history.undo_count() - 1]);
        assert_eq!(label, fp_i18n::t("msg-step-text-change"));
        let ElementKind::Text(t) = &el(&s, id).kind else {
            panic!("no text");
        };
        let a = t.resolved_runs()[0].0.clone();
        assert_eq!(a.leading_mode, Some(to_px as i32));
        assert!((a.leading.unwrap_or(-1.0) - w).abs() < 1e-5);
        assert!((s.prefs.get_real("Text_Leading", -1.0) - w as f64).abs() < 1e-5);
        assert_eq!(s.prefs.get_int("Text_LeadingMode", -1), to_px as i64);
        last = (w, to_px);
    }
}
