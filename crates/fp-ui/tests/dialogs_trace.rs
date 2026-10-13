//! Dialog „Bitmap nachzeichnen…“ (VT-2): erreichbar über Menü, Leinwand-Kontextmenü und
//! Inspektor, Vorgabe wechseln startet eine neue Hintergrundrechnung, Abbrechen ändert nichts,
//! OK legt genau einen Verlaufsschritt (Gruppe auf neuer Ebene) an, Englisch und alle drei Designs
//! ohne Überlauf, auch bei 1024 × 768 und 150 %.
//! Wie alle Testläufe mit `FP_NO_SHELL_OPEN=1` starten (AGENTEN_REGELN §4a).

use egui::epaint::{Shape, TextShape};
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use fp_ui::app_state::{Action, AppState, UiSettings};
use fp_ui::dialogs::{self, trace};
use fp_ui::menu::{ItemAction, Node};
use fp_ui::theme::ThemeKind;
use fp_ui::FirepixelApp;
use serde_json::json;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Sprache und Design sind global: Tests dieser Datei laufen nacheinander.
static LOCK: Mutex<()> = Mutex::new(());

struct Guard(#[allow(dead_code)] std::sync::MutexGuard<'static, ()>);

impl Drop for Guard {
    fn drop(&mut self) {
        fp_i18n::set_language("de");
    }
}

fn lock() -> Guard {
    let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    fp_i18n::set_language("de");
    Guard(g)
}

struct S {
    app: FirepixelApp,
    only_dialogs: bool,
}

type H = Harness<'static, S>;

/// Logo-Bitmap (blauer Kreis, rotes Rechteck auf Weiß) bei (20, 30), 200 × 120, ausgewählt.
fn session() -> fp_engine::Session {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("Dokument");
    let mut img = fp_raster::RgbaImage::filled(fp_geom::IRect::from_xywh(20, 30, 200, 120), fp_color::Rgba8::new(255, 255, 255, 255)).expect("Bild");
    let w = img.width();
    for (i, px) in img.data_mut().iter_mut().enumerate() {
        let (x, y) = ((i % w) as f64 + 0.5, (i / w) as f64 + 0.5);
        if (x - 60.0).hypot(y - 60.0) < 40.0 {
            *px = [20, 60, 200, 255];
        } else if (120.0..180.0).contains(&x) && (30.0..100.0).contains(&y) {
            *px = [220, 30, 40, 255];
        }
    }
    s.clipboard.clear();
    s.clipboard.pixels = Some(img);
    s.execute("edit.paste", json!({})).expect("Bitmap einfügen");
    s
}

fn harness_sized(w: f32, h: f32, ppp: f32) -> H {
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, session(), UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(w, h)).with_pixels_per_point(ppp).with_max_steps(8).with_step_dt(1.0 / 60.0).build_ui_state(
        |ui, s: &mut S| {
            if s.only_dialogs {
                dialogs::show_all(ui.ctx(), &mut s.app.state);
            } else {
                s.app.root_ui(ui);
            }
        },
        S { app, only_dialogs: false },
    );
    h.run_steps(3);
    h
}

fn harness() -> H {
    harness_sized(1400.0, 900.0, 1.0)
}

fn st(h: &mut H) -> &mut AppState {
    &mut h.state_mut().app.state
}

fn open(h: &mut H) {
    st(h).push(Action::Run("modify.traceBitmap".into(), json!({})));
    h.run_steps(2);
    assert!(st(h).dialogs.trace.is_some(), "Dialog offen");
}

/// Bilder zeichnen, bis die Hintergrundrechnung fertig ist.
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

fn find_item<'a>(nodes: &'a [Node], cmd: &str) -> Option<&'a Node> {
    for n in nodes {
        match n {
            Node::Item { action: ItemAction::Command(c), .. } if c == cmd => return Some(n),
            Node::Sub { children, .. } => {
                if let Some(x) = find_item(children, cmd) {
                    return Some(x);
                }
            }
            _ => {}
        }
    }
    None
}

fn enabled(n: Option<&Node>) -> Option<bool> {
    match n {
        Some(Node::Item { enabled, .. }) => Some(*enabled),
        _ => None,
    }
}

