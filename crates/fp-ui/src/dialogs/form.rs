//! Formular-Dialoge: viele Original-Dialoge sind eine Liste beschrifteter Felder mit festen
//! Wertebereichen (`dialoge.json`: DDV-/Schieberbereiche, Kombifeld-Inhalte). [`FormDialog`]
//! beschreibt sie als Daten; daraus entstehen Fenster mit OK/Abbrechen, Bereichsprüfung
//! (ungültige Eingaben sperren OK, Werte werden auf den Bereich geklemmt, 11 §6) und optional
//! **Live-Vorschau**: der Befehl wird angewandt und bei jeder Änderung bzw. beim Abbrechen über
//! den Verlauf zurückgenommen – am Ende bleibt genau ein Verlaufsschritt.

use super::{dialog_frame, push_button, t, DialogClose};
use crate::app_state::AppState;
use crate::inspector::fields;
use crate::theme::Theme;
use egui::{pos2, vec2, Align2, Rect, Sense, Stroke, StrokeKind, Ui};
use serde_json::{json, Map, Value};

/// Art eines Felds.
#[derive(Clone, Debug, PartialEq)]
pub enum FieldKind {
    /// Zahl mit Bereich, Nachkommastellen, optionalem Schieber und Einheit.
    Number { min: f64, max: f64, decimals: usize, slider: bool, unit: &'static str },
    /// Kontrollkästchen.
    Check,
    /// Auswahlliste (Wert = Index bzw. `choice_values[Index]`).
    Choice(Vec<String>),
    /// Optionsfelder (wie Choice, aber nebeneinander).
    Radio(Vec<String>),
    /// Text (ein- oder mehrzeilig).
    Text { multiline: bool },
    /// Farbe (`#rrggbbaa`).
    Color,
    /// Gruppenüberschrift.
    Caption,
    /// Hinweistext.
    Note,
}

/// Einheit eines Zahlenfelds: Textschlüssel (`dlg-…`) übersetzt, Symbole („%“, „°“) wie sie sind.
pub fn unit_text(unit: &str) -> String {
    if unit.starts_with("dlg-") { t(unit) } else { unit.to_string() }
}

/// Ein Feld.
#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    /// Parametername (`_…` = nur Oberfläche, nicht im Befehl).
    pub key: String,
    pub label: String,
    pub kind: FieldKind,
    pub value: Value,
    /// Werte der Auswahlliste (sonst der Index).
    pub choice_values: Option<Vec<Value>>,
    /// Eingabetext (Zahlen), solange er ungültig ist.
    pub error: bool,
    /// Kennlinie des Schiebers (`None` = linear mit dem Schritt der Nachkommastellen).
    pub scale: Option<crate::inspector::scale::Scale>,
    /// Popup-Schieber ▾ neben dem Feld statt einer festen Leiste (wie viele Original-Dialoge).
    pub popup: bool,
    /// Optionsfelder als Symbolknöpfe (Ecken/Endkappen wie im Original), je Eintrag ein [`Pict`].
    pub icons: Option<Vec<Pict>>,
    /// Zahlencodes der Einträge in der Voreinstellung (z. B. `ExpandJoinType`), sonst der Index.
    pub pref_codes: Option<Vec<i64>>,
    /// Optionsfelder untereinander statt nebeneinander (z. B. Leinwandfarbe, 5977).
    pub vertical: bool,
}

/// Bildchen der Symbolknöpfe (selbst gezeichnet).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pict {
    JoinMiter,
    JoinBevel,
    JoinRound,
    CapButt,
    CapSquare,
    CapRound,
}

fn paint_pict(p: &egui::Painter, r: Rect, k: Pict, col: egui::Color32) {
    let s = Stroke::new(1.2, col);
    let c = r.center();
    match k {
        Pict::JoinMiter => {
            p.line_segment([pos2(c.x - 7.0, c.y + 4.0), pos2(c.x, c.y - 4.0)], s);
            p.line_segment([pos2(c.x, c.y - 4.0), pos2(c.x + 7.0, c.y + 4.0)], s);
        }
        Pict::JoinBevel => {
            p.line_segment([pos2(c.x - 7.0, c.y + 4.0), pos2(c.x - 2.0, c.y - 3.0)], s);
            p.line_segment([pos2(c.x - 2.0, c.y - 3.0), pos2(c.x + 2.0, c.y - 3.0)], s);
            p.line_segment([pos2(c.x + 2.0, c.y - 3.0), pos2(c.x + 7.0, c.y + 4.0)], s);
        }
        Pict::JoinRound => {
            let pts: Vec<egui::Pos2> = (0..=12).map(|i| {
                let a = std::f32::consts::PI * (i as f32 / 12.0);
                pos2(c.x - 7.0 * a.cos(), c.y + 4.0 - 8.0 * a.sin())
            }).collect();
            p.add(egui::Shape::line(pts, s));
        }
        Pict::CapButt | Pict::CapSquare | Pict::CapRound => {
            let body = Rect::from_center_size(c, vec2(14.0, 8.0));
            match k {
                Pict::CapButt => {
                    p.rect_stroke(body, 0.0, s, StrokeKind::Inside);
                }
                Pict::CapSquare => {
                    p.rect_stroke(body.expand2(vec2(2.0, 0.0)), 0.0, s, StrokeKind::Inside);
                }
                _ => {
                    p.rect_stroke(body.expand2(vec2(2.0, 0.0)), 4.0, s, StrokeKind::Inside);
                }
            }
            p.line_segment([pos2(body.left(), c.y), pos2(body.right(), c.y)], s);
        }
    }
}

