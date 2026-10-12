//! Exportieren und Bildvorschau über fp-export (in fp-engine noch „nicht umgesetzt“):
//! - **Exportieren** (Datei ▸ Exportieren…): Exportart (`exportFilesStyle`: Nur Bilder, HTML und
//!   Bilder, CSS und Bilder, CSS-Sprites, Seiten/Status/Ebenen in Dateien, Photoshop PSD, SVG),
//!   Segmente, Seite, Basisname; Zielordner über den Windows-Dateidialog.
//! - **Bildvorschau** (4716, Datei ▸ Bildvorschau…): Optionen (Format, Palette, Farben, Qualität,
//!   Matte …), Datei (Skalierung), Vorschaubild mit Größe und Ladezeit, „Exportieren…“ und OK
//!   (übernimmt die Einstellungen in die Seite).

use super::{dialog_frame, dialog_frame_resizable, push_button, refs, t, DialogClose};
use crate::app_state::{Action, AppState};
use crate::inspector::fields;
use crate::panels::optimize::{self, ExportRenderer, FORMATS, PALETTES};
use crate::theme::Theme;
use egui::{pos2, vec2, Align2, Color32, ColorImage, Id, Rect, Sense, Stroke, StrokeKind, TextureOptions};
use fp_export::doc_export::{export_document, DocExportOptions, FilesStyle};
use fp_export::options::{ColorMode, ExportFormat, OptimizeSettings};
use std::sync::Arc;

/// Exportarten: Anzeigename, Art.
/// (Textschlüssel der Anzeigenamen.)
pub const STYLES: &[(&str, FilesStyle)] = &[
    ("dlg-export-style-images", FilesStyle::ImagesOnly),
    ("dlg-export-style-html", FilesStyle::HtmlAndImages),
    ("dlg-export-style-css", FilesStyle::CssAndImages),
    ("dlg-export-style-sprites", FilesStyle::CssSprites),
    ("dlg-export-style-pages", FilesStyle::PagesToFiles),
    ("dlg-export-style-states", FilesStyle::FramesToFiles),
    ("dlg-export-style-layers", FilesStyle::LayersToFiles),
    ("dlg-export-style-psd", FilesStyle::Psd),
    ("dlg-export-style-svg", FilesStyle::Svg),
];

/// Zustand „Exportieren“.
#[derive(Clone, Debug, PartialEq)]
pub struct ExportDialog {
    pub style: usize,
    pub use_slices: bool,
    pub base_name: String,
    pub message: Option<String>,
    /// Exportbereich (Werkzeug „Exportbereich“, Dokumentkoordinaten): exportiert nur diesen
    /// Ausschnitt über `file.exportArea`.
    pub area: Option<fp_geom::Rect>,
}

impl ExportDialog {
    /// Vorgaben aus dem Dokument.
    pub fn new(st: &AppState) -> Option<ExportDialog> {
        let d = st.session.active()?;
        let base = d.title.trim_end_matches(".png").trim_end_matches(".fw").to_string();
        let has_slices = d.doc.pages.get(d.page).is_some_and(|p| p.root.layers.iter().any(|s| matches!(s, fp_model::LayerSlot::Own(l) if l.kind == fp_model::LayerKind::Web && l.cels.iter().any(|c| !c.elements.is_empty()))));
        Some(ExportDialog { style: 0, use_slices: has_slices, base_name: base, message: None, area: None })
    }
}

/// Führt den Export aus (Zielordner `dir`); liefert die Anzahl geschriebener Dateien.
pub fn run_export(st: &AppState, dlg: &ExportDialog, dir: &std::path::Path) -> Result<usize, String> {
    let d = st.session.active().ok_or_else(|| t("dlg-export-no-document"))?;
    let mut o = DocExportOptions::new(&dlg.base_name);
    o.page = d.page;
    o.style = STYLES.get(dlg.style).map(|s| s.1).unwrap_or(FilesStyle::ImagesOnly);
    o.use_slices = dlg.use_slices;
    o.src_file = d.path.as_ref().map(|p| p.display().to_string()).unwrap_or_default();
    let r = ExportRenderer { normal: Arc::clone(&st.renderer) };
    let files = export_document(&d.doc, &r, &o, &fp_export::export::EncodeExtras::default()).map_err(|e| e.to_string())?;
    for f in &files {
        let p = dir.join(&f.path);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&p, &f.bytes).map_err(|e| format!("{}: {e}", p.display()))?;
    }
    Ok(files.len())
}

