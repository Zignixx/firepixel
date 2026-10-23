//! Q-64 (RE spec 62 §2): a text with a gradient or pattern fill is painted like a path with the same
//! fill and the text's fill handles (document coordinates); the glyph coverage masks that paint.
//! Linear gradient white → black from h1 to h2: grey = 255·(1 − (y − y_h1)/(y_h2 − y_h1)) per row;
//! pattern: tile pixel (0,0) exactly on h1, x along h1→h2, rows opposite to h1→h3. Moving the text
//! together with its handles keeps the image pixel-identical; a solid fill is unchanged.

use std::sync::Arc;

use fp_color::Argb;
use fp_geom::Point;
use fp_model::element::BitmapNode;
use fp_model::{
    Document, Element, ElementKind, Fill, FillHandles, Gradient, GradientStop, GradientStops,
    LayerSlot, PathAttrs, Pattern, StopList,
};
use fp_raster::RgbaImage;
use fp_render::{CachePolicy, RenderOptions, Renderer};

const W: i32 = 320;
const H: i32 = 200;
const TW: u32 = 7;
const TH: u32 = 5;

/// Tile colour: red = column, green = row (all pixels distinct).
fn texel(x: u32, y: u32) -> [u8; 3] {
    [(x * 30) as u8, (y * 40) as u8, 99]
}

fn pattern_attrs(edge: i32) -> PathAttrs {
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
    let mut a = PathAttrs::solid_fill(Argb(0xff00_0000));
    if let Some(f) = a.fill.as_mut() {
        f.category = "fc_Pattern".into();
        f.name = "Test".into();
        f.shape = 15;
        f.shape_ext = 0;
        f.edge = edge;
        f.pattern = Some(Pattern {
            name: "Test".into(),
            user_name: None,
            image: Some(Arc::new(node)),
            extra: Default::default(),
        });
    }
    a
}

