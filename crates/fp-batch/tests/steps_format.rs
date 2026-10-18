//! Format, metadata and overlay steps (B-3, `docs/BATCH.md` §3.2, §3.6, §3.7): colour depth,
//! colour profile, metadata sidecar, contact sheet — on small synthetic images.

mod support;

use fp_batch::item::{Item, ItemInfo, ItemKind, still_from_rgba};
use fp_batch::recipe::{TargetKind, TargetSpec};
use fp_batch::steps::format::{convert_to_srgb, sidecar_csv, srgb_profile};
use fp_batch::{
    CancelToken, OutputFormat, Recipe, RunOptions, StepContext, StepRegistry, executor,
};
use serde_json::{Value, json};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use support::*;

fn info(w: u32, h: u32) -> ItemInfo {
    ItemInfo {
        path: PathBuf::from("C:/in/a.png"),
        root: None,
        kind: ItemKind::Image,
        format: "png".into(),
        bytes: 0,
        width: w,
        height: h,
        frames: 1,
        modified: None,
        exif: None,
    }
}

fn apply(id: &str, params: Value, item: &mut Item) -> Vec<String> {
    let reg = StepRegistry::global();
    let step = reg.get(id).expect("step");
    let p = fp_batch::step::with_defaults(&step.params(), &params);
    let (w, h) = item.dimensions();
    let i = info(w, h);
    let cancel = CancelToken::new();
    let mut ctx = StepContext::new(0, 1, &i, &cancel, OutputFormat::Png);
    step.apply(item, &p, &mut ctx).expect("apply");
    ctx.warnings
}

fn px(item: &Item) -> Vec<[u8; 4]> {
    item.preview_image().expect("image").data().to_vec()
}

fn grad(w: u32, h: u32) -> Item {
    still_from_rgba(w, h, &rgba(w, h, false)).expect("item")
}

#[test]
fn color_depth_modes() {
    let mut g = grad(16, 16);
    apply("format.colorDepth", json!({"mode": "grayscale"}), &mut g);
    assert!(px(&g).iter().all(|p| p[0] == p[1] && p[1] == p[2]));
    let mut b = grad(16, 16);
    apply(
        "format.colorDepth",
        json!({"mode": "bits", "bits": 1}),
        &mut b,
    );
    assert!(
        px(&b)
            .iter()
            .all(|p| p[..3].iter().all(|c| *c == 0 || *c == 255))
    );
    let mut i = grad(16, 16);
    apply(
        "format.colorDepth",
        json!({"mode": "indexed", "colors": 4, "dither": "none"}),
        &mut i,
    );
    let colors: HashSet<[u8; 4]> = px(&i).into_iter().collect();
    assert!(colors.len() <= 4, "{}", colors.len());
}

#[test]
fn srgb_profile_is_readable_and_round_trips() {
    let icc = srgb_profile();
    assert_eq!(&icc[36..40], b"acsp");
    assert_eq!(
        u32::from_be_bytes(icc[0..4].try_into().expect("size")) as usize,
        icc.len()
    );
    let desc = fp_batch::meta::icc_description(&icc).unwrap_or_default();
    assert!(desc.contains("sRGB"), "{desc}");
    let item = grad(8, 8);
    let mut img = item.preview_image().expect("img");
    let before = img.data().to_vec();
    assert!(convert_to_srgb(&mut img, &icc));
    for (a, b) in img.data().iter().zip(&before) {
        for c in 0..3 {
            assert!((a[c] as i16 - b[c] as i16).abs() <= 1, "{a:?} {b:?}");
        }
    }
}

/// sRGB primaries with a linear tone curve (`curv` with count 0).
fn linear_profile() -> Vec<u8> {
    let mut icc = srgb_profile();
    let n = u32::from_be_bytes(icc[128..132].try_into().expect("n")) as usize;
    for k in 0..n {
        let e = 132 + 12 * k;
        if &icc[e..e + 4] == b"rTRC" {
            let off = u32::from_be_bytes(icc[e + 4..e + 8].try_into().expect("off")) as usize;
            icc[off + 8..off + 12].copy_from_slice(&0u32.to_be_bytes());
        }
    }
    icc
}

