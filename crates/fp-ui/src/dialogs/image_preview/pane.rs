//! Ein Feld der Bildvorschau (re\17-bildvorschau.md §5.1–5.6): eigene Exporteinstellung,
//! Kopfzeile (Format, Palette bzw. Qualität, Farben, Größe und Ladezeit, „Vorschau“), Vorgabenliste
//! mit „+“, Bildfläche mit Schachbrett.

use super::worker::{self, Job, JobInput, JobKind, PaneResult};
use crate::inspector::fields;
use crate::inspector::glyphs::Glyph;
use crate::panels::optimize;
use crate::theme::{Theme, ThemeKind};
use egui::{pos2, vec2, Align2, Color32, Rect, Stroke, StrokeKind, Ui};
use fp_export::options::{builtin_presets, ColorMode, ExportFormat, OptimizeSettings, PaletteKind};
use fp_export::preview::{modem_seconds, size_label, SizeLabel};
use fp_i18n::t;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Höhe der Kopfzeile in 1-/2-fach (drei Zeilen) bzw. 4-fach (Vorgabenliste + drei Zeilen).
pub const HEADER_H: f32 = 52.0;
pub const HEADER_H_FOUR: f32 = 74.0;
/// Abstand der Bildfläche zum Feldrahmen.
pub const INSET: f32 = 5.0;
/// Entprellung, solange für das Feld noch gerechnet wird (Tippen, Schieber ziehen, §13).
pub const DEBOUNCE: Duration = Duration::from_millis(150);

/// Reihenfolge der eingebauten Vorgaben in der Liste (§8.1); dahinter nach einem Trennstrich die
/// Firepixel-Vorgaben für weitere Formate, dann die eigenen.
/// Indizes in `fp_export::options::builtin_presets`: PNG32, JPEG Höhere Qualität, JPEG Kleinere
/// Datei, GIF Web 216, GIF WebSnap 256, GIF WebSnap 128, GIF Adaptiv 256, Animiertes GIF WebSnap 128
/// (geprüft im Test `preset_list_order_follows_spec`).
const PRESET_ORDER: [usize; 8] = [9, 4, 5, 0, 1, 2, 3, 6];

/// Ein Eintrag der Vorgabenliste.
#[derive(Clone, Debug, PartialEq)]
pub struct PresetEntry {
    /// Anzeigename (übersetzt bei eingebauten Vorgaben).
    pub label: String,
    /// `None` = Trennstrich.
    pub settings: Option<OptimizeSettings>,
}

/// Vorgabenliste: 8 Vorgaben des Originals, Trennstrich, Firepixel-Vorgaben, Trennstrich, eigene.
pub fn preset_entries(own: &[(String, fp_model::ExportSettings)]) -> Vec<PresetEntry> {
    let all = builtin_presets();
    let entry = |p: &fp_export::options::Preset| PresetEntry { label: optimize::preset_label(p.name), settings: Some(p.settings.clone()) };
    let mut out: Vec<PresetEntry> = PRESET_ORDER.iter().filter_map(|&i| all.get(i)).map(entry).collect();
    let extra: Vec<PresetEntry> = all.iter().enumerate().filter(|(i, _)| !PRESET_ORDER.contains(i)).map(|(_, p)| entry(p)).collect();
    if !extra.is_empty() {
        out.push(PresetEntry { label: "------------".into(), settings: None });
        out.extend(extra);
    }
    if !own.is_empty() {
        out.push(PresetEntry { label: "------------".into(), settings: None });
        out.extend(own.iter().map(|(n, e)| PresetEntry { label: n.clone(), settings: Some(OptimizeSettings::from_model(e, None)) }));
    }
    out
}

fn indexed_format(f: ExportFormat, m: ColorMode) -> bool {
    matches!(f, ExportFormat::Gif | ExportFormat::AnimatedGif | ExportFormat::Wbmp) || (m == ColorMode::Indexed && matches!(f, ExportFormat::Png | ExportFormat::Tiff | ExportFormat::Bmp))
}

