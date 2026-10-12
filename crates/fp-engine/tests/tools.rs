//! Werkzeug-Zustandsautomaten mit synthetischen Ereignisfolgen.

use crate::common;
use common::*;
use fp_engine::tools::{info, ToolId, TOOLS};
use fp_engine::{Shortcut, ToolEvent, ToolKey};
use fp_geom::Rect;
use fp_model::ElementKind;
use serde_json::json;

fn tool(s: &mut fp_engine::Session, t: ToolId) {
    s.set_tool(t).unwrap_or_else(|e| panic!("{e}"));
}

fn drag(s: &mut fp_engine::Session, a: (f32, f32), b: (f32, f32)) {
    events(s, &[ToolEvent::down(a.0, a.1), ToolEvent::drag((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0), ToolEvent::drag(b.0, b.1), ToolEvent::up(b.0, b.1)]);
}

fn last(s: &fp_engine::Session) -> fp_model::ElementId {
    top_elements(s)[0].id
}

#[test]
fn rectangle_tool_modifiers() {
    let mut s = doc_session(300.0, 200.0);
    tool(&mut s, ToolId::Rectangle);
    drag(&mut s, (10.0, 10.0), (60.0, 40.0));
    assert!(rect_approx(bounds(&s, last(&s)), Rect::new(10.0, 10.0, 60.0, 40.0)));
    // Klick ohne Ziehen (unter 3 px) erzeugt nichts
    events(&mut s, &[ToolEvent::down(100.0, 100.0), ToolEvent::drag(101.0, 101.0), ToolEvent::up(101.0, 101.0)]);
    assert_eq!(top_elements(&s).len(), 1);
    // Umschalt = Quadrat
    events(&mut s, &[ToolEvent::down(0.0, 100.0), ToolEvent::drag(30.0, 110.0).shift(), ToolEvent::up(30.0, 110.0).shift()]);
    assert!(rect_approx(bounds(&s, last(&s)), Rect::new(0.0, 100.0, 30.0, 130.0)));
    // Alt = von der Mitte (Ende y 122: 7 px von der Quadratmitte 115, rastet nicht ein – Fang ≤ 5)
    events(&mut s, &[ToolEvent::down(200.0, 100.0), ToolEvent::drag(210.0, 122.0).alt(), ToolEvent::up(210.0, 122.0).alt()]);
    assert!(rect_approx(bounds(&s, last(&s)), Rect::new(190.0, 78.0, 210.0, 122.0)));
    // Leertaste verschiebt die Form beim Aufziehen
    events(&mut s, &[
        ToolEvent::down(100.0, 10.0),
        ToolEvent::drag(120.0, 30.0),
        ToolEvent::drag(130.0, 40.0).space(),
        ToolEvent::up(130.0, 40.0),
    ]);
    assert!(rect_approx(bounds(&s, last(&s)), Rect::new(110.0, 20.0, 130.0, 40.0)));
    // Pfeil auf beim Aufziehen: im Original ohne Wirkung (PARITAET_A L17)
    events(&mut s, &[ToolEvent::down(0.0, 150.0), ToolEvent::drag(40.0, 190.0), ToolEvent::key(ToolKey::Up), ToolEvent::key(ToolKey::Up), ToolEvent::up(40.0, 190.0)]);
    assert!(matches!(&el(&s, last(&s)).kind, ElementKind::Path(p) if p.primitive.as_ref().is_some_and(|r| r.roundness.abs() < 1e-4)));
    assert_eq!(top_elements(&s).len(), 5);
    // Esc beim Aufziehen wird ignoriert, das Rechteck entsteht beim Loslassen (L19)
    events(&mut s, &[ToolEvent::down(200.0, 150.0), ToolEvent::drag(240.0, 180.0), ToolEvent::key(ToolKey::Escape), ToolEvent::up(240.0, 180.0)]);
    assert_eq!(top_elements(&s).len(), 6);
    assert!(rect_approx(bounds(&s, last(&s)), Rect::new(200.0, 150.0, 240.0, 180.0)));
}

#[test]
fn ellipse_polygon_line() {
    let mut s = doc_session(300.0, 200.0);
    tool(&mut s, ToolId::Ellipse);
    drag(&mut s, (10.0, 10.0), (50.0, 30.0));
    assert!(rect_approx(bounds(&s, last(&s)), Rect::new(10.0, 10.0, 50.0, 30.0)));
    tool(&mut s, ToolId::Polygon);
    drag(&mut s, (100.0, 100.0), (120.0, 100.0));
    assert!(matches!(&el(&s, last(&s)).kind, ElementKind::Path(p) if p.contours[0].nodes.len() == 5));
    tool(&mut s, ToolId::Line);
    events(&mut s, &[ToolEvent::down(0.0, 0.0), ToolEvent::drag(50.0, 3.0).shift(), ToolEvent::up(50.0, 3.0).shift()]);
    let b = bounds(&s, last(&s));
    assert!(approx(b.height(), 0.0) && approx(b.width(), b.width()));
}

#[test]
fn pen_tool() {
    let mut s = doc_session(300.0, 200.0);
    tool(&mut s, ToolId::Pen);
    events(&mut s, &[
        ToolEvent::down(0.0, 0.0),
        ToolEvent::up(0.0, 0.0),
        ToolEvent::down(50.0, 0.0),
        ToolEvent::up(50.0, 0.0),
        ToolEvent::down(50.0, 50.0),
        ToolEvent::drag(70.0, 50.0),
        ToolEvent::up(70.0, 50.0),
        ToolEvent::drag(80.0, 80.0),
    ]);
    // Vorschau des nächsten Segments (`ShowPenPreview`): gerade Linie bzw. Kurve aus dem Ausgangshenkel bis zum Zeiger
    assert!(
        s.tool_overlay().iter().any(|o| match o {
            fp_engine::Overlay::Line { to, .. } => (to.x, to.y) == (80.0, 80.0),
            fp_engine::Overlay::Path { contours } => contours.first().and_then(|c| c.nodes.last()).is_some_and(|n| (n.anchor.x, n.anchor.y) == (80.0, 80.0)),
            _ => false,
        }),
        "Vorschau des nächsten Segments"
    );
    events(&mut s, &[ToolEvent::down(80.0, 80.0), ToolEvent::up(80.0, 80.0), ToolEvent::double(80.0, 80.0)]);
    match &el(&s, last(&s)).kind {
        ElementKind::Path(p) => {
            assert_eq!(p.contours[0].nodes.len(), 4);
            assert!(!p.contours[0].closed);
            let n = &p.contours[0].nodes[2];
            assert_eq!((n.succ.x, n.pred.x), (70.0, 30.0), "Tangenten symmetrisch");
        }
        _ => panic!("kein Pfad"),
    }
    // geschlossener Pfad: Klick auf den Startpunkt
    events(&mut s, &[
        ToolEvent::down(100.0, 100.0),
        ToolEvent::up(100.0, 100.0),
        ToolEvent::down(150.0, 100.0),
        ToolEvent::up(150.0, 100.0),
        ToolEvent::down(150.0, 150.0),
        ToolEvent::up(150.0, 150.0),
        ToolEvent::down(100.5, 100.5),
        ToolEvent::up(100.5, 100.5),
    ]);
    assert!(matches!(&el(&s, last(&s)).kind, ElementKind::Path(p) if p.contours[0].closed && p.contours[0].nodes.len() == 3));
    // Werkzeugwechsel beendet offen
    events(&mut s, &[ToolEvent::down(0.0, 150.0), ToolEvent::up(0.0, 150.0), ToolEvent::down(20.0, 150.0), ToolEvent::up(20.0, 150.0)]);
    let n = top_elements(&s).len();
    tool(&mut s, ToolId::Pointer);
    assert_eq!(top_elements(&s).len(), n + 1);
}

#[test]
fn freehand_and_text() {
    let mut s = doc_session(300.0, 200.0);
    tool(&mut s, ToolId::VectorPath);
    let mut evs = vec![ToolEvent::down(0.0, 0.0)];
    for i in 1..30 {
        let x = i as f32 * 3.0;
        evs.push(ToolEvent::drag(x, (x / 10.0).sin() * 10.0));
    }
    evs.push(ToolEvent::up(90.0, 0.0));
    events(&mut s, &evs);
    assert!(matches!(&el(&s, last(&s)).kind, ElementKind::Path(p) if p.contours[0].nodes.len() >= 3 && p.attrs.fill.is_none()));
    tool(&mut s, ToolId::Text);
    events(&mut s, &[ToolEvent::down(10.0, 100.0), ToolEvent::up(10.0, 100.0), ToolEvent::text("Hal"), ToolEvent::text("lo"), ToolEvent::key(ToolKey::Backspace), ToolEvent::key(ToolKey::Escape)]);
    assert!(matches!(&el(&s, last(&s)).kind, ElementKind::Text(t) if t.plain_text() == "Hal" .to_string() + "l"));
    // Esc: Werkzeug bleibt Text wie im laufenden Original (UX-A L21/L22)
    assert_eq!(s.tools.active, ToolId::Text);
    // Rahmen aufziehen = feste Breite; vorhandenen Text bearbeiten
    tool(&mut s, ToolId::Text);
    events(&mut s, &[ToolEvent::down(100.0, 10.0), ToolEvent::drag(200.0, 20.0), ToolEvent::up(200.0, 20.0), ToolEvent::text("Rahmen")]);
    tool(&mut s, ToolId::Pointer);
    assert!(matches!(&el(&s, last(&s)).kind, ElementKind::Text(t) if t.frame_type == Some(1)));
    let t = last(&s);
    tool(&mut s, ToolId::Text);
    let b = bounds(&s, t);
    events(&mut s, &[ToolEvent::down(b.left + 1.0, b.top + 1.0), ToolEvent::up(b.left + 1.0, b.top + 1.0), ToolEvent::text("!"), ToolEvent::key(ToolKey::Escape)]);
    assert!(matches!(&el(&s, t).kind, ElementKind::Text(x) if x.plain_text() == "Rahmen!"));
}

#[test]
fn pointer_tool() {
    let mut s = doc_session(300.0, 200.0);
    let a = rect(&mut s, 10.0, 10.0, 50.0, 50.0);
    let b = rect(&mut s, 100.0, 10.0, 140.0, 50.0);
    tool(&mut s, ToolId::Pointer);
    let sel = |s: &fp_engine::Session| s.active().map(|d| d.selection.elements.clone()).unwrap_or_default();
    events(&mut s, &[ToolEvent::down(20.0, 20.0), ToolEvent::up(20.0, 20.0)]);
    assert_eq!(sel(&s), vec![a]);
    events(&mut s, &[ToolEvent::down(120.0, 20.0).shift(), ToolEvent::up(120.0, 20.0).shift()]);
    assert_eq!(sel(&s), vec![a, b]);
    // Ziehen verschiebt beide
    drag(&mut s, (120.0, 20.0), (130.0, 30.0));
    assert!(rect_approx(bounds(&s, a), Rect::new(20.0, 20.0, 60.0, 60.0)));
    assert!(rect_approx(bounds(&s, b), Rect::new(110.0, 20.0, 150.0, 60.0)));
    // Klick ins Leere hebt auf
    events(&mut s, &[ToolEvent::down(250.0, 150.0), ToolEvent::up(250.0, 150.0)]);
    assert!(sel(&s).is_empty());
    // Gummiband
    drag(&mut s, (0.0, 0.0), (70.0, 70.0));
    assert_eq!(sel(&s), vec![a]);
    // Pfeiltasten
    events(&mut s, &[ToolEvent::key(ToolKey::Left), ToolEvent::key(ToolKey::Down).shift()]);
    assert!(rect_approx(bounds(&s, a), Rect::new(19.0, 30.0, 59.0, 70.0)));
    // Griff ziehen skaliert (rechts unten), Umschalt proportional
    drag(&mut s, (59.0, 50.0), (99.0, 50.0));
    assert!(rect_approx(bounds(&s, a), Rect::new(19.0, 30.0, 99.0, 70.0)), "{:?}", bounds(&s, a));
    // Alt-Ziehen kopiert
    events(&mut s, &[ToolEvent::down(50.0, 50.0).alt(), ToolEvent::drag(60.0, 60.0).alt(), ToolEvent::up(60.0, 60.0).alt()]);
    assert_eq!(top_elements(&s).len(), 3);
    // Entf löscht
    events(&mut s, &[ToolEvent::key(ToolKey::Delete)]);
    assert_eq!(top_elements(&s).len(), 2);
    // Doppelklick auf Gruppe wählt das Kind
    select(&mut s, &[a, b]);
    ex(&mut s, "modify.group", json!({}));
    events(&mut s, &[ToolEvent::down(120.0, 30.0), ToolEvent::up(120.0, 30.0), ToolEvent::double(120.0, 30.0)]);
    assert_eq!(sel(&s), vec![b]);
}

#[test]
fn select_behind_and_subselection() {
    let mut s = doc_session(300.0, 200.0);
    let a = rect(&mut s, 0.0, 0.0, 50.0, 50.0);
    let b = rect(&mut s, 10.0, 10.0, 40.0, 40.0);
    let sel = |s: &fp_engine::Session| s.active().map(|d| d.selection.elements.clone()).unwrap_or_default();
    tool(&mut s, ToolId::SelectBehind);
    events(&mut s, &[ToolEvent::down(20.0, 20.0), ToolEvent::up(20.0, 20.0)]);
    assert_eq!(sel(&s), vec![a], "b ist schon ausgewählt → das darunter");
    events(&mut s, &[ToolEvent::down(20.0, 20.0), ToolEvent::up(20.0, 20.0)]);
    assert_eq!(sel(&s), vec![b]);
    select(&mut s, &[a, b]);
    ex(&mut s, "modify.group", json!({}));
    tool(&mut s, ToolId::Subselection);
    events(&mut s, &[ToolEvent::down(5.0, 5.0), ToolEvent::up(5.0, 5.0)]);
    assert_eq!(sel(&s), vec![a]);
    // Punkt ziehen (−7: außerhalb des Fangabstands 5 zur Seitenkante 0)
    drag(&mut s, (0.0, 0.0), (-10.0, -7.0));
    assert!(rect_approx(bounds(&s, a), Rect::new(-10.0, -7.0, 50.0, 50.0)), "{:?}", bounds(&s, a));
}

#[test]
fn transform_and_crop_tools() {
    let mut s = doc_session(300.0, 200.0);
    let a = rect(&mut s, 10.0, 10.0, 50.0, 50.0);
    tool(&mut s, ToolId::Scale);
    drag(&mut s, (50.0, 50.0), (90.0, 90.0));
    s.finish_tool().expect("übernehmen");
    assert!(rect_approx(bounds(&s, a), Rect::new(10.0, 10.0, 90.0, 90.0)));
    // außerhalb ziehen dreht (Umschalt: 15°-Raster) – 90°
    events(&mut s, &[ToolEvent::down(150.0, 50.0), ToolEvent::drag(100.0, 100.0).shift(), ToolEvent::drag(50.0, 150.0).shift(), ToolEvent::up(50.0, 150.0).shift()]);
    s.finish_tool().expect("übernehmen");
    let b = bounds(&s, a);
    assert!(approx(b.width(), 80.0) && approx(b.center().x, 50.0));
    tool(&mut s, ToolId::Skew);
    drag(&mut s, (50.0, 10.0), (70.0, 10.0));
    s.finish_tool().expect("übernehmen");
    assert!(bounds(&s, a).width() > 80.0);
    tool(&mut s, ToolId::Distort);
    let b = bounds(&s, a);
    drag(&mut s, (b.right, b.bottom), (b.right + 20.0, b.bottom + 10.0));
    s.finish_tool().expect("übernehmen");
    assert!(bounds(&s, a).right > b.right);
    events(&mut s, &[ToolEvent::key(ToolKey::Enter)]);
    assert_eq!(s.tools.active, ToolId::Pointer);
    tool(&mut s, ToolId::Crop);
    drag(&mut s, (0.0, 0.0), (100.0, 80.0));
    events(&mut s, &[ToolEvent::key(ToolKey::Enter)]);
    assert_eq!(s.active().map(|d| (d.doc.pages[0].canvas.width, d.doc.pages[0].canvas.height)), Some((100.0, 80.0)));
}

#[test]
fn marquee_lasso_wand() {
    let mut s = doc_session(300.0, 200.0);
    let pb = |s: &fp_engine::Session| s.active().and_then(|d| d.selection.pixels.as_ref().map(|m| m.tight_bounds())).unwrap_or_default();
    tool(&mut s, ToolId::Marquee);
    drag(&mut s, (10.0, 10.0), (30.0, 20.0));
    assert_eq!(pb(&s), fp_geom::IRect::new(10, 10, 30, 20));
    events(&mut s, &[ToolEvent::down(40.0, 40.0).shift(), ToolEvent::drag(50.0, 50.0).shift(), ToolEvent::up(50.0, 50.0).shift()]);
    assert_eq!(pb(&s), fp_geom::IRect::new(10, 10, 50, 50));
    events(&mut s, &[ToolEvent::down(200.0, 150.0), ToolEvent::up(200.0, 150.0)]);
    assert!(!s.active().is_some_and(|d| d.selection.has_pixels()), "Klick hebt auf");
    tool(&mut s, ToolId::OvalMarquee);
    drag(&mut s, (0.0, 0.0), (20.0, 20.0));
    assert_eq!(pb(&s), fp_geom::IRect::new(0, 0, 20, 20));
    tool(&mut s, ToolId::Lasso);
    events(&mut s, &[ToolEvent::down(0.0, 0.0), ToolEvent::drag(40.0, 0.0), ToolEvent::drag(40.0, 40.0), ToolEvent::up(0.0, 40.0)]);
    assert_eq!(pb(&s), fp_geom::IRect::new(0, 0, 40, 40));
    tool(&mut s, ToolId::PolygonLasso);
    events(&mut s, &[ToolEvent::down(0.0, 0.0), ToolEvent::up(0.0, 0.0), ToolEvent::down(60.0, 0.0), ToolEvent::up(60.0, 0.0), ToolEvent::down(60.0, 30.0), ToolEvent::double(60.0, 30.0)]);
    assert_eq!(pb(&s), fp_geom::IRect::new(1, 0, 60, 30), "Pixelmitten im Dreieck");
    let b = bitmap(&mut s, 100, 100, 20, 20, [9, 9, 9, 255]);
    let _ = b;
    tool(&mut s, ToolId::MagicWand);
    events(&mut s, &[ToolEvent::down(105.0, 105.0), ToolEvent::up(105.0, 105.0)]);
    assert_eq!(pb(&s), fp_geom::IRect::new(100, 100, 120, 120));
}

#[test]
fn bitmap_tools() {
    let mut s = doc_session(300.0, 200.0);
    // Bleistift ohne Bitmap erzeugt eine neue Bitmap
    tool(&mut s, ToolId::Pencil);
    events(&mut s, &[ToolEvent::down(10.0, 10.0), ToolEvent::drag(20.0, 10.0), ToolEvent::up(20.0, 10.0)]);
    let p = last(&s);
    assert_eq!(el(&s, p).js_type(), "Image");
    let b = bitmap(&mut s, 100, 100, 20, 20, [255, 255, 255, 255]);
    tool(&mut s, ToolId::Brush);
    events(&mut s, &[ToolEvent::down(105.0, 105.0), ToolEvent::drag(115.0, 105.0), ToolEvent::up(115.0, 105.0)]);
    assert_eq!(top_elements(&s).len(), 2, "Pinsel malt in die ausgewählte Bitmap");
    tool(&mut s, ToolId::Eraser);
    events(&mut s, &[ToolEvent::down(110.0, 115.0), ToolEvent::up(110.0, 115.0)]);
    let a = match &el(&s, b).kind {
        ElementKind::Bitmap(bm) => bm.image.pixel(10, 15).map(|c| c.a),
        _ => None,
    };
    assert_eq!(a, Some(0));
    tool(&mut s, ToolId::PaintBucket);
    s.set_colors(None, Some(fp_color::Argb(0xff00_ff00)));
    events(&mut s, &[ToolEvent::down(101.0, 119.0), ToolEvent::up(101.0, 119.0)]);
    let c = match &el(&s, b).kind {
        ElementKind::Bitmap(bm) => bm.image.pixel(1, 19).map(|c| (c.r, c.g, c.b)),
        _ => None,
    };
    assert_eq!(c, Some((0, 255, 0)));
    tool(&mut s, ToolId::Eyedropper);
    events(&mut s, &[ToolEvent::down(101.0, 119.0), ToolEvent::up(101.0, 119.0).alt()]);
    assert_eq!(s.tools.stroke_color.0, 0xff00_ff00);
    // Füllwerkzeug auf Vektor setzt die Füllung, Verlauf setzt einen Verlauf
    ex(&mut s, "select.none", json!({}));
    let r = rect(&mut s, 200.0, 10.0, 250.0, 60.0);
    ex(&mut s, "select.none", json!({}));
    s.set_colors(None, Some(fp_color::Argb(0xff12_3456)));
    tool(&mut s, ToolId::PaintBucket);
    events(&mut s, &[ToolEvent::down(220.0, 30.0), ToolEvent::up(220.0, 30.0)]);
    assert_eq!(fp_engine::docops::path_attrs_of(&el(&s, r)).map(|a| a.fill_color.0), Some(0xff12_3456));
    select(&mut s, &[r]);
    tool(&mut s, ToolId::Gradient);
    drag(&mut s, (200.0, 30.0), (250.0, 30.0));
    assert_eq!(fp_engine::docops::path_attrs_of(&el(&s, r)).and_then(|a| a.fill.as_ref().map(|f| f.shape)), Some(2));
    // Pinsel ohne Bitmap: wie im Original eine neue Bitmap (PARITAET_A L36, FV-11), kein Pfad
    ex(&mut s, "select.none", json!({}));
    tool(&mut s, ToolId::Brush);
    events(&mut s, &[ToolEvent::down(10.0, 150.0), ToolEvent::drag(30.0, 160.0), ToolEvent::drag(60.0, 150.0), ToolEvent::up(80.0, 170.0)]);
    assert_eq!(el(&s, last(&s)).js_type(), "Image");
}

#[test]
fn web_and_view_tools() {
    let mut s = doc_session(300.0, 200.0);
    tool(&mut s, ToolId::HotspotRect);
    drag(&mut s, (0.0, 0.0), (20.0, 10.0));
    assert_eq!(el(&s, last(&s)).js_type(), "Hotspot");
    tool(&mut s, ToolId::HotspotCircle);
    drag(&mut s, (50.0, 50.0), (70.0, 70.0));
    assert!(matches!(&el(&s, last(&s)).kind, ElementKind::WebObject(w) if w.shape == fp_model::WebShape::Circle));
    tool(&mut s, ToolId::Slice);
    drag(&mut s, (100.0, 0.0), (150.0, 40.0));
    assert_eq!(el(&s, last(&s)).js_type(), "SliceHotspot");
    tool(&mut s, ToolId::PolygonSlice);
    events(&mut s, &[ToolEvent::down(0.0, 100.0), ToolEvent::down(40.0, 100.0), ToolEvent::down(20.0, 140.0), ToolEvent::double(20.0, 140.0)]);
    assert!(matches!(&el(&s, last(&s)).kind, ElementKind::WebObject(w) if w.polygon.is_some()));
    tool(&mut s, ToolId::HotspotPolygon);
    events(&mut s, &[ToolEvent::down(200.0, 100.0), ToolEvent::down(240.0, 100.0), ToolEvent::down(220.0, 140.0), ToolEvent::key(ToolKey::Enter)]);
    assert_eq!(top_elements(&s).len(), 5);
    tool(&mut s, ToolId::Zoom);
    events(&mut s, &[ToolEvent::down(100.0, 100.0), ToolEvent::up(100.0, 100.0)]);
    assert_eq!(s.active().map(|d| d.view.zoom), Some(1.5));
    events(&mut s, &[ToolEvent::down(100.0, 100.0).alt(), ToolEvent::up(100.0, 100.0).alt()]);
    assert_eq!(s.active().map(|d| d.view.zoom), Some(1.0));
    ex(&mut s, "view.setViewport", json!({"width": 400, "height": 400}));
    drag(&mut s, (0.0, 0.0), (100.0, 100.0));
    // stufenlos eingepasst in den Sichtbereich minus 17 px wie im Original (PARITAET_A L25): (400 − 17) / 100
    assert_eq!(s.active().map(|d| d.view.zoom), Some(3.83));
    tool(&mut s, ToolId::Hand);
    let o = s.active().map(|d| d.view.origin).unwrap_or_default();
    // nach dem Verschieben liegt der Zeiger (Dokumentkoordinaten) wieder auf dem Griffpunkt
    events(&mut s, &[ToolEvent::down(10.0, 10.0), ToolEvent::drag(15.0, 10.0), ToolEvent::up(10.0, 10.0)]);
    let o2 = s.active().map(|d| d.view.origin).unwrap_or_default();
    assert!((o.0 - o2.0 - 5.0).abs() < 1e-6, "{o:?} {o2:?}");
}

#[test]
fn tool_table_and_shortcuts() {
    assert_eq!(TOOLS.len(), 52);
    let implemented = TOOLS.iter().filter(|t| t.implemented).count();
    println!("Werkzeuge mit Zustandsautomat: {implemented}/49");
    assert!(implemented >= 34);
    let mut s = doc_session(100.0, 100.0);
    ex(&mut s, "edit.keyboardShortcuts", json!({"set": "Firepixel"}));
    tool(&mut s, ToolId::Rectangle);
    let v = Shortcut::key(b'V' as u16);
    assert_eq!(s.tool_shortcut(&v).ok().flatten(), Some(ToolId::Pointer));
    assert_eq!(s.tool_shortcut(&v).ok().flatten(), Some(ToolId::SelectBehind), "gleiches Kürzel schaltet durch");
    let p = Shortcut::key(b'P' as u16);
    assert_eq!(s.tool_shortcut(&p).ok().flatten(), Some(ToolId::Pen));
    assert_eq!(info(ToolId::Pen).map(|i| i.flyout), Some("Stift"));
    // Menükürzel im aktiven Set
    assert_eq!(s.command_for_shortcut(&Shortcut::parse("Strg+G").unwrap_or(v)).as_deref(), Some("modify.group"));
    assert_eq!(s.command_shortcuts("edit.undo").first().map(|x| x.display_de()), Some("Strg+Z".to_string()));
}
