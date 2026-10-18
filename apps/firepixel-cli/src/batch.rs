//! `firepixel-cli batch --recipe …` und `batch --steps` (moderner Batch-Modus, SPEC-BATCH §5.8).
//! Der alte Aufruf `batch --actions …` bleibt unverändert in `main.rs`.
//!
//! ```text
//! firepixel-cli batch --recipe rezept.fpbatch.json (--in <ordner> [--recursive] | <dateien…>)
//!                     [--out <ordner>] [--conflict rename|skip|overwrite] [--dry-run] [--threads N]
//!                     [--report bericht.json] [--json]
//! firepixel-cli batch --steps [--json]
//! ```
//! Exit-Code 0 = alles fertig, 1 = mindestens ein Fehler, 2 = Aufruf falsch, 3 = abgebrochen,
//! 130 = cancelled with Ctrl+C (running files finish or are discarded, temp files are removed).
//! Fortschritt auf stderr (eine Zeile je Datei, mit `--json` als NDJSON).

use clap::{Arg, ArgAction, ArgMatches, Command};
use fp_batch::{Progress, Recipe, RunOptions, StepRegistry, executor};
use fp_i18n::{t, tr};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Exit code after Ctrl+C (shell convention 128 + SIGINT).
pub const EXIT_INTERRUPTED: u8 = 130;

/// Prüft die Kommandozeile: `batch` mit `--recipe` oder `--steps` übernimmt dieses Modul (sonst
/// `None` → alter Weg in `main.rs`). Steckt außerdem die Batch-Laufzeit für `batch.*`-Befehle ein.
pub fn intercept() -> Option<ExitCode> {
    fp_batch::install();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    let mut json_out = false;
    while i < args.len() {
        match args[i].as_str() {
            "--json" => json_out = true,
            "--lang" => i += 1,
            a if a.starts_with("--lang=") => {}
            _ => break,
        }
        i += 1;
    }
    if args.get(i).map(String::as_str) != Some("batch") {
        return None;
    }
    let rest = &args[i + 1..];
    let legacy = rest
        .iter()
        .any(|a| a == "--actions" || a.starts_with("--actions="));
    // `batch --help` ohne `--actions`: Hilfe des Rezept-Modus samt Vorlagen und der älteren Form (BATCH-11b)
    if !legacy && rest.iter().any(|a| a == "--help" || a == "-h") {
        let mut c = command();
        println!(
            "{}",
            crate::localize_help_text(&c.render_long_help().to_string())
        );
        println!("{}", t("batch-cli-help-templates"));
        for tpl in fp_batch::templates::all() {
            println!("  {:<22} {}", tpl.id, tpl.title);
        }
        println!();
        println!("{}", t("batch-cli-help-legacy"));
        return Some(ExitCode::SUCCESS);
    }
    if !rest.iter().any(|a| {
        a == "--recipe" || a.starts_with("--recipe=") || a == "--steps" || a == "--templates"
    }) {
        return None;
    }
    let m = match command()
        .try_get_matches_from(std::iter::once("batch".to_string()).chain(rest.iter().cloned()))
    {
        Ok(m) => m,
        Err(e) => {
            use clap::error::ErrorKind;
            if matches!(e.kind(), ErrorKind::DisplayHelp | ErrorKind::DisplayVersion) {
                print!("{}", crate::localize_help_text(&e.render().to_string()));
                return Some(ExitCode::SUCCESS);
            }
            eprintln!(
                "{}",
                tr!(
                    "batch-cli-err-args",
                    error = e.to_string().trim().to_string()
                )
            );
            return Some(ExitCode::from(2));
        }
    };
    Some(run(&m, json_out || m.get_flag("json")))
}

fn command() -> Command {
    let c = Command::new("batch")
        .bin_name("firepixel-cli batch")
        .about(t("batch-cli-about"))
        .after_help(t("batch-cli-examples"))
        .disable_version_flag(true)
        .arg(
            Arg::new("recipe")
                .long("recipe")
                .value_name(t("batch-cli-value-recipe"))
                .help(t("batch-cli-arg-recipe")),
        )
        .arg(
            Arg::new("steps")
                .long("steps")
                .action(ArgAction::SetTrue)
                .help(t("batch-cli-arg-steps")),
        )
        .arg(
            Arg::new("templates")
                .long("templates")
                .action(ArgAction::SetTrue)
                .help(t("batch-cli-arg-templates")),
        ) // BATCH-9
        .arg(
            Arg::new("in")
                .long("in")
                .value_name(t("batch-cli-value-folder"))
                .help(t("batch-cli-arg-in")),
        )
        .arg(
            Arg::new("recursive")
                .long("recursive")
                .action(ArgAction::SetTrue)
                .help(t("batch-cli-arg-recursive")),
        )
        .arg(
            Arg::new("files")
                .num_args(0..)
                .value_name(t("batch-cli-value-files"))
                .help(t("batch-cli-arg-files")),
        )
        .arg(
            Arg::new("out")
                .long("out")
                .value_name(t("batch-cli-value-folder"))
                .help(t("batch-cli-arg-out")),
        )
        .arg(
            Arg::new("conflict")
                .long("conflict")
                .value_parser(["rename", "skip", "overwrite"])
                .value_name(t("batch-cli-value-rule"))
                .help(t("batch-cli-arg-conflict")),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(ArgAction::SetTrue)
                .help(t("batch-cli-arg-dry-run")),
        )
        .arg(
            Arg::new("threads")
                .long("threads")
                .value_parser(clap::value_parser!(usize))
                .value_name("N")
                .help(t("batch-cli-arg-threads")),
        )
        .arg(
            Arg::new("report")
                .long("report")
                .value_name(t("batch-cli-value-file"))
                .help(t("batch-cli-arg-report")),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(ArgAction::SetTrue)
                .help(t("batch-cli-arg-json")),
        )
        .arg(
            Arg::new("lang")
                .long("lang")
                .value_name("CODE")
                .help(t("batch-cli-arg-lang")),
        );
    crate::localize_scaffold(c)
}

