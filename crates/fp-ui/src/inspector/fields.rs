//! Bausteine der Inspektor-Felder im Stil des Originals: Platzierung in Dialogeinheiten (DLU,
//! Lage aus `dialoge.json`), Zahlenfeld mit Popup-Schieber („100 ▾“), Kombifeld, Textfeld,
//! Symbol-Umschalter, Kontrollkästchen und Knopf. Alle Farben aus dem [`Theme`].
//!
//! Felder liefern geänderte Werte zurück (`Option<…>`); der Aufrufer schickt daraus einen Befehl.
//! Textfelder übernehmen ihren Wert bei Eingabe bzw. Fokusverlust (Esc verwirft), wie Windows.

use crate::inspector::glyphs::{self, Glyph};
use crate::theme::Theme;
use egui::{pos2, vec2, Align2, Color32, Id, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui};
use std::hash::Hash;

/// Umrechnung DLU → Pixel (11 §6: 1,5 px waagerecht, 1,625 px senkrecht).
pub const DLU_X: f32 = 1.5;
pub const DLU_Y: f32 = 1.625;

/// Ursprung für DLU-Angaben.
#[derive(Clone, Copy, Debug)]
pub struct Dl(pub Pos2);

impl Dl {
    /// Ursprung bei `p`.
    pub fn at(p: Pos2) -> Dl {
        Dl(p)
    }
    /// Rechteck aus `[x, y, b, h]` in DLU.
    pub fn r(&self, x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_min_size(pos2((self.0.x + x * DLU_X).round(), (self.0.y + y * DLU_Y).round()), vec2((w * DLU_X).round(), (h * DLU_Y).round()))
    }
    /// Verschobener Ursprung (DLU).
    pub fn offset(&self, x: f32, y: f32) -> Dl {
        Dl(pos2(self.0.x + x * DLU_X, self.0.y + y * DLU_Y))
    }
}

// ---------------------------------------------------------------------- Einpassen und Prüfprotokoll
//
// Die Felder haben feste Lagen nach dem Original. Übersetzte Beschriftungen können länger sein als
// die deutschen: Im Inspektor ([`set_fit`]) wird ein zu langer Text erst leicht verkleinert, dann
// mit „…“ gekürzt (voller Text als Tooltip). Ohne `set_fit` (Bedienfelder, Dialoge) und in der
// Ausgangssprache Deutsch bleibt alles wie bisher. Für Layout-Tests schreibt [`set_audit`] jede Beschriftung und jedes Feld mit Lage
// in ein Protokoll ([`take_audit`]).

const FIT_ID: &str = "pi-fit-scope";
const AUDIT_ON: &str = "pi-audit-on";
const AUDIT_LOG: &str = "pi-audit-log";

/// Ein Eintrag des Layout-Protokolls.
#[derive(Clone, Debug, PartialEq)]
pub struct TextAudit {
    /// `label`, `check`, `radio`, `button`, `combo-item`, `field`.
    pub kind: &'static str,
    /// Text vor dem Einpassen.
    pub text: String,
    /// Für den Text vorgesehene Fläche.
    pub avail: Rect,
    /// Tatsächlich gezeichnete Fläche (bei `combo-item`: benötigte Fläche des ungekürzten Texts).
    pub drawn: Rect,
    /// Sichtbarer Bereich (Quadrant).
    pub clip: Rect,
    /// 0 = passt, 1 = verkleinert, 2 = mit „…“ gekürzt.
    pub fit: u8,
    /// Im Inspektor gezeichnet (innerhalb von [`set_fit`]).
    pub inspector: bool,
}

/// Einpassen langer Beschriftungen ein-/ausschalten (der Inspektor schaltet es für seine
/// Quadranten ein; in der Ausgangssprache bleibt alles wie bisher).
pub fn set_fit(ctx: &egui::Context, on: bool) {
    ctx.data_mut(|d| d.insert_temp(Id::new(FIT_ID), on));
}

fn fit_scope(ui: &Ui) -> bool {
    ui.ctx().data(|d| d.get_temp::<bool>(Id::new(FIT_ID))).unwrap_or(false)
}

/// Einpassen aktiv: im Inspektor und nicht in der Ausgangssprache (Deutsch sieht exakt aus wie
/// bisher, Regel „Deutsch unverändert“; die Felder sind für die deutschen Texte bemessen).
fn fit_on(ui: &Ui) -> bool {
    fit_scope(ui) && !source_language()
}

/// Ist die Ausgangssprache (Deutsch) eingestellt?
pub fn source_language() -> bool {
    fp_i18n::current_language() == fp_i18n::SOURCE_LANG
}

/// Layout-Protokoll ein-/ausschalten (Tests).
pub fn set_audit(ctx: &egui::Context, on: bool) {
    ctx.data_mut(|d| {
        d.insert_temp(Id::new(AUDIT_ON), on);
        d.remove::<Vec<TextAudit>>(Id::new(AUDIT_LOG));
    });
}

/// Protokoll abholen und leeren.
pub fn take_audit(ctx: &egui::Context) -> Vec<TextAudit> {
    ctx.data_mut(|d| {
        let v = d.get_temp::<Vec<TextAudit>>(Id::new(AUDIT_LOG)).unwrap_or_default();
        d.remove::<Vec<TextAudit>>(Id::new(AUDIT_LOG));
        v
    })
}

fn audit(ui: &Ui, kind: &'static str, text: &str, avail: Rect, drawn: Rect, fit: u8) {
    if !ui.ctx().data(|d| d.get_temp::<bool>(Id::new(AUDIT_ON))).unwrap_or(false) {
        return;
    }
    let e = TextAudit { kind, text: text.to_string(), avail, drawn, clip: ui.clip_rect(), fit, inspector: fit_scope(ui) };
    ui.ctx().data_mut(|d| d.get_temp_mut_or_default::<Vec<TextAudit>>(Id::new(AUDIT_LOG)).push(e));
}

