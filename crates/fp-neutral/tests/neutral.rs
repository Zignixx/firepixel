//! Dauerhafte Neutralitätsprüfung des ganzen Projekts (`cargo test -p fp-neutral`).
//! Meldet jede Fundstelle als `Datei:Zeile` bzw. `Datei (Name)`. Ausnahmen: keine.

use std::path::{Path, PathBuf};

fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

#[test]
fn projekt_ist_neutral() {
    let root = std::fs::canonicalize(project_root()).expect("Projektordner");
    let findings = fp_neutral::scan(&root);
    let list: Vec<String> = findings.iter().map(|f| f.to_string()).collect();
    assert!(
        findings.is_empty(),
        "{} Fundstelle(n) mit dem Namen des Originals bzw. seines Herstellers (docs\\AGENTEN_REGELN.md §1a):\n{}",
        list.len(),
        list.join("\n")
    );
}

/// Gegenprobe ohne das Wort im Quelltext: der Dateiname der Haupt-EXE in der Installationskopie
/// (`FP_CORPUS_DIR` bzw. `..\korpus`, größte EXE) muss erkannt werden – auch in UTF-16.
#[test]
fn erkennt_den_namen_in_der_installationskopie() {
    let corpus = std::env::var_os("FP_CORPUS_DIR").map(PathBuf::from).unwrap_or_else(|| project_root().join("..").join("korpus"));
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
    assert!(!fp_neutral::find_in_bytes(name.as_bytes()).is_empty(), "Name der Haupt-EXE nicht erkannt");
    assert!(!fp_neutral::find_in_bytes(name.to_uppercase().as_bytes()).is_empty());
    let u16: Vec<u8> = name.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
    assert!(!fp_neutral::find_in_bytes(&u16).is_empty(), "UTF-16 nicht erkannt");
}
