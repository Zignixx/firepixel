//! Formular einer aufgeklappten Schrittkarte aus dem Parameter-Schema (`Step::params`,
//! SPEC-BATCH §2.4): Beschriftung-Wert-Raster (DESIGN §5), Schieber mit Zahlenfeld und Einheit,
//! Auswahl, Kästchen, Farbe, Text, Pfad, Zeit, JSON (Profi). Anfänger-Modus zeigt nur die mit
//! `beginner()` markierten Parameter, „Mehr Optionen“ klappt den Rest auf. Ungültige Werte: rote
//! Kante und Meldung in der Karte.
//!
//! **Eigene Kartenformulare** (BATCH-3 ff.): ein Schritt kann statt des generischen Formulars ein
//! eigenes anmelden – `register_card_body("rename", my_body)`; `my_body(ui, ctx) -> bool` bearbeitet
//! `ctx.params` (JSON-Objekt) und meldet `true` bei Änderung. Für die Endkarten heißen die IDs
//! `rename` (Parameter = `RenameSpec` als JSON) und `output` (Parameter = `format` + Formatoptionen).

use super::Mode;
use crate::theme::Theme;
use crate::ui_kit::{self, sp, type_scale};
use egui::{vec2, Color32, Id, Stroke, StrokeKind, Ui};
use fp_batch::step::{param_key, ParamDef, ParamType};
use fp_batch::{ItemInfo, Recipe};
use fp_i18n::t;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

/// Kontext eines Kartenformulars.
pub struct CardCtx<'a> {
    /// Schritt-ID (`resize`, … bzw. `rename`/`output` für die Endkarten).
    pub step_id: &'a str,
    /// Index in `recipe.steps` (None = Endkarte).
    pub index: Option<usize>,
    /// Parameter (JSON-Objekt) – das Formular ändert sie hier.
    pub params: &'a mut Value,
    /// Schema.
    pub defs: &'a [ParamDef],
    pub mode: Mode,
    /// „Mehr Optionen“ aufgeklappt.
    pub show_more: &'a mut bool,
    /// Sichtbare Dateien in Listenreihenfolge (für Live-Tabellen, z. B. Umbenennen).
    pub files: &'a [ItemInfo],
    /// Rezept vor der Änderung (nur lesen).
    pub recipe: &'a Recipe,
    /// Bedienbar (nicht während eines Laufs).
    pub enabled: bool,
    /// Vom Formular gesetzt: Datei-Dialog für diesen Parameter öffnen.
    pub pick_path: Option<String>,
}

/// Eigenes Kartenformular eines Schritts.
pub type CardBody = fn(&mut Ui, &mut CardCtx) -> bool;

fn bodies() -> &'static RwLock<HashMap<String, CardBody>> {
    static B: OnceLock<RwLock<HashMap<String, CardBody>>> = OnceLock::new();
    B.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Meldet ein eigenes Kartenformular für eine Schritt-ID an (ersetzt das generische Formular).
pub fn register_card_body(step_id: &str, f: CardBody) {
    if let Ok(mut m) = bodies().write() {
        m.insert(step_id.to_string(), f);
    }
}

/// Angemeldetes Kartenformular.
pub fn card_body(step_id: &str) -> Option<CardBody> {
    bodies().read().ok().and_then(|m| m.get(step_id).copied())
}

/// Beschriftung eines Parameters (Rückfall: technischer Name).
pub fn param_label(step_id: &str, name: &str) -> String {
    let k = param_key(step_id, name);
    if fp_i18n::has(&k) { t(&k) } else { name.to_string() }
}

/// Beschriftung eines Auswahlwerts: `batch-param-<id>-<param>-<wert>`, sonst `batch-value-<wert>`,
/// sonst der Wert selbst.
pub fn value_label(step_id: &str, name: &str, value: &str) -> String {
    // Formate: Eigennamen (PNG, WebP …), „wie Quelle“ übersetzt
    if name == "format"
        && let Some(f) = fp_batch::output::OutputFormat::parse(value)
    {
        return if f == fp_batch::output::OutputFormat::Same { t("batch-ui-output-same") } else { f.display_name().to_string() };
    }
    // Schreibweisen der Pakete: batch-param-…-<wert> (BATCH-1/2), batch-value-<id>-<param>-<wert>
    // (BATCH-4/5), batch-choice-<id>-<param>-<wert> (BATCH-6), allgemein batch-value-<wert>
    let id = step_id.replace('.', "-");
    for k in [format!("{}-{}", param_key(step_id, name), value), format!("batch-value-{id}-{name}-{value}"), format!("batch-choice-{id}-{name}-{value}"), format!("batch-value-{value}")] {
        if fp_i18n::has(&k) {
            return t(&k);
        }
    }
    value.to_string()
}

