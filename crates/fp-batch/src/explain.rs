//! Umbenennen für Menschen (SPEC-BATCH §4.2, BATCH-3): Klartext-Erklärung eines regulären Ausdrucks
//! aus dem Syntaxbaum von `regex-syntax` (gleiche Crate-Familie wie `regex`), verständliche
//! Fehlermeldungen mit Stelle, Erklärung des Ersetzen-Felds, Übersetzung des Baukastens in ein
//! gleichwertiges Muster, Spickzettel und zeichenweiser Vergleich alt → neu für die Live-Tabelle.
//!
//! Alle Texte kommen aus `locales\<code>\batch_rename.ftl` (Schlüssel `batch-rx-*`, `batch-rn-*`).

use crate::recipe::{
    DateSource, ExifField, NamePart, OriginalTake, RegexRename, RenameSpec, SizeWhich,
};
use fp_i18n::{t, tr};
use regex_syntax::ast::{
    self, Ast, ClassPerlKind, ClassSet, ClassSetItem, GroupKind, RepetitionKind, RepetitionRange,
};

// ---------------------------------------------------------------------- Fehler

/// Ein Problem im Suchmuster: Stelle (1-basiert, in Zeichen), Meldung in Klartext, Text der Bibliothek.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegexProblem {
    pub position: Option<usize>,
    /// Übersetzte Meldung inkl. Stelle („Fehler an Stelle 7: …“).
    pub message: String,
    /// Originaltext der Bibliothek (englisch, für Profis im Tooltip).
    pub detail: String,
}

fn error_key(kind: &ast::ErrorKind) -> &'static str {
    use ast::ErrorKind as K;
    match kind {
        K::GroupUnclosed => "batch-rx-err-group-unclosed",
        K::GroupUnopened => "batch-rx-err-group-unopened",
        K::ClassUnclosed => "batch-rx-err-class-unclosed",
        K::ClassRangeInvalid | K::ClassRangeLiteral => "batch-rx-err-class-range",
        K::RepetitionMissing => "batch-rx-err-repetition-missing",
        K::RepetitionCountInvalid
        | K::RepetitionCountUnclosed
        | K::RepetitionCountDecimalEmpty
        | K::DecimalEmpty
        | K::DecimalInvalid => "batch-rx-err-repetition-count",
        K::EscapeUnexpectedEof => "batch-rx-err-escape-eof",
        K::EscapeUnrecognized
        | K::ClassEscapeInvalid
        | K::EscapeHexEmpty
        | K::EscapeHexInvalid
        | K::EscapeHexInvalidDigit => "batch-rx-err-escape",
        K::GroupNameDuplicate { .. } => "batch-rx-err-group-name-duplicate",
        K::GroupNameEmpty | K::GroupNameInvalid | K::GroupNameUnexpectedEof => {
            "batch-rx-err-group-name"
        }
        K::UnsupportedBackreference => "batch-rx-err-backreference",
        K::UnsupportedLookAround => "batch-rx-err-lookaround",
        K::FlagDanglingNegation
        | K::FlagDuplicate { .. }
        | K::FlagRepeatedNegation { .. }
        | K::FlagUnexpectedEof
        | K::FlagUnrecognized => "batch-rx-err-flag",
        K::UnicodeClassInvalid => "batch-rx-err-unicode-class",
        K::NestLimitExceeded(_) | K::CaptureLimitExceeded => "batch-rx-err-too-complex",
        _ => "batch-rx-err-generic",
    }
}

fn parse(pattern: &str) -> Result<Ast, RegexProblem> {
    ast::parse::ParserBuilder::new()
        .nest_limit(100)
        .build()
        .parse(pattern)
        .map_err(|e| {
            let pos = e.span().start.column;
            let what = t(error_key(e.kind()));
            RegexProblem {
                position: Some(pos),
                message: tr!("batch-rx-err-at", pos = pos as i64, what = what),
                detail: e.kind().to_string(),
            }
        })
}

