//! Q-65 and the current-layer rules of RE spec 64: the current layer after opening and reverting
//! (bottom-most unlocked layer, page 0, state 0, nothing selected), a selection of exactly one
//! object makes its layer current without a History row, locking the current layer moves it up,
//! new layers become current, and the Layers panel's layer-row click writes one "Layer Change" row.

use crate::common::*;
use fp_engine::tools::ToolId;
use fp_engine::{Session, ToolEvent, docops};
use fp_model::{ElementId, ElementKind, LayerId, LayerKind};
use serde_json::json;
use std::path::PathBuf;

/// The measuring document `mkn` of RE spec 64 §2.1: L1 (bottom) with r1, L2 with r2, L3 with r3;
/// L3 is current afterwards.
struct Mkn {
    s: Session,
    l: [LayerId; 3],
    r: [ElementId; 3],
}

fn mkn() -> Mkn {
    fp_i18n::set_language("de");
    let mut s = doc_session(600.0, 400.0);
    let l1 = cur(&s);
    let r1 = rect(&mut s, 50.0, 50.0, 130.0, 110.0);
    let l2 = LayerId(ex(&mut s, "layer.add", json!({"name": "L2"}))["layer"].as_u64().unwrap_or(0));
    let r2 = rect(&mut s, 200.0, 100.0, 280.0, 160.0);
    let l3 = LayerId(ex(&mut s, "layer.add", json!({"name": "L3"}))["layer"].as_u64().unwrap_or(0));
    let r3 = rect(&mut s, 350.0, 150.0, 430.0, 210.0);
    ex(&mut s, "select.none", json!({}));
    assert_eq!(cur(&s), l3);
    Mkn { s, l: [l1, l2, l3], r: [r1, r2, r3] }
}

fn cur(s: &Session) -> LayerId {
    s.active().and_then(|d| d.layer).expect("current layer")
}

fn sel(s: &Session) -> Vec<ElementId> {
    s.active().map(|d| d.selection.elements.clone()).unwrap_or_default()
}

fn rows(s: &Session) -> usize {
    s.active().map(|d| d.history.len()).unwrap_or(0)
}

fn web(s: &Session) -> LayerId {
    let d = s.active().expect("doc");
    docops::own_layers(&d.doc, d.page)
        .into_iter()
        .find(|(_, l)| l.kind == LayerKind::Web)
        .map(|(_, l)| l.id)
        .expect("web layer")
}

fn lock(s: &mut Session, l: LayerId, v: bool) {
    ex(s, "layer.setLocked", json!({"layer": l.0, "locked": v}));
}

fn set_cur(s: &mut Session, l: LayerId) {
    ex(s, "layer.setActive", json!({"layer": l.0}));
}

fn click(s: &mut Session, x: f32, y: f32, m: impl Fn(ToolEvent) -> ToolEvent) {
    events(s, &[m(ToolEvent::down(x, y)), m(ToolEvent::up(x, y))]);
}

fn tmp(name: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(1);
    std::env::temp_dir().join(format!(
        "fp-q65-{}-{}-{name}.fw.png",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ))
}

/// Saves the active document, opens the file again and returns the flat index of the current
/// layer, the page, the state and the selection of the opened document.
fn reopen(s: &mut Session, name: &str) -> (usize, usize, usize, usize) {
    let p = tmp(name);
    ex(s, "file.saveAs", json!({"path": p.to_string_lossy()}));
    ex(s, "file.open", json!({"path": p.to_string_lossy()}));
    let _ = std::fs::remove_file(&p);
    state_of(s)
}

fn state_of(s: &Session) -> (usize, usize, usize, usize) {
    let d = s.active().expect("doc");
    let flat = docops::flat_layers(&d.doc, d.page, d.frame);
    let i = flat.iter().position(|l| Some(l.id) == d.layer).expect("current layer in the list");
    (i, d.page, d.frame, d.selection.elements.len())
}

// ------------------------------------------------------------------ §1 opening (O1–O13)

