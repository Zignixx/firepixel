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
    /// A command that applies the pending value of a pop-up slider or colour pop-up (FB-46): it
    /// runs only while the selection is still `target`, the objects the pop-up was opened for, and
    /// is dropped when the same batch undoes or redoes (see [`AppState::push`]).
    Commit {
        target: Option<crate::inspector::preview::PreviewTarget>,
        name: String,
        params: Value,
    },
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
    SetColors {
        stroke: Option<fp_color::Argb>,
        fill: Option<fp_color::Argb>,
    },
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
        UiSettings {
            theme: ThemeKind::Normal,
            workspace: Workspace::default(),
            named_workspaces: Vec::new(),
        }
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
    /// FB-97 / TW-272: canvas caches of the panes of a split view whose cache is not in
    /// [`AppState::canvases`], by document and pane (swapped in while that pane is drawn,
    /// `canvas::split`; F137: up to three parked panes per document).
    pub pane_canvases: HashMap<(DocId, u64), CanvasCache>,
    /// FB-97: the pane (F137: its stable key, `DocumentState::pane_key`) of a split document
    /// whose cache is in [`AppState::canvases`] (missing = 0).
    pub cache_panes: HashMap<DocId, u64>,
    /// FB-97: the canvas being drawn is a pane without the focus (drawn, takes no input).
    pub pane_passive: bool,
    /// F137: key of the focused pane of the split view being drawn (valid while `pane_passive`).
    pub pane_focus_key: u64,
    /// F137 / TW-275: the pane of the active split document under the pointer (`None`: the
    /// pointer is over no pane, or the view is not split). The status bar and the Info panel
    /// follow it, else the focused pane.
    pub hover_pane: Option<usize>,
    /// F137: the pane [`AppState::pointer_doc`] was measured in (rulers mark it only there).
    pub pointer_pane: usize,
    pub worker: RenderWorker,
    pub renderer: Arc<fp_render::Renderer>,
    pub dialogs: Dialogs,
    actions: Vec<Action>,
    /// Statuszeilen-Meldung mit Zeitpunkt.
    pub status: Option<(String, f64)>,
    /// Synchron rendern (Tests, Bildschirmfotos).
    pub sync_render: bool,
    /// Live preview of a pending inspector slider value on the canvas (FB-09/FB-10); reset at the
    /// start of every frame, set by the inspector while a slider holds a pending value.
    pub inspector_preview: Option<crate::inspector::preview::InspectorPreview>,
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
    /// F28 (Q-50): last canvas presses and releases (tool, hit, modifiers) for the bridge
    /// (`ui_state.pointer_log`, newest last, at most `POINTER_LOG_MAX`).
    pub pointer_log: Vec<serde_json::Value>,
    /// Letzte Werkzeugantwort (Mauszeiger).
    pub tool_cursor: fp_engine::tools::Cursor,
    /// Where the canvas painted its own cursor in this frame (`None`: not painted, the system
    /// cursor is shown). Lets tests check that only one cursor is visible (FB-18, TW-199).
    pub painted_cursor: Option<egui::Pos2>,
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
    /// Importieren: gewählte Datei wartet auf den Platzierungszeiger (Klick setzt, Ziehen skaliert; CT-C-05).
    pub place_import: Option<crate::canvas::PlaceImport>,
    /// Animation playback on the canvas (FB-63).
    pub playback: crate::playback::Playback,
}

impl AppState {
    /// FB-97: the pane (key) of document `id` whose canvas cache is in [`AppState::canvases`].
    pub fn cache_pane(&self, id: DocId) -> u64 {
        self.cache_panes.get(&id).copied().unwrap_or(0)
    }

    /// FB-97: puts the canvas cache of `pane` of document `id` into [`AppState::canvases`] (the
    /// cache there goes to [`AppState::pane_canvases`] under its own pane).
    pub fn use_pane_cache(&mut self, id: DocId, pane: u64) {
        let cur = self.cache_pane(id);
        if cur == pane {
            return;
        }
        let a = self.canvases.remove(&id);
        if let Some(b) = self.pane_canvases.remove(&(id, pane)) {
            self.canvases.insert(id, b);
        }
        if let Some(a) = a {
            self.pane_canvases.insert((id, cur), a);
        }
        if pane == 0 {
            self.cache_panes.remove(&id);
        } else {
            self.cache_panes.insert(id, pane);
        }
    }

