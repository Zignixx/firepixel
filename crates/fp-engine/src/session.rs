//! Sitzung: offene Dokumente, aktives Dokument/Seite/Status/Ebene, Ansichtszustand,
//! Änderungs-Generation für Render-Caches, Werkzeuge, Zwischenablage, Voreinstellungen und
//! Ereignis-Benachrichtigung an die UI.
//!
//! Alles läuft über [`Session::execute`]: Befehl suchen, Verfügbarkeit und Parameter prüfen,
//! ausführen, bei Dokument-/Auswahländerung einen Verlaufsschritt anlegen, Generation erhöhen und
//! Ereignisse in die Warteschlange stellen ([`Session::drain_events`]). Schlägt ein Befehl fehl, wird
//! der Zustand vor dem Befehl wiederhergestellt.

use crate::command::{
    Command, CommandOutput, CommandRegistry, Effect, Params, Requirement, validate,
};
use crate::error::{EngineError, Result};
use crate::geometry::{GeometryOps, default_geometry};
use crate::history::{History, RecordedCommand, Snapshot};

/// Name of a history row whose command or script has no name of its own (TW-137).
pub const STEP_COMMAND_SCRIPT: &str = "msg-step-command-script";
use crate::prefs::Preferences;
use crate::render::PageRenderer;
use crate::selection::{Clipboard, Selection};
use crate::shortcuts::{self, Shortcut, ShortcutSet};
use crate::tools::{
    self, Invocation, ToolContext, ToolEvent, ToolId, ToolMachine, ToolOptions, ToolResponse,
};
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

/// Adressen aller Elemente der Auswahl (Objekte und Punkte) in einem Durchlauf (PERF-1: vor und
/// nach jedem Befehl; eine Suche je ID kostete bei Tausenden gewählten Objekten Sekunden).
fn selection_paths(
    doc: &fp_model::Document,
    page: usize,
    sel: &crate::selection::Selection,
) -> std::collections::HashMap<fp_model::ElementId, Option<fp_model::traverse::ElementPath>> {
    let mut ids: Vec<fp_model::ElementId> = sel.elements.clone();
    ids.extend(sel.points.iter().map(|p| p.element));
    let paths = crate::docops::find_many_on_page(doc, page, &ids);
    ids.into_iter().zip(paths).collect()
}

/// RE spec 64 §3 rule 4: a selection that ends up as exactly one object makes that object's layer
/// or sublayer current, whichever way it was selected (panel row, canvas, marquee, Select All,
/// script); 0 or 2+ objects keep it. Call after the selection's elements changed; `paths` are the
/// addresses of the current selection. Hotspots and slices (web layer) were not measured and keep
/// the current layer. Returns whether the current layer changed.
pub(crate) fn follow_single_selection(
    d: &mut DocumentState,
    paths: &std::collections::HashMap<fp_model::ElementId, Option<fp_model::traverse::ElementPath>>,
) -> bool {
    let [only] = d.selection.elements.as_slice() else {
        return false;
    };
    let Some(Some(path)) = paths.get(only) else {
        return false;
    };
    if d.doc
        .element_at(path)
        .is_some_and(|e| matches!(e.kind, fp_model::ElementKind::WebObject(_)))
    {
        return false;
    }
    match crate::docops::owning_layer(&d.doc, d.page, path) {
        Some(l) if d.layer != Some(l) => {
            d.choose_layer(Some(l));
            true
        }
        // already current: it is the chosen layer now as well (RE spec 65 U23)
        Some(_) => {
            d.layer_choice = None;
            false
        }
        _ => false,
    }
}

/// [`follow_single_selection`] for a selection changed outside a History step (`tool.pick`).
pub(crate) fn follow_single_selection_now(d: &mut DocumentState) -> bool {
    let paths = selection_paths(&d.doc, d.page, &d.selection);
    follow_single_selection(d, &paths)
}

/// Is this step an arrow-key nudge from the canvas (`edit.move` with `nudge`, without `ids`)?
pub(crate) fn is_key_nudge(name: &str, params: &Value) -> bool {
    name == "edit.move"
        && params.get("nudge").and_then(|v| v.as_bool()) == Some(true)
        && params
            .get("ids")
            .and_then(|v| v.as_array())
            .is_none_or(|a| a.is_empty())
}

/// Commands whose history rows Edit ▸ Redo does not repeat (TW-173, re 55 §7.6: after a bitmap
/// brush stroke the entry reads "Can't Redo"): the bitmap strokes of the brush, pencil, eraser and
/// retouch tools.
const NOT_REPEATABLE: &[&str] = &["bitmap.paint", "bitmap.erase", "bitmap.retouch"];

/// Command and parameters that repeat a history row (TW-173). An arrow-key nudge is repeated as a
/// plain move (a new row, not merged into the nudge row); a pen point (`appendPointToPath` in the
/// original) appends the same point to the same contour again.
fn repeat_invocation(step: &crate::history::HistoryStep) -> (String, Value) {
    let mut params = step.params.clone();
    if is_key_nudge(&step.command, &params) {
        if let Some(o) = params.as_object_mut() {
            o.remove("nudge");
        }
    } else if step.command == "path.setContours"
        && params.get("step").and_then(Value::as_str) == Some("pen")
        && let Some(p) = pen_point_again(step)
    {
        params = p;
    }
    (step.command.clone(), params)
}

/// Parameters of a pen point row with its point appended once more: the open contour that gained
/// a point in the row gets a copy of its last node (anchor and handles).
fn pen_point_again(step: &crate::history::HistoryStep) -> Option<Value> {
    let id = fp_model::ElementId(step.params.get("id")?.as_u64()?);
    let before: Vec<usize> =
        match &crate::docops::element_on_page(&step.before.doc, step.before.page, id)?.kind {
            fp_model::ElementKind::Path(pe) => pe.contours.iter().map(|c| c.nodes.len()).collect(),
            _ => return None,
        };
    let mut params = step.params.clone();
    let contours = params.get_mut("contours")?.as_array_mut()?;
    let k = contours.iter().enumerate().position(|(i, c)| {
        let n = c.get("nodes").and_then(Value::as_array).map_or(0, Vec::len);
        before.get(i).is_some_and(|b| n == b + 1)
            && c.get("closed").and_then(Value::as_bool) != Some(true)
    })?;
    let nodes = contours.get_mut(k)?.get_mut("nodes")?.as_array_mut()?;
    let last = nodes.last()?.clone();
    nodes.push(last);
    Some(params)
}

