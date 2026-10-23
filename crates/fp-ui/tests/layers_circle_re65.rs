//! RE spec 65 in the Layers panel: a click on a layer's selection circle moves the selection to the
//! top of that layer (Alt: copies) as one "Layer Change" row and makes the layer current (TW-217);
//! a filled circle, a locked layer or an empty selection do nothing, and the click never falls
//! through to the row click. The circle is grey on every layer without objects of its own.

use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use fp_engine::tools::ToolId;
use fp_ui::FirepixelApp;
use fp_ui::app_state::UiSettings;
use serde_json::{Value, json};

type H = Harness<'static, FirepixelApp>;

fn ex(s: &mut fp_engine::Session, c: &str, p: Value) -> Value {
    s.execute(c, p).unwrap_or_else(|e| panic!("{c}: {e}")).value
}

/// L1 {r1}, L2 {r2}, L3 {r3}; L3 current, nothing selected.
fn doc() -> (fp_engine::Session, [u64; 3], [u64; 3]) {
    fp_i18n::set_language("de");
    let mut s = fp_engine::Session::default();
    ex(&mut s, "file.new", json!({"width": 600, "height": 400, "background": "#ffffff"}));
    let l1 = s.active().and_then(|d| d.layer).map(|l| l.0).expect("layer");
    ex(&mut s, "layer.rename", json!({"name": "L1"}));
    let mut ls = [l1, 0, 0];
    let mut rs = [0u64; 3];
    for i in 0..3 {
        if i > 0 {
            ls[i] = ex(&mut s, "layer.add", json!({"name": format!("L{}", i + 1)}))["layer"].as_u64().expect("layer");
        }
        let x = 50.0 + 150.0 * i as f64;
        rs[i] = ex(&mut s, "draw.rectangle", json!({"rect": [x, 50.0, x + 80.0, 110.0]}))["id"].as_u64().expect("id");
        ex(&mut s, "element.setName", json!({"ids": [rs[i]], "name": format!("r{}", i + 1)}));
    }
    ex(&mut s, "select.none", json!({}));
    (s, ls, rs)
}

fn harness(mut s: fp_engine::Session) -> H {
    s.set_tool(ToolId::Pointer).expect("pointer");
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.os_clipboard = false;
    app.state.sync_render = true;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1500.0, 950.0))
        .with_pixels_per_point(1.0)
        .with_max_steps(8)
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    for _ in 0..2 {
        if find(&h, "L1").is_some() {
            break;
        }
        let tab = find(&h, "Ebenen").expect("Layers tab");
        click_at(&mut h, tab, Modifiers::NONE);
        h.run_steps(30);
    }
    h.run_steps(10);
    h
}

fn ev(h: &mut H, e: Event) {
    h.input_mut().events.push(e);
    h.step();
}

fn click_at(h: &mut H, p: Pos2, modifiers: Modifiers) {
    h.run_steps(30);
    h.input_mut().events.push(Event::ModifiersChanged(modifiers));
    ev(h, Event::PointerMoved(p));
    let e = |down| Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: down, modifiers };
    ev(h, e(true));
    ev(h, e(false));
    h.input_mut().events.push(Event::ModifiersChanged(Modifiers::NONE));
    ev(h, Event::PointerMoved(Pos2::new(5.0, 5.0)));
    h.run_steps(5);
}

fn texts(shape: &egui::epaint::Shape, out: &mut Vec<(String, egui::Rect)>) {
    match shape {
        egui::epaint::Shape::Text(t) => out.push((t.galley.job.text.clone(), egui::Rect::from_min_size(t.pos, t.galley.size()))),
        egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| texts(s, out)),
        _ => {}
    }
}

/// Label in the right part of the window (the Layers panel), rightmost match.
fn find(h: &H, label: &str) -> Option<Pos2> {
    let mut all = Vec::new();
    for cs in &h.output().shapes {
        texts(&cs.shape, &mut all);
    }
    all.into_iter().filter(|(t, r)| t == label && r.center().x > 900.0).map(|(_, r)| r.center()).max_by(|a, b| a.x.total_cmp(&b.x))
}

