//! Startbildschirm (Willkommensfenster), Agent DESIGN, überarbeitet von DES-2 (Nutzerbefund N11:
//! „ohne Liebe“). Erscheint auf der Leinwandfläche, solange kein Dokument offen ist. Inhalt wie der
//! Startbildschirm des Originals, eigenes Branding (Flammen-Pixel-Logo, eigene Symbole, eigene
//! Ziele; keine Online-Angebote):
//!
//! ```text
//! ╭──────────────────────────────────────────────────────────────╮
//! │ [Logo] Firepixel Studio                   · ·▪ ▪·▪ (Pixel)   │  Kopfzeile
//! │        Leitsatz                                              │
//! ╰──────────────────────────────────────────────────────────────╯
//! ╭─ ▣ Neu erstellen ──────╮ ╭─ ▣ Zuletzt geöffnete Dateien ─────╮
//! │ [▣] Firepixel-Dokument │ │ [Vorschau] datei.fw.png           │
//! │     leeres Dokument    │ │            C:\Pfad\…  (gedämpft)  │
//! │ [▣] Aus Vorlage...     │ │ …                                 │
//! ╰────────────────────────╯ │ ───────                           │
//! ╭─ ▣ Hilfe und Lernen ───╮ │ ▣ Öffnen...                       │
//! │ ▣ Kurzanleitung »      │ │                                   │
//! │ …   Erweitern …        │ │                                   │
//! ╰────────────────────────╯ ╰───────────────────────────────────╯
//!  ☐ Nicht mehr anzeigen                              Version x
//! ```
//!
//! Drei Karten in zwei Spalten ab [`TWO_COLUMNS_MIN_W`], darunter einspaltig (Neu, Zuletzt, Hilfe);
//! Bildlauf, wenn die Fläche nicht reicht. Jeder Eintrag ist mit Tab/Umschalt+Tab und den
//! Pfeiltasten erreichbar und mit Eingabe/Leertaste auslösbar (Fokusrahmen in `focus`).
//! „Nicht mehr anzeigen“ setzt die Voreinstellung `ShowStartPage` (wie im Original; wieder
//! einschalten über Voreinstellungen ▸ Allgemein). In der laufenden Sitzung bleibt der Bildschirm
//! sichtbar, damit sich das Häkchen zurücknehmen lässt. Texte: Präfix `welcome-` in
//! `locales\<sprache>\welcome.ftl`. Bausteine aus [`crate::ui_kit`], Vorschaubilder aus
//! [`thumbs`].

mod thumbs;

use crate::app_state::{Action, AppState};
use crate::icons::{self, Icon, IconStyle};
use crate::theme::{Theme, ThemeKind, mix, with_alpha};
use crate::ui_kit::{self, KitIcon, RowIcon, sp, type_scale};
use egui::{
    Align2, Color32, CornerRadius, Id, Rect, Response, Sense, Stroke, StrokeKind, Ui, pos2, vec2,
};
use fp_i18n::t;
use serde_json::json;
use std::path::{Path, PathBuf};

/// Voreinstellung des Originals: Startbildschirm anzeigen.
pub const PREF_SHOW: &str = "ShowStartPage";
/// Höchstens so viele zuletzt geöffnete Dateien.
pub const MAX_RECENT: usize = 8;
/// Größte Breite des Inhalts.
const PANEL_MAX_W: f32 = 940.0;
/// Ab dieser Breite zwei Spalten.
const TWO_COLUMNS_MIN_W: f32 = 600.0;
/// Abstand zwischen den Karten.
const GAP: f32 = sp::L;
/// Innenabstand der Karten.
const PAD: f32 = sp::L;
/// Kopf einer Karte (Symbolchip + Titel).
const HEAD_H: f32 = 34.0;
/// Großer Eintrag „Neu erstellen“ (Symbol, Titel, Erklärung).
const TILE_H: f32 = 54.0;
/// Zeile „Zuletzt geöffnet“ (Vorschau, Name, Ordner).
const RECENT_H: f32 = 44.0;
/// Hilfe-Link.
const LINK_H: f32 = 26.0;
const FOOTER_H: f32 = 28.0;
const RADIUS: u8 = 8;

/// Hat der Nutzer „Nicht mehr anzeigen“ in dieser Sitzung angekreuzt? Dann bleibt der Bildschirm
/// bis zum Neustart sichtbar.
fn ticked_id() -> Id {
    Id::new("fp-welcome-ticked-this-session")
}

/// Wird der Startbildschirm gezeigt?
pub fn is_visible(ctx: &egui::Context, st: &AppState) -> bool {
    st.session.prefs.get_bool(PREF_SHOW, true)
        || ctx
            .data(|d| d.get_temp::<bool>(ticked_id()))
            .unwrap_or(false)
}

/// Werden noch Vorschaubilder der zuletzt geöffneten Dateien gelesen? (Tests, Bildserien.)
pub fn thumbnails_pending(ctx: &egui::Context) -> bool {
    thumbs::pending(ctx)
}

/// Kleines Vorschaubild einer Datei (gleicher Hintergrund-Leser wie „Zuletzt geöffnet“; auch für die
/// Vorschau im Datei-Dialog in der App, BA-060). `None`, solange gelesen wird bzw. bei anderen Formaten.
pub fn file_thumbnail(ctx: &egui::Context, path: &Path) -> Option<egui::TextureHandle> {
    thumbs::get(ctx, path)
}

fn last_id() -> Id {
    Id::new("fp-welcome-last-triggered")
}

/// Zuletzt auf dem Startbildschirm ausgelöster Eintrag: Befehlsname bzw. `open:<pfad>` (Tests,
/// Brücke). Der Befehl selbst läuft wie jeder Menübefehl über die Aktionswarteschlange.
pub fn last_triggered(ctx: &egui::Context) -> Option<String> {
    ctx.data(|d| d.get_temp::<String>(last_id()))
}

