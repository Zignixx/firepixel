//! FB-20 (TW-198): the "Pattern" fill-kind button opens the pattern pop-up together with `pi.fill`.
//! When that command failed ("Cotton not found" on a machine without the reference application's
//! pattern folders) the pop-up stayed marked open without ever being drawn and claimed every later
//! canvas click: nothing could be dragged and no tool worked until the application was closed.
//! Here `pi.fill` is made to fail on purpose; after the error the Pointer still drags the object and
//! a tool switch (tools panel) followed by drawing still works.

use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::{By, Queryable};
use egui_kittest::Harness;
use fp_engine::command::{cmd, CommandRegistry};
use fp_engine::{EngineError, Preferences, Session, ToolId};
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::inspector::obj_quads::{self, DEFAULT_TEXTURE};
use fp_ui::inspector::popups;
use fp_ui::FirepixelApp;
use serde_json::json;
use std::sync::Arc;

type H = Harness<'static, FirepixelApp>;

const W: f32 = 1800.0;
const HH: f32 = 1130.0;
const ERROR: &str = "Cotton not found";

/// Session whose `pi.fill` always fails like on the tester's machine.
fn failing_session() -> (Session, u64) {
    let mut r = CommandRegistry::with_defaults();
    r.add(cmd("pi.fill").run(|_, _| Err(EngineError::InvalidParams { name: "pi.fill".into(), message: ERROR.into() })));
    let mut s = Session::with_registry(Preferences::new(), Arc::new(r));
    s.execute("file.new", json!({"width": 500, "height": 300, "background": "#ffffff"})).expect("document");
    let id = s.execute("draw.rectangle", json!({"rect": [50, 50, 150, 120], "fillColor": "#ff0000", "strokeWidth": 0})).expect("rectangle").value["id"].as_u64().expect("id");
    (s, id)
}

fn harness(s: Session) -> H {
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(W, HH)).with_pixels_per_point(1.0).with_max_steps(20).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    h
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn ev(h: &mut H, e: Event) {
    h.event(e);
    h.run_steps(1);
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

/// Drag like a real mouse: one event per frame.
fn drag(h: &mut H, from: Pos2, to: Pos2) {
    ev(h, Event::PointerMoved(from));
    press(h, from, true);
    for i in 1..=8 {
        let t = i as f32 / 8.0;
        ev(h, Event::PointerMoved(from + (to - from) * t));
    }
    press(h, to, false);
    h.run_steps(3);
}

/// Screen point of a document point.
fn at(h: &H, x: f32, y: f32) -> Pos2 {
    let s = st(h);
    let d = s.session.active().expect("document");
    let pr = s.canvases.get(&d.id).and_then(|c| c.page_screen).expect("page on screen");
    let r = fp_render::Renderer::page_rect(&d.doc, d.page).expect("page");
    let (w, ht) = ((r.right - r.left) as f32, (r.bottom - r.top) as f32);
    egui::pos2(pr.left() + (x - r.left as f32) / w * pr.width(), pr.top() + (y - r.top as f32) / ht * pr.height())
}

fn bounds(h: &H, id: u64) -> fp_geom::Rect {
    let d = st(h).session.active().expect("document");
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, fp_model::ElementId(id)).expect("element");
    fp_engine::docops::element_bounds(&d.doc, &e).expect("bounds")
}

fn count(h: &H) -> usize {
    let d = st(h).session.active().expect("document");
    fp_engine::docops::own_layers(&d.doc, d.page).iter().map(|(_, l)| l.cel(d.frame).map(|c| c.elements.len()).unwrap_or(0)).sum()
}

/// Pattern fill-kind button: fourth button above the fill texture list (as in the real window).
fn pattern_button(h: &H) -> Pos2 {
    let vein = obj_quads::texture_label(DEFAULT_TEXTURE);
    let tex = h
        .get_all(By::new().role(egui::accesskit::Role::ComboBox))
        .filter(|n| n.value().as_deref() == Some(vein.as_str()) && n.rect().top() > HH - 160.0)
        .map(|n| n.rect())
        .min_by(|a, b| a.left().total_cmp(&b.left()))
        .expect("fill texture list");
    egui::pos2(tex.left() + 119.0, tex.top() - 38.0)
}

#[test]
fn failed_pattern_fill_leaves_pointer_and_tools_working() {
    let (s, id) = failing_session();
    let mut h = harness(s);
    let _ = h.state_mut().state.session.set_tool(ToolId::Pointer);
    h.state_mut().state.run("select.set", json!({"ids": [id], "mode": "replace"}));
    h.run_steps(3);
    let p = pattern_button(&h);
    click(&mut h, p);
    h.run_steps(3);
    let now = h.ctx.input(|i| i.time);
    let status = st(&h).shown_status(now).map(str::to_string).unwrap_or_default();
    assert!(status.contains(ERROR), "error shown in the status bar: {status:?}");
    assert!(!popups::fill_popup_open(&h.ctx), "no invisible fill pop-up left open");
    assert!(!popups::anchored_open(&h.ctx, obj_quads::fill_popup_ids(false).1), "pattern pop-up closed");
    // the Pointer still drags the object
    let b0 = bounds(&h, id);
    let (a, b) = (at(&h, 100.0, 85.0), at(&h, 160.0, 125.0));
    drag(&mut h, a, b);
    let b1 = bounds(&h, id);
    assert!((b1.left - b0.left - 60.0).abs() < 1.5 && (b1.top - b0.top - 40.0).abs() < 1.5, "dragged: {b0:?} -> {b1:?}");
    // a tool switch in the tools panel still works, and the new tool draws
    let tool = h.query_all(By::new().role(egui::accesskit::Role::Button).label_contains("Rechteck")).next().expect("rectangle tool button").rect();
    click(&mut h, tool.center());
    assert_eq!(st(&h).session.tools.active, ToolId::Rectangle, "tool switched");
    let n0 = count(&h);
    let (a, b) = (at(&h, 300.0, 150.0), at(&h, 380.0, 220.0));
    drag(&mut h, a, b);
    assert_eq!(count(&h), n0 + 1, "rectangle drawn after the failed command");
}

/// The general safeguard: a fill pop-up marked open but never drawn (whatever the cause) no longer
/// claims canvas input after one frame.
#[test]
fn stale_fill_popup_closes_itself() {
    let (s, id) = failing_session();
    let mut h = harness(s);
    let _ = h.state_mut().state.session.set_tool(ToolId::Pointer);
    h.state_mut().state.run("select.set", json!({"ids": [id], "mode": "replace"}));
    h.run_steps(3);
    // the selected rectangle has a solid fill: its pattern pop-up is never drawn
    popups::set_anchored_open(&h.ctx, obj_quads::fill_popup_ids(false).1, true);
    assert!(popups::fill_popup_open(&h.ctx));
    h.run_steps(3);
    assert!(!popups::fill_popup_open(&h.ctx), "stale pop-up closed");
    let b0 = bounds(&h, id);
    let (a, b) = (at(&h, 100.0, 85.0), at(&h, 120.0, 85.0));
    drag(&mut h, a, b);
    assert!((bounds(&h, id).left - b0.left - 20.0).abs() < 1.5, "dragged");
}
