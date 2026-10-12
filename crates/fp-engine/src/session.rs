//! Sitzung: offene Dokumente, aktives Dokument/Seite/Status/Ebene, Ansichtszustand,
//! Änderungs-Generation für Render-Caches, Werkzeuge, Zwischenablage, Voreinstellungen und
//! Ereignis-Benachrichtigung an die UI.
//!
//! Alles läuft über [`Session::execute`]: Befehl suchen, Verfügbarkeit und Parameter prüfen,
//! ausführen, bei Dokument-/Auswahländerung einen Verlaufsschritt anlegen, Generation erhöhen und
//! Ereignisse in die Warteschlange stellen ([`Session::drain_events`]). Schlägt ein Befehl fehl, wird
//! der Zustand vor dem Befehl wiederhergestellt.

use crate::command::{validate, Command, CommandOutput, CommandRegistry, Effect, Params, Requirement};
use crate::error::{EngineError, Result};
use crate::geometry::{default_geometry, GeometryOps};
use crate::history::{History, RecordedCommand, Snapshot};
use crate::prefs::Preferences;
use crate::render::PageRenderer;
use crate::selection::{Clipboard, Selection};
use crate::shortcuts::{self, Shortcut, ShortcutSet};
use crate::tools::{self, Invocation, ToolContext, ToolEvent, ToolId, ToolMachine, ToolOptions, ToolResponse};
use crate::view::ViewState;
use fp_color::Argb;
use fp_model::{Document, LayerId};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

/// Kennung eines offenen Dokuments in der Sitzung.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DocId(pub u64);

/// Verschiebung eines Pfeiltasten-Befehls (`edit.nudge.*`), sonst `None`.
/// Adressen aller Elemente der Auswahl (Objekte und Punkte) in einem Durchlauf (PERF-1: vor und
/// nach jedem Befehl; eine Suche je ID kostete bei Tausenden gewählten Objekten Sekunden).
fn selection_paths(doc: &fp_model::Document, page: usize, sel: &crate::selection::Selection) -> std::collections::HashMap<fp_model::ElementId, Option<fp_model::traverse::ElementPath>> {
    let mut ids: Vec<fp_model::ElementId> = sel.elements.clone();
    ids.extend(sel.points.iter().map(|p| p.element));
    let paths = crate::docops::find_many_on_page(doc, page, &ids);
    ids.into_iter().zip(paths).collect()
}

fn nudge_delta(name: &str) -> Option<(f64, f64)> {
    Some(match name.strip_prefix("edit.nudge.")? {
        "left" => (-1.0, 0.0),
        "right" => (1.0, 0.0),
        "up" => (0.0, -1.0),
        "down" => (0.0, 1.0),
        "bigLeft" => (-10.0, 0.0),
        "bigRight" => (10.0, 0.0),
        "bigUp" => (0.0, -10.0),
        "bigDown" => (0.0, 10.0),
        _ => return None,
    })
}

/// Hat sich am Text etwas geändert, das den Satz ändert (Inhalt, Attribute, Rahmenart, Größe des
/// festen Rahmens, Richtung)? Reines Verschieben zählt nicht – dann bleibt der gespeicherte Rahmen
/// (Rundreise mit dem Original).
fn text_layout_changed(a: &fp_model::TextElem, b: &fp_model::TextElem) -> bool {
    let dims = |t: &fp_model::TextElem| t.anchor_rect.map(|r| if r.right == 0.0 && r.bottom == 0.0 { (0.0, 0.0) } else { (r.right - r.left, r.bottom - r.top) });
    let on_path = |t: &fp_model::TextElem| t.on_path.as_ref().map(|o| (o.mode, o.offset));
    a.runs != b.runs
        || a.initial != b.initial
        || a.frame_type != b.frame_type
        || a.orientation != b.orientation
        || a.auto_kern != b.auto_kern
        || dims(a) != dims(b)
        || on_path(a) != on_path(b)
}

/// Setzt `TextElem::bounds` neuer bzw. inhaltlich geänderter Texte auf den Rahmen des echten Satzes
/// (09 §5.3: `LFTf…BOTf` = Ergebnis des Satzes). Vorher stand dort nur die Schätzung von
/// `Element::new_text` bzw. der alte Rahmen – der Auswahlrahmen eines 200-pt-Texts blieb winzig.
fn refresh_text_frames(r: &dyn PageRenderer, before: &Document, doc: &mut Arc<Document>) {
    use fp_model::element::ElementKind;
    use std::collections::HashMap;
    let mut old: HashMap<fp_model::ElementId, Arc<fp_model::Element>> = HashMap::new();
    before.walk_elements(|_, _, e| {
        if matches!(e.kind, ElementKind::Text(_)) {
            old.insert(e.id, Arc::clone(e));
        }
    });
    let mut todo = Vec::new();
    doc.walk_elements(|_, path, e| {
        let ElementKind::Text(t) = &e.kind else { return };
        let changed = match old.get(&e.id) {
            Some(o) if Arc::ptr_eq(o, e) => false,
            Some(o) => match &o.kind {
                ElementKind::Text(ot) => text_layout_changed(ot, t),
                _ => true,
            },
            None => true,
        };
        if changed && let Some(f) = r.text_frame(doc, t) && f != t.bounds {
            todo.push((path.clone(), f));
        }
    });
    if todo.is_empty() {
        return;
    }
    let d = Arc::make_mut(doc);
    for (path, f) in todo {
        crate::docops::edit_at(d, &path, |e| {
            if let ElementKind::Text(t) = &mut e.kind {
                t.bounds = f;
            }
        });
    }
}

