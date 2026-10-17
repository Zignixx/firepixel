//! Modifizieren ▸ Bitmap nachzeichnen… (Agent VT-2): eigener Firepixel-Dialog für den Kern
//! [`fp_trace`] und den Befehl `modify.traceBitmap` (VT-1).
//!
//! Aufbau (Regeln in `docs\DESIGN.md`):
//! * **Kopf** mit Symbol (Pixel → Kurve), Titel und Leitsatz,
//! * links die **Vorgaben als Karten** (Logo, Schwarzweiß, Illustration, Foto – je Symbol und ein
//!   Satz), darunter vier **Hauptregler** mit Klartext (Farben bzw. Schwelle, Detail und Glätte,
//!   Ecken, Flecken entfernen) und der einklappbare Abschnitt **„Erweitert“** mit allen
//!   [`TraceOptions`],
//! * rechts die große **Live-Vorschau**: Vergleich mit verschiebbarem Trenner, Original, Vektor
//!   oder Konturen über dem Original; Mausrad zoomt um den Zeiger (eine Rastung = eine Stufe),
//!   Ziehen verschiebt, Doppelklick passt ein; darunter die **Kennzahlen** (Pfade, Punkte,
//!   Farben, Übereinstimmung),
//! * Knopfleiste: „Als SVG speichern…“ und „Original behalten“ links, OK/Abbrechen rechts.
//! * **Aufräumen** (VT-4, Logo-Nachbearbeitung [`fp_trace::RefineOptions`]): Hauptschalter bei den
//!   Einstellungen, die einzelnen Schritte mit Toleranzreglern im Abschnitt „Erweitert“; die
//!   Kennzahlen zeigen dann zusätzlich erkannte Grundformen, eingerastete Geraden und die Knoten
//!   vorher → nachher ([`fp_trace::RefineReport`]).
//! * **Verfahren** (VT-6, Nutzerbefund N29): Zeile über den Vorgaben, „Farbflächen“ (bisher) oder
//!   „Konturen“ (Potrace-Verfahren, [`fp_trace::Engine::Potrace`]); die Hauptregler wechseln mit
//!   (Konturen: Flecken als Fläche, Ecken = `alphaMax`, Kurven zusammenfassen mit Toleranz;
//!   „Erweitert“: Abbiegeregel statt „Aufräumen“). Die Wahl wird sofort gemerkt ([`PREF_ENGINE`]).
//!
//! Die Rechnung läuft in einem **Hintergrund-Thread** je Lauf mit eigenem `AtomicBool`; ein neuer
//! Lauf bricht den alten ab, Regleränderungen werden kurz gesammelt (kein Ruckeln). Die Vorschau
//! wird überabgetastet gerastert (Kantenglättung) und auf höchstens [`PREVIEW_MAX`] Pixel begrenzt.
//! **OK** führt `modify.traceBitmap` mit denselben Optionen aus (ein Verlaufsschritt); passt das
//! letzte Vorschauergebnis zu den Optionen, wird es dem Befehl angeboten und nicht neu gerechnet
//! ([`fp_engine::commands::trace::offer_prepared`]). **Abbrechen** ändert nichts. Vorgabe, Optionen und „Original behalten“ werden in den
//! Voreinstellungen gemerkt ([`PREF_OPTIONS`], [`PREF_KEEP`]).

use super::{DIALOG_TITLE_H, DialogClose, dialog_frame_resizable_sized, report_overflow, t};
use crate::app_state::{Action, AppState};
use crate::icons;
use crate::inspector::fields;
use crate::theme::{Theme, mix, with_alpha};
use crate::ui_kit::{self, BarButton, HeaderIcon, KitIcon, Tone, sp, type_scale};
use egui::{
    Align2, Color32, CornerRadius, FontId, Id, Pos2, Rect, Sense, Shape, Stroke, StrokeKind,
    TextureHandle, Ui, Vec2, pos2, vec2,
};
use fp_raster::RgbaImage;
use fp_trace::{
    ColorMode, CurveMode, Engine, Hierarchy, Preset, RefineReport, Stage, TraceOptions,
    TraceResult, TurnPolicy,
};
use serde_json::json;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, Instant};

/// Kennung des Dialograhmens.
pub const DIALOG_ID: &str = "fp-trace";
/// Voreinstellung: zuletzt benutzte Optionen (JSON von [`TraceOptions`]).
pub const PREF_OPTIONS: &str = "TraceDialogOptions";
/// Voreinstellung: „Original behalten“.
pub const PREF_KEEP: &str = "TraceDialogKeepOriginal";
/// Voreinstellung: zuletzt gewähltes Verfahren (`clusters`/`potrace`, VT-6) – gilt sofort beim
/// Umschalten, auch wenn der Dialog danach abgebrochen wird.
pub const PREF_ENGINE: &str = "TraceDialogEngine";
/// Voreinstellung: vom Nutzer gezogene Größe des Client-Bereichs („Breite Höhe“ in Punkten).
pub const PREF_SIZE: &str = "TraceDialogSize";
/// Ausgangsgröße des Client-Bereichs: außen ≈ 1050 × 830 (Nutzerbefund N20; vorher 900 × 640).
/// Vorschau groß, „Aufräumen“ und „Erweitert“ ohne Bildlauf sichtbar.
pub const DEFAULT_CLIENT: Vec2 = Vec2::new(1048.0, 800.0);
/// Mindestgröße des Client-Bereichs (kleiner wird es nur, wenn das Hauptfenster selbst klein ist).
pub const MIN_CLIENT: Vec2 = Vec2::new(680.0, 500.0);
/// Anteil des Hauptfensters, den der Dialog beim Öffnen höchstens einnimmt.
pub const MAX_SCREEN_SHARE: f32 = 0.9;
/// Eigene Inspektor-Vorlage (Q5 bei Bitmaps): Knopf „Nachzeichnen…“.
pub const INSPECTOR_Q5: u16 = 19101;
/// Höchstabweichung je Farbkanal für „gleich“ in der Übereinstimmung.
pub const MATCH_TOLERANCE: i32 = 40;
/// Längste Seite der Vorschaubilder (Pixel).
pub const PREVIEW_MAX: f32 = 1600.0;
/// Regleränderungen so lange sammeln, bevor ein laufender Lauf ersetzt wird (Sekunden).
const DEBOUNCE: f64 = 0.15;
/// Höhe einer Zeile der Kennzahlen.
const STATS_ROW_H: f32 = 38.0;
/// Höchstzahl der Konturpunkte in der Konturenansicht.
const OUTLINE_POINTS_MAX: usize = 150_000;

/// Ansicht der Vorschau.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewMode {
    /// Original links, Vektor rechts, Trenner verschiebbar.
    Split,
    Original,
    Vector,
    /// Konturen der Pfade über dem Original.
    Overlay,
}

impl ViewMode {
    pub const ALL: [ViewMode; 4] = [
        ViewMode::Split,
        ViewMode::Original,
        ViewMode::Vector,
        ViewMode::Overlay,
    ];

    fn key(self) -> &'static str {
        match self {
            ViewMode::Split => "trace-view-split",
            ViewMode::Original => "trace-view-original",
            ViewMode::Vector => "trace-view-vector",
            ViewMode::Overlay => "trace-view-overlay",
        }
    }
}

/// Kennzahlen des letzten Ergebnisses.
#[derive(Clone, Debug, PartialEq)]
pub struct Stats {
    pub paths: usize,
    pub points: usize,
    pub colors: usize,
    /// Anteil der fast gleichen Pixel (0–100).
    pub match_pct: f32,
    /// Rechenzeit (Nachzeichnen + Vorschau) in Millisekunden.
    pub ms: u64,
    /// Nachgezeichnete Größe, falls verkleinert.
    pub downscaled: Option<(u32, u32)>,
    /// Kennzahlen des Aufräumens (nur wenn es eingeschaltet war).
    pub refine: Option<RefineReport>,
    /// Höchste Deckkraft im Bild, wenn **alle** sichtbaren Pixel unter der Alphaschwelle liegen
    /// (halbtransparente Bitmap: nichts wird nachgezeichnet, NP-1 Nebenbefund zu N29).
    pub faint_alpha: Option<u8>,
}

impl Stats {
    /// Übereinstimmung anzeigen? Ohne Pfade nicht: „100,0 %“ bei einer halbtransparenten Bitmap
    /// (alles unter der Alphaschwelle zählt als „keine Fläche“) täuschte ein perfektes Ergebnis vor.
    pub fn match_shown(&self) -> Option<f32> {
        (self.paths > 0).then_some(self.match_pct)
    }
}

/// Höchste Deckkraft, falls das Bild sichtbare Pixel hat, die **alle** unter der Alphaschwelle liegen.
pub fn faint_alpha(img: &RgbaImage, opts: &TraceOptions) -> Option<u8> {
    let o = opts.validated();
    if !o.ignore_transparent {
        return None;
    }
    let max = img.data().iter().map(|p| p[3]).max().unwrap_or(0);
    (max > 0 && max < o.alpha_threshold).then_some(max)
}

/// Fertiges Ergebnis eines Laufs.
struct Ready {
    /// Optionen, mit denen gerechnet wurde (validiert).
    opts: TraceOptions,
    result: Arc<TraceResult>,
    preview: RgbaImage,
    outlines: Vec<Vec<Pos2>>,
    stats: Stats,
}

/// Nachricht des Hintergrund-Threads.
struct Done {
    run: u64,
    /// `None` = abgebrochen.
    result: Option<Result<Ready, String>>,
}

/// Laufender Hintergrund-Lauf.
struct Job {
    run: u64,
    cancel: Arc<AtomicBool>,
    /// Fortschritt 0–1 (Bits eines `f32`).
    progress: Arc<AtomicU32>,
    /// Abschnitt (0 Vorbereitung, 1 Farbflächen, 2 Kurven, 3 fertig).
    stage: Arc<AtomicU32>,
    rx: Receiver<Done>,
}

impl Drop for Job {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

/// Zustand des Dialogs.
pub struct TraceDialog {
    pub opts: TraceOptions,
    pub keep_original: bool,
    pub view: ViewMode,
    /// Lage des Trenners (Anteil der Bildbreite, 0–1).
    pub split: f32,
    /// Maßstab (Bildschirmpunkte je Bildpixel); `None` = eingepasst.
    pub zoom: Option<f32>,
    /// Verschiebung der Bildmitte gegenüber der Vorschaumitte (nur mit `zoom`).
    pub pan: Vec2,
    /// Abschnitt „Erweitert“ aufgeklappt.
    pub advanced: bool,
    pub stats: Option<Stats>,
    pub error: Option<String>,
    /// Zahl der gestarteten Läufe (Tests, Diagnose).
    pub runs_started: u64,
    source: Arc<RgbaImage>,
    bitmaps: usize,
    original_tex: Option<TextureHandle>,
    vector_tex: Option<TextureHandle>,
    result: Option<Arc<TraceResult>>,
    /// Optionen des Ergebnisses `result` (für die Übernahme bei OK).
    result_opts: Option<TraceOptions>,
    outlines: Vec<Vec<Pos2>>,
    job: Option<Job>,
    next_run: u64,
    dirty: bool,
    last_edit: f64,
    scroll_acc: f32,
    /// Einstellungsspalte beim ersten Zeichnen nach oben rollen (egui merkt sich die Lage
    /// sonst über das Schließen hinaus).
    scroll_top: bool,
}

// ---------------------------------------------------------------------- Öffnen, Schließen

/// Ausgewählte, nicht gesperrte Bitmaps des aktiven Dokuments (oben zuerst).
fn selected_bitmaps(st: &AppState) -> Vec<RgbaImage> {
    let Some(d) = st.session.active() else {
        return Vec::new();
    };
    d.selection
        .elements
        .iter()
        .filter_map(|id| fp_engine::docops::element_on_page(&d.doc, d.page, *id))
        .filter_map(|e| match &e.kind {
            fp_model::ElementKind::Bitmap(b) => fp_engine::docops::bitmap_image(b),
            _ => None,
        })
        .filter(|img| img.width() > 0 && img.height() > 0)
        .collect()
}

/// Gemerkte, vom Nutzer gezogene Größe des Client-Bereichs (`None` = nie geändert).
pub fn remembered_size(st: &AppState) -> Option<Vec2> {
    let s = st.session.prefs.get_str(PREF_SIZE, "");
    let mut it = s.split_whitespace().map(|v| v.parse::<f32>());
    match (it.next(), it.next()) {
        (Some(Ok(w)), Some(Ok(h))) if w.is_finite() && h.is_finite() && w > 0.0 && h > 0.0 => {
            Some(vec2(w, h))
        }
        _ => None,
    }
}

/// Mindest- und Ausgangsgröße des Client-Bereichs für ein Hauptfenster der Größe `screen`
/// (Punkte): Ausgangsgröße die gemerkte bzw. [`DEFAULT_CLIENT`], außen höchstens
/// [`MAX_SCREEN_SHARE`] des Hauptfensters. Die Mindestgröße [`MIN_CLIENT`] geht vor; nur in sehr
/// kleinen Hauptfenstern (z. B. 1024 × 768 bei 150 %) wird sie auf das Fenster abzüglich eines
/// schmalen Rands begrenzt (wie vorher; nie unter 320 × 300), damit der Inhalt nicht überläuft.
pub fn dialog_sizes(screen: Vec2, remembered: Option<Vec2>) -> (Vec2, Vec2) {
    let frame = vec2(2.0, DIALOG_TITLE_H + 2.0);
    let cap = (screen * MAX_SCREEN_SHARE - frame).floor();
    let fit = (screen - vec2(16.0, DIALOG_TITLE_H + 16.0)).floor();
    let min = MIN_CLIENT.min(fit).max(vec2(320.0, 300.0));
    let initial = remembered
        .unwrap_or(DEFAULT_CLIENT)
        .min(cap)
        .max(min)
        .floor();
    (min, initial)
}

/// Gemerkte Optionen aus den Voreinstellungen (sonst die Vorgabe „Logo“), mit dem zuletzt
/// gewählten Verfahren.
pub fn remembered_options(st: &AppState) -> TraceOptions {
    let mut o = options_from_json(&st.session.prefs.get_str(PREF_OPTIONS, ""));
    if let Some(e) = Engine::from_name(&st.session.prefs.get_str(PREF_ENGINE, "")) {
        o.engine = e;
    }
    o
}

/// Gemerkte Optionen lesen. Ältere Einträge (vor dem Aufräumen) haben kein `refine`: sie bekommen
/// die Werte der gewählten Vorgabe (Logo: an), nicht „aus“. Ungültig/leer → Vorgabe „Logo“.
pub fn options_from_json(s: &str) -> TraceOptions {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(s) else {
        return TraceOptions::default();
    };
    let Ok(mut o) = serde_json::from_value::<TraceOptions>(v.clone()) else {
        return TraceOptions::default();
    };
    if v.get("refine").is_none_or(|r| r.is_null()) {
        o.refine = TraceOptions::from_preset(o.preset).refine;
    }
    o.validated()
}

/// Öffnet den Dialog für die ausgewählten Bitmaps; `false` ohne Bitmap (der Befehl meldet dann
/// selbst den Fehler).
pub fn open(st: &mut AppState) -> bool {
    if !st.session.is_available("modify.traceBitmap") {
        return false;
    }
    let images = selected_bitmaps(st);
    let n = images.len();
    let Some(first) = images.into_iter().next() else {
        return false;
    };
    st.dialogs.trace = Some(TraceDialog::new(
        first,
        n,
        remembered_options(st),
        st.session.prefs.get_bool(PREF_KEEP, true),
    ));
    true
}

impl TraceDialog {
    /// Neuer Dialog für ein Bild (`bitmaps` = Zahl der ausgewählten Bitmaps).
    pub fn new(
        source: RgbaImage,
        bitmaps: usize,
        opts: TraceOptions,
        keep_original: bool,
    ) -> TraceDialog {
        TraceDialog {
            opts,
            keep_original,
            view: ViewMode::Split,
            split: 0.5,
            zoom: None,
            pan: Vec2::ZERO,
            advanced: false,
            stats: None,
            error: None,
            runs_started: 0,
            source: Arc::new(source),
            bitmaps,
            original_tex: None,
            vector_tex: None,
            result: None,
            result_opts: None,
            outlines: Vec::new(),
            job: None,
            next_run: 1,
            dirty: true,
            last_edit: f64::NEG_INFINITY,
            scroll_acc: 0.0,
            scroll_top: true,
        }
    }

