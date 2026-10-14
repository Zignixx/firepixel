//! P-DLG9: In Symbol konvertieren (D27), Animieren (D28), Instanzen tweenen (D33) und Popupmenü
//! hinzufügen (D34) wie im Original (PM-6, PARITAET_B §2): Beschriftungen und Anordnung, Rechteck
//! des Animationssymbols mit Bewegungsweg, Verlaufsname, zweite Meldung nach Abbrechen.

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Rect};
use egui_kittest::kittest::{By, NodeT, Queryable};
use egui_kittest::Harness;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn session() -> (fp_engine::Session, u64) {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("doc");
    let id = s.execute("draw.rectangle", json!({"rect": [20, 20, 120, 100]})).expect("rect").value["id"].as_u64().expect("id");
    (s, id)
}

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

fn key(h: &mut H, k: Key) {
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: Modifiers::NONE });
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: false, repeat: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
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
    assert!(st(h).dialogs.form.is_some(), "{cmd}: Formular offen");
}

fn dialog_rect(h: &H) -> Rect {
    fp_ui::dialogs::dialog_rect(&h.ctx, &fp_ui::dialogs::last_dialog_id(&h.ctx).expect("Dialog")).expect("Rechteck")
}

fn client(h: &H) -> egui::Vec2 {
    fp_ui::dialogs::dialog_geom(&h.ctx, &fp_ui::dialogs::last_dialog_id(&h.ctx).expect("Dialog")).expect("Lage").client
}

/// Knoten einer Rolle im Dialog (Rechteck, Name).
fn nodes(h: &H, role: Role) -> Vec<(Rect, String)> {
    let dr = dialog_rect(h);
    let mut v: Vec<(Rect, String)> = h.query_all(By::new().role(role)).map(|n| (n.rect(), n.accesskit_node().label().or_else(|| n.value()).unwrap_or_default())).filter(|(r, _)| dr.contains(r.center())).collect();
    v.sort_by(|a, b| a.0.top().total_cmp(&b.0.top()).then(a.0.left().total_cmp(&b.0.left())));
    v
}

fn node(h: &H, role: Role, label: &str) -> Rect {
    nodes(h, role).into_iter().find(|(_, l)| l == label).unwrap_or_else(|| panic!("{role:?} „{label}“ fehlt: {:?}", nodes(h, role))).0
}

fn focused(h: &H) -> Option<(Role, String)> {
    h.ctx.memory(|m| m.focused())?;
    h.get_all(By::new()).find(|n| n.accesskit_node().is_focused()).map(|n| {
        let node = n.accesskit_node();
        (node.role(), node.label().map(|s| s.to_string()).or_else(|| node.value().map(|s| s.to_string())).unwrap_or_default())
    })
}

fn label_of(h: &H, key: &str) -> String {
    st(h).dialogs.form.as_ref().and_then(|f| f.fields.iter().find(|x| x.key == key)).map(|f| f.label.clone()).unwrap_or_default()
}

fn bounds(s: &fp_engine::Session, id: u64) -> fp_geom::Rect {
    let d = s.active().expect("doc");
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, fp_model::ElementId(id)).expect("Element");
    fp_engine::docops::element_bounds(&d.doc, &e).expect("Grenzen")
}

fn last_step(s: &fp_engine::Session) -> String {
    s.active().and_then(|d| d.history.undo_label().map(|x| x.to_string())).unwrap_or_default()
}

// ---------------------------------------------------------------------- D28 Engine

