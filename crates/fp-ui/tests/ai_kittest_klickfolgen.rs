//! KI-Zauberstäbe bedienen sich wie der klassische Zauberstab (Agent KI, Nutzerbefund):
//! realistische Klickfolgen mit echten Maus-Ereignissen (step_dt 1/60) an verschiedenen Stellen,
//! mit und ohne Umschalt/Alt, Klicks während der Berechnung, Esc, Eingabe und Werkzeugwechsel.
//! Nach jedem Klick wird die Auswahlmaske geprüft. Braucht die Modelle (sonst übersprungen).

use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use fp_engine::commands::ai::fp_ai;
use fp_engine::ToolId;
use fp_ui::app_state::UiSettings;
use fp_ui::FirepixelApp;
use serde_json::json;
use std::time::{Duration, Instant};

type H = Harness<'static, FirepixelApp>;

/// Zwei Scheiben auf unruhigem Grund: A (rot, Mitte 110/150) und B (blau, Mitte 290/150), Radius 50.
const A: (f32, f32) = (110.0, 150.0);
const B: (f32, f32) = (290.0, 150.0);
const R: f32 = 50.0;

fn have(id: fp_ai::models::ModelId) -> bool {
    fp_ai::models::available(id)
}

fn test_bitmap(seed: i32) -> fp_raster::RgbaImage {
    let mut img = fp_raster::RgbaImage::new(fp_geom::IRect::from_xywh(0, 0, 400, 300)).unwrap_or_else(|_| fp_raster::RgbaImage::empty_at(0, 0));
    let w = img.width();
    for (i, px) in img.data_mut().iter_mut().enumerate() {
        let (x, y) = ((i % w) as i32, (i / w) as i32);
        let n = ((x * 7919 + y * 104_729 + seed * 31) ^ (x * y)) & 63;
        let stripe = if (x / 9 + y / 13) % 2 == 0 { 30 } else { 0 };
        let mut c = [60 + n as u8 / 2 + stripe, 120 + n as u8, 50 + stripe, 255];
        let inside = |c: (f32, f32)| {
            let (dx, dy) = (x as f32 - c.0, y as f32 - c.1);
            dx * dx + dy * dy <= R * R
        };
        if inside(A) {
            c = [220, 30 + (n as u8 / 4), 40, 255];
        } else if inside(B) {
            c = [40, 50 + (n as u8 / 4), 220, 255];
        }
        *px = c;
    }
    img
}

fn harness(seed: i32) -> H {
    fp_engine::shell::disable();
    fp_ai::download::disable();
    let mut s = fp_engine::Session::default();
    let _ = s.execute("file.new", json!({"width": 400, "height": 300, "background": "#ffffff"}));
    s.clipboard.pixels = Some(test_bitmap(seed));
    let _ = s.execute("edit.paste", json!({}));
    let _ = s.execute("select.none", json!({}));
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    // asynchron wie in der App
    app.state.sync_render = false;
    let mut h = Harness::builder().with_size(egui::vec2(1500.0, 950.0)).with_pixels_per_point(1.0).with_max_steps(8).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    h
}

fn at(h: &H, p: (f32, f32)) -> Pos2 {
    let st = &h.state().state;
    let d = st.session.active().expect("Dokument");
    let area = st.canvases.get(&d.id).and_then(|c| c.area).expect("Leinwand");
    let (sx, sy) = d.view.doc_to_screen(fp_geom::Point::new(p.0, p.1));
    Pos2::new(area.min.x + sx as f32, area.min.y + sy as f32)
}

/// Ein Mausklick wie von Hand: hinfahren, drücken, ein paar Bilder halten, loslassen.
fn click(h: &mut H, p: (f32, f32), m: Modifiers) {
    let q = at(h, p);
    let ev = |h: &mut H, e: Option<Event>| {
        if let Some(e) = e {
            h.input_mut().events.push(e);
        }
        h.step();
    };
    h.input_mut().events.push(Event::ModifiersChanged(m));
    ev(h, Some(Event::PointerMoved(q)));
    ev(h, Some(Event::PointerButton { pos: q, button: PointerButton::Primary, pressed: true, modifiers: m }));
    ev(h, None);
    ev(h, Some(Event::PointerButton { pos: q, button: PointerButton::Primary, pressed: false, modifiers: m }));
    h.input_mut().events.push(Event::ModifiersChanged(Modifiers::NONE));
    h.step();
    // weg vom Klickpunkt (kein Doppelklick beim nächsten Klick an derselben Stelle)
    std::thread::sleep(Duration::from_millis(5));
}

