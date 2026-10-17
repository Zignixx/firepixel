//! BA-012 (re 29 §9): Bewegungsunschärfe, 22 Messfälle aus dem Original. B1 = 320,200,420,300,
//! deckend; Ergebnis-Grenzen nach dem Menübefehl (Zuwachs beschnitten).

use fp_raster::filters::{MotionBlurParams, PixelFilter, trim_growth};
use fp_raster::{IRect, RgbaImage};

fn bounds(angle: i32, distance: i32) -> IRect {
    let b1 = IRect::new(320, 200, 420, 300);
    let img = RgbaImage::from_raw(b1, vec![[48, 96, 192, 255]; 100 * 100]).unwrap();
    trim_growth(
        img.bounds(),
        &MotionBlurParams { angle, distance }.apply(&img, None),
    )
    .bounds()
}

#[test]
fn ba012_motion_blur_reach_matches_measurements() {
    let r = IRect::new;
    let cases: &[(i32, i32, IRect)] = &[
        (0, 20, r(300, 200, 420, 300)),
        (360, 20, r(300, 200, 420, 300)),
        (90, 20, r(320, 200, 420, 319)),
        (180, 20, r(320, 200, 439, 300)),
        (270, 20, r(320, 181, 420, 300)),
        (45, 20, r(306, 200, 420, 314)),
        (1, 20, r(301, 200, 420, 300)),
        (359, 20, r(301, 200, 420, 300)),
        (89, 20, r(320, 200, 420, 319)),
        (91, 20, r(320, 200, 420, 319)),
        (0, 1, r(319, 200, 420, 300)),
        (0, 2, r(318, 200, 420, 300)),
        (0, 10, r(310, 200, 420, 300)),
        (0, 21, r(299, 200, 420, 300)),
        (0, 100, r(220, 200, 420, 300)),
        (90, 1, r(320, 200, 420, 300)),
        (180, 1, r(320, 200, 420, 300)),
        (270, 1, r(320, 200, 420, 300)),
        (90, 100, r(320, 200, 420, 399)),
    ];
    for &(a, d, want) in cases {
        assert_eq!(bounds(a, d), want, "Winkel {a}, Umfang {d}");
    }
}
