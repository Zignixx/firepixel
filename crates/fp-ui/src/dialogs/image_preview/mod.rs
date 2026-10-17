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
//! - Werkzeugleiste unten ([`toolbar`], §6) mit Werkzeug „Exportbereich“ ([`crop`]) und
//!   Abspielknöpfen; gemeinsamer Status aller Felder ([`ImagePreview::frame`], beim Öffnen Status 1
//!   wie der Export). Registerkarten Datei ([`file_tab`]) und Animation ([`animation_tab`]).
//! - Rechnen **im Hintergrund** je Feld mit Abbruch ([`worker`]); die Vorschau kodiert mit
//!   denselben Kodierern und Parametern wie der echte Export.
//! - Fenster in der Größe veränderbar, Start- = Mindestgröße (§1.1); Größe, Modus und
//!   Feldeinstellungen werden wie im Original **nicht** gemerkt (§7).
//! - Paket P4 (§7): OK bzw. „Exportieren…“ schreiben die Einstellung des aktiven Feldes in einem
//!   Verlaufsschritt in die Seite ([`commit_settings`]); „Exportieren…“ öffnet darüber den
//!   Exportdialog, nach dessen Schließen geht es in der Bildvorschau weiter. Esc/Abbrechen ändert
//!   nichts, Esc in einem Unterdialog schließt nur diesen, Esc beim Ziehen des Exportbereichs bricht
//!   nur das Ziehen ab. Mausrad wie im Dokumentfenster (`wheel`).

pub mod animation_tab;
pub mod color_table;
pub mod crop;
pub mod file_tab;
pub mod optimize_to_size;
pub mod options;
pub mod pane;
pub mod toolbar;
pub mod worker;

pub use pane::Pane;

