//! FB-90 (TW-268, RE spec 68 §4.2, §7): selecting a bitmap mask (mask thumbnail = Edit Mask,
//! Subselection of the mask, bitmap editing) must not change how the masked object looks on the
//! canvas; after the edit and after deselecting the look is the same again. The page is rendered
//! the way the canvas does it ([`fp_render::Renderer::render_device_editing`] with
//! [`fp_engine::DocumentState::bitmap_mask_edit`]), with one renderer for all states (caches).

use crate::common;
use common::*;
use fp_engine::Session;
use fp_geom::IRect;
use fp_model::ElementId;
use serde_json::json;

fn id_of(v: &serde_json::Value) -> ElementId {
    ElementId(
        v["id"]
            .as_u64()
            .or_else(|| v["ids"][0].as_u64())
            .unwrap_or(0),
    )
}

fn page(s: &Session, r: &fp_render::Renderer) -> Vec<u8> {
    let d = s.active().expect("doc");
    let pr = IRect::new(0, 0, 200, 200);
    r.render_device_editing(
        &d.doc,
        d.page,
        d.frame,
        pr,
        (0, 0),
        1.0,
        pr,
        d.bitmap_mask_edit(),
    )
    .image
    .as_bytes()
    .to_vec()
}

fn mask_of(s: &Session, g: ElementId) -> ElementId {
    el(s, g).element_mask().map(|m| m.element.id).expect("mask")
}

/// Selects the mask in each way the UI offers and compares the page with the look before.
fn check(name: &str, mut s: Session, g: ElementId) {
    let r = fp_render::Renderer::new(fp_render::RenderOptions::default());
    select(&mut s, &[]);
    let _ = s.execute("modify.exitBitmapMode", json!({}));
    let before = page(&s, &r);
    assert!(
        before.chunks(4).any(|p| p[3] > 0),
        "{name}: something drawn"
    );
    let m = mask_of(&s, g);
    let same = |s: &Session, what: &str| {
        let now = page(s, &r);
        let diff = before
            .chunks(4)
            .zip(now.chunks(4))
            .filter(|(a, b)| a != b)
            .count();
        assert_eq!(diff, 0, "{name}: {what}: {diff} pixels differ");
    };
    // mask thumbnail / Edit Mask
    select(&mut s, &[g]);
    same(&s, "masked object selected");
    ex(&mut s, "panel.layers.editMask", json!({}));
    assert_eq!(
        s.active().map(|d| d.selection.elements.clone()),
        Some(vec![m])
    );
    same(&s, "mask selected (Edit Mask)");
    let _ = s.execute("modify.exitBitmapMode", json!({}));
    same(&s, "after the stop button");
    select(&mut s, &[]);
    same(&s, "after deselecting");
    // Subselection of the mask, then bitmap editing
    select(&mut s, &[m]);
    same(&s, "mask selected (Subselection)");
    let _ = s.execute("modify.editBitmap", json!({}));
    same(&s, "mask in bitmap editing");
    let _ = s.execute("modify.exitBitmapMode", json!({}));
    select(&mut s, &[]);
    same(&s, "after deselecting again");
}

