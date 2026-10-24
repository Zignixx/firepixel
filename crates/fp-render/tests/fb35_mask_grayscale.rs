//! FB-35/FB-36 (RE spec 68 §2.3, §4.2): the mask mode is the group's switch, not the kind of the
//! mask object. Grayscale ("mask to image") takes Rec. 709 luma × alpha of the mask object's
//! rendered look (fill colour, gradient, feather count); path outline ("mask to path") takes the
//! fill coverage of a path and the alpha of a bitmap. Bitmap masks are held in display form
//! (white = visible) and the live paint pixels (BA-039 override) count during a stroke.

use std::sync::Arc;

use fp_color::Argb;
use fp_geom::{IRect, Point, Rect};
use fp_model::attrs::{Gradient, GradientStop, GradientStops, StopList};
use fp_model::{Document, Element, ElementKind, Fill, FillHandles, LayerSlot, PathAttrs};
use fp_raster::RgbaImage;
use fp_render::{CachePolicy, PixelOverride, RenderOptions, Renderer};

/// Transparent page 100 × 100: a white square 0..100 masked by `mask`; `vector` = "mask to path",
/// `show` = "Show fill and stroke". Returns the document and the mask's id.
fn page(
    mut doc: Document,
    mask: Element,
    vector: bool,
    show: bool,
) -> (Document, fp_model::ElementId) {
    let content = Element::new_rectangle(
        &mut doc.ids,
        Rect::new(0.0, 0.0, 100.0, 100.0),
        0.0,
        PathAttrs::solid_fill(Argb(0xffff_ffff)),
    );
    page_with(doc, mask, content, vector, show)
}

fn page_with(
    mut doc: Document,
    mask: Element,
    content: Element,
    vector: bool,
    show: bool,
) -> (Document, fp_model::ElementId) {
    let mid = mask.id;
    let mut g = Element::new_mask_group(
        &mut doc.ids,
        Arc::new(mask),
        vec![Arc::new(content)],
        vector,
    );
    if let ElementKind::Group(gr) = &mut g.kind {
        gr.mask_flags.hide_attrs = !show;
    }
    let p = Arc::make_mut(&mut doc.pages[0]);
    if let LayerSlot::Own(l) = &mut p.root.layers[0] {
        Arc::make_mut(&mut Arc::make_mut(l).cels[0]).elements = vec![Arc::new(g)];
    }
    (doc, mid)
}

fn new_doc() -> Document {
    Document::new(100.0, 100.0, Argb(0x00ff_ffff))
}

fn rect_mask(doc: &mut Document, r: Rect, attrs: PathAttrs) -> Element {
    Element::new_rectangle(&mut doc.ids, r, 0.0, attrs)
}

fn renderer() -> Renderer {
    Renderer::new(RenderOptions {
        cache_policy: CachePolicy::None,
        ..RenderOptions::default()
    })
}

fn render(doc: &Document) -> RgbaImage {
    let out = renderer().render_page(doc, 0, 0);
    out.image
}

fn alpha(img: &RgbaImage, x: i32, y: i32) -> u8 {
    img.get(x, y).map(|p| p[3]).unwrap_or(0)
}

fn near(a: u8, want: u8) -> bool {
    a.abs_diff(want) <= 1
}

/// Grayscale mask of a solid colour: alpha of the white content = luma709 of the colour.
fn gray_of(color: u32) -> u8 {
    let mut doc = new_doc();
    let m = rect_mask(
        &mut doc,
        Rect::new(10.0, 10.0, 90.0, 90.0),
        PathAttrs::solid_fill(Argb(color)),
    );
    let (doc, _) = page(doc, m, false, false);
    let img = render(&doc);
    assert_eq!(alpha(&img, 5, 5), 0, "outside the mask hidden");
    alpha(&img, 50, 50)
}

#[test]
fn grayscale_vector_mask_uses_rec709_luma_of_the_fill_colour() {
    // measured: red 21 %, green 72 %, blue 7 % (RE spec 68 §2.3)
    let red = gray_of(0xffff_0000);
    assert!(near(red, 54), "red {red} (21 %)");
    let green = gray_of(0xff00_ff00);
    assert!(near(green, 182), "green {green} (72 %)");
    let blue = gray_of(0xff00_00ff);
    assert!(near(blue, 18), "blue {blue} (7 %)");
    assert_eq!(gray_of(0xffff_ffff), 255);
    assert_eq!(gray_of(0xff00_0000), 0);
}

