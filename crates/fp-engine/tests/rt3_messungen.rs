//! RT-3: Soll-Regeln aus der Mess-Sitzung `re\28` (Messwerte des Originals): L3 Beschnitt beim
//! Verlassen des Bitmap-Modus, L4 Einfügen fremder Pixel, L5 Skalieren/Drehen, L7 Rand-Maske,
//! L9 Filter auf Vektor, S3 aktive Unterebene, S5 Auswahl nach „Als Maske einfügen“,
//! E6 Sprites/PDF.

use crate::common;
use common::*;
use fp_engine::tools::{ToolId, ToolKey};
use fp_engine::{Session, ToolEvent};
use fp_geom::Rect;
use serde_json::json;
use std::path::PathBuf;

fn dir() -> PathBuf {
    let d = std::env::temp_dir().join(format!("fp-rt3-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&d);
    d
}

fn tool(s: &mut Session, t: ToolId) {
    s.set_tool(t).expect("Werkzeug");
}

fn history(s: &Session) -> usize {
    s.active().expect("d").history.undo_count()
}

fn esc(s: &mut Session) {
    events(s, &[ToolEvent::key(ToolKey::Escape)]);
}

/// Pixel 400 × 300 mit deckendem Rot 100,50–200,150, sonst Alpha 0.
fn rand_pixels() -> (Vec<u32>, usize, usize) {
    let (w, h) = (400usize, 300usize);
    (
        (0..w * h).map(|i| if (100..200).contains(&(i % w)) && (50..150).contains(&(i / w)) { 0xffff_0000 } else { 0 }).collect(),
        w,
        h,
    )
}

/// `re5a_t1_rand_fp.fw.png`: Bitmap 0,0–400,300 (gespeichert ungeschnitten), Pixel 100,50–200,150.
fn rand_bitmap(name: &str) -> (Session, fp_model::ElementId) {
    let (px, w, h) = rand_pixels();
    let bytes = fp_export::png_out::write_png_truecolor(&px, w, h, true, &fp_export::png_out::PngOptions::default()).expect("PNG");
    let r = fp_io::png_in::decode_png(&bytes).expect("PNG lesen");
    let doc = fp_io::build::document_from_raster(r, fp_io::FileFormat::Png);
    let fw = fp_fwpng::write(&doc, &fp_fwpng::WriteOptions::default(), &fp_fwpng::NoFlattener).expect("FW-PNG");
    let p = dir().join(format!("{name}.fw.png"));
    std::fs::write(&p, fw).expect("schreiben");
    let mut s = doc_session(10.0, 10.0);
    ex(&mut s, "file.open", json!({"path": p.to_string_lossy()}));
    let id = top_elements(&s)[0].id;
    assert_eq!(bounds(&s, id), Rect::new(0.0, 0.0, 400.0, 300.0));
    select(&mut s, &[id]);
    (s, id)
}

fn marquee(s: &mut Session, l: f32, t: f32, r: f32, b: f32) {
    tool(s, ToolId::Marquee);
    events(s, &[ToolEvent::down(l, t), ToolEvent::drag((l + r) / 2.0, (t + b) / 2.0), ToolEvent::drag(r, b), ToolEvent::up(r, b)]);
    assert!(s.active().expect("d").selection.has_pixels(), "Auswahlrahmen");
}

// ------------------------------------------------------------------ L3

/// L3 (re 28, Fall A/B3): Rahmen 100,50–150,150, Entf → Grenzen bleiben, solange der Modus läuft;
/// Strg+D, Esc → Zeiger, Bitmap 150,50, 50 × 100; Kopie übernimmt die beschnittenen Grenzen.
#[test]
fn l3_exit_bitmap_mode_trims_after_delete() {
    let (mut s, id) = rand_bitmap("l3a");
    marquee(&mut s, 100.0, 50.0, 150.0, 150.0);
    ex(&mut s, "edit.clear", json!({}));
    assert_eq!(bounds(&s, id), Rect::new(0.0, 0.0, 400.0, 300.0), "im Modus ungeschnitten");
    assert_eq!(s.active().expect("d").bitmap_mode_bounds(id), Some(Rect::new(0.0, 0.0, 400.0, 300.0)), "meldet Leinwand");
    ex(&mut s, "select.none", json!({}));
    let n = history(&s);
    esc(&mut s);
    assert_eq!(s.tools.active, ToolId::Pointer, "Esc → Zeiger");
    assert_eq!(bounds(&s, id), Rect::new(150.0, 50.0, 200.0, 150.0), "150,50, 50 × 100");
    // Select None above emptied the object selection (Escape itself keeps a selected bitmap
    // selected, re 58 §6); select the bitmap again for the copy
    select(&mut s, &[id]);
    assert!(history(&s) <= n + 1);
    ex(&mut s, "edit.copy", json!({}));
    let v = ex(&mut s, "edit.paste", json!({}));
    let cid = fp_model::ElementId(v["ids"][0].as_u64().expect("id"));
    assert_eq!(bounds(&s, cid), Rect::new(150.0, 50.0, 200.0, 150.0), "Kopie an gleicher Stelle");
    // Rückgängig stellt die ungeschnittenen Grenzen wieder her (Paste, Copy – a row of its own
    // since TW-137 – and the trim)
    ex(&mut s, "edit.undo", json!({}));
    ex(&mut s, "edit.undo", json!({}));
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(bounds(&s, id), Rect::new(0.0, 0.0, 400.0, 300.0));
}

/// L3 B1/B2: Esc ohne Bearbeitung ändert nichts; Rahmen ziehen, Strg+D, Esc (keine Pixeländerung)
/// → 100,50, 100 × 100.
#[test]
fn l3_trim_without_pixel_change_but_not_without_work() {
    let (mut s, id) = rand_bitmap("l3b");
    tool(&mut s, ToolId::Marquee);
    esc(&mut s);
    assert_eq!(bounds(&s, id), Rect::new(0.0, 0.0, 400.0, 300.0), "B1 bleibt");
    marquee(&mut s, 20.0, 20.0, 60.0, 60.0);
    ex(&mut s, "select.none", json!({}));
    esc(&mut s);
    assert_eq!(bounds(&s, id), Rect::new(100.0, 50.0, 200.0, 150.0), "B2 100,50, 100 × 100");
    assert!(s.active().expect("d").bitmap_worked.is_empty());
}

/// L3 E: Radiergummi-Zug quer durchs Quadrat, Esc → 100,50, 100 × 100 (auch der Radierer
/// beschneidet beim Verlassen).
#[test]
fn l3_eraser_then_escape_trims() {
    let (mut s, id) = rand_bitmap("l3e");
    tool(&mut s, ToolId::Eraser);
    events(&mut s, &[ToolEvent::down(90.0, 100.0), ToolEvent::drag(150.0, 100.0), ToolEvent::drag(210.0, 100.0), ToolEvent::up(210.0, 100.0)]);
    assert_eq!(bounds(&s, id), Rect::new(0.0, 0.0, 400.0, 300.0), "Zug selbst beschneidet nicht");
    esc(&mut s);
    assert_eq!(bounds(&s, id), Rect::new(100.0, 50.0, 200.0, 150.0), "nach Esc beschnitten");
}

/// L3: Zeigerklick (außerhalb eines Rahmens) nach der Bearbeitung beendet den Modus mit Beschnitt.
#[test]
fn l3_pointer_click_trims() {
    let (mut s, id) = rand_bitmap("l3p");
    marquee(&mut s, 100.0, 50.0, 150.0, 150.0);
    ex(&mut s, "edit.clear", json!({}));
    ex(&mut s, "select.none", json!({}));
    tool(&mut s, ToolId::Pointer);
    assert_eq!(bounds(&s, id), Rect::new(0.0, 0.0, 400.0, 300.0), "Werkzeugwechsel allein beschneidet nicht");
    events(&mut s, &[ToolEvent::down(170.0, 100.0), ToolEvent::up(170.0, 100.0)]);
    assert_eq!(bounds(&s, id), Rect::new(150.0, 50.0, 200.0, 150.0), "Klick beschneidet");
    assert!(!s.active().expect("d").bitmap_mode);
}

// ------------------------------------------------------------------ L5

/// L5 (re 28): `scaleSelection(0.5, 0.5, …)` skaliert um die Mitte der Objektgrenzen und schneidet
/// nie zu (100,75, 200 × 150); `rotateSelection(90)` dreht um die Objektmitte, mit
/// `autoTrimImages` danach beschnitten (200,50, 100 × 100), ohne 50,−50, 300 × 400.
#[test]
fn l5_numeric_scale_never_trims_rotate_trims_with_option() {
    let (mut s, id) = rand_bitmap("l5a");
    ex(&mut s, "modify.transform.numericTransform", json!({"mode": "scale", "scaleX": 50, "scaleY": 50, "constrain": false, "autoTrim": true}));
    assert_eq!(bounds(&s, id), Rect::new(100.0, 75.0, 300.0, 225.0), "Skalieren 0,5");
    ex(&mut s, "edit.undo", json!({}));
    ex(&mut s, "modify.transform.numericTransform", json!({"mode": "rotate", "angle": 90, "autoTrim": true}));
    assert!(rect_approx(bounds(&s, id), Rect::new(200.0, 50.0, 300.0, 150.0)), "Drehen mit autoTrimImages: {:?}", bounds(&s, id));
    ex(&mut s, "edit.undo", json!({}));
    ex(&mut s, "modify.transform.numericTransform", json!({"mode": "rotate", "angle": 90}));
    assert!(rect_approx(bounds(&s, id), Rect::new(50.0, -50.0, 350.0, 350.0)), "Drehen ohne: {:?}", bounds(&s, id));
}

/// L5: Skalieren-Werkzeug – Griffe am Objektrechteck 0/400 × 0/300: Eckgriff unten rechts +50,+50
/// → 0,0, 456 × 342 (Faktor 1,14, nicht zugeschnitten); Drehen außerhalb des Rahmens beschneidet.
#[test]
fn l5_scale_tool_handles_on_object_bounds_rotation_trims() {
    let (mut s, id) = rand_bitmap("l5b");
    tool(&mut s, ToolId::Scale);
    events(&mut s, &[ToolEvent::down(400.0, 300.0), ToolEvent::drag(425.0, 325.0), ToolEvent::drag(450.0, 350.0), ToolEvent::up(450.0, 350.0)]);
    events(&mut s, &[ToolEvent::key(ToolKey::Enter)]);
    let b = bounds(&s, id);
    assert!((b.left, b.top) == (0.0, 0.0) && (b.width() - 456.0).abs() <= 1.0 && (b.height() - 342.0).abs() <= 1.0, "Eckgriff: {b:?}");
    ex(&mut s, "edit.undo", json!({}));
    select(&mut s, &[id]);
    tool(&mut s, ToolId::Scale);
    // außerhalb des Rahmens ziehen = drehen (um 200,150)
    events(&mut s, &[ToolEvent::down(430.0, 150.0), ToolEvent::drag(400.0, 250.0), ToolEvent::drag(200.0, 380.0), ToolEvent::up(200.0, 380.0)]);
    events(&mut s, &[ToolEvent::key(ToolKey::Enter)]);
    let b = bounds(&s, id);
    let px = fp_engine::docops::element_pixel_rect(&s.active().expect("d").doc, &el(&s, id)).expect("pixelRect");
    assert!(rect_approx(b, px), "Drehen mit dem Werkzeug beschneidet: {b:?} / {px:?}");
    assert!(b.width() < 200.0, "{b:?}");
}

// ------------------------------------------------------------------ L9

/// L9 (re 28): Filter auf einen Vektor nach OK auf die Rückfrage: Rastern und Filtern sind **ein**
/// Verlaufsschritt (Gaußscher Weichzeichner 4,1 an 100,80–250,200 → Bitmap um den Rand
/// vergrößert); ohne Filter-OK bleibt der Vektor und es entsteht kein Schritt.
#[test]
fn l9_filter_on_vector_rasterizes_in_one_step() {
    let mut s = doc_session(400.0, 300.0);
    let r = rect(&mut s, 100.0, 80.0, 250.0, 200.0);
    select(&mut s, &[r]);
    let n = history(&s);
    s.filter_rasterize = true;
    ex(&mut s, "filter.blur.gaussian", json!({"radius": 4.1}));
    s.filter_rasterize = false;
    assert_eq!(history(&s), n + 1, "ein Verlaufsschritt");
    let t = top_elements(&s);
    assert_eq!(t.len(), 1);
    assert!(matches!(t[0].kind, fp_model::ElementKind::Bitmap(_)), "gerastert");
    let b = bounds(&s, t[0].id);
    assert!(b.left < 100.0 && b.top < 80.0 && b.right > 250.0 && b.bottom > 200.0, "Weichzeichner dehnt aus: {b:?}");
    // Rückgängig: wieder der Vektor
    ex(&mut s, "edit.undo", json!({}));
    let t = top_elements(&s);
    assert!(matches!(t[0].kind, fp_model::ElementKind::Path(_)), "Vektor nach Rückgängig");
    assert_eq!(history(&s), n);
    // gleicher Weg über den Parameter
    select(&mut s, &[t[0].id]);
    ex(&mut s, "filter.adjustColor.invert", json!({"rasterize": true}));
    assert_eq!(history(&s), n + 1);
    assert!(matches!(top_elements(&s)[0].kind, fp_model::ElementKind::Bitmap(_)));
}

// ------------------------------------------------------------------ L7

/// L7 (re 28): Rand 5 an 330,60–380,110 – Maske symmetrisch, Grenzen 324,54–386,116, Profil je
/// Kante 15 47 95 159 207 239 239 207 159 95 47 15 (12 px um die Kante), letzte Zeile 29 statt 15
/// (Eigenheit der letzten Zeile auch in der Morphologie); Schwelle ≥ 128: 327,57–383,113.
#[test]
fn l7_border_5_profile_and_last_row() {
    use fp_geom::IRect;
    let m = fp_raster::Mask::from_rect(IRect::new(330, 60, 380, 110)).expect("Maske");
    let b = m.border(5, 1).expect("Rand");
    assert_eq!(b.tight_bounds(), IRect::new(324, 54, 386, 116));
    let prof = [15u8, 47, 95, 159, 207, 239, 239, 207, 159, 95, 47, 15];
    let row: Vec<u8> = (324..386).map(|x| b.get(x, 85)).collect();
    assert_eq!(&row[..12], &prof, "links");
    assert!(row[12..50].iter().all(|&v| v == 0), "Mitte leer: {row:?}");
    assert_eq!(&row[50..], &prof, "rechts");
    let col: Vec<u8> = (54..116).map(|y| b.get(355, y)).collect();
    assert_eq!(&col[..12], &prof, "oben");
    let mut bottom = prof;
    bottom[11] = 29;
    assert_eq!(&col[50..], &bottom, "unten, letzte Zeile 29");
    let (mut l, mut t, mut r, mut bo) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
    for y in 50..120 {
        for x in 320..390 {
            if b.get(x, y) >= 128 {
                l = l.min(x);
                t = t.min(y);
                r = r.max(x + 1);
                bo = bo.max(y + 1);
            }
        }
    }
    assert_eq!((l, t, r, bo), (327, 57, 383, 113), "Schwelle 128");
}

// ------------------------------------------------------------------ S2

fn canvas(s: &Session) -> (f32, f32, f32, f32) {
    let d = s.active().expect("d");
    let c = &d.doc.pages[d.page].canvas;
    (c.origin.x, c.origin.y, c.width, c.height)
}

/// S2 (re 28): Symbol aus Rechteck 0,0–50,50, Instanz nach 200,150. An Position bearbeiten: Leinwand
/// in Dokumentgröße so gelegt, dass der Inhalt (−25,−25, 50 × 50 in Symbolkoordinaten) an der
/// Instanzlage 200,150–250,200 erscheint; Symbol bearbeiten (Bibliothek): Ursprung = Leinwandmitte.
#[test]
fn s2_symbol_edit_origin_in_place_and_library() {
    let mut s = doc_session(400.0, 300.0);
    rect(&mut s, 0.0, 0.0, 100.0, 100.0);
    let b = rect(&mut s, 0.0, 0.0, 50.0, 50.0);
    select(&mut s, &[b]);
    ex(&mut s, "modify.symbol.convertToSymbol", json!({"name": "S2sym"}));
    ex(&mut s, "edit.move", json!({"dx": 200, "dy": 150}));
    let inst = s.active().expect("d").selection.elements[0];
    assert_eq!(bounds(&s, inst), Rect::new(200.0, 150.0, 250.0, 200.0));
    let parent = s.active_id().expect("Dokument");
    ex(&mut s, "modify.symbol.editInPlace", json!({}));
    assert_ne!(s.active_id(), Some(parent));
    let (l, t, w, h) = canvas(&s);
    assert_eq!((w, h), (400.0, 300.0), "Leinwand in Dokumentgröße");
    // der Inhalt liegt 200,150 vom Leinwandrand entfernt (Instanzlage)
    let cb = bounds(&s, top_elements(&s)[0].id);
    assert_eq!((cb.left - l, cb.top - t), (200.0, 150.0), "Lage an Position, Inhalt {cb:?}");
    // L-3: JSF nach selectAll meldet RectanglePrimitive −25,−25, 50 × 50 (Symbolkoordinaten,
    // Ursprung = Mitte des Inhalts bei der Umwandlung)
    assert_eq!(cb, Rect::new(-25.0, -25.0, 25.0, 25.0), "Symbolkoordinaten an Position");
    ex(&mut s, "modify.symbol.exitEdit", json!({}));
    select(&mut s, &[inst]);
    ex(&mut s, "modify.symbol.editSymbol", json!({}));
    let (l, t, w, h) = canvas(&s);
    assert_eq!((l + w / 2.0, t + h / 2.0), (0.0, 0.0), "Ursprung in der Leinwandmitte");
    assert_eq!((w, h), (400.0, 300.0));
    // L-3: Bibliothek: Inhalt 175,125–225,175 auf der Leinwand, Symbolkoordinaten −25…25
    let cb = bounds(&s, top_elements(&s)[0].id);
    assert_eq!(cb, Rect::new(-25.0, -25.0, 25.0, 25.0));
    assert_eq!((cb.left - l, cb.top - t), (175.0, 125.0), "Inhalt mittig auf der Leinwand");
}

/// L-3 (re 28 S2, re 13 T1): Beim Umwandeln liegt der Symbolursprung in der Mitte der
/// Inhaltsgrenzen; `MTXv = ATMv = T(Mitte)`, die Instanz bleibt an ihrer Stelle. T1: Rechteck
/// 100 × 50 bei 100,100 → `T(150,125)`, Symbolgrenzen −50…50/−25…25.
#[test]
fn l3_convert_to_symbol_origin_is_content_center() {
    let mut s = doc_session(400.0, 300.0);
    let r = rect(&mut s, 100.0, 100.0, 200.0, 150.0);
    select(&mut s, &[r]);
    let v = ex(&mut s, "modify.symbol.convertToSymbol", json!({"name": "N"}));
    let inst = fp_model::ElementId(v["id"].as_u64().expect("id"));
    assert_eq!(bounds(&s, inst), Rect::new(100.0, 100.0, 200.0, 150.0), "Instanz bleibt an ihrer Stelle");
    let d = s.active().expect("d");
    let fp_model::element::ElementKind::Instance(i) = &el(&s, inst).kind else { panic!("Instanz") };
    assert_eq!(i.transform, fp_geom::Matrix3::translation(150.0, 125.0));
    assert_eq!(i.alt_transform, Some(fp_geom::Matrix3::translation(150.0, 125.0)));
    let (_, sym) = d.doc.symbol_by_id(i.symbol_id).expect("Symbol");
    assert_eq!(fp_engine::symbol_origin::symbol_content_bounds(&d.doc, sym), Some(Rect::new(-50.0, -25.0, 50.0, 25.0)));
    // ungerade Breite: Ursprung auf ganze Pixel gerundet (Bitmaps bleiben auf dem Raster)
    let q = rect(&mut s, 10.0, 10.0, 21.0, 21.0);
    select(&mut s, &[q]);
    let v = ex(&mut s, "modify.symbol.convertToSymbol", json!({"name": "Q"}));
    let qi = fp_model::ElementId(v["id"].as_u64().expect("id"));
    assert_eq!(bounds(&s, qi), Rect::new(10.0, 10.0, 21.0, 21.0));
    let fp_model::element::ElementKind::Instance(i) = &el(&s, qi).kind else { panic!("Instanz") };
    assert_eq!(i.transform, fp_geom::Matrix3::translation(16.0, 16.0));
}

/// M6 §4a (re\33): Pinselzug ohne Esc, dann Werkzeugwechsel (Taste V) verlässt den Bitmap-Modus
/// wie Esc – eigener Verlaufsschritt (`exitPaintMode`) und sofortiger Beschnitt auf den
/// Pixelinhalt (Original: 0,0, 700 × 400 → 100,100, 201 × 61). Wechsel zwischen Malwerkzeugen
/// bleibt im Modus; der Wechsel vom Auswahlrahmen beschneidet weiter nicht (L3).
#[test]
fn m6_tool_switch_after_brush_exits_paint_mode() {
    let mut s = doc_session(700.0, 400.0);
    tool(&mut s, ToolId::Brush);
    let mut ev = vec![ToolEvent::down(100.0, 100.0)];
    ev.extend((1..=20).map(|i| ToolEvent::drag(100.0 + i as f32 * 10.0, 100.0 + i as f32 * 3.0)));
    ev.push(ToolEvent::up(300.0, 160.0));
    events(&mut s, &ev);
    let id = s.active().expect("d").selection.elements[0];
    let px = {
        let d = s.active().expect("d");
        assert!(d.bitmap_mode || !d.bitmap_worked.is_empty(), "nach dem Zug im Bitmap-Modus");
        fp_engine::docops::element_pixel_rect(&d.doc, &el(&s, id)).expect("Pixel")
    };
    // Bleistift: Malwerkzeug, Modus bleibt
    tool(&mut s, ToolId::Pencil);
    tool(&mut s, ToolId::Brush);
    let n = history(&s);
    tool(&mut s, ToolId::Pointer);
    assert_eq!(history(&s), n + 1, "eigener Schritt exitPaintMode");
    assert!(!s.active().expect("d").bitmap_mode);
    assert_eq!(bounds(&s, id), px, "sofort auf den Pixelinhalt beschnitten");
    // Zeigerklick danach ändert nichts mehr
    events(&mut s, &[ToolEvent::down(650.0, 380.0), ToolEvent::up(650.0, 380.0)]);
    assert_eq!(bounds(&s, id), px);
}

/// re 37 §5a: Radiergummi-Zug, dann Werkzeugwechsel zum Zeiger ohne Esc → `exitPaintMode` als
/// eigener Schritt, sofort auf den Pixelinhalt beschnitten; ein Zeigerklick ändert nichts mehr.
#[test]
fn m7_eraser_tool_switch_exits_paint_mode_and_trims() {
    let (mut s, id) = rand_bitmap("m7e");
    tool(&mut s, ToolId::Eraser);
    let mut ev = vec![ToolEvent::down(80.0, 40.0)];
    ev.extend((0..=12).map(|i| ToolEvent::drag(80.0 + (i % 2) as f32 * 70.0, 40.0 + i as f32 * 10.0)));
    ev.push(ToolEvent::up(80.0, 165.0));
    events(&mut s, &ev);
    assert_eq!(bounds(&s, id), Rect::new(0.0, 0.0, 400.0, 300.0), "Zug selbst beschneidet nicht");
    let px = {
        let d = s.active().expect("d");
        fp_engine::docops::element_pixel_rect(&d.doc, &el(&s, id)).expect("Pixel")
    };
    let n = history(&s);
    tool(&mut s, ToolId::Pointer);
    assert_eq!(history(&s), n + 1, "eigener Schritt exitPaintMode");
    assert!(!s.active().expect("d").bitmap_mode);
    assert_eq!(bounds(&s, id), px, "sofort beschnitten");
    events(&mut s, &[ToolEvent::down(650.0, 380.0), ToolEvent::up(650.0, 380.0)]);
    assert_eq!(bounds(&s, id), px);
}

/// re 37 §5b: Menüfilter beschneidet außerhalb des Bitmap-Modus sofort (A0); mit aktivem Werkzeug
/// des Bitmap-Abschnitts (Auswahlrahmen) ohne Pixelauswahl bleiben die Grenzen (E4), erst das
/// Verlassen des Modus beschneidet.
#[test]
fn m7_filter_in_bitmap_mode_does_not_trim_until_exit() {
    let (mut s, id) = rand_bitmap("m7a0");
    ex(&mut s, "filter.blur.gaussian", json!({"radius": 4}));
    let b = bounds(&s, id);
    assert!(b.left > 80.0 && b.right < 220.0, "A0 beschnitten: {b:?}");
    let (mut s, id) = rand_bitmap("m7e4");
    tool(&mut s, ToolId::Marquee);
    assert!(!s.active().expect("d").selection.has_pixels());
    ex(&mut s, "filter.blur.gaussian", json!({"radius": 4}));
    assert_eq!(bounds(&s, id), Rect::new(0.0, 0.0, 400.0, 300.0), "E4 im Modus nicht beschnitten");
    esc(&mut s);
    let b = bounds(&s, id);
    assert!(b.left > 80.0 && b.right < 220.0, "Verlassen beschneidet: {b:?}");
}

/// M10 (re 43 §5): Ursprung = Mitte der Pfadgrenzen ohne Strich und Effekte, je Achse halb weg
/// von null gerundet (0,0–51,51 → 26, 26; −51,100–0,151 → −26, 126); Strich 10 und Schlagschatten
/// verschieben ihn nicht; 9-Slice-Vorgabehilfslinien immer ±25 um den Ursprung (auch leer).
#[test]
fn t11_symbol_origin_path_bounds_half_away_and_nine_slice_25() {
    let mut s = doc_session(600.0, 400.0);
    let origin = |s: &mut Session, id: fp_model::ElementId, name: &str| {
        select(s, &[id]);
        let v = ex(s, "modify.symbol.convertToSymbol", json!({"name": name}));
        let inst = fp_model::ElementId(v["id"].as_u64().expect("id"));
        let fp_model::element::ElementKind::Instance(i) = &el(s, inst).kind else { panic!("Instanz") };
        (i.transform, i.symbol_id)
    };
    let p = |x: f32, y: f32| fp_geom::Matrix3::translation(x, y);
    let r = rect(&mut s, 0.0, 0.0, 51.0, 51.0);
    assert_eq!(origin(&mut s, r, "A").0, p(26.0, 26.0));
    let r = rect(&mut s, -51.0, 100.0, 0.0, 151.0);
    assert_eq!(origin(&mut s, r, "B").0, p(-26.0, 126.0));
    let r = rect(&mut s, -52.0, 200.0, -1.0, 251.0);
    assert_eq!(origin(&mut s, r, "C").0, p(-27.0, 226.0));
    // Strich 10 außen und Schlagschatten zählen nicht
    let r = rect(&mut s, 100.0, 300.0, 150.0, 350.0);
    select(&mut s, &[r]);
    ex(&mut s, "element.setStroke", json!({"color": "#000000", "width": 10}));
    ex(&mut s, "element.addEffect", json!({"effect": "dropShadow"}));
    let (o, _) = origin(&mut s, r, "D");
    assert_eq!(o, p(125.0, 325.0));
    // 9-Slice: Vorgabehilfslinien ±25 um den Ursprung, nicht Drittel
    let idx = |s: &Session, n: &str| s.active().expect("d").doc.symbols.iter().position(|x| x.name == n).expect("Symbol");
    let i = idx(&s, "D");
    ex(&mut s, "panel.library.nineSlice", json!({"index": i, "on": true}));
    let guides = |s: &Session, k: usize| s.active().expect("d").doc.symbols[k].nine_slice.as_ref().expect("9-Slice").guides.clone();
    assert_eq!(guides(&s, i), vec![-25.0, 25.0, -25.0, 25.0]);
    // großes Symbol (300 × 30) und leeres neues Symbol: ebenfalls ±25
    let r = rect(&mut s, 100.0, 50.0, 400.0, 80.0);
    origin(&mut s, r, "E");
    let i = idx(&s, "E");
    ex(&mut s, "panel.library.nineSlice", json!({"index": i, "on": true}));
    assert_eq!(guides(&s, i), vec![-25.0, 25.0, -25.0, 25.0]);
    ex(&mut s, "panel.library.newSymbol", json!({"name": "N9e", "nineSlice": true}));
    let n = idx(&s, "N9e");
    assert_eq!(guides(&s, n), vec![-25.0, 25.0, -25.0, 25.0]);
}
