//! MN-6: Menüs, die höher als das Fenster sind, wie native Windows-Menüs des Originals: auf das
//! Fenster begrenzt, Pfeilleisten oben/unten (Verweilen und Klick blättern, Klick schließt nicht),
//! Mausrad, Tastatur scrollt den markierten Eintrag in Sicht; Kontextmenüs bleiben im Fenster.

use egui::{Event, Id, Key, Modifiers, Order, PointerButton, Pos2, Rect, Shape};
use egui_kittest::Harness;
use fp_engine::Session;
use fp_ui::app_state::UiSettings;
use fp_ui::menu::{self, Node};
use fp_ui::menu_scroll::ARROW_H;
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn harness(w: f32, h: f32, docs: usize) -> H {
    let ctx = egui::Context::default();
    let mut s = Session::default();
    for _ in 0..docs {
        s.execute("file.new", json!({"width": 300, "height": 200})).expect("doc");
    }
    let a = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    let mut h = Harness::builder().with_size(egui::vec2(w, h)).with_pixels_per_point(1.0).with_max_steps(16).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), a);
    h.run_steps(3);
    h
}

fn collect(shape: &Shape, clip: Rect, out: &mut Vec<(String, Rect, Rect)>) {
    match shape {
        Shape::Text(t) => out.push((t.galley.text().to_string(), t.visual_bounding_rect(), clip)),
        Shape::Vec(v) => v.iter().for_each(|s| collect(s, clip, out)),
        _ => {}
    }
}

/// Rechteck des zuletzt gezeichneten, ganz sichtbaren Texts `label` (im Clip und im Fenster).
fn visible(h: &H, label: &str) -> Option<Rect> {
    let screen = h.ctx.content_rect().expand(0.5);
    let mut v = Vec::new();
    for cs in &h.output().shapes {
        collect(&cs.shape, cs.clip_rect, &mut v);
    }
    v.into_iter().rev().find(|(t, r, c)| t == label && r.is_positive() && c.expand(0.5).contains_rect(*r) && screen.contains_rect(*r)).map(|(_, r, _)| r)
}

/// Rechteck der obersten Menü- bzw. Popup-Fläche.
fn top_menu(h: &H) -> Rect {
    h.ctx.memory(|m| m.areas().top_layer_id(Order::Foreground).and_then(|l| m.area_rect(l.id))).expect("Menü offen")
}

/// Alle sichtbaren Popups liegen im Fenster.
fn assert_menus_inside(h: &H, what: &str) {
    let screen = h.ctx.content_rect().expand(0.5);
    let rects: Vec<Rect> = h.ctx.memory(|m| m.areas().visible_layer_ids().into_iter().filter(|l| l.order == Order::Foreground).filter_map(|l| m.area_rect(l.id)).collect());
    assert!(!rects.is_empty(), "{what}: kein Menü offen");
    for r in rects {
        assert!(screen.contains_rect(r), "{what}: Menü {r:?} ragt aus dem Fenster {screen:?}");
    }
}

fn move_to(h: &mut H, p: Pos2) {
    h.event(Event::PointerMoved(p));
}

fn click(h: &mut H, p: Pos2, button: PointerButton) {
    h.event(Event::PointerMoved(p));
    for pressed in [true, false] {
        h.event(Event::PointerButton { pos: p, button, pressed, modifiers: Modifiers::NONE });
    }
}

fn key(h: &mut H, k: Key) {
    for pressed in [true, false] {
        h.event(Event::Key { key: k, physical_key: None, pressed, repeat: false, modifiers: Modifiers::NONE });
    }
    h.run_steps(2);
}

/// Beschriftung des ersten bzw. letzten aktiven Eintrags.
fn edge_labels(nodes: &[Node]) -> (String, String) {
    let labels: Vec<String> = nodes
        .iter()
        .filter_map(|n| match n {
            Node::Item { label, enabled: true, .. } | Node::Sub { label, enabled: true, .. } => Some(label.clone()),
            _ => None,
        })
        .collect();
    (labels.first().cloned().expect("Eintrag"), labels.last().cloned().expect("Eintrag"))
}

