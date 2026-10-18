//! TW-133 (re 50 section 1): `setTextAlignment("stretch")` reads back "left" on fixed-width and
//! point text; "justify" is stored and reads back "justify" on both.

use fp_script::{ScriptOptions, run_source};

fn result(src: &str) -> String {
    let mut s = fp_engine::Session::default();
    let o = run_source(&mut s, src, &ScriptOptions::default());
    assert!(
        o.ok,
        "error: {:?}\ndiagnostics: {:?}",
        o.error, o.diagnostics
    );
    o.result.unwrap_or_default()
}

const NEW: &str = "var dom = fw.createFirepixelDocument({x: 300, y: 200}, {pixelsPerUnit: 72, units: 'inch'}, '#ffffff'); dom = fw.getDocumentDOM();\n";

fn align_after(rect: &str, align: &str) -> String {
    result(&format!(
        "{NEW} dom.addNewText({rect}, true);\n\
         dom.setTextRuns({{initialAttrs: {{size: '24pt', alignment: 'right'}}, textRuns: [{{changedAttrs: {{}}, characters: 'Abc 123'}}]}});\n\
         dom.setTextAlignment('{align}');\n\
         var t = fw.selection[0].textRuns; t.initialAttrs.alignment + '|' + t.textRuns[0].changedAttrs.alignment"
    ))
}

const FRAME: &str = "{left: 10, top: 10, right: 200, bottom: 60}";
const POINT: &str = "{left: 10, top: 10, right: 10, bottom: 40}";

#[test]
fn stretch_reads_back_left() {
    assert_eq!(align_after(FRAME, "stretch"), "left|left");
    assert_eq!(align_after(POINT, "stretch"), "left|left");
}

#[test]
fn justify_reads_back_justify() {
    assert_eq!(align_after(FRAME, "justify"), "justify|justify");
    assert_eq!(align_after(POINT, "justify"), "justify|justify");
}
