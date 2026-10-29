//! F141 (FB-118 … FB-122, RE spec 76): dragging objects with the Pointer shows the document
//! composed normally with the objects moved (no ghost sprite): blend modes, opacity, live effects
//! and the objects above and below stay right, and the image while the button is held equals the
//! image after the release. No selection frame and no handles during the drag; one History row on
//! release, none after Esc. Start threshold d² > 9 / zoom (document units), offset in whole
//! document pixels, small drags snap back to the start with smart guides on. "Preview drag" off:
//! the old image stays and only the outline moves.

use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use fp_engine::tools::ToolId;
use fp_engine::{Overlay, Session};
use fp_ui::FirepixelApp;
use fp_ui::app_state::UiSettings;
use serde_json::json;

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

fn gpu_harness(s: Session) -> H {
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

fn ev(h: &mut H, e: Event) {
    h.input_mut().events.push(e);
    h.step();
}

fn button(h: &mut H, p: Pos2, pressed: bool) {
    ev(
        h,
        Event::PointerButton {
            pos: p,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        },
    );
}

fn set_tool(h: &mut H, t: ToolId) {
    let _ = h.state_mut().state.session.set_tool(t);
    h.step();
}

fn ok(s: &mut Session, cmd: &str, p: serde_json::Value) {
    s.execute(cmd, p).unwrap_or_else(|e| panic!("{cmd}: {e:?}"));
}

fn selected(s: &Session) -> u64 {
    s.active().expect("document").selection.elements[0].0
}

fn rect(s: &mut Session, r: [i32; 4], color: &str) -> u64 {
    ok(
        s,
        "draw.rectangle",
        json!({"rect": r, "fillColor": color, "strokeWidth": 0}),
    );
    selected(s)
}

/// Variant of the moved object M of scene S76 (RE spec 76).
#[derive(Clone, Copy, PartialEq)]
enum M {
    Normal,
    Multiply,
    Half,
    Shadow,
    Bitmap,
    Group,
}

/// Scene S76: page 600 × 400 white, B blue (60,60)–(260,260), M yellow (160,120)–(320,220), T
/// grey (280,80)–(400,280) above (`invert_top`: blend mode Invert). M selected.
fn scene(m: M, invert_top: bool) -> Session {
    let mut s = Session::default();
    ok(
        &mut s,
        "file.new",
        json!({"width": 600, "height": 400, "background": "#ffffff"}),
    );
    rect(&mut s, [60, 60, 260, 260], "#3366ff");
    let mid = match m {
        M::Bitmap => {
            let img = fp_raster::RgbaImage::filled(
                fp_geom::IRect::new(160, 120, 320, 220),
                fp_color::Rgba8::new(255, 204, 0, 255),
            )
            .expect("image");
            s.clipboard.clear();
            s.clipboard.pixels = Some(img);
            let v = s.execute("edit.paste", json!({})).expect("paste");
            v.value["ids"][0].as_u64().expect("bitmap id")
        }
        M::Group => {
            let a = rect(&mut s, [160, 120, 240, 220], "#ffcc00");
            let b = rect(&mut s, [240, 120, 320, 220], "#ff8800");
            ok(&mut s, "select.set", json!({"ids": [a, b]}));
            ok(&mut s, "modify.group", json!({}));
            selected(&s)
        }
        _ => rect(&mut s, [160, 120, 320, 220], "#ffcc00"),
    };
    ok(&mut s, "select.set", json!({"ids": [mid]}));
    match m {
        M::Multiply | M::Bitmap | M::Group => {
            ok(&mut s, "element.setBlendMode", json!({"mode": "multiply"}))
        }
        M::Half => ok(&mut s, "element.setOpacity", json!({"opacity": 50})),
        M::Shadow => {
            ok(&mut s, "element.setBlendMode", json!({"mode": "multiply"}));
            ok(&mut s, "element.setEffect", json!({"effect": "dropShadow"}));
        }
        M::Normal => {}
    }
    let top = rect(&mut s, [280, 80, 400, 280], "#808080");
    if invert_top {
        ok(
            &mut s,
            "element.setBlendMode",
            json!({"ids": [top], "mode": "invert"}),
        );
    }
    ok(&mut s, "select.set", json!({"ids": [mid]}));
    s
}

fn canvas(h: &H) -> &fp_ui::canvas::render::CanvasCache {
    h.state().state.canvases.values().next().expect("canvas")
}

fn shown(h: &H) -> fp_raster::RgbaImage {
    canvas(h).shown_image.clone().expect("canvas image")
}

fn moved_by(h: &H) -> Option<fp_geom::Point> {
    canvas(h).feedback.iter().find_map(|o| match o {
        Overlay::Ghost { moved_by, .. } => *moved_by,
        _ => None,
    })
}

fn history_len(h: &H) -> usize {
    h.state()
        .state
        .session
        .active()
        .expect("document")
        .history
        .steps()
        .len()
}

/// Presses on M at (200,170), drags it by (40, 0) and checks the held frame against the frame
/// after the release.
fn held_equals_released(m: M, invert_top: bool) {
    let mut h = harness(scene(m, invert_top));
    set_tool(&mut h, ToolId::Pointer);
    h.run_steps(2);
    let before = shown(&h);
    let rows = history_len(&h);
    let (a, b) = (at(&h, 200.0, 170.0), at(&h, 240.0, 170.0));
    ev(&mut h, Event::PointerMoved(a));
    button(&mut h, a, true);
    ev(&mut h, Event::PointerMoved(a + egui::vec2(10.0, 0.0)));
    ev(&mut h, Event::PointerMoved(b));
    h.run_steps(3);
    let held = {
        let cc = canvas(&h);
        assert_eq!(
            moved_by(&h),
            Some(fp_geom::Point::new(40.0, 0.0)),
            "move drag"
        );
        let k = cc.shown.expect("shown");
        assert!(k.preview != 0, "rendered from the moved copy");
        assert_eq!(k.variant, 0, "nothing hidden in the canvas image");
        assert!(
            cc.ghosts.drag.iter().all(Option::is_none),
            "no ghost sprite"
        );
        assert!(cc.layers.is_empty(), "no layers over a sprite");
        assert!(
            cc.sel_frames.is_empty(),
            "no selection frame and no handles while dragging"
        );
        shown(&h)
    };
    assert_ne!(held.as_bytes(), before.as_bytes(), "the drag shows the move");
    assert_eq!(history_len(&h), rows, "no History row while held");
    button(&mut h, b, false);
    h.run_steps(4);
    let cc = canvas(&h);
    assert_eq!(cc.shown.map(|k| k.preview), Some(0), "committed render");
    assert!(!cc.sel_frames.is_empty(), "the frame is back after release");
    let after = shown(&h);
    assert_eq!(held.bounds(), after.bounds());
    let diff = held
        .as_bytes()
        .iter()
        .zip(after.as_bytes())
        .filter(|(x, y)| x != y)
        .count();
    assert_eq!(diff, 0, "held frame equals the frame after the release");
    assert_eq!(history_len(&h), rows + 1, "one History row on release");
}

#[test]
fn held_equals_released_normal() {
    held_equals_released(M::Normal, false);
}

#[test]
fn held_equals_released_multiply() {
    held_equals_released(M::Multiply, false);
}

#[test]
fn held_equals_released_inverted_object_above() {
    held_equals_released(M::Multiply, true);
}

#[test]
fn held_equals_released_half_opacity() {
    held_equals_released(M::Half, true);
}

#[test]
fn held_equals_released_drop_shadow() {
    held_equals_released(M::Shadow, false);
}

#[test]
fn held_equals_released_bitmap() {
    held_equals_released(M::Bitmap, true);
}

#[test]
fn held_equals_released_group() {
    held_equals_released(M::Group, true);
}

/// Esc during the drag puts the objects back: the start image, no History row.
#[test]
fn escape_during_drag_restores_without_history() {
    let mut h = harness(scene(M::Multiply, true));
    set_tool(&mut h, ToolId::Pointer);
    h.run_steps(2);
    let before = shown(&h);
    let rows = history_len(&h);
    let (a, b) = (at(&h, 200.0, 170.0), at(&h, 240.0, 170.0));
    ev(&mut h, Event::PointerMoved(a));
    button(&mut h, a, true);
    ev(&mut h, Event::PointerMoved(a + egui::vec2(10.0, 0.0)));
    ev(&mut h, Event::PointerMoved(b));
    h.run_steps(2);
    assert!(moved_by(&h).is_some());
    ev(
        &mut h,
        Event::Key {
            key: Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        },
    );
    h.run_steps(3);
    assert_eq!(moved_by(&h), None, "drag cancelled");
    assert_eq!(shown(&h).as_bytes(), before.as_bytes(), "start image again");
    button(&mut h, b, false);
    h.run_steps(3);
    assert_eq!(history_len(&h), rows, "Esc writes no History row");
    let d = h.state().state.session.active().expect("document");
    let id = d.selection.elements[0];
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, id).expect("M");
    let bnd = fp_engine::docops::element_bounds(&d.doc, &e).expect("bounds");
    assert_eq!((bnd.left, bnd.top), (160.0, 120.0), "M stays at the start");
}

