//! Linke Spalte der Bildvorschau: Registerkarten Optionen / Datei / Animation. Sie zeigt und ändert
//! immer die Einstellung des **aktiven** Feldes (re\17-bildvorschau.md §5.1). Stand P1: die
//! bisherigen Felder (Format, Palette, Farben, Verlust, Qualität, Matte, Zeilensprung; Skalieren;
//! Statusanzahl). Paket P2 (Optionen vollständig) und P3 (Datei, Animation) bauen hier weiter.

use super::super::{combo_fit_w, extra_w, fit_label, refs, text_w, tl};
use crate::app_state::AppState;
use crate::inspector::fields;
use crate::panels::optimize::{self, FORMATS, PALETTES};
use crate::theme::Theme;
use egui::{pos2, vec2, Align2, Id, Rect, Sense, Stroke, StrokeKind, Ui};
use fp_export::options::{ColorMode, ExportFormat, OptimizeSettings};
use fp_i18n::t;

const TAB_KEYS: [&str; 3] = ["dlg-preview-tab-options", "dlg-preview-tab-file", "dlg-preview-tab-animation"];
const LABEL_KEYS: [&str; 7] = ["dlg-preview-format", "dlg-preview-palette", "dlg-preview-colors", "dlg-preview-loss", "dlg-lbl-quality", "dlg-preview-matte", "dlg-preview-scale"];

/// Mindestbreite der linken Spalte (Original 279 px, §1.1).
pub const MIN_WIDTH: f32 = 279.0;
/// Innenabstand der Registerseite.
const PAD: f32 = 6.0;

/// Maße der linken Spalte (wachsen mit längeren Texten anderer Sprachen).
#[derive(Clone, Copy, Debug)]
pub struct Measure {
    pub width: f32,
    tab_w: f32,
    dx: f32,
    cw: f32,
}

pub fn measure(ctx: &egui::Context) -> Measure {
    let tabs = tl(&TAB_KEYS);
    let labs = tl(&LABEL_KEYS);
    let fnames: Vec<String> = (0..FORMATS.len()).map(optimize::format_name).collect();
    let pnames: Vec<String> = (0..PALETTES.len()).map(optimize::palette_name).collect();
    let tab_w = super::super::buttons_w(ctx, &refs(&tabs), 80.0);
    let dx = extra_w(ctx, &refs(&labs), 70.0 - 2.0);
    let cw = combo_fit_w(ctx, &refs(&fnames), 160.0).max(combo_fit_w(ctx, &refs(&pnames), 160.0));
    let width = MIN_WIDTH.max(3.0 * tab_w + 6.0).max(2.0 * PAD + 76.0 + dx + cw + 4.0).max(2.0 * PAD + 18.0 + text_w(ctx, &t("dlg-preview-interlaced"))).ceil();
    Measure { width, tab_w, dx, cw }
}

