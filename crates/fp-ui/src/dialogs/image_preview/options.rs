//! Linke Spalte der Bildvorschau: Registerkarten Optionen / Datei / Animation. Sie zeigt und ändert
//! immer die Einstellung des **aktiven** Feldes (re\17-bildvorschau.md §5.1).
//!
//! Registerkarte **Optionen** vollständig nach §2 (Paket P2): Formatliste mit Trennstrichen in der
//! Reihenfolge des Originals (§2.2, Firepixel-Formate hinter einem weiteren Trennstrich), Knopf ≡
//! (Palettenmenü, §2.7), Palette (§2.4), Qualität/Glättung/Progressiv/Farbränder (JPEG), Verlust,
//! Farbanzahl mit tatsächlicher Anzahl, Rastern mit Menge, Farbtabelle mit sechs Schaltern,
//! Statuszeile, drei Pipetten und Transparenzliste (§2.5/§2.6), Hintergrund (Matte), „Nicht
//! verwendete Farben entfernen“, Zeilensprung und der Knopf „Optimieren auf Größe“ (§2.8).
//! Sichtbar bzw. grau je Format genau nach der Tabelle §2.3 ([`visibility`]).
//! Datei und Animation: Stand P1 (Paket P3 baut hier weiter).

use super::color_table::{self, FollowUp, MenuState, PaletteCmd, Pipette};
use super::super::{combo_fit_w, extra_w, fit_label, refs, report_overflow, text_w, tl};
use super::{ImagePreview, Tool};
use crate::app_state::AppState;
use crate::inspector::fields;
use crate::panels::optimize::{self, PALETTES};
use crate::theme::Theme;
use egui::{pos2, vec2, Align2, Id, Rect, Sense, Stroke, StrokeKind, Ui};
use fp_export::options::{ColorMode, Dither, ExportFormat, OptimizeSettings, PaletteColor, PaletteKind, Transparency};
use fp_export::palette_edit::PaletteEntry;
use fp_i18n::t;

const TAB_KEYS: [&str; 3] = ["dlg-preview-tab-options", "dlg-preview-tab-file", "dlg-preview-tab-animation"];
const LABEL_KEYS: [&str; 7] = ["image-preview-opt-format", "image-preview-opt-palette", "image-preview-opt-quality", "image-preview-opt-loss", "image-preview-opt-dither", "image-preview-opt-smoothing", "dlg-preview-scale"];
const CHECK_KEYS: [&str; 4] = ["image-preview-opt-progressive", "image-preview-opt-sharpen", "image-preview-opt-remove-unused", "image-preview-opt-interlaced"];
const TRANSPARENCY: [&str; 3] = ["panel-opt-transp-none", "panel-opt-transp-index", "panel-opt-transp-alpha"];
const COLOR_COUNTS: [&str; 8] = ["2", "4", "8", "16", "32", "64", "128", "256"];
const SEP: &str = "------------";

/// Formatliste der Bildvorschau (§2.2): Anzeigename (Formatname oder Schlüssel), Format, Farbtiefe;
/// `None` = Trennstrich. Hinter dem letzten Trennstrich die Firepixel-Formate (§10 P2.1).
pub const FORMATS: &[(&str, Option<(ExportFormat, ColorMode)>)] = &[
    ("GIF", Some((ExportFormat::Gif, ColorMode::Indexed))),
    ("panel-opt-format-anim-gif", Some((ExportFormat::AnimatedGif, ColorMode::Indexed))),
    ("JPEG", Some((ExportFormat::Jpeg, ColorMode::Rgb24))),
    (SEP, None),
    ("PNG 8", Some((ExportFormat::Png, ColorMode::Indexed))),
    ("PNG 24", Some((ExportFormat::Png, ColorMode::Rgb24))),
    ("PNG 32", Some((ExportFormat::Png, ColorMode::Rgba32))),
    (SEP, None),
    ("WBMP", Some((ExportFormat::Wbmp, ColorMode::Indexed))),
    (SEP, None),
    ("TIFF 8", Some((ExportFormat::Tiff, ColorMode::Indexed))),
    ("TIFF 24", Some((ExportFormat::Tiff, ColorMode::Rgb24))),
    ("TIFF 32", Some((ExportFormat::Tiff, ColorMode::Rgba32))),
    (SEP, None),
    ("BMP 8", Some((ExportFormat::Bmp, ColorMode::Indexed))),
    ("BMP 24", Some((ExportFormat::Bmp, ColorMode::Rgb24))),
    (SEP, None),
    ("WebP", Some((ExportFormat::WebP, ColorMode::Rgba32))),
    ("panel-opt-format-webp-lossless", Some((ExportFormat::WebPLossless, ColorMode::Rgba32))),
    ("panel-opt-format-anim-webp", Some((ExportFormat::AnimatedWebP, ColorMode::Rgba32))),
    ("AVIF", Some((ExportFormat::Avif, ColorMode::Rgba32))),
    ("APNG", Some((ExportFormat::Apng, ColorMode::Rgba32))),
    ("QOI", Some((ExportFormat::Qoi, ColorMode::Rgba32))),
    ("ICO", Some((ExportFormat::Ico, ColorMode::Rgba32))),
];

