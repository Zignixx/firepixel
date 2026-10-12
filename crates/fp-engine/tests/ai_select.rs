//! KI-Auswahl (Agent KI): Befehle `select.aiObject`, `select.aiSubject`, `select.aiRefineEdge`
//! und die Werkzeuge im Flyout des Zauberstabs – mit den echten Modellen, sofern sie im
//! Modellordner liegen (sonst werden die Modell-Tests übersprungen und nur die Fehlerwege geprüft).
//!
//! Testbild: eine Bitmap mit unruhigem Hintergrund, einer roten Scheibe und einem blauen Quadrat.

use crate::common;

use common::*;
use fp_engine::commands::ai as ai;
use fp_engine::tools::{ToolEvent, ToolId};
use fp_engine::Session;
use fp_geom::IRect;
use fp_model::ElementId;
use fp_raster::RgbaImage;
use serde_json::{json, Value};

const W: i32 = 400;
const H: i32 = 300;
/// Scheibe (Mitte, Radius) und Quadrat relativ zur Bitmap.
const DISK: (f32, f32, f32) = (120.0, 150.0, 55.0);
const SQUARE: (i32, i32, i32, i32) = (250, 80, 340, 170);

fn have_models() -> bool {
    use ai::fp_ai::models::{available, ModelId};
    available(ModelId::Sam2Tiny) && available(ModelId::BiRefNetLite) && available(ModelId::BiRefNetLiteMatting)
}

macro_rules! need_models {
    () => {
        if !have_models() {
            eprintln!("KI-Modelle fehlen – Test übersprungen");
            return;
        }
    };
}

/// Testbild an (`l`, `t`).
fn test_image(l: i32, t: i32) -> RgbaImage {
    let mut img = RgbaImage::new(IRect::from_xywh(l, t, W, H)).unwrap_or_else(|_| RgbaImage::empty_at(0, 0));
    let w = img.width();
    for (i, px) in img.data_mut().iter_mut().enumerate() {
        let (x, y) = ((i % w) as i32, (i / w) as i32);
        // unruhiger Hintergrund: Streifen + Rauschen in Grüntönen
        let n = ((x * 7919 + y * 104_729) ^ (x * y)) & 63;
        let stripe = if (x / 9 + y / 13) % 2 == 0 { 30 } else { 0 };
        let mut c = [60 + n as u8 / 2 + stripe, 120 + n as u8, 50 + stripe, 255];
        let (dx, dy) = (x as f32 - DISK.0, y as f32 - DISK.1);
        if dx * dx + dy * dy <= DISK.2 * DISK.2 {
            c = [220, 30 + (n as u8 / 4), 40, 255];
        }
        if x >= SQUARE.0 && x < SQUARE.2 && y >= SQUARE.1 && y < SQUARE.3 {
            c = [30, 60, 210 + (n as u8 / 8), 255];
        }
        *px = c;
    }
    img
}

/// Sitzung mit der Testbitmap an (`l`, `t`); liefert die ID.
fn setup(l: i32, t: i32) -> (Session, ElementId) {
    ai::fp_ai::download::disable();
    let mut s = doc_session(800.0, 600.0);
    s.clipboard.clear();
    s.clipboard.pixels = Some(test_image(l, t));
    let v = ex(&mut s, "edit.paste", json!({}));
    let id = ElementId(v["ids"][0].as_u64().unwrap_or(0));
    (s, id)
}

fn sel_bounds(s: &Session) -> Option<IRect> {
    s.active()?.selection.pixels.as_ref().map(|m| m.tight_bounds()).filter(|b| !b.is_empty())
}

fn sel_at(s: &Session, x: i32, y: i32) -> u8 {
    s.active().and_then(|d| d.selection.pixels.as_ref().map(|m| m.get(x, y))).unwrap_or(0)
}

fn near(a: IRect, b: IRect, tol: i32) -> bool {
    (a.left - b.left).abs() <= tol && (a.top - b.top).abs() <= tol && (a.right - b.right).abs() <= tol && (a.bottom - b.bottom).abs() <= tol
}

fn disk_rect(l: i32, t: i32) -> IRect {
    IRect::new(l + (DISK.0 - DISK.2) as i32, t + (DISK.1 - DISK.2) as i32, l + (DISK.0 + DISK.2) as i32 + 1, t + (DISK.1 + DISK.2) as i32 + 1)
}

fn history_len(s: &Session) -> usize {
    s.active().map(|d| d.history.len()).unwrap_or(0)
}