    /// Läuft gerade eine Berechnung (oder steht eine an)?
    pub fn is_busy(&self) -> bool {
        self.job.is_some() || self.dirty
    }

    /// Letztes Ergebnis.
    pub fn result(&self) -> Option<&Arc<TraceResult>> {
        self.result.as_ref()
    }

    /// Letztes Vorschauergebnis, wenn es genau zu den aktuellen Optionen gehört und nichts mehr
    /// rechnet – OK übernimmt es dann, statt neu zu rechnen.
    pub fn reusable_result(&self) -> Option<&Arc<TraceResult>> {
        if self.dirty || self.job.is_some() || self.error.is_some() {
            return None;
        }
        let r = self.result.as_ref()?;
        (self.result_opts.as_ref() == Some(&self.opts.validated())).then_some(r)
    }

    /// Vorgabe wählen: alle Optionen auf deren Werte, sofort neu berechnen. Verfahren und die
    /// Werte des Konturverfahrens bleiben (die Vorgaben legen nur Farben bzw. Schwelle fest).
    pub fn set_preset(&mut self, p: Preset) {
        let (engine, potrace) = (self.opts.engine, self.opts.potrace.clone());
        self.opts = TraceOptions {
            engine,
            potrace,
            ..TraceOptions::from_preset(p)
        };
        self.changed(f64::NEG_INFINITY);
    }

    /// Verfahren wählen (VT-6): übrige Optionen bleiben, sofort neu berechnen.
    pub fn set_engine(&mut self, e: Engine) {
        if self.opts.engine != e {
            self.opts.engine = e;
            self.changed(f64::NEG_INFINITY);
        }
    }

    /// Optionen geändert: neu berechnen (nach kurzer Sammelzeit, wenn gerade gerechnet wird).
    fn changed(&mut self, now: f64) {
        self.opts = self.opts.validated();
        self.dirty = true;
        self.last_edit = now;
    }

    /// Befehlsparameter für `modify.traceBitmap` (alle Optionen + `keepOriginal`).
    pub fn command_params(&self) -> serde_json::Value {
        let mut v = serde_json::to_value(self.opts.validated()).unwrap_or_else(|_| json!({}));
        v["keepOriginal"] = json!(self.keep_original);
        v
    }

    /// Darf OK ausgeführt werden (kein Fehler, nicht leer)?
    fn can_apply(&self) -> bool {
        self.error.is_none() && self.stats.as_ref().is_none_or(|s| s.paths > 0)
    }

    // ------------------------------------------------------------------ Hintergrund

    fn start(&mut self, ctx: &egui::Context) {
        // alter Lauf: abbrechen (Drop setzt das Flag)
        self.job = None;
        self.dirty = false;
        let run = self.next_run;
        self.next_run += 1;
        self.runs_started += 1;
        let cancel = Arc::new(AtomicBool::new(false));
        let progress = Arc::new(AtomicU32::new(0f32.to_bits()));
        let stage = Arc::new(AtomicU32::new(0));
        let (tx, rx) = mpsc::channel();
        let img = Arc::clone(&self.source);
        let opts = self.opts.validated();
        let (c2, p2, s2, ctx2) = (
            Arc::clone(&cancel),
            Arc::clone(&progress),
            Arc::clone(&stage),
            ctx.clone(),
        );
        let spawned = std::thread::Builder::new()
            .name("fp-trace-vorschau".into())
            .spawn(move || {
                let r = compute(&img, &opts, &c2, &p2, &s2);
                let _ = tx.send(Done { run, result: r });
                ctx2.request_repaint();
            });
        match spawned {
            Ok(_) => {
                self.job = Some(Job {
                    run,
                    cancel,
                    progress,
                    stage,
                    rx,
                })
            }
            Err(e) => self.error = Some(fp_i18n::tr!("trace-err-internal", detail = e.to_string())),
        }
    }

    /// Ergebnis des laufenden Laufs abholen (Texturen anlegen).
    fn poll(&mut self, ctx: &egui::Context) {
        let Some(job) = &self.job else { return };
        let msg = match job.rx.try_recv() {
            Ok(m) => m,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => {
                self.job = None;
                return;
            }
        };
        if msg.run != job.run {
            return;
        }
        self.job = None;
        match msg.result {
            None => {}
            Some(Ok(r)) => {
                let img = egui::ColorImage::from_rgba_unmultiplied(
                    [r.preview.width(), r.preview.height()],
                    r.preview.as_bytes(),
                );
                self.vector_tex =
                    Some(ctx.load_texture("fp-trace-vector", img, egui::TextureOptions::LINEAR));
                self.result = Some(r.result);
                self.result_opts = Some(r.opts);
                self.outlines = r.outlines;
                self.stats = Some(r.stats);
                self.error = None;
            }
            Some(Err(e)) => {
                self.error = Some(e);
                self.stats = None;
            }
        }
    }

    fn ensure_original_texture(&mut self, ctx: &egui::Context) {
        if self.original_tex.is_some() {
            return;
        }
        let src = &*self.source;
        let long = src.width().max(src.height()) as f32;
        let shown: std::borrow::Cow<RgbaImage> = if long > PREVIEW_MAX {
            let f = PREVIEW_MAX / long;
            let (w, h) = (
                ((src.width() as f32 * f).round() as u32).max(1),
                ((src.height() as f32 * f).round() as u32).max(1),
            );
            std::borrow::Cow::Owned(fp_raster::resample::resize(
                src,
                w,
                h,
                fp_raster::Sampler::Bilinear,
            ))
        } else {
            std::borrow::Cow::Borrowed(src)
        };
        let img = egui::ColorImage::from_rgba_unmultiplied(
            [shown.width(), shown.height()],
            shown.as_bytes(),
        );
        // Original pixelig vergrößern (zeigt den Unterschied zum Vektor), verkleinert geglättet
        let opts = egui::TextureOptions {
            minification: egui::TextureFilter::Linear,
            ..egui::TextureOptions::NEAREST
        };
        self.original_tex = Some(ctx.load_texture("fp-trace-original", img, opts));
    }

    // ------------------------------------------------------------------ Anwenden

    /// OK: Befehl ausführen und Werte merken. Das fertige Vorschauergebnis wird angeboten
    /// (keine zweite Rechnung, wenn sich seither nichts geändert hat).
    pub fn apply(&self, st: &mut AppState) -> Result<(), String> {
        use fp_engine::commands::trace as cmd_trace;
        if let Some(r) = self.reusable_result() {
            cmd_trace::offer_prepared(Arc::clone(&self.source), &self.opts, Arc::clone(r));
        }
        let out = crate::ui_commands::execute(
            &mut st.session,
            "modify.traceBitmap",
            self.command_params(),
        );
        cmd_trace::clear_prepared();
        out.map_err(|e| e.to_string())?;
        self.remember(st);
        Ok(())
    }

    fn remember(&self, st: &mut AppState) {
        if let Ok(s) = serde_json::to_string(&self.opts.validated()) {
            st.session
                .prefs
                .set(PREF_OPTIONS, fp_engine::prefs::PrefValue::Str(s));
        }
        st.session.prefs.set(
            PREF_KEEP,
            fp_engine::prefs::PrefValue::Bool(self.keep_original),
        );
        st.session.prefs.set(
            PREF_ENGINE,
            fp_engine::prefs::PrefValue::Str(self.opts.engine.name().into()),
        );
    }

    /// „Als SVG speichern…“: Datei-Dialog (nicht blockierend), dann schreiben.
    fn save_svg(&self, st: &mut AppState) {
        let Some(res) = self.result.clone() else {
            return;
        };
        self.remember(st);
        let req = crate::file_dialog::Request::save(
            t("trace-save-svg-title"),
            format!("{}.svg", t("trace-layer-name")),
        )
        .filter(t("trace-svg-filter"), &["svg"]);
        crate::file_dialog::request(st, req, move |st, paths| {
            let Some(p) = paths.into_iter().next() else {
                return;
            };
            let p = if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("svg")) {
                p
            } else {
                p.with_extension("svg")
            };
            match svg_string(&res).and_then(|s| std::fs::write(&p, s).map_err(|e| e.to_string())) {
                Ok(()) => st.push(Action::Status(fp_i18n::tr!(
                    "trace-svg-saved",
                    path = p.display().to_string()
                ))),
                Err(e) => {
                    st.dialogs.message = Some((
                        t("trace-save-svg-title"),
                        fp_i18n::tr!("trace-svg-error", detail = e),
                    ))
                }
            }
        });
    }
}

// ---------------------------------------------------------------------- Rechnung (Hintergrund)

fn stage_code(s: Stage) -> u32 {
    match s {
        Stage::Prepare => 0,
        Stage::Cluster => 1,
        Stage::Paths => 2,
        Stage::Done => 3,
    }
}

fn compute(
    img: &RgbaImage,
    opts: &TraceOptions,
    cancel: &AtomicBool,
    progress: &AtomicU32,
    stage: &AtomicU32,
) -> Option<Result<Ready, String>> {
    let t0 = Instant::now();
    let (res, report) = match fp_trace::trace_with_report(img, opts, cancel, &mut |p| {
        // Bild für die Vorschau kommt noch dazu: 90 % für das Nachzeichnen
        progress.store((p.fraction * 0.9).to_bits(), Ordering::Relaxed);
        stage.store(stage_code(p.stage), Ordering::Relaxed);
    }) {
        Ok(r) => r,
        Err(fp_trace::TraceError::Cancelled) => return None,
        Err(e) => return Some(Err(e.to_string())),
    };
    if cancel.load(Ordering::Relaxed) {
        return None;
    }
    let preview = render_preview(&res);
    if cancel.load(Ordering::Relaxed) {
        return None;
    }
    let match_pct = match_percent(img, &res, opts);
    let outlines = outlines(&res);
    let colors = {
        let mut v: Vec<[u8; 3]> = res
            .paths
            .iter()
            .map(|p| [p.color.r, p.color.g, p.color.b])
            .collect();
        v.sort_unstable();
        v.dedup();
        v.len()
    };
    let stats = Stats {
        paths: res.paths.len(),
        points: res.node_count(),
        colors,
        match_pct,
        ms: t0.elapsed().as_millis() as u64,
        downscaled: res
            .downscaled()
            .then_some((res.traced_width, res.traced_height)),
        refine: report,
        faint_alpha: if res.paths.is_empty() {
            faint_alpha(img, opts)
        } else {
            None
        },
    };
    progress.store(1f32.to_bits(), Ordering::Relaxed);
    Some(Ok(Ready {
        opts: opts.clone(),
        result: Arc::new(res),
        preview,
        outlines,
        stats,
    }))
}

/// Ergebnis mit Maßstab `s` in ein Bild ab (0, 0) rastern.
fn render_scaled(res: &TraceResult, s: f32, w: u32, h: u32) -> RgbaImage {
    let (l, t) = (res.bounds.left as f32, res.bounds.top as f32);
    let map = |p: fp_geom::Point| fp_geom::Point::new((p.x - l) * s, (p.y - t) * s);
    let paths = res
        .paths
        .iter()
        .map(|p| fp_trace::TracedPath {
            contours: p
                .contours
                .iter()
                .map(|c| {
                    fp_geom::BezierContour::new(
                        c.nodes
                            .iter()
                            .map(|n| fp_geom::BezierNode {
                                pt: map(n.pt),
                                ctrl_in: map(n.ctrl_in),
                                ctrl_out: map(n.ctrl_out),
                            })
                            .collect(),
                        c.closed,
                    )
                })
                .collect(),
            color: p.color,
            pixels: p.pixels,
        })
        .collect();
    let scaled = TraceResult {
        paths,
        bounds: fp_geom::IRect::from_xywh(0, 0, w as i32, h as i32),
        traced_width: w,
        traced_height: h,
    };
    scaled.render()
}

/// Vorschau mit Kantenglättung: doppelt so groß rastern, dann 2 × 2 gemittelt verkleinern.
fn render_preview(res: &TraceResult) -> RgbaImage {
    let (w, h) = (
        res.bounds.width().max(1) as f32,
        res.bounds.height().max(1) as f32,
    );
    let k = (PREVIEW_MAX / w.max(h)).min(4.0);
    let (pw, ph) = (
        ((w * k).ceil() as u32).max(1),
        ((h * k).ceil() as u32).max(1),
    );
    let big = render_scaled(res, 2.0 * k, pw * 2, ph * 2);
    downsample2(&big, pw, ph)
}

/// 2 × 2-Mittel (vormultipliziert, damit Kanten zu Transparenz nicht dunkel werden).
fn downsample2(src: &RgbaImage, w: u32, h: u32) -> RgbaImage {
    let Ok(mut out) = RgbaImage::new(fp_geom::IRect::from_xywh(0, 0, w as i32, h as i32)) else {
        return src.clone();
    };
    let sw = src.width();
    let sd = src.data();
    let od = out.data_mut();
    for y in 0..h as usize {
        for x in 0..w as usize {
            let mut acc = [0u32; 4];
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let i = (2 * y + dy) * sw + 2 * x + dx;
                if let Some(p) = sd.get(i) {
                    let a = p[3] as u32;
                    acc[0] += p[0] as u32 * a;
                    acc[1] += p[1] as u32 * a;
                    acc[2] += p[2] as u32 * a;
                    acc[3] += a;
                }
            }
            if let (Some(r), Some(g), Some(b)) = (
                acc[0].checked_div(acc[3]),
                acc[1].checked_div(acc[3]),
                acc[2].checked_div(acc[3]),
            ) {
                od[y * w as usize + x] = [r as u8, g as u8, b as u8, (acc[3] / 4) as u8];
            }
        }
    }
    out
}

