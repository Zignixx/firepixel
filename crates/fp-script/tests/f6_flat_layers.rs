//! F6 (Befund 1 aus `docs\berichte\f2.md`): `dom.layers` führt Unterebenen flach wie das Original –
//! jede Ebene, direkt danach ihre Unterebenen (unten → oben). `frames[n].elements` enthält keine
//! Unterebenen mehr (`elemsandsublayers` schon), `dom.topLayers` bleibt bei der obersten Stufe.

use fp_script::{run_source, ScriptOptions};
use serde_json::json;

fn session() -> fp_engine::Session {
    fp_engine::shell::disable();
    fp_i18n::profile::use_test_profile();
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 200, "height": 150, "background": "#ffffff"})).unwrap_or_else(|e| panic!("{e}"));
    s
}

fn result(s: &mut fp_engine::Session, src: &str) -> String {
    let o = run_source(s, src, &ScriptOptions::default());
    assert!(o.ok, "Fehler: {:?}\nDiagnose: {:?}", o.error, o.diagnostics);
    o.result.unwrap_or_default()
}

/// Ausdruck des Läufers (xs-file-formats/05) ohne `pixelRect`.
const LIST: &str = "(function(){var d=fw.getDocumentDOM(),r=[];for(var l=0;l<d.layers.length;l++){var e=d.layers[l].frames[d.currentFrameNum].elements;for(var i=0;i<e.length;i++){r.push([l,e[i].name,e[i].toString()]);}}return r.toSource();})()";

#[test]
fn layers_flat_with_sublayers() {
    let mut s = session();
    s.execute("draw.rectangle", json!({"rect": [0, 0, 50, 50], "name": "BG"})).unwrap_or_else(|e| panic!("{e}"));
    s.execute("pi.layer.addSub", json!({"name": "S1"})).unwrap_or_else(|e| panic!("{e}"));
    // seit B-5 wird die neue Unterebene aktiv (B-2 BA-030): S2 wieder in Ebene 0 anlegen
    result(&mut s, "fw.getDocumentDOM().currentLayerNum = 0");
    s.execute("pi.layer.addSub", json!({"name": "S2"})).unwrap_or_else(|e| panic!("{e}"));
    let names = result(&mut s, "var d=fw.getDocumentDOM(),r=[];for(var i=0;i<d.layers.length;i++)r.push(d.layers[i].name);r.join('|')");
    let parts: Vec<&str> = names.split('|').collect();
    assert_eq!(parts.len(), 4, "{names}");
    assert_eq!(&parts[1..3], &["S1", "S2"], "Unterebenen direkt nach ihrer Ebene, unten → oben: {names}");
    assert_eq!(result(&mut s, "fw.getDocumentDOM().topLayers.length"), "2");
    // Elemente der Ebene ohne Unterebenen, mit `elemsandsublayers` alle
    assert_eq!(result(&mut s, "var l=fw.getDocumentDOM().layers[0].frames[0];[l.elements.length,l.elemsandsublayers.length].join(',')"), "1,3");
    // eine Unterebene liefert ihren eigenen Inhalt
    assert_eq!(result(&mut s, "fw.getDocumentDOM().layers[1].frames[0].elements.length"), "0");
    assert_eq!(result(&mut s, "fw.getDocumentDOM().frames[0].layers.length"), "4");
}

#[test]
fn par_psd_like_original() {
    let src = std::path::Path::new(r"C:\Users\Administrator\Desktop\REA\re\_testdateien\par\par_import\par.psd");
    if !src.is_file() {
        eprintln!("übersprungen: {src:?} fehlt");
        return;
    }
    let mut s = session();
    s.execute("file.open", json!({"path": src.to_string_lossy()})).unwrap_or_else(|e| panic!("{e}"));
    // Original (xs-file-formats/05_open_psd_result): Background, Pfad, PSD je in eigener Ebene 0, 1, 2
    assert_eq!(result(&mut s, LIST), r#"[[0, "Background", "[object Image]"], [1, "Pfad", "[object Path]"], [2, "PSD", "[object Text]"]]"#);
}
