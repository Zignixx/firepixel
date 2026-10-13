//! TW-02 (`textRuns` mit allen Eigenschaften des Originals, `face` als PostScript-Name) und TW-08
//! (`toSource` ohne umschließende Klammern).

use fp_script::{run_source, ScriptOptions};

fn result(src: &str) -> String {
    let mut s = fp_engine::Session::default();
    let o = run_source(&mut s, src, &ScriptOptions::default());
    assert!(o.ok, "Fehler: {:?}\nDiagnose: {:?}", o.error, o.diagnostics);
    o.result.unwrap_or_default()
}

const NEW: &str = "var dom = fw.createFirepixelDocument({x: 300, y: 200}, {pixelsPerUnit: 72, units: 'inch'}, '#ffffff'); dom = fw.getDocumentDOM();\n\
    dom.addNewText({left: 10, top: 10, right: 200, bottom: 60}, true);\n";

/// Die 18 Schlüssel von `initialAttrs` im Original (Logs menu-text/02, dialogs-d21-d30/06).
const INITIAL_KEYS: &str = "alignment,antiAliasSharpness,antiAliasStrength,baselineShift,bold,face,fillColor,horizontalScale,italic,leading,leadingMode,overSample,paragraphIndent,paragraphSpacingAfter,paragraphSpacingBefore,rangeKerning,size,underline";
/// Die 15 Schlüssel des ersten Laufs (ohne Glättungswerte).
const RUN_KEYS: &str = "alignment,baselineShift,bold,face,fillColor,horizontalScale,italic,leading,leadingMode,paragraphIndent,paragraphSpacingAfter,paragraphSpacingBefore,rangeKerning,size,underline";

#[test]
fn initial_attrs_have_all_original_keys() {
    let src = format!(
        "{NEW} dom.setTextRuns({{initialAttrs: {{face: 'MyriadPro-Regular', size: '24pt'}}, textRuns: [{{changedAttrs: {{}}, characters: 'Abc 123'}}]}});\n\
         var t = fw.selection[0].textRuns, k = [], c = [];\n\
         for (var n in t.initialAttrs) k.push(n); for (var n in t.textRuns[0].changedAttrs) c.push(n);\n\
         k.sort().join(',') + '|' + c.sort().join(',')"
    );
    assert_eq!(result(&src), format!("{INITIAL_KEYS}|{RUN_KEYS}"));
}

#[test]
fn default_values_like_original() {
    let src = format!(
        "{NEW} dom.setTextRuns({{initialAttrs: {{face: 'MyriadPro-Regular', size: '24pt', leading: 1.2, leadingMode: 'percentage'}}, textRuns: [{{changedAttrs: {{}}, characters: 'Abc 123'}}]}});\n\
         var a = fw.selection[0].textRuns.initialAttrs;\n\
         [a.alignment, a.antiAliasSharpness, a.antiAliasStrength, a.overSample, a.baselineShift, a.bold, a.face, a.fillColor, a.horizontalScale, a.italic,\n\
          a.leading, a.leadingMode, a.paragraphIndent, a.paragraphSpacingAfter, a.paragraphSpacingBefore, a.rangeKerning, a.size, a.underline].join('|')"
    );
    assert_eq!(result(&src), "left|192|64|8|0|false|MyriadPro-Regular|#000000|1|false|1.2|percentage|0|0|0|0|24pt|false");
}

#[test]
fn new_fields_can_be_set_and_read_back() {
    let src = format!(
        "{NEW} dom.setTextRuns({{initialAttrs: {{face: 'MyriadPro-Regular', size: '20pt', horizontalScale: 1.5, leading: 30, leadingMode: 'exact',\n\
            paragraphIndent: 4, paragraphSpacingBefore: 2, paragraphSpacingAfter: 3, rangeKerning: 5, baselineShift: 1,\n\
            antiAliasSharpness: 100, antiAliasStrength: 50, overSample: 4, alignment: 'center', fillColor: '#ff0000'}},\n\
            textRuns: [{{changedAttrs: {{}}, characters: 'Abc'}}]}});\n\
         var a = fw.selection[0].textRuns.initialAttrs;\n\
         [a.horizontalScale, a.leading, a.leadingMode, a.paragraphIndent, a.paragraphSpacingBefore, a.paragraphSpacingAfter, a.rangeKerning, a.baselineShift,\n\
          a.antiAliasSharpness, a.antiAliasStrength, a.overSample, a.alignment, a.fillColor].join('|')"
    );
    assert_eq!(result(&src), "1.5|30|exact|4|2|3|5|1|100|50|4|center|#ff0000");
}

