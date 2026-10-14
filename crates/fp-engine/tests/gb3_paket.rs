//! G-B3: Unterschiede aus der Mess-Sitzung B-2b (`docs\berichte\b2b.md`), Paket B3:
//! - BA-003 Alt-Ziehen mit dem Stift an einem Punkt des gewählten Pfads zieht einen glatten Punkt auf
//! - BA-007 Treffertest auf Bitmaps nur an Pixeln mit Alpha > 0
//! - BA-020 Grenzen der Animationsinstanz: Weg waagerecht um „Verschieben“, unabhängig von „Richtung“
//! - BA-039 Pinsel auf gesperrter Ebene malt auf die nächste freie Ebene darunter (wird aktiv)
//! - BA-119 In Bitmap konvertieren backt Deckkraft und Mischmodus ein

use crate::common;
use common::*;
use fp_engine::tools::ToolId;
use fp_engine::{Session, ToolEvent};
use fp_geom::Point;
use fp_model::{ElementId, ElementKind};
use serde_json::json;

fn tool(s: &mut Session, t: ToolId) {
    s.set_tool(t).unwrap_or_else(|e| panic!("{e}"));
}

fn drag_m(s: &mut Session, a: (f32, f32), b: (f32, f32), m: impl Fn(ToolEvent) -> ToolEvent) {
    events(s, &[m(ToolEvent::down(a.0, a.1))]);
    for i in 1..=10 {
        let t = i as f32 / 10.0;
        events(s, &[m(ToolEvent::drag(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t))]);
    }
    events(s, &[m(ToolEvent::up(b.0, b.1))]);
}

fn click(s: &mut Session, x: f32, y: f32) {
    events(s, &[ToolEvent::down(x, y), ToolEvent::up(x, y)]);
}

fn sel(s: &Session) -> Vec<ElementId> {
    s.active().map(|d| d.selection.elements.clone()).unwrap_or_default()
}

fn id_of(v: &serde_json::Value) -> ElementId {
    ElementId(v["id"].as_u64().or_else(|| v["ids"][0].as_u64()).expect("id"))
}

fn contour(s: &Session, id: ElementId) -> fp_model::Contour {
    match &el(s, id).kind {
        ElementKind::Path(p) => p.contours[0].clone(),
        _ => panic!("kein Pfad"),
    }
}

fn pt(p: Point) -> (f32, f32) {
    (p.x.round(), p.y.round())
}

// ------------------------------------------------------------------ BA-003

#[test]
fn ba003_alt_drag_on_points_makes_smooth_points() {
    let mut s = doc_session(600.0, 400.0);
    let v = ex(&mut s, "draw.path", json!({"contours": [{"closed": false, "nodes": [[200, 200], [300, 160], [400, 200]]}]}));
    let id = id_of(&v);
    select(&mut s, &[id]);
    tool(&mut s, ToolId::Pen);
    // mittlerer Eckpunkt: nicht löschen, beide Henkel symmetrisch
    drag_m(&mut s, (300.0, 160.0), (330.0, 160.0), |e| e.alt());
    let c = contour(&s, id);
    assert_eq!(c.nodes.len(), 3, "Punkt bleibt");
    assert_eq!((pt(c.nodes[1].pred), pt(c.nodes[1].succ)), ((270.0, 160.0), (330.0, 160.0)), "glatter Punkt");
    assert_eq!(s.active().and_then(|d| d.history.undo_label().map(|l| l.to_string())).as_deref(), Some("Stift"));
    // Endpunkt: beide Henkel (370 · 430), der Pfad wird fortgesetzt
    drag_m(&mut s, (400.0, 200.0), (430.0, 200.0), |e| e.alt());
    // Alt-Ziehen am Anfangspunkt schließt, nur der Eingangshenkel (230/200)
    drag_m(&mut s, (200.0, 200.0), (170.0, 200.0), |e| e.alt());
    let c = contour(&s, id);
    assert!(c.closed, "geschlossen");
    assert_eq!(c.nodes.len(), 3);
    assert_eq!((pt(c.nodes[2].pred), pt(c.nodes[2].succ)), ((370.0, 200.0), (430.0, 200.0)), "Endpunkt glatt");
    assert_eq!((pt(c.nodes[0].pred), pt(c.nodes[0].succ)), ((230.0, 200.0), (200.0, 200.0)), "Anfangspunkt nur eingehend");
    assert_eq!(top_elements(&s).len(), 1, "kein neuer Pfad");
}

