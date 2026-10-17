//! Moderner Batch-Modus „Batch-Studio“ – Kern ohne Oberfläche (SPEC-BATCH, `docs\BATCH_MODERN.md`).
//!
//! - [`recipe`] – Rezept-Modell (JSON, englische Schlüssel, Version, Migration, Validierung)
//! - [`item`] – Elemente (Kopf lesen, Ordner sammeln, dekodieren)
//! - [`step`] – Schritt-Schnittstelle und Register; Schritte in [`steps`]
//! - [`pipeline`] – Anwendung der Schritte, „nur Dateischritte“, Live-Vorschau
//! - [`naming`] – Umbenennen (Baukasten/Regex), Ausgabeordner, Konflikte, Plan/Trockenlauf
//! - [`executor`] – Thread-Pool, Speicherbudget, Abbrechen, Fortschritt, Lauf im Hintergrund
//! - [`output`] – Kodieren, Temporärdatei + atomares Umbenennen, Sicherung
//! - [`report`] – `batch-report.json`, `batch-log.txt`; [`report_html`] – `batch-report.html`
//! - [`host`] – Befehle `batch.*` im Befehlsregister ([`install_host`])
//!
//! Dieselbe Pipeline dient Oberfläche, CLI (`firepixel-cli batch --recipe`), MCP und JSF.

pub mod error;
pub mod executor;
pub mod explain;
pub mod host;
pub mod item;
pub mod meta;
pub mod naming;
pub mod output;
pub mod pipeline;
pub mod recipe;
pub mod report;
pub mod report_html; // BATCH-11b: batch-report.html
pub mod step;
pub mod steps;
pub mod templates;
pub mod video; // BATCH-9: eingebaute Rezept-Vorlagen

pub use error::{BatchError, Result};
pub use executor::{AskChoice, CancelToken, JobHandle, Progress, RunOptions, plan, run, spawn};
pub use host::{install, install_host};
pub use item::{Item, ItemInfo, ItemKind, collect, probe};
pub use naming::{Plan, PlanAction, PlanEntry};
pub use output::OutputFormat;
pub use pipeline::{Pipeline, PreviewOptions, PreviewResult, preview};
pub use recipe::{Recipe, StepConfig};
pub use report::{ItemReport, ItemStatus, Report};
pub use step::{
    Issue, ParamDef, ParamType, Severity, Step, StepContext, StepGroup, StepKind, StepRegistry,
};

/// Ordner der gespeicherten Rezepte (`%APPDATA%\Firepixel\BatchRecipes`; in Tests das Testprofil).
pub fn recipe_dir() -> Option<std::path::PathBuf> {
    fp_i18n::profile::roaming_dir().map(|d| d.join(recipe::RECIPE_FOLDER))
}
