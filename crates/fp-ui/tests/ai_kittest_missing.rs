//! KI-Auswahl ohne Modelle (Agent KI): Klick mit „Motiv auswählen“ öffnet den Dialog „KI-Modell
//! fehlt“ mit Name, Quelle, Größe und Lizenz; ohne Zustimmung wird nichts geladen, „Abbrechen“
//! schließt ihn ohne Änderung. Eigene Testdatei (eigener Prozess), weil die Modellsuche umgestellt
//! wird.

use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::{By, Queryable};
use egui_kittest::Harness;
use fp_engine::commands::ai::fp_ai;
use fp_engine::ToolId;
use fp_ui::app_state::UiSettings;
use fp_ui::FirepixelApp;
use serde_json::json;

#[test]
fn missing_model_shows_download_dialog_and_downloads_nothing() {
    fp_engine::shell::disable();
    fp_ai::download::disable();
    let dir = std::env::temp_dir().join(format!("fp-ai-ui-leer-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    fp_ai::models::set_search_override(vec![dir.clone()]);
    let mut s = fp_engine::Session::default();
    let _ = s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"}));
    let _ = s.execute("draw.ellipse", json!({"rect": [50, 50, 150, 120]}));
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    let mut h = Harness::builder().with_size(egui::vec2(1500.0, 950.0)).with_pixels_per_point(1.0).with_max_steps(8).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    let doc0 = h.state().state.session.active().map(|d| d.doc.clone()).expect("Dokument");
    let _ = h.state_mut().state.session.set_tool(ToolId::AiSubject);
    h.step();
    let p = {
        let st = &h.state().state;
        let d = st.session.active().expect("Dokument");
        let area = st.canvases.get(&d.id).and_then(|c| c.area).expect("Leinwand");
        let (sx, sy) = d.view.doc_to_screen(fp_geom::Point::new(100.0, 80.0));
        Pos2::new(area.min.x + sx as f32, area.min.y + sy as f32)
    };
    for e in [
        Event::PointerMoved(p),
        Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE },
        Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE },
    ] {
        h.input_mut().events.push(e);
        h.step();
    }
    h.run_steps(2);
    {
        let st = &h.state().state;
        let m = st.dialogs.ai.missing.as_ref().expect("Dialog „KI-Modell fehlt“");
        assert_eq!(m.model, fp_ai::models::ModelId::BiRefNetLite);
        assert!(m.download.is_none(), "ohne Zustimmung kein Download");
        assert!(!st.dialogs.ai.busy());
    }
    // Inhalt: Name, Quelle, Lizenz
    assert!(h.get_all(By::new().label_contains("BiRefNet lite")).next().is_some(), "Modellname");
    assert!(h.get_all(By::new().label_contains("huggingface.co/ZhengPeng7/BiRefNet_lite")).next().is_some(), "Quelle");
    assert!(h.get_all(By::new().label("Lizenz")).next().is_some() && h.get_all(By::new().label("MIT")).next().is_some(), "Lizenz");
    // „Herunterladen“ ist in Tests gesperrt; ein Klick darauf lädt nichts
    if let Some(b) = h.get_all(By::new().label("Herunterladen")).next() {
        b.click();
    }
    h.run_steps(3);
    assert!(h.state().state.dialogs.ai.missing.as_ref().is_some_and(|m| m.download.is_none()), "kein Download");
    // Abbrechen schließt
    let b: Vec<_> = h.get_all(By::new().label("Abbrechen")).collect();
    b.last().expect("Abbrechen").click();
    drop(b);
    h.run_steps(3);
    let st = &h.state().state;
    assert!(st.dialogs.ai.missing.is_none(), "Dialog geschlossen");
    assert!(std::sync::Arc::ptr_eq(&doc0, &st.session.active().expect("Dokument").doc), "Dokument unverändert");
    assert!(!dir.join(fp_ai::models::info(fp_ai::models::ModelId::BiRefNetLite).file).exists());
    fp_ai::models::set_search_override(Vec::new());
    let _ = std::fs::remove_dir_all(&dir);
}
