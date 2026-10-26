//! FB-02 (re 52 §2.3): with Select Behind the red hover frame shows the object a click would select,
//! i.e. the object behind the selected one, together with the selection; a click selects it. The
//! plain Pointer keeps its hover rule (no frame while over the selected topmost object).

use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use fp_engine::Overlay;
use fp_geom::Rect;
use fp_model::ElementId;
use fp_ui::FirepixelApp;
use fp_ui::app_state::UiSettings;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn harness(s: fp_engine::Session) -> H {
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().renderer(crate::gpu_common::renderer()).with_size(egui::vec2(1200.0, 800.0)).with_pixels_per_point(1.0).with_max_steps(12).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    h
}

fn at(h: &H, x: f32, y: f32) -> Pos2 {
    let st = &h.state().state;
    let d = st.session.active().expect("document");
    let area = st.canvases.get(&d.id).and_then(|c| c.area).expect("canvas");
    let (sx, sy) = d.view.doc_to_screen(fp_geom::Point::new(x, y));
    Pos2::new(area.min.x + sx as f32, area.min.y + sy as f32)
}

fn send(h: &mut H, e: Event) {
    h.input_mut().events.push(e);
    h.step();
}

fn hover(h: &mut H, x: f32, y: f32) {
    let p = at(h, x, y);
    send(h, Event::PointerMoved(p));
    h.run_steps(2);
}