/// Wert eines Parameters als kurzer Text (Zusammenfassung).
pub fn value_text(step_id: &str, d: &ParamDef, v: &Value) -> String {
    let unit = d.unit.as_deref().map(|u| format!(" {u}")).unwrap_or_default();
    match (&d.ty, v) {
        (ParamType::Bool, Value::Bool(b)) => t(if *b { "batch-ui-on" } else { "batch-ui-off" }),
        (ParamType::Enum { .. }, Value::String(s)) => value_label(step_id, &d.name, s),
        (ParamType::Multi { .. }, Value::Array(a)) => a.iter().filter_map(|x| x.as_str()).map(|s| value_label(step_id, &d.name, s)).collect::<Vec<_>>().join(", "),
        (ParamType::Int { .. }, v) => format!("{}{unit}", v.as_i64().unwrap_or(0)),
        (ParamType::Number { .. }, v) => format!("{}{unit}", fp_i18n::fmt_number(v.as_f64().unwrap_or(0.0), 0, 2)),
        (_, Value::String(s)) => s.clone(),
        (_, Value::Null) => String::new(),
        (_, v) => v.to_string(),
    }
}

/// Wert mit Vorgabe.
pub fn current(params: &Value, d: &ParamDef) -> Value {
    params.get(&d.name).filter(|v| !v.is_null()).cloned().unwrap_or_else(|| d.default.clone())
}

/// Sichtbare Parameter je Modus. Liefert (sichtbar, versteckt).
pub fn split_defs(defs: &[ParamDef], mode: Mode, show_more: bool) -> (Vec<&ParamDef>, Vec<&ParamDef>) {
    if mode == Mode::Pro || show_more {
        return (defs.iter().collect(), Vec::new());
    }
    defs.iter().partition(|d| d.beginner)
}

/// Generisches Formular. Liefert `true` bei Änderung.
pub fn generic_body(ui: &mut Ui, ctx: &mut CardCtx) -> bool {
    let theme = Theme::current(ui.ctx());
    let (shown, hidden) = split_defs(ctx.defs, ctx.mode, *ctx.show_more);
    if ctx.defs.is_empty() {
        ui.label(egui::RichText::new(t("batch-ui-step-no-params")).color(super::kit::weak(&theme)).size(type_scale::CAPTION));
        return false;
    }
    let mut changed = false;
    let labels: Vec<String> = shown.iter().map(|d| param_label(ctx.step_id, &d.name)).collect();
    let refs: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
    let avail = ui.available_width();
    let label_w = ui_kit::label_width(ui.ctx(), &refs, 70.0).min((avail * 0.42).max(60.0));
    let mut errors: Vec<String> = Vec::new();
    for (d, label) in shown.iter().zip(&labels) {
        let row = ui.horizontal(|ui| {
            let (lr, _) = ui.allocate_exact_size(vec2(label_w, 22.0), egui::Sense::hover());
            let shown_l = ui_kit::elide(ui.ctx(), label, &theme.font(type_scale::BODY), label_w - sp::XS);
            ui.painter().text(lr.left_center(), egui::Align2::LEFT_CENTER, &shown_l, theme.font(type_scale::BODY), super::kit::weak(&theme));
            let tip_key = format!("{}.tooltip", param_key(ctx.step_id, &d.name));
            let lresp = ui.interact(lr, Id::new(("fp-batch-param-label", ctx.step_id, &d.name)), egui::Sense::hover());
            if fp_i18n::has(&tip_key) {
                lresp.on_hover_text(t(&tip_key));
            } else if shown_l != *label {
                lresp.on_hover_text(label.as_str());
            }
            let w = (ui.available_width() - 2.0).max(40.0);
            ui.add_enabled_ui(ctx.enabled, |ui| widget(ui, ctx, d, label, w)).inner
        });
        let (ch, resp) = row.inner;
        changed |= ch;
        let v = current(ctx.params, d);
        if let Some(msg) = d.check(&v) {
            if let Some(r) = resp {
                ui.painter().rect_stroke(r.rect.expand(1.0), 3.0, Stroke::new(1.5, theme.colors.icon_red), StrokeKind::Outside);
            }
            errors.push(msg);
        }
    }
    for e in errors {
        ui.add_space(sp::XS);
        ui.label(egui::RichText::new(e).color(theme.colors.icon_red).size(type_scale::CAPTION));
    }
    if ctx.mode == Mode::Beginner && (!hidden.is_empty() || *ctx.show_more) && ctx.defs.iter().any(|d| !d.beginner) {
        ui.add_space(sp::XS);
        let key = if *ctx.show_more { "batch-ui-step-less" } else { "batch-ui-step-more" };
        if ui_kit::more_link(ui, &t(key), None).clicked() {
            *ctx.show_more = !*ctx.show_more;
        }
    }
    changed
}

