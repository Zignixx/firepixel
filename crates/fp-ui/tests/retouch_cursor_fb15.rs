//! RE spec 57 section 2 (TW-181 rest) and FB-15: the retouch tools show the tip outline only while
//! the pointer is over the canvas, hide the system cursor over an editable bitmap and show "not
//! allowed" (and no outline) where no bitmap can be edited; the smudge inspector has the Firepixel
//! option "Confine to boundaries" (`SmudgeToolConfineToBounds`), read and written by the checkbox.

use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use fp_engine::Session;
use fp_engine::tools::{Cursor, Overlay, ToolId};
use fp_geom::IRect;
use fp_ui::FirepixelApp;
use fp_ui::app_state::{AppState, UiSettings};
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

/// Document 600 x 400; with `bitmap` a grey bitmap 50/50 - 300/300 (selected after pasting).
fn app(bitmap: bool) -> FirepixelApp {
    let mut s = Session::default();
    s.execute(
        "file.new",
        json!({"width": 600, "height": 400, "background": "#ffffff"}),
    )
    .expect("document");
    if bitmap {
        let img = fp_raster::RgbaImage::filled(
            IRect::new(50, 50, 300, 300),
            fp_color::Rgba8::new(100, 100, 100, 255),
        )
        .expect("image");
        s.clipboard.clear();
        s.clipboard.pixels = Some(img);
        s.execute("edit.paste", json!({})).expect("bitmap");
    }
    let ctx = egui::Context::default();
    let mut a = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    a.state.sync_render = true;
    a
}

fn harness(a: FirepixelApp) -> H {
    Harness::builder()
        .renderer(crate::gpu_common::renderer())
        .with_size(egui::vec2(1400.0, 900.0))
        .with_pixels_per_point(1.0)
        .with_max_steps(16)
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), a)
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn screen(h: &H, x: f32, y: f32) -> Pos2 {
    let s = st(h);
    let d = s.session.active().expect("document");
    let pr = s
        .canvases
        .get(&d.id)
        .and_then(|c| c.page_screen)
        .expect("page");
    let r = fp_render::Renderer::page_rect(&d.doc, d.page).expect("page rect");
    let (w, ht) = ((r.right - r.left) as f32, (r.bottom - r.top) as f32);
    egui::pos2(
        pr.left() + (x - r.left as f32) / w * pr.width(),
        pr.top() + (y - r.top as f32) / ht * pr.height(),
    )
}

/// Tip outlines the canvas painted in the last frame (`BrushTip` in the drawn feedback).
fn tips(h: &H) -> usize {
    let s = st(h);
    let d = s.session.active().expect("document");
    s.canvases
        .get(&d.id)
        .map(|c| {
            c.feedback
                .iter()
                .filter(|o| matches!(o, Overlay::BrushTip { .. }))
                .count()
        })
        .unwrap_or(0)
}

/// Circle outlines of the active retouch tip (its diameter at 100 %) painted in the last frame.
fn tip_circles(h: &H) -> usize {
    fn count(s: &egui::Shape, r: f32) -> usize {
        match s {
            egui::Shape::Circle(c) if (c.radius - r).abs() < 0.6 && c.fill.a() == 0 => 1,
            egui::Shape::Vec(v) => v.iter().map(|s| count(s, r)).sum(),
            _ => 0,
        }
    }
    let s = st(h);
    let key = match s.session.tools.active {
        ToolId::Blur => "BlurToolDiameter",
        ToolId::Dodge => "DodgeToolDiameter",
        _ => "SmudgeToolDiameter",
    };
    let r = s.session.prefs.get_int(key, 13) as f32 / 2.0;
    h.output().shapes.iter().map(|cs| count(&cs.shape, r)).sum()
}

fn hover(h: &mut H, p: Pos2) {
    h.event(Event::PointerMoved(p));
    h.run_steps(2);
}

#[test]
fn tip_outline_only_over_the_canvas_and_cursor_per_target() {
    for tool in [ToolId::Blur, ToolId::Smudge, ToolId::Dodge] {
        let mut h = harness(app(true));
        h.run_steps(3);
        h.state_mut().state.session.set_tool(tool).expect("tool");
        h.run_steps(2);
        // over the selected bitmap: tip outline, no system cursor
        let p = screen(&h, 150.0, 150.0);
        hover(&mut h, p);
        assert_eq!(tips(&h), 1, "{tool:?}: tip over the bitmap");
        assert_eq!(tip_circles(&h), 1, "{tool:?}: one tip circle painted");
        assert_eq!(st(&h).tool_cursor, Cursor::Hidden, "{tool:?}: cursor hidden");
        // next to the selected bitmap: still editable (the stroke would act on it)
        let p = screen(&h, 450.0, 350.0);
        hover(&mut h, p);
        assert_eq!(tips(&h), 1, "{tool:?}: tip next to the selected bitmap");
        // the pointer leaves the canvas (onto the tools panel at the left edge): outline gone
        hover(&mut h, egui::pos2(8.0, 450.0));
        assert_eq!(tips(&h), 0, "{tool:?}: no tip outside the canvas");
        assert_eq!(tip_circles(&h), 0, "{tool:?}: no tip circle outside the canvas");
        // the pointer leaves the window
        let p = screen(&h, 150.0, 150.0);
        hover(&mut h, p);
        assert_eq!(tips(&h), 1);
        h.event(Event::PointerGone);
        h.run_steps(2);
        assert_eq!(tips(&h), 0, "{tool:?}: no tip after the pointer left the window");
        // a stroke dragged off the canvas: no outline while outside, the stroke still applies
        let n0 = st(&h).session.active().map(|d| d.history.undo_count()).unwrap_or(0);
        let p0 = screen(&h, 200.0, 200.0);
        hover(&mut h, p0);
        h.event(Event::PointerButton {
            pos: p0,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        });
        h.run_steps(1);
        for i in 1..=10 {
            let p = screen(&h, 200.0 + i as f32 * 8.0, 200.0);
            hover(&mut h, p);
        }
        assert_eq!(tips(&h), 1, "{tool:?}: tip while dragging over the canvas");
        let out = egui::pos2(8.0, 450.0);
        hover(&mut h, out);
        assert_eq!(tips(&h), 0, "{tool:?}: no tip while dragging outside the canvas");
        h.event(Event::PointerButton {
            pos: out,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        });
        h.run_steps(2);
        let n1 = st(&h).session.active().map(|d| d.history.undo_count()).unwrap_or(0);
        assert_eq!(n1, n0 + 1, "{tool:?}: the stroke is one step");
    }
}

