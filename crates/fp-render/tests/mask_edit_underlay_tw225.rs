//! TW-225 rest, RE spec 68 §4.2 rule 5, §4.1 `e9`/`e10`, §7: a bitmap mask with "Show fill and
//! stroke" (Paste Inside onto a bitmap, alpha channel) lies under the content, but not while the
//! mask is in bitmap editing ([`Renderer::render_device_editing`]), with or without a stroke;
//! after the edit the underlay is back. With the option off there is no underlay in any state.
//! Path masks are not affected. The caches of one renderer never mix the two looks.

use std::sync::Arc;

use fp_color::Argb;
use fp_geom::{IRect, Point, Rect};
use fp_model::{Document, Element, ElementId, ElementKind, LayerSlot, PathAttrs};
use fp_raster::RgbaImage;
use fp_render::{PixelOverride, RenderOptions, Renderer};

/// Transparent page 100 × 100: a white rectangle 0..50 masked by `mask` in path-outline/alpha
/// mode; `show` = "Show fill and stroke". Returns the document and the mask's id.
fn page(mut doc: Document, mask: Element, show: bool) -> (Document, ElementId) {
    let content = Element::new_rectangle(
        &mut doc.ids,
        Rect::new(0.0, 0.0, 50.0, 100.0),
        0.0,
        PathAttrs::solid_fill(Argb(0xffff_ffff)),
    );
    let mid = mask.id;
    let mut g =
        Element::new_mask_group(&mut doc.ids, Arc::new(mask), vec![Arc::new(content)], true);
    if let ElementKind::Group(gr) = &mut g.kind {
        gr.mask_flags.hide_attrs = !show;
    }
    let p = Arc::make_mut(&mut doc.pages[0]);
    if let LayerSlot::Own(l) = &mut p.root.layers[0] {
        Arc::make_mut(&mut Arc::make_mut(l).cels[0]).elements = vec![Arc::new(g)];
    }
    (doc, mid)
}

/// Red bitmap mask over x 30..70.
fn bitmap_doc(show: bool) -> (Document, ElementId) {
    let mut doc = Document::new(100.0, 100.0, Argb(0x00ff_ffff));
    let mut tiles = 1;
    let px: Vec<u8> = (0..100 * 100)
        .flat_map(|i| {
            if (30..70).contains(&(i % 100)) {
                [255, 0, 0, 255]
            } else {
                [0, 0, 0, 0]
            }
        })
        .collect();
    let m = Element::new_bitmap(
        &mut doc.ids,
        Point::new(0.0, 0.0),
        100,
        100,
        &px,
        &mut tiles,
    );
    page(doc, m, show)
}

fn alpha(img: &RgbaImage, x: i32, y: i32) -> u8 {
    img.get(x, y).map_or(0, |p| p[3])
}

fn device(r: &Renderer, doc: &Document, edit: Option<ElementId>) -> RgbaImage {
    let pr = IRect::new(0, 0, 100, 100);
    r.render_device_editing(doc, 0, 0, pr, (0, 0), 1.0, pr, edit)
        .image
}

/// The mask pixels with one more red column 5..15 (a stroke in progress).
fn stroke() -> Arc<RgbaImage> {
    let mut img = RgbaImage::new(IRect::new(0, 0, 100, 100)).expect("image");
    for y in 0..100 {
        for x in (5..15).chain(30..70) {
            img.set_pixel(x, y, fp_color::Rgba8::new(255, 0, 0, 255));
        }
    }
    Arc::new(img)
}

#[test]
fn bitmap_mask_underlay_hidden_for_the_whole_edit() {
    // default options: element cache and tile cache on, one renderer for all states
    let r = Renderer::new(RenderOptions::default());
    let (doc, mid) = bitmap_doc(true);
    let red = Some([255, 0, 0, 255]);
    let white = Some([255, 255, 255, 255]);
    let before = device(&r, &doc, None);
    assert_eq!(
        before.get(60, 50),
        red,
        "underlay where the content has no pixels"
    );
    assert_eq!(before.get(40, 50), white);
    let editing = device(&r, &doc, Some(mid));
    assert_eq!(
        alpha(&editing, 60, 50),
        0,
        "no underlay while the mask is edited"
    );
    assert_eq!(
        editing.get(40, 50),
        white,
        "the content stays masked by the bitmap"
    );
    assert_eq!(alpha(&editing, 10, 50), 0, "outside the mask");
    let ov = PixelOverride {
        id: mid,
        image: stroke(),
    };
    let during = r
        .render_region_with(&doc, 0, 0, IRect::new(0, 0, 100, 100), 1.0, &ov)
        .expect("mask bitmap found")
        .image;
    assert_eq!(alpha(&during, 60, 50), 0, "no underlay during a stroke");
    assert_eq!(during.get(10, 50), white, "the stroke reveals live");
    let after = device(&r, &doc, None);
    assert_eq!(
        after.get(60, 50),
        red,
        "after the edit the underlay is back"
    );
    assert_eq!(after, before, "the look without editing is unchanged");
    assert_eq!(
        device(&r, &doc, Some(mid)),
        editing,
        "editing again: same look"
    );
    assert_eq!(
        r.render_page(&doc, 0, 0).image.get(60, 50),
        red,
        "page renders (export) never edit"
    );
}

#[test]
fn option_off_shows_no_underlay_in_any_state() {
    let r = Renderer::new(RenderOptions::default());
    let (doc, mid) = bitmap_doc(false);
    let white = Some([255, 255, 255, 255]);
    for edit in [None, Some(mid)] {
        let img = device(&r, &doc, edit);
        assert_eq!(alpha(&img, 60, 50), 0, "{edit:?}");
        assert_eq!(img.get(40, 50), white, "{edit:?}");
    }
    let ov = PixelOverride {
        id: mid,
        image: stroke(),
    };
    let during = r
        .render_region_with(&doc, 0, 0, IRect::new(0, 0, 100, 100), 1.0, &ov)
        .expect("mask bitmap found")
        .image;
    assert_eq!(alpha(&during, 60, 50), 0);
}

#[test]
fn path_masks_keep_their_underlay() {
    let r = Renderer::new(RenderOptions::default());
    let mut doc = Document::new(100.0, 100.0, Argb(0x00ff_ffff));
    let m = Element::new_rectangle(
        &mut doc.ids,
        Rect::new(30.0, 0.0, 70.0, 100.0),
        0.0,
        PathAttrs::solid_fill(Argb(0xffff_0000)),
    );
    let (doc, mid) = page(doc, m, true);
    let red = Some([255, 0, 0, 255]);
    assert_eq!(device(&r, &doc, None).get(60, 50), red);
    assert_eq!(
        device(&r, &doc, Some(mid)).get(60, 50),
        red,
        "a path mask id changes nothing"
    );
}
