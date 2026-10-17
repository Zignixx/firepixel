//! Fehlertypen.

use thiserror::Error;

/// Fehler beim Lesen oder Schreiben einer FW-PNG. Beschädigte Dateien liefern einen Fehler, nie
/// eine Panik.
#[derive(Debug, Error)]
pub enum FwError {
    #[error("{}", m0("err-fwpng-not-png"))]
    NotPng,
    #[error("{}", m1("err-fwpng-truncated-chunk", "offset", .offset))]
    TruncatedChunk { offset: usize },
    #[error("{}", m0("err-fwpng-missing-ihdr"))]
    MissingIhdr,
    #[error("{}", m2("err-fwpng-bad-chunk", "kind", .kind, "detail", .msg))]
    BadChunk { kind: String, msg: String },
    #[error("{}", m2("err-fwpng-tree", "detail", .msg, "offset", .offset))]
    Tree { offset: usize, msg: String },
    #[error("{}", m2("err-fwpng-atxt", "detail", .msg, "offset", .offset))]
    Ate { offset: usize, msg: String },
    #[error("zlib: {0}")]
    Zlib(String),
    #[error("{}", m1("err-fwpng-document", "detail", .0))]
    Document(String),
    #[error("{}", m1("err-fwpng-png-decode", "detail", .0))]
    PngDecode(String),
    #[error("{}", m1("err-fwpng-png-encode", "detail", .0))]
    PngEncode(String),
    #[error("{}", m1("err-fwpng-io", "detail", .0))]
    Io(#[from] std::io::Error),
}

// ---- Mehrsprachige Meldungen (fp-i18n, `messages.ftl`) ----
fn m0(key: &str) -> String {
    fp_i18n::t(key)
}
fn m1(key: &str, a: &str, va: &dyn std::fmt::Display) -> String {
    fp_i18n::t_args(key, &[(a, fp_i18n::Value::from(va.to_string()))])
}
fn m2(
    key: &str,
    a: &str,
    va: &dyn std::fmt::Display,
    b: &str,
    vb: &dyn std::fmt::Display,
) -> String {
    fp_i18n::t_args(
        key,
        &[
            (a, fp_i18n::Value::from(va.to_string())),
            (b, fp_i18n::Value::from(vb.to_string())),
        ],
    )
}

/// Ergebnistyp des Crates.
pub type Result<T> = std::result::Result<T, FwError>;
