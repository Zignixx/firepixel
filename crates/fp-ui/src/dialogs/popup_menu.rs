//! Pop-up-Menü-Editor (6283, Modifizieren ▸ Pop-up-Menü ▸ hinzufügen/bearbeiten): Registerkarten
//! Inhalt (Einträge mit Text, Link, Ziel; Einrücken = Untermenü), Erscheinungsbild (Ausrichtung,
//! Textfarben, Hervorhebung, Fett), Erweitert (Abstände, Rahmen, Schatten, Ausblendzeit, feste
//! Größe) und Position (Versatz von Menü und Untermenü). OK führt `modify.popUpMenu.addPopUpMenu`
//! bzw. `…editPopUpMenu` aus (ein Verlaufsschritt), Abbrechen ändert nichts.

use super::{dialog_frame, push_button, refs, t, tl, DialogClose};
use crate::app_state::{Action, AppState};
use crate::inspector::fields;
use crate::theme::Theme;
use egui::{pos2, vec2, Align2, Rect, Sense, Stroke, StrokeKind};
use serde_json::{json, Map, Value};

/// Ein Eintrag (Ebene 0 = Hauptmenü, 1.. = Untermenü des vorherigen Eintrags).
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub level: usize,
    pub text: String,
    pub url: String,
    pub target: String,
}

/// Zustand des Editors.
#[derive(Clone, Debug, PartialEq)]
pub struct PopupEditor {
    pub edit: bool,
    pub tab: usize,
    pub rows: Vec<Row>,
    pub selected: Option<usize>,
    /// Darstellungsfelder (`fp_model::PopupMenu`, snake_case).
    pub menu: Map<String, Value>,
}

fn flatten(items: &[Value], level: usize, out: &mut Vec<Row>) {
    for it in items {
        out.push(Row {
            level,
            text: it.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            url: it.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            target: it.get("target").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        });
        if let Some(sub) = it.get("submenu").and_then(|v| v.as_array()) {
            flatten(sub, level + 1, out);
        }
    }
}

/// Einträge als verschachteltes JSON (`[{text, url, target, submenu}]`).
pub fn nest(rows: &[Row]) -> Vec<Value> {
    fn build(rows: &[Row], i: &mut usize, level: usize) -> Vec<Value> {
        let mut out: Vec<Value> = Vec::new();
        while *i < rows.len() {
            let r = &rows[*i];
            if r.level < level {
                break;
            }
            if r.level > level {
                // Untermenü des letzten Eintrags
                let sub = build(rows, i, level + 1);
                if let Some(Value::Object(last)) = out.last_mut() {
                    last.insert("submenu".into(), Value::Array(sub));
                } else {
                    out.extend(sub);
                }
                continue;
            }
            out.push(json!({"text": r.text, "url": r.url, "target": r.target, "submenu": []}));
            *i += 1;
        }
        out
    }
    let mut i = 0;
    build(rows, &mut i, 0)
}

impl PopupEditor {
    /// Neues Menü bzw. das vorhandene Menü des ausgewählten Segments/Hotspots.
    pub fn open(st: &mut AppState, edit: bool) -> Option<PopupEditor> {
        if edit {
            let v = crate::ui_commands::execute(&mut st.session, "modify.popUpMenu.editPopUpMenu", json!({})).ok()?.value;
            let mut rows = Vec::new();
            flatten(v.get("items").and_then(|x| x.as_array()).map(|a| a.as_slice()).unwrap_or(&[]), 0, &mut rows);
            let menu = v.get("menu").and_then(|m| m.as_object()).cloned().unwrap_or_default();
            Some(PopupEditor { edit, tab: 0, selected: (!rows.is_empty()).then_some(0), rows, menu })
        } else {
            let mut menu = Map::new();
            menu.insert("vertical".into(), json!(true));
            Some(PopupEditor { edit, tab: 0, rows: vec![Row { level: 0, text: String::new(), url: String::new(), target: String::new() }], selected: Some(0), menu })
        }
    }

    /// Befehl und Parameter für OK.
    pub fn invocation(&self) -> (&'static str, Value) {
        let items = nest(&self.rows);
        let p = json!({"items": items, "menu": Value::Object(self.menu.clone())});
        (if self.edit { "modify.popUpMenu.editPopUpMenu" } else { "modify.popUpMenu.addPopUpMenu" }, p)
    }

