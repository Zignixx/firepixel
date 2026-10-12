//! Tonwertkorrektur (19111, re\19-kurven-tonwert.md §3, KV-2) nach dem Aufbau des Originals: Kanal-Liste im
//! Gruppenrahmen, drei Eingabefelder über dem Histogramm, Eingabeschieber mit drei geordneten
//! Griffen, zwei Ausgabefelder, Verlaufsbalken und Ausgabeschieber mit zwei kreuzbaren Griffen;
//! rechts OK, Abbrechen, Auto, drei Pipetten und „Vorschau“.
//!
//! Die Rechenregeln (Gamma-Suchen, Histogramm, Auto, Pipetten, Schieber) stehen in
//! [`fp_raster::filters::levels_math`]; gemeinsam mit dem Kurvendialog sind Vorschau-Schalter,
//! Kanal-Histogramme und Pipetten ([`super::levels_common`]).

use super::form::{apply_preview, revert_preview, FormDialog};
use super::levels_common::{self as common, Pipette, CHANNELS};
use super::{dialog_frame, push_button, refs, t, tl, DialogClose};
use crate::app_state::AppState;
use crate::inspector::fields;
use crate::theme::Theme;
use egui::{pos2, vec2, Color32, Id, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, Ui};
use fp_raster::filters::levels_math as lm;
use fp_raster::filters::{LevelsChannel, LevelsParams};
use serde_json::json;

/// Kennung des Dialogfensters.
pub const DIALOG_ID: &str = "fp-levels";

/// Innerer Abstand der Schieberspur links und rechts (§3.5).
const TRACK_INSET: f32 = 5.0;
/// Halbe Griffbreite (Griff `x − 5 … x + 5`).
const HANDLE_HALF: f32 = 5.0;

/// Welcher Schieber gezogen wird.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slider {
    Input,
    Output,
}

/// Zustand der Tonwertkorrektur.
#[derive(Clone, Debug)]
pub struct LevelsDialog {
    pub params: LevelsParams,
    /// 0 = RGB, 1…3 = Rot, Grün, Blau (Start immer RGB).
    pub channel: usize,
    pub preview: FormDialog,
    /// Kanalhistogramme R, G, B des Originalbilds (beim Öffnen, vor jeder Vorschau).
    pub hist: [[u64; 256]; 3],
    /// Angezeigtes Histogramm (bei RGB die Summe nach den Kanaltabellen; neu beim Kanalwechsel).
    pub shown: [u64; 256],
    pub live: bool,
    /// Aktive Pipette (Aufnahmemodus): der nächste Klick auf die Leinwand nimmt die angezeigte Farbe.
    pub picking: Option<Pipette>,
    /// Gezogener Griff.
    pub drag: Option<(Slider, usize)>,
}

impl LevelsDialog {
    /// Neuer Dialog: Grundwerte 0/1/255 und 0/255 in allen Kanälen, nichts gemerkt außer dem
    /// gemeinsamen Vorschau-Schalter (§3.10).
    pub fn new(st: &AppState, live: bool) -> LevelsDialog {
        let base = if live { json!({"live": true}) } else { json!({}) };
        let mut preview = FormDialog::new("19111", &t("dlg-levels-title"), "filter.adjustColor.levels", vec![]).with_base(base).with_preview();
        preview.preview_on = common::preview_pref(st);
        let hist = common::channel_histograms(st);
        let params = LevelsParams::default();
        LevelsDialog { params, channel: 0, preview, hist, shown: lm::display_histogram(&hist, &params, 0), live, picking: None, drag: None }
    }

    /// Parametersatz des gewählten Kanals.
    pub fn channel_ref(&self) -> &LevelsChannel {
        match self.channel {
            1 => &self.params.red,
            2 => &self.params.green,
            3 => &self.params.blue,
            _ => &self.params.rgb,
        }
    }

    fn channel_mut(&mut self) -> &mut LevelsChannel {
        match self.channel {
            1 => &mut self.params.red,
            2 => &mut self.params.green,
            3 => &mut self.params.blue,
            _ => &mut self.params.rgb,
        }
    }

