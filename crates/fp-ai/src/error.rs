//! Fehler der KI-Auswahl (für die Statuszeile bzw. die Fehlermeldung des Befehls; Texte aus
//! fp-i18n, `meldungen.ftl`, Schlüssel `ai-err-…`).

use crate::models::ModelId;

/// Fehler der KI-Auswahl.
#[derive(Debug, thiserror::Error)]
pub enum AiError {
    /// Modelldatei fehlt im Modellordner.
    #[error("{}", m3("ai-err-model-missing", "name", &.model.display_name(), "file", .file, "folder", .folder))]
    ModelMissing { model: ModelId, name: &'static str, file: &'static str, folder: String },
    /// Modelldatei beschädigt oder unpassend.
    #[error("{}", m1("ai-err-weights", "detail", .0))]
    Weights(String),
    /// Download fehlgeschlagen.
    #[error("{}", m1("ai-err-download", "detail", .0))]
    Download(String),
    /// Download ist in diesem Prozess gesperrt (Tests, Stapelbetrieb).
    #[error("{}", m0("ai-err-download-disabled"))]
    DownloadDisabled,
    /// Prüfsumme stimmt nicht.
    #[error("{}", m3("ai-err-checksum", "file", .file, "expected", .expected, "got", .got))]
    Checksum { file: String, expected: String, got: String },
    /// Vom Nutzer abgebrochen.
    #[error("{}", m0("ai-err-cancelled"))]
    Cancelled,
    /// Ungeeignete Eingabe (leeres Bild, Punkt außerhalb …).
    #[error("{}", m1("ai-err-input", "detail", .0))]
    Input(String),
    /// Dateisystem.
    #[error("{}", m1("ai-err-io", "detail", .0))]
    Io(#[from] std::io::Error),
}

// ---- Mehrsprachige Meldungen (fp-i18n, `meldungen.ftl`) ----
fn m0(key: &str) -> String {
    fp_i18n::t(key)
}
fn m1(key: &str, a: &str, va: &dyn std::fmt::Display) -> String {
    fp_i18n::t_args(key, &[(a, fp_i18n::Value::from(va.to_string()))])
}
fn m3(key: &str, a: &str, va: &dyn std::fmt::Display, b: &str, vb: &dyn std::fmt::Display, c: &str, vc: &dyn std::fmt::Display) -> String {
    fp_i18n::t_args(key, &[(a, fp_i18n::Value::from(va.to_string())), (b, fp_i18n::Value::from(vb.to_string())), (c, fp_i18n::Value::from(vc.to_string()))])
}

/// Ergebnis-Typ.
pub type Result<T> = std::result::Result<T, AiError>;
