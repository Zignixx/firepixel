//! Dokument-Dialoge mit eigener Anordnung: Leinwandgröße (5989, Anker im 3×3-Raster), Bildgröße
//! (6011, Pixelabmessungen/Druckgröße mit Einheiten, Proportionen, Neuauflösung) und
//! „Numerisch transformieren“ (6002). Lage der Steuerelemente in DLU wie in `dialoge.json`.

use super::{combo_fit_w, dialog_frame, dlu, dlu_rect, extra_w, group_box, push_button, refs, t, tl, DialogClose};
use crate::app_state::{Action, AppState};
use crate::inspector::fields;
use egui::{vec2, Id, Rect, Ui};
use serde_json::json;

fn label(ui: &mut Ui, _o: egui::Pos2, r: Rect, text: &str) {
    super::fit_label(ui, r, text, true);
}

/// Eingabefeld, das **bei jeder Änderung** den getippten Text meldet (Original: Bildgröße rechnet
/// Höhe und Druckgröße beim Tippen nach, PM-5 D19). Mit Fokus zeigt es den Puffer, sonst `current`.
/// Startfokus, Markieren per Tab und Tab-Folge wie die übrigen Dialogfelder.
fn live_text(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug, r: Rect, current: &str, enabled: bool) -> Option<String> {
    let id = Id::new(id);
    let te_id = id.with("te");
    let theme = crate::theme::Theme::current(ui.ctx());
    let focused = ui.memory(|m| m.has_focus(te_id));
    let mut buf = if focused { ui.data(|d| d.get_temp::<String>(id)).unwrap_or_else(|| current.to_string()) } else { current.to_string() };
    fields::field_frame(ui, r, enabled, focused && theme.glow);
    let inner = r.shrink2(vec2(3.0, 1.0));
    let te = egui::TextEdit::singleline(&mut buf)
        .id(te_id)
        .frame(egui::Frame::NONE)
        .font(theme.ui_font())
        .text_color(if enabled { theme.colors.field_text } else { theme.colors.text_disabled })
        .margin(egui::Margin::ZERO)
        .vertical_align(egui::Align::Center)
        .desired_width(inner.width())
        .event_filter(fields::TEXT_FILTER);
    let resp = ui.add_enabled_ui(enabled, |ui| ui.put(inner, te)).inner;
    super::autofocus_field(ui, &resp, current.chars().count());
    super::select_on_tab(ui, &resp, current.chars().count());
    let changed = resp.changed();
    if ui.memory(|m| m.has_focus(te_id)) {
        ui.data_mut(|d| d.insert_temp(id, buf.clone()));
    } else {
        ui.data_mut(|d| d.remove::<String>(id));
    }
    changed.then_some(buf)
}

/// Kettensymbol „Proportionen beibehalten“ rechts neben zwei Zeilen (Original: Symbole 5871/5872
/// neben den Einheiten der Pixelabmessungen und der Druckgröße), nur bei angehaktem Kästchen.
fn chain(ui: &Ui, first: Rect, second: Rect, enabled: bool) {
    let theme = crate::theme::Theme::current(ui.ctx());
    let r = Rect::from_min_max(egui::pos2(first.right() + 4.0, first.center().y - 2.0), egui::pos2(first.right() + 12.0, second.center().y + 2.0));
    crate::icons::paint(ui.painter(), r, crate::icons::Icon::LinkProportions, &crate::icons::IconStyle::new(&theme, None, enabled));
}

