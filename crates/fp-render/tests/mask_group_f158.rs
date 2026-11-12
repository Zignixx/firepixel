//! F158 (RE spec 81 §1 and §2.4, deviations D1 and D2): mask groups on the render side.
//!
//! - D1 (`tgg3_darken`): inside a mask group with several contents every child is drawn Normal,
//!   also the children of a plain group inside the mask group (only opacity counts).
//! - D2 (`gz_gam`): the path outline value of a group used as mask is the union of the shapes of
//!   its children, without their opacity and colour alpha; Grayscale stays luma × alpha of the
//!   group's look (`gz_gs`).
//!
//! Scene values of the spec: backdrop #FF8000, C #0080FF 20..200 × 40..160, C2 #00C000
//! 120..280 × 60..140, mask M white 10..290 × 10..190; Y #FFD000 for the group mask scenes.

use std::sync::Arc;

use fp_color::{Argb, BlendMode, Opacity};
use fp_geom::{Point, Rect};
use fp_model::{Document, Element, ElementKind, GroupKind, LayerSlot, PathAttrs};
use fp_render::{CachePolicy, RenderOptions, Renderer};

const BG: [u8; 3] = [255, 128, 0];

fn rect(doc: &mut Document, r: [i32; 4], argb: u32) -> Element {
    Element::new_rectangle(
        &mut doc.ids,
        Rect::new(r[0] as f32, r[1] as f32, r[2] as f32, r[3] as f32),
        0.0,
        PathAttrs::solid_fill(Argb(argb)),
    )
}

fn darken() -> BlendMode {
    BlendMode::from_name("darken").unwrap_or(BlendMode(0))
}

fn put(doc: &mut Document, elems: Vec<Element>) {
    let page = Arc::make_mut(&mut doc.pages[0]);
    if let LayerSlot::Own(l) = &mut page.root.layers[0] {
        let cel = Arc::make_mut(&mut Arc::make_mut(l).cels[0]);
        cel.elements = elems.into_iter().map(Arc::new).collect();
    }
}

fn render(doc: &Document, policy: CachePolicy) -> fp_raster::RgbaImage {
    Renderer::new(RenderOptions {
        cache_policy: policy,
        ..RenderOptions::default()
    })
    .render_page(doc, 0, 0)
    .image
}

fn rgb(img: &fp_raster::RgbaImage, x: i32, y: i32) -> [u8; 3] {
    let p = img.pixel(x, y);
    [p.r, p.g, p.b]
}

fn near(a: [u8; 3], b: [u8; 3]) -> bool {
    a.iter()
        .zip(b)
        .all(|(x, y)| (*x as i32 - y as i32).abs() <= 1)
}

/// `tgg3_darken`: mask group with X (magenta) and a plain group G {C2 darken, C}; `g_op` is the
/// opacity of G in per cent.
fn tgg3(g_op: f64) -> Document {
    let mut doc = Document::new(300.0, 200.0, Argb(0xffff_8000));
    let mut c2 = rect(&mut doc, [120, 60, 280, 140], 0xff00_c000);
    c2.common.blend = darken();
    let c = rect(&mut doc, [20, 40, 200, 160], 0xff00_80ff);
    let mut g = Element::new_group(&mut doc.ids, vec![Arc::new(c2), Arc::new(c)]);
    g.common.opacity = Opacity::from_percent(g_op);
    let x = rect(&mut doc, [20, 165, 60, 185], 0xffff_00ff);
    let m = rect(&mut doc, [10, 10, 290, 190], 0xffff_ffff);
    let mg = Element::new_mask_group(
        &mut doc.ids,
        Arc::new(m),
        vec![Arc::new(x), Arc::new(g)],
        true,
    );
    assert!(mg.masked_single().is_none());
    put(&mut doc, vec![mg]);
    doc
}

#[test]
fn d1_grandchild_blend_mode_is_normal_inside_a_mask_group() {
    let doc = tgg3(100.0);
    for policy in [CachePolicy::None, CachePolicy::default()] {
        let img = render(&doc, policy);
        // P_O and P_C2: C2 Normal (0,192,0), not darkened (0,128,0); P_C: C; X drawn
        assert_eq!(rgb(&img, 160, 100), [0, 192, 0], "{policy:?} P_O");
        assert_eq!(rgb(&img, 240, 100), [0, 192, 0], "{policy:?} P_C2");
        assert_eq!(rgb(&img, 60, 100), [0, 128, 255], "{policy:?} P_C");
        assert_eq!(rgb(&img, 40, 175), [255, 0, 255], "{policy:?} X");
        assert_eq!(rgb(&img, 150, 25), BG, "{policy:?} P_M");
    }
}