/// Anteil der Pixel (0–100), die im Ergebnis fast gleich aussehen wie im Original.
/// Transparent (bzw. in Schwarzweiß: hell) zählt als „keine Fläche“.
pub fn match_percent(src: &RgbaImage, res: &TraceResult, opts: &TraceOptions) -> f32 {
    let (w, h) = (src.width(), src.height());
    if w == 0 || h == 0 {
        return 0.0;
    }
    // große Bilder auf ≤ 1 MP vergleichen
    let limit = 1_000_000f64;
    let (cmp_src, rendered): (std::borrow::Cow<RgbaImage>, RgbaImage) = if (w * h) as f64 > limit {
        let f = (limit / (w * h) as f64).sqrt() as f32;
        let (cw, ch) = (
            ((w as f32 * f).floor() as u32).max(1),
            ((h as f32 * f).floor() as u32).max(1),
        );
        (
            std::borrow::Cow::Owned(fp_raster::resample::resize(
                src,
                cw,
                ch,
                fp_raster::Sampler::Bilinear,
            )),
            render_scaled(res, cw as f32 / w as f32, cw, ch),
        )
    } else {
        (std::borrow::Cow::Borrowed(src), res.render())
    };
    let o = opts.validated();
    let bw = o.color_mode == ColorMode::BlackWhite;
    let mut same = 0usize;
    let total = cmp_src.data().len().min(rendered.data().len());
    for (s, r) in cmp_src.data().iter().zip(rendered.data().iter()) {
        let sp: Option<[i32; 3]> = if s[3] < o.alpha_threshold && o.ignore_transparent {
            None
        } else {
            // auf Weiß verrechnen
            let a = s[3] as i32;
            let c = |v: u8| (v as i32 * a + 255 * (255 - a)) / 255;
            let rgb = [c(s[0]), c(s[1]), c(s[2])];
            if bw {
                let l = (rgb[0] * 299 + rgb[1] * 587 + rgb[2] * 114) / 1000;
                (l < o.threshold as i32).then_some([0, 0, 0])
            } else {
                Some(rgb)
            }
        };
        let rp: Option<[i32; 3]> = (r[3] >= 128).then_some([r[0] as i32, r[1] as i32, r[2] as i32]);
        let ok = match (sp, rp) {
            (None, None) => true,
            (Some(a), Some(b)) => (0..3).all(|k| (a[k] - b[k]).abs() <= MATCH_TOLERANCE),
            _ => false,
        };
        if ok {
            same += 1;
        }
    }
    if total == 0 {
        0.0
    } else {
        same as f32 * 100.0 / total as f32
    }
}

/// Konturen als Polylinien in Bildpixeln (relativ zur linken oberen Bildecke).
fn outlines(res: &TraceResult) -> Vec<Vec<Pos2>> {
    let (l, t) = (res.bounds.left as f32, res.bounds.top as f32);
    let mut out = Vec::new();
    let mut total = 0usize;
    for p in res.paths.iter() {
        for c in &p.contours {
            let n = c.nodes.len();
            if n < 2 {
                continue;
            }
            let mut pts = Vec::with_capacity(n * 4);
            for i in 0..n {
                let a = &c.nodes[i];
                let b = &c.nodes[(i + 1) % n];
                let (p0, p1, p2, p3) = (a.pt, a.ctrl_out, b.ctrl_in, b.pt);
                let curved = p1 != p0 || p2 != p3;
                let len = (p0.x - p3.x).hypot(p0.y - p3.y);
                let steps = if curved {
                    (len / 3.0).ceil().clamp(2.0, 16.0) as usize
                } else {
                    1
                };
                for k in 0..steps {
                    let u = k as f32 / steps as f32;
                    let m = 1.0 - u;
                    let f = |a: f32, b: f32, c: f32, d: f32| {
                        m * m * m * a + 3.0 * m * m * u * b + 3.0 * m * u * u * c + u * u * u * d
                    };
                    pts.push(pos2(
                        f(p0.x, p1.x, p2.x, p3.x) - l,
                        f(p0.y, p1.y, p2.y, p3.y) - t,
                    ));
                }
            }
            total += pts.len();
            out.push(pts);
            if total > OUTLINE_POINTS_MAX {
                return out;
            }
        }
    }
    out
}

/// SVG des Ergebnisses über den SVG-Schreiber von fp-export (Leinwand = Bildgröße, transparent).
pub fn svg_string(res: &TraceResult) -> Result<String, String> {
    let b = res.bounds;
    let mut doc = fp_model::Document::new(
        b.width().max(1) as f32,
        b.height().max(1) as f32,
        fp_color::Argb(0x00FF_FFFF),
    );
    let mut ids = std::mem::take(&mut doc.ids);
    let (l, t) = (b.left as f32, b.top as f32);
    let shift = |p: fp_geom::Point| fp_geom::Point::new(p.x - l, p.y - t);
    let mut children = Vec::with_capacity(res.paths.len());
    for tp in res.top_to_bottom() {
        let contours = tp
            .contours
            .iter()
            .map(|c| {
                fp_engine::geometry::from_bezier(&fp_geom::BezierContour::new(
                    c.nodes
                        .iter()
                        .map(|n| fp_geom::BezierNode {
                            pt: shift(n.pt),
                            ctrl_in: shift(n.ctrl_in),
                            ctrl_out: shift(n.ctrl_out),
                        })
                        .collect(),
                    c.closed,
                ))
            })
            .collect();
        let attrs = fp_model::PathAttrs {
            fill_color: tp.color.to_argb(),
            fill: Some(fp_model::Fill::solid()),
            brush: None,
            ..fp_model::PathAttrs::default()
        };
        let mut e = fp_model::Element::new_path(&mut ids, contours, attrs);
        if let fp_model::ElementKind::Path(pe) = &mut e.kind {
            pe.even_odd = true;
        }
        children.push(Arc::new(e));
    }
    let g = fp_model::Element::new_group(&mut ids, children);
    doc.ids = ids;
    if let Some(fp_model::layer::LayerSlot::Own(layer)) =
        doc.page_mut(0).and_then(|p| p.root.layers.first_mut())
    {
        let layer = Arc::make_mut(layer);
        if let Some(cel) = layer.cels.first_mut() {
            Arc::make_mut(cel).elements.push(Arc::new(g));
        }
    }
    fp_export::vector_out::write_svg(&doc, 0, 0, None).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------- Anzeige

/// Zeigt den Dialog (aus `dialogs::show_all`).
pub fn show(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut d) = st.dialogs.trace.take() else {
        return;
    };
    let now = ctx.input(|i| i.time);
    d.poll(ctx);
    // sofort, wenn nichts läuft; sonst erst nach einer kurzen Pause der Eingaben (kein Abbruchsturm)
    if d.dirty && (d.job.is_none() || now - d.last_edit >= DEBOUNCE) {
        d.start(ctx);
    }
    if d.is_busy() {
        ctx.request_repaint_after(Duration::from_millis(40));
    }
    d.ensure_original_texture(ctx);
    let screen = ctx.content_rect();
    let remembered = remembered_size(st);
    let (min, initial) = dialog_sizes(screen.size(), remembered);
    let mut outcome: Option<bool> = None;
    let mut want_svg = false;
    let title = t("trace-dlg-title");
    let engine_before = d.opts.engine;
    let r = dialog_frame_resizable_sized(ctx, DIALOG_ID, &title, min, initial, |ui, body| {
        let (o, svg) = d.ui(ui, body, now);
        outcome = o;
        want_svg = svg;
    });
    // gewähltes Verfahren sofort merken (auch bei Abbrechen)
    if d.opts.engine != engine_before {
        st.session.prefs.set(
            PREF_ENGINE,
            fp_engine::prefs::PrefValue::Str(d.opts.engine.name().into()),
        );
    }
    // vom Nutzer gezogene Größe merken (gilt beim nächsten Öffnen, auch nach einem Neustart)
    if let Some(g) = super::dialog_geom(ctx, DIALOG_ID) {
        let client = g.client.round();
        if client != initial.round() && Some(client) != remembered.map(|v| v.round()) {
            st.session.prefs.set(
                PREF_SIZE,
                fp_engine::prefs::PrefValue::Str(format!("{} {}", client.x, client.y)),
            );
        }
    }
    if want_svg {
        d.save_svg(st);
    }
    match (outcome, r) {
        (Some(true), _) => {
            if let Err(e) = d.apply(st) {
                st.dialogs.message = Some((t("trace-apply-error"), e));
            }
        }
        (Some(false), _) | (_, DialogClose::Cancelled) => {}
        _ => st.dialogs.trace = Some(d),
    }
}