/// Hat sich am Text etwas geändert, das den Satz ändert (Inhalt, Attribute, Rahmenart, Größe des
/// festen Rahmens, Richtung)? Reines Verschieben zählt nicht – dann bleibt der gespeicherte Rahmen
/// (Rundreise mit dem Original).
fn text_layout_changed(a: &fp_model::TextElem, b: &fp_model::TextElem) -> bool {
    let dims = |t: &fp_model::TextElem| {
        t.anchor_rect.map(|r| {
            if r.right == 0.0 && r.bottom == 0.0 {
                (0.0, 0.0)
            } else {
                (r.right - r.left, r.bottom - r.top)
            }
        })
    };
    // mit dem Pfad selbst: Text ▸ Richtung umkehren ändert nur ihn (FP1-03, Rahmen = neue Tinte)
    let on_path = |t: &fp_model::TextElem| {
        t.on_path
            .as_ref()
            .map(|o| (o.mode, o.offset, o.path.clone()))
    };
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
        let ElementKind::Text(t) = &e.kind else {
            return;
        };
        let changed = match old.get(&e.id) {
            Some(o) if Arc::ptr_eq(o, e) => false,
            Some(o) => match &o.kind {
                ElementKind::Text(ot) => text_layout_changed(ot, t),
                _ => true,
            },
            None => true,
        };
        if changed
            && let Some(f) = r.text_frame(doc, t)
            && f != t.bounds
        {
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
    /// Fenstertitel („Unbenannt-1.fw.png“ bzw. Dateiname).
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
    /// RE spec 65 rule 3: while a lock rule shows another layer as current than the chosen one,
    /// `(chosen, current)`; it holds only while `layer` is still that current layer (any other
    /// change of `layer` makes the new current layer the chosen one). See
    /// [`DocumentState::chosen_layer`].
    pub layer_choice: Option<(LayerId, LayerId)>,
    /// RE spec 65 rule 3: the chosen layer of every page visited before, by page (GUID, else
    /// index); a page change brings it back.
    pub page_layers: Vec<(PageKey, LayerId)>,
    /// Steigt bei jeder Inhaltsänderung (auch Undo/Redo): Schlüssel für Render-Caches.
    pub generation: u64,
    /// Generation beim letzten Speichern.
    pub saved_generation: u64,
    /// Bitmap-Bearbeitungsmodus (Strg+E).
    pub bitmap_mode: bool,
    /// Bitmaps, an denen im Bitmap-Bearbeitungsmodus gearbeitet wurde (Auswahlrahmen gezogen,
    /// Pixel gelöscht, gemalt, radiert). Beim Verlassen des Modus (Esc oder Zeigerklick) werden sie
    /// wie im Original auf ihre deckenden Pixel beschnitten, auch ohne Pixeländerung (re 28 L3).
    pub bitmap_worked: Vec<fp_model::ElementId>,
    /// Symbol-Bearbeitungsmodus: dieses Dokument ist die Bearbeitungsansicht eines Symbols.
    pub symbol_edit: Option<SymbolEdit>,
    /// Art der Datei: `true` = „Reduziertes PNG“ (re\20 §5.3, EB-2) – `file.save` schreibt wieder
    /// flach, bis unter einem anderen Typ gespeichert wird; `false` = FW-PNG.
    pub flattened: bool,
    /// Auswahlverlauf für „Vorherige/Nächste Auswahl“ (Q-21).
    pub selection_history: crate::selection::SelectionHistory,
    /// RE spec 59 section 2.3: the tool fill "none" belongs to the document; whether it was none
    /// when this document was left (a new or opened document starts with a solid fill).
    pub tool_fill_none: bool,
}

/// A page in [`DocumentState::page_layers`]: its GUID (stays with the page when pages are moved),
/// else its index.
#[derive(Clone, Debug, PartialEq)]
pub struct PageKey {
    pub id: Option<fp_model::Guid>,
    pub index: usize,
}

impl PageKey {
    fn same_page(&self, o: &PageKey) -> bool {
        match (&self.id, &o.id) {
            (Some(a), Some(b)) => a == b,
            _ => self.index == o.index,
        }
    }
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
    pub fn new(
        id: DocId,
        doc: Document,
        path: Option<PathBuf>,
        title: String,
        prefs: &Preferences,
    ) -> DocumentState {
        // RE spec 64 §3 rule 1 (Q-65): a document opens on page 0 and state 0 with the bottom-most
        // unlocked layer current; the file keeps none of these. `doc.current_page` stays the page
        // the stored root copy and flat image belong to (the writer and render checks need it).
        let page = 0;
        let layer = crate::docops::bottom_unlocked_layer(&doc, page);
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
            layer_choice: None,
            page_layers: Vec::new(),
            generation: 0,
            saved_generation: 0,
            bitmap_mode: false,
            bitmap_worked: Vec::new(),
            symbol_edit: None,
            flattened: false,
            selection_history: Default::default(),
            tool_fill_none: false,
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

    /// Gemeldete Grenzen einer gewählten Bitmap im Bitmap-Bearbeitungsmodus: wie im Original die
    /// Leinwandgrenzen (Eigenschaften und Skript, re 28 L3); sonst `None` (gespeicherte Grenzen).
    pub fn bitmap_mode_bounds(&self, id: fp_model::ElementId) -> Option<fp_geom::Rect> {
        ((self.bitmap_mode || self.selection.has_pixels() || !self.bitmap_worked.is_empty())
            && self.selection.elements.contains(&id)
            && crate::docops::element_on_page(&self.doc, self.page, id)
                .is_some_and(|e| matches!(e.kind, fp_model::ElementKind::Bitmap(_))))
        .then(|| crate::commands::canvas_rect(&self.doc, self.page))
    }

    /// Schnappschuss für den Verlauf.
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            doc: Arc::clone(&self.doc),
            selection: self.selection.clone(),
            page: self.page,
            frame: self.frame,
            layer: self.layer,
            tool_colors: None,
        }
    }

    fn restore(&mut self, s: Snapshot) {
        self.doc = s.doc;
        self.selection = s.selection;
        self.page = s.page.min(self.doc.pages.len().saturating_sub(1));
        self.frame = s.frame;
        self.layer = s.layer;
    }

    /// The chosen layer of the current page (RE spec 65 rule 3): set by a layer row, the selection
    /// circle, `currentLayerNum =`, a single selected object, a new layer, opening and state
    /// changes. The current layer [`DocumentState::layer`] is the first unlocked layer from it
    /// upward; the two differ only while a lock moved the current layer away.
    pub fn chosen_layer(&self) -> Option<LayerId> {
        match self.layer_choice {
            Some((want, at)) if self.layer == Some(at) => Some(want),
            _ => self.layer,
        }
    }

    /// Makes `id` the chosen and the current layer.
    pub fn choose_layer(&mut self, id: Option<LayerId>) {
        self.layer = id;
        self.layer_choice = None;
    }

    /// RE spec 65 rule 3: the current layer again from the chosen one, after locking or unlocking
    /// any layer (the chosen layer stays). A chosen layer outside the page's flat layer list
    /// stays current.
    pub fn resolve_current_layer(&mut self) {
        let Some(want) = self.chosen_layer() else {
            return;
        };
        if let Some(cur) = crate::docops::resolve_layer(&self.doc, self.page, self.frame, want) {
            self.layer = Some(cur);
            self.layer_choice = (cur != want).then_some((want, cur));
        }
    }

    /// RE spec 65 rule 3 for a state change: the current layer from the chosen one in the new
    /// state, and the result becomes the chosen layer (F1–F7).
    pub fn adopt_resolved_layer(&mut self) {
        self.resolve_current_layer();
        self.layer_choice = None;
    }

    fn page_key(&self, page: usize) -> PageKey {
        PageKey {
            id: self.doc.pages.get(page).and_then(|p| p.id.clone()),
            index: page,
        }
    }

    /// RE spec 65 rule 3: notes the chosen layer of the current page before it is left.
    pub fn remember_page_layer(&mut self) {
        let Some(want) = self.chosen_layer() else {
            return;
        };
        let key = self.page_key(self.page);
        self.page_layers.retain(|(k, _)| !k.same_page(&key));
        self.page_layers.push((key, want));
    }

    /// RE spec 65 rule 3 after a page change (call [`DocumentState::remember_page_layer`] on the
    /// page left first): the layer chosen on the last visit of the current page comes back, on
    /// the first visit the bottom-most unlocked layer; the current layer is resolved from it.
    pub fn enter_page_layer(&mut self) {
        let key = self.page_key(self.page);
        let flat = crate::docops::flat_layers(&self.doc, self.page, self.frame);
        let remembered = self
            .page_layers
            .iter()
            .rev()
            .find(|(k, l)| k.same_page(&key) && flat.iter().any(|f| f.id == *l))
            .map(|(_, l)| *l);
        let want =
            remembered.or_else(|| crate::docops::bottom_unlocked_layer(&self.doc, self.page));
        self.choose_layer(want);
        self.resolve_current_layer();
    }

    /// Anzahl Status der aktuellen Seite.
    pub fn frame_count(&self) -> usize {
        self.doc
            .pages
            .get(self.page)
            .map(|p| p.root.frame_count as usize)
            .unwrap_or(1)
            .max(1)
    }
}

/// Ereignisse für die UI (abholen mit [`Session::drain_events`]).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SessionEvent {
    DocumentOpened(DocId),
    DocumentClosed(DocId),
    ActiveDocumentChanged(Option<DocId>),
    /// Inhalt geändert; `generation` = neue Generation.
    DocumentChanged {
        doc: DocId,
        generation: u64,
    },
    SelectionChanged(DocId),
    ViewChanged(DocId),
    HistoryChanged(DocId),
    /// Seite/Status/Ebene gewechselt.
    LocationChanged(DocId),
    Saved {
        doc: DocId,
        path: PathBuf,
    },
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

/// Preference of View ▸ Coordinate Tag (Firepixel extension, FB-41): the tag "x:… y:…" /
/// "w:… h:…" next to the pointer over the canvas (shape tools on hover and while drawing, Pointer
/// while dragging objects, re 58 §4). Independent of View ▸ Tooltips; on by default.
pub const COORDINATE_TAG_PREF: &str = "Firepixel_CoordinateTag";

