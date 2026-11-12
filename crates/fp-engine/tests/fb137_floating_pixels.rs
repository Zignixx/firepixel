//! FB-137 (TW-292, RE spec 80): moved selected pixels float above their bitmap. The bitmap keeps
//! the hole and is not touched while the block moves on; the block is merged only by a merging
//! command (deselect, a new selection, leaving bitmap mode, save, any other edit), not by the
//! release, further drags or nudges, tool changes, Enter or a Pointer click into empty canvas;
//! Delete discards it; undo of a merging row brings it back. Drag and arrow-key rules per tool and
//! key, the soft-edge rule, the bitmap growth and the cut at the canvas edge as measured.

use crate::common;
use common::*;
use fp_engine::prefs::PrefValue;
use fp_engine::tools::{ARROW_KEYS_MOVE_PIXELS_PREF, ToolId, ToolKey};
use fp_engine::{Session, ToolEvent};
use fp_geom::IRect;
use fp_model::{ElementId, ElementKind};
use fp_raster::RgbaImage;
use serde_json::json;

/// The measured test image (RE spec 80): every pixel unique, R = 20 + bx, G = 20 + by, B = 100.
fn pattern(l: i32, t: i32, w: i32, h: i32) -> RgbaImage {
    let mut img = RgbaImage::new(IRect::from_xywh(l, t, w, h)).expect("image");
    for y in 0..h {
        for x in 0..w {
            img.set_pixel(
                l + x,
                t + y,
                fp_color::Rgba8::new((20 + x) as u8, (20 + y) as u8, 100, 255),
            );
        }
    }
    img
}

/// A 400 x 300 document with the pattern bitmap 200 x 150 at 100,75 (selected), like RE spec 80.
fn setup() -> (Session, ElementId, RgbaImage) {
    let mut s = doc_session(400.0, 300.0);
    let img = pattern(100, 75, 200, 150);
    s.clipboard.clear();
    s.clipboard.pixels = Some(img.clone());
    let v = ex(&mut s, "edit.paste", json!({}));
    let id = ElementId(v["ids"][0].as_u64().expect("id"));
    assert_eq!(image(&s, id), img, "pasted in place");
    (s, id, img)
}

fn marquee(s: &mut Session, r: [i32; 4]) {
    ex(s, "select.pixels", json!({"shape": "rect", "rect": r}));
}

fn image(s: &Session, id: ElementId) -> RgbaImage {
    let d = s.active().expect("document");
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, id).expect("bitmap");
    let ElementKind::Bitmap(b) = &e.kind else {
        panic!("not a bitmap")
    };
    fp_engine::docops::bitmap_image(b).expect("image")
}

fn px(s: &Session, id: ElementId, x: i32, y: i32) -> [u8; 4] {
    image(s, id).get(x, y).unwrap_or([0; 4])
}

fn floating(s: &Session) -> Option<(i32, i32)> {
    s.active()
        .and_then(|d| d.selection.floating.as_ref())
        .map(|f| (f.dx, f.dy))
}

fn pixel_bounds(s: &Session) -> Option<IRect> {
    s.active()
        .and_then(|d| d.selection.pixels.as_ref())
        .map(|m| m.tight_bounds())
}

