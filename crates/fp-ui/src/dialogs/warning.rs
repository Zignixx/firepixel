//! Warnung „Befehl ohne passendes Ziel“ (FV-5): Meldungsfenster wie im Original (BEFUNDE EB
//! Z01–Z18, Bild `o_Z01.png`) – Titel = Programmname, Warnsymbol, Text daneben, Trennlinie,
//! links unten „Nicht mehr anzeigen“ (aus), rechts „OK“ (Standard). Rückfragen (Z10 „Popupmenü
//! hinzufügen“) zeigen stattdessen ihre Auswahlknöpfe und „Abbrechen“. Inhalt und Abschaltung
//! kommen aus fp-engine ([`fp_engine::warnings`]).

use super::{DialogClose, dialog_frame, t};
use crate::app_state::{Action, AppState};
use crate::inspector::fields;
use crate::theme::Theme;
use crate::ui_kit::{self, BarButton, KitIcon, sp};
use egui::{Rect, pos2, vec2};
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
        WarningDialog {
            warning,
            dont_show: false,
        }
    }
}

/// Zeigt die Warnung zu `name`, falls der Befehl kein Ziel hat (abgeschaltete still); `true`, wenn
/// der Befehl damit erledigt ist.
pub fn intercept(st: &mut AppState, name: &str) -> bool {
    let Some(w) = st.session.target_warning(name) else {
        return false;
    };
    if !w.suppressed {
        st.dialogs.warning = Some(WarningDialog::new(w));
    }
    true
}

/// Schlüssel der Rückfrage „Dieser Vorgang konvertiert Vektoren in Bitmaps.“ (TW-21).
pub const FILTER_VECTORS: &str = "filterVectors";
/// Voreinstellung ihres Kästchens „Nicht mehr anzeigen“ (`DisableConfirm…`: „Warndialoge
/// zurücksetzen“ schaltet sie wieder ein).
pub const FILTER_VECTORS_PREF: &str = "DisableConfirmFilterVectors";

/// Wird ein Filter (aus Menü oder Kürzel, ohne Parameter) auf eine Auswahl mit Vektorobjekten
/// angewendet, fragt das Original erst „Dieser Vorgang konvertiert Vektoren in Bitmaps.“ (OK,
/// Abbrechen, Nicht mehr anzeigen). `true`: Rückfrage offen, der Befehl wartet auf OK.
pub fn intercept_filter_vectors(st: &mut AppState, name: &str, params: &serde_json::Value) -> bool {
    if !name.starts_with("filter.") || !params.as_object().is_none_or(|o| o.is_empty()) {
        return false;
    }
    if std::mem::take(&mut st.dialogs.filter_vectors_ok) {
        return false;
    }
    let Some(d) = st.session.active() else {
        return false;
    };
    if d.selection.has_pixels() {
        return false;
    }
    let vectors = d
        .selection
        .elements
        .iter()
        .filter_map(|id| fp_engine::docops::element_on_page(&d.doc, d.page, *id))
        .any(|e| !matches!(e.kind, fp_model::ElementKind::Bitmap(_)));
    if !vectors {
        return false;
    }
    if st.session.prefs.get_bool(FILTER_VECTORS_PREF, false) {
        // abgeschaltet: ohne Frage konvertieren wie nach OK
        convert_then_run(st, name);
        return true;
    }
    st.dialogs.warning = Some(WarningDialog::new(TargetWarning {
        key: FILTER_VECTORS.into(),
        command: name.into(),
        text: t("msg-warn-filter-vectors"),
        pref: Some(FILTER_VECTORS_PREF.into()),
        choices: vec![fp_engine::warnings::WarningChoice {
            label: t("btn-ok"),
            command: name.into(),
        }],
        suppressed: false,
    }));
    true
}

/// Nach OK (oder abgeschalteter Rückfrage): der Filter (mit seinem Dialog). Gerastert wird wie im
/// Original erst mit dem Filter selbst, im selben Verlaufsschritt; bricht man den Filterdialog ab,
/// bleibt das Objekt ein Vektor und es entsteht kein Schritt (re 28 L9).
fn convert_then_run(st: &mut AppState, command: &str) {
    st.dialogs.filter_vectors_ok = true;
    st.session.filter_rasterize = true;
    st.push(Action::Run(command.into(), json!({})));
}

