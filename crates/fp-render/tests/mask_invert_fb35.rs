//! FB-35 / RE spec 68 section 6: the inverted mask (Firepixel extension). What lies inside the
//! mask disappears, everything outside stays; the mask object's own opacity scales the strength
//! and in grayscale mode the mask value (its luminance) does the same. A normal mask is unchanged.

use std::sync::Arc;

use fp_color::{Argb, Opacity, Rgba8};
use fp_geom::Rect;
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

/// Green 40 × 40 content masked by a 10..20 square (colour `mask_color`, opacity `mask_opacity`),
/// on a transparent page.
fn render(
    kind: GroupKind,
    invert: bool,
    mask_color: u32,
    mask_opacity: u8,
) -> fp_raster::RgbaImage {
    let mut doc = Document::new(40.0, 40.0, Argb(0x00ff_ffff));
    let content = rect(&mut doc, Rect::new(0.0, 0.0, 40.0, 40.0), 0xff00_ff00);
    let mut mask = rect(&mut doc, Rect::new(10.0, 10.0, 20.0, 20.0), mask_color);
    mask.common.opacity = Opacity(mask_opacity);
    let mut g =
        Element::new_mask_group(&mut doc.ids, Arc::new(mask), vec![Arc::new(content)], true);
    if let ElementKind::Group(gr) = &mut g.kind {
        gr.kind = kind;
        gr.mask_flags.invert = invert;
    }
    put(&mut doc, vec![g]);
    Renderer::new(opts()).render_page(&doc, 0, 0).image
}

#[test]
fn inverted_path_mask_hides_inside_and_shows_outside() {
    let normal = render(GroupKind::VectorMask, false, 0xffff_0000, 255);
    assert_eq!(normal.pixel(15, 15), Rgba8::rgb(0, 255, 0));
    assert_eq!(normal.pixel(30, 30).a, 0);
    let inv = render(GroupKind::VectorMask, true, 0xffff_0000, 255);
    assert_eq!(inv.pixel(15, 15).a, 0, "inside the mask is hidden");
    assert_eq!(inv.pixel(30, 30), Rgba8::rgb(0, 255, 0), "outside stays");
    assert_eq!(
        inv.pixel(2, 2),
        Rgba8::rgb(0, 255, 0),
        "outside the mask bounds stays"
    );
}

#[test]
fn mask_opacity_scales_the_inverted_strength() {
    let inv = render(GroupKind::VectorMask, true, 0xffff_0000, 128);
    let a = inv.pixel(15, 15).a;
    assert!((120..=135).contains(&a), "half strength inside: alpha {a}");
    assert_eq!(inv.pixel(30, 30).a, 255);
    // for a normal mask the own opacity keeps having no effect (RE spec 68 section 6)
    let normal = render(GroupKind::VectorMask, false, 0xffff_0000, 128);
    assert_eq!(normal.pixel(15, 15).a, 255);
}

#[test]
fn grayscale_value_scales_the_inverted_strength() {
    // grayscale mode: whatever value m the mask gives a pixel, the inverted mask gives 255 - m
    // (independent of how the value is derived from the mask's look)
    for color in [0xff80_8080u32, 0xffff_0000, 0xffff_ffff, 0xff00_0000] {
        let normal = render(GroupKind::BitmapMask, false, color, 255);
        let inv = render(GroupKind::BitmapMask, true, color, 255);
        let (n, i) = (normal.pixel(15, 15).a as i32, inv.pixel(15, 15).a as i32);
        assert!(
            (n + i - 255).abs() <= 1,
            "{color:08x}: normal {n} + inverted {i} = 255"
        );
        assert_eq!(inv.pixel(30, 30).a, 255);
    }
}
