//! T-3: Soll-Regeln aus der Mess-Sitzung re 29 (M4), Oberfläche.

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::{By, NodeT, Queryable};
use egui_kittest::Harness;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::menu::{ItemAction, Node};
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn harness_with(s: fp_engine::Session) -> H {
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).with_pixels_per_point(1.0).with_max_steps(12).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(3);
    h
}

fn session() -> fp_engine::Session {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 400, "height": 300, "background": "#ffffff"})).expect("doc");
    s
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn ev(h: &mut H, e: Event) {
    h.event(e);
    h.run_steps(1);
}

fn key_m(h: &mut H, k: Key, m: Modifiers) {
    if m.any() {
        h.event(Event::ModifiersChanged(m));
    }
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: m });
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: false, repeat: false, modifiers: m });
    if m.any() {
        ev(h, Event::ModifiersChanged(Modifiers::NONE));
    }
    h.run_steps(2);
}

fn key(h: &mut H, k: Key) {
    key_m(h, k, Modifiers::NONE);
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

fn sliders(h: &H) -> Vec<(egui::Rect, f64)> {
    h.query_all(By::new().role(Role::Slider)).map(|n| (n.rect(), n.accesskit_node().numeric_value().unwrap_or(f64::NAN))).collect()
}

fn text_fields(h: &H) -> Vec<(egui::Rect, String)> {
    let dr = fp_ui::dialogs::dialog_rect(&h.ctx, &fp_ui::dialogs::last_dialog_id(&h.ctx).expect("Dialog")).expect("Rechteck");
    let mut v: Vec<(egui::Rect, String)> = h.get_all(By::new().role(Role::TextInput)).map(|n| (n.rect(), n.value().unwrap_or_default())).filter(|(r, _)| dr.contains(r.center())).collect();
    v.sort_by(|a, b| a.0.top().total_cmp(&b.0.top()));
    v
}

fn field(h: &H, key: &str) -> f64 {
    st(h).dialogs.form.as_ref().expect("Formular").get(key).as_f64().unwrap_or(f64::NAN)
}

/// BA-019 (re 29 §8): ▾ neben beiden Abständen, 1 … 100 linear, senkrecht 212 px, 100 oben, der
/// aktuelle Wert liegt beim Öffnen unter dem Zeiger, das Feld zeigt den Wert beim Ziehen.
#[test]
fn ba019_grid_popup_sliders() {
    let mut h = harness_with(session());
    h.state_mut().state.run("view.grid.editGrid", json!({}));
    h.run_steps(3);
    assert!(st(&h).dialogs.form.is_some(), "Raster bearbeiten offen");
    assert_eq!(text_fields(&h).len(), 2, "zwei Abstandsfelder");
    let (pad, track) = (fp_ui::inspector::fields::VSLIDER_PAD, fp_ui::inspector::fields::VSLIDER_TRACK);
    assert_eq!(track, 212.0);
    for (i, key) in ["width", "height"].into_iter().enumerate() {
        let f = text_fields(&h)[i].0;
        let v0 = field(&h, key);
        click(&mut h, Pos2::new(f.right() + 8.0, f.center().y));
        let s = sliders(&h);
        assert_eq!(s.len(), 1, "{key}: Popup offen");
        let (r, _) = s[0];
        let knob = r.top() + pad + (1.0 - ((v0.min(100.0) - 1.0) / 99.0) as f32) * track;
        assert!((knob - f.center().y).abs() <= 3.0, "{key}: Wert {v0} unter dem Zeiger ({knob} / {})", f.center().y);
        // ziehen bis über das obere Ende: live 100 im Feld
        let (top, bottom) = (r.top() + pad, r.top() + pad + track);
        let x = r.center().x;
        ev(&mut h, Event::PointerMoved(Pos2::new(x, knob)));
        press(&mut h, Pos2::new(x, knob), true);
        let mut live = Vec::new();
        for k in 1..=20 {
            let y = knob + (top - 30.0 - knob) * k as f32 / 20.0;
            ev(&mut h, Event::PointerMoved(Pos2::new(x, y)));
            live.push(text_fields(&h)[i].1.clone());
        }
        press(&mut h, Pos2::new(x, top - 30.0), false);
        h.run_steps(2);
        assert_eq!(live.last().map(String::as_str), Some("100"), "{key}: live {live:?}");
        // Mitte der Spur ≈ 50 (linear)
        let mid = (top + bottom) / 2.0;
        ev(&mut h, Event::PointerMoved(Pos2::new(x, top)));
        press(&mut h, Pos2::new(x, top), true);
        for k in 1..=10 {
            ev(&mut h, Event::PointerMoved(Pos2::new(x, top + (mid - top) * k as f32 / 10.0)));
        }
        press(&mut h, Pos2::new(x, mid), false);
        h.run_steps(2);
        let shown: f64 = text_fields(&h)[i].1.parse().unwrap_or(f64::NAN);
        assert!((49.0..=52.0).contains(&shown), "{key}: Mitte {shown}");
        // Klick daneben übernimmt
        click(&mut h, Pos2::new(f.left() + 4.0, f.center().y));
        assert!(sliders(&h).is_empty());
        assert!((49.0..=52.0).contains(&field(&h, key)), "{key}: {}", field(&h, key));
    }
}

fn text_and_rect() -> (fp_engine::Session, u64, u64) {
    let mut s = session();
    let t = s.execute("draw.text", json!({"text": "Abc", "anchor": [40, 60]})).expect("Text").value["id"].as_u64().expect("id");
    let r = s.execute("draw.rectangle", json!({"rect": [200, 100, 300, 200]})).expect("rect").value["id"].as_u64().expect("id");
    (s, t, r)
}

fn selection(h: &H) -> Vec<u64> {
    st(h).session.active().map(|d| d.selection.elements.iter().map(|e| e.0).collect()).unwrap_or_default()
}

/// TW-57 (re 29 §5): Text ▸ Ausrichten per Tastatur (Alt+T, U, ↓, Eingabe) wirkt wie der Mausklick;
/// die Eingabetaste erreicht die Leinwand nicht, der Text bleibt gewählt; eine gemischte Auswahl
/// wird auf den Text reduziert.
#[test]
fn tw57_keyboard_menu_keeps_selection() {
    let (s, t, r) = text_and_rect();
    let mut h = harness_with(s);
    h.state_mut().state.run("select.set", json!({"ids": [t]}));
    h.run_steps(2);
    key_m(&mut h, Key::T, Modifiers::ALT);
    key(&mut h, Key::U);
    key(&mut h, Key::ArrowDown);
    key(&mut h, Key::ArrowDown);
    key(&mut h, Key::Enter);
    h.run_steps(3);
    assert_eq!(st(&h).session.command_state("text.align.right").1, Some(true), "rechts");
    assert_eq!(selection(&h), vec![t], "Auswahl bleibt");
    h.state_mut().state.run("select.set", json!({"ids": [t, r]}));
    h.run_steps(2);
    key_m(&mut h, Key::T, Modifiers::ALT);
    key(&mut h, Key::U);
    key(&mut h, Key::ArrowDown);
    key(&mut h, Key::Enter);
    h.run_steps(3);
    assert_eq!(st(&h).session.command_state("text.align.centeredHorizontally").1, Some(true), "zentriert");
    assert_eq!(selection(&h), vec![t], "R fällt heraus");
}

fn find_item<'a>(nodes: &'a [Node], cmd: &str) -> Option<&'a Node> {
    nodes.iter().find_map(|n| match n {
        Node::Item { action: ItemAction::Command(c), .. } if c == cmd => Some(n),
        Node::Sub { children, .. } => find_item(children, cmd),
        _ => None,
    })
}

/// CT-A-13 (re 29 §2): „Vollbildmodus beenden“ steht im Leinwand-Kontextmenü auch im Normalmodus,
/// grau; „Richtung umkehren“ gibt es nur unter Text, nicht unter Modifizieren ▸ Pfad ändern (TW-57).
#[test]
fn ct_a13_exit_full_screen_visible_and_grey() {
    let h = harness_with(session());
    let nodes = fp_ui::canvas::context::build(st(&h), fp_ui::canvas::context::entries("6914"));
    match find_item(&nodes, "view.exitFullScreen") {
        Some(Node::Item { enabled, .. }) => assert!(!enabled, "grau im Normalmodus"),
        _ => panic!("Eintrag fehlt"),
    }
    let menus = fp_ui::menu::build_main_menu_where(st(&h), &|_| true);
    let with_rev: Vec<&String> = menus.iter().filter(|(_, n)| find_item(n, "text.reverseDirection").is_some()).map(|(t, _)| t).collect();
    assert_eq!(with_rev.len(), 1, "nur in einem Menü: {with_rev:?}");
}
