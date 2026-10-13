//! Datei-Dialoge ohne Blockieren der Oberfläche.
//!
//! Früher liefen die nativen Dialoge (rfd, Windows Common Item Dialog) direkt im UI-Thread: solange
//! einer offen war, zeichnete Firepixel nicht weiter und die MCP-Brücke bekam keine Frames – ein
//! Agent kam nicht weiter. Jetzt:
//!
//! - Ein Aufrufer beschreibt den Dialog ([`Request`]) und gibt eine Fortsetzung mit, die das
//!   Ergebnis bekommt (leere Liste = abgebrochen). [`request`] kehrt sofort zurück.
//! - Der native Dialog läuft in einem eigenen Thread, mit dem Hauptfenster als Besitzer
//!   (`set_parent`): er erscheint über Firepixel (Windows legt ihn an dessen linke obere Ecke), modal zum Hauptfenster und ohne eigenen
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
//! - „Speichern unter“ mit Zusatzfeldern ([`SaveExtras`], EB-2): der Dialog in der App zeigt Typliste,
//!   Hinweis, „Optionen…“ und „Als Kopie speichern“; nach dem Windows-Dialog (der weder Zusatzfelder
//!   noch den gewählten Typ liefert, rfd ohne `unsafe`) fragt ein Folgedialog nach, wenn der Name zu
//!   mehreren Typen passt. Ergebnis über [`request_save`] als [`SaveChoice`].

use crate::app_state::AppState;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::{Arc, Mutex, Weak};
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
    /// Vorgewählter Filter (Index in `filters`; Original: „Alle lesbaren Dateien“ an 13. Stelle).
    pub default_filter: usize,
    /// Zusatzfelder von „Speichern unter“ (Typliste, „Als Kopie speichern“, „Optionen…“, Hinweis).
    pub save_extras: Option<SaveExtras>,
    /// Zusatzfelder von „Öffnen“ („Als ‚Unbenannt‘ öffnen“, „Als Animation öffnen“); nur im Dialog
    /// in der App (der Windows-Dialog kann ohne `unsafe` keine Zusatzfelder zeigen).
    pub open_extras: bool,
}

/// Kästchen „Als Kopie speichern“ je Dateityp (re\20 §1.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CopyBox {
    /// wählbar, Vorgabe aus (FW-PNG, Reduziertes PNG)
    Off,
    /// an und gesperrt (Typen, die immer eine Kopie schreiben)
    Locked,
}

/// Zusatzfelder eines Dateityps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TypeExtras {
    pub copy: CopyBox,
    /// „Optionen…“ aktiv
    pub options: bool,
    /// Hinweis „… auf die Hintergrundebene reduziert“ sichtbar
    pub hint: bool,
    /// Kästchen „.fw.png anhängen“ sichtbar (FW-PNG; Vorgabe an)
    pub append_fw: bool,
}

/// Zusatzfelder von „Speichern unter“. Der Windows-Dialog kann ohne `unsafe` weder Zusatzfelder
/// zeigen noch den gewählten Dateityp melden (rfd); darum zeigt der Dialog in der App (Brücke) die
/// Felder selbst, und nach dem Windows-Dialog fragt ein kleiner Folgedialog nach, wenn der Name zu
/// mehreren Typen passt (`.png`: FW-PNG oder Reduziertes PNG).
#[derive(Clone, Debug)]
pub struct SaveExtras {
    /// je Filter (gleiche Reihenfolge wie `Request::filters`)
    pub types: Vec<TypeExtras>,
    /// Filter, zu denen ein gewählter Name passt, wenn der Typ nicht eindeutig ist (sonst leer).
    pub ambiguous: fn(&Path) -> Vec<usize>,
    /// „Optionen…“ geklickt (Filterindex): öffnet den Optionsdialog des Typs.
    pub on_options: fn(&mut AppState, usize),
    /// Ist der Optionsdialog offen? Solange ist der Datei-Dialog ausgeblendet; danach (OK oder
    /// Abbrechen) erscheint er wieder.
    pub options_open: fn(&AppState) -> bool,
}