/// Öffnet Hauptmenü `i` (im schmalen Fenster über das Überlaufmenü „»“).
fn open_main(h: &mut H, i: usize) -> (String, Vec<Node>) {
    let menus = menu::build_main_menu(&h.state().state);
    let (title, nodes) = menus[i].clone();
    let (fit, _) = h.ctx.data(|d| d.get_temp::<(usize, usize)>(Id::new(menu::MENU_FIT_ID))).unwrap_or((menus.len(), menus.len()));
    if i < fit {
        egui::Popup::open_id(&h.ctx, Id::new(("fp-main-menu", i)));
        h.run_steps(3);
    } else {
        egui::Popup::open_id(&h.ctx, Id::new(("fp-main-menu", fit)));
        h.run_steps(3);
        let r = visible(h, &title).unwrap_or_else(|| panic!("„{title}“ im Überlaufmenü"));
        move_to(h, r.center());
        h.run_steps(4);
    }
    (title, nodes)
}

/// Verweilt über der unteren Pfeilleiste, bis `label` ganz zu sehen ist.
fn hover_down_until(h: &mut H, label: &str) -> Rect {
    let m = top_menu(h);
    move_to(h, Pos2::new(m.center().x, m.bottom() - 3.0 - ARROW_H / 2.0));
    for _ in 0..400 {
        h.step();
        if let Some(r) = visible(h, label) {
            return r;
        }
    }
    panic!("„{label}“ nach Verweilen über dem unteren Pfeil nicht sichtbar");
}

#[test]
fn datei_letzter_eintrag_per_pfeil_erreichbar_und_ausloesbar() {
    let mut h = harness(800.0, 512.0, 1);
    let (_, nodes) = open_main(&mut h, 0);
    let (first, last) = edge_labels(&nodes);
    assert_menus_inside(&h, "Datei");
    assert!(visible(&h, &first).is_some(), "erster Eintrag „{first}“ sichtbar");
    assert!(visible(&h, &last).is_none(), "Datei passt bei 800 × 512 nicht ganz, „{last}“ erst nach Bildlauf");
    let m = top_menu(&h);
    // Klick auf die obere Pfeilleiste schließt das Menü nicht
    click(&mut h, Pos2::new(m.center().x, m.top() + 3.0 + ARROW_H / 2.0), PointerButton::Primary);
    h.run_steps(3);
    assert!(egui::Popup::is_id_open(&h.ctx, Id::new(("fp-main-menu", 0))), "Klick auf den Pfeil lässt das Menü offen");
    let r = hover_down_until(&mut h, &last);
    assert_menus_inside(&h, "Datei nach Bildlauf");
    assert!(visible(&h, &first).is_none(), "nach dem Bildlauf ist der Anfang verdeckt");
    // der Eintrag reagiert: „Beenden“ fragt bzw. schließt
    click(&mut h, r.center(), PointerButton::Primary);
    h.run_steps(4);
    let st = &h.state().state;
    assert!(st.quit_confirmed || st.dialogs.any_open(), "„{last}“ wurde ausgelöst");
    assert!(!egui::Popup::is_id_open(&h.ctx, Id::new(("fp-main-menu", 0))), "Menü nach dem Klick zu");
}

#[test]
fn datei_mit_mausrad_blaettern() {
    let mut h = harness(800.0, 512.0, 1);
    let (_, nodes) = open_main(&mut h, 0);
    let (first, last) = edge_labels(&nodes);
    let m = top_menu(&h);
    move_to(&mut h, m.center());
    let mut seen = false;
    for _ in 0..40 {
        h.event(Event::MouseWheel { unit: egui::MouseWheelUnit::Line, delta: egui::vec2(0.0, -1.0), phase: egui::TouchPhase::Move, modifiers: Modifiers::NONE });
        h.run_steps(3);
        if visible(&h, &last).is_some() {
            seen = true;
            break;
        }
    }
    assert!(seen, "Mausrad blättert bis „{last}“");
    for _ in 0..40 {
        h.event(Event::MouseWheel { unit: egui::MouseWheelUnit::Line, delta: egui::vec2(0.0, 1.0), phase: egui::TouchPhase::Move, modifiers: Modifiers::NONE });
        h.run_steps(3);
        if visible(&h, &first).is_some() {
            return;
        }
    }
    panic!("Mausrad blättert zurück bis „{first}“");
}