#[test]
fn object_click_selects_disk_in_document_coordinates() {
    need_models!();
    let (mut s, _) = setup(100, 50);
    let h0 = history_len(&s);
    let v = ex(&mut s, "select.aiObject", json!({"point": [100.0 + DISK.0, 50.0 + DISK.1]}));
    let b = sel_bounds(&s).expect("Auswahl");
    assert!(near(b, disk_rect(100, 50), 4), "Auswahl {b:?} statt {:?}", disk_rect(100, 50));
    assert_eq!(sel_at(&s, 100 + DISK.0 as i32, 50 + DISK.1 as i32), 255);
    assert_eq!(sel_at(&s, 100 + SQUARE.0 + 20, 50 + SQUARE.1 + 20), 0, "Quadrat nicht mit ausgewählt");
    assert!(v["iou"].as_f64().unwrap_or(0.0) > 0.7);
    assert_eq!(v["source"], "bitmap");
    // Auswahländerungen erzeugen keinen Verlaufsschritt
    assert_eq!(history_len(&s), h0);
    // Rahmen statt Punkt: Quadrat
    ex(&mut s, "select.aiObject", json!({"box": [100 + SQUARE.0 - 8, 50 + SQUARE.1 - 8, 100 + SQUARE.2 + 8, 50 + SQUARE.3 + 8]}));
    let b = sel_bounds(&s).expect("Auswahl");
    assert!(near(b, IRect::new(100 + SQUARE.0, 50 + SQUARE.1, 100 + SQUARE.2, 50 + SQUARE.3), 4), "{b:?}");
}

#[test]
fn moved_scaled_rotated_bitmap() {
    need_models!();
    let (mut s, id) = setup(0, 0);
    // verschieben
    select(&mut s, &[id]);
    ex(&mut s, "edit.move", json!({"dx": 220, "dy": 140}));
    ex(&mut s, "select.aiObject", json!({"point": [220.0 + DISK.0, 140.0 + DISK.1]}));
    let b = sel_bounds(&s).expect("Auswahl");
    assert!(near(b, disk_rect(220, 140), 4), "verschoben: {b:?}");
    // skalieren auf 150 %
    select(&mut s, &[id]);
    let bb = bounds(&s, id);
    ex(&mut s, "element.setBounds", json!({"x": bb.left, "y": bb.top, "width": bb.width() * 1.5, "height": bb.height() * 1.5}));
    let nb = bounds(&s, id);
    let (cx, cy) = (nb.left + DISK.0 * 1.5, nb.top + DISK.1 * 1.5);
    ex(&mut s, "select.aiObject", json!({"point": [cx, cy]}));
    let b = sel_bounds(&s).expect("Auswahl");
    let r = DISK.2 * 1.5;
    let exp = IRect::new((cx - r) as i32, (cy - r) as i32, (cx + r) as i32, (cy + r) as i32);
    assert!(near(b, exp, 6), "skaliert: {b:?} statt {exp:?}");
    // drehen um 90° im Uhrzeigersinn: Scheibe wandert nach rechts oben
    select(&mut s, &[id]);
    ex(&mut s, "modify.transform.rotate90DegreeCw", json!({}));
    let rb = bounds(&s, id);
    // vorher (x, y) relativ zur linken oberen Ecke → nachher (H' − y, x)
    let (dx, dy) = (DISK.0 * 1.5, DISK.1 * 1.5);
    let (cx, cy) = (rb.left + (rb.width() - dy), rb.top + dx);
    ex(&mut s, "select.aiObject", json!({"point": [cx, cy]}));
    let b = sel_bounds(&s).expect("Auswahl");
    let exp = IRect::new((cx - r) as i32, (cy - r) as i32, (cx + r) as i32, (cy + r) as i32);
    assert!(near(b, exp, 6), "gedreht: {b:?} statt {exp:?}");
}