/// Übernimmt aus `s` alles, was für das Format der Vorgabe `p` keine Rolle spielt bzw. nicht zur
/// Vorgabe gehört (Matte des Dokuments, Status, Exportbereich, Skalierung). Danach heißt
/// `p == s`: „die Einstellung entspricht genau dieser Vorgabe“ (§5.2).
fn normalized(p: &OptimizeSettings, s: &OptimizeSettings) -> OptimizeSettings {
    let mut q = p.clone();
    q.matte = s.matte;
    q.anim.frames = s.anim.frames.clone();
    q.crop = s.crop;
    q.scale = s.scale;
    let indexed = indexed_format(q.format, q.color_mode);
    if !indexed {
        q.palette_kind = s.palette_kind;
        q.web_adaptive = s.web_adaptive;
        q.websnap_tolerance = s.websnap_tolerance;
        q.max_colors = s.max_colors;
        q.grid_levels = s.grid_levels;
        q.dither = s.dither;
        q.sorting = s.sorting;
        q.palette = s.palette.clone();
    }
    if !matches!(q.format, ExportFormat::Gif | ExportFormat::AnimatedGif) {
        q.lossy = s.lossy;
    }
    if q.format != ExportFormat::Jpeg {
        q.jpeg = s.jpeg;
    }
    if !matches!(q.format, ExportFormat::WebP | ExportFormat::WebPLossless | ExportFormat::AnimatedWebP) {
        q.webp = s.webp;
    }
    if q.format != ExportFormat::Avif {
        q.avif = s.avif;
    }
    if !q.format.is_animated() {
        q.anim = s.anim.clone();
    }
    if q.format == ExportFormat::Jpeg {
        q.interlaced = s.interlaced;
        q.transparency = s.transparency;
    }
    q
}

/// Index des Eintrags, dem die Einstellung genau entspricht (sonst leer, §5.2).
pub fn matching_preset(entries: &[PresetEntry], s: &OptimizeSettings) -> Option<usize> {
    entries.iter().position(|e| e.settings.as_ref().is_some_and(|p| normalized(p, s) == *s))
}

/// Lädt eine Vorgabe in ein Feld: Matte, Status, Exportbereich und Skalierung bleiben.
pub fn load_preset(p: &OptimizeSettings, s: &OptimizeSettings) -> OptimizeSettings {
    let mut q = p.clone();
    q.matte = s.matte;
    q.anim.frames = s.anim.frames.clone();
    q.crop = s.crop;
    q.scale = s.scale;
    q
}

// ---------------------------------------------------------------------- Kopfzeile

/// Texte der Kopfzeile eines Feldes (§5.2, §5.4).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HeaderText {
    pub format: String,
    /// Palette bzw. „Qualität 60 (2)“; leer bei Truecolor-PNG/TIFF/BMP.
    pub palette: String,
    /// „216 Farben“ (angefordert), „Millionen“, „Millionen+Alpha“.
    pub colors: String,
    /// „53.89K  8 s bei 56 KBit/s“; leer, solange kein Ergebnis da ist.
    pub size: String,
}

/// Formatname der Kopfzeile (4662): GIF, JPEG, PNG, Animiertes GIF, Plug-in-Name.
pub fn format_label(s: &OptimizeSettings) -> String {
    match s.format {
        ExportFormat::Gif => "GIF".into(),
        ExportFormat::AnimatedGif => t("panel-opt-format-anim-gif"),
        ExportFormat::WebPLossless => t("panel-opt-format-webp-lossless"),
        ExportFormat::AnimatedWebP => t("panel-opt-format-anim-webp"),
        ExportFormat::Custom(_) => t("panel-opt-palette-custom"),
        f => f.display_name().to_string(),
    }
}

/// Farbtiefe für die Kopfzeile (Format bestimmt sie, wo es nur eine gibt).
fn header_color_mode(s: &OptimizeSettings) -> ColorMode {
    match s.format {
        ExportFormat::Gif | ExportFormat::AnimatedGif | ExportFormat::Wbmp => ColorMode::Indexed,
        ExportFormat::Jpeg => ColorMode::Rgb24,
        ExportFormat::Png | ExportFormat::Tiff | ExportFormat::Bmp | ExportFormat::Pict | ExportFormat::Custom(_) => s.color_mode,
        _ => ColorMode::Rgba32,
    }
}

