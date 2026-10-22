//! FB-23 / TW-203: the "Dither" fill edge (Firepixel extension). The feather ramp of the same width
//! is quantised to fully on / off pixels by an ordered 8 × 8 threshold on document pixel
//! coordinates: no partial alpha in the band, width 0 equals Hard, the pattern is fixed to the
//! document grid.

use fp_color::Argb;
use fp_geom::{IRect, Vec2};
use fp_vector::feather::dither_on;
use fp_vector::render::fill_mask;
use fp_vector::{Fill, FillEdge, PathContour, VectorPath, render_path_pixels};

fn rect_path(l: f64, t: f64, r: f64, b: f64, edge: FillEdge, color: u32) -> VectorPath {
    let pts = vec![
        Vec2::new(l, t),
        Vec2::new(r, t),
        Vec2::new(r, b),
        Vec2::new(l, b),
    ];
    let mut p = VectorPath::new(vec![PathContour::new(pts, true)]);
    let mut f = Fill::solid(Argb(color));
    f.edge = edge;
    p.fill = Some(f);
    p
}

fn alpha_at(px: &[[u8; 4]], win: IRect, x: i32, y: i32) -> u8 {
    let w = win.width() as i32;
    px[((y - win.top) * w + (x - win.left)) as usize][3]
}

const WIN: IRect = IRect {
    left: 0,
    top: 0,
    right: 120,
    bottom: 100,
};

#[test]
fn dither_band_has_only_full_or_empty_pixels() {
    let r = 8;
    let p = rect_path(30.0, 20.0, 90.0, 80.0, FillEdge::Dither(r), 0xffff_0000);
    let (px, missing) = render_path_pixels(&p, WIN);
    assert!(missing.is_empty());
    assert!(
        px.iter().all(|c| c[3] == 0 || c[3] == 255),
        "dither leaves no partial alpha"
    );
    // the same shape with Feather has partial alpha in the band (sanity)
    let f = rect_path(30.0, 20.0, 90.0, 80.0, FillEdge::Feather(r), 0xffff_0000);
    let (fpx, _) = render_path_pixels(&f, WIN);
    assert!(fpx.iter().any(|c| c[3] != 0 && c[3] != 255));
}

#[test]
fn dither_band_spans_the_feather_ramp_of_the_amount() {
    let r = 8i32;
    let p = rect_path(
        30.0,
        20.0,
        90.0,
        80.0,
        FillEdge::Dither(r as u32),
        0xff00_00ff,
    );
    let (px, _) = render_path_pixels(&p, WIN);
    // columns across the left edge (x = 30), rows well inside the shape vertically
    let rows = 35..65;
    let mut band = Vec::new();
    for x in 0..60 {
        let on = rows
            .clone()
            .filter(|&y| alpha_at(&px, WIN, x, y) == 255)
            .count();
        let n = rows.len();
        if x < 30 - r {
            assert_eq!(on, 0, "outside the band left of the edge: x = {x}");
        } else if x >= 30 + r {
            assert_eq!(on, n, "inside the shape beyond the band: x = {x}");
        } else if on != 0 && on != n {
            band.push(x);
        }
    }
    // the dithered transition reaches at most `amount` px to each side of the edge (like Feather)
    // and is at least `amount` px wide
    let (first, last) = (band[0], band[band.len() - 1]);
    assert!(first >= 30 - r && last < 30 + r, "{band:?}");
    assert!(
        last - first + 1 >= r,
        "band {band:?} narrower than the amount {r}"
    );
    // on-density rises across the band (more pixels on towards the inside), in 4-column steps
    let density = |x0: i32| {
        (x0..x0 + 4)
            .flat_map(|x| rows.clone().map(move |y| (x, y)))
            .filter(|&(x, y)| alpha_at(&px, WIN, x, y) == 255)
            .count()
    };
    let d: Vec<usize> = (0..4).map(|k| density(30 - r + 4 * k)).collect();
    assert!(
        d.windows(2).all(|w| w[0] < w[1]),
        "density across the band {d:?}"
    );
}

#[test]
fn dither_amount_zero_equals_hard() {
    // fractional coordinates so Hard and Anti-Alias differ
    let d = rect_path(30.3, 20.6, 90.7, 80.2, FillEdge::Dither(0), 0xff12_3456);
    let h = rect_path(30.3, 20.6, 90.7, 80.2, FillEdge::Hard, 0xff12_3456);
    assert_eq!(render_path_pixels(&d, WIN).0, render_path_pixels(&h, WIN).0);
    assert_eq!(FillEdge::from_tags(3, 0), FillEdge::Hard);
    assert_eq!(FillEdge::from_tags(3, 6), FillEdge::Dither(6));
    assert_eq!(FillEdge::from_tags(2, 6), FillEdge::Feather(6));
}

#[test]
fn dither_pattern_is_fixed_to_the_document_grid() {
    let r = 6u32;
    for (dx, dy) in [(0i32, 0i32), (3, 5), (8, 8), (-2, 1)] {
        let (l, t) = (30.0 + dx as f64, 20.0 + dy as f64);
        let p = rect_path(l, t, l + 50.0, t + 40.0, FillEdge::Dither(r), 0xff00_ff00);
        let ramp = fill_mask(&p, FillEdge::Feather(r), WIN);
        let (px, _) = render_path_pixels(&p, WIN);
        for y in WIN.top..WIN.bottom {
            for x in WIN.left..WIN.right {
                let c = ramp.data[(y * WIN.width() as i32 + x) as usize];
                let want = if dither_on(c, x, y) { 255 } else { 0 };
                assert_eq!(
                    alpha_at(&px, WIN, x, y),
                    want,
                    "({x},{y}) shift ({dx},{dy})"
                );
            }
        }
    }
    // a whole-period move (8 px) keeps the pattern relative to the shape, a 3 px move does not
    let render = |dx: i32| {
        let l = 30.0 + dx as f64;
        let p = rect_path(l, 20.0, l + 50.0, 60.0, FillEdge::Dither(r), 0xff00_ff00);
        render_path_pixels(&p, WIN).0
    };
    let (a, b8, b3) = (render(0), render(8), render(3));
    let rel = |px: &[[u8; 4]], dx: i32| -> Vec<u8> {
        (14..66)
            .flat_map(|y| (20..90).map(move |x| (x, y)))
            .map(|(x, y)| alpha_at(px, WIN, x + dx, y))
            .collect()
    };
    assert_eq!(rel(&a, 0), rel(&b8, 8));
    assert_ne!(rel(&a, 0), rel(&b3, 3));
}

#[test]
fn dither_is_independent_of_the_render_window() {
    // scrolling = another window over the same document pixels
    let p = rect_path(30.0, 20.0, 90.0, 80.0, FillEdge::Dither(7), 0xff00_ff00);
    let other = IRect::new(13, 9, 107, 95);
    let (a, _) = render_path_pixels(&p, WIN);
    let (b, _) = render_path_pixels(&p, other);
    for y in other.top..other.bottom {
        for x in other.left..other.right {
            assert_eq!(
                alpha_at(&a, WIN, x, y),
                alpha_at(&b, other, x, y),
                "({x},{y})"
            );
        }
    }
}

#[test]
fn fill_alpha_applies_after_the_dither() {
    let p = rect_path(30.0, 20.0, 90.0, 80.0, FillEdge::Dither(8), 0x80ff_0000);
    let (px, _) = render_path_pixels(&p, WIN);
    assert!(
        px.iter().all(|c| c[3] == 0 || c[3] == 128),
        "on pixels carry the fill alpha"
    );
    assert!(px.iter().any(|c| c[3] == 128));
}
