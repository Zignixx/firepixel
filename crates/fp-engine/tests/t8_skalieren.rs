//! T-8 (re 35 §3, TW-75): scaling a bitmap uses the separable bicubic filter of the original.
//! Golden image: run 7 `inspector-bitmap/03` (B1 480 × 320 → width 240), export of the original
//! (`files\03_resize_result_orig.png`). Missing files (other machine) skip the test.

use crate::common::*;
use serde_json::json;
use std::path::PathBuf;

fn par_dir() -> PathBuf {
    PathBuf::from(r"C:\Users\Administrator\Desktop\REA\re\_testdateien\par")
}

fn load_png(p: &std::path::Path) -> Option<fp_raster::RgbaImage> {
    let data = std::fs::read(p).ok()?;
    fp_io::png_in::decode_png(&data).ok().map(|r| r.image)
}

#[test]
fn tw75_inspector_bitmap_03_pixel_equal_to_original() {
    let src = par_dir().join("par_bitmap.fw.png");
    let golden = par_dir().join(r"run_lauf7_2026-10-16\inspector-bitmap\files\03_resize_result_orig.png");
    let Some(want) = load_png(&golden) else {
        eprintln!("skipped: {golden:?} missing");
        return;
    };
    if !src.is_file() {
        eprintln!("skipped: {src:?} missing");
        return;
    }
    let p = std::env::temp_dir().join(format!("fp-t8-{}-par_bitmap.fw.png", std::process::id()));
    std::fs::copy(&src, &p).expect("copy");
    let mut s = doc_session(10.0, 10.0);
    ex(&mut s, "file.open", json!({"path": p.to_string_lossy()}));
    let _ = std::fs::remove_file(&p);
    let b1 = {
        let d = s.active().expect("doc");
        let mut id = None;
        for (_, l) in fp_engine::docops::own_layers(&d.doc, 0) {
            for cel in &l.cels {
                for e in &cel.elements {
                    if e.common.name.as_deref() == Some("B1") {
                        id = Some(e.id);
                    }
                }
            }
        }
        id.expect("B1")
    };
    select(&mut s, &[b1]);
    ex(&mut s, "element.setBounds", json!({"width": 240}));
    let d = s.active().expect("doc");
    let got = fp_engine::render::default_renderer().expect("renderer").render_page(&d.doc, d.page, d.frame).expect("image");
    let (mut same, mut max) = (0usize, 0i32);
    for y in 40..360 {
        for x in 40..280 {
            let a = got.get(x, y).expect("got");
            let b = want.get(x, y).expect("want");
            let dmax = (0..4).map(|k| (a[k] as i32 - b[k] as i32).abs()).max().unwrap_or(0);
            if dmax == 0 {
                same += 1;
            }
            max = max.max(dmax);
        }
    }
    assert_eq!((same, max), (76_800, 0), "pixels equal / max deviation in the bitmap area");
}