/// Prüft ein Suchmuster wie es der Lauf übersetzt (`naming::compile_regex`). `None` = in Ordnung.
pub fn check_pattern(pattern: &str, ignore_case: bool) -> Option<RegexProblem> {
    if let Err(p) = parse(pattern) {
        return Some(p);
    }
    let r = RegexRename {
        find: pattern.to_string(),
        ignore_case,
        ..RegexRename::default()
    };
    match crate::naming::compile_regex(&r) {
        Ok(_) => None,
        Err(e) => {
            // z. B. unbekannte Unicode-Klasse oder zu groß: Text der Bibliothek, umrahmt
            let detail = e.to_string();
            Some(RegexProblem {
                position: None,
                message: tr!("batch-rx-err-compile", error = detail.clone()),
                detail,
            })
        }
    }
}

// ---------------------------------------------------------------------- Erklärung des Musters

/// Erklärt ein Suchmuster in Klartext (aktuelle Sprache), z. B. `^IMG_(\d+)` → „am Anfang „IMG_“,
/// dann eine oder mehrere Ziffern (Gruppe 1)“. Fehler → [`RegexProblem`].
pub fn explain_pattern(pattern: &str) -> Result<String, RegexProblem> {
    if pattern.is_empty() {
        return Ok(t("batch-rx-empty"));
    }
    let ast = parse(pattern)?;
    Ok(Explainer { src: pattern }.top(&ast))
}

struct Explainer<'a> {
    src: &'a str,
}

/// Feste Zeichenarten mit eigenen Formen (eins, eins oder mehr, beliebig viele, genau n).
#[derive(Clone, Copy)]
enum Atom {
    Digit,
    NotDigit,
    Word,
    NotWord,
    Space,
    NotSpace,
    Any,
}

impl Atom {
    fn key(self) -> &'static str {
        match self {
            Atom::Digit => "digit",
            Atom::NotDigit => "nondigit",
            Atom::Word => "word",
            Atom::NotWord => "nonword",
            Atom::Space => "space",
            Atom::NotSpace => "nonspace",
            Atom::Any => "any",
        }
    }
}

fn quote(s: &str) -> String {
    tr!("batch-rx-literal", text = s.to_string())
}

fn show_char(c: char) -> String {
    match c {
        '\t' => "\\t".into(),
        '\n' => "\\n".into(),
        '\r' => "\\r".into(),
        c if c.is_control() => c.escape_unicode().to_string(),
        c => c.to_string(),
    }
}

fn literal_run(s: &str) -> String {
    if !s.is_empty() && s.chars().all(|c| c == ' ') {
        let n = s.chars().count();
        return if n == 1 {
            t("batch-rx-space-literal")
        } else {
            tr!("batch-rx-space-literal-count", n = n as i64)
        };
    }
    quote(s)
}