/// Linear gradient white at 0 → black at 1 (the reference's default after a black text).
fn gradient_attrs(edge: i32) -> PathAttrs {
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
    let mut a = PathAttrs::solid_fill(Argb(0xff00_0000));
    if let Some(f) = a.fill.as_mut() {
        f.category = "fc_Linear".into();
        f.name = "cn_BlackWhite".into();
        f.shape = 2;
        f.shape_ext = 0;
        f.edge = edge;
        f.gradient = Some(Gradient {
            category: "fc_Linear".into(),
            name: "cn_BlackWhite".into(),
            user_name: None,
            legacy_stops: None,
            stops: Some(GradientStops {
                colors: stops(&[(0.0, 0xffff_ffff), (1.0, 0xff00_0000)]),
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

/// Renders "Ag" (ArialMT 60 px, baseline at 40/110) moved by `d`, with `attrs` and the fill
/// handles `h` (also moved by `d`); `None` when the system has no usable font.
fn render(attrs: PathAttrs, h: [(f32, f32); 3], d: (i32, i32)) -> Option<RgbaImage> {
    let (dx, dy) = (d.0 as f32, d.1 as f32);
    let mut doc = Document::new(W as f32, H as f32, Argb(0x00ff_ffff));
    let mut e = Element::new_text(
        &mut doc.ids,
        "Ag",
        "ArialMT",
        60.0,
        Argb(0xff20_4060),
        Point::new(40.0 + dx, 110.0 + dy),
    );
    if let ElementKind::Text(t) = &mut e.kind {
        t.attrs = Arc::new(attrs);
        let p = |i: usize| Point::new(h[i].0 + dx, h[i].1 + dy);
        t.fill_handles = FillHandles {
            valid: true,
            start: p(0),
            end: p(1),
            second: p(2),
        };
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
    let img = out.image;
    let inked =
        (0..H).any(|y| (0..W).any(|x| img.bounds().contains(x, y) && img.pixel(x, y).a != 0));
    inked.then_some(img)
}

fn px(img: &RgbaImage, x: i32, y: i32) -> [u8; 4] {
    if !img.bounds().contains(x, y) {
        return [0; 4];
    }
    let p = img.pixel(x, y);
    [p.r, p.g, p.b, p.a]
}

/// Fully covered glyph pixels.
fn opaque(img: &RgbaImage) -> Vec<(i32, i32)> {
    (0..H)
        .flat_map(|y| (0..W).map(move |x| (x, y)))
        .filter(|&(x, y)| px(img, x, y)[3] == 255)
        .collect()
}

const GRAD_H: [(f32, f32); 3] = [(75.0, 40.0), (75.0, 130.0), (165.0, 40.0)];

#[test]
fn text_linear_gradient_ramps_from_h1_to_h2() {
    let Some(img) = render(gradient_attrs(Fill::EDGE_ANTIALIAS), GRAD_H, (0, 0)) else {
        eprintln!("skipped: no usable system font");
        return;
    };
    let ink = opaque(&img);
    assert!(ink.len() > 300, "{} opaque pixels", ink.len());
    let (y1, y2) = (GRAD_H[0].1 as f64, GRAD_H[1].1 as f64);
    let rows: std::collections::BTreeSet<i32> = ink.iter().map(|p| p.1).collect();
    assert!(rows.len() > 30, "{} rows", rows.len());
    for &(x, y) in &ink {
        let p = px(&img, x, y);
        let t = ((y as f64 - y1) / (y2 - y1)).clamp(0.0, 1.0);
        let want = 255.0 * (1.0 - t);
        assert_eq!((p[0], p[1]), (p[1], p[2]), "grey at ({x},{y}): {p:?}");
        assert!(
            (p[0] as f64 - want).abs() <= 2.0,
            "({x},{y}) grey {} want {want:.1}",
            p[0]
        );
    }
}

#[test]
fn text_pattern_tile_origin_sits_on_h1() {
    // h1 (75,73) with tile-sized handles, as a new pattern fill gets them
    let h = [(75.0, 73.0), (82.0, 73.0), (75.0, 68.0)];
    let Some(img) = render(pattern_attrs(Fill::EDGE_HARD), h, (0, 0)) else {
        eprintln!("skipped: no usable system font");
        return;
    };
    let ink = opaque(&img);
    assert!(ink.len() > 300, "{} opaque pixels", ink.len());
    for &(x, y) in &ink {
        let tx = (x - 75).rem_euclid(TW as i32) as u32;
        let ty = (y - 73).rem_euclid(TH as i32) as u32;
        let c = texel(tx, ty);
        assert_eq!(px(&img, x, y), [c[0], c[1], c[2], 255], "({x},{y})");
    }
}

#[test]
fn text_paint_moves_with_the_text_and_its_handles() {
    let pat = [(75.0, 73.0), (82.0, 73.0), (75.0, 68.0)];
    for (attrs, h) in [
        (gradient_attrs as fn(i32) -> PathAttrs, GRAD_H),
        (pattern_attrs as fn(i32) -> PathAttrs, pat),
    ] {
        let Some(a) = render(attrs(Fill::EDGE_ANTIALIAS), h, (0, 0)) else {
            eprintln!("skipped: no usable system font");
            return;
        };
        let b = render(attrs(Fill::EDGE_ANTIALIAS), h, (37, 23)).expect("ink");
        for y in 0..H - 23 {
            for x in 0..W - 37 {
                assert_eq!(px(&a, x, y), px(&b, x + 37, y + 23), "({x},{y})");
            }
        }
    }
}

#[test]
fn text_paint_keeps_the_coverage_of_the_edge_kind() {
    // the alpha of a gradient text equals the alpha of the same text with a solid fill, for every
    // edge kind (the coverage masks the paint; F73 feather/dither path reused)
    for (edge, feather) in [
        (Fill::EDGE_HARD, 0),
        (Fill::EDGE_ANTIALIAS, 0),
        (Fill::EDGE_FEATHER, 3),
        (Fill::EDGE_DITHER, 3),
    ] {
        let with = |mut a: PathAttrs| {
            if let Some(f) = a.fill.as_mut() {
                f.edge = edge;
                f.feather = feather;
            }
            a
        };
        let Some(g) = render(with(gradient_attrs(edge)), GRAD_H, (0, 0)) else {
            eprintln!("skipped: no usable system font");
            return;
        };
        let s = render(
            with(PathAttrs::solid_fill(Argb(0xff00_0000))),
            GRAD_H,
            (0, 0),
        )
        .expect("ink");
        for y in 0..H {
            for x in 0..W {
                assert_eq!(px(&g, x, y)[3], px(&s, x, y)[3], "edge {edge} ({x},{y})");
            }
        }
    }
}

#[test]
fn solid_text_keeps_its_run_colour() {
    let Some(img) = render(PathAttrs::solid_fill(Argb(0xff20_4060)), GRAD_H, (0, 0)) else {
        eprintln!("skipped: no usable system font");
        return;
    };
    for (x, y) in opaque(&img) {
        assert_eq!(px(&img, x, y), [0x20, 0x40, 0x60, 255]);
    }
}
