//! FB-19 (TW-202, RE spec 60 section 1): with the Pen and the Pointer a vector path shows its
//! outline (1 px) and 5 x 5 px anchor squares instead of a bounding box with corner squares.
//! While the pen draws: no box, earlier anchors hollow, the last one filled, the open path without
//! its fill; after closing the fill appears and the first anchor is filled. A selected path with
//! the Pen: hollow anchors (like the Subselection). The Pointer: filled anchors; a rectangle
//! primitive its outline and four filled corner squares. Groups, bitmaps, texts and
//! multi-selections keep the box.

use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use fp_engine::tools::Cursor;
use fp_geom::Rect;
use fp_model::ElementId;
use fp_ui::FirepixelApp;
use fp_ui::app_state::UiSettings;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

const BLUE: [u8; 3] = [0, 153, 255];
const WHITE: [u8; 3] = [255, 255, 255];
const RED: [u8; 3] = [255, 0, 0];

fn harness(s: fp_engine::Session) -> H {
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(1200.0, 800.0)).with_pixels_per_point(1.0).with_max_steps(12).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
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

fn near(a: [u8; 3], b: [u8; 3]) -> bool {
    a.iter().zip(b).all(|(x, y)| (*x as i32 - y as i32).abs() <= 24)
}

fn bounds(h: &H, id: ElementId) -> Rect {
    let d = h.state().state.session.active().expect("document");
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, id).expect("element");
    fp_engine::docops::element_frame_bounds(&d.doc, &e).expect("bounds")
}

fn run(h: &mut H, cmd: &str, args: serde_json::Value) {
    h.state_mut().state.session.execute(cmd, args).expect(cmd);
    h.run_steps(3);
}

fn send(h: &mut H, e: Event) {
    h.input_mut().events.push(e);
    h.step();
}

fn button(p: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos: p, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

fn hover(h: &mut H, x: f32, y: f32) {
    let p = at(h, x, y);
    send(h, Event::PointerMoved(p));
    h.run_steps(2);
}

fn click(h: &mut H, x: f32, y: f32) {
    let p = at(h, x, y);
    send(h, Event::PointerMoved(p));
    send(h, button(p, true));
    send(h, button(p, false));
    // apart from the next click: no double-click
    h.run_steps(30);
}

/// Rendered screenshot as a pixel reader, `None` without a graphics adapter.
struct Shot(image::RgbaImage);

impl Shot {
    fn take(h: &mut H) -> Option<Shot> {
        match h.render() {
            Ok(i) => Some(Shot(i)),
            Err(e) => {
                eprintln!("no graphics adapter, pixel check skipped: {e}");
                None
            }
        }
    }
    fn px(&self, p: Pos2, dx: i32, dy: i32) -> [u8; 3] {
        let x = (p.x.floor() as i32 + dx).max(0) as u32;
        let y = (p.y.floor() as i32 + dy).max(0) as u32;
        let q = self.0.get_pixel(x, y).0;
        [q[0], q[1], q[2]]
    }
    /// Any pixel within `r` px of `p` near the colour.
    fn any_near(&self, p: Pos2, r: i32, col: [u8; 3]) -> bool {
        (-r..=r).any(|dy| (-r..=r).any(|dx| near(self.px(p, dx, dy), col)))
    }
    /// The anti-aliased outline over the black stroke: a clearly blue pixel within `r` px.
    fn bluish(&self, p: Pos2, r: i32) -> bool {
        (-r..=r).any(|dy| (-r..=r).any(|dx| {
            let q = self.px(p, dx, dy);
            q[0] < 60 && q[2] > 190 && q[2] as i32 - q[0] as i32 > 150
        }))
    }
    fn hollow(&self, p: Pos2) -> bool {
        near(self.px(p, 0, 0), WHITE) && near(self.px(p, 2, 0), BLUE) && near(self.px(p, -2, 0), BLUE) && near(self.px(p, 0, 2), BLUE) && near(self.px(p, 0, -2), BLUE)
    }
    fn filled(&self, p: Pos2) -> bool {
        near(self.px(p, 0, 0), BLUE) && near(self.px(p, 1, 1), BLUE) && near(self.px(p, -1, -1), BLUE)
    }
}

fn node(a: (f64, f64), i: (f64, f64), o: (f64, f64)) -> serde_json::Value {
    json!({"anchor": [a.0, a.1], "in": [i.0, i.1], "out": [o.0, o.1]})
}

fn new_id(v: &serde_json::Value) -> ElementId {
    ElementId(v["id"].as_u64().or_else(|| v["ids"][0].as_u64()).expect("id"))
}

fn session() -> fp_engine::Session {
    fp_engine::shell::disable();
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 500, "height": 360, "background": "#ffffff"})).expect("doc");
    s
}

