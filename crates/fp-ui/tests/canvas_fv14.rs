//! FV-14: Leinwand-Befunde aus PARITAET_A Abschnitt 5 (Paket P-LW1), mit der Sitzung der
//! Oberfläche bzw. (L7) dem ganzen Fenster in egui_kittest. Messwerte aus dem echten Original
//! (PM-3, FV-11, FV-14), Testdokument 600 × 400 bei 100 %: R1 50,50–150,120, E1 200,50–300,120,
//! Gruppe G 350–470 × 50–100, Bitmap B 50,250–130,330, Bitmap B2 150–270 × 340–395 mit
//! durchsichtiger Lücke 200–220.
//! - L4: Verschieben rastet nicht mit einer Kante an der Mitte eines anderen Objekts ein (R1 um
//!   +40/+20 bzw. +40/+21 → Oberkante 70 bzw. 71, nicht 75); Kante an Kante rastet weiter.
//! - L35: Stiftpunkte rasten nicht an Objektmitten (585,80 bleibt 585,80).
//! - L33: Alt-Ziehen aus einem Eckpunkt mit der Unterauswahl zieht nur den Ausgangshenkel heraus.
//! - L13: Zauberstab wählt die Bitmap als Objekt; Esc beendet den Bitmapmodus, sie bleibt gewählt.
//! - L25: Zoomrahmen passt in den Sichtbereich minus 17 px je Richtung ein.
//! - L7: Strg+Leertaste+Klick zoomt auch, wenn Tasten und Klick im selben Bild ankommen.
//! - FV-11 → B-2b BA-007 (neu gemessen): Klick auf ein durchsichtiges Pixel einer Bitmap wählt sie
//!   **nicht** (Zeiger, Umschalt, Unterauswahl); Ziehen von dort zieht einen Auswahlrahmen auf.

use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use fp_engine::tools::ToolId;
use fp_engine::{Session, ToolEvent, ToolKey};
use fp_geom::{IRect, Rect};
use fp_model::{ElementId, ElementKind};
use fp_ui::app_state::UiSettings;
use fp_ui::FirepixelApp;
use serde_json::json;

struct T {
    a: FirepixelApp,
    r1: ElementId,
    b: ElementId,
    b2: ElementId,
}

fn id_of(v: &fp_engine::CommandOutput) -> ElementId {
    ElementId(v.value["id"].as_u64().or_else(|| v.value["ids"][0].as_u64()).expect("id"))
}

fn rect(s: &mut Session, r: [i32; 4], c: &str) -> ElementId {
    id_of(&s.execute("draw.rectangle", json!({"rect": r, "fillColor": c, "strokeWidth": 0})).expect("Rechteck"))
}

fn paste(s: &mut Session, img: fp_raster::RgbaImage) -> ElementId {
    s.clipboard.clear();
    s.clipboard.pixels = Some(img);
    id_of(&s.execute("edit.paste", json!({})).expect("Bitmap"))
}

fn session_doc() -> Session {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 600, "height": 400, "background": "#ffffff"})).expect("Dokument");
    s
}

fn build(s: &mut Session) -> (ElementId, ElementId, ElementId, ElementId) {
    let r1 = rect(s, [50, 50, 150, 120], "#ff0000");
    s.execute("draw.ellipse", json!({"rect": [200, 50, 300, 120], "fillColor": "#00aa00", "strokeWidth": 0})).expect("E1");
    let ga = rect(s, [350, 50, 400, 100], "#888800");
    let gb = rect(s, [420, 50, 470, 100], "#888800");
    s.execute("select.set", json!({"ids": [ga.0, gb.0]})).expect("Auswahl");
    let g = id_of(&s.execute("modify.group", json!({})).expect("Gruppe"));
    let b = paste(s, fp_raster::RgbaImage::filled(IRect::new(50, 250, 130, 330), fp_color::Rgba8::new(255, 136, 0, 255)).expect("B"));
    let mut img = fp_raster::RgbaImage::filled(IRect::new(150, 340, 270, 395), fp_color::Rgba8::new(255, 0, 0, 255)).expect("B2");
    img.fill_rect(IRect::new(220, 340, 270, 395), fp_color::Rgba8::new(0, 0, 255, 255));
    img.fill_rect(IRect::new(200, 340, 220, 395), fp_color::Rgba8::new(0, 0, 0, 0));
    let b2 = paste(s, img);
    s.execute("select.none", json!({})).expect("nichts");
    (r1, g, b, b2)
}

