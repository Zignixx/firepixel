//! FB-90 (TW-268, RE spec 68 §4.2, §7): selecting a bitmap mask (mask thumbnail = Edit Mask,
//! Subselection) starts bitmap editing of the mask; the masked object must keep its look on the
//! canvas while the mask is selected and after deselecting. Pixel checks in the rendered window.
//! With Paste Inside the "Show fill and stroke" underlay stays visible while the mask is edited
//! (RE spec 73 §0.5 and §7, superseding RE spec 68 §4.1 `e9`/`e10`).

use egui::Pos2;
use egui_kittest::Harness;
use fp_engine::Session;
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

fn ex(h: &mut H, name: &str, p: Value) -> Value {
    let v = h
        .state_mut()
        .state
        .session
        .execute(name, p)
        .map(|o| o.value)
        .unwrap_or(Value::Null);
    h.run_steps(4);
    v
}

/// RGB at document points; `None` without a graphics adapter.
fn rgb(h: &mut H, pts: &[(f32, f32)]) -> Option<Vec<[u8; 3]>> {
    let ps: Vec<Pos2> = pts.iter().map(|(x, y)| at(h, *x, *y)).collect();
    let shot = match h.render() {
        Ok(i) => i,
        Err(e) => {
            eprintln!("no graphics adapter, pixel check skipped: {e}");
            return None;
        }
    };
    Some(
        ps.iter()
            .map(|p| {
                let px = shot.get_pixel(p.x as u32, p.y as u32).0;
                [px[0], px[1], px[2]]
            })
            .collect(),
    )
}

fn paste(s: &mut Session, r: fp_geom::IRect, c: [u8; 4]) -> u64 {
    let img = fp_raster::RgbaImage::filled(r, fp_color::Rgba8::new(c[0], c[1], c[2], c[3]))
        .expect("image");
    s.clipboard.clear();
    s.clipboard.pixels = Some(img);
    s.execute("edit.paste", json!({})).expect("bitmap").value["ids"][0]
        .as_u64()
        .expect("id")
}

/// Blue bitmap 20..180 masked by a bitmap 60..140 (white, opaque) in `mode`.
fn page(mode: &str) -> (Session, u64) {
    let mut s = Session::default();
    s.execute(
        "file.new",
        json!({"width": 200, "height": 200, "background": "#ffffff"}),
    )
    .expect("document");
    let c = paste(
        &mut s,
        fp_geom::IRect::new(20, 20, 180, 180),
        [0, 0, 255, 255],
    );
    let m = paste(
        &mut s,
        fp_geom::IRect::new(60, 60, 140, 140),
        [255, 255, 255, 255],
    );
    s.execute("select.set", json!({"ids": [c, m]}))
        .expect("select");
    let g = s
        .execute("modify.mask.groupAsMask", json!({}))
        .expect("mask")
        .value;
    let g = g["id"]
        .as_u64()
        .or_else(|| g["ids"][0].as_u64())
        .expect("group");
    s.execute("select.set", json!({"ids": [g]}))
        .expect("select group");
    s.execute("panel.layers.editMask", json!({}))
        .expect("edit mask");
    s.execute("modify.mask.options", json!({"mode": mode}))
        .expect("mode");
    let _ = s.execute("modify.exitBitmapMode", json!({}));
    s.execute("select.set", json!({"ids": []}))
        .expect("deselect");
    (s, g)
}

const PTS: [(f32, f32); 3] = [(100.0, 100.0), (70.0, 130.0), (30.0, 30.0)];

#[test]
fn selected_bitmap_mask_keeps_the_masked_object_fb90() {
    for mode in ["path", "image"] {
        let (s, g) = page(mode);
        let mut h = harness(s);
        let Some(before) = rgb(&mut h, &PTS) else {
            return;
        };
        assert!(
            before[0][2] > 200 && before[0][0] < 50,
            "{mode}: blue inside the mask: {before:?}"
        );
        ex(&mut h, "select.set", json!({"ids": [g]}));
        ex(&mut h, "panel.layers.editMask", json!({}));
        assert!(
            h.state()
                .state
                .session
                .active()
                .is_some_and(|d| d.bitmap_mask_edit().is_some()),
            "{mode}: mask in bitmap editing"
        );
        let during = rgb(&mut h, &PTS).expect("image");
        assert_eq!(during, before, "{mode}: mask selected (Edit Mask)");
        ex(&mut h, "select.set", json!({"ids": []}));
        let after = rgb(&mut h, &PTS).expect("image");
        assert_eq!(after, before, "{mode}: after deselecting");
    }
}

/// Blue bitmap 60..140 pasted inside the red bitmap 20..180 (alpha channel, "Show fill and
/// stroke" on: the red mask bitmap is the underlay where the content has no pixels).
fn paste_inside_page() -> (Session, u64) {
    let mut s = Session::default();
    s.execute(
        "file.new",
        json!({"width": 200, "height": 200, "background": "#ffffff"}),
    )
    .expect("document");
    let target = paste(
        &mut s,
        fp_geom::IRect::new(20, 20, 180, 180),
        [255, 0, 0, 255],
    );
    let content = paste(
        &mut s,
        fp_geom::IRect::new(60, 60, 140, 140),
        [0, 0, 255, 255],
    );
    s.execute("select.set", json!({"ids": [content]}))
        .expect("select");
    s.execute("edit.cut", json!({})).expect("cut");
    s.execute("select.set", json!({"ids": [target]}))
        .expect("select");
    let g = s
        .execute("edit.pasteInside", json!({}))
        .expect("paste inside")
        .value;
    let g = g["id"]
        .as_u64()
        .or_else(|| g["ids"][0].as_u64())
        .expect("group");
    s.execute("select.set", json!({"ids": []}))
        .expect("deselect");
    (s, g)
}

#[test]
fn paste_inside_underlay_stays_while_the_mask_is_edited_fb90() {
    let (s, g) = paste_inside_page();
    let mut h = harness(s);
    let pts = [(100.0, 100.0), (30.0, 30.0), (10.0, 10.0)];
    let Some(before) = rgb(&mut h, &pts) else {
        return;
    };
    assert!(
        before[0][2] > 200 && before[0][0] < 50,
        "blue content inside: {before:?}"
    );
    assert!(
        before[1][0] > 200 && before[1][2] < 50,
        "red underlay where the content has no pixels: {before:?}"
    );
    ex(&mut h, "select.set", json!({"ids": [g]}));
    ex(&mut h, "panel.layers.editMask", json!({}));
    assert!(
        h.state()
            .state
            .session
            .active()
            .is_some_and(|d| d.bitmap_mask_edit().is_some()),
        "mask in bitmap editing"
    );
    let during = rgb(&mut h, &pts).expect("image");
    assert_eq!(
        during, before,
        "the underlay stays while the mask is edited"
    );
    let _ = h
        .state_mut()
        .state
        .session
        .execute("modify.exitBitmapMode", json!({}));
    ex(&mut h, "select.set", json!({"ids": []}));
    let after = rgb(&mut h, &pts).expect("image");
    assert_eq!(after, before, "after the edit");
}
