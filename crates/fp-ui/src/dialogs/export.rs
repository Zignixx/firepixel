//! **Exportieren** (Datei ▸ Exportieren…, re\18-export-dialoge.md §1–§5, §7): ein Ablauf mit
//! Ordner, Dateiname, Exportart und dem Unterpaneel der Art (Grau-/Sichtbarkeitsregeln §2), der
//! Endung je Art (§1.4), den gemessenen Dateinamen (§3, Schreiber in `fp_export::doc_export`), der
//! Rückfrage bei vorhandenen Dateien (§5.1) und dem Merken von Ordner und Art (§7). Kein
//! Verlaufsschritt (§5.3).
//! - „Optionen…“ öffnet je Art den eigenen Optionen-Dialog ([`super::export_options`], Paket E2);
//!   bei den HTML-Arten vorerst die Bildvorschau.
//! - Mit Exportbereich (Werkzeug „Exportbereich“) bleibt der kurze Ablauf mit Speichern-Dialog.
//! - **Bildvorschau**: eigenes Modul [`super::image_preview`] (hier nur die alten Namen).

use super::export_options::{self, FilesOpts, OptionsDialog, OptionsTarget, PdfOpts, PsdOpts, SpriteOpts};
use super::{dialog_frame, push_button, refs, t, DialogClose};
use crate::app_state::{Action, AppState};
use crate::inspector::fields;
use crate::panels::optimize::{self, ExportRenderer};
use crate::theme::Theme;
use egui::{pos2, vec2, Align2, Rect};
use fp_export::doc_export::{export_document_noted, DocExportOptions, ExportFile, ExportNotes, FilesStyle, PageScope, PanelOptions, SliceMode};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Exportarten in der Reihenfolge der Liste (re\18 §1.3 ohne markengebundene Arten; Photoshop PSD
/// und SVG statt FXG nach Entscheid des Nutzers). (Textschlüssel, Art.)
pub const STYLES: &[(&str, FilesStyle)] = &[
    ("dlg-export-style-pdf", FilesStyle::Pdf),
    ("dlg-export-style-html", FilesStyle::HtmlAndImages),
    ("dlg-export-style-images", FilesStyle::ImagesOnly),
    ("dlg-export-style-css", FilesStyle::CssAndImages),
    ("dlg-export-style-sprites", FilesStyle::CssSprites),
    ("dlg-export-style-layers", FilesStyle::LayersToFiles),
    ("dlg-export-style-states", FilesStyle::FramesToFiles),
    ("dlg-export-style-pages", FilesStyle::PagesToFiles),
    ("dlg-export-style-psd", FilesStyle::Psd),
    ("dlg-export-style-svg", FilesStyle::Svg),
];

/// Listenplatz einer Art.
pub fn style_index(s: FilesStyle) -> usize {
    STYLES.iter().position(|x| x.1 == s).unwrap_or(1)
}

/// Zustand „Exportieren“.
#[derive(Clone, Debug, PartialEq)]
pub struct ExportDialog {
    /// Exportart (Index in [`STYLES`]).
    pub style: usize,
    /// Zielordner.
    pub folder: String,
    /// Dateiname (mit Endung der Art nach dem ersten Artwechsel, §1.4).
    pub file_name: String,
    /// „HTML:“ 0 = HTML-Datei exportieren, 1 = In Zwischenablage kopieren.
    pub html_dest: usize,
    /// „Segmente:“ 0 = Ohne, 1 = Segmente exportieren, 2 = entlang Hilfslinien.
    pub slices: usize,
    /// „Seiten:“ 0 = Alle Seiten, 1 = Aktuelle Seite, 2 = Ausgewählte Seiten (SVG: ausgewählte Objekte).
    pub pages: usize,
    /// „Seiten:“ bei PDF (eigene Vorgabe „Alle Seiten“, §2.10).
    pub pdf_pages: usize,
    pub selected_slices_only: bool,
    pub current_frame_only: bool,
    pub include_unsliced: bool,
    pub subfolder: bool,
    /// Bildordner relativ zur HTML-Datei, mit „/“ am Ende.
    pub images_dir: String,
    /// „Bilder zuschneiden“ (Ebenen/Status in Dateien).
    pub trim: bool,
    /// „Exportieren als:“ (Seiten in Dateien) 0 = Bilder, 1 = FW-PNG.
    pub pages_as: usize,
    /// „PDF nach Export anzeigen“.
    pub open_pdf: bool,
    /// Dokument hat Segmente / ein Segment ist ausgewählt (für die Grau-Regeln, §2.1).
    pub has_slices: bool,
    pub slice_selected: bool,
    /// HTML-Endung (HTML-Einrichtung bzw. Voreinstellung `HTMLExtension`) und Endung des
    /// Exportformats der Seite, je ohne Punkt.
    pub html_ext: String,
    pub page_ext: String,
    /// Offene Rückfrage „… ist bereits vorhanden“ mit den vorbereiteten Dateien.
    pub confirm: Option<(String, Vec<ExportFile>)>,
    pub message: Option<String>,
    /// Exportbereich (Werkzeug „Exportbereich“, Dokumentkoordinaten): exportiert nur diesen
    /// Ausschnitt über `file.exportArea`.
    pub area: Option<fp_geom::Rect>,
    /// Optionen je Art (re\18 §4): PDF, CSS-Sprites, Ebenen/Status/Seiten in Dateien, Photoshop.
    pub pdf: PdfOpts,
    pub sprite: SpriteOpts,
    pub files_layers: FilesOpts,
    pub files_frames: FilesOpts,
    pub files_pages: FilesOpts,
    pub psd: PsdOpts,
    /// Offener Optionen-Dialog (Arbeitskopie).
    pub options: Option<OptionsDialog>,
    /// Hinweis nach dem Export (geänderte Selektornamen, §4.2), mit OK zu schließen.
    pub notice: Option<String>,
}

/// Sichtbarkeit eines Bedienelements im Unterpaneel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Vis {
    Hidden,
    /// sichtbar, aber gesperrt
    Grey,
    Free,
}

impl Vis {
    pub fn shown(self) -> bool {
        self != Vis::Hidden
    }
    pub fn free(self) -> bool {
        self == Vis::Free
    }
}

/// Grau- und Sichtbarkeitsregeln des Unterpaneels (re\18 §2) – reine Funktion, ohne Oberfläche testbar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PanelRules {
    pub html: Vis,
    pub slices: Vis,
    pub pages: Vis,
    pub selected_slices: Vis,
    pub current_frame: Vis,
    pub include_unsliced: Vis,
    pub subfolder: Vis,
    pub browse: Vis,
    pub trim: Vis,
    pub current_page: Vis,
    pub export_as: Vis,
    pub open_pdf: Vis,
    /// „Optionen…“ frei.
    pub options: bool,
}