#[test]
fn o1_t07_opens_with_layer_1_current() {
    let src = std::path::Path::new(r"C:\Users\Administrator\Desktop\REA\re\_testdateien\par\par_layers.fw.png");
    if !src.is_file() {
        eprintln!("skipped: {src:?} missing");
        return;
    }
    fp_i18n::set_language("de");
    let p = tmp("t07");
    std::fs::copy(src, &p).expect("copy");
    let mut s = doc_session(10.0, 10.0);
    ex(&mut s, "file.open", json!({"path": p.to_string_lossy()}));
    let _ = std::fs::remove_file(&p);
    assert_eq!(state_of(&s), (0, 0, 0, 0), "Q-65: flat index 0, page 0, state 0, nothing selected");
    let d = s.active().expect("doc");
    let name = docops::layer_by_id(&d.doc, 0, cur(&s)).map(|l| l.name.clone());
    assert_eq!(name.as_deref(), Some("Ebene 1"));
    // T07 continued: a layer-row click on "Ebene 2", then the object row of L1obj → back to 0
    let flat = docops::flat_layers(&d.doc, 0, 0);
    let e2 = flat[1].id;
    let l1obj = d.doc.pages[0].root.layers.iter().find_map(|sl| match sl {
        fp_model::LayerSlot::Own(l) if l.id == flat[0].id => l.cel(0).and_then(|c| c.elements.first()).map(|e| e.id),
        _ => None,
    });
    ex(&mut s, "layer.setActive", json!({"layer": e2.0, "select": []}));
    assert_eq!(state_of(&s).0, 1);
    if let Some(o) = l1obj {
        select(&mut s, &[o]);
        assert_eq!(state_of(&s).0, 0, "the object row makes its layer current");
    }
}

#[test]
fn o2_o7_current_layer_page_and_state_are_not_stored() {
    let mut m = mkn();
    // O2: "L2" current when saving
    set_cur(&mut m.s, m.l[1]);
    assert_eq!(reopen(&mut m.s, "o2"), (0, 0, 0, 0));
    // O3: the web layer current when saving
    let mut m = mkn();
    let w = web(&m.s);
    set_cur(&mut m.s, w);
    assert_eq!(reopen(&mut m.s, "o3"), (0, 0, 0, 0));
    // O5: an object of L3 selected
    let mut m = mkn();
    select(&mut m.s, &[m.r[2]]);
    assert_eq!(reopen(&mut m.s, "o5"), (0, 0, 0, 0), "nothing selected after opening");
    // O6: page 1 current when saving
    let mut m = mkn();
    ex(&mut m.s, "page.add", json!({}));
    ex(&mut m.s, "page.setCurrent", json!({"index": 1}));
    assert_eq!(m.s.active().map(|d| d.page), Some(1));
    assert_eq!(reopen(&mut m.s, "o6"), (0, 0, 0, 0));
    // O7: state 2 current when saving
    let mut m = mkn();
    ex(&mut m.s, "state.add", json!({}));
    ex(&mut m.s, "state.add", json!({}));
    ex(&mut m.s, "state.setCurrent", json!({"index": 2}));
    assert_eq!(m.s.active().map(|d| d.frame), Some(2));
    assert_eq!(reopen(&mut m.s, "o7"), (0, 0, 0, 0));
}

#[test]
fn o8_o12_lock_decides_not_visibility() {
    // O8: L1 locked → L2
    let mut m = mkn();
    lock(&mut m.s, m.l[0], true);
    assert_eq!(cur(&m.s), m.l[2], "locking another layer keeps the current layer");
    assert_eq!(reopen(&mut m.s, "o8").0, 1);
    // O9: L1 hidden → L1
    let mut m = mkn();
    ex(&mut m.s, "layer.setVisible", json!({"layer": m.l[0].0, "visible": false}));
    assert_eq!(reopen(&mut m.s, "o9").0, 0);
    // O10: L1, L2 locked → L3
    let mut m = mkn();
    lock(&mut m.s, m.l[0], true);
    lock(&mut m.s, m.l[1], true);
    assert_eq!(reopen(&mut m.s, "o10").0, 2);
    // O11: all normal layers locked → the web layer
    let mut m = mkn();
    for l in m.l {
        lock(&mut m.s, l, true);
    }
    assert_eq!(reopen(&mut m.s, "o11").0, 3);
    // O12: the web layer locked as well → still the web layer
    let mut m = mkn();
    let w = web(&m.s);
    for l in m.l {
        lock(&mut m.s, l, true);
    }
    lock(&mut m.s, w, true);
    assert_eq!(reopen(&mut m.s, "o12").0, 3);
}