impl Field {
    /// Zahlenfeld.
    pub fn number(key: &str, label: &str, value: f64, min: f64, max: f64) -> Field {
        Field { key: key.into(), label: label.into(), kind: FieldKind::Number { min, max, decimals: 0, slider: false, unit: "" }, value: json!(value), choice_values: None, error: false, scale: None, popup: false, icons: None, pref_codes: None, vertical: false }
    }
    /// Zahlenfeld mit Schieber.
    pub fn slider(key: &str, label: &str, value: f64, min: f64, max: f64) -> Field {
        Field { key: key.into(), label: label.into(), kind: FieldKind::Number { min, max, decimals: 0, slider: true, unit: "" }, value: json!(value), choice_values: None, error: false, scale: None, popup: false, icons: None, pref_codes: None, vertical: false }
    }
    /// Nachkommastellen setzen.
    pub fn decimals(mut self, d: usize) -> Field {
        if let FieldKind::Number { decimals, .. } = &mut self.kind {
            *decimals = d;
        }
        self
    }
    /// Kennlinie des Schiebers wie im Original ([`crate::inspector::scale`]).
    pub fn scale(mut self, s: crate::inspector::scale::Scale) -> Field {
        self.scale = Some(s);
        self
    }
    /// Optionsfelder als Symbolknöpfe zeichnen (je Eintrag ein Bildchen, Text als Tooltip).
    pub fn icons(mut self, p: &[Pict]) -> Field {
        self.icons = Some(p.to_vec());
        self
    }
    /// Codes der Einträge in der Voreinstellung.
    pub fn pref_codes(mut self, c: &[i64]) -> Field {
        self.pref_codes = Some(c.to_vec());
        self
    }
    /// Optionsfelder untereinander (ohne Beschriftungsspalte).
    pub fn vertical(mut self) -> Field {
        self.vertical = true;
        self
    }
    /// Popup-Schieber ▾ neben dem Feld statt fester Leiste.
    pub fn popup(mut self) -> Field {
        if let FieldKind::Number { slider, .. } = &mut self.kind {
            *slider = false;
        }
        self.popup = true;
        self
    }
    /// Kennlinie (gesetzt oder linear nach Nachkommastellen).
    pub fn effective_scale(&self) -> crate::inspector::scale::Scale {
        match (&self.scale, &self.kind) {
            (Some(s), _) => *s,
            (None, FieldKind::Number { decimals, .. }) => crate::inspector::scale::Scale::linear(*decimals),
            _ => crate::inspector::scale::Scale::default(),
        }
    }
    /// Einheit hinter dem Feld (Symbol wie „%“ oder Textschlüssel `dlg-unit-…`).
    pub fn unit(mut self, u: &'static str) -> Field {
        if let FieldKind::Number { unit, .. } = &mut self.kind {
            *unit = u;
        }
        self
    }
    /// Kontrollkästchen.
    pub fn check(key: &str, label: &str, value: bool) -> Field {
        Field { key: key.into(), label: label.into(), kind: FieldKind::Check, value: json!(value), choice_values: None, error: false, scale: None, popup: false, icons: None, pref_codes: None, vertical: false }
    }
    /// Auswahlliste mit Index als Wert.
    pub fn choice(key: &str, label: &str, items: &[&str], index: usize) -> Field {
        Field { key: key.into(), label: label.into(), kind: FieldKind::Choice(items.iter().map(|s| s.to_string()).collect()), value: json!(index), choice_values: None, error: false, scale: None, popup: false, icons: None, pref_codes: None, vertical: false }
    }
    /// Auswahlliste mit eigenen Werten.
    pub fn choice_of(key: &str, label: &str, items: &[&str], values: Vec<Value>, index: usize) -> Field {
        Field { key: key.into(), label: label.into(), kind: FieldKind::Choice(items.iter().map(|s| s.to_string()).collect()), value: json!(index), choice_values: Some(values), error: false, scale: None, popup: false, icons: None, pref_codes: None, vertical: false }
    }
    /// Optionsfelder mit eigenen Werten.
    pub fn radio_of(key: &str, label: &str, items: &[&str], values: Vec<Value>, index: usize) -> Field {
        Field { key: key.into(), label: label.into(), kind: FieldKind::Radio(items.iter().map(|s| s.to_string()).collect()), value: json!(index), choice_values: Some(values), error: false, scale: None, popup: false, icons: None, pref_codes: None, vertical: false }
    }
    /// Textfeld.
    pub fn text(key: &str, label: &str, value: &str) -> Field {
        Field { key: key.into(), label: label.into(), kind: FieldKind::Text { multiline: false }, value: json!(value), choice_values: None, error: false, scale: None, popup: false, icons: None, pref_codes: None, vertical: false }
    }
    /// Mehrzeiliges Textfeld.
    pub fn multiline(key: &str, label: &str, value: &str) -> Field {
        Field { key: key.into(), label: label.into(), kind: FieldKind::Text { multiline: true }, value: json!(value), choice_values: None, error: false, scale: None, popup: false, icons: None, pref_codes: None, vertical: false }
    }
    /// Farbfeld.
    pub fn color(key: &str, label: &str, hex: &str) -> Field {
        Field { key: key.into(), label: label.into(), kind: FieldKind::Color, value: json!(hex), choice_values: None, error: false, scale: None, popup: false, icons: None, pref_codes: None, vertical: false }
    }
    /// Überschrift.
    pub fn caption(label: &str) -> Field {
        Field { key: "_caption".into(), label: label.into(), kind: FieldKind::Caption, value: Value::Null, choice_values: None, error: false, scale: None, popup: false, icons: None, pref_codes: None, vertical: false }
    }
    /// Hinweis.
    pub fn note(label: &str) -> Field {
        Field { key: "_note".into(), label: label.into(), kind: FieldKind::Note, value: Value::Null, choice_values: None, error: false, scale: None, popup: false, icons: None, pref_codes: None, vertical: false }
    }

    /// Wert für den Befehl.
    pub fn param(&self) -> Value {
        match (&self.kind, &self.choice_values) {
            (FieldKind::Choice(_) | FieldKind::Radio(_), Some(vals)) => self.value.as_u64().and_then(|i| vals.get(i as usize).cloned()).unwrap_or(Value::Null),
            // ganzzahlige Felder als Ganzzahl (Befehle mit `Integer`-Parametern lehnen 5.0 ab)
            (FieldKind::Number { decimals: 0, .. }, _) => match self.value.as_f64() {
                Some(v) if v.fract() == 0.0 && v.abs() < 1e15 => json!(v as i64),
                _ => self.value.clone(),
            },
            _ => self.value.clone(),
        }
    }

