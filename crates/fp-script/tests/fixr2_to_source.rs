//! FIX-R2: `toSource()`/`uneval()` wie im Original – Wirtwerte (vom Programm geliefert) und reine
//! JS-Werte werden verschieden ausgegeben. Originalwerte aus dem Paarlauf vom 13.10.2026
//! (`run_lauf1_2026-10-13/*/log.jsonl`).

use fp_script::{ScriptOptions, run_source};

fn result(src: &str) -> String {
    let mut s = fp_engine::Session::default();
    let o = run_source(&mut s, src, &ScriptOptions::default());
    assert!(o.ok, "Fehler: {:?}\nDiagnose: {:?}", o.error, o.diagnostics);
    o.result.unwrap_or_default()
}

const NEW: &str = "var dom = fw.createFirepixelDocument({x: 300, y: 200}, {pixelsPerUnit: 72, units: 'inch'}, '#ffffff'); dom = fw.getDocumentDOM();\n";

// ------------------------------------------------------------------ Regel 1: reine JS-Arrays

#[test]
fn js_array_without_inner_spaces_and_undefined_as_gap() {
    // Original dialogs-d01-d10/02: `[800, 600, , 2, 1]` (d.zoom ist undefined).
    assert_eq!(
        result("[800, 600, undefined, 2, 1].toSource()"),
        "[800, 600, , 2, 1]"
    );
    // Original dialogs-d01-d10/04: zwei undefinierte docTitle -> `[, ]`; drei -> `[, , ]` (menu-file/05).
    assert_eq!(
        result("var r = []; r.push(undefined); r.push(undefined); r.toSource()"),
        "[, ]"
    );
    assert_eq!(
        result("[undefined, undefined, undefined].toSource()"),
        "[, , ]"
    );
    // Original menu-edit/06: `["#808080", , 100]`.
    assert_eq!(
        result("['#808080', undefined, 100].toSource()"),
        "[\"#808080\", , 100]"
    );
    // null ist ebenfalls eine Lücke (BA-073, Beleg M2 q18: `[…, a.brush ? … : null]` → `[…, ]`).
    assert_eq!(result("[1, null].toSource()"), "[1, ]");
}

#[test]
fn js_nested_arrays_like_original() {
    // Original dialogs-d01-d10/11: `[[0, "L1", "[object Path]", 50, 300, 301, 361], …]`.
    assert_eq!(
        result("[[0, 'L1', 50], [1, 'R1', 2]].toSource()"),
        "[[0, \"L1\", 50], [1, \"R1\", 2]]"
    );
    assert_eq!(result("[].toSource()"), "[]");
    assert_eq!(result("uneval([1, [2, 3]])"), "[1, [2, 3]]");
}

// ------------------------------------------------------------------ Regel 2: Wirt-Arrays

#[test]
fn host_arrays_with_inner_spaces_empty_two_spaces() {
    // Original xs-canvas/09: `{ color:"#00ffff", hGuides:[ 300 ], locked:false, vGuides:[  ] }`.
    let src = format!(
        "{NEW} dom.addGuide(300, 'horizontal'); var g = dom.guides; [g.hGuides.toSource(), g.vGuides.toSource()].join('|')"
    );
    assert_eq!(result(&src), "[ 300 ]|[  ]");
}

#[test]
fn host_guides_object_shape() {
    let src = format!("{NEW} dom.addGuide(300, 'horizontal'); dom.guides.toSource()");
    let r = result(&src);
    assert!(r.starts_with("{ "), "{r}");
    assert!(r.contains("hGuides:[ 300 ]"), "{r}");
    assert!(r.contains("vGuides:[  ]"), "{r}");
}

// ------------------------------------------------------------------ Regel 3: Objekte mit/ohne Klammern

#[test]
fn js_object_literal_with_parentheses() {
    assert_eq!(result("({a: 1, b: 'x'}).toSource()"), "({a:1, b:\"x\"})");
    assert_eq!(result("({}).toSource()"), "({})");
    assert_eq!(result("uneval({x: 36, y: 36})"), "({x:36, y:36})");
    // Verschachtelt ohne Klammern, nur außen.
    assert_eq!(
        result("({a: {b: [1, {c: 2}]}}).toSource()"),
        "({a:{b:[1, {c:2}]}})"
    );
    assert_eq!(result("[{x: 3}].toSource()"), "[{x:3}]");
    // Schlüssel, die keine Bezeichner sind, in Anführungszeichen; Indizes ohne.
    assert_eq!(
        result("({'a-b': 1, 0: 2}).toSource()"),
        "({0:2, \"a-b\":1})"
    );
}

#[test]
fn host_objects_without_parentheses() {
    // Original dialogs-d11-d20/15: `{ x:20, y:30 }`.
    let src = format!("{NEW} dom.setGridSize({{x: 20, y: 30}}); dom.gridSize.toSource()");
    assert_eq!(result(&src), "{ x:20, y:30 }");
    // Wirtobjekt bleibt auch innerhalb eines JS-Arrays Wirtform.
    let src = format!("{NEW} dom.setGridSize({{x: 20, y: 30}}); [1, dom.gridSize].toSource()");
    assert_eq!(result(&src), "[1, { x:20, y:30 }]");
}

#[test]
fn host_marker_is_invisible() {
    let src = format!(
        "{NEW} dom.setGridSize({{x: 20, y: 30}}); var g = dom.gridSize, k = []; for (var n in g) k.push(n); k.join(',') + '|' + Object.keys(g).join(',') + '|' + JSON.stringify(g)"
    );
    assert_eq!(result(&src), "x,y|x,y|{\"x\":20,\"y\":30}");
}

// ------------------------------------------------------------------ Regel 4: Steuerzeichen

#[test]
fn host_strings_escape_control_chars_as_unicode() {
    // Original menu-text/15: `characters:"Zeile eins\u000DZeile zwei\u000DZeile drei"`.
    let src = format!(
        "{NEW} dom.addNewText({{left: 10, top: 10, right: 200, bottom: 60}}, true);\n\
         dom.setTextRuns({{initialAttrs: {{face: 'MyriadPro-Regular', size: '14pt'}}, textRuns: [{{changedAttrs: {{}}, characters: 'Zeile eins\\rZeile zwei'}}]}});\n\
         var s = fw.selection[0].textRuns.toSource(); s.substring(s.indexOf('characters:'))"
    );
    assert_eq!(
        result(&src),
        "characters:\"Zeile eins\\u000DZeile zwei\" } ] }"
    );
}

#[test]
fn host_strings_other_control_chars() {
    let src = format!(
        "{NEW} var e = fw.selection; dom.addNewRectangle({{left:10, top:10, right:60, bottom:60}}, 0); var cd = fw.selection[0].customData; cd['arr'] = ['a\\tb\\nc\\u0001\"\\\\']; fw.selection[0].customData.arr.toSource()"
    );
    assert_eq!(result(&src), "[ \"a\\u0009b\\u000Ac\\u0001\\\"\\\\\" ]");
}

#[test]
fn js_strings_like_spidermonkey() {
    assert_eq!(result("uneval('a\\rb')"), "\"a\\rb\"");
    assert_eq!(
        result("['x\\ny\\tz', 'q\"'].toSource()"),
        "[\"x\\ny\\tz\", \"q\\\"\"]"
    );
    assert_eq!(result("uneval('\\u0001\\\\')"), "\"\\x01\\\\\"");
}
