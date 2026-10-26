//! Korpus-Test: die eigenen JSF-Testskripte aus `re\scripts\fwsteuerung\tests` (für das echte
//! Original geschrieben) in Firepixel ausführen – mit demselben Prolog/Epilog wie
//! `jsf-ausfuehren.ps1` (`FP_OUT`, `fp_log`, `fp_try`).
//!
//! „Fehlerfrei“ = keine nicht abgefangene Ausnahme und keine Protokollzeile „FEHLER …“/„ABBRUCH“.
//! Aufruf: `cargo test -p fp-script --test fwsteuerung -- --ignored --nocapture`
//! (Ausgaben in `%TEMP%\fp_script_fwsteuerung`, Eingabedateien werden aus `re\_testdateien` gelesen).

use fp_script::{ScriptOptions, run_source};
use std::path::{Path, PathBuf};

fn rea() -> PathBuf {
    // FP_REA: Projektordner (enthält re\ und korpus\), z. B. bei einem Schatten-Workspace
    std::env::var_os("FP_REA")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("..")
                .join("..")
        })
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut v: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
    v.sort();
    for p in v {
        if p.is_dir() {
            collect(&p, out);
        } else if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("jsf")) {
            out.push(p);
        }
    }
}

const PROLOG: &str = r#"
var FP_LOG = [];
function fp_log(t) { FP_LOG.push(String(t)); }
function fp_try(n, f) { try { var r = f(); fp_log("ok " + n + (r === undefined ? "" : " = " + r)); return r; } catch (e) { fp_log("FEHLER " + n + ": " + e); return null; } }
"#;

/// Ergebnis eines Skripts.
pub struct Run {
    pub name: String,
    pub stem: String,
    pub ok: bool,
    pub clean: bool,
    pub errors: Vec<String>,
    pub not_impl: Vec<String>,
    /// fp_try-Name → ok?
    pub status: Vec<(String, bool)>,
}

/// fp_try-Zeilen eines Protokolls: (Name, ok).
fn parse_log(text: &str) -> Vec<(String, bool)> {
    let mut v = Vec::new();
    for l in text.lines() {
        if let Some(rest) = l.strip_prefix("ok ") {
            let name = rest.split(" = ").next().unwrap_or(rest).trim().to_string();
            v.push((name, true));
        } else if let Some(rest) = l.strip_prefix("FEHLER ") {
            let name = rest.split(": ").next().unwrap_or(rest).trim().to_string();
            v.push((name, false));
        }
    }
    v
}

/// Protokoll des Laufs im echten Original (`re\_testdateien\…\<stem>.log`).
fn original_log(stem: &str) -> Option<Vec<(String, bool)>> {
    fn find(dir: &Path, name: &str) -> Option<PathBuf> {
        let p = dir.join(name);
        if p.is_file() {
            return Some(p);
        }
        let rd = std::fs::read_dir(dir).ok()?;
        for e in rd.flatten() {
            if e.path().is_dir()
                && let Some(f) = find(&e.path(), name)
            {
                return Some(f);
            }
        }
        None
    }
    let p = find(
        &rea().join("re").join("_testdateien"),
        &format!("{stem}.log"),
    )?;
    let text = std::fs::read(&p).ok().map(|b| fp_script::decode_text(&b))?;
    let v = parse_log(&text);
    if v.is_empty() { None } else { Some(v) }
}