#[test]
fn a_selected_bitmap_mask_keeps_the_masked_look() {
    fp_i18n::set_language("en");
    // Add Mask (Reveal All) on a bitmap
    let mut s = doc_session(200.0, 200.0);
    let b = bitmap(&mut s, 20, 20, 120, 100, [255, 0, 0, 255]);
    select(&mut s, &[b]);
    let g = id_of(&ex(&mut s, "modify.mask.revealAll", json!({})));
    check("reveal all", s, g);

    // Group as Mask with a bitmap mask over a bitmap
    let mut s = doc_session(200.0, 200.0);
    let c = bitmap(&mut s, 0, 0, 150, 150, [0, 0, 255, 255]);
    let m = bitmap(&mut s, 30, 30, 60, 60, [255, 255, 255, 255]);
    select(&mut s, &[c, m]);
    let g = id_of(&ex(&mut s, "modify.mask.groupAsMask", json!({})));
    check("group as mask", s, g);

    // Paste as Mask of a bitmap onto a bitmap
    let mut s = doc_session(200.0, 200.0);
    let c = bitmap(&mut s, 0, 0, 150, 150, [0, 128, 0, 255]);
    let m = bitmap(&mut s, 40, 40, 50, 70, [255, 255, 255, 255]);
    select(&mut s, &[m]);
    ex(&mut s, "edit.cut", json!({}));
    select(&mut s, &[c]);
    ex(&mut s, "edit.pasteAsMask", json!({}));
    let g = top_elements(&s)[0].id;
    check("paste as mask", s, g);

    // Paste Inside onto a bitmap (the FB-90 recording): the mask bitmap is also drawn as the
    // "Show fill and stroke" underlay; content and underlay keep their look during the edit and
    // afterwards (RE spec 73 §0.5 and §7, superseding RE spec 68 §4.1 `e9`/`e10`)
    let mut s = doc_session(200.0, 200.0);
    let target = bitmap(&mut s, 20, 20, 120, 120, [255, 0, 0, 255]);
    let content = bitmap(&mut s, 60, 60, 40, 40, [0, 0, 255, 255]);
    select(&mut s, &[content]);
    ex(&mut s, "edit.cut", json!({}));
    select(&mut s, &[target]);
    let g = id_of(&ex(&mut s, "edit.pasteInside", json!({})));
    let m = mask_of(&s, g);
    let r = fp_render::Renderer::new(fp_render::RenderOptions::default());
    select(&mut s, &[]);
    let px = |s: &Session, x: usize, y: usize| page(s, &r)[(y * 200 + x) * 4..][..4].to_vec();
    let (inside, under) = (px(&s, 80, 80), px(&s, 30, 30));
    assert_eq!(
        (inside.clone(), under.clone()),
        (vec![0, 0, 255, 255], vec![255, 0, 0, 255])
    );
    select(&mut s, &[g]);
    ex(&mut s, "panel.layers.editMask", json!({}));
    assert_eq!(s.active().and_then(|d| d.bitmap_mask_edit()), Some(m));
    assert_eq!(
        px(&s, 80, 80),
        inside,
        "paste inside: the content stays while editing"
    );
    assert_eq!(
        px(&s, 30, 30),
        under.clone(),
        "paste inside: the underlay stays while editing"
    );
    let _ = s.execute("modify.exitBitmapMode", json!({}));
    select(&mut s, &[]);
    assert_eq!(
        (px(&s, 80, 80), px(&s, 30, 30)),
        (inside, under),
        "paste inside: unchanged after the edit"
    );
    // and the whole page stays the same in every way of selecting the mask
    check("paste inside", s, g);

    // the same masks switched to the alpha channel (Modify Mask rows as in the report)
    for mode in ["path", "image", "path"] {
        let mut s = doc_session(200.0, 200.0);
        let c = bitmap(&mut s, 0, 0, 150, 150, [0, 0, 255, 255]);
        let m = bitmap(&mut s, 30, 30, 60, 60, [0, 0, 0, 255]);
        select(&mut s, &[c, m]);
        let g = id_of(&ex(&mut s, "modify.mask.groupAsMask", json!({})));
        select(&mut s, &[g]);
        ex(&mut s, "panel.layers.editMask", json!({}));
        ex(&mut s, "modify.mask.options", json!({"mode": mode}));
        let _ = s.execute("modify.exitBitmapMode", json!({}));
        check(&format!("group as mask, {mode}"), s, g);

        let mut s = doc_session(200.0, 200.0);
        let b = bitmap(&mut s, 20, 20, 120, 100, [255, 0, 0, 255]);
        select(&mut s, &[b]);
        let g = id_of(&ex(&mut s, "modify.mask.hideAll", json!({})));
        select(&mut s, &[g]);
        ex(&mut s, "panel.layers.editMask", json!({}));
        ex(&mut s, "modify.mask.options", json!({"mode": mode}));
        let _ = s.execute("modify.exitBitmapMode", json!({}));
        check(&format!("hide all, {mode}"), s, g);
    }
}
