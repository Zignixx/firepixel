//! FB-59 to FB-61 (Firepixel extension): the helpers behind the expandable group and mask rows of
//! the Layers panel (`docops::tree_children`, `mask_members`, `has_selected_descendant`) and the
//! panel switch `panel.layers.groupTree` (preference, no History step).

use fp_engine::Session;
use fp_engine::docops::{self, MaskMembers};
use fp_model::{Element, ElementId};
use serde_json::{Value, json};
use std::sync::Arc;

fn ex(s: &mut Session, c: &str, p: Value) -> Value {
    s.execute(c, p).unwrap_or_else(|e| panic!("{c}: {e}")).value
}

fn rect(s: &mut Session, x: f64) -> u64 {
    ex(s, "draw.rectangle", json!({"rect": [x, 20.0, x + 40.0, 60.0]}))["id"]
        .as_u64()
        .expect("id")
}

fn el(s: &Session, id: u64) -> Arc<Element> {
    let d = s.active().expect("doc");
    docops::element_on_page(&d.doc, d.page, ElementId(id)).expect("element")
}

fn group(s: &mut Session, ids: &[u64]) -> u64 {
    ex(s, "select.set", json!({ "ids": ids }));
    ex(s, "modify.group", json!({}))["id"].as_u64().expect("group id")
}

fn ids(v: Option<&[Arc<Element>]>) -> Vec<u64> {
    v.map(|cs| cs.iter().map(|c| c.id.0).collect()).unwrap_or_default()
}

fn session() -> Session {
    fp_i18n::set_language("en");
    let mut s = Session::default();
    ex(&mut s, "file.new", json!({"width": 400, "height": 200, "background": "#ffffff"}));
    s
}

#[test]
fn group_rows_list_their_children_recursively() {
    let mut s = session();
    let (a, b, c) = (rect(&mut s, 10.0), rect(&mut s, 60.0), rect(&mut s, 110.0));
    let inner = group(&mut s, &[a, b]);
    let outer = group(&mut s, &[inner, c]);
    let g = el(&s, outer);
    let kids = ids(docops::tree_children(&g));
    assert_eq!(kids.len(), 2);
    assert!(kids.contains(&inner) && kids.contains(&c), "{kids:?}");
    let mut inner_kids = ids(docops::tree_children(&el(&s, inner)));
    inner_kids.sort();
    assert_eq!(inner_kids, vec![a, b]);
    assert!(docops::tree_children(&el(&s, c)).is_none(), "a rectangle lists nothing");
    // a child at any depth counts, the group itself does not
    let sel = |want: u64| move |id: ElementId| id.0 == want;
    assert!(docops::has_selected_descendant(&g, &sel(a)));
    assert!(docops::has_selected_descendant(&g, &sel(inner)));
    assert!(!docops::has_selected_descendant(&g, &sel(outer)));
    assert!(!docops::has_selected_descendant(&el(&s, inner), &sel(c)));
}

#[test]
fn mask_rows_list_the_objects_of_the_mask() {
    let mut s = session();
    // a vector mask over two objects: a mask group listing its two contents, the path as mask
    let (a, b, m) = (rect(&mut s, 10.0), rect(&mut s, 30.0), rect(&mut s, 20.0));
    ex(&mut s, "select.set", json!({"ids": [a, b, m]}));
    let mg = ex(&mut s, "modify.mask.groupAsMask", json!({}))["id"].as_u64().expect("mask group");
    let e = el(&s, mg);
    let mut kids = ids(docops::tree_children(&e));
    kids.sort();
    assert_eq!(kids, vec![a, b], "contents without the mask");
    match docops::mask_members(&e) {
        Some(MaskMembers::Single(x)) => assert_eq!(x.id.0, m),
        other => panic!("vector mask of one path: {other:?}"),
    }
    assert!(docops::mask_contains(&e, ElementId(m)));
    // the mask of the object itself is not a "subselected child" (mask editing has its own marks)
    assert!(!docops::has_selected_descendant(&e, &|id| id.0 == m));

    // a masked single object: no contents toggle, the mask path as the one mask row
    let mut s = session();
    let (a, m) = (rect(&mut s, 10.0), rect(&mut s, 20.0));
    ex(&mut s, "select.set", json!({"ids": [a, m]}));
    let mg = ex(&mut s, "modify.mask.groupAsMask", json!({}))["id"].as_u64().expect("mask group");
    let e = el(&s, mg);
    assert!(e.masked_single().is_some());
    assert!(docops::tree_children(&e).is_none());
    assert!(matches!(docops::mask_members(&e), Some(MaskMembers::Single(x)) if x.id.0 == m));

    // a bitmap mask: one "Bitmap mask" row
    let mut s = session();
    let a = rect(&mut s, 10.0);
    ex(&mut s, "select.set", json!({"ids": [a]}));
    let out = ex(&mut s, "panel.layers.addMask", json!({}));
    let e = el(&s, out["id"].as_u64().expect("masked object"));
    assert!(matches!(docops::mask_members(&e), Some(MaskMembers::Bitmap(_))));
    assert!(docops::mask_members(&el(&s, out["mask"].as_u64().expect("mask"))).is_none());

    // a group as mask: one row per object of the mask group
    let mut s = session();
    let a = rect(&mut s, 10.0);
    let (m1, m2) = (rect(&mut s, 20.0), rect(&mut s, 40.0));
    let gm = group(&mut s, &[m1, m2]);
    ex(&mut s, "select.set", json!({"ids": [a, gm]}));
    let mg = ex(&mut s, "modify.mask.groupAsMask", json!({}))["id"].as_u64().expect("mask group");
    let e = el(&s, mg);
    match docops::mask_members(&e) {
        Some(MaskMembers::Group(cs)) => {
            let mut v: Vec<u64> = cs.iter().map(|c| c.id.0).collect();
            v.sort();
            assert_eq!(v, vec![m1, m2]);
        }
        other => panic!("group mask: {other:?}"),
    }
    assert!(docops::mask_has_selected(&e, &|id| id.0 == m2));
    assert!(!docops::mask_has_selected(&e, &|id| id.0 == a));
}

