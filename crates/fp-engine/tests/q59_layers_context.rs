//! Q-59 / TW-193 (RE spec 58 §3): Layers panel context menu. States of Flatten Selection, Merge Down,
//! Add Mask and Share Layer to Pages follow the selection and the current layer; Flatten Selection on a
//! single vector gives a bitmap whose bounds include the stroke; Add Mask adds an empty, linked bitmap
//! mask in one step "Add Mask" and selects the mask.

use crate::common::*;
use fp_engine::docops;
use fp_model::{ElementId, ElementKind, LayerKind};
use serde_json::json;

fn enabled(s: &fp_engine::Session, name: &str) -> bool {
    s.command_state(name).0
}

/// Measured test document: path P1 on top, group G1 (two rectangles), bitmap B1 at the bottom.
fn q59_doc() -> (fp_engine::Session, ElementId, ElementId, ElementId) {
    fp_i18n::set_language("de");
    let mut s = doc_session(600.0, 400.0);
    let b = rect(&mut s, 20.0, 20.0, 120.0, 120.0);
    select(&mut s, &[b]);
    let v = ex(&mut s, "modify.flattenSelection", json!({}));
    let b = ElementId(v["id"].as_u64().expect("bitmap"));
    let g1 = rect(&mut s, 150.0, 20.0, 200.0, 80.0);
    let g2 = rect(&mut s, 210.0, 20.0, 260.0, 80.0);
    select(&mut s, &[g1, g2]);
    let v = ex(&mut s, "modify.group", json!({}));
    let g = ElementId(v["id"].as_u64().unwrap_or_else(|| s.active().expect("d").selection.elements[0].0));
    let p = rect(&mut s, 300.0, 20.0, 450.0, 120.0);
    ex(&mut s, "element.setStroke", json!({"ids": [p.0], "color": "#000000", "width": 4, "placement": "outside"}));
    ex(&mut s, "select.none", json!({}));
    (s, p, g, b)
}

#[test]
fn menu_states_follow_selection() {
    let (mut s, p, g, b) = q59_doc();
    let tops: Vec<ElementId> = top_elements(&s).iter().map(|e| e.id).collect();
    assert_eq!(tops, vec![p, g, b], "path, group, bitmap from the top");
    // nothing selected (layer row): Flatten, Merge Down, Add Mask grey
    for c in ["panel.layers.flattenSelection", "panel.layers.mergeDown", "panel.layers.addMask"] {
        assert!(!enabled(&s, c), "{c} without selection");
    }
    // (a) path above the group, (d) group above the bitmap, bitmap at the bottom
    for (id, merge) in [(p, false), (g, true), (b, false)] {
        select(&mut s, &[id]);
        assert!(enabled(&s, "panel.layers.flattenSelection"), "Flatten {id:?}");
        assert!(enabled(&s, "panel.layers.addMask"), "Add Mask {id:?}");
        assert_eq!(enabled(&s, "panel.layers.mergeDown"), merge, "Merge Down {id:?}");
    }
    // (b) bitmap on top, the path below: Merge Down grey
    select(&mut s, &[b]);
    ex(&mut s, "modify.arrange.bringToFront", json!({}));
    assert_eq!(top_elements(&s)[0].id, b);
    assert!(!enabled(&s, "panel.layers.mergeDown"));
    assert!(enabled(&s, "panel.layers.flattenSelection"));
}

#[test]
fn share_to_pages_grey_on_web_layer() {
    let (mut s, ..) = q59_doc();
    assert!(enabled(&s, "panel.layers.shareToPages"));
    assert!(enabled(&s, "panel.layers.shareToAllPages"));
    let web = {
        let d = s.active().expect("d");
        docops::own_layers(&d.doc, d.page).into_iter().find(|(_, l)| l.kind == LayerKind::Web).map(|(_, l)| l.id).expect("web layer")
    };
    ex(&mut s, "layer.setActive", json!({"layer": web.0}));
    assert!(!enabled(&s, "panel.layers.shareToPages"), "web layer: Share Layer to Pages grey");
    assert!(!enabled(&s, "panel.layers.shareToAllPages"), "web layer: Share Layer to All Pages grey");
}

#[test]
fn flatten_single_vector_bounds_include_stroke() {
    let (mut s, p, ..) = q59_doc();
    select(&mut s, &[p]);
    let v = ex(&mut s, "panel.layers.flattenSelection", json!({}));
    let id = ElementId(v["id"].as_u64().expect("id"));
    assert!(matches!(el(&s, id).kind, ElementKind::Bitmap(_)), "path became a bitmap");
    assert_eq!(s.active().expect("d").selection.elements, vec![id], "bitmap selected");
    let r = bounds(&s, id);
    // RE spec 58 §3.3: 300,20 150 × 100 with a 4 px outside stroke → 296,16 158 × 108
    assert!(rect_approx(r, fp_geom::Rect::new(296.0, 16.0, 454.0, 124.0)), "bounds {r:?}");
}

#[test]
fn add_mask_adds_linked_empty_mask_and_selects_it() {
    let (mut s, p, ..) = q59_doc();
    select(&mut s, &[p]);
    let v = ex(&mut s, "panel.layers.addMask", json!({}));
    let tops = top_elements(&s);
    let e = &tops[0];
    let (child, mask) = e.masked_single().expect("masked single element");
    assert_eq!(child.id, p, "the path carries the mask");
    assert!(matches!(mask.kind, ElementKind::Bitmap(_)), "bitmap mask");
    let m = e.element_mask().expect("mask");
    assert!(m.linked, "linked");
    assert!(!m.to_path, "mask to image");
    assert_eq!(v["mask"].as_u64(), Some(mask.id.0));
    assert_eq!(s.active().expect("d").selection.elements, vec![mask.id], "the mask is selected");
    // empty mask that shows everything (display form in memory, white = visible; the file
    // stores the inverted grey, RE spec 68 M-6)
    let ElementKind::Bitmap(bm) = &mask.kind else { unreachable!() };
    let img = fp_engine::docops::bitmap_image(bm).expect("mask pixels");
    assert!(img.data().iter().all(|px| px[0] == 255 && px[3] == 255), "reveal all");
    assert_eq!(s.active().expect("d").history.undo_label(), Some("Maske hinzufügen"));
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(top_elements(&s)[0].id, p, "undo removes the mask");
}
