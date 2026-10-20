//! FV-5: Warnung bei immer aktiven Menübefehlen ohne passendes Ziel (BEFUNDE EB Z01–Z18) im
//! Meldungsdialog – Wortlaut, Kästchen „Nicht mehr anzeigen“ (Mausklick bzw. Zugriffstaste h),
//! OK/Eingabe/Esc, kein Verlaufsschritt, kein Befehlsdialog; „Warnmeldungen zurücksetzen“ schaltet
//! sie wieder ein; Rückfrage „Popupmenü hinzufügen“ (Hotspot/Segment/Abbrechen); Menü
//! „Wiederholen: <letzter Schritt>“.

use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn harness() -> H {
    fp_engine::shell::disable();
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [20, 20, 120, 80]})).expect("rect");
    s.execute("select.none", json!({})).expect("none");
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(1600.0, 1000.0)).with_pixels_per_point(1.0).with_max_steps(12).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(3);
    h
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn ev(h: &mut H, e: Event) {
    if let Event::Key { modifiers, .. } | Event::PointerButton { modifiers, .. } = &e {
        h.event(Event::ModifiersChanged(*modifiers));
    }
    h.event(e);
    h.run_steps(1);
}

fn key(h: &mut H, k: Key) {
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: Modifiers::NONE });
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: false, repeat: false, modifiers: Modifiers::NONE });
}

fn click(h: &mut H, p: Pos2) {
    ev(h, Event::PointerMoved(p));
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
}

fn texts(shape: &egui::epaint::Shape, out: &mut Vec<(String, egui::Rect)>) {
    match shape {
        egui::epaint::Shape::Text(t) => out.push((t.galley.job.text.clone(), egui::Rect::from_min_size(t.pos, t.galley.size()))),
        egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| texts(s, out)),
        _ => {}
    }
}

fn all_texts(h: &H) -> Vec<(String, egui::Rect)> {
    let mut all = Vec::new();
    for cs in &h.output().shapes {
        texts(&cs.shape, &mut all);
    }
    all
}

fn find(h: &H, s: &str) -> Option<egui::Rect> {
    all_texts(h).into_iter().find(|(t, _)| t == s).map(|(_, r)| r)
}

fn run(h: &mut H, name: &str) {
    h.state_mut().state.run(name, json!({}));
    h.run_steps(3);
}

fn history_len(h: &H) -> usize {
    st(h).session.active().map(|d| d.history.len()).unwrap_or(0)
}

const Z01: &str = "Zum Öffnen, Vereinigen, Überschneiden und Zuschneiden müssen zwei oder mehr Pfade ausgewählt werden.";

