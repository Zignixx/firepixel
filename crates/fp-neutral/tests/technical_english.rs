//! Guard: technical names are always English (docs\AGENTEN_REGELN.md §1c).
//!
//! Scans the runtime sources (`crates\*\src`, `apps\*\src`, without inline test modules) for
//! German words or umlauts in
//! - path constructions: `.join("…")`, `.join(format!("…"))`, `with_file_name`, `with_extension`,
//!   `temp_dir("…")`, `PathBuf::from("…")`, `Path::new("…")`,
//! - serde names: `rename = "…"`, `rename_all`, field and variant names of `Serialize`/`Deserialize`
//!   types (aliases are allowed – they exist exactly to keep reading old German keys),
//! - CLI names: `#[arg(long = "…")]`, `#[command(name = "…")]`, fields and variants of clap types,
//! - environment variables: `var("…")`, `var_os("…")`, `const ENV_…: &str = "…"`.
//!
//! Deliberate exceptions are listed in [`ALLOWED`] with a reason. Old German names that are still
//! read for migration live in constants named `LEGACY_*` and are skipped.

use std::path::{Path, PathBuf};

/// (file path suffix, literal or identifier, reason)
const ALLOWED: &[(&str, &str, &str)] = &[
    ("fp-engine/src/commands/misc.rs", "ANLEITUNG", "Handbuch-Dateien im Repo (docs\\ANLEITUNG*.md); Umbenennung zusammen mit den Dokumenten (TE-2)"),
    ("fp-script/src/builtin_commands.rs", ".jsf", "eingebaute Befehlsskripte: Datei- und Ordnernamen erscheinen im Menü „Befehle“ (wie im Original) und folgen der UI-Sprache"),
];

/// German words that must not appear in technical names (lowercase, compared per word part).
const GERMAN: &[&str] = &[
    "sprache", "sprachen", "modell", "modelle", "bibliothek", "allgemein", "allgemeine", "befehl", "befehle", "vorlage",
    "vorlagen", "stil", "stile", "textur", "texturen", "muster", "sicherung", "sicherungen", "protokoll", "protokolle",
    "absturz", "einstellung", "einstellungen", "voreinstellung", "voreinstellungen", "vorschau", "druck", "drucken", "hilfe",
    "kurzhilfe", "anleitung", "zwischenablage", "datei", "dateien", "ordner", "pfad", "bild", "bilder", "ebene", "ebenen",
    "seite", "seiten", "dok", "dokument", "kurz", "streng", "eingebaut", "pruefen", "liste", "ueberschreiben", "ziel",
    "zielordner", "dunkel", "hell", "nativ", "abbrechen", "fehlt", "kopie", "werkzeug", "werkzeuge", "farbe", "farben",
    "schrift", "verlauf", "fenster", "menue", "bedienfeld", "bedienfelder", "arbeitsbereich", "kuerzel", "tastenkuerzel",
    "foto", "schwarzweiss", "nachzeichnen", "oberflaeche", "meldung", "meldungen", "dialoge", "befehlstexte", "willkommen",
    "inspektor", "kommandozeile", "skript", "skripte", "dateidialog", "auswahl", "groesse", "breite", "hoehe", "speichern",
    "oeffnen", "schliessen", "neu", "aus", "ein", "nur", "alle", "keine", "oben", "unten", "rechts",
    "mitte", "nein", "vorgabe", "vorgaben", "sichern", "wiederherstellen", "texte", "schluessel",
];

/// Extensions of file names the program writes or reads.
const FILE_EXTS: &[&str] = &["json", "ron", "txt", "md", "png", "htm", "html", "eml", "jsf", "ftl", "safetensors", "log", "css", "svg", "pdf", "gif", "jpg", "ico", "bin", "tmp"];

#[derive(Debug)]
struct Hit {
    file: String,
    line: usize,
    what: String,
    text: String,
}

fn root() -> PathBuf {
    std::fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")).expect("project root")
}

