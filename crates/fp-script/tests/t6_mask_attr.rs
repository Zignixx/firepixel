//! T-6 (re 31 §5): „Innen einfügen“ ergibt ein Element mit Maskenattribut. JSF-Sicht wie gemessen
//! (pi_mess A_basis): Klasse des Inhalts (hier Rechteck)/Name/Grenzen/`pathAttributes` vom Inhalt, `pixelRect` und `mask.element`
//! von der Maske; danach nichts gewählt. „Als Maske einfügen“: danach ist die Maske gewählt (S5).

use fp_script::{ScriptOptions, run_source};

fn result(src: &str) -> String {
    let mut s = fp_engine::Session::default();
    // tool stroke of the measurement: 1 px (the program starts with 2 px, RE spec 59 section 3.2)
    s.tools.options.new_stroke = None;
    let o = run_source(&mut s, src, &ScriptOptions::default());
    assert!(o.ok, "Fehler: {:?}\nDiagnose: {:?}", o.error, o.diagnostics);
    o.result.unwrap_or_default()
}

const NEW: &str = "var dom = fw.createFirepixelDocument({x: 800, y: 600}, {pixelsPerUnit: 72, units: 'inch'}, '#ffffff'); dom = fw.getDocumentDOM();\n";

/// A_basis: E1 (100,80 100×70, grün) kopiert, R1 (50,50 100×70, schwarz) gewählt, Innen einfügen.
#[test]
fn paste_inside_jsf_view() {
    let src = format!(
        "{NEW} dom.addNewRectangle({{left: 100, top: 80, right: 200, bottom: 150}}, 0); fw.selection[0].name = 'E1';\n\
         dom.setFillColor('#40ff00');\n\
         fw.getDocumentDOM().clipCut();\n\
         dom.addNewRectangle({{left: 50, top: 50, right: 150, bottom: 120}}, 0); fw.selection[0].name = 'R1';\n\
         dom.setFillColor('#000000');\n\
         dom.clipPasteInside('do not resample');\n\
         var n = fw.selection.length; var els = dom.layers[dom.currentLayerNum].frames[0].elements; var e = els[0];\n\
         var p = e.pixelRect; var m = e.mask;\n\
         [n, els.length, String(e), e.name, e.left, e.top, e.width, e.height, [p.left, p.top, p.right, p.bottom].join(':'),\n\
          e.pathAttributes.fillColor, m.mode, m.linked, m.enabled, m.showAttrs, m.autoExpandImages, m.element.name, m.element.left, m.element.top, m.element.pathAttributes.fillColor].join('|')"
    );
    let r = result(&src);
    let v: Vec<&str> = r.split('|').collect();
    assert_eq!(
        &v[..9],
        [
            "0",
            "1",
            "[object RectanglePrimitive]",
            "E1",
            "100",
            "80",
            "100",
            "70",
            "50:50:150:120"
        ],
        "{r}"
    );
    assert!(
        v[9].starts_with("#40ff00"),
        "pathAttributes vom Inhalt: {r}"
    );
    assert_eq!(
        &v[10..18],
        [
            "mask to path",
            "true",
            "true",
            "true",
            "false",
            "R1",
            "50",
            "50"
        ],
        "{r}"
    );
    assert!(
        v[18].starts_with("#000000"),
        "mask.element.pathAttributes von der Maske: {r}"
    );
}