/// Palette bzw. Qualität der Kopfzeile (4625). WebSnap-Adaptiv heißt „Adaptiv“; Graustufen mit
/// 2 Stufen „Schwarzweiß“ (im Original fälschlich „Graustufen“, §5.2: im Nachbau korrigiert).
pub fn palette_label(s: &OptimizeSettings) -> String {
    match s.format {
        ExportFormat::Jpeg => return fp_i18n::tr!("image-preview-quality-smoothing", quality = s.jpeg.quality.to_string(), smoothing = s.jpeg.smoothing.to_string()),
        ExportFormat::WebP | ExportFormat::AnimatedWebP if !(s.format == ExportFormat::AnimatedWebP && s.webp.lossless) => return fp_i18n::tr!("image-preview-quality", quality = s.webp.quality.to_string()),
        ExportFormat::Avif => return fp_i18n::tr!("image-preview-quality", quality = s.avif.quality.to_string()),
        ExportFormat::Wbmp => return t("panel-opt-palette-bw"),
        _ => {}
    }
    if header_color_mode(s) != ColorMode::Indexed {
        return String::new();
    }
    t(match s.palette_kind {
        PaletteKind::Custom => "panel-opt-palette-custom",
        PaletteKind::Adaptive => "panel-opt-palette-adaptive",
        PaletteKind::Uniform => "panel-opt-palette-uniform",
        PaletteKind::Grayscale if s.max_colors == 2 => "panel-opt-palette-bw",
        PaletteKind::Grayscale => "panel-opt-palette-grayscale",
        PaletteKind::Mac => "panel-opt-palette-mac",
        PaletteKind::Windows => "panel-opt-palette-windows",
        PaletteKind::Exact => "panel-opt-palette-exact",
        PaletteKind::Web216 => "panel-opt-palette-web216",
    })
}

/// Farbzeile (4663): angeforderte Farbanzahl, „Millionen“ bzw. „Millionen+Alpha“.
pub fn colors_label(s: &OptimizeSettings) -> String {
    match header_color_mode(s) {
        ColorMode::Indexed => {
            let n = if s.format == ExportFormat::Wbmp { 2 } else { s.max_colors.clamp(2, 256) };
            fp_i18n::tr!("image-preview-colors", n = n)
        }
        ColorMode::Rgb24 => t("image-preview-millions"),
        ColorMode::Rgba32 => t("image-preview-millions-alpha"),
    }
}

/// Größe und Ladezeit (§5.4): „460.85K  74 s bei 56 KBit/s“, „58 Byte  0 s bei 56 KBit/s“.
pub fn size_text(bytes: usize) -> String {
    let size = match size_label(bytes) {
        SizeLabel::Bytes(n) => fp_i18n::tr!("image-preview-size-bytes", n = n.to_string()),
        SizeLabel::Kilo(k) => k,
    };
    fp_i18n::tr!("image-preview-size-line", size = size, seconds = modem_seconds(bytes).to_string())
}

/// Alle Texte der Kopfzeile.
pub fn header_text(s: &OptimizeSettings, result: Option<&PaneResult>) -> HeaderText {
    let size = match result {
        Some(r) if r.error.is_some() => r.error.clone().unwrap_or_default(),
        Some(r) => size_text(r.bytes),
        None => String::new(),
    };
    HeaderText { format: format_label(s), palette: palette_label(s), colors: colors_label(s), size }
}

// ---------------------------------------------------------------------- Feld

