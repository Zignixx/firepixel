//! Guard: tests never read or write the real user profile (EB-3, AGENTEN_REGELN §4a).
//!
//! Static half of the protection (the runtime half is `fp_i18n::profile`, checked by
//! `crates\fp-engine\tests\no_real_profile.rs`):
//!
//! 1. **Runtime sources** (`crates\*\src`, `apps\*\src`): the profile folders are built only in
//!    `fp-i18n\src\profile.rs`. Nobody else reads `APPDATA`, `LOCALAPPDATA`, `USERPROFILE`,
//!    `XDG_CONFIG_HOME` or `XDG_STATE_HOME` – except the read-only system folders in [`ALLOWED_SRC`].
//! 2. **Tests and examples** (`tests\`, `examples\`): no direct profile paths (`var("APPDATA")`,
//!    `AppData\Roaming`, `AppData\Local\Firepixel` …) – except [`ALLOWED_TEST`].
//! 3. **Child processes**: a test that starts one of our programs (`cargo_bin`, `CARGO_BIN_EXE_`,
//!    `current_exe`) hands it the test profile (`fp_i18n::profile::child_env`,
//!    `FP_TEST_PROFILE`) or its own `APPDATA` and `LOCALAPPDATA`.

use std::path::{Path, PathBuf};

/// (file path suffix, environment variable, reason) – runtime sources.
const ALLOWED_SRC: &[(&str, &str, &str)] = &[
    ("fp-i18n/src/profile.rs", "*", "the one place that builds the profile folders"),
    ("fp-engine/src/commands/spelling.rs", "APPDATA", "reads system dictionaries in %APPDATA%\\Microsoft\\Spelling (read only)"),
    ("fp-text/src/font/mod.rs", "LOCALAPPDATA", "reads user fonts in %LOCALAPPDATA%\\Microsoft\\Windows\\Fonts (read only)"),
    ("fp-io/src/vector.rs", "LOCALAPPDATA", "reads user fonts in %LOCALAPPDATA%\\Microsoft\\Windows\\Fonts (read only)"),
];

/// (file path suffix, needle, reason) – tests and examples.
const ALLOWED_TEST: &[(&str, &str, &str)] = &[
    ("fp-ai/tests/models_path.rs", "var_os(\"LOCALAPPDATA\")", "child process reads the LOCALAPPDATA its parent set to a temporary folder"),
    ("fp-ui/tests/menu_fixes_fv13.rs", "var_os(\"APPDATA\")", "asserts that the commands folder is NOT below %APPDATA%"),
    ("fp-io/tests/roundtrip.rs", "var_os(\"USERPROFILE\")", "reads AVIF test images from ~\\.cargo\\registry (read only, not the profile)"),
];

const PROFILE_VARS: &[&str] = &["APPDATA", "LOCALAPPDATA", "USERPROFILE", "XDG_CONFIG_HOME", "XDG_STATE_HOME"];

fn root() -> PathBuf {
    std::fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")).expect("project root")
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            rust_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// Rust files below `<crate>\<sub>` of every crate and app (without this crate).
fn files(root: &Path, sub: &str) -> Vec<PathBuf> {
    let mut v = Vec::new();
    for group in ["crates", "apps"] {
        let Ok(rd) = std::fs::read_dir(root.join(group)) else { continue };
        for e in rd.flatten() {
            if e.file_name() == "fp-neutral" {
                continue;
            }
            rust_files(&e.path().join(sub), &mut v);
        }
    }
    v.sort();
    v
}

fn rel(root: &Path, p: &Path) -> String {
    p.strip_prefix(root).unwrap_or(p).to_string_lossy().replace('\\', "/")
}

/// Code part of a line (without a trailing `//` comment; good enough for this scan).
fn code(line: &str) -> &str {
    let t = line.trim_start();
    if t.starts_with("//") {
        return "";
    }
    match line.find(" // ") {
        Some(i) => &line[..i],
        None => line,
    }
}

fn env_reads(line: &str) -> Vec<&'static str> {
    PROFILE_VARS
        .iter()
        .copied()
        .filter(|v| ["var_os(\"", "var(\"", "env!(\"", "option_env!(\""].iter().any(|f| line.contains(&format!("{f}{v}\""))))
        .collect()
}

#[test]
fn runtime_sources_build_profile_paths_only_centrally() {
    let root = root();
    let mut hits = Vec::new();
    for f in files(&root, "src") {
        let r = rel(&root, &f);
        let Ok(text) = std::fs::read_to_string(&f) else { continue };
        for (i, line) in text.lines().enumerate() {
            for var in env_reads(code(line)) {
                let ok = ALLOWED_SRC.iter().any(|(suffix, v, _)| r.ends_with(suffix) && (*v == "*" || *v == var));
                if !ok {
                    hits.push(format!("{r}:{}: reads {var} directly – use fp_i18n::profile::roaming_dir()/local_dir(): {}", i + 1, line.trim()));
                }
            }
        }
    }
    assert!(hits.is_empty(), "profile paths outside fp_i18n::profile:\n{}", hits.join("\n"));
}

