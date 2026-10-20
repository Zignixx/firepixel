//! KI-Auswahl für Firepixel.
//!
//! * [`sam2`]: Objektauswahl mit SAM 2.1 (Hiera-Bildencoder + Prompt-Encoder + Maskendecoder). Das
//!   Bild-Encoding wird je Bild zwischengespeichert ([`cache`]), danach läuft pro Klick nur der Decoder.
//! * [`birefnet`]: Motiv auswählen (BiRefNet bzw. BiRefNet_lite) und Kante verfeinern
//!   (BiRefNet-Matting-Varianten) mit echter Halbtransparenz.
//! * [`nn`]: eigene kleine CPU-Inferenz in reinem Rust (Matrixmultiplikation über `faer`, alles andere
//!   mit `rayon` parallel), ohne DLL und ohne `unsafe`.
//! * [`models`]/[`download`]: Modellkatalog (offizielle Quellen, Größe, Lizenz, SHA-256), Suche im
//!   Modellordner und Download erst nach Zustimmung, mit Fortschritt, Prüfsumme und Abbrechen.
//! * [`pipeline`]: Vor-/Nachverarbeitung, Skalierung großer Bilder, Ergebnis als
//!   [`fp_raster::Mask`] in Dokumentkoordinaten.
//! * [`job`]: Rechnen im Hintergrund-Thread mit Fortschritt und Abbrechen.
//! * [`tuning`]: Feineinstellungen (Toleranz, Maskenvariante, Kante, Ausdehnen, Glätten) als
//!   Nachbearbeitung der zwischengespeicherten Modell-Ausgabe.

pub mod birefnet;
pub mod cache;
pub mod cancel;
pub mod download;
pub mod error;
pub mod face; // BA-109: face detection (Haar cascade) for ai.faceBlur
pub mod job;
pub mod models;
pub mod nn;
pub mod pipeline;
pub mod sam2;
pub mod tuning;
pub mod weights;

pub use cancel::{Cancel, Progress, Stage};
pub use error::{AiError, Result};
pub use models::{ModelId, ModelInfo};
pub use pipeline::{ObjectPrompt, PromptPoint};
pub use tuning::{Edge, Tuning, Variant};
