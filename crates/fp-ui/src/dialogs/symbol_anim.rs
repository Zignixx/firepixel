//! Anordnung von „In Symbol konvertieren“ (19104) und „Animieren“ (19146) nach der Vorlage des
//! Originals (PM-6, PARITAET_B §2 D27/D28, P-DLG9). Felder, Werte, Prüfung und OK bleiben die des
//! Formulars aus [`super::catalog`]; hier stehen nur Lage und Größe in Dialogeinheiten aus
//! `dialoge.json`. Längere Texte anderer Sprachen schieben die Spalten nach rechts und verbreitern
//! den Dialog.

use super::form::{CustomLayout, FormDialog, number_input};
use super::{dlu, dlu_rect, t, text_w};
use crate::app_state::AppState;
use crate::inspector::fields;
use crate::theme::Theme;
use egui::{Align2, Rect, Stroke, Ui, pos2, vec2};
use serde_json::json;

fn field_at(d: &FormDialog, key: &str) -> usize {
    d.fields
        .iter()
        .position(|f| f.key == key)
        .unwrap_or(usize::MAX)
}

fn label_of(d: &FormDialog, key: &str) -> String {
    d.fields
        .get(field_at(d, key))
        .map(|f| f.label.replace('&', ""))
        .unwrap_or_default()
}

/// OK und Abbrechen: gemeinsame Breite (mindestens 50 DLU wie in den Vorlagen).
fn ok_cancel_w(ctx: &egui::Context) -> f32 {
    let (ok, cancel) = (t("btn-ok"), t("btn-cancel"));
    super::buttons_w(ctx, &[&ok, &cancel], dlu(50.0, 0.0).x)
}

/// Beschriftung rechtsbündig bis `right` (wie `SS_RIGHT` der Vorlage), mittig zu `row`.
fn right_label(ui: &Ui, right: f32, row: Rect, text: &str) {
    let theme = Theme::current(ui.ctx());
    ui.painter().text(
        pos2(right, row.center().y),
        Align2::RIGHT_CENTER,
        text,
        theme.ui_font(),
        theme.colors.text,
    );
}

/// Beschriftung linksbündig ab `left`, mittig zu `row`.
fn left_label(ui: &Ui, left: f32, row: Rect, text: &str) {
    let theme = Theme::current(ui.ctx());
    ui.painter().text(
        pos2(left, row.center().y),
        Align2::LEFT_CENTER,
        text,
        theme.ui_font(),
        theme.colors.text,
    );
}

fn number_at(ui: &mut Ui, d: &mut FormDialog, key: &str, r: Rect) -> bool {
    let i = field_at(d, key);
    let id = d.id.clone();
    d.fields
        .get_mut(i)
        .is_some_and(|f| number_input(ui, &id, i, f, r, true))
}

fn check_at(ui: &mut Ui, d: &mut FormDialog, key: &str, r: Rect) -> bool {
    let i = field_at(d, key);
    let id = d.id.clone();
    let Some(f) = d.fields.get_mut(i) else {
        return false;
    };
    // gesperrte Kästchen (D14 „In allgemeiner Bibliothek speichern“) grau, kein Tab-Stopp
    if let Some(b) = fields::checkbox(
        ui,
        (id.as_str(), i),
        r,
        &f.label,
        f.value.as_bool().unwrap_or(false),
        !f.disabled,
    ) {
        f.value = json!(b);
        return true;
    }
    false
}

/// Optionsfelder `key` an den Rechtecken `rects` (eine Gruppe = ein Tab-Stopp); `&` in den Texten
/// ist die Zugriffstaste (S-4, D14 G/A/S).
fn radios_at(ui: &mut Ui, d: &mut FormDialog, key: &str, rects: &[Rect]) -> bool {
    let i = field_at(d, key);
    let id = d.id.clone();
    let Some(f) = d.fields.get_mut(i) else {
        return false;
    };
    let super::form::FieldKind::Radio(items) = &f.kind else {
        return false;
    };
    let labels: Vec<&str> = items.iter().map(|s| s.as_str()).collect();
    let sel = f.value.as_u64().unwrap_or(0) as usize;
    if let Some(k) = super::access::radio(ui, (id.as_str(), i, "radio"), rects, &labels, sel, true)
    {
        f.value = json!(k);
        return true;
    }
    false
}

