//! TW-277 (FB-123 to FB-128, RE spec 75 part 2): opacity and blend mode in mask groups.
//!
//! - The mask object's own opacity and blend mode never count.
//! - A masked single element (one content) is the content: its opacity and blend mode apply to the
//!   whole masked result (including the visible mask fill) against the backdrop.
//! - A mask group with several contents is isolated: the children composite with their own opacity
//!   but Normal; the group's opacity and blend mode then apply against the backdrop.
//! - A plain group passes the children's blend modes through to the backdrop.
//!
//! The scenes are the measured cases of the spec (backdrop #FF8000, content C #0080FF 70 %
//! Multiply, C2 #00C000, mask object M white 50 % Screen). Every pixel of the page is compared with
//! the spec's rendering model, written out below from its formulas: opacity as the byte
//! `floor(op * 2.55 + 0.5)`, blend results rounded to a byte before mixing, mixing
//! `blend * a + d * (1 - a)`, the children of a mask group in premultiplied form.

use std::sync::Arc;

use fp_color::{Argb, BlendMode, Opacity};
use fp_geom::Rect;
use fp_model::{Document, Element, ElementKind, LayerSlot, PathAttrs};
use fp_render::{CachePolicy, RenderOptions, Renderer};

const W: i32 = 300;
const H: i32 = 200;
const BG: [u8; 3] = [255, 128, 0];
const RC: [i32; 4] = [20, 40, 200, 160];
const RC2: [i32; 4] = [120, 60, 280, 140];
const MR: [i32; 4] = [60, 30, 240, 170];
const MW: [i32; 4] = [10, 10, 290, 190];
const CC: [u8; 3] = [0, 128, 255];
const CC2: [u8; 3] = [0, 192, 0];
const WHITE: [u8; 3] = [255, 255, 255];
const NORMAL: u8 = 0;

fn mode(name: &str) -> u8 {
    BlendMode::from_name(name).map(|b| b.0).unwrap_or(0)
}

// ------------------------------------------------------------------ the spec's model

/// A rectangle of the model: rect, colour, opacity in %, blend mode.
#[derive(Clone, Copy)]
struct R {
    rect: [i32; 4],
    color: [u8; 3],
    op: f64,
    blend: u8,
}

enum M {
    Rect(R),
    /// Plain group (pass-through), children top first.
    Plain(Vec<R>),
    /// Masked single element: mask rect, content, underlay colour.
    Single([i32; 4], R, Option<[u8; 3]>),
    /// Mask group: mask rect, children top first, group opacity, group blend, underlay colour.
    Group([i32; 4], Vec<R>, f64, u8, Option<[u8; 3]>),
}

fn byte(op: f64) -> f64 {
    (op * 255.0 / 100.0 + 0.5).floor() / 255.0
}

fn inside(r: [i32; 4], x: i32, y: i32) -> bool {
    r[0] <= x && x < r[2] && r[1] <= y && y < r[3]
}

fn blend_ch(bm: u8, s: f64, d: f64) -> f64 {
    let m = (s * d / 255.0).round();
    match bm {
        b if b == mode("multiply") => m,
        b if b == mode("screen") => s + d - m,
        b if b == mode("darken") => s.min(d),
        _ => s,
    }
}

fn mix(d: [f64; 3], s: [f64; 3], a: f64, bm: u8) -> [f64; 3] {
    [0, 1, 2].map(|i| blend_ch(bm, s[i], d[i]) * a + d[i] * (1.0 - a))
}

/// Premultiplied surface of `children` (top first) over transparent, blend modes ignored.
fn surface(children: &[R], x: i32, y: i32, fill: Option<[u8; 3]>) -> [f64; 4] {
    let mut p = match fill {
        Some(f) => [f[0] as f64, f[1] as f64, f[2] as f64, 1.0],
        None => [0.0; 4],
    };
    for c in children.iter().rev() {
        if inside(c.rect, x, y) {
            let a = byte(c.op);
            for (v, col) in p.iter_mut().zip(c.color) {
                *v = col as f64 * a + *v * (1.0 - a);
            }
            p[3] = a + p[3] * (1.0 - a);
        }
    }
    p
}

