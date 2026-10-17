//! HG-Befund: Farbfeld, Stil und Symbol aus den Bedienfeldern auf Objekt bzw. Leinwand ziehen,
//! mit realistischen Ziehfolgen über viele Bilder (Helfer wie `gesamt_alltag.rs`):
//! Farbfeld auf Objekt → Füllung (Umschalt → Strich), Stil auf Objekt → Stil angewendet, Symbol
//! auf die Leinwand → Instanz an der Ablagestelle. Ablegen neben dem Objekt bzw. außerhalb der
//! Leinwand und Esc ändern nichts; jedes Ablegen ist ein Verlaufsschritt, Rückgängig exakt.

use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn harness() -> (H, u64) {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 400, "height": 300, "background": "#ffffff"})).expect("doc");
    // Stil „Rot“ aus einem roten Rechteck mit Strich, Symbol „Stern“ aus einer Ellipse
    s.execute("draw.rectangle", json!({"rect": [300, 200, 380, 280]})).expect("r");
    s.execute("pi.fill", json!({"color": "#ff0000"})).expect("f");
    s.execute("pi.style.new", json!({"name": "Rot"})).expect("style");
    s.execute("edit.clear", json!({})).expect("weg");
    s.execute("draw.ellipse", json!({"rect": [300, 20, 360, 80]})).expect("e");
    s.execute("modify.symbol.convertToSymbol", json!({"name": "Stern"})).expect("sym");
    s.execute("edit.clear", json!({})).expect("weg");
    s.execute("draw.rectangle", json!({"rect": [50, 50, 150, 120]})).expect("ziel");
    s.execute("pi.fill", json!({"color": "#00ff00"})).expect("grün");
    let target = s.active().map(|d| d.selection.elements[0].0).expect("ziel");
    s.execute("select.none", json!({})).expect("none");
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(1600.0, 1000.0)).with_pixels_per_point(1.0).with_max_steps(20).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    (h, target)
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn ev(h: &mut H, e: Event) {
    let m = match &e {
        Event::Key { modifiers, .. } | Event::PointerButton { modifiers, .. } => Some(*modifiers),
        _ => None,
    };
    if let Some(m) = m {
        h.event(Event::ModifiersChanged(m));
    }
    h.event(e);
    h.run_steps(1);
}

fn press(h: &mut H, p: Pos2, down: bool, m: Modifiers) {
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: down, modifiers: m });
}

fn drag(h: &mut H, a: Pos2, b: Pos2, frames: usize, m: Modifiers) {
    ev(h, Event::PointerMoved(a));
    press(h, a, true, m);
    for i in 1..=frames {
        let t = i as f32 / frames as f32;
        ev(h, Event::PointerMoved(a + (b - a) * t));
    }
    press(h, b, false, m);
    ev(h, Event::ModifiersChanged(Modifiers::NONE));
    h.run_steps(3);
}

fn click(h: &mut H, p: Pos2) {
    ev(h, Event::PointerMoved(p));
    press(h, p, true, Modifiers::NONE);
    press(h, p, false, Modifiers::NONE);
    h.run_steps(2);
}

fn screen(h: &H, x: f32, y: f32) -> Pos2 {
    let s = st(h);
    let d = s.session.active().expect("Dokument");
    let area = s.canvases.get(&d.id).and_then(|c| c.area).expect("Leinwand");
    let (sx, sy) = d.view.doc_to_screen(fp_geom::Point::new(x, y));
    Pos2::new(area.min.x + sx as f32, area.min.y + sy as f32)
}

fn texts(shape: &egui::epaint::Shape, out: &mut Vec<(String, egui::Rect)>) {
    match shape {
        egui::epaint::Shape::Text(t) => out.push((t.galley.job.text.clone(), egui::Rect::from_min_size(t.pos, t.galley.size()))),
        egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| texts(s, out)),
        _ => {}
    }
}

/// Registerkarte bzw. Zeile mit der Beschriftung `label` (rechteste Fundstelle = Bedienfelder).
fn text_pos(h: &H, label: &str) -> Pos2 {
    let mut all = Vec::new();
    for cs in &h.output().shapes {
        texts(&cs.shape, &mut all);
    }
    all.iter().filter(|(t, _)| t == label).map(|(_, r)| r.center()).max_by(|a, b| a.x.total_cmp(&b.x)).unwrap_or_else(|| panic!("„{label}“ nicht gezeichnet"))
}

fn tab(h: &mut H, label: &str) {
    let p = text_pos(h, label);
    click(h, p);
}

fn node_rect(h: &H, label: &str) -> egui::Rect {
    h.get_by_label(label).rect()
}

fn attrs(h: &H, id: u64) -> std::sync::Arc<fp_model::PathAttrs> {
    let d = st(h).session.active().expect("d");
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, fp_model::ElementId(id)).expect("Element");
    fp_engine::docops::path_attrs_of(&e).expect("Attribute")
}

