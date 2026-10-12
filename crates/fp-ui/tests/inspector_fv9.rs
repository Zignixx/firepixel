//! Funktionsvergleich mit dem Original (FV-9, Befunde EB-1/PM-2 im Abschnitt IBD): Strg+A im
//! Zahlenfeld markiert den Feldtext (F10), Esc + Klick auf die Leinwand übernimmt (F7), Glättung
//! im Text-Inspektor (O6), Textwerkzeug-Vorgaben Glättung/Unterschneidung/Richtung, Textgrenzen
//! beim Bearbeiten mit 2-px-Rand, Schriftschnitt „Regular“ (W14), zweizeilige Werkzeugnamen
//! (W10/W27/W28), Kurzzeichen F/K (W16). Realistische Eingaben wie egui-winit unter Windows (je
//! Ereignis ein Bild, Tippen als `Event::Text`, Tastendruck und Loslassen getrennt).

use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::{By, NodeT, Queryable};
use egui_kittest::Harness;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::FirepixelApp;
use serde_json::json;
use std::sync::Arc;

type H = Harness<'static, FirepixelApp>;

const W: f32 = 1800.0;
const HH: f32 = 1130.0;

fn harness(setup: impl FnOnce(&mut fp_engine::Session)) -> H {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 500, "height": 300, "background": "#ffffff"})).expect("Dokument");
    setup(&mut s);
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(W, HH)).with_pixels_per_point(1.0).with_max_steps(20).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    h
}

fn rect(s: &mut fp_engine::Session, r: [i32; 4], c: &str) -> u64 {
    let v = s.execute("draw.rectangle", json!({"rect": r, "fillColor": c, "strokeWidth": 0})).expect("Rechteck");
    v.value.get("id").and_then(|x| x.as_u64()).expect("id")
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn ev(h: &mut H, e: Event) {
    h.event(e);
    h.run_steps(1);
}

fn key(h: &mut H, k: Key, m: Modifiers) {
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: m });
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: false, repeat: false, modifiers: m });
}

fn ctrl() -> Modifiers {
    Modifiers { ctrl: true, command: true, ..Default::default() }
}

fn text(h: &mut H, t: &str) {
    for c in t.chars() {
        ev(h, Event::Text(c.to_string()));
    }
}

fn press(h: &mut H, p: Pos2, down: bool) {
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: down, modifiers: Modifiers::NONE });
}

fn click(h: &mut H, p: Pos2) {
    ev(h, Event::PointerMoved(p));
    press(h, p, true);
    press(h, p, false);
    h.run_steps(2);
}