fn text_w(ui: &Ui, s: &str, font: &egui::FontId) -> f32 {
    ui.ctx().fonts_mut(|f| f.layout_no_wrap(s.to_string(), font.clone(), Color32::WHITE).size().x)
}

/// Passt `text` in `max_w` ein: unverändert, leicht verkleinert (bis 85 %, mind. 8 px) oder mit „…“.
fn fit_text(ui: &Ui, text: &str, font: egui::FontId, max_w: f32) -> (String, egui::FontId, u8) {
    if !fit_on(ui) || text_w(ui, text, &font) <= max_w + 0.5 {
        return (text.to_string(), font, 0);
    }
    let min = (font.size * 0.85).max(8.0).min(font.size);
    let mut size = font.size - 0.5;
    while size >= min {
        let f = egui::FontId::new(size, font.family.clone());
        if text_w(ui, text, &f) <= max_w + 0.5 {
            return (text.to_string(), f, 1);
        }
        size -= 0.5;
    }
    let f = egui::FontId::new(min, font.family.clone());
    let mut s: String = text.to_string();
    while !s.is_empty() && text_w(ui, &format!("{}…", s.trim_end()), &f) > max_w {
        s.pop();
    }
    (format!("{}…", s.trim_end()), f, 2)
}

/// Text eingepasst zeichnen (`anchor` an `align`), protokollieren, bei Kürzung Tooltip.
#[allow(clippy::too_many_arguments)]
fn fitted_text(ui: &Ui, kind: &'static str, avail: Rect, anchor: Pos2, align: Align2, text: &str, font: egui::FontId, col: Color32) -> Rect {
    let (shown, f, fit) = fit_text(ui, text, font, avail.width());
    let drawn = ui.painter().text(anchor, align, &shown, f, col);
    if fit == 2 {
        let _ = ui.interact(drawn, Id::new(("pi-fit-tip", kind, avail.min.x as i32, avail.min.y as i32)), Sense::hover()).on_hover_text(text);
    }
    audit(ui, kind, text, avail, drawn, fit);
    drawn
}

/// Beschriftung links ausgerichtet, senkrecht mittig.
pub fn label(ui: &Ui, r: Rect, text: &str, enabled: bool) {
    let t = Theme::current(ui.ctx());
    let col = if enabled { t.colors.text } else { t.colors.text_disabled };
    fitted_text(ui, "label", r, pos2(r.left(), r.center().y), Align2::LEFT_CENTER, text, t.ui_font(), col);
}

/// Beschriftung mit Eigennamen/Messwerten (wird in der Testsprache nicht markiert erwartet).
pub fn label_name(ui: &Ui, r: Rect, text: &str, enabled: bool) {
    let t = Theme::current(ui.ctx());
    let col = if enabled { t.colors.text } else { t.colors.text_disabled };
    fitted_text(ui, "name", r, pos2(r.left(), r.center().y), Align2::LEFT_CENTER, text, t.ui_font(), col);
}

/// Beschriftung rechts ausgerichtet (wie die Original-Labels vor Feldern).
pub fn label_right(ui: &Ui, r: Rect, text: &str, enabled: bool) {
    let t = Theme::current(ui.ctx());
    let col = if enabled { t.colors.text } else { t.colors.text_disabled };
    fitted_text(ui, "label", r, pos2(r.right(), r.center().y), Align2::RIGHT_CENTER, text, t.ui_font(), col);
}

/// Rahmen eines Eingabefelds.
pub fn field_frame(ui: &Ui, r: Rect, enabled: bool, focused: bool) {
    let t = Theme::current(ui.ctx());
    let c = &t.colors;
    ui.painter().rect_filled(r, 0.0, if enabled { c.field_bg } else { c.frame_bg });
    let border = if focused { c.focus } else if enabled { c.field_border } else { c.line_light };
    ui.painter().rect_stroke(r, 0.0, Stroke::new(1.0, border), StrokeKind::Inside);
}

/// Tasten, die ein fokussiertes Textfeld selbst behält: Pfeile und Esc (Esc verwirft im Feld und
/// darf nicht an Leinwand oder Werkzeug gehen).
pub const TEXT_FILTER: egui::EventFilter = egui::EventFilter { horizontal_arrows: true, vertical_arrows: true, tab: false, escape: true };

/// Ein Feld hat in diesem Bild durch einen Mausklick daneben übernommen: der Inspektor führt die
/// Befehle sofort aus ([`take_commit_now`]), **bevor** die Leinwand (im selben Bild danach
/// gezeichnet) den Klick verarbeitet und die Auswahl ändert. Sonst ginge der Wert bei schnellen
/// echten Klicks (Drücken und Loslassen in einem Bild) ins Leere (PARITAET_A F7/F9; im Original
/// übernimmt das Feld bei WM_KILLFOCUS vor dem Mausklick der Leinwand).
const COMMIT_NOW: &str = "pi-commit-now";

fn request_commit_now(ctx: &egui::Context) {
    ctx.data_mut(|d| d.insert_temp(Id::new(COMMIT_NOW), true));
}

/// Soll der Inspektor die Befehle dieses Bildes sofort ausführen? (setzt die Anfrage zurück)
pub fn take_commit_now(ctx: &egui::Context) -> bool {
    ctx.data_mut(|d| {
        let v = d.get_temp::<bool>(Id::new(COMMIT_NOW)).unwrap_or(false);
        d.remove::<bool>(Id::new(COMMIT_NOW));
        v
    })
}

