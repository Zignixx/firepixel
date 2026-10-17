//! Offscreen-Bilder (wgpu) des Inspektors je Elementart, der Bedienfelder Ebenen/Optimieren und
//! eines Filterdialogs nach `docs\screenshots\phase5_ibd_*.png` – zum Vergleich mit
//! `re\_testdateien\11\pi2_*.png` und `re\ui-referenz\*.png`. Ohne Grafikadapter übersprungen.

use egui_kittest::Harness;
use fp_engine::Session;
use fp_ui::app_state::{Action, UiSettings};
use fp_ui::panels::PanelId;
use fp_ui::FirepixelApp;
use serde_json::json;
use std::path::PathBuf;

fn out_dir() -> PathBuf {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("docs").join("screenshots");
    let _ = std::fs::create_dir_all(&p);
    p
}

fn app(s: Session) -> FirepixelApp {
    let ctx = egui::Context::default();
    let mut a = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    a.state.sync_render = true;
    a
}

const W: f32 = 1600.0;
const H: f32 = 1000.0;

fn harness(a: FirepixelApp) -> Harness<'static, FirepixelApp> {
    Harness::builder().renderer(crate::gpu_common::renderer()).with_size(egui::vec2(W, H)).with_pixels_per_point(1.0).with_max_steps(20).renderer(crate::gpu_common::renderer()).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), a)
}

fn ex(s: &mut Session, n: &str, p: serde_json::Value) {
    s.execute(n, p).unwrap_or_else(|e| panic!("{n}: {e}"));
}

fn doc() -> Session {
    let mut s = Session::default();
    ex(&mut s, "file.new", json!({"width": 500, "height": 400}));
    s
}

/// Speichert das ganze Fenster und den Inspektor (unterer Streifen) – `false` ohne Adapter.
fn save(h: &mut Harness<'_, FirepixelApp>, name: &str) -> bool {
    h.run_steps(8);
    let img = match h.render() {
        Ok(i) => i,
        Err(e) => {
            eprintln!("kein Grafikadapter: {e}");
            return false;
        }
    };
    let _ = img.save(out_dir().join(format!("phase5_ibd_{name}.png")));
    true
}

#[test]
fn snapshots_inspector_panels_dialogs() {
    let mut cases: Vec<(&str, Session)> = Vec::new();
    let mut s = doc();
    ex(&mut s, "draw.rectangle", json!({"rect": [40, 40, 440, 340]}));
    ex(&mut s, "pi.fill", json!({"color": "#ff0000"}));
    ex(&mut s, "element.setName", json!({"name": "rot"}));
    cases.push(("rechteck", s));
    let mut s = doc();
    ex(&mut s, "draw.ellipse", json!({"rect": [40, 40, 300, 200]}));
    ex(&mut s, "pi.stroke", json!({"width": 2, "color": "#000000"}));
    cases.push(("pfad", s));
    let mut s = doc();
    ex(&mut s, "draw.text", json!({"text": "Firepixel", "anchor": [40, 120], "size": 36}));
    cases.push(("text", s));
    let mut s = doc();
    ex(&mut s, "draw.ellipse", json!({"rect": [40, 40, 300, 200]}));
    ex(&mut s, "modify.flattenSelection", json!({}));
    cases.push(("bitmap", s));
    let mut s = doc();
    ex(&mut s, "draw.rectangle", json!({"rect": [40, 40, 140, 140]}));
    ex(&mut s, "draw.ellipse", json!({"rect": [160, 40, 300, 200]}));
    ex(&mut s, "select.all", json!({}));
    ex(&mut s, "modify.group", json!({}));
    cases.push(("gruppe", s));
    let mut s = doc();
    ex(&mut s, "draw.ellipse", json!({"rect": [40, 40, 300, 200]}));
    ex(&mut s, "modify.symbol.convertToSymbol", json!({"name": "Knopf", "kind": "button"}));
    cases.push(("schaltflaeche", s));
    cases.push(("dokument", doc()));
    for (name, s) in cases {
        let mut h = harness(app(s));
        if !save(&mut h, &format!("inspektor_{name}")) {
            return;
        }
    }
    // Bedienfelder Ebenen und Optimieren, Filterdialog
    let mut s = doc();
    ex(&mut s, "draw.rectangle", json!({"rect": [40, 40, 240, 200]}));
    ex(&mut s, "draw.ellipse", json!({"rect": [200, 120, 420, 320]}));
    ex(&mut s, "pi.fill", json!({"color": "#3366cc"}));
    ex(&mut s, "layer.add", json!({"name": "Ebene 2"}));
    let mut h = harness(app(s));
    h.state_mut().state.push(Action::ShowPanel(PanelId::Layers));
    save(&mut h, "ebenen");
    h.state_mut().state.push(Action::ShowPanel(PanelId::Optimize));
    save(&mut h, "optimieren");
    h.state_mut().state.dialogs.filter_vectors_ok = true; // TW-21: Rückfrage wie nach OK überspringen
    h.state_mut().state.run("filter.blur.gaussian", json!({}));
    save(&mut h, "filterdialog");
    h.key_press(egui::Key::Escape);
    h.state_mut().state.run("panel.layers.duplicate", json!({}));
    save(&mut h, "dialog_ebene_duplizieren");
    h.key_press(egui::Key::Escape);
    h.state_mut().state.run("edit.keyboardShortcuts", json!({}));
    save(&mut h, "tastaturkurzbefehle");
    h.key_press(egui::Key::Escape);
    h.state_mut().state.run("modify.popUpMenu.addPopUpMenu", json!({}));
    save(&mut h, "meldung_popup");
}
