//! B3 mit Messwerten von B-2 (`docs\berichte\b2.md`): BA-005 Verbinden ergibt immer einen Pfad ohne
//! Namen (auch ein einzelner offener oder geschlossener Pfad), BA-013 In Bitmap konvertieren
//! beschneidet sofort auf die Pixel mit Alpha > 0.

use crate::common;
use common::*;
use fp_model::ElementKind;
use serde_json::json;

#[test]
fn join_single_open_path_closes_it_and_clears_name() {
    let mut s = doc_session(400.0, 300.0);
    let v = ex(&mut s, "draw.line", json!({"from": [10, 10], "to": [100, 60]}));
    let id = fp_model::ElementId(v["id"].as_u64().expect("id"));
    ex(&mut s, "element.setName", json!({"ids": [id.0], "name": "OffenP"}));
    select(&mut s, &[id]);
    ex(&mut s, "modify.combinePaths.join", json!({}));
    let e = el(&s, id);
    assert!(matches!(&e.kind, ElementKind::Path(p) if p.contours.len() == 1 && p.contours[0].closed), "geschlossen");
    assert_eq!(e.common.name, None, "Name danach null wie im Original");
}

#[test]
fn join_single_closed_path_is_no_error_and_clears_name() {
    let mut s = doc_session(400.0, 300.0);
    let r = rect(&mut s, 10.0, 10.0, 60.0, 60.0);
    ex(&mut s, "element.setName", json!({"ids": [r.0], "name": "NamedP"}));
    select(&mut s, &[r]);
    ex(&mut s, "modify.combinePaths.join", json!({}));
    assert_eq!(el(&s, r).common.name, None);
    let b = bounds(&s, r);
    assert_eq!((b.left, b.top, b.right, b.bottom), (10.0, 10.0, 60.0, 60.0), "Form unverändert");
}

#[test]
fn flatten_crops_to_visible_pixels() {
    let mut s = doc_session(400.0, 300.0);
    let hidden = rect(&mut s, 130.0, 180.0, 220.0, 270.0);
    // drawn before the transparent fill: the selected `hidden` would hand it to the tools (Q-58)
    let vis = rect(&mut s, 150.0, 200.0, 200.0, 250.0);
    // unsichtbar über eine Füllung mit Alpha 0 (B-2: „Füllung #…00“)
    ex(&mut s, "element.setFill", json!({"ids": [hidden.0], "color": "#ff000000"}));
    ex(&mut s, "element.setStroke", json!({"ids": [vis.0], "none": true}));
    ex(&mut s, "element.setStroke", json!({"ids": [hidden.0], "none": true}));
    select(&mut s, &[vis, hidden]);
    let v = ex(&mut s, "modify.flattenSelection", json!({}));
    let id = fp_model::ElementId(v["id"].as_u64().expect("id"));
    assert!(matches!(el(&s, id).kind, ElementKind::Bitmap(_)));
    let b = bounds(&s, id);
    assert_eq!((b.left, b.top, b.right, b.bottom), (150.0, 200.0, 200.0, 250.0), "Grenzen = Alpha > 0 (Original 150,200–200,250)");
}
