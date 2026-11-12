//! RE spec 81 §3.2 (D6, FB-146): the measured Subselection target, which Firepixel gives the Mask
//! Element Selection, Pointer + Alt-click and `select.pointAt {deep, mask}`, depends on the
//! underlay of the mask ("Show fill and stroke" in path outline / alpha channel mode):
//! underlay off (or grayscale, or the Firepixel mode "Pixel Alpha") → the mask object; underlay on
//! → a child of a group mask with own pixels, a content child of a mask group, the masked single
//! element itself, or (only the underlay hit) the mask. A further click on the selected content
//! child selects the mask group (rule 3). The Firepixel Subselection keeps reaching the content
//! (FB-80). Also the mask mode "Pixel Alpha" (FB-151) of `modify.mask.options`.

use crate::common;
use common::*;
use fp_engine::tools::ToolId;
use fp_engine::{Session, ToolEvent};
use fp_model::{ElementId, ElementKind, GroupKind};
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

fn alt_click(s: &mut Session, x: f32, y: f32) {
    let (mut d, mut u) = (ToolEvent::down(x, y), ToolEvent::up(x, y));
    d.mods.alt = true;
    u.mods.alt = true;
    events(s, &[d, u]);
}

fn id_of(v: &serde_json::Value) -> ElementId {
    ElementId(v["id"].as_u64().or_else(|| v["ids"][0].as_u64()).unwrap_or(0))
}

fn filled(s: &mut Session, r: [i32; 4], color: &str) -> ElementId {
    id_of(&ex(
        s,
        "draw.rectangle",
        json!({"rect": r, "fillColor": color, "strokeWidth": 0}),
    ))
}

/// The scene of RE spec 81: mask M 10..290 × 10..190 (white), contents C 20..200 × 40..160 and C2
/// 120..280 × 60..140 (on top) pasted inside M (underlay on), or only C (`single`). Returns
/// (session, mask group, mask, C, C2 or C again).
fn scene(single: bool) -> (Session, ElementId, ElementId, ElementId, ElementId) {
    let mut s = doc_session(300.0, 200.0);
    let m = filled(&mut s, [10, 10, 290, 190], "#ffffff");
    let c = filled(&mut s, [20, 40, 200, 160], "#0080ff");
    let mut cut = vec![c];
    if !single {
        cut.push(filled(&mut s, [120, 60, 280, 140], "#00c000"));
    }
    select(&mut s, &cut);
    ex(&mut s, "edit.cut", json!({}));
    select(&mut s, &[m]);
    let g = id_of(&ex(&mut s, "edit.pasteInside", json!({})));
    ex(&mut s, "select.none", json!({}));
    let ge = el(&s, g);
    let ElementKind::Group(gr) = &ge.kind else {
        panic!("mask group");
    };
    assert_eq!(gr.kind, GroupKind::VectorMask);
    assert!(gr.mask_underlay(), "Paste Inside: underlay on");
    let mask = gr.mask.as_ref().expect("mask").id;
    let by_left = |l: f32| {
        gr.children
            .iter()
            .find(|k| (bounds(&s, k.id).left - l).abs() < 0.6)
            .map(|k| k.id)
            .expect("content")
    };
    let c = by_left(20.0);
    let c2 = if single { c } else { by_left(120.0) };
    (s, g, mask, c, c2)
}

#[test]
fn underlay_on_reaches_the_content_children_and_the_mask_on_the_underlay() {
    let (mut s, g, mask, c, c2) = scene(false);
    tool(&mut s, ToolId::MaskElementSelect);
    click(&mut s, 60.0, 100.0);
    assert_eq!(sel(&s), vec![c], "P_C: the content child C");
    ex(&mut s, "select.none", json!({}));
    click(&mut s, 160.0, 100.0);
    assert_eq!(sel(&s), vec![c2], "P_O: the upper content child C2");
    // rule 3: a further click on the selected child selects the mask group
    click(&mut s, 160.0, 100.0);
    assert_eq!(sel(&s), vec![g], "second click: the mask group");
    ex(&mut s, "select.none", json!({}));
    click(&mut s, 150.0, 25.0);
    assert_eq!(sel(&s), vec![mask], "P_M (only the underlay): the mask");
    // Pointer + Alt-click acts like the measured Subselection (rule 4)
    ex(&mut s, "select.none", json!({}));
    tool(&mut s, ToolId::Pointer);
    alt_click(&mut s, 60.0, 100.0);
    assert_eq!(sel(&s), vec![c], "Alt-click: the content child");
    // the script / agent route
    ex(
        &mut s,
        "select.pointAt",
        json!({"point": [240.0, 100.0], "deep": true, "mask": true}),
    );
    assert_eq!(sel(&s), vec![c2], "pointAt mask: C2 at P_C2");
}

