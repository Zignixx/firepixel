//! Bedienfelder „Automatische Formen“ und „Eigenschaften für automatische Form“ (11 §2.4).
//!
//! Die Formen selbst sind Skripte; die Laufzeit liefert fp-script, die Befehle stehen in fp-engine
//! (`autoShape.list/insert/describe/setParams`). Das Bedienfeld zeigt alle Formen der Gruppe
//! „panel“ und „tool“ mit einer Vorschau, die einmalig in einer eigenen Sitzung gezeichnet wird;
//! Doppelklick bzw. „Einfügen“ setzt die Form in die Leinwandmitte (ein Verlaufsschritt). Die
//! Eigenschaften zeigen die Parameter der ausgewählten Form (Zahl, Ganzzahl, Schalter, Auswahl,
//! Text, Farbe); jede Änderung ist `autoShape.setParams` (ein Verlaufsschritt).

use super::common::{self, FooterButton};
use crate::app_state::{Action, AppState};
use crate::inspector::fields;
use crate::inspector::glyphs::Glyph;
use crate::theme::Theme;
use egui::{pos2, vec2, Align2, Color32, Id, Rect, Sense, Stroke, StrokeKind, TextureHandle, Ui};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Eintrag der Formenliste.
#[derive(Clone, Debug, PartialEq)]
pub struct ShapeInfo {
    pub name: String,
    pub label: String,
    pub group: String,
}

/// Liste der Formen (einmal von fp-script gelesen).
pub fn shape_list(st: &mut AppState) -> Vec<ShapeInfo> {
    static LIST: std::sync::OnceLock<Vec<ShapeInfo>> = std::sync::OnceLock::new();
    if let Some(l) = LIST.get() {
        return l.clone();
    }
    let v = crate::ui_commands::execute(&mut st.session, "autoShape.list", json!({})).map(|o| o.value).unwrap_or(Value::Null);
    let list: Vec<ShapeInfo> = v
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| Some(ShapeInfo { name: x.get("name")?.as_str()?.to_string(), label: x.get("label").and_then(|l| l.as_str()).unwrap_or("").to_string(), group: x.get("group").and_then(|g| g.as_str()).unwrap_or("").to_string() }))
                .collect()
        })
        .unwrap_or_default();
    if !list.is_empty() {
        let _ = LIST.set(list.clone());
    }
    list
}

/// Schlüssel einer Form (I18N-MENUE, `werkzeuge.ftl`): `shape-<englischer Name klein, Leerzeichen → ->`,
/// z. B. „Rounded Rectangle“ → `shape-rounded-rectangle`, „3D Box“ → `shape-3d-box`.
pub fn shape_key(name: &str) -> String {
    let slug: String = name.trim().to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
    let mut out = String::from("shape-");
    for part in slug.split('-').filter(|p| !p.is_empty()) {
        if !out.ends_with('-') {
            out.push('-');
        }
        out.push_str(part);
    }
    out
}

/// Anzeigename einer Form: Übersetzung über [`shape_key`], sonst der Name aus fp-script.
pub fn shape_label(name: &str, fallback: &str) -> String {
    let key = shape_key(name);
    if fp_i18n::has(&key) {
        return fp_i18n::t(&key);
    }
    if fallback.is_empty() { name.to_string() } else { fallback.to_string() }
}



/// Übersetzte Parameterbeschriftung (unbekannte Beschriftungen bleiben, wie sie sind).
pub fn param_label(label: &str) -> String {
    fp_script::autoshape::shapes::label_key(label).map(fp_i18n::t).unwrap_or_else(|| label.to_string())
}

/// Übersetzter Auswahlwert (Einheiten wie „px“ bleiben).
pub fn option_label(value: &str) -> String {
    fp_script::autoshape::shapes::option_key(value).map(fp_i18n::t).unwrap_or_else(|| value.to_string())
}

type PreviewCache = Arc<Mutex<HashMap<String, Option<TextureHandle>>>>;

