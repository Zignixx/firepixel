//! F147 (FB-131 … FB-135, TW-282): cost of the live move drag of F141.
//!
//! Measurement (release, software rendering):
//! `cargo test -p fp-ui --release --test all drag_live_perf_f147::bench -- --ignored --nocapture`

use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use fp_engine::Session;
use fp_engine::tools::ToolId;
use fp_ui::FirepixelApp;
use fp_ui::app_state::UiSettings;
use serde_json::json;
use std::time::{Duration, Instant};

type H = Harness<'static, FirepixelApp>;

fn harness(s: Session, sync: bool) -> H {
    let ctx = egui::Context::default();
    let mut a = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    a.state.os_clipboard = false;
    a.state.sync_render = sync;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .with_pixels_per_point(1.0)
        .with_max_steps(8)
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), a);
    h.run_steps(4);
    h
}

fn ok(s: &mut Session, cmd: &str, p: serde_json::Value) {
    s.execute(cmd, p).unwrap_or_else(|e| panic!("{cmd}: {e:?}"));
}

/// Scenes of the measurement (FB-134, FB-135).
#[derive(Clone, Copy, Debug, PartialEq)]
enum Scene {
    /// A large white rectangle covering most of a 1600 × 1200 page.
    Large,
    /// The same with Inner Bevel.
    LargeBevel,
    /// The same with Glow.
    LargeGlow,
    /// 400 small rectangles (20 × 20), all selected.
    Cluster,
}