    fn num(&self, k: &str, d: f64) -> f64 {
        self.menu.get(k).and_then(|v| v.as_f64()).unwrap_or(d)
    }
    fn flag(&self, k: &str, d: bool) -> bool {
        self.menu.get(k).and_then(|v| v.as_bool()).unwrap_or(d)
    }
    fn color(&self, k: &str, d: u32) -> fp_color::Argb {
        fp_color::Argb(self.menu.get(k).and_then(|v| v.as_u64()).map(|v| v as u32).unwrap_or(d) | 0xff00_0000)
    }
}

/// Zeilenhöhe der Inhaltstabelle.
const ROW_H: f32 = 20.0;

/// Reiter „Inhalt“ wie im Original (TW-39): Werkzeugleiste + / − │ Einrücken / Ausrücken, darunter
/// die Tabelle mit den Spalten Text, Hyperlink und Ziel. Die gewählte Zeile ist direkt
/// bearbeitbar, ein Klick auf eine andere Zeile wählt sie.
fn content_page(ui: &mut egui::Ui, d: &mut PopupEditor, area: Rect, cols: &[String]) {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let inner = area.shrink(4.0);
    // Werkzeugleiste
    let bar = Rect::from_min_size(inner.min, vec2(inner.width(), 30.0));
    ui.painter().rect_filled(bar, 0.0, c.line_light);
    let btn = |k: f32, gap: f32| Rect::from_min_size(pos2(bar.left() + 6.0 + k * 24.0 + gap, bar.top() + 4.0), vec2(22.0, 22.0));
    let sel = d.selected.filter(|i| *i < d.rows.len());
    let tips = tl(&["dlg-pm-add", "dlg-pm-delete", "dlg-pm-indent", "dlg-pm-outdent"]);
    let mut clicked = [false; 4];
    let enabled = [true, sel.is_some() && d.rows.len() > 1, sel.is_some_and(|i| i > 0 && d.rows[i].level <= d.rows[i - 1].level), sel.is_some_and(|i| d.rows[i].level > 0)];
    for (k, (glyph, gap)) in [("+", 0.0), ("−", 0.0), ("→", 14.0), ("←", 14.0)].iter().enumerate() {
        // Werkzeugknopf mit Zeichen; Name für Barrierefreiheit = Kurzinfo („Menü hinzufügen“ …)
        let r = btn(k as f32, *gap);
        let en = enabled[k];
        let resp = ui.interact(r, egui::Id::new(("pm-tool", k)), if en { Sense::click() } else { Sense::hover() });
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, en, tips[k].as_str()));
        let hot = en && resp.hovered();
        let bg = if en && resp.is_pointer_button_down_on() { c.button_pressed_bg } else if hot { c.button_hover_bg } else { c.button_bg };
        ui.painter().rect_filled(r, 2.0, bg);
        ui.painter().rect_stroke(r, 2.0, Stroke::new(1.0, if hot { c.button_hover_border } else { c.button_border }), StrokeKind::Inside);
        ui.painter().text(r.center(), Align2::CENTER_CENTER, *glyph, theme.bold(theme.fonts.ui), if en { c.text } else { c.text_disabled });
        clicked[k] = en && resp.clicked();
        resp.on_hover_text(tips[k].as_str());
    }
    if clicked[0] {
        let level = sel.and_then(|i| d.rows.get(i)).map(|r| r.level).unwrap_or(0);
        let at = sel.map(|i| i + 1).unwrap_or(d.rows.len());
        d.rows.insert(at, Row { level, text: String::new(), url: String::new(), target: String::new() });
        d.selected = Some(at);
    }
    if clicked[1]
        && let Some(i) = sel
    {
        d.rows.remove(i);
        d.selected = Some(i.min(d.rows.len().saturating_sub(1)));
    }
    if clicked[2]
        && let Some(i) = sel
    {
        d.rows[i].level += 1;
    }
    if clicked[3]
        && let Some(i) = sel
    {
        d.rows[i].level -= 1;
    }
    // Tabelle
    let table = Rect::from_min_max(pos2(inner.left(), bar.bottom() + 2.0), inner.max);
    ui.painter().rect_filled(table, 0.0, c.list_bg);
    ui.painter().rect_stroke(table, 0.0, Stroke::new(1.0, c.list_border), StrokeKind::Inside);
    let tw = table.width() - 2.0;
    let widths = [(tw * 0.28).max(super::text_w(ui.ctx(), &cols[0]) + 12.0), (tw * 0.34).max(super::text_w(ui.ctx(), &cols[1]) + 12.0), (tw * 0.20).max(super::text_w(ui.ctx(), &cols[2]) + 12.0)];
    let mut xs = [table.left() + 1.0; 4];
    for k in 0..3 {
        xs[k + 1] = xs[k] + widths[k];
    }
    let grid = Stroke::new(1.0, c.line_mid);
    let head = Rect::from_min_max(pos2(table.left() + 1.0, table.top() + 1.0), pos2(xs[3], table.top() + 1.0 + 26.0));
    for k in 0..3 {
        let cr = Rect::from_min_max(pos2(xs[k], head.top()), pos2(xs[k + 1], head.bottom()));
        ui.painter().text(pos2(cr.left() + 6.0, cr.top() + 10.0), Align2::LEFT_CENTER, cols[k].as_str(), theme.ui_font(), c.text);
        ui.painter().vline(cr.right() - 0.5, cr.y_range(), grid);
    }
    ui.painter().hline(head.x_range(), head.bottom() - 0.5, grid);
    let rows_top = head.bottom();
    let max_rows = (((table.bottom() - 1.0 - rows_top) / ROW_H).floor() as usize).max(1);
    let first = sel.map(|i| (i + 1).saturating_sub(max_rows)).unwrap_or(0);
    let clip = ui.clip_rect();
    ui.set_clip_rect(table.shrink(1.0).intersect(clip));
    for (vis, i) in (first..d.rows.len()).take(max_rows).enumerate() {
        let y = rows_top + vis as f32 * ROW_H;
        let rr = Rect::from_min_max(pos2(xs[0], y), pos2(xs[3], y + ROW_H));
        let is_sel = sel == Some(i);
        if !is_sel {
            let resp = ui.interact(rr, egui::Id::new(("pm-row", i)), Sense::click());
            if resp.clicked() {
                d.selected = Some(i);
            }
        }
        for k in 0..3 {
            let indent = if k == 0 { d.rows[i].level as f32 * 12.0 } else { 0.0 };
            let cr = Rect::from_min_max(pos2(xs[k] + 1.0 + indent, y + 1.0), pos2(xs[k + 1] - 1.0, y + ROW_H - 1.0));
            let cur = match k {
                0 => d.rows[i].text.clone(),
                1 => d.rows[i].url.clone(),
                _ => d.rows[i].target.clone(),
            };
            if is_sel {
                if let Some(v) = fields::text(ui, ("pm-cell", k, i), cr, &cur, true) {
                    match k {
                        0 => d.rows[i].text = v,
                        1 => d.rows[i].url = v,
                        _ => d.rows[i].target = v,
                    }
                }
            } else {
                let _ = crate::ui_kit::paint_text_fit(ui.painter(), cr.shrink2(vec2(4.0, 0.0)), &cur, theme.ui_font(), c.text);
            }
            ui.painter().vline(xs[k + 1] - 0.5, y..=y + ROW_H, grid);
        }
        ui.painter().hline(xs[0]..=xs[3], y + ROW_H - 0.5, grid);
    }
    ui.set_clip_rect(clip);
}