fn previews(ctx: &egui::Context) -> PreviewCache {
    let id = Id::new("autoshape-previews");
    if let Some(c) = ctx.data(|d| d.get_temp::<PreviewCache>(id)) {
        return c;
    }
    let c: PreviewCache = Arc::new(Mutex::new(HashMap::new()));
    ctx.data_mut(|d| d.insert_temp(id, Arc::clone(&c)));
    c
}

/// Vorschau einer Form: einmal in einer eigenen Sitzung eingefügt und gezeichnet.
fn preview(ctx: &egui::Context, st: &AppState, name: &str, budget: &mut u32) -> Option<TextureHandle> {
    let cache = previews(ctx);
    let mut c = cache.lock().ok()?;
    if let Some(t) = c.get(name) {
        return t.clone();
    }
    if *budget == 0 {
        ctx.request_repaint();
        return None;
    }
    *budget -= 1;
    let mut s = fp_engine::Session::default();
    fp_script::install_host(&mut s);
    let ok = s.execute("file.new", json!({"width": 64, "height": 64})).is_ok() && s.execute("autoShape.insert", json!({"shape": name, "rect": [6, 6, 58, 58]})).is_ok();
    let tex = if ok {
        s.active().map(|d| st.renderer.render_page(&d.doc, 0, 0).image).filter(|img| img.width() > 0).map(|img| {
            let ci = egui::ColorImage::from_rgba_unmultiplied([img.width(), img.height()], img.as_bytes());
            ctx.load_texture(format!("autoshape-{name}"), ci, egui::TextureOptions::LINEAR)
        })
    } else {
        None
    };
    c.insert(name.to_string(), tex.clone());
    tex
}

/// Bedienfeld „Automatische Formen“.
pub fn panel(ui: &mut Ui, st: &mut AppState) {
    let theme = Theme::current(ui.ctx());
    let c = theme.colors.clone();
    let list = shape_list(st);
    let sel_key = Id::new("autoshape-sel");
    let mut sel: Option<String> = ui.data(|d| d.get_temp(sel_key));
    let mut insert: Option<String> = None;
    let has_doc = st.session.active().is_some();
    if list.is_empty() {
        common::not_available(ui, &common::t("panel-autoshapes-unavailable"));
        return;
    }
    let mut budget = 3u32;
    common::list_frame(ui, "autoshape-grid", common::FOOTER_H, |ui| {
        let cell = 60.0;
        let cols = ((ui.available_width() / cell).floor() as usize).max(1);
        let rows = list.len().div_ceil(cols);
        let (area, _) = ui.allocate_exact_size(vec2(ui.available_width(), rows as f32 * (cell + 14.0) + 4.0), Sense::hover());
        for (i, s) in list.iter().enumerate() {
            let r = Rect::from_min_size(pos2(area.left() + 4.0 + (i % cols) as f32 * cell, area.top() + 4.0 + (i / cols) as f32 * (cell + 14.0)), vec2(cell - 8.0, cell - 8.0));
            let resp = ui.interact(r, Id::new(("autoshape", i)), Sense::click_and_drag());
            if resp.drag_started() && has_doc {
                super::drag_drop::start(ui.ctx(), super::drag_drop::Payload::AutoShape(s.name.clone()));
            }
            let selected = sel.as_deref() == Some(s.name.as_str());
            ui.painter().rect_filled(r, 2.0, Color32::WHITE);
            if let Some(t) = preview(ui.ctx(), st, &s.name, &mut budget) {
                ui.painter().image(t.id(), r.shrink(2.0), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
            }
            ui.painter().rect_stroke(r, 2.0, Stroke::new(if selected { 2.0 } else { 1.0 }, if selected { c.selection } else { c.list_border }), StrokeKind::Inside);
            let label = shape_label(&s.name, &s.label);
            crate::widgets::text_clipped(ui.painter(), pos2(r.center().x, r.bottom() + 7.0), Align2::CENTER_CENTER, &label, theme.small_font(), c.text, cell);
            let resp = resp.on_hover_text(label);
            if resp.clicked() {
                sel = Some(s.name.clone());
            }
            if resp.double_clicked() && has_doc {
                insert = Some(s.name.clone());
            }
        }
    });
    ui.data_mut(|d| match &sel {
        Some(s) => {
            d.insert_temp(sel_key, s.clone());
        }
        None => d.remove::<String>(sel_key),
    });
    let tip_insert = common::t("panel-autoshapes-insert");
    let buttons = [FooterButton { glyph: Glyph::Play, tip: &tip_insert, enabled: has_doc && sel.is_some(), action: sel.clone().map(|n| Action::Run("autoShape.insert".into(), json!({"shape": n}))) }];
    if let Some(a) = common::footer(ui, 8.0, "", &buttons) {
        st.push(a);
    }
    if let Some(n) = insert {
        st.run("autoShape.insert", json!({"shape": n}));
    }
}

/// Ausgewählte automatische Form (Gruppe mit Formdaten).
pub fn selected_shape(st: &AppState) -> Option<u64> {
    let d = st.session.active()?;
    if d.selection.elements.len() != 1 {
        return None;
    }
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, d.selection.elements[0])?;
    match &e.kind {
        fp_model::ElementKind::Group(g) if g.smart_shape.is_some() => Some(e.id.0),
        _ => None,
    }
}

