//! DLG-6: Grenzen der Ergebnis-Bitmap der Weichzeichner beim Menübefehl „Filter“
//! (PARITAET_B D47–D50, gemessen von PM-6 am Original: B1 = 320,200,420,300, deckend).

use fp_raster::filters::{trim_growth, GaussianBlurParams, MotionBlurParams, PixelFilter, RadialBlurParams, ZoomBlurParams};
use fp_raster::{IRect, RgbaImage};

const B1: IRect = IRect { left: 320, top: 200, right: 420, bottom: 300 };

fn b1() -> RgbaImage {
    RgbaImage::from_raw(B1, vec![[200, 40, 40, 255]; 100 * 100]).unwrap()
}

fn menu<F: PixelFilter>(f: F, img: &RgbaImage) -> RgbaImage {
    trim_growth(img.bounds(), &f.apply(img, None))
}

#[test]
fn d47_motion_blur_grows_only_along_the_motion() {
    let out = menu(MotionBlurParams { angle: 90, distance: 20 }, &b1());
    assert_eq!(out.bounds(), IRect::new(320, 200, 420, 319));
    // die letzte Zeile hat genau eine deckende Probe von 20 (Proben i = 0 … 19)
    assert_eq!(out.pixel(370, 318).a, 255 / 20);
    assert_eq!(out.pixel(370, 250).a, 255);
    // Winkel 0: nach rechts (Probe x + i), Winkel 180 nach links
    assert_eq!(menu(MotionBlurParams { angle: 0, distance: 20 }, &b1()).bounds(), IRect::new(301, 200, 420, 300));
    assert_eq!(menu(MotionBlurParams { angle: 180, distance: 20 }, &b1()).bounds(), IRect::new(320, 200, 439, 300));
    // Vorgabe 90/10
    assert_eq!(menu(MotionBlurParams::default(), &b1()).bounds(), IRect::new(320, 200, 420, 309));
}

#[test]
fn d49_radial_blur_bounds_like_the_original() {
    let full = RadialBlurParams { amount: 10, quality: 20 }.apply(&b1(), None);
    assert_eq!(full.bounds(), IRect::new(299, 179, 441, 321), "Puffer (06 §4.12)");
    let out = menu(RadialBlurParams { amount: 10, quality: 20 }, &b1());
    assert_eq!(out.bounds(), IRect::new(317, 197, 424, 304));
}

#[test]
fn d48_d50_bounds_unchanged() {
    assert_eq!(menu(GaussianBlurParams { radius: 2.0, ..Default::default() }, &b1()).bounds(), IRect::new(315, 195, 425, 305));
    assert_eq!(menu(ZoomBlurParams { amount: 10, quality: 20, center: None }, &b1()).bounds(), B1);
}

#[test]
fn trim_keeps_input_bounds_with_transparent_margin() {
    // Bitmap mit durchsichtigem Rand: die Eingabegrenzen bleiben, nur der Zuwachs wird beschnitten
    let mut img = RgbaImage::new(IRect::new(0, 0, 40, 40)).unwrap();
    for y in 10..30 {
        for x in 10..30 {
            img.set_pixel(x, y, fp_color::Rgba8::new(0, 0, 255, 255));
        }
    }
    let out = menu(MotionBlurParams { angle: 90, distance: 5 }, &img);
    assert_eq!(out.bounds(), IRect::new(0, 0, 40, 40));
    let empty = RgbaImage::new(IRect::new(5, 5, 15, 15)).unwrap();
    assert_eq!(menu(RadialBlurParams::default(), &empty).bounds(), IRect::new(5, 5, 15, 15));
}
