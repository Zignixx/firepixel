//! Model folder always English (AGENTEN_REGELN §1c): `Models`, migration of the old German
//! `Modelle`. Only temporary folders – the real `%LOCALAPPDATA%\Firepixel\Modelle` of the user is
//! never touched: the end-to-end case runs in a child process with `LOCALAPPDATA` pointing to a
//! test folder.

use fp_ai::models;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

static SWITCH: Mutex<()> = Mutex::new(());
const CHILD_ENV: &str = "FP_TE1_MODELS_CHILD";

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fp-te1-ai-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("temp dir");
    d
}

/// Old folder `Modelle` with a (small) stand-in model file of the user.
fn old_models(base: &Path) -> PathBuf {
    let old = base.join(models::LEGACY_MODELS_DIR);
    std::fs::create_dir_all(&old).expect("old dir");
    std::fs::write(old.join("user-model.safetensors"), b"weights of the user").expect("file");
    old
}

#[test]
fn names_are_english() {
    assert_eq!(models::MODELS_DIR, "Models");
    assert_eq!(models::LEGACY_MODELS_DIR, "Modelle");
    let base = tmp("names");
    assert_eq!(models::folder_in(&base), base.join("Models"));
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn migrates_old_folder() {
    let _g = SWITCH.lock().unwrap_or_else(|e| e.into_inner());
    let base = tmp("migrate");
    let old = old_models(&base);
    fp_i18n::paths::set_migration_enabled(true);
    let dir = models::folder_in(&base);
    fp_i18n::paths::set_migration_enabled(false);
    assert_eq!(dir, base.join("Models"));
    assert!(!old.exists());
    assert_eq!(
        std::fs::read(dir.join("user-model.safetensors")).expect("moved"),
        b"weights of the user"
    );
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn fallback_when_not_movable() {
    let base = tmp("fallback");
    let old = old_models(&base);
    let dir = fp_i18n::paths::migrate_dir_with(&base.join("Models"), &old, |_, _| {
        Err(std::io::Error::other("file in use"))
    });
    assert_eq!(dir, old);
    assert!(old.join("user-model.safetensors").is_file(), "nothing lost");
    let _ = std::fs::remove_dir_all(&base);
}

/// A model file held open without sharing (e.g. being loaded) blocks the rename on Windows.
#[cfg(windows)]
#[test]
fn fallback_with_locked_model() {
    use std::os::windows::fs::OpenOptionsExt;
    let _g = SWITCH.lock().unwrap_or_else(|e| e.into_inner());
    let base = tmp("locked");
    let old = old_models(&base);
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(old.join("user-model.safetensors"))
        .expect("lock");
    fp_i18n::paths::set_migration_enabled(true);
    let dir = models::folder_in(&base);
    fp_i18n::paths::set_migration_enabled(false);
    drop(lock);
    assert_eq!(dir, old);
    assert!(!base.join("Models").exists());
    assert!(old.join("user-model.safetensors").is_file());
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn no_migration_when_new_exists() {
    let _g = SWITCH.lock().unwrap_or_else(|e| e.into_inner());
    let base = tmp("both");
    let old = old_models(&base);
    std::fs::create_dir_all(base.join("Models")).expect("new");
    fp_i18n::paths::set_migration_enabled(true);
    let dir = models::folder_in(&base);
    fp_i18n::paths::set_migration_enabled(false);
    assert_eq!(dir, base.join("Models"));
    assert!(
        old.join("user-model.safetensors").is_file(),
        "old folder left alone"
    );
    let _ = std::fs::remove_dir_all(&base);
}

/// Child process: `LOCALAPPDATA` points to a test folder (set by [`search_folders_end_to_end`]).
#[test]
fn child_search_folders() {
    let Some(expect) = std::env::var_os(CHILD_ENV) else {
        return;
    };
    let mode = expect.to_string_lossy().to_string();
    let local =
        PathBuf::from(std::env::var_os("LOCALAPPDATA").expect("LOCALAPPDATA")).join("Firepixel");
    if mode == "migrate" {
        fp_i18n::paths::set_migration_enabled(true);
    }
    let folders = models::search_folders();
    println!("FOLDERS={folders:?}");
    match mode.as_str() {
        "migrate" => {
            assert_eq!(folders.first(), Some(&local.join("Models")));
            assert!(
                local
                    .join("Models")
                    .join("user-model.safetensors")
                    .is_file()
            );
            assert!(!local.join("Modelle").exists());
        }
        "fallback" => assert_eq!(
            folders.first(),
            Some(&local.join("Modelle")),
            "without the switch the old folder is used"
        ),
        "both" => {
            assert_eq!(folders.first(), Some(&local.join("Models")));
            assert_eq!(
                folders.get(1),
                Some(&local.join("Modelle")),
                "leftover old folder is searched too"
            );
        }
        other => panic!("unknown mode {other}"),
    }
    assert_eq!(models::download_folder(), folders[0]);
}

#[test]
fn search_folders_end_to_end() {
    if std::env::var_os(CHILD_ENV).is_some() {
        return;
    }
    let exe = std::env::current_exe().expect("test exe");
    for mode in ["migrate", "fallback", "both"] {
        let root = tmp(&format!("e2e-{mode}"));
        let local = root.join("Firepixel");
        old_models(&local);
        if mode == "both" {
            std::fs::create_dir_all(local.join("Models")).expect("new");
        }
        let out = Command::new(&exe)
            .args([
                "child_search_folders",
                "--exact",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(CHILD_ENV, mode)
            .env("LOCALAPPDATA", &root)
            .env("APPDATA", &root)
            .env("FP_NO_SHELL_OPEN", "1")
            .output()
            .expect("child");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            out.status.success() && text.contains("1 passed"),
            "{mode}: {text}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