fn app() -> T {
    let mut s = session_doc();
    let (r1, _g, b, b2) = build(&mut s);
    let ctx = egui::Context::default();
    let mut a = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    a.state.sync_render = true;
    let _ = a.state.session.execute("view.setZoom", json!({"zoom": 1.0}));
    T { a, r1, b, b2 }
}

fn ses(t: &mut T) -> &mut Session {
    &mut t.a.state.session
}

fn sel(s: &Session) -> Vec<ElementId> {
    s.active().map(|d| d.selection.elements.clone()).unwrap_or_default()
}

fn bounds(s: &Session, id: ElementId) -> Rect {
    let d = s.active().expect("Dokument");
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, id).expect("Element");
    fp_engine::docops::element_bounds(&d.doc, &e).expect("Grenzen")
}

fn all_ids(s: &Session) -> Vec<ElementId> {
    let d = s.active().expect("Dokument");
    fp_engine::docops::own_layers(&d.doc, d.page).into_iter().flat_map(|(_, l)| l.cels[0].elements.iter().map(|e| e.id).collect::<Vec<_>>()).collect()
}

fn top_id(s: &Session) -> ElementId {
    let d = s.active().expect("Dokument");
    let l = fp_engine::docops::own_layers(&d.doc, d.page).into_iter().rev().find(|(_, l)| !l.cels[0].elements.is_empty()).expect("Ebene").1;
    l.cels[0].elements.first().expect("Element").id
}

fn tool(s: &mut Session, t: ToolId) {
    s.set_tool(t).expect("Werkzeug");
}

fn ev(s: &mut Session, e: ToolEvent) {
    s.tool_event(&e).expect("Ereignis");
}

/// Ziehen wie die Messskripte: 20 Zwischenschritte, Umschalttasten über `m`.
fn drag(s: &mut Session, a: (f32, f32), b: (f32, f32), m: fn(ToolEvent) -> ToolEvent) {
    ev(s, m(ToolEvent::down(a.0, a.1)));
    for i in 1..=20 {
        let f = i as f32 / 20.0;
        ev(s, m(ToolEvent::drag((a.0 + (b.0 - a.0) * f).round(), (a.1 + (b.1 - a.1) * f).round())));
    }
    ev(s, m(ToolEvent::up(b.0, b.1)));
}

fn plain(e: ToolEvent) -> ToolEvent {
    e
}
fn alt(e: ToolEvent) -> ToolEvent {
    e.alt()
}
fn shift(e: ToolEvent) -> ToolEvent {
    e.shift()
}

fn click(s: &mut Session, x: f32, y: f32, m: fn(ToolEvent) -> ToolEvent) {
    ev(s, m(ToolEvent::down(x, y)));
    ev(s, m(ToolEvent::up(x, y)));
}

fn undo(s: &mut Session) {
    s.execute("edit.undo", json!({})).expect("Rückgängig");
}

fn nodes(s: &Session, id: ElementId) -> Vec<fp_model::element::ContourNode> {
    let d = s.active().expect("Dokument");
    match &fp_engine::docops::element_on_page(&d.doc, d.page, id).expect("Pfad").kind {
        ElementKind::Path(p) => p.contours[0].nodes.clone(),
        _ => panic!("kein Pfad"),
    }
}

fn pt(p: fp_geom::Point) -> (f32, f32) {
    (p.x, p.y)
}

// ------------------------------------------------------------------ L4 Verschieben

#[test]
fn l4_verschieben_rastet_nicht_mit_der_kante_an_einer_objektmitte() {
    let mut t = app();
    let r1 = t.r1;
    let s = ses(&mut t);
    tool(s, ToolId::Pointer);
    // Original: 90,70–190,140 bzw. 90,71–190,141 (G-Mitte y = 75 liegt 5 bzw. 4 px entfernt);
    // mit Alt entsteht die Kopie an derselben Stelle
    for (dy, m, copy) in [(20.0, plain as fn(ToolEvent) -> ToolEvent, false), (21.0, plain, false), (20.0, alt, true), (21.0, alt, true)] {
        click(s, 580.0, 385.0, plain);
        let before = all_ids(s);
        drag(s, (100.0, 85.0), (140.0, 85.0 + dy), m);
        let moved = if copy { all_ids(s).into_iter().find(|i| !before.contains(i)).expect("Kopie") } else { r1 };
        if copy {
            assert_ne!(moved, r1, "Alt-Ziehen legt eine Kopie an");
        }
        let b = bounds(s, moved);
        assert_eq!((b.left, b.top, b.right, b.bottom), (90.0, 50.0 + dy, 190.0, 120.0 + dy), "dy {dy} Alt {copy}: kein Fang an der G-Mitte");
        undo(s);
        assert_eq!(bounds(s, r1).top, 50.0, "Rückgängig stellt her");
    }
}

