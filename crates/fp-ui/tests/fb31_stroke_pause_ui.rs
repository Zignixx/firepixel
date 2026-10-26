//! FB-31 (Firepixel extension) in the user interface: during a Brush stroke on the canvas the
//! status line reads "Press P …"; the P key pauses the stroke (status line with the instructions),
//! the button may be released and the pointer moved, the next press on the canvas resumes the same
//! stroke and the release applies it as one History row; Enter applies a paused stroke. Without a
//! running stroke P keeps its toolbox shortcut, and while paused P does not switch the tool.

use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use fp_engine::shortcuts::Shortcut;
use fp_engine::tools::ToolId;
use fp_engine::{Session, StrokePause};
use fp_ui::FirepixelApp;
use fp_ui::app_state::{AppState, UiSettings};
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn harness() -> H {
    let mut s = Session::default();
    s.execute(
        "file.new",
        json!({"width": 600, "height": 400, "background": "#ffffff"}),
    )
    .expect("document");
    s.execute("select.none", json!({})).expect("nothing");
    s.set_colors(Some(fp_color::Argb(0xff00_0000)), None);
    s.set_tool(ToolId::Brush).expect("tool");
    s.tools.options.opacity = 33;
    let ctx = egui::Context::default();
    let mut a = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    a.state.sync_render = true;
    let mut h = Harness::builder()
        .renderer(crate::gpu_common::renderer())
        .with_size(egui::vec2(1400.0, 900.0))
        .with_pixels_per_point(1.0)
        .with_max_steps(16)
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), a);
    h.run_steps(3);
    h
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn steps(h: &H) -> usize {
    st(h)
        .session
        .active()
        .map(|d| d.history.undo_count())
        .unwrap_or(0)
}

fn status(h: &H) -> String {
    fp_ui::canvas::footer_status(st(h), h.ctx.input(|i| i.time), false)
}

/// Document point → screen (page placement from the last frame).
fn screen(h: &H, x: f32, y: f32) -> Pos2 {
    let s = st(h);
    let d = s.session.active().expect("document");
    let pr = s
        .canvases
        .get(&d.id)
        .and_then(|c| c.page_screen)
        .expect("page placement");
    let r = fp_render::Renderer::page_rect(&d.doc, d.page).expect("page");
    let (w, ht) = ((r.right - r.left) as f32, (r.bottom - r.top) as f32);
    egui::pos2(
        pr.left() + (x - r.left as f32) / w * pr.width(),
        pr.top() + (y - r.top as f32) / ht * pr.height(),
    )
}

fn button(h: &mut H, p: Pos2, pressed: bool) {
    h.event(Event::PointerButton {
        pos: p,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    });
    h.run_steps(1);
}

