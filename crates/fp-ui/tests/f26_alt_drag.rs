//! F26 (TW-97, Lauf 11 `xs-shortcuts/13`): Alt-Zug mit dem Zeiger dupliziert – mit echter Eingabe
//! und in der Bildfolge der Brücke (Läufer: Zusatztasten vor jedem Zeigerereignis, Alt gehalten
//! bis nach dem Loslassen, jedes Ereignis ein eigenes Bild). Dazu TW-99: Zug aus der
//! Dokumentbibliothek fügt genau eine Instanz ein (T06 vorher 4|3, danach 5|3).

use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use fp_engine::tools::ToolId;
use fp_engine::Session;
use fp_ui::app_state::UiSettings;
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

const T04: &str = r"C:\Users\Administrator\Desktop\REA\re\_testdateien\par\par_vector.fw.png";

fn harness(s: Session) -> H {
    let ctx = egui::Context::default();
    let mut a = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    a.state.os_clipboard = false;
    a.state.sync_render = true;
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).with_pixels_per_point(1.0).with_max_steps(8).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), a);
    h.run_steps(4);
    h
}

fn at(h: &H, x: f32, y: f32) -> Pos2 {
    let s = &h.state().state;
    let d = s.session.active().expect("Dokument");
    let pr = s.canvases.get(&d.id).and_then(|c| c.page_screen).expect("Seitenlage");
    let r = fp_render::Renderer::page_rect(&d.doc, d.page).expect("Seite");
    let (w, ht) = ((r.right - r.left) as f32, (r.bottom - r.top) as f32);
    egui::pos2(pr.left() + (x - r.left as f32) / w * pr.width(), pr.top() + (y - r.top as f32) / ht * pr.height())
}

fn count(h: &H) -> usize {
    let d = h.state().state.session.active().expect("Dokument");
    fp_engine::docops::own_layers(&d.doc, d.page).first().and_then(|(_, l)| l.cel(d.frame).map(|c| c.elements.len())).unwrap_or(0)
}

fn names(h: &H) -> Vec<String> {
    let d = h.state().state.session.active().expect("Dokument");
    fp_engine::docops::own_layers(&d.doc, d.page).first().and_then(|(_, l)| l.cel(d.frame).map(|c| c.elements.iter().map(|e| format!("{:?}#{}", e.common.name, e.id.0)).collect())).unwrap_or_default()
}

fn open_t04_r1() -> Option<H> {
    if !std::path::Path::new(T04).exists() {
        eprintln!("Testdatei fehlt, übersprungen");
        return None;
    }
    let mut s = Session::default();
    s.execute("file.open", json!({"path": T04})).expect("öffnen");
    let mut h = harness(s);
    let _ = h.state_mut().state.session.set_tool(ToolId::Pointer);
    let id = {
        let d = h.state().state.session.active().expect("Dokument");
        fp_engine::docops::own_layers(&d.doc, d.page).iter().flat_map(|(_, l)| l.cels.iter().flat_map(|c| c.elements.iter())).find(|e| e.common.name.as_deref() == Some("R1")).map(|e| e.id.0).expect("R1")
    };
    h.state_mut().state.run("select.set", json!({"ids": [id], "mode": "replace"}));
    h.run_steps(3);
    Some(h)
}

/// Ein Bild wie die Brücke: gehaltene Zusatztasten stehen vorn (`hold_modifiers`).
fn frame(h: &mut H, held: Modifiers, evs: Vec<Event>) {
    if held != Modifiers::NONE && !evs.iter().any(|e| matches!(e, Event::ModifiersChanged(_))) {
        h.input_mut().events.push(Event::ModifiersChanged(held));
    }
    for e in evs {
        let e = match e {
            Event::ModifiersChanged(m) => Event::ModifiersChanged(m | held),
            o => o,
        };
        h.input_mut().events.push(e);
    }
    h.step();
}

/// Der Zug des Läufers (`runner.py`, Firepixel-Seite `canvas_drag`) als Bildfolge der Brücke.
fn bridge_drag(h: &mut H, from: Pos2, to: Pos2, m: Modifiers) {
    let n = 20;
    let down = |pressed, pos| Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: m };
    frame(h, m, vec![Event::ModifiersChanged(m)]);
    frame(h, m, vec![Event::PointerMoved(from)]);
    frame(h, m, vec![Event::ModifiersChanged(m)]);
    frame(h, m, vec![down(true, from)]);
    for i in 1..=n {
        frame(h, m, vec![Event::ModifiersChanged(m)]);
        frame(h, m, vec![Event::PointerMoved(from + (to - from) * (i as f32 / n as f32))]);
    }
    frame(h, m, vec![Event::ModifiersChanged(m)]);
    frame(h, m, vec![Event::PointerMoved(to)]);
    frame(h, m, vec![Event::ModifiersChanged(m)]);
    frame(h, m, vec![down(false, to)]);
    for _ in 0..2 {
        frame(h, m, vec![]);
    }
    frame(h, Modifiers::NONE, vec![Event::ModifiersChanged(Modifiers::NONE)]);
    h.run_steps(4);
}

