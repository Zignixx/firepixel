//! Bildvorschau, Paket P3 „Datei, Animation, Exportbereich“ (re\17-bildvorschau.md §3, §4, §6,
//! §10 P3, BV-3): Skalieren und Exportbereich ändern Bild und Größe genau wie der Export, Werkzeug
//! „Exportbereich“ mit Griffen (nur im aktiven Feld), Registerkarte Animation (Verzögerung,
//! Löschmethode, Auge, Laufzeit, Wiederholung), Abspielen und Statuswahl; beim Öffnen Status 1 wie
//! der Export. Wie alle Testläufe mit `FP_NO_SHELL_OPEN=1` starten (AGENTEN_REGELN §4a).

use egui::epaint::Shape;
use egui::{vec2, Pos2, Rect};
use egui_kittest::kittest::{By, NodeT, Queryable};
use egui_kittest::Harness;
use fp_export::options::{ColorMode, ExportFormat, OptimizeSettings};
use fp_export::render::{Background, PageRenderer, RenderRequest};
use fp_ui::app_state::{Action, AppState, UiSettings};
use fp_ui::dialogs::image_preview::{crop, pane, worker, ImagePreview, Tool, ViewMode};
use fp_ui::theme::ThemeKind;
use fp_ui::FirepixelApp;
use serde_json::json;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

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
    fp_engine::shell::disable();
    Guard(g)
}

type H = Harness<'static, FirepixelApp>;

/// Bild 240 × 160 mit Verläufen auf weißer Leinwand; mit `frames` > 1 weitere (leere) Status.
fn session(frames: usize) -> fp_engine::Session {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 240, "height": 160, "background": "#ffffff"})).expect("Dokument");
    let mut img = fp_raster::RgbaImage::filled(fp_geom::IRect::from_xywh(0, 0, 240, 160), fp_color::Rgba8::new(255, 255, 255, 255)).expect("Bild");
    let w = img.width();
    for (i, px) in img.data_mut().iter_mut().enumerate() {
        let (x, y) = ((i % w) as f64, (i / w) as f64);
        *px = [(x * 255.0 / 240.0) as u8, (y * 255.0 / 160.0) as u8, ((x + y) as u32 % 256) as u8, 255];
    }
    s.clipboard.clear();
    s.clipboard.pixels = Some(img);
    s.execute("edit.paste", json!({})).expect("Bitmap einfügen");
    s.execute("select.none", json!({})).ok();
    if frames > 1 {
        s.execute("state.add", json!({"count": frames - 1})).expect("Status");
    }
    s
}

fn harness(frames: usize) -> H {
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, session(frames), UiSettings { theme: ThemeKind::Normal, ..UiSettings::default() }, false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let b = Harness::builder().with_size(vec2(1400.0, 900.0)).with_pixels_per_point(1.0).with_max_steps(8).with_step_dt(1.0 / 60.0);
    let mut h = b.build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(3);
    h
}

fn st(h: &mut H) -> &mut AppState {
    &mut h.state_mut().state
}

fn dlg(h: &mut H) -> &mut ImagePreview {
    st(h).dialogs.preview.as_mut().expect("Bildvorschau offen")
}

fn settings(h: &mut H, i: usize) -> OptimizeSettings {
    dlg(h).panes[i].settings.clone()
}

fn open(h: &mut H) {
    st(h).push(Action::Run("file.imagePreview".into(), json!({})));
    h.run_steps(3);
    assert!(st(h).dialogs.preview.is_some(), "Dialog offen");
    wait_idle(h);
}

/// One frame with the UI clock stopped (egui keeps an explicit `RawInput::time`, so reset it).
fn step_frozen(h: &mut H) {
    let t = h.ctx.input(|i| i.time);
    h.input_mut().time = Some(t);
    h.run_steps(1);
    h.input_mut().time = None;
}

fn wait_idle(h: &mut H) {
    let t0 = Instant::now();
    loop {
        h.run_steps(1);
        let busy = st(h).dialogs.preview.as_ref().is_some_and(|d| d.is_busy());
        if !busy || t0.elapsed() > Duration::from_secs(60) {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    h.run_steps(2);
}

fn click_at(h: &mut H, p: Pos2) {
    h.event(egui::Event::PointerMoved(p));
    h.run_steps(1);
    h.event(egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() });
    h.run_steps(1);
    h.event(egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() });
    h.run_steps(2);
}

