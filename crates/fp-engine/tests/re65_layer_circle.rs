//! RE spec 65: the click on a layer's selection circle moves (Alt: copies) the selection to the top
//! of that layer as one History row "Layer Change" (TW-217), and the current layer follows the
//! chosen layer of the page: the first unlocked layer from it upward with a wrap from flat index 0,
//! re-resolved after every lock and unlock, remembered per page, adopted after a state change.

use crate::common::*;
use fp_engine::{Session, docops};
use fp_model::{ElementId, ElementKind, LayerId};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;

// ------------------------------------------------------------------ helpers

fn cur(s: &Session) -> LayerId {
    s.active().and_then(|d| d.layer).expect("current layer")
}

fn flat(s: &Session) -> Vec<LayerId> {
    let d = s.active().expect("doc");
    docops::flat_layers(&d.doc, d.page, d.frame)
        .into_iter()
        .map(|l| l.id)
        .collect()
}

fn cur_idx(s: &Session) -> usize {
    let c = cur(s);
    flat(s)
        .iter()
        .position(|l| *l == c)
        .expect("current layer in the flat list")
}

fn sel(s: &Session) -> Vec<ElementId> {
    s.active()
        .map(|d| d.selection.elements.clone())
        .unwrap_or_default()
}

fn rows(s: &Session) -> usize {
    s.active().map(|d| d.history.len()).unwrap_or(0)
}

fn named(s: &mut Session, id: ElementId, n: &str) -> ElementId {
    ex(s, "element.setName", json!({"ids": [id.0], "name": n}));
    id
}

fn add_layer(s: &mut Session, name: &str) -> LayerId {
    LayerId(
        ex(s, "layer.add", json!({"name": name}))["layer"]
            .as_u64()
            .expect("layer"),
    )
}

fn set_cur(s: &mut Session, l: LayerId) {
    ex(s, "layer.setActive", json!({"layer": l.0}));
}

fn lock(s: &mut Session, l: LayerId, v: bool) {
    ex(s, "layer.setLocked", json!({"layer": l.0, "locked": v}));
}

fn move_to(s: &mut Session, l: LayerId, copy: bool) {
    ex(
        s,
        "selection.moveToLayer",
        json!({"layer": l.0, "copy": copy}),
    );
}

fn web(s: &Session) -> LayerId {
    let d = s.active().expect("doc");
    docops::web_layer(&d.doc, d.page).expect("web layer")
}

/// The stack of a layer or sublayer in the current state, top first: object names, groups as
/// `g{a/b}`, sublayers as `[S1]`.
fn objs(s: &Session, l: LayerId) -> Vec<String> {
    fn show(e: &fp_model::Element) -> String {
        let n = e.common.name.clone().unwrap_or_else(|| "?".into());
        match &e.kind {
            ElementKind::Group(g) => {
                let kids: Vec<String> = g.children.iter().map(|c| show(c)).collect();
                format!("{n}{{{}}}", kids.join("/"))
            }
            ElementKind::SubLayer(sl) => format!("[{}]", sl.name),
            _ => n,
        }
    }
    let d = s.active().expect("doc");
    let layer = docops::layer_by_id(&d.doc, d.page, l).expect("layer");
    layer
        .cel(d.frame)
        .map(|c| c.elements.iter().map(|e| show(e)).collect())
        .unwrap_or_default()
}

fn tmp(name: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(1);
    std::env::temp_dir().join(format!(
        "fp-re65-{}-{}-{name}.fw.png",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ))
}

// ------------------------------------------------------------------ §1 the circle (TW-217)

/// `mka` of RE spec 65 §1.1: L1 {r1}, L2 {r2}, L3 {r3b over r3a}; nothing selected, L3 current.
struct Mka {
    s: Session,
    l: [LayerId; 3],
    r1: ElementId,
    r2: ElementId,
    r3a: ElementId,
    r3b: ElementId,
}