/// Home folder and known user folders (Documents, Pictures …) also come only from
/// `fp_i18n::profile` (`known_folder`), so that test runs see the test profile (NZ-5).
#[test]
fn runtime_sources_take_user_folders_from_the_profile_guard() {
    let root = root();
    let needles = ["home_dir(", "dirs::", "directories::", "SHGetKnownFolderPath", "var_os(\"HOME\")", "var(\"HOME\")"];
    let mut hits = Vec::new();
    for f in files(&root, "src") {
        let r = rel(&root, &f);
        if r.ends_with("fp-i18n/src/profile.rs") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&f) else { continue };
        for (i, line) in text.lines().enumerate() {
            let c = code(line);
            // whole word only (`model_dirs::` or `my_home_dir(` are not meant)
            let word = |n: &str| c.match_indices(n).any(|(p, _)| !c[..p].ends_with(|ch: char| ch.is_alphanumeric() || ch == '_'));
            for n in needles.iter().filter(|n| word(n)) {
                hits.push(format!("{r}:{}: {n} – use fp_i18n::profile::known_folder(): {}", i + 1, line.trim()));
            }
        }
    }
    assert!(hits.is_empty(), "user folders outside fp_i18n::profile:\n{}", hits.join("\n"));
}

#[test]
fn tests_use_no_direct_profile_paths() {
    let root = root();
    let mut hits = Vec::new();
    // escaped (`"…\\…"`) and raw (`r"…\…"`) spellings
    let needles = ["AppData\\\\Roaming", "AppData\\\\Local\\\\Firepixel", "AppData\\Roaming", "AppData\\Local\\Firepixel", "AppData/Roaming", "AppData/Local/Firepixel"];
    for sub in ["tests", "examples"] {
        for f in files(&root, sub) {
            let r = rel(&root, &f);
            let Ok(text) = std::fs::read_to_string(&f) else { continue };
            for (i, line) in text.lines().enumerate() {
                let c = code(line);
                let mut found: Vec<String> = env_reads(c).into_iter().map(|v| format!("var(\"{v}\")")).collect();
                found.extend(needles.iter().filter(|n| c.contains(*n)).map(|n| n.to_string()));
                for what in found {
                    let ok = ALLOWED_TEST.iter().any(|(suffix, needle, _)| r.ends_with(suffix) && c.contains(needle));
                    if !ok {
                        hits.push(format!("{r}:{}: {what}: {}", i + 1, line.trim()));
                    }
                }
            }
        }
    }
    assert!(hits.is_empty(), "tests must use fp_i18n::profile (test profile), not the real profile:\n{}", hits.join("\n"));
}

#[test]
fn child_processes_get_the_test_profile() {
    let root = root();
    let mut hits = Vec::new();
    for sub in ["tests", "examples"] {
        for f in files(&root, sub) {
            let r = rel(&root, &f);
            let Ok(text) = std::fs::read_to_string(&f) else { continue };
            let spawns_ours = ["cargo_bin(", "CARGO_BIN_EXE_", "current_exe()"].iter().any(|n| text.contains(n)) && text.contains("Command");
            if !spawns_ours {
                continue;
            }
            let handed = text.contains("child_env(")
                || text.contains("ENV_TEST_PROFILE")
                || text.contains("FP_TEST_PROFILE")
                || (text.contains(".env(\"APPDATA\"") && text.contains(".env(\"LOCALAPPDATA\""));
            if !handed {
                hits.push(r);
            }
        }
    }
    assert!(hits.is_empty(), "child processes without test profile (fp_i18n::profile::child_env):\n{}", hits.join("\n"));
}

#[test]
fn central_module_exists_and_guards() {
    let root = root();
    let text = std::fs::read_to_string(root.join("crates/fp-i18n/src/profile.rs")).expect("profile.rs");
    for f in ["pub fn roaming_dir", "pub fn local_dir", "pub fn guard_write", "pub fn use_test_profile", "pub fn child_env", "FP_NO_SHELL_OPEN", "\"deps\""] {
        assert!(text.contains(f), "profile.rs: {f} missing");
    }
    // the write functions of the profile call the guard
    for (file, n) in [
        ("crates/fp-engine/src/prefs.rs", 1),
        ("crates/fp-engine/src/shortcuts.rs", 2),
        ("crates/fp-engine/src/commands/misc.rs", 3),
        ("crates/fp-script/src/builtin_commands.rs", 3),
        ("crates/fp-mcp/src/transport.rs", 1),
        ("crates/fp-ai/src/download.rs", 1),
        ("crates/fp-i18n/src/paths.rs", 4),
    ] {
        let t = std::fs::read_to_string(root.join(file)).unwrap_or_default();
        assert!(t.matches("guard_write(").count() >= n, "{file}: expected at least {n} guard_write calls");
    }
}