/// Ein Feld mit eigener Exporteinstellung.
pub struct Pane {
    pub settings: OptimizeSettings,
    /// Kästchen „Vorschau“ (aus = Original, §5.3).
    pub show_preview: bool,
    /// Schon einmal sichtbar gewesen (neue Felder übernehmen beim Umschalten die Einstellung).
    pub initialized: bool,
    /// Name der zuletzt gewählten Vorgabe (Vorbelegung des Dialogs „Vorgabenname“).
    pub last_preset: Option<String>,
    /// Letztes fertiges Ergebnis (wird weiter gezeigt, solange neu gerechnet wird).
    pub result: Option<Arc<PaneResult>>,
    pub(super) job: Option<Job>,
    /// Erste Änderung, die noch auf ihren Auftrag wartet (Entprellung).
    pub(super) pending_since: Option<Instant>,
    pub(super) texture: Option<(u64, egui::TextureHandle)>,
}

impl Pane {
    pub fn new(settings: OptimizeSettings) -> Pane {
        Pane { settings, show_preview: true, initialized: false, last_preset: None, result: None, job: None, pending_since: None, texture: None }
    }

    /// Wird für dieses Feld gerade gerechnet (oder wartet eine Änderung auf ihren Auftrag)?
    pub fn is_busy(&self) -> bool {
        self.job.is_some() || self.pending_since.is_some()
    }

    /// Textur des angezeigten Ergebnisses.
    pub fn texture_id(&self) -> Option<egui::TextureId> {
        self.texture.as_ref().map(|(_, t)| t.id())
    }

    /// Schlüssel des laufenden Auftrags.
    pub fn job_key(&self) -> Option<u64> {
        self.job.as_ref().map(|j| j.key)
    }

    /// Ergebnis abholen bzw. einen Auftrag starten, wenn sich Dokument oder Einstellung geändert
    /// haben. Ein neuer Auftrag bricht den alten ab; solange noch gerechnet wird, wartet eine
    /// Änderung [`DEBOUNCE`] auf weitere (Tippen, Schieber).
    pub(super) fn update(&mut self, ctx: &egui::Context, base: &JobInput, cache: &Arc<worker::RenderCache>) {
        if let Some(j) = &self.job
            && let Some(r) = j.poll()
        {
            let key = j.key;
            self.job = None;
            if let Some(r) = r
                && r.key == key
            {
                self.result = Some(Arc::new(r));
            }
        }
        let kind = JobKind::Export(Box::new(self.settings.clone()));
        let key = worker::settings_key(base.doc_key, base.page, base.frame, &kind);
        let done = self.result.as_ref().is_some_and(|r| r.key == key);
        let running = self.job.as_ref().is_some_and(|j| j.key == key);
        if done || running {
            self.pending_since = None;
            if done && let Some(j) = self.job.take() {
                j.cancel();
            }
            return;
        }
        // veralteter Auftrag: sofort abbrechen
        if let Some(j) = self.job.take() {
            j.cancel();
            let now = Instant::now();
            self.pending_since.get_or_insert(now);
        }
        if let Some(t0) = self.pending_since
            && t0.elapsed() < DEBOUNCE
        {
            ctx.request_repaint_after(DEBOUNCE.saturating_sub(t0.elapsed()));
            return;
        }
        self.pending_since = None;
        let input = JobInput { kind, key, ..base.clone() };
        self.job = Some(worker::spawn(input, Arc::clone(cache), ctx));
    }

    /// Bild für die Anzeige als Textur (einmal je Ergebnis hochgeladen).
    pub(super) fn texture(&mut self, ctx: &egui::Context, res: &PaneResult) -> Option<egui::TextureHandle> {
        if let Some((k, tex)) = &self.texture
            && *k == res.key
        {
            return Some(tex.clone());
        }
        let tex = upload(ctx, res.image.as_deref()?)?;
        self.texture = Some((res.key, tex.clone()));
        Some(tex)
    }
}

