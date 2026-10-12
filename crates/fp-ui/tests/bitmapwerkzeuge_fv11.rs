//! FV-11: Bitmapwerkzeuge wie im Original (PARITAET_A L36–L39), mit der Sitzung der Oberfläche
//! (Renderer eingehängt). Messwerte: echtes Original, eigenes Testdokument 600 × 400 mit R1
//! (Vektor 50,50–150,120) und B2 (Bitmap rot 150–210 / blau 210–270 × 340–395), 100 %.
//! - Pinsel: malt immer Pixel – ohne ausgewählte Bitmap (nichts oder ein Vektor gewählt) entsteht
//!   eine neue Bitmap in Strichgröße (oben, ausgewählt), mit ausgewählter Bitmap malt er hinein
//!   (sie wächst über ihren Rand hinaus); das Werkzeug bleibt der Pinsel.
//! - Esc in einem Malwerkzeug: mit Bitmap als Ziel → Zeiger, Bitmap bleibt gewählt (auch ohne
//!   Malen); ohne Bitmap bleibt das Werkzeug.
//! - Abdruckprofile von Radiergummi (13 / Rand 2), Weichzeichner (13 / 100 / 50) und Stempel
//!   (16 / Rand 50) als Einzelklick und als Strich.

#![allow(unused_variables)]

use fp_engine::tools::ToolId;
use fp_engine::{Session, ToolEvent, ToolKey};
use fp_geom::IRect;
use fp_model::{ElementId, ElementKind};
use fp_ui::app_state::UiSettings;
use fp_ui::FirepixelApp;
use serde_json::json;

struct T {
    a: FirepixelApp,
    r1: ElementId,
    b2: ElementId,
}

fn app() -> T {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 600, "height": 400, "background": "#ffffff"})).expect("Dokument");
    let ctx = egui::Context::default();
    let mut a = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    a.state.sync_render = true;
    let s = &mut a.state.session;
    let r = s.execute("draw.rectangle", json!({"rect": [50, 50, 150, 120], "fillColor": "#ff0000", "strokeWidth": 0})).expect("R1");
    let r1 = ElementId(r.value["id"].as_u64().expect("id R1"));
    let mut img = fp_raster::RgbaImage::filled(IRect::new(150, 340, 270, 395), fp_color::Rgba8::new(255, 0, 0, 255)).expect("Bild");
    img.fill_rect(IRect::new(210, 340, 270, 395), fp_color::Rgba8::new(0, 0, 255, 255));
    s.clipboard.clear();
    s.clipboard.pixels = Some(img);
    let v = s.execute("edit.paste", json!({})).expect("B2");
    let b2 = ElementId(v.value["ids"][0].as_u64().expect("id B2"));
    s.execute("select.none", json!({})).expect("nichts");
    s.set_colors(Some(fp_color::Argb(0xffc0_4040)), None);
    T { a, r1, b2 }
}

fn ses(t: &mut T) -> &mut Session {
    &mut t.a.state.session
}

fn ids(s: &Session) -> Vec<ElementId> {
    let d = s.active().expect("Dokument");
    fp_engine::docops::own_layers(&d.doc, d.page).into_iter().rev().flat_map(|(_, l)| l.cels[0].elements.iter().map(|e| e.id).collect::<Vec<_>>()).collect()
}

fn sel(s: &Session) -> Vec<ElementId> {
    s.active().map(|d| d.selection.elements.clone()).unwrap_or_default()
}

fn image(s: &Session, id: ElementId) -> fp_raster::RgbaImage {
    let d = s.active().expect("Dokument");
    match &fp_engine::docops::element_on_page(&d.doc, d.page, id).expect("Element").kind {
        ElementKind::Bitmap(b) => fp_engine::docops::bitmap_image(b).expect("Bild"),
        _ => panic!("keine Bitmap"),
    }
}

fn is_bitmap(s: &Session, id: ElementId) -> bool {
    let d = s.active().expect("Dokument");
    fp_engine::docops::element_on_page(&d.doc, d.page, id).is_some_and(|e| matches!(e.kind, ElementKind::Bitmap(_)))
}

fn px(s: &Session, id: ElementId, x: i32, y: i32) -> [u8; 4] {
    image(s, id).get(x, y).unwrap_or([0; 4])
}