/// Eingabefeld mit Puffer. `numeric`: Zahlenfeld des Inspektors – wie im Original verwirft Esc den
/// getippten Wert nicht (übernommen wird beim Fokusverlust bzw. mit Eingabe/Tab, PARITAET_A F7).
/// Strg+A markiert wie in jedem Windows-Eingabefeld den Feldtext (F10, im Original von PM-2 in
/// 3 Läufen gemessen; die Taste bleibt im Feld, kein „Alles auswählen“ im Dokument).
fn buffered_edit(ui: &mut Ui, id: Id, r: Rect, current: &str, enabled: bool, right: bool, numeric: bool) -> (Option<String>, Response) {
    let t = Theme::current(ui.ctx());
    let te_id = id.with("te");
    let focused = ui.memory(|m| m.has_focus(te_id));
    // Puffer beim Tippen; ändert sich der Wert von außen (Schieber, Pfeil) und wurde noch nichts
    // getippt, folgt der Puffer – sonst überschriebe der Fokusverlust den neuen Wert mit dem alten
    let orig_id = id.with("orig");
    let mut buf: String = if focused {
        let stored = ui.data(|d| d.get_temp::<String>(id));
        let orig = ui.data(|d| d.get_temp::<String>(orig_id));
        match (stored, orig) {
            (Some(b), Some(o)) if b == o && o != current => current.to_string(),
            (Some(b), _) => b,
            _ => current.to_string(),
        }
    } else {
        current.to_string()
    };
    if focused {
        let untouched = ui.data(|d| d.get_temp::<String>(orig_id)).is_none_or(|o| o != current && buf == current);
        if untouched {
            ui.data_mut(|d| d.insert_temp(orig_id, current.to_string()));
        }
    } else {
        ui.data_mut(|d| d.insert_temp(orig_id, current.to_string()));
    }
    field_frame(ui, r, enabled, focused && t.glow);
    audit(ui, "field", "", r, r, 0);
    let inner = r.shrink2(vec2(3.0, 1.0));
    let col = if enabled { t.colors.field_text } else { t.colors.text_disabled };
    let te = egui::TextEdit::singleline(&mut buf)
        .id(te_id)
        .frame(egui::Frame::NONE)
        .font(t.ui_font())
        .text_color(col)
        .margin(egui::Margin::ZERO)
        .vertical_align(egui::Align::Center)
        .horizontal_align(if right { egui::Align::Max } else { egui::Align::Min })
        .desired_width(inner.width())
        // Esc selbst behandeln: sonst nimmt egui den Fokus schon zu Beginn des Bildes weg und die
        // Taste erreicht zusätzlich die Leinwand (Auswahl aufheben)
        .event_filter(crate::inspector::fields::TEXT_FILTER);
    let resp = ui.add_enabled_ui(enabled, |ui| ui.put(inner, te)).inner;
    // Fokus des Felds selbst, ohne den Fensterfokus: `Response::has_focus` ist auch `false`, solange
    // das Hauptfenster nicht den Systemfokus hat (RDP ohne Vordergrund, Brücke, Alt+Tab). Dann ging
    // der Puffer nach jedem Bild verloren und jede getippte Ziffer landete wieder auf dem alten Wert
    // (P-DLG1, Befund PM-5 „Zahlenfelder nehmen keine Eingabe mehr an“).
    let has_focus = |ui: &Ui| ui.memory(|m| m.has_focus(te_id));
    // erstes Feld eines frisch geöffneten Dialogs: Fokus, Inhalt ganz markiert
    crate::dialogs::autofocus_field(ui, &resp, current.chars().count());
    crate::dialogs::select_on_tab(ui, &resp, current.chars().count());
    let esc = ui.input(|i| i.key_pressed(egui::Key::Escape));
    if esc && has_focus(ui) && !numeric {
        ui.memory_mut(|m| m.surrender_focus(te_id));
        ui.data_mut(|d| d.remove::<String>(id));
        return (None, resp);
    }
    let mut out = None;
    // Klick daneben (z. B. auf die Leinwand): schon beim Drücken übernehmen – egui nimmt den Fokus
    // erst beim Loslassen weg, und dann hat die Leinwand die Auswahl schon geändert (der Wert
    // ginge ins Leere; Original: übernommen, PARITAET_A F7/F9)
    if has_focus(ui) && !resp.hovered() && ui.input(|i| i.pointer.any_pressed()) {
        ui.memory_mut(|m| m.surrender_focus(te_id));
        ui.data_mut(|d| d.remove::<String>(id));
        let out = (buf != current).then_some(buf);
        if out.is_some() {
            request_commit_now(ui.ctx());
        }
        return (out, resp);
    }
    if resp.lost_focus() && (numeric || !esc) && buf != current {
        out = Some(buf.clone());
        // Fokusverlust durch einen Klick (Drücken und Loslassen im selben Bild, egui nimmt den
        // Fokus beim Loslassen): ebenfalls sofort übernehmen
        if ui.input(|i| i.pointer.any_pressed() || i.pointer.any_released()) {
            request_commit_now(ui.ctx());
        }
    }
    if has_focus(ui) {
        ui.data_mut(|d| d.insert_temp(id, buf));
    } else {
        ui.data_mut(|d| d.remove::<String>(id));
    }
    (out, resp)
}

/// Textfeld; liefert den neuen Text nach Eingabe/Fokusverlust.
pub fn text(ui: &mut Ui, id: impl Hash + std::fmt::Debug, r: Rect, current: &str, enabled: bool) -> Option<String> {
    buffered_edit(ui, Id::new(id), r, current, enabled, false, false).0
}

/// Zahl formatieren (Komma, ohne überflüssige Nachkommastellen).
pub fn fmt_num(v: f64, decimals: usize) -> String {
    let s = format!("{v:.decimals$}");
    let s = if decimals > 0 { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s };
    // Dezimaltrennzeichen der Sprache (Deutsch „,“, Englisch „.“); Eingaben akzeptieren beides
    let sep = fp_i18n::t("fmt-decimal-separator");
    let sep = if sep == "." || sep == "," { sep } else { ",".to_string() };
    s.replace('.', &sep)
}

/// Zahl lesen (Komma oder Punkt; Rest nach der Zahl wird ignoriert wie `%d`).
pub fn parse_num(s: &str) -> Option<f64> {
    let t = s.trim().replace(',', ".");
    let end = t.char_indices().take_while(|(i, c)| c.is_ascii_digit() || *c == '.' || (*i == 0 && (*c == '-' || *c == '+'))).map(|(i, c)| i + c.len_utf8()).last()?;
    t[..end].parse::<f64>().ok().filter(|v| v.is_finite())
}