/// Druckgröße wie im Original: 3 Nachkommastellen, Punkt, ohne überflüssige Nullen
/// (500 px bei 72 ppi → „6.944“, 266,4 px → „3.7“; PM-5 D19).
pub fn fmt_print(v: f64) -> String {
    let s = format!("{v:.3}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

// ---------------------------------------------------------------------- Leinwandgröße

/// Leinwandgröße (5989).
#[derive(Clone, Debug, PartialEq)]
pub struct CanvasSizeDialog {
    pub width: f64,
    pub height: f64,
    pub cur_w: f64,
    pub cur_h: f64,
    pub unit: usize,
    pub anchor: usize,
    pub current_page_only: bool,
    pub resolution: f64,
}

/// Ankernamen für `modify.canvas.canvasSize`.
pub const ANCHORS: [&str; 9] = ["topLeft", "top", "topRight", "left", "center", "right", "bottomLeft", "bottom", "bottomRight"];
/// Tooltips der Ankerknöpfe (Schlüssel).
const ANCHOR_TIPS: [&str; 9] = ["dlg-anchor-top-left", "dlg-anchor-top", "dlg-anchor-top-right", "dlg-anchor-left", "dlg-anchor-center", "dlg-anchor-right", "dlg-anchor-bottom-left", "dlg-anchor-bottom", "dlg-anchor-bottom-right"];
use super::new_document::UNITS;

fn to_unit(px: f64, unit: usize, ppi: f64) -> f64 {
    match unit {
        1 => px / ppi,
        2 => px / ppi * 2.54,
        _ => px,
    }
}

fn from_unit(v: f64, unit: usize, ppi: f64) -> f64 {
    match unit {
        1 => v * ppi,
        2 => v * ppi / 2.54,
        _ => v,
    }
}

impl CanvasSizeDialog {
    /// Werte der aktuellen Seite.
    pub fn new(st: &AppState) -> Option<CanvasSizeDialog> {
        let d = st.session.active()?;
        let c = &d.doc.pages.get(d.page)?.canvas;
        let ppi = if c.resolution_unit == 1 { c.resolution as f64 * 2.54 } else { c.resolution as f64 };
        Some(CanvasSizeDialog { width: c.width as f64, height: c.height as f64, cur_w: c.width as f64, cur_h: c.height as f64, unit: 0, anchor: 4, current_page_only: st.session.prefs.get_bool("ResizeCanvasCurrentPageOnly", true), resolution: ppi.max(1.0) })
    }

    /// Parameter (Pixel, gerundet, 1 … 30000) oder `None` bei ungültigen Werten.
    pub fn params(&self) -> Option<serde_json::Value> {
        let w = from_unit(self.width, self.unit, self.resolution).round();
        let h = from_unit(self.height, self.unit, self.resolution).round();
        ((1.0..=30000.0).contains(&w) && (1.0..=30000.0).contains(&h)).then(|| json!({"width": w, "height": h, "anchor": ANCHORS[self.anchor.min(8)]}))
    }
}

/// Symbol vor der Breite (Original: ↔).
const W_SYMBOL: &str = "\u{2194}";
/// Symbol vor der Höhe (Original: ↕).
const H_SYMBOL: &str = "\u{2195}";

/// Anker 3 × 3 wie im Original: Optionsfelder mit Pfeilen, die vom gewählten Anker wegzeigen (dort
/// ein Kreuz aus vier Pfeilen). Wie [`super::radio_group`] **ein** Tab-Stopp (das gewählte Feld);
/// die Pfeiltasten wählen im Raster, ein Klick wählt und setzt den Fokus. Liefert die neue Wahl.
fn anchor_grid(ui: &mut Ui, id: &str, cells: &[Rect], tips: &[&str], sel: usize) -> Option<usize> {
    use egui::{Sense, Stroke, StrokeKind};
    let theme = crate::theme::Theme::current(ui.ctx());
    let c = theme.colors.clone();
    let gid = Id::new(id);
    let sel = sel.min(8);
    let (sx, sy) = ((sel % 3) as i32, (sel / 3) as i32);
    let mut out = None;
    let mut focus_to = None;
    for (k, r) in cells.iter().copied().enumerate().take(9) {
        let kid = gid.with(k);
        let tip = tips.get(k).copied().unwrap_or("").to_string();
        // nur das gewählte Feld ist fokussierbar (ein Tab-Stopp für das ganze Raster)
        let resp = ui.interact(r, kid, if k == sel { Sense::click() } else { Sense::CLICK }).on_hover_text(&tip);
        resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::RadioButton, true, k == sel, &tip));
        ui.painter().rect_filled(r, 2.0, if k == sel { c.list_selected_bg } else { c.field_bg });
        ui.painter().rect_stroke(r, 2.0, Stroke::new(1.0, if k == sel || resp.hovered() { c.focus } else { c.field_border }), StrokeKind::Inside);
        let (ddx, ddy) = ((k % 3) as i32 - sx, (k / 3) as i32 - sy);
        let stroke = Stroke::new(1.0, c.field_text);
        let m = r.center();
        if (ddx, ddy) == (0, 0) {
            for dir in [vec2(1.0, 0.0), vec2(-1.0, 0.0), vec2(0.0, 1.0), vec2(0.0, -1.0)] {
                ui.painter().arrow(m, dir * 5.0, stroke);
            }
        } else if ddx.abs() <= 1 && ddy.abs() <= 1 {
            let v = vec2(ddx as f32, ddy as f32).normalized() * 8.0;
            ui.painter().arrow(m - v * 0.5, v, stroke);
        }
        if k == sel && ui.memory(|mm| mm.has_focus(kid)) {
            ui.painter().rect_stroke(r.shrink(2.0), 1.0, Stroke::new(1.0, c.focus), StrokeKind::Inside);
            // Pfeiltasten bleiben im Raster (sonst sucht egui das nächste Feld in der Richtung)
            ui.memory_mut(|mm| mm.set_focus_lock_filter(kid, egui::EventFilter { horizontal_arrows: true, vertical_arrows: true, tab: false, escape: false }));
            let (mut nx, mut ny) = (sx, sy);
            ui.input_mut(|i| {
                for (key, mx, my) in [(egui::Key::ArrowLeft, -1, 0), (egui::Key::ArrowRight, 1, 0), (egui::Key::ArrowUp, 0, -1), (egui::Key::ArrowDown, 0, 1)] {
                    if i.consume_key(egui::Modifiers::NONE, key) {
                        nx = (nx + mx).clamp(0, 2);
                        ny = (ny + my).clamp(0, 2);
                    }
                }
            });
            let to = (ny * 3 + nx) as usize;
            if to != sel {
                out = Some(to);
                focus_to = Some(to);
            }
        }
        if resp.clicked() {
            if k != sel {
                out = Some(k);
            }
            focus_to = Some(k);
        }
    }
    if let Some(k) = focus_to {
        ui.memory_mut(|mm| mm.request_focus(gid.with(k)));
    }
    out
}

