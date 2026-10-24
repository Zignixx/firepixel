//! FB-39: the retouch tools (Blur, Sharpen, Dodge, Burn, Smudge) must stay responsive with large
//! tips and long strokes. The live preview works incrementally: each pointer move computes only
//! the part of the bitmap the new dabs can change (and re-tiles only that part), instead of the
//! whole stroke from its first dab. Release computes the stroke once more with `bitmap.retouch`.
//!
//! `retouch_live_preview_timing` is a timing run (ignored; run it with
//! `cargo test -p fp-engine --test all retouch_perf_fb39 -- --ignored --nocapture`): tip 100 px,
//! bitmap 1000 x 800, 200 pointer moves. The always-on test checks the incremental property: the
//! area recomputed per move does not grow with the length of the stroke.

use crate::common;
use common::*;
use fp_engine::tools::ToolId;
use fp_engine::{Overlay, PrefValue, Session, ToolEvent};
use fp_geom::IRect;
use fp_model::ElementId;
use serde_json::json;
use std::time::{Duration, Instant};

/// Opaque 1000 x 800 bitmap at 0/0 with hard edges and a gradient.
fn big_bitmap(s: &mut Session) -> ElementId {
    let r = IRect::new(0, 0, 1000, 800);
    let mut img = fp_raster::RgbaImage::new(r).expect("image");
    for y in r.top..r.bottom {
        for x in r.left..r.right {
            let c = if ((x / 16) + (y / 16)) % 2 == 0 {
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

/// Pointer positions of a long zig-zag stroke: `n` moves of about 5 px.
fn path(n: usize) -> Vec<(f32, f32)> {
    (0..=n)
        .map(|i| {
            // back and forth between x 100 and 900
            let t = (i as f32 * 4.0) % 1600.0;
            let x = 100.0 + if t < 800.0 { t } else { 1600.0 - t };
            let y = 400.0 + ((i as f32) * 0.02).sin() * 250.0;
            (x, y)
        })
        .collect()
}

struct Timing {
    moves: Duration,
    worst: Duration,
    release: Duration,
}

fn run(tool: ToolId, size: i64, n: usize) -> Timing {
    let mut s = doc_session(1000.0, 800.0);
    let id = big_bitmap(&mut s);
    select(&mut s, &[id]);
    let key = match tool {
        ToolId::Blur => "BlurToolDiameter",
        ToolId::Sharpen => "SharpenToolDiameter",
        ToolId::Dodge => "DodgeToolDiameter",
        ToolId::Burn => "BurnToolDiameter",
        _ => "SmudgeToolDiameter",
    };
    s.prefs.set(key, PrefValue::Int(size));
    s.set_tool(tool).unwrap_or_else(|e| panic!("{e}"));
    let pts = path(n);
    events(&mut s, &[ToolEvent::down(pts[0].0, pts[0].1)]);
    let (mut moves, mut worst) = (Duration::ZERO, Duration::ZERO);
    for p in &pts[1..] {
        let t = Instant::now();
        events(&mut s, &[ToolEvent::drag(p.0, p.1)]);
        // the canvas asks for the feedback once per frame
        let fb = s.tool_feedback();
        let d = t.elapsed();
        assert!(
            fb.iter().any(|o| matches!(o, Overlay::Paint(_))),
            "{tool:?}: live preview"
        );
        moves += d;
        worst = worst.max(d);
    }
    let e = pts[pts.len() - 1];
    let t = Instant::now();
    events(&mut s, &[ToolEvent::up(e.0, e.1)]);
    let release = t.elapsed();
    Timing {
        moves,
        worst,
        release,
    }
}

#[test]
#[ignore = "timing run, prints with --nocapture"]
fn retouch_live_preview_timing() {
    for tool in [
        ToolId::Blur,
        ToolId::Sharpen,
        ToolId::Dodge,
        ToolId::Burn,
        ToolId::Smudge,
    ] {
        let t = run(tool, 100, 200);
        println!(
            "FB-39 {tool:?}: tip 100, 200 moves: moves total {:.1} ms, per move {:.2} ms avg, worst {:.1} ms, release {:.1} ms",
            t.moves.as_secs_f64() * 1e3,
            t.moves.as_secs_f64() * 1e3 / 200.0,
            t.worst.as_secs_f64() * 1e3,
            t.release.as_secs_f64() * 1e3,
        );
    }
}

fn image(s: &Session, id: ElementId) -> fp_raster::RgbaImage {
    match &el(s, id).kind {
        fp_model::ElementKind::Bitmap(b) => fp_engine::docops::bitmap_image(b).expect("image"),
        _ => panic!("not a bitmap"),
    }
}

fn paint_preview(s: &mut Session) -> Option<fp_engine::paint::PaintPreview> {
    s.tool_feedback().into_iter().find_map(|o| match o {
        Overlay::Paint(p) => Some(p),
        _ => None,
    })
}

/// FB-39: far into a long stroke with a 100 px tip, a pointer move re-tiles only the tiles around
/// the pointer (the canvas uploads only those), not the whole stroke so far.
#[test]
fn a_move_late_in_a_long_stroke_changes_only_the_tiles_near_the_pointer() {
    for tool in [ToolId::Blur, ToolId::Dodge, ToolId::Smudge] {
        let mut s = doc_session(1000.0, 800.0);
        let id = big_bitmap(&mut s);
        select(&mut s, &[id]);
        let key = match tool {
            ToolId::Blur => "BlurToolDiameter",
            ToolId::Dodge => "DodgeToolDiameter",
            _ => "SmudgeToolDiameter",
        };
        s.prefs.set(key, PrefValue::Int(100));
        s.set_tool(tool).unwrap_or_else(|e| panic!("{e}"));
        let pts = path(160);
        events(&mut s, &[ToolEvent::down(pts[0].0, pts[0].1)]);
        let mut last_gen = 0;
        for (i, p) in pts[1..].iter().enumerate() {
            events(&mut s, &[ToolEvent::drag(p.0, p.1)]);
            let pv = paint_preview(&mut s).unwrap_or_else(|| panic!("{tool:?}: live preview"));
            assert!(
                pv.generation > last_gen,
                "{tool:?}: a new preview state per move"
            );
            last_gen = pv.generation;
            if i < 40 {
                continue;
            }
            let fresh = pv
                .tiles
                .iter()
                .filter(|t| t.version == pv.generation)
                .count();
            assert!(
                (1..=4).contains(&fresh),
                "{tool:?} move {i}: {fresh} of {} tiles changed",
                pv.tiles.len()
            );
            if i + 2 == pts.len() {
                assert!(pv.tiles.len() > 4, "{tool:?}: the stroke spans more tiles");
            }
        }
        let e = pts[pts.len() - 1];
        events(&mut s, &[ToolEvent::up(e.0, e.1)]);
    }
}

/// FB-39: the release hands the finished live stroke to `bitmap.retouch`; the stored bitmap is
/// exactly what the command computes on its own from the same parameters.
#[test]
fn the_tool_result_equals_the_command_computed_on_its_own() {
    for tool in [
        ToolId::Blur,
        ToolId::Sharpen,
        ToolId::Dodge,
        ToolId::Burn,
        ToolId::Smudge,
    ] {
        let mut s = doc_session(1000.0, 800.0);
        let id = big_bitmap(&mut s);
        select(&mut s, &[id]);
        s.set_tool(tool).unwrap_or_else(|e| panic!("{e}"));
        let pts: Vec<(f32, f32)> = (0..=80)
            .map(|i| (40.0 + i as f32 * 3.3, 60.0 + (i as f32 * 0.15).sin() * 40.0))
            .collect();
        let mut evs = vec![ToolEvent::down(pts[0].0, pts[0].1)];
        evs.extend(
            pts[1..pts.len() - 1]
                .iter()
                .map(|p| ToolEvent::drag(p.0, p.1)),
        );
        let e = pts[pts.len() - 1];
        evs.push(ToolEvent::up(e.0 + 0.5, e.1));
        events(&mut s, &evs);
        let got = image(&s, id);
        let top = s.active().expect("document").history.top().expect("step");
        assert_eq!(top.command, "bitmap.retouch");
        let params = top.params.clone();
        let mut t = doc_session(1000.0, 800.0);
        let id2 = big_bitmap(&mut t);
        select(&mut t, &[id2]);
        let before = image(&t, id2);
        ex(&mut t, "bitmap.retouch", params);
        let want = image(&t, id2);
        assert!(want != before, "{tool:?}: the stroke changes pixels");
        assert!(
            got == want,
            "{tool:?}: the stored bitmap equals the command's own result"
        );
    }
}
