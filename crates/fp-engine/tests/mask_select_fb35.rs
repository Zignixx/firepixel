//! FB-35 / FB-36, RE spec 68 (rules 3.2, 7, 8): selecting a mask on the canvas, Disable / Enable
//! Mask, Delete Mask (Discard / Apply), Ungroup, and the Firepixel extensions "children of a group
//! mask editable" (E-1) and "inverted mask" (E-2).
//!
//! Scene: red bitmap 100,100 100 × 100 (content), masked by a square path 120..180 (or by a group
//! of two squares). The mask mode is set to path outline explicitly, so the tests do not depend on
//! the mode a command chooses.

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
    events(s, &[ToolEvent::down(x, y).alt(), ToolEvent::up(x, y).alt()]);
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

/// Plain closed path (not a rectangle primitive, so its anchors move one by one).
fn quad(s: &mut Session, l: f32, t: f32, r: f32, b: f32) -> ElementId {
    let v = ex(s, "draw.path", json!({"contours": [{"closed": true, "nodes": [[l, t], [r, t], [r, b], [l, b]]}]}));
    ElementId(v["id"].as_u64().unwrap_or(0))
}

/// The masked element and its mask (path square 120..180 over a red bitmap).
fn scene() -> (Session, ElementId, ElementId, ElementId) {
    let mut s = doc_session(400.0, 300.0);
    let bmp = bitmap(&mut s, 100, 100, 100, 100, [255, 0, 0, 255]);
    let mask = quad(&mut s, 120.0, 120.0, 180.0, 180.0);
    select(&mut s, &[mask, bmp]);
    let g = ElementId(ex(&mut s, "modify.mask.groupAsMask", json!({}))["id"].as_u64().unwrap_or(0));
    ex(&mut s, "modify.mask.options", json!({"mode": "path", "showAttrs": false}));
    ex(&mut s, "select.none", json!({}));
    (s, g, mask, bmp)
}

/// Group mask: two squares 120..145 and 155..180 (grouped) over the red bitmap.
fn group_scene() -> (Session, ElementId, ElementId, ElementId, ElementId) {
    let mut s = doc_session(400.0, 300.0);
    let _bmp = bitmap(&mut s, 100, 100, 100, 100, [255, 0, 0, 255]);
    let a = quad(&mut s, 120.0, 120.0, 145.0, 180.0);
    let b = quad(&mut s, 155.0, 120.0, 180.0, 180.0);
    select(&mut s, &[a, b]);
    ex(&mut s, "modify.group", json!({}));
    let mg = sel(&s)[0];
    let top = top_elements(&s);
    select(&mut s, &[top[0].id, top[1].id]);
    let g = ElementId(ex(&mut s, "modify.mask.groupAsMask", json!({}))["id"].as_u64().unwrap_or(0));
    ex(&mut s, "modify.mask.options", json!({"mode": "path", "showAttrs": false}));
    ex(&mut s, "select.none", json!({}));
    (s, g, mg, a, b)
}

fn page(s: &Session) -> fp_raster::RgbaImage {
    let d = s.active().expect("doc");
    s.renderer()
        .expect("renderer")
        .render_page(&d.doc, d.page, d.frame)
        .expect("render")
}

fn px(img: &fp_raster::RgbaImage, x: i32, y: i32) -> [u8; 4] {
    img.get(x, y).unwrap_or([0; 4])
}

// ------------------------------------------------------------------ M-7: selecting the mask

#[test]
fn subselection_click_on_masked_bitmap_selects_its_mask() {
    let (mut s, g, mask, _) = scene();
    tool(&mut s, ToolId::Subselection);
    click(&mut s, 150.0, 150.0);
    assert_eq!(sel(&s), vec![mask], "the Subselection selects the mask");
    // the Pointer keeps selecting the masked element
    tool(&mut s, ToolId::Pointer);
    click(&mut s, 150.0, 150.0);
    assert_eq!(sel(&s), vec![g]);
}