use super::{
    DIALOG_TITLE_H, DialogClose, button_w, dialog_frame, dialog_frame_resizable, push_button,
};
use crate::app_state::{Action, AppState};
use crate::panels::optimize;
use crate::theme::Theme;
use egui::{Id, Rect, Sense, Stroke, Vec2, pos2, vec2};
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
    /// Exportbereich (§6): gestrichelter Rahmen mit 8 Griffen im aktiven Feld; Ziehen an Griffen
    /// oder Kanten setzt den Exportbereich dieses Feldes. Das Feld zeigt dabei das ganze Bild.
    ExportArea,
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
    /// Gemeinsamer Punkt in der Feldmitte in **Dokumentpixeln** der Seite (N25: Felder mit
    /// verschiedenem Exportbereich, verschiedener Skalierung oder „Vorschau“ aus zentrieren auf
    /// denselben Dokumentpunkt); `None` = Mitte des Bildes im aktiven Feld.
    pub center: Option<Vec2>,
    /// Ausschnitt vom Benutzer gesetzt (Ziehen, Rad, Zoom am Zeiger). Solange `false`, liegt ein
    /// Bild, das größer als das Feld ist, oben links bündig – auch nach dem Umschalten 1 → 2 → 4
    /// Felder (re 28 D4: „In beiden Fällen begann die Anzeige oben links“).
    pub user_view: bool,
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
    /// Gemeinsamer Status aller Felder (0 = Status 1; beim Öffnen Status 1 wie der Export, §6).
    pub frame: usize,
    /// Anzahl Status der Seite.
    pub frames: usize,
    /// Abspielen läuft (Knopf ▶/■).
    pub playing: bool,
    /// Zeitpunkt (Zeit der Oberfläche) des nächsten Statuswechsels beim Abspielen.
    pub next_tick: Option<f64>,
    /// Größe der Seite in Dokumentpixeln (Bezug für Skalieren und Exportbereich).
    pub doc_size: (usize, usize),
    /// Laufendes Ziehen mit dem Werkzeug „Exportbereich“: Feld, Griff, Bereich und Druckpunkt beim
    /// Drücken, dazu der Exportbereich vor dem Ziehen (Esc stellt ihn wieder her).
    crop_drag: Option<(usize, crop::Handle, [i32; 4], egui::Pos2, Option<[i32; 4]>)>,
    /// Strg+Mausrad: angesammelte Rastungen (eine Zoomstufe je volle Rastung).
    wheel_acc: f32,
    /// „Exportieren…“ hat den Exportdialog über der Bildvorschau geöffnet; nach seinem Schließen
    /// geht es in der Bildvorschau weiter (§7: Abbruch dort → zurück in die Bildvorschau).
    pub export_child: bool,
    /// Zähler erfolgreicher Exporte (`Dialogs::exported`) beim Öffnen des Exportdialogs: ist er
    /// danach höher, schließt die Bildvorschau wie mit OK (re 28 E7).
    pub export_mark: u64,
    /// Bildvorschau nur für ein Segment (TW-44, Bearbeiten-Symbol im Segment-Inspektor): Felder
    /// zeigen nur den Bereich des Segments, OK schreibt die Einstellung in das Segment
    /// (`exportSettings`) statt in die Seite.
    pub slice: Option<fp_model::ElementId>,
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
        Some(ImagePreview::with_settings(
            s,
            &crate::ui_commands::export_presets(&st.session.prefs),
        ))
    }

    /// Bildvorschau nur für das Segment `id` (TW-44): Einstellung des Segments (ohne eigene die der
    /// Seite), Exportbereich = Rechteck des Segments.
    pub fn for_slice(st: &AppState, id: fp_model::ElementId) -> Option<ImagePreview> {
        let d = st.session.active()?;
        let e = fp_engine::docops::element_on_page(&d.doc, d.page, id)?;
        let fp_model::ElementKind::WebObject(w) = &e.kind else {
            return None;
        };
        let sp = w.slice.as_ref()?;
        let b = fp_engine::docops::element_bounds(&d.doc, &e)?;
        let mut s = sp
            .export
            .as_deref()
            .map(|x| OptimizeSettings::from_model(x, None))
            .unwrap_or_else(|| fp_export::doc_export::page_settings(&d.doc, d.page));
        s.crop = Some([
            b.left.floor() as i32,
            b.top.floor() as i32,
            b.right.ceil() as i32,
            b.bottom.ceil() as i32,
        ]);
        let mut p =
            ImagePreview::with_settings(s, &crate::ui_commands::export_presets(&st.session.prefs));
        p.slice = Some(id);
        Some(p)
    }

    /// Wie [`ImagePreview::new`] mit gegebener Einstellung.
    pub fn with_settings(
        s: OptimizeSettings,
        own: &[(String, fp_model::ExportSettings)],
    ) -> ImagePreview {
        let mut panes = [
            Pane::new(s.clone()),
            Pane::new(s.clone()),
            Pane::new(s.clone()),
            Pane::new(s),
        ];
        panes[0].initialized = true;
        let entries = pane::preset_entries(own);
        panes[0].last_preset =
            pane::matching_preset(&entries, &panes[0].settings).map(|i| entries[i].label.clone());
        ImagePreview {
            mode: ViewMode::One,
            panes,
            active: 0,
            zoom_idx: toolbar::ZOOM_100,
            center: None,
            user_view: false,
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
            frame: 0,
            frames: 1,
            playing: false,
            next_tick: None,
            doc_size: (1, 1),
            crop_drag: None,
            wheel_acc: 0.0,
            export_child: false,
            export_mark: 0,
            slice: None,
        }
    }

    /// Abbildung Dokument → angezeigtes Bild des Feldes `i`: Ursprung des Bildes in Dokumentpixeln,
    /// Maßstab (Bildpixel je Dokumentpixel) und Bildgröße. Felder mit „Vorschau“ aus zeigen das
    /// Original (Ursprung 0, Maßstab 1); sonst gelten Exportbereich und Skalierung des Feldes.
    pub fn pane_map(&self, i: usize) -> Option<(Vec2, Vec2, Vec2)> {
        let p = &self.panes[i];
        let res = if p.show_preview {
            p.result.as_ref()
        } else {
            self.original.as_ref()
        };
        let img = res?.image.as_ref()?;
        let size = vec2(img.width() as f32, img.height() as f32);
        if !p.show_preview {
            return Some((Vec2::ZERO, Vec2::splat(1.0), size));
        }
        let s = &p.settings;
        let (fx, fy) = crop::factors(self.doc_size, s);
        // das ganze Bild (Werkzeug „Exportbereich“ im aktiven Feld) beginnt am Seitenursprung
        let origin = if (img.width(), img.height()) == crop::scaled_size(self.doc_size, s) {
            Vec2::ZERO
        } else {
            let a = crop::area_doc(self.doc_size, s);
            vec2(a[0] as f32, a[1] as f32)
        };
        Some((origin, vec2(fx as f32, fy as f32), size))
    }

    /// Maßstab (Bildpixel je Dokumentpixel) des Feldes `i`; 1, solange es kein Bild hat.
    fn pane_scale(&self, i: usize) -> Vec2 {
        self.pane_map(i)
            .map(|(_, f, _)| f)
            .filter(|f| f.x > 0.0 && f.y > 0.0)
            .unwrap_or(Vec2::splat(1.0))
    }

    /// Gemeinsamer Dokumentpunkt in der Feldmitte (ohne Vorgabe: Mitte des Bildes im aktiven Feld).
    pub fn center_doc(&self) -> Vec2 {
        self.center
            .unwrap_or_else(|| match self.pane_map(self.active) {
                Some((o, f, sz)) if f.x > 0.0 && f.y > 0.0 => o + sz / f / 2.0,
                _ => vec2(self.doc_size.0 as f32, self.doc_size.1 as f32) / 2.0,
            })
    }

    /// Bildschirmlage des angezeigten Bildes im Feld `i` (gemeinsamer Zoom und Mittelpunkt).
    pub fn image_rect(&self, i: usize, lay: &pane::PaneLayout) -> Option<Rect> {
        let (o, f, sz) = self.pane_map(i)?;
        let z = self.zoom();
        let c_img = (self.center_doc() - o) * f;
        Some(Rect::from_min_size(lay.image.center() - c_img * z, sz * z))
    }

    /// Bildpunkt (Pixel im angezeigten Bild) unter dem Bildschirmpunkt `p` im Feld `i`.
    pub fn pixel_at(&self, i: usize, lay: &pane::PaneLayout, p: egui::Pos2) -> Option<(i64, i64)> {
        self.panes[i].result.as_ref()?.image.as_ref()?;
        let r = self.image_rect(i, lay)?;
        let q = (p - r.min) / self.zoom();
        (lay.image.contains(p) && q.x >= 0.0 && q.y >= 0.0)
            .then(|| (q.x.floor() as i64, q.y.floor() as i64))
    }

    /// Klick mit Zeiger bzw. Pipette in das Feld `i` an `p`: Zeiger mit gewählter Farbe wählt den
    /// Paletteneintrag des Bildpunkts (§2.5.4), die Pipetten setzen Transparenz (§2.6). `true`, wenn
    /// der Klick etwas bewirkt hat.
    pub fn click_image(&mut self, i: usize, lay: &pane::PaneLayout, p: egui::Pos2) -> bool {
        let Some((x, y)) = self.pixel_at(i, lay, p) else {
            return false;
        };
        let Some(res) = self.panes[i].result.clone() else {
            return false;
        };
        let Some(k) = res.index_at(x, y) else {
            return false;
        };
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

    /// Lage des ganzen, nicht zugeschnittenen Bildes des Feldes `i` auf dem Bildschirm, solange das
    /// Feld es zeigt (Werkzeug „Exportbereich“); sonst `None`.
    pub fn full_image_rect(&self, i: usize, lay: &pane::PaneLayout) -> Option<Rect> {
        let p = &self.panes[i];
        let img = p.result.as_ref()?.image.as_ref()?;
        if (img.width(), img.height()) != crop::scaled_size(self.doc_size, &p.settings) {
            return None;
        }
        let sz = vec2(img.width() as f32, img.height() as f32);
        let (fx, fy) = crop::factors(self.doc_size, &p.settings);
        let c_img = self.center_doc() * vec2(fx as f32, fy as f32);
        Some(Rect::from_min_size(
            lay.image.center() - c_img * self.zoom(),
            sz * self.zoom(),
        ))
    }

    /// Rahmen des Exportbereichs des Feldes `i` auf dem Bildschirm (nur mit dem Werkzeug
    /// „Exportbereich“ im aktiven Feld).
    pub fn export_area_rect(&self, i: usize, lay: &pane::PaneLayout) -> Option<Rect> {
        if self.tool != Tool::ExportArea || i != self.active {
            return None;
        }
        let ir = self.full_image_rect(i, lay)?;
        let s = &self.panes[i].settings;
        Some(crop::screen_rect(
            crop::area_doc(self.doc_size, s),
            ir,
            self.doc_size,
            s,
            self.zoom(),
        ))
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
        self.mode.panes().iter().any(|&i| self.panes[i].is_busy())
            || self.original_job.is_some()
            || self.size_job.is_some()
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
            self.active = if mode == ViewMode::Two {
                self.active % 2
            } else {
                0
            };
        }
    }

    /// Zoomstufe setzen; mit `anchor` (Bildschirmpunkt, Mitte der Bildfläche) bleibt der Punkt
    /// unter dem Zeiger stehen.
    pub fn set_zoom(&mut self, idx: usize, anchor: Option<(egui::Pos2, egui::Pos2)>) {
        self.set_zoom_in(idx, anchor, self.active);
    }

    /// Wie [`ImagePreview::set_zoom`], der Anker liegt im Feld `pane`.
    fn set_zoom_in(&mut self, idx: usize, anchor: Option<(egui::Pos2, egui::Pos2)>, pane: usize) {
        let idx = idx.min(toolbar::ZOOMS.len() - 1);
        if let Some((p, c)) = anchor {
            let f = self.pane_scale(pane);
            let center = self.center_doc();
            let (old, new) = (self.zoom(), toolbar::ZOOMS[idx].1);
            let doc = center + (p - c) / (old * f);
            self.center = Some(doc - (p - c) / (new * f));
            self.user_view = true;
        }
        self.zoom_idx = idx;
    }

    /// Ausschnitt verschieben (alle Felder): `delta` in Bildschirmpunkten.
    pub fn pan(&mut self, delta: Vec2) {
        self.pan_in(delta, self.active);
    }

    /// Wie [`ImagePreview::pan`] mit dem Maßstab des Feldes `pane`.
    fn pan_in(&mut self, delta: Vec2, pane: usize) {
        let f = self.pane_scale(pane);
        self.center = Some(self.center_doc() - delta / (self.zoom() * f));
        self.user_view = true;
    }

    /// Punkt in der Feldmitte begrenzen (im Bild des aktiven Feldes): kleinere Bilder bleiben
    /// mittig, größere lassen sich höchstens bis an ihren Rand schieben (kein Bildlauf ins Leere).
    /// Ohne Benutzerausschnitt ([`ImagePreview::user_view`]) liegen größere Bilder oben links.
    pub fn clamp_center(&mut self, pane: usize, view: Vec2) {
        let Some((o, f, img)) = self
            .pane_map(pane)
            .filter(|(_, f, _)| f.x > 0.0 && f.y > 0.0)
        else {
            return;
        };
        let z = self.zoom();
        let c = (self.center_doc() - o) * f;
        // all visible panes start at the same document point (no differing export areas): otherwise
        // an anchor at one pane's corner could push another pane's image out of view
        let auto = !self.user_view;
        let top_left = auto
            && self
                .mode
                .panes()
                .iter()
                .all(|&j| self.pane_map(j).is_none_or(|(oj, _, _)| oj == o));
        let axis = |c: f32, img: f32, view: f32| {
            let half = view / 2.0 / z;
            if img * z <= view {
                img / 2.0
            } else if top_left {
                half
            } else if auto {
                img / 2.0
            } else {
                c.clamp(half, img - half)
            }
        };
        let c = vec2(axis(c.x, img.x, view.x), axis(c.y, img.y, view.y));
        self.center = Some(o + c / f);
    }

    fn update_jobs(&mut self, ctx: &egui::Context, base: &JobInput) {
        for &i in self.mode.panes() {
            // Werkzeug „Exportbereich“: das aktive Feld zeigt das ganze Bild (§6)
            let uncropped = self.tool == Tool::ExportArea && i == self.active;
            self.panes[i].update(ctx, base, &self.cache, uncropped);
        }
        let need_orig = self
            .mode
            .panes()
            .iter()
            .any(|&i| !self.panes[i].show_preview);
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
        let have = self.original.as_ref().is_some_and(|r| r.key == key)
            || self.original_job.as_ref().is_some_and(|j| j.key == key);
        if need_orig && !have {
            self.original_job = Some(worker::spawn(
                JobInput {
                    kind: JobKind::Original,
                    key,
                    ..base.clone()
                },
                Arc::clone(&self.cache),
                ctx,
            ));
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
            (
                1,
                Rect::from_min_max(
                    pos2(area.left(), area.top() + h2 + gap),
                    pos2(area.left() + w2, area.bottom()),
                ),
            ),
            (
                2,
                Rect::from_min_max(
                    pos2(area.left() + w2 + gap, area.top()),
                    pos2(area.right(), area.top() + h2),
                ),
            ),
            (
                3,
                Rect::from_min_max(
                    pos2(area.left() + w2 + gap, area.top() + h2 + gap),
                    area.max,
                ),
            ),
        ],
    }
}

