//! FB-06 (re 54 §1, §6): with the Subselection tool a selected vector object shows its path outline
//! (1 px, real curves) with a 5 x 5 px square on every anchor (hollow, selected ones filled) and the
//! handles of every segment with a selected end, but no bounding box and no corner squares.
//! Bitmaps keep the box; the Pointer shows the outline with filled anchors (RE spec 60 section 1.3);
//! `ShowSolidPoints` reverses the fill; hidden edges show nothing.

use egui::Pos2;
use egui_kittest::Harness;
use fp_engine::prefs::PrefValue;
use fp_geom::Rect;
use fp_model::ElementId;
use fp_ui::FirepixelApp;
use fp_ui::app_state::UiSettings;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

const BLUE: [u8; 3] = [0, 153, 255];
const WHITE: [u8; 3] = [255, 255, 255];

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
}

fn node(a: (f64, f64), i: (f64, f64), o: (f64, f64)) -> serde_json::Value {
    json!({"anchor": [a.0, a.1], "in": [i.0, i.1], "out": [o.0, o.1]})
}

/// Closed path A(60,60) – B(200,60) –curve– C(200,180) – D(60,180); the curve bulges to x = 245
/// at y = 120 (handles B.out = (260,80), C.in = (260,160)).
fn setup() -> (H, ElementId, ElementId) {
    fp_engine::shell::disable();
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 400, "height": 300, "background": "#ffffff"})).expect("doc");
    let img = fp_raster::RgbaImage::filled(fp_geom::IRect::from_xywh(300, 200, 60, 50), fp_color::Rgba8::new(40, 160, 40, 255)).expect("image");
    s.clipboard.clear();
    s.clipboard.pixels = Some(img);
    let v = s.execute("edit.paste", json!({})).expect("paste");
    let bmp = ElementId(v.value["ids"][0].as_u64().expect("bitmap id"));
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
    let path = ElementId(v.value["id"].as_u64().or_else(|| v.value["ids"][0].as_u64()).expect("path id"));
    s.execute("select.set", json!({"ids": [path.0]})).expect("select");
    s.execute("tool.select", json!({"tool": "tool.subselection"})).expect("tool");
    (harness(s), path, bmp)
}

#[test]
fn subselection_handles_follow_the_segments_of_selected_points() {
    let n = |x: f32| fp_model::ContourNode::corner(fp_geom::Point::new(x, 0.0));
    let open = fp_model::Contour { closed: false, bsl: None, nodes: vec![n(0.0), n(1.0), n(2.0), n(3.0)], extra: Default::default() };
    // a middle point: both own handles and the facing handles of the neighbours
    assert_eq!(fp_ui::canvas::subselection_handles(std::slice::from_ref(&open), &[(0, 1)]), vec![(0, 0, true), (0, 1, false), (0, 1, true), (0, 2, false)]);
    // the first point of an open path: its outer handle too
    assert_eq!(fp_ui::canvas::subselection_handles(std::slice::from_ref(&open), &[(0, 0)]), vec![(0, 0, false), (0, 0, true), (0, 1, false)]);
    // closed: the wrap segment counts
    let closed = fp_model::Contour { closed: true, ..open };
    assert_eq!(fp_ui::canvas::subselection_handles(&[closed], &[(0, 0)]), vec![(0, 0, false), (0, 0, true), (0, 1, false), (0, 3, true)]);
    assert!(fp_ui::canvas::subselection_handles(&[], &[]).is_empty());
}