/// Zeigt den Dialog.
pub fn show_canvas_size(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut d) = st.dialogs.canvas_size.clone() else { return };
    let mut result = None;
    // Texte der aktuellen Sprache; längere Texte schieben die Spalten (Deutsch: Lage wie bisher)
    let units = tl(&UNITS);
    let wh = tl(&["dlg-short-w", "dlg-short-h"]);
    // Felder mit ↔/↕ wie im Original (Symbole 5950/5951); „B:/H:“ steht rechts als Live-Anzeige
    let syms = [W_SYMBOL, H_SYMBOL];
    let (anchor_l, page_l, hint_l, ok_l, cancel_l) = (t("dlg-canvas-size-anchor"), t("dlg-only-current-page"), t("dlg-canvas-size-range"), t("btn-ok"), t("btn-cancel"));
    let dx = extra_w(ctx, &syms, dlu(14.0, 0.0).x - 2.0);
    let cw = combo_fit_w(ctx, &refs(&units), dlu(60.0, 0.0).x);
    let dc = cw - dlu(60.0, 0.0).x;
    // Live-Anzeige „B: 30000“ rechts der Einheiten (Platz bis zum Gruppenrand)
    let live_w = super::max_text_w(ctx, &[&format!("{} 30000", wh[0]), &format!("{} 30000", wh[1])]);
    let dl = (live_w + 4.0 - dlu(208.0 + 7.0 - 152.0 - 4.0, 0.0).x).max(0.0);
    // „Anker:“ links des Rasters (Platz 20 … 66 DLU)
    let da = extra_w(ctx, &[&anchor_l], dlu(46.0, 0.0).x - 4.0);
    let bw = super::buttons_w(ctx, &[&ok_l, &cancel_l], dlu(50.0, 0.0).x);
    let ex = (dx + dc + dl)
        .max(da)
        .max(2.0 * bw + dlu(5.0 + 9.0, 0.0).x - dlu(222.0 - 7.0, 0.0).x)
        .max(super::text_w(ctx, &page_l) + 24.0 + da - dlu(208.0 + 7.0 - 70.0 - 4.0, 0.0).x)
        .max(0.0);
    let shift = move |x: f32| if x >= 150.0 { dx + dc } else if x >= 34.0 { dx } else { 0.0 };
    let rr = move |o: egui::Pos2, r: [f32; 4]| dlu_rect(o, r).translate(vec2(shift(r[0]), 0.0));
    let r = dialog_frame(ctx, "fp-canvas-size", &t("dlg-canvas-size-title"), dlu(222.0, 218.0) + vec2(ex, 0.0), |ui, body| {
        let o = body.min;
        let gw = |r: Rect| r.with_max_x(body.right() - dlu(7.0, 0.0).x);
        // Gruppe „Neue Größe“ mit Feldern, Anker und „Nur aktuelle Seite“ (Original 5989)
        group_box(ui, gw(dlu_rect(o, [7.0, 6.0, 208.0, 118.0])), &t("dlg-canvas-size-new"));
        let short = |r: Rect| Rect::from_min_size(r.min, vec2(r.width() + dx, r.height()));
        let dec = if d.unit == 0 { 0 } else { 2 };
        label(ui, o, short(dlu_rect(o, [20.0, 22.0, 14.0, 10.0])), W_SYMBOL);
        if let Some(t) = live_text(ui, "cs-w", rr(o, [34.0, 20.0, 50.0, 13.0]), &fields::fmt_num(d.width, dec), true)
            && let Some(v) = fields::parse_num(&t)
        {
            d.width = v.max(0.0);
        }
        label(ui, o, short(dlu_rect(o, [20.0, 40.0, 14.0, 10.0])), H_SYMBOL);
        if let Some(t) = live_text(ui, "cs-h", rr(o, [34.0, 38.0, 50.0, 13.0]), &fields::fmt_num(d.height, dec), true)
            && let Some(v) = fields::parse_num(&t)
        {
            d.height = v.max(0.0);
        }
        for (k, key) in [(0usize, "cs-wu"), (1, "cs-hu")] {
            let cr = rr(o, [88.0, 20.0 + k as f32 * 18.0, 60.0, 13.0]);
            if let Some(u) = fields::combo(ui, key, Rect::from_min_size(cr.min, vec2(cw, cr.height())), &refs(&units), Some(d.unit), true) {
                let (w, h) = (from_unit(d.width, d.unit, d.resolution), from_unit(d.height, d.unit, d.resolution));
                d.unit = u;
                d.width = to_unit(w, u, d.resolution);
                d.height = to_unit(h, u, d.resolution);
            }
        }
        // Live-Anzeige des neuen Maßes in Pixeln (Original 5780/5781: „B: 600“, beim Tippen)
        for (k, v) in [d.width, d.height].into_iter().enumerate() {
            let px = from_unit(v, d.unit, d.resolution).round();
            let lr = rr(o, [152.0, 22.0 + k as f32 * 18.0, 56.0, 10.0]);
            label(ui, o, Rect::from_min_size(lr.min, vec2(lr.width() + dl, lr.height())), &format!("{} {px:.0}", wh[k]));
        }
        // Anker 3 × 3 mit Pfeilen (5791–5800): ein Tab-Stopp, Pfeiltasten wählen
        label(ui, o, Rect::from_min_size(dlu_rect(o, [20.0, 62.0, 0.0, 10.0]).min, vec2(dlu(46.0, 0.0).x - 4.0 + da, dlu(0.0, 10.0).y)), &anchor_l);
        let cells: Vec<Rect> = (0..9).map(|i| dlu_rect(o, [70.0 + (i % 3) as f32 * 16.0, 58.0 + (i / 3) as f32 * 15.0, 15.0, 14.0]).translate(vec2(da, 0.0))).collect();
        let tips: Vec<String> = ANCHOR_TIPS.iter().map(|k| t(k)).collect();
        if let Some(a) = anchor_grid(ui, "cs-anchor", &cells, &refs(&tips), d.anchor) {
            d.anchor = a;
        }
        if let Some(b) = fields::checkbox(ui, "cs-page", dlu_rect(o, [70.0, 108.0, 140.0, 10.0]).translate(vec2(da, 0.0)).with_max_x(body.right() - dlu(9.0, 0.0).x), &page_l, d.current_page_only, true) {
            d.current_page_only = b;
        }
        group_box(ui, gw(dlu_rect(o, [7.0, 128.0, 208.0, 40.0])), &t("dlg-canvas-size-current"));
        label(ui, o, dlu_rect(o, [20.0, 141.0, 190.0, 10.0]), &fp_i18n::tr!("dlg-canvas-size-cur-w", w = format!("{:.0}", d.cur_w)));
        label(ui, o, dlu_rect(o, [20.0, 153.0, 190.0, 10.0]), &fp_i18n::tr!("dlg-canvas-size-cur-h", h = format!("{:.0}", d.cur_h)));
        let p = d.params();
        if p.is_none() {
            // Hinweis umbricht bei längeren Texten (Platz bis zu den Knöpfen)
            let hr = dlu_rect(o, [9.0, 172.0, 0.0, 22.0]);
            super::text_block(ui, Rect::from_min_max(hr.min, egui::pos2(body.right() - 8.0, hr.bottom())), hint_l.as_str());
        }
        let b = super::button_row(ui.ctx(), body.right() - dlu(7.0, 0.0).x, dlu_rect(o, [0.0, 196.0, 0.0, 0.0]).top(), &[&ok_l, &cancel_l], bw, dlu(0.0, 14.0).y, dlu(5.0, 0.0).x);
        if push_button(ui, b[0], &ok_l, true, p.is_some()) {
            result = Some(true);
        }
        if push_button(ui, b[1], &cancel_l, false, true) {
            result = Some(false);
        }
    });
    match (result, r) {
        (Some(true), _) => {
            st.dialogs.canvas_size = None;
            if let Some(p) = d.params() {
                st.session.prefs.set("ResizeCanvasCurrentPageOnly", fp_engine::PrefValue::Bool(d.current_page_only));
                st.push(Action::Run("modify.canvas.canvasSize".into(), p));
            }
        }
        (Some(false), _) | (_, DialogClose::Cancelled) => st.dialogs.canvas_size = None,
        _ => st.dialogs.canvas_size = Some(d),
    }
}

