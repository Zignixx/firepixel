//! Fehlertyp von fp-engine.

/// Fehler bei der Ausführung von Befehlen, beim Laden/Speichern und in der Sitzung.
///
/// Befehle liefern Fehler statt einer Panik; nach einem Fehler ist der Dokumentzustand unverändert
/// (die Sitzung stellt den Zustand vor dem Befehl wieder her).
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum EngineError {
    /// Kein Befehl dieses Namens im Register.
    #[error("unbekannter Befehl „{0}“")]
    UnknownCommand(String),
    /// Der Befehl ist registriert (Menü vollständig), aber noch ohne Wirkung.
    #[error("Befehl „{0}“ ist noch nicht umgesetzt")]
    NotImplemented(String),
    /// Das Verfügbarkeits-Prädikat ist nicht erfüllt (z. B. keine Auswahl).
    #[error("Befehl „{name}“ ist nicht verfügbar: {reason}")]
    Unavailable { name: String, reason: String },
    /// Parameter fehlen oder haben den falschen Typ.
    #[error("ungültige Parameter für „{name}“: {message}")]
    InvalidParams { name: String, message: String },
    /// Es ist kein Dokument geöffnet.
    #[error("kein Dokument geöffnet")]
    NoDocument,
    /// Die Auswahl ist leer bzw. passt nicht zum Befehl.
    #[error("keine passende Auswahl: {0}")]
    Selection(String),
    /// Ein Element mit dieser Laufzeit-ID existiert nicht.
    #[error("Element {0} nicht gefunden")]
    ElementNotFound(u64),
    /// Ebene, Seite, Status oder Symbol nicht gefunden.
    #[error("{0} nicht gefunden")]
    NotFound(String),
    /// Dateisystemfehler.
    #[error("Datei: {0}")]
    Io(String),
    /// Fehler beim Lesen/Schreiben einer FW-PNG.
    #[error("FW-PNG: {0}")]
    Format(String),
    /// Für die Operation fehlt ein Baustein (z. B. Renderer oder Pfadoperationen).
    #[error("für diese Operation fehlt {0}")]
    BackendMissing(&'static str),
    /// Nichts rückgängig zu machen bzw. zu wiederholen.
    #[error("{0}")]
    History(&'static str),
    /// Sonstiger fachlicher Fehler.
    #[error("{0}")]
    Failed(String),
}

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