#[test]
fn fb06_subselection_shows_outline_and_anchors_without_box() {
    let (mut h, path, bmp) = setup();
    let b = bounds(&h, path);
    assert!(b.right > 240.0, "bounds include the curve: {b:?}");
    let (a, bb, cc) = (at(&h, 60.0, 60.0), at(&h, 200.0, 60.0), at(&h, 60.0, 180.0));
    let curve = at(&h, 245.0, 120.0);
    let top_mid = at(&h, 130.0, 60.0);
    let box_tr = at(&h, b.right, b.top);
    let box_br = at(&h, b.right, b.bottom);
    let (h_out, c_in) = (at(&h, 260.0, 80.0), at(&h, 260.0, 160.0));
    let Some(s) = Shot::take(&mut h) else { return };
    // no bounding box, no corner squares
    assert!(!s.any_near(box_tr, 3, BLUE), "no box corner square top right");
    assert!(!s.any_near(box_br, 3, BLUE), "no box corner square bottom right");
    // outline along the real curve and the straight edges
    assert!(s.any_near(curve, 1, BLUE), "outline on the curve at x = 245");
    assert!(s.any_near(top_mid, 1, BLUE), "outline on the top edge");
    // hollow anchor squares: white 3 x 3 inside, 1 px blue border, 5 x 5 in all
    for p in [a, bb, cc] {
        assert!(near(s.px(p, 0, 0), WHITE), "hollow anchor: white centre {:?}", s.px(p, 0, 0));
        assert!(near(s.px(p, 1, 1), WHITE), "hollow anchor: white inside");
        assert!(near(s.px(p, 2, 0), BLUE) && near(s.px(p, -2, 0), BLUE) && near(s.px(p, 0, 2), BLUE) && near(s.px(p, 0, -2), BLUE), "hollow anchor: blue border");
    }
    // no handles without selected points
    assert!(!s.any_near(h_out, 1, BLUE), "no handle without a selected point");

    // select point B: filled, its handle and the facing handle of C appear
    run(&mut h, "select.points", json!({"points": [{"element": path.0, "contour": 0, "node": 1}], "mode": "replace"}));
    let Some(s) = Shot::take(&mut h) else { return };
    assert!(near(s.px(bb, 0, 0), BLUE) && near(s.px(bb, 1, 1), BLUE), "selected anchor filled");
    assert!(near(s.px(a, 0, 0), WHITE), "unselected anchor stays hollow");
    assert!(s.any_near(h_out, 1, BLUE), "own handle of the selected point");
    assert!(s.any_near(c_in, 1, BLUE), "facing handle of the neighbour");
    assert!(!s.any_near(box_tr, 3, BLUE), "still no box");

    // Shift+click toggles: B selected, D added, B removed again
    run(&mut h, "select.points", json!({"points": [{"element": path.0, "contour": 0, "node": 3}], "mode": "add"}));
    run(&mut h, "select.points", json!({"points": [{"element": path.0, "contour": 0, "node": 1}], "mode": "subtract"}));
    let pts: Vec<usize> = h.state().state.session.active().expect("doc").selection.points.iter().map(|p| p.node).collect();
    assert_eq!(pts, vec![3]);
    run(&mut h, "select.points", json!({"points": [{"element": path.0, "contour": 0, "node": 1}], "mode": "replace"}));

    // ShowSolidPoints reverses the fill
    h.state_mut().state.session.prefs.set("ShowSolidPoints", PrefValue::Bool(true));
    h.run_steps(3);
    let Some(s) = Shot::take(&mut h) else { return };
    assert!(near(s.px(a, 0, 0), BLUE), "ShowSolidPoints: unselected anchor filled");
    assert!(near(s.px(bb, 0, 0), WHITE), "ShowSolidPoints: selected anchor hollow");
    h.state_mut().state.session.prefs.set("ShowSolidPoints", PrefValue::Bool(false));

    // hidden edges: no selection feedback at all
    run(&mut h, "view.edges", json!({}));
    assert!(!h.state().state.session.active().expect("doc").view.show_edges);
    let Some(s) = Shot::take(&mut h) else { return };
    for p in [a, bb, curve, h_out, box_tr] {
        assert!(!s.any_near(p, 1, BLUE), "edges hidden: nothing at {p:?}");
    }
    run(&mut h, "view.edges", json!({}));

    // Pointer: no box either, the outline with filled anchors (RE spec 60 section 1.3, FB-19)
    run(&mut h, "tool.select", json!({"tool": "tool.pointer"}));
    let Some(s) = Shot::take(&mut h) else { return };
    assert!(!s.any_near(box_tr, 2, BLUE), "Pointer: no box corner square top right");
    assert!(!s.any_near(box_br, 2, BLUE), "Pointer: no box corner square bottom right");
    assert!(s.any_near(curve, 1, BLUE), "Pointer: outline on the curve");
    assert!(near(s.px(a, 0, 0), BLUE), "Pointer: filled anchor");

    // a bitmap with the Subselection tool keeps the box
    run(&mut h, "tool.select", json!({"tool": "tool.subselection"}));
    run(&mut h, "select.set", json!({"ids": [bmp.0]}));
    let bb2 = bounds(&h, bmp);
    let (tl, br) = (at(&h, bb2.left, bb2.top), at(&h, bb2.right, bb2.bottom));
    let Some(s) = Shot::take(&mut h) else { return };
    assert!(s.any_near(tl, 2, BLUE) && s.any_near(br, 2, BLUE), "Subselection on a bitmap: box corner squares");
}