// ---------------------------------------------------------------------- Bildgröße

/// Bildgröße (6011).
#[derive(Clone, Debug, PartialEq)]
pub struct ImageSizeDialog {
    pub cur_w: f64,
    pub cur_h: f64,
    /// Neue Pixelgröße.
    pub w: f64,
    pub h: f64,
    /// Einheit der Pixelabmessungen: 0 Pixel, 1 Prozent.
    pub px_unit: usize,
    /// Einheit der Druckgröße: 0 Prozent, 1 Zoll, 2 Zentimeter.
    pub print_unit: usize,
    /// Auflösung in Pixel/Zoll bzw. Pixel/cm.
    pub resolution: f64,
    pub res_unit: usize,
    pub constrain: bool,
    pub resample: bool,
    pub method: usize,
    pub current_page_only: bool,
    /// Auflösung beim Öffnen (Pixel/Zoll): OK ohne Änderung führt keinen Befehl aus.
    pub cur_ppi: f64,
}


impl ImageSizeDialog {
    /// Werte der aktuellen Seite.
    pub fn new(st: &AppState) -> Option<ImageSizeDialog> {
        let d = st.session.active()?;
        let c = &d.doc.pages.get(d.page)?.canvas;
        let method = (st.session.prefs.get_int("ImageSizeTransformMode", 1).clamp(1, 4) - 1) as usize;
        Some(ImageSizeDialog {
            cur_w: c.width as f64,
            cur_h: c.height as f64,
            w: c.width as f64,
            h: c.height as f64,
            px_unit: 0,
            print_unit: 1,
            resolution: c.resolution.max(1.0) as f64,
            res_unit: if c.resolution_unit == 1 { 1 } else { 0 },
            constrain: st.session.prefs.get_bool("ResizeConstrain", true),
            resample: st.session.prefs.get_bool("ResizeResample", true),
            method,
            current_page_only: st.session.prefs.get_bool("ResizeImageCurrentPageOnly", true),
            cur_ppi: if c.resolution_unit == 1 { c.resolution.max(1.0) as f64 * 2.54 } else { c.resolution.max(1.0) as f64 },
        })
    }

    fn ppi(&self) -> f64 {
        if self.res_unit == 1 { self.resolution * 2.54 } else { self.resolution }
    }

    /// Breite neu setzen (mit Proportionen). Die Höhe bleibt ungerundet: das Original zeigt für
    /// B 333 bei 500 × 400 die Pixelhöhe 266, aber die Druckhöhe 3.7 (= 266,4 px / 72, PM-5 D19);
    /// gerundet wird erst beim Ausführen.
    pub fn set_width(&mut self, w: f64) {
        if self.constrain && self.cur_w > 0.0 {
            self.h = w * self.cur_h / self.cur_w;
        }
        self.w = w;
    }

    /// Höhe neu setzen (mit Proportionen, Breite ungerundet wie bei [`Self::set_width`]).
    pub fn set_height(&mut self, h: f64) {
        if self.constrain && self.cur_h > 0.0 {
            self.w = h * self.cur_w / self.cur_h;
        }
        self.h = h;
    }

    /// Nichts geändert (gleiche Pixelgröße und Auflösung): OK führt keinen Befehl aus, wie im
    /// Original (kein Verlaufsschritt, PM-5 D19).
    pub fn unchanged(&self) -> bool {
        self.w.round() == self.cur_w.round() && self.h.round() == self.cur_h.round() && (self.ppi() - self.cur_ppi).abs() < 1e-6
    }

    /// Parameter für `modify.canvas.imageSize` oder `None` bei ungültigen Werten.
    pub fn params(&self) -> Option<serde_json::Value> {
        let (w, h) = (self.w.round(), self.h.round());
        if !(1.0..=30000.0).contains(&w) || !(1.0..=30000.0).contains(&h) || !(1.0..=5000.0).contains(&self.resolution) {
            return None;
        }
        Some(json!({"width": w, "height": h, "constrain": self.constrain, "resample": self.resample, "resolution": self.ppi()}))
    }
}

