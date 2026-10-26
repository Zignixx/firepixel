//! TW-273, RE spec 73 §0.1 to §0.3 and §7: the Layers thumbnails of a masked object. Object
//! thumbnail at rest: with "Show fill and stroke" on in the alpha channel mode the masked result
//! including the underlay, canvas-related on transparency; with the underlay off or in the
//! grayscale mode the content alone, unmasked. Mask thumbnail: the mask as grayscale on black,
//! canvas-related (alpha → white, grayscale → luma, an empty Reveal All mask all white). Entering
//! the mask edit changes neither thumbnail.

use fp_engine::Session;
use fp_model::Element;
use fp_raster::RgbaImage;
use fp_ui::panels::thumbs;
use serde_json::{Value, json};
use std::sync::Arc;

fn ex(s: &mut Session, c: &str, p: Value) -> Value {
    s.execute(c, p).unwrap_or_else(|e| panic!("{c}: {e}")).value
}

fn id_of(v: &Value) -> u64 {
    v["id"]
        .as_u64()
        .or_else(|| v["ids"][0].as_u64())
        .expect("id")
}

fn paste(s: &mut Session, r: fp_geom::IRect, c: [u8; 4]) -> u64 {
    let img = RgbaImage::filled(r, fp_color::Rgba8::new(c[0], c[1], c[2], c[3])).expect("image");
    s.clipboard.clear();
    s.clipboard.pixels = Some(img);
    ex(s, "edit.paste", json!({}))["ids"][0]
        .as_u64()
        .expect("bitmap")
}

fn new_doc() -> Session {
    let mut s = Session::default();
    ex(
        &mut s,
        "file.new",
        json!({"width": 200, "height": 100, "background": "#ffffff"}),
    );
    s
}

/// Page 200 × 100: the blue bitmap 20..140 × 20..80 pasted inside the red bitmap 100..180 ×
/// 20..80 (bitmap mask, alpha channel, "Show fill and stroke" on). Returns the masked group.
fn paste_inside() -> (Session, u64) {
    let mut s = new_doc();
    let target = paste(
        &mut s,
        fp_geom::IRect::new(100, 20, 180, 80),
        [255, 0, 0, 255],
    );
    let content = paste(
        &mut s,
        fp_geom::IRect::new(20, 20, 140, 80),
        [0, 0, 255, 255],
    );
    ex(&mut s, "select.set", json!({"ids": [content]}));
    ex(&mut s, "edit.cut", json!({}));
    ex(&mut s, "select.set", json!({"ids": [target]}));
    let g = id_of(&ex(&mut s, "edit.pasteInside", json!({})));
    ex(&mut s, "select.set", json!({"ids": []}));
    (s, g)
}

fn element(s: &Session, id: u64) -> Arc<Element> {
    let d = s.active().expect("doc");
    let mut found = None;
    fp_model::traverse::walk_root(
        &d.doc.pages[d.page].root,
        fp_model::traverse::RootRef::Page(d.page),
        &mut |_, _, e| {
            if e.id.0 == id && found.is_none() {
                found = Some(Arc::clone(e));
            }
        },
    );
    found.expect("element on the page")
}

/// (object thumbnail, mask thumbnail) images at 100 px (page 200 × 100 → 100 × 50, scale 0.5).
fn images(s: &Session, g: u64) -> (RgbaImage, RgbaImage) {
    let r = fp_render::Renderer::new(fp_render::RenderOptions::default());
    let d = s.active().expect("doc");
    let e = element(s, g);
    let m = Arc::clone(e.element_mask().expect("mask").element);
    let obj = thumbs::masked_object_image(&r, &d.doc, d.page, &e, d.frame, 100).expect("object");
    let mask = thumbs::mask_image(&r, &d.doc, d.page, &e, &m, d.frame, 100).expect("mask");
    (obj, mask)
}

/// Pixel at document point (x, y) of a 0.5-scaled thumbnail.
fn at(img: &RgbaImage, x: i32, y: i32) -> [u8; 4] {
    img.get(x / 2, y / 2).unwrap_or([0; 4])
}

const BLUE: [u8; 4] = [0, 0, 255, 255];
const RED: [u8; 4] = [255, 0, 0, 255];
const WHITE: [u8; 4] = [255, 255, 255, 255];
const BLACK: [u8; 4] = [0, 0, 0, 255];

