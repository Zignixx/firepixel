//! FORM-1 (Formular-Grundlagen wie im Original, PARITAET_B D14/D26–D28/D35–D39/D41/D42/D46):
//! - Tab-Folge der Knöpfe je Vorlage (D27 Name › Typ › OK › Abbrechen › Kästchen, D28 OK › Abbrechen ›
//!   Felder, D14 wie D27, D41/D42/D46 wie gemessen)
//! - Popup-Schieber ▾ ist kein Tab-Stopp
//! - `FormDialog::load_prefs` stellt Auswahllisten mit Text-Werten wieder her
//! - 19105–19108: ungültige Werte werden erst beim OK geklemmt, der Dialog bleibt dabei offen
//! - D14 „In allgemeiner Bibliothek speichern“ gesperrt
//! - Klick neben einen modalen Dialog schließt ihn nicht (nur Esc, Kreuz, Abbrechen)
//! - D26 Startfokus auf der Liste, Pfeiltasten wählen
//!
//! Realistische Eingaben: Tasten wie egui-winit (Drücken/Loslassen), Tippen mit Taste + Text,
//! Mausklicks über mehrere Bilder.

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Rect};
use egui_kittest::kittest::{By, NodeT, Queryable};
use egui_kittest::Harness;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::dialogs::form::{Field, FormDialog};
use fp_ui::FirepixelApp;
use serde_json::{json, Value};

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

/// Dokument mit dem Rechteck R1 (20,20,120,100), ausgewählt.
fn rect_session() -> fp_engine::Session {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("doc");
    let id = s.execute("draw.rectangle", json!({"rect": [20, 20, 120, 100]})).expect("rect").value["id"].as_u64().expect("id");
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

/// Dokument mit einer Pixelauswahl (Auswahl-Dialoge).
fn marquee_session() -> fp_engine::Session {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 500, "height": 400, "background": "#ffffff"})).expect("doc");
    s.execute("select.pixels", json!({"shape": "rect", "rect": [330, 210, 380, 260]})).expect("Pixelauswahl");
    s
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

fn key_m(h: &mut H, k: Key, m: Modifiers) {
    h.event(Event::ModifiersChanged(m));
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: m });
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: false, repeat: false, modifiers: m });
    ev(h, Event::ModifiersChanged(Modifiers::NONE));
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

fn dialog_rect(h: &H) -> Rect {
    fp_ui::dialogs::dialog_rect(&h.ctx, &fp_ui::dialogs::last_dialog_id(&h.ctx).expect("Dialog")).expect("Rechteck")
}

/// Rolle und Name (bzw. Wert) des fokussierten Bedienelements.
fn focused(h: &H) -> Option<(Role, String)> {
    h.ctx.memory(|m| m.focused())?;
    h.get_all(By::new()).find(|n| n.accesskit_node().is_focused()).map(|n| {
        let node = n.accesskit_node();
        (node.role(), node.label().map(|s| s.to_string()).or_else(|| node.value().map(|s| s.to_string())).unwrap_or_default())
    })
}

fn role_of(h: &H) -> Option<Role> {
    focused(h).map(|f| f.0)
}

/// Tab drücken und je Schritt das fokussierte Element festhalten.
fn tabs(h: &mut H, n: usize) -> Vec<(Role, String)> {
    (0..n)
        .map(|_| {
            key(h, Key::Tab);
            focused(h).unwrap_or((Role::Unknown, String::new()))
        })
        .collect()
}

fn button(h: &H, label: &str) -> Rect {
    let dr = dialog_rect(h);
    h.query_all(By::new().role(Role::Button).label(label)).map(|n| n.rect()).find(|r| dr.contains(r.center())).unwrap_or_else(|| panic!("Knopf {label}"))
}

fn form_value(h: &H, key: &str) -> Value {
    st(h).dialogs.form.as_ref().expect("Formular").get(key)
}

