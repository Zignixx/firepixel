//! FB-30 / TW-208 (RE spec 61 §3): multi-selection in the Layers panel. A click selects that row only
//! and sets the anchor; Shift+click selects the range anchor…row and replaces the selection; Ctrl+click
//! (Cmd on macOS) toggles the row and keeps the anchor; Ctrl+Shift+click behaves like Shift; a click on
//! a row of a multi-selection keeps it; a click on a layer row selects all objects of the layer.

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

/// Measured document (RE spec 61 §3.1): four rectangles O1…O4, O4 on top; nothing selected.
/// Returns the harness and the ids of O1…O4 (index 0 = O1).
fn harness() -> (H, [u64; 4]) {
    fp_i18n::set_language("de");
    let mut s = fp_engine::Session::default();
    ex(&mut s, "file.new", json!({"width": 600, "height": 400, "background": "#ffffff"}));
    let mut ids = [0u64; 4];
    for (i, id) in ids.iter_mut().enumerate() {
        let x = 20.0 + 110.0 * i as f64;
        let v = ex(&mut s, "draw.rectangle", json!({"rect": [x, 20.0, x + 100.0, 120.0]}));
        *id = v["id"].as_u64().expect("id");
        ex(&mut s, "element.setName", json!({"ids": [*id], "name": format!("O{}", i + 1)}));
    }
    ex(&mut s, "select.none", json!({}));
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
        if find(&h, "O1").is_some() {
            break;
        }
        let tab = find(&h, "Ebenen").expect("Layers tab");
        click_at(&mut h, tab, Modifiers::NONE);
        h.run_steps(30);
    }
    for n in ["O1", "O2", "O3", "O4", "Ebene 1"] {
        assert!(find(&h, n).is_some(), "Layers panel shows row {n}");
    }
    (h, ids)
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
    ev(h, Event::PointerMoved(Pos2::new(5.0, 940.0)));
    h.run_steps(5);
}

/// Click on the name of a row of the Layers panel.
fn click(h: &mut H, label: &str, modifiers: Modifiers) {
    let p = find(h, label).unwrap_or_else(|| panic!("row {label}"));
    click_at(h, p, modifiers);
}

fn texts(shape: &egui::epaint::Shape, out: &mut Vec<(String, egui::Rect)>) {
    match shape {
        egui::epaint::Shape::Text(t) => out.push((t.galley.job.text.clone(), egui::Rect::from_min_size(t.pos, t.galley.size()))),
        egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| texts(s, out)),
        _ => {}
    }
}

/// Text in the right part of the window (the Layers panel), rightmost match.
fn find(h: &H, label: &str) -> Option<Pos2> {
    let mut all = Vec::new();
    for cs in &h.output().shapes {
        texts(&cs.shape, &mut all);
    }
    all.iter().filter(|(t, r)| t == label && r.center().x > 900.0).map(|(_, r)| r.center()).max_by(|a, b| a.x.total_cmp(&b.x))
}

fn fills(shape: &egui::epaint::Shape, col: egui::Color32, out: &mut Vec<egui::Rect>) {
    match shape {
        egui::epaint::Shape::Rect(r) if r.fill == col => out.push(r.rect),
        egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| fills(s, col, out)),
        _ => {}
    }
}

fn highlighted(h: &H, label: &str) -> bool {
    let p = find(h, label).expect(label);
    let col = fp_ui::theme::Theme::current(&h.ctx).colors.list_selected_bg;
    let mut rs = Vec::new();
    for cs in &h.output().shapes {
        fills(&cs.shape, col, &mut rs);
    }
    rs.iter().any(|r| r.contains(p))
}

/// Current selection as sorted ids.
fn selection(h: &H) -> Vec<u64> {
    let mut v: Vec<u64> = h.state().state.session.active().expect("d").selection.elements.iter().map(|e| e.0).collect();
    v.sort();
    v
}

/// Sorted ids of the objects O<n> given by number.
fn objs(ids: &[u64; 4], ns: &[usize]) -> Vec<u64> {
    let mut v: Vec<u64> = ns.iter().map(|n| ids[n - 1]).collect();
    v.sort();
    v
}

