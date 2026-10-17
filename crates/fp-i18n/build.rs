//! Bettet alle `locales/<sprache>/*.ftl` ein. Neue Bereichsdateien brauchen keine Codeänderung:
//! Datei anlegen, neu bauen. Erzeugt `OUT_DIR/embedded.rs` mit `EMBEDDED: &[(&str, &str, &str)]`
//! = (Sprachcode, Dateiname, Inhalt).

use std::fmt::Write as _;
use std::path::PathBuf;

fn main() {
    let root =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default()).join("locales");
    println!("cargo:rerun-if-changed={}", root.display());
    let mut entries: Vec<(String, String, PathBuf)> = Vec::new();
    if let Ok(langs) = std::fs::read_dir(&root) {
        for lang in langs.flatten() {
            let lp = lang.path();
            if !lp.is_dir() {
                continue;
            }
            println!("cargo:rerun-if-changed={}", lp.display());
            let code = lang.file_name().to_string_lossy().to_string();
            if let Ok(files) = std::fs::read_dir(&lp) {
                for f in files.flatten() {
                    let fp = f.path();
                    if fp.extension().and_then(|e| e.to_str()) == Some("ftl") {
                        println!("cargo:rerun-if-changed={}", fp.display());
                        entries.push((
                            code.clone(),
                            f.file_name().to_string_lossy().to_string(),
                            fp,
                        ));
                    }
                }
            }
        }
    }
    entries.sort();
    let mut out = String::from("pub(crate) static EMBEDDED: &[(&str, &str, &str)] = &[\n");
    for (code, name, path) in &entries {
        let _ = writeln!(
            out,
            "    ({code:?}, {name:?}, include_str!({:?})),",
            path.display().to_string()
        );
    }
    out.push_str("];\n");
    let dest = PathBuf::from(std::env::var("OUT_DIR").unwrap_or_default()).join("embedded.rs");
    if let Err(e) = std::fs::write(&dest, out) {
        println!("cargo:warning=embedded.rs nicht geschrieben: {e}");
    }
}
