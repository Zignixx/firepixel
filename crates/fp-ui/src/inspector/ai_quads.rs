//! Inspektor-Quadranten der KI-Werkzeuge (Agent KI): Objektauswahl, Motiv auswählen, Kante
//! verfeinern. Die Optionen stehen in den Voreinstellungen (`AiObjectModel`, `AiSubjectModel`,
//! `AiMattingModel`, `AiSource`, `AiRefineRadius`, `AiRefineOutput`) und gelten damit auch für
//! Menü, MCP und Skripte. Eigene Vorlagen-IDs 19001–19006 (keine Vorlagen des Originals).

use super::fields::{self, Dl};
use super::QuadrantRegistry;
use crate::app_state::AppState;
use egui::{Rect, Ui};
use fp_engine::commands::ai::fp_ai::models::{self, ModelId};
use fp_engine::PrefValue;

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
    id.and_then(|id| choices.iter().position(|c| *c == id)).unwrap_or(0)
}

fn set_model(st: &mut AppState, key: &str, id: ModelId) {
    let name = serde_json::to_value(id).ok().and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default();
    st.session.prefs.set(key, PrefValue::Str(name));
}

fn model_combo(ui: &mut Ui, st: &mut AppState, d: Dl, id: &str, key: &str, label: &str, options: &[(ModelId, &str)]) {
    let choices: Vec<ModelId> = options.iter().map(|o| o.0).collect();
    let names: Vec<&str> = options.iter().map(|o| o.1).collect();
    fields::label_right(ui, d.r(2.0, 2.0, 40.0, 8.0), label, true);
    let cur = pref_model(st, key, &choices);
    if let Some(i) = fields::combo(ui, (id, "model"), d.r(44.0, 0.0, 112.0, 13.0), &names, Some(cur), true) {
        set_model(st, key, choices[i]);
    }
}

fn source_combo(ui: &mut Ui, st: &mut AppState, d: Dl, id: &str) {
    fields::label_right(ui, d.r(2.0, 17.0, 40.0, 8.0), "Quelle:", true);
    let cur = if st.session.prefs.get_str("AiSource", "auto") == "page" { 1 } else { 0 };
    if let Some(i) = fields::combo(ui, (id, "source"), d.r(44.0, 15.0, 112.0, 13.0), &["Bitmap, sonst Seite", "Sichtbare Seite"], Some(cur), true) {
        st.session.prefs.set("AiSource", PrefValue::Str(if i == 1 { "page".into() } else { "auto".into() }));
    }
}

/// Stand des gewählten Modells + Knopf „KI-Modelle…“.
fn model_state(ui: &mut Ui, st: &mut AppState, d: Dl, id: &str, model: ModelId) {
    let ok = models::available(model);
    let text = if ok { format!("{} ({})", models::info(model).name, models::human_size(models::info(model).size)) } else { "Modell fehlt – wird beim ersten Benutzen angeboten".to_string() };
    fields::label(ui, d.r(160.0, 2.0, 170.0, 8.0), &text, true);
    if fields::button(ui, (id, "models"), d.r(160.0, 14.0, 60.0, 13.0), "KI-Modelle…", true) {
        crate::dialogs::models::open_manager(st);
    }
}

fn current(st: &AppState, key: &str, choices: &[ModelId]) -> ModelId {
    choices[pref_model(st, key, choices)]
}

const OBJECT_MODELS: [ModelId; 2] = [ModelId::Sam2Tiny, ModelId::Sam2Small];
const SUBJECT_MODELS: [ModelId; 2] = [ModelId::BiRefNetLite, ModelId::BiRefNet];
const MATTING_MODELS: [ModelId; 2] = [ModelId::BiRefNetLiteMatting, ModelId::BiRefNetHrMatting];

fn object_q3(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    model_combo(ui, st, d, "ai-obj", "AiObjectModel", "Modell:", &[(OBJECT_MODELS[0], "SAM 2.1 Tiny (schnell)"), (OBJECT_MODELS[1], "SAM 2.1 Small (genauer)")]);
    source_combo(ui, st, d, "ai-obj");
    let m = current(st, "AiObjectModel", &OBJECT_MODELS);
    model_state(ui, st, d, "ai-obj", m);
}