fn put_surface(d: [f64; 3], p: [f64; 4], op: f64, bm: u8) -> [f64; 3] {
    if p[3] <= 0.0 {
        return d;
    }
    let s = [p[0] / p[3], p[1] / p[3], p[2] / p[3]];
    mix(d, s, p[3] * byte(op), bm)
}

fn model(scene: &[M], x: i32, y: i32) -> [u8; 3] {
    let mut d = BG.map(|v| v as f64);
    for e in scene.iter().rev() {
        match e {
            M::Rect(r) => {
                if inside(r.rect, x, y) {
                    d = mix(d, r.color.map(|v| v as f64), byte(r.op), r.blend);
                }
            }
            M::Plain(ch) => {
                for r in ch.iter().rev() {
                    if inside(r.rect, x, y) {
                        d = mix(d, r.color.map(|v| v as f64), byte(r.op), r.blend);
                    }
                }
            }
            M::Single(mr, c, fill) => {
                if inside(*mr, x, y) {
                    let neutral = R {
                        op: 100.0,
                        blend: NORMAL,
                        ..*c
                    };
                    d = put_surface(d, surface(&[neutral], x, y, *fill), c.op, c.blend);
                }
            }
            M::Group(mr, ch, op, bm, fill) => {
                if inside(*mr, x, y) {
                    d = put_surface(d, surface(ch, x, y, *fill), *op, *bm);
                }
            }
        }
    }
    d.map(|v| v.round() as u8)
}

// ------------------------------------------------------------------ the same scene in Firepixel

fn rect_el(doc: &mut Document, r: R) -> Element {
    let [l, t, rr, b] = r.rect;
    let [cr, cg, cb] = r.color;
    let argb = 0xff00_0000 | (cr as u32) << 16 | (cg as u32) << 8 | cb as u32;
    let mut e = Element::new_rectangle(
        &mut doc.ids,
        Rect::new(l as f32, t as f32, rr as f32, b as f32),
        0.0,
        PathAttrs::solid_fill(Argb(argb)),
    );
    e.common.opacity = Opacity::from_percent(r.op);
    e.common.blend = BlendMode(r.blend);
    e
}

/// The mask object: white, 50 % Screen (values that must not count).
fn mask_el(doc: &mut Document, rect: [i32; 4]) -> Arc<Element> {
    Arc::new(rect_el(
        doc,
        R {
            rect,
            color: WHITE,
            op: 50.0,
            blend: mode("screen"),
        },
    ))
}

fn mask_group(
    doc: &mut Document,
    mr: [i32; 4],
    children: &[R],
    underlay: bool,
    op: f64,
    bm: u8,
) -> Element {
    let m = mask_el(doc, mr);
    let ch = children
        .iter()
        .map(|c| Arc::new(rect_el(doc, *c)))
        .collect();
    let mut g = Element::new_mask_group(&mut doc.ids, m, ch, true);
    if let ElementKind::Group(gr) = &mut g.kind {
        gr.mask_flags.hide_attrs = !underlay;
    }
    g.common.opacity = Opacity::from_percent(op);
    g.common.blend = BlendMode(bm);
    g
}

fn build(scene: &[M]) -> Document {
    let mut doc = Document::new(W as f32, H as f32, Argb(0xffff_8000));
    let mut elems = Vec::new();
    for e in scene {
        let el = match e {
            M::Rect(r) => rect_el(&mut doc, *r),
            M::Plain(ch) => {
                let ch = ch.iter().map(|c| Arc::new(rect_el(&mut doc, *c))).collect();
                Element::new_group(&mut doc.ids, ch)
            }
            M::Single(mr, c, fill) => {
                let g = mask_group(&mut doc, *mr, &[*c], fill.is_some(), 100.0, NORMAL);
                assert!(
                    g.masked_single().is_some(),
                    "one content is a masked single element"
                );
                g
            }
            M::Group(mr, ch, op, bm, fill) => {
                let g = mask_group(&mut doc, *mr, ch, fill.is_some(), *op, *bm);
                assert!(g.masked_single().is_none());
                g
            }
        };
        elems.push(Arc::new(el));
    }
    let page = Arc::make_mut(&mut doc.pages[0]);
    if let LayerSlot::Own(l) = &mut page.root.layers[0] {
        let cel = Arc::make_mut(&mut Arc::make_mut(l).cels[0]);
        cel.elements = elems;
    }
    doc
}

