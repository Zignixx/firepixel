//! FB-150 / TW-294 (Firepixel extension): `pi.fill {ditherStrength}` sets the strength of the
//! gradient dither as one fill history row, is undone like every fill change and survives an
//! edit of the gradient stops; new stops also keep the dither switch itself.

use crate::common;
use common::*;
use fp_engine::Session;
use fp_model::ElementId;
use serde_json::json;

fn fill(s: &Session, id: ElementId) -> fp_model::Fill {
    fp_engine::docops::path_attrs_of(&el(s, id))
        .and_then(|a| a.fill.clone())
        .expect("fill")
}

fn dither(f: &fp_model::Fill) -> Option<bool> {
    f.gradient
        .as_ref()
        .and_then(|g| g.stops.as_ref())
        .and_then(|s| s.dither)
}

#[test]
fn pi_fill_dither_strength_is_a_fill_change() {
    let mut s = doc_session(200.0, 150.0);
    let id = rect(&mut s, 20.0, 20.0, 120.0, 100.0);
    ex(&mut s, "pi.fill", json!({"category": "gradient"}));
    ex(&mut s, "pi.fill", json!({"dither": true}));
    assert_eq!(fill(&s, id).gradient_dither_strength, 100, "default");
    let rows = s.active().expect("doc").history.undo_count();
    ex(&mut s, "pi.fill", json!({"ditherStrength": 250}));
    assert_eq!(fill(&s, id).gradient_dither_strength, 250);
    assert_eq!(s.active().expect("doc").history.undo_count(), rows + 1);
    // range 0..400
    assert!(s.execute("pi.fill", json!({"ditherStrength": 401})).is_err());
    assert!(s.execute("pi.fill", json!({"ditherStrength": -1})).is_err());
    ex(&mut s, "pi.fill", json!({"ditherStrength": 0}));
    assert_eq!(fill(&s, id).gradient_dither_strength, 0);
    // new stops keep the dither switch and the strength
    ex(
        &mut s,
        "pi.fill",
        json!({"stops": [[0.0, "#ff0000"], [0.5, "#00ff00"], [1.0, "#0000ff"]]}),
    );
    let f = fill(&s, id);
    assert_eq!(dither(&f), Some(true));
    assert_eq!(f.gradient_dither_strength, 0);
    ex(&mut s, "pi.fill", json!({"reverse": true}));
    assert_eq!(dither(&fill(&s, id)), Some(true));
    // undo goes back step by step
    ex(&mut s, "edit.undo", json!({}));
    ex(&mut s, "edit.undo", json!({}));
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(fill(&s, id).gradient_dither_strength, 250);
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(fill(&s, id).gradient_dither_strength, 100);
    assert_eq!(dither(&fill(&s, id)), Some(true));
}
