//! B-7: Korrekturen nach den Messungen im Original (Bericht `docs\berichte\b2.md`), je Punkt ein
//! Regressionstest mit den dort gemessenen Werten des Originals.

use fp_script::{ScriptOptions, run_source};
use std::path::PathBuf;

fn dir() -> PathBuf {
    let d = std::env::temp_dir().join(format!("fp-b7-script-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&d);
    d
}

fn url(p: &std::path::Path) -> String {
    format!("file:///{}", p.to_string_lossy().replace('\\', "/"))
}

fn result(src: &str) -> String {
    let mut s = fp_engine::Session::default();
    let o = run_source(&mut s, src, &ScriptOptions::default());
    assert!(o.ok, "Fehler: {:?}\nDiagnose: {:?}", o.error, o.diagnostics);
    o.result.unwrap_or_default()
}

const NEW: &str = "fw.createFirepixelDocument({x: 600, y: 500}, {pixelsPerUnit: 72, units: 'inch'}, '#ffffff'); var dom = fw.getDocumentDOM();\n";

/// `pHYs` (Pixel je Meter x) einer PNG-Datei; `None` ohne Chunk.
fn phys_x(bytes: &[u8]) -> Option<u32> {
    let chunks = fp_fwpng::png::read_chunks(bytes).expect("PNG");
    chunks
        .iter()
        .find(|c| c.is(b"pHYs"))
        .map(|c| u32::from_be_bytes([c.data[0], c.data[1], c.data[2], c.data[3]]))
}

/// BA-009: Beim Einpassen in ein Zielrechteck rundet das Original die skalierte Lage
/// (150 + 10 · 0,75 = 157,5 → 158): 320,158–370,196.
#[test]
fn ba009_import_into_rect_rounds_position() {
    // 100 × 80, deckendes Quadrat 50 × 50 bei (20, 10), Rest durchsichtig (b2_rand.png)
    let (w, h) = (100usize, 80usize);
    let mut px = vec![0u8; w * h * 4];
    for y in 10..60 {
        for x in 20..70 {
            px[(y * w + x) * 4..(y * w + x) * 4 + 4].copy_from_slice(&[200, 30, 30, 255]);
        }
    }
    let p = dir().join("b7_rand.png");
    {
        let f = std::fs::File::create(&p).expect("Datei");
        let mut enc = png::Encoder::new(std::io::BufWriter::new(f), w as u32, h as u32);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.write_header()
            .expect("Kopf")
            .write_image_data(&px)
            .expect("Pixel");
    }
    let r = result(&format!(
        "{NEW} dom.importFile('{}', {{left: 300, top: 150, right: 400, bottom: 210}}, false); var e = fw.selection[0]; [e.left, e.top, e.left + e.width, e.top + e.height].join(',')",
        url(&p)
    ));
    assert_eq!(r, "320,158,370,196");
}

/// BA-014: Ein Filter per Skript auf einem Vektor wandelt ihn ohne Rückfrage in ein Bitmap um
/// (Typ Image, Grenzen gleich), wie das Original.
#[test]
fn ba014_script_filter_rasterizes_vectors() {
    for f in [
        "dom.filterSelectionByName('Adjust Color', 'Invert')",
        "dom.filterSelectionByName('Weichzeichnen', 'Weichzeichnen')",
    ] {
        let r = result(&format!(
            "{NEW} dom.addNewRectanglePrimitive({{left: 20, top: 20, right: 120, bottom: 100}}, 0); dom.setFillColor('#3060a0'); {f}; \
             var e = fw.selection[0]; [String(e), dom.layers[0].frames[0].elements.length, e.left, e.top, e.width, e.height].join(',')"
        ));
        assert_eq!(r, "[object Image],1,20,20,100,80", "{f}");
    }
    // Bitmap bleibt Bitmap, kein zusätzlicher Schritt
    let r = result(&format!(
        "{NEW} dom.addNewRectanglePrimitive({{left: 20, top: 20, right: 120, bottom: 100}}, 0); dom.flattenSelection(); dom.filterSelectionByName('Adjust Color', 'Invert'); \
         var e = fw.selection[0]; [String(e), e.left, e.top, e.width, e.height].join(',')"
    ));
    assert_eq!(r, "[object Image],20,20,100,80");
}

/// BA-017: Rahmen „roh“ eines Texts am Pfad = linke obere Ecke und Breite der Pfadgrenzen (Original:
/// Linie 50,400 → 400,400 ergibt raw 50,400,350×5), auch nach Speichern und Wiederöffnen.
#[test]
fn ba017_raw_rect_of_text_on_path_is_path_rect() {
    let out = dir().join("b7_017.fw.png");
    let raw =
        "function raw(t) { return [t.rawLeft, t.rawTop, t.rawWidth, t.rawHeight].join(','); }\n";
    let r = result(&format!(
        "{NEW}{raw} dom.addNewText({{left: 60, top: 150, right: 200, bottom: 170}}, true); dom.setTextRuns({{initialAttrs: {{size: '14pt', face: 'ArialMT', alignment: 'left'}}, textRuns: [{{changedAttrs: {{}}, characters: 'Hallo Pfad'}}]}}); \
         var t = fw.selection[0]; dom.addNewLine({{x: 50, y: 400}}, {{x: 400, y: 400}}); var p = fw.selection[0]; fw.selection = [p, t]; dom.attachTextToPath(); \
         var a = raw(fw.selection[0]); fw.saveDocument(dom, '{u}'); dom.close(false); fw.openDocument('{u}'); var d2 = fw.getDocumentDOM(); \
         var L = d2.layers[0].frames[0].elements, b = ''; for (var i = 0; i < L.length; i++) if (L[i].rawLeft !== undefined) b = raw(L[i]); a + '|' + b",
        u = url(&out)
    ));
    assert_eq!(r, "50,400,350,5|50,400,350,5");
}

/// BA-020: `convertToAnimSymbol` – die Grenzen der Animationsinstanz schließen den Bewegungsweg ein
/// (20,20,120,100 mit Weg 62/−36 → 20,−16,182,100); es entstehen keine neuen Status.
#[test]
fn ba020_anim_instance_bounds_include_motion_path() {
    let r = result(&format!(
        "{NEW} dom.addNewRectanglePrimitive({{left: 20, top: 20, right: 120, bottom: 100}}, 0); \
         dom.convertToAnimSymbol('AnimS', 3, {{x: 62, y: -36}}, 0, 100, 100, 100); var e = fw.selection[0]; \
         [e.left, e.top, e.left + e.width, e.top + e.height, dom.frames.length].join(',')"
    ));
    assert_eq!(r, "20,-16,182,100,1");
}

/// BA-033: Neue Dokumente und neue Seiten exportieren wie im Original als PNG 32 Bit.
#[test]
fn ba033_new_document_and_page_export_png32() {
    let r = result(&format!(
        "{NEW} var e = dom.exportOptions; var a = e.exportFormat + '/' + e.colorMode; dom.addNewPage(); var e2 = fw.getDocumentDOM().exportOptions; a + '|' + e2.exportFormat + '/' + e2.colorMode"
    ));
    assert_eq!(r, "PNG/32 bit|PNG/32 bit");
}

/// BA-035: `pHYs` = floor(trunc(dpi) / 0,0254) in FW-PNG und PNG-Export (auch per JSF `exportTo`
/// und `fw.exportDocumentAs`): 72 → 2834, 96 → 3779, 150,6 → 5905, 299,7 → 11771.
#[test]
fn ba035_phys_truncates_dpi_like_original() {
    for (i, (dpi, want)) in [(72.0, 2834u32), (96.0, 3779), (150.6, 5905), (299.7, 11771)]
        .into_iter()
        .enumerate()
    {
        let png = dir().join(format!("b7_035_{i}.png"));
        let png2 = dir().join(format!("b7_035_{i}b.png"));
        let fw = dir().join(format!("b7_035_{i}.fw.png"));
        result(&format!(
            "fw.createFirepixelDocument({{x: 20, y: 10}}, {{pixelsPerUnit: {dpi}, units: 'inch'}}, '#ff0000'); var dd = fw.getDocumentDOM(); \
             dd.exportTo('{}', {{exportFormat: 'PNG', colorMode: '32 bit'}}); var e = dd.exportOptions; e.exportFormat = 'PNG'; e.colorMode = '32 bit'; \
             fw.exportDocumentAs(dd, '{}', e); fw.saveDocument(dd, '{}'); dd.close(false); 'ok'",
            url(&png),
            url(&png2),
            url(&fw)
        ));
        for p in [&png, &png2, &fw] {
            assert_eq!(
                phys_x(&std::fs::read(p).expect("Datei")),
                Some(want),
                "{dpi} dpi, {}",
                p.display()
            );
        }
    }
    assert_eq!(fp_fwpng::png::phys_ppm(254.0), 10000);
}
