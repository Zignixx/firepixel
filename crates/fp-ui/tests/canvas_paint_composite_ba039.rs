//! BA-039: the live brush preview is composited in the bitmap's stacking position. The stroke on a
//! half-transparent bitmap shows at 50 % while painting, the eraser shows the object below the
//! bitmap (not a backdrop), and objects above still cover the stroke. Pixel checks in the rendered
//! window (wgpu); `canvas_paint_frame_time_ba039` measures frame times (`--ignored`, release).

use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use fp_engine::Session;
use fp_engine::tools::ToolId;
use fp_ui::FirepixelApp;
use fp_ui::app_state::UiSettings;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn harness(s: Session, size: (f32, f32)) -> H {
    let ctx = egui::Context::default();
    let mut a = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    a.state.os_clipboard = false;
    a.state.sync_render = true;
    let mut h = Harness::builder()
        .renderer(crate::gpu_common::renderer())
        .with_size(egui::vec2(size.0, size.1))
        .with_pixels_per_point(1.0)
        .with_max_steps(8)
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), a);
    h.run_steps(4);
    h
}

fn at(h: &H, x: f32, y: f32) -> Pos2 {
    let s = &h.state().state;
    let d = s.session.active().expect("document");
    let pr = s.canvases.get(&d.id).and_then(|c| c.page_screen).expect("page position");
    let r = fp_render::Renderer::page_rect(&d.doc, d.page).expect("page");
    let (w, ht) = ((r.right - r.left) as f32, (r.bottom - r.top) as f32);
    egui::pos2(pr.left() + (x - r.left as f32) / w * pr.width(), pr.top() + (y - r.top as f32) / ht * pr.height())
}

fn ev(h: &mut H, e: Event) {
    h.input_mut().events.push(e);
    h.step();
}

fn button(h: &mut H, p: Pos2, pressed: bool) {
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
}

fn set_tool(h: &mut H, t: ToolId) {
    let _ = h.state_mut().state.session.set_tool(t);
    h.step();
}

/// RGB at a document point in the rendered window; `None` without a graphics adapter.
fn rgb(h: &mut H, x: f32, y: f32) -> Option<[u8; 3]> {
    let p = at(h, x, y);
    let shot = match h.render() {
        Ok(i) => i,
        Err(e) => {
            eprintln!("no graphics adapter, pixel check skipped: {e}");
            return None;
        }
    };
    let px = shot.get_pixel(p.x as u32, p.y as u32).0;
    Some([px[0], px[1], px[2]])
}

fn near(a: [u8; 3], b: [u8; 3], tol: i32) -> bool {
    a.iter().zip(b).all(|(x, y)| (*x as i32 - y as i32).abs() <= tol)
}

/// White page; optional red rectangle below, a pasted bitmap of `fill` over 40..300 × 40..200 and a
/// green rectangle 160..260 above it. The bitmap is selected; foreground colour black.
fn page(fill: fp_color::Rgba8, red_below: bool, opacity: Option<u32>, size: (i32, i32)) -> Session {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": size.0, "height": size.1, "background": "#ffffff"})).expect("document");
    if red_below {
        s.execute("draw.rectangle", json!({"rect": [40, 40, 300, 200], "fillColor": "#ff0000", "strokeWidth": 0})).expect("red");
    }
    let img = fp_raster::RgbaImage::filled(fp_geom::IRect::new(40, 40, size.0.min(300), size.1.min(200)), fill).expect("image");
    s.clipboard.clear();
    s.clipboard.pixels = Some(img);
    let v = s.execute("edit.paste", json!({})).expect("bitmap");
    let bmp = v.value["ids"][0].as_u64().expect("id");
    s.execute("select.set", json!({"ids": [bmp]})).expect("select bitmap");
    if let Some(o) = opacity {
        s.execute("element.setOpacity", json!({"opacity": o})).expect("opacity");
    }
    s.execute("draw.rectangle", json!({"rect": [160, 50, 260, 150], "fillColor": "#00ff00", "strokeWidth": 0})).expect("green");
    s.execute("select.set", json!({"ids": [bmp]})).expect("select bitmap");
    s.set_colors(Some(fp_color::Argb(0xff00_0000)), None);
    s
}

/// Presses at x0, moves in 10 px steps to x1 on row `y` (still pressed).
fn stroke(h: &mut H, x0: f32, x1: f32, y: f32) -> Pos2 {
    let n = ((x1 - x0) / 10.0) as usize;
    let pts: Vec<Pos2> = (0..=n).map(|i| at(h, x0 + i as f32 * 10.0, y)).collect();
    ev(h, Event::PointerMoved(pts[0]));
    button(h, pts[0], true);
    for p in &pts[1..] {
        ev(h, Event::PointerMoved(*p));
    }
    h.step();
    *pts.last().expect("point")
}

