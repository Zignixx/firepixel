//! Q-55 (re 46 §4.2): with the text tool, a single click inside an existing text block puts the
//! caret at the nearest character boundary without a selection; typed characters are inserted
//! there (runner step xs-shortcuts/15: select TX1, text tool, click, "V", "r", Esc).

use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use fp_ui::app_state::UiSettings;
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn harness(s: fp_engine::Session) -> H {
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .with_pixels_per_point(1.0)
        .with_max_steps(12)
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    h
}

fn screen_of(h: &H, x: f32, y: f32) -> Pos2 {
    let st = &h.state().state;
    let d = st.session.active().expect("doc");
    let area = st.canvases.get(&d.id).and_then(|c| c.area).expect("canvas");
    let (sx, sy) = d.view.doc_to_screen(fp_geom::Point::new(x, y));
    Pos2::new(area.min.x + sx as f32, area.min.y + sy as f32)
}

fn texts(h: &H) -> Vec<String> {
    let d = h.state().state.session.active().expect("doc");
    let mut out = Vec::new();
    for (_, l) in fp_engine::docops::own_layers(&d.doc, d.page) {
        let ci = if l.shared_across_frames { 0 } else { d.frame };
        for e in l.cels.get(ci).map(|c| c.elements.clone()).unwrap_or_default() {
            if let fp_model::ElementKind::Text(t) = &e.kind {
                out.push(t.plain_text());
            }
        }
    }
    out
}

fn click(h: &mut H, p: Pos2) {
    h.event(Event::PointerMoved(p));
    h.run_steps(1);
    h.event(Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.run_steps(1);
    h.event(Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
}

#[test]
fn single_click_in_text_inserts_at_caret() {
    fp_engine::shell::disable();
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("doc");
    let v = s.execute("draw.text", json!({"text": "Abc 123", "anchor": [50, 80], "size": 24, "color": "#000000"})).expect("text");
    let id = v.value["id"].as_u64().unwrap_or(0);
    s.execute("select.set", json!({"ids": [id]})).expect("select");
    s.execute("tool.select", json!({"tool": "tool.text"})).expect("tool");
    let b = {
        let d = s.active().expect("doc");
        fp_engine::docops::ids_bounds(&d.doc, d.page, &[fp_model::ElementId(id)]).expect("bounds")
    };
    let mut h = harness(s);
    // inside the block, right of the "c" before the blank (left of the middle)
    let p = screen_of(&h, b.left + b.width() * 0.4, (b.top + b.bottom) / 2.0);
    click(&mut h, p);
    assert!(h.state().state.session.tool_is_editing_text(), "editing TX1");
    for ch in ["V", "r"] {
        h.event(Event::Text(ch.into()));
        h.run_steps(1);
    }
    h.event(Event::Key { key: Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE });
    h.event(Event::Key { key: Key::Escape, physical_key: None, pressed: false, repeat: false, modifiers: Modifiers::NONE });
    h.run_steps(3);
    let t = texts(&h);
    assert_eq!(t.len(), 1, "no new block: {t:?}");
    assert_eq!(t[0].chars().count(), 9, "inserted, nothing replaced: {t:?}");
    assert!(t[0].contains("Vr") && t[0].replace("Vr", "") == "Abc 123", "{t:?}");
}
