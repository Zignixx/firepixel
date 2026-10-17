//! T-5 (re 30): Q-23 instance name through the Properties inspector, TW-56a expand stroke with the
//! preliminary simplify/refit (P1 width 10 → pixelRect top 167).

use crate::common::*;
use fp_engine::instance_name::{name_sole_unnamed_instance, unique_symbol_name};
use fp_model::ElementId;
use serde_json::json;

fn history_len(s: &fp_engine::Session) -> usize {
    s.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

/// Q-23: convert gives no name; the inspector refresh writes the symbol name without a history
/// step; the dialog proposes "Symbol", "Symbol 1", "Symbol 2".
#[test]
fn q23_instance_named_by_inspector_refresh() {
    let mut s = doc_session(400.0, 300.0);
    let a = rect(&mut s, 50.0, 50.0, 150.0, 120.0);
    ex(&mut s, "element.setName", json!({"ids": [a.0], "name": "R1"}));
    select(&mut s, &[a]);
    assert_eq!(unique_symbol_name(&s.active().expect("doc").doc, "Symbol"), "Symbol");
    let v = ex(&mut s, "modify.symbol.convertToSymbol", json!({}));
    let id = ElementId(v["id"].as_u64().expect("id"));
    assert_eq!(el(&s, id).common.name, None, "convert names nothing");
    let steps = history_len(&s);
    let dirty = s.active().expect("doc").is_dirty();
    assert!(name_sole_unnamed_instance(s.active_mut().expect("doc")));
    assert_eq!(el(&s, id).common.name.as_deref(), Some("Symbol"));
    assert_eq!(history_len(&s), steps, "no history step");
    assert_eq!(s.active().expect("doc").is_dirty(), dirty);
    assert!(!name_sole_unnamed_instance(s.active_mut().expect("doc")), "named once");
    let r = bounds(&s, id);
    assert_eq!([r.left, r.top, r.right, r.bottom], [50.0, 50.0, 150.0, 120.0]);

    // second and third symbol: unique default names
    let b = rect(&mut s, 200.0, 50.0, 250.0, 120.0);
    select(&mut s, &[b]);
    assert_eq!(unique_symbol_name(&s.active().expect("doc").doc, "Symbol"), "Symbol 1");
    let v = ex(&mut s, "modify.symbol.convertToSymbol", json!({}));
    let id2 = ElementId(v["id"].as_u64().expect("id"));
    let c = rect(&mut s, 300.0, 50.0, 350.0, 120.0);
    select(&mut s, &[c]);
    ex(&mut s, "modify.symbol.convertToSymbol", json!({}));
    let names: Vec<String> = s.active().expect("doc").doc.symbols.iter().map(|y| y.name.clone()).collect();
    assert_eq!(names, ["Symbol", "Symbol 1", "Symbol 2"]);

    // two selected instances: nothing named
    select(&mut s, &[id, id2]);
    assert!(!name_sole_unnamed_instance(s.active_mut().expect("doc")));
    assert_eq!(el(&s, id2).common.name, None);
    // an explicit name stays as typed (even when taken)
    let d = rect(&mut s, 50.0, 200.0, 90.0, 240.0);
    select(&mut s, &[d]);
    ex(&mut s, "modify.symbol.convertToSymbol", json!({"name": "Symbol"}));
    assert_eq!(s.active().expect("doc").doc.symbols.last().map(|y| y.name.clone()).as_deref(), Some("Symbol"));
}

fn p1(s: &mut fp_engine::Session) -> ElementId {
    let n = |a: [f64; 2], i: [f64; 2], o: [f64; 2]| json!({"anchor": a, "in": i, "out": o});
    let v = ex(
        s,
        "draw.path",
        json!({"contours": [{"closed": false, "nodes": [
            n([200.0, 200.0], [200.0, 200.0], [240.0, 160.0]),
            n([290.0, 200.0], [260.0, 160.0], [320.0, 240.0]),
            n([370.0, 200.0], [340.0, 240.0], [380.0, 180.0]),
            n([420.0, 240.0], [400.0, 200.0], [420.0, 240.0]),
        ]}]}),
    );
    ElementId(v["id"].as_u64().expect("id"))
}

/// TW-56a: P1 width 10, bevel, butt → pixelRect 196,167–425,243 (before: top 164).
#[test]
fn tw56a_expand_stroke_p1_top_167() {
    let mut s = doc_session(600.0, 400.0);
    let id = p1(&mut s);
    ex(&mut s, "element.setStroke", json!({"ids": [id.0], "color": "#000000", "width": 1}));
    select(&mut s, &[id]);
    ex(&mut s, "modify.alterPath.expandStroke", json!({"width": 10, "join": "bevel", "cap": "butt"}));
    let rid = s.active().expect("doc").selection.elements[0];
    let d = s.active().expect("doc");
    let r = fp_engine::docops::element_pixel_rect(&d.doc, &el(&s, rid)).expect("pixelRect");
    assert_eq!([r.left, r.top, r.right, r.bottom], [196.0, 167.0, 425.0, 243.0]);
}

/// Simplify uses the same refit: with amount 1 every original node of P1 stays (the last curve
/// does not fit its sample tangents within 1 px and is split at a sample).
#[test]
fn simplify_uses_refit() {
    let mut s = doc_session(600.0, 400.0);
    let id = p1(&mut s);
    select(&mut s, &[id]);
    ex(&mut s, "modify.alterPath.simplify", json!({"amount": 1}));
    let e = el(&s, id);
    let fp_model::ElementKind::Path(p) = &e.kind else { panic!("path") };
    let anchors: Vec<(f32, f32)> = p.contours[0].nodes.iter().map(|n| (n.anchor.x, n.anchor.y)).collect();
    for want in [(200.0, 200.0), (290.0, 200.0), (370.0, 200.0), (420.0, 240.0)] {
        assert!(anchors.contains(&want), "{anchors:?}");
    }
    assert_eq!(anchors.len(), 5, "{anchors:?}");
}
