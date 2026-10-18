//! F22 / TW-70: Esc nach Mausöffnung eines Menüs schließt wie unter Windows eine Ebene je Esc
//! (Untermenü, Menü, Leiste); the next Escape reaches the canvas and keeps the selection (re 51 §1).
//! Enter auf einem per Maus geöffneten Menü lässt die Auswahl stehen.

use egui::{Event, Id, Key, Modifiers, PointerButton, Pos2, Rect, Shape};
use egui_kittest::Harness;
use fp_ui::app_state::UiSettings;
use fp_ui::menu::{self, Node};
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn harness() -> H {
    let ctx = egui::Context::default();
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("doc");
    let id = s.execute("draw.rectangle", json!({"rect": [20, 20, 120, 100], "name": "R1"})).expect("rect").value["id"].as_u64().expect("id");
    s.execute("select.set", json!({"ids": [id]})).expect("Auswahl");
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).with_pixels_per_point(1.0).with_max_steps(12).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(3);
    h
}

fn selected(h: &H) -> usize {
    h.state().state.session.active().map(|d| d.selection.elements.len()).unwrap_or(0)
}

fn kbd(h: &H) -> (bool, usize, Vec<usize>) {
    let k = fp_ui::menu_kbd::state(&h.ctx);
    (k.active, k.top, k.path)
}

fn ev(h: &mut H, e: Event) {
    h.event(e);
    h.run_steps(1);
}

fn key(h: &mut H, k: Key) {
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: Modifiers::NONE });
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: false, repeat: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
}

fn click(h: &mut H, p: Pos2) {
    ev(h, Event::PointerMoved(p));
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
}

fn titles(h: &H) -> Vec<Rect> {
    h.ctx.data(|d| d.get_temp::<Vec<Rect>>(Id::new(menu::MENU_TITLES_ID))).expect("Menütitel")
}

fn any_menu_open(h: &H) -> bool {
    (0..=menu::main_menu().len()).any(|i| egui::Popup::is_id_open(&h.ctx, Id::new(("fp-main-menu", i))))
}

fn collect(shape: &Shape, out: &mut Vec<(String, Rect)>) {
    match shape {
        Shape::Text(t) => out.push((t.galley.text().to_string(), t.visual_bounding_rect())),
        Shape::Vec(v) => v.iter().for_each(|s| collect(s, out)),
        _ => {}
    }
}

fn text_rect(h: &H, label: &str) -> Option<Rect> {
    let mut v = Vec::new();
    for cs in &h.output().shapes {
        collect(&cs.shape, &mut v);
    }
    v.into_iter().rev().find(|(t, r)| t == label && r.is_positive()).map(|(_, r)| r)
}

/// Erstes Hauptmenü mit aktivem Untermenü: (Menünummer, Eintragsnummer, Beschriftung).
fn menu_with_sub(h: &H) -> (usize, usize, String) {
    let menus = menu::build_main_menu(&h.state().state);
    let (fit, _) = h.ctx.data(|d| d.get_temp::<(usize, usize)>(Id::new(menu::MENU_FIT_ID))).unwrap_or((menus.len(), menus.len()));
    for (i, (_, nodes)) in menus.iter().enumerate().take(fit) {
        for (j, n) in nodes.iter().enumerate() {
            if let Node::Sub { label, enabled: true, .. } = n {
                return (i, j, label.clone());
            }
        }
    }
    panic!("kein Menü mit Untermenü");
}

#[test]
fn esc_after_mouse_open_closes_one_level_each() {
    let mut h = harness();
    assert_eq!(selected(&h), 1);
    let (i, j, label) = menu_with_sub(&h);
    let t = titles(&h)[i].center();
    click(&mut h, t);
    assert!(any_menu_open(&h), "Menü per Maus offen");
    let r = text_rect(&h, &label).unwrap_or_else(|| panic!("„{label}“ sichtbar"));
    ev(&mut h, Event::PointerMoved(r.center()));
    h.run_steps(8);
    let path: Vec<usize> = h.ctx.data(|d| d.get_temp(Id::new(menu::MENU_MOUSE_PATH_ID))).unwrap_or_default();
    assert_eq!(path, vec![j], "Untermenü „{label}“ offen");
    // Esc 1: Untermenü zu, Menü bleibt offen (Kopf des Untermenüs markiert)
    key(&mut h, Key::Escape);
    assert_eq!(kbd(&h), (true, i, vec![j]), "Esc 1");
    assert!(!any_menu_open(&h), "Maus-Menü in die Tastaturbedienung übernommen");
    assert_eq!(selected(&h), 1, "Esc 1 lässt die Auswahl");
    // Esc 2: Menü zu, Leiste aktiv
    key(&mut h, Key::Escape);
    assert_eq!(kbd(&h), (true, i, vec![]), "Esc 2");
    assert_eq!(selected(&h), 1, "Esc 2 lässt die Auswahl");
    // Esc 3: Leiste aus
    key(&mut h, Key::Escape);
    assert!(!kbd(&h).0, "Esc 3: Leiste aus");
    assert_eq!(selected(&h), 1, "Esc 3 lässt die Auswahl");
    // Escape 4 reaches the canvas: the selection stays
    key(&mut h, Key::Escape);
    assert_eq!(selected(&h), 1, "Escape 4 keeps the selection");
}

#[test]
fn esc_after_mouse_open_one_level_keeps_bar_active() {
    let mut h = harness();
    let t = titles(&h)[0].center();
    click(&mut h, t);
    assert!(any_menu_open(&h));
    key(&mut h, Key::Escape);
    assert_eq!(kbd(&h), (true, 0, vec![]), "Menü zu, Leiste aktiv");
    assert_eq!(selected(&h), 1);
    // ein Klick beendet die Tastaturbedienung und öffnet wieder per Maus (CT-A-05)
    let t = titles(&h)[1].center();
    click(&mut h, t);
    assert!(!kbd(&h).0 && egui::Popup::is_id_open(&h.ctx, Id::new(("fp-main-menu", 1))), "Klick öffnet das nächste Menü");
    key(&mut h, Key::Escape);
    key(&mut h, Key::Escape);
    assert_eq!(selected(&h), 1, "zwei Esc: Auswahl bleibt");
}

#[test]
fn enter_after_mouse_open_keeps_selection() {
    let mut h = harness();
    let (i, _, label) = menu_with_sub(&h);
    let t = titles(&h)[i].center();
    click(&mut h, t);
    let r = text_rect(&h, &label).expect("Untermenü sichtbar");
    ev(&mut h, Event::PointerMoved(r.center()));
    h.run_steps(8);
    key(&mut h, Key::Enter);
    assert_eq!(selected(&h), 1, "Enter auf dem Untermenükopf lässt die Auswahl");
    for _ in 0..3 {
        key(&mut h, Key::Escape);
    }
    assert_eq!(selected(&h), 1, "danach Esc × 3: Auswahl bleibt");
}
