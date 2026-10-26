//! Profile folders of Firepixel and the guard that keeps test runs out of the real user profile
//! (EB-3, AGENTEN_REGELN §4a).
//!
//! Every profile path goes through [`roaming_dir`] (`%APPDATA%\Firepixel`: preferences, saved
//! commands, extensions, shortcut sets, workspace layouts, libraries, languages, window state
//! `data\app.ron`) or [`local_dir`] (`%LOCALAPPDATA%\Firepixel`: AI models, `mcp-*.json`).
//!
//! **Test run** ([`is_test_run`]): cargo test binaries and examples (the exe lies in
//! `target\…\deps` or `target\…\examples`), processes with `FP_TEST_PROFILE` or
//! `FP_NO_SHELL_OPEN` set (AGENTEN_REGELN §4a), or after [`redirect`] / [`use_test_profile`].
//! In a test run
//! * both folders point to a temporary test profile of the process
//!   (`%TEMP%\firepixel-test-profiles\<pid>\{Roaming,Local}\Firepixel`), unless
//!   `FP_TEST_PROFILE=<folder>` names the profile (child processes of a test, see [`child_env`]) or
//!   `APPDATA`/`LOCALAPPDATA` already point somewhere other than the real profile;
//! * [`guard_write`] panics for every path below the real `%APPDATA%\Firepixel` or
//!   `%LOCALAPPDATA%\Firepixel` (real = below `%USERPROFILE%\AppData`). The write functions of
//!   the profile (preferences, saved commands, shortcut sets, libraries, token files, model
//!   downloads, migration …) call it, so code that bypasses the folders above still cannot write.
//! * Reading the user's AI models stays possible through [`real_local_dir`] (read only).
//!
//! The programs (`firepixel.exe`, `firepixel-cli.exe`) are no test run and behave as before.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};

/// Environment variable: `FP_TEST_PROFILE=<folder>` uses `<folder>` as test profile root,
/// `FP_TEST_PROFILE=1` only marks the process as a test run (own temporary profile).
pub const ENV_TEST_PROFILE: &str = "FP_TEST_PROFILE";
/// Name of the application folder in `%APPDATA%` / `%LOCALAPPDATA%`.
pub const APP_DIR: &str = "Firepixel";
/// Sub-folder of a test profile root that stands for `%APPDATA%`.
pub const ROAMING: &str = "Roaming";
/// Sub-folder of a test profile root that stands for `%LOCALAPPDATA%`.
pub const LOCAL: &str = "Local";

static REDIRECT: RwLock<Option<PathBuf>> = RwLock::new(None);