#[test]
fn l4_verschieben_rastet_kante_an_kante_und_mitte_an_mitte() {
    let mut t = app();
    let r1 = t.r1;
    let s = ses(&mut t);
    tool(s, ToolId::Pointer);
    // rechte Kante 197 rastet an der linken Kante von E1 (200) ein
    drag(s, (100.0, 85.0), (147.0, 85.0), plain);
    let b = bounds(s, r1);
    assert_eq!((b.left, b.right), (100.0, 200.0), "Kante an Kante");
    undo(s);
    // Mitte 78 rastet an der Mitte von G (75) ein (Oberkante 43, Unterkante 113 ohne Ziel)
    click(s, 580.0, 385.0, plain);
    drag(s, (100.0, 85.0), (100.0, 78.0), plain);
    let b = bounds(s, r1);
    assert_eq!((b.top, b.bottom), (40.0, 110.0), "Mitte an Mitte");
    undo(s);
    // Mitte 102 rastet nicht an der Unterkante von G (100)
    click(s, 580.0, 385.0, plain);
    drag(s, (100.0, 85.0), (100.0, 102.0), plain);
    assert_eq!(bounds(s, r1).top, 67.0, "Mitte nicht an einer Kante");
}

// ------------------------------------------------------------------ L35 Stift

#[test]
fn l35_stiftpunkte_rasten_nicht_an_objektmitten() {
    let mut t = app();
    let s = ses(&mut t);
    tool(s, ToolId::Pen);
    for (x, y) in [(545.0, 30.0), (585.0, 30.0), (585.0, 80.0)] {
        click(s, x, y, plain);
    }
    click(s, 545.0, 30.0, plain);
    let p = top_id(s);
    let n = nodes(s, p);
    let pts: Vec<(f32, f32)> = n.iter().map(|n| pt(n.anchor)).collect();
    assert_eq!(pts, vec![(545.0, 30.0), (585.0, 30.0), (585.0, 80.0)], "dritter Punkt bleibt 585,80 (G-Mitte 75 liegt 5 px entfernt)");
    assert_eq!(s.tools.active, ToolId::Pen, "Werkzeug bleibt Stift");
    // das Aufziehen von Rechtecken rastet weiter an Mitten (L16): Start 247 → E1-Mitte 250
    tool(s, ToolId::Rectangle);
    drag(s, (247.0, 140.0), (320.0, 180.0), plain);
    let r = bounds(s, top_id(s));
    assert_eq!(r.left, 250.0, "Rechteck rastet an der E1-Mitte");
}

// ------------------------------------------------------------------ L33 Unterauswahl

#[test]
fn l33_alt_ziehen_aus_eckpunkt_zieht_nur_den_ausgangshenkel() {
    let mut t = app();
    let s = ses(&mut t);
    tool(s, ToolId::Pen);
    click(s, 320.0, 140.0, plain);
    drag(s, (400.0, 140.0), (440.0, 180.0), plain);
    ev(s, ToolEvent::key(ToolKey::Escape));
    let p = top_id(s);
    let n = nodes(s, p);
    assert_eq!((pt(n[1].pred), pt(n[1].succ)), ((360.0, 100.0), (440.0, 180.0)), "L31 Henkel von Punkt 2");
    s.execute("select.set", json!({"ids": [p.0]})).expect("Auswahl");
    tool(s, ToolId::Subselection);
    drag(s, (320.0, 140.0), (350.0, 140.0), alt);
    let n = nodes(s, p);
    assert_eq!(pt(n[0].anchor), (320.0, 140.0), "Punkt bleibt");
    assert_eq!(pt(n[0].pred), (320.0, 140.0), "Eingangshenkel bleibt am Punkt (nicht 290,140)");
    assert_eq!(pt(n[0].succ), (350.0, 140.0), "Ausgangshenkel unter dem Zeiger");
    assert_eq!((pt(n[1].pred), pt(n[1].succ)), ((360.0, 100.0), (440.0, 180.0)), "Punkt 2 unverändert");
    undo(s);
    let n = nodes(s, p);
    assert_eq!((pt(n[0].pred), pt(n[0].succ)), ((320.0, 140.0), (320.0, 140.0)), "Rückgängig stellt her");
    // Henkel ohne Alt ziehen: Gegenhenkel dreht mit (L32, unverändert)
    click(s, 400.0, 140.0, plain);
    drag(s, (440.0, 180.0), (460.0, 160.0), plain);
    let n = nodes(s, p);
    assert_eq!(pt(n[1].succ), (460.0, 160.0));
    assert!((n[1].pred.x - 346.3).abs() < 0.1 && (n[1].pred.y - 122.1).abs() < 0.1, "{:?}", n[1].pred);
}