/// Bild hochladen: Vergrößern ohne Glätten (Pixelblöcke), Verkleinern gemittelt (§5.5). Größer als
/// die Grafikkarte erlaubt: nur die Anzeige verkleinern (die Größe stimmt trotzdem).
pub(super) fn upload(ctx: &egui::Context, img: &fp_raster::RgbaImage) -> Option<egui::TextureHandle> {
    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 {
        return None;
    }
    let max = ctx.input(|i| i.max_texture_side).max(256);
    let opts = egui::TextureOptions { magnification: egui::TextureFilter::Nearest, minification: egui::TextureFilter::Linear, ..Default::default() };
    let ci = if w <= max && h <= max {
        egui::ColorImage::from_rgba_unmultiplied([w, h], img.as_bytes())
    } else {
        let k = max as f64 / w.max(h) as f64;
        let (nw, nh) = (((w as f64 * k) as u32).max(1), ((h as f64 * k) as u32).max(1));
        let small = fp_raster::resample::resize(img, nw, nh, fp_raster::Sampler::Bilinear);
        egui::ColorImage::from_rgba_unmultiplied([nw as usize, nh as usize], small.as_bytes())
    };
    Some(ctx.load_texture("fp-image-preview", ci, opts))
}

// ---------------------------------------------------------------------- Zeichnen

/// Wo die Teile eines Feldes liegen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaneLayout {
    pub frame: Rect,
    pub image: Rect,
    pub header: Rect,
    /// Kopfzeile unter dem Bild (Feld 1 im 2-fach-, Felder 1 und 3 im 4-fach-Modus).
    pub header_below: bool,
    /// Schmale Anordnung des 4-fach-Modus (Vorgabenliste in eigener Zeile).
    pub compact: bool,
}

impl PaneLayout {
    pub fn new(frame: Rect, index: usize, mode: super::ViewMode) -> PaneLayout {
        let compact = mode == super::ViewMode::Four;
        let below = mode != super::ViewMode::One && (index == 1 || index == 3);
        let hh = if compact { HEADER_H_FOUR } else { HEADER_H };
        let (image, header) = if below {
            (Rect::from_min_max(frame.min + vec2(INSET, INSET), pos2(frame.right() - INSET, frame.bottom() - hh)), Rect::from_min_max(pos2(frame.left(), frame.bottom() - hh), frame.max))
        } else {
            (Rect::from_min_max(pos2(frame.left() + INSET, frame.top() + hh), frame.max - vec2(INSET, INSET)), Rect::from_min_max(frame.min, pos2(frame.right(), frame.top() + hh)))
        };
        PaneLayout { frame, image, header, header_below: below, compact }
    }
}

/// Was der Nutzer in der Kopfzeile eines Feldes getan hat.
#[derive(Default)]
pub(super) struct HeaderAction {
    /// irgendein Bedienelement betätigt (Feld wird aktiv)
    pub touched: bool,
    pub toggle_preview: bool,
    pub preset: Option<usize>,
    pub save_preset: bool,
}

/// Hintergrund außerhalb des Bildes und Schachbrettfarben (§5.5: #E3E3E3, #FFFFFF/#DFDFDF im
/// hellen Design; die dunklen Designs nehmen ihre eigenen Leinwandfarben).
fn surface_colors(theme: &Theme) -> (Color32, Color32, Color32) {
    if theme.kind == ThemeKind::Normal {
        (Color32::from_rgb(0xE3, 0xE3, 0xE3), Color32::WHITE, Color32::from_rgb(0xDF, 0xDF, 0xDF))
    } else {
        (theme.colors.canvas_bg, theme.colors.checker_a, theme.colors.checker_b)
    }
}

/// 2×2-Schachbretttextur (je Farbpaar einmal).
fn checker_texture(ctx: &egui::Context, a: Color32, b: Color32) -> egui::TextureHandle {
    let id = egui::Id::new(("fp-ipv-checker", a.to_array(), b.to_array()));
    if let Some(t) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return t;
    }
    let t = ctx.load_texture("fp-ipv-checker", egui::ColorImage::new([2, 2], vec![a, b, b, a]), egui::TextureOptions::NEAREST_REPEAT);
    ctx.data_mut(|d| d.insert_temp(id, t.clone()));
    t
}