fn mka() -> Mka {
    fp_i18n::set_language("de");
    let mut s = doc_session(600.0, 400.0);
    let l1 = cur(&s);
    ex(&mut s, "layer.rename", json!({"name": "L1"}));
    let r1 = rect(&mut s, 50.0, 50.0, 130.0, 110.0);
    named(&mut s, r1, "r1");
    let l2 = add_layer(&mut s, "L2");
    let r2 = rect(&mut s, 200.0, 100.0, 280.0, 160.0);
    named(&mut s, r2, "r2");
    let l3 = add_layer(&mut s, "L3");
    let r3a = rect(&mut s, 350.0, 150.0, 430.0, 210.0);
    named(&mut s, r3a, "r3a");
    let r3b = rect(&mut s, 360.0, 160.0, 440.0, 220.0);
    named(&mut s, r3b, "r3b");
    ex(&mut s, "select.none", json!({}));
    assert_eq!(cur(&s), l3);
    assert_eq!(objs(&s, l3), ["r3b", "r3a"]);
    Mka {
        s,
        l: [l1, l2, l3],
        r1,
        r2,
        r3a,
        r3b,
    }
}

#[test]
fn c1_circle_moves_one_object_to_the_top_of_the_layer() {
    let mut m = mka();
    select(&mut m.s, &[m.r1]);
    assert_eq!(cur(&m.s), m.l[0]);
    let n = rows(&m.s);
    move_to(&mut m.s, m.l[2], false);
    assert_eq!(objs(&m.s, m.l[2]), ["r1", "r3b", "r3a"]);
    assert!(objs(&m.s, m.l[0]).is_empty());
    assert_eq!(sel(&m.s), [m.r1], "the moved object stays selected");
    assert_eq!(cur(&m.s), m.l[2], "the target becomes current");
    assert_eq!(rows(&m.s), n + 1, "one History row");
    let d = m.s.active().expect("doc");
    let top = d.history.top().expect("row");
    assert_eq!(top.label, "Ebenenänderung");
    // the row's script text: fp.run("selection.moveToLayer", {"layer": <target>, "copy": false})
    assert_eq!(top.command, "selection.moveToLayer");
    assert_eq!(top.params["layer"].as_u64(), Some(m.l[2].0));
    assert_eq!(top.params["copy"].as_bool(), Some(false));
    // C1u: undo restores layers and selection, the single object's layer is current
    m.s.undo().expect("undo");
    assert_eq!(objs(&m.s, m.l[0]), ["r1"]);
    assert_eq!(objs(&m.s, m.l[2]), ["r3b", "r3a"]);
    assert_eq!(sel(&m.s), [m.r1]);
    assert_eq!(cur(&m.s), m.l[0]);
    // C1r
    m.s.redo().expect("redo");
    assert_eq!(objs(&m.s, m.l[2]), ["r1", "r3b", "r3a"]);
    assert_eq!(cur(&m.s), m.l[2]);
}

#[test]
fn the_label_is_layer_change_in_english() {
    fp_i18n::set_language("en");
    assert_eq!(fp_i18n::t("cmd-selection-moveToLayer"), "Layer Change");
    fp_i18n::set_language("de");
    assert_eq!(fp_i18n::t("cmd-selection-moveToLayer"), "Ebenenänderung");
}

