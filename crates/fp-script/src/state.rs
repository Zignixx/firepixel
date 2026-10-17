//! Laufzeitzustand eines Skriptlaufs (thread-lokal): Sitzung, Handle-Tabelle der Wrapper-Objekte,
//! offene Dateireferenzen, Ausgaben.
//!
//! Regel: Der Zustand wird nur kurz geliehen ([`with_state`]) und **nie** während JS-Code läuft
//! (Getter/Setter rufen wieder hinein). So gibt es keine verschachtelten Ausleihen.

use crate::ScriptOptions;
use crate::bindings::hr;
use fp_engine::{DocId, EngineError, Session};
use fp_model::ElementId;
use serde_json::Value;
use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;

/// Was ein Wrapper-Objekt bezeichnet.
#[derive(Clone, Debug, PartialEq)]
pub enum Handle {
    /// Einzelobjekte (`fw`, `Files`, …): nur Klassenname.
    Singleton,
    Document(DocId),
    Element {
        doc: DocId,
        id: ElementId,
    },
    /// Ebene `slot` der Ebenenliste (unten → oben) einer Seite; `sub` = Unterebene (Element-ID) darin.
    Layer {
        doc: DocId,
        page: usize,
        slot: usize,
        sub: Option<ElementId>,
    },
    Frame {
        doc: DocId,
        page: usize,
        frame: usize,
    },
    /// Inhalt einer Ebene in einem Status (FrameNLayerIntersection).
    Cel {
        doc: DocId,
        page: usize,
        slot: usize,
        sub: Option<ElementId>,
        frame: usize,
    },
    Contour {
        doc: DocId,
        id: ElementId,
        c: usize,
    },
    Node {
        doc: DocId,
        id: ElementId,
        c: usize,
        n: usize,
    },
    /// Lebende Knotenliste einer Kontur.
    Nodes {
        doc: DocId,
        id: ElementId,
        c: usize,
    },
    /// Lebende Konturliste eines Pfads.
    Contours {
        doc: DocId,
        id: ElementId,
    },
    /// Lebendes customData eines Elements.
    CustomData {
        doc: DocId,
        id: ElementId,
    },
    /// Geöffnete Datei (`Files.open`).
    FileRef(usize),
    /// Suchen-und-Ersetzen-Objekt (`dom.makeFind`), Index in [`State::finds`].
    Find(usize),
}

/// Geöffnete Datei.
#[derive(Debug)]
pub struct FileRefState {
    pub path: PathBuf,
    pub write: bool,
    /// Gelesener Inhalt (Zeilen) bzw. Schreibpuffer.
    pub lines: Vec<String>,
    pub pos: usize,
    pub buffer: String,
}

/// Fehler einer API-Methode.
#[derive(Debug, Clone)]
pub enum FwErr {
    /// HRESULT (wird als Zahl geworfen), mit Erklärung für die Diagnose.
    Hr(u32, String),
    /// Member nicht umgesetzt (wird als `Error` mit klarer Meldung geworfen).
    NotImpl(String),
    /// Weiterreichen einer JS-Ausnahme.
    Js(String),
}

impl FwErr {
    /// Ungültiger Parameter.
    pub fn bad(msg: impl Into<String>) -> FwErr {
        FwErr::Hr(hr::E_BAD_PARAM, msg.into())
    }
    /// Allgemeiner Fehler.
    pub fn generic(msg: impl Into<String>) -> FwErr {
        FwErr::Hr(hr::E_GENERIC, msg.into())
    }
}

