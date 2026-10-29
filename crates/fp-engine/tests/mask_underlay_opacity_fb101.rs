//! FB-101 (TW-273, *RE spec* 68 §4.2 rules 2 and 5, *RE spec* 73), updated by TW-277 (*RE spec* 75
//! §2.4, FB-126): with a Paste-Inside bitmap mask (Alpha channel, "Show fill and stroke" on)
//! selected, the Layers panel's *Opacity* (`element.setOpacity` on the selection) acts on the
//! masked element, never on the mask object; the masked element's opacity applies to the whole
//! masked result including the underlay (*RE spec* 26 D2, *RE spec* 75 §2.5). The measured way to
//! remove the underlay is the "Show fill and stroke" option
//! (`modify.mask.options {showAttrs}`, script `setElementMaskShowAttrs`), which the reference's
//! bitmap mask inspector does not offer (*RE spec* 68 §4.2 rule 2: only the mode radios).

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

fn page(s: &Session, r: &fp_render::Renderer) -> fp_raster::RgbaImage {
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
}

fn px(img: &fp_raster::RgbaImage, x: i32, y: i32) -> [u8; 4] {
    let b = img.as_bytes();
    let i = ((y * 200 + x) * 4) as usize;
    [b[i], b[i + 1], b[i + 2], b[i + 3]]
}

/// Red mask bitmap 30..70 × 0..100, blue content 0..50 × 0..100 pasted inside: x 30..50 shows the
/// content, x 50..70 the red underlay, x < 30 is masked out. Returns (session, group, mask).
fn setup() -> (Session, ElementId, ElementId) {
    fp_i18n::set_language("en");
    let mut s = doc_session(200.0, 200.0);
    let target = bitmap(&mut s, 30, 0, 40, 100, [255, 0, 0, 255]);
    let content = bitmap(&mut s, 0, 0, 50, 100, [0, 0, 255, 255]);
    select(&mut s, &[content]);
    ex(&mut s, "edit.cut", json!({}));
    select(&mut s, &[target]);
    let g = id_of(&ex(&mut s, "edit.pasteInside", json!({})));
    let m = el(&s, g)
        .element_mask()
        .map(|m| m.element.id)
        .expect("mask");
    (s, g, m)
}

fn is(p: [u8; 4], rgb: [u8; 3]) -> bool {
    p[0].abs_diff(rgb[0]) <= 2 && p[1].abs_diff(rgb[1]) <= 2 && p[2].abs_diff(rgb[2]) <= 2
}

#[test]
fn mask_opacity_fades_neither_the_underlay_nor_the_masking() {
    let (mut s, g, m) = setup();
    let fresh = |s: &Session| {
        page(
            s,
            &fp_render::Renderer::new(fp_render::RenderOptions::default()),
        )
    };
    let before = fresh(&s);
    assert!(
        is(px(&before, 40, 50), [0, 0, 255]),
        "content inside the mask"
    );
    assert!(is(px(&before, 60, 50), [255, 0, 0]), "red underlay");
    assert!(is(px(&before, 10, 50), [255, 255, 255]), "masked out");
    // the mask selected like a thumbnail click; the Layers panel's Opacity runs this command
    select(&mut s, &[g]);
    ex(&mut s, "panel.layers.editMask", json!({}));
    assert_eq!(
        s.active().map(|d| d.selection.elements.clone()),
        Some(vec![m])
    );
    for v in [50, 0] {
        ex(&mut s, "element.setOpacity", json!({"opacity": v}));
        assert_eq!(
            el(&s, m).common.opacity.0,
            255,
            "{v}: the mask object's own opacity is not touched"
        );
        assert_eq!(
            el(&s, g).placement().0.to_percent().round(),
            v as f64,
            "{v}: the masked element's opacity"
        );
        let p = fresh(&s);
        if v == 0 {
            assert!(is(px(&p, 60, 50), [255, 255, 255]), "0: underlay gone too");
            assert!(is(px(&p, 40, 50), [255, 255, 255]), "0: content gone");
        } else {
            assert!(is(px(&p, 60, 50), [255, 127, 127]), "50: the underlay fades");
            assert!(is(px(&p, 40, 50), [127, 127, 255]), "50: the content fades");
        }
        assert!(is(px(&p, 10, 50), [255, 255, 255]), "{v}: still masked out");
    }
}