fn smart_guides(h: &mut H, on: bool) {
    if let Some(d) = h.state_mut().state.session.active_mut() {
        d.view.snap_to_smart_guides = on;
    }
}

/// Moves of `steps` screen px from `from` (document point); returns the move offset per step
/// (`None` = no drag yet).
fn offsets(h: &mut H, from: (f32, f32), steps: &[(f32, f32)]) -> Vec<Option<(f32, f32)>> {
    let a = at(h, from.0, from.1);
    ev(h, Event::PointerMoved(a));
    button(h, a, true);
    let mut out = Vec::new();
    for (dx, dy) in steps {
        ev(h, Event::PointerMoved(a + egui::vec2(*dx, *dy)));
        out.push(moved_by(h).map(|p| (p.x, p.y)));
    }
    button(h, a, false);
    h.run_steps(2);
    out
}

/// RE spec 76 §4 at 100 %: no drag up to 3 px (d² ≤ 9), 4 px starts it with the full offset,
/// (2,2) does not, (3,2) does; whole document pixels.
#[test]
fn start_threshold_at_100_percent() {
    let mut h = harness(scene(M::Normal, false));
    set_tool(&mut h, ToolId::Pointer);
    smart_guides(&mut h, false);
    h.run_steps(2);
    let o = offsets(&mut h, (200.5, 170.5), &[(1.0, 0.0), (2.0, 0.0), (3.0, 0.0)]);
    assert_eq!(o, vec![None, None, None], "up to 3 px no drag");
    let o = offsets(&mut h, (200.5, 170.5), &[(4.0, 0.0), (5.0, 0.0)]);
    assert_eq!(o, vec![Some((4.0, 0.0)), Some((5.0, 0.0))]);
    let o = offsets(&mut h, (200.5, 170.5), &[(2.0, 2.0)]);
    assert_eq!(o, vec![None], "(2,2): d² = 8");
    let o = offsets(&mut h, (200.5, 170.5), &[(3.0, 2.0)]);
    assert_eq!(o, vec![Some((3.0, 2.0))], "(3,2): d² = 13");
}