#[test]
fn path_outline_takes_the_geometry_whatever_the_colour() {
    let mut doc = new_doc();
    let m = rect_mask(
        &mut doc,
        Rect::new(10.0, 10.0, 90.0, 90.0),
        PathAttrs::solid_fill(Argb(0xffff_0000)),
    );
    let (doc, _) = page(doc, m, true, false);
    let img = render(&doc);
    assert_eq!(alpha(&img, 50, 50), 255);
    assert_eq!(alpha(&img, 5, 5), 0);
}

#[test]
fn grayscale_gradient_mask_ramps() {
    let mut doc = new_doc();
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
    let mut a = PathAttrs::solid_fill(Argb(0xffff_ffff));
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
                colors: stops(&[(0.0, 0xffff_ffff), (1.0, 0xff00_0000)]),
                opacities: stops(&[(0.0, 0xffff_ffff), (1.0, 0xffff_ffff)]),
                dither: None,
                dither_value: None,
                extra: Default::default(),
            }),
            extra: Default::default(),
        });
    }
    let mut m = rect_mask(&mut doc, Rect::new(0.0, 0.0, 100.0, 100.0), a);
    let fh = FillHandles {
        valid: true,
        start: Point::new(0.0, 50.0),
        end: Point::new(100.0, 50.0),
        second: Point::new(0.0, 0.0),
    };
    if let ElementKind::Path(pe) = &mut m.kind {
        pe.fill_handles = fh;
        if let Some(pr) = &mut pe.primitive {
            pr.fill_handles = fh;
        }
    }
    let (doc, _) = page(doc, m, false, false);
    let img = render(&doc);
    let (a1, a2, a3) = (
        alpha(&img, 10, 50),
        alpha(&img, 50, 50),
        alpha(&img, 90, 50),
    );
    assert!(a1 > a2 && a2 > a3, "white → black ramps: {a1} {a2} {a3}");
    assert!(a1 > 200 && a3 < 55, "{a1} {a3}");
}

#[test]
fn grayscale_feathered_mask_ramps() {
    let mut doc = new_doc();
    let mut a = PathAttrs::solid_fill(Argb(0xffff_ffff));
    if let Some(f) = a.fill.as_mut() {
        f.edge = Fill::EDGE_FEATHER;
        f.feather = 10;
    }
    let m = rect_mask(&mut doc, Rect::new(20.0, 20.0, 80.0, 80.0), a);
    let (doc, _) = page(doc, m, false, false);
    let img = render(&doc);
    assert_eq!(alpha(&img, 50, 50), 255, "inside");
    let edge = alpha(&img, 20, 50);
    assert!(
        edge > 0 && edge < 255,
        "the feather ramps at the edge: {edge}"
    );
    assert!(alpha(&img, 22, 50) > alpha(&img, 18, 50));
    assert_eq!(alpha(&img, 2, 50), 0, "outside");
}

/// A bitmap mask 0..100 of one grey value (display form).
fn bitmap_mask(doc: &mut Document, rgba: [u8; 4]) -> Element {
    let mut tiles = 1;
    let px: Vec<u8> = (0..100 * 100).flat_map(|_| rgba).collect();
    Element::new_bitmap(
        &mut doc.ids,
        Point::new(0.0, 0.0),
        100,
        100,
        &px,
        &mut tiles,
    )
}

/// The mask image with a 20 × 20 dab of `rgba` at 40..60.
fn dabbed(base: [u8; 4], dab: [u8; 4]) -> Arc<RgbaImage> {
    let mut img = RgbaImage::filled(
        IRect::new(0, 0, 100, 100),
        fp_color::Rgba8::new(base[0], base[1], base[2], base[3]),
    )
    .expect("image");
    for y in 40..60 {
        for x in 40..60 {
            img.set_pixel(x, y, fp_color::Rgba8::new(dab[0], dab[1], dab[2], dab[3]));
        }
    }
    Arc::new(img)
}

