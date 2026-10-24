//! FB-15 / TW-183 (RE spec 57 section 3): a smudge stroke that starts on a bitmap and runs past
//! its edge carries the colour about 20 px beyond the edge, fading out semi-transparent, and the
//! bitmap grows to those pixels (measured 360 -> 384 for a stroke ending 120 px outside with tip
//! size 22); with "Entire document" the page is smeared in and the bitmap grows to the end of the
//! stroke plus the tip radius. The Firepixel option "Confine to boundaries" (`confineToBounds`)
//! keeps the old clipped behaviour. A stroke entirely outside the selected bitmap changes nothing
//! but still writes one "Smudge" step (with "Entire document" its rectangle joins the bitmap,
//! FB-51); with no bitmap at all nothing happens. The blur grows a
//! bitmap by up to 2 px.

use crate::common;
use common::*;
use fp_engine::Session;
use fp_engine::tools::ToolId;
use fp_engine::{Overlay, ToolEvent};
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
    let id = ElementId(v["ids"][0].as_u64().expect("id"));
    let b = image(s, id).bounds();
    assert_eq!(b, r, "pasted at its origin");
    id
}

fn smudge(s: &mut Session, from: (f32, f32), to: (f32, f32), extra: serde_json::Value) {
    let n = ((to.0 - from.0).hypot(to.1 - from.1) / 2.0).ceil().max(1.0) as usize;
    let pts: Vec<[f32; 2]> = (0..=n)
        .map(|i| {
            let t = i as f32 / n as f32;
            [
                from.0 + (to.0 - from.0) * t,
                from.1 + (to.1 - from.1) * t,
            ]
        })
        .collect();
    let mut p = json!({
        "mode": "smudge", "points": pts, "size": 22, "soft": 0.5, "strength": 50,
    });
    if let (Some(o), Some(e)) = (p.as_object_mut(), extra.as_object()) {
        for (k, v) in e {
            o.insert(k.clone(), v.clone());
        }
    }
    ex(s, "bitmap.retouch", p);
}

fn steps(s: &Session) -> usize {
    s.active().expect("document").history.undo_count()
}

#[test]
fn smudge_runs_past_the_bitmap_edge_and_grows_it() {
    let mut s = doc_session(300.0, 300.0);
    let id = pixel_art(&mut s);
    let base = image(&s, id);
    let n0 = steps(&s);
    smudge(&mut s, (80.0, 150.0), (160.0, 150.0), json!({}));
    assert_eq!(steps(&s), n0 + 1, "one step");
    let img = image(&s, id);
    let b = img.bounds();
    // grows to the right only, about 24 px (measured 24 for tip size 22), never to the stroke end
    assert_eq!((b.left, b.top, b.bottom), (0, 100, 200), "{b:?}");
    assert!(
        (115..=135).contains(&b.right),
        "the bitmap grows about 24 px past its edge: {b:?}"
    );
    // outside the old edge: colour of the bitmap, semi-transparent, fading out to the right
    let row: Vec<[u8; 4]> = (100..b.right).map(|x| img.get(x, 155).unwrap_or([0; 4])).collect();
    assert!(row[0][3] > 100, "the smear starts strong at the edge: {:?}", row[0]);
    assert!(row[0][2] > row[0][0] + 80, "it carries the blue: {:?}", row[0]);
    let alphas: Vec<u8> = row.iter().map(|p| p[3]).collect();
    assert!(
        alphas.iter().any(|a| *a > 0 && *a < 255),
        "semi-transparent: {alphas:?}"
    );
    let last = *alphas.iter().rev().find(|a| **a > 0).unwrap_or(&0);
    assert!(last < alphas[0], "fades out to the right: {alphas:?}");
    // no colour darkening at the fade (premultiplied mixing): faint pixels stay blue
    for p in row.iter().filter(|p| p[3] > 8) {
        assert!(p[2] > 120, "the fade keeps the colour, not black: {p:?}");
    }
    // the old pixels left of the stroke are unchanged, undo restores the old size
    assert_eq!(img.get(10, 120), base.get(10, 120));
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(image(&s, id).bounds(), base.bounds(), "undo restores the size");
}

#[test]
fn confine_to_boundaries_keeps_the_old_clipped_smudge() {
    let mut s = doc_session(300.0, 300.0);
    let id = pixel_art(&mut s);
    smudge(
        &mut s,
        (80.0, 150.0),
        (160.0, 150.0),
        json!({"confineToBounds": true}),
    );
    assert_eq!(image(&s, id).bounds(), IRect::new(0, 100, 100, 200));
}

#[test]
fn smudge_with_entire_document_grows_to_the_stroke_end_plus_radius() {
    let mut s = doc_session(300.0, 300.0);
    let id = pixel_art(&mut s);
    smudge(
        &mut s,
        (80.0, 150.0),
        (160.0, 150.0),
        json!({"wholeDocument": true}),
    );
    let img = image(&s, id);
    let b = img.bounds();
    assert!(
        (170..=174).contains(&b.right),
        "grows to the stroke end + radius (160 + 11): {b:?}"
    );
    // the white canvas is smeared in, opaque
    let p = img.get(165, 150).unwrap_or([0; 4]);
    assert_eq!(p, [255, 255, 255, 255], "canvas colour at the stroke end");
}

