//! F153 (FB-139, TW-287): the live move drag zoomed in (200 % to 800 %) on small objects.
//!
//! Measurement (release, software rendering):
//! `cargo test -p fp-ui --release --test all drag_zoom_perf_f153::bench -- --ignored --nocapture`

use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use fp_engine::Session;
use fp_engine::tools::ToolId;
use fp_ui::FirepixelApp;
use fp_ui::app_state::UiSettings;
use serde_json::json;
use std::time::{Duration, Instant};

type H = Harness<'static, FirepixelApp>;

fn harness(s: Session, sync: bool, size: (f32, f32)) -> H {
    let ctx = egui::Context::default();
    let mut a = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    a.state.os_clipboard = false;
    a.state.sync_render = sync;
    let mut h = Harness::builder()
        .with_size(egui::vec2(size.0, size.1))
        .with_pixels_per_point(1.0)
        .with_max_steps(8)
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), a);
    h.run_steps(4);
    h
}

fn ok(s: &mut Session, cmd: &str, p: serde_json::Value) {
    s.execute(cmd, p).unwrap_or_else(|e| panic!("{cmd}: {e:?}"));
}

/// Scenes of the measurement (FB-139).
#[derive(Clone, Copy, Debug, PartialEq)]
enum Scene {
    /// A small bitmap (47 × 34, like the tester's video).
    Bitmap,
    /// A group of 9 small rectangles.
    Group9,
    /// A small bitmap masked by an ellipse (mask group).
    Masked,
}

/// Grab point (document) of the scene's selection.
const GRAB: (f32, f32) = (400.0, 300.0);

fn scene(sc: Scene) -> Session {
    let mut s = Session::default();
    ok(
        &mut s,
        "file.new",
        json!({"width": 800, "height": 600, "background": "#d0d8e0"}),
    );
    // something below and above, like a real page
    ok(
        &mut s,
        "draw.rectangle",
        json!({"rect": [40, 40, 500, 380], "fillColor": "#3366ff", "strokeWidth": 0}),
    );
    let bitmap = |s: &mut Session| {
        ok(
            s,
            "draw.rectangle",
            json!({"rect": [377, 283, 424, 317], "fillColor": "#cc6633", "strokeWidth": 0}),
        );
        ok(s, "modify.flattenSelection", json!({}));
    };
    match sc {
        Scene::Bitmap => bitmap(&mut s),
        Scene::Group9 => {
            let mut ids = Vec::new();
            for j in 0..3 {
                for i in 0..3 {
                    let (x, y) = (370 + i * 20, 280 + j * 15);
                    ok(
                        &mut s,
                        "draw.rectangle",
                        json!({"rect": [x, y, x + 15, y + 12], "fillColor": "#cc6633", "strokeWidth": 0}),
                    );
                    ids.push(s.active().expect("doc").selection.elements[0].0);
                }
            }
            ok(&mut s, "select.set", json!({"ids": ids}));
            ok(&mut s, "modify.group", json!({}));
        }
        Scene::Masked => {
            bitmap(&mut s);
            let b = s.active().expect("doc").selection.elements[0].0;
            ok(
                &mut s,
                "draw.ellipse",
                json!({"rect": [380, 285, 420, 315], "fillColor": "#ffffff", "strokeWidth": 0}),
            );
            let m = s.active().expect("doc").selection.elements[0].0;
            ok(&mut s, "select.set", json!({"ids": [b, m]}));
            ok(&mut s, "modify.mask.groupAsMask", json!({}));
        }
    }
    let sel = s.active().expect("doc").selection.elements.clone();
    ok(
        &mut s,
        "draw.rectangle",
        json!({"rect": [450, 320, 700, 560], "fillColor": "#808080", "strokeWidth": 0}),
    );
    let ids: Vec<u64> = sel.iter().map(|i| i.0).collect();
    ok(&mut s, "select.set", json!({"ids": ids}));
    s
}

/// Document point → screen point of the canvas.
fn at(h: &H, x: f32, y: f32) -> Pos2 {
    let s = &h.state().state;
    let d = s.session.active().expect("document");
    let pr = s
        .canvases
        .get(&d.id)
        .and_then(|c| c.page_screen)
        .expect("page on screen");
    let r = fp_render::Renderer::page_rect(&d.doc, d.page).expect("page");
    let (w, ht) = ((r.right - r.left) as f32, (r.bottom - r.top) as f32);
    egui::pos2(
        pr.left() + (x - r.left as f32) / w * pr.width(),
        pr.top() + (y - r.top as f32) / ht * pr.height(),
    )
}

