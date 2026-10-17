//! MCP-Werkzeug `batch` (Batch-Studio für Agenten, SPEC-BATCH §5.8, Paket BATCH-9) und die Ressource
//! `firepixel://docs/batch.md` (Schritt- und Vorlagenkatalog).
//!
//! Aktionen (`action`, Vorgabe `run`):
//!
//! | Aktion | Wirkung |
//! |---|---|
//! | `templates` | eingebaute Rezept-Vorlagen (ID, Titel, Beschreibung, Rezept) |
//! | `steps` | Katalog aller Schritte mit Parameter-Schema |
//! | `validate` | Rezept prüfen (Fehler/Warnungen je Schritt) |
//! | `plan` | Trockenlauf ohne Schreiben: Quelle → Ziel, Konflikte |
//! | `preview` | eine Datei vorher/nachher als Bilder + Größen |
//! | `run` | ausführen; mit `async: true` sofort `{jobId}`, sonst blockierend bis zum Bericht |
//! | `status` | Fortschritt eines Auftrags (`jobId`, optional `wait_ms`), am Ende der Bericht |
//! | `cancel` | Auftrag abbrechen (fertige Dateien bleiben, keine Temporärdateien) |
//!
//! `recipe` ist ein Rezept-Objekt, JSON-Text, ein Pfad zu `*.fpbatch.json` oder der Name einer
//! eingebauten Vorlage (`web-webp`, `logos-to-svg` …). Die Aufträge laufen in eigenen Threads mit
//! eigenen headless Sitzungen – nie in der Sitzung der Oberfläche (Brücke bleibt bedienbar).

use crate::tools::{Block, ToolDef, ToolOutput};
use fp_batch::executor::{self, JobHandle, Progress, RunOptions};
use fp_batch::host::{apply_overrides, items_from_params, recipe_from_param};
use fp_batch::pipeline::{self, PreviewOptions};
use fp_batch::{Severity, StepRegistry};
use fp_engine::Session;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// Name des Werkzeugs.
pub const TOOL: &str = "batch";
/// URI der Ressource.
pub const RESOURCE_URI: &str = "firepixel://docs/batch.md";

/// Aktionen des Werkzeugs.
pub const ACTIONS: [&str; 8] = [
    "run",
    "templates",
    "steps",
    "validate",
    "plan",
    "preview",
    "status",
    "cancel",
];

/// Beschreibung (Titel/Texte setzt `tools::tool_defs` aus `mcp-tool-batch*`).
pub fn tool_def() -> ToolDef {
    ToolDef {
        name: TOOL,
        title: String::new(),
        description: String::new(),
        schema: json!({
            "type": "object",
            "properties": {
                "action": {"type": "string", "enum": ACTIONS, "default": "run"},
                "recipe": {"type": ["object", "string"]},
                "files": {"type": "array", "items": {"type": "string"}},
                "folder": {"type": "string"},
                "recursive": {"type": "boolean", "default": true},
                "out": {"type": "string"},
                "conflict": {"type": "string", "enum": ["rename", "skip", "overwrite"]},
                "dryRun": {"type": "boolean", "default": false},
                "threads": {"type": "integer", "minimum": 1, "maximum": 256},
                "async": {"type": "boolean", "default": false},
                "jobId": {"type": "integer", "minimum": 1},
                "wait_ms": {"type": "integer", "minimum": 0, "maximum": 60000, "default": 0},
                "file": {"type": "string"},
                "maxSize": {"type": "integer", "minimum": 16, "maximum": 4096, "default": 768},
                "language": {"type": "string"}
            },
            "required": [],
            "additionalProperties": false
        }),
        read_only: false,
        destructive: true,
    }
}

fn out_json(v: Value) -> ToolOutput {
    let text = serde_json::to_string_pretty(&v).unwrap_or_else(|_| v.to_string());
    ToolOutput {
        blocks: vec![Block::Text(text)],
        structured: Some(v),
        is_error: false,
    }
}

fn out_err(msg: impl Into<String>) -> ToolOutput {
    let m = msg.into();
    ToolOutput {
        blocks: vec![Block::Text(m.clone())],
        structured: Some(json!({"error": m})),
        is_error: true,
    }
}