fn circles(shape: &egui::epaint::Shape, out: &mut Vec<egui::epaint::CircleShape>) {
    match shape {
        egui::epaint::Shape::Circle(c) => out.push(*c),
        egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| circles(s, out)),
        _ => {}
    }
}

/// The selection circle of the row `label`: (centre, filled, ring colour).
fn dot(h: &H, label: &str) -> (Pos2, bool, egui::Color32) {
    let p = find(h, label).unwrap_or_else(|| panic!("row {label}"));
    let mut cs = Vec::new();
    for s in &h.output().shapes {
        circles(&s.shape, &mut cs);
    }
    let row: Vec<_> = cs.iter().filter(|c| (c.center.y - p.y).abs() < 6.0 && c.center.x > p.x).collect();
    let ring = row.iter().find(|c| (c.radius - 4.5).abs() < 0.1).unwrap_or_else(|| panic!("circle of {label}"));
    let filled = row.iter().any(|c| (c.radius - 2.0).abs() < 0.1 && c.fill != egui::Color32::TRANSPARENT);
    (ring.center, filled, ring.stroke.color)
}

fn click_dot(h: &mut H, label: &str, m: Modifiers) {
    let p = dot(h, label).0;
    click_at(h, p, m);
}

struct State {
    layer: Option<u64>,
    sel: Vec<u64>,
    rows: usize,
    per_layer: Vec<Vec<u64>>,
}

fn state(h: &H, ls: &[u64]) -> State {
    let d = h.state().state.session.active().expect("doc");
    let per_layer = ls
        .iter()
        .map(|l| {
            fp_engine::docops::layer_by_id(&d.doc, d.page, fp_model::LayerId(*l))
                .and_then(|x| x.cel(d.frame).map(|c| c.elements.iter().map(|e| e.id.0).collect()))
                .unwrap_or_default()
        })
        .collect();
    State {
        layer: d.layer.map(|l| l.0),
        sel: d.selection.elements.iter().map(|e| e.0).collect(),
        rows: d.history.len(),
        per_layer,
    }
}

fn run(h: &mut H, c: &str, p: Value) {
    h.state_mut().state.session.execute(c, p).unwrap_or_else(|e| panic!("{c}: {e}"));
    h.run_steps(5);
}

#[test]
fn circle_click_moves_the_selection_into_the_layer() {
    let (s, ls, rs) = doc();
    let mut h = harness(s);
    run(&mut h, "select.set", json!({"ids": [rs[0]]}));
    let before = state(&h, &ls);
    assert_eq!(before.layer, Some(ls[0]));
    // Shift changes nothing: a plain move
    click_dot(&mut h, "L3", Modifiers::SHIFT);
    let after = state(&h, &ls);
    assert_eq!(after.per_layer[2], [rs[0], rs[2]], "r1 on top of L3");
    assert!(after.per_layer[0].is_empty());
    assert_eq!(after.sel, [rs[0]]);
    assert_eq!(after.layer, Some(ls[2]), "the target becomes current");
    assert_eq!(after.rows, before.rows + 1);
    let d = h.state().state.session.active().expect("doc");
    assert_eq!(d.history.labels().last().map(String::as_str), Some("Ebenenänderung"));
    assert_eq!(d.history.top().map(|t| t.command.as_str()), Some("selection.moveToLayer"));
}

#[test]
fn alt_click_on_the_circle_copies() {
    let (s, ls, rs) = doc();
    let mut h = harness(s);
    run(&mut h, "select.set", json!({"ids": [rs[0]]}));
    let before = state(&h, &ls);
    click_dot(&mut h, "L2", Modifiers::ALT);
    let after = state(&h, &ls);
    assert_eq!(after.per_layer[0], [rs[0]], "the original stays in L1");
    assert_eq!(after.per_layer[1].len(), 2);
    assert_eq!(after.per_layer[1][1], rs[1]);
    let copy = after.per_layer[1][0];
    assert_ne!(copy, rs[0]);
    assert_eq!(after.sel, [copy], "the copy is selected");
    assert_eq!(after.layer, Some(ls[1]));
    assert_eq!(after.rows, before.rows + 1);
}

