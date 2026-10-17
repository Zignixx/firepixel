//! Bedienfelder offscreen: jedes der 23 Bedienfelder zeichnet ohne Panik (mit und ohne Dokument),
//! jedes Bedienfeldmenü (≡) und jedes Kontextmenü der Bedienfelder führt nur registrierte Befehle
//! aus, und typische Bedienfeldaktionen ändern das Dokument mit Rückgängig.

use egui_kittest::Harness;
use fp_engine::Session;
use fp_ui::app_state::{Action, UiSettings};
use fp_ui::menu::{ItemAction, Node};
use fp_ui::panels::PanelId;
use fp_ui::FirepixelApp;
use serde_json::json;
use std::sync::Arc;

fn session(doc: bool) -> Session {
    let mut s = Session::default();
    if doc {
        s.execute("file.new", json!({"width": 300, "height": 200})).expect("doc");
        s.execute("draw.rectangle", json!({"rect": [20, 20, 120, 80]})).expect("rect");
    }
    s
}

fn harness(doc: bool) -> Harness<'static, FirepixelApp> {
    let ctx = egui::Context::default();
    let mut a = FirepixelApp::with_session(&ctx, session(doc), UiSettings::default(), false);
    a.state.sync_render = true;
    Harness::builder().with_size(egui::vec2(1600.0, 1000.0)).with_pixels_per_point(1.0).with_max_steps(16).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), a)
}

#[test]
fn every_panel_draws_with_and_without_document() {
    for doc in [true, false] {
        for p in PanelId::ALL {
            let mut h = harness(doc);
            h.state_mut().state.push(Action::ShowPanel(*p));
            h.run_steps(3);
            assert!(h.state().state.settings.workspace.is_visible(*p), "{p:?} sichtbar");
        }
    }
}

fn items(nodes: &[Node], out: &mut Vec<(String, ItemAction, bool)>) {
    for n in nodes {
        match n {
            Node::Item { label, action, enabled, .. } => out.push((label.clone(), action.clone(), *enabled)),
            Node::Sub { children, .. } => items(children, out),
            Node::Separator => {}
        }
    }
}

#[test]
fn every_panel_menu_entry_is_a_command() {
    let h = harness(true);
    let st = &h.state().state;
    let mut missing = Vec::new();
    for p in PanelId::ALL {
        let mut v = Vec::new();
        items(&fp_ui::menu::build_panel_menu(st, *p), &mut v);
        for (label, action, _) in v {
            if label == "Hilfe" {
                continue;
            }
            if action == ItemAction::None {
                missing.push(format!("{p:?}: {label}"));
            }
        }
    }
    for res in ["6921", "6922", "6933", "6940", "6942"] {
        let mut v = Vec::new();
        items(&fp_ui::panels::common::resource_menu_nodes(st, res), &mut v);
        for (label, action, _) in v {
            if action == ItemAction::None {
                missing.push(format!("{res}: {label}"));
            }
        }
    }
    assert!(missing.is_empty(), "Einträge ohne Befehl: {missing:#?}");
}

fn doc(h: &Harness<'_, FirepixelApp>) -> Arc<fp_model::Document> {
    Arc::clone(&h.state().state.session.active().expect("doc").doc)
}

/// Bedienfeldmenü-Einträge ohne Dialog direkt ausführen: Dokument ändert sich bzw. Zustand,
/// Rückgängig stellt her (§4b), mehrfaches Ausführen erzeugt keine Duplikate.
#[test]
fn panel_menu_commands_run_and_undo() {
    let toggles = ["panel.layers.hideAll", "panel.layers.lockAll", "panel.states.autoCrop", "panel.optimize.interlaced", "panel.optimize.progressive", "panel.optimize.sharpenJpegEdges", "panel.effects.allOff", "panel.states.loop5", "panel.states.onionBeforeAfter", "panel.states.disposalBackground"];
    for name in toggles {
        let mut h = harness(true);
        h.run_steps(2);
        if name == "panel.effects.allOff" {
            h.state_mut().state.session.execute("element.addEffect", json!({"effect": "dropShadow"})).expect("fx");
        }
        let d0 = doc(&h);
        h.state_mut().state.run(name, json!({}));
        h.run_steps(3);
        assert!(!Arc::ptr_eq(&d0, &doc(&h)), "{name}: keine Wirkung (Status {:?})", h.state().state.status);
        let d1 = doc(&h);
        for _ in 0..5 {
            h.state_mut().state.run(name, json!({}));
            h.run_steps(1);
        }
        // Optionsbefehle (Schleife, Zwiebelschicht, Löschmethode, Alles aus) sind idempotent
        if name.contains("loop") || name.contains("onion") || name.contains("disposal") || name.contains("All") {
            assert!(*doc(&h) == *d1, "{name}: wiederholt nicht idempotent");
        }
        while h.state().state.session.active().is_some_and(|d| !Arc::ptr_eq(&d.doc, &d0) && d.history.can_undo()) {
            h.state_mut().state.session.undo().expect("undo");
        }
        assert!(*doc(&h) == *d0, "{name}: Rückgängig");
    }
}

#[test]
fn auto_shapes_panel_insert_and_properties() {
    let mut h = harness(true);
    h.state_mut().state.push(Action::ShowPanel(PanelId::AutoShapes));
    h.state_mut().state.push(Action::ShowPanel(PanelId::AutoShapeProperties));
    h.run_steps(3);
    let list = fp_ui::panels::autoshapes::shape_list(&mut h.state_mut().state);
    assert!(list.len() >= 10, "Formen: {}", list.len());
    let star = list.iter().find(|s| s.name.to_lowercase().contains("star")).or_else(|| list.first()).expect("Form").name.clone();
    let d0 = doc(&h);
    h.state_mut().state.run("autoShape.insert", json!({"shape": star}));
    h.run_steps(3);
    assert!(!Arc::ptr_eq(&d0, &doc(&h)), "Einfügen ändert das Dokument (Status {:?})", h.state().state.status);
    let id = fp_ui::panels::autoshapes::selected_shape(&h.state().state).expect("automatische Form ausgewählt");
    let desc = h.state_mut().state.session.execute("autoShape.describe", json!({"id": id})).expect("describe").value;
    let params = desc["params"].as_array().cloned().unwrap_or_default();
    // erster Zahlenparameter ändern → Dokument ändert sich, Rückgängig stellt her
    if let Some(p) = params.iter().find(|p| matches!(p["type"].as_str(), Some("number" | "integer"))) {
        let k = p["key"].as_str().expect("key").to_string();
        let v = p["value"].as_f64().or_else(|| p["value"].as_str().and_then(|s| s.parse().ok())).unwrap_or(5.0);
        let max = p["max"].as_f64().unwrap_or(v + 10.0);
        let nv = if v + 1.0 <= max { v + 1.0 } else { v - 1.0 };
        let d1 = doc(&h);
        let mut m = serde_json::Map::new();
        m.insert(k, json!(nv));
        h.state_mut().state.run("autoShape.setParams", json!({"id": id, "params": m}));
        h.run_steps(3);
        assert!(!Arc::ptr_eq(&d1, &doc(&h)), "Parameter ändert die Form (Status {:?})", h.state().state.status);
        h.state_mut().state.session.undo().expect("undo");
        assert!(*doc(&h) == *d1, "Rückgängig");
    }
}
