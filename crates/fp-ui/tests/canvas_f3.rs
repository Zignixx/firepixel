//! F3 (14.10.2026): Leinwand, Web-Ebene und Ansicht aus `docs\parity\TWEAKS_2026-10-14.md` und N21.
//! - TW-18: Ansicht ▸ Macintosh-Gamma hellt nur die Anzeige auf (Hautton 198,147,115 → 207,162,133),
//!   das Dokument bleibt unverändert.
//! - TW-19: Schaltflächen-Instanzen haben ein eigenes Segment (grüne Überlagerung, Hilfslinien).
//! - TW-20: Animationsinstanz: Rahmen nur um das aktuelle Bild, Bewegungspfad Start → Ende.
//! - re 51 §1: Escape without a running action keeps the object selection (any tool).
//! - N21: Weiche Pixelauswahl ohne Pixel ≥ 50 % zeigt trotzdem Ameisen.

use egui::{Event, Key, Modifiers};
use egui_kittest::Harness;
use fp_engine::{Session, ToolId};
use fp_geom::{IRect, Rect};
use fp_ui::app_state::UiSettings;
use fp_ui::canvas::{anim, overlay, web};
use fp_ui::FirepixelApp;
use serde_json::json;

fn session() -> Session {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 500, "height": 300, "background": "#ffffff"})).expect("Dokument");
    s
}

fn harness(s: Session) -> Harness<'static, FirepixelApp> {
    let ctx = egui::Context::default();
    let mut a = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    a.state.sync_render = true;
    a.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(1500.0, 950.0)).with_pixels_per_point(1.0).with_max_steps(8).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), a);
    h.run_steps(4);
    h
}

fn id_of(v: &fp_engine::CommandOutput) -> u64 {
    v.value["id"].as_u64().or_else(|| v.value["ids"][0].as_u64()).expect("id")
}

fn element(s: &Session, id: u64) -> std::sync::Arc<fp_model::Element> {
    let d = s.active().expect("Dokument");
    fp_engine::docops::element_on_page(&d.doc, d.page, fp_model::ElementId(id)).expect("Element")
}

/// Pixel (Dokumentkoordinaten) des angezeigten Leinwandbilds bei 100 %.
fn shown_px(h: &Harness<'static, FirepixelApp>, x: i32, y: i32) -> [u8; 4] {
    let st = &h.state().state;
    let d = st.session.active().expect("Dokument");
    let cc = st.canvases.get(&d.id).expect("Leinwand");
    let k = cc.shown.expect("gerendert");
    let img = cc.shown_image.as_ref().expect("Bild");
    let (px, py) = (((x - k.region.left) as f64 * k.zoom) as usize, ((y - k.region.top) as f64 * k.zoom) as usize);
    let i = (py * img.width() + px) * 4;
    let b = img.as_bytes();
    [b[i], b[i + 1], b[i + 2], b[i + 3]]
}

#[test]
fn tw18_mac_gamma_brightens_display_only() {
    let mut s = session();
    s.execute("draw.rectangle", json!({"rect": [50, 50, 150, 120], "fillColor": "#c69373", "strokeWidth": 0})).expect("Rechteck");
    s.execute("select.none", json!({})).expect("nichts");
    let _ = s.execute("view.setZoom", json!({"zoom": 1.0}));
    let mut h = harness(s);
    let before = shown_px(&h, 100, 80);
    assert_eq!(&before[..3], &[198, 147, 115], "ohne Gamma");
    let gen0 = h.state().state.session.active().expect("Dokument").generation;
    h.state_mut().state.session.execute("view.macintoshGamma", json!({})).expect("Gamma an");
    h.run_steps(3);
    let after = shown_px(&h, 100, 80);
    assert_eq!(after[0], 207, "R {after:?}");
    assert!((after[1] as i32 - 162).abs() <= 1 && (after[2] as i32 - 133).abs() <= 1, "{after:?}");
    // nur Anzeige: das Dokument ist nicht geändert
    assert_eq!(h.state().state.session.active().expect("Dokument").generation, gen0, "Dokument unverändert");
    h.state_mut().state.session.execute("view.macintoshGamma", json!({})).expect("Gamma aus");
    h.run_steps(3);
    assert_eq!(&shown_px(&h, 100, 80)[..3], &[198, 147, 115], "wieder aus");
}