/// Angezeigter Name eines Eintrags von [`FORMATS`].
pub fn format_name(i: usize) -> String {
    FORMATS.get(i).map(|(n, _)| if n.starts_with("panel-opt-") { t(n) } else { n.to_string() }).unwrap_or_default()
}

/// Formate mit nur einer Farbtiefe (die Liste unterscheidet sie nicht).
fn single_depth(f: ExportFormat) -> bool {
    !matches!(f, ExportFormat::Png | ExportFormat::Tiff | ExportFormat::Bmp)
}

/// Eintrag der Formatliste für eine Einstellung.
pub fn format_index(s: &OptimizeSettings) -> Option<usize> {
    FORMATS.iter().position(|(_, f)| f.is_some_and(|(ff, m)| ff == s.format && (m == s.color_mode || single_depth(ff))))
}

/// Angezeigter Name eines Eintrags der Palettenliste (§2.4; Eintrag 1 heißt „WebSnap-Adaptiv“).
pub fn palette_name(i: usize) -> String {
    if i == 1 { t("image-preview-opt-palette-websnap") } else { optimize::palette_name(i) }
}

/// Format wählen (§2.3): WBMP setzt Schwarzweiß mit 2 Farben (bleibt danach für TIFF 8/BMP 8
/// stehen), der Wechsel zu „Animiertes GIF“ verringert 256 Farben auf 255; alle übrigen Werte
/// bleiben.
pub fn apply_format(s: &mut OptimizeSettings, f: ExportFormat, m: ColorMode) {
    let was_anim = s.format == ExportFormat::AnimatedGif;
    s.format = f;
    s.color_mode = m;
    match f {
        ExportFormat::Wbmp => {
            s.palette_kind = PaletteKind::Grayscale;
            s.web_adaptive = false;
            s.max_colors = 2;
        }
        ExportFormat::AnimatedGif if !was_anim && s.max_colors >= 256 => s.max_colors = 255,
        ExportFormat::WebPLossless => s.webp.lossless = true,
        _ => {}
    }
}

/// Palette wählen (§2.4). „Graustufen“ nach „Schwarzweiß“ nimmt wieder 256 Stufen;
/// „Benutzerdefiniert“ ohne eigene Farben übernimmt die angezeigte Palette.
pub fn apply_palette(s: &mut OptimizeSettings, i: usize, current: &[PaletteEntry]) {
    let Some(&(_, k, web, cnt)) = PALETTES.get(i) else { return };
    s.palette_kind = k;
    s.web_adaptive = web;
    match cnt {
        Some(n) => s.max_colors = n,
        None if k == PaletteKind::Grayscale && s.max_colors <= 2 => s.max_colors = 256,
        None => {}
    }
    if k == PaletteKind::Custom && s.palette.len() < 2 && current.len() >= 2 {
        s.palette = current.iter().map(|e| PaletteColor { color: e.shown | 0xFF00_0000, locked: e.locked, transparent: e.transparent, edited: None }).collect();
        s.max_colors = (s.palette.len() as i32).clamp(2, 256);
    }
}

// ---------------------------------------------------------------------- Sichtbarkeit (§2.3)

/// Zustand eines Bedienelements.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Show {
    Hidden,
    /// sichtbar, aber gesperrt
    Disabled,
    Visible,
}

impl Show {
    pub fn shown(self) -> bool {
        self != Show::Hidden
    }
    pub fn enabled(self) -> bool {
        self == Show::Visible
    }
}

/// Welche Felder bei welchem Format erscheinen (Tabelle §2.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Visibility {
    pub options_menu: Show,
    pub palette: Show,
    pub quality: Show,
    pub loss: Show,
    /// Farbanzahl + tatsächliche Anzahl.
    pub colors: Show,
    pub dither: Show,
    pub smoothing: Show,
    /// „Progressive Browseranzeige“, „Farbränder scharfzeichnen“.
    pub jpeg_checks: Show,
    /// Farbtabelle + sechs Knöpfe.
    pub color_table: Show,
    /// Pipetten + Transparenzliste.
    pub transparency: Show,
    pub matte: Show,
    pub remove_unused: Show,
    pub interlaced: Show,
}

