//! Stile-Bedienfeld (`CStylesPanel`, Vorlage 4143): Quelle „Aktuelles Dokument“, Stilraster mit
//! Vorschau (gespeicherte Vorschaubilder bzw. Füll-/Strichprobe), Anwenden per Klick, Umbenennen per
//! Doppelklick, Fußleiste Neuer Stil / Löschen.

use super::common::{self, FooterButton};
use crate::app_state::{Action, AppState};
use crate::inspector::fields;
use crate::inspector::glyphs::Glyph;
use crate::theme::Theme;
use egui::{pos2, vec2, Align2, Color32, ColorImage, Id, Rect, Sense, Stroke, StrokeKind, TextureOptions, Ui};
use serde_json::json;

const RENAME: &str = "styles-rename";

fn preview_texture(ctx: &egui::Context, key: &str, node: &fp_model::BitmapNode) -> Option<egui::TextureHandle> {
    let id = Id::new(("style-prev", key.to_string()));
    if let Some(t) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return Some(t);
    }
    let img = &node.bitmap.image;
    let (rgba, _) = img.to_rgba();
    let (w, h) = (img.grid.width as usize, img.grid.height as usize);
    if w == 0 || h == 0 || rgba.len() < w * h * 4 {
        return None;
    }
    let t = ctx.load_texture(format!("style-{key}"), ColorImage::from_rgba_unmultiplied([w, h], &rgba[..w * h * 4]), TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(id, t.clone()));
    Some(t)
}