#[test]
fn underlay_off_grayscale_and_pixel_alpha_give_the_mask() {
    for opts in [
        json!({"showAttrs": false}),
        json!({"mode": "image"}),
        json!({"mode": "pixelAlpha"}),
    ] {
        let (mut s, g, mask, _, _) = scene(false);
        let mut p = opts.clone();
        p["ids"] = json!([g.0]);
        ex(&mut s, "modify.mask.options", p);
        ex(&mut s, "select.none", json!({}));
        tool(&mut s, ToolId::MaskElementSelect);
        click(&mut s, 60.0, 100.0);
        assert_eq!(sel(&s), vec![mask], "{opts}: P_C gives the mask");
        ex(&mut s, "select.none", json!({}));
        click(&mut s, 160.0, 100.0);
        assert_eq!(sel(&s), vec![mask], "{opts}: P_O gives the mask");
    }
}

#[test]
fn single_content_gives_the_masked_element_and_the_subselection_stays_content() {
    let (mut s, g, mask, c, _) = scene(true);
    assert!(el(&s, g).masked_single().is_some());
    tool(&mut s, ToolId::MaskElementSelect);
    click(&mut s, 60.0, 100.0);
    assert_eq!(sel(&s), vec![g], "pin1: the masked element itself");
    ex(&mut s, "select.none", json!({}));
    click(&mut s, 250.0, 25.0);
    assert_eq!(sel(&s), vec![mask], "the underlay: the mask");
    // the Pointer hits the visible underlay outside the content too: the masked object
    ex(&mut s, "select.none", json!({}));
    tool(&mut s, ToolId::Pointer);
    click(&mut s, 250.0, 25.0);
    assert_eq!(sel(&s), vec![g], "Pointer on the underlay: the masked element");
    // the Firepixel Subselection keeps reaching the content (FB-80)
    ex(&mut s, "select.none", json!({}));
    tool(&mut s, ToolId::Subselection);
    click(&mut s, 60.0, 100.0);
    assert_eq!(sel(&s), vec![c]);
}

#[test]
fn group_mask_child_with_own_pixels_wins_over_the_content() {
    // content Y 30..180 × 30..170, the mask a group of P1 40..120 × 40..120 and P2 160..240 ×
    // 60..140 pasted around Y (Paste Inside into the group: underlay on)
    let mut s = doc_session(300.0, 200.0);
    let p1 = filled(&mut s, [40, 40, 120, 120], "#ff0000");
    let p2 = filled(&mut s, [160, 60, 240, 140], "#00ff00");
    select(&mut s, &[p1, p2]);
    let gm = id_of(&ex(&mut s, "modify.group", json!({})));
    let y = filled(&mut s, [30, 30, 180, 170], "#ffd000");
    select(&mut s, &[y]);
    ex(&mut s, "edit.cut", json!({}));
    select(&mut s, &[gm]);
    let owner = id_of(&ex(&mut s, "edit.pasteInside", json!({})));
    ex(&mut s, "select.none", json!({}));
    let oe = el(&s, owner);
    let ElementKind::Group(og) = &oe.kind else {
        panic!("mask group");
    };
    assert!(og.mask_underlay());
    tool(&mut s, ToolId::MaskElementSelect);
    click(&mut s, 60.0, 80.0);
    assert_eq!(sel(&s), vec![p1], "gmb_pi: the child of the group mask");
}

#[test]
fn pixel_alpha_mode_switches_back_and_undoes() {
    let (mut s, g, _, _, _) = scene(false);
    let kind = |s: &Session| match &el(s, g).kind {
        ElementKind::Group(gr) => (gr.kind, gr.mask_flags.pixel_alpha, gr.mask_underlay()),
        _ => panic!("group"),
    };
    ex(
        &mut s,
        "modify.mask.options",
        json!({"ids": [g.0], "mode": "pixelAlpha"}),
    );
    assert_eq!(kind(&s), (GroupKind::VectorMask, true, false));
    assert!(el(&s, g).element_mask().is_some_and(|m| m.pixel_alpha && m.to_path));
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(kind(&s), (GroupKind::VectorMask, false, true), "undo");
    ex(&mut s, "edit.redo", json!({}));
    assert_eq!(kind(&s), (GroupKind::VectorMask, true, false), "redo");
    ex(&mut s, "modify.mask.options", json!({"ids": [g.0], "mode": "image"}));
    assert_eq!(kind(&s), (GroupKind::BitmapMask, false, false), "grayscale clears it");
    ex(&mut s, "modify.mask.options", json!({"ids": [g.0], "mode": "path"}));
    assert_eq!(kind(&s), (GroupKind::VectorMask, false, true), "path outline, underlay back");
}