impl Explainer<'_> {
    fn source(&self, span: &ast::Span) -> String {
        self.src
            .get(span.start.offset..span.end.offset)
            .unwrap_or("")
            .to_string()
    }

    fn top(&self, ast: &Ast) -> String {
        let s = self.desc(ast);
        if s.is_empty() { t("batch-rx-empty") } else { s }
    }

    /// Beschreibung eines Knotens; zusammengesetzte Teile werden mit „, dann“ verbunden.
    fn desc(&self, ast: &Ast) -> String {
        match ast {
            Ast::Empty(_) => String::new(),
            Ast::Concat(c) => self.concat(&c.asts),
            Ast::Literal(l) => literal_run(&show_char(l.c)),
            _ => self.single(ast),
        }
    }

    /// Ist der Knoten zusammengesetzt (mehrere Teile)?
    fn composite(&self, ast: &Ast) -> bool {
        match ast {
            Ast::Concat(c) => self.concat_parts(&c.asts).0.len() > 1,
            Ast::Alternation(_) => true,
            _ => false,
        }
    }

    /// Teile einer Folge: Literale zu einem Lauf zusammengefasst, gleiche Zeichenarten gezählt
    /// (`\d\d` → „2 Ziffern“), Anker am Anfang/Ende als Merker.
    fn concat_parts(&self, asts: &[Ast]) -> (Vec<String>, bool, bool) {
        let mut parts: Vec<String> = Vec::new();
        let mut run = String::new();
        let mut at_start = false;
        let mut at_end = false;
        // letzte Zeichenart: (Schlüssel, Anzahl, Index in parts)
        let mut last_atom: Option<(&'static str, u32, usize)> = None;
        let n = asts.len();
        for (i, a) in asts.iter().enumerate() {
            match a {
                Ast::Literal(l) => {
                    run.push_str(&show_char(l.c));
                    last_atom = None;
                    continue;
                }
                Ast::Assertion(x)
                    if i == 0
                        && matches!(
                            x.kind,
                            ast::AssertionKind::StartText | ast::AssertionKind::StartLine
                        ) =>
                {
                    at_start = true;
                    continue;
                }
                Ast::Assertion(x)
                    if i + 1 == n
                        && matches!(
                            x.kind,
                            ast::AssertionKind::EndText | ast::AssertionKind::EndLine
                        ) =>
                {
                    at_end = true;
                    continue;
                }
                Ast::Empty(_) => continue,
                _ => {}
            }
            if !run.is_empty() {
                parts.push(literal_run(&std::mem::take(&mut run)));
            }
            if let Some(k) = self.atom(a).map(Atom::key) {
                if let Some((lk, cnt, idx)) = last_atom
                    && lk == k
                {
                    parts[idx] = tr!(&format!("batch-rx-{k}-count"), n = (cnt + 1) as i64);
                    last_atom = Some((k, cnt + 1, idx));
                    continue;
                }
                parts.push(t(&format!("batch-rx-{k}")));
                last_atom = Some((k, 1, parts.len() - 1));
                continue;
            }
            last_atom = None;
            let d = self.single(a);
            if !d.is_empty() {
                parts.push(d);
            }
        }
        if !run.is_empty() {
            parts.push(literal_run(&run));
        }
        (parts, at_start, at_end)
    }

    fn concat(&self, asts: &[Ast]) -> String {
        let (parts, at_start, at_end) = self.concat_parts(asts);
        let joined = parts.join(&t("batch-rx-then"));
        match (at_start, at_end, joined.is_empty()) {
            (true, true, true) => t("batch-rx-empty-name"),
            (true, false, true) => t("batch-rx-start-only"),
            (false, true, true) => t("batch-rx-end-only"),
            (true, true, false) => tr!("batch-rx-whole", what = joined),
            (true, false, false) => tr!("batch-rx-at-start", what = joined),
            (false, true, false) => tr!("batch-rx-at-end", what = joined),
            (false, false, _) => joined,
        }
    }

    fn atom(&self, ast: &Ast) -> Option<Atom> {
        match ast {
            Ast::Dot(_) => Some(Atom::Any),
            Ast::ClassPerl(p) => Some(match (p.kind.clone(), p.negated) {
                (ClassPerlKind::Digit, false) => Atom::Digit,
                (ClassPerlKind::Digit, true) => Atom::NotDigit,
                (ClassPerlKind::Word, false) => Atom::Word,
                (ClassPerlKind::Word, true) => Atom::NotWord,
                (ClassPerlKind::Space, false) => Atom::Space,
                (ClassPerlKind::Space, true) => Atom::NotSpace,
            }),
            _ => None,
        }
    }

    /// Inhalt einer eckigen Klammer als Liste („a–z, A–Z, 0–9“), `None` bei verschachtelten Mengen.
    fn set_items(&self, set: &ClassSet) -> Option<String> {
        let items: Vec<&ClassSetItem> = match set {
            ClassSet::Item(ClassSetItem::Union(u)) => u.items.iter().collect(),
            ClassSet::Item(it) => vec![it],
            ClassSet::BinaryOp(_) => return None,
        };
        let mut out = Vec::new();
        for it in items {
            out.push(match it {
                ClassSetItem::Empty(_) => continue,
                ClassSetItem::Literal(l) => show_char(l.c),
                ClassSetItem::Range(r) => {
                    format!("{}–{}", show_char(r.start.c), show_char(r.end.c))
                }
                ClassSetItem::Perl(p) => match (p.kind.clone(), p.negated) {
                    (ClassPerlKind::Digit, false) => t("batch-rx-set-digits"),
                    (ClassPerlKind::Word, false) => t("batch-rx-set-word"),
                    (ClassPerlKind::Space, false) => t("batch-rx-set-space"),
                    _ => self.source(&p.span),
                },
                ClassSetItem::Unicode(u) => self.source(&u.span),
                ClassSetItem::Ascii(a) => self.source(&a.span),
                ClassSetItem::Bracketed(_) | ClassSetItem::Union(_) => return None,
            });
        }
        Some(out.join(", "))
    }

    /// Einzelner Knoten (kein Literal-Lauf).
    fn single(&self, ast: &Ast) -> String {
        match ast {
            Ast::Empty(_) => String::new(),
            Ast::Literal(l) => literal_run(&show_char(l.c)),
            Ast::Concat(c) => self.concat(&c.asts),
            Ast::Dot(_) | Ast::ClassPerl(_) => t(&format!(
                "batch-rx-{}",
                self.atom(ast).map(Atom::key).unwrap_or("any")
            )),
            Ast::ClassBracketed(b) => {
                let items = self
                    .set_items(&b.kind)
                    .unwrap_or_else(|| self.source(&b.span));
                tr!(
                    if b.negated {
                        "batch-rx-set-not"
                    } else {
                        "batch-rx-set"
                    },
                    items = items
                )
            }
            Ast::ClassUnicode(u) => {
                let name = self.source(&u.span);
                tr!(
                    if u.negated {
                        "batch-rx-unicode-not"
                    } else {
                        "batch-rx-unicode"
                    },
                    name = name
                )
            }
            Ast::Assertion(a) => t(match a.kind {
                ast::AssertionKind::StartText | ast::AssertionKind::StartLine => "batch-rx-start",
                ast::AssertionKind::EndText | ast::AssertionKind::EndLine => "batch-rx-end",
                ast::AssertionKind::WordBoundary => "batch-rx-word-boundary",
                ast::AssertionKind::NotWordBoundary => "batch-rx-not-word-boundary",
                _ => "batch-rx-word-boundary",
            }),
            Ast::Repetition(r) => self.repetition(r),
            Ast::Group(g) => {
                let inner = self.desc(&g.ast);
                let multi = self.composite(&g.ast);
                match &g.kind {
                    GroupKind::CaptureIndex(n) => tr!(
                        if multi {
                            "batch-rx-group-multi"
                        } else {
                            "batch-rx-group"
                        },
                        what = inner,
                        n = *n as i64
                    ),
                    GroupKind::CaptureName { name, .. } => tr!(
                        if multi {
                            "batch-rx-group-named-multi"
                        } else {
                            "batch-rx-group-named"
                        },
                        what = inner,
                        n = name.index as i64,
                        name = name.name.clone()
                    ),
                    GroupKind::NonCapturing(flags) => {
                        let inner = if multi {
                            tr!("batch-rx-paren", what = inner)
                        } else {
                            inner
                        };
                        if flags.items.is_empty() {
                            inner
                        } else {
                            format!("{inner} {}", self.flags_note(flags))
                        }
                    }
                }
            }
            Ast::Alternation(a) => {
                let alts: Vec<String> = a
                    .asts
                    .iter()
                    .map(|x| {
                        let d = self.desc(x);
                        if d.is_empty() {
                            t("batch-rx-nothing")
                        } else {
                            d
                        }
                    })
                    .collect();
                tr!("batch-rx-either", what = alts.join(&t("batch-rx-or")))
            }
            Ast::Flags(f) => self.flags_note(&f.flags),
        }
    }

    fn flags_note(&self, flags: &ast::Flags) -> String {
        let mut negate = false;
        let mut notes = Vec::new();
        for it in &flags.items {
            match &it.kind {
                ast::FlagsItemKind::Negation => negate = true,
                ast::FlagsItemKind::Flag(ast::Flag::CaseInsensitive) => notes.push(t(if negate {
                    "batch-rx-flag-case-on"
                } else {
                    "batch-rx-flag-case-off"
                })),
                ast::FlagsItemKind::Flag(_) => {}
            }
        }
        if notes.is_empty() {
            tr!("batch-rx-flags", flags = self.source(&flags.span))
        } else {
            notes.join(", ")
        }
    }

    fn repetition(&self, r: &ast::Repetition) -> String {
        let lazy = if r.greedy {
            String::new()
        } else {
            t("batch-rx-lazy")
        };
        // feste Zeichenarten mit eigenen, natürlichen Formen
        if let Some(a) = self.atom(&r.ast) {
            let k = a.key();
            let s = match &r.op.kind {
                RepetitionKind::OneOrMore => Some(t(&format!("batch-rx-{k}-plus"))),
                RepetitionKind::ZeroOrMore => Some(t(&format!("batch-rx-{k}-star"))),
                RepetitionKind::Range(RepetitionRange::Exactly(n)) if *n > 1 => {
                    Some(tr!(&format!("batch-rx-{k}-count"), n = *n as i64))
                }
                _ => None,
            };
            if let Some(s) = s {
                return format!("{s}{lazy}");
            }
        }
        if let Ast::ClassBracketed(b) = &*r.ast
            && !b.negated
            && let Some(items) = self.set_items(&b.kind)
        {
            let s = match &r.op.kind {
                RepetitionKind::OneOrMore => Some(tr!("batch-rx-set-plus", items = items.clone())),
                RepetitionKind::ZeroOrMore => Some(tr!("batch-rx-set-star", items = items.clone())),
                RepetitionKind::Range(RepetitionRange::Exactly(n)) if *n > 1 => Some(tr!(
                    "batch-rx-set-count",
                    items = items.clone(),
                    n = *n as i64
                )),
                _ => None,
            };
            if let Some(s) = s {
                return format!("{s}{lazy}");
            }
        }
        let inner = self.single(&r.ast);
        let inner = if self.composite(&r.ast) {
            tr!("batch-rx-paren", what = inner)
        } else {
            inner
        };
        let s = match &r.op.kind {
            RepetitionKind::ZeroOrOne => tr!("batch-rx-rep-optional", what = inner),
            RepetitionKind::ZeroOrMore => tr!("batch-rx-rep-star", what = inner),
            RepetitionKind::OneOrMore => tr!("batch-rx-rep-plus", what = inner),
            RepetitionKind::Range(RepetitionRange::Exactly(n)) => {
                tr!("batch-rx-rep-exactly", what = inner, n = *n as i64)
            }
            RepetitionKind::Range(RepetitionRange::AtLeast(n)) => {
                tr!("batch-rx-rep-at-least", what = inner, n = *n as i64)
            }
            RepetitionKind::Range(RepetitionRange::Bounded(a, b)) => tr!(
                "batch-rx-rep-bounded",
                what = inner,
                min = *a as i64,
                max = *b as i64
            ),
        };
        format!("{s}{lazy}")
    }
}

