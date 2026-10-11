//! Pop-up-Menü-Editor (6283, Modifizieren ▸ Pop-up-Menü ▸ hinzufügen/bearbeiten): Registerkarten
//! Inhalt (Einträge mit Text, Link, Ziel; Einrücken = Untermenü), Erscheinungsbild (Ausrichtung,
//! Textfarben, Hervorhebung, Fett), Erweitert (Abstände, Rahmen, Schatten, Ausblendzeit, feste
//! Größe) und Position (Versatz von Menü und Untermenü). OK führt `modify.popUpMenu.addPopUpMenu`
//! bzw. `…editPopUpMenu` aus (ein Verlaufsschritt), Abbrechen ändert nichts.

use super::{dialog_frame, push_button, DialogClose};
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
            Some(PopupEditor { edit, tab: 0, rows: vec![Row { level: 0, text: "Eintrag 1".into(), url: String::new(), target: String::new() }], selected: Some(0), menu })
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

/// Zeigt den Editor.
pub fn show(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut d) = st.dialogs.popup_editor.clone() else { return };
    let mut result = None;
    let title = if d.edit { "Pop-up-Menü-Editor (bearbeiten)" } else { "Pop-up-Menü-Editor" };
    let r = dialog_frame(ctx, "fp-popup-editor", title, vec2(560.0, 420.0), |ui, body| {
        let theme = Theme::current(ui.ctx());
        let c = &theme.colors;
        let o = body.min + vec2(12.0, 10.0);
        // Registerkarten
        for (k, name) in ["Inhalt", "Erscheinungsbild", "Erweitert", "Position"].iter().enumerate() {
            let tr = Rect::from_min_size(o + vec2(k as f32 * 112.0, 0.0), vec2(108.0, 22.0));
            let resp = ui.interact(tr, egui::Id::new(("pm-tab", k)), Sense::click());
            ui.painter().rect_filled(tr, 0.0, if d.tab == k { c.list_bg } else { c.frame_bg });
            ui.painter().rect_stroke(tr, 0.0, Stroke::new(1.0, c.line_mid), StrokeKind::Inside);
            ui.painter().text(tr.center(), Align2::CENTER_CENTER, *name, theme.ui_font(), c.text);
            if resp.clicked() {
                d.tab = k;
            }
        }
        let area = Rect::from_min_max(o + vec2(0.0, 30.0), pos2(body.right() - 12.0, body.bottom() - 44.0));
        ui.painter().rect_stroke(area, 0.0, Stroke::new(1.0, c.line_mid), StrokeKind::Inside);
        let a = area.min + vec2(10.0, 10.0);
        match d.tab {
            0 => {
                // Liste
                let list = Rect::from_min_size(a, vec2(area.width() - 20.0, 180.0));
                let inner = fields::list_box(ui, list);
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner));
                child.spacing_mut().item_spacing = vec2(0.0, 0.0);
                for (i, row) in d.rows.iter().enumerate() {
                    let label = format!("{}{}    {}    {}", "    ".repeat(row.level), row.text, row.url, row.target);
                    if fields::list_item(&mut child, &label, d.selected == Some(i)).clicked() {
                        d.selected = Some(i);
                    }
                }
                let by = list.bottom() + 6.0;
                let btn = |k: f32| Rect::from_min_size(pos2(a.x + k * 34.0, by), vec2(30.0, 22.0));
                if fields::button(ui, "pm-add", btn(0.0), "+", true) {
                    let level = d.selected.and_then(|i| d.rows.get(i)).map(|r| r.level).unwrap_or(0);
                    let at = d.selected.map(|i| i + 1).unwrap_or(d.rows.len());
                    d.rows.insert(at, Row { level, text: format!("Eintrag {}", d.rows.len() + 1), url: String::new(), target: String::new() });
                    d.selected = Some(at);
                }
                if fields::button(ui, "pm-del", btn(1.0), "−", d.selected.is_some() && d.rows.len() > 1)
                    && let Some(i) = d.selected
                {
                    d.rows.remove(i);
                    d.selected = Some(i.min(d.rows.len().saturating_sub(1)));
                }
                if fields::button(ui, "pm-in", btn(2.0), "→", d.selected.is_some_and(|i| i > 0 && d.rows[i].level <= d.rows[i - 1].level))
                    && let Some(i) = d.selected
                {
                    d.rows[i].level += 1;
                }
                if fields::button(ui, "pm-out", btn(3.0), "←", d.selected.is_some_and(|i| d.rows[i].level > 0))
                    && let Some(i) = d.selected
                {
                    d.rows[i].level -= 1;
                }
                if fields::button(ui, "pm-up", btn(4.0), "↑", d.selected.is_some_and(|i| i > 0))
                    && let Some(i) = d.selected
                {
                    d.rows.swap(i, i - 1);
                    d.selected = Some(i - 1);
                }
                if fields::button(ui, "pm-down", btn(5.0), "↓", d.selected.is_some_and(|i| i + 1 < d.rows.len()))
                    && let Some(i) = d.selected
                {
                    d.rows.swap(i, i + 1);
                    d.selected = Some(i + 1);
                }
                if let Some(i) = d.selected.filter(|i| *i < d.rows.len()) {
                    let fy = by + 32.0;
                    for (k, (label, key)) in [("Text:", 0), ("Link:", 1), ("Ziel:", 2)].iter().enumerate() {
                        let y = fy + k as f32 * 26.0;
                        fields::label(ui, Rect::from_min_size(pos2(a.x, y), vec2(60.0, 20.0)), label, true);
                        let cur = match key {
                            0 => d.rows[i].text.clone(),
                            1 => d.rows[i].url.clone(),
                            _ => d.rows[i].target.clone(),
                        };
                        if let Some(t) = fields::text(ui, ("pm-f", *key, i), Rect::from_min_size(pos2(a.x + 64.0, y), vec2(300.0, 20.0)), &cur, true) {
                            match key {
                                0 => d.rows[i].text = t,
                                1 => d.rows[i].url = t,
                                _ => d.rows[i].target = t,
                            }
                        }
                    }
                }
            }
            1 => {
                let vertical = d.flag("vertical", true);
                fields::label(ui, Rect::from_min_size(a, vec2(100.0, 20.0)), "Ausrichtung:", true);
                if fields::radio(ui, "pm-v", Rect::from_min_size(a + vec2(110.0, 2.0), vec2(110.0, 16.0)), "Vertikal", vertical, true) {
                    d.menu.insert("vertical".into(), json!(true));
                }
                if fields::radio(ui, "pm-h", Rect::from_min_size(a + vec2(230.0, 2.0), vec2(110.0, 16.0)), "Horizontal", !vertical, true) {
                    d.menu.insert("vertical".into(), json!(false));
                }
                fields::label(ui, Rect::from_min_size(a + vec2(0.0, 30.0), vec2(100.0, 20.0)), "Textausrichtung:", true);
                if let Some(n) = fields::combo(ui, "pm-ta", Rect::from_min_size(a + vec2(110.0, 30.0), vec2(120.0, 20.0)), &["Links", "Zentriert", "Rechts"], Some(d.num("text_alignment", 0.0).clamp(0.0, 2.0) as usize), true) {
                    d.menu.insert("text_alignment".into(), json!(n));
                }
                if let Some(b) = fields::checkbox(ui, "pm-bold", Rect::from_min_size(a + vec2(250.0, 32.0), vec2(80.0, 16.0)), "Fett", d.flag("bold", false), true) {
                    d.menu.insert("bold".into(), json!(b));
                }
                for (k, (label, key, def)) in [("Text (Normal):", "text_up_color", 0x000000u32), ("Text (Darüber):", "text_over_color", 0xffffff), ("Hervorhebung:", "hilite_color", 0x336699)].iter().enumerate() {
                    let y = 64.0 + k as f32 * 28.0;
                    fields::label(ui, Rect::from_min_size(a + vec2(0.0, y), vec2(100.0, 20.0)), label, true);
                    let cur = d.color(key, *def);
                    if let Some(crate::inspector::popups::ColorChoice::Color(col)) = crate::inspector::popups::color_button(ui, ("pm-col", k), Rect::from_min_size(a + vec2(110.0, y), vec2(26.0, 20.0)), st, Some(cur), true, false, false) {
                        d.menu.insert(key.to_string(), json!(col.0 & 0x00ff_ffff));
                    }
                }
            }
            2 => {
                let nums: [(&str, &str, f64, f64, f64); 6] = [
                    ("Zellabstand:", "item_spacing", 0.0, 0.0, 100.0),
                    ("Zellauffüllung:", "item_padding", 3.0, 0.0, 100.0),
                    ("Texteinzug:", "text_indent", 0.0, 0.0, 100.0),
                    ("Rahmenbreite:", "border_size", 1.0, 0.0, 20.0),
                    ("Menüverzögerung (ms):", "hide_timeout", 1000.0, 0.0, 10000.0),
                    ("Breite (fest):", "menu_width", 0.0, 0.0, 2000.0),
                ];
                for (k, (label, key, def, lo, hi)) in nums.iter().enumerate() {
                    let y = k as f32 * 28.0;
                    fields::label(ui, Rect::from_min_size(a + vec2(0.0, y), vec2(150.0, 20.0)), label, true);
                    if let Some(v) = fields::number(ui, ("pm-n", k), Rect::from_min_size(a + vec2(160.0, y), vec2(60.0, 20.0)), None, Some(d.num(key, *def)), (*lo, *hi), 0, true) {
                        d.menu.insert(key.to_string(), json!(v as i64));
                        if *key == "menu_width" {
                            d.menu.insert("override_width".into(), json!(v > 0.0));
                        }
                    }
                }
                if let Some(b) = fields::checkbox(ui, "pm-hide", Rect::from_min_size(a + vec2(0.0, 172.0), vec2(260.0, 16.0)), "Menü beim Verlassen ausblenden", d.flag("hide_on_mouseout", true), true) {
                    d.menu.insert("hide_on_mouseout".into(), json!(b));
                }
                for (k, (label, key, def)) in [("Rahmenfarbe:", "border_color", 0x000000u32), ("Schattenfarbe:", "shadow_color", 0x666666)].iter().enumerate() {
                    let y = 200.0 + k as f32 * 28.0;
                    fields::label(ui, Rect::from_min_size(a + vec2(0.0, y), vec2(150.0, 20.0)), label, true);
                    let cur = d.color(key, *def);
                    if let Some(crate::inspector::popups::ColorChoice::Color(col)) = crate::inspector::popups::color_button(ui, ("pm-c2", k), Rect::from_min_size(a + vec2(160.0, y), vec2(26.0, 20.0)), st, Some(cur), true, false, false) {
                        d.menu.insert(key.to_string(), json!(col.0 & 0x00ff_ffff));
                    }
                }
            }
            _ => {
                let nums: [(&str, &str); 4] = [("Menü X:", "horz_offset"), ("Menü Y:", "vert_offset"), ("Untermenü X:", "horz_submenu_offset"), ("Untermenü Y:", "vert_submenu_offset")];
                for (k, (label, key)) in nums.iter().enumerate() {
                    let y = k as f32 * 28.0;
                    fields::label(ui, Rect::from_min_size(a + vec2(0.0, y), vec2(150.0, 20.0)), label, true);
                    if let Some(v) = fields::number(ui, ("pm-p", k), Rect::from_min_size(a + vec2(160.0, y), vec2(60.0, 20.0)), None, Some(d.num(key, 0.0)), (-2000.0, 2000.0), 0, true) {
                        d.menu.insert(key.to_string(), json!(v));
                    }
                }
                if let Some(b) = fields::checkbox(ui, "pm-rel", Rect::from_min_size(a + vec2(0.0, 116.0), vec2(320.0, 16.0)), "Untermenü relativ zum Menüeintrag", d.flag("submenu_relative", false), true) {
                    d.menu.insert("submenu_relative".into(), json!(b));
                }
            }
        }
        if push_button(ui, Rect::from_min_size(pos2(body.right() - 176.0, body.bottom() - 34.0), vec2(76.0, 23.0)), "OK", true, !d.rows.is_empty()) {
            result = Some(true);
        }
        if push_button(ui, Rect::from_min_size(pos2(body.right() - 92.0, body.bottom() - 34.0), vec2(76.0, 23.0)), "Abbrechen", false, true) {
            result = Some(false);
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
