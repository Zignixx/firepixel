//! F157 (FB-137, TW-292): cost of a live drag of floating pixels.
//!
//! Measurement (release, software rendering):
//! `cargo test -p fp-ui --release --test all drag_float_perf_f157::bench -- --ignored --nocapture`

use egui_kittest::Harness;
use fp_engine::Session;
use fp_ui::FirepixelApp;
use fp_ui::app_state::UiSettings;
use serde_json::json;
use std::time::{Duration, Instant};

type H = Harness<'static, FirepixelApp>;

fn harness(s: Session) -> H {
    let ctx = egui::Context::default();
    let mut a = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    a.state.os_clipboard = false;
    a.state.sync_render = true;
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

/// A photo-sized bitmap `w` x `h` with a gradient, a marquee 400 x 300 lifted into a floating
/// block (Ctrl drag of 10 px).
fn scene(w: i32, h: i32) -> Session {
    let mut s = Session::default();
    ok(
        &mut s,
        "file.new",
        json!({"width": w, "height": h, "background": "#ffffff"}),
    );
    let mut img = fp_raster::RgbaImage::new(fp_geom::IRect::from_xywh(0, 0, w, h)).expect("img");
    for y in 0..h {
        for x in 0..w {
            img.set_pixel(x, y, fp_color::Rgba8::new(x as u8, y as u8, (x ^ y) as u8, 255));
        }
    }
    s.clipboard.clear();
    s.clipboard.pixels = Some(img);
    ok(&mut s, "edit.paste", json!({}));
    ok(
        &mut s,
        "select.pixels",
        json!({"shape": "rect", "rect": [100, 100, 500, 400]}),
    );
    ok(&mut s, "select.floatPixels", json!({"dx": 10, "dy": 0}));
    s
}

fn median(v: &mut [f64]) -> f64 {
    v.sort_by(|a, b| a.total_cmp(b));
    v.get(v.len() / 2).copied().unwrap_or(0.0)
}

/// Median time of `n` live steps of the floating block as the render worker does them: the
/// render-only copy with the block moved, then the visible tiles at the view's zoom.
fn render_steps(h: &H, n: usize) -> (f64, f64) {
    let st = &h.state().state;
    let d = st.session.active().expect("doc");
    let key = st
        .canvases
        .get(&d.id)
        .and_then(|c| c.shown)
        .expect("shown");
    let id = d.selection.elements[0];
    let (mut prep, mut all) = (Vec::new(), Vec::new());
    for i in 1..=n {
        let t = Instant::now();
        let lp = fp_ui::canvas::ghost::LivePixels {
            id,
            dx: i as i32,
            dy: 0,
            copy: false,
        };
        let doc = fp_ui::canvas::ghost::apply_live_pixels(&d.doc, d.page, &lp, &d.selection);
        prep.push(t.elapsed().as_secs_f64() * 1000.0);
        let pr = key.page_rect;
        let img = st
            .renderer
            .render_device(&doc, d.page, d.frame, pr, (pr.left, pr.top), key.zoom, key.dev);
        std::hint::black_box(img);
        all.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    (median(&mut prep), median(&mut all))
}

#[test]
#[ignore = "measurement (release): cargo test -p fp-ui --release --test all drag_float_perf_f157::bench -- --ignored --nocapture"]
fn bench() {
    fp_engine::shell::disable();
    for (w, ht) in [(800, 600), (4000, 3000)] {
        for zoom in [1.0, 3.0] {
            let mut h = harness(scene(w, ht));
            let _ = h
                .state_mut()
                .state
                .session
                .execute("view.setZoom", json!({"zoom": zoom, "center": [300, 250]}));
            for _ in 0..50 {
                h.step();
                std::thread::sleep(Duration::from_millis(2));
            }
            let (prep, all) = render_steps(&h, 9);
            println!(
                "F157 {w}x{ht} {:>4} %: block move {prep:6.1} ms, with render {all:6.1} ms per step",
                zoom * 100.0
            );
        }
    }
}
