//! Optimieren-Bedienfeld (`CExportPanel`, Vorlage 4735, 11 §2.2) über fp-export: gespeicherte
//! Einstellungen (Vorgaben inkl. WebP/AVIF/APNG), Format, Palette, Farben, Verlust, Rastern
//! (Dither), Transparenz, Matte (Hintergr.), JPEG-Qualität/Glättung, Farbtabelle mit Sortierung,
//! Farbanzahl und Knopfleiste. Die Einstellungen stehen in den Exporteinstellungen der Seite
//! (`pi.export.settings`, ein Verlaufsschritt).
//!
//! Außerdem der Renderer-Adapter für fp-export ([`ExportRenderer`]) und die zwischengespeicherte
//! Exportvorschau ([`preview`]) für die Farbtabelle und den Dialog „Bildvorschau“.

use super::common;
use crate::app_state::{Action, AppState};
use crate::inspector::fields::{self, Dl};
use crate::theme::Theme;
use egui::{pos2, vec2, Align2, Color32, Id, Rect, Sense, Stroke, StrokeKind, Ui};
use fp_export::options::{builtin_presets, ColorMode, Dither, ExportFormat, OptimizeSettings, PaletteKind, Transparency};
use fp_export::render::{Background, PageRenderer, RenderRequest};
use serde_json::json;
use std::sync::{Arc, Mutex, OnceLock};

/// Renderer-Adapter fp-render → fp-export (Leinwand bzw. transparenter Grund).
pub struct ExportRenderer {
    pub normal: Arc<fp_render::Renderer>,
}

fn transparent_renderer() -> Arc<fp_render::Renderer> {
    static R: OnceLock<Arc<fp_render::Renderer>> = OnceLock::new();
    Arc::clone(R.get_or_init(|| Arc::new(fp_render::Renderer::new(fp_render::RenderOptions { background: fp_render::BackgroundMode::Transparent, ..Default::default() }))))
}

impl PageRenderer for ExportRenderer {
    fn render(&self, doc: &fp_model::Document, req: &RenderRequest) -> Result<fp_raster::RgbaImage, String> {
        let r = match req.background {
            Background::Transparent => transparent_renderer(),
            _ => Arc::clone(&self.normal),
        };
        let rect = req.area.or_else(|| fp_export::render::page_rect(doc, req.page)).ok_or_else(|| fp_i18n::t("panel-opt-no-canvas"))?;
        Ok(r.render_rect(doc, req.page, req.frame, rect).image)
    }
}

// ---------------------------------------------------------------------- Einstellungen

/// Formatliste (4610 + moderne Formate): Anzeigename, Format, Farbtiefe.
pub const FORMATS: &[(&str, Option<(ExportFormat, ColorMode)>)] = &[
    ("GIF", Some((ExportFormat::Gif, ColorMode::Indexed))),
    ("Animiertes GIF", Some((ExportFormat::AnimatedGif, ColorMode::Indexed))),
    ("JPEG", Some((ExportFormat::Jpeg, ColorMode::Rgb24))),
    ("------------", None),
    ("PNG 8", Some((ExportFormat::Png, ColorMode::Indexed))),
    ("PNG 24", Some((ExportFormat::Png, ColorMode::Rgb24))),
    ("PNG32", Some((ExportFormat::Png, ColorMode::Rgba32))),
    ("------------", None),
    ("WebP", Some((ExportFormat::WebP, ColorMode::Rgba32))),
    ("WebP verlustfrei", Some((ExportFormat::WebPLossless, ColorMode::Rgba32))),
    ("Animiertes WebP", Some((ExportFormat::AnimatedWebP, ColorMode::Rgba32))),
    ("AVIF", Some((ExportFormat::Avif, ColorMode::Rgba32))),
    ("APNG", Some((ExportFormat::Apng, ColorMode::Rgba32))),
    ("------------", None),
    ("TIFF 8", Some((ExportFormat::Tiff, ColorMode::Indexed))),
    ("TIFF 24", Some((ExportFormat::Tiff, ColorMode::Rgb24))),
    ("TIFF 32", Some((ExportFormat::Tiff, ColorMode::Rgba32))),
    ("BMP 8", Some((ExportFormat::Bmp, ColorMode::Indexed))),
    ("BMP 24", Some((ExportFormat::Bmp, ColorMode::Rgb24))),
    ("WBMP", Some((ExportFormat::Wbmp, ColorMode::Indexed))),
    ("QOI", Some((ExportFormat::Qoi, ColorMode::Rgba32))),
    ("ICO", Some((ExportFormat::Ico, ColorMode::Rgba32))),
];

