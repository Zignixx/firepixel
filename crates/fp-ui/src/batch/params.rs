//! Formular einer aufgeklappten Schrittkarte aus dem Parameter-Schema (`Step::params`,
//! SPEC-BATCH §2.4, Feinschliff BATCH-11a): Beschriftung-Wert-Raster (DESIGN §5) im Stil des
//! Startbildschirms.
//!
//! - **Zahlen**: Schieber (gefüllte Spur) + Zahlenfeld (Dezimalkomma je Sprache, Einheit gedämpft
//!   dahinter, Tippen außerhalb des Bereichs = rote Kante + Meldung, Wert bleibt). Werte ohne
//!   Vorgabe zeigen „Automatisch“ im leeren Feld.
//! - **Auswahl**: bis drei kurze Werte als Segment-Knopf, sonst Liste; Werte ohne Vorgabe mit dem
//!   Eintrag „Automatisch“. **Mehrfachauswahl** als Chips.
//! - **Kästchen**: Schalter (ohne Vorgabe: Automatisch | An | Aus).
//! - **Farbe**: Farbfeld mit dem Farbwähler-Popup des Originals (FARB-1b: Raster, HEX/RGB,
//!   Pipette) und HEX-Feld; „Ohne“ = transparent.
//! - **Datei**: Feld + „Wählen…“ (Datei-Dialog über [`CardCtx::pick_path`]); **Text**/**Zeit**:
//!   Feld; **JSON** (Profi): Code-Feld.
//! - Sichtbarkeit: Anfänger-Modus nur die mit `beginner()` markierten Parameter („Mehr Optionen“
//!   klappt den Rest auf), Felder mit Bedingung (`ParamDef::when`) nur passend zum Modus des
//!   Schritts. Versteckte Werte bleiben erhalten.
//! - ↺ am Zeilenende setzt einen geänderten Wert auf die Vorgabe zurück (bzw. auf „Automatisch“).
//! - Jede Änderung geht über `Studio::edit` (ein Rückgängig-Schritt je Geste und Parameter) und
//!   stößt die entprellte Live-Vorschau an.
//!
//! **Eigene Kartenformulare** (BATCH-3 ff.): ein Schritt kann statt des generischen Formulars ein
//! eigenes anmelden – `register_card_body("rename", my_body)`; `my_body(ui, ctx) -> bool` bearbeitet
//! `ctx.params` (JSON-Objekt) und meldet `true` bei Änderung. Für die Endkarten heißen die IDs
//! `rename` (Parameter = `RenameSpec` als JSON) und `output` (Parameter = `format` + Formatoptionen).

use super::kit::{self, BtnKind};
use super::Mode;
use crate::app_state::AppState;
use crate::theme::{mix, with_alpha, Theme};
use crate::ui_kit::{self, sp, type_scale, RowIcon};
use egui::{pos2, vec2, Align2, Color32, CornerRadius, Id, Rect, Sense, Stroke, StrokeKind, Ui};
use fp_batch::step::{param_key, value_key, ParamDef, ParamType};
use fp_batch::{ItemInfo, Recipe};
use fp_i18n::t;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

/// Höhe einer Formularzeile.
pub const ROW_H: f32 = 26.0;
/// Breite des Zahlenfelds.
const FIELD_W: f32 = 64.0;
/// Breite des Zurücksetzen-Knopfs am Zeilenende.
const RESET_W: f32 = 20.0;

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
    /// App-Zustand für den Farbwähler des Originals (Pipette über Leinwand und Fenster); `None`
    /// (z. B. in Tests ohne App) → einfacher Farbwähler.
    pub app: Option<&'a AppState>,
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

/// Beschriftung eines Auswahlwerts – ein Schema für alle Schritte (BATCH-11a):
/// `batch-value-<id>-<param>-<wert>`. Formate erscheinen als Eigennamen, Befehle des Registers
/// (`filter.registry`) mit ihrem Menütext; sonst der Wert selbst (Zahlen, Verhältnisse).
pub fn value_label(step_id: &str, name: &str, value: &str) -> String {
    if name == "format"
        && let Some(f) = fp_batch::output::OutputFormat::parse(value)
    {
        return if f == fp_batch::output::OutputFormat::Same { t("batch-ui-output-same") } else { f.display_name().to_string() };
    }
    let k = value_key(step_id, name, value);
    if fp_i18n::has(&k) {
        return t(&k);
    }
    if value.contains('.') {
        let ck = fp_i18n::command_key(value);
        if fp_i18n::has(&ck) {
            return t(&ck).replace("...", "").replace('…', "").trim().to_string();
        }
    }
    value.to_string()
}

