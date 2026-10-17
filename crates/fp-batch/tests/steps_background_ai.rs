//! `background.remove` / `background.replace` mit KI (BiRefNet über `fp-ai`). Eigenes
//! Testprogramm, weil die Modellsuche prozessweit umgestellt wird (AGENTEN_REGELN §2). Es wird nie
//! ein Modell heruntergeladen: ohne Modell muss der Schritt sauber „KI-Modell fehlt“ melden. Der
//! Lauf mit echtem Modell ist `#[ignore]` (braucht `BiRefNet_lite` im Modellordner).

mod support;

use fp_batch::item::{Item, ItemInfo, ItemKind, still_from_rgba};
use fp_batch::{
    BatchError, CancelToken, OutputFormat, Recipe, Severity, StepContext, StepRegistry, executor,
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
    for (id, extra) in [
        ("background.remove", json!({})),
        ("background.remove", json!({"model": "birefnet"})),
        ("background.replace", json!({"method": "ai"})),
    ] {
        let mut it = orig.clone();
        let err = apply(id, extra.clone(), &mut it).expect_err("model missing");
        let msg = err.to_string();
        assert!(matches!(err, BatchError::Step { .. }), "{err:?}");
        assert!(msg.contains("KI-Modell fehlt"), "{msg}");
        assert!(msg.contains("BiRefNet"), "Modellname genannt: {msg}");
        assert_eq!(
            it.preview_image().expect("img").data(),
            orig.preview_image().expect("img").data(),
            "{id}: unverändert"
        );
    }
    // Rezeptprüfung warnt schon vorher (Warnung, kein Fehler: das Modell kann noch geladen werden)
    let mut r = Recipe::named("t");
    r.add_step("background.remove", json!({}));
    r.add_step("background.replace", json!({"method": "color"}));
    let issues = r.validate(&StepRegistry::global());
    let ai: Vec<_> = issues
        .iter()
        .filter(|i| i.code == "aiModelMissing")
        .collect();
    assert_eq!(ai.len(), 1, "nur der KI-Schritt: {issues:?}");
    assert_eq!(ai[0].severity, Severity::Warning);
    // Englisch
    fp_i18n::set_language("en");
    let mut it = orig.clone();
    let msg = apply("background.remove", json!({}), &mut it)
        .expect_err("missing")
        .to_string();
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
    r.output.target = fp_batch::recipe::TargetSpec {
        kind: fp_batch::recipe::TargetKind::Folder,
        path: Some(dir.join("out")),
        ..Default::default()
    };
    r.add_step("background.remove", json!({}));
    let items = fp_batch::collect(std::slice::from_ref(&src), false, false, &[]).items;
    let rep = executor::run(&r, &items, &fp_batch::RunOptions::default()).expect("run");
    assert_eq!(
        rep.items[0].status,
        fp_batch::ItemStatus::Failed,
        "{:?}",
        rep.items[0]
    );
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
    assert!(
        fp_ai::models::available(fp_ai::ModelId::BiRefNetLite),
        "Modell fehlt"
    );
    let (w, h) = (256u32, 256u32);
    let mut v = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let d = ((x as f32 - 128.0).powi(2) + (y as f32 - 128.0).powi(2)).sqrt();
            v.extend_from_slice(if d < 60.0 {
                &[30, 60, 160, 255]
            } else {
                &[245, 245, 245, 255]
            });
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
    apply(
        "background.remove",
        json!({"expand": 10, "edge": "hard"}),
        &mut grown,
    )
    .expect("again");
    assert!(
        t0.elapsed().as_secs_f32() < 5.0,
        "Netzausgabe zwischengespeichert"
    );
    let g = grown.preview_image().expect("img");
    assert!(
        g.data().iter().all(|p| p[3] == 0 || p[3] == 255),
        "harte Kante"
    );
    let count = |i: &fp_raster::RgbaImage| i.data().iter().filter(|p| p[3] > 128).count();
    assert!(count(&g) > count(&d), "Ausdehnen vergrößert das Motiv");
}

/// CT-C-01: Lauf mit KI-Schritt und knappem Speicherbudget (jedes Element läuft allein). Früher holte
/// rayon dem auf das Netz wartenden Arbeits-Thread das nächste Element auf den Stapel; das wartete auf
/// das Budget des Rahmens darunter – der Lauf stand, „Abbrechen“ kam nie an. Jetzt: ein Lauf ohne
/// Abbrechen wird fertig, ein abgebrochener endet in Sekunden mit „abgebrochen“, ohne Temporärdateien.
#[test]
#[ignore = "braucht das KI-Modell BiRefNet_lite im Modellordner"]
fn ct_c_01_ai_run_with_tight_budget_finishes_and_cancels_quickly() {
    let _g = MODELS.lock().unwrap_or_else(|e| e.into_inner());
    fp_ai::models::set_search_override(Vec::new());
    assert!(
        fp_ai::models::available(fp_ai::ModelId::BiRefNetLite),
        "Modell fehlt"
    );
    let dir = temp("ct-c-01");
    let srcs: Vec<PathBuf> = (0..6)
        .map(|i| {
            let p = dir.join(format!("b{i}.png"));
            write_png(&p, 64, 48);
            p
        })
        .collect();
    let mut r = Recipe::named("t");
    r.output.format = "png".into();
    r.output.target = fp_batch::recipe::TargetSpec {
        kind: fp_batch::recipe::TargetKind::Folder,
        path: Some(dir.join("out")),
        ..Default::default()
    };
    r.add_step("background.remove", json!({}));
    let items = fp_batch::collect(&srcs[..3], false, false, &[]).items;
    let opts = || fp_batch::RunOptions {
        threads: Some(2),
        memory_budget: Some(1),
        ..Default::default()
    };
    // 1) ohne Abbrechen: fertig (früher Stillstand)
    let h = fp_batch::spawn(r.clone(), items, opts());
    let t0 = std::time::Instant::now();
    while !h.is_finished() {
        assert!(
            t0.elapsed().as_secs() < 180,
            "Lauf steht (Fremdpool-Warten)"
        );
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let rep = h.join().expect("run");
    assert_eq!(rep.totals.done, 3, "{:?}", rep.items);
    // 2) Abbrechen während das Netz rechnet: endet schnell
    let items = fp_batch::collect(&srcs, false, false, &[]).items;
    fp_ai::cache::clear_results();
    let mut r2 = r.clone();
    r2.output.target.path = Some(dir.join("out2"));
    r2.steps[0].params = json!({"expand": 1});
    let h = fp_batch::spawn(r2, items, opts());
    loop {
        if let fp_batch::Progress::ItemStarted { .. } = h
            .events
            .recv_timeout(std::time::Duration::from_secs(60))
            .expect("Ereignis")
        {
            break;
        }
    }
    std::thread::sleep(std::time::Duration::from_millis(300));
    h.cancel();
    let t1 = std::time::Instant::now();
    while !h.is_finished() {
        assert!(t1.elapsed().as_secs() < 20, "Abbrechen hängt");
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let rep = h.join().expect("run");
    assert!(rep.cancelled, "abgebrochen");
    assert!(rep.totals.done < 6, "{:?}", rep.totals);
    assert!(!has_temp_files(&dir), "keine Temporärdateien");
    let _ = std::fs::remove_dir_all(&dir);
}
