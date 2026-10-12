//! Technical keys always English (AGENTEN_REGELN §1c): the panel table of the command catalogue
//! uses `command_id`; the old key `befehl_id` is still read.

use fp_engine::catalog::{self, PanelEntry};

#[test]
fn panel_table_uses_english_key() {
    let data = include_str!("../data/commands.json");
    assert!(!data.contains("\"befehl_id\""), "old German key in commands.json");
    assert!(data.contains("\"command_id\""));
    assert_eq!(catalog::panels().len(), 20, "all panels read");
    assert!(catalog::panels().iter().all(|p| p.command_id > 0));
}

#[test]
fn old_key_still_read() {
    let old: PanelEntry = serde_json::from_str(r#"{"befehl_id": 35058, "js_floater_name": "layers", "cmd_name": null}"#).expect("old key");
    assert_eq!(old.command_id, 35058);
    let new: PanelEntry = serde_json::from_str(r#"{"command_id": 35059, "js_floater_name": null, "cmd_name": "cmd_X"}"#).expect("new key");
    assert_eq!(new.command_id, 35059);
}
