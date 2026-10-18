//! `format.multiPage` (B-4, `docs/BATCH.md` §3.2): several images → one multi-page TIFF / PDF.

mod support;

use fp_batch::recipe::{TargetKind, TargetSpec};
use fp_batch::step::Severity;
use fp_batch::{Recipe, RunOptions, StepRegistry, executor};
use serde_json::json;
use std::path::{Path, PathBuf};
use support::*;

fn recipe(out: &Path, format: &str, params: serde_json::Value) -> Recipe {
    let mut r = Recipe::named("mp");
    r.output.format = format.into();
    r.output.target = TargetSpec {
        kind: TargetKind::Folder,
        path: Some(out.to_path_buf()),
        ..TargetSpec::default()
    };
    r.add_step("format.multiPage", params);
    r
}

fn sources(dir: &Path) -> Vec<PathBuf> {
    // b is larger than a and c; list order a, b, c
    [("a", 20, 10), ("b", 40, 30), ("c", 10, 10)]
        .iter()
        .map(|(n, w, h)| {
            let p = dir.join(format!("{n}.png"));
            write_png(&p, *w, *h);
            p
        })
        .collect()
}

fn u16le(d: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([d[at], d[at + 1]])
}
fn u32le(d: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([d[at], d[at + 1], d[at + 2], d[at + 3]])
}

/// (width, height) of every IFD in the chain.
fn tiff_pages(d: &[u8]) -> Vec<(u32, u32)> {
    assert_eq!(&d[..4], b"II*\0");
    let mut at = u32le(d, 4) as usize;
    let mut out = Vec::new();
    while at != 0 && out.len() < 100 {
        let n = u16le(d, at) as usize;
        let (mut w, mut h) = (0, 0);
        for k in 0..n {
            let e = at + 2 + 12 * k;
            match u16le(d, e) {
                256 => w = u32le(d, e + 8),
                257 => h = u32le(d, e + 8),
                _ => {}
            }
        }
        out.push((w, h));
        at = u32le(d, at + 2 + 12 * n) as usize;
    }
    out
}

#[test]
fn multi_page_tiff_keeps_each_size_and_order() {
    let dir = temp("mp-tiff");
    let list = fp_batch::collect(&sources(&dir), true, false, &[]).items;
    let out = dir.join("out");
    let r = recipe(&out, "tiff", json!({"order": "nameDesc", "name": "book"}));
    let rep = executor::run(&r, &list, &RunOptions::default()).expect("run");
    assert_eq!(rep.totals.failed, 0, "{:?}", rep.items);
    let data = std::fs::read(out.join("book.tif")).expect("tif");
    assert_eq!(tiff_pages(&data), vec![(10, 10), (40, 30), (20, 10)]);
    // the first page decodes with the regular reader
    let info = fp_batch::probe(&out.join("book.tif")).expect("probe");
    let item = fp_batch::item::load(&info, false).expect("decode");
    assert_eq!(item.dimensions(), (10, 10));
    // largest: all pages 40 × 30
    let out2 = dir.join("largest");
    let r = recipe(&out2, "tiff", json!({"pageSize": "largest"}));
    executor::run(&r, &list, &RunOptions::default()).expect("run 2");
    let data = std::fs::read(out2.join("a.tif")).expect("tif 2");
    assert_eq!(tiff_pages(&data), vec![(40, 30); 3]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn multi_page_pdf_has_one_page_per_image() {
    let dir = temp("mp-pdf");
    let list = fp_batch::collect(&sources(&dir), true, false, &[]).items;
    let out = dir.join("out");
    let r = recipe(
        &out,
        "pdf",
        json!({"pageSize": "a4", "dpi": 72, "margin": 0}),
    );
    let rep = executor::run(&r, &list, &RunOptions::default()).expect("run");
    assert_eq!(rep.totals.failed, 0, "{:?}", rep.items);
    let data = std::fs::read(out.join("a.pdf")).expect("pdf");
    let text = String::from_utf8_lossy(&data);
    assert!(data.starts_with(b"%PDF"));
    assert!(text.contains("/Count 3"), "three pages");
    // A4 at 72 dpi: 595 × 842 pt (landscape images → landscape page)
    assert!(
        text.contains("/MediaBox [0 0 842.0000 595.0000]"),
        "{text:.300}"
    );
    assert!(text.contains("/MediaBox [0 0 595.0000 842.0000]"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn multi_page_needs_tiff_or_pdf() {
    let r = recipe(Path::new("C:/out"), "png", json!({}));
    let issues = r.validate(&StepRegistry::global());
    assert!(
        issues
            .iter()
            .any(|i| i.code == "multiPageFormat" && i.severity == Severity::Error)
    );
    let r = recipe(Path::new("C:/out"), "pdf", json!({}));
    assert!(
        r.validate(&StepRegistry::global())
            .iter()
            .all(|i| i.severity != Severity::Error)
    );
}

#[test]
fn single_page_tiff_is_unchanged() {
    use fp_batch::item::Meta;
    use fp_batch::meta::encoders::{TiffCompression, write_tiff, write_tiff_pages};
    let img = fp_batch::item::still_from_rgba(7, 5, &rgba(7, 5, true))
        .expect("item")
        .preview_image()
        .expect("img");
    let m = Meta::default();
    let one = write_tiff(&img, &m, TiffCompression::Lzw).expect("one");
    let pages = write_tiff_pages(std::slice::from_ref(&img), &m, TiffCompression::Lzw).expect("p");
    assert_eq!(one, pages);
    assert_eq!(tiff_pages(&one), vec![(7, 5)]);
}
