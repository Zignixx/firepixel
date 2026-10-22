//! FB-18 (TW-199): after the window lost the focus and got it back, a second cursor followed the
//! pointer over the canvas (the system cursor plus the painted one) until a click on the toolbar.
//! At any time exactly one cursor representation is shown: either the painted cursor with the
//! system cursor hidden (`CursorIcon::None`), or no painted cursor and a system icon. After focus
//! gain and after the pointer re-enters the window, the hidden state is pushed to the window again
//! (`CursorVisible(true)` then `CursorVisible(false)` in the same frame) without needing a click.

use egui::{CursorIcon, Event, Pos2, ViewportCommand, ViewportId};
use egui_kittest::Harness;
use fp_ui::FirepixelApp;
use fp_ui::app_state::UiSettings;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn harness() -> H {
    fp_engine::shell::disable();
    let mut s = fp_engine::Session::default();
    s.execute(
        "file.new",
        json!({"width": 300, "height": 200, "background": "#ffffff"}),
    )
    .expect("doc");
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

/// A point in the middle of the canvas area.
fn canvas_point(h: &H, dx: f32) -> Pos2 {
    let st = &h.state().state;
    let id = st.session.active_id().expect("document");
    let area = st.canvases.get(&id).and_then(|c| c.area).expect("canvas");
    area.center() + egui::vec2(dx, 0.0)
}

fn icon(h: &H) -> CursorIcon {
    h.output().platform_output.cursor_icon
}

fn painted(h: &H) -> Option<Pos2> {
    h.state().state.painted_cursor
}

/// `CursorVisible` commands of the last frame, in order.
fn visibility_cmds(h: &H) -> Vec<bool> {
    h.output()
        .viewport_output
        .get(&ViewportId::ROOT)
        .map(|v| {
            v.commands
                .iter()
                .filter_map(|c| match c {
                    ViewportCommand::CursorVisible(b) => Some(*b),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Exactly one cursor: painted at the pointer with the system cursor hidden.
fn assert_painted_only(h: &H, at: Pos2, what: &str) {
    assert_eq!(
        painted(h),
        Some(at),
        "{what}: painted cursor at the pointer"
    );
    assert_eq!(icon(h), CursorIcon::None, "{what}: system cursor hidden");
}

/// Exactly one cursor: the system cursor, nothing painted.
fn assert_system_only(h: &H, what: &str) {
    assert_eq!(painted(h), None, "{what}: no painted cursor");
    assert_ne!(icon(h), CursorIcon::None, "{what}: system cursor shown");
}

fn set_focus(h: &mut H, focused: bool) {
    h.input_mut().focused = focused;
    h.event(Event::WindowFocused(focused));
}

#[test]
fn one_cursor_after_focus_loss_and_return() {
    let mut h = harness();
    let p = canvas_point(&h, 0.0);
    h.event(Event::PointerMoved(p));
    h.run_steps(2);
    assert_painted_only(&h, p, "focused window");

    // another window gets the focus; the pointer still moves over the canvas
    set_focus(&mut h, false);
    h.step();
    let q = canvas_point(&h, 30.0);
    h.event(Event::PointerMoved(q));
    h.run_steps(2);
    assert_system_only(&h, "inactive window");
    assert!(
        visibility_cmds(&h).is_empty(),
        "no visibility change while inactive"
    );

    // back (the harness runs one frame per queued event): the very first frame re-applies the
    // hidden state, no click needed
    set_focus(&mut h, true);
    h.step();
    assert_painted_only(&h, q, "focus regained");
    assert_eq!(
        visibility_cmds(&h),
        vec![true, false],
        "hidden state pushed to the window again on focus gain"
    );
    // only once; the painted cursor follows the pointer
    for dx in [-40.0, -20.0] {
        let r = canvas_point(&h, dx);
        h.event(Event::PointerMoved(r));
        h.step();
        assert_painted_only(&h, r, "after focus gain");
        assert!(visibility_cmds(&h).is_empty(), "re-applied only once");
    }
}

#[test]
fn one_cursor_after_the_pointer_leaves_and_reenters_the_window() {
    let mut h = harness();
    let p = canvas_point(&h, 0.0);
    h.event(Event::PointerMoved(p));
    h.run_steps(2);
    assert_painted_only(&h, p, "on the canvas");

    // pointer leaves the window while it is inactive, then the window is activated again and the
    // pointer comes back
    h.event(Event::PointerGone);
    h.step();
    assert_eq!(painted(&h), None, "pointer outside: nothing painted");
    set_focus(&mut h, false);
    h.run_steps(2);
    set_focus(&mut h, true);
    h.step();
    assert_eq!(painted(&h), None, "pointer still outside: nothing painted");

    let q = canvas_point(&h, 25.0);
    h.event(Event::PointerMoved(q));
    h.step();
    assert_painted_only(&h, q, "re-entry");
    assert_eq!(
        visibility_cmds(&h),
        vec![true, false],
        "hidden state re-applied on re-entry"
    );

    // plain re-entry without a focus change re-applies as well
    h.event(Event::PointerGone);
    h.step();
    let r = canvas_point(&h, -25.0);
    h.event(Event::PointerMoved(r));
    h.step();
    assert_painted_only(&h, r, "second re-entry");
    assert_eq!(visibility_cmds(&h), vec![true, false]);
}