/// Sichtbarkeit je Format (§2.3). Firepixel-Formate: WebP/AVIF (verlustbehaftet) mit Qualität,
/// alle übrigen wie PNG 24/32 (nur Hintergrund).
pub fn visibility(s: &OptimizeSettings) -> Visibility {
    use Show::{Disabled as D, Hidden as H, Visible as V};
    let truecolor = Visibility { options_menu: H, palette: H, quality: H, loss: H, colors: H, dither: H, smoothing: H, jpeg_checks: H, color_table: H, transparency: H, matte: V, remove_unused: H, interlaced: H };
    let indexed = Visibility { options_menu: V, palette: V, colors: V, dither: V, color_table: V, remove_unused: V, ..truecolor };
    match s.format {
        ExportFormat::Gif | ExportFormat::AnimatedGif => Visibility { loss: V, transparency: V, interlaced: V, ..indexed },
        ExportFormat::Jpeg => Visibility { quality: V, smoothing: V, jpeg_checks: V, remove_unused: V, ..truecolor },
        ExportFormat::Png if s.color_mode == ColorMode::Indexed => Visibility { transparency: V, interlaced: V, ..indexed },
        ExportFormat::Wbmp => Visibility { palette: D, colors: D, ..indexed },
        ExportFormat::Tiff | ExportFormat::Bmp if s.color_mode == ColorMode::Indexed => indexed,
        ExportFormat::WebP | ExportFormat::Avif => Visibility { quality: V, ..truecolor },
        ExportFormat::AnimatedWebP if !s.webp.lossless => Visibility { quality: V, ..truecolor },
        _ => truecolor,
    }
}

// ---------------------------------------------------------------------- Maße

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

fn smoothing_items() -> Vec<String> {
    let mut v = vec![t("image-preview-opt-smoothing-none")];
    v.extend((1..=7).map(|i| i.to_string()));
    v.push(t("image-preview-opt-smoothing-max"));
    v
}

pub fn measure(ctx: &egui::Context) -> Measure {
    let tabs = tl(&TAB_KEYS);
    let labs = tl(&LABEL_KEYS);
    let fnames: Vec<String> = (0..FORMATS.len()).map(format_name).collect();
    let pnames: Vec<String> = (0..PALETTES.len()).map(palette_name).collect();
    let tab_w = super::super::buttons_w(ctx, &refs(&tabs), 80.0);
    let dx = extra_w(ctx, &refs(&labs), 70.0 - 2.0);
    let cw = combo_fit_w(ctx, &refs(&fnames), 160.0).max(combo_fit_w(ctx, &refs(&pnames), 135.0)).max(combo_fit_w(ctx, &refs(&smoothing_items()), 135.0));
    let checks = tl(&CHECK_KEYS).iter().map(|s| text_w(ctx, s)).fold(0.0, f32::max);
    let tw = combo_fit_w(ctx, &refs(&tl(&TRANSPARENCY)), 150.0);
    let matte_w = text_w(ctx, &t("image-preview-opt-matte"));
    let width = MIN_WIDTH
        .max(3.0 * tab_w + 6.0)
        .max(2.0 * PAD + 76.0 + dx + cw + 4.0 + 22.0)
        .max(2.0 * PAD + 18.0 + checks + 4.0)
        .max(2.0 * PAD + 96.0 + tw)
        .max(2.0 * PAD + matte_w + 40.0)
        .ceil();
    Measure { width, tab_w, dx, cw }
}

/// Beschriftung rechtsbündig (wie die Beschriftungen des Originals vor den Feldern).
fn label_r(ui: &Ui, r: Rect, text: &str, enabled: bool) {
    if text_w(ui.ctx(), text) > r.width() + 0.5 {
        report_overflow(ui.ctx(), format!("Beschriftung „{text}“ breiter als {:.0} px", r.width()));
    }
    fields::label_right(ui, r, text, enabled);
}

/// Tooltip über einem Bedienelement, das selbst keinen zeigt.
/// Der Tooltip ist zugleich der Barrierefreiheits-Name (für `ui_find` und Tests).
fn tip(ui: &Ui, id: &str, r: Rect, key: &str) {
    let text = t(key);
    let resp = ui.interact(r, Id::new(("fp-ipv-opt-tip", id)), Sense::hover());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, true, text.as_str()));
    resp.on_hover_text(text);
}

