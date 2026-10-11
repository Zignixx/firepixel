//! Fehler der KI-Auswahl (deutsch, für die Statuszeile bzw. die Fehlermeldung des Befehls).

use crate::models::ModelId;

/// Fehler der KI-Auswahl.
#[derive(Debug, thiserror::Error)]
pub enum AiError {
    /// Modelldatei fehlt im Modellordner.
    #[error("KI-Modell „{name}“ fehlt ({file}). Bitte über „Modelle verwalten“ laden oder die Datei in den Modellordner legen: {folder}")]
    ModelMissing { model: ModelId, name: &'static str, file: &'static str, folder: String },
    /// Modelldatei beschädigt oder unpassend.
    #[error("KI-Modell ungültig: {0}")]
    Weights(String),
    /// Download fehlgeschlagen.
    #[error("Download fehlgeschlagen: {0}")]
    Download(String),
    /// Download ist in diesem Prozess gesperrt (Tests, Stapelbetrieb).
    #[error("Download ist gesperrt (Test- oder Stapelbetrieb)")]
    DownloadDisabled,
    /// Prüfsumme stimmt nicht.
    #[error("Prüfsumme der Datei {file} stimmt nicht (erwartet {expected}, erhalten {got})")]
    Checksum { file: String, expected: String, got: String },
    /// Vom Nutzer abgebrochen.
    #[error("abgebrochen")]
    Cancelled,
    /// Ungeeignete Eingabe (leeres Bild, Punkt außerhalb …).
    #[error("{0}")]
    Input(String),
    /// Dateisystem.
    #[error("Datei: {0}")]
    Io(#[from] std::io::Error),
}

/// Ergebnis-Typ.
pub type Result<T> = std::result::Result<T, AiError>;
