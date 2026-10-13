//! BM-1 (Nutzerbefund N26, re\22 §1): `pixelRect` einer Bitmap meldet die sichtbaren Pixel
//! (Alpha ≥ 2), `left/top/width/height` und `getSelectionBounds()` die Objektgrenzen; `importFile`
//! beschneidet wie das Original.

use fp_script::{run_source, ScriptOptions};
use std::path::PathBuf;

fn dir() -> PathBuf {
    let d = std::env::temp_dir().join(format!("fp-bm1-script-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&d);
    d
}

/// 400×300 RGBA, rotes Quadrat x 100…200, y 50…150, Rest transparent (re5a_t1_rand).
fn t1_rgba() -> Vec<u8> {
    let (w, h) = (400usize, 300usize);
    let mut v = vec![0u8; w * h * 4];
    for y in 50..150 {
        for x in 100..200 {
            let o = (y * w + x) * 4;
            v[o..o + 4].copy_from_slice(&[255, 0, 0, 255]);
        }
    }
    v
}

fn write_png(name: &str, w: u32, h: u32, rgba: &[u8]) -> PathBuf {
    let p = dir().join(name);
    let f = std::fs::File::create(&p).expect("Datei");
    let mut enc = png::Encoder::new(std::io::BufWriter::new(f), w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().expect("Kopf").write_image_data(rgba).expect("Pixel");
    p
}

/// FW-PNG mit einer Bitmap über die ganze Leinwand (wie ältere Firepixel-Dateien).
fn full_fwpng() -> PathBuf {
    let r = fp_io::RasterImage::from_rgba(400, 300, t1_rgba()).expect("Bild");
    let doc = fp_io::build::document_from_raster(r, fp_io::FileFormat::Png);
    let bytes = fp_fwpng::write(&doc, &fp_fwpng::WriteOptions::default(), &fp_fwpng::NoFlattener).expect("FW-PNG");
    let p = dir().join("t1_full.fw.png");
    std::fs::write(&p, bytes).expect("schreiben");
    p
}

fn result(src: &str) -> String {
    let mut s = fp_engine::Session::default();
    let o = run_source(&mut s, src, &ScriptOptions::default());
    assert!(o.ok, "Fehler: {:?}\nDiagnose: {:?}", o.error, o.diagnostics);
    o.result.unwrap_or_default()
}

fn url(p: &PathBuf) -> String {
    format!("file:///{}", p.to_string_lossy().replace('\\', "/"))
}

const INFO: &str = "function info(){ var dom = fw.getDocumentDOM(); dom.selectAll(); var e = fw.selection[0]; var r = e.pixelRect; var s = dom.getSelectionBounds(); \
    return [e.left, e.top, e.width, e.height, '|', r.left, r.top, r.right, r.bottom, '|', s.left, s.top, s.right, s.bottom].join(','); }\n";

#[test]
fn pixel_rect_of_full_canvas_bitmap_shows_visible_pixels() {
    let p = full_fwpng();
    let r = result(&format!("{INFO} fw.openDocument('{}'); info()", url(&p)));
    assert_eq!(r, "0,0,400,300,|,100,50,200,150,|,0,0,400,300");
    // 50× lesen ändert nichts
    let r = result(&format!("{INFO} fw.openDocument('{}'); var x; for (var i = 0; i < 50; i++) x = info(); x", url(&p)));
    assert_eq!(r, "0,0,400,300,|,100,50,200,150,|,0,0,400,300");
}

#[test]
fn opened_png_is_trimmed_everywhere() {
    let p = write_png("t1.png", 400, 300, &t1_rgba());
    let r = result(&format!("{INFO} fw.openDocument('{}'); info()", url(&p)));
    assert_eq!(r, "100,50,100,100,|,100,50,200,150,|,100,50,200,150");
}

#[test]
fn empty_bitmap_pixel_rect_is_huge() {
    let p = write_png("t4.png", 200, 100, &[0u8; 200 * 100 * 4]);
    let r = result(&format!("fw.openDocument('{}'); var dom = fw.getDocumentDOM(); dom.selectAll(); var r = fw.selection[0].pixelRect; [r.left, r.top, r.right, r.bottom].join(',')", url(&p)));
    assert_eq!(r, "-2147483648,-2147483648,2147483648,2147483648");
}

#[test]
fn import_file_trims_and_keeps_offset() {
    let p = write_png("imp.png", 400, 300, &t1_rgba());
    let new = "var dom = fw.createFirepixelDocument({x: 500, y: 500}, {pixelsPerUnit: 72, units: 'inch'}, '#ffffff'); dom = fw.getDocumentDOM();\n";
    let r = result(&format!("{new} dom.importFile('{}', {{left: 20, top: 30, right: 20, bottom: 30}}, false); var e = fw.selection[0]; [e.left, e.top, e.width, e.height].join(',')", url(&p)));
    assert_eq!(r, "120,80,100,100");
    // ganz transparent: kein Objekt
    let e = write_png("imp_empty.png", 200, 100, &[0u8; 200 * 100 * 4]);
    let r = result(&format!("{new} dom.importFile('{}', {{left: 20, top: 30, right: 20, bottom: 30}}, false); String(dom.frames[0].layers[dom.currentLayerNum].elements.length)", url(&e)));
    assert_eq!(r, "0");
}
