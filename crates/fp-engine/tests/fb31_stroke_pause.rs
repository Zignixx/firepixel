//! FB-31 (Firepixel extension): a running bitmap stroke (Brush, Pencil, Eraser, retouch tools) is
//! paused with P. While paused the button may be released and the pointer moves freely without
//! painting; the next press resumes the SAME stroke as a new, unconnected segment, so a
//! semi-transparent stroke does not get darker where the parts overlap. Enter applies the paused
//! stroke as one History row; Escape, a tool change and a menu command apply it first (Undo discards
//! it and Redo applies it, FB-70).

use crate::common;
use common::*;
use fp_engine::shortcuts::Shortcut;
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

fn alpha(img: &fp_raster::RgbaImage, x: i32, y: i32) -> u8 {
    img.get(x, y).unwrap_or([0; 4])[3]
}

fn hard_brush(s: &mut Session, opacity: u8) {
    s.set_tool(ToolId::Brush).expect("tool");
    let stroke: Value = json!({"preset": "Einfach/Präzise abger.", "width": 12, "softness": 0});
    s.tools.options.brush_size = 12.0;
    s.tools.options.new_stroke = Some(stroke);
    s.tools.options.new_brush = None;
    s.tools.options.opacity = opacity;
}

fn key(s: &mut Session, k: ToolKey) -> fp_engine::ToolResponse {
    s.tool_event(&ToolEvent::key(k)).expect("key")
}

/// Moves with the button held from `a` to `b` in small steps.
fn drag(s: &mut Session, a: (f32, f32), b: (f32, f32)) {
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
    }
}

/// Press at `a`, drag to `b`, P, release at `b`: the stroke is paused.
fn paint_and_pause(s: &mut Session, a: (f32, f32), b: (f32, f32)) {
    events(s, &[ToolEvent::down(a.0, a.1)]);
    drag(s, a, b);
    assert_eq!(
        s.tool_stroke_pause(),
        StrokePause::Painting,
        "a stroke is being painted"
    );
    key(s, ToolKey::Pause);
    assert_eq!(
        s.tool_stroke_pause(),
        StrokePause::Paused,
        "P pauses the stroke"
    );
    events(s, &[ToolEvent::up(b.0, b.1)]);
    assert_eq!(
        s.tool_stroke_pause(),
        StrokePause::Paused,
        "releasing the button keeps the pause"
    );
}

