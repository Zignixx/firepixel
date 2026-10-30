//! F148 (TW-283): View ▸ Gap Measurements (Firepixel's own switch) sits right after the Smart
//! Guides submenu, before Snap to Pixels, unchecked by default; its check mark follows
//! `view.toggleGapMeasurements` and the persisted preference `GapMeasurements`.

use fp_ui::FirepixelApp;
use fp_ui::app_state::UiSettings;
use fp_ui::menu::{self, ItemAction, Node};

const CMD: &str = "view.toggleGapMeasurements";

fn make_app() -> FirepixelApp {
    fp_engine::shell::disable();
    fp_i18n::profile::use_test_profile(); // never the real user profile
    let mut session = fp_engine::Session::default();
    session
        .execute("file.new", serde_json::json!({ "width": 60, "height": 40 }))
        .expect("document");
    FirepixelApp::with_session(
        &egui::Context::default(),
        session,
        UiSettings::default(),
        false,
    )
}

/// (label, check mark, label of the node before, command of the node after) in the View menu.
fn entry(app: &FirepixelApp) -> (String, Option<bool>, String, Option<String>) {
    let menus = menu::build_main_menu(&app.state);
    for (_, ns) in &menus {
        for (i, n) in ns.iter().enumerate() {
            if let Node::Item {
                label,
                checked,
                action: ItemAction::Command(c),
                ..
            } = n
                && c == CMD
            {
                let before = match i.checked_sub(1).map(|j| &ns[j]) {
                    Some(Node::Sub { label, .. }) => label.clone(),
                    other => panic!("not after a submenu: {other:?}"),
                };
                let after = match ns.get(i + 1) {
                    Some(Node::Item {
                        action: ItemAction::Command(c),
                        ..
                    }) => Some(c.clone()),
                    _ => None,
                };
                return (label.clone(), *checked, before, after);
            }
        }
    }
    panic!("menu entry {CMD} missing")
}

#[test]
fn gap_measurements_menu_after_smart_guides_unchecked_by_default_and_toggles() {
    for (lang, text, smart) in [
        ("en", "Gap Measurements", "Smart Guides"),
        ("de", "Abstandsmessung", "Intelligente Hilfslinien"),
    ] {
        fp_i18n::set_language(lang);
        let mut app = make_app();
        let (label, ck, before, after) = entry(&app);
        assert_eq!(label, text);
        assert_eq!(ck, Some(false), "off by default");
        assert_eq!(before, smart);
        assert_eq!(after.as_deref(), Some("view.snapToPixels"));
        app.state
            .session
            .execute(CMD, serde_json::json!({}))
            .expect("toggle");
        assert_eq!(entry(&app).1, Some(true));
        assert!(app.state.session.prefs.get_bool("GapMeasurements", false));
        app.state
            .session
            .execute(CMD, serde_json::json!({}))
            .expect("toggle");
        assert_eq!(entry(&app).1, Some(false));
        assert!(!app.state.session.prefs.get_bool("GapMeasurements", true));
    }
    fp_i18n::set_language("en");
}
