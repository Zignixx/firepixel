//! Fehlertyp von fp-mcp.

/// Fehler des MCP-Servers (Start, Transport, Sitzungszugang).
#[derive(Debug, thiserror::Error)]
pub enum McpError {
    /// Der Sitzungs-Host ist beendet (Oberfläche geschlossen, Arbeits-Thread gestoppt).
    #[error("die Sitzung ist nicht mehr erreichbar")]
    HostGone,
    /// Ein-/Ausgabe (Socket, Token-Datei).
    #[error("E/A-Fehler: {0}")]
    Io(#[from] std::io::Error),
    /// Fehler der MCP-Bibliothek bzw. des Transports.
    #[error("MCP: {0}")]
    Protocol(String),
    /// Ungültige Einstellung (z. B. Adresse außerhalb von 127.0.0.1).
    #[error("ungültige Einstellung: {0}")]
    Config(String),
    /// Oberflächen-Funktion (Bildschirmfoto, Fensterzustand) ohne laufende Oberfläche.
    #[error("nur im Brückenmodus mit laufender Oberfläche verfügbar (Firepixel mit --mcp bzw. FIREPIXEL_MCP=1 starten); headless stattdessen render verwenden")]
    NoUi,
    /// Die Oberfläche hat nicht rechtzeitig geantwortet (z. B. Fenster minimiert).
    #[error("die Oberfläche hat nicht rechtzeitig geantwortet (Fenster minimiert oder blockiert?)")]
    Timeout,
}
