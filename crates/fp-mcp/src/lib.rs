//! MCP-Server von Firepixel (Model Context Protocol, Bibliothek rmcp).
//!
//! KI-Agenten steuern Firepixel über **dasselbe Befehlsregister** wie Oberfläche, CLI und JSF
//! (fp-engine). Zwei Transporte:
//!
//! * **stdio** ([`serve_stdio`], CLI `firepixel-cli mcp`): headless, eine Sitzung je Prozess.
//! * **Streamable HTTP** ([`start_http`], [`start_http_background`], CLI `firepixel-cli mcp --http`):
//!   nur auf `127.0.0.1`, jede Anfrage mit zufälligem Token (Datei [`default_token_file`]).
//!
//! Die Sitzung liegt hinter einem [`SessionHost`]: headless [`LocalHost`] (eigener Thread) oder
//! [`BridgeHost`] für die laufende Oberfläche (Aufträge werden im UI-Thread abgearbeitet; Anleitung
//! zur Verdrahtung in [`host`]).
//!
//! # Werkzeuge
//!
//! | Werkzeug | Zweck |
//! |---|---|
//! | `list_commands`, `describe_command` | Befehlskatalog, JSON-Schema, Prädikate, Kürzel |
//! | `run_command` | Befehl (oder mehrere als ein Schritt) ausführen; Ergebnis + neue Auswahl |
//! | `get_document` | Dokumentbaum als JSON (Seiten, Status, Ebenen, Elemente mit IDs/Grenzen/Attributen) |
//! | `select` | Auswahl nach IDs, alles/nichts, Punkt, Rechteck |
//! | `render` | Seite/Ausschnitt/Element offscreen als PNG-Bild, Zoom |
//! | `open`, `save`, `export` | Dateien (FW-PNG, PNG) |
//! | `undo`, `redo`, `history` | Verlauf |
//! | `new_document`, `documents` | Dokumente anlegen, wechseln, schließen |
//! | `tool_event` | Zeigerereignisse an die Werkzeug-Automaten (Zeichnen wie ein Mensch) |
//! | `run_script` | JSF-JavaScript (fp-script) |
//!
//! Ressourcen: `firepixel://docs/agents.md`, `commands.md`, `document-model.md`, `examples.md`, `jsf.md`.
//!
//! # Beispiel (headless, ohne Transport)
//!
//! ```
//! use fp_mcp::tools::{call, ToolEnv};
//! use serde_json::json;
//! let mut s = fp_engine::Session::default();
//! let env = ToolEnv::default();
//! call(&mut s, &env, "new_document", &json!({"width": 100, "height": 100}));
//! let r = call(&mut s, &env, "run_command", &json!({"name": "draw.ellipse", "params": {"rect": [10, 10, 90, 90]}}));
//! assert!(!r.is_error);
//! ```

pub mod batch; // BATCH-9: Werkzeug `batch`, Ressource batch.md
pub mod docjson;
pub mod error;
pub mod host;
pub mod image;
pub mod proxy;
pub mod render;
pub mod resources;
pub mod server;
pub mod tools;
pub mod transport;

pub use error::McpError;
pub use host::{
    BridgeHost, BridgeReceiver, LocalHost, PendingUi, SessionHost, SessionJob, UiImage, UiReply,
    UiRequest, UiResponder, ui_request, with_session, with_session_blocking,
};
pub use server::FirepixelMcp;
pub use transport::{
    DEFAULT_PORT, HttpOptions, HttpServer, MCP_PATH, bridge_token_file, default_token_file,
    run_stdio_blocking, serve_io, serve_stdio, start_http, start_http_background,
};