#[test]
fn smudge_from_outside_drags_transparency_in_without_growing() {
    let mut s = doc_session(300.0, 300.0);
    let id = pixel_art(&mut s);
    let base = image(&s, id);
    smudge(&mut s, (140.0, 150.0), (60.0, 150.0), json!({}));
    let img = image(&s, id);
    assert_eq!(img.bounds(), base.bounds(), "no growth from outside");
    let holes = (60..100)
        .filter(|x| img.get(*x, 150).is_some_and(|p| p[3] < 255))
        .count();
    assert!(holes > 5, "transparency is smeared into the bitmap ({holes})");
}

#[test]
fn smudge_outside_the_selected_bitmap_changes_nothing_but_writes_a_step() {
    let mut s = doc_session(300.0, 300.0);
    let id = pixel_art(&mut s);
    select(&mut s, &[id]);
    let base = image(&s, id);
    let n0 = steps(&s);
    let count = top_elements(&s).len();
    smudge(&mut s, (180.0, 250.0), (260.0, 270.0), json!({}));
    assert_eq!(steps(&s), n0 + 1, "one Smudge step");
    assert_eq!(image(&s, id), base, "no pixel changed");
    assert_eq!(top_elements(&s).len(), count, "no new object");
    // with "Entire document" the stroke rectangle becomes part of the bitmap, also entirely
    // outside it (FB-51, RE spec 70 section 1.3 case d: a separate strip, the gap transparent)
    smudge(&mut s, (180.0, 250.0), (260.0, 270.0), json!({"wholeDocument": true}));
    let b = image(&s, id).bounds();
    assert!(b.right > 260 && b.bottom > 270, "grown to the stroke: {b:?}");
    assert_eq!(top_elements(&s).len(), count, "still no new object");
    assert_eq!(image(&s, id).get(150, 210).map(|p| p[3]), Some(0), "gap transparent");
}

#[test]
fn smudge_tool_without_any_bitmap_does_nothing_and_writes_no_step() {
    let mut s = doc_session(300.0, 300.0);
    s.set_tool(ToolId::Smudge).unwrap_or_else(|e| panic!("{e}"));
    let n0 = steps(&s);
    let r = s.tool_event(&ToolEvent::drag(150.0, 150.0)).expect("hover");
    assert_eq!(r.cursor, fp_engine::tools::Cursor::NotAllowed);
    assert!(
        !s.tool_feedback()
            .iter()
            .any(|o| matches!(o, Overlay::BrushTip { .. })),
        "no tip outline without a bitmap"
    );
    events(
        &mut s,
        &[
            ToolEvent::down(100.0, 150.0),
            ToolEvent::drag(150.0, 150.0),
            ToolEvent::up(200.0, 150.0),
        ],
    );
    assert_eq!(steps(&s), n0, "no step");
    assert!(top_elements(&s).is_empty(), "no object");
}

#[test]
fn smudge_tool_grows_the_bitmap_and_the_option_confines_it() {
    for (confine, grows) in [(false, true), (true, false)] {
        let mut s = doc_session(300.0, 300.0);
        let id = pixel_art(&mut s);
        select(&mut s, &[id]);
        s.prefs.set(
            "SmudgeToolConfineToBounds",
            fp_engine::PrefValue::Bool(confine),
        );
        s.set_tool(ToolId::Smudge).unwrap_or_else(|e| panic!("{e}"));
        let mut evs = vec![ToolEvent::down(80.0, 150.0)];
        for i in 1..40 {
            evs.push(ToolEvent::drag(80.0 + i as f32 * 2.0, 150.0));
        }
        evs.push(ToolEvent::up(160.0, 150.0));
        events(&mut s, &evs);
        let b = image(&s, id).bounds();
        assert_eq!(b.right > 100, grows, "confine {confine}: {b:?}");
    }
}

#[test]
fn blur_across_the_edge_grows_the_bitmap_by_up_to_two_pixels() {
    // the second ring outside the edge only gets visible coverage from the later passes of a
    // strong stroke (measured 360 -> 362 at intensity 50; Firepixel's blur model reaches the
    // second ring from intensity ~70, below that the faint ring is dropped like other new edge
    // pixels under 2/255)
    for (strength, right) in [(100, 102), (50, 101)] {
        let mut s = doc_session(300.0, 300.0);
        let id = pixel_art(&mut s);
        let pts: Vec<[f32; 2]> = (0..=40).map(|i| [70.0 + i as f32 * 1.5, 155.0]).collect();
        ex(
            &mut s,
            "bitmap.retouch",
            json!({"mode": "blur", "points": pts, "size": 13, "soft": 1.0, "strength": strength}),
        );
        let img = image(&s, id);
        assert_eq!(img.bounds().right, right, "intensity {strength}: {:?}", img.bounds());
        // outside reads transparent white: the new pixels are light and semi-transparent
        let p = img.get(100, 155).unwrap_or([0; 4]);
        assert!(p[3] > 0 && p[3] < 255, "{p:?}");
    }
}
