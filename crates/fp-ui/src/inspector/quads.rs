//! Quadranten: Leervorlagen, Dokument (14100/14031/6564), Werkzeug (14006), Auswahltyp
//! (14101/14102) und B/H/X/Y (14007). Lage in DLU wie in `dialoge.json`.

use super::fields::{self, Dl};
use super::QuadrantRegistry;
use crate::app_state::{Action, AppState};
use crate::icons::{self, IconStyle};
use crate::theme::{Theme, ToolGroup};
use crate::widgets;
use egui::{pos2, vec2, Align2, Color32, Rect, Stroke, StrokeKind, Ui};
use serde_json::json;

pub(super) fn register(r: &mut QuadrantRegistry) {
    for empty in [14079, 14080, 14081, 14082, 14083, 14084] {
        r.register(empty, |_, _, _| {});
    }
    r.register(14100, document_q1);
    r.register(14031, document_q3);
    r.register(6564, document_q4);
    r.register(14006, tool_q1);
    r.register(14101, selection_q1);
    r.register(14102, multi_q1);
    r.register(14007, box_q2);
}

fn thumb_frame(ui: &Ui, r: Rect) {
    let theme = Theme::current(ui.ctx());
    widgets::checker(ui.painter(), r, 5.0, &theme);
    ui.painter().rect_stroke(r, 0.0, Stroke::new(1.0, theme.colors.list_border), StrokeKind::Inside);
}