/// Ein offenes Dokument mit Verlauf, Auswahl und Ansicht.
#[derive(Clone, Debug)]
pub struct DocumentState {
    pub id: DocId,
    /// Dokument (Copy-on-Write; ändern mit [`DocumentState::doc_mut`]).
    pub doc: Arc<Document>,
    /// Datei (None = noch nicht gespeichert).
    pub path: Option<PathBuf>,
    /// Fenstertitel („Unbenannt-1.png“ bzw. Dateiname).
    pub title: String,
    pub history: History,
    pub selection: Selection,
    pub view: ViewState,
    /// Aktuelle Seite.
    pub page: usize,
    /// Aktueller Status.
    pub frame: usize,
    /// Aktive Ebene.
    pub layer: Option<LayerId>,
    /// Steigt bei jeder Inhaltsänderung (auch Undo/Redo): Schlüssel für Render-Caches.
    pub generation: u64,
    /// Generation beim letzten Speichern.
    pub saved_generation: u64,
    /// Bitmap-Bearbeitungsmodus (Strg+E).
    pub bitmap_mode: bool,
    /// Symbol-Bearbeitungsmodus: dieses Dokument ist die Bearbeitungsansicht eines Symbols.
    pub symbol_edit: Option<SymbolEdit>,
    /// Art der Datei: `true` = „Reduziertes PNG“ (re\20 §5.3, EB-2) – `file.save` schreibt wieder
    /// flach, bis unter einem anderen Typ gespeichert wird; `false` = FW-PNG.
    pub flattened: bool,
}

/// Verknüpfung einer Symbol-Bearbeitungsansicht mit ihrem Dokument („Symbol bearbeiten“,
/// „An Position bearbeiten“).
#[derive(Clone, Debug, PartialEq)]
pub struct SymbolEdit {
    /// Dokument, dem das Symbol gehört.
    pub parent: DocId,
    /// Bearbeitetes Symbol.
    pub symbol: fp_model::SymbolId,
    /// An Position: die Oberfläche zeigt die Seite des Dokuments abgeblendet dahinter.
    pub in_place: bool,
    /// Seite und Instanz, von der aus bearbeitet wird (Rückkehr, Abbildung für „an Position“).
    pub page: usize,
    pub instance: Option<fp_model::ElementId>,
    /// Abbildung Symbol → Seite der Instanz (für „an Position“).
    pub transform: Option<fp_geom::Matrix3>,
}

impl DocumentState {
    /// Neuer Zustand für ein Dokument.
    pub fn new(id: DocId, doc: Document, path: Option<PathBuf>, title: String, prefs: &Preferences) -> DocumentState {
        let page = doc.current_page.min(doc.pages.len().saturating_sub(1));
        let layer = crate::docops::top_normal_layer(&doc, page);
        DocumentState {
            id,
            doc: Arc::new(doc),
            path,
            title,
            history: History::new(prefs.max_undos()),
            selection: Selection::default(),
            view: ViewState::from_prefs(prefs),
            page,
            frame: 0,
            layer,
            generation: 0,
            saved_generation: 0,
            bitmap_mode: false,
            symbol_edit: None,
            flattened: false,
        }
    }

    /// Veränderlicher Zugriff (Copy-on-Write).
    pub fn doc_mut(&mut self) -> &mut Document {
        Arc::make_mut(&mut self.doc)
    }

    /// Ungespeicherte Änderungen?
    pub fn is_dirty(&self) -> bool {
        self.generation != self.saved_generation
    }

    /// Schnappschuss für den Verlauf.
    pub fn snapshot(&self) -> Snapshot {
        Snapshot { doc: Arc::clone(&self.doc), selection: self.selection.clone(), page: self.page, frame: self.frame, layer: self.layer }
    }

    fn restore(&mut self, s: Snapshot) {
        self.doc = s.doc;
        self.selection = s.selection;
        self.page = s.page.min(self.doc.pages.len().saturating_sub(1));
        self.frame = s.frame;
        self.layer = s.layer;
    }

    /// Anzahl Status der aktuellen Seite.
    pub fn frame_count(&self) -> usize {
        self.doc.pages.get(self.page).map(|p| p.root.frame_count as usize).unwrap_or(1).max(1)
    }
}

/// Ereignisse für die UI (abholen mit [`Session::drain_events`]).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SessionEvent {
    DocumentOpened(DocId),
    DocumentClosed(DocId),
    ActiveDocumentChanged(Option<DocId>),
    /// Inhalt geändert; `generation` = neue Generation.
    DocumentChanged { doc: DocId, generation: u64 },
    SelectionChanged(DocId),
    ViewChanged(DocId),
    HistoryChanged(DocId),
    /// Seite/Status/Ebene gewechselt.
    LocationChanged(DocId),
    Saved { doc: DocId, path: PathBuf },
    ToolChanged(ToolId),
    ColorsChanged,
    PreferencesChanged,
    PanelsChanged,
    QuitRequested,
    /// Befehl ausgeführt (für Statuszeile/Protokoll).
    CommandExecuted(String),
    /// Befehl ohne passendes Ziel: Warnung wie im Original (siehe [`crate::warnings`]).
    Warning(crate::warnings::TargetWarning),
}

/// UI-Zustand, den Befehle schalten (Bedienfelder, Bildschirmmodus).
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct UiState {
    /// Sichtbarkeit der Bedienfelder nach Befehlsnamen (`window.layers` …).
    pub panels: BTreeMap<String, bool>,
    /// „Bedienfelder ausblenden“ (Tab).
    pub panels_hidden: bool,
    /// Bildschirmmodus (0 Standard, 1 Vollbild mit Menüs, 2 Vollbild).
    pub screen_mode: u8,
    /// QuickInfos.
    pub tooltips: bool,
    /// Beenden angefordert.
    pub quit_requested: bool,
    /// Primär-/Sekundärbrowser für die Vorschau.
    pub browsers: [Option<String>; 2],
    /// Seiteneinrichtung für „Drucken“.
    pub page_setup: crate::commands::PageSetup,
    /// Fensteranordnung der Dokumente: `tabs` (Vorgabe), `cascade` (Überlappend), `tile` (Anordnen).
    pub window_layout: String,
    /// Zusätzliche Fenster („Fenster duplizieren“): Dokumente, die ein zweites Mal angezeigt werden.
    pub extra_windows: Vec<DocId>,
    /// Status-Wiedergabe läuft (Abspielen/Stopp).
    pub playing: bool,
}