#[test]
fn pointer_alt_click_selects_the_mask() {
    let (mut s, _, mask, _) = scene();
    tool(&mut s, ToolId::Pointer);
    let before = top_elements(&s).len();
    alt_click(&mut s, 150.0, 150.0);
    assert_eq!(sel(&s), vec![mask]);
    assert_eq!(top_elements(&s).len(), before, "an Alt-click copies nothing");
}

#[test]
fn pointer_alt_click_on_a_group_selects_the_clicked_child_and_alt_drag_still_copies() {
    let mut s = doc_session(400.0, 300.0);
    let a = rect(&mut s, 20.0, 20.0, 60.0, 60.0);
    let b = rect(&mut s, 100.0, 20.0, 140.0, 60.0);
    select(&mut s, &[a, b]);
    ex(&mut s, "modify.group", json!({}));
    let g = sel(&s)[0];
    ex(&mut s, "select.none", json!({}));
    tool(&mut s, ToolId::Pointer);
    alt_click(&mut s, 120.0, 40.0);
    assert_eq!(sel(&s), vec![b], "measured j1: Alt-click on a group = the clicked child");
    ex(&mut s, "select.none", json!({}));
    click(&mut s, 120.0, 40.0);
    assert_eq!(sel(&s), vec![g], "a plain click keeps selecting the group");
    let n = top_elements(&s).len();
    events(
        &mut s,
        &[
            ToolEvent::down(120.0, 40.0).alt(),
            ToolEvent::drag(150.0, 80.0).alt(),
            ToolEvent::drag(180.0, 120.0).alt(),
            ToolEvent::up(180.0, 120.0).alt(),
        ],
    );
    assert_eq!(top_elements(&s).len(), n + 1, "Alt-drag duplicates");
}

#[test]
fn select_behind_never_picks_the_mask() {
    let (mut s, _, mask, _) = scene();
    // something below to cycle through
    let under = rect(&mut s, 50.0, 50.0, 250.0, 250.0);
    select(&mut s, &[under]);
    ex(&mut s, "modify.arrange.sendToBack", json!({}));
    ex(&mut s, "select.none", json!({}));
    tool(&mut s, ToolId::SelectBehind);
    for _ in 0..4 {
        click(&mut s, 150.0, 150.0);
        assert!(!sel(&s).contains(&mask), "Select Behind selected the mask");
    }
}

#[test]
fn single_path_mask_anchor_drag_moves_only_the_mask() {
    let (mut s, _, mask, bmp) = scene();
    tool(&mut s, ToolId::Subselection);
    click(&mut s, 150.0, 150.0);
    assert_eq!(sel(&s), vec![mask]);
    let content_before = visible_bounds(&s, bmp);
    drag(&mut s, (120.0, 120.0), (105.0, 105.0));
    let b = visible_bounds(&s, mask);
    assert!(approx(b.left, 105.0) && approx(b.top, 105.0), "mask anchor moved: {b:?}");
    assert!(approx(b.right, 180.0) && approx(b.bottom, 180.0), "only one anchor moved: {b:?}");
    assert!(rect_approx(visible_bounds(&s, bmp), content_before), "content stays");
}

#[test]
fn subselection_on_a_bitmap_mask_starts_editing_the_mask() {
    let mut s = doc_session(400.0, 300.0);
    let bmp = bitmap(&mut s, 100, 100, 100, 100, [255, 0, 0, 255]);
    select(&mut s, &[bmp]);
    ex(&mut s, "modify.mask.revealAll", json!({}));
    let g = sel(&s)[0];
    let mask = el(&s, g).element_mask().map(|m| m.element.id).expect("mask");
    let _ = s.execute("modify.exitBitmapMode", json!({}));
    ex(&mut s, "select.none", json!({}));
    tool(&mut s, ToolId::Subselection);
    click(&mut s, 150.0, 150.0);
    assert_eq!(sel(&s), vec![mask]);
    assert!(s.active().is_some_and(|d| d.bitmap_mode), "bitmap editing of the mask");
}

