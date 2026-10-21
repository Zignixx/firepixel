//! Q-59 / TW-193 (RE spec 58 §3.2): context menu of the Layers panel. A right-click on an object row
//! selects that object (and highlights the row) before the menu forms its states: Flatten Selection
//! and Add Mask on for a path, a group and a bitmap; Merge Down only for an object directly above a
//! bitmap. A right-click on a layer row leaves the object selection alone (nothing selected: both
//! grey; an object selected: on) and makes the layer current; on the web layer Share Layer to Pages is
//! grey as well.

use egui::accesskit::Role;
use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::{By, NodeT, Queryable};
use fp_engine::tools::ToolId;
use fp_ui::FirepixelApp;
use fp_ui::app_state::UiSettings;
use serde_json::{Value, json};

type H = Harness<'static, FirepixelApp>;

const FLATTEN: &str = "Auswahl auf Hintergrund reduzieren";
const MERGE: &str = "Mit darunter liegender auf eine Ebene reduzieren";
const MASK: &str = "Maske hinzufügen";
const SHARE: &str = "Ebene für Seiten freigeben...";
const SHARE_ALL: &str = "Ebene für alle Seiten freigeben";

fn ex(s: &mut fp_engine::Session, c: &str, p: Value) -> Value {
    s.execute(c, p).unwrap_or_else(|e| panic!("{c}: {e}")).value
}

fn id_of(v: &Value) -> u64 {
    v["id"].as_u64().expect("id")
}

/// Measured document (RE spec 58 §3.1): path P1 on top, group G1, bitmap B1 at the bottom; nothing
/// selected. Returns the harness and the ids (P1, G1, B1).
fn harness() -> (H, u64, u64, u64) {
    fp_i18n::set_language("de");
    let mut s = fp_engine::Session::default();
    ex(&mut s, "file.new", json!({"width": 600, "height": 400, "background": "#ffffff"}));
    let b = id_of(&ex(&mut s, "draw.rectangle", json!({"rect": [20, 20, 120, 120]})));
    ex(&mut s, "select.set", json!({"ids": [b]}));
    let b = id_of(&ex(&mut s, "modify.flattenSelection", json!({})));
    ex(&mut s, "element.setName", json!({"ids": [b], "name": "B1"}));
    let g1 = id_of(&ex(&mut s, "draw.rectangle", json!({"rect": [150, 20, 200, 80]})));
    let g2 = id_of(&ex(&mut s, "draw.rectangle", json!({"rect": [210, 20, 260, 80]})));
    ex(&mut s, "select.set", json!({"ids": [g1, g2]}));
    ex(&mut s, "modify.group", json!({}));
    let g = s.active().expect("d").selection.elements[0].0;
    ex(&mut s, "element.setName", json!({"ids": [g], "name": "G1"}));
    let p = id_of(&ex(&mut s, "draw.rectangle", json!({"rect": [300, 20, 450, 120]})));
    ex(&mut s, "element.setName", json!({"ids": [p], "name": "P1"}));
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
        if find(&h, "P1").is_some() {
            break;
        }
        let tab = find(&h, "Ebenen").expect("Layers tab");
        click(&mut h, tab);
        h.run_steps(30);
    }
    assert!(find(&h, "P1").is_some(), "Layers panel shows the object rows");
    (h, p, g, b)
}

fn ev(h: &mut H, e: Event) {
    h.input_mut().events.push(e);
    h.step();
}

fn press(h: &mut H, p: Pos2, button: PointerButton) {
    ev(h, Event::PointerMoved(p));
    let e = |down| Event::PointerButton { pos: p, button, pressed: down, modifiers: Modifiers::NONE };
    ev(h, e(true));
    ev(h, e(false));
}

fn click(h: &mut H, p: Pos2) {
    h.run_steps(30);
    press(h, p, PointerButton::Primary);
    ev(h, Event::PointerMoved(Pos2::new(5.0, 940.0)));
    h.run_steps(5);
}

fn right_click(h: &mut H, label: &str) {
    close_menu(h);
    let p = find(h, label).unwrap_or_else(|| panic!("row {label}"));
    press(h, p, PointerButton::Secondary);
    h.run_steps(4);
}