/// Ziffernfeld lesen: alle Nicht-Ziffern werden ignoriert (wie ein Windows-Ziffernfeld).
pub fn parse_digits(s: &str) -> Option<f64> {
    let d: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if d.is_empty() { None } else { d.parse::<f64>().ok() }
}

/// Ganzzahl lesen wie C-`atoi` (Ziffernfelder B/H/X/Y des Originals): führende Leerzeichen,
/// Vorzeichen, Ziffern; der Rest wird ignoriert, ohne Ziffern ergibt sich 0 („x“ → 0).
pub fn parse_atoi(s: &str) -> f64 {
    let t = s.trim_start();
    let (neg, rest) = match t.chars().next() {
        Some('-') => (true, &t[1..]),
        Some('+') => (false, &t[1..]),
        _ => (false, t),
    };
    let d: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    let v = d.parse::<f64>().unwrap_or(0.0);
    if neg { -v } else { v }
}

/// Ganzzahlfeld B/H/X/Y des Inspektors wie im Original (PARITAET_A F12/F13): gelesen wird wie
/// [`parse_atoi`], und ein getipptes Komma fügt kein Zeichen ein, sondern setzt die Schreibmarke
/// an den Anfang („12,5“ → „512“). Liefert den getippten Wert **ungeklemmt** nach Eingabe/Tab/
/// Fokusverlust; Grenzen setzt der Aufrufer.
pub fn number_box(ui: &mut Ui, id: impl Hash + std::fmt::Debug, edit: Rect, value: Option<f64>, enabled: bool) -> Option<f64> {
    let id = Id::new(id);
    let cur = value.map(|v| fmt_num(v, 0)).unwrap_or_default();
    let te_id = id.with("te");
    if enabled && ui.memory(|m| m.has_focus(te_id)) {
        comma_moves_caret_home(ui, id, te_id, &cur);
    }
    let (txt, _) = buffered_edit(ui, id, edit, &cur, enabled, false, true);
    txt.map(|s| parse_atoi(&s))
}

/// Getippte Zeichen eines fokussierten [`number_box`] selbst einfügen, sobald ein Komma darunter
/// ist: Komma = Schreibmarke an den Anfang (Verhalten des Originals), sonst Auswahl ersetzen.
fn comma_moves_caret_home(ui: &mut Ui, id: Id, te_id: Id, current: &str) {
    let has_comma = ui.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Text(t) if t.contains(','))));
    if !has_comma {
        return;
    }
    let mut buf: Vec<char> = ui.data(|d| d.get_temp::<String>(id)).unwrap_or_else(|| current.to_string()).chars().collect();
    let mut state = egui::TextEdit::load_state(ui.ctx(), te_id).unwrap_or_default();
    let n = buf.len();
    let (mut a, mut b) = state
        .cursor
        .char_range()
        .map(|r| (r.primary.index.0.min(r.secondary.index.0).min(n), r.primary.index.0.max(r.secondary.index.0).min(n)))
        .unwrap_or((n, n));
    let events = ui.input_mut(|i| std::mem::take(&mut i.events));
    let mut keep = Vec::with_capacity(events.len());
    for e in events {
        match e {
            egui::Event::Text(t) => {
                for ch in t.chars() {
                    if ch == ',' {
                        (a, b) = (0, 0);
                    } else {
                        buf.splice(a..b, [ch]);
                        a += 1;
                        b = a;
                    }
                }
            }
            other => keep.push(other),
        }
    }
    ui.input_mut(|i| i.events = keep);
    ui.data_mut(|d| d.insert_temp(id, buf.into_iter().collect::<String>()));
    state.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(a))));
    state.store(ui.ctx(), te_id);
}

/// Wert auf einen Bereich klemmen (konservativ wie 11 §6: Eingaben auf den Schieberbereich).
pub fn clamp_range(v: f64, range: (f64, f64)) -> f64 {
    v.clamp(range.0.min(range.1), range.0.max(range.1))
}

/// Zahlenfeld mit optionalem Popup-Schieber (`arrow`). `value = None` zeigt ein leeres Feld
/// (gemischte Auswahl). Liefert den neuen, auf `range` geklemmten Wert.
#[allow(clippy::too_many_arguments)]
pub fn number(ui: &mut Ui, id: impl Hash + std::fmt::Debug, edit: Rect, arrow: Option<Rect>, value: Option<f64>, range: (f64, f64), decimals: usize, enabled: bool) -> Option<f64> {
    let id = Id::new(id);
    // beim Ziehen im Popup-Schieber zeigt das Feld den Wert schon mit (übernommen wird beim Loslassen)
    let live = arrow.and_then(|_| ui.data(|d| d.get_temp::<f64>(id.with("arrow").with("val"))));
    let cur = live.or(value).map(|v| fmt_num(v, decimals)).unwrap_or_default();
    let (txt, _) = buffered_edit(ui, id, edit, &cur, enabled, false, true);
    // nicht negative Ganzzahlfelder sind im Original Ziffernfelder: nur Ziffern zählen
    // („-5“ → 5, „12,5“ → 125 → auf den Bereich begrenzt)
    let digits_only = decimals == 0 && range.0 >= 0.0;
    let mut out = txt.and_then(|s| if digits_only { parse_digits(&s) } else { parse_num(&s) }).map(|v| clamp_range(v, range));
    if let Some(a) = arrow
        && let Some(v) = slider_arrow(ui, id.with("arrow"), a, value.unwrap_or(range.0), range, decimals, enabled)
    {
        out = Some(v);
    }
    out
}

/// Pfeilknopf ▾ mit senkrechtem Popup-Schieber (linear) wie im Original. Übernommen wird beim
/// Loslassen.
pub fn slider_arrow(ui: &mut Ui, id: Id, r: Rect, value: f64, range: (f64, f64), decimals: usize, enabled: bool) -> Option<f64> {
    slider_arrow_vertical(ui, id, r, value, range, crate::inspector::scale::Scale::linear(decimals), enabled)
}

