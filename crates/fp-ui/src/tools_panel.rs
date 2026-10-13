//! Werkzeugbedienfeld links (Vorlage 6594, 11 §4.3, Laufzeit 10 §4a T3): Abschnitte Auswählen,
//! Bitmap, Vektor, Web, Farben, Ansicht mit je zwei Plätzen pro Zeile. Plätze mit mehreren
//! Werkzeugen zeigen ein kleines Dreieck; Drücken-und-Halten oder Klick auf das Dreieck öffnet den
//! Flyout mit Namen und Kürzeln. Der Platz zeigt das zuletzt gewählte Werkzeug der Gruppe.

use crate::app_state::{Action, AppState};
use crate::icons::{self, tool_icon, Icon, IconStyle};
use crate::inspector::color_picker::Footer;
use crate::inspector::popups::ColorChoice;
use crate::theme::{with_alpha, Theme, ToolGroup};
use crate::widgets::{self, ButtonLook};
use egui::{pos2, vec2, Align2, Color32, Id, Rect, Sense, Stroke, StrokeKind, Ui};
use fp_engine::tools::{self, ToolId};
use serde_json::json;

/// Offener Flyout.
#[derive(Clone, Debug, PartialEq)]
pub struct FlyoutState {
    /// Flyout-Gruppe (Spalte `flyout` der Werkzeugtabelle).
    pub group: String,
    /// Platz im Bedienfeld (Bildschirmkoordinaten).
    pub anchor: Rect,
    /// Zeitpunkt des Öffnens.
    pub opened: f64,
}

/// Ein Platz im Werkzeugbedienfeld.
#[derive(Clone, Copy, Debug)]
enum Slot {
    /// Standardwerkzeug des Platzes; die Flyout-Gruppe ist die des Werkzeugs (Werkzeugtabelle).
    Tool(ToolId),
    /// Pseudo-Werkzeug „Segmente ausblenden/einblenden“.
    Slices(bool),
}

const SECTIONS: &[(ToolGroup, &[Slot])] = &[
    (ToolGroup::Select, &[Slot::Tool(ToolId::Pointer), Slot::Tool(ToolId::Subselection), Slot::Tool(ToolId::Scale), Slot::Tool(ToolId::Crop)]),
    (
        ToolGroup::Bitmap,
        &[
            Slot::Tool(ToolId::Marquee),
            Slot::Tool(ToolId::Lasso),
            Slot::Tool(ToolId::MagicWand),
            Slot::Tool(ToolId::Brush),
            Slot::Tool(ToolId::Pencil),
            Slot::Tool(ToolId::Eraser),
            Slot::Tool(ToolId::Blur),
            Slot::Tool(ToolId::RubberStamp),
        ],
    ),
    (ToolGroup::Vector, &[Slot::Tool(ToolId::Line), Slot::Tool(ToolId::Pen), Slot::Tool(ToolId::Rectangle), Slot::Tool(ToolId::Text), Slot::Tool(ToolId::Freeform), Slot::Tool(ToolId::Knife)]),
    (ToolGroup::Web, &[Slot::Tool(ToolId::HotspotRect), Slot::Tool(ToolId::Slice), Slot::Slices(false), Slot::Slices(true)]),
    (ToolGroup::Colors, &[Slot::Tool(ToolId::Eyedropper), Slot::Tool(ToolId::PaintBucket)]),
];

/// Flyout-Gruppe (interne Kennung aus der Werkzeugtabelle von fp-engine) eines Werkzeugs.
fn flyout_of(id: ToolId) -> &'static str {
    tools::info(id).map(|i| i.flyout).unwrap_or("")
}

/// Automatische Formen im Rechteck-Flyout (Laufzeit T3; Logik in fp-script, `autoShape.insert`):
/// deutscher Name (Kennung für `autoShape.insert`, Reihenfolge wie im Original) und englischer
/// Name (daraus der Schlüssel `shape-…`, siehe [`shape_key`]).
pub const AUTO_SHAPES: &[(&str, &str)] = &[
    ("Abgefastes Rechteck", "Chamfer Rectangle"),
    ("Abgeflachtes Rechteck", "Beveled Rectangle"),
    ("Abgerundetes Rechteck", "Rounded Rectangle"),
    ("Donut", "Doughnut"),
    ("Intelligentes Polygon", "Smart Polygon"),
    ("L-Form", "L-Shape"),
    ("Messwerkzeug", "Measure Tool"),
    ("Pfeil", "Arrow"),
    ("Pfeillinie", "Arrow Line"),
    ("Spirale", "Spiral"),
    ("Stern", "Star"),
    ("Torte", "Pie"),
    ("Verbindungslinie", "Connector Line"),
];

