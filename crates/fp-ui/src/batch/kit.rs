//! Bausteine des Batch-Studios im Stil des Startbildschirms (DESIGN §12.1): Karten mit Schatten
//! bzw. Leuchtrand, Banner mit Pixelfeld, moderne Knöpfe, Segment-Knopf, Schalter, Gruppenfarben
//! und -symbole der Schritte. Allgemeine Bausteine wandern später nach `ui_kit` (DES-3).

use crate::icons::{self, Icon, IconStyle};
use crate::theme::{mix, with_alpha, Theme, ThemeKind};
use crate::ui_kit::{self, sp, type_scale, KitIcon, RowIcon};
use egui::{pos2, vec2, Align2, Color32, CornerRadius, Id, Rect, Response, Sense, Stroke, StrokeKind, Ui};
use fp_batch::StepGroup;

/// Eckenradius der Karten.
pub const RADIUS: u8 = 8;
/// Kopf einer Karte (Symbolchip + Titel).
pub const HEAD_H: f32 = 34.0;
/// Innenabstand der Karten.
pub const PAD: f32 = 12.0;
/// Höhe der Knöpfe.
pub const BTN_H: f32 = 24.0;

// ---------------------------------------------------------------------- Farben

/// Kartenfläche (wie der Startbildschirm): Normal hell, Dunkel heller als die Leinwand, OLED schwarz.
pub fn card_fill(theme: &Theme) -> Color32 {
    let c = &theme.colors;
    match theme.kind {
        ThemeKind::Normal => ui_kit::card_bg(theme),
        ThemeKind::Dark => mix(c.menu_bg, c.frame_bg, 0.45),
        ThemeKind::Oled => c.frame_bg,
    }
}

/// Leitfarbe je Karte: 0 = Dateien (grün), 1 = Rezept (cyan), 2 = Vorschau (orange).
pub fn accent(theme: &Theme, card: usize) -> Color32 {
    let c = &theme.colors;
    if theme.glow {
        [c.group_accent[2], c.group_accent[0], c.group_accent[3]][card.min(2)]
    } else {
        [c.icon_green, c.focus, c.icon_yellow][card.min(2)]
    }
}

/// Getönte Fläche in einer Leitfarbe (Symbolchip, Hover).
pub fn tint(theme: &Theme, col: Color32, strength: f32) -> Color32 {
    let t = match theme.kind {
        ThemeKind::Normal => 0.14,
        ThemeKind::Dark => 0.22,
        ThemeKind::Oled => 0.22,
    } * strength;
    mix(card_fill(theme), col, t)
}

/// Gedämpfte Schrift (im hellen Design etwas kräftiger, wie der Startbildschirm).
pub fn weak(theme: &Theme) -> Color32 {
    let c = &theme.colors;
    if theme.kind == ThemeKind::Normal { mix(c.text_weak, c.text, 0.2) } else { c.text_weak }
}

/// Fläche eines Eintrags (Schrittkarte, Kachel) in Ruhe.
pub fn item_fill(theme: &Theme) -> Color32 {
    let c = &theme.colors;
    match theme.kind {
        ThemeKind::Normal => c.field_bg,
        ThemeKind::Dark => mix(card_fill(theme), c.field_bg, 0.35),
        ThemeKind::Oled => c.frame_bg,
    }
}

/// Rand eines Eintrags in Ruhe.
pub fn item_border(theme: &Theme, col: Color32) -> Color32 {
    let c = &theme.colors;
    match theme.kind {
        ThemeKind::Normal => mix(c.line_mid, c.canvas_bg, 0.6),
        ThemeKind::Dark => c.line_dark,
        ThemeKind::Oled => with_alpha(col, 70),
    }
}

