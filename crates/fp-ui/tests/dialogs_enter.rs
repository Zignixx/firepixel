//! HG-Befund: Eingabe bestätigt einen Dialog auch, wenn ein Zahlen-/Textfeld den Fokus hat, und
//! übernimmt vorher den getippten Wert (wie im Original: Eingabe = Standardknopf); Esc bricht ab.
//! Ereignisfolgen wie egui-winit unter Windows (Strg = ctrl + command, Text als `Event::Text`).

use egui::{Event, Key, Modifiers};
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use fp_engine::Session;
use fp_ui::app_state::UiSettings;
use fp_ui::dialogs::{dialog_rect, last_dialog_id};
use fp_ui::FirepixelApp;
use serde_json::json;
use std::sync::Arc;

type H = Harness<'static, FirepixelApp>;

fn session() -> Session {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [20, 20, 120, 80]})).expect("rect");
    s.execute("draw.text", json!({"text": "Hallo Wlet", "anchor": [30, 150], "size": 20})).expect("text");
    s.execute("draw.ellipse", json!({"rect": [150, 20, 250, 90]})).expect("ellipse");
    s.execute("modify.symbol.convertToSymbol", json!({"name": "Sym"})).expect("symbol");
    s.execute("draw.rectangle", json!({"rect": [160, 120, 260, 180]})).expect("rect2");
    s.execute("pi.style.new", json!({"name": "Stil 1"})).expect("style");
    s
}

fn harness() -> H {
    let ctx = egui::Context::default();
    let mut a = FirepixelApp::with_session(&ctx, session(), UiSettings::default(), false);
    a.state.sync_render = true;
    a.state.os_clipboard = false;
    let tmp = std::env::temp_dir().join("fp-ibd-dialogs-enter");
    let _ = std::fs::create_dir_all(&tmp);
    a.state.file_override = Some(fp_ui::files::FilePickOverride { open: vec![tmp.join("nicht-vorhanden.png")], save: Some(tmp.join("ausgabe.png")) });
    a.state.dialogs.panel_sel.styles = vec!["Stil 1".into()];
    a.state.dialogs.panel_sel.symbol = Some(0);
    let mut h = Harness::builder().with_size(egui::vec2(1500.0, 950.0)).with_pixels_per_point(1.0).with_max_steps(8).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), a);
    h.run_steps(3);
    h
}

fn ctrl() -> Modifiers {
    Modifiers { ctrl: true, command: true, ..Default::default() }
}

fn ev(h: &mut H, e: Event) {
    h.event(e);
    h.run_steps(1);
}

fn key(h: &mut H, k: Key, m: Modifiers) {
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: m });
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: false, repeat: false, modifiers: m });
}

/// Erstes Textfeld im offenen Dialog fokussieren (Klick); `false`, wenn der Dialog keines hat.
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

/// Dialoge mit Standardknopf (wie `dialogs_open.rs`, ohne Listen-Dialoge ohne Eingabe-OK).
const DIALOG_COMMANDS: &[&str] = &[
    "modify.canvas.canvasSize",
    "modify.canvas.imageSize",
    "modify.transform.numericTransform",
    "filter.blur.gaussian",
    "filter.blur.motion",
    "filter.sharpen.unsharpMask",
    "filter.noise.addNoise",
    "filter.adjustColor.brightnessContrast",
    "filter.adjustColor.hueSaturation",
    "select.feather",
    "select.expandMarquee",
    "select.contractMarquee",
    "select.borderMarquee",
    "select.smoothMarquee",
    "modify.alterPath.simplify",
    "modify.alterPath.insetPath",
    "modify.alterPath.expandStroke",
    "text.size.other",
    "modify.symbol.tweenInstances",
    "modify.symbol.convertToSymbol",
    "modify.animation.animateSelection",
    "modify.canvas.canvasColor",
    "view.grid.editGrid",
    "view.guides.editGuides",
    "edit.insert.layer",
    "edit.insert.state",
    "panel.layers.newLayer",
    "panel.layers.duplicate",
    "panel.states.add",
    "panel.pages.rename",
    "panel.styles.rename",
    "panel.library.properties",
    "panel.optimize.optimizeToSize",
];

