//! Dialog „Bitmap nachzeichnen…“, Auswahl „Verfahren“ (VT-6, Nutzerbefund N29): Umschalten
//! startet genau eine neue Vorschau, eigene Regler je Verfahren, Kennzahlen zum Vergleich,
//! OK = ein Verlaufsschritt mit dem gewählten Verfahren, die Wahl wird sofort gemerkt (auch bei
//! Abbrechen), Vorgabewechsel behält das Verfahren, 20× hin und her ergibt dasselbe Ergebnis,
//! kein Überlauf in de/en.
//! Wie alle Testläufe mit `FP_NO_SHELL_OPEN=1` starten (AGENTEN_REGELN §4a).

use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use fp_trace::{Engine, Preset};
use fp_ui::app_state::{Action, AppState, UiSettings};
use fp_ui::dialogs::{self, trace};
use fp_ui::FirepixelApp;
use serde_json::json;
use std::time::{Duration, Instant};

struct S {
    app: FirepixelApp,
}

type H = Harness<'static, S>;

/// Strichzeichnung als Bitmap (Ring und Balken, schwarz auf weiß) bei (20, 30), 160 × 120, ausgewählt.
fn session() -> fp_engine::Session {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("Dokument");
    let mut img = fp_raster::RgbaImage::filled(fp_geom::IRect::from_xywh(20, 30, 160, 120), fp_color::Rgba8::new(255, 255, 255, 255)).expect("Bild");
    let w = img.width();
    for (i, px) in img.data_mut().iter_mut().enumerate() {
        let (x, y) = ((i % w) as f64 + 0.5, (i / w) as f64 + 0.5);
        let r = (x - 55.0).hypot(y - 60.0);
        if (30.0..=40.0).contains(&r) || ((105.0..145.0).contains(&x) && (20.0..100.0).contains(&y) && (x - 105.0 - (y - 20.0) * 0.3).abs() < 6.0) {
            *px = [0, 0, 0, 255];
        }
    }
    s.clipboard.clear();
    s.clipboard.pixels = Some(img);
    s.execute("edit.paste", json!({})).expect("Bitmap einfügen");
    s
}

fn harness() -> H {
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, session(), UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).with_max_steps(8).with_step_dt(1.0 / 60.0).build_ui_state(|ui, s: &mut S| s.app.root_ui(ui), S { app });
    h.run_steps(3);
    h
}

fn st(h: &mut H) -> &mut AppState {
    &mut h.state_mut().app.state
}

fn dialog(h: &mut H) -> &mut trace::TraceDialog {
    st(h).dialogs.trace.as_mut().expect("Dialog")
}

fn open(h: &mut H) {
    st(h).push(Action::Run("modify.traceBitmap".into(), json!({})));
    h.run_steps(2);
    assert!(st(h).dialogs.trace.is_some(), "Dialog offen");
}

