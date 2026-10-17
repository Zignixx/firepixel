//! NZ-5: Nacharbeiten zu NZ-4.
//! `ui_file_dialog` (MCP-Brücke) kennt die Zusatzkästchen des Dialogs in der App, und der Startordner
//! „Dokumente“ der Datei-Dialoge kommt über den Profil-Wächter (im Testlauf aus dem Testprofil).
//! Wie alle Testläufe mit `FP_NO_SHELL_OPEN=1` starten (AGENTEN_REGELN §4a).

use fp_ui::files::{self, DialogKind};
use serde_json::{json, Value};

// ------------------------------------------------------------------ 1. ui_file_dialog: Kästchen

#[test]
fn bridge_reads_the_three_check_boxes_with_english_names() {
    use fp_ui::bridge::file_dialog_flags;
    assert_eq!(file_dialog_flags(&json!({})), [None, None, None]);
    assert_eq!(file_dialog_flags(&json!({"action": "choose", "path": "a.png"})), [None, None, None]);
    assert_eq!(
        file_dialog_flags(&json!({"append_fw_png": false, "open_untitled": true, "open_as_animation": true})),
        [Some(false), Some(true), Some(true)]
    );
    assert_eq!(file_dialog_flags(&json!({"open_as_animation": false})), [None, None, Some(false)]);
    // kein bool → bleibt, wie es ist
    assert_eq!(file_dialog_flags(&json!({"open_untitled": "ja"})), [None, None, None]);
}

fn ui_file_dialog_schema() -> Value {
    let d = fp_mcp::tools::tool_defs().into_iter().find(|d| d.name == "ui_file_dialog").expect("ui_file_dialog");
    d.schema
}

#[test]
fn mcp_schema_lists_the_check_boxes_and_the_set_action() {
    let s = ui_file_dialog_schema();
    let props = &s["properties"];
    for k in ["append_fw_png", "open_untitled", "open_as_animation"] {
        assert_eq!(props[k]["type"], "boolean", "{k}");
        assert!(props[k]["description"].as_str().is_some_and(|d| !d.is_empty()), "{k}: Beschreibung");
    }
    let actions: Vec<&str> = props["action"]["enum"].as_array().unwrap().iter().filter_map(Value::as_str).collect();
    assert_eq!(actions, ["status", "set", "cancel", "choose"]);
    // die Anfrage reicht die Parameter unverändert an die Oberfläche weiter
    let args = json!({"action": "choose", "paths": ["a.png", "b.png"], "open_as_animation": true});
    match fp_mcp::tools::ui_request_for("ui_file_dialog", &args) {
        Some(fp_mcp::UiRequest::FileDialog(a)) => assert_eq!(a, args),
        _ => panic!("ui_file_dialog → UiRequest::FileDialog"),
    }
}

#[test]
fn mcp_descriptions_of_the_check_boxes_in_german_and_english() {
    let de = ui_file_dialog_schema();
    assert!(de["properties"]["open_as_animation"]["description"].as_str().unwrap().contains("Als Animation öffnen"));
    fp_i18n::set_language("en");
    let en = ui_file_dialog_schema();
    fp_i18n::set_language("de");
    assert!(en["properties"]["open_as_animation"]["description"].as_str().unwrap().contains("Open as Animation"));
    assert!(en["properties"]["append_fw_png"]["description"].as_str().unwrap().contains("Append .fw.png"));
}

// ------------------------------------------------------------------ 2. Startordner im Testprofil

#[test]
fn documents_folder_comes_from_the_profile_guard() {
    assert!(fp_i18n::profile::is_test_run());
    let root = fp_i18n::profile::redirected_root().unwrap_or_else(fp_i18n::profile::use_test_profile);
    let docs = files::documents_dir().expect("Dokumente");
    assert_eq!(Some(docs.clone()), fp_i18n::profile::known_folder(fp_i18n::profile::KnownFolder::Documents));
    assert!(docs.starts_with(&root), "{docs:?} liegt im Testprofil {root:?}");
    // Rückfall aller Datei-Dialoge ohne gemerkten Ordner
    let prefs = fp_engine::Preferences::new();
    for kind in [DialogKind::Open, DialogKind::Import, DialogKind::SaveAs, DialogKind::UpdateHtml] {
        let d = files::start_dir(&prefs, kind, None).expect("Startordner");
        assert!(d.starts_with(&root), "{kind:?}: {d:?} liegt im Testprofil");
    }
}
