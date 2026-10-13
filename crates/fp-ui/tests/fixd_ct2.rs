//! FIX-D: Befunde aus dem Klicktest CT-2 (`docs\funktionstest\ct2_2026-10-13.md`) in der Oberfläche
//! mit echten Zeigerereignissen.

use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use fp_engine::tools::ToolId;
use fp_engine::Session;
use fp_geom::IRect;
use fp_model::{ElementId, ElementKind};
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn app_with_bitmap(rgba: [u8; 4]) -> (FirepixelApp, ElementId) {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 600, "height": 400, "background": "#ffffff"})).expect("Dokument");
    let img = fp_raster::RgbaImage::filled(IRect::new(50, 50, 550, 350), fp_color::Rgba8::new(rgba[0], rgba[1], rgba[2], rgba[3])).expect("Bild");
    s.clipboard.clear();
    s.clipboard.pixels = Some(img);
    let v = s.execute("edit.paste", json!({})).expect("Bitmap");
    let id = ElementId(v.value["ids"][0].as_u64().expect("id"));
    let ctx = egui::Context::default();
    let mut a = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    a.state.sync_render = true;
    (a, id)
}

fn harness(a: FirepixelApp) -> H {
    Harness::builder().with_size(egui::vec2(1400.0, 900.0)).with_pixels_per_point(1.0).with_max_steps(16).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), a)
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn image(h: &H, id: ElementId) -> fp_raster::RgbaImage {
    let d = st(h).session.active().expect("Dokument");
    match &fp_engine::docops::element_on_page(&d.doc, d.page, id).expect("Element").kind {
        ElementKind::Bitmap(b) => fp_engine::docops::bitmap_image(b).expect("Bild"),
        _ => panic!("keine Bitmap"),
    }
}

/// Dokumentpunkt → Bildschirm (Seitenlage aus dem letzten Bild).
fn screen(h: &H, x: f32, y: f32) -> Pos2 {
    let s = st(h);
    let d = s.session.active().expect("Dokument");
    let pr = s.canvases.get(&d.id).and_then(|c| c.page_screen).expect("Seitenlage");
    let r = fp_render::Renderer::page_rect(&d.doc, d.page).expect("Seite");
    let (w, ht) = ((r.right - r.left) as f32, (r.bottom - r.top) as f32);
    egui::pos2(pr.left() + (x - r.left as f32) / w * pr.width(), pr.top() + (y - r.top as f32) / ht * pr.height())
}

/// Zieht mit echten Zeigerereignissen über viele Bilder (je Bild ≈ 4 px).
fn mouse_drag(h: &mut H, a: (f32, f32), b: (f32, f32)) {
    let p0 = screen(h, a.0, a.1);
    h.event(Event::PointerMoved(p0));
    h.run_steps(1);
    h.event(Event::PointerButton { pos: p0, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.run_steps(1);
    let len = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
    let k = (len / 25.0).ceil().max(1.0) as usize;
    let mut last = p0;
    for i in 1..=k {
        let t = i as f32 / k as f32;
        last = screen(h, a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
        h.event(Event::PointerMoved(last));
        h.run_steps(1);
    }
    h.event(Event::PointerButton { pos: last, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
}

/// CT-2 W8: Farbe ersetzen (Größe 13, Toleranz 32, Stärke 200, Grün → Blau) auf grüner Bitmap –
/// beim Ziehen mit der Maus wird der ganze Strich blau, nicht nur der erste Stempel.
#[test]
fn replace_color_mouse_drag_replaces_whole_stroke() {
    use fp_engine::prefs::PrefValue::{Bool, Int, Str};
    let (a, id) = app_with_bitmap([0, 255, 0, 255]);
    let mut h = harness(a);
    {
        let s = &mut h.state_mut().state.session;
        for (k, v) in [
            ("ReplaceColorToolSrcColor", Str("#00FF00".into())),
            ("ReplaceColorToolDestColor", Str("#0000FF".into())),
            ("ReplaceColorToolTolerance", Int(32)),
            ("ReplaceColorToolStrength", Int(200)),
            ("ReplaceColorToolDiameter", Int(13)),
            ("ReplaceColorToolColorize", Bool(false)),
            ("ReplaceColorToolMode", Bool(false)),
        ] {
            s.prefs.set(k, v);
        }
        s.set_tool(ToolId::ReplaceColor).expect("Werkzeug");
    }
    h.run_steps(3);
    mouse_drag(&mut h, (100.0, 200.0), (400.0, 200.0));
    let img = image(&h, id);
    for x in [100, 160, 250, 330, 398] {
        let p = img.get(x, 200).expect("Pixel");
        assert!(p[1] < 10 && p[2] > 240, "x {x}: {p:?} statt blau");
    }
}
