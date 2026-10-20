//! `ai.upscale` (Real-ESRGAN over `fp-ai`). Own test program because the model search is switched
//! process-wide. Nothing is ever downloaded: without a model the step must report "AI model
//! missing" cleanly. Runs with the real model are `#[ignore]` (environment `FP_ESRGAN_X4` = path of
//! `RealESRGAN_x4plus.safetensors`; its folder becomes the model folder).

mod support;

use fp_batch::item::{Item, ItemInfo, ItemKind, still_from_rgba};
use fp_batch::{
    BatchError, CancelToken, OutputFormat, Recipe, Severity, StepContext, StepRegistry,
};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::Mutex;
use support::*;

static MODELS: Mutex<()> = Mutex::new(());

fn info(w: u32, h: u32) -> ItemInfo {
    ItemInfo {
        path: PathBuf::from("bild.png"),
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

fn apply(params: Value, item: &mut Item, preview: bool) -> fp_batch::Result<Vec<String>> {
    let reg = StepRegistry::global();
    let step = reg.get("ai.upscale").expect("step");
    let p = fp_batch::step::with_defaults(&step.params(), &params);
    let (w, h) = item.dimensions();
    let inf = info(w, h);
    let cancel = CancelToken::new();
    let mut ctx = StepContext::new(0, 1, &inf, &cancel, OutputFormat::Png);
    ctx.preview = preview;
    step.apply(item, &p, &mut ctx)?;
    Ok(ctx.warnings)
}

/// Model folder `dir` for the duration of the test (`None` = empty temporary folder).
struct Models(Option<PathBuf>);
impl Models {
    fn empty(name: &str) -> Models {
        let d = temp(name);
        fp_ai::models::set_search_override(vec![d.clone()]);
        Models(Some(d))
    }
    fn at(dir: PathBuf) -> Models {
        fp_ai::models::set_search_override(vec![dir]);
        Models(None)
    }
}
impl Drop for Models {
    fn drop(&mut self) {
        fp_ai::models::set_search_override(Vec::new());
        if let Some(d) = &self.0 {
            let _ = std::fs::remove_dir_all(d);
        }
    }
}

#[test]
fn missing_model_reports_clearly_and_leaves_image_untouched() {
    let _g = MODELS.lock().unwrap_or_else(|e| e.into_inner());
    let _m = Models::empty("upscale-missing");
    fp_i18n::set_language("de");
    let orig = still_from_rgba(8, 8, &[10, 20, 30, 255].repeat(64)).expect("item");
    for (extra, name) in [
        (json!({}), "Real-ESRGAN x4plus"),
        (json!({"scale": "2"}), "Real-ESRGAN x2plus"),
        (
            json!({"scale": 4, "model": "realEsrganX2"}),
            "Real-ESRGAN x2plus",
        ),
    ] {
        let mut it = orig.clone();
        let err = apply(extra.clone(), &mut it, false).expect_err("model missing");
        let msg = err.to_string();
        assert!(matches!(err, BatchError::Step { .. }), "{err:?}");
        assert!(msg.contains("KI-Modell fehlt"), "{msg}");
        assert!(msg.contains(name), "model named: {msg}");
        assert!(msg.contains("KI-Modelle verwalten"), "hint: {msg}");
        assert_eq!(
            it.preview_image().expect("img").data(),
            orig.preview_image().expect("img").data(),
            "{extra}: unchanged"
        );
        // preview: same clean error
        assert!(apply(extra, &mut it, true).is_err());
    }
    // recipe check warns in advance (warning, not an error: the model can still be loaded)
    let mut r = Recipe::named("t");
    r.add_step("ai.upscale", json!({"scale": 2}));
    let issues = r.validate(&StepRegistry::global());
    let ai: Vec<_> = issues
        .iter()
        .filter(|i| i.code == "aiModelMissing")
        .collect();
    assert_eq!(ai.len(), 1, "{issues:?}");
    assert_eq!(ai[0].severity, Severity::Warning);
    // a numeric scale is valid, other factors are not
    let step = StepRegistry::global()
        .get("ai.upscale")
        .cloned()
        .expect("step");
    let codes = |p: Value| -> Vec<String> {
        step.validate(&p)
            .into_iter()
            .filter(|i| i.code != "aiModelMissing")
            .map(|i| i.message)
            .collect()
    };
    assert!(codes(json!({"scale": 4})).is_empty());
    assert!(codes(json!({"scale": "2", "alpha": "bicubic"})).is_empty());
    assert_eq!(codes(json!({"scale": 3})).len(), 1);
    assert_eq!(codes(json!({"model": "other"})).len(), 1);
    // English
    fp_i18n::set_language("en");
    let mut it = orig.clone();
    let msg = apply(json!({}), &mut it, false)
        .expect_err("missing")
        .to_string();
    assert!(
        msg.contains("AI model missing") && msg.contains("Manage AI Models"),
        "{msg}"
    );
    fp_i18n::set_language("de");
}

#[test]
fn catalog_texts_and_predicted_size() {
    let reg = StepRegistry::global();
    let step = reg.get("ai.upscale").expect("registered");
    assert_eq!(step.group(), fp_batch::StepGroup::Color);
    for lang in ["de", "en"] {
        fp_i18n::with_language(lang, || {
            let label = fp_batch::step::step_label("ai.upscale");
            assert_ne!(label, "ai.upscale", "{lang}");
            let s = step.summary(&json!({"scale": "2"}));
            assert!(s.contains('2') && s.contains("x2plus"), "{lang}: {s}");
        });
    }
    let mut r = Recipe::named("t");
    r.add_step("ai.upscale", json!({}));
    assert_eq!(
        fp_batch::predict::output_size(&r, &info(100, 50), &reg),
        Some((400, 200))
    );
}

fn test_image(w: u32, h: u32) -> Item {
    still_from_rgba(w, h, &rgba(w, h, true)).expect("item")
}

fn real_models() -> Option<PathBuf> {
    let p = PathBuf::from(std::env::var_os("FP_ESRGAN_X4")?);
    p.parent().map(|d| d.to_path_buf())
}

/// With the real ×4 model: sizes of ×4, ×2 (×4 model + Lanczos), preview detail; image with
/// transparent edge.
#[test]
#[ignore = "needs FP_ESRGAN_X4 = path of RealESRGAN_x4plus.safetensors"]
fn upscales_with_real_model() {
    let _g = MODELS.lock().unwrap_or_else(|e| e.into_inner());
    let Some(dir) = real_models() else {
        eprintln!("FP_ESRGAN_X4 not set – skipped");
        return;
    };
    let _m = Models::at(dir);
    let t = std::time::Instant::now();
    let mut it = test_image(40, 30);
    let w = apply(json!({}), &mut it, false).expect("x4");
    eprintln!("40×30 ×4: {:.2} s", t.elapsed().as_secs_f64());
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(it.dimensions(), (160, 120));
    let img = it.preview_image().expect("img");
    assert!(img.data().iter().any(|p| p[3] < 255), "transparency kept");
    let mut it = test_image(40, 30);
    apply(
        json!({"scale": "2", "model": "realEsrganX4", "alpha": "bicubic"}),
        &mut it,
        false,
    )
    .expect("x2 via x4");
    assert_eq!(it.dimensions(), (80, 60));
    // preview: whole proxy bicubic, AI detail in the middle, with a hint
    let mut it = test_image(150, 120);
    let w = apply(json!({}), &mut it, true).expect("preview");
    assert_eq!(it.dimensions(), (600, 480));
    assert_eq!(w.len(), 1, "{w:?}");
}
