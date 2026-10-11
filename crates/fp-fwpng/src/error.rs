//! Fehlertypen.

use thiserror::Error;

/// Fehler beim Lesen oder Schreiben einer FW-PNG. Beschädigte Dateien liefern einen Fehler, nie
/// eine Panik.
#[derive(Debug, Error)]
pub enum FwError {
    #[error("keine PNG-Signatur")]
    NotPng,
    #[error("PNG-Chunk bei Offset {offset} abgeschnitten")]
    TruncatedChunk { offset: usize },
    #[error("PNG ohne IHDR")]
    MissingIhdr,
    #[error("ungültiger Chunk {kind}: {msg}")]
    BadChunk { kind: String, msg: String },
    #[error("Tag-Baum: {msg} (Offset {offset})")]
    Tree { offset: usize, msg: String },
    #[error("aTXT: {msg} (Offset {offset})")]
    Ate { offset: usize, msg: String },
    #[error("zlib: {0}")]
    Zlib(String),
    #[error("Dokument: {0}")]
    Document(String),
    #[error("PNG-Dekodierung: {0}")]
    PngDecode(String),
    #[error("PNG-Kodierung: {0}")]
    PngEncode(String),
    #[error("E/A: {0}")]
    Io(#[from] std::io::Error),
}

/// Ergebnistyp des Crates.
pub type Result<T> = std::result::Result<T, FwError>;
