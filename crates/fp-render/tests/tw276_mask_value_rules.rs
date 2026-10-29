//! TW-276 (FB-112..FB-116, RE spec 75 part 1): what a vector mask object contributes to the mask
//! value in each mode. Layout of the measurement: transparent page 200 × 200, opaque blue content
//! over the whole page, mask rectangle 50..150 (or circle 40..160); the page alpha is mask value ×
//! content alpha.
//!
//! - path outline without "Show fill and stroke": only the geometry, always antialiased (edge
//!   "Hard" ignored), softened by the fill's feather; colour, colour alpha, gradient, dither,
//!   stroke, live effects, the mask object's opacity and blend mode do not count;
//! - path outline with "Show fill and stroke": the fill under the content (edge and feather
//!   visible), the content clipped hard to the antialiased geometry, the stroke over the content;
//! - grayscale: Rec. 709 luma × alpha of the rendered look (fill, stroke, live effects), without
//!   the mask object's opacity and blend mode.

use std::sync::Arc;

use fp_color::{Argb, BlendMode, Opacity};
use fp_geom::{Point, Rect};
use fp_model::attrs::{Gradient, GradientStop, GradientStops, StopList};
use fp_model::effects::{Effect, EffectList};
use fp_model::{Brush, Document, Element, ElementKind, Fill, FillHandles, LayerSlot, PathAttrs};
use fp_raster::RgbaImage;
use fp_render::{CachePolicy, RenderOptions, Renderer};

const INVERT: &str = "{d2541291-70d6-11d1-8c8000a024cdc039}";

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    /// "Group as mask" (path outline, underlay off).
    Outline,
    /// Path outline with "Show fill and stroke" (Paste Inside).
    Shown,
    /// "Grayscale display".
    Gray,
}

fn render(mask: Element, mode: Mode, mut doc: Document) -> RgbaImage {
    let content = Element::new_rectangle(
        &mut doc.ids,
        Rect::new(0.0, 0.0, 200.0, 200.0),
        0.0,
        PathAttrs::solid_fill(Argb(0xff00_00ff)),
    );
    let mut g = Element::new_mask_group(
        &mut doc.ids,
        Arc::new(mask),
        vec![Arc::new(content)],
        mode != Mode::Gray,
    );
    if let ElementKind::Group(gr) = &mut g.kind {
        gr.mask_flags.hide_attrs = mode != Mode::Shown;
    }
    let p = Arc::make_mut(&mut doc.pages[0]);
    if let LayerSlot::Own(l) = &mut p.root.layers[0] {
        Arc::make_mut(&mut Arc::make_mut(l).cels[0]).elements = vec![Arc::new(g)];
    }
    let out = Renderer::new(RenderOptions {
        cache_policy: CachePolicy::None,
        ..RenderOptions::default()
    })
    .render_page(&doc, 0, 0);
    out.image
}

fn new_doc() -> Document {
    Document::new(200.0, 200.0, Argb(0x00ff_ffff))
}

/// Mask object: rectangle 50..150 (`round` = 1: circle 40..160) with `attrs`, adjusted by `f`.
fn variant(mode: Mode, round: bool, attrs: PathAttrs, f: impl FnOnce(&mut Element)) -> RgbaImage {
    let mut doc = new_doc();
    let r = if round {
        Rect::new(40.0, 40.0, 160.0, 160.0)
    } else {
        Rect::new(50.0, 50.0, 150.0, 150.0)
    };
    let mut m = Element::new_rectangle(&mut doc.ids, r, if round { 1.0 } else { 0.0 }, attrs);
    f(&mut m);
    render(m, mode, doc)
}

fn white() -> PathAttrs {
    PathAttrs::solid_fill(Argb(0xffff_ffff))
}

fn with_fill(mut a: PathAttrs, f: impl FnOnce(&mut Fill)) -> PathAttrs {
    if let Some(fl) = a.fill.as_mut() {
        f(fl);
    }
    a
}

