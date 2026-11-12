//! F159 (FB-143, FB-144, FB-147; RE spec 82): the engine side of dragging object rows in the
//! Layers panel (`pi.element.moveTo`: block of objects, slot in the old list, no-op without a
//! History row, into and out of plain groups), Modify > Arrange for objects of a group and of a
//! mask group, and the Pointer / Subselection cursors over anchors.

use fp_engine::Session;
use fp_engine::docops::{self, DragItem, ListOwner};
use fp_engine::tools::{Cursor, ToolEvent, ToolId};
use fp_model::{ElementId, ElementKind, LayerId};
use serde_json::{Value, json};

fn ex(s: &mut Session, c: &str, p: Value) -> Value {
    s.execute(c, p).unwrap_or_else(|e| panic!("{c}: {e}")).value
}

fn rect(s: &mut Session, name: &str, x: f64) -> u64 {
    let id = ex(s, "draw.rectangle", json!({"rect": [x, 20.0, x + 40.0, 60.0]}))["id"]
        .as_u64()
        .expect("id");
    ex(s, "element.setName", json!({"ids": [id], "name": name}));
    id
}

fn group(s: &mut Session, ids: &[u64], name: &str) -> u64 {
    ex(s, "select.set", json!({ "ids": ids }));
    let g = ex(s, "modify.group", json!({}))["id"].as_u64().expect("group id");
    ex(s, "element.setName", json!({"ids": [g], "name": name}));
    g
}

fn name_of(e: &fp_model::Element) -> String {
    e.common.name.clone().unwrap_or_default()
}

/// Names of the objects of layer `l`, top first.
fn layer_names(s: &Session, l: u64) -> Vec<String> {
    let d = s.active().expect("doc");
    let layer = docops::layer_by_id(&d.doc, d.page, LayerId(l)).expect("layer");
    layer
        .cel(d.frame)
        .map(|c| c.elements.iter().map(|e| name_of(e)).collect())
        .unwrap_or_default()
}

/// Names of the children of group `g` (of the mask group when `g` is masked), top first.
fn child_names(s: &Session, g: u64) -> Vec<String> {
    let d = s.active().expect("doc");
    let e = docops::element_on_page(&d.doc, d.page, ElementId(g)).expect("group");
    match &e.kind {
        ElementKind::Group(x) => x.children.iter().map(|c| name_of(c)).collect(),
        _ => panic!("not a group"),
    }
}

fn mask_names(s: &Session, owner: u64) -> Vec<String> {
    let d = s.active().expect("doc");
    let e = docops::element_on_page(&d.doc, d.page, ElementId(owner)).expect("owner");
    let m = e.element_mask().expect("masked").element;
    match &m.kind {
        ElementKind::Group(x) => x.children.iter().map(|c| name_of(c)).collect(),
        _ => panic!("mask is not a group"),
    }
}

