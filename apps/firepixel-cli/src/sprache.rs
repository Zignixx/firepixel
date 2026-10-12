//! `firepixel-cli sprache …`: Sprachdateien prüfen, Vorlage für eine neue Sprache anlegen,
//! verfügbare Sprachen auflisten (Phase 6, I18N-KERN). Die Prüfungen stehen in `fp_i18n::check`.

use clap::Subcommand;
use fp_i18n::check::{self, Kind, Report};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// Unterbefehle von `sprache`.
#[derive(Subcommand, Debug)]
pub enum SpracheCmd {
    /// Sprachdateien prüfen: fehlende/verwaiste Schlüssel, Platzhalter, Syntax, Zugriffstasten
    #[command(after_help = "Beispiele:
  firepixel-cli sprache pruefen %APPDATA%\\Firepixel\\Sprachen          (alle Sprachen im Ordner)
  firepixel-cli sprache pruefen Sprachen\\fr                             (eine Sprache, Ordner mit *.ftl)
  firepixel-cli sprache pruefen fr.ftl                                  (eine Sprache in einer Datei)
  firepixel-cli sprache pruefen Sprachen\\fr\\menue.ftl                   (nur dieser Bereich)
  firepixel-cli sprache pruefen --eingebaut                             (die eingebauten Sprachen)

Maßstab ist das eingebaute Deutsch. Fehler (Exit-Code 1): Syntax, doppelte Schlüssel, Platzhalter,
die das Programm nicht kennt. Warnungen: fehlende/verwaiste Schlüssel, fehlende Platzhalter oder
Zugriffstasten (&), doppelte Zugriffstasten in einem Menü bzw. Dialog.")]
    Pruefen {
        /// Ordner oder .ftl-Datei
        pfad: Option<PathBuf>,
        /// Die eingebauten Sprachen prüfen
        #[arg(long)]
        eingebaut: bool,
        /// Fehlende Schlüssel nicht einzeln auflisten (nur zählen)
        #[arg(long)]
        kurz: bool,
        /// Warnungen als Fehler werten (Exit-Code 1)
        #[arg(long)]
        streng: bool,
    },
    /// Vorlage für eine neue Sprache anlegen (Kopie der englischen Dateien)
    #[command(after_help = "Beispiel:
  firepixel-cli sprache vorlage fr %APPDATA%\\Firepixel\\Sprachen
legt %APPDATA%\\Firepixel\\Sprachen\\fr\\*.ftl an (englische Texte als Ausgangspunkt).
Danach rechts vom = übersetzen, language-name setzen, prüfen und Firepixel neu starten
(oder Ansicht ▸ Sprache ▸ Sprachordner öffnen…).")]
    Vorlage {
        /// Sprachcode (z. B. fr, es, pt-BR, ja)
        code: String,
        /// Zielordner (darin entsteht der Unterordner <code>)
        zielordner: PathBuf,
        /// Vorhandene Dateien überschreiben
        #[arg(long)]
        ueberschreiben: bool,
        /// Ausgangssprache der Vorlage (Vorgabe: en)
        #[arg(long, default_value = "en")]
        von: String,
    },
    /// Verfügbare Sprachen (eingebaut und aus den Sprachordnern) mit Vollständigkeit
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

fn kind_title(k: Kind) -> &'static str {
    match k {
        Kind::Syntax => "Syntaxfehler",
        Kind::Duplicate => "Doppelte Schlüssel",
        Kind::PlaceholderUnknown => "Unbekannte Platzhalter (das Programm übergibt sie nicht)",
        Kind::PlaceholderMissing => "Fehlende Platzhalter (Wert wird nicht angezeigt)",
        Kind::AccessConflict => "Zugriffstasten-Konflikte (gleiche &-Taste im selben Menü/Dialog)",
        Kind::AccessMissing => "Zugriffstaste (&) fehlt",
        Kind::Missing => "Fehlende Schlüssel (Rückfall auf Englisch)",
        Kind::Orphan => "Verwaiste Schlüssel (gibt es im Programm nicht)",
    }
}

fn report_text(r: &Report, kurz: bool) -> String {
    let mut out = String::new();
    for l in &r.langs {
        out.push_str(&format!(
            "Sprache {} ({}): {} Schlüssel, {:.1} % übersetzt, {} fehlen – {} Datei(en)\n",
            l.code,
            l.name.as_deref().unwrap_or("ohne language-name"),
            l.keys,
            l.percent,
            l.missing,
            l.files.len()
        ));
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
    out.push_str(&format!("\nErgebnis: {} Fehler, {} Warnungen\n", r.errors(), r.warnings()));
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
        (None, false) => return Err("Pfad angeben (Ordner oder .ftl-Datei) oder --eingebaut".into()),
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
        return Err(format!("{} Fehler, {} Warnungen", r.errors(), r.warnings()));
    }
    Ok(json!({ "text": text.trim_end() }))
}

fn vorlage(code: &str, ziel: &Path, ueberschreiben: bool, von: &str) -> Result<Value, String> {
    if code.parse::<unic_langid_check::Code>().is_err() {
        return Err(format!("„{code}“ ist kein gültiger Sprachcode (Beispiele: fr, es, pt-BR, zh-TW)"));
    }
    let files: Vec<&(&str, &str, &str)> = fp_i18n::embedded_files().iter().filter(|(c, _, _)| *c == von).collect();
    if files.is_empty() {
        return Err(format!("keine eingebaute Sprache „{von}“"));
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
        body.push_str(&format!(
            "# Firepixel – Sprache „{code}“ (Vorlage aus „{von}“, Datei {name}).\n# Rechts vom = übersetzen; Schlüssel links, Platzhalter {{ $name }} und Auswahl-Varianten [one]/[other] beibehalten.\n# & markiert die Zugriffstaste (Alt+Buchstabe), && ist ein wörtliches &. Prüfen: firepixel-cli sprache pruefen <ordner>\n\n"
        ));
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
    let text = format!(
        "Vorlage für „{code}“ in {}: {} Datei(en) geschrieben{}.\nNächste Schritte: language-name in allgemein.ftl setzen, übersetzen, dann\n  firepixel-cli sprache pruefen \"{}\"",
        dir.display(),
        written.len(),
        if skipped.is_empty() { String::new() } else { format!(", {} vorhandene übersprungen (--ueberschreiben)", skipped.len()) },
        dir.display()
    );
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
    let mut text = String::from("Code    Name                  Herkunft     übersetzt\n");
    for r in &rows {
        text.push_str(&format!(
            "{:<7} {:<21} {:<12} {:>5.1} %\n",
            r["code"].as_str().unwrap_or(""),
            r["name"].as_str().unwrap_or(""),
            if r["builtin"].as_bool() == Some(true) { "eingebaut" } else { "Sprachordner" },
            r["percent"].as_f64().unwrap_or(0.0)
        ));
    }
    text.push_str(&format!("{:<7} {:<21} {:<12}\n", fp_i18n::PSEUDO_LANG, "Testsprache", "eingebaut"));
    text.push_str("\nSprachordner:\n");
    for d in &dirs {
        text.push_str(&format!("  {}{}\n", d.display(), if d.is_dir() { "" } else { "  (nicht vorhanden)" }));
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
