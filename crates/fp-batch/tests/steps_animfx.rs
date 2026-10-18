//! Animation steps (B-3, `docs/BATCH.md` §3.8): boomerang, cross-fade loop and the existing
//! calm background step — on tiny synthetic animations.

use fp_batch::item::{Animation, Frame, Item, ItemInfo, ItemKind, Meta};
use fp_batch::steps::animfx::{boomerang_order, crossfade};
use fp_batch::{CancelToken, OutputFormat, Severity, StepContext, StepRegistry};
use fp_raster::{IRect, RgbaImage};
use serde_json::{Value, json};
use std::path::PathBuf;

fn solid(v: u8) -> RgbaImage {
    RgbaImage::from_raw(IRect::new(0, 0, 2, 2), vec![[v, v, v, 255]; 4]).expect("img")
}

/// Animation whose frame `i` is grey `values[i]`, 100 ms each.
fn anim(values: &[u8]) -> Item {
    Item::Animation(Animation {
        width: 2,
        height: 2,
        frames: values
            .iter()
            .map(|v| Frame {
                image: solid(*v),
                delay_ms: 100,
            })
            .collect(),
        loop_count: 0,
        meta: Meta::default(),
    })
}

fn greys(item: &Item) -> Vec<u8> {
    match item {
        Item::Animation(a) => a.frames.iter().map(|f| f.image.data()[0][0]).collect(),
        _ => panic!("animation"),
    }
}

fn delays(item: &Item) -> Vec<u32> {
    match item {
        Item::Animation(a) => a.frames.iter().map(|f| f.delay_ms).collect(),
        _ => panic!("animation"),
    }
}

fn apply(id: &str, params: Value, item: &mut Item) -> fp_batch::Result<()> {
    let reg = StepRegistry::global();
    let step = reg.get(id).expect("step");
    let p = fp_batch::step::with_defaults(&step.params(), &params);
    let info = ItemInfo {
        path: PathBuf::from("C:/in/a.gif"),
        root: None,
        kind: ItemKind::Animation,
        format: "gif".into(),
        bytes: 0,
        width: 2,
        height: 2,
        frames: 1,
        modified: None,
        exif: None,
    };
    let cancel = CancelToken::new();
    let mut ctx = StepContext::new(0, 1, &info, &cancel, OutputFormat::Gif);
    step.apply(item, &p, &mut ctx)
}

#[test]
fn boomerang_skips_turning_frames_and_holds_ends() {
    assert_eq!(boomerang_order(1), vec![0]);
    assert_eq!(boomerang_order(2), vec![0, 1]);
    assert_eq!(boomerang_order(4), vec![0, 1, 2, 3, 2, 1]);
    let mut a = anim(&[0, 10, 20, 30]);
    apply("anim.boomerang", json!({"hold": 500}), &mut a).expect("apply");
    assert_eq!(greys(&a), vec![0, 10, 20, 30, 20, 10]);
    assert_eq!(delays(&a), vec![600, 100, 100, 600, 100, 100]);
}

#[test]
fn crossfade_overlap_and_append() {
    let mut a = anim(&[0, 40, 80, 120, 160, 200, 240]);
    apply("anim.crossfade", json!({"frames": 2}), &mut a).expect("overlap");
    // starts at frame 2; the last two fade towards frames 0 and 1 by 1/3 and 2/3
    assert_eq!(greys(&a), vec![80, 120, 160, 200 - 67, 240 - 133]);
    let mut b = anim(&[0, 90]);
    apply(
        "anim.crossfade",
        json!({"frames": 2, "mode": "append", "delay": 40}),
        &mut b,
    )
    .expect("append");
    assert_eq!(greys(&b), vec![0, 90, 60, 30]);
    assert_eq!(delays(&b), vec![100, 100, 40, 40]);
    let mut short = anim(&[0, 1, 2]);
    assert!(apply("anim.crossfade", json!({"frames": 2}), &mut short).is_err());
    let frames = vec![Frame {
        image: solid(5),
        delay_ms: 10,
    }];
    assert_eq!(crossfade(frames, 3, false, 0).expect("single").len(), 1);
}

#[test]
fn stabilize_freezes_small_changes_only() {
    let mut a = anim(&[100, 101, 160]);
    apply("anim.stabilize", json!({"threshold": 3}), &mut a).expect("apply");
    assert_eq!(greys(&a), vec![100, 100, 160]);
    let mut b = anim(&[100, 101]);
    apply("anim.stabilize", json!({"threshold": 0}), &mut b).expect("apply");
    assert_eq!(greys(&b), vec![100, 101]);
    let bad = StepRegistry::global()
        .get("anim.stabilize")
        .expect("step")
        .validate(&json!({"threshold": 99}));
    assert!(bad.iter().any(|i| i.severity == Severity::Error));
}
