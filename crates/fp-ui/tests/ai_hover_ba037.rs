//! BA-037: object-selection preview while hovering, offscreen with egui_kittest and a stub decoder
//! (no model files needed):
//! * the outline appears only after the pointer rested for the debounce time, disappears when the
//!   pointer leaves the canvas, and only with the preference on
//! * a result that arrives after the pointer moved on is dropped (never shown)
//! * a tool change cancels the preview

use egui::{Event, Pos2};
use egui_kittest::Harness;
use fp_engine::commands::ai as ai;
use fp_engine::commands::ai::fp_ai::pipeline::{ObjectPrompt, ObjectRaw};
use fp_engine::commands::ai::fp_ai::sam2;
use fp_engine::prefs::PrefValue;
use fp_geom::IRect;
use fp_raster::RgbaImage;
use fp_ui::app_state::UiSettings;
use fp_ui::canvas::tools::ai_object::DEBOUNCE;
use fp_ui::FirepixelApp;
use serde_json::json;
use std::time::{Duration, Instant};

type H = Harness<'static, FirepixelApp>;

const STEP: f32 = 1.0 / 60.0;

fn raw_disk(r: IRect, prompt: &ObjectPrompt) -> ObjectRaw {
    let (cx, cy) = prompt.points.first().map(|q| (q.x, q.y)).unwrap_or((0.0, 0.0));
    let n = sam2::LOW;
    let (w, h) = (r.width() as f32, r.height() as f32);
    let mut plane = vec![-10.0f32; n * n];
    for j in 0..n {
        for i in 0..n {
            let x = r.left as f32 + (i as f32 + 0.5) * w / n as f32;
            let y = r.top as f32 + (j as f32 + 0.5) * h / n as f32;
            if (x - cx).hypot(y - cy) <= 30.0 {
                plane[j * n + i] = 10.0;
            }
        }
    }
    let mut all = Vec::with_capacity(4 * n * n);
    for _ in 0..4 {
        all.extend_from_slice(&plane);
    }
    ObjectRaw { masks: std::sync::Arc::new(all), ious: [0.9; 4], pick: 0, object_score: 5.0, region: r, cached: false }
}

fn fast_stub(_img: &RgbaImage, r: IRect, prompt: &ObjectPrompt) -> ai::fp_ai::Result<ObjectRaw> {
    Ok(raw_disk(r, prompt))
}

fn slow_stub(_img: &RgbaImage, r: IRect, prompt: &ObjectPrompt) -> ai::fp_ai::Result<ObjectRaw> {
    std::thread::sleep(Duration::from_millis(400));
    Ok(raw_disk(r, prompt))
}

struct StubGuard;

impl Drop for StubGuard {
    fn drop(&mut self) {
        ai::set_object_stub(None);
    }
}

fn harness(stub: ai::ObjectStub, enabled: bool) -> (H, StubGuard) {
    fp_engine::shell::disable();
    ai::fp_ai::download::disable();
    ai::set_object_stub(Some(stub));
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("doc");
    let mut img = RgbaImage::new(IRect::from_xywh(0, 0, 300, 200)).expect("image");
    for (i, px) in img.data_mut().iter_mut().enumerate() {
        *px = [(i % 251) as u8, 120, (i % 97) as u8, 255];
    }
    s.clipboard.clear();
    s.clipboard.pixels = Some(img);
    s.execute("edit.paste", json!({})).expect("paste");
    s.execute("select.none", json!({})).expect("none");
    s.execute("tool.select", json!({"tool": "tool.aiObject"})).expect("tool");
    s.prefs.set("AiObjectHoverPreview", PrefValue::Bool(enabled));
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .with_pixels_per_point(1.0)
        .with_max_steps(12)
        .with_step_dt(STEP)
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    (h, StubGuard)
}

fn screen_of(h: &H, x: f32, y: f32) -> Pos2 {
    let st = &h.state().state;
    let d = st.session.active().expect("doc");
    let area = st.canvases.get(&d.id).and_then(|c| c.area).expect("canvas");
    let (sx, sy) = d.view.doc_to_screen(fp_geom::Point::new(x, y));
    Pos2::new(area.min.x + sx as f32, area.min.y + sy as f32)
}

fn hover(h: &H) -> &fp_ui::canvas::tools::ai_object::ObjectHover {
    &h.state().state.dialogs.ai.hover
}

/// Steps (each `STEP` seconds of input time) until `f` holds; real time out after 5 s.
fn until(h: &mut H, f: impl Fn(&H) -> bool) -> bool {
    let t0 = Instant::now();
    while t0.elapsed() < Duration::from_secs(5) {
        if f(h) {
            return true;
        }
        h.step();
        std::thread::sleep(Duration::from_millis(5));
    }
    f(h)
}

#[test]
fn outline_appears_after_debounce_and_goes_away_on_pointer_leave() {
    let (mut h, _g) = harness(fast_stub, true);
    let p = screen_of(&h, 120.0, 100.0);
    h.event(Event::PointerMoved(p));
    h.run_steps(2);
    assert!(!hover(&h).is_shown(), "no preview before the debounce");
    assert_eq!(hover(&h).stats().started, 0, "no job before the debounce ({DEBOUNCE} s)");
    assert!(until(&mut h, |h| hover(h).is_shown()), "preview after the debounce");
    assert_eq!(hover(&h).stats().started, 1, "one job for one resting position");
    let b = hover(&h).shown_mask().map(|m| m.tight_bounds()).expect("mask");
    assert!((b.left - 90).abs() <= 3 && (b.right - 150).abs() <= 3, "outline around the pointer: {b:?}");
    // the UI did not select anything by hovering
    assert!(h.state().state.session.active().is_some_and(|d| !d.selection.has_pixels()));
    h.event(Event::PointerGone);
    h.run_steps(2);
    assert!(!hover(&h).is_shown(), "pointer left the canvas: preview gone");
}

#[test]
fn stale_results_are_dropped() {
    let (mut h, _g) = harness(slow_stub, true);
    let p = screen_of(&h, 120.0, 100.0);
    h.event(Event::PointerMoved(p));
    assert!(until(&mut h, |h| hover(h).job_running()), "job started");
    // the pointer moves on while the job runs: its result must never be shown
    let q = screen_of(&h, 220.0, 150.0);
    h.event(Event::PointerMoved(q));
    h.run_steps(2);
    assert!(until(&mut h, |h| hover(h).stats().dropped == 1), "stale result dropped");
    // only one job at a time: the second one starts after the first ended
    assert!(until(&mut h, |h| hover(h).is_shown()), "preview of the new position");
    let b = hover(&h).shown_mask().map(|m| m.tight_bounds()).expect("mask");
    assert!((b.left - 190).abs() <= 3, "outline belongs to the new position: {b:?}");
    assert_eq!(hover(&h).stats().shown, 1, "the stale result was never shown");
}

#[test]
fn off_by_default_and_cancelled_on_tool_change() {
    let (mut h, _g) = harness(fast_stub, false);
    let p = screen_of(&h, 120.0, 100.0);
    h.event(Event::PointerMoved(p));
    for _ in 0..20 {
        h.step();
    }
    assert_eq!(hover(&h).stats().started, 0, "preference off: no preview");
    h.state_mut().state.session.prefs.set("AiObjectHoverPreview", PrefValue::Bool(true));
    assert!(until(&mut h, |h| hover(h).is_shown()), "preview with the preference on");
    h.state_mut().state.session.execute("tool.select", json!({"tool": "tool.marquee"})).expect("tool");
    h.run_steps(2);
    assert!(!hover(&h).is_shown(), "other tool: preview gone");
}