fn undo_count(s: &Session) -> usize {
    s.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

fn undo_label(s: &Session) -> Option<String> {
    s.active()
        .and_then(|d| d.history.undo_label().map(str::to_string))
}

/// Scene `LD()` of RE spec 82 §1.1: "Ebene 1" top to bottom C, B, G (G2 over G1), A; "Ebene 2"
/// above it with D. Returns (session, layer 1, layer 2, ids by name).
fn ld() -> (Session, u64, u64, std::collections::HashMap<&'static str, u64>) {
    fp_i18n::set_language("de");
    let mut s = Session::default();
    s.prefs.set("MaxUndos", fp_engine::prefs::PrefValue::Int(100));
    ex(
        &mut s,
        "file.new",
        json!({"width": 600, "height": 400, "background": "#ffffff"}),
    );
    let l1 = s.active().and_then(|d| d.layer).expect("layer").0;
    let mut m = std::collections::HashMap::new();
    m.insert("A", rect(&mut s, "A", 10.0));
    m.insert("G1", rect(&mut s, "G1", 60.0));
    m.insert("G2", rect(&mut s, "G2", 110.0));
    let g = group(&mut s, &[m["G1"], m["G2"]], "G");
    m.insert("G", g);
    m.insert("B", rect(&mut s, "B", 160.0));
    m.insert("C", rect(&mut s, "C", 210.0));
    let l2 = ex(&mut s, "layer.add", json!({}))["layer"]
        .as_u64()
        .expect("layer 2");
    m.insert("D", rect(&mut s, "D", 260.0));
    ex(&mut s, "select.none", json!({}));
    assert_eq!(layer_names(&s, l1), ["C", "B", "G", "A"]);
    assert_eq!(layer_names(&s, l2), ["D"]);
    assert_eq!(child_names(&s, g), ["G2", "G1"]);
    (s, l1, l2, m)
}

fn move_to(s: &mut Session, ids: &[u64], layer: u64, index: usize) {
    ex(
        s,
        "pi.element.moveTo",
        json!({"ids": ids, "layer": layer, "index": index}),
    );
}

/// The measured drags of RE spec 82 §1.2 as (block, target layer, slot in the old list).
#[test]
fn layer_drags_as_measured() {
    // (source(s), layer 1 or 2, slot) → layer 1 / layer 2 afterwards
    type Case<'a> = (&'a [&'a str], u64, usize, &'a [&'a str], &'a [&'a str]);
    let cases: &[Case] = &[
        (&["A"], 1, 2, &["C", "B", "A", "G"], &["D"]), // A_Gtop
        (&["B"], 1, 3, &["C", "G", "B", "A"], &["D"]), // B_Gmid / B_Gbot
        (&["C"], 1, 2, &["B", "C", "G", "A"], &["D"]), // C_Bmid
        (&["A"], 1, 0, &["A", "C", "B", "G"], &["D"]), // A_Ctop, A_e1mid
        (&["G"], 1, 0, &["G", "C", "B", "A"], &["D"]), // G_Ctop
        (&["G"], 1, 4, &["C", "B", "A", "G"], &["D"]), // G_Abot
        (&["A"], 2, 0, &["C", "B", "G"], &["A", "D"]), // A_e2mid
        (&["A"], 2, 1, &["C", "B", "G"], &["D", "A"]), // A_Dmid
        (&["D"], 1, 4, &["C", "B", "G", "A", "D"], &[]), // D_Amid
        (&["A", "B"], 1, 0, &["B", "A", "C", "G"], &["D"]), // block, order kept
    ];
    for (src, layer, slot, e1, e2) in cases {
        let (mut s, l1, l2, m) = ld();
        let before = undo_count(&s);
        let ids: Vec<u64> = src.iter().map(|n| m[n]).collect();
        move_to(&mut s, &ids, if *layer == 1 { l1 } else { l2 }, *slot);
        assert_eq!(layer_names(&s, l1), *e1, "{src:?} → {layer}/{slot}");
        assert_eq!(layer_names(&s, l2), *e2, "{src:?} → {layer}/{slot}");
        assert_eq!(undo_count(&s), before + 1, "one History row");
        assert_eq!(undo_label(&s).as_deref(), Some("Ebenenänderung"));
        let d = s.active().expect("doc");
        let mut sel: Vec<u64> = d.selection.elements.iter().map(|e| e.0).collect();
        sel.sort();
        let mut want = ids.clone();
        want.sort();
        assert_eq!(sel, want, "the moved objects stay selected");
        assert_eq!(
            d.layer.map(|l| l.0),
            Some(if *layer == 1 { l1 } else { l2 }),
            "target layer current"
        );
    }
}

/// RE spec 82 §1.4: a drop at the old place changes nothing and writes no History row (the
/// measured "middle zone" step is not copied, TW-299).
#[test]
fn drop_at_the_old_place_is_a_silent_no_op() {
    let (mut s, l1, _, m) = ld();
    let before = undo_count(&s);
    let gen0 = s.active().expect("doc").generation;
    for (src, slot) in [("A", 3), ("A", 4), ("B", 1), ("B", 2), ("G", 2), ("G", 3)] {
        move_to(&mut s, &[m[src]], l1, slot);
    }
    // a block lying together: every gap at or inside it
    for slot in [1, 2, 3] {
        move_to(&mut s, &[m["B"], m["G"]], l1, slot);
    }
    assert_eq!(layer_names(&s, l1), ["C", "B", "G", "A"]);
    assert_eq!(undo_count(&s), before, "no History row");
    assert_eq!(s.active().expect("doc").generation, gen0, "not changed");
    // a scattered block is gathered even at its own gap
    move_to(&mut s, &[m["C"], m["G"]], l1, 0);
    assert_eq!(layer_names(&s, l1), ["C", "G", "B", "A"]);
    assert_eq!(undo_count(&s), before + 1);
}

#[test]
fn drop_is_noop_rule() {
    let it = |id: u64, owner: ListOwner, index: usize| DragItem {
        id: ElementId(id),
        path: fp_model::traverse::ElementPath {
            root: fp_model::traverse::RootRef::Page(0),
            slot: 0,
            cel: 0,
            steps: vec![fp_model::traverse::PathStep::Child(index)],
        },
        owner,
        index,
    };
    let l = ListOwner::Layer(LayerId(1));
    let g = ListOwner::Group(ElementId(9));
    let one = [it(1, l, 3)];
    assert!(docops::drop_is_noop(&one, l, 3) && docops::drop_is_noop(&one, l, 4));
    assert!(!docops::drop_is_noop(&one, l, 2) && !docops::drop_is_noop(&one, l, 5));
    assert!(!docops::drop_is_noop(&one, g, 3), "other list");
    let run = [it(1, l, 1), it(2, l, 2)];
    assert!((1..=3).all(|k| docops::drop_is_noop(&run, l, k)));
    assert!(!docops::drop_is_noop(&run, l, 0) && !docops::drop_is_noop(&run, l, 4));
    let gap = [it(1, l, 0), it(2, l, 2)];
    assert!((0..=3).all(|k| !docops::drop_is_noop(&gap, l, k)));
}

/// Firepixel extension (RE spec 82 §1.5): into a group, inside it, out of it; never into a mask
/// group, a group into itself or a sublayer into a group.
#[test]
fn drops_into_and_out_of_plain_groups() {
    let (mut s, l1, _, m) = ld();
    // B into G as its first object
    ex(
        &mut s,
        "pi.element.moveTo",
        json!({"ids": [m["B"]], "layer": l1, "index": 0, "group": m["G"]}),
    );
    assert_eq!(layer_names(&s, l1), ["C", "G", "A"]);
    assert_eq!(child_names(&s, m["G"]), ["B", "G2", "G1"]);
    assert_eq!(undo_label(&s).as_deref(), Some("Ebenenänderung"));
    assert_eq!(
        docops::drag_place(&s.active().expect("doc").doc, 0, ElementId(m["B"])),
        Some((ListOwner::Group(ElementId(m["G"])), 0))
    );
    // inside the group: G1 to the top
    ex(
        &mut s,
        "pi.element.moveTo",
        json!({"ids": [m["G1"]], "layer": l1, "index": 0, "group": m["G"]}),
    );
    assert_eq!(child_names(&s, m["G"]), ["G1", "B", "G2"]);
    // a no-op inside the group
    let n = undo_count(&s);
    ex(
        &mut s,
        "pi.element.moveTo",
        json!({"ids": [m["B"]], "layer": l1, "index": 2, "group": m["G"]}),
    );
    assert_eq!(undo_count(&s), n);
    // out of the group, below A; the group keeps its other objects
    move_to(&mut s, &[m["G2"]], l1, 3);
    assert_eq!(layer_names(&s, l1), ["C", "G", "A", "G2"]);
    assert_eq!(child_names(&s, m["G"]), ["G1", "B"]);
    // undo brings the object back into the group
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(child_names(&s, m["G"]), ["G1", "B", "G2"]);
    // a group never goes into itself
    assert!(
        s.execute(
            "pi.element.moveTo",
            json!({"ids": [m["G"]], "layer": l1, "index": 0, "group": m["G"]})
        )
        .is_err()
    );
    // a mask group is no target
    let x = rect(&mut s, "X", 300.0);
    let mk = rect(&mut s, "MK", 310.0);
    ex(&mut s, "select.set", json!({"ids": [x, mk]}));
    let mg = ex(&mut s, "modify.mask.groupAsMask", json!({}))["id"]
        .as_u64()
        .expect("mask group");
    assert!(
        s.execute(
            "pi.element.moveTo",
            json!({"ids": [m["C"]], "layer": l1, "index": 0, "group": mg})
        )
        .is_err()
    );
    // and the content of a mask group is no drag source
    let d = s.active().expect("doc");
    assert_eq!(docops::drag_place(&d.doc, 0, ElementId(x)), None);
    assert_eq!(docops::drag_place(&d.doc, 0, ElementId(mk)), None);
}

/// Scene `MK3()` of RE spec 82 §2.1: S masked by the group QM of Q1, Q2, Q3 (Q3 top); group H
/// of H3 over H2 over H1. Returns (session, ids by name).
fn mk3() -> (Session, std::collections::HashMap<&'static str, u64>) {
    fp_i18n::set_language("de");
    let mut s = Session::default();
    s.prefs.set("MaxUndos", fp_engine::prefs::PrefValue::Int(100));
    ex(
        &mut s,
        "file.new",
        json!({"width": 600, "height": 400, "background": "#ffffff"}),
    );
    let mut m = std::collections::HashMap::new();
    m.insert("S", rect(&mut s, "S", 40.0));
    m.insert("Q1", rect(&mut s, "Q1", 40.0));
    m.insert("Q2", rect(&mut s, "Q2", 60.0));
    m.insert("Q3", rect(&mut s, "Q3", 90.0));
    let qm = group(&mut s, &[m["Q1"], m["Q2"], m["Q3"]], "QM");
    m.insert("QM", qm);
    ex(&mut s, "select.set", json!({"ids": [m["S"], qm]}));
    let owner = ex(&mut s, "modify.mask.groupAsMask", json!({}))["id"]
        .as_u64()
        .expect("mask group");
    m.insert("MS", owner);
    m.insert("H1", rect(&mut s, "H1", 300.0));
    m.insert("H2", rect(&mut s, "H2", 350.0));
    m.insert("H3", rect(&mut s, "H3", 400.0));
    let h = group(&mut s, &[m["H1"], m["H2"], m["H3"]], "H");
    m.insert("H", h);
    ex(&mut s, "select.none", json!({}));
    assert_eq!(mask_names(&s, owner), ["Q3", "Q2", "Q1"]);
    assert_eq!(child_names(&s, h), ["H3", "H2", "H1"]);
    (s, m)
}

fn arrange(s: &mut Session, sel: u64, cmd: &str) {
    ex(s, "select.set", json!({"ids": [sel]}));
    ex(s, cmd, json!({}));
}

/// RE spec 82 §2.2: Arrange reorders an object of a mask group or of a group among its
/// siblings; a command that cannot move anything stays silent (Firepixel, TW-300).
#[test]
fn arrange_inside_mask_groups_and_groups() {
    let (mut s, m) = mk3();
    let ms = m["MS"];
    let n0 = undo_count(&s);
    arrange(&mut s, m["Q1"], "modify.arrange.bringToFront");
    assert_eq!(mask_names(&s, ms), ["Q1", "Q3", "Q2"]);
    assert_eq!(undo_count(&s), n0 + 1);
    assert!(undo_label(&s).is_some_and(|l| l.starts_with("In den Vordergrund")));
    // already in front: no change, no row (deliberate deviation from the measured step)
    arrange(&mut s, m["Q1"], "modify.arrange.bringToFront");
    assert_eq!(undo_count(&s), n0 + 1);
    arrange(&mut s, m["Q1"], "modify.arrange.sendToBack");
    assert_eq!(mask_names(&s, ms), ["Q3", "Q2", "Q1"]);
    arrange(&mut s, m["Q1"], "modify.arrange.bringForward");
    assert_eq!(mask_names(&s, ms), ["Q3", "Q1", "Q2"]);
    arrange(&mut s, m["Q1"], "modify.arrange.sendBackward");
    assert_eq!(mask_names(&s, ms), ["Q3", "Q2", "Q1"]);
    // the whole mask selected: nothing to reorder, nothing happens
    let n1 = undo_count(&s);
    arrange(&mut s, m["QM"], "modify.arrange.sendToBack");
    assert_eq!(mask_names(&s, ms), ["Q3", "Q2", "Q1"]);
    assert_eq!(undo_count(&s), n1);
    // undo of a mask reorder
    arrange(&mut s, m["Q2"], "modify.arrange.bringToFront");
    assert_eq!(mask_names(&s, ms), ["Q2", "Q3", "Q1"]);
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(mask_names(&s, ms), ["Q3", "Q2", "Q1"]);
    // an object of a plain group
    let h = m["H"];
    arrange(&mut s, m["H1"], "modify.arrange.bringToFront");
    assert_eq!(child_names(&s, h), ["H1", "H3", "H2"]);
    arrange(&mut s, m["H1"], "modify.arrange.sendBackward");
    assert_eq!(child_names(&s, h), ["H3", "H1", "H2"]);
    assert!(undo_label(&s).is_some_and(|l| l.contains("nach hinten")));
}

fn hover(s: &mut Session, x: f32, y: f32) -> Cursor {
    s.tool_event(&ToolEvent::drag(x, y)).expect("hover").cursor
}

/// RE spec 82 §5: the Pointer keeps its black arrow over the anchors of a selected path; the
/// Subselection shows the hollow arrowhead there, also over the corner of a selected rectangle.
#[test]
fn cursors_over_anchors() {
    fp_i18n::set_language("de");
    let mut s = Session::default();
    ex(
        &mut s,
        "file.new",
        json!({"width": 300, "height": 200, "background": "#ffffff"}),
    );
    let r = rect(&mut s, "R", 100.0);
    let p = ex(
        &mut s,
        "draw.path",
        json!({"contours": [{"closed": true, "nodes": [
            {"anchor": [10.0, 100.0]}, {"anchor": [60.0, 150.0]}, {"anchor": [20.0, 180.0]}
        ]}]}),
    )["id"]
        .as_u64()
        .expect("path");
    ex(&mut s, "select.set", json!({"ids": [p]}));
    s.set_tool(ToolId::Pointer).expect("pointer");
    assert_eq!(hover(&mut s, 60.0, 150.0), Cursor::Arrow, "Pointer over an anchor");
    assert_eq!(hover(&mut s, 250.0, 150.0), Cursor::Arrow);
    s.set_tool(ToolId::Subselection).expect("subselection");
    assert_eq!(hover(&mut s, 60.0, 150.0), Cursor::SubselectPoint);
    assert_eq!(hover(&mut s, 250.0, 150.0), Cursor::Subselect);
    ex(&mut s, "select.set", json!({"ids": [r]}));
    assert_eq!(
        hover(&mut s, 140.0, 20.0),
        Cursor::SubselectPoint,
        "corner of the selected rectangle"
    );
    assert_eq!(hover(&mut s, 120.0, 40.0), Cursor::Subselect, "its fill");
    s.set_tool(ToolId::Pointer).expect("pointer");
    assert_ne!(hover(&mut s, 140.0, 20.0), Cursor::PointerPoint);
}
