//! macOS application menu (FB-05).
//!
//! winit's default menu binds "Quit" (Cmd+Q) to `terminate:`, which ends the event loop without a
//! close request, so unsaved changes were lost (see `native_options`). Firepixel turns that menu
//! off and installs its own with muda: *About Firepixel* (the app's About dialog), *Services*,
//! *Hide Firepixel* / *Hide Others* / *Show All* (system items) and a custom *Quit Firepixel*
//! (Cmd+Q) that runs [`Action::Quit`] with the save prompt. The menu is built on the first frame
//! (the shared `NSApplication` exists then) and again after a language switch; menu clicks are
//! queued by muda's event handler, which also requests a frame, and handled in the next frame. `input::take_quit_key` stays as a fallback for Cmd+Q.
//!
//! The item ids and their actions are platform independent (and tested on every platform); only
//! building the menu is macOS-only.

use crate::app_state::{Action, AppState};

/// Menu id of *About Firepixel*.
pub const ABOUT: &str = "app.about";
/// Menu id of *Quit Firepixel*.
pub const QUIT: &str = "app.quit";

/// Action of a custom app-menu item (the system items need none).
pub fn action_for(id: &str) -> Option<Action> {
    match id {
        ABOUT => Some(Action::Run("help.about".into(), serde_json::json!({}))),
        QUIT => Some(Action::Quit),
        _ => None,
    }
}

/// Menu items chosen since the last frame. muda's event handler (AppKit, main thread) appends here
/// and asks egui for a frame: a menu click is no window event, so without the repaint the action
/// would wait for the next mouse move.
static PENDING: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

/// Queue a chosen menu item and wake the UI (called from the menu event handler).
pub fn menu_chosen(ctx: &egui::Context, id: String) {
    if let Ok(mut p) = PENDING.lock() {
        p.push(id);
    }
    ctx.request_repaint();
}

/// Once per frame (macOS): install or rebuild the menu, then route menu clicks to actions. While a
/// modal dialog is open the clicks are ignored, like Cmd+Q in `root_ui`.
pub fn frame(ctx: &egui::Context, st: &mut AppState) {
    #[cfg(target_os = "macos")]
    mac::ensure_menu(ctx);
    #[cfg(not(target_os = "macos"))]
    let _ = ctx;
    for id in pending_ids() {
        if st.dialogs.any_open() {
            continue;
        }
        if let Some(a) = action_for(&id) {
            st.push(a);
        }
    }
}

/// Ids of the menu items chosen since the last frame.
fn pending_ids() -> Vec<String> {
    PENDING
        .lock()
        .map(|mut p| std::mem::take(&mut *p))
        .unwrap_or_default()
}

#[cfg(target_os = "macos")]
mod mac {
    use super::{ABOUT, QUIT};
    use muda::accelerator::{Accelerator, Code, Modifiers};
    use muda::{Menu, MenuItem, PredefinedMenuItem, Submenu};
    use std::cell::RefCell;

    thread_local! {
        /// Language generation the menu was built for, and the menu (kept alive while installed).
        static MENU: RefCell<Option<(u64, Option<Menu>)>> = const { RefCell::new(None) };
    }

    /// Build the menu on the first frame and after a language switch (UI thread = main thread).
    pub(super) fn ensure_menu(ctx: &egui::Context) {
        let generation = fp_i18n::generation();
        MENU.with(|cell| {
            let mut cell = cell.borrow_mut();
            if cell.as_ref().is_some_and(|(g, _)| *g == generation) {
                return;
            }
            if cell.is_none() {
                let ctx = ctx.clone();
                muda::MenuEvent::set_event_handler(Some(move |ev: muda::MenuEvent| {
                    super::menu_chosen(&ctx, ev.id.0);
                }));
            }
            let menu = match build() {
                Ok(menu) => {
                    menu.init_for_nsapp();
                    Some(menu)
                }
                Err(e) => {
                    log::warn!("macOS app menu not installed: {e}");
                    None
                }
            };
            *cell = Some((generation, menu));
        });
    }

    fn build() -> muda::Result<Menu> {
        let t = fp_i18n::t;
        let app = Submenu::with_items(
            "Firepixel",
            true,
            &[
                &MenuItem::with_id(ABOUT, t("ui-macmenu-about"), true, None),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::services(Some(&t("ui-macmenu-services"))),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::hide(Some(&t("ui-macmenu-hide"))),
                &PredefinedMenuItem::hide_others(Some(&t("ui-macmenu-hide-others"))),
                &PredefinedMenuItem::show_all(Some(&t("ui-macmenu-show-all"))),
                &PredefinedMenuItem::separator(),
                &MenuItem::with_id(
                    QUIT,
                    t("ui-macmenu-quit"),
                    true,
                    Some(Accelerator::new(Modifiers::META, Code::KeyQ)),
                ),
            ],
        )?;
        let menu = Menu::new();
        menu.append(&app)?;
        Ok(menu)
    }
}
