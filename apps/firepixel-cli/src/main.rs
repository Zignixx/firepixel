//! `firepixel-cli`: Kommandozeile von Firepixel (ARCHITEKTUR §6).
//!
//! Alle Unterbefehle arbeiten headless über dasselbe Befehlsregister wie Oberfläche, MCP und JSF.
//! Mit `--json` sind die Ausgaben maschinenlesbar (ein JSON-Objekt auf stdout, Fehler als
//! `{"ok": false, "error": …}` mit Exit-Code 1).

use clap::{ArgMatches, CommandFactory, FromArgMatches, Parser, Subcommand};
use fp_i18n::{t, tr};
use fp_engine::Session;
use fp_mcp::docjson::{self, DocJsonOptions};
use fp_mcp::render::{self as fprender, RenderRequest};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

mod language; // Unterbefehl `language`
mod batch; // BATCH-1: `batch --recipe` und `batch --steps` (fp-batch)

#[derive(Parser, Debug)]
#[command(name = "firepixel-cli", version)]
struct Cli {
    // Hilfetexte aller Befehle und Optionen: cli.ftl (`cli-…`, siehe `localize`)
    #[arg(long, global = true)]
    json: bool,
    #[arg(long, global = true, value_name = "CODE")]
    lang: Option<String>,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    // Hilfetexte: cli.ftl (`cli-<befehl>`, `.examples`, `cli-<befehl>-arg-<option>`)
    Run {
        file: Option<PathBuf>,
        #[arg(long, value_name = "WxH")]
        new: Option<String>,
        #[arg(long = "cmd", value_name = "NAME")]
        cmds: Vec<String>,
        #[arg(long = "params", value_name = "JSON")]
        params: Vec<String>,
        #[arg(long, value_name = "JSON|FILE")]
        steps: Option<String>,
        #[arg(long)]
        group: bool,
        #[arg(long, value_name = "FILE")]
        out: Option<PathBuf>,
        #[arg(long)]
        in_place: bool,
    },
    Batch {
        #[arg(long, value_name = "JSON|FILE")]
        actions: String,
        #[arg(long = "in", value_name = "FOLDER")]
        input: PathBuf,
        #[arg(long = "out", value_name = "FOLDER")]
        output: PathBuf,
        #[arg(long, default_value = "fwpng")]
        format: String,
        #[arg(long)]
        filter: Option<String>,
    },
    Render {
        file: PathBuf,
        #[arg(long)]
        page: Option<usize>,
        #[arg(long)]
        state: Option<usize>,
        #[arg(long, default_value_t = 1.0)]
        zoom: f64,
        #[arg(long, value_name = "L,T,R,B")]
        rect: Option<String>,
        #[arg(long)]
        element: Option<u64>,
        #[arg(long, value_name = "FILE")]
        out: PathBuf,
    },
    Info {
        file: PathBuf,
        #[arg(long, default_value_t = 8)]
        depth: usize,
        #[arg(long)]
        all_pages: bool,
        #[arg(long)]
        page: Option<usize>,
        #[arg(long, value_delimiter = ',')]
        types: Vec<String>,
        #[arg(long)]
        custom_data: bool,
    },
    Commands {
        #[arg(long)]
        filter: Option<String>,
        #[arg(long)]
        category: Option<String>,
        #[arg(long)]
        all: bool,
    },
    Convert { input: PathBuf, output: PathBuf },
    Script {
        script: Option<PathBuf>,
        file: Option<PathBuf>,
        #[arg(long)]
        code: Option<String>,
        #[arg(long, value_name = "FILE")]
        out: Option<PathBuf>,
        #[arg(long)]
        arg: Option<String>,
    },
    Mcp {
        #[arg(long, conflicts_with = "bridge")]
        http: bool,
        #[arg(long)]
        bridge: bool,
        #[arg(long, default_value_t = fp_mcp::DEFAULT_PORT)]
        port: u16,
        #[arg(long)]
        token: Option<String>,
        #[arg(long)]
        token_file: Option<PathBuf>,
    },
    #[command(subcommand, alias = "sprache")]
    Language(language::LanguageCmd),
}

/// Fehler mit Meldung.
struct Fail(String);

impl<E: std::fmt::Display> From<E> for Fail {
    fn from(e: E) -> Self {
        Fail(e.to_string())
    }
}

type Res<T> = Result<T, Fail>;

