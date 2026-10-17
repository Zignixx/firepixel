//! Eingabeaufforderung `prompt(text, vorgabe)` eines Befehlsskripts (CT-1b MN-5, FIX-B).
//!
//! Skripte laufen ohne Unterbrechung durch. Fragt ein Skript, das aus der Oberfläche gestartet
//! wurde, nach einer Eingabe, bricht die Engine den Lauf ohne Änderung am Dokument ab und meldet
//! Text und Vorgabewert (`promptNeeded`). Dieser Dialog zeigt die Frage; OK startet das Skript
//! erneut mit allen bisherigen Antworten (`prompts`), Abbrechen ebenso mit `null` (wie im Browser).
//! Ohne Oberfläche (CLI, MCP, Batch) liefert `prompt()` weiter den Vorgabewert.

use super::{DialogClose, button_row, dialog_frame, push_button, put_left, t, wrapped_h};
use crate::app_state::{Action, AppState};
use egui::{Rect, vec2};
use serde_json::Value;

/// Offene Eingabeaufforderung.
#[derive(Clone, Debug)]
pub struct ScriptPrompt {
    /// Frage des Skripts.
    pub text: String,
    /// Eingabefeld (beginnt mit dem Vorgabewert).
    pub value: String,
    /// Befehl, der das Skript ausführt (`commands.runScript`, `commands.run`).
    pub command: String,
    /// Parameter des Befehls (ohne `prompts`).
    pub params: Value,
    /// Antworten auf die vorherigen Fragen dieses Laufs.
    pub answers: Vec<Value>,
    focused: bool,
}

impl ScriptPrompt {
    pub fn new(
        command: &str,
        params: Value,
        answers: Vec<Value>,
        text: &str,
        default: Option<&str>,
    ) -> Self {
        ScriptPrompt {
            text: text.into(),
            value: default.unwrap_or_default().into(),
            command: command.into(),
            params,
            answers,
            focused: false,
        }
    }

    /// Aktion für den erneuten Lauf mit der nächsten Antwort (`None` = Abbrechen → `null`).
    pub fn rerun(&self, answer: Option<&str>) -> Action {
        let mut p = if self.params.is_object() {
            self.params.clone()
        } else {
            Value::Object(Default::default())
        };
        let mut a = self.answers.clone();
        a.push(
            answer
                .map(|s| Value::String(s.into()))
                .unwrap_or(Value::Null),
        );
        p["prompts"] = Value::Array(a);
        Action::Run(self.command.clone(), p)
    }
}

/// Zeigt die Eingabeaufforderung (Titel „Firepixel“ wie die Meldungen der Befehlsskripte).
pub fn show(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut d) = st.dialogs.script_prompt.clone() else {
        return;
    };
    let mut result: Option<bool> = None;
    let w = 400.0;
    let tw = w - 32.0;
    let th = wrapped_h(ctx, &d.text, tw).max(16.0);
    let (ok_l, cancel_l) = (t("btn-ok"), t("btn-cancel"));
    let h = (16.0 + th + 10.0 + 22.0 + 16.0 + 40.0).ceil();
    let r = dialog_frame(
        ctx,
        "fp-script-prompt",
        "Firepixel",
        vec2(w, h),
        |ui, body| {
            put_left(
                ui,
                Rect::from_min_size(body.min + vec2(16.0, 16.0), vec2(tw, th + 2.0)),
                egui::Label::new(d.text.as_str()).wrap(),
            );
            let field =
                Rect::from_min_size(body.min + vec2(16.0, 16.0 + th + 10.0), vec2(tw, 22.0));
            let te = ui.put(
                field,
                egui::TextEdit::singleline(&mut d.value)
                    .id(egui::Id::new("fp-script-prompt-value")),
            );
            if !d.focused {
                te.request_focus();
                d.focused = true;
            }
            let b = button_row(
                ui.ctx(),
                body.right() - 16.0,
                body.bottom() - 34.0,
                &[&ok_l, &cancel_l],
                76.0,
                23.0,
                8.0,
            );
            if push_button(ui, b[0], &ok_l, true, true) {
                result = Some(true);
            }
            if push_button(ui, b[1], &cancel_l, false, true) {
                result = Some(false);
            }
        },
    );
    let result = result.or((r == DialogClose::Cancelled).then_some(false));
    match result {
        Some(ok) => {
            st.dialogs.script_prompt = None;
            let a = d.rerun(ok.then_some(d.value.as_str()));
            st.push(a);
        }
        None => st.dialogs.script_prompt = Some(d),
    }
}