// ------------------------------------------------------------------ L13 Zauberstab, Esc

#[test]
fn l13_zauberstab_waehlt_die_bitmap_und_esc_laesst_sie_gewaehlt() {
    let mut t = app();
    let (r1, b) = (t.r1, t.b);
    let s = ses(&mut t);
    for pre in [None, Some(r1)] {
        s.execute("select.set", json!({"ids": pre.map(|i| vec![i.0]).unwrap_or_default()})).expect("Auswahl");
        tool(s, ToolId::MagicWand);
        click(s, 90.0, 290.0, plain);
        {
            let d = s.active().expect("Dokument");
            assert!(d.selection.has_pixels(), "Auswahlrahmen");
            assert!(d.bitmap_mode, "Bitmapmodus");
            assert_eq!(d.selection.elements, vec![b], "im Bitmapmodus ist B gewählt (vorher {pre:?})");
        }
        ev(s, ToolEvent::key(ToolKey::Escape));
        let d = s.active().expect("Dokument");
        assert!(!d.selection.has_pixels() && !d.bitmap_mode, "Bitmapmodus beendet");
        assert_eq!(d.selection.elements, vec![b], "B bleibt gewählt");
        assert_eq!(s.tools.active, ToolId::Pointer, "Werkzeug Zeiger");
    }
}

// ------------------------------------------------------------------ L25 Zoomrahmen

#[test]
fn l25_zoomrahmen_passt_mit_17_px_abzug_ein() {
    let mut t = app();
    let r1 = t.r1;
    let s = ses(&mut t);
    // Sichtbereich wie im gemessenen Original (1462 × 822)
    s.execute("view.setViewport", json!({"width": 1462, "height": 822})).expect("Sichtbereich");
    for (r, want) in [([50, 50, 150, 120], 11.5), ([50, 50, 250, 120], 7.225), ([320, 140, 420, 160], 14.45)] {
        s.execute("view.setZoom", json!({"zoom": 1.0})).expect("100 %");
        s.execute("view.zoomToRect", json!({"rect": r})).expect("Zoomrahmen");
        let z = s.active().expect("Dokument").view.zoom;
        assert!((z - want).abs() < 1e-9, "{r:?}: {z}, erwartet {want}");
    }
    // auch über das Zoomwerkzeug (Rahmen aufziehen)
    s.execute("view.setZoom", json!({"zoom": 1.0})).expect("100 %");
    tool(s, ToolId::Zoom);
    drag(s, (50.0, 50.0), (150.0, 120.0), plain);
    let z = s.active().expect("Dokument").view.zoom;
    assert!((z - 11.5).abs() < 1e-9, "Zoomwerkzeug: {z}");
    // „Einpassen“ behält seinen eigenen Rand (15 px je Seite)
    s.execute("select.set", json!({"ids": [r1.0]})).expect("Auswahl");
    s.execute("view.fitSelection", json!({})).expect("Einpassen");
    let z = s.active().expect("Dokument").view.zoom;
    assert!((z - 792.0 / 70.0).abs() < 1e-9, "Auswahl einpassen: {z}");
}

// ------------------------------------------------------------------ BA-007 durchsichtige Pixel

/// B-2b BA-007 (Mess-Sitzung, ersetzt FV-11): Treffer auf Bitmaps nur an Pixeln mit Alpha > 0.
#[test]
fn klick_auf_durchsichtiges_bitmap_pixel_waehlt_nichts() {
    let mut t = app();
    let (r1, b2) = (t.r1, t.b2);
    let s = ses(&mut t);
    tool(s, ToolId::Pointer);
    click(s, 210.0, 367.0, plain);
    assert!(sel(s).is_empty(), "Zeiger: Klick in die Lücke wählt nichts");
    click(s, 170.0, 367.0, plain);
    assert_eq!(sel(s), vec![b2], "deckendes Pixel wählt B2");
    click(s, 580.0, 385.0, plain);
    assert!(sel(s).is_empty());
    // Umschalt-Klick in die Lücke fügt nichts hinzu
    click(s, 100.0, 85.0, plain);
    click(s, 210.0, 367.0, shift);
    assert_eq!(sel(s), vec![r1], "Umschalt-Klick in die Lücke: nur R1");
    click(s, 580.0, 385.0, plain);
    // Ziehen aus der Lücke verschiebt B2 nicht
    drag(s, (210.0, 367.0), (230.0, 367.0), plain);
    assert_eq!(bounds(s, b2).left, 150.0, "Ziehen aus der Lücke verschiebt B2 nicht");
    // Unterauswahl
    click(s, 580.0, 385.0, plain);
    tool(s, ToolId::Subselection);
    click(s, 210.0, 367.0, plain);
    assert!(sel(s).is_empty(), "Unterauswahl: Klick in die Lücke wählt nichts");
}