fn main() -> ExitCode {
    // alte deutsche Ordnernamen einmal auf die englischen umbenennen (AGENTEN_REGELN §1c; nicht in
    // Testläufen mit FP_NO_SHELL_OPEN, siehe fp_i18n::paths)
    fp_i18n::paths::enable_migration();
    init_language();
    if let Some(code) = batch::intercept() {
        return code;
    }
    let matches = match localized_command().try_get_matches() {
        Ok(m) => m,
        Err(e) => return clap_error(e),
    };
    let cli = match Cli::from_arg_matches(&matches) {
        Ok(c) => c,
        Err(e) => return clap_error(e),
    };
    let json_out = cli.json;
    let sub = matches.subcommand().map(|(_, m)| m.clone());
    let r = run(cli, sub.as_ref());
    match r {
        Ok(v) => {
            if json_out {
                println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
            } else if let Some(t) = v.get("text").and_then(|t| t.as_str()) {
                println!("{t}");
            } else if !v.is_null() {
                println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
            }
            ExitCode::SUCCESS
        }
        Err(Fail(m)) => {
            if json_out {
                println!("{}", json!({"ok": false, "error": m}));
            } else {
                eprintln!("{}", tr!("cli-error", message = m));
            }
            ExitCode::from(1)
        }
    }
}

// ------------------------------------------------------------------ Sprache und Hilfe

/// Sprache der Kommandozeile: `--lang <code>` (auch `--lang=<code>`), sonst `FIREPIXEL_LANG`, sonst
/// die Systemsprache. Sprachordner (`Languages\` neben der exe, `%APPDATA%\Firepixel\Languages`; alt `Sprachen`)
/// werden vorher geladen, damit auch eigene Sprachen gehen.
fn init_language() {
    fp_i18n::load_default_dirs();
    let args: Vec<String> = std::env::args().collect();
    let from_args = args.iter().enumerate().find_map(|(i, a)| match a.strip_prefix("--lang=") {
        Some(v) => Some(v.to_string()),
        None if a == "--lang" => args.get(i + 1).cloned(),
        None => None,
    });
    let code = from_args.or_else(|| std::env::var("FIREPIXEL_LANG").ok().filter(|s| !s.trim().is_empty())).unwrap_or_else(fp_i18n::system_language);
    if !fp_i18n::set_language(&code) {
        fp_i18n::set_language(&fp_i18n::system_language());
    }
}

/// Der clap-Befehl mit allen Hilfetexten in der aktuellen Sprache.
fn localized_command() -> clap::Command {
    let version = clap::Arg::new("version").short('V').long("version").action(clap::ArgAction::Version).help(t("cli-version")).help_heading(t("cli-heading-options"));
    localize(Cli::command(), "cli-root").disable_version_flag(true).arg(version)
}

/// Setzt Beschreibung (`<key>`), Beispiele (`<key>.examples`), Optionstexte (`<key>-arg-<id>`, Wertname
/// `.value`), Überschriften, Hilfe-Option und Vorlage – rekursiv für alle Unterbefehle
/// (`cli-<befehl>`, `cli-language-<befehl>`).
fn localize(mut c: clap::Command, key: &str) -> clap::Command {
    use fp_i18n::has;
    if has(key) {
        c = c.about(t(key));
    }
    let examples = format!("{key}.examples");
    if has(&examples) {
        c = c.after_help(t(&examples));
    }
    let (h_args, h_opts) = (t("cli-heading-arguments"), t("cli-heading-options"));
    let ids: Vec<(String, bool)> = c.get_arguments().map(|a| (a.get_id().to_string(), a.is_positional())).collect();
    for (id, positional) in ids {
        let k = format!("{key}-arg-{}", id.replace('_', "-"));
        let k = if has(&k) { k } else { format!("cli-root-arg-{}", id.replace('_', "-")) };
        let heading = if positional { h_args.clone() } else { h_opts.clone() };
        c = c.mut_arg(&id, |mut a| {
            if has(&k) {
                a = a.help(t(&k));
            }
            let v = format!("{k}.value");
            if has(&v) {
                a = a.value_name(t(&v));
            }
            a.help_heading(heading)
        });
    }
    let help = clap::Arg::new("help").short('h').long("help").action(clap::ArgAction::Help).help(t("cli-help")).help_heading(h_opts);
    c = c
        .disable_help_flag(true)
        .arg(help)
        .help_template(format!("{{about-with-newline}}\n{} {{usage}}\n\n{{all-args}}{{after-help}}", t("cli-usage")))
        .subcommand_help_heading(t("cli-heading-commands"))
        .subcommand_value_name(t("cli-value-command"));
    let subs: Vec<String> = c.get_subcommands().map(|s| s.get_name().to_string()).collect();
    if !subs.is_empty() {
        c = c.disable_help_subcommand(true);
    }
    let base = if key == "cli-root" { "cli".to_string() } else { key.to_string() };
    for name in subs {
        let sk = format!("{base}-{name}");
        c = c.mut_subcommand(&name, |sc| localize(sc, &sk));
    }
    c
}

