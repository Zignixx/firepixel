//! TW-139: the canvas draws typed text at once while the text editor is open (run 19
//! `tool-text/06`: a canvas shot taken while editing showed no typed "Q"). Pixel check in the
//! rendered window (wgpu): after typing "Q", dark glyph pixels appear in the text area before the
//! edit is committed, and the document model still has no text object.

use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use fp_engine::Session;
use fp_engine::tools::ToolId;
use fp_model::ElementKind;
use fp_ui::FirepixelApp;
use fp_ui::app_state::UiSettings;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn harness() -> H {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 400, "height": 300, "background": "#ffffff"}))
        .expect("document");
    s.tools.options.text_size = 96.0;
    s.tools.options.text_color = fp_color::Argb(0xff00_0000);
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
    let pr = s.canvases.get(&d.id).and_then(|c| c.page_screen).expect("page position");
    let r = fp_render::Renderer::page_rect(&d.doc, d.page).expect("page");
    let (w, ht) = ((r.right - r.left) as f32, (r.bottom - r.top) as f32);
    egui::pos2(
        pr.left() + (x - r.left as f32) / w * pr.width(),
        pr.top() + (y - r.top as f32) / ht * pr.height(),
    )
}

/// Dark pixels in the document box `l..r × t..b` of the rendered window; `None` without a
/// graphics adapter.
fn dark_pixels(h: &mut H, (l, t, r, b): (f32, f32, f32, f32)) -> Option<usize> {
    let (p0, p1) = (at(h, l, t), at(h, r, b));
    let shot = match h.render() {
        Ok(i) => i,
        Err(e) => {
            eprintln!("no graphics adapter, pixel check skipped: {e}");
            return None;
        }
    };
    let mut n = 0;
    for y in p0.y.max(0.0) as u32..(p1.y as u32).min(shot.height()) {
        for x in p0.x.max(0.0) as u32..(p1.x as u32).min(shot.width()) {
            let px = shot.get_pixel(x, y).0;
            if px[0] < 90 && px[1] < 90 && px[2] < 90 {
                n += 1;
            }
        }
    }
    Some(n)
}

fn model_has_text(h: &H) -> bool {
    let d = h.state().state.session.active().expect("document");
    let mut found = false;
    d.doc.walk_elements(|_, _, e| found |= matches!(e.kind, ElementKind::Text(_)));
    found
}

#[test]
fn typed_glyph_is_drawn_while_editing_tw139() {
    let mut h = harness();
    let _ = h.state_mut().state.session.set_tool(ToolId::Text);
    h.step();
    // click at the first baseline (60, 150); the glyph lies above it
    let p = at(&h, 60.0, 150.0);
    for pressed in [true, false] {
        h.input_mut().events.push(Event::PointerMoved(p));
        h.input_mut().events.push(Event::PointerButton {
            pos: p,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
        h.step();
    }
    h.run_steps(2);
    assert!(h.state().state.session.tool_is_editing_text(), "text editor open");
    let area = (55.0, 60.0, 150.0, 175.0);
    let Some(before) = dark_pixels(&mut h, area) else { return };
    h.input_mut().events.push(Event::Text("Q".into()));
    h.run_steps(3);
    assert!(h.state().state.session.tool_is_editing_text(), "still editing");
    assert!(!model_has_text(&h), "not committed yet");
    let after = dark_pixels(&mut h, area).expect("render");
    eprintln!("dark pixels: {before} before, {after} after typing Q");
    assert!(after > before + 300, "typed Q drawn while editing: {before} → {after}");
}
