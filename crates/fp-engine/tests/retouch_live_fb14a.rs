//! FB-14a / TW-182 (RE spec 57 section 1): all retouch tools (Blur, Sharpen, Dodge, Burn, Smudge,
//! Rubber Stamp, Replace Color) show their effect on the canvas while the button is held: the
//! running stroke is computed live (preview tiles like the brush), the last preview equals the
//! result of `bitmap.retouch`, so releasing changes nothing more; one history step per stroke; no
//! tip outline per stroke point and only the one tip at the pointer. Also with "Entire document"
//! and across the bitmap edge, where the smudge grows the bitmap (FB-15).

use crate::common;
use common::*;
use fp_engine::tools::ToolId;
use fp_engine::{Overlay, PrefValue, Session, ToolEvent};
use fp_geom::IRect;
use fp_model::{ElementId, ElementKind};
use serde_json::json;

fn image(s: &Session, id: ElementId) -> fp_raster::RgbaImage {
    match &el(s, id).kind {
        ElementKind::Bitmap(b) => fp_engine::docops::bitmap_image(b).expect("image"),
        _ => panic!("not a bitmap"),
    }
}

/// Bitmap with hard edges, a gradient and green stripes (Replace Color's default source colour).
fn bitmap(s: &mut Session) -> ElementId {
    let r = IRect::new(20, 20, 380, 280);
    let mut img = fp_raster::RgbaImage::new(r).expect("image");
    for y in r.top..r.bottom {
        for x in r.left..r.right {
            let c = if (x / 12) % 3 == 0 {
                [0, 255, 0, 255]
            } else if ((x / 9) + (y / 9)) % 2 == 0 {
                [230, 40, (y % 256) as u8, 255]
            } else {
                [(x % 256) as u8, 90, 20, 255]
            };
            img.set_pixel(x, y, fp_color::Rgba8::new(c[0], c[1], c[2], c[3]));
        }
    }
    s.clipboard.clear();
    s.clipboard.pixels = Some(img);
    let v = ex(s, "edit.paste", json!({}));
    ElementId(v["ids"][0].as_u64().expect("id"))
}

/// What the canvas shows for (x, y) with the preview `p` over the bitmap `base`, resolved like the
/// in-place composite does it: a transparent tile pixel keeps the bitmap, a shown one replaces it
/// (its true value from `exact` where the tile shows a backdrop mix).
fn seen(p: &fp_engine::paint::PaintPreview, base: &fp_raster::RgbaImage, x: i32, y: i32) -> [u8; 4] {
    let orig = base.get(x, y).unwrap_or([0; 4]);
    let Some(t) = p.tiles.iter().find(|t| t.rect.contains(x, y)) else {
        return orig;
    };
    let v = t.image.get(x, y).unwrap_or([0; 4]);
    if v[3] == 0 {
        return orig;
    }
    t.exact.as_ref().and_then(|e| e.get(x, y)).unwrap_or(v)
}

/// Fully transparent pixels count as equal, whatever their colour.
fn norm(p: [u8; 4]) -> [u8; 4] {
    if p[3] == 0 { [0; 4] } else { p }
}

fn strokes_in(fb: &[Overlay]) -> usize {
    fb.iter()
        .filter(|o| matches!(o, Overlay::Stroke { .. }))
        .count()
}

fn tips_in(fb: &[Overlay]) -> usize {
    fb.iter()
        .filter(|o| matches!(o, Overlay::BrushTip { .. }))
        .count()
}

/// Press, many small moves (one per frame), release; returns the last live preview.
fn stroke(
    s: &mut Session,
    pts: &[(f32, f32)],
    tool: ToolId,
) -> Option<fp_engine::paint::PaintPreview> {
    let mut last = None;
    events(s, &[ToolEvent::down(pts[0].0, pts[0].1)]);
    let mut frames = 0;
    for w in pts.windows(2) {
        let (a, b) = (w[0], w[1]);
        let len = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
        let k = (len / 3.0).ceil().max(1.0) as usize;
        for i in 1..=k {
            let t = i as f32 / k as f32;
            events(
                s,
                &[ToolEvent::drag(
                    a.0 + (b.0 - a.0) * t,
                    a.1 + (b.1 - a.1) * t,
                )],
            );
            frames += 1;
            let fb = s.tool_feedback();
            assert_eq!(
                strokes_in(&fb),
                0,
                "{tool:?}: no tip outline per stroke point (frame {frames})"
            );
            assert_eq!(
                tips_in(&fb),
                1,
                "{tool:?}: one tip at the pointer while dragging (frame {frames})"
            );
            if let Some(Overlay::Paint(p)) = fb.into_iter().find(|o| matches!(o, Overlay::Paint(_)))
            {
                last = Some(p);
            }
        }
    }
    let e = pts[pts.len() - 1];
    events(s, &[ToolEvent::up(e.0, e.1)]);
    last
}

