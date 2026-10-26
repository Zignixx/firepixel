//! Bitmap stroke modifiers and history keys (Firepixel extensions to the pausable stroke, FB-31):
//!
//! * FB-71: Shift pressed in the middle of a stroke keeps everything painted so far and draws a
//!   straight (0/45/90 degree) segment from the point reached; releasing Shift joins the snapped
//!   point to the mouse with a straight line and paints freely on; one History row.
//! * FB-70: Undo and Redo while the button is held are ignored, the stroke goes on (measured);
//!   while paused Undo discards the stroke and Redo applies it (one row, Firepixel extension).
//! * FB-69: Alt while painting or paused subtracts: the brush tip erases only what this stroke
//!   painted, the pixels below stay; releasing Alt paints again.

use crate::common;
use common::*;
use fp_engine::tools::ToolId;
use fp_engine::{Overlay, Session, StrokePause, ToolEvent, ToolKey};
use fp_model::{ElementId, ElementKind};
use serde_json::{Value, json};

fn new_session() -> Session {
    let mut s = doc_session(400.0, 300.0);
    s.set_colors(Some(fp_color::Argb(0xff00_0000)), None);
    s
}

fn steps(s: &Session) -> usize {
    s.active().expect("document").history.undo_count()
}

fn image(s: &Session, id: ElementId) -> fp_raster::RgbaImage {
    match &el(s, id).kind {
        ElementKind::Bitmap(b) => fp_engine::docops::bitmap_image(b).expect("image"),
        _ => panic!("not a bitmap"),
    }
}

fn px(img: &fp_raster::RgbaImage, x: i32, y: i32) -> [u8; 4] {
    img.get(x, y).unwrap_or([0; 4])
}

fn hard_brush(s: &mut Session) {
    s.set_tool(ToolId::Brush).expect("tool");
    // the row of choosing the Brush (RE spec 72 §1) is not part of these counts
    if let Some(d) = s.active_mut() {
        d.history.clear();
    }
    let stroke: Value = json!({"preset": "Einfach/Präzise abger.", "width": 12, "softness": 0});
    s.tools.options.brush_size = 12.0;
    s.tools.options.new_stroke = Some(stroke);
    s.tools.options.new_brush = None;
    s.tools.options.opacity = 100;
}

/// Moves with the button held from `a` to `b` in small steps, each event passed through `m`.
fn drag_m(s: &mut Session, a: (f32, f32), b: (f32, f32), m: impl Fn(ToolEvent) -> ToolEvent) {
    let len = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
    let k = (len / 3.0).ceil().max(1.0) as usize;
    for i in 1..=k {
        let t = i as f32 / k as f32;
        events(
            s,
            &[m(ToolEvent::drag(
                a.0 + (b.0 - a.0) * t,
                a.1 + (b.1 - a.1) * t,
            ))],
        );
    }
}

fn drag(s: &mut Session, a: (f32, f32), b: (f32, f32)) {
    drag_m(s, a, b, |e| e);
}

