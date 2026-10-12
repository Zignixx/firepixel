//! Typregeln und Laufzeitverhalten nach 03 §10.3/§10.4/§10.11/§10.12 sowie Grundfunktionen der API.

use fp_script::{run_source, ScriptOptions};

fn run(src: &str) -> fp_script::ScriptOutcome {
    let mut s = fp_engine::Session::default();
    run_source(&mut s, src, &ScriptOptions::default())
}

fn result(src: &str) -> String {
    let o = run(src);
    assert!(o.ok, "Fehler: {:?}\nDiagnose: {:?}", o.error, o.diagnostics);
    o.result.unwrap_or_default()
}

const NEW: &str = "var dom = fw.createFirepixelDocument({x: 200, y: 200}, {pixelsPerUnit: 72, units: 'inch'}, '#ffffff'); dom = fw.getDocumentDOM();\n";

fn exc(body: &str) -> String {
    result(&format!("{NEW} var r; try {{ {body}; r = 'OK'; }} catch (e) {{ r = typeof e + ' ' + e; }} r"))
}

#[test]
fn t1_too_few_args_throws_bad_param_count() {
    assert_eq!(exc("dom.setElementVisible(-1, -1)"), "number -2147216337");
}

#[test]
fn t2_extra_args_are_ignored() {
    assert_eq!(exc("dom.selectNone(1, 2, 3)"), "OK");
}

#[test]
fn t3_null_for_bool_throws_bad_param_type() {
    assert_eq!(exc("dom.setElementVisible(-1, -1, -1, null)"), "number -2147216343");
}

#[test]
fn t4_null_for_string_is_accepted() {
    assert_eq!(result(&format!("{NEW} var n = dom.layers.length; dom.addNewLayer(null, false); dom.layers.length - n")), "1");
}

#[test]
fn t5_t6_loose_conversion() {
    assert_eq!(result("Files.getExtension(123)"), "");
    assert_eq!(result("Files.getFilename('file:///C|/a/b.txt')"), "b.txt");
    assert_eq!(result("Files.getFilename('file:///C|/a/b.txt', 'true')"), "b");
}

#[test]
fn t8_errors_are_strings_except_user_canceled() {
    assert_eq!(result("typeof Errors.EBadParamCount"), "string");
    assert_eq!(result("typeof Errors.EUserCanceled + ' ' + Errors.EUserCanceled"), "number -2147200504");
}

#[test]
fn t9_readonly_assignment_is_ignored() {
    assert_eq!(result(&format!("{NEW} var a = dom.pagesCount; dom.pagesCount = a + 5; dom.pagesCount == a")), "true");
}

#[test]
fn t10_typeof_and_function_length() {
    assert_eq!(result(&format!("{NEW} typeof dom.selectNone + '/' + typeof fw.getDocumentDOM + '/' + dom.selectNone.length + '/' + typeof dom.pagesCount")), "function/function/0/number");
}

#[test]
fn t11_get_document_dom_variants() {
    assert_eq!(result(&format!("{NEW} fw.getDocumentDOM('document') == dom")), "false");
    assert_eq!(result(&format!("{NEW} fw.getDocumentDOM('master') === null")), "true");
    assert_eq!(exc("fw.getDocumentDOM('foo')"), "number -2147221503");
}

#[test]
fn t12_custom_data_types() {
    let r = result(&format!(
        "{NEW} dom.addNewRectangle({{left:10, top:10, right:60, bottom:60}}, 0); var el = fw.selection[0];
         el.customData['num'] = 1.5; el.customData['bool'] = true; el.customData['arr'] = [1, 'zwei', {{x:3}}];
         el.customData['s12'] = '12'; el.customData['str'] = '[1,2]';
         var cd = fw.selection[0].customData;
         [typeof cd.num, typeof cd.bool, cd.arr instanceof FwArray, cd.arr instanceof Array, cd.arr.length, typeof cd.s12, typeof cd.str, cd.arr.toSource()].join('|')"
    ));
    assert_eq!(r, "number|boolean|true|false|3|number|string|[ 1, \"zwei\", { x:3 } ]");
}

#[test]
fn t13_left_writable_width_readonly() {
    let r = result(&format!(
        "{NEW} dom.addNewRectangle({{left:10, top:10, right:60, bottom:60}}, 0); var el = fw.selection[0];
         var a = el.left; el.left = a + 7; var w = el.width; el.width = w + 7; (el.left - a) + '/' + (el.width - w)"
    ));
    assert_eq!(r, "7/0");
}