// ---------------------------------------------------------------------- Erklärung des Ersetzen-Felds

fn date_desc(key: &str, fmt: &str) -> String {
    tr!(
        key,
        format = if fmt.is_empty() {
            "YYYY-MM-DD".to_string()
        } else {
            fmt.to_string()
        }
    )
}

fn placeholder_desc(inner: &str) -> Option<String> {
    let (key, arg) = match inner.split_once(':') {
        Some((k, a)) => (k, Some(a)),
        None => (inner, None),
    };
    Some(match key {
        "n" | "nf" => {
            let mut it = arg.unwrap_or("").split(':');
            let digits: i64 = it.next().and_then(|a| a.trim().parse().ok()).unwrap_or(1);
            let start: i64 = it.next().and_then(|a| a.trim().parse().ok()).unwrap_or(1);
            let step: i64 = it.next().and_then(|a| a.trim().parse().ok()).unwrap_or(1);
            let base = if start == 1 && step == 1 {
                tr!("batch-rn-counter", digits = digits)
            } else {
                tr!(
                    "batch-rn-counter-full",
                    digits = digits,
                    start = start,
                    step = step
                )
            };
            if key == "nf" {
                tr!("batch-rn-per-folder", what = base)
            } else {
                base
            }
        }
        "date" => date_desc("batch-rn-ph-date", arg.unwrap_or("")),
        "today" => date_desc("batch-rn-ph-today", arg.unwrap_or("")),
        "exif-date" => date_desc("batch-rn-ph-exif-date", arg.unwrap_or("")),
        "w" => t("batch-rn-ph-width"),
        "h" => t("batch-rn-ph-height"),
        "folder" => t("batch-rn-ph-folder"),
        "ext" => t("batch-rn-ph-ext"),
        "format" => t("batch-rn-ph-format"),
        "exif" => t(&format!("batch-rn-exif-{}", arg.unwrap_or("model"))),
        "name" => match arg.map(|a| a.split_once(':').unwrap_or((a, ""))) {
            None => t("batch-rn-ph-name"),
            Some(("first", n)) => tr!(
                "batch-rn-ph-name-first",
                n = n.trim().parse::<i64>().unwrap_or(0)
            ),
            Some(("last", n)) => tr!(
                "batch-rn-ph-name-last",
                n = n.trim().parse::<i64>().unwrap_or(0)
            ),
            Some(("trim", _)) => t("batch-rn-ph-name-trim"),
            _ => return None,
        },
        _ => return None,
    })
}

