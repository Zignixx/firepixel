//! Pfad-Dialoge mit der Anordnung des Originals (PM-6, P-DLG7, Bilder `pm6_D29…D32`):
//! Vereinfachen (6006, D29: „Menge:“), Pfad einlassen (6007, D31: Richtung untereinander),
//! Strich erweitern (6008, D30: „Winkelgrenze“ mit zwei Nachkommastellen, „Endkappen“) und
//! „Pfad in Auswahlrahmen konvertieren“ (D32: „Rand:“ Hart/Anti-Alias/Weiche Kante + Breite,
//! Breite nur bei weicher Kante, Startfokus OK).
//!
//! Beschriftungen rechtsbündig vor den Feldern, Symbolknöpfe als **ein** Tab-Stopp (Pfeiltasten
//! wählen), OK/Abbrechen unten mittig. Werte, Voreinstellungen und OK bleiben die des Formulars.

use super::form::{CustomLayout, Field, FieldKind, FormDialog, Pict, number_input, paint_pict};
use super::{t, text_w};
use crate::app_state::AppState;
use crate::inspector::fields;
use crate::theme::Theme;
use crate::tips::HoverTip;
use egui::{Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use serde_json::json;

/// Rand des Client-Bereichs.
const M: f32 = 14.0;
/// Zeile „Beschriftung: Feld“.
const ROW: f32 = 26.0;
/// Zeile mit Symbolknöpfen.
const ICON_ROW: f32 = 28.0;
/// Zeile je untereinander stehendem Optionsfeld.
const RADIO_ROW: f32 = 22.0;
/// Abstand vor den abgesetzten Zeilen (Original: Leerraum vor Ecken, Endkappen, Breite).
const GAP: f32 = 8.0;
/// Symbolknopf.
const ICON: egui::Vec2 = egui::Vec2::new(24.0, 22.0);
const ICON_STEP: f32 = 28.0;

/// Werte der Kante in D32.
pub const EDGES: [&str; 3] = ["hard", "antiAlias", "feather"];

/// Doppelpfeil ↔ bzw. ↕ (Symbole der Felder in „Numerisch transformieren“, D26).
pub(crate) fn paint_double_arrow(ui: &Ui, r: Rect, horizontal: bool, col: egui::Color32) {
    let p = ui.painter();
    let s = Stroke::new(1.0, col);
    let (a, b) = if horizontal {
        (pos2(r.left(), r.center().y), pos2(r.right(), r.center().y))
    } else {
        (pos2(r.center().x, r.top()), pos2(r.center().x, r.bottom()))
    };
    p.line_segment([a, b], s);
    for (tip, dir) in [(a, 1.0), (b, -1.0)] {
        if horizontal {
            p.line_segment([tip, pos2(tip.x + 3.0 * dir, tip.y - 3.0)], s);
            p.line_segment([tip, pos2(tip.x + 3.0 * dir, tip.y + 3.0)], s);
        } else {
            p.line_segment([tip, pos2(tip.x - 3.0, tip.y + 3.0 * dir)], s);
            p.line_segment([tip, pos2(tip.x + 3.0, tip.y + 3.0 * dir)], s);
        }
    }
}

/// Ecken als Symbolknöpfe in der Reihenfolge des Originals (Gehrung, abgeschrägt, rund); Codes der
/// Voreinstellungen `InsetJoinType`/`ExpandJoinType`: 0 = rund (Vorgabe), 1 = abgeschrägt, 2 = Gehrung.
fn join_field() -> Field {
    Field::radio_of(
        "join",
        t("dlg-lbl-corners").as_str(),
        &[
            t("dlg-join-miter").as_str(),
            t("dlg-join-bevel").as_str(),
            t("dlg-join-round").as_str(),
        ],
        vec![json!("miter"), json!("bevel"), json!("round")],
        2,
    )
    .icons(&[Pict::JoinMiter, Pict::JoinBevel, Pict::JoinRound])
    .pref_codes(&[2, 1, 0])
}

/// Vereinfachen (D29).
pub fn simplify(cmd: &str) -> FormDialog {
    FormDialog::new(
        "6006",
        t("dlg-title-simplify").as_str(),
        cmd,
        vec![
            Field::number(
                "amount",
                t("dlg-lbl-simplify-amount").as_str(),
                10.0,
                0.0,
                100.0,
            )
            .popup(),
        ],
    )
    .with_prefs(&[("amount", "SimplifyAmount")])
    .with_layout(PATH_LAYOUT)
}

/// Pfad einlassen (D31): Richtung untereinander, Breite ▾, Ecken, Winkelgrenze 10.00.
pub fn inset(cmd: &str) -> FormDialog {
    FormDialog::new(
        "6007",
        t("dlg-title-inset-path").as_str(),
        cmd,
        vec![
            Field::radio_of(
                "direction",
                t("dlg-lbl-direction").as_str(),
                &[t("dlg-dir-inside").as_str(), t("dlg-dir-outside").as_str()],
                vec![json!("inside"), json!("outside")],
                0,
            )
            .pref_codes(&[0, 1])
            .vertical(),
            Field::number("distance", t("dlg-lbl-width").as_str(), 10.0, 0.0, 1000.0).popup(),
            join_field(),
            Field::number("miter", t("dlg-lbl-miter-limit").as_str(), 10.0, 4.0, 10.0)
                .decimals(2)
                .dot_decimal(),
        ],
    )
    .with_prefs(&[
        ("direction", "InsetDirection"),
        ("distance", "InsetAmount"),
        ("join", "InsetJoinType"),
        ("miter", "InsetMiterLimit"),
    ])
    .with_layout(PATH_LAYOUT)
}

/// Strich erweitern (D30): Breite ▾, Ecken, Winkelgrenze 10.00, Endkappen.
pub fn expand_stroke(cmd: &str) -> FormDialog {
    FormDialog::new(
        "6008",
        t("dlg-title-expand-stroke").as_str(),
        cmd,
        vec![
            Field::number("width", t("dlg-lbl-width").as_str(), 10.0, 1.0, 1000.0).popup(),
            join_field(),
            Field::number("miter", t("dlg-lbl-miter-limit").as_str(), 10.0, 1.0, 10.0)
                .decimals(2)
                .dot_decimal(),
            Field::radio_of(
                "cap",
                t("dlg-lbl-caps").as_str(),
                &[
                    t("dlg-cap-butt").as_str(),
                    t("dlg-cap-square").as_str(),
                    t("dlg-cap-round").as_str(),
                ],
                vec![json!("butt"), json!("square"), json!("round")],
                0,
            )
            .icons(&[Pict::CapButt, Pict::CapSquare, Pict::CapRound])
            .pref_codes(&[0, 1, 2]),
        ],
    )
    .with_prefs(&[
        ("width", "ExpandWidth"),
        ("join", "ExpandJoinType"),
        ("miter", "ExpandMiterLimit"),
        ("cap", "ExpandCapType"),
    ])
    .with_layout(PATH_LAYOUT)
}

/// Pfad in Auswahlrahmen konvertieren (D32): „Rand:“ mit Anti-Alias als Vorgabe, Breite der
/// weichen Kante (grau außer bei „Weiche Kante“), Startfokus OK; Werte nach OK gemerkt.
pub fn to_marquee(cmd: &str) -> FormDialog {
    let items = [
        t("pi-edge-hard"),
        t("pi-edge-antialias"),
        t("pi-edge-feather"),
    ];
    FormDialog::new(
        "path-to-marquee",
        t("dlg-title-path-to-marquee").as_str(),
        cmd,
        vec![
            Field::choice_of(
                "edge",
                t("pi-edge").as_str(),
                &[items[0].as_str(), items[1].as_str(), items[2].as_str()],
                EDGES.iter().map(|e| json!(e)).collect(),
                1,
            )
            .pref_codes(&[0, 1, 2]),
            Field::number("feather", "", 0.0, 0.0, 1000.0).popup(),
        ],
    )
    .with_prefs(&[
        ("edge", "PathToMarqueeEdge"),
        ("feather", "PathToMarqueeFeather"),
    ])
    .with_focus_ok()
    .with_layout(MARQUEE_LAYOUT)
}

// ---------------------------------------------------------------------- D29–D31

fn row_h(f: &Field) -> f32 {
    match &f.kind {
        FieldKind::Radio(_) if f.icons.is_some() => ICON_ROW,
        FieldKind::Radio(items) if f.vertical => RADIO_ROW * items.len() as f32 + 2.0,
        _ => ROW,
    }
}

fn gap_before(i: usize, f: &Field) -> f32 {
    if i > 0 && matches!(f.key.as_str(), "join" | "cap" | "distance") {
        GAP
    } else {
        0.0
    }
}

fn ctrl_w(ctx: &egui::Context, f: &Field) -> f32 {
    match &f.kind {
        FieldKind::Radio(items) if f.icons.is_some() => {
            items.len() as f32 * ICON_STEP - (ICON_STEP - ICON.x)
        }
        FieldKind::Radio(items) => items
            .iter()
            .map(|s| text_w(ctx, s) + 26.0)
            .fold(0.0, f32::max),
        FieldKind::Number { .. } => 50.0 + if f.popup { 16.0 } else { 0.0 },
        _ => 80.0,
    }
}

fn label_w(ctx: &egui::Context, d: &FormDialog) -> f32 {
    d.fields
        .iter()
        .map(|f| text_w(ctx, &f.label))
        .fold(0.0, f32::max)
        .ceil()
}

fn ok_cancel(ctx: &egui::Context) -> (String, String, f32) {
    let (ok, cancel) = (t("btn-ok"), t("btn-cancel"));
    let w = super::buttons_w(ctx, &[&ok, &cancel], 76.0);
    (ok, cancel, w)
}

fn path_size(ctx: &egui::Context, d: &FormDialog) -> egui::Vec2 {
    let cw = d.fields.iter().map(|f| ctrl_w(ctx, f)).fold(0.0, f32::max);
    let (_, _, bw) = ok_cancel(ctx);
    let w = (2.0 * M + label_w(ctx, d) + 6.0 + cw + 24.0)
        .max(2.0 * M + 2.0 * bw + 8.0)
        .max(200.0);
    let rows: f32 = d
        .fields
        .iter()
        .enumerate()
        .map(|(i, f)| gap_before(i, f) + row_h(f))
        .sum();
    vec2(w.ceil(), (M + rows + 14.0 + 23.0 + M).ceil())
}

fn path_draw(ui: &mut Ui, _st: &AppState, d: &mut FormDialog, body: Rect) -> (bool, [Rect; 2]) {
    let ctx = ui.ctx().clone();
    let lw = label_w(&ctx, d);
    let cw = d.fields.iter().map(|f| ctrl_w(&ctx, f)).fold(0.0, f32::max);
    // Block aus Beschriftungsspalte und Feldern mittig wie im Original
    let x0 = body.left() + ((body.width() - (lw + 6.0 + cw)) / 2.0).floor().max(M);
    let lx = x0 + lw;
    let cx = lx + 6.0;
    let mut y = body.top() + M;
    let id = d.id.clone();
    let mut changed = false;
    for i in 0..d.fields.len() {
        let f = &mut d.fields[i];
        y += gap_before(i, f);
        let h = row_h(f);
        let lh = if f.icons.is_some() { ICON.y } else { 20.0 };
        let tw = text_w(&ctx, &f.label);
        fields::label(
            ui,
            Rect::from_min_size(pos2(lx - tw, y), vec2(tw + 1.0, lh)),
            &f.label,
            true,
        );
        match f.kind.clone() {
            FieldKind::Number { decimals, .. } if decimals > 0 && f.dot => {
                changed |= fixed_input(
                    ui,
                    &id,
                    i,
                    f,
                    Rect::from_min_size(pos2(cx, y), vec2(50.0, 20.0)),
                    decimals,
                )
            }
            FieldKind::Number { .. } => {
                changed |= number_input(
                    ui,
                    &id,
                    i,
                    f,
                    Rect::from_min_size(pos2(cx, y), vec2(50.0, 20.0)),
                    true,
                )
            }
            FieldKind::Radio(_) if f.icons.is_some() => {
                changed |= icon_group(ui, &id, i, f, pos2(cx, y))
            }
            FieldKind::Radio(items) => {
                let rects: Vec<Rect> = items
                    .iter()
                    .enumerate()
                    .map(|(k, it)| {
                        Rect::from_min_size(
                            pos2(cx, y + 2.0 + k as f32 * RADIO_ROW),
                            vec2(text_w(&ctx, it) + 26.0, 16.0),
                        )
                    })
                    .collect();
                let labels: Vec<&str> = items.iter().map(|s| s.as_str()).collect();
                // D31: Startfokus im Feld „Breite“, nicht auf der Richtung davor (PARITAET D31, CT-A-08)
                let af = super::autofocus_id(ui.layer_id());
                let held = if f.key == "direction" {
                    ui.ctx().data_mut(|d| d.remove_temp::<u64>(af))
                } else {
                    None
                };
                if let Some(k) = super::radio_group(
                    ui,
                    (id.as_str(), i, "radio"),
                    &rects,
                    &labels,
                    f.value.as_u64().unwrap_or(0) as usize,
                    true,
                ) {
                    f.value = json!(k);
                    changed = true;
                }
                if let Some(p) = held {
                    ui.ctx().data_mut(|d| d.insert_temp(af, p));
                }
            }
            _ => {}
        }
        y += h;
    }
    let (_, _, bw) = ok_cancel(&ctx);
    let top = body.bottom() - M - 23.0;
    let x = body.center().x - bw - 4.0;
    (
        changed,
        [
            Rect::from_min_size(pos2(x, top), vec2(bw, 23.0)),
            Rect::from_min_size(pos2(x + bw + 8.0, top), vec2(bw, 23.0)),
        ],
    )
}

const PATH_LAYOUT: CustomLayout = CustomLayout {
    size: path_size,
    draw: path_draw,
};

/// Zahlenfeld mit fester Zahl Nachkommastellen und Dezimalpunkt („10.00“ wie im Original).
fn fixed_input(
    ui: &mut Ui,
    dlg_id: &str,
    i: usize,
    f: &mut Field,
    er: Rect,
    decimals: usize,
) -> bool {
    let v = f.value.as_f64().unwrap_or(0.0);
    let txt = format!("{v:.decimals$}");
    let Some(s) = fields::text(ui, (dlg_id, i), er, &txt, true) else {
        if f.error {
            ui.painter().rect_stroke(
                er,
                0.0,
                Stroke::new(1.5, Theme::current(ui.ctx()).colors.icon_red),
                StrokeKind::Inside,
            );
        }
        return false;
    };
    f.set_number_text(&s);
    true
}

/// Symbolknöpfe (Ecken, Endkappen) als **ein** Tab-Stopp wie im Original: nur der gewählte Knopf
/// ist fokussierbar, Pfeiltasten wählen innerhalb der Gruppe; Text als Tooltip und Name.
fn icon_group(ui: &mut Ui, dlg_id: &str, i: usize, f: &mut Field, at: egui::Pos2) -> bool {
    let FieldKind::Radio(items) = f.kind.clone() else {
        return false;
    };
    let icons = f.icons.clone().unwrap_or_default();
    let n = items.len().min(icons.len());
    if n == 0 {
        return false;
    }
    let sel = (f.value.as_u64().unwrap_or(0) as usize).min(n - 1);
    let c = Theme::current(ui.ctx()).colors.clone();
    let ids: Vec<egui::Id> = (0..n)
        .map(|k| egui::Id::new((dlg_id, i, k, "icon")))
        .collect();
    let mut out = None;
    for k in 0..n {
        let br = Rect::from_min_size(pos2(at.x + k as f32 * ICON_STEP, at.y), ICON);
        let sense = if k == sel {
            Sense::click()
        } else {
            Sense::CLICK
        };
        let resp = ui.interact(br, ids[k], sense).hover_tip(items[k].as_str());
        resp.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::RadioButton,
                true,
                k == sel,
                items[k].as_str(),
            )
        });
        let on = k == sel;
        ui.painter().rect_filled(
            br,
            2.0,
            if on {
                c.button_pressed_bg
            } else if resp.hovered() {
                c.button_hover_bg
            } else {
                c.field_bg
            },
        );
        ui.painter().rect_stroke(
            br,
            2.0,
            Stroke::new(1.0, if on { c.focus } else { c.field_border }),
            StrokeKind::Inside,
        );
        paint_pict(ui.painter(), br, icons[k], c.icon_fg);
        if on && ui.memory(|m| m.has_focus(ids[k])) {
            ui.painter().rect_stroke(
                br.shrink(2.0),
                0.0,
                Stroke::new(1.0, c.focus),
                StrokeKind::Inside,
            );
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(
                    ids[k],
                    egui::EventFilter {
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        tab: false,
                        escape: false,
                    },
                )
            });
            let step = ui.input_mut(|inp| {
                let mut s = 0i32;
                for (key, dlt) in [
                    (egui::Key::ArrowDown, 1),
                    (egui::Key::ArrowRight, 1),
                    (egui::Key::ArrowUp, -1),
                    (egui::Key::ArrowLeft, -1),
                ] {
                    if inp.consume_key(egui::Modifiers::NONE, key) {
                        s += dlt;
                    }
                }
                s
            });
            if step != 0 {
                out = Some((sel as i32 + step).rem_euclid(n as i32) as usize);
            }
        }
        if resp.clicked() {
            out = Some(k);
        }
    }
    match out {
        Some(k) => {
            ui.memory_mut(|m| m.request_focus(ids[k]));
            let ch = k != sel;
            f.value = json!(k);
            ch
        }
        None => false,
    }
}