#[test]
fn circle_click_does_nothing_on_filled_locked_or_without_selection() {
    let (s, ls, rs) = doc();
    let mut h = harness(s);
    // nothing selected: no move and no row click (the row would make L1 current and select r1)
    let before = state(&h, &ls);
    click_dot(&mut h, "L1", Modifiers::NONE);
    let after = state(&h, &ls);
    assert_eq!((after.layer, after.sel.clone(), after.rows), (before.layer, before.sel.clone(), before.rows));
    assert_eq!(after.layer, Some(ls[2]));
    // filled circles: [r1, r3] selected, L3 and L1 hold one each
    run(&mut h, "select.set", json!({"ids": [rs[0], rs[2]]}));
    let before = state(&h, &ls);
    assert!(dot(&h, "L3").1 && dot(&h, "L1").1);
    click_dot(&mut h, "L3", Modifiers::NONE);
    click_dot(&mut h, "L1", Modifiers::ALT);
    let after = state(&h, &ls);
    assert_eq!(after.per_layer, before.per_layer);
    assert_eq!((after.layer, after.sel.clone(), after.rows), (before.layer, before.sel.clone(), before.rows));
    // a locked target layer
    run(&mut h, "layer.setLocked", json!({"layer": ls[1], "locked": true}));
    run(&mut h, "select.set", json!({"ids": [rs[0]]}));
    let before = state(&h, &ls);
    click_dot(&mut h, "L2", Modifiers::NONE);
    let after = state(&h, &ls);
    assert_eq!(after.per_layer, before.per_layer);
    assert_eq!((after.layer, after.sel, after.rows), (before.layer, before.sel, before.rows));
}

#[test]
fn circles_are_grey_without_objects_of_their_own() {
    let (s, ls, _) = doc();
    let mut h = harness(s);
    let col = fp_ui::theme::Theme::current(&h.ctx).colors.clone();
    assert_eq!(dot(&h, "L1").2, col.icon_fg);
    assert_eq!(dot(&h, "Webebene").2, col.icon_disabled, "the empty web layer");
    // a new empty layer, then a sublayer in it: the layer stays grey, the sublayer turns black
    // once it holds an object
    run(&mut h, "layer.add", json!({"name": "L4"}));
    assert_eq!(dot(&h, "L4").2, col.icon_disabled, "empty layer");
    run(&mut h, "pi.layer.addSub", json!({"name": "S4"}));
    assert_eq!(dot(&h, "S4").2, col.icon_disabled, "empty sublayer");
    run(&mut h, "draw.rectangle", json!({"rect": [300, 300, 340, 340]}));
    run(&mut h, "select.none", json!({}));
    assert_eq!(dot(&h, "S4").2, col.icon_fg);
    assert_eq!(dot(&h, "L4").2, col.icon_disabled, "only a sublayer: grey");
    // moving L3's only object away leaves L3 grey
    let r3 = state(&h, &ls).per_layer[2][0];
    run(&mut h, "select.set", json!({"ids": [r3]}));
    run(&mut h, "selection.moveToLayer", json!({"layer": ls[0]}));
    assert_eq!(dot(&h, "L3").2, col.icon_disabled);
    // the web layer with a slice is black, with the slice selected filled
    let sl = h.state_mut().state.session.execute("web.addSlice", json!({"shape": "rect", "rect": [500, 300, 560, 360]})).expect("slice").value["id"].as_u64().expect("id");
    run(&mut h, "select.none", json!({}));
    assert_eq!(dot(&h, "Webebene"), (dot(&h, "Webebene").0, false, col.icon_fg));
    run(&mut h, "select.set", json!({"ids": [sl]}));
    assert!(dot(&h, "Webebene").1, "the selected slice fills the circle");
    assert_eq!(dot(&h, "Webebene").2, col.icon_fg);
}