/// Erklärt das Ersetzen-Feld: Gruppenverweise, Platzhalter und wörtlichen Text, z. B.
/// `${1}_{n:3}` → „Gruppe 1 + „_“ + Nummer (3 Stellen)“. Leer → „(nichts – Treffer wird entfernt)“.
pub fn explain_replacement(rep: &str) -> String {
    if rep.is_empty() {
        return t("batch-rn-rep-empty");
    }
    let mut parts: Vec<String> = Vec::new();
    let mut lit = String::new();
    let chars: Vec<char> = rep.chars().collect();
    let mut i = 0;
    let flush = |lit: &mut String, parts: &mut Vec<String>| {
        if !lit.is_empty() {
            parts.push(literal_run(&std::mem::take(lit)));
        }
    };
    while i < chars.len() {
        let c = chars[i];
        if c == '$' {
            match chars.get(i + 1) {
                Some('$') => {
                    lit.push('$');
                    i += 2;
                    continue;
                }
                Some('{') => {
                    if let Some(end) = chars[i + 2..].iter().position(|&x| x == '}') {
                        let name: String = chars[i + 2..i + 2 + end].iter().collect();
                        flush(&mut lit, &mut parts);
                        parts.push(group_ref(&name));
                        i += 3 + end;
                        continue;
                    }
                }
                Some(d) if d.is_ascii_alphanumeric() || *d == '_' => {
                    let mut j = i + 1;
                    while j < chars.len() && (chars[j].is_ascii_alphanumeric() || chars[j] == '_') {
                        j += 1;
                    }
                    // `$1_x` liest der Lauf als Gruppe 1 + „_x“ (naming::normalize_group_refs)
                    let raw: String = chars[i + 1..j].iter().collect();
                    let name = if raw.starts_with(|c: char| c.is_ascii_digit()) {
                        raw.chars().take_while(char::is_ascii_digit).collect()
                    } else {
                        raw
                    };
                    flush(&mut lit, &mut parts);
                    parts.push(group_ref(&name));
                    i += 1 + name.chars().count();
                    continue;
                }
                _ => {}
            }
            lit.push('$');
            i += 1;
            continue;
        }
        if c == '{' {
            if chars.get(i + 1) == Some(&'{') {
                lit.push('{');
                i += 2;
                continue;
            }
            if let Some(end) = chars[i + 1..].iter().position(|&x| x == '}') {
                let inner: String = chars[i + 1..i + 1 + end].iter().collect();
                if let Some(d) = placeholder_desc(&inner) {
                    flush(&mut lit, &mut parts);
                    parts.push(d);
                    i += 2 + end;
                    continue;
                }
            }
        }
        lit.push(c);
        i += 1;
    }
    flush(&mut lit, &mut parts);
    parts.join(" + ")
}