fn c(op: f64, bm: &str) -> R {
    R {
        rect: RC,
        color: CC,
        op,
        blend: mode(bm),
    }
}

fn c2(op: f64, bm: &str) -> R {
    R {
        rect: RC2,
        color: CC2,
        op,
        blend: mode(bm),
    }
}

fn group(mr: [i32; 4], ch: Vec<R>, op: f64, bm: u8, fill: Option<[u8; 3]>) -> Vec<M> {
    vec![M::Group(mr, ch, op, bm, fill)]
}

fn scenes() -> Vec<(&'static str, Vec<M>)> {
    let both = || vec![c2(100.0, "normal"), c(70.0, "multiply")];
    vec![
        ("one", vec![M::Rect(c(70.0, "multiply"))]),
        (
            "two",
            vec![M::Rect(c2(100.0, "normal")), M::Rect(c(70.0, "multiply"))],
        ),
        ("gam1", vec![M::Single(MR, c(70.0, "multiply"), None)]),
        ("pam1", vec![M::Single(MR, c(70.0, "multiply"), None)]),
        (
            "pin1",
            vec![M::Single(MR, c(70.0, "multiply"), Some(WHITE))],
        ),
        ("set1", vec![M::Single(MR, c(40.0, "screen"), None)]),
        ("single", vec![M::Rect(c(100.0, "multiply"))]),
        ("single_m", vec![M::Single(MR, c(100.0, "multiply"), None)]),
        ("gam2", group(MR, both(), 100.0, NORMAL, None)),
        ("gam2o", group(MR, both(), 40.0, NORMAL, None)),
        ("gam2b", group(MR, both(), 60.0, mode("darken"), None)),
        ("pin2", group(MR, both(), 100.0, NORMAL, Some(WHITE))),
        (
            "iso",
            group(
                MW,
                vec![c2(100.0, "normal"), c(100.0, "multiply")],
                100.0,
                NORMAL,
                None,
            ),
        ),
        (
            "iso2",
            group(
                MW,
                vec![c2(100.0, "multiply"), c(100.0, "normal")],
                100.0,
                NORMAL,
                None,
            ),
        ),
        (
            "grpm2",
            group(
                MW,
                vec![c2(70.0, "multiply"), c(100.0, "normal")],
                100.0,
                NORMAL,
                None,
            ),
        ),
        (
            "grp",
            vec![M::Plain(vec![c2(100.0, "multiply"), c(100.0, "normal")])],
        ),
        (
            "nogrp",
            vec![M::Rect(c2(100.0, "multiply")), M::Rect(c(100.0, "normal"))],
        ),
        (
            "gam1u",
            vec![
                M::Rect(R {
                    rect: MR,
                    color: WHITE,
                    op: 50.0,
                    blend: mode("screen"),
                }),
                M::Rect(c(70.0, "multiply")),
            ],
        ),
    ]
}

fn render(doc: &Document, policy: CachePolicy) -> fp_raster::RgbaImage {
    Renderer::new(RenderOptions {
        cache_policy: policy,
        ..RenderOptions::default()
    })
    .render_page(doc, 0, 0)
    .image
}

