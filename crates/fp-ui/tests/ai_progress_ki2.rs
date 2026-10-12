//! Fortschrittsanzeige und Feineinstellungen der KI-Werkzeuge (KI-2, Nutzerbefund N21), offscreen
//! mit egui_kittest:
//! * Karte in 3 Designs × de/en: Titel = Werkzeug, Modell klein darunter, Stufen, Balken,
//!   Abbrechen; alles innerhalb der Karte, kein Überlauf, keine Rohschlüssel, feste Lage (kein
//!   Flackern bei wechselndem Fortschritt)
//! * Abbrechen (Knopf und Esc)
//! * kein Aufblitzen bei schnellen Läufen (< 300 ms), sanftes Ausblenden nach langen
//! * Inspektor-Felder je KI-Werkzeug (Toleranz, Kante, Variante, Ausdehnen, Glätten), passend
//!   eingepasst in de/en; mit Modellen: ein Wert im Inspektor stellt die Auswahl nach
//!
//! Wie alle Testläufe mit `FP_NO_SHELL_OPEN=1` und eigenem `LOCALAPPDATA` starten.

use egui::epaint::{Shape, TextShape};
use egui::{Event, Modifiers, PointerButton, Pos2, Rect};
use egui_kittest::kittest::{By, Queryable};
use egui_kittest::Harness;
use fp_engine::commands::ai::fp_ai::{self, models::ModelId, Stage};
use fp_engine::ToolId;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::dialogs;
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

type H = Harness<'static, FirepixelApp>;

/// Foto aus den Testdaten von fp-ai (CC0), sonst ein Ersatzbild.
fn photo() -> fp_raster::RgbaImage {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../fp-ai/testdata/hund_gras.jpg");
    if let Ok(img) = image::open(&p) {
        let img = img.to_rgba8();
        let (w, h) = (img.width() as i32, img.height() as i32);
        if let Ok(mut out) = fp_raster::RgbaImage::new(fp_geom::IRect::from_xywh(0, 0, w, h)) {
            for (d, s) in out.data_mut().iter_mut().zip(img.pixels()) {
                *d = s.0;
            }
            return out;
        }
    }
    let mut img = fp_raster::RgbaImage::new(fp_geom::IRect::from_xywh(0, 0, 640, 360)).unwrap_or_else(|_| fp_raster::RgbaImage::empty_at(0, 0));
    for (i, px) in img.data_mut().iter_mut().enumerate() {
        let (x, y) = ((i % 640) as f32, (i / 640) as f32);
        *px = if (x - 220.0).hypot(y - 180.0) < 90.0 { [230, 220, 200, 255] } else { [40, 110 + ((x as u32 / 7) % 20) as u8, 40, 255] };
    }
    img
}

fn harness_full(w: f32, h: f32, theme: ThemeKind, wgpu: bool) -> H {
    fp_engine::shell::disable();
    fp_ai::download::disable();
    let mut s = fp_engine::Session::default();
    let img = photo();
    let _ = s.execute("file.new", json!({"width": img.width(), "height": img.height(), "background": "#ffffff"}));
    s.clipboard.clear();
    s.clipboard.pixels = Some(img);
    let _ = s.execute("edit.paste", json!({}));
    let _ = s.execute("select.none", json!({}));
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = false;
    app.state.os_clipboard = false;
    app.state.set_theme(theme);
    let b = Harness::builder().with_size(egui::vec2(w, h)).with_pixels_per_point(1.0).with_max_steps(8).with_step_dt(1.0 / 60.0);
    let b = if wgpu { b.wgpu() } else { b };
    let mut h = b.build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    h
}

fn harness(theme: ThemeKind) -> H {
    harness_full(1400.0, 900.0, theme, false)
}

fn st(h: &mut H) -> &mut AppState {
    &mut h.state_mut().state
}

/// Lauf, der bei `stage`/`p` stehen bleibt, bis er abgebrochen wird.
fn demo_hold(h: &mut H, cmd: &str, model: ModelId, stage: Stage, p: f32) {
    dialogs::models::start_demo(st(h), cmd, model, move |pr| {
        pr.set_stage(stage);
        pr.set(p);
        while !pr.cancel.is_cancelled() {
            std::thread::sleep(Duration::from_millis(10));
        }
        Err(fp_ai::AiError::Cancelled)
    });
}

/// Echte Zeit vergehen lassen und dabei Bilder zeichnen.
fn run_for(h: &mut H, d: Duration) {
    let t = Instant::now();
    while t.elapsed() < d {
        std::thread::sleep(Duration::from_millis(15));
        h.step();
    }
}