/// Wert eines Parameters als kurzer Text (Zusammenfassung).
pub fn value_text(step_id: &str, d: &ParamDef, v: &Value) -> String {
    let unit = d.unit.as_deref().map(|u| format!(" {u}")).unwrap_or_default();
    match (&d.ty, v) {
        (_, Value::Null) => String::new(),
        (ParamType::Bool, Value::Bool(b)) => t(if *b { "batch-ui-on" } else { "batch-ui-off" }),
        (ParamType::Enum { .. }, Value::String(s)) => value_label(step_id, &d.name, s),
        (ParamType::Multi { .. }, Value::Array(a)) => a.iter().filter_map(|x| x.as_str()).map(|s| value_label(step_id, &d.name, s)).collect::<Vec<_>>().join(", "),
        (ParamType::Int { .. }, v) => format!("{}{unit}", v.as_i64().or_else(|| v.as_f64().map(|f| f.round() as i64)).unwrap_or(0)),
        (ParamType::Number { .. }, v) => format!("{}{unit}", fp_i18n::fmt_number(v.as_f64().unwrap_or(0.0), 0, 2)),
        (ParamType::Color, Value::String(s)) if s.eq_ignore_ascii_case("transparent") => t("batch-form-transparent"),
        (_, Value::String(s)) => s.clone(),
        (_, v) => v.to_string(),
    }
}

/// Wert mit Vorgabe.
pub fn current(params: &Value, d: &ParamDef) -> Value {
    params.get(&d.name).filter(|v| !v.is_null()).cloned().unwrap_or_else(|| d.default.clone())
}

/// Sichtbare Parameter je Modus (ohne Bedingungen). Liefert (sichtbar, versteckt).
pub fn split_defs(defs: &[ParamDef], mode: Mode, show_more: bool) -> (Vec<&ParamDef>, Vec<&ParamDef>) {
    if mode == Mode::Pro || show_more {
        return (defs.iter().collect(), Vec::new());
    }
    defs.iter().partition(|d| d.beginner)
}

/// Felder des Formulars: Bedingungen (`ParamDef::when`) und Modus. Liefert (sichtbar, Zahl der
/// nur im Profi-Umfang bzw. unter „Mehr Optionen“ liegenden Felder).
pub fn form_defs<'d>(defs: &'d [ParamDef], params: &Value, mode: Mode, show_more: bool) -> (Vec<&'d ParamDef>, usize) {
    let fits: Vec<&ParamDef> = defs.iter().filter(|d| d.is_visible(defs, params)).collect();
    if mode == Mode::Pro || show_more {
        return (fits, 0);
    }
    let hidden = fits.iter().filter(|d| !d.beginner).count();
    (fits.into_iter().filter(|d| d.beginner).collect(), hidden)
}

