//! BM-1 (Nutzerbefund N26, re\22 §1.2 Regel 3): Der Auswahlrahmen einer Bitmap liegt um die
//! sichtbaren Pixel (Alpha ≥ 2), nicht um die Objektgrenzen; Malen außerhalb lässt ihn wachsen,
//! Rückgängig stellt ihn wieder her.

use fp_engine::tools::ToolId;
use fp_engine::{Session, ToolEvent};
use fp_geom::IRect;
use fp_model::ElementId;
use fp_ui::app_state::UiSettings;
use fp_ui::FirepixelApp;
use serde_json::json;

/// 400×300-Dokument mit einer Bitmap über die ganze Leinwand, sichtbar nur x 100…200, y 50…150.
fn app() -> (FirepixelApp, ElementId) {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 400, "height": 300, "background": "#ffffff"})).expect("Dokument");
    let ctx = egui::Context::default();
    let mut a = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    a.state.sync_render = true;
    a.state.os_clipboard = false;
    let s = &mut a.state.session;
    let mut img = fp_raster::RgbaImage::new(IRect::new(0, 0, 400, 300)).expect("Bild");
    img.fill_rect(IRect::new(100, 50, 200, 150), fp_color::Rgba8::new(255, 0, 0, 255));
    // Alpha 1 zählt nicht
    img.set_pixel(5, 5, fp_color::Rgba8::new(0, 255, 0, 1));
    s.clipboard.clear();
    s.clipboard.pixels = Some(img);
    let v = s.execute("edit.paste", json!({})).expect("Bitmap");
    let id = ElementId(v.value["ids"][0].as_u64().expect("id"));
    (a, id)
}

fn frame(h: &egui_kittest::Harness<'_, FirepixelApp>) -> Option<[f32; 4]> {
    let c = h.state().state.canvases.values().next()?;
    let f = c.sel_frames.first()?;
    let (xs, ys) = (f.map(|p| p.x), f.map(|p| p.y));
    Some([xs.iter().cloned().fold(f32::MAX, f32::min), ys.iter().cloned().fold(f32::MAX, f32::min), xs.iter().cloned().fold(f32::MIN, f32::max), ys.iter().cloned().fold(f32::MIN, f32::max)])
}

#[test]
fn auswahlrahmen_liegt_um_die_sichtbaren_pixel() {
    let (a, id) = app();
    let mut h = egui_kittest::Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .with_pixels_per_point(1.0)
        .with_max_steps(16)
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), a);
    h.state_mut().state.session.execute("select.set", json!({"ids": [id.0]})).expect("Auswahl");
    h.run_steps(2);
    {
        let d = h.state().state.session.active().expect("d");
        let e = fp_engine::docops::element_on_page(&d.doc, d.page, id).expect("Element");
        let b = fp_engine::docops::element_bounds(&d.doc, &e).expect("Grenzen");
        assert_eq!((b.left, b.top, b.right, b.bottom), (0.0, 0.0, 400.0, 300.0), "Objektgrenzen bleiben");
    }
    assert_eq!(frame(&h), Some([100.0, 50.0, 200.0, 150.0]));
    // Inspektor/Info zeigen weiter die Objektgrenzen (X/Y/B/H)
    let ib = {
        let d = h.state().state.session.active().expect("d");
        fp_engine::docops::ids_bounds(&d.doc, d.page, &d.selection.elements).expect("Auswahlgrenzen")
    };
    assert_eq!((ib.left, ib.top, ib.width(), ib.height()), (0.0, 0.0, 400.0, 300.0));

    // Malen außerhalb des sichtbaren Teils: Rahmen wächst mit (Bleistift in die gewählte Bitmap)
    {
        let s = &mut h.state_mut().state.session;
        s.set_colors(Some(fp_color::Argb(0xff00_00ff)), None);
        s.set_tool(ToolId::Pencil).expect("Bleistift");
        s.tool_event(&ToolEvent::down(320.5, 260.5)).expect("ab");
        s.tool_event(&ToolEvent::up(320.5, 260.5)).expect("auf");
        s.set_tool(ToolId::Pointer).expect("Zeiger");
        s.execute("select.set", json!({"ids": [id.0]})).expect("Auswahl");
    }
    h.run_steps(2);
    let f = frame(&h).expect("Rahmen");
    assert_eq!((f[0], f[1]), (100.0, 50.0));
    assert!(f[2] >= 321.0 && f[3] >= 261.0, "Rahmen umfasst das Gemalte: {f:?}");
    // Rückgängig: wieder um das Quadrat
    h.state_mut().state.session.execute("edit.undo", json!({})).expect("Rückgängig");
    h.state_mut().state.session.execute("select.set", json!({"ids": [id.0]})).expect("Auswahl");
    h.run_steps(2);
    assert_eq!(frame(&h), Some([100.0, 50.0, 200.0, 150.0]));
    // viele Bilder hintereinander: gleicher Rahmen (Zwischenspeicher)
    h.run_steps(10);
    assert_eq!(frame(&h), Some([100.0, 50.0, 200.0, 150.0]));
}
