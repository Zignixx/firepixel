//! K-7 (TW-89, re\38): importing with the placement pointer. Dragging yields a rectangle with the
//! aspect ratio of the visible pixels that encloses the pointer; the full bounds (whole bitmap
//! incl. transparent border) are mapped onto it with separate x/y factors; a click places at
//! original size; afterwards nothing is selected.

use crate::common::*;
use fp_engine::commands::{import_visible_bounds, inclusive_drag, place_preview_rect, place_rect};
use fp_geom::{Point, Rect};
use serde_json::json;
use std::path::PathBuf;

fn png(name: &str, w: usize, h: usize, f: impl Fn(usize, usize) -> u32) -> PathBuf {
    let px: Vec<u32> = (0..w * h).map(|i| f(i % w, i / w)).collect();
    let bytes = fp_export::png_out::write_png_truecolor(&px, w, h, true, &fp_export::png_out::PngOptions::default()).expect("PNG");
    let d = std::env::temp_dir().join(format!("fp-k7-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&d);
    let p = d.join(name);
    std::fs::write(&p, bytes).expect("write");
    p
}

/// Like `par.png`: 160 × 120, visible pixels 10,10–151,111 (141 × 101). Each call writes its own
/// file: tests run in parallel, and a shared path could be read while another test rewrites it (K-9).
fn par() -> PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    png(&format!("par_{n}.png"), 160, 120, |x, y| if (10..151).contains(&x) && (10..111).contains(&y) { if x < 80 { 0xff37_9bff } else { 0xffff_9b6d } } else { 0 })
}

fn session() -> fp_engine::Session {
    let mut s = fp_engine::Session::default();
    ex(&mut s, "file.new", json!({"width": 600, "height": 500, "background": "#ffffff"}));
    s
}

fn last_bounds(s: &fp_engine::Session) -> Rect {
    let t = top_elements(s);
    let e = t.first().expect("element");
    fp_engine::docops::element_bounds(&s.active().expect("doc").doc, e).expect("bounds")
}

#[test]
fn place_rect_formula() {
    let src = Rect::new(10.0, 10.0, 151.0, 111.0);
    // run 9: drag end 421,341 (+1) → y leads, x = 100 + 141 · 241/101 = 436.44 → 436
    assert_eq!(place_rect(Point::new(100.0, 100.0), Point::new(421.0, 341.0), &src, true), Rect::new(100.0, 100.0, 436.0, 341.0));
    // dragging up/left keeps the direction, then normalised
    assert_eq!(place_rect(Point::new(436.0, 341.0), Point::new(115.0, 100.0), &src, true), Rect::new(100.0, 100.0, 436.0, 341.0));
    // x leads when its ratio is larger
    assert_eq!(place_rect(Point::new(0.0, 0.0), Point::new(282.0, 10.0), &src, true), Rect::new(0.0, 0.0, 282.0, 202.0));
    // fit (maintainAspectRatio): the smaller ratio leads
    assert_eq!(place_rect(Point::new(0.0, 0.0), Point::new(282.0, 1000.0), &src, false), Rect::new(0.0, 0.0, 282.0, 202.0));
}

#[test]
fn drag_maps_full_bitmap_with_separate_factors() {
    let p = par();
    let mut s = session();
    let undo0 = s.active().expect("doc").history.undo_count();
    // F27 (TW-101): the drag end counts inclusively (+1), so the runner drag 420,340 gives R 436,341
    ex(&mut s, "file.import", json!({"path": p.to_string_lossy(), "placeFrom": [100, 100], "placeTo": [420, 340]}));
    let d = s.active().expect("doc");
    assert!(d.selection.elements.is_empty(), "nothing selected after placing");
    assert_eq!(d.history.undo_count(), undo0 + 1, "one history step");
    let b = last_bounds(&s);
    // visible content 121,120–417,322 (run 12 golden, edges floored)
    assert_eq!((b.left, b.top, b.right, b.bottom), (121.0, 120.0, 417.0, 322.0), "{b:?}");
    // bicubic recalculation, no stored matrix: the bitmap has the new pixel size
    let e = top_elements(&s)[0].clone();
    match &e.kind {
        fp_model::ElementKind::Bitmap(bm) => {
            let img = fp_engine::docops::bitmap_image(bm).expect("image");
            assert_eq!(img.width(), 296);
        }
        _ => panic!("bitmap expected"),
    }
}

#[test]
fn click_places_original_size_and_shift_free() {
    let p = par();
    let mut s = session();
    ex(&mut s, "file.import", json!({"path": p.to_string_lossy(), "placeFrom": [50.4, 60.6]}));
    // full bitmap's left top at the (rounded) click point → visible pixels at +10,+10
    assert_eq!(last_bounds(&s), Rect::new(60.0, 71.0, 201.0, 172.0));
    assert!(s.active().expect("doc").selection.elements.is_empty());
}

#[test]
fn image_without_transparent_border_stays_proportional() {
    let p = png("full.png", 40, 20, |_, _| 0xff5a_5a5a);
    let mut s = session();
    ex(&mut s, "file.import", json!({"path": p.to_string_lossy(), "placeFrom": [10, 10], "placeTo": [90, 90]}));
    // inclusive drag end 91,91 (F27): y leads (81/20 > 81/40): 162 × 81, factor 4.05 in both axes
    assert_eq!(last_bounds(&s), Rect::new(10.0, 10.0, 172.0, 91.0));
    // script rect without aspect: separate factors; with maintainAspectRatio: fitted
    ex(&mut s, "file.import", json!({"path": p.to_string_lossy(), "rect": [200, 200, 300, 300]}));
    assert_eq!(last_bounds(&s), Rect::new(200.0, 200.0, 300.0, 300.0));
    ex(&mut s, "file.import", json!({"path": p.to_string_lossy(), "rect": [300, 300, 400, 400], "maintainAspectRatio": true}));
    assert_eq!(last_bounds(&s), Rect::new(300.0, 300.0, 400.0, 350.0));
    assert_eq!(s.active().expect("doc").selection.elements.len(), 1, "script path keeps selecting");
}

/// T-10 (re 41 §2): pointer points are not rounded, only the result rectangle per coordinate (half
/// away from zero); the source width of the visible pixels counts half-open (10..151 → 141, not 140).
/// The exact runner drag 100,100 → 420,340 gives 100,100,435,340 (the original writes right 435);
/// with the +1 drag end 421,341 it is 100,100,436,341 (content 121–417).
#[test]
fn t10_exact_drag_gives_right_435() {
    let src = Rect::new(10.0, 10.0, 151.0, 111.0);
    assert_eq!(place_rect(Point::new(100.0, 100.0), Point::new(420.0, 340.0), &src, true), Rect::new(100.0, 100.0, 435.0, 340.0));
    assert_eq!(place_rect(Point::new(100.0, 100.0), Point::new(420.0, 341.0), &src, true), Rect::new(100.0, 100.0, 436.0, 341.0));
    // fractional pointer points stay unrounded until the end (100.4 → 100, 340.4: y leads → 435)
    assert_eq!(place_rect(Point::new(100.4, 100.4), Point::new(420.4, 340.4), &src, true), Rect::new(100.0, 100.0, 435.0, 340.0));
    // the import command itself treats the drag end inclusively (F27, see run12_drag_end_inclusive)
}

/// F27 (TW-101, run 12 `xs-file-formats/09`): both sides drag exactly 100,100 → 420,340; the original
/// places 121,120–417,322. The drag end is inclusive (+1 per axis → R 100,100,436,341) and the
/// bitmap edges are floored (x 121.0…417.1, y 120.08…322.9).
#[test]
fn run12_drag_end_inclusive() {
    assert_eq!(inclusive_drag(Point::new(100.0, 100.0), Point::new(420.0, 340.0)), (Point::new(100.0, 100.0), Point::new(421.0, 341.0)));
    // dragging up/left: the press pixel is the inclusive one
    assert_eq!(inclusive_drag(Point::new(420.0, 340.0), Point::new(100.0, 100.0)), (Point::new(421.0, 341.0), Point::new(100.0, 100.0)));
    let p = par();
    let mut s = session();
    ex(&mut s, "file.import", json!({"path": p.to_string_lossy(), "placeFrom": [100, 100], "placeTo": [420, 340]}));
    let b = last_bounds(&s);
    assert_eq!((b.left, b.top, b.right, b.bottom), (121.0, 120.0, 417.0, 322.0), "{b:?}");
    let mut s = session();
    ex(&mut s, "file.import", json!({"path": p.to_string_lossy(), "placeFrom": [420, 340], "placeTo": [100, 100]}));
    let b = last_bounds(&s);
    // anchored at the press point 421,341 (R 85,100,421,341): same size and y as the forward drag
    assert_eq!((b.width(), b.bottom), (296.0, 322.0), "{b:?}");
}

/// K-9 (re 38 mode 1): the placement pointer reads the source once and previews exactly the
/// rectangle `file.import {placeFrom, placeTo}` will fill; without known bounds the raw drag.
#[test]
fn k9_preview_rect_matches_import() {
    let p = par();
    let vis = import_visible_bounds(&p, None).expect("visible bounds");
    assert_eq!(vis, Rect::new(10.0, 10.0, 151.0, 111.0));
    // F27: inclusive drag end like the import (runner drag 420,340 → R 100,100,436,341)
    let (a, b) = (Point::new(100.0, 100.0), Point::new(420.0, 340.0));
    assert_eq!(place_preview_rect(a, b, Some(&vis)), Rect::new(100.0, 100.0, 436.0, 341.0));
    // dragging up/left: normalised like the import
    let (f, t) = inclusive_drag(b, a);
    assert_eq!(place_preview_rect(b, a, Some(&vis)), place_rect(f, t, &vis, true));
    // unknown bounds (unreadable file): raw drag rectangle
    assert_eq!(place_preview_rect(b, a, None), Rect::new(100.0, 100.0, 420.0, 340.0));
    assert!(import_visible_bounds(std::path::Path::new("does-not-exist.png"), None).is_none());
    // the import fills the previewed rectangle with the full bitmap (visible content 121–417)
    let mut s = session();
    ex(&mut s, "file.import", json!({"path": p.to_string_lossy(), "placeFrom": [100, 100], "placeTo": [420, 340]}));
    let bb = last_bounds(&s);
    assert_eq!((bb.left, bb.right), (121.0, 417.0), "{bb:?}");
}
