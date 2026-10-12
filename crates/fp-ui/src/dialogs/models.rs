//! KI-Auswahl in der Oberfläche (Agent KI):
//!
//! * Hintergrundrechnung der KI-Befehle (`select.aiObject`, `select.aiSubject`,
//!   `select.aiRefineEdge`): Bild-Encoding bzw. Freistellung laufen in einem eigenen Thread, ein
//!   kleines Fortschrittsfenster zeigt den Stand und hat „Abbrechen“; danach wird der Befehl
//!   ausgeführt (dann nur noch der schnelle Teil). Die Oberfläche blockiert nie.
//! * Dialog „KI-Modell fehlt“: Name, Quelle, Größe, Lizenz und Zielordner; heruntergeladen wird
//!   erst nach „Herunterladen“ (mit Fortschritt, Prüfsumme und Abbrechen).
//! * Dialog „KI-Modelle verwalten“ (`ai.manageModels`): alle Modelle mit Stand, Modellordner,
//!   Zwischenspeicher leeren.

use super::{dialog_frame, push_button, t, DialogClose};
use crate::app_state::{Action, AppState};
use crate::theme::Theme;
use egui::{pos2, vec2, Rect, Sense, Stroke};
use fp_engine::commands::ai as eng;
use fp_engine::commands::ai::fp_ai;
use fp_ai::job::Job;
use fp_ai::models::{self, ModelId};
use fp_engine::tools::Invocation;

/// Laufende Hintergrundrechnung.
pub struct RunningJob {
    job: Job<()>,
    /// Kennung der Vorbereitung ([`eng::Prepared::key`]).
    key: u64,
    /// Befehle, die nach der Rechnung der Reihe nach ausgeführt werden (Klicks während der
    /// Rechnung; ein einfacher Klick ersetzt die Liste, Umschalt/Alt-Klicks hängen an).
    then: Vec<Invocation>,
    title: String,
}

/// Download (im Dialog „Modell fehlt“ bzw. „Modelle verwalten“).
pub struct Download {
    job: Job<std::path::PathBuf>,
    model: ModelId,
}

/// Dialog „KI-Modell fehlt“.
pub struct MissingDialog {
    pub model: ModelId,
    /// Befehl, der nach dem Download weiterläuft.
    pub pending: Option<Invocation>,
    pub download: Option<Download>,
    pub error: Option<String>,
    /// Download fertig und geprüft (kurz anzeigen, dann automatisch weiter).
    pub done: Option<std::time::Instant>,
}

/// Dialog „KI-Modelle verwalten“.
#[derive(Default)]
pub struct ManagerDialog {
    pub folder: String,
    pub download: Option<Download>,
    pub info: Option<String>,
    /// Modell, dessen Download der Nutzer gerade bestätigen soll.
    pub confirm: Option<ModelId>,
    /// Modell, dessen Entfernen der Nutzer gerade bestätigen soll.
    pub confirm_remove: Option<ModelId>,
    /// Fenstergröße (größenveränderlich über den Griff unten rechts).
    pub size: egui::Vec2,
}

/// Zustand der KI-Oberfläche.
#[derive(Default)]
pub struct AiDialogs {
    pub job: Option<RunningJob>,
    pub missing: Option<MissingDialog>,
    pub manager: Option<ManagerDialog>,
}

impl AiDialogs {
    /// Ist ein modaler KI-Dialog offen (das Fortschrittsfenster zählt nicht)?
    pub fn modal_open(&self) -> bool {
        self.missing.is_some() || self.manager.is_some()
    }
    /// Läuft eine Berechnung?
    pub fn busy(&self) -> bool {
        self.job.is_some()
    }
    /// Wurde die laufende Berechnung abgebrochen (wartet nur noch auf das Ende des Threads)?
    pub fn cancel_requested(&self) -> bool {
        self.job.as_ref().is_some_and(|j| j.job.progress.cancel.is_cancelled())
    }
    /// Laufende Berechnung abbrechen (wie „Abbrechen“ im Fortschrittsfenster).
    pub fn cancel(&self) {
        if let Some(j) = &self.job {
            j.job.cancel();
        }
    }
}

