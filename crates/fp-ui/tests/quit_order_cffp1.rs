//! FP2-01 (re 29 CT-A-19): Datei ▸ Beenden geht die Dokumente rückwärts durch – ab dem zuletzt
//! geöffneten; ungeänderte schließen ohne Nachfrage, beim ersten geänderten kommt die Rückfrage,
//! Abbrechen stoppt dort (davor geöffnete Dokumente bleiben offen).

use egui_kittest::Harness;
use fp_ui::app_state::{Action, UiSettings};
use fp_ui::FirepixelApp;
use serde_json::json;

#[test]
fn fp2_01_quit_walks_documents_backwards() {
    let mut s = fp_engine::Session::default();
    let mut ids = Vec::new();
    for i in 0..3 {
        s.execute("file.new", json!({"width": 100, "height": 80, "background": "#ffffff"})).expect("Dokument");
        ids.push(s.active().expect("aktiv").id);
        if i == 1 {
            s.execute("draw.rectangle", json!({"rect": [10, 10, 50, 50], "fillColor": "#ff0000"})).expect("Rechteck");
        }
    }
    let (a, b, c) = (ids[0], ids[1], ids[2]);
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    let mut h = Harness::builder().with_size(egui::vec2(1200.0, 800.0)).with_pixels_per_point(1.0).with_max_steps(12).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(3);
    h.state_mut().state.push(Action::Quit);
    h.run_steps(4);
    let st = &h.state().state;
    let open: Vec<_> = st.session.documents().iter().map(|d| d.id).collect();
    assert_eq!(open, vec![a, b], "C (zuletzt geöffnet, ungeändert) ohne Nachfrage zu, A noch offen");
    let flow = st.dialogs.unsaved.as_ref().expect("Rückfrage");
    assert_eq!(flow.queue.first(), Some(&b), "Rückfrage für B");
    assert!(!st.close_window, "Fenster bleibt bis zur Antwort");
    // Abbrechen: A und B bleiben
    h.state_mut().state.dialogs.unsaved = None;
    h.run_steps(3);
    let open: Vec<_> = h.state().state.session.documents().iter().map(|d| d.id).collect();
    assert_eq!(open, vec![a, b]);
    let _ = c;
}
