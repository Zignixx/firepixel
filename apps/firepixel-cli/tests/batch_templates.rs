//! `firepixel-cli batch --templates` und `batch --recipe <vorlage>` (BATCH-9, SPEC-BATCH §2.5, §5.8).

use assert_cmd::Command;
use serde_json::Value;
use std::path::{Path, PathBuf};

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fp_cli_batch9_{}_{name}", std::process::id()));
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

fn logo(path: &Path, w: u32, h: u32) {
    let px: Vec<u8> = (0..w * h)
        .flat_map(|i| {
            if (i % w) > w / 3 && (i / w) > h / 3 {
                [30, 60, 200, 255]
            } else {
                [255, 255, 255, 255]
            }
        })
        .collect();
    let item = fp_batch::item::still_from_rgba(w, h, &px).expect("item");
    let (bytes, _) =
        fp_batch::output::encode(&item, fp_batch::OutputFormat::Png, &Default::default())
            .expect("png");
    std::fs::write(path, bytes).expect("write");
}

#[test]
fn templates_listed_in_text_and_json() {
    let out = cli().args(["batch", "--templates"]).output().expect("run");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("logos-to-svg") && text.contains("Logos nachzeichnen → SVG"),
        "{text}"
    );
    let out = cli()
        .args(["--lang", "en", "batch", "--templates"])
        .output()
        .expect("run");
    assert!(String::from_utf8_lossy(&out.stdout).contains("Trace logos → SVG"));
    let out = cli()
        .args(["batch", "--templates", "--json"])
        .output()
        .expect("run");
    let v: Value = serde_json::from_slice(&out.stdout).expect("json");
    assert!(v["templates"].as_array().is_some_and(|a| {
        a.len() >= 10
            && a.iter()
                .any(|t| t["id"] == "web-webp" && t["format"] == "webp")
    }));
}

#[test]
fn recipe_by_template_name_writes_svg() {
    let dir = tmp("svg");
    let input = dir.join("in");
    std::fs::create_dir_all(&input).expect("in");
    logo(&input.join("a.png"), 30, 20);
    logo(&input.join("b.png"), 24, 24);
    let out = dir.join("out");
    let o = cli()
        .args(["batch", "--recipe", "logos-to-svg", "--in"])
        .arg(&input)
        .arg("--out")
        .arg(&out)
        .arg("--json")
        .output()
        .expect("run");
    assert_eq!(
        o.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    for n in ["a.svg", "b.svg"] {
        let svg = std::fs::read_to_string(out.join(n)).expect("svg");
        assert!(svg.contains("<svg") && svg.contains("<path"), "{n}");
    }
    // unbekannter Name ohne Datei: Fehler, Exit ≠ 0
    let o = cli()
        .args(["batch", "--recipe", "gibt-es-nicht", "--in"])
        .arg(&input)
        .output()
        .expect("run");
    assert_ne!(o.status.code(), Some(0));
    let _ = std::fs::remove_dir_all(&dir);
}