#[test]
fn c2_several_objects_move_as_one_block() {
    // C2a: [r2, r1] → L3 as the block r2/r1 on top
    let mut m = mka();
    set_cur(&mut m.s, m.l[0]);
    select(&mut m.s, &[m.r2, m.r1]);
    assert_eq!(cur(&m.s), m.l[0], "two objects keep the current layer L1");
    move_to(&mut m.s, m.l[2], false);
    assert_eq!(objs(&m.s, m.l[2]), ["r2", "r1", "r3b", "r3a"]);
    let mut s1 = sel(&m.s);
    s1.sort();
    let mut want = vec![m.r1, m.r2];
    want.sort();
    assert_eq!(s1, want, "both stay selected");
    assert_eq!(cur(&m.s), m.l[2]);
    // C2a-u: undo puts them back, the target stays current (two objects)
    m.s.undo().expect("undo");
    assert_eq!(objs(&m.s, m.l[0]), ["r1"]);
    assert_eq!(objs(&m.s, m.l[1]), ["r2"]);
    assert_eq!(sel(&m.s).len(), 2);
    assert_eq!(cur(&m.s), m.l[2], "L3 stays current after undo, not L1");
    // C2b: [r3b, r3a] → L1
    let mut m = mka();
    select(&mut m.s, &[m.r3b, m.r3a]);
    move_to(&mut m.s, m.l[0], false);
    assert_eq!(objs(&m.s, m.l[0]), ["r3b", "r3a", "r1"]);
    assert!(objs(&m.s, m.l[2]).is_empty());
    assert_eq!(cur(&m.s), m.l[0]);
    // C2f: [r3a, r1] → L2
    let mut m = mka();
    select(&mut m.s, &[m.r3a, m.r1]);
    move_to(&mut m.s, m.l[1], false);
    assert_eq!(objs(&m.s, m.l[1]), ["r3a", "r1", "r2"]);
    assert_eq!(objs(&m.s, m.l[2]), ["r3b"]);
    assert_eq!(cur(&m.s), m.l[1]);
}

#[test]
fn c4c_alt_copies_and_selects_the_copy() {
    let mut m = mka();
    select(&mut m.s, &[m.r1]);
    let n = rows(&m.s);
    let b = bounds(&m.s, m.r1);
    move_to(&mut m.s, m.l[2], true);
    assert_eq!(objs(&m.s, m.l[2]), ["r1", "r3b", "r3a"]);
    assert_eq!(objs(&m.s, m.l[0]), ["r1"], "the original stays");
    let s1 = sel(&m.s);
    assert_eq!(s1.len(), 1);
    assert_ne!(s1[0], m.r1, "the copy is selected");
    assert!(
        rect_approx(bounds(&m.s, s1[0]), b),
        "the copy is not offset"
    );
    assert_eq!(cur(&m.s), m.l[2]);
    assert_eq!(rows(&m.s), n + 1);
    let d = m.s.active().expect("doc");
    let top = d.history.top().expect("row");
    assert_eq!(top.label, "Ebenenänderung");
    assert_eq!(top.params["copy"].as_bool(), Some(true));
    m.s.undo().expect("undo");
    assert_eq!(objs(&m.s, m.l[2]), ["r3b", "r3a"]);
    assert_eq!(sel(&m.s), [m.r1]);
}

/// The circle does nothing (no History row, layers, selection and current layer unchanged).
fn assert_no_op(m: &mut Mka, target: LayerId, copy: bool) {
    let before: Vec<Vec<String>> = m.l.iter().map(|l| objs(&m.s, *l)).collect();
    let (n, c, s0) = (rows(&m.s), cur(&m.s), sel(&m.s));
    move_to(&mut m.s, target, copy);
    let after: Vec<Vec<String>> = m.l.iter().map(|l| objs(&m.s, *l)).collect();
    assert_eq!(after, before);
    assert_eq!((rows(&m.s), cur(&m.s), sel(&m.s)), (n, c, s0));
}