/// Zeigt den Dialog.
pub fn show_image_size(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut d) = st.dialogs.image_size.clone() else { return };
    let mut result = None;
    let wh = tl(&["dlg-new-doc-width", "dlg-new-doc-height"]);
    let res_l = t("dlg-new-doc-resolution");
    let px_units = tl(&["dlg-unit-pixels", "dlg-unit-percent"]);
    let pr_units = tl(&["dlg-unit-percent", "dlg-unit-inches", "dlg-unit-cm"]);
    let res_units = tl(&super::new_document::RES_UNITS);
    let methods = tl(super::preferences::INTERP);
    let (constrain_l, resample_l, page_l, ok_l, cancel_l) = (t("dlg-image-size-constrain"), t("dlg-image-size-resample"), t("dlg-only-current-page"), t("btn-ok"), t("btn-cancel"));
    let dx = extra_w(ctx, &[&wh[0], &wh[1], &res_l], dlu(46.0, 0.0).x - 4.0);
    let all_units: Vec<&str> = px_units.iter().chain(pr_units.iter()).chain(res_units.iter()).map(|s| s.as_str()).collect();
    let cw = combo_fit_w(ctx, &all_units, dlu(54.0, 0.0).x);
    let mx = dlu(84.0, 0.0).x.max((dlu(9.0, 0.0).x + 18.0 + super::text_w(ctx, &resample_l) + 8.0).ceil());
    let mw = combo_fit_w(ctx, &refs(&methods), dlu(84.0, 0.0).x);
    let bw = super::buttons_w(ctx, &[&ok_l, &cancel_l], dlu(50.0, 0.0).x);
    let checks_w = super::max_text_w(ctx, &[&constrain_l, &page_l]) + 18.0 + dlu(9.0, 0.0).x;
    let ex = (dx + cw - dlu(54.0, 0.0).x)
        .max(mx + mw - dlu(168.0, 0.0).x)
        .max(checks_w + 8.0 - dlu(232.0, 0.0).x)
        .max(2.0 * bw + dlu(5.0 + 7.0 + 7.0, 0.0).x - dlu(232.0, 0.0).x)
        .max(0.0)
        .ceil();
    let rr = move |o: egui::Pos2, r: [f32; 4]| dlu_rect(o, r).translate(vec2(if r[0] >= 60.0 { dx } else { 0.0 }, 0.0));
    let cr = move |o: egui::Pos2, y: f32| Rect::from_min_size(rr(o, [114.0, y, 54.0, 13.0]).min, vec2(cw, dlu(0.0, 13.0).y));
    let r = dialog_frame(ctx, "fp-image-size", &t("dlg-image-size-title"), dlu(232.0, 214.0) + vec2(ex, 0.0), |ui, body| {
        let gw = |r: Rect| r.with_max_x(body.right() - dlu(7.0, 0.0).x);
        let lr = |y: f32| Rect::from_min_size(dlu_rect(body.min, [14.0, y, 0.0, 10.0]).min, vec2(dlu(46.0, 0.0).x - 4.0 + dx, dlu(0.0, 10.0).y));
        let o = body.min;
        let en_px = d.resample;
        group_box(ui, gw(dlu_rect(o, [7.0, 6.0, 218.0, 50.0])), &t("dlg-image-size-pixels"));
        for (k, (lab, key)) in [(&wh[0], "w"), (&wh[1], "h")].iter().enumerate() {
            let y = 20.0 + k as f32 * 18.0;
            label(ui, o, lr(y + 2.0), lab);
            let px = if k == 0 { d.w } else { d.h };
            let cur = if k == 0 { d.cur_w } else { d.cur_h };
            let shown = if d.px_unit == 1 { px / cur.max(1.0) * 100.0 } else { px };
            if let Some(t) = live_text(ui, ("is-px", *key), rr(o, [60.0, y, 50.0, 13.0]), &fields::fmt_num(shown, if d.px_unit == 1 { 1 } else { 0 }), en_px)
                && let Some(v) = fields::parse_num(&t)
            {
                let v = if d.px_unit == 1 { v / 100.0 * cur } else { v };
                if k == 0 {
                    d.set_width(v);
                } else {
                    d.set_height(v);
                }
            }
            if let Some(u) = fields::combo(ui, ("is-pxu", k), cr(o, y), &refs(&px_units), Some(d.px_unit), en_px) {
                d.px_unit = u;
            }
        }
        if d.constrain {
            chain(ui, cr(o, 20.0), cr(o, 38.0), en_px);
        }
        group_box(ui, gw(dlu_rect(o, [7.0, 62.0, 218.0, 70.0])), &t("dlg-image-size-print"));
        if d.constrain {
            chain(ui, cr(o, 76.0), cr(o, 94.0), true);
        }
        let ppi = d.ppi();
        for (k, lab) in wh.iter().enumerate() {
            let y = 76.0 + k as f32 * 18.0;
            label(ui, o, lr(y + 2.0), lab);
            let px = if k == 0 { d.w } else { d.h };
            let cur = if k == 0 { d.cur_w } else { d.cur_h };
            let shown = match d.print_unit {
                0 => px / cur.max(1.0) * 100.0,
                2 => px / ppi * 2.54,
                _ => px / ppi,
            };
            if let Some(t) = live_text(ui, ("is-pr", k), rr(o, [60.0, y, 50.0, 13.0]), &fmt_print(shown), true)
                && let Some(v) = fields::parse_num(&t)
            {
                let px = match d.print_unit {
                    0 => v / 100.0 * cur,
                    2 => v / 2.54 * ppi,
                    _ => v * ppi,
                };
                if d.resample {
                    if k == 0 {
                        d.set_width(px);
                    } else {
                        d.set_height(px);
                    }
                } else if px > 0.0 {
                    // ohne Neuauflösung ändert sich die Auflösung (Photoshop-Logik, A)
                    let inches = if d.print_unit == 2 { v / 2.54 } else { v };
                    if d.print_unit != 0 && inches > 0.0 {
                        let new_ppi = cur / inches;
                        d.resolution = if d.res_unit == 1 { new_ppi / 2.54 } else { new_ppi };
                    }
                }
            }
            if let Some(u) = fields::combo(ui, ("is-pru", k), cr(o, y), &refs(&pr_units), Some(d.print_unit), true) {
                d.print_unit = u;
            }
        }
        label(ui, o, lr(114.0), &res_l);
        if let Some(t) = fields::text(ui, "is-res", rr(o, [60.0, 112.0, 50.0, 13.0]), &fields::fmt_num(d.resolution, 2), true)
            && let Some(v) = fields::parse_num(&t)
        {
            let old_ppi = d.ppi();
            d.resolution = v.clamp(1.0, 5000.0);
            if d.resample && old_ppi > 0.0 {
                let f = d.ppi() / old_ppi;
                d.w = (d.w * f).round();
                d.h = (d.h * f).round();
            }
        }
        if let Some(u) = fields::combo(ui, "is-resu", cr(o, 112.0), &refs(&res_units), Some(d.res_unit), true) {
            if u != d.res_unit {
                d.resolution = if u == 1 { d.resolution / 2.54 } else { d.resolution * 2.54 };
            }
            d.res_unit = u;
        }
        if let Some(b) = super::access::checkbox(ui, "is-constrain", dlu_rect(o, [9.0, 140.0, 120.0, 10.0]).with_max_x(body.right() - 8.0), &constrain_l, d.constrain, true) {
            d.constrain = b;
        }
        if let Some(b) = super::access::checkbox(ui, "is-resample", Rect::from_min_max(dlu_rect(o, [9.0, 154.0, 70.0, 10.0]).min, egui::pos2(o.x + mx - 4.0, dlu_rect(o, [9.0, 154.0, 70.0, 10.0]).bottom())), &resample_l, d.resample, true) {
            d.resample = b;
            if !b {
                d.w = d.cur_w;
                d.h = d.cur_h;
            }
        }
        if let Some(m) = fields::combo(ui, "is-method", Rect::from_min_size(egui::pos2(o.x + mx, dlu_rect(o, [84.0, 152.0, 84.0, 13.0]).top()), vec2(mw, dlu(0.0, 13.0).y)), &refs(&methods), Some(d.method), d.resample) {
            d.method = m;
        }
        if let Some(b) = fields::checkbox(ui, "is-page", dlu_rect(o, [9.0, 170.0, 120.0, 10.0]).with_max_x(body.right() - 8.0), &page_l, d.current_page_only, true) {
            d.current_page_only = b;
        }
        let p = d.params();
        let b = super::button_row(ui.ctx(), body.right() - dlu(7.0, 0.0).x, dlu_rect(o, [0.0, 192.0, 0.0, 0.0]).top(), &[&ok_l, &cancel_l], bw, dlu(0.0, 14.0).y, dlu(5.0, 0.0).x);
        if push_button(ui, b[0], &ok_l, true, p.is_some()) {
            result = Some(true);
        }
        if push_button(ui, b[1], &cancel_l, false, true) {
            result = Some(false);
        }
    });
    match (result, r) {
        (Some(true), _) => {
            st.dialogs.image_size = None;
            if let Some(p) = d.params() {
                st.session.prefs.set("ImageSizeTransformMode", fp_engine::PrefValue::Int(d.method as i64 + 1));
                st.session.prefs.set("ResizeConstrain", fp_engine::PrefValue::Bool(d.constrain));
                st.session.prefs.set("ResizeResample", fp_engine::PrefValue::Bool(d.resample));
                st.session.prefs.set("ResizeImageCurrentPageOnly", fp_engine::PrefValue::Bool(d.current_page_only));
                if !d.unchanged() {
                    st.push(Action::Run("modify.canvas.imageSize".into(), p));
                }
            }
        }
        (Some(false), _) | (_, DialogClose::Cancelled) => st.dialogs.image_size = None,
        _ => st.dialogs.image_size = Some(d),
    }
}