/// Generisches Formular. Liefert `true` bei Änderung.
pub fn generic_body(ui: &mut Ui, ctx: &mut CardCtx) -> bool {
    let theme = Theme::current(ui.ctx());
    if ctx.defs.is_empty() {
        ui.label(egui::RichText::new(t("batch-ui-step-no-params")).color(kit::weak(&theme)).size(type_scale::CAPTION));
        return false;
    }
    let snapshot = ctx.params.clone();
    let (shown, hidden) = form_defs(ctx.defs, &snapshot, ctx.mode, *ctx.show_more);
    let mut changed = false;
    let labels: Vec<String> = shown.iter().map(|d| param_label(ctx.step_id, &d.name)).collect();
    let refs: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
    let avail = ui.available_width();
    let label_w = ui_kit::label_width(ui.ctx(), &refs, 70.0).min((avail * 0.4).max(60.0)).floor();
    let mut errors: Vec<String> = Vec::new();
    for (d, label) in shown.iter().zip(&labels) {
        let row = ui.horizontal(|ui| {
            ui.set_min_height(ROW_H);
            ui.spacing_mut().item_spacing.x = sp::S;
            let (lr, _) = ui.allocate_exact_size(vec2(label_w, ROW_H), Sense::hover());
            let font = theme.font(type_scale::BODY);
            let shown_l = ui_kit::elide(ui.ctx(), label, &font, label_w - sp::XS);
            ui.painter().text(lr.left_center(), Align2::LEFT_CENTER, &shown_l, font, kit::weak(&theme));
            let tip_key = format!("{}.tooltip", param_key(ctx.step_id, &d.name));
            let lresp = ui.interact(lr, Id::new(("fp-batch-param-label", ctx.step_id, ctx.index, &d.name)), Sense::hover());
            if fp_i18n::has(&tip_key) {
                lresp.on_hover_text(t(&tip_key));
            } else if shown_l != *label {
                lresp.on_hover_text(label.as_str());
            }
            let w = (ui.available_width() - RESET_W - sp::S).max(40.0);
            let out = ui.add_enabled_ui(ctx.enabled, |ui| widget(ui, ctx, d, label, w)).inner;
            // ↺ zurücksetzen (nur wenn der Wert von der Vorgabe abweicht)
            let explicit = ctx.params.get(&d.name).is_some_and(|v| !v.is_null());
            let differs = explicit && current(ctx.params, d) != d.default;
            let rest = (ui.available_width()).max(0.0);
            let (rr, _) = ui.allocate_exact_size(vec2(rest, ROW_H), Sense::hover());
            let mut reset = false;
            if differs && ctx.enabled && rr.width() >= 16.0 {
                let r = Rect::from_center_size(pos2(rr.right() - RESET_W / 2.0, rr.center().y), vec2(RESET_W, RESET_W));
                let name = if d.default.is_null() { t("batch-form-reset-auto") } else { t("batch-form-reset") };
                let resp = kit::icon_button(ui, r, Id::new(("fp-batch-param-reset", ctx.step_id, ctx.index, &d.name)), RowIcon::Icon(crate::icons::Icon::Undo), &format!("{name}: {label}"), true);
                reset = resp.clicked();
            }
            (out, reset)
        });
        let (out, reset) = row.inner;
        changed |= out.changed;
        if reset {
            changed |= unset(ctx.params, &d.name);
        }
        let v = current(ctx.params, d);
        let err = out.error.or_else(|| if v.is_null() { None } else { d.check(&v) });
        if let Some(msg) = err {
            if let Some(r) = out.resp {
                ui.painter().rect_stroke(r.rect.expand(1.0), 4.0, Stroke::new(1.5, theme.colors.icon_red), StrokeKind::Outside);
            }
            errors.push(msg);
        }
    }
    for e in errors {
        ui.add_space(sp::XS);
        ui.label(egui::RichText::new(e).color(theme.colors.icon_red).size(type_scale::CAPTION));
    }
    if ctx.mode == Mode::Beginner && (hidden > 0 || *ctx.show_more) && ctx.defs.iter().any(|d| !d.beginner) {
        ui.add_space(sp::XS);
        let text = if *ctx.show_more { t("batch-ui-step-less") } else { fp_i18n::tr!("batch-form-more-count", label = t("batch-ui-step-more"), n = hidden) };
        if ui_kit::more_link(ui, &text, None).clicked() {
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

/// Parameter entfernen (Vorgabe bzw. „Automatisch“ gilt wieder).
fn unset(params: &mut Value, name: &str) -> bool {
    params.as_object_mut().is_some_and(|o| o.remove(name).is_some())
}

/// Ergebnis eines Eingabeelements.
struct Out {
    changed: bool,
    /// Antwort für die rote Kante.
    resp: Option<egui::Response>,
    /// Eigene Fehlermeldung (z. B. Tippfehler im Zahlenfeld).
    error: Option<String>,
}

impl Out {
    fn new(changed: bool, resp: egui::Response) -> Out {
        Out { changed, resp: Some(resp), error: None }
    }
}

/// Zahl aus einem Feld: Dezimalkomma oder -punkt, Leerzeichen und Einheit werden ignoriert.
pub fn parse_number(text: &str, unit: Option<&str>) -> Option<f64> {
    let mut s: String = text.trim().to_string();
    if let Some(u) = unit
        && let Some(stripped) = s.strip_suffix(u)
    {
        s = stripped.to_string();
    }
    let s: String = s.chars().filter(|c| !c.is_whitespace() && *c != '\u{a0}').collect::<String>().replace('−', "-").replace(',', ".");
    if s.is_empty() {
        return None;
    }
    s.parse::<f64>().ok().filter(|x| x.is_finite())
}

/// Zahl für das Feld (Sprache: Dezimalkomma).
pub fn format_number(x: f64, int: bool) -> String {
    if int { format!("{}", x.round() as i64) } else { fp_i18n::fmt_number(x, 0, 2) }
}

/// Ein Eingabeelement.
fn widget(ui: &mut Ui, ctx: &mut CardCtx, d: &ParamDef, label: &str, w: f32) -> Out {
    crate::theme::field_widgets(ui);
    let id = Id::new(("fp-batch-param", ctx.step_id, ctx.index, &d.name));
    match &d.ty {
        ParamType::Bool => bool_widget(ui, ctx, d, label, id),
        ParamType::Int { min, max } => number_widget(ui, ctx, d, label, w, id, *min as f64, *max as f64, true),
        ParamType::Number { min, max } => number_widget(ui, ctx, d, label, w, id, *min, *max, false),
        ParamType::Enum { values } => enum_widget(ui, ctx, d, label, w, id, values),
        ParamType::Multi { values } => multi_widget(ui, ctx, d, label, w, values),
        ParamType::Color => color_widget(ui, ctx, d, label, w, id),
        ParamType::Text | ParamType::Time => {
            let v = current(ctx.params, d);
            let mut text = match &v {
                Value::String(s) => s.clone(),
                Value::Null => String::new(),
                other => other.to_string(),
            };
            let hint = if matches!(d.ty, ParamType::Time) { t("batch-form-time-hint") } else { String::new() };
            let r = ui.add(egui::TextEdit::singleline(&mut text).id(id).desired_width(w).hint_text(hint));
            r.widget_info(|| text_info(label));
            let nv = if matches!(d.ty, ParamType::Time) && text.trim().parse::<f64>().is_ok() { json!(text.trim().parse::<f64>().unwrap_or(0.0)) } else { json!(text) };
            let ch = r.changed() && set(ctx.params, &d.name, nv);
            Out::new(ch, r)
        }
        ParamType::Path => {
            let mut text = current(ctx.params, d).as_str().unwrap_or_default().to_string();
            let mut ch = false;
            let btn_l = t("batch-form-choose");
            let bw = kit::btn_w(ui.ctx(), &btn_l, true).min(w * 0.5);
            let r = ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = sp::XS + 2.0;
                let tr = ui.add(egui::TextEdit::singleline(&mut text).id(id).desired_width((w - bw - sp::XS - 2.0).max(40.0)).hint_text(t("batch-form-no-file")));
                tr.widget_info(|| text_info(label));
                if tr.changed() {
                    ch |= if text.is_empty() { unset(ctx.params, &d.name) } else { set(ctx.params, &d.name, json!(text)) };
                }
                let (br, _) = ui.allocate_exact_size(vec2(bw, 22.0), Sense::hover());
                let resp = kit::button(ui, br, id.with("pick"), &btn_l, Some(RowIcon::Icon(crate::icons::Icon::Folder)), BtnKind::Secondary, ctx.enabled, &format!("{btn_l} {label}"));
                if resp.clicked() {
                    ctx.pick_path = Some(d.name.clone());
                }
                tr
            });
            Out::new(ch, r.inner)
        }
        ParamType::Json => {
            let v = current(ctx.params, d);
            let buf_id = id.with("buf");
            let mut text = ui.ctx().data(|m| m.get_temp::<String>(buf_id)).unwrap_or_else(|| serde_json::to_string_pretty(&v).unwrap_or_default());
            let r = ui.add(egui::TextEdit::multiline(&mut text).id(id).desired_width(w).desired_rows(3).code_editor());
            r.widget_info(|| text_info(label));
            let mut ch = false;
            let mut error = None;
            if r.changed() {
                match serde_json::from_str::<Value>(&text) {
                    Ok(parsed) => ch = set(ctx.params, &d.name, parsed),
                    Err(_) => error = Some(t("batch-form-json-invalid")),
                }
                ui.ctx().data_mut(|m| m.insert_temp(buf_id, text.clone()));
            } else if !r.has_focus() {
                ui.ctx().data_mut(|m| m.remove::<String>(buf_id));
            } else if serde_json::from_str::<Value>(&text).is_err() {
                error = Some(t("batch-form-json-invalid"));
            }
            Out { changed: ch, resp: Some(r), error }
        }
    }
}

/// Schalter; ohne Vorgabe (`null`) als Segment Automatisch | An | Aus.
fn bool_widget(ui: &mut Ui, ctx: &mut CardCtx, d: &ParamDef, label: &str, id: Id) -> Out {
    let v = current(ctx.params, d);
    if d.default.is_null() {
        let labels = vec![t("batch-form-auto"), t("batch-form-on"), t("batch-form-off")];
        let cur = match v.as_bool() {
            None => 0,
            Some(true) => 1,
            Some(false) => 2,
        };
        let w: f32 = labels.iter().map(|l| kit::btn_w(ui.ctx(), l, false)).sum::<f32>() + 4.0;
        let (r, resp) = ui.allocate_exact_size(vec2(w, 22.0), Sense::hover());
        let mut ch = false;
        if let Some(k) = kit::segmented(ui, r, id, &labels, cur, false) {
            ch = match k {
                0 => unset(ctx.params, &d.name),
                1 => set(ctx.params, &d.name, json!(true)),
                _ => set(ctx.params, &d.name, json!(false)),
            };
        }
        return Out::new(ch, resp);
    }
    let on = v.as_bool().unwrap_or(false);
    let (r, _) = ui.allocate_exact_size(vec2(36.0, 22.0), Sense::hover());
    let resp = kit::toggle(ui, r, id, on, label);
    let ch = resp.clicked() && ctx.enabled && set(ctx.params, &d.name, json!(!on));
    Out::new(ch, resp)
}

/// Schieber + Zahlenfeld + Einheit.
#[allow(clippy::too_many_arguments)]
fn number_widget(ui: &mut Ui, ctx: &mut CardCtx, d: &ParamDef, label: &str, w: f32, id: Id, min: f64, max: f64, int: bool) -> Out {
    let theme = Theme::current(ui.ctx());
    let v = current(ctx.params, d);
    let auto = v.is_null();
    let x = v.as_f64().unwrap_or_else(|| d.default.as_f64().unwrap_or(if min <= 0.0 && max >= 0.0 { 0.0 } else { min }));
    let unit = d.unit.clone();
    let font = theme.font(type_scale::BODY);
    let unit_w = unit.as_deref().map(|u| ui_kit::text_w(ui.ctx(), u, &font) + 2.0).unwrap_or(0.0);
    let gap = sp::S - 2.0;
    let slider_w = w - FIELD_W - unit_w - if unit.is_some() { 2.0 * gap } else { gap };
    let mut ch = false;
    let mut error = None;
    let r = ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        if slider_w >= 48.0 && max > min {
            ui.spacing_mut().slider_width = slider_w;
            if theme.glow {
                // OLED: gefüllte Spur in der Leitfarbe, sonst verschwindet sie im Schwarz
                ui.visuals_mut().selection.bg_fill = with_alpha(theme.colors.focus, 150);
                ui.visuals_mut().widgets.inactive.bg_fill = mix(Color32::BLACK, theme.colors.line_mid, 0.6);
            }
            let span = max - min;
            let log = span > 2000.0 && min >= 0.0;
            let mut xx = x.clamp(min, max);
            let s = egui::Slider::new(&mut xx, min..=max).show_value(false).trailing_fill(!auto).handle_shape(egui::style::HandleShape::Circle).logarithmic(log).clamping(egui::SliderClamping::Always);
            let s = if int { s.integer() } else { s };
            let sr = ui.add(s);
            sr.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Slider, true, label));
            if sr.changed() {
                let nv = if int { json!(xx.round() as i64) } else { json!((xx * 100.0).round() / 100.0) };
                ch |= set(ctx.params, &d.name, nv);
            }
        }
        // Zahlenfeld: beim Tippen eigener Puffer, sonst der formatierte Wert
        let fid = id.with("num");
        let buf_id = id.with("numbuf");
        let focused = ui.memory(|m| m.has_focus(fid));
        let cur_text = if auto { String::new() } else { format_number(current(ctx.params, d).as_f64().unwrap_or(x), int) };
        let mut buf = if focused { ui.ctx().data(|m| m.get_temp::<String>(buf_id)).unwrap_or_else(|| cur_text.clone()) } else { cur_text };
        let hint = if auto { t("batch-form-auto") } else { String::new() };
        let tr = ui.add(egui::TextEdit::singleline(&mut buf).id(fid).desired_width(FIELD_W).horizontal_align(egui::Align::RIGHT).hint_text(hint));
        tr.widget_info(|| text_info(label));
        if tr.changed() {
            match parse_number(&buf, unit.as_deref()) {
                Some(n) if n >= min && n <= max => {
                    let nv = if int { json!(n.round() as i64) } else { json!(n) };
                    ch |= set(ctx.params, &d.name, nv);
                }
                None if buf.trim().is_empty() && d.default.is_null() => ch |= unset(ctx.params, &d.name),
                _ => {}
            }
            ui.ctx().data_mut(|m| m.insert_temp(buf_id, buf.clone()));
        }
        if focused || tr.has_focus() {
            let bad = !buf.trim().is_empty() && parse_number(&buf, unit.as_deref()).is_none_or(|n| n < min || n > max);
            if bad {
                error = Some(fp_i18n::tr!("batch-form-range", label = label.to_string(), min = format_number(min, int), max = format_number(max, int)));
            }
        } else {
            ui.ctx().data_mut(|m| m.remove::<String>(buf_id));
        }
        if let Some(u) = &unit {
            ui.label(egui::RichText::new(u).color(kit::weak(&theme)).size(type_scale::BODY));
        }
        tr
    });
    Out { changed: ch, resp: Some(r.inner), error }
}