fn group_ref(name: &str) -> String {
    match name.parse::<i64>() {
        Ok(0) => t("batch-rn-group-whole"),
        Ok(n) => tr!("batch-rn-group", n = n),
        Err(_) => tr!("batch-rn-group-named", name = name.to_string()),
    }
}

// ---------------------------------------------------------------------- Baukasten → Regex

fn escape_replacement_text(s: &str) -> String {
    s.replace('$', "$$").replace('{', "{{")
}

fn exif_name(f: ExifField) -> &'static str {
    match f {
        ExifField::Model => "model",
        ExifField::Make => "make",
        ExifField::Lens => "lens",
        ExifField::Iso => "iso",
        ExifField::Aperture => "aperture",
    }
}

/// Ersetzen-Text eines Bausteins (gleichwertig zum Baukasten in `naming::build_name`).
pub fn part_template(p: &NamePart) -> String {
    match p {
        NamePart::Original { take, count } => match take {
            OriginalTake::Whole => "${1}".to_string(),
            OriginalTake::First => format!("{{name:first:{count}}}"),
            OriginalTake::Last => format!("{{name:last:{count}}}"),
            OriginalTake::NoTrailingDigits => "{name:trim}".to_string(),
        },
        NamePart::Text { text } => escape_replacement_text(text),
        NamePart::Counter {
            start,
            step,
            digits,
            per_folder,
        } => {
            let d = (*digits).clamp(1, 9);
            let key = if *per_folder { "nf" } else { "n" };
            if *start == 1 && *step == 1 {
                format!("{{{key}:{d}}}")
            } else {
                format!("{{{key}:{d}:{start}:{step}}}")
            }
        }
        NamePart::Date { source, format } => {
            let key = match source {
                DateSource::Today => "today",
                DateSource::Modified => "date",
                DateSource::Exif => "exif-date",
            };
            format!("{{{key}:{}}}", format.replace('}', ""))
        }
        NamePart::Size { which } => match which {
            SizeWhich::Both => "{w}x{h}".to_string(),
            SizeWhich::Width => "{w}".to_string(),
            SizeWhich::Height => "{h}".to_string(),
        },
        NamePart::Folder => "{folder}".to_string(),
        NamePart::Exif { field } => format!("{{exif:{}}}", exif_name(*field)),
        NamePart::Format => "{format}".to_string(),
        NamePart::Unknown => String::new(),
    }
}

