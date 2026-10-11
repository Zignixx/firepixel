//! Gemeinsame Zeichen-Bausteine im Stil des Originals: Kopfzeilen mit Verlauf, Trennlinien, Symbolknöpfe,
//! Reiter, Griffleisten. Alle Farben kommen aus dem [`Theme`].

use crate::icons::{self, Icon, IconStyle};
use crate::theme::{with_alpha, Theme, ToolGroup};
use egui::{pos2, vec2, Align2, Color32, CornerRadius, FontId, Painter, Rect, Response, Sense, Stroke, StrokeKind, Ui};

/// Senkrechter Verlauf von `top` nach `bottom` in `rect`.
pub fn vgradient(painter: &Painter, rect: Rect, top: Color32, bottom: Color32) {
    if top == bottom {
        painter.rect_filled(rect, 0.0, top);
        return;
    }
    let mut mesh = egui::epaint::Mesh::default();
    mesh.colored_vertex(rect.left_top(), top);
    mesh.colored_vertex(rect.right_top(), top);
    mesh.colored_vertex(rect.left_bottom(), bottom);
    mesh.colored_vertex(rect.right_bottom(), bottom);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 2, 3);
    painter.add(egui::Shape::mesh(mesh));
}

/// Kopfzeile einer Bedienfeldgruppe bzw. des Inspektors (Lichtkante, Verlauf, dunkle Unterkante).
pub fn header_bar(painter: &Painter, rect: Rect, theme: &Theme) {
    let c = &theme.colors;
    if theme.gradients {
        vgradient(painter, rect, c.header_top, c.header_bottom);
        painter.hline(rect.x_range(), rect.top() + 0.5, Stroke::new(1.0, c.header_highlight));
    } else {
        painter.rect_filled(rect, 0.0, c.header_top);
    }
    painter.hline(rect.x_range(), rect.bottom() - 0.5, Stroke::new(1.0, c.header_line));
}

/// Dunkler Kopfstreifen über Werkzeugbedienfeld und Dock (mit »-Knopf).
pub fn dock_strip(painter: &Painter, rect: Rect, theme: &Theme) {
    let c = &theme.colors;
    if theme.gradients {
        vgradient(painter, rect, c.dock_strip_top, c.dock_strip_bottom);
    } else {
        painter.rect_filled(rect, 0.0, c.dock_strip_top);
    }
    painter.hline(rect.x_range(), rect.top() + 0.5, Stroke::new(1.0, c.line_dark));
    painter.hline(rect.x_range(), rect.bottom() - 0.5, Stroke::new(1.0, c.line_dark));
}

/// Dreifache Trennlinie (dunkel/mittel/dunkel) zwischen großen Bereichen, senkrecht.
pub fn vsplitter(painter: &Painter, x: f32, y_range: egui::Rangef, theme: &Theme) {
    let c = &theme.colors;
    painter.vline(x + 0.5, y_range, Stroke::new(1.0, c.line_dark));
    painter.vline(x + 1.5, y_range, Stroke::new(1.0, c.line_mid));
    painter.vline(x + 2.5, y_range, Stroke::new(1.0, c.line_dark));
}

/// Dreifache Trennlinie, waagerecht.
pub fn hsplitter(painter: &Painter, y: f32, x_range: egui::Rangef, theme: &Theme) {
    let c = &theme.colors;
    painter.hline(x_range, y + 0.5, Stroke::new(1.0, c.line_dark));
    painter.hline(x_range, y + 1.5, Stroke::new(1.0, c.line_mid));
    painter.hline(x_range, y + 2.5, Stroke::new(1.0, c.line_dark));
}