#[test]
fn masked_object_opacity_fades_the_whole_masked_result() {
    let (mut s, g, _) = setup();
    let fresh = |s: &Session| {
        page(
            s,
            &fp_render::Renderer::new(fp_render::RenderOptions::default()),
        )
    };
    select(&mut s, &[g]);
    ex(&mut s, "element.setOpacity", json!({"opacity": 0}));
    let p = fresh(&s);
    assert!(is(px(&p, 40, 50), [255, 255, 255]), "content gone");
    assert!(is(px(&p, 60, 50), [255, 255, 255]), "the underlay with it");
}

#[test]
fn group_as_mask_then_alpha_channel_has_no_underlay() {
    fp_i18n::set_language("en");
    let mut s = doc_session(200.0, 200.0);
    let content = bitmap(&mut s, 0, 0, 50, 100, [0, 0, 255, 255]);
    let mask = bitmap(&mut s, 30, 0, 40, 100, [255, 0, 0, 255]);
    select(&mut s, &[mask, content]);
    ex(&mut s, "modify.mask.groupAsMask", json!({}));
    let g = s.active().expect("doc").selection.elements[0];
    let g = fp_engine::docops::mask_owner(&s.active().unwrap().doc, 0, g)
        .map(|o| o.id)
        .unwrap_or(g);
    select(&mut s, &[g]);
    ex(&mut s, "modify.mask.options", json!({"mode": "path"}));
    let p = page(
        &s,
        &fp_render::Renderer::new(fp_render::RenderOptions::default()),
    );
    assert!(
        is(px(&p, 40, 50), [0, 0, 255]),
        "content inside the mask (alpha)"
    );
    assert!(is(px(&p, 60, 50), [255, 255, 255]), "no underlay");
    assert!(is(px(&p, 10, 50), [255, 255, 255]), "masked out");
}

#[test]
fn show_fill_and_stroke_off_removes_the_underlay_and_keeps_the_mask() {
    let (mut s, g, _) = setup();
    let r = fp_render::Renderer::new(fp_render::RenderOptions::default());
    select(&mut s, &[g]);
    ex(&mut s, "modify.mask.options", json!({"showAttrs": false}));
    let p = page(&s, &r);
    assert!(is(px(&p, 60, 50), [255, 255, 255]), "no underlay");
    assert!(is(px(&p, 40, 50), [0, 0, 255]), "content inside the mask");
    assert!(is(px(&p, 10, 50), [255, 255, 255]), "masked out");
}

#[test]
fn paste_as_mask_then_alpha_channel_has_no_underlay() {
    fp_i18n::set_language("en");
    let mut s = doc_session(200.0, 200.0);
    let content = bitmap(&mut s, 0, 0, 50, 100, [0, 0, 255, 255]);
    let mask = bitmap(&mut s, 30, 0, 40, 100, [255, 0, 0, 255]);
    select(&mut s, &[mask]);
    ex(&mut s, "edit.cut", json!({}));
    select(&mut s, &[content]);
    ex(&mut s, "edit.pasteAsMask", json!({}));
    ex(&mut s, "modify.mask.options", json!({"mode": "path"}));
    let p = page(
        &s,
        &fp_render::Renderer::new(fp_render::RenderOptions::default()),
    );
    assert!(
        is(px(&p, 40, 50), [0, 0, 255]),
        "content inside the mask (alpha)"
    );
    assert!(is(px(&p, 60, 50), [255, 255, 255]), "no underlay");
    assert!(is(px(&p, 10, 50), [255, 255, 255]), "masked out");
}
