//! F143 / BA-037: the object-selection hover preview with the real SAM 2.1 weights (`#[ignore]`,
//! skips cleanly without the model). After a first click (model loaded, embedding cached) the
//! hover job runs only the decoder; a click on the hovered pixel reuses it and gives the same mask.
//!
//! `cargo test -p fp-engine --release --test all ai_real_hover:: -- --ignored --nocapture` (alone or
//! with `--test-threads 1`: other AI tests change the process-wide model search)

use crate::common;

use ai::fp_ai::models::ModelId;
use ai::fp_ai::{Progress, cache, models};
use common::*;
use fp_engine::Session;
use fp_engine::commands::ai;
use fp_engine::selection::SelectMode;
use fp_engine::tools::object_click_params;
use fp_geom::{IRect, Point};
use fp_raster::{Mask, RgbaImage};
use serde_json::json;
use std::time::Instant;

/// Red disk (centre, radius) on a noisy green background, bitmap at (0, 0).
const DISK: (f32, f32, f32) = (150.0, 140.0, 60.0);

fn setup() -> Session {
    ai::fp_ai::download::disable();
    let mut s = doc_session(400.0, 300.0);
    let mut img = RgbaImage::new(IRect::from_xywh(0, 0, 360, 280)).expect("image");
    let w = img.width();
    for (i, px) in img.data_mut().iter_mut().enumerate() {
        let (x, y) = ((i % w) as i32, (i / w) as i32);
        let n = (((x * 7919 + y * 104_729) ^ (x * y)) & 63) as u8;
        let inside = (x as f32 - DISK.0).hypot(y as f32 - DISK.1) <= DISK.2;
        *px = if inside {
            [220, 30 + n / 4, 40, 255]
        } else {
            [60 + n / 2, 120 + n, 50, 255]
        };
    }
    s.clipboard.clear();
    s.clipboard.pixels = Some(img);
    ex(&mut s, "edit.paste", json!({}));
    ex(&mut s, "select.none", json!({}));
    s
}

fn params(s: &Session, x: f32, y: f32) -> serde_json::Value {
    let view = s.active().expect("doc").view.visible_rect();
    object_click_params(Point::new(x, y), SelectMode::Replace, view)
}

fn selection(s: &Session) -> Mask {
    s.active()
        .and_then(|d| d.selection.pixels.clone())
        .expect("selection")
}

fn same(a: &Mask, b: &Mask) -> bool {
    let (ta, tb) = (a.tight_bounds(), b.tight_bounds());
    ta == tb
        && !ta.is_empty()
        && (ta.top..ta.bottom).all(|y| (ta.left..ta.right).all(|x| a.get(x, y) == b.get(x, y)))
}

/// Pixels of `m` (≥ 128) inside / outside the disk.
fn disk_iou(m: &Mask) -> f64 {
    let (mut inter, mut uni) = (0usize, 0usize);
    for y in 0..280 {
        for x in 0..360 {
            let truth = (x as f32 - DISK.0).hypot(y as f32 - DISK.1) <= DISK.2;
            let sel = m.get(x, y) >= 128;
            inter += (truth && sel) as usize;
            uni += (truth || sel) as usize;
        }
    }
    inter as f64 / uni.max(1) as f64
}

#[test]
#[ignore = "needs SAM 2.1 Hiera Tiny in the model folder"]
fn ai_real_hover_matches_click() {
    ai::set_object_stub(None);
    let mut s = setup();
    if !models::available(ModelId::Sam2Tiny) {
        eprintln!("SAM 2.1 Hiera Tiny missing – skipped");
        return;
    }
    cache::unload_all();
    // cold cache: no preview, nothing loads
    let p0 = params(&s, DISK.0, DISK.1);
    assert!(ai::hover_plan(&s, &p0).expect("plan").is_none());
    assert!(!cache::is_loaded(ModelId::Sam2Tiny));
    // first click: loads the model and encodes the image
    let t = Instant::now();
    let v = ex(&mut s, "select.aiObject", p0);
    let first = t.elapsed().as_secs_f64();
    assert_eq!(v["cached"], json!(false));
    let iou_click = disk_iou(&selection(&s));
    // hover over another pixel of the disk: decoder only, on another thread
    let p = params(&s, DISK.0 + 25.0, DISK.1 - 15.0);
    let plan = ai::hover_plan(&s, &p)
        .expect("plan")
        .expect("warm cache: a plan");
    let t = Instant::now();
    let hover = std::thread::spawn(move || plan.compute(&Progress::new()))
        .join()
        .expect("thread")
        .expect("compute")
        .expect("mask");
    let hover_secs = t.elapsed().as_secs_f64();
    let iou_hover = disk_iou(&hover);
    // click on the hovered pixel: reuses the hover decoder output
    let t = Instant::now();
    let v = ex(&mut s, "select.aiObject", p);
    let click2 = t.elapsed().as_secs_f64();
    eprintln!(
        "BA-037 real weights: first click {first:.2} s (load + encode + decode, IoU {iou_click:.3}), hover {hover_secs:.3} s (IoU {iou_hover:.3}), click on the hovered pixel {click2:.3} s (cached {})",
        v["cached"]
    );
    assert_eq!(v["cached"], json!(true));
    assert!(same(&selection(&s), &hover), "click mask equals hover mask");
    assert!(iou_click > 0.85 && iou_hover > 0.85, "{iou_click} {iou_hover}");
    assert!(hover_secs < 5.0, "hover runs the decoder only");
    // cancelled hover: error, nothing kept
    let p3 = params(&s, DISK.0 - 20.0, DISK.1 + 20.0);
    let plan = ai::hover_plan(&s, &p3).expect("plan").expect("plan");
    let prog = Progress::new();
    prog.cancel.cancel();
    assert!(matches!(
        plan.compute(&prog),
        Err(ai::fp_ai::AiError::Cancelled)
    ));
    let v = ex(&mut s, "select.aiObject", p3);
    assert_eq!(v["cached"], json!(true), "embedding stays cached");
}