/// Ergebnis von „Speichern unter“ mit Zusatzfeldern.
#[derive(Clone, Debug, PartialEq)]
pub struct SaveChoice {
    pub path: PathBuf,
    /// gewählter Filter, wenn bekannt (Dialog in der App, Folgedialog, Agent); `None` = nur der Name
    pub filter: Option<usize>,
    /// „Als Kopie speichern“
    pub copy: bool,
    /// „.fw.png anhängen“ (Vorgabe an; aus nur im Dialog in der App oder per Agent)
    pub append_fw_png: bool,
}

/// Wahl der Zusatzfelder von „Öffnen“ (Dialog in der App bzw. Agent; sonst beide aus).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OpenChoice {
    /// „Als ‚Unbenannt‘ öffnen“
    pub untitled: bool,
    /// „Als Animation öffnen“ (nur bei mehreren Dateien wirksam, sonst grau)
    pub animation: bool,
}

/// Zustand aller Kästchen eines Dialogs (an die Fortsetzung übergeben).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Flags {
    pub copy: bool,
    pub append_fw_png: bool,
    pub untitled: bool,
    pub animation: bool,
}

impl Default for Flags {
    fn default() -> Flags {
        Flags { copy: false, append_fw_png: true, untitled: false, animation: false }
    }
}