/// Breite des senkrechten Popup-Schiebers, Länge der Spur und Rand über/unter der Spur (Pixel;
/// Original `re\_testdateien\ux\uxa\werkzeug\bilder\o_t1_c.png`: Popup 24 × 222, Spur ≈ 209).
pub const VSLIDER_W: f32 = 20.0;
pub const VSLIDER_TRACK: f32 = 206.0;
pub const VSLIDER_PAD: f32 = 6.0;
const VSLIDER_MARGIN: f32 = 1.0;

/// Pfeilknopf ▾ mit **senkrechtem** Popup-Schieber über dem Feld (Inspektor, PARITAET_A T4): oben
/// der größte, unten der kleinste Wert; das Popup liegt so, dass der Schieberknopf beim aktuellen
/// Wert auf Höhe des Pfeils steht (am Fensterrand verschoben). Ziehen zeigt den Wert live im Feld,
/// Loslassen bzw. ein Klick auf die Spur übernimmt und schließt; Esc oder ein Klick daneben
/// schließt ohne Änderung.
#[allow(clippy::too_many_arguments)]
pub fn slider_arrow_vertical(ui: &mut Ui, id: Id, r: Rect, value: f64, range: (f64, f64), scale: crate::inspector::scale::Scale, enabled: bool) -> Option<f64> {
    let t = Theme::current(ui.ctx());
    let resp = ui.interact(r, id, if enabled { Sense::click() } else { Sense::hover() });
    audit(ui, "field", "", r, r, 0);
    let c = t.colors.clone();
    if resp.hovered() && enabled {
        ui.painter().rect_filled(r, 0.0, c.button_hover_bg);
        ui.painter().rect_stroke(r, 0.0, Stroke::new(1.0, c.button_hover_border), StrokeKind::Inside);
    }
    glyphs::chevron_down(ui.painter(), r, if enabled { c.icon_fg } else { c.icon_disabled });
    let key = id.with("val");
    let popup_id = egui::Popup::default_response_id(&resp);
    if !egui::Popup::is_id_open(ui.ctx(), popup_id) {
        ui.data_mut(|d| d.remove::<f64>(key));
    }
    let (min, max) = (range.0.min(range.1), range.0.max(range.1));
    let max_pos = scale.max_pos(min, max).max(1.0);
    let frac_of = |v: f64| (scale.to_pos(v, min, max) / max_pos).clamp(0.0, 1.0) as f32;
    // Lage nach dem Wert beim Öffnen (das Popup springt beim Ziehen nicht mit)
    let top = r.center().y - (VSLIDER_MARGIN + VSLIDER_PAD + (1.0 - frac_of(value)) * VSLIDER_TRACK);
    let left = r.center().x - VSLIDER_W / 2.0 - VSLIDER_MARGIN;
    let frame = egui::Frame::NONE.fill(c.frame_bg).stroke(Stroke::new(1.0, c.menu_border)).inner_margin(egui::Margin::same(VSLIDER_MARGIN as i8));
    let mut out = None;
    egui::Popup::from_toggle_button_response(&resp)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .frame(frame)
        .at_position(pos2(left, top))
        .align(egui::RectAlign::BOTTOM_START)
        .gap(0.0)
        .show(|ui| {
            let mut v: f64 = ui.data(|d| d.get_temp(key)).unwrap_or(value);
            let (sr, sresp) = ui.allocate_exact_size(vec2(VSLIDER_W, VSLIDER_TRACK + 2.0 * VSLIDER_PAD), Sense::click_and_drag());
            let track_top = sr.top() + VSLIDER_PAD;
            if (sresp.dragged() || sresp.drag_started() || sresp.clicked())
                && let Some(p) = sresp.interact_pointer_pos()
            {
                let f = (1.0 - (p.y - track_top) / VSLIDER_TRACK).clamp(0.0, 1.0) as f64;
                v = scale.to_value((f * max_pos).round(), min, max);
                ui.data_mut(|d| d.insert_temp(key, v));
            }
            // Rille und Knopf (Pfeil nach links wie im Original)
            let cx = sr.center().x.floor() + 0.5;
            ui.painter().vline(cx - 0.5, track_top..=track_top + VSLIDER_TRACK, Stroke::new(1.0, c.line_mid));
            ui.painter().vline(cx + 0.5, track_top..=track_top + VSLIDER_TRACK, Stroke::new(1.0, c.line_light));
            let y = track_top + (1.0 - frac_of(v)) * VSLIDER_TRACK;
            let col = if sresp.hovered() || sresp.dragged() { c.focus } else { c.icon_fg };
            let pts = vec![pos2(cx - 6.0, y), pos2(cx - 2.0, y - 4.0), pos2(cx + 5.0, y - 4.0), pos2(cx + 5.0, y + 4.0), pos2(cx - 2.0, y + 4.0)];
            ui.painter().add(egui::Shape::convex_polygon(pts, col, Stroke::NONE));
            sresp.widget_info(|| egui::WidgetInfo::slider(true, v, ""));
            if sresp.drag_stopped() || sresp.clicked() {
                out = Some(v);
                ui.data_mut(|d| d.remove::<f64>(key));
                ui.close();
            }
        });
    out
}

