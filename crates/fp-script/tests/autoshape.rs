//! Automatische Formen: Einfügen, Steuerpunkte ziehen, Parameter, Rückgängig – Werte nach
//! Beobachtung im Original (Einfügen bei (300, 300) in ein 600×600-Dokument).

use fp_engine::Session;
use fp_model::ElementKind;
use fp_script::autoshape;
use serde_json::{json, Value};

fn session() -> Session {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 600, "height": 600})).unwrap_or_else(|e| panic!("{e}"));
    s
}

fn shape(s: &Session, id: u64) -> fp_model::Element {
    let d = s.active().unwrap_or_else(|| panic!("kein Dokument"));
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, fp_model::ElementId(id)).unwrap_or_else(|| panic!("Element {id} fehlt"));
    (*e).clone()
}

fn nodes(e: &fp_model::Element) -> Vec<(f32, f32)> {
    let ElementKind::Group(g) = &e.kind else { panic!("keine Gruppe") };
    let ElementKind::Path(p) = &g.children[0].kind else { panic!("kein Pfad") };
    p.contours[0].nodes.iter().map(|n| (n.anchor.x, n.anchor.y)).collect()
}

fn cps(e: &fp_model::Element) -> Vec<(f32, f32)> {
    let ElementKind::Group(g) = &e.kind else { panic!("keine Gruppe") };
    g.smart_shape.as_ref().and_then(|s| s.control_points.as_ref()).map(|c| c.iter().map(|c| (c.position.x, c.position.y)).collect()).unwrap_or_default()
}

fn near(a: (f32, f32), b: (f32, f32)) -> bool {
    (a.0 - b.0).abs() < 0.02 && (a.1 - b.1).abs() < 0.02
}

fn id_of(v: &Value) -> u64 {
    v["id"].as_u64().unwrap_or_else(|| panic!("keine id: {v}"))
}

#[test]
fn rounded_rectangle_like_original() {
    let mut s = session();
    let v = autoshape::op_with_history(&mut s, &json!({"op": "insert", "shape": "Abgerundetes Rechteck", "at": [300, 300]})).unwrap_or_else(|e| panic!("{e}"));
    let id = id_of(&v);
    let e = shape(&s, id);
    let n = nodes(&e);
    assert_eq!(n.len(), 8);
    assert!(near(n[0], (300.0, 337.5)) && near(n[1], (337.5, 300.0)) && near(n[4], (500.0, 412.5)), "{n:?}");
    let c = cps(&e);
    assert!(near(c[0], (337.5, 300.0)) && near(c[4], (510.0, 460.0)), "{c:?}");
    assert_eq!(e.common.custom_data.get_str("shapeName"), Some("rectangle"));
    // Ecke ziehen: alle Ecken 54,5 (wie im Original)
    autoshape::op_with_history(&mut s, &json!({"op": "dragControlPoint", "id": id, "index": 0, "to": [354.5, 311]})).unwrap_or_else(|e| panic!("{e}"));
    let e = shape(&s, id);
    assert!(near(nodes(&e)[1], (354.5, 300.0)) && near(cps(&e)[1], (445.5, 300.0)), "{:?}", nodes(&e));
    // Größe: 217 × 161
    autoshape::op_with_history(&mut s, &json!({"op": "dragControlPoint", "id": id, "index": 4, "to": [527, 471]})).unwrap_or_else(|e| panic!("{e}"));
    let e = shape(&s, id);
    assert!(near(nodes(&e)[4], (517.0, 406.5)), "{:?}", nodes(&e));
    // Verlauf: 3 Schritte, Rückgängig stellt wieder her
    assert_eq!(s.active().map(|d| d.history.undo_count()), Some(3));
    s.undo().unwrap_or_else(|e| panic!("{e}"));
    assert!(near(nodes(&shape(&s, id))[4], (500.0, 395.5)));
    // Parameter setzen
    autoshape::op_with_history(&mut s, &json!({"op": "setParams", "id": id, "params": {"corner0": 0, "corner1": 0, "corner2": 0, "corner3": 0}})).unwrap_or_else(|e| panic!("{e}"));
    assert!(near(nodes(&shape(&s, id))[0], (300.0, 300.0)));
    let d = autoshape::op(&mut s, &json!({"op": "describe", "id": id})).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(d["label"], "Abgerundetes Rechteck");
    assert!(d["params"].as_array().is_some_and(|a| a.iter().any(|p| p["key"] == "width" && p["value"].as_f64() == Some(200.0))), "{d}");
}

