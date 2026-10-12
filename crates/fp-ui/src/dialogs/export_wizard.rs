//! „Exportassistent“ (re\18 §8): die Seitenfolge Start → [Status] → Ziel → Ergebnis als
//! Zustandsautomat in **einem** Fenster mit Zurück, Weiter und Abbrechen (Paket E3, nur
//! funktional; das Original öffnet je Seite ein eigenes Fenster ohne Zurück). „Beenden“ öffnet die
//! Bildvorschau mit dem Vorschlag; erst deren OK übernimmt die Einstellungen (§8.1).

use super::{dialog_frame, push_button, t, DialogClose};
use crate::app_state::AppState;
use crate::inspector::fields;
use egui::{vec2, Rect};
use fp_export::options::{ColorMode, ExportFormat, OptimizeSettings, PaletteKind};
use serde_json::{json, Value};

/// Seite des Assistenten.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// Seite 1 (4717): Format wählen oder aktuelle Einstellungen auswerten, Zielgröße.
    Start,
    /// „Status“ (4726), nur bei mehr als einem Status.
    Frames,
    /// „Ziel wählen“ (4719).
    Destination,
    /// Ergebnisseite mit Erklärtext und „Beenden“.
    Result(ResultPage),
}

/// Ergebnisseiten (§8.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResultPage {
    /// „Analyseergebnisse“ (4720): Bildvorschau 2-fach GIF | JPEG.
    WebAnalysis,
    /// „Ausgewähltes Format“ (4727): TIFF.
    Tiff,
    FixedPalette,
    AdaptivePalette,
    Grayscale,
    Jpeg,
    FullColor,
}

/// Wahl auf der Seite „Status“.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FramesChoice {
    AnimatedGif,
    Rollover,
    SingleImage,
}

/// Ziele der Seite „Ziel wählen“.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Destination {
    Web,
    ImageEditor,
    Publishing,
    WebPage,
}

/// Zustand des Assistenten.
#[derive(Clone, Debug, PartialEq)]
pub struct ExportWizard {
    pub step: Step,
    /// Besuchte Seiten (für „Zurück“).
    pub history: Vec<Step>,
    /// „Aktuelle Formateinstellungen auswerten“ statt „Ein Exportformat auswählen“.
    pub analyze: bool,
    /// Auswertbar (GIF, PNG 8, JPEG); sonst ist die Option grau.
    pub analyzable: bool,
    pub target_on: bool,
    /// Zielgröße in KB.
    pub target_kb: f64,
    pub frames_choice: FramesChoice,
    pub destination: Destination,
    /// Anzahl Status des Dokuments.
    pub frames: usize,
    /// Exporteinstellung der Seite beim Öffnen.
    pub current: OptimizeSettings,
}

/// Ergebnisseite für „Aktuelle Formateinstellungen auswerten“.
pub fn analysis_page(s: &OptimizeSettings) -> Option<ResultPage> {
    match s.format {
        ExportFormat::Jpeg => Some(ResultPage::Jpeg),
        ExportFormat::Gif | ExportFormat::AnimatedGif => Some(palette_page(s)),
        ExportFormat::Png if s.color_mode == ColorMode::Indexed => Some(palette_page(s)),
        _ => None,
    }
}

fn palette_page(s: &OptimizeSettings) -> ResultPage {
    match s.palette_kind {
        PaletteKind::Adaptive => ResultPage::AdaptivePalette,
        PaletteKind::Grayscale => ResultPage::Grayscale,
        _ => ResultPage::FixedPalette,
    }
}

impl ExportWizard {
    pub fn new(st: &AppState) -> Option<ExportWizard> {
        let d = st.session.active()?;
        let current = fp_export::doc_export::page_settings(&d.doc, d.page);
        let analyzable = analysis_page(&current).is_some();
        let frames = d.doc.pages.get(d.page).map(|p| p.root.frame_count.max(1) as usize).unwrap_or(1);
        Some(ExportWizard {
            step: Step::Start,
            history: Vec::new(),
            analyze: analyzable,
            analyzable,
            target_on: false,
            target_kb: 32.0,
            frames_choice: FramesChoice::AnimatedGif,
            destination: Destination::Web,
            frames,
            current,
        })
    }

    /// Nächste Seite nach „Weiter“; `None` = direkt in die Bildvorschau (Animiertes GIF).
    pub fn next_step(&self) -> Option<Step> {
        match self.step {
            Step::Start if self.analyze && self.analyzable => Some(Step::Result(analysis_page(&self.current).unwrap_or(ResultPage::FullColor))),
            Step::Start if self.frames > 1 => Some(Step::Frames),
            Step::Start => Some(Step::Destination),
            Step::Frames => match self.frames_choice {
                FramesChoice::AnimatedGif => None,
                FramesChoice::Rollover => Some(Step::Result(ResultPage::WebAnalysis)),
                FramesChoice::SingleImage => Some(Step::Destination),
            },
            Step::Destination => Some(Step::Result(match self.destination {
                Destination::Web | Destination::WebPage => ResultPage::WebAnalysis,
                Destination::ImageEditor | Destination::Publishing => ResultPage::Tiff,
            })),
            Step::Result(_) => None,
        }
    }