fn mark(ctx: &egui::Context, what: String) {
    ctx.data_mut(|d| d.insert_temp(last_id(), what));
}

fn trigger(ui: &Ui, st: &mut AppState, cmd: &str, params: serde_json::Value) {
    st.run(cmd, params);
    mark(ui.ctx(), cmd.to_string());
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
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if name.ends_with(".fw.png") {
        return RowIcon::Kit(KitIcon::FwDocument);
    }
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "tif" | "tiff" | "avif" | "psd"
        | "ico" | "qoi" | "wbmp" | "tga" => RowIcon::Kit(KitIcon::Image),
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
        .map(|p| RecentItem {
            path: p.clone(),
            name: p
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| p.display().to_string()),
            icon: file_icon(p),
            exists: p.exists(),
        })
        .collect()
}

/// Einträge der Karte „Neu erstellen“: (Symbol, Textschlüssel, Befehl).
fn create_entries() -> [(KitIcon, &'static str, &'static str); 2] {
    [
        (KitIcon::NewDocument, "welcome-new-document", "file.new"),
        (
            KitIcon::Template,
            "welcome-new-template",
            "file.newFromTemplate",
        ),
    ]
}

fn extend_entries(st: &AppState) -> Vec<(KitIcon, &'static str, &'static str)> {
    // „Batch-Verarbeitung…“ öffnet das Batch-Studio (BATCH-9 P2, BA-065)
    [
        (
            KitIcon::Chevrons,
            "welcome-extend-batch",
            "file.batchProcess",
        ),
        (
            KitIcon::Commands,
            "welcome-extend-commands",
            "commands.manageSavedCommands",
        ),
        (
            KitIcon::Extend,
            "welcome-extend-extensions",
            "commands.manageExtensions",
        ),
    ]
    .into_iter()
    .filter(|(_, _, c)| st.session.registry().get(c).is_some())
    .collect()
}

/// Hilfe-Links: (Symbol, Textschlüssel, Befehl, Parameter).
fn help_links() -> [(KitIcon, &'static str, &'static str, serde_json::Value); 4] {
    [
        (
            KitIcon::Book,
            "welcome-help-guide",
            "help.firepixelHelp",
            json!({ "topic": "start" }),
        ),
        (
            KitIcon::Keyboard,
            "welcome-help-shortcuts",
            "edit.keyboardShortcuts",
            json!({}),
        ),
        (
            KitIcon::AiChip,
            "welcome-help-models",
            "ai.manageModels",
            json!({}),
        ),
        (
            KitIcon::Gear,
            "welcome-help-automation",
            "help.firepixelHelp",
            json!({ "topic": "automation" }),
        ),
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
        ThemeKind::Oled => Banner {
            top: c.frame_bg,
            bottom: c.frame_bg,
            text: c.text,
            weak: c.text_weak,
            pixels: c.group_accent.to_vec(),
        },
    }
}

/// Kleiner deterministischer Zufall (Pixelmuster ohne Flackern).
fn hash01(x: i32, y: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(374_761_393) ^ (y as u32).wrapping_mul(668_265_263);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    ((h ^ (h >> 16)) & 0xffff) as f32 / 65535.0
}

/// Kopfbanner (Startbildschirm, Über Firepixel Studio): Fläche aus dem Design, Pixelfeld, Logo, Anzeigename
/// ([`fp_i18n::DISPLAY_NAME`]), Leitsatz.
pub(crate) fn paint_banner(p: &egui::Painter, r: Rect, theme: &Theme, narrow: bool, tagline: &str) {
    let b = banner_colors(theme);
    crate::widgets::vgradient(p, r, b.top, b.bottom);
    // Struktur: Pixelfeld, das nach rechts dichter wird (eigenes Motiv „Funken aus Pixeln“)
    let cell = 8.0;
    let cols = (r.width() / cell).floor() as i32;
    let rows = (r.height() / cell).floor() as i32;
    let p2 = p.with_clip_rect(r);
    // Maße von Logo und Schrift vorab: das Pixelfeld bleibt rechts der Schrift
    let tiny = r.width() < 300.0;
    let logo = if tiny {
        36.0
    } else if narrow {
        48.0
    } else {
        64.0
    };
    let margin = if narrow { sp::L } else { sp::XL };
    let x = r.left() + margin + logo + if tiny { sp::S } else { sp::L };
    let max_w = (r.right() - margin - x).max(40.0);
    let name_size = if tiny {
        18.0
    } else if narrow {
        24.0
    } else {
        type_scale::DISPLAY
    };
    let text_end = x
        + ui_kit::text_w(p.ctx(), tagline, &theme.font(12.5))
            .max(ui_kit::text_w(p.ctx(), fp_i18n::DISPLAY_NAME, &theme.bold(name_size)))
            .min(max_w)
        + sp::L;
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
            let a = if theme.glow {
                40.0 + 110.0 * density * (1.0 - v)
            } else {
                6.0 + 26.0 * density * (1.0 - v)
            };
            let q = Rect::from_min_size(
                pos2(
                    r.left() + i as f32 * cell + 1.0,
                    r.top() + j as f32 * cell + 1.0,
                ),
                vec2(cell - 2.0, cell - 2.0),
            );
            p2.rect_filled(q, 0.0, with_alpha(b.pixels[k], a as u8));
        }
    }
    // Logo, Name, Leitsatz
    let lr = Rect::from_min_size(
        pos2(r.left() + margin, r.center().y - logo / 2.0),
        vec2(logo, logo),
    );
    icons::paint_app_logo(p, lr);
    ui_kit::paint_text_fit(
        p,
        Rect::from_min_max(
            pos2(x, r.center().y - 2.0 - name_size * 1.3),
            pos2(x + max_w, r.center().y - 2.0),
        ),
        fp_i18n::DISPLAY_NAME,
        theme.bold(name_size),
        b.text,
    );
    ui_kit::paint_text_fit(
        p,
        Rect::from_min_size(pos2(x, r.center().y + 2.0), vec2(max_w, 18.0)),
        tagline,
        theme.font(12.5),
        b.weak,
    );
    if theme.glow {
        p.hline(
            r.x_range(),
            r.bottom() - 0.5,
            Stroke::new(1.0, theme.colors.focus),
        );
    }
}

