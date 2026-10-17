//! „Über Firepixel“ (Hilfe-Menü, eigener Inhalt statt des Dialogs des Originals). Gestaltung nach
//! `docs\DESIGN.md` §3 (Info-Fenster): Kopfbanner wie der Startbildschirm (Logo, Name, Version),
//! Beschreibung, Knopfleiste.

use super::{DialogClose, dialog_frame, t};
use crate::app_state::AppState;
use crate::theme::Theme;
use crate::ui_kit::{self, BarButton, sp};
use egui::{Rect, pos2, vec2};

const W: f32 = 400.0;
const BANNER_H: f32 = 88.0;

/// Zeigt den Dialog.
pub fn show(ctx: &egui::Context, st: &mut AppState) {
    if !st.dialogs.about {
        return;
    }
    let mut close = false;
    let desc = t("dlg-about-text");
    let version = fp_i18n::tr!("dlg-about-version", version = env!("CARGO_PKG_VERSION"));
    // längere Beschreibung (andere Sprachen): umbrechen, Dialog wächst nach unten
    let dh = super::wrapped_h(ctx, &desc, W - 2.0 * sp::L);
    let h = (BANNER_H + sp::L + dh + sp::L + sp::BUTTON_BAR_H).ceil();
    let ok = t("btn-ok");
    let r = dialog_frame(
        ctx,
        "fp-about",
        &t("dlg-about-title"),
        vec2(W, h),
        |ui, body| {
            let theme = Theme::current(ui.ctx());
            let banner = Rect::from_min_size(body.min, vec2(body.width(), BANNER_H));
            crate::welcome::paint_banner(
                &ui.painter().with_clip_rect(banner),
                banner,
                &theme,
                true,
                &version,
            );
            super::text_block(
                ui,
                Rect::from_min_size(
                    pos2(body.left() + sp::L, banner.bottom() + sp::L),
                    vec2(body.width() - 2.0 * sp::L, dh + 4.0),
                ),
                egui::RichText::new(desc.as_str())
                    .font(theme.ui_font())
                    .color(theme.colors.text),
            );
            if ui_kit::button_bar(ui, body, &[], &[BarButton::default_button(&ok)])
                .right
                .is_some()
            {
                close = true;
            }
        },
    );
    if close || r == DialogClose::Cancelled {
        st.dialogs.about = false;
    }
}
