//! Bildvorschau, Paket P4 „Verhalten, Fenster, Texte“ (re\17-bildvorschau.md §6, §7, §8.2, §11.5,
//! BV-4): OK schreibt die Einstellung des aktiven Feldes in einem Verlaufsschritt in die Seite,
//! „Exportieren…“ übernimmt sie und öffnet den Exportdialog darüber (Abbruch dort → zurück in die
//! Bildvorschau), Esc/Abbrechen ändert nichts, Esc in Unterdialogen schließt nur diese, Esc beim
//! Ziehen des Exportbereichs bricht nur das Ziehen ab, Benutzervorgaben (Überschreiben, leerer Name,
//! Löschen im Optimieren-Bedienfeld), Mausrad wie im Dokumentfenster. Wie alle Testläufe mit
//! `FP_NO_SHELL_OPEN=1` starten (AGENTEN_REGELN §4a).

use egui::{vec2, Pos2, Rect};
use egui_kittest::kittest::{By, Queryable};
use egui_kittest::Harness;
use fp_export::options::{builtin_presets, ColorMode, ExportFormat, OptimizeSettings};
use fp_ui::app_state::{Action, AppState, UiSettings};
use fp_ui::dialogs::image_preview::{color_table, crop, optimize_to_size, pane, toolbar, ImagePreview, Tool};
use fp_ui::theme::ThemeKind;
use fp_ui::FirepixelApp;
use serde_json::json;
use std::sync::Mutex;
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

/// Bild 240 × 160 mit Verläufen auf weißer Leinwand; mit `frames` > 1 weitere Status.
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

fn open(h: &mut H) {
    st(h).push(Action::Run("file.imagePreview".into(), json!({})));
    h.run_steps(3);
    assert!(st(h).dialogs.preview.is_some(), "Dialog offen");
    wait_idle(h);
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

fn press(h: &mut H, p: Pos2, pressed: bool) {
    h.event(egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() });
    h.run_steps(1);
}

fn move_to(h: &mut H, p: Pos2) {
    h.event(egui::Event::PointerMoved(p));
    h.run_steps(1);
}

/// Bereich der Bildvorschau (Hauptmenü und Bedienfelder haben teils dieselben Namen).
fn dialog_area(h: &mut H) -> Rect {
    let d = dlg(h);
    let f = d.layout.iter().map(|(_, l)| l.frame).reduce(|a, b| a.union(b)).expect("Felder gezeichnet");
    Rect::from_min_max(f.min - vec2(480.0, 40.0), f.max + vec2(10.0, 90.0))
}

/// Element mit diesem Namen im Dialog anklicken (Barrierefreiheits-Aktion wie ein Mausklick).
fn click(h: &mut H, label: &str) {
    let area = dialog_area(h);
    let node = h.query_all(By::new().label(label)).find(|n| area.contains(n.rect().center())).unwrap_or_else(|| panic!("„{label}“ nicht im Dialog"));
    node.click();
    h.run_steps(3);
}

fn esc(h: &mut H) {
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
}

fn page_settings(h: &mut H) -> OptimizeSettings {
    let d = st(h).session.active().expect("doc");
    fp_export::doc_export::page_settings(&d.doc, d.page)
}