/// Fehler beim Lesen der Kommandozeile in der aktuellen Sprache (Hilfe/Version unverändert).
fn clap_error(e: clap::Error) -> ExitCode {
    use clap::error::{ContextKind, ContextValue, ErrorKind};
    match e.kind() {
        ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
            let _ = e.print();
            return ExitCode::SUCCESS;
        }
        ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => {
            let _ = e.print();
            return ExitCode::from(2);
        }
        _ => {}
    }
    let ctx = |k: ContextKind| {
        e.get(k)
            .map(|v| match v {
                ContextValue::String(s) => s.clone(),
                ContextValue::Strings(v) => v.join(", "),
                other => other.to_string(),
            })
            .unwrap_or_default()
    };
    let value = [ContextKind::InvalidValue, ContextKind::InvalidSubcommand].into_iter().map(ctx).find(|v| !v.is_empty()).unwrap_or_default();
    let key = match e.kind() {
        ErrorKind::UnknownArgument => "cli-err-unknown-argument",
        ErrorKind::InvalidSubcommand => "cli-err-unknown-subcommand",
        ErrorKind::InvalidValue | ErrorKind::ValueValidation => "cli-err-invalid-value",
        ErrorKind::MissingRequiredArgument => "cli-err-missing-argument",
        ErrorKind::MissingSubcommand => "cli-err-missing-subcommand",
        ErrorKind::ArgumentConflict => "cli-err-conflict",
        ErrorKind::TooManyValues | ErrorKind::TooFewValues | ErrorKind::WrongNumberOfValues => "cli-err-number-of-values",
        _ => "cli-err-invalid-call",
    };
    eprintln!("{}", tr!(key, arg = ctx(ContextKind::InvalidArg), value = value, other = ctx(ContextKind::PriorArg)));
    eprintln!("{}", t("cli-err-help-hint"));
    ExitCode::from(2)
}

fn run(cli: Cli, sub: Option<&ArgMatches>) -> Res<Value> {
    let json_out = cli.json;
    let _ = &cli.lang; // schon in `init_language` ausgewertet
    match cli.cmd {
        Cmd::Run { file, new, cmds, params, steps, group, out, in_place } => cmd_run(sub, file, new, cmds, params, steps, group, out, in_place),
        Cmd::Batch { actions, input, output, format, filter } => cmd_batch(&actions, &input, &output, &format, filter.as_deref()),
        Cmd::Render { file, page, state, zoom, rect, element, out } => cmd_render(&file, page, state, zoom, rect.as_deref(), element, &out),
        Cmd::Info { file, depth, all_pages, page, types, custom_data } => cmd_info(&file, depth, all_pages, page, types, custom_data, json_out),
        Cmd::Commands { filter, category, all } => cmd_commands(filter.as_deref(), category.as_deref(), all, json_out),
        Cmd::Convert { input, output } => cmd_convert(&input, &output),
        Cmd::Script { script, file, code, out, arg } => cmd_script(script, file, code, out, arg),
        Cmd::Mcp { http, bridge, port, token, token_file } => {
            if bridge {
                cmd_mcp_bridge(token_file)
            } else {
                cmd_mcp(http, port, token, token_file)
            }
        }
        Cmd::Language(c) => language::run(c, json_out).map_err(Fail),
    }
}

// ------------------------------------------------------------------ Hilfen

/// JSON-Text oder Datei mit JSON (`@datei` oder existierender Pfad).
fn json_arg(s: &str) -> Res<Value> {
    let t = s.trim();
    let text = if let Some(p) = t.strip_prefix('@') {
        std::fs::read_to_string(p).map_err(|e| Fail(format!("{p}: {e}")))?
    } else if !(t.starts_with('{') || t.starts_with('[')) && Path::new(t).is_file() {
        std::fs::read_to_string(t).map_err(|e| Fail(format!("{t}: {e}")))?
    } else {
        t.to_string()
    };
    serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|e| Fail(tr!("cli-err-json", error = e.to_string(), text = text.chars().take(80).collect::<String>())))
}

fn open(s: &mut Session, path: &Path) -> Res<()> {
    fp_script::fileio::open_document(s, path).map_err(|e| Fail(format!("{}: {e}", path.display())))?;
    Ok(())
}