fn env_path(k: &str) -> Option<PathBuf> {
    std::env::var_os(k)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

fn env_set(k: &str) -> bool {
    std::env::var_os(k).is_some_and(|v| !v.is_empty() && v != "0")
}

/// `FP_TEST_PROFILE=<folder>` (not `1`/`true`): root given by the parent process.
fn env_profile_root() -> Option<PathBuf> {
    let v = std::env::var_os(ENV_TEST_PROFILE)?;
    let s = v.to_string_lossy();
    if s.is_empty() || s == "0" || s == "1" || s.eq_ignore_ascii_case("true") {
        return None;
    }
    Some(PathBuf::from(v))
}

/// The exe is a cargo test binary or example: `target\<profile>\deps\x-<hash>.exe` or
/// `target\<profile>\examples\x.exe` (next to `.fingerprint`).
fn exe_is_cargo_test() -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let Some(dir) = exe.parent() else {
        return false;
    };
    let name = dir
        .file_name()
        .map(|n| n.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    (name == "deps" || name == "examples")
        && dir
            .parent()
            .is_some_and(|p| p.join(".fingerprint").is_dir())
}

/// Is this process a test run (see module docs)?
pub fn is_test_run() -> bool {
    static AUTO: OnceLock<bool> = OnceLock::new();
    redirected_root().is_some()
        || *AUTO.get_or_init(|| {
            cfg!(test)
                || env_set(ENV_TEST_PROFILE)
                || env_set("FP_NO_SHELL_OPEN")
                || exe_is_cargo_test()
        })
}

/// Redirects all profile folders of this process to `root` (`root\Roaming\Firepixel`,
/// `root\Local\Firepixel`). Marks the process as a test run.
pub fn redirect(root: &Path) {
    if let Ok(mut g) = REDIRECT.write() {
        *g = Some(root.to_path_buf());
    }
}

/// Root of the redirected profile ([`redirect`], [`use_test_profile`] or `FP_TEST_PROFILE=<folder>`).
pub fn redirected_root() -> Option<PathBuf> {
    REDIRECT
        .read()
        .ok()
        .and_then(|g| g.clone())
        .or_else(env_profile_root)
}

/// Switches this process to its test profile and returns the root: the redirected root if there
/// is one, otherwise a new empty folder `%TEMP%\firepixel-test-profiles\<pid>` (once per process;
/// folders of earlier runs older than 12 hours are removed).
pub fn use_test_profile() -> PathBuf {
    if let Some(r) = redirected_root() {
        return r;
    }
    static AUTO: OnceLock<PathBuf> = OnceLock::new();
    let root = AUTO
        .get_or_init(|| {
            let base = std::env::temp_dir().join("firepixel-test-profiles");
            prune_old(&base);
            let d = base.join(std::process::id().to_string());
            let _ = std::fs::remove_dir_all(&d);
            let _ = std::fs::create_dir_all(d.join(ROAMING).join(APP_DIR));
            let _ = std::fs::create_dir_all(d.join(LOCAL).join(APP_DIR));
            d
        })
        .clone();
    if let Ok(mut g) = REDIRECT.write()
        && g.is_none()
    {
        *g = Some(root.clone());
    }
    redirected_root().unwrap_or(root)
}

/// Unique, empty folder for the files of one test: `%TEMP%\firepixel-test-files\<pid>\<tag>-<n>`.
/// Tests run in parallel threads and processes, so fixed names under `%TEMP%` collide
/// ("file in use", "path not found"). The folder of this process is emptied on the first call;
/// folders of earlier processes older than 12 hours are removed. `tag` stays English.
pub fn unique_temp_dir(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static BASE: OnceLock<PathBuf> = OnceLock::new();
    static N: AtomicU64 = AtomicU64::new(0);
    let base = BASE.get_or_init(|| {
        let b = std::env::temp_dir().join("firepixel-test-files");
        prune_old(&b);
        let d = b.join(std::process::id().to_string());
        let _ = std::fs::remove_dir_all(&d);
        d
    });
    let d = base.join(format!("{tag}-{}", N.fetch_add(1, Ordering::Relaxed)));
    let _ = std::fs::remove_dir_all(&d);
    let _ = std::fs::create_dir_all(&d);
    d
}

fn prune_old(base: &Path) {
    let Ok(rd) = std::fs::read_dir(base) else {
        return;
    };
    let limit = std::time::Duration::from_secs(12 * 3600);
    for e in rd.flatten() {
        let old = e
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|age| age > limit);
        if old && e.path().is_dir() {
            let _ = std::fs::remove_dir_all(e.path());
        }
    }
}

/// Sets the environment of a child process (CLI, test binary, app instance) started by a test,
/// so that it uses the same test profile and starts no external programs.
pub fn child_env(cmd: &mut std::process::Command) -> &mut std::process::Command {
    let root = use_test_profile();
    cmd.env(ENV_TEST_PROFILE, root).env("FP_NO_SHELL_OPEN", "1")
}

/// macOS: `~/Library/Application Support/Firepixel` (preferences, templates, ...); the local part
/// (AI models, logs, `mcp-*.json`) lives in its sub-folder `Local`. Other systems: `None`.
fn mac_support_dir(home: &Path, local: bool) -> Option<PathBuf> {
    if !cfg!(target_os = "macos") {
        return None;
    }
    let d = home
        .join("Library")
        .join("Application Support")
        .join(APP_DIR);
    Some(if local { d.join("Local") } else { d })
}

