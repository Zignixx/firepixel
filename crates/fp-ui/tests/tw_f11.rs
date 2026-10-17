//! F11 (docs\parity\TWEAKS_2026-10-14.md, Nachlauf 14.10.):
//! - TW-36: „Symbol bearbeiten“ bleibt im Fenster des Dokuments: Titel „<Dokument> (Symbol)“,
//!   Ansicht in Seitengröße, kein eigener Reiter für das Dokument, Pfadleiste mit „Zurück“
//! - TW-39: Popupmenü-Editor, Reiter „Inhalt“ als Tabelle (Text/Hyperlink/Ziel) mit + / − und
//!   Einrücken/Ausrücken, echte Registerreiter

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Rect};
use egui_kittest::kittest::{By, NodeT, Queryable};
use egui_kittest::Harness;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::dialogs::{dialog_rect, last_dialog_id};
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

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn ev(h: &mut H, e: Event) {
    h.event(e);
    h.run_steps(1);
}

fn click(h: &mut H, p: Pos2) {
    ev(h, Event::PointerMoved(p));
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
}

fn key(h: &mut H, k: Key, m: Modifiers) {
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: m });
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: false, repeat: false, modifiers: m });
}

fn typ(h: &mut H, s: &str) {
    for c in s.chars() {
        ev(h, Event::Text(c.to_string()));
    }
}

fn dlg_rect(h: &H) -> Rect {
    dialog_rect(&h.ctx, &last_dialog_id(&h.ctx).expect("Dialog")).expect("Rechteck")
}

fn button_anywhere(h: &H, label: &str) -> Option<(Rect, bool)> {
    h.query_all(By::new().role(Role::Button).label(label)).map(|n| (n.rect(), !n.accesskit_node().is_disabled())).next()
}

fn button(h: &H, label: &str) -> Option<(Rect, bool)> {
    let dr = dlg_rect(h);
    h.query_all(By::new().role(Role::Button).label(label)).map(|n| (n.rect(), !n.accesskit_node().is_disabled())).find(|(r, _)| dr.contains(r.center()))
}

fn click_button(h: &mut H, label: &str) {
    let (r, on) = button(h, label).unwrap_or_else(|| panic!("Knopf {label}"));
    assert!(on, "Knopf {label} gesperrt");
    click(h, r.center());
    h.run_steps(3);
}

fn text_fields(h: &H) -> Vec<(Rect, String)> {
    let dr = dlg_rect(h);
    let mut v: Vec<(Rect, String)> = h.get_all(By::new().role(Role::TextInput)).map(|n| (n.rect(), n.value().unwrap_or_default())).filter(|(r, _)| dr.contains(r.center())).collect();
    v.sort_by(|a, b| a.0.top().total_cmp(&b.0.top()).then(a.0.left().total_cmp(&b.0.left())));
    v
}

/// Feld `i` anklicken, markieren, `text` tippen und mit Tab übernehmen (Enter wäre „Weiter >“).
fn enter_into(h: &mut H, i: usize, text: &str) {
    let r = text_fields(h)[i].0;
    click(h, r.center());
    key(h, Key::A, Modifiers { ctrl: true, command: true, ..Default::default() });
    typ(h, text);
    key(h, Key::Tab, Modifiers::NONE);
    h.run_steps(2);
}

// ---------------------------------------------------------------------------------------- TW-36

fn symbol_session(w: f64, h: f64) -> fp_engine::Session {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": w, "height": h, "background": "#ffffff"})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [150, 150, 250, 220], "name": "R1"})).expect("rect");
    s.execute("modify.symbol.convertToSymbol", json!({"name": "ST"})).expect("Symbol");
    s
}

/// Symbol bearbeiten: Ansicht so groß wie die Seite, Titel „<Dokument> (Symbol)“, der Inhalt liegt
/// auf der Leinwand; das Dokument hat keinen eigenen Reiter, „Zurück“ beendet die Bearbeitung.
#[test]
fn tw36_symbol_edit_stays_in_document_window() {
    fp_i18n::with_language("de", || {
        let mut s = symbol_session(800.0, 600.0);
        let parent = s.active_id().expect("Dokument");
        let parent_title = s.active().expect("d").title.clone();
        s.execute("modify.symbol.editSymbol", json!({})).expect("bearbeiten");
        let d = s.active().expect("Ansicht");
        assert_eq!(d.title, format!("{parent_title} (Symbol)"));
        let c = &d.doc.pages[0].canvas;
        assert_eq!((c.width, c.height), (800.0, 600.0), "Seitengröße wie im Original");
        // Inhalt vollständig auf der Leinwand
        let e = d.doc.pages[0].root.layers.iter().find_map(|l| if let fp_model::LayerSlot::Own(l) = l { l.cels.first().and_then(|c| c.elements.first().cloned()) } else { None }).expect("Inhalt");
        let b = fp_engine::docops::element_bounds(&d.doc, &e).expect("Grenzen");
        assert!(b.left >= c.origin.x && b.top >= c.origin.y && b.right <= c.origin.x + c.width && b.bottom <= c.origin.y + c.height, "Inhalt {b:?} in {c:?}");

        let mut h = harness_with(s);
        h.run_steps(3);
        assert_eq!(fp_ui::canvas::hidden_by_symbol_edit(st(&h)), vec![parent], "Dokumentreiter vertreten");
        assert_eq!(fp_ui::canvas::symbol_path(st(&h)), Some(("Seite 1".to_string(), "ST".to_string())));
        let (r, on) = button_anywhere(&h, "Zurück").expect("Zurück in der Pfadleiste");
        assert!(on);
        click(&mut h, r.center());
        h.run_steps(3);
        assert_eq!(st(&h).session.active_id(), Some(parent), "zurück im Dokument");
        assert!(fp_ui::canvas::symbol_path(st(&h)).is_none());
        assert!(button_anywhere(&h, "Zurück").is_none(), "Pfadleiste weg");
    });
}