#[test]
fn reachable_from_menu_context_menu_and_inspector() {
    let _g = lock();
    let mut h = harness();
    // Hauptmenü: Modifizieren ▸ Bitmap nachzeichnen... nach „Pfad ändern“
    let menus = fp_ui::menu::build_main_menu(st(&mut h));
    let modify = &menus.iter().find(|(t, _)| t == "Modifizieren").expect("Modifizieren").1;
    let pos = modify.iter().position(|n| matches!(n, Node::Item { action: ItemAction::Command(c), .. } if c == "modify.traceBitmap")).expect("Eintrag im Menü");
    assert!(matches!(&modify[pos - 1], Node::Sub { label, .. } if label == "Pfad ändern"), "nach „Pfad ändern“");
    match &modify[pos] {
        Node::Item { label, enabled, .. } => {
            assert_eq!(label, "Bitmap nachzeichnen...");
            assert!(*enabled, "mit Bitmap-Auswahl aktiv");
        }
        _ => unreachable!(),
    }
    // Kontextmenü „Objekt ausgewählt“
    let ctxm = fp_ui::canvas::context::build(st(&mut h), fp_ui::canvas::context::entries("6925"));
    assert_eq!(enabled(find_item(&ctxm, "modify.traceBitmap")), Some(true), "Kontextmenü");
    // ohne Bitmap gesperrt
    st(&mut h).session.execute("select.none", json!({})).expect("Auswahl aufheben");
    st(&mut h).session.execute("draw.rectangle", json!({"rect": [10, 10, 40, 40]})).expect("Rechteck");
    let menus = fp_ui::menu::build_main_menu(st(&mut h));
    assert_eq!(enabled(find_item(&menus.iter().find(|(t, _)| t == "Modifizieren").expect("Menü").1, "modify.traceBitmap")), Some(false), "ohne Bitmap gesperrt");
    let ctxm = fp_ui::canvas::context::build(st(&mut h), fp_ui::canvas::context::entries("6925"));
    assert_eq!(enabled(find_item(&ctxm, "modify.traceBitmap")), Some(false));
    st(&mut h).push(Action::Run("modify.traceBitmap".into(), json!({})));
    h.run_steps(2);
    assert!(st(&mut h).dialogs.trace.is_none(), "ohne Bitmap kein Dialog");
    // Bitmap wieder wählen: Inspektor-Knopf öffnet den Dialog
    let bmp = st(&mut h).session.active().and_then(|d| fp_engine::docops::own_layers(&d.doc, d.page).into_iter().flat_map(|(_, l)| l.cels.first().map(|c| c.elements.clone()).unwrap_or_default()).find(|e| matches!(e.kind, fp_model::ElementKind::Bitmap(_))).map(|e| e.id.0)).expect("Bitmap");
    st(&mut h).session.execute("select.set", json!({"ids": [bmp], "mode": "replace"})).expect("Auswahl");
    h.run_steps(3);
    h.get_by_label(&fp_i18n::t("trace-inspector-button")).click();
    h.run_steps(3);
    assert!(st(&mut h).dialogs.trace.is_some(), "Inspektor-Knopf öffnet den Dialog");
    assert_eq!(dialogs::last_dialog_id(&h.ctx).as_deref(), Some(trace::DIALOG_ID));
    wait_idle(&mut h);
    let d = st(&mut h).dialogs.trace.as_ref().expect("Dialog");
    let s = d.stats.clone().expect("Kennzahlen");
    assert!(s.paths >= 2 && s.points > 0 && s.colors >= 2 && s.match_pct > 95.0, "{s:?}");
}

