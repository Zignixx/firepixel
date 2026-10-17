//! Automatische Formen nach re 36 (K-3): Bindung beim Aufziehen, Seitenverhältnis-Sperre, Umschalt
//! quadriert freie Formen (TW-88, ersetzt TW-82), logarithmische Spirale (TW-81), Intelligentes Polygon und L-Form (TW-83).
//! Messwerte aus Lauf 8 (Zug 100,100 → 220,200, 120 × 100) und den Beispielwerten in re 36.

use fp_engine::Session;
use fp_model::{ContourNode, ElementKind};
use fp_script::autoshape;
use serde_json::{Value, json};

fn session() -> Session {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 800, "height": 600}))
        .unwrap_or_else(|e| panic!("{e}"));
    s
}

fn insert(s: &mut Session, p: Value) -> fp_model::Element {
    let v = autoshape::op_with_history(s, &p).unwrap_or_else(|e| panic!("{e}"));
    element(
        s,
        v["id"].as_u64().unwrap_or_else(|| panic!("keine id: {v}")),
    )
}

fn element(s: &Session, id: u64) -> fp_model::Element {
    let d = s.active().unwrap_or_else(|| panic!("kein Dokument"));
    (*fp_engine::docops::element_on_page(&d.doc, d.page, fp_model::ElementId(id))
        .unwrap_or_else(|| panic!("Element {id} fehlt")))
    .clone()
}

fn contour(e: &fp_model::Element) -> (Vec<ContourNode>, bool) {
    let ElementKind::Group(g) = &e.kind else {
        panic!("keine Gruppe")
    };
    let ElementKind::Path(p) = &g.children[0].kind else {
        panic!("kein Pfad")
    };
    (p.contours[0].nodes.clone(), p.contours[0].closed)
}

fn anchors(e: &fp_model::Element) -> Vec<(f32, f32)> {
    contour(e)
        .0
        .iter()
        .map(|n| (n.anchor.x, n.anchor.y))
        .collect()
}