/// Übersetzt die Bausteine in ein gleichwertiges Muster (Umschalten Baukasten → Regex): Suchen
/// `^(.*)$` (ganzer Name = Gruppe 1), Ersetzen aus den Platzhaltern. Die übrigen Optionen
/// (Ersetzungen, Leerzeichen, Präfix/Suffix, Schreibweise, Endung) gelten in beiden Arten gleich.
pub fn builder_to_regex(spec: &RenameSpec) -> RegexRename {
    let replace: String = spec.parts.iter().map(part_template).collect();
    RegexRename {
        find: "^(.*)$".to_string(),
        replace,
        ignore_case: false,
        all: true,
        whole_name: false,
    }
}

/// Ist das Muster noch das (unveränderte) Ergebnis einer Übersetzung bzw. leer? Dann darf der
/// Wechsel zum Regex es neu übersetzen, ohne Eingaben des Nutzers zu verlieren.
pub fn regex_is_replaceable(spec: &RenameSpec) -> bool {
    spec.regex.find.trim().is_empty() || spec.regex.find == "^(.*)$" && !spec.regex.whole_name
}

// ---------------------------------------------------------------------- Spickzettel

/// Beispiel des Spickzettels: Titel (Textschlüssel), Suchen, Ersetzen, Beispielname.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CheatEntry {
    pub key: &'static str,
    pub find: &'static str,
    pub replace: &'static str,
    pub sample: &'static str,
}

/// Acht Beispiele zum Einfügen (§4.2).
pub const CHEAT_SHEET: [CheatEntry; 8] = [
    CheatEntry {
        key: "batch-rn-cheat-trailing-number",
        find: r"[\s_-]*\d+$",
        replace: "",
        sample: "DSC_0042",
    },
    CheatEntry {
        key: "batch-rn-cheat-date-swap",
        find: r"(\d{4})-(\d\d)-(\d\d)",
        replace: "$3.$2.$1",
        sample: "scan 2026-10-13",
    },
    CheatEntry {
        key: "batch-rn-cheat-spaces",
        find: r"\s+",
        replace: "_",
        sample: "summer   2026  v2",
    },
    CheatEntry {
        key: "batch-rn-cheat-prefix-swap",
        find: "^IMG_",
        replace: "Photo_",
        sample: "IMG_1234",
    },
    CheatEntry {
        key: "batch-rn-cheat-swap-parts",
        find: r"^(\w+)-(\w+)$",
        replace: "$2-$1",
        sample: "logo-blue",
    },
    CheatEntry {
        key: "batch-rn-cheat-append-number",
        find: "^(.*)$",
        replace: "${1}_{n:3}",
        sample: "image",
    },
    CheatEntry {
        key: "batch-rn-cheat-brackets",
        find: r"\s*\([^)]*\)",
        replace: "",
        sample: "poster (2)",
    },
    CheatEntry {
        key: "batch-rn-cheat-exif-date",
        find: "^",
        replace: "{exif-date:YYYY-MM-DD}_",
        sample: "DSC0815",
    },
];

