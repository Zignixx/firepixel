//! Namensbildung und Plan (SPEC-BATCH §4, §2.7, §5.2 Punkt 2): Baukasten und Regex, Ersetzungen,
//! Schreibweise, ungültige Zeichen, Kürzen, Endung, Ausgabeordner und Konfliktauflösung –
//! deterministisch in Listenreihenfolge, **vor** dem Start (unabhängig von der Parallelität).
//!
//! Reihenfolge (§4.4, verbindlich): 1. Grundname ohne Endung (`.fw.png` zählt als eine Endung),
//! 2. Bausteine bzw. Regex, 3. Ersetzungen → Leerzeichen → Präfix/Suffix → Umlaute/Sonderzeichen →
//! Schreibweise, 4. ungültige Zeichen → `_`, reservierte Namen → `_` anhängen, Länge kürzen,
//! 5. leerer Name → `_`, führender Punkt → `_`, 6. Endung des Zielformats, 7. Konflikte.

use crate::error::{BatchError, Result};
use crate::item::ItemInfo;
use crate::output::{OutputFormat, resolve_format};
use crate::recipe::{
    CaseMode, Conflict, DateSource, ExifField, ExtCase, NamePart, OriginalTake, Recipe,
    RegexRename, RenameMode, RenameSpec, SizeWhich, TargetKind,
};
use chrono::{Datelike, NaiveDateTime, Timelike};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Höchstlänge eines Dateinamens (Zeichen, inkl. Endung).
pub const MAX_NAME_LEN: usize = 255;
/// Ab dieser Gesamtlänge wird mit `\\?\` geschrieben (Warnung im Trockenlauf).
pub const MAX_PATH_LEN: usize = 260;

/// Zerlegt einen Dateinamen in Grundname und Endung (ohne Punkt); `.fw.png` ist eine Endung.
pub fn split_extension(name: &str) -> (String, String) {
    let lower = name.to_lowercase();
    if lower.ends_with(".fw.png") && name.len() > 7 {
        return (
            name[..name.len() - 7].to_string(),
            name[name.len() - 6..].to_string(),
        );
    }
    match name.rfind('.') {
        Some(i) if i > 0 => (name[..i].to_string(), name[i + 1..].to_string()),
        _ => (name.to_string(), String::new()),
    }
}

/// Hinweis zu einem Namen bzw. einem Ziel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum NameHint {
    InvalidChars,
    ReservedName,
    Truncated,
    EmptyName,
    LeadingDot,
    ExifDateMissing,
    /// Zielname schon vergeben (im Lauf oder im Ziel) – nach Regel aufgelöst.
    Conflict,
    /// Ziel existiert und wird überschrieben.
    Overwrite,
    /// Original wird ersetzt (Sicherung in `Original Files`).
    ReplaceOriginal,
    /// Sicherung wurde erzwungen.
    BackupForced,
    /// Übersprungen nach Konfliktregel.
    Skipped,
    /// Gesamtpfad > 260 Zeichen (`\\?\`-Pfad).
    LongPath,
    /// „Wie Quelle“ nicht schreibbar → PNG.
    FormatFallback,
}

impl NameHint {
    /// Meldung in der aktuellen Sprache.
    pub fn message(self) -> String {
        let key = match self {
            NameHint::InvalidChars => "batch-hint-invalid-chars",
            NameHint::ReservedName => "batch-hint-reserved-name",
            NameHint::Truncated => "batch-hint-truncated",
            NameHint::EmptyName => "batch-hint-empty-name",
            NameHint::LeadingDot => "batch-hint-leading-dot",
            NameHint::ExifDateMissing => "batch-hint-exif-date-missing",
            NameHint::Conflict => "batch-hint-conflict",
            NameHint::Overwrite => "batch-hint-overwrite",
            NameHint::ReplaceOriginal => "batch-hint-replace-original",
            NameHint::BackupForced => "batch-hint-backup-forced",
            NameHint::Skipped => "batch-hint-skipped",
            NameHint::LongPath => "batch-hint-long-path",
            NameHint::FormatFallback => "batch-hint-format-fallback",
        };
        fp_i18n::t(key)
    }
}

/// Werte für die Namensbildung eines Elements.
#[derive(Clone, Debug)]
pub struct NameContext<'a> {
    pub info: &'a ItemInfo,
    /// Position in der Liste (0-basiert) und innerhalb des Quellordners.
    pub list_index: usize,
    pub folder_index: usize,
    /// „Heute“ (für Tests fest).
    pub now: NaiveDateTime,
    /// Endung des Zielformats ohne Punkt (schon in der gewünschten Schreibweise).
    pub out_ext: String,
    /// Zielformat (Baustein `format`).
    pub out_format: String,
}

/// Ergebnis der Namensbildung.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NameResult {
    /// Name ohne Endung.
    pub stem: String,
    /// Name mit Endung.
    pub file_name: String,
    pub hints: Vec<NameHint>,
}

