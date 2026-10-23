//! FB-30 / TW-208 (RE spec 61 §3): multi-selection in the Layers panel. A click selects that row only
//! and sets the anchor; Shift+click selects the range anchor…row and replaces the selection; Ctrl+click
//! (Cmd on macOS) toggles the row and keeps the anchor; Ctrl+Shift+click behaves like Shift; a click on
//! a row of a multi-selection keeps it; a click on a layer row selects all objects of the layer.
//! TW-208 rest (RE spec 63 §1): Shift ranges run over the document's object order across layers
//! (collapsed, hidden and sublayers included, locked layers left out), rows of locked layers ignore
//! clicks, Shift/Ctrl on layer rows, the anchor counts while its object is selected.

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
    let h = harness_session(s, &["O1", "O2", "O3", "O4", "Ebene 1"]);
    (h, ids)
}

/// Harness on the session `s` with the Pointer tool and the Layers panel showing the rows `rows`.
fn harness_session(s: fp_engine::Session, rows: &[&str]) -> H {
    harness_sized(s, rows, 950.0)
}

/// Like `harness_session` with the window height `height` (taller: a longer layer list).
fn harness_sized(mut s: fp_engine::Session, rows: &[&str], height: f32) -> H {
    s.set_tool(ToolId::Pointer).expect("pointer");
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.os_clipboard = false;
    app.state.sync_render = true;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1500.0, height))
        .with_pixels_per_point(1.0)
        .with_max_steps(8)
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    for _ in 0..2 {
        if rows.first().is_some_and(|r| find(&h, r).is_some()) {
            break;
        }
        let tab = find(&h, "Ebenen").expect("Layers tab");
        click_at(&mut h, tab, Modifiers::NONE);
        h.run_steps(30);
    }
    for n in rows {
        if find(&h, n).is_none() {
            let mut all = Vec::new();
            for cs in &h.output().shapes {
                texts(&cs.shape, &mut all);
            }
            let v: Vec<_> = all.iter().filter(|(_, r)| r.center().x > 900.0).map(|(t, r)| (t.clone(), r.center())).collect();
            panic!("Layers panel shows row {n}: {v:?}");
        }
    }
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
    // the layer row is the anchor now: a Shift range spans the layer's own objects and the row
    // (RE spec 63 §1.3 rule 3, T7/T12), so Shift+click O2 keeps all four
    click(&mut h, "O2", SHIFT);
    assert_eq!(selection(&h), objs(&o, &[4, 3, 2, 1]));
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
    // Cmd-clicks do not move the anchor, but O4 is no longer selected: the bottom-most selected
    // object O2 is the anchor (RE spec 63 §1.3 rule 5, T26)
    click(&mut h, "O3", SHIFT);
    assert_eq!(selection(&h), objs(&o, &[3, 2]));
}

// ---- TW-208 rest (RE spec 63 §1): Shift ranges over the document's object order ----