#[test]
fn canvas_paint_preview_applies_bitmap_opacity_ba039() {
    let mut h = harness(page(fp_color::Rgba8::new(255, 255, 255, 255), false, Some(50), (400, 300)), (1400.0, 900.0));
    set_tool(&mut h, ToolId::Brush);
    let end = stroke(&mut h, 60.0, 260.0, 100.0);
    let doc = h.state().state.session.active().map(|d| d.id).expect("doc");
    assert!(fp_ui::canvas::paint_preview::cached_tiles(&h.ctx, doc) > 0, "composite tiles held");
    let Some(mid) = rgb(&mut h, 100.0, 100.0) else { return };
    assert!(near(mid, [128, 128, 128], 10), "stroke at 50 % opacity while painting, was {mid:?}");
    let under = rgb(&mut h, 210.0, 100.0).expect("image");
    assert!(near(under, [0, 255, 0], 2), "stroke hidden under the object above, was {under:?}");
    let off = rgb(&mut h, 100.0, 60.0).expect("image");
    assert!(near(off, [255, 255, 255], 2), "unpainted part unchanged, was {off:?}");
    button(&mut h, end, false);
    h.run_steps(3);
    let after = rgb(&mut h, 100.0, 100.0).expect("image");
    assert!(near(after, mid, 3), "preview equals the result: {after:?} vs {mid:?}");
}

#[test]
fn canvas_eraser_preview_shows_object_below_ba039() {
    let mut h = harness(page(fp_color::Rgba8::new(0, 0, 255, 255), true, None, (400, 300)), (1400.0, 900.0));
    set_tool(&mut h, ToolId::Eraser);
    let end = stroke(&mut h, 60.0, 140.0, 100.0);
    let Some(mid) = rgb(&mut h, 100.0, 100.0) else { return };
    assert!(near(mid, [255, 0, 0], 12), "erased pixels show the red rectangle below, was {mid:?}");
    let off = rgb(&mut h, 100.0, 60.0).expect("image");
    assert!(near(off, [0, 0, 255], 2), "unerased bitmap, was {off:?}");
    button(&mut h, end, false);
    h.run_steps(3);
    let after = rgb(&mut h, 100.0, 100.0).expect("image");
    assert!(near(after, mid, 3), "preview equals the result: {after:?} vs {mid:?}");
}

/// Frame time of a stroke on a 2000×1500 bitmap (UI thread, one frame per pointer move).
/// `cargo test --release -p fp-ui --test all canvas_paint_frame_time_ba039 -- --ignored --nocapture`
#[test]
#[ignore]
fn canvas_paint_frame_time_ba039() {
    let s = {
        let mut s = Session::default();
        s.execute("file.new", json!({"width": 2000, "height": 1500, "background": "#ffffff"})).expect("document");
        let img = fp_raster::RgbaImage::filled(fp_geom::IRect::new(0, 0, 2000, 1500), fp_color::Rgba8::new(200, 220, 255, 255)).expect("image");
        s.clipboard.clear();
        s.clipboard.pixels = Some(img);
        let v = s.execute("edit.paste", json!({})).expect("bitmap");
        let bmp = v.value["ids"][0].as_u64().expect("id");
        s.execute("draw.rectangle", json!({"rect": [800, 600, 1200, 900], "fillColor": "#00ff00", "strokeWidth": 0})).expect("green");
        s.execute("select.set", json!({"ids": [bmp]})).expect("select bitmap");
        s.set_colors(Some(fp_color::Argb(0xff00_0000)), None);
        s
    };
    let mut h = harness(s, (1600.0, 1000.0));
    set_tool(&mut h, ToolId::Brush);
    let pts: Vec<Pos2> = (0..=120).map(|i| at(&h, 200.0 + i as f32 * 13.0, 500.0 + (i as f32 * 0.21).sin() * 300.0)).collect();
    ev(&mut h, Event::PointerMoved(pts[0]));
    button(&mut h, pts[0], true);
    let mut times = Vec::new();
    for p in &pts[1..] {
        let t = std::time::Instant::now();
        ev(&mut h, Event::PointerMoved(*p));
        times.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    button(&mut h, *pts.last().expect("point"), false);
    times.sort_by(|a, b| a.total_cmp(b));
    let avg = times.iter().sum::<f64>() / times.len() as f64;
    println!(
        "BA-039 frame time over {} moves: avg {:.2} ms, median {:.2} ms, p90 {:.2} ms, max {:.2} ms",
        times.len(),
        avg,
        times[times.len() / 2],
        times[times.len() * 9 / 10],
        times[times.len() - 1]
    );
}