/// Übersetzt den regulären Ausdruck des Profi-Modus.
pub fn compile_regex(r: &RegexRename) -> Result<regex::Regex> {
    regex::RegexBuilder::new(&r.find)
        .case_insensitive(r.ignore_case)
        .size_limit(1 << 22)
        .build()
        .map_err(|e| BatchError::Invalid(fp_i18n::tr!("batch-err-regex", error = e.to_string())))
}

fn local_from_system(t: std::time::SystemTime) -> NaiveDateTime {
    chrono::DateTime::<chrono::Local>::from(t).naive_local()
}

/// Aktuelle Ortszeit.
pub fn now_local() -> NaiveDateTime {
    chrono::Local::now().naive_local()
}

/// Datum nach Vorlage: `YYYY`, `YY`, `MM`, `DD`, `HH`, `mm`, `ss`; alles andere wörtlich.
pub fn format_date(d: &NaiveDateTime, fmt: &str) -> String {
    let tokens: [(&str, String); 7] = [
        ("YYYY", format!("{:04}", d.year())),
        ("YY", format!("{:02}", d.year().rem_euclid(100))),
        ("MM", format!("{:02}", d.month())),
        ("DD", format!("{:02}", d.day())),
        ("HH", format!("{:02}", d.hour())),
        ("mm", format!("{:02}", d.minute())),
        ("ss", format!("{:02}", d.second())),
    ];
    let mut out = String::new();
    let mut rest = fmt;
    'outer: while !rest.is_empty() {
        for (t, v) in &tokens {
            if let Some(r) = rest.strip_prefix(t) {
                out.push_str(v);
                rest = r;
                continue 'outer;
            }
        }
        let c = rest.chars().next().unwrap_or(' ');
        out.push(c);
        rest = &rest[c.len_utf8()..];
    }
    out
}

fn modified_date(info: &ItemInfo, now: NaiveDateTime) -> NaiveDateTime {
    info.modified.map(local_from_system).unwrap_or(now)
}

/// EXIF-Datum oder Rückfall Änderungsdatum (`true` = Rückfall).
fn exif_date(info: &ItemInfo, now: NaiveDateTime) -> (NaiveDateTime, bool) {
    match info.exif.as_ref().and_then(|e| e.date_taken) {
        Some(d) => (d, false),
        None => (modified_date(info, now), true),
    }
}

fn folder_name(info: &ItemInfo) -> String {
    info.path
        .parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default()
}

fn exif_field(info: &ItemInfo, f: ExifField) -> String {
    let Some(e) = &info.exif else {
        return String::new();
    };
    match f {
        ExifField::Make => e.make.clone().unwrap_or_default(),
        ExifField::Model => e.model.clone().unwrap_or_default(),
        ExifField::Lens => e.lens.clone().unwrap_or_default(),
        ExifField::Iso => e.iso.map(|i| i.to_string()).unwrap_or_default(),
        ExifField::Aperture => e.aperture.clone().unwrap_or_default(),
    }
}

fn counter(start: u64, step: u64, digits: u32, index: usize) -> String {
    let v = start.saturating_add(step.saturating_mul(index as u64));
    format!("{:0width$}", v, width = digits.clamp(1, 9) as usize)
}

fn take_original(base: &str, take: OriginalTake, count: usize) -> String {
    match take {
        OriginalTake::Whole => base.to_string(),
        OriginalTake::First => base.chars().take(count).collect(),
        OriginalTake::Last => {
            let n = base.chars().count();
            base.chars().skip(n.saturating_sub(count)).collect()
        }
        OriginalTake::NoTrailingDigits => base
            .trim_end_matches(|c: char| c.is_ascii_digit())
            .trim_end_matches(['_', '-', ' ', '.'])
            .to_string(),
    }
}

/// Zähler-Platzhalter `{n:Stellen[:Start[:Schritt]]}` (BATCH-3): Vorgaben 1 Stelle, Start 1, Schritt 1.
fn counter_placeholder(arg: Option<&str>, index: usize) -> String {
    let mut it = arg.unwrap_or("").split(':');
    let digits = it.next().and_then(|a| a.trim().parse().ok()).unwrap_or(1);
    let start = it.next().and_then(|a| a.trim().parse().ok()).unwrap_or(1);
    let step = it.next().and_then(|a| a.trim().parse().ok()).unwrap_or(1);
    counter(start, step, digits, index)
}

/// Platzhalter `{name…}`: ganz, `{name:first:N}`, `{name:last:N}`, `{name:trim}` (ohne Ziffern am Ende).
fn name_placeholder(arg: Option<&str>, base: &str) -> Option<String> {
    let Some(arg) = arg else {
        return Some(base.to_string());
    };
    let (what, n) = match arg.split_once(':') {
        Some((w, n)) => (w, n.trim().parse::<usize>().ok()),
        None => (arg, None),
    };
    match what {
        "first" => Some(take_original(base, OriginalTake::First, n.unwrap_or(0))),
        "last" => Some(take_original(base, OriginalTake::Last, n.unwrap_or(0))),
        "trim" => Some(take_original(base, OriginalTake::NoTrailingDigits, 0)),
        _ => None,
    }
}