#[test]
fn preset_change_restarts_trace_and_cancel_changes_nothing() {
    let _g = lock();
    let mut h = harness();
    let before = model(st(&mut h));
    let steps = undo_count(st(&mut h));
    open(&mut h);
    wait_idle(&mut h);
    let runs = st(&mut h).dialogs.trace.as_ref().map(|d| d.runs_started).unwrap_or(0);
    assert!(runs >= 1);
    h.get_by_label(&fp_i18n::t("trace-preset-photo")).click();
    h.run_steps(2);
    let d = st(&mut h).dialogs.trace.as_ref().expect("Dialog");
    assert_eq!(d.opts.preset, fp_trace::Preset::Photo);
    assert_eq!(d.runs_started, runs + 1, "Vorgabe wechseln startet genau eine neue Rechnung");
    wait_idle(&mut h);
    // dieselbe Vorgabe noch einmal: keine neue Rechnung
    h.get_by_label(&fp_i18n::t("trace-preset-photo")).click();
    h.run_steps(2);
    assert_eq!(st(&mut h).dialogs.trace.as_ref().map(|d| d.runs_started), Some(runs + 1));
    // Schwarzweiß: Regler „Schwelle“ statt „Farben“
    h.get_by_label(&fp_i18n::t("trace-preset-bw")).click();
    wait_idle(&mut h);
    assert!(st(&mut h).dialogs.trace.as_ref().is_some_and(|d| d.opts.color_mode == fp_trace::ColorMode::BlackWhite && d.stats.as_ref().is_some_and(|s| s.paths >= 1)));
    // Abbrechen: Dokument und Verlauf unverändert
    h.get_by_label(&fp_i18n::t("btn-cancel")).click();
    h.run_steps(3);
    assert!(st(&mut h).dialogs.trace.is_none(), "geschlossen");
    assert_eq!(undo_count(st(&mut h)), steps, "kein Verlaufsschritt");
    assert_eq!(model(st(&mut h)), before, "Dokument unverändert");
    // Esc bricht ebenso ab
    open(&mut h);
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
    assert!(st(&mut h).dialogs.trace.is_none(), "Esc schließt");
    assert_eq!(model(st(&mut h)), before);
}

#[test]
fn ok_adds_one_history_step_with_group_on_new_layer_and_remembers_options() {
    let _g = lock();
    let mut h = harness();
    let steps = undo_count(st(&mut h));
    let layers_before = {
        let d = st(&mut h).session.active().expect("Dokument");
        fp_engine::docops::own_layers(&d.doc, d.page).len()
    };
    open(&mut h);
    wait_idle(&mut h);
    h.get_by_label(&fp_i18n::t("trace-preset-illustration")).click();
    wait_idle(&mut h);
    h.get_by_label(&fp_i18n::t("btn-ok")).click();
    h.run_steps(3);
    assert!(st(&mut h).dialogs.trace.is_none(), "geschlossen");
    assert!(st(&mut h).dialogs.message.is_none(), "kein Fehler: {:?}", st(&mut h).dialogs.message);
    assert_eq!(undo_count(st(&mut h)), steps + 1, "genau ein Verlaufsschritt");
    let d = st(&mut h).session.active().expect("Dokument");
    let layers = fp_engine::docops::own_layers(&d.doc, d.page);
    assert_eq!(layers.len(), layers_before + 1, "neue Ebene");
    let sel: Vec<_> = d.selection.elements.iter().filter_map(|id| fp_engine::docops::element_on_page(&d.doc, d.page, *id)).collect();
    assert_eq!(sel.len(), 1);
    match &sel[0].kind {
        fp_model::ElementKind::Group(g) => assert!(g.children.len() >= 2, "Pfade in der Gruppe"),
        k => panic!("Gruppe erwartet, nicht {k:?}"),
    }
    // Original bleibt (Vorgabe „Original behalten“)
    let bitmaps = layers.iter().flat_map(|(_, l)| l.cels.first().map(|c| c.elements.clone()).unwrap_or_default()).filter(|e| matches!(e.kind, fp_model::ElementKind::Bitmap(_))).count();
    assert_eq!(bitmaps, 1);
    // gemerkt
    assert_eq!(trace::remembered_options(st(&mut h)).preset, fp_trace::Preset::Illustration);
    assert!(st(&mut h).session.prefs.get_bool(trace::PREF_KEEP, false));
    // Rückgängig stellt den Ausgangszustand her
    st(&mut h).session.execute("edit.undo", json!({})).expect("Rückgängig");
    assert_eq!(undo_count(st(&mut h)), steps);
    // neuer Dialog öffnet mit der gemerkten Vorgabe
    open(&mut h);
    assert_eq!(st(&mut h).dialogs.trace.as_ref().map(|d| d.opts.preset), Some(fp_trace::Preset::Illustration));
    wait_idle(&mut h);
}

