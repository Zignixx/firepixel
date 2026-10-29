//! FB-38 (Firepixel extensions): the uniform dither density of a model fill and the Screentone
//! live filter (`{firepixel-screentone}`) rendered through the page renderer.

use std::sync::Arc;

use fp_color::Argb;
use fp_geom::Rect;
use fp_model::effects::{Effect, EffectList, SCREENTONE_MOA_ID};
use fp_model::{Document, Element, LayerSlot, PathAttrs};
use fp_render::{CachePolicy, RenderOptions, Renderer};

/// Square 32..96 × 16..80 (64 × 64 px) on a transparent 128 × 96 page.
fn render(
    color: u32,
    edge: i32,
    feather: i32,
    density: i32,
    effects: Vec<Effect>,
) -> fp_raster::RgbaImage {
    let mut doc = Document::new(128.0, 96.0, Argb(0x00ff_ffff));
    let mut attrs = PathAttrs::solid_fill(Argb(color));
    if let Some(f) = attrs.fill.as_mut() {
        f.edge = edge;
        f.feather = feather;
        f.dither_density = density;
    }
    let mut e = Element::new_rectangle(&mut doc.ids, Rect::new(32.0, 16.0, 96.0, 80.0), 0.0, attrs);
    if !effects.is_empty() {
        e.common.effects = Some(EffectList {
            effects,
            ..EffectList::default()
        });
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

fn screentone(pairs: &[(&str, &str)]) -> Effect {
    let mut e = Effect::new(SCREENTONE_MOA_ID, true);
    for (k, v) in pairs {
        e.set(k, *v);
    }
    e
}

/// Share of inked (non-transparent) pixels inside the square.
fn share(img: &fp_raster::RgbaImage) -> f64 {
    let mut on = 0usize;
    for y in 16..80 {
        for x in 32..96 {
            on += (img.pixel(x, y).a != 0) as usize;
        }
    }
    on as f64 / (64.0 * 64.0)
}

fn pixels(img: &fp_raster::RgbaImage) -> Vec<[u8; 4]> {
    (16..80)
        .flat_map(|y| (32..96).map(move |x| (x, y)))
        .map(|(x, y)| img.pixel(x, y).to_array())
        .collect()
}

const HARD: i32 = fp_model::Fill::EDGE_HARD;

#[test]
fn fill_density_half_is_the_bayer_half_pattern() {
    let img = render(0xff33_6699, HARD, 0, 50, Vec::new());
    let s = share(&img);
    assert!((s - 0.5).abs() <= 0.02, "share {s}");
    for y in 16..80 {
        for x in 32..96 {
            let p = img.pixel(x, y);
            assert_eq!(p.a != 0, (x + y) % 2 == 0, "({x},{y})");
            if p.a != 0 {
                assert_eq!((p.r, p.g, p.b, p.a), (0x33, 0x66, 0x99, 255));
            }
        }
    }
}

#[test]
fn fill_density_0_50_100_on_a_solid_fill() {
    // FB-109: the tester saw no difference between 0 and 100 on a solid fill (100 used to mean
    // "all pixels kept"); the value is now the share of empty pixels and acts monotonically
    let off = render(0xff33_6699, HARD, 0, 0, Vec::new());
    let half = render(0xff33_6699, HARD, 0, 50, Vec::new());
    let full = render(0xff33_6699, HARD, 0, 100, Vec::new());
    assert_eq!(share(&off), 1.0, "0 = off: the plain solid fill");
    assert!((share(&half) - 0.5).abs() <= 0.02, "50 = checkerboard");
    assert_eq!(share(&full), 0.0, "100 = every pixel empty");
    assert_ne!(pixels(&off), pixels(&full));
    // anti-aliased edge: 0 is exactly the plain fill
    let plain = render(0xff33_6699, 1, 0, 0, Vec::new());
    assert!(pixels(&plain).iter().all(|p| p[3] == 255));
    let mut last = 1.1;
    for d in [10, 25, 50, 75, 90, 100] {
        let s = share(&render(0xff33_6699, HARD, 0, d, Vec::new()));
        assert!(s < last, "density {d}: {s} not below {last}");
        last = s;
    }
}

#[test]
fn fill_density_with_dither_edge() {
    let both = render(0xff33_6699, fp_model::Fill::EDGE_DITHER, 6, 50, Vec::new());
    let edge = render(0xff33_6699, fp_model::Fill::EDGE_DITHER, 6, 0, Vec::new());
    let (b, e) = (pixels(&both), pixels(&edge));
    assert!(b.iter().all(|p| p[3] == 0 || p[3] == 255));
    // every pixel of the combination is also on with the edge alone; both use the same ordered
    // matrix, so the result is the lower of the two tones: the half tone inside the square (where
    // the edge ramp is at least 50 %), thinning out towards the outside of the band
    assert!(b.iter().zip(&e).all(|(b, e)| b[3] == 0 || e[3] != 0));
    let (sb, se) = (share(&both), share(&edge));
    assert!(se > 0.9 && (sb - 0.5).abs() <= 0.02, "{sb} vs {se}");
    assert!(both.pixel(29, 48).a == 0 || edge.pixel(29, 48).a != 0);
    let outside = |img: &fp_raster::RgbaImage| {
        (26..32)
            .map(|x| (img.pixel(x, 48).a != 0) as u32)
            .sum::<u32>()
    };
    assert!(outside(&both) <= outside(&edge));
}

#[test]
fn screentone_on_half_grey_gives_half_coverage_dots() {
    let img = render(
        0xff80_8080,
        HARD,
        0,
        0,
        vec![screentone(&[("screentone_size", "8")])],
    );
    let s = share(&img);
    assert!((s - 0.5).abs() <= 0.05, "coverage {s}");
    // black ink only, no partial alpha
    assert!(
        pixels(&img)
            .iter()
            .all(|p| p[3] == 0 || *p == [0, 0, 0, 255])
    );
    // outside the object nothing is drawn
    assert_eq!(img.pixel(10, 10).a, 0);
    // white gives no ink, black is fully inked
    assert_eq!(
        share(&render(0xffff_ffff, HARD, 0, 0, vec![screentone(&[])])),
        0.0
    );
    assert_eq!(
        share(&render(0xff00_0000, HARD, 0, 0, vec![screentone(&[])])),
        1.0
    );
}

#[test]
fn screentone_dot_coverage_follows_the_grey() {
    for (grey, want) in [(0xc0u32, 0.247), (0x40, 0.749)] {
        let c = 0xff00_0000 | grey << 16 | grey << 8 | grey;
        let s = share(&render(
            c,
            HARD,
            0,
            0,
            vec![screentone(&[("screentone_size", "8")])],
        ));
        assert!((s - want).abs() <= 0.05, "grey {grey:#x}: {s} vs {want}");
    }
}

#[test]
fn screentone_angle_changes_the_screen() {
    let a0 = render(
        0xff80_8080,
        HARD,
        0,
        0,
        vec![screentone(&[("screentone_angle", "0")])],
    );
    let a45 = render(
        0xff80_8080,
        HARD,
        0,
        0,
        vec![screentone(&[("screentone_angle", "45")])],
    );
    assert_ne!(pixels(&a0), pixels(&a45));
    assert!((share(&a0) - share(&a45)).abs() <= 0.05);
    // at 0° the dot screen repeats every 8 px in x and y
    for y in 16..72 {
        for x in 32..88 {
            assert_eq!(a0.pixel(x, y).a, a0.pixel(x + 8, y + 8).a, "({x},{y})");
        }
    }
}

#[test]
fn screentone_patterns_are_distinct() {
    let st = |kind: &str| {
        render(
            0xff80_8080,
            HARD,
            0,
            0,
            vec![screentone(&[
                ("screentone_type", kind),
                ("screentone_size", "32"),
                ("screentone_angle", "0"),
                ("screentone_from_luminance", "false"),
                ("screentone_density", "40"),
            ])],
        )
    };
    let (dots, lines, cross) = (st("0"), st("1"), st("2"));
    let (pd, pl, pc) = (pixels(&dots), pixels(&lines), pixels(&cross));
    assert_ne!(pd, pl);
    assert_ne!(pd, pc);
    assert_ne!(pl, pc);
    // constant density: every pattern inks about 40 %
    for (n, img) in [("dots", &dots), ("lines", &lines), ("cross", &cross)] {
        let s = share(img);
        assert!((s - 0.4).abs() <= 0.05, "{n}: {s}");
    }
    // lines at 0° run horizontally: every row is either fully inked or empty
    for y in 16..80 {
        let row: Vec<u8> = (32..96).map(|x| lines.pixel(x, y).a).collect();
        assert!(row.iter().all(|&a| a == row[0]), "row {y}");
    }
    // cross-hatch at 0° has fully inked rows and columns
    assert!((16..80).any(|y| (32..96).all(|x| cross.pixel(x, y).a != 0)));
    assert!((32..96).any(|x| (16..80).all(|y| cross.pixel(x, y).a != 0)));
}

#[test]
fn screentone_colour_and_keep_colour() {
    let red = render(
        0xffd0_2010,
        HARD,
        0,
        0,
        vec![screentone(&[("screentone_keep_color", "true")])],
    );
    let p = pixels(&red);
    assert!(p.iter().any(|q| q[3] != 0));
    assert!(
        p.iter().all(|q| q[3] == 0 || *q == [0xd0, 0x20, 0x10, 255]),
        "keep colour inks with the object's own colour (same hue)"
    );
    let blue_ink = render(
        0xff80_8080,
        HARD,
        0,
        0,
        vec![screentone(&[("screentone_color", "#0000ff")])],
    );
    assert!(
        pixels(&blue_ink)
            .iter()
            .all(|q| q[3] == 0 || *q == [0, 0, 255, 255])
    );
}