fn fail(json_out: bool, code: u8, msg: String) -> ExitCode {
    if json_out {
        println!("{}", json!({"ok": false, "error": msg}));
    } else {
        eprintln!("{}", tr!("cli-error", message = msg));
    }
    ExitCode::from(code)
}

fn load_recipe(arg: &str) -> Result<Recipe, String> {
    let a = arg.trim();
    if a.starts_with('{') {
        return Recipe::from_json(a).map_err(|e| e.to_string());
    }
    // BATCH-9: Name einer eingebauten Vorlage (eine vorhandene Datei hat Vorrang)
    if !a.starts_with('@')
        && !std::path::Path::new(a).exists()
        && let Some(r) = fp_batch::templates::recipe(a)
    {
        return Ok(r);
    }
    Recipe::load(std::path::Path::new(a.strip_prefix('@').unwrap_or(a))).map_err(|e| e.to_string())
}

fn print_steps(json_out: bool) -> ExitCode {
    let cat = StepRegistry::global().catalog_json();
    if json_out {
        println!("{}", serde_json::to_string_pretty(&cat).unwrap_or_default());
        return ExitCode::SUCCESS;
    }
    let mut last_group = String::new();
    for s in cat["steps"].as_array().cloned().unwrap_or_default() {
        let group = s["group"].as_str().unwrap_or_default().to_string();
        if group != last_group {
            println!("{}", t(&format!("batch-group-{group}")));
            last_group = group;
        }
        let params: Vec<&str> = s["params"]
            .as_array()
            .map(|a| a.iter().filter_map(|p| p["name"].as_str()).collect())
            .unwrap_or_default();
        println!(
            "  {:<22} {}  [{}]",
            s["id"].as_str().unwrap_or_default(),
            s["name"].as_str().unwrap_or_default(),
            params.join(", ")
        );
    }
    ExitCode::SUCCESS
}

/// `batch --templates [--json]`: eingebaute Vorlagen (BATCH-9).
fn print_templates(json_out: bool) -> ExitCode {
    if json_out {
        println!(
            "{}",
            serde_json::to_string_pretty(&fp_batch::templates::catalog_json()).unwrap_or_default()
        );
        return ExitCode::SUCCESS;
    }
    println!("{}", t("batch-cli-templates-header"));
    for tp in fp_batch::templates::all() {
        println!("  {:<22} {} – {}", tp.id, tp.title, tp.description);
    }
    ExitCode::SUCCESS
}