/// Skript-Laufzeit (JSF), die die Anwendung einsteckt (fp-script; fp-engine kennt sie nur über
/// diese Schnittstelle, weil fp-script selbst auf fp-engine aufbaut). Genutzt von „Befehle ▸ Skript
/// ausführen“ und den gespeicherten Befehlen.
pub trait ScriptHost: Send + Sync {
    /// Führt Skriptquelltext aus (`path` = Datei, für relative Pfade und Fehlermeldungen) und liefert
    /// das Ergebnis des letzten Ausdrucks als JSON. Die Befehle des Skripts laufen über
    /// [`Session::execute`] und bilden zusammen einen Verlaufsschritt.
    fn run_script(&self, session: &mut Session, code: &str, path: Option<&std::path::Path>) -> Result<Value>;

    /// Automatische Formen (Smart Shapes, deren Verhalten als Skript vorliegt): `op` enthält die
    /// Befehlsparameter plus `"op"` (`insert`, `dragControlPoint`, `setParams`, `redraw`,
    /// `describe`, `list`). Vorgabe: nicht verfügbar.
    fn auto_shape(&self, _session: &mut Session, _op: &Value) -> Result<Value> {
        Err(EngineError::BackendMissing("err-backend-autoshape-runtime"))
    }

    /// Namen (ohne `.jsf`) der mitgelieferten Befehlsskripte auf oberster Ebene des Befehlsordners.
    /// Sie gehören zum Befehle-Menü, sind aber keine gespeicherten Befehle des Nutzers (im Original
    /// liegen sie im Programmordner); „Gespeicherte Befehle verwalten“ lässt sie weg. Vorgabe: keine.
    fn builtin_commands(&self) -> Vec<String> {
        Vec::new()
    }
}

/// Werkzeugzustand der Sitzung.
pub struct ToolState {
    /// Aktives Werkzeug.
    pub active: ToolId,
    machine: Box<dyn ToolMachine>,
    /// Zuletzt benutztes Werkzeug je Flyout-Gruppe (`active tool for slot …`).
    pub slot_memory: BTreeMap<String, ToolId>,
    /// Strich- und Füllfarbe (Farbfelder im Werkzeugbedienfeld).
    pub stroke_color: Argb,
    pub fill_color: Argb,
    /// Werkzeugoptionen (Eigenschaften-Inspektor).
    pub options: ToolOptions,
}

impl std::fmt::Debug for ToolState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolState").field("active", &self.active).field("stroke", &self.stroke_color).field("fill", &self.fill_color).finish()
    }
}

/// Die Sitzung.
pub struct Session {
    docs: Vec<DocumentState>,
    active: Option<usize>,
    /// Voreinstellungen.
    pub prefs: Preferences,
    /// Datei der Voreinstellungen (None = nicht speichern).
    pub prefs_path: Option<PathBuf>,
    /// Interne Zwischenablage.
    pub clipboard: Clipboard,
    /// Werkzeuge und Farben.
    pub tools: ToolState,
    /// UI-Zustand.
    pub ui: UiState,
    registry: Arc<CommandRegistry>,
    geometry: Arc<dyn GeometryOps>,
    renderer: Option<Arc<dyn PageRenderer>>,
    script_host: Option<Arc<dyn ScriptHost>>,
    events: Vec<SessionEvent>,
    /// Letzter Filteraufruf (für „Filter wiederholen“).
    pub last_filter: Option<RecordedCommand>,
    /// Letzter Dokumentbefehl und seine Verlaufsbeschriftung (Bearbeiten ▸ Wiederholen ohne
    /// zurückgenommenen Schritt wiederholt ihn wie im Original).
    pub last_command: Option<RecordedCommand>,
    pub last_command_label: Option<String>,
    group_depth: u32,
    group_log: Vec<RecordedCommand>,
    /// Nächster Befehl legt auch ohne Änderung einen Verlaufsschritt an (Original: z. B.
    /// „JPEG-Maske löschen“ ohne Maske).
    force_step: bool,
    next_doc: u64,
    untitled: u32,
}

impl Default for Session {
    fn default() -> Self {
        Session::new(Preferences::new())
    }
}

impl Session {
    /// Neue Sitzung mit allen eingebauten Befehlen.
    pub fn new(prefs: Preferences) -> Session {
        Session::with_registry(prefs, Arc::new(CommandRegistry::with_defaults()))
    }

    /// Neue Sitzung mit eigenem Register (z. B. erweitert um Plugin-Befehle).
    pub fn with_registry(prefs: Preferences, registry: Arc<CommandRegistry>) -> Session {
        let (stroke, fill) = prefs.current_colors();
        let options = ToolOptions::from_prefs(&prefs);
        Session {
            docs: Vec::new(),
            active: None,
            prefs_path: None,
            clipboard: Clipboard::default(),
            tools: ToolState {
                active: ToolId::Pointer,
                machine: tools::make_machine(ToolId::Pointer),
                slot_memory: BTreeMap::new(),
                stroke_color: stroke,
                fill_color: fill,
                options,
            },
            ui: UiState { tooltips: true, ..UiState::default() },
            registry,
            geometry: default_geometry(),
            renderer: crate::render::default_renderer(),
            script_host: None,
            events: Vec::new(),
            last_filter: None,
            last_command: None,
            last_command_label: None,
            group_depth: 0,
            group_log: Vec::new(),
            force_step: false,
            next_doc: 1,
            untitled: 0,
            prefs,
        }
    }

    // ------------------------------------------------------------ Zugriffe

