//! Quelltext-Scanner (Phase 6, I18N-KERN): findet feste Nutzertexte, die noch nicht über
//! `fp_i18n::t`/`tr!` laufen, und meldet sie als `Datei:Zeile`.
//!
//! Zwei Arten:
//! 1. **UI-Aufrufe** (fp-ui): Zeichenketten-Literale mit Wörtern, die – auch über `format!`,
//!    `String::from`, `.into()`, Arrays – an UI-Aufrufe gehen (`ui.label`, `Button::new`,
//!    `RichText::new`, `on_hover_text`, `painter.text`, `set_status`, Menü-`item`, Felder
//!    `label:`/`title:`/`text:` …).
//! 2. **Deutsche Literale**: in fp-ui jedes Literal mit Umlaut/ß/„“ außerhalb von Kommentaren und
//!    Tests; in den UI-nahen Crates (fp-engine, fp-io, fp-export, fp-script, fp-ai) Literale mit
//!    Wörtern, die in Fehlermeldungen gehen (`EngineError::Failed(…)`, `Err(…)`, `reason:` …).
//!
//! Ausnahmen (Eigennamen, Formatnamen, Einheiten, bewusst feste Texte): `i18n_ausnahmen.txt`.
//!
//! Aufruf: `cargo test -p fp-ui --test i18n_keine_festen_texte -- --ignored --nocapture`
//! (Filter auf Pfadteile: Umgebungsvariable `FP_I18N_FILTER=menu.rs`). Zunächst nur Bericht;
//! `keine_festen_texte` wird scharf geschaltet, wenn alle Bereiche umgestellt sind.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------- Lexer

#[derive(Debug, Clone)]
struct Lit {
    start: usize,
    line: usize,
    text: String,
}

/// Zerlegt Rust-Quelltext grob: Zeichenketten-Literale (mit Zeile) und eine „maskierte“ Kopie,
/// in der Kommentare und Literal-Inhalte durch Leerzeichen bzw. `x` ersetzt sind (für die Suche
/// nach umschließenden Aufrufen).
fn lex(src: &str) -> (Vec<Lit>, Vec<u8>) {
    let b = src.as_bytes();
    let mut masked = b.to_vec();
    let mut lits = Vec::new();
    let mut i = 0;
    let mut line = 1;
    let is_ident = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    while i < b.len() {
        let c = b[i];
        if c == b'\n' {
            line += 1;
            i += 1;
            continue;
        }
        // Kommentare
        if c == b'/' && i + 1 < b.len() && b[i + 1] == b'/' {
            while i < b.len() && b[i] != b'\n' {
                masked[i] = b' ';
                i += 1;
            }
            continue;
        }
        if c == b'/' && i + 1 < b.len() && b[i + 1] == b'*' {
            let mut depth = 0;
            while i < b.len() {
                if b[i] == b'/' && i + 1 < b.len() && b[i + 1] == b'*' {
                    depth += 1;
                    masked[i] = b' ';
                    masked[i + 1] = b' ';
                    i += 2;
                    continue;
                }
                if b[i] == b'*' && i + 1 < b.len() && b[i + 1] == b'/' {
                    depth -= 1;
                    masked[i] = b' ';
                    masked[i + 1] = b' ';
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                    continue;
                }
                if b[i] == b'\n' {
                    line += 1;
                } else {
                    masked[i] = b' ';
                }
                i += 1;
            }
            continue;
        }
        // Roh-Zeichenketten r"…", r#"…"#, br"…"
        let prev_ident = i > 0 && is_ident(b[i - 1]);
        if !prev_ident && (c == b'r' || (c == b'b' && i + 1 < b.len() && b[i + 1] == b'r')) {
            let mut j = i + if c == b'b' { 2 } else { 1 };
            let mut hashes = 0;
            while j < b.len() && b[j] == b'#' {
                hashes += 1;
                j += 1;
            }
            if j < b.len() && b[j] == b'"' && (hashes > 0 || j == i + 1 || (c == b'b' && j == i + 2)) {
                let start = i;
                let start_line = line;
                let body_start = j + 1;
                let mut k = body_start;
                let closing: Vec<u8> = std::iter::once(b'"').chain(std::iter::repeat_n(b'#', hashes)).collect();
                while k < b.len() && !b[k..].starts_with(&closing) {
                    if b[k] == b'\n' {
                        line += 1;
                    }
                    k += 1;
                }
                let end = (k + closing.len()).min(b.len());
                if c != b'b' {
                    lits.push(Lit { start, line: start_line, text: src[body_start..k.min(b.len())].to_string() });
                }
                for m in masked.iter_mut().take(end).skip(start) {
                    if *m != b'\n' {
                        *m = b'x';
                    }
                }
                i = end;
                continue;
            }
        }
        // normale Zeichenketten (auch b"…")
        if c == b'"' {
            let byte_str = i > 0 && b[i - 1] == b'b' && !(i > 1 && is_ident(b[i - 2]));
            let start = i;
            let start_line = line;
            let mut k = i + 1;
            while k < b.len() && b[k] != b'"' {
                if b[k] == b'\\' {
                    k += 1;
                }
                if k < b.len() && b[k] == b'\n' {
                    line += 1;
                }
                k += 1;
            }
            let end = (k + 1).min(b.len());
            if !byte_str {
                lits.push(Lit { start, line: start_line, text: src[i + 1..k.min(b.len())].to_string() });
            }
            for m in masked.iter_mut().take(end).skip(start + 1) {
                if *m != b'\n' {
                    *m = b'x';
                }
            }
            if end > start + 1 {
                masked[end - 1] = b'"';
            }
            i = end;
            continue;
        }
        // Zeichen-Literale bzw. Lebensdauern
        if c == b'\'' {
            if i + 1 < b.len() && b[i + 1] == b'\\' {
                let mut k = i + 2;
                while k < b.len() && b[k] != b'\'' {
                    k += 1;
                }
                for m in masked.iter_mut().take(k + 1).skip(i) {
                    *m = b'x';
                }
                i = k + 1;
                continue;
            }
            // ein Zeichen (auch mehrbytig) gefolgt von '
            let rest = &src[i + 1..];
            if let Some(ch) = rest.chars().next() {
                let l = ch.len_utf8();
                if i + 1 + l < b.len() && b[i + 1 + l] == b'\'' {
                    for m in masked.iter_mut().take(i + 2 + l).skip(i) {
                        *m = b'x';
                    }
                    i += 2 + l;
                    continue;
                }
            }
            i += 1;
            continue;
        }
        i += 1;
    }
    (lits, masked)
}