/// Pfeilknopf ▾ mit Popup-Schieber und Kennlinie des Originals ([`crate::inspector::scale`]).
#[allow(clippy::too_many_arguments)]
pub fn slider_arrow_scaled(ui: &mut Ui, id: Id, r: Rect, value: f64, range: (f64, f64), decimals: usize, scale: crate::inspector::scale::Scale, enabled: bool) -> Option<f64> {
    let t = Theme::current(ui.ctx());
    let resp = ui.interact(r, id, if enabled { Sense::click() } else { Sense::hover() });
    audit(ui, "field", "", r, r, 0);
    let c = &t.colors;
    if resp.hovered() && enabled {
        ui.painter().rect_filled(r, 0.0, c.button_hover_bg);
        ui.painter().rect_stroke(r, 0.0, Stroke::new(1.0, c.button_hover_border), StrokeKind::Inside);
    }
    glyphs::chevron_down(ui.painter(), r, if enabled { c.icon_fg } else { c.icon_disabled });
    let mut out = None;
    let key = id.with("val");
    egui::Popup::from_toggle_button_response(&resp)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .frame(crate::menu::menu_frame(&t))
        .show(|ui| {
            let mut v: f64 = ui.data(|d| d.get_temp(key)).unwrap_or(value);
            ui.spacing_mut().slider_width = 130.0;
            let r2 = crate::inspector::scale::slider(ui, &mut v, range, scale);
            ui.label(egui::RichText::new(fmt_num(v, decimals)).font(t.small_font()).color(c.text));
            if r2.dragged() || r2.changed() {
                ui.data_mut(|d| d.insert_temp(key, v));
            }
            if r2.drag_stopped() || (r2.changed() && !r2.dragged()) {
                out = Some(v);
                ui.data_mut(|d| d.remove::<f64>(key));
            }
        });
    out
}

/// Ein Eintrag „-----“ in Kombifeld-Listen ist ein Trenner.
pub fn is_separator(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c == '-' || c == '—')
}

/// Zeichnet ein Kombifeld (Feld + Chevron) und liefert die Antwort.
pub fn combo_box_frame(ui: &mut Ui, id: Id, r: Rect, text: &str, enabled: bool) -> Response {
    let t = Theme::current(ui.ctx());
    let c = &t.colors;
    let resp = ui.interact(r, id, if enabled { Sense::click() } else { Sense::hover() });
    // Barrierefreiheit (und egui_kittest): Kombifeld mit angezeigtem Wert (ohne Beschriftung,
    // damit Suchen nach Beschriftungen eindeutig bleiben)
    resp.widget_info(|| {
        let mut i = egui::WidgetInfo::new(egui::WidgetType::ComboBox);
        i.enabled = enabled;
        i.current_text_value = Some(text.to_string());
        i
    });
    audit(ui, "field", "", r, r, 0);
    let open = egui::Popup::is_id_open(ui.ctx(), egui::Popup::default_response_id(&resp));
    let hot = enabled && (resp.hovered() || open);
    ui.painter().rect_filled(r, 0.0, if enabled { c.field_bg } else { c.frame_bg });
    ui.painter().rect_stroke(r, 0.0, Stroke::new(1.0, if hot { c.button_hover_border } else if enabled { c.field_border } else { c.line_light }), StrokeKind::Inside);
    let ar = Rect::from_min_max(pos2(r.right() - 16.0, r.top()), r.max);
    if hot {
        ui.painter().rect_filled(ar.shrink(1.0), 0.0, c.button_hover_bg);
    }
    glyphs::chevron_down(ui.painter(), ar, if enabled { c.icon_fg } else { c.icon_disabled });
    let col = if enabled { c.field_text } else { c.text_disabled };
    crate::widgets::text_clipped(ui.painter(), pos2(r.left() + 4.0, r.center().y), Align2::LEFT_CENTER, text, t.ui_font(), col, r.width() - 22.0);
    resp
}

/// Layout-Protokoll der Einträge eines Kombifelds (nur bei [`set_audit`]): jeder Eintrag muss in
/// das geschlossene Feld passen (Text links, Chevron rechts).
pub fn audit_combo_items(ui: &Ui, r: Rect, items: &[&str]) {
    if !ui.ctx().data(|d| d.get_temp::<bool>(Id::new(AUDIT_ON))).unwrap_or(false) {
        return;
    }
    let avail = Rect::from_min_max(pos2(r.left() + 4.0, r.top()), pos2(r.right() - 18.0, r.bottom()));
    let font = Theme::current(ui.ctx()).ui_font();
    for it in items.iter().filter(|s| !is_separator(s)) {
        let w = text_w(ui, it, &font);
        let need = Rect::from_min_size(avail.min, vec2(w, avail.height()));
        audit(ui, "combo-item", it, avail, need, if w > avail.width() + 0.5 { 2 } else { 0 });
    }
}

/// Kombifeld mit fester Liste. `selected` ist der Index in `items` (Trenner zählen mit, sind aber
/// nicht wählbar). Liefert den gewählten Index.
pub fn combo(ui: &mut Ui, id: impl Hash + std::fmt::Debug, r: Rect, items: &[&str], selected: Option<usize>, enabled: bool) -> Option<usize> {
    let id = Id::new(id);
    let text = selected.and_then(|i| items.get(i)).copied().unwrap_or("");
    let resp = combo_box_frame(ui, id, r, text, enabled);
    let mut out = None;
    let t = Theme::current(ui.ctx());
    audit_combo_items(ui, r, items);
    egui::Popup::menu(&resp).frame(crate::menu::menu_frame(&t)).width(r.width().max(60.0)).show(|ui| {
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
            for (i, it) in items.iter().enumerate() {
                if is_separator(it) {
                    let (sr, _) = ui.allocate_exact_size(vec2(ui.available_width(), 7.0), Sense::hover());
                    ui.painter().hline(sr.x_range().shrink(4.0), sr.center().y.floor() + 0.5, Stroke::new(1.0, t.colors.menu_separator));
                    continue;
                }
                if list_item(ui, it, Some(i) == selected).clicked() {
                    out = Some(i);
                    ui.close();
                }
            }
        });
    });
    out
}

/// Eintrag einer Aufklappliste.
pub fn list_item(ui: &mut Ui, text: &str, selected: bool) -> Response {
    let t = Theme::current(ui.ctx());
    let c = &t.colors;
    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width().max(60.0), 18.0), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(r, 0.0, c.menu_hover_bg);
    } else if selected {
        ui.painter().rect_filled(r, 0.0, c.list_selected_bg);
    }
    let col = if resp.hovered() { c.menu_hover_text } else { c.menu_text };
    ui.painter().text(pos2(r.left() + 6.0, r.center().y), Align2::LEFT_CENTER, text, t.ui_font(), col);
    resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, selected, text));
    resp
}