fn card_rect(h: &H) -> Option<Rect> {
    h.ctx.memory(|m| m.area_rect(egui::Id::new("fp-ai-progress")))
}

fn texts(shape: &Shape, out: &mut Vec<TextShape>) {
    match shape {
        Shape::Text(t) => out.push(t.clone()),
        Shape::Vec(v) => v.iter().for_each(|s| texts(s, out)),
        _ => {}
    }
}

/// Alle gezeichneten Texte innerhalb von `r`.
fn texts_in(h: &H, r: Rect) -> Vec<(String, Rect)> {
    let mut out = Vec::new();
    for cs in &h.output().shapes {
        let mut v = Vec::new();
        texts(&cs.shape, &mut v);
        for t in v {
            let b = t.visual_bounding_rect().intersect(cs.clip_rect);
            if b.is_positive() && r.expand(40.0).intersects(b) {
                out.push((t.galley.text().to_string(), b));
            }
        }
    }
    out
}

fn stop(h: &mut H) {
    st(h).dialogs.ai.cancel();
    run_for(h, Duration::from_millis(120));
}

#[test]
fn progress_card_in_all_themes_and_languages() {
    let _g = lock();
    for lang in ["de", "en"] {
        for theme in [ThemeKind::Normal, ThemeKind::Dark, ThemeKind::Oled] {
            fp_i18n::set_language(lang);
            let mut h = harness(theme);
            demo_hold(&mut h, "select.aiSubject", ModelId::BiRefNet, Stage::Analyze, 0.5);
            run_for(&mut h, Duration::from_millis(500));
            let _ = dialogs::take_overflows(&h.ctx);
            h.run_steps(2);
            assert!(dialogs::take_overflows(&h.ctx).is_empty(), "{lang} {theme:?}: Überlauf");
            let card = card_rect(&h).expect("Karte gezeichnet");
            assert!(h.ctx.content_rect().contains_rect(card), "{lang} {theme:?}: {card:?}");
            // schwebt über der Leinwand, nicht über Inspektor oder Bedienfeldern
            let area = { let s = st(&mut h); s.session.active().and_then(|d| s.canvases.get(&d.id)).and_then(|c| c.area) }.expect("Leinwand");
            assert!(area.contains_rect(card), "{lang} {theme:?}: Karte {card:?} außerhalb der Leinwand {area:?}");
            let cancel = fp_i18n::t("btn-cancel");
            assert!(h.get_all(By::new().label(&cancel)).next().is_some(), "{lang}: Abbrechen-Knopf");
            let ts = texts_in(&h, card);
            let title = fp_i18n::t("dlg-ai-subject");
            let model = fp_ai::models::info(ModelId::BiRefNet).display_name();
            for want in [title.as_str(), model.as_str(), &fp_i18n::t("dlg-ai-stage-analyze"), &fp_i18n::t("dlg-ai-step-mask"), &fp_i18n::tr!("dlg-ai-percent", n = 50)] {
                assert!(ts.iter().any(|(t, _)| t == want), "{lang} {theme:?}: „{want}“ fehlt in {ts:?}");
            }
            // Titel ist der Werkzeugname allein, das Modell steht nicht mehr geklammert im Titel
            assert!(!ts.iter().any(|(t, _)| t.contains(&format!("({model})"))), "{ts:?}");
            let ours: Vec<String> = [title.clone(), model.clone(), fp_i18n::t("dlg-ai-stage-analyze"), fp_i18n::t("dlg-ai-step-load"), fp_i18n::t("dlg-ai-step-analyze"), fp_i18n::t("dlg-ai-step-mask"), cancel.clone()].into();
            for (t, r) in &ts {
                assert!(!t.starts_with("dlg-ai-"), "{lang}: Rohschlüssel {t}");
                if ours.contains(t) || t.ends_with(" s") || t.ends_with('%') {
                    assert!(card.expand(1.0).contains_rect(*r), "{lang} {theme:?}: „{t}“ {r:?} ragt aus {card:?}");
                }
            }
            stop(&mut h);
            assert!(!st(&mut h).dialogs.ai.busy());
        }
    }
}