#[test]
fn d1_plain_subgroup_keeps_its_opacity() {
    let img = render(&tgg3(50.0), CachePolicy::None);
    // G at 50 %: C2 over C Normal (0,192,0), then half over the backdrop
    assert!(
        near(rgb(&img, 160, 100), [127, 160, 0]),
        "{:?}",
        rgb(&img, 160, 100)
    );
    assert!(
        near(rgb(&img, 60, 100), [127, 128, 128]),
        "{:?}",
        rgb(&img, 60, 100)
    );
}

#[test]
fn d1_plain_group_outside_a_mask_still_passes_blend_modes_through() {
    let mut doc = Document::new(300.0, 200.0, Argb(0xffff_8000));
    let mut c2 = rect(&mut doc, [120, 60, 280, 140], 0xff00_c000);
    c2.common.blend = darken();
    let c = rect(&mut doc, [20, 40, 200, 160], 0xff00_80ff);
    let g = Element::new_group(&mut doc.ids, vec![Arc::new(c2), Arc::new(c)]);
    put(&mut doc, vec![g]);
    let img = render(&doc, CachePolicy::None);
    assert_eq!(rgb(&img, 160, 100), [0, 128, 0]);
    assert_eq!(rgb(&img, 240, 100), [0, 128, 0]);
}

/// `gz_*`: Y masked by the group GM {PA red 30 %, PB green colour alpha 25 %, E3 blue path
/// 200..280 masked to 200..240 by a bitmap}; `gray` = Grayscale, else path outline.
fn gz(gray: bool, underlay: bool) -> Document {
    let mut doc = Document::new(300.0, 200.0, Argb(0xffff_8000));
    let mut pa = rect(&mut doc, [40, 40, 120, 120], 0xffff_0000);
    pa.common.opacity = Opacity::from_percent(30.0);
    let pb = rect(&mut doc, [130, 60, 190, 140], 0x4000_ff00);
    let e3 = rect(&mut doc, [200, 40, 280, 120], 0xff00_00ff);
    let px: Vec<u8> = (0..40 * 80).flat_map(|_| [255u8, 255, 255, 255]).collect();
    let mut tiles = 1;
    let bm = Element::new_bitmap(
        &mut doc.ids,
        Point::new(200.0, 40.0),
        40,
        80,
        &px,
        &mut tiles,
    );
    let e3m = Element::new_mask_group(&mut doc.ids, Arc::new(bm), vec![Arc::new(e3)], true);
    let gm = Element::new_group(
        &mut doc.ids,
        vec![Arc::new(e3m), Arc::new(pb), Arc::new(pa)],
    );
    let y = rect(&mut doc, [30, 30, 290, 170], 0xffff_d000);
    let mut g = Element::new_mask_group(&mut doc.ids, Arc::new(gm), vec![Arc::new(y)], !gray);
    if let ElementKind::Group(gr) = &mut g.kind {
        assert_eq!(
            gr.kind,
            if gray {
                GroupKind::BitmapMask
            } else {
                GroupKind::VectorMask
            }
        );
        gr.mask_flags.hide_attrs = !underlay;
    }
    put(&mut doc, vec![g]);
    doc
}

const YF: [u8; 3] = [255, 208, 0];

#[test]
fn d2_group_path_outline_is_the_union_of_the_shapes() {
    for underlay in [false, true] {
        for policy in [CachePolicy::None, CachePolicy::default()] {
            let img = render(&gz(false, underlay), policy);
            let what = format!("underlay {underlay}, {policy:?}");
            assert_eq!(rgb(&img, 80, 80), YF, "PA {what}");
            assert_eq!(rgb(&img, 160, 100), YF, "PB {what}");
            assert_eq!(rgb(&img, 220, 80), YF, "E3 visible {what}");
            assert_eq!(rgb(&img, 260, 80), BG, "E3 cut {what}");
            assert_eq!(rgb(&img, 125, 150), BG, "outside the group {what}");
        }
    }
}

#[test]
fn d2_grayscale_group_mask_keeps_opacity_and_colour_alpha() {
    let img = render(&gz(true, false), CachePolicy::None);
    // luma709 × alpha of the look: red 54 × 0.30 = 16, green 182 × 0.25 = 45, blue 18
    for (x, y, want) in [
        (80, 80, [255, 133, 0]),
        (160, 100, [255, 142, 0]),
        (220, 80, [255, 134, 0]),
        (260, 80, BG),
    ] {
        let got = rgb(&img, x, y);
        assert!(near(got, want), "({x},{y}) {got:?} != {want:?}");
    }
}