impl TraceDialog {
    /// Inhalt; liefert (OK/Abbrechen, „Als SVG speichern“ gedrückt).
    fn ui(&mut self, ui: &mut Ui, body: Rect, now: f64) -> (Option<bool>, bool) {
        let theme = Theme::current(ui.ctx());
        // Kopf (bei wenig Höhe ohne Leitsatz)
        let compact = body.height() < 470.0;
        let lead = t("trace-dlg-lead");
        let title = t("trace-dlg-title");
        let header = {
            let mut hui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(body)
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            ui_kit::dialog_header(
                &mut hui,
                Some(HeaderIcon::Icon(icons::Icon::Pen)),
                &title,
                (!compact).then_some(lead.as_str()),
            )
        };
        // eigenes Symbol über dem Platzhalter: Pixel werden zur Kurve
        let ir = Rect::from_min_size(
            pos2(header.left() + sp::L, header.top() + sp::M),
            vec2(32.0, 32.0),
        );
        ui.painter()
            .rect_filled(ir.expand(1.0), 0.0, ui_kit::card_bg(&theme));
        paint_header_icon(ui.painter(), ir, &theme);

        // Knopfleiste
        let (svg_l, ok_l, cancel_l, keep_l) = (
            t("trace-save-svg"),
            t("btn-ok"),
            t("btn-cancel"),
            t("trace-keep-original"),
        );
        let left = [BarButton::new(&svg_l).enabled(self.result.is_some() && self.error.is_none())];
        let right = [
            BarButton::default_button(&ok_l).enabled(self.can_apply()),
            BarButton::new(&cancel_l),
        ];
        let click = ui_kit::button_bar(ui, body, &left, &right);
        let bar_top = body.bottom() - sp::BUTTON_BAR_H;
        // „Original behalten“ neben „Als SVG speichern…“
        let bw = super::buttons_w(ui.ctx(), &[&svg_l, &ok_l, &cancel_l], sp::BUTTON_MIN_W);
        let kx = body.left() + sp::L + bw + sp::L;
        let kmax = body.right() - sp::L - 2.0 * bw - sp::S - sp::L;
        let kr = Rect::from_min_max(
            pos2(kx, bar_top + 1.0 + sp::M),
            pos2(kmax.max(kx + 40.0), bar_top + 1.0 + sp::M + sp::BUTTON_H),
        );
        if super::text_w(ui.ctx(), &keep_l) + 18.0 > kr.width() + 0.5 {
            report_overflow(ui.ctx(), format!("trace keep {:.0}", kr.width()));
        }
        if let Some(v) = fields::checkbox(ui, "trace-keep", kr, &keep_l, self.keep_original, true) {
            self.keep_original = v;
        }

        // Mitte: Einstellungen links, Vorschau rechts
        let mid = Rect::from_min_max(
            pos2(body.left() + sp::L, header.bottom() + sp::M),
            pos2(body.right() - sp::L, bar_top - sp::M),
        );
        let lw = (mid.width() * 0.38)
            .clamp(250.0, 330.0)
            .min(mid.width() - 160.0)
            .max(200.0);
        let left_r = Rect::from_min_max(mid.min, pos2(mid.left() + lw, mid.bottom()));
        let right_r = Rect::from_min_max(pos2(left_r.right() + sp::L, mid.top()), mid.max);
        {
            let mut lui = ui.new_child(
                egui::UiBuilder::new()
                    .id_salt("trace-left")
                    .max_rect(left_r)
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            let mut area = egui::ScrollArea::vertical()
                .id_salt("trace-settings")
                .auto_shrink([false, false])
                .max_height(left_r.height());
            if std::mem::take(&mut self.scroll_top) {
                area = area.vertical_scroll_offset(0.0);
            }
            area.show(&mut lui, |ui| {
                ui.set_width(left_r.width() - 12.0);
                self.settings_ui(ui, now);
            });
        }
        self.preview_ui(ui, right_r, &theme, now);

        let outcome = match click.right {
            Some(0) => Some(true),
            Some(1) => Some(false),
            _ => None,
        };
        (outcome, click.left == Some(0))
    }

    // ------------------------------------------------------------------ Einstellungen

    fn settings_ui(&mut self, ui: &mut Ui, now: f64) {
        let theme = Theme::current(ui.ctx());
        // Verfahren (VT-6): eine Zeile über den Vorgaben, Erklärung als Tooltip
        self.engine_row(ui, &theme);
        ui.add_space(sp::M);
        ui_kit::section(ui, &t("trace-dlg-presets"));
        ui.add_space(sp::XS);
        self.preset_cards(ui, &theme);
        if self.opts.preset == Preset::Photo {
            // ehrlicher Hinweis: Fotos werden zu Farbflächen (kein Pixelbild mehr)
            ui.add_space(sp::S);
            ui_kit::notice(ui, Tone::Info, None, &t("trace-photo-note"));
        }
        ui.add_space(sp::L);
        ui_kit::section(ui, &t("trace-dlg-settings"));
        ui.add_space(sp::XS);
        let mut o = self.opts.clone();
        let bw = o.color_mode == ColorMode::BlackWhite;
        if bw {
            if let Some(v) = slider_row(
                ui,
                "threshold",
                &t("trace-opt-threshold"),
                o.threshold as f64,
                (0.0, 255.0),
                0,
                1.0,
                "",
                Some((
                    &t("trace-opt-threshold-low"),
                    &t("trace-opt-threshold-high"),
                )),
                true,
            ) {
                o.threshold = v.round() as u8;
            }
        } else if let Some(v) = slider_row(
            ui,
            "colors",
            &t("trace-opt-colors"),
            o.colors as f64,
            (2.0, 64.0),
            0,
            1.0,
            "",
            None,
            true,
        ) {
            o.colors = v.round() as u32;
        }
        if o.engine == Engine::Potrace {
            self.potrace_main_ui(ui, &mut o);
        } else {
            self.clusters_main_ui(ui, &mut o);
        }
        ui.add_space(sp::S);
        // Erweitert (einklappbar)
        let (hr, hresp) = ui.allocate_exact_size(vec2(ui.available_width(), 22.0), Sense::click());
        let adv = t("trace-dlg-advanced");
        hresp.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::CollapsingHeader,
                true,
                self.advanced,
                adv.clone(),
            )
        });
        if hresp.clicked() {
            self.advanced = !self.advanced;
        }
        {
            let p = ui.painter();
            let c = &theme.colors;
            if hresp.hovered() {
                p.rect_filled(hr, CornerRadius::same(3), with_alpha(c.list_hover_bg, 160));
            }
            let a = pos2(hr.left() + 7.0, hr.center().y);
            let pts = if self.advanced {
                vec![
                    a + vec2(-4.0, -2.0),
                    a + vec2(4.0, -2.0),
                    a + vec2(0.0, 3.0),
                ]
            } else {
                vec![
                    a + vec2(-2.0, -4.0),
                    a + vec2(3.0, 0.0),
                    a + vec2(-2.0, 4.0),
                ]
            };
            p.add(Shape::convex_polygon(pts, c.section_label, Stroke::NONE));
            let tr = Rect::from_min_max(
                pos2(hr.left() + 18.0, hr.top()),
                pos2(hr.right(), hr.bottom()),
            );
            let font = theme.bold(type_scale::SECTION);
            let shown = ui_kit::elide(ui.ctx(), &adv, &font, (tr.width() - 24.0).max(20.0));
            let tw = ui_kit::text_w(ui.ctx(), &shown, &font);
            p.text(
                pos2(tr.left(), tr.center().y),
                Align2::LEFT_CENTER,
                shown,
                font,
                c.section_label,
            );
            if tr.left() + tw + sp::S < tr.right() - 4.0 {
                ui_kit::hline(
                    p,
                    egui::Rangef::new(tr.left() + tw + sp::S, tr.right()),
                    tr.center().y.floor(),
                    &theme,
                );
            }
        }
        if self.advanced {
            ui.add_space(sp::XS);
            self.advanced_ui(ui, &mut o);
        }
        ui.add_space(sp::S);
        if o != self.opts {
            self.opts = o;
            self.changed(now);
        }
    }

    /// Zeile „Verfahren“: Farbflächen (bisher) oder Konturen (Potrace-Verfahren, VT-6).
    fn engine_row(&mut self, ui: &mut Ui, theme: &Theme) {
        let w = ui.available_width();
        let (r, _) = ui.allocate_exact_size(vec2(w, 22.0), Sense::hover());
        let label = t("trace-dlg-engine");
        let font = theme.bold(type_scale::BODY);
        let lw = (ui_kit::text_w(ui.ctx(), &label, &font) + sp::M).min(w * 0.45);
        let lr = Rect::from_min_size(r.min, vec2(lw, r.height()));
        if ui_kit::paint_text_fit(ui.painter(), lr, &label, font, theme.colors.text) {
            report_overflow(ui.ctx(), format!("trace engine label {:.0}", lr.width()));
        }
        let items = [t("trace-engine-clusters"), t("trace-engine-potrace")];
        let tips = [
            t("trace-engine-clusters-hint"),
            t("trace-engine-potrace-hint"),
        ];
        let refs: Vec<&str> = items.iter().map(|s| s.as_str()).collect();
        let tip_refs: Vec<&str> = tips.iter().map(|s| s.as_str()).collect();
        let sel = Engine::ALL
            .iter()
            .position(|e| *e == self.opts.engine)
            .unwrap_or(0);
        let sr = Rect::from_min_max(
            pos2(r.left() + lw, r.top() + 1.0),
            pos2(r.right(), r.bottom() - 1.0),
        );
        if let Some(i) =
            segmented_tips(ui, Id::new("trace-engine"), sr, &refs, &tip_refs, sel, true)
        {
            self.set_engine(Engine::ALL[i]);
        }
    }

    /// Hauptregler des Konturverfahrens: Flecken (Fläche), Ecken (alphaMax), Kurven zusammenfassen.
    fn potrace_main_ui(&mut self, ui: &mut Ui, o: &mut TraceOptions) {
        let px = fp_i18n::unit("px");
        let px2 = t("trace-unit-px2");
        let pr = &mut o.potrace;
        if let Some(v) = slider_row(
            ui,
            "turd",
            &t("trace-opt-speckle"),
            pr.turd_size as f64,
            (0.0, 100.0),
            0,
            1.0,
            &px2,
            None,
            true,
        ) {
            pr.turd_size = v.round() as u32;
        }
        if let Some(v) = slider_row(
            ui,
            "alphamax",
            &t("trace-opt-corners"),
            pr.alpha_max,
            (0.0, fp_trace::ALPHA_MAX_LIMIT),
            2,
            0.05,
            "",
            Some((&t("trace-opt-corners-low"), &t("trace-opt-corners-high"))),
            true,
        ) {
            pr.alpha_max = v;
        }
        if let Some(v) = check_row(
            ui,
            "trace-optcurve",
            &t("trace-potrace-optcurve"),
            pr.opt_curve,
            true,
        ) {
            pr.opt_curve = v;
        }
        if let Some(v) = tolerance_row(
            ui,
            "opttol",
            &t("trace-potrace-opttol"),
            pr.opt_tolerance,
            (0.0, 1.0),
            2,
            0.05,
            &px,
            pr.opt_curve,
        ) {
            pr.opt_tolerance = v;
        }
        caption_text(ui, &t("trace-potrace-optcurve-hint"), CHECK_INDENT, true);
    }

    /// Hauptregler der Farbcluster: Detail und Glätte, Ecken, Flecken, Aufräumen.
    fn clusters_main_ui(&mut self, ui: &mut Ui, o: &mut TraceOptions) {
        let px = fp_i18n::unit("px");
        // „Detail und Glätte“: beim Aufräumen die Kurventoleranz, sonst die Mindestlänge der Segmente
        let ends = (
            t("trace-opt-smoothness-low"),
            t("trace-opt-smoothness-high"),
        );
        if o.refine.enabled {
            if let Some(v) = slider_row(
                ui,
                "smooth",
                &t("trace-opt-smoothness"),
                o.refine.fit_tolerance,
                (0.2, 4.0),
                1,
                0.1,
                &px,
                Some((&ends.0, &ends.1)),
                true,
            ) {
                o.refine.fit_tolerance = v;
            }
        } else if let Some(v) = slider_row(
            ui,
            "smooth",
            &t("trace-opt-smoothness"),
            o.length_threshold,
            (3.5, 10.0),
            1,
            0.5,
            &px,
            Some((&ends.0, &ends.1)),
            true,
        ) {
            o.length_threshold = v;
        }
        let deg = fp_i18n::unit("deg");
        let poly = o.mode == CurveMode::Polygon;
        if let Some(v) = slider_row(
            ui,
            "corners",
            &t("trace-opt-corners"),
            o.corner_threshold,
            (0.0, 180.0),
            0,
            5.0,
            &deg,
            Some((&t("trace-opt-corners-low"), &t("trace-opt-corners-high"))),
            !poly,
        ) {
            o.corner_threshold = v;
        }
        if let Some(v) = slider_row(
            ui,
            "speckle",
            &t("trace-opt-speckle"),
            o.filter_speckle as f64,
            (0.0, 32.0),
            0,
            1.0,
            &px,
            None,
            true,
        ) {
            o.filter_speckle = v.round() as u32;
        }
        // Aufräumen (Hauptschalter), darunter ein Satz, was es tut
        if let Some(v) = check_row(
            ui,
            "trace-refine",
            &t("trace-refine"),
            o.refine.enabled,
            true,
        ) {
            o.refine.enabled = v;
        }
        caption_text(ui, &t("trace-refine-hint"), CHECK_INDENT, true);
    }

    fn preset_cards(&mut self, ui: &mut Ui, theme: &Theme) {
        let w = ui.available_width();
        let gap = sp::S;
        let cw = ((w - gap) / 2.0).floor();
        let font_hint = theme.font(type_scale::CAPTION);
        let entries: Vec<(Preset, String, String)> = Preset::ALL
            .iter()
            .map(|p| {
                (
                    *p,
                    t(preset_key(*p)),
                    t(&format!("{}-hint", preset_key(*p))),
                )
            })
            .collect();
        let hint_w = cw - 2.0 * 10.0;
        let hint_h = entries
            .iter()
            .map(|(_, _, h)| {
                ui.ctx().fonts_mut(|f| {
                    f.layout(h.clone(), font_hint.clone(), Color32::WHITE, hint_w)
                        .size()
                        .y
                })
            })
            .fold(0.0, f32::max);
        // passt ein Name nicht neben das Symbol, stehen alle Namen darunter (umbrochen)
        let name_font = theme.bold(type_scale::BODY);
        let beside = cw - 10.0 - 22.0 - sp::S - 8.0 - 18.0;
        let stacked = entries
            .iter()
            .any(|(_, n, _)| ui_kit::text_w(ui.ctx(), n, &name_font) > beside);
        let name_h = if stacked {
            entries
                .iter()
                .map(|(_, n, _)| {
                    ui.ctx().fonts_mut(|f| {
                        f.layout(n.clone(), name_font.clone(), Color32::WHITE, hint_w)
                            .size()
                            .y
                    })
                })
                .fold(0.0, f32::max)
                + sp::XS
        } else {
            0.0
        };
        let ch = (10.0 + 22.0 + 6.0 + name_h + hint_h + 10.0).ceil();
        for row in 0..2 {
            let (rr, _) = ui.allocate_exact_size(vec2(w, ch), Sense::hover());
            for col in 0..2 {
                let (p, name, hint) = &entries[row * 2 + col];
                let r = Rect::from_min_size(
                    pos2(rr.left() + col as f32 * (cw + gap), rr.top()),
                    vec2(cw, ch),
                );
                let selected = self.opts.preset == *p;
                let resp = ui.interact(r, Id::new(("trace-preset", p.name())), Sense::click());
                resp.widget_info(|| {
                    egui::WidgetInfo::selected(
                        egui::WidgetType::RadioButton,
                        true,
                        selected,
                        name.clone(),
                    )
                });
                paint_card(
                    ui,
                    r,
                    *p,
                    name,
                    hint,
                    selected,
                    resp.hovered(),
                    stacked.then_some(name_h),
                    theme,
                );
                if resp.clicked() && !selected {
                    self.set_preset(*p);
                }
            }
            if row == 0 {
                ui.add_space(gap);
            }
        }
    }

    /// Abschnitt „Aufräumen“ im Bereich „Erweitert“: die einzelnen Schritte mit Toleranzreglern.
    fn refine_ui(&mut self, ui: &mut Ui, o: &mut TraceOptions) {
        ui_kit::section(ui, &t("trace-refine-section"));
        ui.add_space(sp::XS);
        let on = o.refine.enabled;
        if !on {
            caption_text(ui, &t("trace-refine-off-hint"), 0.0, true);
            ui.add_space(sp::XS);
        }
        let rf = &mut o.refine;
        let px = fp_i18n::unit("px");
        let deg = fp_i18n::unit("deg");
        let check = |ui: &mut Ui, id: &str, label: &str, v: &mut bool| {
            if let Some(nv) = check_row(ui, &format!("trace-refine-{id}"), label, *v, on) {
                *v = nv;
            }
        };
        check(ui, "colors", &t("trace-refine-colors"), &mut rf.snap_colors);
        if let Some(v) = tolerance_row(
            ui,
            "colors",
            &t("trace-refine-colors-tol"),
            rf.color_tolerance,
            (8.0, 128.0),
            0,
            1.0,
            "",
            on && rf.snap_colors,
        ) {
            rf.color_tolerance = v;
        }
        check(ui, "lines", &t("trace-refine-lines"), &mut rf.straighten);
        if let Some(v) = tolerance_row(
            ui,
            "lines",
            &t("trace-refine-lines-tol"),
            rf.snap_angle,
            (0.0, 15.0),
            1,
            0.5,
            &deg,
            on && rf.straighten,
        ) {
            rf.snap_angle = v;
        }
        check(ui, "shapes", &t("trace-refine-shapes"), &mut rf.shapes);
        if let Some(v) = tolerance_row(
            ui,
            "shapes",
            &t("trace-refine-shapes-tol"),
            rf.shape_tolerance,
            (0.25, 5.0),
            2,
            0.25,
            &px,
            on && rf.shapes,
        ) {
            rf.shape_tolerance = v;
        }
        check(ui, "nodes", &t("trace-refine-nodes"), &mut rf.reduce_nodes);
        if let Some(v) = tolerance_row(
            ui,
            "nodes",
            &t("trace-refine-nodes-tol"),
            rf.fit_tolerance,
            (0.2, 4.0),
            1,
            0.1,
            &px,
            on,
        ) {
            rf.fit_tolerance = v;
        }
        check(ui, "gaps", &t("trace-refine-gaps"), &mut rf.close_gaps);
        check(
            ui,
            "symmetry",
            &t("trace-refine-symmetry"),
            &mut rf.symmetry,
        );
        caption_text(ui, &t("trace-refine-tol-hint"), 0.0, on);
        ui.add_space(sp::M);
        ui_kit::section(ui, &t("trace-dlg-tracing"));
        ui.add_space(sp::XS);
    }

    /// Abschnitt „Konturen“ im Bereich „Erweitert“ (VT-6): Abbiegeregel an diagonalen Pixeln.
    fn potrace_advanced_ui(&mut self, ui: &mut Ui, o: &mut TraceOptions) {
        ui_kit::section(ui, &t("trace-potrace-section"));
        ui.add_space(sp::XS);
        let theme = Theme::current(ui.ctx());
        let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 20.0), Sense::hover());
        if ui_kit::paint_text_fit(
            ui.painter(),
            r,
            &t("trace-potrace-turn"),
            theme.font(type_scale::BODY),
            theme.colors.text,
        ) {
            report_overflow(ui.ctx(), format!("trace turn {:.0}", r.width()));
        }
        let names: Vec<String> = TurnPolicy::ALL
            .iter()
            .map(|p| t(&format!("trace-turn-{}", p.name())))
            .collect();
        let sel = TurnPolicy::ALL
            .iter()
            .position(|p| *p == o.potrace.turn_policy)
            .unwrap_or(0);
        for row in 0..2 {
            let items: Vec<&str> = names[row * 3..row * 3 + 3]
                .iter()
                .map(|s| s.as_str())
                .collect();
            let w = ui.available_width();
            let (r, _) = ui.allocate_exact_size(vec2(w, 22.0), Sense::hover());
            // Auswahl in der anderen Zeile: hier nichts markiert
            let here = if sel / 3 == row { sel % 3 } else { usize::MAX };
            if let Some(i) = segmented(
                ui,
                Id::new(("trace-turn", row)),
                Rect::from_min_size(r.min + vec2(0.0, 1.0), vec2(w, 20.0)),
                &items,
                here,
                true,
            ) {
                o.potrace.turn_policy = TurnPolicy::ALL[row * 3 + i];
            }
        }
        caption_text(ui, &t("trace-potrace-turn-hint"), 0.0, true);
        ui.add_space(sp::M);
        ui_kit::section(ui, &t("trace-dlg-tracing"));
        ui.add_space(sp::XS);
    }

    fn advanced_ui(&mut self, ui: &mut Ui, o: &mut TraceOptions) {
        let pot = o.engine == Engine::Potrace;
        if pot {
            self.potrace_advanced_ui(ui, o);
        } else {
            self.refine_ui(ui, o);
        }
        let labels = [
            t("trace-opt-color-mode"),
            t("trace-opt-curves"),
            t("trace-opt-layering"),
            t("trace-opt-max-colors"),
            t("trace-opt-precision"),
            t("trace-opt-layer-diff"),
            t("trace-opt-length"),
            t("trace-opt-iterations"),
            t("trace-opt-splice"),
            t("trace-opt-decimals"),
            t("trace-opt-alpha"),
            t("trace-opt-max-pixels"),
        ];
        let refs: Vec<&str> = labels[3..].iter().map(|s| s.as_str()).collect();
        let lw = ui_kit::label_width(ui.ctx(), &refs, 60.0).min(ui.available_width() * 0.55);
        let bw = o.color_mode == ColorMode::BlackWhite;
        let seg = |ui: &mut Ui, id: &str, items: &[&str], sel: usize, en: bool| -> Option<usize> {
            let w = ui.available_width();
            let (r, _) = ui.allocate_exact_size(vec2(w, 22.0), Sense::hover());
            segmented(
                ui,
                Id::new(("trace-seg", id)),
                Rect::from_min_size(r.min + vec2(0.0, 1.0), vec2(w, 20.0)),
                items,
                sel,
                en,
            )
        };
        let caption = |ui: &mut Ui, text: &str, en: bool| {
            let th = Theme::current(ui.ctx());
            let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 20.0), Sense::hover());
            if ui_kit::paint_text_fit(
                ui.painter(),
                r,
                text,
                th.font(type_scale::BODY),
                if en {
                    th.colors.text
                } else {
                    th.colors.text_disabled
                },
            ) {
                report_overflow(ui.ctx(), format!("trace caption {:.0}", r.width()));
            }
        };
        let (m_color, m_bw) = (t("trace-opt-mode-color"), t("trace-opt-mode-bw"));
        caption(ui, &labels[0], true);
        if let Some(i) = seg(ui, "mode", &[&m_color, &m_bw], bw as usize, true) {
            o.color_mode = if i == 1 {
                ColorMode::BlackWhite
            } else {
                ColorMode::Color
            };
        }
        if !pot {
            ui.add_space(sp::XS);
            let (c_sp, c_po) = (t("trace-opt-curves-spline"), t("trace-opt-curves-polygon"));
            caption(ui, &labels[1], true);
            if let Some(i) = seg(
                ui,
                "curves",
                &[&c_sp, &c_po],
                (o.mode == CurveMode::Polygon) as usize,
                true,
            ) {
                o.mode = if i == 1 {
                    CurveMode::Polygon
                } else {
                    CurveMode::Spline
                };
            }
            ui.add_space(sp::XS);
            let (l_st, l_cu) = (t("trace-opt-stacked"), t("trace-opt-cutout"));
            caption(ui, &labels[2], !bw);
            if let Some(i) = seg(
                ui,
                "layer",
                &[&l_st, &l_cu],
                (o.hierarchical == Hierarchy::Cutout) as usize,
                !bw,
            ) {
                o.hierarchical = if i == 1 {
                    Hierarchy::Cutout
                } else {
                    Hierarchy::Stacked
                };
            }
        }
        ui.add_space(sp::S);
        let bit = t("trace-unit-bit");
        let deg = fp_i18n::unit("deg");
        let px = fp_i18n::unit("px");
        let mp = t("trace-unit-mp");
        let num = |ui: &mut Ui,
                   label: &str,
                   id: &str,
                   v: f64,
                   range: (f64, f64),
                   dec: usize,
                   unit: &str,
                   en: bool|
         -> Option<f64> {
            ui.add_enabled_ui(en, |ui| {
                ui_kit::kv_row(ui, label, lw, |ui| {
                    let (r, _) =
                        ui.allocate_exact_size(vec2(ui.available_width(), sp::ROW), Sense::hover());
                    let fr =
                        Rect::from_min_size(pos2(r.left(), r.center().y - 10.0), vec2(56.0, 20.0));
                    let out =
                        fields::number(ui, ("trace-adv", id), fr, None, Some(v), range, dec, en);
                    if !unit.is_empty() {
                        let c = Theme::current(ui.ctx());
                        ui_kit::paint_text_fit(
                            ui.painter(),
                            Rect::from_min_max(pos2(fr.right() + sp::XS, r.top()), r.max),
                            unit,
                            c.font(type_scale::BODY),
                            if en {
                                c.colors.text_weak
                            } else {
                                c.colors.text_disabled
                            },
                        );
                    }
                    out
                })
            })
            .inner
        };
        if let Some(v) = num(
            ui,
            &labels[3],
            "colors",
            o.colors as f64,
            (0.0, 256.0),
            0,
            "",
            !bw,
        ) {
            o.colors = v.round() as u32;
        }
        if !pot {
            if let Some(v) = num(
                ui,
                &labels[4],
                "precision",
                o.color_precision as f64,
                (1.0, 8.0),
                0,
                &bit,
                !bw,
            ) {
                o.color_precision = v.round() as u8;
            }
            if let Some(v) = num(
                ui,
                &labels[5],
                "layerdiff",
                o.layer_difference as f64,
                (0.0, 255.0),
                0,
                "",
                !bw,
            ) {
                o.layer_difference = v.round() as u8;
            }
            if let Some(v) = num(
                ui,
                &labels[6],
                "length",
                o.length_threshold,
                (3.5, 10.0),
                1,
                &px,
                true,
            ) {
                o.length_threshold = v;
            }
            if let Some(v) = num(
                ui,
                &labels[7],
                "iterations",
                o.max_iterations as f64,
                (1.0, 50.0),
                0,
                "",
                true,
            ) {
                o.max_iterations = v.round() as usize;
            }
            if let Some(v) = num(
                ui,
                &labels[8],
                "splice",
                o.splice_threshold,
                (0.0, 180.0),
                0,
                &deg,
                o.mode == CurveMode::Spline,
            ) {
                o.splice_threshold = v;
            }
        }
        if let Some(v) = num(
            ui,
            &labels[9],
            "decimals",
            o.path_precision as f64,
            (0.0, 8.0),
            0,
            "",
            true,
        ) {
            o.path_precision = v.round() as u32;
        }
        let w = ui.available_width();
        let (r, _) = ui.allocate_exact_size(vec2(w, sp::ROW), Sense::hover());
        if let Some(v) = fields::checkbox(
            ui,
            "trace-transparent",
            r,
            &t("trace-opt-transparent"),
            o.ignore_transparent,
            true,
        ) {
            o.ignore_transparent = v;
        }
        if let Some(v) = num(
            ui,
            &labels[10],
            "alpha",
            o.alpha_threshold as f64,
            (1.0, 255.0),
            0,
            "",
            o.ignore_transparent,
        ) {
            o.alpha_threshold = v.round() as u8;
        }
        let (r, _) = ui.allocate_exact_size(vec2(w, sp::ROW), Sense::hover());
        if let Some(v) = fields::checkbox(
            ui,
            "trace-downscale",
            r,
            &t("trace-opt-downscale"),
            o.downscale,
            true,
        ) {
            o.downscale = v;
        }
        if let Some(v) = num(
            ui,
            &labels[11],
            "maxpixels",
            o.max_pixels as f64 / 1e6,
            (0.25, fp_trace::HARD_PIXEL_LIMIT as f64 / 1e6),
            2,
            &mp,
            true,
        ) {
            o.max_pixels = (v * 1e6).round() as u64;
        }
        ui.add_space(sp::XS);
        let reset = t("trace-dlg-reset");
        let (r, _) = ui.allocate_exact_size(vec2(w, 20.0), Sense::hover());
        if ui_kit::more_link_at(ui, r, Id::new("trace-reset"), &reset, None).clicked() {
            // Verfahren bleibt; dessen Werte gehen auf die Vorgabe zurück
            *o = TraceOptions {
                engine: o.engine,
                ..TraceOptions::from_preset(o.preset)
            };
        }
    }

    // ------------------------------------------------------------------ Vorschau

    fn preview_ui(&mut self, ui: &mut Ui, area: Rect, theme: &Theme, now: f64) {
        let c = &theme.colors;
        let ctx = ui.ctx().clone();
        // Ansicht (Segmente) und Zustand rechts
        let labels: Vec<String> = ViewMode::ALL.iter().map(|m| t(m.key())).collect();
        let refs: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
        let seg_w = (refs
            .iter()
            .map(|s| super::text_w(&ctx, s))
            .fold(0.0, f32::max)
            + 20.0)
            * refs.len() as f32;
        let seg_w = seg_w.min(area.width()).ceil();
        let sel = ViewMode::ALL
            .iter()
            .position(|m| *m == self.view)
            .unwrap_or(0);
        if let Some(i) = segmented(
            ui,
            Id::new("trace-view"),
            Rect::from_min_size(area.min, vec2(seg_w, 22.0)),
            &refs,
            sel,
            true,
        ) {
            self.view = ViewMode::ALL[i];
        }
        let state_r = Rect::from_min_max(
            pos2(area.left() + seg_w + sp::M, area.top()),
            pos2(area.right(), area.top() + 22.0),
        );
        if state_r.width() > 40.0 {
            let (txt, col) = if let Some(j) = &self.job {
                let stage = match j.stage.load(Ordering::Relaxed) {
                    0 => t("trace-stage-prepare"),
                    1 => t("trace-stage-cluster"),
                    _ => t("trace-stage-paths"),
                };
                (stage, c.text_weak)
            } else if let Some(s) = &self.stats {
                (fp_i18n::tr!("trace-done", ms = s.ms as f64), c.text_weak)
            } else {
                (String::new(), c.text_weak)
            };
            let font = theme.font(type_scale::CAPTION);
            let shown = ui_kit::elide(&ctx, &txt, &font, state_r.width());
            ui.painter().text(
                pos2(state_r.right(), state_r.center().y),
                Align2::RIGHT_CENTER,
                shown,
                font,
                col,
            );
        }
        // Kennzahlen unten (mit Aufräumen zweizeilig), Hinweise darüber
        let stats_h = if self.stats.as_ref().is_some_and(|s| s.refine.is_some()) {
            2.0 * STATS_ROW_H
        } else {
            STATS_ROW_H
        };
        let stats_r = Rect::from_min_max(pos2(area.left(), area.bottom() - stats_h), area.max);
        let mut notices: Vec<(Tone, String)> = Vec::new();
        if self.bitmaps > 1 {
            notices.push((
                Tone::Info,
                fp_i18n::tr!("trace-multi", n = self.bitmaps as f64),
            ));
        }
        if let Some((w, h)) = self.stats.as_ref().and_then(|s| s.downscaled) {
            notices.push((
                Tone::Info,
                fp_i18n::tr!("trace-downscaled", width = w as f64, height = h as f64),
            ));
        }
        let mut bottom = stats_r.top() - sp::S;
        for (tone, text) in notices.iter().rev() {
            let h = ui_kit::notice_height(&ctx, None, text, area.width());
            let r = Rect::from_min_max(pos2(area.left(), bottom - h), pos2(area.right(), bottom));
            ui_kit::notice_at(ui.painter(), r, *tone, None, text, theme);
            bottom = r.top() - sp::S;
        }
        let canvas = Rect::from_min_max(
            pos2(area.left(), area.top() + 22.0 + sp::S),
            pos2(area.right(), bottom.max(area.top() + 22.0 + sp::S + 60.0)),
        );
        self.canvas_ui(ui, canvas, theme, now);
        self.stats_ui(ui, stats_r, theme);
    }

    fn image_rect(&self, canvas: Rect) -> (Rect, f32, f32) {
        let (w, h) = (
            self.source.width().max(1) as f32,
            self.source.height().max(1) as f32,
        );
        let fit = ((canvas.width() - 24.0) / w)
            .min((canvas.height() - 24.0) / h)
            .max(1e-4);
        let scale = self.zoom.unwrap_or(fit);
        let center = canvas.center()
            + if self.zoom.is_some() {
                self.pan
            } else {
                Vec2::ZERO
            };
        (
            Rect::from_center_size(center, vec2(w * scale, h * scale)),
            scale,
            fit,
        )
    }

    fn zoom_by(&mut self, canvas: Rect, factor: f32, around: Pos2) {
        let (img, cur, fit) = self.image_rect(canvas);
        let new = (cur * factor).clamp((fit * 0.5).min(1.0), 64.0);
        let rel = (around - img.center()) / cur;
        let center = around - rel * new;
        self.zoom = Some(new);
        self.pan = center - canvas.center();
    }

    fn canvas_ui(&mut self, ui: &mut Ui, canvas: Rect, theme: &Theme, _now: f64) {
        let c = &theme.colors;
        let p = ui.painter().with_clip_rect(canvas);
        let radius = CornerRadius::same(4);
        p.rect_filled(canvas, radius, c.canvas_bg);
        let resp = ui.interact(canvas, Id::new("trace-canvas"), Sense::click_and_drag());
        let (img, _scale, _fit) = self.image_rect(canvas);
        // Trenner
        let split_x = img.left() + img.width() * self.split;
        let handle_r = Rect::from_center_size(
            pos2(
                split_x,
                canvas.center().y.clamp(
                    img.top().max(canvas.top()) + 16.0,
                    img.bottom().min(canvas.bottom()) - 16.0,
                ),
            ),
            vec2(26.0, 26.0),
        );
        let line_r = Rect::from_min_max(
            pos2(split_x - 6.0, img.top().max(canvas.top())),
            pos2(split_x + 6.0, img.bottom().min(canvas.bottom())),
        );
        let split_on = self.view == ViewMode::Split && self.vector_tex.is_some();
        let sresp = split_on.then(|| {
            let r = ui.interact(
                line_r.union(handle_r),
                Id::new("trace-split"),
                Sense::drag(),
            );
            let lbl = t("trace-split-handle");
            r.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Slider, true, lbl.clone())
            });
            r
        });
        if let Some(s) = &sresp {
            if s.hovered() || s.dragged() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
            }
            if s.dragged()
                && let Some(pp) = s.interact_pointer_pos()
            {
                self.split = ((pp.x - img.left()) / img.width().max(1.0)).clamp(0.0, 1.0);
            }
        }
        // Zoom und Verschieben
        if resp.hovered() {
            // rohe Ereignisse dieses Bildes: eine Rastung (Zeile) = eine Stufe, Punkte (Touchpad,
            // hochauflösende Räder) werden gesammelt; Zwei-Finger-Zoom stufenlos
            let (wheel, pinch) = ui.input(|i| {
                let mut w = Vec::new();
                let mut z = 1.0f32;
                for e in &i.events {
                    match e {
                        egui::Event::MouseWheel { unit, delta, .. } => w.push((*unit, delta.y)),
                        egui::Event::Zoom(f) => z *= *f,
                        _ => {}
                    }
                }
                (w, z)
            });
            if let Some(hp) = ui.input(|i| i.pointer.hover_pos()) {
                if (pinch - 1.0).abs() > 1e-4 {
                    self.zoom_by(canvas, pinch, hp);
                }
                for (unit, dy) in wheel {
                    let notches = match unit {
                        egui::MouseWheelUnit::Point => dy / 100.0,
                        _ if dy == 0.0 => 0.0,
                        _ => dy.signum(),
                    };
                    if notches.signum() != self.scroll_acc.signum() {
                        self.scroll_acc = 0.0;
                    }
                    self.scroll_acc += notches;
                    while self.scroll_acc.abs() >= 1.0 - 1e-4 {
                        let up = self.scroll_acc > 0.0;
                        self.zoom_by(canvas, if up { 1.25 } else { 0.8 }, hp);
                        self.scroll_acc -= if up { 1.0 } else { -1.0 };
                    }
                }
            }
        }
        if resp.dragged() && !sresp.as_ref().is_some_and(|s| s.dragged()) {
            let (img_now, scale, _) = self.image_rect(canvas);
            if self.zoom.is_none() {
                self.zoom = Some(scale);
                self.pan = img_now.center() - canvas.center();
            }
            self.pan += resp.drag_delta();
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        }
        if resp.double_clicked() {
            self.zoom = None;
            self.pan = Vec2::ZERO;
        }
        let (img, scale, _) = self.image_rect(canvas);
        let split_x = img.left() + img.width() * self.split;
        // Schachbrett unter dem Bild, Bild(er)
        let vis = img.intersect(canvas);
        if vis.is_positive() {
            crate::widgets::checker(&p, vis, 8.0, theme);
        }
        let full_uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
        let draw_original = |p: &egui::Painter, r: Rect, uv: Rect, tint: Color32| {
            if let Some(tex) = &self.original_tex {
                p.image(tex.id(), r, uv, tint);
            }
        };
        match (self.view, &self.vector_tex) {
            (ViewMode::Split, Some(vt)) => {
                let lr = Rect::from_min_max(img.min, pos2(split_x, img.bottom()));
                let rr = Rect::from_min_max(pos2(split_x, img.top()), img.max);
                draw_original(
                    &p,
                    lr,
                    Rect::from_min_max(pos2(0.0, 0.0), pos2(self.split, 1.0)),
                    Color32::WHITE,
                );
                p.image(
                    vt.id(),
                    rr,
                    Rect::from_min_max(pos2(self.split, 0.0), pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            (ViewMode::Vector, Some(vt)) => {
                p.image(vt.id(), img, full_uv, Color32::WHITE);
            }
            (ViewMode::Overlay, Some(_)) => {
                draw_original(&p, img, full_uv, Color32::from_white_alpha(255));
                // Original etwas abdunkeln, damit die Konturen hervortreten
                p.rect_filled(
                    img,
                    0.0,
                    Color32::from_black_alpha(if crate::ui_kit::is_light(theme) {
                        40
                    } else {
                        90
                    }),
                );
                // Konturen als einzelne Segmente (ohne Gehrung: spitze Kehren erzeugen sonst Zacken)
                p.add(Shape::mesh(segments_mesh(
                    &self.outlines,
                    img.min,
                    scale,
                    canvas,
                    2.6,
                    Color32::from_black_alpha(110),
                )));
                p.add(Shape::mesh(segments_mesh(
                    &self.outlines,
                    img.min,
                    scale,
                    canvas,
                    1.1,
                    c.focus,
                )));
            }
            _ => {
                // Original bzw. noch kein Ergebnis
                let dim = self.vector_tex.is_none() && self.view != ViewMode::Original;
                draw_original(
                    &p,
                    img,
                    full_uv,
                    if dim {
                        Color32::from_white_alpha(150)
                    } else {
                        Color32::WHITE
                    },
                );
            }
        }
        // Rahmen um das Bild
        p.rect_stroke(
            img,
            0.0,
            Stroke::new(1.0, with_alpha(c.line_dark, 120)),
            StrokeKind::Outside,
        );
        // Trenner und Beschriftungen
        if split_on {
            let top = img.top().max(canvas.top());
            let bot = img.bottom().min(canvas.bottom());
            p.vline(
                split_x,
                top..=bot,
                Stroke::new(3.0, Color32::from_black_alpha(70)),
            );
            p.vline(split_x, top..=bot, Stroke::new(1.5, Color32::WHITE));
            let hc = pos2(
                split_x,
                ((top + bot) / 2.0).clamp(canvas.top() + 16.0, canvas.bottom() - 16.0),
            );
            let hot = sresp.as_ref().is_some_and(|s| s.hovered() || s.dragged());
            if theme.glow {
                p.circle_filled(hc, 15.0, with_alpha(c.focus, 70));
            }
            p.circle_filled(hc, 12.0, if hot { c.focus } else { Color32::WHITE });
            p.circle_stroke(hc, 12.0, Stroke::new(1.0, Color32::from_black_alpha(90)));
            let ac = if hot {
                Color32::WHITE
            } else {
                Color32::from_gray(60)
            };
            p.add(Shape::convex_polygon(
                vec![
                    hc + vec2(-8.0, 0.0),
                    hc + vec2(-3.5, -4.5),
                    hc + vec2(-3.5, 4.5),
                ],
                ac,
                Stroke::NONE,
            ));
            p.add(Shape::convex_polygon(
                vec![
                    hc + vec2(8.0, 0.0),
                    hc + vec2(3.5, 4.5),
                    hc + vec2(3.5, -4.5),
                ],
                ac,
                Stroke::NONE,
            ));
            let font = theme.font(type_scale::CAPTION);
            let (lo, lv) = (t("trace-view-original"), t("trace-view-vector"));
            let pill_y = img.top().max(canvas.top()) + 8.0;
            pill(
                &p,
                pos2(img.left().max(canvas.left()) + 8.0, pill_y),
                Align2::LEFT_TOP,
                &lo,
                font.clone(),
            );
            pill(
                &p,
                pos2(img.right().min(canvas.right()) - 8.0, pill_y),
                Align2::RIGHT_TOP,
                &lv,
                font,
            );
        }
        // Fehler / leeres Ergebnis / erste Berechnung
        if let Some(e) = &self.error {
            let w = (canvas.width() - 32.0).clamp(80.0, 420.0);
            let h = ui_kit::notice_height(ui.ctx(), None, e, w);
            let r = Rect::from_center_size(canvas.center(), vec2(w, h));
            ui_kit::notice_at(&p, r, Tone::Error, None, e, theme);
        } else if self.stats.as_ref().is_some_and(|s| s.paths == 0)
            && self.view != ViewMode::Original
        {
            let faint = self.stats.as_ref().and_then(|s| s.faint_alpha);
            let hh = match faint {
                Some(a) => fp_i18n::tr!(
                    "trace-empty-hint-faint",
                    alpha = a as u32,
                    threshold = self.opts.validated().alpha_threshold as u32
                ),
                None => t("trace-empty-hint"),
            };
            let tt = t("trace-empty-title");
            let w = (canvas.width() - 32.0).clamp(80.0, 360.0);
            let h = ui_kit::empty_state_height(ui.ctx(), &tt, Some(&hh), w);
            let r = Rect::from_center_size(canvas.center(), vec2(w, h));
            p.rect_filled(
                r.expand(6.0),
                CornerRadius::same(6),
                with_alpha(ui_kit::card_bg(theme), 235),
            );
            ui_kit::empty_state_at(&p, r, KitIcon::EmptyTray, &tt, Some(&hh), theme);
        } else if self.vector_tex.is_none() && self.view != ViewMode::Original {
            let font = theme.font(type_scale::BODY);
            pill(
                &p,
                canvas.center(),
                Align2::CENTER_CENTER,
                &t("trace-no-preview"),
                font,
            );
        }
        // Fortschritt: feiner Balken oben im Vorschaubereich
        if let Some(j) = &self.job {
            let f = f32::from_bits(j.progress.load(Ordering::Relaxed)).clamp(0.0, 1.0);
            let bar = Rect::from_min_size(canvas.min, vec2(canvas.width(), 3.0));
            p.rect_filled(bar, 0.0, with_alpha(c.focus, 50));
            p.rect_filled(
                Rect::from_min_size(bar.min, vec2(bar.width() * f.max(0.04), 3.0)),
                0.0,
                c.focus,
            );
            let pct = (f * 100.0).round() as f64;
            pill(
                &p,
                pos2(canvas.center().x, canvas.top() + 10.0),
                Align2::CENTER_TOP,
                &fp_i18n::tr!("trace-busy", pct = pct),
                theme.font(type_scale::CAPTION),
            );
        }
        // Rahmen
        ui.painter().rect_stroke(
            canvas,
            radius,
            Stroke::new(
                1.0,
                if theme.glow {
                    with_alpha(c.focus, 110)
                } else {
                    c.list_border
                },
            ),
            StrokeKind::Inside,
        );
        // Zoomleiste unten rechts
        self.zoom_bar(ui, canvas, theme, scale);
    }

    fn zoom_bar(&mut self, ui: &mut Ui, canvas: Rect, theme: &Theme, scale: f32) {
        let c = &theme.colors;
        let pct = format!(
            "{}{}",
            fp_i18n::fmt_number((scale as f64 * 100.0).round(), 0, 0),
            fp_i18n::unit("percent")
        );
        let font = theme.font(type_scale::CAPTION);
        let lw =
            ui_kit::text_w(ui.ctx(), &pct, &font).max(ui_kit::text_w(ui.ctx(), "6400%", &font))
                + 8.0;
        let b = 22.0;
        let w = b * 3.0 + lw + 8.0;
        if canvas.width() < w + 16.0 || canvas.height() < 60.0 {
            return;
        }
        let r = Rect::from_min_size(
            pos2(canvas.right() - 8.0 - w, canvas.bottom() - 8.0 - (b + 4.0)),
            vec2(w, b + 4.0),
        );
        let p = ui.painter();
        p.rect_filled(
            r,
            CornerRadius::same(13),
            with_alpha(ui_kit::card_bg(theme), 235),
        );
        p.rect_stroke(
            r,
            CornerRadius::same(13),
            Stroke::new(
                1.0,
                if theme.glow {
                    with_alpha(c.focus, 140)
                } else {
                    c.list_border
                },
            ),
            StrokeKind::Inside,
        );
        let mut x = r.left() + 4.0;
        let cy = r.center().y;
        let btn = |ui: &mut Ui, x: f32, kind: u8, tip: String| -> bool {
            let br = Rect::from_center_size(pos2(x + b / 2.0, cy), vec2(b, b));
            let resp = ui.interact(br, Id::new(("trace-zoom", kind)), Sense::click());
            resp.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, true, tip.clone())
            });
            let hot = resp.hovered();
            if hot {
                ui.painter().circle_filled(
                    br.center(),
                    b / 2.0 - 1.0,
                    if theme.glow {
                        with_alpha(c.focus, 90)
                    } else {
                        c.list_hover_bg
                    },
                );
            }
            let col = if hot && theme.glow {
                Color32::WHITE
            } else {
                c.text
            };
            let s = Stroke::new(1.5, col);
            let cc = br.center();
            match kind {
                0 => {
                    ui.painter().hline((cc.x - 5.0)..=(cc.x + 5.0), cc.y, s);
                }
                1 => {
                    ui.painter().hline((cc.x - 5.0)..=(cc.x + 5.0), cc.y, s);
                    ui.painter().vline(cc.x, (cc.y - 5.0)..=(cc.y + 5.0), s);
                }
                _ => {
                    // Einpassen: vier Ecken
                    let k = 5.0;
                    let q = 2.5;
                    for (sx, sy) in [(-1.0f32, -1.0f32), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                        let corner = cc + vec2(sx * k, sy * k);
                        ui.painter()
                            .line_segment([corner, corner - vec2(sx * q, 0.0)], s);
                        ui.painter()
                            .line_segment([corner, corner - vec2(0.0, sy * q)], s);
                    }
                }
            }
            resp.on_hover_text(tip).clicked()
        };
        let zoom_out = btn(ui, x, 0, t("trace-zoom-out"));
        x += b;
        let lr = Rect::from_min_size(pos2(x, r.top()), vec2(lw, r.height()));
        ui.painter()
            .text(lr.center(), Align2::CENTER_CENTER, &pct, font, c.text);
        x += lw;
        let zoom_in = btn(ui, x, 1, t("trace-zoom-in"));
        x += b + 4.0;
        let fit = btn(ui, x, 2, t("trace-zoom-fit"));
        if zoom_out {
            self.zoom_by(canvas, 0.8, canvas.center());
        }
        if zoom_in {
            self.zoom_by(canvas, 1.25, canvas.center());
        }
        if fit {
            self.zoom = None;
            self.pan = Vec2::ZERO;
        }
    }

    fn stats_ui(&self, ui: &mut Ui, r: Rect, theme: &Theme) {
        let c = &theme.colors;
        let p = ui.painter();
        p.rect_filled(r, CornerRadius::same(4), ui_kit::card_bg(theme));
        p.rect_stroke(
            r,
            CornerRadius::same(4),
            Stroke::new(
                1.0,
                if theme.glow {
                    c.line_dark
                } else {
                    c.list_border
                },
            ),
            StrokeKind::Inside,
        );
        let line = if theme.glow {
            c.line_dark
        } else {
            c.list_border
        };
        let s = self.stats.as_ref();
        let r = if let Some(rep) = s.and_then(|s| s.refine.as_ref()) {
            // zweite Zeile: Aufräumen (Grundformen, Geraden, Knoten vorher → nachher); die
            // Trennlinien stehen unter denen der ersten Zeile
            let top = Rect::from_min_size(r.min, vec2(r.width(), STATS_ROW_H));
            let low = Rect::from_min_max(pos2(r.left(), top.bottom()), r.max);
            p.hline(
                (low.left() + 8.0)..=(low.right() - 8.0),
                low.top(),
                Stroke::new(1.0, line),
            );
            let n = |v: usize| fp_i18n::fmt_number(v as f64, 0, 0);
            let cells = [
                (
                    t("trace-stat-shapes"),
                    n(rep.shape_count()),
                    fp_i18n::tr!(
                        "trace-stat-shapes-tip",
                        circles = rep.circles as f64,
                        ellipses = rep.ellipses as f64,
                        rectangles = rep.rectangles as f64,
                        rounded = rep.rounded_rectangles as f64
                    ),
                ),
                (
                    t("trace-stat-lines"),
                    n(rep.snapped_lines),
                    fp_i18n::tr!(
                        "trace-stat-lines-tip",
                        snapped = rep.snapped_lines as f64,
                        lines = rep.lines as f64
                    ),
                ),
                (
                    t("trace-stat-nodes"),
                    format!("{} → {}", n(rep.nodes_before), n(rep.nodes_after)),
                    t("trace-stat-nodes-tip"),
                ),
            ];
            let weights = [1.0f32, 1.0, 2.45];
            self.stat_cells(
                ui,
                low,
                theme,
                &weights,
                cells.iter().map(|(l, v, _)| (l.as_str(), v.as_str())),
                None,
            );
            for (i, (_, _, tip)) in cells.iter().enumerate() {
                let x0 = low.left()
                    + low.width() * weights[..i].iter().sum::<f32>() / weights.iter().sum::<f32>();
                let cr = Rect::from_min_size(
                    pos2(x0, low.top()),
                    vec2(
                        low.width() * weights[i] / weights.iter().sum::<f32>(),
                        low.height(),
                    ),
                );
                let _ = ui
                    .interact(cr, Id::new(("trace-refine-tip", i)), Sense::hover())
                    .on_hover_text(tip.as_str());
            }
            top
        } else {
            r
        };
        let dash = "–".to_string();
        let items = [
            (
                t("trace-stat-paths"),
                s.map(|s| fp_i18n::fmt_number(s.paths as f64, 0, 0))
                    .unwrap_or_else(|| dash.clone()),
            ),
            (
                t("trace-stat-points"),
                s.map(|s| fp_i18n::fmt_number(s.points as f64, 0, 0))
                    .unwrap_or_else(|| dash.clone()),
            ),
            (
                t("trace-stat-colors"),
                s.map(|s| fp_i18n::fmt_number(s.colors as f64, 0, 0))
                    .unwrap_or_else(|| dash.clone()),
            ),
            (
                t("trace-stat-match"),
                s.and_then(|s| s.match_shown())
                    .map(|m| {
                        format!(
                            "{}{}",
                            fp_i18n::fmt_number(m as f64, 1, 1),
                            fp_i18n::unit("percent")
                        )
                    })
                    .unwrap_or_else(|| dash.clone()),
            ),
        ];
        // „Übereinstimmung“ bekommt mehr Platz (längste Beschriftung)
        let weights = [1.0f32, 1.0, 1.0, 1.45];
        let dot = s.and_then(|s| s.match_shown()).map(|m| {
            if m >= 97.0 {
                c.icon_green
            } else if m >= 85.0 {
                c.icon_yellow
            } else {
                c.icon_red
            }
        });
        self.stat_cells(
            ui,
            r,
            theme,
            &weights,
            items.iter().map(|(l, v)| (l.as_str(), v.as_str())),
            dot.map(|d| (3, d)),
        );
        let total: f32 = weights.iter().sum();
        let x0 = r.left() + r.width() * weights[..3].iter().sum::<f32>() / total;
        let cr = Rect::from_min_max(pos2(x0, r.top()), r.max);
        let _ = ui
            .interact(cr, Id::new("trace-match-tip"), Sense::hover())
            .on_hover_text(t("trace-stat-match-tip"));
    }

    /// Eine Zeile Kennzahlen: Wert fett oben, Beschriftung gedämpft darunter, Trennlinien;
    /// `dot` = Ampelpunkt vor dem Wert der Zelle `dot.0`.
    fn stat_cells<'a>(
        &self,
        ui: &Ui,
        r: Rect,
        theme: &Theme,
        weights: &[f32],
        items: impl Iterator<Item = (&'a str, &'a str)>,
        dot: Option<(usize, Color32)>,
    ) {
        let c = &theme.colors;
        let p = ui.painter();
        let total: f32 = weights.iter().sum();
        let vfont = theme.bold(13.0);
        let lfont = theme.font(type_scale::CAPTION);
        for (i, (label, value)) in items.enumerate() {
            let x0 = r.left() + r.width() * weights[..i].iter().sum::<f32>() / total;
            let cr = Rect::from_min_size(
                pos2(x0, r.top()),
                vec2(r.width() * weights[i] / total, r.height()),
            );
            if i > 0 {
                p.vline(
                    cr.left(),
                    (cr.top() + 7.0)..=(cr.bottom() - 7.0),
                    Stroke::new(
                        1.0,
                        if theme.glow {
                            c.line_dark
                        } else {
                            c.list_border
                        },
                    ),
                );
            }
            let inner = cr.shrink2(vec2(6.0, 0.0));
            if let Some((k, col)) = dot
                && k == i
            {
                let vw = ui_kit::text_w(ui.ctx(), value, &vfont);
                p.circle_filled(
                    pos2(inner.center().x - vw / 2.0 - 7.0, inner.top() + 12.0),
                    3.5,
                    col,
                );
            }
            let v = ui_kit::elide(ui.ctx(), value, &vfont, inner.width());
            if v != value {
                report_overflow(ui.ctx(), format!("trace stat value {:.0}", inner.width()));
            }
            p.text(
                pos2(inner.center().x, inner.top() + 12.0),
                Align2::CENTER_CENTER,
                v,
                vfont.clone(),
                c.text,
            );
            let l = ui_kit::elide(ui.ctx(), label, &lfont, inner.width());
            if l != label {
                report_overflow(ui.ctx(), format!("trace stat {:.0}", inner.width()));
            }
            p.text(
                pos2(inner.center().x, inner.bottom() - 10.0),
                Align2::CENTER_CENTER,
                l,
                lfont.clone(),
                c.text_weak,
            );
        }
    }
}

