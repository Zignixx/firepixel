//! Ungespeichert-Abfrage beim Schließen und Beenden (Ja / Nein / Abbrechen wie das Original).

use super::{dialog_frame, push_button, DialogClose};
use crate::app_state::AppState;
use egui::{pos2, vec2, Rect};
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
fn access(ui: &egui::Ui, k: egui::Key) -> bool {
    !ui.ctx().text_edit_focused() && ui.input(|i| i.key_pressed(k) && !i.modifiers.ctrl)
}

/// Zeigt die Abfrage für das erste Dokument der Warteschlange.
pub fn show(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut flow) = st.dialogs.unsaved.clone() else { return };
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
    let title = st.session.document(id).map(|d| d.title.clone()).unwrap_or_default();
    let mut answer: Option<u8> = None;
    let r = dialog_frame(ctx, "fp-unsaved", "Firepixel", vec2(420.0, 120.0), |ui, body| {
        let theme = crate::theme::Theme::current(ui.ctx());
        crate::icons::paint(ui.painter(), Rect::from_min_size(body.min + vec2(18.0, 20.0), vec2(28.0, 28.0)), crate::icons::Icon::Info, &crate::icons::IconStyle::new(&theme, None, true));
        super::put_left(
            ui,
            Rect::from_min_size(body.min + vec2(60.0, 20.0), vec2(body.width() - 76.0, 44.0)),
            egui::Label::new(format!("Änderungen an {} speichern?", without_last_extension(&title))).wrap(),
        );
        let y = body.bottom() - 34.0;
        if push_button(ui, Rect::from_min_size(pos2(body.right() - 252.0, y), vec2(76.0, 23.0)), "&Ja", true, true) || access(ui, egui::Key::J) {
            answer = Some(0);
        }
        if push_button(ui, Rect::from_min_size(pos2(body.right() - 170.0, y), vec2(76.0, 23.0)), "&Nein", false, true) || access(ui, egui::Key::N) {
            answer = Some(1);
        }
        if push_button(ui, Rect::from_min_size(pos2(body.right() - 88.0, y), vec2(76.0, 23.0)), "Abbrechen", false, true) {
            answer = Some(2);
        }
    });
    if r == DialogClose::Cancelled {
        answer = Some(2);
    }
    match answer {
        Some(0) => {
            let _ = st.session.activate(id);
            if crate::files::save_active(st, false) {
                close_doc(st, id);
                flow.queue.remove(0);
                st.dialogs.unsaved = Some(flow);
            } else {
                // Speichern abgebrochen → ganzen Ablauf abbrechen
                st.dialogs.unsaved = None;
            }
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
