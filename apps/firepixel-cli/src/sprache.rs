//! `firepixel-cli sprache …`: Sprachdateien prüfen, Vorlage für eine neue Sprache anlegen,
//! verfügbare Sprachen auflisten (Phase 6, I18N-KERN). Die Prüfungen stehen in `fp_i18n::check`.

use clap::Subcommand;
use fp_i18n::check::{self, Kind, Report};
use fp_i18n::{t, tr};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// Unterbefehle von `sprache`.
#[derive(Subcommand, Debug)]
pub enum SpracheCmd {
    // Hilfetexte: kommandozeile.ftl (`cli-sprache-…`), englische Namen als Alias
    #[command(alias = "check")]
    Pruefen {
        pfad: Option<PathBuf>,
        #[arg(long, alias = "builtin")]
        eingebaut: bool,
        #[arg(long, alias = "short")]
        kurz: bool,
        #[arg(long, alias = "strict")]
        streng: bool,
    },
    #[command(alias = "template")]
    Vorlage {
        code: String,
        zielordner: PathBuf,
        #[arg(long, alias = "overwrite")]
        ueberschreiben: bool,
        #[arg(long, alias = "from", default_value = "en")]
        von: String,
    },
    #[command(alias = "list")]
    Liste,
}

/// Führt einen `sprache`-Unterbefehl aus. `Err` = Fehlermeldung (Exit-Code 1).
pub fn run(cmd: SpracheCmd, json_out: bool) -> Result<Value, String> {
    match cmd {
        SpracheCmd::Pruefen { pfad, eingebaut, kurz, streng } => pruefen(pfad.as_deref(), eingebaut, kurz, streng, json_out),
        SpracheCmd::Vorlage { code, zielordner, ueberschreiben, von } => vorlage(&code, &zielordner, ueberschreiben, &von),
        SpracheCmd::Liste => liste(json_out),
    }
}

fn kind_title(k: Kind) -> String {
    // `&&` in den Texten ist ein wörtliches &
    fp_i18n::strip_access(&t(&format!("cli-lang-kind-{}", k.code()))).0
}

fn report_text(r: &Report, kurz: bool) -> String {
    let mut out = String::new();
    for l in &r.langs {
        let name = l.name.clone().unwrap_or_else(|| t("cli-lang-no-name"));
        out.push_str(&tr!("cli-lang-summary", code = l.code.as_str(), name = name, keys = l.keys, percent = (l.percent * 10.0).round() / 10.0, missing = l.missing, files = l.files.len()));
        out.push('\n');
    }
    let mut kinds: Vec<Kind> = r.problems.iter().map(|p| p.kind).collect();
    kinds.sort();
    kinds.dedup();
    for k in kinds {
        let list: Vec<&check::Problem> = r.problems.iter().filter(|p| p.kind == k).collect();
        out.push_str(&format!("\n{} ({}):\n", kind_title(k), list.len()));
        if kurz && matches!(k, Kind::Missing | Kind::Orphan) {
            continue;
        }
        for p in list {
            out.push_str(&format!("  {p}\n"));
        }
    }
    out.push_str(&format!("\n{}\n", tr!("cli-lang-result", errors = r.errors(), warnings = r.warnings())));
    out
}

fn report_json(r: &Report) -> Value {
    json!({
        "ok": r.errors() == 0,
        "errors": r.errors(),
        "warnings": r.warnings(),
        "languages": r.langs.iter().map(|l| json!({"code": l.code, "name": l.name, "keys": l.keys, "missing": l.missing, "percent": (l.percent * 10.0).round() / 10.0, "files": l.files})).collect::<Vec<_>>(),
        "counts": r.counts().into_iter().map(|(k, n)| (k.code().to_string(), json!(n))).collect::<serde_json::Map<_, _>>(),
        "problems": r.problems.iter().map(|p| json!({"kind": p.kind.code(), "error": p.kind.is_error(), "lang": p.lang, "file": p.file, "line": p.line, "key": p.key, "detail": p.detail})).collect::<Vec<_>>(),
    })
}

fn pruefen(pfad: Option<&Path>, eingebaut: bool, kurz: bool, streng: bool, json_out: bool) -> Result<Value, String> {
    let r = match (pfad, eingebaut) {
        (_, true) => check::check_builtin(),
        (Some(p), false) => check::check_path(p)?,
        (None, false) => return Err(t("cli-lang-path-missing")),
    };
    let failed = r.errors() > 0 || (streng && r.warnings() > 0);
    if json_out {
        let v = report_json(&r);
        if failed {
            // vollständiger Bericht als einziges JSON-Objekt auf stdout, dann Exit-Code 1
            println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
            std::process::exit(1);
        }
        return Ok(v);
    }
    let text = report_text(&r, kurz);
    if failed {
        print!("{text}");
        return Err(tr!("cli-lang-errors-warnings", errors = r.errors(), warnings = r.warnings()));
    }
    Ok(json!({ "text": text.trim_end() }))
}