/// Eigene Exportvorgabe speichern (wie „Einstellungen speichern“ des Optimieren-Bedienfelds, aber
/// mit der Einstellung des Feldes).
fn save_user_preset(st: &mut AppState, name: &str, s: &OptimizeSettings) {
    use fp_engine::PrefValue;
    let key = crate::ui_commands::EXPORT_PRESETS_KEY;
    let base = st
        .session
        .active()
        .and_then(|d| {
            d.doc
                .pages
                .get(d.page)
                .and_then(|p| p.root.export.as_deref().cloned())
        })
        .unwrap_or_default();
    let json = match serde_json::to_string(&s.to_model(&base)) {
        Ok(j) => j,
        Err(e) => {
            st.push(Action::Status(fp_i18n::tr!(
                "panel-opt-settings-error",
                error = e.to_string()
            )));
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
    let Some(mut d) = st.dialogs.preview.take() else {
        return;
    };
    // der über „Exportieren…“ geöffnete Exportdialog ist zu: weiter in der Bildvorschau
    if d.export_child && st.dialogs.export.is_none() {
        d.export_child = false;
        // re 28 E7: erfolgreich exportiert → die Bildvorschau schließt wie mit OK (die Einstellung
        // des aktiven Feldes ist schon übernommen); abgebrochen → sie bleibt offen
        if st.dialogs.exported != d.export_mark {
            return;
        }
    }
    let Some(doc) = st.session.active() else {
        return;
    };
    d.frames = doc.frame_count();
    d.frame = d.frame.min(d.frames - 1);
    d.doc_size = fp_export::render::page_rect(&doc.doc, doc.page)
        .map(|r| (r.width().max(1) as usize, r.height().max(1) as usize))
        .unwrap_or((1, 1));
    animation_tab::tick(&mut d, ctx);
    let base = JobInput {
        doc: Arc::clone(&doc.doc),
        renderer: Arc::clone(&st.renderer),
        page: doc.page,
        // gemeinsamer Status des Dialogs (beim Öffnen Status 1 wie der Export)
        frame: d.frame,
        doc_key: (doc.id.0, doc.generation),
        kind: JobKind::Original,
        key: 0,
    };
    poll_palette_io(&mut d);
    poll_size_job(&mut d);
    let outcome = draw(ctx, st, &mut d);
    // Status im selben Bild gewechselt (Knöpfe, Liste): gleich für den neuen Status rechnen
    let base = JobInput {
        frame: d.frame.min(d.frames - 1),
        ..base
    };
    d.update_jobs(ctx, &base);
    preset_name_dialog(ctx, st, &mut d);
    sub_dialogs(ctx, &mut d, &base);
    match outcome {
        Some(Outcome::Ok | Outcome::Export) if d.slice.is_some() => {
            let s = d.panes[d.active].settings.clone();
            if let Some(id) = d.slice {
                commit_slice_settings(st, id, &s);
            }
        }
        Some(Outcome::Ok) => {
            let s = d.panes[d.active].settings.clone();
            commit_settings(st, &s);
            sync_export_dialog(st, &s);
        }
        Some(Outcome::Export) => {
            let s = d.panes[d.active].settings.clone();
            commit_settings(st, &s);
            sync_export_dialog(st, &s);
            // aus dem Exportdialog geöffnet („Optionen…“): zurück dorthin, er liest die eben
            // übernommene Einstellung der Seite
            if st.dialogs.export.is_none() {
                // der Verlaufsschritt läuft erst nach diesem Bild: Dateiendung schon jetzt nach
                // der übernommenen Einstellung
                st.dialogs.export = super::export::ExportDialog::new(st).map(|mut e| {
                    e.page_ext = s.format.extension().to_string();
                    e
                });
                d.export_child = st.dialogs.export.is_some();
                d.export_mark = st.dialogs.exported;
                st.dialogs.preview = Some(d);
            }
        }
        Some(Outcome::Cancel) => {}
        None => st.dialogs.preview = Some(d),
    }
}

/// Liegt der Exportdialog unter der Bildvorschau (geöffnet über dessen „Optionen…“), folgt seine
/// Dateiendung sofort dem übernommenen Format (der Verlaufsschritt läuft erst nach diesem Bild).
fn sync_export_dialog(st: &mut AppState, s: &OptimizeSettings) {
    if let Some(e) = st.dialogs.export.as_mut() {
        e.page_ext = s.format.extension().to_string();
        if !e.file_name.trim().is_empty() {
            e.file_name = super::export::with_ext(&e.file_name, &e.ext());
        }
    }
}

/// OK bzw. „Exportieren…“ (§7): die Einstellung des aktiven Feldes (Format, Palette, Skalierung,
/// Exportbereich, Animation, Hintergrund) wird die Exporteinstellung der Seite – ein
/// Verlaufsschritt „Optimieren“ (`pi.export.settings`), nur wenn sich etwas ändert. Verzögerung,
/// Löschmethode und Sichtbarkeit der Status gehen nur in die Exporteinstellung (`frameInfo`), nicht
/// zurück in die Status des Dokuments (§4/§11.7 lassen das offen). `true`, wenn geschrieben wurde.
/// OK der Segment-Bildvorschau (TW-44): Einstellung ohne Exportbereich in das Segment schreiben;
/// unverändert kein Verlaufsschritt.
pub fn commit_slice_settings(
    st: &mut AppState,
    id: fp_model::ElementId,
    s: &OptimizeSettings,
) -> bool {
    let Some(d) = st.session.active() else {
        return false;
    };
    let Some(e) = fp_engine::docops::element_on_page(&d.doc, d.page, id) else {
        return false;
    };
    let fp_model::ElementKind::WebObject(w) = &e.kind else {
        return false;
    };
    let page_model = d
        .doc
        .pages
        .get(d.page)
        .and_then(|p| p.root.export.as_deref().cloned());
    let cur = w
        .slice
        .as_ref()
        .and_then(|sp| sp.export.as_deref().cloned());
    let base = cur.clone().or(page_model).unwrap_or_default();
    let mut s = s.clone();
    s.crop = None;
    let model = s.to_model(&base);
    let (Ok(v), old) = (
        serde_json::to_value(&model),
        cur.and_then(|c| serde_json::to_value(c).ok()),
    ) else {
        return false;
    };
    if old.as_ref() == Some(&v) {
        return false;
    }
    st.run(
        "pi.web",
        serde_json::json!({ "ids": [id.0], "exportSettings": v }),
    );
    true
}

pub fn commit_settings(st: &mut AppState, s: &OptimizeSettings) -> bool {
    let Some(d) = st.session.active() else {
        return false;
    };
    let Some(page) = d.doc.pages.get(d.page) else {
        return false;
    };
    if let Some(cur) = page.root.export.as_deref() {
        let same = serde_json::to_value(s.to_model(cur))
            .ok()
            .zip(serde_json::to_value(cur).ok())
            .is_some_and(|(a, b)| a == b);
        if same {
            return false;
        }
    }
    optimize::apply_settings(st, s);
    true
}

/// Ergebnisse von „Palette laden/speichern“ übernehmen.
fn poll_palette_io(d: &mut ImagePreview) {
    let events: Vec<PaletteIo> = d
        .palette_io
        .lock()
        .map(|mut v| std::mem::take(&mut *v))
        .unwrap_or_default();
    for e in events {
        match e {
            PaletteIo::Loaded { pane, colors } => {
                // nach „Palette laden“ steht die Palettenliste auf „Benutzerdefiniert“ (§2.7)
                if let Some(p) = d.panes.get_mut(pane) {
                    fp_export::palette_edit::load_palette(&mut p.settings, &colors);
                }
                d.table.selected.clear();
            }
            PaletteIo::Error(msg) => {
                d.message = Some(fp_i18n::tr!("image-preview-palette-error", error = msg))
            }
        }
    }
}

const PALETTE_OPEN: &[(&str, &[&str])] = &[
    ("dlg-filter-act", &["act"]),
    ("dlg-filter-ase", &["ase"]),
    ("dlg-filter-aco", &["aco"]),
    ("GIF (*.gif)", &["gif"]),
];
const PALETTE_SAVE: &[(&str, &[&str])] = &[
    ("dlg-filter-act", &["act"]),
    ("dlg-filter-ase", &["ase"]),
    ("dlg-filter-aco", &["aco"]),
];

/// „Palette laden…“: Farbtabelle (ACT, ASE, ACO, GIF) wählen; die Farben werden die
/// benutzerdefinierte Palette des Feldes `pane`.
pub(crate) fn load_palette(st: &mut AppState, d: &ImagePreview, pane: usize) {
    let slot = Arc::clone(&d.palette_io);
    super::panel_menus::pick_open(
        st,
        &t("dlg-pmenu-load-palette"),
        PALETTE_OPEN,
        move |_st, p| {
            let ev = match fp_engine::commands::panel::read_swatch_file(&p.to_string_lossy()) {
                Ok(cols) if !cols.is_empty() => PaletteIo::Loaded {
                    pane,
                    colors: cols
                        .iter()
                        .take(256)
                        .map(|c| [c.r(), c.g(), c.b()])
                        .collect(),
                },
                Ok(_) => PaletteIo::Error(t("image-preview-palette-empty")),
                Err(e) => PaletteIo::Error(e.to_string()),
            };
            if let Ok(mut v) = slot.lock() {
                v.push(ev);
            }
        },
    );
}

/// „Palette speichern…“: angezeigte Palette des aktiven Feldes als ACT/ASE/ACO.
pub(crate) fn save_palette(
    st: &mut AppState,
    d: &ImagePreview,
    entries: &[fp_export::palette_edit::PaletteEntry],
) {
    let slot = Arc::clone(&d.palette_io);
    let colors: Vec<fp_color::Argb> = entries
        .iter()
        .map(|e| fp_color::Argb(e.shown | 0xFF00_0000))
        .collect();
    let name = format!("{}.act", t("dlg-pmenu-palette-file"));
    super::panel_menus::pick_save(
        st,
        &t("dlg-pmenu-save-palette"),
        &name,
        PALETTE_SAVE,
        move |_st, p| {
            let ext = p
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            let bytes = fp_engine::commands::panel::swatch_file_bytes(&ext, &colors);
            if let Err(e) = std::fs::write(&p, bytes)
                && let Ok(mut v) = slot.lock()
            {
                v.push(PaletteIo::Error(e.to_string()));
            }
        },
    );
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
            Ok(fp_export::doc_export::SizeOutcome::TooSmall { .. }) => {
                d.message = Some(t("image-preview-size-too-small"))
            }
            Ok(fp_export::doc_export::SizeOutcome::NotApplicable) => {}
            Err(e) if !e.is_empty() => d.message = Some(e),
            Err(_) => {}
        }
    }
}