/// Wendet ein Muster auf einen Beispielnamen an (Spickzettel, Vorschau ohne Datei). Platzhalter
/// bleiben sichtbar stehen.
pub fn apply_sample(find: &str, replace: &str, sample: &str) -> Option<String> {
    let re = regex::Regex::new(find).ok()?;
    Some(
        re.replace_all(
            sample,
            crate::naming::normalize_group_refs(replace).as_str(),
        )
        .to_string(),
    )
}

// ---------------------------------------------------------------------- Vergleich alt → neu

/// Stück einer zeichenweisen Gegenüberstellung: Text und ob es geändert ist (alt: entfernt,
/// neu: eingefügt).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffPart {
    pub text: String,
    pub changed: bool,
}

fn push(v: &mut Vec<DiffPart>, c: char, changed: bool) {
    match v.last_mut() {
        Some(l) if l.changed == changed => l.text.push(c),
        _ => v.push(DiffPart {
            text: c.to_string(),
            changed,
        }),
    }
}

/// Zeichenweiser Vergleich (längste gemeinsame Teilfolge, bei langen Namen gemeinsamer Anfang/Ende).
/// Liefert (alt mit entfernten Stücken, neu mit eingefügten Stücken).
pub fn char_diff(old: &str, new: &str) -> (Vec<DiffPart>, Vec<DiffPart>) {
    let a: Vec<char> = old.chars().collect();
    let b: Vec<char> = new.chars().collect();
    let mut ra = Vec::new();
    let mut rb = Vec::new();
    // gemeinsamer Anfang und gemeinsames Ende vorweg
    let pre = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let suf = a[pre..]
        .iter()
        .rev()
        .zip(b[pre..].iter().rev())
        .take_while(|(x, y)| x == y)
        .count();
    let (am, bm) = (&a[pre..a.len() - suf], &b[pre..b.len() - suf]);
    a[..pre].iter().for_each(|&c| push(&mut ra, c, false));
    b[..pre].iter().for_each(|&c| push(&mut rb, c, false));
    if am.len() * bm.len() <= 40_000 {
        // LCS-Tabelle
        let (n, m) = (am.len(), bm.len());
        let mut dp = vec![0u16; (n + 1) * (m + 1)];
        for i in (0..n).rev() {
            for j in (0..m).rev() {
                dp[i * (m + 1) + j] = if am[i] == bm[j] {
                    dp[(i + 1) * (m + 1) + j + 1] + 1
                } else {
                    dp[(i + 1) * (m + 1) + j].max(dp[i * (m + 1) + j + 1])
                };
            }
        }
        let (mut i, mut j) = (0, 0);
        while i < n && j < m {
            if am[i] == bm[j] {
                push(&mut ra, am[i], false);
                push(&mut rb, bm[j], false);
                i += 1;
                j += 1;
            } else if dp[(i + 1) * (m + 1) + j] >= dp[i * (m + 1) + j + 1] {
                push(&mut ra, am[i], true);
                i += 1;
            } else {
                push(&mut rb, bm[j], true);
                j += 1;
            }
        }
        am[i..].iter().for_each(|&c| push(&mut ra, c, true));
        bm[j..].iter().for_each(|&c| push(&mut rb, c, true));
    } else {
        am.iter().for_each(|&c| push(&mut ra, c, true));
        bm.iter().for_each(|&c| push(&mut rb, c, true));
    }
    a[a.len() - suf..]
        .iter()
        .for_each(|&c| push(&mut ra, c, false));
    b[b.len() - suf..]
        .iter()
        .for_each(|&c| push(&mut rb, c, false));
    (ra, rb)
}
