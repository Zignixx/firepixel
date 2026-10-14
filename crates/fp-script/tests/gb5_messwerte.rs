//! G-B5: Korrekturen nach der Mess-Sitzung B-2b (Bericht `docs\berichte\b2b.md`, Paket B5),
//! je Punkt ein Regressionstest mit den dort gemessenen Werten des Originals.

use fp_script::{run_source, ScriptOptions};
use std::path::PathBuf;

fn dir() -> PathBuf {
    let d = std::env::temp_dir().join(format!("fp-gb5-script-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&d);
    d
}

fn url(p: &std::path::Path) -> String {
    format!("file:///{}", p.to_string_lossy().replace('\\', "/"))
}

fn run(s: &mut fp_engine::Session, src: &str) -> String {
    let o = run_source(s, src, &ScriptOptions::default());
    assert!(o.ok, "Fehler: {:?}\nDiagnose: {:?}\nSkript: {src}", o.error, o.diagnostics);
    o.result.unwrap_or_default()
}

fn result(src: &str) -> String {
    fp_engine::shell::disable();
    fp_i18n::profile::use_test_profile();
    let mut s = fp_engine::Session::default();
    run(&mut s, src)
}

const NEW: &str = "fw.createFirepixelDocument({x: 600, y: 500}, {pixelsPerUnit: 72, units: 'inch'}, '#ffffff'); var dom = fw.getDocumentDOM();\n";

fn phys_x(bytes: &[u8]) -> Option<u32> {
    let chunks = fp_fwpng::png::read_chunks(bytes).expect("PNG");
    chunks.iter().find(|c| c.is(b"pHYs")).map(|c| u32::from_be_bytes([c.data[0], c.data[1], c.data[2], c.data[3]]))
}

// ------------------------------------------------------------------ BA-017

/// BA-017 (HÃ¶he): `rawWidth Ã— rawHeight` eines Texts (â€žHallo Weltâ€œ, Arial) nach
/// `attachTextToPath`, Messwerte des Originals aus B-2b (10 Punkte) und B-2 (Kreis).
#[test]
fn ba017_raw_height_of_text_on_path_matches_measurements() {
    // (Pfad, SchriftgrÃ¶ÃŸe, erwartet "rawWidth,rawHeight")
    let line = |x1: i32, y1: i32, x2: i32, y2: i32| format!("dom.addNewLine({{x: {x1}, y: {y1}}}, {{x: {x2}, y: {y2}}})");
    let ellipse = |l: i32, t: i32, r: i32, b: i32| format!("dom.addNewOval({{left: {l}, top: {t}, right: {r}, bottom: {b}}})");
    let cases: Vec<(String, u32, &str)> = vec![
        (line(50, 400, 400, 400), 14, "350,5"),
        (line(50, 400, 400, 400), 28, "350,10"),
        (line(50, 420, 400, 400), 14, "350,9"),
        (line(50, 440, 400, 400), 14, "350,12"),
        (line(50, 480, 400, 400), 14, "350,19"),
        (line(50, 460, 400, 300), 14, "350,31"),
        (line(50, 440, 400, 400), 28, "350,24"),
        (line(200, 50, 200, 300), 14, "10,65"),
        (ellipse(100, 100, 200, 340), 14, "100,121"),
        (ellipse(100, 100, 220, 220), 14, "120,61"),
    ];
    for (path, size, want) in cases {
        let r = result(&format!(
            "{NEW} dom.addNewText({{left: 60, top: 150, right: 200, bottom: 170}}, true); dom.setTextRuns({{initialAttrs: {{size: '{size}pt', face: 'ArialMT', alignment: 'left'}}, textRuns: [{{changedAttrs: {{}}, characters: 'Hallo Welt'}}]}}); \
             var t = fw.selection[0]; {path}; var p = fw.selection[0]; fw.selection = [p, t]; dom.attachTextToPath(); \
             var e = fw.selection[0]; [e.rawWidth, e.rawHeight].join(',')"
        ));
        assert_eq!(r, want, "{path} {size} pt");
    }
}

// ------------------------------------------------------------------ BA-123

const D: &str = "var d=fw.getDocumentDOM();function ix(n){for(var i=0;i<d.layers.length;i++)if(d.layers[i].name==n)return i;return -1;}\n";
/// Flache Ebenenliste ohne Web-Ebene: `Name` bzw. `Name^Eltern`.
const FLAT: &str = "var r=[];for(var i=0;i<d.layers.length;i++){if(d.layers[i].layerType=='web')continue;var p=d.getParentLayerNum(i);r.push(d.layers[i].name+(p<0?'':'^'+d.layers[p].name));}r.join(',')";

/// Dokument mit `Ebene 1` (umbenannt in E1), `A` mit den Unterebenen `A1`, `A2`, und `B`.
fn layered() -> fp_engine::Session {
    fp_engine::shell::disable();
    fp_i18n::profile::use_test_profile();
    let mut s = fp_engine::Session::default();
    run(&mut s, &format!("{NEW} {D} d.setLayerName(0,'E1'); d.addNewLayer('A',false); d.addNewSubLayer(ix('A'),'A1',false); d.addNewSubLayer(ix('A'),'A2',false); d.currentLayerNum=ix('A'); d.addNewLayer('B',false); 'ok'"));
    s
}

/// BA-123: `reorderLayer(<Unterebene>, <Ebene der obersten Stufe>, false)` â€“ die Unterebene
/// verlÃ¤sst ihre Elternebene und wird Ebene der obersten Stufe direkt unter dem Ziel (flach vor
/// dem Ziel), danach aktiv (Original: `â€¦ A, A2^A, A1, B`; `A1, Ebene 1, A, A2^A`; `A2, A, A1^A`).
#[test]
fn ba123_sublayer_onto_top_level_becomes_top_level_below_target() {
    let mut s = layered();
    let before = run(&mut s, &format!("{D}{FLAT}"));
    assert!(before.starts_with("E1,A,") && before.ends_with(",B") && before.contains("A1^A") && before.contains("A2^A"), "{before}");
    // A1 vor B
    run(&mut s, &format!("{D} d.reorderLayer(ix('A1'), ix('B'), false); 'ok'"));
    assert_eq!(run(&mut s, &format!("{D}{FLAT}")), "E1,A,A2^A,A1,B");
    assert_eq!(run(&mut s, &format!("{D} d.currentLayerNum == ix('A1')")), "true");

    // A1 vor die unterste Ebene
    let mut s = layered();
    run(&mut s, &format!("{D} d.reorderLayer(ix('A1'), 0, false); 'ok'"));
    assert_eq!(run(&mut s, &format!("{D}{FLAT}")), "A1,E1,A,A2^A,B");
    assert_eq!(run(&mut s, &format!("{D} d.currentLayerNum")), "0");

    // A2 vor die eigene Elternebene A
    let mut s = layered();
    run(&mut s, &format!("{D} d.reorderLayer(ix('A2'), ix('A'), false); 'ok'"));
    assert_eq!(run(&mut s, &format!("{D}{FLAT}")), "E1,A2,A,A1^A,B");
    assert_eq!(run(&mut s, &format!("{D} d.currentLayerNum == ix('A2')")), "true");
}

// ------------------------------------------------------------------ BA-124

/// BA-124: Dokument mit Einheit Pixel/cm â€“ `resolution` bleibt, `resolutionUnits` = `cm`,
/// `pHYs` = trunc(px/cm)Â·100 in FW-PNG und PNG-Export (28,35 â†’ 2800, 50 â†’ 5000, 59,06 â†’ 5900).
#[test]
fn ba124_resolution_per_cm_kept_and_phys_truncated() {
    for (i, (ppcm, want)) in [(28.35, 2800u32), (50.0, 5000), (59.06, 5900)].into_iter().enumerate() {
        let fw = dir().join(format!("gb5_124_{i}.fw.png"));
        let png = dir().join(format!("gb5_124_{i}.png"));
        let r = result(&format!(
            "fw.createFirepixelDocument({{x: 20, y: 10}}, {{pixelsPerUnit: {ppcm}, units: 'cm'}}, '#ff0000'); var dd = fw.getDocumentDOM(); \
             var a = [Math.round(dd.resolution * 100) / 100, dd.resolutionUnits].join(','); \
             dd.exportTo('{}', {{exportFormat: 'PNG', colorMode: '32 bit'}}); fw.saveDocument(dd, '{}'); dd.close(false); \
             var d2 = fw.openDocument('{}'); var b = [Math.round(d2.resolution * 100) / 100, d2.resolutionUnits].join(','); d2.close(false); a + '|' + b",
            url(&png),
            url(&fw),
            url(&fw)
        ));
        assert_eq!(r, format!("{ppcm},cm|{ppcm},cm"));
        for p in [&fw, &png] {
            let bytes = std::fs::read(p).expect("Datei");
            assert_eq!(phys_x(&bytes), Some(want), "{} bei {ppcm} px/cm", p.display());
        }
    }
}

// ------------------------------------------------------------------ N1

/// N1: `dom.setExportOptions({exportFormat: "GIF", â€¦})` setzt wie im Original die
/// Exporteinstellungen des Dokuments (Verlaufsschritt, rÃ¼ckgÃ¤ngig machbar).
#[test]
fn n1_set_export_options_changes_document_export_settings() {
    fp_engine::shell::disable();
    fp_i18n::profile::use_test_profile();
    let mut s = fp_engine::Session::default();
    let r = run(
        &mut s,
        &format!("{NEW} var a = dom.exportOptions.exportFormat; dom.setExportOptions({{exportFormat: 'GIF', colorMode: 'indexed', paletteMode: 'web216', numEntriesRequested: 216}}); var e = dom.exportOptions; [a, e.exportFormat, e.colorMode, e.paletteMode, e.name].join('/')"),
    );
    assert_eq!(r, "PNG/GIF/indexed/web216/GIF Web 216");
    // JPEG ohne Farbmodus: 24 Bit, QualitÃ¤t Ã¼bernommen
    let r = run(&mut s, "var dom = fw.getDocumentDOM(); dom.setExportOptions({exportFormat: 'JPEG', jpegQuality: 60}); var e = dom.exportOptions; [e.exportFormat, e.colorMode, e.jpegQuality].join('/')");
    assert_eq!(r, "JPEG/24 bit/60");
    // ein Verlaufsschritt je Aufruf: RÃ¼ckgÃ¤ngig fÃ¼hrt zum GIF zurÃ¼ck
    s.execute("edit.undo", serde_json::json!({})).expect("undo");
    assert_eq!(run(&mut s, "fw.getDocumentDOM().exportOptions.exportFormat"), "GIF");
}
