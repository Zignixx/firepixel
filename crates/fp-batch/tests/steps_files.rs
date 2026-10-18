//! File steps (B-3, `docs/BATCH.md` §3.1): sort into subfolders, process only some, detect
//! duplicates, set date — end to end through the executor on small synthetic PNGs.

mod support;

use fp_batch::item::still_from_rgba;
use fp_batch::recipe::{TargetKind, TargetSpec};
use fp_batch::steps::fileops::{dhash, expand_pattern, glob_regex, parse_date, size_class};
use fp_batch::{ItemStatus, Recipe, RunOptions, Severity, StepRegistry, executor};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use support::*;

fn folder_recipe(out: &Path, format: &str) -> Recipe {
    let mut r = Recipe::named("files");
    r.output.format = format.into();
    r.output.target = TargetSpec {
        kind: TargetKind::Folder,
        path: Some(out.to_path_buf()),
        ..TargetSpec::default()
    };
    r
}

fn items(paths: &[PathBuf]) -> Vec<fp_batch::ItemInfo> {
    fp_batch::collect(paths, true, false, &[]).items
}

fn opts() -> RunOptions {
    RunOptions {
        threads: Some(2),
        ..RunOptions::default()
    }
}

fn png(dir: &Path, name: &str, w: u32, h: u32) -> PathBuf {
    let p = dir.join(name);
    write_png(&p, w, h);
    p
}

fn issues(id: &str, params: Value) -> Vec<String> {
    let reg = StepRegistry::global();
    reg.get(id)
        .expect("step")
        .validate(&params)
        .into_iter()
        .filter(|i| i.severity == Severity::Error)
        .map(|i| i.param.unwrap_or_default())
        .collect()
}

#[test]
fn steps_are_registered_in_the_files_group() {
    let reg = StepRegistry::global();
    for id in [
        "file.organize",
        "file.filterSkip",
        "file.dedupe",
        "file.timestamps",
    ] {
        let s = reg.get(id).unwrap_or_else(|| panic!("{id}"));
        assert_eq!(s.group().name(), "files", "{id}");
    }
    assert!(reg.get("file.organize").is_some_and(|s| s.file_only()));
    assert!(reg.get("file.timestamps").is_some_and(|s| s.file_only()));
    assert!(reg.get("file.filterSkip").is_some_and(|s| !s.file_only()));
}

#[test]
fn helpers_size_class_glob_date() {
    assert_eq!(size_class(0, 0), "unknown");
    assert_eq!(size_class(639, 10), "small");
    assert_eq!(size_class(10, 640), "medium");
    assert_eq!(size_class(3999, 1), "large");
    assert_eq!(size_class(4000, 1), "huge");
    let re = glob_regex("*.PNG; logo?").expect("glob").expect("some");
    assert!(re.is_match("a.png") && re.is_match("logo1") && !re.is_match("logo12"));
    assert!(glob_regex("  ").expect("empty").is_none());
    assert!(parse_date("2024-02-03 04:05").is_some());
    assert!(parse_date("2024-02-03T04:05:06").is_some());
    assert!(parse_date("2024-02-03").is_some());
    assert!(parse_date("03.02.2024").is_none());
}

