//! Dialog „Bildvorschau“ (Datei ▸ Bildvorschau…, `file.imagePreview`) nach
//! `re\17-bildvorschau.md`, Paket P1 „Mehrfachansicht und Vorschau-Kern“:
//! - Vorschaufläche mit **1, 2 oder 4 Feldern** (2-fach übereinander, 4-fach 2 × 2, Feldnummern
//!   0 oben links, 1 unten links, 2 oben rechts, 3 unten rechts, §5.1); **jedes Feld hat eine
//!   eigene Exporteinstellung**.
//! - Kopfzeile je Feld (Format, Palette bzw. Qualität, angeforderte Farben, Größe und Ladezeit
//!   genau nach §5.4), Vorgabenliste mit „+“ (Dialog „Vorgabenname“) und Kästchen „Vorschau“
//!   (aus = Original, §5.3).
//! - **Aktives Feld** mit Doppelrahmen; ein Klick aktiviert es, die linke Spalte
//!   ([`options`]) zeigt und ändert immer seine Einstellung.
//! - Zoom (11 Stufen 6 % … 6400 %) und Verschieben wirken **auf alle Felder gleich** (§5.6).
//! - Werkzeugleiste unten ([`toolbar`], §6); Exportbereich und Animation folgen in Paket P3.
//! - Rechnen **im Hintergrund** je Feld mit Abbruch ([`worker`]); die Vorschau kodiert mit
//!   denselben Kodierern und Parametern wie der echte Export.
//! - Fenster in der Größe veränderbar, Start- = Mindestgröße (§1.1); Größe, Modus und
//!   Feldeinstellungen werden wie im Original **nicht** gemerkt (§7).

pub mod color_table;
pub mod optimize_to_size;
pub mod options;
pub mod pane;
pub mod toolbar;
pub mod worker;

pub use pane::Pane;

use super::{button_w, dialog_frame, dialog_frame_resizable, push_button, DialogClose, DIALOG_TITLE_H};
use crate::app_state::{Action, AppState};
use crate::panels::optimize::{self, ExportRenderer};
use crate::theme::Theme;
use egui::{pos2, vec2, Id, Rect, Sense, Stroke, Vec2};
use fp_export::options::OptimizeSettings;
use fp_i18n::t;
use std::sync::Arc;
use worker::{Job, JobInput, JobKind, PaneResult, RenderCache};

/// Kennung des Dialograhmens.
pub const DIALOG_ID: &str = "fp-imgpreview";
/// Kennung des Dialogs „Vorgabenname“ („+“ im Feld).
pub const PRESET_DIALOG_ID: &str = "fp-imgpreview-preset";
/// Start- und Mindestgröße des Client-Bereichs (§1.1: außen 714 × 621, Client ≈ 698 × 582).
pub const MIN_CLIENT: Vec2 = vec2(698.0, 582.0);
/// Mindestgröße der Vorschaufläche im 1-fach-Modus (Feldrahmen 396 × 492, §5.1).
const MIN_AREA_W: f32 = 396.0;

/// Anzahl Felder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewMode {
    One,
    Two,
    Four,
}

impl ViewMode {
    /// Sichtbare Felder in Feldnummern (0 oben links, 1 unten links, 2 oben rechts, 3 unten rechts).
    pub fn panes(self) -> &'static [usize] {
        match self {
            ViewMode::One => &[0],
            ViewMode::Two => &[0, 1],
            ViewMode::Four => &[0, 1, 2, 3],
        }
    }
}

/// Werkzeug der Vorschau.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    /// Zeiger: Ziehen verschiebt alle Felder, Klick aktiviert ein Feld.
    Pointer,
    /// Zoom: Klick eine Stufe größer, Alt+Klick eine Stufe kleiner, um den Klickpunkt.
    Zoom,
    /// Eine der drei Transparenz-Pipetten (§2.6): Klick in ein Feld nimmt die Palettenfarbe des
    /// Bildpunkts.
    Pipette(color_table::Pipette),
}

/// Ergebnis einer Palettendatei (Laden/Speichern laufen über den Dateidialog und kommen später an).
#[derive(Clone, Debug)]
pub enum PaletteIo {
    Loaded { pane: usize, colors: Vec<[u8; 3]> },
    Error(String),
}