/// `WxH[:#farbe]` → neues Dokument.
fn new_doc(s: &mut Session, spec: &str) -> Res<()> {
    let (size, color) = match spec.split_once(':') {
        Some((a, b)) => (a, Some(b)),
        None => (spec, None),
    };
    let (w, h) = size.split_once(['x', 'X']).ok_or_else(|| Fail(tr!("cli-err-new-size", spec = spec)))?;
    let w: f64 = w.trim().parse().map_err(|_| Fail(tr!("cli-err-width", value = w)))?;
    let h: f64 = h.trim().parse().map_err(|_| Fail(tr!("cli-err-height", value = h)))?;
    let mut p = json!({"width": w, "height": h});
    if let Some(c) = color {
        p["background"] = json!(c);
    }
    s.execute("file.new", p)?;
    Ok(())
}

fn is_flat_png(p: &Path) -> bool {
    let l = p.to_string_lossy().to_lowercase();
    l.ends_with(".png") && !l.ends_with(".fw.png")
}

/// Speichert das aktive Dokument: `.fw.png` editierbar, `.png` flach (PNG32), andere Endungen
/// (gif, jpg, webp, avif, bmp, tiff, qoi, ico, apng) über fp-export.
fn write_output(s: &mut Session, out: &Path) -> Res<Value> {
    let l = out.to_string_lossy().to_lowercase();
    if !l.ends_with(".png") {
        let id = s.doc()?.id;
        let n = fp_script::fileio::export_page(s, id, None, None, out, &Value::Null)
            .map_err(|e| Fail(tr!("cli-err-export-formats", error = e.to_string(), formats = fp_script::fileio::export_extensions().iter().map(|x| format!(".{x}")).collect::<Vec<_>>().join(", "))))?;
        return Ok(json!({"path": out.to_string_lossy(), "format": out.extension().map(|e| e.to_string_lossy().to_string()), "bytes": n}));
    }
    if is_flat_png(out) {
        let d = s.doc()?;
        let r = fprender::render(&fp_render::Renderer::default(), d, &RenderRequest { max_pixels: 400_000_000, ..RenderRequest::default() })?;
        if let Some(dir) = out.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(out, &r.png)?;
        return Ok(json!({"path": out.to_string_lossy(), "format": "png", "width": r.width, "height": r.height}));
    }
    s.execute("file.saveAs", json!({"path": out.to_string_lossy()}))?;
    Ok(json!({"path": out.to_string_lossy(), "format": "fwpng"}))
}

fn state_json(s: &Session) -> Value {
    match s.active() {
        Some(d) => json!({"doc": d.id.0, "undo": d.history.undo_count(), "selection": d.selection.elements.iter().map(|e| e.0).collect::<Vec<_>>()}),
        None => Value::Null,
    }
}

// ------------------------------------------------------------------ Unterbefehle

#[allow(clippy::too_many_arguments)]
fn cmd_run(
    sub: Option<&ArgMatches>,
    file: Option<PathBuf>,
    new: Option<String>,
    cmds: Vec<String>,
    params: Vec<String>,
    steps: Option<String>,
    group: bool,
    out: Option<PathBuf>,
    in_place: bool,
) -> Res<Value> {
    let mut s = new_session();
    match (&file, &new) {
        (Some(f), _) => open(&mut s, f)?,
        (None, Some(spec)) => new_doc(&mut s, spec)?,
        (None, None) => return Err(Fail(t("cli-err-input-or-new"))),
    }
    // --params dem vorangehenden --cmd zuordnen (über die Positionen in der Kommandozeile)
    let mut list: Vec<(String, Value)> = cmds.iter().map(|c| (c.clone(), Value::Null)).collect();
    if !params.is_empty() {
        let ci: Vec<usize> = sub.and_then(|m| m.indices_of("cmds")).map(|i| i.collect()).unwrap_or_default();
        let pi: Vec<usize> = sub.and_then(|m| m.indices_of("params")).map(|i| i.collect()).unwrap_or_default();
        for (k, p) in params.iter().enumerate() {
            let pos = pi.get(k).copied().unwrap_or(usize::MAX);
            let owner = ci.iter().rposition(|c| *c < pos).or(if ci.is_empty() { None } else { Some(k.min(ci.len() - 1)) });
            let Some(o) = owner else { return Err(Fail(t("cli-err-params-without-cmd"))) };
            list[o].1 = json_arg(p)?;
        }
    }
    if let Some(st) = &steps {
        let v = json_arg(st)?;
        let arr = v.get("steps").cloned().unwrap_or(v);
        for item in arr.as_array().ok_or_else(|| Fail(tr!("cli-err-json-list", option = "--steps")))? {
            let name = item.get("cmd").or_else(|| item.get("name")).or_else(|| item.get("command")).and_then(|n| n.as_str()).ok_or_else(|| Fail(t("cli-err-step-without-cmd")))?;
            list.push((name.to_string(), item.get("params").cloned().unwrap_or(Value::Null)));
        }
    }
    let mut results = Vec::new();
    let mut run_all = |s: &mut Session| -> Result<(), fp_engine::EngineError> {
        for (name, p) in &list {
            let o = s.execute(name, p.clone()).map_err(|e| fp_engine::EngineError::Failed(format!("{name}: {e}")))?;
            results.push(json!({"cmd": name, "result": o.value}));
        }
        Ok(())
    };
    if group { s.run_group("CLI", &mut run_all)? } else { run_all(&mut s)? }
    let saved = match (out, in_place, &file) {
        (Some(o), _, _) => Some(write_output(&mut s, &o)?),
        (None, true, Some(f)) => Some(write_output(&mut s, f)?),
        _ => None,
    };
    let n = results.len();
    Ok(json!({"ok": true, "commands": results, "saved": saved, "state": state_json(&s),
              "text": tr!("cli-msg-run-done", n = n, saved = saved.as_ref().and_then(|v| v.get("path")).and_then(|p| p.as_str()).map(|p| tr!("cli-msg-saved", path = p)).unwrap_or_default())}))
}

