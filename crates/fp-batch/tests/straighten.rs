//! `perspective` with automatic straightening (B-4, `docs/BATCH.md` §3.3).

use fp_batch::item::{Item, ItemInfo, ItemKind, still_from_rgba};
use fp_batch::steps::straighten::detect;
use fp_batch::{CancelToken, OutputFormat, StepContext, StepRegistry};
use serde_json::{Value, json};
use std::path::PathBuf;

/// White image with dark lines: horizontals tilted by `h` degrees, verticals by `v` degrees
/// (dx/dy), 32 px apart, antialiased.
fn lines(w: u32, hgt: u32, h: f64, v: f64) -> Item {
    let (th, tv) = (h.to_radians().tan(), v.to_radians().tan());
    let (ch, cv) = (h.to_radians().cos(), v.to_radians().cos());
    let mut px = Vec::with_capacity((w * hgt * 4) as usize);
    for y in 0..hgt {
        for x in 0..w {
            let (fx, fy) = (x as f64 + 0.5, y as f64 + 0.5);
            let dh = {
                let r = (fy - th * fx).rem_euclid(32.0);
                r.min(32.0 - r) * ch
            };
            let dv = {
                let r = (fx - tv * fy).rem_euclid(32.0);
                r.min(32.0 - r) * cv
            };
            let cov = (1.5 - dh.min(dv)).clamp(0.0, 1.0);
            let g = (255.0 * (1.0 - cov)).round() as u8;
            px.extend_from_slice(&[g, g, g, 255]);
        }
    }
    still_from_rgba(w, hgt, &px).expect("item")
}

fn apply(params: Value, item: &mut Item) -> Vec<String> {
    let reg = StepRegistry::global();
    let step = reg.get("perspective").expect("step");
    let p = fp_batch::step::with_defaults(&step.params(), &params);
    let (w, h) = item.dimensions();
    let info = ItemInfo {
        path: PathBuf::from("C:/in/a.png"),
        root: None,
        kind: ItemKind::Image,
        format: "png".into(),
        bytes: 0,
        width: w,
        height: h,
        frames: 1,
        modified: None,
        exif: None,
    };
    let cancel = CancelToken::new();
    let mut ctx = StepContext::new(0, 1, &info, &cancel, OutputFormat::Png);
    step.apply(item, &p, &mut ctx).expect("apply");
    ctx.warnings
}

fn near(a: Option<f64>, want: f64, tol: f64) -> bool {
    a.is_some_and(|a| (a - want).abs() <= tol)
}

#[test]
fn detects_tilt_of_both_families() {
    let it = lines(400, 300, 3.0, -3.0);
    let s = detect(&it.preview_image().expect("img"), 10.0);
    assert!(near(s.horizontal, 3.0, 0.3), "{s:?}");
    assert!(near(s.vertical, -3.0, 0.3), "{s:?}");
    assert!(near(s.rotation(), 3.0, 0.3), "{s:?}");
    // the maximum angle limits the result
    let s = detect(&it.preview_image().expect("img"), 1.0);
    assert!(s.rotation().is_none_or(|a| a.abs() <= 1.0), "{s:?}");
}

#[test]
fn rotate_straightens_and_keeps_the_size() {
    let mut it = lines(400, 300, 4.0, -4.0);
    let w = apply(json!({"straighten": "rotate"}), &mut it);
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(it.dimensions(), (400, 300));
    let img = it.preview_image().expect("img");
    // cropped: no transparent corners
    assert!(img.data().iter().all(|p| p[3] == 255));
    let s = detect(&img, 10.0);
    assert!(near(s.rotation(), 0.0, 0.4), "{s:?}");
}

#[test]
fn shear_corrects_each_family() {
    let mut it = lines(400, 300, 2.0, 5.0);
    apply(json!({"straighten": "shear", "crop": false}), &mut it);
    let s = detect(&it.preview_image().expect("img"), 10.0);
    assert!(near(s.horizontal, 0.0, 0.4), "{s:?}");
    assert!(near(s.vertical, 0.0, 0.4), "{s:?}");
}

#[test]
fn flat_image_stays_unchanged_with_a_warning() {
    let px = vec![200u8; 64 * 32 * 4];
    let mut it = still_from_rgba(64, 32, &px).expect("item");
    let before = it.preview_image().expect("img");
    let w = apply(json!({"straighten": "rotate"}), &mut it);
    assert_eq!(w.len(), 1);
    assert_eq!(it.preview_image().expect("img"), before);
}
