//! FB-14b / TW-181: the retouch tools (Blur, Sharpen, Dodge, Burn, Smudge, Replace Color) show
//! exactly one tip outline, at the pointer, while dragging and after the release; they no longer
//! push one tip outline per stroke point (`Overlay::Stroke`), and nothing of a stroke stays behind
//! (RE-25: the original shows a single tip at all times).

use crate::common;
use common::*;
use fp_engine::tools::ToolId;
use fp_engine::{Overlay, Session, ToolEvent};
use fp_geom::IRect;
use fp_model::{ElementId, ElementKind};
use serde_json::json;

fn image(s: &Session, id: ElementId) -> fp_raster::RgbaImage {
    match &el(s, id).kind {
        ElementKind::Bitmap(b) => fp_engine::docops::bitmap_image(b).expect("image"),
        _ => panic!("not a bitmap"),
    }
}

/// Bitmap with hard edges and green stripes (Replace Color's default source colour).
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

/// Tip outlines of stroke points (`Overlay::Stroke` draws one circle per point).
fn stroke_circles(fb: &[Overlay]) -> usize {
    fb.iter()
        .map(|o| match o {
            Overlay::Stroke { points, .. } => points.len(),
            _ => 0,
        })
        .sum()
}

fn tips_in(fb: &[Overlay]) -> usize {
    fb.iter()
        .filter(|o| matches!(o, Overlay::BrushTip { .. }))
        .count()
}

/// Press, many small moves (one per frame), release. Returns the most stroke-point circles and
/// the most tips seen in one frame while dragging.
fn stroke(s: &mut Session, pts: &[(f32, f32)]) -> (usize, usize) {
    let (mut circles, mut tips) = (0, 0);
    events(s, &[ToolEvent::down(pts[0].0, pts[0].1)]);
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
            let fb = s.tool_feedback();
            circles = circles.max(stroke_circles(&fb));
            tips = tips.max(tips_in(&fb));
        }
    }
    let e = pts[pts.len() - 1];
    events(s, &[ToolEvent::up(e.0, e.1)]);
    (circles, tips)
}

#[test]
fn retouch_strokes_show_one_tip_and_leave_nothing_behind() {
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
            let base = image(&s, id);
            let steps0 = s.active().expect("document").history.undo_count();
            let (circles, tips) = stroke(&mut s, pts);
            // after the release (checked first: the stroke trail is cleared on mouse-up)
            let fb = s.tool_feedback();
            assert_eq!(
                stroke_circles(&fb),
                0,
                "{tool:?}: nothing of stroke {n} stays after the release: {fb:?}"
            );
            assert_eq!(
                tips_in(&fb),
                1,
                "{tool:?}: one tip after the release: {fb:?}"
            );
            events(&mut s, &[ToolEvent::drag(350.0, 260.0)]);
            let fb = s.tool_feedback();
            assert_eq!(stroke_circles(&fb), 0, "{tool:?}: nothing stays: {fb:?}");
            assert_eq!(
                tips_in(&fb),
                1,
                "{tool:?}: exactly one tip at the pointer: {fb:?}"
            );
            assert_eq!(
                s.active().expect("document").history.undo_count(),
                steps0 + 1,
                "{tool:?}: one step per stroke"
            );
            assert_ne!(
                image(&s, id),
                base,
                "{tool:?}: stroke {n} changes the bitmap"
            );
            // while dragging
            assert_eq!(
                circles, 0,
                "{tool:?}: no tip outline per stroke point while dragging (stroke {n})"
            );
            assert_eq!(tips, 1, "{tool:?}: one tip at the pointer while dragging");
        }
    }
}