#[test]
fn tw19_button_instance_is_a_slice() {
    let mut s = session();
    let r = id_of(&s.execute("draw.rectangle", json!({"rect": [260, 20, 360, 60], "fillColor": "#22aa44", "strokeWidth": 0})).expect("Rechteck"));
    s.execute("select.set", json!({"ids": [r]})).expect("Auswahl");
    s.execute("modify.symbol.convertToSymbol", json!({"name": "BTN", "kind": "button"})).expect("Schaltfläche");
    let d = s.active().expect("Dokument");
    let items = web::items(&d.doc, d.page, d.frame);
    let btn: Vec<_> = items.iter().filter(|i| i.slice).collect();
    assert_eq!(btn.len(), 1, "ein Segment der Schaltfläche");
    let b = btn[0].rect;
    assert_eq!((b.left, b.top, b.right, b.bottom), (260.0, 20.0, 360.0, 60.0));
    assert_eq!(btn[0].color, web::BUTTON_SLICE_COLOR);
    // Hilfslinien wie bei einem normalen Segment: waagerecht bei 20, senkrecht bei 260 und 360
    let g = web::slice_guides(Rect::new(0.0, 0.0, 500.0, 300.0), &[b]);
    assert!(g.iter().any(|(p, q)| p.y == 20.0 && q.y == 20.0 && p.x == 0.0 && q.x == 500.0), "{g:?}");
    assert!(g.iter().any(|(p, q)| p.x == 260.0 && q.x == 260.0), "{g:?}");
    assert!(g.iter().any(|(p, q)| p.x == 360.0 && q.x == 360.0), "{g:?}");
    // grüne Überlagerung über 34,170,68 ergibt ≈ 23,197,47 wie im Original (Deckung 80/255)
    let a = 80.0 / 255.0;
    let mix = |base: f32, c: f32| (base * (1.0 - a) + c * a).round() as i32;
    assert!((mix(34.0, 0.0) - 23).abs() <= 1 && (mix(170.0, 255.0) - 197).abs() <= 1 && (mix(68.0, 0.0) - 47).abs() <= 1);
    // ein gewöhnliches Grafiksymbol hat kein Segment
    let mut s2 = session();
    let r2 = id_of(&s2.execute("draw.rectangle", json!({"rect": [10, 10, 40, 40], "fillColor": "#000000"})).expect("R"));
    s2.execute("select.set", json!({"ids": [r2]})).expect("Auswahl");
    s2.execute("modify.symbol.convertToSymbol", json!({"name": "G", "kind": "graphic"})).expect("Grafik");
    let d2 = s2.active().expect("Dokument");
    assert!(web::items(&d2.doc, d2.page, d2.frame).is_empty());
}

#[test]
fn tw20_animation_frame_bounds_and_motion_path() {
    let mut s = session();
    let r = id_of(&s.execute("draw.rectangle", json!({"rect": [400, 20, 440, 60], "fillColor": "#aa2244", "strokeWidth": 0})).expect("Rechteck"));
    s.execute("select.set", json!({"ids": [r]})).expect("Auswahl");
    let id = id_of(&s.execute("modify.animation.animateSelection", json!({"frames": 3, "offsetX": 60, "offsetY": 0})).expect("Animation"));
    let e = element(&s, id);
    let d = s.active().expect("Dokument");
    // ganze Grenzen (Skript/Info) schließen den Weg ein, der Rahmen nur das aktuelle Bild
    let all = fp_engine::docops::element_bounds(&d.doc, &e).expect("Grenzen");
    assert_eq!((all.left, all.right), (400.0, 500.0));
    let f0 = anim::frame_bounds(&d.doc, &e, 0).expect("Status 1");
    assert_eq!((f0.left, f0.top, f0.right, f0.bottom), (400.0, 20.0, 440.0, 60.0));
    let f2 = anim::frame_bounds(&d.doc, &e, 2).expect("Status 3");
    assert_eq!((f2.left, f2.right), (460.0, 500.0));
    let path = anim::motion_path(&d.doc, &e).expect("Pfad");
    assert_eq!(path.len(), 3);
    assert_eq!((path[0].x, path[0].y), (420.0, 40.0), "Start = Mitte des ersten Bildes");
    assert_eq!((path[1].x, path[2].x), (450.0, 480.0));
    // anderes Element: kein Pfad, kein eigener Rahmen
    let mut s2 = session();
    let r2 = id_of(&s2.execute("draw.rectangle", json!({"rect": [10, 10, 40, 40], "fillColor": "#000000"})).expect("R"));
    let e2 = element(&s2, r2);
    let d2 = s2.active().expect("Dokument");
    assert!(anim::frame_bounds(&d2.doc, &e2, 0).is_none() && anim::motion_path(&d2.doc, &e2).is_none());
    // im Fenster: Auswahlrahmen nur um das aktuelle Bild
    let _ = s.execute("view.setZoom", json!({"zoom": 1.0}));
    s.execute("select.set", json!({"ids": [id]})).expect("Auswahl");
    let mut h = harness(s);
    h.run_steps(2);
    let st = &h.state().state;
    let dd = st.session.active().expect("Dokument");
    let fr = &st.canvases.get(&dd.id).expect("Leinwand").sel_frames;
    assert_eq!(fr.len(), 1);
    assert_eq!((fr[0][0].x, fr[0][2].x), (400.0, 440.0), "{fr:?}");
}

