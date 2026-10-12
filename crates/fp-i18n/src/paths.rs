//! Technical names are always English (AGENTEN_REGELN §1c) – migration of old German folder and
//! file names.
//!
//! Older versions created folders such as `%APPDATA%\Firepixel\Sprachen` or
//! `%LOCALAPPDATA%\Firepixel\Modelle`. The new names are English (`Languages`, `Models`). User data
//! must not get lost, so every lookup goes through [`migrate_dir`] / [`migrate_file`]:
//!
//! 1. The new path exists → it is used, the old one is left alone (no migration).
//! 2. Only the old path exists and migration is enabled → it is renamed once to the new name.
//! 3. The rename fails (no write access next to the exe, a locked file …) or migration is not
//!    enabled → the old path is used as a fallback (read and write), nothing is lost.
//! 4. Neither exists → the new path.
//!
//! Migration is **off by default** and is switched on only by the programs themselves
//! ([`enable_migration`] in `firepixel` and `firepixel-cli`). Test and development runs never
//! rename real folders (see [`enable_migration`]) unless `FP_MIGRATE=1` is set as well; library
//! code and tests of other crates therefore only ever read the old folder as a fallback.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

static ENABLED: AtomicBool = AtomicBool::new(false);
static FAILED: Mutex<Option<HashSet<(PathBuf, PathBuf)>>> = Mutex::new(None);

/// Environment variable that forces migration on even in a test environment (`FP_MIGRATE=1`).
pub const ENV_FORCE: &str = "FP_MIGRATE";
/// Environment variable that switches migration off (`FP_NO_MIGRATE=1`).
pub const ENV_OFF: &str = "FP_NO_MIGRATE";

/// Switches the one-time renaming of old German folders on (called by the programs at start).
/// Stays off in test and development runs – `FP_NO_SHELL_OPEN` (AGENTEN_REGELN §4a),
/// `FP_NO_MIGRATE`, or `CARGO_MANIFEST_DIR` (set by `cargo run` / `cargo test`, also inherited by
/// child processes) – unless `FP_MIGRATE=1`. Returns whether migration is now enabled.
pub fn enable_migration() -> bool {
    let set = |k: &str| std::env::var_os(k).is_some_and(|v| !v.is_empty() && v != "0");
    let on = set(ENV_FORCE) || !(set("FP_NO_SHELL_OPEN") || set(ENV_OFF) || set("CARGO_MANIFEST_DIR"));
    ENABLED.store(on, Ordering::SeqCst);
    on
}

/// Sets migration explicitly (tests).
pub fn set_migration_enabled(on: bool) {
    ENABLED.store(on, Ordering::SeqCst);
}

/// Is migration enabled?
pub fn migration_enabled() -> bool {
    ENABLED.load(Ordering::SeqCst)
}

fn failed_before(new: &Path, old: &Path) -> bool {
    FAILED.lock().ok().and_then(|g| g.as_ref().map(|s| s.contains(&(new.to_path_buf(), old.to_path_buf())))).unwrap_or(false)
}

fn mark_failed(new: &Path, old: &Path) {
    if let Ok(mut g) = FAILED.lock() {
        g.get_or_insert_with(HashSet::new).insert((new.to_path_buf(), old.to_path_buf()));
    }
}

/// Folder to use instead of the old German folder `old` (see module docs). A failed rename is not
/// retried during the same run.
pub fn migrate_dir(new: &Path, old: &Path) -> PathBuf {
    if !migration_enabled() || failed_before(new, old) {
        return if !new.is_dir() && old.is_dir() { old.to_path_buf() } else { new.to_path_buf() };
    }
    let r = migrate_dir_with(new, old, |a, b| {
        crate::profile::guard_write(a);
        crate::profile::guard_write(b);
        std::fs::rename(a, b)
    });
    if r.as_path() == old {
        mark_failed(new, old);
    }
    r
}

/// Like [`migrate_dir`] with its own rename function and without the enable switch (tests:
/// simulated failures).
pub fn migrate_dir_with(new: &Path, old: &Path, rename: impl FnOnce(&Path, &Path) -> std::io::Result<()>) -> PathBuf {
    if new.is_dir() || !old.is_dir() {
        return new.to_path_buf();
    }
    match rename(old, new) {
        Ok(()) if new.is_dir() && !old.exists() => {
            log::info!("migrated {} -> {}", old.display(), new.display());
            new.to_path_buf()
        }
        Ok(()) => {
            log::warn!("migration {} -> {} incomplete, using the old folder", old.display(), new.display());
            if old.is_dir() { old.to_path_buf() } else { new.to_path_buf() }
        }
        Err(e) => {
            log::warn!("cannot rename {} -> {} ({e}), using the old folder", old.display(), new.display());
            old.to_path_buf()
        }
    }
}

/// File to use instead of the old German file `old` (same rules as [`migrate_dir`]).
pub fn migrate_file(new: &Path, old: &Path) -> PathBuf {
    if new.exists() || !old.is_file() {
        return new.to_path_buf();
    }
    if !migration_enabled() || failed_before(new, old) {
        return old.to_path_buf();
    }
    crate::profile::guard_write(old);
    crate::profile::guard_write(new);
    match std::fs::rename(old, new) {
        Ok(()) if new.is_file() => new.to_path_buf(),
        _ => {
            mark_failed(new, old);
            if old.is_file() { old.to_path_buf() } else { new.to_path_buf() }
        }
    }
}

/// The old folder, if it still exists next to the folder actually in use `chosen` (both exist:
/// no migration took place). Callers that only read (language files, models) also look there.
pub fn leftover_dir(chosen: &Path, old: &Path) -> Option<PathBuf> {
    (chosen != old && old.is_dir()).then(|| old.to_path_buf())
}