fn click(h: &mut H, x: f32, y: f32) {
    let p = at(h, x, y);
    send(h, Event::PointerMoved(p));
    for pressed in [true, false] {
        send(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
    }
}

/// Red hover frames on the canvas: (bounds rectangle, contours).
fn frames(h: &H) -> Vec<(Option<Rect>, Vec<fp_model::Contour>)> {
    let st = &h.state().state;
    let id = st.session.active_id().expect("document");
    st.canvases
        .get(&id)
        .map(|c| {
            c.feedback
                .iter()
                .filter_map(|o| match o {
                    Overlay::Highlight { rect, contours } => Some((*rect, contours.clone())),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Bounds rectangles of the red hover frames.
fn highlights(h: &H) -> Vec<Rect> {
    frames(h).into_iter().filter_map(|f| f.0).collect()
}

fn selection(h: &H) -> Vec<ElementId> {
    h.state().state.session.active().map(|d| d.selection.elements.clone()).unwrap_or_default()
}

fn bounds(h: &H, id: ElementId) -> Rect {
    let d = h.state().state.session.active().expect("document");
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, id).expect("element");
    fp_engine::docops::element_frame_bounds(&d.doc, &e).expect("bounds")
}

fn near(a: [u8; 3], b: [u8; 3]) -> bool {
    a.iter().zip(b).all(|(x, y)| (*x as i32 - y as i32).abs() <= 8)
}

fn is_reddish(a: [u8; 3]) -> bool {
    a[0] > 150 && a[1] < 200 && (a[0] as i32 - a[1] as i32) > 60
}

/// Rendered pixels around the hovered shape's top edge (doc y = 60, at x = 140) and left edge
/// (doc x = 80, at y = 110): row above, row on, row below, column left of, column on. `None`
/// without a graphics adapter.
fn edge_pixels(h: &mut H) -> Option<[[u8; 3]; 5]> {
    let top = at(h, 140.0, 60.0);
    let left = at(h, 80.0, 110.0);
    let shot = match h.render() {
        Ok(i) => i,
        Err(e) => {
            eprintln!("no graphics adapter, pixel check skipped: {e}");
            return None;
        }
    };
    let px = |x: f32, y: f32| {
        let p = shot.get_pixel(x.round() as u32, y.round() as u32).0;
        [p[0], p[1], p[2]]
    };
    Some([
        px(top.x, top.y - 1.0),
        px(top.x, top.y),
        px(top.x, top.y + 1.0),
        px(left.x - 1.0, left.y),
        px(left.x, left.y),
    ])
}

#[test]
fn select_behind_hover_frames_the_object_behind_the_selection() {
    fp_engine::shell::disable();
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 300, "height": 240, "background": "#ffffff"})).expect("doc");
    // bitmap behind, vector shape in front
    let img = fp_raster::RgbaImage::filled(fp_geom::IRect::from_xywh(40, 40, 120, 100), fp_color::Rgba8::new(40, 160, 40, 255)).expect("image");
    s.clipboard.clear();
    s.clipboard.pixels = Some(img);
    let v = s.execute("edit.paste", json!({})).expect("paste");
    let bmp = ElementId(v.value["ids"][0].as_u64().expect("bitmap id"));
    let v = s.execute("draw.rectangle", json!({"rect": [80, 60, 200, 160]})).expect("rect");
    let shape = ElementId(v.value["id"].as_u64().expect("shape id"));
    s.execute("select.set", json!({"ids": [shape.0]})).expect("select");
    s.execute("tool.select", json!({"tool": "tool.selectBehind"})).expect("tool");
    let mut h = harness(s);
    assert_eq!(selection(&h), vec![shape]);

    hover(&mut h, 120.0, 100.0);
    let hl = highlights(&h);
    assert_eq!(hl, vec![bounds(&h, bmp)], "hover frame = bounds of the bitmap behind the selected shape");
    assert_eq!(selection(&h), vec![shape], "the shape stays selected (blue frame) while hovering");

    click(&mut h, 120.0, 100.0);
    assert_eq!(selection(&h), vec![bmp], "the click selects the framed object");
    // TW-157: right after the click, with the mouse still, no frame until the pointer moves
    h.run_steps(2);
    assert!(frames(&h).is_empty(), "no frame right after a click: {:?}", highlights(&h));
    hover(&mut h, 121.0, 100.0);
    assert_eq!(frames(&h).len(), 1, "the frame comes back with the next pointer move");

    // the frame has a filled 5 x 5 square on each corner of the bitmap bounds
    let b = bounds(&h, bmp);
    let corners = fp_ui::canvas::highlight_corners(&[], Some(b));
    assert_eq!(corners.len(), 4);
    assert_eq!((corners[0].x, corners[0].y, corners[2].x, corners[2].y), (b.left, b.top, b.right, b.bottom));

    // second click cycles back to the shape
    click(&mut h, 120.0, 100.0);
    assert_eq!(selection(&h), vec![shape], "cyclic: shape, bitmap, shape");
    // over the selected shape only: the cycle lands on the shape itself, so no frame
    hover(&mut h, 190.0, 150.0);
    assert!(frames(&h).is_empty(), "no frame over the selected front object alone");
    // over the bitmap only: the bitmap is framed
    hover(&mut h, 50.0, 50.0);
    assert_eq!(highlights(&h), vec![bounds(&h, bmp)]);

    // nothing selected: the topmost object (the shape) is framed on its path geometry
    h.state_mut().state.session.execute("select.none", json!({})).expect("none");
    h.run_steps(2);
    hover(&mut h, 120.0, 101.0);
    let f = frames(&h);
    assert_eq!(f.len(), 1, "nothing selected: frame on the topmost object");
    let anchors: Vec<(f32, f32)> = fp_ui::canvas::highlight_corners(&f[0].1, f[0].0).iter().map(|p| (p.x, p.y)).collect();
    assert_eq!(anchors.len(), 4, "corner squares on the four corners: {anchors:?}");
    for (x, y) in anchors {
        assert!((x == 80.0 || x == 200.0) && (y == 60.0 || y == 160.0), "on the path geometry, not the stroke bounds: {x},{y}");
    }

    // TW-157: the line on the path is a crisp 1 px #E00000 line on the pixel row and column inside
    // the path coordinates (top edge y = 60, left edge x = 80), not anti-aliased over two rows
    if let Some([above, on, below, left_out, left_on]) = edge_pixels(&mut h) {
        assert!(near(on, [224, 0, 0]), "top edge row is #E00000: {on:?}");
        assert!(!is_reddish(above), "nothing above the top edge: {above:?}");
        assert!(!is_reddish(below), "nothing below the top edge: {below:?}");
        assert!(near(left_on, [224, 0, 0]), "left edge column is #E00000: {left_on:?}");
        assert!(!is_reddish(left_out), "nothing left of the left edge: {left_out:?}");
    }

    // frame colour #E00000
    for t in [fp_ui::theme::Theme::normal(), fp_ui::theme::Theme::dark()] {
        assert_eq!(t.colors.hover_highlight, egui::Color32::from_rgb(224, 0, 0));
    }

    // plain Pointer: no frame of the object behind while over the selected topmost object
    h.state_mut().state.session.execute("select.set", json!({"ids": [shape.0]})).expect("select");
    h.state_mut().state.session.execute("tool.select", json!({"tool": "tool.pointer"})).expect("tool");
    h.run_steps(2);
    hover(&mut h, 121.0, 101.0);
    assert!(highlights(&h).is_empty(), "Pointer shows no hover frame over the selected shape: {:?}", highlights(&h));
}