#[test]
fn shift_mid_stroke_keeps_the_painted_part() {
    let mut s = new_session();
    hard_brush(&mut s);
    let n0 = steps(&s);
    // free vertical part, then Shift: a straight segment from the point reached
    events(&mut s, &[ToolEvent::down(50.0, 50.0)]);
    drag(&mut s, (50.0, 50.0), (50.0, 150.0));
    drag_m(&mut s, (50.0, 150.0), (150.0, 160.0), |e| e.shift());
    // the preview still shows the first part while Shift is held
    let preview = s
        .tool_feedback()
        .into_iter()
        .find_map(|o| match o {
            Overlay::Paint(p) => Some(p),
            _ => None,
        })
        .expect("preview");
    let shown = |x: i32, y: i32| {
        preview
            .tiles
            .iter()
            .find(|t| t.rect.contains(x, y))
            .and_then(|t| t.image.get(x, y))
            .unwrap_or([0; 4])[3]
    };
    assert!(
        shown(51, 100) > 0,
        "the part painted before Shift stays in the preview"
    );
    // Shift released: free painting from the end of the straight segment
    drag(&mut s, (150.0, 160.0), (150.0, 250.0));
    // Shift again: another straight segment, nothing is reset
    drag_m(&mut s, (150.0, 250.0), (250.0, 255.0), |e| e.shift());
    events(&mut s, &[ToolEvent::up(250.0, 255.0).shift()]);
    assert_eq!(
        steps(&s),
        n0 + 3,
        "one stroke, one row (after the rows \"Insert Empty Bitmap\" and \"Edit Bitmap\" that start bitmap mode, RE spec 71)"
    );
    let img = image(&s, top_elements(&s)[0].id);
    // even size: points +1/+1
    assert_eq!(px(&img, 51, 100)[3], 255, "the first free part is kept");
    assert_eq!(
        px(&img, 100, 151)[3],
        255,
        "the straight segment from the point reached (horizontal)"
    );
    assert_eq!(
        px(&img, 100, 120)[3],
        0,
        "no line from the stroke's start point"
    );
    assert_eq!(
        px(&img, 151, 200)[3],
        255,
        "free painting after Shift was released"
    );
    assert_eq!(px(&img, 200, 251)[3], 255, "the second straight segment");
    assert_eq!(
        px(&img, 51, 60)[3],
        255,
        "the start is still painted after the second Shift"
    );
}

#[test]
fn shift_at_the_press_still_draws_from_the_press_point() {
    let mut s = new_session();
    hard_brush(&mut s);
    events(&mut s, &[ToolEvent::down(50.0, 50.0).shift()]);
    drag_m(&mut s, (50.0, 50.0), (60.0, 150.0), |e| e.shift());
    drag_m(&mut s, (60.0, 150.0), (250.0, 60.0), |e| e.shift());
    events(&mut s, &[ToolEvent::up(250.0, 60.0).shift()]);
    let img = image(&s, top_elements(&s)[0].id);
    let b = img.alpha_bounds();
    assert!(
        b.height() <= 16,
        "one straight horizontal line from the press point ({b:?})"
    );
}

#[test]
fn undo_and_redo_are_ignored_while_the_button_is_held() {
    let mut s = new_session();
    hard_brush(&mut s);
    // a fresh document: no row yet; Undo while held does nothing and the stroke goes on
    events(&mut s, &[ToolEvent::down(50.0, 100.0)]);
    drag(&mut s, (50.0, 100.0), (150.0, 100.0));
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(
        s.tool_stroke_pause(),
        StrokePause::Painting,
        "the stroke continues"
    );
    drag(&mut s, (150.0, 100.0), (250.0, 100.0));
    events(&mut s, &[ToolEvent::up(250.0, 100.0)]);
    assert_eq!(
        steps(&s),
        3,
        "the whole stroke is applied on release (after \"Insert Empty Bitmap\", \"Edit Bitmap\", RE spec 71)"
    );
    let id = top_elements(&s)[0].id;
    let img = image(&s, id);
    assert_eq!(px(&img, 100, 101)[3], 255, "painted before the Undo key");
    assert_eq!(px(&img, 200, 101)[3], 255, "painted after the Undo key");
    // with an earlier row: Undo while held does not undo it
    events(&mut s, &[ToolEvent::down(50.0, 220.0)]);
    drag(&mut s, (50.0, 220.0), (250.0, 220.0));
    s.undo().expect("undo");
    assert_eq!(steps(&s), 3, "the earlier rows stay");
    assert_eq!(s.tool_stroke_pause(), StrokePause::Painting);
    events(&mut s, &[ToolEvent::up(250.0, 220.0)]);
    // far from the first bitmap: a new one ("Insert Empty Bitmap" and the stroke)
    assert_eq!(steps(&s), 5);
    // Redo while held is ignored too (no repeat of the last row)
    events(&mut s, &[ToolEvent::down(50.0, 260.0)]);
    drag(&mut s, (50.0, 260.0), (150.0, 260.0));
    ex(&mut s, "edit.redo", json!({}));
    assert_eq!(
        s.tool_stroke_pause(),
        StrokePause::Painting,
        "Redo is ignored while held"
    );
    assert_eq!(steps(&s), 5);
    drag(&mut s, (150.0, 260.0), (250.0, 260.0));
    events(&mut s, &[ToolEvent::up(250.0, 260.0)]);
    assert_eq!(steps(&s), 6, "the stroke is applied on release");
}