#[test]
fn warning_with_do_not_show_again_and_reset() {
    fp_i18n::with_language("de", || {
        let mut h = harness();
        let steps = history_len(&h);
        run(&mut h, "modify.combinePaths.union");
        let w = st(&h).dialogs.warning.clone().expect("Warnung offen");
        assert_eq!(w.warning.key, "combinePaths");
        assert!(!w.dont_show);
        // Aussehen wie im Original: Titel Programmname, Text, Kästchen, OK
        assert!(find(&h, "Firepixel").is_some(), "Titel");
        assert!(all_texts(&h).iter().any(|(t, _)| t == Z01), "Wortlaut");
        let check = find(&h, "Nicht mehr anzeigen").expect("Kästchen");
        let ok = find(&h, "OK").expect("OK");
        assert!(check.center().x < ok.center().x && (check.center().y - ok.center().y).abs() < 6.0, "Kästchen links neben OK");
        assert_eq!(history_len(&h), steps, "kein Verlaufsschritt");

        // Kästchen per Mausklick an, per Zugriffstaste h wieder aus und erneut an
        click(&mut h, egui::pos2(check.left() - 11.0, check.center().y));
        assert!(st(&h).dialogs.warning.as_ref().is_some_and(|w| w.dont_show), "Klick hakt an");
        key(&mut h, Key::H);
        assert!(st(&h).dialogs.warning.as_ref().is_some_and(|w| !w.dont_show), "h hakt ab");
        key(&mut h, Key::H);
        assert!(st(&h).dialogs.warning.as_ref().is_some_and(|w| w.dont_show));
        // Eingabe = OK
        key(&mut h, Key::Enter);
        assert!(st(&h).dialogs.warning.is_none(), "geschlossen");
        assert!(st(&h).session.prefs.get_bool("DisableNoticeCombinePaths", false), "in den Voreinstellungen");
        assert_eq!(history_len(&h), steps);

        // zweites Mal (auch anderer Befehl derselben Warnung): nichts
        run(&mut h, "modify.combinePaths.union");
        assert!(st(&h).dialogs.warning.is_none(), "abgeschaltet");
        run(&mut h, "modify.combinePaths.crop");
        assert!(st(&h).dialogs.warning.is_none());
        assert_eq!(history_len(&h), steps);

        // Befehle ▸ Warnmeldungen zurücksetzen
        run(&mut h, "ui.resetWarnings");
        run(&mut h, "modify.combinePaths.union");
        assert!(st(&h).dialogs.warning.is_some(), "nach Zurücksetzen wieder da");
        // Esc schließt ohne Abschalten
        key(&mut h, Key::Escape);
        assert!(st(&h).dialogs.warning.is_none());
        assert!(!st(&h).session.prefs.get_bool("DisableNoticeCombinePaths", false));
        run(&mut h, "modify.combinePaths.union");
        assert!(st(&h).dialogs.warning.is_some());
        // Kästchen anhaken, OK klicken → abgeschaltet
        let check = find(&h, "Nicht mehr anzeigen").expect("Kästchen");
        click(&mut h, egui::pos2(check.left() - 11.0, check.center().y));
        let b = find(&h, "OK").expect("OK").center();
        click(&mut h, b);
        assert!(st(&h).dialogs.warning.is_none(), "OK schließt");
        assert!(st(&h).session.prefs.get_bool("DisableNoticeCombinePaths", false));
        // Befehle ▸ Warnmeldungen zurücksetzen (mitgeliefertes Befehlsskript `fw.resetWarnings()`)
        h.state_mut().state.run("commands.runScript", json!({"code": "fw.resetWarnings();
"}));
        h.run_steps(3);
        assert!(!st(&h).session.prefs.get_bool("DisableNoticeCombinePaths", false), "Skript setzt zurück");
        run(&mut h, "modify.combinePaths.union");
        assert!(st(&h).dialogs.warning.is_some());
        key(&mut h, Key::Enter);
    });
}

#[test]
fn warning_instead_of_command_dialog() {
    fp_i18n::with_language("de", || {
        let mut h = harness();
        // Vereinfachen…/Weiche Kante… haben einen Dialog – ohne Ziel kommt zuerst die Warnung
        for (cmd, key_) in [("modify.alterPath.simplify", "alterPath"), ("select.feather", "feather"), ("modify.symbol.tweenInstances", "tweenInstances")] {
            run(&mut h, cmd);
            assert_eq!(st(&h).dialogs.warning.as_ref().map(|w| w.warning.key.as_str()), Some(key_), "{cmd}");
            assert!(st(&h).dialogs.form.is_none(), "{cmd}: kein Formular");
            key(&mut h, Key::Enter);
            assert!(st(&h).dialogs.warning.is_none());
        }
        // mit Pfad: Vereinfachen öffnet den Dialog, keine Warnung
        let id = st(&h).session.active().and_then(|d| fp_engine::docops::selectable_ids(&d.doc, d.page, d.frame).first().copied()).expect("Rechteck");
        h.state_mut().state.session.execute("select.set", json!({"ids": [id.0]})).expect("select");
        run(&mut h, "modify.alterPath.simplify");
        assert!(st(&h).dialogs.warning.is_none());
    });
}

#[test]
fn english_without_overflow() {
    fp_i18n::with_language("en", || {
        let mut h = harness();
        run(&mut h, "modify.symbol.breakApart");
        h.run_steps(2);
        let text = "To break the link between a library symbol and its instances, first select a symbol or an instance.";
        let r = all_texts(&h).into_iter().find(|(t, _)| t == text).map(|(_, r)| r).expect("englischer Text");
        let check = find(&h, "Don't show again").expect("Kästchen englisch");
        let ok = find(&h, "OK").expect("OK");
        assert!(check.right() < ok.left(), "Kästchen überlappt OK nicht");
        assert!(r.bottom() < check.top(), "Text über der Knopfleiste");
        key(&mut h, Key::Enter);
    });
}

#[test]
fn popup_menu_confirm_hotspot() {
    fp_i18n::with_language("de", || {
        let mut h = harness();
        run(&mut h, "modify.popUpMenu.addPopUpMenu");
        let w = st(&h).dialogs.warning.clone().expect("Rückfrage");
        assert_eq!(w.warning.key, "addPopUpMenu");
        assert!(find(&h, "Nicht mehr anzeigen").is_none(), "Rückfrage ohne Kästchen");
        for b in ["Hotspot", "Segment", "Abbrechen"] {
            assert!(find(&h, b).is_some(), "Knopf {b}");
        }
        // Abbrechen (Zugriffstaste a): keine Wirkung, wie im Original folgt die zweite Meldung
        // „Wählen Sie ein Segment oder einen Hotspot aus …“ (PM-6 D34, DLG-9); OK schließt sie
        let steps = history_len(&h);
        key(&mut h, Key::A);
        assert_eq!(st(&h).dialogs.warning.as_ref().map(|w| w.warning.key.as_str()), Some("insertBehavior"));
        key(&mut h, Key::Enter);
        assert!(st(&h).dialogs.warning.is_none());
        assert_eq!(history_len(&h), steps);
        assert!(st(&h).dialogs.popup_editor.is_none());

        // ohne Auswahl → Hotspot braucht ein Objekt: dessen Warnung
        run(&mut h, "modify.popUpMenu.addPopUpMenu");
        let b = find(&h, "Hotspot").expect("Hotspot").center();
        click(&mut h, b);
        h.run_steps(3);
        assert_eq!(st(&h).dialogs.warning.as_ref().map(|w| w.warning.key.as_str()), Some("insertHotspot"));
        key(&mut h, Key::Enter);

        // mit Objekt: Hotspot entsteht, danach öffnet der Pop-up-Menü-Editor
        let id = st(&h).session.active().and_then(|d| fp_engine::docops::selectable_ids(&d.doc, d.page, d.frame).first().copied()).expect("Rechteck");
        h.state_mut().state.session.execute("select.set", json!({"ids": [id.0]})).expect("select");
        run(&mut h, "modify.popUpMenu.addPopUpMenu");
        assert!(st(&h).dialogs.warning.is_some());
        key(&mut h, Key::Enter); // Standardknopf „Hotspot“
        h.run_steps(4);
        assert!(st(&h).dialogs.warning.is_none());
        assert_eq!(history_len(&h), steps + 1, "Hotspot eingefügt");
        assert!(st(&h).dialogs.popup_editor.is_some(), "Pop-up-Menü-Editor offen");
    });
}

/// Redo is redo only (FB-08, re 55 §2): without an undone step the entry reads
/// „Wiederholen nicht möglich“ and is disabled; after an undo „Wiederholen: <Schritt>“.
#[test]
fn redo_with_last_step_in_menu() {
    fp_i18n::with_language("de", || {
        let mut h = harness();
        fn labels(nodes: &[fp_ui::menu::Node], out: &mut Vec<(String, bool)>) {
            for n in nodes {
                match n {
                    fp_ui::menu::Node::Item { label, enabled, .. } => out.push((label.clone(), *enabled)),
                    fp_ui::menu::Node::Sub { children, .. } => labels(children, out),
                    fp_ui::menu::Node::Separator => {}
                }
            }
        }
        let redo = |h: &H| {
            let mut all = Vec::new();
            for (_, nodes) in &fp_ui::menu::build_main_menu(st(h)) {
                labels(nodes, &mut all);
            }
            all.into_iter().find(|(l, _)| l.replace('&', "").starts_with("Wiederholen")).expect("Wiederholen im Menü")
        };
        let r = redo(&h);
        assert_eq!(r.0.replace('&', ""), "Wiederholen nicht möglich", "wie im Original");
        assert!(!r.1, "inaktiv");
        h.state_mut().state.session.execute("edit.undo", json!({})).expect("undo");
        let r = redo(&h);
        assert_eq!(r.0.replace('&', ""), "Wiederholen: Auswahl aufheben", "wie im Original");
        assert!(r.1, "aktiv");
    });
}
