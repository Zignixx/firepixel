//! RT-1: per Code-Analyse geklärte Treue-Punkte (re 27): L2 Fang beim Verschieben,
//! L3 Radieren ändert die Objektgrenzen nicht (Beschnitt erst beim Kopieren, Konvertieren,
//! Zusammenfügen), L8 Menüfilter beschneidet die Bitmap, D6 Freigaben der gespeicherten
//! Bitmap-Auswahl über alle offenen Dokumente, P3 Anpassung des Vektorpfad-Zugs.

use crate::common;
use common::*;
use fp_engine::tools::ToolId;
use fp_engine::{Session, ToolEvent};
use fp_geom::{Point, Rect};
use fp_model::ElementKind;
use serde_json::json;
use std::path::PathBuf;

fn dir() -> PathBuf {
    let d = std::env::temp_dir().join(format!("fp-rt1-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&d);
    d
}

fn tool(s: &mut Session, t: ToolId) {
    s.set_tool(t).expect("Werkzeug");
}

fn zoom1(s: &mut Session) {
    ex(s, "view.setZoom", json!({"zoom": 1.0}));
}

fn history(s: &Session) -> usize {
    s.active().expect("d").history.undo_count()
}

// ------------------------------------------------------------------ L2

/// L2: beim Verschieben rastet die linke Kante auch an der **rechten** Kante eines anderen Objekts
/// (entgegengesetzte Kanten), 4 px daneben → bündig.
#[test]
fn l2_move_snaps_opposite_edges() {
    let mut s = doc_session(400.0, 300.0);
    zoom1(&mut s);
    rect(&mut s, 100.0, 100.0, 210.0, 160.0);
    let r2 = rect(&mut s, 300.0, 230.0, 350.0, 260.0);
    select(&mut s, &[r2]);
    tool(&mut s, ToolId::Pointer);
    // linke Kante 300 → 214 (4 px rechts der rechten Kante 210 von R1), y fern jeder Linie
    events(&mut s, &[ToolEvent::down(325.0, 245.0), ToolEvent::drag(280.0, 238.0), ToolEvent::drag(239.0, 237.0), ToolEvent::up(239.0, 237.0)]);
    let b = bounds(&s, r2);
    assert_eq!((b.left, b.right), (210.0, 260.0), "linke an rechte Kante: {b:?}");
    // rechte Kante 4 px vor die linke Kante 100 von R1
    let cy = (b.top + b.bottom) / 2.0;
    events(&mut s, &[ToolEvent::down(235.0, cy), ToolEvent::drag(150.0, cy), ToolEvent::drag(71.0, cy), ToolEvent::up(71.0, cy)]);
    let b = bounds(&s, r2);
    assert_eq!((b.left, b.right), (50.0, 100.0), "rechte an linke Kante: {b:?}");
}

// ------------------------------------------------------------------ L3 / L8

/// PNG 400 × 300 mit deckendem Rot 100,50–200,150 als FW-PNG (Bitmap 0,0–400,300) öffnen und wählen.
fn rand_bitmap(name: &str) -> (Session, fp_model::ElementId) {
    let (w, h) = (400usize, 300usize);
    let px: Vec<u32> = (0..w * h).map(|i| if (100..200).contains(&(i % w)) && (50..150).contains(&(i / w)) { 0xffff_0000 } else { 0 }).collect();
    let bytes = fp_export::png_out::write_png_truecolor(&px, w, h, true, &fp_export::png_out::PngOptions::default()).expect("PNG");
    let r = fp_io::png_in::decode_png(&bytes).expect("PNG lesen");
    let doc = fp_io::build::document_from_raster(r, fp_io::FileFormat::Png);
    let fw = fp_fwpng::write(&doc, &fp_fwpng::WriteOptions::default(), &fp_fwpng::NoFlattener).expect("FW-PNG");
    let p = dir().join(format!("{name}.fw.png"));
    std::fs::write(&p, fw).expect("schreiben");
    let mut s = doc_session(10.0, 10.0);
    ex(&mut s, "file.open", json!({"path": p.to_string_lossy()}));
    let t = top_elements(&s);
    assert_eq!(t.len(), 1);
    let id = t[0].id;
    assert_eq!(bounds(&s, id), Rect::new(0.0, 0.0, 400.0, 300.0));
    select(&mut s, &[id]);
    (s, id)
}

/// L3: Radieren (Pixel löschen) bis an den Rand ändert die Objektgrenzen nicht, solange der
/// Bitmap-Modus läuft (kein Esc, kein Zeigerklick, kein Werkzeugwechsel); die Kopie in der
/// Zwischenablage wird auf die deckenden Pixel beschnitten. Beschnitt beim Verlassen des Modus: RT-3 (re 28 L3,
/// `rt3_messungen::l3_*`).
#[test]
fn l3_erasing_keeps_bounds_copy_is_trimmed() {
    let (mut s, id) = rand_bitmap("l3");
    ex(&mut s, "select.pixels", json!({"shape": "rect", "rect": [100, 50, 150, 150]}));
    ex(&mut s, "edit.clear", json!({}));
    ex(&mut s, "select.none", json!({}));
    select(&mut s, &[id]);
    assert_eq!(bounds(&s, id), Rect::new(0.0, 0.0, 400.0, 300.0), "Objektgrenzen bleiben");
    let d = s.active().expect("d");
    assert_eq!(fp_engine::docops::element_pixel_rect(&d.doc, &el(&s, id)), Some(Rect::new(150.0, 50.0, 200.0, 150.0)), "pixelRect folgt den Pixeln");
    // Kopieren/Einfügen: die Kopie ist beschnitten, das Original bleibt
    ex(&mut s, "edit.copy", json!({}));
    assert_eq!(bounds(&s, id), Rect::new(0.0, 0.0, 400.0, 300.0), "Original bleibt beim Kopieren");
    let v = ex(&mut s, "edit.paste", json!({}));
    let cid = fp_model::ElementId(v["ids"][0].as_u64().expect("id"));
    assert_ne!(cid, id);
    assert_eq!(bounds(&s, cid), Rect::new(150.0, 50.0, 200.0, 150.0), "Kopie 150,50,50,100");
    assert_eq!(bounds(&s, id), Rect::new(0.0, 0.0, 400.0, 300.0), "Original bleibt beim Einfügen");
    // Radiergummi-Zug: der Zug selbst beschneidet nicht (re 28 L3); der Werkzeugwechsel beendet
    // den Malmodus und beschneidet (re 37 §5a, K-6 – vorher erwartete dieser Test, dass das
    // Original nach dem Wechsel zum Zeiger unbeschnitten bleibt)
    select(&mut s, &[id]);
    tool(&mut s, ToolId::Eraser);
    events(&mut s, &[ToolEvent::down(190.0, 60.0), ToolEvent::drag(195.0, 60.0), ToolEvent::up(195.0, 60.0)]);
    assert_eq!(bounds(&s, id), Rect::new(0.0, 0.0, 400.0, 300.0), "Radierzug beschneidet nicht");
    let px = {
        let d = s.active().expect("d");
        fp_engine::docops::element_pixel_rect(&d.doc, &el(&s, id)).expect("Pixel")
    };
    tool(&mut s, ToolId::Pointer);
    assert_eq!(bounds(&s, id), px, "Werkzeugwechsel beschneidet");
}

/// L8: Filter ▸ Farbe anpassen ▸ Umkehren an einer Bitmap mit durchsichtigem Rand beschneidet auf
/// die deckenden Pixel (Alpha ≥ 2), in genau einem Verlaufsschritt; Rückgängig stellt die
/// Grenzen wieder her.
#[test]
fn l8_menu_filter_trims_bitmap_in_one_step() {
    let (mut s, id) = rand_bitmap("l8");
    let n = history(&s);
    ex(&mut s, "filter.adjustColor.invert", json!({}));
    assert_eq!(history(&s), n + 1, "ein Verlaufsschritt");
    assert_eq!(bounds(&s, id), Rect::new(100.0, 50.0, 200.0, 150.0), "left/width 100/100");
    let e = el(&s, id);
    let ElementKind::Bitmap(b) = &e.kind else { panic!("Bitmap") };
    assert_eq!(b.image.pixel(0, 0).map(|c| (c.r, c.g, c.b, c.a)), Some((0, 255, 255, 255)), "umgekehrt");
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(bounds(&s, id), Rect::new(0.0, 0.0, 400.0, 300.0));
    // zweimal hintereinander: zweiter Lauf ändert die Grenzen nicht mehr
    ex(&mut s, "edit.redo", json!({}));
    ex(&mut s, "filter.adjustColor.invert", json!({}));
    assert_eq!(bounds(&s, id), Rect::new(100.0, 50.0, 200.0, 150.0));
}

/// L8: Filter nur in einer Pixelauswahl beschneidet nicht (Objektgrenzen bleiben).
#[test]
fn l8_filter_in_pixel_selection_keeps_bounds() {
    let (mut s, id) = rand_bitmap("l8m");
    ex(&mut s, "select.pixels", json!({"shape": "rect", "rect": [100, 50, 150, 150]}));
    ex(&mut s, "filter.adjustColor.invert", json!({}));
    assert_eq!(bounds(&s, id), Rect::new(0.0, 0.0, 400.0, 300.0));
}

/// L3: Zusammenfügen nach unten beschneidet das Ergebnis auf die deckenden Pixel.
#[test]
fn l3_merge_down_trims() {
    let (mut s, id) = rand_bitmap("l3m");
    let r = rect(&mut s, 120.0, 70.0, 140.0, 90.0);
    select(&mut s, &[r]);
    let v = ex(&mut s, "modify.mergeDown", json!({}));
    let nid = fp_model::ElementId(v["id"].as_u64().expect("id"));
    let _ = id;
    assert_eq!(bounds(&s, nid), Rect::new(100.0, 50.0, 200.0, 150.0));
}

// ------------------------------------------------------------------ D6

/// D6: „Bitmap-Auswahl wiederherstellen/löschen“ sind aktiv, sobald irgendein offenes Dokument eine
/// gespeicherte Auswahl hat, auch ein Dokument anderer Größe.
#[test]
fn d6_restore_and_delete_enabled_by_any_open_document() {
    let mut s = doc_session(300.0, 200.0);
    bitmap(&mut s, 10, 10, 50, 50, [255, 0, 0, 255]);
    ex(&mut s, "select.pixels", json!({"shape": "rect", "rect": [10, 10, 30, 30]}));
    let avail = |s: &Session, n: &str| s.is_available(n);
    assert!(!avail(&s, "select.restoreBitmapSelection"));
    assert!(!avail(&s, "select.deleteBitmapSelection"));
    ex(&mut s, "select.saveBitmapSelection", json!({"name": "A"}));
    assert!(avail(&s, "select.restoreBitmapSelection"));
    // zweites Dokument anderer Größe ohne eigene gespeicherte Auswahl
    ex(&mut s, "file.new", json!({"width": 120, "height": 80, "background": "#ffffff"}));
    // re 29 §10 (T-3): wiederherstellen nur mit Auswahlrahmen im aktiven Dokument
    assert!(!avail(&s, "select.restoreBitmapSelection"), "andere Größe, kein Rahmen: grau");
    assert!(avail(&s, "select.deleteBitmapSelection"));
}

// ------------------------------------------------------------------ P3

fn wave() -> Vec<ToolEvent> {
    let mut evs = vec![ToolEvent::down(20.0, 100.0)];
    for i in 1..=120 {
        let x = 20.0 + i as f32 * 2.0;
        evs.push(ToolEvent::drag(x, 100.0 + ((x - 20.0) / 40.0).sin() * 30.0));
    }
    evs.push(ToolEvent::up(260.0, 100.0 + (240.0f32 / 40.0).sin() * 30.0));
    evs
}

fn vector_path_nodes(precision: i64) -> usize {
    let mut s = doc_session(300.0, 200.0);
    s.tools.options.path_precision = precision as u32;
    tool(&mut s, ToolId::VectorPath);
    events(&mut s, &wave());
    let t = top_elements(&s);
    let ElementKind::Path(p) = &t[0].kind else { panic!("Pfad") };
    p.contours[0].nodes.len()
}

/// P3: Der Vektorpfad-Zug wird nach dem Loslassen mit Toleranz 10 − Präzision angepasst: niedrige
/// Präzision (große Toleranz) gibt weniger Knoten; Präzision 10 passt nicht an.
#[test]
fn p3_vector_path_refit_tolerance_follows_precision() {
    let rough = vector_path_nodes(1);
    let mid = vector_path_nodes(5);
    let exact = vector_path_nodes(10);
    assert!(rough >= 2 && rough <= mid && mid <= exact, "Knoten {rough}/{mid}/{exact}");
    assert!(rough < exact, "Anpassung wirkt: {rough} < {exact}");
}

/// P3: Anpassung direkt: Ecken bleiben, Präzision 10 lässt die Kontur unverändert.
#[test]
fn p3_refit_keeps_corners_and_skips_at_precision_10() {
    let geo = fp_engine::geometry::default_geometry();
    let pts: Vec<Point> = (0..=60).map(|i| Point::new(i as f32 * 2.0 + 0.5, 50.0 + ((i as f32) / 6.0).sin() * 20.0)).chain((1..=20).map(|i| Point::new(120.5, 50.0 + i as f32 * 4.0))).collect();
    let c = fp_engine::tools::freehand_contour(&pts, 5, false).expect("Kontur");
    let same = fp_engine::tools::refit_freehand(c.clone(), 10, geo.as_ref());
    assert_eq!(same, c, "Präzision 10: keine Anpassung");
    let r = fp_engine::tools::refit_freehand(c.clone(), 1, geo.as_ref());
    assert!(r.nodes.len() <= c.nodes.len(), "{} ≤ {}", r.nodes.len(), c.nodes.len());
    assert_eq!(r.nodes.first().map(|n| n.anchor), c.nodes.first().map(|n| n.anchor), "Anfang bleibt");
    assert_eq!(r.nodes.last().map(|n| n.anchor), c.nodes.last().map(|n| n.anchor), "Ende bleibt");
}
