//! FB-151 (TW-298): the mask mode "Pixel Alpha" (Firepixel extension) in scripts —
//! `dom.setElementMaskPixelAlpha(bool)` (a Firepixel-own method beside the reference API),
//! `setElementMaskMode("pixel alpha")` and the `pixelAlpha` field of the element's `mask` object
//! (read and written back; `mode` reads "mask to path", the fallback of other readers).

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

/// Rectangle masked by an ellipse (Group as Mask: grayscale); the masked element is selected.
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
    s.execute("select.set", json!({"ids": [b, a]}))
        .expect("select");
    s.execute("modify.mask.groupAsMask", json!({}))
        .expect("mask");
    s
}

fn model(s: &fp_engine::Session) -> (bool, bool) {
    let d = s.active().expect("doc");
    let id = d.selection.elements[0];
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, id).expect("element");
    let m = e.element_mask().expect("mask");
    (m.to_path, m.pixel_alpha)
}

#[test]
fn set_element_mask_pixel_alpha_mode_and_the_mask_object() {
    let mut s = session_with_mask();
    assert_eq!(model(&s), (false, false), "Group as Mask: grayscale");
    assert_eq!(
        run(&mut s, "String(fw.selection[0].mask.pixelAlpha)"),
        "false"
    );
    run(
        &mut s,
        "fw.getDocumentDOM().setElementMaskPixelAlpha(true); 0",
    );
    assert_eq!(
        model(&s),
        (true, true),
        "switches to path outline + Pixel Alpha"
    );
    assert_eq!(
        run(&mut s, "String(fw.selection[0].mask.pixelAlpha)"),
        "true"
    );
    assert_eq!(run(&mut s, "fw.selection[0].mask.mode"), "mask to path");
    // a read-modify-write keeps it
    run(
        &mut s,
        "var e = fw.selection[0]; var m = e.mask; e.mask = m; 0",
    );
    assert_eq!(model(&s), (true, true));
    // grayscale wins over a stale pixelAlpha in the same object
    run(
        &mut s,
        "var e = fw.selection[0]; var m = e.mask; m.mode = 'mask to image'; e.mask = m; 0",
    );
    assert_eq!(model(&s), (false, false));
    run(
        &mut s,
        "fw.getDocumentDOM().setElementMaskMode('pixel alpha'); 0",
    );
    assert_eq!(model(&s), (true, true));
    run(
        &mut s,
        "fw.getDocumentDOM().setElementMaskMode('mask to path'); 0",
    );
    assert_eq!(model(&s), (true, false));
    run(
        &mut s,
        "var e = fw.selection[0]; e.mask = {pixelAlpha: true}; 0",
    );
    assert_eq!(model(&s), (true, true));
    s.undo().expect("undo");
    assert_eq!(model(&s), (true, false), "undo");
}