/// Farbe einer Schrittgruppe (Leitstreifen, Symbolchip).
pub fn group_color(theme: &Theme, g: StepGroup) -> Color32 {
    let c = &theme.colors;
    let base = match g {
        StepGroup::Files => c.icon_green,
        StepGroup::Format => c.focus,
        StepGroup::Geometry => c.group_accent[0],
        StepGroup::Color => c.group_accent[1],
        StepGroup::Background => c.group_accent[2],
        StepGroup::Overlay => c.icon_yellow,
        StepGroup::Metadata => c.group_accent[4],
        StepGroup::Animation => c.group_accent[5],
        StepGroup::Optimize => c.group_accent[3],
        StepGroup::Document => c.icon_blue,
        StepGroup::Advanced => c.icon_red,
    };
    if theme.glow { base } else { mix(base, c.text, 0.12) }
}

/// Symbol einer Schrittgruppe.
pub fn group_icon(g: StepGroup) -> RowIcon {
    match g {
        StepGroup::Files => RowIcon::Icon(Icon::Folder),
        StepGroup::Format => RowIcon::Icon(Icon::Export),
        StepGroup::Geometry => RowIcon::Icon(Icon::Scale),
        StepGroup::Color => RowIcon::Icon(Icon::Swatches),
        StepGroup::Background => RowIcon::Icon(Icon::AiSubject),
        StepGroup::Overlay => RowIcon::Icon(Icon::Text),
        StepGroup::Metadata => RowIcon::Kit(KitIcon::Info),
        StepGroup::Animation => RowIcon::Icon(Icon::Play),
        StepGroup::Optimize => RowIcon::Icon(Icon::Star),
        StepGroup::Document => RowIcon::Kit(KitIcon::FwDocument),
        StepGroup::Advanced => RowIcon::Kit(KitIcon::Script),
    }
}

/// Zeichnet ein Symbol (Programm- oder Baukasten-Symbol).
pub fn paint_icon(p: &egui::Painter, r: Rect, icon: RowIcon, theme: &Theme, enabled: bool) {
    let st = IconStyle::new(theme, None, enabled);
    match icon {
        RowIcon::Icon(i) => icons::paint(p, r, i, &st),
        RowIcon::Kit(k) => ui_kit::paint_icon(p, r, k, &st),
        RowIcon::None => {}
    }
}

/// Einfarbiges Symbol.
pub fn paint_icon_mono(p: &egui::Painter, r: Rect, icon: RowIcon, col: Color32, theme: &Theme) {
    let st = IconStyle::mono(col, card_fill(theme));
    match icon {
        RowIcon::Icon(i) => icons::paint(p, r, i, &st),
        RowIcon::Kit(k) => ui_kit::paint_icon(p, r, k, &st),
        RowIcon::None => {}
    }
}

// ---------------------------------------------------------------------- Banner

struct BannerColors {
    top: Color32,
    bottom: Color32,
    text: Color32,
    weak: Color32,
    pixels: Vec<Color32>,
}

fn banner_colors(theme: &Theme) -> BannerColors {
    let c = &theme.colors;
    match theme.kind {
        ThemeKind::Normal => BannerColors { top: mix(mix(c.line_dark, c.text, 0.35), c.focus, 0.22), bottom: mix(c.text, c.focus, 0.14), text: c.field_bg, weak: mix(c.field_bg, c.focus, 0.25), pixels: vec![c.field_bg] },
        ThemeKind::Dark => BannerColors { top: mix(c.app_bar_bg, c.focus, 0.18), bottom: mix(c.canvas_bg, c.focus, 0.05), text: c.text, weak: c.text_weak, pixels: vec![c.text] },
        ThemeKind::Oled => BannerColors { top: c.frame_bg, bottom: c.frame_bg, text: c.text, weak: c.text_weak, pixels: c.group_accent.to_vec() },
    }
}

/// Schriftfarbe auf dem Banner (für Bedienelemente darauf).
pub fn banner_text(theme: &Theme) -> (Color32, Color32) {
    let b = banner_colors(theme);
    (b.text, b.weak)
}

fn hash01(x: i32, y: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(374_761_393) ^ (y as u32).wrapping_mul(668_265_263);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    ((h ^ (h >> 16)) & 0xffff) as f32 / 65535.0
}

