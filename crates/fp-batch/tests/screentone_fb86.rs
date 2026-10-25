//! FB-86 (Firepixel extension): the Screentone filter as the batch step `filter.screentone`
//! with the parameter `antiAlias` (anti-aliased dot edges, default off = hard dots).

use fp_batch::item::{Item, ItemInfo, ItemKind, still_from_rgba};
use fp_batch::{CancelToken, OutputFormat, Recipe, StepContext, StepRegistry};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::path::PathBuf;

const ID: &str = "filter.screentone";

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

fn grey(w: u32, h: u32) -> Item {
    let raw: Vec<u8> = (0..w * h).flat_map(|_| [128u8, 128, 128, 255]).collect();
    still_from_rgba(w, h, &raw).expect("item")
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

fn alphas(item: &Item) -> BTreeSet<u8> {
    let Item::Still(s) = item else {
        panic!("still")
    };
    s.image.data().iter().map(|p| p[3]).collect()
}

#[test]
fn anti_alias_off_gives_hard_dots_on_gives_intermediate_values() {
    let mut hard = grey(48, 48);
    apply(json!({}), &mut hard);
    assert_eq!(
        alphas(&hard),
        BTreeSet::from([0, 255]),
        "two values by default"
    );
    let mut soft = grey(48, 48);
    apply(json!({"antiAlias": true}), &mut soft);
    let a = alphas(&soft);
    assert!(a.iter().any(|v| *v > 0 && *v < 255), "{a:?}");
}

#[test]
fn anti_alias_parameter_round_trips_in_the_recipe_and_has_texts() {
    let mut r = Recipe::named("tone");
    r.add_step(ID, json!({"pattern": "lines", "antiAlias": true}));
    let back = Recipe::from_json(&r.to_json()).expect("recipe");
    assert_eq!(back.steps[0].id, ID);
    assert_eq!(back.steps[0].params.get("antiAlias"), Some(&json!(true)));
    assert_eq!(back.steps[0].params.get("pattern"), Some(&json!("lines")));
    let reg = StepRegistry::global();
    let step = reg.get(ID).expect("step");
    let def = step
        .params()
        .into_iter()
        .find(|d| d.name == "antiAlias")
        .expect("antiAlias");
    assert_eq!(def.default, json!(false));
    for (lang, label) in [("de", "Glätten"), ("en", "Anti-alias")] {
        let (l, tip, name) = fp_i18n::with_language(lang, || {
            (
                fp_i18n::t("batch-param-filter-screentone-antiAlias"),
                fp_i18n::t("batch-param-filter-screentone-antiAlias.tooltip"),
                fp_i18n::t("batch-step-filter-screentone"),
            )
        });
        assert_eq!(l, label);
        assert!(
            !tip.is_empty() && !tip.contains("batch-param"),
            "{lang}: {tip}"
        );
        assert!(!name.contains("batch-step"), "{lang}: {name}");
    }
}
