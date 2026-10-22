//! FB-23 / TW-203: a model fill with the "Dither" edge (edge 3, Firepixel extension) renders as an
//! ordered-dither transition with no partial alpha; width 0 equals Hard.

use std::sync::Arc;

use fp_color::Argb;
use fp_geom::Rect;
use fp_model::{Document, Element, Fill, LayerSlot, PathAttrs};
use fp_render::{CachePolicy, RenderOptions, Renderer};

fn render(edge: i32, feather: i32) -> fp_raster::RgbaImage {
    let mut doc = Document::new(100.0, 80.0, Argb(0x00ff_ffff));
    let mut attrs = PathAttrs::solid_fill(Argb(0xff20_4060));
    if let Some(f) = attrs.fill.as_mut() {
        f.edge = edge;
        f.feather = feather;
    }
    let e = Element::new_rectangle(&mut doc.ids, Rect::new(20.3, 15.6, 75.7, 60.2), 0.0, attrs);
    let page = Arc::make_mut(&mut doc.pages[0]);
    if let LayerSlot::Own(l) = &mut page.root.layers[0] {
        Arc::make_mut(&mut Arc::make_mut(l).cels[0]).elements = vec![Arc::new(e)];
    }
    let opts = RenderOptions {
        cache_policy: CachePolicy::None,
        ..RenderOptions::default()
    };
    let out = Renderer::new(opts).render_page(&doc, 0, 0);
    assert!(out.report.is_complete(), "{:?}", out.report);
    out.image
}

fn alphas(img: &fp_raster::RgbaImage) -> Vec<u8> {
    let b = img.bounds();
    (b.top..b.bottom)
        .flat_map(|y| (b.left..b.right).map(move |x| (x, y)))
        .map(|(x, y)| img.pixel(x, y).a)
        .collect()
}

#[test]
fn model_dither_edge_renders_without_partial_alpha() {
    let d = alphas(&render(Fill::EDGE_DITHER, 8));
    assert!(d.iter().all(|&a| a == 0 || a == 255));
    let full = d.iter().filter(|&&a| a == 255).count();
    // the feather of the same width has partial alpha; the dither keeps about its coverage
    let f = alphas(&render(Fill::EDGE_FEATHER, 8));
    assert!(f.iter().any(|&a| a != 0 && a != 255));
    let mass: u32 = f.iter().map(|&a| a as u32).sum();
    let want = mass as f64 / 255.0;
    assert!(
        (full as f64 - want).abs() < want * 0.03,
        "dither coverage {full} vs feather {want}"
    );
}

#[test]
fn model_dither_width_zero_equals_hard() {
    assert_eq!(
        alphas(&render(Fill::EDGE_DITHER, 0)),
        alphas(&render(Fill::EDGE_HARD, 0))
    );
}
