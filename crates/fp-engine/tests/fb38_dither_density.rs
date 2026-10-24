//! FB-38 (Firepixel extension): `pi.fill {ditherDensity}` sets the uniform dither density of the
//! selected fills as one fill history row, survives a change of the fill category and is undone
//! like every fill change; the Screentone live filter is added by name with its defaults.

use crate::common;
use common::*;
use fp_engine::Session;
use fp_model::ElementId;
use serde_json::json;

fn density(s: &Session, id: ElementId) -> i32 {
    fp_engine::docops::path_attrs_of(&el(s, id))
        .and_then(|a| a.fill.clone())
        .map(|f| f.dither_density)
        .unwrap_or(-1)
}

fn last_label(s: &Session) -> String {
    let d = s.active().expect("doc");
    let n = d.history.undo_count();
    s.step_label(d.history.steps()[n - 1])
}

#[test]
fn pi_fill_dither_density_is_a_fill_change() {
    let mut s = doc_session(200.0, 150.0);
    let id = rect(&mut s, 20.0, 20.0, 120.0, 100.0);
    ex(&mut s, "pi.fill", json!({"color": "#ff0000"}));
    let fill_row = last_label(&s);
    let rows = s.active().expect("doc").history.undo_count();
    ex(&mut s, "pi.fill", json!({"ditherDensity": 30}));
    assert_eq!(density(&s, id), 30);
    assert_eq!(s.active().expect("doc").history.undo_count(), rows + 1);
    assert_eq!(last_label(&s), fill_row, "history row of a fill change");
    // the density applies to every category and survives a change of it
    ex(&mut s, "pi.fill", json!({"category": "gradient"}));
    assert_eq!(density(&s, id), 30);
    ex(&mut s, "pi.fill", json!({"category": "solid"}));
    assert_eq!(density(&s, id), 30);
    ex(&mut s, "pi.fill", json!({"edge": 3, "feather": 5}));
    assert_eq!(density(&s, id), 30);
    // values are clamped by the parameter range; 0 switches it off
    assert!(s.execute("pi.fill", json!({"ditherDensity": 101})).is_err());
    ex(&mut s, "pi.fill", json!({"ditherDensity": 0}));
    assert_eq!(density(&s, id), 0);
    // undo goes back step by step
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(density(&s, id), 30);
}

#[test]
fn screentone_effect_by_name() {
    let mut s = doc_session(200.0, 150.0);
    let id = rect(&mut s, 20.0, 20.0, 120.0, 100.0);
    ex(
        &mut s,
        "element.addEffect",
        json!({"effect": "screentone", "params": {"screentone_size": "16"}}),
    );
    let e = el(&s, id);
    let fx = &e.common.effects.as_ref().expect("effects").effects;
    assert_eq!(fx.len(), 1);
    assert_eq!(fx[0].kind(), fp_model::EffectKind::Screentone);
    assert!(fx[0].is_firepixel());
    // the given parameter overrides the menu default, the others are the defaults
    assert_eq!(fx[0].get("screentone_size"), Some("16"));
    assert_eq!(fx[0].get("screentone_type"), Some("0"));
    assert_eq!(fx[0].get("screentone_color"), Some("#000000"));
    // editable like the other live filters
    ex(
        &mut s,
        "pi.effect.set",
        json!({"index": 0, "params": {"screentone_type": "1"}}),
    );
    assert_eq!(
        el(&s, id).common.effects.as_ref().expect("effects").effects[0].get("screentone_type"),
        Some("1")
    );
}
