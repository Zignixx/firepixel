//! Prüfung von Sprachdateien unabhängig vom laufenden Sprachzustand: fehlende und verwaiste
//! Schlüssel, Platzhalter-Abweichungen (`{ $name }` je Schlüssel), Syntaxfehler, doppelte
//! Schlüssel und Zugriffstasten-Konflikte (zwei Einträge desselben Menüs bzw. Dialogs mit
//! derselben `&`-Taste). Grundlage für die Kerntests und `firepixel-cli sprache pruefen`.
//!
//! Maßstab ist die eingebaute Ausgangssprache ([`crate::SOURCE_LANG`]).

use fluent_syntax::ast;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Art eines Befunds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    /// Datei nicht lesbar bzw. kein Fluent.
    Syntax,
    /// Schlüssel in derselben Sprache doppelt.
    Duplicate,
    /// Platzhalter in der Übersetzung, den das Programm nicht übergibt (zeigt dann einen Fehler).
    PlaceholderUnknown,
    /// Platzhalter der Ausgangssprache fehlt in der Übersetzung (Wert wird nicht angezeigt).
    PlaceholderMissing,
    /// Zwei Einträge einer Gruppe (Menü/Dialog) mit derselben Zugriffstaste.
    AccessConflict,
    /// Ausgangstext hat eine Zugriffstaste, die Übersetzung nicht.
    AccessMissing,
    /// Schlüssel fehlt (Rückfall auf Englisch bzw. Deutsch).
    Missing,
    /// Schlüssel gibt es in der Ausgangssprache nicht (wird nie benutzt).
    Orphan,
}

impl Kind {
    /// Fehler (sonst Warnung): Syntax, doppelte Schlüssel, unbekannte Platzhalter.
    pub fn is_error(self) -> bool {
        matches!(self, Kind::Syntax | Kind::Duplicate | Kind::PlaceholderUnknown)
    }

    /// Kurzname (für JSON und Ausgabe).
    pub fn code(self) -> &'static str {
        match self {
            Kind::Syntax => "syntax",
            Kind::Duplicate => "doppelt",
            Kind::PlaceholderUnknown => "platzhalter-unbekannt",
            Kind::PlaceholderMissing => "platzhalter-fehlt",
            Kind::AccessConflict => "zugriffstaste-konflikt",
            Kind::AccessMissing => "zugriffstaste-fehlt",
            Kind::Missing => "fehlt",
            Kind::Orphan => "verwaist",
        }
    }
}

/// Ein Befund.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Problem {
    pub kind: Kind,
    /// Sprachcode.
    pub lang: String,
    /// Datei (bei fehlenden Schlüsseln: die Datei der Ausgangssprache).
    pub file: String,
    /// Zeile (1-basiert, 0 = unbekannt).
    pub line: usize,
    /// Schlüssel (leer bei Syntaxfehlern ohne Schlüssel).
    pub key: String,
    /// Erläuterung.
    pub detail: String,
}

impl std::fmt::Display for Problem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let level = crate::t(if self.kind.is_error() { "cli-check-error" } else { "cli-check-warning" });
        write!(f, "{level} [{}] {}", self.kind.code(), self.file)?;
        if self.line > 0 {
            write!(f, ":{}", self.line)?;
        }
        if !self.key.is_empty() {
            write!(f, " {}", self.key)?;
        }
        if !self.detail.is_empty() {
            write!(f, ": {}", self.detail)?;
        }
        Ok(())
    }
}

/// Ein Schlüssel einer Sprache.
#[derive(Clone, Debug, Default)]
pub struct KeyInfo {
    pub file: String,
    pub line: usize,
    /// Platzhalter (`$name`, ohne `$`), aus allen Varianten und Auswahlausdrücken.
    pub vars: BTreeSet<String>,
    /// Lesbarer Text (Vorgabevarianten, Platzhalter als `{ $x }`).
    pub text: String,
    /// Zugriffstaste (klein).
    pub access: Option<char>,
}