fn key(h: &mut H, k: Key) {
    for pressed in [true, false] {
        h.input_mut().events.push(Event::Key { key: k, physical_key: Some(k), pressed, repeat: false, modifiers: Modifiers::NONE });
        h.step();
    }
}

fn busy(h: &H) -> bool {
    h.state().state.dialogs.ai.busy()
}

fn wait(h: &mut H) {
    let t = Instant::now();
    while busy(h) && t.elapsed() < Duration::from_secs(120) {
        std::thread::sleep(Duration::from_millis(20));
        h.step();
    }
    assert!(!busy(h), "Rechnung hängt");
    h.run_steps(2);
}

/// Auswahl im Dokument (Deckung 0…255) als Funktion.
fn sel(h: &H) -> Option<fp_raster::Mask> {
    h.state().state.session.active().and_then(|d| d.selection.pixels.clone()).filter(|m| !m.tight_bounds().is_empty())
}

/// Anteil der Scheibe `c`, der ausgewählt ist, und Anteil der Auswahl außerhalb der beiden Scheiben.
fn coverage(h: &H, c: (f32, f32)) -> f32 {
    let Some(m) = sel(h) else { return 0.0 };
    let (mut n, mut hit) = (0, 0);
    for y in (c.1 - R) as i32..=(c.1 + R) as i32 {
        for x in (c.0 - R) as i32..=(c.0 + R) as i32 {
            let (dx, dy) = (x as f32 + 0.5 - c.0, y as f32 + 0.5 - c.1);
            if dx * dx + dy * dy <= (R - 3.0) * (R - 3.0) {
                n += 1;
                hit += (m.get(x, y) > 127) as i32;
            }
        }
    }
    hit as f32 / n.max(1) as f32
}

fn outside(h: &H) -> f32 {
    let Some(m) = sel(h) else { return 0.0 };
    let b = m.tight_bounds();
    let (mut n, mut out) = (0, 0);
    for y in b.top..b.bottom {
        for x in b.left..b.right {
            if m.get(x, y) > 127 {
                n += 1;
                let d = |c: (f32, f32)| ((x as f32 + 0.5 - c.0).powi(2) + (y as f32 + 0.5 - c.1).powi(2)).sqrt();
                out += (d(A) > R + 4.0 && d(B) > R + 4.0) as i32;
            }
        }
    }
    out as f32 / n.max(1) as f32
}

/// Erwartete Auswahl: welche Scheiben ausgewählt sind.
fn expect(h: &H, a: bool, b: bool, what: &str) {
    let (ca, cb, o) = (coverage(h, A), coverage(h, B), outside(h));
    eprintln!("{what}: A {ca:.3}, B {cb:.3}, außerhalb {o:.3}");
    assert!(if a { ca > 0.97 } else { ca < 0.02 }, "{what}: Scheibe A {ca:.3} (erwartet {a})");
    assert!(if b { cb > 0.97 } else { cb < 0.02 }, "{what}: Scheibe B {cb:.3} (erwartet {b})");
    assert!(o < 0.03, "{what}: Auswahl läuft aus ({o:.3})");
}

fn tool(h: &mut H, t: ToolId) {
    let _ = h.state_mut().state.session.set_tool(t);
    h.step();
}

const NONE: Modifiers = Modifiers::NONE;
const SHIFT: Modifiers = Modifiers::SHIFT;
const ALT: Modifiers = Modifiers::ALT;

fn shift_alt() -> Modifiers {
    Modifiers { shift: true, alt: true, ..Modifiers::NONE }
}

/// Was tut Esc beim klassischen Zauberstab (Auswahl aufheben oder nicht)?
fn wand_esc_clears(h: &mut H) -> bool {
    tool(h, ToolId::MagicWand);
    click(h, A, NONE);
    assert!(sel(h).is_some(), "Zauberstab wählt");
    key(h, Key::Escape);
    h.run_steps(2);
    sel(h).is_none()
}

