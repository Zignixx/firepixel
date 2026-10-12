//! `firepixel-cli language …` with English names (AGENTEN_REGELN §1c), old German names as hidden
//! aliases, and the migration `Sprachen` → `Languages` end to end. `APPDATA`/`LOCALAPPDATA` always
//! point to a test folder – the real user profile is never touched.

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use std::path::{Path, PathBuf};

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fp-te1-cli-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("temp dir");
    d
}

/// CLI with a private profile under `profile`.
fn cli(profile: &Path) -> Command {
    let mut c = Command::cargo_bin("firepixel-cli").expect("exe");
    c.env("FP_NO_SHELL_OPEN", "1").env("FIREPIXEL_LANG", "en").env("APPDATA", profile).env("LOCALAPPDATA", profile).env_remove("FP_MIGRATE");
    c
}

fn json(out: &[u8]) -> Value {
    serde_json::from_slice(out).unwrap_or_else(|e| panic!("no JSON ({e}): {}", String::from_utf8_lossy(out)))
}

fn old_profile(profile: &Path) -> PathBuf {
    let old = profile.join("Firepixel").join("Sprachen").join("xx");
    std::fs::create_dir_all(&old).expect("old dir");
    std::fs::write(old.join("allgemein.ftl"), "language-name = Testsprache TE1\n").expect("ftl");
    old
}

#[test]
fn english_names_and_hidden_aliases() {
    let profile = tmp("names");
    let target = profile.join("out");
    cli(&profile).args(["language", "template", "fr"]).arg(&target).assert().success();
    cli(&profile).args(["language", "template", "fr", "--overwrite", "--from", "en"]).arg(&target).assert().success();
    cli(&profile).args(["language", "check", "--short"]).arg(target.join("fr")).assert().success();
    let _ = cli(&profile).args(["language", "check", "--builtin", "--strict"]).assert();
    cli(&profile).args(["language", "list"]).assert().success();
    // old German names still work
    cli(&profile).args(["sprache", "pruefen", "--eingebaut"]).assert().success();
    cli(&profile).args(["sprache", "vorlage", "fr", "--ueberschreiben", "--von", "en"]).arg(&target).assert().success();
    cli(&profile).args(["sprache", "liste"]).assert().success();
    // … but are not shown in the help
    let help = cli(&profile).args(["language", "--help"]).assert().success().get_output().stdout.clone();
    let help = String::from_utf8_lossy(&help);
    assert!(help.contains("check") && help.contains("template") && help.contains("list"), "{help}");
    assert!(!help.contains("pruefen") && !help.contains("vorlage") && !help.contains("liste"), "{help}");
    let help = cli(&profile).args(["language", "check", "--help"]).assert().success().get_output().stdout.clone();
    let help = String::from_utf8_lossy(&help);
    assert!(help.contains("--builtin") && help.contains("--short") && help.contains("--strict"), "{help}");
    assert!(!help.contains("eingebaut") && !help.contains("Sprachen"), "{help}");
    let root = cli(&profile).arg("--help").assert().success().get_output().stdout.clone();
    assert!(String::from_utf8_lossy(&root).contains("language"));
    let _ = std::fs::remove_dir_all(&profile);
}

/// Test runs (`CARGO_MANIFEST_DIR`, `FP_NO_SHELL_OPEN`) do not migrate: the old folder is read.
#[test]
fn old_folder_is_read_without_migration() {
    let profile = tmp("fallback");
    let old = old_profile(&profile);
    let out = cli(&profile).args(["--json", "language", "list"]).assert().success().get_output().stdout.clone();
    let v = json(&out);
    assert!(v["languages"].as_array().is_some_and(|l| l.iter().any(|x| x["code"] == "xx")), "{v}");
    let folders = v["folders"].to_string();
    assert!(folders.contains("Sprachen"), "fallback folder listed: {folders}");
    assert!(old.join("allgemein.ftl").is_file(), "untouched");
    assert!(!profile.join("Firepixel").join("Languages").exists());
    let _ = std::fs::remove_dir_all(&profile);
}

/// With the switch on (as when the user starts the program): `Sprachen` → `Languages` once.
#[test]
fn migration_end_to_end() {
    let profile = tmp("migrate");
    old_profile(&profile);
    let out = cli(&profile).env("FP_MIGRATE", "1").args(["--json", "language", "list"]).assert().success().get_output().stdout.clone();
    let v = json(&out);
    let base = profile.join("Firepixel");
    assert!(base.join("Languages").join("xx").join("allgemein.ftl").is_file(), "moved");
    assert!(!base.join("Sprachen").exists(), "old folder gone");
    assert!(v["languages"].as_array().is_some_and(|l| l.iter().any(|x| x["code"] == "xx")), "{v}");
    assert!(v["folders"].to_string().contains("Languages"), "{v}");
    // second start: nothing more to do, the language is still there
    let out = cli(&profile).env("FP_MIGRATE", "1").args(["--json", "language", "list"]).assert().success().get_output().stdout.clone();
    assert!(json(&out)["languages"].as_array().is_some_and(|l| l.iter().any(|x| x["code"] == "xx")));
    // new folder exists already: old one is left alone but still read
    let old = old_profile(&profile);
    std::fs::write(old.join("allgemein.ftl"), "language-name = Alt TE1\nte1-only-old = 1\n").expect("ftl");
    let out = cli(&profile).env("FP_MIGRATE", "1").args(["--json", "language", "list"]).assert().success().get_output().stdout.clone();
    let v = json(&out);
    assert!(old.join("allgemein.ftl").is_file(), "no second migration");
    assert!(v["languages"].as_array().is_some_and(|l| l.iter().any(|x| x["code"] == "xx")), "{v}");
    let _ = std::fs::remove_dir_all(&profile);
}

#[test]
fn json_keys_are_english() {
    let profile = tmp("keys");
    let out = cli(&profile).args(["--json", "language", "check", "--builtin"]).assert().get_output().stdout.clone();
    let v = json(&out);
    for k in ["ok", "errors", "warnings", "languages", "counts", "problems"] {
        assert!(v.get(k).is_some(), "{k} missing: {v}");
    }
    cli(&profile).args(["language", "nonsense"]).assert().failure().stderr(predicate::str::is_empty().not());
    let _ = std::fs::remove_dir_all(&profile);
}
