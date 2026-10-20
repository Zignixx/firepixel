//! Funktionsvergleich mit dem Original (FV-1, Befunde UX-A im Abschnitt I18N-INSPEKTOR):
//! Popup-Schieber senkrecht (T4), Esc im Popup hebt die Auswahl nicht auf (T5), Zahlenfelder wie
//! Ziffernfelder des Originals (F2/F3/F7/F10), gemischte Werte (O14), gedrückte Formknöpfe (W2),
//! Textwerkzeug-Vorgaben vor dem Tippen (W15), Textausrichtung (W17), „Auswahl füllen“ je
//! Werkzeug (W25), Strichfeld im Text-Inspektor (O7). Realistische Eingaben wie egui-winit unter
//! Windows (je Ereignis ein Bild, Tippen als `Event::Text`, Ziehen über viele Bilder).

use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::{By, NodeT, Queryable};
use egui_kittest::Harness;
use fp_engine::ToolId;
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

fn drag(h: &mut H, a: Pos2, b: Pos2, frames: usize) {
    ev(h, Event::PointerMoved(a));
    press(h, a, true);
    for i in 1..=frames {
        ev(h, Event::PointerMoved(a + (b - a) * (i as f32 / frames as f32)));
    }
    press(h, b, false);
    h.run_steps(2);
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

fn doc(h: &H) -> Arc<fp_model::Document> {
    Arc::clone(&st(h).session.active().expect("Dokument").doc)
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

fn sliders(h: &H) -> Vec<egui::Rect> {
    h.query_all(By::new().role(egui::accesskit::Role::Slider)).map(|n| n.rect()).collect()
}

/// Feld anklicken, Inhalt wie im Messprotokoll ersetzen (Ende, Umschalt+Pos1, Tippen).
fn type_into(h: &mut H, field: egui::Rect, s: &str) {
    click(h, field.center());
    key(h, Key::End, Modifiers::NONE);
    key(h, Key::Home, Modifiers::SHIFT);
    text(h, s);
}

/// Pfeil ▾ rechts neben einem Zahlenfeld (1 DLU Abstand, 9 DLU breit).
fn arrow_of(field: egui::Rect) -> Pos2 {
    Pos2::new(field.right() + 3.0 + 1.5 + 6.75, field.center().y)
}

/// Zwei Rechtecke (R1 rot ausgewählt, R2 blau), B/H/X/Y ohne „100“.
fn two_rects() -> (H, u64, u64) {
    let mut ids = (0, 0);
    let h = harness(|s| {
        ids.0 = rect(s, [20, 20, 140, 90], "#ff0000");
        ids.1 = rect(s, [200, 40, 290, 110], "#0000ff");
        s.execute("select.set", json!({"ids": [ids.0]})).expect("Auswahl");
    });
    (h, ids.0, ids.1)
}

// ------------------------------------------------------------------ T5: Esc im Popup

/// Esc im geöffneten Popup-Schieber schließt nur das Popup; die Auswahl bleibt (vorher: Auswahl
/// aufgehoben, Inspektor sprang auf „Dokument“). Gleiches für die Kombifeld-Liste (Mischmodus).
#[test]
fn t5_escape_in_popup_keeps_selection() {
    let (mut h, r1, _) = two_rects();
    let f = input_with(&h, "100");
    click(&mut h, arrow_of(f));
    assert_eq!(sliders(&h).len(), 1, "Popup-Schieber offen");
    key(&mut h, Key::Escape, Modifiers::NONE);
    h.run_steps(2);
    assert!(sliders(&h).is_empty(), "Esc schließt den Schieber");
    assert_eq!(sel(&h), vec![r1], "Auswahl bleibt");
    assert_eq!(opacity(&h, r1), 100.0, "Wert unverändert");
    // danach ins Feld tippen trifft weiter die Deckkraft (nicht „Bildgröße…“ des Dokuments)
    let f = input_with(&h, "100");
    type_into(&mut h, f, "60");
    key(&mut h, Key::Enter, Modifiers::NONE);
    assert_eq!(opacity(&h, r1), 60.0);
    // a second Escape without the popup keeps the selection (re 51 §1)
    let p = screen(&h, 450.0, 250.0);
    click(&mut h, p);
    let _ = h.state_mut().state.session.execute("select.set", json!({"ids": [r1]}));
    h.run_steps(2);
    key(&mut h, Key::Escape, Modifiers::NONE);
    h.run_steps(2);
    assert_eq!(sel(&h), vec![r1], "Escape without popup keeps the selection");
}

// ------------------------------------------------------------------ T4: senkrechter Schieber

/// Der Pfeil öffnet einen senkrechten Schieber über dem Feld (oben = 100, unten = 0); Ziehen
/// zeigt den Wert live im Feld. Das per Klick geöffnete Popup bleibt nach dem Loslassen offen,
/// ein Klick daneben übernimmt in einem Schritt und schließt (TW-52).
#[test]
fn t4_popup_slider_is_vertical_and_commits_on_release() {
    let (mut h, r1, _) = two_rects();
    let f = input_with(&h, "100");
    let a = arrow_of(f);
    click(&mut h, a);
    let s = sliders(&h);
    assert_eq!(s.len(), 1, "Popup offen");
    let r = s[0];
    assert!(r.height() > 5.0 * r.width(), "senkrecht: {r:?}");
    assert!(r.left() < a.x + 8.0 && r.right() > a.x - 8.0, "über dem Pfeil: {r:?} / {a:?}");
    assert!(r.bottom() <= HH + 0.5 && r.top() >= 0.0, "im Fenster: {r:?}");
    let track_top = r.top() + fp_ui::inspector::fields::VSLIDER_PAD;
    let track = fp_ui::inspector::fields::VSLIDER_TRACK;
    // Knopf bei 100 steht oben; in die Mitte ziehen → 50
    let n0 = undo_count(&h);
    let knob = Pos2::new(r.center().x, track_top + 1.0);
    let mid = Pos2::new(r.center().x, track_top + track * 0.5);
    ev(&mut h, Event::PointerMoved(knob));
    press(&mut h, knob, true);
    for i in 1..=12 {
        ev(&mut h, Event::PointerMoved(knob + (mid - knob) * (i as f32 / 12.0)));
    }
    h.run_steps(1);
    let live = inputs(&h).into_iter().find(|(rr, _)| (rr.center().y - f.center().y).abs() < 1.0 && (rr.left() - f.left()).abs() < 1.0).map(|x| x.1).unwrap_or_default();
    assert_eq!(live, "50", "Feld zeigt den Wert beim Ziehen");
    assert_eq!(opacity(&h, r1), 100.0, "erst beim Loslassen übernommen");
    press(&mut h, mid, false);
    h.run_steps(2);
    assert_eq!(sliders(&h).len(), 1, "Loslassen lässt den Schieber offen (TW-52)");
    assert_eq!(opacity(&h, r1), 100.0, "noch nicht übernommen");
    click(&mut h, Pos2::new(r.right() + 200.0, r.top() + 20.0));
    assert_eq!(opacity(&h, r1), 50.0, "Klick daneben übernimmt");
    assert_eq!(undo_count(&h), n0 + 1, "ein Verlaufsschritt");
    assert!(sliders(&h).is_empty(), "Klick daneben schließt den Schieber");
    assert_eq!(sel(&h), vec![r1], "Klick daneben hebt die Auswahl nicht auf");
    // §4b: 10× ziehen → Endwert, je Mal ein Schritt, keine Duplikate, alles rückgängig
    let d0 = doc(&h);
    let n1 = undo_count(&h);
    for k in 0..10 {
        let f = inputs(&h).into_iter().find(|(rr, _)| (rr.center().y - f.center().y).abs() < 1.0 && (rr.left() - f.left()).abs() < 1.0).expect("Feld").0;
        click(&mut h, arrow_of(f));
        let r = sliders(&h)[0];
        let tt = r.top() + fp_ui::inspector::fields::VSLIDER_PAD;
        let want = 10.0 * (k + 1) as f32;
        let y = tt + track * (1.0 - want / 100.0);
        drag(&mut h, Pos2::new(r.center().x, y + 15.0), Pos2::new(r.center().x, y), 8);
        click(&mut h, Pos2::new(r.right() + 200.0, r.top() + 20.0));
        assert_eq!(opacity(&h, r1), want as f64, "Ziehen {k}");
    }
    assert_eq!(undo_count(&h), n1 + 10);
    assert_eq!(fp_engine::docops::selectable_ids(&doc(&h), 0, 0).len(), 2, "keine Duplikate");
    for _ in 0..10 {
        h.state_mut().state.session.undo().expect("undo");
    }
    assert!(*doc(&h) == *d0, "Rückgängig stellt her");
}

/// Klick neben den Schieber schließt ohne Änderung.
#[test]
fn t4_click_outside_closes_without_change() {
    let (mut h, r1, _) = two_rects();
    let f = input_with(&h, "100");
    click(&mut h, arrow_of(f));
    let n0 = undo_count(&h);
    let r = sliders(&h)[0];
    click(&mut h, Pos2::new(r.right() + 200.0, r.top() + 20.0));
    assert!(sliders(&h).is_empty());
    assert_eq!(opacity(&h, r1), 100.0);
    assert_eq!(undo_count(&h), n0);
}

// ------------------------------------------------------------------ F2/F3/F7/F10: Zahlenfelder

#[test]
fn number_fields_behave_like_digit_fields() {
    let (mut h, r1, r2) = two_rects();
    // F2: „-5“ → 5
    let f = input_with(&h, "100");
    type_into(&mut h, f, "-5");
    key(&mut h, Key::Enter, Modifiers::NONE);
    assert_eq!(opacity(&h, r1), 5.0, "F2");
    // F3: „12,5“ → 125 → 100
    let f = input_with(&h, "5");
    type_into(&mut h, f, "12,5");
    key(&mut h, Key::Enter, Modifiers::NONE);
    assert_eq!(opacity(&h, r1), 100.0, "F3");
    // F7: „7“ + Esc, dann Klick auf die Leinwand → 7 übernommen, ein Schritt
    let n0 = undo_count(&h);
    let f = input_with(&h, "100");
    type_into(&mut h, f, "7");
    key(&mut h, Key::Escape, Modifiers::NONE);
    assert_eq!(sel(&h), vec![r1], "Esc im Feld hebt die Auswahl nicht auf");
    assert_eq!(opacity(&h, r1), 100.0, "noch nicht übernommen");
    let p = screen(&h, 450.0, 260.0);
    click(&mut h, p);
    assert_eq!(opacity(&h, r1), 7.0, "F7: Esc verwirft nicht, Fokusverlust übernimmt");
    assert_eq!(undo_count(&h), n0 + 1, "ein Schritt");
    // F10 (PM-2, im Original neu gemessen): Strg+A im Feld markiert den Feldtext, die
    // Objektauswahl bleibt; Tippen ersetzt den markierten Text (genauer: inspector_fv9.rs)
    let _ = h.state_mut().state.session.execute("select.set", json!({"ids": [r1]}));
    h.run_steps(2);
    let f = input_with(&h, "7");
    click(&mut h, f.center());
    key(&mut h, Key::A, ctrl());
    h.run_steps(2);
    assert_eq!(sel(&h), vec![r1], "F10: Strg+A wählt keine Objekte aus");
    text(&mut h, "33");
    key(&mut h, Key::Enter, Modifiers::NONE);
    assert_eq!(opacity(&h, r1), 33.0, "F10: Feldtext war markiert und wurde ersetzt");
    let _ = r2;
    // Textfelder (Name) behalten Strg+A und Esc wie bisher (nur Zahlenfelder wie im Original)
}

// ------------------------------------------------------------------ O14: gemischte Werte

/// Multi-selection: opacity/style blank; fill colour of the topmost object in stacking order,
/// independent of the selection order (TW-120, re 45 §3).
#[test]
fn o14_mixed_values_and_topmost_color() {
    let (mut h, r1, r2) = two_rects();
    let _ = h.state_mut().state.session.execute("element.setOpacity", json!({"opacity": 50}));
    let _ = h.state_mut().state.session.execute("select.set", json!({"ids": [r1]}));
    let _ = h.state_mut().state.session.execute("select.set", json!({"ids": [r2], "mode": "add"}));
    h.run_steps(3);
    assert_eq!(sel(&h), vec![r1, r2]);
    let a = fp_ui::inspector::obj_quads::shown_attrs(st(&h)).expect("Attribute");
    assert_eq!(a.fill_color.0 & 0x00ff_ffff, 0x0000ff, "topmost (R2, blue)");
    assert!(inputs(&h).iter().all(|(_, v)| v != "50" && v != "100"), "mixed opacity = blank: {:?}", inputs(&h));
    // reverse selection order: still the topmost (blue)
    let _ = h.state_mut().state.session.execute("select.set", json!({"ids": [r2, r1]}));
    h.run_steps(2);
    let a = fp_ui::inspector::obj_quads::shown_attrs(st(&h)).expect("Attribute");
    assert_eq!(a.fill_color.0 & 0x00ff_ffff, 0x0000ff, "topmost (R2, blue)");
}

// ------------------------------------------------------------------ W2: gedrückte Formknöpfe

/// Radiergummi: der gewählte Formknopf hat den blauen Rand (Fokusfarbe), der andere nicht; ein
/// Klick auf „eckig“ schaltet um.
#[test]
fn w2_selected_brush_shape_is_visible() {
    let mut h = harness(|_| {});
    let _ = h.state_mut().state.session.set_tool(ToolId::Eraser);
    h.run_steps(3);
    fp_ui::inspector::fields::set_audit(&h.ctx, true);
    h.run_steps(1);
    let log = fp_ui::inspector::fields::take_audit(&h.ctx);
    fp_ui::inspector::fields::set_audit(&h.ctx, false);
    let form = fp_i18n::t("pi-shape");
    let i = log.iter().position(|e| e.text == form).expect("Beschriftung Form");
    let btns: Vec<egui::Rect> = log[i + 1..].iter().filter(|e| e.kind == "field").take(2).map(|e| e.avail).collect();
    assert_eq!(btns.len(), 2);
    let focus = fp_ui::theme::Theme::current(&h.ctx).colors.focus;
    let outlined = |h: &H, r: egui::Rect| {
        h.output().shapes.iter().any(|cs| match &cs.shape {
            egui::Shape::Rect(rs) => rs.rect == r && rs.stroke.color == focus && rs.stroke.width >= 1.0,
            _ => false,
        })
    };
    let round = st(&h).session.tools.options.eraser_round;
    assert!(round, "Vorgabe rund");
    assert!(outlined(&h, btns[0]), "rund gedrückt sichtbar");
    assert!(!outlined(&h, btns[1]), "eckig nicht gedrückt");
    click(&mut h, btns[1].center());
    h.run_steps(2);
    assert!(!st(&h).session.tools.options.eraser_round, "eckig gewählt");
    assert!(outlined(&h, btns[1]) && !outlined(&h, btns[0]), "Markierung wechselt");
}

// ------------------------------------------------------------------ W25: Auswahl füllen je Werkzeug

#[test]
fn w25_fill_selection_default_per_tool() {
    let mut h = harness(|_| {});
    let _ = h.state_mut().state.session.set_tool(ToolId::Gradient);
    h.run_steps(3);
    // Farbeimer und Verlauf haben getrennte Felder (FV-16, `gradient_fill_selection`)
    assert!(st(&h).session.tools.options.gradient_fill_selection, "Verlauf: Vorgabe ☑");
    let _ = h.state_mut().state.session.set_tool(ToolId::PaintBucket);
    h.run_steps(3);
    assert!(!st(&h).session.tools.options.fill_selection, "Farbeimer: Vorgabe ☐");
    // Toleranz (32 beim Farbeimer) ist bedienbar; im Verlauf grau → Tippen ohne Wirkung
    let f = input_with(&h, "32");
    type_into(&mut h, f, "40");
    key(&mut h, Key::Enter, Modifiers::NONE);
    assert_eq!(st(&h).session.tools.options.fill_tolerance, 40);
    let _ = h.state_mut().state.session.set_tool(ToolId::Gradient);
    h.run_steps(3);
    // der Verlauf hat seine eigene Toleranz (Werksvorgabe 32)
    let f = input_with(&h, "32");
    type_into(&mut h, f, "50");
    key(&mut h, Key::Enter, Modifiers::NONE);
    assert_eq!(st(&h).session.tools.options.gradient_tolerance, 32, "grau bei „Auswahl füllen“");
    assert_eq!(st(&h).session.tools.options.fill_tolerance, 40);
    // je Werkzeug gemerkt
    let _ = h.state_mut().state.session.set_tool(ToolId::PaintBucket);
    h.run_steps(2);
    let _ = h.state_mut().state.session.set_tool(ToolId::Gradient);
    h.run_steps(2);
    assert!(st(&h).session.tools.options.gradient_fill_selection);
    assert!(!st(&h).session.tools.options.fill_selection);
}

// ------------------------------------------------------------------ W15: Textwerkzeug-Vorgaben

/// Textwerkzeug ohne Text: Laufweite, Zeilenabstand, Einzug, Abstände, Horizontalskala sind
/// bedienbar und gelten für den nächsten neuen Text (ein Verlaufsschritt für den Text).
#[test]
fn w15_text_tool_defaults_before_typing() {
    let mut h = harness(|_| {});
    let _ = h.state_mut().state.session.set_tool(ToolId::Text);
    h.run_steps(3);
    // Zeilenabstand der Werksvorgabe 120 %
    let lead = input_with(&h, "120");
    type_into(&mut h, lead, "150");
    key(&mut h, Key::Enter, Modifiers::NONE);
    assert_eq!(st(&h).session.prefs.get_real("Text_Leading", 0.0), 1.5);
    // Laufweite: Feld links in derselben Zeile
    let row = inputs(&h).into_iter().filter(|(r, _)| (r.center().y - lead.center().y).abs() < 1.0 && r.right() < lead.left()).max_by(|a, b| a.0.left().total_cmp(&b.0.left())).expect("Laufweite").0;
    type_into(&mut h, row, "25");
    key(&mut h, Key::Enter, Modifiers::NONE);
    // Horizontalskala 100 % → 80 (unten links in Zeile 2; das Feld zeigt wie im Original „100%“, O8)
    let hs = inputs(&h).into_iter().filter(|(_, v)| v == "100%").max_by(|a, b| a.0.top().total_cmp(&b.0.top())).expect("Horizontalskala").0;
    type_into(&mut h, hs, "80");
    key(&mut h, Key::Enter, Modifiers::NONE);
    // Vorgaben in den Voreinstellungen des Textwerkzeugs (wie im Original)
    assert!((st(&h).session.prefs.get_real("Text_RangeKerning", 0.0) - 0.25).abs() < 1e-9, "Feldwert / 100");
    assert!((st(&h).session.prefs.get_real("Text_HorizScale", 1.0) - 0.8).abs() < 1e-6);
    // neuen Text tippen
    let n0 = undo_count(&h);
    let p = screen(&h, 100.0, 150.0);
    click(&mut h, p);
    text(&mut h, "Hallo");
    key(&mut h, Key::Escape, Modifiers::NONE);
    h.run_steps(2);
    assert_eq!(undo_count(&h), n0 + 1, "ein Schritt für den neuen Text");
    let d = st(&h).session.active().expect("d");
    let t = fp_engine::docops::selectable_ids(&d.doc, d.page, d.frame).into_iter().find_map(|id| fp_engine::docops::element_on_page(&d.doc, d.page, id).and_then(|e| if let fp_model::ElementKind::Text(t) = &e.kind { Some(t.clone()) } else { None })).expect("Text");
    assert_eq!(t.plain_text(), "Hallo");
    let a = t.resolved_runs().first().map(|(a, _)| a.clone()).expect("Lauf");
    assert_eq!(a.tracking.unwrap_or(0.0), 0.0, "neuer Text: Laufweite 0 (R6)");
    assert_eq!(a.leading, Some(1.5), "Zeilenabstand aus der Vorgabe");
    assert!((a.horizontal_scale.unwrap_or(1.0) - 0.8).abs() < 1e-6, "Horizontalskala");
}

// ------------------------------------------------------------------ W17: Textausrichtung

#[test]
fn w17_text_orientation_button() {
    let mut id = 0;
    let mut h = harness(|s| {
        let v = s.execute("draw.text", json!({"text": "Hallo", "anchor": [30, 60], "size": 24})).expect("Text");
        id = v.value.get("id").and_then(|x| x.as_u64()).unwrap_or(0);
    });
    h.run_steps(2);
    // Knopf über das Layout-Protokoll: Tooltip-Text ist „Textausrichtung festlegen“ → Lage aus
    // der Feldfolge F/K/U, danach der Knopf
    fp_ui::inspector::fields::set_audit(&h.ctx, true);
    h.run_steps(1);
    let log = fp_ui::inspector::fields::take_audit(&h.ctx);
    fp_ui::inspector::fields::set_audit(&h.ctx, false);
    // Unterstrichen-Knopf: 3. von 3 Umschaltern gleicher Größe in Zeile 2 von Q3; der Knopf liegt
    // 3 DLU rechts davon (DLU 184 statt 168)
    let small: Vec<egui::Rect> = log.iter().filter(|e| e.kind == "field" && (e.avail.width() - 20.0).abs() < 0.6 && (e.avail.height() - 20.0).abs() < 0.6).map(|e| e.avail).collect();
    assert!(small.len() >= 7, "F/K/U + 4 Ausrichtungen: {small:?}");
    let u = small[2];
    let orient = Pos2::new(u.right() + (184.0 - 168.0 - 13.0) * 1.5 + 13.0 * 0.75, u.center().y);
    let n0 = undo_count(&h);
    click(&mut h, orient);
    let label = fp_i18n::t("pi-text-orient-vrl");
    let item = h.get_by_label(&label).rect();
    click(&mut h, item.center());
    let e = element(&h, id);
    let fp_model::ElementKind::Text(t) = &e.kind else { panic!("Text") };
    assert_eq!(t.orientation, 1, "vertikal rechts → links");
    assert_eq!(undo_count(&h), n0 + 1);
    h.state_mut().state.session.undo().expect("undo");
    let e = element(&h, id);
    let fp_model::ElementKind::Text(t) = &e.kind else { panic!("Text") };
    assert_eq!(t.orientation, 0);
}

// ------------------------------------------------------------------ O7: Strich im Text-Inspektor

#[test]
fn o7_text_inspector_has_stroke_field() {
    let mut id = 0;
    let h = harness(|s| {
        let v = s.execute("draw.text", json!({"text": "Hallo", "anchor": [30, 60], "size": 24})).expect("Text");
        id = v.value.get("id").and_then(|x| x.as_u64()).unwrap_or(0);
    });
    let mut h = h;
    fp_ui::inspector::fields::set_audit(&h.ctx, true);
    h.run_steps(1);
    let log = fp_ui::inspector::fields::take_audit(&h.ctx);
    fp_ui::inspector::fields::set_audit(&h.ctx, false);
    // Farbfeld 14 × 13 DLU = 21 × 21 px (Strichfarbe), nicht mehr die Grundlinie
    let sw: Vec<egui::Rect> = log.iter().filter(|e| e.kind == "field" && (e.avail.width() - 21.0).abs() < 0.6 && (e.avail.height() - 21.0).abs() < 0.6).map(|e| e.avail).collect();
    // nur Zeile 2 von Q3 (oben liegen gleich große Knöpfe in Q5/Q6)
    let top = sw.iter().map(|r| r.top()).fold(f32::MAX, f32::min);
    let sw: Vec<egui::Rect> = sw.into_iter().filter(|r| r.top() > top + 10.0).collect();
    assert_eq!(sw.len(), 1, "Strichfarbfeld im Text-Inspektor: {sw:?}");
    let _ = id;
    // Wirkung: pi.stroke auf dem Text (wie das Farbfeld nach der Farbwahl)
    let n0 = undo_count(&h);
    h.state_mut().state.run("pi.stroke", json!({"color": "#ff0000", "preset": "Bleistift/1-Pixel weich"}));
    h.run_steps(2);
    let e = element(&h, id);
    let a = fp_engine::docops::path_attrs_of(&e).expect("Attribute");
    assert!(a.brush.is_some() && a.stroke_color.0 & 0x00ff_ffff == 0xff0000);
    assert_eq!(undo_count(&h), n0 + 1);
}

// ------------------------------------------------------------------ N14: Schriftgrößen-Schieber

fn text_size(h: &H, id: u64) -> f32 {
    let e = element(h, id);
    let fp_model::ElementKind::Text(t) = &e.kind else { panic!("Text") };
    t.resolved_runs().first().and_then(|(a, _)| a.size).unwrap_or(0.0)
}

/// Nutzerbefund N14: der Popup-Schieber der Schriftgröße sprang in 100er-Schritten. Wie im
/// Original: senkrecht, 8 … 96 in ganzen Schritten (≈ 2,3 px je Punkt), Feld nimmt mehr an.
#[test]
fn n14_font_size_slider_fine_steps() {
    let mut id = 0;
    let mut h = harness(|s| {
        let v = s.execute("draw.text", json!({"text": "Hallo", "anchor": [30, 60], "size": 24})).expect("Text");
        id = v.value.get("id").and_then(|x| x.as_u64()).unwrap_or(0);
    });
    let f = input_with(&h, "24");
    let open = |h: &mut H| {
        click(h, arrow_of(f));
        let r = sliders(h)[0];
        assert!(r.height() > 5.0 * r.width(), "senkrecht");
        r
    };
    let track = fp_ui::inspector::fields::VSLIDER_TRACK;
    let thumb = |r: egui::Rect, v: f32| r.top() + fp_ui::inspector::fields::VSLIDER_PAD + (1.0 - (v - 8.0) / 88.0) * track;
    // 7 px nach oben = 3 Punkte, nicht 100
    let r = open(&mut h);
    let y = thumb(r, 24.0);
    drag(&mut h, Pos2::new(r.center().x, y), Pos2::new(r.center().x, y - 7.0), 10);
    click(&mut h, Pos2::new(f.left() + 4.0, f.center().y)); // übernehmen (TW-52)
    assert_eq!(text_size(&h, id), 27.0, "feine Schritte");
    // ganz nach oben = 96, ganz nach unten = 8
    let r = open(&mut h);
    let y = thumb(r, 27.0);
    drag(&mut h, Pos2::new(r.center().x, y), Pos2::new(r.center().x, r.top() - 40.0), 12);
    click(&mut h, Pos2::new(f.left() + 4.0, f.center().y)); // übernehmen (TW-52)
    assert_eq!(text_size(&h, id), 96.0);
    let r = open(&mut h);
    let y = thumb(r, 96.0);
    drag(&mut h, Pos2::new(r.center().x, y), Pos2::new(r.center().x, r.bottom() + 40.0), 12);
    click(&mut h, Pos2::new(f.left() + 4.0, f.center().y)); // übernehmen (TW-52)
    assert_eq!(text_size(&h, id), 8.0);
    // getippt geht mehr (T3): 200
    let f2 = input_with(&h, "8");
    type_into(&mut h, f2, "200");
    key(&mut h, Key::Enter, Modifiers::NONE);
    assert_eq!(text_size(&h, id), 200.0);
}

// ------------------------------------------------------------------ FV-7: B/X wie im Original

fn bounds(h: &H, id: u64) -> (f64, f64, f64, f64) {
    let d = st(h).session.active().expect("Dokument");
    let e = element(h, id);
    let b = fp_engine::docops::element_bounds(&d.doc, &e).expect("Grenzen");
    (b.left as f64, b.top as f64, (b.right - b.left) as f64, (b.bottom - b.top) as f64)
}

/// Felder B, X, H, Y (Index 0–3) im Inspektor-Quadranten links unten.
fn box_fields(h: &H) -> Vec<(egui::Rect, String)> {
    let mut v: Vec<_> = inputs(h).into_iter().filter(|(r, _)| r.left() < 220.0 && r.top() > HH - 70.0).collect();
    v.sort_by(|a, b| a.0.top().total_cmp(&b.0.top()).then(a.0.left().total_cmp(&b.0.left())));
    assert_eq!(v.len(), 4, "B/X/H/Y: {v:?}");
    v
}

fn box_field(h: &H, i: usize) -> egui::Rect {
    box_fields(h)[i].0
}

fn box_text(h: &H, i: usize) -> String {
    box_fields(h)[i].1.clone()
}

/// PARITAET_A F12/F13 (Messung PM-1, R1 100 × 70 bei x = 50): Klick, Ende, Umschalt+Pos1,
/// Tippen, Eingabe. B: „-5“ und „0“ verworfen, „12,5“ → 512 (Komma = Schreibmarke an den
/// Anfang), „x“ verworfen, 9999 → 6000; X: „12,5“ → 512, „x“ → 0, -5 und 9999 erlaubt;
/// Pfeiltasten und Mausrad ohne Wirkung.
#[test]
fn f12_f13_width_and_x_fields_like_original() {
    let mut id = 0;
    let mut h = harness(|s| {
        id = rect(s, [50, 20, 150, 90], "#ff0000");
        s.execute("select.set", json!({"ids": [id]})).expect("Auswahl");
    });
    assert_eq!(bounds(&h, id), (50.0, 20.0, 100.0, 70.0));
    // B
    for (typed, want) in [("-5", 100.0), ("0", 100.0), ("x", 100.0)] {
        let n0 = undo_count(&h);
        let f = box_field(&h, 0);
        type_into(&mut h, f, typed);
        key(&mut h, Key::Enter, Modifiers::NONE);
        h.run_steps(2);
        assert_eq!(bounds(&h, id).2, want, "B „{typed}“ verworfen");
        assert_eq!(undo_count(&h), n0, "B „{typed}“: kein Verlaufsschritt");
    }
    let f = box_field(&h, 0);
    type_into(&mut h, f, "12,5");
    assert_eq!(box_text(&h, 0), "512", "Feldtext nach „12,5“");
    key(&mut h, Key::Enter, Modifiers::NONE);
    h.run_steps(2);
    assert_eq!(bounds(&h, id).2, 512.0, "B „12,5“ → 512");
    let f = box_field(&h, 0);
    type_into(&mut h, f, "9999");
    key(&mut h, Key::Enter, Modifiers::NONE);
    h.run_steps(2);
    assert_eq!(bounds(&h, id).2, 6000.0, "B 9999 → 6000");
    let _ = h.state_mut().state.session.execute("element.setBounds", json!({"width": 100}));
    h.run_steps(3);
    // X
    for (typed, want) in [("12,5", 512.0), ("-5", -5.0), ("9999", 9999.0), ("x", 0.0), ("50", 50.0)] {
        let f = box_field(&h, 1);
        type_into(&mut h, f, typed);
        key(&mut h, Key::Enter, Modifiers::NONE);
        h.run_steps(2);
        assert_eq!(bounds(&h, id).0, want, "X „{typed}“");
        assert_eq!(bounds(&h, id).2, 100.0, "X ändert die Breite nicht");
    }
    // Pfeiltasten und Mausrad im fokussierten Feld: nichts
    let before = doc(&h);
    let f = box_field(&h, 0);
    click(&mut h, f.center());
    key(&mut h, Key::ArrowUp, Modifiers::NONE);
    key(&mut h, Key::ArrowDown, Modifiers::NONE);
    ev(&mut h, Event::MouseWheel { unit: egui::MouseWheelUnit::Line, delta: egui::vec2(0.0, 1.0), modifiers: Modifiers::NONE, phase: egui::TouchPhase::Move });
    h.run_steps(4);
    key(&mut h, Key::Enter, Modifiers::NONE);
    h.run_steps(2);
    assert!(Arc::ptr_eq(&before, &doc(&h)) || *before == *doc(&h), "Pfeil/Rad ändern nichts");
    // 50 Eingaben hintereinander → wie einmal der Endwert, je ein Schritt
    for i in 0..50 {
        let n0 = undo_count(&h);
        let f = box_field(&h, 0);
        type_into(&mut h, f, &format!("{}", 101 + i));
        key(&mut h, Key::Enter, Modifiers::NONE);
        // Verlaufslänge ist begrenzt: bis dahin genau ein Schritt je Eingabe
        assert!(undo_count(&h) == n0 + 1 || undo_count(&h) == n0, "ein Schritt je Eingabe");
    }
    assert_eq!(bounds(&h, id).2.round(), 150.0, "Endwert (Gleitkomma-Rest der Skalierung)");
}

/// PARITAET_A W23: Rechteck-Werkzeug ohne Auswahl zeigt keine „Eckrundung des Rechtecks“.
#[test]
fn w23_rectangle_tool_without_roundness_row() {
    let mut h = harness(|_| {});
    let _ = h.state_mut().state.session.set_tool(ToolId::Rectangle);
    fp_ui::inspector::fields::set_audit(&h.ctx, true);
    h.run_steps(3);
    let log: Vec<String> = fp_ui::inspector::fields::take_audit(&h.ctx).into_iter().map(|a| a.text).collect();
    fp_ui::inspector::fields::set_audit(&h.ctx, false);
    assert!(log.iter().any(|t| t.contains("Füllung") || t.contains("Kante") || !t.is_empty()), "Werkzeug-Inspektor gezeichnet: {log:?}");
    assert!(!log.iter().any(|t| t.contains("Eckrundung")), "keine Eckrundung im Werkzeug-Inspektor: {log:?}");
}

/// FV-4-Befund: Kästchen und Optionsfelder stehen mit Beschriftung und Zustand im
/// Barrierefreiheitsbaum (egui_kittest, `ui_find`).
#[test]
fn checkboxes_have_accessibility_info() {
    let mut h = harness(|_| {});
    let _ = h.state_mut().state.session.set_tool(ToolId::Gradient);
    h.run_steps(3);
    let n = h.get_by_role_and_label(egui::accesskit::Role::CheckBox, "Auswahl füllen");
    assert_eq!(n.accesskit_node().toggled(), Some(egui::accesskit::Toggled::True), "Verlauf: Vorgabe ☑");
    n.click();
    h.run_steps(3);
    assert!(!st(&h).session.tools.options.gradient_fill_selection, "Klick über den Baum schaltet um");
    let n = h.get_by_role_and_label(egui::accesskit::Role::CheckBox, "Auswahl füllen");
    assert_eq!(n.accesskit_node().toggled(), Some(egui::accesskit::Toggled::False));
}
