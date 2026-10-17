//! Inspektor-Quadranten der KI-Werkzeuge (Agent KI): Objektauswahl, Motiv auswählen, Kante
//! verfeinern. Die Optionen stehen in den Voreinstellungen (`AiObjectModel`, `AiSubjectModel`,
//! `AiMattingModel`, `AiSource`, `AiRefineRadius`, `AiRefineOutput`) und gelten damit auch für
//! Menü, MCP und Skripte. Eigene Vorlagen-IDs 19001–19006 (keine Vorlagen des Originals).
//!
//! Feineinstellungen (KI-2, Nutzerbefund N21), angelehnt an den Zauberstab – nur, was die Modelle
//! hergeben (Begründung in `docs\MODELLE.md`):
//! * Objektauswahl: Toleranz (Logit-Schwelle), Kante (Hart/Anti-Alias/Weiche Kante + Wert),
//!   Variante (Automatisch/Klein/Mittel/Groß unter den Masken des Decoders), Ausdehnen, Glätten
//! * Motiv auswählen: Toleranz (Tonwerte der weichen Maske), Kante, Ausdehnen, Glätten
//! * Kante verfeinern: Radius (wie bisher)
//!
//! Voreinstellungen `AiObjectTolerance|Edge|Feather|Variant|Expand|Smooth` bzw. `AiSubject…`.
//! Ändert man einen Wert, wird die letzte KI-Auswahl desselben Werkzeugs aus der
//! zwischengespeicherten Modell-Ausgabe neu gebildet (kein Modell-Laufen), solange sie noch die
//! aktuelle Auswahl ist.

use super::QuadrantRegistry;
use super::fields::{self, Dl};
use super::texts::{refs, t, tl};
use crate::app_state::AppState;
use egui::{Rect, Ui};
use fp_engine::PrefValue;
use fp_engine::commands::ai::fp_ai::models::{self, ModelId};

/// Vorlagen der drei Werkzeuge (Q3, Q4).
pub const OBJECT_Q3: u16 = 19001;
pub const OBJECT_Q4: u16 = 19002;
pub const SUBJECT_Q3: u16 = 19003;
pub const SUBJECT_Q4: u16 = 19004;
pub const REFINE_Q3: u16 = 19005;
pub const REFINE_Q4: u16 = 19006;

pub(super) fn register(r: &mut QuadrantRegistry) {
    r.register(OBJECT_Q3, object_q3);
    r.register(OBJECT_Q4, object_q4);
    r.register(SUBJECT_Q3, subject_q3);
    r.register(SUBJECT_Q4, subject_q4);
    r.register(REFINE_Q3, refine_q3);
    r.register(REFINE_Q4, refine_q4);
}

fn pref_model(st: &AppState, key: &str, choices: &[ModelId]) -> usize {
    let v = st.session.prefs.get_str(key, "");
    let id = models::by_name(&v);
    id.and_then(|id| choices.iter().position(|c| *c == id))
        .unwrap_or(0)
}

fn set_model(st: &mut AppState, key: &str, id: ModelId) {
    let name = serde_json::to_value(id)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default();
    st.session.prefs.set(key, PrefValue::Str(name));
}

fn model_combo(
    ui: &mut Ui,
    st: &mut AppState,
    d: Dl,
    id: &str,
    key: &str,
    label: &str,
    options: &[(ModelId, &str)],
) {
    let choices: Vec<ModelId> = options.iter().map(|o| o.0).collect();
    let names: Vec<String> = options.iter().map(|o| t(o.1)).collect();
    fields::label_right(ui, d.r(2.0, 2.0, 40.0, 8.0), &t(label), true);
    let cur = pref_model(st, key, &choices);
    if let Some(i) = fields::combo(
        ui,
        (id, "model"),
        d.r(44.0, 0.0, 112.0, 13.0),
        &refs(&names),
        Some(cur),
        true,
    ) {
        set_model(st, key, choices[i]);
    }
}

fn source_combo(ui: &mut Ui, st: &mut AppState, d: Dl, id: &str) {
    fields::label_right(ui, d.r(2.0, 17.0, 40.0, 8.0), &t("pi-ai-source"), true);
    let cur = if st.session.prefs.get_str("AiSource", "auto") == "page" {
        1
    } else {
        0
    };
    if let Some(i) = fields::combo(
        ui,
        (id, "source"),
        d.r(44.0, 15.0, 112.0, 13.0),
        &refs(&tl(&["pi-ai-source-auto", "pi-ai-source-page"])),
        Some(cur),
        true,
    ) {
        st.session.prefs.set(
            "AiSource",
            PrefValue::Str(if i == 1 { "page".into() } else { "auto".into() }),
        );
    }
}

