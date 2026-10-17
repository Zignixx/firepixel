//! F13 (docs\berichte\m2.md): Q-18 „Auswahlrahmen in Pfad konvertieren“ mit den Werkzeugvorgaben,
//! Q-19 „Instanzen tweenen“ Vorgabe 10 Schritte, Q-20 „Auswahl animieren“ mit/ohne neue Status und
//! Symbolname „Symbol“, Q-21 „Vorherige/Nächste Auswahl“ über den Auswahlverlauf.

use crate::common;

use common::*;
use fp_color::Argb;
use fp_engine::Session;
use fp_model::{ElementId, ElementKind};
use serde_json::json;

fn selected(s: &Session) -> Vec<ElementId> {
    s.active().map(|d| d.selection.elements.clone()).unwrap_or_default()
}

fn marquee_to_path(s: &mut Session) -> std::sync::Arc<fp_model::PathAttrs> {
    ex(s, "select.pixels", json!({"shape": "rect", "rect": [10, 10, 60, 50]}));
    let v = ex(s, "select.convertMarqueeToPath", json!({}));
    let id = ElementId(v["id"].as_u64().expect("id"));
    assert_eq!(selected(s), vec![id], "neuer Pfad gewählt");
    fp_engine::docops::path_attrs_of(&el(s, id)).expect("Pfadattribute")
}

/// Q-18: Füllung und Strich kommen aus den aktuellen Werkzeugfarben und -vorgaben, kein fester
/// 1-px-Bleistiftstrich ohne Füllung.
#[test]
fn marquee_to_path_uses_tool_defaults() {
    let mut s = doc_session(200.0, 100.0);
    s.set_colors(Some(Argb(0xff00_00ff)), Some(Argb(0xffff_0000)));
    let a = marquee_to_path(&mut s);
    assert!(a.fill.is_some(), "gefüllt wie ein neues Rechteck");
    assert_eq!(a.fill_color, Argb(0xffff_0000));
    assert_eq!(a.stroke_color, Argb(0xff00_00ff));
    assert!(a.brush.is_some(), "Strich der Werkzeugvorgabe");

    // Strichart „keine“ und Füllart „keine“ in den Werkzeugvorgaben bleiben keine
    s.tools.options.new_stroke = Some(json!({"none": true}));
    let a = marquee_to_path(&mut s);
    assert!(a.fill.is_some());
    assert!(a.brush.is_none(), "Strich „keiner“");
    s.tools.options.new_fill = Some(json!({"category": "none"}));
    s.tools.options.new_stroke = None;
    let a = marquee_to_path(&mut s);
    assert!(a.fill.is_none(), "Füllung „keine“");
    assert!(a.brush.is_some());

    // ein Verlaufsschritt, Rückgängig stellt den Auswahlrahmen wieder her
    ex(&mut s, "select.pixels", json!({"shape": "rect", "rect": [10, 10, 60, 50]}));
    let n = s.active().map(|d| d.history.undo_count()).unwrap_or(0);
    ex(&mut s, "select.convertMarqueeToPath", json!({}));
    assert_eq!(s.active().map(|d| d.history.undo_count()), Some(n + 1), "ein Schritt");
    s.undo().expect("undo");
    assert!(s.active().is_some_and(|d| d.selection.has_pixels()));
}

/// Q-19: ohne Angabe 10 Zwischenschritte.
#[test]
fn tween_defaults_to_ten_steps() {
    let mut s = doc_session(400.0, 200.0);
    rect(&mut s, 10.0, 10.0, 40.0, 40.0);
    let a = ElementId(ex(&mut s, "modify.symbol.convertToSymbol", json!({"name": "S"}))["id"].as_u64().expect("id"));
    ex(&mut s, "edit.duplicate", json!({}));
    let b = selected(&s)[0];
    ex(&mut s, "edit.move", json!({"dx": 300, "dy": 0}));
    select(&mut s, &[a, b]);
    ex(&mut s, "modify.symbol.tweenInstances", json!({}));
    assert_eq!(selected(&s).len(), 12, "zwei Instanzen + 10 Zwischenschritte");
}

fn frames(s: &Session) -> usize {
    s.active().map(|d| d.frame_count()).unwrap_or(0)
}