#[test]
fn modes_add_subtract_and_sessions() {
    need_models!();
    let (mut s, _) = setup(0, 0);
    let disk = [DISK.0, DISK.1];
    let sq = [(SQUARE.0 + SQUARE.2) as f32 / 2.0, (SQUARE.1 + SQUARE.3) as f32 / 2.0];
    // ausdrückliche Modi
    ex(&mut s, "select.aiObject", json!({"point": disk}));
    ex(&mut s, "select.aiObject", json!({"point": sq, "mode": "add"}));
    assert_eq!(sel_at(&s, DISK.0 as i32, DISK.1 as i32), 255);
    assert_eq!(sel_at(&s, sq[0] as i32, sq[1] as i32), 255);
    ex(&mut s, "select.aiObject", json!({"point": disk, "mode": "subtract"}));
    assert_eq!(sel_at(&s, DISK.0 as i32, DISK.1 as i32), 0, "Scheibe abgezogen");
    assert_eq!(sel_at(&s, sq[0] as i32, sq[1] as i32), 255, "Quadrat bleibt");
    // Sitzung wie das Werkzeug: Klick, Umschalt+Klick (Punkt hinzu), Alt+Klick (Punkt ab)
    let v = ex(&mut s, "select.aiObject", json!({"points": [[disk[0], disk[1], 1]], "mode": "replace"}));
    let tok = v["session"].as_u64().expect("Sitzung");
    let v = ex(&mut s, "select.aiObject", json!({"points": [[sq[0], sq[1], 1]], "session": tok, "refine": true}));
    assert_eq!(v["session"].as_u64(), Some(tok));
    assert_eq!(v["points"], 2);
    let both = s.active().and_then(|d| d.selection.pixels.clone()).expect("Auswahl");
    assert!(both.get(DISK.0 as i32, DISK.1 as i32) > 200 && both.get(sq[0] as i32, sq[1] as i32) > 200, "beide Objekte");
    // 10× derselbe Umschalt-Klick: keine doppelten Punkte, gleiches Ergebnis
    for _ in 0..10 {
        let v = ex(&mut s, "select.aiObject", json!({"points": [[sq[0], sq[1], 1]], "session": tok, "refine": true}));
        assert_eq!(v["points"], 2);
    }
    assert_eq!(s.active().and_then(|d| d.selection.pixels.clone()).as_ref(), Some(&both));
    // Alt-Klick auf das Quadrat (negativer Punkt): Quadrat fällt heraus
    let v = ex(&mut s, "select.aiObject", json!({"points": [[sq[0], sq[1], 0]], "session": tok, "refine": true}));
    assert_eq!(v["points"], 2, "Punkt umgedreht statt verdoppelt");
    assert_eq!(sel_at(&s, sq[0] as i32, sq[1] as i32), 0);
    assert!(sel_at(&s, DISK.0 as i32, DISK.1 as i32) > 200);
    // Auswahl zwischendurch geändert → Sitzung wird nicht fortgesetzt, sondern hinzugefügt
    ex(&mut s, "select.none", json!({}));
    let v = ex(&mut s, "select.aiObject", json!({"points": [[sq[0], sq[1], 1]], "session": tok, "refine": true}));
    assert_eq!(v["points"], 1);
    assert_eq!(sel_at(&s, sq[0] as i32, sq[1] as i32), 255);
}

#[test]
fn tool_requests_and_modifiers() {
    need_models!();
    let (mut s, _) = setup(0, 0);
    s.set_tool(ToolId::AiObject).expect("Werkzeug");
    let click = |s: &mut Session, x: f32, y: f32, shift: bool, alt: bool| -> Value {
        let mut d = ToolEvent::down(x, y);
        let mut u = ToolEvent::up(x, y);
        d.mods.shift = shift;
        u.mods.shift = shift;
        d.mods.alt = alt;
        u.mods.alt = alt;
        s.tool_event(&d).expect("down");
        let r = s.tool_event(&u).expect("up");
        let inv = r.ui_request.expect("Anfrage an die Oberfläche");
        assert_eq!(inv.command, "select.aiObject");
        // so wie die Oberfläche: Befehl nach dem Vorwärmen ausführen
        if let Ok(Some(p)) = ai::prepare(s, &inv.command, &inv.params) {
            ai::run_prepared(&p, &ai::fp_ai::Progress::new()).expect("vorwärmen");
        }
        assert!(matches!(ai::prepare(s, &inv.command, &inv.params), Ok(None)), "nach dem Vorwärmen ist nichts mehr zu rechnen");
        ex(s, &inv.command, inv.params.clone())
    };
    let sq = ((SQUARE.0 + SQUARE.2) as f32 / 2.0, (SQUARE.1 + SQUARE.3) as f32 / 2.0);
    // wie der Zauberstab: jeder Klick ist eine eigene Auswahl, Modifikatoren verknüpfen
    click(&mut s, DISK.0, DISK.1, false, false);
    let disk_only = s.active().and_then(|d| d.selection.pixels.clone());
    click(&mut s, sq.0, sq.1, true, false);
    assert!(sel_at(&s, sq.0 as i32, sq.1 as i32) > 200 && sel_at(&s, DISK.0 as i32, DISK.1 as i32) > 200, "Umschalt-Klick fügt hinzu");
    click(&mut s, sq.0, sq.1, false, true);
    assert_eq!(sel_at(&s, sq.0 as i32, sq.1 as i32), 0, "Alt-Klick zieht ab");
    assert!(sel_at(&s, DISK.0 as i32, DISK.1 as i32) > 200);
    let tb = |m: Option<fp_raster::Mask>| m.map(|m| m.tight_bounds());
    assert_eq!(tb(s.active().and_then(|d| d.selection.pixels.clone())), tb(disk_only), "Hinzufügen und Abziehen desselben Objekts heben sich auf");
    // Umschalt+Alt: Schnittmenge (Scheibe ∩ Quadrat = leer)
    click(&mut s, sq.0, sq.1, true, true);
    assert_eq!(sel_at(&s, DISK.0 as i32, DISK.1 as i32), 0, "Schnittmenge");
    // einfacher Klick: ersetzt
    click(&mut s, DISK.0, DISK.1, false, false);
    click(&mut s, sq.0, sq.1, false, false);
    assert!(sel_at(&s, sq.0 as i32, sq.1 as i32) > 200);
    assert_eq!(sel_at(&s, DISK.0 as i32, DISK.1 as i32), 0, "einfacher Klick ersetzt die Auswahl");
    // Rahmen aufziehen
    s.tool_event(&ToolEvent::down(DISK.0 - 70.0, DISK.1 - 70.0)).expect("down");
    s.tool_event(&ToolEvent::drag(DISK.0, DISK.1)).expect("drag");
    s.tool_event(&ToolEvent::drag(DISK.0 + 70.0, DISK.1 + 70.0)).expect("drag");
    let r = s.tool_event(&ToolEvent::up(DISK.0 + 70.0, DISK.1 + 70.0)).expect("up");
    let inv = r.ui_request.expect("Anfrage");
    assert!(inv.params.get("box").is_some());
}

