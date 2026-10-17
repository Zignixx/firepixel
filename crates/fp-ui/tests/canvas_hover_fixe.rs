//! FIX-E (CT-3, niedrig): Verlässt der Zeiger das Fenster, blieb die rote Vorauswahl-Hervorhebung
//! des Zeigerwerkzeugs stehen (gesehen als rote Linie ab dem Seitenursprung). Sie gilt nur, solange
//! der Zeiger über der Leinwand ist.

use egui::{Event, Pos2};
use egui_kittest::Harness;
use fp_engine::Overlay;
use fp_ui::app_state::UiSettings;
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn harness(s: fp_engine::Session) -> H {
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(1200.0, 800.0)).with_pixels_per_point(1.0).with_max_steps(12).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    h
}

fn highlighted(h: &H) -> bool {
    let st = &h.state().state;
    let id = st.session.active_id().expect("Dokument");
    st.canvases.get(&id).is_some_and(|c| c.feedback.iter().any(|o| matches!(o, Overlay::Highlight { .. })))
}

fn screen_of(h: &H, x: f64, y: f64) -> Pos2 {
    let st = &h.state().state;
    let d = st.session.active().expect("Dokument");
    let area = st.canvases.get(&d.id).and_then(|c| c.area).expect("Leinwand");
    let (sx, sy) = d.view.doc_to_screen(fp_geom::Point::new(x as f32, y as f32));
    Pos2::new(area.min.x + sx as f32, area.min.y + sy as f32)
}

#[test]
fn hover_highlight_goes_away_when_the_pointer_leaves_the_window() {
    fp_engine::shell::disable();
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 200, "height": 150, "background": "#ffffff"})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [20, 20, 120, 90]})).expect("rect");
    s.execute("select.none", json!({})).expect("none");
    s.execute("tool.select", json!({"tool": "tool.pointer"})).expect("tool");
    let mut h = harness(s);
    // mehrmals hinein und hinaus (§4b)
    for _ in 0..3 {
        let p = screen_of(&h, 21.0, 50.0);
        h.event(Event::PointerMoved(p));
        h.run_steps(2);
        assert!(highlighted(&h), "über dem Rand des Rechtecks: rote Hervorhebung");
        h.event(Event::PointerGone);
        h.run_steps(2);
        assert!(!highlighted(&h), "Zeiger außerhalb des Fensters: keine Hervorhebung mehr");
    }
}
