//! FB-148 (Firepixel extensions to the pausable bitmap stroke, FB-31/FB-69/FB-71):
//!
//! * a) Alt subtraction also for Blur, Sharpen, Dodge and Burn: the tip restores what this
//!   stroke changed (not Smudge).
//! * b) preferences "Alt subtraction: hold / toggle" and "Pause stroke: hold / toggle".
//! * c) settings changed while the stroke is paused apply to the part painted after the resume.
//! * d) the Shift straight-line preview is drawn at the real brush width.

use crate::common;
use common::*;
use fp_engine::tools::ToolId;
use fp_engine::tools::STROKE_ALT_MODE_PREF;
use fp_engine::{Overlay, PrefValue, Session, StrokePause, ToolEvent, ToolKey};
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

/// The live preview of the running stroke.
fn preview(s: &mut Session) -> Option<fp_engine::paint::PaintPreview> {
    s.tool_feedback().into_iter().find_map(|o| match o {
        Overlay::Paint(p) => Some(p),
        _ => None,
    })
}

/// Pixel the live preview `p` shows at `x, y` (transparent = unchanged).
fn shown(p: &Option<fp_engine::paint::PaintPreview>, x: i32, y: i32) -> [u8; 4] {
    p.as_ref()
        .and_then(|p| {
            p.tiles
                .iter()
                .find(|t| t.rect.contains(x, y))
                .and_then(|t| t.image.get(x, y))
        })
        .unwrap_or([0; 4])
}

fn brush(s: &mut Session, preset: &str, size: f32) {
    s.set_tool(ToolId::Brush).expect("tool");
    if let Some(d) = s.active_mut() {
        d.history.clear();
    }
    let stroke: Value = json!({"preset": preset, "width": size, "softness": 0});
    s.tools.options.brush_size = size;
    s.tools.options.new_stroke = Some(stroke);
    s.tools.options.new_brush = None;
    s.tools.options.opacity = 100;
}

/// Moves with the button held from `a` to `b` in steps of about 3 px, each event passed through
/// `m` (with `dt` ms between the events, 0 = no time stamps).
fn drag_t(
    s: &mut Session,
    a: (f32, f32),
    b: (f32, f32),
    t0: u64,
    dt: u64,
    m: impl Fn(ToolEvent) -> ToolEvent,
) -> u64 {
    let len = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
    let k = (len / 3.0).ceil().max(1.0) as usize;
    let mut t = t0;
    for i in 1..=k {
        let f = i as f32 / k as f32;
        t += dt;
        let mut e = m(ToolEvent::drag(a.0 + (b.0 - a.0) * f, a.1 + (b.1 - a.1) * f));
        e.time_ms = t;
        events(s, &[e]);
    }
    t
}

/// Rows of opaque-ish pixels in column `x` between `y0` and `y1` (`get` = pixel source).
fn thickness(get: impl Fn(i32, i32) -> [u8; 4], x: i32, y0: i32, y1: i32) -> usize {
    (y0..y1).filter(|y| get(x, *y)[3] >= 128).count()
}

#[test]
fn d_shift_line_preview_has_the_real_width() {
    let mut s = new_session();
    brush(&mut s, "Einfach/Hart abgerundet", 14.0);
    let mut d = ToolEvent::down(50.0, 100.0).shift();
    d.time_ms = 1000;
    events(&mut s, &[d]);
    // a fast pointer (about 6 px/ms) with Shift held: a straight line
    let t = drag_t(&mut s, (50.0, 100.0), (300.0, 104.0), 1000, 1, |e| e.shift());
    let p = preview(&mut s);
    let live = thickness(|x, y| shown(&p, x, y), 175, 60, 140);
    let mut u = ToolEvent::up(300.0, 104.0).shift();
    u.time_ms = t + 1;
    events(&mut s, &[u]);
    let id = top_elements(&s)[0].id;
    let img = image(&s, id);
    let done = thickness(|x, y| px(&img, x, y), 175, 60, 140);
    assert!(done >= 13, "the placed line has the brush width: {done}");
    assert_eq!(live, done, "the preview of the Shift line has the width of the result");
}