// ---------------------------------------------------------------------- Numerisch transformieren

/// Gemerkte Werte von „Numerisch transformieren“ (eigene Schlüssel, technisch englisch).
const NT_MODE: &str = "NumericTransformMode";
const NT_SCALE_X: &str = "NumericTransformScaleX";
const NT_SCALE_Y: &str = "NumericTransformScaleY";
const NT_ANGLE: &str = "NumericTransformAngle";
/// Pixelmaße bei „Größe ändern“ (DLG-7).
const NT_WIDTH: &str = "NumericTransformWidth";
const NT_HEIGHT: &str = "NumericTransformHeight";

/// Numerisch transformieren (6002).
#[derive(Clone, Debug, PartialEq)]
pub struct NumericTransformDialog {
    /// 0 Skalieren, 1 Größe ändern, 2 Drehen.
    pub mode: usize,
    pub sx: f64,
    pub sy: f64,
    pub w: f64,
    pub h: f64,
    pub cur_w: f64,
    pub cur_h: f64,
    pub angle: f64,
    pub scale_attrs: bool,
    pub constrain: bool,
    pub interpolate: bool,
}

impl NumericTransformDialog {
    /// Werte der Auswahl.
    pub fn new(st: &AppState) -> Option<NumericTransformDialog> {
        let d = st.session.active()?;
        let b = fp_engine::docops::ids_bounds(&d.doc, d.page, &d.selection.elements)?;
        // Art, Prozentwerte und Winkel: zuletzt per OK bestätigt (PM-6 D26, Original 50/50 gemerkt)
        let p = &st.session.prefs;
        let pct = |k: &str| p.get_real(k, 100.0).clamp(1.0, 10000.0);
        Some(NumericTransformDialog {
            mode: p.get_int(NT_MODE, 0).clamp(0, 2) as usize,
            sx: pct(NT_SCALE_X),
            sy: pct(NT_SCALE_Y),
            // Pixelmaße bei „Größe ändern“: zuletzt per OK bestätigt (DLG-7), sonst die Auswahl
            w: p.get_real(NT_WIDTH, b.width() as f64).clamp(1.0, 30000.0),
            h: p.get_real(NT_HEIGHT, b.height() as f64).clamp(1.0, 30000.0),
            cur_w: b.width() as f64,
            cur_h: b.height() as f64,
            angle: p.get_real(NT_ANGLE, 0.0).clamp(-360.0, 360.0),
            scale_attrs: p.get_bool("XformToolXformsBFE", true),
            constrain: p.get_bool("TransformConstrain", true),
            interpolate: true,
        })
    }

