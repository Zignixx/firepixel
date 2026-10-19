//! macOS window handling (FB-04): the platform code itself runs only on macOS; these tests cover
//! the platform-independent decisions behind it.

use egui::{ViewportCommand, ViewportId, ViewportInfo};
use egui_kittest::kittest::Queryable as _;

/// One pass with the given "minimised" flag in the root viewport info; returns whether the flag was
/// healed, the commands sent to the root viewport and whether a new frame was requested at once.
fn pass(flag: Option<bool>, window: Option<bool>) -> (bool, Vec<ViewportCommand>, bool) {
    let ctx = egui::Context::default();
    let mut input = egui::RawInput::default();
    input.viewports.insert(
        ViewportId::ROOT,
        ViewportInfo {
            minimized: flag,
            ..Default::default()
        },
    );
    let mut healed = false;
    let mut out = ctx.run_ui(input, |ui| {
        healed = fp_ui::heal_stale_minimized(ui.ctx(), window);
    });
    out.textures_delta.clear();
    let root = out.viewport_output.get(&ViewportId::ROOT);
    let cmds = root.map(|v| v.commands.clone()).unwrap_or_default();
    let now = root.is_some_and(|v| v.repaint_delay.is_zero());
    (healed, cmds, now)
}

/// FB-04: after a restore from the Dock the flag still says "minimised" while the window is back;
/// the flag is cleared and a frame is requested, otherwise eframe would never run `ui` again.
#[test]
fn stale_minimized_flag_is_cleared_after_restore() {
    let (healed, cmds, now) = pass(Some(true), Some(false));
    assert!(healed);
    assert!(cmds.contains(&ViewportCommand::Minimized(false)), "{cmds:?}");
    assert!(now, "repaint requested at once");
}

/// Really minimised, never minimised or unknown window state: nothing to do.
#[test]
fn consistent_minimized_flag_is_left_alone() {
    for (flag, window) in [
        (Some(true), Some(true)),
        (Some(true), None),
        (Some(false), Some(false)),
        (None, Some(false)),
    ] {
        let (healed, cmds, _) = pass(flag, window);
        assert!(!healed, "{flag:?} {window:?}");
        assert!(
            !cmds.contains(&ViewportCommand::Minimized(false)),
            "{flag:?} {window:?}: {cmds:?}"
        );
    }
}

/// One pass with a single key press; returns whether `take_quit_key` saw Quit and whether the key
/// was consumed (no longer visible to later handlers in the same pass).
fn quit_key(key: egui::Key, modifiers: egui::Modifiers) -> (bool, bool) {
    let ctx = egui::Context::default();
    let input = egui::RawInput {
        events: vec![egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }],
        ..Default::default()
    };
    let (mut quit, mut left) = (false, true);
    let mut out = ctx.run_ui(input, |ui| {
        quit = fp_ui::input::take_quit_key(ui.ctx());
        left = ui.ctx().input(|i| !i.events.is_empty());
    });
    out.textures_delta.clear();
    (quit, !left)
}

/// FB-05: Cmd+Q as egui-winit reports it on macOS (`mac_cmd` + `command`) is the Quit command and
/// is consumed, so neither the shortcut set nor a text field sees it a second time.
#[test]
fn cmd_q_is_quit_on_macos() {
    let mac_cmd = egui::Modifiers {
        mac_cmd: true,
        command: true,
        ..Default::default()
    };
    assert_eq!(quit_key(egui::Key::Q, mac_cmd), (true, true));
    // plain Q (tool key), Cmd+W: not Quit and left alone
    assert_eq!(quit_key(egui::Key::Q, egui::Modifiers::NONE), (false, false));
    assert_eq!(quit_key(egui::Key::W, mac_cmd), (false, false));
}

/// App bar alone, with own window chrome (`native = false`) or below the system title bar.
fn app_bar(native: bool) -> egui_kittest::Harness<'static, fp_ui::FirepixelApp> {
    let ctx = egui::Context::default();
    let app = fp_ui::FirepixelApp::with_session(
        &ctx,
        fp_engine::Session::default(),
        fp_ui::app_state::UiSettings::default(),
        false,
    );
    let mut h = egui_kittest::Harness::builder()
        .with_size(egui::vec2(1200.0, 31.0))
        .with_pixels_per_point(1.0)
        .build_ui_state(
            move |ui, app: &mut fp_ui::FirepixelApp| {
                fp_ui::appbar::show_bar(ui, &mut app.state, native)
            },
            app,
        );
    h.run_steps(2);
    h
}

/// FB-03: below the native macOS title bar (traffic lights) the app bar draws no own window
/// buttons; with own chrome (Windows/Linux) they stay. The window decorations follow the same
/// switch.
#[test]
fn native_title_bar_hides_own_window_buttons() {
    let labels = ["ui-window-minimize", "ui-window-maximize", "ui-window-close"]
        .map(fp_i18n::t);
    let own = app_bar(false);
    for l in &labels {
        assert!(own.query_by_label(l).is_some(), "own chrome: {l}");
    }
    let native = app_bar(true);
    for l in &labels {
        assert!(native.query_by_label(l).is_none(), "native title bar: {l}");
    }
    assert_eq!(
        fp_ui::native_options().viewport.decorations,
        Some(fp_ui::appbar::native_title_bar())
    );
    assert_eq!(
        fp_ui::appbar::native_title_bar(),
        cfg!(target_os = "macos")
    );
}
