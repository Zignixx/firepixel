//! TW-203 (rest of FB-23): text objects honour the fill edge kinds like paths. Hard has no partial
//! alpha, Anti-Alias a ramp of at most 1 px, Feather the triangle-filter ramp of its width, and
//! Dither (Firepixel extension) the feather ramp quantised by the Bayer 8 × 8 threshold on
//! document pixels (only 0/255 alpha, about the coverage of the feather).

use std::sync::Arc;

use fp_color::Argb;
use fp_geom::Point;
use fp_model::{Document, Element, ElementKind, Fill, LayerSlot};
use fp_render::{CachePolicy, RenderOptions, Renderer};

const W: i32 = 260;
const H: i32 = 120;

/// Alpha of every page pixel (row by row) of a text "Ag" (ArialMT 60 px, opaque) with the given
/// fill edge, or `None` when the system has no usable font.
fn render(edge: i32, feather: i32) -> Option<Vec<u8>> {
    let mut doc = Document::new(W as f32, H as f32, Argb(0x00ff_ffff));
    let mut e = Element::new_text(
        &mut doc.ids,
        "Ag",
        "ArialMT",
        60.0,
        Argb(0xff20_4060),
        Point::new(40.0, 80.0),
    );
    if let ElementKind::Text(t) = &mut e.kind {
        let a = Arc::make_mut(&mut t.attrs);
        let f = a.fill.as_mut().expect("solid fill");
        f.edge = edge;
        f.feather = feather;
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
    let a: Vec<u8> = (0..H)
        .flat_map(|y| (0..W).map(move |x| (x, y)))
        .map(|(x, y)| {
            if img.bounds().contains(x, y) {
                img.pixel(x, y).a
            } else {
                0
            }
        })
        .collect();
    a.iter().any(|&v| v != 0).then_some(a)
}

fn partial(a: &[u8]) -> usize {
    a.iter().filter(|&&v| v != 0 && v != 255).count()
}

/// Widest ramp: the largest Chebyshev distance from a partial-alpha pixel to the nearest pixel
/// with alpha 0 or 255.
fn ramp_width(a: &[u8]) -> i32 {
    let at = |x: i32, y: i32| -> u8 {
        if (0..W).contains(&x) && (0..H).contains(&y) {
            a[(y * W + x) as usize]
        } else {
            0
        }
    };
    let mut widest = 0;
    for y in 0..H {
        for x in 0..W {
            let v = at(x, y);
            if v == 0 || v == 255 {
                continue;
            }
            let d = (1i32..=20)
                .find(|&d| {
                    (-d..=d).any(|dy: i32| {
                        (-d..=d).any(|dx: i32| {
                            (dx.abs() == d || dy.abs() == d) && {
                                let u = at(x + dx, y + dy);
                                u == 0 || u == 255
                            }
                        })
                    })
                })
                .unwrap_or(21);
            widest = widest.max(d);
        }
    }
    widest
}

#[test]
fn text_hard_edge_has_no_partial_alpha() {
    let Some(hard) = render(Fill::EDGE_HARD, 0) else {
        eprintln!("skipped: no usable system font");
        return;
    };
    assert_eq!(partial(&hard), 0);
    // dither width 0 is Hard, as for paths
    assert_eq!(render(Fill::EDGE_DITHER, 0).expect("ink"), hard);
}

#[test]
fn text_antialias_edge_ramp_is_at_most_one_pixel() {
    let Some(aa) = render(Fill::EDGE_ANTIALIAS, 0) else {
        eprintln!("skipped: no usable system font");
        return;
    };
    assert!(partial(&aa) > 0);
    assert_eq!(ramp_width(&aa), 1);
    // a stale width next to edge 1 (the anti-aliasing level menu keeps it) changes nothing
    assert_eq!(render(Fill::EDGE_ANTIALIAS, 6).expect("ink"), aa);
}

#[test]
fn text_feather_edge_is_a_soft_ramp() {
    let Some(f) = render(Fill::EDGE_FEATHER, 4) else {
        eprintln!("skipped: no usable system font");
        return;
    };
    let aa = render(Fill::EDGE_ANTIALIAS, 0).expect("ink");
    assert!(
        partial(&f) > 2 * partial(&aa),
        "{} vs {}",
        partial(&f),
        partial(&aa)
    );
    assert!(ramp_width(&f) >= 3, "ramp {}", ramp_width(&f));
    // the ramp reaches beyond the glyphs: ink where the anti-aliased text has none
    let outside = f
        .iter()
        .zip(&aa)
        .filter(|&(&fv, &av)| fv != 0 && av == 0)
        .count();
    assert!(outside > 0);
}

#[test]
fn text_dither_edge_is_the_bayer_quantised_feather() {
    let Some(d) = render(Fill::EDGE_DITHER, 4) else {
        eprintln!("skipped: no usable system font");
        return;
    };
    let f = render(Fill::EDGE_FEATHER, 4).expect("ink");
    assert!(d.iter().all(|&a| a == 0 || a == 255));
    // every pixel is the feather coverage through the ordered dither on document coordinates
    for y in 0..H {
        for x in 0..W {
            let c = f[(y * W + x) as usize];
            let want = if c == 0 || c == 255 {
                c
            } else if fp_vector::feather::dither_on(c, x, y) {
                255
            } else {
                0
            };
            assert_eq!(
                d[(y * W + x) as usize],
                want,
                "pixel ({x}, {y}), feather {c}"
            );
        }
    }
    // about the coverage of the feather
    let full = d.iter().filter(|&&a| a == 255).count() as f64;
    let mass = f.iter().map(|&a| a as f64).sum::<f64>() / 255.0;
    assert!(
        (full - mass).abs() < mass * 0.05,
        "dither {full} vs feather {mass}"
    );
}