    /// Das Befehlsregister.
    pub fn registry(&self) -> &Arc<CommandRegistry> {
        &self.registry
    }

    /// Geometrie-Umsetzung.
    pub fn geometry(&self) -> &dyn GeometryOps {
        self.geometry.as_ref()
    }

    /// Geometrie-Umsetzung als geteilter Zeiger.
    pub fn geometry_arc(&self) -> Arc<dyn GeometryOps> {
        Arc::clone(&self.geometry)
    }

    /// Geometrie-Umsetzung tauschen.
    pub fn set_geometry(&mut self, g: Arc<dyn GeometryOps>) {
        self.geometry = g;
    }

    /// Renderer (fp-render) einstecken.
    pub fn set_renderer(&mut self, r: Option<Arc<dyn PageRenderer>>) {
        self.renderer = r;
    }

    /// Eingesteckter Renderer.
    pub fn renderer(&self) -> Option<&Arc<dyn PageRenderer>> {
        self.renderer.as_ref()
    }

    /// Skript-Laufzeit einstecken (fp-script).
    pub fn set_script_host(&mut self, h: Option<Arc<dyn ScriptHost>>) {
        self.script_host = h;
    }

    /// Eingesteckte Skript-Laufzeit.
    pub fn script_host(&self) -> Option<Arc<dyn ScriptHost>> {
        self.script_host.clone()
    }

    /// Offene Dokumente.
    pub fn documents(&self) -> &[DocumentState] {
        &self.docs
    }

    /// Aktives Dokument.
    pub fn active(&self) -> Option<&DocumentState> {
        self.active.and_then(|i| self.docs.get(i))
    }

    /// Aktives Dokument (veränderlich).
    pub fn active_mut(&mut self) -> Option<&mut DocumentState> {
        match self.active {
            Some(i) => self.docs.get_mut(i),
            None => None,
        }
    }

    /// Aktives Dokument oder Fehler.
    pub fn doc(&self) -> Result<&DocumentState> {
        self.active().ok_or(EngineError::NoDocument)
    }

    /// Aktives Dokument (veränderlich) oder Fehler.
    pub fn doc_mut(&mut self) -> Result<&mut DocumentState> {
        self.active_mut().ok_or(EngineError::NoDocument)
    }

    /// Kennung des aktiven Dokuments.
    pub fn active_id(&self) -> Option<DocId> {
        self.active().map(|d| d.id)
    }

    /// Dokument zur Kennung.
    pub fn document(&self, id: DocId) -> Option<&DocumentState> {
        self.docs.iter().find(|d| d.id == id)
    }

    /// Aktiviert ein Dokument.
    pub fn activate(&mut self, id: DocId) -> Result<()> {
        let i = self.docs.iter().position(|d| d.id == id).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-document")))?;
        if self.active != Some(i) {
            self.active = Some(i);
            self.events.push(SessionEvent::ActiveDocumentChanged(Some(id)));
        }
        Ok(())
    }

