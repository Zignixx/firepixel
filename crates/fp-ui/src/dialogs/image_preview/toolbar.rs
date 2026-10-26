//! Werkzeugleiste unter der Vorschaufläche (re\17-bildvorschau.md §6): Zeiger, Exportbereich,
//! Zoom, Zoomliste, 1/2/4 Felder und die Animationsknöpfe mit Statusnummer (Paket P3).

use super::{ImagePreview, Tool, ViewMode};
use crate::icons::{self, Icon, IconStyle};
use crate::inspector::fields;
use crate::theme::Theme;
use crate::tips::HoverTip;
use crate::widgets::{ButtonLook, button_frame};
use egui::{Id, Rect, Sense, Ui, pos2, vec2};
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
pub const WIDTH: f32 = 3.0 * 24.0
    + 6.0
    + ZOOM_W
    + 6.0
    + 3.0 * 24.0
    + 10.0
    + 3.0 * 21.0
    + 6.0
    + 44.0
    + 6.0
    + 2.0 * 21.0;
const ZOOM_W: f32 = 66.0;

/// Symbolknopf der Werkzeugleiste (Schalter-Optik, Tooltip). Liefert „geklickt“.
pub(super) fn tool_button(
    ui: &mut Ui,
    id: impl std::hash::Hash + std::fmt::Debug,
    r: Rect,
    icon: Icon,
    selected: bool,
    enabled: bool,
    tip: &str,
) -> bool {
    let theme = Theme::current(ui.ctx());
    let resp = ui.interact(
        r,
        Id::new(("fp-ipv-tool", id)),
        if enabled {
            Sense::CLICK
        } else {
            Sense::hover()
        },
    );
    resp.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, enabled, selected, tip)
    });
    let look = ButtonLook {
        selected,
        enabled,
        group: None,
    };
    button_frame(ui.painter(), r, &resp, look, &theme);
    if !selected && enabled && !resp.hovered() {
        // ungedrückte Knöpfe mit dünnem Rahmen wie die Knöpfe des Originals
        ui.painter().rect_stroke(
            r,
            2.0,
            egui::Stroke::new(
                1.0,
                crate::theme::with_alpha(theme.colors.button_border, 110),
            ),
            egui::StrokeKind::Inside,
        );
    }
    let st = IconStyle::new(&theme, None, enabled);
    icons::paint(
        ui.painter(),
        Rect::from_center_size(r.center(), vec2(16.0, 16.0)),
        icon,
        &st,
    );
    let clicked = enabled && resp.clicked();
    if !tip.is_empty() {
        resp.hover_tip(tip);
    }
    clicked
}