/// Zustand der Bildvorschau.
pub struct ImagePreview {
    pub mode: ViewMode,
    pub panes: [Pane; 4],
    /// Aktives Feld (Feldnummer).
    pub active: usize,
    /// Stufe in [`toolbar::ZOOMS`] (Start 100 %).
    pub zoom_idx: usize,
    /// Gemeinsamer Bildpunkt in der Feldmitte (Bildpixel); `None` = Bildmitte.
    pub center: Option<Vec2>,
    pub tool: Tool,
    /// Ziehen mit dem Zeiger: Druckpunkt und Bildmitte beim Drücken.
    grab: Option<(egui::Pos2, Vec2)>,
    /// Registerkarte der linken Spalte (0 Optionen, 1 Datei, 2 Animation).
    pub tab: usize,
    /// Offener Dialog „Vorgabenname“: Feld und Name.
    pub preset_name: Option<(usize, String)>,
    /// Unverändertes Original (für Felder mit „Vorschau“ aus).
    pub original: Option<Arc<PaneResult>>,
    original_job: Option<Job>,
    original_tex: Option<(u64, egui::TextureHandle)>,
    cache: Arc<RenderCache>,
    /// Lage der Felder im letzten Bild (für Tests und Diagnose).
    pub layout: Vec<(usize, pane::PaneLayout)>,
    /// Farbtabelle des aktiven Feldes (Auswahl, Farbfeld-Rückmeldung).
    pub table: color_table::TableState,
    /// Offener Dialog „Farbe“.
    pub color_dialog: Option<color_table::ColorDialog>,
    /// Offener Dialog „Für Dateigröße optimieren“.
    pub size_dialog: Option<optimize_to_size::SizeDialog>,
    /// Laufende Suche „Optimieren auf Größe“.
    pub size_job: Option<optimize_to_size::SizeJob>,
    /// Offene Meldung (z. B. Ziel zu klein).
    pub message: Option<String>,
    /// Ergebnisse von „Palette laden/speichern“ (aus dem Dateidialog).
    pub palette_io: Arc<std::sync::Mutex<Vec<PaletteIo>>>,
    feedback_tex: Option<(u64, Vec<usize>, egui::TextureHandle)>,
    /// „Palette laden/speichern“ aus dem Kontextmenü der Vorschau (Dateidialog im nächsten Schritt).
    pending_file: Option<(usize, color_table::PaletteCmd)>,
}

/// Ergebnis eines Bildes.
enum Outcome {
    Ok,
    Export,
    Cancel,
}

impl ImagePreview {
    /// Öffnen: Feld 0 bekommt die Exporteinstellung der Seite; 1-fach, 100 %, Zeiger, Optionen (§7).
    pub fn new(st: &AppState) -> Option<ImagePreview> {
        let d = st.session.active()?;
        let s = fp_export::doc_export::page_settings(&d.doc, d.page);
        Some(ImagePreview::with_settings(s, &crate::ui_commands::export_presets(&st.session.prefs)))
    }

    /// Wie [`ImagePreview::new`] mit gegebener Einstellung.
    pub fn with_settings(s: OptimizeSettings, own: &[(String, fp_model::ExportSettings)]) -> ImagePreview {
        let mut panes = [Pane::new(s.clone()), Pane::new(s.clone()), Pane::new(s.clone()), Pane::new(s)];
        panes[0].initialized = true;
        let entries = pane::preset_entries(own);
        panes[0].last_preset = pane::matching_preset(&entries, &panes[0].settings).map(|i| entries[i].label.clone());
        ImagePreview {
            mode: ViewMode::One,
            panes,
            active: 0,
            zoom_idx: toolbar::ZOOM_100,
            center: None,
            tool: Tool::Pointer,
            grab: None,
            tab: 0,
            preset_name: None,
            original: None,
            original_job: None,
            original_tex: None,
            cache: Arc::new(RenderCache::default()),
            layout: Vec::new(),
            table: color_table::TableState::default(),
            color_dialog: None,
            size_dialog: None,
            size_job: None,
            message: None,
            palette_io: Arc::new(std::sync::Mutex::new(Vec::new())),
            feedback_tex: None,
            pending_file: None,
        }
    }

    /// Bildpunkt (Pixel im angezeigten Bild) unter dem Bildschirmpunkt `p` im Feld `i`.
    pub fn pixel_at(&self, i: usize, lay: &pane::PaneLayout, p: egui::Pos2) -> Option<(i64, i64)> {
        let img = self.panes[i].result.as_ref()?.image.as_ref()?;
        let sz = vec2(img.width() as f32, img.height() as f32);
        let c = self.center.unwrap_or(sz / 2.0);
        let q = (p - (lay.image.center() - c * self.zoom())) / self.zoom();
        (lay.image.contains(p) && q.x >= 0.0 && q.y >= 0.0).then(|| (q.x.floor() as i64, q.y.floor() as i64))
    }

    /// Klick mit Zeiger bzw. Pipette in das Feld `i` an `p`: Zeiger mit gewählter Farbe wählt den
    /// Paletteneintrag des Bildpunkts (§2.5.4), die Pipetten setzen Transparenz (§2.6). `true`, wenn
    /// der Klick etwas bewirkt hat.
    pub fn click_image(&mut self, i: usize, lay: &pane::PaneLayout, p: egui::Pos2) -> bool {
        let Some((x, y)) = self.pixel_at(i, lay, p) else { return false };
        let Some(res) = self.panes[i].result.clone() else { return false };
        let Some(k) = res.index_at(x, y) else { return false };
        let e = res.entries[k];
        match self.tool {
            Tool::Pointer if !self.table.selected.is_empty() && i == self.active => {
                self.table.selected = vec![e.color];
                true
            }
            Tool::Pipette(kind) => {
                color_table::apply_pipette(kind, &mut self.panes[i].settings, e.color);
                true
            }
            _ => false,
        }
    }