fn sel_rect(h: &H) -> Vec<String> {
    let s = &h.state().state.session;
    let d = s.active().expect("Dokument");
    d.selection.elements.iter().filter_map(|id| fp_engine::docops::element_on_page(&d.doc, d.page, *id)).filter_map(|e| fp_engine::docops::element_bounds(&d.doc, &e).map(|b| format!("{:?}:{},{},{},{}", e.common.name, b.left, b.top, b.right, b.bottom))).collect()
}

const T06: &str = r"C:\Users\Administrator\Desktop\REA\re\_testdateien\par\par_symbol_web.fw.png";

fn ctrl_z(h: &mut H) {
    let ctrl = Modifiers { ctrl: true, command: true, ..Modifiers::NONE };
    let key = |pressed| Event::Key { key: egui::Key::Z, physical_key: Some(egui::Key::Z), pressed, repeat: false, modifiers: ctrl };
    frame(h, ctrl, vec![key(true)]);
    frame(h, ctrl, vec![key(false)]);
    frame(h, Modifiers::NONE, vec![Event::ModifiersChanged(Modifiers::NONE)]);
    h.run_steps(3);
}

/// Echte Eingabe: Alt steckt im Drücken, Bewegen und Loslassen kommen in eigenen Bildern.
#[test]
fn alt_drag_duplicates_with_real_input_tw97() {
    let Some(mut h) = open_t04_r1() else { return };
    let (a, b) = (at(&h, 100.0, 85.0), at(&h, 160.0, 95.0));
    for e in [
        Event::PointerMoved(a),
        Event::PointerButton { pos: a, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::ALT },
        Event::PointerMoved(a + egui::vec2(10.0, 0.0)),
        Event::PointerMoved(b),
        Event::PointerButton { pos: b, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::ALT },
    ] {
        h.input_mut().events.push(e);
        h.step();
    }
    h.run_steps(3);
    assert_eq!(count(&h), 8, "Alt-Zug dupliziert: {:?}", names(&h));
    assert_eq!(sel_rect(&h), vec!["Some(\"R1\"):110,60,210,130".to_string()], "Kopie verschoben, ausgewählt");
}

/// Ablauf des Läufers über die Brücke (Lauf 11 `xs-shortcuts/13`): Umschalt-Zug, Strg+Z, Alt-Zug,
/// mit Zeiger und Teilauswahl. Ergebnis wie im Original: 8 Objekte.
#[test]
fn alt_drag_duplicates_in_bridge_frames_tw97() {
    for tool in [ToolId::Pointer, ToolId::Subselection] {
        let Some(mut h) = open_t04_r1() else { return };
        let _ = h.state_mut().state.session.set_tool(tool);
        h.run_steps(2);
        let (a, b) = (at(&h, 100.0, 85.0), at(&h, 160.0, 95.0));
        bridge_drag(&mut h, a, b, Modifiers::SHIFT);
        if tool == ToolId::Pointer {
            // TW-118: Shift on the selected R1 never moves it with the pointer (additive marquee)
            assert_eq!(sel_rect(&h), vec!["Some(\"R1\"):50,50,150,120".to_string()], "{tool:?}: Shift drag does not move");
        } else {
            assert_eq!(sel_rect(&h), vec!["Some(\"R1\"):110,50,210,120".to_string()], "{tool:?}: Umschalt-Zug waagrecht");
            ctrl_z(&mut h);
            assert_eq!(sel_rect(&h), vec!["Some(\"R1\"):50,50,150,120".to_string()], "{tool:?}: Strg+Z");
        }
        bridge_drag(&mut h, a, b, Modifiers::ALT);
        assert_eq!(count(&h), 8, "{tool:?}: Alt-Zug über die Brücke dupliziert: {:?}", names(&h));
    }
}

/// TW-99/Q-42: T06 hat vor dem Zug 4|3 Objekte; das Ablegen von „Grafik1“ auf der Leinwand fügt
/// genau eine Instanz ein (5|3). Das „4|3“ des Originals in Lauf 11 heißt: dort kam der Zug nicht an.
#[test]
fn doclib_drop_inserts_exactly_one_instance_tw99() {
    if !std::path::Path::new(T06).exists() {
        return;
    }
    let mut s = Session::default();
    s.execute("file.open", json!({"path": T06})).expect("öffnen");
    let mut h = harness(s);
    let per_layer = |h: &H| {
        let d = h.state().state.session.active().expect("Dokument");
        fp_engine::docops::own_layers(&d.doc, d.page).iter().map(|(_, l)| l.cel(d.frame).map(|c| c.elements.len()).unwrap_or(0)).collect::<Vec<_>>()
    };
    assert_eq!(per_layer(&h), vec![4, 3], "Ausgangslage T06");
    let i = h.state().state.session.active().and_then(|d| d.doc.symbols.iter().position(|s| s.name == "Grafik1")).expect("Grafik1");
    fp_ui::panels::drag_drop::start(&h.ctx, fp_ui::panels::drag_drop::Payload::Symbol(i));
    let p = at(&h, 600.0, 400.0);
    h.input_mut().events.push(Event::PointerMoved(p));
    h.run_steps(6);
    assert_eq!(per_layer(&h), vec![5, 3], "genau eine Instanz");
    assert!(fp_ui::panels::drag_drop::current(&h.ctx).is_none(), "Ziehen beendet");
}
