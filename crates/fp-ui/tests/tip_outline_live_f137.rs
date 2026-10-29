//! F137 / TW-273 (RE spec 74 §2 case a3): during a brush stroke the tip outline XORs the live
//! pixels under it, the stroke drawn so far included: a red stroke on white turns the outline over
//! the stroke green (#FF0000 XOR #FFFF00), over white it is blue (white XOR #FFFF00).
//! Both preview paths are covered: a stroke into a new bitmap (tile overlay) and a stroke on a
//! selected bitmap (composited in its stacking position, BA-039). Pixel checks in the rendered
//! window (wgpu).

use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use fp_engine::Session;
use fp_engine::tools::{Overlay, ToolId};
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
        .renderer(crate::gpu_common::renderer())
        .with_size(egui::vec2(1400.0, 900.0))
        .with_pixels_per_point(1.0)
        .with_max_steps(8)
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), a);
    h.run_steps(4);
    h
}

/// Screen point of a document point and the zoom (screen points per document pixel).
fn at(h: &H, x: f32, y: f32) -> (Pos2, f32) {
    let s = &h.state().state;
    let d = s.session.active().expect("document");
    let pr = s
        .canvases
        .get(&d.id)
        .and_then(|c| c.page_screen)
        .expect("page position");
    let r = fp_render::Renderer::page_rect(&d.doc, d.page).expect("page");
    let (w, ht) = ((r.right - r.left) as f32, (r.bottom - r.top) as f32);
    (
        egui::pos2(
            pr.left() + (x - r.left as f32) / w * pr.width(),
            pr.top() + (y - r.top as f32) / ht * pr.height(),
        ),
        pr.width() / w,
    )
}

fn ev(h: &mut H, e: Event) {
    h.input_mut().events.push(e);
    h.step();
}

/// White page 400 × 300, red 30 px brush; with `bitmap` a white bitmap 40..360 × 40..260 selected.
fn page(bitmap: bool) -> Session {
    let mut s = Session::default();
    s.execute(
        "file.new",
        json!({"width": 400, "height": 300, "background": "#ffffff"}),
    )
    .expect("document");
    if bitmap {
        let img = fp_raster::RgbaImage::filled(
            fp_geom::IRect::new(40, 40, 360, 260),
            fp_color::Rgba8::new(255, 255, 255, 255),
        )
        .expect("image");
        s.clipboard.clear();
        s.clipboard.pixels = Some(img);
        let v = s.execute("edit.paste", json!({})).expect("bitmap");
        let id = v.value["ids"][0].as_u64().expect("id");
        s.execute("select.set", json!({"ids": [id]}))
            .expect("select");
    }
    s.set_colors(Some(fp_color::Argb(0xffff_0000)), None);
    // a hard 30 px brush tip
    s.tools.options.new_brush = None;
    s.tools.options.new_stroke = Some(json!({"width": 30}));
    s
}

/// Paints from x 80 to 240 on row 150 and keeps the button held; returns the pointer position.
fn hold_stroke(h: &mut H) -> Pos2 {
    let pts: Vec<Pos2> = (0..=16).map(|i| at(h, 80.0 + i as f32 * 10.0, 150.0).0).collect();
    ev(h, Event::PointerMoved(pts[0]));
    ev(
        h,
        Event::PointerButton {
            pos: pts[0],
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        },
    );
    for p in &pts[1..] {
        ev(h, Event::PointerMoved(*p));
    }
    h.run_steps(2);
    *pts.last().expect("point")
}

fn tip_diameter(h: &H) -> f32 {
    let st = &h.state().state;
    let id = st.session.active().expect("doc").id;
    st.canvases
        .get(&id)
        .and_then(|c| {
            c.feedback.iter().find_map(|o| match o {
                Overlay::BrushTip { diameter, .. } => Some(*diameter),
                _ => None,
            })
        })
        .expect("brush tip shown")
}

fn check(bitmap: bool) {
    let mut h = harness(page(bitmap));
    let _ = h.state_mut().state.session.set_tool(ToolId::Brush);
    h.step();
    let end = hold_stroke(&mut h);
    let (_, zoom) = at(&h, 0.0, 0.0);
    let r = tip_diameter(&h) * zoom / 2.0;
    assert!(r >= 3.0, "tip radius {r}");
    let shot = match h.render() {
        Ok(i) => i,
        Err(e) => {
            eprintln!("no graphics adapter, pixel check skipped: {e}");
            return;
        }
    };
    let px = |x: f32, y: f32| {
        let v = shot.get_pixel(x.floor() as u32, y.floor() as u32).0;
        [v[0], v[1], v[2]]
    };
    // the left end of the ring lies on the stroke painted so far (the ring's other pixels lie on
    // the edge of the last dab, painted as well)
    // (searched within 2 px: the ring is a polygon approximation)
    let find = |cx: f32, want: &dyn Fn([u8; 3]) -> bool| {
        (-2..=2).any(|dy| (-2..=2).any(|dx| want(px(cx + dx as f32, end.y + dy as f32))))
    };
    let green = |c: [u8; 3]| c[0] < 80 && c[1] > 180 && c[2] < 80;
    let blue = |c: [u8; 3]| c == [0, 0, 255];
    assert!(
        find(end.x - r, &green),
        "outline over the live red stroke is green, was {:?} (bitmap {bitmap})",
        px(end.x - r, end.y)
    );
    assert!(
        !find(end.x - r, &blue),
        "no blue (pre-stroke white) outline over the stroke (bitmap {bitmap})"
    );
    ev(
        &mut h,
        Event::PointerButton {
            pos: end,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        },
    );
    h.run_steps(4);
    // hovering over white after the stroke: the outline is blue there
    let (away, _) = at(&h, 320.0, 70.0);
    ev(&mut h, Event::PointerMoved(away));
    h.run_steps(3);
    let Ok(shot) = h.render() else { return };
    let blue_at = |x: f32, y: f32| {
        (-2..=2).any(|dy| {
            (-2..=2).any(|dx| {
                let v = shot
                    .get_pixel((x + dx as f32) as u32, (y + dy as f32) as u32)
                    .0;
                [v[0], v[1], v[2]] == [0, 0, 255]
            })
        })
    };
    assert!(
        blue_at(away.x + r, away.y) && blue_at(away.x - r, away.y),
        "outline over white is blue (bitmap {bitmap})"
    );
}

#[test]
fn tip_outline_xors_the_live_stroke_in_a_new_bitmap_f137() {
    check(false);
}

#[test]
fn tip_outline_xors_the_live_stroke_on_a_selected_bitmap_f137() {
    check(true);
}