/// Schlüssel einer automatischen Form aus ihrem englischen Namen (`autoShape.list` → `en`):
/// „Rounded Rectangle“ → `shape-rounded-rectangle`, „3D Box“ → `shape-3d-box`.
pub fn shape_key(en_name: &str) -> String {
    let mut s = String::from("shape-");
    let mut dash = false;
    for c in en_name.chars() {
        if c.is_ascii_alphanumeric() {
            if dash && !s.ends_with('-') {
                s.push('-');
            }
            dash = false;
            s.push(c.to_ascii_lowercase());
        } else {
            dash = true;
        }
    }
    s
}

/// Angezeigter Name einer automatischen Form in der aktuellen Sprache (englischer Name als
/// Schlüssel, deutscher als Rückfall).
pub fn shape_label(en_name: &str, de_name: &str) -> String {
    let k = shape_key(en_name);
    let s = fp_i18n::t(&k);
    if s == k { de_name.to_string() } else { s }
}

/// Schlüssel eines Werkzeugs: `tool.polygonLasso` → `tool-polygonLasso`.
pub fn tool_key(id: ToolId) -> String {
    let name = tools::info(id).map(|i| i.name).unwrap_or("tool.unknown");
    name.replace(['.', '_'], "-")
}

/// Name eines Werkzeugs in der aktuellen Sprache (`tools.ftl`, Rückfall: deutscher Name).
pub fn tool_label(id: ToolId) -> String {
    let k = tool_key(id);
    let s = fp_i18n::t(&k);
    if s == k { tools::info(id).map(|i| i.label.to_string()).unwrap_or_default() } else { s }
}

/// Welche Farbe die Knöpfe „Keine“/Pipette betreffen (zuletzt angeklicktes Farbfeld).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ActiveWell {
    Stroke,
    #[default]
    Fill,
}

fn slot_tool(st: &AppState, group: &str, default: ToolId) -> ToolId {
    st.session.tools.slot_memory.get(group).copied().unwrap_or(default)
}

fn shortcut_text(st: &AppState, id: ToolId) -> String {
    let set = st.session.shortcut_set();
    tools::tool_shortcuts(set, id).iter().map(crate::menu::shortcut_text).collect::<Vec<_>>().join(", ")
}

/// Tooltip eines Werkzeugs wie in CS6: „Name (Kürzel)“.
pub fn tool_tooltip(st: &AppState, id: ToolId) -> String {
    let label = tool_label(id);
    let sc = shortcut_text(st, id);
    if sc.is_empty() { label } else { format!("{label} ({sc})") }
}

fn argb_to_color(a: fp_color::Argb) -> Color32 {
    Color32::from_rgba_unmultiplied((a.0 >> 16 & 0xff) as u8, (a.0 >> 8 & 0xff) as u8, (a.0 & 0xff) as u8, (a.0 >> 24) as u8)
}