// ------------------------------------------------------------------ L7 Strg+Leertaste+Klick

type H = Harness<'static, FirepixelApp>;

fn harness() -> (H, ElementId) {
    let mut s = session_doc();
    let (r1, ..) = build(&mut s);
    let _ = s.execute("view.setZoom", json!({"zoom": 1.0}));
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.os_clipboard = false;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1600.0, 1000.0))
        .with_pixels_per_point(1.0)
        .with_max_steps(20)
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    let _ = h.state_mut().state.session.execute("view.setZoom", json!({"zoom": 1.0}));
    h.run_steps(2);
    (h, r1)
}

fn screen(h: &H, x: f32, y: f32) -> Pos2 {
    let s = &h.state().state;
    let d = s.session.active().expect("Dokument");
    let area = s.canvases.get(&d.id).and_then(|c| c.area).expect("Leinwand");
    let (sx, sy) = d.view.doc_to_screen(fp_geom::Point::new(x, y));
    Pos2::new(area.min.x + sx as f32, area.min.y + sy as f32)
}

fn zoom(h: &H) -> f64 {
    h.state().state.session.active().expect("Dokument").view.zoom
}

fn space(pressed: bool, m: Modifiers) -> Event {
    Event::Key { key: Key::Space, physical_key: Some(Key::Space), pressed, repeat: false, modifiers: m }
}

fn button(p: Pos2, pressed: bool, m: Modifiers) -> Event {
    Event::PointerButton { pos: p, button: PointerButton::Primary, pressed, modifiers: m }
}

#[test]
fn l7_strg_leertaste_klick_im_selben_bild_zoomt() {
    let (mut h, r1) = harness();
    let ctrl = Modifiers { ctrl: true, command: true, ..Default::default() };
    let p = screen(&h, 100.0, 85.0);
    h.event(Event::PointerMoved(p));
    h.run_steps(1);
    // alles in einem Bild: Strg, Leertaste, Drücken, Loslassen, Leertaste und Strg los
    for e in [Event::ModifiersChanged(ctrl), space(true, ctrl), button(p, true, ctrl), button(p, false, ctrl), space(false, ctrl), Event::ModifiersChanged(Modifiers::NONE)] {
        h.event(e);
    }
    h.run_steps(2);
    assert!((zoom(&h) - 1.5).abs() < 1e-9, "100 → 150 %: {}", zoom(&h));
    let s = &h.state().state.session;
    assert!(s.active().expect("Dokument").selection.elements.is_empty(), "kein Zeigerklick auf R1 ({r1:?})");
    // Tasten schon ein Bild vorher gehalten (bisheriger Fall): 150 → 200 %
    let p = screen(&h, 100.0, 85.0);
    h.event(Event::PointerMoved(p));
    h.event(Event::ModifiersChanged(ctrl));
    h.event(space(true, ctrl));
    h.run_steps(1);
    h.event(button(p, true, ctrl));
    h.run_steps(1);
    h.event(button(p, false, ctrl));
    h.run_steps(1);
    h.event(space(false, ctrl));
    h.event(Event::ModifiersChanged(Modifiers::NONE));
    h.run_steps(2);
    assert!((zoom(&h) - 2.0).abs() < 1e-9, "150 → 200 %: {}", zoom(&h));
    assert!(h.state().state.session.active().expect("Dokument").selection.elements.is_empty(), "wieder kein Zeigerklick");
    // ein normaler Klick danach wählt R1 wie gewohnt
    let p = screen(&h, 100.0, 85.0);
    h.event(Event::PointerMoved(p));
    h.event(button(p, true, Modifiers::NONE));
    h.event(button(p, false, Modifiers::NONE));
    h.run_steps(2);
    assert_eq!(h.state().state.session.active().expect("Dokument").selection.elements, vec![r1], "normaler Klick wählt R1");
}
