//! FB-148 (Firepixel extensions to the pausable bitmap stroke) in the user interface: the Shift
//! straight-line preview of the Brush shows the real brush width in the rendered window, also
//! where the line leaves the bitmap it paints into.

use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use fp_engine::Session;
use fp_engine::tools::ToolId;
use fp_ui::FirepixelApp;
use fp_ui::app_state::UiSettings;
use serde_json::{Value, json};

type H = Harness<'static, FirepixelApp>;

fn harness(s: Session) -> H {
    let ctx = egui::Context::default();
    let mut a = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    a.state.os_clipboard = false;
    a.state.sync_render = true;
    let mut h = Harness::builder()
        .renderer(crate::gpu_common::renderer())
        .with_size(egui::vec2(1400.0, 900.0))
        .with_pixels_per_point(1.0)
        .with_max_steps(8)
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), a);
    h.run_steps(4);
    h
}

/// White page 500 × 400 with a selected black bitmap over 60..200 × 60..120; Brush 14 px, hard.
fn page() -> Session {
    let mut s = Session::default();
    s.execute(
        "file.new",
        json!({"width": 500, "height": 400, "background": "#ffffff"}),
    )
    .expect("document");
    let img = fp_raster::RgbaImage::filled(
        fp_geom::IRect::new(60, 60, 200, 120),
        fp_color::Rgba8::new(0, 0, 0, 255),
    )
    .expect("image");
    s.clipboard.clear();
    s.clipboard.pixels = Some(img);
    let v = s.execute("edit.paste", json!({})).expect("bitmap");
    let bmp = v.value["ids"][0].as_u64().expect("id");
    s.execute("select.set", json!({"ids": [bmp]}))
        .expect("select bitmap");
    s.set_colors(Some(fp_color::Argb(0xff00_0000)), None);
    s.set_tool(ToolId::Brush).expect("tool");
    let stroke: Value = json!({"preset": "Einfach/Präzise abger.", "width": 14, "softness": 0});
    s.tools.options.brush_size = 14.0;
    s.tools.options.new_stroke = Some(stroke);
    s.tools.options.new_brush = None;
    s.tools.options.opacity = 100;
    s
}

fn at(h: &H, x: f32, y: f32) -> Pos2 {
    let s = &h.state().state;
    let d = s.session.active().expect("document");
    let pr = s
        .canvases
        .get(&d.id)
        .and_then(|c| c.page_screen)
        .expect("page position");
    let r = fp_render::Renderer::page_rect(&d.doc, d.page).expect("page");
    let (w, ht) = ((r.right - r.left) as f32, (r.bottom - r.top) as f32);
    egui::pos2(
        pr.left() + (x - r.left as f32) / w * pr.width(),
        pr.top() + (y - r.top as f32) / ht * pr.height(),
    )
}

fn ev(h: &mut H, e: Event) {
    h.input_mut().events.push(e);
    h.step();
}

fn button(h: &mut H, p: Pos2, pressed: bool, m: Modifiers) {
    ev(
        h,
        Event::PointerButton {
            pos: p,
            button: PointerButton::Primary,
            pressed,
            modifiers: m,
        },
    );
}

/// Dark pixels (screen) in the column through the document point `x` between the rows `y0` and
/// `y1`, in document pixels; `None` without a graphics adapter.
fn dark_rows(h: &mut H, x: f32, y0: f32, y1: f32) -> Option<usize> {
    let shot = match h.render() {
        Ok(i) => i,
        Err(e) => {
            eprintln!("no graphics adapter, pixel check skipped: {e}");
            return None;
        }
    };
    let (a, b) = (at(h, x, y0), at(h, x, y1));
    let scale = (b.y - a.y) / (y1 - y0);
    let n = (a.y as u32..b.y as u32)
        .filter(|y| shot.get_pixel(a.x as u32, *y).0[0] < 100)
        .count();
    Some((n as f32 / scale).round() as usize)
}

