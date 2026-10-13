//! PAR-X-INSP-1: Inspektor Pfad/Rechteck und Füllung wie das Original (docs\parity\inspector-path-rect.md,
//! inspector-fill.md): Verlaufsarten, Vorgabeverlauf und -griffe, Muster, Kante, Rundung nach
//! B/H-Änderung, Strich mitskalieren, `pixelRect` mit Strich, Verlaufsschritt „Füllen“.
//! Logikfälle nach §4b (Wiederholung, Rückgängig, Auswahlwechsel).

use fp_engine::commands::panel::{self, CONTOUR_SHAPE, GRADIENT_SHAPES};
use fp_engine::Session;
use fp_model::{Element, ElementId, ElementKind, Fill};
use serde_json::json;
use std::sync::Arc;

fn session() -> Session {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 400, "height": 300, "background": "#ffffff"})).expect("Dokument");
    s
}

fn rect(s: &mut Session, r: [i32; 4]) -> u64 {
    s.execute("draw.rectangle", json!({"rect": r, "fillColor": "#808080", "strokeWidth": 0})).expect("Rechteck").value["id"].as_u64().expect("id")
}

fn el(s: &Session, id: u64) -> Arc<Element> {
    let d = s.active().expect("Dokument");
    fp_engine::docops::element_on_page(&d.doc, d.page, ElementId(id)).expect("Element")
}

fn fill(s: &Session, id: u64) -> Fill {
    fp_engine::docops::path_attrs_of(&el(s, id)).and_then(|a| a.fill.clone()).expect("Füllung")
}

fn handles(s: &Session, id: u64) -> fp_model::FillHandles {
    panel::handles_of(&el(s, id)).expect("Griffe")
}

#[test]
fn gradient_shapes_like_original() {
    let names: Vec<&str> = GRADIENT_SHAPES.iter().map(|g| g.0).collect();
    assert_eq!(names, ["Linear", "Radial", "Rechteck", "Kegel", "Kontur", "Glanz", "Supernova", "Falten", "Ellipse", "Streifen", "Kleine Wellen", "Wellen"]);
    let cats: Vec<&str> = GRADIENT_SHAPES.iter().map(|g| g.2).collect();
    assert_eq!(cats, ["fc_Linear", "fc_Circular", "fc_Rectangular", "fc_Conical", "fc_ContourGrad", "fc_Satin", "fc_Pinch", "fc_Folds", "fc_Elliptical", "fc_Bars", "fc_Ripple", "fc_Waves"]);
}

/// Umschalten auf Verlauf: Weiß → Füllfarbe, senkrecht nach unten (270°), Griffe über das Objekt –
/// auch wenn die Griffe vorher entartet waren (Objekte aus Dateien, PAR 18/03).
#[test]
fn gradient_default_and_handles() {
    let mut s = session();
    let id = rect(&mut s, [50, 50, 150, 120]);
    // entartete Griffe wie bei R1 aus der Datei
    {
        let d = s.doc_mut().expect("Dokument");
        let doc = Arc::make_mut(&mut d.doc);
        fp_engine::docops::edit_by_id(doc, 0, ElementId(id), |e| {
            if let ElementKind::Path(p) = &mut e.kind {
                p.fill_handles = fp_model::FillHandles { valid: true, start: fp_geom::Point::new(0.0, 0.0), end: fp_geom::Point::new(0.0, 0.0), second: fp_geom::Point::new(0.0, 0.0) };
            }
        });
    }
    s.execute("pi.fill", json!({"category": "gradient"})).expect("Verlauf");
    let f = fill(&s, id);
    let g = f.gradient.as_ref().expect("Verlauf");
    let cs: Vec<u32> = g.color_stops().iter().map(|c| c.color.0).collect();
    assert_eq!(cs, [0xffff_ffff, 0xff80_8080], "Weiß → Füllfarbe");
    let h = handles(&s, id);
    assert_eq!((h.start.x, h.start.y, h.end.x, h.end.y), (100.0, 50.0, 100.0, 120.0), "von oben nach unten");
    assert!((panel::handles_angle(&h) - 270.0).abs() < 1e-6);
    // Art wechseln: unveränderte Vorgabegriffe werden die der neuen Art (PAR 18/03b, Kontur von der
    // Mitte aus), Kontur wird als FSHi 3 + FSXi 17 gespeichert
    s.execute("pi.fill", json!({"shape": CONTOUR_SHAPE})).expect("Kontur");
    let f = fill(&s, id);
    assert!(f.is_contour_gradient() && f.shape == 3 && f.category == "fc_ContourGrad");
    assert_eq!(panel::shape_id(&f), CONTOUR_SHAPE);
    let hc = handles(&s, id);
    assert_eq!((hc.start.x, hc.start.y, hc.end.x, hc.end.y), (100.0, 85.0, 100.0, 120.0), "Kontur: Mitte → halbe Höhe");
    // Winkel 0° dreht um den Startpunkt; 50× gesetzt = einmal (§4b)
    for _ in 0..50 {
        s.execute("pi.fill", json!({"angle": 0})).expect("Winkel");
    }
    let h0 = handles(&s, id);
    assert!((panel::handles_angle(&h0) - 0.0).abs() < 1e-3, "{h0:?}");
    assert!((h0.end.x - 135.0).abs() < 1e-3 && (h0.end.y - 85.0).abs() < 1e-3, "{h0:?}");
    s.execute("edit.undo", json!({})).expect("undo");
    // 50 gleiche Eingaben = ein Schritt (Wiederholung ohne Änderung legt keinen an, FIX-F)
    let hu = handles(&s, id);
    assert_eq!((hu.start.x, hu.start.y, hu.end.x, hu.end.y), (100.0, 85.0, 100.0, 120.0), "Rückgängig: wieder Kontur-Vorgabe");
}

