//! CF-A: Befunde des Klicktests CT-A (Menüs, Dialoge, Kürzel; `docs\funktionstest\ct_a_2026-10-15.md`).
//! - CT-A-06 Zugriffstasten (Alt+Buchstabe) schreiben ihren Buchstaben nicht ins Feld
//! - CT-A-05 Klick auf einen anderen Menütitel bei offenem Menü öffnet ihn
//! - CT-A-03/04 Befehlsskripte ohne Dokument bzw. ohne passende Auswahl: verständliche Meldung
//! - CT-A-07/08/09/14 Tab-Folge und Startfokus (D41, D31, D43, D44, D17, D08)
//! - CT-A-01/02 Rückgängig/Wiederholen ohne „&“, Schrittname in der Sprache der Oberfläche
//! - CT-A-13/15/16/17 Freigaben und Meldungstexte
//!
//! Ereignisse wie egui-winit unter Windows: Taste + Text je Zeichen (auch bei Alt), Mausklicks über
//! mehrere Bilder.

use egui::accesskit::Role;
use egui::output::OutputEvent;
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Rect};
use egui_kittest::kittest::{By, NodeT, Queryable};
use egui_kittest::Harness;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::menu::Node;
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn harness_with(s: fp_engine::Session) -> H {
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).with_pixels_per_point(1.0).with_max_steps(12).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(3);
    h
}

/// Dokument 300 × 200 mit dem Rechteck R1, ausgewählt.
fn rect_session() -> fp_engine::Session {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("doc");
    let id = s.execute("draw.rectangle", json!({"rect": [20, 20, 120, 100], "name": "R1"})).expect("rect").value["id"].as_u64().expect("id");
    s.execute("select.set", json!({"ids": [id]})).expect("Auswahl");
    s
}

/// Dokument mit der Bitmap B1, ausgewählt (Filterdialoge).
fn bitmap_session() -> fp_engine::Session {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [40, 40, 140, 120], "fillColor": "#3366cc", "name": "B1"})).expect("rect");
    s.execute("modify.flattenSelection", json!({})).expect("reduzieren");
    s
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn ev(h: &mut H, e: Event) {
    h.event(e);
    h.run_steps(1);
}

fn key_m(h: &mut H, k: Key, m: Modifiers) {
    if m.any() {
        h.event(Event::ModifiersChanged(m));
    }
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: m });
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: false, repeat: false, modifiers: m });
    if m.any() {
        ev(h, Event::ModifiersChanged(Modifiers::NONE));
    }
    h.run_steps(2);
}

fn key(h: &mut H, k: Key) {
    key_m(h, k, Modifiers::NONE);
}

fn alt() -> Modifiers {
    Modifiers { alt: true, ..Default::default() }
}

/// Alt+Buchstabe wie egui-winit unter Windows (WM_SYSKEYDOWN + WM_SYSCHAR): Taste **und** Text.
fn alt_letter(h: &mut H, c: char) {
    let k = Key::from_name(&c.to_ascii_uppercase().to_string()).expect("Taste");
    h.event(Event::ModifiersChanged(alt()));
    h.event(Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: alt() });
    ev(h, Event::Text(c.to_string()));
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: false, repeat: false, modifiers: alt() });
    ev(h, Event::ModifiersChanged(Modifiers::NONE));
    h.run_steps(2);
}

fn typ(h: &mut H, s: &str) {
    for c in s.chars() {
        let k = Key::from_name(&c.to_string());
        if let Some(k) = k {
            h.event(Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: Modifiers::NONE });
        }
        ev(h, Event::Text(c.to_string()));
        if let Some(k) = k {
            ev(h, Event::Key { key: k, physical_key: Some(k), pressed: false, repeat: false, modifiers: Modifiers::NONE });
        }
    }
}

fn click(h: &mut H, p: Pos2) {
    ev(h, Event::PointerMoved(p));
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
}

fn open(h: &mut H, cmd: &str) {
    h.state_mut().state.run(cmd, json!({}));
    h.run_steps(4);
    assert!(st(h).dialogs.any_open(), "{cmd}: Dialog offen");
}

/// Rolle und Name (bzw. Wert) des fokussierten Bedienelements.
fn focused(h: &H) -> Option<(Role, String)> {
    h.ctx.memory(|m| m.focused())?;
    h.get_all(By::new()).find(|n| n.accesskit_node().is_focused()).map(|n| {
        let node = n.accesskit_node();
        (node.role(), node.label().map(|s| s.to_string()).or_else(|| node.value().map(|s| s.to_string())).unwrap_or_default())
    })
}

