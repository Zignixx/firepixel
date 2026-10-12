//! Exportieren und Bildvorschau über fp-export (in fp-engine noch „nicht umgesetzt“):
//! - **Exportieren** (Datei ▸ Exportieren…): Exportart (`exportFilesStyle`: Nur Bilder, HTML und
//!   Bilder, CSS und Bilder, CSS-Sprites, Seiten/Status/Ebenen in Dateien, Photoshop PSD, SVG),
//!   Segmente, Seite, Basisname; Zielordner über den Windows-Dateidialog.
//! - **Bildvorschau**: eigenes Modul [`super::image_preview`] (hier nur die alten Namen).

use super::{dialog_frame, push_button, refs, t, DialogClose};
use crate::app_state::{Action, AppState};
use crate::inspector::fields;
use crate::panels::optimize::{self, ExportRenderer};
use crate::theme::Theme;
use egui::{pos2, vec2, Align2, Rect};
use fp_export::doc_export::{export_document, DocExportOptions, FilesStyle};
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
    let preset = pset.as_ref().map(|s| optimize::preset_index(s).map(|i| optimize::preset_label(fp_export::options::builtin_presets()[i].name)).unwrap_or_else(|| optimize::settings_label(s))).unwrap_or_default();
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
            // Dateidialog ohne Blockieren: der Exportdialog wartet ausgeblendet und kommt bei Abbruch
            // bzw. Fehler zurück
            st.dialogs.export = None;
            let req = crate::file_dialog::Request::save(t("dlg-export-area-title"), format!("{}.{ext}", d.base_name)).filter(image_l, &["png", "gif", "jpg", "webp", "avif", "bmp", "tif"]);
            crate::file_dialog::request(st, req, move |st, r| match r.into_iter().next() {
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
            });
        }
        (Some(true), _) => {
            st.dialogs.export = None;
            crate::file_dialog::request(st, crate::file_dialog::Request::folder(t("dlg-export-pick-folder")), move |st, r| match r.into_iter().next() {
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
            });
        }
        (Some(false), _) | (_, DialogClose::Cancelled) => st.dialogs.export = None,
        _ => st.dialogs.export = Some(d),
    }
}

// ---------------------------------------------------------------------- Bildvorschau

// Die Bildvorschau hat ein eigenes Modul (`dialogs::image_preview`, Spezifikation 17-bildvorschau.md);
// die alten Namen bleiben als schmale Weiterleitung.
pub use super::image_preview::ImagePreview as PreviewDialog;

/// Zeigt die Bildvorschau (Weiterleitung auf [`super::image_preview::show`]).
pub fn show_preview(ctx: &egui::Context, st: &mut AppState) {
    super::image_preview::show(ctx, st);
}