/// Bedienfeld „Eigenschaften für automatische Form“.
pub fn properties(ui: &mut Ui, st: &mut AppState) {
    let theme = Theme::current(ui.ctx());
    let Some(id) = selected_shape(st) else {
        common::not_available(ui, &common::t("panel-asp-none"));
        return;
    };
    let generation = st.session.active().map(|d| d.generation).unwrap_or(0);
    let key = Id::new("autoshape-describe");
    let cached: Option<(u64, u64, Value)> = ui.data(|d| d.get_temp(key));
    let desc = match cached {
        Some((g, i, v)) if g == generation && i == id => v,
        _ => {
            let v = crate::ui_commands::execute(&mut st.session, "autoShape.describe", json!({"id": id})).map(|o| o.value).unwrap_or(Value::Null);
            ui.data_mut(|d| d.insert_temp(key, (generation, id, v.clone())));
            v
        }
    };
    let de_label = desc.get("label").and_then(|v| v.as_str()).unwrap_or("");
    let title = match desc.get("shape").and_then(|v| v.as_str()) {
        Some(name) => shape_label(name, de_label),
        None if !de_label.is_empty() => de_label.to_string(),
        None => common::t("panel-asp-default-title"),
    };
    // gleiche Höhe wie die bisherige Beschriftung (eine Zeile der fetten Schrift), lange Namen mit „…“
    let title_font = theme.bold(theme.fonts.ui);
    let row_h = ui.ctx().fonts_mut(|f| f.layout_no_wrap(title.clone(), title_font.clone(), theme.colors.text).size().y);
    let (tr_, _) = ui.allocate_exact_size(vec2(ui.available_width(), row_h), Sense::hover());
    common::fit_text(ui, tr_, Align2::LEFT_CENTER, &title, title_font, theme.colors.text);
    let params = desc.get("params").and_then(|p| p.as_array()).cloned().unwrap_or_default();
    if params.is_empty() {
        common::not_available(ui, &common::t("panel-asp-no-params"));
        return;
    }
    // Beschriftungsspalte: bis 120 px, bei schmalen Bedienfeldern weniger (Text mit „…“)
    let label_w = (ui.available_width() - 100.0).clamp(50.0, 120.0);
    let mut change: Option<(String, Value)> = None;
    for (i, p) in params.iter().enumerate() {
        let k = p.get("key").and_then(|v| v.as_str()).unwrap_or("").to_string();
        // Beschriftung über den Schlüssel aus fp-script (`labelKey`), sonst der gelieferte Text
        let label = match p.get("labelKey").and_then(|v| v.as_str()) {
            Some(key) => fp_i18n::t(key),
            None => param_label(p.get("label").and_then(|v| v.as_str()).unwrap_or(&k)),
        };
        let kind = p.get("type").and_then(|v| v.as_str()).unwrap_or("number");
        let value = p.get("value").cloned().unwrap_or(Value::Null);
        let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), 24.0), Sense::hover());
        let lr = Rect::from_min_size(row.min + vec2(4.0, 2.0), vec2(label_w, 20.0));
        let cr = Rect::from_min_max(pos2(row.left() + label_w + 8.0, row.top() + 2.0), pos2(row.right() - 4.0, row.bottom() - 2.0));
        match kind {
            "bool" => {
                let on = value.as_bool().unwrap_or_else(|| value.as_str() == Some("true"));
                let cbr = Rect::from_min_size(row.min + vec2(4.0, 4.0), vec2(row.width() - 8.0, 16.0));
                if let Some(b) = fields::checkbox(ui, ("as-b", i), cbr, "", on, true) {
                    change = Some((k, json!(b)));
                }
                common::label_fit(ui, Rect::from_min_max(pos2(cbr.left() + 18.0, cbr.top()), cbr.max), &label, true);
            }
            "choice" => {
                common::label_fit(ui, lr, &label, true);
                let opts: Vec<String> = p.get("options").and_then(|o| o.as_array()).map(|a| a.iter().map(|x| x.as_str().map(|s| s.to_string()).unwrap_or_else(|| x.to_string())).collect()).unwrap_or_default();
                let keys = p.get("optionKeys").and_then(|o| o.as_array());
                let shown: Vec<String> = opts.iter().enumerate().map(|(j, o)| keys.and_then(|a| a.get(j)).and_then(|v| v.as_str()).map(fp_i18n::t).unwrap_or_else(|| option_label(o))).collect();
                let refs: Vec<&str> = shown.iter().map(|s| s.as_str()).collect();
                let cur = value.as_str().map(|s| s.to_string()).unwrap_or_else(|| value.to_string());
                let sel = opts.iter().position(|o| *o == cur);
                if let Some(n) = fields::combo(ui, ("as-c", i), Rect::from_min_size(cr.min, vec2(cr.width().min(160.0), 20.0)), &refs, sel, true) {
                    change = Some((k, json!(opts[n])));
                }
            }
            "color" => {
                common::label_fit(ui, lr, &label, true);
                let cur = value.as_str().and_then(fp_color::Rgba8::parse_hex).map(|c| c.to_argb());
                if let Some(crate::inspector::popups::ColorChoice::Color(col)) = crate::inspector::popups::color_button(ui, ("as-col", i), Rect::from_min_size(cr.min, vec2(26.0, 20.0)), st, cur, true, false, false) {
                    change = Some((k, json!(crate::ui_commands::hex(col))));
                }
            }
            "text" => {
                common::label_fit(ui, lr, &label, true);
                if let Some(t) = fields::text(ui, ("as-t", i), Rect::from_min_size(cr.min, vec2(cr.width().min(200.0), 20.0)), value.as_str().unwrap_or(""), true) {
                    change = Some((k, json!(t)));
                }
            }
            _ => {
                common::label_fit(ui, lr, &label, true);
                let min = p.get("min").and_then(|v| v.as_f64()).unwrap_or(-100000.0);
                let max = p.get("max").and_then(|v| v.as_f64()).unwrap_or(100000.0);
                let dec = if kind == "integer" { 0 } else { 2 };
                let cur = value.as_f64().or_else(|| value.as_str().and_then(|s| s.parse().ok()));
                if let Some(v) = fields::number(ui, ("as-n", i), Rect::from_min_size(cr.min, vec2(60.0, 20.0)), Some(Rect::from_min_size(cr.min + vec2(62.0, 0.0), vec2(14.0, 20.0))), cur, (min, max), dec, true) {
                    change = Some((k, if kind == "integer" { json!(v.round() as i64) } else { json!(v) }));
                }
            }
        }
    }
    if let Some((k, v)) = change {
        let mut params = serde_json::Map::new();
        params.insert(k, v);
        st.run("autoShape.setParams", json!({"id": id, "params": params}));
    }
}
