//! B-9: Aktive Unterebene im Ebenen-Bedienfeld – Zeile hervorgehoben wie eine aktive Ebene, Klick auf
//! die Unterebenenzeile macht sie aktiv (neue Objekte landen darin), Klick auf die Ebenenzeile
//! wieder die Ebene.

use egui::{Event, Modifiers, Pos2};
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
    s.execute("layer.rename", json!({"name": "Hauptebene"})).expect("rename");
    s.execute("pi.layer.addSub", json!({"name": "Unter A"})).expect("sub");
    s.execute("select.none", json!({})).expect("none");
    s.set_tool(ToolId::Pointer).expect("Zeiger");
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.os_clipboard = false;
    app.state.sync_render = true;
    let mut h = Harness::builder().with_size(egui::vec2(1500.0, 950.0)).with_pixels_per_point(1.0).with_max_steps(8).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    for _ in 0..2 {
        if find(&h, "Unter A").is_some() {
            break;
        }
        let p = find(&h, "Ebenen").expect("Karte Ebenen");
        click(&mut h, p);
        h.run_steps(30);
    }
    h
}

fn ev(h: &mut H, e: Event) {
    h.input_mut().events.push(e);
    h.step();
}

fn click(h: &mut H, p: Pos2) {
    h.run_steps(30);
    ev(h, Event::PointerMoved(p));
    let press = |down| Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: down, modifiers: Modifiers::NONE };
    ev(h, press(true));
    ev(h, press(false));
    // Zeiger weg, damit keine Hover-Hervorhebung bleibt
    ev(h, Event::PointerMoved(Pos2::new(5.0, 940.0)));
    h.run_steps(5);
}

fn texts(shape: &egui::epaint::Shape, out: &mut Vec<(String, egui::Rect)>) {
    match shape {
        egui::epaint::Shape::Text(t) => out.push((t.galley.job.text.clone(), egui::Rect::from_min_size(t.pos, t.galley.size()))),
        egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| texts(s, out)),
        _ => {}
    }
}

fn find(h: &H, label: &str) -> Option<Pos2> {
    let mut all = Vec::new();
    for cs in &h.output().shapes {
        texts(&cs.shape, &mut all);
    }
    all.iter().filter(|(t, _)| t == label).map(|(_, r)| r.center()).max_by(|a, b| a.x.total_cmp(&b.x))
}

fn fills(shape: &egui::epaint::Shape, col: egui::Color32, out: &mut Vec<egui::Rect>) {
    match shape {
        egui::epaint::Shape::Rect(r) if r.fill == col => out.push(r.rect),
        egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| fills(s, col, out)),
        _ => {}
    }
}

/// Liegt die Beschriftung `label` auf einer hervorgehobenen Zeile (Farbe „ausgewählt“)?
fn highlighted(h: &H, label: &str) -> bool {
    let p = find(h, label).expect(label);
    let col = fp_ui::theme::Theme::current(&h.ctx).colors.list_selected_bg;
    let mut rs = Vec::new();
    for cs in &h.output().shapes {
        fills(&cs.shape, col, &mut rs);
    }
    rs.iter().any(|r| r.contains(p))
}

fn active(h: &H) -> fp_model::LayerId {
    h.state().state.session.active().and_then(|d| d.layer).expect("aktiv")
}

fn sub_id(h: &H) -> fp_model::LayerId {
    let d = h.state().state.session.active().expect("d");
    for (_, l) in fp_engine::docops::own_layers(&d.doc, d.page) {
        for e in l.cels.first().map(|c| c.elements.clone()).unwrap_or_default() {
            if let fp_model::ElementKind::SubLayer(sl) = &e.kind {
                return sl.id;
            }
        }
    }
    panic!("Unterebene")
}

#[test]
fn active_sublayer_is_highlighted_and_click_activates_it() {
    let mut h = harness();
    let sub = sub_id(&h);
    // „Neue Unterebene“ hat sie aktiv gemacht: Zeile hervorgehoben, Ebenenzeile nicht
    assert_eq!(active(&h), sub);
    assert!(highlighted(&h, "Unter A"), "aktive Unterebene hervorgehoben");
    assert!(!highlighted(&h, "Hauptebene"), "Ebene der obersten Stufe nicht hervorgehoben");
    // Klick auf die Ebenenzeile: die Ebene wird aktiv
    let p = find(&h, "Hauptebene").expect("Ebenenzeile");
    click(&mut h, p);
    assert_ne!(active(&h), sub);
    assert!(highlighted(&h, "Hauptebene"));
    // (die Unterebenenzeile bleibt hier hervorgehoben: sie gehört zu den ausgewählten Objekten der Ebene)
    // Klick auf die Unterebenenzeile: aktiv, hervorgehoben, Auswahl = ihre (keine) Objekte
    let p = find(&h, "Unter A").expect("Unterebenenzeile");
    click(&mut h, p);
    assert_eq!(active(&h), sub);
    assert!(highlighted(&h, "Unter A"));
    assert!(!highlighted(&h, "Hauptebene"));
    // neue Objekte landen in der aktiven Unterebene
    h.state_mut().state.session.execute("draw.ellipse", json!({"rect": [200, 100, 260, 160]})).expect("Ellipse");
    let d = h.state().state.session.active().expect("d");
    let l = fp_engine::docops::layer_by_id(&d.doc, d.page, sub).expect("Unterebene");
    assert_eq!(l.cel(d.frame).map(|c| c.elements.len()), Some(1));
    // Ebenenbefehle mit Vorgabe „aktive Ebene“ arbeiten mit der Unterebene
    h.state_mut().state.session.execute("layer.duplicate", json!({})).expect("duplizieren");
    h.state_mut().state.session.execute("layer.move", json!({"index": 0})).expect("verschieben");
    h.state_mut().state.session.execute("layer.setShared", json!({"shared": true})).expect("teilen");
}