/// Runs `pts` with `tool` on the bitmap `id` and checks: live preview, one step, the last preview
/// equals the result everywhere (also on pixels the stroke grew past the bitmap edge), nothing
/// left after the release. Returns the number of changed pixels.
fn check_stroke(s: &mut Session, id: ElementId, tool: ToolId, pts: &[(f32, f32)], n: usize) -> usize {
    let base = image(s, id);
    let steps0 = s.active().expect("document").history.undo_count();
    let p = stroke(s, pts, tool)
        .unwrap_or_else(|| panic!("{tool:?}: live preview while dragging (stroke {n})"));
    assert_eq!(
        s.active().expect("document").history.undo_count(),
        steps0 + 1,
        "{tool:?}: one step per stroke"
    );
    let res = image(s, id);
    let (mut changed, mut diff) = (0, 0);
    let b = base.bounds().union(&res.bounds());
    for y in b.top..b.bottom {
        for x in b.left..b.right {
            let r = norm(res.get(x, y).unwrap_or([0; 4]));
            let o = norm(base.get(x, y).unwrap_or([0; 4]));
            if r != o {
                changed += 1;
            }
            if norm(seen(&p, &base, x, y)) != r {
                diff += 1;
            }
        }
    }
    assert_eq!(
        diff, 0,
        "{tool:?}: the last preview equals the result (stroke {n}, {changed} changed)"
    );
    // after the release: no preview, no stroke circles, only the tip at the pointer
    events(s, &[ToolEvent::drag(350.0, 260.0)]);
    let fb = s.tool_feedback();
    assert_eq!(strokes_in(&fb), 0, "{tool:?}: nothing of the stroke stays: {fb:?}");
    assert!(
        !fb.iter().any(|o| matches!(o, Overlay::Paint(_))),
        "{tool:?}: no preview after the release"
    );
    assert_eq!(tips_in(&fb), 1, "{tool:?}: exactly one tip: {fb:?}");
    changed
}

#[test]
fn retouch_strokes_preview_live_and_leave_one_tip() {
    for tool in [
        ToolId::Blur,
        ToolId::Sharpen,
        ToolId::Dodge,
        ToolId::Burn,
        ToolId::Smudge,
        ToolId::ReplaceColor,
    ] {
        let mut s = doc_session(400.0, 300.0);
        let id = bitmap(&mut s);
        s.set_tool(tool).unwrap_or_else(|e| panic!("{e}"));
        for (n, pts) in [
            [(60.0, 80.0), (160.0, 120.0), (260.0, 90.0)],
            [(80.0, 200.0), (200.0, 170.0), (330.0, 230.0)],
        ]
        .iter()
        .enumerate()
        {
            let changed = check_stroke(&mut s, id, tool, pts, n);
            assert!(changed > 50, "{tool:?}: the stroke changes pixels ({changed})");
        }
    }
}

#[test]
fn stamp_and_entire_document_smudge_preview_live() {
    // rubber stamp: Alt+click sets the source, then a stroke
    let mut s = doc_session(400.0, 300.0);
    let id = bitmap(&mut s);
    select(&mut s, &[id]);
    s.set_tool(ToolId::RubberStamp).unwrap_or_else(|e| panic!("{e}"));
    let mut alt = ToolEvent::down(100.0, 100.0);
    alt.mods.alt = true;
    events(&mut s, &[alt, ToolEvent::up(100.0, 100.0)]);
    let changed = check_stroke(&mut s, id, ToolId::RubberStamp, &[(200.0, 150.0), (300.0, 180.0)], 0);
    assert!(changed > 50, "stamp changes pixels ({changed})");
    // smudge with "Entire document": the page is rendered at the press for the preview
    let mut s = doc_session(400.0, 300.0);
    let id = bitmap(&mut s);
    select(&mut s, &[id]);
    s.prefs.set("SmudgeToolUseEntireDocument", PrefValue::Bool(true));
    s.set_tool(ToolId::Smudge).unwrap_or_else(|e| panic!("{e}"));
    let changed = check_stroke(&mut s, id, ToolId::Smudge, &[(60.0, 80.0), (160.0, 120.0)], 0);
    assert!(changed > 50, "entire document smudge changes pixels ({changed})");
}

#[test]
fn smudge_across_the_edge_previews_the_grown_bitmap() {
    for whole in [false, true] {
        let mut s = doc_session(400.0, 300.0);
        let id = bitmap(&mut s);
        select(&mut s, &[id]);
        s.prefs.set("SmudgeToolUseEntireDocument", PrefValue::Bool(whole));
        s.set_tool(ToolId::Smudge).unwrap_or_else(|e| panic!("{e}"));
        let before = image(&s, id).bounds();
        check_stroke(&mut s, id, ToolId::Smudge, &[(340.0, 150.0), (395.0, 150.0)], 0);
        let after = image(&s, id).bounds();
        assert!(
            after.right > before.right,
            "entire document {whole}: the bitmap grows ({before:?} -> {after:?})"
        );
    }
}