/// Zeigt den Editor.
pub fn show(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut d) = st.dialogs.popup_editor.clone() else { return };
    let mut result = None;
    let title = if d.edit { t("dlg-pm-title-edit") } else { t("dlg-pm-title") };
    // Texte der aktuellen Sprache; Spalten wachsen mit (Deutsch: Lage wie bisher)
    let tabs = tl(&["dlg-pm-tab-content", "dlg-pm-tab-appearance", "dlg-pm-tab-advanced", "dlg-pm-tab-position"]);
    let l0 = tl(&["dlg-pm-col-text", "dlg-pm-col-link", "dlg-pm-col-target"]);
    let l1 = tl(&["dlg-pm-orientation", "dlg-pm-text-align", "dlg-pm-text-up", "dlg-pm-text-over", "dlg-pm-highlight"]);
    let radios = tl(&["dlg-pm-vertical", "dlg-pm-horizontal"]);
    let aligns = tl(&["dlg-pm-align-left", "dlg-pm-align-center", "dlg-pm-align-right"]);
    let l2 = tl(&["dlg-pm-cell-spacing", "dlg-pm-cell-padding", "dlg-pm-text-indent", "dlg-pm-border-width", "dlg-pm-hide-delay", "dlg-pm-fixed-width", "dlg-pm-border-color", "dlg-pm-shadow-color"]);
    let l3 = tl(&["dlg-pm-menu-x", "dlg-pm-menu-y", "dlg-pm-sub-x", "dlg-pm-sub-y"]);
    let (bold_l, hide_l, rel_l, ok_l, cancel_l) = (t("dlg-pm-bold"), t("dlg-pm-hide-on-leave"), t("dlg-pm-sub-relative"), t("dlg-pm-finish"), t("btn-cancel"));
    let tab_w = super::buttons_w(ctx, &refs(&tabs), 108.0);
    let dx0 = super::extra_w(ctx, &refs(&l0), 60.0 - 4.0);
    let dx1 = super::extra_w(ctx, &refs(&l1), 100.0 - 4.0);
    let l23: Vec<&str> = l2.iter().chain(l3.iter()).map(|s| s.as_str()).collect();
    let dx2 = super::extra_w(ctx, &l23, 150.0 - 4.0);
    let rw: Vec<f32> = radios.iter().map(|r| (super::text_w(ctx, r) + 20.0).max(110.0)).collect();
    let acw = super::combo_fit_w(ctx, &refs(&aligns), 120.0);
    let bold_w = (18.0 + super::text_w(ctx, &bold_l)).max(80.0);
    let inner_w = (3.0 * dx0 + 420.0)
        .max(110.0 + dx1 + rw[0] + 10.0 + rw[1])
        .max(110.0 + dx1 + acw + 20.0 + bold_w)
        .max(160.0 + dx2 + 60.0)
        .max(18.0 + super::max_text_w(ctx, &[&hide_l, &rel_l]));
    let width = 560f32.max(24.0 + 4.0 * (tab_w + 4.0)).max(24.0 + 20.0 + inner_w).max(super::button_row_w(ctx, &[&cancel_l, &t("dlg-wiz-nav-back"), &t("dlg-wiz-nav-next"), &ok_l], 76.0, 8.0) + 32.0).ceil();
    let r = dialog_frame(ctx, "fp-popup-editor", &title, vec2(width, 420.0), |ui, body| {
        let theme = Theme::current(ui.ctx());
        let c = &theme.colors;
        let o = body.min + vec2(12.0, 10.0);
        // Registerkarten wie im Original (TW-39): echte Reiter links, so breit wie ihr Text; der
        // aktive Reiter ist höher und geht in die Seite über
        let area = Rect::from_min_max(o + vec2(-8.0, 22.0), pos2(body.right() - 4.0, body.bottom() - 44.0));
        let mut tx = area.left() + 2.0;
        let mut active_tab = None;
        for (k, name) in tabs.iter().enumerate() {
            let w = super::text_w(ui.ctx(), name) + 14.0;
            let on = d.tab == k;
            let tr = if on { Rect::from_min_max(pos2(tx - 2.0, area.top() - 22.0), pos2(tx + w + 2.0, area.top() + 1.0)) } else { Rect::from_min_max(pos2(tx, area.top() - 19.0), pos2(tx + w, area.top())) };
            let resp = ui.interact(tr, egui::Id::new(("pm-tab", k)), Sense::click());
            resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, on, name.as_str()));
            if !on {
                ui.painter().rect_filled(tr, 0.0, if resp.hovered() { c.tab_hover_bg } else { c.line_light });
                ui.painter().rect_stroke(tr, 0.0, Stroke::new(1.0, c.line_mid), StrokeKind::Inside);
                ui.painter().text(tr.center(), Align2::CENTER_CENTER, name.as_str(), theme.ui_font(), c.text);
            } else {
                active_tab = Some((tr, name.as_str()));
            }
            if resp.clicked() {
                d.tab = k;
            }
            tx += w;
        }
        ui.painter().rect_stroke(area, 0.0, Stroke::new(1.0, c.line_mid), StrokeKind::Inside);
        if let Some((tr, name)) = active_tab {
            ui.painter().rect_filled(Rect::from_min_max(tr.min + vec2(1.0, 1.0), pos2(tr.right() - 1.0, area.top() + 1.0)), 0.0, c.frame_bg);
            ui.painter().text(tr.center() - vec2(0.0, 1.0), Align2::CENTER_CENTER, name, theme.ui_font(), c.text);
            let s = Stroke::new(1.0, c.line_mid);
            ui.painter().vline(tr.left() + 0.5, tr.top()..=area.top(), s);
            ui.painter().vline(tr.right() - 0.5, tr.top()..=area.top(), s);
            ui.painter().hline(tr.left()..=tr.right(), tr.top() + 0.5, s);
        }
        let a = area.min + vec2(10.0, 10.0);
        match d.tab {
            0 => content_page(ui, &mut d, area, &l0),            1 => {
                let vertical = d.flag("vertical", true);
                let cx = 110.0 + dx1;
                super::fit_label(ui, Rect::from_min_size(a, vec2(100.0 + dx1, 20.0)), &l1[0], true);
                // Optionsgruppe = ein Tab-Stopp, Pfeiltasten wählen
                let rects = [Rect::from_min_size(a + vec2(cx, 2.0), vec2(rw[0], 16.0)), Rect::from_min_size(a + vec2(cx + rw[0] + 10.0, 2.0), vec2(rw[1], 16.0))];
                if let Some(k) = super::radio_group(ui, "pm-orient", &rects, &[radios[0].as_str(), radios[1].as_str()], if vertical { 0 } else { 1 }, true) {
                    d.menu.insert("vertical".into(), json!(k == 0));
                }
                super::fit_label(ui, Rect::from_min_size(a + vec2(0.0, 30.0), vec2(100.0 + dx1, 20.0)), &l1[1], true);
                if let Some(n) = fields::combo(ui, "pm-ta", Rect::from_min_size(a + vec2(cx, 30.0), vec2(acw, 20.0)), &refs(&aligns), Some(d.num("text_alignment", 0.0).clamp(0.0, 2.0) as usize), true) {
                    d.menu.insert("text_alignment".into(), json!(n));
                }
                if let Some(b) = fields::checkbox(ui, "pm-bold", Rect::from_min_size(a + vec2(cx + acw + 20.0, 32.0), vec2(bold_w, 16.0)), &bold_l, d.flag("bold", false), true) {
                    d.menu.insert("bold".into(), json!(b));
                }
                for (k, (label, key, def)) in [(&l1[2], "text_up_color", 0x000000u32), (&l1[3], "text_over_color", 0xffffff), (&l1[4], "hilite_color", 0x336699)].iter().enumerate() {
                    let y = 64.0 + k as f32 * 28.0;
                    super::fit_label(ui, Rect::from_min_size(a + vec2(0.0, y), vec2(100.0 + dx1, 20.0)), label, true);
                    let cur = d.color(key, *def);
                    if let Some(crate::inspector::popups::ColorChoice::Color(col)) = crate::inspector::popups::color_button(ui, ("pm-col", k), Rect::from_min_size(a + vec2(cx, y), vec2(26.0, 20.0)), st, Some(cur), true, false, false) {
                        d.menu.insert(key.to_string(), json!(col.0 & 0x00ff_ffff));
                    }
                }
            }
            2 => {
                let nums: [(&str, &str, f64, f64, f64); 6] = [
                    (l2[0].as_str(), "item_spacing", 0.0, 0.0, 100.0),
                    (l2[1].as_str(), "item_padding", 3.0, 0.0, 100.0),
                    (l2[2].as_str(), "text_indent", 0.0, 0.0, 100.0),
                    (l2[3].as_str(), "border_size", 1.0, 0.0, 20.0),
                    (l2[4].as_str(), "hide_timeout", 1000.0, 0.0, 10000.0),
                    (l2[5].as_str(), "menu_width", 0.0, 0.0, 2000.0),
                ];
                for (k, (label, key, def, lo, hi)) in nums.iter().enumerate() {
                    let y = k as f32 * 28.0;
                    super::fit_label(ui, Rect::from_min_size(a + vec2(0.0, y), vec2(150.0 + dx2, 20.0)), label, true);
                    if let Some(v) = fields::number(ui, ("pm-n", k), Rect::from_min_size(a + vec2(160.0 + dx2, y), vec2(60.0, 20.0)), None, Some(d.num(key, *def)), (*lo, *hi), 0, true) {
                        d.menu.insert(key.to_string(), json!(v as i64));
                        if *key == "menu_width" {
                            d.menu.insert("override_width".into(), json!(v > 0.0));
                        }
                    }
                }
                if let Some(b) = fields::checkbox(ui, "pm-hide", Rect::from_min_size(a + vec2(0.0, 172.0), vec2((18.0 + super::text_w(ui.ctx(), &hide_l)).max(260.0), 16.0)), &hide_l, d.flag("hide_on_mouseout", true), true) {
                    d.menu.insert("hide_on_mouseout".into(), json!(b));
                }
                for (k, (label, key, def)) in [(l2[6].as_str(), "border_color", 0x000000u32), (l2[7].as_str(), "shadow_color", 0x666666)].iter().enumerate() {
                    let y = 200.0 + k as f32 * 28.0;
                    super::fit_label(ui, Rect::from_min_size(a + vec2(0.0, y), vec2(150.0 + dx2, 20.0)), label, true);
                    let cur = d.color(key, *def);
                    if let Some(crate::inspector::popups::ColorChoice::Color(col)) = crate::inspector::popups::color_button(ui, ("pm-c2", k), Rect::from_min_size(a + vec2(160.0 + dx2, y), vec2(26.0, 20.0)), st, Some(cur), true, false, false) {
                        d.menu.insert(key.to_string(), json!(col.0 & 0x00ff_ffff));
                    }
                }
            }
            _ => {
                let nums: [(&str, &str); 4] = [(l3[0].as_str(), "horz_offset"), (l3[1].as_str(), "vert_offset"), (l3[2].as_str(), "horz_submenu_offset"), (l3[3].as_str(), "vert_submenu_offset")];
                for (k, (label, key)) in nums.iter().enumerate() {
                    let y = k as f32 * 28.0;
                    super::fit_label(ui, Rect::from_min_size(a + vec2(0.0, y), vec2(150.0 + dx2, 20.0)), label, true);
                    if let Some(v) = fields::number(ui, ("pm-p", k), Rect::from_min_size(a + vec2(160.0 + dx2, y), vec2(60.0, 20.0)), None, Some(d.num(key, 0.0)), (-2000.0, 2000.0), 0, true) {
                        d.menu.insert(key.to_string(), json!(v));
                    }
                }
                if let Some(b) = fields::checkbox(ui, "pm-rel", Rect::from_min_size(a + vec2(0.0, 116.0), vec2((18.0 + super::text_w(ui.ctx(), &rel_l)).max(320.0), 16.0)), &rel_l, d.flag("submenu_relative", false), true) {
                    d.menu.insert("submenu_relative".into(), json!(b));
                }
            }
        }
        // Navigation wie im Original (TW-23): Abbrechen · < Zurück · Weiter > · Fertig; Weiter ist der
        // Standardknopf und blättert die Registerkarten durch, auf der letzten Karte Fertig
        let (back_l, next_l) = (t("dlg-wiz-nav-back"), t("dlg-wiz-nav-next"));
        let last = d.tab + 1 >= tabs.len();
        let b = super::button_row(ui.ctx(), body.right() - 16.0, body.bottom() - 34.0, &[&cancel_l, &back_l, &next_l, &ok_l], 76.0, 23.0, 8.0);
        if push_button(ui, b[0], &cancel_l, false, true) {
            result = Some(false);
        }
        if push_button(ui, b[1], &back_l, false, d.tab > 0) {
            d.tab -= 1;
        }
        if push_button(ui, b[2], &next_l, !last, !last) {
            d.tab += 1;
        }
        if push_button(ui, b[3], &ok_l, last, !d.rows.is_empty()) {
            result = Some(true);
        }
    });
    match (result, r) {
        (Some(true), _) => {
            st.dialogs.popup_editor = None;
            let (cmd, p) = d.invocation();
            if let Err(e) = crate::ui_commands::execute(&mut st.session, cmd, p) {
                st.push(Action::Status(e.to_string()));
            }
        }
        (Some(false), _) | (_, DialogClose::Cancelled) => st.dialogs.popup_editor = None,
        _ => st.dialogs.popup_editor = Some(d),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nesting_by_level() {
        let r = |level, t: &str| Row { level, text: t.into(), url: String::new(), target: String::new() };
        let rows = vec![r(0, "A"), r(1, "A1"), r(1, "A2"), r(0, "B"), r(1, "B1"), r(2, "B1a")];
        let v = nest(&rows);
        assert_eq!(v.len(), 2);
        assert_eq!(v[0]["submenu"].as_array().map(|a| a.len()), Some(2));
        assert_eq!(v[1]["submenu"][0]["submenu"][0]["text"], json!("B1a"));
        let mut back = Vec::new();
        flatten(&v, 0, &mut back);
        assert_eq!(back, rows);
    }
}