/// Ziehen mit der Maus über mehrere Bilder (realistische Eingabe, AGENTEN_REGELN §4c).
fn drag(h: &mut H, from: Pos2, to: Pos2) {
    h.event(egui::Event::PointerMoved(from));
    h.run_steps(1);
    h.event(egui::Event::PointerButton { pos: from, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() });
    h.run_steps(1);
    for k in 1..=8 {
        let p = from + (to - from) * (k as f32 / 8.0);
        h.event(egui::Event::PointerMoved(p));
        h.run_steps(1);
    }
    h.event(egui::Event::PointerButton { pos: to, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() });
    h.run_steps(2);
}

/// Bereich der Bildvorschau (Hauptmenü und Bedienfelder haben teils dieselben Namen, z. B. „Datei“).
fn dialog_area(h: &mut H) -> Rect {
    let d = dlg(h);
    let f = d.layout.iter().map(|(_, l)| l.frame).reduce(|a, b| a.union(b)).expect("Felder gezeichnet");
    Rect::from_min_max(f.min - vec2(480.0, 40.0), f.max + vec2(10.0, 36.0))
}

fn in_dialog(h: &mut H, label: &str) -> Rect {
    let area = dialog_area(h);
    h.query_all(By::new().label(label)).map(|n| n.rect()).find(|r| area.contains(r.center())).unwrap_or_else(|| panic!("„{label}“ nicht im Dialog"))
}

/// Element mit diesem Namen im Dialog anklicken (Barrierefreiheits-Aktion wie ein Mausklick).
fn click(h: &mut H, label: &str) {
    let area = dialog_area(h);
    let node = h.query_all(By::new().label(label)).find(|n| area.contains(n.rect().center())).unwrap_or_else(|| panic!("„{label}“ nicht im Dialog"));
    node.click();
    h.run_steps(3);
}

fn label_rect(h: &mut H, label: &str) -> Rect {
    in_dialog(h, label)
}

fn is_enabled(h: &mut H, label: &str) -> bool {
    let area = dialog_area(h);
    h.query_all(By::new().label(label)).find(|n| area.contains(n.rect().center())).is_some_and(|n| !n.accesskit_node().is_disabled())
}

/// In das Zahlenfeld mit dem Tooltip `tip_key` tippen (jede Taste ein Ereignis) und mit Tab
/// verlassen (Enter würde wie im Original den Standardknopf OK auslösen).
fn type_into(h: &mut H, tip_key: &str, text: &str) {
    let r = label_rect(h, &fp_i18n::t(tip_key));
    click_at(h, pos(r.left() + 8.0, r.center().y));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    for c in text.chars() {
        h.event(egui::Event::Text(c.to_string()));
        h.run_steps(1);
    }
    h.key_press(egui::Key::Tab);
    h.run_steps(3);
}

fn pos(x: f32, y: f32) -> Pos2 {
    Pos2::new(x, y)
}

fn painted_texts(h: &H) -> Vec<String> {
    fn walk(s: &Shape, out: &mut Vec<String>) {
        match s {
            Shape::Text(t) => out.push(t.galley.text().to_string()),
            Shape::Vec(v) => v.iter().for_each(|x| walk(x, out)),
            _ => {}
        }
    }
    let mut v = Vec::new();
    for cs in &h.output().shapes {
        walk(&cs.shape, &mut v);
    }
    v
}

fn result(h: &mut H, i: usize) -> Arc<worker::PaneResult> {
    dlg(h).panes[i].result.clone().expect("Ergebnis")
}

/// Bytes des echten Exports (Status 1) mit dieser Einstellung.
fn real_export(h: &mut H, s: &OptimizeSettings) -> usize {
    let st = st(h);
    let d = st.session.active().expect("doc");
    let r = fp_ui::panels::optimize::ExportRenderer { normal: Arc::clone(&st.renderer) };
    fp_export::doc_export::export_whole(&r, &d.doc, d.page, s, &fp_export::export::EncodeExtras::default()).expect("Export").len()
}

