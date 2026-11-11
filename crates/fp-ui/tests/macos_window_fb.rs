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

/// FB-05: the custom items of the macOS app menu: *Quit Firepixel* runs the Quit action (save
/// prompt), *About Firepixel* the About command; the system items (Services, Hide …) need none.
#[test]
fn app_menu_items_route_to_actions() {
    use fp_ui::app_menu::{ABOUT, QUIT, action_for};
    use fp_ui::app_state::Action;
    assert_eq!(action_for(QUIT), Some(Action::Quit));
    assert!(matches!(action_for(ABOUT), Some(Action::Run(c, _)) if c == "help.about"));
    assert_eq!(action_for("unknown"), None);
    // the menu labels exist in both languages (checked by fp-i18n) and are not empty
    for key in ["ui-macmenu-about", "ui-macmenu-quit", "ui-macmenu-hide"] {
        assert!(!fp_i18n::t(key).is_empty() && fp_i18n::t(key) != key, "{key}");
    }
}

/// FB-05: a menu click is no window event; queuing it requests a frame at once, and the next frame
/// runs the item's action.
#[test]
fn app_menu_click_wakes_the_ui_and_runs_quit() {
    let ctx = egui::Context::default();
    let mut app = fp_ui::FirepixelApp::with_session(
        &ctx,
        fp_engine::Session::default(),
        fp_ui::app_state::UiSettings::default(),
        false,
    );
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
        fp_ui::app_menu::menu_chosen(ui.ctx(), fp_ui::app_menu::QUIT.into());
    });
    out.textures_delta.clear();
    assert!(out.viewport_output[&ViewportId::ROOT].repaint_delay.is_zero());
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
        fp_ui::app_menu::frame(ui.ctx(), &mut app.state);
        app.state.process_actions(ui.ctx());
    });
    out.textures_delta.clear();
    // nothing unsaved: Quit closes the window right away
    assert!(app.state.quit_confirmed && app.state.close_window);
}

/// FB-138: a quit request for the whole application (macOS Dock > Quit, `terminate:`, logout) reaches
/// the app as a close request of the main window (patched winit, `vendor/winit`). It must take the
/// same way as the window's close button: cancel the close, ask about unsaved documents, and on
/// *Cancel* keep running with every document open; a later quit request asks again.
#[test]
fn app_quit_request_asks_to_save_and_cancel_keeps_running() {
    use serde_json::json;
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 40, "height": 30}))
        .expect("doc");
    s.execute("draw.rectangle", json!({"rect": [1, 1, 10, 10]}))
        .expect("rect");
    let ctx = egui::Context::default();
    let app = fp_ui::FirepixelApp::with_session(
        &ctx,
        s,
        fp_ui::app_state::UiSettings::default(),
        false,
    );
    let mut h = egui_kittest::Harness::builder()
        .with_size(egui::vec2(1000.0, 700.0))
        .with_max_steps(8)
        .build_ui_state(|ui, app: &mut fp_ui::FirepixelApp| app.root_ui(ui), app);
    h.run_steps(2);
    let quit_request = |h: &mut egui_kittest::Harness<'_, fp_ui::FirepixelApp>| {
        h.input_mut()
            .viewports
            .entry(ViewportId::ROOT)
            .or_default()
            .events
            .push(egui::ViewportEvent::Close);
    };
    quit_request(&mut h);
    h.run_steps(1);
    let st = &h.state().state;
    assert!(st.dialogs.unsaved.is_some(), "save prompt open");
    assert!(!st.quit_confirmed && !st.close_window, "not quit yet");
    // Cancel: the app keeps running, the document stays open and modified
    h.state_mut().state.dialogs.unsaved = None;
    h.run_steps(2);
    let st = &h.state().state;
    assert!(!st.quit_confirmed && !st.close_window, "Cancel aborts the quit");
    assert_eq!(st.session.documents().len(), 1);
    assert!(st.session.documents()[0].is_dirty());
    // the next quit request asks again
    quit_request(&mut h);
    h.run_steps(1);
    assert!(h.state().state.dialogs.unsaved.is_some(), "asks again");
}

/// FB-138: the macOS application-terminate hook lives in the vendored winit copy; guard against a
/// dependency update that silently drops it (the hook itself can only run on macOS).
#[test]
fn vendored_winit_keeps_the_terminate_hook() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest = std::fs::read_to_string(root.join("Cargo.toml")).expect("Cargo.toml");
    assert!(
        manifest.contains("winit = { path = \"vendor/winit\" }"),
        "[patch.crates-io] winit"
    );
    let delegate =
        std::fs::read_to_string(root.join("vendor/winit/src/platform_impl/macos/app_state.rs"))
            .expect("vendored winit");
    assert!(delegate.contains("#[method(applicationShouldTerminate:)]"));
    assert!(delegate.contains("NSApplicationTerminateReply::NSTerminateCancel"));
    assert!(delegate.contains("WindowEvent::CloseRequested"));
}