/// Auswahl: Segment-Knopf (bis drei kurze Werte) oder Liste.
#[allow(clippy::too_many_arguments)]
fn enum_widget(ui: &mut Ui, ctx: &mut CardCtx, d: &ParamDef, label: &str, w: f32, id: Id, values: &[String]) -> Out {
    let v = current(ctx.params, d);
    let cur = v.as_str().map(str::to_string);
    let auto = d.default.is_null();
    let mut opts: Vec<(Option<String>, String)> = Vec::new();
    if auto {
        opts.push((None, t("batch-form-auto")));
    }
    opts.extend(values.iter().map(|x| (Some(x.clone()), value_label(ctx.step_id, &d.name, x))));
    let labels: Vec<String> = opts.iter().map(|o| o.1.clone()).collect();
    let seg_w: f32 = labels.iter().map(|l| kit::btn_w(ui.ctx(), l, false)).sum::<f32>() + 4.0;
    let sel = opts.iter().position(|o| o.0 == cur).unwrap_or(0);
    let mut pick: Option<usize> = None;
    let resp = if opts.len() <= 3 && seg_w <= w {
        let (r, resp) = ui.allocate_exact_size(vec2(seg_w.max((w * 0.6).min(seg_w * 1.4)), 22.0), Sense::hover());
        pick = kit::segmented(ui, r, id, &labels, sel, false);
        resp
    } else {
        let mut chosen = sel;
        let r = egui::ComboBox::from_id_salt(id).selected_text(labels.get(sel).cloned().unwrap_or_default()).width((w - 4.0).min(280.0)).show_ui(ui, |ui| {
            for (k, l) in labels.iter().enumerate() {
                ui.selectable_value(&mut chosen, k, l);
            }
        });
        r.response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, label));
        if chosen != sel {
            pick = Some(chosen);
        }
        r.response
    };
    let ch = match pick.and_then(|k| opts.get(k)) {
        Some((None, _)) => unset(ctx.params, &d.name),
        Some((Some(x), _)) => set(ctx.params, &d.name, json!(x)),
        None => false,
    };
    Out::new(ch, resp)
}

