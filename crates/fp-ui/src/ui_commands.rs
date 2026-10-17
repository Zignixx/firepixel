//! Brücke zu den Bedienfeld-/Inspektorbefehlen. Die Befehle (`pi.*`, `panel.*`) liegen seit
//! Phase 5 in fp-engine ([`fp_engine::commands::panel`]) und stehen damit im Standardregister –
//! auch für CLI, MCP und JSF. Dieses Modul behält die alte Schnittstelle der Oberfläche
//! (Hilfsfunktionen, [`execute`], [`new_session`]) bei.

pub use fp_engine::commands::panel::*;

use fp_engine::command::{CommandOutput, CommandRegistry};
use fp_engine::{Result, Session};
use serde_json::Value;
use std::sync::{Arc, OnceLock};

/// Standardregister (enthält die Bedienfeldbefehle).
pub fn registry() -> Arc<CommandRegistry> {
    static R: OnceLock<Arc<CommandRegistry>> = OnceLock::new();
    Arc::clone(R.get_or_init(|| Arc::new(CommandRegistry::with_defaults())))
}

/// Neue Sitzung mit dem Standardregister.
pub fn new_session(prefs: fp_engine::Preferences) -> Session {
    Session::with_registry(prefs, registry())
}

/// Führt einen Befehl aus (wie [`Session::execute`]).
pub fn execute(s: &mut Session, name: &str, params: Value) -> Result<CommandOutput> {
    s.execute(name, params)
}