/// Pen: two, then three clicks; no box, anchors hollow + last filled, outline; after closing the
/// first anchor filled, the others hollow.
#[test]
fn fb19_pen_while_drawing_shows_outline_and_anchors_without_box() {
    let mut s = session();
    s.execute("tool.select", json!({"tool": "tool.pen"})).expect("tool");
    let mut h = harness(s);
    hover(&mut h, 400.0, 300.0);
    assert_eq!(h.state().state.tool_cursor, Cursor::PenNew, "pen with x before a path");
    click(&mut h, 60.0, 60.0);
    click(&mut h, 200.0, 80.0);
    hover(&mut h, 300.0, 250.0);
    assert_eq!(h.state().state.tool_cursor, Cursor::Pen, "plain pen while drawing");
    let (a, b) = (at(&h, 60.0, 60.0), at(&h, 200.0, 80.0));
    let mid = at(&h, 130.0, 70.0);
    if let Some(s) = Shot::take(&mut h) {
        assert!(s.hollow(a), "first anchor hollow");
        assert!(s.filled(b), "last anchor filled");
        assert!(s.bluish(mid, 1), "outline over the stroke");
        assert!(!s.any_near(at(&h, 60.0, 80.0), 2, BLUE), "no box corner bottom left");
        assert!(!s.any_near(at(&h, 200.0, 60.0), 2, BLUE), "no box corner top right");
    }
    click(&mut h, 120.0, 200.0);
    hover(&mut h, 420.0, 300.0);
    let c = at(&h, 120.0, 200.0);
    let id = h.state().state.session.active().expect("doc").selection.elements[0];
    let bx = bounds(&h, id);
    let (box_bl, box_br) = (at(&h, bx.left, bx.bottom), at(&h, bx.right, bx.bottom));
    if let Some(s) = Shot::take(&mut h) {
        assert!(s.hollow(a) && s.hollow(b), "earlier anchors hollow");
        assert!(s.filled(c), "last anchor filled");
        assert!(!s.any_near(box_bl, 2, BLUE) && !s.any_near(box_br, 2, BLUE), "no box corners");
    }
    // close on the first point: the first anchor filled, the others hollow, still no box
    click(&mut h, 60.0, 60.0);
    hover(&mut h, 420.0, 300.0);
    if let Some(s) = Shot::take(&mut h) {
        assert!(s.filled(a), "first anchor filled after closing");
        assert!(s.hollow(b) && s.hollow(c), "other anchors hollow after closing");
        assert!(!s.any_near(box_bl, 2, BLUE) && !s.any_near(box_br, 2, BLUE), "no box after closing");
    }
}

/// Choosing the Pen on a selected open path with a fill resumes drawing: the fill is not shown
/// while the path is open and appears when a click on its first point closes it.
#[test]
fn fb19_pen_open_path_fill_hidden_until_closed() {
    let mut s = session();
    let v = s
        .execute(
            "draw.path",
            json!({"contours": [{"closed": false, "nodes": [
                node((60.0, 60.0), (60.0, 60.0), (60.0, 60.0)),
                node((200.0, 80.0), (200.0, 80.0), (200.0, 80.0)),
                node((120.0, 200.0), (120.0, 200.0), (120.0, 200.0)),
            ]}], "fillColor": "#ff0000", "strokeColor": "none"}),
        )
        .expect("path");
    let id = new_id(&v.value);
    s.execute("select.set", json!({"ids": [id.0]})).expect("select");
    s.execute("pi.fill", json!({"ids": [id.0], "category": "solid", "color": "#ff0000"})).expect("fill");
    let mut h = harness(s);
    let inside = at(&h, 127.0, 113.0);
    if let Some(s) = Shot::take(&mut h) {
        assert!(near(s.px(inside, 0, 0), RED), "Pointer: the open path shows its fill: {:?}", s.px(inside, 0, 0));
    }
    run(&mut h, "tool.select", json!({"tool": "tool.pen"}));
    hover(&mut h, 420.0, 300.0);
    assert_eq!(h.state().state.tool_cursor, Cursor::Pen, "drawing resumed: plain pen");
    if let Some(s) = Shot::take(&mut h) {
        assert!(near(s.px(inside, 0, 0), WHITE), "no fill while the path is open: {:?}", s.px(inside, 0, 0));
        assert!(s.filled(at(&h, 120.0, 200.0)), "the end point filled");
        assert!(s.hollow(at(&h, 60.0, 60.0)), "the first point hollow");
    }
    click(&mut h, 60.0, 60.0);
    hover(&mut h, 420.0, 300.0);
    if let Some(s) = Shot::take(&mut h) {
        assert!(near(s.px(inside, 0, 0), RED), "fill after closing: {:?}", s.px(inside, 0, 0));
        assert!(s.filled(at(&h, 60.0, 60.0)), "first anchor filled after closing");
        assert!(s.hollow(at(&h, 200.0, 80.0)), "other anchors hollow after closing");
    }
}