#[test]
fn o13_sublayer_inherits_the_lock_of_its_parent() {
    let mut m = mkn();
    set_cur(&mut m.s, m.l[0]);
    ex(&mut m.s, "pi.layer.addSub", json!({"name": "S1"}));
    let s1 = cur(&m.s);
    {
        let d = m.s.active().expect("doc");
        let flat = docops::flat_layers(&d.doc, d.page, d.frame);
        assert_eq!(flat[1].id, s1, "a sublayer follows its parent in flat order");
    }
    lock(&mut m.s, m.l[0], true);
    assert_eq!(reopen(&mut m.s, "o13").0, 2, "L2 (flat index 2)");
}

#[test]
fn revert_uses_the_open_rule() {
    let mut m = mkn();
    lock(&mut m.s, m.l[0], true);
    let p = tmp("revert");
    ex(&mut m.s, "file.saveAs", json!({"path": p.to_string_lossy()}));
    set_cur(&mut m.s, m.l[2]);
    select(&mut m.s, &[m.r[2]]);
    rect(&mut m.s, 10.0, 10.0, 20.0, 20.0);
    ex(&mut m.s, "file.revert", json!({}));
    let _ = std::fs::remove_file(&p);
    assert_eq!(state_of(&m.s), (1, 0, 0, 0), "bottom-most unlocked layer (L2) after revert");
}

