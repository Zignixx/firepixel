//! FB-12: View ▸ Snap to Pixels sits in the View menu, unchecked by default, and its check mark
//! follows `view.snapToPixels`.

use fp_ui::app_state::UiSettings;
use fp_ui::menu::{self, ItemAction, Node};
use fp_ui::FirepixelApp;

fn make_app() -> FirepixelApp {
    let mut session = fp_engine::Session::default();
    session.execute("file.new", serde_json::json!({ "width": 60, "height": 40 })).expect("document");
    FirepixelApp::with_session(&egui::Context::default(), session, UiSettings::default(), false)
}

fn checked(app: &FirepixelApp) -> Option<bool> {
    fn walk(ns: &[Node]) -> Option<Option<bool>> {
        ns.iter().find_map(|n| match n {
            Node::Item { checked, action: ItemAction::Command(c), .. } if c == "view.snapToPixels" => Some(*checked),
            Node::Sub { children, .. } => walk(children),
            _ => None,
        })
    }
    menu::build_main_menu(&app.state).iter().find_map(|(_, ns)| walk(ns)).expect("menu entry")
}

#[test]
fn snap_to_pixels_menu_unchecked_by_default_and_toggles() {
    let mut app = make_app();
    assert_eq!(checked(&app), Some(false));
    app.state.session.execute("view.snapToPixels", serde_json::json!({})).expect("toggle");
    assert_eq!(checked(&app), Some(true));
    assert!(app.state.session.prefs.get_bool("SnapToPixelGrid", false));
    app.state.session.execute("view.snapToPixels", serde_json::json!({})).expect("toggle");
    assert_eq!(checked(&app), Some(false));
}