fn cmd_batch(actions: &str, input: &Path, output: &Path, format: &str, filter: Option<&str>) -> Res<Value> {
    let v = json_arg(actions)?;
    let arr = v.get("actions").cloned().unwrap_or(v);
    let list: Vec<(String, Value)> = arr
        .as_array()
        .ok_or_else(|| Fail(tr!("cli-err-json-list", option = "--actions")))?
        .iter()
        .map(|a| {
            let n = a.get("cmd").or_else(|| a.get("name")).or_else(|| a.get("command")).and_then(|n| n.as_str()).unwrap_or("").to_string();
            (n, a.get("params").cloned().unwrap_or(Value::Null))
        })
        .collect();
    if list.iter().any(|(n, _)| n.is_empty()) {
        return Err(Fail(t("cli-err-action-without-cmd")));
    }
    let out_ext = match format.trim_start_matches('.').to_lowercase().as_str() {
        "fwpng" | "fw.png" => "fw.png".to_string(),
        e if fp_script::fileio::export_extensions().contains(&e) => e.to_string(),
        other => return Err(Fail(tr!("cli-err-format", formats = fp_script::fileio::export_extensions().join(", "), value = other))),
    };
    std::fs::create_dir_all(output)?;
    let mut files: Vec<PathBuf> = std::fs::read_dir(input)
        .map_err(|e| Fail(format!("{}: {e}", input.display())))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            let l = p.to_string_lossy().to_lowercase();
            p.is_file() && fp_script::fileio::import_extensions().iter().any(|e| l.ends_with(&format!(".{e}")))
        })
        .filter(|p| filter.is_none_or(|f| p.file_name().is_some_and(|n| n.to_string_lossy().contains(f))))
        .collect();
    files.sort();
    let mut report = Vec::new();
    let mut failed = 0;
    for f in &files {
        let name = f.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let lower = name.to_lowercase();
        let base = if lower.ends_with(".fw.png") { name[..name.len() - 7].to_string() } else { f.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default() };
        let target = output.join(format!("{base}.{out_ext}"));
        let mut s = new_session();
        let r: Res<()> = (|| {
            open(&mut s, f)?;
            for (n, p) in &list {
                s.execute(n, p.clone()).map_err(|e| Fail(format!("{n}: {e}")))?;
            }
            write_output(&mut s, &target)?;
            Ok(())
        })();
        match r {
            Ok(()) => report.push(json!({"file": name, "ok": true, "out": target.to_string_lossy()})),
            Err(Fail(m)) => {
                failed += 1;
                report.push(json!({"file": name, "ok": false, "error": m}));
            }
        }
    }
    let ok = files.len() - failed;
    Ok(json!({"ok": failed == 0, "processed": files.len(), "failed": failed, "files": report, "text": tr!("cli-msg-batch-done", ok = ok, n = files.len(), dir = output.display().to_string())}))
}

