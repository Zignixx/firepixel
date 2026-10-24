//! FB-38: uniform dither density of a fill (Firepixel extension). The whole fill is quantised to
//! on/off pixels by the ordered 8 × 8 (Bayer) threshold on document coordinates at the given
//! density: 50 % is a checkerboard half tone of the fill colour over transparency, 0 and 100 leave
//! the fill unchanged, the edge (also the Dither edge) applies on top.

use fp_color::Argb;
use fp_geom::{IRect, Vec2};
use fp_vector::feather::{density_active, density_on};
use fp_vector::{
    Fill, FillEdge, FillKind, Gradient, GradientHandles, GradientShape, GradientStop, PathContour,
    VectorPath, render_path_pixels,
};

const WIN: IRect = IRect {
    left: 0,
    top: 0,
    right: 120,
    bottom: 100,
};

fn rect_path(edge: FillEdge, density: u8) -> VectorPath {
    let pts = vec![
        Vec2::new(20.0, 10.0),
        Vec2::new(100.0, 10.0),
        Vec2::new(100.0, 90.0),
        Vec2::new(20.0, 90.0),
    ];
    let mut p = VectorPath::new(vec![PathContour::new(pts, true)]);
    let mut f = Fill::solid(Argb(0xff20_4080));
    f.edge = edge;
    f.dither_density = density;
    p.fill = Some(f);
    p
}

fn px_at(px: &[[u8; 4]], x: i32, y: i32) -> [u8; 4] {
    px[((y - WIN.top) * WIN.width() as i32 + (x - WIN.left)) as usize]
}

/// Share of opaque pixels inside the rectangle 20..100 × 10..90.
fn on_share(px: &[[u8; 4]]) -> f64 {
    let (mut on, mut n) = (0usize, 0usize);
    for y in 10..90 {
        for x in 20..100 {
            n += 1;
            if px_at(px, x, y)[3] != 0 {
                on += 1;
            }
        }
    }
    on as f64 / n as f64
}

#[test]
fn density_function_counts_per_block() {
    for d in 0u8..=100 {
        let on = (0..8)
            .flat_map(|y| (0..8).map(move |x| (x, y)))
            .filter(|&(x, y)| density_on(d, x, y))
            .count();
        let want = (d as f64 * 64.0 / 100.0).round() as usize;
        assert!(
            (on as i64 - want as i64).abs() <= 1,
            "density {d}: {on} of 64 on, expected about {want}"
        );
    }
    assert!(!density_active(0) && !density_active(100) && density_active(50));
    // fixed to the document grid: period 8 in both directions, also for negative coordinates
    for (x, y) in [(-1, -1), (-9, 3), (5, -17)] {
        assert_eq!(density_on(37, x, y), density_on(37, x + 8, y + 16));
    }
}

#[test]
fn half_density_is_a_checkerboard_half_tone() {
    let (px, missing) = render_path_pixels(&rect_path(FillEdge::Hard, 50), WIN);
    assert!(missing.is_empty());
    let share = on_share(&px);
    assert!((share - 0.5).abs() <= 0.02, "on share {share}");
    // exactly a checkerboard: neighbours alternate, on pixels keep the full fill colour
    for y in 10..90 {
        for x in 20..100 {
            let c = px_at(&px, x, y);
            let on = (x + y) % 2 == 0;
            assert_eq!(c[3] != 0, on, "({x},{y})");
            if on {
                assert_eq!(c, [0x20, 0x40, 0x80, 255]);
            }
        }
    }
    // nothing outside the shape
    assert_eq!(px_at(&px, 10, 5)[3], 0);
}

#[test]
fn density_shares_follow_the_value() {
    for d in [10u8, 25, 75, 90] {
        let (px, _) = render_path_pixels(&rect_path(FillEdge::AntiAliased, d), WIN);
        let share = on_share(&px);
        assert!(
            (share - d as f64 / 100.0).abs() <= 0.02,
            "density {d}: share {share}"
        );
        assert!(px.iter().all(|c| c[3] == 0 || c[3] == 255));
    }
}

