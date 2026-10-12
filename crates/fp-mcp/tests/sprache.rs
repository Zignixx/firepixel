//! MCP-Antworten an KI-Agenten kommen auf Englisch (nur im Thread des Aufrufs), auf Wunsch in
//! einer anderen Sprache (`language`); die Sprache der Oberfläche bleibt unverändert.

use fp_mcp::tools::{call, ToolEnv};
use serde_json::json;

#[test]
fn antworten_englisch_ohne_oberflaeche_umzuschalten() {
    let mut s = fp_engine::Session::default();
    let env = ToolEnv { language: "en".into(), ..ToolEnv::default() };
    let before = fp_i18n::current_language();
    let r = call(&mut s, &env, "describe_command", &json!({"name": "file.open"}));
    let v = r.structured.clone().unwrap_or_default();
    assert!(v["label"].as_str().is_some_and(|l| l.starts_with("Open")), "{v}");
    let r = call(&mut s, &env, "describe_command", &json!({"name": "datei.gibtsnicht"}));
    assert!(r.is_error && r.text().contains("Unknown command"), "{}", r.text());
    // Deutsch auf Wunsch je Aufruf
    let r = call(&mut s, &env, "describe_command", &json!({"name": "datei.gibtsnicht", "language": "de"}));
    assert!(r.text().contains("unbekannter Befehl"), "{}", r.text());
    let r = call(&mut s, &env, "describe_command", &json!({"name": "file.open", "language": "de"}));
    assert!(r.structured.unwrap_or_default()["label"].as_str().is_some_and(|l| l.starts_with("Öffnen")));
    assert_eq!(fp_i18n::current_language(), before, "Oberflächensprache unverändert");
    // Liste: englische Beschriftungen, Suche auch über sie
    let r = call(&mut s, &env, "list_commands", &json!({"filter": "group"}));
    assert!(r.text().contains("modify.group"), "{}", r.text());
}
