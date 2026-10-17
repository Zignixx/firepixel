//! T-6 (re 31 §5): Maske als Attribut des Elements. Sicht `Element::element_mask`, Verschieben mit
//! und ohne Verknüpfung, „Entkoppeln“ (Kettensymbol), „Als Maske einfügen“ wählt die Maske (RT-3 S5),
//! Maskenoptionen über die gewählte Maske.

use crate::common;
use common::*;
use fp_engine::Session;
use fp_model::{ElementId, ElementKind};
use serde_json::json;

/// R1 50,50–150,120 (schwarz) als Ziel, E1 100,80–200,150 (grün) als Inhalt: Innen einfügen.
fn pasted() -> (Session, ElementId, ElementId, ElementId) {
    let mut s = doc_session(400.0, 300.0);
    let r1 = ElementId(ex(&mut s, "draw.rectangle", json!({"rect": [50, 50, 150, 120], "fillColor": "#000000", "strokeWidth": 0}))["id"].as_u64().unwrap_or(0));
    let e1 = ElementId(ex(&mut s, "draw.rectangle", json!({"rect": [100, 80, 200, 150], "fillColor": "#40ff00", "strokeWidth": 0}))["id"].as_u64().unwrap_or(0));
    select(&mut s, &[e1]);
    ex(&mut s, "edit.cut", json!({}));
    select(&mut s, &[r1]);
    let g = ElementId(ex(&mut s, "edit.pasteInside", json!({}))["id"].as_u64().unwrap_or(0));
    (s, g, r1, e1)
}

fn mask_bounds(s: &Session, g: ElementId) -> fp_geom::Rect {
    let e = el(s, g);
    let d = s.active().expect("d");
    fp_engine::docops::element_bounds(&d.doc, e.element_mask().expect("Maske").element).expect("Grenzen")
}

fn content_bounds(s: &Session, g: ElementId) -> fp_geom::Rect {
    let e = el(s, g);
    let d = s.active().expect("d");
    fp_engine::docops::element_bounds(&d.doc, e.masked_content()).expect("Grenzen")
}

/// §5.1/§5.2: ein Element (Inhalt E1) mit Maske R1, Flagbyte 0x17, nichts gewählt.
#[test]
fn paste_inside_gives_element_with_mask_attribute() {
    let (s, g, r1, _) = pasted();
    let e = el(&s, g);
    let m = e.element_mask().expect("Maske als Attribut");
    assert!(m.single && m.to_path && m.linked && m.enabled && m.show_attrs && !m.auto_expand);
    assert_eq!(m.element.id, r1);
    assert_eq!(e.js_type(), "RectanglePrimitive", "Klasse des Inhalts");
    assert_eq!(content_bounds(&s, g), fp_geom::Rect::new(100.0, 80.0, 200.0, 150.0));
    assert_eq!(top_elements(&s).len(), 1);
    assert!(s.active().is_some_and(|d| d.selection.elements.is_empty()));
}

/// §5.4: verknüpft bewegt sich die Maske mit; „Entkoppeln“ hebt nur die Verknüpfung auf (ein Objekt
/// bleibt), danach bewegt sich nur der Inhalt. Verlaufsschritte wie im Original benannt.
#[test]
fn linked_mask_moves_unlinked_stays() {
    let (mut s, g, _, _) = pasted();
    select(&mut s, &[g]);
    ex(&mut s, "edit.move", json!({"dx": 10, "dy": 5}));
    assert_eq!(mask_bounds(&s, g), fp_geom::Rect::new(60.0, 55.0, 160.0, 125.0));
    assert_eq!(content_bounds(&s, g), fp_geom::Rect::new(110.0, 85.0, 210.0, 155.0));
    ex(&mut s, "modify.mask.link", json!({"ids": [g.0]}));
    assert_eq!(s.active().and_then(|d| d.history.undo_label().map(str::to_string)).as_deref(), Some("Verknüpfung der Maske aufheben"));
    let e = el(&s, g);
    assert!(!e.element_mask().expect("Maske").linked);
    assert_eq!(top_elements(&s).len(), 1, "Entkoppeln löst nicht auf");
    select(&mut s, &[g]);
    ex(&mut s, "edit.move", json!({"dx": 20, "dy": 0}));
    assert_eq!(mask_bounds(&s, g), fp_geom::Rect::new(60.0, 55.0, 160.0, 125.0), "Maske bleibt stehen");
    assert_eq!(content_bounds(&s, g), fp_geom::Rect::new(130.0, 85.0, 230.0, 155.0));
    ex(&mut s, "modify.mask.link", json!({"ids": [g.0]}));
    assert_eq!(s.active().and_then(|d| d.history.undo_label().map(str::to_string)).as_deref(), Some("Maske verknüpfen"));
    assert!(el(&s, g).element_mask().expect("Maske").linked);
}

/// RT-3 S5 / re 28 S5: „Als Maske einfügen“ – danach ist die Maske gewählt, „Füllung und
/// Pinselstrich anzeigen“ aus. Maskenoptionen wirken über die gewählte Maske auf das Element.
#[test]
fn paste_as_mask_selects_mask() {
    let mut s = doc_session(400.0, 300.0);
    let a = ElementId(ex(&mut s, "draw.rectangle", json!({"rect": [50, 50, 150, 120]}))["id"].as_u64().unwrap_or(0));
    let b = ElementId(ex(&mut s, "draw.ellipse", json!({"rect": [60, 60, 140, 110]}))["id"].as_u64().unwrap_or(0));
    select(&mut s, &[b]);
    ex(&mut s, "edit.cut", json!({}));
    select(&mut s, &[a]);
    ex(&mut s, "edit.pasteAsMask", json!({}));
    let d = s.active().expect("d");
    assert_eq!(d.selection.elements.len(), 1);
    let sel = d.selection.elements[0];
    let owner = fp_engine::docops::mask_owner(&d.doc, d.page, sel).expect("gewählt ist eine Maske");
    let m = owner.element_mask().expect("Maske");
    assert_eq!(m.element.id, sel);
    assert!(!m.show_attrs && m.linked && m.enabled);
    assert_eq!(owner.masked_content().id, a);
    // Inspektor-Optionen auf der gewählten Maske
    ex(&mut s, "modify.mask.options", json!({"showAttrs": true}));
    let d = s.active().expect("d");
    let owner = fp_engine::docops::mask_owner(&d.doc, d.page, sel).expect("Maske");
    assert!(owner.element_mask().expect("Maske").show_attrs);
    // Element wieder wählen (Klick auf Miniatur/Name)
    let oid = owner.id;
    select(&mut s, &[oid]);
    assert!(fp_engine::docops::mask_owner(&s.active().expect("d").doc, 0, oid).is_none());
}

/// Klick auf die Masken-Miniatur (`select.set` mit der Masken-ID) wählt die Maske; Verschieben
/// bewegt dann nur die Maske.
#[test]
fn selecting_mask_moves_only_mask() {
    let (mut s, g, r1, _) = pasted();
    ex(&mut s, "select.set", json!({"ids": [r1.0]}));
    assert_eq!(s.active().map(|d| d.selection.elements.clone()), Some(vec![r1]));
    ex(&mut s, "edit.move", json!({"dx": 5, "dy": 0}));
    assert_eq!(mask_bounds(&s, g), fp_geom::Rect::new(55.0, 50.0, 155.0, 120.0));
    assert_eq!(content_bounds(&s, g), fp_geom::Rect::new(100.0, 80.0, 200.0, 150.0));
    assert!(matches!(el(&s, g).kind, ElementKind::Group(_)));
}