/// Whether View ▸ Coordinate Tag is on ([`COORDINATE_TAG_PREF`], default on).
pub fn coordinate_tag(prefs: &crate::prefs::Preferences) -> bool {
    prefs.get_bool(COORDINATE_TAG_PREF, true)
}

/// Preference of Layers panel options ▸ Show Group Contents (Firepixel extension, FB-59 to
/// FB-61): group rows get a toggle that lists their objects (and masked objects one that lists
/// the objects of the mask) indented below; a click on such a row selects the object like the
/// Subselection tool. Off: the panel lists top-level objects only, like the reference. On by
/// default.
pub const LAYERS_GROUP_TREE_PREF: &str = "Firepixel_LayersGroupTree";

/// Whether Show Group Contents is on ([`LAYERS_GROUP_TREE_PREF`], default on).
pub fn layers_group_tree(prefs: &crate::prefs::Preferences) -> bool {
    prefs.get_bool(LAYERS_GROUP_TREE_PREF, true)
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
    /// View ▸ Tooltips: tooltips of buttons, fields and panels (FB-37).
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
    fn run_script(
        &self,
        session: &mut Session,
        code: &str,
        path: Option<&std::path::Path>,
    ) -> Result<Value>;

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

/// FB-31: commands that keep a paused bitmap stroke paused (view only: zoom, scroll, screen mode);
/// every other command applies the stroke first.
const PAUSE_KEEPING_COMMANDS: &[&str] = &[
    "view.zoomIn",
    "view.zoomOut",
    "view.setZoom",
    "view.zoomToRect",
    "view.pan",
    "view.setViewport",
    "view.fitAll",
    "view.fitSelection",
    "view.toggleScreenMode",
    "view.exitFullScreen",
    "view.tooltips",
];

/// Werkzeugzustand der Sitzung.
pub struct ToolState {
    /// Aktives Werkzeug.
    pub active: ToolId,
    machine: Box<dyn ToolMachine>,
    /// Previous tool that still finishes its pointer gesture after a tool change
    /// (`ToolResponse::keep_gesture`): gets the moves and the release, then is dropped.
    held: Option<Box<dyn ToolMachine>>,
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
        f.debug_struct("ToolState")
            .field("active", &self.active)
            .field("stroke", &self.stroke_color)
            .field("fill", &self.fill_color)
            .finish()
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
    /// K-2: step name text → language-neutral source, per language (built on first use).
    step_keys: std::collections::HashMap<
        String,
        std::collections::HashMap<String, crate::history::StepKey>,
    >,
    /// Letzter Filteraufruf (für „Filter wiederholen“).
    pub last_filter: Option<RecordedCommand>,
    /// Rückfrage „Dieser Vorgang konvertiert Vektoren in Bitmaps.“ mit OK beantwortet: der nächste
    /// Filteraufruf (Vorschau oder OK im Filterdialog) rastert gewählte Vektoren im selben
    /// Verlaufsschritt (re 28 L9). Die Oberfläche setzt es zurück, sobald kein Dialog mehr offen ist.
    pub filter_rasterize: bool,
    pub(crate) group_depth: u32,
    /// Q-58: open holds on the tool-attribute follow (a running script, see
    /// [`Session::hold_tool_follow`]).
    pub(crate) follow_hold: u32,
    /// Q-58: (document, generation, selection) the tool attributes last looked at.
    pub(crate) follow_seen: Option<(DocId, u64, Vec<fp_model::ElementId>)>,
    /// Q-58: counts changes of the tool attributes made by the engine (the user interface mirrors
    /// them into its inspector fields when it changes).
    pub(crate) tool_attrs_gen: u64,
    /// RE spec 59 section 1.4: the running command changed the tool attributes as a side effect
    /// that undo does not take back (Arrow Line, Measure Tool); the History row keeps no toolbox
    /// colours then (see [`Session::keep_tool_change_out_of_history`]).
    pub(crate) tool_change_unrecorded: bool,
    /// TW-139: live text view for script read-back: document whose model currently shows the open
    /// text editor's state, with the committed model to put back (see [`Session::begin_live_text_view`]).
    live_text: Option<(DocId, Arc<Document>)>,
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
                held: None,
                slot_memory: BTreeMap::new(),
                stroke_color: stroke,
                fill_color: fill,
                options,
            },
            ui: UiState {
                // View ▸ Tooltips persists as the preference `Tooltips` (re 58 §4). It switches
                // the tooltips of buttons and fields (FB-37), on by default; the coordinate tag at
                // the pointer has its own switch View ▸ Coordinate Tag ([`COORDINATE_TAG_PREF`])
                tooltips: prefs.get_bool("Tooltips", true),
                ..UiState::default()
            },
            registry,
            geometry: default_geometry(),
            renderer: crate::render::default_renderer(),
            script_host: None,
            events: Vec::new(),
            step_keys: std::collections::HashMap::new(),
            last_filter: None,
            filter_rasterize: false,
            group_depth: 0,
            follow_hold: 0,
            follow_seen: None,
            tool_attrs_gen: 0,
            tool_change_unrecorded: false,
            live_text: None,
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
        self.end_live_text_view();
        let i = self
            .docs
            .iter()
            .position(|d| d.id == id)
            .ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-document")))?;
        if self.active != Some(i) {
            self.leave_document();
            self.active = Some(i);
            self.enter_document_tool_fill();
            self.events
                .push(SessionEvent::ActiveDocumentChanged(Some(id)));
        }
        Ok(())
    }

    /// Fügt ein Dokument hinzu und aktiviert es.
    pub fn add_document(&mut self, doc: Document, path: Option<PathBuf>) -> DocId {
        let id = DocId(self.next_doc);
        self.next_doc += 1;
        let title = match &path {
            Some(p) => p
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| fp_i18n::t("msg-default-document-nameless")),
            None => {
                // Vorgabetitel in der Sprache der Oberfläche beim Anlegen (wie die übrigen Standardnamen);
                // Endung .fw.png wie im Original, dessen „.fw.png anhängen“ ab Werk an ist (TW-30)
                self.untitled += 1;
                format!(
                    "{}.fw.png",
                    fp_i18n::tr!("msg-default-document", n = self.untitled.to_string())
                )
            }
        };
        let mut st = DocumentState::new(id, doc, path, title, &self.prefs);
        st.view.viewport = self
            .active()
            .map(|d| d.view.viewport)
            .unwrap_or(st.view.viewport);
        self.leave_document();
        self.docs.push(st);
        self.active = Some(self.docs.len() - 1);
        self.enter_document_tool_fill();
        self.events.push(SessionEvent::DocumentOpened(id));
        self.events
            .push(SessionEvent::ActiveDocumentChanged(Some(id)));
        id
    }

    /// Schließt ein Dokument (ohne Rückfrage; die UI fragt vorher nach `is_dirty`).
    pub fn close_document(&mut self, id: DocId) -> Result<()> {
        self.end_live_text_view();
        let i = self
            .docs
            .iter()
            .position(|d| d.id == id)
            .ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-document")))?;
        if self.active == Some(i) {
            // laufender Werkzeugvorgang gehört zum geschlossenen Dokument (PAR 09/06)
            self.tools.machine = tools::make_machine(self.tools.active);
            self.tools.held = None;
        }
        self.docs.remove(i);
        self.events.push(SessionEvent::DocumentClosed(id));
        self.active = if self.docs.is_empty() {
            None
        } else {
            Some(i.min(self.docs.len() - 1))
        };
        self.enter_document_tool_fill();
        self.events
            .push(SessionEvent::ActiveDocumentChanged(self.active_id()));
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

    /// Aktives Tastenkürzel-Set (Voreinstellung `CurrentShortcutSet`, Rückfall „Web Standard“ wie im Original, BA-127).
    pub fn shortcut_set(&self) -> &'static ShortcutSet {
        static EMPTY: std::sync::OnceLock<ShortcutSet> = std::sync::OnceLock::new();
        shortcuts::any_set(&self.prefs.shortcut_set())
            .or_else(|| shortcuts::builtin_set(crate::prefs::DEFAULT_SHORTCUT_SET))
            .unwrap_or_else(|| EMPTY.get_or_init(ShortcutSet::default))
    }

    /// Kürzel eines Befehls im aktiven Set.
    pub fn command_shortcuts(&self, name: &str) -> Vec<Shortcut> {
        self.registry
            .get(name)
            .map(|c| c.info().shortcuts(self.shortcut_set()).to_vec())
            .unwrap_or_default()
    }

    /// Befehl zu einem Menü-Tastenkürzel im aktiven Set.
    pub fn command_for_shortcut(&self, sc: &Shortcut) -> Option<String> {
        let set = self.shortcut_set();
        set.commands_for(sc)
            .into_iter()
            .find_map(|cmd| self.registry.by_fw_cmd(cmd).map(|c| c.info().name.clone()))
            .or_else(|| {
                // dynamic filter shortcuts (re 47 §1.4) name the Firepixel command directly
                set.dynamic_commands_for(sc)
                    .into_iter()
                    .find_map(|name| self.registry.get(name).map(|c| c.info().name.clone()))
            })
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
            // ein Durchlauf statt einer Suche je ID (PERF-1: Menüs/Werkzeugleisten fragen das je
            // Bild), Abbruch beim ersten entscheidenden Element (BA-099)
            Requirement::AllOf(k) => d.is_some_and(|d| {
                let (mut any, mut all) = (false, true);
                crate::docops::visit_on_page(&d.doc, d.page, &d.selection.elements, &mut |_, e| {
                    any = true;
                    all = crate::command::element_is(e, *k, &d.doc);
                    !all
                });
                any && all
            }),
            Requirement::AnyOf(k) => d.is_some_and(|d| {
                let mut hit = false;
                crate::docops::visit_on_page(&d.doc, d.page, &d.selection.elements, &mut |_, e| {
                    hit = crate::command::element_is(e, *k, &d.doc);
                    hit
                });
                hit
            }),
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
        self.registry.get(name).is_some_and(|c| {
            c.info().implemented && self.unmet(c.as_ref()).is_none() && c.menu_available(self)
        })
    }

    /// Menüzustand eines Befehls: (aktiv, Häkchen).
    pub fn command_state(&self, name: &str) -> (bool, Option<bool>) {
        match self.registry.get(name) {
            Some(c) => (
                c.info().implemented && self.unmet(c.as_ref()).is_none() && c.menu_available(self),
                c.checked(self),
            ),
            None => (false, None),
        }
    }

    // ------------------------------------------------------------ Ausführen

    /// Führt einen Befehl aus (siehe Moduldoku).
    pub fn execute(&mut self, name: &str, params: Value) -> Result<CommandOutput> {
        self.end_live_text_view();
        // FB-31: a menu command applies a paused bitmap stroke first; zooming and scrolling keep
        // the pause
        if !PAUSE_KEEPING_COMMANDS.contains(&name) {
            self.apply_paused_stroke()?;
        }
        let reg = Arc::clone(&self.registry);
        let cmd = reg
            .get(name)
            .ok_or_else(|| EngineError::UnknownCommand(name.into()))?;
        let info = cmd.info();
        if !info.implemented {
            return Err(EngineError::NotImplemented(name.into()));
        }
        // Offener Text-Editor: Textattribute aus Inspektor und Menü „Text“ gehen live an den
        // Editor (Markierung bzw. ganzer Text), der Editor bleibt offen; kein Verlaufsschritt –
        // übernommen wird beim Beenden der Bearbeitung. The `Text_*` defaults are written as
        // without editor (TW-138).
        if let Some(cur) = self.tools.machine.text_edit_attrs()
            && let Some(patch) = crate::commands::editor_patch(name, &params, &cur)
        {
            let ev = ToolEvent {
                input: tools::ToolInput::Edit(tools::EditOp::Attrs(patch.clone())),
                mods: Default::default(),
                pressure: 1.0,
                time_ms: 0,
            };
            self.tool_event(&ev)?;
            crate::commands::store_editor_prefs(self, name, &params, &patch)?;
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
            d.selection.retain_existing(|id| {
                paths
                    .get(&id)
                    .and_then(|p| p.as_ref())
                    .is_none_or(|p| !crate::docops::is_locked(&doc, page, p))
            });
        }
        if let Some(reason) = self.unmet(cmd.as_ref()) {
            return Err(EngineError::Unavailable {
                name: name.into(),
                reason,
            });
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
        if self.group_depth == 0 {
            self.tool_change_unrecorded = false;
        }
        let doc_id = self.active_id();
        let before = self.active().map(|d| d.snapshot());
        let colors_before = (self.tools.stroke_color, self.tools.fill_color);
        let rows_before = self.history_rows();
        let res = cmd.apply(self, &p);
        let mut out = match res {
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
        // Werkzeuge benennen den Verlaufsschritt wie das Original (`step`: Endung eines
        // `msg-step-…`-Schlüssels, z. B. Stift-Klick auf ein Segment → „Stift“, PAR 09/05)
        if let Some(k) = p
            .value
            .get("step")
            .and_then(|v| v.as_str())
            .map(|k| format!("msg-step-{k}"))
            .filter(|k| fp_i18n::has(k))
        {
            out.label = Some(fp_i18n::t(&k));
        }
        // Verlaufsname in der Sprache der Oberfläche, auch wenn der Befehl über die MCP-Brücke in der
        // Antwortsprache der Agenten lief (CT-1b N1), dazu die sprachneutrale Quelle (K-2): erkannt
        // in der Sprache, in der der Befehl den Namen vergab
        let (step_name, step_key) = if out.effect == Effect::Document {
            match &out.label {
                Some(l) if *l != info.step_label() => match self.step_key_current(l) {
                    Some(k) => (
                        fp_i18n::with_ui_language(|| self.key_text(&k))
                            .unwrap_or_else(|| fp_i18n::step_label_in_ui_language(l)),
                        Some(k),
                    ),
                    None => (fp_i18n::step_label_in_ui_language(l), None),
                },
                _ => (
                    fp_i18n::with_ui_language(|| info.step_label()),
                    Some(crate::history::StepKey::Command(name.into())),
                ),
            }
        } else {
            (String::new(), None)
        };
        if name.starts_with("filter.") && name != "filter.repeat" {
            self.last_filter = Some(RecordedCommand {
                command: name.into(),
                params: p.value.clone(),
            });
        }
        match out.effect {
            Effect::Document => {
                if let (Some(id), Some(b)) = (doc_id, before) {
                    self.finish_step(id, b, step_name, step_key, name, &p.value);
                    self.note_tool_colors(rows_before, colors_before);
                }
            }
            Effect::View => {
                if let Some(id) = self.active_id() {
                    self.events.push(SessionEvent::ViewChanged(id));
                }
            }
            Effect::Session | Effect::None => {}
        }
        // re 28 L3: Arbeit im Bitmap-Modus merken (Auswahlrahmen, Löschen, Malen …); beim Verlassen
        // des Modus wird die Bitmap dann beschnitten
        if name != "modify.exitBitmapMode"
            && let Some(d) = self.active_mut()
            && (d.bitmap_mode
                || d.selection.has_pixels()
                || matches!(name, "bitmap.paint" | "bitmap.erase" | "bitmap.retouch"))
            && (d.selection.has_pixels() || out.effect == Effect::Document)
        {
            for (_, e) in crate::commands::edit::selected_bitmaps(d) {
                if !d.bitmap_worked.contains(&e.id) {
                    d.bitmap_worked.push(e.id);
                }
            }
        }
        if self.group_depth > 0 {
            self.group_log.push(RecordedCommand {
                command: name.into(),
                params: p.value.clone(),
            });
        }
        // Auswahlverlauf (Q-21): jede neue Objektauswahl ist ein Eintrag; „Vorherige/Nächste
        // Auswahl“ selbst bewegen sich nur darin
        if !matches!(name, "select.next" | "select.previous")
            && let Some(d) = self.active_mut()
        {
            let sel = d.selection.elements.clone();
            d.selection_history.note(&sel);
        }
        self.events.push(SessionEvent::CommandExecuted(name.into()));
        if self.group_depth == 0 && out.effect == Effect::Document {
            self.sync_tool();
        }
        // Q-58: the tool attributes follow the selection once the command is done
        self.follow_selection();
        Ok(out)
    }

    /// Lets the active tool follow a document change it did not make itself
    /// ([`ToolMachine::sync`]; e.g. the pen after undoing its last point, FB-07).
    fn sync_tool(&mut self) {
        self.with_tool_ctx(|m, ctx| m.sync(ctx));
    }

    /// Führt einen Befehl innerhalb eines anderen aus, ohne eigenen Verlaufsschritt (der äußere Befehl
    /// legt den Schritt an; z. B. Rastern vor dem Filter, re 28 L9).
    pub(crate) fn apply_inline(&mut self, name: &str, params: Value) -> Result<CommandOutput> {
        let reg = Arc::clone(&self.registry);
        let cmd = reg
            .get(name)
            .ok_or_else(|| EngineError::UnknownCommand(name.into()))?;
        let p = Params::new(name, params);
        cmd.apply(self, &p)
    }

    /// Ändert ein (auch nicht aktives) Dokument als eigenen Verlaufsschritt.
    pub(crate) fn modify_document(
        &mut self,
        id: DocId,
        label: &str,
        command: &str,
        f: impl FnOnce(&mut DocumentState) -> Result<()>,
    ) -> Result<()> {
        let i = self
            .docs
            .iter()
            .position(|d| d.id == id)
            .ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-document")))?;
        let before = self.docs[i].snapshot();
        if let Err(e) = f(&mut self.docs[i]) {
            self.docs[i].restore(before);
            return Err(e);
        }
        let key = self.step_key(label);
        self.finish_step(id, before, label.into(), key, command, &Value::Null);
        Ok(())
    }

    fn finish_step(
        &mut self,
        id: DocId,
        before: Snapshot,
        label: String,
        key: Option<crate::history::StepKey>,
        name: &str,
        params: &Value,
    ) {
        let grouping = self.group_depth > 0;
        let renderer = self.renderer.clone();
        let Some(d) = self.docs.iter_mut().find(|d| d.id == id) else {
            return;
        };
        if let Some(r) = &renderer
            && !Arc::ptr_eq(&before.doc, &d.doc)
        {
            refresh_text_frames(r.as_ref(), &before.doc, &mut d.doc);
        }
        let page = d.page;
        let doc = Arc::clone(&d.doc);
        let paths = selection_paths(&doc, page, &d.selection);
        d.selection
            .retain_existing(|eid| paths.get(&eid).is_some_and(Option::is_some));
        // RE spec 64 §3 rule 4 (see `follow_single_selection`); a group step was already handled
        // by its inner commands (a script may set the layer after selecting). The change writes
        // no History row of its own (rule 6): `loc_changed` alone never adds a row, and `SILENT`
        // selection commands stay out of the History.
        if name != "history.replay" && before.selection.elements != d.selection.elements {
            follow_single_selection(d, &paths);
        }
        let doc_changed = !Arc::ptr_eq(&before.doc, &d.doc);
        let sel_changed = before.selection != d.selection;
        let loc_changed =
            before.page != d.page || before.frame != d.frame || before.layer != d.layer;
        let forced = std::mem::take(&mut self.force_step) && !grouping;
        if !doc_changed && !sel_changed && !loc_changed && !forced {
            return;
        }
        if doc_changed {
            d.generation += 1;
            self.events.push(SessionEvent::DocumentChanged {
                doc: id,
                generation: d.generation,
            });
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
        const SILENT: &[&str] = &[
            "select.set",
            "select.pointAt",
            "select.inRect",
            "select.points",
            "select.next",
            "select.previous",
        ];
        if !doc_changed && SILENT.contains(&name) {
            return;
        }
        let now = d.snapshot();
        // Arrow keys: consecutive key nudges of an unchanged selection are one history row with
        // the summed offset, across directions and Shift (measured, TW-148: Shift+Right twice and
        // Shift+Down → one row moveSelectionBy({x:20, y:10})). A selection change, another step or
        // an undo in between starts a new row; script calls (`edit.move` without `nudge`,
        // `edit.nudge.*`) stay one row each. A nudge of the marquee alone (bitmap selection
        // tool, re 62 §1.8) changes only the pixel selection and merges the same way.
        if (doc_changed || sel_changed)
            && is_key_nudge(name, params)
            && let Some(top) = d.history.last_mergeable()
            && is_key_nudge(&top.command, &top.params)
            && top.params.get("copy") == params.get("copy")
            && Arc::ptr_eq(&top.after.doc, &before.doc)
            && top.after.selection == before.selection
            && top.after.page == before.page
            && top.after.frame == before.frame
        {
            let sum = |k: &str| {
                top.params.get(k).and_then(|v| v.as_f64()).unwrap_or(0.0)
                    + params.get(k).and_then(|v| v.as_f64()).unwrap_or(0.0)
            };
            let (dx, dy) = (sum("dx"), sum("dy"));
            if let Some(o) = top.params.as_object_mut() {
                o.insert("dx".into(), serde_json::json!(dx));
                o.insert("dy".into(), serde_json::json!(dy));
            }
            top.after = now;
            self.events.push(SessionEvent::HistoryChanged(id));
            return;
        }
        // Ziehen über die Ebenen-Augen ist im Original EIN Verlaufsschritt (TW-69 Rest): die
        // Fortsetzungen eines Zugs (`dragContinue`) gehen in den Schritt seiner ersten Zeile ein.
        if doc_changed
            && name == "layer.setVisible"
            && params.get("dragContinue").and_then(|v| v.as_bool()) == Some(true)
            && let Some(top) = d.history.last_mergeable()
            && top.command == "layer.setVisible"
            && Arc::ptr_eq(&top.after.doc, &before.doc)
            && top.after.page == before.page
            && top.after.frame == before.frame
        {
            top.after = now;
            self.events.push(SessionEvent::HistoryChanged(id));
            return;
        }
        if doc_changed || sel_changed || forced {
            let after = now;
            // a step without a name reads "Command script" like the original (TW-137)
            let (label, key) = if label.trim().is_empty() {
                (
                    fp_i18n::with_ui_language(|| fp_i18n::t(STEP_COMMAND_SCRIPT)),
                    Some(crate::history::StepKey::Text(STEP_COMMAND_SCRIPT.into())),
                )
            } else {
                (label, key)
            };
            d.history.push(crate::history::HistoryStep {
                label,
                key,
                command: name.into(),
                params: params.clone(),
                recordable: true,
                before,
                after,
            });
            self.events.push(SessionEvent::HistoryChanged(id));
        }
    }

    /// Rows of the active document's History including the dropped ones (to see whether a command
    /// added a row).
    fn history_rows(&self) -> usize {
        self.active()
            .map(|d| d.history.len() + d.history.dropped())
            .unwrap_or(0)
    }

    /// A new History row whose command changed the toolbox colours (Swap Colours, TW-186) keeps
    /// them before and after, so undo, redo and History jumps restore them like the objects.
    fn note_tool_colors(&mut self, rows_before: usize, before: (Argb, Argb)) {
        if std::mem::take(&mut self.tool_change_unrecorded) {
            return;
        }
        let now = (self.tools.stroke_color, self.tools.fill_color);
        if now == before || self.history_rows() <= rows_before {
            return;
        }
        if let Some(top) = self.active_mut().and_then(|d| d.history.top_mut()) {
            top.before.tool_colors = Some(before);
            top.after.tool_colors = Some(now);
        }
    }

    /// The running command changes the tool attributes in a way undo does not take back (measured
    /// for the auto shapes Arrow Line and Measure Tool, RE spec 59 section 1.4): its History row
    /// keeps no toolbox colours, so Undo, Redo and History jumps leave the tool as it is.
    pub fn keep_tool_change_out_of_history(&mut self) {
        self.tool_change_unrecorded = true;
    }

    /// Der laufende Befehl legt auch ohne Änderung einen Verlaufsschritt an.
    pub(crate) fn force_history_step(&mut self) {
        self.force_step = true;
    }

    /// Führt mehrere Befehle als **einen** Verlaufsschritt aus (wie ein Skript im Original,
    /// Laufzeit T6). Bei einem Fehler wird alles zurückgenommen.
    pub fn run_group(
        &mut self,
        label: &str,
        f: impl FnOnce(&mut Session) -> Result<()>,
    ) -> Result<()> {
        self.end_live_text_view();
        if self.group_depth == 0 {
            self.tool_change_unrecorded = false;
        }
        let doc_id = self.active_id();
        let before = self.active().map(|d| d.snapshot());
        let colors_before = (self.tools.stroke_color, self.tools.fill_color);
        let rows_before = self.history_rows();
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
            let key = self.step_key(label);
            self.finish_step(id, b, label.into(), key, "history.replay", &params);
            self.note_tool_colors(rows_before, colors_before);
            self.sync_tool();
        }
        if outer {
            self.follow_selection();
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

    /// FB-31 (Firepixel extension): state of a pausable bitmap stroke of the active tool (Brush,
    /// Pencil, Eraser, the retouch tools): the user interface sends [`tools::ToolKey::Pause`] for
    /// the P key only while it is [`tools::StrokePause::Painting`] and shows the status text.
    pub fn tool_stroke_pause(&self) -> tools::StrokePause {
        self.tools.machine.stroke_pause()
    }

    /// FB-31: applies a paused bitmap stroke (one history row, as on release); no-op otherwise.
    pub fn apply_paused_stroke(&mut self) -> Result<()> {
        if self.tools.machine.stroke_pause() == tools::StrokePause::Paused {
            self.finish_tool()?;
        }
        Ok(())
    }

    /// Rückgängig im aktiven Dokument.
    pub fn undo(&mut self) -> Result<()> {
        self.end_live_text_view();
        // FB-31: a paused stroke is applied first, then undone like any stroke
        self.apply_paused_stroke()?;
        // a Vector Path stroke with the mouse button held ignores Ctrl+Z (FB-07)
        if self.tools.machine.blocks_history() {
            return Ok(());
        }
        let d = self.doc_mut()?;
        // RE spec 65 rule 1.6: undoing "move the selection to a layer" restores the objects and
        // the selection; the target stays current unless the selection is a single object (its
        // layer, RE spec 64 rule 4)
        let keep_layer = d
            .history
            .top()
            .filter(|t| t.command == crate::commands::MOVE_SELECTION_TO_LAYER)
            .map(|t| t.after.layer);
        let mut snap = d
            .history
            .undo()
            .ok_or(EngineError::History("err-req-can-undo"))?;
        if let Some(l) = keep_layer {
            snap.layer = l;
        }
        self.apply_history_snapshot(snap);
        if keep_layer.is_some()
            && let Some(d) = self.active_mut()
        {
            follow_single_selection_now(d);
        }
        Ok(())
    }

    /// K-2: language-neutral source of a step name given in the user interface language (groups,
    /// replays, documents changed by name). See [`Session::step_key_current`].
    fn step_key(&mut self, label: &str) -> Option<crate::history::StepKey> {
        fp_i18n::with_ui_language(|| self.step_key_current(label))
    }

    /// K-2: language-neutral source of a step name given in the current language: a command's step
    /// name (e.g. „Animate“ is named like „Convert to Symbol“) or a `msg-step-…`/`tool-…` text.
    /// `None` for own names (groups, scripts) and texts with arguments: those stay as stored.
    fn step_key_current(&mut self, label: &str) -> Option<crate::history::StepKey> {
        use crate::history::StepKey;
        let lang = fp_i18n::current_language();
        let registry = Arc::clone(&self.registry);
        let map = self.step_keys.entry(lang.clone()).or_insert_with(|| {
            let mut m = std::collections::HashMap::new();
            let keys = fp_i18n::keys(&lang);
            for k in keys.iter().filter(|k| k.starts_with("msg-step-")) {
                m.entry(fp_i18n::t(k))
                    .or_insert_with(|| StepKey::Text(k.clone()));
            }
            for c in registry.iter() {
                m.entry(c.info().step_label())
                    .or_insert_with(|| StepKey::Command(c.info().name.clone()));
            }
            for k in keys.iter().filter(|k| k.starts_with("tool-")) {
                m.entry(fp_i18n::t(k).replace('&', ""))
                    .or_insert_with(|| StepKey::Text(k.clone()));
            }
            m
        });
        map.get(label).cloned()
    }

    /// Text of a step source in the current language (`None`: command or key unknown).
    fn key_text(&self, key: &crate::history::StepKey) -> Option<String> {
        use crate::history::StepKey;
        match key {
            StepKey::Command(n) => self.registry.get(n).map(|c| c.info().step_label()),
            StepKey::Text(k) => fp_i18n::has(k).then(|| fp_i18n::t(k).replace('&', "")),
        }
    }

    /// K-2: name of a history step in the current language (panel: user interface language; MCP:
    /// response language). Steps without a source show their stored name.
    pub fn step_label(&self, step: &crate::history::HistoryStep) -> String {
        step.key
            .as_ref()
            .and_then(|k| self.key_text(k))
            .unwrap_or_else(|| step.label.clone())
    }

    /// Labels for "Undo: …" and "Redo: …" in the Edit menu: the last step and the undone step. Without
    /// an undone step there is no redo label ("Can't Redo", tester feedback FB-08); the last command
    /// is never repeated.
    pub fn undo_redo_labels(&self) -> (Option<String>, Option<String>) {
        let d = self.active();
        let undo = d.and_then(|d| d.history.undo_label().map(str::to_string));
        let redo = d.and_then(|d| d.history.redo_label().map(str::to_string));
        (undo, redo)
    }

    /// The step Edit ▸ Redo repeats when nothing is undone (TW-173, re 55 §7.6): the last history
    /// row, if it is repeatable and the last change of the position was not a redo. `None` while a
    /// redo is available.
    pub fn repeat_step(&self) -> Option<&crate::history::HistoryStep> {
        let h = &self.active()?.history;
        if h.can_redo() || h.after_redo() {
            return None;
        }
        h.top()
            .filter(|st| self.repeatable_command(&st.command, &st.params, true))
    }

    /// Name of the step Edit ▸ Redo would repeat ("Repeat: <step>", TW-173), in the current
    /// language.
    pub fn repeat_label(&self) -> Option<String> {
        self.repeat_step().map(|st| self.step_label(st))
    }

    /// Can Edit ▸ Redo repeat the last step (nothing undone, TW-173)?
    pub fn can_repeat(&self) -> bool {
        self.repeat_step().is_some()
    }

    /// Repeatable history row (TW-173, re 55 §7.6): a step with recorded parameters that can be run
    /// again — every command step and every group of such steps (`history.replay`), as the History
    /// can replay them. Not repeatable: bitmap strokes (brush, pencil, eraser and the retouch tools;
    /// their rows have no script counterpart in the original), steps without recorded parameters
    /// and commands that are unavailable now (`top` only: the outer step is checked against the
    /// current state).
    fn repeatable_command(&self, command: &str, params: &Value, top: bool) -> bool {
        if !params.is_object() || NOT_REPEATABLE.contains(&command) {
            return false;
        }
        let Some(c) = self.registry.get(command) else {
            return false;
        };
        if !c.info().implemented || (top && self.unmet(c.as_ref()).is_some()) {
            return false;
        }
        if command == "history.replay" {
            return params
                .get("commands")
                .and_then(Value::as_array)
                .is_some_and(|cs| {
                    !cs.is_empty()
                        && cs.iter().all(|c| {
                            match (c.get("command").and_then(Value::as_str), c.get("params")) {
                                (Some(n), Some(p)) => self.repeatable_command(n, p, false),
                                _ => false,
                            }
                        })
                });
        }
        true
    }

    /// Edit ▸ Redo without an undone step: runs the last history row again as a new row with the
    /// same command and parameters and the same name (TW-173, re 55 §7.6: after Clone a second
    /// clone, after a move the same move, after a pen point the same point appended again). A
    /// repeated row is itself repeatable.
    pub fn repeat_last(&mut self) -> Result<()> {
        self.end_live_text_view();
        // like undo/redo: ignored while a Vector Path stroke holds the mouse button (FB-07)
        if self.tools.machine.blocks_history() {
            return Ok(());
        }
        let step = self
            .repeat_step()
            .ok_or(EngineError::History("err-req-can-redo"))?;
        let (label, key) = (step.label.clone(), step.key.clone());
        let (command, params) = repeat_invocation(step);
        let rows = |s: &Session| {
            s.active()
                .map(|d| d.history.len() + d.history.dropped())
                .unwrap_or(0)
        };
        let n0 = rows(self);
        self.execute(&command, params)?;
        if rows(self) > n0
            && let Some(top) = self.active_mut().and_then(|d| d.history.top_mut())
        {
            top.label = label;
            top.key = key;
        }
        Ok(())
    }

    /// Wiederholen im aktiven Dokument.
    pub fn redo(&mut self) -> Result<()> {
        self.end_live_text_view();
        self.apply_paused_stroke()?;
        if self.tools.machine.blocks_history() {
            return Ok(());
        }
        let d = self.doc_mut()?;
        let snap = d
            .history
            .redo()
            .ok_or(EngineError::History("err-req-can-redo"))?;
        self.apply_history_snapshot(snap);
        Ok(())
    }

    /// Animation playback (FB-63): shows state `index` of the active document like
    /// `state.setCurrent` (current layer per RE spec 65 rule 3, selection cleared, the Pages and
    /// States panels follow), but as a view change: no History row, no paused stroke applied.
    /// `false` if there is no document, the index is out of range or already current.
    pub fn show_state(&mut self, index: usize) -> bool {
        self.end_live_text_view();
        let Some(d) = self.active_mut() else {
            return false;
        };
        if index >= d.frame_count() || d.frame == index {
            return false;
        }
        let id = d.id;
        d.frame = index;
        d.adopt_resolved_layer();
        let had_selection = !d.selection.is_empty();
        d.selection.clear();
        if had_selection {
            self.events.push(SessionEvent::SelectionChanged(id));
        }
        self.events.push(SessionEvent::LocationChanged(id));
        true
    }

    /// Springt im Verlauf (Klick im Verlauf-Bedienfeld).
    pub fn history_go_to(&mut self, position: usize) -> Result<()> {
        self.apply_paused_stroke()?;
        let d = self.doc_mut()?;
        if let Some(snap) = d.history.go_to(position) {
            self.apply_history_snapshot(snap);
        }
        Ok(())
    }

    fn apply_history_snapshot(&mut self, mut snap: Snapshot) {
        let tool_colors = snap.tool_colors.take();
        self.restore_history_snapshot(snap);
        // the pen follows its path: undoing a point keeps drawing from the new last point (FB-07)
        self.sync_tool();
        // the toolbox follows the restored selection; the colours stored with the row (Swap
        // Colours, TW-186) only come back when no selected object supplies them (RE spec 58)
        self.follow_selection_or(tool_colors);
    }

    fn restore_history_snapshot(&mut self, snap: Snapshot) {
        let Some(d) = self.active_mut() else { return };
        // Das Raster gehört wie im Original nicht zum Verlauf (PM-5 D18: „Raster bearbeiten“ legt
        // keinen Schritt an, Strg+Z ändert das Raster nicht) – es bleibt beim Springen im Verlauf
        // stehen, solange die Seiten dieselben sind.
        let grids: Vec<Option<fp_model::Grid>> =
            d.doc.pages.iter().map(|p| p.root.grid.clone()).collect();
        let doc_before = Arc::clone(&d.doc);
        let loc_before = (d.page, d.frame, d.layer);
        d.restore(snap);
        if d.doc.pages.len() == grids.len()
            && d.doc
                .pages
                .iter()
                .zip(&grids)
                .any(|(p, g)| p.root.grid != *g)
        {
            let doc = Arc::make_mut(&mut d.doc);
            for (p, g) in doc.pages.iter_mut().zip(grids) {
                if p.root.grid != g {
                    Arc::make_mut(p).root.grid = g;
                }
            }
        }
        // steps that leave the document untouched (Copy, Select All …) do not mark it as changed
        // when they are undone or redone (TW-137)
        if Arc::ptr_eq(&doc_before, &d.doc) && loc_before == (d.page, d.frame, d.layer) {
            let id = d.id;
            self.events.push(SessionEvent::SelectionChanged(id));
            self.events.push(SessionEvent::HistoryChanged(id));
            return;
        }
        d.generation += 1;
        let (id, g) = (d.id, d.generation);
        self.events.push(SessionEvent::DocumentChanged {
            doc: id,
            generation: g,
        });
        self.events.push(SessionEvent::SelectionChanged(id));
        self.events.push(SessionEvent::HistoryChanged(id));
    }

    // ------------------------------------------------------------ Werkzeuge

    /// Wechselt das Werkzeug; ein laufender Vorgang des alten Werkzeugs wird abgeschlossen
    /// (z. B. Text übernehmen). Liefert die Antwort des alten Werkzeugs.
    pub fn set_tool(&mut self, id: ToolId) -> Result<ToolResponse> {
        self.end_live_text_view();
        let mut resp = ToolResponse::default();
        if id != self.tools.active {
            let old_resp = self.with_tool_ctx(|m, ctx| m.finish(ctx));
            resp = self.run_tool_response(old_resp)?;
            // M6 §4a: Wechsel vom Pinsel/Bleistift zu einem Werkzeug außerhalb des Bitmap-Abschnitts
            // verlässt den Malmodus wie Esc (`exitPaintMode`, eigener Verlaufsschritt) und
            // beschneidet die Bitmap sofort auf ihren Pixelinhalt; ebenso vom Radiergummi
            // (re 37 §5a).
            let paint_tool = matches!(
                self.tools.active,
                ToolId::Brush | ToolId::Pencil | ToolId::Eraser
            );
            let leaves_bitmap = tools::info(id).is_none_or(|i| i.section != "Bitmap");
            let painted = self
                .active()
                .is_some_and(|d| d.bitmap_mode || !d.bitmap_worked.is_empty());
            if paint_tool && leaves_bitmap && painted {
                self.execute("modify.exitBitmapMode", serde_json::json!({"step": true}))?;
            }
            self.tools.active = id;
            self.tools.machine = tools::make_machine(id);
            self.with_tool_ctx(|m, ctx| m.activated(ctx));
            if let Some(g) = tools::info(id).map(|i| i.flyout) {
                self.tools.slot_memory.insert(g.to_string(), id);
            }
            self.events.push(SessionEvent::ToolChanged(id));
            // R5 (re 56 section 3): activating the text tool resets its paragraph defaults
            if id == ToolId::Text {
                crate::commands::reset_text_tool_paragraph_defaults(self);
            }
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
        self.with_machine_ctx(false, f)
    }

    /// Like [`Session::with_tool_ctx`]; `held` = the machine that finishes its gesture after a
    /// tool change (falls back to the active machine without one).
    fn with_machine_ctx<R>(
        &mut self,
        held: bool,
        f: impl FnOnce(&mut dyn ToolMachine, &ToolContext) -> R,
    ) -> R {
        // Felder einzeln leihen (kein Klonen von Auswahl/Pixelmaske je Ereignis bzw. Bild)
        let empty_doc;
        let empty_sel;
        let empty_view;
        let active = self.active.and_then(|i| self.docs.get(i));
        let (doc, page, frame, sel, zoom, bitmap_mode, view): (
            &Document,
            usize,
            usize,
            &Selection,
            f64,
            bool,
            &ViewState,
        ) = match active {
            Some(d) => (
                &d.doc,
                d.page,
                d.frame,
                &d.selection,
                d.view.zoom,
                d.bitmap_mode,
                &d.view,
            ),
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
            layer: active.and_then(|d| d.layer),
            selection: sel,
            zoom,
            prefs: &self.prefs,
            options: &self.tools.options,
            geometry: self.geometry.as_ref(),
            stroke_color: self.tools.stroke_color,
            fill_color: self.tools.fill_color,
            bitmap_mode,
            bitmap_worked: active.is_some_and(|d| !d.bitmap_worked.is_empty()),
            view,
            renderer: self.renderer.as_deref(),
        };
        let m: &mut dyn ToolMachine = match self.tools.held.as_deref_mut() {
            Some(h) if held => h,
            _ => self.tools.machine.as_mut(),
        };
        f(m, &ctx)
    }

    /// Rückmeldung des aktiven Werkzeugs für den aktuellen Zustand (auch ohne Ereignis, z. B.
    /// Transformationsrahmen, Pfadpunkte der Auswahl, Text-Editor).
    pub fn tool_feedback(&mut self) -> Vec<tools::Overlay> {
        let mut v = if self.tools.held.is_some() {
            self.with_machine_ctx(true, |m, ctx| m.feedback(ctx))
        } else {
            Vec::new()
        };
        v.extend(self.with_tool_ctx(|m, ctx| m.feedback(ctx)));
        v
    }

    /// Läuft im aktiven Werkzeug ein Text-Editor (Tasten an das Werkzeug statt an Menükürzel)?
    pub fn tool_is_editing_text(&self) -> bool {
        self.tools.machine.is_editing_text()
    }

    /// F28 (Q-50): what the active tool's running press hit (`ToolMachine::press_hit`).
    pub fn tool_press_hit(&self) -> Option<&'static str> {
        self.tools.machine.press_hit()
    }

    /// Rahmen, den das aktive Werkzeug gerade aufzieht (Inspektor B/H/X/Y live, PAR 08/17, 10/15).
    pub fn tool_live_rect(&self) -> Option<fp_geom::Rect> {
        self.tools.machine.live_rect()
    }

    /// Soll der Inspektor während des laufenden Zugs den Zustand vor dem Druck zeigen (PAR 08/21)?
    pub fn tool_inspector_hold(&self) -> bool {
        self.tools.machine.inspector_hold()
    }

    /// Vor einem Dokumentwechsel: laufenden Werkzeugvorgang im bisherigen Dokument abschließen und
    /// das Werkzeug zurücksetzen – ein offener Stiftpfad wandert nicht ins nächste Dokument (PAR 09/06).
    fn leave_document(&mut self) {
        if self.active().is_some() {
            let _ = self.finish_tool();
            self.note_document_tool_fill();
        }
        self.tools.machine = tools::make_machine(self.tools.active);
        self.tools.held = None;
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
        let v = self
            .tools
            .machine
            .overlay()
            .into_iter()
            .find_map(|o| match o {
                tools::Overlay::TextEdit(v) => Some(v),
                _ => None,
            })?;
        let e = v.preview_element(&d.doc)?;
        let fp_model::element::ElementKind::Text(t) = &e.kind else {
            return None;
        };
        self.renderer.as_ref()?.text_frame(&d.doc, t)
    }

    /// TW-139: while the text tool edits, put the editor's live state into the active document's
    /// model for read-back (scripts: element list, `textChars`, `pixelRect`), as the original shows
    /// the new text object from the first click and the typed text at once. The edited object gets
    /// the editor's text and formatting; a new text is inserted at the top of the active layer as
    /// `draw.text` would. No history step, nothing marked dirty: the committed model is kept and put
    /// back by [`Session::end_live_text_view`], which every changing entry point (commands, groups,
    /// undo/redo, tool events, document switches) calls first. Returns whether a view is active.
    pub fn begin_live_text_view(&mut self) -> bool {
        self.end_live_text_view();
        let Some(v) = self
            .tools
            .machine
            .overlay()
            .into_iter()
            .find_map(|o| match o {
                tools::Overlay::TextEdit(v) => Some(v),
                _ => None,
            })
        else {
            return false;
        };
        let Some(orig) = self.active().map(|d| Arc::clone(&d.doc)) else {
            return false;
        };
        let Some(mut e) = v.preview_element(&orig) else {
            return false;
        };
        // TW-143: the element carries the editor's current layout bounds (as the commit sets
        // them), not the bounds of the committed text
        if let fp_model::element::ElementKind::Text(t) = &mut e.kind
            && let Some(f) = self.renderer.as_ref().and_then(|r| r.text_frame(&orig, t))
        {
            t.bounds = f;
        }
        let Some(d) = self.active_mut() else {
            return false;
        };
        let mut doc = (*orig).clone();
        let placed = match v.target {
            Some(t) => {
                crate::docops::edit_by_id(&mut doc, d.page, fp_model::ElementId(t), |x| *x = e)
                    .is_some()
            }
            None => {
                e.id = doc.ids.element();
                crate::docops::insert_element(&mut doc, d.page, d.frame, d.layer, 0, Arc::new(e))
                    .is_some()
            }
        };
        if !placed {
            return false;
        }
        d.doc = Arc::new(doc);
        self.live_text = Some((d.id, orig));
        true
    }

    /// Ends the live text view of [`Session::begin_live_text_view`]: the document gets its
    /// committed model back. No-op without a view.
    pub fn end_live_text_view(&mut self) {
        if let Some((id, orig)) = self.live_text.take()
            && let Some(d) = self.docs.iter_mut().find(|d| d.id == id)
        {
            d.doc = orig;
        }
    }

    /// Angezeigte Grenzen des Texts im offenen Editor wie im Original (Rahmen + 2 px Rand je Seite,
    /// `docops::TEXT_MARGIN`) – für B/H/X/Y im Inspektor und das Info-Bedienfeld während der Bearbeitung.
    pub fn text_edit_bounds(&self) -> Option<fp_geom::Rect> {
        self.text_edit_frame()
            .map(|f| crate::docops::text_visible_frame(&f))
    }

    /// Laufenden Werkzeugvorgang abschließen (vor einem Menübefehl, 10 §2.2 „Werkzeug vs.
    /// Menübefehl“), z. B. Text übernehmen oder einen offenen Stiftpfad beenden.
    pub fn finish_tool(&mut self) -> Result<ToolResponse> {
        self.end_live_text_view();
        let resp = self.with_tool_ctx(|m, ctx| m.finish(ctx));
        self.run_tool_response(resp)
    }

    fn run_tool_response(&mut self, mut resp: ToolResponse) -> Result<ToolResponse> {
        let cmds: Vec<Invocation> = std::mem::take(&mut resp.commands);
        let mut first_err = None;
        // rows made so far (counting rows dropped by `MaxUndos`), for `undo_after`
        let rows = |s: &Session| {
            s.active()
                .map(|d| (d.id, d.history.dropped() + d.history.undo_count()))
        };
        let mark = if resp.undo_after { rows(self) } else { None };
        match resp.group.take() {
            Some(label) if !cmds.is_empty() && self.active().is_some() => {
                // one history step for all commands; an error takes everything back
                let mut results = Vec::new();
                if let Err(e) = self.run_group(&label, |s| {
                    for c in &cmds {
                        results.push(s.execute(&c.command, c.params.clone())?.value);
                    }
                    Ok(())
                }) {
                    first_err = Some(e);
                }
                results.resize(cmds.len(), Value::Null);
                resp.results.extend(results);
            }
            _ => {
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
            }
        }
        resp.commands = cmds;
        let keep = std::mem::take(&mut resp.keep_gesture);
        if first_err.is_none()
            && let Some(m) = mark
            && rows(self).is_some_and(|r| r.0 == m.0 && r.1 > m.1)
            && let Err(e) = self.undo()
        {
            first_err = Some(e);
        }
        if let Some(t) = resp.switch_tool.take() {
            let activates_text = t == ToolId::Text && self.tools.active != ToolId::Text;
            self.tools.active = t;
            let old = std::mem::replace(&mut self.tools.machine, tools::make_machine(t));
            if keep {
                self.tools.held = Some(old);
            }
            if let Some(g) = tools::info(t).map(|i| i.flyout) {
                self.tools.slot_memory.insert(g.to_string(), t);
            }
            self.events.push(SessionEvent::ToolChanged(t));
            if activates_text {
                crate::commands::reset_text_tool_paragraph_defaults(self);
            }
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
        self.end_live_text_view();
        if self.tools.held.is_some()
            && let tools::ToolInput::Pointer { kind, .. } = &ev.input
        {
            match kind {
                // the previous tool finishes its gesture (re 62 §3.4)
                tools::PointerKind::Move | tools::PointerKind::Up => {
                    let up = *kind == tools::PointerKind::Up;
                    let mut resp = self.with_machine_ctx(true, |m, ctx| m.handle(ev, ctx));
                    resp.switch_tool = None;
                    resp.keep_gesture = false;
                    resp.replay.clear();
                    if up {
                        self.tools.held = None;
                    }
                    return self.run_tool_response(resp);
                }
                // a new press: the release of the old gesture was lost
                _ => self.tools.held = None,
            }
        }
        let resp = self.with_tool_ctx(|m, ctx| m.handle(ev, ctx));
        self.run_tool_response(resp)
    }

    /// Aktuelle Vorschau des aktiven Werkzeugs (und eines Werkzeugs, das seinen Zug noch beendet).
    pub fn tool_overlay(&self) -> Vec<tools::Overlay> {
        let mut v = self
            .tools
            .held
            .as_ref()
            .map(|h| h.overlay())
            .unwrap_or_default();
        v.extend(self.tools.machine.overlay());
        v
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
