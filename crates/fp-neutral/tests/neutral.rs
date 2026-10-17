//! Dauerhafte Neutralitätsprüfung des ganzen Projekts (`cargo test -p fp-neutral`).
//! Meldet jede Fundstelle als `Datei:Zeile` bzw. `Datei (Name)`.
//!
//! Geprüft werden (docs\AGENTEN_REGELN.md §1a):
//! * alle Dateien und Namen unter `firepixel\` außer `target*` und `.git` – Quelltext, Daten,
//!   `STATUS.md`, `docs\*.md`, `docs\berichte\*.md`, `docs\funktionstest\*` (Bytes, UTF-8/UTF-16);
//! * die Bilder unter `docs\screenshots\`: Dateiname und Bytes wie oben, dazu der sichtbare Text
//!   per Windows-Texterkennung (`tests\ocr_words.ps1`), geprüft mit `fp_neutral::find_in_bytes`.
//!
//! Bewusst erlaubte Treffer stehen mit Begründung in [`ALLOWED`]; Bilder sind dort nicht zulässig.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Bewusst erlaubte Treffer: (Pfad relativ zu `firepixel\` mit `/`, Begründung).
/// Jeder Eintrag braucht eine Begründung und muss noch gebraucht werden. Keine Bilder, keine
/// Systemschriften – Bilder mit solchen Namen werden neu erzeugt oder entfernt.
const ALLOWED: &[(&str, &str)] = &[];

fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn slash(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

#[test]
fn projekt_ist_neutral() {
    for (path, why) in ALLOWED {
        assert!(why.trim().len() >= 10, "ALLOWED {path}: Begründung fehlt");
        assert!(
            !path.starts_with("docs/screenshots"),
            "ALLOWED {path}: Bilder müssen sauber sein"
        );
    }
    let root = std::fs::canonicalize(project_root()).expect("Projektordner");
    let findings = fp_neutral::scan(&root);
    let mut used = BTreeSet::new();
    let list: Vec<String> = findings
        .iter()
        .filter(
            |f| match ALLOWED.iter().find(|(p, _)| slash(&f.path) == *p) {
                Some((p, _)) => {
                    used.insert(*p);
                    false
                }
                None => true,
            },
        )
        .map(|f| f.to_string())
        .collect();
    assert!(
        list.is_empty(),
        "{} Fundstelle(n) mit dem Namen des Originals bzw. seines Herstellers (docs\\AGENTEN_REGELN.md §1a):\n{}",
        list.len(),
        list.join("\n")
    );
    let stale: Vec<&str> = ALLOWED
        .iter()
        .map(|(p, _)| *p)
        .filter(|p| !used.contains(p))
        .collect();
    assert!(
        stale.is_empty(),
        "ALLOWED-Einträge ohne Treffer (entfernen): {stale:?}"
    );
}

/// Die Prüfung muss die Dokumente und Bilder wirklich erfassen (Vorfall 12.10.: ein Treffer in der
/// Doku blieb unbemerkt).
#[test]
fn scan_covers_documents_and_images() {
    let root = std::fs::canonicalize(project_root()).expect("Projektordner");
    let (_, files) = fp_neutral::scan_with_files(&root);
    let files: Vec<String> = files.iter().map(|p| slash(p)).collect();
    let has = |pred: &dyn Fn(&str) -> bool| files.iter().any(|f| pred(f));
    assert!(has(&|f| f == "README.md"), "README.md nicht geprüft");
    assert!(
        has(&|f| f.starts_with("docs/") && f.ends_with(".md") && f.matches('/').count() == 1),
        "docs/*.md nicht geprüft"
    );
    for dir in ["docs/berichte/", "docs/funktionstest/"] {
        if root.join(dir).is_dir() {
            assert!(has(&|f| f.starts_with(dir)), "{dir} nicht geprüft");
        }
    }
    if root.join("docs/screenshots").is_dir() {
        assert!(
            has(&|f| f.starts_with("docs/screenshots/") && f.ends_with(".png")),
            "docs/screenshots nicht geprüft"
        );
    }
}

fn pngs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for p in rd.flatten().map(|e| e.path()) {
        if p.is_dir() {
            pngs(&p, out);
        } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("png")) {
            out.push(p);
        }
    }
}