/// Zeichnet das Werkzeugbedienfeld.
pub fn show(ui: &mut Ui, st: &mut AppState) {
    let theme = Theme::current(ui.ctx());
    let c = theme.colors.clone();
    let full = ui.max_rect();
    ui.painter().rect_filled(full, 0.0, c.frame_bg);
    let strip = Rect::from_min_size(full.min, vec2(full.width(), theme.metrics.dock_strip_height));
    widgets::dock_strip(ui.painter(), strip, &theme);
    icons::paint(ui.painter(), Rect::from_center_size(pos2(strip.left() + 9.0, strip.center().y), vec2(12.0, 10.0)), Icon::CollapseRight, &IconStyle::mono(c.dock_strip_arrow, c.frame_bg));
    // Griffleiste
    widgets::grip(ui.painter(), Rect::from_min_max(pos2(full.left() + 18.0, strip.bottom() + 3.0), pos2(full.right() - 18.0, strip.bottom() + 9.0)), false, &theme);
    // Bei zu niedrigem Fenster lässt sich das Werkzeugbedienfeld mit dem Mausrad verschieben
    // (nichts wird dauerhaft abgeschnitten); der Bereich unter der Griffleiste ist der Bildlaufbereich.
    let scroll_id = Id::new("tools-panel-scroll");
    let mut off: f32 = ui.ctx().data(|d| d.get_temp(scroll_id)).unwrap_or(0.0);
    let body = Rect::from_min_max(pos2(full.left(), strip.bottom() + 10.0), full.max);
    ui.set_clip_rect(body.intersect(ui.clip_rect()));
    let mut y = strip.bottom() + 11.0 - off;
    let cell = theme.metrics.tool_cell;
    let left = full.left() + (full.width() - 2.0 * cell.x) / 2.0;
    let now = ui.input(|i| i.time);
    let active = st.session.tools.active;
    for (si, (group, slots)) in SECTIONS.iter().enumerate() {
        if si > 0 {
            section_separator(ui, full, y, &theme);
            y += 5.0;
        }
        section_label(ui, full, y, *group, &theme);
        y += theme.metrics.tool_section_label_height;
        for (k, slot) in slots.iter().enumerate() {
            let col = k % 2;
            let r = Rect::from_min_size(pos2(left + col as f32 * cell.x, y), cell);
            match slot {
                Slot::Tool(default) => tool_slot(ui, st, r, flyout_of(*default), *default, *group, active, now),
                Slot::Slices(show) => {
                    let hidden = st.session.active().is_some_and(|d| d.view.hide_slices);
                    let selected = if *show { !hidden } else { hidden };
                    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r));
                    let icon = if *show { Icon::ShowSlices } else { Icon::HideSlices };
                    let tip = fp_i18n::t(if *show { "tool-tip-showSlices" } else { "tool-tip-hideSlices" });
                    if widgets::icon_button(&mut child, icon, cell - vec2(2.0, 2.0), theme.metrics.icon_size, ButtonLook { selected, enabled: st.session.active().is_some(), group: Some(*group) }, &tip).clicked() && !selected {
                        st.push(Action::Run("view.toggleSlices".into(), json!({})));
                    }
                }
            }
            if col == 1 || k + 1 == slots.len() {
                y += cell.y;
            }
        }
        if *group == ToolGroup::Colors {
            y = color_section(ui, st, full, y, &theme);
        }
    }
    // Ansicht
    section_separator(ui, full, y, &theme);
    y += 5.0;
    section_label(ui, full, y, ToolGroup::View, &theme);
    y += theme.metrics.tool_section_label_height;
    let modes = [(Icon::ScreenStandard, "tool-tip-screenStandard"), (Icon::ScreenFullMenus, "tool-tip-screenFullMenus"), (Icon::ScreenFull, "tool-tip-screenFull")];
    let small = vec2(18.0, 18.0);
    let total = small.x * 3.0 + 2.0 * 2.0;
    for (i, (icon, tip)) in modes.iter().enumerate() {
        let r = Rect::from_min_size(pos2(full.center().x - total / 2.0 + i as f32 * (small.x + 2.0), y), small);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r));
        let selected = st.session.ui.screen_mode as usize == i;
        if widgets::icon_button(&mut child, *icon, small, 13.0, ButtonLook { selected, enabled: true, group: Some(ToolGroup::View) }, &fp_i18n::t(tip)).clicked() {
            st.session.ui.screen_mode = i as u8;
            st.session.emit(fp_engine::SessionEvent::PanelsChanged);
        }
    }
    y += small.y + 4.0;
    for (k, default) in [ToolId::Hand, ToolId::Zoom].iter().enumerate() {
        let r = Rect::from_min_size(pos2(left + k as f32 * cell.x, y), cell);
        tool_slot(ui, st, r, flyout_of(*default), *default, ToolGroup::View, active, now);
    }
    y += cell.y + 4.0;
    section_separator(ui, full, y, &theme);
    // Bildlauf: Inhalt höher als der Bereich → Mausrad verschiebt, Pfeile zeigen weiteren Inhalt
    let content_h = y + 6.0 + off - body.top();
    let max_off = (content_h - body.height()).max(0.0);
    if max_off > 0.0 && ui.rect_contains_pointer(body) {
        off -= ui.input(|i| i.smooth_scroll_delta.y);
    }
    off = off.clamp(0.0, max_off);
    ui.ctx().data_mut(|d| d.insert_temp(scroll_id, off));
    if max_off > 0.0 {
        let col = theme.colors.dock_strip_arrow;
        if off < max_off {
            let p = pos2(full.center().x, full.bottom() - 4.0);
            ui.painter().add(egui::Shape::convex_polygon(vec![p + vec2(-4.0, -3.0), p + vec2(4.0, -3.0), p + vec2(0.0, 1.0)], col, Stroke::NONE));
        }
        if off > 0.0 {
            let p = pos2(full.center().x, body.top() + 3.0);
            ui.painter().add(egui::Shape::convex_polygon(vec![p + vec2(-4.0, 3.0), p + vec2(4.0, 3.0), p + vec2(0.0, -1.0)], col, Stroke::NONE));
        }
    }
    ui.allocate_rect(full, Sense::hover());
    flyout_popup(ui.ctx(), st);
}