#[test]
fn card_does_not_move_or_resize_while_progress_changes() {
    let _g = lock();
    let mut h = harness(ThemeKind::Dark);
    dialogs::models::start_demo(st(&mut h), "select.aiObject", ModelId::Sam2Tiny, |pr| {
        pr.set_stage(Stage::LoadModel);
        std::thread::sleep(Duration::from_millis(300));
        pr.set_stage(Stage::Analyze);
        let mut i = 0u32;
        while !pr.cancel.is_cancelled() {
            pr.set((i as f32 / 300.0).min(0.99));
            i += 1;
            std::thread::sleep(Duration::from_millis(5));
        }
        Err(fp_ai::AiError::Cancelled)
    });
    run_for(&mut h, Duration::from_millis(400));
    let r0 = card_rect(&h).expect("Karte");
    let mut seen = std::collections::HashSet::new();
    for _ in 0..40 {
        run_for(&mut h, Duration::from_millis(20));
        assert_eq!(card_rect(&h), Some(r0), "Karte bleibt an Ort und Größe");
        let ts = texts_in(&h, r0);
        if let Some((p, _)) = ts.iter().find(|(t, _)| t.ends_with('%')) {
            seen.insert(p.clone());
        }
    }
    assert!(seen.len() > 3, "Prozent ändert sich: {seen:?}");
    stop(&mut h);
}

#[test]
fn cancel_with_button_and_escape() {
    let _g = lock();
    let mut h = harness(ThemeKind::Normal);
    demo_hold(&mut h, "select.aiObject", ModelId::Sam2Tiny, Stage::Analyze, 0.3);
    run_for(&mut h, Duration::from_millis(450));
    h.get_by_label("Abbrechen").click();
    run_for(&mut h, Duration::from_millis(200));
    assert!(!st(&mut h).dialogs.ai.busy(), "Knopf bricht ab");
    let status = st(&mut h).status.clone().map(|s| s.0).unwrap_or_default();
    assert!(status.contains("abgebrochen"), "{status}");
    assert!(st(&mut h).dialogs.ai.fading.is_none(), "nach Abbrechen kein Ausblenden mit „Fertig“");
    // Esc
    demo_hold(&mut h, "select.aiObject", ModelId::Sam2Tiny, Stage::Analyze, 0.3);
    run_for(&mut h, Duration::from_millis(400));
    h.input_mut().events.push(Event::Key { key: egui::Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE });
    h.step();
    run_for(&mut h, Duration::from_millis(200));
    assert!(!st(&mut h).dialogs.ai.busy(), "Esc bricht ab");
}

#[test]
fn fast_runs_do_not_flash_and_slow_runs_fade_out() {
    let _g = lock();
    let mut h = harness(ThemeKind::Oled);
    let f0 = st(&mut h).dialogs.ai.progress_frames;
    dialogs::models::start_demo(st(&mut h), "select.aiSubject", ModelId::BiRefNetLite, |pr| {
        pr.set_stage(Stage::Analyze);
        std::thread::sleep(Duration::from_millis(120));
        pr.set(1.0);
        Ok(())
    });
    run_for(&mut h, Duration::from_millis(600));
    assert!(!st(&mut h).dialogs.ai.busy());
    assert_eq!(st(&mut h).dialogs.ai.progress_frames, f0, "kurzer Lauf: Anzeige nie sichtbar");
    assert!(st(&mut h).dialogs.ai.fading.is_none());
    assert!(h.query_all(By::new().label("Abbrechen")).next().is_none(), "kein Abbrechen-Knopf zu sehen");
    // langer Lauf: sichtbar, danach kurz „Fertig“ und weg
    dialogs::models::start_demo(st(&mut h), "select.aiSubject", ModelId::BiRefNetLite, |pr| {
        pr.set_stage(Stage::Analyze);
        for i in 0..50 {
            pr.set(i as f32 / 50.0);
            std::thread::sleep(Duration::from_millis(12));
        }
        Ok(())
    });
    run_for(&mut h, Duration::from_millis(180));
    assert_eq!(st(&mut h).dialogs.ai.progress_frames, f0, "vor 300 ms unsichtbar");
    run_for(&mut h, Duration::from_millis(400));
    assert!(st(&mut h).dialogs.ai.progress_frames > f0, "danach sichtbar");
    run_for(&mut h, Duration::from_millis(500));
    assert!(!st(&mut h).dialogs.ai.busy());
    assert!(st(&mut h).dialogs.ai.fading.is_none(), "ausgeblendet");
}