fn object_q4(ui: &mut Ui, _st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    fields::label(ui, d.r(4.0, 2.0, 330.0, 8.0), "Klick: Objekt auswählen · Rahmen aufziehen: Objekt im Rahmen", true);
    fields::label(ui, d.r(4.0, 14.0, 330.0, 8.0), "Umschalt+Klick: hinzufügen · Alt+Klick: abziehen · Esc: neues Objekt", true);
}

fn subject_q3(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    model_combo(ui, st, d, "ai-subj", "AiSubjectModel", "Qualität:", &[(SUBJECT_MODELS[0], "Schnell (BiRefNet lite)"), (SUBJECT_MODELS[1], "Hoch (BiRefNet, langsam)")]);
    source_combo(ui, st, d, "ai-subj");
    let m = current(st, "AiSubjectModel", &SUBJECT_MODELS);
    model_state(ui, st, d, "ai-subj", m);
}

fn subject_q4(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    fields::label(ui, d.r(4.0, 2.0, 330.0, 8.0), "Klick: Hauptmotiv als weiche Pixelauswahl · Umschalt: hinzufügen · Alt: abziehen", true);
    if fields::button(ui, "ai-subj-run", d.r(4.0, 14.0, 80.0, 13.0), "Motiv auswählen", st.session.active().is_some()) {
        crate::dialogs::models::request(st, &fp_engine::tools::Invocation::new("select.aiSubject", serde_json::json!({})));
    }
}

fn refine_q3(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    model_combo(ui, st, d, "ai-ref", "AiMattingModel", "Qualität:", &[(MATTING_MODELS[0], "Schnell (Matting lite)"), (MATTING_MODELS[1], "Hoch (HR-Matting, langsam)")]);
    fields::label_right(ui, d.r(2.0, 17.0, 40.0, 8.0), "Radius:", true);
    let rad = st.session.prefs.get_int("AiRefineRadius", 0) as f64;
    if let Some(v) = fields::number(ui, "ai-ref-radius", d.r(44.0, 15.0, 21.0, 13.0), Some(d.r(66.0, 15.0, 9.0, 13.0)), Some(rad), (0.0, 200.0), 0, true) {
        st.session.prefs.set("AiRefineRadius", PrefValue::Int(v as i64));
    }
    fields::label(ui, d.r(78.0, 17.0, 60.0, 8.0), if rad <= 0.0 { "(0 = automatisch)" } else { "Pixel" }, true);
    let m = current(st, "AiMattingModel", &MATTING_MODELS);
    model_state(ui, st, d, "ai-ref", m);
}

fn refine_q4(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    fields::label_right(ui, d.r(2.0, 2.0, 40.0, 8.0), "Ausgabe:", true);
    let cur = if st.session.prefs.get_str("AiRefineOutput", "selection") == "mask" { 1 } else { 0 };
    if let Some(i) = fields::combo(ui, "ai-ref-out", d.r(44.0, 0.0, 112.0, 13.0), &["Weiche Pixelauswahl", "Bitmapmaske"], Some(cur), true) {
        st.session.prefs.set("AiRefineOutput", PrefValue::Str(if i == 1 { "mask".into() } else { "selection".into() }));
    }
    let has_sel = st.session.active().is_some_and(|d| d.selection.has_pixels());
    if fields::button(ui, "ai-ref-run", d.r(44.0, 15.0, 112.0, 13.0), "Kante jetzt verfeinern", has_sel) {
        crate::dialogs::models::request(st, &fp_engine::tools::Invocation::new("select.aiRefineEdge", serde_json::json!({})));
    }
    fields::label(ui, d.r(160.0, 2.0, 180.0, 8.0), if has_sel { "Klick in die Auswahl: Kante verfeinern" } else { "Erst eine Pixelauswahl erstellen" }, true);
}
