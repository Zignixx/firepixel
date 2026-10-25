//! TW-225 rest, RE spec 68 §4.2 rule 5, §4.1 `e9`/`e10`, §7: the "Show fill and stroke" underlay of
//! a bitmap mask (Paste Inside onto a bitmap) is not drawn while the mask is in bitmap editing,
//! for the whole edit and not only during strokes; after leaving the edit (stop button) it is back
//! and the option itself stays on. The canvas asks [`DocumentState::bitmap_mask_edit`] for the
//! mask being edited; vector masks are not affected.

use crate::common;
use common::*;
use fp_engine::Session;
use fp_model::{ElementId, ElementKind};
use serde_json::json;

fn mask_edit(s: &Session) -> Option<ElementId> {
    s.active().and_then(|d| d.bitmap_mask_edit())
}

fn id_of(v: &serde_json::Value) -> ElementId {
    ElementId(
        v["id"]
            .as_u64()
            .or_else(|| v["ids"][0].as_u64())
            .unwrap_or(0),
    )
}

fn filled_rect(s: &mut Session, r: [i32; 4], color: &str) -> ElementId {
    id_of(&ex(
        s,
        "draw.rectangle",
        json!({"rect": r, "fillColor": color, "strokeWidth": 0}),
    ))
}

fn show_attrs(s: &Session, g: ElementId) -> bool {
    match &el(s, g).kind {
        ElementKind::Group(gr) => !gr.mask_flags.hide_attrs,
        _ => panic!("mask group"),
    }
}

/// Paste Inside of a white rectangle onto `target`: (masked group, mask id).
fn paste_inside(s: &mut Session, target: ElementId) -> (ElementId, ElementId) {
    let content = filled_rect(s, [0, 0, 100, 100], "#ffffff");
    select(s, &[content]);
    ex(s, "edit.cut", json!({}));
    select(s, &[target]);
    let g = id_of(&ex(s, "edit.pasteInside", json!({})));
    let m = el(s, g).element_mask().map(|m| m.element.id).expect("mask");
    (g, m)
}

#[test]
fn bitmap_mask_edit_lasts_for_the_whole_bitmap_editing() {
    fp_i18n::set_language("en");
    let mut s = doc_session(200.0, 200.0);
    let target = bitmap(&mut s, 30, 0, 40, 100, [255, 0, 0, 255]);
    let (g, m) = paste_inside(&mut s, target);
    assert!(show_attrs(&s, g), "Paste Inside: show fill and stroke on");
    assert_eq!(mask_edit(&s), None, "not editing after Paste Inside");
    select(&mut s, &[g]);
    assert_eq!(
        mask_edit(&s),
        None,
        "the masked object selected: no mask edit"
    );
    ex(&mut s, "panel.layers.editMask", json!({}));
    assert_eq!(
        mask_edit(&s),
        Some(m),
        "editing the bitmap mask, no stroke yet"
    );
    assert!(show_attrs(&s, g), "the option stays on while editing");
    ex(&mut s, "modify.exitBitmapMode", json!({}));
    assert_eq!(
        s.active().map(|d| d.selection.elements.clone()),
        Some(vec![m]),
        "the mask stays selected after the stop button"
    );
    assert_eq!(
        mask_edit(&s),
        None,
        "after leaving the edit the underlay is back"
    );
    assert!(show_attrs(&s, g));
}

#[test]
fn vector_masks_and_plain_bitmaps_are_no_mask_edit() {
    fp_i18n::set_language("en");
    let mut s = doc_session(200.0, 200.0);
    let target = filled_rect(&mut s, [10, 10, 90, 90], "#ff0000");
    let (g, m) = paste_inside(&mut s, target);
    select(&mut s, &[g]);
    ex(&mut s, "panel.layers.editMask", json!({}));
    assert_eq!(
        s.active().map(|d| d.selection.elements.clone()),
        Some(vec![m])
    );
    assert_eq!(
        mask_edit(&s),
        None,
        "a vector mask keeps its underlay while selected"
    );

    let mut s = doc_session(200.0, 200.0);
    let b = bitmap(&mut s, 0, 0, 50, 50, [0, 0, 255, 255]);
    select(&mut s, &[b]);
    ex(&mut s, "modify.editBitmap", json!({}));
    assert_eq!(mask_edit(&s), None, "a bitmap that is no mask");
}
