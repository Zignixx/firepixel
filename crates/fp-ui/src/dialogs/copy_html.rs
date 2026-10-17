//! Assistent „HTML-Code kopieren“ (Vorlage 6046, TW-22): drei Seiten wie im Original – HTML-Stil
//! (6047) mit Einführungstext, Basis-Dateiname mit „HTML-Einrichtung...“ (19116), Bilderordner mit
//! „Durchsuchen...“ (19118). Knöpfe Abbrechen · < Zurück · Weiter > · Fertig stellen; „Fertig
//! stellen“ exportiert die Bilder in den Ordner und kopiert den Code in die Zwischenablage
//! (`edit.copyHtmlCode` mit `dir`, `baseName`, `xhtml`). Ohne gewählten Ordner fragt „Fertig
//! stellen“ zuerst nach dem Ordner.

use super::{DialogClose, dialog_frame, push_button, refs, t};
use crate::app_state::AppState;
use crate::inspector::fields;
use crate::theme::Theme;
use egui::{Rect, Stroke, StrokeKind, pos2, vec2};

/// Letzte Seite (0 Stil, 1 Basisname, 2 Bilderordner).
pub const LAST_PAGE: usize = 2;

/// Zustand des Assistenten.
#[derive(Clone, Debug, PartialEq)]
pub struct CopyHtmlWizard {
    pub page: usize,
    /// HTML-Stil: `false` Allgemeines HTML, `true` Allgemeines XHTML (Vorgabe aus der HTML-Einrichtung).
    pub xhtml: bool,
    pub base_name: String,
    /// Ordner für die Bilder (leer: „Fertig stellen“ fragt danach).
    pub folder: String,
    /// „Durchsuchen...“ gedrückt (Ordnerwahl startet [`show`]).
    browse: bool,
    /// „HTML-Einrichtung...“ gedrückt.
    setup: bool,
}

impl CopyHtmlWizard {
    /// Vorgaben aus dem aktiven Dokument; `None` ohne Dokument.
    pub fn new(st: &AppState) -> Option<CopyHtmlWizard> {
        let d = st.session.active()?;
        let xhtml = d
            .doc
            .pages
            .get(d.page)
            .and_then(|p| p.root.export.as_ref())
            .and_then(|e| e.html.as_deref())
            .and_then(|h| h.xhtml)
            .unwrap_or(1)
            != 0;
        let name = d
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| d.title.clone());
        let folder = d
            .path
            .as_ref()
            .and_then(|p| p.parent())
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        Some(CopyHtmlWizard {
            page: 0,
            xhtml,
            base_name: super::export::file_stem(&name),
            folder,
            browse: false,
            setup: false,
        })
    }

    /// Parameter für `edit.copyHtmlCode`.
    pub fn params(&self) -> serde_json::Value {
        let mut p = serde_json::json!({"style": "htmlAndImages", "xhtml": self.xhtml, "baseName": self.base_name.trim()});
        if !self.folder.trim().is_empty() {
            p["dir"] = serde_json::json!(self.folder.trim());
        }
        p
    }
}

/// Öffnet den Assistenten (Bearbeiten ▸ HTML-Code kopieren...).
pub fn open(st: &mut AppState) -> bool {
    st.dialogs.copy_html = CopyHtmlWizard::new(st);
    true
}

/// Kopiert mit den Werten des Assistenten (Erfolg in der Statuszeile, Fehler als Meldung).
pub fn finish(st: &mut AppState, w: &CopyHtmlWizard) {
    super::menu_dialogs::copy_html_with(st, w.params());
}

