//! „Strich bearbeiten“ ohne Auswahl: Pinsel der Werkzeugvorgabe (`ToolOptions::new_brush`).
//! Dialog öffnen, Werte vielfach ändern, Abbrechen = unverändert, OK = Vorgabepinsel; neu
//! gezeichnete Rechtecke und Stiftpfade tragen ihn; Strich-Felder des Inspektors schreiben hinein
//! (AGENTEN_REGELN §4b).

use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use fp_engine::tools::ToolId;
use fp_engine::{Session, ToolEvent, ToolKey};
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::FirepixelApp;
use serde_json::{json, Value};
use std::sync::Arc;

fn app() -> FirepixelApp {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200})).expect("doc");
    s.execute("select.none", json!({})).expect("none");
    let ctx = egui::Context::default();
    let mut a = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    a.state.sync_render = true;
    a
}

fn harness(a: FirepixelApp) -> Harness<'static, FirepixelApp> {
    Harness::builder().with_size(egui::vec2(1400.0, 900.0)).with_pixels_per_point(1.0).with_max_steps(16).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), a)
}

fn st<'a>(h: &'a Harness<'_, FirepixelApp>) -> &'a AppState {
    &h.state().state
}

fn doc(st: &AppState) -> Arc<fp_model::Document> {
    Arc::clone(&st.session.active().expect("doc").doc)
}

fn undo_count(st: &AppState) -> usize {
    st.session.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

fn set_field(h: &mut Harness<'_, FirepixelApp>, key: &str, v: Value) {
    let f = h.state_mut().state.dialogs.form.as_mut().expect("Dialog offen");
    f.field_mut(key).unwrap_or_else(|| panic!("Feld {key}")).value = v;
    h.run_steps(1);
}

fn click_button(h: &mut Harness<'_, FirepixelApp>, label: &str) {
    h.run_steps(1);
    let b: Vec<_> = h.get_all(egui_kittest::kittest::By::new().label(label)).collect();
    b.last().unwrap_or_else(|| panic!("Knopf {label}")).click();
    drop(b);
    h.run_steps(3);
}

fn open(h: &mut Harness<'_, FirepixelApp>) {
    h.state_mut().state.dialogs.open_stroke_options(true);
    h.run_steps(2);
    let f = st(h).dialogs.form.as_ref().expect("Strich bearbeiten offen");
    assert_eq!(f.command, "pi.tool.brush", "ohne Auswahl wirkt der Dialog auf die Werkzeugvorgabe");
}

fn tool_brush(st: &AppState) -> fp_model::Brush {
    fp_engine::commands::panel::tool_brush(&st.session.tools.options)
}

/// Pinsel des zuletzt gezeichneten (ausgewählten) Objekts.
fn new_object_brush(st: &AppState) -> fp_model::Brush {
    let d = st.session.active().expect("doc");
    let id = *d.selection.elements.first().expect("neues Objekt ausgewählt");
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, id).expect("Element");
    fp_engine::docops::path_attrs_of(&e).and_then(|a| a.brush.clone()).expect("Pinsel")
}

fn events(h: &mut Harness<'_, FirepixelApp>, list: &[ToolEvent]) {
    for e in list {
        h.state_mut().state.session.tool_event(e).expect("Ereignis");
    }
    h.run_steps(1);
}

