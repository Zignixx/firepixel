//! Zustand der Oberfläche: Sitzung (fp-engine), Design, Arbeitsbereich, Leinwand-Zustände, Dialoge
//! und die Aktions-Warteschlange.
//!
//! Widgets ändern den Zustand nicht direkt, sondern stellen [`Action`]s in die Warteschlange
//! ([`AppState::push`]); sie werden am Ende des Frames in [`AppState::process_actions`] ausgeführt.
//! So gibt es keine Konflikte mit geborgten Teilen des Zustands, und jede Aktion läuft über dieselbe
//! Stelle (Befehlsregister, Rückfragen, Dateidialoge).

use crate::canvas::render::{CanvasCache, RenderWorker};
use crate::dialogs::Dialogs;
use crate::panels::PanelId;
use crate::theme::{Theme, ThemeKind};
use crate::workspace::Workspace;
use fp_engine::{DocId, Session, SessionEvent, ToolId};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// Eine Aktion der Oberfläche.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    /// Befehl aus dem Register ausführen (mit Abfangen der Befehle, die Dialoge brauchen).
    Run(String, Value),
    /// Werkzeug wählen.
    SetTool(ToolId),
    /// Design umschalten.
    SetTheme(ThemeKind),
    /// Bedienfeld ein-/ausblenden (Fenster-Menü).
    TogglePanel(PanelId),
    /// Bedienfeld zeigen (und nach vorn holen).
    ShowPanel(PanelId),
    /// Dokument aktivieren (Reiter).
    Activate(DocId),
    /// Dokument schließen (mit Rückfrage bei ungespeicherten Änderungen).
    CloseDocument(DocId),
    /// Datei öffnen.
    OpenPath(PathBuf),
    /// Zoom des aktiven Dokuments setzen (1 = 100 %).
    SetZoom(f64),
    /// Seite wechseln.
    SetPage(usize),
    /// Status wechseln.
    SetFrame(usize),
    /// Strich-/Füllfarbe setzen.
    SetColors { stroke: Option<fp_color::Argb>, fill: Option<fp_color::Argb> },
    /// Arbeitsbereich auf den Standard zurücksetzen.
    ResetWorkspace,
    /// Benannten Arbeitsbereich laden.
    LoadWorkspace(String),
    /// Aktuellen Arbeitsbereich unter einem Namen speichern.
    SaveWorkspaceAs(String),
    /// Programm beenden (mit Rückfragen).
    Quit,
    /// Meldung in der Statuszeile.
    Status(String),
}

/// Einstellungen der Oberfläche, die eframe dauerhaft speichert.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct UiSettings {
    pub theme: ThemeKind,
    pub workspace: Workspace,
    /// Gespeicherte, benannte Arbeitsbereiche.
    pub named_workspaces: Vec<(String, Workspace)>,
}

impl Default for UiSettings {
    fn default() -> Self {
        UiSettings { theme: ThemeKind::Normal, workspace: Workspace::default(), named_workspaces: Vec::new() }
    }
}

/// Schlüssel im eframe-Speicher.
pub const SETTINGS_KEY: &str = "firepixel-ui";

/// Der gesamte Zustand.
pub struct AppState {
    pub session: Session,
    pub theme: Arc<Theme>,
    pub settings: UiSettings,
    pub canvases: HashMap<DocId, CanvasCache>,
    pub worker: RenderWorker,
    pub renderer: Arc<fp_render::Renderer>,
    pub dialogs: Dialogs,
    actions: Vec<Action>,
    /// Statuszeilen-Meldung mit Zeitpunkt.
    pub status: Option<(String, f64)>,
    /// Synchron rendern (Tests, Bildschirmfotos).
    pub sync_render: bool,
    /// Theme muss neu auf egui angewendet werden.
    pub theme_dirty: bool,
    /// Beenden bestätigt (alle Rückfragen erledigt).
    pub quit_confirmed: bool,
    /// Fenster schließen angefordert (an eframe weiterreichen).
    pub close_window: bool,
    /// Offener Flyout des Werkzeugbedienfelds (Flyout-Gruppe).
    pub flyout: Option<crate::tools_panel::FlyoutState>,
    /// Leertaste gedrückt (vorübergehend Hand).
    pub space_down: bool,
    /// Letzte Mausposition auf der Leinwand in Dokumentkoordinaten (Info, Lineale).
    pub pointer_doc: Option<fp_geom::Point>,
    /// Letzte Werkzeugantwort (Mauszeiger).
    pub tool_cursor: fp_engine::tools::Cursor,
    /// Version der Oberfläche (für Ereignisse zur Neuzeichnung).
    pub frame_counter: u64,
    /// Laufzeitzustand des Dockings (Reiter ziehen, Symbolmodus).
    pub dock: crate::docking::DockUi,
    /// Ersatz für die nativen Dateidialoge (Tests/Automatisierung): gesetzt = kein Dialog.
    pub file_override: Option<crate::files::FilePickOverride>,
    /// Systemzwischenablage für Bilder benutzen (Tests: aus).
    pub os_clipboard: bool,
    /// Abgelegte Dateien, deren Ablageort noch bestimmt wird (Drag & Drop).
    pub pending_drop: Option<crate::files::PendingDrop>,
    /// Batch-Studio (Reiter „Batch“, BATCH-2); `None` = geschlossen.
    pub batch: Option<Box<crate::batch::Studio>>,
}