/// Schlüssel der Rückfrage „Automatisch neue Status hinzufügen?“ (Q-20, M2).
pub const ANIMATE_STATES: &str = "animateStates";
/// Voreinstellung ihres Kästchens „Nicht mehr anzeigen“ („Warndialoge zurücksetzen“ schaltet sie
/// wieder ein).
pub const ANIMATE_STATES_PREF: &str = "DisableConfirmAnimateStates";

/// „Auswahl animieren“ nach dem Dialog: Reicht die Animation über den letzten Status hinaus, legt
/// das Original die Animation **sofort** an (ohne neue Status, TW-53) und fragt dann „Automatisch
/// neue Status hinzufügen?“ (OK, Abbrechen, Nicht mehr anzeigen). OK hängt die fehlenden Status
/// an, Abbrechen lässt es dabei; abgeschaltet gilt OK (ein Schritt mit den Status).
/// `true`: Befehl eingereiht (und Rückfrage offen).
pub fn intercept_animate_states(st: &mut AppState, name: &str, params: &serde_json::Value) -> bool {
    if name != "modify.animation.animateSelection"
        || params
            .as_object()
            .is_none_or(|o| o.is_empty() || o.contains_key("addStates"))
    {
        return false;
    }
    let Some(d) = st.session.active() else {
        return false;
    };
    let frames = params
        .get("frames")
        .and_then(|v| v.as_f64())
        .unwrap_or(fp_model::AnimParams::default().frames as f64);
    if frames <= d.frame_count() as f64 {
        return false;
    }
    if st.session.prefs.get_bool(ANIMATE_STATES_PREF, false) {
        run_animate(st, name, params.clone(), true);
        return true;
    }
    run_animate(st, name, params.clone(), false);
    st.dialogs.animate_states = Some(json!({ "frames": frames.round() as i64 }));
    st.dialogs.warning = Some(WarningDialog::new(TargetWarning {
        key: ANIMATE_STATES.into(),
        command: name.into(),
        text: t("msg-confirm-animate-add-states"),
        pref: Some(ANIMATE_STATES_PREF.into()),
        choices: vec![fp_engine::warnings::WarningChoice {
            label: t("btn-ok"),
            command: name.into(),
        }],
        suppressed: false,
    }));
    true
}

/// Antwort auf „Automatisch neue Status hinzufügen?“: OK hängt an die eben angelegte (gewählte)
/// Animation die fehlenden Status an, Abbrechen lässt die Animation ohne neue Status.
pub fn answer_animate_states(st: &mut AppState, ok: bool) {
    if let Some(mut p) = st.dialogs.animate_states.take()
        && ok
    {
        // zweiter Verlaufsschritt „Status hinzufügen“ wie im Original (TW-53, re 29 §4)
        p["step"] = json!("add-states");
        st.push(Action::Run("modify.animation.settings".into(), p));
    }
}

fn run_animate(st: &mut AppState, name: &str, mut params: serde_json::Value, add: bool) {
    params["addStates"] = json!(add);
    st.push(Action::Run(name.into(), params));
}

/// Schlüssel der Meldung „Die Pfadausbesserungs-Werkzeuge können nur für Pfade verwendet werden …“
/// (TW-80).
pub const PATH_TOOLS_PRIMITIVE: &str = "pathToolsOnPrimitive";
/// Voreinstellung ihres Kästchens „Nicht mehr anzeigen“ („Warndialoge zurücksetzen“ schaltet sie
/// wieder ein).
pub const PATH_TOOLS_PRIMITIVE_PREF: &str = "DisableConfirmPathToolsOnPrimitive";

/// Freiform/Umformen/Pfadausbesserung auf einer Grundform (TW-80, Original
/// `tool-path-reshape/11`): Meldung mit OK und „Nicht mehr anzeigen“; das Objekt bleibt unverändert.
pub fn path_tools_on_primitive(st: &mut AppState) {
    if st.session.prefs.get_bool(PATH_TOOLS_PRIMITIVE_PREF, false) || st.dialogs.warning.is_some() {
        return;
    }
    st.dialogs.warning = Some(WarningDialog::new(TargetWarning {
        key: PATH_TOOLS_PRIMITIVE.into(),
        command: String::new(),
        text: t("msg-warn-path-tools-primitive"),
        pref: Some(PATH_TOOLS_PRIMITIVE_PREF.into()),
        choices: Vec::new(),
        suppressed: false,
    }));
}

