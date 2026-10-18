//! BA-039: the live paint preview replaces the pixels of the painted bitmap and lets the normal
//! compositor draw the page, so objects above cover the stroke and the bitmap's opacity applies.

use std::sync::Arc;

use fp_color::{Argb, Opacity, Rgba8};
use fp_geom::{IRect, Point, Rect};
use fp_model::{Document, Element, ElementId, LayerSlot, PathAttrs};
use fp_raster::RgbaImage;
use fp_render::{PixelOverride, Renderer};

/// White page 200×100; a white bitmap 0..200 × 0..100 (below) and a green rectangle 100..200
/// (above). Returns the document and the bitmap's id.
fn page(bitmap_opacity: u8) -> (Document, ElementId) {
    let mut doc = Document::new(200.0, 100.0, Argb(0xffff_ffff));
    let mut tiles = 1;
    let rgba = vec![255u8; 200 * 100 * 4];
    let mut b = Element::new_bitmap(
        &mut doc.ids,
        Point::new(0.0, 0.0),
        200,
        100,
        &rgba,
        &mut tiles,
    );
    b.common.opacity = Opacity(bitmap_opacity);
    let id = b.id;
    let green = Element::new_rectangle(
        &mut doc.ids,
        Rect::new(100.0, 0.0, 200.0, 100.0),
        0.0,
        PathAttrs::solid_fill(Argb(0xff00_ff00)),
    );
    let p = Arc::make_mut(&mut doc.pages[0]);
    if let LayerSlot::Own(l) = &mut p.root.layers[0] {
        // stored top first
        Arc::make_mut(&mut Arc::make_mut(l).cels[0]).elements = vec![Arc::new(green), Arc::new(b)];
    }
    (doc, id)
}

/// The bitmap with a black horizontal stroke y 40..60 across the whole width.
fn stroked() -> Arc<RgbaImage> {
    let mut img = RgbaImage::filled(IRect::new(0, 0, 200, 100), Rgba8::new(255, 255, 255, 255))
        .expect("image");
    for y in 40..60 {
        for x in 0..200 {
            img.set_pixel(x, y, Rgba8::new(0, 0, 0, 255));
        }
    }
    Arc::new(img)
}

fn px(img: &RgbaImage, x: i32, y: i32) -> [u8; 4] {
    img.get(x, y).expect("pixel")
}

#[test]
fn stroke_under_covering_rectangle_is_hidden_ba039() {
    let (doc, id) = page(255);
    let r = Renderer::default();
    let ov = PixelOverride {
        id,
        image: stroked(),
    };
    let out = r
        .render_region_with(&doc, 0, 0, IRect::new(0, 0, 200, 100), 1.0, &ov)
        .expect("bitmap on the page");
    assert_eq!(
        px(&out.image, 50, 50),
        [0, 0, 0, 255],
        "stroke visible where uncovered"
    );
    assert_eq!(
        px(&out.image, 150, 50),
        [0, 255, 0, 255],
        "stroke hidden under the rectangle"
    );
    assert_eq!(
        px(&out.image, 50, 10),
        [255, 255, 255, 255],
        "unchanged pixels stay"
    );
    // the document itself is untouched, and a later plain render shows no stroke (no stale cache)
    let plain = r.render_region(&doc, 0, 0, IRect::new(0, 0, 200, 100), 1.0);
    assert_eq!(px(&plain.image, 50, 50), [255, 255, 255, 255]);
}

#[test]
fn bitmap_opacity_applies_to_the_stroke_ba039() {
    let (doc, id) = page(128);
    let r = Renderer::default();
    // a cropped override (only the rendered part) is enough without effects
    let full = stroked();
    let part = Arc::new(full.cropped(IRect::new(20, 30, 80, 70)));
    let ov = PixelOverride { id, image: part };
    let out = r
        .render_region_with(&doc, 0, 0, IRect::new(20, 30, 80, 70), 1.0, &ov)
        .expect("bitmap on the page");
    let p = out.image.get(30, 20).expect("pixel");
    assert!(
        (120..=136).contains(&p[0]) && p[0] == p[1] && p[3] == 255,
        "50 % black over white, was {p:?}"
    );
    let white = out.image.get(30, 2).expect("pixel");
    assert_eq!(white, [255, 255, 255, 255]);
}

#[test]
fn unknown_bitmap_returns_none_ba039() {
    let (doc, _) = page(255);
    let r = Renderer::default();
    let ov = PixelOverride {
        id: ElementId(999_999),
        image: stroked(),
    };
    assert!(
        r.render_region_with(&doc, 0, 0, IRect::new(0, 0, 10, 10), 1.0, &ov)
            .is_none()
    );
}