/// Muster: Umschalten setzt das Vorgabemuster „Cotton“ mit gültigen Griffen; nochmal „Muster“ behält es.
#[test]
fn pattern_category_default() {
    if fp_engine::resources::load_pattern(panel::DEFAULT_PATTERN).is_none() {
        return;
    }
    let mut s = session();
    let id = rect(&mut s, [50, 50, 150, 120]);
    s.execute("pi.fill", json!({"category": "pattern"})).expect("Muster");
    let f = fill(&s, id);
    assert_eq!(f.shape, 15);
    assert_eq!(f.pattern.as_ref().map(|p| p.name.as_str()), Some("Cotton"));
    let h = handles(&s, id);
    assert!(h.valid && (h.end.x - h.start.x).abs() > 1.0, "{h:?}");
    let other = fp_engine::resources::files_in(&fp_engine::resources::dirs(fp_engine::resources::PATTERNS)).into_iter().map(|(n, _)| n).find(|n| n != "Cotton");
    if let Some(n) = other {
        s.execute("pi.fill", json!({"pattern": n})).expect("anderes Muster");
        s.execute("pi.fill", json!({"category": "pattern"})).expect("nochmal Muster");
        assert_eq!(fill(&s, id).pattern.as_ref().map(|p| p.name.clone()), Some(n));
    }
}

/// Weiche Kante 10 → Anti-Alias: die Weichheit gilt nicht mehr (PAR 18/11).
#[test]
fn edge_antialias_resets_feather() {
    let mut s = session();
    let id = rect(&mut s, [50, 50, 150, 120]);
    s.execute("pi.fill", json!({"edge": 2, "feather": 10})).expect("weich");
    assert_eq!((fill(&s, id).edge, fill(&s, id).feather), (2, 10));
    s.execute("pi.fill", json!({"edge": 1})).expect("Anti-Alias");
    assert_eq!((fill(&s, id).edge, fill(&s, id).feather), (1, 0));
    s.execute("edit.undo", json!({})).expect("undo");
    assert_eq!((fill(&s, id).edge, fill(&s, id).feather), (2, 10));
    s.execute("pi.fill", json!({"edge": 0, "feather": 5})).expect("hart");
    assert_eq!((fill(&s, id).edge, fill(&s, id).feather), (0, 0), "hart ohne Weichheit");
}

