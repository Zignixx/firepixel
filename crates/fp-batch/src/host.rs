//! Anbindung an das Befehlsregister (`batch.*` in fp-engine über den Haken
//! [`fp_engine::commands::batch::BatchHost`]) – gemeinsam für Oberfläche, CLI, MCP und JSF.

use crate::executor::{self, RunOptions};
use crate::item::{self, ItemInfo};
use crate::naming::PlanOptions;
use crate::pipeline::{self, PreviewOptions};
use crate::recipe::{Conflict, Recipe, TargetKind};
use crate::step::StepRegistry;
use fp_engine::Session;
use fp_engine::commands::batch::{BatchHost, BatchOp};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::Arc;

/// Steckt die Batch-Laufzeit ein (prozessweit; die Sitzung ist für spätere Erweiterungen da).
pub fn install_host(_session: &mut Session) {
    install();
}

/// Steckt die Batch-Laufzeit ohne Sitzung ein.
pub fn install() {
    fp_engine::commands::batch::set_batch_host(Some(Arc::new(Host)));
}

struct Host;

/// Rezept aus einem Parameter: Objekt, JSON-Text, Pfad oder Name einer eingebauten Vorlage
/// (BATCH-9, [`crate::templates::find`]; eine vorhandene Datei hat Vorrang).
pub fn recipe_from_param(v: Option<&Value>) -> Result<Recipe, String> {
    match v {
        Some(Value::Object(_)) => {
            Recipe::from_value(v.cloned().unwrap_or(Value::Null)).map_err(|e| e.to_string())
        }
        Some(Value::String(s)) if s.trim_start().starts_with('{') => {
            Recipe::from_json(s).map_err(|e| e.to_string())
        }
        Some(Value::String(s)) => {
            match crate::templates::recipe(s).filter(|_| !std::path::Path::new(s).exists()) {
                Some(r) => Ok(r),
                None => Recipe::load(std::path::Path::new(s)).map_err(|e| e.to_string()),
            }
        }
        _ => Err(fp_i18n::t("batch-err-recipe-missing")),
    }
}

/// Dateiliste aus `files` (Liste oder Text) bzw. `folder` + `recursive`.
pub fn items_from_params(p: &Value, allow_other: bool) -> Result<Vec<ItemInfo>, String> {
    let mut paths: Vec<PathBuf> = match p.get("files") {
        Some(Value::Array(a)) => a
            .iter()
            .filter_map(Value::as_str)
            .map(PathBuf::from)
            .collect(),
        Some(Value::String(s)) => vec![PathBuf::from(s)],
        _ => Vec::new(),
    };
    if let Some(f) = p.get("folder").and_then(Value::as_str) {
        paths.push(PathBuf::from(f));
    }
    if paths.is_empty() {
        return Err(fp_i18n::t("batch-err-no-files"));
    }
    let recursive = p.get("recursive").and_then(Value::as_bool).unwrap_or(true);
    Ok(item::collect(&paths, recursive, allow_other, &[]).items)
}

/// B-5: orders `items` by `recipe.extra["fileOrder"]` when present: a list of entries, each a
/// string `"folder/name"` (or just `"name"`) or an object `{"folder": …, "name": …}`. An item
/// matches the first entry whose name equals its file name and whose folder (if given) is the end of
/// its parent folder (`/` or backslash, case-insensitive). Matching items come first in list order,
/// the others keep their order after them.
pub fn apply_file_order(recipe: &Recipe, items: &mut [ItemInfo]) {
    let Some(Value::Array(list)) = recipe.extra.get("fileOrder") else {
        return;
    };
    let norm = |s: &str| s.replace('\\', "/").trim_matches('/').to_lowercase();
    let entries: Vec<(String, String)> = list
        .iter()
        .filter_map(|e| match e {
            Value::String(s) => {
                let s = norm(s);
                Some(match s.rsplit_once('/') {
                    Some((f, n)) => (f.to_string(), n.to_string()),
                    None => (String::new(), s),
                })
            }
            Value::Object(o) => Some((
                norm(o.get("folder").and_then(Value::as_str).unwrap_or_default()),
                norm(o.get("name").and_then(Value::as_str)?),
            )),
            _ => None,
        })
        .collect();
    if entries.is_empty() {
        return;
    }
    let rank = |it: &ItemInfo| -> usize {
        let name = it
            .path
            .file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let parent = it
            .path
            .parent()
            .map(|p| norm(&p.to_string_lossy()))
            .unwrap_or_default();
        entries
            .iter()
            .position(|(f, n)| {
                *n == name && (f.is_empty() || parent == *f || parent.ends_with(&format!("/{f}")))
            })
            .unwrap_or(usize::MAX)
    };
    items.sort_by_key(rank); // stable: unmatched items keep their order
}