    /// Setzt einen Zahlenwert mit Bereichsprüfung: liefert `false` bei ungültiger Eingabe, klemmt sonst.
    pub fn set_number_text(&mut self, text: &str) -> bool {
        let FieldKind::Number { min, max, decimals, .. } = self.kind else { return false };
        match fields::parse_num(text) {
            Some(v) => {
                let v = fields::clamp_range(v, (min, max));
                let p = 10f64.powi(decimals as i32);
                self.value = json!((v * p).round() / p);
                self.error = false;
                true
            }
            None => {
                self.error = true;
                false
            }
        }
    }
}

/// Angewandte Vorschau: das Dokument (Zeiger) unmittelbar danach und der Zustand davor.
/// Zurücknehmen stellt den Zustand exakt her, damit nach Abbrechen weder ein Rückgängig- noch ein
/// Wiederholen-Schritt übrig bleibt, ein vorher vorhandener Wiederholen-Stapel erhalten bleibt und
/// Strg+Y nicht die Vorschau als „letzten Befehl“ wiederholt (N01, §4b.2).
#[derive(Clone, Debug)]
pub struct AppliedPreview {
    /// Dokument nach der Vorschau.
    pub doc: std::sync::Arc<fp_model::Document>,
    /// Verlauf (Rückgängig- und Wiederholen-Stapel), „gespeichert“ und letzter Befehl davor.
    pub history: fp_engine::History,
    pub clean: bool,
    pub last_command: Option<fp_engine::RecordedCommand>,
    pub last_command_label: Option<String>,
}

/// Ein Formular-Dialog.
#[derive(Clone, Debug)]
pub struct FormDialog {
    pub id: String,
    pub title: String,
    pub fields: Vec<Field>,
    /// Befehl beim OK (bzw. in der Vorschau).
    pub command: String,
    /// Feste Zusatzparameter.
    pub base: Value,
    /// Live-Vorschau möglich (Kontrollkästchen „Vorschau“).
    pub preview: bool,
    pub preview_on: bool,
    /// Verlaufsposition nach der angewandten Vorschau.
    pub applied: Option<usize>,
    pub applied_params: Option<Value>,
    /// Dokument (Zeiger) unmittelbar nach der Vorschau: nur dann darf die Vorschau per Rückgängig
    /// zurückgenommen werden (sonst würde ein fremder Schritt zurückgenommen); dazu der Zustand
    /// vor der Vorschau ([`AppliedPreview`]).
    pub applied_doc: Option<AppliedPreview>,
    /// Die Vorschau ließ sich nicht mehr zurücknehmen (fremde Änderung dazwischen): Parameter der
    /// stehengebliebenen Vorschau.
    pub stale: Option<Value>,
    /// Breite des Inhalts.
    pub width: f32,
    /// Umformung der Feldwerte in die Befehlsparameter (z. B. verschachtelte Effektparameter).
    pub transform: Option<fn(&FormDialog) -> Value>,
    /// Feld ↔ Voreinstellung (Vorgabe beim Öffnen, Zurückschreiben bei OK, wie im Original).
    /// Der Feldschlüssel [`PREVIEW_FIELD`] steht für das Kästchen „Vorschau“.
    pub prefs: Vec<(String, String)>,
    /// Zusätzliches Zurückschreiben bei OK, wenn es vom Zustand abhängt (z. B. die benutzerdefinierte
    /// Leinwandfarbe nur, wenn „Benutzerdefiniert“ gewählt ist).
    pub store_hook: Option<fn(&FormDialog, &mut fp_engine::Preferences)>,
    /// Startfokus auf OK statt auf das erste Feld (wie im Original gemessen, PM-5).
    pub focus_ok: bool,
    /// Parameter beim Öffnen: OK mit denselben Werten führt keinen Befehl aus (Original: Raster
    /// bearbeiten ohne Änderung, PM-5 D18), siehe [`FormDialog::with_skip_unchanged`].
    pub initial: Option<Value>,
    /// Eigene Anordnung nach der Vorlage des Originals statt der Standardzeilen.
    pub custom: Option<CustomLayout>,
}

/// Eigene Anordnung eines Formulars (Felder an den Stellen der Vorlage, z. B. Raster bearbeiten,
/// Textgröße, Selektives JPEG). Werte, Prüfung, Voreinstellungen und OK bleiben die des Formulars.
#[derive(Clone, Copy, Debug)]
pub struct CustomLayout {
    /// Größe des Client-Bereichs für die Texte der aktuellen Sprache.
    pub size: fn(&egui::Context, &FormDialog) -> egui::Vec2,
    /// Zeichnet die Felder in den Client-Bereich; liefert „geändert“ und die Rechtecke von OK und
    /// Abbrechen.
    pub draw: fn(&mut Ui, &AppState, &mut FormDialog, Rect) -> (bool, [Rect; 2]),
}

/// Pseudo-Feldschlüssel des Kästchens „Vorschau“ in [`FormDialog::with_prefs`]
/// (z. B. `MotionBlurShowPreview`).
pub const PREVIEW_FIELD: &str = "_preview";

