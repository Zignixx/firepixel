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

fn three(dir: &Path) -> Vec<fp_batch::ItemInfo> {
    let srcs: Vec<PathBuf> = ["a", "b", "c"]
        .iter()
        .enumerate()
        .map(|(k, n)| {
            let p = dir.join(format!("{n}.png"));
            write_png(&p, 20 + 4 * k as u32, 10);
            p
        })
        .collect();
    fp_batch::collect(&srcs, true, false, &[]).items
}

fn status_of(rep: &fp_batch::Report, name: &str) -> (ItemStatus, Option<String>) {
    let r = rep
        .items
        .iter()
        .find(|i| i.source.ends_with(name))
        .unwrap_or_else(|| panic!("{name}: {:?}", rep.items));
    (r.status, r.error.clone())
}

#[test]
fn item_failing_between_collect_steps_is_reported_failed() {
    // the failing item is the first input (it used to lend its report to the stage result) and a
    // later one
    for fail in [0, 1] {
        let dir = temp(&format!("collect-chain-fail{fail}"));
        let list = three(&dir);
        let out = dir.join("out");
        let mut r = recipe(&out);
        r.add_step("file.dedupe", json!({}));
        r.add_step(
            "script.code",
            json!({ "code": format!("if (batch.index == {fail}) throw new Error('boom');") }),
        );
        r.add_step(
            "overlay.contactSheet",
            json!({"columns": 4, "cellSize": 32, "spacing": 0, "captions": false}),
        );
        let rep = executor::run(&r, &list, &RunOptions::default()).expect("run");
        let names = ["a.png", "b.png", "c.png"];
        let (st, err) = status_of(&rep, names[fail]);
        assert_eq!(st, ItemStatus::Failed, "{fail}: {:?}", rep.items);
        assert!(err.is_some_and(|e| e.contains("boom")), "{fail}");
        for (k, n) in names.iter().enumerate().filter(|(k, _)| *k != fail) {
            assert_eq!(status_of(&rep, n).0, ItemStatus::Done, "{fail}/{k}");
        }
        assert_eq!(rep.totals.failed, 1, "{fail}");
        assert_eq!(rep.totals.done, 2, "{fail}");
        // two cells left on the sheet
        assert_eq!(dims(&out.join("contact-sheet.png")), (64, 32), "{fail}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[test]
fn condition_gate_does_not_cross_a_collect_step() {
    let dir = temp("collect-chain-gate");
    let list = three(&dir);
    let out = dir.join("out");
    let mut r = recipe(&out);
    // never met: gates the next two per-item cards of *this* stage (there are none)
    r.add_step(
        "script.condition",
        json!({"condition": "width > 10000", "steps": 2}),
    );
    r.add_step("file.dedupe", json!({}));
    r.add_step("format.colorDepth", json!({"mode": "grayscale"}));
    r.add_step(
        "overlay.contactSheet",
        json!({"columns": 4, "cellSize": 32, "spacing": 0, "captions": false}),
    );
    // a gate inside the middle stage still works there
    let rep = executor::run(&r, &list, &RunOptions::default()).expect("run");
    assert_eq!(rep.totals.done, 3, "{:?}", rep.items);
    let px = pixels(&out.join("contact-sheet.png"));
    assert!(
        px.chunks(4).all(|p| p[0] == p[1] && p[1] == p[2]),
        "the gate of stage 1 skipped a card of stage 2"
    );
    let out2 = dir.join("out2");
    let mut r = recipe(&out2);
    r.add_step("file.dedupe", json!({}));
    r.add_step("script.condition", json!({"condition": "width > 10000"}));
    r.add_step("format.colorDepth", json!({"mode": "grayscale"}));
    r.add_step(
        "overlay.contactSheet",
        json!({"columns": 4, "cellSize": 32, "spacing": 0, "captions": false}),
    );
    executor::run(&r, &list, &RunOptions::default()).expect("run 2");
    let px = pixels(&out2.join("contact-sheet.png"));
    assert!(
        px.chunks(4).any(|p| p[0] != p[1] || p[1] != p[2]),
        "gate inside the stage"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