/// Mehrfachauswahl als Chips.
fn multi_widget(ui: &mut Ui, ctx: &mut CardCtx, d: &ParamDef, label: &str, w: f32, values: &[String]) -> Out {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let cur: Vec<String> = current(ctx.params, d).as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default();
    let mut next = cur.clone();
    let font = theme.font(type_scale::CAPTION);
    let r = ui.horizontal_wrapped(|ui| {
        ui.set_max_width(w);
        ui.spacing_mut().item_spacing = vec2(sp::XS + 2.0, sp::XS + 2.0);
        for val in values {
            let text = value_label(ctx.step_id, &d.name, val);
            let on = next.contains(val);
            let cw = ui_kit::text_w(ui.ctx(), &text, &font) + 2.0 * sp::S + if on { 12.0 } else { 0.0 };
            let (cr, resp) = ui.allocate_exact_size(vec2(cw.min(w), 22.0), Sense::click());
            resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, ctx.enabled, on, format!("{label}: {text}")));
            let hot = resp.hovered() || resp.has_focus();
            let fill = if on { if theme.glow { with_alpha(c.focus, 60) } else { kit::tint(&theme, c.focus, if hot { 1.8 } else { 1.3 }) } } else if hot { kit::tint(&theme, c.focus, 0.6) } else { kit::item_fill(&theme) };
            let border = if on { mix(kit::card_fill(&theme), c.focus, 0.7) } else { kit::item_border(&theme, c.focus) };
            let p = ui.painter();
            p.rect_filled(cr, CornerRadius::same(11), fill);
            p.rect_stroke(cr, CornerRadius::same(11), Stroke::new(1.0, border), StrokeKind::Inside);
            if resp.has_focus() {
                p.rect_stroke(cr.expand(1.5), CornerRadius::same(12), Stroke::new(1.5, c.focus), StrokeKind::Outside);
            }
            let mut tx = cr.left() + sp::S;
            if on {
                // Häkchen
                let cy = cr.center().y;
                let s = Stroke::new(1.6, if theme.glow { c.focus } else { mix(c.focus, c.text, 0.25) });
                p.line_segment([pos2(tx, cy), pos2(tx + 3.0, cy + 3.0)], s);
                p.line_segment([pos2(tx + 3.0, cy + 3.0), pos2(tx + 8.0, cy - 3.5)], s);
                tx += 12.0;
            }
            let shown = ui_kit::elide(ui.ctx(), &text, &font, (cr.right() - sp::S - tx).max(8.0));
            p.text(pos2(tx, cr.center().y), Align2::LEFT_CENTER, shown, font.clone(), if ctx.enabled { c.text } else { c.text_disabled });
            if resp.hovered() && ctx.enabled {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if resp.clicked() && ctx.enabled {
                if on {
                    next.retain(|x| x != val);
                } else {
                    next.push(val.clone());
                }
            }
        }
    });
    // Reihenfolge wie im Schema (stabil für Rezept-Dateien)
    next.sort_by_key(|x| values.iter().position(|y| y == x).unwrap_or(usize::MAX));
    let ch = next != cur && set(ctx.params, &d.name, json!(next));
    Out::new(ch, r.response)
}