impl PanelRules {
    fn hidden() -> PanelRules {
        let h = Vis::Hidden;
        PanelRules { html: h, slices: h, pages: h, selected_slices: h, current_frame: h, include_unsliced: h, subfolder: h, browse: h, trim: h, current_page: h, export_as: h, open_pdf: h, options: true }
    }
}

/// Regeln für die Art des Dialogs.
pub fn panel_rules(d: &ExportDialog) -> PanelRules {
    use Vis::{Free, Grey, Hidden};
    let mut r = PanelRules::hidden();
    let free_if = |b: bool| if b { Free } else { Grey };
    let browse = |sub: bool| if sub { Free } else { Grey };
    match d.kind() {
        FilesStyle::HtmlAndImages | FilesStyle::ImagesOnly => {
            let images = d.kind() == FilesStyle::ImagesOnly;
            let slicing = d.has_slices && d.slices == 1;
            r.html = if images { Grey } else { Free };
            r.options = !images;
            r.slices = free_if(d.has_slices);
            r.pages = Free;
            r.selected_slices = free_if(slicing && d.slice_selected);
            r.current_frame = free_if(slicing);
            r.include_unsliced = free_if(slicing);
            r.subfolder = if images { Hidden } else { Free };
            r.browse = if images { Hidden } else { browse(d.subfolder) };
        }
        FilesStyle::CssAndImages => {
            r.pages = Free;
            r.trim = Grey;
            r.current_frame = Grey;
            r.subfolder = Free;
            r.browse = browse(d.subfolder);
        }
        FilesStyle::CssSprites => {
            r.pages = Free;
            r.selected_slices = free_if(d.slice_selected);
            r.current_frame = Free;
        }
        FilesStyle::LayersToFiles => {
            r.trim = Free;
            r.current_page = Grey;
            r.current_frame = Grey;
        }
        FilesStyle::FramesToFiles => {
            r.trim = Free;
            r.current_page = Grey;
        }
        FilesStyle::PagesToFiles => {
            r.export_as = Free;
            r.pages = Free;
            r.trim = Grey;
            r.current_frame = Grey;
        }
        FilesStyle::Pdf => {
            r.pages = Free;
            r.open_pdf = Free;
        }
        FilesStyle::Psd => r.pages = Free,
        FilesStyle::Svg => {
            r.pages = Free;
            r.options = false;
        }
    }
    r
}

/// Bekannte Endungen, die beim Artwechsel ersetzt werden.
const KNOWN_EXT: &[&str] = &[
    "pdf", "htm", "html", "xhtml", "shtml", "asp", "jsp", "php", "cfm", "png", "gif", "jpg", "jpeg", "psd", "svg", "webp", "avif", "bmp", "tif", "tiff", "css", "wbmp", "qoi", "jxl", "ico",
];

/// Dateiname ohne bekannte Endung.
pub fn file_stem(name: &str) -> String {
    let n = name.trim();
    let low = n.to_lowercase();
    if low.ends_with(".fw.png") {
        return n[..n.len() - 7].to_string();
    }
    match n.rfind('.') {
        Some(i) if KNOWN_EXT.contains(&low[i + 1..].as_ref()) => n[..i].to_string(),
        _ => n.to_string(),
    }
}

/// Ersetzt nur die Endung (§1.4: der Name bleibt beim Artwechsel erhalten).
pub fn with_ext(name: &str, ext: &str) -> String {
    format!("{}.{ext}", file_stem(name))
}

fn default_images_dir(st: &AppState) -> String {
    let set = st.session.prefs.values.get("ExportSlicesImageFolder").and_then(|v| v.as_str()).filter(|s| !s.trim().is_empty());
    match set {
        Some(s) if s.ends_with('/') => s,
        Some(s) => format!("{s}/"),
        None => t("dlg-export-images-folder"),
    }
}

fn existing_dir(s: &str) -> Option<String> {
    let s = s.trim();
    (!s.is_empty() && Path::new(s).is_dir()).then(|| s.to_string())
}

