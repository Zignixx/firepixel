//! FB-35/FB-36 (RE spec 68 §1, §2, §7, §8): mask mode per command, mode kept apart from the kind
//! of the mask object, grayscale from the rendered luma, History row names (de/en), colour wells
//! grey while a mask is selected, grayscale bitmap masks in display form.

use crate::common;
use common::*;
use fp_engine::Session;
use fp_model::{ElementId, ElementKind, GroupKind};
use serde_json::json;

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

fn label(s: &Session) -> Option<String> {
    s.active()
        .and_then(|d| d.history.undo_label().map(str::to_string))
}

fn group_kind(s: &Session, g: ElementId) -> (GroupKind, bool) {
    match &el(s, g).kind {
        ElementKind::Group(gr) => (gr.kind, gr.mask_flags.hide_attrs),
        _ => panic!("no group"),
    }
}

/// Alpha of the masked element at `x`,`y` (white content: alpha = mask value).
fn alpha_at(s: &Session, g: ElementId, x: i32, y: i32) -> u8 {
    let d = s.active().expect("doc");
    let e = el(s, g);
    let r = fp_render::Renderer::new(fp_render::RenderOptions {
        cache_policy: fp_render::CachePolicy::None,
        ..fp_render::RenderOptions::default()
    });
    let img = r
        .render_element(&d.doc, &e, &fp_render::PointMap::identity(), 0)
        .image;
    img.get(x, y).map(|p| p[3]).unwrap_or(0)
}

/// White square 0..100 with a red square 10..90 above, grouped as mask.
fn red_mask(lang: &str) -> (Session, ElementId) {
    fp_i18n::set_language(lang);
    let mut s = doc_session(200.0, 200.0);
    let c = filled_rect(&mut s, [0, 0, 100, 100], "#ffffff");
    let m = filled_rect(&mut s, [10, 10, 90, 90], "#ff0000");
    select(&mut s, &[c, m]);
    let g = id_of(&ex(&mut s, "modify.mask.groupAsMask", json!({})));
    (s, g)
}

#[test]
fn group_as_mask_is_grayscale_and_named_group() {
    let (s, g) = red_mask("de");
    assert_eq!(
        group_kind(&s, g),
        (GroupKind::BitmapMask, true),
        "mask to image, show attrs off"
    );
    assert_eq!(
        s.active().map(|d| d.selection.elements.clone()),
        Some(vec![g]),
        "masked element selected"
    );
    assert_eq!(label(&s).as_deref(), Some("Gruppieren"));
    let a = alpha_at(&s, g, 50, 50);
    assert!(a.abs_diff(54) <= 1, "red → 21 %: {a}");
    assert_eq!(alpha_at(&s, g, 5, 5), 0);
    let (s, _) = red_mask("en");
    assert_eq!(label(&s).as_deref(), Some("Group"));
}

#[test]
fn group_as_mask_is_grayscale_for_bitmaps_and_groups() {
    fp_i18n::set_language("de");
    let mut s = doc_session(200.0, 200.0);
    let c = filled_rect(&mut s, [0, 0, 100, 100], "#ffffff");
    let b = bitmap(&mut s, 10, 10, 50, 50, [0, 255, 0, 255]);
    select(&mut s, &[c, b]);
    let g = id_of(&ex(&mut s, "modify.mask.groupAsMask", json!({})));
    assert_eq!(group_kind(&s, g).0, GroupKind::BitmapMask);
    let a = alpha_at(&s, g, 30, 30);
    assert!(a.abs_diff(182) <= 1, "green bitmap → 72 %: {a}");

    let mut s = doc_session(200.0, 200.0);
    let c = filled_rect(&mut s, [0, 0, 100, 100], "#ffffff");
    let m1 = filled_rect(&mut s, [10, 10, 40, 40], "#ffffff");
    let m2 = filled_rect(&mut s, [50, 50, 90, 90], "#ffffff");
    select(&mut s, &[m1, m2]);
    let mg = id_of(&ex(&mut s, "modify.group", json!({})));
    select(&mut s, &[c, mg]);
    let g = id_of(&ex(&mut s, "modify.mask.groupAsMask", json!({})));
    assert_eq!(group_kind(&s, g).0, GroupKind::BitmapMask);
    assert_eq!(alpha_at(&s, g, 20, 20), 255);
    assert_eq!(alpha_at(&s, g, 45, 20), 0);
}