#[test]
fn white_dab_on_grayscale_bitmap_mask_reveals_live() {
    let mut doc = new_doc();
    let m = bitmap_mask(&mut doc, [0, 0, 0, 255]);
    let (doc, mid) = page(doc, m, false, false);
    let img = render(&doc);
    assert_eq!(alpha(&img, 50, 50), 0, "black hides (display form, M-6)");
    let ov = PixelOverride {
        id: mid,
        image: dabbed([0, 0, 0, 255], [255, 255, 255, 255]),
    };
    let out = renderer()
        .render_region_with(&doc, 0, 0, IRect::new(0, 0, 100, 100), 1.0, &ov)
        .expect("mask bitmap found");
    assert_eq!(
        alpha(&out.image, 50, 50),
        255,
        "the white dab reveals during the stroke"
    );
    assert_eq!(alpha(&out.image, 10, 10), 0, "black stays hidden");
}

#[test]
fn black_dab_on_grayscale_bitmap_mask_hides() {
    let mut doc = new_doc();
    let m = bitmap_mask(&mut doc, [255, 255, 255, 255]);
    let (doc, mid) = page(doc, m, false, false);
    assert_eq!(alpha(&render(&doc), 50, 50), 255, "white shows");
    let ov = PixelOverride {
        id: mid,
        image: dabbed([255, 255, 255, 255], [0, 0, 0, 255]),
    };
    let out = renderer()
        .render_region_with(&doc, 0, 0, IRect::new(0, 0, 100, 100), 1.0, &ov)
        .expect("mask bitmap found");
    assert_eq!(alpha(&out.image, 50, 50), 0, "the black dab hides");
    assert_eq!(alpha(&out.image, 10, 10), 255);
    // a red dab gives the luma, 21 %
    let ov = PixelOverride {
        id: mid,
        image: dabbed([255, 255, 255, 255], [255, 0, 0, 255]),
    };
    let out = renderer()
        .render_region_with(&doc, 0, 0, IRect::new(0, 0, 100, 100), 1.0, &ov)
        .expect("mask bitmap found");
    assert!(near(alpha(&out.image, 50, 50), 54));
}

#[test]
fn alpha_channel_bitmap_mask_paints_live_without_the_underlay() {
    // alpha channel ("mask to path") + show fill and stroke (Paste Inside): the bitmap lies under
    // the content, but not while it is painted; the stroke's alpha masks live
    let mut doc = new_doc();
    let mut tiles = 1;
    let px: Vec<u8> = (0..100 * 100)
        .flat_map(|i| {
            let x = i % 100;
            if (30..70).contains(&x) {
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
    let content = Element::new_rectangle(
        &mut doc.ids,
        Rect::new(0.0, 0.0, 50.0, 100.0),
        0.0,
        PathAttrs::solid_fill(Argb(0xffff_ffff)),
    );
    let (doc, mid) = page_with(doc, m, content, true, true);
    let img = render(&doc);
    assert_eq!(
        img.get(60, 50),
        Some([255, 0, 0, 255]),
        "red underlay where the content has no pixels"
    );
    assert_eq!(img.get(40, 50), Some([255, 255, 255, 255]));
    assert_eq!(alpha(&img, 10, 50), 0, "outside the mask");
    let mut stroke = (*dabbed([0, 0, 0, 0], [255, 0, 0, 255])).clone();
    for y in 0..100 {
        for x in 30..70 {
            stroke.set_pixel(x, y, fp_color::Rgba8::new(255, 0, 0, 255));
        }
    }
    for y in 0..100 {
        for x in 5..15 {
            stroke.set_pixel(x, y, fp_color::Rgba8::new(255, 0, 0, 255));
        }
    }
    let ov = PixelOverride {
        id: mid,
        image: Arc::new(stroke),
    };
    let out = renderer()
        .render_region_with(&doc, 0, 0, IRect::new(0, 0, 100, 100), 1.0, &ov)
        .expect("mask bitmap found");
    assert_eq!(
        alpha(&out.image, 60, 50),
        0,
        "no underlay while the mask is painted"
    );
    assert_eq!(
        out.image.get(10, 50),
        Some([255, 255, 255, 255]),
        "the new stroke reveals live"
    );
}