/// Stand des gewählten Modells + Knopf „KI-Modelle…“.
fn model_state(ui: &mut Ui, st: &mut AppState, d: Dl, id: &str, model: ModelId) {
    let ok = models::available(model);
    if ok {
        let text = format!(
            "{} ({})",
            models::info(model).display_name(),
            models::human_size(models::info(model).size)
        );
        fields::label_name(ui, d.r(160.0, 2.0, 170.0, 8.0), &text, true);
    } else {
        fields::label(
            ui,
            d.r(160.0, 2.0, 170.0, 8.0),
            &t("pi-ai-model-missing"),
            true,
        );
    }
    if fields::button(
        ui,
        (id, "models"),
        d.r(160.0, 14.0, 60.0, 13.0),
        &t("pi-ai-models"),
        true,
    ) {
        crate::dialogs::models::open_manager(st);
    }
}

/// Knopf „KI-Modelle…“ an fester Stelle (Werkzeuge mit Feineinstellungen).
fn models_button(ui: &mut Ui, st: &mut AppState, r: Rect, id: &str) {
    if fields::button(ui, (id, "models"), r, &t("pi-ai-models"), true) {
        crate::dialogs::models::open_manager(st);
    }
}

fn current(st: &AppState, key: &str, choices: &[ModelId]) -> ModelId {
    choices[pref_model(st, key, choices)]
}

const OBJECT_MODELS: [ModelId; 2] = [ModelId::Sam2Tiny, ModelId::Sam2Small];
const SUBJECT_MODELS: [ModelId; 2] = [ModelId::BiRefNetLite, ModelId::BiRefNet];
const MATTING_MODELS: [ModelId; 2] = [ModelId::BiRefNetLiteMatting, ModelId::BiRefNetHrMatting];

// ---------------------------------------------------------------------- Feineinstellungen

/// Kante wie beim Zauberstab: Hart / Anti-Alias / Weiche Kante.
const EDGE3: [&str; 3] = ["pi-edge-hard", "pi-edge-antialias", "pi-edge-feather"];
const EDGE_NAMES: [&str; 3] = ["hard", "antiAlias", "feather"];
const VARIANTS: [&str; 4] = [
    "pi-ai-variant-auto",
    "pi-ai-variant-small",
    "pi-ai-variant-medium",
    "pi-ai-variant-large",
];
const VARIANT_NAMES: [&str; 4] = ["auto", "small", "medium", "large"];

/// Werkzeug, dessen Einstellungen ein Quadrant zeigt.
#[derive(Clone, Copy)]
struct Tune {
    /// Präfix der Voreinstellungen.
    pre: &'static str,
    /// Befehl zum Nachstellen.
    cmd: &'static str,
    /// Kennung der Felder.
    id: &'static str,
}

const OBJECT_TUNE: Tune = Tune {
    pre: "AiObject",
    cmd: "select.aiObject",
    id: "ai-obj",
};
const SUBJECT_TUNE: Tune = Tune {
    pre: "AiSubject",
    cmd: "select.aiSubject",
    id: "ai-subj",
};

fn pk(tn: Tune, k: &str) -> String {
    format!("{}{k}", tn.pre)
}

/// Wert gesetzt → merken und die letzte Auswahl dieses Werkzeugs neu bilden.
fn changed(st: &mut AppState, tn: Tune, key: &str, v: PrefValue) {
    st.session.prefs.set(&pk(tn, key), v);
    crate::dialogs::models::retune(st, tn.cmd);
}

/// „Toleranz:“ + Zahlenfeld (0…100) ab `x` in Zeile `y` (DLU).
fn tolerance_field(ui: &mut Ui, st: &mut AppState, d: Dl, tn: Tune, x: f32, y: f32) {
    fields::label_right(ui, d.r(x, y + 2.0, 40.0, 8.0), &t("pi-tolerance"), true);
    let v = st.session.prefs.get_int(&pk(tn, "Tolerance"), 50) as f64;
    if let Some(n) = fields::number(
        ui,
        (tn.id, "tol"),
        d.r(x + 42.0, y, 21.0, 13.0),
        Some(d.r(x + 64.0, y, 9.0, 13.0)),
        Some(v),
        (0.0, 100.0),
        0,
        true,
    ) {
        changed(st, tn, "Tolerance", PrefValue::Int(n.round() as i64));
    }
}