/// Beginn eines `#[cfg(test)]`-Moduls (Tests stehen am Dateiende): ab hier nicht prüfen.
fn test_module_start(src: &str) -> usize {
    let mut pos = 0;
    for line in src.split_inclusive('\n') {
        let t = line.trim();
        if t == "#[cfg(test)]" {
            let after = &src[pos + line.len()..];
            let next = after.lines().map(str::trim).find(|l| !l.is_empty() && !l.starts_with("#["));
            if next.is_some_and(|n| n.starts_with("mod ") || n.starts_with("pub mod ") || n.starts_with("pub(crate) mod ")) {
                return pos;
            }
        }
        pos += line.len();
    }
    src.len()
}

// ---------------------------------------------------------------------- Umschließender Aufruf

/// Aufrufe, deren Text in der Oberfläche erscheint.
const UI_SINKS: &[&str] = &[
    "label", "heading", "small", "strong", "weak", "monospace", "code", "button", "small_button", "checkbox", "radio", "radio_value",
    "selectable_label", "selectable_value", "hyperlink_to", "menu_button", "collapsing", "on_hover_text", "on_disabled_hover_text",
    "on_hover_text_at_pointer", "show_tooltip_text", "text", "set_status", "Status", "item", "fit_label", "push_button", "dialog_frame",
    "text_block", "combo", "button_row", "RichText::new", "Button::new", "Label::new", "Checkbox::new", "SelectableLabel::new",
    "WidgetText::from", "CollapsingHeader::new", "Window::new", "Hyperlink::from_label_and_url", "hint_text", "tooltip", "title",
    "confirm", "message", "status", "caption", "header", "section", "row", "label_row", "set_title", "Title",
];

/// Aufrufe, deren Fehlertext beim Nutzer landet (UI-nahe Crates).
const MSG_SINKS: &[&str] = &["Failed", "Err", "Selection", "NotFound", "Io", "Format", "NotImplemented", "Unavailable", "InvalidParams", "set_status", "Status", "msg", "fail", "bail"];

