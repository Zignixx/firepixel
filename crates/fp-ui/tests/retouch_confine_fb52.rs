//! FB-52 (Firepixel extension): the inspector of Blur, Rubber Stamp, Brush and Pencil has the
//! checkbox "Confine to boundaries" like the smudge (Q5, default off). It reads and writes the
//! preference of the active tool (`<Tool>ConfineToBounds`), each tool keeps its own value. Sharpen,
//! Dodge and Burn (they work only inside the bitmap), Replace Color and Red Eye have none.

use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use fp_engine::Session;
use fp_engine::tools::ToolId;
use fp_geom::IRect;
use fp_ui::FirepixelApp;
use fp_ui::app_state::{AppState, UiSettings};
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn app() -> FirepixelApp {
    let mut s = Session::default();
    s.execute(
        "file.new",
        json!({"width": 600, "height": 400, "background": "#ffffff"}),
    )
    .expect("document");
    let img = fp_raster::RgbaImage::filled(
        IRect::new(50, 50, 300, 300),
        fp_color::Rgba8::new(100, 100, 100, 255),
    )
    .expect("image");
    s.clipboard.clear();
    s.clipboard.pixels = Some(img);
    s.execute("edit.paste", json!({})).expect("bitmap");
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

fn click(h: &mut H, c: Pos2) {
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
}

fn toggled(h: &H, label: &str) -> Option<egui::accesskit::Toggled> {
    h.get_by_label(label).accesskit_node().toggled()
}

fn set_tool(h: &mut H, t: ToolId) {
    h.state_mut().state.session.set_tool(t).expect("tool");
    h.run_steps(3);
}

const TOOLS: [(ToolId, &str); 4] = [
    (ToolId::Blur, "BlurToolConfineToBounds"),
    (ToolId::RubberStamp, "StampToolConfineToBounds"),
    (ToolId::Brush, "BrushToolConfineToBounds"),
    (ToolId::Pencil, "PencilToolConfineToBounds"),
];

#[test]
fn retouch_inspectors_confine_to_boundaries_read_and_write_the_tool_pref() {
    let mut h = harness(app());
    h.run_steps(3);
    let label = fp_i18n::t("pi-confine-bounds");
    let pref = |h: &H, k: &str| st(h).session.prefs.get_bool(k, false);
    for (tool, key) in TOOLS {
        set_tool(&mut h, tool);
        assert!(!pref(&h, key), "{tool:?}: default off");
        assert_eq!(
            toggled(&h, &label),
            Some(egui::accesskit::Toggled::False),
            "{tool:?}: the checkbox is shown, off"
        );
        let c = h.get_by_label(&label).rect().center();
        click(&mut h, c);
        assert!(pref(&h, key), "{tool:?}: the checkbox writes {key}");
        assert_eq!(
            toggled(&h, &label),
            Some(egui::accesskit::Toggled::True),
            "{tool:?}"
        );
    }
    // each tool keeps its own value: the smudge is still off, the blur still on
    set_tool(&mut h, ToolId::Smudge);
    assert_eq!(toggled(&h, &label), Some(egui::accesskit::Toggled::False));
    assert!(!pref(&h, "SmudgeToolConfineToBounds"));
    set_tool(&mut h, ToolId::Blur);
    assert_eq!(toggled(&h, &label), Some(egui::accesskit::Toggled::True));
    // a pref set elsewhere (e.g. over MCP) shows in the checkbox
    h.state_mut().state.session.prefs.set(
        "PencilToolConfineToBounds",
        fp_engine::PrefValue::Bool(false),
    );
    set_tool(&mut h, ToolId::Pencil);
    assert_eq!(toggled(&h, &label), Some(egui::accesskit::Toggled::False));
    // tools that never paint past the bitmap edge have no such option
    for tool in [
        ToolId::Sharpen,
        ToolId::Dodge,
        ToolId::Burn,
        ToolId::ReplaceColor,
        ToolId::RedEyeRemoval,
    ] {
        set_tool(&mut h, tool);
        assert!(
            h.query_by_label(&label).is_none(),
            "{tool:?}: no Confine to boundaries"
        );
    }
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

fn bitmap_right(h: &H) -> i32 {
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
}

/// Blur stroke down along the right bitmap edge (x 295, y 100 to 200) with the mouse.
fn blur_along_the_edge(h: &mut H) {
    let p0 = screen(h, 295.0, 100.0);
    h.event(Event::PointerMoved(p0));
    h.event(Event::PointerButton {
        pos: p0,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    h.run_steps(1);
    for i in 1..=20 {
        let p = screen(h, 295.0, 100.0 + i as f32 * 5.0);
        h.event(Event::PointerMoved(p));
        h.run_steps(1);
    }
    let pe = screen(h, 295.0, 200.0);
    h.event(Event::PointerButton {
        pos: pe,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    h.run_steps(2);
}

/// The checkbox of the blur reaches the stroke: without it a blur along the bitmap edge grows the
/// bitmap by up to 2 px, with it the bitmap keeps its size.
#[test]
fn blur_confine_checkbox_keeps_the_bitmap_size() {
    let mut h = harness(app());
    h.run_steps(3);
    set_tool(&mut h, ToolId::Blur);
    blur_along_the_edge(&mut h);
    assert!(bitmap_right(&h) > 300, "without the option the blur grows");
    h.state_mut()
        .state
        .session
        .execute("edit.undo", json!({}))
        .expect("undo");
    h.run_steps(3);
    assert_eq!(bitmap_right(&h), 300);
    let label = fp_i18n::t("pi-confine-bounds");
    let c = h.get_by_label(&label).rect().center();
    click(&mut h, c);
    blur_along_the_edge(&mut h);
    assert_eq!(
        bitmap_right(&h),
        300,
        "confined: the blur does not grow the bitmap"
    );
}
