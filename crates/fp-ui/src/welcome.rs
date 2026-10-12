//! Startbildschirm (Willkommensfenster), Agent DESIGN. Erscheint auf der Leinwandfläche, solange
//! kein Dokument offen ist – Aufbau wie der Startbildschirm des Originals, aber mit eigenem
//! Branding (Flammen-Pixel-Logo, eigene Symbole, eigene Ziele; keine Online-Angebote):
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────┐
//! │ [Logo] Firepixel                         · ·▪ ▪·▪ (Pixel)  │  Kopfbanner
//! │        Leitsatz                                             │
//! ├─────────────────────────────┬──────────────────────────────┤
//! │ Zuletzt geöffnete Dateien   │ Neu erstellen                │
//! │  ▣ datei.fw.png             │  ▣ Firepixel-Dokument (FW-PNG)│
//! │  …                          │  ▣ Aus Vorlage...             │
//! │ ─────────                   │ Erweitern                     │
//! │  ▣ Öffnen...                │  ▣ Gespeicherte Befehle...    │
//! ├─────────────────────────────┴──────────────────────────────┤
//! │ Kurzanleitung »   Tastenkürzel »   …              [Logo]    │
//! ├────────────────────────────────────────────────────────────┤
//! │ ☐ Nicht mehr anzeigen                           Version x  │
//! └────────────────────────────────────────────────────────────┘
//! ```
//!
//! „Nicht mehr anzeigen“ setzt die Voreinstellung `ShowStartPage` (wie im Original; wieder
//! einschalten über Voreinstellungen ▸ Allgemein). In der laufenden Sitzung bleibt der Bildschirm
//! sichtbar, damit sich das Häkchen zurücknehmen lässt. Texte: Präfix `welcome-` in
//! `locales\<sprache>\willkommen.ftl`. Bausteine aus [`crate::ui_kit`].

use crate::app_state::{Action, AppState};
use crate::icons::{self, Icon, IconStyle};
use crate::theme::{mix, with_alpha, Theme, ThemeKind};
use crate::ui_kit::{self, sp, type_scale, KitIcon, RowIcon};
use egui::{pos2, vec2, Align2, Color32, CornerRadius, Id, Rect, Sense, Stroke, StrokeKind, Ui};
use fp_i18n::t;
use serde_json::json;
use std::path::{Path, PathBuf};

/// Voreinstellung des Originals: Startbildschirm anzeigen.
pub const PREF_SHOW: &str = "ShowStartPage";
/// Höchstens so viele zuletzt geöffnete Dateien.
pub const MAX_RECENT: usize = 8;
/// Größte Breite der Karte.
const PANEL_MAX_W: f32 = 760.0;
/// Ab dieser Breite zwei Spalten.
const TWO_COLUMNS_MIN_W: f32 = 540.0;
const TITLE_H: f32 = 26.0;
const LINK_H: f32 = 22.0;
const FOOTER_H: f32 = 40.0;

/// Hat der Nutzer „Nicht mehr anzeigen“ in dieser Sitzung angekreuzt? Dann bleibt der Bildschirm
/// bis zum Neustart sichtbar.
fn ticked_id() -> Id {
    Id::new("fp-welcome-ticked-this-session")
}

/// Wird der Startbildschirm gezeigt?
pub fn is_visible(ctx: &egui::Context, st: &AppState) -> bool {
    st.session.prefs.get_bool(PREF_SHOW, true) || ctx.data(|d| d.get_temp::<bool>(ticked_id())).unwrap_or(false)
}

/// Ein Eintrag der Liste „Zuletzt geöffnet“.
#[derive(Clone, Debug, PartialEq)]
pub struct RecentItem {
    pub path: PathBuf,
    pub name: String,
    pub icon: RowIcon,
    pub exists: bool,
}

/// Symbol nach Dateiart.
pub fn file_icon(path: &Path) -> RowIcon {
    let name = path.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
    if name.ends_with(".fw.png") {
        return RowIcon::Kit(KitIcon::FwDocument);
    }
    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "tif" | "tiff" | "avif" | "psd" | "ico" | "qoi" | "wbmp" | "tga" => RowIcon::Kit(KitIcon::Image),
        "jsf" | "js" => RowIcon::Kit(KitIcon::Script),
        _ => RowIcon::Icon(Icon::Document),
    }
}

