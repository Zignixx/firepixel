//! FB-44: "Lock pattern to document" (Firepixel extension) in scripts — `setFill` with
//! `lockToDocument`, `pathAttributes.fill.lockToDocument` (reported only when on), a pattern fill
//! object passed to `setFill` keeps the pattern, and a locked pattern stays in the document when
//! `moveSelectionBy` moves the object.

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

fn session_with_pattern_rect() -> fp_engine::Session {
    let mut s = fp_engine::Session::default();
    s.execute(
        "file.new",
        json!({"width": 300, "height": 200, "background": "#ffffff"}),
    )
    .expect("document");
    s.execute(
        "draw.rectangle",
        json!({"rect": [40, 40, 200, 150], "fillColor": "#336699", "strokeWidth": 0}),
    )
    .expect("rectangle");
    s.execute("pi.fill", json!({"category": "pattern"}))
        .expect("pattern");
    s
}

fn model_fill(s: &fp_engine::Session) -> (fp_model::Fill, fp_model::FillHandles) {
    let d = s.active().expect("doc");
    let id = d.selection.elements[0];
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, id).expect("element");
    let f = fp_engine::docops::path_attrs_of(&e)
        .and_then(|a| a.fill.clone())
        .expect("fill");
    let h = fp_engine::commands::panel::handles_of(&e).expect("handles");
    (f, h)
}

const LOCK: &str = "String(fw.selection[0].pathAttributes.fill.lockToDocument)";

#[test]
fn set_fill_round_trips_lock_to_document() {
    let mut s = session_with_pattern_rect();
    let (f0, _) = model_fill(&s);
    assert_eq!(f0.shape, 15, "pattern fill");
    assert!(!f0.lock_to_document);
    assert_eq!(
        run(&mut s, LOCK),
        "undefined",
        "not reported at the default"
    );
    // read-modify-write of the fill object: the pattern stays, the flag is set
    run(
        &mut s,
        "var f = fw.selection[0].pathAttributes.fill; f.lockToDocument = true; \
         fw.getDocumentDOM().setFill(f); 0",
    );
    let (f1, _) = model_fill(&s);
    assert_eq!(
        f1.shape, 15,
        "setFill with a pattern object keeps the pattern"
    );
    assert_eq!(
        f1.pattern.as_ref().map(|p| &p.name),
        f0.pattern.as_ref().map(|p| &p.name)
    );
    assert!(f1.lock_to_document);
    assert_eq!(run(&mut s, LOCK), "true");
    // and off again
    run(
        &mut s,
        "var f = fw.selection[0].pathAttributes.fill; f.lockToDocument = false; \
         fw.getDocumentDOM().setFill(f); 0",
    );
    assert!(!model_fill(&s).0.lock_to_document);
    assert_eq!(run(&mut s, LOCK), "undefined");
}

#[test]
fn move_selection_by_leaves_locked_handles() {
    let mut s = session_with_pattern_rect();
    let (_, h0) = model_fill(&s);
    run(
        &mut s,
        "var f = fw.selection[0].pathAttributes.fill; f.lockToDocument = true; \
         var d = fw.getDocumentDOM(); d.setFill(f); d.moveSelectionBy({x: 37, y: 23}, false, false); 0",
    );
    let (_, h1) = model_fill(&s);
    assert_eq!(h1, h0, "locked: handles stay in the document");
    assert_eq!(
        run(&mut s, "String(fw.selection[0].left)"),
        "77",
        "the object moved"
    );
    // moveSelectionTo, unlocked: the handles move along
    run(
        &mut s,
        "var f = fw.selection[0].pathAttributes.fill; f.lockToDocument = false; \
         var d = fw.getDocumentDOM(); d.setFill(f); d.moveSelectionTo({x: 87, y: 63}, false, false); 0",
    );
    let (_, h2) = model_fill(&s);
    assert_eq!((h2.start.x, h2.start.y), (h0.start.x + 10.0, h0.start.y));
}
