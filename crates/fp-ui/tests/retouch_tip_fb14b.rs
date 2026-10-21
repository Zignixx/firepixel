//! FB-14b / TW-181: with the Blur tool (and Sharpen, Dodge, Burn, Smudge) on a bitmap the blue
//! outline of the brush tip must exist only once, at the pointer. Painting two strokes with real
//! pointer events over many frames must not leave one tip circle per stroke point on the canvas,
//! neither while dragging nor after the release.

use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use fp_engine::Session;
use fp_engine::tools::{Overlay, ToolId};
use fp_geom::IRect;
use fp_ui::FirepixelApp;
use fp_ui::app_state::{AppState, UiSettings};
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn app() -> FirepixelApp {
    let mut s = Session::default();
    s.execute(
        "file.new",
        json!({"width": 600, "height": 400, "background": "#ffffff"}),
    )
    .expect("document");
    let img = fp_raster::RgbaImage::filled(
        IRect::new(50, 50, 550, 350),
        fp_color::Rgba8::new(100, 100, 100, 255),
    )
    .expect("image");
    s.clipboard.clear();
    s.clipboard.pixels = Some(img);
    s.execute("edit.paste", json!({})).expect("bitmap");
    let ctx = egui::Context::default();
    let mut a = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    a.state.sync_render = true;
    a
}

fn harness(a: FirepixelApp) -> H {
    Harness::builder()
        .renderer(crate::gpu_common::renderer())
        .with_size(egui::vec2(1400.0, 900.0))
        .with_pixels_per_point(1.0)
        .with_max_steps(16)
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), a)
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn screen(h: &H, x: f32, y: f32) -> Pos2 {
    let s = st(h);
    let d = s.session.active().expect("document");
    let pr = s
        .canvases
        .get(&d.id)
        .and_then(|c| c.page_screen)
        .expect("page");
    let r = fp_render::Renderer::page_rect(&d.doc, d.page).expect("page rect");
    let (w, ht) = ((r.right - r.left) as f32, (r.bottom - r.top) as f32);
    egui::pos2(
        pr.left() + (x - r.left as f32) / w * pr.width(),
        pr.top() + (y - r.top as f32) / ht * pr.height(),
    )
}

/// Overlays of the active tool in the last frame (what the canvas painted).
fn feedback(h: &H) -> Vec<Overlay> {
    let s = st(h);
    let d = s.session.active().expect("document");
    s.canvases
        .get(&d.id)
        .map(|c| c.feedback.clone())
        .unwrap_or_default()
}

/// Circle outlines of the tip size (diameter 13 at 100 %, radius 6.5) painted in the last frame.
fn tip_circles(h: &H) -> usize {
    fn count(s: &egui::Shape) -> usize {
        match s {
            egui::Shape::Circle(c) if (c.radius - 6.5).abs() < 0.6 && c.fill.a() == 0 => 1,
            egui::Shape::Vec(v) => v.iter().map(count).sum(),
            _ => 0,
        }
    }
    h.output().shapes.iter().map(|cs| count(&cs.shape)).sum()
}

fn strokes_in(fb: &[Overlay]) -> usize {
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

/// One stroke with real pointer events, one frame per ~4 px. Returns the most stroke-point
/// circles in the tool feedback, tips in the feedback and tip circles painted in one frame.
fn stroke(h: &mut H, from: (f32, f32), to: (f32, f32)) -> (usize, usize, usize) {
    let p0 = screen(h, from.0, from.1);
    h.event(Event::PointerMoved(p0));
    h.run_steps(1);
    h.event(Event::PointerButton {
        pos: p0,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    h.run_steps(1);
    let len = ((to.0 - from.0).powi(2) + (to.1 - from.1).powi(2)).sqrt();
    let k = (len / 4.0).ceil().max(1.0) as usize;
    let mut last = p0;
    let (mut strokes, mut tips, mut circles) = (0, 0, 0);
    for i in 1..=k {
        let t = i as f32 / k as f32;
        last = screen(
            h,
            from.0 + (to.0 - from.0) * t,
            from.1 + (to.1 - from.1) * t,
        );
        h.event(Event::PointerMoved(last));
        h.run_steps(1);
        let fb = feedback(h);
        strokes = strokes.max(strokes_in(&fb));
        tips = tips.max(tips_in(&fb));
        circles = circles.max(tip_circles(h));
    }
    h.event(Event::PointerButton {
        pos: last,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    h.run_steps(2);
    (strokes, tips, circles)
}

#[test]
fn retouch_tools_leave_no_tip_outlines_behind() {
    for tool in [
        ToolId::Blur,
        ToolId::Sharpen,
        ToolId::Dodge,
        ToolId::Burn,
        ToolId::Smudge,
    ] {
        let mut h = harness(app());
        h.run_steps(3);
        h.state_mut().state.session.set_tool(tool).expect("tool");
        h.run_steps(2);
        let a = stroke(&mut h, (120.0, 120.0), (300.0, 160.0));
        let b = stroke(&mut h, (150.0, 250.0), (400.0, 220.0));
        // after the release, the pointer rests: only the one tip at the pointer remains
        // (checked first: the trail is cleared on mouse-up)
        let p = screen(&h, 420.0, 300.0);
        h.event(Event::PointerMoved(p));
        h.run_steps(2);
        let fb = feedback(&h);
        assert_eq!(
            strokes_in(&fb),
            0,
            "{tool:?}: no stroke circles left after the release: {fb:?}"
        );
        assert_eq!(
            tips_in(&fb),
            1,
            "{tool:?}: exactly one tip at the pointer: {fb:?}"
        );
        assert!(
            tip_circles(&h) <= 1,
            "{tool:?}: at most one tip circle painted after the strokes: {}",
            tip_circles(&h)
        );
        let undo = st(&h)
            .session
            .active()
            .map(|d| d.history.undo_count())
            .unwrap_or(0);
        assert!(
            undo >= 2,
            "{tool:?}: both strokes were applied ({undo} steps)"
        );
        // while dragging: the tool feedback and the painted frame hold one tip only
        for (n, (strokes, tips, circles)) in [a, b].into_iter().enumerate() {
            assert_eq!(
                strokes, 0,
                "{tool:?}: no tip outline per stroke point while dragging (stroke {n})"
            );
            assert_eq!(
                tips, 1,
                "{tool:?}: one tip at the pointer while dragging (stroke {n})"
            );
            assert!(
                circles <= 1,
                "{tool:?}: at most one tip circle painted per frame while dragging (stroke {n}): {circles}"
            );
        }
    }
}