#[test]
fn circle_no_op_cases() {
    // C2c: [r3a, r1] and the filled circle of L3 (holds r3a)
    let mut m = mka();
    select(&mut m.s, &[m.r3a, m.r1]);
    let t = m.l[2];
    assert_no_op(&mut m, t, false);
    assert_no_op(&mut m, t, true);
    // C2g: the filled circle of L1
    let t = m.l[0];
    assert_no_op(&mut m, t, false);
    // C2d: [r3a] and its own layer
    let mut m = mka();
    select(&mut m.s, &[m.r3a]);
    let t = m.l[2];
    assert_no_op(&mut m, t, false);
    // C2e: nothing selected – the current layer L3 stays
    let mut m = mka();
    let t = m.l[0];
    assert_no_op(&mut m, t, false);
    assert_eq!(cur(&m.s), m.l[2]);
    // B7: locked target
    let mut m = mka();
    select(&mut m.s, &[m.r1]);
    let t = m.l[2];
    lock(&mut m.s, t, true);
    assert_no_op(&mut m, t, false);
    // B6: the web layer as target for a normal object
    let mut m = mka();
    select(&mut m.s, &[m.r1]);
    let w = web(&m.s);
    assert_no_op(&mut m, w, false);
    // E3: a selected slice and a normal layer
    let mut m = mka();
    let sl = ElementId(
        ex(
            &mut m.s,
            "web.addSlice",
            json!({"shape": "rect", "rect": [500, 300, 560, 360]}),
        )["id"]
            .as_u64()
            .expect("slice"),
    );
    select(&mut m.s, &[sl]);
    let t = m.l[2];
    assert_no_op(&mut m, t, false);
    // E4: slice present, [r1] and the web layer
    select(&mut m.s, &[m.r1]);
    let w = web(&m.s);
    assert_no_op(&mut m, w, false);
}

#[test]
fn hidden_and_new_empty_layers_take_the_selection() {
    // B8: a hidden target layer
    let mut m = mka();
    ex(
        &mut m.s,
        "layer.setVisible",
        json!({"layer": m.l[2].0, "visible": false}),
    );
    select(&mut m.s, &[m.r1]);
    move_to(&mut m.s, m.l[2], false);
    assert_eq!(objs(&m.s, m.l[2]), ["r1", "r3b", "r3a"]);
    assert_eq!(sel(&m.s), [m.r1]);
    assert_eq!(cur(&m.s), m.l[2]);
    // E1: a new empty layer
    let mut m = mka();
    let l4 = add_layer(&mut m.s, "L4");
    select(&mut m.s, &[m.r1]);
    move_to(&mut m.s, l4, false);
    assert_eq!(objs(&m.s, l4), ["r1"]);
    assert_eq!(cur(&m.s), l4);
    let _ = (m.r2, m.r3b);
}

/// `mkb` of RE spec 65 §1.2: L1 {sublayer S1 {s1} over r1}, L2 {g(gb over ga) over r2}, L3 {r3}.
struct Mkb {
    s: Session,
    l1: LayerId,
    sub: LayerId,
    l2: LayerId,
    l3: LayerId,
    r1: ElementId,
    s1: ElementId,
    g: ElementId,
    ga: ElementId,
}

fn mkb() -> Mkb {
    fp_i18n::set_language("de");
    let mut s = doc_session(600.0, 400.0);
    let l1 = cur(&s);
    ex(&mut s, "layer.rename", json!({"name": "L1"}));
    let r1 = rect(&mut s, 20.0, 20.0, 60.0, 60.0);
    named(&mut s, r1, "r1");
    ex(&mut s, "pi.layer.addSub", json!({"name": "S1"}));
    let sub = cur(&s);
    let s1 = rect(&mut s, 80.0, 20.0, 120.0, 60.0);
    named(&mut s, s1, "s1");
    set_cur(&mut s, l1);
    let l2 = add_layer(&mut s, "L2");
    let r2 = rect(&mut s, 20.0, 120.0, 60.0, 160.0);
    named(&mut s, r2, "r2");
    let ga = rect(&mut s, 80.0, 120.0, 120.0, 160.0);
    named(&mut s, ga, "ga");
    let gb = rect(&mut s, 140.0, 120.0, 180.0, 160.0);
    named(&mut s, gb, "gb");
    select(&mut s, &[ga, gb]);
    let g = ElementId(
        ex(&mut s, "modify.group", json!({}))["ids"][0]
            .as_u64()
            .or_else(|| sel(&s).first().map(|e| e.0))
            .expect("group"),
    );
    named(&mut s, g, "g");
    let l3 = add_layer(&mut s, "L3");
    let r3 = rect(&mut s, 20.0, 220.0, 60.0, 260.0);
    named(&mut s, r3, "r3");
    ex(&mut s, "select.none", json!({}));
    assert_eq!(objs(&s, l1), ["[S1]", "r1"]);
    assert_eq!(objs(&s, sub), ["s1"]);
    assert_eq!(objs(&s, l2), ["g{gb/ga}", "r2"]);
    assert_eq!(flat(&s)[..4], [l1, sub, l2, l3]);
    Mkb {
        s,
        l1,
        sub,
        l2,
        l3,
        r1,
        s1,
        g,
        ga,
    }
}