#[test]
fn star_like_original() {
    let mut s = session();
    let id = id_of(&autoshape::op_with_history(&mut s, &json!({"op": "insert", "shape": "Star", "at": [300, 300]})).unwrap_or_else(|e| panic!("{e}")));
    let n = nodes(&shape(&s, id));
    assert_eq!(n.len(), 10);
    assert!(near(n[0], (400.0, 300.0)) && near(n[1], (429.39, 359.55)) && near(n[4], (458.78, 480.9)), "{n:?}");
    // Radius 1 ziehen
    autoshape::op_with_history(&mut s, &json!({"op": "dragControlPoint", "id": id, "index": 0, "to": [417, 311]})).unwrap_or_else(|e| panic!("{e}"));
    let n = nodes(&shape(&s, id));
    assert!(near(n[0], (400.0, 309.39)) && near(n[2], (486.17, 372.0)), "{n:?}");
    // Rundung 2 wie im Original (Innenpunkte auf 69,76 mit Henkeln)
    let mut s = session();
    let id = id_of(&autoshape::op_with_history(&mut s, &json!({"op": "insert", "shape": "Star", "at": [300, 300]})).unwrap_or_else(|e| panic!("{e}")));
    autoshape::op_with_history(&mut s, &json!({"op": "dragControlPoint", "id": id, "index": 3, "to": [464.55, 426.45]})).unwrap_or_else(|e| panic!("{e}"));
    let e = shape(&s, id);
    let ElementKind::Group(g) = &e.kind else { panic!() };
    let ElementKind::Path(p) = &g.children[0].kind else { panic!() };
    let n1 = &p.contours[0].nodes[1];
    assert!(near((n1.anchor.x, n1.anchor.y), (441.0, 343.56)) && near((n1.pred.x, n1.pred.y), (410.59, 321.47)), "{n1:?}");
    // Punkte: nach unten ziehen verringert die Zacken (5 → 3)
    autoshape::op_with_history(&mut s, &json!({"op": "dragControlPoint", "id": id, "index": 4, "to": [300, 528]})).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(nodes(&shape(&s, id)).len(), 6);
}

/// Punkte-Griff wie im Original (a1run tool-autoshapes 05, paths.json d05_star_points_*): nur
/// senkrecht; Radius 60: +40 → 3 Punkte, −80 → 34 Punkte, waagrecht keine Änderung; danach steht
/// der Griff bei Mitte + Radius − (n − 3) · 2 · Radius / 47 (3 → 220, 5 → 214,89, 34 → 140,85).
#[test]
fn star_points_handle_vertical_like_original() {
    let star = |s: &mut Session| id_of(&autoshape::op_with_history(s, &json!({"op": "insert", "shape": "Star", "rect": [100, 100, 220, 220]})).unwrap_or_else(|e| panic!("{e}")));
    let drag = |s: &mut Session, id: u64, to: [f32; 2]| {
        autoshape::op_with_history(s, &json!({"op": "dragControlPoint", "id": id, "index": 4, "to": to})).unwrap_or_else(|e| panic!("{e}"));
    };
    let mut s = session();
    let id = star(&mut s);
    assert!(near(cps(&shape(&s, id))[4], (100.0, 212.8)), "{:?}", cps(&shape(&s, id)));
    drag(&mut s, id, [140.0, 212.8]);
    assert_eq!(nodes(&shape(&s, id)).len(), 10, "waagrecht: keine Änderung");
    assert!(near(cps(&shape(&s, id))[4], (100.0, 214.89)), "{:?}", cps(&shape(&s, id)));
    let mut s = session();
    let id = star(&mut s);
    drag(&mut s, id, [100.0, 252.8]);
    assert_eq!(nodes(&shape(&s, id)).len(), 6, "+40 → 3 Punkte");
    assert!(near(cps(&shape(&s, id))[4], (100.0, 220.0)));
    let mut s = session();
    let id = star(&mut s);
    drag(&mut s, id, [100.0, 132.8]);
    assert_eq!(nodes(&shape(&s, id)).len(), 68, "−80 → 34 Punkte");
    assert!(near(cps(&shape(&s, id))[4], (100.0, 140.85)), "{:?}", cps(&shape(&s, id)));
}

