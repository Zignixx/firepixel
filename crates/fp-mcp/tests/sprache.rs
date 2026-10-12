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

/// Menüstufen, die im Deutschen und Englischen gleich heißen.
const SAME: &[&str] = &["Text", "Symbol", "Animation", "Info", "Normal", "URL", "Hotspot", "Firepixel Exchange", "Firepixel Support Center", "Gamma 1.8"];

/// UE-3: Werkzeugliste, Hinweise, Menüpfade, Ressourcen und Fehler folgen der Antwortsprache.
#[test]
fn werkzeuge_ressourcen_und_menuepfade_in_beiden_sprachen() {
    let german = |t: &str| t.chars().any(|c| "äöüÄÖÜß„".contains(c));
    for lang in ["de", "en"] {
        let tools = fp_i18n::with_language(lang, fp_mcp::FirepixelMcp::tool_list);
        assert_eq!(tools.len(), fp_mcp::tools::tool_defs().len());
        for t in &tools {
            let title = t.title.clone().unwrap_or_default();
            let desc = t.description.clone().unwrap_or_default().to_string();
            assert!(!title.is_empty() && !title.starts_with("mcp-tool-") && desc.len() > 30, "{lang}: {} {title:?}", t.name);
            let props = t.input_schema.get("properties").and_then(|p| p.as_object()).cloned().unwrap_or_default();
            for (p, v) in &props {
                if let Some(d) = v.get("description").and_then(|d| d.as_str()) {
                    assert!(!d.starts_with("mcp-"), "{lang}: {}.{p}", t.name);
                    if lang == "en" {
                        assert!(!german(d), "{}.{p}: {d}", t.name);
                    }
                }
            }
            if lang == "en" {
                assert!(!german(&title) && !german(&desc), "{}: {title} – {desc}", t.name);
            }
        }
        let res = fp_i18n::with_language(lang, fp_mcp::resources::resources);
        assert!(res.iter().all(|r| !r.title.starts_with("mcp-res-") && !r.description.is_empty()));
    }
    // deutscher Wortlaut wie bisher
    let de = fp_i18n::with_language("de", fp_mcp::tools::tool_defs);
    assert_eq!(de[0].title, "Befehle auflisten");
    assert!(de.iter().find(|d| d.name == "run_command").is_some_and(|d| d.schema["properties"]["doc"]["description"].as_str().is_some_and(|x| x.starts_with("Dokument-ID"))));
    let en = fp_i18n::with_language("en", fp_mcp::tools::tool_defs);
    assert_eq!(en[0].title, "List commands");
    assert_eq!(fp_i18n::with_language("en", || fp_mcp::server::instructions(false)).split(' ').next(), Some("Firepixel"));
    assert!(fp_i18n::with_language("en", || fp_mcp::server::instructions(true)).contains("Bridge mode"));
    let reg = fp_engine::Session::default();
    let md = fp_i18n::with_language("en", || fp_mcp::resources::commands_markdown(reg.registry()));
    assert!(md.starts_with("# Firepixel command catalog") && md.contains("Gaussian Blur"), "{}", &md[..200]);
    let md = fp_i18n::with_language("en", || fp_mcp::resources::read("firepixel://docs/examples.md", reg.registry())).unwrap_or_default();
    assert!(md.starts_with("# Examples"));
    // Menüpfade englisch, Fehler und Hinweise in der Antwortsprache
    let mut s = fp_engine::Session::default();
    let env = fp_mcp::tools::ToolEnv { language: "en".into(), ..fp_mcp::tools::ToolEnv::default() };
    let v = call(&mut s, &env, "describe_command", &json!({"name": "modify.align.left"})).structured.unwrap_or_default();
    assert_eq!(v["menu"], json!(["Modify", "Align", "Left"]), "{v}");
    let v = call(&mut s, &env, "describe_command", &json!({"name": "modify.align.left", "language": "de"})).structured.unwrap_or_default();
    assert_eq!(v["menu"], json!(["Modifizieren", "Ausrichten", "Links"]), "{v}");
    let r = call(&mut s, &env, "get_document", &json!({}));
    assert!(r.structured.unwrap_or_default()["hint"].as_str().is_some_and(|h| h.starts_with("no document open")));
    let r = call(&mut s, &env, "undo", &json!({}));
    assert!(r.is_error && !german(&r.text()), "{}", r.text());
    let mut bad = Vec::new();
    for c in s.registry().iter() {
        let i = c.info();
        let en = fp_i18n::with_language("en", || fp_mcp::tools::command_menu_path(i));
        let de = fp_i18n::with_language("de", || fp_mcp::tools::command_menu_path(i));
        assert_eq!(de, i.menu_path, "deutscher Menüpfad unverändert: {}", i.name);
        for (seg, d) in en.iter().zip(&de) {
            // gleich gebliebene Stufen nur, wenn das Wort in beiden Sprachen gleich ist (Text, Filter …)
            if german(seg) || (seg == d && seg.chars().any(char::is_alphabetic) && !SAME.contains(&seg.as_str())) {
                bad.push(format!("{}: {seg}", i.name));
            }
        }
    }
    assert!(bad.is_empty(), "{} Menüpfade mit deutschen Stufen:\n{}", bad.len(), bad.join("\n"));
}