#[test]
fn ba003_plain_click_still_removes_and_new_path_start_keeps_out_handle() {
    let mut s = doc_session(600.0, 400.0);
    let v = ex(&mut s, "draw.path", json!({"contours": [{"closed": false, "nodes": [[200, 200], [300, 160], [400, 200]]}]}));
    let id = id_of(&v);
    select(&mut s, &[id]);
    tool(&mut s, ToolId::Pen);
    click(&mut s, 300.0, 160.0);
    assert_eq!(contour(&s, id).nodes.len(), 2, "Klick ohne Alt entfernt den Eckpunkt");
    // Alt-Klick ohne Bewegung wie ein Klick
    ex(&mut s, "edit.undo", json!({}));
    events(&mut s, &[ToolEvent::down(300.0, 160.0).alt(), ToolEvent::up(300.0, 160.0).alt()]);
    assert_eq!(contour(&s, id).nodes.len(), 2, "Alt-Klick ohne Ziehen wie Klick");
    // neuer Pfad: Alt-Ziehen am soeben gesetzten Startpunkt nur Ausgangshenkel (PAR 09/09)
    ex(&mut s, "select.none", json!({}));
    click(&mut s, 100.0, 300.0);
    drag_m(&mut s, (100.0, 300.0), (130.0, 300.0), |e| e.alt());
    click(&mut s, 200.0, 300.0);
    events(&mut s, &[ToolEvent::key(fp_engine::ToolKey::Enter)]);
    let nid = sel(&s)[0];
    let c = contour(&s, nid);
    assert_eq!((pt(c.nodes[0].pred), pt(c.nodes[0].succ)), ((100.0, 300.0), (130.0, 300.0)));
}

// ------------------------------------------------------------------ BA-007

#[test]
fn ba007_bitmap_hits_only_opaque_pixels() {
    let mut s = doc_session(800.0, 600.0);
    let r1 = rect(&mut s, 50.0, 50.0, 150.0, 120.0);
    // „gerasterte Ellipse“: Bitmap mit durchsichtiger Ecke
    let mut img = fp_raster::RgbaImage::filled(fp_geom::IRect::new(500, 300, 600, 400), fp_color::Rgba8::new(0, 0, 255, 255)).expect("Bild");
    img.fill_rect(fp_geom::IRect::new(500, 300, 520, 320), fp_color::Rgba8::new(0, 0, 0, 0));
    s.clipboard.clear();
    s.clipboard.pixels = Some(img);
    let eb = id_of(&ex(&mut s, "edit.paste", json!({})));
    ex(&mut s, "select.none", json!({}));
    tool(&mut s, ToolId::Pointer);
    click(&mut s, 503.0, 303.0);
    assert!(sel(&s).is_empty(), "durchsichtige Ecke: nichts gewählt");
    click(&mut s, 550.0, 350.0);
    assert_eq!(sel(&s), vec![eb], "Mitte wählt EB");
    // Umschalt-Klick auf die Ecke bei gewähltem R1 → nur R1
    click(&mut s, 100.0, 85.0);
    events(&mut s, &[ToolEvent::down(503.0, 303.0).shift(), ToolEvent::up(503.0, 303.0).shift()]);
    assert_eq!(sel(&s), vec![r1], "Umschalt-Klick auf die Ecke fügt nichts hinzu");
    // Unterauswahl: nichts
    ex(&mut s, "select.none", json!({}));
    tool(&mut s, ToolId::Subselection);
    click(&mut s, 503.0, 303.0);
    assert!(sel(&s).is_empty(), "Unterauswahl: nichts");
    // Ziehen aus der Ecke verschiebt EB nicht (Auswahlrahmen)
    tool(&mut s, ToolId::Pointer);
    let before = bounds(&s, eb);
    drag_m(&mut s, (503.0, 303.0), (507.0, 330.0), |e| e);
    assert_eq!(bounds(&s, eb), before, "nicht verschoben");
}

// ------------------------------------------------------------------ BA-020

#[test]
fn ba020_animation_bounds_ignore_direction() {
    let mut s = doc_session(600.0, 400.0);
    let r1 = rect(&mut s, 50.0, 50.0, 150.0, 120.0);
    select(&mut s, &[r1]);
    // Dialog: Verschieben 72, Richtung 30 → Versatz (62, −36)
    let dir = 30f64.to_radians();
    let v = ex(&mut s, "modify.animation.animateSelection", json!({"frames": 3, "offsetX": (72.0 * dir.cos()).round(), "offsetY": (-72.0 * dir.sin()).round(), "addStates": false}));
    let b = bounds(&s, id_of(&v));
    assert_eq!((b.left.round(), b.top.round(), b.right.round(), b.bottom.round()), (50.0, 50.0, 222.0, 120.0), "Original 50,50–222,120: {b:?}");
}

// ------------------------------------------------------------------ BA-039