    /// Einstellung des aktiven Feldes.
    pub fn active_settings(&self) -> &OptimizeSettings {
        &self.panes[self.active].settings
    }

    /// Zoomfaktor (Bildschirmpunkte je Bildpixel).
    pub fn zoom(&self) -> f32 {
        toolbar::ZOOMS[self.zoom_idx.min(toolbar::ZOOMS.len() - 1)].1
    }

    /// Textur des Originals (Felder mit „Vorschau“ aus).
    pub fn original_texture_id(&self) -> Option<egui::TextureId> {
        self.original_tex.as_ref().map(|(_, t)| t.id())
    }

    /// Wird in einem sichtbaren Feld noch gerechnet?
    pub fn is_busy(&self) -> bool {
        self.mode.panes().iter().any(|&i| self.panes[i].is_busy()) || self.original_job.is_some() || self.size_job.is_some()
    }

    /// 1/2/4 Felder (§5.1): neu sichtbare Felder übernehmen die Einstellung des aktiven Feldes;
    /// aktiv wird das zuletzt hinzugekommene (2-fach: unten, 4-fach: unten rechts). Ausgeblendete
    /// Felder behalten ihre Einstellung, solange der Dialog offen ist.
    pub fn set_mode(&mut self, mode: ViewMode) {
        if mode == self.mode {
            return;
        }
        let grow = mode.panes().len() > self.mode.panes().len();
        let src = self.panes[self.active].settings.clone();
        let last = self.panes[self.active].last_preset.clone();
        for &i in mode.panes() {
            if !self.panes[i].initialized {
                let p = &mut self.panes[i];
                p.settings = src.clone();
                p.last_preset = last.clone();
                p.initialized = true;
            }
        }
        self.mode = mode;
        if grow {
            self.active = *mode.panes().last().unwrap_or(&0);
        } else if !mode.panes().contains(&self.active) {
            // im 2-fach-Modus bleibt die Zeile erhalten (oben → 0, unten → 1)
            self.active = if mode == ViewMode::Two { self.active % 2 } else { 0 };
        }
    }

    /// Zoomstufe setzen; mit `anchor` (Bildschirmpunkt, Mitte der Bildfläche) bleibt der Punkt
    /// unter dem Zeiger stehen.
    pub fn set_zoom(&mut self, idx: usize, anchor: Option<(egui::Pos2, egui::Pos2)>) {
        let idx = idx.min(toolbar::ZOOMS.len() - 1);
        if let (Some((p, c)), Some(center)) = (anchor, self.center) {
            let (old, new) = (self.zoom(), toolbar::ZOOMS[idx].1);
            let img = center + (p - c) / old;
            self.center = Some(img - (p - c) / new);
        }
        self.zoom_idx = idx;
    }

    /// Ausschnitt verschieben (alle Felder): `delta` in Bildschirmpunkten.
    pub fn pan(&mut self, delta: Vec2) {
        if let Some(c) = self.center {
            self.center = Some(c - delta / self.zoom());
        }
    }

    /// Bildpunkt in der Feldmitte begrenzen: kleinere Bilder bleiben mittig, größere lassen sich
    /// höchstens bis an ihren Rand schieben (kein Bildlauf ins Leere).
    fn clamp_center(&mut self, img: Vec2, view: Vec2) {
        let z = self.zoom();
        let c = self.center.unwrap_or(img / 2.0);
        let axis = |c: f32, img: f32, view: f32| {
            let half = view / 2.0 / z;
            if img * z <= view { img / 2.0 } else { c.clamp(half, img - half) }
        };
        self.center = Some(vec2(axis(c.x, img.x, view.x), axis(c.y, img.y, view.y)));
    }

    fn update_jobs(&mut self, ctx: &egui::Context, base: &JobInput) {
        for &i in self.mode.panes() {
            self.panes[i].update(ctx, base, &self.cache);
        }
        let need_orig = self.mode.panes().iter().any(|&i| !self.panes[i].show_preview);
        let key = worker::settings_key(base.doc_key, base.page, base.frame, &JobKind::Original);
        if let Some(j) = &self.original_job
            && let Some(r) = j.poll()
        {
            if let Some(r) = r
                && r.key == j.key
            {
                self.original = Some(Arc::new(r));
            }
            self.original_job = None;
        }
        let have = self.original.as_ref().is_some_and(|r| r.key == key) || self.original_job.as_ref().is_some_and(|j| j.key == key);
        if need_orig && !have {
            self.original_job = Some(worker::spawn(JobInput { kind: JobKind::Original, key, ..base.clone() }, Arc::clone(&self.cache), ctx));
        }
    }
}

