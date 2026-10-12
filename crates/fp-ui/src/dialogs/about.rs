//! „Über Firepixel“ (Hilfe-Menü, eigener Inhalt statt des Dialogs des Originals).

use super::{dialog_frame, push_button, t, DialogClose};
use crate::app_state::AppState;
use crate::theme::Theme;
use egui::{pos2, vec2, Rect};

/// Zeigt den Dialog.
pub fn show(ctx: &egui::Context, st: &mut AppState) {
    if !st.dialogs.about {
        return;
    }
    let mut close = false;
    let desc = t("dlg-about-text");
    // längere Beschreibung (andere Sprachen): umbrechen, Dialog wächst nach unten
    let dh = super::wrapped_h(ctx, &desc, 380.0 - 100.0);
    let r = dialog_frame(ctx, "fp-about", &t("dlg-about-title"), vec2(380.0, 170.0f32.max(74.0 + dh + 50.0).ceil()), |ui, body| {
        let theme = Theme::current(ui.ctx());
        let st_icon = crate::icons::IconStyle::new(&theme, Some(crate::theme::ToolGroup::Select), true);
        crate::icons::paint(ui.painter(), Rect::from_min_size(body.min + vec2(20.0, 20.0), vec2(48.0, 48.0)), crate::icons::Icon::Logo, &st_icon);
        ui.painter().text(body.min + vec2(84.0, 26.0), egui::Align2::LEFT_TOP, "Firepixel", theme.bold(20.0), theme.colors.text);
        ui.painter().text(body.min + vec2(84.0, 54.0), egui::Align2::LEFT_TOP, fp_i18n::tr!("dlg-about-version", version = env!("CARGO_PKG_VERSION")), theme.ui_font(), theme.colors.text_weak);
        super::text_block(ui, Rect::from_min_size(body.min + vec2(84.0, 74.0), vec2(body.width() - 100.0, dh + 4.0)), egui::RichText::new(desc.as_str()).font(theme.ui_font()).color(theme.colors.text));
        let ok = t("btn-ok");
        let w = super::button_w(ui.ctx(), &ok, 76.0);
        if push_button(ui, Rect::from_min_size(pos2(body.right() - 16.0 - w, body.bottom() - 34.0), vec2(w, 23.0)), &ok, true, true) {
            close = true;
        }
    });
    if close || r == DialogClose::Cancelled {
        st.dialogs.about = false;
    }
}
