//! S-1: Grenzen einer Maskengruppe (re 29b §1b) und `dom.getPixelMask` (re 28 L7).

use fp_script::{ScriptOptions, run_source};

fn result(src: &str) -> String {
    fp_engine::shell::disable();
    let mut s = fp_engine::Session::default();
    let o = run_source(&mut s, src, &ScriptOptions::default());
    assert!(o.ok, "Fehler: {:?}\nDiagnose: {:?}", o.error, o.diagnostics);
    o.result.unwrap_or_default()
}

const NEW: &str = "var dom = fw.createFirepixelDocument({x: 500, y: 300}, {pixelsPerUnit: 72, units: 'inch'}, '#ffffff'); dom = fw.getDocumentDOM();\n";

/// R4 und E1 unter M1, „Als Maske gruppieren“: B/H/X/Y = Vereinigung der Kinder (50,50, 140 × 140),
/// `pixelRect` = Maskenbereich 70,70–170,170.
#[test]
fn mask_group_reports_union_of_children() {
    let src = format!(
        "{NEW} dom.addNewRectangle({{left: 50, top: 50, right: 150, bottom: 150}}, 0);\n\
         dom.addNewOval({{left: 90, top: 90, right: 190, bottom: 190}});\n\
         dom.addNewRectangle({{left: 70, top: 70, right: 170, bottom: 170}}, 0);\n\
         dom.selectAll(); dom.group('mask to path');\n\
         var e = fw.selection[0], p = e.pixelRect;\n\
         [e.left, e.top, e.width, e.height, p.left, p.top, p.right, p.bottom].join(',')"
    );
    assert_eq!(result(&src), "50,50,140,140,70,70,170,170");
}

/// Ohne Pixelauswahl `null`; Rechteck als Auswahlrahmen → Grenzen und 255 je Pixel.
#[test]
fn get_pixel_mask_returns_bounds_and_bytes() {
    assert_eq!(result(&format!("{NEW} String(dom.getPixelMask())")), "null");
    let src = format!(
        "{NEW} dom.addNewRectangle({{left: 330, top: 60, right: 380, bottom: 110}}, 0);\n\
         dom.convertPathToMarquee(0, 0);\n\
         var m = dom.getPixelMask(), b = m.maskBounds, d = m.maskData, all = true;\n\
         for (var i = 0; i < d.length; i++) if (d[i] != 255) all = false;\n\
         [b.left, b.top, b.right, b.bottom, m.width, m.height, d.length, all].join(',')"
    );
    assert_eq!(result(&src), "330,60,380,110,50,50,2500,true");
}

/// Rand 5 (re 28 L7): weiche Maske 324,54–386,116, Profil der Zeile y = 85 links 15 … 239 … 15.
#[test]
fn get_pixel_mask_border_profile() {
    let src = format!(
        "{NEW} dom.addNewRectangle({{left: 330, top: 60, right: 380, bottom: 110}}, 0);\n\
         dom.convertPathToMarquee(0, 0); dom.selectAdjustPixelSel('border', 5);\n\
         var m = dom.getPixelMask(), b = m.maskBounds, row = [];\n\
         for (var x = 0; x < 12; x++) row.push(m.maskData[(85 - b.top) * m.width + x]);\n\
         [b.left, b.top, b.right, b.bottom].join(',') + '|' + row.join(' ')"
    );
    assert_eq!(
        result(&src),
        "324,54,386,116|15 47 95 159 207 239 239 207 159 95 47 15"
    );
}