/// Alle Schlüssel einer Sprache (inkl. `key.attribut` und Terme `-name`).
#[derive(Clone, Debug, Default)]
pub struct LangData {
    pub code: String,
    /// `language-name`, falls vorhanden.
    pub name: Option<String>,
    pub files: Vec<String>,
    pub keys: BTreeMap<String, KeyInfo>,
    /// Syntaxfehler und Doppelungen.
    pub problems: Vec<Problem>,
}

fn line_of(text: &str, byte: usize) -> usize {
    text[..byte.min(text.len())].bytes().filter(|b| *b == b'\n').count() + 1
}

/// Zeilen, in denen Einträge beginnen: Name → Zeilennummern (in Reihenfolge).
fn entry_lines(text: &str) -> BTreeMap<String, Vec<usize>> {
    let mut out: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, line) in text.lines().enumerate() {
        let first = line.chars().next();
        if !first.is_some_and(|c| c.is_ascii_alphabetic() || c == '-') {
            continue;
        }
        if let Some((name, _)) = line.split_once('=') {
            let name = name.trim();
            if !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
                out.entry(name.to_string()).or_default().push(i + 1);
            }
        }
    }
    out
}

fn inline_vars(e: &ast::InlineExpression<&str>, vars: &mut BTreeSet<String>, text: &mut String) {
    match e {
        ast::InlineExpression::VariableReference { id } => {
            vars.insert(id.name.to_string());
            text.push_str(&format!("{{ ${} }}", id.name));
        }
        ast::InlineExpression::StringLiteral { value } => text.push_str(value),
        ast::InlineExpression::NumberLiteral { value } => text.push_str(value),
        ast::InlineExpression::FunctionReference { id, arguments } => {
            let mut inner = String::new();
            for a in &arguments.positional {
                inline_vars(a, vars, &mut inner);
            }
            for n in &arguments.named {
                inline_vars(&n.value, vars, &mut String::new());
            }
            text.push_str(&format!("{{ {}({inner}) }}", id.name));
        }
        ast::InlineExpression::MessageReference { id, attribute } => {
            text.push_str(&format!("{{ {}{} }}", id.name, attribute.as_ref().map(|a| format!(".{}", a.name)).unwrap_or_default()));
        }
        ast::InlineExpression::TermReference { id, arguments, .. } => {
            if let Some(args) = arguments {
                for a in &args.positional {
                    inline_vars(a, vars, &mut String::new());
                }
                for n in &args.named {
                    inline_vars(&n.value, vars, &mut String::new());
                }
            }
            text.push_str(&format!("{{ -{} }}", id.name));
        }
        ast::InlineExpression::Placeable { expression } => expr_vars(expression, vars, text),
    }
}

fn expr_vars(e: &ast::Expression<&str>, vars: &mut BTreeSet<String>, text: &mut String) {
    match e {
        ast::Expression::Inline(i) => inline_vars(i, vars, text),
        ast::Expression::Select { selector, variants } => {
            inline_vars(selector, vars, &mut String::new());
            for v in variants {
                let mut vt = String::new();
                pattern_vars(&v.value, vars, &mut vt);
                if v.default {
                    text.push_str(&vt);
                }
            }
        }
    }
}

fn pattern_vars(p: &ast::Pattern<&str>, vars: &mut BTreeSet<String>, text: &mut String) {
    for el in &p.elements {
        match el {
            ast::PatternElement::TextElement { value } => text.push_str(value),
            ast::PatternElement::Placeable { expression } => expr_vars(expression, vars, text),
        }
    }
}

fn key_info(file: &str, line: usize, p: &ast::Pattern<&str>) -> KeyInfo {
    let mut vars = BTreeSet::new();
    let mut text = String::new();
    pattern_vars(p, &mut vars, &mut text);
    let access = crate::format::access_info(&text).key;
    KeyInfo { file: file.to_string(), line, vars, text, access }
}

impl LangData {
    /// Leere Sprache.
    pub fn new(code: &str) -> LangData {
        LangData { code: code.to_string(), ..LangData::default() }
    }