#[test]
fn subject_and_refine_edge_with_mask_output_undo_redo() {
    need_models!();
    let (mut s, id) = setup(50, 40);
    ex(&mut s, "select.aiSubject", json!({}));
    let b = sel_bounds(&s).expect("Motiv");
    let img = IRect::from_xywh(50, 40, W, H);
    assert!(b.left >= img.left && b.top >= img.top && b.right <= img.right && b.bottom <= img.bottom, "Motiv innerhalb der Bitmap: {b:?}");
    // Kante verfeinern als weiche Auswahl
    ex(&mut s, "select.aiObject", json!({"point": [50.0 + DISK.0, 40.0 + DISK.1]}));
    let before = s.active().and_then(|d| d.selection.pixels.clone()).expect("Auswahl");
    let h0 = history_len(&s);
    ex(&mut s, "select.aiRefineEdge", json!({"output": "selection"}));
    let after = s.active().and_then(|d| d.selection.pixels.clone()).expect("verfeinert");
    assert_eq!(history_len(&s), h0, "Auswahl: kein Verlaufsschritt");
    assert!(after.get(50 + DISK.0 as i32, 40 + DISK.1 as i32) > 240, "Inneres bleibt");
    assert!(near(after.tight_bounds(), before.tight_bounds(), 12));
    // als Bitmapmaske: genau ein Verlaufsschritt
    let doc_before = s.active().map(|d| d.doc.clone()).expect("Dokument");
    ex(&mut s, "select.aiRefineEdge", json!({"output": "mask"}));
    assert_eq!(history_len(&s), h0 + 1);
    let is_mask_group = |s: &Session| top_elements(s).iter().any(|e| matches!(&e.kind, fp_model::element::ElementKind::Group(g) if g.kind.is_mask()));
    assert!(is_mask_group(&s), "Maskengruppe angelegt");
    let _ = el(&s, id);
    assert!(!s.active().map(|d| d.selection.has_pixels()).unwrap_or(true), "Pixelauswahl in die Maske übernommen");
    s.undo().expect("rückgängig");
    assert!(std::sync::Arc::ptr_eq(&doc_before, &s.active().expect("Dokument").doc), "Rückgängig stellt exakt wieder her");
    assert!(!is_mask_group(&s));
    s.redo().expect("wiederholen");
    assert!(is_mask_group(&s));
    assert_eq!(history_len(&s), h0 + 1);
}