/// Palettenliste (4616): Anzeigename → (Art, Web-adaptiv, feste Farbzahl).
pub const PALETTES: &[(&str, PaletteKind, bool, Option<i32>)] = &[
    ("Adaptiv", PaletteKind::Adaptive, false, None),
    ("Web-Adaptiv", PaletteKind::Adaptive, true, None),
    ("Web 216", PaletteKind::Web216, false, Some(216)),
    ("Exakt", PaletteKind::Exact, false, None),
    ("Macintosh", PaletteKind::Mac, false, Some(256)),
    ("Windows", PaletteKind::Windows, false, Some(256)),
    ("Graustufen", PaletteKind::Grayscale, false, None),
    ("Schwarzweiß", PaletteKind::Grayscale, false, Some(2)),
    ("Einheitlich", PaletteKind::Uniform, false, None),
    ("Benutzerdefiniert", PaletteKind::Custom, false, None),
];

const COLOR_COUNTS: [&str; 8] = ["2", "4", "8", "16", "32", "64", "128", "256"];
const TRANSPARENCY: [&str; 3] = ["panel-opt-transp-none", "panel-opt-transp-index", "panel-opt-transp-alpha"];

/// Schlüssel der übersetzbaren Namen in [`FORMATS`], [`PALETTES`] und den eingebauten Vorgaben
/// (`fp_export::options::builtin_presets`); die übrigen Namen sind Formatnamen und bleiben.
const NAME_KEYS: &[(&str, &str)] = &[
    ("Animiertes GIF", "panel-opt-format-anim-gif"),
    ("WebP verlustfrei", "panel-opt-format-webp-lossless"),
    ("Animiertes WebP", "panel-opt-format-anim-webp"),
    ("Adaptiv", "panel-opt-palette-adaptive"),
    ("Web-Adaptiv", "panel-opt-palette-web-adaptive"),
    ("Web 216", "panel-opt-palette-web216"),
    ("Exakt", "panel-opt-palette-exact"),
    ("Macintosh", "panel-opt-palette-mac"),
    ("Windows", "panel-opt-palette-windows"),
    ("Graustufen", "panel-opt-palette-grayscale"),
    ("Schwarzweiß", "panel-opt-palette-bw"),
    ("Einheitlich", "panel-opt-palette-uniform"),
    ("Benutzerdefiniert", "panel-opt-palette-custom"),
];

/// Vorgabenamen aus fp-export → Schlüssel.
const PRESET_KEYS: &[(&str, &str)] = &[
    ("GIF Web 216", "panel-opt-preset-gif-web216"),
    ("GIF WebSnap 256", "panel-opt-preset-gif-websnap256"),
    ("GIF WebSnap 128", "panel-opt-preset-gif-websnap128"),
    ("GIF Adaptiv 256", "panel-opt-preset-gif-adaptive256"),
    ("JPEG – Höhere Qualität", "panel-opt-preset-jpeg-better"),
    ("JPEG – Kleinere Datei", "panel-opt-preset-jpeg-smaller"),
    ("Animiertes GIF WebSnap 128", "panel-opt-preset-anim-gif-websnap128"),
    ("PNG8", "panel-opt-preset-png8"),
    ("PNG24", "panel-opt-preset-png24"),
    ("PNG32", "panel-opt-preset-png32"),
    ("WebP 80", "panel-opt-preset-webp80"),
    ("WebP verlustfrei", "panel-opt-preset-webp-lossless"),
    ("AVIF 60", "panel-opt-preset-avif60"),
    ("APNG", "panel-opt-preset-apng"),
    ("Animiertes WebP", "panel-opt-preset-anim-webp"),
];

fn translated(name: &str, table: &[(&str, &str)]) -> String {
    table.iter().find(|(de, _)| *de == name).map(|(_, k)| fp_i18n::t(k)).unwrap_or_else(|| name.to_string())
}

/// Angezeigter Name eines Eintrags von [`FORMATS`] (Trennlinien bleiben).
pub fn format_name(i: usize) -> String {
    FORMATS.get(i).map(|f| translated(f.0, NAME_KEYS)).unwrap_or_default()
}

/// Angezeigter Name eines Eintrags von [`PALETTES`].
pub fn palette_name(i: usize) -> String {
    PALETTES.get(i).map(|p| translated(p.0, NAME_KEYS)).unwrap_or_default()
}

/// Übersetzter Anzeigename einer Export-Vorgabe (`Preset::name` aus fp-export); unbekannte
/// (z. B. eigene, gespeicherte) Namen bleiben unverändert.
pub fn preset_label(name: &str) -> String {
    translated(name, PRESET_KEYS)
}