fn scene(sc: Scene) -> Session {
    let mut s = Session::default();
    ok(
        &mut s,
        "file.new",
        json!({"width": 1600, "height": 1200, "background": "#d0d8e0"}),
    );
    // something below and above, like a real page
    ok(
        &mut s,
        "draw.rectangle",
        json!({"rect": [40, 40, 400, 300], "fillColor": "#3366ff", "strokeWidth": 0}),
    );
    match sc {
        Scene::Cluster => {
            let mut ids = Vec::new();
            for j in 0..20 {
                for i in 0..20 {
                    let (x, y) = (300 + i * 40, 200 + j * 40);
                    ok(
                        &mut s,
                        "draw.rectangle",
                        json!({"rect": [x, y, x + 30, y + 30], "fillColor": "#cc6633", "strokeWidth": 0}),
                    );
                    ids.push(s.active().expect("doc").selection.elements[0].0);
                }
            }
            ok(&mut s, "select.set", json!({"ids": ids}));
        }
        _ => {
            ok(
                &mut s,
                "draw.rectangle",
                json!({"rect": [100, 100, 1500, 1100], "fillColor": "#ffffff", "strokeWidth": 0}),
            );
            match sc {
                Scene::LargeBevel => ok(&mut s, "element.addEffect", json!({"effect": "innerBevel"})),
                Scene::LargeGlow => ok(&mut s, "element.addEffect", json!({"effect": "glow"})),
                _ => {}
            }
        }
    }
    let sel = s.active().expect("doc").selection.elements.clone();
    ok(
        &mut s,
        "draw.rectangle",
        json!({"rect": [1200, 900, 1560, 1160], "fillColor": "#808080", "strokeWidth": 0}),
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

fn median(v: &mut [f64]) -> f64 {
    v.sort_by(|a, b| a.total_cmp(b));
    v.get(v.len() / 2).copied().unwrap_or(0.0)
}

/// One measurement: drag for `secs` seconds in real time at 60 frames per second, one document
/// pixel per frame; returns (UI ms per frame median, render ms per step median, shown live images
/// per second).
fn measure(sc: Scene, zoom: f64, secs: f64) -> (f64, f64, f64) {
    let mut h = harness(scene(sc), false);
    let _ = h.state_mut().state.session.set_tool(ToolId::Pointer);
    if let Some(d) = h.state_mut().state.session.active_mut() {
        d.view.snap_to_smart_guides = false;
    }
    let grab = match sc {
        Scene::Cluster => (315.0, 215.0),
        _ => (700.0, 600.0),
    };
    let _ = h
        .state_mut()
        .state
        .session
        .execute("view.setZoom", json!({"zoom": zoom, "center": [grab.0, grab.1]}));
    settle(&mut h);
    // render thread time per step, measured on the renderer directly like the worker does
    let render_ms = render_steps(&h, 12);
    let a = at(&h, grab.0, grab.1);
    h.input_mut().events.push(Event::PointerMoved(a));
    h.step();
    button(&mut h, a, true);
    let mut ui = Vec::new();
    let mut shown = std::collections::HashSet::new();
    let t0 = Instant::now();
    let mut i = 0;
    while t0.elapsed().as_secs_f64() < secs {
        let f0 = Instant::now();
        i += 1;
        let p = a + egui::vec2((i as f64 * zoom) as f32, 0.0) + egui::vec2(10.0, 0.0);
        h.input_mut().events.push(Event::PointerMoved(p));
        h.step();
        ui.push(f0.elapsed().as_secs_f64() * 1000.0);
        if let Some(k) = h.state().state.canvases.values().next().and_then(|c| c.shown)
            && k.preview != 0
        {
            shown.insert(k.preview);
        }
        let el = f0.elapsed();
        if el < Duration::from_millis(16) {
            std::thread::sleep(Duration::from_millis(16) - el);
        }
    }
    let total = t0.elapsed().as_secs_f64();
    let fallback = h
        .state()
        .state
        .canvases
        .values()
        .next()
        .is_some_and(|c| c.live_fallback);
    if fallback {
        println!("F147 {sc:?} {zoom}: sprite fallback (budget)");
    }
    button(&mut h, a, false);
    (median(&mut ui), render_ms, shown.len() as f64 / total)
}

/// Median time of `n` live move steps rendered directly on the app's renderer (the worker's job:
/// moved copy + visible tiles).
fn render_steps(h: &H, n: usize) -> f64 {
    let st = &h.state().state;
    let d = st.session.active().expect("doc");
    let key = st
        .canvases
        .get(&d.id)
        .and_then(|c| c.shown)
        .expect("shown");
    let ids = d.selection.elements.clone();
    let mut v = Vec::new();
    for i in 1..=n {
        let t = Instant::now();
        let lm = fp_ui::canvas::ghost::LiveMove {
            ids: ids.clone(),
            dx: i as f32,
            dy: 0.0,
            copy: false,
        };
        let (doc, moved) = fp_ui::canvas::ghost::apply_live_move(&d.doc, d.page, &lm);
        let pr = key.page_rect;
        let img = st.renderer.with_moved_looks(&moved, |r| {
            r.render_device(&doc, d.page, d.frame, pr, (pr.left, pr.top), key.zoom, key.dev)
        });
        std::hint::black_box(img);
        v.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    median(&mut v)
}

#[test]
#[ignore = "measurement (release): cargo test -p fp-ui --release --test all drag_live_perf_f147::bench -- --ignored --nocapture"]
fn bench() {
    fp_engine::shell::disable();
    for sc in [Scene::Large, Scene::LargeBevel, Scene::LargeGlow, Scene::Cluster] {
        for zoom in [1.0, 8.0] {
            let (ui, render, sps) = measure(sc, zoom, 3.0);
            println!(
                "F147 {sc:?} {:>4} %: UI {ui:6.1} ms/frame, render {render:6.1} ms/step, {sps:5.1} live images/s",
                zoom * 100.0
            );
        }
    }
}

// ------------------------------------------------------------------ behaviour (not ignored)

fn canvas(h: &H) -> &fp_ui::canvas::render::CanvasCache {
    h.state().state.canvases.values().next().expect("canvas")
}

fn canvas_mut(h: &mut H) -> &mut fp_ui::canvas::render::CanvasCache {
    h.state_mut()
        .state
        .canvases
        .values_mut()
        .next()
        .expect("canvas")
}

fn move_to(h: &mut H, p: Pos2) {
    h.input_mut().events.push(Event::PointerMoved(p));
    h.step();
}

/// Small scene: page 600 × 400, a blue rectangle below, M (160,120)–(320,220) with Inner Bevel and
/// a drop shadow, a grey rectangle with Invert above; M selected.
fn small_scene() -> Session {
    let mut s = Session::default();
    ok(
        &mut s,
        "file.new",
        json!({"width": 600, "height": 400, "background": "#ffffff"}),
    );
    ok(
        &mut s,
        "draw.rectangle",
        json!({"rect": [60, 60, 260, 260], "fillColor": "#3366ff", "strokeWidth": 0}),
    );
    ok(
        &mut s,
        "draw.rectangle",
        json!({"rect": [160, 120, 320, 220], "fillColor": "#ffcc00", "strokeWidth": 0}),
    );
    let m = s.active().expect("doc").selection.elements[0].0;
    ok(&mut s, "element.addEffect", json!({"effect": "innerBevel"}));
    ok(&mut s, "element.addEffect", json!({"effect": "dropShadow"}));
    ok(
        &mut s,
        "draw.rectangle",
        json!({"rect": [280, 80, 400, 280], "fillColor": "#808080", "strokeWidth": 0}),
    );
    ok(&mut s, "element.setBlendMode", json!({"mode": "invert"}));
    ok(&mut s, "select.set", json!({"ids": [m]}));
    s
}

fn start_drag(h: &mut H) -> Pos2 {
    let _ = h.state_mut().state.session.set_tool(ToolId::Pointer);
    if let Some(d) = h.state_mut().state.session.active_mut() {
        d.view.snap_to_smart_guides = false;
    }
    h.run_steps(2);
    let a = at(h, 200.0, 170.0);
    move_to(h, a);
    button(h, a, true);
    move_to(h, a + egui::vec2(10.0, 0.0));
    a
}

/// The moved element's look comes from the cache across drag steps: no cache miss, no new
/// effect chain (Inner Bevel, drop shadow), and the held image still equals the released one.
#[test]
fn look_cache_hits_across_drag_steps() {
    let mut h = harness(small_scene(), true);
    canvas_mut(&mut h).live_budget_ms = Some(f64::INFINITY);
    let a = start_drag(&mut h);
    move_to(&mut h, a + egui::vec2(40.0, 0.0));
    h.run_steps(2);
    let stats = |h: &H| h.state().state.renderer.with_cache(|c| c.stats());
    let s1 = stats(&h);
    let k1 = canvas(&h).shown.expect("shown").preview;
    assert!(k1 != 0, "live move image");
    move_to(&mut h, a + egui::vec2(41.0, 0.0));
    h.run_steps(2);
    move_to(&mut h, a + egui::vec2(47.0, 3.0));
    h.run_steps(2);
    let s2 = stats(&h);
    let k2 = canvas(&h).shown.expect("shown").preview;
    assert!(k2 != 0 && k2 != k1, "a new step was rendered");
    assert_eq!(s2.misses, s1.misses, "no element look computed again");
    assert!(s2.moved_hits >= s1.moved_hits + 2, "the moved look came from its source");
    let held = canvas(&h).shown_image.clone().expect("image");
    button(&mut h, a + egui::vec2(47.0, 3.0), false);
    h.run_steps(4);
    let cc = canvas(&h);
    assert_eq!(cc.shown.map(|k| k.preview), Some(0), "committed render");
    assert_eq!(
        held.as_bytes(),
        cc.shown_image.as_ref().expect("image").as_bytes(),
        "held frame equals the frame after the release"
    );
}

/// With the render thread, a live move drag has at most one render on the way per pane: a new
/// offset is only sent once the previous step is back, and every step sent is shown.
#[test]
fn one_live_move_render_in_flight() {
    let mut h = harness(small_scene(), false);
    settle(&mut h);
    canvas_mut(&mut h).live_budget_ms = Some(f64::INFINITY);
    let a = start_drag(&mut h);
    let mut shown = std::collections::HashSet::new();
    let mut prev = (canvas(&h).requested, canvas(&h).shown);
    for i in 0..60 {
        move_to(&mut h, a + egui::vec2(12.0 + i as f32, 0.0));
        let cc = canvas(&h);
        let (req, sh) = (cc.requested, cc.shown);
        if prev.0 != prev.1 {
            // a job was on its way: no new one unless it came back first
            assert!(
                req == prev.0 || sh == prev.0,
                "frame {i}: a second job was sent while one was on its way"
            );
        }
        if let Some(k) = sh
            && k.preview != 0
        {
            shown.insert(k.preview);
        }
        prev = (req, sh);
        std::thread::sleep(Duration::from_millis(2));
    }
    // the newest offset is rendered once the pointer rests
    let last = a + egui::vec2(12.0 + 59.0, 0.0);
    settle(&mut h);
    move_to(&mut h, last);
    settle(&mut h);
    let cc = canvas(&h);
    assert!(cc.requested == cc.shown);
    assert!(cc.shown.is_some_and(|k| k.preview != 0));
    assert!(shown.len() >= 2, "steps are shown while dragging ({})", shown.len());
    button(&mut h, last, false);
}

/// Two steps in a row over the budget switch the drag to the sprite preview; the release ends it
/// and the next drag is live again.
#[test]
fn budget_fallback_to_sprites_and_back() {
    let mut h = harness(small_scene(), true);
    canvas_mut(&mut h).live_budget_ms = Some(0.0);
    let a = start_drag(&mut h);
    for dx in [40.0, 44.0, 48.0] {
        move_to(&mut h, a + egui::vec2(dx, 0.0));
        h.run_steps(2);
    }
    {
        let cc = canvas(&h);
        assert!(cc.live_fallback, "fell back after two slow steps");
        assert!(
            cc.ghosts.drag.iter().any(Option::is_some),
            "the sprite preview is drawn"
        );
        assert_eq!(cc.shown.map(|k| k.preview), Some(0), "no live move render");
    }
    button(&mut h, a + egui::vec2(48.0, 0.0), false);
    h.run_steps(4);
    assert!(!canvas(&h).live_fallback, "released: back to the live path");
    canvas_mut(&mut h).live_budget_ms = None;
    let b = at(&h, 250.0, 170.0);
    move_to(&mut h, b);
    button(&mut h, b, true);
    move_to(&mut h, b + egui::vec2(10.0, 0.0));
    move_to(&mut h, b + egui::vec2(30.0, 0.0));
    h.run_steps(2);
    let cc = canvas(&h);
    assert!(!cc.live_fallback);
    assert!(cc.shown.is_some_and(|k| k.preview != 0), "live again");
    button(&mut h, b + egui::vec2(30.0, 0.0), false);
}