fn undo_count(h: &H) -> usize {
    st(h).session.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

fn text_fields(h: &H) -> Vec<(Rect, String)> {
    let dr = dialog_rect(h);
    let mut v: Vec<(Rect, String)> = h.get_all(By::new().role(Role::TextInput)).map(|n| (n.rect(), n.value().unwrap_or_default())).filter(|(r, _)| dr.contains(r.center())).collect();
    v.sort_by(|a, b| a.0.top().total_cmp(&b.0.top()));
    v
}

fn ok_l() -> String {
    fp_i18n::t("btn-ok")
}

fn cancel_l() -> String {
    fp_i18n::t("btn-cancel")
}

// ---------------------------------------------------------------------- Tab-Folge der Knöpfe

/// D27 In Symbol konvertieren: Name › Typ (ein Stopp) › OK › Abbrechen › 9-Slice › Bibliothek.
#[test]
fn d27_tab_order_buttons_after_type() {
    let mut h = harness_with(rect_session());
    open(&mut h, "modify.symbol.convertToSymbol");
    assert_eq!(role_of(&h), Some(Role::TextInput), "Startfokus Name");
    let seq = tabs(&mut h, 5);
    let want = [(Role::RadioButton, fp_i18n::t("dlg-symbol-graphic")), (Role::Button, ok_l()), (Role::Button, cancel_l()), (Role::CheckBox, fp_i18n::t("dlg-lbl-enable-nineslice")), (Role::CheckBox, fp_i18n::t("dlg-lbl-save-common-library"))];
    assert_eq!(seq, want.to_vec(), "Tab-Folge wie im Original");
}

/// D28 Animieren: Startfokus OK, Tab OK › Abbrechen › Status (erstes Feld).
#[test]
fn d28_tab_order_buttons_first() {
    let mut h = harness_with(rect_session());
    open(&mut h, "modify.animation.animateSelection");
    assert_eq!(focused(&h), Some((Role::Button, ok_l())), "Startfokus OK");
    let seq = tabs(&mut h, 3);
    assert_eq!(seq[0], (Role::Button, cancel_l()), "nach OK Abbrechen");
    assert_eq!(seq[1].0, Role::TextInput, "dann das erste Feld");
    assert_eq!(seq[1].1, "5", "Status");
    assert_eq!(seq[2].0, Role::TextInput, "▾ ist kein Tab-Stopp: nach Status gleich Verschieben ({seq:?})");
    assert_eq!(seq[2].1, "72", "Verschieben");
    // Enter mit Fokus im Feld löst weiter OK aus
    key(&mut h, Key::Enter);
    h.run_steps(4);
    assert!(st(&h).dialogs.form.is_none(), "Enter = OK");
}

/// D14 Neues Symbol: wie D27, „In allgemeiner Bibliothek speichern“ gesperrt (kein Tab-Stopp, Klick
/// ohne Wirkung); Tab Name › Grafik › OK › Abbrechen › 9-Slice.
#[test]
fn d14_common_library_disabled_and_tab_order() {
    let mut h = harness_with(rect_session());
    open(&mut h, "edit.insert.newSymbol");
    let lib = fp_i18n::t("dlg-lbl-save-common-library");
    let dr = dialog_rect(&h);
    let node = h.query_all(By::new().role(Role::CheckBox).label(lib.as_str())).find(|n| dr.contains(n.rect().center())).map(|n| (n.rect(), n.accesskit_node().is_disabled())).expect("Kästchen Bibliothek");
    assert!(node.1, "gesperrt wie im Original");
    assert_eq!(role_of(&h), Some(Role::TextInput), "Startfokus Name");
    let seq = tabs(&mut h, 5);
    let want = [(Role::RadioButton, fp_i18n::t("dlg-symbol-graphic")), (Role::Button, ok_l()), (Role::Button, cancel_l()), (Role::CheckBox, fp_i18n::t("dlg-lbl-enable-nineslice")), (Role::TextInput, fp_i18n::t("dlg-symbol-default-name"))];
    assert_eq!(&seq[..4], &want[..4], "Tab-Folge wie im Original");
    assert_ne!(seq[4], (Role::CheckBox, lib.clone()), "gesperrtes Kästchen ist kein Tab-Stopp");
    click(&mut h, node.0.center());
    assert_eq!(form_value(&h, "_common"), json!(false), "Klick ohne Wirkung");
    // OK legt das Symbol an (Parameter ohne das Oberflächenfeld)
    let ok = button(&h, &ok_l());
    click(&mut h, ok.center());
    h.run_steps(4);
    assert!(st(&h).dialogs.form.is_none(), "OK schließt");
    assert_eq!(st(&h).session.active().map(|d| d.doc.symbols.len()), Some(1), "ein Symbol");
}

/// D42 Helligkeit/Kontrast: Startfokus Schieber; Tab Schieber › Feld › Schieber › Feld › Vorschau › OK
/// › Abbrechen.
#[test]
fn d42_tab_order() {
    let mut h = harness_with(bitmap_session());
    open(&mut h, "filter.adjustColor.brightnessContrast");
    assert_eq!(role_of(&h), Some(Role::Slider), "Startfokus Schieber");
    let seq: Vec<Role> = tabs(&mut h, 6).into_iter().map(|x| x.0).collect();
    assert_eq!(seq, vec![Role::TextInput, Role::Slider, Role::TextInput, Role::CheckBox, Role::Button, Role::Button], "Tab-Folge");
    assert_eq!(focused(&h), Some((Role::Button, cancel_l())), "zuletzt Abbrechen");
    key(&mut h, Key::Escape);
    h.run_steps(4);
}

/// D46 Unscharf maskieren: Felder › Vorschau › Schieber › OK › Abbrechen.
#[test]
fn d46_tab_order() {
    let mut h = harness_with(bitmap_session());
    open(&mut h, "filter.sharpen.unsharpMask");
    assert_eq!(role_of(&h), Some(Role::TextInput), "Startfokus erstes Feld");
    let seq = tabs(&mut h, 8);
    let roles: Vec<Role> = seq.iter().map(|x| x.0).collect();
    assert_eq!(roles, vec![Role::TextInput, Role::TextInput, Role::CheckBox, Role::Slider, Role::Slider, Role::Slider, Role::Button, Role::Button], "Tab-Folge {seq:?}");
    assert_eq!(seq[2].1, fp_i18n::t("dlg-preview"));
    assert_eq!(seq[6].1, ok_l());
    key(&mut h, Key::Escape);
    h.run_steps(4);
}

/// D41 Farbton/Sättigung: nur Felder › OK › Abbrechen › Kolorieren › Vorschau (Schieber keine Stopps).
#[test]
fn d41_sliders_are_no_tab_stops() {
    let mut h = harness_with(bitmap_session());
    open(&mut h, "filter.adjustColor.hueSaturation");
    assert_eq!(role_of(&h), Some(Role::TextInput), "Startfokus Farbton");
    let seq = tabs(&mut h, 6);
    let roles: Vec<Role> = seq.iter().map(|x| x.0).collect();
    assert_eq!(roles, vec![Role::TextInput, Role::TextInput, Role::Button, Role::Button, Role::CheckBox, Role::CheckBox], "Tab-Folge {seq:?}");
    assert_eq!(seq[4].1, fp_i18n::t("dlg-lbl-colorize"));
    assert_eq!(seq[5].1, fp_i18n::t("dlg-preview"));
    key(&mut h, Key::Escape);
    h.run_steps(4);
}

// ---------------------------------------------------------------------- Popup-Schieber

/// ▾ ist kein Tab-Stopp: Textgröße (ein Feld mit ▾) Tab vom Feld direkt zu OK; Bedienung per Maus geht.
#[test]
fn popup_arrow_is_no_tab_stop() {
    let mut h = harness_with(rect_session());
    open(&mut h, "text.size.other");
    assert_eq!(role_of(&h), Some(Role::TextInput), "Startfokus Feld");
    let seq = tabs(&mut h, 1);
    assert_eq!(seq[0], (Role::Button, ok_l()), "vom Feld direkt zu OK");
    // Klick auf ▾ öffnet weiter den senkrechten Schieber
    let field = text_fields(&h)[0].0;
    click(&mut h, egui::pos2(field.right() + 8.0, field.center().y));
    assert!(egui::Popup::is_any_open(&h.ctx), "▾ öffnet den Popup-Schieber");
    key(&mut h, Key::Escape);
    h.run_steps(2);
    assert!(st(&h).dialogs.any_open(), "Esc schließt erst nur das Popup");
}

// ---------------------------------------------------------------------- load_prefs

/// Auswahlliste mit Text-Werten ohne `pref_codes`: der gemerkte Text wählt den richtigen Eintrag
/// (vorher immer Eintrag 0); Zahlenwerte wie bisher; Unbekanntes lässt die Vorgabe.
#[test]
fn load_prefs_restores_text_choices() {
    use fp_engine::prefs::PrefValue;
    let mk = || {
        FormDialog::new("t", "T", "x.y", vec![Field::choice_of("mode", "Art", &["A", "B", "C"], vec![json!("a"), json!("b"), json!("c")], 0), Field::choice_of("n", "Zahl", &["1", "2", "3"], vec![json!(10), json!(20), json!(30)], 0)])
            .with_prefs(&[("mode", "Form1TestMode"), ("n", "Form1TestNumber")])
    };
    let mut p = fp_engine::Preferences::new();
    p.set("Form1TestMode", PrefValue::Str("c".into()));
    p.set("Form1TestNumber", PrefValue::Int(20));
    let mut d = mk();
    d.load_prefs(&p);
    assert_eq!(d.get("mode"), json!("c"), "Text-Wert wiederhergestellt");
    assert_eq!(d.get("n"), json!(20), "Zahl wiederhergestellt");
    // OK schreibt zurück, erneutes Laden ergibt dasselbe
    let mut p2 = fp_engine::Preferences::new();
    d.store_prefs(&mut p2);
    let mut d2 = mk();
    d2.load_prefs(&p2);
    assert_eq!((d2.get("mode"), d2.get("n")), (json!("c"), json!(20)), "Rundlauf");
    // unbekannter Text: Vorgabe bleibt
    p.set("Form1TestMode", PrefValue::Str("zzz".into()));
    let mut d3 = mk();
    d3.load_prefs(&p);
    assert_eq!(d3.get("mode"), json!("a"), "unbekannter Wert lässt die Vorgabe");
}

// ---------------------------------------------------------------------- Klemmen beim OK

/// 19105–19108: 5000 bzw. 0 bleiben bis zum OK stehen; OK korrigiert das Feld auf 100 bzw. 1 und
/// lässt den Dialog offen (kein Verlaufsschritt); das zweite OK wendet den geklemmten Wert an.
#[test]
fn marquee_values_clamped_on_ok_dialog_stays_open() {
    let mut h = harness_with(marquee_session());
    for (cmd, typed, clamped) in [("select.expandMarquee", "5000", 100.0), ("select.contractMarquee", "0", 1.0), ("select.borderMarquee", "250", 100.0), ("select.smoothMarquee", "101", 100.0)] {
        let n = undo_count(&h);
        open(&mut h, cmd);
        let r = text_fields(&h)[0].0;
        click(&mut h, r.center());
        key_m(&mut h, Key::A, Modifiers { ctrl: true, command: true, ..Default::default() });
        typ(&mut h, typed);
        key(&mut h, Key::Tab);
        assert_eq!(text_fields(&h)[0].1, typed, "{cmd}: Feld zeigt den getippten Wert bis zum OK");
        let ok = button(&h, &ok_l());
        click(&mut h, ok.center());
        h.run_steps(3);
        assert!(st(&h).dialogs.form.is_some(), "{cmd}: Dialog bleibt offen");
        assert_eq!(form_value(&h, "amount").as_f64(), Some(clamped), "{cmd}: geklemmt");
        assert_eq!(text_fields(&h)[0].1, format!("{clamped}"), "{cmd}: Feld korrigiert");
        assert_eq!(undo_count(&h), n, "{cmd}: noch kein Schritt");
        let ok = button(&h, &ok_l());
        click(&mut h, ok.center());
        h.run_steps(4);
        assert!(st(&h).dialogs.form.is_none(), "{cmd}: zweites OK schließt");
        assert_eq!(undo_count(&h), n + 1, "{cmd}: ein Schritt");
        h.state_mut().state.session.undo().expect("undo");
        h.run_steps(2);
    }
}

/// Mit Enter statt Mausklick: Enter klemmt und lässt offen, zweites Enter bestätigt.
#[test]
fn marquee_enter_clamps_then_confirms() {
    let mut h = harness_with(marquee_session());
    let n = undo_count(&h);
    open(&mut h, "select.expandMarquee");
    assert!(h.ctx.text_edit_focused(), "Startfokus Feld");
    typ(&mut h, "500");
    key(&mut h, Key::Enter);
    h.run_steps(3);
    assert!(st(&h).dialogs.form.is_some(), "Enter mit 500: offen");
    assert_eq!(form_value(&h, "amount").as_f64(), Some(100.0));
    key(&mut h, Key::Enter);
    h.run_steps(4);
    assert!(st(&h).dialogs.form.is_none(), "zweites Enter schließt");
    assert_eq!(undo_count(&h), n + 1);
}

// ---------------------------------------------------------------------- Modaler Dialog

/// Klick auf den abgedunkelten Hintergrund schließt den Dialog nicht (Windows: Signalton); Esc schon.
#[test]
fn click_beside_modal_dialog_keeps_it_open() {
    let mut h = harness_with(marquee_session());
    open(&mut h, "select.expandMarquee");
    let dr = dialog_rect(&h);
    for p in [egui::pos2(dr.left() - 60.0, dr.top() - 40.0), egui::pos2(dr.right() + 80.0, dr.bottom() + 60.0), egui::pos2(700.0, 880.0)] {
        click(&mut h, p);
        assert!(st(&h).dialogs.form.is_some(), "Klick bei {p:?} lässt den Dialog offen");
    }
    key(&mut h, Key::Escape);
    h.run_steps(4);
    assert!(st(&h).dialogs.form.is_none(), "Esc schließt");
}

// ---------------------------------------------------------------------- D26

/// Numerisch transformieren: Startfokus auf der Liste Skalieren/Größe ändern/Drehen, Pfeil nach
/// unten wählt „Größe ändern“, Tab geht weiter zum ersten Feld.
#[test]
fn d26_start_focus_on_list() {
    let mut h = harness_with(rect_session());
    open(&mut h, "modify.transform.numericTransform");
    assert_eq!(role_of(&h), Some(Role::ComboBox), "Startfokus Liste ({:?})", focused(&h));
    assert!(!h.ctx.text_edit_focused());
    key(&mut h, Key::ArrowDown);
    assert_eq!(st(&h).dialogs.numeric_transform.as_ref().map(|d| d.mode), Some(1), "Pfeil wählt „Größe ändern“");
    assert_eq!(role_of(&h), Some(Role::ComboBox), "Fokus bleibt auf der Liste");
    key(&mut h, Key::ArrowUp);
    assert_eq!(st(&h).dialogs.numeric_transform.as_ref().map(|d| d.mode), Some(0), "zurück auf Skalieren");
    let seq = tabs(&mut h, 1);
    assert_eq!(seq[0].0, Role::TextInput, "Tab zum ersten Feld");
    key(&mut h, Key::Escape);
    h.run_steps(4);
}