/// Gepunktete Griffleiste (Hauptsymbolleiste links, Werkzeugbedienfeld oben).
pub fn grip(painter: &Painter, rect: Rect, vertical: bool, theme: &Theme) {
    let c = &theme.colors;
    if vertical {
        let x = rect.center().x.floor();
        let mut y = rect.top() + 2.0;
        while y < rect.bottom() - 2.0 {
            painter.rect_filled(Rect::from_min_size(pos2(x, y), vec2(1.0, 1.0)), 0.0, c.grip_dark);
            painter.rect_filled(Rect::from_min_size(pos2(x + 1.0, y + 1.0), vec2(1.0, 1.0)), 0.0, c.grip_light);
            y += 2.0;
        }
    } else {
        let y = rect.center().y.floor();
        let mut x = rect.left() + 2.0;
        while x < rect.right() - 2.0 {
            painter.rect_filled(Rect::from_min_size(pos2(x, y), vec2(1.0, 1.0)), 0.0, c.grip_dark);
            painter.rect_filled(Rect::from_min_size(pos2(x + 1.0, y + 1.0), vec2(1.0, 1.0)), 0.0, c.grip_light);
            x += 2.0;
        }
    }
}

/// Zustand eines Symbolknopfs.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ButtonLook {
    pub selected: bool,
    pub enabled: bool,
    /// Gruppe für die OLED-Akzentfarbe.
    pub group: Option<ToolGroup>,
}

/// Hintergrund eines Symbolknopfs je nach Zustand (Hover, gedrückt, ausgewählt; OLED mit Schein).
pub fn button_frame(painter: &Painter, rect: Rect, resp: &Response, look: ButtonLook, theme: &Theme) {
    let c = &theme.colors;
    let accent = look.group.map(|g| theme.accent(g)).unwrap_or(c.focus);
    let hovered = look.enabled && resp.hovered();
    let pressed = look.enabled && resp.is_pointer_button_down_on();
    let r = CornerRadius::same(2);
    if look.selected {
        if theme.glow {
            painter.rect_filled(rect.expand(1.5), CornerRadius::same(4), with_alpha(accent, 40));
            painter.rect_filled(rect, r, with_alpha(accent, 34));
            painter.rect_stroke(rect, r, Stroke::new(1.0, accent), StrokeKind::Inside);
        } else {
            painter.rect_filled(rect, r, c.tool_selected_bg);
            painter.rect_stroke(rect, r, Stroke::new(1.0, c.tool_selected_border), StrokeKind::Inside);
        }
    } else if pressed {
        painter.rect_filled(rect, r, c.button_pressed_bg);
        painter.rect_stroke(rect, r, Stroke::new(1.0, if theme.glow { accent } else { c.button_hover_border }), StrokeKind::Inside);
    } else if hovered {
        if theme.glow {
            painter.rect_filled(rect, r, with_alpha(accent, 26));
            painter.rect_stroke(rect, r, Stroke::new(1.0, with_alpha(accent, 170)), StrokeKind::Inside);
        } else {
            painter.rect_filled(rect, r, c.tool_hover_bg);
            painter.rect_stroke(rect, r, Stroke::new(1.0, c.tool_hover_border), StrokeKind::Inside);
        }
    }
}

/// Symbolknopf fester Größe mit Tooltip. Liefert die Antwort (geklickt nur, wenn aktiv).
pub fn icon_button(ui: &mut Ui, icon: Icon, size: egui::Vec2, icon_size: f32, look: ButtonLook, tooltip: &str) -> Response {
    let theme = Theme::current(ui.ctx());
    let sense = if look.enabled { Sense::click() } else { Sense::hover() };
    let (rect, resp) = ui.allocate_exact_size(size, sense);
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        button_frame(painter, rect, &resp, look, &theme);
        let st = IconStyle::new(&theme, look.group, look.enabled);
        let ir = Rect::from_center_size(rect.center(), vec2(icon_size, icon_size));
        icons::paint(painter, ir, icon, &st);
    }
    if tooltip.is_empty() { resp } else { resp.on_hover_text(tooltip) }
}

/// Kleines Dreieck unten rechts (Flyout-Kennzeichen bzw. Aufklapp-Pfeil an Farbfeldern).
pub fn corner_triangle(painter: &Painter, rect: Rect, color: Color32) {
    let b = rect.right_bottom() - vec2(1.0, 1.0);
    painter.add(egui::Shape::convex_polygon(vec![pos2(b.x - 4.0, b.y), pos2(b.x, b.y), pos2(b.x, b.y - 4.0)], color, Stroke::NONE));
}

