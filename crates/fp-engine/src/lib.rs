//! Programmkern von Firepixel (Phase 3): UI-unabhängig, headless nutzbar von der Oberfläche
//! (fp-ui), der Kommandozeile (firepixel-cli), dem MCP-Server (fp-mcp) und der JSF-Schicht (fp-script).
//!
//! # Überblick
//!
//! | Modul | Inhalt | Spezifikation |
//! |---|---|---|
//! | [`command`] | Befehlsregister: `Command`, Parameter mit JSON-Schema, Verfügbarkeits-Prädikate, Menübaum | ARCHITEKTUR §3, 10 §4, 11 §2.3.2 |
//! | [`commands`] | alle eingebauten Befehle (Menüs, Werkzeug- und Bedienfeldbefehle) | 10 §4.2 |
//! | [`catalog`] | Befehlskatalog aus `menuebefehle.json` (260 Hauptmenübefehle + 20 Kürzel) | 10 §4 |
//! | [`history`] | Undo/Redo-Verlauf mit Copy-on-Write-Schnappschüssen, `MaxUndos`, Befehlsliste | 10 §3 |
//! | [`selection`] | Auswahl (Elemente, Pfadpunkte, Pixelmaske), Zwischenablage | 10 §2.6 |
//! | [`tools`] | Werkzeug-Zustandsautomaten (Zeiger … Zoom), Werkzeugtabelle mit GUIDs | 10 §2 |
//! | [`shortcuts`] | Tastenkürzel und die fünf Tastenkürzel-Sets | 10 §4.1 Nr. 11 |
//! | [`prefs`] | Voreinstellungen (644 Schlüssel mit Vorgaben), Design, Einheiten | 14 §3, §4 |
//! | [`session`] | Sitzung: Dokumente, Ausführung, Verlauf, Ereignisse, Werkzeugereignisse | – |
//! | [`view`] | Zoom (17 Stufen), Bildlauf, Raster/Hilfslinien-Schalter | 10 §4.4 |
//! | [`docops`] | Bausteine für Dokumentänderungen (Suchen, Einfügen, Transformieren, Grenzen …) | 04 |
//! | [`geometry`] | schmale Geometrie-Schnittstelle (fp-vector optional), Grundformen | 16 |
//! | [`render`] | Schnittstelle zum Compositor (fp-render) | – |
//!
//! # Beispiel
//!
//! ```
//! use fp_engine::Session;
//! use serde_json::json;
//!
//! let mut s = Session::default();
//! s.execute("file.new", json!({"width": 200, "height": 100})).unwrap();
//! s.execute("draw.rectangle", json!({"rect": [10, 10, 60, 40]})).unwrap();
//! s.execute("modify.transform.flipHorizontal", json!({})).unwrap();
//! assert_eq!(s.active().unwrap().history.undo_count(), 2);
//! s.execute("edit.undo", json!({})).unwrap();
//! assert_eq!(s.active().unwrap().history.undo_count(), 1);
//! ```

pub mod catalog;
pub mod command;
pub mod commands;
pub mod docops;
pub mod error;
pub mod export_backend;
pub mod geometry;
pub mod history;
pub mod instance_name;
pub mod paint;
pub mod prefs;
pub mod render;
#[cfg(feature = "render")]
pub mod render_backend;
pub mod resources;
pub mod script_prompt;
pub mod selection;
pub mod session;
pub mod shell;
pub mod shortcuts;
pub mod symbol_origin;
pub mod tool_follow;
pub mod tools;
pub mod url_library;
#[cfg(feature = "vector")]
pub mod vector_backend;
pub mod view;
pub mod warnings;

pub use command::{
    Command, CommandInfo, CommandOutput, CommandRegistry, Effect, ObjectKind, ParamKind, ParamSpec,
    Params, Requirement,
};
pub use error::{EngineError, Result};
pub use history::{History, HistoryStep, RecordedCommand, StepKey};
pub use prefs::{PrefValue, Preferences, Theme};
pub use selection::{Clipboard, PointRef, SelectMode, Selection, SelectionHistory};
pub use session::{DocId, DocumentState, ScriptHost, Session, SessionEvent};
pub use shortcuts::{Shortcut, ShortcutSet};
pub use tool_follow::ToolStyle;
pub use tools::{Modifiers, Overlay, ToolEvent, ToolId, ToolKey, ToolResponse};
pub use view::ViewState;
pub use warnings::{TargetWarning, WarningChoice};