impl ExportDialog {
    /// Vorgaben aus Dokument und Voreinstellungen (§1.3, §1.4, §7).
    pub fn new(st: &AppState) -> Option<ExportDialog> {
        let d = st.session.active()?;
        let page = d.doc.pages.get(d.page)?;
        let base = d.path.as_ref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| d.title.clone());
        let base = file_stem(&base);
        let html = page.root.export.as_ref().and_then(|e| e.html.as_deref()).cloned().unwrap_or_else(fp_model::HtmlSettings::cs6_default);
        let svg_last = page.last_export_file.as_deref().is_some_and(|f| f.to_lowercase().ends_with(".svg"));
        let has_slices = page.root.layers.iter().any(|s| matches!(s, fp_model::LayerSlot::Own(l) if l.kind == fp_model::LayerKind::Web && l.cels.iter().any(|c| c.elements.iter().any(|e| matches!(&e.kind, fp_model::ElementKind::WebObject(w) if w.slice.is_some())))));
        // Vorgabe wie im Original: ohne Segmente „Nur Bilder“ (HTML „Ohne“), solange im Dokument
        // noch keine Art gewählt wurde (`setByUser`); danach die zuletzt gewählte Art (Q-07)
        let user_set = html.not_yet_set == Some(false);
        let kind = if svg_last {
            FilesStyle::Svg
        } else if !has_slices && !user_set {
            FilesStyle::ImagesOnly
        } else {
            FilesStyle::from_code(html.export_files_style.unwrap_or(1))
        };
        let slice_selected = selected_slice_ids(st).next().is_some();
        // Ordner: zuletzt benutzter Exportordner, sonst der des Dokuments bzw. seiner Datei, sonst der letzte Öffnen-Ordner, sonst „Bilder“ (§7.1)
        let use_last = st.session.prefs.get_bool("UseLastExportDirectory", true);
        let folder = use_last
            .then(|| st.session.prefs.get_str("LastExportDirectory", ""))
            .and_then(|s| existing_dir(&s))
            .or_else(|| page.last_export_dir.as_deref().and_then(existing_dir))
            .or_else(|| d.path.as_ref().and_then(|p| p.parent()).and_then(|p| existing_dir(&p.to_string_lossy())))
            .or_else(|| existing_dir(&st.session.prefs.get_str("LastOpenLocation", "")))
            .or_else(|| fp_i18n::profile::known_folder(fp_i18n::profile::KnownFolder::Pictures).map(|p| p.display().to_string()))
            .unwrap_or_default();
        let html_ext = html
            .file_extension
            .clone()
            .filter(|e| !e.trim().is_empty())
            .unwrap_or_else(|| st.session.prefs.get_str("HTMLExtension", ".htm"))
            .trim()
            .trim_start_matches('.')
            .to_string();
        let page_ext = fp_export::doc_export::page_settings(&d.doc, d.page).format.extension().to_string();
        let sprite = SpriteOpts::load(st, &html);
        let images_dir = html.images_dir.clone().filter(|s| !s.trim().is_empty()).map(|s| if s.ends_with('/') { s } else { format!("{s}/") }).unwrap_or_else(|| default_images_dir(st));
        Some(ExportDialog {
            style: style_index(kind),
            folder,
            file_name: base,
            html_dest: 0,
            slices: if !has_slices {
                0
            } else if html.slice_along_guides == Some(true) {
                2
            } else {
                1
            },
            pages: 1,
            // „Seiten:“ und „PDF nach Export anzeigen“ der PDF-Art aus den Voreinstellungen (§2.10)
            pdf_pages: (st.session.prefs.get_int("PDFExport_PagesToExport", 1) - 1).clamp(0, 2) as usize,
            selected_slices_only: false,
            current_frame_only: false,
            include_unsliced: !html.discard_unspecified_slices.unwrap_or(false),
            subfolder: false,
            images_dir,
            trim: true,
            pages_as: 0,
            open_pdf: st.session.prefs.get_bool("PDFExport_OpenPDFAfterExport", true),
            has_slices,
            slice_selected,
            html_ext: if html_ext.is_empty() { "htm".into() } else { html_ext },
            page_ext,
            confirm: None,
            message: None,
            area: None,
            pdf: PdfOpts::from_prefs(st),
            sprite,
            files_layers: FilesOpts::from_prefs(st, FilesStyle::LayersToFiles),
            files_frames: FilesOpts::from_prefs(st, FilesStyle::FramesToFiles),
            files_pages: FilesOpts::from_prefs(st, FilesStyle::PagesToFiles),
            psd: PsdOpts::from_prefs(st),
            options: None,
            notice: None,
        })
    }

    /// Gewählte Art.
    pub fn kind(&self) -> FilesStyle {
        STYLES.get(self.style).map(|s| s.1).unwrap_or(FilesStyle::HtmlAndImages)
    }

    /// Endung der Art ohne Punkt (§1.4).
    pub fn ext(&self) -> String {
        match self.kind() {
            FilesStyle::Pdf => "pdf".into(),
            FilesStyle::HtmlAndImages | FilesStyle::CssAndImages => self.html_ext.clone(),
            FilesStyle::Psd => "psd".into(),
            FilesStyle::Svg => "svg".into(),
            FilesStyle::CssSprites => self.sprite.ext(),
            FilesStyle::LayersToFiles | FilesStyle::FramesToFiles | FilesStyle::PagesToFiles => self.files_opts(self.kind()).ext().unwrap_or_else(|| self.page_ext.clone()),
            _ => self.page_ext.clone(),
        }
    }

    /// Präfix/Suffix/Dateityp der Art (Ebenen/Status/Seiten in Dateien).
    pub fn files_opts(&self, kind: FilesStyle) -> &FilesOpts {
        match kind {
            FilesStyle::LayersToFiles => &self.files_layers,
            FilesStyle::FramesToFiles => &self.files_frames,
            _ => &self.files_pages,
        }
    }

    fn files_opts_mut(&mut self, kind: FilesStyle) -> &mut FilesOpts {
        match kind {
            FilesStyle::LayersToFiles => &mut self.files_layers,
            FilesStyle::FramesToFiles => &mut self.files_frames,
            _ => &mut self.files_pages,
        }
    }

    /// „Optionen…“: öffnet den Optionen-Dialog der Art mit einer Arbeitskopie (§2, §4) und
    /// liefert das Ziel. Bei den HTML-Arten öffnet der Aufrufer die Bildvorschau.
    pub fn open_options(&mut self) -> OptionsTarget {
        let kind = self.kind();
        let target = export_options::options_target(kind);
        self.options = match target {
            OptionsTarget::Pdf => Some(OptionsDialog::Pdf(self.pdf.clone())),
            OptionsTarget::Sprites => Some(OptionsDialog::Sprites(self.sprite.clone())),
            OptionsTarget::Files => Some(OptionsDialog::Files(kind, self.files_opts(kind).clone())),
            OptionsTarget::Psd => Some(OptionsDialog::Psd(self.psd.clone())),
            OptionsTarget::ImagePreview | OptionsTarget::None => None,
        };
        target
    }

    /// Schließt den Optionen-Dialog: OK übernimmt die Werte (PDF, Photoshop und Ebenen/Status/
    /// Seiten zusätzlich in die Voreinstellungen), Abbrechen verwirft sie.
    pub fn close_options(&mut self, st: &mut AppState, ok: bool) {
        let Some(o) = self.options.take() else { return };
        if !ok {
            return;
        }
        match o {
            OptionsDialog::Pdf(v) => {
                v.to_prefs(st);
                self.pdf = v;
            }
            OptionsDialog::Sprites(v) => self.sprite = v,
            OptionsDialog::Files(kind, v) => {
                v.to_prefs(st, kind);
                *self.files_opts_mut(kind) = v;
            }
            OptionsDialog::Psd(v) => {
                v.to_prefs(st);
                self.psd = v;
            }
        }
        // die Endung im Namen folgt einem geänderten Dateityp bzw. Sprite-Format
        if !self.file_name.trim().is_empty() {
            self.file_name = with_ext(&self.file_name, &self.ext());
        }
    }

    /// Art wechseln: nur die Endung im Namen wird ersetzt.
    pub fn set_style(&mut self, i: usize) {
        self.style = i.min(STYLES.len() - 1);
        if !self.file_name.trim().is_empty() {
            self.file_name = with_ext(&self.file_name, &self.ext());
        }
    }

    /// Seitenwahl der Art.
    fn page_scope(&self) -> PageScope {
        let i = if self.kind() == FilesStyle::Pdf { self.pdf_pages } else { self.pages };
        match i {
            0 => PageScope::All,
            2 => PageScope::Selected,
            _ => PageScope::Current,
        }
    }

    /// Wirksame Paneelwerte nach den Regeln (graue Kästchen mit festem Wert, §2).
    pub fn panel_options(&self, st: &AppState) -> PanelOptions {
        let r = panel_rules(self);
        let k = self.kind();
        let d = st.session.active();
        let slices = match (self.has_slices, self.slices) {
            (_, 2) => SliceMode::AlongGuides,
            (true, 1) => SliceMode::Export,
            _ => SliceMode::None,
        };
        let current_frame_only = match k {
            FilesStyle::CssAndImages | FilesStyle::LayersToFiles | FilesStyle::PagesToFiles => true,
            _ => r.current_frame.free() && self.current_frame_only,
        };
        PanelOptions {
            pages: self.page_scope(),
            selected_pages: d.map(|x| vec![x.page]).unwrap_or_default(),
            slices: if k == FilesStyle::CssSprites { SliceMode::Export } else { slices },
            selected_slices: (r.selected_slices.free() && self.selected_slices_only).then(|| selected_slice_ids(st).collect()),
            current_frame_only,
            frame: d.map(|x| x.frame).unwrap_or(0),
            include_unsliced: if r.include_unsliced.free() { self.include_unsliced } else { true },
            images_dir: if r.subfolder.shown() && self.subfolder { self.images_dir.clone() } else { String::new() },
            trim_images: match k {
                FilesStyle::CssAndImages => true,
                FilesStyle::PagesToFiles => false,
                _ => self.trim,
            },
            html_ext: self.html_ext.clone(),
            selected_elements: d.map(|x| x.selection.elements.clone()).unwrap_or_default(),
            sprite: self.sprite.writer(),
            sprite_settings: self.sprite.settings(),
            file_prefix: FilesOpts::affix(self.files_opts(k).add_prefix, self.files_opts(k).prefix),
            file_suffix: FilesOpts::affix(self.files_opts(k).add_suffix, self.files_opts(k).suffix),
            file_settings: self.files_opts(k).settings(),
            doc_name: d.map(|x| file_stem(&x.path.as_ref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| x.title.clone()))).unwrap_or_default(),
            pdf: self.pdf.writer(d.map(|x| fp_export::doc_export::page_context(&x.doc, x.page).resolution_dpi).unwrap_or(72.0)),
            psd_flatten: self.psd.objects == 1,
        }
    }
}

