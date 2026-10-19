//! FB-01 (re 52 §1.4): the tester's sequence fb01_1..3 with real pointer events. A corner point, a
//! dragged curve point, then a click on that curve point: its outgoing handle is retracted (the
//! incoming one stays), the path stays in progress and the preview of the next segment leaves the
//! point straight; the next click appends.

use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use fp_engine::Overlay;
use fp_ui::FirepixelApp;
use fp_ui::app_state::UiSettings;
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

fn at(h: &H, x: f32, y: f32) -> Pos2 {
    let st = &h.state().state;
    let d = st.session.active().expect("document");
    let area = st.canvases.get(&d.id).and_then(|c| c.area).expect("canvas");
    let (sx, sy) = d.view.doc_to_screen(fp_geom::Point::new(x, y));
    Pos2::new(area.min.x + sx as f32, area.min.y + sy as f32)
}

fn send(h: &mut H, e: Event) {
    h.input_mut().events.push(e);
    h.step();
}

fn button(p: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos: p, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

fn click(h: &mut H, x: f32, y: f32) {
    let p = at(h, x, y);
    send(h, Event::PointerMoved(p));
    send(h, button(p, true));
    send(h, button(p, false));
}

fn drag(h: &mut H, a: (f32, f32), b: (f32, f32)) {
    let pa = at(h, a.0, a.1);
    send(h, Event::PointerMoved(pa));
    send(h, button(pa, true));
    for i in 1..=8 {
        let t = i as f32 / 8.0;
        let p = at(h, a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
        send(h, Event::PointerMoved(p));
    }
    let pb = at(h, b.0, b.1);
    send(h, button(pb, false));
}

fn feedback(h: &H) -> Vec<Overlay> {
    let st = &h.state().state;
    let id = st.session.active_id().expect("document");
    st.canvases.get(&id).map(|c| c.feedback.clone()).unwrap_or_default()
}

#[test]
fn pen_click_on_last_curve_point_retracts_outgoing_handle_kittest() {
    fp_engine::shell::disable();
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 500, "height": 400, "background": "#ffffff"})).expect("doc");
    s.execute("tool.select", json!({"tool": "tool.pen"})).expect("tool");
    let mut h = harness(s);
    // fb01_1: corner point, then a curve point pulled down-right
    click(&mut h, 60.0, 260.0);
    drag(&mut h, (250.0, 150.0), (290.0, 260.0));
    // fb01_2: click on the curve point
    click(&mut h, 250.0, 150.0);
    // fb01_3: hovering away shows a straight preview from the point, the path is still in progress
    let p = at(&h, 330.0, 150.0);
    send(&mut h, Event::PointerMoved(p));
    h.run_steps(2);
    let fb = feedback(&h);
    let straight = fb.iter().any(|o| matches!(o, Overlay::Line { from, .. } if (from.x - 250.0).abs() < 0.01 && (from.y - 150.0).abs() < 0.01));
    assert!(straight, "straight preview leaves the point: {fb:?}");
    click(&mut h, 330.0, 150.0);
    send(&mut h, Event::Key { key: egui::Key::Enter, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
    let st = &h.state().state;
    let d = st.session.active().expect("document");
    let id = d.selection.elements[0];
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, id).expect("path");
    let fp_model::ElementKind::Path(pe) = &e.kind else { panic!("not a path") };
    let n = &pe.contours[0].nodes;
    assert_eq!(n.len(), 3, "no duplicate point, path appended once");
    assert!(!pe.contours[0].closed);
    assert_eq!(n[1].succ, n[1].anchor, "outgoing handle retracted exactly");
    assert!((n[1].pred.x - 210.0).abs() < 0.01 && (n[1].pred.y - 40.0).abs() < 0.01, "incoming handle kept: {:?}", n[1].pred);
}
