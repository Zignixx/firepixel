//! Technical names always English (AGENTEN_REGELN §1c): language folder `Languages`, migration of
//! the old German `Sprachen` (`fp_i18n::paths`). Only temporary folders are used – never the real
//! user profile.

use fp_i18n::paths;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// The migration switch is process-wide: tests that change it run one after the other.
static SWITCH: Mutex<()> = Mutex::new(());

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fp-te1-i18n-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("temp dir");
    d
}

/// Old folder `Sprachen\fr\common.ftl` with a language of the user.
fn old_language_dir(base: &Path) -> PathBuf {
    let old = base.join(fp_i18n::LEGACY_LANGUAGE_DIR);
    std::fs::create_dir_all(old.join("fr")).expect("old dir");
    std::fs::write(
        old.join("fr").join("common.ftl"),
        "language-name = Français\nte1-probe = Bonjour\n",
    )
    .expect("ftl");
    old
}

#[test]
fn names_are_english() {
    assert_eq!(fp_i18n::LANGUAGE_DIR, "Languages");
    assert_eq!(fp_i18n::LEGACY_LANGUAGE_DIR, "Sprachen");
    let base = tmp("names");
    assert_eq!(
        fp_i18n::language_dir_in(&base),
        base.join("Languages"),
        "neither exists: new name"
    );
    // (the real user folder is deliberately not touched here)
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn migrates_old_folder_once() {
    let _g = SWITCH.lock().unwrap_or_else(|e| e.into_inner());
    let base = tmp("migrate");
    let old = old_language_dir(&base);
    paths::set_migration_enabled(true);
    let dir = fp_i18n::language_dir_in(&base);
    paths::set_migration_enabled(false);
    assert_eq!(dir, base.join("Languages"));
    assert!(!old.exists(), "old folder renamed");
    let text =
        std::fs::read_to_string(dir.join("fr").join("common.ftl")).expect("file moved along");
    assert!(text.contains("te1-probe = Bonjour"), "content unchanged");
    // second call: nothing more to do
    assert_eq!(fp_i18n::language_dir_in(&base), base.join("Languages"));
    // the language is still found
    let codes = fp_i18n::load_external_dir(&dir);
    assert!(codes.contains(&"fr".to_string()), "{codes:?}");
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn fallback_when_rename_fails() {
    let base = tmp("fallback");
    let old = old_language_dir(&base);
    let new = base.join("Languages");
    let dir = paths::migrate_dir_with(&new, &old, |_, _| {
        Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "no write access next to the exe",
        ))
    });
    assert_eq!(dir, old, "old folder is read as a fallback");
    assert!(!new.exists());
    assert!(old.join("fr").join("common.ftl").is_file(), "nothing lost");
    let _ = std::fs::remove_dir_all(&base);
}

/// A file opened without sharing blocks renaming its folder on Windows: the old folder stays in use.
#[cfg(windows)]
#[test]
fn fallback_with_locked_file() {
    use std::os::windows::fs::OpenOptionsExt;
    let _g = SWITCH.lock().unwrap_or_else(|e| e.into_inner());
    let base = tmp("locked");
    let old = old_language_dir(&base);
    let locked = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(old.join("fr").join("common.ftl"))
        .expect("lock");
    paths::set_migration_enabled(true);
    let dir = fp_i18n::language_dir_in(&base);
    // a failed attempt is not repeated in the same run
    let again = fp_i18n::language_dir_in(&base);
    paths::set_migration_enabled(false);
    drop(locked);
    assert_eq!(dir, old, "locked: old folder used");
    assert_eq!(again, old);
    assert!(!base.join("Languages").exists());
    assert!(
        std::fs::read_to_string(old.join("fr").join("common.ftl"))
            .expect("file")
            .contains("Bonjour")
    );
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn no_migration_when_new_exists() {
    let _g = SWITCH.lock().unwrap_or_else(|e| e.into_inner());
    let base = tmp("both");
    let old = old_language_dir(&base);
    let new = base.join("Languages");
    std::fs::create_dir_all(new.join("es")).expect("new dir");
    std::fs::write(
        new.join("es").join("common.ftl"),
        "language-name = Español\n",
    )
    .expect("ftl");
    paths::set_migration_enabled(true);
    let dir = fp_i18n::language_dir_in(&base);
    paths::set_migration_enabled(false);
    assert_eq!(dir, new);
    assert!(
        old.join("fr").join("common.ftl").is_file(),
        "old folder left alone"
    );
    assert!(new.join("es").join("common.ftl").is_file());
    assert!(!new.join("fr").exists(), "nothing merged");
    // readers also look into the leftover old folder
    assert_eq!(paths::leftover_dir(&dir, &old), Some(old.clone()));
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn disabled_migration_reads_old_folder() {
    let _g = SWITCH.lock().unwrap_or_else(|e| e.into_inner());
    let base = tmp("disabled");
    let old = old_language_dir(&base);
    paths::set_migration_enabled(false);
    assert_eq!(
        fp_i18n::language_dir_in(&base),
        old,
        "without the switch: fallback, no rename"
    );
    assert!(old.is_dir());
    assert_eq!(paths::leftover_dir(&old, &old), None);
    let _ = std::fs::remove_dir_all(&base);
}

/// `cargo test` sets `CARGO_MANIFEST_DIR`: the programs' switch stays off, so tests never rename
/// the real `%APPDATA%\Firepixel\Sprachen`.
#[test]
fn switch_stays_off_in_test_runs() {
    let _g = SWITCH.lock().unwrap_or_else(|e| e.into_inner());
    if std::env::var_os(paths::ENV_FORCE).is_some() {
        return;
    }
    assert!(!paths::enable_migration());
    assert!(!paths::migration_enabled());
}

#[test]
fn files_migrate_too() {
    let _g = SWITCH.lock().unwrap_or_else(|e| e.into_inner());
    let base = tmp("file");
    let (new, old) = (
        base.join(".firepixel-commands"),
        base.join(".firepixel-befehle"),
    );
    std::fs::write(&old, "0.1").expect("marker");
    paths::set_migration_enabled(false);
    assert_eq!(
        paths::migrate_file(&new, &old),
        old,
        "fallback without the switch"
    );
    paths::set_migration_enabled(true);
    assert_eq!(paths::migrate_file(&new, &old), new);
    paths::set_migration_enabled(false);
    assert_eq!(std::fs::read_to_string(&new).expect("moved"), "0.1");
    assert!(!old.exists());
    let _ = std::fs::remove_dir_all(&base);
}