// ---------------------------------------------------------------------- D32

fn marquee_widths(ctx: &egui::Context, d: &FormDialog) -> (f32, f32, f32) {
    let lw = text_w(ctx, &d.fields[0].label).ceil();
    let items: Vec<&str> = match &d.fields[0].kind {
        FieldKind::Choice(v) => v.iter().map(|s| s.as_str()).collect(),
        _ => Vec::new(),
    };
    let cw = super::combo_fit_w(ctx, &items, 110.0);
    let (_, _, bw) = ok_cancel(ctx);
    (lw, cw, bw)
}

fn marquee_size(ctx: &egui::Context, d: &FormDialog) -> egui::Vec2 {
    let (lw, cw, bw) = marquee_widths(ctx, d);
    let w = 2.0 * M + 24.0 + lw + 6.0 + cw + 8.0 + 34.0 + 15.0 + 24.0 + bw;
    vec2(w.ceil(), (M + 23.0 + 6.0 + 23.0 + M).ceil())
}

fn marquee_draw(ui: &mut Ui, _st: &AppState, d: &mut FormDialog, body: Rect) -> (bool, [Rect; 2]) {
    let ctx = ui.ctx().clone();
    let (lw, cw, bw) = marquee_widths(&ctx, d);
    let id = d.id.clone();
    let y = body.top() + M + 1.0;
    let lx = body.left() + M + 24.0;
    let mut changed = false;
    fields::label(
        ui,
        Rect::from_min_size(pos2(lx, y), vec2(lw + 1.0, 20.0)),
        &d.fields[0].label,
        true,
    );
    let items: Vec<String> = match &d.fields[0].kind {
        FieldKind::Choice(v) => v.clone(),
        _ => Vec::new(),
    };
    let refs: Vec<&str> = items.iter().map(|s| s.as_str()).collect();
    let cr = Rect::from_min_size(pos2(lx + lw + 6.0, y), vec2(cw, 20.0));
    let sel = d.fields[0].value.as_u64().map(|v| v as usize);
    if let Some(k) = fields::combo(ui, (id.as_str(), 0usize), cr, &refs, sel, true) {
        d.fields[0].value = json!(k);
        changed = true;
    }
    let feather_on = d.fields[0].value.as_u64() == Some(2);
    let er = Rect::from_min_size(pos2(cr.right() + 8.0, y), vec2(34.0, 20.0));
    changed |= number_input(ui, &id, 1, &mut d.fields[1], er, feather_on);
    let x = body.right() - M - bw;
    (
        changed,
        [
            Rect::from_min_size(pos2(x, body.top() + M), vec2(bw, 23.0)),
            Rect::from_min_size(pos2(x, body.top() + M + 29.0), vec2(bw, 23.0)),
        ],
    )
}

const MARQUEE_LAYOUT: CustomLayout = CustomLayout {
    size: marquee_size,
    draw: marquee_draw,
};
