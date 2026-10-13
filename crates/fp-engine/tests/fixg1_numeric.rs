//! FIX-G1 (TW-01): „Numerisch transformieren“ mit „Attribute skalieren“ skaliert den Strich mit
//! (√(sx·sy) wie Info-Bedienfeld/Inspektor), ohne Haken bleibt er; Drehen ändert ihn nie.

use fp_engine::Session;
use fp_model::{Element, ElementId};
use serde_json::json;
use std::sync::Arc;

fn session_with_stroke() -> (Session, u64) {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 400, "height": 300, "background": "#ffffff"})).expect("Dokument");
    let id = s.execute("draw.ellipse", json!({"rect": [50, 50, 150, 120]})).expect("Ellipse").value["id"].as_u64().expect("id");
    s.execute("pi.stroke", json!({"preset": "Einfach/Präzise abger.", "width": 8, "color": "#ff0000"})).expect("Strich");
    (s, id)
}

fn el(s: &Session, id: u64) -> Arc<Element> {
    let d = s.active().expect("Dokument");
    fp_engine::docops::element_on_page(&d.doc, d.page, ElementId(id)).expect("Element")
}

fn diameter(s: &Session, id: u64) -> i32 {
    fp_engine::docops::path_attrs_of(&el(s, id)).and_then(|a| a.brush.clone()).expect("Pinsel").diameter as i32
}

#[test]
fn scale_attributes_scales_the_stroke() {
    let (mut s, id) = session_with_stroke();
    s.execute("modify.transform.numericTransform", json!({"mode": "scale", "scaleX": 200, "scaleY": 200, "scaleAttributes": true})).expect("skalieren");
    assert_eq!(diameter(&s, id), 16, "8 · 2");
    s.execute("edit.undo", json!({})).expect("undo");
    assert_eq!(diameter(&s, id), 8, "ein Schritt");
    s.execute("modify.transform.numericTransform", json!({"mode": "scale", "scaleX": 200, "scaleY": 200, "scaleAttributes": false})).expect("skalieren");
    assert_eq!(diameter(&s, id), 8, "ohne Haken bleibt der Strich");
    s.execute("modify.transform.numericTransform", json!({"mode": "rotate", "angle": 30, "scaleAttributes": true})).expect("drehen");
    assert_eq!(diameter(&s, id), 8, "Drehen ändert den Strich nicht");
}

#[test]
fn resize_with_scale_attributes() {
    let (mut s, id) = session_with_stroke();
    let b = fp_engine::docops::ids_bounds(&s.active().expect("Dokument").doc, 0, &[ElementId(id)]).expect("Grenzen");
    s.execute("modify.transform.numericTransform", json!({"mode": "resize", "width": b.width() * 2.0, "height": b.height() * 2.0})).expect("Größe");
    assert_eq!(diameter(&s, id), 16, "Vorgabe: Attribute skalieren ein");
}
