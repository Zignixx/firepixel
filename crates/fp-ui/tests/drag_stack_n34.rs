//! N34 (Nutzerbefund 15.10.): Beim Ziehen bleibt das gezogene Objekt in seiner Stapelfolge – ein
//! graues Rechteck unter einem grünen wird während des Ziehens weiter vom grünen verdeckt (vorher lag
//! die Zieh-Vorschau über allem). Dasselbe für die Malvorschau (BA-039): Objekte über der bemalten
//! Bitmap verdecken den Strich schon während des Malens. Pixelprüfung im gerenderten Fenster (wgpu).

use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use fp_engine::tools::ToolId;
use fp_engine::Session;
use fp_ui::app_state::UiSettings;
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn harness(s: Session) -> H {
    let ctx = egui::Context::default();
    let mut a = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    a.state.os_clipboard = false;
    a.state.sync_render = true;
    let mut h = Harness::builder().renderer(crate::gpu_common::renderer()).with_size(egui::vec2(1400.0, 900.0)).with_pixels_per_point(1.0).with_max_steps(8).renderer(crate::gpu_common::renderer()).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), a);
    h.run_steps(4);
    h
}

fn at(h: &H, x: f32, y: f32) -> Pos2 {
    let s = &h.state().state;
    let d = s.session.active().expect("Dokument");
    let pr = s.canvases.get(&d.id).and_then(|c| c.page_screen).expect("Seitenlage");
    let r = fp_render::Renderer::page_rect(&d.doc, d.page).expect("Seite");
    let (w, ht) = ((r.right - r.left) as f32, (r.bottom - r.top) as f32);
    egui::pos2(pr.left() + (x - r.left as f32) / w * pr.width(), pr.top() + (y - r.top as f32) / ht * pr.height())
}

fn ev(h: &mut H, e: Event) {
    h.input_mut().events.push(e);
    h.step();
}

fn button(h: &mut H, p: Pos2, pressed: bool) {
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
}

fn set_tool(h: &mut H, t: ToolId) {
    let _ = h.state_mut().state.session.set_tool(t);
    h.step();
}

/// RGB an einem Dokumentpunkt im gerenderten Fenster; `None` ohne Grafikadapter.
fn rgb(h: &mut H, x: f32, y: f32) -> Option<[u8; 3]> {
    let p = at(h, x, y);
    let shot = match h.render() {
        Ok(i) => i,
        Err(e) => {
            eprintln!("kein Grafikadapter, Bildprüfung übersprungen: {e}");
            return None;
        }
    };
    let px = shot.get_pixel(p.x as u32, p.y as u32).0;
    Some([px[0], px[1], px[2]])
}

fn near(a: [u8; 3], b: [u8; 3]) -> bool {
    a.iter().zip(b).all(|(x, y)| (*x as i32 - y as i32).abs() <= 2)
}

const GRAY: [u8; 3] = [128, 128, 128];
const GREEN: [u8; 3] = [0, 255, 0];
const WHITE: [u8; 3] = [255, 255, 255];

/// Graues Rechteck (unten) 50..150, grünes (oben) 160..260; beide ohne Strich.
fn gray_under_green() -> Session {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 400, "height": 300, "background": "#ffffff"})).expect("Dokument");
    s.execute("draw.rectangle", json!({"rect": [50, 50, 150, 150], "fillColor": "#808080", "strokeWidth": 0})).expect("grau");
    s.execute("draw.rectangle", json!({"rect": [160, 50, 260, 150], "fillColor": "#00ff00", "strokeWidth": 0})).expect("grün");
    s.execute("select.none", json!({})).expect("nichts");
    s
}

#[test]
fn dragged_object_stays_below_object_above_n34() {
    let mut h = harness(gray_under_green());
    set_tool(&mut h, ToolId::Pointer);
    // Grau am linken Rand greifen und 60 px nach rechts ziehen: 110..210 überlappt Grün 160..210
    let (a, b) = (at(&h, 70.0, 100.0), at(&h, 130.0, 100.0));
    ev(&mut h, Event::PointerMoved(a));
    button(&mut h, a, true);
    ev(&mut h, Event::PointerMoved(a + egui::vec2(10.0, 0.0)));
    ev(&mut h, Event::PointerMoved(b));
    h.step();
    {
        let st = &h.state().state;
        let cc = st.canvases.values().next().expect("Leinwand");
        assert!(cc.shown.is_some_and(|k| k.variant != 0), "Hintergrundbild ohne Grau und Grün");
        assert!(cc.layers.iter().any(Option::is_some), "Schicht mit dem grünen Rechteck über der Vorschau");
    }
    let Some(overlap) = rgb(&mut h, 185.0, 80.0) else { return };
    assert!(near(overlap, GREEN), "Überlappung während des Ziehens: Grün liegt oben, war {overlap:?}");
    let moved = rgb(&mut h, 130.0, 80.0).expect("Bild");
    assert!(near(moved, GRAY), "Vorschau des grauen Rechtecks sichtbar, war {moved:?}");
    let old = rgb(&mut h, 60.0, 80.0).expect("Bild");
    assert!(near(old, WHITE), "Original ausgeblendet, war {old:?}");
    let green_only = rgb(&mut h, 240.0, 80.0).expect("Bild");
    assert!(near(green_only, GREEN), "{green_only:?}");
    button(&mut h, b, false);
    h.run_steps(3);
    let after = rgb(&mut h, 185.0, 80.0).expect("Bild");
    assert!(near(after, GREEN), "nach dem Loslassen unverändert, war {after:?}");
    let cc = h.state().state.canvases.values().next().expect("Leinwand");
    assert!(cc.shown.is_some_and(|k| k.variant == 0) && cc.layers.is_empty(), "danach wieder ein Bild ohne Schichten");
}