    /// Bestätigte Werte merken (nur bei OK).
    pub fn remember(&self, p: &mut fp_engine::Preferences) {
        p.set("TransformConstrain", fp_engine::PrefValue::Bool(self.constrain));
        p.set(NT_MODE, fp_engine::PrefValue::Int(self.mode as i64));
        match self.mode {
            0 => {
                p.set(NT_SCALE_X, fp_engine::PrefValue::Real(self.sx));
                p.set(NT_SCALE_Y, fp_engine::PrefValue::Real(self.sy));
            }
            2 => p.set(NT_ANGLE, fp_engine::PrefValue::Real(self.angle)),
            _ => {
                p.set(NT_WIDTH, fp_engine::PrefValue::Real(self.w));
                p.set(NT_HEIGHT, fp_engine::PrefValue::Real(self.h));
            }
        }
    }

    /// Parameter für `modify.transform.numericTransform`.
    pub fn params(&self) -> Option<serde_json::Value> {
        match self.mode {
            1 => ((1.0..=30000.0).contains(&self.w) && (1.0..=30000.0).contains(&self.h)).then(|| json!({"mode": "resize", "width": self.w, "height": self.h, "constrain": self.constrain})),
            2 => self.angle.is_finite().then(|| json!({"mode": "rotate", "angle": self.angle})),
            _ => (self.sx > 0.0 && self.sy > 0.0 && self.sx <= 10000.0 && self.sy <= 10000.0).then(|| json!({"mode": "scale", "scaleX": self.sx, "scaleY": self.sy, "constrain": self.constrain})),
        }
    }
}

impl NumericTransformDialog {
    /// Getippter Wert in Feld `k` (0 = waagerecht, 1 = senkrecht) der aktuellen Art; mit
    /// „Proportionen beibehalten“ rechnet das andere Feld sofort mit (Original: B 50 → H 50 beim
    /// Tippen, PM-6 D26).
    pub fn set_value(&mut self, k: usize, nv: f64) {
        let nv = nv.max(0.0);
        if self.mode == 1 {
            if k == 0 {
                if self.constrain && self.cur_w > 0.0 {
                    self.h = (nv * self.cur_h / self.cur_w).round();
                }
                self.w = nv;
            } else {
                if self.constrain && self.cur_h > 0.0 {
                    self.w = (nv * self.cur_w / self.cur_h).round();
                }
                self.h = nv;
            }
        } else if k == 0 {
            self.sx = nv;
            if self.constrain {
                self.sy = nv;
            }
        } else {
            self.sy = nv;
            if self.constrain {
                self.sx = nv;
            }
        }
    }

    /// Objektgröße nach der Transformation, rechts im Dialog („B: 100 / H: 80“); beim Drehen keine.
    pub fn object_size(&self) -> Option<(f64, f64)> {
        match self.mode {
            0 => Some((self.cur_w * self.sx / 100.0, self.cur_h * self.sy / 100.0)),
            1 => Some((self.w, self.h)),
            _ => None,
        }
    }
}