/// Tab drücken, Bild für Bild die gemeldeten Fokuswechsel (`focus_gained` wie die Brücke) sammeln.
fn tab_focus_events(h: &mut H, n: usize) -> Vec<String> {
    let mut out = Vec::new();
    for _ in 0..n {
        h.event(Event::Key { key: Key::Tab, physical_key: Some(Key::Tab), pressed: true, repeat: false, modifiers: Modifiers::NONE });
        for i in 0..4 {
            if i == 1 {
                h.event(Event::Key { key: Key::Tab, physical_key: Some(Key::Tab), pressed: false, repeat: false, modifiers: Modifiers::NONE });
            }
            h.step();
            for e in &h.output().platform_output.events {
                if let OutputEvent::FocusGained(info) = e {
                    out.push(format!("{:?}:{}", info.typ, info.label.clone().unwrap_or_default()));
                }
            }
        }
    }
    out
}

fn ok_l() -> String {
    fp_i18n::t("btn-ok")
}

// ---------------------------------------------------------------------- CT-A-06

/// Text nach Alt+Buchstabe wird verworfen, AltGr (Strg+Alt) und andere Zeichen bleiben.
#[test]
fn ct_a_06_filter_drops_alt_letter_text_only() {
    let k = |key: Key, m: Modifiers| Event::Key { key, physical_key: Some(key), pressed: true, repeat: false, modifiers: m };
    let altgr = Modifiers { alt: true, ctrl: true, ..Default::default() };
    let mut ev = vec![k(Key::H, alt()), Event::Text("h".into()), k(Key::Q, altgr), Event::Text("@".into()), k(Key::A, Modifiers::NONE), Event::Text("a".into()), k(Key::O, alt()), Event::Text("ø".into())];
    fp_ui::input::filter_alt_text(&mut ev);
    let texts: Vec<&str> = ev.iter().filter_map(|e| if let Event::Text(t) = e { Some(t.as_str()) } else { None }).collect();
    assert_eq!(texts, ["@", "a", "ø"], "nur der Buchstabe der Zugriffstaste fällt weg");
}

/// D01 Neues Dokument mit echter Tastatur: Alt+H wählt „Höhe“, schreibt aber kein „h“ in „Breite“;
/// Alt+T wählt „Transparent“ ohne „t“ im Feld.
#[test]
fn ct_a_06_access_keys_write_no_letter() {
    let mut h = harness_with(rect_session());
    open(&mut h, "file.new");
    let w0 = st(&h).dialogs.new_document.as_ref().expect("offen").width.clone();
    alt_letter(&mut h, 'h');
    typ(&mut h, "77");
    alt_letter(&mut h, 't');
    let d = st(&h).dialogs.new_document.as_ref().expect("offen");
    assert_eq!((d.width.as_str(), d.height.as_str()), (w0.as_str(), "77"), "Breite unverändert, Höhe 77 ohne „t“");
    assert_eq!(format!("{:?}", d.canvas), "Transparent", "Alt+T wirkt");
    key(&mut h, Key::Escape);
}

// ---------------------------------------------------------------------- CT-A-05

fn titles(h: &H) -> Vec<Rect> {
    h.ctx.data(|d| d.get_temp::<Vec<Rect>>(egui::Id::new(fp_ui::menu::MENU_TITLES_ID))).expect("Menütitel")
}

fn menu_open(h: &H, i: usize) -> bool {
    egui::Popup::is_id_open(&h.ctx, egui::Id::new(("fp-main-menu", i)))
}

