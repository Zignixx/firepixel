//! FB-97 / TW-272: split document view (Window ▸ Duplicate Window) in the whole main window:
//! two panes with different pages drawn side by side, a click gives a pane the focus, tools act in
//! the pane under the pointer, closing the pane. F137 / TW-275: up to four panes, the status bar
//! follows the pane under the pointer.

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

/// Canvas area of pane `p`.
fn pane_area(h: &Harness<'static, FirepixelApp>, p: usize) -> Rect {
    let st = &h.state().state;
    let d = st.session.active().expect("doc");
    let key = d.pane_key(p);
    let cc = if st.cache_pane(d.id) == key {
        st.canvases.get(&d.id)
    } else {
        st.pane_canvases.get(&(d.id, key))
    };
    cc.and_then(|c| c.area).expect("pane area")
}

/// Canvas areas of the left and right pane.
fn areas(h: &Harness<'static, FirepixelApp>) -> (Rect, Rect) {
    (pane_area(h, 0), pane_area(h, 1))
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
    assert_eq!(
        d.page, 0,
        "F137: the focused pane closed, the left one on page 0 stays"
    );
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

fn hover(h: &mut Harness<'static, FirepixelApp>, p: Pos2) {
    h.input_mut().events.push(Event::PointerMoved(p));
    h.run_steps(3);
}

/// The parked (passive) pane's canvas cache (two panes).
fn passive_cache<'a>(h: &'a Harness<'static, FirepixelApp>) -> &'a super::render::CanvasCache {
    let st = &h.state().state;
    let d = st.session.active().expect("doc");
    let key = d.pane_key(1 - d.live_pane());
    st.pane_canvases.get(&(d.id, key)).expect("passive cache")
}

/// F132: the pane without the focus shows the hover feedback of the active tool (coordinate tag,
/// tool cursor, hover highlight, brush tip) while the pointer is over it; the focus stays.
#[test]
fn hover_in_the_passive_pane_shows_feedback_without_focus_change() {
    let mut h = harness(false);
    exec(&mut h, "window.duplicateWindow", json!({}));
    exec(&mut h, "page.setCurrent", json!({"index": 1}));
    assert_eq!(live(&h), (1, 1));
    let (l, r) = areas(&h);
    // Rectangle: coordinate tag and the tool cursor follow the pointer into the left pane
    let _ = h.state_mut().state.session.set_tool(ToolId::Rectangle);
    hover(&mut h, l.center());
    assert_eq!(live(&h), (1, 1), "hovering does not move the focus");
    let tag = passive_cache(&h).pointer_tag.clone();
    assert!(tag.as_deref().is_some_and(|t| t.contains("x")), "{tag:?}");
    assert_eq!(h.state().state.painted_cursor, Some(l.center()));
    // Pointer: hover highlight of the red bitmap on the left pane's page 0
    let _ = h.state_mut().state.session.set_tool(ToolId::Pointer);
    hover(&mut h, l.center() + egui::vec2(5.0, 5.0));
    assert!(
        passive_cache(&h)
            .feedback
            .iter()
            .any(|o| matches!(o, fp_engine::Overlay::Highlight { .. })),
        "hover highlight in the passive pane"
    );
    // Brush: the brush tip outline
    let _ = h.state_mut().state.session.set_tool(ToolId::Brush);
    hover(&mut h, l.center());
    assert!(
        passive_cache(&h)
            .feedback
            .iter()
            .any(|o| matches!(o, fp_engine::Overlay::BrushTip { .. })),
        "brush tip in the passive pane"
    );
    assert_eq!(live(&h), (1, 1));
    // back over the focused pane: the passive pane drops its hover feedback
    let _ = h.state_mut().state.session.set_tool(ToolId::Rectangle);
    hover(&mut h, r.center());
    assert_eq!(passive_cache(&h).pointer_tag, None);
    assert!(
        !passive_cache(&h)
            .feedback
            .iter()
            .any(|o| matches!(o, fp_engine::Overlay::BrushTip { .. }))
    );
    let st = &h.state().state;
    let id = st.session.active().expect("doc").id;
    assert!(
        st.canvases
            .get(&id)
            .is_some_and(|c| c.pointer_tag.is_some())
    );
}

/// F132: Window ▸ Split ▸ Top and Bottom puts the panes one above the other; both keep their
/// page and zoom; the splitter position is remembered for the document.
#[test]
fn orientation_switch_keeps_pages_and_the_splitter_is_remembered() {
    let mut h = harness(false);
    exec(&mut h, "window.duplicateWindow", json!({}));
    exec(&mut h, "page.setCurrent", json!({"index": 1}));
    exec(&mut h, "view.zoomIn", json!({}));
    let state = |h: &Harness<'static, FirepixelApp>| {
        let d = h.state().state.session.active().expect("doc");
        let o = d.other_pane().expect("other");
        (d.live_pane(), d.page, d.view.zoom, o.page, o.view.zoom)
    };
    let before = state(&h);
    exec(
        &mut h,
        "window.splitOrientation",
        json!({"orientation": "horizontal"}),
    );
    h.run_steps(3);
    let (t, b) = areas(&h);
    assert!(t.bottom() <= b.top(), "one above the other: {t:?} {b:?}");
    assert!(t.width() > 1000.0 && b.width() > 1000.0, "{t:?} {b:?}");
    assert_eq!(state(&h), before, "pages and zoom stay");
    // drag the splitter 100 px down: the top pane grows, the ratio stays with the document
    let s = egui::pos2(t.center().x, t.bottom() + super::split::SPLITTER_W / 2.0);
    let ratio0 = h
        .state()
        .state
        .session
        .active()
        .map(|d| d.split_layout.ratio);
    for e in [
        Event::PointerMoved(s),
        Event::PointerButton {
            pos: s,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        },
        Event::PointerMoved(s + egui::vec2(0.0, 50.0)),
        Event::PointerMoved(s + egui::vec2(0.0, 100.0)),
        Event::PointerButton {
            pos: s + egui::vec2(0.0, 100.0),
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        },
    ] {
        h.input_mut().events.push(e);
        h.step();
    }
    h.run_steps(2);
    let ratio1 = h
        .state()
        .state
        .session
        .active()
        .map(|d| d.split_layout.ratio);
    assert!(ratio1 > ratio0, "{ratio0:?} -> {ratio1:?}");
    assert_eq!(state(&h), before, "the splitter drag moves no focus");
    exec(&mut h, "window.closeDuplicate", json!({}));
    exec(&mut h, "window.duplicateWindow", json!({}));
    let d = h.state().state.session.active().expect("doc");
    assert_eq!(Some(d.split_layout.ratio), ratio1);
    assert_eq!(
        d.split_layout.orientation,
        fp_engine::view_pane::SplitOrientation::Horizontal
    );
    // back to side by side: pages stay as well
    exec(
        &mut h,
        "window.splitOrientation",
        json!({"orientation": "vertical"}),
    );
    let (l, r) = areas(&h);
    assert!(l.right() <= r.left(), "{l:?} {r:?}");
}