impl FormDialog {
    /// Neuer Dialog.
    pub fn new(id: &str, title: &str, command: &str, fields: Vec<Field>) -> FormDialog {
        FormDialog { id: id.into(), title: title.into(), fields, command: command.into(), base: json!({}), preview: false, preview_on: false, applied: None, applied_params: None, applied_doc: None, stale: None, width: 340.0, transform: None, prefs: Vec::new(), store_hook: None, focus_ok: false, initial: None, custom: None }
    }
    /// Startfokus auf OK (Original: Exportassistent, HTML-Einrichtung, Selektives JPEG,
    /// Rechtschreibung einrichten).
    pub fn with_focus_ok(mut self) -> FormDialog {
        self.focus_ok = true;
        self
    }
    /// OK ohne Änderung führt keinen Befehl aus (Werte beim Aufruf dieser Funktion gelten als
    /// Ausgangsstand; nach den Feldern und Voreinstellungen aufrufen).
    pub fn with_skip_unchanged(mut self) -> FormDialog {
        self.initial = Some(self.params());
        self
    }
    /// Eigene Anordnung (siehe [`CustomLayout`]).
    pub fn with_layout(mut self, l: CustomLayout) -> FormDialog {
        self.custom = Some(l);
        self
    }
    /// Mit Umformung der Parameter.
    pub fn with_transform(mut self, f: fn(&FormDialog) -> Value) -> FormDialog {
        self.transform = Some(f);
        self
    }
    /// Felder mit Voreinstellungen verknüpfen: `(Feldschlüssel, Voreinstellung)`.
    pub fn with_prefs(mut self, pairs: &[(&str, &str)]) -> FormDialog {
        self.prefs.extend(pairs.iter().map(|(f, k)| (f.to_string(), k.to_string())));
        self
    }
    /// Zusätzliches Zurückschreiben bei OK (siehe [`FormDialog::store_hook`]).
    pub fn with_store_hook(mut self, f: fn(&FormDialog, &mut fp_engine::Preferences)) -> FormDialog {
        self.store_hook = Some(f);
        self
    }
    /// Vorgaben aus den Voreinstellungen lesen (fehlt der Schlüssel, bleibt die Feldvorgabe).
    pub fn load_prefs(&mut self, p: &fp_engine::Preferences) {
        for (fk, pk) in self.prefs.clone() {
            let Some(pv) = p.get(&pk) else { continue };
            if fk == PREVIEW_FIELD {
                if let Some(b) = pv.as_bool()
                    && self.preview
                {
                    self.preview_on = b;
                }
                continue;
            }
            let Some(f) = self.fields.iter_mut().find(|f| f.key == fk) else { continue };
            match &f.kind {
                FieldKind::Number { min, max, .. } => {
                    if let Some(v) = pv.as_real() {
                        f.value = json!(v.clamp(*min, *max));
                    }
                }
                FieldKind::Check => {
                    if let Some(b) = pv.as_bool() {
                        f.value = json!(b);
                    }
                }
                FieldKind::Choice(items) | FieldKind::Radio(items) => {
                    // Wert der Voreinstellung = Eintrag in `choice_values` bzw. Index
                    let n = items.len();
                    let idx = match (&f.pref_codes, &f.choice_values) {
                        (Some(codes), _) => pv.as_int().and_then(|c| codes.iter().position(|x| *x == c)),
                        (None, vals) => match vals {
                        Some(vals) => vals.iter().position(|v| v.as_i64() == pv.as_int() || v.as_str().map(|s| s.to_string()) == pv.as_str()),
                        None => pv.as_int().map(|i| i as usize),
                        },
                    };
                    if let Some(i) = idx.filter(|i| *i < n) {
                        f.value = json!(i);
                    }
                }
                FieldKind::Color | FieldKind::Text { .. } => {
                    if let Some(s) = pv.as_str() {
                        f.value = json!(s);
                    }
                }
                _ => {}
            }
        }
    }
    /// Feldwerte in die Voreinstellungen zurückschreiben (bei OK).
    pub fn store_prefs(&self, p: &mut fp_engine::Preferences) {
        use fp_engine::prefs::PrefValue;
        if let Some(h) = self.store_hook {
            h(self, p);
        }
        for (fk, pk) in &self.prefs {
            if fk == PREVIEW_FIELD {
                if self.preview {
                    p.set(pk, PrefValue::Bool(self.preview_on));
                }
                continue;
            }
            let Some(f) = self.fields.iter().find(|f| &f.key == fk) else { continue };
            let v = match &f.kind {
                FieldKind::Number { decimals: 0, .. } => f.value.as_f64().map(|v| PrefValue::Int(v.round() as i64)),
                // Ganzzahl-Voreinstellungen (z. B. `InsetMiterLimit`) bleiben ganzzahlig
                FieldKind::Number { .. } if fp_engine::prefs::definition(pk).is_some_and(|d| d.kind == fp_engine::prefs::PrefType::Integer) => f.value.as_f64().map(|v| PrefValue::Int(v.round() as i64)),
                FieldKind::Number { .. } => f.value.as_f64().map(PrefValue::Real),
                FieldKind::Check => f.value.as_bool().map(PrefValue::Bool),
                FieldKind::Choice(_) | FieldKind::Radio(_) if f.pref_codes.is_some() => {
                    let i = f.value.as_u64().unwrap_or(0) as usize;
                    f.pref_codes.as_ref().and_then(|c| c.get(i)).map(|c| PrefValue::Int(*c))
                }
                FieldKind::Choice(_) | FieldKind::Radio(_) => match f.param() {
                    Value::Number(n) => n.as_i64().map(PrefValue::Int),
                    Value::String(s) => Some(PrefValue::Str(s)),
                    _ => None,
                },
                FieldKind::Color | FieldKind::Text { .. } => f.value.as_str().map(|s| PrefValue::Str(s.to_string())),
                _ => None,
            };
            if let Some(v) = v {
                p.set(pk, v);
            }
        }
    }
    /// Feldwerte (ohne Umformung) als Objekt.
    pub fn raw_values(&self) -> Map<String, Value> {
        let mut m = Map::new();
        for f in &self.fields {
            if f.key == "_caption" || f.key == "_note" {
                continue;
            }
            m.insert(f.key.clone(), f.param());
        }
        m
    }
    /// Wert eines Felds.
    pub fn get(&self, key: &str) -> Value {
        self.fields.iter().find(|f| f.key == key).map(|f| f.param()).unwrap_or(Value::Null)
    }
    /// Mit Live-Vorschau.
    pub fn with_preview(mut self) -> FormDialog {
        self.preview = true;
        self.preview_on = true;
        self
    }
    /// Zusatzparameter.
    pub fn with_base(mut self, base: Value) -> FormDialog {
        self.base = base;
        self
    }
    /// Breite.
    pub fn with_width(mut self, w: f32) -> FormDialog {
        self.width = w;
        self
    }

    /// Parameter des Befehls.
    pub fn params(&self) -> Value {
        if let Some(t) = self.transform {
            return t(self);
        }
        let mut m = match &self.base {
            Value::Object(o) => o.clone(),
            _ => Map::new(),
        };
        for f in &self.fields {
            if f.key.starts_with('_') {
                continue;
            }
            m.insert(f.key.clone(), f.param());
        }
        Value::Object(m)
    }

    /// Befehl und Parameter für OK bzw. Vorschau. Liefert die Umformung eine Befehlsliste
    /// (`{"commands": […], "label": …}`), läuft sie als ein Verlaufsschritt über `history.replay`.
    pub fn invocation(&self) -> (String, Value) {
        let p = self.params();
        if self.command != "history.replay" && p.get("commands").is_some_and(|c| c.is_array()) {
            ("history.replay".to_string(), p)
        } else {
            (self.command.clone(), p)
        }
    }

