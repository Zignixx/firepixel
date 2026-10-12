//! Tests never write into the real user profile (EB-3, AGENTEN_REGELN §4a).
//!
//! 1. Every profile folder of a test run lies in the test profile of the process.
//! 2. The guard refuses writes below the real `%APPDATA%\Firepixel` / `%LOCALAPPDATA%\Firepixel`
//!    (with a panic, before anything is written).
//! 3. A representative run of the commands that write profile data (saved commands, shortcut
//!    sets, extensions, preferences, model folder) leaves the real profile
//!    untouched: the real folders are only listed (read only) before and after.
//!
//! The static half (no direct profile paths in tests, child processes get the test profile) is
//! `crates\fp-neutral\tests\no_real_profile.rs`.

mod common;

use common::*;
use fp_i18n::profile;
use serde_json::json;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

fn real_dirs() -> Vec<PathBuf> {
    [profile::real_roaming_dir(), profile::real_local_dir()].into_iter().flatten().collect()
}

/// Read-only listing: path → (modified, size) of everything below `dirs`.
fn snapshot(dirs: &[PathBuf]) -> BTreeMap<PathBuf, (Option<SystemTime>, u64)> {
    fn walk(d: &Path, out: &mut BTreeMap<PathBuf, (Option<SystemTime>, u64)>) {
        let Ok(rd) = std::fs::read_dir(d) else { return };
        for e in rd.flatten() {
            let p = e.path();
            let md = e.metadata().ok();
            out.insert(p.clone(), (md.as_ref().and_then(|m| m.modified().ok()), md.as_ref().map(|m| m.len()).unwrap_or(0)));
            if md.is_some_and(|m| m.is_dir()) {
                walk(&p, out);
            }
        }
    }
    let mut out = BTreeMap::new();
    for d in dirs {
        if let Ok(md) = std::fs::metadata(d) {
            out.insert(d.clone(), (md.modified().ok(), 0));
        }
        walk(d, &mut out);
    }
    out
}

#[test]
fn profile_folders_point_to_the_test_profile() {
    let _ = doc_session(10.0, 10.0);
    assert!(profile::is_test_run(), "cargo test binary must count as test run");
    let root = profile::use_test_profile();
    assert!(!profile::is_real(&root), "{root:?}");
    let folders: Vec<(&str, Option<PathBuf>)> = vec![
        ("roaming", profile::roaming_dir()),
        ("local", profile::local_dir()),
        ("preferences", fp_engine::Preferences::default_path()),
        ("commands", Some(fp_engine::commands::misc::commands_dir())),
        ("extensions", Some(fp_engine::commands::misc::extensions_dir())),
        ("shortcuts", Some(fp_engine::shortcuts::custom_dir())),
        ("config", fp_i18n::user_config_dir()),
        ("languages", fp_i18n::user_language_dir()),
        ("model download", Some(fp_ai::models::download_folder())),
    ];
    for (what, p) in folders {
        let p = p.unwrap_or_else(|| panic!("{what}: no folder"));
        assert!(p.starts_with(&root), "{what}: {p:?} not in the test profile {root:?}");
        assert!(!profile::is_real(&p), "{what}: {p:?}");
    }
}

#[test]
fn guard_refuses_the_real_profile() {
    let _ = doc_session(10.0, 10.0);
    let Some(roaming) = profile::real_roaming_dir() else { return };
    let prefs = roaming.join("preferences.json");
    assert!(profile::is_real(&prefs));
    assert!(profile::check_write(&prefs).is_err());
    assert!(profile::check_write(&roaming.join("Commands").join("x.jsf")).is_err());
    let before = std::fs::metadata(&prefs).and_then(|m| m.modified()).ok();
    let r = std::panic::catch_unwind(|| fp_engine::Preferences::new().save(&prefs));
    assert!(r.is_err(), "saving preferences into the real profile must panic");
    assert_eq!(std::fs::metadata(&prefs).and_then(|m| m.modified()).ok(), before, "nothing written");
    if let Some(local) = profile::real_local_dir() {
        assert!(profile::check_write(&local.join("mcp-http.json")).is_err());
        assert!(profile::check_write(&local.join("Models").join("x.safetensors")).is_err());
    }
    // the test profile itself and %TEMP% are fine
    assert!(profile::check_write(&profile::use_test_profile().join("x")).is_ok());
    assert!(profile::check_write(&std::env::temp_dir().join("fp-eb3-probe")).is_ok());
}

