//! Menüs, die höher als das Fenster sind (MN-6), wie native Windows-Menüs des Originals:
//!
//! * Das Menü wird auf den Platz im Fenster begrenzt: Hauptmenüs auf den Platz unter ihrem Titel,
//!   Kontext-, Bedienfeld- und Untermenüs auf die Fensterhöhe. egui schiebt das Popup dann ganz ins
//!   Fenster (nach oben bzw. auf die andere Seite des Eintrags).
//! * Oben und unten steht je eine Leiste mit Pfeil. Verweilen über dem Pfeil blättert fortlaufend,
//!   ein Klick blättert einen Eintrag weiter, das Mausrad blättert auch. Ein Klick auf einen Pfeil
//!   schließt das Menü nicht.
//! * Die Tastatursteuerung ([`crate::menu_kbd`]) scrollt den markierten Eintrag in Sicht.
//!
//! Menüs, die passen, laufen unverändert (kein zusätzlicher Code-Pfad).

use crate::menu::Node;
use crate::theme::Theme;
use egui::scroll_area::{ScrollBarVisibility, ScrollSource};
use egui::{pos2, vec2, Rect, Sense, Ui};

/// Höhe der Pfeilleisten oben und unten.
pub const ARROW_H: f32 = 16.0;
/// Höhe eines Trenners (siehe `menu::paint_separator`).
pub const SEPARATOR_H: f32 = 9.0;
/// Abstand des Menüs zum Fensterrand.
const EDGE: f32 = 2.0;
/// Rahmen des Menü-Popups oben + unten (Innenabstand 2 und Linie 1 je Seite, `menu::menu_frame`).
const FRAME_V: f32 = 6.0;
/// Verweilen über einem Pfeil: Einträge je Sekunde.
const HOVER_ITEMS_PER_S: f32 = 14.0;

/// Natürliche Höhe der Einträge (ohne Rahmen).
pub fn natural_height(ctx: &egui::Context, nodes: &[Node]) -> f32 {
    let ih = Theme::current(ctx).metrics.menu_item_height;
    nodes.iter().map(|n| if matches!(n, Node::Separator) { SEPARATOR_H } else { ih }).sum()
}

fn min_body(ctx: &egui::Context) -> f32 {
    2.0 * ARROW_H + 3.0 * Theme::current(ctx).metrics.menu_item_height
}

/// Höchste Höhe des Inhalts eines Menüs, das irgendwo im Fenster stehen darf (Kontext-,
/// Bedienfeld-, Untermenüs): die Fensterhöhe ohne Rand und Rahmen.
pub fn max_height(ctx: &egui::Context) -> f32 {
    (ctx.content_rect().height() - 2.0 * EDGE - FRAME_V).max(min_body(ctx))
}

/// Höchste Höhe des Inhalts eines Menüs, das unter `top` aufklappt (Hauptmenü unter seinem Titel).
pub fn max_height_below(ctx: &egui::Context, top: f32) -> f32 {
    (ctx.content_rect().bottom() - top - EDGE - FRAME_V).max(min_body(ctx))
}

/// Zeigt den Inhalt eines Menüs (`add`, natürliche Höhe `natural_h`). Ist er höher als `max_h`,
/// läuft er mit Pfeilleisten oben/unten und Bildlauf. Liefert `true`, wenn geblättert werden kann.
pub fn body(ui: &mut Ui, natural_h: f32, max_h: f32, add: impl FnOnce(&mut Ui)) -> bool {
    if natural_h <= max_h + 0.5 {
        add(ui);
        return false;
    }
    let theme = Theme::current(ui.ctx());
    let ih = theme.metrics.menu_item_height;
    let id = ui.make_persistent_id("fp-menu-scroll");
    let pending_id = id.with("pending");
    let pending: Option<f32> = ui.data_mut(|d| d.remove_temp(pending_id));
    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
    let view_h = (max_h - 2.0 * ARROW_H).max(ih);
    let (up_slot, _) = ui.allocate_exact_size(vec2(1.0, ARROW_H), Sense::hover());
    let mut sa = egui::ScrollArea::vertical()
        .id_salt(id.with("area"))
        .max_height(view_h)
        .min_scrolled_height(view_h)
        .auto_shrink([true, false])
        .scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden)
        .scroll_source(ScrollSource::MOUSE_WHEEL);
    if let Some(off) = pending {
        sa = sa.vertical_scroll_offset(off);
    }
    let o = sa.show(ui, add);
    let (down_slot, _) = ui.allocate_exact_size(vec2(1.0, ARROW_H), Sense::hover());
    let x = ui.min_rect().x_range();
    let up = Rect::from_x_y_ranges(x, up_slot.y_range());
    let down = Rect::from_x_y_ranges(x, down_slot.y_range());
    let offset = o.state.offset.y;
    let max_off = (o.content_size.y - o.inner_rect.height()).max(0.0);
    let can_up = offset > 0.5;
    let can_down = offset < max_off - 0.5;
    let ur = ui.interact(up, id.with("up"), Sense::click());
    let dr = ui.interact(down, id.with("down"), Sense::click());
    let c = &theme.colors;
    for (r, up_dir, on) in [(up, true, can_up), (down, false, can_down)] {
        let col = if on { c.menu_text } else { c.menu_text_disabled };
        let m = r.center();
        let (a, b) = if up_dir { (m.y + 2.5, m.y - 2.5) } else { (m.y - 2.5, m.y + 2.5) };
        ui.painter().add(egui::Shape::convex_polygon(vec![pos2(m.x - 5.0, a), pos2(m.x + 5.0, a), pos2(m.x, b)], col, egui::Stroke::NONE));
    }
    let dt = ui.input(|i| i.stable_dt).clamp(0.0, 0.1);
    let speed = ih * HOVER_ITEMS_PER_S * dt;
    let mut target = None;
    if can_up && ur.hovered() {
        target = Some(offset - speed - if ur.clicked() { ih } else { 0.0 });
    } else if can_down && dr.hovered() {
        target = Some(offset + speed + if dr.clicked() { ih } else { 0.0 });
    }
    if let Some(t) = target {
        ui.data_mut(|d| d.insert_temp(pending_id, t.clamp(0.0, max_off)));
        ui.ctx().request_repaint();
    }
    true
}