    /// Fügt eine Datei hinzu (Syntaxfehler und doppelte Schlüssel landen in `problems`).
    pub fn add_file(&mut self, file: &str, text: &str) {
        self.files.push(file.to_string());
        let text = text.trim_start_matches('\u{feff}');
        let lines = entry_lines(text);
        let mut seen: BTreeMap<String, usize> = BTreeMap::new();
        let mut next_line = |name: &str| {
            let n = seen.entry(name.to_string()).or_insert(0);
            let l = lines.get(name).and_then(|v| v.get(*n)).copied().unwrap_or(0);
            *n += 1;
            l
        };
        let res = match fluent_syntax::parser::parse(text) {
            Ok(r) => r,
            Err((r, errs)) => {
                for e in errs {
                    let snippet: String = text.get(e.pos.clone()).unwrap_or("").chars().take(40).collect();
                    self.problems.push(Problem {
                        kind: Kind::Syntax,
                        lang: self.code.clone(),
                        file: file.to_string(),
                        line: line_of(text, e.pos.start),
                        key: String::new(),
                        detail: crate::tr!("cli-check-syntax", kind = e.kind.to_string(), text = snippet.trim()),
                    });
                }
                r
            }
        };
        for entry in &res.body {
            let (name, value, attrs) = match entry {
                ast::Entry::Message(m) => (m.id.name.to_string(), m.value.as_ref(), &m.attributes),
                ast::Entry::Term(t) => (format!("-{}", t.id.name), Some(&t.value), &t.attributes),
                _ => continue,
            };
            let line = next_line(&name);
            if let Some(prev) = self.keys.get(&name) {
                self.problems.push(Problem {
                    kind: Kind::Duplicate,
                    lang: self.code.clone(),
                    file: file.to_string(),
                    line,
                    key: name.clone(),
                    detail: crate::tr!("cli-check-duplicate", file = prev.file.as_str(), line = prev.line),
                });
                continue;
            }
            let info = match value {
                Some(p) => key_info(file, line, p),
                None => KeyInfo { file: file.to_string(), line, ..KeyInfo::default() },
            };
            if name == "language-name" {
                self.name = Some(info.text.clone());
            }
            self.keys.insert(name.clone(), info);
            for a in attrs.iter() {
                self.keys.insert(format!("{name}.{}", a.id.name), key_info(file, line, &a.value));
            }
        }
    }

    /// Sprache aus den eingebauten Dateien.
    pub fn builtin(code: &str) -> LangData {
        let mut d = LangData::new(code);
        for (c, file, text) in crate::embedded_files() {
            if *c == code {
                d.add_file(&format!("{c}/{file}"), text);
            }
        }
        d
    }

    /// Nur die Schlüssel einer Datei (Dateiname ohne Ordner, z. B. `menue.ftl`).
    pub fn only_file(&self, file_name: &str) -> LangData {
        let mut d = LangData { code: self.code.clone(), name: self.name.clone(), ..LangData::default() };
        d.files = self.files.iter().filter(|f| file_base(f) == file_name).cloned().collect();
        d.keys = self.keys.iter().filter(|(_, k)| file_base(&k.file) == file_name).map(|(n, k)| (n.clone(), k.clone())).collect();
        d
    }
}

fn file_base(f: &str) -> &str {
    f.rsplit(['/', '\\']).next().unwrap_or(f)
}

/// Gruppe für Zugriffstasten: Einträge derselben Gruppe stehen im selben Menü bzw. Dialog.
/// * `menu-…`/`cmd-…`: Menüpfad = Schlüssel ohne Präfix bis zum letzten `-`
///   (`cmd-modify-align-left` und `menu-modify-align` → Gruppen `modify-align` bzw. `modify`;
///   `menu-file` → Menüleiste).
/// * `dlg-<dialog>-…` → `dlg-<dialog>`; `btn-…` → gemeinsame Knöpfe.
/// * sonst: Schlüssel bis zum letzten `-`.
pub fn access_group(key: &str) -> String {
    let parent = |s: &str| s.rsplit_once('-').map(|(a, _)| a.to_string()).unwrap_or_default();
    if let Some(rest) = key.strip_prefix("cmd-").or_else(|| key.strip_prefix("menu-")) {
        return format!("menu:{}", parent(rest));
    }
    if let Some(rest) = key.strip_prefix("dlg-") {
        return format!("dlg-{}", rest.split('-').next().unwrap_or(""));
    }
    if key.starts_with("btn-") {
        return "btn".to_string();
    }
    parent(key)
}