    /// Fügt ein Dokument hinzu und aktiviert es.
    pub fn add_document(&mut self, doc: Document, path: Option<PathBuf>) -> DocId {
        let id = DocId(self.next_doc);
        self.next_doc += 1;
        let title = match &path {
            Some(p) => p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| fp_i18n::t("msg-default-document-nameless")),
            None => {
                // Vorgabetitel in der Sprache der Oberfläche beim Anlegen (wie die übrigen Standardnamen)
                self.untitled += 1;
                format!("{}.png", fp_i18n::tr!("msg-default-document", n = self.untitled.to_string()))
            }
        };
        let mut st = DocumentState::new(id, doc, path, title, &self.prefs);
        st.view.viewport = self.active().map(|d| d.view.viewport).unwrap_or(st.view.viewport);
        self.docs.push(st);
        self.active = Some(self.docs.len() - 1);
        self.events.push(SessionEvent::DocumentOpened(id));
        self.events.push(SessionEvent::ActiveDocumentChanged(Some(id)));
        id
    }

    /// Schließt ein Dokument (ohne Rückfrage; die UI fragt vorher nach `is_dirty`).
    pub fn close_document(&mut self, id: DocId) -> Result<()> {
        let i = self.docs.iter().position(|d| d.id == id).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-document")))?;
        self.docs.remove(i);
        self.events.push(SessionEvent::DocumentClosed(id));
        self.active = if self.docs.is_empty() { None } else { Some(i.min(self.docs.len() - 1)) };
        self.events.push(SessionEvent::ActiveDocumentChanged(self.active_id()));
        Ok(())
    }

    /// Ereignisse abholen (leert die Warteschlange).
    pub fn drain_events(&mut self) -> Vec<SessionEvent> {
        std::mem::take(&mut self.events)
    }

    /// Ereignis melden.
    pub fn emit(&mut self, e: SessionEvent) {
        self.events.push(e);
    }

    /// Aktives Tastenkürzel-Set (Voreinstellung `CurrentShortcutSet`, Rückfall „Firepixel“).
    pub fn shortcut_set(&self) -> &'static ShortcutSet {
        static EMPTY: std::sync::OnceLock<ShortcutSet> = std::sync::OnceLock::new();
        shortcuts::any_set(&self.prefs.shortcut_set())
            .or_else(|| shortcuts::builtin_set("Firepixel"))
            .unwrap_or_else(|| EMPTY.get_or_init(ShortcutSet::default))
    }

    /// Kürzel eines Befehls im aktiven Set.
    pub fn command_shortcuts(&self, name: &str) -> Vec<Shortcut> {
        self.registry.get(name).map(|c| c.info().shortcuts(self.shortcut_set()).to_vec()).unwrap_or_default()
    }

    /// Befehl zu einem Menü-Tastenkürzel im aktiven Set.
    pub fn command_for_shortcut(&self, sc: &Shortcut) -> Option<String> {
        let set = self.shortcut_set();
        set.commands_for(sc).into_iter().find_map(|cmd| self.registry.by_fw_cmd(cmd).map(|c| c.info().name.clone()))
    }

    // ------------------------------------------------------------ Verfügbarkeit

    /// Ist eine Bedingung erfüllt?
    pub fn requirement_met(&self, r: &Requirement) -> bool {
        let d = self.active();
        match r {
            Requirement::Document => d.is_some(),
            Requirement::Selection => d.is_some_and(|d| !d.selection.elements.is_empty()),
            Requirement::MinSelected(n) => d.is_some_and(|d| d.selection.elements.len() >= *n),
            Requirement::PixelSelection => d.is_some_and(|d| d.selection.has_pixels()),
            Requirement::SelectionOrPixels => d.is_some_and(|d| !d.selection.is_empty()),
            Requirement::AllOf(k) => d.is_some_and(|d| {
                // ein Durchlauf statt einer Suche je ID (PERF-1: Menüs/Werkzeugleisten fragen das je Bild)
                let els = crate::docops::elements_on_page(&d.doc, d.page, &d.selection.elements);
                !els.is_empty() && els.iter().all(|e| crate::command::element_is(e, *k, &d.doc))
            }),
            Requirement::AnyOf(k) => d.is_some_and(|d| crate::docops::elements_on_page(&d.doc, d.page, &d.selection.elements).iter().any(|e| crate::command::element_is(e, *k, &d.doc))),
            Requirement::Clipboard => !self.clipboard.is_empty(),
            Requirement::CanUndo => d.is_some_and(|d| d.history.can_undo()),
            Requirement::CanRedo => d.is_some_and(|d| d.history.can_redo()),
            Requirement::HasPath => d.is_some_and(|d| d.path.is_some()),
            Requirement::MultiplePages => d.is_some_and(|d| d.doc.pages.len() > 1),
            Requirement::MultipleFrames => d.is_some_and(|d| d.frame_count() > 1),
        }
    }

    fn unmet(&self, c: &dyn Command) -> Option<String> {
        for r in &c.info().requires {
            if !self.requirement_met(r) {
                return Some(r.reason());
            }
        }
        if !c.extra_available(self) {
            return Some(fp_i18n::t("err-req-state"));
        }
        None
    }

    /// Ist der Befehl verfügbar (Menüeintrag aktiv)?
    pub fn is_available(&self, name: &str) -> bool {
        self.registry.get(name).is_some_and(|c| c.info().implemented && self.unmet(c.as_ref()).is_none() && c.menu_available(self))
    }

    /// Menüzustand eines Befehls: (aktiv, Häkchen).
    pub fn command_state(&self, name: &str) -> (bool, Option<bool>) {
        match self.registry.get(name) {
            Some(c) => (c.info().implemented && self.unmet(c.as_ref()).is_none() && c.menu_available(self), c.checked(self)),
            None => (false, None),
        }
    }

    // ------------------------------------------------------------ Ausführen

    /// Führt einen Befehl aus (siehe Moduldoku).
    pub fn execute(&mut self, name: &str, params: Value) -> Result<CommandOutput> {
        let reg = Arc::clone(&self.registry);
        let cmd = reg.get(name).ok_or_else(|| EngineError::UnknownCommand(name.into()))?;
        let info = cmd.info();
        if !info.implemented {
            return Err(EngineError::NotImplemented(name.into()));
        }
        // Offener Text-Editor: Textattribute aus Inspektor und Menü „Text“ gehen live an den
        // Editor (Markierung bzw. ganzer Text), der Editor bleibt offen; kein Verlaufsschritt –
        // übernommen wird beim Beenden der Bearbeitung.
        if let Some(cur) = self.tools.machine.text_edit_attrs()
            && let Some(patch) = crate::commands::editor_patch(name, &params, &cur)
        {
            let ev = ToolEvent { input: tools::ToolInput::Edit(tools::EditOp::Attrs(patch)), mods: Default::default(), pressure: 1.0, time_ms: 0 };
            self.tool_event(&ev)?;
            self.events.push(SessionEvent::CommandExecuted(name.into()));
            return Ok(CommandOutput::value(Value::Null));
        }
        // Gesperrte Objekte sind wie im Original nicht ausgewählt (z. B. nach dem Sperren oder bei
        // Auswahl über Ebenen/Skripte) – kein Befehl verändert sie über die Auswahl.
        if name != "element.setLocked"
            && let Some(d) = self.active_mut()
            && !d.selection.elements.is_empty()
        {
            let (doc, page) = (Arc::clone(&d.doc), d.page);
            let paths = selection_paths(&doc, page, &d.selection);
            d.selection.retain_existing(|id| paths.get(&id).and_then(|p| p.as_ref()).is_none_or(|p| !crate::docops::is_locked(&doc, page, p)));
        }
        if let Some(reason) = self.unmet(cmd.as_ref()) {
            return Err(EngineError::Unavailable { name: name.into(), reason });
        }
        // Immer aktive Befehle ohne Ziel: Warnung des Originals statt Wirkung
        if let Some(w) = self.target_warning(name) {
            let text = w.text.clone();
            if !w.suppressed {
                self.events.push(SessionEvent::Warning(w));
            }
            return Err(EngineError::Selection(text));
        }
        let p = Params::new(name, params);
        self.force_step = false;
        validate(info, &p)?;
        let doc_id = self.active_id();
        let before = self.active().map(|d| d.snapshot());
        let res = cmd.apply(self, &p);
        let out = match res {
            Ok(o) => o,
            Err(e) => {
                if let (Some(id), Some(b)) = (doc_id, before)
                    && let Some(d) = self.docs.iter_mut().find(|d| d.id == id)
                {
                    d.restore(b);
                }
                return Err(e);
            }
        };
        if out.effect == Effect::Document && !matches!(name, "edit.undo" | "edit.redo") && !name.starts_with("history.") {
            self.last_command = Some(RecordedCommand { command: name.into(), params: p.value.clone() });
            self.last_command_label = Some(out.label.clone().unwrap_or_else(|| info.label.clone()));
        }
        if name.starts_with("filter.") && name != "filter.repeat" {
            self.last_filter = Some(RecordedCommand { command: name.into(), params: p.value.clone() });
        }
        match out.effect {
            Effect::Document => {
                if let (Some(id), Some(b)) = (doc_id, before) {
                    self.finish_step(id, b, out.label.clone().unwrap_or_else(|| info.step_label()), name, &p.value);
                }
            }
            Effect::View => {
                if let Some(id) = self.active_id() {
                    self.events.push(SessionEvent::ViewChanged(id));
                }
            }
            Effect::Session | Effect::None => {}
        }
        if self.group_depth > 0 {
            self.group_log.push(RecordedCommand { command: name.into(), params: p.value.clone() });
        }
        self.events.push(SessionEvent::CommandExecuted(name.into()));
        Ok(out)
    }

    /// Ändert ein (auch nicht aktives) Dokument als eigenen Verlaufsschritt.
    pub(crate) fn modify_document(&mut self, id: DocId, label: &str, command: &str, f: impl FnOnce(&mut DocumentState) -> Result<()>) -> Result<()> {
        let i = self.docs.iter().position(|d| d.id == id).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-document")))?;
        let before = self.docs[i].snapshot();
        if let Err(e) = f(&mut self.docs[i]) {
            self.docs[i].restore(before);
            return Err(e);
        }
        self.finish_step(id, before, label.into(), command, &Value::Null);
        Ok(())
    }

    fn finish_step(&mut self, id: DocId, before: Snapshot, label: String, name: &str, params: &Value) {
        let grouping = self.group_depth > 0;
        let renderer = self.renderer.clone();
        let Some(d) = self.docs.iter_mut().find(|d| d.id == id) else { return };
        if let Some(r) = &renderer
            && !Arc::ptr_eq(&before.doc, &d.doc)
        {
            refresh_text_frames(r.as_ref(), &before.doc, &mut d.doc);
        }
        let page = d.page;
        let doc = Arc::clone(&d.doc);
        let paths = selection_paths(&doc, page, &d.selection);
        d.selection.retain_existing(|eid| paths.get(&eid).is_some_and(Option::is_some));
        let doc_changed = !Arc::ptr_eq(&before.doc, &d.doc);
        let sel_changed = before.selection != d.selection;
        let loc_changed = before.page != d.page || before.frame != d.frame || before.layer != d.layer;
        let forced = std::mem::take(&mut self.force_step) && !grouping;
        if !doc_changed && !sel_changed && !loc_changed && !forced {
            return;
        }
        if doc_changed {
            d.generation += 1;
            self.events.push(SessionEvent::DocumentChanged { doc: id, generation: d.generation });
        }
        if sel_changed {
            self.events.push(SessionEvent::SelectionChanged(id));
        }
        if loc_changed {
            self.events.push(SessionEvent::LocationChanged(id));
        }
        if grouping {
            return;
        }
        // Reine Auswahl per Klick/Rahmen/Agent (nicht die Auswahl-Menübefehle) landet wie im Original
        // nicht im Verlauf – sonst nähme z. B. „Abbrechen“ einer Dialogvorschau nach einem
        // Auswahlwechsel die Vorschau nicht mehr zurück.
        const SILENT: &[&str] = &["select.set", "select.pointAt", "select.inRect", "select.points", "select.next", "select.previous"];
        if !doc_changed && SILENT.contains(&name) {
            return;
        }
        // Pfeiltasten: aufeinanderfolgendes Verschieben (auch Umschalt, andere Richtung) ist im
        // Original EIN Schritt (gemessen: 3× rechts + Umschalt+unten → `moveSelectionBy({x:3, y:10})`,
        // `docs\UX_ABGLEICH_A.md` N03). Zusammengefasst wird nur direkt Aufeinanderfolgendes mit
        // gleicher Auswahl; dazwischen ein anderer Schritt beginnt einen neuen.
        let now = d.snapshot();
        if doc_changed
            && let Some((dx, dy)) = nudge_delta(name)
            && let Some(top) = d.history.last_mergeable()
            && (top.command.starts_with("edit.nudge.") || (top.command == "edit.move" && top.params.get("nudge").is_some()))
            && Arc::ptr_eq(&top.after.doc, &before.doc)
            && top.after.selection == before.selection
            && top.after.page == before.page
            && top.after.frame == before.frame
        {
            let (x0, y0) = match nudge_delta(&top.command) {
                Some(v) => v,
                None => (top.params["dx"].as_f64().unwrap_or(0.0), top.params["dy"].as_f64().unwrap_or(0.0)),
            };
            top.command = "edit.move".into();
            top.params = serde_json::json!({"dx": x0 + dx, "dy": y0 + dy, "nudge": true});
            top.after = now;
            self.events.push(SessionEvent::HistoryChanged(id));
            return;
        }
        if doc_changed || sel_changed || forced {
            let after = now;
            d.history.push(crate::history::HistoryStep {
                label,
                command: name.into(),
                params: params.clone(),
                recordable: true,
                before,
                after,
            });
            self.events.push(SessionEvent::HistoryChanged(id));
        }
    }

    /// Der laufende Befehl legt auch ohne Änderung einen Verlaufsschritt an.
    pub(crate) fn force_history_step(&mut self) {
        self.force_step = true;
    }

    /// Führt mehrere Befehle als **einen** Verlaufsschritt aus (wie ein Skript im Original,
    /// Laufzeit T6). Bei einem Fehler wird alles zurückgenommen.
    pub fn run_group(&mut self, label: &str, f: impl FnOnce(&mut Session) -> Result<()>) -> Result<()> {
        let doc_id = self.active_id();
        let before = self.active().map(|d| d.snapshot());
        let outer = self.group_depth == 0;
        if outer {
            self.group_log.clear();
        }
        self.group_depth += 1;
        let r = f(self);
        self.group_depth -= 1;
        if let Err(e) = r {
            if let (Some(id), Some(b)) = (doc_id, before)
                && let Some(d) = self.docs.iter_mut().find(|d| d.id == id)
            {
                d.restore(b);
            }
            return Err(e);
        }
        if outer && let (Some(id), Some(b)) = (doc_id, before) {
            let log = std::mem::take(&mut self.group_log);
            let params = serde_json::json!({ "commands": log, "label": label });
            self.finish_step(id, b, label.into(), "history.replay", &params);
        }
        Ok(())
    }

    /// Führt aufgezeichnete Befehle als ein Verlaufsschritt aus („Schritte wiederholen“).
    pub fn replay(&mut self, cmds: &[RecordedCommand], label: &str) -> Result<()> {
        self.run_group(label, |s| {
            for c in cmds {
                s.execute(&c.command, c.params.clone())?;
            }
            Ok(())
        })
    }

    /// Rückgängig im aktiven Dokument.
    pub fn undo(&mut self) -> Result<()> {
        let d = self.doc_mut()?;
        let snap = d.history.undo().ok_or(EngineError::History("err-req-can-undo"))?;
        self.apply_history_snapshot(snap);
        Ok(())
    }

    /// Beschriftungen für „Rückgängig: …“ und „Wiederholen: …“ im Menü (Original): Rückgängig = letzter
    /// Verlaufsschritt; Wiederholen = zurückgenommener Schritt, sonst der letzte Befehl.
    pub fn undo_redo_labels(&self) -> (Option<String>, Option<String>) {
        let d = self.active();
        let undo = d.and_then(|d| d.history.undo_label().map(str::to_string));
        let redo = d.and_then(|d| d.history.redo_label().map(str::to_string)).or_else(|| d.and(self.last_command_label.clone()));
        (undo, redo)
    }

    /// Wiederholen im aktiven Dokument.
    pub fn redo(&mut self) -> Result<()> {
        let d = self.doc_mut()?;
        let snap = d.history.redo().ok_or(EngineError::History("err-req-can-redo"))?;
        self.apply_history_snapshot(snap);
        Ok(())
    }

    /// Springt im Verlauf (Klick im Verlauf-Bedienfeld).
    pub fn history_go_to(&mut self, position: usize) -> Result<()> {
        let d = self.doc_mut()?;
        if let Some(snap) = d.history.go_to(position) {
            self.apply_history_snapshot(snap);
        }
        Ok(())
    }

    fn apply_history_snapshot(&mut self, snap: Snapshot) {
        let Some(d) = self.active_mut() else { return };
        d.restore(snap);
        d.generation += 1;
        let (id, g) = (d.id, d.generation);
        self.events.push(SessionEvent::DocumentChanged { doc: id, generation: g });
        self.events.push(SessionEvent::SelectionChanged(id));
        self.events.push(SessionEvent::HistoryChanged(id));
    }

    // ------------------------------------------------------------ Werkzeuge

    /// Wechselt das Werkzeug; ein laufender Vorgang des alten Werkzeugs wird abgeschlossen
    /// (z. B. Text übernehmen). Liefert die Antwort des alten Werkzeugs.
    pub fn set_tool(&mut self, id: ToolId) -> Result<ToolResponse> {
        let mut resp = ToolResponse::default();
        if id != self.tools.active {
            let old_resp = self.with_tool_ctx(|m, ctx| m.finish(ctx));
            resp = self.run_tool_response(old_resp)?;
            self.tools.active = id;
            self.tools.machine = tools::make_machine(id);
            if let Some(g) = tools::info(id).map(|i| i.flyout) {
                self.tools.slot_memory.insert(g.to_string(), id);
            }
            self.events.push(SessionEvent::ToolChanged(id));
        }
        Ok(resp)
    }

    /// Werkzeug per Tastenkürzel (aktives Set; mehrfach gleiches Kürzel schaltet durch).
    pub fn tool_shortcut(&mut self, sc: &Shortcut) -> Result<Option<ToolId>> {
        let next = tools::cycle_tool_for_key(self.shortcut_set(), sc, self.tools.active);
        if let Some(t) = next {
            self.set_tool(t)?;
        }
        Ok(next)
    }

    fn with_tool_ctx<R>(&mut self, f: impl FnOnce(&mut dyn ToolMachine, &ToolContext) -> R) -> R {
        // Felder einzeln leihen (kein Klonen von Auswahl/Pixelmaske je Ereignis bzw. Bild)
        let empty_doc;
        let empty_sel;
        let empty_view;
        let active = self.active.and_then(|i| self.docs.get(i));
        let (doc, page, frame, sel, zoom, bitmap_mode, view): (&Document, usize, usize, &Selection, f64, bool, &ViewState) = match active {
            Some(d) => (&d.doc, d.page, d.frame, &d.selection, d.view.zoom, d.bitmap_mode, &d.view),
            None => {
                empty_doc = Document::new(1.0, 1.0, Argb(0));
                empty_sel = Selection::default();
                empty_view = ViewState::from_prefs(&self.prefs);
                (&empty_doc, 0, 0, &empty_sel, 1.0, false, &empty_view)
            }
        };
        let has_doc = active.is_some();
        let ctx = ToolContext {
            doc,
            has_document: has_doc,
            page,
            frame,
            selection: sel,
            zoom,
            prefs: &self.prefs,
            options: &self.tools.options,
            geometry: self.geometry.as_ref(),
            stroke_color: self.tools.stroke_color,
            fill_color: self.tools.fill_color,
            bitmap_mode,
            view,
        };
        f(self.tools.machine.as_mut(), &ctx)
    }

    /// Rückmeldung des aktiven Werkzeugs für den aktuellen Zustand (auch ohne Ereignis, z. B.
    /// Transformationsrahmen, Pfadpunkte der Auswahl, Text-Editor).
    pub fn tool_feedback(&mut self) -> Vec<tools::Overlay> {
        self.with_tool_ctx(|m, ctx| m.feedback(ctx))
    }

    /// Läuft im aktiven Werkzeug ein Text-Editor (Tasten an das Werkzeug statt an Menükürzel)?
    pub fn tool_is_editing_text(&self) -> bool {
        self.tools.machine.is_editing_text()
    }

    /// Attribute im offenen Text-Editor an der Markierung bzw. am Caret (Inspektor zeigt sie
    /// während der Bearbeitung statt der des Objekts).
    pub fn text_edit_attrs(&self) -> Option<fp_model::text::TextAttrs> {
        self.tools.machine.text_edit_attrs()
    }

    /// Rahmen des Texts im offenen Editor (Satz mit allen Änderungen, Dokumentkoordinaten vor
    /// `MTXv`) – für Info-Bedienfeld und B/H/X/Y während der Bearbeitung.
    pub fn text_edit_frame(&self) -> Option<fp_geom::Rect> {
        let d = self.active()?;
        let v = self.tools.machine.overlay().into_iter().find_map(|o| match o {
            tools::Overlay::TextEdit(v) => Some(v),
            _ => None,
        })?;
        let e = v.preview_element(&d.doc)?;
        let fp_model::element::ElementKind::Text(t) = &e.kind else { return None };
        self.renderer.as_ref()?.text_frame(&d.doc, t)
    }

    /// Angezeigte Grenzen des Texts im offenen Editor wie im Original (Rahmen + 2 px Rand je Seite,
    /// `docops::TEXT_MARGIN`) – für B/H/X/Y im Inspektor und das Info-Bedienfeld während der Bearbeitung.
    pub fn text_edit_bounds(&self) -> Option<fp_geom::Rect> {
        self.text_edit_frame().map(|f| crate::docops::text_visible_frame(&f))
    }

    /// Laufenden Werkzeugvorgang abschließen (vor einem Menübefehl, 10 §2.2 „Werkzeug vs.
    /// Menübefehl“), z. B. Text übernehmen oder einen offenen Stiftpfad beenden.
    pub fn finish_tool(&mut self) -> Result<ToolResponse> {
        let resp = self.with_tool_ctx(|m, ctx| m.finish(ctx));
        self.run_tool_response(resp)
    }

    fn run_tool_response(&mut self, mut resp: ToolResponse) -> Result<ToolResponse> {
        let cmds: Vec<Invocation> = std::mem::take(&mut resp.commands);
        let mut first_err = None;
        for c in &cmds {
            match self.execute(&c.command, c.params.clone()) {
                Ok(o) => resp.results.push(o.value),
                Err(e) => {
                    resp.results.push(Value::Null);
                    if first_err.is_none() {
                        first_err = Some(e);
                    }
                }
            }
        }
        resp.commands = cmds;
        if let Some(t) = resp.switch_tool.take() {
            self.tools.active = t;
            self.tools.machine = tools::make_machine(t);
            if let Some(g) = tools::info(t).map(|i| i.flyout) {
                self.tools.slot_memory.insert(g.to_string(), t);
            }
            self.events.push(SessionEvent::ToolChanged(t));
            resp.switch_tool = Some(t);
        }
        if let Some(e) = first_err {
            return Err(e);
        }
        // nachgereichte Ereignisse an das (neue) Werkzeug – nur eine Ebene tief
        let replay = std::mem::take(&mut resp.replay);
        for ev in replay {
            let r = self.with_tool_ctx(|m, ctx| m.handle(&ev, ctx));
            let mut r = r;
            r.replay.clear();
            let r = self.run_tool_response(r)?;
            resp.overlay = r.overlay;
            resp.cursor = r.cursor;
            resp.commands.extend(r.commands);
            resp.results.extend(r.results);
            if r.switch_tool.is_some() {
                resp.switch_tool = r.switch_tool;
            }
        }
        Ok(resp)
    }

    /// Ein Eingabe-Ereignis an das aktive Werkzeug geben; die erzeugten Befehle werden ausgeführt.
    pub fn tool_event(&mut self, ev: &ToolEvent) -> Result<ToolResponse> {
        let resp = self.with_tool_ctx(|m, ctx| m.handle(ev, ctx));
        self.run_tool_response(resp)
    }

    /// Aktuelle Vorschau des aktiven Werkzeugs.
    pub fn tool_overlay(&self) -> Vec<tools::Overlay> {
        self.tools.machine.overlay()
    }

    /// Setzt Strich- oder Füllfarbe des Werkzeugbedienfelds.
    pub fn set_colors(&mut self, stroke: Option<Argb>, fill: Option<Argb>) {
        if let Some(s) = stroke {
            self.tools.stroke_color = s;
        }
        if let Some(f) = fill {
            self.tools.fill_color = f;
        }
        self.events.push(SessionEvent::ColorsChanged);
    }

    // ------------------------------------------------------------ Voreinstellungen

    /// Speichert die Voreinstellungen (falls ein Pfad gesetzt ist).
    pub fn save_prefs(&self) -> Result<()> {
        match &self.prefs_path {
            Some(p) => self.prefs.save(p),
            None => Ok(()),
        }
    }

    /// Nach einer Änderung der Voreinstellungen: Ereignis und ggf. sofort speichern
    /// (`SavePrefsWhileRunning`).
    pub fn prefs_changed(&mut self) {
        self.events.push(SessionEvent::PreferencesChanged);
        if self.prefs.get_bool("SavePrefsWhileRunning", true) {
            let _ = self.save_prefs();
        }
    }
}
