//! F9: TW-35 `pixelRect` einer Vektor-Maskengruppe = Inhalt ∩ aktive Maske (Als Maske gruppieren,
//! Lauf `menu-modify-a/16`, `menu-modify-b/20`), nach Maske deaktivieren der Inhalt; Stapelplatz
//! der Maskengruppe = Platz des untersten Objekts. TW-34 Einfügen direkt über der Auswahl.

use fp_script::{run_source, ScriptOptions};

fn result(src: &str) -> String {
    let mut s = fp_engine::Session::default();
    let o = run_source(&mut s, src, &ScriptOptions::default());
    assert!(o.ok, "Fehler: {:?}\nDiagnose: {:?}", o.error, o.diagnostics);
    o.result.unwrap_or_default()
}

const NEW: &str = "var dom = fw.createFirepixelDocument({x: 800, y: 600}, {pixelsPerUnit: 72, units: 'inch'}, '#ffffff'); dom = fw.getDocumentDOM();\n\
    dom.addNewRectangle({left: 50, top: 50, right: 150, bottom: 120}, 0); fw.selection[0].name = 'R1';\n\
    dom.addNewRectangle({left: 200, top: 50, right: 300, bottom: 120}, 0); fw.selection[0].name = 'R2';\n\
    dom.addNewOval({left: 50, top: 170, right: 150, bottom: 240}); fw.selection[0].name = 'E1';\n\
    var els = function() { return dom.layers[dom.currentLayerNum].frames[0].elements; };\n\
    var pr = function(e) { var p = e.pixelRect; return [p.left, p.top, p.right, p.bottom].join(','); };\n\
    var names = function() { var r = [], e = els(); for (var i = 0; i < e.length; i++) r.push(e[i].name); return r.join(' '); };\n";

/// E1 (oben) als Maske über R1 ohne Überlappung: leerer Schnitt ±2³¹, Gruppe an R1s Platz unter R2;
/// deaktiviert wieder R1 50,50–150,120 (Original-Werte des Laufs).
#[test]
fn group_as_mask_reports_intersection() {
    let src = format!(
        "{NEW} fw.selection = [els()[0], els()[2]];\n\
         dom.group('mask to path');\n\
         var a = names() + '|' + pr(els()[1]);\n\
         dom.enableElementMask(false);\n\
         a + '|' + pr(els()[1])"
    );
    assert_eq!(result(&src), "R2 R1|-2147483648,-2147483648,2147483648,2147483648|50,50,150,120");
}

/// Mit Überlappung meldet die Maskengruppe den Schnitt aus Inhalt und Maske.
#[test]
fn group_as_mask_overlap_is_cut() {
    let src = format!(
        "{NEW} dom.addNewRectangle({{left: 100, top: 80, right: 400, bottom: 300}}, 0);\n\
         fw.selection = [els()[0], els()[3]];\n\
         dom.group('mask to path');\n\
         pr(els()[els().length - 1])"
    );
    assert_eq!(result(&src), "100,80,150,120");
}

/// Strg+C, 2× Strg+V auf R1: Die Kopien liegen direkt über R1.
#[test]
fn paste_lands_above_selection() {
    let src = format!(
        "{NEW} fw.selection = [els()[2]];\n\
         dom.clipCopy(); dom.clipPaste(); dom.clipPaste();\n\
         names() + '|' + pr(els()[2])"
    );
    assert_eq!(result(&src), "E1 R2 R1 R1 R1|50,50,150,120");
}
