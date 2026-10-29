//! F138: the PSD-style Stroke live effect as the batch step `effect.stroke` (size, position,
//! blend mode, opacity, colour; the canvas grows to an outside stroke).

use fp_batch::item::{Item, ItemInfo, ItemKind, still_from_rgba};
use fp_batch::{CancelToken, OutputFormat, Recipe, StepContext, StepRegistry};
use serde_json::{Value, json};
use std::path::PathBuf;

const ID: &str = "effect.stroke";

fn info(w: u32, h: u32) -> ItemInfo {
    ItemInfo {
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
    }
}

/// 20 × 20 image with an opaque grey 10 × 10 square in the middle.
fn square() -> Item {
    let raw: Vec<u8> = (0..20 * 20)
        .flat_map(|i| {
            let (x, y) = (i % 20, i / 20);
            if (5..15).contains(&x) && (5..15).contains(&y) {
                [128u8, 128, 128, 255]
            } else {
                [0, 0, 0, 0]
            }
        })
        .collect();
    still_from_rgba(20, 20, &raw).expect("item")
}

fn apply(params: Value, item: &mut Item) {
    let reg = StepRegistry::global();
    let step = reg.get(ID).expect("step registered");
    assert!(step.validate(&params).is_empty(), "valid parameters");
    let p = fp_batch::step::with_defaults(&step.params(), &params);
    let (w, h) = item.dimensions();
    let inf = info(w, h);
    let cancel = CancelToken::new();
    let mut ctx = StepContext::new(0, 1, &inf, &cancel, OutputFormat::Png);
    step.apply(item, &p, &mut ctx).expect("apply");
}

fn px(item: &Item, x: i32, y: i32) -> [u8; 4] {
    let Item::Still(s) = item else {
        panic!("still")
    };
    let b = s.image.bounds();
    s.image.get(b.left + x, b.top + y).unwrap_or([0; 4])
}

#[test]
fn stroke_step_draws_the_band() {
    // inside 2 px: the image keeps its size, the edge of the square turns blue
    let mut it = square();
    apply(
        json!({"size": 2, "position": "inside", "color": "#0000ff"}),
        &mut it,
    );
    assert_eq!(it.dimensions(), (20, 20));
    assert_eq!(px(&it, 5, 10), [0, 0, 255, 255]);
    assert_eq!(px(&it, 4, 10)[3], 0);
    assert_eq!(px(&it, 9, 10), [128, 128, 128, 255]);
    // outside 3 px at 50 %: the square stays grey, the band is half transparent
    let mut it = square();
    apply(
        json!({"size": 3, "opacity": 50, "expandCanvas": false}),
        &mut it,
    );
    assert_eq!(it.dimensions(), (20, 20));
    assert_eq!(px(&it, 4, 10), [255, 0, 0, 128]);
    assert_eq!(px(&it, 5, 10), [128, 128, 128, 255]);
}

#[test]
fn stroke_step_round_trips_in_the_recipe_and_has_texts() {
    let mut r = Recipe::named("stroke");
    r.add_step(
        ID,
        json!({"size": 5, "position": "center", "blendMode": "multiply"}),
    );
    let back = Recipe::from_json(&r.to_json()).expect("recipe");
    assert_eq!(back.steps[0].id, ID);
    assert_eq!(back.steps[0].params.get("position"), Some(&json!("center")));
    let reg = StepRegistry::global();
    let step = reg.get(ID).expect("step");
    assert!(!step.validate(&json!({"position": "around"})).is_empty());
    for l in ["en", "de"] {
        fp_i18n::set_language(l);
        assert_ne!(
            fp_i18n::t("batch-step-effect-stroke"),
            "batch-step-effect-stroke"
        );
    }
    fp_i18n::set_language("en");
    assert_eq!(fp_i18n::t("batch-step-effect-stroke"), "Stroke");
}