/// Original: R1 (20,20,120,100), Status 3, Verschieben 72 (Richtung 0) → Instanz 20,20,192,100
/// (Bewegungsweg eingerechnet), Schritt „In Symbol konvertieren“. Richtung 90 → nach oben.
#[test]
fn animation_symbol_rect_contains_motion_path() {
    let (mut s, r1) = session();
    s.execute("select.set", json!({"ids": [r1]})).expect("Auswahl");
    let id = s.execute("modify.animation.animateSelection", json!({"frames": 3, "offsetX": 72, "offsetY": 0})).expect("animieren").value["id"].as_u64().expect("id");
    let b = bounds(&s, id);
    assert_eq!((b.left.round(), b.top.round(), b.right.round(), b.bottom.round()), (20.0, 20.0, 192.0, 100.0), "Rechteck mit Bewegungsweg: {b:?}");
    assert_eq!(last_step(&s), "In Symbol konvertieren", "Verlaufsname wie im Original");
    s.execute("edit.undo", json!({})).expect("undo");
    let b0 = bounds(&s, r1);
    assert_eq!((b0.left.round(), b0.top.round(), b0.right.round(), b0.bottom.round()), (20.0, 20.0, 120.0, 100.0), "Rückgängig stellt das Rechteck wieder her");
    // Richtung 90° (nach oben), 50 px: oben 20 − 50
    s.execute("select.set", json!({"ids": [r1]})).expect("Auswahl");
    let id = s.execute("modify.animation.animateSelection", json!({"frames": 5, "offsetX": 0, "offsetY": -50})).expect("animieren").value["id"].as_u64().expect("id");
    let b = bounds(&s, id);
    assert_eq!((b.left.round(), b.top.round(), b.right.round(), b.bottom.round()), (20.0, -30.0, 120.0, 100.0), "Weg nach oben: {b:?}");
    // ohne Bewegung (1 Status) bleibt das Rechteck des Inhalts
    s.execute("modify.animation.settings", json!({"frames": 1})).expect("Einstellungen");
    let b = bounds(&s, id);
    assert_eq!((b.left.round(), b.top.round(), b.right.round(), b.bottom.round()), (20.0, 20.0, 120.0, 100.0), "ein Status: {b:?}");
}

/// Englischer Verlaufsname: der von „In Symbol konvertieren“.
#[test]
fn animate_step_name_english() {
    fp_i18n::set_language("en");
    let (mut s, r1) = session();
    s.execute("select.set", json!({"ids": [r1]})).expect("Auswahl");
    s.execute("modify.animation.animateSelection", json!({"frames": 3, "offsetX": 72})).expect("animieren");
    let conv = s.registry().get("modify.symbol.convertToSymbol").map(|c| c.info().step_label()).unwrap_or_default();
    assert_eq!(last_step(&s), conv);
    assert!(!conv.is_empty() && conv != "In Symbol konvertieren", "englisch: {conv}");
    fp_i18n::set_language("de");
}

// ---------------------------------------------------------------------- D28 Dialog