fn cps(e: &fp_model::Element) -> Vec<(f32, f32, String)> {
    let ElementKind::Group(g) = &e.kind else {
        panic!("keine Gruppe")
    };
    g.smart_shape
        .as_ref()
        .and_then(|s| s.control_points.as_ref())
        .map(|c| {
            c.iter()
                .map(|c| {
                    (
                        c.position.x,
                        c.position.y,
                        c.tooltip.clone().unwrap_or_default(),
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Ausdehnung der Bézierkurve (abgetastet), ohne Strich.
fn curve_bounds(e: &fp_model::Element) -> (f32, f32, f32, f32) {
    let (n, closed) = contour(e);
    let mut b = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    let segs = if closed { n.len() } else { n.len() - 1 };
    for i in 0..segs {
        let (p0, p3) = (&n[i], &n[(i + 1) % n.len()]);
        let (a, c1, c2, d) = (p0.anchor, p0.succ, p3.pred, p3.anchor);
        for k in 0..=200 {
            let t = k as f32 / 200.0;
            let u = 1.0 - t;
            let x =
                u * u * u * a.x + 3.0 * u * u * t * c1.x + 3.0 * u * t * t * c2.x + t * t * t * d.x;
            let y =
                u * u * u * a.y + 3.0 * u * u * t * c1.y + 3.0 * u * t * t * c2.y + t * t * t * d.y;
            b = (b.0.min(x), b.1.min(y), b.2.max(x), b.3.max(y));
        }
    }
    b
}

fn near(a: f32, b: f32, tol: f32) -> bool {
    (a - b).abs() <= tol
}

fn num(e: &fp_model::Element, key: &str) -> f64 {
    e.common
        .custom_data
        .get_str(key)
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(f64::NAN)
}

const DRAG: [f32; 4] = [100.0, 100.0, 220.0, 200.0];

#[test]
fn spiral_nominal_is_logarithmic() {
    let mut s = session();
    let e = insert(
        &mut s,
        json!({"op": "insert", "shape": "Spirale", "at": [100, 100]}),
    );
    let (n, closed) = contour(&e);
    assert!(!closed);
    assert_eq!(n.len(), 27, "27 Knoten");
    // erster Knoten rechts der Mitte bei r = 0,5, Ende auf r = 100
    assert!(
        near(n[0].anchor.x, 200.5, 1e-3) && near(n[0].anchor.y, 200.0, 1e-3),
        "{:?}",
        n[0].anchor
    );
    let last = n[26].anchor;
    assert!(near(
        ((last.x - 200.0).powi(2) + (last.y - 200.0).powi(2)).sqrt(),
        100.0,
        1e-2
    ));
    // Nennspirale in der 200er-Box: x 22,2…172,9, y 40,8…189,9 (unten hier 189,74: Abtastung der
    // Bézierkurve; re 36 nennt ≈ 189,9)
    let b = curve_bounds(&e);
    assert!(
        near(b.0, 122.2, 0.1)
            && near(b.2, 272.9, 0.1)
            && near(b.1, 140.8, 0.1)
            && near(b.3, 289.9, 0.2),
        "{b:?}"
    );
    let c = cps(&e);
    assert_eq!(c.len(), 3);
    assert_eq!(c[2].2, "Spiralen: 6");
    // Windungsgriff: x = M.x − R, d = −0,53846
    assert!(
        near(c[2].0, 100.0, 1e-3) && near(c[2].1, 200.0 - 53.846, 1e-2),
        "{c:?}"
    );
}

#[test]
fn spiral_drag_scales_nominal_box_120() {
    let mut s = session();
    let e = insert(
        &mut s,
        json!({"op": "insert", "shape": "Spiral", "rect": DRAG, "from": [100, 100]}),
    );
    assert_eq!(contour(&e).0.len(), 27, "skaliert, nicht neu berechnet");
    assert_eq!(num(&e, "radius"), 60.0);
    // Lauf 8 `03_handles_spiral`: 113,3…203,7 × 124,5…213,9
    let b = curve_bounds(&e);
    assert!(
        near(b.0, 113.3, 0.15)
            && near(b.2, 203.7, 0.15)
            && near(b.1, 124.5, 0.15)
            && near(b.3, 213.9, 0.15),
        "{b:?}"
    );
    assert_eq!(cps(&e)[2].2, "Spiralen: 6");
}

#[test]
fn spiral_turns_handle_recomputes_with_a_half() {
    let mut s = session();
    let e = insert(
        &mut s,
        json!({"op": "insert", "shape": "Spiral", "rect": DRAG, "from": [100, 100]}),
    );
    let id = e.id.0;
    let h = cps(&e)[2].clone();
    // Lauf 8 `05_handle_drag_spiral_turns`: Griff +40 → R 60, b ≈ 75°, 13 Knoten, ≈ 3 Windungen
    autoshape::op_with_history(&mut s, &json!({"op": "dragControlPoint", "id": id, "index": 2, "from": [h.0, h.1], "to": [h.0, h.1 + 40.0]})).unwrap_or_else(|e| panic!("{e}"));
    let e = element(&s, id);
    assert_eq!(contour(&e).0.len(), 13);
    assert_eq!(cps(&e)[2].2, "Spiralen: 3");
    // Griff nur senkrecht und auf M.y ± R begrenzt
    let h = cps(&e)[2].clone();
    autoshape::op_with_history(&mut s, &json!({"op": "dragControlPoint", "id": id, "index": 2, "from": [h.0, h.1], "to": [h.0 + 30.0, h.1 + 500.0]})).unwrap_or_else(|e| panic!("{e}"));
    let c = cps(&element(&s, id));
    assert!(
        near(c[2].0, 100.0, 1e-3) && near(c[2].1, 220.0, 1e-3),
        "{c:?}"
    );
    // Klick auf die Mitte: geschlossen und wieder offen
    autoshape::op_with_history(&mut s, &json!({"op": "dragControlPoint", "id": id, "index": 0, "from": [160, 160], "to": [160, 160]})).unwrap_or_else(|e| panic!("{e}"));
    assert!(contour(&element(&s, id)).1, "geschlossen");
    autoshape::op_with_history(&mut s, &json!({"op": "dragControlPoint", "id": id, "index": 0, "from": [160, 160], "to": [160, 160]})).unwrap_or_else(|e| panic!("{e}"));
    assert!(!contour(&element(&s, id)).1, "offen");
}

#[test]
fn smart_polygon_seven_corners_box_120() {
    let mut s = session();
    let e = insert(
        &mut s,
        json!({"op": "insert", "shape": "Intelligentes Polygon", "rect": DRAG, "from": [100, 100]}),
    );
    let (n, closed) = contour(&e);
    assert!(closed);
    assert_eq!(n.len(), 7);
    assert!(
        n.iter().all(|k| k.pred == k.anchor && k.succ == k.anchor),
        "ohne Henkel"
    );
    assert!(
        near(n[0].anchor.x, 160.0, 1e-3) && near(n[0].anchor.y, 100.0, 1e-3),
        "erste Ecke oben"
    );
    assert!(n[1].anchor.x > 160.0, "im Uhrzeigersinn");
    let (x0, x1) = n.iter().fold((f32::MAX, f32::MIN), |a, k| {
        (a.0.min(k.anchor.x), a.1.max(k.anchor.x))
    });
    let (y0, y1) = n.iter().fold((f32::MAX, f32::MIN), |a, k| {
        (a.0.min(k.anchor.y), a.1.max(k.anchor.y))
    });
    // Lauf 8: 101…218 × 100…214 (re 36: 101,5…218,5 × 100…214,06)
    assert!(
        near(x0, 101.5, 0.05)
            && near(x1, 218.5, 0.05)
            && near(y0, 100.0, 0.05)
            && near(y1, 214.06, 0.05),
        "{x0} {x1} {y0} {y1}"
    );
}

#[test]
fn smart_polygon_corner_handle_formula() {
    let mut s = session();
    let e = insert(
        &mut s,
        json!({"op": "insert", "shape": "Smart Polygon", "at": [100, 100]}),
    );
    let id = e.id.0;
    let c = cps(&e);
    assert_eq!(c.len(), 5);
    // Ecken-Griff beim Anlegen (M.x − R, M.y + 0,6 R) = (100, 260)
    assert!(
        near(c[3].0, 100.0, 1e-3) && near(c[3].1, 260.0, 1e-3),
        "{c:?}"
    );
    let drag_to = |s: &mut Session, y: f32| {
        let h = cps(&element(s, id))[3].clone();
        autoshape::op_with_history(s, &json!({"op": "dragControlPoint", "id": id, "index": 3, "from": [h.0, h.1], "to": [h.0, y]})).unwrap_or_else(|e| panic!("{e}"));
        anchors(&element(s, id)).len()
    };
    // N = round(22·(M.y + R − y)/2R) + 3
    assert_eq!(drag_to(&mut s, 50.0), 25, "ganz oben");
    assert_eq!(drag_to(&mut s, 400.0), 3, "ganz unten");
    assert_eq!(drag_to(&mut s, 200.0), 14, "Mitte: 11 + 3");
    // halbe Werte runden nach +∞ wie JS: 22·(300 − y)/200 = 5,5 → y = 250 → 6 + 3
    assert_eq!(drag_to(&mut s, 250.0), 9);
}

#[test]
fn l_shape_click_and_drag() {
    let mut s = session();
    let e = insert(
        &mut s,
        json!({"op": "insert", "shape": "L-Form", "at": [100, 100]}),
    );
    let n = anchors(&e);
    assert_eq!(n.len(), 8);
    assert_eq!(n[1], (300.0, 100.0));
    assert_eq!(n[2], (300.0, 200.0));
    assert_eq!(n[3], (200.0, 200.0));
    assert_eq!(n[5], (200.0, 300.0));
    // Zug 120 × 100: Außenmaße = Zugrechteck, Dicke = 100/3 (Lauf 8: 33 px)
    let e = insert(
        &mut s,
        json!({"op": "insert", "shape": "L-Shape", "rect": DRAG, "from": [100, 100]}),
    );
    let n = anchors(&e);
    let t = 100.0 / 3.0;
    assert!(near(n[1].0, 220.0, 1e-3) && near(n[1].1, 100.0, 1e-3));
    assert!(
        near(n[2].0, 220.0, 1e-3) && near(n[2].1, 100.0 + t, 1e-3),
        "{n:?}"
    );
    assert!(
        near(n[3].0, 100.0 + t, 1e-3) && near(n[3].1, 100.0 + t, 1e-3),
        "{n:?}"
    );
    assert!(
        near(n[5].0, 100.0 + t, 1e-3) && near(n[5].1, 200.0, 1e-3),
        "{n:?}"
    );
    assert!(
        near(n[6].0, 100.0, 1e-3) && near(n[6].1, 200.0, 1e-3),
        "{n:?}"
    );
    let c = cps(&e);
    // Griffe: Dicke (x + T, y + H), Breite (x + W, y + T/2), Höhe (x + T/2, y + H)
    assert!(
        near(c[1].0, 100.0 + t, 1e-3) && near(c[1].1, 200.0, 1e-3),
        "{c:?}"
    );
    assert!(
        near(c[2].0, 220.0, 1e-3) && near(c[2].1, 100.0 + t / 2.0, 1e-3),
        "{c:?}"
    );
    assert!(
        near(c[3].0, 100.0 + t / 2.0, 1e-3) && near(c[3].1, 200.0, 1e-3),
        "{c:?}"
    );
}

/// TW-88 (ersetzt TW-82): Umschalt beim Aufziehen macht freie Formen zum Quadrat mit der längeren
/// Seite, gesperrte Formen ändert es nicht. re 36 §1.4 („Umschalt ohne Wirkung“) stützte sich auf
/// Lauf 8 (115 × 58), ein Artefakt des alten Läufer-Zug-Endes; Lauf 9 misst mit dem Zug
/// 100,100 → 220,160 im Original abgerundetes Rechteck und Pfeil 100,100–221,221 (121 × 121 Pixel
/// mit Strich, also Rahmen 120 × 120), den Stern unverändert.
#[test]
fn shift_squares_free_shapes_and_aspect_lock() {
    let mut s = session();
    // abgerundetes Rechteck 120 × 60 mit Umschalt → 120 × 120, Eckenradius min/4, Größengriff +10
    let e = insert(
        &mut s,
        json!({"op": "insert", "shape": "Abgerundetes Rechteck", "rect": [100, 100, 220, 160], "from": [100, 100], "shift": true}),
    );
    assert_eq!((num(&e, "width"), num(&e, "height")), (120.0, 120.0));
    assert_eq!(num(&e, "corner0"), 30.0);
    let c = cps(&e);
    assert!(
        near(c[4].0, 230.0, 1e-3) && near(c[4].1, 230.0, 1e-3),
        "{c:?}"
    );
    // ohne Umschalt bleibt es 120 × 60
    let e = insert(
        &mut s,
        json!({"op": "insert", "shape": "Abgerundetes Rechteck", "rect": [100, 100, 220, 160], "from": [100, 100]}),
    );
    assert_eq!((num(&e, "width"), num(&e, "height")), (120.0, 60.0));
    // Pfeil mit Umschalt: Rahmen 120 × 120 (Pfeilbreite = Höhe des Rahmens)
    let e = insert(
        &mut s,
        json!({"op": "insert", "shape": "Pfeil", "rect": [100, 100, 220, 160], "from": [100, 100], "shift": true}),
    );
    assert_eq!(
        (
            num(&e, "x"),
            num(&e, "y"),
            num(&e, "width") + num(&e, "arrowTip"),
            num(&e, "arrowWidth")
        ),
        (100.0, 100.0, 120.0, 120.0)
    );
    let e = insert(
        &mut s,
        json!({"op": "insert", "shape": "Pfeil", "rect": [100, 100, 220, 160], "from": [100, 100]}),
    );
    assert_eq!(num(&e, "arrowWidth"), 60.0);
    // Stern mit und ohne Umschalt gleich: Quadrat 120 → R 60, Innenradius 30, 5 Spitzen
    let a = insert(
        &mut s,
        json!({"op": "insert", "shape": "Stern", "rect": DRAG, "from": [100, 100], "shift": true}),
    );
    let b = insert(
        &mut s,
        json!({"op": "insert", "shape": "Stern", "rect": DRAG, "from": [100, 100]}),
    );
    assert_eq!(anchors(&a), anchors(&b));
    assert_eq!((num(&b, "radius0"), num(&b, "radius1")), (60.0, 30.0));
    assert_eq!(anchors(&b).len(), 10);
    // L-Form ist nicht gesperrt: ohne Umschalt folgt sie dem Zug, mit Umschalt wird sie quadratisch
    let e = insert(
        &mut s,
        json!({"op": "insert", "shape": "L-Form", "rect": [100, 100, 220, 160], "from": [100, 100]}),
    );
    assert_eq!((num(&e, "width"), num(&e, "height")), (120.0, 60.0));
    let e = insert(
        &mut s,
        json!({"op": "insert", "shape": "L-Form", "rect": [100, 100, 220, 160], "from": [100, 100], "shift": true}),
    );
    assert_eq!((num(&e, "width"), num(&e, "height")), (120.0, 120.0));
    // gesperrt, Zug nach links oben: Quadrat mit der längeren Seite in Zugrichtung
    let e = insert(
        &mut s,
        json!({"op": "insert", "shape": "Smart Polygon", "rect": DRAG, "from": [220, 200]}),
    );
    assert_eq!(
        (num(&e, "cx"), num(&e, "cy"), num(&e, "radius")),
        (160.0, 140.0, 60.0)
    );
}

/// M10 (re 43 §2): Die Pfeillinie quadriert mit Umschalt nicht, sie rastet die Richtung in
/// 45°-Schritten ein (waagerecht/senkrecht mit der Hauptachse, 45° mit max(|dx|, |dy|)).
#[test]
fn t11_arrow_line_shift_snaps_45() {
    let mut s = session();
    let ends = |e: &fp_model::Element| {
        anchors(e)
            .first()
            .copied()
            .zip(anchors(e).last().copied())
            .unwrap()
    };
    for (rect, end) in [
        ([300, 300, 500, 360], (500.0, 300.0)),
        ([400, 50, 440, 190], (400.0, 190.0)),
        ([50, 50, 150, 130], (150.0, 150.0)),
        ([50, 200, 150, 230], (150.0, 200.0)),
    ] {
        let e = insert(
            &mut s,
            json!({"op": "insert", "shape": "Pfeillinie", "rect": rect, "from": [rect[0], rect[1]], "shift": true}),
        );
        let (a, b) = ends(&e);
        assert_eq!((a, b), ((rect[0] as f32, rect[1] as f32), end), "{rect:?}");
    }
    // ohne Umschalt folgt sie dem Zug
    let e = insert(
        &mut s,
        json!({"op": "insert", "shape": "Pfeillinie", "rect": [300, 200, 500, 260], "from": [300, 200]}),
    );
    assert_eq!(ends(&e), ((300.0, 200.0), (500.0, 260.0)));
    // L-Form quadriert mit Umschalt auf max(|dx|, |dy|), auch bei höherem als breitem Zug
    let e = insert(
        &mut s,
        json!({"op": "insert", "shape": "L-Form", "rect": [300, 50, 400, 250], "from": [300, 50], "shift": true}),
    );
    assert_eq!((num(&e, "width"), num(&e, "height")), (200.0, 200.0));
}
