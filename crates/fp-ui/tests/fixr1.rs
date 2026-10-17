//! FIX-R1 (fixg5 Q-05): Unterebene im Ebenen-Bedienfeld per Doppelklick umbenennen wie Ebenen und
//! Objekte – Eingabe übernimmt den Namen der Unterebene (Ebenenname) in einem Schritt, Esc verwirft.

use egui::{Event, Key, Modifiers, Pos2};
use egui_kittest::Harness;
use fp_engine::tools::ToolId;
use fp_ui::app_state::UiSettings;
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn harness() -> H {
    fp_i18n::set_language("de");
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 400, "height": 300, "background": "#ffffff"})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [20, 20, 120, 80]})).expect("a");
    s.execute("pi.layer.addSub", json!({"name": "Unter A"})).expect("sub");
    s.execute("select.none", json!({})).expect("none");
    s.set_tool(ToolId::Pointer).expect("Zeiger");
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.os_clipboard = false;
    app.state.sync_render = true;
    let mut h = Harness::builder().with_size(egui::vec2(1500.0, 950.0)).with_pixels_per_point(1.0).with_max_steps(8).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    // Registerkarte „Ebenen“ nach vorn holen (Klick auf die aktive Karte klappt ein: dann noch einmal)
    for _ in 0..2 {
        if find(&h, "Unter A").is_some() {
            break;
        }
        let p = find(&h, "Ebenen").expect("Karte Ebenen");
        click(&mut h, p, false);
        h.run_steps(30);
    }
    h
}

fn ev(h: &mut H, e: Event) {
    h.input_mut().events.push(e);
    h.step();
}

fn key(h: &mut H, k: Key) {
    ev(h, Event::Key { key: k, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE });
    ev(h, Event::Key { key: k, physical_key: None, pressed: false, repeat: false, modifiers: Modifiers::NONE });
}

fn click(h: &mut H, p: Pos2, double: bool) {
    h.run_steps(30);
    ev(h, Event::PointerMoved(p));
    let press = |down| Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: down, modifiers: Modifiers::NONE };
    ev(h, press(true));
    ev(h, press(false));
    if double {
        ev(h, press(true));
        ev(h, press(false));
    }
    h.run_steps(3);
}

fn texts(shape: &egui::epaint::Shape, out: &mut Vec<(String, egui::Rect)>) {
    match shape {
        egui::epaint::Shape::Text(t) => out.push((t.galley.job.text.clone(), egui::Rect::from_min_size(t.pos, t.galley.size()))),
        egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| texts(s, out)),
        _ => {}
    }
}

/// Mitte der gezeichneten Beschriftung (rechteste, d. h. im Bedienfeld rechts).
fn find(h: &H, label: &str) -> Option<Pos2> {
    let mut all = Vec::new();
    for cs in &h.output().shapes {
        texts(&cs.shape, &mut all);
    }
    all.iter().filter(|(t, _)| t == label).map(|(_, r)| r.center()).max_by(|a, b| a.x.total_cmp(&b.x))
}

fn sub_name(h: &H) -> String {
    let d = h.state().state.session.active().expect("d");
    let mut out = None;
    for (_, l) in fp_engine::docops::own_layers(&d.doc, d.page) {
        for e in l.cels.first().map(|c| c.elements.clone()).unwrap_or_default() {
            if let fp_model::ElementKind::SubLayer(sl) = &e.kind {
                out = Some(sl.name.clone());
            }
        }
    }
    out.expect("Unterebene")
}

fn undo_count(h: &H) -> usize {
    h.state().state.session.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

#[test]
fn sublayer_rename_by_double_click() {
    let mut h = harness();
    let n0 = undo_count(&h);
    // Esc verwirft
    let p = find(&h, "Unter A").expect("Zeile der Unterebene");
    click(&mut h, p, true);
    assert!(h.ctx.text_edit_focused(), "Doppelklick öffnet das Umbenennen-Feld");
    ev(&mut h, Event::Text("Nichts".into()));
    key(&mut h, Key::Escape);
    h.run_steps(3);
    assert_eq!((sub_name(&h), undo_count(&h)), ("Unter A".to_string(), n0), "Esc verwirft");
    // Doppelklick markiert den Namen, Tippen ersetzt, Eingabe übernimmt in einem Schritt
    let p = find(&h, "Unter A").expect("Zeile der Unterebene");
    click(&mut h, p, true);
    assert!(h.ctx.text_edit_focused());
    ev(&mut h, Event::Text("Unter B".into()));
    key(&mut h, Key::Enter);
    h.run_steps(3);
    assert_eq!(sub_name(&h), "Unter B");
    assert_eq!(undo_count(&h), n0 + 1, "genau ein Schritt");
    assert!(find(&h, "Unter B").is_some(), "neuer Name im Bedienfeld");
}
