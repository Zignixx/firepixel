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

use super::{dialog_frame, push_button, DialogClose};
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

fn title_for(cmd: &str) -> &'static str {
    match cmd {
        "select.aiObject" => "Objektauswahl",
        "select.aiSubject" => "Motiv auswählen",
        "select.aiRefineEdge" => "Kante verfeinern",
        _ => "KI-Auswahl",
    }
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
                let title = title_for(&inv.command).to_string();
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
                    Some(iou) => format!("{title}: Objekt ausgewählt (Güte {:.0} %). Umschalt+Klick fügt hinzu, Alt+Klick zieht ab.", iou * 100.0),
                    None => format!("{title}: fertig"),
                },
                _ => format!("{title}: fertig"),
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
            Err(fp_ai::AiError::Cancelled) => status(st, format!("{}: abgebrochen", j.title)),
            Err(e) => {
                st.dialogs.message = Some((j.title.clone(), e.to_string()));
            }
        }
        return;
    }
    let p = j.job.progress.get();
    let label = j.job.label.clone();
    let mut cancel = false;
    egui::Window::new("KI-Auswahl")
        .id(egui::Id::new("fp-ai-progress"))
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_BOTTOM, vec2(0.0, -48.0))
        .show(ctx, |ui| {
            ui.set_width(320.0);
            ui.label(format!("{label} – Bild wird analysiert …"));
            ui.add(egui::ProgressBar::new(p).show_percentage());
            ui.horizontal(|ui| {
                ui.label(format!("{:.1} s", j.job.elapsed()));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Abbrechen").clicked() {
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

/// Zahl mit deutschem Dezimalkomma.
fn de(v: f64, dec: usize) -> String {
    format!("{v:.dec$}").replace('.', ",")
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

fn task_text(t: models::Task) -> &'static str {
    match t {
        models::Task::Object => "Objektauswahl",
        models::Task::Subject => "Motiv auswählen",
        models::Task::Matting => "Kante verfeinern",
    }
}

/// Lizenzhinweis eines Modells (ein Absatz).
fn license_note(m: &models::ModelInfo) -> String {
    match m.task {
        models::Task::Object => format!("Lizenz {}: Gewichte und Code von Meta, kommerzielle Nutzung erlaubt. Trainiert auf SA-1B und SA-V.", m.license),
        _ => format!(
            "Lizenz {} (Gewichte und Code von Peng Zheng). Hinweis: Laut Autor stammen die Trainingsdaten teils aus Datensätzen, die nur für nichtkommerzielle Forschung freigegeben sind (z. B. P3M-10k{}).",
            m.license,
            if m.task == models::Task::Matting { ", AM-2k, Distinctions-646" } else { "" }
        ),
    }
}

/// Fortschrittstext „27 MB von 156 MB · 5,4 MB/s · noch ca. 24 s“.
pub fn progress_text(p: f32, size: u64, elapsed: f32) -> String {
    let done = (p as f64 * size as f64) as u64;
    let mut s = format!("{} von {}", size_text(done), size_text(size));
    if elapsed > 0.3 && done > 0 {
        let rate = done as f64 / elapsed as f64;
        s.push_str(&format!(" · {} MB/s", de(rate / 1e6, 1)));
        let rest = (size.saturating_sub(done)) as f64 / rate.max(1.0);
        if rest >= 1.0 {
            s.push_str(&if rest >= 90.0 { format!(" · noch ca. {} min", (rest / 60.0).round() as u64) } else { format!(" · noch ca. {} s", rest.round() as u64) });
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
    ui.push_id(id, |ui| {
        ui.spacing_mut().item_spacing.y = 6.0;
        for (label, f) in rows.iter_mut() {
            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(vec2(96.0, 20.0), egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.set_min_size(vec2(96.0, 20.0));
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
        if ui.small_button("Kopieren").on_hover_text("SHA-256 in die Zwischenablage").clicked() {
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
    let tool = d.pending.as_ref().map(|i| title_for(&i.command)).unwrap_or("die KI-Auswahl");
    #[derive(PartialEq)]
    enum Btn {
        None,
        Download,
        Cancel,
        CancelDownload,
    }
    let mut pressed = Btn::None;
    let dl_state = d.download.as_ref().map(|x| (x.job.progress.get(), x.job.elapsed()));
    let r = dialog_frame(ctx, "fp-ai-missing", "KI-Modell fehlt", vec2(600.0, 410.0), |ui, body| {
        let inner = Rect::from_min_max(body.min + vec2(PAD, PAD), body.max - vec2(PAD, PAD + BTN.y + 12.0));
        let mut c = ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(egui::Layout::top_down(egui::Align::Min)));
        header(&mut c, &format!("„{}“ wird gebraucht", m.name), &format!("Für „{tool}“ fehlt ein KI-Modell; Firepixel lädt es erst nach Ihrer Zustimmung herunter."));
        c.add_space(10.0);
        c.separator();
        c.add_space(6.0);
        let src = short_source(m);
        let fshort = ellipsize(&folder, 56);
        info_grid(
            &mut c,
            "fp-ai-missing-grid",
            &mut [
                ("Modell", &mut |ui: &mut egui::Ui| {
                    ui.label(egui::RichText::new(m.name).strong());
                }),
                ("Aufgabe", &mut |ui: &mut egui::Ui| {
                    ui.label(task_text(m.task));
                }),
                ("Größe", &mut |ui: &mut egui::Ui| {
                    ui.label(size_text(m.size));
                }),
                ("Lizenz", &mut |ui: &mut egui::Ui| {
                    ui.label(m.license);
                }),
                ("Quelle", &mut |ui: &mut egui::Ui| {
                    ui.label(&src).on_hover_text(m.url);
                }),
                ("Prüfsumme", &mut |ui: &mut egui::Ui| checksum_row(ui, m.sha256)),
                ("Zielordner", &mut |ui: &mut egui::Ui| {
                    ui.label(&fshort).on_hover_text(&folder);
                }),
            ],
        );
        c.add_space(8.0);
        hint_block(&mut c, &license_note(m));
        c.add_space(8.0);
        if d.done.is_some() {
            let g = Theme::current(c.ctx()).colors.icon_green;
            c.label(egui::RichText::new(format!("✓ {} geladen und geprüft – „{tool}“ wird ausgeführt …", m.name)).color(g).strong());
        } else if let Some((p, t)) = dl_state {
            let w = c.available_width();
            progress_bar(&mut c, p, w);
            c.label(egui::RichText::new(progress_text(p, m.size, t)).color(weak(&c)));
        } else if let Some(e) = &d.error {
            let red = egui::Color32::from_rgb(210, 60, 50);
            c.label(egui::RichText::new("Download fehlgeschlagen").color(red).strong());
            c.add(egui::Label::new(egui::RichText::new(e).color(red)).wrap());
        }
        // Knöpfe unten rechts, einheitliche Größe
        let by = body.bottom() - PAD - BTN.y;
        let right = Rect::from_min_size(pos2(body.right() - PAD - BTN.x, by), BTN);
        let left = Rect::from_min_size(pos2(right.left() - 8.0 - BTN.x, by), BTN);
        if d.done.is_some() {
            // automatisch weiter
        } else if d.download.is_some() {
            if push_button(ui, right, "Abbrechen", false, true) {
                pressed = Btn::CancelDownload;
            }
        } else {
            let label = if d.error.is_some() { "Erneut versuchen" } else { "Herunterladen" };
            if push_button(ui, left, label, true, fp_ai::download::allowed()) {
                pressed = Btn::Download;
            }
            if push_button(ui, right, "Abbrechen", false, true) {
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
            Err(fp_ai::AiError::Cancelled) => d.error = Some("Der Download wurde abgebrochen.".into()),
            Err(e) => d.error = Some(e.to_string()),
        }
    }
    if let Some(t) = d.done {
        if t.elapsed() >= std::time::Duration::from_millis(900) {
            status(st, format!("{} geladen.", m.name));
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

fn cols(w: f32) -> Cols {
    let status_w = 200.0;
    let fixed = 130.0 + 80.0 + 90.0 + status_w + 4.0 * 12.0;
    let name_w = (w - fixed - 10.0).max(180.0);
    let x_task = 10.0 + name_w + 12.0;
    let x_size_r = x_task + 130.0 + 12.0 + 80.0;
    let x_lic = x_size_r + 12.0;
    let x_status = x_lic + 90.0 + 12.0;
    Cols { name: (10.0, name_w), task: (x_task, 130.0), size_right: x_size_r, license: (x_lic, 90.0), status: (x_status, status_w) }
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
    let size = d.size;
    let r = dialog_frame(ctx, "fp-ai-models", "KI-Modelle verwalten", size, |ui, body| {
        let t = Theme::current(ui.ctx());
        let c = t.colors.clone();
        let inner = Rect::from_min_max(body.min + vec2(PAD, PAD), body.max - vec2(PAD, PAD + BTN.y + 12.0));
        let mut u = ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(egui::Layout::top_down(egui::Align::Min)));
        header(
            &mut u,
            &format!("{n_av} von {} Modellen vorhanden · {} belegt", models::MODELS.len(), size_text(used)),
            "Die KI-Modelle werden nicht mit Firepixel ausgeliefert. Heruntergeladen wird nur, was Sie hier bestätigen.",
        );
        u.add_space(10.0);
        // --- Tabelle
        let tw = u.available_width();
        let cl = cols(tw);
        let row_h = 32.0;
        let (hr, _) = u.allocate_exact_size(vec2(tw, 24.0), Sense::hover());
        let p = u.painter();
        p.rect_filled(hr, 0.0, crate::theme::mix(c.frame_bg, c.line_mid, 0.25));
        let hf = t.bold(11.5);
        let hy = hr.center().y;
        p.text(pos2(hr.left() + cl.name.0, hy), egui::Align2::LEFT_CENTER, "Modell", hf.clone(), c.text);
        p.text(pos2(hr.left() + cl.task.0, hy), egui::Align2::LEFT_CENTER, "Aufgabe", hf.clone(), c.text);
        p.text(pos2(hr.left() + cl.size_right, hy), egui::Align2::RIGHT_CENTER, "Größe", hf.clone(), c.text);
        p.text(pos2(hr.left() + cl.license.0, hy), egui::Align2::LEFT_CENTER, "Lizenz", hf.clone(), c.text);
        p.text(pos2(hr.left() + cl.status.0, hy), egui::Align2::LEFT_CENTER, "Stand", hf, c.text);
        p.hline(hr.x_range(), hr.bottom() - 0.5, Stroke::new(1.0, c.line_mid));
        let font = t.ui_font();
        for (i, m) in models::MODELS.iter().enumerate() {
            let (rr, resp) = u.allocate_exact_size(vec2(tw, row_h), Sense::hover());
            let bg = if resp.hovered() { c.list_hover_bg } else if i % 2 == 1 { crate::theme::mix(c.list_bg, c.line_light, 0.35) } else { c.list_bg };
            u.painter().rect_filled(rr, 0.0, bg);
            u.painter().hline(rr.x_range(), rr.bottom() - 0.5, Stroke::new(1.0, c.list_row_line));
            let y = rr.center().y;
            let name = if m.default { format!("{}  (Vorgabe)", m.name) } else { m.name.to_string() };
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
                    if ui.small_button("×").on_hover_text("Download abbrechen").clicked() {
                        cancel_dl = true;
                    }
                } else if avail[i] {
                    if d.confirm_remove == Some(m.id) {
                        ui.label("Entfernen?");
                        if ui.small_button("Ja").clicked() {
                            remove = Some(m.id);
                        }
                        if ui.small_button("Nein").clicked() {
                            ask_remove = None;
                            d.confirm_remove = None;
                        }
                    } else {
                        ui.label(egui::RichText::new("✓ Vorhanden").color(c.icon_green).strong());
                        ui.add_space(6.0);
                        if ui.add(egui::Button::new(egui::RichText::new("Entfernen…").small()).frame(false)).on_hover_text("Modelldatei löschen").clicked() {
                            ask_remove = Some(m.id);
                        }
                    }
                } else if ui.add_enabled(dl.is_none() && fp_ai::download::allowed(), egui::Button::new("Herunterladen").min_size(vec2(130.0, 22.0))).clicked() {
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
                &format!("„{}“ herunterladen? {} von {} (SHA-256 {}…). {}", m.name, size_text(m.size), short_source(m), &m.sha256[..12], license_note(m)),
            );
            u.horizontal(|ui| {
                if ui.add_sized(BTN, egui::Button::new("Herunterladen")).clicked() {
                    start = Some(id);
                }
                if ui.add_sized(BTN, egui::Button::new("Abbrechen")).clicked() {
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
            ui.label(egui::RichText::new("Speicherort").strong());
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let w = (ui.available_width() - 2.0 * (BTN.x + 8.0)).max(120.0);
                let resp = ui.add(egui::TextEdit::singleline(&mut d.folder).desired_width(w).hint_text("leer = %LOCALAPPDATA%\\Firepixel\\Modelle"));
                if resp.lost_focus() {
                    folder_action = Some("apply");
                }
                if ui.add_sized(BTN, egui::Button::new("Durchsuchen…")).clicked() {
                    folder_action = Some("browse");
                }
                if ui.add_sized(BTN, egui::Button::new("Standard")).clicked() {
                    folder_action = Some("default");
                }
            });
            ui.horizontal(|ui| {
                let f = models::download_folder().display().to_string();
                ui.label(egui::RichText::new(format!("Aktuell: {}", ellipsize(&f, 70))).color(c.text_weak)).on_hover_text(f);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add_sized(BTN, egui::Button::new("Ordner öffnen")).clicked() {
                        folder_action = Some("open");
                    }
                });
            });
        });
        u.add_space(8.0);
        u.horizontal(|ui| {
            ui.label(format!("Zwischenspeicher im Arbeitsspeicher: {}", size_text(mem)));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add_enabled(mem > 0, egui::Button::new("Zwischenspeicher leeren").min_size(vec2(170.0, BTN.y))).on_hover_text("Geladene Modelle und Bild-Einbettungen freigeben").clicked() {
                    clear = true;
                }
            });
        });
        // Knopf „Schließen“ und Größengriff
        let close_r = Rect::from_min_size(pos2(body.right() - PAD - BTN.x, body.bottom() - PAD - BTN.y), BTN);
        if push_button(ui, close_r, "Schließen", true, true) {
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
    d.size = vec2((d.size.x + resize.x).max(680.0), (d.size.y + resize.y).max(520.0));
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
            Some(Ok(())) => format!("{} entfernt.", models::info(id).name),
            Some(Err(e)) => format!("Entfernen fehlgeschlagen: {e}"),
            None => "Datei nicht gefunden.".into(),
        });
    }
    if cancel_dl && let Some(x) = &d.download {
        x.job.cancel();
    }
    match folder_action {
        Some("browse") => {
            if let Some(f) = rfd::FileDialog::new().set_title("Modellordner wählen").pick_folder() {
                d.folder = f.display().to_string();
                let _ = st.session.execute("ai.setModelFolder", serde_json::json!({"path": d.folder}));
                d.info = Some("Modellordner übernommen.".into());
            }
        }
        Some("default") => {
            d.folder.clear();
            let _ = st.session.execute("ai.setModelFolder", serde_json::json!({"path": ""}));
            d.info = Some("Standard-Modellordner.".into());
        }
        Some("apply") => {
            if st.session.prefs.get_str("AiModelFolder", "") != d.folder.trim() {
                let _ = st.session.execute("ai.setModelFolder", serde_json::json!({"path": d.folder.trim()}));
                d.info = Some("Modellordner übernommen.".into());
            }
        }
        Some("open") => {
            let f = models::download_folder();
            let _ = std::fs::create_dir_all(&f);
            if !matches!(fp_engine::shell::shell_open(&f, None), Ok(true)) {
                d.info = Some(format!("Ordner: {}", f.display()));
            }
        }
        _ => {}
    }
    if clear {
        let _ = st.session.execute("ai.clearCache", serde_json::json!({}));
        d.info = Some("Zwischenspeicher geleert.".into());
    }
    if let Some(x) = d.download.as_mut()
        && let Some(res) = x.job.poll()
    {
        let name = models::info(x.model).name;
        d.info = Some(match res {
            Ok(_) => format!("✓ {name} geladen und geprüft."),
            Err(fp_ai::AiError::Cancelled) => "Download abgebrochen.".into(),
            Err(e) => format!("Download fehlgeschlagen: {e}"),
        });
        d.download = None;
    }
    if d.download.is_some() {
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
    st.dialogs.ai.manager = Some(d);
}
