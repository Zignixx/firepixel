//! F9: TW-34 Einfügen legt die Kopien direkt über das gewählte Objekt (Lauf `menu-edit/04`),
//! TW-35 Als Maske gruppieren nimmt den Platz des untersten gewählten Objekts ein
//! (`menu-modify-a/16`, `menu-modify-b/20`).

use crate::common::{doc_session, ellipse, ex, rect, select, top_elements};
use fp_model::ElementId;
use serde_json::json;

fn order(s: &fp_engine::Session) -> Vec<ElementId> {
    top_elements(s).iter().map(|e| e.id).collect()
}

/// Strg+C, 2× Strg+V auf dem untersten Objekt: die Kopien liegen direkt darüber, an derselben Stelle.
#[test]
fn paste_goes_directly_above_selection() {
    let mut s = doc_session(800.0, 600.0);
    let r1 = rect(&mut s, 50.0, 50.0, 150.0, 120.0);
    let r2 = rect(&mut s, 200.0, 50.0, 300.0, 120.0);
    let e1 = ellipse(&mut s, 50.0, 170.0, 150.0, 240.0);
    select(&mut s, &[r1]);
    ex(&mut s, "edit.copy", json!({}));
    let a = ex(&mut s, "edit.paste", json!({}));
    let b = ex(&mut s, "edit.paste", json!({}));
    let (a, b) = (ElementId(a["ids"][0].as_u64().unwrap_or(0)), ElementId(b["ids"][0].as_u64().unwrap_or(0)));
    assert_eq!(order(&s), vec![e1, r2, b, a, r1]);
    let d = s.active().expect("Dokument");
    let bb = fp_engine::docops::ids_bounds(&d.doc, d.page, &[b]).expect("Grenzen");
    assert_eq!((bb.left, bb.top), (50.0, 50.0));
}

/// Ohne Auswahl bleibt es beim Einfügen oben in der Ebene.
#[test]
fn paste_without_selection_goes_on_top() {
    let mut s = doc_session(800.0, 600.0);
    let r1 = rect(&mut s, 50.0, 50.0, 150.0, 120.0);
    let r2 = rect(&mut s, 200.0, 50.0, 300.0, 120.0);
    select(&mut s, &[r1]);
    ex(&mut s, "edit.copy", json!({}));
    select(&mut s, &[]);
    let a = ex(&mut s, "edit.paste", json!({}));
    let a = ElementId(a["ids"][0].as_u64().unwrap_or(0));
    assert_eq!(order(&s), vec![a, r2, r1]);
}

/// Mehrere Objekte im Zwischenspeicher behalten ihre Reihenfolge über dem gewählten Objekt.
#[test]
fn paste_several_keeps_order_above_selection() {
    let mut s = doc_session(800.0, 600.0);
    let r1 = rect(&mut s, 50.0, 50.0, 150.0, 120.0);
    let r2 = rect(&mut s, 200.0, 50.0, 300.0, 120.0);
    let e1 = ellipse(&mut s, 50.0, 170.0, 150.0, 240.0);
    select(&mut s, &[r2, e1]);
    ex(&mut s, "edit.copy", json!({}));
    select(&mut s, &[r1]);
    let v = ex(&mut s, "edit.paste", json!({}));
    let ids: Vec<ElementId> = v["ids"].as_array().expect("ids").iter().map(|x| ElementId(x.as_u64().unwrap_or(0))).collect();
    assert_eq!(ids.len(), 2);
    assert_eq!(order(&s), vec![e1, r2, ids[0], ids[1], r1]);
}

/// Als Maske gruppieren (E1 über R1, R2 dazwischen): die Gruppe liegt an R1s Platz unter R2.
#[test]
fn group_as_mask_takes_place_of_lowest() {
    let mut s = doc_session(800.0, 600.0);
    let r1 = rect(&mut s, 50.0, 50.0, 150.0, 120.0);
    let r2 = rect(&mut s, 200.0, 50.0, 300.0, 120.0);
    let e1 = ellipse(&mut s, 50.0, 170.0, 150.0, 240.0);
    let p1 = rect(&mut s, 200.0, 170.0, 420.0, 240.0);
    // Stapel wie in par_vector: E1 unter R2, aber über R1
    select(&mut s, &[e1]);
    ex(&mut s, "modify.arrange.sendBackward", json!({}));
    assert_eq!(order(&s), vec![p1, r2, e1, r1]);
    select(&mut s, &[r1, e1]);
    let g = ex(&mut s, "modify.mask.groupAsMask", json!({}));
    let g = ElementId(g["id"].as_u64().unwrap_or(0));
    assert_eq!(order(&s), vec![p1, r2, g]);
}

/// Normale Gruppe bleibt am Platz des obersten Objekts.
#[test]
fn group_takes_place_of_topmost() {
    let mut s = doc_session(800.0, 600.0);
    let r1 = rect(&mut s, 50.0, 50.0, 150.0, 120.0);
    let r2 = rect(&mut s, 200.0, 50.0, 300.0, 120.0);
    let e1 = ellipse(&mut s, 50.0, 170.0, 150.0, 240.0);
    select(&mut s, &[r1, r2]);
    let g = ex(&mut s, "modify.group", json!({}));
    let g = ElementId(g["id"].as_u64().unwrap_or(0));
    assert_eq!(order(&s), vec![e1, g]);
}