/// The six-object document of RE spec 63 §1.1: "L1" (bottom) with e, f, "L2" with c, d, "L3" (top)
/// with a, b (rows named "obj a" …), in each layer the first object above the second. Variants of
/// L2: "normal", "hidden", "locked", "collapsed", "sub" (sublayer "U2" with g, h at the top of L2),
/// "sublocked" (sub, L2 locked). Nothing selected, thumbnails off so every row fits.
fn doc3(variant: &str) -> (H, Vec<(String, u64)>) {
    fp_i18n::set_language("de");
    let mut s = fp_engine::Session::default();
    s.prefs.set("LayersList_ThumbnailSize", fp_engine::prefs::PrefValue::Int(0));
    ex(&mut s, "file.new", json!({"width": 700, "height": 400, "background": "#ffffff"}));
    let mut ids: Vec<(String, u64)> = Vec::new();
    let mut rect = |s: &mut fp_engine::Session, n: &str, x: f64| {
        let v = ex(s, "draw.rectangle", json!({"rect": [x, 20.0, x + 60.0, 80.0]}));
        let id = v["id"].as_u64().expect("id");
        ex(s, "element.setName", json!({"ids": [id], "name": format!("obj {n}")}));
        ids.push((n.to_string(), id));
    };
    let l1 = s.active().and_then(|d| d.layer).expect("layer").0;
    ex(&mut s, "layer.rename", json!({"layer": l1, "name": "L1"}));
    rect(&mut s, "f", 20.0);
    rect(&mut s, "e", 100.0);
    let l2 = ex(&mut s, "layer.add", json!({"name": "L2"}))["layer"].as_u64().expect("L2");
    rect(&mut s, "d", 180.0);
    rect(&mut s, "c", 260.0);
    ex(&mut s, "layer.add", json!({"name": "L3"}));
    rect(&mut s, "b", 340.0);
    rect(&mut s, "a", 420.0);
    if variant.starts_with("sub") {
        ex(&mut s, "layer.setActive", json!({"layer": l2}));
        ex(&mut s, "pi.layer.addSub", json!({"name": "U2"}));
        rect(&mut s, "h", 500.0);
        rect(&mut s, "g", 580.0);
    }
    match variant {
        "hidden" => {
            ex(&mut s, "layer.setVisible", json!({"layer": l2, "visible": false}));
        }
        "locked" | "sublocked" => {
            ex(&mut s, "layer.setLocked", json!({"layer": l2, "locked": true}));
        }
        "collapsed" => {
            ex(&mut s, "layer.setExpanded", json!({"layer": l2, "expanded": false}));
        }
        _ => {}
    }
    ex(&mut s, "select.none", json!({}));
    let h = harness_sized(s, &["obj a", "obj f", "L1", "L2", "L3"], 1500.0);
    (h, ids)
}

/// Selection as object letters in the order of the spec's lists (a … h by panel position).
fn sel3(h: &H, ids: &[(String, u64)]) -> String {
    let sel = selection(h);
    let mut v: Vec<&str> = ids.iter().filter(|(_, id)| sel.contains(id)).map(|(n, _)| n.as_str()).collect();
    let rank = |n: &str| "abghcdef".find(n).unwrap_or(99);
    v.sort_by_key(|n| rank(n));
    v.join(" ")
}

/// Rows of the History panel of the active document.
fn history_rows(h: &H) -> usize {
    h.state().state.session.active().expect("d").history.len()
}

/// A selection change on the canvas or by a command (not through the panel).
fn canvas_select(h: &mut H, ids: &[(String, u64)], names: &[&str]) {
    let v: Vec<u64> = names.iter().map(|n| ids.iter().find(|(k, _)| k == n).expect(n).1).collect();
    h.state_mut().state.session.execute("select.set", json!({"ids": v})).expect("select.set");
    h.run_steps(5);
}

/// Nothing selected, as after a canvas click into empty space (no History row).
fn reset(h: &mut H) {
    h.state_mut().state.session.execute("select.set", json!({"ids": []})).expect("select.set");
    h.run_steps(5);
}

const ALL: &str = "a b c d e f";