fn edge(e: i32, feather: i32) -> PathAttrs {
    with_fill(white(), |f| {
        f.edge = e;
        f.feather = feather;
    })
}

/// Antialiased white stroke of `width` with colour `color` (centred).
fn stroked(mut a: PathAttrs, color: u32, width: i32) -> PathAttrs {
    let mut b = Brush::basic(width);
    b.antialiased = true;
    a.brush = Some(b);
    a.stroke_color = Argb(color);
    a
}

fn invert(m: &mut Element) {
    m.common.effects = Some(EffectList {
        effects: vec![Effect::new(INVERT, true)],
        ..EffectList::default()
    });
}

fn stops(v: &[(f32, u32)]) -> StopList {
    StopList {
        stops: v
            .iter()
            .map(|&(p, c)| GradientStop {
                position: p,
                color: Argb(c),
                extra: Default::default(),
            })
            .collect(),
        extra: Default::default(),
    }
}

/// Linear gradient fill over x = 50..150: `colors`, `opacities`, `dither`.
fn gradient(colors: &[(f32, u32)], opacities: &[(f32, u32)], dither: bool) -> PathAttrs {
    with_fill(white(), |f| {
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
                colors: stops(colors),
                opacities: stops(opacities),
                dither: Some(dither),
                dither_value: None,
                extra: Default::default(),
            }),
            extra: Default::default(),
        });
    })
}

fn gradient_handles(m: &mut Element) {
    let fh = FillHandles {
        valid: true,
        start: Point::new(50.0, 100.0),
        end: Point::new(150.0, 100.0),
        second: Point::new(50.0, 50.0),
    };
    if let ElementKind::Path(pe) = &mut m.kind {
        pe.fill_handles = fh;
        if let Some(pr) = &mut pe.primitive {
            pr.fill_handles = fh;
        }
    }
}

const BW: &[(f32, u32)] = &[(0.0, 0xffff_ffff), (1.0, 0xff00_0000)];
const OPAQUE: &[(f32, u32)] = &[(0.0, 0xffff_ffff), (1.0, 0xffff_ffff)];

fn a(img: &RgbaImage, x: i32, y: i32) -> u8 {
    img.get(x, y).map(|p| p[3]).unwrap_or(0)
}

fn px(img: &RgbaImage, x: i32, y: i32) -> [u8; 4] {
    img.get(x, y).unwrap_or([0; 4])
}

fn row(img: &RgbaImage, xs: std::ops::RangeInclusive<i32>, y: i32) -> Vec<u8> {
    xs.map(|x| a(img, x, y)).collect()
}

fn same(l: &RgbaImage, r: &RgbaImage) -> bool {
    (0..200).all(|y| (0..200).all(|x| px(l, x, y) == px(r, x, y)))
}

/// The plain rectangle 50..150: 255 inside, 0 outside.
fn assert_plain_rect(img: &RgbaImage, what: &str) {
    for x in [49, 150, 151] {
        assert_eq!(a(img, x, 100), 0, "{what}: outside x={x}");
    }
    for x in [50, 100, 149] {
        assert_eq!(a(img, x, 100), 255, "{what}: inside x={x}");
    }
    assert_eq!(
        a(img, 100, 46),
        0,
        "{what}: the stroke does not extend the mask"
    );
}

// ---- path outline without underlay (spec 75 §1.2, §1.3) -------------------------------------

#[test]
fn outline_ignores_the_hard_edge() {
    let aa = variant(Mode::Outline, true, edge(Fill::EDGE_ANTIALIAS, 0), |_| {});
    let hard = variant(Mode::Outline, true, edge(Fill::EDGE_HARD, 0), |_| {});
    assert!(same(&aa, &hard), "Hard and Anti-Alias give the same mask");
    let d = a(&hard, 57, 57);
    assert_eq!(d, 160, "antialiased diagonal 57 (reference 160)");
}