/// Platzhalter im Ersetzen-Feld des Regex-Modus: `{n}`, `{n:3}`, `{n:3:10:5}` (Stellen, Start,
/// Schritt), `{nf:…}` (je Ordner), `{date:…}`, `{today:…}`, `{exif-date:…}`, `{w}`, `{h}`, `{folder}`,
/// `{ext}`, `{name}`, `{name:first:N}`, `{name:last:N}`, `{name:trim}`, `{exif:model|make|lens|iso|aperture}`,
/// `{format}`; `{{` = wörtliches `{`. Unbekannte bleiben stehen.
fn expand_placeholders(
    template: &str,
    base: &str,
    ctx: &NameContext,
    hints: &mut Vec<NameHint>,
) -> String {
    let mut out = String::new();
    let mut rest = template;
    while let Some(i) = rest.find('{') {
        out.push_str(&rest[..i]);
        if rest[i + 1..].starts_with('{') {
            out.push('{');
            rest = &rest[i + 2..];
            continue;
        }
        let after = &rest[i + 1..];
        let Some(j) = after.find('}') else {
            out.push_str(&rest[i..]);
            return out;
        };
        let inner = &after[..j];
        let (key, arg) = match inner.split_once(':') {
            Some((k, a)) => (k, Some(a)),
            None => (inner, None),
        };
        let v = match key {
            "n" => Some(counter_placeholder(arg, ctx.list_index)),
            "nf" => Some(counter_placeholder(arg, ctx.folder_index)),
            "format" => Some(ctx.out_format.clone()),
            "exif" => match arg.unwrap_or("model") {
                "model" => Some(exif_field(ctx.info, ExifField::Model)),
                "make" => Some(exif_field(ctx.info, ExifField::Make)),
                "lens" => Some(exif_field(ctx.info, ExifField::Lens)),
                "iso" => Some(exif_field(ctx.info, ExifField::Iso)),
                "aperture" => Some(exif_field(ctx.info, ExifField::Aperture)),
                _ => None,
            },
            "date" => Some(format_date(
                &modified_date(ctx.info, ctx.now),
                arg.unwrap_or("YYYY-MM-DD"),
            )),
            "today" => Some(format_date(&ctx.now, arg.unwrap_or("YYYY-MM-DD"))),
            "exif-date" => {
                let (d, fallback) = exif_date(ctx.info, ctx.now);
                if fallback && !hints.contains(&NameHint::ExifDateMissing) {
                    hints.push(NameHint::ExifDateMissing);
                }
                Some(format_date(&d, arg.unwrap_or("YYYY-MM-DD")))
            }
            "w" => Some(ctx.info.width.to_string()),
            "h" => Some(ctx.info.height.to_string()),
            "folder" => Some(folder_name(ctx.info)),
            "ext" => Some(split_extension(&ctx.info.file_name()).1),
            "name" => name_placeholder(arg, base),
            _ => None,
        };
        match v {
            // `$` im eingesetzten Wert nicht als Gruppenverweis lesen
            Some(v) => out.push_str(&v.replace('$', "$$")),
            None => {
                out.push('{');
                out.push_str(inner);
                out.push('}');
            }
        }
        rest = &after[j + 1..];
    }
    out.push_str(rest);
    out
}

/// `$1_x` wie erwartet lesen: `$<Ziffern>` wird zu `${Ziffern}` (die regex-Crate läse sonst die
/// Gruppe `1_x`); `$$` bleibt ein wörtliches `$`.
pub fn normalize_group_refs(rep: &str) -> String {
    let mut out = String::with_capacity(rep.len() + 8);
    let mut it = rep.chars().peekable();
    while let Some(c) = it.next() {
        if c != '$' {
            out.push(c);
            continue;
        }
        match it.peek().copied() {
            Some('$') => {
                it.next();
                out.push_str("$$");
            }
            Some(d) if d.is_ascii_digit() => {
                let mut n = String::new();
                while let Some(d) = it.peek().copied().filter(char::is_ascii_digit) {
                    n.push(d);
                    it.next();
                }
                out.push_str(&format!("${{{n}}}"));
            }
            _ => out.push('$'),
        }
    }
    out
}

fn replace_text(s: &str, find: &str, rep: &str, all: bool, match_case: bool) -> String {
    if find.is_empty() {
        return s.to_string();
    }
    if match_case {
        return if all {
            s.replace(find, rep)
        } else {
            s.replacen(find, rep, 1)
        };
    }
    let re = regex::RegexBuilder::new(&regex::escape(find))
        .case_insensitive(true)
        .build();
    match re {
        Ok(re) => {
            let rep = regex::NoExpand(rep);
            if all {
                re.replace_all(s, rep).to_string()
            } else {
                re.replace(s, rep).to_string()
            }
        }
        Err(_) => s.to_string(),
    }
}

