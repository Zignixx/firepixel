//! Datei-Dialoge ohne Blockieren der Oberfläche.
//!
//! Früher liefen die nativen Dialoge (rfd, Windows Common Item Dialog) direkt im UI-Thread: solange
//! einer offen war, zeichnete Firepixel nicht weiter und die MCP-Brücke bekam keine Frames – ein
//! Agent kam nicht weiter. Jetzt:
//!
//! - Ein Aufrufer beschreibt den Dialog ([`Request`]) und gibt eine Fortsetzung mit, die das
//!   Ergebnis bekommt (leere Liste = abgebrochen). [`request`] kehrt sofort zurück.
//! - Der native Dialog läuft in einem eigenen Thread, mit dem Hauptfenster als Besitzer
//!   (`set_parent`): er erscheint zentriert über Firepixel, modal zum Hauptfenster und ohne eigenen
//!   Taskleisten-Knopf. Die App zeichnet weiter; Leinwand, Menüs und Kürzel sind wie bei einem
//!   modalen Dialog gesperrt (egui-Modal, `Dialogs::any_open`).
//! - Bei laufendem KI-Steuerkanal (MCP-Brücke) erscheint statt des nativen Dialogs ein Dialog in
//!   der App, den Agenten mit dem Werkzeug `ui_file_dialog` (oder der Nutzer per Eingabe)
//!   beantworten. `FP_FILE_DIALOG=native` (alt `FP_DATEIDIALOG=nativ`) erzwingt auch dann den nativen Dialog.
//! - Testmodus `FP_FILE_DIALOG=cancel` bzw. `FP_FILE_DIALOG=path:<ordner>`: kein Dialog, sofort
//!   abgebrochen bzw. mit einer Datei im Ordner beantwortet (Speichern: vorgeschlagener Name,
//!   Öffnen: passende Dateien des Ordners, Ordner: der Ordner selbst). Jedes Ereignis wird geloggt
//!   und steht in `ui_state` (`file_dialog`).
//! - [`crate::files::FilePickOverride`] (Tests) beantwortet wie bisher sofort.

use crate::app_state::AppState;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Art des Dialogs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Eine Datei öffnen.
    Open,
    /// Mehrere Dateien öffnen.
    OpenMany,
    /// Speichern unter.
    Save,
    /// Ordner wählen.
    Folder,
}

impl Kind {
    fn name(self) -> &'static str {
        match self {
            Kind::Open => "open",
            Kind::OpenMany => "open_many",
            Kind::Save => "save",
            Kind::Folder => "folder",
        }
    }
}

/// Beschreibung eines Datei-Dialogs.
#[derive(Clone, Debug)]
pub struct Request {
    pub kind: Kind,
    pub title: String,
    /// (Beschreibung, Endungen ohne Punkt; `*` = alle).
    pub filters: Vec<(String, Vec<String>)>,
    pub dir: Option<PathBuf>,
    /// Vorgeschlagener Dateiname (Speichern).
    pub file_name: Option<String>,
}

impl Request {
    pub fn new(kind: Kind, title: impl Into<String>) -> Request {
        Request { kind, title: title.into(), filters: Vec::new(), dir: None, file_name: None }
    }
    pub fn open(title: impl Into<String>) -> Request {
        Request::new(Kind::Open, title)
    }
    pub fn open_many(title: impl Into<String>) -> Request {
        Request::new(Kind::OpenMany, title)
    }
    pub fn save(title: impl Into<String>, file_name: impl Into<String>) -> Request {
        Request { file_name: Some(file_name.into()), ..Request::new(Kind::Save, title) }
    }
    pub fn folder(title: impl Into<String>) -> Request {
        Request::new(Kind::Folder, title)
    }
    pub fn filter(mut self, name: impl Into<String>, exts: &[&str]) -> Request {
        self.filters.push((name.into(), exts.iter().map(|e| e.trim_start_matches('.').to_string()).collect()));
        self
    }
    pub fn dir(mut self, dir: Option<&Path>) -> Request {
        self.dir = dir.map(Path::to_path_buf);
        self
    }

