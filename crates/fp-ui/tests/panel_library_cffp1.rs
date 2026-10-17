//! FP1-05: Doppelklick in der Dokumentbibliothek wie im Original (re 28 S2): auf das Symbol der
//! Zeile = Symbol bearbeiten (eigene Symbolansicht), auf den Namen = Umbenennen.

use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use fp_ui::app_state::{Action, UiSettings};
use fp_ui::panels::PanelId;
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn harness() -> H {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 500, "height": 300, "background": "#ffffff"})).expect("Dokument");
    s.execute("draw.rectangle", json!({"rect": [20, 20, 140, 90], "fillColor": "#ff0000", "strokeWidth": 0})).expect("Rechteck");
    s.execute("modify.symbol.convertToSymbol", json!({})).expect("Symbol");
    s.execute("pi.symbol.rename", json!({"index": 0, "name": "Qsym"})).expect("Name");
    s.execute("select.none", json!({})).expect("nichts");
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(1800.0, 1130.0)).with_pixels_per_point(1.0).with_max_steps(20).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    h.state_mut().state.push(Action::ShowPanel(PanelId::DocumentLibrary));
    h.run_steps(6);
    h
}

fn texts(shape: &egui::epaint::Shape, out: &mut Vec<(String, egui::Rect)>) {
    match shape {
        egui::epaint::Shape::Text(t) => out.push((t.galley.job.text.clone(), egui::Rect::from_min_size(t.pos, t.galley.size()))),
        egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| texts(s, out)),
        _ => {}
    }
}

fn find(h: &H, label: &str) -> egui::Rect {
    let mut all = Vec::new();
    for cs in &h.output().shapes {
        texts(&cs.shape, &mut all);
    }
    all.into_iter().find(|(t, _)| t == label).map(|(_, r)| r).unwrap_or_else(|| panic!("„{label}“ nicht gezeichnet"))
}

fn double_click(h: &mut H, p: Pos2) {
    // Abstand zu früheren Klicks (sonst zählt egui einen Dreifachklick)
    h.run_steps(60);
    h.event(Event::PointerMoved(p));
    h.run_steps(1);
    for _ in 0..2 {
        for down in [true, false] {
            h.event(Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: down, modifiers: Modifiers::NONE });
            h.run_steps(1);
        }
    }
    h.run_steps(4);
}

fn editing(h: &H) -> bool {
    h.state().state.session.active().is_some_and(|d| d.symbol_edit.is_some())
}

#[test]
fn fp1_05_double_click_icon_edits_name_renames() {
    let mut h = harness();
    let name = find(&h, "Qsym");
    // Name: Umbenennen-Feld, keine Symbolbearbeitung
    double_click(&mut h, name.center());
    assert!(!editing(&h), "Doppelklick auf den Namen bearbeitet nicht");
    assert!(h.ctx.text_edit_focused(), "Umbenennen-Feld offen");
    h.event(Event::Key { key: egui::Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE });
    h.run_steps(4);
    assert!(!h.ctx.text_edit_focused());
    // Symbol der Zeile (links vom Namen): Symbolbearbeitung
    let icon = Pos2::new(name.left() - 13.0, name.center().y);
    double_click(&mut h, icon);
    assert!(editing(&h), "Doppelklick auf das Symbol der Zeile startet die Symbolbearbeitung");
    assert!(!h.ctx.text_edit_focused(), "kein Umbenennen-Feld");
}