/// V0, T6, T5, T7, T8, T9, T20, T19, T25, T28 (RE spec 63 §1.2) with real clicks; no History rows.
#[test]
fn layers_shift_range_over_layers_and_layer_rows() {
    let (mut h, o) = doc3("normal");
    let rows = history_rows(&h);
    // V0: range across all three layers
    click(&mut h, "obj a", Modifiers::NONE);
    click(&mut h, "obj f", SHIFT);
    assert_eq!(sel3(&h, &o), ALL, "V0");
    // T6: Shift on a layer row ends at the span of its own objects
    reset(&mut h);
    click(&mut h, "obj a", Modifiers::NONE);
    click(&mut h, "L2", SHIFT);
    assert_eq!(sel3(&h, &o), "a b c d", "T6");
    // T7: a layer row as the anchor
    click(&mut h, "L2", Modifiers::NONE);
    assert_eq!(sel3(&h, &o), "c d");
    click(&mut h, "obj f", SHIFT);
    assert_eq!(sel3(&h, &o), "c d e f", "T7");
    // T8: Ctrl on an object row and on a layer row (each own object toggled one by one)
    reset(&mut h);
    click(&mut h, "obj a", Modifiers::NONE);
    click(&mut h, "obj f", SHIFT);
    click(&mut h, "obj c", CTRL);
    assert_eq!(sel3(&h, &o), "a b d e f");
    click(&mut h, "L2", CTRL);
    assert_eq!(sel3(&h, &o), "a b c e f", "T8: c on, d off");
    // T5, T9: Shift on the bottom layer row, then Ctrl e
    reset(&mut h);
    click(&mut h, "obj a", Modifiers::NONE);
    click(&mut h, "L1", SHIFT);
    assert_eq!(sel3(&h, &o), ALL, "T5");
    click(&mut h, "obj e", CTRL);
    assert_eq!(sel3(&h, &o), "a b c d f", "T9");
    // T20: the anchor stays valid while its object is selected, even after a canvas change
    reset(&mut h);
    click(&mut h, "obj a", Modifiers::NONE);
    canvas_select(&mut h, &o, &["a", "e"]);
    click(&mut h, "obj c", SHIFT);
    assert_eq!(sel3(&h, &o), "a b c", "T20");
    // T19: anchor no longer selected: the bottom-most selected object is the anchor
    reset(&mut h);
    click(&mut h, "obj a", Modifiers::NONE);
    canvas_select(&mut h, &o, &["e"]);
    click(&mut h, "obj c", SHIFT);
    assert_eq!(sel3(&h, &o), "c d e", "T19");
    // T25: nothing selected: from the row to the bottom-most object of the document
    reset(&mut h);
    click(&mut h, "obj a", Modifiers::NONE);
    reset(&mut h);
    click(&mut h, "obj c", SHIFT);
    assert_eq!(sel3(&h, &o), "c d e f", "T25");
    reset(&mut h);
    click(&mut h, "obj a", SHIFT);
    assert_eq!(sel3(&h, &o), ALL, "T28");
    assert_eq!(history_rows(&h), rows, "selection changes in the panel write no History rows");
}

/// V3, T17: objects of a collapsed layer (no rows shown) count in the range.
#[test]
fn layers_shift_range_includes_collapsed_layer() {
    let (mut h, o) = doc3("collapsed");
    assert!(find(&h, "obj c").is_none(), "L2 collapsed: c has no row");
    click(&mut h, "obj a", Modifiers::NONE);
    click(&mut h, "obj f", SHIFT);
    assert_eq!(sel3(&h, &o), ALL, "V3");
    reset(&mut h);
    click(&mut h, "obj a", Modifiers::NONE);
    click(&mut h, "L2", SHIFT);
    assert_eq!(sel3(&h, &o), "a b c d", "T17");
}

/// V1: objects of a hidden layer count in the range.
#[test]
fn layers_shift_range_includes_hidden_layer() {
    let (mut h, o) = doc3("hidden");
    click(&mut h, "obj a", Modifiers::NONE);
    click(&mut h, "obj f", SHIFT);
    assert_eq!(sel3(&h, &o), ALL, "V1");
}

/// V2, T15, T1, T13, T14, T2: a locked layer is left out of ranges and its rows ignore clicks.
#[test]
fn layers_locked_layer_left_out_and_rows_ignore_clicks() {
    let (mut h, o) = doc3("locked");
    let rows = history_rows(&h);
    click(&mut h, "obj a", Modifiers::NONE);
    click(&mut h, "obj f", SHIFT);
    assert_eq!(sel3(&h, &o), "a b e f", "V2");
    reset(&mut h);
    click(&mut h, "obj e", Modifiers::NONE);
    click(&mut h, "obj a", SHIFT);
    assert_eq!(sel3(&h, &o), "a b e", "T15");
    reset(&mut h);
    click(&mut h, "obj a", Modifiers::NONE);
    click(&mut h, "obj c", SHIFT);
    assert_eq!(sel3(&h, &o), "a", "T1: Shift on a locked object row does nothing");
    click(&mut h, "obj c", CTRL);
    assert_eq!(sel3(&h, &o), "a", "T13: Ctrl on a locked object row does nothing");
    click(&mut h, "L2", SHIFT);
    assert_eq!(sel3(&h, &o), "a", "T14: Shift on a locked layer row does nothing");
    let layer = h.state().state.session.active().and_then(|d| d.layer);
    click(&mut h, "L2", Modifiers::NONE);
    assert_eq!(sel3(&h, &o), "a", "a click on a locked layer row does nothing");
    assert_eq!(h.state().state.session.active().and_then(|d| d.layer), layer, "the current layer stays");
    // T2: a click on a locked row keeps the empty selection; Shift f then ranges from the
    // bottom-most object of the document
    reset(&mut h);
    click(&mut h, "obj c", Modifiers::NONE);
    assert_eq!(sel3(&h, &o), "", "T2");
    click(&mut h, "obj f", SHIFT);
    assert_eq!(sel3(&h, &o), "f", "T2");
    assert_eq!(history_rows(&h), rows);
}