/// Banner wie auf dem Startbildschirm (Fläche, Pixelfeld nach rechts dichter, Logo), mit Titel und
/// Untertitel; `text_right` = rechte Grenze der Schrift (rechts davon liegen Bedienelemente).
pub fn paint_banner(p: &egui::Painter, r: Rect, theme: &Theme, title: &str, subtitle: &str, text_right: f32) {
    let b = banner_colors(theme);
    let c = &theme.colors;
    let cr = CornerRadius::same(RADIUS);
    let p2 = p.with_clip_rect(r.intersect(p.clip_rect()));
    // Fläche mit Verlauf in runder Form: zwei Hälften (Mesh), dann die Ecken mit der Leinwand überdecken
    crate::widgets::vgradient(&p2, r, b.top, b.bottom);
    let small = r.height() < 64.0;
    let logo = if small { 32.0 } else { 40.0 };
    let x = r.left() + sp::L + logo + sp::M;
    let text_w = (text_right - x).max(40.0);
    let title_font = theme.bold(type_scale::TITLE + if small { 0.0 } else { 2.0 });
    let tw = ui_kit::text_w(p.ctx(), title, &title_font).max(ui_kit::text_w(p.ctx(), subtitle, &theme.font(type_scale::CAPTION + 0.5))).min(text_w);
    let text_end = x + tw + sp::L;
    let cell = 8.0;
    let cols = (r.width() / cell).floor() as i32;
    let rows = (r.height() / cell).floor() as i32;
    for j in 0..rows {
        for i in 0..cols {
            let fx = i as f32 / cols.max(1) as f32;
            let px = r.left() + i as f32 * cell;
            if fx < 0.30 || px < text_end {
                continue;
            }
            let v = hash01(i, j);
            let density = (fx - 0.30) / 0.70;
            if v > density * 0.5 {
                continue;
            }
            let k = ((hash01(j, i) * b.pixels.len() as f32) as usize).min(b.pixels.len() - 1);
            let a = if theme.glow { 30.0 + 100.0 * density * (1.0 - v) } else { 5.0 + 22.0 * density * (1.0 - v) };
            let q = Rect::from_min_size(pos2(px + 1.0, r.top() + j as f32 * cell + 1.0), vec2(cell - 2.0, cell - 2.0));
            p2.rect_filled(q, 0.0, with_alpha(b.pixels[k], a as u8));
        }
    }
    let lr = Rect::from_min_size(pos2(r.left() + sp::L, r.center().y - logo / 2.0), vec2(logo, logo));
    icons::paint_app_logo(&p2, lr);
    let th = title_font.size * 1.3;
    ui_kit::paint_text_fit(&p2, Rect::from_min_max(pos2(x, r.center().y - 1.0 - th), pos2(x + text_w, r.center().y - 1.0)), title, title_font, b.text);
    ui_kit::paint_text_fit(&p2, Rect::from_min_size(pos2(x, r.center().y + 2.0), vec2(text_w, 16.0)), subtitle, theme.font(type_scale::CAPTION + 0.5), b.weak);
    p.rect_stroke(r, cr, Stroke::new(4.0, c.canvas_bg), StrokeKind::Outside);
    if theme.glow {
        p.rect_stroke(r, cr, Stroke::new(1.0, with_alpha(c.focus, 200)), StrokeKind::Inside);
    }
}

// ---------------------------------------------------------------------- Karten

