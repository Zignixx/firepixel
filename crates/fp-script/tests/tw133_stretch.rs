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

/// Runs `src` and returns the session for inspection.
fn session_after(src: &str) -> fp_engine::Session {
    let mut s = fp_engine::Session::default();
    let o = run_source(&mut s, src, &ScriptOptions::default());
    assert!(
        o.ok,
        "error: {:?}\ndiagnostics: {:?}",
        o.error, o.diagnostics
    );
    s
}

#[test]
fn stretch_follows_the_align_command_path() {
    fp_i18n::set_language("en");
    let s = session_after(&format!(
        "{NEW} dom.addNewText({FRAME}, true);\n\
         dom.setTextRuns({{initialAttrs: {{size: '24pt'}}, textRuns: [{{changedAttrs: {{}}, characters: 'Abc 123'}}]}});\n\
         dom.setTextAlignment('stretch');"
    ));
    assert_eq!(s.prefs.get_int("Text_Justification", 0), 4);
    let d = s.active().expect("doc");
    let labels: Vec<String> = d
        .history
        .steps()
        .iter()
        .map(|st| s.step_label(st))
        .collect();
    assert_eq!(
        labels
            .iter()
            .filter(|l| l.as_str() == "Text Change")
            .count(),
        1,
        "{labels:?}"
    );
    assert_eq!(labels.last().map(String::as_str), Some("Text Change"));
    assert!(d.is_dirty());
    let s2 = result(&format!(
        "{NEW} dom.addNewText({FRAME}, true);\n\
         dom.setTextRuns({{initialAttrs: {{size: '24pt'}}, textRuns: [{{changedAttrs: {{}}, characters: 'Abc 123'}}]}});\n\
         dom.setTextAlignment('stretch'); fw.selection[0].textRuns.initialAttrs.alignment"
    ));
    assert_eq!(s2, "left");
}

#[test]
fn set_alignment_without_selection_only_writes_the_default() {
    let s = session_after(&format!(
        "{NEW} dom.selectNone(); dom.setTextAlignment('right');"
    ));
    assert_eq!(s.prefs.get_int("Text_Justification", 0), 2);
}

#[test]
fn add_new_text_false_ignores_the_text_defaults() {
    let r = result(&format!(
        "{NEW} dom.selectNone(); dom.setTextAlignment('right');\n\
         dom.addNewText({FRAME}, false); var a = fw.selection[0].textRuns.initialAttrs.alignment;\n\
         dom.addNewText({FRAME}, true); a + '|' + fw.selection[0].textRuns.initialAttrs.alignment"
    ));
    assert_eq!(r, "left|right");
}
