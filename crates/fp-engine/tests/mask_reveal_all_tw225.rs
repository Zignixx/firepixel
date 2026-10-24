//! TW-225 rest, RE spec 68 §8 and §7: Modify ▸ Mask ▸ Reveal All / Hide All (and the Layers
//! panel's Add Mask) put the new mask into mask editing like a click on its Layers thumbnail
//! (`panel.layers.editMask`): the mask is selected, bitmap editing is on, one History row
//! "Add Mask"; the mask is an empty grayscale bitmap of the canvas size (parity run 44
//! `inspector-bitmap/08`: 800 × 600 at 0, 0 for a 480 × 320 bitmap at 40, 40). Undo restores the
//! unmasked object.

use crate::common;
use common::*;
use fp_engine::Session;
use fp_engine::docops;
use fp_model::{ElementId, ElementKind, GroupKind};
use serde_json::json;

fn sel(s: &Session) -> Vec<ElementId> {
    s.active()
        .map(|d| d.selection.elements.clone())
        .unwrap_or_default()
}

fn bitmap_mode(s: &Session) -> bool {
    s.active().is_some_and(|d| d.bitmap_mode)
}

fn labels(s: &Session) -> Vec<String> {
    s.active().map(|d| d.history.labels()).unwrap_or_default()
}

fn mask_rect(s: &Session, g: ElementId) -> (i32, i32, u32, u32) {
    let e = el(s, g);
    let m = e.element_mask().expect("mask");
    match &m.element.kind {
        ElementKind::Bitmap(b) => {
            let img = docops::bitmap_image(b).expect("mask pixels");
            let r = img.bounds();
            (r.left, r.top, img.width() as u32, img.height() as u32)
        }
        _ => panic!("bitmap mask"),
    }
}

/// Bitmap (T02 shape, scaled) or vector object as the object to mask.
fn objects(s: &mut Session) -> Vec<(&'static str, ElementId)> {
    let b = bitmap(s, 40, 40, 480, 320, [200, 30, 30, 255]);
    let r = rect(s, 600.0, 400.0, 700.0, 500.0);
    vec![("bitmap", b), ("vector", r)]
}

#[test]
fn reveal_and_hide_all_enter_mask_editing() {
    fp_i18n::set_language("en");
    for cmd in ["modify.mask.revealAll", "modify.mask.hideAll", "panel.layers.addMask"] {
        let mut s = doc_session(800.0, 600.0);
        for (what, obj) in objects(&mut s) {
            let _ = s.execute("modify.exitBitmapMode", json!({}));
            select(&mut s, &[obj]);
            let rows = labels(&s).len();
            let out = ex(&mut s, cmd, json!({}));
            let g = ElementId(out["id"].as_u64().expect("masked object"));
            let mask = el(&s, g).element_mask().map(|m| m.element.id).expect("mask");
            assert_eq!(out["mask"].as_u64(), Some(mask.0), "{cmd} {what}: mask id reported");
            assert_eq!(sel(&s), vec![mask], "{cmd} {what}: the new mask is selected");
            assert!(bitmap_mode(&s), "{cmd} {what}: bitmap editing of the mask");
            assert_eq!(
                docops::mask_owner(&s.active().expect("d").doc, 0, mask).map(|o| o.id),
                Some(g),
                "{cmd} {what}: the selected mask belongs to the masked object"
            );
            let ElementKind::Group(gr) = &el(&s, g).kind else { panic!("mask group") };
            assert_eq!(gr.kind, GroupKind::BitmapMask, "{cmd} {what}: grayscale");
            assert!(gr.mask_flags.linked, "{cmd} {what}: linked");
            assert_eq!(mask_rect(&s, g), (0, 0, 800, 600), "{cmd} {what}: canvas-size mask");
            let after = labels(&s);
            assert_eq!(after.len(), rows + 1, "{cmd} {what}: one History row");
            assert_eq!(after.last().map(String::as_str), Some("Add Mask"), "{cmd} {what}");
        }
    }
}

#[test]
fn reveal_all_undo_restores_the_object() {
    fp_i18n::set_language("en");
    let mut s = doc_session(800.0, 600.0);
    let b = bitmap(&mut s, 40, 40, 480, 320, [200, 30, 30, 255]);
    select(&mut s, &[b]);
    ex(&mut s, "modify.mask.revealAll", json!({}));
    assert!(top_elements(&s)[0].element_mask().is_some());
    ex(&mut s, "edit.undo", json!({}));
    let tops = top_elements(&s);
    assert_eq!(tops.len(), 1);
    assert_eq!(tops[0].id, b, "undo removes the mask");
    assert!(tops[0].element_mask().is_none());
    assert!(
        sel(&s).iter().all(|id| *id == b),
        "undo leaves no selected mask behind: {:?}",
        sel(&s)
    );
    ex(&mut s, "edit.redo", json!({}));
    assert!(top_elements(&s)[0].element_mask().is_some(), "redo adds it again");
}

#[test]
fn object_beyond_the_canvas_keeps_its_pixels() {
    // the canvas-size mask widens to an object that leaves the canvas, so Reveal All hides nothing
    let mut s = doc_session(200.0, 100.0);
    let b = bitmap(&mut s, 150, 50, 100, 100, [0, 0, 255, 255]);
    select(&mut s, &[b]);
    let g = ElementId(ex(&mut s, "modify.mask.revealAll", json!({}))["id"].as_u64().expect("id"));
    assert_eq!(mask_rect(&s, g), (0, 0, 250, 150));
}