#[test]
fn commands_leave_the_real_profile_untouched() {
    let token = format!("eb3-probe-{}", std::process::id());
    let real = real_dirs();
    let before = snapshot(&real);
    let mut s = doc_session(80.0, 60.0);
    s.prefs_path = fp_engine::Preferences::default_path();
    // gespeicherte Befehle (all_commands schrieb früher Testbefehl.jsf ins echte Profil)
    ex(&mut s, "commands.manageSavedCommands", json!({"action": "save", "name": token, "code": "select.all"}));
    ex(&mut s, "commands.manageSavedCommands", json!({"action": "rename", "name": token, "newName": format!("{token}-2")}));
    let saved = fp_engine::commands::misc::commands_dir().join(format!("{token}-2.jsf"));
    assert!(saved.is_file(), "{saved:?}");
    // Tastaturkürzel-Set (phase5 legte es früher im echten Ordner Shortcuts an)
    ex(&mut s, "edit.keyboardShortcuts", json!({"duplicate": token}));
    assert!(fp_engine::shortcuts::custom_dir().join(format!("{token}.json")).is_file());
    // Erweiterung installieren
    let ext = std::env::temp_dir().join(format!("{token}.mxp"));
    std::fs::write(&ext, b"x").expect("extension");
    let mgr = fp_engine::CommandRegistry::with_defaults().iter().map(|c| c.info().name.clone()).find(|n| n.contains("anageExtensions")).expect("extension manager");
    ex(&mut s, &mgr, json!({"action": "install", "path": ext.to_string_lossy()}));
    assert!(fp_engine::commands::misc::extensions_dir().join(format!("{token}.mxp")).is_file());
    // Voreinstellungen und Modellordner
    ex(&mut s, "edit.preferences", json!({"key": "MaxUndos", "value": 37}));
    ex(&mut s, "ai.setModelFolder", json!({"path": ""}));
    s.save_prefs().expect("prefs");
    let prefs = fp_engine::Preferences::default_path().expect("prefs path");
    assert!(std::fs::read_to_string(&prefs).is_ok_and(|t| t.contains("37")), "{prefs:?}");
    // aufräumen im Testprofil (darf), dann vergleichen
    ex(&mut s, "commands.manageSavedCommands", json!({"action": "delete", "name": format!("{token}-2")}));
    ex(&mut s, "edit.keyboardShortcuts", json!({"deleteSet": token}));
    ex(&mut s, &mgr, json!({"action": "remove", "name": format!("{token}.mxp")}));
    let _ = std::fs::remove_file(&ext);

    let after = snapshot(&real);
    let new: Vec<&PathBuf> = after.keys().filter(|p| !before.contains_key(*p)).collect();
    let changed: Vec<&PathBuf> = after.iter().filter(|(p, v)| before.get(*p).is_some_and(|b| b != *v)).map(|(p, _)| p).collect();
    let gone: Vec<&PathBuf> = before.keys().filter(|p| !after.contains_key(*p)).collect();
    let ours: Vec<&&PathBuf> = new.iter().chain(changed.iter()).chain(gone.iter()).filter(|p| p.to_string_lossy().contains(&token) || p.file_name().is_some_and(|n| n == "preferences.json" || n == "Commands" || n == "Shortcuts" || n == "Extensions")).collect();
    if !(new.is_empty() && changed.is_empty() && gone.is_empty()) {
        eprintln!("real profile changed during the test (other processes?): new {new:?}, changed {changed:?}, gone {gone:?}");
    }
    assert!(ours.is_empty(), "the test wrote into the real profile: {ours:?}");
    if let Some(real_prefs) = profile::real_roaming_dir().map(|d| d.join("preferences.json")) {
        assert!(!std::fs::read_to_string(real_prefs).is_ok_and(|t| t.contains("\"MaxUndos\": 37")));
    }
}