#[test]
fn group_tree_switch_is_a_preference_without_history() {
    let mut s = session();
    assert!(fp_engine::session::layers_group_tree(&s.prefs), "on by default");
    assert_eq!(s.command_state("panel.layers.groupTree").1, Some(true));
    let steps = s.active().map(|d| d.history.undo_count()).unwrap_or(0);
    ex(&mut s, "panel.layers.groupTree", json!({}));
    assert!(!fp_engine::session::layers_group_tree(&s.prefs));
    assert_eq!(s.command_state("panel.layers.groupTree").1, Some(false));
    assert_eq!(s.active().map(|d| d.history.undo_count()).unwrap_or(0), steps, "no History step");
    ex(&mut s, "panel.layers.groupTree", json!({}));
    assert!(fp_engine::session::layers_group_tree(&s.prefs));
}

/// FB-80 (Firepixel extension): the content of a masked single object is an element of its own
/// (the one child of the mask group); selecting it alone moves it without the mask, like the
/// children of a mask group over several objects (vector and bitmap mask alike).
#[test]
fn content_of_a_masked_single_object_is_selectable_alone() {
    let bounds = |s: &Session, id: u64| {
        let d = s.active().expect("doc");
        docops::element_bounds(&d.doc, &el(s, id)).expect("bounds")
    };
    // vector mask
    let mut s = session();
    let (a, m) = (rect(&mut s, 10.0), rect(&mut s, 20.0));
    ex(&mut s, "select.set", json!({"ids": [a, m]}));
    let mg = ex(&mut s, "modify.mask.groupAsMask", json!({}))["id"].as_u64().expect("mask group");
    let e = el(&s, mg);
    assert_eq!(ids(docops::mask_content(&e)), vec![a], "the content is listed");
    let (a0, m0) = (bounds(&s, a), bounds(&s, m));
    ex(&mut s, "select.set", json!({"ids": [a]}));
    assert_eq!(s.active().expect("d").selection.elements, vec![ElementId(a)]);
    ex(&mut s, "edit.move", json!({"dx": 15.0, "dy": 0.0}));
    let (a1, m1) = (bounds(&s, a), bounds(&s, m));
    assert!((a1.left - a0.left - 15.0).abs() < 0.01, "content moved: {a0:?} -> {a1:?}");
    assert!((m1.left - m0.left).abs() < 0.01, "mask stays: {m0:?} -> {m1:?}");
    assert!(docops::has_selected_descendant(&el(&s, mg), &|id| id.0 == a), "row tinted");
    // bitmap mask
    let mut s = session();
    let a = rect(&mut s, 10.0);
    ex(&mut s, "select.set", json!({"ids": [a]}));
    let out = ex(&mut s, "panel.layers.addMask", json!({}));
    let e = el(&s, out["id"].as_u64().expect("masked object"));
    let content = ids(docops::mask_content(&e));
    assert_eq!(content.len(), 1, "one content object");
    let a0 = bounds(&s, content[0]);
    ex(&mut s, "select.set", json!({"ids": [content[0]]}));
    ex(&mut s, "edit.move", json!({"dx": 0.0, "dy": 12.0}));
    let a1 = bounds(&s, content[0]);
    assert!((a1.top - a0.top - 12.0).abs() < 0.01, "content moved: {a0:?} -> {a1:?}");
}