#[test]
fn b2_b5_sublayers_and_groups() {
    // B2: [r1] → S1
    let mut m = mkb();
    select(&mut m.s, &[m.r1]);
    move_to(&mut m.s, m.sub, false);
    assert_eq!(objs(&m.s, m.sub), ["r1", "s1"]);
    assert_eq!(objs(&m.s, m.l1), ["[S1]"]);
    assert_eq!(cur(&m.s), m.sub);
    assert_eq!(sel(&m.s), [m.r1]);
    // B3: [s1] → L1, above the sublayer
    let mut m = mkb();
    select(&mut m.s, &[m.s1]);
    assert_eq!(cur(&m.s), m.sub);
    move_to(&mut m.s, m.l1, false);
    assert_eq!(objs(&m.s, m.l1), ["s1", "[S1]", "r1"]);
    assert!(objs(&m.s, m.sub).is_empty());
    assert_eq!(cur(&m.s), m.l1);
    // B4: a selected group goes whole
    let mut m = mkb();
    select(&mut m.s, &[m.g]);
    move_to(&mut m.s, m.l3, false);
    assert_eq!(objs(&m.s, m.l3), ["g{gb/ga}", "r3"]);
    assert_eq!(objs(&m.s, m.l2), ["r2"]);
    assert_eq!(cur(&m.s), m.l3);
    // B5: a subselected object leaves its group alone
    let mut m = mkb();
    select(&mut m.s, &[m.ga]);
    assert_eq!(cur(&m.s), m.l2, "the group's layer is current");
    move_to(&mut m.s, m.l3, false);
    assert_eq!(objs(&m.s, m.l3), ["ga", "r3"]);
    assert_eq!(objs(&m.s, m.l2), ["g{gb}", "r2"]);
    assert_eq!(sel(&m.s), [m.ga]);
    assert_eq!(cur(&m.s), m.l3);
    // a sublayer locked through its parent takes nothing
    let mut m = mkb();
    lock(&mut m.s, m.l1, true);
    select(&mut m.s, &[m.g]);
    let n = rows(&m.s);
    move_to(&mut m.s, m.sub, false);
    assert_eq!(rows(&m.s), n);
    assert_eq!(objs(&m.s, m.l2), ["g{gb/ga}", "r2"]);
}

// ------------------------------------------------------------------ §2.3 lock rule (U1–U23)

/// `mk5` of RE spec 65 §2.3: flat 0 L1 {r1}, 1 S1 (sublayer of L1), 2 L2, 3 L3, 4 L4, 5 L5,
/// 6 web layer.
fn mk5() -> (Session, Vec<LayerId>, ElementId) {
    fp_i18n::set_language("de");
    let mut s = doc_session(600.0, 400.0);
    let l1 = cur(&s);
    ex(&mut s, "layer.rename", json!({"name": "L1"}));
    let r1 = rect(&mut s, 20.0, 20.0, 60.0, 60.0);
    ex(&mut s, "pi.layer.addSub", json!({"name": "S1"}));
    set_cur(&mut s, l1);
    for i in 2..=5 {
        add_layer(&mut s, &format!("L{i}"));
    }
    ex(&mut s, "select.none", json!({}));
    let f = flat(&s);
    assert_eq!(f.len(), 7);
    assert_eq!(f[6], web(&s));
    (s, f, r1)
}

#[derive(Clone, Copy, Debug)]
enum U {
    Cur(usize),
    Lock(usize),
    Unlock(usize),
    Hide(usize),
    /// Lock or unlock the sublayer S1 like the Layers panel (its element).
    LockSubElement(bool),
    SelectR1,
}