    fn json(&self) -> Value {
        json!({
            "kind": self.kind.name(), "title": self.title,
            "filters": self.filters.iter().map(|(n, e)| json!({"name": n, "extensions": e})).collect::<Vec<_>>(),
            "directory": self.dir.as_ref().map(|d| d.display().to_string()),
            "file_name": self.file_name,
        })
    }

    /// Passt eine Datei zu den Filtern?
    fn matches(&self, p: &Path) -> bool {
        if self.filters.is_empty() {
            return true;
        }
        let n = p.to_string_lossy().to_lowercase();
        self.filters.iter().any(|(_, exts)| exts.iter().any(|e| e == "*" || n.ends_with(&format!(".{}", e.to_lowercase()))))
    }
}

/// Fortsetzung mit dem Ergebnis (leer = abgebrochen).
pub type Done = Box<dyn FnOnce(&mut AppState, Vec<PathBuf>) + Send>;

enum Mode {
    /// Nativer Dialog im eigenen Thread.
    Native(Receiver<Vec<PathBuf>>),
    /// Dialog in der App (Brückenmodus): Pfad-Eingabe.
    Internal { path: String },
}

struct Pending {
    id: u64,
    req: Request,
    mode: Mode,
    done: Option<Done>,
    /// Antwort von außen (MCP `ui_file_dialog`) oder aus dem Dialog in der App.
    answer: Option<(Vec<PathBuf>, &'static str)>,
    started: Instant,
}

struct Inner {
    pending: Option<Pending>,
    next_id: u64,
    log: Vec<Value>,
    parent: Option<Arc<winit::window::Window>>,
    /// egui-Kontext (aus [rame]) für Neuzeichnen aus dem Dialog-Thread.
    ctx: Option<egui::Context>,
}

static STATE: Mutex<Inner> = Mutex::new(Inner { pending: None, next_id: 1, log: Vec::new(), parent: None, ctx: None });

fn state() -> std::sync::MutexGuard<'static, Inner> {
    STATE.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Hauptfenster als Besitzer der nativen Dialoge (einmal beim Start).
pub fn set_parent_window(window: Arc<winit::window::Window>) {
    state().parent = Some(window);
}

/// Ist gerade ein Datei-Dialog offen?
pub fn is_open() -> bool {
    STATE.try_lock().map(|g| g.pending.is_some()).unwrap_or(true)
}

fn log_event(g: &mut Inner, ev: Value) {
    log::info!("Dateidialog: {ev}");
    g.log.push(ev);
    let n = g.log.len();
    if n > 50 {
        g.log.drain(..n - 50);
    }
}

/// Testmodus aus `FP_FILE_DIALOG` (alt `FP_DATEIDIALOG`).
enum EnvMode {
    Cancel,
    Path(PathBuf),
    Native,
    Normal,
}

/// Umgebungsvariable des Testmodus (technisch englisch, AGENTEN_REGELN §1c).
pub const ENV_FILE_DIALOG: &str = "FP_FILE_DIALOG";
/// Alter deutscher Name, wird weiter gelesen (wenn `FP_FILE_DIALOG` nicht gesetzt ist).
pub const LEGACY_ENV_FILE_DIALOG: &str = "FP_DATEIDIALOG";

fn env_mode() -> EnvMode {
    let v = std::env::var(ENV_FILE_DIALOG).or_else(|_| std::env::var(LEGACY_ENV_FILE_DIALOG)).unwrap_or_default();
    let t = v.trim();
    let lower = t.to_lowercase();
    if lower == "abbrechen" || lower == "cancel" {
        EnvMode::Cancel
    } else if let Some(p) = t.strip_prefix("pfad:").or_else(|| t.strip_prefix("path:")) {
        EnvMode::Path(PathBuf::from(p.trim()))
    } else if lower == "nativ" || lower == "native" {
        EnvMode::Native
    } else {
        EnvMode::Normal
    }
}

/// Antwort im Testmodus `path:<ordner>`.
fn auto_answer(req: &Request, dir: &Path) -> Vec<PathBuf> {
    match req.kind {
        Kind::Folder => vec![dir.to_path_buf()],
        Kind::Save => vec![dir.join(req.file_name.clone().unwrap_or_else(|| "Firepixel".into()))],
        Kind::Open | Kind::OpenMany => {
            let mut files: Vec<PathBuf> = std::fs::read_dir(dir).map(|r| r.flatten().map(|e| e.path()).filter(|p| p.is_file() && req.matches(p)).collect()).unwrap_or_default();
            files.sort();
            if req.kind == Kind::Open {
                files.truncate(1);
            }
            files
        }
    }
}

/// Öffnet einen Datei-Dialog. `done` läuft mit dem Ergebnis – sofort (Testvorgaben, Testmodus)
/// oder in einem späteren Frame, wenn der Nutzer bzw. Agent geantwortet hat.
pub fn request(st: &mut AppState, req: Request, done: impl FnOnce(&mut AppState, Vec<PathBuf>) + Send + 'static) {
    // Testvorgaben der Oberfläche (unverändert wie bisher)
    if let Some(o) = &st.file_override {
        let r: Vec<PathBuf> = match req.kind {
            Kind::Save => o.save.clone().into_iter().collect(),
            Kind::Folder => o.save.as_ref().and_then(|p| p.parent().map(Path::to_path_buf)).or_else(|| o.open.first().and_then(|p| p.parent().map(Path::to_path_buf))).into_iter().collect(),
            Kind::OpenMany => o.open.clone(),
            Kind::Open => o.open.iter().find(|p| req.matches(p)).or(o.open.first()).cloned().into_iter().collect(),
        };
        done(st, r);
        return;
    }
    let mode = env_mode();
    {
        let mut g = state();
        if g.pending.is_some() {
            // nur ein Dialog zur Zeit (wie modal): der zweite wird abgebrochen
            log_event(&mut g, json!({"event": "rejected", "reason": "busy", "request": req.json()}));
            drop(g);
            done(st, Vec::new());
            return;
        }
        match &mode {
            EnvMode::Cancel => {
                log_event(&mut g, json!({"event": "cancelled", "by": ENV_FILE_DIALOG, "request": req.json()}));
                drop(g);
                done(st, Vec::new());
                return;
            }
            EnvMode::Path(dir) => {
                let r = auto_answer(&req, dir);
                log_event(&mut g, json!({"event": if r.is_empty() { "cancelled" } else { "chosen" }, "by": ENV_FILE_DIALOG, "request": req.json(), "paths": r.iter().map(|p| p.display().to_string()).collect::<Vec<_>>()}));
                drop(g);
                done(st, r);
                return;
            }
            _ => {}
        }
        let internal = matches!(mode, EnvMode::Normal) && crate::bridge::running_url().is_some();
        let id = g.next_id;
        g.next_id += 1;
        let m = if internal {
            let path = match req.kind {
                Kind::Save => req.dir.clone().unwrap_or_default().join(req.file_name.clone().unwrap_or_default()).display().to_string(),
                _ => req.dir.as_ref().map(|d| d.display().to_string()).unwrap_or_default(),
            };
            Mode::Internal { path }
        } else {
            Mode::Native(spawn_native(&req, g.parent.clone(), g.ctx.clone()))
        };
        log_event(&mut g, json!({"event": "opened", "id": id, "mode": if internal { "internal" } else { "native" }, "request": req.json()}));
        g.pending = Some(Pending { id, req, mode: m, done: Some(Box::new(done)), answer: None, started: Instant::now() });
        if let Some(c) = &g.ctx {
            c.request_repaint();
        }
    }
}

/// Startet den nativen Dialog in einem eigenen Thread (Besitzer = Hauptfenster).
fn spawn_native(req: &Request, parent: Option<Arc<winit::window::Window>>, ctx: Option<egui::Context>) -> Receiver<Vec<PathBuf>> {
    let mut d = rfd::FileDialog::new().set_title(&req.title);
    for (n, e) in &req.filters {
        let exts: Vec<&str> = e.iter().map(String::as_str).collect();
        d = d.add_filter(n, &exts);
    }
    if let Some(dir) = &req.dir {
        d = d.set_directory(dir);
    }
    if let Some(n) = &req.file_name {
        d = d.set_file_name(n);
    }
    if let Some(w) = parent.as_deref() {
        d = d.set_parent(w);
    }
    let kind = req.kind;
    let (tx, rx) = std::sync::mpsc::channel();
    let spawned = std::thread::Builder::new().name("Firepixel-Dateidialog".into()).spawn(move || {
        let r: Vec<PathBuf> = match kind {
            Kind::Open => d.pick_file().into_iter().collect(),
            Kind::OpenMany => d.pick_files().unwrap_or_default(),
            Kind::Save => d.save_file().into_iter().collect(),
            Kind::Folder => d.pick_folder().into_iter().collect(),
        };
        let _ = tx.send(r);
        if let Some(c) = ctx {
            c.request_repaint();
        }
    });
    if spawned.is_err() {
        // ohne Thread: als abgebrochen melden (Kanal schließt, `frame` räumt auf)
        log::warn!("Dateidialog: Thread nicht gestartet");
    }
    rx
}

/// Je Frame (aus [`crate::bridge::frame`]): Ergebnis abholen, Fortsetzung ausführen, modale Sperre
/// bzw. den Dialog in der App zeigen.
pub fn frame(ctx: &egui::Context, st: &mut AppState) {
    let finished: Option<(Done, Vec<PathBuf>)> = {
        let mut g = state();
        g.ctx = Some(ctx.clone());
        let Some(p) = g.pending.as_mut() else { return };
        let mut result: Option<(Vec<PathBuf>, &'static str)> = p.answer.take();
        if result.is_none()
            && let Mode::Native(rx) = &p.mode
        {
            match rx.try_recv() {
                Ok(r) => result = Some((r, "user")),
                Err(TryRecvError::Disconnected) => result = Some((Vec::new(), "error")),
                Err(TryRecvError::Empty) => {}
            }
        }
        match result {
            Some((paths, by)) => {
                let p = g.pending.take();
                match p {
                    Some(mut p) => {
                        let ev = json!({"event": if paths.is_empty() { "cancelled" } else { "chosen" }, "by": by, "id": p.id, "request": p.req.json(), "paths": paths.iter().map(|x| x.display().to_string()).collect::<Vec<_>>(), "seconds": p.started.elapsed().as_secs_f64()});
                        log_event(&mut g, ev);
                        p.done.take().map(|d| (d, paths))
                    }
                    None => None,
                }
            }
            None => None,
        }
    };
    if let Some((done, paths)) = finished {
        done(st, paths);
        ctx.request_repaint();
        return;
    }
    show_modal(ctx);
}

/// Modale Sperre bzw. Dialog in der App.
fn show_modal(ctx: &egui::Context) {
    let mut g = state();
    let Some(p) = g.pending.as_mut() else { return };
    let title = p.req.title.clone();
    let kind = p.req.kind;
    let filters: Vec<String> = p.req.filters.iter().map(|(n, _)| n.clone()).collect();
    let mut answer: Option<Vec<PathBuf>> = None;
    match &mut p.mode {
        Mode::Native(_) => {
            egui::Modal::new(egui::Id::new("fp-file-dialog-wait")).show(ctx, |ui| {
                ui.label(fp_i18n::tr!("fdlg-waiting", title = title.clone()));
            });
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        Mode::Internal { path } => {
            egui::Modal::new(egui::Id::new("fp-file-dialog-internal")).show(ctx, |ui| {
                ui.heading(&title);
                ui.label(fp_i18n::t("fdlg-internal-hint"));
                if !filters.is_empty() {
                    ui.label(filters.join(" · "));
                }
                let r = ui.add(egui::TextEdit::singleline(path).desired_width(480.0));
                ui.horizontal(|ui| {
                    let ok = ui.button(fp_i18n::t("btn-ok")).clicked() || (r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
                    if ok && !path.trim().is_empty() {
                        answer = Some(split_paths(kind, path));
                    }
                    if ui.button(fp_i18n::t("btn-cancel")).clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        answer = Some(Vec::new());
                    }
                });
            });
        }
    }
    if let Some(a) = answer {
        p.answer = Some((a, "user"));
        ctx.request_repaint();
    }
}

fn split_paths(kind: Kind, text: &str) -> Vec<PathBuf> {
    if kind == Kind::OpenMany {
        text.split(['\n', ';']).map(str::trim).filter(|s| !s.is_empty()).map(PathBuf::from).collect()
    } else {
        vec![PathBuf::from(text.trim())]
    }
}

/// Zustand für `ui_state`/`ui_file_dialog status`.
pub fn status_json() -> Value {
    let g = state();
    let open = g.pending.as_ref().map(|p| {
        let mut v = p.req.json();
        v["id"] = json!(p.id);
        v["mode"] = json!(match p.mode {
            Mode::Native(_) => "native",
            Mode::Internal { .. } => "internal",
        });
        v["seconds_open"] = json!((p.started.elapsed().as_secs_f64() * 10.0).round() / 10.0);
        v
    });
    json!({"open": open, "last_events": g.log.iter().rev().take(5).cloned().collect::<Vec<_>>()})
}

/// Beantwortet den offenen Dialog von außen (`ui_file_dialog`): `paths` leer = abbrechen.
/// Ein nativer Dialog bleibt dabei sichtbar, bis der Nutzer ihn schließt; sein Ergebnis wird dann
/// verworfen. Liefert eine Fehlermeldung, wenn keiner offen ist.
pub fn answer(paths: Vec<PathBuf>) -> Result<Value, String> {
    let mut g = state();
    let Some(p) = g.pending.as_mut() else { return Err(fp_i18n::t("fdlg-none-open")) };
    let native = matches!(p.mode, Mode::Native(_));
    if p.req.kind == Kind::Open && paths.len() > 1 {
        return Err(fp_i18n::t("fdlg-only-one"));
    }
    p.answer = Some((paths, "mcp"));
    let id = p.id;
    if native {
        // Ergebnis des nativen Dialogs wird verworfen, sobald er schließt
        p.mode = Mode::Internal { path: String::new() };
    }
    Ok(json!({"ok": true, "id": id, "native_window_still_open": native}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn testmodus_pfad_beantwortet_nach_art() {
        let dir = std::env::temp_dir().join(format!("fp-dateidialog-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(dir.join("b.png"), b"x");
        let _ = std::fs::write(dir.join("a.png"), b"x");
        let _ = std::fs::write(dir.join("c.txt"), b"x");
        let req = Request::open("t").filter("PNG", &["png"]);
        assert_eq!(auto_answer(&req, &dir), vec![dir.join("a.png")]);
        let req = Request::open_many("t").filter("PNG", &["png"]);
        assert_eq!(auto_answer(&req, &dir).len(), 2);
        assert_eq!(auto_answer(&Request::save("t", "x.fw.png"), &dir), vec![dir.join("x.fw.png")]);
        assert_eq!(auto_answer(&Request::folder("t"), &dir), vec![dir.clone()]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn mehrere_pfade_nur_bei_mehrfachauswahl() {
        assert_eq!(split_paths(Kind::OpenMany, "a.png; b.png\nc.png").len(), 3);
        assert_eq!(split_paths(Kind::Open, " a;b.png ").len(), 1);
    }
}
