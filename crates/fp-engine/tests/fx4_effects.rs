//! FX-4: Effekte der Gruppe „Farbe anpassen, Rauschen, Sonstige“ über die Befehle (re\23):
//! Kurven-Punktlisten bei `element.addEffect`, Logikfälle nach AGENTEN_REGELN §4b.

use crate::common;
use common::*;
use fp_engine::Session;
use fp_model::{DataValue, ElementId};
use serde_json::json;

const CURVES: &str = "{3439b08e-1923-11d3-9bde00e02910d580}";
const HUE: &str = "{3439b08d-1922-11d3-9bde00e02910d580}";
const NOISE: &str = "{e4c0f4bc-c0a3-4cb3-b3513822027e4d9f}";
const INVERT: &str = "{d2541291-70d6-11d1-8c8000a024cdc039}";
const FILL: &str = "{dd54adc0-a279-11d3-b92a000502f3fdbe}";

fn effect_ids(s: &Session, id: ElementId) -> Vec<String> {
    el(s, id).common.effects.as_ref().map(|l| l.effects.iter().map(|e| e.moa_id().unwrap_or("").to_ascii_lowercase()).collect()).unwrap_or_default()
}

fn param(s: &Session, id: ElementId, i: usize, k: &str) -> Option<String> {
    el(s, id).common.effects.as_ref().and_then(|l| l.effects.get(i)).and_then(|e| e.get(k).map(|v| v.to_string()))
}

#[test]
fn add_effect_keeps_curve_point_lists() {
    let mut s = doc_session(100.0, 100.0);
    let a = rect(&mut s, 0.0, 0.0, 40.0, 40.0);
    // JSF-Form `[{x, y}, …]` (applyEffects/effectList) und `[[x, y], …]`
    ex(&mut s, "element.addEffect", json!({"effect": CURVES, "params": {
        "rgb_points": [{"x": 0, "y": 0}, {"x": 128, "y": 145}, {"x": 255, "y": 255}],
        "red_points": [[0, 0], [100, 160], [255, 255]],
        "MB_filter_preview_tile_size": "-1 -1"}}));
    let e = el(&s, a);
    let fx = &e.common.effects.as_ref().expect("Effekt").effects[0];
    match fx.params.entries.iter().find(|d| d.key == "rgb_points").map(|d| &d.value) {
        Some(DataValue::Points(p)) => assert_eq!(p.points.len(), 3),
        other => panic!("rgb_points als Punktliste erwartet, ist {other:?}"),
    }
    match fx.params.entries.iter().find(|d| d.key == "red_points").map(|d| &d.value) {
        Some(DataValue::Points(p)) => assert_eq!((p.points[1].x, p.points[1].y), (100.0, 160.0)),
        other => panic!("red_points als Punktliste erwartet, ist {other:?}"),
    }
    assert_eq!(param(&s, a, 0, "MB_filter_preview_tile_size").as_deref(), Some("-1 -1"));
    // fehlende Listen (grün/blau): kein Fehler, Effekt bleibt in der Liste
    assert_eq!(effect_ids(&s, a), vec![CURVES.to_string()]);
}

/// §4b.1/§4b.3: Wert 50× ändern → ein Effekt mit dem Endwert; Rückgängig/Wiederholen exakt.
#[test]
fn repeated_param_changes_keep_one_effect() {
    let mut s = doc_session(100.0, 100.0);
    let a = rect(&mut s, 0.0, 0.0, 40.0, 40.0);
    ex(&mut s, "element.addEffect", json!({"effect": HUE, "params": {"hue_amount": "0", "saturation_amount": "0", "lightness_amount": "0", "hls_colorize": "false"}}));
    for v in -25..25 {
        ex(&mut s, "pi.effect.set", json!({"index": 0, "params": {"hue_amount": (v * 7).to_string()}}));
    }
    assert_eq!(effect_ids(&s, a).len(), 1);
    assert_eq!(param(&s, a, 0, "hue_amount").as_deref(), Some("168"));
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(param(&s, a, 0, "hue_amount").as_deref(), Some("161"));
    ex(&mut s, "edit.redo", json!({}));
    assert_eq!(param(&s, a, 0, "hue_amount").as_deref(), Some("168"));
    assert_eq!(effect_ids(&s, a).len(), 1);
}

/// §4b.3: A, B, Rückgängig, C – Liste exakt („Wiederholen“ nach einer neuen Aktion wiederholt wie im
/// Original den letzten Befehl und wird hier nicht geprüft).
#[test]
fn undo_redo_sequence() {
    let mut s = doc_session(100.0, 100.0);
    let a = rect(&mut s, 0.0, 0.0, 40.0, 40.0);
    ex(&mut s, "element.addEffect", json!({"effect": NOISE, "params": {"add_noise_amount": "50", "add_noise_use_color": "false"}}));
    ex(&mut s, "element.addEffect", json!({"effect": INVERT}));
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(effect_ids(&s, a), vec![NOISE.to_string()]);
    ex(&mut s, "element.addEffect", json!({"effect": FILL, "params": {"Color": "#0000ff", "Opacity": "20", "Blendmode": "0"}}));
    assert_eq!(effect_ids(&s, a), vec![NOISE.to_string(), FILL.to_string()]);
    ex(&mut s, "edit.undo", json!({}));
    ex(&mut s, "edit.undo", json!({}));
    assert!(effect_ids(&s, a).is_empty());
}

/// §4b.4/§4b.5: Auswahlwechsel, Mehrfachauswahl, leere Auswahl.
#[test]
fn selection_cases() {
    let mut s = doc_session(200.0, 100.0);
    let a = rect(&mut s, 0.0, 0.0, 40.0, 40.0);
    let b = rect(&mut s, 60.0, 0.0, 100.0, 40.0);
    select(&mut s, &[a]);
    ex(&mut s, "element.addEffect", json!({"effect": INVERT}));
    select(&mut s, &[b]);
    // b hat keinen Effekt 0 → Fehler, a bleibt unverändert
    assert!(s.execute("pi.effect.set", json!({"index": 0, "params": {"hue_amount": "10"}})).is_err());
    assert_eq!(effect_ids(&s, a), vec![INVERT.to_string()]);
    assert!(effect_ids(&s, b).is_empty());
    select(&mut s, &[a, b]);
    ex(&mut s, "element.addEffect", json!({"effect": FILL, "params": {"Color": "#00ff00", "Opacity": "70", "Blendmode": "1"}}));
    assert_eq!(effect_ids(&s, a), vec![INVERT.to_string(), FILL.to_string()]);
    assert_eq!(effect_ids(&s, b), vec![FILL.to_string()]);
    select(&mut s, &[]);
    let r = s.execute("element.addEffect", json!({"effect": INVERT}));
    assert!(r.is_err() || (effect_ids(&s, a).len() == 2 && effect_ids(&s, b).len() == 1));
}