/// Nur für Bildschirmfotos/Tests: ein Download im angegebenen Stand (lädt nichts, endet erst mit
/// „Abbrechen“).
pub fn demo_download(model: ModelId, progress: f32) -> Download {
    let job = Job::spawn("Demo", move |p| {
        p.set(progress);
        while !p.cancel.is_cancelled() {
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        Err(fp_ai::AiError::Cancelled)
    });
    Download { job, model }
}

/// Übernimmt diese Oberfläche den Befehl (aus Werkzeug, Menü oder Kürzel)?
pub fn handles(name: &str) -> bool {
    eng::is_ai_command(name) || name == "ai.manageModels"
}

fn title_for(cmd: &str) -> String {
    t(match cmd {
        "select.aiObject" => "dlg-ai-object",
        "select.aiSubject" => "dlg-ai-subject",
        "select.aiRefineEdge" => "dlg-ai-refine-edge",
        _ => "dlg-ai-selection",
    })
}

fn status(st: &mut AppState, s: impl Into<String>) {
    st.push(Action::Status(s.into()));
}

/// Befehl ohne Parameter aus dem Menü: Dialog bzw. Hintergrundrechnung. `true` = übernommen.
pub fn open_command(st: &mut AppState, name: &str) -> bool {
    if name == "ai.manageModels" {
        open_manager(st);
        return true;
    }
    if eng::is_ai_command(name) {
        request(st, &Invocation::new(name, serde_json::json!({})));
        return true;
    }
    false
}

/// „KI-Modelle verwalten“ öffnen.
pub fn open_manager(st: &mut AppState) {
    let folder = st.session.prefs.get_str("AiModelFolder", "");
    st.dialogs.ai.manager = Some(ManagerDialog { folder, ..ManagerDialog::default() });
}

/// Ersetzt der Befehl die Auswahl (einfacher Klick) oder verknüpft er sie mit der bisherigen
/// (Umschalt/Alt)? Kante verfeinern arbeitet immer auf der aktuellen Auswahl.
fn replaces(inv: &Invocation) -> bool {
    match inv.command.as_str() {
        "select.aiRefineEdge" => false,
        _ => inv.params.get("mode").and_then(|v| v.as_str()).is_none_or(|m| m == "replace") && !inv.params.get("refine").and_then(|v| v.as_bool()).unwrap_or(false),
    }
}

/// Führt einen KI-Befehl aus wie der klassische Zauberstab einen Klick: fehlt das Modell → Dialog;
/// ist der Zwischenspeicher kalt → Hintergrundrechnung, danach der Befehl; sonst sofort.
///
/// Klick während einer laufenden Rechnung: Braucht er dieselbe Vorbereitung (gleiches Bild), läuft
/// sie weiter und der Klick wird danach ausgeführt (ein einfacher Klick ersetzt die wartenden
/// Klicks, Umschalt/Alt-Klicks werden angehängt). Braucht ein einfacher Klick etwas anderes, wird
/// die alte Rechnung abgebrochen und die neue gestartet. Nichts wird verschluckt, nichts blockiert.
pub fn request(st: &mut AppState, inv: &Invocation) {
    if st.session.active().is_none() {
        return;
    }
    match eng::missing_model(&st.session, &inv.command, &inv.params) {
        Ok(Some(model)) => {
            if let Some(j) = st.dialogs.ai.job.take() {
                j.job.cancel();
            }
            st.dialogs.ai.missing = Some(MissingDialog { model, pending: Some(inv.clone()), download: None, error: None, done: None });
            return;
        }
        Ok(None) => {}
        Err(e) => {
            status(st, e.to_string());
            return;
        }
    }
    let replace = replaces(inv);
    if st.dialogs.ai.job.is_some() {
        let prep = eng::prepare(&st.session, &inv.command, &inv.params);
        let Some(j) = st.dialogs.ai.job.as_mut() else { return };
        match prep {
            // dieselbe Vorbereitung läuft schon: danach ausführen
            Ok(Some(p)) if p.key == j.key => {
                if replace {
                    j.then.clear();
                }
                j.then.push(inv.clone());
            }
            // anderes Bild bzw. schon vorbereitet: ein einfacher Klick bricht die alte Rechnung ab
            // und gilt sofort; verknüpfende Klicks warten auf die Klicks davor
            Ok(_) if replace => {
                if let Some(old) = st.dialogs.ai.job.take() {
                    old.job.cancel();
                }
                start(st, vec![inv.clone()]);
            }
            Ok(_) => j.then.push(inv.clone()),
            Err(e) => status(st, e.to_string()),
        }
        return;
    }
    start(st, vec![inv.clone()]);
}

/// Arbeitet die Befehle der Reihe nach ab; braucht einer eine Hintergrundrechnung, startet sie mit
/// ihm und den restlichen Befehlen als Folge.
fn start(st: &mut AppState, mut queue: Vec<Invocation>) {
    while !queue.is_empty() {
        let inv = queue.remove(0);
        match eng::prepare(&st.session, &inv.command, &inv.params) {
            Ok(None) => execute(st, &inv),
            // Tests/Bildschirmfotos mit synchronem Rendern: ebenfalls synchron rechnen
            Ok(Some(prep)) if st.sync_render => match eng::run_prepared(&prep, &fp_ai::Progress::new()) {
                Ok(()) => execute(st, &inv),
                Err(e) => status(st, e.to_string()),
            },
            Ok(Some(prep)) => {
                let title = title_for(&inv.command);
                let model = models::info(prep.model).name;
                let key = prep.key;
                let job = Job::spawn(format!("{title} ({model})"), move |p| eng::run_prepared(&prep, p));
                queue.insert(0, inv);
                st.dialogs.ai.job = Some(RunningJob { job, key, then: queue, title });
                return;
            }
            Err(e) => status(st, e.to_string()),
        }
    }
}

fn execute(st: &mut AppState, inv: &Invocation) {
    match st.session.execute(&inv.command, inv.params.clone()) {
        Ok(out) => {
            let title = title_for(&inv.command);
            let msg = match inv.command.as_str() {
                "select.aiObject" => match out.value.get("iou").and_then(|v| v.as_f64()) {
                    Some(iou) => fp_i18n::tr!("dlg-ai-object-selected", title = title.as_str(), quality = format!("{:.0}", iou * 100.0)),
                    None => fp_i18n::tr!("dlg-ai-done", title = title.as_str()),
                },
                _ => fp_i18n::tr!("dlg-ai-done", title = title.as_str()),
            };
            status(st, msg);
        }
        Err(e) => status(st, e.to_string()),
    }
}

/// Je Bild: Hintergrundrechnung abfragen, Fortschritt und Dialoge zeigen.
pub fn show(ctx: &egui::Context, st: &mut AppState) {
    poll_job(ctx, st);
    show_missing(ctx, st);
    show_manager(ctx, st);
}

fn poll_job(ctx: &egui::Context, st: &mut AppState) {
    let Some(j) = st.dialogs.ai.job.as_mut() else { return };
    if let Some(r) = j.job.poll() {
        let Some(j) = st.dialogs.ai.job.take() else { return };
        match r {
            Ok(()) => start(st, j.then),
            Err(fp_ai::AiError::Cancelled) => status(st, fp_i18n::tr!("dlg-ai-cancelled", title = j.title.as_str())),
            Err(e) => {
                st.dialogs.message = Some((j.title.clone(), e.to_string()));
            }
        }
        return;
    }
    let p = j.job.progress.get();
    let label = j.job.label.clone();
    let mut cancel = false;
    egui::Window::new(t("dlg-ai-selection"))
        .id(egui::Id::new("fp-ai-progress"))
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_BOTTOM, vec2(0.0, -48.0))
        .show(ctx, |ui| {
            ui.set_width(320.0);
            ui.label(fp_i18n::tr!("dlg-ai-analyzing", task = label.as_str()));
            ui.add(egui::ProgressBar::new(p).show_percentage());
            ui.horizontal(|ui| {
                ui.label(format!("{:.1} s", j.job.elapsed()));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(t("btn-cancel")).clicked() {
                        cancel = true;
                    }
                });
            });
        });
    if cancel || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        j.job.cancel();
    }
    ctx.request_repaint_after(std::time::Duration::from_millis(50));
}