/// Sichtbarer Text in den Bildern unter `docs\screenshots` (Texterkennung, siehe Modulkopf).
/// Saubere Bilder merkt sich ein Zwischenspeicher im Build-Ordner (Schlüssel: FNV-1a + Länge),
/// so dass nur neue oder geänderte Bilder erkannt werden. Ohne Windows-Texterkennung übersprungen.
#[test]
fn screenshots_show_no_blocked_names() {
    // ohne das Präfix \\?\ von canonicalize, das die Windows-Bildschnittstelle nicht versteht
    let root = std::fs::canonicalize(project_root()).expect("Projektordner");
    let root = PathBuf::from(
        root.to_string_lossy()
            .trim_start_matches(r"\\?\")
            .to_string(),
    );
    let mut all = Vec::new();
    pngs(&root.join("docs").join("screenshots"), &mut all);
    all.sort();
    let tmp = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    let cache_file = tmp.join("fp_neutral_ocr_clean.txt");
    let mut clean: BTreeSet<String> = std::fs::read_to_string(&cache_file)
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect();
    let key = |p: &Path| {
        std::fs::read(p)
            .map(|d| format!("{:016x}-{}", fp_neutral::fnv64(&d), d.len()))
            .unwrap_or_default()
    };
    let todo: Vec<(PathBuf, String)> = all
        .iter()
        .map(|p| (p.clone(), key(p)))
        .filter(|(_, k)| !clean.contains(k))
        .collect();
    if todo.is_empty() {
        return;
    }
    let list = tmp.join("fp_neutral_ocr_list.txt");
    let out = tmp.join("fp_neutral_ocr_out.txt");
    std::fs::write(
        &list,
        todo.iter()
            .map(|(p, _)| p.to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join("\n"),
    )
    .expect("Liste");
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("ocr_words.ps1");
    let status = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(&script)
        .arg("-List")
        .arg(&list)
        .arg("-Out")
        .arg(&out)
        .status();
    match status {
        Ok(s) if s.success() => {}
        Ok(s) if s.code() == Some(3) => {
            eprintln!("übersprungen: Texterkennung auf diesem System nicht verfügbar");
            return;
        }
        Ok(s) => panic!("Texterkennung fehlgeschlagen ({s}): {}", script.display()),
        Err(e) => {
            eprintln!("übersprungen: powershell nicht startbar ({e})");
            return;
        }
    }
    let text = std::fs::read_to_string(&out).expect("Ergebnis der Texterkennung");
    // Pfad → Fundstellen (Wortrechteck); die Zeilentexte selbst werden nicht ausgegeben (§1a)
    let mut bad: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for row in text.lines() {
        let c: Vec<&str> = row.split('\t').collect();
        if c.len() < 7 {
            continue;
        }
        let hit_line = !fp_neutral::find_in_bytes(c[1].as_bytes()).is_empty();
        let hit_word = !fp_neutral::find_in_bytes(c[2].as_bytes()).is_empty();
        if c[3] == "-1" {
            bad.entry(c[0].to_string())
                .or_default()
                .push("nicht lesbar".into());
        } else if hit_line || hit_word {
            let what = if hit_word {
                format!("Wort bei x={} y={} ({}×{})", c[3], c[4], c[5], c[6])
            } else {
                format!("Zeile bei y={}", c[4])
            };
            bad.entry(c[0].to_string()).or_default().push(what);
        }
    }
    for (p, k) in &todo {
        if !bad.contains_key(&p.to_string_lossy().to_string()) {
            clean.insert(k.clone());
        }
    }
    let _ = std::fs::write(
        &cache_file,
        clean.iter().cloned().collect::<Vec<_>>().join("\n"),
    );
    let report: Vec<String> = bad
        .iter()
        .map(|(p, w)| {
            let rel = Path::new(p)
                .strip_prefix(&root)
                .map(slash)
                .unwrap_or_else(|_| p.clone());
            let mut w = w.clone();
            w.dedup();
            format!("{rel}: {}", w.join(", "))
        })
        .collect();
    assert!(
        report.is_empty(),
        "{} Bild(er) zeigen den Namen des Originals bzw. seines Herstellers (§1a; Bild neu erzeugen oder entfernen):\n{}",
        report.len(),
        report.join("\n")
    );
}

/// Gegenprobe ohne das Wort im Quelltext: der Dateiname der Haupt-EXE in der Installationskopie
/// (`FP_CORPUS_DIR` bzw. `..\korpus`, größte EXE) muss erkannt werden – auch in UTF-16.
#[test]
fn erkennt_den_namen_in_der_installationskopie() {
    let corpus = std::env::var_os("FP_CORPUS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| project_root().join("..").join("korpus"));
    let Ok(rd) = std::fs::read_dir(&corpus) else {
        eprintln!("übersprungen: kein Korpus unter {}", corpus.display());
        return;
    };
    let exe = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("exe")))
        .max_by_key(|p| std::fs::metadata(p).map(|m| m.len()).unwrap_or(0));
    let Some(exe) = exe else {
        eprintln!("übersprungen: keine EXE im Korpus");
        return;
    };
    let name = exe.file_stem().unwrap().to_string_lossy().to_string();
    assert!(
        !fp_neutral::find_in_bytes(name.as_bytes()).is_empty(),
        "Name der Haupt-EXE nicht erkannt"
    );
    assert!(!fp_neutral::find_in_bytes(name.to_uppercase().as_bytes()).is_empty());
    let u16: Vec<u8> = name.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
    assert!(
        !fp_neutral::find_in_bytes(&u16).is_empty(),
        "UTF-16 nicht erkannt"
    );
}