/// Bild, das der Export kodiert: Status 1 gerendert, dann Exportbereich und Skalierung.
fn export_pixels(h: &mut H, s: &OptimizeSettings) -> fp_raster::RgbaImage {
    let st = st(h);
    let d = st.session.active().expect("doc");
    let r = fp_ui::panels::optimize::ExportRenderer { normal: Arc::clone(&st.renderer) };
    let img = r.render(&d.doc, &RenderRequest { page: d.page, frame: 0, background: Background::Canvas, ..Default::default() }).expect("render");
    fp_export::doc_export::apply_crop_scale(img, s)
}

fn png32() -> OptimizeSettings {
    OptimizeSettings { format: ExportFormat::Png, color_mode: ColorMode::Rgba32, ..OptimizeSettings::default() }
}

/// Angezeigtes Bild und Größe des Feldes = Bild und Größe des echten Exports.
fn assert_like_export(h: &mut H, i: usize, what: &str) {
    let s = settings(h, i);
    let res = result(h, i);
    let img = res.image.clone().expect("Bild");
    let want = export_pixels(h, &s);
    assert_eq!((img.width(), img.height()), (want.width(), want.height()), "{what}: Größe in Pixeln");
    assert!(img.as_bytes() == want.as_bytes(), "{what}: Bildpunkte wie beim Export");
    assert_eq!(res.bytes, real_export(h, &s), "{what}: Dateigröße wie beim Export");
}

// ---------------------------------------------------------------------- Tests

/// Registerkarte „Datei“: %, B/H mit „Proportionen beibehalten“ und Exportbereich X/Y/B/H wirken
/// auf Vorschau und Größenanzeige genau wie der Export; Werte in Ausgabepixeln (§3).
#[test]
fn scale_and_export_area_change_size_and_image_like_export() {
    let _g = lock();
    let mut h = harness(1);
    open(&mut h);
    // ein Status: Registerkarte Animation und Abspielknöpfe grau (§4, §6)
    assert!(!is_enabled(&mut h, &fp_i18n::t("dlg-preview-tab-animation")), "Animation grau bei einem Status");
    assert!(!is_enabled(&mut h, &fp_i18n::t("image-preview-play")), "Abspielen grau bei einem Status");
    dlg(&mut h).panes[0].settings = png32();
    wait_idle(&mut h);
    let full = result(&mut h, 0).bytes;
    click(&mut h, &fp_i18n::t("dlg-preview-tab-file"));
    assert_eq!(dlg(&mut h).tab, 1);
    assert_eq!(settings(&mut h, 0).crop, None, "Exportbereich ohne Haken");

    // % = 50: B/H sofort 120/80, Bild und Größe wie der Export
    type_into(&mut h, "image-preview-tip-scale-percent", "50");
    wait_idle(&mut h);
    let s = settings(&mut h, 0);
    assert_eq!(crop::scaled_size((240, 160), &s), (120, 80), "B/H folgen %");
    assert_eq!(crop::percent_shown((240, 160), &s), 50.0);
    assert!(painted_texts(&h).iter().any(|t| t == "120") && painted_texts(&h).iter().any(|t| t == "80"), "B/H zeigen 120/80");
    assert_like_export(&mut h, 0, "50 %");
    let half = result(&mut h, 0).bytes;
    assert!(half < full, "Größe fällt: {half} < {full}");
    // dasselbe noch einmal: keine Änderung (§4b)
    type_into(&mut h, "image-preview-tip-scale-percent", "50");
    assert_eq!(settings(&mut h, 0), s, "gleicher Wert ändert nichts");

    // B = 60 mit Proportionen: H = 40, % = 25
    type_into(&mut h, "image-preview-tip-scale-width", "60");
    wait_idle(&mut h);
    let s = settings(&mut h, 0);
    assert_eq!(crop::scaled_size((240, 160), &s), (60, 40));
    assert_eq!(crop::percent_shown((240, 160), &s), 25.0);
    assert_like_export(&mut h, 0, "B 60");

    // Exportbereich anhaken: ganzes Bild, Felder in Ausgabepixeln (0, 0, 60, 40)
    click(&mut h, &fp_i18n::t("image-preview-file-export-area"));
    let s = settings(&mut h, 0);
    assert_eq!(s.crop, Some([0, 0, 240, 160]));
    assert_eq!(crop::area_shown((240, 160), &s), [0, 0, 60, 40]);
    // B = 30, X = 10 (Ausgabepixel) = 120 bzw. 40 Dokumentpixel
    type_into(&mut h, "image-preview-tip-area-width", "30");
    type_into(&mut h, "image-preview-tip-area-x", "10");
    type_into(&mut h, "image-preview-tip-area-height", "20");
    wait_idle(&mut h);
    let s = settings(&mut h, 0);
    assert_eq!(s.crop, Some([40, 0, 160, 80]), "Exportbereich in Dokumentpixeln");
    assert_eq!(crop::area_shown((240, 160), &s), [10, 0, 30, 20]);
    assert_like_export(&mut h, 0, "Exportbereich mit Skalierung");
    let size_line = pane::size_text(result(&mut h, 0).bytes);
    assert!(painted_texts(&h).contains(&size_line), "Kopfzeile zeigt die neue Größe „{size_line}“");

    // Haken weg: wieder das ganze (skalierte) Bild
    click(&mut h, &fp_i18n::t("image-preview-file-export-area"));
    wait_idle(&mut h);
    assert_eq!(settings(&mut h, 0).crop, None);
    assert_like_export(&mut h, 0, "ohne Exportbereich");

    // Proportionen aus: H unabhängig von B
    click(&mut h, &fp_i18n::t("image-preview-file-constrain"));
    type_into(&mut h, "image-preview-tip-scale-height", "100");
    wait_idle(&mut h);
    let s = settings(&mut h, 0);
    assert_eq!(crop::scaled_size((240, 160), &s), (60, 100));
    assert_like_export(&mut h, 0, "ohne Proportionen");
}