/// Kombifeld mit Eingabe (ComboDropDown, z. B. Hyperlink): Text frei, Liste zur Auswahl.
pub fn combo_edit(ui: &mut Ui, id: impl Hash + std::fmt::Debug, r: Rect, current: &str, items: &[String], enabled: bool) -> Option<String> {
    let id = Id::new(id);
    let er = Rect::from_min_max(r.min, pos2(r.right() - 16.0, r.bottom()));
    let mut out = buffered_edit(ui, id, er, current, enabled, false, false).0;
    let ar = Rect::from_min_max(pos2(r.right() - 17.0, r.top()), r.max);
    let t = Theme::current(ui.ctx());
    let resp = ui.interact(ar, id.with("arrow"), if enabled { Sense::click() } else { Sense::hover() });
    audit(ui, "field", "", ar, ar, 0);
    ui.painter().rect_filled(ar, 0.0, if enabled { t.colors.field_bg } else { t.colors.frame_bg });
    ui.painter().rect_stroke(ar, 0.0, Stroke::new(1.0, if enabled { t.colors.field_border } else { t.colors.line_light }), StrokeKind::Inside);
    glyphs::chevron_down(ui.painter(), ar, if enabled { t.colors.icon_fg } else { t.colors.icon_disabled });
    egui::Popup::menu(&resp).frame(crate::menu::menu_frame(&t)).width(r.width()).show(|ui| {
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        if items.is_empty() {
            ui.label(egui::RichText::new(fp_i18n::t("pi-no-entries")).color(t.colors.menu_text_disabled));
        }
        for it in items {
            if list_item(ui, it, it == current).clicked() {
                out = Some(it.clone());
                ui.close();
            }
        }
    });
    out
}

/// Symbol-Umschalter (Füllart, Strichposition, Pfadoperationen, Ausrichtung …).
pub fn toggle(ui: &mut Ui, id: impl Hash + std::fmt::Debug, r: Rect, glyph: Glyph, on: bool, enabled: bool, tip: &str) -> bool {
    let t = Theme::current(ui.ctx());
    let c = &t.colors;
    let resp = ui.interact(r, Id::new(id), if enabled { Sense::click() } else { Sense::hover() });
    audit(ui, "field", "", r, r, 0);
    let hot = enabled && resp.hovered();
    if on {
        if t.glow {
            ui.painter().rect_filled(r, 2.0, crate::theme::with_alpha(c.focus, 40));
            ui.painter().rect_stroke(r, 2.0, Stroke::new(1.0, c.focus), StrokeKind::Inside);
        } else {
            // gedrückt wie im Original (UX-A W2, a_116): blauer Rand mit hellblauem Innenrand –
            // vorher kaum von „aus“ zu unterscheiden (weiß statt hellgrau)
            ui.painter().rect_filled(r, 2.0, c.tool_selected_bg);
            ui.painter().rect_stroke(r.shrink(1.0), 1.0, Stroke::new(2.0, crate::theme::with_alpha(c.focus, 60)), StrokeKind::Inside);
            ui.painter().rect_stroke(r, 2.0, Stroke::new(1.0, c.focus), StrokeKind::Inside);
        }
    } else {
        ui.painter().rect_filled(r, 2.0, if hot { c.button_hover_bg } else { c.button_bg });
        ui.painter().rect_stroke(r, 2.0, Stroke::new(1.0, if hot { c.button_hover_border } else { c.button_border }), StrokeKind::Inside);
    }
    glyphs::paint(ui.painter(), r, glyph, &t, enabled);
    let clicked = enabled && resp.clicked();
    if !tip.is_empty() {
        resp.on_hover_text(tip);
    }
    clicked
}

/// Flacher Symbolknopf (Stil-Knöpfe, Filter + / −).
pub fn icon_button(ui: &mut Ui, id: impl Hash + std::fmt::Debug, r: Rect, glyph: Glyph, enabled: bool, tip: &str) -> Response {
    let t = Theme::current(ui.ctx());
    let c = &t.colors;
    let resp = ui.interact(r, Id::new(id), if enabled { Sense::click() } else { Sense::hover() });
    audit(ui, "field", "", r, r, 0);
    if enabled && resp.hovered() {
        ui.painter().rect_filled(r, 2.0, c.button_hover_bg);
        ui.painter().rect_stroke(r, 2.0, Stroke::new(1.0, c.button_hover_border), StrokeKind::Inside);
    }
    glyphs::paint(ui.painter(), r, glyph, &t, enabled);
    if tip.is_empty() { resp } else { resp.on_hover_text(tip) }
}

/// Knopf mit Text (Windows-Stil).
pub fn button(ui: &mut Ui, id: impl Hash + std::fmt::Debug, r: Rect, text: &str, enabled: bool) -> bool {
    let t = Theme::current(ui.ctx());
    let c = &t.colors;
    let resp = ui.interact(r, Id::new(id), if enabled { Sense::click() } else { Sense::hover() });
    let hot = enabled && resp.hovered();
    let pressed = enabled && resp.is_pointer_button_down_on();
    let bg = if pressed { c.button_pressed_bg } else if hot { c.button_hover_bg } else { c.button_bg };
    ui.painter().rect_filled(r, 2.0, bg);
    ui.painter().rect_stroke(r, 2.0, Stroke::new(1.0, if hot { c.button_hover_border } else { c.button_border }), StrokeKind::Inside);
    let col = if enabled { c.text } else { c.text_disabled };
    audit(ui, "field", "", r, r, 0);
    if fit_on(ui) {
        fitted_text(ui, "button", r.shrink2(vec2(3.0, 0.0)), r.center(), Align2::CENTER_CENTER, text, t.ui_font(), col);
    } else {
        let drawn = crate::widgets::text_clipped(ui.painter(), r.center(), Align2::CENTER_CENTER, text, t.ui_font(), col, r.width() - 6.0);
        audit(ui, "button", text, r.shrink2(vec2(3.0, 0.0)), drawn, 0);
    }
    enabled && resp.clicked()
}