fn rows(s: &Session) -> usize {
    s.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

fn label(s: &Session) -> String {
    s.active()
        .and_then(|d| d.history.undo_label())
        .unwrap_or_default()
        .to_string()
}

fn drag(s: &mut Session, from: (f32, f32), to: (f32, f32), ctrl: bool, alt: bool) {
    let m = |e: ToolEvent| {
        let e = if ctrl { e.ctrl() } else { e };
        if alt { e.alt() } else { e }
    };
    let mid = ((from.0 + to.0) / 2.0, (from.1 + to.1) / 2.0);
    events(
        s,
        &[
            m(ToolEvent::down(from.0, from.1)),
            m(ToolEvent::drag(mid.0, mid.1)),
            m(ToolEvent::drag(to.0, to.1)),
            m(ToolEvent::up(to.0, to.1)),
        ],
    );
}

fn key(s: &mut Session, k: ToolKey, shift: bool, ctrl: bool, alt: bool) {
    let mut e = ToolEvent::key(k);
    if shift {
        e = e.shift();
    }
    if ctrl {
        e = e.ctrl();
    }
    if alt {
        e = e.alt();
    }
    events(s, &[e]);
}

/// The pattern pixel of bitmap coordinate (x, y) in document pixels.
fn orig(img: &RgbaImage, x: i32, y: i32) -> [u8; 4] {
    img.get(x, y).unwrap_or([0; 4])
}

/// Tester repro 1 (video 1569949619452907581): nudging the selected pixels over other pixels and
/// back leaves the covered pixels intact; the merge puts the block back exactly.
#[test]
fn fb137_nudge_over_other_pixels_and_back_keeps_them() {
    let (mut s, id, img) = setup();
    marquee(&mut s, [120, 95, 180, 145]);
    s.set_tool(ToolId::Marquee).expect("tool");
    let r0 = rows(&s);
    // Ctrl+arrow lifts, then the arrows move the floating block (RE spec 80 section 3)
    key(&mut s, ToolKey::Right, false, true, false);
    assert_eq!(floating(&s), Some((1, 0)), "lifted by Ctrl+Right");
    for _ in 0..6 {
        key(&mut s, ToolKey::Right, true, false, false);
    }
    assert_eq!(floating(&s), Some((61, 0)));
    assert_eq!(pixel_bounds(&s), Some(IRect::new(181, 95, 241, 145)));
    // the bitmap holds the hole only: the pixels under the block are untouched
    assert_eq!(px(&s, id, 150, 120)[3], 0, "hole");
    for (x, y) in [(200, 100), (230, 140), (181, 95), (240, 144)] {
        assert_eq!(px(&s, id, x, y), orig(&img, x, y), "covered pixel {x},{y} intact");
    }
    for _ in 0..6 {
        key(&mut s, ToolKey::Left, true, false, false);
    }
    key(&mut s, ToolKey::Left, false, false, false);
    assert_eq!(floating(&s), Some((0, 0)), "back home");
    assert_eq!(rows(&s), r0 + 1, "one nudge row");
    assert_eq!(label(&s), fp_i18n::t("msg-step-move"));
    ex(&mut s, "select.none", json!({}));
    assert_eq!(floating(&s), None, "merged");
    assert!(image(&s, id) == img, "the image is whole again");
}

/// Tester repro 2 (video 1569955409006043196): marquee half a shape, drag beside, release, drag
/// back over, release, drag away: the pixels the block covered stay where they are.
#[test]
fn fb137_drag_beside_back_over_and_away_keeps_the_covered_pixels() {
    let (mut s, id, img) = setup();
    marquee(&mut s, [120, 95, 160, 135]);
    s.set_tool(ToolId::Pointer).expect("tool");
    let r0 = rows(&s);
    drag(&mut s, (130.0, 100.0), (200.0, 100.0), false, false);
    assert_eq!(floating(&s), Some((70, 0)), "lifted and floating after release");
    assert_eq!(rows(&s), r0 + 1);
    drag(&mut s, (200.0, 110.0), (170.0, 110.0), false, false);
    assert_eq!(floating(&s), Some((40, 0)), "the block moves back over");
    assert_eq!(rows(&s), r0 + 2, "each drag one row");
    drag(&mut s, (170.0, 110.0), (170.0, 210.0), false, false);
    assert_eq!(floating(&s), Some((40, 100)));
    // the area the block covered at 160..200 x 95..135 is intact (nothing travelled with it)
    for (x, y) in [(165, 100), (190, 120), (199, 134)] {
        assert_eq!(px(&s, id, x, y), orig(&img, x, y), "{x},{y} intact");
    }
    ex(&mut s, "select.none", json!({}));
    let out = image(&s, id);
    for (x, y) in [(165, 100), (190, 120), (199, 134), (250, 200)] {
        assert_eq!(out.get(x, y), img.get(x, y), "{x},{y} intact after the merge");
    }
    // the hole stays transparent, the block landed at +40, +100
    assert_eq!(out.get(125, 100).map(|p| p[3]), Some(0), "hole");
    assert_eq!(out.get(165, 200), img.get(125, 100), "block at its place");
}

/// RE spec 80 section 4: what merges and what does not.
#[test]
fn fb137_merge_triggers() {
    let lifted = || {
        let (mut s, id, img) = setup();
        marquee(&mut s, [120, 95, 180, 145]);
        s.set_tool(ToolId::Marquee).expect("tool");
        drag(&mut s, (150.0, 120.0), (210.0, 120.0), true, false);
        assert_eq!(floating(&s), Some((60, 0)));
        (s, id, img)
    };
    // no merge: tool changes, Enter, a Pointer click into empty canvas, a marquee drag
    let (mut s, id, _) = lifted();
    // (choosing the Brush may write its "Stroke Color" row; the block floats on)
    for t in [ToolId::Pointer, ToolId::Brush, ToolId::Lasso, ToolId::Marquee] {
        s.set_tool(t).expect("tool");
        assert_eq!(floating(&s), Some((60, 0)), "{t:?} keeps the block");
    }
    let r = rows(&s);
    key(&mut s, ToolKey::Enter, false, false, false);
    s.set_tool(ToolId::Pointer).expect("tool");
    events(&mut s, &[ToolEvent::down(20.0, 20.0), ToolEvent::up(20.0, 20.0)]);
    assert_eq!(floating(&s), Some((60, 0)), "Pointer click into empty canvas");
    assert_eq!(rows(&s), r, "no rows");
    // the Marquee without a key drags the floating block (RE spec 80 D6)
    s.set_tool(ToolId::Marquee).expect("tool");
    drag(&mut s, (200.0, 120.0), (200.0, 130.0), false, false);
    assert_eq!(floating(&s), Some((60, 10)), "the block moved, not the marquee");
    assert_eq!(px(&s, id, 200, 120)[3], 255, "bitmap untouched under the block");

    // Ctrl+D: merged in the row "Deselect"
    let (mut s, id, img) = lifted();
    let r = rows(&s);
    ex(&mut s, "select.none", json!({}));
    assert_eq!(floating(&s), None);
    assert_eq!(rows(&s), r + 1, "no extra row");
    assert_eq!(px(&s, id, 190, 100), orig(&img, 130, 100), "merged");
    // undo of the merging row: floating again
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(floating(&s), Some((60, 0)), "undo brings the block back");
    assert_eq!(px(&s, id, 190, 100), orig(&img, 190, 100), "unmerged");

    // a new selection merges, one row
    let (mut s, id, img) = lifted();
    let r = rows(&s);
    marquee(&mut s, [0, 0, 10, 10]);
    assert_eq!(floating(&s), None);
    assert_eq!(rows(&s), r + 1);
    assert_eq!(px(&s, id, 190, 100), orig(&img, 130, 100));

    // exit bitmap mode merges in its row
    let (mut s, id, img) = lifted();
    ex(&mut s, "modify.exitBitmapMode", json!({"row": true}));
    assert_eq!(floating(&s), None);
    assert_eq!(px(&s, id, 190, 100), orig(&img, 130, 100));
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(floating(&s), Some((60, 0)), "undo of exit bitmap mode floats again");

    // any other edit merges first (safe default), in its own row
    let (mut s, id, img) = lifted();
    let r = rows(&s);
    ex(&mut s, "element.setOpacity", json!({"opacity": 50}));
    assert_eq!(floating(&s), None, "merged before the edit");
    assert_eq!(rows(&s), r + 1);
    assert_eq!(px(&s, id, 190, 100), orig(&img, 130, 100));

    // Delete discards the block
    let (mut s, id, _) = lifted();
    ex(&mut s, "edit.clear", json!({}));
    assert_eq!(floating(&s), None, "Delete discards the block");
    assert_eq!(px(&s, id, 190, 100)[3], 255, "the bitmap under it stays");
    assert_eq!(px(&s, id, 150, 100)[3], 0, "the hole stays");
    assert_eq!(pixel_bounds(&s), Some(IRect::new(180, 95, 240, 145)), "marquee at the block");
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(floating(&s), Some((60, 0)), "undo of Delete brings the block back");
}

/// RE spec 80 section 7: saving merges first in a row of its own "Edit Bitmap" and drops the
/// pixel selection; the file holds the merged bitmap; undo brings the block back (Firepixel).
#[test]
fn fb137_save_merges_in_a_row_of_its_own() {
    let (mut s, id, img) = setup();
    marquee(&mut s, [120, 95, 180, 145]);
    s.set_tool(ToolId::Marquee).expect("tool");
    drag(&mut s, (150.0, 120.0), (210.0, 120.0), true, false);
    let dir = std::env::temp_dir().join(format!("fp-fb137-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("dir");
    let path = dir.join("float.fw.png");
    let r = rows(&s);
    ex(
        &mut s,
        "file.saveAs",
        json!({"path": path.to_string_lossy()}),
    );
    assert_eq!(floating(&s), None, "merged");
    assert_eq!(pixel_bounds(&s), None, "pixel selection gone");
    assert_eq!(rows(&s), r + 1, "one row");
    assert_eq!(label(&s), fp_i18n::t("msg-step-edit-bitmap"));
    assert_eq!(px(&s, id, 190, 100), orig(&img, 130, 100));
    // the saved file holds the merged pixels
    let mut s2 = Session::default();
    ex(&mut s2, "file.open", json!({"path": path.to_string_lossy()}));
    let mut found = None;
    for e in top_elements(&s2) {
        if let ElementKind::Bitmap(b) = &e.kind {
            assert!(found.is_none(), "one bitmap");
            found = fp_engine::docops::bitmap_image(b);
        }
    }
    let f = found.expect("one bitmap in the file");
    assert_eq!(f.get(190, 100), img.get(130, 100), "merged in the file");
    assert_eq!(f.get(150, 100).map(|p| p[3]), Some(0), "hole in the file");
    let _ = std::fs::remove_dir_all(&dir);
}

/// RE spec 80 section 6.3: with a soft selection the hole keeps B * (1 - M) and the block
/// B * M; the merge composites source-over.
#[test]
fn fb137_feathered_selection() {
    let (mut s, id, img) = setup();
    ex(
        &mut s,
        "select.pixels",
        json!({"shape": "rect", "rect": [120, 95, 180, 145], "feather": 10}),
    );
    let m = s
        .active()
        .and_then(|d| d.selection.pixels.clone())
        .expect("mask");
    s.set_tool(ToolId::Marquee).expect("tool");
    drag(&mut s, (150.0, 120.0), (210.0, 120.0), true, false);
    assert_eq!(floating(&s), Some((60, 0)));
    for x in [112, 116, 120, 124, 128, 140] {
        let k = m.get(x, 120) as u32;
        let a = (255 * (255 - k) + 127) / 255;
        assert_eq!(px(&s, id, x, 120)[3] as u32, a, "hole alpha at {x}: 1 - M");
    }
    // merged: the block (B * M) over the rest of the bitmap at the target
    ex(&mut s, "select.none", json!({}));
    for x in [182, 186, 190, 200] {
        let k = m.get(x - 60, 120) as u32;
        let under = img.get(x, 120).expect("pixel");
        let rest_a = (under[3] as u32 * (255 - m.get(x, 120) as u32) + 127) / 255;
        let sa = (255 * k + 127) / 255;
        let da = rest_a * (255 - sa) / 255;
        let got = px(&s, id, x, 120);
        assert_eq!(got[3] as u32, (sa + da).min(255), "alpha at {x}");
    }
}

/// RE spec 80 section 6.4: the block keeps all its pixels while it floats (also past the canvas);
/// the merge cuts at the canvas edge, the bitmap grows up to it.
#[test]
fn fb137_canvas_edge_cut_on_merge() {
    let (mut s, id, img) = setup();
    marquee(&mut s, [240, 100, 300, 140]);
    s.set_tool(ToolId::Marquee).expect("tool");
    drag(&mut s, (250.0, 120.0), (380.0, 120.0), true, false);
    assert_eq!(floating(&s), Some((130, 0)));
    // back before the merge: nothing was lost past the edge
    drag(&mut s, (380.0, 120.0), (310.0, 120.0), false, false);
    assert_eq!(floating(&s), Some((60, 0)));
    let f = s
        .active()
        .and_then(|d| d.selection.floating.clone())
        .expect("block");
    assert_eq!(f.bounds(), IRect::new(300, 100, 360, 140), "all pixels kept");
    drag(&mut s, (310.0, 120.0), (380.0, 120.0), false, false);
    ex(&mut s, "select.none", json!({}));
    let out = image(&s, id);
    assert_eq!(out.bounds().right, 400, "grown up to the canvas edge only");
    assert_eq!(out.get(399, 120), img.get(269, 120), "last column inside the canvas");
}

/// RE spec 80 section 2.1: Alt drags move a copy; with a floating block the block is placed at
/// its old place and floats on (three copies after two Alt drags).
#[test]
fn fb137_alt_drags_copy_and_stamp() {
    let (mut s, id, img) = setup();
    marquee(&mut s, [120, 95, 140, 115]);
    s.set_tool(ToolId::Marquee).expect("tool");
    // Alt inside the marquee of a selection tool: a copy, the bitmap stays (E4b)
    drag(&mut s, (130.0, 100.0), (160.0, 100.0), false, true);
    assert_eq!(floating(&s), Some((30, 0)));
    assert!(image(&s, id) == img, "a copy leaves the bitmap");
    assert_eq!(label(&s), fp_i18n::t("msg-step-duplicate"));
    // Pointer + Alt with a floating block: placed at the old place, floats on (E3)
    s.set_tool(ToolId::Pointer).expect("tool");
    drag(&mut s, (155.0, 100.0), (185.0, 100.0), false, true);
    assert_eq!(floating(&s), Some((60, 0)));
    assert_eq!(px(&s, id, 155, 100), orig(&img, 125, 100), "stamped copy");
    ex(&mut s, "select.none", json!({}));
    for x in [125, 155, 185] {
        assert_eq!(px(&s, id, x, 100), orig(&img, 125, 100), "copy at {x}");
    }
}

/// RE spec 80 section 3: arrows move the marquee alone until pixels float, then the block; Ctrl
/// lifts with the selection tools, Alt lifts a copy with every tool; a mixed sequence is one row.
#[test]
fn fb137_arrow_key_rules() {
    let (mut s, id, img) = setup();
    marquee(&mut s, [120, 95, 180, 145]);
    s.set_tool(ToolId::Marquee).expect("tool");
    let r0 = rows(&s);
    for _ in 0..3 {
        key(&mut s, ToolKey::Right, false, false, false);
    }
    assert_eq!(floating(&s), None, "plain arrows move the marquee");
    assert_eq!(pixel_bounds(&s), Some(IRect::new(123, 95, 183, 145)));
    assert!(image(&s, id) == img);
    key(&mut s, ToolKey::Right, false, true, false);
    key(&mut s, ToolKey::Right, false, true, false);
    assert_eq!(floating(&s), Some((2, 0)), "Ctrl lifts from the marquee's place");
    assert_eq!(px(&s, id, 123, 100)[3], 0, "the hole is where the marquee was");
    key(&mut s, ToolKey::Down, true, false, true);
    assert_eq!(floating(&s), Some((2, 10)), "Alt+Shift: placed and on by 10");
    assert_eq!(rows(&s), r0 + 1, "the whole sequence is one row");
    assert_eq!(label(&s), fp_i18n::t("msg-step-duplicate"), "named after the last press");
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(floating(&s), None);
    assert!(image(&s, id) == img, "one undo takes the sequence back");
    assert_eq!(pixel_bounds(&s), Some(IRect::new(120, 95, 180, 145)));

    // the Pointer: plain arrows move the marquee, Alt lifts a copy, then arrows move the block
    let (mut s, id, img) = setup();
    marquee(&mut s, [120, 95, 180, 145]);
    s.set_tool(ToolId::Pointer).expect("tool");
    key(&mut s, ToolKey::Left, false, false, false);
    assert_eq!(floating(&s), None);
    key(&mut s, ToolKey::Right, false, false, true);
    assert_eq!(floating(&s), Some((1, 0)), "Alt+arrow lifts a copy");
    assert!(image(&s, id) == img, "copy: the bitmap stays");
    key(&mut s, ToolKey::Right, true, false, false);
    assert_eq!(floating(&s), Some((11, 0)), "then the arrows move the block");
}

/// FB-94 (Firepixel option) with the floating model: on, a plain arrow with a pointer-type tool
/// lifts the pixels into a floating block; off, it moves the marquee.
#[test]
fn fb137_fb94_option_lifts_with_plain_arrows() {
    for on in [true, false] {
        let (mut s, id, img) = setup();
        s.prefs
            .set(ARROW_KEYS_MOVE_PIXELS_PREF, PrefValue::Bool(on));
        marquee(&mut s, [120, 95, 180, 145]);
        s.set_tool(ToolId::Pointer).expect("tool");
        key(&mut s, ToolKey::Right, false, false, false);
        if on {
            assert_eq!(floating(&s), Some((1, 0)), "on: lifted");
            assert_eq!(px(&s, id, 120, 100)[3], 0, "hole");
        } else {
            assert_eq!(floating(&s), None, "off: marquee only");
            assert!(image(&s, id) == img);
        }
    }
}

/// Scripts keep the one-shot `select.movePixels` (move and merge at once), cut at the canvas.
#[test]
fn fb137_script_move_pixels_stays_one_shot() {
    let (mut s, id, img) = setup();
    marquee(&mut s, [120, 95, 180, 145]);
    ex(&mut s, "select.movePixels", json!({"dx": 60, "dy": 0}));
    assert_eq!(floating(&s), None, "no floating block");
    assert_eq!(px(&s, id, 190, 100), orig(&img, 130, 100), "merged at once");
    ex(&mut s, "select.movePixels", json!({"dx": 200, "dy": 0}));
    assert_eq!(image(&s, id).bounds().right, 400, "cut at the canvas edge");
}

/// Commands that change neither the document nor the selection keep the block (they see the
/// merged pixels): Copy puts the merged pixels on the clipboard, the block floats on.
#[test]
fn fb137_copy_sees_the_merged_pixels_and_keeps_the_block() {
    let (mut s, _id, img) = setup();
    marquee(&mut s, [120, 95, 180, 145]);
    s.set_tool(ToolId::Marquee).expect("tool");
    drag(&mut s, (150.0, 120.0), (160.0, 120.0), true, false);
    ex(&mut s, "edit.copy", json!({}));
    assert_eq!(floating(&s), Some((10, 0)), "still floating");
    let c = s.clipboard.pixels.clone().expect("pixels");
    assert_eq!(c.get(135, 100), img.get(125, 100), "block pixels on the clipboard");
}



/// The bitmap `id` of a document copy rendered with `preview_floating` (the canvas preview).
fn preview(s: &Session, id: ElementId, d: (i32, i32), copy: bool) -> RgbaImage {
    let st = s.active().expect("document");
    let mut doc = (*st.doc).clone();
    fp_engine::commands::preview_floating(&mut doc, st.page, id, &st.selection, d, copy);
    let e = fp_engine::docops::element_on_page(&doc, st.page, id).expect("bitmap");
    let ElementKind::Bitmap(b) = &e.kind else {
        panic!("not a bitmap")
    };
    fp_engine::docops::bitmap_image(b).expect("image")
}

/// The canvas preview (tile by tile inside the bitmap, whole image otherwise) equals what the
/// release and the merge make: lift drags, floating drags, Alt drags, soft edges, past the edge.
#[test]
fn fb137_preview_equals_the_result() {
    for feather in [0, 6] {
        for (d, copy) in [((37, 21), false), ((-90, 130), false), ((15, -4), true), ((150, 0), false)] {
            // a lift drag: the preview equals the merged one-shot move
            let (mut s, id, _) = setup();
            ex(
                &mut s,
                "select.pixels",
                json!({"shape": "rect", "rect": [110, 85, 250, 215], "feather": feather}),
            );
            let p = preview(&s, id, d, copy);
            ex(
                &mut s,
                "select.floatPixels",
                json!({"dx": d.0, "dy": d.1, "copy": copy}),
            );
            let at_rest = preview(&s, id, (0, 0), false);
            assert!(p == at_rest, "lift preview = floating block at rest {d:?} {copy} f{feather}");
            // a floating drag on (Alt: placed and moved on)
            let p2 = preview(&s, id, (-20, 9), copy);
            ex(
                &mut s,
                "select.floatPixels",
                json!({"dx": -20, "dy": 9, "copy": copy}),
            );
            let p3 = preview(&s, id, (0, 0), false);
            assert!(p2 == p3, "floating drag preview = result {d:?} {copy} f{feather}");
            ex(&mut s, "select.none", json!({}));
            let merged = image(&s, id);
            // the merge cuts at the canvas, the preview does not: compare inside the canvas
            let r = merged.bounds().intersect(&p3.bounds());
            for y in r.top..r.bottom {
                for x in r.left..r.right {
                    assert_eq!(merged.get(x, y), p3.get(x, y), "merge = preview at {x},{y}");
                }
            }
        }
    }
}

/// Esc while a floating block is dragged ends bitmap mode like Esc without a drag (re 62 section
/// 1.5): the block is merged where it was before the drag, the release does nothing.
#[test]
fn fb137_escape_during_a_floating_drag() {
    let (mut s, id, img) = setup();
    marquee(&mut s, [120, 95, 180, 145]);
    s.set_tool(ToolId::Marquee).expect("tool");
    drag(&mut s, (150.0, 120.0), (210.0, 120.0), true, false);
    events(
        &mut s,
        &[ToolEvent::down(200.0, 120.0), ToolEvent::drag(230.0, 150.0)],
    );
    key(&mut s, ToolKey::Escape, false, false, false);
    assert_eq!(s.tools.active, ToolId::Pointer);
    assert_eq!(floating(&s), None, "merged");
    events(&mut s, &[ToolEvent::up(230.0, 150.0)]);
    assert_eq!(floating(&s), None, "the release does nothing");
    assert_eq!(px(&s, id, 190, 100), orig(&img, 130, 100), "merged where it was");
}

/// RE spec 80 §5: a drag of the selected pixels (lift and further drags of the floating block) is
/// the plain move row ("Verschieben" / "Move"), an Alt drag "Duplicate", in either language, also
/// when the History panel shows the row in the other language (F165: in English the row of a
/// drag made in German read "Move Floating Pixels").
#[test]
fn fb137_floating_drag_rows_have_the_plain_move_name() {
    // the History panel's name of the last row in both languages
    fn shown(s: &Session) -> (String, String) {
        let d = s.active().expect("document");
        let step = d.history.top().expect("row");
        let lang = fp_i18n::current_language();
        fp_i18n::set_language("de");
        let de = s.step_label(step);
        fp_i18n::set_language("en");
        let en = s.step_label(step);
        fp_i18n::set_language(&lang);
        (de, en)
    }
    let mv = ("Verschieben".to_string(), "Move".to_string());
    let dup = ("Duplizieren".to_string(), "Duplicate".to_string());
    for lang in ["en", "de"] {
        fp_i18n::set_language(lang);
        let (mut s, _id, _img) = setup();
        marquee(&mut s, [120, 95, 160, 135]);
        s.set_tool(ToolId::Pointer).expect("tool");
        drag(&mut s, (130.0, 100.0), (200.0, 100.0), false, false);
        assert_eq!(shown(&s), mv, "{lang}: lift");
        drag(&mut s, (200.0, 110.0), (170.0, 110.0), false, false);
        assert_eq!(shown(&s), mv, "{lang}: block dragged on");
        ex(&mut s, "select.floatPixels", json!({"dx": 5, "dy": 0}));
        assert_eq!(shown(&s), mv, "{lang}: command");
        ex(&mut s, "select.floatPixels", json!({"dx": 5, "dy": 0, "copy": true}));
        assert_eq!(shown(&s), dup, "{lang}: copy");
    }
    fp_i18n::set_language("de");
}
