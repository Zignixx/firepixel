//! F10: TW-40 (Rest Q-15) – Text in einem festen Rahmen, der schmaler als ein Zeichen ist (TX1, 10 px):
//! Das Original setzt nichts. `pixelRect` ist leer (±2³¹) auch bei 18 pt (`menu-text/16`), und
//! *In Pfade konvertieren* lässt den Text stehen (`menu-modify-b/21`).

use fp_script::{ScriptOptions, run_source};

fn result(src: &str) -> String {
    let mut s = fp_engine::Session::default();
    let o = run_source(&mut s, src, &ScriptOptions::default());
    assert!(o.ok, "Fehler: {:?}\nDiagnose: {:?}", o.error, o.diagnostics);
    o.result.unwrap_or_default()
}

const NEW: &str = "var dom = fw.createFirepixelDocument({x: 800, y: 600}, {pixelsPerUnit: 72, units: 'inch'}, '#ffffff'); dom = fw.getDocumentDOM();\n";

fn text(right: u32, size: u32) -> String {
    format!(
        "{NEW} dom.addNewText({{left: 50, top: 50, right: {right}, bottom: 1050}}, true);\n\
         dom.setTextRuns({{initialAttrs: {{face: 'MyriadPro-Regular', size: '{size}pt'}}, textRuns: [{{changedAttrs: {{}}, characters: 'Abc 123'}}]}});\n"
    )
}

const EMPTY: &str = "-2147483648,-2147483648,2147483648,2147483648";

#[test]
fn too_narrow_frame_has_no_ink() {
    for size in [18, 24] {
        let src = format!(
            "{} var p = fw.selection[0].pixelRect; [p.left, p.top, p.right, p.bottom].join(',')",
            text(60, size)
        );
        assert_eq!(result(&src), EMPTY, "{size} pt");
    }
    // Gegenprobe: breiter Rahmen hat Tinte
    let src = format!(
        "{} var p = fw.selection[0].pixelRect; [p.left, p.top, p.right, p.bottom].join(',')",
        text(300, 18)
    );
    assert_ne!(result(&src), EMPTY);
}

#[test]
fn convert_to_paths_leaves_too_narrow_text() {
    let src = format!(
        "{} dom.convertToPaths(); var e = dom.layers[dom.currentLayerNum].frames[0].elements;\n\
         e.length + '|' + e[0].toString() + '|' + fw.selection.length",
        text(60, 18)
    );
    assert_eq!(result(&src), "1|[object Text]|1");
    // Gegenprobe: im breiten Rahmen entsteht eine Gruppe
    let src = format!(
        "{} dom.convertToPaths(); dom.layers[dom.currentLayerNum].frames[0].elements[0].toString()",
        text(300, 18)
    );
    assert_eq!(result(&src), "[object Group]");
}