/// Führt das Werkzeug aus (Sprache setzt der Aufrufer, `tools::call`).
pub fn call(s: &mut Session, args: &Value) -> ToolOutput {
    fp_batch::install();
    let action = args.get("action").and_then(Value::as_str).unwrap_or("run");
    match action {
        "templates" => out_json(fp_batch::templates::catalog_json()),
        "steps" => out_json(StepRegistry::global().catalog_json()),
        "validate" | "plan" => match s.execute(&format!("batch.{action}"), args.clone()) {
            Ok(o) => out_json(o.value),
            Err(e) => out_err(e.to_string()),
        },
        "preview" => preview(args),
        "run" if args.get("async").and_then(Value::as_bool).unwrap_or(false) => start(args),
        "run" => match s.execute("batch.run", args.clone()) {
            Ok(o) => {
                let ok = o.value.get("ok").and_then(Value::as_bool).unwrap_or(true);
                let mut r = out_json(o.value);
                r.is_error = !ok;
                r
            }
            Err(e) => out_err(e.to_string()),
        },
        "status" | "cancel" => {
            let Some(id) = args.get("jobId").and_then(Value::as_u64) else {
                return out_err(fp_i18n::t("mcp-batch-err-job-id"));
            };
            if action == "cancel" {
                with_job(id, |j| j.handle.as_ref().map(JobHandle::cancel));
            }
            let wait = args
                .get("wait_ms")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                .min(60_000);
            match status(
                id,
                Duration::from_millis(if action == "cancel" {
                    wait.max(2_000)
                } else {
                    wait
                }),
            ) {
                Some(v) => out_json(v),
                None => out_err(fp_i18n::tr!("mcp-batch-err-unknown-job", id = id)),
            }
        }
        other => out_err(fp_i18n::tr!(
            "mcp-batch-err-action",
            action = other.to_string(),
            actions = ACTIONS.join(", ")
        )),
    }
}

// ------------------------------------------------------------------ Vorschau

fn png(img: &fp_raster::RgbaImage) -> Option<Vec<u8>> {
    let item = fp_batch::Item::Still(fp_batch::item::Still {
        image: img.clone(),
        meta: fp_batch::item::Meta {
            has_transparency: true,
            ..Default::default()
        },
    });
    fp_batch::output::encode(&item, fp_batch::OutputFormat::Png, &serde_json::Map::new())
        .ok()
        .map(|(b, _)| b)
}

fn preview(args: &Value) -> ToolOutput {
    let r = match recipe_from_param(args.get("recipe")) {
        Ok(r) => r,
        Err(e) => return out_err(e),
    };
    let Some(file) = args.get("file").and_then(Value::as_str) else {
        return out_err(fp_i18n::t("mcp-batch-err-file"));
    };
    let info = match fp_batch::probe(std::path::Path::new(file)) {
        Ok(i) => i,
        Err(e) => return out_err(e.to_string()),
    };
    let max = args
        .get("maxSize")
        .and_then(Value::as_u64)
        .unwrap_or(768)
        .clamp(16, 4096) as u32;
    let opts = PreviewOptions {
        max_size: max,
        max_size_animation: max.min(480),
        encode_full: true,
        ..PreviewOptions::default()
    };
    let reg = StepRegistry::global();
    let res = match pipeline::preview(&r, &info, &reg, &opts) {
        Ok(r) => r,
        Err(e) => return out_err(e.to_string()),
    };
    let v = json!({
        "file": file,
        "format": res.format.name(),
        "before": {"width": res.before_size.0, "height": res.before_size.1, "bytes": res.bytes_before},
        "after": {"width": res.after_size.0, "height": res.after_size.1, "bytes": res.bytes_after},
        "warnings": res.warnings,
        "ms": res.ms,
        "images": ["before", "after"],
    });
    let mut out = out_json(v);
    out.blocks.extend(png(&res.before).map(Block::Image));
    out.blocks
        .extend(res.after.as_ref().and_then(png).map(Block::Image));
    out
}