/// macOS: the user's font folder `~/Library/Fonts` (system folder, only read, never redirected);
/// other systems `None` (Windows user fonts are read in `fp-text` from `LOCALAPPDATA`).
pub fn mac_user_fonts_dir() -> Option<PathBuf> {
    if !cfg!(target_os = "macos") {
        return None;
    }
    env_path("HOME").map(|h| h.join("Library").join("Fonts"))
}

fn env_roaming() -> Option<PathBuf> {
    env_path("APPDATA")
        .map(|p| p.join(APP_DIR))
        .or_else(|| env_path("HOME").and_then(|h| mac_support_dir(&h, false)))
        .or_else(|| env_path("XDG_CONFIG_HOME").map(|p| p.join("firepixel")))
        .or_else(|| env_path("HOME").map(|h| h.join(".config").join("firepixel")))
}

fn env_local() -> Option<PathBuf> {
    env_path("LOCALAPPDATA")
        .map(|p| p.join(APP_DIR))
        .or_else(|| env_path("HOME").and_then(|h| mac_support_dir(&h, true)))
        .or_else(|| env_path("XDG_STATE_HOME").map(|p| p.join("firepixel")))
        .or_else(|| env_path("HOME").map(|h| h.join(".local").join("state").join("firepixel")))
}

/// The user's real `%APPDATA%\Firepixel` (from `%USERPROFILE%`, so that a changed `APPDATA`
/// of a test does not hide it; without `USERPROFILE` from `APPDATA`). Never redirected.
pub fn real_roaming_dir() -> Option<PathBuf> {
    if cfg!(windows) {
        env_path("USERPROFILE")
            .map(|u| u.join("AppData").join("Roaming").join(APP_DIR))
            .or_else(env_roaming)
    } else {
        env_path("HOME")
            .map(|h| {
                mac_support_dir(&h, false).unwrap_or_else(|| h.join(".config").join("firepixel"))
            })
            .or_else(env_roaming)
    }
}

/// The user's real `%LOCALAPPDATA%\Firepixel` (see [`real_roaming_dir`]). Never redirected; in a
/// test run only for reading (AI models).
pub fn real_local_dir() -> Option<PathBuf> {
    if cfg!(windows) {
        env_path("USERPROFILE")
            .map(|u| u.join("AppData").join("Local").join(APP_DIR))
            .or_else(env_local)
    } else {
        env_path("HOME")
            .map(|h| {
                mac_support_dir(&h, true)
                    .unwrap_or_else(|| h.join(".local").join("state").join("firepixel"))
            })
            .or_else(env_local)
    }
}

/// Known user folders (only read, e.g. as start folder of a file dialog).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnownFolder {
    /// `%USERPROFILE%\Pictures` (`$HOME/Pictures`)
    Pictures,
    /// `%USERPROFILE%\Documents` (`$HOME/Documents`)
    Documents,
}

impl KnownFolder {
    fn dir_name(self) -> &'static str {
        match self {
            KnownFolder::Pictures => "Pictures",
            KnownFolder::Documents => "Documents",
        }
    }
}

/// Home folder of a test profile given by `APPDATA` (`<home>\Roaming` or
/// `<home>\AppData\Roaming`), e.g. a GUI test instance started with its own `APPDATA` and
/// `FP_NO_SHELL_OPEN` (`fpmcp.ps1`).
fn home_from_appdata(appdata: &Path) -> Option<PathBuf> {
    let parent = appdata.parent()?;
    if parent.as_os_str().is_empty() {
        return None;
    }
    Some(
        if parent
            .file_name()
            .is_some_and(|n| n.eq_ignore_ascii_case("AppData"))
        {
            parent.parent()?.to_path_buf()
        } else {
            parent.to_path_buf()
        },
    )
}

/// In a test run without redirect: `APPDATA` points outside the real profile → its home folder.
fn env_test_home() -> Option<PathBuf> {
    if env_roaming().as_deref().is_none_or(is_real) {
        return None;
    }
    home_from_appdata(&env_path("APPDATA")?)
}