    /// Sind alle Eingaben gültig?
    pub fn valid(&self) -> bool {
        self.fields.iter().all(|f| !f.error)
    }

    /// Feld nach Schlüssel.
    pub fn field_mut(&mut self, key: &str) -> Option<&mut Field> {
        self.fields.iter_mut().find(|f| f.key == key)
    }

    fn row_height(f: &Field) -> f32 {
        match &f.kind {
            FieldKind::Text { multiline: true } => 120.0,
            FieldKind::Number { slider: true, .. } => 46.0,
            FieldKind::Radio(items) if f.vertical => RADIO_ROW_H * items.len() as f32,
            FieldKind::Caption => 24.0,
            FieldKind::Note => 34.0,
            _ => 26.0,
        }
    }

    /// Höhe des Inhalts.
    pub fn content_height(&self) -> f32 {
        self.fields.iter().map(Self::row_height).sum::<f32>() + 60.0 + if self.preview { 24.0 } else { 0.0 }
    }

    /// Maße für die Texte der aktuellen Sprache: Beschriftungsspalte (mindestens 130 px wie
    /// bisher), Dialogbreite (mindestens [`FormDialog::width`]) und Zeilenhöhen (Hinweise brechen um).
    pub fn layout(&self, ctx: &egui::Context) -> FormLayout {
        let theme = Theme::current(ctx);
        let tw = |s: &str| super::text_w(ctx, s);
        let mut lw: f32 = 0.0;
        // Breite rechts der Beschriftungsspalte bzw. über die ganze Zeile
        let mut ctrl: f32 = 0.0;
        let mut full: f32 = 0.0;
        for f in &self.fields {
            match &f.kind {
                FieldKind::Number { unit, .. } => {
                    lw = lw.max(tw(&f.label));
                    ctrl = ctrl.max(60.0 + if f.popup { 16.0 } else { 0.0 } + if unit.is_empty() { 0.0 } else { 6.0 + tw(&unit_text(unit)) });
                }
                FieldKind::Choice(items) => {
                    lw = lw.max(tw(&f.label));
                    let refs: Vec<&str> = items.iter().map(|s| s.as_str()).collect();
                    ctrl = ctrl.max(super::combo_fit_w(ctx, &refs, 80.0));
                }
                FieldKind::Radio(items) if f.vertical => full = full.max(items.iter().map(|it| tw(it) + 26.0).fold(0.0, f32::max)),
                FieldKind::Radio(items) => {
                    let w: f32 = items.iter().map(|it| tw(it) + 26.0 + 8.0).sum();
                    if f.label.is_empty() {
                        full = full.max(w);
                    } else {
                        lw = lw.max(tw(&f.label));
                        ctrl = ctrl.max(w);
                    }
                }
                // Farbfeld ohne Beschriftung: eingerückt unter der Zeile davor (Leinwandfarbe)
                FieldKind::Color if f.label.is_empty() => full = full.max(COLOR_INDENT + 26.0),
                FieldKind::Text { multiline: false } | FieldKind::Color => lw = lw.max(tw(&f.label)),
                FieldKind::Text { multiline: true } => full = full.max(tw(&f.label)),
                FieldKind::Check => full = full.max(18.0 + tw(&f.label)),
                FieldKind::Caption => full = full.max(ctx.fonts_mut(|fo| fo.layout_no_wrap(f.label.clone(), theme.bold(theme.fonts.ui), egui::Color32::WHITE).size().x) + 24.0),
                FieldKind::Note => {}
            }
        }
        let label_w = 130f32.max((lw + 8.0).ceil());
        let (ok, cancel) = (t("btn-ok"), t("btn-cancel"));
        let buttons = super::button_row_w(ctx, &[&ok, &cancel], 76.0, 8.0) + 16.0 + if self.preview { 18.0 + tw(&t("dlg-preview")) + 12.0 } else { 0.0 };
        let width = self.width.max((28.0 + label_w + ctrl).ceil()).max((28.0 + full).ceil()).max((buttons + 14.0).ceil());
        let note_w = width - 28.0;
        let rows: Vec<f32> = self
            .fields
            .iter()
            .map(|f| match f.kind {
                FieldKind::Note => {
                    let h = ctx.fonts_mut(|fo| fo.layout(f.label.clone(), theme.small_font(), egui::Color32::WHITE, note_w).size().y);
                    Self::row_height(f).max((h + 6.0).ceil())
                }
                _ => Self::row_height(f),
            })
            .collect();
        let height = rows.iter().sum::<f32>() + 60.0 + if self.preview { 24.0 } else { 0.0 };
        FormLayout { label_w, width, height, rows }
    }
}

/// Zeilenhöhe untereinander stehender Optionsfelder (Original 14 DLU).
const RADIO_ROW_H: f32 = 22.0;
/// Einrückung eines Farbfelds ohne Beschriftung (Original 12 DLU).
const COLOR_INDENT: f32 = 18.0;

/// Maße eines Formulars (siehe [`FormDialog::layout`]).
#[derive(Clone, Debug, PartialEq)]
pub struct FormLayout {
    pub label_w: f32,
    pub width: f32,
    pub height: f32,
    pub rows: Vec<f32>,
}

/// Vorschau zurücknehmen (falls angewandt). Nur wenn der oberste Verlaufsschritt noch die Vorschau
/// ist (gleiche Position, gleiches Dokument); sonst bleibt sie stehen und der Dialog wechselt in den
/// „stehengebliebenen“ Zustand (keine weiteren Vorschauen, die sich addieren könnten).
pub fn revert_preview(st: &mut AppState, d: &mut FormDialog) {
    if let Some(pos) = d.applied {
        let ours = st.session.active().is_some_and(|doc| doc.history.undo_count() == pos && d.applied_doc.as_ref().is_some_and(|a| std::sync::Arc::ptr_eq(&a.doc, &doc.doc)));
        if ours {
            let before = d.applied_doc.take();
            let _ = st.session.undo();
            // Abbrechen bzw. neue Vorschau: weder Rückgängig- noch Wiederholen-Schritt der Vorschau
            if let Some(b) = before {
                st.session.last_command = b.last_command;
                st.session.last_command_label = b.last_command_label;
                if let Some(doc) = st.session.active_mut() {
                    doc.history = b.history;
                    if b.clean {
                        doc.saved_generation = doc.generation;
                    }
                }
            }
        } else {
            d.stale = d.applied_params.clone();
            adopt_stale_effect(st, d);
        }
    }
    d.applied = None;
    d.applied_params = None;
    d.applied_doc = None;
}