#[test]
fn ba039_brush_on_locked_layer_paints_on_free_layer_below() {
    let mut s = doc_session(800.0, 600.0);
    let d = s.active().expect("Dokument");
    let lower = d.layer.expect("Ebene 1");
    ex(&mut s, "layer.add", json!({}));
    let upper = s.active().and_then(|d| d.layer).expect("neue Ebene");
    assert_ne!(lower, upper);
    ex(&mut s, "layer.setLocked", json!({"locked": true}));
    tool(&mut s, ToolId::Brush);
    events(&mut s, &[ToolEvent::down(60.0, 450.0), ToolEvent::drag(130.0, 450.0)]);
    s.tool_event(&ToolEvent::up(200.0, 450.0)).expect("keine Meldung");
    assert_eq!(s.active().and_then(|d| d.layer), Some(lower), "Ebene darunter ist danach aktiv");
    let d = s.active().expect("Dokument");
    let (_, l) = fp_engine::docops::own_layers(&d.doc, d.page).into_iter().find(|(_, l)| l.id == lower).expect("Ebene 1");
    let els = &l.cels[0].elements;
    assert_eq!(els.len(), 1, "neue Bitmap auf Ebene 1");
    assert!(matches!(els[0].kind, ElementKind::Bitmap(_)));
    let (_, u) = fp_engine::docops::own_layers(&d.doc, d.page).into_iter().find(|(_, l)| l.id == upper).expect("gesperrt");
    assert!(u.cels[0].elements.is_empty(), "gesperrte Ebene bleibt leer");
}

// ------------------------------------------------------------------ BA-119

fn flatten(s: &mut Session, ids: &[ElementId]) -> ElementId {
    select(s, ids);
    id_of(&ex(s, "modify.flattenSelection", json!({})))
}

fn pixel(s: &Session, id: ElementId, x: i32, y: i32) -> [u8; 4] {
    let ElementKind::Bitmap(b) = &el(s, id).kind else { panic!("keine Bitmap") };
    fp_engine::docops::bitmap_image(b).and_then(|i| i.get(x, y)).expect("Pixel")
}

fn solid_rect(s: &mut Session, r: [f32; 4], color: &str, opacity: f64) -> ElementId {
    let id = rect(s, r[0], r[1], r[2], r[3]);
    ex(s, "element.setFill", json!({"ids": [id.0], "color": color}));
    ex(s, "element.setStroke", json!({"ids": [id.0], "none": true}));
    if opacity < 100.0 {
        ex(s, "element.setOpacity", json!({"ids": [id.0], "opacity": opacity}));
    }
    id
}

#[test]
fn ba119_flatten_bakes_opacity() {
    // grünes Rechteck + rotes mit Deckkraft 0 → nur das grüne (20,20–100,100)
    let mut s = doc_session(400.0, 300.0);
    s.execute("modify.canvas.canvasColor", json!({"color": "#ffffff00"})).expect("transparent");
    let g = solid_rect(&mut s, [20.0, 20.0, 100.0, 100.0], "#00ff00", 100.0);
    let r = solid_rect(&mut s, [60.0, 60.0, 140.0, 140.0], "#ff0000", 0.0);
    let id = flatten(&mut s, &[r, g]);
    let b = bounds(&s, id);
    assert_eq!((b.left, b.top, b.right, b.bottom), (20.0, 20.0, 100.0, 100.0), "Deckkraft 0 trägt nichts bei");
    assert_eq!(pixel(&s, id, 90, 90), [0, 255, 0, 255], "keine roten Pixel");
    // blaues Rechteck Deckkraft 50 allein → 0,0,255,128, Objekt-Deckkraft danach 100
    let blue = solid_rect(&mut s, [200.0, 20.0, 260.0, 80.0], "#0000ff", 50.0);
    let id = flatten(&mut s, &[blue]);
    assert_eq!(pixel(&s, id, 230, 50), [0, 0, 255, 128]);
    assert_eq!(el(&s, id).common.opacity.0, 255, "Deckkraft 100");
    // gelb deckend + blau 50 % → Mischpixel 127,127,128,255
    let y = solid_rect(&mut s, [20.0, 150.0, 120.0, 250.0], "#ffff00", 100.0);
    let bl = solid_rect(&mut s, [70.0, 150.0, 170.0, 250.0], "#0000ff", 50.0);
    let id = flatten(&mut s, &[bl, y]);
    let p = pixel(&s, id, 100, 200);
    assert!(p[3] == 255 && p[0].abs_diff(127) <= 1 && p[1].abs_diff(127) <= 1 && p[2].abs_diff(128) <= 1, "Mischpixel wie im Original 127,127,128,255: {p:?}");
}
