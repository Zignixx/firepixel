//! F20 / TW-68: Ausrichten-Bedienfeld (`pi.align`) auf ganze Pixel wie das Original. Sollwerte:
//! `pixelRect` des Originals aus Lauf 4 (`re\_testdateien\par\run_lauf4_2026-10-15\panel-info-align-path\log.jsonl`,
//! Schritte `06_align_buttons_*`), Auswahl R1, E1, PG aus `par_vector.fw.png`.
//!
//! Abgeleitete Regel (PG ist ein Sechseck 465.36…534.64 × 165…245):
//! * Ausrichten (links/Mitte/rechts, oben/Mitte/unten): Bezug und Objekte als `pixelRect`, Versatz
//!   ungerundet (hcenter: E1 242.5 → 242…343, PG → 257…328).
//! * Verteilen: genaue Grenzen, Versatz auf ganze Pixel, ½ nach oben (distribute_bottom: −57.5 → −57).
//! * Gleichmäßiger Abstand: erstes und letztes Objekt bleiben, die übrigen auf ganze Pixel (space_v).
//! * Angleichen: Zielrechteck ganzzahlig (size_height: PG → 465…534).
//!
//! Fälle mit Gleichstand (TW-67, Sortierregel offen) sind hier bewusst nicht dabei. Fehlt die
//! Testdatei (anderer Rechner), wird der Fall übersprungen.

use crate::common::*;
use fp_model::ElementId;
use serde_json::json;

