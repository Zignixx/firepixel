//! FB-151 (TW-298, Firepixel extension): the mask mode "Pixel Alpha". The mask value is the alpha
//! of the mask object's rendered look (a path's colour alpha counts, unlike path outline), and the
//! mask object is never drawn under the content, also with "Show fill and stroke" on. A bitmap
//! mask in "Pixel Alpha" masks like Alpha channel, without its coloured underlay.

use std::sync::Arc;

use fp_color::{Argb, Rgba8};
use fp_geom::{Point, Rect};
use fp_model::{Document, Element, ElementKind, GroupKind, LayerSlot, PathAttrs};
use fp_render::{CachePolicy, RenderOptions, Renderer};

fn opts() -> RenderOptions {
    RenderOptions {
        cache_policy: CachePolicy::None,
        ..RenderOptions::default()
    }
}

fn rect(doc: &mut Document, r: Rect, color: u32) -> Element {
    Element::new_rectangle(&mut doc.ids, r, 0.0, PathAttrs::solid_fill(Argb(color)))
}

fn put(doc: &mut Document, elems: Vec<Element>) {
    let page = Arc::make_mut(&mut doc.pages[0]);
    if let LayerSlot::Own(l) = &mut page.root.layers[0] {
        let cel = Arc::make_mut(&mut Arc::make_mut(l).cels[0]);
        cel.elements = elems.into_iter().map(Arc::new).collect();
    }
}

/// Green content 0..15 × 0..40 masked by `mask` (path outline group, "Show fill and stroke"
/// `show`, "Pixel Alpha" `pixel`) on a transparent 40 × 40 page.
fn render(doc: &mut Document, mask: Element, show: bool, pixel: bool) -> fp_raster::RgbaImage {
    let content = rect(doc, Rect::new(0.0, 0.0, 15.0, 40.0), 0xff00_ff00);
    let mut g =
        Element::new_mask_group(&mut doc.ids, Arc::new(mask), vec![Arc::new(content)], true);
    if let ElementKind::Group(gr) = &mut g.kind {
        gr.kind = GroupKind::VectorMask;
        gr.mask_flags.hide_attrs = !show;
        gr.mask_flags.pixel_alpha = pixel;
        assert_eq!(gr.mask_underlay(), show && !pixel);
    }
    put(doc, vec![g]);
    Renderer::new(opts()).render_page(doc, 0, 0).image
}

#[test]
fn path_mask_colour_alpha_counts_and_no_underlay() {
    // red mask 10..20 with 25 % colour alpha
    let run = |show: bool, pixel: bool| {
        let mut doc = Document::new(40.0, 40.0, Argb(0x00ff_ffff));
        let m = rect(&mut doc, Rect::new(10.0, 10.0, 20.0, 20.0), 0x40ff_0000);
        render(&mut doc, m, show, pixel)
    };
    // path outline: the geometry, colour alpha does not count
    let outline = run(false, false);
    assert_eq!(outline.pixel(12, 15), Rgba8::rgb(0, 255, 0));
    // Pixel Alpha: the content keeps 25 %
    let pixel = run(false, true);
    let a = pixel.pixel(12, 15).a;
    assert!((60..=68).contains(&a), "alpha of the mask's look: {a}");
    assert_eq!(pixel.pixel(5, 5).a, 0, "outside the mask nothing");
    // with "Show fill and stroke" on: path outline draws the underlay right of the content,
    // Pixel Alpha never does
    let under = run(true, false);
    assert!(under.pixel(17, 15).a > 0, "path outline: underlay visible");
    let pixel_show = run(true, true);
    assert_eq!(pixel_show.pixel(17, 15).a, 0, "Pixel Alpha: no underlay");
    assert_eq!(
        pixel_show.pixel(12, 15),
        pixel.pixel(12, 15),
        "same mask value"
    );
}

#[test]
fn bitmap_mask_pixel_alpha_is_alpha_channel_without_the_underlay() {
    let run = |show: bool, pixel: bool| {
        let mut doc = Document::new(40.0, 40.0, Argb(0x00ff_ffff));
        let mut tiles = 1;
        // blue mask bitmap 10..30 × 10..30: left half alpha 255, right half alpha 128
        let px: Vec<u8> = (0..20 * 20)
            .flat_map(|i| {
                if i % 20 < 10 {
                    [0, 0, 255, 255]
                } else {
                    [0, 0, 255, 128]
                }
            })
            .collect();
        let m = Element::new_bitmap(
            &mut doc.ids,
            Point::new(10.0, 10.0),
            20,
            20,
            &px,
            &mut tiles,
        );
        // content over the whole page this time
        let content = rect(&mut doc, Rect::new(0.0, 0.0, 40.0, 40.0), 0xff00_ff00);
        let mut g =
            Element::new_mask_group(&mut doc.ids, Arc::new(m), vec![Arc::new(content)], true);
        if let ElementKind::Group(gr) = &mut g.kind {
            gr.mask_flags.hide_attrs = !show;
            gr.mask_flags.pixel_alpha = pixel;
        }
        put(&mut doc, vec![g]);
        Renderer::new(opts()).render_page(&doc, 0, 0).image
    };
    let alpha = run(false, false);
    let pixel = run(true, true);
    for (x, y) in [(12, 15), (25, 15), (5, 5)] {
        assert_eq!(
            alpha.pixel(x, y),
            pixel.pixel(x, y),
            "({x},{y}) like alpha channel"
        );
    }
    assert_eq!(pixel.pixel(12, 15), Rgba8::rgb(0, 255, 0));
    let a = pixel.pixel(25, 15).a;
    assert!((126..=130).contains(&a), "half alpha: {a}");
    // the alpha channel underlay tints the half-transparent part blue, Pixel Alpha does not
    let under = run(true, false);
    assert!(under.pixel(25, 15).b > 0, "{:?}", under.pixel(25, 15));
    assert_eq!(pixel.pixel(25, 15).b, 0);
}