/// Liste „Zuletzt geöffnet“ aus den Voreinstellungen.
pub fn recent_items(st: &AppState) -> Vec<RecentItem> {
    st.session
        .prefs
        .recent_files
        .iter()
        .take(MAX_RECENT)
        .map(|p| RecentItem { path: p.clone(), name: p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| p.display().to_string()), icon: file_icon(p), exists: p.exists() })
        .collect()
}

/// Einträge der Spalte „Neu erstellen“ / „Erweitern“: (Symbol, Textschlüssel, Befehl).
fn create_entries() -> [(KitIcon, &'static str, &'static str); 2] {
    [(KitIcon::NewDocument, "welcome-new-document", "file.new"), (KitIcon::Template, "welcome-new-template", "file.newFromTemplate")]
}

fn extend_entries(st: &AppState) -> Vec<(KitIcon, &'static str, &'static str)> {
    [(KitIcon::Commands, "welcome-extend-commands", "commands.manageSavedCommands"), (KitIcon::Extend, "welcome-extend-extensions", "commands.manageExtensions")]
        .into_iter()
        .filter(|(_, _, c)| st.session.registry().get(c).is_some())
        .collect()
}

/// Links unten: (Symbol, Textschlüssel, Befehl, Parameter).
fn help_links() -> [(KitIcon, &'static str, &'static str, serde_json::Value); 4] {
    [
        (KitIcon::Book, "welcome-help-guide", "help.firepixelHelp", json!({ "topic": "start" })),
        (KitIcon::Keyboard, "welcome-help-shortcuts", "edit.keyboardShortcuts", json!({})),
        (KitIcon::AiChip, "welcome-help-models", "ai.manageModels", json!({})),
        (KitIcon::Gear, "welcome-help-automation", "help.firepixelHelp", json!({ "topic": "automation" })),
    ]
}

/// Farben des Kopfbanners (aus dem Design abgeleitet): oben, unten, Schrift, Leitsatz, Pixel.
struct Banner {
    top: Color32,
    bottom: Color32,
    text: Color32,
    weak: Color32,
    pixels: Vec<Color32>,
}

fn banner_colors(theme: &Theme) -> Banner {
    let c = &theme.colors;
    match theme.kind {
        // dunkle, ruhige Fläche mit leichtem Blaustich aus der Fokusfarbe
        ThemeKind::Normal => Banner {
            top: mix(mix(c.line_dark, c.text, 0.35), c.focus, 0.22),
            bottom: mix(c.text, c.focus, 0.14),
            text: c.field_bg,
            weak: mix(c.field_bg, c.focus, 0.25),
            pixels: vec![c.field_bg],
        },
        ThemeKind::Dark => Banner {
            top: mix(c.app_bar_bg, c.focus, 0.18),
            bottom: mix(c.canvas_bg, c.focus, 0.05),
            text: c.text,
            weak: c.text_weak,
            pixels: vec![c.text],
        },
        // OLED: Schwarz, Pixel in den Gruppenfarben
        ThemeKind::Oled => Banner { top: c.frame_bg, bottom: c.frame_bg, text: c.text, weak: c.text_weak, pixels: c.group_accent.to_vec() },
    }
}

/// Kleiner deterministischer Zufall (Pixelmuster ohne Flackern).
fn hash01(x: i32, y: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(374_761_393) ^ (y as u32).wrapping_mul(668_265_263);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    ((h ^ (h >> 16)) & 0xffff) as f32 / 65535.0
}