/// Index der Formatliste.
pub fn format_index(s: &OptimizeSettings) -> Option<usize> {
    FORMATS.iter().position(|(_, f)| *f == Some((s.format, s.color_mode))).or_else(|| FORMATS.iter().position(|(_, f)| f.is_some_and(|x| x.0 == s.format)))
}

/// Index der Palettenliste.
pub fn palette_index(s: &OptimizeSettings) -> usize {
    if s.palette_kind == PaletteKind::Grayscale && s.max_colors == 2 {
        return 7;
    }
    PALETTES.iter().position(|p| p.1 == s.palette_kind && (p.1 != PaletteKind::Adaptive || p.2 == s.web_adaptive)).unwrap_or(0)
}

/// Passende eingebaute Vorgabe.
pub fn preset_index(s: &OptimizeSettings) -> Option<usize> {
    builtin_presets().iter().position(|p| {
        let q = &p.settings;
        q.format == s.format
            && q.color_mode == s.color_mode
            && (s.color_mode != ColorMode::Indexed || (q.palette_kind == s.palette_kind && q.web_adaptive == s.web_adaptive && q.max_colors == s.max_colors))
            && (s.format != ExportFormat::Jpeg || q.jpeg.quality == s.jpeg.quality)
            && (!matches!(s.format, ExportFormat::WebP) || q.webp.quality == s.webp.quality)
            && (!matches!(s.format, ExportFormat::Avif) || q.avif.quality == s.avif.quality)
    })
}

/// Kurzbeschreibung (wenn keine Vorgabe passt), z. B. „GIF Adaptiv 128“.
pub fn settings_label(s: &OptimizeSettings) -> String {
    let f = format_index(s).map(format_name).unwrap_or_else(|| fp_i18n::t("panel-opt-custom"));
    if s.color_mode == ColorMode::Indexed && s.format != ExportFormat::Wbmp {
        format!("{f} {} {}", palette_name(palette_index(s)), s.max_colors)
    } else if s.format == ExportFormat::Jpeg {
        format!("JPEG {}", s.jpeg.quality)
    } else {
        f
    }
}

/// Einstellungen in die Seite schreiben (ein Verlaufsschritt).
pub fn apply_settings(st: &mut AppState, s: &OptimizeSettings) {
    let Some(d) = st.session.active() else { return };
    let base = d.doc.pages.get(d.page).and_then(|p| p.root.export.as_deref().cloned()).unwrap_or_default();
    let model = s.to_model(&base);
    match serde_json::to_value(&model) {
        Ok(v) => st.run("pi.export.settings", json!({"settings": v})),
        Err(e) => st.push(Action::Status(fp_i18n::tr!("panel-opt-settings-error", error = e.to_string()))),
    }
}

fn current(st: &AppState) -> Option<OptimizeSettings> {
    st.session.active().map(|d| fp_export::doc_export::page_settings(&d.doc, d.page))
}

// ---------------------------------------------------------------------- Vorschau-Cache

/// Ergebnis der Exportvorschau.
#[derive(Clone)]
pub struct PreviewResult {
    pub bytes: usize,
    pub colors: usize,
    pub palette: Vec<u32>,
    /// Farbtabelle mit Zähler und Kennzeichen (`fp_export::palette_edit`, nur indiziert).
    pub entries: Vec<fp_export::palette_edit::PaletteEntry>,
    pub image: Option<fp_raster::RgbaImage>,
    pub error: Option<String>,
}

type PreviewCache = Arc<Mutex<Option<(String, PreviewResult)>>>;

fn preview_cache(ctx: &egui::Context) -> PreviewCache {
    let id = Id::new("optimize-preview-cache");
    if let Some(c) = ctx.data(|d| d.get_temp::<PreviewCache>(id)) {
        return c;
    }
    let c: PreviewCache = Arc::new(Mutex::new(None));
    ctx.data_mut(|d| d.insert_temp(id, Arc::clone(&c)));
    c
}

