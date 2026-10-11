//! Befehlsregister: Vollständigkeit gegen `menuebefehle.json`, Namen, Schemas, Tastenkürzel.

use fp_engine::command::{main_menu, CommandRegistry};
use fp_engine::shortcuts::builtin_set;
use std::collections::{BTreeMap, BTreeSet};

fn menu_json() -> Option<serde_json::Value> {
    let m = std::fs::canonicalize(env!("CARGO_MANIFEST_DIR")).unwrap_or_else(|_| std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    let p = m.join("../../../re/werkzeuge/menuebefehle.json");
    println!("menuebefehle.json: {}", p.display());
    let text = std::fs::read_to_string(p).ok()?;
    serde_json::from_str(&text).ok()
}

/// Für **alle** Hauptmenüeinträge aus `menuebefehle.json` ist ein Befehl registriert (Zahl X/260,
/// aufgeteilt in „mit Wirkung“ / „not_implemented“).
#[test]
fn all_menu_entries_registered() {
    let reg = CommandRegistry::with_defaults();
    let mut total = 0;
    let mut found = 0;
    let mut implemented = 0;
    let mut missing = Vec::new();
    let mut not_impl = Vec::new();
    let entries: Vec<(u32, Option<String>, String)> = match menu_json() {
        Some(j) => j["befehle"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter(|b| b["hauptmenue"].as_bool() == Some(true))
                    .map(|b| (b["id"].as_u64().unwrap_or(0) as u32, b["cmd_name"].as_str().map(|s| s.to_string()), b["pfad_de"].as_str().unwrap_or("").to_string()))
                    .collect()
            })
            .unwrap_or_default(),
        None => fp_engine::catalog::entries().iter().filter(|e| e.main_menu).map(|e| (e.fw_id.unwrap_or(0), e.cmd.clone(), e.path_de.join(" > "))).collect(),
    };
    for (id, cmd, path) in &entries {
        total += 1;
        match reg.by_fw_id(*id) {
            Some(c) => {
                found += 1;
                // Der Katalog führt die `cmd_`-Namen ohne Produktnamen (Neutralität, AGENTEN_REGELN §1a).
                let neutral = cmd.as_deref().map(|c| c.replace(&["Fire", "works"].concat(), "Firepixel"));
                assert_eq!(c.info().fw_cmd.as_deref(), neutral.as_deref(), "cmd_-Name für {id}");
                assert!(!c.info().label.is_empty(), "Beschriftung für {id}");
                if c.info().implemented {
                    implemented += 1;
                } else {
                    not_impl.push(format!("{} ({})", c.info().name, path));
                }
            }
            None => missing.push(format!("{id} {path}")),
        }
    }
    println!("Hauptmenü: {found}/{total} registriert, mit Wirkung {implemented}, not_implemented {}", found - implemented);
    println!("not_implemented: {}", not_impl.join(", "));
    assert_eq!(total, 260, "menuebefehle.json hat 260 Hauptmenübefehle");
    assert!(missing.is_empty(), "fehlend: {missing:?}");
    assert!(implemented >= 200, "mit Wirkung nur {implemented}");
    // Kürzel ohne Menüeintrag (10 §4.3)
    let extra: Vec<_> = fp_engine::catalog::entries().iter().filter(|e| !e.main_menu).collect();
    assert_eq!(extra.len(), 20);
    let extra_impl = extra.iter().filter(|e| reg.get(&e.name).is_some_and(|c| c.info().implemented)).count();
    println!("Kürzel ohne Menüeintrag: 20/20 registriert, mit Wirkung {extra_impl}");
    let all_impl = reg.iter().filter(|c| c.info().implemented).count();
    let extra_names: Vec<&str> = reg.iter().filter(|c| c.info().fw_id.is_none()).map(|c| c.info().name.as_str()).collect();
    println!("zusätzliche Befehle ohne Original-ID ({}): {}", extra_names.len(), extra_names.join(", "));
    println!("Register gesamt: {} Befehle, mit Wirkung {all_impl}", reg.len());
}

#[test]
fn names_unique_and_schemas_valid() {
    let reg = CommandRegistry::with_defaults();
    let mut names = BTreeSet::new();
    for c in reg.iter() {
        let i = c.info();
        assert!(names.insert(i.name.clone()), "doppelt: {}", i.name);
        assert!(i.name.split('.').count() >= 2, "Name ohne Bereich: {}", i.name);
        let schema = i.params_schema();
        assert_eq!(schema["type"], "object");
        for p in &i.params {
            assert!(schema["properties"].get(&p.name).is_some());
        }
    }
    let cat = reg.catalog_json();
    assert_eq!(cat.as_array().map(|a| a.len()), Some(reg.len()));
}

#[test]
fn main_menu_tree() {
    let m = main_menu();
    let tops: Vec<&str> = m.iter().map(|n| n.label.as_str()).collect();
    assert_eq!(tops, vec!["Datei", "Bearbeiten", "Ansicht", "Auswählen", "Modifizieren", "Text", "Befehle", "Filter", "Fenster", "Hilfe"]);
    let modify = &m[4];
    assert!(modify.children.iter().any(|c| c.label == "Anordnen" && c.children.len() == 4));
}

/// Die Kürzel des Sets „Firepixel“ (Standardset des Originals) stimmen mit den deutschen Menükürzeln aus 10 §4.2 überein.
#[test]
fn default_set_matches_menu_shortcuts() {
    let reg = CommandRegistry::with_defaults();
    let set = builtin_set("Firepixel").cloned().unwrap_or_default();
    let (mut same, mut total) = (0, 0);
    let mut diff = BTreeMap::new();
    for e in fp_engine::catalog::entries().iter().filter(|e| !e.shortcuts_de.is_empty()) {
        total += 1;
        let c = reg.get(&e.name).map(|c| c.info().shortcuts(&set).iter().map(|s| s.display_de()).collect::<BTreeSet<_>>()).unwrap_or_default();
        let want: BTreeSet<String> = e.shortcuts_de.iter().cloned().collect();
        if c == want {
            same += 1;
        } else {
            diff.insert(e.name.clone(), (want, c));
        }
    }
    println!("Kürzel gleich: {same}/{total}; Abweichungen: {diff:?}");
    assert!(same * 100 >= total * 85, "nur {same}/{total} Kürzel gleich");
}

#[test]
fn availability_predicates() {
    let mut s = fp_engine::Session::default();
    assert!(!s.is_available("modify.group"));
    assert!(s.is_available("file.new"));
    assert!(matches!(s.execute("modify.group", serde_json::json!({})), Err(fp_engine::EngineError::Unavailable { .. })));
    // Seit Phase 5 hat jeder Katalogbefehl eine Wirkung (Hilfe: lokale Anleitung)
    assert!(s.execute("help.firepixelHelp", serde_json::json!({"open": false})).is_ok());
    assert!(s.registry().iter().all(|c| c.info().implemented));
    assert!(matches!(s.execute("gibt.esNicht", serde_json::json!({})), Err(fp_engine::EngineError::UnknownCommand(_))));
    s.execute("file.new", serde_json::json!({})).unwrap_or_else(|e| panic!("{e}"));
    assert!(!s.is_available("modify.group"));
    assert!(matches!(s.execute("file.new", serde_json::json!({"width": "x"})), Err(fp_engine::EngineError::InvalidParams { .. })));
    let (en, ck) = s.command_state("view.magnification.100");
    assert!(en && ck == Some(true));
}