    /// „Weiter“; `true`, wenn der Assistent jetzt in die Bildvorschau wechselt.
    pub fn advance(&mut self) -> bool {
        match self.next_step() {
            Some(s) => {
                self.history.push(self.step);
                self.step = s;
                false
            }
            None => true,
        }
    }

    /// „Zurück“.
    pub fn back(&mut self) {
        if let Some(s) = self.history.pop() {
            self.step = s;
        }
    }

    /// Vorschlag für die Bildvorschau: eine Einstellung je Feld (1 oder 2 Felder).
    pub fn proposal(&self) -> Vec<OptimizeSettings> {
        let base = self.current.clone();
        let with = |v: Value| fp_engine::commands::export::apply_settings_json(base.clone(), &v, "file.exportWizard").unwrap_or_else(|_| base.clone());
        let gif = || with(json!({"format": "gif", "palette": "adaptive", "colors": 128}));
        let jpeg = || with(json!({"format": "jpeg", "quality": 80, "smoothing": 0}));
        match self.step {
            Step::Frames if self.frames_choice == FramesChoice::AnimatedGif => vec![with(json!({"format": "animatedGif", "palette": "adaptive", "colors": 128}))],
            Step::Result(ResultPage::WebAnalysis) => vec![gif(), jpeg()],
            Step::Result(ResultPage::Tiff) => {
                let mut s = with(json!({"format": "tiff"}));
                s.color_mode = ColorMode::Rgb24;
                vec![s]
            }
            _ => vec![base],
        }
    }
}

/// Öffnet den Assistenten (Datei ▸ Exportassistent…, Knopf im Optimieren-Bedienfeld).
pub fn open(st: &mut AppState) -> bool {
    st.dialogs.wizard = ExportWizard::new(st);
    true
}

/// „Beenden“: Bildvorschau mit dem Vorschlag öffnen (bei Zielgröße auf die Größe optimiert).
pub fn finish(st: &mut AppState, w: &ExportWizard) {
    let mut list = w.proposal();
    if w.target_on {
        let bytes = (w.target_kb.max(0.1) * 1024.0).round() as i64;
        for s in list.iter_mut() {
            let v = json!({"settings": fp_engine::commands::export::settings_json(s), "targetSize": bytes});
            if let Ok(o) = crate::ui_commands::execute(&mut st.session, "file.imagePreview", v)
                && let Ok(fitted) = fp_engine::commands::export::apply_settings_json(s.clone(), &o.value["settings"], "file.exportWizard")
            {
                *s = fitted;
            }
        }
    }
    let own = crate::ui_commands::export_presets(&st.session.prefs);
    let mut p = super::image_preview::ImagePreview::with_settings(list[0].clone(), &own);
    if let Some(second) = list.get(1) {
        p.set_mode(super::image_preview::ViewMode::Two);
        p.panes[1].settings = second.clone();
        p.active = 0;
    }
    st.dialogs.preview = Some(p);
}

/// Zeigt den Assistenten.
pub fn show(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut w) = st.dialogs.wizard.take() else { return };
    match show_dialog(ctx, &mut w) {
        Some(true) => finish(st, &w),
        Some(false) => {}
        None => st.dialogs.wizard = Some(w),
    }
}

/// Text der Ergebnisseite.
fn result_text(p: ResultPage) -> (String, String) {
    let k = match p {
        ResultPage::WebAnalysis => "analysis",
        ResultPage::Tiff => "tiff",
        ResultPage::FixedPalette => "fixed",
        ResultPage::AdaptivePalette => "adaptive",
        ResultPage::Grayscale => "gray",
        ResultPage::Jpeg => "jpeg",
        ResultPage::FullColor => "full",
    };
    (t(&format!("dlg-wiz-res-{k}-title")), t(&format!("dlg-wiz-res-{k}")))
}