/// V5, T21, T21b, T16: sublayers in place; a locked parent locks its sublayer.
#[test]
fn layers_shift_range_with_sublayers() {
    let (mut h, o) = doc3("sub");
    click(&mut h, "obj a", Modifiers::NONE);
    click(&mut h, "obj f", SHIFT);
    assert_eq!(sel3(&h, &o), "a b g h c d e f", "V5");
    reset(&mut h);
    click(&mut h, "obj a", Modifiers::NONE);
    click(&mut h, "L2", SHIFT);
    assert_eq!(sel3(&h, &o), "a b g h c d", "T21");
    reset(&mut h);
    click(&mut h, "obj f", Modifiers::NONE);
    click(&mut h, "L2", SHIFT);
    assert_eq!(sel3(&h, &o), "c d e f", "T21b: the objects of the sublayer are not part of the span of L2");
    let (mut h, o) = doc3("sublocked");
    click(&mut h, "obj a", Modifiers::NONE);
    click(&mut h, "obj f", SHIFT);
    assert_eq!(sel3(&h, &o), "a b e f", "T16");
    click(&mut h, "obj g", SHIFT);
    assert_eq!(sel3(&h, &o), "a b e f", "a row of a sublayer of a locked layer ignores clicks");
}

/// Parity step `panel-layers/08_shift_ctrl_result` (test file T07): click the object of "Ebene 1",
/// Shift+click the object of "Gemeinsam" (locked through "Ebene 3"): nothing changes (1); Ctrl+click
/// the object of "Ebene 1": 0 (RE spec 63 §1.2).
#[test]
fn layers_t07_shift_on_locked_target_then_ctrl() {
    let src = std::path::PathBuf::from(r"C:\Users\Administrator\Desktop\REA\re\_testdateien\par\par_layers.fw.png");
    if !src.is_file() {
        return;
    }
    fp_i18n::set_language("de");
    let dir = std::env::temp_dir().join(format!("fp-tw208-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let p = dir.join("t07.fw.png");
    std::fs::copy(&src, &p).expect("copy");
    let mut s = fp_engine::Session::default();
    s.prefs.set("LayersList_ThumbnailSize", fp_engine::prefs::PrefValue::Int(0));
    ex(&mut s, "file.open", json!({"path": p.to_string_lossy()}));
    let mut h = harness_sized(s, &["Ebene 1", "Gemeinsam"], 1500.0);
    let rows = history_rows(&h);
    let below = |h: &H, label: &str| {
        let at = find(h, label).unwrap_or_else(|| panic!("row {label}"));
        let mut all = Vec::new();
        for cs in &h.output().shapes {
            texts(&cs.shape, &mut all);
        }
        all.iter()
            .filter(|(_, r)| r.center().x > 900.0 && r.center().y > at.y + 8.0)
            .min_by(|a, b| a.1.center().y.total_cmp(&b.1.center().y))
            .map(|(t, r)| (t.clone(), r.center()))
            .unwrap_or_else(|| panic!("row below {label}"))
    };
    let (name, l1obj) = below(&h, "Ebene 1");
    assert_eq!(name, "L1obj", "object row of Ebene 1");
    click_at(&mut h, l1obj, Modifiers::NONE);
    assert_eq!(selection(&h).len(), 1, "click L1obj");
    let (name, target) = below(&h, "Gemeinsam");
    assert_eq!(name, "Pfad", "object row of Gemeinsam");
    click_at(&mut h, target, SHIFT);
    assert_eq!(selection(&h).len(), 1, "Shift+click on a row locked through Ebene 3 does nothing");
    click_at(&mut h, l1obj, CTRL);
    assert_eq!(selection(&h).len(), 0, "Ctrl+click L1obj deselects it");
    assert_eq!(history_rows(&h), rows);
}