/// Zeigt den Assistenten.
pub fn show(ctx: &egui::Context, st: &mut AppState) {
    // während „HTML-Einrichtung“ darüber offen ist, bleibt der Assistent stehen (wie das Original)
    let Some(mut w) = st.dialogs.copy_html.take() else {
        return;
    };
    let ans = show_dialog(ctx, &mut w);
    let (browse, setup) = (std::mem::take(&mut w.browse), std::mem::take(&mut w.setup));
    match ans {
        Some(true) if w.folder.trim().is_empty() => {
            // ohne Ordner: erst den Ordner wählen, dann kopieren
            crate::file_dialog::request(
                st,
                crate::file_dialog::Request::folder(t("dlg-md-copy-html-folder")),
                move |st, r| {
                    if let Some(p) = r.into_iter().next() {
                        let mut w = w;
                        w.folder = p.to_string_lossy().to_string();
                        finish(st, &w);
                    }
                },
            );
        }
        Some(true) => finish(st, &w),
        Some(false) => {}
        None => st.dialogs.copy_html = Some(w),
    }
    if ans.is_none() && browse {
        let start = st
            .dialogs
            .copy_html
            .as_ref()
            .map(|w| std::path::PathBuf::from(w.folder.trim()))
            .filter(|p| p.is_dir());
        crate::file_dialog::request(
            st,
            crate::file_dialog::Request::folder(t("dlg-md-copy-html-folder")).dir(start.as_deref()),
            |st, r| {
                if let (Some(p), Some(w)) = (r.into_iter().next(), st.dialogs.copy_html.as_mut()) {
                    w.folder = p.to_string_lossy().to_string();
                }
            },
        );
    }
    if ans.is_none() && setup && st.dialogs.html_setup.is_none() {
        super::html_setup::open(st);
    }
}