// ------------------------------------------------------------------ Aufträge im Hintergrund

struct Job {
    handle: Option<JobHandle>,
    recipe: String,
    total: usize,
    done: usize,
    failed: usize,
    skipped: usize,
    cancelled: usize,
    running: Vec<usize>,
    errors: Vec<Value>,
    started: Instant,
    report: Option<Value>,
}

fn jobs() -> &'static Mutex<BTreeMap<u64, Job>> {
    static J: OnceLock<Mutex<BTreeMap<u64, Job>>> = OnceLock::new();
    J.get_or_init(|| Mutex::new(BTreeMap::new()))
}

fn with_job<R>(id: u64, f: impl FnOnce(&mut Job) -> R) -> Option<R> {
    let mut m = jobs().lock().unwrap_or_else(|e| e.into_inner());
    m.get_mut(&id).map(f)
}

fn start(args: &Value) -> ToolOutput {
    let reg = StepRegistry::global();
    let mut recipe = match recipe_from_param(args.get("recipe")) {
        Ok(r) => r,
        Err(e) => return out_err(e),
    };
    if let Err(e) = apply_overrides(&mut recipe, args) {
        return out_err(e);
    }
    let errors: Vec<String> = recipe
        .validate(&reg)
        .into_iter()
        .filter(|i| i.severity == Severity::Error)
        .map(|i| i.message)
        .collect();
    if !errors.is_empty() {
        return out_err(errors.join("\n"));
    }
    let items = match items_from_params(args, pipeline::is_file_only(&recipe, &reg)) {
        Ok(i) if !i.is_empty() => i,
        Ok(_) => return out_err(fp_i18n::t("batch-err-no-files")),
        Err(e) => return out_err(e),
    };
    let opts = RunOptions {
        dry_run: args.get("dryRun").and_then(Value::as_bool).unwrap_or(false),
        threads: args
            .get("threads")
            .and_then(Value::as_u64)
            .map(|n| n as usize),
        log: true,
        ..RunOptions::default()
    };
    let total = items.len();
    let name = recipe.name.clone();
    let handle = executor::spawn(recipe, items, opts);
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let id = NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let job = Job {
        handle: Some(handle),
        recipe: name,
        total,
        done: 0,
        failed: 0,
        skipped: 0,
        cancelled: 0,
        running: Vec::new(),
        errors: Vec::new(),
        started: Instant::now(),
        report: None,
    };
    jobs()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(id, job);
    out_json(
        json!({"jobId": id, "state": "running", "total": total, "hint": fp_i18n::tr!("mcp-batch-hint-status", id = id)}),
    )
}

/// Ereignisse abholen; liefert `true`, wenn der Auftrag fertig ist (Bericht liegt vor).
fn pump(j: &mut Job) -> bool {
    if let Some(h) = &j.handle {
        while let Ok(ev) = h.events.try_recv() {
            match ev {
                Progress::Started { total } => j.total = total,
                Progress::ItemStarted { index } => j.running.push(index),
                Progress::ItemProgress { .. } => {}
                Progress::ItemDone { index, .. } => {
                    j.done += 1;
                    j.running.retain(|i| *i != index);
                }
                Progress::ItemSkipped { index, .. } => {
                    j.skipped += 1;
                    j.running.retain(|i| *i != index);
                }
                Progress::ItemFailed {
                    index,
                    step,
                    message,
                } => {
                    j.failed += 1;
                    j.running.retain(|i| *i != index);
                    if j.errors.len() < 50 {
                        j.errors
                            .push(json!({"index": index, "step": step, "message": message}));
                    }
                }
                Progress::ItemCancelled { index } => {
                    j.cancelled += 1;
                    j.running.retain(|i| *i != index);
                }
                Progress::Finished(rep) => {
                    let mut v = serde_json::to_value(&*rep).unwrap_or(Value::Null);
                    v["summary"] = json!(rep.summary_text());
                    v["ok"] = json!(rep.totals.failed == 0 && !rep.cancelled);
                    j.report = Some(v);
                }
            }
        }
    }
    if j.report.is_none() && j.handle.as_ref().is_some_and(JobHandle::is_finished) {
        // Faden beendet ohne `Finished` (z. B. Fehler vor dem Start): Ergebnis abholen.
        if let Some(h) = j.handle.take() {
            j.report = Some(match h.join() {
                Ok(rep) => serde_json::to_value(&rep).unwrap_or(Value::Null),
                Err(e) => json!({"ok": false, "error": e.to_string()}),
            });
        }
    }
    j.report.is_some()
}

