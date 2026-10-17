//! Guard: no doubly UTF-8-encoded text in sources and translations (FIX-B, GT-1 13.10.2026).
//!
//! A file that was read as Windows-1252 and saved again as UTF-8 turns „ü“ into U+00C3 U+00BC
//! and „…“ into U+00E2 U+20AC U+00A6. That broke `bitmap.rs` (command labels no longer matched the `.ftl`
//! register). This test scans the text files under `crates\`, `apps\` and `tools\` (Rust, TOML,
//! Fluent, scripts, JSON, Markdown) plus the workspace `Cargo.toml` and reports `file:line` for
//! every typical double-encoding sequence. Files that are not valid UTF-8 are skipped (test data
//! in other encodings is fine); documents under `docs\` may quote the broken text and are not scanned.
//!
//! The patterns are written with `\u{…}` escapes so this file does not match itself.

use std::path::{Path, PathBuf};

const EXTS: &[&str] = &[
    "rs", "toml", "ftl", "jsf", "js", "py", "ps1", "json", "md", "txt", "tsv", "csv", "wgsl",
];

fn root() -> PathBuf {
    std::fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(".."))
        .expect("project root")
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        if p.is_dir() {
            if name.starts_with("target")
                || name.starts_with('.')
                || name == "node_modules"
                || name == "out"
            {
                continue;
            }
            walk(&p, out);
        } else if p
            .extension()
            .and_then(|x| x.to_str())
            .is_some_and(|x| EXTS.contains(&x))
        {
            out.push(p);
        }
    }
}

/// Second characters that follow U+00C3 when a UTF-8 lead byte 0xC3 was read as Windows-1252/Latin-1:
/// U+0080–U+00BF and the Windows-1252 replacements for 0x80–0x9F.
fn is_cont(c: char) -> bool {
    matches!(c, '\u{80}'..='\u{bf}')
        || matches!(
            c,
            '\u{20ac}'
                | '\u{201a}'
                | '\u{0192}'
                | '\u{201e}'
                | '\u{2026}'
                | '\u{2020}'
                | '\u{2021}'
                | '\u{02c6}'
                | '\u{2030}'
                | '\u{0160}'
                | '\u{2039}'
                | '\u{0152}'
                | '\u{017d}'
                | '\u{2018}'
                | '\u{2019}'
                | '\u{201c}'
                | '\u{201d}'
                | '\u{2022}'
                | '\u{2013}'
                | '\u{2014}'
                | '\u{02dc}'
                | '\u{2122}'
                | '\u{0161}'
                | '\u{203a}'
                | '\u{0153}'
                | '\u{017e}'
                | '\u{0178}'
        )
}

/// First doubly encoded sequence in a line, if any.
fn find_double(line: &str) -> Option<String> {
    let cs: Vec<char> = line.chars().collect();
    for i in 0..cs.len().saturating_sub(1) {
        let (a, b) = (cs[i], cs[i + 1]);
        // U+00C3 + continuation: ä ö ü ß Ä Ö Ü é …
        // U+00C2 + U+00A0–U+00BF: non-breaking space, °, §, ·, «, » …
        // U+00E2 U+20AC + x: – — „ “ ” ‘ ’ … • €; U+00E2 U+2020 + x: → ←; U+00EF U+00BB U+00BF: BOM
        let hit = (a == '\u{c3}' && is_cont(b))
            || (a == '\u{c2}' && matches!(b, '\u{a0}'..='\u{bf}'))
            || (a == '\u{e2}' && b == '\u{20ac}')
            || (a == '\u{e2}' && b == '\u{2020}')
            || (a == '\u{ef}' && b == '\u{bb}' && cs.get(i + 2) == Some(&'\u{bf}'));
        if hit {
            return Some(cs[i..(i + 3).min(cs.len())].iter().collect());
        }
    }
    None
}

#[test]
fn no_double_utf8_encoding_in_sources() {
    let root = root();
    let mut files = Vec::new();
    for group in ["crates", "apps", "tools"] {
        walk(&root.join(group), &mut files);
    }
    files.push(root.join("Cargo.toml"));
    files.sort();
    assert!(
        files.len() > 500,
        "Prüfung findet zu wenige Dateien: {}",
        files.len()
    );
    let mut hits = Vec::new();
    for f in &files {
        let Ok(bytes) = std::fs::read(f) else {
            continue;
        };
        let Ok(text) = String::from_utf8(bytes) else {
            continue;
        };
        let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
        for (n, line) in text.lines().enumerate() {
            if let Some(seq) = find_double(line) {
                let rel = f.strip_prefix(&root).unwrap_or(f).display().to_string();
                hits.push(format!(
                    "{rel}:{}: {seq:?} in {:?}",
                    n + 1,
                    line.chars().take(120).collect::<String>()
                ));
            }
        }
    }
    assert!(
        hits.is_empty(),
        "{} Zeilen doppelt UTF-8-kodiert (als UTF-8 neu speichern):\n{}",
        hits.len(),
        hits.join("\n")
    );
}

#[test]
fn detector_finds_double_encoding() {
    // „Füllen“, „…“, „–“, „°“ einmal falsch als Windows-1252 gelesen und wieder als UTF-8 gespeichert
    for good in [
        "Füllen",
        "Punktfolge […]",
        "a – b",
        "90°",
        "Größe",
        "Übersicht",
        "„Text“",
    ] {
        let bytes = good.as_bytes();
        let double: String = bytes.iter().map(|&b| cp1252(b)).collect();
        assert!(
            find_double(&double).is_some(),
            "nicht erkannt: {double:?} (aus {good:?})"
        );
        assert!(find_double(good).is_none(), "falscher Treffer: {good:?}");
    }
    assert!(find_double("Ätzen Öl ß é ç").is_none());
}

/// Windows-1252 decoding of one byte (as an editor with the wrong encoding would show it).
fn cp1252(b: u8) -> char {
    const HIGH: [char; 32] = [
        '\u{20ac}', '\u{81}', '\u{201a}', '\u{0192}', '\u{201e}', '\u{2026}', '\u{2020}',
        '\u{2021}', '\u{02c6}', '\u{2030}', '\u{0160}', '\u{2039}', '\u{0152}', '\u{8d}',
        '\u{017d}', '\u{8f}', '\u{90}', '\u{2018}', '\u{2019}', '\u{201c}', '\u{201d}', '\u{2022}',
        '\u{2013}', '\u{2014}', '\u{02dc}', '\u{2122}', '\u{0161}', '\u{203a}', '\u{0153}',
        '\u{9d}', '\u{017e}', '\u{0178}',
    ];
    match b {
        0x80..=0x9f => HIGH[(b - 0x80) as usize],
        _ => b as char,
    }
}