thread_local! {
    /// Begründung des letzten „nicht verfügbar“-Fehlers eines Befehls im laufenden Skript (CT-A-04):
    /// damit meldet die Oberfläche einen verständlichen Text statt HRESULT und Befehlskennung.
    static LAST_UNAVAILABLE: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Nimmt die zuletzt gemerkte Begründung (siehe `LAST_UNAVAILABLE`) und leert sie.
pub fn take_last_unavailable() -> Option<String> {
    LAST_UNAVAILABLE.with(|c| c.borrow_mut().take())
}

impl From<EngineError> for FwErr {
    fn from(e: EngineError) -> FwErr {
        let msg = e.to_string();
        let reason = match &e {
            EngineError::Unavailable { reason, .. } => Some(reason.clone()),
            EngineError::Selection(m) => Some(m.clone()),
            _ => None,
        };
        if let Some(r) = reason {
            LAST_UNAVAILABLE.with(|c| *c.borrow_mut() = Some(r));
        }
        match e {
            EngineError::NoDocument => FwErr::Hr(hr::E_NO_ACTIVE_DOCUMENT, msg),
            EngineError::Unavailable { .. } | EngineError::Selection(_) => {
                FwErr::Hr(hr::E_BAD_SELECTION, msg)
            }
            EngineError::InvalidParams { .. }
            | EngineError::ElementNotFound(_)
            | EngineError::NotFound(_)
            | EngineError::UnknownCommand(_) => FwErr::Hr(hr::E_BAD_PARAM, msg),
            EngineError::NotImplemented(n) => FwErr::NotImpl(format!("Befehl {n} (fp-engine)")),
            EngineError::Io(_) => FwErr::Hr(hr::E_FILE_NOT_FOUND, msg),
            EngineError::Format(_) => FwErr::Hr(0x8004_1421, msg),
            _ => FwErr::Hr(hr::E_GENERIC, msg),
        }
    }
}

/// Ergebnis-Typ der API-Umsetzung.
pub type FwResult<T> = Result<T, FwErr>;

/// Vorgaben für neue Objekte, solange nichts ausgewählt ist (setBrush/setFill* ohne Auswahl).
#[derive(Clone, Debug, Default)]
pub struct DefaultAttrs {
    /// `Some(None)` = keine Füllung.
    pub fill: Option<Option<String>>,
    /// `Some(None)` = kein Strich; sonst (Breite, Farbe).
    pub stroke: Option<Option<(f64, Option<String>)>>,
    /// Fülltextur nach Namen (`""` = keine), `None` = unverändert.
    pub fill_texture: Option<String>,
    /// Strichtextur nach Namen (`""` = keine), `None` = unverändert.
    pub stroke_texture: Option<String>,
}

/// Zustand eines Laufs.
pub struct State {
    pub session: Session,
    pub handles: Vec<Handle>,
    pub opts: ScriptOptions,
    pub alerts: Vec<String>,
    pub log: Vec<String>,
    pub diagnostics: Vec<String>,
    pub not_implemented: BTreeSet<String>,
    pub used: BTreeSet<String>,
    pub files: Vec<Option<FileRefState>>,
    pub script_stack: Vec<PathBuf>,
    pub export_options: HashMap<u64, Value>,
    pub defaults: HashMap<u64, DefaultAttrs>,
    pub extra_prefs: HashMap<String, Value>,
    pub text_encoding: String,
    pub last_file_error: String,
    /// Modelle geschlossener Dokumente (Wrapper bleiben lesbar).
    pub closed: HashMap<u64, std::sync::Arc<fp_model::Document>>,
    /// Einstellungen für Suchen und Ersetzen (`fw.setUpFindReplace`).
    pub find: Value,
    /// Suchobjekte (`dom.makeFind`): Einstellungen und Zahl der Ersetzungen.
    pub finds: Vec<(Value, usize)>,
}

impl State {
    /// Neuer Zustand mit der Sitzung.
    pub fn new(session: Session, opts: ScriptOptions) -> State {
        State {
            session,
            handles: Vec::new(),
            opts,
            alerts: Vec::new(),
            log: Vec::new(),
            diagnostics: Vec::new(),
            not_implemented: BTreeSet::new(),
            used: BTreeSet::new(),
            files: Vec::new(),
            script_stack: Vec::new(),
            export_options: HashMap::new(),
            defaults: HashMap::new(),
            extra_prefs: HashMap::new(),
            text_encoding: "utf-8".into(),
            last_file_error: String::new(),
            closed: HashMap::new(),
            find: Value::Null,
            finds: Vec::new(),
        }
    }

    /// Neues Handle; liefert den Index.
    pub fn push_handle(&mut self, h: Handle) -> usize {
        self.handles.push(h);
        self.handles.len() - 1
    }

    /// Führt einen Befehl im Dokument `doc` aus (aktiviert es vorübergehend).
    pub fn exec_on(
        &mut self,
        doc: Option<DocId>,
        name: &str,
        params: Value,
    ) -> FwResult<fp_engine::CommandOutput> {
        let prev = self.session.active_id();
        if let Some(d) = doc
            && prev != Some(d)
        {
            self.session.activate(d).map_err(|_| {
                FwErr::Hr(
                    hr::E_NO_ACTIVE_DOCUMENT,
                    fp_i18n::tr!("err-script-doc-n-closed", id = d.0.to_string()),
                )
            })?;
        }
        let r = self.session.execute(name, params);
        if let (Some(d), Some(p)) = (doc, prev)
            && p != d
            && self.session.document(p).is_some()
            && self.session.document(d).is_some()
        {
            let _ = self.session.activate(p);
        }
        r.map_err(|e| {
            let fe = FwErr::from(e);
            if let FwErr::Hr(_, m) = &fe {
                self.diagnostics.push(format!("{name}: {m}"));
            }
            fe
        })
    }
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

/// Setzt den Zustand (Start eines Laufs) bzw. nimmt ihn zurück (`None`).
pub fn install(st: Option<State>) -> Option<State> {
    STATE.with(|c| match c.try_borrow_mut() {
        Ok(mut g) => std::mem::replace(&mut *g, st),
        Err(_) => st,
    })
}

/// Ist ein Lauf aktiv (verschachtelter Aufruf)?
pub fn is_installed() -> bool {
    STATE.with(|c| c.try_borrow().map(|g| g.is_some()).unwrap_or(true))
}

/// Leiht den Zustand kurz aus.
pub fn with_state<R>(f: impl FnOnce(&mut State) -> R) -> FwResult<R> {
    STATE.with(|c| match c.try_borrow_mut() {
        Ok(mut g) => match g.as_mut() {
            Some(st) => Ok(f(st)),
            None => Err(FwErr::Hr(hr::E_INTERNAL, "kein Skriptlauf aktiv".into())),
        },
        Err(_) => Err(FwErr::Hr(
            hr::E_INTERNAL,
            "Zustand ist belegt (verschachtelter Zugriff)".into(),
        )),
    })
}

/// Wie [`with_state`], für Funktionen, die selbst `FwResult` liefern.
pub fn st<R>(f: impl FnOnce(&mut State) -> FwResult<R>) -> FwResult<R> {
    with_state(f)?
}