fn argb_of(rgba: [u8; 4]) -> fp_color::Argb {
    fp_color::Argb((rgba[3] as u32) << 24 | (rgba[0] as u32) << 16 | (rgba[1] as u32) << 8 | rgba[2] as u32)
}

fn hex_of(c: fp_color::Argb) -> String {
    if c.a() == 255 { format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b()) } else { format!("#{:02x}{:02x}{:02x}{:02x}", c.r(), c.g(), c.b(), c.a()) }
}

/// Farbfeld (Farbwähler des Originals) + HEX-Feld.
fn color_widget(ui: &mut Ui, ctx: &mut CardCtx, d: &ParamDef, label: &str, w: f32, id: Id) -> Out {
    let theme = Theme::current(ui.ctx());
    let v = current(ctx.params, d);
    let s = v.as_str().unwrap_or("#000000").to_string();
    let rgba = fp_batch::step::parse_color(&s).unwrap_or([0, 0, 0, 255]);
    let transparent = rgba[3] == 0;
    let mut ch = false;
    let r = ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = sp::S - 2.0;
        let (sr, _) = ui.allocate_exact_size(vec2(34.0, 22.0), Sense::hover());
        let sresp = ui.interact(sr, id.with("swatch"), if ctx.enabled { Sense::click() } else { Sense::hover() });
        sresp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::ColorButton, ctx.enabled, label));
        let col = Color32::from_rgba_unmultiplied(rgba[0], rgba[1], rgba[2], rgba[3]);
        crate::widgets::color_well(ui.painter(), sr, col, &theme);
        if ctx.enabled && (sresp.hovered() || sresp.has_focus()) {
            ui.painter().rect_stroke(sr, 0.0, Stroke::new(1.0, theme.colors.focus), StrokeKind::Inside);
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        let sresp = sresp.on_hover_text(t("batch-form-color.tooltip"));
        match ctx.app {
            Some(app) => {
                let footer = if d.default.as_str().is_some_and(|x| x.eq_ignore_ascii_case("transparent")) || transparent { crate::inspector::color_picker::Footer::NoneOnly } else { crate::inspector::color_picker::Footer::Plain };
                match crate::inspector::color_picker::popup(ui, &sresp, app, (!transparent).then(|| argb_of(rgba)), footer) {
                    Some(crate::inspector::popups::ColorChoice::Color(c)) => ch |= set(ctx.params, &d.name, json!(hex_of(c))),
                    Some(crate::inspector::popups::ColorChoice::None) => ch |= set(ctx.params, &d.name, json!("transparent")),
                    _ => {}
                }
            }
            None => {
                // ohne App (Tests): einfacher Farbwähler von egui
                let mut c32 = col;
                let pid = id.with("egui-picker");
                if sresp.clicked() {
                    egui::Popup::toggle_id(ui.ctx(), pid);
                }
                egui::Popup::from_response(&sresp).id(pid).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
                    if egui::widgets::color_picker::color_picker_color32(ui, &mut c32, egui::widgets::color_picker::Alpha::OnlyBlend) {
                        let [r, g, b, a] = c32.to_srgba_unmultiplied();
                        ch |= set(ctx.params, &d.name, json!(hex_of(argb_of([r, g, b, a]))));
                    }
                });
            }
        }
        // beim Tippen eigener Puffer (Zwischenstände wie „#12“ sind noch keine Farbe)
        let buf_id = id.with("hexbuf");
        let shown = if transparent { t("batch-form-transparent") } else { s.clone() };
        let focused = ui.memory(|m| m.has_focus(id));
        let mut text = if focused { ui.ctx().data(|m| m.get_temp::<String>(buf_id)).unwrap_or(shown) } else { shown };
        let tw = (w - 34.0 - sp::S).clamp(60.0, 110.0);
        let tr = ui.add(egui::TextEdit::singleline(&mut text).id(id).desired_width(tw).font(egui::TextStyle::Monospace));
        tr.widget_info(|| text_info(label));
        if tr.changed() {
            let tx = text.trim();
            if fp_batch::step::parse_color(tx).is_some() {
                ch |= set(ctx.params, &d.name, json!(tx));
            } else if tx.eq_ignore_ascii_case(&t("batch-form-transparent")) {
                ch |= set(ctx.params, &d.name, json!("transparent"));
            }
            ui.ctx().data_mut(|m| m.insert_temp(buf_id, text.clone()));
        }
        if !(focused || tr.has_focus()) {
            ui.ctx().data_mut(|m| m.remove::<String>(buf_id));
        }
        sresp
    });
    Out::new(ch, r.inner)
}

