//! Registerkarte „Datei“ der Bildvorschau (re\17-bildvorschau.md §3): Gruppe „Skalieren“ mit
//! %, B, H (Pixel), Kettensymbol und „Proportionen beibehalten“; Gruppe „Exportbereich“ mit
//! Kästchen als Kopf und X/Y/B/H, grau ohne Haken. Werte in Ausgabepixeln ([`super::crop`]); alles
//! gehört zur Einstellung des **aktiven** Feldes und wirkt auf Vorschau und Größenanzeige.

use super::super::{text_w, tl};
use super::crop;
use super::ImagePreview;
use crate::icons::{self, Icon, IconStyle};
use crate::inspector::fields;
use crate::theme::Theme;
use egui::{pos2, vec2, Align2, Id, Rect, Sense, Stroke, Ui};
use fp_i18n::t;

const LABEL_KEYS: [&str; 6] = ["image-preview-file-percent", "image-preview-file-width", "image-preview-file-height", "image-preview-file-x", "image-preview-file-y", "image-preview-file-pixels"];

/// Mindestbreite der Registerkarte.
pub fn min_width(ctx: &egui::Context) -> f32 {
    let check = text_w(ctx, &t("image-preview-file-constrain")).max(text_w(ctx, &t("image-preview-file-export-area")));
    let labels = tl(&LABEL_KEYS).iter().map(|s| text_w(ctx, s)).fold(0.0, f32::max);
    (40.0 + check + 20.0).max(2.0 * (labels + 4.0 + 56.0) + 40.0)
}

/// Tooltip über einem Feld (zugleich Barrierefreiheits-Name).
fn tip(ui: &Ui, id: &str, r: Rect, key: &str) {
    let text = t(key);
    let resp = ui.interact(r, Id::new(("fp-ipv-file-tip", id)), Sense::hover());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, true, text.as_str()));
    resp.on_hover_text(text);
}

/// Gruppenrahmen; `head_w` = Breite des Kopfes (Text bzw. Kästchen), der im oberen Strich steht.
fn frame(ui: &Ui, r: Rect, head_w: f32) {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let stroke = Stroke::new(1.0, if theme.glow { c.line_dark } else { c.line_mid });
    let top = r.top() + 8.0;
    let p = ui.painter();
    p.line_segment([pos2(r.left(), top), pos2(r.left() + 6.0, top)], stroke);
    p.line_segment([pos2(r.left() + 10.0 + head_w + 4.0, top), pos2(r.right(), top)], stroke);
    p.line_segment([pos2(r.right(), top), pos2(r.right(), r.bottom())], stroke);
    p.line_segment([pos2(r.right(), r.bottom()), pos2(r.left(), r.bottom())], stroke);
    p.line_segment([pos2(r.left(), r.bottom()), pos2(r.left(), top)], stroke);
}

