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

/// Befund I18N-MENUE/UE-2: englische Beschriftungen und Beschreibungen für alle Befehle in
/// `describe_command`/`list_commands` (aus `cmd-…`, `.label`, `.description` in menue.ftl).
#[test]
fn englische_beschriftungen_fuer_alle_befehle() {
    let mut s = fp_engine::Session::default();
    let env = ToolEnv { language: "en".into(), ..ToolEnv::default() };
    let describe = |s: &mut fp_engine::Session, name: &str| call(s, &env, "describe_command", &json!({"name": name})).structured.unwrap_or_default();
    assert_eq!(describe(&mut s, "file.open")["label"], "Open");
    assert_eq!(describe(&mut s, "select.feather")["label"], "Feather");
    assert_eq!(describe(&mut s, "filter.blur.gaussian")["label"], "Gaussian Blur");
    // `.label` macht Bedienfeldmenü-Befehle eindeutig
    assert_eq!(describe(&mut s, "panel.states.properties")["label"], "State Properties");
    assert_eq!(describe(&mut s, "file.open")["description"], "Opens an existing document");
    // Suche auch über englische Beschreibungen, Kategorie-Hinweis englisch
    let r = call(&mut s, &env, "list_commands", &json!({"filter": "existing document"}));
    assert!(r.text().contains("file.open"), "{}", r.text());
    let r = call(&mut s, &env, "list_commands", &json!({}));
    assert!(r.structured.unwrap_or_default()["hint"].as_str().is_some_and(|h| h.starts_with("call list_commands")));
    let names: Vec<String> = s.registry().iter().map(|c| c.info().name.clone()).collect();
    let mut bad = Vec::new();
    for n in &names {
        let v = describe(&mut s, n);
        let label = v["label"].as_str().unwrap_or_default().to_string();
        let german = |t: &str| t.chars().any(|c| "äöüÄÖÜß".contains(c));
        if label.is_empty() || label.contains('&') || label.ends_with("...") || label.ends_with('…') || german(&label) {
            bad.push(format!("{n}: label {label:?}"));
        }
        if let Some(d) = v["description"].as_str().filter(|d| german(d)) {
            bad.push(format!("{n}: description {d:?}"));
        }
    }
    assert!(bad.is_empty(), "{} Befehle ohne englische Texte:\n{}", bad.len(), bad.join("\n"));
}