#[test]
fn color_profile_convert_assign_strip() {
    let mut item = still_from_rgba(1, 1, &[128, 128, 128, 255]).expect("item");
    if let Some(m) = item.meta_mut() {
        m.icc = Some(linear_profile());
    }
    apply("format.colorProfile", json!({}), &mut item);
    let p = px(&item)[0];
    assert!(
        (p[0] as i16 - 188).abs() <= 1,
        "linear 0.5 → sRGB 188: {p:?}"
    );
    assert_eq!(
        item.meta().and_then(|m| m.icc.clone()),
        Some(srgb_profile())
    );
    apply("format.colorProfile", json!({"mode": "strip"}), &mut item);
    assert!(item.meta().is_some_and(|m| m.icc.is_none()));
    apply(
        "format.colorProfile",
        json!({"mode": "assignSrgb"}),
        &mut item,
    );
    assert!(item.meta().is_some_and(|m| m.icc.is_some()));
    // unsupported profile: warning, pixels unchanged
    let mut odd = still_from_rgba(1, 1, &[10, 20, 30, 255]).expect("item");
    if let Some(m) = odd.meta_mut() {
        m.icc = Some(vec![0; 140]);
    }
    let w = apply("format.colorProfile", json!({}), &mut odd);
    assert_eq!(w.len(), 1);
    assert_eq!(px(&odd)[0], [10, 20, 30, 255]);
}

#[test]
fn sidecar_csv_quotes_cells() {
    let d = json!({"file": "a,b.png", "format": "png", "bytes": 3, "width": 1, "height": 2,
        "frames": 1, "metadata": {"exif": {"make": "Cam \"X\""}}});
    let csv = sidecar_csv(&d);
    let mut lines = csv.lines();
    assert!(
        lines
            .next()
            .is_some_and(|h| h.starts_with("file,format,bytes"))
    );
    let row = lines.next().expect("row");
    assert!(
        row.starts_with("\"a,b.png\",png,3,1,2,1,\"Cam \"\"X\"\"\""),
        "{row}"
    );
}

fn folder_recipe(out: &Path) -> Recipe {
    let mut r = Recipe::named("fmt");
    r.output.format = "png".into();
    r.output.target = TargetSpec {
        kind: TargetKind::Folder,
        path: Some(out.to_path_buf()),
        ..TargetSpec::default()
    };
    r
}

#[test]
fn metadata_export_writes_a_sidecar() {
    let dir = temp("sidecar");
    let src = dir.join("pic.png");
    write_png(&src, 6, 4);
    let list = fp_batch::collect(std::slice::from_ref(&src), true, false, &[]).items;
    for fmt in ["json", "csv"] {
        let out = dir.join(fmt);
        let mut r = folder_recipe(&out);
        r.add_step("metadata.export", json!({"format": fmt}));
        let rep = executor::run(&r, &list, &RunOptions::default()).expect("run");
        assert_eq!(rep.totals.done, 1, "{:#?}", rep.items);
        let side = out.join(format!("pic.{fmt}"));
        assert_eq!(rep.items[0].extra_targets, vec![side.clone()]);
        let text = std::fs::read_to_string(&side).expect("sidecar");
        if fmt == "json" {
            let v: Value = serde_json::from_str(&text).expect("json");
            assert_eq!(
                (v["width"].as_u64(), v["height"].as_u64()),
                (Some(6), Some(4))
            );
            assert_eq!(v["file"], "pic.png");
        } else {
            assert!(
                text.lines()
                    .nth(1)
                    .is_some_and(|l| l.starts_with("pic.png,png,"))
            );
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn contact_sheet_lays_out_a_grid() {
    let dir = temp("sheet");
    let srcs: Vec<PathBuf> = (0..5)
        .map(|k| {
            let p = dir.join(format!("img{k}.png"));
            write_png(&p, 20 + k, 10);
            p
        })
        .collect();
    let list = fp_batch::collect(&srcs, true, false, &[]).items;
    let out = dir.join("out");
    let mut r = folder_recipe(&out);
    r.add_step(
        "overlay.contactSheet",
        json!({"columns": 3, "cellSize": 32, "spacing": 4, "captions": false}),
    );
    executor::run(&r, &list, &RunOptions::default()).expect("run");
    // 3 columns × 2 rows of 32 px cells with 4 px spacing
    assert_eq!(
        dims(&out.join("contact-sheet.png")),
        (3 * 32 + 4 * 4, 2 * 32 + 3 * 4)
    );
    // pages of two, captions 12 px → 20 px caption row
    let out2 = dir.join("pages");
    let mut r = folder_recipe(&out2);
    r.add_step(
        "overlay.contactSheet",
        json!({"columns": 2, "cellSize": 16, "spacing": 0, "perPage": 2, "name": "sheet"}),
    );
    executor::run(&r, &list, &RunOptions::default()).expect("run 2");
    assert_eq!(dims(&out2.join("sheet-1.png")), (32, 16 + 20));
    assert_eq!(dims(&out2.join("sheet-3.png")), (16, 16 + 20));
    let _ = std::fs::remove_dir_all(&dir);
}