/// Element-IDs der ausgewählten Segmente.
fn selected_slice_ids(st: &AppState) -> impl Iterator<Item = fp_model::ElementId> + '_ {
    let d = st.session.active();
    let ids: Vec<fp_model::ElementId> = d
        .map(|d| {
            let Some(p) = d.doc.pages.get(d.page) else { return Vec::new() };
            d.selection
                .elements
                .iter()
                .copied()
                .filter(|id| {
                    p.root.layers.iter().any(|s| match s {
                        fp_model::LayerSlot::Own(l) if l.kind == fp_model::LayerKind::Web => l.cels.iter().any(|c| c.elements.iter().any(|e| e.id == *id && matches!(&e.kind, fp_model::ElementKind::WebObject(w) if w.slice.is_some()))),
                        _ => false,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    ids.into_iter()
}

/// Erzeugt die Dateien des Exports (Pfade relativ zum Ordner), ohne zu schreiben.
pub fn export_files(st: &AppState, dlg: &ExportDialog) -> Result<Vec<ExportFile>, String> {
    export_files_noted(st, dlg).map(|r| r.0)
}

/// Wie [`export_files`], dazu die Hinweise des Schreibers (geänderte Selektornamen).
pub fn export_files_noted(st: &AppState, dlg: &ExportDialog) -> Result<(Vec<ExportFile>, ExportNotes), String> {
    let d = st.session.active().ok_or_else(|| t("dlg-export-no-document"))?;
    let stem = file_stem(&dlg.file_name);
    let mut o = DocExportOptions::new(&stem);
    o.page = d.page;
    o.style = dlg.kind();
    o.src_file = d.path.as_ref().map(|p| p.display().to_string()).unwrap_or_default();
    o.panel = Some(dlg.panel_options(st));
    o.bounds = Some(fp_engine::docops::element_bounds);
    // HTML-Einrichtung „Allgemein“/„Tabelle“ (Kleinbuchstaben, leere Zellen; Voreinstellungen) –
    // vorher nur im Engine-Befehl gesetzt, der Dialog exportierte mit den Vorgaben (CT-1a E3)
    o.html_prefs = fp_engine::commands::export::html_prefs(&st.session.prefs);
    let r = ExportRenderer { normal: Arc::clone(&st.renderer) };
    if dlg.kind() == FilesStyle::PagesToFiles && dlg.pages_as == 1 {
        return pages_as_fwpng(&st.session, &d.doc, d.page, &stem, o.panel.as_ref().map(|p| p.pages).unwrap_or_default()).map(|f| (f, ExportNotes::default()));
    }
    let mut notes = ExportNotes::default();
    let mut files = export_document_noted(&d.doc, &r, &o, &fp_export::export::EncodeExtras::default(), &mut notes).map_err(|e| e.to_string())?;
    // „HTML: In Zwischenablage kopieren“: die HTML-Datei wird nicht geschrieben (§3)
    if dlg.kind() == FilesStyle::HtmlAndImages && dlg.html_dest == 1 {
        let ext = format!(".{}", dlg.html_ext.to_lowercase());
        files.retain(|f| !(f.path.to_lowercase().ends_with(&ext) && !f.path.contains('/')));
    }
    Ok((files, notes))
}

/// HTML für die Zwischenablage (Art HTML und Bilder, „In Zwischenablage kopieren“).
fn clipboard_html(st: &AppState, dlg: &ExportDialog) -> Option<String> {
    let mut d2 = dlg.clone();
    d2.html_dest = 0;
    let files = export_files(st, &d2).ok()?;
    let name = format!("{}.{}", file_stem(&dlg.file_name), dlg.html_ext).to_lowercase();
    files.into_iter().find(|f| f.path.to_lowercase() == name).map(|f| String::from_utf8_lossy(&f.bytes).into_owned())
}

/// „Seiten in Dateien“ als FW-PNG: je Seite ein eigenständiges Dokument (§3).
fn pages_as_fwpng(session: &fp_engine::Session, doc: &fp_model::Document, page: usize, stem: &str, scope: PageScope) -> Result<Vec<ExportFile>, String> {
    let all: Vec<usize> = (0..doc.pages.len()).filter(|&i| !doc.pages[i].master()).collect();
    let pages = if scope == PageScope::All { all } else { vec![page] };
    let mut used = std::collections::HashSet::new();
    let mut out = Vec::new();
    for i in pages {
        let mut one = doc.clone();
        let keep = Arc::clone(&doc.pages[i]);
        one.pages = vec![keep];
        let name = if scope == PageScope::All { doc.pages[i].name.clone().filter(|n| !n.trim().is_empty()).unwrap_or_else(|| format!("{}", i + 1)) } else { stem.to_string() };
        let mut name: String = name.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '_' }).collect();
        while !used.insert(name.to_lowercase()) {
            name.push_str("_1");
        }
        one.current_page = 0;
        one.invalidate_root_copy();
        let opts = fp_fwpng::WriteOptions { software: "Firepixel".into(), ..fp_fwpng::WriteOptions::default() };
        let bytes = match session.renderer() {
            Some(r) => match r.flattener() {
                Some(f) => fp_fwpng::write(&one, &opts, f),
                None => fp_fwpng::write(&one, &opts, &fp_engine::render::RendererFlattener(r.as_ref())),
            },
            None => fp_fwpng::write(&one, &opts, &fp_fwpng::NoFlattener),
        }
        .map_err(|e| e.to_string())?;
        out.push(ExportFile { path: format!("{name}.fw.png"), bytes });
    }
    Ok(out)
}

/// Ergebnis eines Export-Versuchs.
#[derive(Clone, Debug, PartialEq)]
pub enum ExportStep {
    /// geschrieben: Anzahl Dateien
    Done(usize),
    /// Rückfrage offen (Text steht in `dlg.confirm`)
    Confirm(String),
    Error(String),
}

/// „Exportieren“: Dateien bilden, auf vorhandene prüfen (§5.1), sonst schreiben und merken.
pub fn begin_export(st: &mut AppState, dlg: &mut ExportDialog) -> ExportStep {
    let folder = PathBuf::from(dlg.folder.trim());
    if dlg.folder.trim().is_empty() || !folder.is_dir() {
        return ExportStep::Error(fp_i18n::tr!("dlg-export-folder-missing", path = dlg.folder.trim()));
    }
    if file_stem(&dlg.file_name).trim().is_empty() {
        return ExportStep::Error(fp_i18n::tr!("dlg-export-failed", error = t("dlg-export-file-name")));
    }
    let files = match export_files_noted(st, dlg) {
        Ok((f, notes)) => {
            // Meldung nach dem Export (§3, §4.2)
            dlg.notice = notes.selectors_modified.then(|| t("dlg-expopt-sprite-modified"));
            f
        }
        Err(e) => return ExportStep::Error(fp_i18n::tr!("dlg-export-failed", error = e)),
    };
    // Firepixel prüft alle Dateien (strenger als das Original, §11), fragt aber nur einmal und
    // nennt die erste vorhandene
    if let Some(f) = files.iter().find(|f| folder.join(&f.path).exists()) {
        let name = Path::new(&f.path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| f.path.clone());
        let q = fp_i18n::tr!("dlg-export-replace", file = name);
        dlg.confirm = Some((q.clone(), files));
        return ExportStep::Confirm(q);
    }
    finish_export(st, dlg, files)
}

/// Antwort auf die Rückfrage: OK schreibt alles, Abbrechen schreibt nichts (zurück in den Dialog).
pub fn answer_confirm(st: &mut AppState, dlg: &mut ExportDialog, ok: bool) -> ExportStep {
    let Some((_, files)) = dlg.confirm.take() else { return ExportStep::Error(String::new()) };
    if !ok {
        return ExportStep::Error(String::new());
    }
    finish_export(st, dlg, files)
}

fn finish_export(st: &mut AppState, dlg: &mut ExportDialog, files: Vec<ExportFile>) -> ExportStep {
    let folder = PathBuf::from(dlg.folder.trim());
    for f in &files {
        let p = folder.join(&f.path);
        if let Some(parent) = p.parent()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            return ExportStep::Error(fp_i18n::tr!("dlg-export-failed", error = e.to_string()));
        }
        if let Err(e) = std::fs::write(&p, &f.bytes) {
            return ExportStep::Error(fp_i18n::tr!("dlg-export-failed", error = format!("{}: {e}", p.display())));
        }
    }
    if dlg.kind() == FilesStyle::HtmlAndImages && dlg.html_dest == 1 {
        st.dialogs.clipboard = clipboard_html(st, dlg);
    }
    remember(st, dlg);
    if dlg.kind() == FilesStyle::Pdf
        && dlg.open_pdf
        && let Some(f) = files.first()
    {
        let _ = fp_engine::commands::export::open_with_system(&folder.join(&f.path), None);
    }
    st.dialogs.exported = st.dialogs.exported.wrapping_add(1);
    ExportStep::Done(files.len())
}

/// Merken nach dem Export (§7): Ordner in den Voreinstellungen, Art/Ordner/Name/Bildordner im
/// Dokument (Exporteinstellungen der Seite). Kein Verlaufsschritt.
pub fn remember(st: &mut AppState, dlg: &ExportDialog) {
    let folder = dlg.folder.trim().to_string();
    let kind = dlg.kind();
    let file = with_ext(&dlg.file_name, &dlg.ext());
    if let Some(ds) = st.session.active_mut() {
        let page = ds.page;
        let doc = Arc::make_mut(&mut ds.doc);
        if let Some(p) = doc.page_mut(page) {
            p.last_export_dir = Some(folder.clone());
            p.last_export_file = Some(file);
            // auch ohne bisherige Exporteinstellungen merken (letzte Wahl, Q-07)
            {
                let e = p.root.export.get_or_insert_with(|| Box::new(fp_model::ExportSettings::default()));
                let h = e.html.get_or_insert_with(|| Box::new(fp_model::HtmlSettings::cs6_default()));
                if let Some(c) = kind.code() {
                    h.export_files_style = Some(c);
                }
                h.not_yet_set = Some(false);
                h.last_export_dir = Some(folder.clone());
                h.base_name = Some(file_stem(&dlg.file_name));
                if matches!(kind, FilesStyle::HtmlAndImages | FilesStyle::ImagesOnly) {
                    h.discard_unspecified_slices = Some(!dlg.include_unsliced);
                    h.slice_along_guides = Some(dlg.slices == 2);
                }
                if dlg.subfolder && matches!(kind, FilesStyle::HtmlAndImages | FilesStyle::CssAndImages) {
                    h.images_dir = Some(dlg.images_dir.clone());
                }
                if kind == FilesStyle::CssSprites {
                    dlg.sprite.store(h);
                }
            }
        }
    }
    st.session.prefs.set("LastExportDirectory", fp_engine::prefs::PrefValue::Str(folder));
    if kind == FilesStyle::Pdf {
        st.session.prefs.set("PDFExport_PagesToExport", fp_engine::prefs::PrefValue::Int(dlg.pdf_pages as i64 + 1));
        st.session.prefs.set("PDFExport_OpenPDFAfterExport", fp_engine::prefs::PrefValue::Bool(dlg.open_pdf));
    }
    st.session.prefs_changed();
}

/// Bildordner relativ zum Exportordner („bilder/“).
fn relative_dir(dir: &Path, folder: &Path) -> String {
    let rel = dir.strip_prefix(folder).map(|p| p.to_path_buf()).unwrap_or_else(|_| dir.to_path_buf());
    let s = rel.to_string_lossy().replace('\\', "/");
    if s.is_empty() || s.ends_with('/') { s } else { format!("{s}/") }
}

/// Zeigt „Exportieren“.
pub fn show_export(ctx: &egui::Context, st: &mut AppState) {
    let Some(d) = st.dialogs.export.clone() else { return };
    if d.area.is_some() {
        show_area(ctx, st, d);
        return;
    }
    show_full(ctx, st, d);
}

fn show_full(ctx: &egui::Context, st: &mut AppState, mut d: ExportDialog) {
    enum Act {
        Export,
        Cancel,
        Browse,
        BrowseImages,
        Options,
    }
    let mut act = None;
    let rules = panel_rules(&d);
    let kind = d.kind();
    let names: Vec<String> = STYLES
        .iter()
        .map(|s| if s.1 == FilesStyle::CssAndImages { format!("{} (.{})", t(s.0), d.html_ext) } else { t(s.0) })
        .collect();
    let l = |k: &str| t(k);
    let (folder_l, file_l, style_l, browse_l, export_l, cancel_l, options_l) = (l("dlg-export-folder"), l("dlg-export-file-name"), l("dlg-export-style"), l("dlg-export-browse"), l("dlg-export-save"), l("btn-cancel"), l("dlg-export-options"));
    let (html_l, slices_l, pages_l, as_l) = (l("dlg-export-html"), l("dlg-export-slices-label"), l("dlg-export-pages"), l("dlg-export-as"));
    let html_items: Vec<String> = if kind == FilesStyle::ImagesOnly {
        vec![l("dlg-export-html-file"), l("dlg-export-html-clipboard"), l("dlg-export-none")]
    } else {
        vec![l("dlg-export-html-file"), l("dlg-export-html-clipboard")]
    };
    let slice_items = vec![l("dlg-export-none"), l("dlg-export-slices"), l("dlg-export-slices-guides")];
    let page_items = if kind == FilesStyle::Svg {
        vec![l("dlg-export-pages-all"), l("dlg-export-pages-current"), l("dlg-export-pages-objects")]
    } else {
        vec![l("dlg-export-pages-all"), l("dlg-export-pages-current"), l("dlg-export-pages-selected")]
    };
    let as_items = vec![l("dlg-export-as-images"), "FW-PNG (*.png)".to_string()];
    let checks = [
        l("dlg-export-selected-slices"),
        l("dlg-export-current-frame"),
        l("dlg-export-include-unsliced"),
        l("dlg-export-subfolder"),
        l("dlg-export-trim"),
        l("dlg-export-current-page"),
        l("dlg-export-view-pdf"),
    ];
    // Spalten (zweispaltiges Raster wie der Dateiteil, §1.2)
    let lw = super::max_text_w(ctx, &[&folder_l, &file_l, &style_l, &html_l, &slices_l, &pages_l, &as_l]).max(84.0).ceil() + 6.0;
    let fw = 300f32.max(super::combo_fit_w(ctx, &refs(&names), 200.0)).max(super::combo_fit_w(ctx, &refs(&page_items), 200.0));
    let cw = checks.iter().map(|c| 22.0 + super::text_w(ctx, c)).fold(0f32, f32::max).ceil();
    let fw = fw.max(cw * 2.0 + 12.0);
    let bw = super::buttons_w(ctx, &[&browse_l, &export_l, &cancel_l, &options_l], 90.0);
    let width = (16.0 + lw + 8.0 + fw + 16.0 + bw + 16.0).ceil();
    let r = dialog_frame(ctx, "fp-export", &t("dlg-export-title"), vec2(width, 372.0), |ui, body| {
        let theme = Theme::current(ui.ctx());
        let o = body.min + vec2(16.0, 14.0);
        let fx = lw + 8.0;
        let bx = fx + fw + 16.0;
        let modal = d.confirm.is_none() && d.options.is_none() && d.notice.is_none();
        let row = |y: f32| (Rect::from_min_size(o + vec2(0.0, y + 1.0), vec2(lw, 20.0)), Rect::from_min_size(o + vec2(fx, y), vec2(fw, 22.0)), Rect::from_min_size(o + vec2(bx, y - 1.0), vec2(bw, 23.0)));
        // Ordner
        let (lr, fr, br) = row(0.0);
        super::fit_label(ui, lr, &folder_l, true);
        if let Some(v) = fields::text(ui, "ex-folder", fr, &d.folder, modal) {
            d.folder = v;
        }
        if fields::button(ui, "ex-browse", br, &browse_l, modal) {
            act = Some(Act::Browse);
        }
        // Dateiname + Exportieren
        let (lr, fr, br) = row(30.0);
        super::fit_label(ui, lr, &file_l, true);
        if let Some(v) = fields::text(ui, "ex-name", fr, &d.file_name, modal) {
            d.file_name = v;
        }
        if push_button(ui, br, &export_l, true, modal && !file_stem(&d.file_name).trim().is_empty()) {
            act = Some(Act::Export);
        }
        // Art + Abbrechen
        let (lr, fr, br) = row(58.0);
        super::fit_label(ui, lr, &style_l, true);
        if let Some(i) = fields::combo(ui, "ex-style", fr, &refs(&names), Some(d.style), modal) {
            d.set_style(i);
        }
        if push_button(ui, br, &cancel_l, false, true) {
            act = Some(Act::Cancel);
        }
        let sep = o.y + 90.0;
        ui.painter().hline(body.left() + 12.0..=body.right() - 12.0, sep, egui::Stroke::new(1.0, theme.colors.line_mid));
        // Unterpaneel (§2)
        let mut y = 100.0;
        let mut opt_y = y;
        let combo_row = |ui: &mut egui::Ui, y: f32, id: &str, label: &str, items: &[String], sel: usize, vis: Vis| -> Option<usize> {
            let (lr, fr, _) = row(y);
            super::fit_label(ui, lr, label, vis.free());
            let w = if matches!(kind, FilesStyle::Pdf) { fw } else { fw.min(260.0) };
            fields::combo(ui, id, Rect::from_min_size(fr.min, vec2(w, 22.0)), &refs(items), Some(sel), vis.free() && modal)
        };
        if kind == FilesStyle::PagesToFiles {
            if let Some(i) = combo_row(ui, y, "ex-as", &as_l, &as_items, d.pages_as, rules.export_as) {
                d.pages_as = i;
            }
            y += 28.0;
            opt_y = y;
        }
        if rules.html.shown() {
            let sel = if kind == FilesStyle::ImagesOnly { 2 } else { d.html_dest.min(1) };
            if let Some(i) = combo_row(ui, y, "ex-html", &html_l, &html_items, sel, rules.html) {
                d.html_dest = i.min(1);
            }
            y += 28.0;
        }
        if rules.slices.shown() {
            let sel = if d.has_slices { d.slices } else { 0 };
            if let Some(i) = combo_row(ui, y, "ex-slices", &slices_l, &slice_items, sel, rules.slices) {
                d.slices = i;
            }
            y += 28.0;
        }
        if rules.pages.shown() {
            let sel = if kind == FilesStyle::Pdf { d.pdf_pages } else { d.pages };
            if let Some(i) = combo_row(ui, y, "ex-pages", &pages_l, &page_items, sel, rules.pages) {
                if kind == FilesStyle::Pdf {
                    d.pdf_pages = i;
                } else {
                    d.pages = i;
                }
            }
            y += 30.0;
        }
        // Kästchen in zwei Spalten
        let forced = |k: FilesStyle, which: usize, cur: bool| -> bool {
            match (k, which) {
                (FilesStyle::CssAndImages, 1) | (FilesStyle::LayersToFiles, 1) | (FilesStyle::PagesToFiles, 1) => true,
                (FilesStyle::CssAndImages, 4) => true,
                (FilesStyle::PagesToFiles, 4) => false,
                (_, 5) => true,
                _ => cur,
            }
        };
        let mut items: Vec<(usize, Vis, bool)> = Vec::new();
        let push = |items: &mut Vec<(usize, Vis, bool)>, which: usize, vis: Vis, cur: bool| {
            if vis.shown() {
                items.push((which, vis, if vis.free() { cur } else { forced(kind, which, cur) }));
            }
        };
        match kind {
            FilesStyle::HtmlAndImages | FilesStyle::ImagesOnly => {
                push(&mut items, 0, rules.selected_slices, d.selected_slices_only);
                push(&mut items, 1, rules.current_frame, d.current_frame_only);
                push(&mut items, 2, rules.include_unsliced, if d.has_slices { d.include_unsliced } else { d.include_unsliced });
                push(&mut items, 3, rules.subfolder, d.subfolder);
            }
            FilesStyle::CssAndImages => {
                push(&mut items, 4, rules.trim, true);
                push(&mut items, 1, rules.current_frame, true);
                push(&mut items, 3, rules.subfolder, d.subfolder);
            }
            FilesStyle::CssSprites => {
                push(&mut items, 0, rules.selected_slices, d.selected_slices_only);
                push(&mut items, 1, rules.current_frame, d.current_frame_only);
            }
            FilesStyle::LayersToFiles => {
                push(&mut items, 4, rules.trim, d.trim);
                push(&mut items, 5, rules.current_page, true);
                push(&mut items, 1, rules.current_frame, true);
            }
            FilesStyle::FramesToFiles => {
                push(&mut items, 4, rules.trim, d.trim);
                push(&mut items, 5, rules.current_page, true);
            }
            FilesStyle::PagesToFiles => {
                push(&mut items, 4, rules.trim, false);
                push(&mut items, 1, rules.current_frame, true);
            }
            FilesStyle::Pdf => push(&mut items, 6, rules.open_pdf, d.open_pdf),
            _ => {}
        }
        for (n, (which, vis, cur)) in items.iter().enumerate() {
            let col = (n % 2) as f32;
            let r = Rect::from_min_size(o + vec2(fx + col * (cw + 12.0), y + (n / 2) as f32 * 22.0), vec2(cw, 18.0));
            if let Some(b) = fields::checkbox(ui, ("ex-check", *which), r, &checks[*which], *cur, vis.free() && modal) {
                match which {
                    0 => d.selected_slices_only = b,
                    1 => d.current_frame_only = b,
                    2 => d.include_unsliced = b,
                    3 => d.subfolder = b,
                    4 => d.trim = b,
                    6 => d.open_pdf = b,
                    _ => {}
                }
            }
        }
        y += items.len().div_ceil(2) as f32 * 22.0 + 4.0;
        if rules.browse.shown() {
            let br = Rect::from_min_size(o + vec2(fx, y), vec2(super::button_w(ui.ctx(), &browse_l, 84.0), 23.0));
            if fields::button(ui, "ex-browse-images", br, &browse_l, rules.browse.free() && modal) {
                act = Some(Act::BrowseImages);
            }
            if d.subfolder {
                ui.painter().text(pos2(br.right() + 8.0, br.center().y), Align2::LEFT_CENTER, d.images_dir.as_str(), theme.ui_font(), theme.colors.text);
            }
        }
        // „Optionen…“ rechts (§2): eigener Dialog je Art (export_options)
        if fields::button(ui, "ex-options", Rect::from_min_size(o + vec2(bx, opt_y - 1.0), vec2(bw, 23.0)), &options_l, rules.options && modal) {
            act = Some(Act::Options);
        }
        if let Some(m) = &d.message {
            super::put_left(ui, Rect::from_min_size(pos2(o.x, body.bottom() - 44.0), vec2(body.width() - 32.0, 36.0)), egui::Label::new(egui::RichText::new(m).color(theme.colors.text_weak)).wrap());
        }
    });
    // Optionen-Dialog der Art (§4) über dem Exportdialog
    if let Some(mut o) = d.options.take() {
        let kind_name = STYLES.get(d.style).map(|s| t(s.0)).unwrap_or_default();
        let ans = export_options::show(ctx, &mut o, &kind_name);
        d.options = Some(o);
        if let Some(ok) = ans {
            d.close_options(st, ok);
        }
        st.dialogs.export = Some(d);
        return;
    }
    // Hinweis nach dem Export (geänderte Selektornamen, §4.2): OK schließt den Exportdialog
    if let Some(n) = d.notice.clone() {
        let ok_l = t("btn-ok");
        let w = 420f32.max(32.0 + super::text_w(ctx, &n).min(560.0)).ceil();
        let h = 64.0 + super::wrapped_h(ctx, &n, w - 32.0);
        let mut done = false;
        let c = dialog_frame(ctx, "fp-export-notice", "Firepixel", vec2(w, h + 40.0), |ui, body| {
            super::put_left(ui, Rect::from_min_size(body.min + vec2(16.0, 16.0), vec2(body.width() - 32.0, h - 48.0)), egui::Label::new(n.as_str()).wrap());
            let rects = super::button_row(ui.ctx(), body.right() - 12.0, body.bottom() - 35.0, &[&ok_l], 75.0, 23.0, 8.0);
            done = push_button(ui, rects[0], &ok_l, true, true);
        });
        st.dialogs.export = if done || c == DialogClose::Cancelled { None } else { Some(d) };
        return;
    }
    // Rückfrage „… ist bereits vorhanden“ (§5.1): OK / Abbrechen
    let mut answer = None;
    if let Some((q, _)) = &d.confirm {
        let q = q.clone();
        let (ok_l, cancel_l) = (t("btn-ok"), t("btn-cancel"));
        let w = 420f32.max(32.0 + super::text_w(ctx, &q).min(560.0)).ceil();
        let h = 64.0 + super::wrapped_h(ctx, &q, w - 32.0);
        let c = dialog_frame(ctx, "fp-export-replace", "Firepixel", vec2(w, h + 40.0), |ui, body| {
            super::put_left(ui, Rect::from_min_size(body.min + vec2(16.0, 16.0), vec2(body.width() - 32.0, h - 48.0)), egui::Label::new(q.as_str()).wrap());
            let rects = super::button_row(ui.ctx(), body.right() - 12.0, body.bottom() - 35.0, &[&ok_l, &cancel_l], 75.0, 23.0, 8.0);
            if push_button(ui, rects[0], &ok_l, true, true) {
                answer = Some(true);
            }
            if push_button(ui, rects[1], &cancel_l, false, true) {
                answer = Some(false);
            }
        });
        if c == DialogClose::Cancelled {
            answer = Some(false);
        }
    }
    if let Some(ok) = answer {
        match answer_confirm(st, &mut d, ok) {
            ExportStep::Done(n) => {
                st.push(Action::Status(fp_i18n::tr!("dlg-export-done", n = n, path = d.folder.trim())));
                st.dialogs.export = d.notice.is_some().then_some(d);
                return;
            }
            ExportStep::Error(e) => d.message = (!e.is_empty()).then_some(e),
            ExportStep::Confirm(_) => {}
        }
    }
    match (act, r) {
        (Some(Act::Export), _) => match begin_export(st, &mut d) {
            ExportStep::Done(n) => {
                st.push(Action::Status(fp_i18n::tr!("dlg-export-done", n = n, path = d.folder.trim())));
                st.dialogs.export = d.notice.is_some().then_some(d);
            }
            ExportStep::Confirm(_) => st.dialogs.export = Some(d),
            ExportStep::Error(e) => {
                d.message = Some(e);
                st.dialogs.export = Some(d);
            }
        },
        (Some(Act::Cancel), _) => st.dialogs.export = None,
        (_, DialogClose::Cancelled) if d.confirm.is_none() && d.options.is_none() && d.notice.is_none() => st.dialogs.export = None,
        (Some(Act::Browse), _) => {
            let start = existing_dir(&d.folder).map(PathBuf::from);
            st.dialogs.export = Some(d);
            crate::file_dialog::request(st, crate::file_dialog::Request::folder(t("dlg-export-pick-folder")).dir(start.as_deref()), move |st, r| {
                if let (Some(p), Some(x)) = (r.into_iter().next(), st.dialogs.export.as_mut()) {
                    x.folder = p.display().to_string();
                }
            });
        }
        (Some(Act::BrowseImages), _) => {
            let folder = PathBuf::from(d.folder.trim());
            let start = folder.join(d.images_dir.trim_end_matches('/'));
            let start = if start.is_dir() { start } else { folder.clone() };
            st.dialogs.export = Some(d);
            crate::file_dialog::request(st, crate::file_dialog::Request::folder(t("dlg-export-pick-images-folder")).dir(Some(&start)), move |st, r| {
                if let (Some(p), Some(x)) = (r.into_iter().next(), st.dialogs.export.as_mut()) {
                    x.images_dir = relative_dir(&p, &folder);
                }
            });
        }
        (Some(Act::Options), _) => {
            // HTML-Arten: „Optionen…“ öffnet die HTML-Einrichtung (re\18 §6.1, E3)
            let html = d.open_options() == OptionsTarget::ImagePreview;
            st.dialogs.export = Some(d);
            if html {
                super::html_setup::open(st);
            }
        }
        _ => st.dialogs.export = Some(d),
    }
}

/// Kurzer Ablauf für den Exportbereich: Name, dann Speichern-Dialog.
fn show_area(ctx: &egui::Context, st: &mut AppState, mut d: ExportDialog) {
    let mut result = None;
    let title = t("dlg-export-area-title");
    let (file_l, export_l, cancel_l, options_l) = (t("dlg-export-file-name"), t("dlg-export-button"), t("btn-cancel"), t("dlg-export-options"));
    let pset = st.session.active().map(|x| fp_export::doc_export::page_settings(&x.doc, x.page));
    let preset = pset.as_ref().map(|s| optimize::preset_index(s).map(|i| optimize::preset_label(fp_export::options::builtin_presets()[i].name)).unwrap_or_else(|| optimize::settings_label(s))).unwrap_or_default();
    let opt_text = fp_i18n::tr!("dlg-export-optimize", preset = preset.as_str());
    let dx = super::extra_w(ctx, &[&file_l], 110.0 - 4.0);
    let ow = super::button_w(ctx, &options_l, 110.0);
    let ox = 240f32.max((super::text_w(ctx, &opt_text) + 12.0).ceil());
    let bw = super::button_w(ctx, &export_l, 96.0);
    let cbw = super::button_w(ctx, &cancel_l, 76.0);
    let width = 440f32.max(32.0 + 120.0 + dx + 200.0).max(32.0 + ox + ow).max(16.0 + bw + 8.0 + cbw + 16.0).ceil();
    let r = dialog_frame(ctx, "fp-export", &title, vec2(width, 200.0), |ui, body| {
        let theme = Theme::current(ui.ctx());
        let o = body.min + vec2(16.0, 16.0);
        super::fit_label(ui, Rect::from_min_size(o, vec2(110.0 + dx, 20.0)), &file_l, true);
        if let Some(t) = fields::text(ui, "ex-name", Rect::from_min_size(o + vec2(120.0 + dx, 0.0), vec2(200.0, 20.0)), &d.file_name, true) {
            d.file_name = t;
        }
        ui.painter().text(o + vec2(0.0, 36.0), Align2::LEFT_TOP, opt_text.as_str(), theme.ui_font(), theme.colors.text_weak);
        if fields::button(ui, "ex-options", Rect::from_min_size(o + vec2(ox, 32.0), vec2(ow, 22.0)), &options_l, true) {
            st.push(Action::Run("file.imagePreview".into(), serde_json::json!({})));
        }
        if let Some(a) = d.area {
            let area = fp_i18n::tr!("dlg-export-area", x = format!("{:.0}", a.left), y = format!("{:.0}", a.top), w = format!("{:.0}", a.width()), h = format!("{:.0}", a.height()));
            super::put_left(ui, Rect::from_min_size(o + vec2(0.0, 64.0), vec2(body.width() - 32.0, 18.0)), egui::Label::new(egui::RichText::new(area).color(theme.colors.text)).wrap());
        }
        if let Some(m) = &d.message {
            super::put_left(ui, Rect::from_min_size(o + vec2(0.0, 86.0), vec2(body.width() - 32.0, 36.0)), egui::Label::new(egui::RichText::new(m).color(theme.colors.text_weak)).wrap());
        }
        let by = body.bottom() - 36.0;
        if push_button(ui, Rect::from_min_size(pos2(body.right() - 16.0 - cbw - 8.0 - bw, by), vec2(bw, 23.0)), &export_l, true, !file_stem(&d.file_name).trim().is_empty()) {
            result = Some(true);
        }
        if push_button(ui, Rect::from_min_size(pos2(body.right() - 16.0 - cbw, by), vec2(cbw, 23.0)), &cancel_l, false, true) {
            result = Some(false);
        }
    });
    match (result, r) {
        (Some(true), _) => {
            let a = d.area.unwrap_or_default();
            let ext = d.page_ext.clone();
            let image_l = t("dlg-filter-image");
            // Dateidialog ohne Blockieren: der Exportdialog wartet ausgeblendet und kommt bei Abbruch
            // bzw. Fehler zurück
            st.dialogs.export = None;
            let req = crate::file_dialog::Request::save(t("dlg-export-area-title"), format!("{}.{ext}", file_stem(&d.file_name))).filter(image_l, &["png", "gif", "jpg", "webp", "avif", "bmp", "tif"]);
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