/// „Kante:“ + Kombifeld + Breite (wie beim Zauberstab).
fn edge_fields(ui: &mut Ui, st: &mut AppState, d: Dl, tn: Tune, x: f32, y: f32) {
    fields::label_right(ui, d.r(x, y + 2.0, 40.0, 8.0), &t("pi-edge"), true);
    let name = st.session.prefs.get_str(&pk(tn, "Edge"), "antiAlias");
    let i = EDGE_NAMES
        .iter()
        .position(|n| n.eq_ignore_ascii_case(&name))
        .unwrap_or(1);
    let f = st.session.prefs.get_int(&pk(tn, "Feather"), 0);
    if let Some(n) = fields::combo(
        ui,
        (tn.id, "edge"),
        d.r(x + 42.0, y, 65.0, 13.0),
        &refs(&tl(&EDGE3)),
        Some(i),
        true,
    ) {
        // „Weiche Kante“ ohne Wert: 10 Pixel wie beim Zauberstab
        if n == 2 && f <= 0 {
            st.session.prefs.set(&pk(tn, "Feather"), PrefValue::Int(10));
        }
        changed(st, tn, "Edge", PrefValue::Str(EDGE_NAMES[n].into()));
    }
    let shown = if i == 2 { f.max(0) } else { 0 };
    if let Some(v) = fields::number(
        ui,
        (tn.id, "feather"),
        d.r(x + 109.0, y, 21.0, 13.0),
        Some(d.r(x + 131.0, y, 9.0, 13.0)),
        Some(shown as f64),
        (0.0, 100.0),
        0,
        i == 2,
    ) {
        changed(st, tn, "Feather", PrefValue::Int(v.max(1.0) as i64));
    }
}

/// „Ausdehnen:“ (−100…100, negativ = zusammenziehen) und „Glätten:“ (0…100).
fn expand_smooth_fields(ui: &mut Ui, st: &mut AppState, d: Dl, tn: Tune, x: f32, y: f32) {
    fields::label_right(ui, d.r(x, y + 2.0, 40.0, 8.0), &t("pi-ai-expand"), true);
    let e = st.session.prefs.get_int(&pk(tn, "Expand"), 0) as f64;
    if let Some(v) = fields::number(
        ui,
        (tn.id, "expand"),
        d.r(x + 42.0, y, 21.0, 13.0),
        Some(d.r(x + 64.0, y, 9.0, 13.0)),
        Some(e),
        (-100.0, 100.0),
        0,
        true,
    ) {
        changed(st, tn, "Expand", PrefValue::Int(v.round() as i64));
    }
    fields::label_right(
        ui,
        d.r(x + 76.0, y + 2.0, 36.0, 8.0),
        &t("pi-ai-smooth"),
        true,
    );
    let s = st.session.prefs.get_int(&pk(tn, "Smooth"), 0) as f64;
    if let Some(v) = fields::number(
        ui,
        (tn.id, "smooth"),
        d.r(x + 114.0, y, 21.0, 13.0),
        Some(d.r(x + 136.0, y, 9.0, 13.0)),
        Some(s),
        (0.0, 100.0),
        0,
        true,
    ) {
        changed(st, tn, "Smooth", PrefValue::Int(v.round() as i64));
    }
}

fn variant_combo(ui: &mut Ui, st: &mut AppState, d: Dl, x: f32, y: f32) {
    let tn = OBJECT_TUNE;
    fields::label_right(ui, d.r(x, y + 2.0, 40.0, 8.0), &t("pi-ai-variant"), true);
    let name = st.session.prefs.get_str("AiObjectVariant", "auto");
    let i = VARIANT_NAMES.iter().position(|n| *n == name).unwrap_or(0);
    if let Some(n) = fields::combo(
        ui,
        (tn.id, "variant"),
        d.r(x + 42.0, y, 112.0, 13.0),
        &refs(&tl(&VARIANTS)),
        Some(i),
        true,
    ) {
        changed(st, tn, "Variant", PrefValue::Str(VARIANT_NAMES[n].into()));
    }
}

/// Hinweiszeile; fehlt das Modell, stattdessen der Hinweis darauf.
fn hint_line(ui: &mut Ui, d: Dl, y: f32, model: ModelId, hint: &str) {
    let text = if models::available(model) {
        t(hint)
    } else {
        t("pi-ai-model-missing")
    };
    fields::label(ui, d.r(4.0, y + 2.0, 380.0, 8.0), &text, true);
}

// ---------------------------------------------------------------------- Quadranten

fn object_q3(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    model_combo(
        ui,
        st,
        d,
        "ai-obj",
        "AiObjectModel",
        "pi-ai-model",
        &[
            (OBJECT_MODELS[0], "pi-ai-sam-tiny"),
            (OBJECT_MODELS[1], "pi-ai-sam-small"),
        ],
    );
    source_combo(ui, st, d, "ai-obj");
    tolerance_field(ui, st, d, OBJECT_TUNE, 160.0, 0.0);
    edge_fields(ui, st, d, OBJECT_TUNE, 160.0, 15.0);
    models_button(ui, st, d.r(310.0, 0.0, 60.0, 13.0), "ai-obj");
}

