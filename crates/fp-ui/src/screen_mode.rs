//! Screen modes (toolbox ▸ View, toggle key `F`, canvas context menu "Exit Full Screen Mode"),
//! TW-158: 0 Standard, 1 full screen with menus, 2 full screen.
//!
//! * Standard: the normal window with every bar and panel.
//! * Full screen with menus: borderless over the monitor, menu bar and main toolbar stay, no
//!   document tabs; the toolbox floats at the left edge, the right dock is collapsed to an icon
//!   column floating at the right edge, Properties stays docked at the bottom; the document area
//!   spans the full width below the floating panels.
//! * Full screen: borderless over the monitor and only the canvas on black – no menu bar, toolbar,
//!   toolbox, panels, document tabs, preview bar, status bar, rulers or scroll bars; the page is
//!   centred on the monitor at the unchanged zoom, a 1 px edge at its top/left and no shadow.
//!
//! Esc does not leave a full-screen mode.

use crate::app_state::AppState;
use crate::theme::Theme;
use crate::workspace::DockMode;
use egui::{Color32, Id, Rect, pos2, vec2};

/// Standard screen mode.
pub const STANDARD: u8 = 0;
/// Full screen with menus.
pub const FULL_WITH_MENUS: u8 = 1;
/// Full screen (canvas only).
pub const FULL: u8 = 2;

/// Pasteboard colour in full-screen mode.
pub const FULL_PASTEBOARD: Color32 = Color32::BLACK;
/// Page edge (top and left, 1 px) in full-screen mode.
pub const PAGE_EDGE: Color32 = Color32::from_rgb(120, 120, 120);

/// Which parts of the main window a screen mode shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chrome {
    /// Application bar with the menus.
    pub app_bar: bool,
    /// Main toolbar (still subject to the workspace setting).
    pub toolbar: bool,
    /// Toolbox docked at the left edge (Standard).
    pub tools_docked: bool,
    /// Toolbox floating above the document area (full screen with menus).
    pub tools_floating: bool,
    /// Right panel dock docked (Standard).
    pub dock_docked: bool,
    /// Right panel dock as an icon column floating above the document area.
    pub dock_icons_floating: bool,
    /// Properties panel docked at the bottom.
    pub inspector: bool,
    /// Floating panel groups.
    pub floating_panels: bool,
    /// Document tabs.
    pub doc_tabs: bool,
    /// Preview bar (Original / Preview / 2-Up / 4-Up, page).
    pub view_bar: bool,
    /// Document status bar.
    pub footer: bool,
    /// Rulers (still subject to the view setting).
    pub rulers: bool,
    /// Scroll bars of the canvas.
    pub scrollbars: bool,
    /// Pasteboard black, page without shadow, page edge at top/left.
    pub black: bool,
}

/// Window parts shown in screen mode `mode`.
pub fn chrome(mode: u8) -> Chrome {
    match mode {
        FULL_WITH_MENUS => Chrome {
            app_bar: true,
            toolbar: true,
            tools_docked: false,
            tools_floating: true,
            dock_docked: false,
            dock_icons_floating: true,
            inspector: true,
            floating_panels: true,
            doc_tabs: false,
            view_bar: true,
            footer: true,
            rulers: true,
            scrollbars: true,
            black: false,
        },
        FULL => Chrome {
            app_bar: false,
            toolbar: false,
            tools_docked: false,
            tools_floating: false,
            dock_docked: false,
            dock_icons_floating: false,
            inspector: false,
            floating_panels: false,
            doc_tabs: false,
            view_bar: false,
            footer: false,
            rulers: false,
            scrollbars: false,
            black: true,
        },
        _ => Chrome {
            app_bar: true,
            toolbar: true,
            tools_docked: true,
            tools_floating: false,
            dock_docked: true,
            dock_icons_floating: false,
            inspector: true,
            floating_panels: true,
            doc_tabs: true,
            view_bar: true,
            footer: true,
            rulers: true,
            scrollbars: true,
            black: false,
        },
    }
}

/// Window parts of the current screen mode.
pub fn current(st: &AppState) -> Chrome {
    chrome(st.session.ui.screen_mode)
}

/// Once per frame before the layout: full screen without any open document would leave a black
/// window without menus and no way back (`F` needs a document), so it falls back to Standard.
pub fn frame(st: &mut AppState) {
    if st.session.ui.screen_mode == FULL && st.session.documents().is_empty() {
        st.session.ui.screen_mode = STANDARD;
        st.session.emit(fp_engine::SessionEvent::PanelsChanged);
    }
}

/// Full screen with menus: the toolbox floating at the left and the dock as an icon column at the
/// right edge of `area` (the document area), both above the canvas.
pub fn show_floating_chrome(ctx: &egui::Context, st: &mut AppState, area: Rect) {
    let ch = current(st);
    let theme = Theme::current(ctx);
    let m = theme.metrics.clone();
    if ch.tools_floating && st.settings.workspace.show_tools && !crate::batch::is_front(st) {
        let r = Rect::from_min_size(area.min, vec2(m.tools_width, area.height()));
        egui::Area::new(Id::new("fp-tools-floating"))
            .order(egui::Order::Middle)
            .fixed_pos(r.min)
            .show(ctx, |ui| {
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r));
                crate::tools_panel::show(&mut child, st);
                ui.allocate_rect(r, egui::Sense::hover());
            });
    }
    if ch.dock_icons_floating && !crate::batch::is_front(st) {
        let w = 38.0 + m.splitter;
        let r = Rect::from_min_size(pos2(area.right() - w, area.top()), vec2(w, area.height()));
        egui::Area::new(Id::new("fp-dock-floating"))
            .order(egui::Order::Middle)
            .fixed_pos(r.min)
            .show(ctx, |ui| {
                let inner = Rect::from_min_max(pos2(r.left() + m.splitter, r.top()), r.max);
                crate::widgets::vsplitter(ui.painter(), r.left(), r.y_range(), &theme);
                // the dock collapses to icons in this mode without changing the saved workspace
                let saved = st.settings.workspace.dock_mode;
                st.settings.workspace.dock_mode = DockMode::Iconic;
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner));
                crate::docking::show_dock(&mut child, st);
                st.settings.workspace.dock_mode = saved;
                ui.allocate_rect(r, egui::Sense::hover());
            });
    }
}
