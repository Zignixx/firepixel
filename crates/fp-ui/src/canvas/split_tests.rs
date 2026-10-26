//! FB-97 / TW-272: split document view (Window ▸ Duplicate Window) in the whole main window:
//! two panes with different pages drawn side by side, a click gives a pane the focus, tools act in
//! the pane under the pointer, closing the pane.

use crate::FirepixelApp;
use crate::app_state::UiSettings;
use egui::{Event, Modifiers, PointerButton, Pos2, Rect};
use egui_kittest::Harness;
use fp_engine::ToolId;
use serde_json::json;

/// A 400 × 300 document: page 0 covered red, page 1 covered blue (pasted bitmaps); page 0 current.
fn two_page_session() -> fp_engine::Session {
    let mut s = fp_engine::Session::default();
    let _ = s.execute(
        "file.new",
        json!({"width": 400, "height": 300, "background": "#ffffff"}),
    );
    let _ = s.execute("page.add", json!({}));
    for (page, rgb) in [(0usize, (255u8, 0u8, 0u8)), (1, (0, 0, 255))] {
        let _ = s.execute("page.setCurrent", json!({"index": page}));
        let img = fp_raster::RgbaImage::filled(
            fp_geom::IRect::from_xywh(0, 0, 400, 300),
            fp_color::Rgba8::new(rgb.0, rgb.1, rgb.2, 255),
        )
        .expect("image");
        s.clipboard.clear();
        s.clipboard.pixels = Some(img);
        let _ = s.execute("edit.paste", json!({}));
        let _ = s.execute("select.none", json!({}));
    }
    let _ = s.execute("page.setCurrent", json!({"index": 0}));
    s
}

fn harness(wgpu: bool) -> Harness<'static, FirepixelApp> {
    let ctx = egui::Context::default();
    let mut app =
        FirepixelApp::with_session(&ctx, two_page_session(), UiSettings::default(), false);
    app.state.os_clipboard = false;
    app.state.sync_render = true;
    let b = Harness::builder()
        .with_size(egui::vec2(1500.0, 950.0))
        .with_pixels_per_point(1.0)
        .with_max_steps(8)
        .with_step_dt(1.0 / 60.0);
    let b = if wgpu { b.wgpu() } else { b };
    let mut h = b.build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    h
}

fn exec(h: &mut Harness<'static, FirepixelApp>, name: &str, p: serde_json::Value) {
    let r = h.state_mut().state.session.execute(name, p);
    assert!(r.is_ok(), "{name}: {r:?}");
    h.run_steps(3);
}

/// Canvas areas of the left and right pane.
fn areas(h: &Harness<'static, FirepixelApp>) -> (Rect, Rect) {
    let st = &h.state().state;
    let d = st.session.active().expect("doc");
    let live = st.canvases.get(&d.id).and_then(|c| c.area).expect("live");
    let other = st
        .pane_canvases
        .get(&d.id)
        .and_then(|c| c.area)
        .expect("other pane");
    if d.live_pane() == 0 {
        (live, other)
    } else {
        (other, live)
    }
}

fn click(h: &mut Harness<'static, FirepixelApp>, p: Pos2) {
    for e in [
        Event::PointerMoved(p),
        Event::PointerButton {
            pos: p,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        },
        Event::PointerButton {
            pos: p,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        },
    ] {
        h.input_mut().events.push(e);
        h.step();
    }
    h.run_steps(2);
}

fn live(h: &Harness<'static, FirepixelApp>) -> (usize, usize) {
    let d = h.state().state.session.active().expect("doc");
    (d.live_pane(), d.page)
}

