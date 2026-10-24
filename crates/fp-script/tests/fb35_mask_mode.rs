//! FB-35 (RE spec 68 §2.1, §2.2, M-8, M-12): `setElementMaskMode("mask to path" / "mask to image")`
//! switches the mode of the mask group (the old test for "alpha" never matched the original's
//! values), `mask.mode` reads it back, and `dom.pathAttributes` reports grey while a mask is
//! selected (red → `#363636`).

use fp_script::{ScriptOptions, run_source};
use serde_json::json;

fn run(s: &mut fp_engine::Session, src: &str) -> String {
    let o = run_source(s, src, &ScriptOptions::default());
    assert!(
        o.ok,
        "error: {:?}\ndiagnostics: {:?}",
        o.error, o.diagnostics
    );
    o.result.unwrap_or_default()
}

fn id(v: &serde_json::Value) -> u64 {
    v["id"].as_u64().unwrap_or(0)
}

/// White square with a red square above, grouped as mask (grayscale); the group is selected.
fn masked() -> fp_engine::Session {
    let mut s = fp_engine::Session::default();
    s.execute(
        "file.new",
        json!({"width": 300, "height": 200, "background": "#ffffff"}),
    )
    .expect("document");
    let c = s
        .execute(
            "draw.rectangle",
            json!({"rect": [0, 0, 100, 100], "fillColor": "#ffffff", "strokeWidth": 0}),
        )
        .expect("content")
        .value;
    let m = s
        .execute(
            "draw.rectangle",
            json!({"rect": [10, 10, 90, 90], "fillColor": "#ff0000", "strokeWidth": 0}),
        )
        .expect("mask")
        .value;
    s.execute("select.set", json!({"ids": [id(&c), id(&m)]}))
        .expect("select");
    s.execute("modify.mask.groupAsMask", json!({}))
        .expect("group as mask");
    s
}

fn kind(s: &fp_engine::Session) -> fp_model::GroupKind {
    let d = s.active().expect("doc");
    let e =
        fp_engine::docops::element_on_page(&d.doc, d.page, d.selection.elements[0]).expect("group");
    match &e.kind {
        fp_model::ElementKind::Group(g) => g.kind,
        _ => panic!("group expected"),
    }
}

#[test]
fn set_element_mask_mode_round_trip() {
    let mut s = masked();
    assert_eq!(kind(&s), fp_model::GroupKind::BitmapMask);
    assert_eq!(run(&mut s, "fw.selection[0].mask.mode"), "mask to image");
    run(
        &mut s,
        "fw.getDocumentDOM().setElementMaskMode('mask to path')",
    );
    assert_eq!(kind(&s), fp_model::GroupKind::VectorMask);
    assert_eq!(run(&mut s, "fw.selection[0].mask.mode"), "mask to path");
    run(
        &mut s,
        "fw.getDocumentDOM().setElementMaskMode('mask to image')",
    );
    assert_eq!(kind(&s), fp_model::GroupKind::BitmapMask);
    assert_eq!(run(&mut s, "fw.selection[0].mask.mode"), "mask to image");
    // an unknown value changes nothing
    run(&mut s, "fw.getDocumentDOM().setElementMaskMode('nonsense')");
    assert_eq!(kind(&s), fp_model::GroupKind::BitmapMask);
}

#[test]
fn path_attributes_are_grey_while_the_mask_is_selected() {
    let mut s = masked();
    s.execute("panel.layers.editMask", json!({}))
        .expect("edit mask");
    assert!(s.mask_selected());
    assert_eq!(
        run(&mut s, "fw.getDocumentDOM().pathAttributes.fillColor"),
        "#363636"
    );
}