fn radio_items(d: &FormDialog, key: &str) -> Vec<String> {
    match d.fields.get(field_at(d, key)).map(|f| &f.kind) {
        Some(super::form::FieldKind::Radio(items)) => items.clone(),
        _ => Vec::new(),
    }
}

/// Breite eines Optionsfelds bzw. Kästchens mit Text (Kreis/Kästchen + Abstand + Text).
fn option_w(ctx: &egui::Context, text: &str) -> f32 {
    18.0 + text_w(ctx, text) + 8.0
}

fn etched_h(ui: &Ui, x0: f32, x1: f32, y: f32) {
    let theme = Theme::current(ui.ctx());
    ui.painter().hline(
        x0..=x1,
        y.round() + 0.5,
        Stroke::new(1.0, theme.colors.line_mid),
    );
}

// ------------------------------------------------- In Symbol konvertieren (19104)
//
// 326 × 102 DLU. Senkrechte Trennlinie [68,8,1,76] (links davon die im Original leere Vorschau);
// „Name:“ rechtsbündig bis 124, Feld [127,9,157,12]; „Typ:“ bis 124, Grafik/Animation/Schaltfläche
// untereinander [127,31] [127,43] [127,55]; OK [259,29,50,14], Abbrechen [259,48,50,14] rechts;
// „Optionen:“ bis 124 mit 9-Slice [128,72] und „In allgemeiner Bibliothek speichern“ [128,88].

/// Schlüssel des 9-Slice-Kästchens: „In Symbol konvertieren“ (`_nineslice`, über die Umformung)
/// bzw. „Neues Symbol“ (`nineSlice`, direkt Befehlsparameter; D14 hat dieselbe Vorlage).
fn nine_key(d: &FormDialog) -> &'static str {
    if d.fields.iter().any(|f| f.key == "nineSlice") {
        "nineSlice"
    } else {
        "_nineslice"
    }
}

/// Verschiebung der Spalte ab 127 DLU, wenn eine Beschriftung nicht vor die Felder passt.
fn convert_dx(ctx: &egui::Context, d: &FormDialog) -> f32 {
    let room = dlu(124.0 - 72.0, 0.0).x;
    let w = [
        label_of(d, "name"),
        label_of(d, "kind"),
        t("dlg-lbl-options"),
    ]
    .iter()
    .map(|l| text_w(ctx, l))
    .fold(0.0, f32::max);
    (w - room).max(0.0).ceil()
}

fn convert_size(ctx: &egui::Context, d: &FormDialog) -> egui::Vec2 {
    let dx = convert_dx(ctx, d);
    let checks = dlu(128.0 + 9.0, 0.0).x
        + dx
        + option_w(ctx, &label_of(d, nine_key(d))).max(option_w(ctx, &label_of(d, "_common")));
    let radios = radio_items(d, "kind")
        .iter()
        .map(|s| option_w(ctx, s))
        .fold(0.0, f32::max);
    let buttons = dlu(127.0 + 6.0, 0.0).x + dx + radios + ok_cancel_w(ctx) + dlu(17.0, 0.0).x;
    vec2(
        (dlu(326.0, 0.0).x + dx).max(checks).max(buttons).ceil(),
        dlu(0.0, 102.0).y.ceil(),
    )
}