// ---------------------------------------------------------------------- Bausteine

fn preset_key(p: Preset) -> &'static str {
    match p {
        Preset::Logo => "trace-preset-logo",
        Preset::BlackWhite => "trace-preset-bw",
        Preset::Illustration => "trace-preset-illustration",
        Preset::Photo => "trace-preset-photo",
    }
}

/// Dreiecksnetz aus allen Kontursegmenten (je Segment ein Rechteck der Breite `w`), auf `clip` beschränkt.
fn segments_mesh(
    polys: &[Vec<Pos2>],
    origin: Pos2,
    scale: f32,
    clip: Rect,
    w: f32,
    color: Color32,
) -> egui::epaint::Mesh {
    let mut mesh = egui::epaint::Mesh::default();
    let h = w / 2.0;
    let clip = clip.expand(w);
    for poly in polys {
        let n = poly.len();
        for i in 0..n {
            let a = origin + poly[i].to_vec2() * scale;
            let b = origin + poly[(i + 1) % n].to_vec2() * scale;
            let d = b - a;
            let len = d.length();
            if len < 0.05
                || (!clip.contains(a)
                    && !clip.contains(b)
                    && !Rect::from_two_pos(a, b).intersects(clip))
            {
                continue;
            }
            let t = d / len * (h * 0.5);
            let nrm = vec2(-d.y, d.x) / len * h;
            let base = mesh.vertices.len() as u32;
            for q in [a - t + nrm, b + t + nrm, b + t - nrm, a - t - nrm] {
                mesh.colored_vertex(q, color);
            }
            mesh.add_triangle(base, base + 1, base + 2);
            mesh.add_triangle(base, base + 2, base + 3);
        }
    }
    mesh
}