/// Überschreibungen aus den Parametern (`out`, `conflict`).
pub fn apply_overrides(recipe: &mut Recipe, p: &Value) -> Result<(), String> {
    if let Some(out) = p
        .get("out")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
    {
        recipe.output.target.kind = TargetKind::Folder;
        recipe.output.target.path = Some(PathBuf::from(out));
    }
    if let Some(c) = p.get("conflict").and_then(Value::as_str) {
        recipe.output.conflict = match c {
            "rename" => Conflict::Rename,
            "skip" => Conflict::Skip,
            "overwrite" => Conflict::Overwrite,
            "ask" => Conflict::Ask,
            other => {
                return Err(fp_i18n::tr!(
                    "batch-err-conflict-value",
                    value = other.to_string()
                ));
            }
        };
    }
    Ok(())
}

/// Base64 (Standardalphabet, mit Auffüllung).
pub fn base64(data: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16
            | (*c.get(1).unwrap_or(&0) as u32) << 8
            | *c.get(2).unwrap_or(&0) as u32;
        out.push(A[(n >> 18) as usize & 63] as char);
        out.push(A[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 {
            A[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if c.len() > 2 {
            A[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

fn png_base64(img: &fp_raster::RgbaImage) -> String {
    let item = crate::item::Item::Still(crate::item::Still {
        image: img.clone(),
        meta: crate::item::Meta {
            has_transparency: true,
            ..Default::default()
        },
    });
    crate::output::encode(
        &item,
        crate::output::OutputFormat::Png,
        &serde_json::Map::new(),
    )
    .map(|(b, _)| base64(&b))
    .unwrap_or_default()
}

impl BatchHost for Host {
    fn handle(&self, _session: &mut Session, op: BatchOp, p: &Value) -> Result<Value, String> {
        let lang = p
            .get("language")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(fp_i18n::current_language);
        fp_i18n::with_language(&lang, || handle(op, p))
    }
}

fn handle(op: BatchOp, p: &Value) -> Result<Value, String> {
    let reg = StepRegistry::global();
    match op {
        BatchOp::Steps => Ok(reg.catalog_json()),
        BatchOp::Validate => {
            let r = recipe_from_param(p.get("recipe"))?;
            let issues = r.validate(&reg);
            let ok = !issues
                .iter()
                .any(|i| i.severity == crate::step::Severity::Error);
            Ok(json!({"ok": ok, "issues": issues}))
        }
        BatchOp::Plan | BatchOp::Run => {
            let mut r = recipe_from_param(p.get("recipe"))?;
            apply_overrides(&mut r, p)?;
            let items = items_from_params(p, pipeline::is_file_only(&r, &reg))?;
            if op == BatchOp::Plan {
                let plan = executor::plan(&r, &items, &reg, &PlanOptions::default())
                    .map_err(|e| e.to_string())?;
                return serde_json::to_value(plan).map_err(|e| e.to_string());
            }
            let opts = RunOptions {
                dry_run: p.get("dryRun").and_then(Value::as_bool).unwrap_or(false),
                threads: p.get("threads").and_then(Value::as_u64).map(|n| n as usize),
                log: true,
                caller: crate::security::current_caller(),
                ..RunOptions::default()
            };
            let report = executor::run(&r, &items, &opts).map_err(|e| e.to_string())?;
            let mut v = serde_json::to_value(&report).map_err(|e| e.to_string())?;
            v["summary"] = json!(report.summary_text());
            v["ok"] = json!(report.totals.failed == 0 && !report.cancelled);
            Ok(v)
        }
        BatchOp::Preview => {
            let r = recipe_from_param(p.get("recipe"))?;
            let file = p
                .get("file")
                .and_then(Value::as_str)
                .ok_or_else(|| fp_i18n::t("batch-err-no-files"))?;
            let info = item::probe(std::path::Path::new(file)).map_err(|e| e.to_string())?;
            let max = p
                .get("maxSize")
                .and_then(Value::as_u64)
                .unwrap_or(1024)
                .clamp(16, 8192) as u32;
            let opts = PreviewOptions {
                max_size: max,
                max_size_animation: max.min(480),
                encode_full: true,
                ..PreviewOptions::default()
            };
            let res = pipeline::preview(&r, &info, &reg, &opts).map_err(|e| e.to_string())?;
            Ok(json!({
                "format": res.format.name(),
                "before": {"width": res.before_size.0, "height": res.before_size.1, "bytes": res.bytes_before, "png": png_base64(&res.before)},
                "after": {"width": res.after_size.0, "height": res.after_size.1, "bytes": res.bytes_after, "png": res.after.as_ref().map(png_base64)},
                "warnings": res.warnings,
                "ms": res.ms,
            }))
        }
    }
}