fn undo_count(h: &mut H) -> usize {
    st(h).session.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

fn image_center(h: &mut H, i: usize) -> Pos2 {
    dlg(h).layout.iter().find(|(k, _)| *k == i).map(|(_, l)| l.image.center()).expect("Feld gezeichnet")
}

fn wheel(h: &mut H, unit: egui::MouseWheelUnit, delta: egui::Vec2, modifiers: egui::Modifiers) {
    h.event(egui::Event::MouseWheel { unit, delta, phase: egui::TouchPhase::Move, modifiers });
    h.run_steps(1);
}

fn jpeg() -> OptimizeSettings {
    builtin_presets().iter().map(|p| p.settings.clone()).find(|s| s.format == ExportFormat::Jpeg).expect("JPEG-Vorgabe")
}

// ---------------------------------------------------------------------- Tests

/// OK (§7): Format, Palette, Skalierung, Exportbereich, Animation und Hintergrund des **aktiven**
/// Feldes werden die Exporteinstellung der Seite – genau ein Verlaufsschritt, Rückgängig stellt
/// den alten Stand her; OK ohne Änderung legt keinen Schritt an. Die Status-Eigenschaften gehen als
/// `frameInfo` der Exporteinstellung mit (dort liegen sie auch im Dokument, §4).
#[test]
fn ok_writes_the_active_pane_in_one_undo_step() {
    let _g = lock();
    let mut h = harness(3);
    let before = page_settings(&mut h);
    open(&mut h);
    click(&mut h, &fp_i18n::t("image-preview-view-2"));
    assert_eq!(dlg(&mut h).active, 1, "unteres Feld aktiv");
    let mut s = dlg(&mut h).panes[1].settings.clone();
    s.format = ExportFormat::Gif;
    s.color_mode = ColorMode::Indexed;
    s.max_colors = 64;
    s.lossy = 20;
    s.matte = Some(0xFFFF_0000);
    crop::set_percent(&mut s, (240, 160), 50.0);
    s.crop = Some([10, 20, 200, 140]);
    while s.anim.frames.len() < 3 {
        s.anim.frames.push(Default::default());
    }
    s.anim.frames[1].delay = 55;
    s.anim.frames[2].visible = false;
    s.anim.loop_count = 3;
    dlg(&mut h).panes[1].settings = s.clone();
    // das andere Feld hat etwas anderes: es darf nicht übernommen werden
    dlg(&mut h).panes[0].settings = jpeg();
    wait_idle(&mut h);
    let n0 = undo_count(&mut h);
    click(&mut h, &fp_i18n::t("btn-ok"));
    h.run_steps(2);
    assert!(st(&mut h).dialogs.preview.is_none(), "OK schließt");
    assert_eq!(undo_count(&mut h), n0 + 1, "genau ein Verlaufsschritt");
    let p = page_settings(&mut h);
    assert_eq!((p.format, p.max_colors, p.lossy, p.matte), (ExportFormat::Gif, 64, 20, Some(0xFFFF_0000)), "Format/Palette/Hintergrund des aktiven Feldes");
    assert_eq!(p.crop, Some([10, 20, 200, 140]), "Exportbereich");
    assert_eq!(crop::percent_shown((240, 160), &p), 50.0, "Skalierung");
    assert_eq!((p.anim.frames[1].delay, p.anim.frames[2].visible, p.anim.loop_count), (55, false, 3), "Animation");

    // Rückgängig: wieder der Stand vor OK
    st(&mut h).push(Action::Run("edit.undo".into(), json!({})));
    h.run_steps(3);
    let p = page_settings(&mut h);
    assert_eq!((p.format, p.lossy, p.crop, p.matte), (before.format, before.lossy, before.crop, before.matte), "Rückgängig stellt her");
    st(&mut h).push(Action::Run("edit.redo".into(), json!({})));
    h.run_steps(3);
    assert_eq!(page_settings(&mut h).lossy, 20, "Wiederholen");

    // erneut öffnen (nichts vom letzten Dialog bleibt, §7) und OK ohne Änderung: kein Schritt
    open(&mut h);
    assert_eq!(dlg(&mut h).mode, fp_ui::dialogs::image_preview::ViewMode::One, "1-fach beim Öffnen");
    let n1 = undo_count(&mut h);
    click(&mut h, &fp_i18n::t("btn-ok"));
    h.run_steps(2);
    assert!(st(&mut h).dialogs.preview.is_none());
    assert_eq!(undo_count(&mut h), n1, "OK ohne Änderung legt keinen Verlaufsschritt an");
}

/// Abbrechen und Esc schließen ohne Änderung; Esc in den Unterdialogen („Vorgabenname“, „Farbe“,
/// „Für Dateigröße optimieren“, Meldung) schließt nur diese.
#[test]
fn esc_and_cancel_change_nothing_and_sub_dialogs_close_alone() {
    let _g = lock();
    let mut h = harness(1);
    let before = page_settings(&mut h);
    let n0 = undo_count(&mut h);
    open(&mut h);
    dlg(&mut h).panes[0].settings = jpeg();
    h.run_steps(2);
    esc(&mut h);
    assert!(st(&mut h).dialogs.preview.is_none(), "Esc schließt");
    assert_eq!((undo_count(&mut h), page_settings(&mut h).format), (n0, before.format), "Esc ändert nichts");

    open(&mut h);
    dlg(&mut h).panes[0].settings = jpeg();
    h.run_steps(2);
    click(&mut h, &fp_i18n::t("btn-cancel"));
    assert!(st(&mut h).dialogs.preview.is_none(), "Abbrechen schließt");
    assert_eq!((undo_count(&mut h), page_settings(&mut h).format), (n0, before.format), "Abbrechen ändert nichts");

    // Unterdialoge: Esc schließt nur den obersten
    open(&mut h);
    type Setter = fn(&mut ImagePreview);
    type Check = fn(&ImagePreview) -> bool;
    let cases: [(&str, Setter, Check); 4] = [
        ("Vorgabenname", |d| d.preset_name = Some((0, "x".into())), |d| d.preset_name.is_some()),
        ("Farbe", |d| d.color_dialog = Some(color_table::ColorDialog::new(0, color_table::ColorMode::Add, 0xFF80_8080)), |d| d.color_dialog.is_some()),
        ("Dateigröße", |d| d.size_dialog = Some(optimize_to_size::SizeDialog { pane: 0, text: "5".into() }), |d| d.size_dialog.is_some()),
        ("Meldung", |d| d.message = Some("Test".into()), |d| d.message.is_some()),
    ];
    for (what, set, is_open) in cases {
        set(dlg(&mut h));
        h.run_steps(3);
        assert!(is_open(dlg(&mut h)), "{what} offen");
        esc(&mut h);
        assert!(st(&mut h).dialogs.preview.is_some(), "{what}: Bildvorschau bleibt offen");
        assert!(!is_open(dlg(&mut h)), "{what}: Esc schließt den Unterdialog");
    }
    esc(&mut h);
    assert!(st(&mut h).dialogs.preview.is_none(), "danach schließt Esc die Bildvorschau");
    assert_eq!(undo_count(&mut h), n0);
}

/// „Exportieren…“ (§7): übernimmt die Einstellung des aktiven Feldes (ein Verlaufsschritt) und
/// öffnet den Exportdialog über der Bildvorschau; Esc dort führt zurück in die Bildvorschau. Aus
/// dem Exportdialog („Optionen…“) geöffnet, geht es nach „Exportieren…“ dorthin zurück.
#[test]
fn export_button_applies_and_opens_the_export_dialog() {
    let _g = lock();
    let mut h = harness(1);
    open(&mut h);
    dlg(&mut h).panes[0].settings = jpeg();
    wait_idle(&mut h);
    let n0 = undo_count(&mut h);
    click(&mut h, &fp_i18n::t("dlg-export-button"));
    h.run_steps(3);
    assert!(st(&mut h).dialogs.export.is_some(), "Exportdialog offen");
    assert!(st(&mut h).dialogs.preview.as_ref().is_some_and(|d| d.export_child), "Bildvorschau bleibt darunter offen");
    assert_eq!(undo_count(&mut h), n0 + 1, "ein Verlaufsschritt");
    assert_eq!(page_settings(&mut h).format, ExportFormat::Jpeg, "Einstellung des aktiven Feldes in der Seite");
    assert_eq!(st(&mut h).dialogs.export.as_ref().map(|e| e.page_ext.clone()).as_deref(), Some("jpg"), "Exportdialog sieht das neue Format");
    assert_eq!(fp_ui::dialogs::last_dialog_id(&h.ctx).as_deref(), Some(fp_ui::dialogs::image_preview::DIALOG_ID), "beide gezeichnet");
    // Esc schließt nur den Exportdialog: zurück in die Bildvorschau
    esc(&mut h);
    assert!(st(&mut h).dialogs.export.is_none(), "Exportdialog zu");
    assert!(st(&mut h).dialogs.preview.as_ref().is_some_and(|d| !d.export_child), "zurück in der Bildvorschau");
    // die Bildvorschau ist wieder bedienbar: Abbrechen schließt sie
    click(&mut h, &fp_i18n::t("btn-cancel"));
    assert!(st(&mut h).dialogs.preview.is_none());

    // aus dem Exportdialog geöffnet: „Exportieren…“ führt in den Exportdialog zurück
    let ed = fp_ui::dialogs::export::ExportDialog::new(st(&mut h));
    st(&mut h).dialogs.export = ed;
    h.run_steps(3);
    open(&mut h);
    let mut png = OptimizeSettings { format: ExportFormat::Png, color_mode: ColorMode::Rgba32, ..OptimizeSettings::default() };
    png.matte = None;
    dlg(&mut h).panes[0].settings = png;
    wait_idle(&mut h);
    click(&mut h, &fp_i18n::t("dlg-export-button"));
    h.run_steps(3);
    assert!(st(&mut h).dialogs.preview.is_none(), "Bildvorschau zu");
    assert!(st(&mut h).dialogs.export.is_some(), "Exportdialog bleibt offen");
    assert_eq!(page_settings(&mut h).format, ExportFormat::Png);
    assert_eq!(st(&mut h).dialogs.export.as_ref().map(|e| e.page_ext.clone()).as_deref(), Some("png"), "Endung folgt dem Format");
}

/// Esc während des Ziehens mit dem Werkzeug „Exportbereich“ bricht nur das Ziehen ab: Bereich wie
/// vorher, Dialog bleibt offen; danach zieht das Werkzeug wieder normal.
#[test]
fn esc_cancels_an_export_area_drag_only() {
    let _g = lock();
    let mut h = harness(1);
    open(&mut h);
    click(&mut h, &fp_i18n::t("image-preview-tool-export-area"));
    assert_eq!(dlg(&mut h).tool, Tool::ExportArea);
    wait_idle(&mut h);
    let lay = dlg(&mut h).layout.iter().find(|(k, _)| *k == 0).map(|(_, l)| *l).expect("Feld 0");
    let sr = dlg(&mut h).export_area_rect(0, &lay).expect("Rahmen");
    let before = dlg(&mut h).panes[0].settings.crop;
    // Griff rechts unten ziehen, mitten im Ziehen Esc
    let from = sr.max - vec2(1.0, 1.0);
    move_to(&mut h, from);
    press(&mut h, from, true);
    for k in 1..=6 {
        move_to(&mut h, from - vec2(10.0, 7.0) * k as f32);
    }
    assert!(dlg(&mut h).panes[0].settings.crop.is_some(), "während des Ziehens geändert");
    esc(&mut h);
    assert!(st(&mut h).dialogs.preview.is_some(), "Esc schließt den Dialog nicht");
    assert_eq!(dlg(&mut h).panes[0].settings.crop, before, "Bereich wie vor dem Ziehen");
    // weiter bewegen und loslassen: nichts mehr
    move_to(&mut h, from - vec2(80.0, 60.0));
    press(&mut h, from - vec2(80.0, 60.0), false);
    h.run_steps(2);
    assert_eq!(dlg(&mut h).panes[0].settings.crop, before, "nach Esc wirkt das Ziehen nicht mehr");
    // ein neues Ziehen wirkt wieder
    move_to(&mut h, from);
    press(&mut h, from, true);
    for k in 1..=6 {
        move_to(&mut h, from - vec2(10.0, 10.0) * k as f32);
    }
    press(&mut h, from - vec2(60.0, 60.0), false);
    h.run_steps(2);
    assert_eq!(dlg(&mut h).panes[0].settings.crop, Some([0, 0, 180, 100]), "neues Ziehen");
    // ohne Ziehen schließt Esc den Dialog
    esc(&mut h);
    assert!(st(&mut h).dialogs.preview.is_none());
}

/// Mausrad über einem Feld (§11.5, wie im Dokumentfenster): Rad verschiebt alle Felder senkrecht,
/// Strg+Rad zoomt je volle Rastung genau eine Stufe um den Zeiger, kleine Touchpad-Schritte sammeln
/// sich, Umschalt+Rad und Rad über der linken Spalte tun nichts.
#[test]
fn mouse_wheel_scrolls_and_zooms_like_the_document_window() {
    let _g = lock();
    let mut h = harness(1);
    open(&mut h);
    dlg(&mut h).zoom_idx = toolbar::ZOOM_100 + 2; // 400 %: Bild größer als das Feld
    h.run_steps(3);
    let p = image_center(&mut h, 0);
    move_to(&mut h, p);
    let c0 = dlg(&mut h).center.expect("Mitte");
    // eine Rastung nach unten: 52 Bildschirmpunkte = 13 Bildpunkte bei 400 %
    wheel(&mut h, egui::MouseWheelUnit::Line, vec2(0.0, -1.0), egui::Modifiers::NONE);
    h.run_steps(1);
    let c1 = dlg(&mut h).center.expect("Mitte");
    assert!((c1.y - c0.y - 13.0).abs() < 0.01 && (c1.x - c0.x).abs() < 0.01, "Rad verschiebt senkrecht: {c0:?} → {c1:?}");
    assert_eq!(dlg(&mut h).zoom_idx, toolbar::ZOOM_100 + 2, "ohne Strg kein Zoom");
    // Umschalt+Rad: nichts
    wheel(&mut h, egui::MouseWheelUnit::Line, vec2(0.0, -1.0), egui::Modifiers::SHIFT);
    h.run_steps(1);
    assert_eq!(dlg(&mut h).center, Some(c1), "Umschalt+Rad tut nichts");
    // Strg+Rad: eine Rastung = eine Stufe, der Punkt unter dem Zeiger bleibt stehen
    wheel(&mut h, egui::MouseWheelUnit::Line, vec2(0.0, 1.0), egui::Modifiers::CTRL);
    h.run_steps(1);
    assert_eq!(dlg(&mut h).zoom_idx, toolbar::ZOOM_100 + 3, "eine Stufe größer");
    wheel(&mut h, egui::MouseWheelUnit::Line, vec2(0.0, -3.0), egui::Modifiers::CTRL);
    h.run_steps(1);
    assert_eq!(dlg(&mut h).zoom_idx, toolbar::ZOOM_100 + 2, "eine Rastung mit 3 Zeilen = eine Stufe");
    // Touchpad: 30 Punkte je Ereignis, erst die vierte (120 ≥ 100) zoomt
    for _ in 0..3 {
        wheel(&mut h, egui::MouseWheelUnit::Point, vec2(0.0, 30.0), egui::Modifiers::CTRL);
    }
    assert_eq!(dlg(&mut h).zoom_idx, toolbar::ZOOM_100 + 2, "Teilrastungen sammeln sich");
    wheel(&mut h, egui::MouseWheelUnit::Point, vec2(0.0, 30.0), egui::Modifiers::CTRL);
    assert_eq!(dlg(&mut h).zoom_idx, toolbar::ZOOM_100 + 3, "volle Rastung");
    // nicht über 6400 % hinaus
    for _ in 0..12 {
        wheel(&mut h, egui::MouseWheelUnit::Line, vec2(0.0, 1.0), egui::Modifiers::CTRL);
    }
    assert_eq!(dlg(&mut h).zoom_idx, toolbar::ZOOMS.len() - 1, "höchste Stufe");
    // über der linken Spalte: nichts
    let (z, c) = (dlg(&mut h).zoom_idx, dlg(&mut h).center);
    let left = dialog_area(&mut h).min + vec2(120.0, 200.0);
    move_to(&mut h, left);
    wheel(&mut h, egui::MouseWheelUnit::Line, vec2(0.0, -1.0), egui::Modifiers::CTRL);
    wheel(&mut h, egui::MouseWheelUnit::Line, vec2(0.0, -1.0), egui::Modifiers::NONE);
    h.run_steps(1);
    assert_eq!((dlg(&mut h).zoom_idx, dlg(&mut h).center), (z, c), "Rad außerhalb der Felder");
}

/// Benutzervorgaben (§8.2): gespeichert unter dem Namen (Leerraum abgeschnitten), derselbe Name
/// überschreibt (ein Eintrag), leerer Name lässt sich nicht speichern, die Vorgabe steht sofort in
/// den Vorgabenlisten; gelöscht wird im Optimieren-Bedienfeld, die offene Bildvorschau sieht das.
#[test]
fn user_presets_save_overwrite_and_delete() {
    let _g = lock();
    let mut h = harness(1);
    open(&mut h);
    let mut s = jpeg();
    s.jpeg.quality = 33;
    dlg(&mut h).panes[0].settings = s;
    h.run_steps(2);
    let tip = fp_i18n::t("image-preview-save-current-tip");
    let save = |h: &mut H, name: &str| {
        let f0 = dlg(h).layout.iter().find(|(k, _)| *k == 0).map(|(_, l)| l.frame).expect("Feld 0");
        let plus = h.query_all(By::new().label(tip.as_str())).map(|n| n.rect()).find(|r| f0.contains(r.center())).expect("„+“ in Feld 0");
        move_to(h, plus.center());
        press(h, plus.center(), true);
        press(h, plus.center(), false);
        h.run_steps(2);
        assert!(dlg(h).preset_name.is_some(), "Vorgabenname offen");
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
        h.key_press(egui::Key::Delete);
        if !name.is_empty() {
            h.event(egui::Event::Text(name.into()));
        }
        h.run_steps(2);
        h.key_press(egui::Key::Enter);
        h.run_steps(3);
    };
    // leerer Name: Enter speichert nicht, der Dialog bleibt
    save(&mut h, "   ");
    assert!(dlg(&mut h).preset_name.is_some(), "leerer Name: Dialog bleibt offen");
    esc(&mut h);
    assert!(dlg(&mut h).preset_name.is_none());
    assert!(fp_ui::ui_commands::export_presets(&st(&mut h).session.prefs).is_empty(), "nichts gespeichert");

    save(&mut h, "  Klein  ");
    let own = fp_ui::ui_commands::export_presets(&st(&mut h).session.prefs);
    assert_eq!(own.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(), ["Klein"], "Name ohne Leerraum");
    assert_eq!(OptimizeSettings::from_model(&own[0].1, None).jpeg.quality, 33);
    assert!(pane::preset_entries(&own).iter().any(|e| e.label == "Klein"), "in der Vorgabenliste");
    // derselbe Name überschreibt
    dlg(&mut h).panes[0].settings.jpeg.quality = 44;
    h.run_steps(2);
    save(&mut h, "Klein");
    let own = fp_ui::ui_commands::export_presets(&st(&mut h).session.prefs);
    assert_eq!(own.len(), 1, "überschrieben, kein zweiter Eintrag");
    assert_eq!(OptimizeSettings::from_model(&own[0].1, None).jpeg.quality, 44, "neue Werte");
    // Löschen im Optimieren-Bedienfeld (in der Bildvorschau gibt es keinen Knopf, §8.2)
    st(&mut h).session.execute("panel.optimize.deleteSettings", json!({"name": "Klein"})).expect("löschen");
    h.run_steps(3);
    let own = fp_ui::ui_commands::export_presets(&st(&mut h).session.prefs);
    assert!(own.is_empty() && !pane::preset_entries(&own).iter().any(|e| e.label == "Klein"), "aus allen Listen entfernt");
    assert!(st(&mut h).dialogs.preview.is_some(), "Bildvorschau bleibt offen");
}

/// RAND-1: „Browseranzeige mit Zeilensprungverfahren“ ist so breit wie „Nicht verwendete Farben
/// entfernen“ (vorher 30 px schmaler).
#[test]
fn interlaced_checkbox_has_full_width() {
    let _g = lock();
    let mut h = harness(1);
    open(&mut h);
    let mut s = OptimizeSettings { format: ExportFormat::Gif, color_mode: ColorMode::Indexed, ..OptimizeSettings::default() };
    s.max_colors = 128;
    dlg(&mut h).panes[0].settings = s;
    wait_idle(&mut h);
    let area = dialog_area(&mut h);
    let rect = |h: &mut H, key: &str| h.query_all(By::new().label(fp_i18n::t(key).as_str())).map(|n| n.rect()).find(|r| area.contains(r.center())).unwrap_or_else(|| panic!("{key}"));
    let a = rect(&mut h, "image-preview-opt-interlaced");
    let b = rect(&mut h, "image-preview-opt-remove-unused");
    assert!((a.width() - b.width()).abs() < 1.0, "gleich breit: {} / {}", a.width(), b.width());
}