/// A known user folder: `<home>\Pictures` or `<home>\Documents` if it exists, else the home
/// folder itself (home = `%USERPROFILE%` on Windows, else `$HOME`). In a test run the test
/// profile root stands in for the home folder, so tests never see the user's real folders: the
/// redirected root, else the profile that `APPDATA` names (then preferences, templates and these
/// folders share one root, CT-3), else the process's own test profile. Never creates anything.
pub fn known_folder(f: KnownFolder) -> Option<PathBuf> {
    let home = if is_test_run() {
        redirected_root()
            .or_else(env_test_home)
            .unwrap_or_else(use_test_profile)
    } else if cfg!(windows) {
        env_path("USERPROFILE").or_else(|| env_path("HOME"))?
    } else {
        env_path("HOME")?
    };
    let dir = home.join(f.dir_name());
    Some(if dir.is_dir() { dir } else { home })
}

fn resolve(sub: &str, env: Option<PathBuf>) -> Option<PathBuf> {
    if let Some(r) = redirected_root() {
        return Some(r.join(sub).join(APP_DIR));
    }
    if is_test_run() && env.as_deref().is_none_or(is_real) {
        return Some(use_test_profile().join(sub).join(APP_DIR));
    }
    env
}

/// `%APPDATA%\Firepixel` (macOS `~/Library/Application Support/Firepixel`, else `$XDG_CONFIG_HOME/firepixel`
/// or `$HOME/.config/firepixel`); in a
/// test run the test profile (see module docs).
pub fn roaming_dir() -> Option<PathBuf> {
    resolve(ROAMING, env_roaming())
}

/// `%LOCALAPPDATA%\Firepixel` (macOS `~/Library/Application Support/Firepixel/Local`, else
/// `$XDG_STATE_HOME/firepixel` or `$HOME/.local/state/firepixel`);
/// in a test run the test profile (see module docs).
pub fn local_dir() -> Option<PathBuf> {
    resolve(LOCAL, env_local())
}

fn absolute(p: &Path) -> PathBuf {
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|c| c.join(p))
            .unwrap_or_else(|_| p.to_path_buf())
    }
}

/// Lexical form of `p` (absolute, `.`/`..` resolved, see [`clean`]).
fn lexical(p: &Path) -> String {
    let mut out = PathBuf::new();
    for c in absolute(p).components() {
        match c {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            c => out.push(c.as_os_str()),
        }
    }
    clean(&out)
}