/// Zugriffstasten-Konflikte einer Sprache: (Gruppe, Taste) → Schlüssel (nur Werte, keine Attribute).
pub fn access_conflicts(d: &LangData) -> BTreeMap<(String, char), Vec<String>> {
    let mut by: BTreeMap<(String, char), Vec<String>> = BTreeMap::new();
    for (k, info) in &d.keys {
        if k.contains('.') {
            continue;
        }
        if let Some(c) = info.access {
            by.entry((access_group(k), c)).or_default().push(k.clone());
        }
    }
    by.retain(|_, v| v.len() > 1);
    by
}

/// Vergleicht eine Sprache mit der Ausgangssprache `reference`.
/// `complete`: fehlende Schlüssel melden (bei Teil-Übersetzungen/Korrekturdateien abschalten).
pub fn compare(reference: &LangData, cand: &LangData, complete: bool) -> Vec<Problem> {
    let mut out = cand.problems.clone();
    let p = |kind: Kind, file: &str, line: usize, key: &str, detail: String| Problem { kind, lang: cand.code.clone(), file: file.to_string(), line, key: key.to_string(), detail };
    if complete {
        for (k, info) in &reference.keys {
            if !cand.keys.contains_key(k) {
                out.push(p(Kind::Missing, &info.file, info.line, k, crate::tr!("cli-check-source-text", text = info.text.as_str())));
            }
        }
    }
    for (k, info) in &cand.keys {
        let Some(r) = reference.keys.get(k) else {
            out.push(p(Kind::Orphan, &info.file, info.line, k, String::new()));
            continue;
        };
        let unknown: Vec<String> = info.vars.difference(&r.vars).map(|v| format!("${v}")).collect();
        if !unknown.is_empty() {
            out.push(p(Kind::PlaceholderUnknown, &info.file, info.line, k, crate::tr!("cli-check-allowed", vars = unknown.join(", "), allowed = fmt_vars(&r.vars))));
        }
        let missing: Vec<String> = r.vars.difference(&info.vars).map(|v| format!("${v}")).collect();
        if !missing.is_empty() {
            out.push(p(Kind::PlaceholderMissing, &info.file, info.line, k, missing.join(", ")));
        }
        if r.access.is_some() && info.access.is_none() && !k.contains('.') {
            out.push(p(Kind::AccessMissing, &info.file, info.line, k, crate::tr!("cli-check-access-missing", text = r.text.as_str())));
        }
    }
    // Konflikte, die es in der Ausgangssprache so nicht gibt
    let ref_conf = access_conflicts(reference);
    let ref_sets: BTreeSet<Vec<String>> = ref_conf.values().cloned().collect();
    for ((group, c), keys) in access_conflicts(cand) {
        if ref_sets.contains(&keys) {
            continue;
        }
        let first = &cand.keys[&keys[0]];
        let texts: Vec<String> = keys.iter().map(|k| format!("{k} „{}“", cand.keys[k].text)).collect();
        out.push(p(Kind::AccessConflict, &first.file, first.line, &keys[0], crate::tr!("cli-check-access-conflict", key = c.to_string(), group = group.as_str(), list = texts.join(", "))));
    }
    out.sort();
    out
}

fn fmt_vars(v: &BTreeSet<String>) -> String {
    if v.is_empty() { crate::t("cli-check-none") } else { v.iter().map(|x| format!("${x}")).collect::<Vec<_>>().join(", ") }
}

/// Namen der eingebauten Bereichsdateien (`menue.ftl` …).
pub fn builtin_file_names() -> BTreeSet<String> {
    crate::embedded_files().iter().map(|(_, f, _)| f.to_string()).collect()
}

/// Ergebnis einer Prüfung.
#[derive(Clone, Debug, Default)]
pub struct Report {
    /// Geprüfte Sprachen: (Code, Name, Schlüssel, fehlende Schlüssel, Dateien).
    pub langs: Vec<LangSummary>,
    pub problems: Vec<Problem>,
}