fn undo_count(h: &H) -> usize {
    st(h).session.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

fn doc(h: &H) -> fp_model::Document {
    (*st(h).session.active().expect("d").doc).clone()
}

#[test]
fn swatch_onto_object_sets_fill_or_stroke() {
    let (mut h, target) = harness();
    tab(&mut h, "Farbfelder");
    let grid = node_rect(&h, "Farbfelder-Raster");
    let cell = grid.min + egui::vec2(6.0, 6.0);
    let swatch_color = fp_ui::panels::swatch_colors(&h.ctx, st(&h))[0];
    let base = doc(&h);
    let n0 = undo_count(&h);
    // neben das Objekt: nichts
    { let dst = screen(&h, 250.0, 250.0); drag(&mut h, cell, dst, 20, Modifiers::NONE); }
    assert_eq!(doc(&h), base, "Ablegen neben dem Objekt ändert nichts");
    // außerhalb der Leinwand (zurück ins Bedienfeld): nichts
    drag(&mut h, cell, cell + egui::vec2(40.0, 20.0), 12, Modifiers::NONE);
    assert_eq!(doc(&h), base);
    // auf das Objekt: Füllung
    { let dst = screen(&h, 100.0, 85.0); drag(&mut h, cell, dst, 30, Modifiers::NONE); }
    assert_eq!(attrs(&h, target).fill_color.0 | 0xff00_0000, swatch_color.0 | 0xff00_0000, "Farbfeld setzt die Füllung");
    assert_eq!(undo_count(&h), n0 + 1, "ein Verlaufsschritt");
    assert!(st(&h).session.active().is_some_and(|d| d.selection.elements.is_empty()), "Auswahl bleibt unverändert");
    // mit Umschalt: Strichfarbe
    { let dst = screen(&h, 100.0, 85.0); drag(&mut h, cell, dst, 30, Modifiers::SHIFT); }
    assert_eq!(attrs(&h, target).stroke_color.0 | 0xff00_0000, swatch_color.0 | 0xff00_0000, "Umschalt: Strich");
    // Esc während des Ziehens bricht ab
    let before = doc(&h);
    ev(&mut h, Event::PointerMoved(cell));
    press(&mut h, cell, true, Modifiers::NONE);
    for i in 1..=10 {
        let p = cell + (screen(&h, 100.0, 85.0) - cell) * (i as f32 / 10.0);
        ev(&mut h, Event::PointerMoved(p));
    }
    ev(&mut h, Event::Key { key: Key::Escape, physical_key: Some(Key::Escape), pressed: true, repeat: false, modifiers: Modifiers::NONE });
    let dst = screen(&h, 100.0, 85.0);
    press(&mut h, dst, false, Modifiers::NONE);
    h.run_steps(3);
    assert_eq!(doc(&h), before, "Esc bricht das Ziehen ab");
    // Rückgängig exakt
    h.state_mut().state.session.execute("edit.undo", json!({})).expect("u");
    h.state_mut().state.session.execute("edit.undo", json!({})).expect("u");
    assert_eq!(doc(&h), base);
}

#[test]
fn style_onto_object_applies_style() {
    let (mut h, target) = harness();
    // „Stile“ ist in der Vorgabe-Anordnung schon vorn (Klick auf die aktive Karte klappt die Gruppe ein)
    let cell = node_rect(&h, "Stil Rot").center();
    let n0 = undo_count(&h);
    { let dst = screen(&h, 100.0, 85.0); drag(&mut h, cell, dst, 30, Modifiers::NONE); }
    let d = st(&h).session.active().expect("d");
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, fp_model::ElementId(target)).expect("e");
    assert_eq!(e.common.style.as_deref(), Some("Rot"), "Stil verknüpft");
    assert_eq!(attrs(&h, target).fill_color.0 & 0x00ff_ffff, 0x00ff_0000, "Stilfüllung rot");
    assert_eq!(undo_count(&h), n0 + 1);
    // neben das Objekt: nichts
    let base = doc(&h);
    { let dst = screen(&h, 250.0, 250.0); drag(&mut h, cell, dst, 20, Modifiers::NONE); }
    assert_eq!(doc(&h), base);
}

#[test]
fn symbol_onto_canvas_places_instance_at_drop_point() {
    let (mut h, _) = harness();
    tab(&mut h, "Dokumentbibliothek");
    let row = text_pos(&h, "Stern");
    let n0 = undo_count(&h);
    { let dst = screen(&h, 220.0, 160.0); drag(&mut h, row, dst, 40, Modifiers::NONE); }
    let d = st(&h).session.active().expect("d");
    let id = *d.selection.elements.first().expect("Instanz ausgewählt");
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, id).expect("e");
    assert!(matches!(e.kind, fp_model::ElementKind::Instance(_)), "Instanz");
    let b = fp_engine::docops::element_bounds(&d.doc, &e).expect("b");
    let c = ((b.left + b.right) / 2.0, (b.top + b.bottom) / 2.0);
    assert!((c.0 - 220.0).abs() <= 1.0 && (c.1 - 160.0).abs() <= 1.0, "an der Ablagestelle: {c:?}");
    assert_eq!(undo_count(&h), n0 + 1);
    // außerhalb der Leinwand: nichts
    let base = doc(&h);
    drag(&mut h, row, row + egui::vec2(-30.0, 40.0), 15, Modifiers::NONE);
    assert_eq!(doc(&h), base);
}