impl Request {
    pub fn new(kind: Kind, title: impl Into<String>) -> Request {
        Request { kind, title: title.into(), filters: Vec::new(), dir: None, file_name: None, default_filter: 0, save_extras: None, open_extras: false }
    }
    /// Zusatzfelder von „Öffnen“ („Als ‚Unbenannt‘ öffnen“, „Als Animation öffnen“).
    pub fn open_extras(mut self) -> Request {
        self.open_extras = true;
        self
    }
    /// Zusatzfelder von „Speichern unter“.
    pub fn save_extras(mut self, extras: SaveExtras) -> Request {
        self.save_extras = Some(extras);
        self
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
    /// Vorgewählter Filter (Index in `filters`).
    pub fn default_filter(mut self, index: usize) -> Request {
        self.default_filter = index;
        self
    }

    /// Filter in der Reihenfolge des nativen Dialogs: rfd kann keinen Filter vorwählen und nimmt
    /// den ersten, darum steht der vorgewählte dort vorn, die übrigen in ihrer Reihenfolge.
    pub fn native_filters(&self) -> Vec<(String, Vec<String>)> {
        let mut f = self.filters.clone();
        if self.default_filter > 0 && self.default_filter < f.len() {
            let d = f.remove(self.default_filter);
            f.insert(0, d);
        }
        f
    }

    fn json(&self) -> Value {
        json!({
            "kind": self.kind.name(), "title": self.title,
            "filters": self.filters.iter().map(|(n, e)| json!({"name": n, "extensions": e})).collect::<Vec<_>>(),
            "directory": self.dir.as_ref().map(|d| d.display().to_string()),
            "file_name": self.file_name,
            "default_filter": self.default_filter,
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

/// Fortsetzung mit Ergebnis, gewähltem Filter (falls bekannt) und den Kästchen.
type DoneEx = Box<dyn FnOnce(&mut AppState, Vec<PathBuf>, Option<usize>, Flags) + Send>;

enum Mode {
    /// Nativer Dialog im eigenen Thread.
    Native(Receiver<Vec<PathBuf>>),
    /// Dialog in der App (Brückenmodus): Pfad-Eingabe.
    Internal { path: String },
    /// Folgedialog nach dem Windows-Dialog: Dateityp unter `choices` wählen (Name passt zu mehreren).
    Followup { path: PathBuf, choices: Vec<usize> },
}

/// Antwort: Pfade, gewählter Filter, Kästchen, von wem.
type Answer = (Vec<PathBuf>, Option<usize>, Flags, &'static str);

struct Pending {
    id: u64,
    req: Request,
    mode: Mode,
    done: Option<DoneEx>,
    /// Antwort von außen (MCP `ui_file_dialog`) oder aus dem Dialog in der App.
    answer: Option<Answer>,
    started: Instant,
    /// gewählter Dateityp (Typliste im Dialog in der App bzw. Folgedialog)
    filter: usize,
    /// Kästchen „Als Kopie speichern“
    copy: bool,
    /// Kästchen „.fw.png anhängen“, „Als ‚Unbenannt‘ öffnen“, „Als Animation öffnen“
    append_fw: bool,
    untitled: bool,
    animation: bool,
    /// „Optionen…“ geklickt (wird im nächsten Schritt mit `AppState` ausgeführt)
    options_clicked: bool,
}

impl Pending {
    fn extras(&self) -> Option<TypeExtras> {
        self.req.save_extras.as_ref().and_then(|x| x.types.get(self.filter).copied())
    }
    /// Wert des Kästchens (gesperrt = immer an).
    fn copy_value(&self) -> bool {
        match self.extras().map(|e| e.copy) {
            Some(CopyBox::Locked) => true,
            _ => self.copy,
        }
    }
    /// Kästchen für die Fortsetzung („.fw.png anhängen“ nur bei FW-PNG wirksam, sonst an).
    fn flags(&self) -> Flags {
        let append = self.append_fw || !self.extras().is_some_and(|e| e.append_fw);
        Flags { copy: self.copy_value(), append_fw_png: append, untitled: self.req.open_extras && self.untitled, animation: self.req.open_extras && self.animation }
    }
}

struct Inner {
    pending: Option<Pending>,
    next_id: u64,
    log: Vec<Value>,
    /// Nur schwach gehalten: ein starker Verweis hielte das Fenster nach dem Schließen am Leben;
    /// eframe beendet die Ereignisschleife erst, wenn das Fenster zerstört ist (WindowEvent::
    /// Destroyed), sonst lief Firepixel nach WM_CLOSE von außen weiter (P-HG1).
    parent: Option<Weak<winit::window::Window>>,
    /// egui-Kontext (aus [rame]) für Neuzeichnen aus dem Dialog-Thread.
    ctx: Option<egui::Context>,
}

static STATE: Mutex<Inner> = Mutex::new(Inner { pending: None, next_id: 1, log: Vec::new(), parent: None, ctx: None });

fn state() -> std::sync::MutexGuard<'static, Inner> {
    STATE.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Hauptfenster als Besitzer der nativen Dialoge (einmal beim Start).
pub fn set_parent_window(window: Arc<winit::window::Window>) {
    state().parent = Some(Arc::downgrade(&window));
}

/// Ist gerade ein Datei-Dialog offen?
///
/// Wartet kurz auf die Sperre statt bei Gleichzeitigkeit „offen“ zu melden (NZ-2): `frame` hält sie
/// in jedem Bild, und mit `try_lock` sah jede andere App-Instanz im selben Prozess (parallele
/// Oberflächentests) für dieses Bild einen offenen Dialog. Dann sperrte `DialogState::any_open`
/// alle Tastenkürzel und die Leinwand schluckte Strg+C/V, Text und Esc
/// (`canvas::tests::shortcuts_respect_focus_text_editor_and_dialogs` scheiterte gelegentlich).
/// Unter der Sperre wird nie `is_open` aufgerufen (Fortsetzungen laufen nach dem Freigeben).
pub fn is_open() -> bool {
    state().pending.is_some()
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

/// Testmodus ohne Umgebungsvariable (Tests im selben Prozess; `std::env::set_var` ist dort
/// `unsafe`): gleiche Werte wie `FP_FILE_DIALOG`, `None` = wieder die Umgebungsvariable.
static TEST_MODE: Mutex<Option<String>> = Mutex::new(None);

/// Setzt den Testmodus (siehe [`ENV_FILE_DIALOG`]) für diesen Prozess.
pub fn set_test_mode(mode: Option<&str>) {
    *TEST_MODE.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = mode.map(str::to_string);
}

fn env_mode() -> EnvMode {
    let forced = TEST_MODE.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone();
    let v = forced.unwrap_or_else(|| std::env::var(ENV_FILE_DIALOG).or_else(|_| std::env::var(LEGACY_ENV_FILE_DIALOG)).unwrap_or_default());
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
    request_ex(st, req, Box::new(move |st, r, _, _| done(st, r)));
}

/// „Speichern unter“ mit Zusatzfeldern ([`Request::save_extras`]): `done` bekommt die Wahl oder
/// `None` (abgebrochen). Testvorgaben und Testmodus liefern nur den Namen (`filter: None`).
pub fn request_save(st: &mut AppState, req: Request, done: impl FnOnce(&mut AppState, Option<SaveChoice>) + Send + 'static) {
    request_ex(
        st,
        req,
        Box::new(move |st, r, filter, f: Flags| {
            let choice = r.into_iter().next().map(|path| SaveChoice { path, filter, copy: f.copy, append_fw_png: f.append_fw_png });
            done(st, choice)
        }),
    );
}

/// „Öffnen“ mit Zusatzfeldern ([`Request::open_extras`]): `done` bekommt die Dateien (leer =
/// abgebrochen) und die Kästchen (Testvorgaben, Testmodus und Windows-Dialog: beide aus).
pub fn request_open(st: &mut AppState, req: Request, done: impl FnOnce(&mut AppState, Vec<PathBuf>, OpenChoice) + Send + 'static) {
    request_ex(st, req.open_extras(), Box::new(move |st, r, _, f: Flags| done(st, r, OpenChoice { untitled: f.untitled, animation: f.animation })));
}

fn request_ex(st: &mut AppState, req: Request, done: DoneEx) {
    // Testvorgaben der Oberfläche (unverändert wie bisher)
    if let Some(o) = &st.file_override {
        let r: Vec<PathBuf> = match req.kind {
            Kind::Save => o.save.clone().into_iter().collect(),
            Kind::Folder => o.save.as_ref().and_then(|p| p.parent().map(Path::to_path_buf)).or_else(|| o.open.first().and_then(|p| p.parent().map(Path::to_path_buf))).into_iter().collect(),
            Kind::OpenMany => o.open.clone(),
            Kind::Open => o.open.iter().find(|p| req.matches(p)).or(o.open.first()).cloned().into_iter().collect(),
        };
        done(st, r, None, Flags::default());
        return;
    }
    let mode = env_mode();
    {
        let mut g = state();
        if g.pending.is_some() {
            // nur ein Dialog zur Zeit (wie modal): der zweite wird abgebrochen
            log_event(&mut g, json!({"event": "rejected", "reason": "busy", "request": req.json()}));
            drop(g);
            done(st, Vec::new(), None, Flags::default());
            return;
        }
        match &mode {
            EnvMode::Cancel => {
                log_event(&mut g, json!({"event": "cancelled", "by": ENV_FILE_DIALOG, "request": req.json()}));
                drop(g);
                done(st, Vec::new(), None, Flags::default());
                return;
            }
            EnvMode::Path(dir) => {
                let r = auto_answer(&req, dir);
                log_event(&mut g, json!({"event": if r.is_empty() { "cancelled" } else { "chosen" }, "by": ENV_FILE_DIALOG, "request": req.json(), "paths": r.iter().map(|p| p.display().to_string()).collect::<Vec<_>>()}));
                drop(g);
                done(st, r, None, Flags::default());
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
            Mode::Native(spawn_native(&req, g.parent.as_ref().and_then(Weak::upgrade), g.ctx.clone()))
        };
        log_event(&mut g, json!({"event": "opened", "id": id, "mode": if internal { "internal" } else { "native" }, "request": req.json()}));
        let filter = req.default_filter;
        g.pending = Some(Pending { id, req, mode: m, done: Some(done), answer: None, started: Instant::now(), filter, copy: false, append_fw: true, untitled: false, animation: false, options_clicked: false });
        if let Some(c) = &g.ctx {
            c.request_repaint();
        }
    }
}

/// Startet den nativen Dialog in einem eigenen Thread (Besitzer = Hauptfenster).
fn spawn_native(req: &Request, parent: Option<Arc<winit::window::Window>>, ctx: Option<egui::Context>) -> Receiver<Vec<PathBuf>> {
    let mut d = rfd::FileDialog::new().set_title(&req.title);
    for (n, e) in &req.native_filters() {
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
    type Finished = (DoneEx, Vec<PathBuf>, Option<usize>, Flags);
    let finished: Option<Finished> = {
        let mut g = state();
        g.ctx = Some(ctx.clone());
        let Some(p) = g.pending.as_mut() else { return };
        let mut result: Option<Answer> = p.answer.take();
        if result.is_none()
            && let Mode::Native(rx) = &p.mode
        {
            match rx.try_recv() {
                Ok(r) => result = Some((r, None, Flags::default(), "user")),
                Err(TryRecvError::Disconnected) => result = Some((Vec::new(), None, Flags::default(), "error")),
                Err(TryRecvError::Empty) => {}
            }
            // Windows-Dialog meldet den Dateityp nicht: passt der Name zu mehreren Typen, fragt ein
            // Folgedialog nach (Typ, „Als Kopie speichern“, „Optionen…“)
            if let Some((paths, None, _, "user")) = &result
                && let (Some(path), Some(x)) = (paths.first(), p.req.save_extras.as_ref())
            {
                let choices = (x.ambiguous)(path);
                if choices.len() > 1 {
                    let path = path.clone();
                    p.filter = choices[0];
                    p.copy = false;
                    p.mode = Mode::Followup { path: path.clone(), choices };
                    result = None;
                    let id = p.id;
                    log_event(&mut g, json!({"event": "followup", "id": id, "path": path.display().to_string()}));
                }
            }
        }
        match result {
            Some((paths, filter, copy, by)) => match g.pending.take() {
                Some(mut p) => {
                    let ev = json!({"event": if paths.is_empty() { "cancelled" } else { "chosen" }, "by": by, "id": p.id, "request": p.req.json(), "paths": paths.iter().map(|x| x.display().to_string()).collect::<Vec<_>>(), "filter": filter, "copy": copy.copy, "append_fw_png": copy.append_fw_png, "untitled": copy.untitled, "animation": copy.animation, "seconds": p.started.elapsed().as_secs_f64()});
                    log_event(&mut g, ev);
                    p.done.take().map(|d| (d, paths, filter, copy))
                }
                None => None,
            },
            None => None,
        }
    };
    if let Some((done, paths, filter, copy)) = finished {
        done(st, paths, filter, copy);
        ctx.request_repaint();
        return;
    }
    // „Optionen…“: Optionsdialog öffnen; solange er offen ist, bleibt der Datei-Dialog verborgen
    let (options, extras) = {
        let mut g = state();
        match g.pending.as_mut() {
            Some(p) => (std::mem::take(&mut p.options_clicked).then_some(p.filter), p.req.save_extras.clone()),
            None => (None, None),
        }
    };
    if let (Some(i), Some(x)) = (options, &extras) {
        (x.on_options)(st, i);
        ctx.request_repaint();
    }
    if extras.as_ref().is_some_and(|x| (x.options_open)(st)) {
        return;
    }
    show_modal(ctx);
}

/// Zusatzfelder eines Dateityps unter der Typliste wie im Original: Hinweis (Symbol + Text),
/// „Optionen…“ und „Als Kopie speichern“. Gibt `true` zurück, wenn „Optionen…“ geklickt wurde.
pub fn save_extras_ui(ui: &mut egui::Ui, extras: TypeExtras, copy: &mut bool) -> bool {
    save_extras_ui_ex(ui, extras, copy, &mut true)
}

/// Wie [`save_extras_ui`], dazu „.fw.png anhängen“ (nur bei FW-PNG sichtbar).
pub fn save_extras_ui_ex(ui: &mut egui::Ui, extras: TypeExtras, copy: &mut bool, append_fw: &mut bool) -> bool {
    if extras.hint {
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("ⓘ").strong());
            ui.label(fp_i18n::t("fdlg-flatten-hint"));
        });
    }
    let mut clicked = false;
    if extras.append_fw {
        ui.checkbox(append_fw, fp_i18n::t("fdlg-append-fw-png"));
    }
    ui.horizontal(|ui| {
        clicked = ui.add_enabled(extras.options, egui::Button::new(fp_i18n::t("fdlg-options"))).clicked();
        match extras.copy {
            CopyBox::Off => {
                crate::theme::with_field_widgets(ui, |ui| ui.checkbox(copy, fp_i18n::t("fdlg-save-as-copy")));
            }
            CopyBox::Locked => {
                let mut on = true;
                crate::theme::with_field_widgets(ui, |ui| ui.add_enabled(false, egui::Checkbox::new(&mut on, fp_i18n::t("fdlg-save-as-copy"))));
            }
        }
    });
    clicked
}

/// Typliste und Zusatzfelder (Dialog in der App bzw. Folgedialog); `choices` = wählbare Filter.
fn save_type_ui(ui: &mut egui::Ui, filters: &[String], choices: &[usize], x: &SaveExtras, filter: &mut usize, copy: &mut bool, append_fw: &mut bool) -> bool {
    ui.horizontal(|ui| {
        ui.label(fp_i18n::t("fdlg-file-type"));
        let current = filters.get(*filter).cloned().unwrap_or_default();
        egui::ComboBox::from_id_salt("fp-file-dialog-type").selected_text(current).width(260.0).show_ui(ui, |ui| {
            for &i in choices {
                if let Some(n) = filters.get(i) {
                    ui.selectable_value(filter, i, n);
                }
            }
        });
    });
    match x.types.get(*filter) {
        Some(t) => save_extras_ui_ex(ui, *t, copy, append_fw),
        None => false,
    }
}

/// Modale Sperre bzw. Dialog in der App.
fn show_modal(ctx: &egui::Context) {
    let mut g = state();
    let Some(p) = g.pending.as_mut() else { return };
    let title = p.req.title.clone();
    let kind = p.req.kind;
    let filters: Vec<String> = p.req.filters.iter().map(|(n, _)| n.clone()).collect();
    let extras = p.req.save_extras.clone();
    let (mut filter, mut copy, mut options) = (p.filter, p.copy, false);
    let (mut append_fw, mut untitled, mut animation) = (p.append_fw, p.untitled, p.animation);
    let open_extras = p.req.open_extras;
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
                match &extras {
                    // Speichern unter: Typliste und Zusatzfelder
                    Some(x) => {
                        let all: Vec<usize> = (0..filters.len()).collect();
                        options = save_type_ui(ui, &filters, &all, x, &mut filter, &mut copy, &mut append_fw);
                    }
                    None if !filters.is_empty() => {
                        ui.label(filters.join(" · "));
                    }
                    None => {}
                }
                let r = ui.add(egui::TextEdit::singleline(path).desired_width(480.0));
                if open_extras {
                    // Original: „Als Animation öffnen“ grau, bis mehrere Dateien gewählt sind
                    let many = kind == Kind::OpenMany && split_paths(kind, path).len() > 1;
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut untitled, fp_i18n::t("fdlg-open-untitled"));
                        ui.add_enabled(many, egui::Checkbox::new(&mut animation, fp_i18n::t("fdlg-open-as-animation")));
                    });
                }
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
        Mode::Followup { path, choices } => {
            let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let path = path.clone();
            let choices = choices.clone();
            egui::Modal::new(egui::Id::new("fp-file-dialog-followup")).show(ctx, |ui| {
                ui.set_max_width(460.0);
                ui.heading(&title);
                ui.label(fp_i18n::tr!("fdlg-followup-question", name = name));
                if let Some(x) = &extras {
                    options = save_type_ui(ui, &filters, &choices, x, &mut filter, &mut copy, &mut append_fw);
                }
                ui.horizontal(|ui| {
                    if ui.button(fp_i18n::t("btn-ok")).clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        answer = Some(vec![path.clone()]);
                    }
                    if ui.button(fp_i18n::t("btn-cancel")).clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        answer = Some(Vec::new());
                    }
                });
            });
        }
    }
    p.filter = filter;
    p.copy = copy;
    p.append_fw = append_fw;
    p.untitled = untitled;
    p.animation = animation;
    if options {
        p.options_clicked = true;
        ctx.request_repaint();
    }
    if let Some(a) = answer {
        let with_type = p.req.save_extras.is_some() && !a.is_empty();
        let mut f = p.flags();
        f.copy &= with_type;
        f.append_fw_png |= !with_type;
        p.answer = Some((a, with_type.then_some(filter), f, "user"));
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
            Mode::Followup { .. } => "followup",
        });
        if let Mode::Followup { path, choices } = &p.mode {
            v["followup"] = json!({"path": path.display().to_string(), "choices": choices});
        }
        if p.req.save_extras.is_some() {
            v["filter"] = json!(p.filter);
            v["save_as_copy"] = json!(p.copy_value());
            v["append_fw_png"] = json!(p.flags().append_fw_png);
        }
        if p.req.open_extras {
            v["open_untitled"] = json!(p.untitled);
            v["open_as_animation"] = json!(p.animation);
        }
        v["seconds_open"] = json!((p.started.elapsed().as_secs_f64() * 10.0).round() / 10.0);
        v
    });
    json!({"open": open, "last_events": g.log.iter().rev().take(5).cloned().collect::<Vec<_>>()})
}