    /// Neuer Zustand mit Sitzung und Einstellungen.
    pub fn new(mut session: Session, settings: UiSettings, ctx: Option<egui::Context>) -> AppState {
        let renderer = Arc::new(fp_render::Renderer::default());
        session.set_renderer(Some(Arc::new(crate::canvas::render::EngineRenderer(
            Arc::clone(&renderer),
        ))));
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
            pane_canvases: HashMap::new(),
            cache_panes: HashMap::new(),
            pane_passive: false,
            pane_focus_key: 0,
            hover_pane: None,
            pointer_pane: 0,
            worker,
            renderer,
            dialogs: Dialogs::default(),
            actions: Vec::new(),
            status: None,
            sync_render: false,
            inspector_preview: None,
            theme_dirty: true,
            quit_confirmed: false,
            close_window: false,
            flyout: None,
            space_down: false,
            pointer_doc: None,
            pointer_log: Vec::new(),
            tool_cursor: fp_engine::tools::Cursor::Arrow,
            painted_cursor: None,
            frame_counter: 0,
            dock: crate::docking::DockUi::default(),
            file_override: None,
            os_clipboard: true,
            pending_drop: None,
            batch: None,
            place_import: None,
            playback: crate::playback::Playback::default(),
        };
        st.sync_panels_to_engine();
        st
    }

    /// Aktion einreihen. The first command after a pop-up commit of this frame is queued as an
    /// [`Action::Commit`] for the objects that pop-up was opened for (FB-46).
    pub fn push(&mut self, a: Action) {
        let a = match a {
            Action::Run(name, params) => match crate::inspector::preview::take_commit_stamp() {
                Some(target) => Action::Commit {
                    target,
                    name,
                    params,
                },
                None => Action::Run(name, params),
            },
            a => a,
        };
        self.actions.push(a);
    }

    /// Befehl einreihen (Kurzform).
    pub fn run(&mut self, name: &str, params: Value) {
        self.push(Action::Run(name.to_string(), params));
    }

    /// Wie lange eine Meldung in der Statuszeile steht (Sekunden), danach wieder die Auswahl-Info.
    pub const STATUS_SECONDS: f64 = 6.0;

    /// Die gerade sichtbare Meldung der Statuszeile (abgelaufene nicht mehr; CT-C-12).
    pub fn shown_status(&self, now: f64) -> Option<&str> {
        self.status
            .as_ref()
            .filter(|(_, t)| now - t < Self::STATUS_SECONDS)
            .map(|(m, _)| m.as_str())
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
        self.session
            .ui
            .panels
            .insert("window.toolbars.main".into(), ws.show_toolbar);
        self.session
            .ui
            .panels
            .insert("window.tools".into(), ws.show_tools);
        self.session
            .ui
            .panels
            .insert("window.properties".into(), ws.show_inspector);
    }

    /// Ereignisse der Sitzung verarbeiten (Leinwand-Caches ungültig machen usw.).
    pub fn handle_session_events(&mut self) {
        for e in self.session.drain_events() {
            match e {
                SessionEvent::DocumentClosed(id) => {
                    self.canvases.remove(&id);
                    self.pane_canvases.retain(|k, _| k.0 != id);
                    self.cache_panes.remove(&id);
                }
                SessionEvent::DocumentChanged { doc, .. } | SessionEvent::LocationChanged(doc) => {
                    if let Some(c) = self.canvases.get_mut(&doc) {
                        c.invalidate();
                    }
                    for (_, c) in self.pane_canvases.iter_mut().filter(|(k, _)| k.0 == doc) {
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
                SessionEvent::CommandExecuted(name)
                    if name.starts_with("file.previewInBrowser.preview") =>
                {
                    if let Some(id) = self.session.active_id() {
                        let dir = fp_engine::commands::export::preview_dir(id.0);
                        self.actions.push(Action::Status(fp_i18n::tr!(
                            "ui-preview-created",
                            path = dir.to_string_lossy().to_string()
                        )));
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
            let mut batch = std::mem::take(&mut self.actions);
            // FB-46: Undo / Redo / a History jump discard what a pop-up wanted to apply in the
            // same frame (the pop-up is discarded, not applied)
            if batch.iter().any(
                |a| matches!(a, Action::Run(n, _) if crate::inspector::preview::moves_history(n)),
            ) {
                batch.retain(|a| !matches!(a, Action::Commit { .. }));
            }
            for a in batch {
                crate::actions::execute(self, ctx, a);
            }
            self.handle_session_events();
        }
        crate::inspector::preview::clear_commit_stamp();
        // Rastern für den Filter nach der Vektor-Rückfrage gilt nur bis zum Schließen des
        // Filterdialogs (OK oder Abbrechen, re 28 L9)
        if self.session.filter_rasterize && self.actions.is_empty() && !self.dialogs.any_open() {
            self.session.filter_rasterize = false;
        }
    }

    /// Title of the main window: the display name, then the document name as in the original, with
    /// a leading star for unsaved changes ("Firepixel Studio – *Name.fw.png", no zoom); without a
    /// document only the display name ([`fp_i18n::DISPLAY_NAME`]).
    pub fn window_title(&self) -> String {
        match self.session.active() {
            Some(d) => {
                let star = if d.is_dirty() { "*" } else { "" };
                let name = d
                    .path
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| d.title.clone());
                format!("{} – {star}{name}", fp_i18n::DISPLAY_NAME)
            }
            None => fp_i18n::DISPLAY_NAME.to_string(),
        }
    }

    /// Hat irgendein Dokument ungespeicherte Änderungen?
    pub fn any_dirty(&self) -> bool {
        self.session.documents().iter().any(|d| d.is_dirty())
    }
}