fn section_label(ui: &Ui, full: Rect, y: f32, group: ToolGroup, theme: &Theme) {
    let color = if theme.glow { theme.accent(group) } else { theme.colors.section_label };
    // Überschrift in der aktuellen Sprache (`ui-toolgroup-…`, theme.rs); Breite des Bedienfelds
    // ist fest – zu lange Übersetzungen werden mit „…“ gekürzt statt überzulaufen
    let font = theme.font(theme.fonts.section);
    let name = group.name();
    let text = elide(ui, &name, &font, full.width() - 4.0);
    if text != name {
        // für Tests: Überschrift musste gekürzt werden
        ui.ctx().data_mut(|d| d.insert_temp(Id::new(SECTION_ELIDED_ID), true));
    }
    ui.painter().text(pos2(full.center().x, y + theme.metrics.tool_section_label_height / 2.0), Align2::CENTER_CENTER, text, font, color);
    if theme.glow {
        let w = 18.0;
        ui.painter().hline((full.center().x - w)..=(full.center().x + w), y + theme.metrics.tool_section_label_height - 2.5, Stroke::new(1.0, with_alpha(theme.accent(group), 120)));
    }
}

/// Kennung im egui-Speicher: `true`, sobald eine Abschnittsüberschrift gekürzt werden musste.
pub const SECTION_ELIDED_ID: &str = "fp-tools-section-elided";

