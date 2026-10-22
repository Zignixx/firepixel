//! FB-23 / TW-203: "Dither" fill edge (Firepixel extension) in the Properties inspector.
//! - The fill Edge combo has a fourth entry "Dither"; choosing it sets edge 3 with the default
//!   width 10 and enables the width field (the Feather field is reused).
//! - Typing a width keeps the dither kind; the pop-up slider previews live and keeps the kind.
//! - Tool defaults carry the dither edge and its width to new objects (attrs_params round trip).
//!
//! Realistic input like egui-winit on Windows (one event per frame, typing as `Event::Text`).

use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::{By, NodeT, Queryable};
use fp_ui::FirepixelApp;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::inspector::obj_quads::{EDGE_ITEMS, ToolAttrs};
use fp_ui::inspector::preview::PreviewValue;
use serde_json::json;
use std::sync::Arc;

type H = Harness<'static, FirepixelApp>;

const W: f32 = 1800.0;
const HH: f32 = 1130.0;

fn harness_with_rect() -> (H, u64) {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 500, "height": 300, "background": "#ffffff"})).expect("doc");
    let v = s.execute("draw.rectangle", json!({"rect": [40, 40, 300, 200], "fillColor": "#336699", "strokeWidth": 0})).expect("rect");
    let id = v.value.get("id").and_then(|x| x.as_u64()).expect("id");
    s.execute("select.set", json!({"ids": [id]})).expect("select");
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(W, HH)).with_pixels_per_point(1.0).with_max_steps(20).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    (h, id)
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn ev(h: &mut H, e: Event) {
    h.event(e);
    h.run_steps(1);
}

fn key(h: &mut H, k: Key, m: Modifiers) {
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: m });
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: false, repeat: false, modifiers: m });
}

fn press(h: &mut H, p: Pos2, down: bool) {
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: down, modifiers: Modifiers::NONE });
}

fn click(h: &mut H, p: Pos2) {
    ev(h, Event::PointerMoved(p));
    press(h, p, true);
    press(h, p, false);
    h.run_steps(2);
}

fn doc(h: &H) -> Arc<fp_model::Document> {
    Arc::clone(&st(h).session.active().expect("doc").doc)
}

fn edge_of(d: &fp_model::Document, id: u64) -> (i32, i32) {
    let e = fp_engine::docops::element_on_page(d, 0, fp_model::ElementId(id)).expect("element");
    fp_engine::docops::path_attrs_of(&e).and_then(|a| a.fill.as_ref().map(|f| (f.edge, f.feather))).expect("fill")
}

fn combos(h: &H) -> Vec<(egui::Rect, String)> {
    h.get_all(By::new().role(egui::accesskit::Role::ComboBox)).map(|n| (n.rect(), n.value().unwrap_or_default())).filter(|(r, _)| r.top() > HH - 160.0).collect()
}

/// Number field right of the Edge combo (same row, to its right).
fn width_field(h: &H, combo: egui::Rect) -> (egui::Rect, String, bool) {
    h.get_all(By::new().role(egui::accesskit::Role::TextInput))
        .filter(|n| (n.rect().center().y - combo.center().y).abs() < 2.0 && n.rect().left() > combo.right())
        .min_by(|a, b| a.rect().left().total_cmp(&b.rect().left()))
        .map(|n| (n.rect(), n.value().unwrap_or_default(), !n.accesskit_node().is_disabled()))
        .expect("width field")
}

fn type_into(h: &mut H, field: egui::Rect, s: &str) {
    click(h, field.center());
    key(h, Key::End, Modifiers::NONE);
    key(h, Key::Home, Modifiers::SHIFT);
    for c in s.chars() {
        ev(h, Event::Text(c.to_string()));
    }
}

#[test]
fn edge_items_have_dither_as_fourth_entry() {
    assert_eq!(EDGE_ITEMS.len(), 4);
    assert_eq!(EDGE_ITEMS[3], "pi-edge-dither");
    for lang in ["de", "en"] {
        assert_eq!(fp_i18n::with_language(lang, || fp_i18n::t("pi-edge-dither")), "Dither", "{lang}");
    }
}

#[test]
fn choosing_dither_sets_edge_and_enables_the_width_field() {
    let (mut h, id) = harness_with_rect();
    assert_eq!(edge_of(&doc(&h), id), (1, 0));
    let c = combos(&h).into_iter().find(|(_, v)| v == "Anti-Alias").expect("fill edge combo").0;
    let (_, _, enabled) = width_field(&h, c);
    assert!(!enabled, "width field disabled for Anti-Alias");
    click(&mut h, c.center());
    let item = h.get_by_label("Dither").rect();
    click(&mut h, item.center());
    h.run_steps(2);
    assert_eq!(edge_of(&doc(&h), id), (fp_model::Fill::EDGE_DITHER, 10), "dither with the default width");
    assert!(combos(&h).iter().any(|(_, v)| v == "Dither"), "combo shows Dither: {:?}", combos(&h));
    let (f, value, enabled) = width_field(&h, c);
    assert_eq!(value, "10");
    assert!(enabled, "width field enabled for Dither");
    // a typed width keeps the dither kind
    type_into(&mut h, f, "6");
    key(&mut h, Key::Enter, Modifiers::NONE);
    h.run_steps(2);
    assert_eq!(edge_of(&doc(&h), id), (fp_model::Fill::EDGE_DITHER, 6));
    // the pop-up slider previews live with the dither kind kept, nothing applied while dragging
    let a = Pos2::new(f.right() + 3.0 + 1.5 + 6.75, f.center().y);
    ev(&mut h, Event::PointerMoved(a));
    press(&mut h, a, true);
    let to = Pos2::new(a.x, a.y - 40.0);
    for i in 1..=10 {
        ev(&mut h, Event::PointerMoved(a + (to - a) * (i as f32 / 10.0)));
    }
    h.run_steps(1);
    let p = st(&h).inspector_preview.clone().expect("preview");
    let PreviewValue::Feather(live) = p.value else { panic!("feather/dither preview: {:?}", p.value) };
    assert!(live > 6.0, "dragged up: {live}");
    assert_eq!(edge_of(&p.apply(&doc(&h)), id), (fp_model::Fill::EDGE_DITHER, live as i32), "preview keeps dither");
    assert_eq!(edge_of(&doc(&h), id), (fp_model::Fill::EDGE_DITHER, 6), "document untouched while dragging");
    press(&mut h, to, false);
    h.run_steps(2);
    assert_eq!(edge_of(&doc(&h), id), (fp_model::Fill::EDGE_DITHER, live as i32), "release applies");
    // back to Hard drops the width
    let c = combos(&h).into_iter().find(|(_, v)| v == "Dither").expect("combo").0;
    click(&mut h, c.center());
    let item = h.get_by_label(&fp_i18n::t("pi-edge-hard")).rect();
    click(&mut h, item.center());
    h.run_steps(2);
    assert_eq!(edge_of(&doc(&h), id), (0, 0));
}

#[test]
fn tool_defaults_carry_the_dither_edge() {
    let ta = ToolAttrs { edge: fp_model::Fill::EDGE_DITHER, feather: 5, ..ToolAttrs::default() };
    let (fill, _) = fp_ui::canvas::overlay::attrs_params(&ta);
    let fill = fill.expect("non-default fill");
    assert_eq!(fill["edge"], json!(3));
    assert_eq!(fill["feather"], json!(5));
    let mut back = ToolAttrs::default();
    fp_ui::canvas::overlay::import_attrs_params(&mut back, Some(&fill), None);
    assert_eq!((back.edge, back.feather), (3, 5));
}