/// Datei offen → Klick auf „Bearbeiten“ öffnet Bearbeiten; zweiter Klick schließt; nach Esc öffnet der
/// nächste Klick auf einen anderen Titel zuverlässig.
#[test]
fn ct_a_05_click_other_title_switches_menu() {
    let mut h = harness_with(rect_session());
    let t = titles(&h);
    click(&mut h, t[0].center());
    assert!(menu_open(&h, 0), "Datei offen");
    // Zeiger wandert langsam hinüber (Bearbeiten öffnet beim Überfahren), dann Klick
    for k in 1..=4 {
        ev(&mut h, Event::PointerMoved(t[0].center().lerp(t[1].center(), k as f32 / 4.0)));
    }
    click(&mut h, t[1].center());
    assert!(menu_open(&h, 1) && !menu_open(&h, 0), "Klick auf Bearbeiten lässt Bearbeiten offen");
    click(&mut h, t[1].center());
    assert!(!menu_open(&h, 1), "zweiter Klick auf denselben Titel schließt");
    // Klick ohne Zwischenbild (Zeiger springt): Datei öffnen, dann sofort Ansicht
    click(&mut h, t[0].center());
    assert!(menu_open(&h, 0));
    click(&mut h, t[2].center());
    assert!(menu_open(&h, 2) && !menu_open(&h, 0), "Ansicht offen");
    key(&mut h, Key::Escape);
    assert!(!menu_open(&h, 2), "Esc schließt");
    click(&mut h, t[3].center());
    assert!(menu_open(&h, 3), "nach Esc öffnet der nächste Titelklick");
    key(&mut h, Key::Escape);
}

// ---------------------------------------------------------------------- CT-A-03/04

fn script_file(name: &str, src: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("fp-cfa-{}-{:?}", std::process::id(), std::thread::current().id()).replace(['(', ')'], ""));
    std::fs::create_dir_all(&dir).expect("Ordner");
    let p = dir.join(format!("{name}.jsf"));
    std::fs::write(&p, src).expect("Skript");
    p
}

fn run_script(h: &mut H, p: &std::path::Path) {
    h.state_mut().state.dialogs.message = None;
    h.state_mut().state.status = None;
    h.state_mut().state.run("commands.runScript", json!({ "path": p.to_string_lossy() }));
    h.run_steps(3);
}

fn message(h: &H) -> Option<String> {
    st(h).dialogs.message.as_ref().map(|(_, m)| m.clone())
}

fn status_text(h: &H) -> String {
    st(h).status.as_ref().map(|s| format!("{s:?}")).unwrap_or_default()
}

