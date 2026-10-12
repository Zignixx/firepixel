//! Ausrichten-Bedienfeld (in CS6 das Flash-Bedienfeld `Align.swf`, nativ nachgebaut, 11 §2.4):
//! „Relativ zum Objekt/zur Leinwand“, Ausrichten (6), Proportionieren (6), Größe (3), Abstand (2)
//! mit „Gleichm.“. Ausgeführt über `pi.align` (Adapter, ein Verlaufsschritt).

use super::common;
use crate::app_state::AppState;
use crate::inspector::fields;
use crate::theme::Theme;
use egui::{pos2, vec2, Color32, Id, Painter, Rect, Sense, Stroke, StrokeKind, Ui};
use serde_json::json;

const TO_CANVAS: &str = "align-to-canvas";

fn bar(p: &Painter, r: Rect, col: Color32) {
    p.rect_filled(r, 0.0, col);
}

/// Symbol einer Ausrichtungsart.
fn paint_mode(p: &Painter, r: Rect, mode: &str, t: &Theme, enabled: bool) {
    let fg = if enabled { t.colors.icon_fg } else { t.colors.icon_disabled };
    let acc = if enabled { t.colors.icon_blue } else { t.colors.icon_disabled };
    let c = r.center();
    let s = Stroke::new(1.0, fg);
    match mode {
        "left" | "hcenter" | "right" => {
            let x = match mode {
                "left" => c.x - 6.0,
                "right" => c.x + 6.0,
                _ => c.x,
            };
            p.vline(x, (c.y - 7.0)..=(c.y + 7.0), s);
            let (w1, w2) = (10.0, 6.0);
            let x1 = |w: f32| match mode {
                "left" => x + 1.0,
                "right" => x - w - 1.0,
                _ => x - w / 2.0,
            };
            bar(p, Rect::from_min_size(pos2(x1(w1), c.y - 5.0), vec2(w1, 4.0)), acc);
            bar(p, Rect::from_min_size(pos2(x1(w2), c.y + 1.0), vec2(w2, 4.0)), acc);
        }
        "top" | "vcenter" | "bottom" => {
            let y = match mode {
                "top" => c.y - 6.0,
                "bottom" => c.y + 6.0,
                _ => c.y,
            };
            p.hline((c.x - 7.0)..=(c.x + 7.0), y, s);
            let y1 = |h: f32| match mode {
                "top" => y + 1.0,
                "bottom" => y - h - 1.0,
                _ => y - h / 2.0,
            };
            bar(p, Rect::from_min_size(pos2(c.x - 5.0, y1(10.0)), vec2(4.0, 10.0)), acc);
            bar(p, Rect::from_min_size(pos2(c.x + 1.0, y1(6.0)), vec2(4.0, 6.0)), acc);
        }
        "distTop" | "distVCenter" | "distBottom" => {
            for k in 0..3 {
                let y = c.y - 6.0 + k as f32 * 6.0;
                p.hline((c.x - 7.0)..=(c.x + 7.0), y, s);
                let off = match mode {
                    "distTop" => 1.0,
                    "distBottom" => -3.0,
                    _ => -1.0,
                };
                bar(p, Rect::from_min_size(pos2(c.x - 4.0, y + off), vec2(8.0, 2.0)), acc);
            }
        }
        "distLeft" | "distHCenter" | "distRight" => {
            for k in 0..3 {
                let x = c.x - 6.0 + k as f32 * 6.0;
                p.vline(x, (c.y - 7.0)..=(c.y + 7.0), s);
                let off = match mode {
                    "distLeft" => 1.0,
                    "distRight" => -3.0,
                    _ => -1.0,
                };
                bar(p, Rect::from_min_size(pos2(x + off, c.y - 4.0), vec2(2.0, 8.0)), acc);
            }
        }
        "matchWidth" => {
            bar(p, Rect::from_min_size(pos2(c.x - 7.0, c.y - 6.0), vec2(14.0, 4.0)), acc);
            bar(p, Rect::from_min_size(pos2(c.x - 7.0, c.y + 2.0), vec2(14.0, 4.0)), acc);
        }
        "matchHeight" => {
            bar(p, Rect::from_min_size(pos2(c.x - 6.0, c.y - 7.0), vec2(4.0, 14.0)), acc);
            bar(p, Rect::from_min_size(pos2(c.x + 2.0, c.y - 7.0), vec2(4.0, 14.0)), acc);
        }
        "matchBoth" => {
            p.rect_stroke(Rect::from_center_size(c, vec2(12.0, 12.0)), 0.0, s, StrokeKind::Inside);
            bar(p, Rect::from_center_size(c, vec2(6.0, 6.0)), acc);
        }
        "spaceH" => {
            bar(p, Rect::from_min_size(pos2(c.x - 7.0, c.y - 4.0), vec2(3.0, 8.0)), acc);
            bar(p, Rect::from_min_size(pos2(c.x - 1.5, c.y - 4.0), vec2(3.0, 8.0)), acc);
            bar(p, Rect::from_min_size(pos2(c.x + 4.0, c.y - 4.0), vec2(3.0, 8.0)), acc);
        }
        _ => {
            bar(p, Rect::from_min_size(pos2(c.x - 4.0, c.y - 7.0), vec2(8.0, 3.0)), acc);
            bar(p, Rect::from_min_size(pos2(c.x - 4.0, c.y - 1.5), vec2(8.0, 3.0)), acc);
            bar(p, Rect::from_min_size(pos2(c.x - 4.0, c.y + 4.0), vec2(8.0, 3.0)), acc);
        }
    }
}