/// Exportvorschau des aktiven Dokuments mit den Einstellungen `s` (zwischengespeichert je
/// Dokument-Generation, Seite, Status und Einstellungen). `with_image` liefert auch das Bild.
pub fn preview(ctx: &egui::Context, st: &AppState, s: &OptimizeSettings, with_image: bool) -> Option<PreviewResult> {
    let d = st.session.active()?;
    let key = format!("{}-{}-{}-{}-{:?}-{with_image}", d.id.0, d.generation, d.page, d.frame, s);
    let cache = preview_cache(ctx);
    if let Ok(g) = cache.lock()
        && let Some((k, r)) = g.as_ref()
        && *k == key
    {
        return Some(r.clone());
    }
    let r = ExportRenderer { normal: Arc::clone(&st.renderer) };
    let req = RenderRequest { page: d.page, frame: d.frame, background: if s.matte.is_some() { Background::Transparent } else { Background::Canvas }, ..Default::default() };
    let res = match r.render(&d.doc, &req) {
        Ok(img) => {
            let ctxe = fp_export::doc_export::page_context(&d.doc, d.page);
            match fp_export::export::export_image(&img, s, &ctxe, &fp_export::export::EncodeExtras::standalone()) {
                Ok(ex) => {
                    let palette = match &ex.optimized {
                        fp_export::Optimized::Indexed(ix) => ix.palette.clone(),
                        _ => Vec::new(),
                    };
                    let image = with_image.then(|| fp_export::export::preview(&ex, s, (0, 0)));
                    let entries = if palette.is_empty() { Vec::new() } else { fp_export::palette_edit::current_palette(&img, s, &ctxe) };
                    PreviewResult { bytes: ex.bytes.len(), colors: ex.colors, palette, entries, image, error: None }
                }
                Err(e) => PreviewResult { bytes: 0, colors: 0, palette: Vec::new(), entries: Vec::new(), image: None, error: Some(e.to_string()) },
            }
        }
        Err(e) => PreviewResult { bytes: 0, colors: 0, palette: Vec::new(), entries: Vec::new(), image: None, error: Some(e) },
    };
    if let Ok(mut g) = cache.lock() {
        *g = Some((key, res.clone()));
    }
    Some(res)
}

// ---------------------------------------------------------------------- Bedienfeld

