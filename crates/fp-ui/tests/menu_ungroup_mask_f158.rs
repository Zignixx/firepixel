//! F158 (FB-152, Firepixel extension): *Modify ▸ Ungroup Mask Completely* sits right after
//! *Ungroup* in the Modify menu and in the canvas context menu of selected objects; it is enabled
//! only for a mask group whose Ungroup would leave a group.

use fp_ui::FirepixelApp;
use fp_ui::app_state::UiSettings;
use fp_ui::menu::{self, ItemAction, Node};
use serde_json::json;

const CMD: &str = "modify.ungroupMaskCompletely";

fn make_app() -> FirepixelApp {
    fp_engine::shell::disable();
    fp_i18n::profile::use_test_profile(); // never the real user profile
    let mut session = fp_engine::Session::default();
    session
        .execute("file.new", json!({ "width": 300, "height": 200 }))
        .expect("document");
    FirepixelApp::with_session(
        &egui::Context::default(),
        session,
        UiSettings::default(),
        false,
    )
}

fn command_at(ns: &[Node], i: usize) -> Option<&str> {
    match ns.get(i) {
        Some(Node::Item {
            action: ItemAction::Command(c),
            ..
        }) => Some(c.as_str()),
        _ => None,
    }
}

/// (enabled, command of the node before) of the entry in a menu list.
fn find(ns: &[Node]) -> Option<(bool, Option<String>)> {
    ns.iter().enumerate().find_map(|(i, n)| match n {
        Node::Item {
            enabled,
            action: ItemAction::Command(c),
            ..
        } if c == CMD => Some((
            *enabled,
            i.checked_sub(1)
                .and_then(|j| command_at(ns, j))
                .map(str::to_owned),
        )),
        _ => None,
    })
}

fn modify_entry(app: &FirepixelApp) -> (bool, Option<String>) {
    menu::build_main_menu(&app.state)
        .iter()
        .find_map(|(_, ns)| find(ns))
        .expect("menu entry")
}

fn context_entry(app: &FirepixelApp) -> (bool, Option<String>) {
    let ns = fp_ui::canvas::context::build(&app.state, fp_ui::canvas::context::entries("6925"));
    find(&ns).expect("context menu entry")
}

fn rect(app: &mut FirepixelApp, r: [i32; 4]) -> serde_json::Value {
    app.state
        .session
        .execute("draw.rectangle", json!({ "rect": r }))
        .expect("rectangle")
        .value["id"]
        .clone()
}

#[test]
fn entry_after_ungroup_enabled_only_for_mask_groups_that_leave_a_group() {
    let mut app = make_app();
    let c = rect(&mut app, [20, 40, 200, 160]);
    let c2 = rect(&mut app, [120, 60, 280, 140]);
    let m = rect(&mut app, [10, 10, 290, 190]);
    let s = &mut app.state.session;
    s.execute("select.set", json!({"ids": [c2, c]}))
        .expect("select");
    // plain objects: shown, disabled
    let (en, before) = modify_entry(&app);
    assert_eq!(before.as_deref(), Some("modify.ungroup"));
    assert!(!en);
    let (en, before) = context_entry(&app);
    assert_eq!(before.as_deref(), Some("modify.ungroup"));
    assert!(!en);
    // mask group with two contents: enabled, one History step dissolves it
    let s = &mut app.state.session;
    s.execute("select.set", json!({"ids": [m, c2, c]}))
        .expect("select");
    s.execute("modify.mask.groupAsMask", json!({}))
        .expect("mask");
    assert!(modify_entry(&app).0);
    assert!(context_entry(&app).0);
    let s = &mut app.state.session;
    s.execute(CMD, json!({})).expect("ungroup mask completely");
    assert_eq!(s.active().map(|d| d.selection.elements.len()), Some(3));
}
