//! Alle 49 Werkzeuge auf der Leinwand: synthetische Zeigerfolgen wie von der Oberfläche, Prüfung des
//! Ergebnisses am Dokument, der Modifikatoren (Umschalt/Alt/Strg) und von Rückgängig/Wiederholen.

mod common;
use common::*;
use fp_engine::tools::{EditOp, ToolId, ToolInput, TOOLS};
use fp_engine::{Overlay, Session, ToolEvent, ToolKey};
use fp_geom::{IRect, Point, Rect};
use fp_model::{Document, ElementId, ElementKind};
use serde_json::json;

// ------------------------------------------------------------------ Hilfen

fn tool(s: &mut Session, t: ToolId) {
    s.set_tool(t).unwrap_or_else(|e| panic!("{e}"));
}

fn drag(s: &mut Session, a: (f32, f32), b: (f32, f32)) {
    events(s, &[ToolEvent::down(a.0, a.1), ToolEvent::drag((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0), ToolEvent::drag(b.0, b.1), ToolEvent::up(b.0, b.1)]);
}

fn click(s: &mut Session, x: f32, y: f32) {
    events(s, &[ToolEvent::down(x, y), ToolEvent::up(x, y)]);
}

fn doc(s: &Session) -> Document {
    (*s.active().expect("Dokument").doc).clone()
}

fn last(s: &Session) -> ElementId {
    top_elements(s)[0].id
}

fn sel(s: &Session) -> Vec<ElementId> {
    s.active().map(|d| d.selection.elements.clone()).unwrap_or_default()
}

/// Rückgängig stellt `before` exakt wieder her, Wiederholen das Ergebnis `after`.
fn undo_redo(s: &mut Session, before: &Document) {
    let after = doc(s);
    assert_ne!(&after, before, "Werkzeug hat das Dokument nicht geändert");
    s.undo().expect("Rückgängig");
    assert_eq!(&doc(s), before, "Rückgängig stellt nicht exakt wieder her");
    s.redo().expect("Wiederholen");
    assert_eq!(doc(s), after, "Wiederholen liefert ein anderes Ergebnis");
}

fn pixel_bounds(s: &Session) -> IRect {
    s.active().and_then(|d| d.selection.pixels.as_ref().filter(|_| d.selection.has_pixels()).map(|m| m.tight_bounds())).unwrap_or_default()
}

fn image(s: &Session, id: ElementId) -> fp_raster::RgbaImage {
    match &el(s, id).kind {
        ElementKind::Bitmap(b) => fp_engine::docops::bitmap_image(b).expect("Bild"),
        _ => panic!("keine Bitmap"),
    }
}

fn px(s: &Session, id: ElementId, x: i32, y: i32) -> [u8; 4] {
    image(s, id).get(x, y).unwrap_or([0; 4])
}

/// Bitmap mit Muster: links schwarz, rechts weiß, rechts unten ein rotes Feld (für Retusche-Werkzeuge).
fn pattern_bitmap(s: &mut Session, l: i32, t: i32) -> ElementId {
    let mut img = fp_raster::RgbaImage::filled(IRect::from_xywh(l, t, 40, 40), fp_color::Rgba8::new(255, 255, 255, 255)).expect("Bild");
    for y in t..t + 40 {
        for x in l..l + 40 {
            let c = if x < l + 20 {
                fp_color::Rgba8::new(20, 20, 20, 255)
            } else if y >= t + 25 && x >= l + 25 {
                fp_color::Rgba8::new(220, 30, 30, 255)
            } else if y < t + 10 && x >= l + 30 {
                fp_color::Rgba8::new(0, 255, 0, 255)
            } else {
                fp_color::Rgba8::new(200, 200, 200, 255)
            };
            img.set_pixel(x, y, c);
        }
    }
    s.clipboard.clear();
    s.clipboard.pixels = Some(img);
    let v = ex(s, "edit.paste", json!({}));
    ElementId(v["ids"][0].as_u64().expect("id"))
}

fn path_of(s: &Session, id: ElementId) -> fp_model::element::PathElem {
    match &el(s, id).kind {
        ElementKind::Path(p) => p.clone(),
        _ => panic!("kein Pfad"),
    }
}

// ------------------------------------------------------------------ Auswählen

#[test]
fn pointer_select_move_copy_and_corner_handle_scales() {
    let mut s = doc_session(300.0, 200.0);
    let a = rect(&mut s, 10.0, 10.0, 50.0, 50.0);
    let b = ellipse(&mut s, 100.0, 10.0, 140.0, 50.0);
    tool(&mut s, ToolId::Pointer);
    click(&mut s, 30.0, 30.0);
    assert_eq!(sel(&s), vec![a]);
    // Umschalt schaltet um (hinzufügen, erneut = entfernen)
    events(&mut s, &[ToolEvent::down(120.0, 30.0).shift(), ToolEvent::up(120.0, 30.0).shift()]);
    assert_eq!(sel(&s), vec![a, b]);
    events(&mut s, &[ToolEvent::down(120.0, 30.0).shift(), ToolEvent::up(120.0, 30.0).shift()]);
    assert_eq!(sel(&s), vec![a]);
    // Verschieben (Umschalt = 45°-Raster) mit Rückgängig
    let before = doc(&s);
    events(&mut s, &[ToolEvent::down(30.0, 30.0), ToolEvent::drag(36.0, 32.0).shift(), ToolEvent::drag(40.0, 30.5).shift(), ToolEvent::up(40.0, 30.5).shift()]);
    let b0 = bounds(&s, a);
    assert!(approx(b0.top, 10.0) && (b0.left - 20.0).abs() < 0.1, "waagerecht eingerastet: {b0:?}");
    undo_redo(&mut s, &before);
    s.undo().expect("undo");
    ex(&mut s, "edit.move", json!({"dx": 10, "dy": 0}));
    // Alt-Ziehen kopiert, das Original bleibt
    let n = top_elements(&s).len();
    events(&mut s, &[ToolEvent::down(40.0, 30.0).alt(), ToolEvent::drag(40.0, 100.0).alt(), ToolEvent::up(40.0, 100.0).alt()]);
    assert_eq!(top_elements(&s).len(), n + 1);
    assert!(rect_approx(bounds(&s, a), Rect::new(20.0, 10.0, 60.0, 50.0)));
    // Vorschau beim Ziehen: Ghost-Overlay mit den IDs der Auswahl
    select(&mut s, &[a]);
    events(&mut s, &[ToolEvent::down(40.0, 30.0), ToolEvent::drag(45.0, 35.0)]);
    let fb = s.tool_feedback();
    assert!(fb.iter().any(|o| matches!(o, Overlay::Ghost { ids, copy: false, .. } if ids == &vec![a.0])), "{fb:?}");
    events(&mut s, &[ToolEvent::key(ToolKey::Escape)]);
    // Eckgriff einer Rechteck-Grundform ziehen = Skalieren wie bei allen Objekten, die Rundung
    // bleibt 0 (Original live: R1 50,50–150,120, Eckgriff +40/+30 → größer, PARITAET_A L26)
    let a = rect(&mut s, 150.0, 100.0, 250.0, 170.0);
    tool(&mut s, ToolId::Pointer);
    let fb = s.tool_feedback();
    assert!(fb.iter().any(|o| matches!(o, Overlay::PathPoints { contours, .. } if contours[0].nodes.len() == 4)), "Eckpunkte der Grundform");
    // Überfahren des Eckgriffs: Pfeil ohne Zusatz, weiß mit schwarzem Rand (K2, PM-1 im echten
    // Fenster); Kantengriff: schwarzer Pfeil (K3)
    let r = s.tool_event(&ToolEvent::drag(250.0, 170.0)).expect("hover");
    assert_eq!(r.cursor, fp_engine::tools::Cursor::Subselect, "Eckgriff");
    let r = s.tool_event(&ToolEvent::drag(250.0, 135.0)).expect("hover");
    assert_eq!(r.cursor, fp_engine::tools::Cursor::Arrow, "Kantengriff");
    let before = doc(&s);
    let mut seq = vec![ToolEvent::down(250.0, 170.0)];
    for i in 1..=20 {
        let f = i as f32 / 20.0;
        seq.push(ToolEvent::drag(250.0 + 40.0 * f, 170.0 + 30.0 * f));
    }
    seq.push(ToolEvent::up(290.0, 200.0));
    events(&mut s, &seq);
    let p = path_of(&s, a);
    let r = p.primitive.as_ref().map(|x| x.roundness).unwrap_or(0.0);
    assert_eq!(r, 0.0, "Rundung bleibt 0");
    assert!(rect_approx(bounds(&s, a), Rect::new(150.0, 100.0, 290.0, 200.0)), "größer: {:?}", bounds(&s, a));
    undo_redo(&mut s, &before);
    // jede Ecke: die gegenüberliegende bleibt stehen
    for (from, to, want) in [
        ((150.0, 100.0), (130.0, 90.0), Rect::new(130.0, 90.0, 250.0, 170.0)),
        ((250.0, 100.0), (260.0, 80.0), Rect::new(150.0, 80.0, 260.0, 170.0)),
        ((150.0, 170.0), (140.0, 190.0), Rect::new(140.0, 100.0, 250.0, 190.0)),
    ] {
        s.undo().expect("undo");
        select(&mut s, &[a]);
        tool(&mut s, ToolId::Pointer);
        drag(&mut s, from, to);
        assert!(rect_approx(bounds(&s, a), want), "{from:?}: {:?}", bounds(&s, a));
        assert_eq!(path_of(&s, a).primitive.as_ref().map(|x| x.roundness), Some(0.0));
    }
}

#[test]
fn pointer_rubber_band_keys_and_double_click() {
    let mut s = doc_session(300.0, 200.0);
    let a = rect(&mut s, 10.0, 10.0, 50.0, 50.0);
    let b = rect(&mut s, 100.0, 10.0, 140.0, 50.0);
    tool(&mut s, ToolId::Pointer);
    drag(&mut s, (0.0, 0.0), (160.0, 70.0));
    assert_eq!(sel(&s).len(), 2);
    events(&mut s, &[ToolEvent::key(ToolKey::Escape)]);
    assert!(sel(&s).is_empty(), "Esc hebt die Auswahl auf");
    select(&mut s, &[a]);
    let before = doc(&s);
    events(&mut s, &[ToolEvent::key(ToolKey::Right).shift()]);
    assert!(rect_approx(bounds(&s, a), Rect::new(20.0, 10.0, 60.0, 50.0)), "Umschalt+Pfeil = 10 px");
    undo_redo(&mut s, &before);
    // Alt+Pfeil kopiert
    let n = top_elements(&s).len();
    events(&mut s, &[ToolEvent::key(ToolKey::Down).alt()]);
    assert_eq!(top_elements(&s).len(), n + 1);
    // Doppelklick auf einen Pfad wechselt zur Teilauswahl
    click(&mut s, 120.0, 30.0);
    events(&mut s, &[ToolEvent::double(120.0, 30.0)]);
    assert_eq!(s.tools.active, ToolId::Subselection);
    assert_eq!(sel(&s), vec![b]);
}

#[test]
fn select_behind_cycles_down() {
    let mut s = doc_session(300.0, 200.0);
    let a = rect(&mut s, 0.0, 0.0, 50.0, 50.0);
    let b = rect(&mut s, 10.0, 10.0, 40.0, 40.0);
    tool(&mut s, ToolId::SelectBehind);
    click(&mut s, 20.0, 20.0);
    assert_eq!(sel(&s), vec![a]);
    click(&mut s, 20.0, 20.0);
    assert_eq!(sel(&s), vec![b]);
}

#[test]
fn subselection_points_toggle_and_handles() {
    let mut s = doc_session(300.0, 200.0);
    tool(&mut s, ToolId::Pen);
    events(&mut s, &[ToolEvent::down(10.0, 10.0), ToolEvent::up(10.0, 10.0), ToolEvent::down(60.0, 10.0), ToolEvent::up(60.0, 10.0), ToolEvent::down(60.0, 60.0), ToolEvent::up(60.0, 60.0), ToolEvent::key(ToolKey::Enter)]);
    let p = last(&s);
    tool(&mut s, ToolId::Subselection);
    // Gummiband über zwei Punkte
    drag(&mut s, (0.0, 0.0), (70.0, 20.0));
    let pts = |s: &Session| s.active().map(|d| d.selection.points.len()).unwrap_or(0);
    assert_eq!(pts(&s), 2);
    // Umschalt-Gummiband schaltet um: (60,10) ab, (60,60) dazu
    events(&mut s, &[ToolEvent::down(45.0, -10.0).shift(), ToolEvent::drag(55.0, 30.0).shift(), ToolEvent::drag(70.0, 70.0).shift(), ToolEvent::up(70.0, 70.0).shift()]);
    let chosen: Vec<usize> = s.active().map(|d| d.selection.points.iter().map(|r| r.node).collect()).unwrap_or_default();
    assert!(chosen.contains(&0) && chosen.contains(&2) && !chosen.contains(&1), "{chosen:?}");
    // Punkte gemeinsam mit Pfeiltasten verschieben
    let before = doc(&s);
    events(&mut s, &[ToolEvent::key(ToolKey::Left)]);
    let c = &path_of(&s, p).contours[0];
    assert_eq!((c.nodes[0].anchor.x, c.nodes[1].anchor.x, c.nodes[2].anchor.x), (9.0, 60.0, 59.0));
    undo_redo(&mut s, &before);
    // Alt-Ziehen am Anker zieht wie im Original nur den Ausgangshenkel heraus (PARITAET_A L33)
    events(&mut s, &[ToolEvent::down(60.0, 10.0), ToolEvent::up(60.0, 10.0)]);
    events(&mut s, &[ToolEvent::down(60.0, 10.0).alt(), ToolEvent::drag(65.0, 10.0).alt(), ToolEvent::drag(80.0, 10.0).alt(), ToolEvent::up(80.0, 10.0).alt()]);
    let n = &path_of(&s, p).contours[0].nodes[1];
    assert_eq!((n.succ.x, n.pred.x), (80.0, 60.0));
    // Entf löscht unterausgewählte Punkte
    events(&mut s, &[ToolEvent::key(ToolKey::Delete)]);
    assert_eq!(path_of(&s, p).contours[0].nodes.len(), 2);
}

// ------------------------------------------------------------------ Pixelauswahl

#[test]
fn marquee_tools_with_modifiers_and_undo() {
    let mut s = doc_session(300.0, 200.0);
    tool(&mut s, ToolId::Marquee);
    let before = doc(&s);
    drag(&mut s, (10.0, 10.0), (40.0, 30.0));
    assert_eq!(pixel_bounds(&s), IRect::new(10, 10, 40, 30));
    let _ = before;
    // Umschalt = hinzufügen, Alt = abziehen
    events(&mut s, &[ToolEvent::down(40.0, 10.0).shift(), ToolEvent::drag(50.0, 20.0).shift(), ToolEvent::drag(60.0, 30.0).shift(), ToolEvent::up(60.0, 30.0).shift()]);
    assert_eq!(pixel_bounds(&s), IRect::new(10, 10, 60, 30));
    events(&mut s, &[ToolEvent::down(10.0, 10.0).alt(), ToolEvent::drag(20.0, 20.0).alt(), ToolEvent::drag(30.0, 30.0).alt(), ToolEvent::up(30.0, 30.0).alt()]);
    assert_eq!(pixel_bounds(&s), IRect::new(30, 10, 60, 30));
    // Rückgängig nimmt das Abziehen zurück
    s.undo().expect("undo");
    assert_eq!(pixel_bounds(&s), IRect::new(10, 10, 60, 30));
    s.redo().expect("redo");
    tool(&mut s, ToolId::OvalMarquee);
    drag(&mut s, (100.0, 100.0), (140.0, 120.0));
    let m = s.active().and_then(|d| d.selection.pixels.clone()).expect("Maske");
    assert!(m.get(120, 110) > 200 && m.get(101, 101) < 50, "Ellipse");
    tool(&mut s, ToolId::Lasso);
    events(&mut s, &[ToolEvent::down(0.0, 0.0), ToolEvent::drag(40.0, 0.0), ToolEvent::drag(40.0, 40.0), ToolEvent::up(0.0, 40.0)]);
    assert_eq!(pixel_bounds(&s), IRect::new(0, 0, 40, 40));
    tool(&mut s, ToolId::PolygonLasso);
    events(&mut s, &[ToolEvent::down(0.0, 0.0), ToolEvent::up(0.0, 0.0), ToolEvent::down(60.0, 0.0), ToolEvent::up(60.0, 0.0), ToolEvent::down(60.0, 30.0), ToolEvent::double(60.0, 30.0)]);
    assert!(pixel_bounds(&s).right == 60 && pixel_bounds(&s).bottom == 30);
}

#[test]
fn magic_wand_tolerance_and_shift() {
    let mut s = doc_session(300.0, 200.0);
    let b = pattern_bitmap(&mut s, 100, 100);
    select(&mut s, &[b]);
    tool(&mut s, ToolId::MagicWand);
    click(&mut s, 105.0, 105.0);
    assert_eq!(pixel_bounds(&s), IRect::new(100, 100, 120, 140), "schwarze Hälfte");
    events(&mut s, &[ToolEvent::down(135.0, 135.0).shift(), ToolEvent::up(135.0, 135.0).shift()]);
    assert_eq!(pixel_bounds(&s), IRect::new(100, 100, 140, 140), "rotes Feld dazu");
}

// ------------------------------------------------------------------ Zuschneiden

#[test]
fn crop_and_export_area() {
    let mut s = doc_session(300.0, 200.0);
    rect(&mut s, 10.0, 10.0, 50.0, 50.0);
    tool(&mut s, ToolId::Crop);
    let before = doc(&s);
    drag(&mut s, (0.0, 0.0), (100.0, 80.0));
    assert!(s.tool_feedback().iter().any(|o| matches!(o, Overlay::Shade { .. })), "außen abgedunkelt");
    events(&mut s, &[ToolEvent::key(ToolKey::Enter)]);
    assert_eq!(s.active().map(|d| (d.doc.pages[0].canvas.width, d.doc.pages[0].canvas.height)), Some((100.0, 80.0)));
    undo_redo(&mut s, &before);
    // Exportbereich: Rahmen aufziehen, Esc verwirft ohne Änderung
    tool(&mut s, ToolId::ExportArea);
    let d0 = doc(&s);
    drag(&mut s, (10.0, 10.0), (60.0, 60.0));
    assert!(s.tool_feedback().iter().any(|o| matches!(o, Overlay::Shade { .. })));
    events(&mut s, &[ToolEvent::key(ToolKey::Escape)]);
    assert_eq!(doc(&s), d0);
    assert_eq!(s.tools.active, ToolId::ExportArea);
    // Zuschneiden: Rahmen ziehen, Esc → Leinwand unverändert, Werkzeug Zeiger (PARITAET_A L28)
    tool(&mut s, ToolId::Crop);
    let h0 = s.active().unwrap().history.len();
    drag(&mut s, (20.0, 20.0), (120.0, 90.0));
    events(&mut s, &[ToolEvent::key(ToolKey::Escape)]);
    assert_eq!(doc(&s), d0, "Leinwand unverändert");
    assert_eq!(s.active().unwrap().history.len(), h0, "kein Verlaufsschritt");
    assert_eq!(s.tools.active, ToolId::Pointer, "Werkzeug wechselt zum Zeiger");
    assert!(!s.tool_feedback().iter().any(|o| matches!(o, Overlay::Shade { .. })));
}

// ------------------------------------------------------------------ Vektor

#[test]
fn shapes_line_and_modifiers() {
    let mut s = doc_session(300.0, 200.0);
    for (t, a, b, want) in [
        (ToolId::Rectangle, (10.0, 10.0), (60.0, 40.0), Rect::new(10.0, 10.0, 60.0, 40.0)),
        (ToolId::Ellipse, (100.0, 10.0), (140.0, 32.0), Rect::new(100.0, 10.0, 140.0, 32.0)),
    ] {
        tool(&mut s, t);
        let before = doc(&s);
        drag(&mut s, a, b);
        assert!(rect_approx(bounds(&s, last(&s)), want), "{t:?} {:?}", bounds(&s, last(&s)));
        undo_redo(&mut s, &before);
        // Umschalt = Quadrat/Kreis, Alt = von der Mitte
        events(&mut s, &[ToolEvent::down(200.0, 100.0), ToolEvent::drag(210.0, 120.0).alt().shift(), ToolEvent::up(210.0, 120.0).alt().shift()]);
        assert!(rect_approx(bounds(&s, last(&s)), Rect::new(180.0, 80.0, 220.0, 120.0)), "{t:?} Umschalt+Alt {:?}", bounds(&s, last(&s)));
    }
    tool(&mut s, ToolId::Polygon);
    let before = doc(&s);
    drag(&mut s, (100.0, 100.0), (130.0, 100.0));
    assert_eq!(path_of(&s, last(&s)).contours[0].nodes.len(), 5);
    undo_redo(&mut s, &before);
    tool(&mut s, ToolId::Line);
    let before = doc(&s);
    events(&mut s, &[ToolEvent::down(0.0, 150.0), ToolEvent::drag(30.0, 152.0).shift(), ToolEvent::drag(60.0, 153.0).shift(), ToolEvent::up(60.0, 153.0).shift()]);
    let b = bounds(&s, last(&s));
    assert!(approx(b.height(), 0.0) || b.height() <= 1.0, "45°-Raster: waagerecht {b:?}");
    assert!(path_of(&s, last(&s)).attrs.fill.is_none(), "Linie ohne Füllung");
    undo_redo(&mut s, &before);
}

#[test]
fn shapes_use_inspector_tool_attributes() {
    let mut s = doc_session(300.0, 200.0);
    s.tools.options.new_fill = Some(json!({"category": "gradient", "shape": 2, "edge": 2, "feather": 4}));
    s.tools.options.new_stroke = Some(json!({"width": 5, "placement": "outside"}));
    tool(&mut s, ToolId::Rectangle);
    let before = doc(&s);
    let hist = s.active().map(|d| d.history.undo_count()).unwrap_or(0);
    drag(&mut s, (10.0, 10.0), (90.0, 60.0));
    let p = path_of(&s, last(&s));
    let f = p.attrs.fill.as_ref().expect("Füllung");
    assert!(f.gradient.is_some() && f.shape == 2, "Verlauf aus dem Inspektor");
    assert_eq!((f.edge, f.feather), (2, 4), "weiche Kante");
    assert_eq!(p.attrs.brush.as_ref().map(|b| b.diameter), Some(5));
    assert_eq!(p.stroke_placement, 2, "außen");
    assert_eq!(s.active().map(|d| d.history.undo_count()).unwrap_or(0), hist + 1, "ein Verlaufsschritt");
    undo_redo(&mut s, &before);
    // Linie: Strich ja, Füllung nein
    tool(&mut s, ToolId::Line);
    drag(&mut s, (0.0, 150.0), (80.0, 150.0));
    let p = path_of(&s, last(&s));
    assert!(p.attrs.fill.is_none());
    assert_eq!(p.attrs.brush.as_ref().map(|b| b.diameter), Some(5));
    // keine Füllung
    s.tools.options.new_fill = Some(json!({"category": "none"}));
    tool(&mut s, ToolId::Ellipse);
    drag(&mut s, (150.0, 100.0), (200.0, 150.0));
    assert!(path_of(&s, last(&s)).attrs.fill.is_none());
}

#[test]
fn auto_shape_tool_drags_a_frame() {
    let mut s = doc_session(300.0, 200.0);
    tool(&mut s, ToolId::AutoShape);
    s.tools.options.auto_shape = "Pfeil".into();
    events(&mut s, &[ToolEvent::down(10.0, 10.0), ToolEvent::drag(30.0, 20.0)]);
    assert!(s.tool_feedback().iter().any(|o| matches!(o, Overlay::Rect { .. })), "Rahmen-Vorschau");
    // Das Werkzeug ruft `autoShape.insert {shape, rect}` (Formen liefert die Automatisierung)
    match s.tool_event(&ToolEvent::up(60.0, 40.0)) {
        Ok(r) => assert_eq!(r.commands.first().map(|c| (c.command.as_str(), c.params["shape"].as_str())), Some(("autoShape.insert", Some("Pfeil")))),
        // ohne fp-script-Laufzeit (reine Engine) meldet der Befehl das fehlende Backend
        Err(e) => assert!(e.to_string().contains("autoShape.insert") || e.to_string().contains("automatischen Formen"), "{e}"),
    }
}

#[test]
fn auto_shape_control_points_drag() {
    let mut s = doc_session(300.0, 200.0);
    let a = rect(&mut s, 10.0, 10.0, 110.0, 80.0);
    select(&mut s, &[a]);
    ex(&mut s, "modify.group", json!({}));
    let g = sel(&s)[0];
    // Gruppe zur automatischen Form machen (Steuerpunkt bei 30,10 mit Tooltip)
    let ss: fp_model::element::SmartShape = serde_json::from_value(json!({
        "name": "Test", "script_chunks": [],
        "control_points": [{"position": [30.0, 10.0], "kind": 0, "tooltip": "Radius", "tooltip_tracks_drag": false, "visible": true},
                           {"position": [60.0, 10.0], "kind": 0, "tooltip_tracks_drag": false, "visible": false}]
    }))
    .expect("SmartShape");
    {
        let d = s.active_mut().expect("d");
        let page = d.page;
        fp_engine::docops::edit_by_id(std::sync::Arc::make_mut(&mut d.doc), page, g, |e| {
            if let ElementKind::Group(gr) = &mut e.kind {
                gr.smart_shape = Some(ss.clone());
            }
        });
    }
    for t in [ToolId::Pointer, ToolId::AutoShape] {
        tool(&mut s, t);
        let fb = s.tool_feedback();
        assert_eq!(fb.iter().filter(|o| matches!(o, Overlay::ControlPoint { .. })).count(), 1, "{t:?}: nur sichtbare Steuerpunkte");
        events(&mut s, &[ToolEvent::down(30.0, 10.0), ToolEvent::drag(35.0, 12.0)]);
        assert!(s.tool_feedback().iter().any(|o| matches!(o, Overlay::ControlPoint { hot: true, at, .. } if at.x == 35.0)));
        let before = doc(&s);
        match s.tool_event(&ToolEvent::up(40.0, 15.0).shift()) {
            Ok(r) => {
                let c = r.commands.first().expect("Befehl");
                assert_eq!(c.command, "autoShape.dragControlPoint");
                assert_eq!((c.params["index"].as_u64(), c.params["to"][0].as_f64(), c.params["shift"].as_bool()), (Some(0), Some(40.0), Some(true)));
            }
            // reine Engine ohne Formen-Laufzeit: Befehl meldet das fehlende Backend, Dokument unverändert
            Err(e) => {
                assert!(e.to_string().contains("automatischen Formen") || e.to_string().contains("autoShape"), "{e}");
                assert_eq!(doc(&s), before);
            }
        }
        // Esc mitten im Ziehen: nichts
        events(&mut s, &[ToolEvent::down(30.0, 10.0), ToolEvent::drag(50.0, 12.0), ToolEvent::key(ToolKey::Escape)]);
        let _ = s.tool_event(&ToolEvent::up(50.0, 12.0));
        assert_eq!(doc(&s), before);
    }
}

#[test]
fn pen_continue_insert_remove_and_close() {
    let mut s = doc_session(300.0, 200.0);
    tool(&mut s, ToolId::Pen);
    let before = doc(&s);
    events(&mut s, &[ToolEvent::down(10.0, 10.0), ToolEvent::up(10.0, 10.0), ToolEvent::down(60.0, 10.0), ToolEvent::drag(70.0, 10.0), ToolEvent::up(70.0, 10.0), ToolEvent::key(ToolKey::Enter)]);
    let p = last(&s);
    let n = &path_of(&s, p).contours[0].nodes[1];
    assert_eq!((n.succ.x, n.pred.x), (70.0, 50.0), "Kurvenpunkt durch Ziehen");
    undo_redo(&mut s, &before);
    // Pfad fortsetzen am Endpunkt
    select(&mut s, &[p]);
    events(&mut s, &[ToolEvent::down(60.0, 10.0), ToolEvent::up(60.0, 10.0), ToolEvent::down(60.0, 60.0), ToolEvent::up(60.0, 60.0), ToolEvent::key(ToolKey::Enter)]);
    assert_eq!(path_of(&s, p).contours[0].nodes.len(), 3, "fortgesetzt statt neu");
    // Punkt auf dem Segment einfügen, Punkt entfernen
    select(&mut s, &[p]);
    click(&mut s, 60.0, 35.0);
    assert_eq!(path_of(&s, p).contours[0].nodes.len(), 4);
    select(&mut s, &[p]);
    click(&mut s, 60.0, 35.0);
    assert_eq!(path_of(&s, p).contours[0].nodes.len(), 3);
    // Rücktaste nimmt den letzten Punkt zurück, Klick auf den Start schließt
    ex(&mut s, "select.none", json!({}));
    events(&mut s, &[
        ToolEvent::down(100.0, 100.0),
        ToolEvent::up(100.0, 100.0),
        ToolEvent::down(150.0, 100.0),
        ToolEvent::up(150.0, 100.0),
        ToolEvent::down(170.0, 170.0),
        ToolEvent::up(170.0, 170.0),
        ToolEvent::key(ToolKey::Backspace),
        ToolEvent::down(150.0, 150.0),
        ToolEvent::up(150.0, 150.0),
        ToolEvent::down(100.0, 100.0),
        ToolEvent::up(100.0, 100.0),
    ]);
    let c = &path_of(&s, last(&s)).contours[0];
    assert!(c.closed && c.nodes.len() == 3, "{c:?}");
}

#[test]
fn text_tool_editor_keys_clipboard_and_undo() {
    let mut s = doc_session(300.0, 200.0);
    tool(&mut s, ToolId::Text);
    let before = doc(&s);
    click(&mut s, 20.0, 50.0);
    assert!(s.tool_is_editing_text());
    let send = |s: &mut Session, i: ToolInput| s.tool_event(&ToolEvent { input: i, mods: Default::default(), pressure: 1.0, time_ms: 0 }).expect("Ereignis");
    send(&mut s, ToolInput::Text("Hallo Welt".into()));
    // Caret per Layout (Pos1 der UI) an den Anfang, Strg+Rechts = Wortsprung
    send(&mut s, ToolInput::TextCaret { index: 0, extend: false });
    s.tool_event(&ToolEvent::key(ToolKey::Right).ctrl()).expect("Wort");
    // Umschalt+Ende markiert bis zum Ende, Ausschneiden liefert die Zwischenablage
    s.tool_event(&ToolEvent::key(ToolKey::End).shift()).expect("Ende");
    let r = send(&mut s, ToolInput::Edit(EditOp::Cut));
    assert_eq!(r.clipboard.as_deref(), Some("Welt"));
    send(&mut s, ToolInput::Edit(EditOp::Paste("Firepixel".into())));
    // IME-Vorschau erscheint im Editor, aber nicht im Text
    send(&mut s, ToolInput::Preedit("ab".into()));
    let fb = s.tool_feedback();
    assert!(fb.iter().any(|o| matches!(o, Overlay::TextEdit(v) if v.display_text() == "Hallo Firepixelab" && v.text == "Hallo Firepixel")));
    send(&mut s, ToolInput::Preedit(String::new()));
    // Strg+Rücktaste löscht ein Wort
    s.tool_event(&ToolEvent::key(ToolKey::Backspace).ctrl()).expect("Wort löschen");
    send(&mut s, ToolInput::Text("Welt".into()));
    s.tool_event(&ToolEvent::key(ToolKey::Escape)).expect("übernehmen");
    assert!(!s.tool_is_editing_text());
    let t = top_elements(&s).iter().find(|e| matches!(e.kind, ElementKind::Text(_))).map(|e| e.id).expect("Text");
    assert!(matches!(&el(&s, t).kind, ElementKind::Text(x) if x.plain_text() == "Hallo Welt"));
    assert_eq!(s.tools.active, ToolId::Text, "Esc: Werkzeug bleibt Text (wie im Original, UX-A L21/L22)");
    undo_redo(&mut s, &before);
    // Vorhandenen Text bearbeiten: alles markieren und ersetzen
    tool(&mut s, ToolId::Text);
    let b = bounds(&s, t);
    click(&mut s, b.left + 2.0, b.top + 2.0);
    let send = |s: &mut Session, i: ToolInput| s.tool_event(&ToolEvent { input: i, mods: Default::default(), pressure: 1.0, time_ms: 0 }).expect("Ereignis");
    send(&mut s, ToolInput::Edit(EditOp::SelectAll));
    send(&mut s, ToolInput::Text("Neu".into()));
    tool(&mut s, ToolId::Pointer);
    assert!(matches!(&el(&s, t).kind, ElementKind::Text(x) if x.plain_text() == "Neu"));
}

fn text_of(s: &Session, id: ElementId) -> fp_model::TextElem {
    match &el(s, id).kind {
        ElementKind::Text(t) => (**t).clone(),
        _ => panic!("kein Text"),
    }
}

fn send_in(s: &mut Session, i: ToolInput) {
    s.tool_event(&ToolEvent { input: i, mods: Default::default(), pressure: 1.0, time_ms: 0 }).expect("Ereignis");
}

/// Nutzerbefund: 200-pt-Text „Ha“ – Grenzen kommen aus dem Satz (Auswahlrahmen, Treffertest,
/// Ausrichten, Info), sofort nach jeder Größen-/Text-Änderung; Inspektor-Größe während der
/// Bearbeitung wirkt live im Editor (mit Markierung Strg+A auf den ganzen Text, N15), der Editor
/// bleibt offen.
#[test]
fn text_bounds_follow_layout_and_live_size_in_editor() {
    let mut s = doc_session(1200.0, 800.0);
    tool(&mut s, ToolId::Text);
    let before = doc(&s);
    click(&mut s, 50.0, 50.0);
    send_in(&mut s, ToolInput::Text("Ha".into()));
    let small = s.text_edit_frame().expect("Rahmen im Editor");
    // Inspektor ändert die Größe (so wie das Feld es schickt: text.setAttributes ohne ids)
    send_in(&mut s, ToolInput::Edit(EditOp::SelectAll));
    ex(&mut s, "text.setAttributes", json!({"size": 200}));
    assert!(s.tool_is_editing_text(), "Editor bleibt offen");
    assert_eq!(doc(&s), before, "während der Bearbeitung kein Dokumentschritt");
    assert_eq!(s.text_edit_attrs().and_then(|a| a.size), Some(200.0), "Inspektor zeigt die Editor-Größe");
    let big = s.text_edit_frame().expect("Rahmen im Editor");
    assert!(big.height() > small.height() * 8.0 && big.width() > small.width() * 8.0, "Rahmen wächst mit: {small:?} → {big:?}");
    // weitertippen (Einfügemarke ans Ende) behält die neue Größe
    s.tool_event(&ToolEvent::key(ToolKey::End)).expect("Ende");
    send_in(&mut s, ToolInput::Text("!".into()));
    s.tool_event(&ToolEvent::key(ToolKey::Escape)).expect("übernehmen");
    let t = last(&s);
    let tx = text_of(&s, t);
    assert_eq!(tx.plain_text(), "Ha!");
    assert!(tx.resolved_runs().iter().all(|(a, _)| a.size == Some(200.0)), "{:?}", tx.runs);
    let b = bounds(&s, t);
    assert!(b.height() > 180.0 && b.width() > 250.0, "Grenzen aus dem Satz, nicht geschätzt: {b:?}");
    let frame = s.renderer().and_then(|r| r.text_frame(&s.active().unwrap().doc, &tx)).expect("Satz");
    assert_eq!(b, frame, "Grenzen = Layout-Rahmen");
    // Treffertest in der unteren rechten Ecke des großen Rahmens (vorher außerhalb der Schätzung)
    tool(&mut s, ToolId::Pointer);
    select(&mut s, &[]);
    click(&mut s, b.right - 20.0, b.bottom - 30.0);
    assert_eq!(sel(&s), vec![t], "Klick in den großen Text trifft ihn");
    // ein Verlaufsschritt für den ganzen Text
    s.undo().expect("Rückgängig");
    assert_eq!(doc(&s), before);
    s.redo().expect("Wiederholen");
    assert_eq!(bounds(&s, t), b);
    // Größe per Inspektor ohne Editor (Auswahl): Grenzen sofort neu, Rückgängig exakt
    select(&mut s, &[t]);
    let big_doc = doc(&s);
    ex(&mut s, "text.setAttributes", json!({"size": 20}));
    let b20 = bounds(&s, t);
    assert!(b20.height() < 40.0 && b20.width() < 60.0, "{b20:?}");
    s.undo().expect("Rückgängig");
    assert_eq!(doc(&s), big_doc);
    // Menü Text ▸ Größe und Text ▸ Schrift ändern die Grenzen ebenfalls
    ex(&mut s, "text.size.12", json!({}));
    assert!(bounds(&s, t).height() < 25.0);
    // Verschieben behält den Rahmen (keine Neuberechnung → Rundreise stabil)
    let b12 = bounds(&s, t);
    ex(&mut s, "edit.move", json!({"dx": 10, "dy": 5}));
    assert_eq!(bounds(&s, t), b12.translate(10.0, 5.0));
}

/// PARITAET_A T6 (PM-1, Original gemessen): Text „Abc“ tippen, Größe 48 im Inspektor ohne
/// Markierung, „D“ tippen, Esc → Läufe [Abc 24 pt][D 48 pt]: die neue Größe gilt nur für danach
/// getippte Zeichen; der Inspektor zeigt sie schon an der Einfügemarke. Wandert die Einfügemarke
/// vorher weg, gilt sie nicht mehr.
#[test]
fn text_size_at_caret_applies_to_next_characters_only() {
    let mut s = doc_session(800.0, 400.0);
    s.tools.options.text_size = 24.0;
    tool(&mut s, ToolId::Text);
    click(&mut s, 50.0, 100.0);
    send_in(&mut s, ToolInput::Text("A".into()));
    send_in(&mut s, ToolInput::Text("b".into()));
    send_in(&mut s, ToolInput::Text("c".into()));
    let frame = s.text_edit_frame().expect("Rahmen");
    ex(&mut s, "text.setAttributes", json!({"size": 48}));
    assert_eq!(s.text_edit_attrs().and_then(|a| a.size), Some(48.0), "Inspektor zeigt 48 an der Einfügemarke");
    assert_eq!(s.text_edit_frame(), Some(frame), "vorhandener Text bleibt 24 pt");
    // jede Taste ein Ereignis, wie das Feld beim Tippen von „48“: wiederholt = einmal
    ex(&mut s, "text.setAttributes", json!({"size": 4}));
    ex(&mut s, "text.setAttributes", json!({"size": 48}));
    send_in(&mut s, ToolInput::Text("D".into()));
    s.tool_event(&ToolEvent::key(ToolKey::Escape)).expect("Esc");
    let tx = text_of(&s, last(&s));
    let runs: Vec<(String, Option<f32>)> = tx.resolved_runs().into_iter().map(|(a, t)| (t.to_string(), a.size)).collect();
    assert_eq!(runs, vec![("Abc".to_string(), Some(24.0)), ("D".to_string(), Some(48.0))]);
    assert_eq!(s.tools.active, ToolId::Text);
    // Einfügemarke verschoben: die ausstehende Größe verfällt
    tool(&mut s, ToolId::Text);
    click(&mut s, 50.0, 300.0);
    send_in(&mut s, ToolInput::Text("xy".into()));
    ex(&mut s, "text.setAttributes", json!({"size": 60}));
    s.tool_event(&ToolEvent::key(ToolKey::Left)).expect("links");
    send_in(&mut s, ToolInput::Text("z".into()));
    s.tool_event(&ToolEvent::key(ToolKey::Escape)).expect("Esc");
    let tx = text_of(&s, last(&s));
    assert_eq!(tx.plain_text(), "xzy");
    assert!(tx.resolved_runs().iter().all(|(a, _)| a.size == Some(24.0)), "{:?}", tx.runs);
}

/// Markierung im Editor: Größe/Farbe nur für die Markierung, Ausrichtung für den Absatz, Strg+Z
/// im Editor nimmt die Formatierung zurück; beim Übernehmen entstehen mehrere Läufe.
#[test]
fn text_editor_attributes_apply_to_selection_only() {
    let mut s = doc_session(800.0, 400.0);
    ex(&mut s, "draw.text", json!({"text": "Hallo Welt", "anchor": [40, 100], "size": 20}));
    let t = last(&s);
    let b0 = bounds(&s, t);
    tool(&mut s, ToolId::Text);
    click(&mut s, b0.left + 3.0, b0.top + 5.0);
    assert!(s.tool_is_editing_text());
    // „Welt“ markieren
    send_in(&mut s, ToolInput::TextCaret { index: 6, extend: false });
    send_in(&mut s, ToolInput::TextCaret { index: 10, extend: true });
    ex(&mut s, "text.setAttributes", json!({"size": 60, "color": "#ff0000"}));
    // Menü während der Bearbeitung: Fett für die Markierung
    ex(&mut s, "text.style.fauxBold", json!({}));
    let a = s.text_edit_attrs().expect("Attribute");
    assert_eq!((a.size, a.color, a.bold), (Some(60.0), Some(0xffff_0000), Some(true)));
    // Strg+Z im Editor: Fett zurück, Größe bleibt
    send_in(&mut s, ToolInput::Edit(EditOp::Undo));
    assert_eq!(s.text_edit_attrs().and_then(|a| a.bold), Some(false));
    // Ausrichtung zentriert (Absatzattribut) gilt für den ganzen Absatz
    ex(&mut s, "text.align.centeredHorizontally", json!({}));
    tool(&mut s, ToolId::Pointer);
    let tx = text_of(&s, t);
    assert_eq!(tx.plain_text(), "Hallo Welt");
    let runs = tx.resolved_runs();
    let hallo = runs.iter().find(|(_, s)| s.starts_with("Hallo")).expect("Lauf Hallo");
    let welt = runs.iter().find(|(_, s)| s.contains("Welt")).expect("Lauf Welt");
    assert_eq!(hallo.0.size, Some(20.0));
    assert_eq!(welt.0.size, Some(60.0));
    assert_eq!(welt.0.color, Some(0xffff_0000));
    assert!(runs.iter().all(|(a, _)| a.alignment == Some(1)), "Absatz zentriert");
    assert!(bounds(&s, t).height() > b0.height() * 2.0, "Rahmen wächst mit der größten Schrift");
    // ein Verlaufsschritt
    s.undo().expect("Rückgängig");
    assert_eq!(text_of(&s, t).runs.len(), 1);
    assert_eq!(bounds(&s, t), b0);
}

/// Logikfehler (§4b): Formatieren ohne offenen Editor ändert den Editor nicht; Esc ohne Änderung
/// erzeugt keinen Schritt; leerer neuer Text mit geänderter Größe erzeugt nichts; Werkzeugwechsel
/// übernimmt die Formatierung.
#[test]
fn text_editor_attribute_edge_cases() {
    let mut s = doc_session(600.0, 300.0);
    ex(&mut s, "draw.text", json!({"text": "Abc", "anchor": [40, 100], "size": 30}));
    let t = last(&s);
    let b0 = bounds(&s, t);
    // unveränderter Editor: kein Schritt
    tool(&mut s, ToolId::Text);
    let d0 = doc(&s);
    let hist = s.active().unwrap().history.len();
    click(&mut s, b0.left + 3.0, b0.top + 5.0);
    s.tool_event(&ToolEvent::key(ToolKey::Escape)).expect("Esc");
    assert_eq!(doc(&s), d0);
    assert_eq!(s.active().unwrap().history.len(), hist, "kein Schritt ohne Änderung");
    // leerer neuer Text: Größe ändern, dann Esc → nichts
    tool(&mut s, ToolId::Text);
    click(&mut s, 300.0, 200.0);
    ex(&mut s, "text.setAttributes", json!({"size": 90}));
    s.tool_event(&ToolEvent::key(ToolKey::Escape)).expect("Esc");
    assert_eq!(doc(&s), d0);
    // neuer Text: erst Größe, dann tippen → Text in 90 px; Werkzeugwechsel übernimmt
    tool(&mut s, ToolId::Text);
    click(&mut s, 300.0, 200.0);
    ex(&mut s, "text.size.other", json!({"size": 90}));
    send_in(&mut s, ToolInput::Text("X".into()));
    tool(&mut s, ToolId::Rectangle);
    let x = last(&s);
    assert_ne!(x, t);
    assert_eq!(text_of(&s, x).resolved_runs()[0].0.size, Some(90.0));
    assert!(bounds(&s, x).height() > 60.0);
    // ids-Parameter: wirkt auf das Objekt, nicht auf einen Editor (hier keiner offen)
    ex(&mut s, "text.setAttributes", json!({"ids": [t.0], "size": 10}));
    assert!(bounds(&s, t).height() < b0.height());
}

/// UX-A W14/W18, W3: Textwerkzeug mit Vorgabeschrift des Originals (Myriad Pro, falls installiert,
/// sonst `Text_Font`) und eigener Textfarbe (schwarz) statt der Füllfarbe; Radiergummi-Rand 2.
#[test]
fn text_tool_uses_original_font_and_own_text_color() {
    let mut s = doc_session(400.0, 200.0);
    let o = &s.tools.options;
    let myriad = fp_text::FontDb::shared().find_family("Myriad Pro", false, false).is_some();
    assert_eq!(o.text_font, if myriad { "Myriad Pro" } else { "Arial" });
    assert_eq!(o.text_color, fp_color::Argb(0xff00_0000));
    assert_eq!((o.eraser_size, o.eraser_edge), (13.0, 2));
    // Füllfarbe grau wie im Werkzeugbedienfeld
    s.set_colors(None, Some(fp_color::Argb(0xff99_9999)));
    tool(&mut s, ToolId::Text);
    click(&mut s, 20.0, 60.0);
    send_in(&mut s, ToolInput::Text("Hallo".into()));
    s.tool_event(&ToolEvent::key(ToolKey::Escape)).expect("übernehmen");
    let tx = text_of(&s, last(&s));
    let (a, _) = &tx.resolved_runs()[0];
    assert_eq!(a.color, Some(0xff00_0000), "Text schwarz, nicht die Füllfarbe");
    assert_eq!(tx.attrs.fill_color, fp_color::Argb(0xff00_0000));
    assert_eq!(a.font.as_deref(), Some(s.tools.options.text_font.as_str()));
    // eigene Textfarbe des Werkzeugs gilt auch für draw.text ohne Farbe
    s.tools.options.text_color = fp_color::Argb(0xff12_3456);
    ex(&mut s, "draw.text", json!({"text": "X", "anchor": [10, 150]}));
    assert_eq!(text_of(&s, last(&s)).resolved_runs()[0].0.color, Some(0xff12_3456));
}

/// UX-A L1, L13, L22, L25, O4: Gummiband wählt berührte Objekte; Esc im Zauberstab beendet den
/// Bitmapmodus (Auswahlrahmen weg, Zeiger, Bitmap bleibt gewählt, ein Schritt); Klick mit dem
/// Textwerkzeug = Grundlinie, Zeilenabstand 120 %, Werkzeug bleibt Text; Zoomrahmen stufenlos.
#[test]
fn parity_ux_a_rubber_escape_text_zoom() {
    let mut s = doc_session(800.0, 600.0);
    let r1 = rect(&mut s, 50.0, 50.0, 150.0, 120.0);
    select(&mut s, &[]);
    tool(&mut s, ToolId::Pointer);
    drag(&mut s, (20.0, 20.0), (100.0, 100.0));
    assert_eq!(sel(&s), vec![r1], "teilweise berührt zählt");
    // Zauberstab + Esc
    let b = bitmap(&mut s, 300, 50, 60, 60, [200, 0, 0, 255]);
    select(&mut s, &[b]);
    tool(&mut s, ToolId::MagicWand);
    click(&mut s, 320.0, 70.0);
    assert!(s.active().unwrap().selection.has_pixels(), "Auswahlrahmen");
    let n = s.active().unwrap().history.len();
    s.tool_event(&ToolEvent::key(ToolKey::Escape)).expect("Esc");
    let d = s.active().unwrap();
    assert!(!d.selection.has_pixels() && !d.bitmap_mode, "Bitmapmodus beendet");
    assert_eq!(d.selection.elements, vec![b], "Bitmap bleibt gewählt");
    assert_eq!(s.tools.active, ToolId::Pointer);
    assert_eq!(s.active().unwrap().history.len(), n + 1, "Verlaufsschritt");
    // Esc ohne Bitmapmodus und Auswahlrahmen: nichts
    tool(&mut s, ToolId::MagicWand);
    let n = s.active().unwrap().history.len();
    s.tool_event(&ToolEvent::key(ToolKey::Escape)).expect("Esc");
    assert_eq!((s.tools.active, s.active().unwrap().history.len()), (ToolId::MagicWand, n));
    // Text per Klick: Klickpunkt = Grundlinie, 120 %, Werkzeug bleibt Text
    tool(&mut s, ToolId::Text);
    click(&mut s, 300.0, 340.0);
    send_in(&mut s, ToolInput::Text("Abc".into()));
    s.tool_event(&ToolEvent::key(ToolKey::Escape)).expect("Esc");
    assert_eq!(s.tools.active, ToolId::Text);
    let t = last(&s);
    let tx = text_of(&s, t);
    assert_eq!(tx.anchor_rect.map(|r| (r.left, r.top)), Some((300.0, 340.0)), "Grundlinie am Klickpunkt");
    assert_eq!(tx.resolved_runs()[0].0.leading, Some(1.2));
    let bt = bounds(&s, t);
    assert!(bt.top < 340.0 && bt.bottom > 340.0 && bt.bottom < 345.0, "{bt:?}");
    // auch per Skript neue Texte mit 120 %
    ex(&mut s, "draw.text", json!({"text": "S", "anchor": [10, 500]}));
    assert_eq!(text_of(&s, last(&s)).initial.leading, Some(1.2));
    // Zoomrahmen 50,50–150,120: stufenlos eingepasst (keine Tabellenstufe)
    ex(&mut s, "view.zoomToRect", json!({"rect": [50, 50, 150, 120]}));
    let z = s.active().unwrap().view.zoom;
    assert!(![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 8.0, 12.0, 16.0].iter().any(|t| (z - t).abs() < 1e-9), "stufenlos: {z}");
}

#[test]
fn freehand_tools() {
    let mut s = doc_session(300.0, 200.0);
    tool(&mut s, ToolId::VectorPath);
    let before = doc(&s);
    let mut evs = vec![ToolEvent::down(0.0, 50.0)];
    for i in 1..40 {
        let x = i as f32 * 3.0;
        evs.push(ToolEvent::drag(x, 50.0 + (x / 12.0).sin() * 15.0));
    }
    evs.push(ToolEvent::up(120.0, 50.0));
    events(&mut s, &evs);
    let p = last(&s);
    assert!(path_of(&s, p).contours[0].nodes.len() >= 3);
    undo_redo(&mut s, &before);
    // Umschalt = gerade Linie
    events(&mut s, &[ToolEvent::down(0.0, 150.0), ToolEvent::drag(40.0, 160.0).shift(), ToolEvent::drag(80.0, 150.0).shift(), ToolEvent::up(80.0, 150.0).shift()]);
    assert_eq!(path_of(&s, last(&s)).contours[0].nodes.len(), 2);
    // Pfad nachzeichnen ersetzt die Konturen des ausgewählten Pfads
    select(&mut s, &[p]);
    tool(&mut s, ToolId::RedrawPath);
    let n = top_elements(&s).len();
    drag(&mut s, (0.0, 100.0), (100.0, 100.0));
    assert_eq!(top_elements(&s).len(), n);
    assert!(bounds(&s, p).top > 90.0);
}

// ------------------------------------------------------------------ Transformieren

/// PARITAET_A L30 (PM-1, `uxp_pm_3.ps1`): Frei transformieren (Strg+T) an R1 50,50–150,120, Eckgriff
/// von 151,121 nach 191,151. Original: Ecke immer proportional mit dem größeren Faktor
/// (50,50–193,150), Umschalt ändert nichts; Alt = proportional von der Mitte (7,20–193,150); Kante
/// rechts folgt dem Zeiger (191); die Änderung ist schon nach dem Loslassen sichtbar; Esc behält sie
/// und wechselt zum Zeiger; Eingabe/Doppelklick übernehmen, ein Verlaufsschritt.
#[test]
fn free_transform_like_original() {
    fn setup() -> (Session, ElementId) {
        let mut s = doc_session(600.0, 400.0);
        let r1 = rect(&mut s, 50.0, 50.0, 150.0, 120.0);
        rect(&mut s, 450.0, 150.0, 520.0, 200.0); // R2 (Oberkante 150)
        ellipse(&mut s, 200.0, 50.0, 300.0, 120.0); // E1
        select(&mut s, &[r1]);
        ex(&mut s, "modify.transform.freeTransform", json!({}));
        assert_eq!(s.tools.active, ToolId::Scale);
        (s, r1)
    }
    fn pull(s: &mut Session, from: (f32, f32), to: (f32, f32), mods: &str) {
        let m = |e: ToolEvent| {
            let e = if mods.contains('s') { e.shift() } else { e };
            if mods.contains('a') { e.alt() } else { e }
        };
        let mut seq = vec![m(ToolEvent::drag(from.0, from.1)), m(ToolEvent::down(from.0, from.1))];
        for i in 1..=20 {
            let f = i as f32 / 20.0;
            seq.push(m(ToolEvent::drag((from.0 + (to.0 - from.0) * f).round(), (from.1 + (to.1 - from.1) * f).round())));
        }
        seq.push(m(ToolEvent::up(to.0, to.1)));
        events(s, &seq);
    }
    let px = |r: Rect| (r.left.round(), r.top.round(), r.right.round(), r.bottom.round());
    for (mods, want) in [("", (50.0, 50.0, 193.0, 150.0)), ("s", (50.0, 50.0, 193.0, 150.0)), ("a", (7.0, 20.0, 193.0, 150.0)), ("as", (7.0, 20.0, 193.0, 150.0))] {
        let (mut s, r1) = setup();
        let n = s.active().unwrap().history.undo_count();
        pull(&mut s, (151.0, 121.0), (191.0, 151.0), mods);
        // nach dem Loslassen: Vorschau mit der neuen Größe sichtbar
        let fb = s.tool_feedback();
        let ghost = fb.iter().find_map(|o| match o {
            Overlay::Ghost { quad, .. } => Rect::from_points(*quad),
            _ => None,
        });
        assert_eq!(ghost.map(px), Some(want), "Vorschau nach dem Loslassen, Tasten {mods:?}");
        s.tool_event(&ToolEvent::key(ToolKey::Enter)).expect("Eingabe");
        assert_eq!(px(bounds(&s, r1)), want, "Tasten {mods:?}");
        assert_eq!(s.tools.active, ToolId::Pointer);
        assert_eq!(s.active().unwrap().history.undo_count(), n + 1, "ein Schritt");
        s.undo().expect("undo");
        assert_eq!(px(bounds(&s, r1)), (50.0, 50.0, 150.0, 120.0));
    }
    // Kantengriff rechts: die Kante folgt dem Zeiger
    let (mut s, r1) = setup();
    pull(&mut s, (151.0, 85.0), (191.0, 85.0), "");
    s.tool_event(&ToolEvent::key(ToolKey::Enter)).expect("Eingabe");
    assert_eq!(px(bounds(&s, r1)), (50.0, 50.0, 191.0, 120.0));
    // Esc behält die Änderung, beendet den Modus (Zeiger), ein Schritt; Strg+Z nimmt alles zurück
    let (mut s, r1) = setup();
    let n = s.active().unwrap().history.undo_count();
    pull(&mut s, (151.0, 121.0), (191.0, 151.0), "");
    s.tool_event(&ToolEvent::key(ToolKey::Escape)).expect("Esc");
    assert_eq!(px(bounds(&s, r1)), (50.0, 50.0, 193.0, 150.0), "Esc behält");
    assert_eq!(s.tools.active, ToolId::Pointer, "Werkzeug danach Zeiger");
    assert_eq!(s.active().unwrap().history.undo_count(), n + 1);
    // der nächste Klick wählt (Zeiger) statt zu skalieren
    click(&mut s, 400.0, 300.0);
    assert!(sel(&s).is_empty(), "Klick ins Leere hebt die Auswahl auf");
    s.undo().expect("undo");
    assert_eq!(px(bounds(&s, r1)), (50.0, 50.0, 150.0, 120.0));
    // Doppelklick innen übernimmt
    let (mut s, r1) = setup();
    pull(&mut s, (151.0, 121.0), (191.0, 151.0), "");
    events(&mut s, &[ToolEvent::down(100.0, 85.0), ToolEvent::up(100.0, 85.0), ToolEvent::double(100.0, 85.0)]);
    assert_eq!(px(bounds(&s, r1)), (50.0, 50.0, 193.0, 150.0));
    assert_eq!(s.tools.active, ToolId::Pointer);
    // Esc ohne Änderung: Modus beenden, Dokument unverändert
    let (mut s, _) = setup();
    let before = doc(&s);
    s.tool_event(&ToolEvent::key(ToolKey::Escape)).expect("Esc");
    assert_eq!((doc(&s), s.tools.active), (before, ToolId::Pointer));
}

#[test]
fn scale_skew_distort_and_rotate() {
    let mut s = doc_session(300.0, 200.0);
    let a = rect(&mut s, 10.0, 10.0, 50.0, 50.0);
    tool(&mut s, ToolId::Scale);
    assert!(s.tool_feedback().iter().any(|o| matches!(o, Overlay::TransformBox { .. })), "Rahmen gleich nach dem Werkzeugwechsel");
    let before = doc(&s);
    // Umschalt = proportional
    events(&mut s, &[ToolEvent::down(50.0, 50.0), ToolEvent::drag(70.0, 60.0).shift(), ToolEvent::drag(90.0, 70.0).shift(), ToolEvent::up(90.0, 70.0).shift()]);
    // wie im Original: bis Eingabe/Doppelklick/Werkzeugwechsel nur Vorschau
    assert_eq!(doc(&s), before, "noch Vorschau");
    assert!(s.tool_feedback().iter().any(|o| matches!(o, Overlay::Ghost { .. })), "Vorschau sichtbar");
    // zweiter Zug auf derselben Vorschau, dann Eingabe = EIN Verlaufsschritt
    let hist = s.active().map(|d| d.history.undo_count()).unwrap_or(0);
    let q = s.tool_feedback().iter().find_map(|o| match o {
        Overlay::TransformBox { quad, .. } => Some(quad[2]),
        _ => None,
    });
    let c = q.expect("Rahmen der Vorschau");
    events(&mut s, &[ToolEvent::down(c.x, c.y), ToolEvent::drag(c.x + 5.0, c.y + 5.0).shift(), ToolEvent::drag(c.x + 10.0, c.y + 10.0).shift(), ToolEvent::up(c.x + 10.0, c.y + 10.0).shift()]);
    events(&mut s, &[ToolEvent::key(ToolKey::Enter)]);
    let b = bounds(&s, a);
    assert!(approx(b.width(), b.height()) && approx(b.right, c.x + 10.0), "proportional, zweiter Zug auf der Vorschau {b:?} {c:?}");
    assert_eq!(s.active().map(|d| d.history.undo_count()).unwrap_or(0), hist + 1, "ein Schritt");
    undo_redo(&mut s, &before);
    // Esc mitten im Zug bricht nur den Zug ab: Ausgangslage
    tool(&mut s, ToolId::Scale);
    let before = doc(&s);
    let b0 = bounds(&s, a);
    events(&mut s, &[ToolEvent::down(b0.right, b0.bottom), ToolEvent::drag(b0.right + 20.0, b0.bottom + 20.0), ToolEvent::key(ToolKey::Escape)]);
    s.finish_tool().expect("Werkzeugwechsel");
    assert_eq!(doc(&s), before, "Esc im Zug bricht das Skalieren ab");
    // Alt = um die Mitte (Werkzeugwechsel übernimmt)
    tool(&mut s, ToolId::Scale);
    events(&mut s, &[ToolEvent::down(b0.right, b0.bottom), ToolEvent::drag(b0.right + 5.0, b0.bottom + 5.0).alt(), ToolEvent::drag(b0.right + 10.0, b0.bottom + 10.0).alt(), ToolEvent::up(b0.right + 10.0, b0.bottom + 10.0).alt()]);
    tool(&mut s, ToolId::Skew);
    assert!(approx(bounds(&s, a).center().x, b0.center().x));
    let b = bounds(&s, a);
    drag(&mut s, (b.center().x, b.top), (b.center().x + 20.0, b.top));
    s.finish_tool().expect("übernehmen");
    assert!(bounds(&s, a).width() > b.width());
    let a = rect(&mut s, 200.0, 100.0, 260.0, 150.0);
    tool(&mut s, ToolId::Distort);
    let b = bounds(&s, a);
    drag(&mut s, (b.right, b.bottom), (b.right + 20.0, b.bottom + 10.0));
    let r = s.tool_event(&ToolEvent::key(ToolKey::Enter)).expect("Eingabe");
    assert!(bounds(&s, a).right > b.right, "{:?} {b:?} {:?}", r.commands, bounds(&s, a));
    assert_eq!(s.tools.active, ToolId::Pointer);
}

#[test]
fn nine_slice_keeps_corners() {
    let mut s = doc_session(400.0, 300.0);
    let a = rect(&mut s, 10.0, 10.0, 70.0, 40.0);
    ex(&mut s, "pi.roundness", json!({"value": 8, "pixels": true}));
    tool(&mut s, ToolId::NineSliceScale);
    let fb = s.tool_feedback();
    assert_eq!(fb.iter().filter(|o| matches!(o, Overlay::SmartGuide { .. })).count(), 4, "vier 9-Slice-Hilfslinien");
    // Hilfslinie links verschieben (Ränder in px), dann rechts unten auf das Doppelte ziehen
    events(&mut s, &[ToolEvent::down(30.0, 25.0), ToolEvent::drag(25.0, 25.0), ToolEvent::drag(20.0, 25.0), ToolEvent::up(20.0, 25.0)]);
    let before = doc(&s);
    drag(&mut s, (70.0, 40.0), (130.0, 70.0));
    s.finish_tool().expect("übernehmen");
    assert!(rect_approx(bounds(&s, a), Rect::new(10.0, 10.0, 130.0, 70.0)), "{:?}", bounds(&s, a));
    let p = path_of(&s, a);
    let pr = p.primitive.as_ref().expect("Grundform bleibt");
    let px = pr.roundness * pr.rect.width().min(pr.rect.height()) / 2.0;
    assert!((px - 8.0).abs() < 1e-3, "Eckenradius bleibt 8 px, ist {px}");
    undo_redo(&mut s, &before);
    // Pfad: Punkte im Eckbereich behalten ihren Abstand zur Ecke
    let e = ellipse(&mut s, 200.0, 200.0, 260.0, 260.0);
    tool(&mut s, ToolId::NineSliceScale);
    drag(&mut s, (260.0, 260.0), (320.0, 260.0));
    s.finish_tool().expect("übernehmen");
    let c = &path_of(&s, e).contours[0];
    let left = c.nodes.iter().map(|n| n.anchor.x).fold(f32::INFINITY, f32::min);
    assert!(approx(left, 200.0) && approx(bounds(&s, e).right, 320.0));
}

// ------------------------------------------------------------------ Pfade bearbeiten

#[test]
fn freeform_reshape_knife_and_scrub() {
    let mut s = doc_session(300.0, 200.0);
    tool(&mut s, ToolId::Line);
    drag(&mut s, (10.0, 100.0), (210.0, 100.0));
    let l = last(&s);
    select(&mut s, &[l]);
    // Freiform: am Pfad ziehen
    tool(&mut s, ToolId::Freeform);
    let before = doc(&s);
    drag(&mut s, (110.0, 100.0), (110.0, 130.0));
    assert!(bounds(&s, l).bottom > 115.0);
    undo_redo(&mut s, &before);
    // Bereich umformen
    tool(&mut s, ToolId::Reshape);
    let before = doc(&s);
    drag(&mut s, (60.0, 100.0), (60.0, 80.0));
    assert!(bounds(&s, l).top < 95.0);
    undo_redo(&mut s, &before);
    // Pfadausbesserung: subtraktiv senkt, additiv hebt den Druck
    let pressure = |s: &Session| -> f32 {
        path_of(s, l).contours[0].nodes.iter().filter_map(|n| n.pressure.as_ref()).flat_map(|p| p.samples.iter().filter_map(|x| x.pressure)).fold(1.0, f32::min)
    };
    tool(&mut s, ToolId::PathScrubSubtract);
    let before = doc(&s);
    drag(&mut s, (20.0, 100.0), (200.0, 100.0));
    let low = pressure(&s);
    assert!(low < 1.0, "Druck gesenkt: {low}");
    undo_redo(&mut s, &before);
    tool(&mut s, ToolId::PathScrubAdd);
    drag(&mut s, (20.0, 100.0), (200.0, 100.0));
    assert!(pressure(&s) > low);
    // Messer teilt den Pfad
    tool(&mut s, ToolId::Knife);
    let n = top_elements(&s).len();
    let before = doc(&s);
    events(&mut s, &[ToolEvent::down(150.0, 50.0), ToolEvent::drag(150.0, 100.0).shift(), ToolEvent::drag(150.0, 160.0).shift(), ToolEvent::up(150.0, 160.0).shift()]);
    assert_eq!(top_elements(&s).len(), n + 1);
    undo_redo(&mut s, &before);
}

// ------------------------------------------------------------------ Farben und Bitmap

#[test]
fn eyedropper_bucket_gradient() {
    let mut s = doc_session(300.0, 200.0);
    let b = pattern_bitmap(&mut s, 100, 100);
    tool(&mut s, ToolId::Eyedropper);
    click(&mut s, 135.0, 135.0);
    assert_eq!(s.tools.fill_color.0 & 0xffffff, 0xdc1e1e, "Füllfarbe = Rot");
    events(&mut s, &[ToolEvent::down(105.0, 105.0).alt(), ToolEvent::up(105.0, 105.0).alt()]);
    assert_eq!(s.tools.stroke_color.0 & 0xffffff, 0x141414, "Alt = Strichfarbe");
    // Farbeimer auf der Bitmap (Flutfüllung)
    select(&mut s, &[b]);
    s.set_colors(None, Some(fp_color::Argb(0xff00_00ff)));
    tool(&mut s, ToolId::PaintBucket);
    let before = doc(&s);
    click(&mut s, 105.0, 105.0);
    assert_eq!(px(&s, b, 105, 105), [0, 0, 255, 255]);
    assert_eq!(px(&s, b, 125, 105)[2], 200, "andere Farbe bleibt");
    undo_redo(&mut s, &before);
    // Verlauf auf Vektor
    let r = rect(&mut s, 10.0, 10.0, 60.0, 60.0);
    select(&mut s, &[r]);
    tool(&mut s, ToolId::Gradient);
    let before = doc(&s);
    drag(&mut s, (10.0, 30.0), (60.0, 30.0));
    assert!(fp_engine::docops::path_attrs_of(&el(&s, r)).and_then(|a| a.fill.as_ref().map(|f| f.gradient.is_some())).unwrap_or(false));
    undo_redo(&mut s, &before);
}

#[test]
fn pencil_brush_eraser() {
    let mut s = doc_session(300.0, 200.0);
    let b = pattern_bitmap(&mut s, 100, 100);
    select(&mut s, &[b]);
    s.set_colors(Some(fp_color::Argb(0xff00_ff00)), None);
    tool(&mut s, ToolId::Pencil);
    let before = doc(&s);
    events(&mut s, &[ToolEvent::down(122.0, 115.0), ToolEvent::drag(130.0, 115.0), ToolEvent::up(135.0, 115.0)]);
    assert_eq!(px(&s, b, 126, 115), [0, 255, 0, 255]);
    undo_redo(&mut s, &before);
    tool(&mut s, ToolId::Brush);
    let before = doc(&s);
    events(&mut s, &[ToolEvent::down(122.0, 118.0), ToolEvent::drag(130.0, 118.0), ToolEvent::up(135.0, 118.0)]);
    assert_ne!(px(&s, b, 128, 118), [200, 200, 200, 255]);
    undo_redo(&mut s, &before);
    tool(&mut s, ToolId::Eraser);
    let before = doc(&s);
    click(&mut s, 110.0, 120.0);
    assert_eq!(px(&s, b, 110, 120)[3], 0);
    undo_redo(&mut s, &before);
}

#[test]
fn retouch_tools_change_pixels_and_undo() {
    for t in [ToolId::Blur, ToolId::Sharpen, ToolId::Dodge, ToolId::Burn, ToolId::Smudge, ToolId::ReplaceColor, ToolId::RedEyeRemoval, ToolId::RubberStamp] {
        let mut s = doc_session(300.0, 200.0);
        let b = pattern_bitmap(&mut s, 100, 100);
        select(&mut s, &[b]);
        tool(&mut s, t);
        let img0 = image(&s, b);
        let before = doc(&s);
        let (a, z) = match t {
            ToolId::ReplaceColor => ((132.0, 104.0), (138.0, 104.0)),
            ToolId::RedEyeRemoval => ((132.0, 132.0), (136.0, 136.0)),
            ToolId::Dodge | ToolId::Burn => ((122.0, 115.0), (128.0, 115.0)),
            _ => ((115.0, 115.0), (125.0, 115.0)),
        };
        if t == ToolId::RubberStamp {
            // ohne Quelle: Hinweis, keine Änderung; Alt+Klick setzt die Quelle (schwarz) → auf Grau malen
            let r = s.tool_event(&ToolEvent::down(125.0, 115.0)).expect("Ereignis");
            assert!(r.status.is_some());
            events(&mut s, &[ToolEvent::up(125.0, 115.0), ToolEvent::down(105.0, 115.0).alt(), ToolEvent::up(105.0, 115.0).alt()]);
            drag(&mut s, (125.0, 115.0), (128.0, 115.0));
        } else {
            drag(&mut s, a, z);
        }
        assert_ne!(image(&s, b).as_bytes(), img0.as_bytes(), "{t:?} ändert keine Pixel");
        undo_redo(&mut s, &before);
    }
}

// ------------------------------------------------------------------ Web und Ansicht

#[test]
fn web_tools() {
    let mut s = doc_session(300.0, 200.0);
    for (t, js) in [(ToolId::HotspotRect, "Hotspot"), (ToolId::HotspotCircle, "Hotspot"), (ToolId::Slice, "SliceHotspot")] {
        tool(&mut s, t);
        let before = doc(&s);
        drag(&mut s, (10.0, 10.0), (50.0, 40.0));
        assert_eq!(el(&s, last(&s)).js_type(), js, "{t:?}");
        undo_redo(&mut s, &before);
    }
    for t in [ToolId::HotspotPolygon, ToolId::PolygonSlice] {
        tool(&mut s, t);
        let before = doc(&s);
        events(&mut s, &[ToolEvent::down(100.0, 100.0), ToolEvent::down(140.0, 100.0), ToolEvent::down(120.0, 140.0), ToolEvent::key(ToolKey::Enter)]);
        assert!(matches!(&el(&s, last(&s)).kind, ElementKind::WebObject(w) if w.polygon.is_some()), "{t:?}");
        undo_redo(&mut s, &before);
    }
}

#[test]
fn hand_and_zoom() {
    let mut s = doc_session(300.0, 200.0);
    ex(&mut s, "view.setViewport", json!({"width": 400, "height": 400}));
    tool(&mut s, ToolId::Zoom);
    click(&mut s, 100.0, 100.0);
    let z1 = s.active().map(|d| d.view.zoom).unwrap_or(0.0);
    assert!(z1 > 1.0);
    events(&mut s, &[ToolEvent::down(100.0, 100.0).alt(), ToolEvent::up(100.0, 100.0).alt()]);
    assert!(s.active().map(|d| d.view.zoom).unwrap_or(0.0) < z1, "Alt = verkleinern");
    tool(&mut s, ToolId::Hand);
    let o = s.active().map(|d| d.view.origin).unwrap_or_default();
    events(&mut s, &[ToolEvent::down(10.0, 10.0), ToolEvent::drag(15.0, 12.0), ToolEvent::up(15.0, 12.0)]);
    assert_ne!(s.active().map(|d| d.view.origin).unwrap_or_default(), o);
}

/// UX-A L27: Doppelklick mit dem Zeiger auf eine Bitmap (Folge der Leinwand) öffnet den
/// Bitmapmodus, das Werkzeug wechselt zum Auswahlrahmen; Esc ohne Auswahlrahmen ändert nichts,
/// mit Rahmen beendet es den Bitmapmodus (L13).
#[test]
fn pointer_double_click_bitmap_enters_bitmap_mode() {
    let mut s = doc_session(400.0, 300.0);
    let b = bitmap(&mut s, 100, 100, 60, 60, [0, 120, 200, 255]);
    select(&mut s, &[]);
    tool(&mut s, ToolId::Pointer);
    events(&mut s, &[
        ToolEvent::drag(130.0, 130.0),
        ToolEvent::down(130.0, 130.0),
        ToolEvent::up(130.0, 130.0),
        ToolEvent::drag(130.0, 130.0),
        ToolEvent::down(130.0, 130.0),
        ToolEvent::up(130.0, 130.0),
        ToolEvent::double(130.0, 130.0),
        ToolEvent::drag(130.0, 130.0),
    ]);
    let d = s.active().unwrap();
    assert!(d.bitmap_mode, "Bitmapmodus");
    assert_eq!(d.selection.elements, vec![b]);
    assert_eq!(s.tools.active, ToolId::Marquee, "Werkzeug Auswahlrahmen");
    // Esc ohne Auswahlrahmen: im Original bleibt alles (Werkzeug Auswahlrahmen, Bitmap gewählt; PM-1)
    let n = s.active().unwrap().history.len();
    s.tool_event(&ToolEvent::key(ToolKey::Escape)).expect("Esc");
    let d = s.active().unwrap();
    assert_eq!((s.tools.active, d.selection.elements.clone(), d.history.len()), (ToolId::Marquee, vec![b], n));
    assert!(d.bitmap_mode, "Bitmapmodus bleibt");
    // mit Auswahlrahmen beendet Esc den Bitmapmodus wie nach dem Zauberstab (L13)
    drag(&mut s, (110.0, 110.0), (140.0, 140.0));
    assert!(s.active().unwrap().selection.has_pixels());
    s.tool_event(&ToolEvent::key(ToolKey::Escape)).expect("Esc");
    let d = s.active().unwrap();
    assert!(!d.bitmap_mode && !d.selection.has_pixels());
    assert_eq!((s.tools.active, d.selection.elements.clone()), (ToolId::Pointer, vec![b]));
}

/// PARITAET_A K6–K9: Mauszeiger beim Überfahren wie im Original.
#[test]
fn hover_cursors_like_original() {
    use fp_engine::tools::Cursor;
    let mut s = doc_session(400.0, 300.0);
    let r = rect(&mut s, 50.0, 50.0, 150.0, 120.0);
    select(&mut s, &[]);
    let hover = |s: &mut Session, x: f32, y: f32| s.tool_event(&ToolEvent::drag(x, y)).expect("hover").cursor;
    // Füllwerkzeug: über leerer Fläche ⊘, über einem Objekt der Farbeimer
    tool(&mut s, ToolId::PaintBucket);
    assert_eq!(hover(&mut s, 300.0, 250.0), Cursor::NotAllowed);
    assert_eq!(hover(&mut s, 100.0, 80.0), Cursor::Bucket);
    // Zuschneiden: Zuschneide-Symbol (Exportbereich bleibt beim Fadenkreuz)
    tool(&mut s, ToolId::Crop);
    assert_eq!(hover(&mut s, 300.0, 250.0), Cursor::Crop);
    tool(&mut s, ToolId::ExportArea);
    assert_eq!(hover(&mut s, 300.0, 250.0), Cursor::Crosshair);
    // Stift über leerer Leinwand: Federspitze (Cursor::Pen, ohne Zusatz)
    tool(&mut s, ToolId::Pen);
    assert_eq!(hover(&mut s, 300.0, 250.0), Cursor::Pen);
    // Hand: offen beim Überfahren, Faust nur beim Ziehen, danach wieder offen
    tool(&mut s, ToolId::Hand);
    assert_eq!(hover(&mut s, 300.0, 250.0), Cursor::Hand);
    assert_eq!(s.tool_event(&ToolEvent::down(300.0, 250.0)).expect("down").cursor, Cursor::Grabbing);
    let _ = s.tool_event(&ToolEvent::drag(305.0, 252.0));
    let _ = s.tool_event(&ToolEvent::up(305.0, 252.0));
    assert_eq!(hover(&mut s, 305.0, 252.0), Cursor::Hand);
    let _ = r;
}

/// UX-A L24: Stift Klick, Klick, Doppelklick – genau die Folge der Leinwand (je Klick Drücken,
/// Loslassen, Überfahren; beim zweiten Klick des Doppelklicks danach `DoubleClick`). Original:
/// offener Pfad mit drei Punkten 250,150 → 330,150 → 330,220, ein Verlaufsschritt.
#[test]
fn pen_double_click_keeps_open_path_like_original() {
    let mut s = doc_session(600.0, 400.0);
    tool(&mut s, ToolId::Pen);
    let before = doc(&s);
    let n0 = top_elements(&s).len();
    let h0 = s.active().unwrap().history.len();
    let click = |x: f32, y: f32| vec![ToolEvent::drag(x, y), ToolEvent::down(x, y), ToolEvent::up(x, y), ToolEvent::drag(x, y)];
    let mut seq = click(250.0, 150.0);
    seq.extend(click(330.0, 150.0));
    seq.extend(click(330.0, 220.0));
    seq.extend([ToolEvent::down(330.0, 220.0), ToolEvent::up(330.0, 220.0), ToolEvent::double(330.0, 220.0), ToolEvent::drag(330.0, 220.0)]);
    events(&mut s, &seq);
    assert_eq!(top_elements(&s).len(), n0 + 1, "Pfad bleibt");
    assert_eq!(s.active().unwrap().history.len(), h0 + 1, "ein Verlaufsschritt");
    let c = &path_of(&s, last(&s)).contours[0];
    assert!(!c.closed);
    let pts: Vec<(f32, f32)> = c.nodes.iter().map(|n| (n.anchor.x, n.anchor.y)).collect();
    assert_eq!(pts, vec![(250.0, 150.0), (330.0, 150.0), (330.0, 220.0)]);
    assert_eq!(s.tools.active, ToolId::Pen, "Werkzeug bleibt");
    undo_redo(&mut s, &before);
    // danach beginnt ein Klick einen neuen Pfad (nicht das Fortsetzen des eben gezeichneten)
    let mut seq = click(100.0, 300.0);
    seq.extend(click(150.0, 300.0));
    seq.extend([ToolEvent::down(150.0, 300.0), ToolEvent::up(150.0, 300.0), ToolEvent::double(150.0, 300.0)]);
    events(&mut s, &seq);
    assert_eq!(top_elements(&s).len(), n0 + 2);
    assert_eq!(path_of(&s, last(&s)).contours[0].nodes.len(), 2);
}

/// UX-A L15/L16, PM-1 (14 Messfälle im Original, `uxp_pm_l16.ps1`): Start- und Endpunkt rasten
/// getrennt in x und y an Kanten und Mitten anderer Objekte ein, bei Abstand ≤ 5 Bildschirmpixel
/// (5 rastet, 6 nicht); mit Alt rastet der Mittelpunkt wie sonst der Start; mit Umschalt rastet
/// nichts (auch nicht mit Alt+Umschalt). Testdokument wie R1/R2/E1/T1 der Messung, viele kleine
/// Bewegungen wie mit der Maus.
#[test]
fn rectangle_start_snap_shift_alt_like_original() {
    fn draw(s: &mut Session, from: (f32, f32), to: (f32, f32), mods: &str) -> Rect {
        let (shift, alt) = (mods.contains('s'), mods.contains('a'));
        let m = |e: ToolEvent| {
            let e = if shift { e.shift() } else { e };
            if alt { e.alt() } else { e }
        };
        let mut seq = vec![m(ToolEvent::drag(from.0, from.1)), m(ToolEvent::down(from.0, from.1))];
        for i in 1..=20 {
            let f = i as f32 / 20.0;
            seq.push(m(ToolEvent::drag((from.0 + (to.0 - from.0) * f).round(), (from.1 + (to.1 - from.1) * f).round())));
        }
        seq.push(m(ToolEvent::up(to.0, to.1)));
        events(s, &seq);
        let r = bounds(s, last(s));
        s.undo().expect("undo");
        r
    }
    let mut s = doc_session(600.0, 400.0);
    rect(&mut s, 50.0, 50.0, 150.0, 120.0); // R1
    rect(&mut s, 450.0, 150.0, 520.0, 200.0); // R2: Oberkante 150, Mitte 175
    ellipse(&mut s, 200.0, 50.0, 300.0, 120.0); // E1: Mitte x 250, rechts 300
    rect(&mut s, 50.0, 170.0, 110.0, 230.0); // T1: Oberkante 170
    select(&mut s, &[]);
    tool(&mut s, ToolId::Rectangle);
    let r = |l, t, rr, b| Rect::new(l, t, rr, b);
    // (Name, Start, Ende, Tasten s/a, erwartetes Rechteck)
    type Case<'a> = (&'a str, (f32, f32), (f32, f32), &'a str, Rect);
    let cases: [Case; 17] = [
        // Messwerte des Originals (Befund PM-1)
        ("A1 ohne", (240.0, 145.0), (300.0, 175.0), "", r(240.0, 150.0, 300.0, 175.0)),
        ("A2 Umschalt", (240.0, 145.0), (300.0, 175.0), "s", r(240.0, 145.0, 300.0, 205.0)),
        ("A3 Alt", (240.0, 145.0), (300.0, 175.0), "a", r(180.0, 125.0, 300.0, 175.0)),
        ("A4 Alt+Umschalt", (240.0, 145.0), (300.0, 175.0), "as", r(180.0, 85.0, 300.0, 205.0)),
        ("B1 Alt d=4", (240.0, 146.0), (300.0, 176.0), "a", r(180.0, 125.0, 300.0, 175.0)),
        ("B2 Alt d=6", (240.0, 144.0), (300.0, 174.0), "a", r(180.0, 113.0, 300.0, 175.0)),
        ("B3 Alt d=7 (Gleichstand 175/170 nicht)", (240.0, 143.0), (300.0, 173.0), "a", r(180.0, 111.0, 300.0, 175.0)),
        ("B4 Alt d=3 unten", (240.0, 153.0), (300.0, 183.0), "a", r(180.0, 117.0, 300.0, 183.0)),
        ("C1 ohne d=3", (240.0, 147.0), (300.0, 190.0), "", r(240.0, 150.0, 300.0, 190.0)),
        ("C2 Umschalt d=3", (240.0, 147.0), (300.0, 177.0), "s", r(240.0, 147.0, 300.0, 207.0)),
        ("C3 ohne d=6", (240.0, 144.0), (300.0, 190.0), "", r(240.0, 144.0, 300.0, 190.0)),
        ("D1 ohne x d=4", (246.0, 135.0), (300.0, 185.0), "", r(250.0, 135.0, 300.0, 185.0)),
        ("D2 Alt x d=4", (246.0, 135.0), (300.0, 185.0), "a", r(200.0, 85.0, 300.0, 185.0)),
        ("E1 Ende nahe E1-Rand 300", (240.0, 135.0), (298.0, 185.0), "", r(240.0, 135.0, 300.0, 185.0)),
        ("Umschalt, Ende 298,160", (240.0, 135.0), (298.0, 160.0), "s", r(240.0, 135.0, 298.0, 193.0)),
        ("Alt+Umschalt, Start 240,147", (240.0, 147.0), (300.0, 175.0), "as", r(180.0, 87.0, 300.0, 207.0)),
        ("Abstand 5 am Ende rastet", (240.0, 135.0), (295.0, 185.0), "", r(240.0, 135.0, 300.0, 185.0)),
    ];
    for (name, a, b, mods, want) in cases {
        assert_eq!(draw(&mut s, a, b, mods), want, "{name}");
    }
    // bei 200 % sind 4 Dokumentpixel 8 Bildschirmpixel: kein Einrasten, 2 Dokumentpixel = 4: Einrasten
    ex(&mut s, "view.setZoom", json!({"zoom": 2.0}));
    assert_eq!(draw(&mut s, (246.0, 136.0), (290.0, 160.0), ""), Rect::new(246.0, 136.0, 290.0, 160.0));
    assert_eq!(draw(&mut s, (248.0, 136.0), (290.0, 160.0), ""), Rect::new(250.0, 136.0, 290.0, 160.0));
}

// ------------------------------------------------------------------ Fang und Vollständigkeit

#[test]
fn snapping_to_guides_while_moving() {
    let mut s = doc_session(300.0, 200.0);
    ex(&mut s, "guide.add", json!({"orientation": "vertical", "position": 100}));
    ex(&mut s, "view.guides.snapToGuides", json!({}));
    let snapped = s.active().is_some_and(|d| d.view.snap_to_guides);
    let a = rect(&mut s, 10.0, 10.0, 50.0, 50.0);
    tool(&mut s, ToolId::Pointer);
    events(&mut s, &[ToolEvent::down(30.0, 30.0), ToolEvent::drag(60.0, 30.0), ToolEvent::drag(78.0, 30.0), ToolEvent::up(78.0, 30.0)]);
    let b = bounds(&s, a);
    if snapped {
        assert!(approx(b.left, 100.0) || approx(b.right, 100.0), "Kante rastet an der Hilfslinie ein: {b:?}");
    } else {
        assert!(approx(b.left, 58.0), "{b:?}");
    }
}

// ------------------------------------------------------------------ Logikfehler

/// Esc mitten im Ziehen: das Dokument bleibt exakt wie vorher (alle ziehenden Werkzeuge).
#[test]
fn escape_mid_drag_leaves_document_unchanged() {
    for t in [
        ToolId::Pointer,
        ToolId::Subselection,
        // Rechteck: Esc wird im Original ignoriert (L19), eigener Test unten
        ToolId::Ellipse,
        ToolId::Polygon,
        ToolId::Line,
        ToolId::VectorPath,
        ToolId::Scale,
        ToolId::Skew,
        ToolId::Distort,
        ToolId::NineSliceScale,
        ToolId::Freeform,
        ToolId::Reshape,
        ToolId::PathScrubAdd,
        ToolId::Knife,
        ToolId::Crop,
        ToolId::HotspotRect,
        ToolId::Slice,
        ToolId::Blur,
        ToolId::Text,
    ] {
        let mut s = doc_session(300.0, 200.0);
        let a = rect(&mut s, 50.0, 50.0, 150.0, 120.0);
        select(&mut s, &[a]);
        tool(&mut s, t);
        let before = doc(&s);
        let sel0 = sel(&s);
        events(&mut s, &[ToolEvent::down(150.0, 120.0), ToolEvent::drag(160.0, 125.0), ToolEvent::drag(180.0, 140.0), ToolEvent::key(ToolKey::Escape)]);
        // Loslassen nach Esc darf nichts mehr übernehmen
        let _ = s.tool_event(&ToolEvent::up(180.0, 140.0));
        if t == ToolId::Text {
            // Text: Esc übernimmt den (leeren) Text = nichts
            assert_eq!(top_elements(&s).len(), 1);
            continue;
        }
        assert_eq!(doc(&s), before, "{t:?}: Esc mitten im Ziehen hat das Dokument geändert");
        assert_eq!(sel(&s), sel0, "{t:?}: Auswahl geändert");
    }
}

/// Werkzeugwechsel mitten im Ziehen bricht ab (kein halbes Objekt), Text wird übernommen.
#[test]
fn tool_switch_mid_drag() {
    for t in [ToolId::Rectangle, ToolId::Pointer, ToolId::Scale, ToolId::NineSliceScale, ToolId::Marquee, ToolId::Freeform] {
        let mut s = doc_session(300.0, 200.0);
        let a = rect(&mut s, 50.0, 50.0, 150.0, 120.0);
        select(&mut s, &[a]);
        tool(&mut s, t);
        let before = doc(&s);
        events(&mut s, &[ToolEvent::down(150.0, 120.0), ToolEvent::drag(170.0, 140.0)]);
        tool(&mut s, ToolId::Ellipse);
        let _ = s.tool_event(&ToolEvent::up(170.0, 140.0));
        assert_eq!(doc(&s), before, "{t:?}");
    }
}

/// Wiederholtes Ziehen ändert, ohne etwas zu verdoppeln; Rückgängig-Folgen führen exakt zurück.
#[test]
fn repeated_drags_no_duplicates_and_undo_chain() {
    let mut s = doc_session(400.0, 300.0);
    let a = rect(&mut s, 50.0, 50.0, 150.0, 120.0);
    let start = doc(&s);
    tool(&mut s, ToolId::Pointer);
    // Eckgriff mehrmals ziehen (skaliert, PARITAET_A L26): Kontur bleibt eine, Rundung 0
    let mut from = (150.0, 120.0);
    for (i, to) in [(160.0, 130.0), (140.0, 110.0), (150.0, 120.0)].into_iter().enumerate() {
        events(&mut s, &[ToolEvent::down(from.0, from.1), ToolEvent::drag((from.0 + to.0) / 2.0, (from.1 + to.1) / 2.0), ToolEvent::drag(to.0, to.1), ToolEvent::up(to.0, to.1)]);
        from = to;
        let p = path_of(&s, a);
        assert_eq!(p.contours.len(), 1, "Durchgang {i}");
        assert_eq!(p.primitive.as_ref().map(|r| r.roundness), Some(0.0), "Durchgang {i}");
        assert!(rect_approx(bounds(&s, a), Rect::new(50.0, 50.0, to.0, to.1)), "Durchgang {i}: {:?}", bounds(&s, a));
    }
    assert_eq!(top_elements(&s).len(), 1);
    // Skalieren und 9-Slice mehrmals
    tool(&mut s, ToolId::Scale);
    // Ecke skaliert proportional (L30): 100 × 70 → 130 × 91 → 110 × 77
    drag(&mut s, (150.0, 120.0), (180.0, 141.0));
    s.finish_tool().expect("übernehmen");
    drag(&mut s, (180.0, 141.0), (160.0, 127.0));
    s.finish_tool().expect("übernehmen");
    assert!(rect_approx(bounds(&s, a), Rect::new(50.0, 50.0, 160.0, 127.0)), "{:?}", bounds(&s, a));
    tool(&mut s, ToolId::NineSliceScale);
    drag(&mut s, (160.0, 127.0), (200.0, 150.0));
    s.finish_tool().expect("übernehmen");
    drag(&mut s, (200.0, 150.0), (220.0, 160.0));
    s.finish_tool().expect("übernehmen");
    assert_eq!(top_elements(&s).len(), 1);
    assert_eq!(path_of(&s, a).contours.len(), 1);
    assert!(rect_approx(bounds(&s, a), Rect::new(50.0, 50.0, 220.0, 160.0)), "{:?}", bounds(&s, a));
    // Pfadausbesserung mehrmals: Druck bleibt in 0..1, Profile werden nicht vervielfacht
    tool(&mut s, ToolId::PathScrubSubtract);
    for _ in 0..5 {
        drag(&mut s, (60.0, 50.0), (200.0, 50.0));
    }
    for n in &path_of(&s, a).contours[0].nodes {
        if let Some(p) = &n.pressure {
            assert_eq!(p.samples.len(), 64);
            assert!(p.samples.iter().all(|x| x.pressure.is_some_and(|v| (0.0..=1.0).contains(&v))));
        }
    }
    // alles rückgängig = Ausgangsdokument, alles wiederholen = Endstand
    let end = doc(&s);
    while s.active().is_some_and(|d| d.history.can_undo()) && doc(&s) != start {
        s.undo().expect("undo");
    }
    assert_eq!(doc(&s), start);
    while s.active().is_some_and(|d| d.history.can_redo()) {
        s.redo().expect("redo");
    }
    assert_eq!(doc(&s), end);
}

/// Gesperrte und ausgeblendete Objekte werden nicht getroffen; Gruppen und Mehrfachauswahl.
#[test]
fn locked_hidden_groups_and_multi_selection() {
    let mut s = doc_session(400.0, 300.0);
    let a = rect(&mut s, 10.0, 10.0, 60.0, 60.0);
    let b = rect(&mut s, 100.0, 10.0, 150.0, 60.0);
    let c = ellipse(&mut s, 200.0, 10.0, 250.0, 60.0);
    select(&mut s, &[a]);
    ex(&mut s, "element.setLocked", json!({"locked": true}));
    select(&mut s, &[b]);
    ex(&mut s, "element.setVisible", json!({"visible": false}));
    ex(&mut s, "select.none", json!({}));
    tool(&mut s, ToolId::Pointer);
    click(&mut s, 30.0, 30.0);
    assert!(sel(&s).is_empty(), "gesperrt");
    click(&mut s, 120.0, 30.0);
    assert!(sel(&s).is_empty(), "ausgeblendet");
    // Gummiband wählt nur das sichtbare, ungesperrte Objekt
    drag(&mut s, (0.0, 0.0), (300.0, 100.0));
    assert_eq!(sel(&s), vec![c]);
    // Gruppe: Skalieren und 9-Slice wirken auf die Kinder, Gruppe bleibt eine
    let d = rect(&mut s, 10.0, 150.0, 60.0, 200.0);
    let e = ellipse(&mut s, 80.0, 150.0, 130.0, 200.0);
    select(&mut s, &[d, e]);
    ex(&mut s, "modify.group", json!({}));
    let g = sel(&s)[0];
    tool(&mut s, ToolId::NineSliceScale);
    let before = doc(&s);
    drag(&mut s, (130.0, 200.0), (230.0, 220.0));
    s.finish_tool().expect("übernehmen");
    assert!(rect_approx(bounds(&s, g), Rect::new(10.0, 150.0, 230.0, 220.0)), "{:?}", bounds(&s, g));
    assert!(matches!(&el(&s, g).kind, ElementKind::Group(gr) if gr.children.len() == 2));
    undo_redo(&mut s, &before);
    // Mehrfachauswahl verschieben: beide bewegen sich, nichts wird kopiert
    let f = rect(&mut s, 300.0, 200.0, 320.0, 220.0);
    select(&mut s, &[c, f]);
    tool(&mut s, ToolId::Pointer);
    let n = top_elements(&s).len();
    // um 10/18 (bei 20 rastete die Oberkante mit Abstand 5 an der Mitte von a ein, wie im Original)
    drag(&mut s, (310.0, 210.0), (320.0, 228.0));
    assert_eq!(top_elements(&s).len(), n);
    assert!(rect_approx(bounds(&s, f), Rect::new(310.0, 218.0, 330.0, 238.0)), "{:?} {:?}", bounds(&s, f), bounds(&s, c));
    assert!(approx(bounds(&s, c).left, 210.0));
    // Auswahl wechselt mitten im Ziehen (z. B. über ein Bedienfeld): kein Fehler, nichts verschoben
    events(&mut s, &[ToolEvent::down(320.0, 230.0), ToolEvent::drag(330.0, 240.0)]);
    ex(&mut s, "select.none", json!({}));
    let before = doc(&s);
    let _ = s.tool_event(&ToolEvent::up(340.0, 250.0));
    assert_eq!(doc(&s), before);
}

/// Symbole: Doppelklick meldet nur, Transformieren wirkt auf die Instanz.
#[test]
fn symbols_with_tools() {
    let mut s = doc_session(300.0, 200.0);
    let a = rect(&mut s, 10.0, 10.0, 60.0, 60.0);
    select(&mut s, &[a]);
    ex(&mut s, "modify.symbol.convertToSymbol", json!({"name": "S"}));
    let i = sel(&s)[0];
    tool(&mut s, ToolId::Scale);
    let before = doc(&s);
    let b = bounds(&s, i);
    drag(&mut s, (b.right, b.bottom), (b.right + 40.0, b.bottom + 40.0));
    s.finish_tool().expect("übernehmen");
    assert!(bounds(&s, i).width() > b.width() + 30.0);
    undo_redo(&mut s, &before);
    tool(&mut s, ToolId::Pointer);
    let before = doc(&s);
    let r = s.tool_event(&ToolEvent::double(30.0, 30.0)).expect("Doppelklick");
    assert!(r.status.is_some());
    assert_eq!(doc(&s), before);
}

#[test]
fn pseudo_tool_keys_and_legacy_guids() {
    use fp_engine::tools::{cycle_tool_for_key, pseudo_command_for_key};
    use fp_engine::Shortcut;
    let k = |c: u8| Shortcut::key(c as u16);
    for set in ["Firepixel", "Photoshop", "FreeHand", "Web Standard"] {
        let Some(s) = fp_engine::shortcuts::builtin_set(set) else { continue };
        assert_eq!(pseudo_command_for_key(s, &k(b'D')), Some("color.defaults"), "{set}");
        assert_eq!(pseudo_command_for_key(s, &k(b'X')), Some("color.swap"), "{set}");
    }
    let ps = fp_engine::shortcuts::builtin_set("Photoshop").expect("Photoshop");
    assert_eq!(pseudo_command_for_key(ps, &k(b'F')), Some("view.toggleScreenMode"));
    assert_eq!(pseudo_command_for_key(ps, &k(b'2')), Some("view.toggleSlices"));
    assert_eq!(cycle_tool_for_key(ps, &k(b'U'), ToolId::Pointer), Some(ToolId::Rectangle), "U = Rechteck-Flyout");
    // die Befehle wirken und sind rückgängig machbar bzw. Sitzungszustand
    let mut s = doc_session(100.0, 100.0);
    ex(&mut s, "view.toggleScreenMode", json!({}));
    assert_eq!(s.ui.screen_mode, 1);
    ex(&mut s, "view.exitFullScreen", json!({}));
    assert_eq!(s.ui.screen_mode, 0);
    let before = doc(&s);
    ex(&mut s, "view.toggleSlices", json!({}));
    assert_eq!(doc(&s), before, "Segmente ein/aus ändert das Dokument nicht");
}

#[test]
fn context_menu_path_combine_commands() {
    let mut s = doc_session(300.0, 200.0);
    let a = rect(&mut s, 10.0, 10.0, 60.0, 60.0);
    let b = ellipse(&mut s, 40.0, 40.0, 100.0, 100.0);
    select(&mut s, &[a, b]);
    let before = doc(&s);
    for (name, id) in [("path.combine.union", 49111u32), ("path.combine.intersect", 49112), ("path.combine.punch", 49113), ("path.combine.crop", 49114), ("path.combine.normal", 49115)] {
        assert_eq!(s.registry().by_fw_id(id).map(|c| c.info().name.clone()).as_deref(), Some(name));
    }
    ex(&mut s, "path.combine.punch", json!({}));
    assert_eq!(top_elements(&s).len(), 1, "zusammengesetzte Form");
    // mehrfach umschalten ohne Duplikate
    ex(&mut s, "path.combine.union", json!({}));
    ex(&mut s, "path.combine.intersect", json!({}));
    assert_eq!(top_elements(&s).len(), 1);
    while doc(&s) != before && s.active().is_some_and(|d| d.history.can_undo()) {
        s.undo().expect("undo");
    }
    assert_eq!(doc(&s), before);
    assert_eq!(s.registry().by_fw_id(35589).map(|c| c.info().name.clone()).as_deref(), Some("view.exitFullScreen"));
}

/// Elemente auf über Seiten geteilten Ebenen: mit dem Zeiger auswählen, verschieben, Rundung ziehen,
/// skalieren, Punkte bearbeiten – auf beiden Seiten sichtbar, Rückgängig exakt.
#[test]
fn shared_layer_elements_are_editable() {
    let mut s = doc_session(300.0, 200.0);
    let a = rect(&mut s, 50.0, 50.0, 150.0, 120.0);
    ex(&mut s, "page.add", json!({}));
    ex(&mut s, "page.setCurrent", json!({"index": 0}));
    ex(&mut s, "panel.layers.shareToAllPages", json!({}));
    ex(&mut s, "select.none", json!({}));
    tool(&mut s, ToolId::Pointer);
    click(&mut s, 100.0, 80.0);
    assert_eq!(sel(&s), vec![a], "Element der geteilten Ebene getroffen");
    let before = doc(&s);
    // die Mitte (95) rastet mit Abstand 5 an der Seitenmitte 100 ein (Fangabstand ≤ 5 wie im Original)
    drag(&mut s, (100.0, 80.0), (120.0, 90.0));
    assert!(rect_approx(bounds(&s, a), Rect::new(70.0, 65.0, 170.0, 135.0)), "{:?}", bounds(&s, a));
    undo_redo(&mut s, &before);
    // Eckgriff skaliert (auch auf der geteilten Ebene), die Rundung bleibt
    let before = doc(&s);
    events(&mut s, &[ToolEvent::down(70.0, 65.0), ToolEvent::drag(75.0, 66.0), ToolEvent::drag(80.0, 67.0), ToolEvent::up(80.0, 67.0)]);
    assert!(rect_approx(bounds(&s, a), Rect::new(80.0, 67.0, 170.0, 135.0)), "{:?}", bounds(&s, a));
    assert!(path_of(&s, a).primitive.as_ref().is_some_and(|p| p.roundness == 0.0));
    undo_redo(&mut s, &before);
    s.undo().expect("undo");
    // Skalieren und Teilauswahl-Punkt
    tool(&mut s, ToolId::Scale);
    let before = doc(&s);
    // Ecke skaliert proportional (L30): 100 × 70 → 130 × 91
    drag(&mut s, (170.0, 135.0), (200.0, 156.0));
    s.finish_tool().expect("übernehmen");
    assert!(rect_approx(bounds(&s, a), Rect::new(70.0, 65.0, 200.0, 156.0)), "{:?}", bounds(&s, a));
    undo_redo(&mut s, &before);
    // auf der zweiten Seite ist dasselbe Element zu sehen und greifbar
    ex(&mut s, "page.setCurrent", json!({"index": 1}));
    ex(&mut s, "select.none", json!({}));
    tool(&mut s, ToolId::Pointer);
    click(&mut s, 120.0, 100.0);
    assert_eq!(sel(&s), vec![a], "auf Seite 2 greifbar");
}

#[test]
fn tool_default_brush_is_used_for_new_shapes() {
    let mut s = doc_session(300.0, 200.0);
    let mut b = fp_model::Brush::basic(6);
    b.softness = 300;
    s.tools.options.new_brush = Some(serde_json::to_value(&b).expect("Pinsel"));
    for t in [ToolId::Rectangle, ToolId::Line, ToolId::VectorPath] {
        tool(&mut s, t);
        let before = doc(&s);
        drag(&mut s, (10.0, 10.0), (90.0, 60.0));
        let p = path_of(&s, last(&s));
        assert_eq!(p.attrs.brush.as_ref().map(|x| (x.diameter, x.softness)), Some((6, 300)), "{t:?}");
        undo_redo(&mut s, &before);
    }
    // Stift
    tool(&mut s, ToolId::Pen);
    events(&mut s, &[ToolEvent::down(10.0, 150.0), ToolEvent::up(10.0, 150.0), ToolEvent::down(90.0, 150.0), ToolEvent::up(90.0, 150.0), ToolEvent::key(ToolKey::Enter)]);
    assert_eq!(path_of(&s, last(&s)).attrs.brush.as_ref().map(|x| x.softness), Some(300));
}

#[test]
fn double_click_text_with_pointer_opens_editor_at_click() {
    let mut s = doc_session(400.0, 400.0);
    let v = ex(&mut s, "draw.text", json!({"text": "Firepixel Test", "anchor": [40, 330], "size": 24}));
    let t = ElementId(v["id"].as_u64().expect("id"));
    ex(&mut s, "select.none", json!({}));
    tool(&mut s, ToolId::Pointer);
    let b = bounds(&s, t);
    let (x, y) = (b.left + 30.0, (b.top + b.bottom) / 2.0);
    events(&mut s, &[ToolEvent::down(x, y), ToolEvent::up(x, y), ToolEvent::drag(x, y), ToolEvent::down(x, y), ToolEvent::up(x, y), ToolEvent::drag(x, y), ToolEvent::double(x, y)]);
    assert_eq!(s.tools.active, ToolId::Text);
    assert!(s.tool_is_editing_text());
    // die Einfügemarke an der Klickstelle setzt die Oberfläche über das Textlayout (TextCaret)
    s.tool_event(&ToolEvent { input: fp_engine::tools::ToolInput::TextCaret { index: 3, extend: false }, mods: Default::default(), pressure: 1.0, time_ms: 0 }).expect("Caret");
    let caret = s.tool_feedback().iter().find_map(|o| match o {
        Overlay::TextEdit(v) => Some(v.caret),
        _ => None,
    });
    assert_eq!(caret, Some(3));
}

/// UX-A Z22: Leertaste während des Aufziehens verschiebt die Form (Rechteck, Linie, Auswahlrahmen,
/// Zoomrahmen) – Sonde: 200,200 → +60,+40, dann mit Leertaste +50 → 250,200–310,240.
#[test]
fn space_moves_shape_while_dragging() {
    let mut s = doc_session(500.0, 400.0);
    for t in [ToolId::Rectangle, ToolId::Ellipse] {
        tool(&mut s, t);
        events(&mut s, &[ToolEvent::down(200.0, 200.0), ToolEvent::drag(230.0, 220.0), ToolEvent::drag(260.0, 240.0), ToolEvent::drag(285.0, 240.0).space(), ToolEvent::drag(310.0, 240.0).space(), ToolEvent::up(310.0, 240.0)]);
        assert!(rect_approx(bounds(&s, last(&s)), Rect::new(250.0, 200.0, 310.0, 240.0)), "{t:?} {:?}", bounds(&s, last(&s)));
    }
    tool(&mut s, ToolId::Marquee);
    events(&mut s, &[ToolEvent::down(20.0, 20.0), ToolEvent::drag(40.0, 30.0), ToolEvent::drag(60.0, 40.0), ToolEvent::drag(80.0, 40.0).space(), ToolEvent::up(80.0, 40.0)]);
    assert_eq!(pixel_bounds(&s), IRect::new(40, 20, 80, 40), "Auswahlrahmen verschoben");
}

/// UX-A P02: Klick ins Leere hebt die Auswahl ohne Verlaufsschritt auf; Klicks auf Objekte und
/// Gummiband sind ebenfalls keine Schritte (nur Strg+D ist einer).
#[test]
fn pointer_selection_clicks_are_not_history_steps() {
    let mut s = doc_session(300.0, 200.0);
    let a = rect(&mut s, 10.0, 10.0, 50.0, 50.0);
    ex(&mut s, "select.none", json!({}));
    let steps = |s: &Session| s.active().map(|d| d.history.undo_count()).unwrap_or(0);
    let n = steps(&s);
    tool(&mut s, ToolId::Pointer);
    click(&mut s, 30.0, 30.0);
    assert_eq!(sel(&s), vec![a]);
    events(&mut s, &[ToolEvent::down(30.0, 30.0).shift(), ToolEvent::up(30.0, 30.0).shift()]);
    click(&mut s, 30.0, 30.0);
    click(&mut s, 200.0, 150.0);
    assert!(sel(&s).is_empty(), "Klick ins Leere hebt auf");
    drag(&mut s, (0.0, 0.0), (60.0, 60.0));
    assert_eq!(sel(&s), vec![a]);
    assert_eq!(steps(&s), n, "keine Verlaufsschritte durch Auswählen mit der Maus");
}

/// UX-A P13/W06: Umschalt-Einrasten projiziert auf die Achse – ganzzahlige Lagen.
#[test]
fn shift_constraint_projects_onto_axis() {
    let mut s = doc_session(400.0, 300.0);
    let a = rect(&mut s, 51.0, 51.0, 91.0, 91.0);
    tool(&mut s, ToolId::Pointer);
    events(&mut s, &[ToolEvent::down(70.0, 70.0), ToolEvent::drag(90.0, 75.0).shift(), ToolEvent::drag(110.0, 80.0).shift(), ToolEvent::up(110.0, 80.0).shift()]);
    let b = bounds(&s, a);
    assert_eq!((b.left, b.top), (91.0, 51.0), "+40 waagerecht, keine Bruchteile");
    tool(&mut s, ToolId::Line);
    events(&mut s, &[ToolEvent::down(200.0, 200.0), ToolEvent::drag(230.0, 207.0).shift(), ToolEvent::drag(256.0, 215.0).shift(), ToolEvent::up(256.0, 215.0).shift()]);
    let l = bounds(&s, last(&s));
    assert_eq!((l.right, l.height()), (256.0, 0.0), "Linie endet bei x = 256");
    // 45°: d = (dx + dy) / 2
    events(&mut s, &[ToolEvent::down(100.0, 100.0), ToolEvent::drag(120.0, 118.0).shift(), ToolEvent::drag(140.0, 136.0).shift(), ToolEvent::up(140.0, 136.0).shift()]);
    let l = bounds(&s, last(&s));
    assert_eq!((l.right, l.bottom), (138.0, 138.0));
}

#[test]
fn every_tool_has_a_machine() {
    assert_eq!(TOOLS.len(), 52);
    let missing: Vec<&str> = TOOLS.iter().filter(|t| !t.implemented).map(|t| t.name).collect();
    assert!(missing.is_empty(), "ohne Automat: {missing:?}");
    let mut s = doc_session(100.0, 100.0);
    for t in TOOLS {
        tool(&mut s, t.id);
        assert_eq!(s.tools.active, t.id);
        // Überfahren ohne Taste darf nie scheitern
        s.tool_event(&ToolEvent::drag(50.0, 50.0)).unwrap_or_else(|e| panic!("{}: {e}", t.name));
        let _ = s.tool_feedback();
        s.tool_event(&ToolEvent::key(ToolKey::Escape)).unwrap_or_else(|e| panic!("{}: {e}", t.name));
    }
    let _ = Point::new(0.0, 0.0);
}