/// Felder in der Vorschaufläche: 2-fach übereinander, 4-fach 2 × 2, je 2 px Abstand (§5.1).
pub fn pane_rects(area: Rect, mode: ViewMode) -> Vec<(usize, Rect)> {
    let gap = 2.0;
    let (w2, h2) = ((area.width() - gap) / 2.0, (area.height() - gap) / 2.0);
    let top = Rect::from_min_size(area.min, vec2(area.width(), h2));
    let bottom = Rect::from_min_max(pos2(area.left(), area.top() + h2 + gap), area.max);
    match mode {
        ViewMode::One => vec![(0, area)],
        ViewMode::Two => vec![(0, top), (1, bottom)],
        ViewMode::Four => vec![
            (0, Rect::from_min_size(area.min, vec2(w2, h2))),
            (1, Rect::from_min_max(pos2(area.left(), area.top() + h2 + gap), pos2(area.left() + w2, area.bottom()))),
            (2, Rect::from_min_max(pos2(area.left() + w2 + gap, area.top()), pos2(area.right(), area.top() + h2))),
            (3, Rect::from_min_max(pos2(area.left() + w2 + gap, area.top() + h2 + gap), area.max)),
        ],
    }
}

/// Eigene Exportvorgabe speichern (wie „Einstellungen speichern“ des Optimieren-Bedienfelds, aber
/// mit der Einstellung des Feldes).
fn save_user_preset(st: &mut AppState, name: &str, s: &OptimizeSettings) {
    use fp_engine::PrefValue;
    let key = crate::ui_commands::EXPORT_PRESETS_KEY;
    let base = st.session.active().and_then(|d| d.doc.pages.get(d.page).and_then(|p| p.root.export.as_deref().cloned())).unwrap_or_default();
    let json = match serde_json::to_string(&s.to_model(&base)) {
        Ok(j) => j,
        Err(e) => {
            st.push(Action::Status(fp_i18n::tr!("panel-opt-settings-error", error = e.to_string())));
            return;
        }
    };
    let mut m = match st.session.prefs.get(key) {
        Some(PrefValue::Dict(m)) => m,
        _ => Default::default(),
    };
    m.insert(name.to_string(), PrefValue::Str(json));
    st.session.prefs.set(key, PrefValue::Dict(m));
    st.session.prefs_changed();
}

/// Zeigt die Bildvorschau (und darüber ggf. „Vorgabenname“).
pub fn show(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut d) = st.dialogs.preview.take() else { return };
    let Some(doc) = st.session.active() else { return };
    let base = JobInput {
        doc: Arc::clone(&doc.doc),
        renderer: Arc::clone(&st.renderer),
        page: doc.page,
        frame: doc.frame,
        doc_key: (doc.id.0, doc.generation),
        kind: JobKind::Original,
        key: 0,
    };
    poll_palette_io(&mut d);
    poll_size_job(&mut d);
    let outcome = draw(ctx, st, &mut d);
    d.update_jobs(ctx, &base);
    preset_name_dialog(ctx, st, &mut d);
    sub_dialogs(ctx, &mut d, &base);
    match outcome {
        Some(Outcome::Ok) => optimize::apply_settings(st, &d.panes[d.active].settings.clone()),
        Some(Outcome::Export) => export_with(st, d.panes[d.active].settings.clone()),
        Some(Outcome::Cancel) => {}
        None => st.dialogs.preview = Some(d),
    }
}

/// Ergebnisse von „Palette laden/speichern“ übernehmen.
fn poll_palette_io(d: &mut ImagePreview) {
    let events: Vec<PaletteIo> = d.palette_io.lock().map(|mut v| std::mem::take(&mut *v)).unwrap_or_default();
    for e in events {
        match e {
            PaletteIo::Loaded { pane, colors } => {
                // nach „Palette laden“ steht die Palettenliste auf „Benutzerdefiniert“ (§2.7)
                if let Some(p) = d.panes.get_mut(pane) {
                    fp_export::palette_edit::load_palette(&mut p.settings, &colors);
                }
                d.table.selected.clear();
            }
            PaletteIo::Error(msg) => d.message = Some(fp_i18n::tr!("image-preview-palette-error", error = msg)),
        }
    }
}

const PALETTE_OPEN: &[(&str, &[&str])] = &[("dlg-filter-act", &["act"]), ("dlg-filter-ase", &["ase"]), ("dlg-filter-aco", &["aco"]), ("GIF (*.gif)", &["gif"])];
const PALETTE_SAVE: &[(&str, &[&str])] = &[("dlg-filter-act", &["act"]), ("dlg-filter-ase", &["ase"]), ("dlg-filter-aco", &["aco"])];