/// Klick wie ein schneller SendInput-Klick: Bewegen, Drücken und Loslassen in **einem** Bild.
fn fast_click(h: &mut H, p: Pos2) {
    h.event(Event::PointerMoved(p));
    h.event(Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.event(Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(3);
}

fn screen(h: &H, x: f32, y: f32) -> Pos2 {
    let s = st(h);
    let d = s.session.active().expect("Dokument");
    let area = s.canvases.get(&d.id).and_then(|c| c.area).expect("Leinwand");
    let (sx, sy) = d.view.doc_to_screen(fp_geom::Point::new(x, y));
    Pos2::new(area.min.x + sx as f32, area.min.y + sy as f32)
}

fn sel(h: &H) -> Vec<u64> {
    st(h).session.active().map(|d| d.selection.elements.iter().map(|e| e.0).collect()).unwrap_or_default()
}

fn undo_count(h: &H) -> usize {
    st(h).session.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

fn element(h: &H, id: u64) -> Arc<fp_model::Element> {
    let d = st(h).session.active().expect("Dokument");
    fp_engine::docops::element_on_page(&d.doc, d.page, fp_model::ElementId(id)).expect("Element")
}

fn opacity(h: &H, id: u64) -> f64 {
    element(h, id).common.opacity.to_percent().round()
}

/// Eingabefelder im Inspektor (unterer Fensterteil): Rechteck und angezeigter Text.
fn inputs(h: &H) -> Vec<(egui::Rect, String)> {
    let mut v: Vec<(egui::Rect, String)> = h.get_all(By::new().role(egui::accesskit::Role::TextInput)).map(|n| (n.rect(), n.value().unwrap_or_default())).filter(|(r, _)| r.top() > HH - 160.0).collect();
    v.sort_by(|a, b| a.0.top().total_cmp(&b.0.top()).then(a.0.left().total_cmp(&b.0.left())));
    v
}

fn input_with(h: &H, value: &str) -> egui::Rect {
    let all = inputs(h);
    let m: Vec<_> = all.iter().filter(|(_, v)| v == value).collect();
    assert_eq!(m.len(), 1, "genau ein Feld mit „{value}“: {all:?}");
    m[0].0
}

/// Feld anklicken, Inhalt wie im Messprotokoll ersetzen (Ende, Umschalt+Pos1, Tippen).
fn type_into(h: &mut H, field: egui::Rect, s: &str) {
    click(h, field.center());
    key(h, Key::End, Modifiers::NONE);
    key(h, Key::Home, Modifiers::SHIFT);
    text(h, s);
}

/// Zwei Rechtecke (R1 rot ausgewählt, R2 blau).
fn two_rects() -> (H, u64, u64) {
    let mut ids = (0, 0);
    let h = harness(|s| {
        ids.0 = rect(s, [20, 20, 140, 90], "#ff0000");
        ids.1 = rect(s, [200, 40, 290, 110], "#0000ff");
        s.execute("select.set", json!({"ids": [ids.0]})).expect("Auswahl");
    });
    (h, ids.0, ids.1)
}

// ------------------------------------------------------------------ F10: Strg+A im Zahlenfeld

/// Wie das Messprotokoll PM-2 (`pm2\f2.ps1`): Klick ins Feld + Strg+A markiert den Feldtext,
/// Auswahl bleibt; „55“ tippen + Strg+A markiert „55“, Eingabe übernimmt 55 (ein Schritt).
#[test]
fn f10_ctrl_a_selects_field_text() {
    let (mut h, r1, _) = two_rects();
    let n0 = undo_count(&h);
    let f = input_with(&h, "100");
    click(&mut h, f.center());
    key(&mut h, Key::A, ctrl());
    h.run_steps(3);
    assert_eq!(sel(&h), vec![r1], "Strg+A wählt keine Objekte aus");
    assert_eq!(undo_count(&h), n0, "kein Verlaufsschritt");
    // markierter Text wird durch Tippen ersetzt
    text(&mut h, "8");
    key(&mut h, Key::Enter, Modifiers::NONE);
    assert_eq!(opacity(&h, r1), 8.0, "markierter Feldtext ersetzt");
    assert_eq!(undo_count(&h), n0 + 1);
    // „55“ tippen, Strg+A, Eingabe → 55
    let f = input_with(&h, "8");
    type_into(&mut h, f, "55");
    key(&mut h, Key::A, ctrl());
    h.run_steps(2);
    assert_eq!(sel(&h), vec![r1]);
    assert_eq!(opacity(&h, r1), 8.0, "noch nicht übernommen");
    key(&mut h, Key::Enter, Modifiers::NONE);
    assert_eq!(opacity(&h, r1), 55.0, "getippter Wert bleibt und wird übernommen");
    assert_eq!(undo_count(&h), n0 + 2);
    // §4b: Strg+A 20× (Tastenwiederholung) – weiter keine Objektauswahl, kein Schritt
    let f = input_with(&h, "55");
    click(&mut h, f.center());
    for _ in 0..20 {
        ev(&mut h, Event::Key { key: Key::A, physical_key: Some(Key::A), pressed: true, repeat: true, modifiers: ctrl() });
    }
    ev(&mut h, Event::Key { key: Key::A, physical_key: Some(Key::A), pressed: false, repeat: false, modifiers: ctrl() });
    assert_eq!(sel(&h), vec![r1]);
    assert_eq!(undo_count(&h), n0 + 2);
    // ohne Fokus im Feld bleibt Strg+A „Alles auswählen“
    let p = screen(&h, 450.0, 260.0);
    click(&mut h, p);
    key(&mut h, Key::A, ctrl());
    h.run_steps(2);
    assert_eq!(sel(&h).len(), 2, "Strg+A auf der Leinwand wählt alles");
    // Rückgängig stellt her
    while undo_count(&h) > n0 {
        h.state_mut().state.session.undo().expect("undo");
    }
    assert_eq!(opacity(&h, r1), 100.0);
}

// ------------------------------------------------------------------ F7: Esc, dann Klick daneben

/// „7“ tippen, Esc, Klick auf die leere Leinwand: Original übernimmt 7 (Esc verwirft nicht, der
/// Fokusverlust übernimmt). Wie im Messprotokoll mit Pausen nach Esc, schnellem Klick (Drücken
/// und Loslassen in einem Bild) und langsamem Klick, im Dokument und daneben (grau).
#[test]
fn f7_escape_then_canvas_click_commits() {
    for (variant, fast, x, y) in [("langsam im Dokument", false, 450.0, 260.0), ("schnell im Dokument", true, 450.0, 260.0), ("schnell daneben", true, 560.0, 330.0), ("langsam daneben", false, 560.0, 330.0)] {
        let (mut h, r1, _) = two_rects();
        let n0 = undo_count(&h);
        let f = input_with(&h, "100");
        type_into(&mut h, f, "7");
        key(&mut h, Key::Escape, Modifiers::NONE);
        h.run_steps(6);
        assert_eq!(sel(&h), vec![r1], "{variant}: Esc im Feld hebt die Auswahl nicht auf");
        assert_eq!(opacity(&h, r1), 100.0, "{variant}: noch nicht übernommen");
        let p = screen(&h, x, y);
        if fast {
            fast_click(&mut h, p);
        } else {
            click(&mut h, p);
        }
        assert_eq!(opacity(&h, r1), 7.0, "{variant}: Esc + Klick auf die Leinwand übernimmt 7");
        assert_eq!(undo_count(&h), n0 + 1, "{variant}: ein Schritt");
    }
}

// ------------------------------------------------------------------ O6: Glättung im Text-Inspektor

fn text_elem(h: &H, id: u64) -> fp_model::TextElem {
    let e = element(h, id);
    let fp_model::ElementKind::Text(t) = &e.kind else { panic!("Text") };
    (**t).clone()
}

fn hard_edge(t: &fp_model::TextElem) -> bool {
    t.attrs.fill.as_ref().is_some_and(|f| f.edge == 0)
}

/// Kombifelder im Inspektor (unterer Fensterteil): Rechteck, Wert, bedienbar.
fn combos(h: &H) -> Vec<(egui::Rect, String, bool)> {
    h.get_all(By::new().role(egui::accesskit::Role::ComboBox))
        .map(|n| (n.rect(), n.value().unwrap_or_default(), !n.accesskit_node().is_disabled()))
        .filter(|(r, _, _)| r.top() > HH - 160.0)
        .collect()
}

fn combo_with(h: &H, value: &str) -> (egui::Rect, bool) {
    let all = combos(h);
    let m: Vec<_> = all.iter().filter(|(_, v, _)| v == value).collect();
    assert_eq!(m.len(), 1, "genau ein Kombifeld mit „{value}“: {all:?}");
    (m[0].0, m[0].2)
}

/// Kombifeld öffnen und Eintrag wählen.
fn choose(h: &mut H, combo: egui::Rect, item: &str) {
    click(h, combo.center());
    let r = h.get_by_label(item).rect();
    click(h, r.center());
}

fn aa_label(name: &str) -> String {
    fp_i18n::t(&format!("pi-aa-{name}"))
}

fn doc_texts(h: &H) -> Vec<fp_model::TextElem> {
    let d = st(h).session.active().expect("d");
    fp_engine::docops::selectable_ids(&d.doc, d.page, d.frame)
        .into_iter()
        .filter_map(|id| fp_engine::docops::element_on_page(&d.doc, d.page, id).and_then(|e| if let fp_model::ElementKind::Text(t) = &e.kind { Some((**t).clone()) } else { None }))
        .collect()
}

/// Ein per Skript angelegter Text ist glatt (`TAAi` 0, weiche Kante) und zeigt „Anti-Alias glatt“
/// (vorher „Kein Anti-Alias“). Wählen schreibt wie `text.setProperties {antialias}`: „Kein
/// Anti-Alias“ = harte Füllkante, „glatt“ = `TAAi` 0; je Wahl ein Schritt, Rückgängig stellt her.
#[test]
fn o6_text_antialias_list_maps_like_set_properties() {
    let mut id = 0;
    let mut h = harness(|s| {
        let v = s.execute("draw.text", json!({"text": "Hallo", "anchor": [30, 60], "size": 24})).expect("Text");
        id = v.value.get("id").and_then(|x| x.as_u64()).unwrap_or(0);
    });
    h.run_steps(2);
    let t0 = text_elem(&h, id);
    assert_eq!((t0.antialias, hard_edge(&t0)), (0, false), "neuer Text glatt");
    let (c, en) = combo_with(&h, &aa_label("smooth"));
    assert!(en);
    let d0 = Arc::clone(&st(&h).session.active().expect("d").doc);
    let n0 = undo_count(&h);
    choose(&mut h, c, &aa_label("crisp"));
    let t = text_elem(&h, id);
    assert_eq!((t.antialias, hard_edge(&t)), (1, false), "scharf = TAAi 1");
    assert_eq!(combo_with(&h, &aa_label("crisp")).0, c);
    choose(&mut h, c, &aa_label("none"));
    let t = text_elem(&h, id);
    assert!(hard_edge(&t), "Kein Anti-Alias = harte Füllkante");
    assert_eq!(combo_with(&h, &aa_label("none")).0, c);
    choose(&mut h, c, &aa_label("strong"));
    let t = text_elem(&h, id);
    assert_eq!((t.antialias, hard_edge(&t)), (2, false), "stark = TAAi 2, weiche Kante");
    choose(&mut h, c, &aa_label("smooth"));
    let t = text_elem(&h, id);
    assert_eq!((t.antialias, hard_edge(&t)), (0, false), "glatt = TAAi 0");
    assert_eq!(undo_count(&h), n0 + 4, "je Wahl ein Schritt");
    // System (3) zeigt das Original als „scharf“
    h.state_mut().state.session.execute("text.setProperties", json!({"antialias": "system"})).expect("system");
    h.run_steps(2);
    combo_with(&h, &aa_label("crisp"));
    // §4b: 12× wählen → Endwert, keine Duplikate, alles rückgängig
    // (12 Wahlen: zusammen mit den 5 Schritten oben unter der Verlaufsgrenze von 20 Schritten)
    for k in 0..12 {
        let name = ["crisp", "none", "strong", "smooth"][k % 4];
        choose(&mut h, c, &aa_label(name));
    }
    let t = text_elem(&h, id);
    assert_eq!((t.antialias, hard_edge(&t)), (0, false));
    assert_eq!(doc_texts(&h).len(), 1, "keine Duplikate");
    while undo_count(&h) > n0 {
        h.state_mut().state.session.undo().expect("undo");
    }
    let t = text_elem(&h, id);
    assert_eq!((t.antialias, hard_edge(&t), t.auto_kern), (t0.antialias, hard_edge(&t0), t0.auto_kern));
    let (a, b) = (format!("{:?}", st(&h).session.active().expect("d").doc), format!("{d0:?}"));
    if a != b {
        let i = a.chars().zip(b.chars()).position(|(x, y)| x != y).unwrap_or(0);
        panic!("Rückgängig stellt nicht her, ab {i}: {} | {}", &a[i.saturating_sub(200)..(i + 200).min(a.len())], &b[i.saturating_sub(200)..(i + 200).min(b.len())]);
    }
}

// ------------------------------------------------------------------ Textwerkzeug-Vorgaben

/// Knopf „ab/cd“ über das Layout-Protokoll (Lage wie in inspector_paritaet::w17).
fn orientation_button(h: &mut H) -> Pos2 {
    fp_ui::inspector::fields::set_audit(&h.ctx, true);
    h.run_steps(1);
    let log = fp_ui::inspector::fields::take_audit(&h.ctx);
    fp_ui::inspector::fields::set_audit(&h.ctx, false);
    let small: Vec<egui::Rect> = log.iter().filter(|e| e.kind == "field" && (e.avail.width() - 20.0).abs() < 0.6 && (e.avail.height() - 20.0).abs() < 0.6).map(|e| e.avail).collect();
    assert!(small.len() >= 7, "F/K/U + 4 Ausrichtungen: {small:?}");
    let u = small[2];
    Pos2::new(u.right() + (184.0 - 168.0 - 13.0) * 1.5 + 13.0 * 0.75, u.center().y)
}

/// Textwerkzeug ohne Auswahl: Glättung, Autom. Unterschneidung und Textausrichtung zeigen die
/// Vorgaben aus den Voreinstellungen (`tool_text_defaults`), sind bedienbar und setzen sie mit
/// `text.setToolDefaults` (kein Verlaufsschritt); der nächste getippte Text bekommt sie.
#[test]
fn text_tool_defaults_antialias_kern_orientation() {
    let mut h = harness(|_| {});
    let _ = h.state_mut().state.session.set_tool(fp_engine::ToolId::Text);
    h.run_steps(3);
    assert_eq!(fp_engine::commands::tool_text_defaults(&st(&h).session.prefs), (0, false, 0));
    let (c, en) = combo_with(&h, &aa_label("smooth"));
    assert!(en, "Glättung im Textwerkzeug bedienbar");
    let n0 = undo_count(&h);
    choose(&mut h, c, &aa_label("strong"));
    assert_eq!(fp_engine::commands::tool_text_defaults(&st(&h).session.prefs).0, 2);
    combo_with(&h, &aa_label("strong"));
    // Autom. Unterschneidung
    let ak = fp_i18n::t("pi-text-autokern");
    assert!(!h.get_by_label(&ak).accesskit_node().is_disabled(), "Unterschneidung bedienbar");
    let r = h.get_by_label(&ak).rect();
    click(&mut h, Pos2::new(r.left() + 6.0, r.center().y));
    assert!(fp_engine::commands::tool_text_defaults(&st(&h).session.prefs).1, "Unterschneidung ein");
    // Textausrichtung
    let orient = orientation_button(&mut h);
    click(&mut h, orient);
    let item = h.get_by_label(&fp_i18n::t("pi-text-orient-vrl")).rect();
    click(&mut h, item.center());
    assert_eq!(fp_engine::commands::tool_text_defaults(&st(&h).session.prefs), (2, true, 1));
    assert_eq!(undo_count(&h), n0, "Vorgaben ohne Verlaufsschritt");
    // „Kein Anti-Alias“ als Vorgabe
    choose(&mut h, c, &aa_label("none"));
    assert_eq!(fp_engine::commands::tool_text_defaults(&st(&h).session.prefs).0, -1);
    combo_with(&h, &aa_label("none"));
    choose(&mut h, c, &aa_label("strong"));
    // neuer Text bekommt die Vorgaben
    let p = screen(&h, 100.0, 150.0);
    click(&mut h, p);
    text(&mut h, "Xy");
    // beim Tippen zeigt der Inspektor weiter die Vorgabe, die der neue Text bekommt
    combo_with(&h, &aa_label("strong"));
    key(&mut h, Key::Escape, Modifiers::NONE);
    h.run_steps(2);
    assert_eq!(undo_count(&h), n0 + 1, "ein Schritt für den neuen Text");
    let ts = doc_texts(&h);
    assert_eq!(ts.len(), 1);
    let t = &ts[0];
    assert_eq!((t.antialias, hard_edge(t), t.auto_kern, t.orientation), (2, false, true, 1));
    // §4b: 30× Vorgabe wechseln → Endwert, weiter kein Schritt
    let _ = h.state_mut().state.session.execute("select.set", json!({"ids": []}));
    let _ = h.state_mut().state.session.set_tool(fp_engine::ToolId::Text);
    h.run_steps(3);
    let n1 = undo_count(&h);
    for k in 0..30 {
        let name = ["crisp", "smooth", "none"][k % 3];
        choose(&mut h, c, &aa_label(name));
    }
    assert_eq!(fp_engine::commands::tool_text_defaults(&st(&h).session.prefs).0, -1);
    assert_eq!(undo_count(&h), n1);
}

// ------------------------------------------------------------------ Textgrenzen beim Bearbeiten

fn box_texts(h: &H) -> Vec<String> {
    let mut v: Vec<_> = inputs(h).into_iter().filter(|(r, _)| r.left() < 220.0 && r.top() > HH - 70.0).collect();
    v.sort_by(|a, c| a.0.top().total_cmp(&c.0.top()).then(a.0.left().total_cmp(&c.0.left())));
    v.into_iter().map(|x| x.1).collect()
}

/// Beim Tippen in einem vorhandenen Text zeigen B/H/X/Y die Textgrenzen mit 2-px-Rand wie
/// außerhalb der Bearbeitung (vorher sprangen sie beim Öffnen und Übernehmen um 2 px).
#[test]
fn text_bounds_while_editing_with_margin() {
    let mut id = 0;
    let mut h = harness(|s| {
        let v = s.execute("draw.text", json!({"text": "Hallo", "anchor": [30, 60], "size": 24})).expect("Text");
        id = v.value.get("id").and_then(|x| x.as_u64()).unwrap_or(0);
        s.execute("select.set", json!({"ids": [id]})).expect("Auswahl");
    });
    h.run_steps(2);
    let before = box_texts(&h);
    let d = st(&h).session.active().expect("d");
    let vis = fp_engine::docops::element_bounds(&d.doc, &element(&h, id)).expect("Grenzen");
    let want0: Vec<String> = [vis.width(), vis.left, vis.height(), vis.top].iter().map(|v| format!("{}", v.round() as i64)).collect();
    assert_eq!(before, want0, "außerhalb der Bearbeitung mit Rand");
    // mit dem Textwerkzeug in den Text klicken (Bearbeiten), nichts tippen: kein Sprung
    let _ = h.state_mut().state.session.set_tool(fp_engine::ToolId::Text);
    h.run_steps(2);
    let p = screen(&h, vis.left as f32 + 12.0, vis.top as f32 + vis.height() as f32 / 2.0);
    click(&mut h, p);
    assert!(st(&h).session.tool_is_editing_text(), "Text in Bearbeitung");
    assert_eq!(box_texts(&h), want0, "Öffnen zum Bearbeiten: B/X/H/Y unverändert");
    // tippen: Grenzen wachsen mit, mit Rand
    key(&mut h, Key::End, Modifiers::NONE);
    text(&mut h, " Welt");
    h.run_steps(2);
    let b = st(&h).session.text_edit_bounds().expect("Grenzen");
    let f = st(&h).session.text_edit_frame().expect("Rahmen");
    assert_eq!(b.left, f.left - 2.0);
    let want: Vec<String> = [b.width(), b.left, b.height(), b.top].iter().map(|v| format!("{}", v.round() as i64)).collect();
    assert_ne!(want, want0, "Text ist breiter geworden");
    assert_eq!(box_texts(&h), want, "B/X/H/Y beim Tippen mit Rand: {:?}", inputs(&h));
    key(&mut h, Key::Escape, Modifiers::NONE);
    h.run_steps(3);
    assert_eq!(box_texts(&h), want, "nach dem Übernehmen unverändert (kein Sprung)");
}

// ------------------------------------------------------------------ W14: Schriftschnitt

/// Ein Text in einer Familie, deren erste Schriftdatei nicht „Regular“ ist (Myriad Pro: „Bold“),
/// zeigt im Schnitt-Feld „Regular“ (Objekt und Werkzeug), solange er nicht fett ist.
#[test]
fn w14_font_style_shows_regular() {
    fp_ui::inspector::fonts::warm_up();
    let t0 = std::time::Instant::now();
    while fp_ui::inspector::fonts::families().len() <= 5 && t0.elapsed().as_secs() < 60 {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    let fams = fp_ui::inspector::fonts::families();
    let pick = fams.iter().find(|f| f.name == "Myriad Pro").or_else(|| fams.iter().find(|f| f.styles.first().is_some_and(|s| s.0 != "Regular") && f.styles.iter().any(|s| s.0 == "Regular")));
    let Some(fam) = pick.cloned() else {
        eprintln!("keine Familie mit „Regular“ an späterer Stelle installiert – übersprungen");
        return;
    };
    assert_eq!(fp_ui::inspector::fonts::lookup(&fam.name), (fam.name.clone(), "Regular".to_string()), "Familienname → Regular");
    for (style, ps) in &fam.styles {
        assert_eq!(&fp_ui::inspector::fonts::lookup(ps).1, style, "PostScript-Name behält seinen Schnitt");
    }
    let fname = fam.name.clone();
    let mut h = harness(|s| {
        s.execute("draw.text", json!({"text": "Hallo", "anchor": [30, 60], "size": 24, "font": fname})).expect("Text");
    });
    h.run_steps(2);
    combo_with(&h, &fam.name);
    combo_with(&h, "Regular");
    // Textwerkzeug mit dieser Schrift, ohne Auswahl
    let _ = h.state_mut().state.session.execute("select.set", json!({"ids": []}));
    h.state_mut().state.session.tools.options.text_font = fam.name.clone();
    let _ = h.state_mut().state.session.set_tool(fp_engine::ToolId::Text);
    h.run_steps(3);
    combo_with(&h, "Regular");
}

// ------------------------------------------------------------------ W10/W27/W28: Werkzeugname

/// Lange Werkzeugnamen stehen wie im Original zweizeilig im Inspektorkopf (vorher „…“); die
/// automatische Form zeigt den Namen der gewählten Form.
#[test]
fn w10_w27_w28_tool_name_two_lines() {
    let mut h = harness(|_| {});
    let squash = |s: &str| s.replace(['-', ' '], "");
    for tool in [
        fp_engine::ToolId::RedEyeRemoval,
        fp_engine::ToolId::PathScrubAdd,
        fp_engine::ToolId::PathScrubSubtract,
        fp_engine::ToolId::Reshape,
        fp_engine::ToolId::PolygonSlice,
        fp_engine::ToolId::HotspotRect,
    ] {
        let want = fp_ui::inspector::texte::tool_label(fp_engine::tools::info(tool).expect("Werkzeug"));
        let _ = h.state_mut().state.session.set_tool(tool);
        h.run_steps(3);
        let lines = fp_ui::inspector::tool_header_lines(&h.ctx);
        assert!(lines.len() == 2 && lines.iter().all(|l| !l.contains('…')), "{want}: zweizeilig, ungekürzt: {lines:?}");
        assert_eq!(squash(&lines.concat()), squash(&want), "{want}: voller Name");
    }
    let _ = h.state_mut().state.session.set_tool(fp_engine::ToolId::RedEyeRemoval);
    h.run_steps(3);
    assert_eq!(fp_ui::inspector::tool_header_lines(&h.ctx), vec!["Rote Augen".to_string(), "entfernen".to_string()]);
    // kurze Namen einzeilig
    let _ = h.state_mut().state.session.set_tool(fp_engine::ToolId::Text);
    h.run_steps(3);
    assert_eq!(fp_ui::inspector::tool_header_lines(&h.ctx), vec!["Text".to_string()]);
    // Auto-Form: Name der gewählten Form
    h.state_mut().state.session.tools.options.auto_shape = "Abgefastes Rechteck".into();
    let _ = h.state_mut().state.session.set_tool(fp_engine::ToolId::AutoShape);
    h.run_steps(3);
    assert_eq!(fp_ui::inspector::tool_header_lines(&h.ctx).join(" "), fp_i18n::t("shape-chamfer-rectangle"));
}

// ------------------------------------------------------------------ W16: Kurzzeichen F/K

/// Deutsch „F“/„K“ wie im Original, Englisch „B“/„I“.
#[test]
fn w16_bold_italic_letters_per_language() {
    assert_eq!(fp_i18n::t("pi-text-bold-glyph"), "F");
    assert_eq!(fp_i18n::t("pi-text-italic-glyph"), "K");
    let en = fp_i18n::with_language("en", || (fp_i18n::t("pi-text-bold-glyph"), fp_i18n::t("pi-text-italic-glyph")));
    assert_eq!(en, ("B".to_string(), "I".to_string()));
}