#[test]
fn drag_insert_and_list() {
    let mut s = session();
    let id = id_of(&autoshape::op_with_history(&mut s, &json!({"op": "insert", "shape": "rounded rectangle", "rect": [10, 20, 110, 70]})).unwrap_or_else(|e| panic!("{e}")));
    let n = nodes(&shape(&s, id));
    assert!(near(n[2], (97.5, 20.0)), "{n:?}");
    let l = autoshape::list();
    assert!(l.as_array().is_some_and(|a| a.len() >= 2));
    assert!(autoshape::op(&mut s, &json!({"op": "insert", "shape": "gibt es nicht"})).is_err());
    // Verschieben mit edit.move, danach Steuerpunkt ziehen: Form bleibt an der neuen Stelle
    s.execute("edit.move", json!({"dx": 100, "dy": 0})).unwrap_or_else(|e| panic!("{e}"));
    let moved = nodes(&shape(&s, id));
    assert!(near(moved[0], (110.0, 32.5)), "{moved:?}");
    autoshape::op_with_history(&mut s, &json!({"op": "dragControlPoint", "id": id, "index": 4, "to": [230, 90]})).unwrap_or_else(|e| panic!("{e}"));
    let n = nodes(&shape(&s, id));
    assert!(near(n[0], (110.0, 32.5)) && near(n[4], (220.0, 67.5)), "{n:?}");
}