/// Zeigt „Exportieren“.
pub fn show_export(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut d) = st.dialogs.export.clone() else { return };
    let mut result = None;
    let title = if d.area.is_some() { t("dlg-export-area-title") } else { t("dlg-export-title") };
    let names: Vec<String> = STYLES.iter().map(|s| t(s.0)).collect();
    let (style_l, file_l, slices_l, options_l, export_l, cancel_l) = (t("dlg-export-style"), t("dlg-export-file-name"), t("dlg-export-slices"), t("dlg-export-options"), t("dlg-export-button"), t("btn-cancel"));
    let pset = st.session.active().map(|x| fp_export::doc_export::page_settings(&x.doc, x.page));
    let preset = pset.as_ref().map(|s| optimize::preset_index(s).map(|i| optimize::preset_label(&fp_export::options::builtin_presets()[i].name)).unwrap_or_else(|| optimize::settings_label(s))).unwrap_or_default();
    let opt_text = fp_i18n::tr!("dlg-export-optimize", preset = preset.as_str());
    // Spalten wachsen mit den Texten (Deutsch: wie bisher)
    let dx = super::extra_w(ctx, &[&style_l, &file_l], 110.0 - 4.0);
    let cw = super::combo_fit_w(ctx, &refs(&names), 200.0);
    let ow = super::button_w(ctx, &options_l, 110.0);
    let ox = 240f32.max((super::text_w(ctx, &opt_text) + 12.0).ceil());
    let bw = super::button_w(ctx, &export_l, 96.0);
    let cbw = super::button_w(ctx, &cancel_l, 76.0);
    let width = 440f32
        .max(32.0 + 120.0 + dx + cw.max(18.0 + super::text_w(ctx, &slices_l)))
        .max(32.0 + ox + ow)
        .max(16.0 + bw + 8.0 + cbw + 16.0)
        .ceil();
    let r = dialog_frame(ctx, "fp-export", &title, vec2(width, 230.0), |ui, body| {
        let theme = Theme::current(ui.ctx());
        let o = body.min + vec2(16.0, 16.0);
        super::fit_label(ui, Rect::from_min_size(o, vec2(110.0 + dx, 20.0)), &style_l, true);
        if let Some(i) = fields::combo(ui, "ex-style", Rect::from_min_size(o + vec2(120.0 + dx, 0.0), vec2(cw, 20.0)), &refs(&names), Some(d.style), true) {
            d.style = i;
        }
        super::fit_label(ui, Rect::from_min_size(o + vec2(0.0, 32.0), vec2(110.0 + dx, 20.0)), &file_l, true);
        if let Some(t) = fields::text(ui, "ex-name", Rect::from_min_size(o + vec2(120.0 + dx, 32.0), vec2(200.0, 20.0)), &d.base_name, true) {
            d.base_name = t;
        }
        let slices_ok = matches!(STYLES[d.style].1, FilesStyle::ImagesOnly | FilesStyle::HtmlAndImages | FilesStyle::CssAndImages);
        if let Some(b) = fields::checkbox(ui, "ex-slices", Rect::from_min_size(o + vec2(120.0 + dx, 64.0), vec2(cw.max(18.0 + super::text_w(ui.ctx(), &slices_l)), 16.0)), &slices_l, d.use_slices, slices_ok) {
            d.use_slices = b;
        }
        ui.painter().text(o + vec2(0.0, 98.0), Align2::LEFT_TOP, opt_text.as_str(), theme.ui_font(), theme.colors.text_weak);
        if fields::button(ui, "ex-options", Rect::from_min_size(o + vec2(ox, 94.0), vec2(ow, 22.0)), &options_l, true) {
            st.push(Action::Run("file.imagePreview".into(), serde_json::json!({})));
        }
        if let Some(a) = d.area {
            let area = fp_i18n::tr!("dlg-export-area", x = format!("{:.0}", a.left), y = format!("{:.0}", a.top), w = format!("{:.0}", a.width()), h = format!("{:.0}", a.height()));
            super::put_left(ui, Rect::from_min_size(o + vec2(0.0, 116.0), vec2(body.width() - 32.0, 18.0)), egui::Label::new(egui::RichText::new(area).color(theme.colors.text)).wrap());
        }
        if let Some(m) = &d.message {
            super::put_left(ui, Rect::from_min_size(o + vec2(0.0, 134.0), vec2(body.width() - 32.0, 36.0)), egui::Label::new(egui::RichText::new(m).color(theme.colors.text_weak)).wrap());
        }
        let by = body.bottom() - 36.0;
        if push_button(ui, Rect::from_min_size(pos2(body.right() - 16.0 - cbw - 8.0 - bw, by), vec2(bw, 23.0)), &export_l, true, !d.base_name.trim().is_empty()) {
            result = Some(true);
        }
        if push_button(ui, Rect::from_min_size(pos2(body.right() - 16.0 - cbw, by), vec2(cbw, 23.0)), &cancel_l, false, true) {
            result = Some(false);
        }
    });
    match (result, r) {
        (Some(true), _) if d.area.is_some() => {
            let a = d.area.unwrap_or_default();
            let ext = st.session.active().map(|x| fp_export::doc_export::page_settings(&x.doc, x.page)).map(|s| s.format.extension().to_string()).unwrap_or_else(|| "png".into());
            let image_l = t("dlg-filter-image");
            let path = super::panel_menus::pick_save(st, &t("dlg-export-area-title"), &format!("{}.{ext}", d.base_name), &[(image_l.as_str(), &["png", "gif", "jpg", "webp", "avif", "bmp", "tif"])]);
            match path {
                Some(p) => match crate::ui_commands::execute(&mut st.session, "file.exportArea", serde_json::json!({"rect": [a.left, a.top, a.right, a.bottom], "path": p.to_string_lossy()})) {
                    Ok(_) => {
                        st.dialogs.export = None;
                        st.push(Action::Status(fp_i18n::tr!("dlg-export-area-done", path = p.display().to_string())));
                    }
                    Err(e) => {
                        d.message = Some(fp_i18n::tr!("dlg-export-failed", error = e.to_string()));
                        st.dialogs.export = Some(d);
                    }
                },
                None => st.dialogs.export = Some(d),
            }
        }
        (Some(true), _) => {
            let dir = match &st.file_override {
                Some(o) => o.save.as_ref().and_then(|p| p.parent().map(|x| x.to_path_buf())),
                None => rfd::FileDialog::new().set_title(t("dlg-export-pick-folder")).pick_folder(),
            };
            match dir {
                Some(dir) => match run_export(st, &d, &dir) {
                    Ok(n) => {
                        st.dialogs.export = None;
                        st.push(Action::Status(fp_i18n::tr!("dlg-export-done", n = n, path = dir.display().to_string())));
                    }
                    Err(e) => {
                        d.message = Some(fp_i18n::tr!("dlg-export-failed", error = e));
                        st.dialogs.export = Some(d);
                    }
                },
                None => st.dialogs.export = Some(d),
            }
        }
        (Some(false), _) | (_, DialogClose::Cancelled) => st.dialogs.export = None,
        _ => st.dialogs.export = Some(d),
    }
}