fn draw(ctx: &egui::Context, st: &mut AppState, d: &mut ImagePreview) -> Option<Outcome> {
    let m = options::measure(ctx);
    let (export_l, ok_l, cancel_l) = (t("dlg-export-button"), t("btn-ok"), t("btn-cancel"));
    let (ex_w, ok_w, cancel_w) = (
        button_w(ctx, &export_l, 90.0),
        button_w(ctx, &ok_l, 90.0),
        button_w(ctx, &cancel_l, 75.0),
    );
    let screen = ctx.content_rect();
    let need_w = (8.0 + m.width + 7.0 + MIN_AREA_W.max(toolbar::WIDTH + 3.0) + 8.0)
        .max(8.0 + ex_w + 5.0 + ok_w + 4.0 + cancel_w + 8.0);
    // wie im Original Start- = Mindestgröße; auf kleinen Bildschirmen höchstens so groß wie das Fenster
    let min = vec2(
        MIN_CLIENT
            .x
            .max(need_w)
            .min((screen.width() - 2.0).max(need_w.min(MIN_CLIENT.x))),
        MIN_CLIENT
            .y
            .min(screen.height() - DIALOG_TITLE_H - 2.0)
            .max(420.0),
    )
    .floor();
    let own = crate::ui_commands::export_presets(&st.session.prefs);
    let entries = pane::preset_entries(&own);
    let mut out = None;
    let r = dialog_frame_resizable(ctx, DIALOG_ID, &t("dlg-preview-title"), min, |ui, body| {
        let theme = Theme::current(ui.ctx());
        let left = Rect::from_min_size(
            body.min + vec2(8.0, 10.0),
            vec2(m.width, body.height() - 10.0 - 52.0),
        );
        let area = Rect::from_min_max(
            pos2(left.right() + 7.0, body.top() + 11.0),
            pos2(body.right() - 8.0, body.bottom() - 79.0),
        );
        options::show(ui, st, left, m, d);
        draw_panes(ui, d, area, &entries);
        if let Some((i, k)) = d.pending_file.take() {
            if k == color_table::PaletteCmd::LoadPalette {
                load_palette(st, d, i);
            } else {
                let entries = d.panes[i]
                    .result
                    .as_ref()
                    .map(|r| r.entries.clone())
                    .unwrap_or_default();
                save_palette(st, d, &entries);
            }
        }
        toolbar::show(ui, d, pos2(area.left() + 3.0, body.bottom() - 75.0));
        ui.painter().hline(
            (body.left() + 8.0)..=(body.right() - 8.0),
            body.bottom() - 41.5,
            Stroke::new(1.0, theme.colors.line_mid),
        );
        let by = body.bottom() - 31.0;
        let cr = Rect::from_min_size(
            pos2(body.right() - 8.0 - cancel_w, by),
            vec2(cancel_w, 23.0),
        );
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
    let lays: Vec<(usize, pane::PaneLayout)> = rects
        .iter()
        .map(|&(i, r)| (i, pane::PaneLayout::new(r, i, d.mode)))
        .collect();
    // gemeinsamer Ausschnitt (Dokumentpunkt in der Feldmitte): Bezug ist das Bild des aktiven Feldes
    let view = lays
        .iter()
        .map(|(_, l)| l.image.size())
        .fold(vec2(f32::MAX, f32::MAX), |a, b| a.min(b));
    if let Some(i) = std::iter::once(d.active)
        .chain(lays.iter().map(|(i, _)| *i))
        .find(|&i| d.pane_map(i).is_some())
    {
        d.clamp_center(i, view);
    }
    d.layout = lays.clone();
    // Alt aus dem Zustand oder aus dem Klick selbst (Taste und Klick im selben Bild)
    let alt = ui.input(|i| {
        i.modifiers.alt
            || i.events
                .iter()
                .any(|e| matches!(e, egui::Event::PointerButton { modifiers, .. } if modifiers.alt))
    });
    for (i, lay) in &lays {
        let i = *i;
        let resp = ui.interact(
            lay.frame,
            Id::new(("fp-ipv-pane", i)),
            Sense::CLICK | Sense::DRAG,
        );
        resp.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Other,
                true,
                fp_i18n::tr!("image-preview-pane", n = i as i64 + 1),
            )
        });
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
        if i == d.active
            && options::visibility(&d.panes[i].settings)
                .color_table
                .shown()
        {
            let entries = d.panes[i]
                .result
                .as_ref()
                .map(|r| r.entries.clone())
                .unwrap_or_default();
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
                            color_table::ColorMode::Edit(o) => entries
                                .iter()
                                .find(|e| e.color & 0xFF_FFFF == o & 0xFF_FFFF)
                                .map(|e| e.shown)
                                .unwrap_or(o),
                            color_table::ColorMode::Add => 0xFF80_8080,
                        };
                        d.color_dialog = Some(color_table::ColorDialog::new(i, mode, start));
                    }
                    color_table::FollowUp::LoadPalette | color_table::FollowUp::SavePalette => {
                        d.pending_file = Some((i, k))
                    }
                    color_table::FollowUp::None => {}
                }
            }
        }
        // Leertaste + Ziehen bzw. mittlere Taste verschieben mit jedem Werkzeug (wie im
        // Dokumentfenster); der gegriffene Punkt bleibt unter dem Zeiger
        let space = ui.input(|inp| inp.key_down(egui::Key::Space)) && !ui.ctx().text_edit_focused();
        let hand = resp.dragged_by(egui::PointerButton::Middle)
            || (space && resp.dragged_by(egui::PointerButton::Primary) && d.crop_drag.is_none());
        if hand || (d.tool == Tool::Pointer && resp.dragged()) {
            if d.grab.is_none() {
                d.grab = ui
                    .input(|inp| inp.pointer.press_origin())
                    .map(|o| (o, d.center_doc()));
            }
            if let (Some((o, c0)), Some(p)) = (d.grab, ui.input(|inp| inp.pointer.latest_pos())) {
                d.center = Some(c0 - (p - o) / (d.zoom() * d.pane_scale(i)));
                d.user_view = true;
            }
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            continue;
        }
        if space && on_image {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
            continue;
        }
        match d.tool {
            Tool::Pipette(_) => {
                if on_image {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                }
            }
            Tool::Pointer => {
                // Ziehen siehe oben (der gegriffene Punkt bleibt unter dem Zeiger, ab dem Drücken
                // gerechnet, auch die Bewegung, bevor egui das Ziehen erkennt)
                if on_image {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                }
            }
            Tool::Zoom => {
                if on_image {
                    ui.ctx().set_cursor_icon(if alt {
                        egui::CursorIcon::ZoomOut
                    } else {
                        egui::CursorIcon::ZoomIn
                    });
                }
                if resp.clicked()
                    && let Some(p) = resp.interact_pointer_pos()
                {
                    let idx = if alt {
                        d.zoom_idx.saturating_sub(1)
                    } else {
                        (d.zoom_idx + 1).min(toolbar::ZOOMS.len() - 1)
                    };
                    d.set_zoom(idx, Some((p, lay.image.center())));
                }
            }
            Tool::ExportArea => crop_tool(ui, d, i, lay, &resp),
        }
    }
    if !ui.input(|inp| inp.pointer.any_down()) {
        d.crop_drag = None;
        d.grab = None;
    }
    // Esc während des Ziehens mit „Exportbereich“: Ziehen abbrechen, Bereich wie vorher; die Taste
    // wird verbraucht, damit sie nicht zugleich den Dialog schließt
    if let Some((i, .., before)) = d.crop_drag
        && ui.input_mut(|inp| inp.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
    {
        d.panes[i].settings.crop = before;
        d.crop_drag = None;
    }
    wheel(ui, d, &lays);
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
                (Some(r), _) => r
                    .image
                    .as_deref()
                    .and_then(|img| pane::upload(ui.ctx(), img))
                    .inspect(|t| d.original_tex = Some((r.key, t.clone()))),
                _ => None,
            };
            (res, tex)
        };
        // gemeinsamer Dokumentpunkt in der Feldmitte, je Feld mit dessen Exportbereich/Skalierung
        let img_rect = res
            .as_ref()
            .and_then(|r| r.image.as_ref())
            .and_then(|_| d.image_rect(i, lay));
        pane::paint_image(ui, lay.image, img_rect, tex.as_ref());
        // Werkzeug „Exportbereich“: Rahmen mit Griffen im aktiven Feld
        if show
            && i == d.active
            && let Some(sr) = d.export_area_rect(i, lay)
            && let Some(ir) = d.full_image_rect(i, lay)
        {
            crop::paint(ui, lay.image, ir, sr);
        }
        // Farbfeld-Rückmeldung: Pixel der gewählten Farben im aktiven Feld hervorheben (§2.7)
        if show
            && i == d.active
            && d.table.feedback
            && let (Some(r), Some(ir)) = (res.as_ref(), img_rect)
        {
            let sel: Vec<usize> = d
                .table
                .selected_entries(&r.entries)
                .into_iter()
                .map(|(k, _)| k)
                .collect();
            if !sel.is_empty() {
                let cached = d
                    .feedback_tex
                    .as_ref()
                    .filter(|(k, s, _)| *k == r.key && *s == sel)
                    .map(|(_, _, t)| t.clone());
                let tex = cached.or_else(|| {
                    let img = r.image.as_ref()?;
                    let col = crate::theme::with_alpha(Theme::current(ui.ctx()).colors.focus, 200);
                    let ci = color_table::feedback_image(
                        &r.indices,
                        img.width(),
                        img.height(),
                        &sel,
                        col,
                    )?;
                    let t =
                        ui.ctx()
                            .load_texture("fp-ipv-feedback", ci, egui::TextureOptions::NEAREST);
                    d.feedback_tex = Some((r.key, sel.clone(), t.clone()));
                    Some(t)
                });
                if let Some(t) = tex {
                    ui.painter()
                        .with_clip_rect(lay.image.intersect(ui.clip_rect()))
                        .image(
                            t.id(),
                            ir,
                            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
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
            let name = pane::matching_preset(entries, &p.settings)
                .map(|k| entries[k].label.clone())
                .or_else(|| p.last_preset.clone())
                .unwrap_or_else(|| optimize::settings_label(&p.settings));
            d.preset_name = Some((i, name));
        }
    }
}

/// Bildschirmpunkte je Zeilen-Rastung beim Bildlauf (wie im Dokumentfenster).
const WHEEL_LINE_PX: f32 = 52.0;
/// Punkte eines Rad-Ereignisses, die einer Rastung entsprechen (Touchpads, hochauflösende Räder).
const WHEEL_POINTS_PER_NOTCH: f32 = 100.0;

/// Mausrad über einem Feld (§11.5 im Original nicht gemessen; Verhalten wie im Dokumentfenster):
/// Rad = alle Felder senkrecht verschieben (waagerechtes Rad bzw. Touchpad waagerecht), Umschalt+Rad
/// tut nichts, Strg+Rad = je volle Rastung genau eine der 11 Zoomstufen um den Mauszeiger.
fn wheel(ui: &egui::Ui, d: &mut ImagePreview, lays: &[(usize, pane::PaneLayout)]) {
    // nur, wenn die Bildvorschau oben liegt und der Zeiger über einem Bild steht
    let Some((pane, lay)) = lays
        .iter()
        .find(|(_, l)| ui.rect_contains_pointer(l.image))
        .map(|(i, l)| (*i, l))
    else {
        return;
    };
    let Some(p) = ui.input(|i| i.pointer.hover_pos()) else {
        return;
    };
    let events: Vec<(egui::MouseWheelUnit, Vec2, bool, bool)> = ui.input(|i| {
        i.events
            .iter()
            .filter_map(|e| match e {
                egui::Event::MouseWheel {
                    unit,
                    delta,
                    modifiers,
                    ..
                } => Some((
                    *unit,
                    *delta,
                    modifiers.ctrl || modifiers.command,
                    modifiers.shift,
                )),
                _ => None,
            })
            .collect()
    });
    let mut pan = Vec2::ZERO;
    for (unit, delta, ctrl, shift) in events {
        if ctrl {
            let n = match unit {
                egui::MouseWheelUnit::Point => delta.y / WHEEL_POINTS_PER_NOTCH,
                _ if delta.y == 0.0 => 0.0,
                _ => delta.y.signum(),
            };
            // Richtungswechsel verwirft den Rest; höchstens eine Stufe je Ereignis
            if n * d.wheel_acc < 0.0 {
                d.wheel_acc = 0.0;
            }
            d.wheel_acc += n;
            let step = if d.wheel_acc >= 1.0 {
                1
            } else if d.wheel_acc <= -1.0 {
                -1
            } else {
                0
            };
            if step != 0 {
                d.wheel_acc -= step as f32;
                let idx = if step > 0 {
                    (d.zoom_idx + 1).min(toolbar::ZOOMS.len() - 1)
                } else {
                    d.zoom_idx.saturating_sub(1)
                };
                d.set_zoom_in(idx, Some((p, lay.image.center())), pane);
            }
        } else if !shift {
            let k = match unit {
                egui::MouseWheelUnit::Point => 1.0,
                egui::MouseWheelUnit::Line => WHEEL_LINE_PX,
                egui::MouseWheelUnit::Page => lay.image.height(),
            };
            pan += delta * k;
        }
    }
    if pan != Vec2::ZERO {
        d.pan_in(pan, pane);
    }
}

/// Werkzeug „Exportbereich“ im Feld `i` (§6): Ziehen an Griffen oder Kanten ändert den Bereich,
/// Ziehen im Inneren verschiebt ihn. Das Kästchen „Exportbereich“ wird dabei angehakt.
fn crop_tool(
    ui: &egui::Ui,
    d: &mut ImagePreview,
    i: usize,
    lay: &pane::PaneLayout,
    resp: &egui::Response,
) {
    let Some(sr) = d.export_area_rect(i, lay) else {
        return;
    };
    if resp.drag_started()
        && let Some(o) = ui.input(|inp| inp.pointer.press_origin())
        && let Some(h) = crop::hit(sr, o)
    {
        d.crop_drag = Some((
            i,
            h,
            crop::area_doc(d.doc_size, &d.panes[i].settings),
            o,
            d.panes[i].settings.crop,
        ));
    }
    if let Some((pi, h, start, o, _)) = d.crop_drag
        && pi == i
    {
        if resp.dragged()
            && let Some(p) = ui.input(|inp| inp.pointer.latest_pos())
        {
            let (fx, fy) = crop::factors(d.doc_size, &d.panes[i].settings);
            let z = d.zoom() as f64;
            let delta = p - o;
            let new = crop::drag(
                start,
                h,
                delta.x as f64 / (fx * z),
                delta.y as f64 / (fy * z),
                d.doc_size,
            );
            d.panes[i].settings.crop = Some(new);
        }
        ui.ctx().set_cursor_icon(h.cursor());
    } else if let Some(h) = resp.hover_pos().and_then(|p| crop::hit(sr, p)) {
        ui.ctx().set_cursor_icon(h.cursor());
    }
}

/// Dialog „Vorgabenname“ (§2.8): Name vorbelegt und markiert, OK speichert die Einstellung des
/// Feldes als eigene Vorgabe.
fn preset_name_dialog(ctx: &egui::Context, st: &mut AppState, d: &mut ImagePreview) {
    let Some((i, mut name)) = d.preset_name.clone() else {
        return;
    };
    let (title, name_l, ok_l, cancel_l) = (
        t("image-preview-preset-name-title"),
        t("image-preview-preset-name-label"),
        t("btn-ok"),
        t("btn-cancel"),
    );
    let dx = super::extra_w(ctx, &[&name_l], 50.0);
    let mut result = None;
    let r = dialog_frame(
        ctx,
        PRESET_DIALOG_ID,
        &title,
        vec2(340.0 + dx, 110.0),
        |ui, body| {
            super::put_left(
                ui,
                Rect::from_min_size(body.min + vec2(16.0, 18.0), vec2(54.0 + dx, 20.0)),
                egui::Label::new(name_l.as_str()).extend(),
            );
            let te = ui.put(
                Rect::from_min_size(body.min + vec2(70.0 + dx, 14.0), vec2(250.0, 22.0)),
                egui::TextEdit::singleline(&mut name),
            );
            super::autofocus_field(ui, &te, name.chars().count());
            // einziges Eingabefeld: behält den Fokus (wie „Arbeitsbereich speichern“)
            te.request_focus();
            let b = super::button_row(
                ui.ctx(),
                body.right() - 16.0,
                body.bottom() - 34.0,
                &[&ok_l, &cancel_l],
                76.0,
                23.0,
                8.0,
            );
            let ok = push_button(ui, b[0], &ok_l, true, !name.trim().is_empty());
            if ok || (ui.input(|inp| inp.key_pressed(egui::Key::Enter)) && !name.trim().is_empty())
            {
                result = Some(true);
            }
            if push_button(ui, b[1], &cancel_l, false, true) {
                result = Some(false);
            }
        },
    );
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
