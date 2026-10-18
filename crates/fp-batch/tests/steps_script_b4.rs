//! `script.condition`, `script.code`, `script.external` (B-4, `docs/BATCH.md` §3.12).

mod support;

use fp_batch::item::{Item, ItemInfo, ItemKind, still_from_rgba};
use fp_batch::recipe::{TargetKind, TargetSpec};
use fp_batch::step::Severity;
use fp_batch::{
    CancelToken, ItemStatus, OutputFormat, Recipe, RunOptions, StepContext, StepRegistry, executor,
};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use support::*;

fn info(w: u32, h: u32) -> ItemInfo {
    ItemInfo {
        path: PathBuf::from("C:/in/photo.png"),
        root: None,
        kind: ItemKind::Image,
        format: "png".into(),
        bytes: 1000,
        width: w,
        height: h,
        frames: 1,
        modified: None,
        exif: None,
    }
}

fn apply(id: &str, params: Value, item: &mut Item) -> fp_batch::Result<()> {
    let reg = StepRegistry::global();
    let step = reg.get(id).expect("step");
    let p = fp_batch::step::with_defaults(&step.params(), &params);
    let (w, h) = item.dimensions();
    let i = info(w, h);
    let cancel = CancelToken::new();
    let mut ctx = StepContext::new(0, 1, &i, &cancel, OutputFormat::Png);
    step.apply(item, &p, &mut ctx)
}

fn errors(id: &str, params: Value) -> Vec<String> {
    let mut r = Recipe::named("v");
    r.add_step(id, params);
    r.validate(&StepRegistry::global())
        .into_iter()
        .filter(|i| i.severity == Severity::Error)
        .map(|i| i.message)
        .collect()
}

fn recipe(out: &Path) -> Recipe {
    let mut r = Recipe::named("s");
    r.output.format = "png".into();
    r.output.target = TargetSpec {
        kind: TargetKind::Folder,
        path: Some(out.to_path_buf()),
        ..TargetSpec::default()
    };
    r
}

fn gray(path: &Path) -> bool {
    pixels(path).chunks(4).all(|p| p[0] == p[1] && p[1] == p[2])
}

#[test]
fn condition_gates_the_following_card() {
    let dir = temp("cond-gate");
    let wide = dir.join("wide.png");
    let small = dir.join("small.png");
    write_png(&wide, 40, 10);
    write_png(&small, 20, 10);
    let list = fp_batch::collect(&[wide, small], true, false, &[]).items;
    let out = dir.join("out");
    let mut r = recipe(&out);
    r.add_step("script.condition", json!({"condition": "width >= 30"}));
    r.add_step("format.colorDepth", json!({"mode": "grayscale"}));
    executor::run(&r, &list, &RunOptions::default()).expect("run");
    assert!(gray(&out.join("wide.png")));
    assert!(!gray(&out.join("small.png")));
    // skip mode: the small image is reported as skipped
    let out2 = dir.join("out2");
    let mut r = recipe(&out2);
    r.add_step(
        "script.condition",
        json!({"condition": "name ~ 'w*' and width > 30", "mode": "skipItem"}),
    );
    let rep = executor::run(&r, &list, &RunOptions::default()).expect("run 2");
    assert_eq!(rep.totals.done, 1);
    assert!(
        rep.items
            .iter()
            .any(|i| i.status == ItemStatus::Skipped && i.source.ends_with("small.png"))
    );
    assert!(!out2.join("small.png").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn condition_validation() {
    assert!(errors("script.condition", json!({"condition": "width >"})).len() == 1);
    assert!(errors("script.condition", json!({"condition": "colour > 3"}))[0].contains("colour"));
    assert!(
        errors(
            "script.condition",
            json!({"condition": "hasAlpha or size < 2mb"})
        )
        .is_empty()
    );
}

#[test]
fn code_runs_on_the_document() {
    let mut it = still_from_rgba(20, 10, &rgba(20, 10, false)).expect("item");
    apply(
        "script.code",
        json!({"code": "var d = fw.getDocumentDOM(); d.setDocumentCanvasSize({left: 0, top: 0, right: 8 + batch.total, bottom: 5});"}),
        &mut it,
    )
    .expect("code");
    assert_eq!(it.dimensions(), (9, 5));
}

#[test]
fn code_is_sandboxed_by_default() {
    let mut it = still_from_rgba(4, 4, &rgba(4, 4, false)).expect("item");
    let e = apply(
        "script.code",
        json!({"code": "Files.exists('C:/');"}),
        &mut it,
    )
    .expect_err("Files blocked");
    assert!(e.to_string().contains("sandbox"), "{e}");
    let e = apply(
        "script.code",
        json!({"code": "fw.getDocumentDOM().exportTo('C:/x.png');"}),
        &mut it,
    )
    .expect_err("export blocked");
    assert!(e.to_string().contains("sandbox"), "{e}");
    let e = apply(
        "script.code",
        json!({"code": "fw.saveDocument(fw.getDocumentDOM(), 'C:/x.fw.png');"}),
        &mut it,
    )
    .expect_err("save blocked");
    assert!(e.to_string().contains("sandbox"), "{e}");
    // allowIo: the same call works
    apply(
        "script.code",
        json!({"code": "Files.exists(batch.source);", "allowIo": true}),
        &mut it,
    )
    .expect("io allowed");
    assert!(!errors("script.code", json!({"code": ""})).is_empty());
}

#[test]
fn external_needs_opt_in() {
    let e = errors("script.external", json!({"command": "tool {in} {out}"}));
    assert!(e.iter().any(|m| m.contains("allowExternal")), "{e:?}");
    let e = errors(
        "script.external",
        json!({"command": "tool {in}", "allowExternal": true}),
    );
    assert_eq!(e.len(), 1, "needs {{out}}: {e:?}");
    assert!(
        errors(
            "script.external",
            json!({"command": "tool {in} {out}", "allowExternal": true})
        )
        .is_empty()
    );
    let mut it = still_from_rgba(4, 4, &rgba(4, 4, false)).expect("item");
    assert!(
        apply(
            "script.external",
            json!({"command": "tool {in} {out}"}),
            &mut it
        )
        .is_err()
    );
}

#[cfg(windows)]
#[test]
fn external_program_runs_with_placeholders_and_timeout() {
    let mut it = still_from_rgba(6, 3, &rgba(6, 3, false)).expect("item");
    apply(
        "script.external",
        json!({"command": "cmd /c copy {in} {out}", "allowExternal": true}),
        &mut it,
    )
    .expect("copy");
    assert_eq!(it.dimensions(), (6, 3));
    let e = apply(
        "script.external",
        json!({"command": "cmd /c ping -n 4 127.0.0.1", "allowExternal": true, "result": "ignore", "timeout": 1}),
        &mut it,
    )
    .expect_err("timeout");
    assert!(e.to_string().contains("1 s"), "{e}");
    let e = apply(
        "script.external",
        json!({"command": "cmd /c exit 3", "allowExternal": true, "result": "ignore"}),
        &mut it,
    )
    .expect_err("exit code");
    assert!(e.to_string().contains('3'), "{e}");
}