#[test]
fn outline_ignores_colour_alpha_gradient_dither_pattern_stroke_effects_opacity_blend() {
    let cases: Vec<(&str, RgbaImage)> = vec![
        (
            "red",
            variant(
                Mode::Outline,
                false,
                PathAttrs::solid_fill(Argb(0xffff_0000)),
                |_| {},
            ),
        ),
        (
            "alpha 50 %",
            variant(
                Mode::Outline,
                false,
                PathAttrs::solid_fill(Argb(0x80ff_ffff)),
                |_| {},
            ),
        ),
        (
            "gradient",
            variant(
                Mode::Outline,
                false,
                gradient(BW, OPAQUE, false),
                gradient_handles,
            ),
        ),
        (
            "opacity gradient",
            variant(
                Mode::Outline,
                false,
                gradient(OPAQUE, &[(0.0, 0xffff_ffff), (1.0, 0x00ff_ffff)], false),
                gradient_handles,
            ),
        ),
        (
            "dither",
            variant(
                Mode::Outline,
                false,
                gradient(BW, OPAQUE, true),
                gradient_handles,
            ),
        ),
        (
            "stroke 10",
            variant(
                Mode::Outline,
                false,
                stroked(white(), 0xffff_ffff, 10),
                |_| {},
            ),
        ),
        (
            "black stroke",
            variant(
                Mode::Outline,
                false,
                stroked(white(), 0xff00_0000, 10),
                |_| {},
            ),
        ),
        ("invert", variant(Mode::Outline, false, white(), invert)),
        (
            "opacity 50 + multiply",
            variant(Mode::Outline, false, white(), |m| {
                m.common.opacity = Opacity(128);
                m.common.blend = BlendMode::MULTIPLY;
            }),
        ),
        (
            "stroke only",
            variant(
                Mode::Outline,
                false,
                stroked(PathAttrs::default(), 0xffff_ffff, 10),
                |_| {},
            ),
        ),
    ];
    for (what, img) in &cases {
        assert_plain_rect(img, what);
        assert_eq!(
            px(img, 100, 100),
            [0, 0, 255, 255],
            "{what}: content unchanged"
        );
    }
}

#[test]
fn outline_feather_4_row_matches_the_reference() {
    let img = variant(Mode::Outline, false, edge(Fill::EDGE_FEATHER, 4), |_| {});
    assert_eq!(
        row(&img, 44..=56, 100),
        vec![0, 0, 10, 30, 61, 102, 153, 193, 224, 244, 255, 255, 255]
    );
}

#[test]
fn outline_feather_12_row_matches_the_reference() {
    let img = variant(Mode::Outline, false, edge(Fill::EDGE_FEATHER, 12), |_| {});
    let want = [
        0u8, 0, 1, 4, 9, 15, 22, 31, 42, 54, 67, 82, 99, 117, 137, 155, 172, 187, 200, 212, 223,
        232, 239, 245, 250, 253, 255,
    ];
    let got = row(&img, 36..=62, 100);
    for (i, (g, w)) in got.iter().zip(want).enumerate() {
        assert!(g.abs_diff(w) <= 1, "x={}: {g} vs {w}: {got:?}", 36 + i);
    }
}

// ---- path outline with underlay (spec 75 §1.2, §1.3, §1.6) ----------------------------------

#[test]
fn shown_feather_lies_under_the_hard_content() {
    let img = variant(Mode::Shown, false, edge(Fill::EDGE_FEATHER, 4), |_| {});
    assert_eq!(
        row(&img, 46..=49, 100),
        vec![10, 30, 61, 102],
        "white feather outside"
    );
    assert_eq!(px(&img, 48, 100)[0], 255, "the underlay is the white fill");
    for x in [50, 51, 52, 100] {
        assert_eq!(
            px(&img, x, 100),
            [0, 0, 255, 255],
            "content hard from x=50 (x={x})"
        );
    }
}

