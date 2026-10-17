//! `firepixel-cli batch --recipe … / --steps` (BATCH-1, SPEC-BATCH §5.8): Exit-Codes, JSON,
//! Trockenlauf, Fortschritt auf stderr; der alte Aufruf `batch --actions` bleibt.

use assert_cmd::Command;
use serde_json::Value;
use std::path::{Path, PathBuf};

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fp_cli_batch_{}_{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let _ = std::fs::create_dir_all(&d);
    d
}

fn cli() -> Command {
    let mut c = Command::cargo_bin("firepixel-cli").unwrap_or_else(|e| panic!("{e}"));
    c.env("FIREPIXEL_LANG", "de");
    c.env(
        fp_i18n::profile::ENV_TEST_PROFILE,
        fp_i18n::profile::use_test_profile(),
    )
    .env("FP_NO_SHELL_OPEN", "1");
    c
}

fn png(path: &Path, w: u32, h: u32) {
    let px: Vec<u8> = (0..w * h)
        .flat_map(|i| [(i % 255) as u8, 40, 200, 255])
        .collect();
    let item = fp_batch::item::still_from_rgba(w, h, &px).expect("item");
    let (bytes, _) =
        fp_batch::output::encode(&item, fp_batch::OutputFormat::Png, &Default::default())
            .expect("png");
    std::fs::write(path, bytes).expect("write");
}

fn json_of(out: &[u8]) -> Value {
    serde_json::from_slice(out)
        .unwrap_or_else(|e| panic!("kein JSON ({e}): {}", String::from_utf8_lossy(out)))
}

#[test]
fn steps_catalog_text_and_json() {
    let out = cli()
        .args(["batch", "--steps", "--json"])
        .output()
        .expect("run");
    assert!(out.status.success());
    let v = json_of(&out.stdout);
    assert!(
        v["steps"]
            .as_array()
            .is_some_and(|a| a.iter().any(|s| s["id"] == "file.copyOnly"))
    );
    let out = cli()
        .args(["--lang", "en", "batch", "--steps"])
        .output()
        .expect("run");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("Copy/Move Only"), "{text}");
}

#[test]
fn recipe_run_dry_run_and_exit_codes() {
    let dir = tmp("run");
    let input = dir.join("in");
    std::fs::create_dir_all(input.join("sub")).expect("in");
    png(&input.join("a.png"), 12, 8);
    png(&input.join("sub").join("b.png"), 6, 6);
    let recipe = dir.join("web.fpbatch.json");
    std::fs::write(&recipe, r#"{"format":"firepixel-batch-recipe","version":1,"name":"web","output":{"format":"webp"}}"#).expect("recipe");
    // Trockenlauf: nichts geschrieben
    let out = cli()
        .args(["batch", "--recipe"])
        .arg(&recipe)
        .arg("--in")
        .arg(&input)
        .args(["--recursive", "--dry-run", "--json"])
        .output()
        .expect("dry");
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v = json_of(&out.stdout);
    assert_eq!(v["totals"]["planned"], 2);
    assert!(!input.join("batch-output").exists());
    // echter Lauf mit eigenem Ausgabeordner und Bericht
    let outdir = dir.join("out");
    let report = dir.join("bericht.json");
    let out = cli()
        .args(["batch", "--recipe"])
        .arg(&recipe)
        .arg("--in")
        .arg(&input)
        .arg("--recursive")
        .arg("--out")
        .arg(&outdir)
        .arg("--report")
        .arg(&report)
        .output()
        .expect("run");
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(outdir.join("a.webp").is_file());
    assert!(outdir.join("sub").join("b.webp").is_file());
    let r: Value =
        serde_json::from_str(&std::fs::read_to_string(&report).expect("report")).expect("json");
    assert_eq!(r["totals"]["done"], 2);
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("[2/2]"),
        "Fortschritt je Datei auf stderr: {err}"
    );
    // Fehler in einer Datei → Exit 1, die andere wird trotzdem geschrieben
    std::fs::write(input.join("kaputt.png"), b"\x89PNG\r\n\x1a\nxx").expect("bad");
    let out = cli()
        .args(["batch", "--recipe"])
        .arg(&recipe)
        .arg(input.join("kaputt.png"))
        .arg(input.join("a.png"))
        .arg("--out")
        .arg(dir.join("out2"))
        .output()
        .expect("run");
    assert_eq!(out.status.code(), Some(1));
    assert!(dir.join("out2").join("a.webp").is_file());
    // falscher Aufruf → Exit 2
    let out = cli()
        .args(["batch", "--recipe"])
        .arg(&recipe)
        .output()
        .expect("no files");
    assert_eq!(out.status.code(), Some(2));
    let out = cli()
        .args(["batch", "--recipe", "{\"output\":{\"format\":\"bogus\"}}"])
        .arg(input.join("a.png"))
        .output()
        .expect("bad recipe");
    assert_eq!(out.status.code(), Some(2));
    let out = cli()
        .args(["batch", "--recipe"])
        .arg(&recipe)
        .args(["--conflict", "explode"])
        .arg(input.join("a.png"))
        .output()
        .expect("bad conflict");
    assert_eq!(out.status.code(), Some(2));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn legacy_actions_call_still_works() {
    let dir = tmp("legacy");
    let input = dir.join("in");
    std::fs::create_dir_all(&input).expect("in");
    png(&input.join("a.png"), 10, 10);
    let out = cli()
        .args(["batch", "--actions", "[]", "--format", "png", "--json"])
        .arg("--in")
        .arg(&input)
        .arg("--out")
        .arg(dir.join("out"))
        .output()
        .expect("legacy");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(dir.join("out").join("a.png").is_file());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn batch_commands_are_available_in_cli_sessions() {
    // `run --cmd batch.steps` läuft über das Befehlsregister (Haken BatchHost)
    let out = cli()
        .args(["run", "--new", "10x10", "--cmd", "batch.steps", "--out"])
        .arg(tmp("cmd").join("x.png"))
        .arg("--json")
        .output()
        .expect("run");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "{text}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(text.contains("file.copyOnly"), "{text}");
}