/// Halbtransparente Beschriftung („Pille“) über dem Bild.
fn pill(p: &egui::Painter, at: Pos2, align: Align2, text: &str, font: FontId) {
    let g = p.layout_no_wrap(text.to_string(), font, Color32::WHITE);
    let size = g.size() + vec2(16.0, 6.0);
    let r = align.anchor_size(at, size);
    p.rect_filled(
        r,
        CornerRadius::same((size.y / 2.0).round() as u8),
        Color32::from_black_alpha(150),
    );
    p.galley(r.min + vec2(8.0, 3.0), g, Color32::WHITE);
}

/// Segmentierte Auswahl (gleich breite Segmente, ausgewähltes getönt mit Akzentrahmen).
fn segmented(
    ui: &mut Ui,
    id: Id,
    rect: Rect,
    items: &[&str],
    selected: usize,
    enabled: bool,
) -> Option<usize> {
    segmented_tips(ui, id, rect, items, &[], selected, enabled)
}

/// Wie [`segmented`], mit Tooltip je Segment (`tips[i]`, fehlt = keiner).
fn segmented_tips(
    ui: &mut Ui,
    id: Id,
    rect: Rect,
    items: &[&str],
    tips: &[&str],
    selected: usize,
    enabled: bool,
) -> Option<usize> {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let n = items.len().max(1) as f32;
    let sw = rect.width() / n;
    let radius = CornerRadius::same(4);
    ui.painter()
        .rect_filled(rect, radius, if enabled { c.field_bg } else { c.frame_bg });
    let mut out = None;
    let font = theme.font(type_scale::BODY);
    for (i, label) in items.iter().enumerate() {
        let r = Rect::from_min_size(
            pos2(rect.left() + i as f32 * sw, rect.top()),
            vec2(sw, rect.height()),
        );
        let resp = ui.interact(
            r,
            id.with(i),
            if enabled {
                Sense::click()
            } else {
                Sense::hover()
            },
        );
        let sel = i == selected;
        resp.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::RadioButton,
                enabled,
                sel,
                label.to_string(),
            )
        });
        let cr = CornerRadius {
            nw: if i == 0 { 4 } else { 0 },
            sw: if i == 0 { 4 } else { 0 },
            ne: if i + 1 == items.len() { 4 } else { 0 },
            se: if i + 1 == items.len() { 4 } else { 0 },
        };
        if sel {
            ui.painter()
                .rect_filled(r, cr, ui_kit::tint(&theme, c.focus));
        } else if enabled && resp.hovered() {
            ui.painter().rect_filled(r, cr, c.list_hover_bg);
        }
        if i > 0 && !sel && selected.checked_add(1) != Some(i) {
            ui.painter().vline(
                r.left(),
                (r.top() + 4.0)..=(r.bottom() - 4.0),
                Stroke::new(1.0, c.field_border),
            );
        }
        let col = if !enabled {
            c.text_disabled
        } else if sel && theme.glow {
            Color32::WHITE
        } else {
            c.text
        };
        let shown = ui_kit::elide(ui.ctx(), label, &font, sw - 8.0);
        if shown != *label {
            report_overflow(ui.ctx(), format!("trace segment {:.0}", sw));
        }
        ui.painter().with_clip_rect(r).text(
            r.center(),
            Align2::CENTER_CENTER,
            shown,
            font.clone(),
            col,
        );
        if enabled && resp.clicked() && !sel {
            out = Some(i);
        }
        if let Some(tip) = tips.get(i) {
            let _ = resp.on_hover_text(*tip);
        }
    }
    ui.painter().rect_stroke(
        rect,
        radius,
        Stroke::new(1.0, c.field_border),
        StrokeKind::Inside,
    );
    if let Some(r) = (selected < items.len()).then(|| {
        Rect::from_min_size(
            pos2(rect.left() + selected as f32 * sw, rect.top()),
            vec2(sw, rect.height()),
        )
    }) {
        let cr = CornerRadius {
            nw: if selected == 0 { 4 } else { 0 },
            sw: if selected == 0 { 4 } else { 0 },
            ne: if selected + 1 == items.len() { 4 } else { 0 },
            se: if selected + 1 == items.len() { 4 } else { 0 },
        };
        ui.painter().rect_stroke(
            r,
            cr,
            Stroke::new(1.0, if enabled { c.focus } else { c.text_disabled }),
            StrokeKind::Inside,
        );
    }
    out
}