    /// Kanal wechseln: Felder, Griffe und Histogramm zeigen den neuen Kanal (§3.7). Das
    /// RGB-Histogramm wird dabei aus den aktuellen Kanaltabellen neu summiert (§3.3).
    pub fn set_channel(&mut self, c: usize) {
        self.channel = c.min(3);
        self.shown = lm::display_histogram(&self.hist, &self.params, self.channel);
    }

    /// „Auto“: Kanalsätze auf die 0,5-%-Grenzen, RGB bleibt; der Dialog bleibt offen (§3.8).
    pub fn auto(&mut self) {
        lm::auto_levels_from(&mut self.params, &self.hist);
    }

    /// Pipette mit der aufgenommenen (angezeigten) Farbe anwenden (§3.9).
    pub fn pick(&mut self, p: Pipette, color: [u8; 3]) {
        lm::apply_pick(&mut self.params, p, color);
        self.picking = None;
    }

    /// Griff `idx` eines Schiebers auf den Wert `v` (0…255) ziehen (Regeln §3.5/§3.6).
    pub fn drag_handle(&mut self, slider: Slider, idx: usize, v: f64) {
        let c = self.channel_mut();
        match slider {
            Slider::Input => lm::apply_input_drag(c, idx, v),
            Slider::Output => lm::apply_output_drag(c, idx, v),
        }
    }

    /// Feldwert übernehmen (§3.4): 0 = Schwarz, 1 = Gamma, 2 = Weiß, 3 = Ausgabe unten,
    /// 4 = Ausgabe oben. Begrenzung wie im Original; die Ausgabe hat keine Ordnung.
    pub fn set_field(&mut self, field: usize, v: f64) {
        let c = self.channel_mut();
        let i = v.round() as i32;
        match field {
            0 => c.source_low = i.clamp(0, c.source_high - 2),
            1 => c.gamma = if v.is_nan() { 1.0 } else { v.clamp(0.1, 10.0) },
            2 => c.source_high = i.clamp(c.source_low + 2, 255),
            3 => c.dest_low = i.clamp(0, 255),
            _ => c.dest_high = i.clamp(0, 255),
        }
    }

    /// Befehlsparameter.
    pub fn command_params(&self) -> serde_json::Value {
        let mut v = serde_json::to_value(self.params).unwrap_or(json!({}));
        if self.live {
            v["live"] = json!(true);
        }
        v
    }
}

/// Wert eines Schiebers aus der Zeigerlage (Spur 5 px eingerückt, begrenzt auf 0…255).
pub fn slider_value(track: Rect, x: f32) -> f64 {
    (((x - track.left() - TRACK_INSET) / (track.width() - 2.0 * TRACK_INSET)) as f64 * 255.0).clamp(0.0, 255.0)
}

/// Bildschirmlage eines Schieberwerts.
pub fn slider_x(track: Rect, v: f64) -> f32 {
    track.left() + TRACK_INSET + (v / 255.0) as f32 * (track.width() - 2.0 * TRACK_INSET)
}

/// Griff unter `x`: nur die Griffe selbst treffen, die Spur daneben nicht (§3.5); bei Überlappung
/// gewinnt der erste in Listenreihenfolge.
pub fn handle_at(track: Rect, values: &[f64], x: f32) -> Option<usize> {
    values.iter().position(|v| (x - slider_x(track, *v)).abs() <= HANDLE_HALF)
}

/// Bildschirmlage der Spur eines Schiebers im letzten Bild (für Tests und die Brücke).
pub fn track_rect(ctx: &egui::Context, which: Slider) -> Option<Rect> {
    ctx.data(|m| m.get_temp::<Rect>(Id::new(("lv-track", which as u8))))
}

/// Dreieckiger Griff (11 px breit, 6 px hoch, Spitze oben).
fn handle(ui: &Ui, x: f32, top: f32, fill: Color32) {
    let x = x.round() + 0.5;
    ui.painter().add(Shape::convex_polygon(vec![pos2(x, top), pos2(x + HANDLE_HALF, top + 6.0), pos2(x - HANDLE_HALF, top + 6.0)], fill, Stroke::new(1.0, Color32::BLACK)));
}