fn set(params: &mut Value, name: &str, v: Value) -> bool {
    if !params.is_object() {
        *params = json!({});
    }
    let o = params.as_object_mut().expect("Objekt");
    if o.get(name) == Some(&v) {
        return false;
    }
    o.insert(name.to_string(), v);
    true
}

/// Ein Eingabeelement. Liefert (geändert, Antwort für die rote Kante).
fn widget(ui: &mut Ui, ctx: &mut CardCtx, d: &ParamDef, label: &str, w: f32) -> (bool, Option<egui::Response>) {
    crate::theme::field_widgets(ui);
    let id = Id::new(("fp-batch-param", ctx.step_id, ctx.index, &d.name));
    let v = current(ctx.params, d);
    let unit = d.unit.clone().map(|u| format!(" {u}")).unwrap_or_default();
    match &d.ty {
        ParamType::Bool => {
            let mut b = v.as_bool().unwrap_or(false);
            let r = ui.checkbox(&mut b, "");
            r.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, b, label));
            let ch = r.changed() && set(ctx.params, &d.name, json!(b));
            (ch, Some(r))
        }
        ParamType::Int { min, max } => {
            let mut x = v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)).unwrap_or(*min);
            ui.spacing_mut().slider_width = (w - 70.0).max(40.0);
            let span = (*max - *min) as f64;
            let r = ui.add(egui::Slider::new(&mut x, *min..=*max).suffix(unit).logarithmic(span > 2000.0 && *min >= 0).clamping(egui::SliderClamping::Always));
            r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Slider, true, label));
            let ch = r.changed() && set(ctx.params, &d.name, json!(x));
            (ch, Some(r))
        }
        ParamType::Number { min, max } => {
            let mut x = v.as_f64().unwrap_or(*min);
            ui.spacing_mut().slider_width = (w - 70.0).max(40.0);
            let span = max - min;
            let r = ui.add(egui::Slider::new(&mut x, *min..=*max).suffix(unit).logarithmic(span > 2000.0 && *min >= 0.0).clamping(egui::SliderClamping::Always));
            r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Slider, true, label));
            let ch = r.changed() && set(ctx.params, &d.name, json!(x));
            (ch, Some(r))
        }
        ParamType::Enum { values } => {
            let cur = v.as_str().unwrap_or_default().to_string();
            let mut sel = cur.clone();
            let r = egui::ComboBox::from_id_salt(id).selected_text(value_label(ctx.step_id, &d.name, &cur)).width((w - 8.0).min(260.0)).show_ui(ui, |ui| {
                for val in values {
                    ui.selectable_value(&mut sel, val.clone(), value_label(ctx.step_id, &d.name, val));
                }
            });
            let ch = sel != cur && set(ctx.params, &d.name, json!(sel));
            (ch, Some(r.response))
        }
        ParamType::Multi { values } => {
            let cur: Vec<String> = v.as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default();
            let mut next = cur.clone();
            let r = ui.horizontal_wrapped(|ui| {
                ui.set_max_width(w);
                for val in values {
                    let mut on = next.contains(val);
                    if ui.checkbox(&mut on, value_label(ctx.step_id, &d.name, val)).changed() {
                        if on {
                            next.push(val.clone());
                        } else {
                            next.retain(|x| x != val);
                        }
                    }
                }
            });
            let ch = next != cur && set(ctx.params, &d.name, json!(next));
            (ch, Some(r.response))
        }
        ParamType::Color => {
            let s = v.as_str().unwrap_or("#000000").to_string();
            let rgba = fp_batch::step::parse_color(&s).unwrap_or([0, 0, 0, 255]);
            let mut col = Color32::from_rgba_unmultiplied(rgba[0], rgba[1], rgba[2], rgba[3]);
            let mut ch = false;
            let r = ui.horizontal(|ui| {
                let cr = egui::widgets::color_picker::color_edit_button_srgba(ui, &mut col, egui::widgets::color_picker::Alpha::OnlyBlend);
                if cr.changed() {
                    let [r, g, b, a] = col.to_srgba_unmultiplied();
                    let hex = if a == 255 { format!("#{r:02x}{g:02x}{b:02x}") } else { format!("#{r:02x}{g:02x}{b:02x}{a:02x}") };
                    ch |= set(ctx.params, &d.name, json!(hex));
                }
                let mut text = s.clone();
                let tr = ui.add(egui::TextEdit::singleline(&mut text).desired_width((w - 50.0).clamp(40.0, 110.0)));
                tr.widget_info(|| egui::WidgetInfo::text_edit(true, "", label, ""));
                if tr.changed() {
                    ch |= set(ctx.params, &d.name, json!(text));
                }
            });
            (ch, Some(r.response))
        }
        ParamType::Text | ParamType::Time => {
            let mut text = match &v {
                Value::String(s) => s.clone(),
                Value::Null => String::new(),
                other => other.to_string(),
            };
            let r = ui.add(egui::TextEdit::singleline(&mut text).desired_width(w));
            r.widget_info(|| egui::WidgetInfo::text_edit(true, "", label, ""));
            let ch = r.changed() && set(ctx.params, &d.name, if matches!(d.ty, ParamType::Time) && text.trim().parse::<f64>().is_ok() { json!(text.trim().parse::<f64>().unwrap_or(0.0)) } else { json!(text) });
            (ch, Some(r))
        }
        ParamType::Path => {
            let mut text = v.as_str().unwrap_or_default().to_string();
            let mut ch = false;
            let r = ui.horizontal(|ui| {
                let tr = ui.add(egui::TextEdit::singleline(&mut text).desired_width((w - 34.0).max(40.0)));
                tr.widget_info(|| egui::WidgetInfo::text_edit(true, "", label, ""));
                if tr.changed() {
                    ch |= set(ctx.params, &d.name, json!(text));
                }
                if ui.button("…").on_hover_text(t("batch-ui-choose-file")).clicked() {
                    ctx.pick_path = Some(d.name.clone());
                }
            });
            (ch, Some(r.response))
        }
        ParamType::Json => {
            let buf_id = id.with("buf");
            let mut text = ui.ctx().data(|m| m.get_temp::<String>(buf_id)).unwrap_or_else(|| serde_json::to_string_pretty(&v).unwrap_or_default());
            let r = ui.add(egui::TextEdit::multiline(&mut text).desired_width(w).desired_rows(3).code_editor());
            r.widget_info(|| egui::WidgetInfo::text_edit(true, "", label, ""));
            let mut ch = false;
            if r.changed() {
                if let Ok(parsed) = serde_json::from_str::<Value>(&text) {
                    ch = set(ctx.params, &d.name, parsed);
                }
                ui.ctx().data_mut(|m| m.insert_temp(buf_id, text));
            } else if !r.has_focus() {
                ui.ctx().data_mut(|m| m.remove::<String>(buf_id));
            }
            (ch, Some(r))
        }
    }
}

/// Automatische Zusammenfassung (wenn der Schritt keine eigene liefert): sichtbare Parameter
/// „Name Wert“, dazu „+ N Profi-Einstellungen“, wenn versteckte Werte von der Vorgabe abweichen.
pub fn auto_summary(step_id: &str, defs: &[ParamDef], params: &Value, mode: Mode) -> String {
    let mut parts = Vec::new();
    for d in defs.iter().filter(|d| mode == Mode::Pro || d.beginner).take(4) {
        let v = current(params, d);
        let txt = value_text(step_id, d, &v);
        if !txt.is_empty() {
            parts.push(format!("{} {}", param_label(step_id, &d.name), txt));
        }
    }
    let hidden_changed = if mode == Mode::Beginner { defs.iter().filter(|d| !d.beginner && params.get(&d.name).is_some_and(|v| *v != d.default)).count() } else { 0 };
    let mut s = parts.join(", ");
    if hidden_changed > 0 {
        let extra = fp_i18n::tr!("batch-ui-step-hidden-pro", n = hidden_changed);
        s = if s.is_empty() { extra } else { format!("{s} · {extra}") };
    }
    s
}