#[test]
fn new_document_and_new_layers_become_current() {
    // RE spec 64 §1.3
    let mut s = doc_session(600.0, 400.0);
    assert_eq!(state_of(&s).0, 0, "new document: Ebene 1");
    ex(&mut s, "layer.add", json!({}));
    assert_eq!(state_of(&s).0, 1);
    ex(&mut s, "layer.add", json!({}));
    assert_eq!(state_of(&s).0, 2);
    // addNewSubLayer(0): a sublayer of the bottom layer at flat index 1, current
    let bottom = {
        let d = s.active().expect("doc");
        docops::flat_layers(&d.doc, 0, 0)[0].id
    };
    set_cur(&mut s, bottom);
    ex(&mut s, "pi.layer.addSub", json!({}));
    assert_eq!(state_of(&s).0, 1);
    let sub = cur(&s);
    // the panel's New Layer button with the current sublayer: a sibling above it in the same
    // parent at flat index 2, current
    let new = LayerId(ex(&mut s, "layer.add", json!({"sibling": true}))["layer"].as_u64().unwrap_or(0));
    assert_eq!(cur(&s), new);
    assert_eq!(state_of(&s).0, 2);
    let d = s.active().expect("doc");
    let parent = d.doc.pages[0].root.layers.iter().find_map(|sl| match sl {
        fp_model::LayerSlot::Own(l) if l.id == bottom => Some(l.clone()),
        _ => None,
    });
    let subs: Vec<LayerId> = parent
        .and_then(|l| l.cel(0).cloned())
        .map(|c| {
            c.elements
                .iter()
                .filter_map(|e| match &e.kind {
                    ElementKind::SubLayer(sl) => Some(sl.id),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default();
    assert_eq!(subs, [new, sub], "the new sublayer lies above the current one in the same parent");
    // the button on a top-level layer: a new top-level layer directly above it
    let mut m = mkn();
    set_cur(&mut m.s, m.l[0]);
    let n = LayerId(ex(&mut m.s, "layer.add", json!({"sibling": true}))["layer"].as_u64().unwrap_or(0));
    let d = m.s.active().expect("doc");
    let ids: Vec<LayerId> = docops::own_layers(&d.doc, 0).iter().map(|(_, l)| l.id).collect();
    assert_eq!(&ids[..4], &[m.l[0], n, m.l[1], m.l[2]]);
    assert_eq!(cur(&m.s), n);
}

// ------------------------------------------------------------------ §2 selection (S cases)

#[test]
fn s1_s23_panel_row_and_script_selection_of_one_object() {
    let mut m = mkn();
    let h = rows(&m.s);
    // S1 / S23: one object selected (object row, `fw.selection = [r1]`) → its layer
    select(&mut m.s, &[m.r[0]]);
    assert_eq!(cur(&m.s), m.l[0]);
    // deselecting keeps the current layer (S7)
    ex(&mut m.s, "select.set", json!({"ids": []}));
    assert_eq!(cur(&m.s), m.l[0]);
    select(&mut m.s, &[m.r[1]]);
    assert_eq!(cur(&m.s), m.l[1]);
    assert_eq!(rows(&m.s), h, "selection changes and the layer change write no History row");
}

#[test]
fn s8_s21_two_or_more_objects_keep_the_current_layer() {
    let mut m = mkn();
    // S8: row r1, Ctrl+row r3 → [r3, r1], L1 stays
    select(&mut m.s, &[m.r[0]]);
    ex(&mut m.s, "select.set", json!({"ids": [m.r[2].0], "mode": "toggle"}));
    assert_eq!(sel(&m.s).len(), 2);
    assert_eq!(cur(&m.s), m.l[0]);
    // S9b: row r3, Ctrl+row r1 → L3 stays
    select(&mut m.s, &[m.r[2]]);
    ex(&mut m.s, "select.set", json!({"ids": [m.r[0].0], "mode": "toggle"}));
    assert_eq!(cur(&m.s), m.l[2]);
    // S16: r1, Shift r3, Shift r1 off → [r3] → L3
    select(&mut m.s, &[m.r[0]]);
    ex(&mut m.s, "select.set", json!({"ids": [m.r[2].0], "mode": "toggle"}));
    assert_eq!(cur(&m.s), m.l[0]);
    ex(&mut m.s, "select.set", json!({"ids": [m.r[0].0], "mode": "toggle"}));
    assert_eq!(sel(&m.s), [m.r[2]]);
    assert_eq!(cur(&m.s), m.l[2]);
    // S18: r1, Shift r2, Shift r3, Shift r1 off → [r3, r2]: L1 stays without a selected object
    select(&mut m.s, &[m.r[0]]);
    for r in [m.r[1], m.r[2], m.r[0]] {
        ex(&mut m.s, "select.set", json!({"ids": [r.0], "mode": "toggle"}));
    }
    assert_eq!(sel(&m.s).len(), 2);
    assert_eq!(cur(&m.s), m.l[0]);
    // S5: Select All with three objects keeps the layer
    select(&mut m.s, &[m.r[0]]);
    ex(&mut m.s, "select.all", json!({}));
    assert_eq!(sel(&m.s).len(), 3);
    assert_eq!(cur(&m.s), m.l[0]);
    // S14/S15: from nothing, Shift/Ctrl adds one → its layer
    set_cur(&mut m.s, m.l[2]);
    ex(&mut m.s, "select.none", json!({}));
    ex(&mut m.s, "select.set", json!({"ids": [m.r[0].0], "mode": "add"}));
    assert_eq!(cur(&m.s), m.l[0]);
}

#[test]
fn s3_s4_s12_s14_canvas_clicks_with_the_pointer() {
    let mut m = mkn();
    m.s.set_tool(ToolId::Pointer).unwrap_or_else(|e| panic!("{e}"));
    let h = rows(&m.s);
    // S3: click r2 → L2
    click(&mut m.s, 240.0, 130.0, |e| e);
    assert_eq!(sel(&m.s), [m.r[1]]);
    assert_eq!(cur(&m.s), m.l[1]);
    // S4: click into empty canvas → [] and L2 stays
    click(&mut m.s, 520.0, 330.0, |e| e);
    assert!(sel(&m.s).is_empty());
    assert_eq!(cur(&m.s), m.l[1]);
    // S12/S13: r3, Shift r1 → L3 stays; Shift r1 off → [r3], L3
    click(&mut m.s, 390.0, 180.0, |e| e);
    assert_eq!(cur(&m.s), m.l[2]);
    click(&mut m.s, 90.0, 80.0, ToolEvent::shift);
    assert_eq!(sel(&m.s).len(), 2);
    assert_eq!(cur(&m.s), m.l[2]);
    // S14: nothing selected, Shift+click r1 → L1
    click(&mut m.s, 520.0, 330.0, |e| e);
    click(&mut m.s, 90.0, 80.0, ToolEvent::shift);
    assert_eq!(sel(&m.s), [m.r[0]]);
    assert_eq!(cur(&m.s), m.l[0]);
    assert_eq!(rows(&m.s), h, "canvas selection writes no History row");
}

#[test]
fn s17_s22_marquee_over_one_or_two_objects() {
    let mut m = mkn();
    let h = rows(&m.s);
    // S17: marquee over r1 and r2 → L3 stays
    ex(&mut m.s, "select.inRect", json!({"rect": [40, 40, 290, 170]}));
    assert_eq!(sel(&m.s).len(), 2);
    assert_eq!(cur(&m.s), m.l[2]);
    // S22: marquee over r1 only → L1
    ex(&mut m.s, "select.none", json!({}));
    let h2 = rows(&m.s);
    ex(&mut m.s, "select.inRect", json!({"rect": [40, 40, 140, 120]}));
    assert_eq!(sel(&m.s), [m.r[0]]);
    assert_eq!(cur(&m.s), m.l[0]);
    assert!(rows(&m.s) == h2 && h2 >= h, "the marquee writes no History row");
}

#[test]
fn s24_select_all_with_one_selectable_object_and_the_lock_rule() {
    let mut m = mkn();
    let w = web(&m.s);
    // locking L2, then the current L3 → the web layer becomes current (rule 7)
    lock(&mut m.s, m.l[1], true);
    assert_eq!(cur(&m.s), m.l[2]);
    lock(&mut m.s, m.l[2], true);
    assert_eq!(cur(&m.s), w);
    // Select All finds only r1 → L1
    ex(&mut m.s, "select.all", json!({}));
    assert_eq!(sel(&m.s), [m.r[0]]);
    assert_eq!(cur(&m.s), m.l[0]);
}

#[test]
fn lock_rule_moves_up_or_to_index_0() {
    // a middle current layer moves to the next unlocked one above
    let mut m = mkn();
    set_cur(&mut m.s, m.l[1]);
    lock(&mut m.s, m.l[1], true);
    assert_eq!(cur(&m.s), m.l[2]);
    // all layers, the web layer included, locked → flat index 0 (O12)
    let mut m = mkn();
    let w = web(&m.s);
    for l in m.l {
        lock(&mut m.s, l, true);
    }
    assert_eq!(cur(&m.s), w);
    lock(&mut m.s, w, true);
    assert_eq!(cur(&m.s), m.l[0]);
    // unlocking moves nothing
    lock(&mut m.s, m.l[2], false);
    assert_eq!(cur(&m.s), m.l[0]);
}

#[test]
fn s31_sublayer_hidden_layer_and_group_objects() {
    let mut m = mkn();
    // an object in a sublayer of L1 makes the sublayer current
    set_cur(&mut m.s, m.l[0]);
    ex(&mut m.s, "pi.layer.addSub", json!({"name": "Unterebene"}));
    let sub = cur(&m.s);
    let o = rect(&mut m.s, 60.0, 300.0, 90.0, 330.0);
    ex(&mut m.s, "select.none", json!({}));
    set_cur(&mut m.s, m.l[2]);
    select(&mut m.s, &[o]);
    assert_eq!(cur(&m.s), sub);
    // an object in a hidden layer makes that layer current
    ex(&mut m.s, "layer.setVisible", json!({"layer": m.l[1].0, "visible": false}));
    select(&mut m.s, &[m.r[1]]);
    assert_eq!(cur(&m.s), m.l[1]);
    // a child of a group: the group's layer
    ex(&mut m.s, "layer.setVisible", json!({"layer": m.l[1].0, "visible": true}));
    set_cur(&mut m.s, m.l[2]);
    let a = rect(&mut m.s, 450.0, 300.0, 470.0, 320.0);
    let b = rect(&mut m.s, 480.0, 300.0, 500.0, 320.0);
    select(&mut m.s, &[a, b]);
    ex(&mut m.s, "modify.group", json!({}));
    set_cur(&mut m.s, m.l[0]);
    select(&mut m.s, &[a]);
    assert_eq!(cur(&m.s), m.l[2]);
}

#[test]
fn new_objects_land_at_the_top_of_the_current_layer() {
    // S25: after the object row r1 the new rectangle lies at the top of L1
    let mut m = mkn();
    select(&mut m.s, &[m.r[0]]);
    let n = rect(&mut m.s, 420.0, 300.0, 500.0, 370.0);
    let d = m.s.active().expect("doc");
    let l1 = docops::layer_by_id(&d.doc, 0, m.l[0]).expect("L1");
    let ids: Vec<ElementId> = l1.cel(0).map(|c| c.elements.iter().map(|e| e.id).collect()).unwrap_or_default();
    assert_eq!(ids, [n, m.r[0]]);
    // S26: two objects of other layers selected, current L1 → the new object lands in L1
    let mut m = mkn();
    select(&mut m.s, &[m.r[0]]);
    ex(&mut m.s, "select.set", json!({"ids": [m.r[1].0, m.r[2].0], "mode": "add"}));
    ex(&mut m.s, "select.set", json!({"ids": [m.r[0].0], "mode": "subtract"}));
    assert_eq!(cur(&m.s), m.l[0]);
    let n = rect(&mut m.s, 420.0, 300.0, 500.0, 370.0);
    let d = m.s.active().expect("doc");
    let l1 = docops::layer_by_id(&d.doc, 0, m.l[0]).expect("L1");
    assert_eq!(l1.cel(0).and_then(|c| c.elements.first()).map(|e| e.id), Some(n));
}

#[test]
fn layer_row_click_writes_one_layer_change_row() {
    // RE spec 64 §2.4: the layer row L2 (selects r2, L2 current) → "Ebenenänderung"; undo → L3,
    // nothing selected; redo → L2 with r2
    let mut m = mkn();
    let h = rows(&m.s);
    ex(&mut m.s, "layer.setActive", json!({"layer": m.l[1].0, "select": [m.r[1].0]}));
    assert_eq!(cur(&m.s), m.l[1]);
    assert_eq!(sel(&m.s), [m.r[1]]);
    assert_eq!(rows(&m.s), h + 1);
    let labels = m.s.active().map(|d| d.history.labels()).unwrap_or_default();
    assert_eq!(labels.last().map(String::as_str), Some("Ebenenänderung"));
    // object row and canvas selection afterwards add no row
    select(&mut m.s, &[m.r[0]]);
    assert_eq!(cur(&m.s), m.l[0]);
    assert_eq!(rows(&m.s), h + 1);
    m.s.undo().expect("undo");
    assert_eq!(cur(&m.s), m.l[2]);
    assert!(sel(&m.s).is_empty());
    m.s.redo().expect("redo");
    assert_eq!(cur(&m.s), m.l[1]);
    assert_eq!(sel(&m.s), [m.r[1]]);
    // a hidden layer without objects: the row still writes a row (it changes the current layer)
    let mut m = mkn();
    ex(&mut m.s, "layer.setVisible", json!({"layer": m.l[1].0, "visible": false}));
    select(&mut m.s, &[m.r[1]]);
    ex(&mut m.s, "edit.clear", json!({}));
    set_cur(&mut m.s, m.l[2]);
    let h = rows(&m.s);
    ex(&mut m.s, "layer.setActive", json!({"layer": m.l[1].0, "select": []}));
    assert_eq!(rows(&m.s), h + 1);
    // a click on the current layer that changes nothing writes no row
    ex(&mut m.s, "layer.setActive", json!({"layer": m.l[1].0, "select": []}));
    assert_eq!(rows(&m.s), h + 1);
    // without `select` (right-click, scripts) only the current layer changes, no row
    ex(&mut m.s, "layer.setActive", json!({"layer": m.l[0].0}));
    assert_eq!(cur(&m.s), m.l[0]);
    assert_eq!(rows(&m.s), h + 1);
    let mut e = mkn();
    fp_i18n::set_language("en");
    ex(&mut e.s, "layer.setActive", json!({"layer": e.l[0].0, "select": [e.r[0].0]}));
    let labels = e.s.active().map(|d| d.history.labels()).unwrap_or_default();
    fp_i18n::set_language("de");
    assert_eq!(labels.last().map(String::as_str), Some("Layer Change"));
}
