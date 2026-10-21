//! Q-60 (run 31, `xs-preferences/20_tab_focus_focus`): a click on a category of the Preferences
//! dialog, then Down. The reference moves its category list from the clicked row to the next one.
//! Events as the bridge's `ui_input` sends them: move, press, release in frames of their own; keys
//! as press and release.

use egui::{Event, Key, Modifiers, PointerButton, Pos2, Rect};
use egui_kittest::kittest::{By, Queryable};
use egui_kittest::Harness;
use fp_ui::app_state::UiSettings;
use fp_ui::FirepixelApp;
use serde_json::{json, Value};

type H = Harness<'static, FirepixelApp>;

fn harness() -> H {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 500, "height": 400, "background": "#ffffff"})).expect("doc");
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .with_pixels_per_point(1.0)
        .with_max_steps(12)
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    // background window like the bridge instance
    h.input_mut().focused = false;
    h.run_steps(3);
    h
}

fn category(h: &H) -> usize {
    h.state().state.dialogs.preferences.as_ref().expect("dialog open").category
}

fn click(h: &mut H, p: Pos2) {
    for e in [
        Event::PointerMoved(p),
        Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE },
        Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE },
    ] {
        h.event(e);
        h.run_steps(1);
    }
    h.run_steps(2);
}

fn key(h: &mut H, k: Key) {
    for pressed in [true, false] {
        h.event(Event::Key { key: k, physical_key: Some(k), pressed, repeat: false, modifiers: Modifiers::NONE });
        h.run_steps(1);
    }
    h.run_steps(2);
}

fn open(h: &mut H) {
    h.state_mut().state.run("edit.preferences", json!({}));
    h.run_steps(4);
    assert_eq!(category(h), 0);
}

/// Centre of the category row `label` (its accessibility node).
fn row_center(h: &H, label: &str) -> Pos2 {
    h.get_all(By::new().label(label)).map(|n| n.rect().center()).next().unwrap_or_else(|| panic!("row {label}"))
}

/// Hits of the bridge's `ui_find` for `label` (what the parity runner looks up for `pref_category`).
fn bridge_hits(h: &mut H, label: &str) -> Vec<Value> {
    let ctx = h.ctx.clone();
    let raw = egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1400.0, 900.0))), focused: false, ..Default::default() };
    let app = h.state_mut();
    let out = ctx.run_ui(raw, |ui| app.root_ui(ui));
    fp_ui::bridge::find_in_output(&json!({"text": label, "exact": true}), &out)
}

fn pos(v: &Value) -> Pos2 {
    Pos2::new(v["center"][0].as_f64().unwrap_or(0.0) as f32, v["center"][1].as_f64().unwrap_or(0.0) as f32)
}

fn focus_row(h: &H) -> Option<usize> {
    let f = h.ctx.memory(|m| m.focused())?;
    (0..20usize).find(|i| f == egui::Id::new(("prefcat", *i)))
}

#[test]
fn click_on_selected_category_then_down_moves_to_next() {
    let mut h = harness();
    open(&mut h);
    // focus elsewhere first (Tab leaves the list), so the click has to give it back
    key(&mut h, Key::Tab);
    assert_eq!(focus_row(&h), None, "Tab left the list");
    let p = row_center(&h, "Allgemein");
    click(&mut h, p);
    assert_eq!(category(&h), 0);
    assert_eq!(focus_row(&h), Some(0), "the click gives the list the keyboard focus");
    key(&mut h, Key::ArrowDown);
    assert_eq!(category(&h), 1, "Down moves to the next category");
    assert_eq!(focus_row(&h), Some(1));
}

#[test]
fn click_on_other_category_then_down_moves_to_next() {
    let mut h = harness();
    open(&mut h);
    key(&mut h, Key::Tab);
    let p = row_center(&h, "Bearbeitung");
    click(&mut h, p);
    assert_eq!(category(&h), 1);
    assert_eq!(focus_row(&h), Some(1));
    key(&mut h, Key::ArrowDown);
    assert_eq!(category(&h), 2);
}

/// Q-60 verdict: `ui_find` for "Allgemein" ranks the page heading (painted text right of the list,
/// drawn later) before the list row, so a click on its first hit lands on the heading, not on the
/// category. Clicked where `ui_find` reports the row itself (its accessibility node), Down moves on.
#[test]
fn bridge_click_on_the_category_row_then_down_moves_to_next() {
    let mut h = harness();
    open(&mut h);
    let hits = bridge_hits(&mut h, "Allgemein");
    let row = hits.iter().find(|v| v["source"] == "accesskit").cloned().unwrap_or_else(|| panic!("row node: {hits:?}"));
    assert!(hits.iter().any(|v| v["source"] == "text" && pos(v).x > pos(&row).x + 100.0), "page heading right of the list: {hits:?}");
    click(&mut h, pos(&row));
    assert_eq!(focus_row(&h), Some(0), "{row}");
    key(&mut h, Key::ArrowDown);
    assert_eq!(category(&h), 1, "Down after the click on the row");
}
