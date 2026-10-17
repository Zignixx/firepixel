//! Ressourcenordner im Testkorpus (`FP_CORPUS_DIR` bzw. `..\korpus`): Haupt-EXE und CMYK-Tabelle.
//! `cargo test -p fp-color --test install_corpus -- --ignored --nocapture`

use std::path::{Path, PathBuf};

#[test]
#[ignore]
fn corpus_exe_has_cmyk_table() {
    let dir = std::env::var_os("FP_CORPUS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../korpus"));
    let Ok(rd) = std::fs::read_dir(&dir) else {
        println!("kein Korpus unter {}", dir.display());
        return;
    };
    for p in rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("exe")))
    {
        let d = std::fs::read(&p).unwrap();
        let r = fp_color::install::find_resource(&d, "CMYK", 1001);
        println!(
            "{} ({} Byte): CMYK/1001 {:?}",
            p.file_name().unwrap().to_string_lossy(),
            d.len(),
            r.as_ref().map(|v| v.len())
        );
    }
    println!(
        "CMYK-Ressource: {:?}",
        fp_color::install::cmyk_resource_file(&dir)
    );
    let exe = fp_color::install::main_exe_in(&dir);
    println!("Haupt-EXE: {exe:?}");
    assert!(exe.is_some());
}