/// Schlüssel der Rückfrage „Es sind mehrere Elemente ausgewählt. Ein Segment oder mehrere Segmente
/// erstellen?“ (Bearbeiten ▸ Objekt einfügen, TW-42).
pub const INSERT_WEB: &str = "insertWebMultiple";

/// Bearbeiten ▸ Objekt einfügen ▸ Hotspot/Segment/Polygonsegment aus Menü oder Kürzel mit mehreren
/// gewählten Objekten: Das Original fragt „Einzeln“ (ein Objekt über alle) oder „Mehrere“ (eines je
/// Objekt), ohne „Abbrechen“; Esc = Einzeln (Spezifikation `docs/parity/tool-web.orig.md`, Eintrag 12).
/// `true`: Rückfrage offen.
pub fn intercept_insert_web(st: &mut AppState, name: &str, params: &serde_json::Value) -> bool {
    if !matches!(
        name,
        "edit.insert.hotspot" | "edit.insert.slice" | "edit.insert.polygonSlice"
    ) || !params.as_object().is_none_or(|o| o.is_empty())
    {
        return false;
    }
    if st
        .session
        .active()
        .is_none_or(|d| d.selection.elements.len() < 2)
    {
        return false;
    }
    let text = t(if name == "edit.insert.hotspot" {
        "msg-insert-web-multi-hotspot"
    } else {
        "msg-insert-web-multi-slice"
    });
    let choice = |k: &str| fp_engine::warnings::WarningChoice {
        label: t(k),
        command: name.into(),
    };
    st.dialogs.warning = Some(WarningDialog::new(TargetWarning {
        key: INSERT_WEB.into(),
        command: name.into(),
        text,
        pref: None,
        choices: vec![
            choice("btn-insert-web-single"),
            choice("btn-insert-web-multiple"),
        ],
        suppressed: false,
    }));
    true
}

/// Zugriffstaste ohne Alt (Meldungsfenster wie im Original).
fn access(ui: &egui::Ui, label: &str) -> bool {
    let Some(k) = super::access_key(label) else {
        return false;
    };
    !ui.ctx().text_edit_focused() && ui.input(|i| i.key_pressed(k) && !i.modifiers.ctrl)
}

enum Outcome {
    Close,
    Choice(usize),
}