#[test]
fn fenster_letzter_eintrag_erreichbar_und_ausloesbar() {
    for (w, hh) in [(800.0, 512.0), (1024.0, 768.0)] {
        // zwei Dokumente: die Fensterliste am Ende des Menüs hat zwei Einträge
        let mut h = harness(w, hh, 2);
        let first_doc = h.state().state.session.documents()[0].id;
        let last_doc = h.state().state.session.documents()[1].id;
        h.state_mut().state.push(fp_ui::app_state::Action::Activate(first_doc));
        h.run_steps(3);
        let (_, nodes) = open_main(&mut h, 8);
        let (_, last) = edge_labels(&nodes);
        assert_menus_inside(&h, "Fenster");
        let r = match visible(&h, &last) {
            Some(r) => r,
            None => hover_down_until(&mut h, &last),
        };
        assert_menus_inside(&h, "Fenster nach Bildlauf");
        click(&mut h, r.center(), PointerButton::Primary);
        h.run_steps(4);
        assert_eq!(h.state().state.session.active_id(), Some(last_doc), "{w}×{hh}: „{last}“ aktiviert das zweite Dokument");
    }
}

#[test]
fn tastatur_scrollt_markierten_eintrag_in_sicht() {
    let mut h = harness(800.0, 512.0, 1);
    let nodes = menu::build_main_menu(&h.state().state)[0].1.clone();
    let (first, last) = edge_labels(&nodes);
    key(&mut h, Key::F10);
    key(&mut h, Key::ArrowDown);
    h.run_steps(4);
    assert!(visible(&h, &first).is_some(), "Datei per Tastatur offen, „{first}“ markiert");
    assert!(visible(&h, &last).is_none(), "„{last}“ zunächst außerhalb");
    let settle = |h: &mut H, label: &str, what: &str| {
        for _ in 0..60 {
            h.step();
            if visible(h, label).is_some() {
                return;
            }
        }
        panic!("{what}: „{label}“ nicht in Sicht gescrollt");
    };
    // Pfeil hoch vom ersten Eintrag springt auf den letzten
    key(&mut h, Key::ArrowUp);
    settle(&mut h, &last, "Pfeil hoch");
    key(&mut h, Key::Home);
    settle(&mut h, &first, "Pos1");
    key(&mut h, Key::End);
    settle(&mut h, &last, "Ende");
    // das Tastaturmenü bleibt im Fenster
    let r = h.ctx.memory(|m| m.area_rect(Id::new(("fp-menu-kbd-level", 0)))).expect("Tastaturmenü");
    assert!(h.ctx.content_rect().expand(0.5).contains_rect(r), "Tastaturmenü {r:?} im Fenster");
    key(&mut h, Key::Enter);
    h.run_steps(3);
    let st = &h.state().state;
    assert!(st.quit_confirmed || st.dialogs.any_open(), "Eingabe löst „{last}“ aus");
}

#[test]
fn kontextmenue_der_leinwand_bleibt_im_fenster() {
    let mut h = harness(800.0, 512.0, 1);
    let rect = |id: &str| egui::containers::panel::PanelState::load(&h.ctx, Id::new(id)).map(|p| p.outer_rect);
    let left = rect("fp-tools").map(|r| r.right()).unwrap_or(0.0);
    let top = rect("fp-toolbar").or(rect("fp-appbar")).map(|r| r.bottom()).unwrap_or(0.0);
    let right = rect("fp-dock").map(|r| r.left()).unwrap_or(800.0);
    let bottom = rect("fp-inspector").map(|r| r.top()).unwrap_or(512.0);
    // Rechtsklick auf die Leinwand (Mitte und weiter unten): das Menü (bei 800 × 512 höher als das
    // Fenster) wird begrenzt und ganz ins Fenster geschoben
    let p = Pos2::new((left + right) / 2.0, (top + bottom) / 2.0);
    click(&mut h, p, PointerButton::Secondary);
    h.run_steps(4);
    assert_menus_inside(&h, "Kontextmenü der Leinwand (Mitte)");
    h.event(Event::Key { key: Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE });
    h.run_steps(3);
    let p = Pos2::new((left + right) / 2.0, top + (bottom - top) * 0.85);
    click(&mut h, p, PointerButton::Secondary);
    h.run_steps(4);
    assert_menus_inside(&h, "Kontextmenü der Leinwand");
    let m = top_menu(&h);
    assert!(m.height() <= h.ctx.content_rect().height(), "höchstens fensterhoch: {m:?}");
}