// ---------------------------------------------------------------------- Farben der Karten

/// Kartenfläche: hell im Normal-Design, im Dunkel-Design etwas heller als die Leinwand (sonst
/// verschwinden die Karten), im OLED-Design schwarz.
fn card_fill(theme: &Theme) -> Color32 {
    let c = &theme.colors;
    match theme.kind {
        ThemeKind::Normal => ui_kit::card_bg(theme),
        ThemeKind::Dark => mix(c.menu_bg, c.frame_bg, 0.45),
        ThemeKind::Oled => c.frame_bg,
    }
}

/// Leitfarbe je Karte: 0 = Neu erstellen, 1 = Zuletzt geöffnet, 2 = Hilfe. OLED farbenfroh aus
/// den Gruppenfarben, sonst ruhige Symbolfarben des Designs.
fn accent(theme: &Theme, card: usize) -> Color32 {
    let c = &theme.colors;
    if theme.glow {
        [c.group_accent[2], c.group_accent[0], c.group_accent[3]][card.min(2)]
    } else {
        [c.icon_green, c.focus, c.icon_yellow][card.min(2)]
    }
}

/// Getönte Fläche in der Leitfarbe (Symbolchip, Hover).
fn accent_tint(theme: &Theme, col: Color32, strength: f32) -> Color32 {
    let base = card_fill(theme);
    let t = match theme.kind {
        ThemeKind::Normal => 0.14,
        ThemeKind::Dark => 0.22,
        ThemeKind::Oled => 0.22,
    } * strength;
    mix(base, col, t)
}

// ---------------------------------------------------------------------- Maße

/// Maße des Startbildschirms für eine verfügbare Fläche.
#[derive(Clone, Copy, Debug)]
struct Layout {
    w: f32,
    two: bool,
    banner_h: f32,
    left_w: f32,
    right_w: f32,
    start_h: f32,
    help_h: f32,
    recent_h: f32,
    cols_h: f32,
    total_h: f32,
}

fn layout(ctx: &egui::Context, avail: egui::Vec2, n_recent: usize, n_extend: usize) -> Layout {
    let w = (avail.x - 2.0 * sp::L).clamp(160.0, PANEL_MAX_W).floor();
    let two = w >= TWO_COLUMNS_MIN_W;
    let narrow = w < 420.0;
    let banner_h = if narrow { 84.0 } else { 112.0 };
    let left_w = if two {
        (w * 0.4).clamp(250.0, 350.0).floor()
    } else {
        w
    };
    let right_w = if two { w - left_w - GAP } else { w };
    let start_h = PAD + HEAD_H + 2.0 * TILE_H + sp::S + PAD;
    let mut help_h = PAD
        + HEAD_H
        + help_links().len() as f32 * LINK_H
        + if n_extend > 0 {
            sp::M + 18.0 + sp::XS + n_extend as f32 * LINK_H
        } else {
            0.0
        }
        + PAD;
    let empty_h = ui_kit::empty_state_height(
        ctx,
        &t("welcome-recent-empty"),
        Some(&t("welcome-recent-empty-hint")),
        right_w - 2.0 * PAD,
    );
    let list_h = if n_recent == 0 {
        empty_h
    } else {
        n_recent as f32 * RECENT_H
    };
    let mut recent_h = PAD + HEAD_H + list_h + sp::S + 2.0 + sp::S + sp::ROW + PAD;
    let cols_h = if two {
        let left = start_h + GAP + help_h;
        let h = left.max(recent_h);
        help_h += h - left;
        recent_h = h;
        h
    } else {
        start_h + GAP + recent_h + GAP + help_h
    };
    let total_h = banner_h + GAP + cols_h + sp::S + FOOTER_H;
    Layout {
        w,
        two,
        banner_h,
        left_w,
        right_w,
        start_h,
        help_h,
        recent_h,
        cols_h,
        total_h,
    }
}

// ---------------------------------------------------------------------- Zeichnen

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
    // Fokus vor dem Zeichnen: egui verschiebt ihn bei Tab erst, während sich die Einträge anmelden
    let focus_before = ui.ctx().memory(|m| m.focused());
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    child.set_clip_rect(rect.intersect(ui.clip_rect()));
    egui::ScrollArea::both()
        .id_salt("fp-welcome-scroll")
        .auto_shrink([false, false])
        .show(&mut child, |ui| {
            let need = vec2(lay.w + 2.0 * sp::L, lay.total_h + 2.0 * sp::L);
            let (outer, _) = ui.allocate_exact_size(
                vec2(need.x.max(rect.width()), need.y.max(rect.height())),
                Sense::hover(),
            );
            let panel = Rect::from_min_size(
                pos2(
                    outer.left() + ((outer.width() - lay.w) / 2.0).max(sp::L).round(),
                    outer.top() + ((outer.height() - lay.total_h) / 2.0).max(sp::L).round(),
                ),
                vec2(lay.w, lay.total_h),
            );
            draw_panel(ui, st, &theme, &lay, panel, &recent, &extend, focus_before);
        });
}

/// Fokus innerhalb des Startbildschirms: Einträge in Lesereihenfolge (Tab, Pfeiltasten).
#[derive(Default)]
struct Nav {
    ids: Vec<Id>,
    rects: Vec<Rect>,
}

impl Nav {
    fn track(&mut self, r: &Response) {
        self.ids.push(r.id);
        self.rects.push(r.rect);
    }
}

/// Tastenwunsch auf dem Startbildschirm: Tab vorwärts/rückwärts in Lesereihenfolge, Pfeiltasten
/// räumlich zum nächsten Eintrag in der Richtung.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    Next,
    Prev,
    Dir(egui::Vec2),
}