/// Neuer Live-Effekt, dessen Vorschau stehen blieb: der Dialog bearbeitet ab jetzt genau diesen
/// Effekt (`pi.effect.set` mit Index), statt einen weiteren hinzuzufügen.
fn adopt_stale_effect(st: &AppState, d: &mut FormDialog) {
    if d.command != "element.addEffect" {
        return;
    }
    let ids: Vec<fp_model::ElementId> = d.base.get("ids").and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|x| x.as_u64()).map(fp_model::ElementId).collect()).unwrap_or_default();
    let Some(doc) = st.session.active() else { return };
    let Some(id) = ids.first().copied().or_else(|| doc.selection.elements.first().copied()) else { return };
    let n = fp_engine::docops::element_on_page(&doc.doc, doc.page, id).and_then(|e| e.common.effects.clone()).map(|l| l.effects.len()).unwrap_or(0);
    if n == 0 {
        return;
    }
    d.base["index"] = serde_json::json!(n - 1);
    if d.base.get("ids").is_none() {
        d.base["ids"] = serde_json::json!([id.0]);
    }
    d.command = "pi.effect.set".into();
    d.base["_adopted"] = serde_json::json!(true);
    d.stale = None;
}

/// Vorschau mit den aktuellen Parametern anwenden.
pub fn apply_preview(st: &mut AppState, d: &mut FormDialog) {
    let p = d.params();
    if d.applied_params.as_ref() == Some(&p) {
        return;
    }
    revert_preview(st, d);
    if d.stale.is_some() {
        // stehengebliebene Vorschau: nicht noch einmal anwenden (würde sich addieren)
        d.preview_on = false;
        st.push(crate::app_state::Action::Status(t("dlg-preview-interrupted")));
        return;
    }
    let (cmd, p) = d.invocation();
    let before = st.session.active().map(|x| (x.history.clone(), !x.is_dirty()));
    let last = (st.session.last_command.clone(), st.session.last_command_label.clone());
    match crate::ui_commands::execute(&mut st.session, &cmd, p.clone()) {
        Ok(_) => {
            d.applied = st.session.active().map(|x| x.history.undo_count());
            d.applied_doc = st.session.active().zip(before).map(|(x, (history, clean))| AppliedPreview { doc: std::sync::Arc::clone(&x.doc), history, clean, last_command: last.0, last_command_label: last.1 });
            d.applied_params = Some(d.params());
        }
        Err(e) => st.push(crate::app_state::Action::Status(e.to_string())),
    }
}
/// Zeichnet die Felder; liefert `true`, wenn sich ein Wert geändert hat.
pub fn draw_fields(ui: &mut Ui, st: &AppState, d: &mut FormDialog, area: Rect) -> bool {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let mut changed = false;
    let lay = d.layout(ui.ctx());
    let label_w = lay.label_w;
    let mut y = area.top() + 12.0;
    let x0 = area.left() + 14.0;
    let ctrl_x = x0 + label_w;
    let ctrl_w = (area.right() - 14.0 - ctrl_x).max(80.0);
    let dlg_id = d.id.clone();
    for (i, f) in d.fields.iter_mut().enumerate() {
        let h = lay.rows.get(i).copied().unwrap_or_else(|| FormDialog::row_height(f));
        let lr = Rect::from_min_size(pos2(x0, y), vec2(label_w - 6.0, 20.0));
        let label = |ui: &Ui, text: &str| super::fit_label(ui, lr, text, true);
        let id = (dlg_id.as_str(), i);
        match &f.kind {
            FieldKind::Caption => {
                ui.painter().text(pos2(x0, y + 12.0), Align2::LEFT_CENTER, &f.label, theme.bold(theme.fonts.ui), c.text);
                ui.painter().hline((x0 + crate::widgets::text_width(ui, &f.label, &theme.bold(theme.fonts.ui)) + 8.0)..=(area.right() - 14.0), y + 12.5, Stroke::new(1.0, c.line_mid));
            }
            FieldKind::Note => {
                super::put_left(ui, Rect::from_min_size(pos2(x0, y), vec2(area.width() - 28.0, h - 4.0)), egui::Label::new(egui::RichText::new(&f.label).color(c.text_weak).font(theme.small_font())).wrap());
            }
            FieldKind::Number { min, max, decimals, slider, unit } => {
                label(ui, &f.label);
                let v = f.value.as_f64().unwrap_or(*min);
                let (min, max, decimals, slider, unit) = (*min, *max, *decimals, *slider, *unit);
                let er = Rect::from_min_size(pos2(ctrl_x, y), vec2(60.0, 20.0));
                changed |= number_input(ui, &dlg_id, i, f, er, true);
                let unit_x = if f.popup { er.right() + 15.0 } else { er.right() };
                if !unit.is_empty() {
                    ui.painter().text(pos2(unit_x + 6.0, er.center().y), Align2::LEFT_CENTER, unit_text(unit), theme.ui_font(), c.text);
                }
                if slider {
                    let sr = Rect::from_min_size(pos2(x0, y + 24.0), vec2(area.width() - 28.0, 18.0));
                    let mut val = v;
                    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(sr));
                    child.spacing_mut().slider_width = sr.width() - 8.0;
                    // Kennlinie des Originals (stückweise linear, z. B. Weichzeichnungsradius)
                    let r = crate::inspector::scale::slider(&mut child, &mut val, (min, max), f.effective_scale());
                    if r.changed() {
                        let p = 10f64.powi(decimals as i32);
                        f.value = json!((val * p).round() / p);
                        f.error = false;
                        changed = true;
                    }
                }
            }
            FieldKind::Check => {
                let on = f.value.as_bool().unwrap_or(false);
                if let Some(b) = fields::checkbox(ui, id, Rect::from_min_size(pos2(x0, y + 2.0), vec2(area.width() - 28.0, 16.0)), &f.label, on, true) {
                    f.value = json!(b);
                    changed = true;
                }
            }
            FieldKind::Choice(items) => {
                label(ui, &f.label);
                let refs: Vec<&str> = items.iter().map(|s| s.as_str()).collect();
                let sel = f.value.as_u64().map(|v| v as usize);
                let cw = super::combo_fit_w(ui.ctx(), &refs, ctrl_w.min(200.0)).min(ctrl_w);
                if let Some(n) = fields::combo(ui, id, Rect::from_min_size(pos2(ctrl_x, y), vec2(cw, 20.0)), &refs, sel, true) {
                    f.value = json!(n);
                    changed = true;
                }
            }
            FieldKind::Radio(items) => {
                if !f.label.is_empty() {
                    label(ui, &f.label);
                }
                let sel = f.value.as_u64().unwrap_or(0) as usize;
                let mut x = if f.label.is_empty() { x0 } else { ctrl_x };
                // Symbolknöpfe (Ecken/Endkappen) wie im Original; Text als Tooltip
                if let Some(icons) = f.icons.clone() {
                    for (k, (it, pict)) in items.iter().zip(icons).enumerate() {
                        let br = Rect::from_min_size(pos2(x, y), vec2(24.0, 22.0));
                        let resp = ui.interact(br, egui::Id::new((dlg_id.as_str(), i, k, "icon")), Sense::click()).on_hover_text(it.as_str());
                        resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::RadioButton, true, sel == k, it.as_str()));
                        let on = sel == k;
                        ui.painter().rect_filled(br, 2.0, if on { c.button_pressed_bg } else if resp.hovered() { c.button_hover_bg } else { c.field_bg });
                        ui.painter().rect_stroke(br, 2.0, Stroke::new(1.0, if on { c.focus } else { c.field_border }), StrokeKind::Inside);
                        paint_pict(ui.painter(), br, pict, c.icon_fg);
                        if resp.clicked() {
                            f.value = json!(k);
                            changed = true;
                        }
                        x += 28.0;
                    }
                    y += h;
                    continue;
                }
                // eine Optionsgruppe = ein Tab-Stopp, Pfeiltasten innerhalb (wie im Original);
                // nebeneinander bzw. untereinander (`vertical`, Zeilenabstand wie im Original)
                let mut rects = Vec::with_capacity(items.len());
                let mut ry = y + 2.0;
                for it in items.iter() {
                    let w = crate::widgets::text_width(ui, it, &theme.ui_font()) + 26.0;
                    rects.push(Rect::from_min_size(pos2(x, ry), vec2(w, 16.0)));
                    if f.vertical {
                        ry += RADIO_ROW_H;
                    } else {
                        x += w + 8.0;
                    }
                }
                let labels: Vec<&str> = items.iter().map(|s| s.as_str()).collect();
                if let Some(k) = super::radio_group(ui, (dlg_id.as_str(), i, "radio"), &rects, &labels, sel, true) {
                    f.value = json!(k);
                    changed = true;
                }
            }
            FieldKind::Text { multiline } => {
                if *multiline {
                    fields::label(ui, Rect::from_min_size(pos2(x0, y), vec2(area.width(), 16.0)), &f.label, true);
                    let tr = Rect::from_min_size(pos2(x0, y + 18.0), vec2(area.width() - 28.0, h - 22.0));
                    ui.painter().rect_filled(tr, 0.0, c.field_bg);
                    ui.painter().rect_stroke(tr, 0.0, Stroke::new(1.0, c.field_border), StrokeKind::Inside);
                    let mut s = f.value.as_str().unwrap_or("").to_string();
                    let r = ui.put(tr.shrink(2.0), egui::TextEdit::multiline(&mut s).frame(egui::Frame::NONE).font(theme.ui_font()).desired_width(tr.width() - 4.0));
                    if r.changed() {
                        f.value = json!(s);
                        changed = true;
                    }
                } else {
                    label(ui, &f.label);
                    if let Some(t) = fields::text(ui, id, Rect::from_min_size(pos2(ctrl_x, y), vec2(ctrl_w, 20.0)), f.value.as_str().unwrap_or(""), true) {
                        f.value = json!(t);
                        changed = true;
                    }
                }
            }
            FieldKind::Color => {
                label(ui, &f.label);
                // ohne Beschriftung eingerückt unter dem Feld davor (Leinwandfarbe: unter „Benutzerdefiniert:“)
                let cx = if f.label.is_empty() { x0 + COLOR_INDENT } else { ctrl_x };
                let cur = f.value.as_str().and_then(fp_color::Rgba8::parse_hex).map(|c| c.to_argb());
                if let Some(crate::inspector::popups::ColorChoice::Color(col)) = crate::inspector::popups::color_button(ui, id, Rect::from_min_size(pos2(cx, y), vec2(26.0, 20.0)), st, cur, true, false, false) {
                    let a = cur.map(|c| c.a()).unwrap_or(255) as u32;
                    f.value = json!(crate::ui_commands::hex(fp_color::Argb((col.0 & 0x00ff_ffff) | a << 24)));
                    changed = true;
                }
            }
        }
        y += h;
    }
    changed
}

