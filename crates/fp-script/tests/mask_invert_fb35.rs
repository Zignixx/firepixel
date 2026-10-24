//! FB-35 / RE spec 68 section 6: the inverted mask (Firepixel extension) in scripts —
//! `dom.setElementMaskInvert(bool)` (a Firepixel-own method beside the reference API) and the
//! `invert` field of the element's `mask` object (read and written back).

use fp_script::{ScriptOptions, run_source};
use serde_json::json;

fn run(s: &mut fp_engine::Session, src: &str) -> String {
    let o = run_source(s, src, &ScriptOptions::default());
    assert!(o.ok, "error: {:?}\ndiagnostics: {:?}", o.error, o.diagnostics);
    o.result.unwrap_or_default()
}

/// Rectangle masked by an ellipse; the masked element is selected.
fn session_with_mask() -> fp_engine::Session {
    let mut s = fp_engine::Session::default();
    s.execute(
        "file.new",
        json!({"width": 300, "height": 200, "background": "#ffffff"}),
    )
    .expect("document");
    let a = s
        .execute("draw.rectangle", json!({"rect": [40, 40, 200, 150]}))
        .expect("rectangle")
        .value["id"]
        .clone();
    let b = s
        .execute("draw.ellipse", json!({"rect": [80, 60, 160, 130]}))
        .expect("ellipse")
        .value["id"]
        .clone();
    s.execute("select.set", json!({"ids": [b, a]})).expect("select");
    s.execute("modify.mask.groupAsMask", json!({})).expect("mask");
    s
}

fn model_invert(s: &fp_engine::Session) -> bool {
    let d = s.active().expect("doc");
    let id = d.selection.elements[0];
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, id).expect("element");
    e.element_mask().is_some_and(|m| m.invert)
}

#[test]
fn set_element_mask_invert_and_the_mask_object() {
    let mut s = session_with_mask();
    assert_eq!(run(&mut s, "String(fw.selection[0].mask.invert)"), "false");
    run(&mut s, "fw.getDocumentDOM().setElementMaskInvert(true); 0");
    assert!(model_invert(&s));
    assert_eq!(run(&mut s, "String(fw.selection[0].mask.invert)"), "true");
    // writing the mask object back keeps / changes the field
    run(
        &mut s,
        "var e = fw.selection[0]; var m = e.mask; m.invert = false; e.mask = m; 0",
    );
    assert!(!model_invert(&s));
    run(&mut s, "var e = fw.selection[0]; e.mask = {invert: true}; 0");
    assert!(model_invert(&s));
    // undo restores the previous state
    s.undo().expect("undo");
    assert!(!model_invert(&s));
}

#[test]
fn the_extension_method_is_not_part_of_the_reference_binding_table() {
    let doc = fp_script::bindings::CLASSES
        .iter()
        .find(|c| c.js == "Document")
        .expect("Document");
    assert!(!doc.methods.iter().any(|m| m.name == "setElementMaskInvert"));
    assert!(
        fp_script::bindings::EXT_METHODS
            .iter()
            .any(|(c, m)| *c == "Document" && m.name == "setElementMaskInvert")
    );
}