/// Waagerechter Schieber (Spur mit Akzentfüllung, runder Knopf). Pfeiltasten bei Fokus.
#[allow(clippy::too_many_arguments)]
fn slider(
    ui: &mut Ui,
    id: Id,
    rect: Rect,
    value: f64,
    range: (f64, f64),
    step: f64,
    enabled: bool,
    label: &str,
) -> Option<f64> {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let resp = ui.interact(
        rect,
        id,
        if enabled {
            Sense::click_and_drag()
        } else {
            Sense::hover()
        },
    );
    let span = (range.1 - range.0).max(1e-9);
    let track = rect.shrink2(vec2(8.0, 0.0));
    let frac = ((value - range.0) / span).clamp(0.0, 1.0) as f32;
    let snap = |v: f64| ((v / step).round() * step).clamp(range.0, range.1);
    let mut out = None;
    if enabled
        && (resp.dragged() || resp.clicked() || resp.drag_started())
        && let Some(pp) = resp.interact_pointer_pos()
    {
        let f = ((pp.x - track.left()) / track.width().max(1.0)).clamp(0.0, 1.0) as f64;
        out = Some(snap(range.0 + f * span));
    }
    if enabled && resp.has_focus() {
        let (l, r) = ui.input(|i| {
            (
                i.key_pressed(egui::Key::ArrowLeft),
                i.key_pressed(egui::Key::ArrowRight),
            )
        });
        if l {
            out = Some(snap(value - step));
        }
        if r {
            out = Some(snap(value + step));
        }
    }
    resp.widget_info(|| egui::WidgetInfo::slider(enabled, value, label));
    let p = ui.painter();
    let cy = rect.center().y;
    let tr = Rect::from_min_max(pos2(track.left(), cy - 2.0), pos2(track.right(), cy + 2.0));
    let base = if theme.glow {
        c.line_dark
    } else {
        mix(c.field_border, c.frame_bg, 0.35)
    };
    p.rect_filled(tr, CornerRadius::same(2), base);
    let kx = track.left() + track.width() * frac;
    let fill = if enabled { c.focus } else { c.text_disabled };
    p.rect_filled(
        Rect::from_min_max(tr.min, pos2(kx, tr.bottom())),
        CornerRadius::same(2),
        fill,
    );
    let hot = enabled && (resp.hovered() || resp.dragged() || resp.has_focus());
    let kc = pos2(kx, cy);
    if theme.glow && hot {
        p.circle_filled(kc, 10.0, with_alpha(c.focus, 70));
    }
    p.circle_filled(kc + vec2(0.0, 1.0), 7.0, Color32::from_black_alpha(40));
    p.circle_filled(kc, 7.0, if enabled { c.field_bg } else { c.frame_bg });
    p.circle_stroke(kc, 7.0, Stroke::new(if hot { 2.0 } else { 1.5 }, fill));
    out.filter(|v| (*v - value).abs() > 1e-9)
}

/// Zeile „Beschriftung … [Wert] Einheit“, darunter der Schieber und ggf. die Endbeschriftungen.
#[allow(clippy::too_many_arguments)]
fn slider_row(
    ui: &mut Ui,
    key: &str,
    label: &str,
    value: f64,
    range: (f64, f64),
    decimals: usize,
    step: f64,
    unit: &str,
    ends: Option<(&str, &str)>,
    enabled: bool,
) -> Option<f64> {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let w = ui.available_width();
    let font = theme.font(type_scale::BODY);
    let (r, _) = ui.allocate_exact_size(vec2(w, 22.0), Sense::hover());
    let unit_w = if unit.is_empty() {
        0.0
    } else {
        ui_kit::text_w(ui.ctx(), unit, &font).max(10.0) + sp::XS
    };
    let field_w = 46.0;
    let fr = Rect::from_min_size(
        pos2(r.right() - unit_w - field_w, r.top() + 1.0),
        vec2(field_w, 20.0),
    );
    let lr = Rect::from_min_max(r.min, pos2(fr.left() - sp::S, r.bottom()));
    if ui_kit::paint_text_fit(
        ui.painter(),
        lr,
        label,
        font.clone(),
        if enabled { c.text } else { c.text_disabled },
    ) {
        report_overflow(ui.ctx(), format!("trace label {:.0}", lr.width()));
    }
    // der Schieber reicht bis 64 Farben; das Feld nimmt größere Werte (Erweitert) an
    let field_range = if key == "colors" { (2.0, 256.0) } else { range };
    let mut out = fields::number(
        ui,
        ("trace-num", key),
        fr,
        None,
        Some(value),
        field_range,
        decimals,
        enabled,
    );
    if !unit.is_empty() {
        ui.painter().text(
            pos2(fr.right() + sp::XS, fr.center().y),
            Align2::LEFT_CENTER,
            unit,
            font.clone(),
            if enabled {
                c.text_weak
            } else {
                c.text_disabled
            },
        );
    }
    let (sr, _) = ui.allocate_exact_size(vec2(w, 20.0), Sense::hover());
    if let Some(v) = slider(
        ui,
        Id::new(("trace-slider", key)),
        sr,
        value.clamp(range.0, range.1),
        range,
        step,
        enabled,
        label,
    ) {
        out = Some(v);
    }
    if let Some((a, b)) = ends {
        let (er, _) = ui.allocate_exact_size(vec2(w, 14.0), Sense::hover());
        let cf = theme.font(type_scale::CAPTION);
        let half = er.width() / 2.0 - sp::XS;
        let col = if enabled {
            c.text_weak
        } else {
            c.text_disabled
        };
        let a2 = ui_kit::elide(ui.ctx(), a, &cf, half);
        let b2 = ui_kit::elide(ui.ctx(), b, &cf, half);
        if a2 != a || b2 != b {
            report_overflow(ui.ctx(), format!("trace ends {:.0}", half));
        }
        ui.painter().text(
            pos2(er.left() + 2.0, er.center().y),
            Align2::LEFT_CENTER,
            a2,
            cf.clone(),
            col,
        );
        ui.painter().text(
            pos2(er.right() - 2.0, er.center().y),
            Align2::RIGHT_CENTER,
            b2,
            cf,
            col,
        );
    }
    ui.add_space(sp::S);
    out
}

/// Einzug der Erklärsätze unter einem Kontrollkästchen (Kästchen 13 px + 5 px Abstand).
const CHECK_INDENT: f32 = 18.0;

/// Zeile mit Kontrollkästchen (wie `fields::checkbox`), zusätzlich mit Rolle und Beschriftung für
/// Barrierefreiheit, Tests und `ui_find`.
fn check_row(ui: &mut Ui, id: &str, label: &str, checked: bool, enabled: bool) -> Option<bool> {
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), sp::ROW), Sense::hover());
    let out = fields::checkbox(ui, id, r, label, checked, enabled);
    // dieselbe Kennung und dasselbe Rechteck: dieselbe Schaltfläche, nur mit Beschreibung
    let resp = ui.interact(
        r,
        Id::new(id),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let now = out.unwrap_or(checked);
    resp.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, enabled, now, label)
    });
    out
}

/// Umbrochener Erklärsatz (Stufe CAPTION, gedämpft), um `indent` eingerückt.
fn caption_text(ui: &mut Ui, text: &str, indent: f32, enabled: bool) {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let w = ui.available_width();
    let col = if enabled {
        c.text_weak
    } else {
        c.text_disabled
    };
    let g = ui.ctx().fonts_mut(|f| {
        f.layout(
            text.to_string(),
            theme.font(type_scale::CAPTION),
            col,
            (w - indent).max(40.0),
        )
    });
    let (r, _) = ui.allocate_exact_size(vec2(w, g.size().y), Sense::hover());
    ui.painter()
        .galley(pos2(r.left() + indent, r.top()), g, col);
}

