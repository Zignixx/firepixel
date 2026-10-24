//! FB-40 (tester's screen recording, smudge tool, tip 59, edge 50, pressure 50, 400 % zoom): a
//! smudge stroke that starts outside the selected bitmap, runs through it and out at the far side
//! was cut off hard at the old bitmap rectangle, while the same stroke started on the bitmap grows
//! it. Without "Confine to boundaries" the smear now runs past the far edge and grows the bitmap
//! there too (dragging transparency in from outside still grows nothing); with the option the
//! stroke stays clipped. Long strokes at 400 % zoom (quarter-pixel pointer positions) keep the
//! live preview equal to the stored result.

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

/// Pixel-art bitmap 100 x 100 at 0/100: blue with a dark grid every 10 px.
fn pixel_art(s: &mut Session) -> ElementId {
    let r = IRect::new(0, 100, 100, 200);
    let mut img = fp_raster::RgbaImage::new(r).expect("image");
    for y in r.top..r.bottom {
        for x in r.left..r.right {
            let c = if x % 10 == 0 || y % 10 == 0 {
                [20, 20, 60, 255]
            } else {
                [0, 51, 204, 255]
            };
            img.set_pixel(x, y, fp_color::Rgba8::new(c[0], c[1], c[2], c[3]));
        }
    }
    s.clipboard.clear();
    s.clipboard.pixels = Some(img);
    let v = ex(s, "edit.paste", json!({}));
    ElementId(v["ids"][0].as_u64().expect("id"))
}

fn setup(confine: bool) -> (Session, ElementId) {
    let mut s = doc_session(300.0, 300.0);
    let id = pixel_art(&mut s);
    select(&mut s, &[id]);
    s.prefs.set("SmudgeToolDiameter", PrefValue::Int(22));
    s.prefs
        .set("SmudgeToolConfineToBounds", PrefValue::Bool(confine));
    s.set_tool(ToolId::Smudge).unwrap_or_else(|e| panic!("{e}"));
    (s, id)
}

/// Press, moves, release at the last point; returns the last live preview.
fn drag(s: &mut Session, pts: &[(f32, f32)]) -> Option<fp_engine::paint::PaintPreview> {
    events(s, &[ToolEvent::down(pts[0].0, pts[0].1)]);
    let mut last = None;
    for p in &pts[1..] {
        events(s, &[ToolEvent::drag(p.0, p.1)]);
        last = s.tool_feedback().into_iter().find_map(|o| match o {
            Overlay::Paint(p) => Some(p),
            _ => None,
        });
    }
    let e = pts[pts.len() - 1];
    events(s, &[ToolEvent::up(e.0, e.1)]);
    last
}

fn line(from: (f32, f32), to: (f32, f32), n: usize) -> Vec<(f32, f32)> {
    (0..=n)
        .map(|i| {
            let t = i as f32 / n as f32;
            (from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t)
        })
        .collect()
}

#[test]
fn smudge_from_below_through_the_bitmap_grows_it_past_the_top_edge() {
    let (mut s, id) = setup(false);
    drag(&mut s, &line((55.0, 240.0), (55.0, 50.0), 95));
    let b = image(&s, id).bounds();
    assert!(
        b.top < 95,
        "the smear runs past the top edge and grows the bitmap: {b:?}"
    );
    assert_eq!(
        b.bottom, 200,
        "dragging transparency in from below grows nothing: {b:?}"
    );
    assert_eq!((b.left, b.right), (0, 100), "{b:?}");
    let img = image(&s, id);
    let p = img.get(55, 97).unwrap_or([0; 4]);
    assert!(
        p[3] > 60 && p[2] > p[0] + 60,
        "the bitmap's blue is carried over the edge: {p:?}"
    );
}

#[test]
fn confine_to_boundaries_keeps_the_stroke_from_outside_clipped() {
    let (mut s, id) = setup(true);
    drag(&mut s, &line((50.0, 240.0), (50.0, 50.0), 95));
    assert_eq!(image(&s, id).bounds(), IRect::new(0, 100, 100, 200));
}

#[test]
fn smudge_entirely_outside_still_changes_nothing() {
    let (mut s, id) = setup(false);
    let base = image(&s, id);
    drag(&mut s, &line((150.0, 250.0), (250.0, 280.0), 50));
    assert_eq!(image(&s, id), base, "no pixel changed, no growth");
}

/// Long stroke at 400 % zoom (pointer positions on quarter pixels) back and forth across the
/// bitmap edges: the last live preview equals the stored bitmap everywhere.
#[test]
fn long_smudge_stroke_at_400_percent_preview_equals_result() {
    for confine in [false, true] {
        let (mut s, id) = setup(confine);
        ex(&mut s, "view.setZoom", json!({"zoom": 4.0}));
        let base = image(&s, id);
        let pts: Vec<(f32, f32)> = (0..=600)
            .map(|i| {
                let t = i as f32 * 0.25;
                let x = 50.0 + (t * 0.09).sin() * 75.0;
                let y = 150.0 + (t * 0.05).cos() * 70.0;
                ((x * 4.0).round() / 4.0, (y * 4.0).round() / 4.0)
            })
            .collect();
        let p = drag(&mut s, &pts).expect("live preview");
        let res = image(&s, id);
        let norm = |p: [u8; 4]| if p[3] == 0 { [0; 4] } else { p };
        let u = res.bounds().union(&base.bounds());
        let mut diff = 0;
        for y in u.top..u.bottom {
            for x in u.left..u.right {
                let orig = base.get(x, y).unwrap_or([0; 4]);
                let seen = match p.tiles.iter().find(|t| t.rect.contains(x, y)) {
                    Some(t) => {
                        let v = t.image.get(x, y).unwrap_or([0; 4]);
                        if v[3] == 0 {
                            orig
                        } else {
                            t.exact.as_ref().and_then(|e| e.get(x, y)).unwrap_or(v)
                        }
                    }
                    None => orig,
                };
                if norm(seen) != norm(res.get(x, y).unwrap_or([0; 4])) {
                    diff += 1;
                }
            }
        }
        assert_eq!(diff, 0, "confine {confine}: preview equals the result");
        assert_eq!(
            res.bounds() != base.bounds(),
            !confine,
            "confine {confine}: grows only without the option: {:?}",
            res.bounds()
        );
    }
}