pub fn show(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut d) = st.dialogs.warning.clone() else {
        return;
    };
    let w = &d.warning;
    let question = !w.choices.is_empty();
    let ok = t("btn-ok");
    let cancel = t("msg-warn-choice-cancel");
    let check = t("msg-warn-dont-show");
    let mut labels: Vec<String> = w.choices.iter().map(|c| c.label.clone()).collect();
    // „Einzeln/Mehrere“ hat im Original kein „Abbrechen“ (TW-42)
    if question && w.key == INSERT_WEB {
    } else if question {
        labels.push(cancel.clone());
    } else {
        labels.push(ok.clone());
    }
    let buttons: Vec<BarButton> = labels
        .iter()
        .enumerate()
        .map(|(i, l)| {
            if i == 0 {
                BarButton::default_button(l)
            } else {
                BarButton::new(l)
            }
        })
        .collect();
    let check_w = if w.pref.is_some() {
        13.0 + 5.0
            + ui_kit::text_w(ctx, &check.replace('&', ""), &Theme::current(ctx).ui_font())
            + sp::L
    } else {
        0.0
    };
    let width = 380f32.max(ui_kit::button_bar_width(ctx, &[], &buttons) + check_w);
    let text_x = sp::L + 32.0 + sp::M;
    let th = super::wrapped_h(ctx, &w.text, width - text_x - sp::L);
    let content_h = th.max(32.0);
    let mut outcome = None;
    let mut toggled = None;
    super::without_backdrop(ctx, "fp-warning"); // TW-91: Hinweis als Meldungsfenster, Leinwand bleibt hell
    let r = dialog_frame(
        ctx,
        "fp-warning",
        "Firepixel",
        vec2(width, (sp::L + content_h + sp::L + sp::BUTTON_BAR_H).ceil()),
        |ui, body| {
            let theme = Theme::current(ui.ctx());
            let icon = Rect::from_min_size(body.min + vec2(sp::L, sp::L), vec2(32.0, 32.0));
            ui_kit::paint_icon(
                ui.painter(),
                icon,
                KitIcon::Warning,
                &crate::icons::IconStyle::new(&theme, None, true),
            );
            let ty = if th < 32.0 {
                icon.center().y - th / 2.0
            } else {
                icon.top()
            };
            super::put_left(
                ui,
                Rect::from_min_size(
                    pos2(body.left() + text_x, ty),
                    vec2(body.width() - text_x - sp::L, th + 4.0),
                ),
                egui::Label::new(w.text.as_str()).wrap(),
            );
            let bar = ui_kit::button_bar(ui, body, &[], &buttons);
            if let Some(i) = bar.right {
                outcome = Some(if question && i < w.choices.len() {
                    Outcome::Choice(i)
                } else {
                    Outcome::Close
                });
            }
            for (i, l) in labels.iter().enumerate() {
                if outcome.is_none() && access(ui, l) {
                    outcome = Some(if question && i < w.choices.len() {
                        Outcome::Choice(i)
                    } else {
                        Outcome::Close
                    });
                }
            }
            if w.pref.is_some() {
                let y = body.bottom() - sp::M - sp::BUTTON_H;
                let cr = Rect::from_min_size(
                    pos2(body.left() + sp::L, y),
                    vec2(check_w - sp::L, sp::BUTTON_H),
                );
                let text = check.replace('&', "");
                if let Some(v) =
                    fields::checkbox(ui, "fp-warning-dont-show", cr, &text, d.dont_show, true)
                {
                    toggled = Some(v);
                } else if access(ui, &check) {
                    toggled = Some(!d.dont_show);
                }
            }
        },
    );
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
    if d.warning.key == INSERT_WEB {
        // Esc bzw. Schließen = „Einzeln“
        let multiple = matches!(outcome, Outcome::Choice(1));
        st.push(Action::Run(
            d.warning.command.clone(),
            json!({ "multiple": multiple }),
        ));
        return;
    }
    if d.warning.key == ANIMATE_STATES {
        // die Animation steht schon (TW-53); OK hängt die fehlenden Status an, Abbrechen/Esc nicht
        answer_animate_states(st, matches!(outcome, Outcome::Choice(_)));
        return;
    }
    if question && matches!(outcome, Outcome::Close) {
        // Rückfrage abgebrochen (Knopf oder Esc): wie im Original folgt eine zweite Meldung
        // (Popupmenü: „Wählen Sie ein Segment oder einen Hotspot aus …“, PM-6 D34)
        if let Some(next) = st.session.warning_after_cancel(&d.warning)
            && !next.suppressed
        {
            st.dialogs.warning = Some(WarningDialog::new(next));
        }
        return;
    }
    if let Outcome::Choice(_) = outcome
        && d.warning.key == FILTER_VECTORS
    {
        // OK: wie angekündigt erst in Bitmap konvertieren, dann den Filter ausführen (bzw. seinen
        // Dialog öffnen), ohne erneut zu fragen
        convert_then_run(st, &d.warning.command);
        return;
    }
    if let Outcome::Choice(i) = outcome {
        // Rückfrage (Popupmenü): Hotspot/Segment aus der Auswahl anlegen, dann der ursprüngliche
        // Befehl mit dem neuen Objekt; fehlt auch dafür das Ziel, kommt dessen Warnung.
        let choice = d.warning.choices[i].command.clone();
        if !intercept(st, &choice) {
            // ohne Rückfrage „Einzeln/Mehrere“ (sonst liefe der Befehl vor der Antwort weiter)
            st.push(Action::Run(choice, json!({ "multiple": false })));
            st.push(Action::Run(d.warning.command.clone(), json!({})));
        }
    }
}
