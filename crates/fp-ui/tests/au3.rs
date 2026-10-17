//! AU-3 (von FV-16): „Andere…“ in der Texturliste des Werkzeug-Inspektors merkt den vollen Pfad
//! (Anzeige weiter der Dateiname ohne Endung). Eine Datei außerhalb der Texturordner wirkt so auch
//! an neu gezeichneten Objekten (Maske vorhanden). Realistische Eingaben: echte Klicks und ein
//! Ziehen auf der Leinwand, je Ereignis ein Bild.

use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::{By, Queryable};
use egui_kittest::Harness;
use fp_engine::ToolId;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::inspector::obj_quads::{self, DEFAULT_TEXTURE};
use fp_ui::FirepixelApp;
use serde_json::json;
use std::path::{Path, PathBuf};

type H = Harness<'static, FirepixelApp>;

const W: f32 = 1800.0;
const HH: f32 = 1130.0;

fn harness() -> H {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 500, "height": 300, "background": "#ffffff"})).expect("Dokument");
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(W, HH)).with_pixels_per_point(1.0).with_max_steps(20).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    h
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn ev(h: &mut H, e: Event) {
    h.event(e);
    h.run_steps(1);
}

fn button(h: &mut H, p: Pos2, down: bool) {
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: down, modifiers: Modifiers::NONE });
}

fn click(h: &mut H, p: Pos2) {
    ev(h, Event::PointerMoved(p));
    button(h, p, true);
    button(h, p, false);
    h.run_steps(2);
}

fn drag(h: &mut H, a: Pos2, b: Pos2, frames: usize) {
    ev(h, Event::PointerMoved(a));
    button(h, a, true);
    for i in 1..=frames {
        ev(h, Event::PointerMoved(a + (b - a) * (i as f32 / frames as f32)));
    }
    button(h, b, false);
    h.run_steps(3);
}

/// Bildschirmstelle eines Dokumentpunkts auf der Leinwand.
fn screen_of(h: &H, x: f32, y: f32) -> Pos2 {
    let d = st(h).session.active().expect("doc");
    let area = st(h).canvases.get(&d.id).and_then(|c| c.area).expect("Leinwand");
    let v = &d.view;
    area.min + egui::vec2(((x as f64 - v.origin.0) * v.zoom) as f32, ((y as f64 - v.origin.1) * v.zoom) as f32)
}

/// Kombifelder im Inspektor: Rechteck, Wert.
fn combos(h: &H) -> Vec<(egui::Rect, String)> {
    let mut v: Vec<(egui::Rect, String)> = h
        .get_all(By::new().role(egui::accesskit::Role::ComboBox))
        .map(|n| (n.rect(), n.value().unwrap_or_default()))
        .filter(|(r, _)| r.top() > HH - 160.0)
        .collect();
    v.sort_by(|a, b| a.0.left().total_cmp(&b.0.left()).then(a.0.top().total_cmp(&b.0.top())));
    v
}

fn tmp_dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fp_au3_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("Ordner");
    d
}

/// Kleines Graustufen-PNG (Textur: Helligkeit = Maske).
fn write_png(p: &Path) {
    let mut img = image::RgbaImage::new(8, 8);
    for (x, y, px) in img.enumerate_pixels_mut() {
        let v = if (x + y) % 2 == 0 { 255 } else { 0 };
        *px = image::Rgba([v, v, v, 255]);
    }
    img.save(p).expect("PNG");
}

#[test]
fn display_name_of_full_path_is_file_stem() {
    let p = std::env::temp_dir().join("Holz 3.png");
    assert_eq!(obj_quads::texture_display_name(&p.to_string_lossy()), "Holz 3");
    assert_eq!(obj_quads::texture_display_name("Holz 3"), "Holz 3");
    assert_eq!(obj_quads::texture_display_name(""), "");
}

/// Rechteck-Werkzeug, Fülltextur „Andere…“ mit einer Datei außerhalb der Texturordner, dann ein
/// Rechteck aufziehen: Das neue Rechteck hat die Textur mit Maske und dem Dateinamen als Namen;
/// das Feld zeigt den Namen, nicht den Pfad. (Die Strichtextur läuft über dieselben Funktionen.)
#[test]
fn other_texture_as_tool_default_reaches_new_objects() {
    let base = tmp_dir("other");
    let file = base.join("Fremde Textur.png");
    write_png(&file);
    let mut h = harness();
    let _ = h.state_mut().state.session.set_tool(ToolId::Rectangle);
    h.run_steps(3);
    let vein = obj_quads::texture_label(DEFAULT_TEXTURE);
    h.state_mut().state.file_override = Some(fp_ui::files::FilePickOverride { open: vec![file.clone()], save: None });
    // Fülltextur: erstes Feld mit „Ader“ (links)
    let c = combos(&h).into_iter().find(|(_, v)| *v == vein).expect("Texturfeld mit der Vorgabe").0;
    click(&mut h, c.center());
    let other = h.get_by_label("Andere...").rect();
    click(&mut h, other.center());
    h.run_steps(2);
    assert_eq!(obj_quads::tool_attrs(&h.ctx).fill_texture, file.to_string_lossy(), "voller Pfad gemerkt");
    assert!(combos(&h).iter().any(|(_, v)| v == "Fremde Textur"), "Feld zeigt den Namen: {:?}", combos(&h));
    assert!(!combos(&h).iter().any(|(_, v)| v.contains(std::path::MAIN_SEPARATOR)), "kein Pfad im Feld: {:?}", combos(&h));
    let n0 = st(&h).session.active().map(|d| d.history.undo_count()).unwrap_or(0);
    let (a, b) = (screen_of(&h, 60.0, 60.0), screen_of(&h, 200.0, 160.0));
    drag(&mut h, a, b, 6);
    let d = st(&h).session.active().expect("doc");
    assert!(d.history.undo_count() > n0, "Rechteck gezeichnet");
    let id = *d.selection.elements.last().expect("neues Rechteck gewählt");
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, id).expect("Element");
    let attrs = fp_engine::docops::path_attrs_of(&e).expect("Pfadattribute");
    let ft = attrs.fill_texture.as_ref().expect("Fülltextur am neuen Rechteck");
    assert_eq!(ft.name, "Fremde Textur");
    assert!(ft.mask.is_some(), "Datei außerhalb der Texturordner gefunden (Maske)");
    let _ = std::fs::remove_dir_all(&base);
}