fn parse_rect(s: &str) -> Res<fp_geom::Rect> {
    let v: Vec<f32> = s.split(',').map(|x| x.trim().parse::<f32>()).collect::<Result<_, _>>().map_err(|_| Fail(tr!("cli-err-rect", value = s)))?;
    if v.len() != 4 {
        return Err(Fail(t("cli-err-rect-four")));
    }
    Ok(fp_geom::Rect::new(v[0], v[1], v[2], v[3]))
}

fn cmd_render(file: &Path, page: Option<usize>, state: Option<usize>, zoom: f64, rect: Option<&str>, element: Option<u64>, out: &Path) -> Res<Value> {
    let mut s = new_session();
    open(&mut s, file)?;
    let req = RenderRequest { page, state, zoom, rect: rect.map(parse_rect).transpose()?, element, max_pixels: 400_000_000 };
    let d = s.doc()?;
    let r = fprender::render(&fp_render::Renderer::default(), d, &req)?;
    if let Some(dir) = out.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(out, &r.png)?;
    let mut m = r.meta();
    m["ok"] = json!(true);
    m["path"] = json!(out.to_string_lossy());
    m["text"] = json!(format!("{}×{} px → {}", r.width, r.height, out.display()));
    Ok(m)
}

fn tree_text(v: &Value) -> String {
    let mut out = String::new();
    out += &tr!("cli-msg-tree-doc", title = v["title"].as_str().unwrap_or(""), n = v["pages"].as_array().map_or(0, |a| a.len()));
    out.push('\n');
    fn el(out: &mut String, e: &Value, ind: usize) {
        let b = &e["bounds"];
        out.push_str(&format!(
            "{:ind$}#{} {} {}{} [{}, {}, {}, {}]\n",
            "",
            e["id"],
            e["type"].as_str().unwrap_or(""),
            e["name"].as_str().map(|n| format!("{} ", tr!("cli-msg-quoted", text = n))).unwrap_or_default(),
            e["text"].as_str().map(|t| format!("\"{}\" ", t.chars().take(30).collect::<String>())).unwrap_or_default(),
            b["left"],
            b["top"],
            b["right"],
            b["bottom"],
            ind = ind
        ));
        for c in e["children"].as_array().into_iter().flatten() {
            el(out, c, ind + 2);
        }
    }
    for p in v["pages"].as_array().into_iter().flatten() {
        out += &tr!("cli-msg-tree-page", index = p["index"].to_string(), name = p["name"].as_str().unwrap_or(""), width = p["width"].to_string(), height = p["height"].to_string(), states = p["states"].as_u64().unwrap_or(1) as usize, background = p["background"].as_str().unwrap_or(""));
        out.push('\n');
        for l in p["layers"].as_array().into_iter().flatten().rev() {
            let hidden = if l["visible"] == json!(false) { t("cli-msg-tree-hidden") } else { String::new() };
            out += &format!("  {}\n", tr!("cli-msg-tree-layer", id = l["id"].to_string(), name = l["name"].as_str().unwrap_or(""), kind = l["kind"].as_str().unwrap_or(""), hidden = hidden));
            for e in l["elements"].as_array().into_iter().flatten() {
                el(&mut out, e, 4);
            }
        }
    }
    out
}

fn cmd_info(file: &Path, depth: usize, all_pages: bool, page: Option<usize>, types: Vec<String>, custom_data: bool, json_out: bool) -> Res<Value> {
    let mut s = new_session();
    open(&mut s, file)?;
    let d = s.doc()?;
    let opts = DocJsonOptions { page, all_pages, depth, types, custom_data, max_elements: 100_000, ..DocJsonOptions::default() };
    let mut v = docjson::document_json(d, &opts);
    if !json_out {
        let t = tree_text(&v);
        v = json!({"text": t.trim_end()});
    }
    Ok(v)
}