#[test]
fn mode_switch_keeps_the_vector_mask_and_is_modify_mask() {
    let (mut s, g) = red_mask("en");
    let mask_id = el(&s, g)
        .element_mask()
        .map(|m| m.element.id)
        .expect("mask");
    ex(&mut s, "panel.layers.editMask", json!({}));
    assert_eq!(
        s.active().map(|d| d.selection.elements.clone()),
        Some(vec![mask_id])
    );
    assert_eq!(
        label(&s).as_deref(),
        Some("Modify Mask"),
        "selecting the mask is a row"
    );
    // wells grey while the mask is selected (red → #363636)
    let gray = fp_engine::tool_follow::mask_gray(fp_color::Argb(0xffff_0000));
    assert_eq!(gray, fp_color::Argb(0xff36_3636));
    assert!(s.mask_selected());

    ex(&mut s, "modify.mask.options", json!({"mode": "path"}));
    assert_eq!(label(&s).as_deref(), Some("Modify Mask"));
    assert_eq!(group_kind(&s, g).0, GroupKind::VectorMask);
    let e = el(&s, g);
    let m = e.element_mask().expect("mask");
    assert!(
        matches!(m.element.kind, ElementKind::Path(_)),
        "still the path"
    );
    assert_eq!(m.element.id, mask_id);
    assert_eq!(alpha_at(&s, g, 50, 50), 255, "path outline: geometry");

    ex(&mut s, "modify.mask.options", json!({"mode": "image"}));
    assert_eq!(group_kind(&s, g).0, GroupKind::BitmapMask);
    assert!(alpha_at(&s, g, 50, 50).abs_diff(54) <= 1);
    ex(&mut s, "modify.mask.options", json!({"mode": "path"}));
    assert_eq!(
        group_kind(&s, g).0,
        GroupKind::VectorMask,
        "the way back always works"
    );
    ex(&mut s, "modify.mask.options", json!({"showAttrs": true}));
    assert_eq!(group_kind(&s, g), (GroupKind::VectorMask, false));
    fp_i18n::set_language("de");
    ex(&mut s, "modify.mask.options", json!({"showAttrs": false}));
    assert_eq!(label(&s).as_deref(), Some("Maske modifizieren"));
}

#[test]
fn paste_inside_and_paste_as_mask_modes_and_rows() {
    fp_i18n::set_language("en");
    let mut s = doc_session(200.0, 200.0);
    let target = filled_rect(&mut s, [10, 10, 90, 90], "#ff0000");
    let content = filled_rect(&mut s, [0, 0, 100, 100], "#ffffff");
    select(&mut s, &[content]);
    ex(&mut s, "edit.cut", json!({}));
    select(&mut s, &[target]);
    let g = id_of(&ex(&mut s, "edit.pasteInside", json!({})));
    assert_eq!(
        group_kind(&s, g),
        (GroupKind::VectorMask, false),
        "path outline + show fill and stroke"
    );
    assert_eq!(label(&s).as_deref(), Some("Paste"));

    // Paste as Mask: vector → path outline, the mask selected
    let mut s = doc_session(200.0, 200.0);
    let content = filled_rect(&mut s, [0, 0, 100, 100], "#ffffff");
    let m = filled_rect(&mut s, [10, 10, 90, 90], "#ff0000");
    select(&mut s, &[m]);
    ex(&mut s, "edit.cut", json!({}));
    select(&mut s, &[content]);
    ex(&mut s, "edit.pasteAsMask", json!({}));
    fp_i18n::set_language("de");
    assert_eq!(
        label(&s).as_deref(),
        Some("Paste"),
        "label taken when the command ran"
    );
    let g = top_elements(&s)[0].id;
    assert_eq!(group_kind(&s, g), (GroupKind::VectorMask, true));
    let mask_id = el(&s, g).element_mask().map(|m| m.element.id);
    assert_eq!(
        s.active()
            .and_then(|d| d.selection.elements.first().copied()),
        mask_id
    );

    // bitmap → grayscale, bitmap editing of the mask, row "Einfügen"
    let mut s = doc_session(200.0, 200.0);
    let content = filled_rect(&mut s, [0, 0, 100, 100], "#ffffff");
    let b = bitmap(&mut s, 10, 10, 50, 50, [255, 255, 255, 255]);
    select(&mut s, &[b]);
    ex(&mut s, "edit.cut", json!({}));
    select(&mut s, &[content]);
    ex(&mut s, "edit.pasteAsMask", json!({}));
    assert_eq!(label(&s).as_deref(), Some("Einfügen"));
    let g = top_elements(&s)[0].id;
    assert_eq!(group_kind(&s, g).0, GroupKind::BitmapMask);
    assert!(s.active().is_some_and(|d| d.bitmap_mode));
    assert_eq!(alpha_at(&s, g, 30, 30), 255, "white bitmap shows");
}

#[test]
fn hide_all_and_reveal_all_add_a_display_form_mask() {
    fp_i18n::set_language("de");
    let mut s = doc_session(200.0, 200.0);
    let c = filled_rect(&mut s, [0, 0, 100, 100], "#ffffff");
    select(&mut s, &[c]);
    let g = id_of(&ex(&mut s, "modify.mask.hideAll", json!({})));
    assert_eq!(label(&s).as_deref(), Some("Maske hinzufügen"));
    assert_eq!(group_kind(&s, g).0, GroupKind::BitmapMask);
    let mask_px = |s: &Session, g: ElementId| {
        let e = el(s, g);
        let m = e.element_mask().expect("mask");
        match &m.element.kind {
            ElementKind::Bitmap(b) => b.image.to_rgba().0[..4].to_vec(),
            _ => panic!("bitmap mask"),
        }
    };
    assert_eq!(
        mask_px(&s, g),
        vec![0, 0, 0, 255],
        "hide all = black in display form"
    );
    assert_eq!(alpha_at(&s, g, 50, 50), 0);

    fp_i18n::set_language("en");
    let mut s = doc_session(200.0, 200.0);
    let c = filled_rect(&mut s, [0, 0, 100, 100], "#ffffff");
    select(&mut s, &[c]);
    let g = id_of(&ex(&mut s, "modify.mask.revealAll", json!({})));
    assert_eq!(label(&s).as_deref(), Some("Add Mask"));
    assert_eq!(
        mask_px(&s, g),
        vec![255, 255, 255, 255],
        "reveal all = white"
    );
    assert_eq!(alpha_at(&s, g, 50, 50), 255);
}
