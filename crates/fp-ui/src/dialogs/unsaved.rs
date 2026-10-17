//! Ungespeichert-Abfrage beim Schließen und Beenden (Ja / Nein / Abbrechen wie das Original).

use super::{DialogClose, dialog_frame, push_button, t};
use crate::app_state::AppState;
use egui::{Rect, vec2};
use fp_engine::DocId;
use serde_json::json;

/// Was nach den Abfragen passiert.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PendingClose {
    /// Dokumente schließen.
    CloseDocs,
    /// Programm beenden.
    Quit,
}

/// Ablauf über mehrere Dokumente.
#[derive(Clone, Debug, PartialEq)]
pub struct UnsavedFlow {
    pub queue: Vec<DocId>,
    pub mode: PendingClose,
}

impl UnsavedFlow {
    /// Neuer Ablauf.
    pub fn new(queue: Vec<DocId>, mode: PendingClose) -> UnsavedFlow {
        UnsavedFlow { queue, mode }
    }
}

fn close_doc(st: &mut AppState, id: DocId) {
    if st.session.activate(id).is_ok() {
        let _ = st.session.execute("file.close", json!({ "discard": true }));
    }
}

fn finish(st: &mut AppState, mode: PendingClose) {
    st.dialogs.unsaved = None;
    if mode == PendingClose::Quit {
        st.quit_confirmed = true;
        st.close_window = true;
    }
}

/// Dokumentname ohne letzte Endung, wie im Text des Originals („uxb_doc1.fw.png“ → „uxb_doc1.fw“).
pub fn without_last_extension(name: &str) -> &str {
    match name.rfind('.') {
        Some(i) if i > 0 => &name[..i],
        _ => name,
    }
}

/// Zugriffstaste ohne Alt (Windows-Meldungsfenster), nur ohne Textfeldfokus.
fn access(ui: &egui::Ui, k: Option<egui::Key>) -> bool {
    let Some(k) = k else { return false };
    !ui.ctx().text_edit_focused() && ui.input(|i| i.key_pressed(k) && !i.modifiers.ctrl)
}

/// Zeigt die Abfrage für das erste Dokument der Warteschlange.
pub fn show(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut flow) = st.dialogs.unsaved.clone() else {
        return;
    };
    // nicht geänderte Dokumente ohne Abfrage schließen
    while let Some(&id) = flow.queue.first() {
        match st.session.document(id) {
            Some(d) if d.is_dirty() => break,
            Some(_) => {
                close_doc(st, id);
                flow.queue.remove(0);
            }
            None => {
                flow.queue.remove(0);
            }
        }
    }
    let Some(&id) = flow.queue.first() else {
        finish(st, flow.mode);
        return;
    };
    let title = st
        .session
        .document(id)
        .map(|d| d.title.clone())
        .unwrap_or_default();
    let mut answer: Option<u8> = None;
    let (yes, no, cancel) = (t("btn-yes"), t("btn-no"), t("btn-cancel"));
    // wie das Original ohne die letzte Endung („par_layers.fw“); CT-C-07 ist Original-Verhalten
    let text = fp_i18n::tr!("dlg-unsaved-text", name = without_last_extension(&title));
    let row_w = super::button_row_w(ctx, &[&yes, &no, &cancel], 76.0, 6.0);
    let w = 420f32.max(row_w + 24.0);
    // langer Text (Dateiname, andere Sprachen): Höhe wächst mit dem Umbruch
    let th = super::wrapped_h(ctx, &text, w - 76.0);
    let r = dialog_frame(
        ctx,
        "fp-unsaved",
        "Firepixel",
        vec2(w, 120f32.max(20.0 + th + 56.0).ceil()),
        |ui, body| {
            let theme = crate::theme::Theme::current(ui.ctx());
            // Gestaltung (docs\DESIGN.md §3, Rückfrage): Warnsymbol 32 px, kurzer Text mittig daneben
            let icon = Rect::from_min_size(body.min + vec2(16.0, 18.0), vec2(32.0, 32.0));
            crate::ui_kit::paint_icon(
                ui.painter(),
                icon,
                crate::ui_kit::KitIcon::Warning,
                &crate::icons::IconStyle::new(&theme, None, true),
            );
            let ty = if th < icon.height() {
                icon.center().y - th / 2.0
            } else {
                icon.top()
            };
            super::put_left(
                ui,
                Rect::from_min_size(
                    egui::pos2(body.left() + 60.0, ty),
                    vec2(body.width() - 76.0, th.max(20.0) + 4.0),
                ),
                egui::Label::new(text.as_str()).wrap(),
            );
            // Linie und Fläche über den Knöpfen zeichnet der Dialograhmen als Fußleiste (DES-1)
            let b = super::button_row(
                ui.ctx(),
                body.right() - 12.0,
                body.bottom() - 34.0,
                &[&yes, &no, &cancel],
                76.0,
                23.0,
                6.0,
            );
            if push_button(ui, b[0], &yes, true, true) || access(ui, super::access_key(&yes)) {
                answer = Some(0);
            }
            if push_button(ui, b[1], &no, false, true) || access(ui, super::access_key(&no)) {
                answer = Some(1);
            }
            if push_button(ui, b[2], &cancel, false, true) || access(ui, super::access_key(&cancel))
            {
                answer = Some(2);
            }
        },
    );
    if r == DialogClose::Cancelled {
        answer = Some(2);
    }
    match answer {
        Some(0) => {
            let _ = st.session.activate(id);
            // Abfrage ausblenden, solange „Speichern unter“ offen ist (Dateidialog blockiert nicht)
            st.dialogs.unsaved = None;
            crate::files::save_active(st, false, move |st, ok| {
                if ok {
                    close_doc(st, id);
                    flow.queue.remove(0);
                    st.dialogs.unsaved = Some(flow);
                } else {
                    // Speichern abgebrochen → ganzen Ablauf abbrechen
                    st.dialogs.unsaved = None;
                }
            });
        }
        Some(1) => {
            close_doc(st, id);
            flow.queue.remove(0);
            st.dialogs.unsaved = Some(flow);
        }
        Some(_) => st.dialogs.unsaved = None,
        None => st.dialogs.unsaved = Some(flow),
    }
}