fn mode_button(ui: &mut Ui, st: &mut AppState, r: Rect, mode: &str, tip: &str, enabled: bool, to_canvas: bool) {
    let t = Theme::current(ui.ctx());
    let resp = ui.interact(r, Id::new(("align", mode)), if enabled { Sense::click() } else { Sense::hover() });
    if enabled && resp.hovered() {
        ui.painter().rect_filled(r, 2.0, t.colors.button_hover_bg);
        ui.painter().rect_stroke(r, 2.0, Stroke::new(1.0, t.colors.button_hover_border), StrokeKind::Inside);
    }
    paint_mode(ui.painter(), r, mode, &t, enabled);
    crate::ui_kit::icon_button_info(&resp, enabled, tip, None);
    let resp = resp.on_hover_text(tip);
    if enabled && resp.clicked() {
        st.run("pi.align", json!({"mode": mode, "toCanvas": to_canvas}));
    }
}

/// Zeichnet das Bedienfeld.
pub fn show(ui: &mut Ui, st: &mut AppState) {
    let t = Theme::current(ui.ctx());
    let n = st.session.active().map(|d| d.selection.elements.len()).unwrap_or(0);
    let to_canvas: bool = ui.data(|d| d.get_temp(Id::new(TO_CANVAS))).unwrap_or(false);
    let (area, _) = ui.allocate_exact_size(vec2(ui.available_width(), 200.0), Sense::hover());
    let x0 = area.left() + 10.0;
    let mut y = area.top() + 6.0;
    // Relativ zu
    let rel = common::t(if to_canvas { "panel-align-relative-canvas" } else { "panel-align-relative-object" });
    let tr = Rect::from_min_size(pos2(area.right() - 32.0, y), vec2(24.0, 18.0));
    common::fit_text(ui, Rect::from_min_max(pos2(area.left() + 4.0, tr.top()), pos2(tr.left() - 6.0, tr.bottom())), egui::Align2::RIGHT_CENTER, &rel, t.ui_font(), t.colors.text);
    if fields::toggle(ui, "align-rel", tr, crate::inspector::glyphs::Glyph::FillSolid, to_canvas, true, &common::t("panel-align-to-canvas")) {
        ui.data_mut(|d| d.insert_temp(Id::new(TO_CANVAS), !to_canvas));
    }
    y += 22.0;
    let enabled1 = n >= 1;
    let row = |ui: &mut Ui, st: &mut AppState, y: f32, label: &str, label_w: f32, modes: &[(&str, &str)], enabled: bool, gap_after: usize| {
        let t = Theme::current(ui.ctx());
        common::fit_text(ui, Rect::from_min_size(pos2(x0, y), vec2(label_w, 14.0)), egui::Align2::LEFT_TOP, &common::t(label), t.ui_font(), t.colors.text);
        let mut x = x0;
        for (i, (m, tip)) in modes.iter().enumerate() {
            if i == gap_after {
                x += 14.0;
            }
            mode_button(ui, st, Rect::from_min_size(pos2(x, y + 16.0), vec2(24.0, 22.0)), m, &common::t(tip), enabled, to_canvas);
            x += 28.0;
        }
    };
    let full_w = (area.right() - x0 - 4.0).max(0.0);
    row(ui, st, y, "panel-align-align", full_w, &[("left", "panel-align-left"), ("hcenter", "panel-align-hcenter"), ("right", "panel-align-right"), ("top", "panel-align-top"), ("vcenter", "panel-align-vcenter"), ("bottom", "panel-align-bottom")], enabled1, 3);
    y += 46.0;
    let enabled_dist = n >= 2 || (to_canvas && n >= 1);
    row(ui, st, y, "panel-align-distribute", full_w, &[("distTop", "panel-align-dist-top"), ("distVCenter", "panel-align-dist-vcenter"), ("distBottom", "panel-align-dist-bottom"), ("distLeft", "panel-align-dist-left"), ("distHCenter", "panel-align-dist-hcenter"), ("distRight", "panel-align-dist-right")], enabled_dist, 3);
    y += 46.0;
    let sx = x0 + 3.0 * 28.0 + 20.0;
    row(ui, st, y, "panel-align-size", sx - x0 - 6.0, &[("matchWidth", "panel-align-match-width"), ("matchHeight", "panel-align-match-height"), ("matchBoth", "panel-align-match-both")], enabled1, 9);
    common::fit_text(ui, Rect::from_min_size(pos2(sx, y), vec2((area.right() - sx - 4.0).max(0.0), 14.0)), egui::Align2::LEFT_TOP, &common::t("panel-align-space"), t.ui_font(), t.colors.text);
    let enabled_space = n >= 3 || (to_canvas && n >= 1);
    mode_button(ui, st, Rect::from_min_size(pos2(sx, y + 16.0), vec2(24.0, 22.0)), "spaceV", &common::t("panel-align-space-v"), enabled_space, to_canvas);
    mode_button(ui, st, Rect::from_min_size(pos2(sx + 28.0, y + 16.0), vec2(24.0, 22.0)), "spaceH", &common::t("panel-align-space-h"), enabled_space, to_canvas);
    let _ = fields::combo(ui, "align-space", Rect::from_min_size(pos2(sx, y + 42.0), vec2(80.0, 20.0)), &[common::t("panel-align-space-even").as_str()], Some(0), enabled_space);
}
