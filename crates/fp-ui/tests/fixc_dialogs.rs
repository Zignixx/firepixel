//! FIX-C (Befunde von CT-1a, Dialoge):
//! - D43 Kurven: gepunktetes Raster und Kurvenlinie werden sichtbar gezeichnet
//! - D42 Helligkeit/Kontrast: Pfeiltasten bewegen den fokussierten Schieber
//!
//! Realistische Eingaben: Tasten wie egui-winit (Drücken/Loslassen), Klicks über mehrere Bilder.

use egui::accesskit::Role;
use egui::{Color32, Event, Key, Modifiers, PointerButton, Pos2, Rect};
use egui_kittest::kittest::{By, NodeT, Queryable};
use egui_kittest::Harness;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::FirepixelApp;
use serde_json::{json, Value};

type H = Harness<'static, FirepixelApp>;

fn harness_with(s: fp_engine::Session) -> H {
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).with_pixels_per_point(1.0).with_max_steps(12).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(3);
    h
}

/// Dokument mit der Bitmap B1, ausgewählt (Filterdialoge).
fn bitmap_session() -> fp_engine::Session {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [40, 40, 140, 120], "fillColor": "#3366cc", "name": "B1"})).expect("rect");
    s.execute("modify.flattenSelection", json!({})).expect("reduzieren");
    s
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn ev(h: &mut H, e: Event) {
    h.event(e);
    h.run_steps(1);
}

fn key(h: &mut H, k: Key) {
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: Modifiers::NONE });
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: false, repeat: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
}

#[allow(dead_code)]
fn click(h: &mut H, p: Pos2) {
    ev(h, Event::PointerMoved(p));
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
}

fn open(h: &mut H, cmd: &str) {
    h.state_mut().state.run(cmd, json!({}));
    h.run_steps(4);
    assert!(st(h).dialogs.any_open(), "{cmd}: Dialog offen");
}

#[allow(dead_code)]
fn dialog_rect(h: &H) -> Rect {
    fp_ui::dialogs::dialog_rect(&h.ctx, &fp_ui::dialogs::last_dialog_id(&h.ctx).expect("Dialog")).expect("Rechteck")
}

fn focused(h: &H) -> Option<(Role, String)> {
    h.ctx.memory(|m| m.focused())?;
    h.get_all(By::new()).find(|n| n.accesskit_node().is_focused()).map(|n| {
        let node = n.accesskit_node();
        (node.role(), node.label().map(|s| s.to_string()).or_else(|| node.value().map(|s| s.to_string())).unwrap_or_default())
    })
}

fn form_value(h: &H, key: &str) -> Value {
    st(h).dialogs.form.as_ref().expect("Formular").get(key)
}

/// Einzelpixel (1 × 1) einer Farbe in den Netzen des letzten Bildes innerhalb von `r`.
fn mesh_pixels(h: &H, r: Rect, c: Color32) -> usize {
    let mut n = 0;
    for cs in &h.output().shapes {
        let egui::epaint::Shape::Mesh(m) = &cs.shape else { continue };
        for q in m.vertices.chunks(4) {
            if q.len() == 4 && q[0].color == c && r.contains(q[0].pos) {
                let b = Rect::from_points(&q.iter().map(|v| v.pos).collect::<Vec<_>>());
                if (b.width() - 1.0).abs() < 0.01 && (b.height() - 1.0).abs() < 0.01 {
                    n += 1;
                }
            }
        }
    }
    n
}

/// D43: Raster (3 + 3 Linien, jedes zweite Pixel) und Kurve (ein Pixel je Spalte) werden als
/// ungeglättete Einzelpixel gezeichnet – vorher als 1 × 1-Rechtecke, die der Tessellator zu
/// kaum sichtbaren Linienstücken macht.
#[test]
fn d43_curves_grid_and_line_visible() {
    let mut h = harness_with(bitmap_session());
    open(&mut h, "filter.adjustColor.curves");
    let r = fp_ui::dialogs::dialog_rect(&h.ctx, "fp-curves").expect("Kurven");
    let w = fp_ui::dialogs::curves::FIELD as usize;
    assert_eq!(mesh_pixels(&h, r, Color32::from_gray(0x80)), 3 * 2 * w / 2, "gepunktetes Raster 4 × 4");
    assert_eq!(mesh_pixels(&h, r, Color32::BLACK), w, "Kurve: ein Pixel je Spalte");
    key(&mut h, Key::Escape);
}

/// D42: Pfeiltasten verschieben den fokussierten Schieber; Wert und Feld folgen.
#[test]
fn d42_arrow_keys_move_focused_slider() {
    let mut h = harness_with(bitmap_session());
    open(&mut h, "filter.adjustColor.brightnessContrast");
    assert_eq!(focused(&h).map(|f| f.0), Some(Role::Slider), "Startfokus Schieber");
    for _ in 0..5 {
        key(&mut h, Key::ArrowRight);
    }
    assert_eq!(form_value(&h, "brightness").as_f64(), Some(5.0), "→ ×5");
    key(&mut h, Key::ArrowLeft);
    assert_eq!(form_value(&h, "brightness").as_f64(), Some(4.0), "←");
    assert_eq!(focused(&h).map(|f| f.0), Some(Role::Slider), "Fokus bleibt am Schieber");
    // per Tab erreichter zweiter Schieber
    key(&mut h, Key::Tab);
    key(&mut h, Key::Tab);
    assert_eq!(focused(&h).map(|f| f.0), Some(Role::Slider), "Kontrast-Schieber");
    key(&mut h, Key::ArrowLeft);
    key(&mut h, Key::ArrowLeft);
    assert_eq!(form_value(&h, "contrast").as_f64(), Some(-2.0), "← ×2 am Kontrast");
    // Klick auf den Schieber, dann Pfeil
    let sl: Vec<Rect> = h.get_all(By::new().role(Role::Slider)).map(|n| n.rect()).collect();
    click(&mut h, sl[0].center());
    let v = form_value(&h, "brightness").as_f64().expect("Wert");
    key(&mut h, Key::ArrowRight);
    assert_eq!(form_value(&h, "brightness").as_f64(), Some(v + 1.0), "→ nach Klick");
    key(&mut h, Key::Escape);
}