/// Skripte, die im Original als Folge in **einer** Sitzung laufen (`…_aufbau` legt das Dokument
/// an, die folgenden arbeiten darauf): gleiche Sitzung in der Reihenfolge aufbau → Rest → speichern
/// → schliessen.
fn sequences(files: &[PathBuf]) -> Vec<Vec<PathBuf>> {
    let stem = |p: &Path| {
        p.file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default()
    };
    // Gruppe = gleicher Ordner und gleiches erstes Namenssegment (T4_…, T7g_…, T16_…)
    let prefix = |p: &Path| {
        let s = stem(p);
        match s.find('_') {
            Some(i) => format!(
                "{}|{}",
                p.parent()
                    .map(|d| d.display().to_string())
                    .unwrap_or_default(),
                &s[..i]
            ),
            None => String::new(),
        }
    };
    let stem = |p: &Path| {
        stem(p)
            .trim_end_matches(|c: char| c.is_ascii_digit())
            .to_string()
    };
    let mut groups: Vec<Vec<PathBuf>> = Vec::new();
    let mut done: Vec<PathBuf> = Vec::new();
    for f in files {
        if done.contains(f) {
            continue;
        }
        if stem(f).ends_with("_aufbau") {
            let pre = prefix(f);
            let mut g: Vec<PathBuf> = files.iter().filter(|x| prefix(x) == pre).cloned().collect();
            let rank = |p: &PathBuf| {
                let s = stem(p);
                if s.ends_with("_aufbau") {
                    0
                } else if s.ends_with("_speichern") {
                    2
                } else if s.ends_with("_schliessen") {
                    3
                } else {
                    1
                }
            };
            g.sort_by_key(|p| (rank(p), p.clone()));
            done.extend(g.iter().cloned());
            groups.push(g);
        }
    }
    for f in files {
        if !done.contains(f) {
            groups.push(vec![f.clone()]);
        }
    }
    groups
}

fn run_one(path: &Path, out_dir: &Path, session: &mut fp_engine::Session) -> Run {
    let name = path
        .strip_prefix(
            rea()
                .join("re")
                .join("scripts")
                .join("fwsteuerung")
                .join("tests"),
        )
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let src = fp_script::decode_text(&std::fs::read(path).unwrap_or_default());
    let fp_out = fp_script::urls::path_to_url(out_dir) + "/";
    let wrapped = format!("try {{\n{src}\n}} catch (e) {{ fp_log(\"ABBRUCH: \" + e); }}");
    let opts = ScriptOptions {
        script_path: Some(path.to_path_buf()),
        prelude: Some(format!("var FP_OUT = \"{fp_out}\";\n{PROLOG}")),
        epilogue: Some("write(FP_LOG.join(\"\\n\"));".into()),
        read_fallback: vec![
            (out_dir.to_path_buf(), rea().join("re").join("_testdateien")),
            // PSD-Eingaben (Skripte bilden den Pfad aus FP_OUT: „_testdateien/“ → „import/_scratch/testpsd/“)
            (
                out_dir
                    .parent()
                    .unwrap_or(out_dir)
                    .join("import")
                    .join("_scratch")
                    .join("testpsd"),
                rea()
                    .join("re")
                    .join("import")
                    .join("_scratch")
                    .join("testpsd"),
            ),
        ],
        loop_limit: 50_000_000,
        ..ScriptOptions::default()
    };
    let o = run_source(session, &wrapped, &opts);
    let log = o.log.join("\n");
    let errors: Vec<String> = log
        .lines()
        .filter(|l| l.starts_with("FEHLER") || l.starts_with("ABBRUCH"))
        .map(String::from)
        .collect();
    let mut errs = errors.clone();
    if let Some(e) = &o.error {
        errs.push(format!("nicht abgefangen: {e}"));
    }
    if !errors.is_empty() {
        errs.extend(o.diagnostics.iter().take(3).map(|d| format!("diag: {d}")));
    }
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    Run {
        name,
        stem,
        ok: o.ok,
        clean: o.ok && errors.is_empty(),
        errors: errs,
        not_impl: o.not_implemented,
        status: parse_log(&log),
    }
}