/// Zahlenfeld `i` eines Formulars im Rechteck `er`, mit Popup-Schieber ▾ direkt rechts daneben
/// (14 px breit), wenn das Feld einen hat; ungültige Eingaben rot umrandet. Liefert `true`, wenn
/// sich der Wert geändert hat. Auch für eigene Anordnungen ([`CustomLayout`]).
pub fn number_input(ui: &mut Ui, dlg_id: &str, i: usize, f: &mut Field, er: Rect, enabled: bool) -> bool {
    let FieldKind::Number { min, max, decimals, .. } = f.kind else { return false };
    let theme = Theme::current(ui.ctx());
    let mut changed = false;
    let v = f.value.as_f64().unwrap_or(min);
    // beim Ziehen im Popup-Schieber zeigt das Feld den Wert schon mit (wie im Inspektor)
    let popup_id = egui::Id::new((dlg_id, i, "popup"));
    let live = if f.popup { ui.data(|d| d.get_temp::<f64>(popup_id.with("val"))) } else { None };
    let txt = fields::fmt_num(live.unwrap_or(v), decimals);
    if let Some(t) = fields::text(ui, (dlg_id, i), er, &txt, enabled) {
        let mut ff = f.clone();
        ff.set_number_text(&t);
        f.value = ff.value;
        f.error = ff.error;
        changed = true;
    }
    if f.error {
        ui.painter().rect_stroke(er, 0.0, Stroke::new(1.5, theme.colors.icon_red), StrokeKind::Inside);
    }
    // Popup-Schieber ▾ direkt rechts am Feld, senkrecht über dem Feld wie im Original und im
    // Inspektor (gleiche Komponente)
    if f.popup {
        let ar = Rect::from_min_size(pos2(er.right() + 1.0, er.top()), vec2(14.0, er.height()));
        if let Some(nv) = fields::slider_arrow_vertical(ui, popup_id, ar, v, (min, max), f.effective_scale(), enabled) {
            let p = 10f64.powi(decimals as i32);
            f.value = json!((nv * p).round() / p);
            f.error = false;
            changed = true;
        }
    }
    changed
}