fn undo_count(s: &Session) -> usize {
    s.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

fn tool(s: &mut Session, t: ToolId) {
    s.set_tool(t).expect("Werkzeug");
}

/// Ziehen wie die Messskripte: 20 Zwischenschritte.
fn drag(s: &mut Session, a: (f32, f32), b: (f32, f32)) {
    s.tool_event(&ToolEvent::down(a.0, a.1)).expect("down");
    for i in 1..=20 {
        let f = i as f32 / 20.0;
        s.tool_event(&ToolEvent::drag((a.0 + (b.0 - a.0) * f).round(), (a.1 + (b.1 - a.1) * f).round())).expect("drag");
    }
    s.tool_event(&ToolEvent::up(b.0, b.1)).expect("up");
}

fn click(s: &mut Session, x: f32, y: f32) {
    s.tool_event(&ToolEvent::down(x, y)).expect("down");
    s.tool_event(&ToolEvent::up(x, y)).expect("up");
}

fn esc(s: &mut Session) {
    s.tool_event(&ToolEvent::key(ToolKey::Escape)).expect("Esc");
}

fn select(s: &mut Session, id: ElementId) {
    s.execute("select.set", json!({"ids": [id.0]})).expect("Auswahl");
}

fn near(v: u8, want: i32, tol: i32, what: &str) {
    assert!((v as i32 - want).abs() <= tol, "{what}: {v}, erwartet {want} ± {tol}");
}

// ------------------------------------------------------------------ L36 Pinsel

#[test]
fn pinsel_ohne_auswahl_legt_neue_bitmap_an_und_malt_weiter_hinein() {
    let mut t = app();
    let (r1, b) = (t.r1, t.b2);
    let s = ses(&mut t);
    tool(s, ToolId::Brush);
    let n0 = ids(s).len();
    let h0 = undo_count(s);
    // Esc ohne Bitmap: Werkzeug bleibt (Original M0)
    esc(s);
    assert_eq!(s.tools.active, ToolId::Brush);
    drag(s, (300.0, 20.0), (400.0, 20.0));
    let all = ids(s);
    assert_eq!(all.len(), n0 + 1, "genau ein neues Objekt");
    let nb = all[0];
    assert!(is_bitmap(s, nb), "Pinsel legt eine Bitmap an, keinen Pfad");
    assert_eq!(image(s, nb).alpha_bounds(), IRect::new(300, 20, 400, 21), "Strichgröße wie im Original (300,20–400,21)");
    assert_eq!(px(s, nb, 350, 20), [0xc0, 0x40, 0x40, 255], "mit der Strichfarbe");
    assert_eq!(sel(s), vec![nb], "neue Bitmap ausgewählt");
    assert_eq!(s.tools.active, ToolId::Brush, "Werkzeug bleibt Pinsel");
    assert_eq!(undo_count(s), h0 + 1, "ein Verlaufsschritt");
    // zweiter Strich malt in dieselbe (ausgewählte) Bitmap
    drag(s, (300.0, 40.0), (400.0, 40.0));
    assert_eq!(ids(s).len(), n0 + 1, "kein weiteres Objekt");
    assert_eq!(image(s, nb).alpha_bounds(), IRect::new(300, 20, 400, 41));
    // Esc: Zeiger, Bitmap bleibt gewählt (Original M3)
    esc(s);
    assert_eq!(s.tools.active, ToolId::Pointer);
    assert_eq!(sel(s), vec![nb]);
    // Rückgängig: erst der zweite Strich, dann die neue Bitmap
    s.undo().expect("undo");
    assert_eq!(image(s, nb).alpha_bounds(), IRect::new(300, 20, 400, 21));
    s.undo().expect("undo");
    assert_eq!(ids(s).len(), n0);
}

#[test]
fn pinsel_mit_gewaehltem_vektor_legt_neue_bitmap_an() {
    let mut t = app();
    let (r1, b) = (t.r1, t.b2);
    let s = ses(&mut t);
    select(s, r1);
    let before = s.active().map(|d| fp_engine::docops::element_on_page(&d.doc, d.page, r1)).expect("R1");
    let n0 = ids(s).len();
    tool(s, ToolId::Brush);
    drag(s, (60.0, 100.0), (140.0, 100.0));
    let all = ids(s);
    assert_eq!(all.len(), n0 + 1);
    assert!(is_bitmap(s, all[0]));
    assert_eq!(image(s, all[0]).alpha_bounds(), IRect::new(60, 100, 140, 101), "Original T0: 60,100–140,101");
    assert_eq!(sel(s), vec![all[0]]);
    let after = s.active().map(|d| fp_engine::docops::element_on_page(&d.doc, d.page, r1)).expect("R1");
    assert_eq!(before, after, "R1 unverändert");
}

#[test]
fn pinsel_malt_in_gewaehlte_bitmap_und_erweitert_sie() {
    let mut t = app();
    let (r1, b) = (t.r1, t.b2);
    let s = ses(&mut t);
    select(s, b);
    let n0 = ids(s).len();
    tool(s, ToolId::Brush);
    drag(s, (160.0, 330.0), (260.0, 330.0));
    assert_eq!(ids(s).len(), n0, "kein neues Objekt");
    assert_eq!(image(s, b).bounds().top, 330, "B2 wächst bis 330 (Original 150,330–270,395)");
    assert_eq!(px(s, b, 200, 330), [0xc0, 0x40, 0x40, 255]);
    drag(s, (160.0, 360.0), (260.0, 360.0));
    assert_eq!(px(s, b, 200, 360), [0xc0, 0x40, 0x40, 255]);
    assert_eq!(px(s, b, 200, 361), [255, 0, 0, 255]);
    assert_eq!(sel(s), vec![b]);
    esc(s);
    assert_eq!(s.tools.active, ToolId::Pointer);
    assert_eq!(sel(s), vec![b]);
}

#[test]
fn pinsel_vielfach_ohne_duplikate() {
    // §4b: 30 Striche in die gewählte neue Bitmap → genau ein neues Objekt, 30 Verlaufsschritte
    let mut t = app();
    let (r1, b) = (t.r1, t.b2);
    let s = ses(&mut t);
    tool(s, ToolId::Brush);
    let n0 = ids(s).len();
    let h0 = undo_count(s);
    for i in 0..30 {
        drag(s, (300.0, 100.0 + i as f32), (400.0, 100.0 + i as f32));
    }
    assert_eq!(ids(s).len(), n0 + 1);
    assert!(undo_count(s) > h0, "Verlaufsschritte");
    assert_eq!(image(s, ids(s)[0]).alpha_bounds(), IRect::new(300, 100, 400, 130));
    s.undo().expect("undo");
    assert_eq!(image(s, ids(s)[0]).alpha_bounds(), IRect::new(300, 100, 400, 129), "Rückgängig nimmt genau den letzten Strich");
}

// ------------------------------------------------------------------ L37 Radiergummi

#[test]
fn radiergummi_abdruck_und_strich_wie_im_original() {
    let mut t = app();
    let (r1, b) = (t.r1, t.b2);
    let s = ses(&mut t);
    select(s, b);
    tool(s, ToolId::Eraser);
    // Esc ohne Malen: Zeiger, B2 bleibt gewählt (Original M6a)
    esc(s);
    assert_eq!(s.tools.active, ToolId::Pointer);
    assert_eq!(sel(s), vec![b]);
    tool(s, ToolId::Eraser);
    click(s, 185.0, 367.0);
    // Einzelabdruck: Mitte auf Pixel 184,366; Rand bei Abstand 6 ≈ 86/255 gelöscht, 7 nur rechts/unten
    assert_eq!(px(s, b, 184, 366)[3], 0);
    assert_eq!(px(s, b, 179, 366)[3], 0);
    near(px(s, b, 178, 366)[3], 255 - 86, 12, "178,366");
    near(px(s, b, 190, 366)[3], 255 - 86, 12, "190,366");
    near(px(s, b, 191, 366)[3], 255 - 9, 6, "191,366");
    assert_eq!(px(s, b, 177, 366)[3], 255, "links von 178 unberührt");
    near(px(s, b, 184, 360)[3], 255 - 86, 12, "184,360");
    near(px(s, b, 184, 373)[3], 255 - 9, 6, "184,373");
    assert_eq!(px(s, b, 184, 359)[3], 255);
    esc(s);
    assert_eq!(s.tools.active, ToolId::Pointer, "Esc nach dem Radieren: Zeiger");
    assert_eq!(sel(s), vec![b]);
    s.undo().expect("undo");
    assert_eq!(px(s, b, 184, 366), [255, 0, 0, 255], "Strg+Z stellt her");
    // Strich 160 → 260 bei y 367 (PM-3): Spalte 185 voll gelöscht 361–371, 360/372 ≈ 81 %, 373 ≈ 13 %
    tool(s, ToolId::Eraser);
    drag(s, (160.0, 367.0), (260.0, 367.0));
    for y in 361..=371 {
        assert_eq!(px(s, b, 185, y)[3], 0, "185,{y} voll gelöscht");
    }
    near(px(s, b, 185, 360)[3], 49, 25, "185,360");
    near(px(s, b, 185, 372)[3], 49, 25, "185,372");
    near(px(s, b, 185, 373)[3], 223, 25, "185,373");
    assert_eq!(px(s, b, 185, 359)[3], 255);
    for x in 154..=264 {
        assert_eq!(px(s, b, x, 367)[3], 0, "{x},367 voll gelöscht");
    }
    assert!(px(s, b, 153, 367)[3] < 255 && px(s, b, 153, 367)[3] > 0, "153 Übergang");
    assert!(px(s, b, 265, 367)[3] < 255 && px(s, b, 266, 367)[3] < 255, "265/266 Übergang");
    assert_eq!(px(s, b, 152, 367)[3], 255);
}

#[test]
fn radiergummi_ohne_bitmap_hebt_auswahl_auf() {
    let mut t = app();
    let (r1, b) = (t.r1, t.b2);
    let s = ses(&mut t);
    select(s, r1);
    let doc0 = s.active().map(|d| d.doc.clone()).expect("Dokument");
    tool(s, ToolId::Eraser);
    drag(s, (60.0, 85.0), (140.0, 85.0));
    assert!(sel(s).is_empty(), "Original T4: Auswahl aufgehoben");
    assert_eq!(s.active().map(|d| d.doc.clone()).expect("Dokument"), doc0, "R1 unverändert");
    esc(s);
    assert_eq!(s.tools.active, ToolId::Eraser, "Esc ohne Bitmap: Werkzeug bleibt");
}

// ------------------------------------------------------------------ L38 Weichzeichner

#[test]
fn weichzeichner_staerke_und_wachsen_wie_im_original() {
    let mut t = app();
    let (r1, b) = (t.r1, t.b2);
    let s = ses(&mut t);
    select(s, b);
    tool(s, ToolId::Blur);
    click(s, 210.0, 367.0);
    // Einzelabdruck Intensität 50: an der Farbkante je 15/255 (Original M7)
    near(px(s, b, 209, 366)[2], 15, 3, "209 Blau");
    near(px(s, b, 210, 366)[0], 15, 3, "210 Rot");
    near(px(s, b, 209, 372)[2], 1, 2, "209,372");
    assert_eq!(px(s, b, 208, 366), [255, 0, 0, 255], "208 unberührt");
    s.undo().expect("undo");
    // Strich x = 210, 345 → 390 (PM-3): 208 FC0002, 209 D60028, 210 2800D6, 211 0200FC; Bitmap wächst nach oben
    drag(s, (210.0, 345.0), (210.0, 390.0));
    near(px(s, b, 209, 367)[2], 0x28, 12, "209 Blau");
    near(px(s, b, 210, 367)[0], 0x28, 12, "210 Rot");
    near(px(s, b, 208, 367)[2], 2, 6, "208 Blau");
    near(px(s, b, 211, 367)[0], 2, 6, "211 Rot");
    let r = image(s, b).bounds();
    assert_eq!((r.left, r.top, r.right, r.bottom), (150, 339, 270, 395), "Original 150,339–270,395");
    s.undo().expect("undo");
    assert_eq!(image(s, b).bounds(), IRect::new(150, 340, 270, 395));
    // Klick an der Oberkante: Bitmap wächst um 1 px (Original M7b), Kante hellt auf (30/255)
    click(s, 180.0, 341.0);
    assert_eq!(image(s, b).bounds().top, 339);
    near(255 - px(s, b, 179, 340)[3], 20, 12, "Deckkraftverlust 179,340");
    esc(s);
    assert_eq!(s.tools.active, ToolId::Pointer);
    assert_eq!(sel(s), vec![b]);
}

// ------------------------------------------------------------------ L39 Stempel

#[test]
fn stempel_kante_und_esc_wie_im_original() {
    let mut t = app();
    let (r1, b) = (t.r1, t.b2);
    let s = ses(&mut t);
    select(s, b);
    tool(s, ToolId::RubberStamp);
    s.tool_event(&ToolEvent::down(180.0, 367.0).alt()).expect("Quelle");
    s.tool_event(&ToolEvent::up(180.0, 367.0).alt()).expect("Quelle");
    click(s, 240.0, 367.0);
    // Einzelabdruck 16 / Rand 50 (Original M8): 232 ≈ 20, 233 ≈ 61, 234 ≈ 158, ab 235 voll rot
    near(px(s, b, 232, 366)[0], 20, 10, "232");
    near(px(s, b, 233, 366)[0], 61, 15, "233");
    near(px(s, b, 234, 366)[0], 158, 20, "234");
    assert_eq!(px(s, b, 235, 366)[0], 255);
    assert_eq!(px(s, b, 231, 366), [0, 0, 255, 255]);
    s.undo().expect("undo");
    // Strich 240,350 → 240,385 (PM-3): Kern 235–244, Übergang 232–234 (40, A1, F5)
    drag(s, (240.0, 350.0), (240.0, 385.0));
    for x in 235..=244 {
        assert!(px(s, b, x, 367)[0] >= 240, "{x} Kern rot: {:?}", px(s, b, x, 367));
    }
    near(px(s, b, 234, 367)[0], 0xF5, 25, "234");
    near(px(s, b, 233, 367)[0], 0xA1, 35, "233");
    near(px(s, b, 232, 367)[0], 0x40, 30, "232");
    near(px(s, b, 245, 367)[0], 0xF5, 25, "245");
    assert_eq!(px(s, b, 231, 367), [0, 0, 255, 255]);
    esc(s);
    assert_eq!(s.tools.active, ToolId::Pointer, "Esc beendet den Bitmapmodus: Zeiger (Original)");
    assert_eq!(sel(s), vec![b], "B2 bleibt gewählt");
}

#[test]
fn esc_in_malwerkzeugen_mit_und_ohne_bitmap() {
    for t in [ToolId::Brush, ToolId::Pencil, ToolId::Eraser, ToolId::Blur, ToolId::Sharpen, ToolId::Dodge, ToolId::Burn, ToolId::Smudge, ToolId::RubberStamp, ToolId::ReplaceColor, ToolId::RedEyeRemoval] {
        let mut tt = app();
        let (r1, b) = (tt.r1, tt.b2);
        let s = ses(&mut tt);
        tool(s, t);
        esc(s);
        assert_eq!(s.tools.active, t, "{t:?} ohne Auswahl: Werkzeug bleibt");
            select(s, r1);
        esc(s);
        assert_eq!(s.tools.active, t, "{t:?} mit Vektor: Werkzeug bleibt");
            select(s, b);
        let h0 = undo_count(s);
        esc(s);
        assert_eq!(s.tools.active, ToolId::Pointer, "{t:?} mit Bitmap: Zeiger");
        assert_eq!(sel(s), vec![b], "{t:?}: Bitmap bleibt gewählt");
        assert_eq!(undo_count(s), h0, "{t:?}: Esc ohne Bitmapmodus ist kein Verlaufsschritt");
    }
}

// ------------------------------------------------------------------ Auswahlrahmen beim Malen

/// Das Original zeigt um die gewählte Bitmap keinen Rahmen und keine Griffe, solange ein
/// Malwerkzeug aktiv ist (Paarbilder fv11_*); mit dem Zeiger erscheinen sie wieder.
#[test]
fn malwerkzeug_zeigt_keinen_auswahlrahmen_um_die_bitmap() {
    let t = app();
    let (r1, b) = (t.r1, t.b2);
    let mut h = egui_kittest::Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .with_pixels_per_point(1.0)
        .with_max_steps(16)
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), t.a);
    let frames = |h: &egui_kittest::Harness<'_, FirepixelApp>| h.state().state.canvases.values().next().map(|c| c.sel_frames.len()).unwrap_or(0);
    h.state_mut().state.session.execute("select.set", json!({"ids": [b.0]})).expect("Auswahl");
    h.run_steps(2);
    assert_eq!(frames(&h), 1, "Zeiger: Rahmen um B2");
    for tl in [ToolId::Brush, ToolId::Eraser, ToolId::Blur, ToolId::RubberStamp] {
        h.state_mut().state.session.set_tool(tl).expect("Werkzeug");
        h.run_steps(2);
        assert_eq!(frames(&h), 0, "{tl:?}: kein Rahmen um die Bitmap");
    }
    // Vektor bleibt mit Rahmen (im Original nicht gemessen, Verhalten unverändert)
    h.state_mut().state.session.execute("select.set", json!({"ids": [r1.0]})).expect("Auswahl");
    h.run_steps(2);
    assert_eq!(frames(&h), 1);
    h.state_mut().state.session.set_tool(ToolId::Pointer).expect("Werkzeug");
    h.state_mut().state.session.execute("select.set", json!({"ids": [b.0]})).expect("Auswahl");
    h.run_steps(2);
    assert_eq!(frames(&h), 1, "zurück beim Zeiger: Rahmen wieder da");
}
