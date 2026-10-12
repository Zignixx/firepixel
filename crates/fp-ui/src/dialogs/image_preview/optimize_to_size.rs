//! „Für Dateigröße optimieren“ (Knopf „Assistent für Dateigrößenoptimierung“, re\17-bildvorschau.md
//! §2.8): Zielgröße in K (vorbelegt mit der aktuellen Größe des aktiven Feldes, eine
//! Nachkommastelle), danach Bisektion über Farbanzahl bzw. JPEG-Qualität (`fp_export::doc_export::
//! optimize_whole_to_size`, 08 §7) im Hintergrund. Liegt das Ziel unter der kleinstmöglichen
//! Größe: Meldung „Die resultierende Datei wäre zu klein …“, die Werte bleiben.

use super::super::{dialog_frame, push_button, DialogClose};
use crate::panels::optimize::ExportRenderer;
use egui::{pos2, vec2, Rect};
use fp_export::doc_export::SizeOutcome;
use fp_export::options::OptimizeSettings;
use fp_i18n::t;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Arc;

/// Kennung des Dialogs.
pub const DIALOG_ID: &str = "fp-imgpreview-size";
/// Kennung des Meldungsfensters der Bildvorschau.
pub const MESSAGE_ID: &str = "fp-imgpreview-message";

/// Offener Dialog „Für Dateigröße optimieren“.
#[derive(Clone, Debug, PartialEq)]
pub struct SizeDialog {
    pub pane: usize,
    /// Eingabe in K.
    pub text: String,
}

/// Vorbelegung: aktuelle Größe in K (1 K = 1000 Byte) mit einer Nachkommastelle, gerundet aus der
/// angezeigten Größe („53.9“ bei „53.89K“).
pub fn prefill(bytes: usize) -> String {
    let hundredths = (bytes + 5) / 10;
    format!("{:.1}", (hundredths as f64 / 10.0).round() / 10.0)
}

/// Zielgröße in Byte aus der Eingabe (Punkt oder Komma).
pub fn parse_target(s: &str) -> Option<usize> {
    let v: f64 = s.trim().trim_end_matches(['k', 'K']).trim().replace(',', ".").parse().ok()?;
    (v.is_finite() && v > 0.0).then(|| (v * 1000.0).round() as usize)
}

/// Laufende Suche.
pub struct SizeJob {
    pub pane: usize,
    /// Einstellung beim Start (ändert der Nutzer das Feld inzwischen, wird das Ergebnis verworfen).
    pub start: OptimizeSettings,
    cancel: Arc<AtomicBool>,
    rx: Receiver<Result<SizeOutcome, String>>,
}

impl Drop for SizeJob {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::SeqCst);
    }
}

impl SizeJob {
    /// Startet die Suche für das Feld `pane` mit der Einstellung `s` und dem Ziel `target` Byte.
    pub fn spawn(ctx: &egui::Context, base: &super::worker::JobInput, pane: usize, s: OptimizeSettings, target: usize) -> SizeJob {
        let (tx, rx) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let (doc, renderer, page, frame) = (Arc::clone(&base.doc), Arc::clone(&base.renderer), base.page, base.frame);
        let (s2, c2, ctx2) = (s.clone(), Arc::clone(&cancel), ctx.clone());
        let spawned = std::thread::Builder::new().name("fp-image-preview-size".into()).spawn(move || {
            let r = ExportRenderer { normal: renderer };
            let res = fp_export::doc_export::optimize_whole_to_size(&r, &doc, page, frame, &s2, target, &fp_export::export::EncodeExtras::default()).map_err(|e| e.to_string());
            if !c2.load(Ordering::SeqCst) {
                let _ = tx.send(res);
            }
            ctx2.request_repaint();
        });
        if let Err(e) = spawned {
            log::warn!("optimize to size: {e}");
        }
        SizeJob { pane, start: s, cancel, rx }
    }

    /// Fertiges Ergebnis (`Some(Err)` auch, wenn der Thread ohne Antwort endete).
    pub fn poll(&self) -> Option<Result<SizeOutcome, String>> {
        match self.rx.try_recv() {
            Ok(r) => Some(r),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Err(String::new())),
        }
    }
}