/// Startet den Download eines Modells (nur aus den Dialogen nach Klick auf „Herunterladen“).
fn start_download(model: ModelId) -> Download {
    let job = Job::spawn(format!("Download {}", models::info(model).name), move |p| fp_ai::download::download(model, p));
    Download { job, model }
}

// ---------------------------------------------------------------------- Darstellung

/// Einheitliche Knopfgröße der KI-Dialoge.
const BTN: egui::Vec2 = egui::Vec2::new(116.0, 24.0);
/// Innenrand der KI-Dialoge.
const PAD: f32 = 16.0;

/// Zahl mit dem Dezimaltrennzeichen der Sprache (Deutsch: Komma).
fn de(v: f64, dec: usize) -> String {
    fp_i18n::fmt_decimal(v, dec)
}

/// Größe lesbar: „156 MB“, „1,2 GB“.
pub fn size_text(bytes: u64) -> String {
    let b = bytes as f64;
    if b >= 1e9 { format!("{} GB", de(b / 1e9, 1)) } else { format!("{} MB", (b / 1e6).round() as u64) }
}

/// Quelle als kurze Adresse ohne Protokoll („huggingface.co/facebook/sam2.1-hiera-tiny“).
pub fn short_source(m: &models::ModelInfo) -> String {
    m.page.trim_start_matches("https://").trim_start_matches("http://").to_string()
}

/// Langen Text in der Mitte kürzen („C:\Users\…\Modelle“).
pub fn ellipsize(s: &str, max: usize) -> String {
    let c: Vec<char> = s.chars().collect();
    if c.len() <= max || max < 8 {
        return s.to_string();
    }
    let head = (max - 1) / 2;
    let tail = max - 1 - head;
    let mut o: String = c[..head].iter().collect();
    o.push('…');
    o.extend(&c[c.len() - tail..]);
    o
}

fn task_text(task: models::Task) -> String {
    t(match task {
        models::Task::Object => "dlg-ai-object",
        models::Task::Subject => "dlg-ai-subject",
        models::Task::Matting => "dlg-ai-refine-edge",
    })
}

/// Lizenzhinweis eines Modells (ein Absatz).
fn license_note(m: &models::ModelInfo) -> String {
    match m.task {
        models::Task::Object => fp_i18n::tr!("dlg-ai-license-object", license = m.license),
        _ => fp_i18n::tr!("dlg-ai-license-matting", license = m.license, datasets = if m.task == models::Task::Matting { "P3M-10k, AM-2k, Distinctions-646" } else { "P3M-10k" }),
    }
}

/// Fortschrittstext „27 MB von 156 MB · 5,4 MB/s · noch ca. 24 s“.
pub fn progress_text(p: f32, size: u64, elapsed: f32) -> String {
    let done = (p as f64 * size as f64) as u64;
    // Teile mit „ · “ verbunden (Liste); jeder Teil ein eigener Text
    let mut s = fp_i18n::tr!("dlg-ai-progress-of", done = size_text(done), total = size_text(size));
    if elapsed > 0.3 && done > 0 {
        let rate = done as f64 / elapsed as f64;
        s.push_str(" · ");
        s.push_str(&fp_i18n::tr!("dlg-ai-rate", rate = de(rate / 1e6, 1)));
        let rest = (size.saturating_sub(done)) as f64 / rate.max(1.0);
        if rest >= 1.0 {
            s.push_str(" · ");
            s.push_str(&if rest >= 90.0 { fp_i18n::tr!("dlg-ai-remaining-min", n = (rest / 60.0).round() as u64) } else { fp_i18n::tr!("dlg-ai-remaining-s", n = rest.round() as u64) });
        }
    }
    s
}