/// Zeichnet das Bedienfeld.
pub fn show(ui: &mut Ui, st: &mut AppState) {
    let theme = Theme::current(ui.ctx());
    let c = theme.colors.clone();
    let Some(s) = current(st) else {
        common::not_available(ui, &common::t("panel-opt-no-document"));
        return;
    };
    let mut ns = s.clone();
    let mut custom_preset: Option<String> = None;
    // Optionsbereich bis unter die Kästchen „Zeilensprung/Progressiv“ (DLU 79 + 10) mit 2 px Luft;
    // vorher 128 px, die Kästchen reichten 19 px darüber hinaus (N23)
    let (area, _) = ui.allocate_exact_size(vec2(ui.available_width(), (2.0 + 89.0 * fields::DLU_Y + 2.0).ceil()), Sense::hover());
    let d = Dl::at(area.min + vec2(4.0, 2.0));
    // Gespeicherte Einstellungen (4749)
    let presets = builtin_presets();
    let names_s: Vec<String> = presets.iter().map(|p| preset_label(p.name)).collect();
    let names: Vec<&str> = names_s.iter().map(|s| s.as_str()).collect();
    let psel = preset_index(&s);
    let label = psel.map(|i| names[i].to_string()).unwrap_or_else(|| settings_label(&s));
    let resp = fields::combo_box_frame(ui, Id::new("opt-preset"), d.r(16.0, 2.0, 113.0, 12.0), &label, true);
    egui::Popup::menu(&resp).frame(crate::menu::menu_frame(&theme)).show(|ui| {
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        for (i, n) in names.iter().enumerate() {
            if fields::list_item(ui, n, Some(i) == psel).clicked() {
                ns = presets[i].settings.clone();
                ns.matte = s.matte;
                ui.close();
            }
        }
        // eigene, gespeicherte Einstellungen (Bedienfeldmenü „Einstellungen speichern“)
        let own = crate::ui_commands::export_presets(&st.session.prefs);
        if !own.is_empty() {
            ui.separator();
            for (n, _) in &own {
                if fields::list_item(ui, n, false).clicked() {
                    custom_preset = Some(n.clone());
                    ui.close();
                }
            }
        }
    });
    if let Some(n) = custom_preset.take() {
        st.run("panel.optimize.applySettings", json!({"name": n}));
        return;
    }
    ui.painter().hline(d.r(131.0, 8.0, 17.0, 1.0).x_range(), d.r(0.0, 8.0, 1.0, 1.0).top(), Stroke::new(1.0, c.line_mid));
    // Format (4610), Matte (4685)
    let fnames_s: Vec<String> = (0..FORMATS.len()).map(format_name).collect();
    let fnames: Vec<&str> = fnames_s.iter().map(|s| s.as_str()).collect();
    if let Some(i) = fields::combo(ui, "opt-format", d.r(2.0, 17.0, 71.0, 13.0), &fnames, format_index(&s), true)
        && let Some((f, m)) = FORMATS[i].1
    {
        ns.format = f;
        ns.color_mode = m;
        if f == ExportFormat::WebPLossless || (f == ExportFormat::AnimatedWebP) {
            ns.webp.lossless = f == ExportFormat::WebPLossless || ns.webp.lossless;
        }
    }
    common::label_right_fit(ui, d.r(74.0, 18.0, 39.0, 11.0), &common::t("panel-opt-matte"), true);
    let matte = s.matte.map(fp_color::Argb);
    match crate::inspector::popups::color_button(ui, "opt-matte", d.r(114.0, 17.0, 16.0, 13.0), st, matte, true, true, false) {
        Some(crate::inspector::popups::ColorChoice::Color(col)) => ns.matte = Some(col.0 | 0xff00_0000),
        Some(crate::inspector::popups::ColorChoice::None) => ns.matte = None,
        _ => {}
    }
    let indexed = ns.color_mode == ColorMode::Indexed;
    let jpeg = ns.format == ExportFormat::Jpeg;
    let lossy_modern = matches!(ns.format, ExportFormat::WebP | ExportFormat::Avif) || (ns.format == ExportFormat::AnimatedWebP && !ns.webp.lossless);
    if indexed {
        // Palette (4616), Farben (4602)
        let pnames_s: Vec<String> = (0..PALETTES.len()).map(palette_name).collect();
        let pnames: Vec<&str> = pnames_s.iter().map(|s| s.as_str()).collect();
        if let Some(i) = fields::combo(ui, "opt-palette", d.r(2.0, 32.0, 71.0, 13.0), &pnames, Some(palette_index(&s)), true) {
            let (_, k, web, cnt) = PALETTES[i];
            ns.palette_kind = k;
            ns.web_adaptive = web;
            if let Some(n) = cnt {
                ns.max_colors = n;
            }
        }
        common::label_right_fit(ui, d.r(75.0, 33.0, 37.0, 11.0), &common::t("panel-opt-colors"), true);
        let cnt_items: Vec<String> = COLOR_COUNTS.iter().map(|s| s.to_string()).collect();
        if let Some(v) = fields::combo_edit(ui, "opt-colors", d.r(113.0, 32.0, 33.0, 13.0), &s.max_colors.to_string(), &cnt_items, true)
            && let Some(n) = fields::parse_num(&v)
        {
            ns.max_colors = (n as i32).clamp(2, 256);
        }
        // Verlust (4677), Rastern (4676)
        let gif = matches!(ns.format, ExportFormat::Gif | ExportFormat::AnimatedGif);
        common::label_fit(ui, d.r(0.0, 48.0, 38.0, 11.0), &common::t("panel-opt-loss"), gif);
        if let Some(v) = fields::number(ui, "opt-lossy", d.r(39.0, 48.0, 24.0, 13.0), Some(d.r(63.0, 48.0, 9.0, 12.0)), Some(s.lossy as f64), (0.0, 100.0), 0, gif) {
            ns.lossy = v as i32;
        }
        common::label_right_fit(ui, d.r(73.0, 49.0, 40.0, 11.0), &common::t("panel-opt-dither"), true);
        let dp = match s.dither {
            Dither::Diffusion(p) => p,
            Dither::Pattern2x2 => 100,
            Dither::None => 0,
        };
        if let Some(v) = fields::number(ui, "opt-dither", d.r(113.0, 48.0, 23.0, 13.0), Some(d.r(137.0, 48.0, 9.0, 12.0)), Some(dp as f64), (0.0, 100.0), 0, true) {
            ns.dither = if v <= 0.0 { Dither::None } else { Dither::Diffusion(v as i32) };
        }
        // Transparenz (4603)
        let ti = match s.transparency {
            Transparency::None => 0,
            Transparency::Index => 1,
            _ => 2,
        };
        let tn: Vec<String> = TRANSPARENCY.iter().map(|k| common::t(k)).collect();
        let tn_refs: Vec<&str> = tn.iter().map(|s| s.as_str()).collect();
        if let Some(i) = fields::combo(ui, "opt-transp", d.r(2.0, 63.0, 136.0, 13.0), &tn_refs, Some(ti), true) {
            ns.transparency = match i {
                1 => Transparency::Index,
                2 => Transparency::IndexAlpha,
                _ => Transparency::None,
            };
        }
    } else if jpeg || lossy_modern {
        common::label_right_fit(ui, d.r(40.0, 34.0, 72.0, 11.0), &common::t("panel-opt-quality"), true);
        let q = if jpeg { s.jpeg.quality } else if ns.format == ExportFormat::Avif { s.avif.quality } else { s.webp.quality };
        if let Some(v) = fields::number(ui, "opt-quality", d.r(113.0, 32.0, 24.0, 13.0), Some(d.r(137.0, 32.0, 9.0, 12.0)), Some(q as f64), (1.0, 100.0), 0, true) {
            let v = v as i32;
            if jpeg {
                ns.jpeg.quality = v;
            } else if ns.format == ExportFormat::Avif {
                ns.avif.quality = v;
            } else {
                ns.webp.quality = v;
            }
        }
        if jpeg {
            common::label_right_fit(ui, d.r(0.0, 49.0, 112.0, 11.0), &common::t("panel-opt-selective-quality"), s.jpeg.selective.enabled);
            if let Some(v) = fields::number(ui, "opt-selq", d.r(113.0, 48.0, 18.0, 13.0), None, Some(s.jpeg.selective.quality as f64), (1.0, 100.0), 0, s.jpeg.selective.enabled) {
                ns.jpeg.selective.quality = v as i32;
            }
            common::label_right_fit(ui, d.r(0.0, 64.0, 112.0, 11.0), &common::t("panel-opt-smoothing"), true);
            let sm: Vec<String> = (0..=8).map(|i| i.to_string()).collect();
            let smr: Vec<&str> = sm.iter().map(|s| s.as_str()).collect();
            if let Some(i) = fields::combo(ui, "opt-smooth", d.r(114.0, 63.0, 32.0, 13.0), &smr, Some(s.jpeg.smoothing.clamp(0, 8) as usize), true) {
                ns.jpeg.smoothing = i as i32;
            }
        } else if ns.format == ExportFormat::Avif {
            common::label_right_fit(ui, d.r(0.0, 49.0, 112.0, 11.0), &common::t("panel-opt-speed"), true);
            if let Some(v) = fields::number(ui, "opt-speed", d.r(113.0, 48.0, 24.0, 13.0), Some(d.r(137.0, 48.0, 9.0, 12.0)), Some(s.avif.speed as f64), (1.0, 10.0), 0, true) {
                ns.avif.speed = v as u8;
            }
        }
    } else {
        // 24/32-Bit-Formate: Transparenz folgt der Farbtiefe
        let t = common::t(if ns.color_mode == ColorMode::Rgba32 { "panel-opt-transp-alpha" } else { "panel-opt-transp-none" });
        let _ = fields::combo(ui, "opt-transp-tc", d.r(2.0, 63.0, 136.0, 13.0), &[t.as_str()], Some(0), false);
    }
    // Optionen (Bedienfeldmenü 6941: Zeilensprung, progressives JPEG)
    if let Some(b) = common::checkbox_fit(ui, "opt-interlace", d.r(2.0, 79.0, 70.0, 10.0), &common::t("panel-opt-interlaced"), s.interlaced, matches!(ns.format, ExportFormat::Gif | ExportFormat::AnimatedGif | ExportFormat::Png)) {
        ns.interlaced = b;
    }
    if let Some(b) = common::checkbox_fit(ui, "opt-progressive", d.r(76.0, 79.0, 70.0, 10.0), &common::t("panel-opt-progressive"), s.jpeg.progressive, jpeg) {
        ns.jpeg.progressive = b;
    }
    if ns != s {
        apply_settings(st, &ns);
    }
    // Farbtabelle (4758) mit Auswahl, Markierungen (gesperrt, bearbeitet, transparent),
    // Sortierung und Anzahl; Kontextmenü 6942
    let pv = if indexed { preview(ui.ctx(), st, &s, false) } else { None };
    let sort_i = st.session.prefs.get_int("Firepixel_OptimizeSort", 0).clamp(0, 2) as usize;
    let model = st.session.active().and_then(|d| d.doc.pages.get(d.page)).and_then(|p| p.root.export.as_deref().cloned()).unwrap_or_default();
    let mut sel: Vec<u32> = st.dialogs.panel_sel.palette.clone();
    // Farbtabelle immer unter dem Optionsbereich (N23: bei JPEG/PNG 24/32/WebP/AVIF rückte der
    // Layout-Cursor nicht über die Kästchen hinaus und die Tabelle verdeckte ihre untere Hälfte)
    let avail = ui.available_rect_before_wrap();
    let top = avail.top().max(area.bottom()) + 2.0;
    let table = Rect::from_min_max(pos2(avail.left() + 2.0, top), pos2(avail.right() - 2.0, (avail.bottom() - 44.0).max(top + 30.0)));
    let inner = fields::list_box(ui, table);
    let tresp = ui.interact(inner, Id::new("opt-table"), Sense::click());
    if let Some(p) = &pv {
        let mut ents = p.entries.clone();
        if ents.is_empty() {
            ents = p.palette.iter().map(|c| fp_export::palette_edit::PaletteEntry { color: *c, shown: *c, count: 0, locked: false, transparent: false, edited: false, web_safe: false }).collect();
        }
        match sort_i {
            1 => ents.sort_by_key(|e| (((e.shown >> 16) & 0xff) * 299 + ((e.shown >> 8) & 0xff) * 587 + (e.shown & 0xff) * 114) as i64),
            2 => ents.sort_by_key(|e| std::cmp::Reverse(e.count)),
            _ => {}
        }
        let pal: Vec<u32> = ents.iter().map(|e| e.shown).collect();
        let cell = 14.0;
        let cols = ((inner.width() / cell).floor() as usize).max(1);
        for (i, col) in pal.iter().enumerate() {
            let r = Rect::from_min_size(pos2(inner.left() + 1.0 + (i % cols) as f32 * cell, inner.top() + 1.0 + (i / cols) as f32 * cell), vec2(cell - 1.0, cell - 1.0));
            if r.bottom() > inner.bottom() {
                break;
            }
            let a = (col >> 24) as u8;
            if a == 0 {
                crate::widgets::checker(ui.painter(), r, 3.0, &theme);
            } else {
                ui.painter().rect_filled(r, 0.0, Color32::from_rgb((col >> 16) as u8, (col >> 8) as u8, *col as u8));
            }
            let pe = &ents[i];
            let key = pe.color | 0xff00_0000;
            // Markierungen wie im Original: Quadrat unten rechts = gesperrt, Raute = bearbeitet,
            // Schachbrett-Ecke = transparent, Punkt = websicher
            {
                let e = pe;
                if e.web_safe {
                    ui.painter().circle_filled(r.center(), 1.5, Color32::from_gray(if (col >> 8 & 0xff) > 128 { 0 } else { 255 }));
                }
                if e.locked {
                    ui.painter().rect_filled(Rect::from_min_size(r.right_bottom() - vec2(4.0, 4.0), vec2(3.0, 3.0)), 0.0, Color32::WHITE);
                    ui.painter().rect_stroke(Rect::from_min_size(r.right_bottom() - vec2(4.0, 4.0), vec2(3.0, 3.0)), 0.0, Stroke::new(1.0, Color32::BLACK), StrokeKind::Inside);
                }
                if e.edited {
                    let m = r.left_bottom() + vec2(3.0, -3.0);
                    ui.painter().add(egui::Shape::convex_polygon(vec![m + vec2(0.0, -2.5), m + vec2(2.5, 0.0), m + vec2(0.0, 2.5), m + vec2(-2.5, 0.0)], Color32::WHITE, Stroke::new(0.5, Color32::BLACK)));
                }
                if e.transparent {
                    crate::widgets::checker(ui.painter(), Rect::from_min_size(r.min, vec2(6.0, 6.0)), 3.0, &theme);
                }
            }
            if sel.contains(&key) {
                ui.painter().rect_stroke(r.expand(1.0), 0.0, Stroke::new(2.0, c.focus), StrokeKind::Inside);
            }
            let hit = tresp.interact_pointer_pos().is_some_and(|pp| r.contains(pp));
            if hit && (tresp.clicked() || tresp.secondary_clicked()) {
                let m = ui.input(|i| i.modifiers);
                if m.command || m.shift {
                    if let Some(k) = sel.iter().position(|x| *x == key) {
                        sel.remove(k);
                    } else {
                        sel.push(key);
                    }
                } else if !(tresp.secondary_clicked() && sel.contains(&key)) {
                    sel = vec![key];
                }
            }
            if hit && tresp.double_clicked() {
                sel = vec![key];
                st.dialogs.panel_sel.palette = sel.clone();
                st.run("panel.optimize.editColor", json!({}));
            }
        }
        if let Some(e) = &p.error {
            ui.painter().text(inner.center(), Align2::CENTER_CENTER, e, theme.small_font(), c.text_disabled);
        }
    }
    st.dialogs.panel_sel.palette = sel.clone();
    let menu_nodes = common::resource_menu_nodes(st, "6942");
    let mut menu_actions = Vec::new();
    tresp.context_menu(|ui| {
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        for node in &menu_nodes {
            match node {
                crate::menu::Node::Item { label, action: crate::menu::ItemAction::Command(cmd), enabled, checked, .. } => {
                    if ui.add_enabled_ui(*enabled, |ui| fields::list_item(ui, label, *checked == Some(true))).inner.clicked() {
                        menu_actions.push(Action::Run(cmd.clone(), json!({})));
                        ui.close();
                    }
                }
                crate::menu::Node::Separator => {
                    ui.separator();
                }
                _ => {}
            }
        }
    });
    for a in menu_actions {
        st.push(a);
    }
    let y = table.bottom() + 4.0;
    common::fit_text(ui, Rect::from_min_size(pos2(avail.left() + 2.0, y), vec2(78.0, 18.0)), Align2::LEFT_CENTER, &common::t("panel-opt-sort-by"), theme.small_font(), c.text);
    let sorts = ["panel-opt-sort-none", "panel-opt-sort-luminance", "panel-opt-sort-popularity"].map(common::t);
    if let Some(i) = fields::combo(ui, "opt-sort", Rect::from_min_size(pos2(avail.left() + 82.0, y), vec2(60.0, 18.0)), &[sorts[0].as_str(), sorts[1].as_str(), sorts[2].as_str()], Some(sort_i), indexed) {
        st.run(["panel.optimize.sortNone", "panel.optimize.sortLuminance", "panel.optimize.sortPopularity"][i.min(2)], json!({}));
    }
    let count = pv.as_ref().map(|p| p.colors).unwrap_or(0);
    let count_r = Rect::from_min_max(pos2(avail.left() + 146.0, y), pos2(avail.right() - 4.0, y + 18.0));
    common::fit_text(ui, count_r, Align2::RIGHT_CENTER, &fp_i18n::tr!("panel-opt-color-count", n = count), theme.small_font(), c.text);
    // Knopfleiste (4630–4699): Transparenz, Bearbeiten, Websicher, Sperren, Hinzufügen, Löschen
    // bei niedriger Gruppe nicht über die Sortierzeile legen (Inhalt wird dann am Gruppenrand abgeschnitten)
    let by = (avail.bottom() - 20.0).max(y + 22.0);
    let has_sel = !sel.is_empty() && indexed;
    let colors: Vec<String> = sel.iter().map(|c| format!("#{:06x}", c & 0x00ff_ffff)).collect();
    let glyphs: [(crate::inspector::glyphs::Glyph, &str, bool, &str, serde_json::Value); 8] = [
        (crate::inspector::glyphs::Glyph::Plus, "panel-opt-add-transparent", has_sel, "panel.optimize.makeTransparent", json!({"colors": colors, "on": true})),
        (crate::inspector::glyphs::Glyph::Minus, "panel-opt-remove-transparent", has_sel, "panel.optimize.makeTransparent", json!({"colors": colors, "on": false})),
        (crate::inspector::glyphs::Glyph::Eyedropper, "panel-opt-select-transparent", has_sel, "history.replay", json!({"commands": [{"command": "panel.optimize.makeTransparent", "params": {"colors": model.palette.iter().filter(|e| e.transparent == Some(true)).map(|e| format!("#{:06x}", e.color & 0xff_ffff)).collect::<Vec<_>>(), "on": false}}, {"command": "panel.optimize.makeTransparent", "params": {"colors": colors, "on": true}}], "label": common::t("panel-opt-select-transparent-label")})),
        (crate::inspector::glyphs::Glyph::Gear, "panel-opt-edit-color", has_sel, "panel.optimize.editColor", json!({})),
        (crate::inspector::glyphs::Glyph::Picker, "panel-opt-snap-web", has_sel, "panel.optimize.snapToWeb", json!({"colors": colors})),
        (crate::inspector::glyphs::Glyph::Lock, "panel-opt-lock-color", has_sel, "panel.optimize.lockColors", json!({"colors": colors})),
        (crate::inspector::glyphs::Glyph::StyleNew, "panel-opt-add-color", indexed, "panel.optimize.addColorMenu", json!({})),
        (crate::inspector::glyphs::Glyph::Trash, "panel-opt-delete-color", has_sel, "panel.optimize.deleteColors", json!({"colors": colors})),
    ];
    for (i, (g, tip, en, cmd, params)) in glyphs.iter().enumerate() {
        let x = avail.left() + 4.0 + i as f32 * 21.0 + if i >= 3 { 12.0 } else { 0.0 } + if i >= 6 { 12.0 } else { 0.0 };
        if fields::icon_button(ui, ("opt-btn", i), Rect::from_min_size(pos2(x, by), vec2(19.0, 16.0)), *g, *en, &common::t(tip)).clicked() && *en {
            st.run(cmd, params.clone());
        }
    }
    let _ = ui.allocate_rect(Rect::from_min_max(avail.min, pos2(avail.right(), avail.bottom())), Sense::hover());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_roundtrip() {
        for (i, p) in builtin_presets().iter().enumerate() {
            assert_eq!(preset_index(&p.settings), Some(i), "{}", p.name);
            assert!(format_index(&p.settings).is_some(), "{}", p.name);
        }
        let s = OptimizeSettings { palette_kind: PaletteKind::Grayscale, max_colors: 2, ..Default::default() };
        assert_eq!(PALETTES[palette_index(&s)].0, "Schwarzweiß");
        // jeder Vorgabename hat einen Schlüssel (sonst bliebe er in jeder Sprache deutsch)
        for p in builtin_presets() {
            assert!(PRESET_KEYS.iter().any(|(de, _)| *de == p.name), "Vorgabe ohne Schlüssel: {}", p.name);
        }
    }
}