#[test]
fn top_object_needs_no_extra_layer_n34() {
    let mut h = harness(gray_under_green());
    set_tool(&mut h, ToolId::Pointer);
    // Grün (oben) über Grau ziehen: Vorschau oben, keine zusätzliche Schicht
    let (a, b) = (at(&h, 240.0, 100.0), at(&h, 180.0, 100.0));
    ev(&mut h, Event::PointerMoved(a));
    button(&mut h, a, true);
    ev(&mut h, Event::PointerMoved(a - egui::vec2(10.0, 0.0)));
    ev(&mut h, Event::PointerMoved(b));
    h.step();
    assert!(h.state().state.canvases.values().next().expect("Leinwand").layers.iter().all(Option::is_none));
    let Some(overlap) = rgb(&mut h, 130.0, 80.0) else { return };
    assert!(near(overlap, GREEN), "Grün liegt oben, war {overlap:?}");
    button(&mut h, b, false);
}

#[test]
fn alt_copy_preview_lies_directly_above_original_n34() {
    let mut h = harness(gray_under_green());
    set_tool(&mut h, ToolId::Pointer);
    let (a, b) = (at(&h, 70.0, 100.0), at(&h, 130.0, 100.0));
    ev(&mut h, Event::PointerMoved(a));
    ev(&mut h, Event::PointerButton { pos: a, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::ALT });
    ev(&mut h, Event::PointerMoved(a + egui::vec2(10.0, 0.0)));
    ev(&mut h, Event::PointerMoved(b));
    h.step();
    let Some(overlap) = rgb(&mut h, 185.0, 80.0) else { return };
    assert!(near(overlap, GREEN), "Kopie liegt unter Grün, war {overlap:?}");
    let orig = rgb(&mut h, 60.0, 80.0).expect("Bild");
    assert!(near(orig, GRAY), "Original bleibt stehen, war {orig:?}");
    ev(&mut h, Event::PointerButton { pos: b, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::ALT });
    h.run_steps(3);
    let after = rgb(&mut h, 185.0, 80.0).expect("Bild");
    assert!(near(after, GREEN), "Duplikat direkt über dem Original, unter Grün: {after:?}");
}

#[test]
fn paint_preview_stays_below_object_above_ba039() {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 400, "height": 300, "background": "#ffffff"})).expect("Dokument");
    let img = fp_raster::RgbaImage::filled(fp_geom::IRect::new(40, 40, 300, 200), fp_color::Rgba8::new(255, 255, 255, 255)).expect("Bild");
    s.clipboard.clear();
    s.clipboard.pixels = Some(img);
    let v = s.execute("edit.paste", json!({})).expect("Bitmap");
    let bmp = v.value["ids"][0].as_u64().expect("id");
    s.execute("draw.rectangle", json!({"rect": [160, 50, 260, 150], "fillColor": "#00ff00", "strokeWidth": 0})).expect("grün");
    s.execute("select.set", json!({"ids": [bmp]})).expect("Bitmap wählen");
    s.set_colors(Some(fp_color::Argb(0xff00_0000)), None);
    let mut h = harness(s);
    set_tool(&mut h, ToolId::Brush);
    let pts: Vec<Pos2> = (0..=20).map(|i| at(&h, 60.0 + i as f32 * 10.0, 100.0)).collect();
    ev(&mut h, Event::PointerMoved(pts[0]));
    button(&mut h, pts[0], true);
    for p in &pts[1..] {
        ev(&mut h, Event::PointerMoved(*p));
    }
    h.step();
    let Some(stroke) = rgb(&mut h, 100.0, 100.0) else { return };
    assert!(stroke[0] < 80, "Strich außerhalb von Grün sichtbar, war {stroke:?}");
    let under = rgb(&mut h, 210.0, 100.0).expect("Bild");
    assert!(near(under, GREEN), "Strich unter Grün verdeckt, war {under:?}");
    button(&mut h, *pts.last().expect("Punkt"), false);
    h.run_steps(3);
    let after = rgb(&mut h, 210.0, 100.0).expect("Bild");
    assert!(near(after, GREEN), "nach dem Loslassen gleich, war {after:?}");
}