/// Zeichnet die linke Spalte in `left` für das aktive Feld von `d`.
pub fn show(ui: &mut Ui, st: &mut AppState, left: Rect, m: Measure, d: &mut ImagePreview) {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let tabs = tl(&TAB_KEYS);
    // Registerkarten Optionen / Datei / Animation
    for (i, label) in tabs.iter().enumerate() {
        let tr = Rect::from_min_size(left.min + vec2(i as f32 * (m.tab_w + 2.0), 0.0), vec2(m.tab_w, 20.0));
        let resp = ui.interact(tr, Id::new(("pv-tab", i)), Sense::click());
        resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, d.tab == i, label));
        ui.painter().rect_filled(tr, 0.0, if d.tab == i { c.tab_active_bg } else { c.tab_inactive_bg });
        ui.painter().rect_stroke(tr, 0.0, Stroke::new(1.0, c.line_mid), StrokeKind::Inside);
        ui.painter().text(tr.center(), Align2::CENTER_CENTER, label.as_str(), theme.ui_font(), c.text);
        if resp.clicked() {
            d.tab = i;
        }
    }
    let page = Rect::from_min_max(pos2(left.left(), left.top() + 20.0), left.max);
    ui.painter().rect_stroke(page, 0.0, Stroke::new(1.0, c.line_mid), StrokeKind::Inside);
    let inner = page.shrink2(vec2(PAD, 0.0));
    match d.tab {
        0 => options_tab(ui, st, page, inner, m, d),
        1 => {
            let s = &mut d.panes[d.active].settings;
            let (dx, lw) = (m.dx, 70.0 + m.dx);
            let cx = inner.left() + 76.0 + dx;
            let y0 = page.top() + 10.0;
            fit_label(ui, Rect::from_min_size(pos2(inner.left(), y0), vec2(lw, 20.0)), &t("dlg-preview-scale"), true);
            let percent = s.scale.filter(|sc| sc.use_scale).map(|sc| sc.percent as f64).unwrap_or(100.0);
            if let Some(v) = fields::number(ui, "pv-scale", Rect::from_min_size(pos2(cx, y0), vec2(50.0, 20.0)), Some(Rect::from_min_size(pos2(cx + 52.0, y0), vec2(14.0, 20.0))), Some(percent), (1.0, 1000.0), 0, true) {
                s.scale = Some(fp_model::ExportScale { use_scale: (v - 100.0).abs() > 0.01, constrain: true, percent: v.round() as i32, width: 0, height: 0 });
            }
            ui.painter().text(pos2(cx + 70.0, y0 + 10.0), Align2::LEFT_CENTER, "%", theme.ui_font(), c.text);
        }
        _ => {
            let y0 = page.top() + 10.0;
            let frames = st.session.active().map(|x| x.frame_count()).unwrap_or(1);
            ui.painter().text(pos2(inner.left(), y0), Align2::LEFT_TOP, fp_i18n::tr!("dlg-preview-states", n = frames), theme.ui_font(), c.text);
            let looping = crate::panels::states::loop_text(st.session.active().and_then(|x| x.doc.pages.get(x.page)).map(|p| p.loop_count()).unwrap_or(0));
            ui.painter().text(pos2(inner.left(), y0 + 20.0), Align2::LEFT_TOP, fp_i18n::tr!("dlg-preview-loop", looping = looping), theme.ui_font(), c.text);
        }
    }
    // „Assistent für Dateigrößenoptimierung“ sitzt auf allen Registerseiten an derselben Stelle (§2.1)
    let br = Rect::from_min_size(pos2(inner.right() - 23.0, page.bottom() - 28.0), vec2(23.0, 24.0));
    let size_tip = t("image-preview-tip-optimize-size");
    let th = theme.clone();
    if color_table::toggle_button(ui, "optimize-size", br, false, true, &size_tip, |p, r, en| paint_size_icon(&th, p, r, en)).clicked() {
        let bytes = d.panes[d.active].result.as_ref().map(|r| r.bytes).unwrap_or(0);
        d.size_dialog = Some(super::optimize_to_size::SizeDialog { pane: d.active, text: super::optimize_to_size::prefill(bytes) });
    }
}

/// Symbol „Optimieren auf Größe“: Blatt mit Pfeil nach unten.
fn paint_size_icon(theme: &Theme, p: &egui::Painter, r: Rect, enabled: bool) {
    let fg = if enabled { theme.colors.icon_fg } else { theme.colors.icon_disabled };
    let sheet = Rect::from_min_size(r.min + vec2(2.0, 1.0), vec2(9.0, 12.0));
    p.rect_filled(sheet, 0.0, theme.colors.icon_paper);
    p.rect_stroke(sheet, 0.0, Stroke::new(1.0, fg), StrokeKind::Inside);
    let x = r.right() - 3.5;
    p.vline(x, (r.top() + 4.0)..=(r.bottom() - 2.0), Stroke::new(1.3, fg));
    p.line(vec![pos2(x - 3.0, r.bottom() - 5.0), pos2(x, r.bottom() - 2.0), pos2(x + 3.0, r.bottom() - 5.0)], Stroke::new(1.3, fg));
}