fn run(m: &ArgMatches, json_out: bool) -> ExitCode {
    if m.get_flag("steps") {
        return print_steps(json_out);
    }
    if m.get_flag("templates") {
        return print_templates(json_out); // BATCH-9
    }
    let Some(recipe_arg) = m.get_one::<String>("recipe") else {
        return fail(json_out, 2, t("batch-err-recipe-missing"));
    };
    let mut recipe = match load_recipe(recipe_arg) {
        Ok(r) => r,
        Err(e) => return fail(json_out, 2, e),
    };
    let mut over = json!({});
    if let Some(o) = m.get_one::<String>("out") {
        over["out"] = json!(o);
    }
    if let Some(c) = m.get_one::<String>("conflict") {
        over["conflict"] = json!(c);
    }
    if let Err(e) = fp_batch::host::apply_overrides(&mut recipe, &over) {
        return fail(json_out, 2, e);
    }
    let reg = StepRegistry::global();
    let mut paths: Vec<PathBuf> = m
        .get_many::<String>("files")
        .map(|v| v.map(PathBuf::from).collect())
        .unwrap_or_default();
    if let Some(dir) = m.get_one::<String>("in") {
        paths.push(PathBuf::from(dir));
    }
    if paths.is_empty() {
        return fail(json_out, 2, t("batch-err-no-files"));
    }
    let collected = fp_batch::collect(
        &paths,
        m.get_flag("recursive"),
        fp_batch::pipeline::is_file_only(&recipe, &reg),
        &[],
    );
    for p in &collected.skipped {
        eprintln!(
            "{}",
            tr!("batch-cli-skipped-file", path = p.display().to_string())
        );
    }
    let errors: Vec<String> = recipe
        .validate(&reg)
        .into_iter()
        .filter(|i| i.severity == fp_batch::Severity::Error)
        .map(|i| i.message)
        .collect();
    if !errors.is_empty() {
        return fail(json_out, 2, errors.join("\n"));
    }
    let opts = RunOptions {
        dry_run: m.get_flag("dry-run"),
        threads: m.get_one::<usize>("threads").copied(),
        report_path: m.get_one::<String>("report").map(PathBuf::from),
        log: true,
        ..RunOptions::default()
    };
    let items = collected.items;
    let sources: Vec<PathBuf> = items.iter().map(|i| i.path.clone()).collect();
    let handle = executor::spawn(recipe, items, opts);
    let interrupted = Arc::new(AtomicBool::new(false));
    watch_ctrl_c(handle.cancel.clone(), interrupted.clone());
    let total = sources.len();
    let mut n = 0usize;
    let src = |i: usize| {
        sources
            .get(i)
            .map(|p| p.display().to_string())
            .unwrap_or_default()
    };
    while let Ok(ev) = handle.events.recv() {
        let line: Option<(Value, String)> = match &ev {
            Progress::ItemDone {
                index,
                target,
                bytes_out,
            } => {
                n += 1;
                Some((
                    json!({"event": "done", "index": index, "source": src(*index), "target": target, "bytesOut": bytes_out}),
                    tr!(
                        "batch-cli-progress-done",
                        n = n,
                        total = total,
                        source = src(*index),
                        target = target.display().to_string()
                    ),
                ))
            }
            Progress::ItemSkipped { index, reason } => {
                n += 1;
                Some((
                    json!({"event": "skipped", "index": index, "source": src(*index), "reason": reason}),
                    tr!(
                        "batch-cli-progress-skipped",
                        n = n,
                        total = total,
                        source = src(*index),
                        reason = reason.clone()
                    ),
                ))
            }
            Progress::ItemFailed {
                index,
                message,
                step,
            } => {
                n += 1;
                Some((
                    json!({"event": "failed", "index": index, "source": src(*index), "step": step, "error": message}),
                    tr!(
                        "batch-cli-progress-failed",
                        n = n,
                        total = total,
                        source = src(*index),
                        error = message.clone()
                    ),
                ))
            }
            Progress::Finished(_) => break,
            _ => None,
        };
        if let Some((j, text)) = line {
            if json_out {
                eprintln!("{j}");
            } else {
                eprintln!("{text}");
            }
        }
    }
    let report = match handle.join() {
        Ok(r) => r,
        Err(e) => return fail(json_out, 2, e.to_string()),
    };
    if json_out {
        println!("{}", report.to_json());
    } else {
        if report.dry_run {
            for i in &report.items {
                println!(
                    "{} → {}",
                    i.source.display(),
                    i.target
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_default()
                );
            }
        }
        println!("{}", report.summary_text());
        if let Some(p) = &report.report_file {
            println!(
                "{}",
                tr!("batch-cli-report-written", path = p.display().to_string())
            );
        }
    }
    ExitCode::from(exit_code(
        report.exit_code(),
        interrupted.load(Ordering::SeqCst),
    ))
}

/// Exit code of a run: 130 after Ctrl+C, otherwise the report's own code.
fn exit_code(report_code: u8, interrupted: bool) -> u8 {
    if interrupted {
        EXIT_INTERRUPTED
    } else {
        report_code
    }
}

/// Ctrl+C during `batch --recipe` (BA-106): the first press cancels the run cleanly (no new
/// files are started, running ones finish or are discarded, temp files are removed, the
/// summary is printed); a second press ends the process at once. Portable via `tokio::signal`;
/// if the handler cannot be installed, Ctrl+C keeps its default behaviour.
fn watch_ctrl_c(cancel: fp_batch::CancelToken, interrupted: Arc<AtomicBool>) {
    let _ = std::thread::Builder::new()
        .name("ctrl-c".into())
        .spawn(move || {
            let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            else {
                return;
            };
            rt.block_on(async move {
                while tokio::signal::ctrl_c().await.is_ok() {
                    if interrupted.swap(true, Ordering::SeqCst) {
                        std::process::exit(i32::from(EXIT_INTERRUPTED));
                    }
                    cancel.cancel();
                    eprintln!("{}", t("batch-cli-interrupted"));
                }
            });
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ctrl_c_exit_code_is_130() {
        assert_eq!(exit_code(0, false), 0);
        assert_eq!(exit_code(3, false), 3);
        assert_eq!(exit_code(3, true), EXIT_INTERRUPTED);
        assert_eq!(EXIT_INTERRUPTED, 130);
    }
}