/// Verlaufsschritt der Füllungsänderung heißt wie im Original „Füllen“ (PAR 18/20).
#[test]
fn fill_step_named_like_original() {
    let mut s = session();
    rect(&mut s, [50, 50, 150, 120]);
    s.execute("pi.fill", json!({"color": "#ff0000"})).expect("rot");
    let d = s.active().expect("Dokument");
    assert_eq!(d.history.undo_label(), Some("Füllen"));
}

/// Rundung nach B/H-Änderung bezieht sich auf die aktuelle Größe (PAR 17/05).
#[test]
fn roundness_after_resize_uses_current_size() {
    let mut s = session();
    let id = rect(&mut s, [100, 105, 220, 185]);
    s.execute("element.setBounds", json!({"width": 150, "height": 100})).expect("Größe");
    s.execute("pi.roundness", json!({"value": 20})).expect("20 %");
    let pr = match &el(&s, id).kind {
        ElementKind::Path(p) => p.primitive.clone().expect("Grundform"),
        _ => panic!("Pfad"),
    };
    let (w, h) = panel::primitive_size(&pr);
    assert!((w - 150.0).abs() < 1e-3 && (h - 100.0).abs() < 1e-3, "{w} × {h}");
    assert!((panel::primitive_roundness_px(&pr) - 10.0).abs() < 1e-3, "20 % = 10 px");
    s.execute("pi.roundness", json!({"value": 10, "pixels": true})).expect("10 px");
    let pr = match &el(&s, id).kind {
        ElementKind::Path(p) => p.primitive.clone().expect("Grundform"),
        _ => panic!("Pfad"),
    };
    assert!((pr.roundness - 0.2).abs() < 1e-6, "{}", pr.roundness);
    assert_eq!(pr.roundness_pixels, Some(true));
}

/// B/H im Inspektor skaliert den Strich mit (Vorgabe „Attribute skalieren“, PAR 17/16) und
/// `pixelRect` enthält den Strich (PAR 17/08).
#[test]
fn stroke_scales_with_bounds_and_pixel_rect() {
    let mut s = session();
    let id = s.execute("draw.ellipse", json!({"rect": [50, 170, 150, 240]})).expect("Ellipse").value["id"].as_u64().expect("id");
    s.execute("pi.stroke", json!({"preset": "Einfach/Präzise abger.", "width": 8, "placement": "outside", "color": "#ff0000"})).expect("Strich");
    let px = |s: &Session| {
        let d = s.active().expect("Dokument");
        fp_engine::docops::element_pixel_rect(&d.doc, &el(s, id)).expect("pixelRect")
    };
    let r = px(&s);
    assert_eq!((r.left, r.top, r.right, r.bottom), (42.0, 162.0, 158.0, 248.0), "außen + 8");
    s.execute("pi.stroke", json!({"placement": "center"})).expect("mittig");
    let r = px(&s);
    assert_eq!((r.left, r.top, r.right, r.bottom), (46.0, 166.0, 154.0, 244.0), "mittig + 4");
    s.execute("pi.stroke", json!({"placement": "inside"})).expect("innen");
    let r = px(&s);
    assert_eq!((r.left, r.top, r.right, r.bottom), (50.0, 170.0, 150.0, 240.0), "innen");
    s.execute("pi.stroke", json!({"placement": "outside"})).expect("außen");
    s.execute("element.setBounds", json!({"width": 200})).expect("Breite");
    let b = fp_engine::docops::path_attrs_of(&el(&s, id)).and_then(|a| a.brush.clone()).expect("Pinsel");
    assert_eq!(b.diameter, 11, "8 · √2");
    let r = px(&s);
    assert_eq!((r.left, r.top, r.right, r.bottom), (39.0, 159.0, 261.0, 251.0));
    s.execute("edit.undo", json!({})).expect("undo");
    let b = fp_engine::docops::path_attrs_of(&el(&s, id)).and_then(|a| a.brush.clone()).expect("Pinsel");
    assert_eq!(b.diameter, 8, "Rückgängig in einem Schritt");
    // nur verschieben: Strich bleibt
    s.execute("element.setBounds", json!({"x": 60})).expect("X");
    let b = fp_engine::docops::path_attrs_of(&el(&s, id)).and_then(|a| a.brush.clone()).expect("Pinsel");
    assert_eq!(b.diameter, 8);
}