/// Werkzeug „Exportbereich“: gestrichelter Rahmen mit Griffen im aktiven Feld; Ziehen an Griffen,
/// Kanten und im Inneren setzt den Bereich nur dieses Feldes; zurück beim Zeiger zeigt das Feld
/// nur den Bereich und die Größe wie der Export (§6).
#[test]
fn export_area_tool_drags_handles_in_active_pane_only() {
    let _g = lock();
    let mut h = harness(1);
    open(&mut h);
    dlg(&mut h).panes[0].settings = png32();
    click(&mut h, &fp_i18n::t("image-preview-view-2"));
    assert_eq!(dlg(&mut h).mode, ViewMode::Two);
    assert_eq!(dlg(&mut h).active, 1, "unteres Feld aktiv");
    click(&mut h, &fp_i18n::t("image-preview-tool-export-area"));
    assert_eq!(dlg(&mut h).tool, Tool::ExportArea);
    wait_idle(&mut h);
    let lay = dlg(&mut h).layout.iter().find(|(k, _)| *k == 1).map(|(_, l)| *l).expect("Feld 1");
    let sr = dlg(&mut h).export_area_rect(1, &lay).expect("Rahmen im aktiven Feld");
    assert_eq!(sr.size(), vec2(240.0, 160.0), "ohne Exportbereich: ganzes Bild");
    let lay0 = dlg(&mut h).layout.iter().find(|(k, _)| *k == 0).map(|(_, l)| *l).expect("Feld 0");
    assert!(dlg(&mut h).export_area_rect(0, &lay0).is_none(), "nur im aktiven Feld");

    // Griff rechts unten 60/40 nach innen
    drag(&mut h, sr.max - vec2(1.0, 1.0), sr.max - vec2(61.0, 41.0));
    assert_eq!(settings(&mut h, 1).crop, Some([0, 0, 180, 120]), "Griff rechts unten");
    // im Inneren verschieben
    let sr = dlg(&mut h).export_area_rect(1, &lay).expect("Rahmen");
    drag(&mut h, sr.center(), sr.center() + vec2(20.0, 10.0));
    assert_eq!(settings(&mut h, 1).crop, Some([20, 10, 200, 130]), "verschoben");
    // obere Kante 15 nach unten
    let sr = dlg(&mut h).export_area_rect(1, &lay).expect("Rahmen");
    drag(&mut h, pos(sr.center().x + 30.0, sr.top()), pos(sr.center().x + 30.0, sr.top() + 15.0));
    assert_eq!(settings(&mut h, 1).crop, Some([20, 25, 200, 130]), "obere Kante");
    // Griff links oben über den Bildrand hinaus: bleibt im Bild
    let sr = dlg(&mut h).export_area_rect(1, &lay).expect("Rahmen");
    drag(&mut h, sr.min, sr.min - vec2(100.0, 100.0));
    assert_eq!(settings(&mut h, 1).crop, Some([0, 0, 200, 130]), "begrenzt auf das Bild");
    assert_eq!(settings(&mut h, 0).crop, None, "das andere Feld bleibt unverändert");
    // während des Werkzeugs zeigt das Feld das ganze Bild
    assert_eq!(result(&mut h, 1).image.as_ref().map(|i| (i.width(), i.height())), Some((240, 160)));
    // Kästchen „Exportbereich“ der Registerkarte Datei ist angehakt
    click(&mut h, &fp_i18n::t("dlg-preview-tab-file"));
    let checked = h.query_all(By::new().label(fp_i18n::t("image-preview-file-export-area").as_str())).any(|n| n.accesskit_node().toggled() == Some(egui::accesskit::Toggled::True));
    assert!(checked, "Kästchen angehakt");
    // zurück zum Zeiger: nur der Bereich, Größe wie der Export
    click(&mut h, &fp_i18n::t("image-preview-tool-pointer"));
    wait_idle(&mut h);
    assert_eq!(result(&mut h, 1).image.as_ref().map(|i| (i.width(), i.height())), Some((200, 130)));
    assert_like_export(&mut h, 1, "gezogener Exportbereich");
    assert_like_export(&mut h, 0, "anderes Feld");
    assert_eq!(result(&mut h, 0).image.as_ref().map(|i| (i.width(), i.height())), Some((240, 160)));
}