/// Zeichnet den Assistenten; `Some(true)` Fertig stellen, `Some(false)` Abbrechen/Esc.
pub fn show_dialog(ctx: &egui::Context, w: &mut CopyHtmlWizard) -> Option<bool> {
    let (cancel_l, back_l, next_l, finish_l) = (
        t("btn-cancel"),
        t("dlg-wiz-nav-back"),
        t("dlg-wiz-nav-next"),
        t("dlg-wiz-nav-finish"),
    );
    let labels = [
        cancel_l.as_str(),
        back_l.as_str(),
        next_l.as_str(),
        finish_l.as_str(),
    ];
    // Vorlage 294 × 177 DLU ≈ 441 × 288 px; Knöpfe 63 DLU breit, in längeren Sprachen breiter
    let bw = super::buttons_w(ctx, &labels, 94.0);
    let width = 441f32.max(4.0 * bw + 3.0 * 8.0 + 24.0).ceil();
    let mut ans = None;
    let c = dialog_frame(
        ctx,
        "fp-copy-html",
        &t("dlg-md-copy-html"),
        vec2(width, 288.0),
        |ui, body| {
            let theme = Theme::current(ui.ctx());
            let col = &theme.colors;
            let o = body.min + vec2(9.0, 9.0);
            let inner_w = body.width() - 18.0;
            let wrap = |ui: &mut egui::Ui, r: Rect, text: &str| {
                super::text_block(ui, r, text);
            };
            // obere Seite (6047 / 19116 / 19118)
            match w.page {
                0 => {
                    super::fit_label(
                        ui,
                        Rect::from_min_size(o, vec2(inner_w, 16.0)),
                        &t("dlg-ch-style-q"),
                        true,
                    );
                    let items = [t("dlg-hs-style-html"), t("dlg-hs-style-xhtml")];
                    if let Some(i) = fields::combo(
                        ui,
                        "ch-style",
                        Rect::from_min_size(o + vec2(0.0, 18.0), vec2(285f32.min(inner_w), 22.0)),
                        &refs(&items),
                        Some(usize::from(w.xhtml)),
                        true,
                    ) {
                        w.xhtml = i == 1;
                    }
                }
                1 => {
                    super::fit_label(
                        ui,
                        Rect::from_min_size(o, vec2(inner_w, 16.0)),
                        &t("dlg-ch-base-name"),
                        true,
                    );
                    if let Some(v) = fields::text(
                        ui,
                        "ch-base",
                        Rect::from_min_size(o + vec2(0.0, 18.0), vec2(inner_w, 22.0)),
                        &w.base_name,
                        true,
                    ) {
                        w.base_name = v;
                    }
                    super::fit_label(
                        ui,
                        Rect::from_min_size(o + vec2(0.0, 50.0), vec2(inner_w, 16.0)),
                        &t("dlg-ch-optional"),
                        true,
                    );
                    let setup_l = t("dlg-ch-html-setup");
                    let sw = super::button_w(ui.ctx(), &setup_l, 164.0);
                    if push_button(
                        ui,
                        Rect::from_min_size(o + vec2(0.0, 69.0), vec2(sw, 23.0)),
                        &setup_l,
                        false,
                        true,
                    ) {
                        w.setup = true;
                    }
                }
                _ => {
                    super::fit_label(
                        ui,
                        Rect::from_min_size(o, vec2(inner_w, 16.0)),
                        &t("dlg-ch-images"),
                        true,
                    );
                    let browse_l = t("dlg-export-browse");
                    let brw = super::button_w(ui.ctx(), &browse_l, 90.0);
                    let fr =
                        Rect::from_min_size(o + vec2(0.0, 18.0), vec2(inner_w - brw - 6.0, 22.0));
                    // schreibgeschützt wie im Original (nur „Durchsuchen...“ ändert den Ordner)
                    let _ = fields::text(ui, "ch-folder", fr, &w.folder, false);
                    if push_button(
                        ui,
                        Rect::from_min_size(
                            pos2(fr.right() + 6.0, fr.top() - 1.0),
                            vec2(brw, 23.0),
                        ),
                        &browse_l,
                        false,
                        true,
                    ) {
                        w.browse = true;
                    }
                    wrap(
                        ui,
                        Rect::from_min_size(o + vec2(0.0, 46.0), vec2(inner_w, 40.0)),
                        &t("dlg-ch-images-hint"),
                    );
                }
            }
            // Rahmen mit dem Erklärungstext der Seite (19120 / 19122 / 19121)
            let frame = Rect::from_min_max(
                pos2(body.left() + 9.0, body.top() + 110.0),
                pos2(body.right() - 9.0, body.bottom() - 48.0),
            );
            ui.painter().rect_filled(frame, 0.0, col.frame_bg);
            ui.painter().rect_stroke(
                frame,
                0.0,
                Stroke::new(1.0, col.line_mid),
                StrokeKind::Inside,
            );
            let (a, b) = match w.page {
                0 => ("dlg-ch-intro-1", "dlg-ch-intro-2"),
                1 => ("dlg-ch-base-1", "dlg-ch-base-2"),
                _ => ("dlg-ch-folder-1", "dlg-ch-folder-2"),
            };
            let tw = frame.width() - 16.0;
            let ta = t(a);
            let ha = super::wrapped_h(ui.ctx(), &ta, tw);
            wrap(
                ui,
                Rect::from_min_size(frame.min + vec2(8.0, 8.0), vec2(tw, ha)),
                &ta,
            );
            wrap(
                ui,
                Rect::from_min_size(
                    frame.min + vec2(8.0, 8.0 + ha.max(48.0) + 8.0),
                    vec2(tw, (frame.height() - ha - 24.0).max(16.0)),
                ),
                &t(b),
            );
            // Knöpfe wie in der Vorlage: Abbrechen links, < Zurück · Weiter > · Fertig stellen rechts
            let y = body.bottom() - 35.0;
            let last = w.page >= LAST_PAGE;
            if push_button(
                ui,
                Rect::from_min_size(pos2(body.left() + 12.0, y), vec2(bw, 23.0)),
                &cancel_l,
                false,
                true,
            ) {
                ans = Some(false);
            }
            let row = super::button_row(
                ui.ctx(),
                body.right() - 12.0,
                y,
                &[&back_l, &next_l, &finish_l],
                94.0,
                23.0,
                8.0,
            );
            if push_button(ui, row[0], &back_l, false, w.page > 0) {
                w.page -= 1;
            }
            if push_button(ui, row[1], &next_l, !last, !last) {
                w.page += 1;
            }
            if push_button(ui, row[2], &finish_l, last, last) {
                ans = Some(true);
            }
        },
    );
    if c == DialogClose::Cancelled {
        Some(false)
    } else {
        ans
    }
}