/// Neues (leeres) Symbol: ebenfalls Seitengröße statt 100 × 100.
#[test]
fn tw36_new_symbol_uses_page_size() {
    fp_i18n::with_language("de", || {
        let mut s = fp_engine::Session::default();
        s.execute("file.new", json!({"width": 400, "height": 300})).expect("doc");
        let t = s.active().expect("d").title.clone();
        s.execute("edit.insert.newSymbol", json!({"name": "Leer", "edit": true})).expect("neu");
        let d = s.active().expect("Ansicht");
        assert_eq!(d.title, format!("{t} (Symbol)"));
        let c = &d.doc.pages[0].canvas;
        assert_eq!((c.width, c.height, c.origin.x, c.origin.y), (400.0, 300.0, -200.0, -150.0), "Bezugspunkt in der Mitte");
    });
}

// ---------------------------------------------------------------------------------------- TW-39

fn slice_session() -> fp_engine::Session {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [10, 10, 100, 60]})).expect("rect");
    s.execute("edit.insert.slice", json!({})).expect("slice");
    s
}

/// Reiter „Inhalt“: echte Registerreiter, eine leere Tabellenzeile mit drei Zellen, + legt eine
/// Zeile an, Einrücken macht sie zum Untermenü, − löscht; Fertig legt das verschachtelte Menü an.
#[test]
fn tw39_popup_editor_content_table() {
    fp_i18n::with_language("de", || {
        let mut h = harness_with(slice_session());
        h.state_mut().state.run("modify.popUpMenu.addPopUpMenu", json!({}));
        h.run_steps(4);
        let d = st(&h).dialogs.popup_editor.clone().expect("Editor offen");
        assert_eq!(d.rows.len(), 1);
        assert_eq!(d.rows[0].text, "", "leere Zeile wie im Original");
        // Registerreiter
        let tab = |h: &H, l: &str| h.query_all(By::new().label(l)).map(|n| n.accesskit_node().toggled().map(|t| t == egui::accesskit::Toggled::True)).next();
        assert_eq!(tab(&h, "Inhalt"), Some(Some(true)), "Reiter Inhalt aktiv");
        assert_eq!(tab(&h, "Erscheinungsbild"), Some(Some(false)));
        // drei Zellen der gewählten Zeile, nebeneinander
        let f = text_fields(&h);
        assert_eq!(f.len(), 3, "Text/Hyperlink/Ziel");
        assert!((f[0].0.top() - f[2].0.top()).abs() < 1.0 && f[0].0.left() < f[1].0.left() && f[1].0.left() < f[2].0.left());
        enter_into(&mut h, 0, "Start");
        enter_into(&mut h, 1, "index.html");
        // vier Werkzeugknöpfe statt sechs
        assert!(button(&h, "↑").is_none() && button(&h, "↓").is_none());
        for l in ["Menü hinzufügen", "Menü löschen", "Menü einrücken", "Menü ausrücken"] {
            assert!(button(&h, l).is_some(), "Werkzeugknopf {l}");
        }
        assert_eq!(button(&h, "Menü einrücken").map(|b| b.1), Some(false), "erste Zeile nicht einrückbar");
        click_button(&mut h, "Menü hinzufügen");
        enter_into(&mut h, 0, "Unter");
        click_button(&mut h, "Menü einrücken");
        let d = st(&h).dialogs.popup_editor.clone().expect("Editor");
        assert_eq!(d.rows.iter().map(|r| (r.level, r.text.as_str(), r.url.as_str())).collect::<Vec<_>>(), vec![(0, "Start", "index.html"), (1, "Unter", "")]);
        let (_, p) = d.invocation();
        assert_eq!(p["items"][0]["submenu"][0]["text"], json!("Unter"));
        // dritte Zeile anlegen und wieder löschen
        click_button(&mut h, "Menü hinzufügen");
        assert_eq!(st(&h).dialogs.popup_editor.as_ref().map(|d| d.rows.len()), Some(3));
        click_button(&mut h, "Menü löschen");
        assert_eq!(st(&h).dialogs.popup_editor.as_ref().map(|d| d.rows.len()), Some(2));
        // Fertig über die Karten
        for _ in 0..3 {
            click_button(&mut h, "Weiter >");
        }
        let n0 = st(&h).session.active().map(|d| d.history.undo_count()).unwrap_or(0);
        click_button(&mut h, "Fertig");
        assert!(st(&h).dialogs.popup_editor.is_none());
        assert_eq!(st(&h).session.active().map(|d| d.history.undo_count()), Some(n0 + 1), "Menü angelegt");
    });
}