/// F132: Shift+F6 (Window ▸ Next Pane) moves the focus between the panes, only while split;
/// Ctrl+Tab keeps cycling the documents.
#[test]
fn shift_f6_moves_the_focus_to_the_next_pane() {
    let mut h = harness(false);
    let shift = Modifiers {
        shift: true,
        ..Default::default()
    };
    h.key_press_modifiers(shift, egui::Key::F6);
    h.run_steps(2);
    assert_eq!(live(&h), (0, 0), "no split: nothing happens");
    exec(&mut h, "window.duplicateWindow", json!({}));
    exec(&mut h, "page.setCurrent", json!({"index": 1}));
    assert_eq!(live(&h), (1, 1));
    h.key_press_modifiers(shift, egui::Key::F6);
    h.run_steps(2);
    assert_eq!(live(&h), (0, 0));
    h.key_press_modifiers(shift, egui::Key::F6);
    h.run_steps(2);
    assert_eq!(live(&h), (1, 1));
}

/// F137 / TW-275: Duplicate Window on a split view splits the focused pane again, up to four
/// panes in a row, each with its own page; Shift+F6 cycles all of them; the × of a pane header
/// closes that pane and the focus stays.
#[test]
fn four_panes_in_a_row_cycle_and_close() {
    let mut h = harness(true);
    exec(&mut h, "window.duplicateWindow", json!({}));
    exec(&mut h, "window.duplicateWindow", json!({}));
    exec(&mut h, "window.duplicateWindow", json!({}));
    h.run_steps(3);
    let d = h.state().state.session.active().expect("doc");
    assert_eq!((d.pane_count(), d.live_pane()), (4, 3));
    // pane 3 shows page 1, the others page 0
    exec(&mut h, "page.setCurrent", json!({"index": 1}));
    h.run_steps(4);
    let r: Vec<Rect> = (0..4).map(|p| pane_area(&h, p)).collect();
    for w in r.windows(2) {
        assert!(w[0].right() <= w[1].left(), "in a row: {r:?}");
    }
    // each split halves the focused pane: 1/2, 1/4, 1/8, 1/8 of the area
    assert!(r.iter().all(|a| a.width() > 100.0), "{r:?}");
    assert!(r[0].width() > 3.0 * r[3].width(), "{r:?}");
    if let Ok(shot) = h.render() {
        let px = |p: Pos2| {
            let v = shot.get_pixel(p.x as u32, p.y as u32).0;
            (v[0], v[1], v[2])
        };
        assert_eq!(px(r[0].center()), (255, 0, 0), "pane 0 on page 0");
        assert_eq!(px(r[2].center()), (255, 0, 0), "pane 2 on page 0");
        assert_eq!(px(r[3].center()), (0, 0, 255), "pane 3 on page 1");
    } else {
        eprintln!("no graphics adapter, pixel check skipped");
    }
    // Shift+F6 cycles 3 -> 0 -> 1 -> 2 -> 3
    let shift = Modifiers {
        shift: true,
        ..Default::default()
    };
    let mut seen = Vec::new();
    for _ in 0..4 {
        h.key_press_modifiers(shift, egui::Key::F6);
        h.run_steps(2);
        seen.push(live(&h).0);
    }
    assert_eq!(seen, vec![0, 1, 2, 3]);
    // the × of pane 1 (top right of its header): pane 1 closes, pane 3 keeps the focus (now 2)
    let a1 = pane_area(&h, 1);
    let x = egui::pos2(
        a1.right() - 11.0,
        a1.top() - super::split::HEADER_H / 2.0 + 1.0,
    );
    click(&mut h, x);
    h.run_steps(3);
    let d = h.state().state.session.active().expect("doc");
    assert_eq!((d.pane_count(), d.live_pane(), d.page), (3, 2, 1));
    // Close Duplicate Window closes the focused pane: the focus goes to the pane before it
    exec(&mut h, "window.closeDuplicate", json!({}));
    let d = h.state().state.session.active().expect("doc");
    assert_eq!((d.pane_count(), d.live_pane(), d.page), (2, 1, 0));
    h.run_steps(3);
    let (l, rr) = areas(&h);
    assert!(l.right() <= rr.left() && rr.width() > 200.0, "{l:?} {rr:?}");
}