/// Interne Ausgaben (Protokoll, Tests, Entwicklerhinweise): Literale darin zählen nicht.
const INTERNAL: &[&str] = &[
    "debug", "info", "warn", "error", "trace", "log", "panic", "assert", "assert_eq", "assert_ne", "debug_assert", "expect", "unreachable",
    "eprintln", "println", "eprint", "print", "report_overflow", "todo", "unimplemented",
];

/// Hüllen, durch die hindurch weiter nach außen gesucht wird.
const TRANSPARENT: &[&str] = &["format", "concat", "String::from", "from", "Some", "Ok", "vec", "Cow::Borrowed", "Arc::from", "Box::from", "into", "to_string", "to_owned"];

/// Felder in Struktur-Literalen, deren Wert sichtbar ist.
const UI_FIELDS: &[&str] = &["label", "title", "text", "tooltip", "caption", "message", "status", "hint", "heading", "description", "reason"];

fn ident_before(m: &[u8], mut i: usize) -> (String, usize) {
    // i zeigt auf '(' bzw. '!' davor; Leerzeichen überspringen
    while i > 0 && m[i - 1] == b' ' {
        i -= 1;
    }
    if i > 0 && m[i - 1] == b'!' {
        i -= 1;
    }
    let end = i;
    while i > 0 && (m[i - 1].is_ascii_alphanumeric() || m[i - 1] == b'_' || m[i - 1] == b':') {
        i -= 1;
    }
    let full = String::from_utf8_lossy(&m[i..end]).to_string();
    // nur die letzten zwei Pfadteile (`egui::RichText::new` → `RichText::new`)
    let parts: Vec<&str> = full.split("::").filter(|s| !s.is_empty()).collect();
    let name = match parts.len() {
        0 => String::new(),
        1 => parts[0].to_string(),
        n => format!("{}::{}", parts[n - 2], parts[n - 1]),
    };
    (name, i)
}

fn field_before(m: &[u8], pos: usize) -> Option<String> {
    let mut i = pos;
    while i > 0 && (m[i - 1] == b' ' || m[i - 1] == b'\n' || m[i - 1] == b'\r' || m[i - 1] == b'&') {
        i -= 1;
    }
    if i == 0 || m[i - 1] != b':' || (i > 1 && m[i - 2] == b':') {
        return None;
    }
    i -= 1;
    let end = i;
    while i > 0 && (m[i - 1].is_ascii_alphanumeric() || m[i - 1] == b'_') {
        i -= 1;
    }
    Some(String::from_utf8_lossy(&m[i..end]).to_string())
}

/// Name des sichtbaren Aufrufs/Felds, an den das Literal bei `pos` geht (oder `None`).
fn enclosing_sink(m: &[u8], pos: usize, sinks: &[&str]) -> Option<String> {
    let matches_sink = |n: &str| sinks.contains(&n) || n.rsplit("::").next().is_some_and(|last| sinks.contains(&last) && !n.contains("::"));
    let mut child_start = pos;
    for _ in 0..8 {
        if let Some(f) = field_before(m, child_start)
            && UI_FIELDS.contains(&f.as_str())
        {
            return Some(format!("{f}:"));
        }
        let mut depth = 0i32;
        let mut i = child_start;
        let mut found = None;
        let limit = child_start.saturating_sub(1500);
        while i > limit {
            i -= 1;
            match m[i] {
                b')' | b']' | b'}' => depth += 1,
                b'(' | b'[' | b'{' => {
                    if depth == 0 {
                        found = Some(i);
                        break;
                    }
                    depth -= 1;
                }
                b';' if depth == 0 => return None,
                _ => {}
            }
        }
        let open = found?;
        match m[open] {
            b'(' => {
                let (name, name_start) = ident_before(m, open);
                if matches_sink(&name) {
                    return Some(name);
                }
                if TRANSPARENT.contains(&name.as_str()) || name.is_empty() {
                    // Tupel bzw. Hülle: weiter außen suchen
                    child_start = if name.is_empty() { open } else { name_start };
                    continue;
                }
                return None;
            }
            b'[' => {
                // vec![…] bzw. Array: weiter außen
                let (name, name_start) = ident_before(m, open);
                child_start = if name == "vec" { name_start } else { open };
                continue;
            }
            _ => return None,
        }
    }
    None
}