/// Animieren: Beschriftungen wie im Original, Deckkraft „… bis …“ und Drehen mit „Nach rechts/Nach
/// links“ je in einer Zeile, Startfokus OK, Größe ≈ 215 × 152 DLU; Enter → Animationssymbol mit Weg.
#[test]
fn animate_dialog_like_the_original() {
    let (s, r1) = session();
    let mut h = harness_with(s);
    h.state_mut().state.session.execute("select.set", json!({"ids": [r1]})).expect("Auswahl");
    h.run_steps(2);
    open(&mut h, "modify.animation.animateSelection");
    for (k, want) in [("frames", "Status:"), ("_dist", "Verschieben:"), ("_dir", "Richtung:"), ("scale", "Skalieren auf:"), ("startOpacity", "Deckkraft:"), ("endOpacity", "bis"), ("_rot", "Drehen:")] {
        assert_eq!(label_of(&h, k), want, "Beschriftung {k}");
    }
    let c = client(&h);
    assert!((c.x - 322.5).abs() < 12.0 && (c.y - 247.0).abs() < 8.0, "Größe wie im Original (≈ 323 × 247): {c:?}");
    assert_eq!(focused(&h).map(|f| f.0), Some(Role::Button), "Startfokus OK");
    let fields: Vec<(Rect, String)> = nodes(&h, Role::TextInput);
    assert_eq!(fields.len(), 7, "sieben Felder: {fields:?}");
    // Deckkraft von/bis in einer Zeile
    let (a, b) = (fields[4].0, fields[5].0);
    assert!((a.top() - b.top()).abs() < 1.0 && b.left() > a.right() + 20.0, "Deckkraft und bis nebeneinander: {a:?} {b:?}");
    let rot = fields[6].0;
    let cw = node(&h, Role::RadioButton, "Nach rechts");
    let ccw = node(&h, Role::RadioButton, "Nach links");
    assert!((cw.center().y - rot.center().y).abs() < 3.0 && (ccw.center().y - rot.center().y).abs() < 3.0, "Drehrichtung in der Zeile von Drehen");
    assert!(cw.left() > rot.right() && ccw.left() > cw.right(), "Nach rechts, dann Nach links");
    // Status oben, durch eine Trennlinie abgesetzt: deutlich größerer Abstand zur nächsten Zeile
    assert!(fields[1].0.top() - fields[0].0.top() > 1.5 * (fields[2].0.top() - fields[1].0.top()), "Abstand nach Status (Trennlinie)");
    let ok = node(&h, Role::Button, &fp_i18n::t("btn-ok"));
    let cancel = node(&h, Role::Button, &fp_i18n::t("btn-cancel"));
    assert!(ok.top() > rot.bottom() + 20.0 && cancel.left() > ok.right(), "OK/Abbrechen unten nebeneinander");
    // alles innerhalb des Dialogs, nichts überlappt
    let dr = dialog_rect(&h);
    for (r, l) in nodes(&h, Role::TextInput).into_iter().chain(nodes(&h, Role::RadioButton)).chain(nodes(&h, Role::Button)) {
        assert!(dr.contains_rect(r), "{l} im Dialog: {r:?} / {dr:?}");
    }
    // Enter (OK hat den Fokus) → Animationssymbol, Rechteck mit dem Weg (Vorgabe Richtung 0, 72 px)
    key(&mut h, Key::Enter);
    h.run_steps(4);
    assert!(st(&h).dialogs.form.is_none(), "Enter schließt");
    // Q-20: 5 Status > 1 → Rückfrage „Automatisch neue Status hinzufügen?“, Enter = OK
    let w = st(&h).dialogs.warning.clone().expect("Rückfrage");
    assert_eq!(w.warning.key, fp_ui::dialogs::warning::ANIMATE_STATES);
    key(&mut h, Key::Enter);
    h.run_steps(4);
    assert!(st(&h).dialogs.warning.is_none(), "Enter bestätigt die Rückfrage");
    assert_eq!(st(&h).session.active().map(|d| d.frame_count()), Some(5), "OK hängt die Status an");
    let s = &st(&h).session;
    let d = s.active().expect("doc");
    let id = d.selection.elements.first().expect("Instanz").0;
    let b = bounds(s, id);
    assert_eq!((b.left.round(), b.top.round(), b.right.round(), b.bottom.round()), (20.0, 20.0, 192.0, 100.0), "Rechteck mit Weg: {b:?}");
}

/// Englisch: Beschriftungen übersetzt, nichts ragt aus dem Dialog.
#[test]
fn animate_dialog_english_fits() {
    fp_i18n::set_language("en");
    let (s, r1) = session();
    let mut h = harness_with(s);
    h.state_mut().state.session.execute("select.set", json!({"ids": [r1]})).expect("Auswahl");
    h.run_steps(2);
    open(&mut h, "modify.animation.animateSelection");
    assert_eq!(label_of(&h, "_dist"), "Move:");
    assert_eq!(label_of(&h, "startOpacity"), "Opacity:");
    let dr = dialog_rect(&h);
    for (r, l) in nodes(&h, Role::RadioButton).into_iter().chain(nodes(&h, Role::Button)).chain(nodes(&h, Role::TextInput)) {
        assert!(dr.contains_rect(r), "{l} im Dialog");
    }
    key(&mut h, Key::Escape);
    fp_i18n::set_language("de");
}

// ---------------------------------------------------------------------- D27

