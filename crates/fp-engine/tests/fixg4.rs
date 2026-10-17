//! FIX-G4 (TWEAKS TW-05): `pixelRect` von Pfaden mit Strich wie im Original
//! (`re\_testdateien\par\run\menu-edit\log.jsonl` u. a.): Strichkante auf die nächste Pixelgrenze,
//! offene Pfade mit ungerader Strichstärke reichen rechts/unten 1 px weiter. Fehlt die Testdatei
//! (anderer Rechner), wird der Fall übersprungen.

use crate::common::*;
use serde_json::json;

#[test]
fn path_pixel_rect_with_1px_stroke_like_original() {
    let src = std::path::Path::new(r"C:\Users\Administrator\Desktop\REA\re\_testdateien\par\par_vector.fw.png");
    if !src.is_file() {
        eprintln!("übersprungen: {src:?} fehlt");
        return;
    }
    let p = std::env::temp_dir().join(format!("fp-fixg4-{}-par_vector.fw.png", std::process::id()));
    std::fs::copy(src, &p).expect("Kopie");
    let mut s = doc_session(10.0, 10.0);
    ex(&mut s, "file.open", json!({"path": p.to_string_lossy()}));
    let d = s.active().expect("Dokument");
    let mut got = Vec::new();
    for (_, l) in fp_engine::docops::own_layers(&d.doc, 0) {
        for cel in &l.cels {
            for e in &cel.elements {
                let r = fp_engine::docops::element_pixel_rect(&d.doc, e).expect("pixelRect");
                got.push((e.common.name.clone().unwrap_or_default(), [r.left, r.top, r.right, r.bottom]));
            }
        }
    }
    let _ = std::fs::remove_file(&p);
    // Sollwerte des Originals (menu-edit/02_redo_last_result)
    for (name, want) in [
        ("L1", [50.0, 300.0, 301.0, 361.0]),
        ("ST", [654.0, 205.0, 846.0, 386.0]),
        ("PG", [465.0, 165.0, 535.0, 245.0]),
        ("P1", [200.0, 170.0, 421.0, 241.0]),
        ("E1", [50.0, 170.0, 150.0, 240.0]),
        ("R2", [200.0, 50.0, 300.0, 120.0]),
        ("R1", [50.0, 50.0, 150.0, 120.0]),
    ] {
        let r = got.iter().find(|(n, _)| n == name).map(|(_, r)| *r);
        assert_eq!(r, Some(want), "{name}: {got:?}");
    }
}