/// Zeichnet die aktuelle Seite; `Some(true)` = in die Bildvorschau, `Some(false)` = Abbrechen.
pub fn show_dialog(ctx: &egui::Context, w: &mut ExportWizard) -> Option<bool> {
    let mut ans = None;
    let title = match w.step {
        Step::Result(p) => result_text(p).0,
        _ => t("dlg-md-export-wizard"),
    };
    let c = dialog_frame(ctx, "fp-export-wizard", &title, vec2(440.0, 330.0), |ui, body| {
        // Startfokus auf dem Standardknopf „Weiter“ (Enter = Weiter)
        super::autofocus_default_button(ui);
        let o = body.min + vec2(16.0, 14.0);
        let full = body.width() - 32.0;
        let text = |ui: &mut egui::Ui, y: f32, h: f32, s: &str, on: bool| super::put_left(ui, Rect::from_min_size(o + vec2(0.0, y), vec2(full, h)), egui::Label::new(if on { egui::RichText::new(s) } else { egui::RichText::new(s).weak() }).wrap());
        match w.step {
            Step::Start => {
                text(ui, 0.0, 20.0, &t("dlg-wiz-helps"), true);
                for (k, key) in ["dlg-wiz-help-format", "dlg-wiz-help-optimize", "dlg-wiz-help-size"].iter().enumerate() {
                    text(ui, 24.0 + k as f32 * 36.0, 34.0, &format!("• {}", t(key)), true);
                }
                text(ui, 140.0, 20.0, &t("dlg-wiz-next-q"), true);
                if fields::radio(ui, "wiz-select", Rect::from_min_size(o + vec2(8.0, 164.0), vec2(full - 8.0, 20.0)), &t("dlg-wiz-select-format"), !w.analyze || !w.analyzable, true) {
                    w.analyze = false;
                }
                if fields::radio(ui, "wiz-analyze", Rect::from_min_size(o + vec2(8.0, 186.0), vec2(full - 8.0, 20.0)), &t("dlg-wiz-analyze"), w.analyze && w.analyzable, w.analyzable) {
                    w.analyze = true;
                }
                if let Some(b) = fields::checkbox(ui, "wiz-target", Rect::from_min_size(o + vec2(8.0, 214.0), vec2(full - 8.0, 18.0)), &t("dlg-wiz-target"), w.target_on, true) {
                    w.target_on = b;
                }
                if let Some(v) = fields::number(ui, "wiz-kb", Rect::from_min_size(o + vec2(28.0, 236.0), vec2(60.0, 22.0)), None, Some(w.target_kb), (0.1, 100000.0), 2, w.target_on) {
                    w.target_kb = v;
                }
                ui.painter().text(o + vec2(94.0, 247.0), egui::Align2::LEFT_CENTER, t("unit-kb"), crate::theme::Theme::current(ui.ctx()).ui_font(), crate::theme::Theme::current(ui.ctx()).colors.text);
            }
            Step::Frames => {
                text(ui, 0.0, 40.0, &t("dlg-wiz-frames-q"), true);
                for (k, (key, v)) in [("dlg-wiz-animated-gif", FramesChoice::AnimatedGif), ("dlg-wiz-rollover", FramesChoice::Rollover), ("dlg-wiz-single", FramesChoice::SingleImage)].into_iter().enumerate() {
                    if fields::radio(ui, ("wiz-frames", k), Rect::from_min_size(o + vec2(8.0, 48.0 + k as f32 * 24.0), vec2(full - 8.0, 20.0)), &t(key), w.frames_choice == v, true) {
                        w.frames_choice = v;
                    }
                }
            }
            Step::Destination => {
                text(ui, 0.0, 40.0, &t("dlg-wiz-dest-q"), true);
                for (k, (key, v)) in [("dlg-wiz-dest-web", Destination::Web), ("dlg-wiz-dest-image", Destination::ImageEditor), ("dlg-wiz-dest-dtp", Destination::Publishing), ("dlg-wiz-dest-page", Destination::WebPage)].into_iter().enumerate() {
                    if fields::radio(ui, ("wiz-dest", k), Rect::from_min_size(o + vec2(8.0, 48.0 + k as f32 * 24.0), vec2(full - 8.0, 20.0)), &t(key), w.destination == v, true) {
                        w.destination = v;
                    }
                }
            }
            Step::Result(p) => {
                text(ui, 0.0, 220.0, &result_text(p).1, true);
            }
        }
        // Zurück | Weiter bzw. Beenden | Abbrechen
        let finish_now = w.next_step().is_none();
        let (back_l, next_l, cancel_l) = (t("dlg-wiz-back"), if finish_now { t("dlg-wiz-finish") } else { t("dlg-wiz-continue") }, t("btn-cancel"));
        let rects = super::button_row(ui.ctx(), body.right() - 12.0, body.bottom() - 35.0, &[&back_l, &next_l, &cancel_l], 75.0, 23.0, 8.0);
        if push_button(ui, rects[0], &back_l, false, !w.history.is_empty()) {
            w.back();
        }
        if push_button(ui, rects[1], &next_l, true, true) && w.advance() {
            ans = Some(true);
        }
        if push_button(ui, rects[2], &cancel_l, false, true) {
            ans = Some(false);
        }
    });
    if c == DialogClose::Cancelled { Some(false) } else { ans }
}