#[test]
fn duplicate_window_draws_two_panes_with_their_own_pages() {
    let mut h = harness(true);
    exec(&mut h, "window.duplicateWindow", json!({}));
    // the right pane has the focus: it switches to page 1, the left pane stays on page 0
    exec(&mut h, "page.setCurrent", json!({"index": 1}));
    h.run_steps(4);
    let (l, r) = areas(&h);
    assert!(l.right() <= r.left(), "side by side: {l:?} {r:?}");
    assert!(l.width() > 300.0 && r.width() > 300.0, "{l:?} {r:?}");
    let shot = match h.render() {
        Ok(i) => i,
        Err(e) => {
            eprintln!("no graphics adapter, pixel check skipped: {e}");
            return;
        }
    };
    let px = |p: Pos2| {
        let v = shot.get_pixel(p.x as u32, p.y as u32).0;
        (v[0], v[1], v[2])
    };
    assert_eq!(px(l.center()), (255, 0, 0), "left pane shows page 0");
    assert_eq!(px(r.center()), (0, 0, 255), "right pane shows page 1");
    // the right pane zooms; the left keeps its zoom
    let left_zoom = h
        .state()
        .state
        .session
        .active()
        .and_then(|d| d.other_pane())
        .map(|p| p.view.zoom);
    exec(&mut h, "view.zoomIn", json!({}));
    let d = h.state().state.session.active().expect("doc");
    assert!(d.view.zoom > 1.0);
    assert_eq!(d.other_pane().map(|p| p.view.zoom), left_zoom);
}

#[test]
fn a_click_focuses_the_pane_and_tools_act_in_it() {
    let mut h = harness(false);
    exec(&mut h, "window.duplicateWindow", json!({}));
    exec(&mut h, "page.setCurrent", json!({"index": 1}));
    assert_eq!(live(&h), (1, 1));
    // a click into the left pane gives it the focus: the Pages panel and the status bar now
    // follow page 0
    let (l, _) = areas(&h);
    click(&mut h, l.center());
    assert_eq!(live(&h), (0, 0));
    // a rectangle drawn in the right pane lands on page 1, the right pane's page
    let _ = h.state_mut().state.session.set_tool(ToolId::Rectangle);
    h.step();
    let (_, r) = areas(&h);
    let (a, b) = (
        r.center() - egui::vec2(40.0, 30.0),
        r.center() + egui::vec2(40.0, 30.0),
    );
    let count = |h: &Harness<'static, FirepixelApp>, page: usize| {
        let d = h.state().state.session.active().expect("doc");
        let mut n = 0;
        d.doc.walk_elements(|_, path, _| {
            if path.root == fp_model::RootRef::Page(page) {
                n += 1;
            }
        });
        n
    };
    let (n0, n1) = (count(&h, 0), count(&h, 1));
    for e in [
        Event::PointerMoved(a),
        Event::PointerButton {
            pos: a,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        },
        Event::PointerMoved(a + (b - a) * 0.5),
        Event::PointerMoved(b),
        Event::PointerButton {
            pos: b,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        },
    ] {
        h.input_mut().events.push(e);
        h.step();
    }
    h.run_steps(2);
    assert_eq!(live(&h), (1, 1), "the press focused the right pane");
    assert_eq!((count(&h, 0), count(&h, 1)), (n0, n1 + 1));
    // the edit shows in the left pane as well (same document): its page 0 is unchanged, page 1
    // has the rectangle whichever pane looks at it
    let left = areas(&h).0.center();
    click(&mut h, left);
    assert_eq!(live(&h), (0, 0));
    assert_eq!(count(&h, 1), n1 + 1);
}

#[test]
fn closing_the_pane_returns_to_one_view() {
    let mut h = harness(false);
    exec(&mut h, "window.duplicateWindow", json!({}));
    exec(&mut h, "page.setCurrent", json!({"index": 1}));
    assert!(!h.state().state.pane_canvases.is_empty());
    exec(&mut h, "window.closeDuplicate", json!({}));
    h.run_steps(2);
    let st = &h.state().state;
    let d = st.session.active().expect("doc");
    assert!(!d.is_split());
    assert_eq!(d.page, 1, "the focused pane stays");
    assert!(st.pane_canvases.is_empty());
    let area = st.canvases.get(&d.id).and_then(|c| c.area).expect("canvas");
    assert!(
        area.width() > 1000.0,
        "one pane fills the area again: {area:?}"
    );
}

#[test]
fn pane_rects_split_at_the_ratio() {
    let r = Rect::from_min_size(Pos2::ZERO, egui::vec2(1005.0, 600.0));
    let (l, s, rr) = super::split::pane_rects(r, 0.5);
    assert_eq!((l.width(), s.width(), rr.width()), (500.0, 5.0, 500.0));
    let (l, _, _) = super::split::pane_rects(r, 0.0);
    assert_eq!(l.width(), 100.0, "clamped to 10 %");
}