/// Text auf `max_w` kürzen (mit „…“), damit nichts über den Rand läuft.
fn elide(ui: &Ui, text: &str, font: &egui::FontId, max_w: f32) -> String {
    if widgets::text_width(ui, text, font) <= max_w {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let mut n = chars.len();
    while n > 1 {
        n -= 1;
        let s: String = chars[..n].iter().collect::<String>() + "…";
        if widgets::text_width(ui, &s, font) <= max_w {
            return s;
        }
    }
    "…".to_string()
}

fn section_separator(ui: &Ui, full: Rect, y: f32, theme: &Theme) {
    let c = &theme.colors;
    if theme.gradients {
        for (i, v) in [206u8, 198, 190, 182].iter().enumerate() {
            let col = if theme.kind == crate::theme::ThemeKind::Normal { Color32::from_gray(*v) } else { crate::theme::mix(c.line_light, c.line_dark, i as f32 / 3.0) };
            ui.painter().hline(full.x_range(), y + i as f32 + 0.5, Stroke::new(1.0, col));
        }
    } else {
        ui.painter().hline(full.x_range(), y + 1.5, Stroke::new(1.0, c.line_dark));
    }
}

#[allow(clippy::too_many_arguments)]
fn tool_slot(ui: &mut Ui, st: &mut AppState, r: Rect, fly: &str, default: ToolId, group: ToolGroup, active: ToolId, now: f64) {
    let theme = Theme::current(ui.ctx());
    let members = tools::flyout_members(fly);
    let shown = slot_tool(st, fly, default);
    let selected = members.contains(&active);
    let shown = if selected { active } else { shown };
    let id = Id::new(("tool-slot", fly));
    let r = r.shrink(1.0);
    let resp = ui.interact(r, id, Sense::click_and_drag());
    widgets::button_frame(ui.painter(), r, &resp, ButtonLook { selected, enabled: true, group: Some(group) }, &theme);
    let ir = Rect::from_center_size(r.center(), vec2(theme.metrics.icon_size, theme.metrics.icon_size));
    icons::paint(ui.painter(), ir, tool_icon(shown), &IconStyle::new(&theme, Some(group), true));
    let has_flyout = members.len() > 1;
    if has_flyout {
        let col = if theme.colorful_icons { theme.accent(group) } else { theme.colors.icon_fg };
        widgets::corner_triangle(ui.painter(), r, col);
    }
    let tip = tool_tooltip(st, shown);
    crate::ui_kit::icon_button_info(&resp, true, &tip, Some(selected));
    let resp = resp.on_hover_text(tip);
    // Drücken und Halten (0,4 s) bzw. Klick auf das Dreieck öffnet den Flyout
    let mut open_flyout = false;
    if has_flyout {
        if resp.is_pointer_button_down_on() && ui.input(|i| i.pointer.press_start_time().is_some_and(|t| now - t > 0.4)) {
            open_flyout = true;
        }
        if resp.clicked()
            && let Some(p) = resp.interact_pointer_pos()
            && p.x > r.right() - 8.0
            && p.y > r.bottom() - 8.0
        {
            open_flyout = true;
        }
    }
    if open_flyout {
        if st.flyout.as_ref().is_none_or(|f| f.group != fly) {
            st.flyout = Some(FlyoutState { group: fly.to_string(), anchor: r, opened: now });
        }
    } else if resp.clicked() && st.flyout.is_none() {
        st.push(Action::SetTool(shown));
    }
    // wie im Original: Doppelklick auf das Zoom-Werkzeug = 100 %, auf die Hand = Alles einpassen
    if resp.double_clicked() {
        match shown {
            ToolId::Zoom => st.push(Action::Run("view.magnification.100".into(), json!({}))),
            ToolId::Hand => st.push(Action::Run("view.fitAll".into(), json!({}))),
            _ => {}
        }
    }
}

fn flyout_popup(ctx: &egui::Context, st: &mut AppState) {
    let Some(f) = st.flyout.clone() else { return };
    let theme = Theme::current(ctx);
    let c = theme.colors.clone();
    let members = tools::flyout_members(&f.group);
    let active = st.session.tools.active;
    let mut chosen: Option<ToolId> = None;
    let mut auto_shape: Option<String> = None;
    let pos = pos2(f.anchor.right() + 2.0, f.anchor.top());
    let area = egui::Area::new(Id::new("tool-flyout")).fixed_pos(pos).order(egui::Order::Foreground).show(ctx, |ui| {
        crate::menu::menu_frame(&theme).show(ui, |ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            // (Anzeigetext, Werkzeug, Trenner, Kennung der automatischen Form)
            let rows: Vec<(String, Option<ToolId>, bool, &str)> = {
                let mut v: Vec<(String, Option<ToolId>, bool, &str)> = members
                    .iter()
                    .filter(|t| **t != ToolId::AutoShape)
                    .map(|t| (tool_tooltip(st, *t), Some(*t), false, ""))
                    .collect();
                if f.group == flyout_of(ToolId::Rectangle) {
                    v.push((String::new(), None, true, ""));
                    for (de, en) in AUTO_SHAPES {
                        v.push((shape_label(en, de), Some(ToolId::AutoShape), false, de));
                    }
                }
                v
            };
            // Breite nach dem längsten Eintrag (Übersetzungen sind oft länger), mindestens 200
            let font = theme.menu_font();
            let width = rows.iter().map(|(t, ..)| crate::widgets::text_width(ui, t, &font) + 60.0).fold(200.0_f32, f32::max);
            for (text, tool, sep, shape) in rows {
                if sep {
                    let (r, _) = ui.allocate_exact_size(vec2(width, 9.0), Sense::hover());
                    ui.painter().hline((r.left() + 26.0)..=(r.right() - 6.0), r.center().y.floor() + 0.5, Stroke::new(1.0, c.menu_separator));
                    continue;
                }
                let resp = ui.allocate_response(vec2(width, 22.0), Sense::click());
                let r = resp.rect;
                if resp.hovered() {
                    ui.painter().rect_filled(r.shrink2(vec2(1.0, 0.0)), 0.0, c.menu_hover_bg);
                    ui.painter().rect_stroke(r.shrink2(vec2(1.0, 0.0)), 0.0, Stroke::new(1.0, c.menu_hover_border), StrokeKind::Inside);
                }
                if tool == Some(active) && tool != Some(ToolId::AutoShape) {
                    icons::paint(ui.painter(), Rect::from_center_size(pos2(r.left() + 10.0, r.center().y), vec2(11.0, 11.0)), Icon::Check, &IconStyle::mono(c.menu_check, c.menu_bg));
                }
                let icon = match tool {
                    Some(ToolId::AutoShape) => Icon::AutoShape,
                    Some(t) => tool_icon(t),
                    None => Icon::AutoShape,
                };
                let group = tool.and_then(tools::info).map(|i| ToolGroup::from_section(i.section));
                icons::paint(ui.painter(), Rect::from_center_size(pos2(r.left() + 30.0, r.center().y), vec2(15.0, 15.0)), icon, &IconStyle::new(&theme, group, true));
                ui.painter().text(pos2(r.left() + 46.0, r.center().y), Align2::LEFT_CENTER, &text, theme.menu_font(), c.menu_text);
                if resp.clicked() {
                    chosen = tool;
                    if tool == Some(ToolId::AutoShape) {
                        // das Werkzeug zieht die gewählte Form auf (autoShape.insert, fp-script);
                        // Kennung bleibt der deutsche Name, unabhängig von der Sprache
                        auto_shape = Some(shape.to_string());
                    }
                }
            }
        });
    });
    let clicked_outside = ctx.input(|i| i.pointer.any_pressed()) && ctx.pointer_interact_pos().is_some_and(|p| !area.response.rect.contains(p) && !f.anchor.contains(p));
    if let Some(t) = chosen {
        st.flyout = None;
        st.push(Action::SetTool(t));
        if let Some(s) = auto_shape {
            st.session.tools.options.auto_shape = s;
        }
    } else if clicked_outside || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        st.flyout = None;
    }
}