/// In Symbol konvertieren: Typ untereinander, „Optionen:“ mit 9-Slice und „In allgemeiner
/// Bibliothek speichern“, OK/Abbrechen rechts oben, Größe ≈ 326 × 102 DLU; Name hat den Fokus.
#[test]
fn convert_to_symbol_like_the_original() {
    let (s, r1) = session();
    let mut h = harness_with(s);
    h.state_mut().state.session.execute("select.set", json!({"ids": [r1]})).expect("Auswahl");
    h.run_steps(2);
    open(&mut h, "modify.symbol.convertToSymbol");
    let c = client(&h);
    assert!((c.x - 489.0).abs() < 12.0 && (c.y - 166.0).abs() < 8.0, "Größe wie im Original (≈ 489 × 166): {c:?}");
    assert_eq!(fp_i18n::t("dlg-lbl-options"), "Optionen:");
    let g = node(&h, Role::RadioButton, "Grafik");
    let a = node(&h, Role::RadioButton, "Animation");
    let b = node(&h, Role::RadioButton, "Schaltfläche");
    assert!((g.left() - a.left()).abs() < 1.0 && (a.left() - b.left()).abs() < 1.0 && g.top() < a.top() && a.top() < b.top(), "Typ untereinander");
    let nine = node(&h, Role::CheckBox, "Hilfslinien für 9-Slice-Skalierung aktivieren");
    let lib = node(&h, Role::CheckBox, "In allgemeiner Bibliothek speichern");
    assert!(nine.top() > b.bottom() && lib.top() > nine.bottom(), "Optionen unter dem Typ");
    let ok = node(&h, Role::Button, &fp_i18n::t("btn-ok"));
    let cancel = node(&h, Role::Button, &fp_i18n::t("btn-cancel"));
    assert!((ok.left() - cancel.left()).abs() < 1.0 && cancel.top() > ok.bottom(), "OK über Abbrechen");
    assert!(ok.left() > g.right() && ok.bottom() < nine.top(), "Knöpfe rechts oben");
    let name = nodes(&h, Role::TextInput);
    assert_eq!(name.len(), 1);
    assert_eq!(name[0].1, "Symbol");
    assert_eq!(focused(&h).map(|f| f.0), Some(Role::TextInput), "Startfokus Name");
    let dr = dialog_rect(&h);
    for (r, l) in nodes(&h, Role::RadioButton).into_iter().chain(nodes(&h, Role::CheckBox)).chain(nodes(&h, Role::Button)) {
        assert!(dr.contains_rect(r), "{l} im Dialog");
    }
    // Animation wählen, Enter → Animationssymbol
    click(&mut h, a.center());
    key(&mut h, Key::Enter);
    h.run_steps(4);
    assert!(st(&h).dialogs.form.is_none(), "Enter schließt");
    let s = &st(&h).session;
    let d = s.active().expect("doc");
    let sym = d.doc.symbols.last().expect("Symbol");
    assert_eq!(sym.kind(), fp_model::SymbolKind::Animation);
}

/// Englisch: „Options:“, „Save in Common Library“, alles im Dialog.
#[test]
fn convert_to_symbol_english_fits() {
    fp_i18n::set_language("en");
    let (s, r1) = session();
    let mut h = harness_with(s);
    h.state_mut().state.session.execute("select.set", json!({"ids": [r1]})).expect("Auswahl");
    h.run_steps(2);
    open(&mut h, "modify.symbol.convertToSymbol");
    let _ = node(&h, Role::CheckBox, "Save in Common Library");
    let dr = dialog_rect(&h);
    for (r, l) in nodes(&h, Role::RadioButton).into_iter().chain(nodes(&h, Role::CheckBox)).chain(nodes(&h, Role::Button)).chain(nodes(&h, Role::TextInput)) {
        assert!(dr.contains_rect(r), "{l} im Dialog");
    }
    key(&mut h, Key::Escape);
    fp_i18n::set_language("de");
}

// ---------------------------------------------------------------------- D33