/// Umlaute und Akzente umschreiben (ä → ae, ß → ss, é → e …).
pub fn transliterate(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        let r: &str = match c {
            'ä' => "ae",
            'ö' => "oe",
            'ü' => "ue",
            'Ä' => "Ae",
            'Ö' => "Oe",
            'Ü' => "Ue",
            'ß' => "ss",
            'ẞ' => "SS",
            'æ' => "ae",
            'Æ' => "Ae",
            'œ' => "oe",
            'Œ' => "Oe",
            'á' | 'à' | 'â' | 'ã' | 'å' | 'ā' | 'ą' => "a",
            'Á' | 'À' | 'Â' | 'Ã' | 'Å' | 'Ā' | 'Ą' => "A",
            'é' | 'è' | 'ê' | 'ë' | 'ē' | 'ę' | 'ě' => "e",
            'É' | 'È' | 'Ê' | 'Ë' | 'Ē' | 'Ę' | 'Ě' => "E",
            'í' | 'ì' | 'î' | 'ï' | 'ī' => "i",
            'Í' | 'Ì' | 'Î' | 'Ï' | 'Ī' => "I",
            'ó' | 'ò' | 'ô' | 'õ' | 'ø' | 'ō' => "o",
            'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ø' | 'Ō' => "O",
            'ú' | 'ù' | 'û' | 'ū' | 'ů' => "u",
            'Ú' | 'Ù' | 'Û' | 'Ū' | 'Ů' => "U",
            'ý' | 'ÿ' => "y",
            'Ý' => "Y",
            'ç' | 'č' | 'ć' => "c",
            'Ç' | 'Č' | 'Ć' => "C",
            'ñ' | 'ń' | 'ň' => "n",
            'Ñ' | 'Ń' | 'Ň' => "N",
            'š' | 'ś' => "s",
            'Š' | 'Ś' => "S",
            'ž' | 'ź' | 'ż' => "z",
            'Ž' | 'Ź' | 'Ż' => "Z",
            'ł' => "l",
            'Ł' => "L",
            'ř' => "r",
            'Ř' => "R",
            'đ' => "d",
            'Đ' => "D",
            _ => {
                out.push(c);
                continue;
            }
        };
        out.push_str(r);
    }
    out
}

fn words(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut prev_lower = false;
    for c in s.chars() {
        if c.is_whitespace() || c == '_' || c == '-' {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            prev_lower = false;
            continue;
        }
        if c.is_uppercase() && prev_lower && !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
        prev_lower = c.is_lowercase() || c.is_ascii_digit();
        cur.push(c);
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn capitalize(w: &str) -> String {
    let mut cs = w.chars();
    match cs.next() {
        Some(f) => f
            .to_uppercase()
            .chain(cs.flat_map(char::to_lowercase))
            .collect(),
        None => String::new(),
    }
}

/// Schreibweise anwenden.
pub fn apply_case(s: &str, mode: CaseMode) -> String {
    match mode {
        CaseMode::Keep => s.to_string(),
        CaseMode::Lower => s.to_lowercase(),
        CaseMode::Upper => s.to_uppercase(),
        CaseMode::Sentence => capitalize(s),
        CaseMode::Title => {
            let mut out = String::new();
            let mut start = true;
            for c in s.chars() {
                if c.is_whitespace() || c == '_' || c == '-' {
                    start = true;
                    out.push(c);
                } else if start {
                    out.extend(c.to_uppercase());
                    start = false;
                } else {
                    out.extend(c.to_lowercase());
                }
            }
            out
        }
        CaseMode::Kebab => words(s)
            .iter()
            .map(|w| w.to_lowercase())
            .collect::<Vec<_>>()
            .join("-"),
        CaseMode::Snake => words(s)
            .iter()
            .map(|w| w.to_lowercase())
            .collect::<Vec<_>>()
            .join("_"),
        CaseMode::Camel => words(s)
            .iter()
            .enumerate()
            .map(|(i, w)| {
                if i == 0 {
                    w.to_lowercase()
                } else {
                    capitalize(w)
                }
            })
            .collect(),
    }
}

const RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Schritte 4 und 5 von §4.4 auf einen Grundnamen (ohne Endung): ungültige Zeichen → `_`,
/// Punkt/Leerzeichen am Ende → `_`, reservierte Namen → `_` anhängen, leer → `_`, führender Punkt → `_`.
pub fn sanitize_stem(stem: &str, hints: &mut Vec<NameHint>) -> String {
    let mut s: String = stem
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') {
                '_'
            } else {
                c
            }
        })
        .collect();
    if s != stem {
        hints.push(NameHint::InvalidChars);
    }
    let trimmed = s.trim_end_matches(['.', ' ']);
    if trimmed.len() != s.len() {
        let n = s.len() - trimmed.len();
        s = format!("{trimmed}{}", "_".repeat(n));
        if !hints.contains(&NameHint::InvalidChars) {
            hints.push(NameHint::InvalidChars);
        }
    }
    let upper = s.to_ascii_uppercase();
    let head = upper.split('.').next().unwrap_or("");
    if RESERVED.contains(&head) {
        s.push('_');
        hints.push(NameHint::ReservedName);
    }
    if s.is_empty() {
        s.push('_');
        hints.push(NameHint::EmptyName);
    }
    if s.starts_with('.') {
        s.replace_range(0..1, "_");
        hints.push(NameHint::LeadingDot);
    }
    s
}

