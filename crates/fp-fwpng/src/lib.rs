//! Leser und Schreiber für FW-PNG (`.fw.png`, Spezifikation 02; Referenz `re/format/fwpng.py`).
//!
//! # Schichten
//!
//! | Modul | Inhalt |
//! |---|---|
//! | [`png`] | PNG-Container auf Chunk-Ebene (eigene Implementierung), `IDAT` über das `png`-Crate, `tEXt`/`iTXt`/`pHYs` |
//! | [`chunks`] | private Chunks `mkBF`, `mkBT` (Kacheln), `prVW`/`pRVW` (Vorschau), `mkPG` |
//! | [`tree`] | getaggter Textbaum von `mkTS`/`mkBS`: Parser, Serialisierer (byte-identisch), Zahlformat |
//! | [`ate`] | `aTXT` (Text-Engine (ATE)): COS-Parser/-Serialisierer, Story-Dekodierung; Schlüssel in [`ate_keys`] |
//! | [`mod@read`] / [`mod@write`] | Abbildung Tag-Baum ↔ [`fp_model::Document`] (verlustfrei über [`fp_model::Extra`]) |
//! | [`preview`] | Erzeugung von `prVW` exakt nach 02 §12.8 |
//! | [`mod@file`] | ganze Dateien lesen/schreiben ([`fn@read`], [`fn@write`]) |
//! | [`flatten`] | Schnittstelle zum Renderer ([`Flattener`]) für `IDAT` und Vorschau |
//! | [`vocab`] | Tag-Vokabular des Korpus |
//!
//! # Beispiel
//!
//! ```no_run
//! let bytes = std::fs::read("bild.fw.png").unwrap();
//! let doc = fp_fwpng::read(&bytes).unwrap();
//! println!("{} Seite(n), {} Symbol(e)", doc.pages.len(), doc.symbols.len());
//! let out = fp_fwpng::write(&doc, &fp_fwpng::WriteOptions::default(), &fp_fwpng::NoFlattener).unwrap();
//! std::fs::write("kopie.fw.png", out).unwrap();
//! ```

pub mod ate;
#[allow(clippy::all)]
pub mod ate_keys;
pub mod chunks;
pub mod error;
pub mod file;
pub mod flatten;
pub mod png;
pub mod preview;
pub mod read;
pub mod schema;
pub mod tree;
pub mod vocab;
pub mod write;
pub mod zlib;

pub use error::{FwError, Result};

/// `tEXt Software` neuer Firepixel-Dateien. „r2“: Symbolursprung in der Mitte des Inhalts wie im
/// Original (L-3). Steht der Text schon in der Datei (`FileMeta::text_chunks`), wird er unverändert
/// geschrieben.
pub const SOFTWARE: &str = "Firepixel r2";
/// `tEXt Software` älterer Firepixel-Dateien: Symbolursprung in Dokumentkoordinaten (Inhalt beim
/// Umwandeln nicht verschoben, Instanz mit Einheitsmatrix). Beim Laden über
/// `fp_engine::symbol_origin::read_fwpng` umgerechnet.
pub const SOFTWARE_LEGACY: &str = "Firepixel";
pub use file::{
    PreviewMode, RawFile, ReadOptions, WriteOptions, document_from_raw, is_fwpng, model_roundtrip,
    read, read_raw, read_with, write,
};
pub use flatten::{Flattener, NoFlattener};
pub use write::CachePolicy;
