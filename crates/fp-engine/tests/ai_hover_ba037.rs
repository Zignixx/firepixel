//! BA-037: object-selection preview while hovering. The hover job computes `select.aiObject` on a
//! copy of the pixels; a click with the same prompt reuses its decoder output and yields exactly
//! the same mask. Runs with a stub decoder (no model files needed).

use crate::common;

use common::*;
use fp_engine::commands::ai as ai;
use fp_engine::selection::SelectMode;
use fp_engine::tools::object_click_params;
use fp_engine::Session;
use fp_geom::{IRect, Point};
use fp_raster::{Mask, RgbaImage};
use ai::fp_ai::pipeline::{ObjectPrompt, ObjectRaw};
use ai::fp_ai::{sam2, Progress};
use serde_json::json;
use std::cell::Cell;

thread_local! {
    static CALLS: Cell<usize> = const { Cell::new(0) };
}

/// Stub decoder: a disk of radius 40 px around the first point, on the low-resolution grid of the
/// region like the real decoder.
fn disk_stub(_img: &RgbaImage, r: IRect, prompt: &ObjectPrompt) -> ai::fp_ai::Result<ObjectRaw> {
    CALLS.with(|c| c.set(c.get() + 1));
    let (cx, cy) = prompt.points.first().map(|q| (q.x, q.y)).unwrap_or((0.0, 0.0));
    let n = sam2::LOW;
    let (w, h) = (r.width() as f32, r.height() as f32);
    let mut plane = vec![-10.0f32; n * n];
    for j in 0..n {
        for i in 0..n {
            let x = r.left as f32 + (i as f32 + 0.5) * w / n as f32;
            let y = r.top as f32 + (j as f32 + 0.5) * h / n as f32;
            if (x - cx).hypot(y - cy) <= 40.0 {
                plane[j * n + i] = 10.0;
            }
        }
    }
    let mut all = Vec::with_capacity(4 * n * n);
    for _ in 0..4 {
        all.extend_from_slice(&plane);
    }
    Ok(ObjectRaw {
        masks: std::sync::Arc::new(all),
        ious: [0.9; 4],
        pick: 0,
        object_score: 5.0,
        region: r,
        cached: false,
    })
}

fn calls() -> usize {
    CALLS.with(|c| c.get())
}

struct StubGuard;

impl Drop for StubGuard {
    fn drop(&mut self) {
        ai::set_object_stub(None);
    }
}

fn stub() -> StubGuard {
    CALLS.with(|c| c.set(0));
    ai::set_object_stub(Some(disk_stub));
    StubGuard
}

fn setup() -> Session {
    ai::fp_ai::download::disable();
    let mut s = doc_session(400.0, 300.0);
    let mut img = RgbaImage::new(IRect::from_xywh(0, 0, 300, 200)).expect("image");
    for (i, px) in img.data_mut().iter_mut().enumerate() {
        *px = [(i % 251) as u8, 120, (i % 97) as u8, 255];
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

fn same(a: &Mask, b: &Mask) -> bool {
    let (ta, tb) = (a.tight_bounds(), b.tight_bounds());
    if ta != tb || ta.is_empty() {
        return false;
    }
    (ta.top..ta.bottom).all(|y| (ta.left..ta.right).all(|x| a.get(x, y) == b.get(x, y)))
}

#[test]
fn hover_job_yields_the_same_mask_as_the_click() {
    let _g = stub();
    let mut s = setup();
    let p = params(&s, 120.0, 100.0);
    let plan = ai::hover_plan(&s, &p).expect("plan").expect("stub: always a plan");
    // on another thread, like the UI does it
    let hover = std::thread::spawn(move || plan.compute(&Progress::new()))
        .join()
        .expect("thread")
        .expect("compute")
        .expect("mask");
    assert_eq!(calls(), 0, "the hover job captured the stub but counted on its own thread");
    let b = hover.tight_bounds();
    assert!((b.left - 80).abs() <= 3 && (b.right - 160).abs() <= 3, "disk bounds {b:?}");
    // click with the same prompt: the decoder does not run again, the mask is identical
    let v = ex(&mut s, "select.aiObject", p);
    assert_eq!(calls(), 0, "click reuses the hover decoder output");
    assert_eq!(v["cached"], json!(true));
    let sel = s.active().and_then(|d| d.selection.pixels.clone()).expect("selection");
    assert!(same(&sel, &hover), "click mask equals hover mask");
    // another pixel: the click decodes itself
    let p2 = params(&s, 200.0, 120.0);
    ex(&mut s, "select.aiObject", p2);
    assert_eq!(calls(), 1);
}

#[test]
fn cancelled_hover_job_leaves_nothing_behind() {
    let _g = stub();
    let mut s = setup();
    let p = params(&s, 60.0, 60.0);
    let plan = ai::hover_plan(&s, &p).expect("plan").expect("plan");
    let prog = Progress::new();
    prog.cancel.cancel();
    assert!(matches!(plan.compute(&prog), Err(ai::fp_ai::AiError::Cancelled)));
    ex(&mut s, "select.aiObject", p);
    assert_eq!(calls(), 1, "nothing cached by the cancelled job: the click decodes");
}

#[test]
fn cold_cache_gives_no_hover_plan() {
    use ai::fp_ai::models::ModelId;
    ai::set_object_stub(None);
    let s = setup();
    if ai::fp_ai::cache::is_loaded(ModelId::Sam2Tiny) || ai::fp_ai::cache::is_loaded(ModelId::Sam2Small) {
        return;
    }
    let p = params(&s, 120.0, 100.0);
    assert!(ai::hover_plan(&s, &p).expect("plan").is_none(), "no model loaded: no preview, nothing loads");
}