/// Karte mit Schatten, Rand und Kopf (Symbolchip in der Leitfarbe, Titel, rechts ein Zähler).
/// Liefert die Innenfläche unter dem Kopf.
pub fn card(p: &egui::Painter, r: Rect, theme: &Theme, col: Color32, icon: RowIcon, title: &str, right: Option<&str>) -> Rect {
    let c = &theme.colors;
    let cr = CornerRadius::same(RADIUS);
    if !theme.glow {
        let shadow = egui::Shadow { offset: [0, 2], blur: 10, spread: 0, color: Color32::from_black_alpha(if theme.kind == ThemeKind::Normal { 34 } else { 110 }) };
        p.add(shadow.as_shape(r, cr));
    }
    p.rect_filled(r, cr, card_fill(theme));
    let border = match theme.kind {
        ThemeKind::Normal => mix(c.line_mid, c.canvas_bg, 0.55),
        ThemeKind::Dark => c.line_dark,
        ThemeKind::Oled => with_alpha(col, 170),
    };
    p.rect_stroke(r, cr, Stroke::new(1.0, border), StrokeKind::Inside);
    if theme.glow {
        let strip = Rect::from_min_size(r.min + vec2(RADIUS as f32, 0.0), vec2((r.width() - 2.0 * RADIUS as f32).max(0.0), 2.0));
        p.rect_filled(strip, 1.0, col);
    }
    let chip = Rect::from_min_size(pos2(r.left() + PAD, r.top() + PAD - 2.0), vec2(24.0, 24.0));
    p.rect_filled(chip, CornerRadius::same(6), if theme.glow { with_alpha(col, 46) } else { tint(theme, col, 1.4) });
    paint_icon(p, Rect::from_center_size(chip.center(), vec2(16.0, 16.0)), icon, theme, true);
    let mut title_right = r.right() - PAD;
    if let Some(t) = right {
        let f = theme.font(type_scale::CAPTION);
        let w = ui_kit::text_w(p.ctx(), t, &f).min((r.width() * 0.45).max(20.0));
        let shown = ui_kit::elide(p.ctx(), t, &f, w + 0.5);
        p.text(pos2(r.right() - PAD, chip.center().y), Align2::RIGHT_CENTER, shown, f, weak(theme));
        title_right -= w + sp::S;
    }
    let tr = Rect::from_min_max(pos2(chip.right() + sp::S + 2.0, chip.top()), pos2(title_right.max(chip.right() + 20.0), chip.bottom()));
    let title_color = if theme.glow { col } else { c.text };
    ui_kit::paint_text_fit(p, tr, title, theme.bold(type_scale::SECTION + 1.0), title_color);
    Rect::from_min_max(pos2(r.left() + PAD, r.top() + PAD + HEAD_H - 4.0), pos2(r.right() - PAD, r.bottom() - PAD))
}

/// Fokusrahmen (Tastatur) um ein Element.
pub fn focus_ring(p: &egui::Painter, resp: &Response, r: Rect, theme: &Theme) {
    if resp.has_focus() {
        p.rect_stroke(r, CornerRadius::same(5), Stroke::new(1.5, theme.colors.focus), StrokeKind::Inside);
    }
}

/// Gestrichelter Rahmen (Ablegen, „Schritt hinzufügen“).
pub fn dashed_rect(p: &egui::Painter, r: Rect, col: Color32) {
    let dash = 6.0;
    let gap = 4.0;
    let s = Stroke::new(1.0, col);
    let seg = |a: egui::Pos2, b: egui::Pos2| {
        let len = (b - a).length();
        let dir = (b - a) / len.max(1.0);
        let mut t = 0.0;
        while t < len {
            let e = (t + dash).min(len);
            p.line_segment([a + dir * t, a + dir * e], s);
            t += dash + gap;
        }
    };
    let r = r.shrink(0.5);
    seg(r.left_top(), r.right_top());
    seg(r.right_top(), r.right_bottom());
    seg(r.right_bottom(), r.left_bottom());
    seg(r.left_bottom(), r.left_top());
}

// ---------------------------------------------------------------------- Knöpfe

/// Art eines Knopfs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BtnKind {
    /// Hauptknopf (Fläche `focus`, helle Schrift).
    Primary,
    /// Normaler Knopf (Fläche wie ein Eintrag, Rand).
    Secondary,
    /// Nur Text/Symbol, Fläche erst beim Überfahren.
    Ghost,
}

/// Breite eines Knopfs mit Beschriftung und optionalem Symbol.
pub fn btn_w(ctx: &egui::Context, label: &str, icon: bool) -> f32 {
    let theme = Theme::current(ctx);
    (ui_kit::text_w(ctx, label, &theme.font(type_scale::BODY)) + 2.0 * sp::M + if icon { 16.0 + sp::XS + 2.0 } else { 0.0 }).ceil()
}

