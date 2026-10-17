//! T-9 (re 40 §2, TW-86): Ebenen-Zug auf die Mitte einer Ebene schiebt sie als oberste Unterebene
//! hinein (ein Schritt „Ebene verschieben“); „Ebene statusübergreifend freigeben“ übernimmt den
//! aktuellen Status (Schritt „Ebenenoptionen“ über `step`), Aufheben kopiert in den aktuellen oder
//! in alle Status.

use fp_engine::docops;
use fp_model::{ElementKind, LayerId};
use serde_json::json;

fn ex(s: &mut fp_engine::Session, c: &str, p: serde_json::Value) {
    s.execute(c, p).unwrap_or_else(|e| panic!("{c}: {e}"));
}

fn session() -> fp_engine::Session {
    fp_i18n::set_language("de");
    let mut s = fp_engine::Session::default();
    ex(&mut s, "file.new", json!({"width": 400, "height": 300, "background": "#ffffff"}));
    s
}

fn own_layer_ids(s: &fp_engine::Session) -> Vec<LayerId> {
    let d = s.active().expect("d");
    docops::own_layers(&d.doc, d.page).into_iter().filter(|(_, l)| l.kind == fp_model::LayerKind::Normal).map(|(_, l)| l.id).collect()
}

fn undo_label(s: &fp_engine::Session) -> String {
    s.active().expect("d").history.undo_label().unwrap_or_default().to_string()
}

#[test]
fn layer_drop_into_becomes_top_sublayer_in_one_step() {
    let mut s = session();
    ex(&mut s, "draw.rectangle", json!({"rect": [10, 10, 50, 50]}));
    ex(&mut s, "layer.add", json!({}));
    ex(&mut s, "draw.rectangle", json!({"rect": [60, 10, 90, 50]}));
    ex(&mut s, "layer.add", json!({}));
    ex(&mut s, "draw.rectangle", json!({"rect": [100, 10, 150, 50]}));
    let ids = own_layer_ids(&s);
    assert_eq!(ids.len(), 3);
    let (l1, l3) = (ids[0], ids[2]);
    let n0 = s.active().expect("d").history.undo_count();
    ex(&mut s, "layer.move", json!({"layer": l1.0, "index": 0, "into": l3.0}));
    assert_eq!(s.active().expect("d").history.undo_count(), n0 + 1, "ein Schritt");
    assert_eq!(undo_label(&s), "Ebene verschieben");
    assert_eq!(own_layer_ids(&s), vec![ids[1], l3], "Ebene 1 ist keine eigene Ebene mehr");
    let d = s.active().expect("d");
    let target = docops::layer_by_id(&d.doc, d.page, l3).expect("Ebene 3");
    let top = &target.cel(d.frame).expect("cel").elements[0];
    assert!(matches!(&top.kind, ElementKind::SubLayer(l) if l.id == l1), "oberstes Kind = Ebene 1");
    assert_eq!(d.layer, Some(l1), "gezogene Ebene aktiv");
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(own_layer_ids(&s), ids);
}

#[test]
fn share_takes_current_state_and_unshare_copies_current_or_all() {
    let mut s = session();
    ex(&mut s, "draw.rectangle", json!({"rect": [10, 10, 50, 50]}));
    ex(&mut s, "state.add", json!({}));
    let frames = s.active().expect("d").frame_count();
    assert!(frames >= 2);
    let count = |s: &fp_engine::Session, f: usize| {
        let d = s.active().expect("d");
        let l = docops::layer_by_id(&d.doc, d.page, d.layer.expect("aktiv")).expect("Ebene");
        let i = if l.shared_across_frames { 0 } else { f };
        l.cels.get(i).map_or(0, |c| c.elements.len())
    };
    let f = s.active().expect("d").frame;
    ex(&mut s, "draw.rectangle", json!({"rect": [60, 60, 90, 90]}));
    let in_current = count(&s, f);
    ex(&mut s, "layer.setShared", json!({"shared": true, "step": "layer-options"}));
    assert_eq!(undo_label(&s), "Ebenenoptionen");
    for k in 0..frames {
        assert_eq!(count(&s, k), in_current, "Status {k} zeigt den Inhalt des aktuellen");
    }
    // Aufheben „Aktuell“: nur der aktuelle Status behält die Daten
    ex(&mut s, "layer.setShared", json!({"shared": false, "copyTo": "current"}));
    for k in 0..frames {
        assert_eq!(count(&s, k), if k == f { in_current } else { 0 }, "Status {k}");
    }
    ex(&mut s, "edit.undo", json!({}));
    // Aufheben „Alle“
    ex(&mut s, "layer.setShared", json!({"shared": false, "copyTo": "all"}));
    for k in 0..frames {
        assert_eq!(count(&s, k), in_current, "Status {k}");
    }
    fp_i18n::set_language("en");
    ex(&mut s, "edit.undo", json!({}));
    ex(&mut s, "edit.undo", json!({}));
    ex(&mut s, "layer.setShared", json!({"shared": true, "step": "layer-options"}));
    assert_eq!(undo_label(&s), "Layer Options");
    fp_i18n::set_language("de");
}

/// T-10 (re 41 §4): Freigeben und Aufheben (Aktuell wie Alle, auch über den Menübefehl) sind ein
/// Schritt „Ebenenoptionen“; der Zug „hinein“ heißt „Ebene verschieben“ (oben geprüft).
#[test]
fn t10_unshare_step_is_layer_options() {
    let mut s = session();
    ex(&mut s, "state.add", json!({}));
    ex(&mut s, "draw.rectangle", json!({"rect": [60, 60, 90, 90]}));
    ex(&mut s, "layer.setShared", json!({"shared": true, "step": "layer-options"}));
    for to in ["current", "all"] {
        let n0 = s.active().expect("d").history.undo_count();
        ex(&mut s, "layer.setShared", json!({"shared": false, "copyTo": to, "step": "layer-options"}));
        assert_eq!(s.active().expect("d").history.undo_count(), n0 + 1, "{to}: ein Schritt");
        assert_eq!(undo_label(&s), "Ebenenoptionen", "{to}");
        ex(&mut s, "edit.undo", json!({}));
    }
    // Menübefehl „Ebene statusübergreifend freigeben“: Aufheben und Freigeben
    ex(&mut s, "panel.layers.shareAcrossStates", json!({}));
    assert_eq!(undo_label(&s), "Ebenenoptionen");
    ex(&mut s, "panel.layers.shareAcrossStates", json!({}));
    assert_eq!(undo_label(&s), "Ebenenoptionen");
}