#[test]
fn rest_with_underlay_is_the_masked_result_canvas_related() {
    let (s, g) = paste_inside();
    let (obj, mask) = images(&s, g);
    assert_eq!((obj.width(), obj.height()), (100, 50), "page format");
    assert_eq!(at(&obj, 120, 50), BLUE, "content inside the mask");
    assert_eq!(
        at(&obj, 160, 50),
        RED,
        "underlay where the content has no pixels"
    );
    assert_eq!(at(&obj, 50, 50)[3], 0, "content outside the mask is masked");
    assert_eq!(at(&obj, 190, 90)[3], 0, "transparent around it");
    assert_eq!((mask.width(), mask.height()), (100, 50));
    assert_eq!(at(&mask, 120, 50), WHITE, "alpha of the mask → white");
    assert_eq!(at(&mask, 160, 50), WHITE);
    assert_eq!(at(&mask, 50, 50), BLACK, "outside the mask: black");
    assert_eq!(at(&mask, 10, 10), BLACK);
}

#[test]
fn underlay_off_or_grayscale_shows_the_content_alone_unmasked() {
    for opts in [json!({"showAttrs": false}), json!({"mode": "image"})] {
        let (mut s, g) = paste_inside();
        let mut p = opts.clone();
        p["ids"] = json!([g]);
        ex(&mut s, "modify.mask.options", p);
        let (obj, _) = images(&s, g);
        assert_eq!(at(&obj, 50, 50), BLUE, "{opts}: the content unmasked");
        assert_eq!(at(&obj, 120, 50), BLUE, "{opts}");
        assert_eq!(at(&obj, 160, 50)[3], 0, "{opts}: no underlay");
    }
}

#[test]
fn mask_thumbnail_per_mode() {
    // grayscale: the luma of the red mask bitmap
    let (mut s, g) = paste_inside();
    ex(
        &mut s,
        "modify.mask.options",
        json!({"ids": [g], "mode": "image"}),
    );
    let (_, mask) = images(&s, g);
    let l = fp_color::models::luma709(255, 0, 0);
    assert_eq!(at(&mask, 160, 50), [l, l, l, 255], "grayscale: luma");
    assert_eq!(at(&mask, 50, 50), BLACK);
    // Reveal All: an empty grayscale mask is all white
    let mut s = new_doc();
    let b = paste(
        &mut s,
        fp_geom::IRect::new(40, 30, 120, 70),
        [0, 128, 0, 255],
    );
    ex(&mut s, "select.set", json!({"ids": [b]}));
    let g = id_of(&ex(&mut s, "modify.mask.revealAll", json!({})));
    let (obj, mask) = images(&s, g);
    for (x, y) in [(2, 2), (100, 50), (196, 96)] {
        assert_eq!(at(&mask, x, y), WHITE, "Reveal All: white at {x},{y}");
    }
    assert_eq!(at(&obj, 80, 50), [0, 128, 0, 255], "content");
    assert_eq!(
        at(&obj, 10, 10)[3],
        0,
        "canvas-related: transparent around it"
    );
}

#[test]
fn entering_the_mask_edit_changes_no_thumbnail() {
    let (mut s, g) = paste_inside();
    let before = images(&s, g);
    ex(&mut s, "select.set", json!({"ids": [g]}));
    ex(&mut s, "panel.layers.editMask", json!({}));
    assert!(
        s.active().is_some_and(|d| d.bitmap_mask_edit().is_some()),
        "bitmap editing of the mask"
    );
    let during = images(&s, g);
    assert!(before.0 == during.0, "object thumbnail unchanged");
    assert!(before.1 == during.1, "mask thumbnail unchanged");
}

#[test]
fn vector_mask_with_and_without_underlay() {
    let mut s = new_doc();
    let b = paste(
        &mut s,
        fp_geom::IRect::new(20, 20, 140, 80),
        [0, 0, 255, 255],
    );
    let m = id_of(&ex(
        &mut s,
        "draw.path",
        json!({"contours": [{"closed": true, "nodes": [[100, 20], [180, 20], [180, 80], [100, 80]]}]}),
    ));
    ex(&mut s, "select.set", json!({"ids": [m, b]}));
    let g = id_of(&ex(&mut s, "modify.mask.groupAsMask", json!({})));
    ex(
        &mut s,
        "modify.mask.options",
        json!({"ids": [g], "mode": "path", "showAttrs": true}),
    );
    let e = element(&s, g);
    assert!(
        Arc::ptr_eq(&thumbs::thumb_subject(&e), &e),
        "underlay on: the masked result"
    );
    ex(
        &mut s,
        "modify.mask.options",
        json!({"ids": [g], "showAttrs": false}),
    );
    let e = element(&s, g);
    assert!(
        !Arc::ptr_eq(&thumbs::thumb_subject(&e), &e),
        "underlay off: the content alone"
    );
    let (obj, mask) = images(&s, g);
    assert_eq!(at(&obj, 50, 50), BLUE, "unmasked content");
    assert_eq!(at(&mask, 140, 50), WHITE, "the path shape white");
    assert_eq!(at(&mask, 50, 50), BLACK);
}