#[test]
fn all_measured_cases_match_the_model_pixel_for_pixel() {
    let mut report = Vec::new();
    for (name, scene) in scenes() {
        let doc = build(&scene);
        for policy in [CachePolicy::None, CachePolicy::default()] {
            let img = render(&doc, policy);
            let mut bad = 0;
            let mut first = None;
            for y in 0..H {
                for x in 0..W {
                    let p = img.pixel(x, y);
                    let want = model(&scene, x, y);
                    if [p.r, p.g, p.b] != want {
                        bad += 1;
                        first.get_or_insert((x, y, [p.r, p.g, p.b], want));
                    }
                }
            }
            if bad > 0 {
                report.push(format!(
                    "{name} ({policy:?}): {bad} pixels, first {first:?}"
                ));
            }
        }
    }
    assert!(report.is_empty(), "{}", report.join("\n"));
}

/// The spec's measuring points (RE spec 75 section 2.3).
#[test]
fn measured_points() {
    let at = |name: &str, x: i32, y: i32| {
        let (_, sc) = scenes()
            .into_iter()
            .find(|(n, _)| *n == name)
            .expect("case");
        let p = render(&build(&sc), CachePolicy::None).pixel(x, y);
        [p.r, p.g, p.b]
    };
    // a child's blend mode is ignored inside a mask group: C 70 % Normal over the backdrop
    assert_eq!(
        at("gam2", 80, 100),
        [76, 128, 179],
        "Multiply would be (76,83,0)"
    );
    assert_eq!(
        at("iso2", 150, 100),
        [0, 192, 0],
        "C2 Multiply over C shows Normal"
    );
    assert_eq!(at("grpm2", 150, 100), [0, 173, 76], "C2 70 % Normal over C");
    // a plain group passes the blend mode through to the backdrop
    assert_eq!(at("grp", 250, 100), [0, 96, 0]);
    // the group's own opacity and blend mode act against the backdrop (FB-123)
    assert_eq!(at("gam2o", 80, 100), [183, 128, 72]);
    assert_eq!(at("gam2b", 80, 100), [148, 128, 0]);
    // a single content's blend mode applies to the masked result (FB-125)
    assert_eq!(at("single_m", 80, 100), [0, 64, 0]);
    // the underlay lies under the content inside the group
    assert_eq!(at("pin2", 80, 100), [76, 166, 255]);
}

/// `gam2b`: an existing group becomes the masked element. It keeps its values and is isolated
/// like a mask group with several contents (the children's blend modes are ignored).
#[test]
fn masked_existing_group_is_isolated_and_keeps_its_values() {
    let children = vec![c2(100.0, "multiply"), c(70.0, "normal")];
    let scene = group(MW, children.clone(), 60.0, mode("darken"), None);
    let mut doc = Document::new(W as f32, H as f32, Argb(0xffff_8000));
    let kids = children
        .iter()
        .map(|r| Arc::new(rect_el(&mut doc, *r)))
        .collect();
    let mut g = Element::new_group(&mut doc.ids, kids);
    g.common.opacity = Opacity::from_percent(60.0);
    g.common.blend = BlendMode(mode("darken"));
    let m = mask_el(&mut doc, MW);
    let mut masked = Element::new_mask_group(&mut doc.ids, m, vec![Arc::new(g)], true);
    assert!(
        masked.masked_single().is_some(),
        "the group is the masked element"
    );
    assert_eq!(masked.placement().0, Opacity::from_percent(60.0));
    if let ElementKind::Group(gr) = &mut masked.kind {
        gr.mask_flags.hide_attrs = true;
    }
    let page = Arc::make_mut(&mut doc.pages[0]);
    if let LayerSlot::Own(l) = &mut page.root.layers[0] {
        let cel = Arc::make_mut(&mut Arc::make_mut(l).cels[0]);
        cel.elements = vec![Arc::new(masked)];
    }
    let img = render(&doc, CachePolicy::None);
    let mut bad = 0;
    for y in 0..H {
        for x in 0..W {
            let p = img.pixel(x, y);
            bad += ([p.r, p.g, p.b] != model(&scene, x, y)) as u32;
        }
    }
    assert_eq!(bad, 0, "pixels off the model");
}