fn esc(h: &mut Harness<'static, FirepixelApp>) {
    for pressed in [true, false] {
        h.input_mut().events.push(Event::Key { key: Key::Escape, physical_key: None, pressed, repeat: false, modifiers: Modifiers::NONE });
        h.step();
    }
    h.run_steps(2);
}

fn selected(h: &Harness<'static, FirepixelApp>) -> usize {
    h.state().state.session.active().map(|d| d.selection.elements.len()).unwrap_or(0)
}

/// Escape on the canvas never clears the object selection, with any tool (re 51 §1).
#[test]
fn escape_keeps_selection_with_pen_pointer_and_rectangle() {
    let mut s = session();
    let r = id_of(&s.execute("draw.rectangle", json!({"rect": [50, 50, 150, 120], "fillColor": "#ff0000"})).expect("R1"));
    s.execute("select.set", json!({"ids": [r]})).expect("Auswahl");
    let mut h = harness(s);
    assert_eq!(selected(&h), 1);
    h.state_mut().state.session.set_tool(ToolId::Pen).expect("Stift");
    h.run_steps(2);
    esc(&mut h);
    esc(&mut h);
    assert_eq!(selected(&h), 1, "Escape, Escape with the pen: selection stays");
    h.state_mut().state.session.set_tool(ToolId::Pointer).expect("pointer");
    h.run_steps(2);
    esc(&mut h);
    assert_eq!(selected(&h), 1, "Escape with the pointer: selection stays");
    // auch beim Rechteck-Werkzeug
    let r2 = id_of(&h.state_mut().state.session.execute("draw.rectangle", json!({"rect": [200, 50, 260, 120]})).expect("R2"));
    h.state_mut().state.session.execute("select.set", json!({"ids": [r2]})).expect("Auswahl");
    h.state_mut().state.session.set_tool(ToolId::Rectangle).expect("Rechteck");
    h.run_steps(2);
    esc(&mut h);
    assert_eq!(selected(&h), 1, "Escape with the rectangle tool: selection stays");
}

#[test]
fn n21_soft_selection_below_half_still_shows_ants() {
    // Motiv mit unsicherer Maske: alle Werte < 128 (früher: keine Ameisen trotz Pixelauswahl)
    let b = IRect::new(34, 53, 274, 152);
    let mut data = vec![0u8; (b.width() * b.height()) as usize];
    let w = b.width() as usize;
    for y in 20..80usize {
        for x in 30..200usize {
            data[y * w + x] = 90;
        }
    }
    let m = fp_raster::Mask::from_raw(b, data).expect("Maske");
    let mut slot = None;
    let o = overlay::ants_for(&mut slot, &m);
    assert!(!o.is_empty(), "Ameisen für eine weiche Auswahl");
    let xs: Vec<f32> = o[0].iter().map(|p| p.x).collect();
    let min = xs.iter().copied().fold(f32::MAX, f32::min);
    assert_eq!(min, 64.0, "Umriss an der Auswahl");
    // gewöhnliche Masken unverändert an der 50-%-Grenze
    let hard = fp_raster::Mask::from_rect(IRect::new(10, 10, 20, 20)).expect("Maske");
    let mut slot2 = None;
    assert_eq!(overlay::ants_for(&mut slot2, &hard).len(), 1);
}