fn object_q4(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    variant_combo(ui, st, d, 2.0, 0.0);
    expand_smooth_fields(ui, st, d, OBJECT_TUNE, 160.0, 0.0);
    let m = current(st, "AiObjectModel", &OBJECT_MODELS);
    hint_line(ui, d, 15.0, m, "pi-ai-object-hint2");
}

fn subject_q3(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    model_combo(
        ui,
        st,
        d,
        "ai-subj",
        "AiSubjectModel",
        "pi-ai-quality",
        &[
            (SUBJECT_MODELS[0], "pi-ai-subject-fast"),
            (SUBJECT_MODELS[1], "pi-ai-subject-high"),
        ],
    );
    source_combo(ui, st, d, "ai-subj");
    tolerance_field(ui, st, d, SUBJECT_TUNE, 160.0, 0.0);
    edge_fields(ui, st, d, SUBJECT_TUNE, 160.0, 15.0);
    models_button(ui, st, d.r(310.0, 0.0, 60.0, 13.0), "ai-subj");
}

fn subject_q4(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    if fields::button(
        ui,
        "ai-subj-run",
        d.r(44.0, 0.0, 112.0, 13.0),
        &t("pi-ai-subject-run"),
        st.session.active().is_some(),
    ) {
        crate::dialogs::models::request(
            st,
            &fp_engine::tools::Invocation::new("select.aiSubject", serde_json::json!({})),
        );
    }
    expand_smooth_fields(ui, st, d, SUBJECT_TUNE, 160.0, 0.0);
    let m = current(st, "AiSubjectModel", &SUBJECT_MODELS);
    hint_line(ui, d, 15.0, m, "pi-ai-subject-hint");
}

fn refine_q3(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    model_combo(
        ui,
        st,
        d,
        "ai-ref",
        "AiMattingModel",
        "pi-ai-quality",
        &[
            (MATTING_MODELS[0], "pi-ai-matting-fast"),
            (MATTING_MODELS[1], "pi-ai-matting-high"),
        ],
    );
    fields::label_right(ui, d.r(2.0, 17.0, 40.0, 8.0), &t("pi-ai-radius"), true);
    let rad = st.session.prefs.get_int("AiRefineRadius", 0) as f64;
    if let Some(v) = fields::number(
        ui,
        "ai-ref-radius",
        d.r(44.0, 15.0, 21.0, 13.0),
        Some(d.r(66.0, 15.0, 9.0, 13.0)),
        Some(rad),
        (0.0, 200.0),
        0,
        true,
    ) {
        st.session
            .prefs
            .set("AiRefineRadius", PrefValue::Int(v as i64));
    }
    fields::label(
        ui,
        d.r(78.0, 17.0, 60.0, 8.0),
        &t(if rad <= 0.0 {
            "pi-ai-radius-auto"
        } else {
            "pi-ai-pixels"
        }),
        true,
    );
    let m = current(st, "AiMattingModel", &MATTING_MODELS);
    model_state(ui, st, d, "ai-ref", m);
}

fn refine_q4(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    fields::label_right(ui, d.r(2.0, 2.0, 40.0, 8.0), &t("pi-ai-output"), true);
    let cur = if st.session.prefs.get_str("AiRefineOutput", "selection") == "mask" {
        1
    } else {
        0
    };
    if let Some(i) = fields::combo(
        ui,
        "ai-ref-out",
        d.r(44.0, 0.0, 112.0, 13.0),
        &refs(&tl(&["pi-ai-output-selection", "pi-ai-output-mask"])),
        Some(cur),
        true,
    ) {
        st.session.prefs.set(
            "AiRefineOutput",
            PrefValue::Str(if i == 1 {
                "mask".into()
            } else {
                "selection".into()
            }),
        );
    }
    let has_sel = st
        .session
        .active()
        .is_some_and(|d| d.selection.has_pixels());
    if fields::button(
        ui,
        "ai-ref-run",
        d.r(44.0, 15.0, 112.0, 13.0),
        &t("pi-ai-refine-run"),
        has_sel,
    ) {
        crate::dialogs::models::request(
            st,
            &fp_engine::tools::Invocation::new("select.aiRefineEdge", serde_json::json!({})),
        );
    }
    fields::label(
        ui,
        d.r(160.0, 2.0, 180.0, 8.0),
        &t(if has_sel {
            "pi-ai-refine-hint"
        } else {
            "pi-ai-need-selection"
        }),
        true,
    );
}
