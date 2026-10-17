//! EB-1 (Befund PM-1): `RectanglePrimitive.originalSides` liefert wie im Original absolute
//! Dokumentkoordinaten statt halber Breite/Höhe um die Mitte.

use fp_script::{ScriptOptions, run_source};

fn result(src: &str) -> String {
    let mut s = fp_engine::Session::default();
    let o = run_source(&mut s, src, &ScriptOptions::default());
    assert!(o.ok, "Fehler: {:?}\nDiagnose: {:?}", o.error, o.diagnostics);
    o.result.unwrap_or_default()
}

const NEW: &str = "var dom = fw.createFirepixelDocument({x: 400, y: 300}, {pixelsPerUnit: 72, units: 'inch'}, '#ffffff'); dom = fw.getDocumentDOM();\n";
const SIDES: &str = "function sides(){ var r = fw.selection[0].originalSides; return [r.left, r.top, r.right, r.bottom].join(','); }\n";

#[test]
fn original_sides_are_absolute() {
    // R1 50,50–150,120 (Original: {left: 50, top: 50, right: 150, bottom: 120})
    assert_eq!(
        result(&format!(
            "{NEW}{SIDES} dom.addNewRectangle({{left:50, top:50, right:150, bottom:120}}, 0); sides()"
        )),
        "50,50,150,120"
    );
    // neu aufgezogenes Rechteck 240,145–300,175 (vorher -30,-15,30,15)
    assert_eq!(
        result(&format!(
            "{NEW}{SIDES} dom.addNewRectangle({{left:240, top:145, right:300, bottom:175}}, 0); sides()"
        )),
        "240,145,300,175"
    );
    // Eckgriff gezogen (Größe/Lage gesetzt): die neuen Seiten 50,50,191,150
    assert_eq!(
        result(&format!(
            "{NEW}{SIDES} dom.addNewRectangle({{left:50, top:50, right:150, bottom:120}}, 0); dom.setSelectionBounds({{left:50, top:50, right:191, bottom:150}}, 'autoTrimImages transformAttributes'); sides()"
        )),
        "50,50,191,150"
    );
    // verschoben
    assert_eq!(
        result(&format!(
            "{NEW}{SIDES} dom.addNewRectangle({{left:50, top:50, right:150, bottom:120}}, 0); dom.moveSelectionBy({{x:10, y:5}}, false, false); sides()"
        )),
        "60,55,160,125"
    );
    // gedreht (Transformation separat): das ungedrehte Rechteck um die Mitte bleibt
    assert_eq!(
        result(&format!(
            "{NEW}{SIDES} dom.addNewRectangle({{left:50, top:50, right:150, bottom:120}}, 0); dom.rotateSelection(30, 'autoTrimImages transformAttributes'); sides()"
        )),
        "50,50,150,120"
    );
    // 50× lesen ändert nichts
    assert_eq!(
        result(&format!(
            "{NEW}{SIDES} dom.addNewRectangle({{left:50, top:50, right:150, bottom:120}}, 0); var x; for (var i = 0; i < 50; i++) x = sides(); x"
        )),
        "50,50,150,120"
    );
}
