//! Fehlertyp von fp-engine.
//!
//! # Mehrsprachigkeit
//! Jede Variante hat einen stabilen Schlüssel ([`EngineError::message_key`], Bereichsdatei
//! `messages.ftl` von fp-i18n) und benannte Argumente ([`EngineError::args`]). `Display` setzt
//! daraus den Text in der aktuellen Sprache zusammen (Deutsch wie bisher).
//!
//! Die Nutzlast der Varianten ist bereits übersetzt (wird beim Erzeugen über `fp_i18n::t`
//! geholt) – mit zwei Ausnahmen: [`EngineError::BackendMissing`] und [`EngineError::History`]
//! tragen einen Schlüssel (`&'static str`), den erst `Display` übersetzt. Fremde Texte
//! (Fehler aus anderen Crates, Dateipfade) werden unverändert eingesetzt.

use std::fmt;

/// Fehler bei der Ausführung von Befehlen, beim Laden/Speichern und in der Sitzung.
///
/// Befehle liefern Fehler statt einer Panik; nach einem Fehler ist der Dokumentzustand unverändert
/// (die Sitzung stellt den Zustand vor dem Befehl wieder her).
#[derive(Debug, Clone, PartialEq)]
pub enum EngineError {
    /// Kein Befehl dieses Namens im Register.
    UnknownCommand(String),
    /// Der Befehl ist registriert (Menü vollständig), aber noch ohne Wirkung.
    NotImplemented(String),
    /// Das Verfügbarkeits-Prädikat ist nicht erfüllt (z. B. keine Auswahl).
    Unavailable { name: String, reason: String },
    /// Parameter fehlen oder haben den falschen Typ.
    InvalidParams { name: String, message: String },
    /// Es ist kein Dokument geöffnet.
    NoDocument,
    /// Die Auswahl ist leer bzw. passt nicht zum Befehl.
    Selection(String),
    /// Ein Element mit dieser Laufzeit-ID existiert nicht.
    ElementNotFound(u64),
    /// Ebene, Seite, Status oder Symbol nicht gefunden.
    NotFound(String),
    /// Dateisystemfehler.
    Io(String),
    /// Fehler beim Lesen/Schreiben einer FW-PNG.
    Format(String),
    /// Für die Operation fehlt ein Baustein (z. B. Renderer oder Pfadoperationen); Nutzlast =
    /// Schlüssel `err-backend-…`.
    BackendMissing(&'static str),
    /// Nichts rückgängig zu machen bzw. zu wiederholen; Nutzlast = Schlüssel `err-req-…`.
    History(&'static str),
    /// Sonstiger fachlicher Fehler.
    Failed(String),
}

impl EngineError {
    /// Stabiler Schlüssel der Meldung (`messages.ftl`), z. B. `err-not-found`. Für Skripte, MCP
    /// und Tests, die die Art des Fehlers unabhängig von der Sprache prüfen.
    pub fn message_key(&self) -> &'static str {
        match self {
            EngineError::UnknownCommand(_) => "err-unknown-command",
            EngineError::NotImplemented(_) => "err-not-implemented",
            EngineError::Unavailable { .. } => "err-unavailable",
            EngineError::InvalidParams { .. } => "err-invalid-params",
            EngineError::NoDocument => "err-no-document",
            EngineError::Selection(_) => "err-selection",
            EngineError::ElementNotFound(_) => "err-element-not-found",
            EngineError::NotFound(_) => "err-not-found",
            EngineError::Io(_) => "err-io",
            EngineError::Format(_) => "err-format",
            EngineError::BackendMissing(_) => "err-backend-missing",
            EngineError::History(_) => "err-failed",
            EngineError::Failed(_) => "err-failed",
        }
    }

    /// Benannte Argumente der Meldung (Werte in der aktuellen Sprache).
    pub fn args(&self) -> Vec<(&'static str, String)> {
        match self {
            EngineError::UnknownCommand(n) | EngineError::NotImplemented(n) => {
                vec![("name", n.clone())]
            }
            EngineError::Unavailable { name, reason } => {
                vec![("name", name.clone()), ("reason", reason.clone())]
            }
            EngineError::InvalidParams { name, message } => {
                vec![("name", name.clone()), ("message", message.clone())]
            }
            EngineError::NoDocument => Vec::new(),
            EngineError::Selection(d)
            | EngineError::Io(d)
            | EngineError::Format(d)
            | EngineError::Failed(d) => vec![("detail", d.clone())],
            EngineError::ElementNotFound(id) => vec![("id", id.to_string())],
            EngineError::NotFound(w) => vec![("what", w.clone())],
            EngineError::BackendMissing(k) => vec![("what", fp_i18n::t(k))],
            EngineError::History(k) => vec![("detail", fp_i18n::t(k))],
        }
    }
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let args: Vec<(&str, fp_i18n::Value<'_>)> = self
            .args()
            .into_iter()
            .map(|(k, v)| (k, fp_i18n::Value::from(v)))
            .collect();
        f.write_str(&fp_i18n::t_args(self.message_key(), &args))
    }
}

impl std::error::Error for EngineError {}

impl From<std::io::Error> for EngineError {
    fn from(e: std::io::Error) -> Self {
        EngineError::Io(e.to_string())
    }
}

impl From<fp_fwpng::FwError> for EngineError {
    fn from(e: fp_fwpng::FwError) -> Self {
        EngineError::Format(e.to_string())
    }
}

impl From<fp_raster::RasterError> for EngineError {
    fn from(e: fp_raster::RasterError) -> Self {
        EngineError::Failed(e.to_string())
    }
}

/// Ergebnis-Alias.
pub type Result<T> = std::result::Result<T, EngineError>;

/// Alle Varianten mit Beispielwerten (für die Vollständigkeitsprüfung der Schlüssel).
#[doc(hidden)]
pub fn all_variants_for_tests() -> Vec<EngineError> {
    vec![
        EngineError::UnknownCommand("x.y".into()),
        EngineError::NotImplemented("x.y".into()),
        EngineError::Unavailable {
            name: "x.y".into(),
            reason: "r".into(),
        },
        EngineError::InvalidParams {
            name: "x.y".into(),
            message: "m".into(),
        },
        EngineError::NoDocument,
        EngineError::Selection("s".into()),
        EngineError::ElementNotFound(7),
        EngineError::NotFound("n".into()),
        EngineError::Io("i".into()),
        EngineError::Format("f".into()),
        EngineError::BackendMissing("err-backend-renderer"),
        EngineError::History("err-req-can-undo"),
        EngineError::Failed("x".into()),
    ]
}