/// Q-20: `addStates` false (Abbrechen) legt die Animation ohne neue Status an, Vorgabe/OK hängt
/// sie an; das Symbol heißt „Symbol“, die Instanz bleibt ohne Namen.
#[test]
fn animate_selection_states_and_name() {
    let mut s = doc_session(300.0, 200.0);
    rect(&mut s, 10.0, 10.0, 60.0, 60.0);
    let v = ex(&mut s, "modify.animation.animateSelection", json!({"frames": 5, "offsetX": 72, "addStates": false}));
    let id = ElementId(v["id"].as_u64().expect("id"));
    assert_eq!(frames(&s), 1, "Abbrechen: keine neuen Status");
    assert!(matches!(el(&s, id).kind, ElementKind::Instance(_)), "Animation trotzdem angelegt");
    assert_eq!(el(&s, id).common.name, None, "Instanz ohne Namen");
    assert_eq!(s.active().and_then(|d| d.doc.symbols.last().map(|y| y.name.clone())).as_deref(), Some("Symbol"));

    rect(&mut s, 100.0, 10.0, 150.0, 60.0);
    ex(&mut s, "modify.animation.animateSelection", json!({"frames": 5, "offsetX": 72}));
    assert_eq!(frames(&s), 5, "OK bzw. abgeschaltete Frage: Status angehängt");
}

/// Q-21: R1, E1, P1 nacheinander auswählen; zurück → E1, zurück → R1, vor → E1. Am Ende des
/// Verlaufs bleibt die Auswahl (nur R1 gewählt, vor → R1).
#[test]
fn next_previous_follow_selection_history() {
    let mut s = doc_session(400.0, 300.0);
    let r1 = rect(&mut s, 10.0, 10.0, 60.0, 60.0);
    let e1 = ellipse(&mut s, 100.0, 10.0, 150.0, 60.0);
    let p1 = rect(&mut s, 200.0, 10.0, 250.0, 60.0);
    rect(&mut s, 300.0, 10.0, 350.0, 60.0);
    // nur R1: kein späterer Eintrag
    select(&mut s, &[r1]);
    ex(&mut s, "select.next", json!({}));
    assert_eq!(selected(&s), vec![r1]);

    select(&mut s, &[e1]);
    select(&mut s, &[p1]);
    ex(&mut s, "select.previous", json!({}));
    assert_eq!(selected(&s), vec![e1]);
    ex(&mut s, "select.previous", json!({}));
    assert_eq!(selected(&s), vec![r1]);
    ex(&mut s, "select.next", json!({}));
    assert_eq!(selected(&s), vec![e1]);
    ex(&mut s, "select.next", json!({}));
    assert_eq!(selected(&s), vec![p1]);
    ex(&mut s, "select.next", json!({}));
    assert_eq!(selected(&s), vec![p1], "läuft nicht im Kreis");

    // zurück, dann neu auswählen: spätere Einträge fallen weg
    ex(&mut s, "select.previous", json!({}));
    select(&mut s, &[r1]);
    ex(&mut s, "select.next", json!({}));
    assert_eq!(selected(&s), vec![r1]);
    ex(&mut s, "select.previous", json!({}));
    assert_eq!(selected(&s), vec![e1]);

    // gelöschte Objekte werden übersprungen
    select(&mut s, &[p1]);
    select(&mut s, &[r1]);
    select(&mut s, &[e1]);
    ex(&mut s, "select.set", json!({"ids": [r1.0]}));
    ex(&mut s, "edit.clear", json!({}));
    select(&mut s, &[e1]);
    ex(&mut s, "select.previous", json!({}));
    assert_eq!(selected(&s), vec![p1], "R1 gelöscht: übersprungen");
}

#[test]
fn selection_history_unit() {
    let (a, b) = (ElementId(1), ElementId(2));
    let mut h = fp_engine::SelectionHistory::default();
    assert_eq!(h.step(-1, &[], |_| true), None);
    h.note(&[a]);
    h.note(&[]);
    h.note(&[a]);
    h.note(&[b]);
    assert_eq!(h.len(), 2, "leere und gleiche Auswahl ergeben keinen Eintrag");
    // Auswahl aufgehoben: zurück führt erst zum aktuellen Eintrag
    assert_eq!(h.step(-1, &[], |_| true), Some(vec![b]));
    assert_eq!(h.step(-1, &[b], |_| true), Some(vec![a]));
    assert_eq!(h.step(-1, &[a], |_| true), None);
}