/// Bildet den Dateinamen eines Elements (§4.4 Schritte 1–6, ohne Konflikte).
pub fn build_name(spec: &RenameSpec, ctx: &NameContext) -> Result<NameResult> {
    let mut hints = Vec::new();
    let file_name = ctx.info.file_name();
    let (base, _src_ext) = split_extension(&file_name);
    // 2. Bausteine bzw. Regex
    let mut s = match spec.mode {
        RenameMode::Builder => {
            let mut out = String::new();
            for p in &spec.parts {
                match p {
                    NamePart::Original { take, count } => {
                        out.push_str(&take_original(&base, *take, *count))
                    }
                    NamePart::Text { text } => out.push_str(text),
                    NamePart::Counter {
                        start,
                        step,
                        digits,
                        per_folder,
                    } => out.push_str(&counter(
                        *start,
                        *step,
                        *digits,
                        if *per_folder {
                            ctx.folder_index
                        } else {
                            ctx.list_index
                        },
                    )),
                    NamePart::Date { source, format } => {
                        let d = match source {
                            DateSource::Today => ctx.now,
                            DateSource::Modified => modified_date(ctx.info, ctx.now),
                            DateSource::Exif => {
                                let (d, fallback) = exif_date(ctx.info, ctx.now);
                                if fallback && !hints.contains(&NameHint::ExifDateMissing) {
                                    hints.push(NameHint::ExifDateMissing);
                                }
                                d
                            }
                        };
                        out.push_str(&format_date(&d, format));
                    }
                    NamePart::Size { which } => out.push_str(&match which {
                        SizeWhich::Both => format!("{}x{}", ctx.info.width, ctx.info.height),
                        SizeWhich::Width => ctx.info.width.to_string(),
                        SizeWhich::Height => ctx.info.height.to_string(),
                    }),
                    NamePart::Folder => out.push_str(&folder_name(ctx.info)),
                    NamePart::Exif { field } => out.push_str(&exif_field(ctx.info, *field)),
                    NamePart::Format => out.push_str(&ctx.out_format),
                    NamePart::Unknown => {}
                }
            }
            out
        }
        RenameMode::Regex => {
            let re = compile_regex(&spec.regex)?;
            let subject = if spec.regex.whole_name {
                file_name.clone()
            } else {
                base.clone()
            };
            let rep = normalize_group_refs(&expand_placeholders(
                &spec.regex.replace,
                &base,
                ctx,
                &mut hints,
            ));
            let replaced = if spec.regex.all {
                re.replace_all(&subject, rep.as_str()).to_string()
            } else {
                re.replace(&subject, rep.as_str()).to_string()
            };
            if spec.regex.whole_name {
                split_extension(&replaced).0
            } else {
                replaced
            }
        }
    };
    // 3. Ersetzungen (in beiden Arten, damit der Wechsel Baukasten → Regex gleichwertig bleibt) →
    // Leerzeichen → Präfix/Suffix → Umlaute/Sonderzeichen → Schreibweise
    for r in &spec.replacements {
        s = replace_text(&s, &r.find, &r.replace, r.all, r.match_case);
    }
    if let Some(sp) = &spec.spaces {
        s = s.replace(' ', sp);
    }
    s = format!("{}{s}{}", spec.prefix, spec.suffix);
    if spec.transliterate {
        s = transliterate(&s);
    }
    if spec.strip_special {
        s.retain(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | ' '));
    }
    s = apply_case(&s, spec.case);
    // 4./5. ungültige Zeichen, reservierte Namen, leer, führender Punkt
    let mut stem = sanitize_stem(&s, &mut hints);
    // Länge kürzen (Endung bleibt)
    let ext_len = if ctx.out_ext.is_empty() {
        0
    } else {
        ctx.out_ext.chars().count() + 1
    };
    let room = MAX_NAME_LEN.saturating_sub(ext_len).max(1);
    if stem.chars().count() > room {
        stem = stem.chars().take(room).collect();
        // kein Punkt/Leerzeichen am Ende nach dem Kürzen
        if stem.ends_with(['.', ' ']) {
            stem.pop();
            stem.push('_');
        }
        hints.push(NameHint::Truncated);
    }
    // 6. Endung
    let file_name = if ctx.out_ext.is_empty() {
        stem.clone()
    } else {
        format!("{stem}.{}", ctx.out_ext)
    };
    Ok(NameResult {
        stem,
        file_name,
        hints,
    })
}

/// Endung des Ziels in der gewünschten Schreibweise. Bei reinen Dateischritten die Originalendung.
pub fn target_extension(
    spec: &RenameSpec,
    fmt: OutputFormat,
    source_name: &str,
    file_only: bool,
) -> String {
    let raw = if file_only || fmt == OutputFormat::Same {
        let ext = split_extension(source_name).1;
        if spec.jpeg_as_jpg && ext.eq_ignore_ascii_case("jpeg") {
            "jpg".to_string()
        } else {
            ext
        }
    } else {
        fmt.extension(spec.jpeg_as_jpg).to_string()
    };
    match spec.extension {
        ExtCase::Lower => raw.to_lowercase(),
        ExtCase::Upper => raw.to_uppercase(),
        ExtCase::Keep => raw,
    }
}

// ------------------------------------------------------------------ Plan