/// Instanzen tweenen ohne zwei Instanzen: Meldung mit dem Wortlaut des Originals, kein Schritt.
#[test]
fn tween_instances_warning_text() {
    let (s, _) = session();
    let mut h = harness_with(s);
    let before = st(&h).session.active().map(|d| d.history.undo_count());
    h.state_mut().state.run("modify.symbol.tweenInstances", json!({}));
    h.run_steps(4);
    let w = st(&h).dialogs.warning.clone().expect("Meldung");
    assert_eq!(w.warning.text, "Wählen Sie zwei oder mehr Instanzen desselben Bibliothekssymbols aus, bevor Sie den Tween-Befehl verwenden.");
    assert!(w.warning.pref.is_some(), "Nicht mehr anzeigen");
    key(&mut h, Key::Enter);
    assert!(st(&h).dialogs.warning.is_none());
    assert_eq!(st(&h).session.active().map(|d| d.history.undo_count()), before, "kein Verlaufsschritt");
}

// ---------------------------------------------------------------------- D34

const SECOND: &str = "Wählen Sie ein Segment oder einen Hotspot aus, bevor Sie den Befehl „Verhalten einfügen“ verwenden.";

fn open_popup_question(h: &mut H) {
    h.state_mut().state.run("modify.popUpMenu.addPopUpMenu", json!({}));
    h.run_steps(4);
    let w = st(h).dialogs.warning.clone().expect("Rückfrage");
    assert_eq!(w.warning.choices.len(), 2, "Hotspot/Segment");
}

/// Popupmenü hinzufügen ohne Hotspot/Segment: nach Esc und nach „Abbrechen“ folgt die zweite
/// Meldung des Originals (mit „Nicht mehr anzeigen“); OK schließt sie ohne Verlaufsschritt.
#[test]
fn popup_cancel_shows_second_message() {
    let (s, _) = session();
    let mut h = harness_with(s);
    let before = st(&h).session.active().map(|d| d.history.undo_count());
    // Esc
    open_popup_question(&mut h);
    key(&mut h, Key::Escape);
    let w = st(&h).dialogs.warning.clone().expect("zweite Meldung nach Esc");
    assert_eq!(w.warning.text, SECOND);
    assert!(w.warning.choices.is_empty() && w.warning.pref.is_some(), "Meldung mit OK und „Nicht mehr anzeigen“");
    let _ = node(&h, Role::CheckBox, &fp_i18n::t("msg-warn-dont-show").replace('&', ""));
    key(&mut h, Key::Enter);
    assert!(st(&h).dialogs.warning.is_none(), "OK schließt");
    // Knopf „Abbrechen“
    open_popup_question(&mut h);
    let cancel = node(&h, Role::Button, &fp_i18n::t("msg-warn-choice-cancel").replace('&', ""));
    click(&mut h, cancel.center());
    let w = st(&h).dialogs.warning.clone().expect("zweite Meldung nach Abbrechen");
    assert_eq!(w.warning.text, SECOND);
    // „Nicht mehr anzeigen“ ankreuzen → beim nächsten Abbrechen keine zweite Meldung mehr
    let check = node(&h, Role::CheckBox, &fp_i18n::t("msg-warn-dont-show").replace('&', ""));
    click(&mut h, check.center());
    key(&mut h, Key::Enter);
    assert!(st(&h).dialogs.warning.is_none());
    open_popup_question(&mut h);
    key(&mut h, Key::Escape);
    assert!(st(&h).dialogs.warning.is_none(), "abgeschaltet: keine zweite Meldung");
    assert_eq!(st(&h).session.active().map(|d| d.history.undo_count()), before, "kein Verlaufsschritt");
    // „Warnmeldungen zurücksetzen“ bringt sie zurück
    h.state_mut().state.session.prefs.reset_warnings();
    open_popup_question(&mut h);
    key(&mut h, Key::Escape);
    assert_eq!(st(&h).dialogs.warning.as_ref().map(|w| w.warning.text.clone()).as_deref(), Some(SECOND));
}

/// Englischer Wortlaut der zweiten Meldung.
#[test]
fn popup_second_message_english() {
    fp_i18n::set_language("en");
    let (s, _) = session();
    let mut h = harness_with(s);
    open_popup_question(&mut h);
    key(&mut h, Key::Escape);
    let w = st(&h).dialogs.warning.clone().expect("zweite Meldung");
    assert_eq!(w.warning.text, "Select a slice or a hotspot before using the \"Add Behavior\" command.");
    fp_i18n::set_language("de");
}
