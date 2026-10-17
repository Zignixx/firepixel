//! B-9: Ebenenbefehle mit aktiver Unterebene (Nachfix zu B-5, BA-030/BA-074). Vorher meldeten
//! `layer.move`, `layer.duplicate`, „Ebene duplizieren“ und „Statusübergreifend freigeben“ bei
//! aktiver Unterebene „Ebene nicht gefunden“, `layer.add` legte die neue Ebene ganz oben an.

use fp_engine::docops;
use fp_model::{ElementKind, LayerId};
use serde_json::json;

fn session() -> fp_engine::Session {
    fp_i18n::set_language("de");
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 400, "height": 300, "background": "#ffffff"})).unwrap_or_else(|e| panic!("{e}"));
    s.execute("draw.rectangle", json!({"rect": [10, 10, 50, 50]})).unwrap_or_else(|e| panic!("{e}"));
    s.execute("pi.layer.addSub", json!({"name": "S1"})).unwrap_or_else(|e| panic!("{e}"));
    s
}

fn active(s: &fp_engine::Session) -> LayerId {
    s.active().and_then(|d| d.layer).expect("aktive Ebene")
}

/// Einträge der obersten Ebene (aktueller Status), oben zuerst: Name der Unterebene bzw. "*".
fn top_entries(s: &fp_engine::Session) -> Vec<String> {
    let d = s.active().expect("d");
    let (_, l) = docops::own_layers(&d.doc, d.page).into_iter().find(|(_, l)| l.kind == fp_model::LayerKind::Normal).expect("Ebene");
    l.cel(d.frame)
        .map(|c| {
            c.elements
                .iter()
                .map(|e| match &e.kind {
                    ElementKind::SubLayer(sl) => sl.name.clone(),
                    _ => "*".into(),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn layer_names(s: &fp_engine::Session) -> Vec<String> {
    let d = s.active().expect("d");
    docops::own_layers(&d.doc, d.page).into_iter().map(|(_, l)| l.name.clone()).collect()
}

#[test]
fn duplicate_and_move_active_sublayer() {
    let mut s = session();
    let sub = active(&s);
    assert_eq!(top_entries(&s), ["S1", "*"]);
    // layer.duplicate (Vorgabe aktive Ebene): Kopie direkt darüber, wird aktiv
    let o = s.execute("layer.duplicate", json!({})).unwrap_or_else(|e| panic!("duplicate: {e}"));
    assert_eq!(top_entries(&s), ["S1 Kopie", "S1", "*"]);
    let copy = active(&s);
    assert_ne!(copy, sub);
    assert_eq!(o.value["layer"], json!(copy.0));
    // layer.move: Position in der Elternebene, 0 = unten
    s.execute("layer.move", json!({"index": 0})).unwrap_or_else(|e| panic!("move: {e}"));
    assert_eq!(top_entries(&s), ["S1", "*", "S1 Kopie"]);
    s.execute("layer.move", json!({"index": 2})).unwrap_or_else(|e| panic!("move: {e}"));
    assert_eq!(top_entries(&s), ["S1 Kopie", "S1", "*"]);
    // Rückgängig: je ein Schritt
    s.undo().expect("undo");
    assert_eq!(top_entries(&s), ["S1", "*", "S1 Kopie"]);
    s.undo().expect("undo");
    s.undo().expect("undo");
    assert_eq!(top_entries(&s), ["S1", "*"]);
}

#[test]
fn panel_duplicate_positions_for_sublayer() {
    let mut s = session();
    s.execute("panel.layers.duplicate", json!({"count": 2, "position": "bottom"})).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(top_entries(&s), ["S1", "*", "S1 Kopie", "S1 Kopie"]);
    // neue aktive Unterebene ist eine Kopie
    let a = active(&s);
    let d = s.active().expect("d");
    assert!(docops::as_sublayer(&d.doc, d.page, a).is_some());
}

#[test]
fn share_across_states_with_active_sublayer() {
    let mut s = session();
    let sub = active(&s);
    s.execute("layer.setShared", json!({"shared": true})).unwrap_or_else(|e| panic!("setShared: {e}"));
    let shared = |s: &fp_engine::Session| {
        let d = s.active().expect("d");
        docops::layer_by_id(&d.doc, d.page, sub).expect("Unterebene").shared_across_frames
    };
    assert!(shared(&s));
    assert_eq!(s.command_state("panel.layers.shareAcrossStates").1, Some(true), "Häkchen im Menü");
    s.execute("panel.layers.shareAcrossStates", json!({})).unwrap_or_else(|e| panic!("toggle: {e}"));
    assert!(!shared(&s));
}

#[test]
fn add_layer_above_parent_and_share_to_pages_refused() {
    let mut s = session();
    // „Für Seiten freigeben“ gilt nur für Ebenen der obersten Stufe
    assert!(!s.command_state("panel.layers.shareToAllPages").0, "bei aktiver Unterebene nicht verfügbar");
    let err = s.execute("panel.layers.shareToAllPages", json!({})).expect_err("Unterebene");
    assert!(err.to_string().contains("Unterebene"), "{err}");
    // neue Ebene über der Ebene der obersten Stufe, in der die Unterebene liegt
    let before = layer_names(&s);
    s.execute("layer.add", json!({"name": "Neu"})).unwrap_or_else(|e| panic!("{e}"));
    let after = layer_names(&s);
    let parent = before.iter().position(|n| n != "Web-Ebene").expect("Ebene");
    assert_eq!(after.get(parent + 1).map(String::as_str), Some("Neu"), "{after:?}");
}

#[test]
fn new_objects_land_in_active_sublayer_after_activation() {
    let mut s = session();
    let sub = active(&s);
    // andere Ebene aktiv, dann die Unterebene wieder aktivieren
    s.execute("layer.add", json!({})).unwrap_or_else(|e| panic!("{e}"));
    s.execute("layer.setActive", json!({"layer": sub.0})).unwrap_or_else(|e| panic!("{e}"));
    s.execute("draw.ellipse", json!({"rect": [60, 60, 90, 90]})).unwrap_or_else(|e| panic!("{e}"));
    let d = s.active().expect("d");
    let l = docops::layer_by_id(&d.doc, d.page, sub).expect("Unterebene");
    assert_eq!(l.cel(d.frame).map(|c| c.elements.len()), Some(1), "Ellipse in der Unterebene");
}