/// Knopf im Stil des Studios (abgerundet, Hover getönt, Fokusrahmen, Handzeiger). `name` = Name für
/// Barrierefreiheit und Tooltip (falls die Beschriftung leer ist).
#[allow(clippy::too_many_arguments)]
pub fn button(ui: &mut Ui, rect: Rect, id: Id, label: &str, icon: Option<RowIcon>, kind: BtnKind, enabled: bool, name: &str) -> Response {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let resp = ui.interact(rect, id, if enabled { Sense::click() } else { Sense::hover() });
    let shown_name = if label.is_empty() { name } else { label };
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, shown_name));
    let hot = enabled && (resp.hovered() || resp.has_focus());
    let down = enabled && resp.is_pointer_button_down_on();
    let p = ui.painter();
    let cr = CornerRadius::same(6);
    let (bg, border, fg) = match kind {
        BtnKind::Primary => {
            let base = if theme.glow { with_alpha(c.focus, if down { 90 } else if hot { 70 } else { 46 }) } else if down { mix(c.focus, c.text, 0.2) } else if hot { mix(c.focus, c.field_bg, 0.12) } else { c.focus };
            let fg = if theme.glow { c.text } else { Color32::WHITE };
            if enabled { (base, if theme.glow { c.focus } else { mix(c.focus, c.text, 0.25) }, fg) } else { (mix(card_fill(&theme), c.text_disabled, 0.18), c.line_mid, c.text_disabled) }
        }
        BtnKind::Secondary => {
            let bg = if down { tint(&theme, c.focus, 1.6) } else if hot { tint(&theme, c.focus, 0.8) } else { item_fill(&theme) };
            (bg, if hot { mix(card_fill(&theme), c.focus, 0.55) } else { item_border(&theme, c.focus) }, if enabled { c.text } else { c.text_disabled })
        }
        BtnKind::Ghost => {
            let bg = if down { tint(&theme, c.focus, 1.4) } else if hot { tint(&theme, c.focus, 0.7) } else { Color32::TRANSPARENT };
            (bg, Color32::TRANSPARENT, if enabled { c.text } else { c.text_disabled })
        }
    };
    p.rect_filled(rect, cr, bg);
    if border != Color32::TRANSPARENT {
        p.rect_stroke(rect, cr, Stroke::new(1.0, border), StrokeKind::Inside);
    }
    if resp.has_focus() {
        p.rect_stroke(rect.shrink(1.0), CornerRadius::same(5), Stroke::new(1.5, if kind == BtnKind::Primary && !theme.glow { c.field_bg } else { c.focus }), StrokeKind::Inside);
    }
    let font = theme.font(type_scale::BODY);
    let lw = if label.is_empty() { 0.0 } else { ui_kit::text_w(ui.ctx(), label, &font) };
    let iw = if icon.is_some() { 16.0 } else { 0.0 };
    let gap = if icon.is_some() && !label.is_empty() { sp::XS + 2.0 } else { 0.0 };
    let avail = (rect.width() - 2.0 * sp::S).max(4.0);
    let total = (iw + gap + lw).min(avail);
    let mut x = rect.center().x - total / 2.0;
    if let Some(ic) = icon {
        let ir = Rect::from_min_size(pos2(x, rect.center().y - 8.0), vec2(16.0, 16.0));
        if kind == BtnKind::Primary && !theme.glow && enabled {
            paint_icon_mono(p, ir, ic, fg, &theme);
        } else {
            paint_icon(p, ir, ic, &theme, enabled);
        }
        x += iw + gap;
    }
    if !label.is_empty() {
        let shown = ui_kit::elide(ui.ctx(), label, &font, (rect.right() - sp::S - x).max(8.0));
        p.text(pos2(x, rect.center().y), Align2::LEFT_CENTER, shown, font, fg);
    }
    let resp = if resp.hovered() && enabled { resp.on_hover_cursor(egui::CursorIcon::PointingHand) } else { resp };
    if label.is_empty() && !name.is_empty() { resp.on_hover_text(name) } else { resp }
}