fn paint_banner(p: &egui::Painter, r: Rect, theme: &Theme, narrow: bool) {
    let b = banner_colors(theme);
    crate::widgets::vgradient(p, r, b.top, b.bottom);
    // Struktur: Pixelfeld, das nach rechts dichter wird (eigenes Motiv „Funken aus Pixeln“)
    let cell = 8.0;
    let cols = (r.width() / cell).floor() as i32;
    let rows = (r.height() / cell).floor() as i32;
    let p2 = p.with_clip_rect(r);
    // Maße von Logo und Schrift vorab: das Pixelfeld bleibt rechts der Schrift
    let tiny = r.width() < 300.0;
    let logo = if tiny { 36.0 } else if narrow { 48.0 } else { 64.0 };
    let margin = if narrow { sp::L } else { sp::XL };
    let x = r.left() + margin + logo + if tiny { sp::S } else { sp::L };
    let max_w = (r.right() - margin - x).max(40.0);
    let name_size = if tiny { 18.0 } else if narrow { 24.0 } else { type_scale::DISPLAY };
    let tagline = t("welcome-tagline");
    let text_end = x + ui_kit::text_w(p.ctx(), &tagline, &theme.font(12.5)).max(ui_kit::text_w(p.ctx(), "Firepixel", &theme.bold(name_size))).min(max_w) + sp::L;
    for j in 0..rows {
        for i in 0..cols {
            let fx = i as f32 / cols.max(1) as f32;
            if fx < 0.42 || r.left() + i as f32 * cell < text_end {
                continue;
            }
            let v = hash01(i, j);
            let density = (fx - 0.42) / 0.58;
            if v > density * 0.55 {
                continue;
            }
            let k = ((hash01(j, i) * b.pixels.len() as f32) as usize).min(b.pixels.len() - 1);
            let a = if theme.glow { 40.0 + 110.0 * density * (1.0 - v) } else { 6.0 + 26.0 * density * (1.0 - v) };
            let q = Rect::from_min_size(pos2(r.left() + i as f32 * cell + 1.0, r.top() + j as f32 * cell + 1.0), vec2(cell - 2.0, cell - 2.0));
            p2.rect_filled(q, 0.0, with_alpha(b.pixels[k], a as u8));
        }
    }
    // Logo, Name, Leitsatz
    let lr = Rect::from_min_size(pos2(r.left() + margin, r.center().y - logo / 2.0), vec2(logo, logo));
    icons::paint_app_logo(p, lr);
    ui_kit::paint_text_fit(p, Rect::from_min_max(pos2(x, r.center().y - 2.0 - name_size * 1.3), pos2(x + max_w, r.center().y - 2.0)), "Firepixel", theme.bold(name_size), b.text);
    ui_kit::paint_text_fit(p, Rect::from_min_size(pos2(x, r.center().y + 2.0), vec2(max_w, 18.0)), &tagline, theme.font(12.5), b.weak);
    if theme.glow {
        p.hline(r.x_range(), r.bottom() - 0.5, Stroke::new(1.0, theme.colors.focus));
    }
}

/// Maße des Startbildschirms für eine verfügbare Fläche.
#[derive(Clone, Copy, Debug)]
struct Layout {
    w: f32,
    two: bool,
    pad: f32,
    banner_h: f32,
    col_w: f32,
    left_h: f32,
    right_h: f32,
    cols_h: f32,
    bottom_h: f32,
    total_h: f32,
}

fn layout(ctx: &egui::Context, avail: egui::Vec2, n_recent: usize, n_extend: usize) -> Layout {
    let w = (avail.x - 2.0 * sp::L).clamp(200.0, PANEL_MAX_W).floor();
    let two = w >= TWO_COLUMNS_MIN_W;
    let narrow = w < 420.0;
    let pad = if two { sp::XL } else { sp::L };
    let banner_h = if narrow { 84.0 } else { 108.0 };
    let col_w = if two { (w - 2.0 * pad - sp::XL) / 2.0 } else { w - 2.0 * pad };
    let empty_h = ui_kit::empty_state_height(ctx, &t("welcome-recent-empty"), Some(&t("welcome-recent-empty-hint")), col_w);
    let list_h = if n_recent == 0 { empty_h } else { n_recent as f32 * sp::ROW };
    let left_h = TITLE_H + list_h + sp::S + 2.0 + sp::S + sp::ROW;
    let right_h = TITLE_H + 2.0 * sp::ROW + if n_extend > 0 { sp::L + TITLE_H + n_extend as f32 * sp::ROW } else { 0.0 };
    let cols_h = if two { left_h.max(right_h) } else { left_h + sp::L + right_h };
    let links = help_links().len() as f32;
    let bottom_h = if two { (links * LINK_H).max(64.0 + 20.0) } else { links * LINK_H };
    let total_h = banner_h + pad + cols_h + pad + 2.0 + sp::L + bottom_h + sp::L + 2.0 + FOOTER_H;
    Layout { w, two, pad, banner_h, col_w, left_h, right_h, cols_h, bottom_h, total_h }
}