#[test]
fn expand_pattern_tokens_and_sanitizing() {
    let dir = temp("organize-pattern");
    let p = png(&dir, "Foto.PNG", 30, 20);
    let info = fp_batch::probe(&p).expect("probe");
    let now = chrono::NaiveDate::from_ymd_opt(2020, 1, 2)
        .and_then(|d| d.and_hms_opt(3, 4, 5))
        .expect("date");
    let got = expand_pattern("{ext}/{size}/{orientation}/{width}x{height}", &info, now);
    assert_eq!(got, PathBuf::from("png/small/landscape/30x20"));
    // `..`, empty segments and invalid characters never leave the output folder
    let got = expand_pattern("../a:b//./{camera}", &info, now);
    assert_eq!(got, PathBuf::from("_/a_b/unknown-camera"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn organize_by_type_copies_into_subfolders_without_decoding() {
    let dir = temp("organize-type");
    let a = png(&dir.join("in"), "a.png", 8, 6);
    let b = dir.join("in").join("notes.txt");
    std::fs::write(&b, b"hello").expect("txt");
    let out = dir.join("out");
    let mut r = folder_recipe(&out, "same");
    r.add_step("file.organize", json!({"by": "type"}));
    let list = fp_batch::collect(&[a.clone(), b.clone()], true, true, &[]).items;
    let rep = executor::run(&r, &list, &opts()).expect("run");
    assert_eq!(rep.totals.done, 2, "{:#?}", rep.items);
    assert_eq!(checksum(&out.join("png").join("a.png")), checksum(&a));
    assert!(out.join("txt").join("notes.txt").is_file());
    assert!(a.is_file() && b.is_file(), "copy keeps the sources");
    // move
    let mut r = folder_recipe(&dir.join("moved"), "same");
    r.add_step("file.organize", json!({"by": "type", "action": "move"}));
    let rep = executor::run(&r, &items(std::slice::from_ref(&a)), &opts()).expect("move");
    assert_eq!(rep.totals.done, 1);
    assert!(!a.exists() && dir.join("moved").join("png").join("a.png").is_file());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn organize_pattern_works_with_image_steps_and_dry_run() {
    let dir = temp("organize-run");
    let a = png(&dir, "wide.png", 12, 6);
    let b = png(&dir, "tall.png", 6, 12);
    let out = dir.join("out");
    let mut r = folder_recipe(&out, "webp");
    r.add_step(
        "file.organize",
        json!({"by": "pattern", "pattern": "{orientation}"}),
    );
    let plan = executor::plan(
        &r,
        &items(&[a.clone(), b.clone()]),
        &StepRegistry::global(),
        &Default::default(),
    )
    .expect("plan");
    let targets: Vec<PathBuf> = plan.entries.iter().map(|e| e.target.clone()).collect();
    assert_eq!(
        targets,
        vec![
            out.join("landscape").join("wide.webp"),
            out.join("portrait").join("tall.webp")
        ]
    );
    let rep = executor::run(&r, &items(&[a, b]), &opts()).expect("run");
    assert_eq!(rep.totals.done, 2, "{:#?}", rep.items);
    assert_eq!(dims(&out.join("portrait").join("tall.webp")), (6, 12));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn filter_skip_by_width_skips_or_copies() {
    let dir = temp("filter");
    let small = png(&dir, "small.png", 8, 6);
    let big = png(&dir, "big.png", 20, 6);
    let out = dir.join("out");
    let mut r = folder_recipe(&out, "webp");
    r.add_step("file.filterSkip", json!({"minWidth": 10}));
    let rep = executor::run(&r, &items(&[small.clone(), big.clone()]), &opts()).expect("run");
    assert_eq!(
        (rep.totals.done, rep.totals.skipped),
        (1, 1),
        "{:#?}",
        rep.items
    );
    let skipped = rep
        .items
        .iter()
        .find(|i| i.status == ItemStatus::Skipped)
        .expect("skipped");
    assert!(skipped.source.ends_with("small.png"));
    assert!(
        skipped.warnings.iter().any(|w| w.contains("8")),
        "{skipped:?}"
    );
    assert!(out.join("big.webp").is_file() && !out.join("small.webp").exists());
    // reversed + copy: the big one is copied unchanged, the small one converted
    let out2 = dir.join("out2");
    let mut r = folder_recipe(&out2, "webp");
    r.add_step(
        "file.filterSkip",
        json!({"minWidth": 10, "invert": true, "action": "copy"}),
    );
    let rep = executor::run(&r, &items(&[small, big.clone()]), &opts()).expect("run 2");
    assert_eq!(
        (rep.totals.done, rep.totals.skipped),
        (1, 1),
        "{:#?}",
        rep.items
    );
    assert!(out2.join("small.webp").is_file());
    assert_eq!(checksum(&out2.join("big.png")), checksum(&big));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn filter_skip_by_name_and_transparency() {
    let dir = temp("filter-name");
    let a = png(&dir, "logo-1.png", 8, 6);
    let b = png(&dir, "photo.png", 8, 6);
    let mut r = folder_recipe(&dir.join("out"), "png");
    r.add_step(
        "file.filterSkip",
        json!({"name": "logo*", "transparency": "without"}),
    );
    let rep = executor::run(&r, &items(&[a, b]), &opts()).expect("run");
    assert_eq!(
        (rep.totals.done, rep.totals.skipped),
        (1, 1),
        "{:#?}",
        rep.items
    );
    assert!(dir.join("out").join("logo-1.png").is_file());
    let mut r = folder_recipe(&dir.join("out-t"), "png");
    r.add_step("file.filterSkip", json!({"transparency": "with"}));
    let rep = executor::run(&r, &items(&[dir.join("photo.png")]), &opts()).expect("run t");
    assert_eq!(rep.totals.skipped, 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn dedupe_exact_keeps_first_and_own_names() {
    let dir = temp("dedupe");
    let a = png(&dir, "a.png", 8, 6);
    let b = png(&dir, "b.png", 9, 6);
    let c = png(&dir, "c.png", 8, 6); // same pixels as a
    let out = dir.join("out");
    let mut r = folder_recipe(&out, "png");
    r.add_step("file.dedupe", json!({}));
    let rep = executor::run(&r, &items(&[a, b, c]), &opts()).expect("run");
    assert_eq!(
        (rep.totals.done, rep.totals.skipped),
        (2, 1),
        "{:#?}",
        rep.items
    );
    assert!(out.join("a.png").is_file() && out.join("b.png").is_file());
    assert!(!out.join("c.png").exists());
    let c_rep = rep.items.iter().find(|i| i.index == 2).expect("c");
    assert_eq!(c_rep.status, ItemStatus::Skipped);
    assert!(c_rep.warnings.iter().any(|w| w.contains('a')), "{c_rep:?}");
    let b_rep = rep.items.iter().find(|i| i.index == 1).expect("b");
    assert_eq!(b_rep.target.as_deref(), Some(out.join("b.png").as_path()));
    // mark only: all three written, the duplicate carries a note
    let mut r = folder_recipe(&dir.join("out-mark"), "png");
    r.add_step("file.dedupe", json!({"action": "mark"}));
    let src: Vec<PathBuf> = ["a.png", "b.png", "c.png"]
        .iter()
        .map(|n| dir.join(n))
        .collect();
    let rep = executor::run(&r, &items(&src), &opts()).expect("mark");
    assert_eq!(rep.totals.done, 3, "{:#?}", rep.items);
    assert!(!rep.items[2].warnings.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn dhash_is_stable_under_scaling_and_differs_for_other_content() {
    let grad = |w: u32, h: u32| {
        let item = still_from_rgba(w, h, &rgba(w, h, false)).expect("item");
        item.preview_image().expect("img")
    };
    let a = dhash(&grad(90, 80));
    let b = dhash(&grad(45, 40));
    assert!((a ^ b).count_ones() <= 4, "{a:x} {b:x}");
    let mut flipped = rgba(90, 80, false);
    for p in flipped.chunks_mut(4) {
        p[0] = 255 - p[0];
        p[1] = 255 - p[1];
    }
    let f = still_from_rgba(90, 80, &flipped).expect("item");
    let c = dhash(&f.preview_image().expect("img"));
    assert!((a ^ c).count_ones() > 16, "{a:x} {c:x}");
}

#[test]
fn timestamps_fixed_date_is_set_on_output() {
    let dir = temp("timestamps");
    let a = png(&dir, "a.png", 8, 6);
    let out = dir.join("out");
    let mut r = folder_recipe(&out, "png");
    r.add_step(
        "file.timestamps",
        json!({"mode": "fixed", "date": "2001-02-03 04:05:06"}),
    );
    let rep = executor::run(&r, &items(std::slice::from_ref(&a)), &opts()).expect("run");
    assert_eq!(rep.totals.done, 1, "{:#?}", rep.items);
    let t = std::fs::metadata(out.join("a.png"))
        .and_then(|m| m.modified())
        .expect("mtime");
    let local = chrono::DateTime::<chrono::Local>::from(t).naive_local();
    assert_eq!(local.to_string(), "2001-02-03 04:05:06");
    // source mode with only file steps: copy keeps the source time
    let mut r = folder_recipe(&dir.join("out2"), "same");
    r.add_step("file.timestamps", json!({"mode": "source"}));
    executor::run(&r, &items(std::slice::from_ref(&a)), &opts()).expect("run 2");
    let src_t = std::fs::metadata(&a)
        .and_then(|m| m.modified())
        .expect("src");
    let dst_t = std::fs::metadata(dir.join("out2").join("a.png"))
        .and_then(|m| m.modified())
        .expect("dst");
    let diff = src_t
        .duration_since(dst_t)
        .or_else(|_| dst_t.duration_since(src_t))
        .expect("diff");
    assert!(diff.as_secs() < 2, "{diff:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn validation_of_the_file_steps() {
    assert_eq!(
        issues(
            "file.organize",
            json!({"by": "pattern", "pattern": "{nope}/x"})
        ),
        vec!["pattern"]
    );
    assert!(
        issues(
            "file.organize",
            json!({"by": "pattern", "pattern": "{date:YYYY}"})
        )
        .is_empty()
    );
    assert_eq!(
        issues("file.filterSkip", json!({"minWidth": 50, "maxWidth": 10})),
        vec!["maxWidth"]
    );
    assert!(issues("file.filterSkip", json!({"minWidth": 50})).is_empty());
    assert_eq!(
        issues("file.timestamps", json!({"mode": "fixed", "date": "soon"})),
        vec!["date"]
    );
    assert_eq!(
        issues("file.dedupe", json!({"distance": 40})),
        vec!["distance"]
    );
}