fn wait_idle(h: &mut H) {
    let t0 = Instant::now();
    loop {
        h.run_steps(1);
        let busy = st(h).dialogs.trace.as_ref().is_some_and(|d| d.is_busy());
        if !busy || t0.elapsed() > Duration::from_secs(60) {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    h.run_steps(2);
}

fn model(st: &AppState) -> String {
    serde_json::to_string(&*st.session.active().expect("Dokument").doc).unwrap_or_default()
}

fn undo_count(st: &AppState) -> usize {
    st.session.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

fn click_radio(h: &mut H, label: &str) {
    h.get_by_role_and_label(egui::accesskit::Role::RadioButton, label).scroll_to_me();
    h.run_steps(3);
    h.get_by_role_and_label(egui::accesskit::Role::RadioButton, label).click();
    h.run_steps(2);
}

#[test]
fn engine_switch_compares_and_ok_uses_the_chosen_engine() {
    fp_i18n::set_language("de");
    let mut h = harness();
    let before = model(st(&mut h));
    let steps = undo_count(st(&mut h));
    open(&mut h);
    wait_idle(&mut h);
    assert_eq!(dialog(&mut h).opts.engine, Engine::Clusters, "Vorgabe: bisheriges Verfahren");
    let clusters = dialog(&mut h).stats.clone().expect("Kennzahlen");
    let runs = dialog(&mut h).runs_started;
    // eigene Regler: Farbflächen zeigen „Aufräumen“, aber nicht „Kurven zusammenfassen“
    assert!(h.query_by_label(&fp_i18n::t("trace-refine")).is_some());
    assert!(h.query_by_label(&fp_i18n::t("trace-potrace-optcurve")).is_none());
    // Umschalten auf „Konturen“: genau eine neue Rechnung
    click_radio(&mut h, &fp_i18n::t("trace-engine-potrace"));
    assert_eq!(dialog(&mut h).opts.engine, Engine::Potrace);
    assert_eq!(dialog(&mut h).runs_started, runs + 1, "eine neue Vorschau");
    wait_idle(&mut h);
    let pot = dialog(&mut h).stats.clone().expect("Kennzahlen");
    assert!(pot.paths >= 1 && pot.points > 0 && pot.match_pct > 95.0, "{pot:?}");
    assert!(pot.refine.is_none(), "keine Aufräum-Kennzahlen beim Konturverfahren");
    assert!(h.query_by_label(&fp_i18n::t("trace-potrace-optcurve")).is_some(), "eigene Regler");
    assert!(h.query_by_label(&fp_i18n::t("trace-refine")).is_none());
    // sofort gemerkt (auch wenn danach abgebrochen wird)
    assert_eq!(st(&mut h).session.prefs.get_str(trace::PREF_ENGINE, ""), "potrace");
    // Regler „Kurven zusammenfassen“ aus: mehr Punkte
    h.get_by_role_and_label(egui::accesskit::Role::CheckBox, &fp_i18n::t("trace-potrace-optcurve")).click();
    h.run_steps(2);
    wait_idle(&mut h);
    let plain = dialog(&mut h).stats.clone().expect("Kennzahlen");
    assert!(!dialog(&mut h).opts.potrace.opt_curve);
    assert!(plain.points > pot.points, "ohne Zusammenfassen mehr Punkte: {} gegen {}", plain.points, pot.points);
    h.get_by_role_and_label(egui::accesskit::Role::CheckBox, &fp_i18n::t("trace-potrace-optcurve")).click();
    h.run_steps(2);
    wait_idle(&mut h);
    // Vorgabe wechseln behält das Verfahren
    dialog(&mut h).set_preset(Preset::BlackWhite);
    assert_eq!(dialog(&mut h).opts.engine, Engine::Potrace);
    wait_idle(&mut h);
    // Abbrechen: Dokument unverändert, Verfahren bleibt gemerkt
    h.get_by_label(&fp_i18n::t("btn-cancel")).click();
    h.run_steps(3);
    assert!(st(&mut h).dialogs.trace.is_none());
    assert_eq!(model(st(&mut h)), before);
    assert_eq!(undo_count(st(&mut h)), steps);
    // neu öffnen: Konturen vorgewählt; OK übernimmt die Vorschau (ein Verlaufsschritt)
    open(&mut h);
    assert_eq!(dialog(&mut h).opts.engine, Engine::Potrace, "gemerkte Wahl");
    wait_idle(&mut h);
    let paths = dialog(&mut h).stats.as_ref().map(|s| s.paths).expect("Kennzahlen");
    assert_eq!(dialog(&mut h).command_params()["engine"], json!("potrace"));
    let computed = fp_engine::commands::trace::computed_traces();
    h.get_by_label(&fp_i18n::t("btn-ok")).click();
    h.run_steps(3);
    assert!(st(&mut h).dialogs.trace.is_none() && st(&mut h).dialogs.message.is_none(), "{:?}", st(&mut h).dialogs.message);
    assert_eq!(fp_engine::commands::trace::computed_traces(), computed, "Vorschau übernommen");
    assert_eq!(undo_count(st(&mut h)), steps + 1, "genau ein Verlaufsschritt");
    let d = st(&mut h).session.active().expect("Dokument");
    let g = d.selection.elements.iter().filter_map(|id| fp_engine::docops::element_on_page(&d.doc, d.page, *id)).next().expect("Gruppe");
    match &g.kind {
        fp_model::ElementKind::Group(g) => assert_eq!(g.children.len(), paths),
        k => panic!("Gruppe erwartet, nicht {k:?}"),
    }
    assert_eq!(trace::remembered_options(st(&mut h)).engine, Engine::Potrace);
    st(&mut h).session.execute("edit.undo", json!({})).expect("Rückgängig");
    assert_eq!(model(st(&mut h)), before, "Rückgängig stellt exakt her");
    assert!(clusters.points > 0);
}

#[test]
fn switching_back_and_forth_gives_the_same_preview() {
    fp_i18n::set_language("de");
    let mut h = harness();
    open(&mut h);
    dialog(&mut h).set_engine(Engine::Potrace);
    wait_idle(&mut h);
    let first = dialog(&mut h).result().cloned().expect("Ergebnis");
    // 20× hin und her (jeder Wechsel ersetzt die laufende Vorschau), am Ende Konturen
    for k in 0..20 {
        dialog(&mut h).set_engine(if k % 2 == 0 { Engine::Clusters } else { Engine::Potrace });
        h.run_steps(1);
    }
    wait_idle(&mut h);
    let d = dialog(&mut h);
    assert_eq!(d.opts.engine, Engine::Potrace);
    assert!(d.reusable_result().is_some(), "Vorschau gehört zu den Optionen");
    assert_eq!(**d.result().expect("Ergebnis"), *first, "gleiches Ergebnis wie beim ersten Mal");
    // dieselbe Wahl noch einmal: keine neue Rechnung
    let runs = d.runs_started;
    d.set_engine(Engine::Potrace);
    assert!(!d.is_busy());
    assert_eq!(d.runs_started, runs);
}

#[test]
fn layout_without_overflow_for_both_engines() {
    for lang in ["de", "en"] {
        fp_i18n::set_language(lang);
        let mut h = harness();
        open(&mut h);
        for engine in Engine::ALL {
            dialog(&mut h).set_engine(engine);
            dialog(&mut h).advanced = true;
            wait_idle(&mut h);
            let _ = dialogs::take_overflows(&h.ctx);
            h.run_steps(2);
            let over = dialogs::take_overflows(&h.ctx);
            assert!(over.is_empty(), "{lang} {engine:?}: Überlauf {over:?}");
            // Abbiegeregeln als Auswahlknöpfe erreichbar
            if engine == Engine::Potrace {
                click_radio(&mut h, &fp_i18n::t("trace-turn-black"));
                assert_eq!(dialog(&mut h).opts.potrace.turn_policy, fp_trace::TurnPolicy::Black);
                wait_idle(&mut h);
            }
        }
        h.get_by_label(&fp_i18n::t("btn-cancel")).click();
        h.run_steps(2);
    }
    fp_i18n::set_language("de");
}
