//! F138 (BA-101 PSD part): the PSD-style Stroke live effect. `effect.stroke` adds the PSD live
//! effect container `{8eeadf50-…}` with one `stroke` record (format of the reference, 05 §9.2),
//! edits an existing one by index without touching its other records, is one History row named
//! "Stroke" and survives a FW-PNG round trip as a plain `EPSv` of the reference format.

use crate::common;
use common::*;
use fp_effects::{StrokeParams, StrokePosition};
use fp_engine::Session;
use fp_model::{EffectKind, ElementId, ElementKind, LayerSlot};
use serde_json::json;

fn effects(s: &Session, id: ElementId) -> Vec<fp_model::Effect> {
    el(s, id)
        .common
        .effects
        .as_ref()
        .map(|l| l.effects.clone())
        .unwrap_or_default()
}

fn last_label(s: &Session) -> String {
    let d = s.active().expect("doc");
    let n = d.history.undo_count();
    s.step_label(d.history.steps()[n - 1])
}

#[test]
fn stroke_command_adds_and_edits_the_record() {
    fp_i18n::set_language("en");
    let mut s = doc_session(200.0, 150.0);
    let id = rect(&mut s, 20.0, 20.0, 120.0, 100.0);
    let rows = s.active().expect("doc").history.undo_count();
    ex(
        &mut s,
        "effect.stroke",
        json!({"size": 5, "position": "center", "color": "#0080ff", "opacity": 60, "blendMode": "multiply"}),
    );
    assert_eq!(s.active().expect("doc").history.undo_count(), rows + 1);
    assert_eq!(last_label(&s), "Stroke");
    let fx = effects(&s, id);
    assert_eq!(fx.len(), 1);
    assert_eq!(fx[0].kind(), EffectKind::PhotoshopLive);
    assert!(fx[0].is_psd_stroke_only());
    assert_eq!(fx[0].label_key(), Some("effect-stroke"));
    assert_eq!(
        fx[0].get("stroke"),
        Some(
            "1&60.000000&5.000000&multiply&RGBColor;0.000000;128.000000;255.000000&centeredFrame&solidColor&"
        )
    );
    // edit by index: only the given fields change
    ex(
        &mut s,
        "effect.stroke",
        json!({"index": 0, "position": "inside", "size": 250}),
    );
    let sp = StrokeParams::from_record(effects(&s, id)[0].get("stroke").expect("stroke"));
    assert_eq!(sp.position, StrokePosition::Inside);
    assert_eq!(sp.size, 250.0);
    assert!(
        s.execute("effect.stroke", json!({"index": 0, "size": 900}))
            .is_err(),
        "outside the PSD range"
    );
    assert_eq!((sp.opacity, sp.color), (60.0, [0, 128, 255]));
    // editing keeps the other records of an imported container
    ex(
        &mut s,
        "pi.effect.set",
        json!({"index": 0, "params": {"dropShadow": "0&75.000000&1&120.000000&0.000000&5.000000&5.000000&0.000000&0&1&multiply&RGBColor;0.000000;0.000000;0.000000&2$$0.000000;0.000000;1$255.000000;255.000000;1"}}),
    );
    ex(&mut s, "effect.stroke", json!({"index": 0, "enabled": false}));
    let e = &effects(&s, id)[0];
    assert!(e.get("dropShadow").is_some_and(|r| r.starts_with("0&75")));
    assert!(e.get("stroke").is_some_and(|r| r.starts_with("0&")));
    assert_eq!(e.label_key(), Some("effect-photoshop-live"));
    // an index that is not a PSD live effect is refused; wrong values too
    ex(&mut s, "element.addEffect", json!({"effect": "dropShadow"}));
    assert!(s.execute("effect.stroke", json!({"index": 1})).is_err());
    assert!(
        s.execute("effect.stroke", json!({"position": "around"}))
            .is_err()
    );
    // undo removes the step again
    let n = effects(&s, id).len();
    ex(&mut s, "effect.stroke", json!({}));
    assert_eq!(effects(&s, id).len(), n + 1);
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(effects(&s, id).len(), n);
}

#[test]
fn add_effect_stroke_uses_the_defaults() {
    let mut s = doc_session(200.0, 150.0);
    let id = rect(&mut s, 20.0, 20.0, 120.0, 100.0);
    ex(&mut s, "element.addEffect", json!({"effect": "stroke"}));
    let fx = effects(&s, id);
    assert_eq!(fx[0].kind(), EffectKind::PhotoshopLive);
    assert_eq!(
        fx[0].get("stroke"),
        Some(StrokeParams::default().to_record().as_str())
    );
}

#[test]
fn stroke_round_trips_through_fw_png() {
    let mut s = doc_session(200.0, 150.0);
    let id = rect(&mut s, 20.0, 20.0, 120.0, 100.0);
    ex(
        &mut s,
        "effect.stroke",
        json!({"size": 7, "position": "outside", "color": "#00ff00", "opacity": 80}),
    );
    let before = effects(&s, id);
    let doc = s.active().expect("doc").doc.clone();
    let bytes = fp_fwpng::write(
        &doc,
        &fp_fwpng::WriteOptions::default(),
        &fp_fwpng::NoFlattener,
    )
    .expect("write");
    // stored as a plain effect of the reference format, not as a Firepixel marker
    let raw = fp_fwpng::read_raw(&bytes, false).expect("raw");
    fn has(nodes: &[fp_model::RawNode], key: &[u8; 4]) -> bool {
        nodes
            .iter()
            .any(|n| &n.key.0 == key || has(n.children(), key))
    }
    let trees: Vec<&Vec<fp_model::RawNode>> = [&raw.ts, &raw.bs].into_iter().flatten().collect();
    assert!(trees.iter().any(|t| has(t, b"EPSv")));
    assert!(!trees.iter().any(|t| has(t, b"fpLv")));
    let back = fp_fwpng::read(&bytes).expect("read");
    let mut found = None;
    for p in &back.pages {
        for slot in &p.root.layers {
            if let LayerSlot::Own(l) = slot {
                for c in &l.cels {
                    for e in &c.elements {
                        if matches!(e.kind, ElementKind::Path(_)) {
                            found = e.common.effects.clone();
                        }
                    }
                }
            }
        }
    }
    let fx = found.expect("effects").effects;
    assert_eq!(fx.len(), 1);
    assert_eq!(fx[0].get("stroke"), before[0].get("stroke"));
    assert_eq!(fx[0].moa_id(), before[0].moa_id());
    let sp = StrokeParams::from_record(fx[0].get("stroke").expect("stroke"));
    assert_eq!(
        (sp.size, sp.position, sp.color, sp.opacity),
        (7.0, StrokePosition::Outside, [0, 255, 0], 80.0)
    );
}