impl AppState {
    /// Neuer Zustand mit Sitzung und Einstellungen.
    pub fn new(mut session: Session, settings: UiSettings, ctx: Option<egui::Context>) -> AppState {
        let renderer = Arc::new(fp_render::Renderer::default());
        session.set_renderer(Some(Arc::new(crate::canvas::render::EngineRenderer(Arc::clone(&renderer)))));
        // JSF-Laufzeit (Befehle > Skript ausführen, Befehle-Menü, Verlauf > Als Befehl speichern)
        if session.script_host().is_none() {
            fp_script::install_host(&mut session);
        }
        let theme = Arc::new(Theme::new(settings.theme));
        let worker = RenderWorker::new(Arc::clone(&renderer), ctx);
        let mut st = AppState {
            session,
            theme,
            settings,
            canvases: HashMap::new(),
            worker,
            renderer,
            dialogs: Dialogs::default(),
            actions: Vec::new(),
            status: None,
            sync_render: false,
            theme_dirty: true,
            quit_confirmed: false,
            close_window: false,
            flyout: None,
            space_down: false,
            pointer_doc: None,
            tool_cursor: fp_engine::tools::Cursor::Arrow,
            frame_counter: 0,
            dock: crate::docking::DockUi::default(),
            file_override: None,
            os_clipboard: true,
            pending_drop: None,
            batch: None,
        };
        st.sync_panels_to_engine();
        st
    }

    /// Aktion einreihen.
    pub fn push(&mut self, a: Action) {
        self.actions.push(a);
    }

    /// Befehl einreihen (Kurzform).
    pub fn run(&mut self, name: &str, params: Value) {
        self.actions.push(Action::Run(name.to_string(), params));
    }

    /// Meldung in der Statuszeile setzen.
    pub fn set_status(&mut self, msg: impl Into<String>, now: f64) {
        self.status = Some((msg.into(), now));
    }

    /// Design wechseln (wirkt sofort, wird gespeichert).
    pub fn set_theme(&mut self, kind: ThemeKind) {
        self.settings.theme = kind;
        self.theme = Arc::new(Theme::new(kind));
        self.theme_dirty = true;
        self.session.prefs.theme = kind.to_engine();
        self.session.prefs_changed();
    }

    /// Sichtbarkeit der Bedienfelder an fp-engine melden (Häkchen der Fenster-Befehle).
    pub fn sync_panels_to_engine(&mut self) {
        for p in PanelId::ALL {
            if let Some(cmd) = p.engine_command() {
                let vis = self.settings.workspace.is_visible(*p);
                self.session.ui.panels.insert(cmd.to_string(), vis);
            }
        }
        let ws = &self.settings.workspace;
        self.session.ui.panels.insert("window.toolbars.main".into(), ws.show_toolbar);
        self.session.ui.panels.insert("window.tools".into(), ws.show_tools);
        self.session.ui.panels.insert("window.properties".into(), ws.show_inspector);
    }

    /// Ereignisse der Sitzung verarbeiten (Leinwand-Caches ungültig machen usw.).
    pub fn handle_session_events(&mut self) {
        for e in self.session.drain_events() {
            match e {
                SessionEvent::DocumentClosed(id) => {
                    self.canvases.remove(&id);
                }
                SessionEvent::DocumentChanged { doc, .. } | SessionEvent::LocationChanged(doc) => {
                    if let Some(c) = self.canvases.get_mut(&doc) {
                        c.invalidate();
                    }
                }
                SessionEvent::PreferencesChanged => {
                    let k = ThemeKind::from_engine(self.session.prefs.theme);
                    if k != self.settings.theme {
                        self.settings.theme = k;
                        self.theme = Arc::new(Theme::new(k));
                        self.theme_dirty = true;
                    }
                }
                SessionEvent::QuitRequested => {
                    self.session.ui.quit_requested = false;
                    self.actions.push(Action::Quit);
                }
                SessionEvent::DocumentOpened(id) => {
                    self.canvases.entry(id).or_default().needs_initial_fit = true;
                }
                // Vorschau im Browser (auch aus „Demo – aktuelles Dokument“): Rückmeldung mit dem
                // Ordner, auch wenn kein Browser startet (CT-1b MN-4)
                SessionEvent::CommandExecuted(name) if name.starts_with("file.previewInBrowser.preview") => {
                    if let Some(id) = self.session.active_id() {
                        let dir = fp_engine::commands::export::preview_dir(id.0);
                        self.actions.push(Action::Status(fp_i18n::tr!("ui-preview-created", path = dir.to_string_lossy().to_string())));
                    }
                }
                _ => {}
            }
        }
    }

    /// Alle eingereihten Aktionen ausführen.
    pub fn process_actions(&mut self, ctx: &egui::Context) {
        let mut guard = 0;
        while !self.actions.is_empty() && guard < 64 {
            guard += 1;
            let batch = std::mem::take(&mut self.actions);
            for a in batch {
                crate::actions::execute(self, ctx, a);
            }
            self.handle_session_events();
        }
    }

    /// Titel des Hauptfensters wie im Original: nur der Dokumentname, mit Stern vorn bei
    /// ungespeicherten Änderungen („*Name.fw.png“, ohne Zoom); ohne Dokument „Firepixel“.
    pub fn window_title(&self) -> String {
        match self.session.active() {
            Some(d) => {
                let star = if d.is_dirty() { "*" } else { "" };
                let name = d.path.as_ref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| d.title.clone());
                format!("{star}{name}")
            }
            None => "Firepixel".to_string(),
        }
    }

    /// Hat irgendein Dokument ungespeicherte Änderungen?
    pub fn any_dirty(&self) -> bool {
        self.session.documents().iter().any(|d| d.is_dirty())
    }
}