/// Nächster Eintrag in Richtung `dir` (Einheitsvektor) vom Eintrag `cur` aus: nur Einträge, die
/// in dieser Richtung liegen, der nächste gewinnt (Querabstand zählt doppelt). Am Rand bleibt
/// der Fokus stehen; ohne Fokus im Startbildschirm der erste Eintrag.
fn spatial_focus(ids: &[Id], rects: &[Rect], cur: Option<Id>, dir: egui::Vec2) -> Option<Id> {
    let Some(i) = cur.and_then(|c| ids.iter().position(|x| *x == c)) else {
        return ids.first().copied();
    };
    let from = rects.get(i)?.center();
    // erst im 90°-Kegel der Richtung suchen (sonst gewönne ein kaum versetzter Eintrag darunter),
    // dann im ganzen Halbraum
    for cone in [true, false] {
        let mut best: Option<(f32, Id)> = None;
        for (k, (id, r)) in ids.iter().zip(rects).enumerate() {
            if k == i {
                continue;
            }
            let d = r.center() - from;
            let along = d.x * dir.x + d.y * dir.y;
            let across = (d.x * dir.y - d.y * dir.x).abs();
            if along <= 1.0 || (cone && across > along) {
                continue;
            }
            let score = along + 2.0 * across;
            if best.is_none_or(|(b, _)| score < b) {
                best = Some((score, *id));
            }
        }
        if let Some((_, id)) = best {
            return Some(id);
        }
    }
    Some(ids[i])
}

/// Nächster Eintrag in `ids` nach `cur` (ringförmig); ohne Fokus im Startbildschirm der erste bzw.
/// (rückwärts) der letzte.
fn step_focus(ids: &[Id], cur: Option<Id>, step: Step) -> Option<Id> {
    if ids.is_empty() {
        return None;
    }
    let n = ids.len();
    let i = match (cur.and_then(|c| ids.iter().position(|x| *x == c)), step) {
        (Some(i), Step::Next) => (i + 1) % n,
        (Some(i), Step::Prev) => (i + n - 1) % n,
        (None, Step::Prev) => n - 1,
        (None, _) | (Some(_), Step::Dir(_)) => 0,
    };
    Some(ids[i])
}

