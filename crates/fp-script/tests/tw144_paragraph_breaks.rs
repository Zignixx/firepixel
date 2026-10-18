//! TW-144 (re 50 section 3): `textChars` and `textRuns[].characters` separate paragraphs with
//! `\r` (no trailing `\r`), a forced line break is U+0003; text set by scripts with CRLF or LF is
//! stored with CR.

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

const NEW: &str = r"var dom = fw.createFirepixelDocument({x: 300, y: 200}, {pixelsPerUnit: 72, units: 'inch'}, '#ffffff'); dom = fw.getDocumentDOM();
dom.addNewText({left: 10, top: 10, right: 200, bottom: 60}, true);
function show(s) { return s.replace(/\r/g, '<CR>').replace(/\n/g, '<LF>').replace(/\u0003/g, '<ETX>'); }
";

/// `chars` is JS source text (escapes like `\r` stay escapes for the script).
fn set_runs(chars: &str) -> String {
    format!(
        "{NEW} dom.setTextRuns({{initialAttrs: {{size: '12pt'}}, textRuns: [{{changedAttrs: {{}}, characters: '{chars}'}}]}});\n"
    )
}

#[test]
fn text_chars_and_runs_use_cr_without_trailing_cr() {
    let src = set_runs(r"Zeile eins\rZeile zwei\rZeile drei")
        + "var e = fw.selection[0]; show(e.textChars) + '|' + show(e.textRuns.textRuns[0].characters)";
    assert_eq!(
        result(&src),
        "Zeile eins<CR>Zeile zwei<CR>Zeile drei|Zeile eins<CR>Zeile zwei<CR>Zeile drei"
    );
}

#[test]
fn forced_line_break_is_u0003() {
    let src = set_runs(r"eins\u0003zwei\rdrei") + "show(fw.selection[0].textChars)";
    assert_eq!(result(&src), "eins<ETX>zwei<CR>drei");
}

#[test]
fn crlf_and_lf_input_become_cr() {
    let src = set_runs(r"a\nb\r\nc")
        + r"var e = fw.selection[0]; var r1 = show(e.textChars);
e.textChars = 'x\r\ny\nz'; r1 + '|' + show(e.textChars) + '|' + show(e.textRuns.textRuns[0].characters)";
    assert_eq!(result(&src), "a<CR>b<CR>c|x<CR>y<CR>z|x<CR>y<CR>z");
}
