//! Batch-Verarbeitung im Befehlsregister (SPEC-BATCH §5.1): `batch.run`, `batch.plan`,
//! `batch.preview`, `batch.steps`, `batch.validate`. Die Arbeit macht das Crate `fp-batch`, das auf
//! fp-engine aufbaut; es steckt sich über den Haken [`BatchHost`] ein (`fp_batch::install_host`),
//! damit kein Abhängigkeitszyklus entsteht. Ohne Haken melden die Befehle „nicht verfügbar“.

use crate::command::{cmd, CommandOutput, CommandRegistry, ParamKind, Params};
use crate::error::{EngineError, Result};
use crate::session::Session;
use serde_json::{json, Value};
use std::sync::{Arc, RwLock};

/// Vorgang eines Batch-Befehls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BatchOp {
    /// Rezept ausführen (`dryRun` = nur planen) → Bericht.
    Run,
    /// Quelle → Ziel mit Konflikten, ohne Schreiben.
    Plan,
    /// Vorschau vorher/nachher einer Datei.
    Preview,
    /// Katalog aller Schritte mit Schema.
    Steps,
    /// Fehler/Warnungen je Schritt.
    Validate,
}

/// Batch-Laufzeit, die `fp-batch` einsteckt. `params` sind die Befehlsparameter (JSON-Objekt).
pub trait BatchHost: Send + Sync {
    fn handle(&self, session: &mut Session, op: BatchOp, params: &Value) -> std::result::Result<Value, String>;
}

static HOST: RwLock<Option<Arc<dyn BatchHost>>> = RwLock::new(None);

/// Setzt die Batch-Laufzeit (prozessweit; `None` entfernt sie).
pub fn set_batch_host(h: Option<Arc<dyn BatchHost>>) {
    if let Ok(mut g) = HOST.write() {
        *g = h;
    }
}

/// Die eingesteckte Batch-Laufzeit.
pub fn batch_host() -> Option<Arc<dyn BatchHost>> {
    HOST.read().ok().and_then(|g| g.clone())
}

fn call(s: &mut Session, op: BatchOp, p: &Params) -> Result<CommandOutput> {
    let host = batch_host().ok_or(EngineError::BackendMissing("err-backend-batch-runtime"))?;
    let params = if p.value.is_object() { p.value.clone() } else { json!({}) };
    let v = host.handle(s, op, &params).map_err(EngineError::Failed)?;
    Ok(CommandOutput::value(v))
}

pub(crate) fn register(r: &mut CommandRegistry) {
    let files = |b: crate::command::CommandBuilder| {
        b.opt("files", ParamKind::Json, "Liste der Dateien")
            .opt("folder", ParamKind::Path, "Ordner mit den Dateien")
            .opt_default("recursive", ParamKind::Bool, json!(true), "Unterordner einbeziehen")
    };
    r.add(
        files(
            cmd("batch.run")
                .label("Batch ausführen")
                .describe("Führt ein Batch-Rezept über eine Dateiliste aus und liefert den Bericht")
                .param("recipe", ParamKind::Json, "Rezept (Objekt) oder Pfad einer Rezept-Datei"),
        )
        .opt_default("dryRun", ParamKind::Bool, json!(false), "nur planen, nichts schreiben")
        .opt("threads", ParamKind::Integer { min: Some(1), max: Some(256) }, "Zahl der Arbeits-Threads")
        .opt("out", ParamKind::Path, "Ausgabeordner (überschreibt das Ziel des Rezepts)")
        .opt("conflict", ParamKind::Enum { values: vec!["rename".into(), "skip".into(), "overwrite".into()] }, "Konfliktregel (überschreibt das Rezept)")
        .run(|s, p| call(s, BatchOp::Run, p)),
    );
    r.add(
        files(cmd("batch.plan").label("Batch planen").describe("Plant Quelle und Ziel jeder Datei mit Konflikten, ohne zu schreiben").param("recipe", ParamKind::Json, "Rezept (Objekt) oder Pfad einer Rezept-Datei"))
            .opt("out", ParamKind::Path, "Ausgabeordner (überschreibt das Ziel des Rezepts)")
            .run(|s, p| call(s, BatchOp::Plan, p)),
    );
    r.add(
        cmd("batch.preview")
            .label("Batch-Vorschau")
            .describe("Rechnet die Vorschau einer Datei vorher und nachher")
            .param("recipe", ParamKind::Json, "Rezept (Objekt) oder Pfad einer Rezept-Datei")
            .param("file", ParamKind::Path, "Datei")
            .opt_default("maxSize", ParamKind::Integer { min: Some(16), max: Some(8192) }, json!(1024), "längste Seite der Vorschau in Pixeln")
            .run(|s, p| call(s, BatchOp::Preview, p)),
    );
    r.add(
        cmd("batch.steps")
            .label("Batch-Schritte")
            .describe("Katalog aller Batch-Schritte mit Parametern")
            .run(|s, p| call(s, BatchOp::Steps, p)),
    );
    r.add(
        cmd("batch.validate")
            .label("Batch-Rezept prüfen")
            .describe("Prüft ein Batch-Rezept und meldet Fehler und Warnungen je Schritt")
            .param("recipe", ParamKind::Json, "Rezept (Objekt) oder Pfad einer Rezept-Datei")
            .run(|s, p| call(s, BatchOp::Validate, p)),
    );
}
