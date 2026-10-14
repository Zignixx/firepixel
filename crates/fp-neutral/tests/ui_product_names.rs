//! Sichtbare Texte ohne fremde Produktnamen (Nutzerwunsch 14.10.2026, F12): Die Werte der `.ftl`-Dateien
//! (alle Sprachen) und das Handbuch/AGENTS (DE+EN) nennen kein Bildbearbeitungsprodukt beim Namen. Interne
//! Kennungen (ftl-Schlüssel, Voreinstellungsnamen, Formatkennungen im Code) sind davon ausgenommen.

use std::path::{Path, PathBuf};

const BLOCKED: &[&str] = &["photoshop"];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn ftl_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            ftl_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "ftl") {
            out.push(p);
        }
    }
}

fn hits(text: &str) -> bool {
    let l = text.to_lowercase();
    BLOCKED.iter().any(|b| l.contains(b))
}

#[test]
fn ftl_values_have_no_product_names() {
    let mut files = Vec::new();
    ftl_files(&root().join("crates").join("fp-i18n").join("locales"), &mut files);
    assert!(!files.is_empty(), "keine ftl-Dateien gefunden");
    let mut bad = Vec::new();
    for f in &files {
        let text = std::fs::read_to_string(f).unwrap_or_default();
        for (n, line) in text.lines().enumerate() {
            if line.trim_start().starts_with('#') {
                continue;
            }
            // Wert hinter `schlüssel =` bzw. Fortsetzungszeile
            let value = match line.split_once('=') {
                Some((k, v)) if !k.starts_with(' ') => v,
                _ => line,
            };
            if hits(value) {
                bad.push(format!("{}:{}: {}", f.display(), n + 1, line.trim()));
            }
        }
    }
    assert!(bad.is_empty(), "Produktnamen in sichtbaren Texten:\n{}", bad.join("\n"));
}

#[test]
fn manual_and_agents_have_no_product_names() {
    let mut bad = Vec::new();
    for name in ["MANUAL.md", "MANUAL.en.md", "AGENTS.md", "AGENTS.en.md"] {
        let p = root().join("docs").join(name);
        let text = std::fs::read_to_string(&p).unwrap_or_default();
        for (n, line) in text.lines().enumerate() {
            if hits(line) {
                bad.push(format!("{name}:{}: {}", n + 1, line.trim()));
            }
        }
    }
    assert!(bad.is_empty(), "Produktnamen im Handbuch:\n{}", bad.join("\n"));
}