/// Registerkarte „Animation“ und Abspielknöpfe: Status 1 beim Öffnen (wie der Export), Statuswahl
/// über Knöpfe, Statusfeld und Liste, Verzögerung, Löschmethode, Auge, Laufzeit, Wiederholung,
/// Abspielen mit den Verzögerungen.
#[test]
fn animation_delay_playback_and_frame_choice() {
    let _g = lock();
    let mut h = harness(3);
    assert_eq!(st(&mut h).session.active().map(|d| (d.frame_count(), d.frame)), Some((3, 1)), "Dokument steht auf Status 2");
    open(&mut h);
    // beim Öffnen Status 1 wie der Export (Abweichung aus BV-1 behoben)
    assert_eq!(dlg(&mut h).frame, 0);
    let s = settings(&mut h, 0);
    let res = result(&mut h, 0);
    if !s.format.is_animated() {
        assert_eq!(res.bytes, real_export(&mut h, &s), "Einzelbild: Größe = Export (Status 1)");
    }
    let f0 = res.image.clone().expect("Bild Status 1");
    // Grau-Regeln auf Status 1
    let (first, prev, next, last) = (fp_i18n::t("image-preview-first-frame"), fp_i18n::t("image-preview-prev-frame"), fp_i18n::t("image-preview-next-frame"), fp_i18n::t("image-preview-last-frame"));
    let dbg: Vec<String> = [&first, &prev, &next, &last].iter().flat_map(|l| h.query_all(By::new().label(l.as_str())).map(|n| format!("{l} {:?} dis={}", n.rect(), n.accesskit_node().is_disabled())).collect::<Vec<_>>()).collect();
    assert!(!is_enabled(&mut h, &first) && !is_enabled(&mut h, &prev) && is_enabled(&mut h, &next) && is_enabled(&mut h, &last), "{dbg:#?} frames {}", dlg(&mut h).frames);
    click(&mut h, &next);
    assert_eq!(dlg(&mut h).frame, 1, "nächster Status");
    wait_idle(&mut h);
    let f1 = result(&mut h, 0).image.clone().expect("Bild Status 2");
    assert!(f0.as_bytes() != f1.as_bytes(), "Vorschau zeigt den gewählten Status");
    click(&mut h, &last);
    assert_eq!(dlg(&mut h).frame, 2);
    assert!(!is_enabled(&mut h, &last) && !is_enabled(&mut h, &next) && is_enabled(&mut h, &first));
    click(&mut h, &first);
    assert_eq!(dlg(&mut h).frame, 0);
    // Statusfeld: Nummer tippen
    let fr = label_rect(&mut h, &fp_i18n::t("image-preview-current-frame"));
    click_at(&mut h, pos(fr.left() + 6.0, fr.center().y));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text("3".into()));
    h.key_press(egui::Key::Tab);
    h.run_steps(3);
    assert_eq!(dlg(&mut h).frame, 2, "Statusfeld");

    // Registerkarte Animation: Klick in die Liste wählt den Status
    click(&mut h, &fp_i18n::t("dlg-preview-tab-animation"));
    assert_eq!(dlg(&mut h).tab, 2);
    click(&mut h, "Status 2");
    assert_eq!(dlg(&mut h).frame, 1, "Liste wählt den Status");
    // Verzögerung des gewählten Status
    type_into(&mut h, "image-preview-tip-frame-delay", "50");
    let a = dlg(&mut h).active;
    let s = settings(&mut h, a);
    assert_eq!(s.anim.frames.get(1).map(|f| f.delay), Some(50));
    assert_eq!(fp_ui::dialogs::image_preview::animation_tab::frame_info(&s, 0).delay, 7, "andere Status unverändert");
    let rt = fp_i18n::tr!("image-preview-anim-runtime", seconds = "0.64");
    assert!(painted_texts(&h).contains(&rt), "Laufzeit 7 + 50 + 7 = {rt}");
    assert!(painted_texts(&h).iter().any(|t| t == "7 /100 Sek."), "Einheit nur in der ersten Zeile");
    // Löschmethode über das Popup
    click(&mut h, &fp_i18n::t("image-preview-tip-disposal"));
    click(&mut h, &fp_i18n::t("image-preview-disposal-background"));
    assert_eq!(settings(&mut h, a).anim.frames[1].disposal, 2);
    // Auge: Status 3 nicht exportieren → Laufzeit ohne ihn
    click(&mut h, &fp_i18n::tr!("image-preview-anim-export-frame", n = 3));
    assert!(!settings(&mut h, a).anim.frames[2].visible);
    assert!(painted_texts(&h).contains(&fp_i18n::tr!("image-preview-anim-runtime", seconds = "0.57")));
    // Einmal abspielen / Schleife
    click(&mut h, &fp_i18n::t("image-preview-anim-play-once"));
    assert_eq!(settings(&mut h, a).anim.loop_count, -1);
    click(&mut h, &fp_i18n::t("image-preview-anim-loop"));
    assert_eq!(settings(&mut h, a).anim.loop_count, 0, "Schleife endlos");
    // Kästchen
    click(&mut h, &fp_i18n::t("image-preview-anim-crop-each"));
    assert!(!settings(&mut h, a).anim.auto_crop);

    // Abspielen: Status 1 (7/100 s) → Status 2 (50/100 s) → Status 1 (Status 3 ausgeblendet)
    click(&mut h, &first);
    h.run_steps(1);
    click(&mut h, &fp_i18n::t("image-preview-play"));
    let mut seen = vec![dlg(&mut h).frame];
    let t0 = h.ctx.input(|i| i.time);
    let mut switch_at = Vec::new();
    for _ in 0..80 {
        h.run_steps(1);
        // BA-117: playback shows a state only once the worker rendered it (FB-63). The worker runs
        // on real threads; while playback waits for it, the UI clock is stopped, so the measured
        // delays are those of the playback logic, not of the machine load.
        let w0 = Instant::now();
        while dlg(&mut h).play_wait.is_some() {
            assert!(w0.elapsed() < Duration::from_secs(60), "Worker rendert den nächsten Status");
            std::thread::sleep(Duration::from_millis(5));
            step_frozen(&mut h);
        }
        let f = dlg(&mut h).frame;
        if Some(&f) != seen.last() {
            seen.push(f);
            switch_at.push(h.ctx.input(|i| i.time) - t0);
        }
    }
    assert!(dlg(&mut h).playing, "spielt");
    // RE spec 72 §3.2: Play shows the next included state at once (in the click's frame or the
    // next one)
    let (k, shown) = if seen.first() == Some(&0) {
        assert!(switch_at[0] < 0.1, "jumps at once: {switch_at:?}");
        (1, switch_at[0])
    } else {
        (0, 0.0)
    };
    assert!(seen.len() >= k + 3 && seen[k..k + 3] == [1, 0, 1], "Folge {seen:?}");
    assert!(!seen.contains(&2), "ausgeblendeter Status wird übersprungen");
    let dt = switch_at[k] - shown;
    assert!((0.45..0.6).contains(&dt), "Status 2 steht 50/100 s: {dt:.3}");
    click(&mut h, &fp_i18n::t("image-preview-play"));
    assert!(!dlg(&mut h).playing, "gestoppt");
    let f = dlg(&mut h).frame;
    h.run_steps(40);
    assert_eq!(dlg(&mut h).frame, f, "steht nach dem Stoppen");
}

