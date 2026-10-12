//! KI-Auswahl in der Oberfläche (Agent KI), offscreen mit egui_kittest und echten Maus-Ereignissen:
//! Werkzeug „Objektauswahl“ wählen, auf die Bitmap klicken → Hintergrundrechnung mit
//! Fortschrittsfenster → Pixelauswahl mit laufenden Ameisen; Abbrechen mitten in der Rechnung.
//! Braucht die Modelle im Modellordner (sonst übersprungen).

use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::{By, Queryable};
use egui_kittest::Harness;
use fp_engine::commands::ai::fp_ai;
use fp_engine::ToolId;
use fp_ui::app_state::UiSettings;
use fp_ui::FirepixelApp;
use serde_json::json;
use std::time::{Duration, Instant};

fn have_models() -> bool {
    fp_ai::models::available(fp_ai::models::ModelId::Sam2Tiny)
}

/// Bitmap mit roter Scheibe (Mitte 120/150, Radius 55) auf unruhigem Grund; `seed` macht das Bild
/// je Test verschieden (kalter Zwischenspeicher).
fn test_bitmap(seed: i32) -> fp_raster::RgbaImage {
    let mut img = fp_raster::RgbaImage::new(fp_geom::IRect::from_xywh(0, 0, 400, 300)).unwrap_or_else(|_| fp_raster::RgbaImage::empty_at(0, 0));
    let w = img.width();
    for (i, px) in img.data_mut().iter_mut().enumerate() {
        let (x, y) = ((i % w) as i32, (i / w) as i32);
        let n = ((x * 7919 + y * 104_729 + seed * 31) ^ (x * y)) & 63;
        let stripe = if (x / 9 + y / 13) % 2 == 0 { 30 } else { 0 };
        let mut c = [60 + n as u8 / 2 + stripe, 120 + n as u8, 50 + stripe, 255];
        let (dx, dy) = (x as f32 - 120.0, y as f32 - 150.0);
        if dx * dx + dy * dy <= 55.0 * 55.0 {
            c = [220, 30 + (n as u8 / 4), 40, 255];
        }
        *px = c;
    }
    img
}

fn harness(seed: i32) -> Harness<'static, FirepixelApp> {
    fp_engine::shell::disable();
    fp_ai::download::disable();
    let mut s = fp_engine::Session::default();
    let _ = s.execute("file.new", json!({"width": 400, "height": 300, "background": "#ffffff"}));
    s.clipboard.pixels = Some(test_bitmap(seed));
    let _ = s.execute("edit.paste", json!({}));
    let _ = s.execute("select.none", json!({}));
    let ctx = egui::Context::default();
    // asynchron wie in der App (bei sync_render rechnet die Oberfläche synchron, z. B. im Gesamttest)
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = false;
    let mut h = Harness::builder().with_size(egui::vec2(1500.0, 950.0)).with_pixels_per_point(1.0).with_max_steps(8).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    h
}

/// Dokumentpunkt → Bildschirmpunkt der Leinwand.
fn at(h: &Harness<'static, FirepixelApp>, x: f32, y: f32) -> Pos2 {
    let st = &h.state().state;
    let d = st.session.active().expect("Dokument");
    let area = st.canvases.get(&d.id).and_then(|c| c.area).expect("Leinwand");
    let (sx, sy) = d.view.doc_to_screen(fp_geom::Point::new(x, y));
    Pos2::new(area.min.x + sx as f32, area.min.y + sy as f32)
}

fn click(h: &mut Harness<'static, FirepixelApp>, p: Pos2) {
    for e in [
        Event::PointerMoved(p),
        Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE },
        Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE },
    ] {
        h.input_mut().events.push(e);
        h.step();
    }
}

/// Bilder zeichnen, bis die Fortschrittsanzeige sichtbar ist (nach 300 ms).
fn show_progress(h: &mut Harness<'static, FirepixelApp>) {
    let t = Instant::now();
    let f0 = h.state().state.dialogs.ai.progress_frames;
    // ein paar sichtbare Bilder abwarten (das erste Bild einer Fläche ist nur ein Größenlauf)
    while t.elapsed() < Duration::from_secs(5) && h.state().state.dialogs.ai.busy() && h.state().state.dialogs.ai.progress_frames < f0 + 3 {
        std::thread::sleep(Duration::from_millis(20));
        h.step();
    }
}

/// Bilder laufen lassen, bis die Hintergrundrechnung fertig ist.
fn wait_job(h: &mut Harness<'static, FirepixelApp>, max: Duration) {
    let t = Instant::now();
    while h.state().state.dialogs.ai.busy() && t.elapsed() < max {
        std::thread::sleep(Duration::from_millis(25));
        h.step();
    }
    h.run_steps(2);
}