/// Runs the steps on a fresh `mk5` and returns the flat index of the current layer after each.
fn run_u(steps: &[U]) -> Vec<usize> {
    let (mut s, f, r1) = mk5();
    let mut out = Vec::new();
    for st in steps {
        match *st {
            U::Cur(i) => set_cur(&mut s, f[i]),
            U::Lock(i) => lock(&mut s, f[i], true),
            U::Unlock(i) => lock(&mut s, f[i], false),
            U::Hide(i) => {
                ex(
                    &mut s,
                    "layer.setVisible",
                    json!({"layer": f[i].0, "visible": false}),
                );
            }
            U::LockSubElement(v) => {
                let d = s.active().expect("doc");
                let el = docops::sublayer_element(&d.doc, d.page, f[1]).expect("S1 element");
                ex(
                    &mut s,
                    "element.setLocked",
                    json!({"ids": [el.0], "locked": v}),
                );
            }
            U::SelectR1 => select(&mut s, &[r1]),
        }
        out.push(cur_idx(&s));
    }
    out
}

#[test]
fn u_lock_rule_cases() {
    use U::*;
    let cases: &[(&str, &[U], usize)] = &[
        ("U1", &[Cur(2), Lock(2)], 3),
        ("U2", &[Cur(2), Hide(3), Lock(2)], 3),
        ("U3", &[Cur(2), Lock(3), Lock(2)], 4),
        ("U4", &[Cur(2), Lock(3), Lock(4), Lock(5), Lock(2)], 0), // FB-64: not the web layer
        (
            "U5",
            &[Cur(2), Lock(3), Lock(4), Lock(5), Lock(6), Lock(2)],
            0,
        ),
        ("U6", &[Cur(5), Lock(5)], 0), // FB-64: not the web layer
        ("U7", &[Cur(5), Lock(6), Lock(5)], 0),
        ("U8", &[Cur(0), Lock(0)], 2),
        ("U9", &[Cur(1), Lock(1)], 2),
        ("U9 panel", &[Cur(1), LockSubElement(true)], 2),
        ("U10", &[Cur(4), Lock(0), Lock(5), Lock(6), Lock(4)], 2),
        (
            "U11",
            &[Cur(5), Lock(0), Lock(2), Lock(3), Lock(4), Lock(6), Lock(5)],
            0,
        ),
        ("U14", &[Cur(6), Lock(6)], 0),
        ("U13", &[Cur(2), Lock(4)], 2),
        ("U15", &[Lock(4), Cur(2), Unlock(4)], 2),
        ("U17", &[Cur(2), Hide(2)], 2),
        ("U18", &[Cur(2), Unlock(4)], 2),
        ("U23", &[Cur(2), Lock(2), SelectR1, Unlock(2)], 0),
    ];
    for (name, steps, want) in cases {
        let got = run_u(steps);
        assert_eq!(got.last().copied(), Some(*want), "{name}: {got:?}");
    }
    // the sequences with the value after each step
    assert_eq!(run_u(&[Cur(3), Lock(3), Unlock(3)]), [3, 4, 3], "U12");
    assert_eq!(
        run_u(&[Cur(3), Lock(3), Cur(5), Unlock(3)]),
        [3, 4, 5, 5],
        "U16"
    );
    assert_eq!(
        run_u(&[Cur(2), Lock(2), Lock(3), Unlock(2), Unlock(3)]),
        [2, 3, 4, 2, 2],
        "U19"
    );
    assert_eq!(
        run_u(&[Cur(2), Lock(2), Lock(3), Unlock(3), Unlock(2)]),
        [2, 3, 4, 3, 2],
        "U20"
    );
    assert_eq!(
        run_u(&[Cur(5), Lock(5), Lock(6), Unlock(6)]),
        [5, 0, 0, 0],
        "U22 (FB-64)"
    );
    assert_eq!(
        run_u(&[Cur(1), LockSubElement(true), LockSubElement(false)]),
        [1, 2, 1],
        "S1 in the panel"
    );
}

