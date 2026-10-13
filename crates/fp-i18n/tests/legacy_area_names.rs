//! Bereichsdateien mit den deutschen Namen bis 13.10.2026 (`menue.ftl`, `allgemein.ftl` …) in
//! Sprachpaketen der Nutzer: werden geladen, und `check` ordnet sie dem neuen Bereich zu (TE2-A).

use fp_i18n::check::{self, Kind, LEGACY_AREA_NAMES};
use std::path::PathBuf;

fn builtin(code: &str, file: &str) -> &'static str {
    fp_i18n::embedded_files().iter().find(|(c, f, _)| *c == code && *f == file).map(|(_, _, t)| *t).expect("builtin file")
}

fn temp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fp-i18n-te2a-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("temp dir");
    d
}

#[test]
fn legacy_table_matches_builtin_files() {
    let areas = check::builtin_file_names();
    for (old, new) in LEGACY_AREA_NAMES {
        assert!(areas.contains(&format!("{new}.ftl")), "{new}.ftl is built in");
        assert!(!areas.contains(&format!("{old}.ftl")), "{old}.ftl no longer built in");
        assert_eq!(check::area_file_name(&format!("{old}.ftl")), format!("{new}.ftl"));
        assert_eq!(check::legacy_area_file_name(&format!("{new}.ftl")), Some(format!("{old}.ftl")));
    }
    assert_eq!(check::area_file_name("Menue.FTL"), "menu.ftl");
    assert_eq!(check::area_file_name("menu.ftl"), "menu.ftl");
    assert_eq!(check::area_file_name("fr.ftl"), "fr.ftl");
    assert_eq!(check::legacy_area_file_name("mcp.ftl"), None);
    // 18 Bereiche bis 13.10.2026; neue Bereichsdateien (z. B. batch.ftl, BATCH-1) kommen dazu
    assert!(areas.len() >= 18, "{areas:?}");
}

#[test]
fn old_area_file_is_checked_against_its_new_area() {
    let dir = temp("file");
    let fr = dir.join("fr");
    std::fs::create_dir_all(&fr).expect("fr");
    let f = fr.join("menue.ftl");
    std::fs::write(&f, builtin("en", "menu.ftl")).expect("write");
    let r = check::check_path(&f).expect("check");
    assert_eq!(r.langs.len(), 1);
    assert_eq!(r.langs[0].code, "fr", "area file → language from the folder");
    assert_eq!(r.langs[0].missing, 0, "only the menu area is compared: {:?}", r.problems.iter().take(5).collect::<Vec<_>>());
    assert!(!r.problems.iter().any(|p| p.kind == Kind::Orphan), "no orphans");
    // ohne Zuordnung würde die Datei als Sprache „menue“ gegen alle Bereiche geprüft
    assert!(r.langs[0].percent > 99.9);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn folder_with_only_old_names_is_a_language_folder() {
    let dir = temp("folder");
    let fr = dir.join("fr");
    std::fs::create_dir_all(&fr).expect("fr");
    for (old, new) in LEGACY_AREA_NAMES {
        std::fs::write(fr.join(format!("{old}.ftl")), builtin("en", &format!("{new}.ftl"))).expect("write");
    }
    // Ordner `fr` direkt: eine Sprache „fr“ (nicht je Datei eine Sprache)
    let r = check::check_path(&fr).expect("check");
    assert_eq!(r.langs.len(), 1, "{:?}", r.langs.iter().map(|l| &l.code).collect::<Vec<_>>());
    assert_eq!(r.langs[0].code, "fr");
    // es fehlen nur die Bereiche, die nie umbenannt wurden
    // (alle eingebauten Bereiche ohne alten Namen, auch später neu hinzugekommene wie batch.ftl)
    let unrenamed: usize = check::builtin_file_names()
        .iter()
        .filter(|f| !LEGACY_AREA_NAMES.iter().any(|(_, new)| **f == format!("{new}.ftl")))
        .map(|f| check::LangData::builtin("de").only_file(f).keys.len())
        .sum();
    assert_eq!(r.langs[0].missing, unrenamed);
    assert!(!r.problems.iter().any(|p| matches!(p.kind, Kind::Orphan | Kind::Duplicate | Kind::Syntax)));
    // Elternordner (`Languages\`) findet die Sprache ebenso
    let r = check::check_path(&dir).expect("check parent");
    assert_eq!(r.langs.iter().map(|l| l.code.as_str()).collect::<Vec<_>>(), ["fr"]);

    // Laden ist namensunabhängig
    let loaded = fp_i18n::load_external_dir(&dir);
    assert!(loaded.iter().any(|c| c == "fr"), "{loaded:?}");
    let label = fp_i18n::with_language("fr", || fp_i18n::t("btn-cancel"));
    assert_eq!(label, fp_i18n::with_language("en", || fp_i18n::t("btn-cancel")));
    let _ = std::fs::remove_dir_all(&dir);
}