#[test]
fn round_trip_through_set_text_runs_keeps_values() {
    let src = format!(
        "{NEW} dom.setTextRuns({{initialAttrs: {{face: 'MyriadPro-Regular', size: '18pt', horizontalScale: 0.8, rangeKerning: 2}}, textRuns: [{{changedAttrs: {{}}, characters: 'X'}}]}});\n\
         var before = fw.selection[0].textRuns.toSource(); dom.setTextRuns(fw.selection[0].textRuns);\n\
         before == fw.selection[0].textRuns.toSource()"
    );
    assert_eq!(result(&src), "true");
}

#[test]
fn face_is_postscript_name_for_family_names() {
    let src = format!(
        "{NEW} dom.setTextRuns({{initialAttrs: {{face: 'Myriad Pro', size: '24pt'}}, textRuns: [{{changedAttrs: {{}}, characters: 'Abc'}}]}});\n\
         fw.selection[0].textRuns.initialAttrs.face"
    );
    assert_eq!(result(&src), "MyriadPro-Regular");
}

#[test]
fn text_runs_to_source_has_original_shape() {
    let src = format!(
        "{NEW} dom.setTextRuns({{initialAttrs: {{face: 'MyriadPro-Regular', size: '24pt', leading: 1.2}}, textRuns: [{{changedAttrs: {{}}, characters: 'Abc 123'}}]}});\n\
         fw.selection[0].textRuns.toSource()"
    );
    assert_eq!(
        result(&src),
        "{ initialAttrs:{ alignment:\"left\", antiAliasSharpness:192, antiAliasStrength:64, baselineShift:0, bold:false, face:\"MyriadPro-Regular\", fillColor:\"#000000\", horizontalScale:1, italic:false, leading:1.2, leadingMode:\"percentage\", overSample:8, paragraphIndent:0, paragraphSpacingAfter:0, paragraphSpacingBefore:0, rangeKerning:0, size:\"24pt\", underline:false }, textRuns:[ { changedAttrs:{ alignment:\"left\", baselineShift:0, bold:false, face:\"MyriadPro-Regular\", fillColor:\"#000000\", horizontalScale:1, italic:false, leading:1.2, leadingMode:\"percentage\", paragraphIndent:0, paragraphSpacingAfter:0, paragraphSpacingBefore:0, rangeKerning:0, size:\"24pt\", underline:false }, characters:\"Abc 123\" } ] }"
    );
}

// ------------------------------------------------------------------ TW-08

#[test]
fn to_source_objects_without_parentheses() {
    assert_eq!(result("({a: 1, b: [1, 2], c: {d: 'x'}}).toSource()"), "{ a:1, b:[ 1, 2 ], c:{ d:\"x\" } }");
    assert_eq!(result("({}).toSource()"), "{}");
    assert_eq!(result("uneval({x: 36, y: 36})"), "{ x:36, y:36 }");
}

#[test]
fn to_source_guides_like_original() {
    let src = format!("{NEW} var g = dom.guides.toSource(); g.charAt(0) + g.charAt(g.length - 1)");
    assert_eq!(result(&src), "{}");
}

#[test]
fn to_source_arrays_and_strings_unchanged() {
    assert_eq!(result("[1, 'zwei', {x: 3}].toSource()"), "[ 1, \"zwei\", { x:3 } ]");
    assert_eq!(result("[].toSource()"), "[]");
    assert_eq!(result("uneval('a\"b')"), "\"a\\\"b\"");
}
