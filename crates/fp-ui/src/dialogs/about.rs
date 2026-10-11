//! „Über Firepixel“ (Hilfe-Menü, eigener Inhalt statt des Dialogs des Originals).

use super::{dialog_frame, push_button, DialogClose};
use crate::app_state::AppState;
use crate::theme::Theme;
use egui::{pos2, vec2, Rect};

/// Zeigt den Dialog.
pub fn show(ctx: &egui::Context, st: &mut AppState) {
    if !st.dialogs.about {
        return;
    }
    let mut close = false;
    let r = dialog_frame(ctx, "fp-about", "Über Firepixel", vec2(380.0, 170.0), |ui, body| {
        let theme = Theme::current(ui.ctx());
        let st_icon = crate::icons::IconStyle::new(&theme, Some(crate::theme::ToolGroup::Select), true);
        crate::icons::paint(ui.painter(), Rect::from_min_size(body.min + vec2(20.0, 20.0), vec2(48.0, 48.0)), crate::icons::Icon::Logo, &st_icon);
        ui.painter().text(body.min + vec2(84.0, 26.0), egui::Align2::LEFT_TOP, "Firepixel", theme.bold(20.0), theme.colors.text);
        ui.painter().text(body.min + vec2(84.0, 54.0), egui::Align2::LEFT_TOP, format!("Version {}", env!("CARGO_PKG_VERSION")), theme.ui_font(), theme.colors.text_weak);
        ui.painter().text(body.min + vec2(84.0, 74.0), egui::Align2::LEFT_TOP, "Grafikprogramm für Bildschirm- und Webgrafik, in Rust.", theme.ui_font(), theme.colors.text);
        if push_button(ui, Rect::from_min_size(pos2(body.right() - 92.0, body.bottom() - 34.0), vec2(76.0, 23.0)), "OK", true, true) {
            close = true;
        }
    });
    if close || r == DialogClose::Cancelled {
        st.dialogs.about = false;
    }
}