fn open_vector() -> Option<fp_engine::Session> {
    let src = std::path::Path::new(r"C:\Users\Administrator\Desktop\REA\re\_testdateien\par\par_vector.fw.png");
    if !src.is_file() {
        eprintln!("übersprungen: {src:?} fehlt");
        return None;
    }
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(1);
    let p = std::env::temp_dir().join(format!("fp-f20-{}-{}-par_vector.fw.png", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
    std::fs::copy(src, &p).ok()?;
    let mut s = doc_session(10.0, 10.0);
    ex(&mut s, "file.open", json!({"path": p.to_string_lossy()}));
    let _ = std::fs::remove_file(&p);
    Some(s)
}

fn id_of(s: &fp_engine::Session, name: &str) -> ElementId {
    let d = s.active().expect("Dokument");
    for (_, l) in fp_engine::docops::own_layers(&d.doc, 0) {
        for cel in &l.cels {
            for e in &cel.elements {
                if e.common.name.as_deref() == Some(name) {
                    return e.id;
                }
            }
        }
    }
    panic!("{name} fehlt")
}

fn pixel_rects(s: &fp_engine::Session, names: &[&str]) -> String {
    let d = s.active().expect("Dokument");
    names
        .iter()
        .map(|n| {
            let id = id_of(s, n);
            let e = fp_engine::docops::element_on_page(&d.doc, d.page, id).expect("Element");
            let r = fp_engine::docops::element_pixel_rect(&d.doc, &e).expect("pixelRect");
            format!("{n}:{},{},{},{}", r.left, r.top, r.right, r.bottom)
        })
        .collect::<Vec<_>>()
        .join("|")
}

#[test]
fn align_panel_whole_pixels_like_original_run4() {
    // (Modus, Sollwert des Originals in der Reihenfolge PG|E1|R1)
    let cases = [
        ("left", "PG:50,165,120,245|E1:50,170,150,240|R1:50,50,150,120"),
        ("hcenter", "PG:257,165,328,245|E1:242,170,343,240|R1:242,50,343,120"),
        ("right", "PG:465,165,535,245|E1:435,170,535,240|R1:435,50,535,120"),
        ("top", "PG:465,50,535,130|E1:50,50,150,120|R1:50,50,150,120"),
        ("vcenter", "PG:465,107,535,188|E1:50,112,150,183|R1:50,112,150,183"),
        ("bottom", "PG:465,165,535,245|E1:50,175,150,245|R1:50,175,150,245"),
        ("distTop", "PG:465,110,535,190|E1:50,170,150,240|R1:50,50,150,120"),
        ("distBottom", "PG:465,165,535,245|E1:50,113,150,183|R1:50,50,150,120"),
        ("matchWidth", "PG:465,165,565,245|E1:50,170,150,240|R1:50,50,150,120"),
        ("matchHeight", "PG:465,165,534,245|E1:50,170,150,250|R1:50,50,150,130"),
        ("matchBoth", "PG:465,165,565,245|E1:50,170,150,250|R1:50,50,150,130"),
        ("spaceV", "PG:465,108,535,188|E1:50,170,150,240|R1:50,50,150,120"),
    ];
    let mut bad = Vec::new();
    for (mode, want) in cases {
        let Some(mut s) = open_vector() else { return };
        let ids = [id_of(&s, "PG"), id_of(&s, "E1"), id_of(&s, "R1")];
        select(&mut s, &ids);
        ex(&mut s, "pi.align", json!({"mode": mode}));
        let got = pixel_rects(&s, &["PG", "E1", "R1"]);
        if got != want {
            bad.push(format!("{mode}: {got} statt {want}"));
        }
    }
    assert!(bad.is_empty(), "{bad:#?}");
}

/// F22 / TW-79: Die Rauschgrenze gilt nur an rechter/unterer Kante. Links/oben rundet das Original
/// eine Kante knapp unter der Pixelgrenze nach außen (9.9995 → 9), rechts/unten nicht (90.0002 → 90).
#[test]
fn pixel_rect_tolerance_only_right_bottom() {
    let mut s = doc_session(200.0, 200.0);
    let id = rect(&mut s, 9.9995, 9.9995, 90.0002, 90.0002);
    let d = s.active().expect("Dokument");
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, id).expect("Element");
    let r = fp_engine::docops::element_pixel_rect(&d.doc, &e).expect("pixelRect");
    assert_eq!((r.left, r.top, r.right, r.bottom), (9.0, 9.0, 90.0, 90.0));
}

/// F22 / TW-79 mit der Datei aus `xs-file-formats/05`: Pfad „Pfad“ des PSD-Beispiels hat im
/// Original `pixelRect` 9,9–90,90 (Lauf 7 und Lauf 8). Fehlt die Datei, wird der Fall übersprungen.
#[test]
fn psd_path_pixel_rect_like_original() {
    let src = std::path::Path::new(r"C:\Users\Administrator\Desktop\REA\re\_testdateien\par\par_import\par.psd");
    if !src.is_file() {
        eprintln!("übersprungen: {src:?} fehlt");
        return;
    }
    let p = std::env::temp_dir().join(format!("fp-f22-{}-par.psd", std::process::id()));
    std::fs::copy(src, &p).expect("Kopie");
    let mut s = doc_session(10.0, 10.0);
    ex(&mut s, "file.open", json!({"path": p.to_string_lossy()}));
    let _ = std::fs::remove_file(&p);
    // der Pfad liegt in der Unterebene „Pfad“ und trägt selbst keinen Namen, daher nach der Art gesucht
    fn find_path(l: &fp_model::Layer) -> Option<std::sync::Arc<fp_model::Element>> {
        l.cels.iter().flat_map(|c| c.elements.iter()).find_map(|e| match &e.kind {
            fp_model::ElementKind::Path(_) => Some(e.clone()),
            fp_model::ElementKind::SubLayer(sub) => find_path(sub),
            _ => None,
        })
    }
    let d = s.active().expect("Dokument");
    let e = fp_engine::docops::own_layers(&d.doc, 0).into_iter().find_map(|(_, l)| find_path(&l)).expect("Pfad");
    let r = fp_engine::docops::element_pixel_rect(&d.doc, &e).expect("pixelRect");
    assert_eq!((r.left, r.top, r.right, r.bottom), (9.0, 9.0, 90.0, 90.0));
}