/// Kleiner Symbolknopf (20 × 20) ohne Fläche in Ruhe.
pub fn icon_button(ui: &mut Ui, rect: Rect, id: Id, icon: RowIcon, name: &str, enabled: bool) -> Response {
    button(ui, rect, id, "", Some(icon), BtnKind::Ghost, enabled, name)
}

/// Pfeil ▾ bzw. ▸ (Aufklappen) als Linien.
pub fn chevron(p: &egui::Painter, center: egui::Pos2, open: bool, col: Color32) {
    let s = Stroke::new(1.5, col);
    if open {
        p.line_segment([center + vec2(-4.0, -2.0), center + vec2(0.0, 2.0)], s);
        p.line_segment([center + vec2(0.0, 2.0), center + vec2(4.0, -2.0)], s);
    } else {
        p.line_segment([center + vec2(-2.0, -4.0), center + vec2(2.0, 0.0)], s);
        p.line_segment([center + vec2(2.0, 0.0), center + vec2(-2.0, 4.0)], s);
    }
}

/// Drei Punkte ⋯ (Menü).
pub fn dots(p: &egui::Painter, center: egui::Pos2, col: Color32) {
    for dx in [-5.0, 0.0, 5.0] {
        p.circle_filled(center + vec2(dx, 0.0), 1.5, col);
    }
}

/// Griff ⠿ (zum Ziehen).
pub fn grip(p: &egui::Painter, center: egui::Pos2, col: Color32) {
    for dy in [-5.0, 0.0, 5.0] {
        for dx in [-2.5, 2.5] {
            p.circle_filled(center + vec2(dx, dy), 1.2, col);
        }
    }
}

/// Segment-Knopf (z. B. Anfänger | Profi, Reiter der Vorschau). Liefert den neu gewählten Index.
/// `on_dark`: auf dem Banner (helle Schrift).
pub fn segmented(ui: &mut Ui, rect: Rect, id: Id, labels: &[String], selected: usize, on_banner: bool) -> Option<usize> {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let p = ui.painter().clone();
    let cr = CornerRadius::same(6);
    let (txt, txt_weak) = if on_banner { banner_text(&theme) } else { (c.text, weak(&theme)) };
    let track = if on_banner { with_alpha(Color32::BLACK, if theme.kind == ThemeKind::Normal { 60 } else { 90 }) } else { item_fill(&theme) };
    p.rect_filled(rect, cr, track);
    p.rect_stroke(rect, cr, Stroke::new(1.0, if on_banner { with_alpha(txt, 50) } else { item_border(&theme, c.focus) }), StrokeKind::Inside);
    let font = theme.font(type_scale::BODY);
    let ws: Vec<f32> = labels.iter().map(|l| ui_kit::text_w(ui.ctx(), l, &font) + 2.0 * sp::M).collect();
    let total: f32 = ws.iter().sum();
    let scale = if total > rect.width() - 4.0 { (rect.width() - 4.0) / total } else { 1.0 };
    let extra = if total < rect.width() - 4.0 { (rect.width() - 4.0 - total) / labels.len().max(1) as f32 } else { 0.0 };
    let mut x = rect.left() + 2.0;
    let mut out = None;
    for (i, l) in labels.iter().enumerate() {
        let w = ws[i] * scale + extra;
        let r = Rect::from_min_size(pos2(x, rect.top() + 2.0), vec2(w, rect.height() - 4.0));
        let resp = ui.interact(r, id.with(i), Sense::click());
        resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::RadioButton, true, i == selected, l));
        let on = i == selected;
        if on {
            let fill = if on_banner { with_alpha(Color32::WHITE, if theme.kind == ThemeKind::Normal { 230 } else { 40 }) } else if theme.glow { with_alpha(c.focus, 50) } else { card_fill(&theme) };
            p.rect_filled(r, CornerRadius::same(5), fill);
            if !on_banner && !theme.glow {
                p.rect_stroke(r, CornerRadius::same(5), Stroke::new(1.0, item_border(&theme, c.focus)), StrokeKind::Inside);
            }
        } else if resp.hovered() {
            p.rect_filled(r, CornerRadius::same(5), with_alpha(if on_banner { txt } else { c.focus }, 26));
        }
        if resp.has_focus() {
            p.rect_stroke(r, CornerRadius::same(5), Stroke::new(1.5, c.focus), StrokeKind::Inside);
        }
        let col = if on {
            if on_banner && theme.kind == ThemeKind::Normal { c.text } else if theme.glow { c.focus } else { txt }
        } else {
            txt_weak
        };
        let shown = ui_kit::elide(ui.ctx(), l, &font, (w - 2.0 * sp::XS).max(6.0));
        p.text(r.center(), Align2::CENTER_CENTER, shown, font.clone(), col);
        if resp.clicked() && !on {
            out = Some(i);
        }
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        x += w;
    }
    out
}