/// Ohne Dokument (re 29 §2, T-3): „Skript konnte nicht ausgeführt werden.“ mit dem Grund in der
/// zweiten Zeile statt „Skript: TypeError …“; Skripte ohne Dokumentbezug bleiben still.
#[test]
fn ct_a_03_scripts_without_document() {
    let mut h = harness_with(fp_engine::Session::default());
    assert!(st(&h).session.active().is_none());
    let head = fp_i18n::t("ui-script-failed");
    let failing = script_file("Früheres Vektorrendern verwenden", "fw.getDocumentDOM().setUseAGMRenderingForSelection(false);
");
    run_script(&mut h, &failing);
    let m = message(&h).expect("Meldung");
    assert!(m.starts_with(&head) && m.lines().count() == 2, "Fehler ohne Dokument: {m}");
    assert!(!status_text(&h).contains("TypeError"), "kein Rohfehler: {}", status_text(&h));
    let silent = script_file("Als Großbuchstaben", "var sel = fw.selection;
for (var i = 0; i < sel.length; i++) {}
");
    run_script(&mut h, &silent);
    let want = format!("{head}
{}", fp_i18n::t("ui-script-error"));
    assert_eq!(message(&h).as_deref(), Some(want.as_str()), "wirkungslos ohne Dokument");
    let reset = script_file("Warnmeldungen zurücksetzen", "fw.resetWarnings();
");
    run_script(&mut h, &reset);
    assert_eq!(message(&h), None, "ohne Dokumentbezug keine Meldung");
}

/// Mit Dokument, aber unpassender Auswahl: im Original weder Meldung noch Statuszeilentext (re 29 §2;
/// die CF-A-Meldung ist zurückgenommen).
#[test]
fn ct_a_04_script_with_unsuitable_selection() {
    let mut h = harness_with(rect_session());
    let p = script_file("Als Pfade wiedergeben", "if (fw.selection.length) fw.getDocumentDOM().convertToPaths();
");
    run_script(&mut h, &p);
    assert_eq!(message(&h), None);
    assert_eq!(status_text(&h), "");
}

// ---------------------------------------------------------------------- CT-A-07/08/09/14

/// D41 Farbton/Sättigung: Tab erreicht nie einen Schieber, auch nicht für ein Bild (die Brücke meldet
/// jeden Fokuswechsel).
#[test]
fn ct_a_07_d41_sliders_never_get_focus() {
    let mut h = harness_with(bitmap_session());
    open(&mut h, "filter.adjustColor.hueSaturation");
    let seq = tab_focus_events(&mut h, 8);
    assert!(!seq.iter().any(|s| s.starts_with("Slider")), "{seq:?}");
    assert!(seq.iter().any(|s| s.starts_with("Button")), "{seq:?}");
    key(&mut h, Key::Escape);
}

/// D31 Pfad einlassen: Startfokus im Feld „Breite“ (markiert: Tippen ersetzt).
#[test]
fn ct_a_08_d31_starts_in_width() {
    let mut h = harness_with(rect_session());
    open(&mut h, "modify.alterPath.insetPath");
    let f = focused(&h).expect("Fokus");
    assert_eq!(f.0, Role::TextInput, "{f:?}");
    typ(&mut h, "5");
    key(&mut h, Key::Tab);
    assert_eq!(st(&h).dialogs.form.as_ref().expect("Formular").get("distance").as_f64(), Some(5.0));
    key(&mut h, Key::Escape);
}

/// D43 Kurven, D44 Tonwertkorrektur, D17 Tastaturkurzbefehle: Startfokus OK.
#[test]
fn ct_a_09_start_focus_ok() {
    let mut h = harness_with(bitmap_session());
    for cmd in ["filter.adjustColor.curves", "filter.adjustColor.levels", "edit.keyboardShortcuts"] {
        open(&mut h, cmd);
        h.run_steps(2);
        assert_eq!(focused(&h), Some((Role::Button, ok_l())), "{cmd}: Startfokus OK");
        assert!(!h.ctx.text_edit_focused(), "{cmd}");
        key(&mut h, Key::Escape);
        h.run_steps(3);
        assert!(!st(&h).dialogs.any_open(), "{cmd}: Esc schließt");
    }
}

/// D08 Bildvorschau: Tab läuft nur über Bedienelemente mit Namen (keine unsichtbaren Stopps auf
/// Ansichten, Reitern, Werkzeugknöpfen) und kommt wieder bei OK an.
#[test]
fn ct_a_14_d08_tab_cycle_stays_visible() {
    let mut h = harness_with(rect_session());
    open(&mut h, "file.imagePreview");
    h.run_steps(4);
    let mut seen = Vec::new();
    for _ in 0..40 {
        key(&mut h, Key::Tab);
        let f = focused(&h);
        assert!(f.is_some(), "Fokus verschwindet nach {seen:?}");
        seen.push(f.expect("Fokus"));
    }
    assert!(seen.iter().any(|(r, l)| *r == Role::Button && *l == ok_l()), "Tab-Kreis kommt zu OK zurück: {seen:?}");
    key(&mut h, Key::Escape);
}

// ---------------------------------------------------------------------- CT-A-01/02

fn edit_items(h: &H) -> Vec<String> {
    let menus = fp_ui::menu::build_main_menu(st(h));
    let (_, nodes) = &menus[1];
    nodes.iter().filter_map(|n| if let Node::Item { label, .. } = n { Some(label.clone()) } else { None }).collect()
}

/// Ohne Verlauf „Rückgängig nicht möglich“/„Wiederholen nicht möglich“ ohne „&“ (DE und EN).
#[test]
fn ct_a_01_undo_impossible_without_ampersand() {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 100, "height": 100})).expect("doc");
    let h = harness_with(s);
    let items = edit_items(&h);
    assert_eq!(&items[..2], ["Rückgängig nicht möglich", "Wiederholen nicht möglich"], "{items:?}");
    fp_i18n::set_language("en");
    let items = edit_items(&h);
    assert_eq!(&items[..2], ["Can't Undo", "Can't Redo"], "{items:?}");
    fp_i18n::set_language("de");
}

/// "Redo: <undone step>" in the user interface language, even when the command ran over the bridge
/// in the response language English (moving the marquee → „Verschieben“, not "Move"). Without an
/// undone step there is nothing to redo (FB-08): the last command is not repeated.
#[test]
fn ct_a_02_redo_label_in_ui_language() {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200})).expect("doc");
    s.execute("select.pixels", json!({"shape": "rect", "rect": [20, 20, 80, 80]})).expect("Pixelauswahl");
    fp_i18n::with_language("en", || s.execute("select.moveMarquee", json!({"dx": 10, "dy": 5}))).expect("verschieben");
    let (undo, redo) = s.undo_redo_labels();
    assert_eq!(undo.as_deref(), Some("Verschieben"));
    assert_eq!(redo, None, "nothing undone: no redo label");
    s.execute("edit.undo", json!({})).expect("undo");
    let (_, redo) = s.undo_redo_labels();
    assert_eq!(redo.as_deref(), Some("Verschieben"), "Wiederholen nennt den Schritt deutsch");
    let h = harness_with(s);
    let items = edit_items(&h);
    assert!(items.iter().any(|i| i == "Wiederholen: Verschieben"), "{items:?}");
}

