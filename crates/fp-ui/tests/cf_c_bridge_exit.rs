//! CT-C-10: Beim Beenden der App (Strg+Q, WM_CLOSE → eframe `on_exit`) wird der KI-Steuerkanal
//! angehalten und `mcp-bridge.json` gelöscht. Eigener Prozess: startet die echte Brücke (HTTP,
//! prozessweiter Zustand in `fp_ui::bridge`).

use fp_ui::FirepixelApp;
use fp_ui::app_state::UiSettings;

#[test]
fn ct_c_10_bridge_token_file_removed_on_exit() {
    fp_engine::shell::disable();
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(
        &ctx,
        fp_engine::Session::default(),
        UiSettings::default(),
        false,
    );
    let file = fp_mcp::bridge_token_file();
    fp_ui::bridge::enable(0);
    fp_ui::bridge::frame(&ctx, &mut app.state);
    assert!(fp_ui::bridge::running_url().is_some(), "Steuerkanal läuft");
    assert!(
        file.is_file(),
        "Token-Datei geschrieben: {}",
        file.display()
    );
    eframe::App::on_exit(&mut app);
    assert!(
        fp_ui::bridge::running_url().is_none(),
        "Steuerkanal angehalten"
    );
    assert!(!file.exists(), "Token-Datei gelöscht");
    // zweites Beenden (run_native kehrt zurück) schadet nicht
    fp_ui::bridge::shutdown();
    assert!(!file.exists());
}