/// „Palette laden…“: Farbtabelle (ACT, ASE, ACO, GIF) wählen; die Farben werden die
/// benutzerdefinierte Palette des Feldes `pane`.
pub(crate) fn load_palette(st: &mut AppState, d: &ImagePreview, pane: usize) {
    let slot = Arc::clone(&d.palette_io);
    super::panel_menus::pick_open(st, &t("dlg-pmenu-load-palette"), PALETTE_OPEN, move |_st, p| {
        let ev = match fp_engine::commands::panel::read_swatch_file(&p.to_string_lossy()) {
            Ok(cols) if !cols.is_empty() => PaletteIo::Loaded { pane, colors: cols.iter().take(256).map(|c| [c.r(), c.g(), c.b()]).collect() },
            Ok(_) => PaletteIo::Error(t("image-preview-palette-empty")),
            Err(e) => PaletteIo::Error(e.to_string()),
        };
        if let Ok(mut v) = slot.lock() {
            v.push(ev);
        }
    });
}

/// „Palette speichern…“: angezeigte Palette des aktiven Feldes als ACT/ASE/ACO.
pub(crate) fn save_palette(st: &mut AppState, d: &ImagePreview, entries: &[fp_export::palette_edit::PaletteEntry]) {
    let slot = Arc::clone(&d.palette_io);
    let colors: Vec<fp_color::Argb> = entries.iter().map(|e| fp_color::Argb(e.shown | 0xFF00_0000)).collect();
    let name = format!("{}.act", t("dlg-pmenu-palette-file"));
    super::panel_menus::pick_save(st, &t("dlg-pmenu-save-palette"), &name, PALETTE_SAVE, move |_st, p| {
        let ext = p.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        let bytes = fp_engine::commands::panel::swatch_file_bytes(&ext, &colors);
        if let Err(e) = std::fs::write(&p, bytes)
            && let Ok(mut v) = slot.lock()
        {
            v.push(PaletteIo::Error(e.to_string()));
        }
    });
}

/// Dialoge über der Bildvorschau: „Farbe“, „Für Dateigröße optimieren“, Meldung; Ergebnis der Suche.
fn sub_dialogs(ctx: &egui::Context, d: &mut ImagePreview, base: &JobInput) {
    if let Some(cd) = d.color_dialog.take() {
        match color_table::color_dialog(ctx, cd) {
            color_table::ColorOutcome::Open(cd) => d.color_dialog = Some(cd),
            color_table::ColorOutcome::Ok(cd) => {
                if let Some(p) = d.panes.get_mut(cd.pane) {
                    color_table::apply_color_dialog(&cd, &mut p.settings);
                }
            }
            color_table::ColorOutcome::Cancel => {}
        }
    }
    if let Some(sd) = d.size_dialog.take() {
        match optimize_to_size::dialog(ctx, sd) {
            optimize_to_size::SizeDialogOutcome::Open(sd) => d.size_dialog = Some(sd),
            optimize_to_size::SizeDialogOutcome::Ok(pane, target) => {
                let s = d.panes[pane].settings.clone();
                d.size_job = Some(optimize_to_size::SizeJob::spawn(ctx, base, pane, s, target));
            }
            optimize_to_size::SizeDialogOutcome::Cancel => {}
        }
    }
    if let Some(m) = d.message.clone()
        && optimize_to_size::message(ctx, &m)
    {
        d.message = None;
    }
}

/// Ergebnis der Suche „Optimieren auf Größe“ übernehmen (vor dem Starten der Feldaufträge, damit
/// das Feld im selben Bild neu rechnet).
fn poll_size_job(d: &mut ImagePreview) {
    if let Some(job) = &d.size_job
        && let Some(r) = job.poll()
    {
        let (pane, start) = (job.pane, job.start.clone());
        d.size_job = None;
        match r {
            Ok(fp_export::doc_export::SizeOutcome::Found(res)) => {
                // inzwischen selbst geändert: Ergebnis verwerfen
                if d.panes[pane].settings == start {
                    d.panes[pane].settings = res.settings;
                }
            }
            Ok(fp_export::doc_export::SizeOutcome::TooSmall { .. }) => d.message = Some(t("image-preview-size-too-small")),
            Ok(fp_export::doc_export::SizeOutcome::NotApplicable) => {}
            Err(e) if !e.is_empty() => d.message = Some(e),
            Err(_) => {}
        }
    }
}

/// „Exportieren…“: Einstellung des aktiven Feldes übernehmen und exportieren (Verhalten wie bisher;
/// der volle Exportdialog über der Bildvorschau folgt in Paket P4).
fn export_with(st: &mut AppState, settings: OptimizeSettings) {
    optimize::apply_settings(st, &settings);
    let ext = settings.format.extension();
    let name = st.session.active().map(|x| x.title.trim_end_matches(".png").trim_end_matches(".fw").to_string()).unwrap_or_else(|| t("dlg-default-image-name"));
    let req = crate::file_dialog::Request::save(t("dlg-export-title"), format!("{name}.{ext}")).filter(settings.format.display_name(), &[ext]);
    crate::file_dialog::request(st, req, move |st, r| {
        if let Some(path) = r.into_iter().next()
            && let Some(doc) = st.session.active()
        {
            let r = ExportRenderer { normal: Arc::clone(&st.renderer) };
            let msg = match fp_export::doc_export::export_whole(&r, &doc.doc, doc.page, &settings, &fp_export::export::EncodeExtras::default()) {
                Ok(bytes) => match std::fs::write(&path, bytes) {
                    Ok(()) => fp_i18n::tr!("dlg-export-file-done", path = path.display().to_string()),
                    Err(e) => fp_i18n::tr!("dlg-export-failed", error = e.to_string()),
                },
                Err(e) => fp_i18n::tr!("dlg-export-failed", error = e.to_string()),
            };
            st.push(Action::Status(msg));
        }
    });
}