fn vorlage(code: &str, ziel: &Path, ueberschreiben: bool, von: &str) -> Result<Value, String> {
    if code.parse::<unic_langid_check::Code>().is_err() {
        return Err(tr!("cli-lang-bad-code", code = code));
    }
    let files: Vec<&(&str, &str, &str)> = fp_i18n::embedded_files().iter().filter(|(c, _, _)| *c == von).collect();
    if files.is_empty() {
        return Err(tr!("cli-lang-no-builtin", code = von));
    }
    let dir = ziel.join(code);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut written = Vec::new();
    let mut skipped = Vec::new();
    for (_, name, text) in files {
        let p = dir.join(name);
        if p.exists() && !ueberschreiben {
            skipped.push(p.display().to_string());
            continue;
        }
        let mut body = String::new();
        body.push_str(&tr!("cli-lang-template-header", code = code, from = von, file = *name));
        body.push_str("\n\n");
        for line in text.lines() {
            if line.starts_with("language-name") {
                body.push_str(&format!("language-name = {code}\n"));
            } else {
                body.push_str(line);
                body.push('\n');
            }
        }
        std::fs::write(&p, body).map_err(|e| format!("{}: {e}", p.display()))?;
        written.push(p.display().to_string());
    }
    let skipped_text = if skipped.is_empty() { String::new() } else { tr!("cli-lang-template-skipped", n = skipped.len()) };
    let text = tr!("cli-lang-template-done", code = code, dir = dir.display().to_string(), n = written.len(), skipped = skipped_text);
    Ok(json!({ "ok": true, "dir": dir.display().to_string(), "written": written, "skipped": skipped, "text": text }))
}

fn liste(json_out: bool) -> Result<Value, String> {
    let dirs: Vec<PathBuf> = [fp_i18n::exe_language_dir(), fp_i18n::user_language_dir()].into_iter().flatten().collect();
    fp_i18n::load_default_dirs();
    let reference = check::LangData::builtin(fp_i18n::SOURCE_LANG);
    let total = reference.keys.len();
    let mut rows = Vec::new();
    for l in fp_i18n::languages() {
        let have = fp_i18n::keys(&l.code);
        let missing = reference.keys.keys().filter(|k| !have.contains(*k)).count();
        let pct = if total == 0 { 100.0 } else { (total - missing) as f64 * 100.0 / total as f64 };
        rows.push(json!({"code": l.code, "name": l.name, "builtin": l.builtin, "percent": (pct * 10.0).round() / 10.0, "missing": missing}));
    }
    if json_out {
        return Ok(json!({ "languages": rows, "folders": dirs.iter().map(|d| d.display().to_string()).collect::<Vec<_>>(), "pseudo": fp_i18n::PSEUDO_LANG }));
    }
    let (h_code, h_name, h_origin, h_pct) = (t("cli-lang-col-code"), t("cli-lang-col-name"), t("cli-lang-col-origin"), t("cli-lang-col-translated"));
    let mut text = format!("{h_code:<7} {h_name:<21} {h_origin:<12} {h_pct}\n");
    for r in &rows {
        text.push_str(&format!(
            "{:<7} {:<21} {:<12} {:>5} %\n",
            r["code"].as_str().unwrap_or(""),
            r["name"].as_str().unwrap_or(""),
            if r["builtin"].as_bool() == Some(true) { t("cli-lang-builtin") } else { t("cli-lang-folder") },
            fp_i18n::fmt_decimal(r["percent"].as_f64().unwrap_or(0.0), 1)
        ));
    }
    text.push_str(&format!("{:<7} {:<21} {:<12}\n", fp_i18n::PSEUDO_LANG, t("cli-lang-pseudo"), t("cli-lang-builtin")));
    text.push_str(&format!("\n{}\n", t("cli-lang-folders")));
    for d in &dirs {
        let missing = if d.is_dir() { String::new() } else { format!("  {}", t("cli-lang-folder-missing")) };
        text.push_str(&format!("  {}{missing}\n", d.display()));
    }
    Ok(json!({ "text": text.trim_end() }))
}

/// Gültigkeit eines Sprachcodes ohne eigene Abhängigkeit: Teile aus Buchstaben/Ziffern, getrennt
/// durch `-`, erster Teil 2–3 Buchstaben (BCP 47 vereinfacht).
mod unic_langid_check {
    pub struct Code;
    impl std::str::FromStr for Code {
        type Err = ();
        fn from_str(s: &str) -> Result<Code, ()> {
            let mut parts = s.split('-');
            let first = parts.next().ok_or(())?;
            if !(2..=3).contains(&first.len()) || !first.chars().all(|c| c.is_ascii_alphabetic()) {
                return Err(());
            }
            if parts.all(|p| (1..=8).contains(&p.len()) && p.chars().all(|c| c.is_ascii_alphanumeric())) { Ok(Code) } else { Err(()) }
        }
    }
}