#[test]
fn paused_stroke_undo_discards_redo_applies_via_commands() {
    let mut s = new_session();
    hard_brush(&mut s);
    events(&mut s, &[ToolEvent::down(50.0, 100.0)]);
    drag(&mut s, (50.0, 100.0), (250.0, 100.0));
    s.tool_event(&ToolEvent::key(ToolKey::Pause)).expect("P");
    events(&mut s, &[ToolEvent::up(250.0, 100.0)]);
    assert_eq!(s.tool_stroke_pause(), StrokePause::Paused);
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(s.tool_stroke_pause(), StrokePause::Idle);
    assert_eq!(steps(&s), 0, "discarded, no row");
    events(&mut s, &[ToolEvent::down(50.0, 100.0)]);
    drag(&mut s, (50.0, 100.0), (250.0, 100.0));
    s.tool_event(&ToolEvent::key(ToolKey::Pause)).expect("P");
    events(&mut s, &[ToolEvent::up(250.0, 100.0)]);
    ex(&mut s, "edit.redo", json!({}));
    assert_eq!(s.tool_stroke_pause(), StrokePause::Idle);
    assert_eq!(
        steps(&s),
        3,
        "Redo applied the paused stroke as one row (after \"Insert Empty Bitmap\", \"Edit Bitmap\", RE spec 71)"
    );
    let img = image(&s, top_elements(&s)[0].id);
    assert_eq!(px(&img, 150, 101)[3], 255);
}

#[test]
fn retouch_undo_while_paused_discards_the_stroke() {
    let mut s = new_session();
    let b = bitmap(&mut s, 20, 20, 360, 260, [100, 100, 100, 255]);
    select(&mut s, &[b]);
    s.set_tool(ToolId::Dodge).expect("tool");
    let n0 = steps(&s);
    events(&mut s, &[ToolEvent::drag(50.0, 100.0)]);
    events(&mut s, &[ToolEvent::down(50.0, 100.0)]);
    drag(&mut s, (50.0, 100.0), (250.0, 100.0));
    s.tool_event(&ToolEvent::key(ToolKey::Pause)).expect("P");
    events(&mut s, &[ToolEvent::up(250.0, 100.0)]);
    assert_eq!(s.tool_stroke_pause(), StrokePause::Paused);
    s.undo().expect("undo");
    assert_eq!(s.tool_stroke_pause(), StrokePause::Idle);
    assert_eq!(steps(&s), n0, "no row");
    assert_eq!(
        px(&image(&s, b), 150, 100),
        [100, 100, 100, 255],
        "nothing dodged"
    );
}