/// Zeichnet den Startbildschirm in `rect` (Leinwandfläche ohne Dokument). Ist er abgeschaltet,
/// bleibt die Fläche leer.
pub fn show(ui: &mut Ui, st: &mut AppState, rect: Rect) {
    let theme = Theme::current(ui.ctx());
    let c = theme.colors.clone();
    ui.painter().rect_filled(rect, 0.0, c.canvas_bg);
    if !is_visible(ui.ctx(), st) || rect.width() < 60.0 || rect.height() < 60.0 {
        return;
    }
    let recent = recent_items(st);
    let extend = extend_entries(st);
    let lay = layout(ui.ctx(), rect.size(), recent.len(), extend.len());
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(egui::Layout::top_down(egui::Align::Min)));
    child.set_clip_rect(rect.intersect(ui.clip_rect()));
    egui::ScrollArea::both().id_salt("fp-welcome-scroll").auto_shrink([false, false]).show(&mut child, |ui| {
        let need = vec2(lay.w + 2.0 * sp::L, lay.total_h + 2.0 * sp::L);
        let (outer, _) = ui.allocate_exact_size(vec2(need.x.max(rect.width()), need.y.max(rect.height())), Sense::hover());
        let panel = Rect::from_min_size(pos2(outer.left() + ((outer.width() - lay.w) / 2.0).max(sp::L).round(), outer.top() + ((outer.height() - lay.total_h) / 2.0).max(sp::L).round()), vec2(lay.w, lay.total_h));
        draw_panel(ui, st, &theme, &lay, panel, &recent, &extend);
    });
}