fn status(id: u64, wait: Duration) -> Option<Value> {
    let until = Instant::now() + wait;
    loop {
        let (finished, v) = with_job(id, |j| {
            let finished = pump(j);
            let processed = j.done + j.failed + j.skipped + j.cancelled;
            let elapsed = j.started.elapsed().as_millis() as u64;
            let eta = (processed > 0 && !finished)
                .then(|| elapsed * (j.total.saturating_sub(processed)) as u64 / processed as u64);
            let cancelled = j
                .report
                .as_ref()
                .and_then(|r| r.get("cancelled"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let state = if !finished {
                "running"
            } else if cancelled {
                "cancelled"
            } else {
                "finished"
            };
            let mut v = json!({
                "jobId": id,
                "recipe": j.recipe,
                "state": state,
                "total": j.total,
                "processed": processed,
                "done": j.done,
                "failed": j.failed,
                "skipped": j.skipped,
                "cancelled": j.cancelled,
                "percent": if j.total == 0 { 100.0 } else { (processed as f64 * 1000.0 / j.total as f64).round() / 10.0 },
                "running": j.running,
                "elapsedMs": elapsed,
                "etaMs": eta,
                "errors": j.errors,
            });
            if let Some(r) = &j.report {
                v["report"] = r.clone();
            }
            (finished, v)
        })?;
        if finished || Instant::now() >= until {
            return Some(v);
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

// ------------------------------------------------------------------ Ressource

/// Ressource `firepixel://docs/batch.md`: Werkzeug, Vorlagen und Schritte in der aktuellen Sprache.
pub fn batch_markdown() -> String {
    use std::fmt::Write as _;
    let mut s = String::new();
    let _ = writeln!(s, "# {}\n", fp_i18n::t("mcp-res-batch-title"));
    let _ = writeln!(s, "{}\n", fp_i18n::t("mcp-res-batch-intro"));
    let _ = writeln!(s, "## {}\n", fp_i18n::t("mcp-res-batch-templates"));
    for t in fp_batch::templates::all() {
        let steps: Vec<String> = t
            .recipe
            .steps
            .iter()
            .map(|x| format!("`{}`", x.id))
            .collect();
        let _ = writeln!(
            s,
            "- `{}` – **{}**: {} ({} → {})",
            t.id,
            t.title,
            t.description,
            if steps.is_empty() {
                "–".to_string()
            } else {
                steps.join(", ")
            },
            t.recipe.output.format
        );
    }
    let _ = writeln!(s, "\n## {}\n", fp_i18n::t("mcp-res-batch-steps"));
    let cat = StepRegistry::global().catalog_json();
    for st in cat["steps"].as_array().cloned().unwrap_or_default() {
        let params: Vec<String> = st["params"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|p| {
                        let name = p["name"].as_str().unwrap_or_default();
                        match p.get("values").and_then(Value::as_array) {
                            Some(v) => format!(
                                "`{name}` ({})",
                                v.iter()
                                    .filter_map(Value::as_str)
                                    .collect::<Vec<_>>()
                                    .join("/")
                            ),
                            None => format!("`{name}`"),
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();
        let _ = writeln!(
            s,
            "- `{}` ({}) – **{}**{}{}",
            st["id"].as_str().unwrap_or_default(),
            st["group"].as_str().unwrap_or_default(),
            st["name"].as_str().unwrap_or_default(),
            st["description"]
                .as_str()
                .map(|d| format!(": {d}"))
                .unwrap_or_default(),
            if params.is_empty() {
                String::new()
            } else {
                format!(" – {}", params.join(", "))
            }
        );
    }
    s
}
