//! Werkzeugleiste unter der Vorschaufläche (re\17-bildvorschau.md §6): Zeiger, Exportbereich
//! (Paket P3, grau), Zoom, Zoomliste, 1/2/4 Felder und die Animationsknöpfe (P3, grau).

use super::{ImagePreview, Tool, ViewMode};
use crate::icons::{self, Icon, IconStyle};
use crate::inspector::fields;
use crate::theme::Theme;
use crate::widgets::{button_frame, ButtonLook};
use egui::{pos2, vec2, Id, Rect, Sense, Ui};
use fp_i18n::t;

/// Zoomstufen der Zoomliste (11 Stufen 6 % … 6400 %, ohne „Einpassen“, §6).
pub const ZOOMS: [(&str, f32); 11] = [
    ("6%", 0.0625),
    ("12%", 0.125),
    ("25%", 0.25),
    ("50%", 0.5),
    ("100%", 1.0),
    ("200%", 2.0),
    ("400%", 4.0),
    ("800%", 8.0),
    ("1600%", 16.0),
    ("3200%", 32.0),
    ("6400%", 64.0),
];

/// Startstufe 100 %.
pub const ZOOM_100: usize = 4;

/// Breite der Werkzeugleiste (für die Mindestbreite des Dialogs).
pub const WIDTH: f32 = 3.0 * 24.0 + 6.0 + ZOOM_W + 6.0 + 3.0 * 24.0 + 10.0 + 3.0 * 21.0 + 6.0 + 44.0 + 6.0 + 2.0 * 21.0;
const ZOOM_W: f32 = 66.0;

/// Symbolknopf der Werkzeugleiste (Schalter-Optik, Tooltip). Liefert „geklickt“.
pub(super) fn tool_button(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug, r: Rect, icon: Icon, selected: bool, enabled: bool, tip: &str) -> bool {
    let theme = Theme::current(ui.ctx());
    let resp = ui.interact(r, Id::new(("fp-ipv-tool", id)), if enabled { Sense::click() } else { Sense::hover() });
    resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, enabled, selected, tip));
    let look = ButtonLook { selected, enabled, group: None };
    button_frame(ui.painter(), r, &resp, look, &theme);
    if !selected && enabled && !resp.hovered() {
        // ungedrückte Knöpfe mit dünnem Rahmen wie die Knöpfe des Originals
        ui.painter().rect_stroke(r, 2.0, egui::Stroke::new(1.0, crate::theme::with_alpha(theme.colors.button_border, 110)), egui::StrokeKind::Inside);
    }
    let st = IconStyle::new(&theme, None, enabled);
    icons::paint(ui.painter(), Rect::from_center_size(r.center(), vec2(16.0, 16.0)), icon, &st);
    let clicked = enabled && resp.clicked();
    if !tip.is_empty() {
        resp.on_hover_text(tip);
    }
    clicked
}

/// Zeichnet die Werkzeugleiste ab `origin` (linke obere Ecke, Höhe 24).
pub(super) fn show(ui: &mut Ui, d: &mut ImagePreview, origin: egui::Pos2) {
    let mut x = origin.x;
    let y = origin.y;
    let b = |x: f32| Rect::from_min_size(pos2(x, y), vec2(24.0, 24.0));
    if tool_button(ui, "pointer", b(x), Icon::Pointer, d.tool == Tool::Pointer, true, &t("image-preview-tool-pointer")) {
        d.tool = Tool::Pointer;
    }
    x += 24.0;
    // Exportbereich: Paket P3
    tool_button(ui, "export-area", b(x), Icon::ExportArea, false, false, &t("image-preview-tool-export-area"));
    x += 24.0;
    if tool_button(ui, "zoom", b(x), Icon::Zoom, d.tool == Tool::Zoom, true, &t("image-preview-tool-zoom")) {
        d.tool = Tool::Zoom;
    }
    x += 24.0 + 6.0;
    let names: Vec<&str> = ZOOMS.iter().map(|z| z.0).collect();
    let zr = Rect::from_min_size(pos2(x, y + 2.0), vec2(ZOOM_W, 20.0));
    let resp_id = ("fp-ipv-zoom", 0);
    if let Some(i) = fields::combo(ui, resp_id, zr, &names, Some(d.zoom_idx), true) {
        d.set_zoom(i, None);
    }
    let zoom_tip = t("image-preview-zoom-tip");
    ui.interact(zr, Id::new("fp-ipv-zoom-tip"), Sense::hover()).on_hover_text(zoom_tip);
    x += ZOOM_W + 6.0;
    for (mode, icon, key) in [(ViewMode::One, Icon::ViewPreview, "image-preview-view-1"), (ViewMode::Two, Icon::View2Up, "image-preview-view-2"), (ViewMode::Four, Icon::View4Up, "image-preview-view-4")] {
        if tool_button(ui, key, b(x), icon, d.mode == mode, true, &t(key)) {
            d.set_mode(mode);
        }
        x += 24.0;
    }
    // Animation (Paket P3): Plätze mit grauen Knöpfen
    x += 10.0;
    let a = |x: f32| Rect::from_min_size(pos2(x, y + 2.0), vec2(21.0, 20.0));
    for (icon, key) in [(Icon::FirstFrame, "image-preview-first-frame"), (Icon::Play, "image-preview-play"), (Icon::LastFrame, "image-preview-last-frame")] {
        tool_button(ui, key, a(x), icon, false, false, &t(key));
        x += 21.0;
    }
    x += 6.0;
    let fr = Rect::from_min_size(pos2(x, y + 2.0), vec2(44.0, 20.0));
    fields::field_frame(ui, fr, false, false);
    let theme = Theme::current(ui.ctx());
    ui.painter().text(pos2(fr.left() + 4.0, fr.center().y), egui::Align2::LEFT_CENTER, "1", theme.ui_font(), theme.colors.text_disabled);
    ui.interact(fr, Id::new("fp-ipv-frame-tip"), Sense::hover()).on_hover_text(t("image-preview-current-frame"));
    x += 44.0 + 6.0;
    for (icon, key) in [(Icon::PrevFrame, "image-preview-prev-frame"), (Icon::NextFrame, "image-preview-next-frame")] {
        tool_button(ui, key, a(x), icon, false, false, &t(key));
        x += 21.0;
    }
}
