//! Auswahl-Dialoge in der Anordnung des Originals (P-DLG8, PM-6 D35–D39): Weiche Kante (Vorlage
//! 5976) und Auswahlrahmen erweitern/verkleinern/begrenzen/glätten (19105–19108).
//!
//! Alle fünf haben dieselbe Form: Beschriftung rechtsbündig vor einem schmalen Zahlenfeld, „Pixel“
//! dahinter, OK und Abbrechen rechts untereinander. Lage und Größe in Dialogeinheiten aus
//! `dialoge.json`; ist eine Beschriftung in einer anderen Sprache länger als in der Vorlage, rücken
//! Feld, Einheit und Knöpfe nach rechts und der Dialog wird breiter.

use super::form::{number_input, unit_text, CustomLayout, FieldKind, FormDialog};
use super::{dlu, dlu_rect, t, text_w};
use crate::app_state::AppState;
use crate::inspector::fields;
use egui::{pos2, vec2, Rect, Ui};

/// Vorlage eines Auswahl-Dialogs (alle Angaben in DLU).
struct Template {
    id: &'static str,
    /// Client-Bereich.
    size: [f32; 2],
    /// Beschriftung `[x, y, b, h]` (rechtsbündig bis 4 DLU vor dem Feld).
    label: [f32; 4],
    /// Zahlenfeld.
    edit: [f32; 4],
    /// Einheit „Pixel“.
    unit: [f32; 4],
    /// Linke Kante von OK (oben, y 7) und Abbrechen (y 24), je 50 × 14.
    buttons_x: f32,
}

const TEMPLATES: [Template; 5] = [
    Template { id: "5976", size: [205.0, 46.0], label: [3.0, 18.0, 56.0, 8.0], edit: [64.0, 16.0, 19.0, 12.0], unit: [87.0, 18.0, 57.0, 8.0], buttons_x: 146.0 },
    Template { id: "19105", size: [214.0, 46.0], label: [3.0, 18.0, 75.0, 8.0], edit: [82.0, 16.0, 19.0, 12.0], unit: [105.0, 18.0, 49.0, 8.0], buttons_x: 156.0 },
    Template { id: "19106", size: [214.0, 46.0], label: [4.0, 18.0, 66.0, 8.0], edit: [73.0, 16.0, 19.0, 12.0], unit: [96.0, 18.0, 57.0, 8.0], buttons_x: 156.0 },
    Template { id: "19107", size: [187.0, 46.0], label: [3.0, 18.0, 52.0, 8.0], edit: [60.0, 16.0, 19.0, 12.0], unit: [83.0, 18.0, 42.0, 8.0], buttons_x: 130.0 },
    Template { id: "19108", size: [226.0, 50.0], label: [3.0, 18.0, 85.0, 8.0], edit: [92.0, 16.0, 19.0, 12.0], unit: [115.0, 18.0, 50.0, 8.0], buttons_x: 168.0 },
];

fn template(d: &FormDialog) -> &'static Template {
    TEMPLATES.iter().find(|x| x.id == d.id).unwrap_or(&TEMPLATES[0])
}

/// Beschriftung und Einheit des (einzigen) Zahlenfelds.
fn texts(d: &FormDialog) -> (String, String) {
    match d.fields.first() {
        Some(f) => (f.label.clone(), if let FieldKind::Number { unit, .. } = &f.kind { unit_text(unit) } else { String::new() }),
        None => (String::new(), String::new()),
    }
}

/// OK und Abbrechen: gemeinsame Breite (mindestens 50 DLU wie in den Vorlagen).
fn ok_cancel_w(ctx: &egui::Context) -> f32 {
    let (ok, cancel) = (t("btn-ok"), t("btn-cancel"));
    super::buttons_w(ctx, &[&ok, &cancel], dlu(50.0, 0.0).x)
}

/// Verschiebung von Feld und Einheit (längere Beschriftung) und linke Kante der Knöpfe.
fn shifts(ctx: &egui::Context, d: &FormDialog) -> (f32, f32) {
    let tp = template(d);
    let (label, unit) = texts(d);
    // Platz bis 4 DLU vor dem Feld (die Beschriftung steht rechtsbündig davor)
    let room = dlu(tp.edit[0] - 4.0 - tp.label[0], 0.0).x;
    let dx = (text_w(ctx, &label) - room).max(0.0).ceil();
    let unit_end = dlu(tp.unit[0], 0.0).x + dx + text_w(ctx, &unit);
    let bx = (dlu(tp.buttons_x, 0.0).x + dx).max(unit_end + dlu(4.0, 0.0).x).ceil();
    (dx, bx)
}

fn size(ctx: &egui::Context, d: &FormDialog) -> egui::Vec2 {
    let tp = template(d);
    let (_, bx) = shifts(ctx, d);
    let right = dlu(tp.size[0] - tp.buttons_x - 50.0, 0.0).x;
    vec2(dlu(tp.size[0], 0.0).x.max(bx + ok_cancel_w(ctx) + right).ceil(), dlu(0.0, tp.size[1]).y.ceil())
}

fn draw(ui: &mut Ui, _st: &AppState, d: &mut FormDialog, body: Rect) -> (bool, [Rect; 2]) {
    let tp = template(d);
    let o = body.min;
    let (dx, bx) = shifts(ui.ctx(), d);
    let (label, unit) = texts(d);
    let er = dlu_rect(o, tp.edit).translate(vec2(dx, 0.0));
    let lr = Rect::from_min_max(pos2(o.x + dlu(tp.label[0], 0.0).x, er.top()), pos2(er.left() - dlu(4.0, 0.0).x, er.bottom()));
    if text_w(ui.ctx(), &label) > lr.width() + 0.5 {
        super::report_overflow(ui.ctx(), format!("Beschriftung „{label}“ breiter als {:.0} px", lr.width()));
    }
    fields::label_right(ui, lr, &label, true);
    let id = d.id.clone();
    let changed = d.fields.first_mut().is_some_and(|f| number_input(ui, &id, 0, f, er, true));
    if !unit.is_empty() {
        let ux = o.x + dlu(tp.unit[0], 0.0).x + dx;
        fields::label(ui, Rect::from_min_max(pos2(ux, er.top()), pos2(ux + text_w(ui.ctx(), &unit) + 1.0, er.bottom())), &unit, true);
    }
    let (bw, h) = (ok_cancel_w(ui.ctx()), dlu(0.0, 14.0).y);
    let x = o.x + bx;
    (changed, [Rect::from_min_size(pos2(x, o.y + dlu(0.0, 7.0).y), vec2(bw, h)), Rect::from_min_size(pos2(x, o.y + dlu(0.0, 24.0).y), vec2(bw, h))])
}

/// Anordnung der Auswahl-Dialoge 5976 und 19105–19108.
pub const LAYOUT: CustomLayout = CustomLayout { size, draw };
