//! FB-24 / FB-25 / TW-207 (RE spec 61 §1): a pattern or gradient fill moves with the object. A shape
//! rendered with its fill handles at `h`, and the same shape with shape and handles translated by
//! (37, 23), have identical pixels relative to the shape; the pattern tile pixel (0,0) sits on
//! handle 1 and there is no doubled column or row at handle 1.

use std::sync::Arc;

use fp_color::Argb;
use fp_geom::{Point, Rect};
use fp_model::element::BitmapNode;
use fp_model::{
    Document, Element, ElementKind, Fill, FillHandles, Gradient, GradientStop, GradientStops,
    LayerSlot, PathAttrs, Pattern, StopList,
};
use fp_render::{CachePolicy, RenderOptions, Renderer};

const TW: u32 = 7;
const TH: u32 = 5;

/// Tile colour: red = column, green = row (all pixels distinct).
fn texel(x: u32, y: u32) -> [u8; 3] {
    [(x * 30) as u8, (y * 40) as u8, 99]
}

fn pattern_attrs() -> PathAttrs {
    let bytes: Vec<u8> = (0..TH)
        .flat_map(|y| {
            (0..TW).flat_map(move |x| {
                let c = texel(x, y);
                [c[0], c[1], c[2], 255]
            })
        })
        .collect();
    let mut next = 1;
    let node = BitmapNode::from_rgba(Point::new(0.0, 0.0), TW, TH, &bytes, &mut next);
    let mut a = PathAttrs::solid_fill(Argb(0xff80_8080));
    if let Some(f) = a.fill.as_mut() {
        f.category = "fc_Pattern".into();
        f.name = "Test".into();
        f.shape = 15;
        f.shape_ext = 0;
        f.edge = Fill::EDGE_HARD;
        f.pattern = Some(Pattern {
            name: "Test".into(),
            user_name: None,
            image: Some(Arc::new(node)),
            extra: Default::default(),
        });
    }
    a
}

fn gradient_attrs() -> PathAttrs {
    let stops = |v: &[(f32, u32)]| StopList {
        stops: v
            .iter()
            .map(|&(p, c)| GradientStop {
                position: p,
                color: Argb(c),
                extra: Default::default(),
            })
            .collect(),
        extra: Default::default(),
    };
    let mut a = PathAttrs::solid_fill(Argb(0xff80_8080));
    if let Some(f) = a.fill.as_mut() {
        f.category = "fc_Linear".into();
        f.name = "Linear".into();
        f.shape = 2;
        f.shape_ext = 0;
        f.edge = Fill::EDGE_HARD;
        f.gradient = Some(Gradient {
            category: "fc_Linear".into(),
            name: "Linear".into(),
            user_name: None,
            legacy_stops: None,
            stops: Some(GradientStops {
                colors: stops(&[(0.0, 0xffff_0000), (1.0, 0xff00_00ff)]),
                opacities: stops(&[(0.0, 0xffff_ffff), (1.0, 0xffff_ffff)]),
                dither: None,
                dither_value: None,
                extra: Default::default(),
            }),
            extra: Default::default(),
        });
    }
    a
}

/// Renders a 100,100–260,220 rectangle moved by `d` with fill handles `h` (also moved by `d`).
fn render(attrs: PathAttrs, h: [(f32, f32); 3], d: (f32, f32)) -> fp_raster::RgbaImage {
    let mut doc = Document::new(400.0, 300.0, Argb(0x00ff_ffff));
    let r = Rect::new(100.0 + d.0, 100.0 + d.1, 260.0 + d.0, 220.0 + d.1);
    let mut e = Element::new_rectangle(&mut doc.ids, r, 0.0, attrs);
    let p = |i: usize| Point::new(h[i].0 + d.0, h[i].1 + d.1);
    let fh = FillHandles {
        valid: true,
        start: p(0),
        end: p(1),
        second: p(2),
    };
    if let ElementKind::Path(pe) = &mut e.kind {
        pe.fill_handles = fh;
        if let Some(pr) = &mut pe.primitive {
            pr.fill_handles = fh;
        }
    }
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

fn rgb(img: &fp_raster::RgbaImage, x: i32, y: i32) -> [u8; 4] {
    let p = img.pixel(x, y);
    [p.r, p.g, p.b, p.a]
}

/// Inside of the rectangle (3 px margin) before and after the move are pixel-identical.
fn assert_moves_with_shape(attrs: fn() -> PathAttrs, h: [(f32, f32); 3]) {
    let a = render(attrs(), h, (0.0, 0.0));
    let b = render(attrs(), h, (37.0, 23.0));
    for y in 103..217 {
        for x in 103..257 {
            assert_eq!(rgb(&a, x, y), rgb(&b, x + 37, y + 23), "({x},{y}) {h:?}");
        }
    }
}

#[test]
fn pattern_moves_with_the_shape_tile_sized_handles() {
    // FB-24: handles of a new pattern fill (centre, one tile right, one tile up)
    assert_moves_with_shape(
        pattern_attrs,
        [(180.0, 160.0), (187.0, 160.0), (180.0, 155.0)],
    );
}

#[test]
fn pattern_moves_with_the_shape_scaled_handles() {
    // FB-25: the old default handles (left middle, right middle, top left) stretched the pattern
    // and it slid by d·(1 − w/L) on every move
    assert_moves_with_shape(
        pattern_attrs,
        [(100.0, 160.0), (260.0, 160.0), (100.0, 100.0)],
    );
}

#[test]
fn pattern_tile_origin_on_handle_1_without_doubled_column() {
    let img = render(
        pattern_attrs(),
        [(180.0, 160.0), (187.0, 160.0), (180.0, 155.0)],
        (0.0, 0.0),
    );
    let px = |x: i32, y: i32| {
        let p = rgb(&img, x, y);
        [p[0], p[1], p[2]]
    };
    assert_eq!(px(180, 160), texel(0, 0));
    assert_eq!(px(181, 161), texel(1, 1));
    assert_eq!(px(186, 164), texel(6, 4));
    assert_eq!(px(187, 165), texel(0, 0));
    // left of and above handle 1: the neighbouring tile, no seam
    assert_eq!(px(179, 160), texel(6, 0));
    assert_eq!(px(173, 160), texel(0, 0));
    assert_eq!(px(180, 159), texel(0, 4));
    assert_eq!(px(180, 155), texel(0, 0));
    // the pattern is not fixed to the document: moving only the handles moves it
    let moved = render(
        pattern_attrs(),
        [(217.0, 183.0), (224.0, 183.0), (217.0, 178.0)],
        (0.0, 0.0),
    );
    let p = rgb(&moved, 217, 183);
    assert_eq!([p[0], p[1], p[2]], texel(0, 0));
}

#[test]
fn gradient_moves_with_the_shape() {
    assert_moves_with_shape(
        gradient_attrs,
        [(180.0, 100.0), (180.0, 220.0), (300.0, 100.0)],
    );
    assert_moves_with_shape(
        gradient_attrs,
        [(117.5, 130.25), (240.75, 201.5), (60.0, 250.0)],
    );
}
