//! Fehlertyp von fp-mcp. Die Meldungen kommen aus `mcp.ftl` (`err-mcp-*`) in der Sprache des
//! aufrufenden Threads (für KI-Agenten über `fp_i18n::with_language`).

/// Fehler des MCP-Servers (Start, Transport, Sitzungszugang).
#[derive(Debug)]
pub enum McpError {
    /// Der Sitzungs-Host ist beendet (Oberfläche geschlossen, Arbeits-Thread gestoppt).
    HostGone,
    /// Ein-/Ausgabe (Socket, Token-Datei).
    Io(std::io::Error),
    /// Fehler der MCP-Bibliothek bzw. des Transports.
    Protocol(String),
    /// Ungültige Einstellung (z. B. Adresse außerhalb von 127.0.0.1).
    Config(String),
    /// Oberflächen-Funktion (Bildschirmfoto, Fensterzustand) ohne laufende Oberfläche.
    NoUi,
    /// Die Oberfläche hat nicht rechtzeitig geantwortet (z. B. Fenster minimiert).
    Timeout,
}

impl std::fmt::Display for McpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            McpError::HostGone => fp_i18n::t("err-mcp-host-gone"),
            McpError::Io(e) => fp_i18n::tr!("err-mcp-io", error = e.to_string()),
            McpError::Protocol(m) => fp_i18n::tr!("err-mcp-protocol", error = m.clone()),
            McpError::Config(m) => fp_i18n::tr!("err-mcp-config", error = m.clone()),
            McpError::NoUi => fp_i18n::t("err-mcp-no-ui"),
            McpError::Timeout => fp_i18n::t("err-mcp-timeout"),
        };
        f.write_str(&text)
    }
}

impl std::error::Error for McpError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            McpError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for McpError {
    fn from(e: std::io::Error) -> Self {
        McpError::Io(e)
    }
}
