//! F13 (docs\berichte\m2.md): Q-19 „Instanzen tweenen“ Vorgabe 10, zuletzt bestätigter Wert gilt
//! bis Programmende; Q-20 Rückfrage „Automatisch neue Status hinzufügen?“ nach „Auswahl animieren“
//! (OK hängt Status an, Abbrechen nicht, „Nicht mehr anzeigen“ zurücksetzbar).

use egui::{Event, Key, Modifiers};
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use fp_ui::app_state::{Action, AppState, UiSettings};
use fp_ui::dialogs::warning::{answer_animate_states, ANIMATE_STATES, ANIMATE_STATES_PREF};
use fp_ui::dialogs::{dialog_rect, last_dialog_id};
use fp_ui::FirepixelApp;
use serde_json::{json, Value};

type H = Harness<'static, FirepixelApp>;

/// Zwei Instanzen desselben Symbols, beide gewählt.
fn tween_session() -> fp_engine::Session {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 400, "height": 200, "background": "#ffffff"})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [10, 10, 40, 40]})).expect("rect");
    let a = s.execute("modify.symbol.convertToSymbol", json!({"name": "S"})).expect("symbol").value["id"].as_u64().expect("id");
    s.execute("edit.duplicate", json!({})).expect("dup");
    let b = s.active().expect("doc").selection.elements[0].0;
    s.execute("edit.move", json!({"dx": 300, "dy": 0})).expect("move");
    s.execute("select.set", json!({"ids": [a, b]})).expect("sel");
    s
}

fn harness(s: fp_engine::Session) -> H {
    let ctx = egui::Context::default();
    let mut a = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    a.state.sync_render = true;
    a.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).with_pixels_per_point(1.0).with_max_steps(8).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), a);
    h.run_steps(3);
    h
}

fn ev(h: &mut H, e: Event) {
    h.event(e);
    h.run_steps(1);
}

fn key(h: &mut H, k: Key, m: Modifiers) {
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: m });
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: false, repeat: false, modifiers: m });
    h.run_steps(3);
}

fn focus_first_field(h: &mut H) -> bool {
    let Some(id) = last_dialog_id(&h.ctx) else { return false };
    let Some(dr) = dialog_rect(&h.ctx, &id) else { return false };
    let nodes: Vec<_> = h.get_all(egui_kittest::kittest::By::new().role(egui::accesskit::Role::TextInput)).filter(|n| dr.contains(n.rect().center())).collect();
    let Some(n) = nodes.first() else { return false };
    n.click();
    drop(nodes);
    h.run_steps(3);
    h.ctx.text_edit_focused()
}

fn open_tween(h: &mut H) -> f64 {
    h.state_mut().state.run("modify.symbol.tweenInstances", json!({}));
    h.run_steps(4);
    let f = h.state().state.dialogs.form.as_ref().expect("Dialog „Instanzen tweenen“");
    f.get("steps").as_f64().expect("Schritte")
}

fn selected_len(h: &H) -> usize {
    h.state().state.session.active().map(|d| d.selection.elements.len()).unwrap_or(0)
}

/// Q-19: erster Aufruf 10; nach OK mit 3 zeigt der nächste Aufruf 3; Abbrechen merkt nichts.
#[test]
fn tween_steps_default_and_remembered() {
    let mut h = harness(tween_session());
    assert_eq!(open_tween(&mut h), 10.0, "Vorgabe ab Werk");
    assert!(focus_first_field(&mut h), "Feld Schritte");
    key(&mut h, Key::A, Modifiers { ctrl: true, command: true, ..Default::default() });
    ev(&mut h, Event::Text("3".into()));
    key(&mut h, Key::Enter, Modifiers::NONE);
    h.run_steps(3);
    assert!(h.state().state.dialogs.form.is_none(), "OK");
    assert_eq!(selected_len(&h), 5, "3 Schritte: zwei Instanzen + 3 Zwischeninstanzen");
    h.state_mut().state.session.execute("select.set", json!({"ids": []})).expect("leer");
    let ids: Vec<u64> = {
        let d = h.state().state.session.active().expect("doc");
        fp_engine::docops::selectable_ids(&d.doc, d.page, d.frame).iter().map(|i| i.0).take(2).collect()
    };
    h.state_mut().state.session.execute("select.set", json!({"ids": ids})).expect("sel");
    assert_eq!(open_tween(&mut h), 3.0, "zuletzt bestätigter Wert");
    key(&mut h, Key::Escape, Modifiers::NONE);
    assert!(h.state().state.dialogs.form.is_none(), "Abbrechen");
    assert_eq!(open_tween(&mut h), 3.0, "Abbrechen ändert den gemerkten Wert nicht");
    // nicht in den Voreinstellungen
    assert!(!format!("{:?}", h.state().state.session.prefs).contains("Tween"), "kein Einstellungsschlüssel");
}

