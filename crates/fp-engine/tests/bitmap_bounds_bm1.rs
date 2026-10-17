//! BM-1 (Nutzerbefund N26): Bitmap-Grenzen wie im Original (re\22 §1).
//! Öffnen/Importieren einer PNG beschneidet das Objekt auf die sichtbaren Pixel (Alpha ≥ 2), FW-PNG
//! behält gespeicherte Grenzen, der Auswahlrahmen zeigt immer die sichtbaren Pixel.

use crate::common;
use common::*;
use fp_engine::tools::ToolId;
use fp_engine::{Session, ToolEvent};
use fp_geom::Rect;
use fp_model::ElementKind;
use serde_json::json;
use std::path::PathBuf;

fn dir() -> PathBuf {
    let d = std::env::temp_dir().join(format!("fp-bm1-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&d);
    d
}

/// PNG32 aus ARGB-Pixeln (`f(x, y)` = 0xAARRGGBB).
fn png(name: &str, w: usize, h: usize, f: impl Fn(usize, usize) -> u32) -> PathBuf {
    let px: Vec<u32> = (0..w * h).map(|i| f(i % w, i / w)).collect();
    let bytes = fp_export::png_out::write_png_truecolor(&px, w, h, true, &fp_export::png_out::PngOptions::default()).expect("PNG");
    let p = dir().join(name);
    std::fs::write(&p, bytes).expect("schreiben");
    p
}

/// re5a_t1_rand: 400×300, rotes Quadrat x 100…200, y 50…150; dazu `extra` bei (10,10) mit dieser
/// Deckkraft und ein Pixel mit Farbe, aber Alpha 0, bei (390,290).
fn t1(name: &str, extra: u8) -> PathBuf {
    png(name, 400, 300, move |x, y| {
        if (100..200).contains(&x) && (50..150).contains(&y) {
            0xffff_0000
        } else if (x, y) == (10, 10) {
            (extra as u32) << 24 | 0x0000_ff00
        } else if (x, y) == (390, 290) {
            0x0012_3456
        } else {
            0
        }
    })
}

fn open(s: &mut Session, p: &PathBuf) {
    ex(s, "file.open", json!({"path": p.to_string_lossy()}));
}

fn only_bitmap(s: &Session) -> std::sync::Arc<fp_model::Element> {
    let t = top_elements(s);
    assert_eq!(t.len(), 1, "genau ein Objekt");
    assert!(matches!(t[0].kind, ElementKind::Bitmap(_)));
    t[0].clone()
}

fn obj(s: &Session) -> Rect {
    let e = only_bitmap(s);
    fp_engine::docops::element_bounds(&s.active().expect("d").doc, &e).expect("Grenzen")
}

fn frame(s: &Session, e: &fp_model::Element) -> Rect {
    fp_engine::docops::element_frame_bounds(&s.active().expect("d").doc, e).expect("Rahmen")
}

fn canvas(s: &Session) -> (f32, f32) {
    let d = s.active().expect("d");
    let c = &d.doc.pages[d.page].canvas;
    (c.width, c.height)
}

fn fresh() -> Session {
    doc_session(10.0, 10.0)
}

#[test]
fn open_png_trims_to_visible_pixels() {
    let mut s = fresh();
    open(&mut s, &t1("t1.png", 0));
    assert_eq!(obj(&s), Rect::new(100.0, 50.0, 200.0, 150.0));
    assert_eq!(canvas(&s), (400.0, 300.0), "Leinwand bleibt in Bildgröße");
    // Alpha 1 und Farbe mit Alpha 0 zählen nicht
    let mut s = fresh();
    open(&mut s, &t1("t2.png", 1));
    assert_eq!(obj(&s), Rect::new(100.0, 50.0, 200.0, 150.0));
    // ab Alpha 2 zählt das Pixel
    for a in [2u8, 3, 16, 128] {
        let mut s = fresh();
        open(&mut s, &t1(&format!("t5_a{a}.png"), a));
        assert_eq!(obj(&s), Rect::new(10.0, 10.0, 200.0, 150.0), "Alpha {a}");
    }
    // Rand nur links/rechts (wie N26)
    let mut s = fresh();
    open(&mut s, &png("t3.png", 300, 400, |x, _| if (80..220).contains(&x) { 0xff00_00ff } else { 0 }));
    assert_eq!(obj(&s), Rect::new(80.0, 0.0, 220.0, 400.0));
    // Pixel bleiben an ihrer Stelle
    let e = only_bitmap(&s);
    let ElementKind::Bitmap(b) = &e.kind else { unreachable!() };
    assert_eq!(b.image.pixel(0, 0).map(|c| (c.b, c.a)), Some((255, 255)));
}

#[test]
fn open_empty_png_gives_zero_bitmap() {
    let mut s = fresh();
    open(&mut s, &png("t4.png", 200, 100, |_, _| 0));
    let e = only_bitmap(&s);
    let r = fp_engine::docops::element_bounds(&s.active().expect("d").doc, &e).expect("Grenzen");
    assert_eq!(r, Rect::new(0.0, 0.0, 0.0, 0.0), "Bitmap 0×0 bei (0,0)");
    assert_eq!(canvas(&s), (200.0, 100.0));
    assert_eq!(fp_engine::docops::element_pixel_rect(&s.active().expect("d").doc, &e), None, "leer: pixelRect ±2³¹");
    // Speichern/Wiederöffnen läuft ohne Fehler
    let p = dir().join("t4_fp.fw.png");
    ex(&mut s, "file.saveAs", json!({"path": p.to_string_lossy()}));
    let mut s2 = fresh();
    open(&mut s2, &p);
    assert_eq!(canvas(&s2), (200.0, 100.0));
}

/// FIX-A: Das Original verwirft leere Bitmaps (0×0) beim Speichern. Dokument und Verlauf bleiben.
#[test]
fn save_drops_empty_bitmap_but_keeps_document() {
    let mut s = fresh();
    open(&mut s, &png("t4_drop.png", 200, 100, |_, _| 0));
    let e = only_bitmap(&s);
    // daneben ein normales Objekt, das erhalten bleiben muss
    let r = rect(&mut s, 10.0, 10.0, 40.0, 30.0);
    let hist = s.active().expect("d").history.len();
    let n = top_elements(&s).len();
    assert_eq!(n, 2);
    for i in 0..3 {
        let p = dir().join(format!("t4_drop_{i}.fw.png"));
        ex(&mut s, "file.saveAs", json!({"path": p.to_string_lossy()}));
        // Dokument und Verlauf unverändert
        assert_eq!(top_elements(&s).len(), n, "Dokument behält die leere Bitmap");
        assert!(top_elements(&s).iter().any(|x| x.id == e.id));
        assert_eq!(s.active().expect("d").history.len(), hist, "kein Verlaufsschritt");
        // Datei: nur das Rechteck
        let mut s2 = fresh();
        open(&mut s2, &p);
        let t = top_elements(&s2);
        assert_eq!(t.len(), 1, "leere Bitmap nicht geschrieben");
        assert!(!matches!(t[0].kind, ElementKind::Bitmap(_)));
        assert_eq!(fp_engine::docops::element_bounds(&s2.active().expect("d").doc, &t[0]), Some(bounds(&s, r)));
    }
    // Rückgängig nach dem Speichern wirkt weiter auf das Rechteck
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(top_elements(&s).len(), 1);
    assert!(matches!(top_elements(&s)[0].kind, ElementKind::Bitmap(_)));
}

#[test]
fn opaque_png_and_jpeg_like_stay_full() {
    let mut s = fresh();
    open(&mut s, &png("opaque.png", 50, 40, |x, _| 0xff00_0000 | x as u32));
    assert_eq!(obj(&s), Rect::new(0.0, 0.0, 50.0, 40.0));
}

#[test]
fn import_trims_and_keeps_offset_to_target_point() {
    let p = t1("imp_t1.png", 0);
    let mut s = doc_session(500.0, 500.0);
    let r = rect(&mut s, 0.0, 0.0, 10.0, 10.0);
    ex(&mut s, "file.import", json!({"path": p.to_string_lossy(), "at": [20, 30]}));
    let d = s.active().expect("d");
    assert_eq!(d.selection.elements.len(), 1);
    let id = d.selection.elements[0];
    assert_ne!(id, r);
    assert_eq!(bounds(&s, id), Rect::new(120.0, 80.0, 220.0, 180.0));
    // ohne Zielpunkt: Lage aus der Datei
    ex(&mut s, "file.import", json!({"path": p.to_string_lossy()}));
    let id2 = s.active().expect("d").selection.elements[0];
    assert_eq!(bounds(&s, id2), Rect::new(100.0, 50.0, 200.0, 150.0));
    // Rückgängig/Wiederholen: genau ein Schritt je Import
    let n = top_elements(&s).len();
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(top_elements(&s).len(), n - 1);
    ex(&mut s, "edit.redo", json!({}));
    assert_eq!(top_elements(&s).len(), n);
    assert_eq!(bounds(&s, top_elements(&s)[0].id), Rect::new(100.0, 50.0, 200.0, 150.0));
    // Rand nur links/rechts, Zielpunkt (20,30)
    let p3 = png("imp_t3.png", 300, 400, |x, _| if (80..220).contains(&x) { 0xff00_00ff } else { 0 });
    ex(&mut s, "file.import", json!({"path": p3.to_string_lossy(), "at": [20, 30]}));
    let id3 = s.active().expect("d").selection.elements[0];
    assert_eq!(bounds(&s, id3), Rect::new(100.0, 30.0, 240.0, 430.0));
}

#[test]
fn import_empty_png_creates_nothing() {
    let p = png("imp_t4.png", 200, 100, |_, _| 0);
    let mut s = doc_session(500.0, 500.0);
    let before = top_elements(&s).len();
    assert!(s.execute("file.import", json!({"path": p.to_string_lossy(), "at": [20, 30]})).is_err());
    assert_eq!(top_elements(&s).len(), before, "kein Objekt");
}

#[test]
fn fwpng_keeps_stored_bounds_and_frame_shows_visible_pixels() {
    // FW-PNG mit Bitmap über die ganze Leinwand (wie ältere Firepixel-Dateien)
    let data = std::fs::read(t1("full_src.png", 0)).expect("lesen");
    let r = fp_io::png_in::decode_png(&data).expect("PNG");
    let doc = fp_io::build::document_from_raster(r, fp_io::FileFormat::Png);
    let bytes = fp_fwpng::write(&doc, &fp_fwpng::WriteOptions::default(), &fp_fwpng::NoFlattener).expect("FW-PNG");
    let p = dir().join("t1_full.fw.png");
    std::fs::write(&p, bytes).expect("schreiben");
    let mut s = fresh();
    open(&mut s, &p);
    let e = only_bitmap(&s);
    assert_eq!(obj(&s), Rect::new(0.0, 0.0, 400.0, 300.0), "gespeicherte Grenzen bleiben");
    assert_eq!(frame(&s, &e), Rect::new(100.0, 50.0, 200.0, 150.0), "Rahmen um die sichtbaren Pixel");
    assert_eq!(fp_engine::docops::element_pixel_rect(&s.active().expect("d").doc, &e), Some(Rect::new(100.0, 50.0, 200.0, 150.0)));
    // Rundlauf: Speichern/Öffnen ändert die Grenzen nicht
    let p2 = dir().join("t1_full_again.fw.png");
    ex(&mut s, "file.saveAs", json!({"path": p2.to_string_lossy()}));
    let mut s2 = fresh();
    open(&mut s2, &p2);
    assert_eq!(obj(&s2), Rect::new(0.0, 0.0, 400.0, 300.0));

    // beschnittene PNG als FW-PNG speichern und wieder öffnen: beschnittene Grenzen bleiben
    let mut s3 = fresh();
    open(&mut s3, &t1("t1_trim_src.png", 0));
    let p3 = dir().join("t1_fp.fw.png");
    ex(&mut s3, "file.saveAs", json!({"path": p3.to_string_lossy()}));
    let mut s4 = fresh();
    open(&mut s4, &p3);
    assert_eq!(obj(&s4), Rect::new(100.0, 50.0, 200.0, 150.0));
    assert_eq!(canvas(&s4), (400.0, 300.0));
}

#[test]
fn painting_outside_updates_frame_and_undo_restores() {
    let data = std::fs::read(t1("paint_src.png", 0)).expect("lesen");
    let r = fp_io::png_in::decode_png(&data).expect("PNG");
    let doc = fp_io::build::document_from_raster(r, fp_io::FileFormat::Png);
    let bytes = fp_fwpng::write(&doc, &fp_fwpng::WriteOptions::default(), &fp_fwpng::NoFlattener).expect("FW-PNG");
    let p = dir().join("paint.fw.png");
    std::fs::write(&p, bytes).expect("schreiben");
    let mut s = fresh();
    open(&mut s, &p);
    let id = only_bitmap(&s).id;
    select(&mut s, &[id]);
    assert_eq!(frame(&s, &el(&s, id)), Rect::new(100.0, 50.0, 200.0, 150.0));
    s.set_colors(Some(fp_color::Argb(0xff00_00ff)), None);
    s.set_tool(ToolId::Pencil).expect("Bleistift");
    events(&mut s, &[ToolEvent::down(300.5, 250.5), ToolEvent::up(300.5, 250.5)]);
    let f = frame(&s, &el(&s, id));
    assert_eq!((f.left, f.top), (100.0, 50.0));
    assert!(f.right >= 301.0 && f.bottom >= 251.0, "Rahmen wächst mit dem Gemalten: {f:?}");
    assert_eq!(bounds(&s, id), Rect::new(0.0, 0.0, 400.0, 300.0), "Objektgrenzen bleiben");
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(frame(&s, &el(&s, id)), Rect::new(100.0, 50.0, 200.0, 150.0), "Rückgängig stellt den Rahmen wieder her");
    ex(&mut s, "edit.redo", json!({}));
    assert!(frame(&s, &el(&s, id)).right >= 301.0);
    // wiederholtes Abfragen liefert dasselbe (Zwischenspeicher)
    for _ in 0..50 {
        assert!(frame(&s, &el(&s, id)).right >= 301.0);
    }
}

#[test]
fn hover_highlight_uses_visible_pixels() {
    let mut s = fresh();
    open(&mut s, &t1("hover.png", 0));
    let e = only_bitmap(&s);
    {
        let d = s.active().expect("d");
        // geöffnete PNG: Objekt und Rahmen fallen zusammen
        assert_eq!(fp_engine::docops::element_frame_bounds(&d.doc, &e), fp_engine::docops::element_bounds(&d.doc, &e));
    }
    // FW-PNG mit Bitmap über die ganze Leinwand: Hervorhebung beim Überfahren um die sichtbaren Pixel
    let data = std::fs::read(t1("hover_src.png", 0)).expect("lesen");
    let r = fp_io::png_in::decode_png(&data).expect("PNG");
    let doc = fp_io::build::document_from_raster(r, fp_io::FileFormat::Png);
    let bytes = fp_fwpng::write(&doc, &fp_fwpng::WriteOptions::default(), &fp_fwpng::NoFlattener).expect("FW-PNG");
    let p = dir().join("hover.fw.png");
    std::fs::write(&p, bytes).expect("schreiben");
    let mut s = fresh();
    open(&mut s, &p);
    ex(&mut s, "select.none", json!({}));
    s.set_tool(ToolId::Pointer).expect("Zeiger");
    let resp = s.tool_event(&ToolEvent::drag(150.0, 100.0)).expect("Ereignis");
    let rect = resp.overlay.iter().find_map(|o| match o {
        fp_engine::Overlay::Highlight { rect, .. } => *rect,
        _ => None,
    });
    assert_eq!(rect, Some(Rect::new(100.0, 50.0, 200.0, 150.0)), "{:?}", resp.overlay);
}

/// N32: FW-PNG mit Bitmap über die ganze Leinwand (Objekt [0,0,400,300], sichtbar [100,50,200,150]),
/// gewählt mit dem Zeiger.
fn n32_session(name: &str) -> (Session, fp_model::ElementId) {
    let data = std::fs::read(t1(&format!("{name}_src.png"), 0)).expect("lesen");
    let r = fp_io::png_in::decode_png(&data).expect("PNG");
    let doc = fp_io::build::document_from_raster(r, fp_io::FileFormat::Png);
    let bytes = fp_fwpng::write(&doc, &fp_fwpng::WriteOptions::default(), &fp_fwpng::NoFlattener).expect("FW-PNG");
    let p = dir().join(format!("{name}.fw.png"));
    std::fs::write(&p, bytes).expect("schreiben");
    let mut s = fresh();
    open(&mut s, &p);
    let id = only_bitmap(&s).id;
    select(&mut s, &[id]);
    s.set_tool(ToolId::Pointer).expect("Zeiger");
    (s, id)
}

fn rects(ov: &[fp_engine::Overlay]) -> Vec<Rect> {
    ov.iter()
        .filter_map(|o| match o {
            fp_engine::Overlay::Ghost { quad, .. } => Rect::from_points(*quad).map(|r| r.normalized()),
            fp_engine::Overlay::Rect { rect, .. } => Some(*rect),
            fp_engine::Overlay::TransformBox { quad, .. } => Rect::from_points(*quad).map(|r| r.normalized()),
            fp_engine::Overlay::Highlight { rect, .. } => *rect,
            _ => None,
        })
        .collect()
}

/// N32: Beim Verschieben zeigte die Vorschau (Ghost) die gespeicherten Objektgrenzen, der
/// Auswahlrahmen mit Griffen die sichtbaren Pixel – zwei blaue Rechtecke. Jetzt nur noch eines.
#[test]
fn n32_move_preview_uses_visible_pixels() {
    let (mut s, id) = n32_session("n32_move");
    s.tool_event(&ToolEvent::down(150.0, 100.0)).expect("down");
    let resp = s.tool_event(&ToolEvent::drag(170.0, 110.0)).expect("drag");
    let r = rects(&resp.overlay);
    assert_eq!(r, vec![Rect::new(120.0, 60.0, 220.0, 160.0)], "nur ein Rahmen um die sichtbaren Pixel: {:?}", resp.overlay);
    // weit über die Leinwand hinaus (Bild teilweise außerhalb)
    let resp = s.tool_event(&ToolEvent::drag(400.0, 100.0)).expect("drag");
    assert_eq!(rects(&resp.overlay), vec![Rect::new(350.0, 50.0, 450.0, 150.0)], "{:?}", resp.overlay);
    s.tool_event(&ToolEvent::up(400.0, 100.0)).expect("up");
    assert_eq!(frame(&s, &el(&s, id)), Rect::new(350.0, 50.0, 450.0, 150.0));
    assert_eq!(bounds(&s, id), Rect::new(250.0, 0.0, 650.0, 300.0), "Objektgrenzen wandern mit");
    // nochmal anfassen und ziehen: Vorschau wieder um die sichtbaren Pixel, nicht um [250,0,650,300]
    s.tool_event(&ToolEvent::down(380.0, 100.0)).expect("down");
    let resp = s.tool_event(&ToolEvent::drag(370.0, 100.0)).expect("drag");
    assert_eq!(rects(&resp.overlay), vec![Rect::new(340.0, 50.0, 440.0, 150.0)], "{:?}", resp.overlay);
    s.tool_event(&ToolEvent::up(370.0, 100.0)).expect("up");
}

/// N32: Die Griffe sitzen an den Ecken des gezeichneten Rahmens (sichtbare Pixel); an den Ecken der
/// gespeicherten Objektgrenzen gibt es keinen Griff.
#[test]
fn n32_handles_sit_on_visible_frame() {
    let (mut s, id) = n32_session("n32_handles");
    // Eckgriff rechts unten des sichtbaren Rahmens ziehen → Skalieren mit einem Rahmen
    s.tool_event(&ToolEvent::down(200.0, 150.0)).expect("down");
    let resp = s.tool_event(&ToolEvent::drag(250.0, 200.0)).expect("drag");
    // Vorschau (Ghost) und Rahmen decken sich: ein sichtbares Rechteck
    let r = rects(&resp.overlay);
    assert!(!r.is_empty() && r.iter().all(|x| *x == Rect::new(100.0, 50.0, 250.0, 200.0)), "Skalieren vom sichtbaren Rahmen aus: {:?}", resp.overlay);
    s.tool_event(&ToolEvent::up(250.0, 200.0)).expect("up");
    let f = frame(&s, &el(&s, id));
    // Neuberechnung beim Skalieren: Kantenpixel der Interpolation (± 2 px)
    assert!((f.left - 100.0).abs() <= 2.0 && (f.right - 250.0).abs() <= 2.0 && (f.bottom - 200.0).abs() <= 2.0, "{f:?}");
    // alte Ecke der Objektgrenzen ist kein Griff mehr: Ziehen dort skaliert nicht
    let (mut s, id) = n32_session("n32_handles2");
    s.tool_event(&ToolEvent::down(399.0, 299.0)).expect("down");
    let resp = s.tool_event(&ToolEvent::drag(380.0, 280.0)).expect("drag");
    s.tool_event(&ToolEvent::up(380.0, 280.0)).expect("up");
    assert!(resp.overlay.iter().all(|o| !matches!(o, fp_engine::Overlay::Handles { .. })), "kein Skalieren an der alten Ecke: {:?}", resp.overlay);
    let b = bounds(&s, id);
    assert_eq!((b.width(), b.height()), (400.0, 300.0), "nicht skaliert: {b:?}");
}

/// N32 → re 28 L5 (RT-3): Das Original setzt Rahmen, Griffe und Mittelpunkt des Skalieren-Werkzeugs
/// auf das gespeicherte **Objektrechteck** (gemessen 0/400 × 0/300 bei Pixeln 100–200), nicht auf die
/// sichtbaren Pixel; der Zeiger zeigt seine Griffe weiter an den sichtbaren Pixeln.
#[test]
fn n32_transform_box_uses_object_bounds() {
    let (mut s, _) = n32_session("n32_transform");
    s.set_tool(ToolId::Scale).expect("Skalieren");
    let resp = s.tool_event(&ToolEvent::drag(10.0, 10.0)).expect("hover");
    let boxes: Vec<Rect> = resp
        .overlay
        .iter()
        .filter_map(|o| match o {
            fp_engine::Overlay::TransformBox { quad, .. } => Rect::from_points(*quad).map(|r| r.normalized()),
            _ => None,
        })
        .collect();
    assert_eq!(boxes, vec![Rect::new(0.0, 0.0, 400.0, 300.0)], "{:?}", resp.overlay);
}