fn weak(ui: &egui::Ui) -> egui::Color32 {
    Theme::current(ui.ctx()).colors.text_weak
}

/// Beschriftung/Wert-Raster: Beschriftungen rechtsbündig in gedämpfter Farbe.
/// Zeile des Rasters: Beschriftung und Inhalt.
type InfoRow<'a> = (&'a str, &'a mut dyn FnMut(&mut egui::Ui));

fn info_grid(ui: &mut egui::Ui, id: &str, rows: &mut [InfoRow]) {
    let w = weak(ui);
    // Beschriftungsspalte: mindestens 96 px wie bisher, längere Beschriftungen bekommen Platz
    let body_font = egui::TextStyle::Body.resolve(ui.style());
    let lw = rows.iter().map(|(l, _)| ui.ctx().fonts_mut(|f| f.layout_no_wrap(l.to_string(), body_font.clone(), egui::Color32::WHITE).size().x) + 4.0).fold(96.0f32, f32::max).ceil();
    ui.push_id(id, |ui| {
        ui.spacing_mut().item_spacing.y = 6.0;
        for (label, f) in rows.iter_mut() {
            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(vec2(lw, 20.0), egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.set_min_size(vec2(lw, 20.0));
                    ui.label(egui::RichText::new(*label).color(w));
                });
                ui.add_space(6.0);
                f(ui);
            });
        }
    });
}

/// Dezent hervorgehobener Hinweisblock.
fn hint_block(ui: &mut egui::Ui, text: &str) {
    let t = Theme::current(ui.ctx());
    let c = &t.colors;
    egui::Frame::new()
        .fill(crate::theme::mix(c.frame_bg, c.focus, 0.10))
        .stroke(Stroke::new(1.0, crate::theme::with_alpha(c.focus, 120)))
        .corner_radius(3.0)
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_top(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
                crate::icons::paint(ui.painter(), r, crate::icons::Icon::Info, &crate::icons::IconStyle::mono(c.focus, c.frame_bg));
                ui.add(egui::Label::new(egui::RichText::new(text).color(c.text)).wrap());
            });
        });
}

/// Fortschrittsbalken in Akzentfarbe mit Prozent.
fn progress_bar(ui: &mut egui::Ui, p: f32, width: f32) {
    let c = Theme::current(ui.ctx()).colors.clone();
    ui.add(egui::ProgressBar::new(p).desired_width(width).fill(c.focus).text(egui::RichText::new(format!("{} %", (p * 100.0).round() as u32)).color(egui::Color32::WHITE)));
}

/// Prüfsumme gekürzt in Monospace, Tooltip mit vollem Wert, Kopieren-Knopf.
fn checksum_row(ui: &mut egui::Ui, sha: &str) {
    let short = format!("{}…{}", &sha[..12], &sha[sha.len() - 8..]);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(short).font(egui::FontId::monospace(12.0))).on_hover_text(sha);
        if ui.small_button(t("dlg-ai-copy")).on_hover_text(t("dlg-ai-copy-tip")).clicked() {
            ui.ctx().copy_text(sha.to_string());
        }
    });
}

/// Kopfbereich mit Symbol, Titel und einem erklärenden Satz.
fn header(ui: &mut egui::Ui, title: &str, sentence: &str) {
    let t = Theme::current(ui.ctx());
    let c = t.colors.clone();
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(vec2(36.0, 36.0), Sense::hover());
        ui.painter().rect_filled(r, 6.0, crate::theme::mix(c.frame_bg, c.focus, 0.18));
        crate::icons::paint(ui.painter(), r.shrink(8.0), crate::icons::Icon::Import, &crate::icons::IconStyle::mono(c.focus, c.frame_bg));
        ui.add_space(6.0);
        ui.vertical(|ui| {
            ui.label(egui::RichText::new(title).font(t.bold(14.0)).color(c.text));
            ui.add(egui::Label::new(egui::RichText::new(sentence).color(c.text_weak)).wrap());
        });
    });
}

// ---------------------------------------------------------------------- „KI-Modell fehlt“