#[test]
fn normal_groups_keep_their_subselection() {
    let mut s = doc_session(400.0, 300.0);
    let a = rect(&mut s, 20.0, 20.0, 60.0, 60.0);
    let b = rect(&mut s, 100.0, 20.0, 140.0, 60.0);
    select(&mut s, &[a, b]);
    ex(&mut s, "modify.group", json!({}));
    ex(&mut s, "select.none", json!({}));
    tool(&mut s, ToolId::Subselection);
    click(&mut s, 40.0, 40.0);
    assert_eq!(sel(&s), vec![a]);
    click(&mut s, 120.0, 40.0);
    assert_eq!(sel(&s), vec![b]);
}

// ------------------------------------------------------------------ E-1: group mask children

#[test]
fn group_mask_children_are_selectable_and_editable() {
    let (mut s, _, mg, a, b) = group_scene();
    tool(&mut s, ToolId::Subselection);
    click(&mut s, 130.0, 150.0);
    assert_eq!(sel(&s), vec![mg], "first click: the whole group mask (as measured)");
    // extension: a further click reaches the child under the pointer
    click(&mut s, 170.0, 150.0);
    assert_eq!(sel(&s), vec![b]);
    click(&mut s, 130.0, 150.0);
    assert_eq!(sel(&s), vec![a]);
    // drag the right anchor of child a: the mask recomposes, more content shows
    let before = page(&s);
    assert_eq!(px(&before, 150, 150)[1], px(&before, 150, 150)[0], "gap between the squares shows the page");
    drag(&mut s, (145.0, 120.0), (160.0, 120.0));
    drag(&mut s, (145.0, 180.0), (160.0, 180.0));
    let b2 = visible_bounds(&s, a);
    assert!(approx(b2.right, 160.0) && approx(b2.left, 120.0), "child anchors moved: {b2:?}");
    let after = page(&s);
    assert_eq!(px(&after, 150, 150), [255, 0, 0, 255], "the gap now shows the red content");
    assert_ne!(px(&before, 150, 150), px(&after, 150, 150));
}

#[test]
fn pointer_double_click_enters_a_selected_group_mask() {
    let (mut s, _, mg, _, b) = group_scene();
    tool(&mut s, ToolId::Subselection);
    click(&mut s, 130.0, 150.0);
    assert_eq!(sel(&s), vec![mg]);
    tool(&mut s, ToolId::Pointer);
    events(&mut s, &[ToolEvent::double(170.0, 150.0)]);
    assert_eq!(sel(&s), vec![b]);
}

// ------------------------------------------------------------------ M-10 / M-11 / Ungroup

fn last_step(s: &Session) -> String {
    s.undo_redo_labels().0.unwrap_or_default()
}

#[test]
fn disable_mask_toggles_with_rows_and_without_check_mark() {
    fp_i18n::set_language("de");
    let (mut s, g, mask, _) = scene();
    select(&mut s, &[g]);
    ex(&mut s, "modify.mask.disableMask", json!({}));
    assert!(matches!(&el(&s, g).kind, ElementKind::Group(gr) if !gr.mask_flags.enabled));
    assert_eq!(last_step(&s), "Maske deaktivieren");
    assert_eq!(s.command_state("modify.mask.disableMask").1, None, "no check mark");
    ex(&mut s, "modify.mask.disableMask", json!({}));
    assert!(matches!(&el(&s, g).kind, ElementKind::Group(gr) if gr.mask_flags.enabled));
    assert_eq!(last_step(&s), "Maske aktivieren");
    // with the mask itself selected, the command works on its owner
    select(&mut s, &[mask]);
    let _ = s.execute("modify.mask.disableMask", json!({}));
}