#[test]
fn d_shift_line_preview_has_the_brush_width_outside_the_bitmap() {
    let mut h = harness(page());
    let p0 = at(&h, 100.0, 90.0);
    ev(&mut h, Event::PointerMoved(p0));
    button(&mut h, p0, true, Modifiers::NONE);
    ev(&mut h, Event::ModifiersChanged(Modifiers::SHIFT));
    // a 45 degree line down to the right, far out of the bitmap, in a few moves
    let mut last = p0;
    for i in 1..=8 {
        let d = i as f32 * 30.0;
        last = at(&h, 100.0 + d, 90.0 + d);
        ev(&mut h, Event::PointerMoved(last));
    }
    h.run_steps(2);
    // column x 250: the line centre is at y 240; a 14 px line at 45 degrees is about 19 px high
    let Some(live) = dark_rows(&mut h, 250.0, 200.0, 280.0) else {
        return;
    };
    button(&mut h, last, false, Modifiers::SHIFT);
    ev(&mut h, Event::ModifiersChanged(Modifiers::NONE));
    h.run_steps(4);
    let done = dark_rows(&mut h, 250.0, 200.0, 280.0).expect("image");
    eprintln!("live {live} done {done}");
    assert!((18..=21).contains(&done), "the placed line: {done}");
    assert!(
        live.abs_diff(done) <= 1,
        "the Shift line preview has the width of the placed line: {live} vs {done}"
    );
}

fn key_event(h: &mut H, k: egui::Key, pressed: bool) {
    ev(
        h,
        Event::Key {
            key: k,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: Modifiers::NONE,
        },
    );
}

fn pause(h: &H) -> fp_engine::StrokePause {
    h.state().state.session.tool_stroke_pause()
}

#[test]
fn b_pause_hold_and_toggle_preferences() {
    use fp_engine::StrokePause as P;
    for hold in [false, true] {
        let mut s = page();
        s.prefs.set(
            fp_engine::tools::STROKE_PAUSE_MODE_PREF,
            fp_engine::PrefValue::Int(if hold { 1 } else { 2 }),
        );
        let mut h = harness(s);
        let p0 = at(&h, 250.0, 200.0);
        ev(&mut h, Event::PointerMoved(p0));
        button(&mut h, p0, true, Modifiers::NONE);
        let q = at(&h, 300.0, 200.0);
        ev(&mut h, Event::PointerMoved(q));
        assert_eq!(pause(&h), P::Painting);
        let hint = fp_ui::canvas::footer_status(&h.state().state, 0.0, false);
        let want = if hold { "ui-stroke-pause-hint-hold" } else { "ui-stroke-pause-hint" };
        assert_eq!(hint, fp_i18n::t(want), "hold {hold}");
        key_event(&mut h, egui::Key::P, true);
        assert_eq!(pause(&h), P::Paused, "P pauses (hold {hold})");
        key_event(&mut h, egui::Key::P, false);
        if hold {
            assert_eq!(pause(&h), P::Painting, "releasing P paints on");
        } else {
            assert_eq!(pause(&h), P::Paused, "toggle: stays paused after the release");
            key_event(&mut h, egui::Key::P, true);
            key_event(&mut h, egui::Key::P, false);
            assert_eq!(pause(&h), P::Painting, "toggle: P again paints on");
        }
        let p1 = at(&h, 350.0, 200.0);
        ev(&mut h, Event::PointerMoved(p1));
        button(&mut h, p1, false, Modifiers::NONE);
        assert_eq!(pause(&h), P::Idle);
    }
}

#[test]
fn a_burn_alt_shows_the_restore_hint() {
    let mut s = page();
    s.set_tool(ToolId::Burn).expect("tool");
    let mut h = harness(s);
    let p0 = at(&h, 80.0, 90.0);
    ev(&mut h, Event::PointerMoved(p0));
    button(&mut h, p0, true, Modifiers::NONE);
    let q = at(&h, 150.0, 90.0);
    ev(&mut h, Event::PointerMoved(q));
    let st = &h.state().state;
    assert_eq!(
        fp_ui::canvas::footer_status(st, 0.0, true),
        fp_i18n::t("ui-stroke-restore")
    );
    let p1 = at(&h, 150.0, 90.0);
    button(&mut h, p1, false, Modifiers::NONE);
}