/// Inspektor-Beschriftungen des aktuellen Werkzeugs (Layout-Protokoll).
fn inspector_audit(h: &mut H) -> Vec<fp_ui::inspector::fields::TextAudit> {
    fp_ui::inspector::fields::set_audit(&h.ctx, true);
    h.run_steps(2);
    let a = fp_ui::inspector::fields::take_audit(&h.ctx);
    fp_ui::inspector::fields::set_audit(&h.ctx, false);
    // je Bild einmal protokolliert: doppelte Einträge zusammenfassen
    let mut seen = std::collections::HashSet::new();
    a.into_iter().filter(|e| e.inspector).filter(|e| seen.insert((e.kind, e.text.clone(), e.drawn.min.x as i32, e.drawn.min.y as i32))).collect()
}

#[test]
fn inspector_fields_per_ai_tool() {
    let _g = lock();
    for lang in ["de", "en"] {
        fp_i18n::set_language(lang);
        let mut h = harness(ThemeKind::Normal);
        let l = |k: &str| fp_i18n::t(k);
        for (tool, want, absent) in [
            (ToolId::AiObject, vec![l("pi-tolerance"), l("pi-edge"), l("pi-ai-variant"), l("pi-ai-expand"), l("pi-ai-smooth")], vec![l("pi-ai-radius")]),
            (ToolId::AiSubject, vec![l("pi-tolerance"), l("pi-edge"), l("pi-ai-expand"), l("pi-ai-smooth")], vec![l("pi-ai-variant"), l("pi-ai-radius")]),
            (ToolId::AiRefineEdge, vec![l("pi-ai-radius"), l("pi-ai-output")], vec![l("pi-tolerance"), l("pi-ai-variant"), l("pi-ai-expand")]),
        ] {
            let _ = st(&mut h).session.set_tool(tool);
            let a = inspector_audit(&mut h);
            let labels: Vec<&str> = a.iter().filter(|e| e.kind == "label").map(|e| e.text.as_str()).collect();
            for w in &want {
                assert!(labels.contains(&w.as_str()), "{lang} {tool:?}: „{w}“ fehlt in {labels:?}");
            }
            for w in &absent {
                assert!(!labels.contains(&w.as_str()), "{lang} {tool:?}: „{w}“ gehört nicht dazu");
            }
            // nichts mit „…“ gekürzt, nichts außerhalb des Quadranten, keine Überlappung
            for e in &a {
                assert!(e.fit < 2, "{lang} {tool:?}: „{}“ gekürzt", e.text);
                if e.kind == "label" {
                    assert!(e.clip.expand(0.5).contains_rect(e.drawn), "{lang} {tool:?}: „{}“ {:?} außerhalb {:?}", e.text, e.drawn, e.clip);
                    assert!(e.avail.expand(1.0).contains_rect(e.drawn), "{lang} {tool:?}: „{}“ {:?} ragt aus der Fläche {:?}", e.text, e.drawn, e.avail);
                }
            }
            let boxes: Vec<&fp_ui::inspector::fields::TextAudit> = a.iter().filter(|e| e.kind == "label" || e.kind == "field" || e.kind == "button").collect();
            for (i, x) in boxes.iter().enumerate() {
                for y in &boxes[i + 1..] {
                    // Text im eigenen Rahmen (Knopf, Feld) ist keine Überlappung
                    if x.drawn.expand(0.5).contains_rect(y.drawn) || y.drawn.expand(0.5).contains_rect(x.drawn) {
                        continue;
                    }
                    let ov = x.drawn.intersect(y.drawn);
                    assert!(!(ov.width() > 1.0 && ov.height() > 1.0), "{lang} {tool:?}: {} „{}“ {:?} überlappt {} „{}“ {:?}", x.kind, x.text, x.drawn, y.kind, y.text, y.drawn);
                }
            }
        }
    }
}

