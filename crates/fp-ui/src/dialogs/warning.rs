//! Warnung „Befehl ohne passendes Ziel“ (FV-5): Meldungsfenster wie im Original (BEFUNDE EB
//! Z01–Z18, Bild `o_Z01.png`) – Titel = Programmname, Warnsymbol, Text daneben, Trennlinie,
//! links unten „Nicht mehr anzeigen“ (aus), rechts „OK“ (Standard). Rückfragen (Z10 „Popupmenü
//! hinzufügen“) zeigen stattdessen ihre Auswahlknöpfe und „Abbrechen“. Inhalt und Abschaltung
//! kommen aus fp-engine ([`fp_engine::warnings`]).

use super::{dialog_frame, t, DialogClose};
use crate::app_state::{Action, AppState};
use crate::inspector::fields;
use crate::theme::Theme;
use crate::ui_kit::{self, sp, BarButton, KitIcon};
use egui::{pos2, vec2, Rect};
use fp_engine::TargetWarning;
use serde_json::json;

/// Offene Warnung mit dem Zustand des Kästchens „Nicht mehr anzeigen“.
#[derive(Clone, Debug, PartialEq)]
pub struct WarningDialog {
    pub warning: TargetWarning,
    pub dont_show: bool,
}

impl WarningDialog {
    pub fn new(warning: TargetWarning) -> WarningDialog {
        WarningDialog { warning, dont_show: false }
    }
}

/// Zeigt die Warnung zu `name`, falls der Befehl kein Ziel hat (abgeschaltete still); `true`, wenn
/// der Befehl damit erledigt ist.
pub fn intercept(st: &mut AppState, name: &str) -> bool {
    let Some(w) = st.session.target_warning(name) else { return false };
    if !w.suppressed {
        st.dialogs.warning = Some(WarningDialog::new(w));
    }
    true
}

/// Zugriffstaste ohne Alt (Meldungsfenster wie im Original).
fn access(ui: &egui::Ui, label: &str) -> bool {
    let Some(k) = super::access_key(label) else { return false };
    !ui.ctx().text_edit_focused() && ui.input(|i| i.key_pressed(k) && !i.modifiers.ctrl)
}

enum Outcome {
    Close,
    Choice(usize),
}

pub fn show(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut d) = st.dialogs.warning.clone() else { return };
    let w = &d.warning;
    let question = !w.choices.is_empty();
    let ok = t("btn-ok");
    let cancel = t("msg-warn-choice-cancel");
    let check = t("msg-warn-dont-show");
    let mut labels: Vec<String> = w.choices.iter().map(|c| c.label.clone()).collect();
    if question {
        labels.push(cancel.clone());
    } else {
        labels.push(ok.clone());
    }
    let buttons: Vec<BarButton> = labels.iter().enumerate().map(|(i, l)| if i == 0 { BarButton::default_button(l) } else { BarButton::new(l) }).collect();
    let check_w = if w.pref.is_some() { 13.0 + 5.0 + ui_kit::text_w(ctx, &check.replace('&', ""), &Theme::current(ctx).ui_font()) + sp::L } else { 0.0 };
    let width = 380f32.max(ui_kit::button_bar_width(ctx, &[], &buttons) + check_w);
    let text_x = sp::L + 32.0 + sp::M;
    let th = super::wrapped_h(ctx, &w.text, width - text_x - sp::L);
    let content_h = th.max(32.0);
    let mut outcome = None;
    let mut toggled = None;
    let r = dialog_frame(ctx, "fp-warning", "Firepixel", vec2(width, (sp::L + content_h + sp::L + sp::BUTTON_BAR_H).ceil()), |ui, body| {
        let theme = Theme::current(ui.ctx());
        let icon = Rect::from_min_size(body.min + vec2(sp::L, sp::L), vec2(32.0, 32.0));
        ui_kit::paint_icon(ui.painter(), icon, KitIcon::Warning, &crate::icons::IconStyle::new(&theme, None, true));
        let ty = if th < 32.0 { icon.center().y - th / 2.0 } else { icon.top() };
        super::put_left(ui, Rect::from_min_size(pos2(body.left() + text_x, ty), vec2(body.width() - text_x - sp::L, th + 4.0)), egui::Label::new(w.text.as_str()).wrap());
        let bar = ui_kit::button_bar(ui, body, &[], &buttons);
        if let Some(i) = bar.right {
            outcome = Some(if question && i < w.choices.len() { Outcome::Choice(i) } else { Outcome::Close });
        }
        for (i, l) in labels.iter().enumerate() {
            if outcome.is_none() && access(ui, l) {
                outcome = Some(if question && i < w.choices.len() { Outcome::Choice(i) } else { Outcome::Close });
            }
        }
        if w.pref.is_some() {
            let y = body.bottom() - sp::M - sp::BUTTON_H;
            let cr = Rect::from_min_size(pos2(body.left() + sp::L, y), vec2(check_w - sp::L, sp::BUTTON_H));
            let text = check.replace('&', "");
            if let Some(v) = fields::checkbox(ui, "fp-warning-dont-show", cr, &text, d.dont_show, true) {
                toggled = Some(v);
            } else if access(ui, &check) {
                toggled = Some(!d.dont_show);
            }
        }
    });
    if let Some(v) = toggled {
        d.dont_show = v;
    }
    let outcome = match (outcome, r) {
        (Some(o), _) => o,
        (None, DialogClose::Cancelled) => Outcome::Close,
        _ => {
            st.dialogs.warning = Some(d);
            return;
        }
    };
    st.dialogs.warning = None;
    if d.dont_show {
        st.session.set_warning_disabled(&d.warning, true);
    }
    if let Outcome::Choice(i) = outcome {
        // Rückfrage (Popupmenü): Hotspot/Segment aus der Auswahl anlegen, dann der ursprüngliche
        // Befehl mit dem neuen Objekt; fehlt auch dafür das Ziel, kommt dessen Warnung.
        let choice = d.warning.choices[i].command.clone();
        if !intercept(st, &choice) {
            st.push(Action::Run(choice, json!({})));
            st.push(Action::Run(d.warning.command.clone(), json!({})));
        }
    }
}