/// Zusammenfassung je Sprache.
#[derive(Clone, Debug, Default)]
pub struct LangSummary {
    pub code: String,
    pub name: Option<String>,
    pub files: Vec<String>,
    pub keys: usize,
    pub missing: usize,
    /// Anteil übersetzter Schlüssel der Ausgangssprache in Prozent.
    pub percent: f64,
}

impl Report {
    pub fn errors(&self) -> usize {
        self.problems.iter().filter(|p| p.kind.is_error()).count()
    }
    pub fn warnings(&self) -> usize {
        self.problems.len() - self.errors()
    }
    /// Anzahl je Art.
    pub fn counts(&self) -> BTreeMap<Kind, usize> {
        let mut m = BTreeMap::new();
        for p in &self.problems {
            *m.entry(p.kind).or_insert(0) += 1;
        }
        m
    }
}

fn read_ftl(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    String::from_utf8(bytes).map_err(|_| crate::tr!("cli-check-not-utf8", path = path.display().to_string()))
}

fn ftl_files(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut v: Vec<_> = std::fs::read_dir(dir)
        .map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.is_file() && p.extension().is_some_and(|x| x.eq_ignore_ascii_case("ftl"))).collect())
        .unwrap_or_default();
    v.sort();
    v
}

fn file_name(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
}

/// Sprachen aus einem Pfad lesen:
/// * Datei `<code>.ftl` → eine Sprache in einer Datei; Bereichsdatei (`fr\menue.ftl`) → nur dieser Bereich.
/// * Ordner mit Bereichsdateien (`fr\`) → eine Sprache (Code = Ordnername).
/// * Ordner mit Sprachordnern bzw. `<code>.ftl`-Dateien (`Sprachen\`, `locales\`) → alle.
///
/// Rückgabe: (Sprache, nur diese Bereichsdatei prüfen).
pub fn load_path(path: &Path) -> Result<Vec<(LangData, Option<String>)>, String> {
    let areas = builtin_file_names();
    let mut out = Vec::new();
    if path.is_file() {
        let name = file_name(path);
        let text = read_ftl(path)?;
        if areas.contains(&name) {
            let code = path.parent().map(file_name).unwrap_or_default();
            let mut d = LangData::new(&code);
            d.add_file(&path.display().to_string(), &text);
            out.push((d, Some(name)));
        } else {
            let code = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            let mut d = LangData::new(&code);
            d.add_file(&path.display().to_string(), &text);
            out.push((d, None));
        }
        return Ok(out);
    }
    if !path.is_dir() {
        return Err(crate::tr!("cli-check-no-path", path = path.display().to_string()));
    }
    let direct = ftl_files(path);
    let is_lang_folder = direct.iter().any(|p| areas.contains(&file_name(p)));
    if is_lang_folder {
        let mut d = LangData::new(&file_name(path));
        for f in &direct {
            match read_ftl(f) {
                Ok(t) => d.add_file(&f.display().to_string(), &t),
                Err(e) => d.problems.push(Problem { kind: Kind::Syntax, lang: d.code.clone(), file: f.display().to_string(), line: 0, key: String::new(), detail: e }),
            }
        }
        out.push((d, None));
        return Ok(out);
    }
    for f in &direct {
        let code = f.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let mut d = LangData::new(&code);
        match read_ftl(f) {
            Ok(t) => d.add_file(&f.display().to_string(), &t),
            Err(e) => d.problems.push(Problem { kind: Kind::Syntax, lang: code.clone(), file: f.display().to_string(), line: 0, key: String::new(), detail: e }),
        }
        out.push((d, None));
    }
    let mut dirs: Vec<_> = std::fs::read_dir(path).map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect()).unwrap_or_default();
    dirs.sort();
    for dir in dirs {
        let files = ftl_files(&dir);
        if files.is_empty() {
            continue;
        }
        let mut d = LangData::new(&file_name(&dir));
        for f in &files {
            match read_ftl(f) {
                Ok(t) => d.add_file(&f.display().to_string(), &t),
                Err(e) => d.problems.push(Problem { kind: Kind::Syntax, lang: d.code.clone(), file: f.display().to_string(), line: 0, key: String::new(), detail: e }),
            }
        }
        out.push((d, None));
    }
    if out.is_empty() {
        return Err(crate::tr!("cli-check-no-ftl", path = path.display().to_string()));
    }
    Ok(out)
}