/// Was mit einem Element geschieht.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PlanAction {
    /// Neues Ziel schreiben.
    Write,
    /// Vorhandenes Ziel (≠ Quelle) überschreiben.
    Overwrite,
    /// Original ersetzen (Sicherung in `Original Files`).
    ReplaceOriginal,
    /// Überspringen (Konfliktregel).
    Skip,
    /// Lauf fragt nach (Regel `ask`); ohne Antwort wie `rename` mit `altTarget`.
    Ask,
}

/// Ein Eintrag des Plans.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanEntry {
    /// Position in der Liste.
    pub index: usize,
    pub source: PathBuf,
    pub target: PathBuf,
    /// Ausweichziel `name (k).ext` (bei `Ask`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt_target: Option<PathBuf>,
    pub action: PlanAction,
    /// Zielformat (aufgelöst, technisch).
    pub format: String,
    pub hints: Vec<NameHint>,
    /// Konflikt mit dem Eintrag dieses Index (gleicher Zielname im Lauf).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflict_with: Option<usize>,
}

impl PlanEntry {
    /// Hinweise als Text (aktuelle Sprache).
    pub fn hint_messages(&self) -> Vec<String> {
        self.hints.iter().map(|h| h.message()).collect()
    }
}

/// Plan eines Auftrags (Trockenlauf-Tabelle).
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub entries: Vec<PlanEntry>,
    /// Anzulegende Ausgabeordner.
    pub output_dirs: Vec<PathBuf>,
    /// Nur Dateischritte (kein Dekodieren).
    pub file_only: bool,
    /// Zahl der Einträge mit Konflikt.
    pub conflicts: usize,
}

/// Optionen des Plans.
#[derive(Clone, Debug, Default)]
pub struct PlanOptions {
    /// „Heute“ (Tests); sonst Ortszeit.
    pub now: Option<NaiveDateTime>,
}

fn key(p: &Path) -> String {
    let s = p.to_string_lossy().replace('/', "\\");
    if cfg!(windows) { s.to_lowercase() } else { s }
}

fn dir_names(cache: &mut HashMap<PathBuf, HashSet<String>>, dir: &Path) -> HashSet<String> {
    cache
        .entry(dir.to_path_buf())
        .or_insert_with(|| {
            std::fs::read_dir(crate::output::long_path(dir))
                .map(|rd| {
                    rd.flatten()
                        .map(|e| {
                            let n = e.file_name().to_string_lossy().to_string();
                            if cfg!(windows) { n.to_lowercase() } else { n }
                        })
                        .collect()
                })
                .unwrap_or_default()
        })
        .clone()
}

fn exists_in(cache: &mut HashMap<PathBuf, HashSet<String>>, p: &Path) -> bool {
    let (Some(dir), Some(name)) = (p.parent(), p.file_name()) else {
        return false;
    };
    let n = name.to_string_lossy();
    let n = if cfg!(windows) {
        n.to_lowercase()
    } else {
        n.to_string()
    };
    if let Some(set) = cache.get(dir) {
        return set.contains(&n);
    }
    dir_names(cache, dir).contains(&n)
}

/// Ordnername für den neuen Ausgabeordner in `base`: `batch-output`, wenn er fehlt oder leer ist,
/// sonst `batch-output-2`, `-3` …
fn unique_folder(base: &Path, name: &str) -> PathBuf {
    let free = |p: &Path| {
        !p.exists()
            || (p.is_dir()
                && std::fs::read_dir(p)
                    .map(|mut d| d.next().is_none())
                    .unwrap_or(false))
    };
    let first = base.join(name);
    if free(&first) {
        return first;
    }
    (2..)
        .map(|k| base.join(format!("{name}-{k}")))
        .find(|p| free(p))
        .unwrap_or(first)
}

/// Ausgabeordner je Element (§2.6).
fn output_dir(
    recipe: &Recipe,
    info: &ItemInfo,
    new_folders: &mut HashMap<PathBuf, PathBuf>,
) -> PathBuf {
    let t = &recipe.output.target;
    let rel = if t.keep_structure {
        info.rel_dir()
    } else {
        PathBuf::new()
    };
    match t.kind {
        TargetKind::SameAsSource => info.dir(),
        TargetKind::Folder => t.path.clone().unwrap_or_else(|| info.dir()).join(rel),
        TargetKind::NewFolder => {
            let base = t
                .path
                .clone()
                .unwrap_or_else(|| info.root.clone().unwrap_or_else(|| info.dir()));
            let folder = new_folders
                .entry(key(&base).into())
                .or_insert_with(|| unique_folder(&base, &t.folder_name))
                .clone();
            folder.join(rel)
        }
    }
}

fn numbered(dir: &Path, stem: &str, ext: &str, k: usize) -> PathBuf {
    dir.join(if ext.is_empty() {
        format!("{stem} ({k})")
    } else {
        format!("{stem} ({k}).{ext}")
    })
}