fn draw(ctx: &egui::Context, st: &mut AppState, d: &mut ImagePreview) -> Option<Outcome> {
    let m = options::measure(ctx);
    let (export_l, ok_l, cancel_l) = (t("dlg-export-button"), t("btn-ok"), t("btn-cancel"));
    let (ex_w, ok_w, cancel_w) = (button_w(ctx, &export_l, 90.0), button_w(ctx, &ok_l, 90.0), button_w(ctx, &cancel_l, 75.0));
    let screen = ctx.content_rect();
    let need_w = (8.0 + m.width + 7.0 + MIN_AREA_W.max(toolbar::WIDTH + 3.0) + 8.0).max(8.0 + ex_w + 5.0 + ok_w + 4.0 + cancel_w + 8.0);
    // wie im Original Start- = Mindestgröße; auf kleinen Bildschirmen höchstens so groß wie das Fenster
    let min = vec2(MIN_CLIENT.x.max(need_w).min((screen.width() - 2.0).max(need_w.min(MIN_CLIENT.x))), MIN_CLIENT.y.min(screen.height() - DIALOG_TITLE_H - 2.0).max(420.0)).floor();
    let own = crate::ui_commands::export_presets(&st.session.prefs);
    let entries = pane::preset_entries(&own);
    let mut out = None;
    let r = dialog_frame_resizable(ctx, DIALOG_ID, &t("dlg-preview-title"), min, |ui, body| {
        let theme = Theme::current(ui.ctx());
        let left = Rect::from_min_size(body.min + vec2(8.0, 10.0), vec2(m.width, body.height() - 10.0 - 52.0));
        let area = Rect::from_min_max(pos2(left.right() + 7.0, body.top() + 11.0), pos2(body.right() - 8.0, body.bottom() - 79.0));
        options::show(ui, st, left, m, d);
        draw_panes(ui, d, area, &entries);
        if let Some((i, k)) = d.pending_file.take() {
            if k == color_table::PaletteCmd::LoadPalette {
                load_palette(st, d, i);
            } else {
                let entries = d.panes[i].result.as_ref().map(|r| r.entries.clone()).unwrap_or_default();
                save_palette(st, d, &entries);
            }
        }
        toolbar::show(ui, d, pos2(area.left() + 3.0, body.bottom() - 75.0));
        ui.painter().hline((body.left() + 8.0)..=(body.right() - 8.0), body.bottom() - 41.5, Stroke::new(1.0, theme.colors.line_mid));
        let by = body.bottom() - 31.0;
        let cr = Rect::from_min_size(pos2(body.right() - 8.0 - cancel_w, by), vec2(cancel_w, 23.0));
        let okr = Rect::from_min_size(pos2(cr.left() - 4.0 - ok_w, by), vec2(ok_w, 23.0));
        let exr = Rect::from_min_size(pos2(okr.left() - 5.0 - ex_w, by), vec2(ex_w, 23.0));
        if push_button(ui, exr, &export_l, false, true) {
            out = Some(Outcome::Export);
        }
        if push_button(ui, okr, &ok_l, true, true) {
            out = Some(Outcome::Ok);
        }
        if push_button(ui, cr, &cancel_l, false, true) {
            out = Some(Outcome::Cancel);
        }
    });
    if out.is_none() && r == DialogClose::Cancelled {
        out = Some(Outcome::Cancel);
    }
    out
}