/// Schalter an/aus (30 × 16).
pub fn toggle(ui: &mut Ui, rect: Rect, id: Id, on: bool, name: &str) -> Response {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let resp = ui.interact(rect, id, Sense::click());
    resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, on, name));
    let p = ui.painter();
    let track = Rect::from_center_size(rect.center(), vec2(28.0, 16.0));
    let col = if on { if theme.glow { with_alpha(c.focus, 120) } else { c.focus } } else { mix(item_fill(&theme), c.text_weak, 0.35) };
    p.rect_filled(track, CornerRadius::same(8), col);
    let knob_x = if on { track.right() - 8.0 } else { track.left() + 8.0 };
    p.circle_filled(pos2(knob_x, track.center().y), 6.0, if theme.glow && on { c.focus } else { c.field_bg });
    if resp.has_focus() {
        p.rect_stroke(track.expand(2.0), CornerRadius::same(10), Stroke::new(1.5, c.focus), StrokeKind::Inside);
    }
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp.on_hover_text(name)
}

/// Zustandssymbol einer Dateizeile (wartet: nichts, läuft: Kreis, fertig ✓, Warnung ⚠, Fehler ✕).
pub fn status_mark(p: &egui::Painter, center: egui::Pos2, status: super::EntryStatus, theme: &Theme, time: f64) {
    use super::EntryStatus as S;
    let c = &theme.colors;
    let r = Rect::from_center_size(center, vec2(14.0, 14.0));
    match status {
        S::Waiting => {}
        S::Running => {
            let a0 = (time * 6.0) as f32;
            let pts: Vec<egui::Pos2> = (0..=16).map(|k| {
                let a = a0 + k as f32 / 16.0 * 4.5;
                center + vec2(a.cos(), a.sin()) * 5.5
            }).collect();
            p.add(egui::Shape::line(pts, Stroke::new(1.8, c.focus)));
        }
        S::Done => ui_kit::paint_icon(p, r, KitIcon::Success, &IconStyle::new(theme, None, true)),
        S::Warning | S::Skipped => ui_kit::paint_icon(p, r, KitIcon::Warning, &IconStyle::new(theme, None, true)),
        S::Failed => ui_kit::paint_icon(p, r, KitIcon::Error, &IconStyle::new(theme, None, true)),
        S::Cancelled => {
            p.circle_stroke(center, 5.0, Stroke::new(1.2, c.text_weak));
        }
    }
}

/// Text mit Kürzung an fester Stelle (links ausgerichtet, senkrecht mittig).
pub fn text_left(p: &egui::Painter, x: f32, y: f32, max_w: f32, text: &str, font: egui::FontId, col: Color32) -> f32 {
    let shown = ui_kit::elide(p.ctx(), text, &font, max_w.max(6.0));
    let w = ui_kit::text_w(p.ctx(), &shown, &font);
    p.text(pos2(x, y), Align2::LEFT_CENTER, shown, font, col);
    w
}