/// Closed path A(60,60) – B(200,60) –curve– C(200,180) – D(60,180) (bulge to x = 245), a rectangle
/// primitive, a bitmap, a text, a group of two ellipses.
#[test]
fn fb19_selected_path_with_pen_and_pointer_kinds() {
    let mut s = session();
    let v = s
        .execute(
            "draw.path",
            json!({"contours": [{"closed": true, "nodes": [
                node((60.0, 60.0), (60.0, 60.0), (60.0, 60.0)),
                node((200.0, 60.0), (200.0, 60.0), (260.0, 80.0)),
                node((200.0, 180.0), (260.0, 160.0), (200.0, 180.0)),
                node((60.0, 180.0), (60.0, 180.0), (60.0, 180.0)),
            ]}], "fillColor": "#ffffff", "strokeColor": "none", "name": "P"}),
        )
        .expect("path");
    let path = new_id(&v.value);
    let rect = new_id(&s.execute("draw.rectangle", json!({"rect": [300, 40, 420, 120]})).expect("rect").value);
    let img = fp_raster::RgbaImage::filled(fp_geom::IRect::from_xywh(300, 200, 60, 50), fp_color::Rgba8::new(40, 160, 40, 255)).expect("image");
    s.clipboard.clear();
    s.clipboard.pixels = Some(img);
    let bmp = new_id(&s.execute("edit.paste", json!({})).expect("paste").value);
    let text = new_id(&s.execute("draw.text", json!({"text": "Testtext", "anchor": [400, 300], "size": 20})).expect("text").value);
    let e1 = new_id(&s.execute("draw.ellipse", json!({"rect": [40, 230, 100, 290]})).expect("e1").value);
    let e2 = new_id(&s.execute("draw.ellipse", json!({"rect": [120, 250, 200, 330]})).expect("e2").value);
    s.execute("select.set", json!({"ids": [path.0]})).expect("select");
    s.execute("tool.select", json!({"tool": "tool.pen"})).expect("tool");
    let mut h = harness(s);
    let b = bounds(&h, path);
    let (a, bb, cc) = (at(&h, 60.0, 60.0), at(&h, 200.0, 60.0), at(&h, 60.0, 180.0));
    let curve = at(&h, 245.0, 120.0);
    let box_tr = at(&h, b.right, b.top);
    let box_br = at(&h, b.right, b.bottom);

    // Pen: outline + hollow anchors, no box
    if let Some(s) = Shot::take(&mut h) {
        assert!(!s.any_near(box_tr, 3, BLUE) && !s.any_near(box_br, 3, BLUE), "Pen: no box corners");
        assert!(s.any_near(curve, 1, BLUE), "Pen: outline on the curve");
        for p in [a, bb, cc] {
            assert!(s.hollow(p), "Pen: hollow anchor at {p:?}");
        }
    }

    // Pointer: outline + filled anchors, no box
    run(&mut h, "tool.select", json!({"tool": "tool.pointer"}));
    if let Some(s) = Shot::take(&mut h) {
        assert!(!s.any_near(box_tr, 3, BLUE) && !s.any_near(box_br, 3, BLUE), "Pointer: no box corners");
        assert!(s.any_near(curve, 1, BLUE), "Pointer: outline on the curve");
        for p in [a, bb, cc] {
            assert!(s.filled(p), "Pointer: filled anchor at {p:?}");
        }
    }

    // rectangle primitive, Pointer and Pen: outline + four filled corner squares
    run(&mut h, "select.set", json!({"ids": [rect.0]}));
    let corners = [at(&h, 300.0, 40.0), at(&h, 420.0, 40.0), at(&h, 420.0, 120.0), at(&h, 300.0, 120.0)];
    let edge = at(&h, 360.0, 40.0);
    for tool in ["tool.pointer", "tool.pen"] {
        run(&mut h, "tool.select", json!({"tool": tool}));
        if let Some(s) = Shot::take(&mut h) {
            for p in corners {
                assert!(s.filled(p), "{tool}: filled corner square at {p:?}");
            }
            assert!(s.any_near(edge, 1, BLUE), "{tool}: outline on the top edge");
        }
    }

    // Pointer: bitmap, text, group and a multi-selection keep the box with its corner squares
    run(&mut h, "tool.select", json!({"tool": "tool.pointer"}));
    run(&mut h, "select.set", json!({"ids": [e1.0, e2.0]}));
    let b1 = bounds(&h, e1);
    let multi_tr = at(&h, b1.right, b1.top);
    if let Some(s) = Shot::take(&mut h) {
        assert!(s.any_near(multi_tr, 2, BLUE), "multi-selection: box corner square");
    }
    run(&mut h, "modify.group", json!({}));
    let group = h.state().state.session.active().expect("doc").selection.elements[0];
    for (what, id) in [("bitmap", bmp), ("text", text), ("group", group)] {
        run(&mut h, "select.set", json!({"ids": [id.0]}));
        let bx = bounds(&h, id);
        let (tr, bl) = (at(&h, bx.right, bx.top), at(&h, bx.left, bx.bottom));
        if let Some(s) = Shot::take(&mut h) {
            assert!(s.any_near(tr, 2, BLUE) && s.any_near(bl, 2, BLUE), "Pointer on a {what}: box corner squares");
        }
    }
}