// ---------------------------------------------------------------------- Bildvorschau

/// Zustand der Bildvorschau.
#[derive(Clone, Debug, PartialEq)]
pub struct PreviewDialog {
    pub settings: OptimizeSettings,
    pub tab: usize,
    pub scale: f64,
    pub zoom: usize,
}

/// Zoomstufen (Prozentangaben; „Einpassen“ als Textschlüssel).
const ZOOMS: [(&str, f32); 7] = [("25%", 0.25), ("50%", 0.5), ("100%", 1.0), ("200%", 2.0), ("400%", 4.0), ("800%", 8.0), ("dlg-preview-fit", 0.0)];

impl PreviewDialog {
    /// Einstellungen der Seite.
    pub fn new(st: &AppState) -> Option<PreviewDialog> {
        let d = st.session.active()?;
        Some(PreviewDialog { settings: fp_export::doc_export::page_settings(&d.doc, d.page), tab: 0, scale: 100.0, zoom: 6 })
    }
}

/// Zeigt die Bildvorschau.
pub fn show_preview(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut d) = st.dialogs.preview.clone() else { return };
    let mut result: Option<u8> = None;
    let tabs = super::tl(&["dlg-preview-tab-options", "dlg-preview-tab-file", "dlg-preview-tab-animation"]);
    let labs = super::tl(&["dlg-preview-format", "dlg-preview-palette", "dlg-preview-colors", "dlg-preview-loss", "dlg-lbl-quality", "dlg-preview-matte", "dlg-preview-scale"]);
    let fnames: Vec<String> = (0..FORMATS.len()).map(optimize::format_name).collect();
    let pnames: Vec<String> = (0..PALETTES.len()).map(optimize::palette_name).collect();
    let interl_l = t("dlg-preview-interlaced");
    let (pexport_l, ok_l, cancel_l) = (t("dlg-export-button"), t("btn-ok"), t("btn-cancel"));
    // linke Spalte wächst mit Reitern, Beschriftungen und Listen (Deutsch: 250 px wie bisher)
    let tab_w = super::buttons_w(ctx, &refs(&tabs), 80.0);
    let dx = super::extra_w(ctx, &refs(&labs), 70.0 - 2.0);
    let cw = super::combo_fit_w(ctx, &refs(&fnames), 160.0).max(super::combo_fit_w(ctx, &refs(&pnames), 160.0));
    let left_w = 250f32.max(3.0 * tab_w + 6.0).max(76.0 + dx + cw + 4.0).max(18.0 + super::text_w(ctx, &interl_l)).ceil();
    let bw_ex = super::button_w(ctx, &pexport_l, 100.0);
    let row = super::button_row_w(ctx, &[&ok_l, &cancel_l], 76.0, 8.0);
    let min_w = 820f32.max(10.0 + left_w + 12.0 + 100.0 + 8.0 + bw_ex + 14.0 + row + 16.0).ceil();
    let r = dialog_frame_resizable(ctx, "fp-imgpreview", &t("dlg-preview-title"), vec2(min_w, 520.0), |ui, body| {
        let theme = Theme::current(ui.ctx());
        let c = &theme.colors;
        let left = Rect::from_min_size(body.min + vec2(10.0, 10.0), vec2(left_w, body.height() - 60.0));
        let lw = 70.0 + dx;
        let cx = left.left() + 76.0 + dx;
        // Registerkarten Optionen / Datei / Animation
        for (i, t) in tabs.iter().enumerate() {
            let tr = Rect::from_min_size(left.min + vec2(i as f32 * (tab_w + 2.0), 0.0), vec2(tab_w, 20.0));
            let resp = ui.interact(tr, Id::new(("pv-tab", i)), Sense::click());
            ui.painter().rect_filled(tr, 0.0, if d.tab == i { c.tab_active_bg } else { c.tab_inactive_bg });
            ui.painter().rect_stroke(tr, 0.0, Stroke::new(1.0, c.line_mid), StrokeKind::Inside);
            ui.painter().text(tr.center(), Align2::CENTER_CENTER, t.as_str(), theme.ui_font(), c.text);
            if resp.clicked() {
                d.tab = i;
            }
        }
        let y0 = left.top() + 30.0;
        let s = &mut d.settings;
        match d.tab {
            0 => {
                super::fit_label(ui, Rect::from_min_size(pos2(left.left(), y0), vec2(lw, 20.0)), &labs[0], true);
                if let Some(i) = fields::combo(ui, "pv-format", Rect::from_min_size(pos2(cx, y0), vec2(cw, 20.0)), &refs(&fnames), optimize::format_index(s), true)
                    && let Some((f, m)) = FORMATS[i].1
                {
                    s.format = f;
                    s.color_mode = m;
                }
                let mut y = y0 + 28.0;
                if s.color_mode == ColorMode::Indexed {
                    super::fit_label(ui, Rect::from_min_size(pos2(left.left(), y), vec2(lw, 20.0)), &labs[1], true);
                    if let Some(i) = fields::combo(ui, "pv-pal", Rect::from_min_size(pos2(cx, y), vec2(cw, 20.0)), &refs(&pnames), Some(optimize::palette_index(s)), true) {
                        let (_, k, web, cnt) = PALETTES[i];
                        s.palette_kind = k;
                        s.web_adaptive = web;
                        if let Some(n) = cnt {
                            s.max_colors = n;
                        }
                    }
                    y += 28.0;
                    super::fit_label(ui, Rect::from_min_size(pos2(left.left(), y), vec2(lw, 20.0)), &labs[2], true);
                    if let Some(v) = fields::number(ui, "pv-colors", Rect::from_min_size(pos2(cx, y), vec2(50.0, 20.0)), Some(Rect::from_min_size(pos2(cx + 52.0, y), vec2(14.0, 20.0))), Some(s.max_colors as f64), (2.0, 256.0), 0, true) {
                        s.max_colors = v as i32;
                    }
                    y += 28.0;
                    super::fit_label(ui, Rect::from_min_size(pos2(left.left(), y), vec2(lw, 20.0)), &labs[3], true);
                    if let Some(v) = fields::number(ui, "pv-lossy", Rect::from_min_size(pos2(cx, y), vec2(50.0, 20.0)), None, Some(s.lossy as f64), (0.0, 100.0), 0, matches!(s.format, ExportFormat::Gif | ExportFormat::AnimatedGif)) {
                        s.lossy = v as i32;
                    }
                    y += 28.0;
                } else if matches!(s.format, ExportFormat::Jpeg | ExportFormat::WebP | ExportFormat::Avif) {
                    super::fit_label(ui, Rect::from_min_size(pos2(left.left(), y), vec2(lw, 20.0)), &labs[4], true);
                    let q = match s.format {
                        ExportFormat::Jpeg => s.jpeg.quality,
                        ExportFormat::Avif => s.avif.quality,
                        _ => s.webp.quality,
                    };
                    if let Some(v) = fields::number(ui, "pv-q", Rect::from_min_size(pos2(cx, y), vec2(50.0, 20.0)), Some(Rect::from_min_size(pos2(cx + 52.0, y), vec2(14.0, 20.0))), Some(q as f64), (1.0, 100.0), 0, true) {
                        match s.format {
                            ExportFormat::Jpeg => s.jpeg.quality = v as i32,
                            ExportFormat::Avif => s.avif.quality = v as i32,
                            _ => s.webp.quality = v as i32,
                        }
                    }
                    y += 28.0;
                }
                super::fit_label(ui, Rect::from_min_size(pos2(left.left(), y), vec2(lw, 20.0)), &labs[5], true);
                match crate::inspector::popups::color_button(ui, "pv-matte", Rect::from_min_size(pos2(cx, y), vec2(26.0, 20.0)), st, s.matte.map(fp_color::Argb), true, true, false) {
                    Some(crate::inspector::popups::ColorChoice::Color(col)) => s.matte = Some(col.0 | 0xff00_0000),
                    Some(crate::inspector::popups::ColorChoice::None) => s.matte = None,
                    _ => {}
                }
                y += 28.0;
                if let Some(b) = fields::checkbox(ui, "pv-interl", Rect::from_min_size(pos2(left.left(), y), vec2(left.width(), 16.0)), &interl_l, s.interlaced || s.jpeg.progressive, true) {
                    s.interlaced = b;
                    s.jpeg.progressive = b;
                }
            }
            1 => {
                super::fit_label(ui, Rect::from_min_size(pos2(left.left(), y0), vec2(lw, 20.0)), &labs[6], true);
                if let Some(v) = fields::number(ui, "pv-scale", Rect::from_min_size(pos2(cx, y0), vec2(50.0, 20.0)), Some(Rect::from_min_size(pos2(cx + 52.0, y0), vec2(14.0, 20.0))), Some(d.scale), (1.0, 1000.0), 0, true) {
                    d.scale = v;
                    let sc = fp_model::ExportScale { use_scale: (v - 100.0).abs() > 0.01, constrain: true, percent: v.round() as i32, width: 0, height: 0 };
                    d.settings.scale = Some(sc);
                }
                ui.painter().text(pos2(left.left(), y0 + 30.0), Align2::LEFT_TOP, "%", theme.ui_font(), c.text);
            }
            _ => {
                let frames = st.session.active().map(|x| x.frame_count()).unwrap_or(1);
                ui.painter().text(pos2(left.left(), y0), Align2::LEFT_TOP, fp_i18n::tr!("dlg-preview-states", n = frames), theme.ui_font(), c.text);
                ui.painter().text(pos2(left.left(), y0 + 20.0), Align2::LEFT_TOP, fp_i18n::tr!("dlg-preview-loop", looping = crate::panels::states::loop_text(st.session.active().and_then(|x| x.doc.pages.get(x.page)).map(|p| p.loop_count()).unwrap_or(0))), theme.ui_font(), c.text);
            }
        }
        // Vorschaufläche
        let pr = Rect::from_min_max(pos2(left.right() + 12.0, body.top() + 34.0), pos2(body.right() - 10.0, body.bottom() - 54.0));
        crate::widgets::checker(ui.painter(), pr, 8.0, &theme);
        let pv = optimize::preview(ui.ctx(), st, &d.settings, true);
        if let Some(p) = &pv {
            if let Some(img) = &p.image {
                let key = Id::new("pv-texture");
                let tkey = format!("{:?}-{:?}-{}", d.settings, st.session.active().map(|x| (x.id.0, x.generation, x.page, x.frame)), p.bytes);
                let cached: Option<(String, egui::TextureHandle)> = ui.data(|dd| dd.get_temp(key));
                let tex = match cached {
                    Some((k, t)) if k == tkey => t,
                    _ => {
                        let t = ctx.load_texture("img-preview", ColorImage::from_rgba_unmultiplied([img.width(), img.height()], img.as_bytes()), TextureOptions::NEAREST);
                        ui.data_mut(|dd| dd.insert_temp(key, (tkey, t.clone())));
                        t
                    }
                };
                let ts = tex.size_vec2();
                let z = ZOOMS[d.zoom].1;
                let k = if z == 0.0 { (pr.width() / ts.x).min(pr.height() / ts.y).min(1.0) } else { z };
                let ir = Rect::from_center_size(pr.center(), ts * k);
                let clip = ui.clip_rect();
                ui.set_clip_rect(pr.intersect(clip));
                ui.painter().image(tex.id(), ir, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                ui.set_clip_rect(clip);
            }
            let info = if let Some(e) = &p.error {
                e.clone()
            } else {
                let fmt = FORMATS.iter().position(|f| f.1 == Some((d.settings.format, d.settings.color_mode))).map(optimize::format_name).unwrap_or_default();
                let secs = fp_export::preview::download_seconds(p.bytes, 56_000);
                fp_i18n::tr!("dlg-preview-info", format = fmt, size = fp_export::preview::format_size(p.bytes), secs = secs.to_string(), colors = p.colors as i64)
            };
            ui.painter().text(pos2(pr.left(), body.top() + 16.0), Align2::LEFT_CENTER, info, theme.ui_font(), c.text);
        }
        ui.painter().rect_stroke(pr, 0.0, Stroke::new(1.0, c.list_border), StrokeKind::Inside);
        let zn: Vec<String> = ZOOMS.iter().map(|z| if z.0.starts_with("dlg-") { t(z.0) } else { z.0.to_string() }).collect();
        if let Some(z) = fields::combo(ui, "pv-zoom", Rect::from_min_size(pos2(pr.left(), body.bottom() - 44.0), vec2(super::combo_fit_w(ui.ctx(), &refs(&zn), 100.0), 20.0)), &refs(&zn), Some(d.zoom), true) {
            d.zoom = z;
        }
        let b = super::button_row(ui.ctx(), body.right() - 16.0, body.bottom() - 36.0, &[&ok_l, &cancel_l], 76.0, 23.0, 8.0);
        if push_button(ui, Rect::from_min_size(pos2(b[0].left() - 14.0 - bw_ex, b[0].top()), vec2(bw_ex, 23.0)), &pexport_l, false, true) {
            result = Some(2);
        }
        if push_button(ui, b[0], &ok_l, true, true) {
            result = Some(1);
        }
        if push_button(ui, b[1], &cancel_l, false, true) {
            result = Some(0);
        }
    });
    match (result, r) {
        (Some(1), _) => {
            st.dialogs.preview = None;
            optimize::apply_settings(st, &d.settings);
        }
        (Some(2), _) => {
            st.dialogs.preview = None;
            optimize::apply_settings(st, &d.settings);
            let ext = d.settings.format.extension();
            let name = st.session.active().map(|x| x.title.trim_end_matches(".png").trim_end_matches(".fw").to_string()).unwrap_or_else(|| t("dlg-default-image-name"));
            if let Some(path) = rfd::FileDialog::new().set_title(t("dlg-export-title")).set_file_name(format!("{name}.{ext}")).add_filter(d.settings.format.display_name(), &[ext]).save_file()
                && let Some(doc) = st.session.active()
            {
                let r = ExportRenderer { normal: Arc::clone(&st.renderer) };
                match fp_export::doc_export::export_whole(&r, &doc.doc, doc.page, &d.settings, &fp_export::export::EncodeExtras::default()) {
                    Ok(bytes) => match std::fs::write(&path, bytes) {
                        Ok(()) => st.push(Action::Status(fp_i18n::tr!("dlg-export-file-done", path = path.display().to_string()))),
                        Err(e) => st.push(Action::Status(fp_i18n::tr!("dlg-export-failed", error = e.to_string()))),
                    },
                    Err(e) => st.push(Action::Status(fp_i18n::tr!("dlg-export-failed", error = e.to_string()))),
                }
            }
        }
        (Some(_), _) | (_, DialogClose::Cancelled) => st.dialogs.preview = None,
        _ => st.dialogs.preview = Some(d),
    }
}
