//! F21 (Tweak-Liste 16.10., Lauf 8): Hinweis bei Freiform/Umformen/Pfadausbesserung auf einer
//! Grundform (TW-80). TW-78 steht in `inspector_fv9.rs`, TW-69/85 in `f18.rs`.

use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use fp_engine::tools::ToolId;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn harness(setup: impl FnOnce(&mut fp_engine::Session)) -> H {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 500, "height": 300, "background": "#ffffff"})).expect("Dokument");
    setup(&mut s);
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(1800.0, 1130.0)).with_pixels_per_point(1.0).with_max_steps(20).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    h
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn screen(h: &H, x: f32, y: f32) -> Pos2 {
    let s = st(h);
    let d = s.session.active().expect("Dokument");
    let area = s.canvases.get(&d.id).and_then(|c| c.area).expect("Leinwand");
    let (sx, sy) = d.view.doc_to_screen(fp_geom::Point::new(x, y));
    Pos2::new(area.min.x + sx as f32, area.min.y + sy as f32)
}

fn ev(h: &mut H, e: Event) {
    h.event(e);
    h.run_steps(1);
}

fn drag(h: &mut H, a: Pos2, b: Pos2) {
    ev(h, Event::PointerMoved(a));
    ev(h, Event::PointerButton { pos: a, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    for i in 1..=6 {
        ev(h, Event::PointerMoved(a + (b - a) * (i as f32 / 6.0)));
    }
    ev(h, Event::PointerButton { pos: b, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(3);
}

fn undo_count(h: &H) -> usize {
    st(h).session.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

fn element(h: &H, id: u64) -> std::sync::Arc<fp_model::Element> {
    let d = st(h).session.active().expect("Dokument");
    fp_engine::docops::element_on_page(&d.doc, d.page, fp_model::ElementId(id)).expect("Objekt")
}

fn warning_key(h: &H) -> Option<String> {
    st(h).dialogs.warning.as_ref().map(|w| w.warning.key.clone())
}

/// Rechteck-Grundform gewählt; Freiform, Umformen, Pfadausbesserung: Meldung des Originals (OK,
/// Nicht mehr anzeigen), keine Änderung, kein Verlaufsschritt. Esc schließt.
#[test]
fn tw80_path_tools_on_rect_primitive_warn_and_do_nothing() {
    let mut id = 0;
    let mut h = harness(|s| {
        let v = s.execute("draw.rectangle", json!({"rect": [100, 100, 200, 180], "fillColor": "#999999", "strokeWidth": 0})).expect("R");
        id = v.value["id"].as_u64().expect("id");
        s.execute("select.set", json!({"ids": [id]})).expect("Auswahl");
    });
    let before = element(&h, id);
    assert!(matches!(&before.kind, fp_model::ElementKind::Path(p) if p.primitive.is_some()), "Grundform");
    let n0 = undo_count(&h);
    for tool in [ToolId::Freeform, ToolId::Reshape, ToolId::PathScrubAdd, ToolId::PathScrubSubtract] {
        h.state_mut().state.session.set_tool(tool).expect("Werkzeug");
        h.run_steps(2);
        let (a, b) = (screen(&h, 150.0, 140.0), screen(&h, 190.0, 160.0));
        drag(&mut h, a, b);
        assert_eq!(warning_key(&h).as_deref(), Some(fp_ui::dialogs::warning::PATH_TOOLS_PRIMITIVE), "{tool:?}: Meldung offen");
        let w = st(&h).dialogs.warning.clone().expect("Meldung");
        assert!(w.warning.choices.is_empty() && w.warning.pref.is_some(), "nur OK und „Nicht mehr anzeigen“");
        assert!(w.warning.text.contains("Pfadausbesserungs-Werkzeuge"), "{}", w.warning.text);
        assert_eq!(element(&h, id), before, "{tool:?}: Grundform unverändert");
        assert_eq!(undo_count(&h), n0, "{tool:?}: kein Verlaufsschritt");
        ev(&mut h, Event::Key { key: egui::Key::Escape, physical_key: Some(egui::Key::Escape), pressed: true, repeat: false, modifiers: Modifiers::NONE });
        ev(&mut h, Event::Key { key: egui::Key::Escape, physical_key: Some(egui::Key::Escape), pressed: false, repeat: false, modifiers: Modifiers::NONE });
        h.run_steps(2);
        assert!(warning_key(&h).is_none(), "{tool:?}: Esc schließt");
    }
    // „Nicht mehr anzeigen“: danach still, weiterhin ohne Änderung
    h.state_mut().state.session.prefs.set(fp_ui::dialogs::warning::PATH_TOOLS_PRIMITIVE_PREF, fp_engine::prefs::PrefValue::Bool(true));
    h.state_mut().state.session.set_tool(ToolId::Freeform).expect("Freiform");
    let (a, b) = (screen(&h, 150.0, 140.0), screen(&h, 190.0, 160.0));
    drag(&mut h, a, b);
    assert!(warning_key(&h).is_none(), "abgeschaltet");
    assert_eq!(element(&h, id), before);
}

/// Ein gewöhnlicher Pfad (Ellipse) bekommt keine Meldung, Freiform arbeitet wie bisher.
#[test]
fn tw80_freeform_on_path_still_works() {
    let mut id = 0;
    let mut h = harness(|s| {
        let v = s.execute("draw.ellipse", json!({"rect": [100, 100, 200, 180], "fillColor": "#999999", "strokeWidth": 0})).expect("E");
        id = v.value["id"].as_u64().expect("id");
        s.execute("select.set", json!({"ids": [id]})).expect("Auswahl");
    });
    let before = element(&h, id);
    h.state_mut().state.session.set_tool(ToolId::Freeform).expect("Freiform");
    h.run_steps(2);
    let (a, b) = (screen(&h, 200.0, 140.0), screen(&h, 240.0, 140.0));
    drag(&mut h, a, b);
    assert!(warning_key(&h).is_none(), "keine Meldung");
    assert_ne!(element(&h, id), before, "Pfad gezogen");
}

/// TW-84 (re 37 §4): Spalte „Name“ der Symboleigenschaften bleibt 35 px (34 px Zelle + 1 px
/// Trennlinie); ein längerer Kopftext wird abgeschnitten („Nam“), die Spalte wird nicht breiter.
#[test]
fn tw84_symbol_properties_name_column_stays_35_px() {
    assert_eq!(fp_ui::panels::misc::SYMPROPS_NAME_W, 35.0);
}