/// Zeichnet die Registerkarte „Datei“ in `page` (`inner` = Seite ohne Randabstand).
pub fn show(ui: &mut Ui, page: Rect, inner: Rect, d: &mut ImagePreview) {
    let theme = Theme::current(ui.ctx());
    let c = theme.colors.clone();
    let font = theme.ui_font();
    let doc = d.doc_size;
    let active = d.active;
    let (left, right) = (inner.left(), inner.right());
    let labels = tl(&LABEL_KEYS);
    let lw = labels[..5].iter().map(|s| text_w(ui.ctx(), s)).fold(22.0, f32::max).ceil();

    // Gruppe „Skalieren“
    let g1 = Rect::from_min_max(pos2(left, page.top() + 8.0), pos2(right, page.top() + 140.0));
    let head = t("image-preview-file-scale");
    frame(ui, g1, text_w(ui.ctx(), &head));
    ui.painter().text(pos2(g1.left() + 10.0, g1.top() + 8.0), Align2::LEFT_CENTER, &head, font.clone(), c.text);
    let s = &mut d.panes[active].settings;
    let lx = g1.left() + 14.0;
    let fx = lx + lw + 6.0;
    let row = |k: f32| g1.top() + 24.0 + 26.0 * k;
    let lab = |y: f32| Rect::from_min_size(pos2(lx, y), vec2(lw, 20.0));
    fields::label_right(ui, lab(row(0.0)), &labels[0], true);
    let pr = Rect::from_min_size(pos2(fx, row(0.0)), vec2(50.0, 20.0));
    if let Some(v) = fields::number(ui, "pv-file-percent", pr, Some(Rect::from_min_size(pos2(fx + 51.0, row(0.0)), vec2(14.0, 20.0))), Some(crop::percent_shown(doc, s)), crop::PERCENT_RANGE, 0, true) {
        crop::set_percent(s, doc, v);
    }
    tip(ui, "percent", pr, "image-preview-tip-scale-percent");
    let (sw, sh) = crop::scaled_size(doc, s);
    for (k, (key, horizontal, val, tipk)) in [("pv-file-w", true, sw, "image-preview-tip-scale-width"), ("pv-file-h", false, sh, "image-preview-tip-scale-height")].into_iter().enumerate() {
        let y = row(1.0 + k as f32);
        fields::label_right(ui, lab(y), &labels[1 + k], true);
        let r = Rect::from_min_size(pos2(fx, y), vec2(50.0, 20.0));
        if let Some(v) = fields::number(ui, key, r, None, Some(val as f64), (1.0, crop::MAX_SIZE), 0, true) {
            crop::set_size(s, doc, horizontal, v);
        }
        tip(ui, key, r, tipk);
        ui.painter().text(pos2(r.right() + 5.0, y + 10.0), Align2::LEFT_CENTER, &labels[5], font.clone(), c.text);
    }
    // Kettensymbol mit Klammer rechts von B/H (zeigt die Kopplung)
    let constrain = crop::constrain(s);
    let px_w = text_w(ui.ctx(), &labels[5]);
    let bx = (fx + 50.0 + 5.0 + px_w + 8.0).min(right - 22.0);
    let (y1, y2) = (row(1.0) + 10.0, row(2.0) + 10.0);
    let lc = if constrain { c.text } else { c.text_disabled };
    ui.painter().line(vec![pos2(bx, y1), pos2(bx + 5.0, y1), pos2(bx + 5.0, y2), pos2(bx, y2)], Stroke::new(1.0, lc));
    let ir = Rect::from_center_size(pos2(bx + 12.0, (y1 + y2) / 2.0), vec2(14.0, 16.0));
    ui.painter().rect_filled(ir, 0.0, c.frame_bg);
    icons::paint(ui.painter(), ir, Icon::LinkProportions, &IconStyle::new(&theme, None, constrain));
    let cr = Rect::from_min_size(pos2(lx, row(3.0) + 2.0), vec2(g1.right() - lx - 4.0, 16.0));
    if let Some(b) = fields::checkbox(ui, "pv-file-constrain", cr, &t("image-preview-file-constrain"), constrain, true) {
        crop::set_constrain(s, doc, b);
    }
    tip(ui, "constrain", cr, "image-preview-tip-constrain");

    // Gruppe „Exportbereich“ mit Kästchen als Kopf
    let g2 = Rect::from_min_max(pos2(left, g1.bottom() + 14.0), pos2(right, g1.bottom() + 100.0));
    let area_l = t("image-preview-file-export-area");
    let on = s.crop.is_some();
    frame(ui, g2, 18.0 + text_w(ui.ctx(), &area_l));
    let hr = Rect::from_min_size(pos2(g2.left() + 10.0, g2.top()), vec2(18.0 + text_w(ui.ctx(), &area_l) + 2.0, 16.0));
    ui.painter().rect_filled(hr, 0.0, c.frame_bg);
    if let Some(b) = fields::checkbox(ui, "pv-file-area", hr, &area_l, on, true) {
        crop::set_area_enabled(s, doc, b);
    }
    tip(ui, "area", hr, "image-preview-tip-export-area");
    let vals = crop::area_shown(doc, s);
    let half = (g2.width() - 14.0) / 2.0;
    let cells = [(0usize, 0.0, 0.0, 3usize, "image-preview-tip-area-x"), (1, 0.0, 1.0, 4, "image-preview-tip-area-y"), (2, 1.0, 0.0, 1, "image-preview-tip-area-width"), (3, 1.0, 1.0, 2, "image-preview-tip-area-height")];
    for (k, col, rowk, label, tipk) in cells {
        let x = g2.left() + 14.0 + col * half;
        let y = g2.top() + 26.0 + 26.0 * rowk;
        fields::label_right(ui, Rect::from_min_size(pos2(x, y), vec2(lw, 20.0)), &labels[label], on);
        let r = Rect::from_min_size(pos2(x + lw + 6.0, y), vec2(46.0f32.min(half - lw - 10.0), 20.0));
        if let Some(v) = fields::number(ui, ("pv-file-area-v", k), r, None, Some(vals[k] as f64), (if k < 2 { 0.0 } else { 1.0 }, crop::MAX_SIZE), 0, on) {
            crop::set_area_value(s, doc, k, v);
        }
        tip(ui, &format!("area-{k}"), r, tipk);
    }
}