#[allow(clippy::too_many_arguments)]
fn draw_panel(ui: &mut Ui, st: &mut AppState, theme: &Theme, lay: &Layout, panel: Rect, recent: &[RecentItem], extend: &[(KitIcon, &'static str, &'static str)]) {
    let c = &theme.colors;
    let p = ui.painter().clone();
    // Karte mit Schatten
    let shadow = egui::Shadow { offset: [0, 3], blur: 14, spread: 0, color: Color32::from_black_alpha(if theme.kind == ThemeKind::Normal { 45 } else { 120 }) };
    p.add(shadow.as_shape(panel, CornerRadius::same(4)));
    p.rect_filled(panel, CornerRadius::same(4), ui_kit::card_bg(theme));
    let banner = Rect::from_min_size(panel.min, vec2(panel.width(), lay.banner_h));
    {
        let pc = p.with_clip_rect(Rect::from_min_max(panel.min, pos2(panel.right(), banner.bottom())));
        // Banner mit runden oberen Ecken: Ecken über die Karte zeichnen lassen
        paint_banner(&pc, banner, theme, lay.w < 420.0);
    }
    let x0 = panel.left() + lay.pad;
    let mut y = banner.bottom() + lay.pad;
    // ------------------------------------------------ Spalten
    let left = Rect::from_min_size(pos2(x0, y), vec2(lay.col_w, lay.left_h));
    let right = if lay.two { Rect::from_min_size(pos2(x0 + lay.col_w + sp::XL, y), vec2(lay.col_w, lay.right_h)) } else { Rect::from_min_size(pos2(x0, y + lay.left_h + sp::L), vec2(lay.col_w, lay.right_h)) };
    if lay.two {
        ui_kit::vline(&p, x0 + lay.col_w + sp::XL / 2.0 - 1.0, egui::Rangef::new(y, y + lay.cols_h), theme);
    }
    // links: zuletzt geöffnet
    ui_kit::section_at(&p, Rect::from_min_size(left.min, vec2(left.width(), 18.0)), &t("welcome-recent-title"), theme);
    let mut ly = left.top() + TITLE_H;
    if recent.is_empty() {
        let h = ui_kit::empty_state_height(ui.ctx(), &t("welcome-recent-empty"), Some(&t("welcome-recent-empty-hint")), left.width());
        ui_kit::empty_state_at(&p, Rect::from_min_size(pos2(left.left(), ly), vec2(left.width(), h)), KitIcon::EmptyTray, &t("welcome-recent-empty"), Some(&t("welcome-recent-empty-hint")), theme);
        ly += h;
    }
    for (i, it) in recent.iter().enumerate() {
        let r = Rect::from_min_size(pos2(left.left(), ly), vec2(left.width(), sp::ROW));
        let tip = if it.exists { it.path.display().to_string() } else { format!("{}\n{}", t("welcome-recent-missing"), it.path.display()) };
        if ui_kit::link_row_at(ui, r, Id::new(("fp-welcome-recent", i)), it.icon, &it.name, Some(&tip), it.exists).clicked() {
            st.push(Action::OpenPath(it.path.clone()));
        }
        ly += sp::ROW;
    }
    ly += sp::S;
    ui_kit::hline(&p, egui::Rangef::new(left.left(), left.right()), ly, theme);
    ly += 2.0 + sp::S;
    let r = Rect::from_min_size(pos2(left.left(), ly), vec2(left.width(), sp::ROW));
    if ui_kit::link_row_at(ui, r, Id::new("fp-welcome-open"), RowIcon::Icon(Icon::Open), &t("welcome-open"), Some(&t("welcome-open.tooltip")), true).clicked() {
        st.run("file.open", json!({}));
    }
    // rechts: neu erstellen, erweitern
    ui_kit::section_at(&p, Rect::from_min_size(right.min, vec2(right.width(), 18.0)), &t("welcome-new-title"), theme);
    let mut ry = right.top() + TITLE_H;
    for (icon, key, cmd) in create_entries() {
        let r = Rect::from_min_size(pos2(right.left(), ry), vec2(right.width(), sp::ROW));
        if ui_kit::link_row_at(ui, r, Id::new(("fp-welcome-new", key)), RowIcon::Kit(icon), &t(key), Some(&t(&format!("{key}.tooltip"))), true).clicked() {
            st.run(cmd, json!({}));
        }
        ry += sp::ROW;
    }
    if !extend.is_empty() {
        ry += sp::L;
        ui_kit::section_at(&p, Rect::from_min_size(pos2(right.left(), ry), vec2(right.width(), 18.0)), &t("welcome-extend-title"), theme);
        ry += TITLE_H;
        for (icon, key, cmd) in extend {
            let r = Rect::from_min_size(pos2(right.left(), ry), vec2(right.width(), sp::ROW));
            if ui_kit::link_row_at(ui, r, Id::new(("fp-welcome-extend", *key)), RowIcon::Kit(*icon), &t(key), Some(&t(&format!("{key}.tooltip"))), true).clicked() {
                st.run(cmd, json!({}));
            }
            ry += sp::ROW;
        }
    }
    y += lay.cols_h + lay.pad;
    // ------------------------------------------------ unten: Links und Logo
    ui_kit::hline(&p, egui::Rangef::new(panel.left() + lay.pad, panel.right() - lay.pad), y, theme);
    y += 2.0 + sp::L;
    let logo_w = if lay.two { 64.0 + sp::XL } else { 0.0 };
    let links_w = panel.width() - 2.0 * lay.pad - logo_w;
    for (k, (icon, key, cmd, params)) in help_links().into_iter().enumerate() {
        let r = Rect::from_min_size(pos2(x0, y + k as f32 * LINK_H), vec2(links_w, LINK_H));
        // Symbol vor dem »-Link
        paint_small_icon(&p, Rect::from_center_size(pos2(r.left() + 8.0, r.center().y), vec2(16.0, 16.0)), icon, theme);
        let lr = Rect::from_min_max(pos2(r.left() + 16.0 + sp::S, r.top()), r.max);
        if ui_kit::more_link_at(ui, lr, Id::new(("fp-welcome-help", k)), &t(key), Some(&t(&format!("{key}.tooltip")))).clicked() {
            st.run(cmd, params);
        }
    }
    if lay.two {
        let lr = Rect::from_min_size(pos2(panel.right() - lay.pad - 64.0, y), vec2(64.0, 64.0));
        icons::paint_app_logo(&p, lr);
        let ver = fp_i18n::tr!("welcome-version", version = env!("CARGO_PKG_VERSION"));
        p.text(pos2(lr.center().x, lr.bottom() + sp::XS), Align2::CENTER_TOP, ver, theme.font(type_scale::CAPTION), c.text_weak);
    }
    y += lay.bottom_h + sp::L;
    // ------------------------------------------------ Fußzeile: Nicht mehr anzeigen
    ui_kit::hline(&p, panel.x_range(), y, theme);
    y += 2.0;
    let foot = Rect::from_min_max(pos2(panel.left(), y), pos2(panel.right(), panel.bottom()));
    p.rect_filled(foot, CornerRadius { nw: 0, ne: 0, sw: 4, se: 4 }, if theme.glow { c.frame_bg } else { mix(ui_kit::card_bg(theme), c.frame_bg, 0.5) });
    let mut dont = !st.session.prefs.get_bool(PREF_SHOW, true);
    let cb_r = Rect::from_min_max(pos2(x0, foot.top()), pos2(foot.right() - lay.pad, foot.bottom()));
    let mut cu = ui.new_child(egui::UiBuilder::new().max_rect(cb_r).layout(egui::Layout::left_to_right(egui::Align::Center)));
    let resp = cu.add(egui::Checkbox::new(&mut dont, t("welcome-dont-show"))).on_hover_text(t("welcome-dont-show.tooltip"));
    if resp.changed() {
        st.session.prefs.set(PREF_SHOW, fp_engine::PrefValue::Bool(!dont));
        st.session.prefs_changed();
        ui.ctx().data_mut(|d| d.insert_temp(ticked_id(), true));
    }
    p.rect_stroke(panel, CornerRadius::same(4), Stroke::new(1.0, if theme.glow { with_alpha(c.focus, 200) } else { c.line_dark }), StrokeKind::Outside);
}

