//! FB-23 / TW-203: the "Dither" fill edge (Firepixel extension) in scripts — `pi.fill {edge: 3}`,
//! `pathAttributes.fill.edgeType == "dither"` with the width in `feather`, `setFillEdgeMode` and
//! `setFill` with `edgeType: "dither"`.

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

fn session_with_rect() -> fp_engine::Session {
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
    s
}

const EDGE: &str = "var f = fw.selection[0].pathAttributes.fill; f.edgeType + '|' + f.feather";

fn model_edge(s: &fp_engine::Session) -> (i32, i32) {
    let d = s.active().expect("doc");
    let id = d.selection.elements[0];
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, id).expect("element");
    let f = fp_engine::docops::path_attrs_of(&e)
        .and_then(|a| a.fill.clone())
        .expect("fill");
    (f.edge, f.feather)
}

#[test]
fn pi_fill_sets_dither_and_scripts_read_it() {
    let mut s = session_with_rect();
    s.execute("pi.fill", json!({"edge": 3, "feather": 6}))
        .expect("dither");
    assert_eq!(model_edge(&s), (fp_model::Fill::EDGE_DITHER, 6));
    assert_eq!(run(&mut s, EDGE), "dither|6");
    // the width field keeps the dither kind
    s.execute("pi.fill", json!({"feather": 9})).expect("width");
    assert_eq!(run(&mut s, EDGE), "dither|9");
    // feather ↔ dither keep the width, hard drops it
    s.execute("pi.fill", json!({"edge": 2})).expect("feather");
    assert_eq!(model_edge(&s), (fp_model::Fill::EDGE_FEATHER, 9));
    s.execute("pi.fill", json!({"edge": 3}))
        .expect("dither again");
    assert_eq!(run(&mut s, EDGE), "dither|9");
    s.execute("pi.fill", json!({"edge": 0})).expect("hard");
    assert_eq!(run(&mut s, EDGE), "hard|0");
}

#[test]
fn set_fill_edge_mode_accepts_dither() {
    let mut s = session_with_rect();
    run(
        &mut s,
        "fw.getDocumentDOM().setFillEdgeMode('dither', 7); 0",
    );
    assert_eq!(run(&mut s, EDGE), "dither|7");
    run(
        &mut s,
        "fw.getDocumentDOM().setFillEdgeMode('feather', 4); 0",
    );
    assert_eq!(model_edge(&s), (fp_model::Fill::EDGE_FEATHER, 4));
    run(&mut s, "fw.getDocumentDOM().setFillEdgeMode('hard', 0); 0");
    assert_eq!(run(&mut s, EDGE), "hard|0");
    // unknown modes change nothing
    run(
        &mut s,
        "fw.getDocumentDOM().setFillEdgeMode('splatter', 3); 0",
    );
    assert_eq!(run(&mut s, EDGE), "hard|0");
}

#[test]
fn set_fill_with_dither_edge_type() {
    let mut s = session_with_rect();
    run(
        &mut s,
        "var f = fw.selection[0].pathAttributes.fill; f.edgeType = 'dither'; f.feather = 5; \
         fw.getDocumentDOM().setFill(f); 0",
    );
    assert_eq!(run(&mut s, EDGE), "dither|5");
}

/// TW-203 rest: the dither edge also reaches text objects, which have no fill Edge combo in the
/// inspector, through `setFill {edgeType: "dither"}` and `setFillEdgeMode`.
#[test]
fn text_takes_the_dither_edge_from_scripts() {
    let mut s = fp_engine::Session::default();
    s.execute(
        "file.new",
        json!({"width": 300, "height": 200, "background": "#ffffff"}),
    )
    .expect("document");
    s.execute(
        "draw.text",
        json!({"text": "Ag", "anchor": [40, 120], "size": 60, "color": "#336699"}),
    )
    .expect("text");
    {
        let d = s.active().expect("doc");
        let id = d.selection.elements[0];
        let e = fp_engine::docops::element_on_page(&d.doc, d.page, id).expect("element");
        assert!(matches!(e.kind, fp_model::ElementKind::Text(_)));
    }
    run(
        &mut s,
        "var f = fw.selection[0].pathAttributes.fill; f.edgeType = 'dither'; f.feather = 4; \
         fw.getDocumentDOM().setFill(f); 0",
    );
    assert_eq!(model_edge(&s), (fp_model::Fill::EDGE_DITHER, 4));
    assert_eq!(run(&mut s, EDGE), "dither|4");
    run(
        &mut s,
        "fw.getDocumentDOM().setFillEdgeMode('feather', 3); 0",
    );
    assert_eq!(model_edge(&s), (fp_model::Fill::EDGE_FEATHER, 3));
    run(
        &mut s,
        "fw.getDocumentDOM().setFillEdgeMode('dither', 5); 0",
    );
    assert_eq!(run(&mut s, EDGE), "dither|5");
}