/// F137 / TW-275: the status bar (size, zoom, state) and the pointer position follow the pane
/// under the pointer; outside every pane they show the focused pane.
#[test]
fn status_bar_follows_the_pane_under_the_pointer() {
    let mut h = harness(false);
    exec(&mut h, "window.duplicateWindow", json!({}));
    exec(&mut h, "page.setCurrent", json!({"index": 1}));
    exec(&mut h, "view.zoomIn", json!({}));
    h.run_steps(3);
    let (l, r) = areas(&h);
    let (live_zoom, left_zoom) = {
        let d = h.state().state.session.active().expect("doc");
        (
            d.view.zoom,
            d.other_pane().map(|p| p.view.zoom).expect("left"),
        )
    };
    assert!(live_zoom > left_zoom);
    let shown = |h: &Harness<'static, FirepixelApp>| {
        let st = &h.state().state;
        (st.hover_pane, super::status_view(st))
    };
    // over the left pane (no focus): its page and zoom
    let _ = h.state_mut().state.session.set_tool(ToolId::Rectangle);
    hover(&mut h, l.center());
    assert_eq!(live(&h), (1, 1), "hovering does not move the focus");
    assert_eq!(shown(&h), (Some(0), Some((0, 0, left_zoom))));
    let pd = h.state().state.pointer_doc.expect("pointer position");
    assert_eq!(h.state().state.pointer_pane, 0);
    let left_doc = {
        let d = h.state().state.session.active().expect("doc");
        let v = &d.other_pane().expect("left").view;
        v.screen_to_doc(
            (l.center().x - l.left()) as f64,
            (l.center().y - l.top()) as f64,
        )
    };
    assert!(
        (pd.x - left_doc.x).abs() < 1.0 && (pd.y - left_doc.y).abs() < 1.0,
        "{pd:?} vs {left_doc:?}"
    );
    // over the focused pane: its own
    hover(&mut h, r.center());
    assert_eq!(shown(&h), (Some(1), Some((1, 0, live_zoom))));
    assert_eq!(h.state().state.pointer_pane, 1);
    // outside every pane (status bar): the focused pane
    hover(&mut h, egui::pos2(r.center().x, 945.0));
    assert_eq!(shown(&h), (None, Some((1, 0, live_zoom))));
}
