//! K-2: the bridge reports the dialog rectangle from the egui window state and an open drop-down
//! list separately (`ui_state.dialog`, `ui_state.popup`). Before, the runner measured the frame in
//! the image and an open channel list in „Kurven“ shortened the dialog to 79 px (r2.md item 10).
//! Also: history panel and menu show step names in the current language (GT-0k finding 2).

use egui::{Event, PointerButton, Pos2};
use egui_kittest::Harness;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn harness() -> H {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("doc");
    let img = fp_raster::RgbaImage::filled(fp_geom::IRect::from_xywh(10, 10, 120, 80), fp_color::Rgba8::new(200, 100, 50, 255)).expect("Bild");
    s.clipboard.clear();
    s.clipboard.pixels = Some(img);
    s.execute("edit.paste", json!({})).expect("einfügen");
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

fn click(h: &mut H, p: Pos2) {
    for e in [Event::PointerMoved(p), Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Default::default() }] {
        h.event(e);
    }
    h.run_steps(2);
    h.event(Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Default::default() });
    h.run_steps(3);
}

fn rect_of(v: &serde_json::Value) -> [f32; 4] {
    let a = v.as_array().expect("Rechteck");
    [0, 1, 2, 3].map(|i| a[i].as_f64().expect("Zahl") as f32)
}

#[test]
fn dialog_rect_from_window_state_and_popup_apart() {
    fp_i18n::set_language("de");
    let mut h = harness();
    let ui = fp_ui::bridge::ui_state(&h.ctx, st(&h));
    assert!(ui["dialog"].is_null() && ui["popup"].is_null(), "ohne Dialog: {ui}");
    h.state_mut().state.run("filter.adjustColor.curves", json!({}));
    h.run_steps(3);
    let area = fp_ui::dialogs::dialog_rect(&h.ctx, "fp-curves").expect("Dialog");
    let ui = fp_ui::bridge::ui_state(&h.ctx, st(&h));
    assert_eq!(ui["dialog"]["id"], json!("fp-curves"));
    assert_eq!(rect_of(&ui["dialog"]["rect"]), [area.min.x, area.min.y, area.max.x, area.max.y]);
    assert!(ui["popup"].is_null(), "Liste zu: {}", ui["popup"]);
    // Kanal-Liste öffnen (Feld im Kopf der Gruppe „Kanal:“, ab x = 81 im Client-Bereich)
    let client = area.min + egui::vec2(1.0, 1.0 + fp_ui::dialogs::DIALOG_TITLE_H);
    click(&mut h, client + egui::vec2(81.0 + 60.0, 2.0 + 10.0));
    let ui = fp_ui::bridge::ui_state(&h.ctx, st(&h));
    let popup = &ui["popup"];
    assert!(popup.is_object(), "Liste offen: {ui}");
    let items: Vec<&str> = popup["items"].as_array().expect("Einträge").iter().filter_map(|v| v.as_str()).collect();
    assert!(items.len() >= 4 && items[0] == "RGB", "Einträge: {items:?}");
    assert_eq!(popup["selected"], json!("RGB"));
    let pr = rect_of(&popup["rect"]);
    let dr = rect_of(&ui["dialog"]["rect"]);
    // das Dialogrechteck bleibt ganz (nicht 79 px hoch), die Liste liegt darüber
    assert_eq!(dr, [area.min.x, area.min.y, area.max.x, area.max.y], "Dialog unverändert");
    assert!(dr[3] - dr[1] > 300.0, "Dialoghöhe {}", dr[3] - dr[1]);
    assert!(pr[1] >= client.y && pr[3] > pr[1] + 4.0 * 16.0, "Liste {pr:?}");
}

#[test]
fn history_panel_names_follow_language() {
    fp_i18n::set_language("en");
    let mut h = harness();
    h.state_mut().state.session.execute("modify.animation.animateSelection", json!({"addStates": false})).expect("Animieren");
    h.run_steps(2);
    let s = &st(&h).session;
    let d = s.active().expect("doc");
    let last = d.history.steps().last().copied().cloned().expect("Schritt");
    let conv = |lang: &str| fp_i18n::with_language(lang, || s.registry().get("modify.symbol.convertToSymbol").map(|c| c.info().step_label()).unwrap_or_default());
    assert_eq!(last.label, conv("en"));
    assert_eq!(last.key, Some(fp_engine::StepKey::Command("modify.symbol.convertToSymbol".into())));
    // Sprache der Oberfläche gewechselt: der Verlauf zeigt den Namen in der neuen Sprache
    fp_i18n::set_language("de");
    assert_eq!(s.step_label(&last), conv("de"));
    assert_ne!(conv("de"), conv("en"));
}