#[test]
#[ignore]
fn eigene_testskripte() {
    // Die Skripte nutzen API-Namen des Originals; die Kompatibilitäts-Aliase brauchen dessen
    // Ressourcenordner (hier die Installationskopie im Korpus, falls nichts anderes gesetzt ist).
    if std::env::var_os(fp_color::install::ENV_DIR).is_none() {
        let corpus = std::env::var_os("FP_CORPUS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| rea().join("korpus"));
        fp_color::install::set_configured_dir(Some(corpus));
    }
    let dir = rea()
        .join("re")
        .join("scripts")
        .join("fwsteuerung")
        .join("tests");
    let mut files = Vec::new();
    collect(&dir, &mut files);
    assert!(
        !files.is_empty(),
        "keine Testskripte unter {}",
        dir.display()
    );
    // wie im Original endet der Ausgabeordner auf „_testdateien“ (Skripte leiten Nachbarordner daraus ab)
    let out_dir = fp_i18n::profile::unique_temp_dir("script-control").join("_testdateien");
    let _ = std::fs::remove_dir_all(&out_dir);
    let _ = std::fs::create_dir_all(&out_dir);
    let mut runs = Vec::new();
    for group in sequences(&files) {
        let mut session = fp_engine::Session::default();
        if group.len() > 1 {
            // Folgen liefen im Original neben einem fremden, offenen Dokument (Prüfung „nicht mein Dokument“)
            let _ = session.execute("file.new", serde_json::json!({"width": 10, "height": 10}));
        }
        for f in &group {
            let r = run_one(f, &out_dir, &mut session);
            println!(
                "{} {}",
                if r.clean {
                    "OK    "
                } else if r.ok {
                    "FEHLER"
                } else {
                    "ABBR. "
                },
                r.name
            );
            for e in r.errors.iter().take(6) {
                println!("        {}", e.chars().take(220).collect::<String>());
            }
            runs.push(r);
        }
    }
    let clean = runs.iter().filter(|r| r.clean).count();
    let ok = runs.iter().filter(|r| r.ok).count();
    let mut ni: std::collections::BTreeMap<String, usize> = Default::default();
    for r in &runs {
        for n in &r.not_impl {
            *ni.entry(n.clone()).or_default() += 1;
        }
    }
    let mut ni: Vec<(String, usize)> = ni.into_iter().collect();
    ni.sort_by_key(|x| std::cmp::Reverse(x.1));
    println!(
        "\nNicht implementiert (Skripte): {}",
        ni.iter()
            .take(40)
            .map(|(n, c)| format!("{n} ({c})"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    // Vergleich mit den Protokollen der Läufe im echten Original (gleicher Status je fp_try)
    let (mut same, mut total, mut scripts_cmp, mut scripts_same) = (0usize, 0usize, 0usize, 0usize);
    for r in &runs {
        let Some(fw) = original_log(&r.stem) else {
            continue;
        };
        let mut all = true;
        let mut any = false;
        for (n, ok_fw) in &fw {
            if let Some((_, ok_fp)) = r.status.iter().find(|(m, _)| m == n) {
                any = true;
                total += 1;
                if ok_fw == ok_fp {
                    same += 1;
                } else {
                    all = false;
                    println!(
                        "ABWEICHUNG {}: {n} Original={} Firepixel={}",
                        r.name,
                        if *ok_fw { "ok" } else { "FEHLER" },
                        if *ok_fp { "ok" } else { "FEHLER" }
                    );
                }
            }
        }
        if any {
            scripts_cmp += 1;
            if all {
                scripts_same += 1;
            }
        }
    }
    println!(
        "\nVergleich mit den Protokollen des Originals: {same}/{total} fp_try-Ergebnisse gleich, {scripts_same}/{scripts_cmp} Skripte mit gleichem Statusmuster."
    );
    println!(
        "\n{clean} von {} Skripten laufen fehlerfrei; {ok} ohne Abbruch.",
        runs.len()
    );
    let report = runs
        .iter()
        .map(|r| {
            format!(
                "{}\t{}\t{}",
                if r.clean { "ok" } else { "fehler" },
                r.name,
                r.errors.join(" | ")
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let _ = std::fs::write(out_dir.join("bericht.tsv"), report);
}