fn button(h: &mut H, p: Pos2, pressed: bool) {
    h.input_mut().events.push(Event::PointerButton {
        pos: p,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    });
    h.step();
}

fn settle(h: &mut H) {
    for _ in 0..400 {
        h.step();
        let cc = h.state().state.canvases.values().next().expect("canvas");
        if cc.requested.is_some() && cc.requested == cc.shown {
            return;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn pct(v: &mut [f64], p: f64) -> f64 {
    v.sort_by(|a, b| a.total_cmp(b));
    let i = ((v.len() as f64 - 1.0) * p).round() as usize;
    v.get(i).copied().unwrap_or(0.0)
}

struct Run {
    ui_med: f64,
    ui_max: f64,
    /// frames over 50 ms
    stalls: usize,
    /// shown live images per second
    sps: f64,
    /// largest live image (device pixels)
    max_px: u64,
}

/// Fast circles (radius 30 document pixels, one turn per second) for `secs` seconds in real time
/// at 60 frames per second; every third frame the pointer does not move (a mouse slower than the
/// display).
fn measure(sc: Scene, zoom: f64, secs: f64, size: (f32, f32), legacy: bool) -> Run {
    let mut h = harness(scene(sc), false, size);
    // the budget fallback (F147) off: the measurement shows the live path itself (on a loaded
    // machine two slow steps would switch to the sprites); `legacy`: as before F153
    canvas_budget(&mut h);
    h.state_mut()
        .state
        .canvases
        .values_mut()
        .for_each(|c| c.legacy_live = legacy);
    let _ = h.state_mut().state.session.set_tool(ToolId::Pointer);
    if let Some(d) = h.state_mut().state.session.active_mut() {
        d.view.snap_to_smart_guides = false;
    }
    let _ = h
        .state_mut()
        .state
        .session
        .execute("view.setZoom", json!({"zoom": zoom, "center": [GRAB.0, GRAB.1]}));
    settle(&mut h);
    let a = at(&h, GRAB.0, GRAB.1);
    h.input_mut().events.push(Event::PointerMoved(a));
    h.step();
    button(&mut h, a, true);
    let mut ui = Vec::new();
    let mut shown = std::collections::HashSet::new();
    let mut max_px = 0u64;
    let r = 30.0 * zoom as f32;
    let t0 = Instant::now();
    let mut i = 0;
    while t0.elapsed().as_secs_f64() < secs {
        let f0 = Instant::now();
        i += 1;
        if i % 3 != 0 {
            let ang = t0.elapsed().as_secs_f32() * std::f32::consts::TAU;
            let p = a + egui::vec2(r * ang.cos() - r + 10.0, r * ang.sin());
            h.input_mut().events.push(Event::PointerMoved(p));
        }
        h.step();
        ui.push(f0.elapsed().as_secs_f64() * 1000.0);
        if let Some(c) = h.state().state.canvases.values().next()
            && let Some(k) = c.shown
            && k.preview != 0
        {
            shown.insert(k.preview);
            max_px = max_px.max(k.dev.width() as u64 * k.dev.height() as u64);
        }
        let el = f0.elapsed();
        if el < Duration::from_millis(16) {
            std::thread::sleep(Duration::from_millis(16) - el);
        }
    }
    let total = t0.elapsed().as_secs_f64();
    if h
        .state()
        .state
        .canvases
        .values()
        .next()
        .is_some_and(|c| c.live_fallback)
    {
        println!("F153 {sc:?} {zoom}: sprite fallback (budget)");
    }
    button(&mut h, a, false);
    let stalls = ui.iter().filter(|&&t| t > 50.0).count();
    Run {
        ui_med: pct(&mut ui, 0.5),
        ui_max: pct(&mut ui, 1.0),
        stalls,
        sps: shown.len() as f64 / total,
        max_px,
    }
}

/// Render thread time per live move step (median of `n` steps of one document pixel), measured on
/// the app's renderer: (whole visible part as before F153, only the changed pixels against the
/// previous step as since F153).
fn step_costs(sc: Scene, zoom: f64, size: (f32, f32), n: usize) -> (f64, f64) {
    let mut h = harness(scene(sc), true, size);
    let _ = h
        .state_mut()
        .state
        .session
        .execute("view.setZoom", json!({"zoom": zoom, "center": [GRAB.0, GRAB.1]}));
    h.run_steps(6);
    let st = &h.state().state;
    let d = st.session.active().expect("doc");
    let key = st.canvases.get(&d.id).and_then(|c| c.shown).expect("shown");
    let pr = key.page_rect;
    let ids = d.selection.elements.clone();
    let step = |i: usize| {
        let lm = fp_ui::canvas::ghost::LiveMove {
            ids: ids.clone(),
            dx: i as f32,
            dy: 0.0,
            copy: false,
        };
        fp_ui::canvas::ghost::apply_live_move(&d.doc, d.page, &lm)
    };
    let (mut whole, mut patch) = (Vec::new(), Vec::new());
    let (_, mut base) = st.renderer.render_device_listed(
        &d.doc,
        d.page,
        d.frame,
        pr,
        (pr.left, pr.top),
        key.zoom,
        key.dev,
    );
    for i in 1..=n {
        let (doc, moved) = step(i);
        let t = Instant::now();
        let img = st.renderer.with_moved_looks(&moved, |r| {
            r.render_device(&doc, d.page, d.frame, pr, (pr.left, pr.top), key.zoom, key.dev)
        });
        std::hint::black_box(img);
        whole.push(t.elapsed().as_secs_f64() * 1000.0);
        let t = Instant::now();
        let p = st.renderer.with_moved_looks(&moved, |r| {
            r.render_device_patch(
                &doc,
                d.page,
                d.frame,
                pr,
                (pr.left, pr.top),
                key.zoom,
                key.dev,
                base.as_ref().expect("list"),
            )
        });
        patch.push(t.elapsed().as_secs_f64() * 1000.0);
        base = p.map(|p| p.list);
    }
    (pct(&mut whole, 0.5), pct(&mut patch, 0.5))
}

#[test]
#[ignore = "measurement (release): cargo test -p fp-ui --release --test all drag_zoom_perf_f153::bench -- --ignored --nocapture"]
fn bench() {
    fp_engine::shell::disable();
    for size in [(1400.0, 900.0), (2400.0, 1400.0)] {
        for sc in [Scene::Bitmap, Scene::Group9, Scene::Masked] {
            for zoom in [1.0, 2.0, 4.0, 8.0] {
                let (whole, patch) = step_costs(sc, zoom, size, 40);
                println!(
                    "F153 {:>4}x{:<4} {sc:<7?} {:>4} %: render per step: whole part {whole:6.2} ms, changed pixels {patch:6.2} ms",
                    size.0,
                    size.1,
                    zoom * 100.0
                );
                for legacy in [true, false] {
                let r = measure(sc, zoom, 3.0, size, legacy);
                println!(
                    "F153 {:>4}x{:<4} {sc:<7?} {:>4} % {}: UI median {:5.1} ms, max {:6.1} ms, frames > 50 ms {:3}, {:5.1} live images/s, largest live image {:>5} kpx",
                    size.0,
                    size.1,
                    zoom * 100.0,
                    if legacy { "before" } else { "after " },
                    r.ui_med,
                    r.ui_max,
                    r.stalls,
                    r.sps,
                    r.max_px / 1000
                );
                }
            }
        }
    }
}

// ------------------------------------------------------------------ behaviour (not ignored)

fn canvas(h: &H) -> &fp_ui::canvas::render::CanvasCache {
    h.state().state.canvases.values().next().expect("canvas")
}

fn move_to(h: &mut H, p: Pos2) {
    h.input_mut().events.push(Event::PointerMoved(p));
    h.step();
}

/// Offset of the running move drag (the Pointer's preview).
fn drag_offset(h: &H) -> Option<(f32, f32)> {
    h.state()
        .state
        .session
        .tool_overlay()
        .into_iter()
        .find_map(|o| match o {
            fp_engine::Overlay::Ghost {
                pixels: false,
                moved_by: Some(m),
                ..
            } => Some((m.x, m.y)),
            _ => None,
        })
}

/// The shown canvas image equals a whole render of the document with the drag applied.
fn assert_equals_whole_render(h: &H, what: &str) {
    let st = &h.state().state;
    let d = st.session.active().expect("doc");
    let cc = canvas(h);
    let k = cc.shown.expect("shown");
    let doc = match drag_offset(h) {
        Some((dx, dy)) => {
            let lm = fp_ui::canvas::ghost::LiveMove {
                ids: d.selection.elements.clone(),
                dx,
                dy,
                copy: false,
            };
            fp_ui::canvas::ghost::apply_live_move(&d.doc, d.page, &lm).0
        }
        None => std::sync::Arc::clone(&d.doc),
    };
    let pr = k.page_rect;
    let whole = st
        .renderer
        .render_device(&doc, d.page, d.frame, pr, (pr.left, pr.top), k.zoom, k.dev)
        .image;
    let shown = cc.shown_image.as_ref().expect("image");
    assert_eq!(
        (shown.width(), shown.height()),
        (whole.width(), whole.height()),
        "{what}: size"
    );
    assert!(
        shown.as_bytes() == whole.as_bytes(),
        "{what}: the shown image differs from a whole render"
    );
}

/// F153 (FB-139): zoomed in, a live move step redraws only the pixels of the moved objects on
/// the shown image (its part and margin stay, no render of the whole view), every step is
/// pixel-identical to a whole render, and resting the pointer starts no margin render.
#[test]
fn zoomed_live_move_patches_the_shown_image() {
    for sc in [Scene::Bitmap, Scene::Group9, Scene::Masked] {
        let mut s = scene(sc);
        // a live effect that reaches beyond the objects
        ok(&mut s, "element.addEffect", json!({"effect": "dropShadow"}));
        let mut h = harness(s, true, (900.0, 700.0));
        canvas_budget(&mut h);
        let _ = h.state_mut().state.session.set_tool(ToolId::Pointer);
        if let Some(d) = h.state_mut().state.session.active_mut() {
            d.view.snap_to_smart_guides = false;
        }
        let _ = h
            .state_mut()
            .state
            .session
            .execute("view.setZoom", json!({"zoom": 4, "center": [GRAB.0, GRAB.1]}));
        h.run_steps(6);
        let k0 = canvas(&h).shown.expect("shown");
        assert_eq!(k0.preview, 0);
        let a = at(&h, GRAB.0, GRAB.1);
        move_to(&mut h, a);
        button(&mut h, a, true);
        for (i, dx) in [12.0f32, 20.0, 36.0, 37.0, 60.0, 100.0].into_iter().enumerate() {
            move_to(&mut h, a + egui::vec2(dx, dx * 0.5));
            h.run_steps(3);
            let cc = canvas(&h);
            let k = cc.shown.expect("shown");
            assert!(k.preview != 0, "{sc:?} step {i}: live image");
            assert_eq!(k.dev, k0.dev, "{sc:?} step {i}: the shown part stays (patched)");
            assert_eq!(cc.requested, cc.shown, "{sc:?} step {i}: no margin render");
            assert_equals_whole_render(&h, &format!("{sc:?} step {i}"));
        }
        button(&mut h, a + egui::vec2(100.0, 50.0), false);
        h.run_steps(6);
        assert_eq!(canvas(&h).shown.map(|k| k.preview), Some(0), "committed");
        assert_equals_whole_render(&h, &format!("{sc:?} released"));
    }
}

fn canvas_budget(h: &mut H) {
    h.state_mut()
        .state
        .canvases
        .values_mut()
        .for_each(|c| c.live_budget_ms = Some(f64::INFINITY));
}

/// F153 (FB-139): Properties X/Y follow a dragged group live (as a single object), not only on
/// the release.
#[test]
fn properties_box_follows_a_dragged_group() {
    let mut h = harness(scene(Scene::Group9), true, (900.0, 700.0));
    let _ = h.state_mut().state.session.set_tool(ToolId::Pointer);
    if let Some(d) = h.state_mut().state.session.active_mut() {
        d.view.snap_to_smart_guides = false;
    }
    h.run_steps(2);
    let a = at(&h, GRAB.0, GRAB.1);
    move_to(&mut h, a);
    button(&mut h, a, true);
    let b = at(&h, GRAB.0 + 30.0, GRAB.1 + 12.0);
    move_to(&mut h, b);
    h.run_steps(2);
    let off = drag_offset(&h).expect("move drag");
    assert!((29.0..=31.0).contains(&off.0) && (11.0..=13.0).contains(&off.1), "{off:?}");
    assert_eq!(
        fp_ui::inspector::live_move_offset(&h.state().state),
        Some(off),
        "the box shows the moved bounds while the button is held"
    );
    button(&mut h, b, false);
    h.run_steps(2);
    assert_eq!(fp_ui::inspector::live_move_offset(&h.state().state), None);
}