fn close_menu(h: &mut H) {
    for pressed in [true, false] {
        ev(h, Event::Key { key: egui::Key::Escape, physical_key: None, pressed, repeat: false, modifiers: Modifiers::NONE });
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

/// Rect of an entry of the open context menu. Menu rows are wide; the footer button with the same
/// tooltip is a small icon.
fn item_rect(h: &H, label: &str) -> Option<egui::Rect> {
    h.query_all(By::new().role(Role::Button).label(label)).map(|n| n.rect()).find(|r| r.width() > 80.0)
}

/// Enabled state of an entry of the open context menu (`None`: not shown).
fn item(h: &H, label: &str) -> Option<bool> {
    h.query_all(By::new().role(Role::Button).label(label)).find(|n| n.rect().width() > 80.0).map(|n| !n.accesskit_node().is_disabled())
}

fn selection(h: &H) -> Vec<u64> {
    h.state().state.session.active().expect("d").selection.elements.iter().map(|e| e.0).collect()
}

fn active_layer(h: &H) -> Option<fp_model::LayerId> {
    h.state().state.session.active().and_then(|d| d.layer)
}

#[test]
fn right_click_object_row_selects_and_sets_menu_states() {
    let (mut h, p, g, b) = harness();
    assert!(selection(&h).is_empty());
    // (a) path above the group
    right_click(&mut h, "P1");
    assert_eq!(selection(&h), vec![p], "right-click selects the path");
    assert!(highlighted(&h, "P1"), "path row highlighted");
    assert_eq!(item(&h, FLATTEN), Some(true), "Flatten on for a path");
    assert_eq!(item(&h, MASK), Some(true), "Add Mask on for a path");
    assert_eq!(item(&h, MERGE), Some(false), "Merge Down grey: a group lies below");
    // (d) group directly above the bitmap
    right_click(&mut h, "G1");
    assert_eq!(selection(&h), vec![g], "right-click selects the group");
    assert!(highlighted(&h, "G1"));
    assert!(!highlighted(&h, "P1"));
    assert_eq!(item(&h, FLATTEN), Some(true));
    assert_eq!(item(&h, MASK), Some(true));
    assert_eq!(item(&h, MERGE), Some(true), "Merge Down on: the bitmap lies directly below");
    // the bitmap itself
    right_click(&mut h, "B1");
    assert_eq!(selection(&h), vec![b], "right-click selects the bitmap");
    assert_eq!(item(&h, FLATTEN), Some(true));
    assert_eq!(item(&h, MASK), Some(true));
    assert_eq!(item(&h, MERGE), Some(false), "Merge Down grey for the bitmap itself");
    close_menu(&mut h);
}

#[test]
fn right_click_layer_row_keeps_selection() {
    let (mut h, p, ..) = harness();
    // (c) Layer 1, nothing selected
    right_click(&mut h, "Ebene 1");
    assert!(selection(&h).is_empty(), "layer row: selection unchanged");
    assert_eq!(item(&h, FLATTEN), Some(false), "Flatten grey without selection");
    assert_eq!(item(&h, MASK), Some(false), "Add Mask grey without selection");
    assert_eq!(item(&h, MERGE), Some(false));
    assert_eq!(item(&h, SHARE), Some(true), "Share Layer to Pages on for a normal layer");
    // (c') Layer 1 with the path selected
    close_menu(&mut h);
    h.state_mut().state.session.execute("select.set", json!({"ids": [p]})).expect("select");
    h.run_steps(3);
    right_click(&mut h, "Ebene 1");
    assert_eq!(selection(&h), vec![p], "layer row: selection stays 1 → 1");
    assert_eq!(item(&h, FLATTEN), Some(true));
    assert_eq!(item(&h, MASK), Some(true));
    assert_eq!(item(&h, MERGE), Some(false));
    close_menu(&mut h);
}

#[test]
fn right_click_web_layer_greys_sharing() {
    let (mut h, ..) = harness();
    let before = active_layer(&h);
    right_click(&mut h, "Webebene");
    assert!(selection(&h).is_empty(), "web layer row: selection unchanged");
    assert_ne!(active_layer(&h), before, "the web layer becomes the current layer");
    assert_eq!(item(&h, FLATTEN), Some(false));
    assert_eq!(item(&h, MASK), Some(false));
    assert_eq!(item(&h, SHARE), Some(false), "Share Layer to Pages grey on the web layer");
    assert_eq!(item(&h, SHARE_ALL), Some(false), "Share Layer to All Pages grey on the web layer");
    close_menu(&mut h);
}

#[test]
fn add_mask_from_context_menu_selects_the_mask() {
    let (mut h, p, ..) = harness();
    right_click(&mut h, "P1");
    let r = item_rect(&h, MASK).expect("menu entry");
    click(&mut h, r.center());
    h.run_steps(5);
    let d = h.state().state.session.active().expect("d");
    let top = fp_engine::docops::own_layers(&d.doc, d.page)
        .into_iter()
        .rev()
        .find_map(|(_, l)| l.cels.first().and_then(|c| c.elements.first().cloned()))
        .expect("top element");
    let (child, mask) = top.masked_single().expect("masked path");
    assert_eq!(child.id.0, p);
    assert_eq!(selection(&h), vec![mask.id.0], "the new mask is selected");
    assert_eq!(d.history.undo_label(), Some("Maske hinzufügen"));
}