/// Plant Ziele für alle Elemente (Schritte 1–7 von §4.4, Ausgabeordner, Konflikte). Schreibt nichts.
pub fn plan(
    recipe: &Recipe,
    items: &[ItemInfo],
    file_only: bool,
    opts: &PlanOptions,
) -> Result<Plan> {
    let now = opts.now.unwrap_or_else(now_local);
    let fmt = OutputFormat::parse(&recipe.output.format).ok_or_else(|| {
        BatchError::Invalid(fp_i18n::tr!(
            "batch-issue-unknown-format",
            format = recipe.output.format.clone()
        ))
    })?;
    if recipe.rename.mode == RenameMode::Regex {
        compile_regex(&recipe.rename.regex)?;
    }
    let conflict = recipe.output.conflict;
    let mut new_folders: HashMap<PathBuf, PathBuf> = HashMap::new();
    let mut dir_cache: HashMap<PathBuf, HashSet<String>> = HashMap::new();
    let mut claimed: HashMap<String, usize> = HashMap::new();
    let mut folder_counts: HashMap<String, usize> = HashMap::new();
    let mut dirs: Vec<PathBuf> = Vec::new();
    let mut seen_dirs: HashSet<String> = HashSet::new();
    let mut plan = Plan {
        file_only,
        ..Plan::default()
    };
    for (index, info) in items.iter().enumerate() {
        let folder_index = {
            let c = folder_counts.entry(key(&info.dir())).or_insert(0);
            *c += 1;
            *c - 1
        };
        let (resolved, fallback) = if file_only {
            (OutputFormat::Same, None)
        } else {
            resolve_format(fmt, &info.format)
        };
        let out_ext = target_extension(&recipe.rename, resolved, &info.file_name(), file_only);
        let ctx = NameContext {
            info,
            list_index: index,
            folder_index,
            now,
            out_ext: out_ext.clone(),
            out_format: if file_only {
                split_extension(&info.file_name()).1.to_lowercase()
            } else {
                resolved.name().to_string()
            },
        };
        let name = build_name(&recipe.rename, &ctx)?;
        let mut hints = name.hints.clone();
        if fallback.is_some() {
            hints.push(NameHint::FormatFallback);
        }
        let dir = output_dir(recipe, info, &mut new_folders);
        if seen_dirs.insert(key(&dir)) {
            dirs.push(dir.clone());
        }
        let mut target = dir.join(&name.file_name);
        let same_as_source = key(&target) == key(&info.path);
        let earlier = claimed.get(&key(&target)).copied();
        let on_disk = !same_as_source && exists_in(&mut dir_cache, &target);
        let mut action = PlanAction::Write;
        let mut alt_target = None;
        let mut conflict_with = None;
        let free_name = |claimed: &HashMap<String, usize>,
                         cache: &mut HashMap<PathBuf, HashSet<String>>|
         -> PathBuf {
            (2..)
                .map(|k| numbered(&dir, &name.stem, &out_ext, k))
                .find(|p| !claimed.contains_key(&key(p)) && !exists_in(cache, p))
                .unwrap_or_else(|| dir.join(&name.file_name))
        };
        if let Some(j) = earlier {
            // gleicher Zielname im Lauf: nie die Ausgabe eines früheren Elements überschreiben
            conflict_with = Some(j);
            hints.push(NameHint::Conflict);
            match conflict {
                Conflict::Skip => action = PlanAction::Skip,
                Conflict::Ask => {
                    action = PlanAction::Ask;
                    alt_target = Some(free_name(&claimed, &mut dir_cache));
                }
                Conflict::Rename | Conflict::Overwrite => {
                    target = free_name(&claimed, &mut dir_cache)
                }
            }
        } else if same_as_source {
            match conflict {
                Conflict::Overwrite => {
                    action = PlanAction::ReplaceOriginal;
                    hints.push(NameHint::ReplaceOriginal);
                    if !recipe.output.backup.enabled {
                        hints.push(NameHint::BackupForced);
                    }
                }
                Conflict::Skip => action = PlanAction::Skip,
                Conflict::Ask => {
                    action = PlanAction::Ask;
                    alt_target = Some(free_name(&claimed, &mut dir_cache));
                    hints.push(NameHint::Conflict);
                }
                Conflict::Rename => {
                    target = free_name(&claimed, &mut dir_cache);
                    hints.push(NameHint::Conflict);
                }
            }
        } else if on_disk {
            hints.push(NameHint::Conflict);
            match conflict {
                Conflict::Overwrite => {
                    action = PlanAction::Overwrite;
                    hints.push(NameHint::Overwrite);
                }
                Conflict::Skip => action = PlanAction::Skip,
                Conflict::Ask => {
                    action = PlanAction::Ask;
                    alt_target = Some(free_name(&claimed, &mut dir_cache));
                }
                Conflict::Rename => target = free_name(&claimed, &mut dir_cache),
            }
        }
        if action == PlanAction::Skip {
            hints.push(NameHint::Skipped);
        } else {
            claimed.insert(key(&target), index);
            if let Some(a) = &alt_target {
                claimed.insert(key(a), index);
            }
        }
        if hints.contains(&NameHint::Conflict) {
            plan.conflicts += 1;
        }
        if target.to_string_lossy().chars().count() > MAX_PATH_LEN {
            hints.push(NameHint::LongPath);
        }
        plan.entries.push(PlanEntry {
            index,
            source: info.path.clone(),
            target,
            alt_target,
            action,
            format: if file_only {
                "same".into()
            } else {
                resolved.name().into()
            },
            hints,
            conflict_with,
        });
    }
    plan.output_dirs = dirs;
    Ok(plan)
}