/// Zeichnet die Werkzeugleiste ab `origin` (linke obere Ecke, Höhe 24).
pub(super) fn show(ui: &mut Ui, d: &mut ImagePreview, origin: egui::Pos2) {
    let mut x = origin.x;
    let y = origin.y;
    let b = |x: f32| Rect::from_min_size(pos2(x, y), vec2(24.0, 24.0));
    if tool_button(
        ui,
        "pointer",
        b(x),
        Icon::Pointer,
        d.tool == Tool::Pointer,
        true,
        &t("image-preview-tool-pointer"),
    ) {
        d.tool = Tool::Pointer;
    }
    x += 24.0;
    // Exportbereich (§6): Rahmen mit Griffen im aktiven Feld
    if tool_button(
        ui,
        "export-area",
        b(x),
        Icon::ExportArea,
        d.tool == Tool::ExportArea,
        true,
        &t("image-preview-tool-export-area"),
    ) {
        d.tool = Tool::ExportArea;
    }
    x += 24.0;
    if tool_button(
        ui,
        "zoom",
        b(x),
        Icon::Zoom,
        d.tool == Tool::Zoom,
        true,
        &t("image-preview-tool-zoom"),
    ) {
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
    ui.interact(zr, Id::new("fp-ipv-zoom-tip"), Sense::hover())
        .hover_tip(zoom_tip);
    x += ZOOM_W + 6.0;
    for (mode, icon, key) in [
        (ViewMode::One, Icon::ViewPreview, "image-preview-view-1"),
        (ViewMode::Two, Icon::View2Up, "image-preview-view-2"),
        (ViewMode::Four, Icon::View4Up, "image-preview-view-4"),
    ] {
        if tool_button(ui, key, b(x), icon, d.mode == mode, true, &t(key)) {
            d.set_mode(mode);
        }
        x += 24.0;
    }
    // Animation (§6): erster, abspielen/stoppen, letzter, Statusnummer, vorheriger, nächster.
    // Grau: alles bei nur einem Status; auf Status 1 „erster“/„vorheriger“, auf dem letzten
    // „letzter“/„nächster“. Der Status gilt für alle Felder.
    x += 10.0;
    let n = d.frames.max(1);
    let multi = n > 1;
    let cur = d.frame.min(n - 1);
    let a = |x: f32| Rect::from_min_size(pos2(x, y + 2.0), vec2(21.0, 20.0));
    if tool_button(
        ui,
        "image-preview-first-frame",
        a(x),
        Icon::FirstFrame,
        false,
        multi && cur > 0,
        &t("image-preview-first-frame"),
    ) {
        super::animation_tab::go_to(d, 0);
    }
    x += 21.0;
    if play_button(ui, a(x), d.playing, multi) {
        d.playing = !d.playing;
        // RE spec 72 §3.2: Play shows the next included state at once (due at time 0)
        d.next_tick = d.playing.then_some(0.0);
    }
    x += 21.0;
    if tool_button(
        ui,
        "image-preview-last-frame",
        a(x),
        Icon::LastFrame,
        false,
        multi && cur + 1 < n,
        &t("image-preview-last-frame"),
    ) {
        super::animation_tab::go_to(d, n - 1);
    }
    x += 21.0 + 6.0;
    let fr = Rect::from_min_size(pos2(x, y + 2.0), vec2(44.0, 20.0));
    let items: Vec<String> = (1..=n).map(|i| i.to_string()).collect();
    if let Some(v) = fields::combo_edit(
        ui,
        "fp-ipv-frame",
        fr,
        &(cur + 1).to_string(),
        &items,
        multi,
    ) && let Some(k) = fields::parse_num(&v)
    {
        super::animation_tab::go_to(d, (k.round().max(1.0) as usize).min(n) - 1);
    }
    let tip = t("image-preview-current-frame");
    let tr = ui.interact(fr, Id::new("fp-ipv-frame-tip"), Sense::hover());
    tr.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, multi, tip.as_str()));
    tr.hover_tip(tip);
    x += 44.0 + 6.0;
    if tool_button(
        ui,
        "image-preview-prev-frame",
        a(x),
        Icon::PrevFrame,
        false,
        multi && cur > 0,
        &t("image-preview-prev-frame"),
    ) {
        super::animation_tab::go_to(d, cur - 1);
    }
    x += 21.0;
    if tool_button(
        ui,
        "image-preview-next-frame",
        a(x),
        Icon::NextFrame,
        false,
        multi && cur + 1 < n,
        &t("image-preview-next-frame"),
    ) {
        super::animation_tab::go_to(d, cur + 1);
    }
}

/// Knopf „Abspielen / Stoppen“: ▶ bzw. ■ während des Abspielens.
fn play_button(ui: &mut Ui, r: Rect, playing: bool, enabled: bool) -> bool {
    let tip = t("image-preview-play");
    if !playing {
        return tool_button(
            ui,
            "image-preview-play",
            r,
            Icon::Play,
            false,
            enabled,
            &tip,
        );
    }
    let theme = Theme::current(ui.ctx());
    let resp = ui.interact(
        r,
        Id::new(("fp-ipv-tool", "image-preview-play")),
        if enabled {
            Sense::CLICK
        } else {
            Sense::hover()
        },
    );
    resp.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, enabled, true, tip.as_str())
    });
    button_frame(
        ui.painter(),
        r,
        &resp,
        ButtonLook {
            selected: true,
            enabled,
            group: None,
        },
        &theme,
    );
    ui.painter().rect_filled(
        Rect::from_center_size(r.center(), vec2(8.0, 8.0)),
        0.0,
        theme.colors.icon_fg,
    );
    let clicked = enabled && resp.clicked();
    resp.hover_tip(tip);
    clicked
}