/// Logikfehler (AGENTEN_REGELN §4b): wiederholtes Setzen, mehrfaches Ziehen, Rückgängig/Wiederholen,
/// richtiges Ziel nach Auswahlwechsel.
#[test]
fn repeated_operations_do_not_accumulate() {
    let mut s = session();
    let a = id_of(&autoshape::op_with_history(&mut s, &json!({"op": "insert", "shape": "Star", "at": [10, 10]})).unwrap_or_else(|e| panic!("{e}")));
    let b = id_of(&autoshape::op_with_history(&mut s, &json!({"op": "insert", "shape": "Abgerundetes Rechteck", "at": [300, 300]})).unwrap_or_else(|e| panic!("{e}")));
    let count = |s: &Session| {
        let d = s.active().unwrap_or_else(|| panic!());
        fp_engine::docops::selectable_ids(&d.doc, d.page, d.frame).len()
    };
    let before = count(&s);
    // 50× denselben Parameter setzen: genau ein Pfad, gleiche Geometrie
    for _ in 0..50 {
        autoshape::op_with_history(&mut s, &json!({"op": "setParams", "id": a, "params": {"nodes": 7}})).unwrap_or_else(|e| panic!("{e}"));
    }
    let e = shape(&s, a);
    let ElementKind::Group(g) = &e.kind else { panic!() };
    assert_eq!(g.children.len(), 1);
    assert_eq!(nodes(&e).len(), 14);
    assert_eq!(cps(&e).len(), 6);
    assert_eq!(count(&s), before, "keine zusätzlichen Objekte");
    // 20× hin und her ziehen: Ergebnis wie einmal
    for k in 0..20 {
        let to = if k % 2 == 0 { [354.5, 300.0] } else { [337.5, 300.0] };
        let from = cps(&shape(&s, b))[0];
        autoshape::op_with_history(&mut s, &json!({"op": "dragControlPoint", "id": b, "index": 0, "from": [from.0, from.1], "to": to})).unwrap_or_else(|e| panic!("{e}"));
    }
    let e = shape(&s, b);
    assert!(near(nodes(&e)[1], (337.5, 300.0)), "{:?}", nodes(&e));
    assert!(e.common.custom_data.get("_drag").is_none(), "Ziehzustand bleibt nicht im Dokument");
    // Rückgängig/Wiederholen-Folge stellt exakt wieder her
    let snap = shape(&s, b);
    for _ in 0..5 {
        s.undo().unwrap_or_else(|e| panic!("{e}"));
    }
    for _ in 0..5 {
        s.redo().unwrap_or_else(|e| panic!("{e}"));
    }
    assert_eq!(shape(&s, b), snap);
    // Auswahlwechsel: Operation wirkt auf die angegebene Form, die andere bleibt unverändert
    s.execute("select.set", json!({"ids": [a]})).unwrap_or_else(|e| panic!("{e}"));
    let star = shape(&s, a);
    autoshape::op_with_history(&mut s, &json!({"op": "setParams", "id": b, "params": {"width": 120}})).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(shape(&s, a), star);
    assert!(near(nodes(&shape(&s, b))[4], (420.0, 412.5)));
}
/// Jede eingebaute Form: Einfügen (Punkt und Rahmen), jeden Steuerpunkt ziehen und anklicken,
/// Parameter setzen, alles rückgängig.
#[test]
fn every_shape_works() {
    for def in autoshape::SHAPES {
        let mut s = session();
        let id = id_of(&autoshape::op_with_history(&mut s, &json!({"op": "insert", "shape": def.en, "at": [200, 200]})).unwrap_or_else(|e| panic!("{}: {e}", def.en)));
        let e = shape(&s, id);
        let ElementKind::Group(g) = &e.kind else { panic!() };
        assert!(!g.children.is_empty(), "{}: keine Kinder", def.en);
        let ncp = cps(&e).len();
        assert!(ncp >= 1, "{}: keine Steuerpunkte", def.en);
        let b = fp_engine::docops::element_bounds(&s.active().unwrap_or_else(|| panic!()).doc, &e).unwrap_or_else(|| panic!("{}: keine Grenzen", def.en));
        assert!(b.width() > 1.0 || b.height() > 1.0, "{}: leer {b:?}", def.en);
        for i in 0..ncp {
            let Some(&c) = cps(&shape(&s, id)).get(i) else { break };
            autoshape::op_with_history(&mut s, &json!({"op": "dragControlPoint", "id": id, "index": i, "from": [c.0, c.1], "to": [c.0 + 17.0, c.1 + 11.0]})).unwrap_or_else(|e| panic!("{} cp{i}: {e}", def.en));
            let now = cps(&shape(&s, id));
            let k = i.min(now.len() - 1);
            let c = now[k];
            autoshape::op_with_history(&mut s, &json!({"op": "dragControlPoint", "id": id, "index": k, "from": [c.0, c.1], "to": [c.0, c.1]})).unwrap_or_else(|e| panic!("{} klick cp{i}: {e}", def.en));
        }
        let d = autoshape::op(&mut s, &json!({"op": "describe", "id": id})).unwrap_or_else(|e| panic!("{}: {e}", def.en));
        let first = d["params"][0].clone();
        if !first.is_null() {
            autoshape::op_with_history(&mut s, &json!({"op": "setParams", "id": id, "params": {first["key"].as_str().unwrap_or("x"): first["value"].clone()}})).unwrap_or_else(|e| panic!("{}: {e}", def.en));
        }
        while s.undo().is_ok() {}
        let r = autoshape::op_with_history(&mut s, &json!({"op": "insert", "shape": def.de, "rect": [50, 60, 170, 140]})).unwrap_or_else(|e| panic!("{}: {e}", def.de));
        let e = shape(&s, id_of(&r));
        let b = fp_engine::docops::element_bounds(&s.active().unwrap_or_else(|| panic!()).doc, &e).unwrap_or_else(|| panic!());
        // Dateiinfo und Perspektive richten sich nach der Leinwand, nicht nach dem Rahmen
        assert!(["File Info", "Perspective"].contains(&def.en) || b.right > 50.0 && b.left < 170.0 && b.bottom > 60.0 && b.top < 140.0, "{}: Rahmen {b:?}", def.en);
    }
}

/// Jede Parameterbeschriftung und jeder Auswahlwert hat einen Schlüssel (`labelKey`/`optionKeys`,
/// UE-1); Deutsch = bisherige Beschriftung (Vollständigkeit de/en prüft fp-i18n).
#[test]
fn param_labels_have_translation_keys() {
    for sd in autoshape::SHAPES {
        for p in sd.params {
            let key = p.label_key().unwrap_or_else(|| panic!("{}: Beschriftung „{}“ ohne Schlüssel", sd.en, p.label));
            fp_i18n::with_language("de", || assert_eq!(fp_i18n::t(key), p.label, "{key}"));
            fp_i18n::with_language("en", || assert_ne!(fp_i18n::t(key), key));
            for o in p.options {
                if !["px", "inch", "cm"].contains(o) {
                    assert!(autoshape::shapes::option_key(o).is_some(), "{}: Auswahlwert „{o}“ ohne Schlüssel", sd.en);
                }
            }
        }
    }
    let v = autoshape::list();
    let first = &v[0]["params"][0];
    assert!(first["labelKey"].as_str().is_some_and(|k| k.starts_with("panel-asp-p-")), "{first}");
}