/// Moves with the button held, no time stamps.
fn drag_m(s: &mut Session, a: (f32, f32), b: (f32, f32), m: impl Fn(ToolEvent) -> ToolEvent) {
    drag_t(s, a, b, 0, 0, m);
}

fn drag(s: &mut Session, a: (f32, f32), b: (f32, f32)) {
    drag_m(s, a, b, |e| e);
}

/// A grey bitmap over 20..380 × 20..280, selected, and the retouch tool `t` with diameter 16.
fn retouch_session(t: ToolId) -> (Session, ElementId) {
    let mut s = new_session();
    let b = bitmap(&mut s, 20, 20, 360, 260, [128, 128, 128, 255]);
    select(&mut s, &[b]);
    s.set_tool(t).expect("tool");
    if let Some(d) = s.active_mut() {
        d.history.clear();
    }
    let key = match t {
        ToolId::Blur => "BlurTool",
        ToolId::Sharpen => "SharpenTool",
        ToolId::Dodge => "DodgeTool",
        ToolId::Burn => "BurnTool",
        _ => "SmudgeTool",
    };
    s.prefs.set(&format!("{key}Diameter"), PrefValue::Int(16));
    s.prefs.set(&format!("{key}SoftEdge"), PrefValue::Int(0));
    (s, b)
}

#[test]
fn a_alt_restores_what_the_burn_stroke_changed() {
    let (mut s, b) = retouch_session(ToolId::Burn);
    let n0 = steps(&s);
    events(&mut s, &[ToolEvent::down(50.0, 100.0)]);
    drag(&mut s, (50.0, 100.0), (250.0, 100.0));
    assert!(s.tool_stroke_subtract_mode(true), "Alt announces the subtraction");
    // Alt with the button held: back over the right half
    drag_m(&mut s, (250.0, 100.0), (150.0, 100.0), |e| e.alt());
    assert!(s.tool_stroke_subtracting(), "subtraction mode");
    let p = preview(&mut s);
    assert_eq!(shown(&p, 200, 100)[3], 0, "the preview shows the bitmap unchanged there");
    assert_ne!(shown(&p, 100, 100)[3], 0, "the burnt part stays in the preview");
    // Alt released: burning again from here (downwards)
    drag(&mut s, (150.0, 100.0), (150.0, 200.0));
    assert!(!s.tool_stroke_subtracting());
    events(&mut s, &[ToolEvent::up(150.0, 200.0)]);
    assert_eq!(steps(&s), n0 + 1, "one row");
    let img = image(&s, b);
    assert!(px(&img, 100, 100)[0] < 128, "the part not restored stays burnt");
    assert_eq!(px(&img, 200, 100), [128, 128, 128, 255], "restored");
    assert_eq!(px(&img, 300, 100), [128, 128, 128, 255], "never touched");
    assert!(px(&img, 150, 180)[0] < 128, "burnt after Alt was released");
    let top = s.active().expect("document").history.top().cloned().expect("row");
    assert_eq!(top.command, "bitmap.retouch");
    assert!(top.params.get("subtract").is_some(), "the passes are part of the command");
    // the command alone gives the same pixels (replay without the live stroke)
    let (mut s2, b2) = retouch_session(ToolId::Burn);
    s2.execute("bitmap.retouch", top.params.clone()).expect("replay");
    let img2 = image(&s2, b2);
    for (x, y) in [(100, 100), (200, 100), (150, 180), (150, 100), (250, 100)] {
        assert_eq!(px(&img2, x, y), px(&img, x, y), "replay at {x},{y}");
    }
}