#[test]
fn not_allowed_and_no_outline_without_an_editable_bitmap() {
    let mut h = harness(app(false));
    h.run_steps(3);
    h.state_mut().state.session.set_tool(ToolId::Smudge).expect("tool");
    h.run_steps(2);
    let p = screen(&h, 150.0, 150.0);
    hover(&mut h, p);
    assert_eq!(st(&h).tool_cursor, Cursor::NotAllowed);
    assert_eq!(tips(&h), 0, "no tip without a bitmap");
    assert_eq!(tip_circles(&h), 0);
    // a stroke does nothing and writes no step (and no message)
    let n0 = st(&h).session.active().map(|d| d.history.undo_count()).unwrap_or(0);
    h.event(Event::PointerButton {
        pos: p,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    h.run_steps(1);
    let pe = screen(&h, 250.0, 160.0);
    hover(&mut h, pe);
    h.event(Event::PointerButton {
        pos: pe,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    h.run_steps(2);
    let n1 = st(&h).session.active().map(|d| d.history.undo_count()).unwrap_or(0);
    assert_eq!(n1, n0, "no step");
}

#[test]
fn smudge_inspector_confine_to_boundaries_reads_and_writes_the_pref() {
    let mut h = harness(app(true));
    h.run_steps(3);
    h.state_mut().state.session.set_tool(ToolId::Smudge).expect("tool");
    h.run_steps(3);
    let label = fp_i18n::t("pi-confine-bounds");
    let pref = |h: &H| {
        st(h)
            .session
            .prefs
            .get_bool("SmudgeToolConfineToBounds", false)
    };
    assert!(!pref(&h), "default off (the original's behaviour)");
    let node = h.get_by_label(&label);
    assert_eq!(node.accesskit_node().toggled(), Some(egui::accesskit::Toggled::False));
    let c = node.rect().center();
    let click = |h: &mut H, c: Pos2| {
        h.event(Event::PointerMoved(c));
        h.event(Event::PointerButton {
            pos: c,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        });
        h.event(Event::PointerButton {
            pos: c,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        });
        h.run_steps(3);
    };
    click(&mut h, c);
    assert!(pref(&h), "the checkbox writes SmudgeToolConfineToBounds");
    assert_eq!(
        h.get_by_label(&label).accesskit_node().toggled(),
        Some(egui::accesskit::Toggled::True)
    );
    // the pref reaches the stroke: a smudge across the bitmap edge keeps the bitmap size
    let p0 = screen(&h, 280.0, 150.0);
    h.event(Event::PointerMoved(p0));
    h.event(Event::PointerButton {
        pos: p0,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    h.run_steps(1);
    for i in 1..=15 {
        let p = screen(&h, 280.0 + i as f32 * 4.0, 150.0);
        h.event(Event::PointerMoved(p));
        h.run_steps(1);
    }
    let pe = screen(&h, 340.0, 150.0);
    h.event(Event::PointerButton {
        pos: pe,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    h.run_steps(2);
    let right = |h: &H| {
        let d = st(h).session.active().expect("document");
        fp_engine::docops::own_layers(&d.doc, d.page)
            .into_iter()
            .flat_map(|(_, l)| l.cels[0].elements.clone())
            .find_map(|e| match &e.kind {
                fp_model::ElementKind::Bitmap(b) => {
                    fp_engine::docops::bitmap_image(b).map(|i| i.bounds().right)
                }
                _ => None,
            })
            .unwrap_or(0)
    };
    assert_eq!(right(&h), 300, "confined: the bitmap keeps its edge");
    // a pref set elsewhere (e.g. over MCP) shows in the checkbox
    h.state_mut()
        .state
        .session
        .prefs
        .set("SmudgeToolConfineToBounds", fp_engine::PrefValue::Bool(false));
    h.state_mut().state.session.set_tool(ToolId::Blur).expect("tool");
    h.run_steps(2);
    h.state_mut().state.session.set_tool(ToolId::Smudge).expect("tool");
    h.run_steps(3);
    assert_eq!(
        h.get_by_label(&label).accesskit_node().toggled(),
        Some(egui::accesskit::Toggled::False)
    );
}