/// RE spec 76 §4 with smart guides on (default): a small drag starts but snaps back to the start
/// position (offset 0).
#[test]
fn small_drag_snaps_back_to_the_start_with_smart_guides() {
    let mut h = harness(scene(M::Normal, false));
    set_tool(&mut h, ToolId::Pointer);
    smart_guides(&mut h, true);
    h.run_steps(2);
    let o = offsets(&mut h, (200.5, 170.5), &[(0.0, 4.0)]);
    assert_eq!(o, vec![Some((0.0, 0.0))], "4 px down snaps back to 0");
    let o = offsets(&mut h, (200.5, 170.5), &[(0.0, 4.0), (0.0, 30.0)]);
    assert_eq!(o[1], Some((0.0, 30.0)), "a larger drag moves freely");
}

/// RE spec 76 §4 at 800 %: the drag starts beyond 3·√8 ≈ 8.49 screen px, the offset is
/// floor(p) − floor(p0) in whole document pixels (8-screen-px steps).
#[test]
fn start_threshold_and_whole_pixels_at_800_percent() {
    let mut s = Session::default();
    ok(
        &mut s,
        "file.new",
        json!({"width": 100, "height": 70, "background": "#ffffff"}),
    );
    rect(&mut s, [30, 20, 60, 50], "#000000");
    let mut h = harness(s);
    set_tool(&mut h, ToolId::Pointer);
    smart_guides(&mut h, false);
    let _ = h
        .state_mut()
        .state
        .session
        .execute("view.setZoom", json!({"zoom": 8, "center": [45, 35]}));
    h.run_steps(3);
    let o = offsets(&mut h, (45.5, 35.5), &[(6.0, 0.0), (8.0, 0.0)]);
    assert_eq!(o, vec![None, None], "up to 8 screen px no drag");
    let o = offsets(
        &mut h,
        (45.5, 35.5),
        &[(9.0, 0.0), (12.0, 0.0), (17.0, 0.0), (24.0, 0.0), (64.0, 0.0)],
    );
    assert_eq!(
        o,
        vec![
            Some((1.0, 0.0)),
            Some((2.0, 0.0)),
            Some((2.0, 0.0)),
            Some((3.0, 0.0)),
            Some((8.0, 0.0))
        ]
    );
}