fn show_missing(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut d) = st.dialogs.ai.missing.take() else { return };
    let m = models::info(d.model);
    let folder = models::download_folder().display().to_string();
    let tool = d.pending.as_ref().map(|i| title_for(&i.command)).unwrap_or_else(|| t("dlg-ai-the-selection"));
    #[derive(PartialEq)]
    enum Btn {
        None,
        Download,
        Cancel,
        CancelDownload,
    }
    let mut pressed = Btn::None;
    let dl_state = d.download.as_ref().map(|x| (x.job.progress.get(), x.job.elapsed()));
    let (dl_l, retry_l, cancel_l) = (t("dlg-ai-download"), t("dlg-ai-retry"), t("btn-cancel"));
    let bw = super::buttons_w(ctx, &[&dl_l, &retry_l, &cancel_l], BTN.x);
    let btn = vec2(bw, BTN.y);
    let r = dialog_frame(ctx, "fp-ai-missing", &t("dlg-ai-missing-title"), vec2(600.0, 410.0), |ui, body| {
        let inner = Rect::from_min_max(body.min + vec2(PAD, PAD), body.max - vec2(PAD, PAD + BTN.y + 12.0));
        let mut c = ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(egui::Layout::top_down(egui::Align::Min)));
        header(&mut c, &fp_i18n::tr!("dlg-ai-needed", model = m.name), &fp_i18n::tr!("dlg-ai-missing-text", task = tool.as_str()));
        c.add_space(10.0);
        c.separator();
        c.add_space(6.0);
        let src = short_source(m);
        let fshort = ellipsize(&folder, 56);
        let gl = super::tl(&["dlg-ai-model", "dlg-ai-task", "dlg-ai-size", "dlg-ai-license", "dlg-ai-source", "dlg-ai-checksum", "dlg-ai-folder"]);
        info_grid(
            &mut c,
            "fp-ai-missing-grid",
            &mut [
                (gl[0].as_str(), &mut |ui: &mut egui::Ui| {
                    ui.label(egui::RichText::new(m.name).strong());
                }),
                (gl[1].as_str(), &mut |ui: &mut egui::Ui| {
                    ui.label(task_text(m.task));
                }),
                (gl[2].as_str(), &mut |ui: &mut egui::Ui| {
                    ui.label(size_text(m.size));
                }),
                (gl[3].as_str(), &mut |ui: &mut egui::Ui| {
                    ui.label(m.license);
                }),
                (gl[4].as_str(), &mut |ui: &mut egui::Ui| {
                    ui.label(&src).on_hover_text(m.url);
                }),
                (gl[5].as_str(), &mut |ui: &mut egui::Ui| checksum_row(ui, m.sha256)),
                (gl[6].as_str(), &mut |ui: &mut egui::Ui| {
                    ui.label(&fshort).on_hover_text(&folder);
                }),
            ],
        );
        c.add_space(8.0);
        hint_block(&mut c, &license_note(m));
        c.add_space(8.0);
        if d.done.is_some() {
            let g = Theme::current(c.ctx()).colors.icon_green;
            c.label(egui::RichText::new(fp_i18n::tr!("dlg-ai-loaded-running", model = m.name, task = tool.as_str())).color(g).strong());
        } else if let Some((p, t)) = dl_state {
            let w = c.available_width();
            progress_bar(&mut c, p, w);
            c.label(egui::RichText::new(progress_text(p, m.size, t)).color(weak(&c)));
        } else if let Some(e) = &d.error {
            let red = egui::Color32::from_rgb(210, 60, 50);
            c.label(egui::RichText::new(t("dlg-ai-download-failed")).color(red).strong());
            c.add(egui::Label::new(egui::RichText::new(e).color(red)).wrap());
        }
        // Knöpfe unten rechts, einheitliche Größe
        let by = body.bottom() - PAD - BTN.y;
        let right = Rect::from_min_size(pos2(body.right() - PAD - btn.x, by), btn);
        let left = Rect::from_min_size(pos2(right.left() - 8.0 - btn.x, by), btn);
        if d.done.is_some() {
            // automatisch weiter
        } else if d.download.is_some() {
            if push_button(ui, right, &cancel_l, false, true) {
                pressed = Btn::CancelDownload;
            }
        } else {
            let label = if d.error.is_some() { &retry_l } else { &dl_l };
            if push_button(ui, left, label, true, fp_ai::download::allowed()) {
                pressed = Btn::Download;
            }
            if push_button(ui, right, &cancel_l, false, true) {
                pressed = Btn::Cancel;
            }
        }
    });
    if r == DialogClose::Cancelled || pressed == Btn::Cancel {
        if let Some(x) = &d.download {
            x.job.cancel();
        }
        return;
    }
    match pressed {
        Btn::CancelDownload => {
            if let Some(x) = &d.download {
                x.job.cancel();
            }
        }
        Btn::Download => {
            d.error = None;
            d.download = Some(start_download(d.model));
        }
        _ => {}
    }
    if let Some(x) = d.download.as_mut()
        && let Some(res) = x.job.poll()
    {
        d.download = None;
        match res {
            Ok(_) => d.done = Some(std::time::Instant::now()),
            Err(fp_ai::AiError::Cancelled) => d.error = Some(t("dlg-ai-download-cancelled")),
            Err(e) => d.error = Some(e.to_string()),
        }
    }
    if let Some(t) = d.done {
        if t.elapsed() >= std::time::Duration::from_millis(900) {
            status(st, fp_i18n::tr!("dlg-ai-loaded", model = m.name));
            if let Some(inv) = d.pending.take() {
                request(st, &inv);
            }
            return;
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
    if d.download.is_some() {
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
    st.dialogs.ai.missing = Some(d);
}

// ---------------------------------------------------------------------- „KI-Modelle verwalten“

/// Spaltenlage der Modelltabelle (x relativ zur Tabelle, Breite).
struct Cols {
    name: (f32, f32),
    task: (f32, f32),
    size_right: f32,
    license: (f32, f32),
    status: (f32, f32),
}

/// `task_w`/`status_w`: Spaltenbreiten für die Texte der aktuellen Sprache (mindestens 130/200 px).
fn cols(w: f32, task_w: f32, status_w: f32) -> Cols {
    let fixed = task_w + 80.0 + 90.0 + status_w + 4.0 * 12.0;
    let name_w = (w - fixed - 10.0).max(180.0);
    let x_task = 10.0 + name_w + 12.0;
    let x_size_r = x_task + task_w + 12.0 + 80.0;
    let x_lic = x_size_r + 12.0;
    let x_status = x_lic + 90.0 + 12.0;
    Cols { name: (10.0, name_w), task: (x_task, task_w), size_right: x_size_r, license: (x_lic, 90.0), status: (x_status, status_w) }
}

fn cell(ui: &mut egui::Ui, r: Rect, f: impl FnOnce(&mut egui::Ui)) {
    let mut c = ui.new_child(egui::UiBuilder::new().max_rect(r).layout(egui::Layout::left_to_right(egui::Align::Center)));
    f(&mut c);
}

fn show_manager(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut d) = st.dialogs.ai.manager.take() else { return };
    if d.size == egui::Vec2::ZERO {
        d.size = vec2(800.0, 600.0);
    }
    let mut close = false;
    let mut start: Option<ModelId> = None;
    let mut ask: Option<ModelId> = None;
    let mut cancel_dl = false;
    let mut remove: Option<ModelId> = None;
    let mut ask_remove: Option<ModelId> = None;
    let mut folder_action: Option<&'static str> = None;
    let mut clear = false;
    let mut resize = egui::Vec2::ZERO;
    let dl = d.download.as_ref().map(|x| (x.model, x.job.progress.get(), x.job.elapsed()));
    let avail: Vec<bool> = models::MODELS.iter().map(|m| models::available(m.id)).collect();
    let n_av = avail.iter().filter(|a| **a).count();
    let used: u64 = models::MODELS.iter().zip(&avail).filter(|(_, a)| **a).map(|(m, _)| m.size).sum();
    let mem = fp_ai::cache::memory_bytes();
    // Texte der aktuellen Sprache; Spalten und Knöpfe wachsen mit (Deutsch: wie bisher)
    let heads = super::tl(&["dlg-ai-model", "dlg-ai-task", "dlg-ai-size", "dlg-ai-license", "dlg-ai-status"]);
    let tasks: Vec<String> = [models::Task::Object, models::Task::Subject, models::Task::Matting].into_iter().map(task_text).collect();
    let (dl_l, present_l, remove_l, remove_q, yes_l, no_l) = (t("dlg-ai-download"), t("dlg-ai-present"), t("dlg-ai-remove"), t("dlg-ai-remove-question"), t("dlg-ai-yes"), t("dlg-ai-no"));
    let (browse_l, default_l, open_l, clear_l, close_l, cancel_l) = (t("btn-browse"), t("dlg-ai-default-folder"), t("dlg-ai-open-folder"), t("dlg-ai-clear-cache"), t("btn-close"), t("btn-cancel"));
    let task_w = super::max_text_w(ctx, &super::refs(&tasks)).max(super::text_w(ctx, &heads[1])).max(130.0 - 8.0) + 8.0;
    let dlw = super::button_w(ctx, &dl_l, 130.0);
    let status_w = 200f32
        .max(dlw)
        .max(super::text_w(ctx, &present_l) * 1.1 + 6.0 + super::text_w(ctx, &remove_l) + 12.0)
        .max(super::text_w(ctx, &remove_q) + super::text_w(ctx, &yes_l) + super::text_w(ctx, &no_l) + 40.0)
        .ceil();
    let bw = super::buttons_w(ctx, &[&dl_l, &cancel_l, &browse_l, &default_l, &open_l, &close_l], BTN.x);
    let btn = vec2(bw, BTN.y);
    let cw = super::button_w(ctx, &clear_l, 170.0);
    let min_w = 680f32.max(2.0 * PAD + 10.0 + 180.0 + task_w + 80.0 + 90.0 + status_w + 4.0 * 12.0).ceil();
    let size = vec2(d.size.x.max(min_w), d.size.y);
    let r = dialog_frame(ctx, "fp-ai-models", &t("dlg-ai-manage-title"), size, |ui, body| {
        let th = Theme::current(ui.ctx());
        let c = th.colors.clone();
        let inner = Rect::from_min_max(body.min + vec2(PAD, PAD), body.max - vec2(PAD, PAD + BTN.y + 12.0));
        let mut u = ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(egui::Layout::top_down(egui::Align::Min)));
        header(
            &mut u,
            &fp_i18n::tr!("dlg-ai-manage-summary", present = n_av, total = models::MODELS.len(), used = size_text(used)),
            &t("dlg-ai-manage-text"),
        );
        u.add_space(10.0);
        // --- Tabelle
        let tw = u.available_width();
        let cl = cols(tw, task_w, status_w);
        let row_h = 32.0;
        let (hr, _) = u.allocate_exact_size(vec2(tw, 24.0), Sense::hover());
        let p = u.painter();
        p.rect_filled(hr, 0.0, crate::theme::mix(c.frame_bg, c.line_mid, 0.25));
        let hf = th.bold(11.5);
        let hy = hr.center().y;
        p.text(pos2(hr.left() + cl.name.0, hy), egui::Align2::LEFT_CENTER, heads[0].as_str(), hf.clone(), c.text);
        p.text(pos2(hr.left() + cl.task.0, hy), egui::Align2::LEFT_CENTER, heads[1].as_str(), hf.clone(), c.text);
        p.text(pos2(hr.left() + cl.size_right, hy), egui::Align2::RIGHT_CENTER, heads[2].as_str(), hf.clone(), c.text);
        p.text(pos2(hr.left() + cl.license.0, hy), egui::Align2::LEFT_CENTER, heads[3].as_str(), hf.clone(), c.text);
        p.text(pos2(hr.left() + cl.status.0, hy), egui::Align2::LEFT_CENTER, heads[4].as_str(), hf, c.text);
        p.hline(hr.x_range(), hr.bottom() - 0.5, Stroke::new(1.0, c.line_mid));
        let font = th.ui_font();
        for (i, m) in models::MODELS.iter().enumerate() {
            let (rr, resp) = u.allocate_exact_size(vec2(tw, row_h), Sense::hover());
            let bg = if resp.hovered() { c.list_hover_bg } else if i % 2 == 1 { crate::theme::mix(c.list_bg, c.line_light, 0.35) } else { c.list_bg };
            u.painter().rect_filled(rr, 0.0, bg);
            u.painter().hline(rr.x_range(), rr.bottom() - 0.5, Stroke::new(1.0, c.list_row_line));
            let y = rr.center().y;
            let name = if m.default { fp_i18n::tr!("dlg-ai-default-model", model = m.name) } else { m.name.to_string() };
            let name_rect = Rect::from_min_size(pos2(rr.left() + cl.name.0, rr.top()), vec2(cl.name.1, row_h));
            u.painter().with_clip_rect(name_rect).text(pos2(name_rect.left(), y), egui::Align2::LEFT_CENTER, name, font.clone(), c.text);
            u.interact(name_rect, egui::Id::new(("fp-ai-row", i)), Sense::hover()).on_hover_text(format!("{}\n{}", short_source(m), m.note));
            u.painter().text(pos2(rr.left() + cl.task.0, y), egui::Align2::LEFT_CENTER, task_text(m.task), font.clone(), c.text);
            u.painter().text(pos2(rr.left() + cl.size_right, y), egui::Align2::RIGHT_CENTER, size_text(m.size), font.clone(), c.text);
            u.painter().text(pos2(rr.left() + cl.license.0, y), egui::Align2::LEFT_CENTER, m.license, font.clone(), c.text);
            let sr = Rect::from_min_size(pos2(rr.left() + cl.status.0, rr.top() + 4.0), vec2(cl.status.1, row_h - 8.0));
            cell(&mut u, sr, |ui| {
                if let Some((id, pr, _)) = dl
                    && id == m.id
                {
                    progress_bar(ui, pr, 130.0);
                    if ui.small_button("×").on_hover_text(t("dlg-ai-cancel-download")).clicked() {
                        cancel_dl = true;
                    }
                } else if avail[i] {
                    if d.confirm_remove == Some(m.id) {
                        ui.label(remove_q.as_str());
                        if ui.small_button(yes_l.as_str()).clicked() {
                            remove = Some(m.id);
                        }
                        if ui.small_button(no_l.as_str()).clicked() {
                            ask_remove = None;
                            d.confirm_remove = None;
                        }
                    } else {
                        ui.label(egui::RichText::new(present_l.as_str()).color(c.icon_green).strong());
                        ui.add_space(6.0);
                        if ui.add(egui::Button::new(egui::RichText::new(remove_l.as_str()).small()).frame(false)).on_hover_text(t("dlg-ai-remove-tip")).clicked() {
                            ask_remove = Some(m.id);
                        }
                    }
                } else if ui.add_enabled(dl.is_none() && fp_ai::download::allowed(), egui::Button::new(dl_l.as_str()).min_size(vec2(130.0, 22.0))).clicked() {
                    ask = Some(m.id);
                }
            });
        }
        // laufender Download: Kennzahlen
        if let Some((id, pr, el)) = dl {
            u.add_space(4.0);
            u.label(egui::RichText::new(format!("{}: {}", models::info(id).name, progress_text(pr, models::info(id).size, el))).color(c.text_weak));
        }
        // Bestätigung eines Downloads mit allen Angaben
        if let Some(id) = d.confirm {
            let m = models::info(id);
            u.add_space(8.0);
            hint_block(
                &mut u,
                &fp_i18n::tr!("dlg-ai-confirm-download", model = m.name, size = size_text(m.size), source = short_source(m), sha = &m.sha256[..12], license = license_note(m)),
            );
            u.horizontal(|ui| {
                if ui.add_sized(btn, egui::Button::new(dl_l.as_str())).clicked() {
                    start = Some(id);
                }
                if ui.add_sized(btn, egui::Button::new(cancel_l.as_str())).clicked() {
                    d.confirm = None;
                }
            });
        }
        if let Some(i) = &d.info {
            u.add_space(4.0);
            u.label(egui::RichText::new(i).color(c.text_weak));
        }
        u.add_space(10.0);
        // --- Speicherort
        egui::Frame::group(u.style()).inner_margin(egui::Margin::same(10)).show(&mut u, |ui| {
            ui.set_width(ui.available_width());
            ui.label(egui::RichText::new(t("dlg-ai-location")).strong());
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let w = (ui.available_width() - 2.0 * (btn.x + 8.0)).max(120.0);
                let resp = ui.add(egui::TextEdit::singleline(&mut d.folder).desired_width(w).hint_text(t("dlg-ai-folder-hint")));
                if resp.lost_focus() {
                    folder_action = Some("apply");
                }
                if ui.add_sized(btn, egui::Button::new(browse_l.as_str())).clicked() {
                    folder_action = Some("browse");
                }
                if ui.add_sized(btn, egui::Button::new(default_l.as_str())).clicked() {
                    folder_action = Some("default");
                }
            });
            ui.horizontal(|ui| {
                let f = models::download_folder().display().to_string();
                ui.label(egui::RichText::new(fp_i18n::tr!("dlg-ai-current-folder", path = ellipsize(&f, 70))).color(c.text_weak)).on_hover_text(f);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add_sized(btn, egui::Button::new(open_l.as_str())).clicked() {
                        folder_action = Some("open");
                    }
                });
            });
        });
        u.add_space(8.0);
        u.horizontal(|ui| {
            ui.label(fp_i18n::tr!("dlg-ai-memory", size = size_text(mem)));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add_enabled(mem > 0, egui::Button::new(clear_l.as_str()).min_size(vec2(cw, BTN.y))).on_hover_text(t("dlg-ai-clear-cache-tip")).clicked() {
                    clear = true;
                }
            });
        });
        // Knopf „Schließen“ und Größengriff
        let close_r = Rect::from_min_size(pos2(body.right() - PAD - btn.x, body.bottom() - PAD - btn.y), btn);
        if push_button(ui, close_r, &close_l, true, true) {
            close = true;
        }
        let grip = Rect::from_min_size(body.max - vec2(14.0, 14.0), vec2(14.0, 14.0));
        let gr = ui.interact(grip, egui::Id::new("fp-ai-models-grip"), Sense::drag());
        if gr.hovered() || gr.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeNwSe);
        }
        resize = gr.drag_delta();
        for k in 0..3 {
            let o = 3.0 + 4.0 * k as f32;
            ui.painter().line_segment([pos2(grip.right() - o, grip.bottom() - 2.0), pos2(grip.right() - 2.0, grip.bottom() - o)], Stroke::new(1.0, c.line_mid));
        }
    });
    if r == DialogClose::Cancelled || close {
        if let Some(x) = &d.download {
            x.job.cancel();
        }
        return;
    }
    d.size = vec2((size.x + resize.x).max(min_w), (d.size.y + resize.y).max(520.0));
    if let Some(id) = ask {
        d.confirm = Some(id);
        d.confirm_remove = None;
    }
    if let Some(id) = ask_remove {
        d.confirm_remove = Some(id);
    }
    if let Some(id) = start {
        d.confirm = None;
        d.info = None;
        d.download = Some(start_download(id));
    }
    if let Some(id) = remove {
        d.confirm_remove = None;
        fp_ai::cache::unload(id);
        d.info = Some(match models::find(id).map(std::fs::remove_file) {
            Some(Ok(())) => fp_i18n::tr!("dlg-ai-removed", model = models::info(id).name),
            Some(Err(e)) => fp_i18n::tr!("dlg-ai-remove-failed", error = e.to_string()),
            None => t("dlg-ai-file-not-found"),
        });
    }
    if cancel_dl && let Some(x) = &d.download {
        x.job.cancel();
    }
    match folder_action {
        Some("browse") => {
            if let Some(f) = rfd::FileDialog::new().set_title(t("dlg-ai-pick-folder")).pick_folder() {
                d.folder = f.display().to_string();
                let _ = st.session.execute("ai.setModelFolder", serde_json::json!({"path": d.folder}));
                d.info = Some(t("dlg-ai-folder-applied"));
            }
        }
        Some("default") => {
            d.folder.clear();
            let _ = st.session.execute("ai.setModelFolder", serde_json::json!({"path": ""}));
            d.info = Some(t("dlg-ai-folder-default"));
        }
        Some("apply") => {
            if st.session.prefs.get_str("AiModelFolder", "") != d.folder.trim() {
                let _ = st.session.execute("ai.setModelFolder", serde_json::json!({"path": d.folder.trim()}));
                d.info = Some(t("dlg-ai-folder-applied"));
            }
        }
        Some("open") => {
            let f = models::download_folder();
            let _ = std::fs::create_dir_all(&f);
            if !matches!(fp_engine::shell::shell_open(&f, None), Ok(true)) {
                d.info = Some(fp_i18n::tr!("dlg-ai-folder-path", path = f.display().to_string()));
            }
        }
        _ => {}
    }
    if clear {
        let _ = st.session.execute("ai.clearCache", serde_json::json!({}));
        d.info = Some(t("dlg-ai-cache-cleared"));
    }
    if let Some(x) = d.download.as_mut()
        && let Some(res) = x.job.poll()
    {
        let name = models::info(x.model).name;
        d.info = Some(match res {
            Ok(_) => fp_i18n::tr!("dlg-ai-loaded-checked", model = name),
            Err(fp_ai::AiError::Cancelled) => t("dlg-ai-download-aborted"),
            Err(e) => fp_i18n::tr!("dlg-ai-download-failed-error", error = e.to_string()),
        });
        d.download = None;
    }
    if d.download.is_some() {
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
    st.dialogs.ai.manager = Some(d);
}