#[test]
fn objektauswahl_wie_der_zauberstab() {
    if !have(fp_ai::models::ModelId::Sam2Tiny) {
        eprintln!("KI-Modelle fehlen – Test übersprungen");
        return;
    }
    let mut h = harness(11);
    let esc_clears = wand_esc_clears(&mut h);
    eprintln!("Zauberstab: Esc hebt die Auswahl auf = {esc_clears}");
    let _ = h.state_mut().state.session.execute("select.none", json!({}));
    tool(&mut h, ToolId::AiObject);
    // erster Klick: Rechnung im Hintergrund, danach A
    click(&mut h, A, NONE);
    wait(&mut h);
    expect(&h, true, false, "Klick A");
    // Klick woanders ersetzt (sofort, Zwischenspeicher warm)
    click(&mut h, B, NONE);
    assert!(!busy(&h), "kein Warten beim zweiten Klick");
    expect(&h, false, true, "Klick B ersetzt");
    click(&mut h, A, NONE);
    expect(&h, true, false, "Klick A ersetzt wieder");
    click(&mut h, B, SHIFT);
    expect(&h, true, true, "Umschalt+Klick B fügt hinzu");
    click(&mut h, A, ALT);
    expect(&h, false, true, "Alt+Klick A zieht ab");
    click(&mut h, A, SHIFT);
    expect(&h, true, true, "Umschalt+Klick A");
    click(&mut h, B, shift_alt());
    expect(&h, false, true, "Umschalt+Alt+Klick B = Schnittmenge");
    // derselbe Punkt zweimal schnell hintereinander (Doppelklick) bleibt eine gültige Auswahl
    click(&mut h, A, NONE);
    click(&mut h, A, NONE);
    expect(&h, true, false, "Doppelklick A");
    // Eingabe tut nichts, das Werkzeug bleibt klickbereit
    key(&mut h, Key::Enter);
    expect(&h, true, false, "nach Eingabe");
    click(&mut h, B, NONE);
    expect(&h, false, true, "Klick nach Eingabe");
    // Esc wie beim Zauberstab
    key(&mut h, Key::Escape);
    h.run_steps(2);
    assert_eq!(sel(&h).is_none(), esc_clears, "Esc wie beim Zauberstab (hebt auf: {esc_clears})");
    click(&mut h, A, NONE);
    expect(&h, true, false, "Klick nach Esc");
    // Werkzeugwechsel und zurück: sofort klickbereit
    tool(&mut h, ToolId::MagicWand);
    click(&mut h, B, NONE);
    assert!(coverage(&h, B) > 0.9 && coverage(&h, A) < 0.02, "Zauberstab nach KI-Werkzeug");
    tool(&mut h, ToolId::AiObject);
    click(&mut h, A, SHIFT);
    expect(&h, true, true, "Umschalt+Klick auf Zauberstab-Auswahl");
    // Rahmen aufziehen (Zusatz), danach wieder Klicks
    let (p0, p1) = (at(&h, (A.0 - 65.0, A.1 - 65.0)), at(&h, (A.0 + 65.0, A.1 + 65.0)));
    for e in [
        Event::PointerMoved(p0),
        Event::PointerButton { pos: p0, button: PointerButton::Primary, pressed: true, modifiers: NONE },
        Event::PointerMoved(Pos2::new((p0.x + p1.x) / 2.0, (p0.y + p1.y) / 2.0)),
        Event::PointerMoved(p1),
        Event::PointerButton { pos: p1, button: PointerButton::Primary, pressed: false, modifiers: NONE },
    ] {
        h.input_mut().events.push(e);
        h.step();
    }
    h.run_steps(2);
    expect(&h, true, false, "Rahmen um A");
    click(&mut h, B, NONE);
    expect(&h, false, true, "Klick nach dem Rahmen");
}