#[test]
fn tool_brush_dialog_cancel_ok_and_drawing() {
    let mut h = harness(app());
    h.state_mut().state.session.set_tool(ToolId::Rectangle).expect("Rechteck");
    h.run_steps(2);
    let (d0, u0) = (doc(st(&h)), undo_count(st(&h)));
    // Abbrechen nach 50 Änderungen: Vorgabe unverändert, Dokument unberührt
    open(&mut h);
    for k in 0..50 {
        set_field(&mut h, "diameter", json!(2 + k % 9));
        set_field(&mut h, "_PZ", json!(k % 4 * 10));
    }
    click_button(&mut h, "Abbrechen");
    assert!(st(&h).dialogs.form.is_none());
    assert!(st(&h).session.tools.options.new_brush.is_none(), "Abbrechen schreibt nichts");
    assert!(Arc::ptr_eq(&d0, &doc(st(&h))));
    // OK nach vielen Änderungen: nur die letzten Werte zählen
    open(&mut h);
    for k in 0..50 {
        set_field(&mut h, "diameter", json!(2 + k % 9));
        set_field(&mut h, "angle", json!(k));
    }
    set_field(&mut h, "diameter", json!(9));
    set_field(&mut h, "softness", json!(30));
    set_field(&mut h, "angle", json!(45));
    set_field(&mut h, "_PZ", json!(20));
    click_button(&mut h, "OK");
    assert!(st(&h).dialogs.form.is_none());
    let b = tool_brush(st(&h));
    assert_eq!((b.diameter, b.softness, b.angle, b.sensitivity(b'P', b'Z')), (9, 300, 45, 200));
    assert!(Arc::ptr_eq(&d0, &doc(st(&h))), "Werkzeugvorgabe ist kein Dokumentschritt");
    assert_eq!(undo_count(st(&h)), u0);
    // erneut öffnen zeigt die Vorgabe; Abbrechen lässt sie stehen
    open(&mut h);
    let f = st(&h).dialogs.form.as_ref().expect("offen");
    assert_eq!(f.get("diameter").as_f64(), Some(9.0));
    assert_eq!(f.get("_PZ").as_f64(), Some(20.0));
    set_field(&mut h, "diameter", json!(33));
    click_button(&mut h, "Abbrechen");
    assert_eq!(tool_brush(st(&h)).diameter, 9);
    // Rechteck aufziehen: neues Objekt trägt den Pinsel, ein Verlaufsschritt
    events(&mut h, &[ToolEvent::down(20.0, 20.0), ToolEvent::drag(60.0, 50.0), ToolEvent::up(100.0, 80.0)]);
    let nb = new_object_brush(st(&h));
    assert_eq!((nb.diameter, nb.softness, nb.angle, nb.sensitivity(b'P', b'Z')), (9, 300, 45, 200), "Rechteck");
    assert_eq!(undo_count(st(&h)), u0 + 1);
    // Stiftpfad
    h.state_mut().state.session.set_tool(ToolId::Pen).expect("Stift");
    h.state_mut().state.session.execute("select.none", json!({})).expect("none");
    h.run_steps(1);
    events(&mut h, &[ToolEvent::down(150.0, 30.0), ToolEvent::up(150.0, 30.0), ToolEvent::down(250.0, 90.0), ToolEvent::up(250.0, 90.0), ToolEvent::key(ToolKey::Enter)]);
    let nb = new_object_brush(st(&h));
    assert_eq!((nb.diameter, nb.angle), (9, 45), "Stift");
}

#[test]
fn inspector_stroke_fields_write_tool_brush() {
    let mut h = harness(app());
    h.state_mut().state.session.set_tool(ToolId::Rectangle).expect("Rechteck");
    h.run_steps(2);
    // ohne Vorgabepinsel bleibt es bei der Strichvorgabe
    fp_ui::inspector::obj_quads::sync_tool_brush(&mut h.state_mut().state, &json!({"width": 5}));
    h.run_steps(2);
    assert!(st(&h).session.tools.options.new_brush.is_none());
    h.state_mut().state.run("pi.tool.brush", json!({"angle": 60, "diameter": 4}));
    h.run_steps(2);
    // Stärke 50× ändern: der Vorgabepinsel folgt, die übrigen Pinselwerte bleiben
    for k in 0..50 {
        fp_ui::inspector::obj_quads::sync_tool_brush(&mut h.state_mut().state, &json!({"width": 1 + k % 7, "softness": k % 100}));
        h.run_steps(1);
    }
    fp_ui::inspector::obj_quads::sync_tool_brush(&mut h.state_mut().state, &json!({"width": 12, "softness": 10}));
    h.run_steps(2);
    let b = tool_brush(st(&h));
    assert_eq!((b.diameter, b.softness, b.angle), (12, 100, 60));
    // Strichvorgabe wechseln: Pinselart wird ersetzt, Stärke bleibt
    fp_ui::inspector::obj_quads::sync_tool_brush(&mut h.state_mut().state, &json!({"preset": "Kohle/Pastell"}));
    h.run_steps(2);
    let b = tool_brush(st(&h));
    assert_eq!((b.category.as_str(), b.diameter), ("bc_Charcoal", 12));
    // „kein Strich“ entfernt den Vorgabepinsel
    fp_ui::inspector::obj_quads::sync_tool_brush(&mut h.state_mut().state, &json!({"none": true}));
    h.run_steps(2);
    assert!(st(&h).session.tools.options.new_brush.is_none());
}