/// Text links ausgerichtet, abgeschnitten auf `max_w` (mit „…“ wie Windows).
pub fn text_clipped(painter: &Painter, pos: egui::Pos2, align: Align2, text: &str, font: FontId, color: Color32, max_w: f32) -> Rect {
    let fonts_w = |s: &str| painter.ctx().fonts_mut(|f| f.layout_no_wrap(s.to_string(), font.clone(), color).size().x);
    let mut t = text.to_string();
    if fonts_w(&t) > max_w {
        while !t.is_empty() && fonts_w(&format!("{t}…")) > max_w {
            t.pop();
        }
        t.push('…');
    }
    painter.text(pos, align, t, font, color)
}

/// Breite eines Textes.
pub fn text_width(ui: &Ui, text: &str, font: &FontId) -> f32 {
    ui.ctx().fonts_mut(|f| f.layout_no_wrap(text.to_string(), font.clone(), Color32::WHITE).size().x)
}

/// Farbfeld (Strich/Füllung) mit Aufklapp-Dreieck; transparente Farben zeigen den roten Schrägstrich.
pub fn color_well(painter: &Painter, rect: Rect, color: Color32, theme: &Theme) {
    let c = &theme.colors;
    painter.rect_filled(rect, 0.0, c.field_bg);
    painter.rect_stroke(rect, 0.0, Stroke::new(1.0, c.line_dark), StrokeKind::Inside);
    let inner = rect.shrink(2.0);
    if color.a() == 0 {
        painter.rect_filled(inner, 0.0, Color32::WHITE);
        painter.line_segment([inner.left_bottom(), inner.right_top()], Stroke::new(1.5, c.icon_red));
    } else {
        checker(painter, inner, 4.0, theme);
        painter.rect_filled(inner, 0.0, color);
        // innerer Rand, damit auch Schwarz auf schwarzem Grund (OLED) sichtbar bleibt
        painter.rect_stroke(inner, 0.0, Stroke::new(1.0, c.field_border), StrokeKind::Outside);
    }
    corner_triangle(painter, rect, c.icon_fg);
}

/// Transparenz-Schachbrett.
pub fn checker(painter: &Painter, rect: Rect, cell: f32, theme: &Theme) {
    let c = &theme.colors;
    painter.rect_filled(rect, 0.0, c.checker_a);
    let cols = (rect.width() / cell).ceil() as i32;
    let rows = (rect.height() / cell).ceil() as i32;
    if cols * rows > 40_000 {
        return;
    }
    for j in 0..rows {
        for i in 0..cols {
            if (i + j) % 2 == 1 {
                let r = Rect::from_min_size(pos2(rect.left() + i as f32 * cell, rect.top() + j as f32 * cell), vec2(cell, cell)).intersect(rect);
                painter.rect_filled(r, 0.0, c.checker_b);
            }
        }
    }
}

/// Kleiner Aufklapp-Pfeil (▼) zentriert in `rect`.
pub fn drop_arrow(painter: &Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    painter.add(egui::Shape::convex_polygon(vec![pos2(c.x - 3.0, c.y - 1.5), pos2(c.x + 3.0, c.y - 1.5), pos2(c.x, c.y + 1.5)], color, Stroke::NONE));
}

/// Fokus-/Leuchtrand (OLED) bzw. dezenter Rand (Normal).
pub fn focus_ring(painter: &Painter, rect: Rect, theme: &Theme, group: Option<ToolGroup>) {
    let accent = group.map(|g| theme.accent(g)).unwrap_or(theme.colors.focus);
    if theme.glow {
        painter.rect_stroke(rect.expand(1.0), 2.0, Stroke::new(2.0, with_alpha(accent, 90)), StrokeKind::Outside);
    }
    painter.rect_stroke(rect, 1.0, Stroke::new(1.0, accent), StrokeKind::Inside);
}