#[test]
fn cancel_midway_keeps_document_and_selection() {
    need_models!();
    ai::fp_ai::cache::clear_results();
    let (mut s, _) = setup(10, 10);
    ex(&mut s, "select.pixels", json!({"shape": "rect", "rect": [20, 20, 60, 60]}));
    let sel0 = s.active().and_then(|d| d.selection.pixels.clone());
    let doc0 = s.active().map(|d| d.doc.clone()).expect("Dokument");
    // anderes Bild als in den übrigen Tests (sonst wäre der Zwischenspeicher warm)
    let p = ai::prepare(&s, "select.aiSubject", &json!({"region": [11, 11, 409, 309]})).expect("vorbereiten").expect("kalt");
    let job = ai::fp_ai::job::Job::spawn("Test", move |pr| ai::run_prepared(&p, pr));
    std::thread::sleep(std::time::Duration::from_millis(150));
    job.cancel();
    assert!(matches!(job.wait(), Err(ai::fp_ai::AiError::Cancelled)));
    assert_eq!(s.active().and_then(|d| d.selection.pixels.clone()), sel0);
    assert!(std::sync::Arc::ptr_eq(&doc0, &s.active().expect("Dokument").doc));
    // danach ist nichts halb im Zwischenspeicher: erneut kalt
    assert!(ai::prepare(&s, "select.aiSubject", &json!({"region": [11, 11, 409, 309]})).expect("vorbereiten").is_some());
}

#[test]
fn pages_hidden_and_locked_layers() {
    need_models!();
    let (mut s, id) = setup(0, 0);
    // zweite Seite mit eigener Bitmap an anderer Stelle
    ex(&mut s, "page.add", json!({}));
    let p2 = s.active().map(|d| d.page).unwrap_or(0);
    assert_eq!(p2, 1);
    s.clipboard.clear();
    s.clipboard.pixels = Some(test_image(300, 250));
    ex(&mut s, "edit.paste", json!({}));
    ex(&mut s, "select.none", json!({}));
    ex(&mut s, "select.aiObject", json!({"point": [300.0 + DISK.0, 250.0 + DISK.1]}));
    let b = sel_bounds(&s).expect("Seite 2");
    assert!(near(b, disk_rect(300, 250), 4), "{b:?}");
    // zurück auf Seite 1
    ex(&mut s, "page.setCurrent", json!({"index": 0}));
    ex(&mut s, "select.aiObject", json!({"point": [DISK.0, DISK.1]}));
    assert!(near(sel_bounds(&s).expect("Seite 1"), disk_rect(0, 0), 4));
    // ausgeblendete Bitmap: nicht als Quelle, die Seite ohne sie hat dort keine Scheibe
    select(&mut s, &[id]);
    ex(&mut s, "element.setVisible", json!({"visible": false}));
    ex(&mut s, "select.none", json!({}));
    let r = s.execute("select.aiObject", json!({"point": [DISK.0, DISK.1], "source": "bitmap"}));
    assert!(r.is_err(), "ausgeblendete Bitmap ist keine Quelle");
    let v = ex(&mut s, "select.aiObject", json!({"point": [DISK.0, DISK.1]}));
    assert_eq!(v["source"], "page");
    let nb = sel_bounds(&s);
    assert!(nb.is_none_or(|b| !near(b, disk_rect(0, 0), 4)), "keine Scheibe aus der ausgeblendeten Bitmap");
    // gesperrte Bitmap: Auswahl aus der Seitenansicht geht, Maskenausgabe nicht
    select(&mut s, &[id]);
    ex(&mut s, "element.setVisible", json!({"visible": true}));
    ex(&mut s, "element.setLocked", json!({"locked": true, "ids": [id.0]}));
    ex(&mut s, "select.none", json!({}));
    let v = ex(&mut s, "select.aiObject", json!({"point": [DISK.0, DISK.1]}));
    assert_eq!(v["source"], "page");
    assert!(near(sel_bounds(&s).expect("Auswahl"), disk_rect(0, 0), 4));
    let h0 = history_len(&s);
    assert!(s.execute("select.aiRefineEdge", json!({"output": "mask", "source": "bitmap"})).is_err());
    assert_eq!(history_len(&s), h0);
}

#[test]
fn registered_with_catalog_data() {
    let s = Session::default();
    for n in ["select.aiObject", "select.aiSubject", "select.aiRefineEdge", "ai.models", "ai.setModelFolder", "ai.clearCache", "ai.manageModels"] {
        let c = s.registry().get(n).unwrap_or_else(|| panic!("{n} fehlt"));
        assert!(c.info().implemented, "{n}");
        assert!(!c.info().label.is_empty(), "{n} ohne Beschriftung");
    }
    for t in [ToolId::AiObject, ToolId::AiSubject, ToolId::AiRefineEdge] {
        let i = fp_engine::tools::info(t).expect("Werkzeug");
        assert_eq!(i.flyout, "Zauberstab");
        assert!(fp_engine::tools::flyout_members("Zauberstab").contains(&t));
    }
    assert_eq!(fp_engine::tools::flyout_members("Zauberstab")[0], ToolId::MagicWand, "Zauberstab bleibt Vorgabe des Platzes");
}
