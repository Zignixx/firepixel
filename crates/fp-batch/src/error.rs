//! Fehler des Batch-Kerns. Die Meldungen sind beim Anlegen schon übersetzt (`fp_i18n`).

/// Fehler eines Auftrags, eines Elements oder eines Schritts.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum BatchError {
    /// Rezept nicht lesbar/ungültig.
    #[error("{0}")]
    Recipe(String),
    /// Datei lesen/schreiben.
    #[error("{0}")]
    Io(String),
    /// Datei nicht dekodierbar.
    #[error("{0}")]
    Decode(String),
    /// Kodieren fehlgeschlagen.
    #[error("{0}")]
    Encode(String),
    /// Ein Schritt ist fehlgeschlagen.
    #[error("{step}: {message}")]
    Step { step: String, message: String },
    /// Ungültige Parameter.
    #[error("{0}")]
    Invalid(String),
    /// Nicht unterstützt (Format, Element-Art).
    #[error("{0}")]
    Unsupported(String),
    /// Abgebrochen (Abbrechen-Knopf, neue Vorschau-Generation).
    #[error("{0}")]
    Cancelled(String),
    /// The item does not pass a filter step (`file.filterSkip`, `file.dedupe`): it is reported
    /// as skipped with `reason`; `copy` = copy the source file unchanged into the output folder.
    #[error("{reason}")]
    Filtered { reason: String, copy: bool },
}

impl BatchError {
    /// Abbruch mit übersetzter Meldung.
    pub fn cancelled() -> BatchError {
        BatchError::Cancelled(fp_i18n::t("batch-msg-cancelled"))
    }

    /// Fehler eines Schritts.
    pub fn step(step: &str, message: impl Into<String>) -> BatchError {
        BatchError::Step {
            step: step.to_string(),
            message: message.into(),
        }
    }

    /// Skipped by a filter step (see [`BatchError::Filtered`]).
    pub fn filtered(reason: impl Into<String>, copy: bool) -> BatchError {
        BatchError::Filtered {
            reason: reason.into(),
            copy,
        }
    }

    /// Ist es ein Abbruch?
    pub fn is_cancelled(&self) -> bool {
        matches!(self, BatchError::Cancelled(_))
    }
}

/// Ergebnis mit [`BatchError`].
pub type Result<T> = std::result::Result<T, BatchError>;