fn convert_draw(ui: &mut Ui, _st: &AppState, d: &mut FormDialog, body: Rect) -> (bool, [Rect; 2]) {
    let o = body.min;
    let dx = convert_dx(ui.ctx(), d);
    let sh = |r: Rect| r.translate(vec2(dx, 0.0));
    let label_right = o.x + dlu(124.0, 0.0).x + dx;
    let mut changed = false;
    // senkrechte Trennlinie rechts der (leeren) Vorschau
    let line = dlu_rect(o, [68.0, 8.0, 1.0, 76.0]);
    let theme = Theme::current(ui.ctx());
    ui.painter().vline(
        line.left().round() + 0.5,
        line.top()..=line.bottom(),
        Stroke::new(1.0, theme.colors.line_mid),
    );
    let bw = ok_cancel_w(ui.ctx());
    // Name
    let nr = sh(dlu_rect(o, [127.0, 9.0, 157.0, 12.0]));
    let ok_x = body.right() - dlu(17.0, 0.0).x - bw;
    let nr = nr.with_max_x(nr.right().min(ok_x + bw));
    right_label(ui, label_right, nr, &label_of(d, "name"));
    {
        let i = field_at(d, "name");
        let id = d.id.clone();
        // re\32 D14: Alt+N → Namensfeld, Inhalt markiert
        if let Some(f) = d.fields.get(i) {
            let raw = f.label.clone();
            super::access::underline(
                ui,
                label_right - text_w(ui.ctx(), &raw),
                nr.center().y,
                &raw,
                true,
            );
            if super::access::pressed(ui, &raw) {
                super::access::focus_field(
                    ui,
                    (id.as_str(), i),
                    f.value.as_str().unwrap_or("").chars().count(),
                );
            }
        }
        if let Some(f) = d.fields.get_mut(i)
            && let Some(v) = fields::text(
                ui,
                (id.as_str(), i),
                nr,
                f.value.as_str().unwrap_or(""),
                true,
            )
        {
            f.value = json!(v);
            changed = true;
        }
    }
    // Typ untereinander
    let items = radio_items(d, "kind");
    let rects: Vec<Rect> = items
        .iter()
        .enumerate()
        .map(|(k, s)| {
            let r = sh(dlu_rect(o, [127.0, 31.0 + 12.0 * k as f32, 91.0, 10.0]));
            Rect::from_min_size(r.min, vec2(option_w(ui.ctx(), s), r.height()))
        })
        .collect();
    if let Some(first) = rects.first() {
        right_label(ui, label_right, *first, &label_of(d, "kind"));
    }
    changed |= radios_at(ui, d, "kind", &rects);
    // Tab-Folge wie im Original: Name › Typ › OK › Abbrechen › 9-Slice › Bibliothek (PM-6 D27, D14)
    let h = dlu(0.0, 14.0).y;
    let ok = Rect::from_min_size(pos2(ok_x, o.y + dlu(0.0, 29.0).y), vec2(bw, h));
    let cancel = Rect::from_min_size(pos2(ok_x, o.y + dlu(0.0, 48.0).y), vec2(bw, h));
    super::form::ok_cancel(ui, d, [ok, cancel]);
    // Optionen
    let nine = nine_key(d);
    let c1 = sh(dlu_rect(o, [128.0, 72.0, 125.0, 12.0]));
    let c1 = Rect::from_min_size(
        c1.min,
        vec2(option_w(ui.ctx(), &label_of(d, nine)), c1.height()),
    );
    let c2 = sh(dlu_rect(o, [128.0, 88.0, 122.0, 11.0]));
    let c2 = Rect::from_min_size(
        c2.min,
        vec2(option_w(ui.ctx(), &label_of(d, "_common")), c2.height()),
    );
    right_label(ui, label_right, c1, &t("dlg-lbl-options"));
    changed |= check_at(ui, d, nine, c1);
    changed |= check_at(ui, d, "_common", c2);
    (changed, [ok, cancel])
}

/// Anordnung von „In Symbol konvertieren“ (19104).
pub const CONVERT_LAYOUT: CustomLayout = CustomLayout {
    size: convert_size,
    draw: convert_draw,
};

// ------------------------------------------------- Animieren (19146)
//
// 215 × 154 DLU (ohne den im Original ausgeblendeten Knopf „Bearbeiten…“: 215 × 152). Beschriftungen
// rechtsbündig bis 55, Felder [59,y,23,12] mit Popup-Schieber ▾ daneben: Status y 8, Trennlinie
// [14,27,183,1], Verschieben 37, Richtung 54, Skalieren auf 71, Deckkraft 88 mit „bis“ [97,90] und
// zweitem Feld [114,88], Drehen 105 mit „Nach rechts“ [98,105] und „Nach links“ [160,105] in derselben
// Zeile; Trennlinie [4,124,183,1]; OK [82,135,50,14], Abbrechen [138,135,50,14]. Startfokus OK.

const ANIM_ROWS: [(&str, f32); 6] = [
    ("frames", 8.0),
    ("_dist", 37.0),
    ("_dir", 54.0),
    ("scale", 71.0),
    ("startOpacity", 88.0),
    ("_rot", 105.0),
];

fn anim_dx(ctx: &egui::Context, d: &FormDialog) -> f32 {
    let w = ANIM_ROWS
        .iter()
        .map(|(k, _)| text_w(ctx, &label_of(d, k)))
        .fold(0.0, f32::max);
    (w - dlu(53.0, 0.0).x).max(0.0).ceil()
}