// ---------------------------------------------------------------------- Bewertung

struct Exceptions {
    words: BTreeSet<String>,
    literals: Vec<(String, String)>,
    /// Pfadteile, deren Dateien nur auf UI-Aufrufe geprüft werden (Texte für KI-Agenten u. ä.).
    no_german: Vec<String>,
}

/// Steht das Literal in einer internen Ausgabe (`log::warn!`, `assert!`, `report_overflow` …)?
fn in_internal_call(m: &[u8], pos: usize) -> bool {
    let mut child_start = pos;
    for _ in 0..6 {
        let mut depth = 0i32;
        let mut i = child_start;
        let mut found = None;
        let limit = child_start.saturating_sub(1500);
        while i > limit {
            i -= 1;
            match m[i] {
                b')' | b']' | b'}' => depth += 1,
                b'(' | b'[' | b'{' => {
                    if depth == 0 {
                        found = Some(i);
                        break;
                    }
                    depth -= 1;
                }
                b';' if depth == 0 => return false,
                _ => {}
            }
        }
        let Some(open) = found else { return false };
        if m[open] != b'(' {
            return false;
        }
        let (name, name_start) = ident_before(m, open);
        let last = name.rsplit("::").next().unwrap_or("");
        if INTERNAL.contains(&last) {
            return true;
        }
        if TRANSPARENT.contains(&name.as_str()) || name.is_empty() {
            child_start = if name.is_empty() { open } else { name_start };
            continue;
        }
        return false;
    }
    false
}

fn exceptions() -> Exceptions {
    let text = include_str!("i18n_ausnahmen.txt");
    let mut words = BTreeSet::new();
    let mut literals = Vec::new();
    let mut no_german = Vec::new();
    for line in text.lines() {
        let l = line.trim();
        if l.is_empty() || l.starts_with('#') {
            continue;
        }
        if let Some(rest) = l.strip_prefix("nur-ui ") {
            no_german.push(rest.trim().to_string());
        } else if let Some(rest) = l.strip_prefix("text ") {
            match rest.split_once(" | ") {
                Some((file, lit)) => literals.push((file.trim().to_string(), lit.to_string())),
                None => literals.push(("*".to_string(), rest.to_string())),
            }
        } else {
            for w in l.split_whitespace() {
                words.insert(w.to_string());
            }
        }
    }
    Exceptions { words, literals, no_german }
}

fn is_key_like(s: &str) -> bool {
    // fp-i18n-Schlüssel (`menu-file`, `cmd-file-open.tooltip`), Befehlsnamen (`file.saveAs`),
    // Bezeichner/Kennungen ohne Leerzeichen
    if s.is_empty() || s.contains(char::is_whitespace) {
        return false;
    }
    let key = s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_' | ':' | '/' | '\\' | '#' | '%' | '*' | '?' | '=' | '@'));
    key && !s.chars().next().is_some_and(|c| c.is_uppercase()) || s.contains('\\') || s.contains('/') && !s.contains(' ')
}

fn strip_placeholders(s: &str) -> String {
    let mut out = String::new();
    let mut depth = 0;
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' if chars.peek() == Some(&'{') => {
                chars.next();
                out.push('{');
            }
            '}' if depth == 0 && chars.peek() == Some(&'}') => {
                chars.next();
                out.push('}');
            }
            '{' => depth += 1,
            '}' if depth > 0 => depth -= 1,
            '\\' => {
                // Escape-Folgen (\n, \t, \u{…}, \") sind kein Text
                match chars.next() {
                    Some('u') => {
                        for c2 in chars.by_ref() {
                            if c2 == '}' {
                                break;
                            }
                        }
                    }
                    _ => out.push(' '),
                }
            }
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out
}

/// Wörter (≥ 2 Buchstaben) eines Literals, die keine Ausnahme sind.
fn words(s: &str, ex: &Exceptions) -> Vec<String> {
    let t = strip_placeholders(s);
    let mut out = Vec::new();
    for w in t.split(|c: char| !(c.is_alphabetic() || c == '-' || c == '\'')) {
        let w = w.trim_matches(|c| c == '-' || c == '\'');
        if w.chars().filter(|c| c.is_alphabetic()).count() < 2 {
            continue;
        }
        if ex.words.contains(w) {
            continue;
        }
        // Hex-Farben/Zahlen wie ff00ff
        if w.len() <= 8 && w.chars().all(|c| c.is_ascii_hexdigit()) && w.chars().any(|c| c.is_ascii_digit()) {
            continue;
        }
        out.push(w.to_string());
    }
    out
}