/// Tab und Pfeiltasten auf dem Startbildschirm (CT-3): Tab/Umschalt+Tab wandern nur durch die
/// Einträge (das Kürzel „Bedienfelder ausblenden“ greift hier nicht, siehe `input::handle`),
/// die Pfeiltasten ebenso. Andere Fenster (Dialog, Popup, Menü) haben Vorrang.
fn keyboard(ui: &mut Ui, st: &AppState, nav: &Nav, focus_before: Option<Id>) {
    let ids = &nav.ids[..];
    let ctx = ui.ctx().clone();
    if st.dialogs.any_open() || egui::Popup::is_any_open(&ctx) || ids.is_empty() {
        return;
    }
    let focus = focus_before;
    let inside = focus.is_some_and(|f| ids.contains(&f));
    // ein Textfeld außerhalb (z. B. Zoomfeld der Werkzeugleiste) behält Tab und Pfeile
    if !inside && focus.is_some() && ctx.text_edit_focused() {
        return;
    }
    let step = ctx.input_mut(|i| {
        use egui::{Key, Modifiers as M};
        if i.consume_key(M::NONE, Key::Tab) {
            return Some(Step::Next);
        }
        if i.consume_key(M::SHIFT, Key::Tab) {
            return Some(Step::Prev);
        }
        [
            (Key::ArrowDown, vec2(0.0, 1.0)),
            (Key::ArrowRight, vec2(1.0, 0.0)),
            (Key::ArrowUp, vec2(0.0, -1.0)),
            (Key::ArrowLeft, vec2(-1.0, 0.0)),
        ]
        .into_iter()
        .find(|(k, _)| i.consume_key(M::NONE, *k))
        .map(|(_, d)| Step::Dir(d))
    });
    // egui hat Tab zu Beginn des Bildes schon selbst ausgewertet (evtl. in die Bedienfelder):
    // der Startbildschirm legt das Ziel danach fest
    let cur = if inside { focus } else { None };
    let target = match step {
        Some(Step::Dir(d)) => spatial_focus(ids, &nav.rects, cur, d),
        Some(s) => step_focus(ids, cur, s),
        None => None,
    };
    if let Some(target) = target {
        // egui wertet Pfeiltasten am Bildende selbst räumlich aus (über alle Widgets): abschalten
        ctx.memory_mut(|m| {
            m.request_focus(target);
            m.move_focus(egui::FocusDirection::None);
        });
        ctx.request_repaint();
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_panel(
    ui: &mut Ui,
    st: &mut AppState,
    theme: &Theme,
    lay: &Layout,
    panel: Rect,
    recent: &[RecentItem],
    extend: &[(KitIcon, &'static str, &'static str)],
    focus_before: Option<Id>,
) {
    let c = &theme.colors;
    let p = ui.painter().clone();
    let mut nav = Nav::default();
    // ------------------------------------------------ Kopfzeile
    let banner = Rect::from_min_size(panel.min, vec2(panel.width(), lay.banner_h));
    paint_banner(
        &p.with_clip_rect(banner),
        banner,
        theme,
        lay.w < 420.0,
        &t("welcome-tagline"),
    );
    // runde Ecken: die Ecken außerhalb der Rundung mit der Leinwandfarbe überdecken
    p.rect_stroke(
        banner,
        CornerRadius::same(RADIUS),
        Stroke::new(4.0, c.canvas_bg),
        StrokeKind::Outside,
    );
    if theme.glow {
        p.rect_stroke(
            banner,
            CornerRadius::same(RADIUS),
            Stroke::new(1.0, with_alpha(c.focus, 200)),
            StrokeKind::Inside,
        );
    }
    // ------------------------------------------------ Karten
    let top = banner.bottom() + GAP;
    let (start_r, recent_r, help_r) = if lay.two {
        let s = Rect::from_min_size(pos2(panel.left(), top), vec2(lay.left_w, lay.start_h));
        let h = Rect::from_min_size(
            pos2(panel.left(), s.bottom() + GAP),
            vec2(lay.left_w, lay.help_h),
        );
        let r = Rect::from_min_size(
            pos2(panel.left() + lay.left_w + GAP, top),
            vec2(lay.right_w, lay.recent_h),
        );
        (s, r, h)
    } else {
        let s = Rect::from_min_size(pos2(panel.left(), top), vec2(lay.w, lay.start_h));
        let r = Rect::from_min_size(
            pos2(panel.left(), s.bottom() + GAP),
            vec2(lay.w, lay.recent_h),
        );
        let h = Rect::from_min_size(
            pos2(panel.left(), r.bottom() + GAP),
            vec2(lay.w, lay.help_h),
        );
        (s, r, h)
    };
    // Neu erstellen
    let a0 = accent(theme, 0);
    let inner = card(
        &p,
        start_r,
        theme,
        a0,
        RowIcon::Kit(KitIcon::NewDocument),
        &t("welcome-new-title"),
    );
    let mut y = inner.top();
    for (icon, key, cmd) in create_entries() {
        let r = Rect::from_min_size(pos2(inner.left(), y), vec2(inner.width(), TILE_H));
        let resp = action_tile(
            ui,
            r,
            Id::new(("fp-welcome-new", key)),
            icon,
            &t(key),
            &t(&format!("{key}.tooltip")),
            a0,
            theme,
        );
        nav.track(&resp);
        if resp.clicked() {
            trigger(ui, st, cmd, json!({}));
        }
        y += TILE_H + sp::S;
    }
    // Zuletzt geöffnet
    let a1 = accent(theme, 1);
    let inner = card(
        &p,
        recent_r,
        theme,
        a1,
        RowIcon::Icon(Icon::Folder),
        &t("welcome-recent-title"),
    );
    let mut y = inner.top();
    if recent.is_empty() {
        let h = ui_kit::empty_state_height(
            ui.ctx(),
            &t("welcome-recent-empty"),
            Some(&t("welcome-recent-empty-hint")),
            inner.width(),
        );
        ui_kit::empty_state_at(
            &p,
            Rect::from_min_size(pos2(inner.left(), y), vec2(inner.width(), h)),
            KitIcon::EmptyTray,
            &t("welcome-recent-empty"),
            Some(&t("welcome-recent-empty-hint")),
            theme,
        );
        y += h;
    }
    for (i, it) in recent.iter().enumerate() {
        let r = Rect::from_min_size(pos2(inner.left(), y), vec2(inner.width(), RECENT_H));
        let thumb = if it.exists {
            thumbs::get(ui.ctx(), &it.path)
        } else {
            None
        };
        let resp = recent_row(
            ui,
            r,
            Id::new(("fp-welcome-recent", i)),
            it,
            thumb.as_ref(),
            theme,
        );
        nav.track(&resp);
        if resp.clicked() {
            st.push(Action::OpenPath(it.path.clone()));
            mark(ui.ctx(), format!("open:{}", it.path.display()));
        }
        y += RECENT_H;
    }
    // „Öffnen...“ unten an der Karte (die Karte ist so hoch wie die linke Spalte)
    y = (y + sp::S).max(inner.bottom() - sp::ROW - sp::S - 2.0);
    ui_kit::hline(&p, egui::Rangef::new(inner.left(), inner.right()), y, theme);
    y += 2.0 + sp::S;
    let r = Rect::from_min_size(pos2(inner.left(), y), vec2(inner.width(), sp::ROW));
    let resp = ui_kit::link_row_at(
        ui,
        r,
        Id::new("fp-welcome-open"),
        RowIcon::Icon(Icon::Open),
        &t("welcome-open"),
        Some(&t("welcome-open.tooltip")),
        true,
    );
    focus_ring(&p, &resp, r, theme);
    nav.track(&resp);
    if resp.clicked() {
        trigger(ui, st, "file.open", json!({}));
    }
    // Hilfe und Erweitern
    let a2 = accent(theme, 2);
    let inner = card(
        &p,
        help_r,
        theme,
        a2,
        RowIcon::Kit(KitIcon::Book),
        &t("welcome-help-title"),
    );
    let mut y = inner.top();
    for (k, (icon, key, cmd, params)) in help_links().into_iter().enumerate() {
        let r = Rect::from_min_size(pos2(inner.left(), y), vec2(inner.width(), LINK_H));
        paint_small_icon(
            &p,
            Rect::from_center_size(
                pos2(r.left() + sp::XS + 8.0, r.center().y),
                vec2(16.0, 16.0),
            ),
            icon,
            theme,
        );
        let lr = Rect::from_min_max(pos2(r.left() + sp::XS + 16.0 + sp::S, r.top()), r.max);
        let resp = ui_kit::more_link_at(
            ui,
            lr,
            Id::new(("fp-welcome-help", k)),
            &t(key),
            Some(&t(&format!("{key}.tooltip"))),
        );
        focus_ring(&p, &resp, r, theme);
        nav.track(&resp);
        if resp.clicked() {
            trigger(ui, st, cmd, params);
        }
        y += LINK_H;
    }
    if !extend.is_empty() {
        y += sp::M;
        ui_kit::section_at(
            &p,
            Rect::from_min_size(pos2(inner.left(), y), vec2(inner.width(), 18.0)),
            &t("welcome-extend-title"),
            theme,
        );
        y += 18.0 + sp::XS;
        for (icon, key, cmd) in extend {
            let r = Rect::from_min_size(
                pos2(inner.left(), y + 1.0),
                vec2(inner.width(), LINK_H - 2.0),
            );
            let resp = ui_kit::link_row_at(
                ui,
                r,
                Id::new(("fp-welcome-extend", *key)),
                RowIcon::Kit(*icon),
                &t(key),
                Some(&t(&format!("{key}.tooltip"))),
                true,
            );
            focus_ring(&p, &resp, r, theme);
            nav.track(&resp);
            if resp.clicked() {
                trigger(ui, st, cmd, json!({}));
            }
            y += LINK_H;
        }
    }
    // ------------------------------------------------ Fußzeile: Nicht mehr anzeigen, Version
    let foot = Rect::from_min_size(
        pos2(
            panel.left(),
            panel.top() + lay.banner_h + GAP + lay.cols_h + sp::S,
        ),
        vec2(panel.width(), FOOTER_H),
    );
    let ver = fp_i18n::tr!("welcome-version", version = env!("CARGO_PKG_VERSION"));
    let ver_font = theme.font(type_scale::CAPTION);
    let ver_w = ui_kit::text_w(ui.ctx(), &ver, &ver_font);
    let ver_color = if theme.kind == ThemeKind::Normal {
        mix(c.text_weak, c.text, 0.4)
    } else {
        c.text_weak
    };
    let show_ver = foot.width() > ver_w + 200.0;
    if show_ver {
        p.text(
            pos2(foot.right() - sp::XS, foot.center().y),
            Align2::RIGHT_CENTER,
            ver,
            ver_font,
            ver_color,
        );
    }
    let mut dont = !st.session.prefs.get_bool(PREF_SHOW, true);
    let cb_r = Rect::from_min_max(
        pos2(foot.left() + sp::XS, foot.top()),
        pos2(
            if show_ver {
                foot.right() - ver_w - sp::L
            } else {
                foot.right()
            },
            foot.bottom(),
        ),
    );
    let mut cu = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(cb_r)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    crate::theme::field_widgets(&mut cu);
    let resp = cu
        .add(egui::Checkbox::new(&mut dont, t("welcome-dont-show")))
        .on_hover_text(t("welcome-dont-show.tooltip"));
    nav.track(&resp);
    if resp.changed() {
        st.session
            .prefs
            .set(PREF_SHOW, fp_engine::PrefValue::Bool(!dont));
        st.session.prefs_changed();
        ui.ctx().data_mut(|d| d.insert_temp(ticked_id(), true));
    }
    // ------------------------------------------------ Tab und Pfeiltasten bewegen den Fokus
    keyboard(ui, st, &nav, focus_before);
}

/// Karte mit Schatten, Rahmen und Kopf (Symbolchip in der Leitfarbe, Titel). Liefert die
/// Innenfläche unter dem Kopf.
fn card(
    p: &egui::Painter,
    r: Rect,
    theme: &Theme,
    col: Color32,
    icon: RowIcon,
    title: &str,
) -> Rect {
    let c = &theme.colors;
    let cr = CornerRadius::same(RADIUS);
    if !theme.glow {
        let shadow = egui::Shadow {
            offset: [0, 2],
            blur: 10,
            spread: 0,
            color: Color32::from_black_alpha(if theme.kind == ThemeKind::Normal {
                34
            } else {
                110
            }),
        };
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
        // OLED: leuchtender Streifen oben in der Leitfarbe
        let strip = Rect::from_min_size(
            r.min + vec2(RADIUS as f32, 0.0),
            vec2(r.width() - 2.0 * RADIUS as f32, 2.0),
        );
        p.rect_filled(strip, 1.0, col);
    }
    // Kopf
    let chip = Rect::from_min_size(pos2(r.left() + PAD, r.top() + PAD - 2.0), vec2(24.0, 24.0));
    p.rect_filled(
        chip,
        CornerRadius::same(6),
        if theme.glow {
            with_alpha(col, 46)
        } else {
            accent_tint(theme, col, 1.4)
        },
    );
    let ir = Rect::from_center_size(chip.center(), vec2(16.0, 16.0));
    let st = IconStyle::new(theme, None, true);
    match icon {
        RowIcon::Icon(i) => icons::paint(p, ir, i, &st),
        RowIcon::Kit(k) => ui_kit::paint_icon(p, ir, k, &st),
        RowIcon::None => {}
    }
    let tr = Rect::from_min_max(
        pos2(chip.right() + sp::S + 2.0, chip.top()),
        pos2(r.right() - PAD, chip.bottom()),
    );
    let title_color = if theme.glow { col } else { c.text };
    ui_kit::paint_text_fit(
        p,
        tr,
        title,
        theme.bold(type_scale::SECTION + 1.0),
        title_color,
    );
    Rect::from_min_max(
        pos2(r.left() + PAD, r.top() + PAD + HEAD_H),
        pos2(r.right() - PAD, r.bottom() - PAD),
    )
}

/// Fokusrahmen (Tastaturbedienung) um ein Element.
fn focus_ring(p: &egui::Painter, resp: &Response, r: Rect, theme: &Theme) {
    if resp.has_focus() {
        p.rect_stroke(
            r,
            CornerRadius::same(5),
            Stroke::new(1.5, theme.colors.focus),
            StrokeKind::Inside,
        );
    }
}

/// Großer Eintrag: Symbolchip, Titel fett, Erklärsatz gedämpft. Hover getönt, Fokusrahmen.
#[allow(clippy::too_many_arguments)]
fn action_tile(
    ui: &mut Ui,
    rect: Rect,
    id: Id,
    icon: KitIcon,
    title: &str,
    subtitle: &str,
    col: Color32,
    theme: &Theme,
) -> Response {
    let c = &theme.colors;
    let resp = ui.interact(rect, id, Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, title));
    let hot = resp.hovered() || resp.has_focus();
    let down = resp.is_pointer_button_down_on();
    let p = ui.painter();
    let cr = CornerRadius::same(6);
    let (bg, border) = match (hot, theme.kind) {
        (true, ThemeKind::Oled) => (
            with_alpha(col, if down { 60 } else { 34 }),
            with_alpha(col, 220),
        ),
        (true, _) => (
            accent_tint(theme, col, if down { 1.6 } else { 0.9 }),
            mix(card_fill(theme), col, 0.55),
        ),
        (false, ThemeKind::Normal) => (c.field_bg, mix(c.line_mid, c.canvas_bg, 0.6)),
        (false, ThemeKind::Dark) => (mix(card_fill(theme), c.field_bg, 0.35), c.line_dark),
        (false, ThemeKind::Oled) => (c.frame_bg, with_alpha(col, 70)),
    };
    p.rect_filled(rect, cr, bg);
    p.rect_stroke(rect, cr, Stroke::new(1.0, border), StrokeKind::Inside);
    if resp.has_focus() {
        p.rect_stroke(
            rect.shrink(1.0),
            CornerRadius::same(5),
            Stroke::new(1.5, c.focus),
            StrokeKind::Inside,
        );
    }
    let chip = Rect::from_min_size(
        pos2(rect.left() + sp::S + 2.0, rect.center().y - 17.0),
        vec2(34.0, 34.0),
    );
    p.rect_filled(
        chip,
        CornerRadius::same(7),
        if theme.glow {
            with_alpha(col, 40)
        } else {
            accent_tint(theme, col, 1.2)
        },
    );
    ui_kit::paint_icon(
        p,
        Rect::from_center_size(chip.center(), vec2(22.0, 22.0)),
        icon,
        &IconStyle::new(theme, None, true),
    );
    let x = chip.right() + sp::M;
    let tw = (rect.right() - sp::S - x).max(20.0);
    let title_color = if hot && theme.glow { col } else { c.text };
    ui_kit::paint_text_fit(
        p,
        Rect::from_min_size(pos2(x, rect.center().y - 17.0), vec2(tw, 17.0)),
        title,
        theme.bold(type_scale::SECTION),
        title_color,
    );
    let shown = ui_kit::elide(ui.ctx(), subtitle, &theme.font(type_scale::CAPTION), tw);
    let sub_color = if theme.kind == ThemeKind::Normal {
        mix(c.text_weak, c.text, 0.2)
    } else {
        c.text_weak
    };
    p.text(
        pos2(x, rect.center().y + 9.0),
        Align2::LEFT_CENTER,
        &shown,
        theme.font(type_scale::CAPTION),
        sub_color,
    );
    let resp = if resp.hovered() {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        resp
    };
    if shown != subtitle {
        resp.on_hover_text(subtitle)
    } else {
        resp
    }
}

/// Zeile „Zuletzt geöffnet“: Vorschau (oder Symbol der Dateiart), Name, Ordner gedämpft.
fn recent_row(
    ui: &mut Ui,
    rect: Rect,
    id: Id,
    it: &RecentItem,
    thumb: Option<&egui::TextureHandle>,
    theme: &Theme,
) -> Response {
    let c = &theme.colors;
    let resp = ui.interact(rect, id, Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Link, true, &it.name));
    let hot = resp.hovered() || resp.has_focus();
    let p = ui.painter();
    let cr = CornerRadius::same(5);
    if hot {
        if theme.glow {
            p.rect_filled(rect, cr, with_alpha(c.focus, 30));
            p.rect_stroke(
                rect,
                cr,
                Stroke::new(1.0, with_alpha(c.focus, 160)),
                StrokeKind::Inside,
            );
        } else {
            p.rect_filled(rect, cr, c.list_hover_bg);
        }
    }
    if resp.has_focus() {
        p.rect_stroke(rect, cr, Stroke::new(1.5, c.focus), StrokeKind::Inside);
    }
    // Vorschau bzw. Symbol
    let bx = Rect::from_min_size(
        pos2(rect.left() + sp::XS + 2.0, rect.center().y - 18.0),
        vec2(36.0, 36.0),
    );
    let box_bg = if theme.kind == ThemeKind::Normal {
        c.field_bg
    } else {
        mix(card_fill(theme), c.field_bg, 0.6)
    };
    p.rect_filled(bx, CornerRadius::same(4), box_bg);
    match thumb {
        Some(tex) => {
            let sz = tex.size_vec2();
            let s = (34.0 / sz.x.max(1.0)).min(34.0 / sz.y.max(1.0));
            let ir = Rect::from_center_size(bx.center(), sz * s);
            if tex_has_room(ir) {
                crate::widgets::checker(p, ir, 4.0, theme);
            }
            p.image(
                tex.id(),
                ir,
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        }
        None => {
            let ir = Rect::from_center_size(bx.center(), vec2(20.0, 20.0));
            let st = IconStyle::new(theme, None, it.exists);
            match it.icon {
                RowIcon::Icon(i) => icons::paint(p, ir, i, &st),
                RowIcon::Kit(k) => ui_kit::paint_icon(p, ir, k, &st),
                RowIcon::None => {}
            }
        }
    }
    let border = if theme.glow {
        with_alpha(c.focus, 90)
    } else {
        mix(c.line_mid, box_bg, 0.55)
    };
    p.rect_stroke(
        bx,
        CornerRadius::same(4),
        Stroke::new(1.0, border),
        StrokeKind::Inside,
    );
    // Name und Ordner
    let x = bx.right() + sp::M - 2.0;
    let tw = (rect.right() - sp::S - x).max(20.0);
    let name_font = theme.font(type_scale::BODY + 0.5);
    let name_color = if !it.exists {
        c.text_disabled
    } else if hot {
        if theme.glow {
            c.focus
        } else {
            c.menu_hover_text
        }
    } else {
        c.text
    };
    let name = ui_kit::elide(ui.ctx(), &it.name, &name_font, tw);
    p.text(
        pos2(x, rect.center().y - 8.0),
        Align2::LEFT_CENTER,
        &name,
        name_font,
        name_color,
    );
    let folder = it
        .path
        .parent()
        .map(|d| d.display().to_string())
        .unwrap_or_default();
    if !folder.is_empty() {
        let f = theme.font(type_scale::CAPTION);
        let shown = ui_kit::elide_middle(ui.ctx(), &folder, &f, tw);
        let weak = if !it.exists {
            c.text_disabled
        } else if theme.kind == ThemeKind::Normal {
            mix(c.text_weak, c.text, 0.15)
        } else {
            c.text_weak
        };
        p.text(
            pos2(x, rect.center().y + 9.0),
            Align2::LEFT_CENTER,
            shown,
            f,
            weak,
        );
    }
    let resp = if resp.hovered() {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        resp
    };
    let tip = if it.exists {
        it.path.display().to_string()
    } else {
        format!("{}\n{}", t("welcome-recent-missing"), it.path.display())
    };
    resp.on_hover_text(tip)
}

/// Schachbrett nur hinter Vorschauen, die groß genug sind, dass es nicht flimmert.
fn tex_has_room(r: Rect) -> bool {
    r.width() >= 8.0 && r.height() >= 8.0
}

fn paint_small_icon(p: &egui::Painter, r: Rect, icon: KitIcon, theme: &Theme) {
    ui_kit::paint_icon(p, r, icon, &IconStyle::new(theme, None, true));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_icons_by_type() {
        assert_eq!(
            file_icon(Path::new(r"C:\a\b.fw.png")),
            RowIcon::Kit(KitIcon::FwDocument)
        );
        assert_eq!(
            file_icon(Path::new(r"C:\a\B.FW.PNG")),
            RowIcon::Kit(KitIcon::FwDocument)
        );
        assert_eq!(
            file_icon(Path::new(r"C:\a\b.jpg")),
            RowIcon::Kit(KitIcon::Image)
        );
        assert_eq!(
            file_icon(Path::new(r"C:\a\b.png")),
            RowIcon::Kit(KitIcon::Image)
        );
        assert_eq!(
            file_icon(Path::new(r"C:\a\b.jsf")),
            RowIcon::Kit(KitIcon::Script)
        );
        assert_eq!(
            file_icon(Path::new(r"C:\a\b.xyz")),
            RowIcon::Icon(Icon::Document)
        );
    }

    #[test]
    fn focus_steps_in_reading_order_and_wraps() {
        let ids: Vec<Id> = (0..4).map(|i| Id::new(("t", i))).collect();
        assert_eq!(step_focus(&ids, None, Step::Next), Some(ids[0]));
        assert_eq!(step_focus(&ids, None, Step::Prev), Some(ids[3]));
        assert_eq!(step_focus(&ids, Some(ids[1]), Step::Next), Some(ids[2]));
        assert_eq!(step_focus(&ids, Some(ids[3]), Step::Next), Some(ids[0]));
        assert_eq!(step_focus(&ids, Some(ids[0]), Step::Prev), Some(ids[3]));
        // Fokus außerhalb (Bedienfeld) zählt wie kein Fokus
        assert_eq!(
            step_focus(&ids, Some(Id::new("anders")), Step::Next),
            Some(ids[0])
        );
        assert_eq!(step_focus(&[], None, Step::Next), None);
    }

    #[test]
    fn arrows_pick_the_nearest_entry_in_that_direction() {
        // zwei Spalten: links a (oben), b (darunter); rechts c (auf Höhe von a), d (auf Höhe von b)
        let r = |x: f32, y: f32| Rect::from_min_size(pos2(x, y), vec2(100.0, 40.0));
        let ids: Vec<Id> = ["a", "b", "c", "d"].iter().map(|n| Id::new(*n)).collect();
        let rects = [r(0.0, 0.0), r(0.0, 60.0), r(300.0, 0.0), r(300.0, 60.0)];
        let go = |cur: usize, d: egui::Vec2| spatial_focus(&ids, &rects, Some(ids[cur]), d);
        assert_eq!(go(0, vec2(0.0, 1.0)), Some(ids[1]), "ab");
        assert_eq!(
            go(1, vec2(1.0, 0.0)),
            Some(ids[3]),
            "rechts auf gleicher Höhe"
        );
        assert_eq!(go(3, vec2(0.0, -1.0)), Some(ids[2]), "auf");
        assert_eq!(go(2, vec2(-1.0, 0.0)), Some(ids[0]), "links");
        // am Rand bleibt der Fokus stehen
        assert_eq!(go(0, vec2(0.0, -1.0)), Some(ids[0]));
        assert_eq!(go(0, vec2(-1.0, 0.0)), Some(ids[0]));
        // ein kaum nach rechts versetzter Eintrag weit darunter verliert gegen den Eintrag rechts
        let ids2: Vec<Id> = ["t", "h", "r"].iter().map(|n| Id::new(*n)).collect();
        let rects2 = [r(0.0, 0.0), r(14.0, 120.0), r(480.0, 20.0)];
        assert_eq!(
            spatial_focus(&ids2, &rects2, Some(ids2[0]), vec2(1.0, 0.0)),
            Some(ids2[2])
        );
        // ohne Fokus: erster Eintrag
        assert_eq!(
            spatial_focus(&ids, &rects, None, vec2(0.0, 1.0)),
            Some(ids[0])
        );
    }

    #[test]
    fn layout_fits_and_switches_columns() {
        crate::ui_kit::tests::with_ctx(|ctx| {
            let wide = layout(ctx, vec2(1200.0, 800.0), 8, 2);
            assert!(wide.two && wide.w <= PANEL_MAX_W);
            assert!((wide.left_w + GAP + wide.right_w - wide.w).abs() < 0.5);
            // Spalten gleich hoch
            assert!((wide.start_h + GAP + wide.help_h - wide.recent_h).abs() < 0.5);
            let narrow = layout(ctx, vec2(400.0, 500.0), 0, 2);
            assert!(!narrow.two && narrow.w <= 400.0 - 2.0 * sp::L + 0.5);
            assert!(narrow.total_h > wide.total_h - 200.0);
        });
    }
}