/// Histogramm wie im Original: schwarz auf Hellgrau, Maßstab Mittel der 5 größten Klassen,
/// höhere Säulen abgeschnitten, sichtbare Säule `h − 1` (§3.3).
fn draw_histogram(ui: &Ui, outer: Rect, hist: &[u64; 256]) {
    let t = Theme::current(ui.ctx());
    ui.painter().rect_stroke(outer, 0.0, Stroke::new(1.0, t.colors.line_mid), StrokeKind::Inside);
    let client = outer.shrink(1.0);
    ui.painter().rect_filled(client, 0.0, Color32::from_gray(240));
    let heights = lm::histogram_heights(hist, client.width().round() as usize, client.height().round() as u32);
    for (x, h) in heights.iter().enumerate() {
        let vis = h.saturating_sub(1) as f32;
        if vis > 0.0 {
            let l = client.left() + x as f32;
            ui.painter().rect_filled(Rect::from_min_max(pos2(l, client.bottom() - vis), pos2(l + 1.0, client.bottom())), 0.0, Color32::BLACK);
        }
    }
}

/// Waagrechter Verlauf schwarz → weiß (nur Anzeige).
fn draw_ramp(ui: &Ui, r: Rect) {
    let mut mesh = egui::epaint::Mesh::default();
    mesh.colored_vertex(r.left_top(), Color32::BLACK);
    mesh.colored_vertex(r.right_top(), Color32::WHITE);
    mesh.colored_vertex(r.left_bottom(), Color32::BLACK);
    mesh.colored_vertex(r.right_bottom(), Color32::WHITE);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 2, 3);
    ui.painter().add(Shape::mesh(mesh));
}

/// Schieber mit Griffen; liefert (Griff, Wert) während des Ziehens.
#[allow(clippy::too_many_arguments)]
fn slider(ui: &mut Ui, d: &mut LevelsDialog, which: Slider, track: Rect, values: &[f64], fills: &[Color32], top: f32, tip: &str) -> Option<(usize, f64)> {
    ui.ctx().data_mut(|m| m.insert_temp(Id::new(("lv-track", which as u8)), track));
    let resp = ui.interact(track, Id::new(("lv-slider", which as u8)), Sense::drag());
    for (v, f) in values.iter().zip(fills) {
        handle(ui, slider_x(track, *v), top, *f);
    }
    if resp.drag_started() {
        let origin = ui.ctx().input(|i| i.pointer.press_origin()).or(resp.interact_pointer_pos());
        d.drag = origin.and_then(|p| handle_at(track, values, p.x)).map(|i| (which, i));
    }
    let out = match (d.drag, resp.interact_pointer_pos()) {
        (Some((s, i)), Some(p)) if s == which && resp.dragged() => Some((i, slider_value(track, p.x))),
        _ => None,
    };
    if resp.drag_stopped() && d.drag.is_some_and(|(s, _)| s == which) {
        d.drag = None;
    }
    if !tip.is_empty() {
        resp.on_hover_text(tip);
    }
    out
}