#[test]
fn svg_is_written_through_the_file_dialog() {
    let _g = lock();
    let mut h = harness();
    let tmp = std::env::temp_dir().join(format!("fp-trace-svg-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let out = tmp.join("logo");
    st(&mut h).file_override = Some(fp_ui::files::FilePickOverride { open: vec![], save: Some(out.clone()) });
    open(&mut h);
    wait_idle(&mut h);
    h.get_by_label(&fp_i18n::t("trace-save-svg")).click();
    h.run_steps(4);
    let svg = std::fs::read_to_string(out.with_extension("svg")).expect("SVG geschrieben");
    assert!(svg.contains("<svg") && svg.contains("<path") && svg.contains("evenodd"));
    assert!(st(&mut h).dialogs.trace.is_some(), "Dialog bleibt offen");
    let _ = std::fs::remove_dir_all(&tmp);
}

fn texts(shape: &Shape, out: &mut Vec<TextShape>) {
    match shape {
        Shape::Text(t) => out.push(t.clone()),
        Shape::Vec(v) => v.iter().for_each(|s| texts(s, out)),
        _ => {}
    }
}

#[test]
fn layout_fits_in_de_en_all_themes_and_small_screens() {
    let _g = lock();
    for (w, hh, ppp) in [(1920.0, 1080.0, 1.0), (1024.0 / 1.5, 768.0 / 1.5, 1.5), (800.0, 600.0, 1.0)] {
        for lang in ["de", "en"] {
            for theme in [ThemeKind::Normal, ThemeKind::Dark, ThemeKind::Oled] {
                fp_i18n::set_language(lang);
                let mut h = harness_sized(w, hh, ppp);
                st(&mut h).set_theme(theme);
                h.run_steps(2);
                open(&mut h);
                // „Erweitert“ aufklappen: alle Optionen sichtbar (mit Bildlauf)
                if let Some(d) = st(&mut h).dialogs.trace.as_mut() {
                    d.advanced = true;
                }
                h.state_mut().only_dialogs = true;
                wait_idle(&mut h);
                let _ = dialogs::take_overflows(&h.ctx);
                h.run_steps(2);
                let over = dialogs::take_overflows(&h.ctx);
                assert!(over.is_empty(), "{lang} {theme:?} {w}×{hh}: Überlauf {over:?}");
                let rect = dialogs::dialog_rect(&h.ctx, trace::DIALOG_ID).expect("Dialog gezeichnet");
                let screen = h.ctx.content_rect();
                assert!(screen.expand(0.5).contains_rect(rect), "{lang} {theme:?} {w}×{hh}: Dialog {rect:?} passt nicht in {screen:?}");
                // keine Rohschlüssel, alle Texte im Dialog
                for cs in &h.output().shapes {
                    let mut v = Vec::new();
                    texts(&cs.shape, &mut v);
                    for t in v {
                        let s = t.galley.text();
                        assert!(!(s.starts_with("trace-") && !s.contains(' ')), "{lang}: Rohschlüssel {s}");
                        let r = t.visual_bounding_rect().intersect(cs.clip_rect);
                        if r.is_positive() {
                            assert!(rect.expand(1.0).contains_rect(r), "{lang} {theme:?} {w}×{hh}: Text „{s}“ {r:?} außerhalb {rect:?}");
                        }
                    }
                }
                // mit Aufräumen zweizeilige Kennzahlen; Foto mit Hinweis unter den Karten, ohne Aufräumen
                assert!(st(&mut h).dialogs.trace.as_ref().is_some_and(|d| d.stats.as_ref().is_some_and(|s| s.refine.is_some())), "Logo: Aufräum-Kennzahlen");
                if let Some(d) = st(&mut h).dialogs.trace.as_mut() {
                    d.set_preset(fp_trace::Preset::Photo);
                }
                wait_idle(&mut h);
                let _ = dialogs::take_overflows(&h.ctx);
                h.run_steps(2);
                let over = dialogs::take_overflows(&h.ctx);
                assert!(over.is_empty(), "Foto {lang} {theme:?} {w}×{hh}: Überlauf {over:?}");
                let rect2 = dialogs::dialog_rect(&h.ctx, trace::DIALOG_ID).expect("Dialog gezeichnet");
                assert!(screen.expand(0.5).contains_rect(rect2));
                h.state_mut().only_dialogs = false;
            }
        }
    }
}

// ---------------------------------------------------------------------- Aufräumen (VT-4)

fn dialog(h: &mut H) -> &mut trace::TraceDialog {
    st(h).dialogs.trace.as_mut().expect("Dialog")
}

/// Wie ein Mensch: Bedienelement in der Einstellungsspalte erst in den sichtbaren Bereich rollen,
/// dann anklicken.
fn click_in_view(h: &mut H, label: &str) {
    // als Kästchen suchen: Abschnittsüberschriften (z. B. „Aufräumen“) stehen jetzt ebenfalls im
    // Barrierefreiheitsbaum (FV-7)
    h.get_by_role_and_label(egui::accesskit::Role::CheckBox, label).scroll_to_me();
    h.run_steps(3);
    h.get_by_role_and_label(egui::accesskit::Role::CheckBox, label).click();
    h.run_steps(2);
}

#[test]
fn clean_up_switch_changes_stats() {
    let _g = lock();
    let mut h = harness();
    open(&mut h);
    wait_idle(&mut h);
    // Vorgabe „Logo“: Aufräumen an, Kennzahlen mit Grundformen und Knoten vorher → nachher
    let d = dialog(&mut h);
    assert_eq!(d.opts.preset, fp_trace::Preset::Logo);
    assert!(d.opts.refine.enabled, "Logo räumt auf");
    let on = d.stats.clone().expect("Kennzahlen");
    let rep = on.refine.clone().expect("Kennzahlen des Aufräumens");
    assert!(rep.shape_count() >= 2, "Kreis und Rechteck erkannt: {rep:?}");
    assert!(rep.nodes_before > rep.nodes_after && rep.nodes_after == on.points, "{rep:?} / {on:?}");
    let runs = d.runs_started;
    // Schalter „Aufräumen“ aus: genau eine neue Rechnung, keine Aufräum-Kennzahlen, mehr Knoten
    click_in_view(&mut h, &fp_i18n::t("trace-refine"));
    assert_eq!(dialog(&mut h).runs_started, runs + 1);
    wait_idle(&mut h);
    let d = dialog(&mut h);
    assert!(!d.opts.refine.enabled);
    let off = d.stats.clone().expect("Kennzahlen");
    assert!(off.refine.is_none(), "ohne Aufräumen keine Grundformen-Zeile");
    assert!(off.points > on.points, "ohne Aufräumen mehr Knoten: {} gegen {}", off.points, on.points);
    // Einzelschritt im Bereich „Erweitert“ (gesperrt, solange Aufräumen aus ist)
    d.advanced = true;
    h.run_steps(2);
    click_in_view(&mut h, &fp_i18n::t("trace-refine-shapes"));
    assert!(dialog(&mut h).opts.refine.shapes, "gesperrter Schalter ändert nichts");
    click_in_view(&mut h, &fp_i18n::t("trace-refine"));
    wait_idle(&mut h);
    click_in_view(&mut h, &fp_i18n::t("trace-refine-shapes"));
    wait_idle(&mut h);
    let d = dialog(&mut h);
    assert!(d.opts.refine.enabled && !d.opts.refine.shapes);
    let rep = d.stats.as_ref().and_then(|s| s.refine.clone()).expect("Kennzahlen");
    assert_eq!(rep.shape_count(), 0, "ohne Grundformen keine erkannt");
    // OK übernimmt die Schalter in den Befehl (Parameter `refine`)
    assert_eq!(d.command_params()["refine"]["shapes"], json!(false));
    assert_eq!(d.command_params()["refine"]["enabled"], json!(true));
}

#[test]
fn ok_without_changes_reuses_the_preview() {
    let _g = lock();
    let mut h = harness();
    let steps = undo_count(st(&mut h));
    open(&mut h);
    wait_idle(&mut h);
    let paths = dialog(&mut h).stats.as_ref().map(|s| s.paths).expect("Kennzahlen");
    assert!(dialog(&mut h).reusable_result().is_some(), "Vorschau passt zu den Optionen");
    let computed = fp_engine::commands::trace::computed_traces();
    let runs = dialog(&mut h).runs_started;
    h.get_by_label(&fp_i18n::t("btn-ok")).click();
    h.run_steps(3);
    assert!(st(&mut h).dialogs.trace.is_none() && st(&mut h).dialogs.message.is_none(), "{:?}", st(&mut h).dialogs.message);
    assert_eq!(fp_engine::commands::trace::computed_traces(), computed, "OK ohne Änderung rechnet nicht neu");
    assert_eq!(undo_count(st(&mut h)), steps + 1, "ein Verlaufsschritt");
    let d = st(&mut h).session.active().expect("Dokument");
    let g = d.selection.elements.iter().filter_map(|id| fp_engine::docops::element_on_page(&d.doc, d.page, *id)).next().expect("Gruppe");
    match &g.kind {
        fp_model::ElementKind::Group(g) => assert_eq!(g.children.len(), paths, "Ergebnis der Vorschau übernommen"),
        k => panic!("Gruppe erwartet, nicht {k:?}"),
    }
    assert!(runs >= 1);
    // Optionen weichen von der letzten Vorschau ab (z. B. OK, bevor die neue fertig ist): der
    // Befehl rechnet selbst
    st(&mut h).session.execute("edit.undo", json!({})).expect("Rückgängig");
    open(&mut h);
    wait_idle(&mut h);
    assert!(dialog(&mut h).reusable_result().is_some());
    dialog(&mut h).opts.colors = 9;
    assert!(dialog(&mut h).reusable_result().is_none(), "Vorschau gehört zu alten Optionen");
    let computed = fp_engine::commands::trace::computed_traces();
    h.get_by_label(&fp_i18n::t("btn-ok")).click();
    h.run_steps(3);
    assert!(st(&mut h).dialogs.trace.is_none());
    assert_eq!(fp_engine::commands::trace::computed_traces(), computed + 1, "neu gerechnet");
    assert_eq!(undo_count(st(&mut h)), steps + 1);
}

#[test]
fn old_remembered_options_get_the_presets_clean_up() {
    let _g = lock();
    // Eintrag aus der Zeit vor dem Aufräumen (ohne `refine`)
    let mut old = serde_json::to_value(fp_trace::TraceOptions::from_preset(fp_trace::Preset::Logo)).expect("JSON");
    old.as_object_mut().expect("Objekt").remove("refine");
    old["colors"] = json!(12);
    let o = trace::options_from_json(&old.to_string());
    assert_eq!(o.colors, 12, "gemerkte Werte bleiben");
    assert_eq!(o.refine, fp_trace::RefineOptions::logo(), "Logo: Aufräumen wie die Vorgabe (an)");
    old["preset"] = json!("photo");
    assert!(!trace::options_from_json(&old.to_string()).refine.enabled, "Foto: Vorgabe ohne Aufräumen");
    // ausdrücklich ausgeschaltet bleibt aus; leer/ungültig → Vorgabe „Logo“
    let mut off = fp_trace::TraceOptions::from_preset(fp_trace::Preset::Logo);
    off.refine.enabled = false;
    assert!(!trace::options_from_json(&serde_json::to_string(&off).expect("JSON")).refine.enabled);
    assert_eq!(trace::options_from_json(""), fp_trace::TraceOptions::default());
    // über die Voreinstellungen beim Öffnen des Dialogs
    let mut h = harness();
    old["preset"] = json!("logo");
    st(&mut h).session.prefs.set(trace::PREF_OPTIONS, fp_engine::prefs::PrefValue::Str(old.to_string()));
    open(&mut h);
    let d = dialog(&mut h);
    assert!(d.opts.refine.enabled && d.opts.colors == 12, "{:?}", d.opts);
    wait_idle(&mut h);
    assert!(dialog(&mut h).stats.as_ref().is_some_and(|s| s.refine.is_some()));
}
