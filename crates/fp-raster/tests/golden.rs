//! Golden-Tests gegen die Python-Referenzen (`tools\golden\fp-raster\gen_golden.py`):
//! Filter (`fwfilters.py`), Masken (`effekte\fx.py`: Dreiecksfilter, Morphologie) und
//! 9-Slice-Abtastung (`symbole\laufzeit\t2_9slice.py`). Pixelgenau, alle vier Kanäle.

mod common;

use std::path::PathBuf;

use fp_raster::filters::{Filter, PixelFilter};
use fp_raster::resample::{Sampler, nine_slice_scale};
use fp_raster::{IRect, Mask, RgbaImage};
use serde_json::Value;

fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join("golden")
}

fn origin(v: &Value, key: &str) -> (i32, i32) {
    let a = &v[key];
    (
        a[0].as_i64().unwrap_or(0) as i32,
        a[1].as_i64().unwrap_or(0) as i32,
    )
}

fn to_mask(img: &RgbaImage) -> Mask {
    Mask::from_raw(img.bounds(), img.data().iter().map(|p| p[0]).collect()).unwrap()
}

fn from_mask(m: &Mask) -> RgbaImage {
    RgbaImage::from_raw(
        m.bounds(),
        m.data().iter().map(|&v| [v, v, v, 255]).collect(),
    )
    .unwrap()
}

#[test]
fn golden_against_python_reference() {
    let dir = data_dir();
    let text = std::fs::read_to_string(dir.join("manifest.json"))
        .expect("manifest.json (gen_golden.py ausführen)");
    let cases: Vec<Value> = serde_json::from_str(&text).unwrap();
    assert!(cases.len() > 150);
    let mut failures = Vec::new();
    let (mut px_ok, mut px_all) = (0usize, 0usize);
    for c in &cases {
        let name = c["name"].as_str().unwrap();
        let (ix, iy) = origin(c, "input_origin");
        let (ox, oy) = origin(c, "output_origin");
        let input = common::load_png(&dir.join(c["input"].as_str().unwrap()), ix, iy);
        let expect = common::load_png(&dir.join(c["output"].as_str().unwrap()), ox, oy);
        let got = match c["kind"].as_str().unwrap() {
            "filter" => {
                let f: Filter = serde_json::from_value(c["filter"].clone())
                    .unwrap_or_else(|e| panic!("{name}: {e}"));
                f.apply(&input, None)
            }
            "mask" => {
                let m = to_mask(&input);
                let amt = c["amount"].as_u64().unwrap() as u32;
                let out = match c["op"].as_str().unwrap() {
                    "feather" => m.feather(amt),
                    "tent" => m.tent_blur(amt, amt, false),
                    "expand" => m.expand(amt, 1),
                    "contract" => m.contract(amt, 1),
                    "border" => m.border(amt, 1),
                    "smooth" => m.smooth(amt, 1),
                    o => panic!("{o}"),
                }
                .unwrap();
                let img = from_mask(&out);
                if c["compare"].as_str() == Some("input_bounds") {
                    img.cropped(input.bounds())
                } else {
                    img
                }
            }
            "nineslice" => {
                let ins: Vec<u32> = c["insets"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_u64().unwrap() as u32)
                    .collect();
                let size = &c["size"];
                nine_slice_scale(
                    &input,
                    [ins[0], ins[1], ins[2], ins[3]],
                    size[0].as_u64().unwrap() as u32,
                    size[1].as_u64().unwrap() as u32,
                    Sampler::Bicubic,
                )
            }
            k => panic!("{k}"),
        };
        let (eq, tot, mx) = common::strict_diff(&got, &expect);
        px_ok += eq;
        px_all += tot;
        if got.bounds() != expect.bounds() || eq != tot {
            failures.push(format!(
                "{name}: Grenzen {:?} / erwartet {:?}, {eq}/{tot} gleich, max {mx}",
                got.bounds(),
                expect.bounds()
            ));
        }
    }
    println!(
        "Golden: {} Fälle, {}/{} Pixel gleich",
        cases.len(),
        px_ok,
        px_all
    );
    assert!(
        failures.is_empty(),
        "{} von {} Fällen abweichend:\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n")
    );
}

#[test]
fn mask_frame_matches_t7_construction() {
    // weiche Auswahl über das Original: Kantenwerte liegen zwischen Original und Umkehrung
    let mut img = RgbaImage::new(IRect::new(10, 10, 110, 110)).unwrap();
    for y in 0..100 {
        for x in 0..100 {
            img.set_pixel(
                10 + x,
                10 + y,
                fp_color::Rgba8::rgb((x as f64 * 2.55) as u8, (y as f64 * 2.55) as u8, 128),
            );
        }
    }
    let m = Mask::from_rect(IRect::new(35, 35, 85, 85))
        .unwrap()
        .feather(10)
        .unwrap();
    let out = Filter::Invert.apply(&img, Some(&m));
    assert_eq!(out.bounds(), img.bounds());
    assert_eq!(out.pixel(60, 60).b, 127);
    assert_eq!(out.pixel(12, 12), img.pixel(12, 12));
}