#[test]
fn resolve_layer_wraps_from_index_0() {
    let (mut s, f, _) = mk5();
    for i in [0, 5, 6] {
        lock(&mut s, f[i], true);
    }
    let d = s.active().expect("doc");
    // U10: from L4 (4) locked: 5, 6 locked → wrap: 0, 1 (inherits) locked → 2
    let mut doc = (*d.doc).clone();
    docops::edit_layer_by_id(&mut doc, 0, f[4], |l| {
        for c in &mut l.cels {
            Arc::make_mut(c).locked = true;
        }
    });
    assert_eq!(docops::resolve_layer(&doc, 0, 0, f[4]), Some(f[2]));
    assert_eq!(docops::resolve_layer(&doc, 0, 0, f[3]), Some(f[3]));
    assert_eq!(docops::resolve_layer(&doc, 0, 0, LayerId(u64::MAX)), None);
}

// ------------------------------------------------------------------ §2.1 page change

/// `mkp` of RE spec 65 §2.1: page 0 L1/L2/L3, page 1 P1/P2/P3; page 1 and P3 current.
fn mkp() -> (Session, [LayerId; 3], [LayerId; 3]) {
    fp_i18n::set_language("de");
    let mut s = doc_session(600.0, 400.0);
    let l1 = cur(&s);
    ex(&mut s, "layer.rename", json!({"name": "L1"}));
    let l2 = add_layer(&mut s, "L2");
    let l3 = add_layer(&mut s, "L3");
    ex(&mut s, "page.add", json!({}));
    assert_eq!(s.active().map(|d| d.page), Some(1));
    // P8: a new page starts on its layer 0
    assert_eq!(cur_idx(&s), 0);
    let p1 = cur(&s);
    ex(&mut s, "layer.rename", json!({"name": "P1"}));
    let p2 = add_layer(&mut s, "P2");
    let p3 = add_layer(&mut s, "P3");
    assert_eq!(cur(&s), p3);
    (s, [l1, l2, l3], [p1, p2, p3])
}

fn page(s: &mut Session, i: usize) {
    ex(s, "page.setCurrent", json!({"index": i}));
}

#[test]
fn p_every_page_keeps_its_chosen_layer() {
    let (mut s, l, p) = mkp();
    page(&mut s, 0);
    assert_eq!(cur(&s), l[2], "P1: the layer current when page 0 was left");
    set_cur(&mut s, l[1]);
    page(&mut s, 1);
    assert_eq!(cur(&s), p[2], "P3");
    page(&mut s, 0);
    assert_eq!(
        cur(&s),
        l[1],
        "P4: remembered, neither the top nor the bottom layer"
    );
    // P5: the menu steps alike
    ex(&mut s, "view.page.next", json!({}));
    assert_eq!(cur(&s), p[2]);
    ex(&mut s, "view.page.previous", json!({}));
    assert_eq!(cur(&s), l[1]);
    // P6
    page(&mut s, 1);
    set_cur(&mut s, p[0]);
    page(&mut s, 0);
    assert_eq!(cur(&s), l[1]);
    page(&mut s, 1);
    assert_eq!(cur(&s), p[0]);
    // P8: a new page starts on layer 0
    ex(&mut s, "page.add", json!({}));
    assert_eq!(cur_idx(&s), 0);
}

#[test]
fn p7_first_visit_after_opening_takes_the_bottom_unlocked_layer() {
    let (mut s, _, p) = mkp();
    lock(&mut s, p[0], true);
    set_cur(&mut s, p[2]);
    let path = tmp("p7");
    ex(
        &mut s,
        "file.saveAs",
        json!({"path": path.to_string_lossy()}),
    );
    ex(&mut s, "file.open", json!({"path": path.to_string_lossy()}));
    let _ = std::fs::remove_file(&path);
    page(&mut s, 1);
    assert_eq!(
        cur_idx(&s),
        1,
        "P2: the bottom-most unlocked layer, not the saved P3"
    );
    let opened = flat(&s);
    set_cur(&mut s, opened[2]);
    page(&mut s, 0);
    page(&mut s, 1);
    assert_eq!(cur_idx(&s), 2);
}