/// Zeichnet die linke Spalte in `left` für die Einstellung `s` des aktiven Feldes.
pub fn show(ui: &mut Ui, st: &mut AppState, left: Rect, m: Measure, tab: &mut usize, s: &mut OptimizeSettings) {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let tabs = tl(&TAB_KEYS);
    let labs = tl(&LABEL_KEYS);
    let fnames: Vec<String> = (0..FORMATS.len()).map(optimize::format_name).collect();
    let pnames: Vec<String> = (0..PALETTES.len()).map(optimize::palette_name).collect();
    let interl_l = t("dlg-preview-interlaced");
    let (dx, cw) = (m.dx, m.cw);
    let lw = 70.0 + dx;
    // Registerkarten Optionen / Datei / Animation
    for (i, label) in tabs.iter().enumerate() {
        let tr = Rect::from_min_size(left.min + vec2(i as f32 * (m.tab_w + 2.0), 0.0), vec2(m.tab_w, 20.0));
        let resp = ui.interact(tr, Id::new(("pv-tab", i)), Sense::click());
        resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, *tab == i, label));
        ui.painter().rect_filled(tr, 0.0, if *tab == i { c.tab_active_bg } else { c.tab_inactive_bg });
        ui.painter().rect_stroke(tr, 0.0, Stroke::new(1.0, c.line_mid), StrokeKind::Inside);
        ui.painter().text(tr.center(), Align2::CENTER_CENTER, label.as_str(), theme.ui_font(), c.text);
        if resp.clicked() {
            *tab = i;
        }
    }
    let page = Rect::from_min_max(pos2(left.left(), left.top() + 20.0), left.max);
    ui.painter().rect_stroke(page, 0.0, Stroke::new(1.0, c.line_mid), StrokeKind::Inside);
    let left = page.shrink2(vec2(PAD, 0.0));
    let cx = left.left() + 76.0 + dx;
    let y0 = page.top() + 10.0;
    match *tab {
        0 => {
            fit_label(ui, Rect::from_min_size(pos2(left.left(), y0), vec2(lw, 20.0)), &labs[0], true);
            if let Some(i) = fields::combo(ui, "pv-format", Rect::from_min_size(pos2(cx, y0), vec2(cw, 20.0)), &refs(&fnames), optimize::format_index(s), true)
                && let Some((f, mode)) = FORMATS[i].1
            {
                s.format = f;
                s.color_mode = mode;
            }
            let mut y = y0 + 28.0;
            if s.color_mode == ColorMode::Indexed {
                fit_label(ui, Rect::from_min_size(pos2(left.left(), y), vec2(lw, 20.0)), &labs[1], true);
                if let Some(i) = fields::combo(ui, "pv-pal", Rect::from_min_size(pos2(cx, y), vec2(cw, 20.0)), &refs(&pnames), Some(optimize::palette_index(s)), true) {
                    let (_, k, web, cnt) = PALETTES[i];
                    s.palette_kind = k;
                    s.web_adaptive = web;
                    if let Some(n) = cnt {
                        s.max_colors = n;
                    }
                }
                y += 28.0;
                fit_label(ui, Rect::from_min_size(pos2(left.left(), y), vec2(lw, 20.0)), &labs[2], true);
                if let Some(v) = fields::number(ui, "pv-colors", Rect::from_min_size(pos2(cx, y), vec2(50.0, 20.0)), Some(Rect::from_min_size(pos2(cx + 52.0, y), vec2(14.0, 20.0))), Some(s.max_colors as f64), (2.0, 256.0), 0, true) {
                    s.max_colors = v as i32;
                }
                y += 28.0;
                fit_label(ui, Rect::from_min_size(pos2(left.left(), y), vec2(lw, 20.0)), &labs[3], true);
                if let Some(v) = fields::number(ui, "pv-lossy", Rect::from_min_size(pos2(cx, y), vec2(50.0, 20.0)), None, Some(s.lossy as f64), (0.0, 100.0), 0, matches!(s.format, ExportFormat::Gif | ExportFormat::AnimatedGif)) {
                    s.lossy = v as i32;
                }
                y += 28.0;
            } else if matches!(s.format, ExportFormat::Jpeg | ExportFormat::WebP | ExportFormat::Avif) {
                fit_label(ui, Rect::from_min_size(pos2(left.left(), y), vec2(lw, 20.0)), &labs[4], true);
                let q = match s.format {
                    ExportFormat::Jpeg => s.jpeg.quality,
                    ExportFormat::Avif => s.avif.quality,
                    _ => s.webp.quality,
                };
                if let Some(v) = fields::number(ui, "pv-q", Rect::from_min_size(pos2(cx, y), vec2(50.0, 20.0)), Some(Rect::from_min_size(pos2(cx + 52.0, y), vec2(14.0, 20.0))), Some(q as f64), (1.0, 100.0), 0, true) {
                    match s.format {
                        ExportFormat::Jpeg => s.jpeg.quality = v as i32,
                        ExportFormat::Avif => s.avif.quality = v as i32,
                        _ => s.webp.quality = v as i32,
                    }
                }
                y += 28.0;
            }
            fit_label(ui, Rect::from_min_size(pos2(left.left(), y), vec2(lw, 20.0)), &labs[5], true);
            match crate::inspector::popups::color_button(ui, "pv-matte", Rect::from_min_size(pos2(cx, y), vec2(26.0, 20.0)), st, s.matte.map(fp_color::Argb), true, true, false) {
                Some(crate::inspector::popups::ColorChoice::Color(col)) => s.matte = Some(col.0 | 0xff00_0000),
                Some(crate::inspector::popups::ColorChoice::None) => s.matte = None,
                _ => {}
            }
            y += 28.0;
            if let Some(b) = fields::checkbox(ui, "pv-interl", Rect::from_min_size(pos2(left.left(), y), vec2(left.width(), 16.0)), &interl_l, s.interlaced || s.jpeg.progressive, true) {
                s.interlaced = b;
                s.jpeg.progressive = b;
            }
        }
        1 => {
            fit_label(ui, Rect::from_min_size(pos2(left.left(), y0), vec2(lw, 20.0)), &labs[6], true);
            let percent = s.scale.filter(|sc| sc.use_scale).map(|sc| sc.percent as f64).unwrap_or(100.0);
            if let Some(v) = fields::number(ui, "pv-scale", Rect::from_min_size(pos2(cx, y0), vec2(50.0, 20.0)), Some(Rect::from_min_size(pos2(cx + 52.0, y0), vec2(14.0, 20.0))), Some(percent), (1.0, 1000.0), 0, true) {
                s.scale = Some(fp_model::ExportScale { use_scale: (v - 100.0).abs() > 0.01, constrain: true, percent: v.round() as i32, width: 0, height: 0 });
            }
            ui.painter().text(pos2(cx + 70.0, y0 + 10.0), Align2::LEFT_CENTER, "%", theme.ui_font(), c.text);
        }
        _ => {
            let frames = st.session.active().map(|x| x.frame_count()).unwrap_or(1);
            ui.painter().text(pos2(left.left(), y0), Align2::LEFT_TOP, fp_i18n::tr!("dlg-preview-states", n = frames), theme.ui_font(), c.text);
            let looping = crate::panels::states::loop_text(st.session.active().and_then(|x| x.doc.pages.get(x.page)).map(|p| p.loop_count()).unwrap_or(0));
            ui.painter().text(pos2(left.left(), y0 + 20.0), Align2::LEFT_TOP, fp_i18n::tr!("dlg-preview-loop", looping = looping), theme.ui_font(), c.text);
        }
    }
}