#[test]
fn zero_and_full_density_leave_the_fill_unchanged() {
    for edge in [
        FillEdge::Hard,
        FillEdge::AntiAliased,
        FillEdge::Feather(6),
        FillEdge::Dither(6),
    ] {
        let (plain, _) = render_path_pixels(&rect_path(edge, 0), WIN);
        let (full, _) = render_path_pixels(&rect_path(edge, 100), WIN);
        assert_eq!(plain, full, "{edge:?}");
    }
    // 100 is a solid fill: the inside is completely opaque
    let (full, _) = render_path_pixels(&rect_path(FillEdge::Hard, 100), WIN);
    assert_eq!(on_share(&full), 1.0);
}

#[test]
fn density_combines_with_the_dither_edge() {
    let r = 8;
    let (edge_only, _) = render_path_pixels(&rect_path(FillEdge::Dither(r), 0), WIN);
    let (both, _) = render_path_pixels(&rect_path(FillEdge::Dither(r), 50), WIN);
    assert!(both.iter().all(|c| c[3] == 0 || c[3] == 255));
    for y in WIN.top..WIN.bottom {
        for x in WIN.left..WIN.right {
            let (e, b) = (px_at(&edge_only, x, y)[3], px_at(&both, x, y)[3]);
            // a pixel is on exactly when the edge and the density both keep it
            assert_eq!(b != 0, e != 0 && density_on(50, x, y), "({x},{y})");
        }
    }
    // the edge band is thinner than with the edge alone, the core is the half tone
    let core = |px: &[[u8; 4]]| {
        let mut n = 0;
        for y in 40..60 {
            for x in 50..70 {
                n += (px_at(px, x, y)[3] != 0) as usize;
            }
        }
        n
    };
    assert_eq!(core(&edge_only), 400);
    assert_eq!(core(&both), 200);
}

#[test]
fn density_applies_to_gradients_and_patterns() {
    let mut p = rect_path(FillEdge::Hard, 25);
    if let Some(f) = &mut p.fill {
        f.kind = FillKind::Gradient(Gradient {
            shape: GradientShape::Linear,
            color_stops: vec![
                GradientStop {
                    pos: 0.0,
                    color: Argb(0xffff_0000),
                },
                GradientStop {
                    pos: 1.0,
                    color: Argb(0xff00_00ff),
                },
            ],
            opacity_stops: Vec::new(),
        });
        f.handles = GradientHandles {
            p1: Vec2::new(20.0, 50.0),
            p2: Vec2::new(100.0, 50.0),
            p3: Vec2::new(20.0, 10.0),
        };
    }
    let (px, missing) = render_path_pixels(&p, WIN);
    assert!(missing.is_empty());
    assert!((on_share(&px) - 0.25).abs() <= 0.02);
    // the kept pixels carry the gradient colours (red on the left, blue on the right)
    let first_on = |range: std::ops::Range<i32>| {
        range
            .flat_map(|x| (10..90).map(move |y| (x, y)))
            .map(|(x, y)| px_at(&px, x, y))
            .find(|c| c[3] != 0)
            .expect("an on pixel")
    };
    assert!(first_on(20..24)[0] > 200);
    assert!(first_on(96..100)[2] > 200);
    // pattern fills keep their texels on the kept pixels
    let tile =
        fp_raster::RgbaImage::from_raw(IRect::new(0, 0, 1, 1), vec![[1, 2, 3, 255]]).expect("tile");
    let mut q = rect_path(FillEdge::Hard, 50);
    if let Some(f) = &mut q.fill {
        f.kind = FillKind::Pattern(std::sync::Arc::new(tile));
        f.handles = GradientHandles {
            p1: Vec2::new(0.0, 0.0),
            p2: Vec2::new(1.0, 0.0),
            p3: Vec2::new(0.0, -1.0),
        };
    }
    let (qx, _) = render_path_pixels(&q, WIN);
    assert!((on_share(&qx) - 0.5).abs() <= 0.02);
    assert!(qx.iter().all(|c| c[3] == 0 || *c == [1, 2, 3, 255]));
}