#[test]
fn klicks_waehrend_der_berechnung() {
    if !have(fp_ai::models::ModelId::Sam2Tiny) || !have(fp_ai::models::ModelId::BiRefNetLite) {
        eprintln!("KI-Modelle fehlen – Test übersprungen");
        return;
    }
    // einfacher Klick während der Rechnung: der neue Klick gilt (ersetzt), nichts wird verschluckt
    let mut h = harness(21);
    tool(&mut h, ToolId::AiObject);
    click(&mut h, A, NONE);
    assert!(busy(&h), "erster Klick rechnet im Hintergrund");
    click(&mut h, B, NONE);
    assert!(busy(&h));
    wait(&mut h);
    expect(&h, false, true, "Klick B während der Rechnung ersetzt A");
    // gleich danach wieder klickbereit
    click(&mut h, A, NONE);
    assert!(!busy(&h));
    expect(&h, true, false, "Klick nach der Rechnung");

    // Umschalt-/Alt-Klicks während der Rechnung werden der Reihe nach angewandt
    let mut h = harness(22);
    tool(&mut h, ToolId::AiObject);
    click(&mut h, A, NONE);
    assert!(busy(&h));
    click(&mut h, B, SHIFT);
    wait(&mut h);
    expect(&h, true, true, "Umschalt+Klick während der Rechnung");
    let mut h = harness(23);
    tool(&mut h, ToolId::AiObject);
    click(&mut h, A, NONE);
    click(&mut h, B, SHIFT);
    click(&mut h, A, ALT);
    wait(&mut h);
    expect(&h, false, true, "Klick, Umschalt, Alt während der Rechnung");

    // anderes Werkzeug (andere Rechnung) während der Rechnung: alte abbrechen, neue starten
    let mut h = harness(24);
    tool(&mut h, ToolId::AiSubject);
    click(&mut h, (200.0, 20.0), NONE);
    assert!(busy(&h), "Motiv rechnet");
    let t = Instant::now();
    tool(&mut h, ToolId::AiObject);
    click(&mut h, A, NONE);
    assert!(busy(&h), "Objektauswahl rechnet");
    wait(&mut h);
    eprintln!("Objektauswahl nach Abbruch des Motivs: {:.1} s", t.elapsed().as_secs_f32());
    expect(&h, true, false, "Objektauswahl statt Motiv");
    let st = &h.state().state;
    assert!(st.dialogs.message.is_none(), "keine Fehlermeldung: {:?}", st.dialogs.message);
}

#[test]
fn motiv_und_kante_wie_der_zauberstab() {
    if !have(fp_ai::models::ModelId::BiRefNetLite) || !have(fp_ai::models::ModelId::BiRefNetLiteMatting) || !have(fp_ai::models::ModelId::Sam2Tiny) {
        eprintln!("KI-Modelle fehlen – Test übersprungen");
        return;
    }
    let mut h = harness(31);
    let esc_clears = wand_esc_clears(&mut h);
    let _ = h.state_mut().state.session.execute("select.none", json!({}));
    // Motiv auswählen
    tool(&mut h, ToolId::AiSubject);
    click(&mut h, (200.0, 20.0), NONE);
    wait(&mut h);
    let m1 = sel(&h).map(|m| m.tight_bounds());
    assert!(m1.is_some(), "Motiv ausgewählt");
    click(&mut h, (390.0, 290.0), NONE);
    assert!(!busy(&h), "zweiter Klick sofort");
    assert_eq!(sel(&h).map(|m| m.tight_bounds()), m1, "Klick an anderer Stelle: wieder das Motiv (ersetzt)");
    click(&mut h, (20.0, 20.0), ALT);
    assert!(sel(&h).is_none(), "Alt+Klick zieht das Motiv ab");
    click(&mut h, (20.0, 20.0), SHIFT);
    assert_eq!(sel(&h).map(|m| m.tight_bounds()), m1, "Umschalt+Klick fügt hinzu");
    click(&mut h, (20.0, 20.0), shift_alt());
    assert_eq!(sel(&h).map(|m| m.tight_bounds()), m1, "Schnittmenge mit sich selbst");
    key(&mut h, Key::Escape);
    h.run_steps(2);
    assert_eq!(sel(&h).is_none(), esc_clears, "Esc wie beim Zauberstab");
    // Kante verfeinern auf einer Objektauswahl
    tool(&mut h, ToolId::AiObject);
    click(&mut h, A, NONE);
    wait(&mut h);
    expect(&h, true, false, "Objekt A");
    tool(&mut h, ToolId::AiRefineEdge);
    click(&mut h, (380.0, 280.0), NONE);
    wait(&mut h);
    expect(&h, true, false, "Kante verfeinert");
    // gleich danach: Werkzeugwechsel und neuer Klick
    tool(&mut h, ToolId::AiObject);
    click(&mut h, B, NONE);
    wait(&mut h);
    expect(&h, false, true, "Objekt B nach Kante verfeinern");
    key(&mut h, Key::Enter);
    expect(&h, false, true, "Eingabe ändert nichts");
}
