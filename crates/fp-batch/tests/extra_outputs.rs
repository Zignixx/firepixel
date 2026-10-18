//! Several outputs per item (BA-105): sprite sheet + atlas `.json`/`.css`, plus the editable
//! FW-PNG in the same run; single-output runs stay byte-identical.

mod support;

use fp_batch::executor::{self, RunOptions, atlas_file};
use fp_batch::item::{Animation, Frame, Item, Meta};
use fp_batch::output::{OutputFormat, encode};
use fp_batch::recipe::{Conflict, Recipe, StepConfig, TargetKind, TargetSpec};
use fp_batch::steps::files;
use fp_raster::{IRect, RgbaImage};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

fn gif(path: &Path) {
    let frames = (0..4u8)
        .map(|k| Frame {
            image: RgbaImage::from_raw(IRect::new(0, 0, 6, 4), vec![[k * 60, 20, 200, 255]; 24])
                .expect("frame"),
            delay_ms: 100,
        })
        .collect();
    let it = Item::Animation(Animation {
        width: 6,
        height: 4,
        frames,
        loop_count: 0,
        meta: Meta::default(),
    });
    let bytes = encode(&it, OutputFormat::Gif, &Default::default())
        .expect("gif")
        .0;
    std::fs::write(path, bytes).expect("write");
}

fn recipe(out: &Path, atlas: &str) -> Recipe {
    let mut r = Recipe::named("sheet-test");
    r.output.format = "png".into();
    r.output.target = TargetSpec {
        kind: TargetKind::Folder,
        path: Some(out.to_path_buf()),
        ..TargetSpec::default()
    };
    r.steps.push(StepConfig::new(
        "anim.spriteSheet",
        json!({"columns": 2, "spacing": 1, "atlas": atlas}),
    ));
    r
}

fn run(r: &Recipe, src: &[PathBuf]) -> fp_batch::Report {
    let items = fp_batch::collect(src, false, false, &[]).items;
    executor::run(
        r,
        &items,
        &RunOptions {
            threads: Some(1),
            ..RunOptions::default()
        },
    )
    .expect("run")
}

fn names(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = support::files_under(dir)
        .iter()
        .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .filter(|n| !n.starts_with("batch-report"))
        .collect();
    v.sort();
    v
}

#[test]
fn sprite_sheet_writes_atlas_and_fwpng_in_one_run() {
    let dir = support::temp("ba105-atlas");
    let src = dir.join("walk.gif");
    gif(&src);
    // three outputs per item: sheet, atlas, FW-PNG
    let out = dir.join("out");
    let mut r = recipe(&out, "json");
    r.output
        .options
        .insert(files::ALSO_FWPNG.into(), json!(true));
    let rep = run(&r, std::slice::from_ref(&src));
    assert_eq!(rep.totals.done, 1, "{:?}", rep.items[0].error);
    assert_eq!(names(&out), ["walk.fw.png", "walk.json", "walk.png"]);
    assert_eq!(rep.items[0].extra_targets.len(), 2);
    let atlas: Value =
        serde_json::from_str(&std::fs::read_to_string(out.join("walk.json")).expect("json"))
            .expect("parse");
    assert_eq!(atlas["image"], "walk.png");
    assert_eq!(atlas["size"], json!([13, 9]));
    assert_eq!(atlas["frames"].as_array().map(Vec::len), Some(4));
    assert_eq!(
        atlas["frames"][3],
        json!({"x": 7, "y": 5, "w": 6, "h": 4, "delayMs": 100})
    );
    // the run again: extras follow the conflict rule (rename) like the main file
    let rep2 = run(&r, std::slice::from_ref(&src));
    assert_eq!(rep2.totals.done, 1);
    assert!(out.join("walk (2).json").is_file(), "{:?}", names(&out));
    assert!(!support::has_temp_files(&out));
    // skip: nothing overwritten
    let before = support::checksum(&out.join("walk.json"));
    r.output.conflict = Conflict::Skip;
    run(&r, std::slice::from_ref(&src));
    assert_eq!(support::checksum(&out.join("walk.json")), before);

    // CSS atlas
    let css_out = dir.join("css");
    run(&recipe(&css_out, "css"), std::slice::from_ref(&src));
    let css = std::fs::read_to_string(css_out.join("walk.css")).expect("css");
    assert!(
        css.contains(".walk { background-image: url(\"walk.png\")"),
        "{css}"
    );
    assert!(css.contains(".walk-4 { width: 6px; height: 4px; background-position: -7px -5px; }"));

    // no atlas → one file, byte-identical to the sheet of the atlas run
    let plain = dir.join("plain");
    let rep = run(&recipe(&plain, "none"), std::slice::from_ref(&src));
    assert!(rep.items[0].extra_targets.is_empty());
    assert_eq!(names(&plain), ["walk.png"]);
    assert_eq!(
        std::fs::read(plain.join("walk.png")).expect("plain"),
        std::fs::read(css_out.join("walk.png")).expect("css run")
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn atlas_scales_with_the_final_sheet() {
    let atlas = json!({"size": [20, 10], "columns": 2, "rows": 1, "frames": [
        {"x": 0, "y": 0, "w": 10, "h": 10, "delayMs": 50},
        {"x": 10, "y": 0, "w": 10, "h": 10, "delayMs": 50}
    ]});
    let j: Value =
        serde_json::from_str(&atlas_file(&atlas, "json", "s.png", (10, 5)).expect("json"))
            .expect("parse");
    assert_eq!(
        j["frames"][1],
        json!({"x": 5, "y": 0, "w": 5, "h": 5, "delayMs": 50})
    );
    let css = atlas_file(&atlas, "css", "1 sheet.webp", (20, 10)).expect("css");
    assert!(css.starts_with(".s-1-sheet {"), "{css}");
    assert!(atlas_file(&atlas, "none", "s.png", (20, 10)).is_none());
    assert_eq!(
        executor::extra_target(Path::new("C:/o/a.fw.png"), "json"),
        PathBuf::from("C:/o/a.json")
    );
}
