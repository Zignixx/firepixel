//! `background.remove` / `background.replace` mit KI (BiRefNet über `fp-ai`). Eigenes
//! Testprogramm, weil die Modellsuche prozessweit umgestellt wird (AGENTEN_REGELN §2). Es wird nie
//! ein Modell heruntergeladen: ohne Modell muss der Schritt sauber „KI-Modell fehlt“ melden. Der
//! Lauf mit echtem Modell ist `#[ignore]` (braucht `BiRefNet_lite` im Modellordner).

mod support;

use fp_batch::item::{still_from_rgba, Item, ItemInfo, ItemKind};
use fp_batch::{executor, BatchError, CancelToken, OutputFormat, Recipe, Severity, StepContext, StepRegistry};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Mutex;
use support::*;

static MODELS: Mutex<()> = Mutex::new(());

fn info(w: u32, h: u32) -> ItemInfo {
    ItemInfo { path: PathBuf::from("bild.png"), root: None, kind: ItemKind::Image, format: "png".into(), bytes: 0, width: w, height: h, frames: 1, modified: None, exif: None }
}

fn apply(id: &str, params: Value, item: &mut Item) -> fp_batch::Result<Vec<String>> {
    let reg = StepRegistry::global();
    let step = reg.get(id).expect("step");
    let p = fp_batch::step::with_defaults(&step.params(), &params);
    let (w, h) = item.dimensions();
    let inf = info(w, h);
    let cancel = CancelToken::new();
    let mut ctx = StepContext::new(0, 1, &inf, &cancel, OutputFormat::Png);
    step.apply(item, &p, &mut ctx)?;
    Ok(ctx.warnings)
}

/// Leerer Modellordner für die Dauer des Tests.
struct NoModels(PathBuf);
impl NoModels {
    fn new(name: &str) -> NoModels {
        let d = temp(name);
        fp_ai::models::set_search_override(vec![d.clone()]);
        NoModels(d)
    }
}
impl Drop for NoModels {
    fn drop(&mut self) {
        fp_ai::models::set_search_override(Vec::new());
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn missing_model_reports_clearly_and_leaves_image_untouched() {
    let _g = MODELS.lock().unwrap_or_else(|e| e.into_inner());
    let _m = NoModels::new("ai-missing");
    fp_i18n::set_language("de");
    let orig = still_from_rgba(8, 8, &[10, 20, 30, 255].repeat(64)).expect("item");
    for (id, extra) in [("background.remove", json!({})), ("background.remove", json!({"model": "birefnet"})), ("background.replace", json!({"method": "ai"}))] {
        let mut it = orig.clone();
        let err = apply(id, extra.clone(), &mut it).expect_err("model missing");
        let msg = err.to_string();
        assert!(matches!(err, BatchError::Step { .. }), "{err:?}");
        assert!(msg.contains("KI-Modell fehlt"), "{msg}");
        assert!(msg.contains("BiRefNet"), "Modellname genannt: {msg}");
        assert_eq!(it.preview_image().expect("img").data(), orig.preview_image().expect("img").data(), "{id}: unverändert");
    }
    // Rezeptprüfung warnt schon vorher (Warnung, kein Fehler: das Modell kann noch geladen werden)
    let mut r = Recipe::named("t");
    r.add_step("background.remove", json!({}));
    r.add_step("background.replace", json!({"method": "color"}));
    let issues = r.validate(&StepRegistry::global());
    let ai: Vec<_> = issues.iter().filter(|i| i.code == "aiModelMissing").collect();
    assert_eq!(ai.len(), 1, "nur der KI-Schritt: {issues:?}");
    assert_eq!(ai[0].severity, Severity::Warning);
    // Englisch
    fp_i18n::set_language("en");
    let mut it = orig.clone();
    let msg = apply("background.remove", json!({}), &mut it).expect_err("missing").to_string();
    assert!(msg.contains("AI model missing"), "{msg}");
    fp_i18n::set_language("de");
}

#[test]
fn missing_model_in_a_run_fails_the_item_but_not_the_job() {
    let _g = MODELS.lock().unwrap_or_else(|e| e.into_inner());
    let _m = NoModels::new("ai-missing-run");
    let dir = temp("ai-run");
    let src = dir.join("a.png");
    write_png(&src, 6, 6);
    let sum = checksum(&src);
    let mut r = Recipe::named("t");
    r.output.format = "png".into();
    r.output.target = fp_batch::recipe::TargetSpec { kind: fp_batch::recipe::TargetKind::Folder, path: Some(dir.join("out")), ..Default::default() };
    r.add_step("background.remove", json!({}));
    let items = fp_batch::collect(std::slice::from_ref(&src), false, false, &[]).items;
    let rep = executor::run(&r, &items, &fp_batch::RunOptions::default()).expect("run");
    assert_eq!(rep.items[0].status, fp_batch::ItemStatus::Failed, "{:?}", rep.items[0]);
    assert!(!dir.join("out").join("a.png").exists());
    assert!(!has_temp_files(&dir));
    assert_eq!(checksum(&src), sum, "Original unverändert");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Mit echtem Modell: weißes Bild mit dunklem Kreis → Kreis bleibt, Ränder werden transparent;
/// zweiter Lauf mit anderen Feineinstellungen nutzt die zwischengespeicherte Netzausgabe.
#[test]
#[ignore = "braucht das KI-Modell BiRefNet_lite im Modellordner"]
fn removes_background_with_real_model() {
    let _g = MODELS.lock().unwrap_or_else(|e| e.into_inner());
    fp_ai::models::set_search_override(Vec::new());
    assert!(fp_ai::models::available(fp_ai::ModelId::BiRefNetLite), "Modell fehlt");
    let (w, h) = (256u32, 256u32);
    let mut v = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let d = ((x as f32 - 128.0).powi(2) + (y as f32 - 128.0).powi(2)).sqrt();
            v.extend_from_slice(if d < 60.0 { &[30, 60, 160, 255] } else { &[245, 245, 245, 255] });
        }
    }
    let src = still_from_rgba(w, h, &v).expect("item");
    let mut it = src.clone();
    apply("background.remove", json!({}), &mut it).expect("remove");
    let d = it.preview_image().expect("img");
    assert!(d.data()[0][3] < 30, "Ecke transparent: {:?}", d.data()[0]);
    assert!(d.data()[(128 * w + 128) as usize][3] > 220, "Motiv bleibt");
    let t0 = std::time::Instant::now();
    let mut grown = src.clone();
    apply("background.remove", json!({"expand": 10, "edge": "hard"}), &mut grown).expect("again");
    assert!(t0.elapsed().as_secs_f32() < 5.0, "Netzausgabe zwischengespeichert");
    let g = grown.preview_image().expect("img");
    assert!(g.data().iter().all(|p| p[3] == 0 || p[3] == 255), "harte Kante");
    let count = |i: &fp_raster::RgbaImage| i.data().iter().filter(|p| p[3] > 128).count();
    assert!(count(&g) > count(&d), "Ausdehnen vergrößert das Motiv");
}