/// Eingerückte Toleranzzeile unter einem Schalter: Schieber, Zahlenfeld, Einheit (eine Zeile).
#[allow(clippy::too_many_arguments)]
fn tolerance_row(
    ui: &mut Ui,
    key: &str,
    label: &str,
    value: f64,
    range: (f64, f64),
    decimals: usize,
    step: f64,
    unit: &str,
    enabled: bool,
) -> Option<f64> {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let w = ui.available_width();
    let font = theme.font(type_scale::BODY);
    let (r, _) = ui.allocate_exact_size(vec2(w, 22.0), Sense::hover());
    // feste Einheitenspalte (breiteste Einheit der Zeilen), damit Felder und Schieber aller
    // Toleranzzeilen auf denselben Kanten stehen
    let unit_w = [fp_i18n::unit("px"), fp_i18n::unit("deg")]
        .iter()
        .map(|u| ui_kit::text_w(ui.ctx(), u, &font))
        .fold(10.0, f32::max)
        + sp::XS;
    let field_w = 46.0;
    let fr = Rect::from_min_size(
        pos2(r.right() - unit_w - field_w, r.top() + 1.0),
        vec2(field_w, 20.0),
    );
    // die Spur beginnt unter der Beschriftung des Schalters (der Schieber rückt sie 8 px ein)
    let sr = Rect::from_min_max(
        pos2(r.left() + CHECK_INDENT - 8.0, r.top()),
        pos2(fr.left() - sp::XS, r.bottom()),
    );
    let mut out = fields::number(
        ui,
        ("trace-tol", key),
        fr,
        None,
        Some(value),
        range,
        decimals,
        enabled,
    );
    if !unit.is_empty() {
        ui.painter().text(
            pos2(fr.right() + sp::XS, fr.center().y),
            Align2::LEFT_CENTER,
            unit,
            font,
            if enabled {
                c.text_weak
            } else {
                c.text_disabled
            },
        );
    }
    if let Some(v) = slider(
        ui,
        Id::new(("trace-tol-slider", key)),
        sr,
        value.clamp(range.0, range.1),
        range,
        step,
        enabled,
        label,
    ) {
        out = Some(v);
    }
    ui.add_space(sp::XS);
    out
}

/// Karte einer Vorgabe: Symbol, Name, ein Satz; ausgewählt getönt mit Akzentrahmen und Haken.
#[allow(clippy::too_many_arguments)]
fn paint_card(
    ui: &Ui,
    r: Rect,
    preset: Preset,
    name: &str,
    hint: &str,
    selected: bool,
    hovered: bool,
    stacked: Option<f32>,
    theme: &Theme,
) {
    let c = &theme.colors;
    let p = ui.painter();
    let radius = CornerRadius::same(5);
    let bg = if selected {
        ui_kit::tint(theme, c.focus)
    } else if hovered {
        mix(ui_kit::card_bg(theme), c.list_hover_bg, 0.7)
    } else {
        ui_kit::card_bg(theme)
    };
    if selected && theme.glow {
        p.rect_stroke(
            r.expand(1.5),
            CornerRadius::same(6),
            Stroke::new(3.0, with_alpha(c.focus, 60)),
            StrokeKind::Outside,
        );
    }
    p.rect_filled(r, radius, bg);
    let border = if selected {
        Stroke::new(1.5, c.focus)
    } else if hovered {
        Stroke::new(1.0, mix(c.list_border, c.focus, 0.55))
    } else {
        Stroke::new(
            1.0,
            if theme.glow {
                c.line_dark
            } else {
                c.list_border
            },
        )
    };
    p.rect_stroke(r, radius, border, StrokeKind::Inside);
    let ir = Rect::from_min_size(r.min + vec2(10.0, 10.0), vec2(22.0, 22.0));
    paint_preset_icon(p, ir, preset, theme);
    let font = theme.bold(type_scale::BODY);
    let mut hint_y = ir.bottom() + 6.0;
    if let Some(nh) = stacked {
        let g = p.layout(name.to_string(), font, c.text, r.width() - 20.0);
        p.galley(pos2(r.left() + 10.0, ir.bottom() + 6.0), g, c.text);
        hint_y += nh;
    } else {
        let nr = Rect::from_min_max(
            pos2(ir.right() + sp::S, ir.top()),
            pos2(r.right() - 8.0 - 18.0, ir.bottom()),
        );
        if ui_kit::paint_text_fit(p, nr, name, font, c.text) {
            report_overflow(ui.ctx(), format!("trace card {:.0}", nr.width()));
        }
    }
    if selected {
        let cc = pos2(r.right() - 14.0, r.top() + 14.0);
        p.circle_filled(cc, 7.0, c.focus);
        p.line(
            vec![
                cc + vec2(-3.5, 0.2),
                cc + vec2(-1.0, 2.8),
                cc + vec2(3.5, -2.5),
            ],
            Stroke::new(1.6, Color32::WHITE),
        );
    }
    let g = p.layout(
        hint.to_string(),
        theme.font(type_scale::CAPTION),
        c.text_weak,
        r.width() - 20.0,
    );
    p.galley(pos2(r.left() + 10.0, hint_y), g, c.text_weak);
}

/// Kleine Symbole der Vorgaben (eigene Zeichnung, Farben aus dem Design).
fn paint_preset_icon(p: &egui::Painter, r: Rect, preset: Preset, theme: &Theme) {
    let c = &theme.colors;
    let s = r.width() / 22.0;
    let at = |x: f32, y: f32| r.min + vec2(x * s, y * s);
    match preset {
        Preset::Logo => {
            p.rect_filled(
                Rect::from_min_max(at(2.0, 8.0), at(14.0, 20.0)),
                CornerRadius::same((3.0 * s) as u8),
                c.icon_blue,
            );
            p.circle_filled(at(15.0, 7.5), 6.0 * s, c.icon_red);
            p.circle_stroke(
                at(15.0, 7.5),
                6.0 * s,
                Stroke::new(1.0, with_alpha(c.icon_paper, 200)),
            );
        }
        Preset::BlackWhite => {
            let cc = at(11.0, 11.0);
            let rad = 9.0 * s;
            p.circle_filled(cc, rad, c.icon_paper);
            // linke Hälfte gefüllt
            let n = 24;
            let mut pts = vec![cc + vec2(0.0, -rad)];
            for k in 0..=n {
                let a = std::f32::consts::FRAC_PI_2 + std::f32::consts::PI * k as f32 / n as f32;
                pts.push(cc + vec2(a.cos() * rad, -a.sin() * rad));
            }
            p.add(Shape::convex_polygon(pts, c.icon_fg, Stroke::NONE));
            p.circle_stroke(cc, rad, Stroke::new(1.2, c.icon_fg));
            // Pinselschwung in Gegenfarbe
            p.line_segment(
                [at(5.0, 14.0), at(9.0, 8.0)],
                Stroke::new(1.6 * s, c.icon_paper),
            );
            p.line_segment(
                [at(13.0, 14.0), at(17.0, 8.0)],
                Stroke::new(1.6 * s, c.icon_fg),
            );
        }
        Preset::Illustration => {
            p.circle_filled(at(8.0, 8.5), 5.5 * s, with_alpha(c.icon_red, 230));
            p.circle_filled(at(14.5, 9.5), 5.5 * s, with_alpha(c.icon_yellow, 220));
            p.circle_filled(at(11.0, 15.0), 5.5 * s, with_alpha(c.icon_blue, 210));
        }
        Preset::Photo => {
            let fr = Rect::from_min_max(at(1.5, 3.5), at(20.5, 18.5));
            p.rect_filled(fr, CornerRadius::same((2.0 * s) as u8), c.icon_paper);
            p.add(Shape::convex_polygon(
                vec![at(3.0, 17.0), at(9.0, 9.0), at(14.0, 17.0)],
                c.icon_green,
                Stroke::NONE,
            ));
            p.add(Shape::convex_polygon(
                vec![at(10.0, 17.0), at(14.5, 11.5), at(19.0, 17.0)],
                mix(c.icon_green, c.icon_fg, 0.3),
                Stroke::NONE,
            ));
            p.circle_filled(at(15.5, 7.5), 2.2 * s, c.icon_yellow);
            p.rect_stroke(
                fr,
                CornerRadius::same((2.0 * s) as u8),
                Stroke::new(1.2, c.icon_fg),
                StrokeKind::Inside,
            );
        }
    }
}

/// Kopfsymbol wie die Vergleichsansicht: ein Kreis, links aus Pixeln (Original), rechts glatt
/// mit Ankerpunkten (Vektor).
fn paint_header_icon(p: &egui::Painter, r: Rect, theme: &Theme) {
    let c = &theme.colors;
    let s = r.width() / 32.0;
    let at = |x: f32, y: f32| r.min + vec2(x * s, y * s);
    let (cx, cy, rad) = (16.0f32, 16.0f32, 12.5f32);
    // linke Hälfte: Pixel auf einem 4er-Raster
    let g = 3.0;
    let mut y = cy - 12.0;
    while y < cy + 12.0 {
        let mut x = cx - 12.0;
        while x < cx - 0.5 {
            let (mx, my) = (x + g / 2.0, y + g / 2.0);
            if (mx - cx).hypot(my - cy) <= rad {
                p.rect_filled(
                    Rect::from_min_max(at(x, y), at(x + g, y + g)).shrink((0.3 * s).max(0.25)),
                    0.0,
                    with_alpha(c.icon_fg, 190),
                );
            }
            x += g;
        }
        y += g;
    }
    // rechte Hälfte: glatter Halbkreis in der Akzentfarbe
    let n = 28;
    let mut pts = Vec::with_capacity(n + 1);
    for k in 0..=n {
        let a = -std::f32::consts::FRAC_PI_2 + std::f32::consts::PI * k as f32 / n as f32;
        pts.push(at(cx + 1.0 + a.cos() * rad, cy + a.sin() * rad));
    }
    p.add(Shape::convex_polygon(pts, c.focus, Stroke::NONE));
    // Ankerpunkte
    let k = (4.0 * s).round().max(3.0);
    for (x, y) in [
        (cx + 1.0, cy - rad),
        (cx + 1.0 + rad, cy),
        (cx + 1.0, cy + rad),
    ] {
        let ar = Rect::from_center_size(at(x, y).round(), vec2(k, k));
        p.rect_filled(ar, 0.0, c.icon_paper);
        p.rect_stroke(ar, 0.0, Stroke::new(1.0, c.icon_fg), StrokeKind::Inside);
    }
}

// ---------------------------------------------------------------------- Inspektor

/// Inspektor bei Bitmaps (Q5): Knopf „Nachzeichnen…“ öffnet diesen Dialog.
pub fn inspector_q5(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let label = t("trace-inspector-button");
    let en = st.session.is_available("modify.traceBitmap");
    let font = theme.ui_font();
    let w = (ui_kit::text_w(ui.ctx(), &label, &font) + 34.0)
        .min(r.width() - 12.0)
        .max(40.0);
    let br = Rect::from_min_size(r.min + vec2(6.0, 6.0), vec2(w, 22.0));
    let resp = ui.interact(
        br,
        Id::new("pi-trace-bitmap"),
        if en { Sense::click() } else { Sense::hover() },
    );
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, en, label.clone()));
    let hot = en && resp.hovered();
    let pressed = en && resp.is_pointer_button_down_on();
    let p = ui.painter();
    p.rect_filled(
        br,
        2.0,
        if pressed {
            c.button_pressed_bg
        } else if hot {
            c.button_hover_bg
        } else {
            c.button_bg
        },
    );
    p.rect_stroke(
        br,
        2.0,
        Stroke::new(
            1.0,
            if hot {
                c.button_hover_border
            } else {
                c.button_border
            },
        ),
        StrokeKind::Inside,
    );
    let ir = Rect::from_min_size(pos2(br.left() + 5.0, br.center().y - 8.0), vec2(16.0, 16.0));
    if en {
        paint_header_icon(p, ir, &theme);
    } else {
        // gesperrt: grau
        let mut g = (*theme).clone();
        g.colors.focus = c.icon_disabled;
        g.colors.icon_fg = c.icon_disabled;
        paint_header_icon(p, ir, &g);
    }
    let tr = Rect::from_min_max(
        pos2(ir.right() + 5.0, br.top()),
        pos2(br.right() - 4.0, br.bottom()),
    );
    ui_kit::paint_text_fit(
        p,
        tr,
        &label,
        font,
        if en { c.text } else { c.text_disabled },
    );
    if resp.on_hover_text(t("trace-inspector-tip")).clicked() && en {
        st.push(Action::Run("modify.traceBitmap".into(), json!({})));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fp_geom::IRect;

    fn logo() -> RgbaImage {
        let mut img = RgbaImage::filled(
            IRect::from_xywh(10, 20, 120, 80),
            fp_color::Rgba8::new(255, 255, 255, 255),
        )
        .expect("Bild");
        let w = img.width();
        for (i, px) in img.data_mut().iter_mut().enumerate() {
            let (x, y) = ((i % w) as f64 + 0.5, (i / w) as f64 + 0.5);
            if (x - 35.0).hypot(y - 40.0) < 25.0 {
                *px = [20, 60, 200, 255];
            } else if (70.0..110.0).contains(&x) && (15.0..65.0).contains(&y) {
                *px = [220, 30, 40, 255];
            }
        }
        img
    }

    #[test]
    fn halbtransparente_bitmap_zeigt_keine_100_prozent() {
        // NP-1 (N29 Nebenbefund): Pinselstrich mit Deckkraft 50 % → keine Pfade, trotzdem „100,0 %“
        let mut img = logo();
        for px in img.data_mut().iter_mut() {
            px[3] = px[3].min(127);
        }
        let opts = TraceOptions::from_preset(Preset::Logo);
        let res = fp_trace::trace(&img, &opts).expect("nachzeichnen");
        assert!(res.paths.is_empty(), "nichts über der Alphaschwelle");
        let stats = Stats {
            paths: 0,
            points: 0,
            colors: 0,
            match_pct: match_percent(&img, &res, &opts),
            ms: 0,
            downscaled: None,
            refine: None,
            faint_alpha: faint_alpha(&img, &opts),
        };
        assert_eq!(
            stats.match_shown(),
            None,
            "keine Übereinstimmung ohne Pfade"
        );
        assert_eq!(stats.faint_alpha, Some(127));
        assert_eq!(
            faint_alpha(&logo(), &opts),
            None,
            "deckendes Bild ist nicht halbtransparent"
        );
    }

    #[test]
    fn preview_match_and_svg() {
        let img = logo();
        let opts = TraceOptions::from_preset(Preset::Logo);
        let res = fp_trace::trace(&img, &opts).expect("nachzeichnen");
        let pct = match_percent(&img, &res, &opts);
        assert!(pct > 97.0, "Übereinstimmung {pct}");
        let pv = render_preview(&res);
        assert!(pv.width() >= 120 && pv.width() as f32 <= PREVIEW_MAX);
        // Vorschau hat Kantenglättung: Zwischentöne zwischen Blau und Weiß
        assert!(
            pv.data()
                .iter()
                .any(|p| p[3] == 255 && p[0] > 40 && p[0] < 200 && p[2] > 210)
        );
        let svg = svg_string(&res).expect("svg");
        assert!(
            svg.contains("<path")
                && svg.contains("fill-rule=\"evenodd\"")
                && svg.contains("width=\"120\""),
            "{svg}"
        );
        // Koordinaten relativ zur Bitmap (kein Versatz 10/20)
        assert!(!outlines(&res).is_empty());
    }

    #[test]
    fn background_run_is_cancelled_by_a_new_one() {
        let ctx = egui::Context::default();
        let mut d = TraceDialog::new(logo(), 1, TraceOptions::default(), true);
        d.start(&ctx);
        let first = Arc::clone(&d.job.as_ref().expect("Lauf").cancel);
        d.set_preset(Preset::Photo);
        d.start(&ctx);
        assert!(first.load(Ordering::Relaxed), "alter Lauf abgebrochen");
        let t0 = Instant::now();
        while d.job.is_some() && t0.elapsed() < Duration::from_secs(20) {
            d.poll(&ctx);
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(d.runs_started, 2);
        assert!(
            d.stats.as_ref().is_some_and(|s| s.paths > 0),
            "{:?}",
            d.error
        );
        assert_eq!(d.command_params()["preset"], json!("photo"));
        assert_eq!(d.command_params()["keepOriginal"], json!(true));
    }
}