/// Felder zeichnen und bedienen.
fn draw_panes(ui: &mut egui::Ui, d: &mut ImagePreview, area: Rect, entries: &[pane::PresetEntry]) {
    let rects = pane_rects(area, d.mode);
    let lays: Vec<(usize, pane::PaneLayout)> = rects.iter().map(|&(i, r)| (i, pane::PaneLayout::new(r, i, d.mode))).collect();
    // gemeinsamer Ausschnitt: Bezug ist das Bild des aktiven Feldes
    let img_size = |p: &Pane, orig: &Option<Arc<PaneResult>>| -> Option<Vec2> {
        let r = if p.show_preview { p.result.as_ref() } else { orig.as_ref() };
        r.and_then(|r| r.image.as_ref()).map(|i| vec2(i.width() as f32, i.height() as f32))
    };
    let view = lays.iter().map(|(_, l)| l.image.size()).fold(vec2(f32::MAX, f32::MAX), |a, b| a.min(b));
    if let Some(sz) = img_size(&d.panes[d.active], &d.original).or_else(|| lays.iter().find_map(|(i, _)| img_size(&d.panes[*i], &d.original))) {
        d.clamp_center(sz, view);
    }
    d.layout = lays.clone();
    // Alt aus dem Zustand oder aus dem Klick selbst (Taste und Klick im selben Bild)
    let alt = ui.input(|i| i.modifiers.alt || i.events.iter().any(|e| matches!(e, egui::Event::PointerButton { modifiers, .. } if modifiers.alt)));
    for (i, lay) in &lays {
        let i = *i;
        let resp = ui.interact(lay.frame, Id::new(("fp-ipv-pane", i)), Sense::click_and_drag());
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, true, fp_i18n::tr!("image-preview-pane", n = i as i64 + 1)));
        if resp.is_pointer_button_down_on() || resp.clicked() {
            d.active = i;
        }
        let on_image = resp.hover_pos().is_some_and(|p| lay.image.contains(p));
        // Klick ins Bild: Zeiger mit gewählter Farbe wählt den Eintrag des Bildpunkts, Pipetten
        if resp.clicked()
            && let Some(p) = resp.interact_pointer_pos()
        {
            d.click_image(i, lay, p);
        }
        // Rechtsklick in die Vorschau: Palettenmenü (§2.5.4), wenn das Feld eine Farbtabelle hat
        if i == d.active && options::visibility(&d.panes[i].settings).color_table.shown() {
            let entries = d.panes[i].result.as_ref().map(|r| r.entries.clone()).unwrap_or_default();
            let s = &d.panes[i].settings;
            let sel = d.table.selected_entries(&entries);
            let ms = color_table::MenuState {
                has_selection: !sel.is_empty(),
                any_locked: fp_export::palette_edit::any_locked(s),
                selection_edited: sel.iter().any(|(_, e)| e.edited),
                any_edited: fp_export::palette_edit::any_edited(s),
                sorting: s.sorting,
                feedback: d.table.feedback,
            };
            if let Some(k) = color_table::context_menu(&resp, ms) {
                match color_table::apply(k, &mut d.panes[i].settings, &entries, &mut d.table) {
                    color_table::FollowUp::ColorDialog(mode) => {
                        let start = match mode {
                            color_table::ColorMode::Edit(o) => entries.iter().find(|e| e.color & 0xFF_FFFF == o & 0xFF_FFFF).map(|e| e.shown).unwrap_or(o),
                            color_table::ColorMode::Add => 0xFF80_8080,
                        };
                        d.color_dialog = Some(color_table::ColorDialog::new(i, mode, start));
                    }
                    color_table::FollowUp::LoadPalette | color_table::FollowUp::SavePalette => d.pending_file = Some((i, k)),
                    color_table::FollowUp::None => {}
                }
            }
        }
        match d.tool {
            Tool::Pipette(_) => {
                if on_image {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                }
            }
            Tool::Pointer => {
                // der gegriffene Bildpunkt bleibt unter dem Zeiger (ab dem Drücken gerechnet, auch
                // die Bewegung, bevor egui das Ziehen erkennt)
                if resp.drag_started() {
                    let origin = ui.input(|inp| inp.pointer.press_origin());
                    d.grab = origin.zip(d.center);
                }
                if resp.dragged() {
                    if let (Some((o, c0)), Some(p)) = (d.grab, ui.input(|inp| inp.pointer.latest_pos())) {
                        d.center = Some(c0 - (p - o) / d.zoom());
                    } else {
                        d.pan(resp.drag_delta());
                    }
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                } else if on_image {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                }
            }
            Tool::Zoom => {
                if on_image {
                    ui.ctx().set_cursor_icon(if alt { egui::CursorIcon::ZoomOut } else { egui::CursorIcon::ZoomIn });
                }
                if resp.clicked()
                    && let Some(p) = resp.interact_pointer_pos()
                {
                    let idx = if alt { d.zoom_idx.saturating_sub(1) } else { (d.zoom_idx + 1).min(toolbar::ZOOMS.len() - 1) };
                    d.set_zoom(idx, Some((p, lay.image.center())));
                }
            }
        }
    }
    let z = d.zoom();
    let center = d.center;
    for (i, lay) in &lays {
        let i = *i;
        let show = d.panes[i].show_preview;
        let (res, tex) = if show {
            let res = d.panes[i].result.clone();
            let tex = res.as_ref().and_then(|r| d.panes[i].texture(ui.ctx(), r));
            (res, tex)
        } else {
            let res = d.original.clone();
            let tex = match (&res, &d.original_tex) {
                (Some(r), Some((k, t))) if *k == r.key => Some(t.clone()),
                (Some(r), _) => r.image.as_deref().and_then(|img| pane::upload(ui.ctx(), img)).inspect(|t| d.original_tex = Some((r.key, t.clone()))),
                _ => None,
            };
            (res, tex)
        };
        let img_rect = res.as_ref().and_then(|r| r.image.as_ref()).map(|img| {
            let sz = vec2(img.width() as f32, img.height() as f32);
            let c = center.unwrap_or(sz / 2.0);
            Rect::from_min_size(lay.image.center() - c * z, sz * z)
        });
        pane::paint_image(ui, lay.image, img_rect, tex.as_ref());
        // Farbfeld-Rückmeldung: Pixel der gewählten Farben im aktiven Feld hervorheben (§2.7)
        if show
            && i == d.active
            && d.table.feedback
            && let (Some(r), Some(ir)) = (res.as_ref(), img_rect)
        {
            let sel: Vec<usize> = d.table.selected_entries(&r.entries).into_iter().map(|(k, _)| k).collect();
            if !sel.is_empty() {
                let cached = d.feedback_tex.as_ref().filter(|(k, s, _)| *k == r.key && *s == sel).map(|(_, _, t)| t.clone());
                let tex = cached.or_else(|| {
                    let img = r.image.as_ref()?;
                    let col = crate::theme::with_alpha(Theme::current(ui.ctx()).colors.focus, 200);
                    let ci = color_table::feedback_image(&r.indices, img.width(), img.height(), &sel, col)?;
                    let t = ui.ctx().load_texture("fp-ipv-feedback", ci, egui::TextureOptions::NEAREST);
                    d.feedback_tex = Some((r.key, sel.clone(), t.clone()));
                    Some(t)
                });
                if let Some(t) = tex {
                    ui.painter().with_clip_rect(lay.image.intersect(ui.clip_rect())).image(t.id(), ir, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), egui::Color32::WHITE);
                }
            }
        }
        pane::paint_frame(ui, lay.frame, d.active == i);
        let busy = d.panes[i].is_busy();
        let act = pane::header(ui, i, lay, &d.panes[i], entries, busy);
        if act.touched {
            d.active = i;
        }
        if act.toggle_preview {
            d.panes[i].show_preview = !d.panes[i].show_preview;
        }
        if let Some(k) = act.preset
            && let Some(e) = entries.get(k)
            && let Some(s) = &e.settings
        {
            d.panes[i].settings = pane::load_preset(s, &d.panes[i].settings);
            d.panes[i].last_preset = Some(e.label.clone());
        }
        if act.save_preset {
            let p = &d.panes[i];
            let name = pane::matching_preset(entries, &p.settings).map(|k| entries[k].label.clone()).or_else(|| p.last_preset.clone()).unwrap_or_else(|| optimize::settings_label(&p.settings));
            d.preset_name = Some((i, name));
        }
    }
}