#[test]
fn enter_confirms_every_dialog_even_with_focused_field() {
    let mut no_field = Vec::new();
    for name in DIALOG_COMMANDS {
        let mut h = harness();
        h.state_mut().state.dialogs.filter_vectors_ok = true; // TW-21: Rückfrage wie nach OK überspringen
        // Q-20: Rückfrage „Automatisch neue Status hinzufügen?“ abgeschaltet (gilt als OK)
        h.state_mut().state.session.prefs.set(fp_ui::dialogs::warning::ANIMATE_STATES_PREF, fp_engine::PrefValue::Bool(true));
        h.state_mut().state.run(name, json!({}));
        h.run_steps(3);
        assert!(h.state().state.dialogs.any_open(), "{name}: Dialog offen");
        if !focus_first_field(&mut h) {
            no_field.push(*name);
        }
        key(&mut h, Key::Enter, Modifiers::NONE);
        h.run_steps(4);
        assert!(!h.state().state.dialogs.any_open(), "{name}: Eingabe bestätigt den Dialog nicht");
    }
    assert!(no_field.len() < DIALOG_COMMANDS.len() / 2, "zu wenige Dialoge mit Feld geprüft: {no_field:?}");
}

#[test]
fn esc_cancels_every_dialog_even_with_focused_field() {
    for name in DIALOG_COMMANDS {
        let mut h = harness();
        let d0 = Arc::clone(&h.state().state.session.active().expect("d").doc);
        h.state_mut().state.run(name, json!({}));
        h.run_steps(3);
        focus_first_field(&mut h);
        ev(&mut h, Event::Text("7".into()));
        key(&mut h, Key::Escape, Modifiers::NONE);
        h.run_steps(4);
        assert!(!h.state().state.dialogs.any_open(), "{name}: Esc schließt nicht");
        assert!(*h.state().state.session.active().expect("d").doc == *d0, "{name}: Esc hat das Dokument geändert");
    }
}

/// Leinwandgröße: Breite tippen, Eingabe → der getippte Wert gilt (ohne Fokuswechsel vorher).
#[test]
fn enter_takes_typed_value_canvas_size() {
    let mut h = harness();
    h.state_mut().state.run("modify.canvas.canvasSize", json!({}));
    h.run_steps(3);
    assert!(focus_first_field(&mut h), "Breitenfeld fokussiert");
    key(&mut h, Key::A, ctrl());
    ev(&mut h, Event::Text("500".into()));
    key(&mut h, Key::Enter, Modifiers::NONE);
    h.run_steps(4);
    assert!(!h.state().state.dialogs.any_open(), "Eingabe schließt");
    let d = h.state().state.session.active().expect("d");
    assert_eq!(d.doc.pages[d.page].canvas.width, 500.0, "getippte Breite übernommen");
    // Bildgröße ebenso
    h.state_mut().state.run("modify.canvas.imageSize", json!({}));
    h.run_steps(3);
    assert!(focus_first_field(&mut h));
    key(&mut h, Key::A, ctrl());
    ev(&mut h, Event::Text("250".into()));
    key(&mut h, Key::Enter, Modifiers::NONE);
    h.run_steps(4);
    let d = h.state().state.session.active().expect("d");
    assert_eq!(d.doc.pages[d.page].canvas.width, 250.0, "Bildgröße: getippte Breite übernommen");
}

/// Mehrzeiliges Textfeld (Texteditor): Eingabe bricht die Zeile um, schließt nicht.
#[test]
fn enter_in_multiline_field_inserts_newline() {
    let mut h = harness();
    let text_id = {
        let d = h.state().state.session.active().expect("d");
        fp_engine::docops::own_layers(&d.doc, d.page).into_iter().flat_map(|(_, l)| l.cels.first().map(|c| c.elements.clone()).unwrap_or_default()).find(|e| matches!(e.kind, fp_model::ElementKind::Text(_))).map(|e| e.id.0).expect("Text")
    };
    h.state_mut().state.session.execute("select.set", json!({"ids": [text_id]})).expect("sel");
    h.state_mut().state.run("text.editor", json!({}));
    h.run_steps(3);
    assert!(h.state().state.dialogs.any_open(), "Texteditor offen");
    let nodes: Vec<_> = h.get_all(egui_kittest::kittest::By::new().role(egui::accesskit::Role::MultilineTextInput)).collect();
    if let Some(n) = nodes.first() {
        n.click();
        drop(nodes);
        h.run_steps(3);
        key(&mut h, Key::Enter, Modifiers::NONE);
        h.run_steps(3);
        assert!(h.state().state.dialogs.any_open(), "Eingabe im mehrzeiligen Feld schließt nicht");
    }
}