// ------------------------------------------------------------------ Output size (BA-105)

/// Does the name depend on the image size (`size` part, `{w}`/`{h}` in the regex replacement)?
pub fn uses_size(spec: &RenameSpec) -> bool {
    match spec.mode {
        RenameMode::Builder => spec
            .parts
            .iter()
            .any(|p| matches!(p, NamePart::Size { .. })),
        RenameMode::Regex => {
            let r = &spec.regex.replace;
            r.contains("{w}") || r.contains("{h}")
        }
    }
}

/// Name of item `index` for the **output** size `size` (after `resize`/`crop` …), built exactly
/// like the plan does. `None` if the name does not use the size or the size did not change.
pub fn name_for_size(
    recipe: &Recipe,
    items: &[ItemInfo],
    index: usize,
    now: NaiveDateTime,
    size: (u32, u32),
) -> Result<Option<NameResult>> {
    let Some(src) = items.get(index) else {
        return Ok(None);
    };
    if !uses_size(&recipe.rename) || (src.width, src.height) == size {
        return Ok(None);
    }
    let fmt = OutputFormat::parse(&recipe.output.format).unwrap_or(OutputFormat::Same);
    let (resolved, _) = resolve_format(fmt, &src.format);
    let dir = key(&src.dir());
    let folder_index = items[..index]
        .iter()
        .filter(|i| key(&i.dir()) == dir)
        .count();
    let mut info = src.clone();
    (info.width, info.height) = size;
    let ctx = NameContext {
        info: &info,
        list_index: index,
        folder_index,
        now,
        out_ext: target_extension(&recipe.rename, resolved, &src.file_name(), false),
        out_format: resolved.name().to_string(),
    };
    build_name(&recipe.rename, &ctx).map(Some)
}

/// Target names claimed during a run (list index per name). Lets a worker re-resolve name
/// conflicts when its name changes after processing (size token, BA-105).
#[derive(Debug, Default)]
pub struct Claims {
    map: HashMap<String, usize>,
}

/// New target of an entry after renaming at run time.
#[derive(Clone, Debug, PartialEq)]
pub struct Retarget {
    pub target: PathBuf,
    pub alt_target: Option<PathBuf>,
    pub action: PlanAction,
}

impl Claims {
    /// All targets (and fallback targets) the plan reserved.
    pub fn from_plan(plan: &Plan) -> Claims {
        let mut map = HashMap::new();
        for e in plan.entries.iter().filter(|e| e.action != PlanAction::Skip) {
            map.insert(key(&e.target), e.index);
            if let Some(a) = &e.alt_target {
                map.insert(key(a), e.index);
            }
        }
        Claims { map }
    }

    /// Moves entry `e` to the new name `name` in the same folder and resolves conflicts with the
    /// recipe's rule, like the plan: a name another item of this run holds is never overwritten
    /// (`name (2).ext` …, or skip/ask); an existing file follows the rule; the source itself
    /// is replaced only with "overwrite".
    pub fn retarget(&mut self, recipe: &Recipe, e: &PlanEntry, name: &NameResult) -> Retarget {
        let index = e.index;
        self.map.retain(|_, v| *v != index);
        let dir = e.target.parent().map(Path::to_path_buf).unwrap_or_default();
        let ext = split_extension(&name.file_name).1;
        let ext = if name.file_name.len() > name.stem.len() {
            ext
        } else {
            String::new()
        };
        let mut target = dir.join(&name.file_name);
        let taken = |m: &HashMap<String, usize>, p: &Path| {
            m.contains_key(&key(p)) || crate::output::long_path(p).exists()
        };
        let free = |m: &HashMap<String, usize>| {
            (2..10_000)
                .map(|k| numbered(&dir, &name.stem, &ext, k))
                .find(|p| !taken(m, p))
                .unwrap_or_else(|| dir.join(&name.file_name))
        };
        let in_run = self.map.contains_key(&key(&target));
        let same_as_source = key(&target) == key(&e.source);
        let on_disk = !same_as_source && crate::output::long_path(&target).exists();
        let mut action = PlanAction::Write;
        let mut alt_target = None;
        let conflict = recipe.output.conflict;
        if in_run || same_as_source || on_disk {
            match conflict {
                Conflict::Skip => action = PlanAction::Skip,
                Conflict::Ask => {
                    action = PlanAction::Ask;
                    alt_target = Some(free(&self.map));
                }
                Conflict::Overwrite if !in_run => {
                    action = if same_as_source {
                        PlanAction::ReplaceOriginal
                    } else {
                        PlanAction::Overwrite
                    }
                }
                Conflict::Rename | Conflict::Overwrite => target = free(&self.map),
            }
        }
        if action != PlanAction::Skip {
            self.map.insert(key(&target), index);
            if let Some(a) = &alt_target {
                self.map.insert(key(a), index);
            }
        }
        Retarget {
            target,
            alt_target,
            action,
        }
    }
}
