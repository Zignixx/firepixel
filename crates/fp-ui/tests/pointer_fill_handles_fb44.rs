//! FB-44 rest (RE spec 69 §3) with real canvas input: the Pointer over a gradient's fill handle
//! shows the hollow arrow, over the handle line the rotate cursor; dragging the line turns both
//! arms about h1 (one row "Verlaufsgriffe drehen"), Shift limits a handle drag to the dominant
//! axis, Alt on a handle clones the object and leaves the handles alone.

use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use fp_engine::tools::{Cursor, ToolId};
use fp_model::{ElementKind, FillHandles};
use fp_ui::FirepixelApp;
use fp_ui::app_state::UiSettings;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

/// Rectangle 50,50–450,250 with a linear gradient h1 (200,150) → h2 (320,150) (h3 (200,30)),
/// selected, Pointer active. Returns the harness and the rectangle's id.
fn harness() -> (H, u64) {
    fp_engine::shell::disable();
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 500, "height": 300, "background": "#ffffff"})).expect("doc");
    let r = s.execute("draw.rectangle", json!({"rect": [50, 50, 450, 250], "strokeWidth": 0})).expect("rect").value["id"].as_u64().expect("id");
    s.execute("select.set", json!({"ids": [r]})).expect("select");
    s.execute("element.setGradient", json!({"from": [200, 150], "to": [320, 150]})).expect("gradient");
    s.set_tool(ToolId::Pointer).expect("Pointer");
    let ctx = egui::Context::default();
    let mut a = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    a.state.os_clipboard = false;
    a.state.sync_render = true;
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).with_pixels_per_point(1.0).with_max_steps(8).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), a);
    h.run_steps(4);
    (h, r)
}

fn at(h: &H, x: f32, y: f32) -> Pos2 {
    let st = &h.state().state;
    let d = st.session.active().expect("doc");
    let area = st.canvases.get(&d.id).and_then(|c| c.area).expect("canvas");
    let (sx, sy) = d.view.doc_to_screen(fp_geom::Point::new(x, y));
    Pos2::new(area.min.x + sx as f32, area.min.y + sy as f32)
}

fn handles(h: &H, id: u64) -> FillHandles {
    let d = h.state().state.session.active().expect("doc");
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, fp_model::ElementId(id)).expect("element");
    match &e.kind {
        ElementKind::Path(p) => p.fill_handles,
        _ => panic!("path"),
    }
}

fn count(h: &H) -> usize {
    let d = h.state().state.session.active().expect("doc");
    fp_engine::docops::own_layers(&d.doc, d.page).first().and_then(|(_, l)| l.cel(d.frame).map(|c| c.elements.len())).unwrap_or(0)
}

fn undo_count(h: &H) -> usize {
    h.state().state.session.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

fn undo_label(h: &H) -> String {
    h.state().state.session.active().and_then(|d| d.history.undo_label().map(str::to_string)).unwrap_or_default()
}

fn near(p: fp_geom::Point, x: f32, y: f32) -> bool {
    (p.x - x).abs() < 0.05 && (p.y - y).abs() < 0.05
}

/// One frame; held modifiers go first like egui-winit.
fn frame(h: &mut H, held: Modifiers, evs: Vec<Event>) {
    h.input_mut().events.push(Event::ModifiersChanged(held));
    for e in evs {
        h.input_mut().events.push(e);
    }
    h.step();
}

fn hover(h: &mut H, p: Pos2) -> Cursor {
    frame(h, Modifiers::NONE, vec![Event::PointerMoved(p)]);
    frame(h, Modifiers::NONE, vec![Event::PointerMoved(p + egui::vec2(0.0, 0.0))]);
    h.state().state.tool_cursor
}

/// A drag in 20 frames with `m` held from before the press until after the release.
fn drag(h: &mut H, from: Pos2, to: Pos2, m: Modifiers) {
    let button = |pressed, pos| Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: m };
    frame(h, m, vec![Event::PointerMoved(from)]);
    frame(h, m, vec![button(true, from)]);
    for i in 1..=20 {
        frame(h, m, vec![Event::PointerMoved(from + (to - from) * (i as f32 / 20.0))]);
    }
    frame(h, m, vec![button(false, to)]);
    frame(h, m, vec![]);
    frame(h, Modifiers::NONE, vec![]);
    h.run_steps(3);
}

#[test]
fn hover_cursors_over_fill_handles_and_their_line() {
    let (mut h, _) = harness();
    let p = at(&h, 320.0, 150.0);
    assert_eq!(hover(&mut h, p), Cursor::FillHandle, "over h2: hollow arrow");
    let p = at(&h, 200.0, 150.0);
    assert_eq!(hover(&mut h, p), Cursor::FillHandle, "over h1");
    let p = at(&h, 260.0, 151.0);
    assert_eq!(hover(&mut h, p), Cursor::FillRotate, "over the line h1–h2: rotate cursor");
    let p = at(&h, 120.0, 220.0);
    let c = hover(&mut h, p);
    assert!(c != Cursor::FillRotate && c != Cursor::FillHandle, "elsewhere: {c:?}");
}

#[test]
fn line_drag_rotates_both_arms_in_one_row() {
    fp_i18n::set_language("de");
    let (mut h, r) = harness();
    let n0 = undo_count(&h);
    let (from, to) = (at(&h, 260.0, 150.0), at(&h, 260.0, 190.0));
    drag(&mut h, from, to, Modifiers::NONE);
    let hd = handles(&h, r);
    // toward = (60,40) relative to h1: the arm keeps its length 120 (RE spec 69 §3.2)
    assert!(near(hd.start, 200.0, 150.0), "h1 stays: {hd:?}");
    assert!(near(hd.end, 299.846, 216.564), "{hd:?}");
    assert!(near(hd.second, 266.564, 50.154), "h3 turned along: {hd:?}");
    assert_eq!(undo_count(&h), n0 + 1, "one row");
    assert_eq!(undo_label(&h), "Verlaufsgriffe drehen");
    assert_eq!(count(&h), 1, "no copy");
}

#[test]
fn shift_constrains_a_handle_drag() {
    let (mut h, r) = harness();
    let shift = Modifiers { shift: true, ..Modifiers::NONE };
    let (from, to) = (at(&h, 320.0, 150.0), at(&h, 330.0, 190.0));
    drag(&mut h, from, to, shift);
    let hd = handles(&h, r);
    assert!(near(hd.end, 320.0, 190.0), "only the dominant y: {hd:?}");
    assert!(near(hd.start, 200.0, 150.0), "{hd:?}");
}

#[test]
fn alt_drag_on_a_handle_clones_the_object() {
    let (mut h, r) = harness();
    let h0 = handles(&h, r);
    let n0 = count(&h);
    let alt = Modifiers { alt: true, ..Modifiers::NONE };
    let (from, to) = (at(&h, 320.0, 150.0), at(&h, 320.0, 190.0));
    drag(&mut h, from, to, alt);
    assert_eq!(count(&h), n0 + 1, "a copy of the object");
    assert_eq!(handles(&h, r), h0, "the original's handles unchanged");
}