#[test]
fn inspector_values_are_remembered_and_passed_as_parameters() {
    let _g = lock();
    let mut h = harness(ThemeKind::Normal);
    let _ = st(&mut h).session.set_tool(ToolId::AiObject);
    // Wert über das Feld wie ein Mensch: ins Feld klicken, alles markieren, tippen, Eingabe
    let a = inspector_audit(&mut h);
    let lab = a.iter().find(|e| e.kind == "label" && e.text == fp_i18n::t("pi-ai-expand")).expect("Ausdehnen:").avail;
    let field = Pos2::new(lab.right() + 3.0 + 12.0, lab.center().y);
    for e in [
        Event::PointerMoved(field),
        Event::PointerButton { pos: field, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE },
        Event::PointerButton { pos: field, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE },
    ] {
        h.input_mut().events.push(e);
        h.step();
    }
    h.input_mut().events.push(Event::Key { key: egui::Key::A, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::COMMAND });
    h.step();
    h.input_mut().events.push(Event::Text("-7".into()));
    h.step();
    h.input_mut().events.push(Event::Key { key: egui::Key::Enter, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE });
    h.run_steps(3);
    assert_eq!(st(&mut h).session.prefs.get_int("AiObjectExpand", 0), -7, "Feld setzt die Voreinstellung");
    st(&mut h).session.prefs.set("AiObjectVariant", fp_engine::PrefValue::Str("large".into()));
    st(&mut h).session.prefs.set("AiObjectEdge", fp_engine::PrefValue::Str("hard".into()));
    // mit Modellen: Klick auf das Foto → Ergebnis trägt die Werte, Änderung im Inspektor stellt nach
    if !fp_ai::models::available(ModelId::Sam2Tiny) {
        eprintln!("KI-Modelle fehlen – Teil mit Modell übersprungen");
        return;
    }
    let s = st(&mut h);
    s.session.execute("select.aiObject", json!({"point": [300, 400], "expand": -7, "variant": "large", "edge": "hard"})).expect("Objekt");
    let before = s.session.active().and_then(|d| d.selection.pixels.clone()).expect("Auswahl");
    s.session.prefs.set("AiObjectExpand", fp_engine::PrefValue::Int(4));
    assert!(dialogs::models::retune(s, "select.aiObject"), "nachgestellt");
    let after = s.session.active().and_then(|d| d.selection.pixels.clone()).expect("Auswahl");
    assert!(after.tight_bounds().width() > before.tight_bounds().width() + 15, "{:?} → {:?}", before.tight_bounds(), after.tight_bounds());
    s.session.prefs.set("AiObjectExpand", fp_engine::PrefValue::Int(0));
    s.session.prefs.set("AiObjectVariant", fp_engine::PrefValue::Str("auto".into()));
    s.session.prefs.set("AiObjectEdge", fp_engine::PrefValue::Str("antiAlias".into()));
}

/// Bilder für NUTZERBEFUNDE (N21): Fortschrittskarte und Inspektor in Normal/Dunkel/OLED, de/en.
/// Nur auf Wunsch: `cargo test -p fp-ui --test ai_progress_ki2 -- --ignored`.
#[test]
#[ignore]
fn n21_screenshots() {
    let _g = lock();
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/screenshots/nutzerbefunde");
    for lang in ["de", "en"] {
        fp_i18n::set_language(lang);
        for (theme, tname) in [(ThemeKind::Normal, "normal"), (ThemeKind::Dark, "dunkel"), (ThemeKind::Oled, "oled")] {
            let mut h = harness_full(1600.0, 1000.0, theme, true);
            let _ = st(&mut h).session.set_tool(ToolId::AiSubject);
            demo_hold(&mut h, "select.aiSubject", ModelId::BiRefNet, Stage::Analyze, 0.46);
            run_for(&mut h, Duration::from_millis(1600));
            h.run_steps(2);
            let card = card_rect(&h).expect("Karte");
            match h.render() {
                Ok(img) => {
                    let r = card.expand2(egui::vec2(140.0, 60.0)).intersect(h.ctx.content_rect());
                    let c = image::imageops::crop_imm(&img, r.left() as u32, r.top() as u32, r.width() as u32, r.height() as u32).to_image();
                    let _ = c.save(out.join(format!("N21_nachher_fortschritt_{tname}_{lang}.png")));
                    // Inspektor unten (volle Breite des Fensters, untere 130 px)
                    let (w, hh) = (img.width(), img.height());
                    let i = image::imageops::crop_imm(&img, 0, hh.saturating_sub(150), w.min(1100), 150).to_image();
                    let _ = i.save(out.join(format!("N21_nachher_inspektor_motiv_{tname}_{lang}.png")));
                }
                Err(e) => eprintln!("kein Grafikadapter: {e}"),
            }
            stop(&mut h);
            let _ = st(&mut h).session.set_tool(ToolId::AiObject);
            h.run_steps(3);
            if let Ok(img) = h.render() {
                let (w, hh) = (img.width(), img.height());
                let i = image::imageops::crop_imm(&img, 0, hh.saturating_sub(150), w.min(1100), 150).to_image();
                let _ = i.save(out.join(format!("N21_nachher_inspektor_objekt_{tname}_{lang}.png")));
            }
        }
    }
}