#[test]
fn object_tool_click_selects_with_marching_ants() {
    if !have_models() {
        eprintln!("KI-Modelle fehlen – Test übersprungen");
        return;
    }
    let mut h = harness(1);
    let _ = h.state_mut().state.session.set_tool(ToolId::AiObject);
    h.step();
    let p = at(&h, 120.0, 150.0);
    click(&mut h, p);
    // erster Klick: Bild-Encoding läuft im Hintergrund, die Oberfläche bleibt bedienbar
    assert!(h.state().state.dialogs.ai.busy(), "Hintergrundrechnung läuft");
    // die Fortschrittsanzeige erscheint erst nach 300 ms (kein Aufblitzen bei kurzen Läufen)
    show_progress(&mut h);
    assert!(h.get_all(By::new().label("Abbrechen")).next().is_some(), "Fortschrittsfenster mit Abbrechen");
    wait_job(&mut h, Duration::from_secs(60));
    let st = &h.state().state;
    let sel = st.session.active().and_then(|d| d.selection.pixels.clone()).expect("Pixelauswahl");
    let b = sel.tight_bounds();
    assert!((b.left - 65).abs() <= 4 && (b.right - 176).abs() <= 4 && (b.top - 95).abs() <= 4 && (b.bottom - 206).abs() <= 4, "{b:?}");
    let cc = st.canvases.values().next().expect("Leinwand");
    assert!(cc.ants.is_some(), "laufende Ameisen");
    // zweiter Klick (Umschalt) reagiert sofort: kein Hintergrundauftrag mehr nötig
    let p = at(&h, 300.0, 60.0);
    for e in [
        Event::PointerMoved(p),
        Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::SHIFT },
        Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::SHIFT },
    ] {
        h.input_mut().events.push(e);
        h.step();
    }
    assert!(!h.state().state.dialogs.ai.busy(), "Zwischenspeicher warm: sofort ausgeführt");
}

#[test]
fn cancel_in_the_middle_keeps_selection_and_document() {
    if !have_models() {
        eprintln!("KI-Modelle fehlen – Test übersprungen");
        return;
    }
    let mut h = harness(2);
    let _ = h.state_mut().state.session.execute("select.pixels", json!({"shape": "rect", "rect": [10, 10, 40, 40]}));
    let sel0 = h.state().state.session.active().and_then(|d| d.selection.pixels.clone());
    let doc0 = h.state().state.session.active().map(|d| d.doc.clone()).expect("Dokument");
    let _ = h.state_mut().state.session.set_tool(ToolId::AiSubject);
    h.step();
    let p = at(&h, 200.0, 150.0);
    click(&mut h, p);
    assert!(h.state().state.dialogs.ai.busy());
    show_progress(&mut h);
    let b: Vec<_> = h.get_all(By::new().label("Abbrechen")).collect();
    b.last().expect("Abbrechen").click();
    drop(b);
    h.step();
    h.step();
    eprintln!("abgebrochen angefordert: {} (läuft: {})", h.state().state.dialogs.ai.cancel_requested(), h.state().state.dialogs.ai.busy());
    assert!(h.state().state.dialogs.ai.cancel_requested() || !h.state().state.dialogs.ai.busy(), "Abbrechen-Knopf wirkt");
    wait_job(&mut h, Duration::from_secs(30));
    let st = &h.state().state;
    assert!(!st.dialogs.ai.busy());
    assert_eq!(st.session.active().and_then(|d| d.selection.pixels.clone()), sel0, "Auswahl unverändert");
    assert!(std::sync::Arc::ptr_eq(&doc0, &st.session.active().expect("Dokument").doc), "Dokument unverändert");
    assert!(st.status.as_ref().is_some_and(|(s, _)| s.contains("abgebrochen")), "Meldung „abgebrochen“: {:?}", st.status);
}

/// Bildschirmfotos der KI-Werkzeuge (offscreen mit wgpu) nach `docs\screenshots\phase5_ki_ui_*.png`:
/// `cargo test -p fp-ui --test ai_kittest bildschirmfotos -- --ignored`.
#[test]
#[ignore]
fn bildschirmfotos() {
    if !fp_ai::models::available(fp_ai::models::ModelId::Sam2Tiny) {
        return;
    }
    fp_engine::shell::disable();
    fp_ai::download::disable();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
    let out = root.join("docs").join("screenshots");
    let photo = root.join("crates").join("fp-ai").join("testdata").join("hund_gras.jpg");
    let mut s = fp_engine::Session::default();
    s.execute("file.open", json!({"path": photo.display().to_string()})).expect("Foto öffnen");
    let ctx = egui::Context::default();
    // synchron rechnen (Bildschirmfoto nach dem Ergebnis)
    let app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    let mut h = Harness::builder().with_size(egui::vec2(1600.0, 1000.0)).with_pixels_per_point(1.0).with_max_steps(8).wgpu().build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(6);
    let _ = h.state_mut().state.session.execute("view.fitAll", json!({}));
    h.run_steps(4);
    let shot = |h: &mut Harness<'static, FirepixelApp>, name: &str| {
        h.run_steps(3);
        match h.render() {
            Ok(i) => {
                let _ = i.save(out.join(format!("phase5_ki_ui_{name}.png")));
            }
            Err(e) => eprintln!("kein Grafikadapter: {e}"),
        }
    };
    let _ = h.state_mut().state.session.set_tool(ToolId::AiObject);
    h.step();
    let p = at(&h, 230.0, 330.0);
    click(&mut h, p);
    shot(&mut h, "objekt");
    let _ = h.state_mut().state.session.set_tool(ToolId::AiSubject);
    h.step();
    let p = at(&h, 640.0, 360.0);
    click(&mut h, p);
    shot(&mut h, "motiv");
    let _ = h.state_mut().state.session.set_tool(ToolId::AiRefineEdge);
    h.step();
    let p = at(&h, 200.0, 300.0);
    click(&mut h, p);
    shot(&mut h, "kante");
}