//! FB-80 (Firepixel extension, TW-266): the Subselection reaches the **content** of a masked
//! object (also a single masked path or bitmap), the new Mask Element Selection (Subselection
//! flyout, keys A / 1) its **mask** (the measured Subselection target, RE spec 68 §3.2). The
//! Pointer is unchanged (click = masked object, Alt-click = mask).

use crate::common;
use common::*;
use fp_engine::shortcuts::Shortcut;
use fp_engine::tools::ToolId;
use fp_engine::{Session, ToolEvent};
use fp_model::ElementId;
use serde_json::json;

fn sel(s: &Session) -> Vec<ElementId> {
    s.active()
        .map(|d| d.selection.elements.clone())
        .unwrap_or_default()
}

fn tool(s: &mut Session, t: ToolId) {
    s.set_tool(t).unwrap_or_else(|e| panic!("{e}"));
}

fn click(s: &mut Session, x: f32, y: f32) {
    events(s, &[ToolEvent::down(x, y), ToolEvent::up(x, y)]);
}

fn drag(s: &mut Session, a: (f32, f32), b: (f32, f32)) {
    events(
        s,
        &[
            ToolEvent::down(a.0, a.1),
            ToolEvent::drag((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0),
            ToolEvent::drag(b.0, b.1),
            ToolEvent::up(b.0, b.1),
        ],
    );
}

fn quad(s: &mut Session, l: f32, t: f32, r: f32, b: f32) -> ElementId {
    let v = ex(
        s,
        "draw.path",
        json!({"contours": [{"closed": true, "nodes": [[l, t], [r, t], [r, b], [l, b]]}]}),
    );
    ElementId(v["id"].as_u64().unwrap_or(0))
}

/// A single path 100..200 masked by the path square 120..180: (session, masked object, mask,
/// content).
fn path_scene() -> (Session, ElementId, ElementId, ElementId) {
    let mut s = doc_session(400.0, 300.0);
    let content = quad(&mut s, 100.0, 100.0, 200.0, 200.0);
    ex(
        &mut s,
        "element.setFill",
        json!({"ids": [content.0], "color": "#ff0000"}),
    );
    let mask = quad(&mut s, 120.0, 120.0, 180.0, 180.0);
    select(&mut s, &[mask, content]);
    let g = ElementId(
        ex(&mut s, "modify.mask.groupAsMask", json!({}))["id"]
            .as_u64()
            .unwrap_or(0),
    );
    ex(
        &mut s,
        "modify.mask.options",
        json!({"mode": "path", "showAttrs": false}),
    );
    ex(&mut s, "select.none", json!({}));
    (s, g, mask, content)
}

#[test]
fn subselection_click_on_a_single_masked_path_selects_the_content() {
    let (mut s, g, mask, content) = path_scene();
    assert!(
        el(&s, g).masked_single().is_some(),
        "a single masked element"
    );
    tool(&mut s, ToolId::Subselection);
    click(&mut s, 150.0, 150.0);
    assert_eq!(sel(&s), vec![content]);
    assert_ne!(sel(&s), vec![mask]);
}

#[test]
fn moving_the_subselected_content_leaves_the_mask() {
    fp_i18n::set_language("en");
    let (mut s, g, mask, content) = path_scene();
    let mask_before = visible_bounds(&s, mask);
    tool(&mut s, ToolId::Subselection);
    click(&mut s, 150.0, 150.0);
    assert_eq!(sel(&s), vec![content]);
    drag(&mut s, (150.0, 150.0), (170.0, 160.0));
    let b = visible_bounds(&s, content);
    assert!(
        approx(b.left, 120.0) && approx(b.top, 110.0),
        "content moved: {b:?}"
    );
    assert!(
        rect_approx(visible_bounds(&s, mask), mask_before),
        "the mask stays"
    );
    assert!(
        el(&s, g).masked_single().is_some(),
        "still one masked object"
    );
    assert_eq!(sel(&s), vec![content], "the content stays selected");
    assert_eq!(s.undo_redo_labels().0.as_deref(), Some("Move"));
    s.undo().expect("undo");
    let b = visible_bounds(&s, content);
    assert!(approx(b.left, 100.0) && approx(b.top, 100.0), "undo: {b:?}");
}

#[test]
fn subselection_drags_an_anchor_of_the_content() {
    let (mut s, _, mask, content) = path_scene();
    let mask_before = visible_bounds(&s, mask);
    tool(&mut s, ToolId::Subselection);
    click(&mut s, 150.0, 150.0);
    drag(&mut s, (100.0, 100.0), (90.0, 95.0));
    let b = visible_bounds(&s, content);
    assert!(
        approx(b.left, 90.0) && approx(b.top, 95.0),
        "anchor moved: {b:?}"
    );
    assert!(
        approx(b.right, 200.0) && approx(b.bottom, 200.0),
        "only one anchor: {b:?}"
    );
    assert!(
        rect_approx(visible_bounds(&s, mask), mask_before),
        "the mask stays"
    );
}

#[test]
fn subselection_selects_masked_bitmap_content_without_bitmap_editing() {
    let mut s = doc_session(400.0, 300.0);
    let bmp = bitmap(&mut s, 100, 100, 100, 100, [255, 0, 0, 255]);
    let mask = quad(&mut s, 120.0, 120.0, 180.0, 180.0);
    select(&mut s, &[mask, bmp]);
    ex(&mut s, "modify.mask.groupAsMask", json!({}));
    ex(&mut s, "select.none", json!({}));
    tool(&mut s, ToolId::Subselection);
    click(&mut s, 150.0, 150.0);
    assert_eq!(sel(&s), vec![bmp]);
    assert!(!s.active().is_some_and(|d| d.bitmap_mode));
}

#[test]
fn mask_element_selection_selects_the_mask_and_edits_its_anchors() {
    let (mut s, _, mask, content) = path_scene();
    let content_before = visible_bounds(&s, content);
    tool(&mut s, ToolId::MaskElementSelect);
    click(&mut s, 150.0, 150.0);
    assert_eq!(sel(&s), vec![mask]);
    drag(&mut s, (120.0, 120.0), (110.0, 110.0));
    let b = visible_bounds(&s, mask);
    assert!(
        approx(b.left, 110.0) && approx(b.top, 110.0),
        "mask anchor moved: {b:?}"
    );
    assert!(
        rect_approx(visible_bounds(&s, content), content_before),
        "content stays"
    );
}

#[test]
fn mask_element_selection_on_ordinary_objects_works_like_the_subselection() {
    let mut s = doc_session(400.0, 300.0);
    let a = rect(&mut s, 20.0, 20.0, 60.0, 60.0);
    let b = rect(&mut s, 100.0, 20.0, 140.0, 60.0);
    select(&mut s, &[a, b]);
    ex(&mut s, "modify.group", json!({}));
    ex(&mut s, "select.none", json!({}));
    tool(&mut s, ToolId::MaskElementSelect);
    click(&mut s, 120.0, 40.0);
    assert_eq!(sel(&s), vec![b]);
}

#[test]
fn pointer_is_unchanged() {
    let (mut s, g, mask, _) = path_scene();
    tool(&mut s, ToolId::Pointer);
    click(&mut s, 150.0, 150.0);
    assert_eq!(sel(&s), vec![g]);
    events(
        &mut s,
        &[
            ToolEvent::down(150.0, 150.0).alt(),
            ToolEvent::up(150.0, 150.0).alt(),
        ],
    );
    assert_eq!(sel(&s), vec![mask]);
}

#[test]
fn point_at_command_reaches_content_or_mask() {
    let (mut s, _, mask, content) = path_scene();
    ex(
        &mut s,
        "select.pointAt",
        json!({"point": [150, 150], "deep": true}),
    );
    assert_eq!(sel(&s), vec![content]);
    ex(
        &mut s,
        "select.pointAt",
        json!({"point": [150, 150], "deep": true, "mask": true}),
    );
    assert_eq!(sel(&s), vec![mask]);
}

#[test]
fn keys_a_and_1_cycle_subselection_and_mask_element_selection() {
    let mut s = doc_session(100.0, 100.0);
    ex(
        &mut s,
        "edit.keyboardShortcuts",
        json!({"set": "Firepixel"}),
    );
    let info = fp_engine::tools::info(ToolId::MaskElementSelect).expect("in the table");
    assert_eq!(info.flyout, "Teilauswahl");
    assert_eq!(
        fp_engine::tools::flyout_members("Teilauswahl"),
        vec![ToolId::Subselection, ToolId::MaskElementSelect]
    );
    for key in *b"A1" {
        tool(&mut s, ToolId::Rectangle);
        let sc = Shortcut::key(key as u16);
        assert_eq!(
            s.tool_shortcut(&sc).ok().flatten(),
            Some(ToolId::Subselection)
        );
        assert_eq!(
            s.tool_shortcut(&sc).ok().flatten(),
            Some(ToolId::MaskElementSelect)
        );
        assert_eq!(
            s.tool_shortcut(&sc).ok().flatten(),
            Some(ToolId::Subselection)
        );
    }
    let set = s.shortcut_set().clone();
    assert_eq!(
        fp_engine::tools::tool_shortcuts(&set, ToolId::MaskElementSelect),
        fp_engine::tools::tool_shortcuts(&set, ToolId::Subselection)
    );
}