/// Dialog „Vorgabenname“ (§2.8): Name vorbelegt und markiert, OK speichert die Einstellung des
/// Feldes als eigene Vorgabe.
fn preset_name_dialog(ctx: &egui::Context, st: &mut AppState, d: &mut ImagePreview) {
    let Some((i, mut name)) = d.preset_name.clone() else { return };
    let (title, name_l, ok_l, cancel_l) = (t("image-preview-preset-name-title"), t("image-preview-preset-name-label"), t("btn-ok"), t("btn-cancel"));
    let dx = super::extra_w(ctx, &[&name_l], 50.0);
    let mut result = None;
    let r = dialog_frame(ctx, PRESET_DIALOG_ID, &title, vec2(340.0 + dx, 110.0), |ui, body| {
        super::put_left(ui, Rect::from_min_size(body.min + vec2(16.0, 18.0), vec2(54.0 + dx, 20.0)), egui::Label::new(name_l.as_str()).extend());
        let te = ui.put(Rect::from_min_size(body.min + vec2(70.0 + dx, 14.0), vec2(250.0, 22.0)), egui::TextEdit::singleline(&mut name));
        super::autofocus_field(ui, &te, name.chars().count());
        // einziges Eingabefeld: behält den Fokus (wie „Arbeitsbereich speichern“)
        te.request_focus();
        let b = super::button_row(ui.ctx(), body.right() - 16.0, body.bottom() - 34.0, &[&ok_l, &cancel_l], 76.0, 23.0, 8.0);
        let ok = push_button(ui, b[0], &ok_l, true, !name.trim().is_empty());
        if ok || (ui.input(|inp| inp.key_pressed(egui::Key::Enter)) && !name.trim().is_empty()) {
            result = Some(true);
        }
        if push_button(ui, b[1], &cancel_l, false, true) {
            result = Some(false);
        }
    });
    match (result, r) {
        (Some(true), _) => {
            let n = name.trim().to_string();
            save_user_preset(st, &n, &d.panes[i].settings);
            d.panes[i].last_preset = Some(n);
            d.preset_name = None;
        }
        (Some(false), _) | (_, DialogClose::Cancelled) => d.preset_name = None,
        _ => d.preset_name = Some((i, name)),
    }
}
