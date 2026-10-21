//! TW-173 (re 55 §7.6): the Edit menu's Redo entry. With an undone step "Redo: <step>"; with nothing
//! undone and a repeatable last step "Repeat: <step>" (German „Wiederholen: <Schritt>“ for both, as
//! in the original); right after a redo and after a bitmap brush stroke "Can't Redo" (disabled).

use fp_engine::tools::ToolId;
use fp_engine::ToolEvent;
use fp_ui::app_state::UiSettings;
use fp_ui::menu::{self, ItemAction, Node};
use fp_ui::FirepixelApp;
use serde_json::json;

fn make_app() -> FirepixelApp {
    let mut session = fp_engine::Session::default();
    session.execute("file.new", json!({ "width": 400, "height": 300 })).expect("document");
    FirepixelApp::with_session(&egui::Context::default(), session, UiSettings::default(), false)
}

fn redo_item(app: &FirepixelApp) -> (String, bool) {
    fn walk(ns: &[Node]) -> Option<(String, bool)> {
        ns.iter().find_map(|n| match n {
            Node::Item { label, enabled, action: ItemAction::Command(c), .. } if c == "edit.redo" => {
                Some((label.replace('&', ""), *enabled))
            }
            Node::Sub { children, .. } => walk(children),
            _ => None,
        })
    }
    menu::build_main_menu(&app.state).iter().find_map(|(_, ns)| walk(ns)).expect("Redo in the Edit menu")
}

fn run(app: &mut FirepixelApp, cmd: &str) {
    app.state.session.execute(cmd, json!({})).unwrap_or_else(|e| panic!("{cmd}: {e}"));
}

/// Clone → "Repeat: Clone" / „Wiederholen: Klonen“; Ctrl+Z → "Redo: Clone"; after the redo
/// "Can't Redo" (disabled); a new step makes the repeat available again.
#[test]
fn redo_entry_reads_redo_repeat_or_cant_redo() {
    let mut app = make_app();
    let v = app.state.session.execute("draw.rectangle", json!({ "rect": [40, 40, 140, 120] })).expect("rect");
    let id = v.value["id"].as_u64().expect("id");
    app.state.session.execute("select.set", json!({ "ids": [id] })).expect("select");
    run(&mut app, "edit.clone");
    fp_i18n::set_language("de");
    assert_eq!(redo_item(&app), ("Wiederholen: Klonen".to_string(), true));
    fp_i18n::set_language("en");
    assert_eq!(redo_item(&app), ("Repeat: Clone".to_string(), true));
    // the repeat runs from the menu entry's command: a third element, a new row
    let rows = app.state.session.active().map(|d| d.history.undo_count()).unwrap_or(0);
    run(&mut app, "edit.redo");
    assert_eq!(app.state.session.active().map(|d| d.history.undo_count()), Some(rows + 1));
    assert_eq!(redo_item(&app), ("Repeat: Clone".to_string(), true), "a repeat is repeatable");
    run(&mut app, "edit.undo");
    // the redo names the undone row as stored (its creation language); the verb is "Redo"
    let (label, enabled) = redo_item(&app);
    assert!(label.starts_with("Redo: ") && enabled, "{label}");
    fp_i18n::set_language("de");
    assert_eq!(redo_item(&app), ("Wiederholen: Klonen".to_string(), true));
    run(&mut app, "edit.redo");
    assert_eq!(redo_item(&app), ("Wiederholen nicht möglich".to_string(), false), "right after a redo");
    fp_i18n::set_language("en");
    assert_eq!(redo_item(&app), ("Can't Redo".to_string(), false));
    run(&mut app, "edit.clone");
    assert_eq!(redo_item(&app), ("Repeat: Clone".to_string(), true));
    fp_i18n::set_language("de");
}

/// After a bitmap brush stroke the entry is disabled: „Wiederholen nicht möglich“.
#[test]
fn redo_entry_greyed_after_brush_stroke() {
    let mut app = make_app();
    app.state.session.set_tool(ToolId::Brush).expect("brush");
    let mut evs = vec![ToolEvent::down(50.0, 50.0)];
    evs.extend((1..=10).map(|i| ToolEvent::drag(50.0 + 10.0 * i as f32, 60.0)));
    evs.push(ToolEvent::up(150.0, 60.0));
    for e in &evs {
        app.state.session.tool_event(e).expect("brush event");
    }
    assert!(app.state.session.active().is_some_and(|d| d.history.can_undo()), "the stroke is a row");
    fp_i18n::set_language("de");
    assert_eq!(redo_item(&app), ("Wiederholen nicht möglich".to_string(), false));
}