#[test]
fn not_implemented_throws_clear_error() {
    // bewusst ausgelassener Member (Flash-Export)
    let r = exc("fw.exportSWF(null, \"file:///C|/x.swf\")");
    assert!(r.contains("nicht implementiert: Firepixel.exportSWF"), "{r}");
    // UE-5: Meldung folgt der Sprache (Schlüssel err-script-not-implemented)
    let en = fp_i18n::with_language("en", || exc("fw.exportSWF(null, \"file:///C|/x.swf\")"));
    assert!(en.contains("not implemented: Firepixel.exportSWF"), "{en}");
    assert!(exc("fw.throwInternalError()").contains("-2147221484"));
}

#[test]
fn drawing_attributes_and_effects() {
    let r = result(&format!(
        "{NEW} dom.addNewOval({{left:20, top:20, right:120, bottom:80}});
         dom.setFillNColor({{category:'fc_Solid', name:'fn_Normal', shape:'solid'}}, '#3366cc');
         dom.setBrushNColor({{category:'bc_Basic', name:'bn_Hard', diameter:3}}, '#ff0000');
         dom.applyEffects({{category:'UNUSED', name:'UNUSED', effects:[{{EffectMoaID:'{{a7944db8-6ce2-11d1-8c76000502701850}}', EffectIsVisible:true, ShadowDistance:7, ShadowColor:'#000000a2'}}]}});
         var e = fw.selection[0];
         [String(e), e.pathAttributes.fillColor, e.pathAttributes.brushColor, e.pathAttributes.brush.diameter, e.effectList.effects.length, e.effectList.effects[0].ShadowDistance].join('|')"
    ));
    assert_eq!(r, "[object Path]|#3366cc|#ff0000|3|1|7");
}

#[test]
fn layers_frames_elements_and_contours() {
    let r = result(&format!(
        "{NEW} dom.addNewLine({{x:10, y:10}}, {{x:90, y:10}}); var p = fw.selection[0];
         var c = p.contours[0]; c.nodes.length = 3; c.nodes[2].x = 50; c.nodes[2].y = 60; c.isClosed = true;
         var e = dom.layers[0].frames[0].elements;
         [dom.layers.length, dom.layers[dom.layers.length-1].layerType, e.length, p.contours[0].nodes.length, p.contours[0].nodes[2].y, p.contours[0].isClosed].join('|')"
    ));
    assert_eq!(r, "2|web|1|3|60|true");
}

#[test]
fn undo_history_and_selection_assignment() {
    let r = result(&format!(
        "{NEW} dom.addNewRectangle({{left:0, top:0, right:10, bottom:10}}, 0); var a = fw.selection[0];
         dom.addNewRectangle({{left:20, top:0, right:30, bottom:10}}, 0); var b = fw.selection[0];
         fw.selection = [a, b]; var n = fw.selection.length; dom.group();
         var g = fw.selection[0] + ''; fw.undo();
         [n, g, dom.layers[0].frames[0].elements.length, fw.historyPalette.getStepCount() > 0].join('|')"
    ));
    assert_eq!(r, "2|[object Group]|2|true");
}

#[test]
fn files_roundtrip_and_save_reopen() {
    let dir = std::env::temp_dir().join(format!("fp_script_api_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let url = fp_script::urls::path_to_url(&dir);
    let r = result(&format!(
        "var D = '{url}/'; Files.createDirectory(D);
         var f = Files.open(D + 'a.txt', true); f.write('Zeile1\\nZeile2'); f.close();
         var g = Files.open(D + 'a.txt', false); var l1 = g.readline(); var l2 = g.readline(); var l3 = g.readline(); g.close();
         {NEW} dom.addNewRectangle({{left:5, top:5, right:50, bottom:40}}, 0); fw.selection[0].customData['k'] = 'v';
         fw.saveDocument(dom, D + 'x.fw.png'); dom.close(false);
         var d2 = fw.openDocument(D + 'x.fw.png', false, true);
         var els = d2.layers[0].frames[0].elements;
         [l1, l2, l3, Files.exists(D + 'x.fw.png'), els.length, els[0].customData.k, Files.enumFiles(D).length].join('|')"
    ));
    assert_eq!(r, "Zeile1|Zeile2||true|1|v|2");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn single_undo_step_option() {
    let mut s = fp_engine::Session::default();
    assert!(s.execute("file.new", serde_json::json!({"width": 50, "height": 50})).is_ok());
    let o = run_source(
        &mut s,
        "var d = fw.getDocumentDOM(); d.addNewRectangle({left:0,top:0,right:5,bottom:5},0); d.addNewOval({left:10,top:10,right:20,bottom:20});",
        &ScriptOptions { single_undo_step: true, ..ScriptOptions::default() },
    );
    assert!(o.ok, "{:?}", o.error);
    assert_eq!(s.active().map(|d| d.history.undo_count()), Some(1));
}