#[test]
fn delete_mask_discard_keeps_the_content() {
    fp_i18n::set_language("de");
    let (mut s, g, mask, bmp) = scene();
    select(&mut s, &[g]);
    ex(&mut s, "modify.mask.deleteMask", json!({"apply": false}));
    let ids: Vec<ElementId> = top_elements(&s).iter().map(|e| e.id).collect();
    assert_eq!(ids, vec![bmp]);
    assert!(!ids.contains(&mask));
    assert_eq!(sel(&s), vec![bmp]);
    assert_eq!(last_step(&s), "Maske löschen");
}

#[test]
fn delete_mask_apply_bakes_the_composed_look_into_a_bitmap() {
    let (mut s, g, _, _) = scene();
    let composed = page(&s);
    select(&mut s, &[g]);
    let out = ex(&mut s, "modify.mask.deleteMask", json!({"apply": true}));
    let id = ElementId(out["ids"][0].as_u64().expect("new bitmap"));
    let tops = top_elements(&s);
    assert_eq!(tops.len(), 1);
    assert_eq!(tops[0].id, id);
    let ElementKind::Bitmap(b) = &tops[0].kind else {
        panic!("not a bitmap")
    };
    let img = fp_engine::docops::bitmap_image(b).expect("image");
    let r = img.bounds();
    assert_eq!((r.left, r.top, r.right, r.bottom), (120, 120, 180, 180), "trimmed to the mask");
    assert_eq!(page(&s), composed, "the page looks the same after applying");
    s.undo().expect("undo");
    assert_eq!(top_elements(&s)[0].id, g);
}

#[test]
fn delete_mask_apply_rasterises_vector_content() {
    let mut s = doc_session(400.0, 300.0);
    let content = ellipse(&mut s, 100.0, 100.0, 200.0, 200.0);
    let mask = rect(&mut s, 150.0, 80.0, 250.0, 220.0);
    select(&mut s, &[mask, content]);
    ex(&mut s, "modify.mask.groupAsMask", json!({}));
    ex(&mut s, "modify.mask.options", json!({"mode": "path"}));
    let composed = page(&s);
    ex(&mut s, "modify.mask.deleteMask", json!({"apply": true}));
    assert!(matches!(top_elements(&s)[0].kind, ElementKind::Bitmap(_)));
    assert_eq!(page(&s), composed);
}

#[test]
fn ungroup_gives_mask_on_top_and_content_both_selected() {
    let (mut s, g, mask, bmp) = scene();
    select(&mut s, &[g]);
    ex(&mut s, "modify.ungroup", json!({}));
    let ids: Vec<ElementId> = top_elements(&s).iter().map(|e| e.id).collect();
    assert_eq!(ids, vec![mask, bmp]);
    let mut got = sel(&s);
    got.sort_by_key(|i| i.0);
    let mut want = vec![mask, bmp];
    want.sort_by_key(|i| i.0);
    assert_eq!(got, want);
}

// ------------------------------------------------------------------ E-2: invert

#[test]
fn invert_option_hides_inside_and_round_trips_through_undo() {
    let (mut s, g, mask, _) = scene();
    let normal = page(&s);
    assert_eq!(px(&normal, 150, 150), [255, 0, 0, 255]);
    assert_eq!(px(&normal, 110, 110), [255, 255, 255, 255]);
    select(&mut s, &[mask]);
    ex(&mut s, "modify.mask.options", json!({"invert": true}));
    assert!(matches!(&el(&s, g).kind, ElementKind::Group(gr) if gr.mask_flags.invert && gr.kind == GroupKind::VectorMask));
    let inv = page(&s);
    assert_eq!(px(&inv, 150, 150), [255, 255, 255, 255], "inside the mask disappears");
    assert_eq!(px(&inv, 110, 110), [255, 0, 0, 255], "outside stays");
    // the masked element keeps the content bounds when inverted
    let b = visible_bounds(&s, g);
    assert!(approx(b.left, 100.0) && approx(b.right, 200.0), "{b:?}");
    s.undo().expect("undo");
    assert!(matches!(&el(&s, g).kind, ElementKind::Group(gr) if !gr.mask_flags.invert));
}