/// Abschnitt „Farben“: Strich- und Füllfarbfeld, darunter Standardfarben/Keine/Tauschen.
fn color_section(ui: &mut Ui, st: &mut AppState, full: Rect, mut y: f32, theme: &Theme) -> f32 {
    let c = &theme.colors;
    let cell = theme.metrics.tool_cell;
    let left = full.left() + (full.width() - 2.0 * cell.x) / 2.0;
    let well_id = Id::new("active-color-well");
    let active_well: ActiveWell = ui.ctx().data(|d| d.get_temp(well_id)).unwrap_or_default();
    for (is_stroke, icon, tip_key) in [(true, Icon::StrokeColor, "tool-tip-strokeColor"), (false, Icon::FillColor, "tool-tip-fillColor")] {
        let tip = fp_i18n::t(tip_key);
        let row = Rect::from_min_size(pos2(left, y), vec2(cell.x * 2.0, cell.y));
        let is_active = (active_well == ActiveWell::Stroke) == is_stroke;
        if is_active {
            if theme.glow {
                widgets::focus_ring(ui.painter(), row.shrink(1.0), theme, Some(ToolGroup::Colors));
            } else {
                ui.painter().rect_filled(row.shrink(1.0), 2.0, c.tool_selected_bg);
                ui.painter().rect_stroke(row.shrink(1.0), 2.0, Stroke::new(1.0, c.tool_selected_border), StrokeKind::Inside);
            }
        }
        icons::paint(ui.painter(), Rect::from_center_size(pos2(left + cell.x / 2.0, row.center().y), vec2(15.0, 15.0)), icon, &IconStyle::new(theme, Some(ToolGroup::Colors), true));
        let well = Rect::from_center_size(pos2(left + cell.x * 1.5, row.center().y), vec2(20.0, 18.0));
        let argb = if is_stroke { st.session.tools.stroke_color } else { st.session.tools.fill_color };
        widgets::color_well(ui.painter(), well, argb_to_color(argb), theme);
        let resp = ui.interact(row, Id::new(("well", is_stroke)), Sense::click()).on_hover_text(&tip);
        crate::ui_kit::icon_button_info(&resp, true, &tip, None);
        if resp.clicked() {
            ui.ctx().data_mut(|d| d.insert_temp(well_id, if is_stroke { ActiveWell::Stroke } else { ActiveWell::Fill }));
            // die Pipette setzt die Farbe des aktiven Felds (PAR 07/11)
            st.session.tools.options.stroke_well = is_stroke;
        }
        // Farbwähler-Popup des Originals (N27b): Fußleiste Füllung bzw. Kontur, Pipette ab dem Öffnen
        let open_at = resp.clicked().then(|| resp.interact_pointer_pos()).flatten().filter(|p| well.contains(*p)).map(|p| p.x);
        let has_sel = st.session.active().is_some_and(|d| !d.selection.elements.is_empty());
        let footer = if is_stroke { Footer::Stroke { placement: None, enabled: has_sel } } else { Footer::Fill { kinds: has_sel } };
        let shown_col = (argb.a() > 0).then_some(argb);
        let choice = crate::inspector::color_picker::show(ui, Id::new(("well", is_stroke)), well, open_at, st, shown_col, footer);
        let none = fp_color::Argb(0x00ff_ffff);
        let set = |c| if is_stroke { Action::SetColors { stroke: Some(c), fill: None } } else { Action::SetColors { stroke: None, fill: Some(c) } };
        // gewählte Vektoren/Texte (keine Bitmaps): die Farbe der Werkzeugleiste wirkt wie im
        // Original auch auf die Auswahl (CT-2 FARB-1b), ein Verlaufsschritt
        let vectors = st.session.active().is_some_and(|d| {
            d.selection.elements.iter().any(|id| fp_engine::docops::element_on_page(&d.doc, d.page, *id).is_some_and(|e| !matches!(e.kind, fp_model::ElementKind::Bitmap(_))))
        });
        match choice {
            Some(ColorChoice::Color(nc)) => {
                st.push(set(nc));
                let hex = fp_engine::commands::argb_hex(nc);
                if vectors {
                    if is_stroke {
                        st.run("pi.stroke", serde_json::json!({"color": hex}));
                    } else if st.session.tools.active == fp_engine::ToolId::PaintBucket {
                        // Füllwerkzeug: Schritt „Füllen“ mit einfarbiger Füllung (PAR 07/03)
                        st.run("pi.fill", serde_json::json!({"category": "solid", "color": hex}));
                    } else {
                        st.run("pi.fill", serde_json::json!({"color": hex}));
                    }
                }
            }
            Some(ColorChoice::None) => {
                st.push(set(none));
                if vectors {
                    if is_stroke {
                        st.run("pi.stroke", serde_json::json!({"none": true}));
                    } else {
                        st.run("pi.fill", serde_json::json!({"category": "none"}));
                    }
                }
            }
            Some(ColorChoice::Gradient) => st.run("pi.fill", serde_json::json!({"category": "gradient"})),
            Some(ColorChoice::Pattern) => {
                if let Some((_, p)) = crate::inspector::popups::pattern_files().first() {
                    st.run("element.setPattern", serde_json::json!({"image": p.to_string_lossy()}));
                }
            }
            Some(ColorChoice::Placement(k)) => st.run("pi.stroke", serde_json::json!({"placement": crate::inspector::color_picker::placement_name(k)})),
            Some(ColorChoice::Pipette) | None => {}
        }
        y += cell.y;
    }
    // Standardfarben, Keine, Tauschen
    let small = vec2(17.0, 16.0);
    let total = small.x * 3.0 + 4.0;
    let items = [(Icon::DefaultColors, "tool-tip-defaultColors"), (Icon::NoColor, "tool-tip-noColor"), (Icon::SwapColors, "tool-tip-swapColors")];
    for (i, (icon, tip)) in items.iter().enumerate() {
        let r = Rect::from_min_size(pos2(full.center().x - total / 2.0 + i as f32 * (small.x + 2.0), y + 2.0), small);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r));
        if widgets::icon_button(&mut child, *icon, small, 12.0, ButtonLook { selected: false, enabled: true, group: Some(ToolGroup::Colors) }, &fp_i18n::t(tip)).clicked() {
            match i {
                0 => {
                    let (s, f) = st.session.prefs.default_colors();
                    st.push(Action::SetColors { stroke: Some(s), fill: Some(f) });
                }
                1 => {
                    let none = fp_color::Argb(0x00ff_ffff);
                    st.push(if active_well == ActiveWell::Stroke { Action::SetColors { stroke: Some(none), fill: None } } else { Action::SetColors { stroke: None, fill: Some(none) } });
                }
                _ => {
                    let (s, f) = (st.session.tools.stroke_color, st.session.tools.fill_color);
                    st.push(Action::SetColors { stroke: Some(f), fill: Some(s) });
                }
            }
        }
    }
    y + small.y + 6.0
}