/// Zeigt einen Formular-Dialog; liefert `Some(true)` bei OK, `Some(false)` bei Abbrechen.
pub fn show_form(ctx: &egui::Context, st: &mut AppState, d: &mut FormDialog) -> Option<bool> {
    let size = match d.custom {
        Some(l) => (l.size)(ctx, d),
        None => {
            let lay = d.layout(ctx);
            vec2(lay.width, lay.height)
        }
    };
    let mut result = None;
    let mut changed = false;
    let id = format!("fp-form-{}", d.id);
    let title = d.title.clone();
    let r = dialog_frame(ctx, &id, &title, size, |ui, body| {
        if d.focus_ok {
            super::autofocus_default_button(ui);
        }
        let custom = d.custom.map(|l| (l.draw)(ui, st, d, body));
        changed = match custom {
            Some((ch, _)) => ch,
            None => draw_fields(ui, st, d, body),
        };
        let y = body.bottom() - 36.0;
        if d.preview {
            let pl = t("dlg-preview");
            let pw = (18.0 + super::text_w(ui.ctx(), &pl)).max(90.0);
            if let Some(b) = fields::checkbox(ui, (id.as_str(), "preview"), Rect::from_min_size(pos2(body.left() + 14.0, y + 3.0), vec2(pw, 16.0)), &pl, d.preview_on, true) {
                d.preview_on = b;
                changed = true;
            }
        }
        let valid = d.valid();
        let (ok_l, cancel_l) = (t("btn-ok"), t("btn-cancel"));
        let b = match custom {
            Some((_, b)) => b.to_vec(),
            None => super::button_row(ui.ctx(), body.right() - 16.0, y, &[&ok_l, &cancel_l], 76.0, 23.0, 8.0),
        };
        if push_button(ui, b[0], &ok_l, true, valid) {
            result = Some(true);
        }
        if push_button(ui, b[1], &cancel_l, false, true) {
            result = Some(false);
        }
        let _ = ui.allocate_rect(body, Sense::hover());
    });
    if r == DialogClose::Cancelled {
        result = Some(false);
    }
    let dragging = ctx.input(|i| i.pointer.any_down());
    if d.preview && result.is_none() {
        if d.preview_on && (changed || d.applied.is_none() || d.applied_params.as_ref() != Some(&d.params())) && !dragging && d.valid() {
            apply_preview(st, d);
        } else if !d.preview_on && d.applied.is_some() {
            revert_preview(st, d);
        }
    }
    match result {
        Some(true) => {
            d.store_prefs(&mut st.session.prefs);
            if d.preview && d.applied.is_some() && d.applied_params.as_ref() == Some(&d.params()) {
                // Vorschau ist bereits der endgültige Schritt
            } else {
                if d.preview {
                    revert_preview(st, d);
                }
                let (cmd, p) = d.invocation();
                if d.stale.as_ref() == Some(&d.params()) {
                    // die stehengebliebene Vorschau ist bereits das Ergebnis
                } else if d.initial.as_ref() == Some(&d.params()) {
                    // nichts geändert: wie im Original kein Befehl, kein Verlaufsschritt
                } else if cmd.is_empty() {
                    // reiner Hinweisdialog
                } else if p.as_object().is_some_and(|o| o.is_empty()) {
                    // ohne Parameter nicht über die Aktionen (die würden den Dialog erneut öffnen)
                    if let Err(e) = crate::ui_commands::execute(&mut st.session, &cmd, p) {
                        st.push(crate::app_state::Action::Status(e.to_string()));
                    }
                } else {
                    st.run(&cmd, p);
                }
            }
            Some(true)
        }
        Some(false) => {
            if d.preview {
                revert_preview(st, d);
            }
            // übernommener (stehengebliebener) neuer Effekt: beim Abbrechen wieder entfernen
            if d.base.get("_adopted").is_some_and(|v| v.as_bool() == Some(true))
                && let Some(i) = d.base.get("index").cloned()
            {
                let mut p = json!({"index": i});
                if let Some(ids) = d.base.get("ids") {
                    p["ids"] = ids.clone();
                }
                if let Err(e) = crate::ui_commands::execute(&mut st.session, "element.removeEffect", p) {
                    st.push(crate::app_state::Action::Status(e.to_string()));
                }
            } else if d.stale.is_some() {
                st.push(crate::app_state::Action::Status(t("dlg-preview-cancel-failed")));
            }
            Some(false)
        }
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_fields_validate_and_clamp() {
        let mut f = Field::number("amount", "Pixel:", 1.0, 1.0, 100.0);
        assert!(f.set_number_text("150"));
        assert_eq!(f.value, json!(100.0));
        assert!(f.set_number_text("0"));
        assert_eq!(f.value, json!(1.0));
        assert!(!f.set_number_text("abc"));
        assert!(f.error);
        let mut g = Field::number("radius", "Radius:", 4.1, 0.1, 250.0).decimals(1);
        assert!(g.set_number_text("3,25"));
        assert_eq!(g.value, json!(3.3));
        let d = FormDialog::new("t", "T", "filter.blur.gaussian", vec![g, Field::choice_of("mode", "Art", &["A", "B"], vec![json!("a"), json!("b")], 1), Field::caption("x")]);
        assert_eq!(d.params(), json!({"radius": 3.3, "mode": "b"}));
        assert!(d.valid());
    }
}