#[test]
fn a_alt_while_paused_restores_for_blur_sharpen_dodge() {
    for t in [ToolId::Blur, ToolId::Sharpen, ToolId::Dodge] {
        let (mut s, b) = retouch_session(t);
        // an edge to blur or sharpen: the right half of the bitmap white
        if t != ToolId::Dodge {
            let mut img = image(&s, b);
            for y in 20..280 {
                for x in 200..380 {
                    img.set_pixel(x, y, fp_color::Rgba8::new(255, 255, 255, 255));
                }
            }
            s.clipboard.clear();
            s.clipboard.pixels = Some(img);
            s.execute("edit.paste", json!({})).expect("paste");
            let id = top_elements(&s)[0].id;
            select(&mut s, &[id]);
        }
        let target = top_elements(&s)[0].id;
        let before = image(&s, target);
        events(&mut s, &[ToolEvent::down(200.0, 60.0)]);
        drag(&mut s, (200.0, 60.0), (200.0, 240.0));
        s.tool_event(&ToolEvent::key(ToolKey::Pause)).expect("P");
        events(&mut s, &[ToolEvent::up(200.0, 240.0)]);
        events(&mut s, &[ToolEvent::down(200.0, 150.0).alt()]);
        drag_m(&mut s, (200.0, 150.0), (200.0, 240.0), |e| e.alt());
        events(&mut s, &[ToolEvent::up(200.0, 240.0).alt()]);
        assert_eq!(s.tool_stroke_pause(), StrokePause::Paused, "{t:?} still paused");
        s.tool_event(&ToolEvent::key(ToolKey::Enter)).expect("Enter");
        let img = image(&s, target);
        assert!(
            (194..206).any(|x| px(&img, x, 80) != px(&before, x, 80)),
            "{t:?} worked above"
        );
        for x in [195, 199, 200, 201, 204] {
            assert_eq!(px(&img, x, 200), px(&before, x, 200), "{t:?} restored at {x}");
        }
    }
}

#[test]
fn a_smudge_has_no_alt_subtraction() {
    let (mut s, _) = retouch_session(ToolId::Smudge);
    events(&mut s, &[ToolEvent::down(50.0, 100.0)]);
    drag(&mut s, (50.0, 100.0), (150.0, 100.0));
    drag_m(&mut s, (150.0, 100.0), (100.0, 100.0), |e| e.alt());
    assert!(!s.tool_stroke_subtracting());
    assert!(!s.tool_stroke_subtract_mode(true), "no subtraction hint");
    events(&mut s, &[ToolEvent::up(100.0, 100.0)]);
}

#[test]
fn b_alt_toggle_preference_switches_the_mode_per_press() {
    let mut s = new_session();
    let b = bitmap(&mut s, 20, 20, 360, 260, [0, 0, 255, 255]);
    select(&mut s, &[b]);
    brush(&mut s, "Einfach/Präzise abger.", 12.0);
    s.prefs.set(STROKE_ALT_MODE_PREF, PrefValue::Int(2));
    events(&mut s, &[ToolEvent::down(50.0, 100.0)]);
    drag(&mut s, (50.0, 100.0), (250.0, 100.0));
    assert!(!s.tool_stroke_subtract_mode(false));
    // Alt pressed once (on one move) and released: subtraction stays on
    events(&mut s, &[ToolEvent::drag(249.0, 100.0).alt()]);
    assert!(s.tool_stroke_subtract_mode(false), "toggled on by the Alt press");
    drag(&mut s, (249.0, 100.0), (150.0, 100.0));
    assert!(s.tool_stroke_subtracting(), "subtracting without Alt held");
    // Alt again: painting again
    events(&mut s, &[ToolEvent::drag(150.0, 100.0).alt()]);
    events(&mut s, &[ToolEvent::drag(150.0, 101.0)]);
    assert!(!s.tool_stroke_subtract_mode(false), "toggled off");
    drag(&mut s, (150.0, 101.0), (150.0, 200.0));
    events(&mut s, &[ToolEvent::up(150.0, 200.0)]);
    let img = image(&s, b);
    assert_eq!(px(&img, 100, 101), [0, 0, 0, 255], "painted");
    assert_eq!(px(&img, 200, 101), [0, 0, 255, 255], "subtracted in toggle mode");
    assert_eq!(px(&img, 151, 170), [0, 0, 0, 255], "painted after the second Alt press");
    // the next stroke starts painting
    events(&mut s, &[ToolEvent::down(300.0, 50.0)]);
    drag(&mut s, (300.0, 50.0), (300.0, 80.0));
    assert!(!s.tool_stroke_subtract_mode(false), "a new stroke starts painting");
    events(&mut s, &[ToolEvent::up(300.0, 80.0)]);
}