#[test]
fn u24_the_chosen_layer_survives_a_page_change() {
    let (mut s, l, _) = mkp();
    page(&mut s, 0);
    set_cur(&mut s, l[1]);
    lock(&mut s, l[1], true);
    assert_eq!(cur(&s), l[2]);
    page(&mut s, 1);
    page(&mut s, 0);
    assert_eq!(cur(&s), l[2]);
    lock(&mut s, l[1], false);
    assert_eq!(cur(&s), l[1], "L2 is still the chosen layer of page 0");
}

// ------------------------------------------------------------------ §2.2 state change

/// `mkaf`: `mka` with a second state, state 1 current; `locked` are locked in state 2 only.
fn mkaf(locked: &[usize], current: usize) -> (Mka, LayerId) {
    let mut m = mka();
    ex(&mut m.s, "state.add", json!({"count": 1}));
    ex(&mut m.s, "state.setCurrent", json!({"index": 0}));
    let w = web(&m.s);
    for &i in locked {
        let id = if i == 3 { w } else { m.l[i] };
        let d = m.s.active_mut().expect("doc");
        let page = d.page;
        docops::edit_layer_by_id(d.doc_mut(), page, id, |l| {
            let k = if l.cels.len() > 1 { 1 } else { 0 };
            Arc::make_mut(&mut l.cels[k]).locked = true;
        })
        .expect("layer");
    }
    let id = if current == 3 { w } else { m.l[current] };
    set_cur(&mut m.s, id);
    (m, w)
}

fn state(s: &mut Session, i: usize) {
    ex(s, "state.setCurrent", json!({"index": i}));
}

#[test]
fn f_state_change_resolves_and_adopts() {
    // F1
    let (mut m, _) = mkaf(&[1], 1);
    assert_eq!(cur(&m.s), m.l[1]);
    state(&mut m.s, 1);
    assert_eq!(cur(&m.s), m.l[2], "F1: L2 locked in state 2 → L3");
    state(&mut m.s, 0);
    assert_eq!(cur(&m.s), m.l[2], "F1: L3 stays in state 1");
    // F2
    let (mut m, _) = mkaf(&[0, 1], 0);
    state(&mut m.s, 1);
    assert_eq!(cur(&m.s), m.l[2]);
    state(&mut m.s, 0);
    assert_eq!(cur(&m.s), m.l[2]);
    // F3
    let (mut m, w) = mkaf(&[0, 1, 2], 2);
    state(&mut m.s, 1);
    assert_eq!(cur(&m.s), w, "F3: the web layer");
    state(&mut m.s, 0);
    assert_eq!(cur(&m.s), w);
    // F4: the menu's state steps alike
    let (mut m, _) = mkaf(&[1], 1);
    ex(&mut m.s, "view.state.next", json!({}));
    assert_eq!(cur(&m.s), m.l[2]);
    ex(&mut m.s, "view.state.previous", json!({}));
    assert_eq!(cur(&m.s), m.l[2]);
    // F5
    let (mut m, _) = mkaf(&[], 0);
    state(&mut m.s, 1);
    assert_eq!(cur(&m.s), m.l[0]);
    // F6: L3 and the web layer locked → wrap to index 0
    let (mut m, _) = mkaf(&[2, 3], 2);
    state(&mut m.s, 1);
    assert_eq!(cur_idx(&m.s), 0);
    // F7: unlocking L2 in state 2 keeps L3
    let (mut m, _) = mkaf(&[1], 1);
    state(&mut m.s, 1);
    assert_eq!(cur(&m.s), m.l[2]);
    lock(&mut m.s, m.l[1], false);
    assert_eq!(cur(&m.s), m.l[2], "F7");
}