/// Zeichnet das Bedienfeld.
pub fn show(ui: &mut Ui, st: &mut AppState) {
    let theme = Theme::current(ui.ctx());
    let c = theme.colors.clone();
    let (head, _) = ui.allocate_exact_size(vec2(ui.available_width(), 26.0), Sense::hover());
    crate::inspector::glyphs::paint(ui.painter(), Rect::from_min_size(head.min + vec2(6.0, 5.0), vec2(16.0, 16.0)), Glyph::Folder, &theme, true);
    let _ = fields::combo(ui, "styles-src", Rect::from_min_size(head.min + vec2(26.0, 3.0), vec2(160.0_f32.min(head.width() - 30.0).max(40.0), 20.0)), &[common::t("panel-styles-source-document").as_str()], Some(0), true);
    let styles: Vec<std::sync::Arc<fp_model::Style>> = st.session.active().and_then(|d| d.doc.styles.clone()).unwrap_or_default();
    let doc_key = st.session.active().map(|d| d.id.0).unwrap_or(0);
    let sel: Option<String> = st.dialogs.panel_sel.styles.first().cloned();
    let multi: Vec<String> = st.dialogs.panel_sel.styles.clone();
    let rename: Option<String> = ui.data(|d| d.get_temp(Id::new(RENAME)));
    let has_selection = st.session.active().is_some_and(|d| !d.selection.elements.is_empty());
    // Bedienfeldmenü „Große Symbole“
    let large = st.session.prefs.get_bool("StylesOptions_LargePreview", false);
    let mut actions = Vec::new();
    common::list_frame(ui, "styles-grid", common::FOOTER_H, |ui| {
        let cell = if large { 72.0 } else { 46.0 };
        let cols = ((ui.available_width() / cell).floor() as usize).max(1);
        let rows = styles.len().div_ceil(cols);
        let (area, _) = ui.allocate_exact_size(vec2(ui.available_width(), rows as f32 * (cell + 14.0) + 4.0), Sense::hover());
        for (i, s) in styles.iter().enumerate() {
            let r = Rect::from_min_size(pos2(area.left() + 2.0 + (i % cols) as f32 * cell, area.top() + 2.0 + (i / cols) as f32 * (cell + 14.0)), vec2(cell - 4.0, cell - 4.0));
            let resp = ui.interact(r, Id::new(("style", i)), Sense::click_and_drag());
            resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, true, fp_i18n::tr!("panel-styles-item", name = s.name.as_str())));
            if resp.drag_started() {
                super::drag_drop::start(ui.ctx(), super::drag_drop::Payload::Style(s.name.clone()));
            }
            ui.painter().rect_filled(r, 0.0, Color32::WHITE);
            let prev = s.previews.last().and_then(|n| preview_texture(ui.ctx(), &format!("{doc_key}-{}-{i}", s.name), n));
            match prev {
                Some(t) => {
                    let ts = t.size_vec2();
                    let k = (r.width() / ts.x).min(r.height() / ts.y).min(1.0);
                    ui.painter().image(t.id(), Rect::from_center_size(r.center(), ts * k), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                }
                None => {
                    let a = s.attrs.as_deref();
                    let inner = r.shrink(8.0);
                    if let Some(a) = a {
                        if a.fill.is_some() {
                            ui.painter().rect_filled(inner, 3.0, Color32::from_rgb(a.fill_color.r(), a.fill_color.g(), a.fill_color.b()));
                        }
                        if let Some(b) = &a.brush {
                            ui.painter().rect_stroke(inner, 3.0, Stroke::new((b.diameter as f32).clamp(1.0, 4.0), Color32::from_rgb(a.stroke_color.r(), a.stroke_color.g(), a.stroke_color.b())), StrokeKind::Middle);
                        }
                    } else {
                        ui.painter().text(r.center(), Align2::CENTER_CENTER, "Aa", theme.bold(16.0), c.text);
                    }
                }
            }
            let selected = multi.contains(&s.name);
            ui.painter().rect_stroke(r, 0.0, Stroke::new(if selected { 2.0 } else { 1.0 }, if selected { c.selection } else { c.list_border }), StrokeKind::Inside);
            let lr = Rect::from_min_size(pos2(r.left() - 1.0, r.bottom() + 1.0), vec2(cell - 2.0, 12.0));
            if rename.as_deref() == Some(s.name.as_str()) {
                if let Some(res) = common::inline_rename(ui, Id::new(("style-rename", i)), lr, &s.name) {
                    if let Some(n) = res {
                        actions.push(Action::Run("pi.style.rename".into(), json!({"name": s.name, "newName": n})));
                    }
                    ui.data_mut(|d| d.remove::<String>(Id::new(RENAME)));
                }
            } else {
                crate::widgets::text_clipped(ui.painter(), lr.center(), Align2::CENTER_CENTER, &s.name, theme.small_font(), c.text, lr.width());
            }
            let resp = resp.on_hover_text(&s.name);
            if resp.clicked() {
                if ui.input(|i| i.modifiers.command) {
                    actions.push(Action::Status(format!("\u{1}toggle-style:{}", s.name)));
                } else {
                    actions.push(Action::Status(format!("\u{1}select-style:{}", s.name)));
                }
                if has_selection && !ui.input(|i| i.modifiers.command) {
                    actions.push(Action::Run("pi.style.apply".into(), json!({"name": s.name})));
                }
            }
            if resp.double_clicked() {
                ui.data_mut(|d| d.insert_temp(Id::new(RENAME), s.name.clone()));
            }
        }
        if styles.is_empty() {
            let r = Rect::from_min_max(area.left_top() + vec2(8.0, 8.0), pos2(area.right() - 4.0, area.top() + 22.0));
            common::fit_text(ui, r, Align2::LEFT_TOP, &common::t("panel-styles-empty"), theme.small_font(), c.text_disabled);
        }
    });
    let (tip_new, tip_del) = (common::t("panel-styles-new"), common::t("panel-styles-delete"));
    let buttons = [
        FooterButton { glyph: Glyph::StyleNew, tip: &tip_new, enabled: has_selection, action: Some(Action::Run("panel.styles.new".into(), json!({}))) },
        FooterButton { glyph: Glyph::Trash, tip: &tip_del, enabled: sel.is_some(), action: sel.clone().map(|_| Action::Run("panel.styles.delete".into(), json!({"names": multi.clone()}))) },
    ];
    if let Some(a) = common::footer(ui, 8.0, "", &buttons) {
        actions.push(a);
    }
    for a in actions {
        if let Action::Status(s) = &a {
            if let Some(n) = s.strip_prefix("\u{1}select-style:") {
                st.dialogs.panel_sel.styles = vec![n.to_string()];
                continue;
            }
            if let Some(n) = s.strip_prefix("\u{1}toggle-style:") {
                let v = &mut st.dialogs.panel_sel.styles;
                if let Some(i) = v.iter().position(|x| x == n) {
                    v.remove(i);
                } else {
                    v.push(n.to_string());
                }
                continue;
            }
        }
        if let Action::Run(n, _) = &a
            && n == "panel.styles.new"
        {
            st.dialogs.open_new_style();
            continue;
        }
        st.push(a);
    }
}