#[test]
fn alt_subtracts_only_the_strokes_own_pixels() {
    let mut s = new_session();
    let b = bitmap(&mut s, 20, 20, 360, 260, [0, 0, 255, 255]);
    select(&mut s, &[b]);
    hard_brush(&mut s);
    let n0 = steps(&s);
    events(&mut s, &[ToolEvent::down(50.0, 100.0)]);
    drag(&mut s, (50.0, 100.0), (250.0, 100.0));
    // Alt with the button held: back over the right half, erasing only this stroke
    drag_m(&mut s, (250.0, 100.0), (150.0, 100.0), |e| e.alt());
    assert!(s.tool_stroke_subtracting(), "subtraction mode");
    // the preview shows the bitmap unchanged there
    let preview = s
        .tool_feedback()
        .into_iter()
        .find_map(|o| match o {
            Overlay::Paint(p) => Some(p),
            _ => None,
        })
        .expect("preview");
    let shown = preview
        .tiles
        .iter()
        .find(|t| t.rect.contains(200, 101))
        .and_then(|t| t.image.get(200, 101))
        .unwrap_or([0; 4]);
    assert_eq!(
        shown[3], 0,
        "the preview shows no change where the stroke was subtracted"
    );
    // Alt released: painting again (a new part from here)
    drag(&mut s, (150.0, 100.0), (150.0, 200.0));
    assert!(!s.tool_stroke_subtracting());
    events(&mut s, &[ToolEvent::up(150.0, 200.0)]);
    assert_eq!(steps(&s), n0 + 1, "one row");
    let img = image(&s, b);
    assert_eq!(
        px(&img, 100, 101),
        [0, 0, 0, 255],
        "the part not subtracted stays"
    );
    assert_eq!(
        px(&img, 200, 101),
        [0, 0, 255, 255],
        "subtracted: the bitmap below is untouched"
    );
    assert_eq!(
        px(&img, 151, 170),
        [0, 0, 0, 255],
        "painting after Alt was released"
    );
    assert_eq!(px(&img, 300, 101), [0, 0, 255, 255]);
    let cmd = s
        .active()
        .expect("document")
        .history
        .top()
        .map(|t| t.command.clone());
    assert_eq!(cmd.as_deref(), Some("bitmap.paint"));
}

#[test]
fn alt_while_paused_subtracts_and_the_stroke_stays_paused() {
    let mut s = new_session();
    let b = bitmap(&mut s, 20, 20, 360, 260, [0, 0, 255, 255]);
    select(&mut s, &[b]);
    hard_brush(&mut s);
    events(&mut s, &[ToolEvent::down(50.0, 100.0)]);
    drag(&mut s, (50.0, 100.0), (250.0, 100.0));
    s.tool_event(&ToolEvent::key(ToolKey::Pause)).expect("P");
    events(&mut s, &[ToolEvent::up(250.0, 100.0)]);
    events(&mut s, &[ToolEvent::down(180.0, 100.0).alt()]);
    drag_m(&mut s, (180.0, 100.0), (240.0, 100.0), |e| e.alt());
    events(&mut s, &[ToolEvent::up(240.0, 100.0).alt()]);
    assert_eq!(
        s.tool_stroke_pause(),
        StrokePause::Paused,
        "still paused after the Alt drag"
    );
    s.tool_event(&ToolEvent::key(ToolKey::Enter))
        .expect("Enter");
    let img = image(&s, b);
    assert_eq!(
        px(&img, 210, 101),
        [0, 0, 255, 255],
        "subtracted while paused"
    );
    assert_eq!(px(&img, 100, 101), [0, 0, 0, 255]);
    // Alt held at the press paints (the Brush has no Alt function at the press)
    events(&mut s, &[ToolEvent::down(50.0, 200.0).alt()]);
    drag_m(&mut s, (50.0, 200.0), (150.0, 200.0), |e| e.alt());
    events(&mut s, &[ToolEvent::up(150.0, 200.0).alt()]);
    assert_eq!(
        px(&image(&s, b), 100, 201),
        [0, 0, 0, 255],
        "Alt at the press paints"
    );
}

#[test]
fn subtract_parameter_of_bitmap_paint() {
    let mut s = new_session();
    let b = bitmap(&mut s, 0, 0, 200, 100, [255, 255, 255, 255]);
    select(&mut s, &[b]);
    ex(
        &mut s,
        "bitmap.paint",
        json!({"points": [[20, 50], [180, 50]], "size": 10, "color": "#000000",
               "subtract": [{"at": 2, "size": 20, "round": true, "points": [[100, 50], [180, 50]]}]}),
    );
    let img = image(&s, b);
    assert_eq!(px(&img, 50, 50), [0, 0, 0, 255]);
    assert_eq!(
        px(&img, 140, 50),
        [255, 255, 255, 255],
        "the pass removed the stroke there"
    );
}
