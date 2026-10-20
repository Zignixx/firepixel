//! F4: TW-16/Q-15 `pixelRect` von Text = gezeichnete Glyphen (ohne Glyphen ±2³¹), Q-14
//! `pixelRect` nach „Einfügen innen“ = Maskenbereich, TW-28 `dom.guides.locked`.

use fp_script::{ScriptOptions, run_source};

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

fn result(src: &str) -> String {
    let mut s = fp_engine::Session::default();
    let o = run_source(&mut s, src, &ScriptOptions::default());
    assert!(o.ok, "Fehler: {:?}\nDiagnose: {:?}", o.error, o.diagnostics);
    o.result.unwrap_or_default()
}

const NEW: &str = "var dom = fw.createFirepixelDocument({x: 800, y: 600}, {pixelsPerUnit: 72, units: 'inch'}, '#ffffff'); dom = fw.getDocumentDOM();\n";

fn rect(r: &str) -> [f64; 4] {
    let v: Vec<f64> = r
        .split(',')
        .map(|x| x.trim().parse().unwrap_or(f64::NAN))
        .collect();
    [v[0], v[1], v[2], v[3]]
}

/// TX2 aus par_text: Rahmen 48,148–252,200 (Textfeld 50,150–250,198), drei kurze Zeilen 14 pt.
/// Original 50,150–107,194 (Läufer menu-text/06).
#[test]
fn text_pixel_rect_is_ink() {
    if !has_myriad() {
        return;
    }
    let src = format!(
        "{NEW} dom.addNewText({{left: 50, top: 150, right: 250, bottom: 198}}, true);\n\
         dom.setTextRuns({{initialAttrs: {{face: 'MyriadPro-Regular', size: '14pt'}}, textRuns: [{{changedAttrs: {{}}, characters: 'Zeile eins\\rZeile zwei\\rZeile drei'}}]}});\n\
         var p = fw.selection[0].pixelRect; [p.left, p.top, p.right, p.bottom].join(',')"
    );
    let r = rect(&result(&src));
    eprintln!("TX2 pixelRect {r:?}");
    // Tinte statt Rahmen (vorher 48,148–252,200); Schrift-Ersatz erlaubt ±3 px
    let want = [50.0, 150.0, 107.0, 194.0];
    assert!(
        r.iter().zip(want).all(|(a, b)| (a - b).abs() <= 3.0),
        "{r:?} statt {want:?}"
    );
}

/// Q-15: Text ohne gezeichnete Glyphen (leer) → leeres Rechteck ±2³¹ wie im Original.
#[test]
fn text_without_glyphs_is_empty_rect() {
    let src = format!(
        "{NEW} dom.addNewText({{left: 48, top: 48, right: 58, bottom: 1048}}, true);\n\
         dom.setTextRuns({{initialAttrs: {{face: 'MyriadPro-Regular', size: '14pt'}}, textRuns: [{{changedAttrs: {{}}, characters: ''}}]}});\n\
         var p = fw.selection[0].pixelRect; [p.left, p.top, p.right, p.bottom].join(',')"
    );
    assert_eq!(
        result(&src),
        "-2147483648,-2147483648,2147483648,2147483648"
    );
}

/// Q-14: E1 ohne Überlappung innen in R1 einfügen → pixelRect = R1, danach nichts ausgewählt.
#[test]
fn paste_inside_reports_mask_area() {
    let src = format!(
        "{NEW} dom.addNewRectangle({{left: 50, top: 50, right: 150, bottom: 120}}, 0);\n\
         dom.addNewOval({{left: 50, top: 170, right: 150, bottom: 240}});\n\
         fw.getDocumentDOM().clipCopy();\n\
         fw.selection = [dom.layers[dom.currentLayerNum].frames[0].elements[1]];\n\
         dom.clipPasteInside('do not resample');\n\
         var n = fw.selection.length; var e = dom.layers[dom.currentLayerNum].frames[0].elements[1]; var p = e.pixelRect;\n\
         n + '|' + [p.left, p.top, p.right, p.bottom].join(',')"
    );
    assert_eq!(result(&src), "0|50,50,150,120");
}

/// TW-28: Ansicht ▸ Hilfslinien sperren wirkt auf `dom.guides.locked`.
#[test]
fn guides_locked_reads_view_lock() {
    let src = format!(
        "{NEW} var a = dom.guides.locked; dom.lockGuides(true); a + '|' + dom.guides.locked + '|' + dom.getLockGuides()"
    );
    assert_eq!(result(&src), "false|true|true");
}