/// "Preview drag" off (RE spec 76 §3): the old image stays, no frame and no handles, the outline
/// is drawn 1 px opaque #00A8FF at the new position; the release commits normally.
#[test]
fn preview_drag_off_keeps_the_image_and_draws_the_outline() {
    let mut h = gpu_harness(scene(M::Multiply, true));
    h.state_mut()
        .state
        .session
        .prefs
        .set("PreviewDrag", fp_engine::prefs::PrefValue::Bool(false));
    set_tool(&mut h, ToolId::Pointer);
    smart_guides(&mut h, false);
    h.run_steps(2);
    let before = shown(&h);
    let (a, b) = (at(&h, 200.0, 170.0), at(&h, 240.0, 170.0));
    ev(&mut h, Event::PointerMoved(a));
    button(&mut h, a, true);
    ev(&mut h, Event::PointerMoved(a + egui::vec2(10.0, 0.0)));
    ev(&mut h, Event::PointerMoved(b));
    h.run_steps(3);
    {
        let cc = canvas(&h);
        assert_eq!(moved_by(&h), Some(fp_geom::Point::new(40.0, 0.0)));
        assert_eq!(cc.shown.map(|k| (k.preview, k.variant)), Some((0, 0)));
        assert!(cc.ghosts.drag.iter().all(Option::is_none), "no sprite");
        assert!(cc.sel_frames.is_empty(), "no frame, no handles");
    }
    assert_eq!(shown(&h).as_bytes(), before.as_bytes(), "the old image stays");
    // the outline at the new place: M's top edge y = 120 from x = 200 to 360
    let p = at(&h, 300.0, 120.0);
    match h.render() {
        Ok(img) => {
            let hit = (-1..=1).any(|dy| {
                let px = img.get_pixel(p.x as u32, (p.y as i32 + dy) as u32).0;
                px[0] == 0x00 && px[1] == 0xa8 && px[2] == 0xff
            });
            assert!(hit, "outline #00A8FF at the new top edge");
        }
        Err(e) => eprintln!("no graphics adapter, outline pixel check skipped: {e}"),
    }
    button(&mut h, b, false);
    h.run_steps(4);
    let d = h.state().state.session.active().expect("document");
    let id = d.selection.elements[0];
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, id).expect("M");
    let bnd = fp_engine::docops::element_bounds(&d.doc, &e).expect("bounds");
    assert_eq!((bnd.left, bnd.top), (200.0, 120.0), "moved on release");
}