fn rust_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            rust_sources(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// Runtime sources: `src` of every crate and app, without `tests.rs` files.
fn files(root: &Path) -> Vec<PathBuf> {
    let mut v = Vec::new();
    for group in ["crates", "apps"] {
        let Ok(rd) = std::fs::read_dir(root.join(group)) else { continue };
        for e in rd.flatten() {
            if e.file_name() == "fp-neutral" {
                continue;
            }
            rust_sources(&e.path().join("src"), &mut v);
        }
    }
    v.retain(|p| !p.file_name().is_some_and(|n| n == "tests.rs"));
    v.sort();
    v
}

/// Words of a name: split at non-alphanumerics and camelCase boundaries, lowercase.
fn words(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut prev_lower = false;
    for c in s.chars() {
        if !c.is_alphanumeric() {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            prev_lower = false;
            continue;
        }
        if c.is_uppercase() && prev_lower && !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
        prev_lower = c.is_lowercase() || c.is_numeric();
        cur.extend(c.to_lowercase());
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// German part of a technical name, if any.
fn german(name: &str) -> Option<String> {
    if let Some(c) = name.chars().find(|c| "äöüÄÖÜß".contains(*c)) {
        return Some(format!("umlaut {c}"));
    }
    // file-format placeholders like `{base}` do not count
    let cleaned: String = {
        let mut s = String::new();
        let mut depth = 0;
        for c in name.chars() {
            match c {
                '{' => depth += 1,
                '}' => depth -= 1,
                _ if depth == 0 => s.push(c),
                _ => {}
            }
        }
        s
    };
    words(&cleaned).into_iter().find(|w| GERMAN.contains(&w.as_str()))
}

/// First string literal starting at `s` (after the opening quote), without escapes handling beyond `\"`.
fn literal_after(s: &str) -> Option<&str> {
    let start = s.find('"')? + 1;
    let rest = &s[start..];
    let mut end = 0;
    let b = rest.as_bytes();
    while end < b.len() {
        if b[end] == b'\\' {
            end += 2;
            continue;
        }
        if b[end] == b'"' {
            return Some(&rest[..end]);
        }
        end += 1;
    }
    None
}

fn scan_file(root: &Path, path: &Path, hits: &mut Vec<Hit>) {
    let Ok(text) = std::fs::read_to_string(path) else { return };
    let rel = path.strip_prefix(root).unwrap_or(path).to_string_lossy().replace('\\', "/");
    let lines: Vec<&str> = text.lines().collect();
    let mut push = |line: usize, what: &str, name: &str| {
        if let Some(g) = german(name) {
            hits.push(Hit { file: rel.clone(), line: line + 1, what: format!("{what} ({g})"), text: name.to_string() });
        }
    };
    // state for serde / clap types
    let mut pending_derive: Option<&str> = None;
    let mut in_type: Option<(&str, i32)> = None;
    let mut depth = 0i32;
    for (i, raw) in lines.iter().enumerate() {
        let line = raw.trim();
        // inline test modules (at the end of the file) are not runtime code
        if line == "#[cfg(test)]" && lines.get(i + 1).is_some_and(|n| n.trim_start().starts_with("mod ")) {
            break;
        }
        if line.starts_with("//") {
            continue;
        }
        let code = match line.find(" //") {
            Some(p) if !line[..p].contains('"') || line[..p].matches('"').count() % 2 == 0 => &line[..p],
            _ => line,
        };
        if code.contains("LEGACY_") {
            continue;
        }
        for pat in [".join(\"", ".join(format!(\"", "with_file_name(\"", "with_extension(\"", "temp_dir(\"", "PathBuf::from(\"", "Path::new(\""] {
            let mut from = 0;
            while let Some(p) = code[from..].find(pat) {
                let at = from + p + pat.len() - 1;
                if let Some(lit) = literal_after(&code[at..]) {
                    push(i, "path", lit);
                }
                from = at + 1;
            }
        }
        // any literal that looks like a file name (`name.json`, `{base}_x.png` …)
        let mut rest = code;
        while let Some(p) = rest.find('"') {
            let Some(lit) = literal_after(&rest[p..]) else { break };
            let ext = lit.rsplit('.').next().unwrap_or("");
            if lit.contains('.') && !lit.contains(' ') && FILE_EXTS.contains(&ext) {
                push(i, "file name", lit);
            }
            rest = &rest[p + lit.len() + 2..];
        }
        for pat in ["var(\"", "var_os(\"", "env(\""] {
            if let Some(p) = code.find(pat)
                && let Some(lit) = literal_after(&code[p + pat.len() - 1..])
                && lit.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
            {
                push(i, "environment variable", lit);
            }
        }
        if (code.starts_with("pub const ENV_") || code.starts_with("const ENV_"))
            && let Some(lit) = literal_after(code)
        {
            push(i, "environment variable", lit);
        }
        if code.starts_with("#[serde(") || code.starts_with("#[arg(") || code.starts_with("#[command(") || code.starts_with("#[clap(") {
            for key in ["rename = ", "rename_all = ", "long = ", "name = "] {
                if let Some(p) = code.find(key)
                    && let Some(lit) = literal_after(&code[p..])
                {
                    push(i, "serde/clap name", lit);
                }
            }
        }
        if code.starts_with("#[derive(") {
            if code.contains("Serialize") || code.contains("Deserialize") {
                pending_derive = Some("serde");
            } else if code.contains("Parser") || code.contains("Subcommand") || code.contains("Args") || code.contains("ValueEnum") {
                pending_derive = Some("clap");
            }
        }
        let opens = code.matches('{').count() as i32;
        let closes = code.matches('}').count() as i32;
        if let Some(kind) = pending_derive
            && (code.contains("struct ") || code.contains("enum "))
        {
            pending_derive = None;
            if code.contains('{') {
                in_type = Some((kind, depth + 1));
            }
            let name = code.split_whitespace().skip_while(|w| *w != "struct" && *w != "enum").nth(1).unwrap_or("");
            let name: String = name.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            if kind == "clap" {
                push(i, "clap type", &name);
            }
        } else if let Some((kind, d)) = in_type
            && depth == d
            && !code.starts_with('#')
            && !code.contains("alias")
        {
            // field `name: Type` or variant `Name`, `Name {`, `Name(…)`
            let ident: String = code.trim_start_matches("pub ").trim_start_matches("pub(crate) ").chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            if !ident.is_empty() && ident != "r" {
                push(i, if kind == "serde" { "serde field/variant" } else { "clap field/variant" }, &ident);
            }
        }
        depth += opens - closes;
        if let Some((_, d)) = in_type
            && depth < d
        {
            in_type = None;
        }
    }
}

#[test]
fn technical_names_are_english() {
    let root = root();
    let mut hits = Vec::new();
    for f in files(&root) {
        scan_file(&root, &f, &mut hits);
    }
    let allowed = |h: &Hit, (file, lit, _): &(&str, &str, &str)| h.file.ends_with(file) && h.text.contains(lit);
    // stale exceptions are removed from the list (and prove that the scan finds them)
    let stale: Vec<&str> = ALLOWED.iter().filter(|a| !hits.iter().any(|h| allowed(h, a))).map(|a| a.1).collect();
    assert!(stale.is_empty(), "exceptions without a finding (remove from ALLOWED): {stale:?}");
    hits.retain(|h| !ALLOWED.iter().any(|a| allowed(h, a)));
    let list: Vec<String> = hits.iter().map(|h| format!("{}:{}: {} „{}“", h.file, h.line, h.what, h.text)).collect();
    assert!(
        hits.is_empty(),
        "{} German technical name(s) (docs\\AGENTEN_REGELN.md §1c: technical names always English; old names only as `LEGACY_*` constant or serde alias):\n{}",
        list.len(),
        list.join("\n")
    );
}

/// Self-test of the detector (no project files involved).
#[test]
fn detector_finds_german_names() {
    assert_eq!(german("Sprachen").as_deref(), Some("sprachen"));
    assert_eq!(german("Allgemeine Bibliothek").as_deref(), Some("allgemeine"));
    assert_eq!(german("{base}_druck_{}.png").as_deref(), Some("druck"));
    assert_eq!(german("zwischenablage-{}.png").as_deref(), Some("zwischenablage"));
    assert_eq!(german("befehlId").as_deref(), Some("befehl"));
    assert!(german("Größe").is_some());
    for ok in ["Languages", "Models", "Common Library", "{base}_print_{}.png", "clipboard-{}.png", "preferences.json", "Workspace Layouts", "FP_FILE_DIALOG", "command_id", "Original Files"] {
        assert_eq!(german(ok), None, "{ok}");
    }
    // allow-list entries carry a reason
    assert!(ALLOWED.iter().all(|(_, _, why)| !why.is_empty()));
}