/// Automatische Zusammenfassung (wenn der Schritt keine eigene liefert): sichtbare Parameter
/// „Name Wert“ (nur die zum Modus des Schritts passenden), dazu „+ N Profi-Einstellungen“, wenn
/// versteckte Werte von der Vorgabe abweichen.
pub fn auto_summary(step_id: &str, defs: &[ParamDef], params: &Value, mode: Mode) -> String {
    let mut parts = Vec::new();
    for d in defs.iter().filter(|d| (mode == Mode::Pro || d.beginner) && d.is_visible(defs, params) && !matches!(d.ty, ParamType::Json)).take(4) {
        let v = current(params, d);
        let txt = value_text(step_id, d, &v);
        if !txt.is_empty() {
            parts.push(format!("{} {}", param_label(step_id, &d.name), txt));
        }
    }
    let hidden_changed = if mode == Mode::Beginner { defs.iter().filter(|d| !d.beginner && d.is_visible(defs, params) && params.get(&d.name).is_some_and(|v| !v.is_null() && *v != d.default)).count() } else { 0 };
    let mut s = parts.join(", ");
    if hidden_changed > 0 {
        let extra = fp_i18n::tr!("batch-ui-step-hidden-pro", n = hidden_changed);
        s = if s.is_empty() { extra } else { format!("{s} · {extra}") };
    }
    s
}

/// Barrierefreiheit eines Eingabefelds: Rolle Textfeld mit dem Namen des Parameters (für `ui_find`).
fn text_info(label: &str) -> egui::WidgetInfo {
    let mut i = egui::WidgetInfo::new(egui::WidgetType::TextEdit);
    i.label = Some(label.to_string());
    i
}