#[test]
fn b_resume_key_resumes_only_with_the_button_held() {
    let mut s = new_session();
    brush(&mut s, "Einfach/Präzise abger.", 12.0);
    events(&mut s, &[ToolEvent::down(50.0, 100.0)]);
    drag(&mut s, (50.0, 100.0), (150.0, 100.0));
    s.tool_event(&ToolEvent::key(ToolKey::Pause)).expect("P");
    assert_eq!(s.tool_stroke_pause(), StrokePause::Paused);
    // moved while paused (button still down): nothing is painted
    drag(&mut s, (150.0, 100.0), (150.0, 200.0));
    // "hold" released P / "toggle" pressed P again: painting goes on at the pointer
    s.tool_event(&ToolEvent::key(ToolKey::Resume)).expect("resume");
    assert_eq!(s.tool_stroke_pause(), StrokePause::Painting);
    drag(&mut s, (150.0, 200.0), (250.0, 200.0));
    s.tool_event(&ToolEvent::key(ToolKey::Pause)).expect("P");
    events(&mut s, &[ToolEvent::up(250.0, 200.0)]);
    // released while paused: Resume does nothing, the next press resumes
    s.tool_event(&ToolEvent::key(ToolKey::Resume)).expect("resume");
    assert_eq!(s.tool_stroke_pause(), StrokePause::Paused);
    s.tool_event(&ToolEvent::key(ToolKey::Enter)).expect("Enter");
    let img = image(&s, top_elements(&s)[0].id);
    assert_eq!(px(&img, 100, 101)[3], 255, "first part");
    assert_eq!(px(&img, 151, 150)[3], 0, "not painted while paused");
    assert_eq!(px(&img, 200, 201)[3], 255, "painted after the resume");
}

#[test]
fn c_settings_changed_while_paused_apply_after_the_resume() {
    let mut s = new_session();
    let b = bitmap(&mut s, 20, 20, 360, 260, [255, 255, 255, 255]);
    select(&mut s, &[b]);
    brush(&mut s, "Einfach/Präzise abger.", 12.0);
    let n0 = steps(&s);
    events(&mut s, &[ToolEvent::down(50.0, 80.0)]);
    drag(&mut s, (50.0, 80.0), (250.0, 80.0));
    s.tool_event(&ToolEvent::key(ToolKey::Pause)).expect("P");
    events(&mut s, &[ToolEvent::up(250.0, 80.0)]);
    // while paused: size 30, opacity 50 %, red
    s.tools.options.brush_size = 30.0;
    s.tools.options.new_stroke =
        Some(json!({"preset": "Einfach/Präzise abger.", "width": 30, "softness": 0}));
    s.tools.options.opacity = 50;
    s.set_colors(Some(fp_color::Argb(0xffff_0000)), None);
    events(&mut s, &[ToolEvent::down(50.0, 180.0)]);
    drag(&mut s, (50.0, 180.0), (250.0, 180.0));
    let p = preview(&mut s);
    let live_wide = thickness(|x, y| shown(&p, x, y), 150, 150, 215);
    let live_red = shown(&p, 150, 180);
    // the new settings also apply to the subtraction tip
    events(&mut s, &[ToolEvent::up(250.0, 180.0)]);
    assert_eq!(steps(&s), n0 + 1, "one row for both parts");
    let img = image(&s, b);
    let thin = (60..100).filter(|y| px(&img, 150, *y) != [255, 255, 255, 255]).count();
    let wide = (150..215).filter(|y| px(&img, 150, *y) != [255, 255, 255, 255]).count();
    assert!((11..=13).contains(&thin), "the first part keeps size 12: {thin}");
    assert!((29..=31).contains(&wide), "the part after the resume has size 30: {wide}");
    assert_eq!(px(&img, 150, 80), [0, 0, 0, 255], "first part black at 100 %");
    let red = px(&img, 150, 180);
    assert!(red[0] == 255 && (120..=135).contains(&red[1]), "red at 50 %: {red:?}");
    assert_eq!(live_wide, wide, "the preview shows the new size");
    assert_eq!(live_red, red, "the preview shows the new colour and opacity");
    let top = s.active().expect("document").history.top().cloned().expect("row");
    assert_eq!(top.params["parts"].as_array().map(|a| a.len()), Some(1));
}

