//! macOS window handling (FB-04): the platform code itself runs only on macOS; these tests cover
//! the platform-independent decisions behind it.

use egui::{ViewportCommand, ViewportId, ViewportInfo};

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