#[test]
fn shown_hard_edge_clips_the_content_antialiased() {
    let hard = variant(Mode::Shown, true, edge(Fill::EDGE_HARD, 0), |_| {});
    let outline = variant(Mode::Outline, true, edge(Fill::EDGE_HARD, 0), |_| {});
    // where the hard fill is empty only the antialiased content shows
    let mut checked = 0;
    for y in 40..160 {
        for x in 40..160 {
            let o = px(&outline, x, y);
            let h = px(&hard, x, y);
            if o[3] != 0 && o[3] != 255 && h[0] == 0 {
                assert_eq!(h, o, "content × AA geometry at {x},{y}");
                checked += 1;
            }
        }
    }
    assert!(checked > 0, "some edge pixels lie outside the hard fill");
}

#[test]
fn shown_stroke_is_drawn_over_the_content() {
    let img = variant(
        Mode::Shown,
        false,
        stroked(white(), 0xffff_ffff, 10),
        |_| {},
    );
    assert_eq!(
        px(&img, 47, 100),
        [255, 255, 255, 255],
        "stroke outside the path"
    );
    assert_eq!(
        px(&img, 52, 100),
        [255, 255, 255, 255],
        "stroke over the content"
    );
    assert_eq!(px(&img, 60, 100), [0, 0, 255, 255], "content inside");
    assert_eq!(a(&img, 43, 100), 0, "nothing beyond the stroke");
    let blk = variant(
        Mode::Shown,
        false,
        stroked(white(), 0xff00_0000, 10),
        |_| {},
    );
    assert_eq!(
        px(&blk, 52, 100),
        [0, 0, 0, 255],
        "black edge over the content"
    );
}

#[test]
fn shown_half_transparent_stroke_mixes_over_the_content() {
    let img = variant(
        Mode::Shown,
        false,
        stroked(white(), 0x80ff_ffff, 10),
        |_| {},
    );
    let out = px(&img, 47, 100);
    assert!((125..=128).contains(&out[3]), "outside α 125/128: {out:?}");
    let inside = px(&img, 52, 100);
    assert_eq!(inside[3], 255);
    assert!(
        (127..=129).contains(&inside[0]),
        "50 % white over blue: {inside:?}"
    );
}

#[test]
fn shown_ignores_effects_opacity_and_blend() {
    let plain = variant(Mode::Shown, false, white(), |_| {});
    let fx = variant(Mode::Shown, false, white(), invert);
    let op = variant(Mode::Shown, false, white(), |m| {
        m.common.opacity = Opacity(128);
        m.common.blend = BlendMode::MULTIPLY;
    });
    assert!(
        same(&plain, &fx),
        "live effects are ignored, also at the underlay"
    );
    assert!(same(&plain, &op), "opacity and blend mode are ignored");
    assert_eq!(a(&plain, 49, 100), 0);
}

// ---- grayscale (spec 75 §1.4) ---------------------------------------------------------------

fn gray_center(attrs: PathAttrs, f: impl FnOnce(&mut Element)) -> u8 {
    let img = variant(Mode::Gray, false, attrs, f);
    assert_eq!(a(&img, 20, 100), 0, "outside");
    a(&img, 100, 100)
}

#[test]
fn gray_luma_times_alpha_without_opacity_and_blend() {
    assert_eq!(gray_center(white(), |_| {}), 255);
    assert_eq!(
        gray_center(PathAttrs::solid_fill(Argb(0xffff_0000)), |_| {}),
        54
    );
    assert_eq!(
        gray_center(PathAttrs::solid_fill(Argb(0xffff_0000)), |m| {
            m.common.blend = BlendMode::MULTIPLY
        }),
        54,
        "blend mode ignored"
    );
    assert_eq!(
        gray_center(PathAttrs::solid_fill(Argb(0x80ff_ffff)), |_| {}),
        128
    );
    assert_eq!(
        gray_center(white(), |m| m.common.opacity = Opacity(128)),
        255,
        "opacity ignored"
    );
}