const SHIFT: Modifiers = Modifiers::SHIFT;
/// Ctrl on Windows/Linux (egui sets `ctrl` and `command`).
const CTRL: Modifiers = Modifiers { alt: false, ctrl: true, shift: false, mac_cmd: false, command: true };
const CTRL_SHIFT: Modifiers = Modifiers { alt: false, ctrl: true, shift: true, mac_cmd: false, command: true };
/// Cmd on macOS (egui sets `mac_cmd` and `command`).
const MAC_CMD: Modifiers = Modifiers { alt: false, ctrl: false, shift: false, mac_cmd: true, command: true };

#[test]
fn panel_layers_shift_click_selects_range() {
    let (mut h, o) = harness();
    click(&mut h, "O4", Modifiers::NONE);
    assert_eq!(selection(&h), objs(&o, &[4]));
    click(&mut h, "O1", SHIFT);
    assert_eq!(selection(&h), objs(&o, &[4, 3, 2, 1]), "Shift+click three rows below: four rows selected");
    for n in ["O1", "O2", "O3", "O4"] {
        assert!(highlighted(&h, n), "row {n} highlighted");
    }
}

#[test]
fn panel_layers_ctrl_click_toggles_and_keeps_anchor() {
    let (mut h, o) = harness();
    click(&mut h, "O4", Modifiers::NONE);
    click(&mut h, "O1", SHIFT);
    click(&mut h, "O2", CTRL);
    assert_eq!(selection(&h), objs(&o, &[4, 3, 1]), "Ctrl+click takes O2 out");
    assert!(!highlighted(&h, "O2"));
    click(&mut h, "O2", CTRL);
    assert_eq!(selection(&h), objs(&o, &[4, 3, 2, 1]), "Ctrl+click puts O2 back");
    click(&mut h, "O3", SHIFT);
    assert_eq!(selection(&h), objs(&o, &[4, 3]), "range from the anchor O4: Ctrl-clicks did not move it");
}

#[test]
fn panel_layers_ctrl_shift_click_acts_like_shift() {
    let (mut h, o) = harness();
    click(&mut h, "O1", Modifiers::NONE);
    click(&mut h, "O4", CTRL);
    assert_eq!(selection(&h), objs(&o, &[4, 1]));
    click(&mut h, "O2", CTRL_SHIFT);
    assert_eq!(selection(&h), objs(&o, &[2, 1]), "Ctrl+Shift: range from O1, replaces (O4 falls out)");
}

#[test]
fn panel_layers_click_on_selected_row_keeps_multi_selection() {
    let (mut h, o) = harness();
    click(&mut h, "O1", Modifiers::NONE);
    click(&mut h, "O4", SHIFT);
    assert_eq!(selection(&h), objs(&o, &[4, 3, 2, 1]));
    click(&mut h, "O3", Modifiers::NONE);
    assert_eq!(selection(&h), objs(&o, &[4, 3, 2, 1]), "a click on a selected row keeps the multi-selection");
    click(&mut h, "O3", SHIFT);
    assert_eq!(selection(&h), objs(&o, &[3, 2, 1]), "range from the anchor O1");
    // a click on a row outside the selection selects only that row
    click(&mut h, "O4", Modifiers::NONE);
    assert_eq!(selection(&h), objs(&o, &[4]));
}

#[test]
fn panel_layers_layer_row_selects_all_its_objects() {
    let (mut h, o) = harness();
    click(&mut h, "Ebene 1", Modifiers::NONE);
    assert_eq!(selection(&h), objs(&o, &[4, 3, 2, 1]), "layer row: all objects of the layer");
    // the layer row is the anchor now: Shift+click O2 ranges from it
    click(&mut h, "O2", SHIFT);
    assert_eq!(selection(&h), objs(&o, &[4, 3, 2]));
}

#[test]
fn panel_layers_mac_cmd_toggles_like_ctrl() {
    let (mut h, o) = harness();
    click(&mut h, "O4", Modifiers::NONE);
    click(&mut h, "O2", MAC_CMD);
    assert_eq!(selection(&h), objs(&o, &[4, 2]), "Cmd+click (macOS) adds the row");
    // the Ctrl key alone (as on a Mac, where it does not set `command`) toggles as well
    click(&mut h, "O3", Modifiers::CTRL);
    assert_eq!(selection(&h), objs(&o, &[4, 3, 2]));
    click(&mut h, "O4", MAC_CMD);
    assert_eq!(selection(&h), objs(&o, &[3, 2]), "Cmd+click takes the row out");
    // anchor still O4 (Cmd-clicks do not move it)
    click(&mut h, "O3", SHIFT);
    assert_eq!(selection(&h), objs(&o, &[4, 3]));
}