/// Zeigt den Dialog (Original 6002, PM-6 D26): Liste Skalieren/Größe ändern/Drehen; in einem Rahmen
/// die Felder mit ↔/↕-Symbol, Einheit, Kette (bei „Proportionen beibehalten“) und rechts die
/// Objektgröße „B: … / H: …“; darunter die Kästchen, OK/Abbrechen rechts oben.
pub fn show_numeric_transform(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut d) = st.dialogs.numeric_transform.clone() else { return };
    let mut result = None;
    let modes = tl(&["dlg-numeric-scale", "dlg-numeric-resize", "dlg-numeric-rotate"]);
    let wh = tl(&["dlg-new-doc-width", "dlg-new-doc-height"]);
    let angle_l = t("dlg-numeric-angle");
    let px_l = t("dlg-unit-pixels");
    let (attrs_l, constrain_l, ok_l, cancel_l) = (t("dlg-numeric-scale-attributes"), t("dlg-constrain"), t("btn-ok"), t("btn-cancel"));
    let mw = combo_fit_w(ctx, &refs(&modes), dlu(120.0, 0.0).x);
    let bw = super::buttons_w(ctx, &[&ok_l, &cancel_l], dlu(56.0, 0.0).x);
    let checks = super::max_text_w(ctx, &[&attrs_l, &constrain_l]) + 18.0 + dlu(8.0, 0.0).x + 8.0;
    // Spalten im Rahmen: Symbol, Feld, Einheit, Kette, Objektgröße
    let field_x = dlu(30.0, 0.0).x.max(dlu(14.0, 0.0).x + super::text_w(ctx, &angle_l) + 6.0).ceil();
    let field_w = dlu(34.0, 0.0).x;
    let unit_w = super::max_text_w(ctx, &["%", &px_l]).ceil();
    let size_x = field_x + field_w + 6.0 + unit_w + 22.0;
    let size_w = super::max_text_w(ctx, &[&fp_i18n::tr!("dlg-numeric-size-w", v = "00000"), &fp_i18n::tr!("dlg-numeric-size-h", v = "00000")]).ceil();
    let frame_r = (size_x + size_w + 10.0).max(dlu(8.0, 0.0).x + mw).ceil();
    // Knopfspalte rechts von Rahmen und Modusliste
    let bx = dlu(150.0, 0.0).x.max(frame_r + 12.0).ceil();
    let width = (bx + bw + dlu(8.0, 0.0).x).max(dlu(214.0, 0.0).x).max(checks);
    let r = dialog_frame(ctx, "fp-numeric", &t("dlg-numeric-title"), vec2(width, dlu(0.0, 130.0).y), |ui, body| {
        let o = body.min;
        let theme = crate::theme::Theme::current(ui.ctx());
        if let Some(m) = fields::combo(ui, "nt-mode", Rect::from_min_size(dlu_rect(o, [8.0, 8.0, 120.0, 13.0]).min, vec2(mw, dlu(0.0, 13.0).y)), &refs(&modes), Some(d.mode), true) {
            d.mode = m;
        }
        let frame = Rect::from_min_max(dlu_rect(o, [8.0, 26.0, 0.0, 0.0]).min, egui::pos2(o.x + frame_r, dlu_rect(o, [0.0, 68.0, 0.0, 0.0]).top()));
        ui.painter().rect_stroke(frame, 2.0, egui::Stroke::new(1.0, if theme.glow { theme.colors.line_dark } else { theme.colors.line_mid }), egui::StrokeKind::Inside);
        let row = |k: usize| dlu_rect(o, [0.0, 32.0 + k as f32 * 18.0, 0.0, 13.0]);
        let fr = |k: usize| Rect::from_min_size(egui::pos2(o.x + field_x, row(k).top()), vec2(field_w, row(k).height()));
        match d.mode {
            2 => {
                label(ui, o, Rect::from_min_size(egui::pos2(o.x + dlu(14.0, 0.0).x, fr(0).top()), vec2(field_x - dlu(14.0, 0.0).x - 4.0, fr(0).height())), &angle_l);
                if let Some(t) = live_text(ui, "nt-angle", fr(0), &fields::fmt_num(d.angle, 1), true)
                    && let Some(v) = fields::parse_num(&t)
                {
                    d.angle = v.clamp(-360.0, 360.0);
                }
                label(ui, o, Rect::from_min_size(egui::pos2(fr(0).right() + 6.0, fr(0).top()), vec2(12.0, fr(0).height())), "°");
            }
            m => {
                let (a, b, unit) = if m == 1 { (d.w, d.h, px_l.as_str()) } else { (d.sx, d.sy, "%") };
                let mut units = [Rect::NOTHING; 2];
                for (k, v) in [a, b].into_iter().enumerate() {
                    let f = fr(k);
                    // Symbol ↔ bzw. ↕ statt „Breite:/Höhe:“ (Text bleibt Tooltip und Name)
                    let sr = Rect::from_center_size(egui::pos2(o.x + dlu(14.0, 0.0).x + 6.0, f.center().y), if k == 0 { vec2(13.0, 7.0) } else { vec2(7.0, 13.0) });
                    super::path_dialogs::paint_double_arrow(ui, sr, k == 0, theme.colors.text);
                    let hover = ui.interact(sr.expand(3.0), Id::new(("nt-symbol", k)), egui::Sense::hover());
                    hover.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, wh[k].as_str()));
                    let _ = hover.on_hover_text(wh[k].as_str());
                    if let Some(t) = live_text(ui, ("nt-v", m, k), f, &fields::fmt_num(v, 1), true)
                        && let Some(nv) = fields::parse_num(&t)
                    {
                        d.set_value(k, nv);
                    }
                    units[k] = Rect::from_min_size(egui::pos2(f.right() + 6.0, f.top()), vec2(unit_w, f.height()));
                    label(ui, o, units[k], unit);
                }
                if d.constrain {
                    chain(ui, units[0], units[1], true);
                }
                if let Some((w, h)) = d.object_size() {
                    for (k, (key, v)) in [("dlg-numeric-size-w", w), ("dlg-numeric-size-h", h)].into_iter().enumerate() {
                        let s = fp_i18n::tr!(key, v = format!("{}", v.round() as i64));
                        fields::label(ui, Rect::from_min_size(egui::pos2(o.x + size_x, fr(k).top()), vec2(size_w + 2.0, fr(k).height())), &s, true);
                    }
                }
            }
        }
        if let Some(b) = fields::checkbox(ui, "nt-attrs", dlu_rect(o, [8.0, 74.0, 120.0, 10.0]).with_max_x(body.right() - 4.0), &attrs_l, d.scale_attrs, d.mode != 2) {
            d.scale_attrs = b;
        }
        if let Some(b) = fields::checkbox(ui, "nt-constrain", dlu_rect(o, [8.0, 88.0, 120.0, 10.0]).with_max_x(body.right() - 4.0), &constrain_l, d.constrain, d.mode != 2) {
            d.constrain = b;
        }
        let p = d.params();
        let bh = dlu(0.0, 14.0).y;
        if push_button(ui, Rect::from_min_size(egui::pos2(o.x + bx, dlu_rect(o, [0.0, 8.0, 0.0, 0.0]).top()), vec2(bw, bh)), &ok_l, true, p.is_some()) {
            result = Some(true);
        }
        if push_button(ui, Rect::from_min_size(egui::pos2(o.x + bx, dlu_rect(o, [0.0, 26.0, 0.0, 0.0]).top()), vec2(bw, bh)), &cancel_l, false, true) {
            result = Some(false);
        }
    });
    match (result, r) {
        (Some(true), _) => {
            st.dialogs.numeric_transform = None;
            if let Some(p) = d.params() {
                d.remember(&mut st.session.prefs);
                st.push(Action::Run("modify.transform.numericTransform".into(), p));
            }
        }
        (Some(false), _) | (_, DialogClose::Cancelled) => st.dialogs.numeric_transform = None,
        _ => st.dialogs.numeric_transform = Some(d),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn st_with_doc() -> AppState {
        let mut s = fp_engine::Session::default();
        let _ = s.execute("file.new", json!({"width": 200, "height": 100}));
        AppState::new(s, crate::app_state::UiSettings::default(), None)
    }

    #[test]
    fn canvas_and_image_size_validation() {
        let st = st_with_doc();
        let mut c = CanvasSizeDialog::new(&st).expect("Dialog");
        assert_eq!(c.params().map(|p| p["width"].clone()), Some(json!(200.0)));
        c.width = 0.0;
        assert!(c.params().is_none());
        c.width = 2.0;
        c.unit = 1;
        assert_eq!(c.params().map(|p| p["width"].clone()), Some(json!(144.0)));
        let mut i = ImageSizeDialog::new(&st).expect("Dialog");
        i.set_width(400.0);
        assert_eq!(i.h, 200.0);
        i.constrain = false;
        i.set_height(50.0);
        assert_eq!((i.w, i.h), (400.0, 50.0));
        i.w = 40000.0;
        assert!(i.params().is_none());
    }
}
