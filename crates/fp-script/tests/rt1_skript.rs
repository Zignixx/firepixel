//! RT-1 (re 27 K1): `createFirepixelDocument` kennt als Einheit der Auflösung nur „inch“, „cm“ und
//! „pixels“; jeder andere Name (z. B. „centimeter“) ist der Skriptfehler 0x80040001, die Zahl wird
//! nie in dpi umgerechnet.

use fp_script::{ScriptOptions, run_source};

fn result(src: &str) -> String {
    fp_engine::shell::disable();
    fp_i18n::profile::use_test_profile();
    let mut s = fp_engine::Session::default();
    let o = run_source(&mut s, src, &ScriptOptions::default());
    assert!(
        o.ok,
        "Fehler: {:?}\nDiagnose: {:?}\nSkript: {src}",
        o.error, o.diagnostics
    );
    o.result.unwrap_or_default()
}

fn create(units: &str) -> String {
    result(&format!(
        "var r; try {{ var d = fw.createFirepixelDocument({{x: 20, y: 10}}, {{pixelsPerUnit: 28.35, units: '{units}'}}, '#ffffff'); \
         r = [Math.round(d.resolution * 100) / 100, d.resolutionUnits].join(','); d.close(false); }} catch (e) {{ r = typeof e + ' ' + e; }} r"
    ))
}

#[test]
fn k1_units_names_like_original() {
    // 0x80040001 als vorzeichenbehaftete Zahl
    let bad = format!("number {}", 0x8004_0001u32 as i32);
    assert_eq!(create("centimeter"), bad, "„centimeter“ ist ein Fehler");
    assert_eq!(create("Centimeters"), bad);
    assert_eq!(create("dpi"), bad);
    assert_eq!(create("cm"), "28.35,cm", "Zahl unverändert, Einheit cm");
    assert_eq!(create("inch"), "28.35,inch", "Zahl unverändert als dpi");
    assert_eq!(
        create("pixels").split(',').next(),
        Some("28.35"),
        "„pixels“ wird angenommen"
    );
    // ohne Einheit: wie bisher Zoll
    let r = result(
        "var d = fw.createFirepixelDocument({x: 20, y: 10}, {pixelsPerUnit: 96}, '#ffffff'); var r = d.resolution + ',' + d.resolutionUnits; d.close(false); r",
    );
    assert_eq!(r, "96,inch");
}