/// Registerkarte „Optionen“ (§2.1). Lage nach dem Original, bezogen auf den oberen bzw. unteren
/// Rand der Registerseite; die Farbtabelle nimmt den Platz dazwischen.
fn options_tab(ui: &mut Ui, st: &mut AppState, page: Rect, inner: Rect, m: Measure, d: &mut ImagePreview) {
    let theme = Theme::current(ui.ctx());
    let c = theme.colors.clone();
    let active = d.active;
    let result = d.panes[active].result.clone();
    let entries: Vec<PaletteEntry> = result.as_ref().map(|r| r.entries.clone()).unwrap_or_default();
    let alpha_palette = entries.iter().any(|e| e.shown >> 24 != 0 && e.shown >> 24 != 255);
    {
        let s = &d.panes[active].settings;
        let owner = Some((active, s.format, s.color_mode));
        if d.table.owner != owner {
            d.table.owner = owner;
            d.table.selected.clear();
        }
        d.table.retain(&entries);
    }
    let vis = visibility(&d.panes[active].settings);
    let (dx, cw) = (m.dx, m.cw);
    let lw = 70.0 + dx;
    let cx = inner.left() + 76.0 + dx;
    let (top, bottom, right) = (page.top(), page.bottom(), inner.right());
    let row = |k: usize| top + 7.0 + 26.0 * k as f32;
    let lab = |y: f32| Rect::from_min_size(pos2(inner.left(), y), vec2(lw, 20.0));
    let mut cmd: Option<PaletteCmd> = None;
    let menu_state = |d: &ImagePreview| {
        let s = &d.panes[d.active].settings;
        let sel = d.table.selected_entries(&entries);
        MenuState {
            has_selection: !sel.is_empty(),
            any_locked: fp_export::palette_edit::any_locked(s),
            selection_edited: sel.iter().any(|(_, e)| e.edited),
            any_edited: fp_export::palette_edit::any_edited(s),
            sorting: s.sorting,
            feedback: d.table.feedback,
        }
    };

    // Format (4610) und ≡ (4624)
    let fnames: Vec<String> = (0..FORMATS.len()).map(format_name).collect();
    label_r(ui, lab(row(0)), &t("image-preview-opt-format"), true);
    let fr = Rect::from_min_size(pos2(cx, row(0)), vec2(cw, 20.0));
    let s = &mut d.panes[active].settings;
    if let Some(i) = fields::combo(ui, "pv-format", fr, &refs(&fnames), format_index(s), true)
        && let Some((f, mode)) = FORMATS[i].1
    {
        apply_format(s, f, mode);
    }
    tip(ui, "format", fr, "image-preview-tip-format");
    if vis.options_menu.shown() {
        let mr = Rect::from_min_size(pos2(right - 20.0, row(0)), vec2(20.0, 20.0));
        let th = theme.clone();
        let resp = color_table::toggle_button(ui, "palette-menu", mr, false, true, &t("image-preview-tip-options"), |p, r, en| color_table::paint_icon(&th, p, r, en, crate::icons::Icon::PanelMenu));
        if let Some(k) = color_table::popup_menu(ui, &resp, menu_state(d)) {
            cmd = Some(k);
        }
    }
    let s = &mut d.panes[active].settings;
    let vis = visibility(s);

    // Zeile 2: Palette (4616) bzw. Qualität (JPEG, WebP, AVIF)
    if vis.palette.shown() {
        let en = vis.palette.enabled();
        label_r(ui, lab(row(1)), &t("image-preview-opt-palette"), en);
        let pnames: Vec<String> = (0..PALETTES.len()).map(palette_name).collect();
        let pr = Rect::from_min_size(pos2(cx, row(1)), vec2(cw.min(right - cx), 20.0));
        let sel = if s.format == ExportFormat::Wbmp { 7 } else { optimize::palette_index(s) };
        if let Some(i) = fields::combo(ui, "pv-pal", pr, &refs(&pnames), Some(sel), en) {
            apply_palette(s, i, &entries);
        }
        tip(ui, "palette", pr, "image-preview-tip-palette");
    } else if vis.quality.shown() {
        label_r(ui, lab(row(1)), &t("image-preview-opt-quality"), true);
        let q = match s.format {
            ExportFormat::Jpeg => s.jpeg.quality,
            ExportFormat::Avif => s.avif.quality,
            _ => s.webp.quality,
        };
        let qr = Rect::from_min_size(pos2(cx, row(1)), vec2(36.0, 20.0));
        if let Some(v) = fields::number(ui, "pv-q", qr, Some(Rect::from_min_size(pos2(cx + 37.0, row(1)), vec2(14.0, 20.0))), Some(q as f64), (1.0, 100.0), 0, true) {
            match s.format {
                ExportFormat::Jpeg => s.jpeg.quality = v as i32,
                ExportFormat::Avif => s.avif.quality = v as i32,
                _ => s.webp.quality = v as i32,
            }
        }
        tip(ui, "quality", qr, "image-preview-tip-quality");
    }

    // Zeile 3: Verlust (4677) und Farbanzahl (4602) mit tatsächlicher Anzahl (4620)
    if vis.loss.shown() {
        label_r(ui, lab(row(2)), &t("image-preview-opt-loss"), true);
        let lr = Rect::from_min_size(pos2(cx, row(2)), vec2(36.0, 20.0));
        if let Some(v) = fields::number(ui, "pv-lossy", lr, Some(Rect::from_min_size(pos2(cx + 37.0, row(2)), vec2(14.0, 20.0))), Some(s.lossy as f64), (0.0, 100.0), 0, true) {
            s.lossy = v as i32;
        }
        tip(ui, "loss", lr, "image-preview-tip-loss");
    }
    if vis.colors.shown() {
        let en = vis.colors.enabled();
        let gx = right - 53.0;
        super::super::group_box(ui, Rect::from_min_max(pos2(gx, row(2) - 9.0), pos2(right, row(3) + 20.0)), "");
        let cr = Rect::from_min_size(pos2(gx + 3.0, row(2)), vec2(47.0, 20.0));
        let items: Vec<String> = COLOR_COUNTS.iter().map(|x| x.to_string()).collect();
        let shown = if s.format == ExportFormat::Wbmp { 2 } else { s.max_colors };
        if let Some(v) = fields::combo_edit(ui, "pv-colors", cr, &shown.to_string(), &items, en)
            && let Some(n) = fields::parse_num(&v)
        {
            s.max_colors = (n as i32).clamp(2, 256);
        }
        tip(ui, "colors", cr, "image-preview-tip-colors");
        let actual = if s.format == ExportFormat::Wbmp { 2 } else { result.as_ref().map(|r| r.colors).unwrap_or(0) };
        let ar = Rect::from_min_size(pos2(gx + 3.0, row(3)), vec2(47.0, 20.0));
        let resp = ui.interact(ar, Id::new("pv-actual-colors"), Sense::hover());
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, actual.to_string()));
        ui.painter().text(ar.center(), Align2::CENTER_CENTER, actual.to_string(), theme.ui_font(), if en { c.text } else { c.text_disabled });
        resp.on_hover_text(t("image-preview-tip-actual-colors"));
    }

    // Zeile 4: Rastern (4671 + Menge 4676) bzw. Glättung (4683)
    if vis.dither.shown() {
        label_r(ui, lab(row(3)), &t("image-preview-opt-dither"), true);
        let on = !matches!(s.dither, Dither::None);
        let pct = match s.dither {
            Dither::Diffusion(p) => p,
            _ => 100,
        };
        let cbr = Rect::from_min_size(pos2(cx, row(3) + 2.0), vec2(16.0, 16.0));
        if let Some(b) = fields::checkbox(ui, "pv-dither", cbr, "", on, true) {
            s.dither = if b { Dither::Diffusion(100) } else { Dither::None };
        }
        tip(ui, "dither", cbr, "image-preview-tip-dither");
        let nr = Rect::from_min_size(pos2(cx + 21.0, row(3)), vec2(36.0, 20.0));
        if let Some(v) = fields::number(ui, "pv-dither-amount", nr, Some(Rect::from_min_size(pos2(cx + 58.0, row(3)), vec2(14.0, 20.0))), Some(pct as f64), (0.0, 100.0), 0, on) {
            s.dither = Dither::Diffusion((v as i32).clamp(1, 100));
        }
        ui.painter().text(pos2(cx + 75.0, row(3) + 10.0), Align2::LEFT_CENTER, "%", theme.ui_font(), if on { c.text } else { c.text_disabled });
        tip(ui, "dither-amount", nr, "image-preview-tip-dither-amount");
    } else if vis.smoothing.shown() {
        label_r(ui, lab(row(3)), &t("image-preview-opt-smoothing"), true);
        let items = smoothing_items();
        let sr = Rect::from_min_size(pos2(cx, row(3)), vec2(cw.min(right - cx), 20.0));
        if let Some(i) = fields::combo(ui, "pv-smooth", sr, &refs(&items), Some(s.jpeg.smoothing.clamp(0, 8) as usize), true) {
            s.jpeg.smoothing = i as i32;
        }
        tip(ui, "smoothing", sr, "image-preview-tip-smoothing");
    }

    // Mitte: Farbtabelle bzw. JPEG-Kästchen
    let table_top = top + 113.0;
    let table_bottom = (bottom - 170.0).max(table_top + 20.0);
    let s = &mut d.panes[active].settings;
    if vis.jpeg_checks.shown() {
        ui.painter().hline((inner.left() + 8.0)..=(right - 6.0), top + 114.5, Stroke::new(1.0, c.line_mid));
        let pr = Rect::from_min_size(pos2(inner.left() + 9.0, top + 122.0), vec2(right - inner.left() - 9.0, 16.0));
        if let Some(b) = fields::checkbox(ui, "pv-progressive", pr, &t("image-preview-opt-progressive"), s.jpeg.progressive, true) {
            s.jpeg.progressive = b;
        }
        tip(ui, "progressive", pr, "image-preview-tip-progressive");
        let sr = Rect::from_min_size(pos2(inner.left() + 9.0, top + 141.0), vec2(right - inner.left() - 9.0, 16.0));
        if let Some(b) = fields::checkbox(ui, "pv-sharpen", sr, &t("image-preview-opt-sharpen"), s.jpeg.subsampling == 0, true) {
            s.jpeg.subsampling = if b { 0 } else { 1 };
        }
        tip(ui, "sharpen", sr, "image-preview-tip-sharpen");
    } else if !vis.color_table.shown() {
        ui.painter().hline((inner.left() + 8.0)..=(right - 6.0), top + 114.5, Stroke::new(1.0, c.line_mid));
    }
    let mut follow = FollowUp::None;
    if vis.color_table.shown() {
        let area = Rect::from_min_max(pos2(inner.left(), table_top), pos2(right, table_bottom));
        let act = color_table::table(ui, area, &entries, &mut d.table);
        if let Some(col) = act.edit {
            follow = FollowUp::ColorDialog(color_table::ColorMode::Edit(col));
        }
        if let Some(resp) = &act.response
            && let Some(k) = color_table::context_menu(resp, menu_state(d))
        {
            cmd = Some(k);
        }
        // sechs Schalterknöpfe (4828, 4829, 4823, 4697 · 4698, 4699)
        let sel = d.table.selected_entries(&entries);
        let has = !sel.is_empty();
        let all = |f: &dyn Fn(&PaletteEntry) -> bool| has && sel.iter().all(|(_, e)| f(e));
        let pressed = [all(&|e| e.edited), all(&|e| e.transparent), all(&|e| e.web_safe), all(&|e| e.locked), false, false];
        let tips = ["image-preview-tip-edit-color", "image-preview-tip-transparent", "image-preview-tip-snap-web", "image-preview-tip-lock", "image-preview-tip-add", "image-preview-tip-delete"];
        let by = bottom - 169.0;
        let xs = [right - 27.0 * 6.0 - 10.0, right - 27.0 * 5.0 - 10.0, right - 27.0 * 4.0 - 10.0, right - 27.0 * 3.0 - 10.0, right - 54.0, right - 27.0];
        let cmds = [PaletteCmd::EditColor, PaletteCmd::Transparent, PaletteCmd::SnapWeb, PaletteCmd::Lock, PaletteCmd::AddColor, PaletteCmd::DeleteColor];
        let enabled = vis.color_table.enabled();
        for k in 0..6 {
            let en = enabled && (has || k == 4);
            let r = Rect::from_min_size(pos2(xs[k], by), vec2(27.0, 21.0));
            let th = theme.clone();
            let resp = color_table::toggle_button(ui, ("table-btn", k), r, pressed[k] && en, en, &t(tips[k]), |p, r, en| color_table::paint_button_glyph(&th, p, r, en, k));
            if k == 4 {
                // „Farbe hinzufügen“: normales Farbfeld-Popup (Farbraster, HEX)
                if let Some(crate::inspector::popups::ColorChoice::Color(col)) = crate::inspector::popups::color_popup(ui, &resp, Id::new("fp-ipv-add-color"), st, None, false, false) {
                    fp_export::palette_edit::add_color(&mut d.panes[active].settings, col.0 | 0xFF00_0000);
                }
            } else if resp.clicked() {
                cmd = Some(cmds[k]);
            }
        }
    }

    // Pipetten + Transparenzliste bzw. Statuszeile der gewählten Farbe; Hintergrund (Matte)
    let py = bottom - 138.0;
    let first = d.table.selected_entries(&entries).first().map(|(i, e)| (*i, **e));
    let show_status = vis.color_table.shown() && first.is_some();
    if let Some((i, e)) = first.filter(|_| show_status) {
        let text = color_table::status_text(&e, i, alpha_palette);
        let sr = Rect::from_min_size(pos2(inner.left(), py), vec2(right - inner.left(), 60.0));
        let resp = ui.interact(sr, Id::new("pv-color-status"), Sense::hover());
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, text.clone()));
        super::super::text_block(ui, sr.shrink2(vec2(0.0, 4.0)), text.as_str());
    } else {
        let s = &mut d.panes[active].settings;
        if vis.transparency.shown() {
            let pips = [(Pipette::Set, "image-preview-tip-pipette-set"), (Pipette::Add, "image-preview-tip-pipette-add"), (Pipette::Remove, "image-preview-tip-pipette-remove")];
            for (k, (kind, key)) in pips.into_iter().enumerate() {
                let r = Rect::from_min_size(pos2(inner.left() + 3.0 + 28.5 * k as f32, py), vec2(29.0, 29.0));
                let th = theme.clone();
                let on = d.tool == Tool::Pipette(kind);
                if color_table::toggle_button(ui, ("pipette", k), r, on, true, &t(key), |p, r, en| color_table::paint_pipette(&th, p, r, en, kind)).clicked() {
                    d.tool = if on { Tool::Pointer } else { Tool::Pipette(kind) };
                }
            }
            let names = tl(&TRANSPARENCY);
            let ti = match s.transparency {
                Transparency::None => 0,
                Transparency::Index => 1,
                _ => 2,
            };
            let tr = Rect::from_min_max(pos2(inner.left() + 96.0, py + 5.0), pos2(right, py + 25.0));
            if let Some(i) = fields::combo(ui, "pv-transp", tr, &refs(&names), Some(ti), true) {
                s.transparency = [Transparency::None, Transparency::Index, Transparency::IndexAlpha][i.min(2)];
            }
            tip(ui, "transparency", tr, "image-preview-tip-transparency");
        }
        let my = bottom - 101.0;
        let mr = Rect::from_min_size(pos2(right - 26.0, my), vec2(24.0, 21.0));
        label_r(ui, Rect::from_min_max(pos2(inner.left(), my), pos2(mr.left() - 6.0, my + 20.0)), &t("image-preview-opt-matte"), true);
        match crate::inspector::popups::color_button(ui, "pv-matte", mr, st, s.matte.map(fp_color::Argb), true, true, false) {
            Some(crate::inspector::popups::ColorChoice::Color(col)) => s.matte = Some(col.0 | 0xff00_0000),
            Some(crate::inspector::popups::ColorChoice::None) => s.matte = None,
            _ => {}
        }
        tip(ui, "matte", mr, "image-preview-tip-matte");
    }

    // unten: „Nicht verwendete Farben entfernen“ (4612), Zeilensprung (4639)
    let s = &mut d.panes[active].settings;
    ui.painter().hline((inner.left() + 8.0)..=(right - 6.0), bottom - 69.5, Stroke::new(1.0, c.line_mid));
    if vis.remove_unused.shown() {
        let r = Rect::from_min_size(pos2(inner.left() + 9.0, bottom - 65.0), vec2(right - inner.left() - 9.0, 18.0));
        if let Some(b) = fields::checkbox(ui, "pv-optimized", r, &t("image-preview-opt-remove-unused"), s.optimized, vis.remove_unused.enabled()) {
            s.optimized = b;
        }
        tip(ui, "remove-unused", r, "image-preview-tip-remove-unused");
    }
    if vis.interlaced.shown() {
        let r = Rect::from_min_size(pos2(inner.left() + 9.0, bottom - 45.0), vec2(right - inner.left() - 9.0 - 30.0, 16.0));
        if let Some(b) = fields::checkbox(ui, "pv-interl", r, &t("image-preview-opt-interlaced"), s.interlaced, true) {
            s.interlaced = b;
        }
        tip(ui, "interlaced", r, "image-preview-tip-interlaced");
    }

    // Befehle des Palettenmenüs bzw. der Knöpfe ausführen
    if let Some(k) = cmd {
        follow = color_table::apply(k, &mut d.panes[active].settings, &entries, &mut d.table);
    }
    match follow {
        FollowUp::None => {}
        FollowUp::ColorDialog(mode) => {
            let start = match mode {
                color_table::ColorMode::Edit(orig) => entries.iter().find(|e| e.color & 0xFF_FFFF == orig & 0xFF_FFFF).map(|e| e.shown).unwrap_or(orig),
                color_table::ColorMode::Add => 0xFF80_8080,
            };
            d.color_dialog = Some(color_table::ColorDialog::new(active, mode, start));
        }
        FollowUp::LoadPalette => super::load_palette(st, d, active),
        FollowUp::SavePalette => super::save_palette(st, d, &entries),
    }
}