fn cmd_commands(filter: Option<&str>, category: Option<&str>, all: bool, json_out: bool) -> Res<Value> {
    let s = new_session();
    let f = filter.map(|x| x.to_lowercase());
    let c = category.map(|x| x.to_lowercase());
    let mut list = Vec::new();
    let mut lines = Vec::new();
    for cmd in s.registry().iter() {
        let i = cmd.info();
        if !all && !i.implemented {
            continue;
        }
        if let Some(c) = &c {
            let n = i.name.to_lowercase();
            if !(n == *c || n.starts_with(&format!("{c}."))) {
                continue;
            }
        }
        if let Some(f) = &f {
            let hay = format!(
                "{} {} {} {} {} {} {}",
                i.name,
                i.label,
                fp_mcp::tools::command_label(i),
                i.description.clone().unwrap_or_default(),
                fp_mcp::tools::command_description(i).unwrap_or_default(),
                i.menu_path.join(" "),
                fp_mcp::tools::command_menu_path(i).join(" ")
            )
            .to_lowercase();
            if !hay.contains(f.as_str()) {
                continue;
            }
        }
        let ps: Vec<String> = i.params.iter().map(|p| if p.required { format!("{}*", p.name) } else { p.name.clone() }).collect();
        let label = fp_mcp::tools::command_label(i);
        lines.push(format!("{:<44} {}{}", i.name, label, if ps.is_empty() { String::new() } else { format!("  ({})", ps.join(", ")) }));
        list.push(json!({"name": i.name, "label": label, "menu": fp_mcp::tools::command_menu_path(i), "implemented": i.implemented, "params": i.params_schema(), "requires": i.requires, "description": fp_mcp::tools::command_description(i)}));
    }
    if json_out {
        Ok(json!({"count": list.len(), "commands": list}))
    } else {
        Ok(json!({"text": format!("{}\n\n{}", lines.join("\n"), tr!("cli-msg-commands-count", n = lines.len()))}))
    }
}

fn cmd_convert(input: &Path, output: &Path) -> Res<Value> {
    let mut s = new_session();
    open(&mut s, input)?;
    let mut v = write_output(&mut s, output)?;
    v["ok"] = json!(true);
    v["text"] = json!(format!("{} → {}", input.display(), output.display()));
    Ok(v)
}

fn cmd_script(script: Option<PathBuf>, file: Option<PathBuf>, code: Option<String>, out: Option<PathBuf>, arg: Option<String>) -> Res<Value> {
    let mut s = new_session();
    if let Some(f) = &file {
        open(&mut s, f)?;
    }
    let mut opts = fp_script::ScriptOptions::default();
    if let Some(a) = arg {
        opts.globals.push((String::from("FP_ARG"), json!(a)));
    }
    let outcome = match (&code, &script) {
        (Some(c), _) => fp_script::run_source(&mut s, c, &opts),
        (None, Some(p)) => fp_script::run_file(&mut s, p, &opts),
        (None, None) => return Err(Fail(t("cli-err-script-or-code"))),
    };
    let saved = match &out {
        Some(o) if s.active().is_some() => Some(write_output(&mut s, o)?),
        Some(_) => return Err(Fail(t("cli-err-no-document-to-save"))),
        None => None,
    };
    let mut v = outcome.to_json();
    v["saved"] = json!(saved);
    let mut text = String::new();
    for a in &outcome.alerts {
        text += &format!("alert: {a}\n");
    }
    for l in &outcome.log {
        text += &format!("{l}\n");
    }
    if let Some(r) = &outcome.result {
        text += &format!("{}\n", tr!("cli-msg-script-result", result = r.to_string()));
    }
    if !outcome.not_implemented.is_empty() {
        text += &format!("{}\n", tr!("cli-msg-script-not-implemented", list = outcome.not_implemented.join(", ")));
    }
    if let Some(e) = &outcome.error {
        return Err(Fail(format!("{text}{}", tr!("cli-err-script", error = e.to_string()))));
    }
    v["text"] = json!(text.trim_end());
    Ok(v)
}

fn cmd_mcp(http: bool, port: u16, token: Option<String>, token_file: Option<PathBuf>) -> Res<Value> {
    let host: std::sync::Arc<dyn fp_mcp::SessionHost> = std::sync::Arc::new(fp_mcp::LocalHost::with_default_session());
    if !http {
        fp_mcp::run_stdio_blocking(host)?;
        return Ok(Value::Null);
    }
    let rt = tokio_runtime()?;
    rt.block_on(async move {
        let opts = fp_mcp::HttpOptions { port, token, token_file, write_token_file: true };
        let srv = fp_mcp::start_http(host, opts).await.map_err(|e| Fail(e.to_string()))?;
        eprintln!("{}", tr!("cli-msg-mcp-http-running", url = srv.url()));
        eprintln!("{}", tr!("cli-msg-mcp-token", token = srv.token()));
        eprintln!("{}", tr!("cli-msg-mcp-header", token = srv.token()));
        eprintln!("{}", tr!("cli-msg-mcp-token-file", path = srv.token_file().display().to_string()));
        eprintln!("{}", t("cli-msg-mcp-stop"));
        srv.wait().await;
        Ok::<(), Fail>(())
    })?;
    Ok(Value::Null)
}