fn german(s: &str) -> bool {
    s.chars().any(|c| matches!(c, 'ä' | 'ö' | 'ü' | 'Ä' | 'Ö' | 'Ü' | 'ß' | '„' | '“'))
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Hit {
    file: String,
    line: usize,
    kind: &'static str,
    via: String,
    text: String,
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut v: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
    v.sort();
    for p in v {
        if p.is_dir() {
            rs_files(&p, out);
        } else if p.extension().is_some_and(|e| e == "rs") {
            // ausgelagerte Testmodule (`#[cfg(test)] mod tests;`)
            let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            if name == "tests.rs" || name.ends_with("_tests.rs") || p.parent().is_some_and(|d| d.ends_with("tests")) {
                continue;
            }
            out.push(p);
        }
    }
}

fn rel(p: &Path) -> String {
    let r = root();
    let r = r.canonicalize().unwrap_or(r);
    let p2 = p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    p2.strip_prefix(&r).map(|x| x.display().to_string()).unwrap_or_else(|_| p.display().to_string()).replace("\\\\?\\", "")
}

fn scan_file(path: &Path, ui_crate: bool, ex: &Exceptions, hits: &mut Vec<Hit>) {
    let Ok(src) = std::fs::read_to_string(path) else { return };
    let stop = test_module_start(&src);
    let (lits, masked) = lex(&src);
    let file = rel(path);
    let fname = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let check_german = !ex.no_german.iter().any(|p| file.replace('\\', "/").contains(p.as_str()));
    for lit in lits {
        if lit.start >= stop || lit.text.trim().is_empty() || is_key_like(&lit.text) || in_internal_call(&masked, lit.start) {
            continue;
        }
        if ex.literals.iter().any(|(f, l)| (f == "*" || *f == fname || file.ends_with(f.as_str())) && *l == lit.text) {
            continue;
        }
        let w = words(&lit.text, ex);
        let short: String = lit.text.chars().take(70).collect();
        if ui_crate {
            if !w.is_empty()
                && let Some(via) = enclosing_sink(&masked, lit.start, UI_SINKS)
            {
                hits.push(Hit { file: file.clone(), line: lit.line, kind: "UI", via, text: short });
                continue;
            }
            if check_german && german(&lit.text) && !w.is_empty() {
                hits.push(Hit { file: file.clone(), line: lit.line, kind: "deutsch", via: String::new(), text: short });
            }
        } else if !w.is_empty()
            && let Some(via) = enclosing_sink(&masked, lit.start, MSG_SINKS)
        {
            hits.push(Hit { file: file.clone(), line: lit.line, kind: "Meldung", via, text: short });
        }
    }
}

/// Alle Treffer (UI-Crate fp-ui, UI-nahe Crates nur Meldungen).
fn scan() -> Vec<Hit> {
    let ex = exceptions();
    let r = root();
    let mut hits = Vec::new();
    let mut ui = Vec::new();
    rs_files(&r.join("crates").join("fp-ui").join("src"), &mut ui);
    for f in &ui {
        scan_file(f, true, &ex, &mut hits);
    }
    for c in ["fp-engine", "fp-io", "fp-export", "fp-script", "fp-ai"] {
        let mut files = Vec::new();
        rs_files(&r.join("crates").join(c).join("src"), &mut files);
        for f in &files {
            scan_file(f, false, &ex, &mut hits);
        }
    }
    if let Ok(filter) = std::env::var("FP_I18N_FILTER") {
        hits.retain(|h| h.file.contains(&filter));
    }
    hits.sort();
    hits
}

fn report(hits: &[Hit]) -> String {
    let mut out = String::new();
    let mut by_file: BTreeMap<&str, Vec<&Hit>> = BTreeMap::new();
    for h in hits {
        by_file.entry(&h.file).or_default().push(h);
    }
    for (file, list) in &by_file {
        out.push_str(&format!("{file} ({})\n", list.len()));
        for h in list {
            let via = if h.via.is_empty() { String::new() } else { format!(" → {}", h.via) };
            out.push_str(&format!("  {}:{} [{}{via}] \"{}\"\n", h.file, h.line, h.kind, h.text));
        }
    }
    let mut by_crate: BTreeMap<String, usize> = BTreeMap::new();
    for h in hits {
        let c = h.file.split(['\\', '/']).nth(1).unwrap_or("?").to_string();
        *by_crate.entry(c).or_insert(0) += 1;
    }
    let kinds = |k: &str| hits.iter().filter(|h| h.kind == k).count();
    out.push_str(&format!(
        "\nfeste Texte übrig: {} (UI-Aufrufe: {}, deutsche Literale in fp-ui: {}, Meldungen: {}) – je Crate: {:?}\n",
        hits.len(),
        kinds("UI"),
        kinds("deutsch"),
        kinds("Meldung"),
        by_crate
    ));
    out
}

/// Bericht (immer grün): `cargo test -p fp-ui --test i18n_keine_festen_texte -- --ignored --nocapture`.
#[test]
#[ignore]
fn feste_texte_bericht() {
    let hits = scan();
    println!("{}", report(&hits));
}

/// Scharf: keine festen Texte mehr. Wird freigeschaltet (ohne `#[ignore]`), wenn alle Bereiche
/// umgestellt sind.
#[test]
#[ignore]
fn keine_festen_texte() {
    let hits = scan();
    assert!(hits.is_empty(), "{}", report(&hits));
}

/// Der Scanner selbst (läuft immer).
#[test]
fn scanner_erkennt_typische_faelle() {
    let src = r##"
fn f(ui: &mut Ui, st: &mut AppState) {
    ui.label("Ebenen");
    ui.label(t("panel-layers"));
    ui.add(egui::Button::new(format!("{} Objekte", n)));
    let x = RichText::new("Deckkraft").strong();
    resp.on_hover_text(String::from("Ausblenden"));
    st.set_status(format!("Datei „{name}“ gespeichert"), now);
    log::debug!("intern: {x}");
    log::warn!("{}", format!("Sprache „{x}“ fehlt"));
    let k = "file.saveAs";
    nodes.push(Node::Item { label: "Keine".into(), shortcut: None });
    painter.text(p, Align2::LEFT_TOP, "px", font, c);
    let a = ["Andere", "Farbe"];
    let c = '"'; let s = "Größe";
    // ui.label("Kommentar");
    let r = r#"roh "Text" mit Wörtern"#;
}
#[cfg(test)]
mod tests { fn g() { ui.label("Testtext"); } }
"##;
    let (lits, masked) = lex(src);
    let ex = Exceptions { words: ["px"].iter().map(|s| s.to_string()).collect(), literals: Vec::new(), no_german: Vec::new() };
    let mut found = BTreeMap::new();
    let stop = test_module_start(src);
    for l in &lits {
        if l.start >= stop || is_key_like(&l.text) || words(&l.text, &ex).is_empty() || in_internal_call(&masked, l.start) {
            continue;
        }
        found.insert(l.text.clone(), enclosing_sink(&masked, l.start, UI_SINKS));
    }
    let via = |s: &str| found.get(s).cloned().flatten();
    assert_eq!(via("Ebenen").as_deref(), Some("label"));
    assert_eq!(via("{} Objekte").as_deref(), Some("Button::new"));
    assert_eq!(via("Deckkraft").as_deref(), Some("RichText::new"));
    assert_eq!(via("Ausblenden").as_deref(), Some("on_hover_text"));
    assert_eq!(via("Datei „{name}“ gespeichert").as_deref(), Some("set_status"));
    assert_eq!(via("Keine").as_deref(), Some("label:"));
    assert!(!found.contains_key("intern: {x}") && !found.contains_key("Sprache „{x}“ fehlt"), "Protokoll zählt nicht");
    assert_eq!(via("Andere"), None, "Tabellen ohne UI-Aufruf zählen nur als deutsche Literale");
    assert!(german("Größe") && !german("Groesse"));
    assert!(!found.contains_key("file.saveAs") && !found.contains_key("panel-layers") && !found.contains_key("px"));
    assert!(!found.contains_key("Kommentar") && !found.contains_key("Testtext"));
    assert!(found.contains_key("roh \"Text\" mit Wörtern"));
}