/// Zeichnet die Bildfläche: Grund, Schachbrett (8 Bildschirmpunkte, Weiß links oben am
/// Bildursprung), Bild, Rahmen. `img_rect` = Lage des ganzen Bildes auf dem Bildschirm.
pub(super) fn paint_image(ui: &Ui, area: Rect, img_rect: Option<Rect>, tex: Option<&egui::TextureHandle>) {
    let theme = Theme::current(ui.ctx());
    let (bg, ca, cb) = surface_colors(&theme);
    let p = ui.painter().with_clip_rect(area.intersect(ui.clip_rect()));
    p.rect_filled(area, 0.0, bg);
    if let Some(ir) = img_rect {
        let vis = ir.intersect(area);
        if vis.is_positive() {
            let ct = checker_texture(ui.ctx(), ca, cb);
            let cell = 8.0;
            let uv = Rect::from_min_max(((vis.min - ir.min) / (2.0 * cell)).to_pos2(), ((vis.max - ir.min) / (2.0 * cell)).to_pos2());
            p.image(ct.id(), vis, uv, Color32::WHITE);
            if let Some(t) = tex {
                p.image(t.id(), ir, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
            }
        }
    }
    ui.painter().rect_stroke(area.expand(1.0), 0.0, Stroke::new(1.0, theme.colors.line_dark), StrokeKind::Inside);
}

/// Feldrahmen; das aktive Feld trägt einen zusätzlichen inneren Doppelrahmen (§5.1).
pub(super) fn paint_frame(ui: &Ui, frame: Rect, active: bool) {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    ui.painter().rect_stroke(frame, 0.0, Stroke::new(1.0, c.line_mid), StrokeKind::Inside);
    if active {
        ui.painter().rect_stroke(frame.shrink(1.0), 0.0, Stroke::new(1.0, c.focus), StrokeKind::Inside);
        ui.painter().rect_stroke(frame.shrink(3.0), 0.0, Stroke::new(1.0, crate::theme::with_alpha(c.focus, 150)), StrokeKind::Inside);
    }
}

/// Tooltip über einem Bedienelement, das selbst keinen hat (nimmt keine Klicks weg).
fn tip(ui: &Ui, id: impl std::hash::Hash + std::fmt::Debug, r: Rect, key: &str) {
    ui.interact(r, egui::Id::new(id), egui::Sense::hover()).on_hover_text(t(key));
}

/// Kopfzeile mit Bedienelementen.
pub(super) fn header(ui: &mut Ui, index: usize, lay: &PaneLayout, pane: &Pane, entries: &[PresetEntry], busy: bool) -> HeaderAction {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let font = theme.ui_font();
    let mut act = HeaderAction::default();
    let h = header_text(&pane.settings, pane.result.as_deref());
    let names: Vec<&str> = entries.iter().map(|e| e.label.as_str()).collect();
    let sel = matching_preset(entries, &pane.settings);
    let preview_l = t("image-preview-preview");
    let saved_l = t("image-preview-saved-settings");
    let check_w = (13.0 + 5.0 + super::super::text_w(ui.ctx(), &preview_l) + 2.0).ceil();
    let hr = lay.header;
    let text = |ui: &Ui, x: f32, y: f32, s: &str, max_w: f32, align: Align2| {
        crate::widgets::text_clipped(ui.painter(), pos2(x, y), align, s, font.clone(), c.text, max_w.max(8.0));
    };
    let size_line = if busy && h.size.is_empty() { t("image-preview-busy") } else { h.size.clone() };
    let combo_row = |ui: &mut Ui, r: Rect, act: &mut HeaderAction| {
        let cr = Rect::from_min_max(r.min, pos2(r.right() - 22.0, r.bottom()));
        if let Some(i) = fields::combo(ui, ("fp-ipv-preset", index), cr, &names, sel, true) {
            act.preset = Some(i);
            act.touched = true;
        }
        tip(ui, ("fp-ipv-preset-tip", index), cr, "image-preview-saved-settings-tip");
        let pr = Rect::from_center_size(pos2(r.right() - 9.0, r.center().y), vec2(18.0, 18.0));
        let plus_tip = t("image-preview-save-current-tip");
        let plus = fields::icon_button(ui, ("fp-ipv-plus", index), pr, Glyph::Plus, true, &plus_tip);
        plus.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, plus_tip.as_str()));
        if plus.clicked() {
            act.save_preset = true;
            act.touched = true;
        }
    };
    if lay.compact {
        // 4-fach: Vorgabenliste in eigener Zeile (oben bzw. ganz unten), Infozeilen dazwischen
        let (list_y, info_y) = if lay.header_below { (hr.bottom() - 26.0, hr.top() + 2.0) } else { (hr.top() + 4.0, hr.top() + 26.0) };
        combo_row(ui, Rect::from_min_size(pos2(hr.left() + INSET, list_y), vec2(hr.width() - 2.0 * INSET, 20.0)), &mut act);
        let x0 = hr.left() + INSET + 1.0;
        let right = hr.right() - INSET - 1.0;
        let cb = Rect::from_min_size(pos2(right - check_w, info_y), vec2(check_w, 16.0));
        let toggled = fields::checkbox(ui, ("fp-ipv-show", index), cb, &preview_l, pane.show_preview, true).is_some();
        tip(ui, ("fp-ipv-show-tip", index), cb, "image-preview-preview-tip");
        if toggled {
            act.toggle_preview = true;
            act.touched = true;
        }
        let row = |k: f32| info_y + 8.0 + 16.0 * k;
        if pane.show_preview {
            text(ui, x0, row(0.0), &h.format, cb.left() - x0 - 4.0, Align2::LEFT_CENTER);
        }
        let colors_w = super::super::text_w(ui.ctx(), &h.colors);
        if pane.show_preview {
            text(ui, x0, row(1.0), &h.palette, right - colors_w - 8.0 - x0, Align2::LEFT_CENTER);
            text(ui, x0, row(2.0), &size_line, right - x0, Align2::LEFT_CENTER);
        }
        text(ui, right, row(1.0), &h.colors, right - x0, Align2::RIGHT_CENTER);
    } else {
        // 1-/2-fach: links Format/Palette/Größe, Mitte „Vorschau“ und Farben, rechts nach einer
        // senkrechten Linie „Gespeicherte Einstellungen:“ und die Liste mit „+“
        let y0 = if lay.header_below { hr.top() + 1.0 } else { hr.top() + 4.0 };
        let rb = (super::super::text_w(ui.ctx(), &saved_l) + 8.0).max(178.0).min(hr.width() * 0.48).ceil();
        let vx = hr.right() - INSET - rb - 6.0;
        ui.painter().vline(vx.floor() + 0.5, (y0 + 1.0)..=(y0 + 44.0), Stroke::new(1.0, c.line_mid));
        let rx = vx + 6.0;
        text(ui, rx, y0 + 8.0, &saved_l, hr.right() - INSET - rx, Align2::LEFT_CENTER);
        combo_row(ui, Rect::from_min_max(pos2(rx, y0 + 18.0), pos2(hr.right() - INSET, y0 + 38.0)), &mut act);
        let x0 = hr.left() + INSET + 1.0;
        let mid_r = vx - 8.0;
        let cb = Rect::from_min_size(pos2(mid_r - check_w, y0), vec2(check_w, 16.0));
        let toggled = fields::checkbox(ui, ("fp-ipv-show", index), cb, &preview_l, pane.show_preview, true).is_some();
        tip(ui, ("fp-ipv-show-tip", index), cb, "image-preview-preview-tip");
        if toggled {
            act.toggle_preview = true;
            act.touched = true;
        }
        if pane.show_preview {
            text(ui, x0, y0 + 8.0, &h.format, cb.left() - x0 - 4.0, Align2::LEFT_CENTER);
            text(ui, x0, y0 + 24.0, &h.palette, cb.left() - x0 - 4.0, Align2::LEFT_CENTER);
            text(ui, x0, y0 + 40.0, &size_line, vx - 4.0 - x0, Align2::LEFT_CENTER);
        }
        text(ui, mid_r, y0 + 24.0, &h.colors, mid_r - x0, Align2::RIGHT_CENTER);
    }
    act
}
