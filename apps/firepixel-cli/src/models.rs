//! `firepixel-cli models …` (F143): list, download and verify the AI models of the catalogue
//! (`fp_ai::models::MODELS`) without the user interface, e.g. on a batch server. Downloads go through
//! the same path as "Manage AI Models..." (`fp_ai::download`: official source, `*.part` file,
//! size and SHA-256 check, then rename). Calling `models download` is the explicit consent; the
//! source, size and licence are printed before each download.

use clap::Subcommand;
use fp_ai::models::{self, ModelId, ModelInfo};
use fp_ai::{Progress, download};
use fp_engine::commands::ai::fp_ai;
use fp_i18n::{t, tr};
use serde_json::{Value, json};
use std::io::Write;
use std::time::Instant;

/// Sub-commands of `models`.
#[derive(Subcommand, Debug)]
pub enum ModelsCmd {
    // help texts: cli.ftl (`cli-models-…`)
    List,
    Download {
        #[arg(required = true)]
        models: Vec<String>,
        #[arg(long)]
        force: bool,
    },
    Verify {
        models: Vec<String>,
    },
}

/// Runs a `models` sub-command. `Err` = message (exit code 1).
pub fn run(cmd: ModelsCmd, json_out: bool) -> Result<Value, String> {
    sync_folder();
    match cmd {
        ModelsCmd::List => Ok(list()),
        ModelsCmd::Download { models, force } => download_all(&resolve(&models)?, force, json_out),
        ModelsCmd::Verify { models } => verify_all(&resolve(&models)?),
    }
}

/// Model folder from the preferences (`AiModelFolder`), as in the user interface.
fn sync_folder() {
    let s = fp_engine::Session::default();
    let f = s.prefs.get_str("AiModelFolder", "");
    models::set_user_folder((!f.trim().is_empty()).then(|| f.trim().into()));
}

fn id_name(id: ModelId) -> String {
    serde_json::to_value(id)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
}

/// Model names (`sam2Tiny`, file name …) or `all`; empty = all.
fn resolve(names: &[String]) -> Result<Vec<ModelId>, String> {
    if names.is_empty() || names.iter().any(|n| n.eq_ignore_ascii_case("all")) {
        return Ok(models::MODELS.iter().map(|m| m.id).collect());
    }
    let mut out = Vec::new();
    for n in names {
        let id = models::by_name(n).ok_or_else(|| {
            let known: Vec<String> = models::MODELS.iter().map(|m| id_name(m.id)).collect();
            tr!(
                "cli-models-unknown",
                name = n.as_str(),
                known = known.join(", ")
            )
        })?;
        if !out.contains(&id) {
            out.push(id);
        }
    }
    Ok(out)
}

fn model_json(m: &ModelInfo) -> Value {
    json!({
        "id": id_name(m.id),
        "name": m.display_name(),
        "file": m.file,
        "size": m.size,
        "license": m.license,
        "url": m.url,
        "available": models::available(m.id),
        "path": models::find(m.id).map(|p| p.display().to_string()),
    })
}

fn list() -> Value {
    let mut text = tr!(
        "cli-models-folder",
        folder = models::download_folder().display().to_string()
    );
    for m in models::MODELS {
        let state = match models::find(m.id) {
            Some(p) => tr!("cli-models-present", path = p.display().to_string()),
            None => t("cli-models-absent"),
        };
        text.push_str(&format!(
            "\n  {:<22} {:<38} {:>7}  {:<13} {state}",
            id_name(m.id),
            m.display_name(),
            models::human_size(m.size),
            m.license
        ));
    }
    json!({
        "text": text,
        "folder": models::download_folder().display().to_string(),
        "folders": models::search_folders().iter().map(|f| f.display().to_string()).collect::<Vec<_>>(),
        "models": models::MODELS.iter().map(model_json).collect::<Vec<_>>(),
    })
}

/// Progress on stderr in steps of 10 % (stdout stays free for `--json`).
fn with_progress<T>(
    f: impl FnOnce(&Progress) -> fp_ai::Result<T> + Send,
    label: &str,
) -> fp_ai::Result<T>
where
    T: Send,
{
    let prog = Progress::new();
    std::thread::scope(|sc| {
        let p2 = prog.clone();
        let h = sc.spawn(move || f(&p2));
        let mut last = -1i32;
        while !h.is_finished() {
            let step = (prog.get() * 10.0).floor() as i32;
            if step != last {
                last = step;
                eprint!("\r{label} {:>3} %", step.clamp(0, 10) * 10);
                let _ = std::io::stderr().flush();
            }
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
        eprintln!("\r{label} 100 %");
        h.join().unwrap_or(Err(fp_ai::AiError::Cancelled))
    })
}

fn download_all(ids: &[ModelId], force: bool, json_out: bool) -> Result<Value, String> {
    if !download::allowed() {
        return Err(fp_ai::AiError::DownloadDisabled.to_string());
    }
    let mut lines = Vec::new();
    let mut rows = Vec::new();
    for &id in ids {
        let m = models::info(id);
        if !force && let Ok(p) = download::verify(id, None) {
            lines.push(tr!(
                "cli-models-skip",
                name = m.display_name(),
                path = p.display().to_string()
            ));
            rows.push(json!({"id": id_name(id), "skipped": true, "path": p.display().to_string()}));
            continue;
        }
        let head = tr!(
            "cli-models-dl-start",
            name = m.display_name(),
            url = m.url,
            size = models::human_size(m.size),
            license = m.license
        );
        if json_out {
            eprintln!("{head}");
        } else {
            println!("{head}");
        }
        let t0 = Instant::now();
        let p = with_progress(|p| download::download(id, p), m.file).map_err(|e| e.to_string())?;
        let secs = t0.elapsed().as_secs_f64();
        let mbps = m.size as f64 / 1e6 / secs.max(1e-3);
        lines.push(tr!(
            "cli-models-dl-done",
            name = m.display_name(),
            path = p.display().to_string(),
            seconds = format!("{secs:.1}"),
            rate = format!("{mbps:.1}")
        ));
        rows.push(json!({"id": id_name(id), "path": p.display().to_string(), "size": m.size, "seconds": secs, "sha256": m.sha256}));
    }
    Ok(json!({"text": lines.join("\n"), "ok": true, "models": rows}))
}

fn verify_all(ids: &[ModelId]) -> Result<Value, String> {
    let mut lines = Vec::new();
    let mut rows = Vec::new();
    let mut bad = 0;
    for &id in ids {
        let m = models::info(id);
        if !models::available(id) {
            lines.push(format!("{}: {}", m.display_name(), t("cli-models-absent")));
            rows.push(json!({"id": id_name(id), "available": false}));
            continue;
        }
        let t0 = Instant::now();
        match download::verify(id, None) {
            Ok(p) => {
                lines.push(tr!(
                    "cli-models-verify-ok",
                    name = m.display_name(),
                    path = p.display().to_string(),
                    seconds = format!("{:.1}", t0.elapsed().as_secs_f64())
                ));
                rows.push(json!({"id": id_name(id), "available": true, "ok": true}));
            }
            Err(e) => {
                bad += 1;
                lines.push(format!("{}: {e}", m.display_name()));
                rows.push(json!({"id": id_name(id), "available": true, "ok": false, "error": e.to_string()}));
            }
        }
    }
    if bad > 0 {
        return Err(lines.join("\n"));
    }
    Ok(json!({"text": lines.join("\n"), "ok": true, "models": rows}))
}