/// Registerkarten Datei und Animation in Deutsch und Englisch, groß und 800 × 600: kein Überlauf,
/// kein Text ragt in die Vorschau, keine Rohschlüssel (AGENTEN_REGELN §4c).
#[test]
fn file_and_animation_tabs_fit_in_de_and_en() {
    let _g = lock();
    for lang in ["de", "en"] {
        for (w, hh) in [(1400.0, 900.0), (800.0, 600.0)] {
            fp_i18n::set_language(lang);
            let ctx = egui::Context::default();
            let mut app = FirepixelApp::with_session(&ctx, session(3), UiSettings { theme: ThemeKind::Normal, ..UiSettings::default() }, false);
            app.state.sync_render = true;
            app.state.os_clipboard = false;
            let mut h = Harness::builder().with_size(vec2(w, hh)).with_pixels_per_point(1.0).with_max_steps(8).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
            h.run_steps(3);
            open(&mut h);
            for tab in [1usize, 2] {
                dlg(&mut h).tab = tab;
                h.run_steps(2);
                let _ = fp_ui::dialogs::take_overflows(&h.ctx);
                h.run_steps(2);
                let over = fp_ui::dialogs::take_overflows(&h.ctx);
                assert!(over.is_empty(), "{lang} {w}×{hh} Registerkarte {tab}: Überlauf {over:?}");
                let left_r = dlg(&mut h).layout.iter().map(|(_, l)| l.frame.left()).fold(f32::MAX, f32::min);
                let rect = fp_ui::dialogs::dialog_rect(&h.ctx, fp_ui::dialogs::image_preview::DIALOG_ID).expect("Dialog");
                let all: Vec<(String, Rect)> = h.output().shapes.iter().flat_map(|cs| match &cs.shape {
                    Shape::Text(t) => vec![(t.galley.text().to_string(), t.visual_bounding_rect())],
                    _ => Vec::new(),
                }).collect();
                // nur was über dem Dialog gezeichnet wird (ab seinem Titel; dahinter liegt das Hauptfenster)
                let title = fp_i18n::t("dlg-preview-title");
                let start = all.iter().position(|(s, r)| *s == title && rect.contains(r.center())).unwrap_or(all.len());
                assert!(start < all.len(), "Titel gezeichnet");
                for s in all[start..].iter().cloned() {
                    assert!(!s.0.starts_with("image-preview-"), "{lang}: Rohschlüssel {}", s.0);
                    let r = s.1;
                    if r.is_positive() && rect.contains(r.center()) && r.center().x < left_r && r.center().y > rect.top() + 30.0 {
                        assert!(r.right() <= left_r - 4.0, "{lang} {w}×{hh} Registerkarte {tab}: „{}“ {r:?} ragt in die Vorschau", s.0);
                    }
                }
            }
        }
    }
}