#[test]
fn gray_live_effects_count() {
    let img = variant(Mode::Gray, false, white(), invert);
    assert_eq!(a(&img, 100, 100), 0, "invert makes the white mask black");
}

#[test]
fn gray_hard_edge_counts() {
    let img = variant(Mode::Gray, true, edge(Fill::EDGE_HARD, 0), |_| {});
    for y in 30..170 {
        for x in 30..170 {
            let v = a(&img, x, y);
            assert!(v == 0 || v == 255, "hard edge at {x},{y}: {v}");
        }
    }
    assert_eq!(a(&img, 57, 57), 0, "diagonal 57");
}

#[test]
fn gray_feather_matches_the_outline() {
    let g = variant(Mode::Gray, false, edge(Fill::EDGE_FEATHER, 4), |_| {});
    assert_eq!(
        row(&g, 44..=56, 100),
        vec![0, 0, 10, 30, 61, 102, 153, 193, 224, 244, 255, 255, 255]
    );
}

#[test]
fn gray_gradient_and_opacity_gradient_ramp_linearly() {
    let want = [255u8, 230, 204, 179, 153, 128, 102, 77, 51, 26];
    let g = variant(
        Mode::Gray,
        false,
        gradient(BW, OPAQUE, false),
        gradient_handles,
    );
    let o = variant(
        Mode::Gray,
        false,
        gradient(OPAQUE, &[(0.0, 0xffff_ffff), (1.0, 0x00ff_ffff)], false),
        gradient_handles,
    );
    for (k, w) in want.iter().enumerate() {
        let x = 50 + 10 * k as i32;
        for img in [&g, &o] {
            let v = a(img, x, 100);
            assert!(v.abs_diff(*w) <= 1, "x={x}: {v} vs {w}");
        }
    }
}

#[test]
fn gray_dithered_gradient_follows_the_s_curve_with_noise() {
    // column means of the reference (spec 75 §1.5), x = 50 + 10 k
    let want = [
        255.0, 238.6, 216.3, 189.2, 159.1, 127.5, 95.9, 65.8, 38.7, 16.4,
    ];
    let img = variant(
        Mode::Gray,
        false,
        gradient(BW, OPAQUE, true),
        gradient_handles,
    );
    let mut noisy = 0;
    for (k, w) in want.iter().enumerate() {
        let x = 50 + 10 * k as i32;
        let col: Vec<u8> = (50..150).map(|y| a(&img, x, y)).collect();
        let mean = col.iter().map(|&v| v as f64).sum::<f64>() / col.len() as f64;
        assert!((mean - w).abs() <= 1.0, "x={x}: mean {mean:.1} vs {w}");
        let lo = *col.iter().min().unwrap_or(&0);
        let hi = *col.iter().max().unwrap_or(&0);
        assert!(hi - lo <= 2, "noise ±1 at x={x}: {lo}..{hi}");
        noisy += (hi != lo) as usize;
    }
    assert!(noisy >= 5, "the dither adds noise");
}

#[test]
fn gray_stroke_extends_or_hides() {
    let w = variant(Mode::Gray, false, stroked(white(), 0xffff_ffff, 10), |_| {});
    assert!(a(&w, 47, 100) > 200, "a white stroke extends the mask");
    assert_eq!(a(&w, 100, 100), 255);
    let b = variant(Mode::Gray, false, stroked(white(), 0xff00_0000, 10), |_| {});
    assert_eq!(a(&b, 51, 100), 0, "a black stroke hides");
    assert_eq!(a(&b, 100, 100), 255);
    let s = variant(
        Mode::Gray,
        false,
        stroked(PathAttrs::default(), 0xffff_ffff, 10),
        |_| {},
    );
    assert_eq!(a(&s, 100, 100), 0, "stroke only: the inside stays empty");
    assert!(a(&s, 50, 100) > 200, "the ring counts");
}