/// Zeigt die Tonwertkorrektur.
pub fn show_levels(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut d) = st.dialogs.levels.clone() else { return };
    // Esc im Aufnahmemodus beendet nur die Aufnahme (§1.3)
    if d.picking.is_some() && common::consume_escape(ctx) {
        d.picking = None;
    }
    let mut result = None;
    let channels = tl(&CHANNELS);
    let (ch_tip, min_tip, gamma_tip, max_tip, auto_tip) = (t("dlg-adjust-channel-tip"), t("dlg-levels-min-tip"), t("dlg-levels-gamma-tip"), t("dlg-levels-max-tip"), t("dlg-levels-auto-tip"));
    let (ch_l, in_l, out_l, auto_l, prev_l, ok_l, cancel_l) = (t("dlg-channel"), t("dlg-levels-input"), t("dlg-levels-output"), t("dlg-levels-auto"), t("dlg-preview"), t("btn-ok"), t("btn-cancel"));
    // Lage nach §3.2 (Client-Koordinaten des Originals, 393 × 263); breitere Knopftexte
    // verbreitern nur die Knopfspalte
    let bw = super::buttons_w(ctx, &[&ok_l, &cancel_l, &auto_l], 98.0);
    let pw = (18.0 + super::text_w(ctx, &prev_l)).max(108.0);
    let width = (287.0 + bw.max(pw - 2.0) + 8.0).max(393.0f32).ceil();
    let mut preview_toggled = None;
    let r = dialog_frame(ctx, DIALOG_ID, &t("dlg-levels-title"), vec2(width, 263.0), |ui, body| {
        let o = body.min;
        let at = |x: f32, y: f32, w: f32, h: f32| Rect::from_min_size(o + vec2(x, y), vec2(w, h));
        // Gruppenrahmen mit Kanal-Liste in der Überschrift
        super::group_box(ui, at(8.0, 10.0, 270.0, 245.0), "");
        let cw = super::combo_fit_w(ui.ctx(), &refs(&channels), 143.0);
        let lw = super::text_w(ui.ctx(), &ch_l);
        let cx = 86.0f32.max(16.0 + lw + 6.0);
        let cover = Theme::current(ui.ctx()).colors.frame_bg;
        ui.painter().rect_filled(at(14.0, 7.0, cx + cw - 10.0, 21.0), 0.0, cover);
        super::fit_label(ui, at(cx - 6.0 - lw, 7.0, lw + 1.0, 21.0), &ch_l, true);
        let cr = at(cx, 7.0, cw, 21.0);
        if let Some(c) = fields::combo(ui, "lv-ch", cr, &refs(&channels), Some(d.channel), true) {
            d.set_channel(c);
        }
        let _ = ui.interact(cr, Id::new("lv-ch-tip"), Sense::hover()).on_hover_text(&ch_tip);
        let ch = *d.channel_ref();
        // Eingabetonwerte: Schwarz, Gamma, Weiß
        crate::inspector::fields::label_right(ui, at(11.0, 34.0, 135.0, 21.0), &in_l, true);
        for (k, (x, v, rng, dec)) in [(152.0, ch.source_low as f64, (0.0, 255.0), 0), (192.0, ch.gamma, (0.1, 10.0), 2), (233.0, ch.source_high as f64, (0.0, 255.0), 0)].into_iter().enumerate() {
            let fr = at(x, 34.0, 36.0, 21.0);
            if let Some(nv) = fields::number(ui, ("lv-in", k), fr, None, Some(v), rng, dec, true) {
                d.set_field(k, nv);
            }
            let _ = ui.interact(fr, Id::new(("lv-in-tip", k)), Sense::hover()).on_hover_text([&min_tip, &gamma_tip, &max_tip][k].as_str());
        }
        // Histogramm
        draw_histogram(ui, at(21.0, 60.0, 241.0, 111.0), &d.shown);
        // Eingabeschieber (drei Griffe, geordnet)
        let track = at(17.0, 174.0, 249.0, 13.0);
        let ch = *d.channel_ref();
        let hv = lm::input_handles(&ch);
        if let Some((i, v)) = slider(ui, &mut d, Slider::Input, track, &hv, &[Color32::BLACK, Color32::from_gray(128), Color32::WHITE], track.top() + 1.0, &t("dlg-levels-input-tip")) {
            d.drag_handle(Slider::Input, i, v);
        }
        // Ausgabetonwerte
        crate::inspector::fields::label_right(ui, at(5.0, 192.0, 182.0, 21.0), &out_l, true);
        let ch = *d.channel_ref();
        for (k, (x, v)) in [(191.0, ch.dest_low), (231.0, ch.dest_high)].into_iter().enumerate() {
            let fr = at(x, 192.0, 36.0, 21.0);
            if let Some(nv) = fields::number(ui, ("lv-out", k), fr, None, Some(v as f64), (0.0, 255.0), 0, true) {
                d.set_field(3 + k, nv);
            }
            let _ = ui.interact(fr, Id::new(("lv-out-tip", k)), Sense::hover()).on_hover_text(if k == 0 { min_tip.as_str() } else { max_tip.as_str() });
        }
        draw_ramp(ui, at(21.0, 218.0, 241.0, 13.0));
        let otrack = at(17.0, 232.0, 249.0, 13.0);
        let ch = *d.channel_ref();
        if let Some((i, v)) = slider(ui, &mut d, Slider::Output, otrack, &[ch.dest_low as f64, ch.dest_high as f64], &[Color32::BLACK, Color32::WHITE], otrack.top(), &t("dlg-levels-output-tip")) {
            d.drag_handle(Slider::Output, i, v);
        }
        // Knopfspalte
        if push_button(ui, at(287.0, 16.0, bw, 23.0), &ok_l, true, true) {
            result = Some(true);
        }
        if push_button(ui, at(287.0, 46.0, bw, 23.0), &cancel_l, false, true) {
            result = Some(false);
        }
        let ar = at(287.0, 75.0, bw, 23.0);
        if push_button(ui, ar, &auto_l, false, true) {
            d.auto();
        }
        let _ = ui.interact(ar, Id::new("lv-auto-tip"), Sense::hover()).on_hover_text(&auto_tip);
        let px = 287.0 + (bw - 72.0) / 2.0;
        if let Some(p) = common::pipette_buttons(ui, "lv", [o.x + px, o.x + px + 24.0, o.x + px + 48.0], o.y + 109.0, d.picking) {
            d.picking = if d.picking == Some(p) { None } else { Some(p) };
        }
        if let Some(b) = fields::checkbox(ui, "lv-prev", at(285.0, 141.0, pw, 16.0), &prev_l, d.preview.preview_on, true) {
            d.preview.preview_on = b;
            preview_toggled = Some(b);
        }
    });
    // Vorschau-Schalter sofort app-weit merken, auch wenn danach abgebrochen wird (§1.2)
    if let Some(b) = preview_toggled {
        common::set_preview_pref(st, b);
    }
    // Aufnahmemodus: ein Klick außerhalb des Dialogs nimmt die angezeigte Farbe der Leinwand; der
    // modale Hintergrund meldet diesen Klick sonst als „schließen“
    let mut close = r == DialogClose::Cancelled;
    if let Some(p) = d.picking
        && let Some(rect) = super::dialog_rect(ctx, DIALOG_ID)
        && let Some(pos) = common::outside_click(ctx, rect)
    {
        close = false;
        pick_at(st, &mut d, p, pos);
    }
    let params = d.command_params();
    finish(st, &mut d.preview, params, result.or(if close { Some(false) } else { None }));
    if result.is_some() || close {
        st.dialogs.levels = None;
    } else {
        st.dialogs.levels = Some(d);
    }
}