/// Kontrollkästchen mit Beschriftung; liefert den neuen Zustand.
pub fn checkbox(ui: &mut Ui, id: impl Hash + std::fmt::Debug, r: Rect, text: &str, checked: bool, enabled: bool) -> Option<bool> {
    let t = Theme::current(ui.ctx());
    let c = &t.colors;
    let resp = ui.interact(r, Id::new(id), if enabled { Sense::click() } else { Sense::hover() });
    // Barrierefreiheit (und egui_kittest/`ui_find`): Kästchen mit Beschriftung und Zustand
    resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, enabled, checked, text));
    let bx = Rect::from_min_size(pos2(r.left(), r.center().y - 6.5), vec2(13.0, 13.0));
    ui.painter().rect_filled(bx, 0.0, if enabled { c.field_bg } else { c.frame_bg });
    ui.painter().rect_stroke(bx, 0.0, Stroke::new(1.0, if enabled && resp.hovered() { c.focus } else if enabled { c.field_border } else { c.line_light }), StrokeKind::Inside);
    if checked {
        let col = if enabled { c.field_text } else { c.text_disabled };
        ui.painter().line(vec![pos2(bx.left() + 3.0, bx.center().y), pos2(bx.left() + 5.5, bx.bottom() - 3.5), pos2(bx.right() - 3.0, bx.top() + 3.5)], Stroke::new(1.6, col));
    }
    audit(ui, "field", "", bx, bx, 0);
    if !text.is_empty() {
        let avail = Rect::from_min_max(pos2(bx.right() + 5.0, r.top()), pos2(r.right().max(bx.right() + 5.0), r.bottom()));
        fitted_text(ui, "check", avail, pos2(bx.right() + 5.0, r.center().y), Align2::LEFT_CENTER, text, t.ui_font(), if enabled { c.text } else { c.text_disabled });
    }
    (enabled && resp.clicked()).then_some(!checked)
}

/// Optionsfeld (rund) mit Beschriftung; liefert `true` beim Anklicken.
pub fn radio(ui: &mut Ui, id: impl Hash + std::fmt::Debug, r: Rect, text: &str, on: bool, enabled: bool) -> bool {
    let t = Theme::current(ui.ctx());
    let c = &t.colors;
    let resp = ui.interact(r, Id::new(id), if enabled { Sense::click() } else { Sense::hover() });
    resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::RadioButton, enabled, on, text));
    let center = pos2(r.left() + 6.5, r.center().y);
    ui.painter().circle_filled(center, 6.0, if enabled { c.field_bg } else { c.frame_bg });
    ui.painter().circle_stroke(center, 6.0, Stroke::new(1.0, if enabled && resp.hovered() { c.focus } else { c.field_border }));
    if on {
        ui.painter().circle_filled(center, 3.0, if enabled { c.field_text } else { c.text_disabled });
    }
    if !text.is_empty() {
        let avail = Rect::from_min_max(pos2(r.left() + 17.0, r.top()), pos2(r.right().max(r.left() + 17.0), r.bottom()));
        fitted_text(ui, "radio", avail, pos2(r.left() + 17.0, r.center().y), Align2::LEFT_CENTER, text, t.ui_font(), if enabled { c.text } else { c.text_disabled });
    }
    enabled && resp.clicked()
}

/// Farbfeld mit Aufklapp-Dreieck (für Popups); `None` = keine Farbe (roter Schrägstrich).
pub fn swatch(ui: &mut Ui, id: impl Hash + std::fmt::Debug, r: Rect, color: Option<fp_color::Argb>, enabled: bool) -> Response {
    let t = Theme::current(ui.ctx());
    let resp = ui.interact(r, Id::new(id), if enabled { Sense::click() } else { Sense::hover() });
    audit(ui, "field", "", r, r, 0);
    let col = match color {
        Some(c) => Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), c.a()),
        None => Color32::TRANSPARENT,
    };
    crate::widgets::color_well(ui.painter(), r, col, &t);
    if enabled && resp.hovered() {
        ui.painter().rect_stroke(r, 0.0, Stroke::new(1.0, t.colors.focus), StrokeKind::Inside);
    }
    resp
}

/// Senkrechte Trennlinie (zwischen Teilfeldern im Quadranten, z. B. Füllung | Strich).
pub fn vsep(ui: &Ui, x: f32, y0: f32, y1: f32) {
    let t = Theme::current(ui.ctx());
    ui.painter().vline(x.floor() + 0.5, y0..=y1, Stroke::new(1.0, t.colors.line_mid));
    ui.painter().vline(x.floor() + 1.5, y0..=y1, Stroke::new(1.0, t.colors.line_light));
}

/// Liste mit Rahmen (Filterliste, Listen in Dialogen); liefert das Innenrechteck.
pub fn list_box(ui: &Ui, r: Rect) -> Rect {
    let t = Theme::current(ui.ctx());
    ui.painter().rect_filled(r, 0.0, t.colors.list_bg);
    ui.painter().rect_stroke(r, 0.0, Stroke::new(1.0, t.colors.list_border), StrokeKind::Inside);
    r.shrink(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_parse_and_format() {
        assert_eq!(parse_num("12,5"), Some(12.5));
        assert_eq!(parse_num(" 40 px"), Some(40.0));
        assert_eq!(parse_num("-3"), Some(-3.0));
        assert_eq!(parse_num("abc"), None);
        assert_eq!(parse_digits("-5"), Some(5.0));
        assert_eq!(parse_digits("12,5"), Some(125.0));
        assert_eq!(parse_digits("x"), None);
        assert_eq!(fmt_num(4.10, 1), "4,1");
        assert_eq!(fmt_num(100.0, 0), "100");
        assert_eq!(clamp_range(150.0, (0.0, 100.0)), 100.0);
        assert!(is_separator("-----"));
        assert!(!is_separator("Normal"));
    }
}