// ---------------------------------------------------------------------- CT-A-13/15/16/17

/// „Vollbildmodus beenden“ im Leinwand-Kontextmenü nur im Vollbildmodus aktiv.
#[test]
fn ct_a_13_exit_full_screen_only_in_full_screen() {
    let mut h = harness_with(rect_session());
    let state = |h: &H| -> Option<bool> {
        let entries = fp_ui::canvas::context::entries(fp_ui::canvas::context::menu_for(st(h)));
        let nodes = fp_ui::canvas::context::build(st(h), entries);
        nodes.iter().find_map(|n| match n {
            Node::Item { label, enabled, .. } if *label == fp_i18n::t("ctx-exitFullScreen") => Some(*enabled),
            _ => None,
        })
    };
    assert_eq!(state(&h), Some(false), "Normalmodus: grau");
    h.state_mut().state.session.ui.screen_mode = 2;
    assert_eq!(state(&h), Some(true), "Vollbild: aktiv");
    h.state_mut().state.session.ui.screen_mode = 0;
}

/// Bibliothek-≡ ohne gewähltes Symbol: Symbolbefehle grau (wie Stile-≡).
#[test]
fn ct_a_15_library_menu_gray_without_symbol() {
    let mut h = harness_with(rect_session());
    let enabled = |h: &H, name: &str| -> bool {
        let mut nodes = vec![Node::Item { label: name.into(), shortcut: None, action: fp_ui::menu::ItemAction::Command(name.into()), enabled: true, checked: None }];
        fp_ui::dialogs::panel_menus::adjust_menu(st(h), &mut nodes);
        matches!(nodes[0], Node::Item { enabled: true, .. })
    };
    for n in ["panel.library.duplicate", "panel.library.delete", "panel.library.edit", "panel.library.properties", "panel.library.nineSlice", "panel.library.saveToCommon"] {
        assert!(!enabled(&h, n), "{n}: grau ohne Symbol");
    }
    assert!(enabled(&h, "panel.library.newSymbol"), "Neues Symbol bleibt aktiv");
    h.state_mut().state.session.execute("modify.symbol.convertToSymbol", json!({"name": "S1", "type": "graphic"})).expect("Symbol");
    h.state_mut().state.dialogs.panel_sel.symbol = Some(0);
    assert!(enabled(&h, "panel.library.delete"), "mit Symbol aktiv");
}

/// Meldungen der Engine in der Statuszeile beginnen groß.
#[test]
fn ct_a_16_status_sentence_case() {
    assert_eq!(fp_ui::actions::sentence_case("das Exportformat hat keine Palette"), "Das Exportformat hat keine Palette");
    assert_eq!(fp_ui::actions::sentence_case("„x“ fehlt"), "„x“ fehlt");
    assert_eq!(fp_ui::actions::sentence_case(""), "");
}

/// „Vorschau in“ ohne Sekundärbrowser grau, auch mit Dokument (PARITAET: grau in allen Lagen).
#[test]
fn ct_a_17_preview_in_secondary_gray_without_browser() {
    let h = harness_with(rect_session());
    let menus = fp_ui::menu::build_main_menu(st(&h));
    let file = &menus[0].1;
    let sub = file
        .iter()
        .find_map(|n| match n {
            Node::Sub { children, .. } if children.iter().any(|c| matches!(c, Node::Item { action: fp_ui::menu::ItemAction::Command(c), .. } if c == "file.previewInBrowser.previewInSecondaryBrowser")) => Some(children.clone()),
            _ => None,
        })
        .expect("Vorschau im Browser");
    let item = sub
        .iter()
        .find_map(|n| match n {
            Node::Item { label, enabled, action: fp_ui::menu::ItemAction::Command(c), .. } if c == "file.previewInBrowser.previewInSecondaryBrowser" => Some((label.clone(), *enabled)),
            _ => None,
        })
        .expect("Eintrag");
    assert_eq!(item, ("Vorschau in".to_string(), false));
}