#[test]
fn c_alt_after_a_settings_change_erases_both_parts() {
    let mut s = new_session();
    let b = bitmap(&mut s, 20, 20, 360, 260, [0, 0, 255, 255]);
    select(&mut s, &[b]);
    brush(&mut s, "Einfach/Präzise abger.", 12.0);
    events(&mut s, &[ToolEvent::down(50.0, 100.0)]);
    drag(&mut s, (50.0, 100.0), (250.0, 100.0));
    s.tool_event(&ToolEvent::key(ToolKey::Pause)).expect("P");
    events(&mut s, &[ToolEvent::up(250.0, 100.0)]);
    s.tools.options.brush_size = 20.0;
    s.tools.options.new_stroke =
        Some(json!({"preset": "Einfach/Präzise abger.", "width": 20, "softness": 0}));
    events(&mut s, &[ToolEvent::down(200.0, 50.0)]);
    drag(&mut s, (200.0, 50.0), (200.0, 150.0));
    // Alt: back over the crossing of both parts
    drag_m(&mut s, (200.0, 150.0), (200.0, 60.0), |e| e.alt());
    events(&mut s, &[ToolEvent::up(200.0, 60.0).alt()]);
    let img = image(&s, b);
    assert_eq!(px(&img, 200, 101), [0, 0, 255, 255], "both parts erased at the crossing");
    assert_eq!(px(&img, 100, 101), [0, 0, 0, 255], "the first part elsewhere stays");
}

#[test]
fn c_retouch_size_changed_while_paused_applies_after_the_resume() {
    let (mut s, b) = retouch_session(ToolId::Burn);
    events(&mut s, &[ToolEvent::down(50.0, 80.0)]);
    drag(&mut s, (50.0, 80.0), (250.0, 80.0));
    s.tool_event(&ToolEvent::key(ToolKey::Pause)).expect("P");
    events(&mut s, &[ToolEvent::up(250.0, 80.0)]);
    s.prefs.set("BurnToolDiameter", PrefValue::Int(40));
    events(&mut s, &[ToolEvent::down(50.0, 180.0)]);
    drag(&mut s, (50.0, 180.0), (250.0, 180.0));
    let p = preview(&mut s);
    let live = (140..220).filter(|y| shown(&p, 150, *y)[3] != 0).count();
    events(&mut s, &[ToolEvent::up(250.0, 180.0)]);
    let img = image(&s, b);
    let first = (60..100).filter(|y| px(&img, 150, *y) != [128, 128, 128, 255]).count();
    let second = (140..220).filter(|y| px(&img, 150, *y) != [128, 128, 128, 255]).count();
    assert!((15..=17).contains(&first), "first part 16 px: {first}");
    assert!((39..=41).contains(&second), "second part 40 px: {second}");
    assert_eq!(live, second, "preview = result");
    // replay of the command
    let top = s.active().expect("document").history.top().cloned().expect("row");
    let (mut s2, b2) = retouch_session(ToolId::Burn);
    s2.execute("bitmap.retouch", top.params.clone()).expect("replay");
    let img2 = image(&s2, b2);
    for y in [70, 80, 90, 160, 180, 200] {
        assert_eq!(px(&img2, 150, y), px(&img, 150, y), "replay at y {y}");
    }
}

