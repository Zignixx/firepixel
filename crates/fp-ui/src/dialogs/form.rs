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
    /// Dezimalpunkt in jeder Sprache („4.1“ wie im Original, PM-6 D46/D48); Eingabe mit Komma geht weiter.
    pub dot: bool,
    /// Kästchen in der Knopfspalte rechts statt links bei den Feldern ([`FormDialog::with_side_buttons`],
    /// z. B. „Kolorieren“ unter Abbrechen).
    pub side: bool,
    /// Gesperrt (grau, kein Tab-Stopp, Klick ohne Wirkung), z. B. „In allgemeiner Bibliothek
    /// speichern“ in „Neues Symbol“ (PARITAET_B D14).
    pub disabled: bool,
    /// Zahlenfeld, das wie im Original erst **beim OK** auf den Bereich geklemmt wird: das Feld wird
    /// korrigiert und der Dialog bleibt offen (Vorlagen 19105–19108, re\11 §3.5.5). Bis dahin zeigt
    /// es den getippten Wert.
    pub clamp_ok: bool,
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

pub(crate) fn paint_pict(p: &egui::Painter, r: Rect, k: Pict, col: egui::Color32) {
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
        Field { key: key.into(), label: label.into(), kind: FieldKind::Number { min, max, decimals: 0, slider: false, unit: "" }, value: json!(value), choice_values: None, error: false, scale: None, popup: false, icons: None, pref_codes: None, vertical: false, dot: false, side: false, disabled: false, clamp_ok: false }
    }
    /// Zahlenfeld mit Schieber.
    pub fn slider(key: &str, label: &str, value: f64, min: f64, max: f64) -> Field {
        Field { key: key.into(), label: label.into(), kind: FieldKind::Number { min, max, decimals: 0, slider: true, unit: "" }, value: json!(value), choice_values: None, error: false, scale: None, popup: false, icons: None, pref_codes: None, vertical: false, dot: false, side: false, disabled: false, clamp_ok: false }
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
    /// Dezimalpunkt in jeder Sprache (siehe [`Field::dot`]).
    pub fn dot_decimal(mut self) -> Field {
        self.dot = true;
        self
    }
    /// Kästchen in der Knopfspalte (siehe [`Field::side`]).
    pub fn beside_buttons(mut self) -> Field {
        self.side = true;
        self
    }
    /// Gesperrt (siehe [`Field::disabled`]).
    pub fn disabled(mut self) -> Field {
        self.disabled = true;
        self
    }
    /// Erst beim OK klemmen (siehe [`Field::clamp_ok`]).
    pub fn clamp_on_ok(mut self) -> Field {
        self.clamp_ok = true;
        self
    }
    /// Liegt der Wert außerhalb des Bereichs, wird er geklemmt; liefert `true`, wenn sich etwas
    /// geändert hat (OK-Prüfung von [`Field::clamp_ok`]).
    pub fn clamp_now(&mut self) -> bool {
        let FieldKind::Number { min, max, .. } = self.kind else { return false };
        let Some(v) = self.value.as_f64() else { return false };
        let c = fields::clamp_range(v, (min, max));
        if c == v {
            return false;
        }
        self.value = json!(c);
        true
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
        Field { key: key.into(), label: label.into(), kind: FieldKind::Check, value: json!(value), choice_values: None, error: false, scale: None, popup: false, icons: None, pref_codes: None, vertical: false, dot: false, side: false, disabled: false, clamp_ok: false }
    }
    /// Auswahlliste mit Index als Wert.
    pub fn choice(key: &str, label: &str, items: &[&str], index: usize) -> Field {
        Field { key: key.into(), label: label.into(), kind: FieldKind::Choice(items.iter().map(|s| s.to_string()).collect()), value: json!(index), choice_values: None, error: false, scale: None, popup: false, icons: None, pref_codes: None, vertical: false, dot: false, side: false, disabled: false, clamp_ok: false }
    }
    /// Auswahlliste mit eigenen Werten.
    pub fn choice_of(key: &str, label: &str, items: &[&str], values: Vec<Value>, index: usize) -> Field {
        Field { key: key.into(), label: label.into(), kind: FieldKind::Choice(items.iter().map(|s| s.to_string()).collect()), value: json!(index), choice_values: Some(values), error: false, scale: None, popup: false, icons: None, pref_codes: None, vertical: false, dot: false, side: false, disabled: false, clamp_ok: false }
    }
    /// Optionsfelder mit eigenen Werten.
    pub fn radio_of(key: &str, label: &str, items: &[&str], values: Vec<Value>, index: usize) -> Field {
        Field { key: key.into(), label: label.into(), kind: FieldKind::Radio(items.iter().map(|s| s.to_string()).collect()), value: json!(index), choice_values: Some(values), error: false, scale: None, popup: false, icons: None, pref_codes: None, vertical: false, dot: false, side: false, disabled: false, clamp_ok: false }
    }
    /// Textfeld.
    pub fn text(key: &str, label: &str, value: &str) -> Field {
        Field { key: key.into(), label: label.into(), kind: FieldKind::Text { multiline: false }, value: json!(value), choice_values: None, error: false, scale: None, popup: false, icons: None, pref_codes: None, vertical: false, dot: false, side: false, disabled: false, clamp_ok: false }
    }
    /// Mehrzeiliges Textfeld.
    pub fn multiline(key: &str, label: &str, value: &str) -> Field {
        Field { key: key.into(), label: label.into(), kind: FieldKind::Text { multiline: true }, value: json!(value), choice_values: None, error: false, scale: None, popup: false, icons: None, pref_codes: None, vertical: false, dot: false, side: false, disabled: false, clamp_ok: false }
    }
    /// Farbfeld.
    pub fn color(key: &str, label: &str, hex: &str) -> Field {
        Field { key: key.into(), label: label.into(), kind: FieldKind::Color, value: json!(hex), choice_values: None, error: false, scale: None, popup: false, icons: None, pref_codes: None, vertical: false, dot: false, side: false, disabled: false, clamp_ok: false }
    }
    /// Überschrift.
    pub fn caption(label: &str) -> Field {
        Field { key: "_caption".into(), label: label.into(), kind: FieldKind::Caption, value: Value::Null, choice_values: None, error: false, scale: None, popup: false, icons: None, pref_codes: None, vertical: false, dot: false, side: false, disabled: false, clamp_ok: false }
    }
    /// Hinweis.
    pub fn note(label: &str) -> Field {
        Field { key: "_note".into(), label: label.into(), kind: FieldKind::Note, value: Value::Null, choice_values: None, error: false, scale: None, popup: false, icons: None, pref_codes: None, vertical: false, dot: false, side: false, disabled: false, clamp_ok: false }
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
                let v = if self.clamp_ok { v } else { fields::clamp_range(v, (min, max)) };
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
    /// Anordnung der Filterdialoge des Originals ([`FormDialog::with_side_buttons`]).
    pub side: bool,
    /// Tab-Folge der Filteranordnung ([`SideOrder`]).
    pub side_order: SideOrder,
    /// Schieberleisten der Filteranordnung sind Tab-Stopps (Original D42/D46; D41 nicht).
    pub slider_tabs: bool,
    /// Nur während des Zeichnens: OK/Abbrechen hat die eigene Anordnung schon selbst an ihrer Stelle
    /// der Tab-Folge gezeichnet ([`ok_cancel`]); `Some(Some(true))` = OK gedrückt.
    pub pressed: Option<Option<bool>>,
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

/// Entspricht der Wert eines Listeneintrags dem gemerkten Wert? Zahlen werden als Zahl, Texte als
/// Text, Wahrheitswerte als Wahrheitswert verglichen.
fn pref_matches(v: &Value, pv: &fp_engine::prefs::PrefValue) -> bool {
    use fp_engine::prefs::PrefValue;
    match v {
        Value::Number(n) => !matches!(pv, PrefValue::Bool(_)) && n.as_f64().is_some_and(|a| pv.as_real() == Some(a)),
        Value::String(s) => matches!(pv, PrefValue::Str(p) if p == s),
        Value::Bool(b) => matches!(pv, PrefValue::Bool(p) if p == b),
        _ => false,
    }
}

/// Reihenfolge der Bedienelemente (Tab-Folge) in der Filteranordnung
/// ([`FormDialog::with_side_buttons`]), je Vorlage wie im Original gemessen (PARITAET_B §2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SideOrder {
    /// Je Zeile Feld, dann Schieber; OK › Abbrechen › Kästchen der Knopfspalte › Vorschau
    /// (D41 Farbton/Sättigung).
    #[default]
    FieldsFirst,
    /// Je Zeile Schieber, dann Feld; Vorschau › OK › Abbrechen (D42 Helligkeit/Kontrast,
    /// Startfokus Schieber).
    SlidersFirst,
    /// Alle Felder › Vorschau › alle Schieber › OK › Abbrechen (D46 Unscharf maskieren).
    FieldsPreviewSliders,
}

/// Pseudo-Feldschlüssel des Kästchens „Vorschau“ in [`FormDialog::with_prefs`]
/// (z. B. `MotionBlurShowPreview`).
pub const PREVIEW_FIELD: &str = "_preview";

impl FormDialog {
    /// Neuer Dialog.
    pub fn new(id: &str, title: &str, command: &str, fields: Vec<Field>) -> FormDialog {
        FormDialog { id: id.into(), title: title.into(), fields, command: command.into(), base: json!({}), preview: false, preview_on: false, applied: None, applied_params: None, applied_doc: None, stale: None, width: 340.0, transform: None, prefs: Vec::new(), store_hook: None, focus_ok: false, initial: None, custom: None, side: false, side_order: SideOrder::FieldsFirst, slider_tabs: true, pressed: None }
    }
    /// Tab-Folge der Filteranordnung (siehe [`SideOrder`]).
    pub fn with_side_order(mut self, o: SideOrder) -> FormDialog {
        self.side_order = o;
        self
    }
    /// Schieberleisten der Filteranordnung sind keine Tab-Stopps (Original D41: „Tab nur Felder“).
    pub fn without_slider_tab_stops(mut self) -> FormDialog {
        self.slider_tabs = false;
        self
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
    /// Anordnung der Filterdialoge des Originals (PM-6 D41/D42/D45/D46/D48): Felder links in einem
    /// Gruppenrahmen, bei Schiebern die Beschriftung über der Leiste und das Feld rechts daneben;
    /// OK/Abbrechen rechts oben, Kästchen der Knopfspalte und „Vorschau“ rechts unten.
    pub fn with_side_buttons(mut self) -> FormDialog {
        self.side = true;
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
        // Kästchen zuerst, damit „Kolorieren“ die Bereiche von Farbton/Sättigung vor dem Klemmen
        // umschaltet (FX-4)
        for (fk, pk) in self.prefs.clone() {
            if let (Some(pv), Some(f)) = (p.get(&pk), self.fields.iter_mut().find(|f| f.key == fk && f.kind == FieldKind::Check))
                && let Some(b) = pv.as_bool()
            {
                f.value = json!(b);
            }
        }
        sync_colorize_ranges(self);
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
                        // Zahl mit Zahl, Text mit Text vergleichen (vorher war bei Text-Werten
                        // `None == None` wahr und es blieb immer Eintrag 0, Befund DLG-7)
                        Some(vals) => vals.iter().position(|v| pref_matches(v, &pv)),
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
                // Abschnittsüberschrift wie `ui_kit::section` (DES-1, DESIGN.md §4): Abschnittsfarbe, geätzte Linie
                ui.painter().text(pos2(x0, y + 12.0), Align2::LEFT_CENTER, &f.label, theme.bold(theme.fonts.ui), c.section_label);
                let lx = x0 + crate::widgets::text_width(ui, &f.label, &theme.bold(theme.fonts.ui)) + 8.0;
                if lx < area.right() - 18.0 {
                    crate::ui_kit::hline(ui.painter(), egui::Rangef::new(lx, area.right() - 14.0), y + 12.0, &theme);
                }
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
                if let Some(b) = fields::checkbox(ui, id, Rect::from_min_size(pos2(x0, y + 2.0), vec2(area.width() - 28.0, 16.0)), &f.label, on, !f.disabled) {
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

// ------------------------------------------------- Anordnung der Filterdialoge (PM-6, DLG-5)

/// Rand um Gruppenrahmen und Knopfspalte.
const SIDE_M: f32 = 10.0;
/// Innenabstand des Gruppenrahmens.
const SIDE_PAD: f32 = 10.0;
/// Zahlenfeld neben einer Schieberleiste.
const SIDE_FIELD_W: f32 = 44.0;
/// Zeile „Beschriftung über Schieber + Feld“.
const SIDE_SLIDER_ROW: f32 = 46.0;
/// Zeile „Beschriftung: Feld ▾“.
const SIDE_ROW: f32 = 26.0;
/// Zeile eines Kästchens.
const SIDE_CHECK_ROW: f32 = 22.0;
/// Mindestbreite des Inhalts mit Schiebern (Original D41/D42/D46/D48: Gruppe ≈ 280–330 px).
const SIDE_MIN_SLIDER_W: f32 = 280.0;
/// Mindestbreite des Inhalts ohne Schieber (Original D45: Gruppe ≈ 190 px).
const SIDE_MIN_W: f32 = 160.0;

/// Maße der Filteranordnung ([`FormDialog::with_side_buttons`]), relativ zum Client-Bereich.
#[derive(Clone, Debug, PartialEq)]
pub struct SideLayout {
    /// Größe des Client-Bereichs.
    pub size: egui::Vec2,
    /// Gruppenrahmen der Felder.
    pub group: Rect,
    /// Oberkante und Höhe je Feld im Gruppenrahmen (`None`: Kästchen der Knopfspalte).
    pub rows: Vec<Option<(f32, f32)>>,
    /// Breite der Beschriftung vor Feldern ohne Schieber.
    pub label_w: f32,
    /// Knopfspalte: linke Kante und Breite der Knöpfe.
    pub col_x: f32,
    pub button_w: f32,
}

impl FormDialog {
    /// Maße der Filteranordnung für die Texte der aktuellen Sprache.
    pub fn side_layout(&self, ctx: &egui::Context) -> SideLayout {
        let tw = |s: &str| super::text_w(ctx, s);
        let (ok, cancel) = (t("btn-ok"), t("btn-cancel"));
        let button_w = super::buttons_w(ctx, &[&ok, &cancel], 76.0);
        let mut col_w = button_w;
        let mut col_checks = 0usize;
        let (mut slider_lbl, mut lw, mut ctrl, mut check_w, mut any_slider) = (0f32, 0f32, 0f32, 0f32, false);
        for f in &self.fields {
            match &f.kind {
                FieldKind::Number { slider: true, unit, .. } => {
                    any_slider = true;
                    slider_lbl = slider_lbl.max(tw(&f.label));
                    if !unit.is_empty() {
                        ctrl = ctrl.max(SIDE_FIELD_W + 6.0 + tw(&unit_text(unit)));
                    }
                }
                FieldKind::Number { unit, .. } => {
                    lw = lw.max(tw(&f.label));
                    ctrl = ctrl.max(50.0 + if f.popup { 16.0 } else { 0.0 } + if unit.is_empty() { 0.0 } else { 6.0 + tw(&unit_text(unit)) });
                }
                FieldKind::Check if f.side => {
                    col_w = col_w.max(18.0 + tw(&f.label));
                    col_checks += 1;
                }
                FieldKind::Check => check_w = check_w.max(18.0 + tw(&f.label)),
                _ => lw = lw.max(tw(&f.label)),
            }
        }
        if self.preview {
            col_w = col_w.max(18.0 + tw(&t("dlg-preview")));
            col_checks += 1;
        }
        let inner = if any_slider { SIDE_MIN_SLIDER_W } else { SIDE_MIN_W }.max(slider_lbl + 8.0).max(lw + 6.0 + ctrl).max(check_w);
        let label_w = lw.ceil();
        let mut y = SIDE_PAD;
        let rows: Vec<Option<(f32, f32)>> = self
            .fields
            .iter()
            .map(|f| {
                let h = match &f.kind {
                    FieldKind::Check if f.side => return None,
                    FieldKind::Number { slider: true, .. } => SIDE_SLIDER_ROW,
                    FieldKind::Check => SIDE_CHECK_ROW,
                    _ => SIDE_ROW,
                };
                let r = (y, h);
                y += h;
                Some(r)
            })
            .collect();
        let group_h = (y + SIDE_PAD - 4.0).ceil();
        let col_h = 23.0 + 6.0 + 23.0 + 12.0 + col_checks as f32 * SIDE_CHECK_ROW;
        let h = group_h.max(col_h);
        let gw = (inner + 2.0 * SIDE_PAD).ceil();
        let group = Rect::from_min_size(pos2(SIDE_M, SIDE_M), vec2(gw, h));
        let col_x = group.right() + SIDE_M;
        let size = vec2((col_x + col_w + SIDE_M).ceil(), (h + 2.0 * SIDE_M).ceil());
        SideLayout { size, group, rows, label_w, col_x, button_w }
    }
}

/// Ein Bedienelement der Filteranordnung an seiner Stelle der Tab-Folge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SideStop {
    /// Zahlenfeld (mit Einheit und ggf. ▾) der Zeile `i`.
    Field(usize),
    /// Schieberleiste der Zeile `i`.
    Slider(usize),
    /// Kästchen im Gruppenrahmen.
    Check(usize),
    Ok,
    Cancel,
    /// Kästchen der Knopfspalte (z. B. „Kolorieren“).
    Column,
    Preview,
}

/// Tab-Folge der Filteranordnung nach [`FormDialog::side_order`]. egui führt den Fokus in der
/// Reihenfolge, in der die Bedienelemente angelegt werden; gezeichnet wird deshalb in dieser Folge.
fn side_stops(d: &FormDialog, rows: &[Option<(f32, f32)>]) -> Vec<SideStop> {
    use SideStop::*;
    let (mut v, mut sliders) = (Vec::new(), Vec::new());
    for (i, f) in d.fields.iter().enumerate() {
        if !matches!(rows.get(i), Some(Some(_))) {
            continue;
        }
        match &f.kind {
            FieldKind::Number { slider: true, .. } => match d.side_order {
                SideOrder::FieldsFirst => v.extend([Field(i), Slider(i)]),
                SideOrder::SlidersFirst => v.extend([Slider(i), Field(i)]),
                SideOrder::FieldsPreviewSliders => {
                    v.push(Field(i));
                    sliders.push(Slider(i));
                }
            },
            FieldKind::Number { .. } => v.push(Field(i)),
            FieldKind::Check => v.push(Check(i)),
            _ => {}
        }
    }
    match d.side_order {
        SideOrder::FieldsFirst => v.extend([Ok, Cancel, Column, Preview]),
        SideOrder::SlidersFirst => v.extend([Column, Preview, Ok, Cancel]),
        SideOrder::FieldsPreviewSliders => {
            v.extend([Column, Preview]);
            v.extend(sliders);
            v.extend([Ok, Cancel]);
        }
    }
    v
}

/// Filteranordnung ([`FormDialog::with_side_buttons`]): Felder im Gruppenrahmen, OK/Abbrechen
/// rechts oben, Kästchen der Knopfspalte und „Vorschau“ rechts unten (bündig mit dem Rahmen); alle
/// Bedienelemente in der Tab-Folge des Originals ([`SideOrder`]). Liefert „geändert“ und den
/// gedrückten Knopf (`Some(true)` = OK).
fn draw_side(ui: &mut Ui, d: &mut FormDialog, body: Rect, id: &str) -> (bool, Option<bool>) {
    let theme = Theme::current(ui.ctx());
    let c = theme.colors.clone();
    let sl = d.side_layout(ui.ctx());
    let group = sl.group.translate(body.min.to_vec2());
    ui.painter().rect_stroke(group, 2.0, Stroke::new(1.0, if theme.glow { c.line_dark } else { c.line_mid }), StrokeKind::Inside);
    let x0 = group.left() + SIDE_PAD;
    let right = group.right() - SIDE_PAD;
    let dlg_id = d.id.clone();
    let ctx = ui.ctx().clone();
    let slider_tabs = d.slider_tabs;
    // Lage je Zeile: Feld, Schieber (bei Schieberzeilen), Kästchen
    let geom = |d: &FormDialog, i: usize| -> Option<(Rect, Option<Rect>)> {
        let Some(Some((top, _))) = sl.rows.get(i).copied() else { return None };
        let y = group.top() + top;
        let f = &d.fields[i];
        match &f.kind {
            FieldKind::Number { slider: true, unit, .. } => {
                let uw = if unit.is_empty() { 0.0 } else { 6.0 + super::text_w(&ctx, &unit_text(unit)) };
                let er = Rect::from_min_size(pos2(right - uw - SIDE_FIELD_W, y + 18.0), vec2(SIDE_FIELD_W, 20.0));
                Some((er, Some(Rect::from_min_max(pos2(x0, y + 19.0), pos2(er.left() - 8.0, y + 37.0)))))
            }
            FieldKind::Number { .. } => Some((Rect::from_min_size(pos2(x0 + sl.label_w + 6.0, y), vec2(50.0, 20.0)), None)),
            FieldKind::Check => Some((Rect::from_min_size(pos2(x0, y + 2.0), vec2(right - x0, 16.0)), None)),
            _ => None,
        }
    };
    // Beschriftungen und Einheiten (nicht bedienbar, Reihenfolge egal)
    for i in 0..d.fields.len() {
        let Some((er, slider)) = geom(d, i) else { continue };
        let f = &d.fields[i];
        let FieldKind::Number { unit, .. } = &f.kind else { continue };
        if slider.is_some() {
            super::fit_label(ui, Rect::from_min_size(pos2(x0, er.top() - 18.0), vec2(right - x0, 16.0)), &f.label, true);
        } else {
            super::fit_label(ui, Rect::from_min_size(pos2(x0, er.top() + 2.0), vec2(sl.label_w, 16.0)), &f.label, true);
        }
        if !unit.is_empty() {
            let ux = if slider.is_none() && f.popup { er.right() + 15.0 } else { er.right() };
            ui.painter().text(pos2(ux + 6.0, er.center().y), Align2::LEFT_CENTER, unit_text(unit), theme.ui_font(), c.text);
        }
    }
    // Knopfspalte
    let bx = body.left() + sl.col_x;
    let by = body.top() + SIDE_M;
    let buttons = [Rect::from_min_size(pos2(bx, by), vec2(sl.button_w, 23.0)), Rect::from_min_size(pos2(bx, by + 29.0), vec2(sl.button_w, 23.0))];
    let col_w = body.right() - SIDE_M - bx;
    let col: Vec<usize> = d.fields.iter().enumerate().filter(|(_, f)| f.side && f.kind == FieldKind::Check).map(|(i, _)| i).collect();
    let col_top = body.top() + sl.group.bottom() - (col.len() + usize::from(d.preview)) as f32 * SIDE_CHECK_ROW + 3.0;
    let valid = d.valid();
    let (ok_l, cancel_l) = (t("btn-ok"), t("btn-cancel"));
    let (mut changed, mut result) = (false, None);
    for stop in side_stops(d, &sl.rows) {
        match stop {
            SideStop::Field(i) => {
                let Some((er, _)) = geom(d, i) else { continue };
                changed |= number_input(ui, &dlg_id, i, &mut d.fields[i], er, true);
            }
            SideStop::Slider(i) => {
                let Some((_, Some(sr))) = geom(d, i) else { continue };
                let f = &mut d.fields[i];
                let FieldKind::Number { min, max, decimals, .. } = f.kind else { continue };
                // keine Tab-Stopps (D41): erreicht Tab die Leiste, geht der Fokus weiter
                let skip = egui::Id::new((dlg_id.as_str(), i, "fp-slider-tab"));
                if !slider_tabs {
                    super::skip_tab_stop_before(ui, skip);
                }
                let mut val = f.value.as_f64().unwrap_or(min);
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(sr));
                child.spacing_mut().slider_width = sr.width() - 8.0;
                let r = crate::inspector::scale::slider(&mut child, &mut val, (min, max), f.effective_scale());
                if slider_tabs {
                    // erstes Bedienelement (D42): Startfokus auf der Leiste
                    super::autofocus_widget(&child, r.id);
                    // Klick bzw. Ziehen gibt der Leiste den Fokus (Win32-Trackbar), danach wirken
                    // die Pfeiltasten; Fokusrahmen um die ganze Leiste (CT-1a)
                    if r.drag_started() || r.clicked() {
                        r.request_focus();
                    }
                    if r.has_focus() {
                        ui.painter().rect_stroke(sr, 0.0, Stroke::new(1.0, c.focus), StrokeKind::Inside);
                    }
                } else {
                    super::skip_tab_stop_after(ui, skip, &r);
                }
                if r.changed() {
                    let p = 10f64.powi(decimals as i32);
                    f.value = json!((val * p).round() / p);
                    f.error = false;
                    changed = true;
                }
            }
            SideStop::Check(i) => {
                let Some((cr, _)) = geom(d, i) else { continue };
                let f = &mut d.fields[i];
                if let Some(b) = fields::checkbox(ui, (dlg_id.as_str(), i), cr, &f.label, f.value.as_bool().unwrap_or(false), !f.disabled) {
                    f.value = json!(b);
                    changed = true;
                }
            }
            SideStop::Ok => {
                if push_button(ui, buttons[0], &ok_l, true, valid) {
                    result = Some(true);
                }
            }
            SideStop::Cancel => {
                if push_button(ui, buttons[1], &cancel_l, false, true) {
                    result = Some(false);
                }
            }
            SideStop::Column => {
                for (k, &i) in col.iter().enumerate() {
                    let f = &mut d.fields[i];
                    let r = Rect::from_min_size(pos2(bx, col_top + k as f32 * SIDE_CHECK_ROW), vec2(col_w, 16.0));
                    if let Some(b) = fields::checkbox(ui, (dlg_id.as_str(), i), r, &f.label, f.value.as_bool().unwrap_or(false), !f.disabled) {
                        f.value = json!(b);
                        changed = true;
                    }
                }
            }
            SideStop::Preview => {
                let r = Rect::from_min_size(pos2(bx, col_top + col.len() as f32 * SIDE_CHECK_ROW), vec2(col_w, 16.0));
                if d.preview
                    && let Some(b) = fields::checkbox(ui, (id, "preview"), r, &t("dlg-preview"), d.preview_on, true)
                {
                    d.preview_on = b;
                    changed = true;
                }
            }
        }
    }
    (changed, result)
}

/// OK und Abbrechen eines Formulars an den Rechtecken `rects` zeichnen – für eigene Anordnungen
/// ([`CustomLayout`]), deren Vorlage die Knöpfe in der Tab-Folge vor bzw. zwischen den Feldern führt
/// (D27 Name › Typ › OK › Abbrechen › Kästchen, D28 OK › Abbrechen › Felder). Ohne diesen Aufruf
/// zeichnet [`show_form`] die Knöpfe nach den Feldern.
pub fn ok_cancel(ui: &mut Ui, d: &mut FormDialog, rects: [Rect; 2]) {
    let valid = d.valid();
    let mut r = None;
    if push_button(ui, rects[0], &t("btn-ok"), true, valid) {
        r = Some(true);
    }
    if push_button(ui, rects[1], &t("btn-cancel"), false, true) {
        r = Some(false);
    }
    d.pressed = Some(r);
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
    let txt = if f.dot { txt.replace(',', ".") } else { txt };
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
/// Farbton/Sättigung (Filter und Live-Effekt): Bereiche mit „Kolorieren“ umschalten wie im
/// Original (re): Farbton −180…180 bzw. 0…360, Sättigung −100…100 bzw. 0…100. Beim Umschalten
/// wird der Farbton auf den Kreis umgerechnet (−30 ↔ 330), die Sättigung geklemmt (Annahme FX-4).
/// Ohne Felder `hue`/`saturation`/`colorize` ohne Wirkung.
pub fn sync_colorize_ranges(d: &mut FormDialog) {
    let Some(on) = d.fields.iter().find(|f| f.key == "colorize" && f.kind == FieldKind::Check).and_then(|f| f.value.as_bool()) else { return };
    for f in d.fields.iter_mut() {
        let (lo, hi, sc) = match (f.key.as_str(), on) {
            ("hue", false) => (-180.0, 180.0, Some(crate::inspector::scale::HUE)),
            ("hue", true) => (0.0, 360.0, None),
            ("saturation", false) => (-100.0, 100.0, Some(crate::inspector::scale::SATURATION)),
            ("saturation", true) => (0.0, 100.0, None),
            _ => continue,
        };
        let is_hue = f.key == "hue";
        if let FieldKind::Number { min, max, .. } = &mut f.kind {
            if *min == lo && *max == hi {
                continue;
            }
            *min = lo;
            *max = hi;
            f.scale = sc;
            if let Some(v) = f.value.as_f64() {
                let nv = match (is_hue, on) {
                    (true, true) => v.rem_euclid(360.0),
                    (true, false) if v > 180.0 => v - 360.0,
                    _ => v.clamp(lo, hi),
                };
                f.value = json!(nv);
                f.error = false;
            }
        }
    }
}

pub fn show_form(ctx: &egui::Context, st: &mut AppState, d: &mut FormDialog) -> Option<bool> {
    sync_colorize_ranges(d);
    let side = d.side && d.custom.is_none();
    let size = match d.custom {
        Some(l) => (l.size)(ctx, d),
        None if side => d.side_layout(ctx).size,
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
        if side {
            // Felder, Knöpfe und Kästchen in der Tab-Folge des Originals
            let (ch, res) = draw_side(ui, d, body, &id);
            changed = ch;
            result = res;
            let _ = ui.allocate_rect(body, Sense::hover());
            return;
        }
        d.pressed = None;
        let custom = d.custom.map(|l| (l.draw)(ui, st, d, body));
        changed = match custom {
            Some((ch, _)) => ch,
            None => draw_fields(ui, st, d, body),
        };
        // eigene Anordnung hat OK/Abbrechen schon an ihrer Stelle der Tab-Folge gezeichnet
        if let Some(res) = d.pressed.take() {
            result = res;
            let _ = ui.allocate_rect(body, Sense::hover());
            return;
        }
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
    // Felder, die erst beim OK geklemmt werden (19105–19108): Wert korrigieren, Dialog bleibt offen,
    // Fokus mit markiertem Inhalt im ersten korrigierten Feld (re\11 §3.5.5)
    if result == Some(true) {
        let mut first = None;
        for (i, f) in d.fields.iter_mut().enumerate() {
            if f.clamp_ok && f.clamp_now() && first.is_none() {
                first = Some(i);
            }
        }
        if let Some(i) = first {
            result = None;
            let te = egui::Id::new((d.id.as_str(), i)).with("te");
            let len = d.fields[i].value.as_f64().map(|v| fields::fmt_num(v, 0).chars().count()).unwrap_or(0);
            ctx.memory_mut(|m| m.request_focus(te));
            super::select_all(ctx, te, len);
            ctx.request_repaint();
        }
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