fn state() -> (AppState, egui::Context) {
    fp_engine::shell::disable();
    fp_i18n::profile::use_test_profile();
    let session = fp_ui::ui_commands::new_session(fp_engine::Preferences::new());
    let mut st = AppState::new(session, UiSettings::default(), None);
    st.sync_render = true;
    st.os_clipboard = false;
    st.session.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("Dokument");
    (st, egui::Context::default())
}

fn run(st: &mut AppState, ctx: &egui::Context, name: &str, p: Value) {
    st.push(Action::Run(name.into(), p));
    st.process_actions(ctx);
}

fn frames(st: &AppState) -> usize {
    st.session.active().map(|d| d.frame_count()).unwrap_or(0)
}

fn animate_params() -> Value {
    json!({"frames": 5, "offsetX": 72, "offsetY": 0, "scale": 100, "startOpacity": 100, "endOpacity": 100, "rotation": 0})
}

/// Q-20: Rückfrage nach dem Dialog; die Animation steht schon vor der Antwort (TW-53), Abbrechen
/// lässt sie ohne neue Status, OK hängt sie an; „Nicht mehr anzeigen“ gilt als OK,
/// „Warndialoge zurücksetzen“ schaltet die Frage wieder ein.
#[test]
fn animate_asks_for_new_states() {
    fp_i18n::with_language("de", || {
        let (mut st, ctx) = state();
        st.session.execute("draw.rectangle", json!({"rect": [20, 20, 60, 60]})).expect("Rechteck");
        run(&mut st, &ctx, "modify.animation.animateSelection", animate_params());
        let w = st.dialogs.warning.clone().expect("Rückfrage offen");
        assert_eq!(w.warning.key, ANIMATE_STATES);
        assert_eq!(w.warning.text, "Die Animation dieses Symbols reicht über den letzten Status im Dokument hinaus. Automatisch neue Status hinzufügen?");
        assert_eq!(w.warning.choices.len(), 1, "OK, dazu Abbrechen");
        assert_eq!(w.warning.pref.as_deref(), Some(ANIMATE_STATES_PREF), "mit „Nicht mehr anzeigen“");
        assert!(st.session.active().is_some_and(|d| d.doc.symbols.len() == 1), "TW-53: Animation schon vor der Antwort angelegt");
        assert_eq!(frames(&st), 1, "noch keine neuen Status");

        // Abbrechen (wie Knopf/Esc: Ergebnis der Rückfrage ohne Wahl)
        st.dialogs.warning = None;
        answer_animate_states(&mut st, false);
        st.process_actions(&ctx);
        assert_eq!(frames(&st), 1, "Abbrechen: keine neuen Status");
        let d = st.session.active().expect("doc");
        let id = d.selection.elements[0];
        let e = fp_engine::docops::element_on_page(&d.doc, d.page, id).expect("Instanz");
        assert!(matches!(e.kind, fp_model::ElementKind::Instance(_)), "Animation trotzdem angelegt");
        assert_eq!(e.common.name, None, "Instanz ohne Namen");
        assert_eq!(d.doc.symbols.last().map(|y| y.name.as_str()), Some("Symbol"));

        // OK
        st.session.execute("draw.rectangle", json!({"rect": [100, 20, 140, 60]})).expect("Rechteck");
        run(&mut st, &ctx, "modify.animation.animateSelection", animate_params());
        assert!(st.dialogs.warning.is_some(), "wieder gefragt");
        st.dialogs.warning = None;
        answer_animate_states(&mut st, true);
        st.process_actions(&ctx);
        assert_eq!(frames(&st), 5, "OK: Status angehängt");
        let d = st.session.active().expect("doc");
        assert_eq!(d.doc.symbols.len(), 2, "OK legt kein zweites Symbol an");

        // genug Status: keine Frage
        st.session.execute("draw.rectangle", json!({"rect": [20, 100, 60, 140]})).expect("Rechteck");
        run(&mut st, &ctx, "modify.animation.animateSelection", animate_params());
        assert!(st.dialogs.warning.is_none(), "5 Status vorhanden: keine Frage");

        // „Nicht mehr anzeigen“: ohne Frage wie OK; zurücksetzbar
        st.session.set_warning_disabled(&w.warning, true);
        st.session.execute("draw.rectangle", json!({"rect": [100, 100, 140, 140]})).expect("Rechteck");
        run(&mut st, &ctx, "modify.animation.animateSelection", json!({"frames": 8, "offsetX": 72}));
        st.process_actions(&ctx);
        assert!(st.dialogs.warning.is_none(), "abgeschaltet");
        assert_eq!(frames(&st), 8, "abgeschaltet gilt OK");
        run(&mut st, &ctx, "ui.resetWarnings", json!({}));
        st.session.execute("draw.rectangle", json!({"rect": [200, 100, 240, 140]})).expect("Rechteck");
        run(&mut st, &ctx, "modify.animation.animateSelection", json!({"frames": 9, "offsetX": 72}));
        assert!(st.dialogs.warning.is_some(), "nach „Warndialoge zurücksetzen“ wieder da");
    });
}