/// Farbe an der Bildschirmstelle aufnehmen (angezeigte Farbe, mit Vorschau schon korrigiert).
/// Außerhalb der Leinwand bleibt die Pipette aktiv.
fn pick_at(st: &AppState, d: &mut LevelsDialog, p: Pipette, pos: Pos2) {
    if let Some(c) = common::displayed_color_at(st, pos) {
        d.pick(p, c);
    }
}

/// Vorschau und Abschluss. Anders als bei den Kurven rechnet die Tonwertkorrektur die Vorschau bei
/// jeder Wertänderung neu, auch während des Ziehens (§3.5). OK = genau ein Verlaufsschritt,
/// Abbrechen nimmt die Vorschau zurück und hinterlässt keinen Schritt.
fn finish(st: &mut AppState, pv: &mut FormDialog, params: serde_json::Value, result: Option<bool>) {
    pv.base = params.clone();
    match result {
        Some(true) => {
            if !(pv.applied.is_some() && pv.applied_params.as_ref() == Some(&params)) {
                revert_preview(st, pv);
                st.run(&pv.command.clone(), params);
            }
        }
        Some(false) => revert_preview(st, pv),
        None => {
            if pv.preview_on {
                apply_preview(st, pv);
            } else {
                revert_preview(st, pv);
            }
        }
    }
}