/// FIX-R1: Runde Formen aus dem Ziehrahmen wie im Original (a1run tool-autoshapes 02, paths.json):
/// Zug 100,100 → 220,200 (120 × 100) ergibt Radius 60 und Mitte 160,160 (halbe längere Seite, Kreis
/// ab dem Startpunkt in Zugrichtung). Senkrechter Zug ohne Breite (b05): 100,212.8 → +40 ergibt
/// Radius 20, Mitte 120,232.8; 160,100 → −20 ergibt Radius 10, Mitte 150,90.
#[test]
fn radial_shapes_from_frame_like_original() {
    let ins = |s: &mut Session, name: &str, rect: [f32; 4], from: Option<[f32; 2]>| {
        let mut p = json!({"op": "insert", "shape": name, "rect": rect});
        if let Some(f) = from {
            p["from"] = json!(f);
        }
        let id = id_of(&autoshape::op_with_history(s, &p).unwrap_or_else(|e| panic!("{e}")));
        shape(s, id)
    };
    let mut s = session();
    // Stern: Radius 1 bei 160,100, Mitte (unsichtbarer Griff 5) 160,160, Grenzen 102,100–218,209
    let e = ins(&mut s, "Stern", [100.0, 100.0, 220.0, 200.0], Some([100.0, 100.0]));
    let c = cps(&e);
    assert!(near(c[0], (160.0, 100.0)) && near(c[5], (160.0, 160.0)) && near(c[4], (100.0, 212.8)), "{c:?}");
    assert_eq!(e.common.custom_data.get_str("radius0").and_then(|v| v.parse::<f64>().ok()), Some(60.0));
    // ohne `from` (Skript-Aufruf mit Rahmen) gleich
    let c = cps(&ins(&mut s, "Star", [100.0, 100.0, 220.0, 200.0], None));
    assert!(near(c[0], (160.0, 100.0)) && near(c[5], (160.0, 160.0)), "{c:?}");
    // Torte/Donut 120 × 120 (Griffe 160,160 / 220,160 bzw. Innenradius 184,160)
    let c = cps(&ins(&mut s, "Torte", [100.0, 100.0, 220.0, 200.0], None));
    assert!(near(c[0], (160.0, 160.0)) && near(c[3], (220.0, 160.0)), "{c:?}");
    let c = cps(&ins(&mut s, "Donut", [100.0, 100.0, 220.0, 200.0], None));
    assert!(near(c[0], (160.0, 160.0)) && near(c[1], (184.0, 160.0)) && near(c[2], (160.0, 100.0)), "{c:?}");
    // Intelligentes Polygon: Mitte 160,160, Ecke oben 160,100 (Griff-Reihenfolge hier eigene)
    let c = cps(&ins(&mut s, "Intelligentes Polygon", [100.0, 100.0, 220.0, 200.0], None));
    assert!(c.iter().any(|&p| near(p, (160.0, 160.0))) && c.iter().any(|&p| near(p, (160.0, 100.0))), "{c:?}");
    // Spirale: Mitte 160,160, Radius 60
    let e = ins(&mut s, "Spirale", [100.0, 100.0, 220.0, 200.0], None);
    assert!(near(cps(&e)[0], (160.0, 160.0)), "{:?}", cps(&e));
    assert_eq!(e.common.custom_data.get_str("radius").and_then(|v| v.parse::<f64>().ok()), Some(60.0));
    // senkrechte Züge ohne Breite (Original b05_star_points_drag40 / b05_star_r0_up20)
    let c = cps(&ins(&mut s, "Stern", [100.0, 212.8, 100.0, 252.8], Some([100.0, 212.8])));
    assert!(near(c[0], (120.0, 212.8)) && near(c[5], (120.0, 232.8)), "{c:?}");
    let c = cps(&ins(&mut s, "Stern", [160.0, 80.0, 160.0, 100.0], Some([160.0, 100.0])));
    assert!(near(c[0], (150.0, 80.0)) && near(c[5], (150.0, 90.0)), "{c:?}");
    // Zug nach links oben: Kreis liegt links oberhalb des Startpunkts
    let c = cps(&ins(&mut s, "Stern", [100.0, 100.0, 220.0, 200.0], Some([220.0, 200.0])));
    assert!(near(c[5], (160.0, 140.0)), "{c:?}");
}