/// stdio-MCP, weitergeleitet an die laufende Oberfläche (Adresse/Token aus der Token-Datei).
fn cmd_mcp_bridge(token_file: Option<PathBuf>) -> Res<Value> {
    let file = token_file.unwrap_or_else(fp_mcp::bridge_token_file);
    let target = fp_mcp::proxy::BridgeTarget::from_token_file(&file)?;
    eprintln!("{}", tr!("cli-msg-mcp-bridge", address = format!("127.0.0.1:{}{}", target.port, target.path)));
    fp_mcp::proxy::run_stdio_proxy(target)?;
    Ok(Value::Null)
}

/// Sitzung mit eingesteckter JSF-Laufzeit (commands.runScript/commands.run).
fn new_session() -> Session {
    let mut s = Session::default();
    fp_script::install_host(&mut s);
    s
}

fn tokio_runtime() -> Res<tokio::runtime::Runtime> {
    Ok(tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// UE-3: jeder Befehl, jede Option und jeder Wertname hat Texte in Deutsch und Englisch.
    #[test]
    fn hilfe_vollstaendig_in_de_und_en() {
        fn walk(c: &clap::Command, key: &str, out: &mut Vec<String>) {
            out.push(key.to_string());
            for a in c.get_arguments() {
                let id = a.get_id().to_string().replace('_', "-");
                if id != "json" && id != "lang" {
                    out.push(format!("{key}-arg-{id}"));
                }
            }
            let base = if key == "cli-root" { "cli".to_string() } else { key.to_string() };
            for s in c.get_subcommands() {
                walk(s, &format!("{base}-{}", s.get_name()), out);
            }
        }
        let mut keys = vec!["cli-root-arg-json".to_string(), "cli-root-arg-lang".to_string()];
        walk(&Cli::command(), "cli-root", &mut keys);
        let mut missing = Vec::new();
        for lang in ["de", "en"] {
            for k in &keys {
                if !fp_i18n::with_language(lang, || fp_i18n::keys(lang).contains(k)) {
                    missing.push(format!("{lang}: {k}"));
                }
            }
        }
        assert!(missing.is_empty(), "fehlende Hilfetexte:\n{}", missing.join("\n"));
        // lokalisierter Befehl: englische Hilfe ohne deutsche Reste
        let help = fp_i18n::with_language("en", || {
            let mut c = localized_command();
            let mut s = c.render_help().to_string();
            for sub in ["run", "batch", "render", "info", "commands", "convert", "script", "mcp", "language"] {
                if let Some(sc) = c.find_subcommand_mut(sub) {
                    s += &sc.render_help().to_string();
                }
            }
            s
        });
        assert!(help.contains("Usage:") && help.contains("Options") && help.contains("Examples:"), "{help}");
        assert!(!help.chars().any(|c| "äöüÄÖÜß„".contains(c)), "{help}");
        let de = fp_i18n::with_language("de", || localized_command().render_help().to_string());
        assert!(de.contains("Aufruf:") && de.contains("Beispiele:") && de.contains("Ausgabe als JSON (maschinenlesbar)"), "{de}");
    }
    /// UE-3: jede Parameterbeschreibung und jeder Hinweis aller Befehle (Engine + JSF-Laufzeit) hat
    /// einen Schlüssel in command_texts.ftl (de wortgleich, en ohne deutsche Reste).
    #[test]
    fn parameterbeschreibungen_in_de_und_en() {
        let s = new_session();
        let german = |t: &str| t.chars().any(|c| "äöüÄÖÜß„".contains(c));
        let mut bad = Vec::new();
        for c in s.registry().iter() {
            let i = c.info();
            let texts = i.params.iter().map(|p| p.description.clone()).chain(i.note.clone());
            for de in texts.filter(|t| !t.is_empty()) {
                let key = fp_engine::command::source_text_key(&de);
                let (has_de, has_en) = (fp_i18n::keys("de").contains(&key), fp_i18n::keys("en").contains(&key));
                if !has_de || !has_en {
                    bad.push(format!("{}: {key} ({de})", i.name));
                    continue;
                }
                assert_eq!(fp_i18n::with_language("de", || fp_engine::command::source_text(&de)), de);
                let en = fp_i18n::with_language("en", || fp_engine::command::source_text(&de));
                if german(&en) {
                    bad.push(format!("{}: en „{en}“", i.name));
                }
            }
        }
        assert!(bad.is_empty(), "{} Texte ohne Übersetzung:\n{}", bad.len(), bad.join("\n"));
        let schema = fp_i18n::with_language("en", || s.registry().get("file.new").map(|c| c.info().params_schema()).unwrap_or_default());
        assert_eq!(schema["properties"]["height"]["description"], "Height in pixels", "{schema}");
    }
}