fn paint_small_icon(p: &egui::Painter, r: Rect, icon: KitIcon, theme: &Theme) {
    ui_kit::paint_icon(p, r, icon, &IconStyle::new(theme, None, true));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_icons_by_type() {
        assert_eq!(file_icon(Path::new(r"C:\a\b.fw.png")), RowIcon::Kit(KitIcon::FwDocument));
        assert_eq!(file_icon(Path::new(r"C:\a\B.FW.PNG")), RowIcon::Kit(KitIcon::FwDocument));
        assert_eq!(file_icon(Path::new(r"C:\a\b.jpg")), RowIcon::Kit(KitIcon::Image));
        assert_eq!(file_icon(Path::new(r"C:\a\b.png")), RowIcon::Kit(KitIcon::Image));
        assert_eq!(file_icon(Path::new(r"C:\a\b.jsf")), RowIcon::Kit(KitIcon::Script));
        assert_eq!(file_icon(Path::new(r"C:\a\b.xyz")), RowIcon::Icon(Icon::Document));
    }

    #[test]
    fn layout_fits_and_switches_columns() {
        crate::ui_kit::tests::with_ctx(|ctx| {
            let wide = layout(ctx, vec2(1200.0, 800.0), 8, 2);
            assert!(wide.two && wide.w <= PANEL_MAX_W);
            let narrow = layout(ctx, vec2(400.0, 500.0), 0, 2);
            assert!(!narrow.two && narrow.w <= 400.0 - 2.0 * sp::L + 0.5);
            assert!(narrow.total_h > wide.total_h - 200.0);
        });
    }
}
