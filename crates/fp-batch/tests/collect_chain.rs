//! Several collect steps in one recipe form a chain (B-4, `docs/BATCH.md` §5.2).

mod support;

use fp_batch::recipe::{TargetKind, TargetSpec};
use fp_batch::{ItemStatus, Recipe, RunOptions, executor};
use serde_json::json;
use std::path::{Path, PathBuf};
use support::*;

fn recipe(out: &Path) -> Recipe {
    let mut r = Recipe::named("chain");
    r.output.format = "png".into();
    r.output.target = TargetSpec {
        kind: TargetKind::Folder,
        path: Some(out.to_path_buf()),
        ..TargetSpec::default()
    };
    r
}

#[test]
fn dedupe_then_contact_sheet() {
    let dir = temp("collect-chain");
    let a = dir.join("a.png");
    let b = dir.join("b.png");
    let c = dir.join("c.png");
    write_png(&a, 20, 10);
    std::fs::copy(&a, &b).expect("copy");
    write_png(&c, 30, 12);
    let srcs: Vec<PathBuf> = vec![a, b, c];
    let list = fp_batch::collect(&srcs, true, false, &[]).items;
    let out = dir.join("out");
    let mut r = recipe(&out);
    r.add_step("file.dedupe", json!({}));
    r.add_step("format.colorDepth", json!({"mode": "grayscale"}));
    r.add_step(
        "overlay.contactSheet",
        json!({"columns": 4, "cellSize": 32, "spacing": 0, "captions": false}),
    );
    let rep = executor::run(&r, &list, &RunOptions::default()).expect("run");
    // two images left after dedupe → two cells
    assert_eq!(dims(&out.join("contact-sheet.png")), (64, 32));
    let px = pixels(&out.join("contact-sheet.png"));
    assert!(
        px.chunks(4).all(|p| p[0] == p[1] && p[1] == p[2]),
        "step between ran"
    );
    let skipped: Vec<_> = rep
        .items
        .iter()
        .filter(|i| i.status == ItemStatus::Skipped)
        .collect();
    assert_eq!(skipped.len(), 1);
    assert!(skipped[0].source.ends_with("b.png"));
    let images: Vec<_> = files_under(&out)
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e == "png"))
        .collect();
    assert_eq!(images.len(), 1, "only the sheet is written: {images:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn single_collect_is_unchanged() {
    let dir = temp("collect-single");
    let a = dir.join("a.png");
    let b = dir.join("b.png");
    write_png(&a, 20, 10);
    std::fs::copy(&a, &b).expect("copy");
    let list = fp_batch::collect(&[a, b], true, false, &[]).items;
    let out = dir.join("out");
    let mut r = recipe(&out);
    r.add_step("file.dedupe", json!({}));
    let rep = executor::run(&r, &list, &RunOptions::default()).expect("run");
    assert_eq!(rep.totals.done, 1);
    assert!(out.join("a.png").exists());
    assert!(!out.join("b.png").exists());
    let _ = std::fs::remove_dir_all(&dir);
}