#[test]
fn resumed_stroke_is_one_row_and_the_overlap_does_not_darken() {
    let mut s = new_session();
    hard_brush(&mut s, 33);
    let n0 = steps(&s);
    let before = s.active().expect("document").doc.clone();
    paint_and_pause(&mut s, (50.0, 100.0), (250.0, 100.0));
    // moving freely while paused paints nothing and changes nothing
    for (x, y) in [(260.0, 150.0), (320.0, 250.0), (150.0, 40.0)] {
        events(&mut s, &[ToolEvent::drag(x, y)]);
    }
    assert_eq!(steps(&s), n0, "no history row while paused");
    assert!(
        std::sync::Arc::ptr_eq(&before, &s.active().expect("document").doc),
        "the document is unchanged while paused"
    );
    let fb = s.tool_feedback();
    assert!(
        fb.iter().any(|o| matches!(o, Overlay::BrushTip { .. })),
        "the brush outline follows the pointer while paused"
    );
    assert!(
        fb.iter().any(|o| matches!(o, Overlay::Paint(_))),
        "the paused stroke stays visible"
    );
    // resume with a press elsewhere: a vertical part crossing the horizontal one
    events(&mut s, &[ToolEvent::down(150.0, 40.0)]);
    assert_eq!(
        s.tool_stroke_pause(),
        StrokePause::Painting,
        "the press resumes the stroke"
    );
    drag(&mut s, (150.0, 40.0), (150.0, 200.0));
    // P again pauses again
    key(&mut s, ToolKey::Pause);
    assert_eq!(s.tool_stroke_pause(), StrokePause::Paused);
    events(&mut s, &[ToolEvent::up(150.0, 200.0)]);
    // the live preview shows exactly what Enter applies
    let preview = s
        .tool_feedback()
        .into_iter()
        .find_map(|o| match o {
            Overlay::Paint(p) => Some(p),
            _ => None,
        })
        .expect("preview");
    let r = key(&mut s, ToolKey::Enter);
    assert_eq!(r.commands.len(), 1, "Enter applies the paused stroke");
    let n = r.commands[0].params["points"]
        .as_array()
        .map(|a| a.len())
        .unwrap_or(0);
    let breaks = r.commands[0].params["breaks"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert_eq!(breaks.len(), 1, "one resume: {breaks:?}");
    let k = breaks[0].as_u64().unwrap_or(0) as usize;
    assert!(
        k > 1 && k < n - 1,
        "the resumed part starts inside the points ({k} of {n})"
    );
    assert_eq!(
        r.commands[0].params["points"][k],
        json!([151.0, 41.0]),
        "the resumed part starts at the press (even size: +1/+1)"
    );
    assert_eq!(s.tool_stroke_pause(), StrokePause::Idle);
    assert_eq!(
        steps(&s),
        n0 + 3,
        "the whole stroke is ONE history row (after the rows \"Insert Empty Bitmap\" and \"Edit Bitmap\" that start bitmap mode, RE spec 71)"
    );
    let id = top_elements(&s)[0].id;
    let img = image(&s, id);
    let single_h = alpha(&img, 100, 100);
    let single_v = alpha(&img, 150, 170);
    let overlap = alpha(&img, 150, 100);
    assert!((83..=85).contains(&single_h), "opacity 33 % ({single_h})");
    assert_eq!(single_v, single_h, "the resumed part has the same opacity");
    assert_eq!(
        overlap, single_h,
        "the overlap within the one stroke is not darker"
    );
    assert_eq!(
        alpha(&img, 200, 70),
        0,
        "the parts are not joined by a line"
    );
    assert_eq!(alpha(&img, 320, 250), 0, "nothing painted while paused");
    let b = img.bounds();
    let mut diff = 0;
    for y in b.top..b.bottom {
        for x in b.left..b.right {
            let v = img.get(x, y).unwrap_or([0; 4]);
            let shown = preview
                .tiles
                .iter()
                .find(|t| t.rect.contains(x, y))
                .and_then(|t| t.exact.as_ref().unwrap_or(&t.image).get(x, y))
                .unwrap_or([0; 4]);
            if v[3] > 0 && shown != v {
                diff += 1;
            }
        }
    }
    assert_eq!(diff, 0, "preview and result differ in {diff} pixels");
    // control: two separate strokes darken the overlap
    s.undo().expect("undo");
    hard_brush(&mut s, 33);
    events(&mut s, &[ToolEvent::down(50.0, 100.0)]);
    drag(&mut s, (50.0, 100.0), (250.0, 100.0));
    events(&mut s, &[ToolEvent::up(250.0, 100.0)]);
    events(&mut s, &[ToolEvent::down(150.0, 40.0)]);
    drag(&mut s, (150.0, 40.0), (150.0, 200.0));
    events(&mut s, &[ToolEvent::up(150.0, 200.0)]);
    let img = image(&s, top_elements(&s)[0].id);
    assert!(
        alpha(&img, 150, 100) > alpha(&img, 100, 100) + 20,
        "two strokes: the overlap is darker"
    );
}

#[test]
fn escape_while_paused_applies_the_stroke() {
    let mut s = new_session();
    hard_brush(&mut s, 50);
    let n0 = steps(&s);
    paint_and_pause(&mut s, (50.0, 100.0), (250.0, 100.0));
    // a press with Space held pans the view: it does not resume the stroke
    events(
        &mut s,
        &[
            ToolEvent::down(150.0, 200.0).space(),
            ToolEvent::drag(160.0, 220.0).space(),
            ToolEvent::up(160.0, 220.0).space(),
        ],
    );
    assert_eq!(
        s.tool_stroke_pause(),
        StrokePause::Paused,
        "Space + press keeps the pause"
    );
    let r = key(&mut s, ToolKey::Escape);
    assert_eq!(
        r.commands.first().map(|c| c.command.as_str()),
        Some("bitmap.paint")
    );
    assert_eq!(steps(&s), n0 + 3, "Escape applies the paused stroke");
    assert_eq!(s.tools.active, ToolId::Brush, "the tool stays");
    assert_eq!(s.tool_stroke_pause(), StrokePause::Idle);
    let img = image(&s, top_elements(&s)[0].id);
    assert!(alpha(&img, 150, 100) > 0, "the stroke is painted");
    assert_eq!(alpha(&img, 155, 210), 0, "the Space drag painted nothing");
}

#[test]
fn tool_change_while_paused_applies_the_stroke() {
    let mut s = new_session();
    hard_brush(&mut s, 50);
    paint_and_pause(&mut s, (50.0, 100.0), (250.0, 100.0));
    s.set_tool(ToolId::Pointer).expect("tool");
    let img = image(&s, top_elements(&s)[0].id);
    assert!(
        alpha(&img, 150, 100) > 0,
        "the tool change applied the stroke"
    );
    assert_eq!(s.tool_stroke_pause(), StrokePause::Idle);
}

/// FB-70: Undo while paused discards the stroke (no row, nothing painted); Redo applies it.
#[test]
fn undo_while_paused_discards_and_redo_applies() {
    let mut s = new_session();
    hard_brush(&mut s, 50);
    let n0 = steps(&s);
    let elements = top_elements(&s).len();
    paint_and_pause(&mut s, (50.0, 100.0), (250.0, 100.0));
    s.undo().expect("undo");
    assert_eq!(steps(&s), n0, "the paused stroke is discarded, no row");
    assert_eq!(top_elements(&s).len(), elements, "no bitmap");
    assert_eq!(
        s.tool_stroke_pause(),
        StrokePause::Idle,
        "no stroke left behind"
    );
    paint_and_pause(&mut s, (50.0, 100.0), (250.0, 100.0));
    s.redo().expect("redo");
    assert_eq!(
        steps(&s),
        n0 + 3,
        "Redo applies the paused stroke (after the rows \"Insert Empty Bitmap\" and \"Edit Bitmap\" that start bitmap mode, RE spec 71)"
    );
    assert_eq!(s.tool_stroke_pause(), StrokePause::Idle);
    let img = image(&s, top_elements(&s)[0].id);
    assert!(alpha(&img, 150, 100) > 0);
    // the tool keeps working normally afterwards
    events(&mut s, &[ToolEvent::down(50.0, 200.0)]);
    drag(&mut s, (50.0, 200.0), (250.0, 200.0));
    events(&mut s, &[ToolEvent::up(250.0, 200.0)]);
    // far from the bitmap: a new one ("Insert Empty Bitmap" and the stroke, still bitmap mode)
    assert_eq!(steps(&s), n0 + 5);
}

#[test]
fn menu_command_applies_but_zoom_keeps_the_pause() {
    let mut s = new_session();
    hard_brush(&mut s, 50);
    let n0 = steps(&s);
    paint_and_pause(&mut s, (50.0, 100.0), (250.0, 100.0));
    ex(&mut s, "view.zoomIn", json!({}));
    assert_eq!(
        s.tool_stroke_pause(),
        StrokePause::Paused,
        "zooming keeps the pause"
    );
    assert_eq!(steps(&s), n0);
    ex(&mut s, "select.none", json!({}));
    assert_eq!(
        s.tool_stroke_pause(),
        StrokePause::Idle,
        "a menu command applies the stroke"
    );
    assert_eq!(steps(&s), n0 + 4, "the stroke row, then the command");
}

#[test]
fn p_without_a_stroke_keeps_the_toolbox_shortcut() {
    let mut s = new_session();
    hard_brush(&mut s, 50);
    assert_eq!(s.tool_stroke_pause(), StrokePause::Idle, "no stroke");
    let r = key(&mut s, ToolKey::Pause);
    assert!(
        r.commands.is_empty() && r.switch_tool.is_none(),
        "the pause key does nothing without a stroke"
    );
    let p = Shortcut {
        ctrl: false,
        alt: false,
        shift: false,
        vk: 0x50,
    };
    let t = s.tool_shortcut(&p).expect("shortcut");
    assert!(
        t.is_some_and(|t| t != ToolId::Brush),
        "P selects its tool as before ({t:?})"
    );
}

#[test]
fn pencil_and_eraser_pause_too() {
    let mut s = new_session();
    s.set_tool(ToolId::Pencil).expect("tool");
    let n0 = steps(&s);
    paint_and_pause(&mut s, (50.0, 100.0), (250.0, 100.0));
    events(&mut s, &[ToolEvent::down(150.0, 40.0)]);
    drag(&mut s, (150.0, 40.0), (150.0, 200.0));
    events(&mut s, &[ToolEvent::up(150.0, 200.0)]);
    assert_eq!(
        steps(&s),
        n0 + 3,
        "release after the resume applies the whole stroke"
    );
    let id = top_elements(&s)[0].id;
    let img = image(&s, id);
    assert!(
        alpha(&img, 100, 100) > 0 && alpha(&img, 150, 170) > 0,
        "both parts painted"
    );
    assert_eq!(alpha(&img, 200, 70), 0, "not joined");
    // the eraser with 50 % opacity: the overlap is erased once
    let b = bitmap(&mut s, 20, 20, 360, 260, [0, 0, 0, 255]);
    select(&mut s, &[b]);
    s.set_tool(ToolId::Eraser).expect("tool");
    s.tools.options.eraser_opacity = 50;
    s.tools.options.eraser_size = 12.0;
    s.tools.options.eraser_edge = 0;
    let n1 = steps(&s);
    paint_and_pause(&mut s, (50.0, 150.0), (250.0, 150.0));
    events(&mut s, &[ToolEvent::down(150.0, 60.0)]);
    drag(&mut s, (150.0, 60.0), (150.0, 240.0));
    key(&mut s, ToolKey::Pause);
    key(&mut s, ToolKey::Enter);
    assert_eq!(steps(&s), n1 + 1, "one eraser row");
    let img = image(&s, b);
    let single = alpha(&img, 100, 150);
    assert!((120..=135).contains(&single), "half erased ({single})");
    assert_eq!(alpha(&img, 150, 150), single, "the overlap is erased once");
    assert_eq!(alpha(&img, 150, 200), single);
}

#[test]
fn retouch_dodge_pause_covers_the_overlap_once() {
    let mut s = new_session();
    let b = bitmap(&mut s, 20, 20, 360, 260, [100, 100, 100, 255]);
    select(&mut s, &[b]);
    s.set_tool(ToolId::Dodge).expect("tool");
    let n0 = steps(&s);
    events(&mut s, &[ToolEvent::drag(50.0, 100.0)]);
    paint_and_pause(&mut s, (50.0, 100.0), (250.0, 100.0));
    events(&mut s, &[ToolEvent::drag(150.0, 40.0)]);
    events(&mut s, &[ToolEvent::down(150.0, 40.0)]);
    assert_eq!(s.tool_stroke_pause(), StrokePause::Painting);
    drag(&mut s, (150.0, 40.0), (150.0, 200.0));
    events(&mut s, &[ToolEvent::up(150.0, 200.0)]);
    assert_eq!(steps(&s), n0 + 1, "one Dodge row");
    let img = image(&s, b);
    let single = img.get(100, 100).expect("px")[0];
    assert!(single > 100, "dodged ({single})");
    assert_eq!(
        img.get(150, 170).expect("px")[0],
        single,
        "the resumed part"
    );
    assert_eq!(
        img.get(150, 100).expect("px")[0],
        single,
        "the overlap is dodged once"
    );
    assert_eq!(img.get(200, 70).expect("px")[0], 100, "not joined");
}

/// Opaque 300 x 200 bitmap at 0/0 with hard edges (blur, sharpen and smudge change it).
fn pattern_bitmap(s: &mut Session) -> ElementId {
    let r = fp_geom::IRect::new(0, 0, 300, 200);
    let mut img = fp_raster::RgbaImage::new(r).expect("image");
    for y in r.top..r.bottom {
        for x in r.left..r.right {
            let c = if ((x / 9) + (y / 7)) % 2 == 0 {
                [220, 40, (y % 256) as u8, 255]
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

fn last_preview(s: &mut Session) -> Option<fp_engine::paint::PaintPreview> {
    s.tool_feedback().into_iter().find_map(|o| match o {
        Overlay::Paint(p) => Some(p),
        _ => None,
    })
}

/// FB-31 with the incremental retouch engine (FB-39): a paused and resumed Blur, Sharpen, Dodge,
/// Burn or Smudge stroke is previewed part by part with the breaks, the last live preview equals
/// the stored bitmap, and the stored bitmap equals `bitmap.retouch` computed on its own from the
/// sent parameters (with `breaks`). Applied with the release and with Enter while paused.
#[test]
fn retouch_pause_live_stroke_equals_the_command() {
    for (k, tool) in [
        ToolId::Blur,
        ToolId::Sharpen,
        ToolId::Dodge,
        ToolId::Burn,
        ToolId::Smudge,
    ]
    .into_iter()
    .enumerate()
    {
        for enter in [false, true] {
            let mut s = new_session();
            let id = pattern_bitmap(&mut s);
            select(&mut s, &[id]);
            s.set_tool(tool).expect("tool");
            let base = image(&s, id);
            let n0 = steps(&s);
            paint_and_pause(&mut s, (40.0, 60.0), (200.0, 60.0 + k as f32));
            events(&mut s, &[ToolEvent::drag(120.0, 20.0)]);
            events(&mut s, &[ToolEvent::down(120.0, 20.0)]);
            assert_eq!(
                s.tool_stroke_pause(),
                StrokePause::Painting,
                "{tool:?}: resumed"
            );
            drag(&mut s, (120.0, 20.0), (125.0, 160.0));
            let p = last_preview(&mut s).unwrap_or_else(|| panic!("{tool:?}: live preview"));
            let r = if enter {
                key(&mut s, ToolKey::Pause);
                events(&mut s, &[ToolEvent::up(125.0, 160.0)]);
                key(&mut s, ToolKey::Enter)
            } else {
                s.tool_event(&ToolEvent::up(125.0, 160.0)).expect("up")
            };
            assert_eq!(steps(&s), n0 + 1, "{tool:?}: one row");
            let breaks = r.commands[0].params["breaks"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            assert_eq!(breaks.len(), 1, "{tool:?}: one resume {breaks:?}");
            let res = image(&s, id);
            assert!(res != base, "{tool:?}: the stroke changes pixels");
            // the last live preview is the result
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
            assert_eq!(diff, 0, "{tool:?} enter {enter}: preview equals the result");
            // the command on its own computes the same bitmap
            let params = s
                .active()
                .expect("document")
                .history
                .top()
                .expect("step")
                .params
                .clone();
            let mut t = new_session();
            let id2 = pattern_bitmap(&mut t);
            select(&mut t, &[id2]);
            ex(&mut t, "bitmap.retouch", params);
            assert!(
                image(&t, id2) == res,
                "{tool:?} enter {enter}: the command's own result"
            );
        }
    }
}