/// Beantwortet den offenen Dialog von außen (`ui_file_dialog`): `paths` leer = abbrechen.
/// Ein nativer Dialog bleibt dabei sichtbar, bis der Nutzer ihn schließt; sein Ergebnis wird dann
/// verworfen. Liefert eine Fehlermeldung, wenn keiner offen ist.
pub fn answer(paths: Vec<PathBuf>) -> Result<Value, String> {
    answer_with(paths, None, None)
}

/// Kästchen des offenen Dialogs von außen setzen (`ui_file_dialog`, Tests): „.fw.png anhängen“,
/// „Als ‚Unbenannt‘ öffnen“, „Als Animation öffnen“. Liefert eine Fehlermeldung, wenn keiner offen ist.
pub fn set_flags(append_fw_png: Option<bool>, untitled: Option<bool>, animation: Option<bool>) -> Result<(), String> {
    let mut g = state();
    let Some(p) = g.pending.as_mut() else { return Err(fp_i18n::t("fdlg-none-open")) };
    if let Some(v) = append_fw_png {
        p.append_fw = v;
    }
    if let Some(v) = untitled {
        p.untitled = v;
    }
    if let Some(v) = animation {
        p.animation = v;
    }
    Ok(())
}

/// Wie [`answer`], bei „Speichern unter“ mit Dateityp (`filter`, Index in den Filtern; ohne =
/// der im Dialog gewählte) und „Als Kopie speichern“ (ohne = das Kästchen im Dialog).
pub fn answer_with(paths: Vec<PathBuf>, filter: Option<usize>, copy: Option<bool>) -> Result<Value, String> {
    let mut g = state();
    let Some(p) = g.pending.as_mut() else { return Err(fp_i18n::t("fdlg-none-open")) };
    let native = matches!(p.mode, Mode::Native(_));
    if p.req.kind == Kind::Open && paths.len() > 1 {
        return Err(fp_i18n::t("fdlg-only-one"));
    }
    let (f, flags) = if p.req.save_extras.is_some() && !paths.is_empty() {
        if let Some(i) = filter.filter(|i| *i < p.req.filters.len()) {
            p.filter = i;
        }
        if let Some(c) = copy {
            p.copy = c;
        }
        (Some(p.filter), p.flags())
    } else {
        let mut fl = p.flags();
        fl.copy = false;
        fl.append_fw_png = true;
        (None, fl)
    };
    p.answer = Some((paths, f, flags, "mcp"));
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
    fn test_mode_path_answers_by_kind() {
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

    /// Ein anderer Thread hält die Sperre (wie `frame` einer zweiten App-Instanz): ohne offenen
    /// Dialog meldet `is_open` trotzdem „zu“ (vorher `try_lock` → „offen“).
    #[test]
    fn is_open_ignores_a_lock_held_by_another_instance() {
        let (tx, rx) = std::sync::mpsc::channel();
        let t = std::thread::spawn(move || {
            let g = state();
            let open = g.pending.is_some();
            tx.send(()).ok();
            std::thread::sleep(std::time::Duration::from_millis(150));
            drop(g);
            open
        });
        rx.recv().ok();
        let open_elsewhere = t.is_finished();
        let seen = is_open();
        let pending = t.join().unwrap_or(true);
        assert!(!open_elsewhere, "Sperre war während der Abfrage belegt");
        assert_eq!(seen, pending, "kein Dialog offen, nur die Sperre belegt");
    }

    #[test]
    fn multiple_paths_only_for_multi_select() {
        assert_eq!(split_paths(Kind::OpenMany, "a.png; b.png\nc.png").len(), 3);
        assert_eq!(split_paths(Kind::Open, " a;b.png ").len(), 1);
    }
}