/// Linke Kante von „bis“, zweitem Deckkraftfeld und den Drehrichtungen (relativ, ohne `dx`).
fn anim_right_parts(ctx: &egui::Context, d: &FormDialog) -> (f32, f32, f32, f32) {
    let to_x = dlu(97.0, 0.0).x;
    let field2_x =
        (to_x + text_w(ctx, &label_of(d, "endOpacity")) + dlu(4.0, 0.0).x).max(dlu(114.0, 0.0).x);
    let items = radio_items(d, "_cw");
    let cw_x = dlu(98.0, 0.0).x;
    let ccw_x =
        (cw_x + option_w(ctx, items.first().map(String::as_str).unwrap_or("")) + dlu(4.0, 0.0).x)
            .max(dlu(160.0, 0.0).x);
    let end = (ccw_x + option_w(ctx, items.get(1).map(String::as_str).unwrap_or("")))
        .max(field2_x + dlu(23.0 + 10.0, 0.0).x);
    (field2_x, cw_x, ccw_x, end)
}

fn anim_size(ctx: &egui::Context, d: &FormDialog) -> egui::Vec2 {
    let dx = anim_dx(ctx, d);
    let (_, _, _, end) = anim_right_parts(ctx, d);
    let buttons = dlu(82.0, 0.0).x + 2.0 * ok_cancel_w(ctx) + dlu(6.0 + 27.0, 0.0).x;
    vec2(
        (dlu(215.0, 0.0).x + dx)
            .max(end + dx + dlu(7.0, 0.0).x)
            .max(buttons + dx)
            .ceil(),
        dlu(0.0, 152.0).y.ceil(),
    )
}

fn anim_draw(ui: &mut Ui, _st: &AppState, d: &mut FormDialog, body: Rect) -> (bool, [Rect; 2]) {
    let o = body.min;
    let ctx = ui.ctx().clone();
    let dx = anim_dx(&ctx, d);
    let (field2_x, cw_x, ccw_x, _) = anim_right_parts(&ctx, d);
    let label_right = o.x + dlu(55.0, 0.0).x + dx;
    let mut changed = false;
    // Tab-Folge wie im Original: OK › Abbrechen › Felder (PM-6 D28); Startfokus OK
    let bw = ok_cancel_w(&ctx);
    let top = o.y + dlu(0.0, 135.0).y;
    let h = dlu(0.0, 14.0).y;
    let cancel = Rect::from_min_size(pos2(body.right() - dlu(27.0, 0.0).x - bw, top), vec2(bw, h));
    let ok = Rect::from_min_size(pos2(cancel.left() - dlu(6.0, 0.0).x - bw, top), vec2(bw, h));
    super::form::ok_cancel(ui, d, [ok, cancel]);
    for (key, y) in ANIM_ROWS {
        let er = dlu_rect(o, [59.0, y, 23.0, 12.0]).translate(vec2(dx, 0.0));
        right_label(ui, label_right, er, &label_of(d, key));
        changed |= number_at(ui, d, key, er);
        if key == "startOpacity" {
            left_label(
                ui,
                o.x + dlu(97.0, 0.0).x + dx,
                er,
                &label_of(d, "endOpacity"),
            );
            let er2 = Rect::from_min_size(pos2(o.x + field2_x + dx, er.top()), er.size());
            changed |= number_at(ui, d, "endOpacity", er2);
        }
        if key == "_rot" {
            let items = radio_items(d, "_cw");
            let rects: Vec<Rect> = [cw_x, ccw_x]
                .iter()
                .zip(items.iter())
                .map(|(x, s)| {
                    Rect::from_min_size(
                        pos2(o.x + x + dx, er.top() - 1.0),
                        vec2(option_w(&ctx, s), dlu(0.0, 10.0).y),
                    )
                })
                .collect();
            changed |= radios_at(ui, d, "_cw", &rects);
        }
    }
    let x1 = body.right() - dlu(11.0, 0.0).x;
    etched_h(ui, o.x + dlu(14.0, 0.0).x, x1, o.y + dlu(0.0, 27.0).y);
    etched_h(ui, o.x + dlu(4.0, 0.0).x, x1, o.y + dlu(0.0, 124.0).y);
    (changed, [ok, cancel])
}

/// Anordnung von „Animieren“ (19146).
pub const ANIMATE_LAYOUT: CustomLayout = CustomLayout {
    size: anim_size,
    draw: anim_draw,
};