/// Prüft einen Pfad (siehe [`load_path`]) gegen die eingebaute Ausgangssprache.
pub fn check_path(path: &Path) -> Result<Report, String> {
    let reference = LangData::builtin(crate::SOURCE_LANG);
    let mut report = Report::default();
    for (d, area) in load_path(path)? {
        let r = match &area {
            Some(f) => reference.only_file(f),
            None => reference.clone(),
        };
        report.problems.extend(compare(&r, &d, true));
        report.langs.push(summary(&r, &d));
    }
    Ok(report)
}

/// Zusammenfassung einer Sprache gegen die Referenz.
pub fn summary(reference: &LangData, d: &LangData) -> LangSummary {
    let total = reference.keys.len();
    let missing = reference.keys.keys().filter(|k| !d.keys.contains_key(*k)).count();
    LangSummary {
        code: d.code.clone(),
        name: d.name.clone(),
        files: d.files.clone(),
        keys: d.keys.len(),
        missing,
        percent: if total == 0 { 100.0 } else { (total - missing) as f64 * 100.0 / total as f64 },
    }
}

/// Prüft alle eingebauten Sprachen gegen die Ausgangssprache (für Tests).
pub fn check_builtin() -> Report {
    let reference = LangData::builtin(crate::SOURCE_LANG);
    let mut codes: Vec<&str> = crate::embedded_files().iter().map(|(c, _, _)| *c).collect();
    codes.dedup();
    let mut report = Report::default();
    // Konflikte der Ausgangssprache selbst werden nicht gemeldet (Wortlaut des Originals bleibt)
    report.problems.extend(reference.problems.clone());
    for code in codes {
        if code == crate::SOURCE_LANG {
            continue;
        }
        let d = LangData::builtin(code);
        report.problems.extend(compare(&reference, &d, true));
        report.langs.push(summary(&reference, &d));
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lang(code: &str, text: &str) -> LangData {
        let mut d = LangData::new(code);
        d.add_file("x.ftl", text);
        d
    }

    #[test]
    fn finds_all_kinds() {
        let de = lang("de", "a = &Datei\nb = Bea&rbeiten\nc = { $n } Ebenen\nd = Hallo { $name }\n");
        let fr = lang("fr", "a = &Fichier\nb = &Fusion\nc = { $n } calques { $x }\nd = Bonjour\nzz = verwaist\nzz = doppelt\nkaputt = { \n");
        let probs = compare(&de, &fr, true);
        let kinds: BTreeSet<Kind> = probs.iter().map(|p| p.kind).collect();
        for k in [Kind::Syntax, Kind::Duplicate, Kind::PlaceholderUnknown, Kind::PlaceholderMissing, Kind::AccessConflict, Kind::Orphan] {
            assert!(kinds.contains(&k), "{k:?} fehlt: {probs:#?}");
        }
        let dup = probs.iter().find(|p| p.kind == Kind::Duplicate).map(|p| p.line);
        assert_eq!(dup, Some(6));
        let unk = probs.iter().find(|p| p.kind == Kind::PlaceholderUnknown).map(|p| (p.line, p.key.clone()));
        assert_eq!(unk, Some((3, "c".to_string())));
    }

    #[test]
    fn plural_variants_count_as_placeholders() {
        let de = lang("de", "n = { $n ->\n    [one] eine Ebene\n   *[other] { $n } Ebenen\n}\n");
        let en = lang("en", "n = { $n ->\n    [one] one layer\n   *[other] { $n } layers\n}\n");
        assert!(compare(&de, &en, true).is_empty());
        assert_eq!(de.keys["n"].text, "{ $n } Ebenen");
    }

    #[test]
    fn groups() {
        assert_eq!(access_group("cmd-modify-align-left"), "menu:modify-align");
        assert_eq!(access_group("menu-modify-align"), "menu:modify");
        assert_eq!(access_group("cmd-file-open"), "menu:file");
        assert_eq!(access_group("menu-file"), "menu:");
        assert_eq!(access_group("dlg-prefs-undo-steps"), "dlg-prefs");
        assert_eq!(access_group("btn-yes"), "btn");
    }
}