/// Ergebnis des Dialogs.
pub enum SizeDialogOutcome {
    Open(SizeDialog),
    /// OK mit Ziel in Byte.
    Ok(usize, usize),
    Cancel,
}

/// Dialog „Für Dateigröße optimieren“: „Zielgröße:“ Feld, Einheit „k“, OK/Abbrechen.
pub fn dialog(ctx: &egui::Context, mut d: SizeDialog) -> SizeDialogOutcome {
    let (title, label, unit, ok_l, cancel_l) = (t("image-preview-size-title"), t("image-preview-size-target"), t("image-preview-size-unit"), t("btn-ok"), t("btn-cancel"));
    let dx = super::super::extra_w(ctx, &[&label], 70.0);
    let mut result = None;
    let r = dialog_frame(ctx, DIALOG_ID, &title, vec2(279.0 + dx, 114.0), |ui, body| {
        super::super::fit_label(ui, Rect::from_min_size(body.min + vec2(14.0, 16.0), vec2(70.0 + dx, 20.0)), &label, true);
        let te = ui.put(Rect::from_min_size(body.min + vec2(88.0 + dx, 14.0), vec2(80.0, 22.0)), egui::TextEdit::singleline(&mut d.text));
        super::super::autofocus_field(ui, &te, d.text.chars().count());
        te.request_focus();
        let theme = crate::theme::Theme::current(ui.ctx());
        ui.painter().text(pos2(body.left() + 174.0 + dx, body.top() + 25.0), egui::Align2::LEFT_CENTER, unit.as_str(), theme.ui_font(), theme.colors.text);
        let valid = parse_target(&d.text).is_some();
        let b = super::super::button_row(ui.ctx(), body.right() - 14.0, body.bottom() - 34.0, &[&ok_l, &cancel_l], 76.0, 23.0, 8.0);
        if push_button(ui, b[0], &ok_l, true, valid) || (valid && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
            result = Some(true);
        }
        if push_button(ui, b[1], &cancel_l, false, true) {
            result = Some(false);
        }
    });
    match (result, r) {
        (Some(true), _) => match parse_target(&d.text) {
            Some(n) => SizeDialogOutcome::Ok(d.pane, n),
            None => SizeDialogOutcome::Open(d),
        },
        (Some(false), _) | (_, DialogClose::Cancelled) => SizeDialogOutcome::Cancel,
        _ => SizeDialogOutcome::Open(d),
    }
}

/// Meldungsfenster der Bildvorschau (z. B. „Die resultierende Datei wäre zu klein …“); `true` =
/// geschlossen.
pub fn message(ctx: &egui::Context, text: &str) -> bool {
    let ok_l = t("btn-ok");
    let w = 360.0;
    let th = super::super::wrapped_h(ctx, text, w - 32.0);
    let mut done = false;
    let r = dialog_frame(ctx, MESSAGE_ID, &t("dlg-preview-title"), vec2(w, th + 80.0), |ui, body| {
        super::super::text_block(ui, Rect::from_min_size(body.min + vec2(16.0, 14.0), vec2(w - 32.0, th + 4.0)), text);
        let b = super::super::button_row(ui.ctx(), body.right() - 14.0, body.bottom() - 34.0, &[&ok_l], 76.0, 23.0, 8.0);
        if push_button(ui, b[0], &ok_l, true, true) || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            done = true;
        }
    });
    done || r == DialogClose::Cancelled
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefill_and_parse_follow_spec() {
        assert_eq!(prefill(53_890), "53.9", "53.89K → 53.9");
        assert_eq!(prefill(19_890), "19.9");
        assert_eq!(prefill(460_854), "460.9");
        assert_eq!(parse_target("20"), Some(20_000));
        assert_eq!(parse_target("19,5"), Some(19_500));
        assert_eq!(parse_target(" 7.25 k"), Some(7_250));
        assert_eq!(parse_target("0"), None);
        assert_eq!(parse_target("x"), None);
    }
}