fn move_to(h: &mut H, a: (f32, f32), b: (f32, f32)) -> Pos2 {
    let mut last = screen(h, a.0, a.1);
    for i in 1..=12 {
        let t = i as f32 / 12.0;
        last = screen(h, a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
        h.event(Event::PointerMoved(last));
        h.run_steps(1);
    }
    last
}

fn key(h: &mut H, k: Key) {
    h.event(Event::Key {
        key: k,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    });
    h.event(Event::Key {
        key: k,
        physical_key: None,
        pressed: false,
        repeat: false,
        modifiers: Modifiers::NONE,
    });
    h.run_steps(2);
}

#[test]
fn p_pauses_and_the_next_press_resumes_the_stroke() {
    let mut h = harness();
    let n0 = steps(&h);
    assert_ne!(
        status(&h),
        fp_i18n::t("ui-stroke-pause-hint"),
        "no hint without a stroke"
    );
    let p0 = screen(&h, 80.0, 100.0);
    h.event(Event::PointerMoved(p0));
    h.run_steps(1);
    button(&mut h, p0, true);
    let p1 = move_to(&mut h, (80.0, 100.0), (400.0, 100.0));
    assert_eq!(st(&h).session.tool_stroke_pause(), StrokePause::Painting);
    assert_eq!(
        status(&h),
        fp_i18n::t("ui-stroke-pause-hint"),
        "hint while painting"
    );
    key(&mut h, Key::P);
    assert_eq!(
        st(&h).session.tool_stroke_pause(),
        StrokePause::Paused,
        "P pauses"
    );
    assert_eq!(
        st(&h).session.tools.active,
        ToolId::Brush,
        "P does not switch the tool during a stroke"
    );
    assert_eq!(
        status(&h),
        fp_i18n::t("ui-stroke-paused"),
        "instructions while paused"
    );
    button(&mut h, p1, false);
    move_to(&mut h, (400.0, 100.0), (240.0, 40.0));
    h.run_steps(5);
    assert_eq!(
        st(&h).session.tool_stroke_pause(),
        StrokePause::Paused,
        "released and moved: still paused"
    );
    assert_eq!(steps(&h), n0, "nothing applied while paused");
    // P while paused does not switch to the P tool either
    key(&mut h, Key::P);
    assert_eq!(st(&h).session.tools.active, ToolId::Brush);
    assert_eq!(st(&h).session.tool_stroke_pause(), StrokePause::Paused);
    // resume with a press on the canvas, release applies the whole stroke
    let q0 = screen(&h, 240.0, 40.0);
    button(&mut h, q0, true);
    assert_eq!(
        st(&h).session.tool_stroke_pause(),
        StrokePause::Painting,
        "the press resumes"
    );
    let q1 = move_to(&mut h, (240.0, 40.0), (240.0, 300.0));
    button(&mut h, q1, false);
    h.run_steps(2);
    assert_eq!(
        steps(&h),
        n0 + 3,
        "one History row for the whole stroke (after \"Insert Empty Bitmap\", \"Edit Bitmap\", RE spec 71)"
    );
    assert_eq!(st(&h).session.tool_stroke_pause(), StrokePause::Idle);
    assert_ne!(
        status(&h),
        fp_i18n::t("ui-stroke-paused"),
        "the status line is free again"
    );
    assert_ne!(status(&h), fp_i18n::t("ui-stroke-pause-hint"));
}

#[test]
fn enter_applies_a_paused_stroke() {
    let mut h = harness();
    let n0 = steps(&h);
    let p0 = screen(&h, 80.0, 200.0);
    h.event(Event::PointerMoved(p0));
    h.run_steps(1);
    button(&mut h, p0, true);
    let p1 = move_to(&mut h, (80.0, 200.0), (300.0, 200.0));
    key(&mut h, Key::P);
    button(&mut h, p1, false);
    key(&mut h, Key::Enter);
    assert_eq!(
        steps(&h),
        n0 + 3,
        "Enter applies the paused stroke (after \"Insert Empty Bitmap\", \"Edit Bitmap\")"
    );
    assert_eq!(st(&h).session.tool_stroke_pause(), StrokePause::Idle);
    assert_eq!(st(&h).session.tools.active, ToolId::Brush);
}

#[test]
fn p_without_a_stroke_is_the_toolbox_shortcut() {
    let mut h = harness();
    let sc = Shortcut {
        ctrl: false,
        alt: false,
        shift: false,
        vk: 0x50,
    };
    let expected =
        fp_engine::tools::cycle_tool_for_key(st(&h).session.shortcut_set(), &sc, ToolId::Brush);
    key(&mut h, Key::P);
    assert_eq!(
        st(&h).session.tools.active,
        expected.unwrap_or(ToolId::Brush),
        "P selects its tool as before"
    );
}

#[test]
fn status_texts_in_german_and_english() {
    fp_i18n::set_language("en");
    let en = (
        fp_i18n::t("ui-stroke-pause-hint"),
        fp_i18n::t("ui-stroke-paused"),
    );
    fp_i18n::set_language("de");
    let de = (
        fp_i18n::t("ui-stroke-pause-hint"),
        fp_i18n::t("ui-stroke-paused"),
    );
    assert_eq!(
        en.0,
        "Press P on the keyboard to pause the current brush stroke."
    );
    assert_eq!(
        en.1,
        "Stroke paused. It is safe to let go of mouse button or lift stylus. Click anywhere on the screen to resume current stroke. Hit Enter to apply paused stroke."
    );
    assert_eq!(
        de.0,
        "P auf der Tastatur hält den aktuellen Pinselstrich an."
    );
    assert_eq!(
        de.1,
        "Strich angehalten. Maustaste loslassen oder Stift abheben ist jetzt sicher. Klick auf die Leinwand setzt den Strich fort, Enter wendet ihn an."
    );
}
