//! FIX-G4 (TWEAKS TW-07): Bearbeiten-Menü ohne Verlauf wie im Original „Rückgängig nicht möglich“ /
//! „Wiederholen nicht möglich“ (grau); mit Verlauf „Rückgängig: <Schritt>“ wie bisher.

use fp_ui::app_state::UiSettings;
use fp_ui::menu::{self, ItemAction, Node};
use fp_ui::FirepixelApp;

fn make_app() -> FirepixelApp {
    let mut session = fp_engine::Session::default();
    session.execute("file.new", serde_json::json!({ "width": 400, "height": 300 })).expect("Dokument");
    FirepixelApp::with_session(&egui::Context::default(), session, UiSettings::default(), false)
}

fn item(app: &FirepixelApp, cmd: &str) -> (String, bool) {
    fn walk(ns: &[Node], cmd: &str) -> Option<(String, bool)> {
        ns.iter().find_map(|n| match n {
            Node::Item { label, enabled, action: ItemAction::Command(c), .. } if c == cmd => Some((label.clone(), *enabled)),
            Node::Sub { children, .. } => walk(children, cmd),
            _ => None,
        })
    }
    menu::build_main_menu(&app.state).iter().find_map(|(_, ns)| walk(ns, cmd)).expect("Eintrag im Menü")
}

#[test]
fn undo_redo_without_history_say_not_possible() {
    fp_i18n::set_language("de");
    let mut app = make_app();
    let (u, ue) = item(&app, "edit.undo");
    let (r, re) = item(&app, "edit.redo");
    assert_eq!((u.replace('&', ""), ue), ("Rückgängig nicht möglich".to_string(), false));
    assert_eq!((r.replace('&', ""), re), ("Wiederholen nicht möglich".to_string(), false));
    fp_i18n::set_language("en");
    assert_eq!(item(&app, "edit.undo").0.replace('&', ""), "Can't Undo");
    assert_eq!(item(&app, "edit.redo").0.replace('&', ""), "Can't Redo");
    fp_i18n::set_language("de");
    // mit Verlauf wie bisher: „Rückgängig: <Schritt>“, aktiv
    app.state.session.execute("draw.rectangle", serde_json::json!({ "rect": [10, 10, 60, 40] })).expect("Rechteck");
    let step = app.state.session.active().and_then(|d| d.history.undo_label().map(str::to_string)).expect("Schritt");
    let (u, ue) = item(&app, "edit.undo");
    assert_eq!((u.replace('&', ""), ue), (format!("Rückgängig: {step}"), true));
}