/// Dokument: Miniatur, „Dokument“, Dokumentname.
fn document_q1(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let d = Dl::at(r.min + vec2(6.0, 0.0));
    let thumb = d.r(0.0, 3.0, 27.0, 23.0);
    thumb_frame(ui, thumb);
    let Some(doc) = st.session.active() else { return };
    let bg = doc.doc.pages.get(doc.page).map(|p| p.root.background).unwrap_or(fp_color::Argb(0xffff_ffff));
    if (bg.0 >> 24) > 0 {
        ui.painter().rect_filled(thumb.shrink(1.0), 0.0, Color32::from_rgb((bg.0 >> 16) as u8, (bg.0 >> 8) as u8, bg.0 as u8));
    }
    if let Some(tex) = st.canvases.get(&doc.id).and_then(|cc| cc.texture.clone()) {
        let ts = tex.size_vec2();
        let s = (thumb.width() / ts.x).min(thumb.height() / ts.y);
        let ir = Rect::from_center_size(thumb.center(), ts * s);
        ui.painter().image(tex.id(), ir, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
    }
    ui.painter().text(pos2(thumb.right() + 6.0, d.r(0.0, 3.0, 1.0, 8.0).center().y), Align2::LEFT_CENTER, "Dokument", theme.ui_font(), c.text);
    let name = doc.title.trim_end_matches(".png").trim_end_matches(".fw").to_string();
    widgets::text_clipped(ui.painter(), pos2(thumb.right() + 6.0, d.r(0.0, 16.0, 1.0, 8.0).center().y), Align2::LEFT_CENTER, &name, theme.ui_font(), c.text, r.right() - thumb.right() - 10.0);
}

/// Dokument Zeile 1 (14031): Leinwandfarbe, Leinwandgröße…, Bildgröße…, Leinwand anpassen;
/// Standardoptionen für Export.
fn document_q3(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let theme = Theme::current(ui.ctx());
    let d = Dl::at(r.min + vec2(6.0, 2.0));
    let en = st.session.active().is_some();
    fields::label(ui, d.r(0.0, 2.0, 41.0, 8.0), "Leinwand:", en);
    let bg = st.session.active().and_then(|d| d.doc.pages.get(d.page).map(|p| p.root.background)).unwrap_or(fp_color::Argb(0xffff_ffff));
    match super::popups::color_button(ui, "pi-canvas-color", d.r(45.0, 0.0, 16.0, 13.0), st, (bg.a() > 0).then_some(bg), en, true, false) {
        Some(super::popups::ColorChoice::Color(c)) => st.run("modify.canvas.canvasColor", json!({"color": crate::ui_commands::hex(c)})),
        Some(super::popups::ColorChoice::None) => st.run("modify.canvas.canvasColor", json!({"color": "#ffffff00"})),
        _ => {}
    }
    for (label, cmd, rr) in [("Leinwandgröße...", "modify.canvas.canvasSize", d.r(66.0, 0.0, 75.0, 12.0)), ("Bildgröße...", "modify.canvas.imageSize", d.r(146.0, 0.0, 75.0, 12.0)), ("Leinwand anpassen", "modify.canvas.fitCanvas", d.r(230.0, 0.0, 84.0, 12.0))] {
        let available = st.session.is_available(cmd);
        if fields::button(ui, ("pi-doc", cmd), rr, label, available) {
            st.push(Action::Run(cmd.into(), json!({})));
        }
    }
    // Standardoptionen für Export (14755) mit Symbol (14754)
    icons::paint(ui.painter(), d.r(28.0, 17.0, 12.0, 11.0), icons::Icon::Export, &IconStyle::new(&theme, Some(ToolGroup::Web), en));
    let presets = fp_export::options::builtin_presets();
    let names: Vec<&str> = presets.iter().map(|p| p.name).collect();
    let cur = st.session.active().map(|d| fp_export::doc_export::page_settings(&d.doc, d.page));
    let sel = cur.as_ref().and_then(crate::panels::optimize::preset_index);
    let text = match (sel, cur.as_ref()) {
        (Some(i), _) => names[i].to_string(),
        (None, Some(c)) => crate::panels::optimize::settings_label(c),
        _ => String::new(),
    };
    let resp = fields::combo_box_frame(ui, egui::Id::new("pi-export-preset"), d.r(45.0, 16.0, 178.0, 13.0), &text, en);
    let mut chosen = None;
    egui::Popup::menu(&resp).frame(crate::menu::menu_frame(&theme)).width(d.r(0.0, 0.0, 178.0, 1.0).width()).show(|ui| {
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        for (i, n) in names.iter().enumerate() {
            if fields::list_item(ui, n, Some(i) == sel).clicked() {
                chosen = Some(i);
                ui.close();
            }
        }
    });
    if let Some(i) = chosen {
        crate::panels::optimize::apply_settings(st, &presets[i].settings);
    }
    if fields::button(ui, "pi-export-edit", d.r(227.0, 17.0, 51.0, 12.0), "Bearbeiten...", en) {
        st.push(Action::Run("file.imagePreview".into(), json!({})));
    }
}

/// Dokument Zeile 2 (6564): Status.
fn document_q4(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let Some(doc) = st.session.active() else { return };
    let (n, frame) = (doc.frame_count(), doc.frame);
    let d = Dl::at(r.min + vec2(6.0, 4.0));
    fields::label(ui, d.r(9.0, 4.0, 24.0, 8.0), "Status:", true);
    let names: Vec<String> = (0..n).map(|i| crate::panels::state_name(st, i)).collect();
    let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
    if let Some(i) = fields::combo(ui, "pi-state", d.r(37.0, 2.0, 119.0, 13.0), &refs, Some(frame), true)
        && i != frame
    {
        st.push(Action::SetFrame(i));
    }
}

/// Werkzeug: Symbol und Name (14006).
fn tool_q1(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let theme = Theme::current(ui.ctx());
    let id = st.session.tools.active;
    let info = fp_engine::tools::info(id);
    let group = info.map(|i| ToolGroup::from_section(i.section));
    let d = Dl::at(r.min + vec2(6.0, 0.0));
    icons::paint(ui.painter(), d.r(0.0, 5.0, 14.0, 13.0), icons::tool_icon(id), &IconStyle::new(&theme, group, true));
    let label = info.map(|i| i.label).unwrap_or("");
    let lr = d.r(21.0, 1.0, 54.0, 26.0);
    widgets::text_clipped(ui.painter(), pos2(lr.left(), lr.top() + 8.0), Align2::LEFT_CENTER, label, theme.small_font(), theme.colors.text, lr.width());
}

/// Auswahl (14101): Miniatur, Typtext, Objektname (editierbar).
fn selection_q1(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let Some(doc) = st.session.active() else { return };
    let (doc_id, doc_gen) = (doc.id, doc.generation);
    let Some(e) = doc.selection.elements.first().and_then(|id| fp_engine::docops::element_on_page(&doc.doc, doc.page, *id)) else { return };
    let docref = std::sync::Arc::clone(&doc.doc);
    let d = Dl::at(r.min + vec2(6.0, 0.0));
    let thumb = d.r(0.0, 3.0, 27.0, 23.0);
    thumb_frame(ui, thumb);
    if let Some(tex) = crate::panels::thumbs::element_thumb(ui.ctx(), st, doc_id, doc_gen, &docref, &e, 64) {
        let ts = tex.size_vec2();
        let s = (thumb.width() / ts.x).min(thumb.height() / ts.y).min(1.0) * 0.95;
        let ir = Rect::from_center_size(thumb.center(), ts * s);
        ui.painter().image(tex.id(), ir, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
    }
    let label = super::obj_quads::type_label(&e);
    let label = match &e.kind {
        fp_model::ElementKind::Instance(i) => match st.session.active().and_then(|d| d.doc.symbol_by_id(i.symbol_id)).map(|(_, s)| s.kind()) {
            Some(fp_model::SymbolKind::Button) => "Schaltfläche".to_string(),
            Some(fp_model::SymbolKind::Animation) => "Animation".to_string(),
            _ => label,
        },
        _ => label,
    };
    ui.painter().text(pos2(thumb.right() + 5.0, d.r(0.0, 3.0, 1.0, 10.0).center().y), Align2::LEFT_CENTER, label, theme.ui_font(), c.text);
    let name = e.common.name.clone().unwrap_or_default();
    if let Some(n) = fields::text(ui, "pi-objname", d.r(30.0, 14.0, 46.0, 12.0), &name, true) {
        st.run("element.setName", json!({"name": n}));
    }
}

/// Mehrfachauswahl (14102): „n Objekte“.
fn multi_q1(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let theme = Theme::current(ui.ctx());
    let n = st.session.active().map(|d| d.selection.elements.len()).unwrap_or(0);
    let d = Dl::at(r.min + vec2(6.0, 0.0));
    let thumb = d.r(0.0, 3.0, 27.0, 23.0);
    thumb_frame(ui, thumb);
    ui.painter().text(pos2(thumb.right() + 6.0, thumb.center().y), Align2::LEFT_CENTER, format!("{n} Objekte"), theme.ui_font(), theme.colors.text);
}

/// Obergrenze der B/H-Felder des Inspektors (Original: 6000).
pub const BOX_MAX: f64 = 6000.0;

/// B/H/X/Y der Auswahl mit „Proportionen beibehalten“ (14007).
fn box_q2(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let theme = Theme::current(ui.ctx());
    let b = st.session.active().and_then(|d| fp_engine::docops::ids_bounds(&d.doc, d.page, &d.selection.elements));
    let en = b.is_some();
    let (w, h, x, y) = b.map(|b| (b.width() as f64, b.height() as f64, b.left as f64, b.top as f64)).unwrap_or((0.0, 0.0, 0.0, 0.0));
    let d = Dl::at(r.min + vec2(6.0, 2.0));
    let key = egui::Id::new("pi-box-prop");
    let prop: bool = ui.data(|dd| dd.get_temp(key)).unwrap_or(false);
    let link = d.r(-2.0, 0.0, 7.0, 27.0);
    let lresp = ui.interact(link, key.with("btn"), egui::Sense::click()).on_hover_text("Proportionen beibehalten");
    icons::paint(ui.painter(), link, icons::Icon::LinkProportions, &IconStyle::new(&theme, None, en));
    if prop {
        ui.painter().rect_stroke(link, 1.0, Stroke::new(1.0, theme.colors.focus), StrokeKind::Inside);
    }
    if lresp.clicked() {
        ui.data_mut(|dd| dd.insert_temp(key, !prop));
    }
    fields::label(ui, d.r(7.0, 3.0, 10.0, 8.0), "B:", en);
    fields::label(ui, d.r(9.0, 16.0, 8.0, 8.0), "H:", en);
    fields::label(ui, d.r(43.0, 3.0, 8.0, 8.0), "X:", en);
    fields::label(ui, d.r(43.0, 16.0, 8.0, 8.0), "Y:", en);
    let val = |v: f64| en.then_some(v.round());
    // B/H wie im Original 1 … 6000; zu große Werte werden begrenzt und gemeldet (nicht still)
    let limit = |st: &mut AppState, v: f64| -> f64 {
        let c = v.clamp(1.0, BOX_MAX);
        if c != v {
            st.push(crate::app_state::Action::Status(format!("Der Wert muss zwischen 1 und {BOX_MAX} liegen; auf {c} begrenzt.")));
        }
        c
    };
    if let Some(v) = fields::number(ui, "pi-box-w", d.r(17.0, 1.0, 24.0, 12.0), None, val(w), (f64::MIN, f64::MAX), 0, en) {
        let v = limit(st, v);
        let p = if prop && w > 0.0 { json!({"width": v, "height": (h * v / w).round()}) } else { json!({"width": v}) };
        st.run("element.setBounds", p);
    }
    if let Some(v) = fields::number(ui, "pi-box-h", d.r(17.0, 15.0, 24.0, 12.0), None, val(h), (f64::MIN, f64::MAX), 0, en) {
        let v = limit(st, v);
        let p = if prop && h > 0.0 { json!({"height": v, "width": (w * v / h).round()}) } else { json!({"height": v}) };
        st.run("element.setBounds", p);
    }
    if let Some(v) = fields::number(ui, "pi-box-x", d.r(52.0, 1.0, 24.0, 12.0), None, val(x), (-30000.0, 30000.0), 0, en) {
        st.run("element.setBounds", json!({"x": v}));
    }
    if let Some(v) = fields::number(ui, "pi-box-y", d.r(52.0, 15.0, 24.0, 12.0), None, val(y), (-30000.0, 30000.0), 0, en) {
        st.run("element.setBounds", json!({"y": v}));
    }
}