/// Canonical form of `p` (long names instead of `ADMINI~1`, see [`clean`]); parts that do not
/// exist yet are appended to the canonical form of the deepest existing ancestor. A packaged
/// (MSIX) parent process virtualizes `AppData`: `…\AppData\Local\Packages\<app>\LocalCache\Roaming\…`
/// counts as `…\AppData\Roaming\…` (and `Local` likewise).
fn norm(p: &Path) -> String {
    let abs = absolute(p);
    let mut base: &Path = &abs;
    let mut rest: Vec<&OsStr> = Vec::new();
    let resolved = loop {
        if let Ok(c) = std::fs::canonicalize(base) {
            break rest.iter().rev().fold(c, |acc, r| acc.join(r));
        }
        match (base.parent(), base.file_name()) {
            (Some(parent), Some(name)) => {
                rest.push(name);
                base = parent;
            }
            _ => break abs.clone(),
        }
    };
    let s = clean(&resolved);
    if cfg!(windows)
        && let Some(i) = s.find(r"\appdata\local\packages\")
    {
        let after = &s[i + r"\appdata\local\packages\".len()..];
        if let Some(j) = after.find('\\') {
            for kind in ["roaming", "local"] {
                let cache = format!(r"\localcache\{kind}");
                if let Some(tail) = after[j..].strip_prefix(&cache)
                    && (tail.is_empty() || tail.starts_with('\\'))
                {
                    return format!(r"{}\appdata\{kind}{tail}", &s[..i]);
                }
            }
        }
    }
    s
}

/// String form for comparisons: on Windows `\` only, without `\\?\`, lower case; no trailing separator.
fn clean(p: &Path) -> String {
    let mut s = p.to_string_lossy().to_string();
    if cfg!(windows) {
        s = s.replace('/', "\\");
        if let Some(x) = s.strip_prefix(r"\\?\UNC\") {
            s = format!(r"\\{x}");
        } else if let Some(x) = s.strip_prefix(r"\\?\") {
            s = x.to_string();
        }
        s = s.to_lowercase();
    }
    while s.len() > 1 && (s.ends_with('\\') || s.ends_with('/')) {
        s.pop();
    }
    s
}

fn under(p: &str, root: &str) -> bool {
    p == root
        || p.strip_prefix(root)
            .is_some_and(|r| r.starts_with(['\\', '/']))
}

/// Does `p` lie in the user's real profile ([`real_roaming_dir`], [`real_local_dir`])?
pub fn is_real(p: &Path) -> bool {
    let (l, n) = (lexical(p), norm(p));
    [real_roaming_dir(), real_local_dir()]
        .into_iter()
        .flatten()
        .any(|r| under(&l, &lexical(&r)) || under(&n, &norm(&r)))
}

/// In a test run: error for a write (create, change, rename, delete) below the real profile;
/// otherwise `Ok`. Outside test runs always `Ok`.
pub fn check_write(path: &Path) -> std::io::Result<()> {
    if !is_test_run() || !is_real(path) {
        return Ok(());
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        crate::tr!("err-test-profile-write", path = path.display().to_string()),
    ))
}

/// Like [`check_write`], but panics – called by every write function of the profile.
pub fn guard_write(path: &Path) {
    if let Err(e) = check_write(path) {
        panic!("{e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_tests_are_test_runs_and_redirected() {
        assert!(is_test_run());
        let r = roaming_dir().expect("roaming");
        let l = local_dir().expect("local");
        assert!(!is_real(&r) && !is_real(&l), "{r:?} {l:?}");
        assert!(check_write(&r.join("preferences.json")).is_ok());
        if let Some(real) = real_roaming_dir() {
            assert!(
                is_real(&real.join("preferences.json")),
                "{} / {}",
                norm(&real.join("preferences.json")),
                norm(&real)
            );
            assert!(is_real(&real.join("..").join(APP_DIR).join("x")));
            assert!(check_write(&real.join("Commands").join("x.jsf")).is_err());
            assert!(
                std::panic::catch_unwind(|| guard_write(&real.join("preferences.json"))).is_err()
            );
        }
        if let Some(real) = real_local_dir() {
            assert!(check_write(&real.join("mcp-http.json")).is_err());
            assert!(
                !is_real(&real.with_file_name("Temp").join("x")),
                "%TEMP% is not the profile"
            );
        }
    }

    /// MSIX-virtualized AppData of a packaged parent process counts as the real profile.
    #[test]
    #[cfg(windows)]
    fn virtualized_appdata_counts_as_real() {
        let Some(u) = env_path("USERPROFILE") else {
            return;
        };
        let v = u
            .join("AppData")
            .join("Local")
            .join("Packages")
            .join("Some_App_123")
            .join("LocalCache")
            .join("Roaming")
            .join(APP_DIR)
            .join("preferences.json");
        assert!(
            norm(&v).ends_with(r"\appdata\roaming\firepixel\preferences.json"),
            "{}",
            norm(&v)
        );
        assert!(is_real(&v));
        let other = u
            .join("AppData")
            .join("Local")
            .join("Packages")
            .join("Some_App_123")
            .join("LocalCache")
            .join("Temp")
            .join("x");
        assert!(!is_real(&other));
    }

    #[test]
    fn home_of_a_test_instance_from_its_appdata() {
        let b = PathBuf::from(r"C:\t\fphome");
        assert_eq!(home_from_appdata(&b.join("Roaming")), Some(b.clone()));
        assert_eq!(
            home_from_appdata(&b.join("AppData").join("Roaming")),
            Some(b.clone())
        );
        assert_eq!(home_from_appdata(Path::new("Roaming")), None);
    }

    #[test]
    fn under_matches_whole_components() {
        assert!(under(r"c:\a\firepixel", r"c:\a\firepixel"));
        assert!(under(r"c:\a\firepixel\x", r"c:\a\firepixel"));
        assert!(!under(r"c:\a\firepixel2\x", r"c:\a\firepixel"));
    }
}
