//! S-5: TW-56b (re 33 §1) Text ▸ In Pfade konvertieren liefert rastergerechte Glyphenkonturen.

use fp_script::{ScriptOptions, run_source};

fn result(src: &str) -> String {
    let mut s = fp_engine::Session::default();
    let o = run_source(&mut s, src, &ScriptOptions::default());
    assert!(o.ok, "Fehler: {:?}\nDiagnose: {:?}", o.error, o.diagnostics);
    o.result.unwrap_or_default()
}

const NEW: &str = "var dom = fw.createFirepixelDocument({x: 800, y: 600}, {pixelsPerUnit: 72, units: 'inch'}, '#ffffff'); dom = fw.getDocumentDOM();\n";

/// TX2 (Myriad Pro 14 pt, drei Zeilen): letztes „i“ von „zwei“ 105,0625–105,9375, Gruppe
/// `pixelRect` rechts 106 (vorher ungehintet 104,868–106,408 → 107).
#[test]
fn convert_to_paths_tx2_is_hinted() {
    if !has_myriad() {
        return;
    }
    let src = format!(
        "{NEW} dom.addNewText({{left: 50, top: 150, right: 250, bottom: 198}}, true);\n\
         dom.setTextRuns({{initialAttrs: {{face: 'MyriadPro-Regular', size: '14pt'}}, textRuns: [{{changedAttrs: {{}}, characters: 'Zeile eins\\rZeile zwei\\rZeile drei'}}]}});\n\
         dom.convertToPaths(); var g = fw.selection[0]; var p = g.pixelRect;\n\
         var xs = []; for (var i = 0; i < g.elements.length; i++) {{ var cs = g.elements[i].contours;\n\
           for (var c = 0; c < cs.length; c++) {{ var ns = cs[c].nodes; for (var k = 0; k < ns.length; k++) {{\n\
             var y = ns[k].y; if (ns[k].x > 104.5 && y > 165 && y < 178) xs.push(ns[k].x.toFixed(4)); }} }} }}\n\
         [p.left, p.top, p.right, p.bottom].join(',') + '|' + xs.join(',')"
    );
    let r = result(&src);
    eprintln!("TX2 In Pfade: {r}");
    let (rect, xs) = r.split_once('|').expect("Ergebnis");
    let v: Vec<f64> = rect
        .split(',')
        .map(|x| x.parse().unwrap_or(f64::NAN))
        .collect();
    assert_eq!(v[2], 106.0, "pixelRect rechts 106 wie im Original: {r}");
    assert_eq!((v[0], v[1], v[3]), (50.0, 150.0, 194.0), "{r}");
    let xs: Vec<&str> = xs.split(',').collect();
    assert!(
        xs.contains(&"105.0625") && xs.contains(&"105.9375"),
        "Stamm des „i“: {xs:?}"
    );
    assert!(
        xs.iter().all(|x| x
            .parse::<f64>()
            .is_ok_and(|x| (105.0625..=105.9375).contains(&x))),
        "„i“ innerhalb 105,0625…105,9375: {xs:?}"
    );
}

/// Myriad Pro installiert? Die Metrik-Werte stammen aus dem Original mit Myriad Pro; ohne die Schrift
/// (z. B. auf den macOS-/Linux-Testläufern) wird der Test übersprungen.
fn has_myriad() -> bool {
    let ok = fp_text::FontDb::shared()
        .find_ps("MyriadPro-Regular")
        .is_some();
    if !ok {
        eprintln!("skipped: MyriadPro-Regular not installed");
    }
    ok
}
