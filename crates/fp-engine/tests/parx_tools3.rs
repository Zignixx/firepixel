//! PAR-X-TOOLS-3: Zeiger/Transformieren/Zuschneiden, Linie/Stift/Vektorpfad, Rechteck/Ellipse/Polygon
//! nach den Messwerten des Originals (Einheiten 08–10, `docs\parity\tool-pointer-transform-crop.md`,
//! `tool-line-pen.md`, `tool-shapes.md`). Sollwerte aus `re\_testdateien\par\px\<einheit>\paths.json`
//! (Auszug ohne Oberfläche in `tests\data\parx-tools3\expected.json`). Dazu die Fälle aus
//! AGENTEN_REGELN §4b (Wiederholung, Abbrechen, Rückgängig/Wiederholen, Auswahlwechsel).

use crate::common;
use common::*;
use fp_engine::tools::ToolId;
use fp_engine::{Session, ToolEvent, ToolKey};
use fp_geom::{Point, Rect};
use fp_model::{Contour, ElementId, ElementKind};
use serde_json::{json, Value};

// ------------------------------------------------------------------ Hilfen

fn expected() -> Value {
    let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("data").join("parx-tools3").join("expected.json");
    serde_json::from_str(&std::fs::read_to_string(p).expect("expected.json")).expect("JSON")
}

/// Rechteck des Sollwerts `[l, t, r, b]`.
fn exp_rect(v: &Value) -> Rect {
    let f = |i: usize| v[i].as_f64().expect("Zahl") as f32;
    Rect::new(f(0), f(1), f(2), f(3))
}

fn tool(s: &mut Session, t: ToolId) {
    s.set_tool(t).unwrap_or_else(|e| panic!("{e}"));
}

/// Zug in 30 Schritten wie die Messung (mit Modifikatoren).
fn drag_m(s: &mut Session, a: (f32, f32), b: (f32, f32), m: impl Fn(ToolEvent) -> ToolEvent) {
    events(s, &[m(ToolEvent::down(a.0, a.1))]);
    for i in 1..=30 {
        let t = i as f32 / 30.0;
        events(s, &[m(ToolEvent::drag(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t))]);
    }
    events(s, &[m(ToolEvent::up(b.0, b.1))]);
}

fn drag(s: &mut Session, a: (f32, f32), b: (f32, f32)) {
    drag_m(s, a, b, |e| e);
}

fn click(s: &mut Session, x: f32, y: f32) {
    events(s, &[ToolEvent::down(x, y), ToolEvent::up(x, y)]);
}

fn key(s: &mut Session, k: ToolKey) {
    events(s, &[ToolEvent::key(k)]);
}

fn label(s: &Session) -> Option<String> {
    s.active().and_then(|d| d.history.undo_label().map(|l| l.to_string()))
}

fn undo_count(s: &Session) -> usize {
    s.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

fn sel(s: &Session) -> Vec<ElementId> {
    s.active().map(|d| d.selection.elements.clone()).unwrap_or_default()
}

fn contours(s: &Session, id: ElementId) -> Vec<Contour> {
    match &el(s, id).kind {
        ElementKind::Path(p) => p.contours.clone(),
        _ => panic!("kein Pfad"),
    }
}

fn anchors(s: &Session, id: ElementId) -> Vec<Point> {
    contours(s, id)[0].nodes.iter().map(|n| n.anchor).collect()
}

fn near(a: Point, x: f32, y: f32, tol: f32) -> bool {
    (a.x - x).abs() <= tol && (a.y - y).abs() <= tol
}

fn rect_near(a: Rect, b: Rect, tol: f32) -> bool {
    (a.left - b.left).abs() <= tol && (a.top - b.top).abs() <= tol && (a.right - b.right).abs() <= tol && (a.bottom - b.bottom).abs() <= tol
}

/// Gewöhnlicher Pfad (keine Rechteck-Grundform) wie R1 in T04.
fn plain_rect(s: &mut Session, l: f32, t: f32, r: f32, b: f32, name: &str) -> ElementId {
    let v = ex(s, "draw.path", json!({"contours": [{"closed": true, "nodes": [[l, t], [r, t], [r, b], [l, b]]}], "name": name}));
    ElementId(v["id"].as_u64().expect("id"))
}

/// Offener Kurvenpfad P1 aus T04 (Knoten wie in `paths.json`).
fn p1(s: &mut Session) -> ElementId {
    let v = ex(
        s,
        "draw.path",
        json!({"contours": [{"closed": false, "nodes": [
            {"anchor": [200, 200], "in": [200, 200], "out": [240, 160]},
            {"anchor": [290, 200], "in": [260, 160], "out": [320, 240]},
            {"anchor": [370, 200], "in": [340, 240], "out": [380, 180]},
            {"anchor": [420, 240], "in": [400, 200], "out": [420, 240]}]}], "name": "P1", "fill": false}),
    );
    ElementId(v["id"].as_u64().expect("id"))
}

fn session() -> Session {
    let mut s = doc_session(800.0, 600.0);
    s.set_colors(Some(fp_color::Argb(0xffc0_4040)), Some(fp_color::Argb(0xff80_8080)));
    s
}

/// T04-Ausgangslage: R1 gewählt.
fn r1_session() -> (Session, ElementId) {
    let mut s = session();
    let r1 = plain_rect(&mut s, 50.0, 50.0, 150.0, 120.0, "R1");
    select(&mut s, &[r1]);
    (s, r1)
}

// ------------------------------------------------------------------ 08 Transformieren

#[test]
fn scale_corner_projects_on_diagonal() {
    let e = expected();
    for (state, alt) in [("c10_scale_free", false), ("c10_scale_alt", true)] {
        let (mut s, r1) = r1_session();
        tool(&mut s, ToolId::Scale);
        drag_m(&mut s, (150.0, 120.0), (200.0, 140.0), |ev| if alt { ev.alt() } else { ev });
        key(&mut s, ToolKey::Enter);
        let want = exp_rect(&e["unit08"][state]);
        assert!(rect_near(bounds(&s, r1), want, 0.6), "{state}: {:?} statt {want:?}", bounds(&s, r1));
        assert_eq!(label(&s).as_deref(), Some("Transformieren"));
        // §4b: Rückgängig stellt R1 wieder her, Wiederholen das Ergebnis
        s.undo().expect("undo");
        assert!(rect_near(bounds(&s, r1), Rect::new(50.0, 50.0, 150.0, 120.0), 0.01));
        s.redo().expect("redo");
        assert!(rect_near(bounds(&s, r1), want, 0.6));
    }
    // Umschalt ändert nichts (schon proportional)
    let (mut s, r1) = r1_session();
    tool(&mut s, ToolId::Scale);
    drag_m(&mut s, (150.0, 120.0), (200.0, 140.0), |ev| ev.shift());
    key(&mut s, ToolKey::Enter);
    assert!(rect_near(bounds(&s, r1), exp_rect(&e["unit08"]["c10_scale_shift"]), 0.6));
}

#[test]
fn scale_inside_drag_moves() {
    let e = expected();
    let (mut s, r1) = r1_session();
    tool(&mut s, ToolId::Scale);
    drag(&mut s, (80.0, 70.0), (120.0, 80.0));
    key(&mut s, ToolKey::Enter);
    assert!(rect_near(bounds(&s, r1), exp_rect(&e["unit08"]["c10_scale_inside_drag"]), 0.01), "{:?}", bounds(&s, r1));
}

#[test]
fn transform_esc_discards_without_step() {
    for t in [ToolId::Scale, ToolId::Skew, ToolId::Distort] {
        let (mut s, r1) = r1_session();
        tool(&mut s, t);
        let n = undo_count(&s);
        drag(&mut s, (150.0, 120.0), (200.0, 140.0));
        key(&mut s, ToolKey::Escape);
        assert!(rect_near(bounds(&s, r1), Rect::new(50.0, 50.0, 150.0, 120.0), 0.01), "{t:?}: Esc verwirft");
        assert_eq!(undo_count(&s), n, "{t:?}: kein Verlaufsschritt");
        assert_eq!(s.tools.active, ToolId::Pointer);
        assert_eq!(sel(&s), vec![r1], "Auswahl bleibt");
    }
}

#[test]
fn rotate_and_distort_are_named_transform() {
    let (mut s, _) = r1_session();
    tool(&mut s, ToolId::Scale);
    drag(&mut s, (170.0, 140.0), (100.0, 160.0));
    key(&mut s, ToolKey::Enter);
    assert_eq!(label(&s).as_deref(), Some("Transformieren"));
    let (mut s, _) = r1_session();
    tool(&mut s, ToolId::Distort);
    drag(&mut s, (150.0, 50.0), (170.0, 30.0));
    key(&mut s, ToolKey::Enter);
    assert_eq!(label(&s).as_deref(), Some("Transformieren"));
}

#[test]
fn skew_moves_only_the_grabbed_edge() {
    let e = expected();
    // Mittelgriff oben +30: nur die obere Kante
    let (mut s, r1) = r1_session();
    tool(&mut s, ToolId::Skew);
    drag(&mut s, (100.0, 50.0), (130.0, 50.0));
    key(&mut s, ToolKey::Enter);
    let a = anchors(&s, r1);
    assert!(a.iter().any(|p| near(*p, 80.0, 50.0, 0.6)) && a.iter().any(|p| near(*p, 180.0, 50.0, 0.6)), "{a:?}");
    assert!(a.iter().any(|p| near(*p, 50.0, 120.0, 0.6)) && a.iter().any(|p| near(*p, 150.0, 120.0, 0.6)), "unten bleibt: {a:?}");
    assert!(rect_near(bounds(&s, r1), exp_rect(&e["unit08"]["c13_skew_topmid"]), 0.6));
    // Ecke senkrecht −20: rechte Kante spiegelbildlich
    let (mut s, r1) = r1_session();
    tool(&mut s, ToolId::Skew);
    drag(&mut s, (150.0, 50.0), (150.0, 30.0));
    key(&mut s, ToolKey::Enter);
    let a = anchors(&s, r1);
    assert!(a.iter().any(|p| near(*p, 150.0, 30.0, 0.6)) && a.iter().any(|p| near(*p, 150.0, 140.0, 0.6)), "{a:?}");
    assert!(a.iter().any(|p| near(*p, 50.0, 50.0, 0.6)) && a.iter().any(|p| near(*p, 50.0, 120.0, 0.6)), "links bleibt: {a:?}");
    assert!(rect_near(bounds(&s, r1), exp_rect(&e["unit08"]["c13_skew_corner"]), 0.6));
    // Ecke waagrecht +20: obere Kante symmetrisch breiter
    let (mut s, r1) = r1_session();
    tool(&mut s, ToolId::Skew);
    drag(&mut s, (150.0, 50.0), (170.0, 50.0));
    key(&mut s, ToolKey::Enter);
    let a = anchors(&s, r1);
    assert!(a.iter().any(|p| near(*p, 30.0, 50.0, 0.6)) && a.iter().any(|p| near(*p, 170.0, 50.0, 0.6)), "{a:?}");
    assert!(rect_near(bounds(&s, r1), exp_rect(&e["unit08"]["c13_skew_corner_h"]), 0.6));
}

#[test]
fn distort_edge_moves_both_corners() {
    let e = expected();
    let (mut s, r1) = r1_session();
    tool(&mut s, ToolId::Distort);
    drag(&mut s, (100.0, 50.0), (120.0, 40.0));
    key(&mut s, ToolKey::Enter);
    let a = anchors(&s, r1);
    assert!(a.iter().any(|p| near(*p, 70.0, 40.0, 0.6)) && a.iter().any(|p| near(*p, 170.0, 40.0, 0.6)), "{a:?}");
    assert!(rect_near(bounds(&s, r1), exp_rect(&e["unit08"]["c14_distort_side"]), 0.6));
}

#[test]
fn bitmap_scale_projection() {
    let e = expected();
    let mut s = session();
    let b1 = bitmap(&mut s, 40, 40, 480, 320, [10, 120, 200, 255]);
    select(&mut s, &[b1]);
    tool(&mut s, ToolId::Scale);
    drag(&mut s, (520.0, 360.0), (600.0, 420.0));
    key(&mut s, ToolKey::Enter);
    assert!(rect_near(bounds(&s, b1), exp_rect(&e["unit08"]["x16_bitmap_scale"]), 1.0), "{:?}", bounds(&s, b1));
}

// ------------------------------------------------------------------ 08 Zeiger

#[test]
fn pointer_corner_point_moves_object() {
    let e = expected();
    let (mut s, r1) = r1_session();
    tool(&mut s, ToolId::Pointer);
    drag(&mut s, (50.0, 50.0), (90.0, 80.0));
    assert!(rect_near(bounds(&s, r1), exp_rect(&e["unit08"]["c04_corner_r1"]), 0.01), "{:?}", bounds(&s, r1));
    assert_eq!(label(&s).as_deref(), Some("Verschieben"));
    // Knoten eines offenen Pfads: ganzer Pfad wandert
    let mut s = session();
    let p = p1(&mut s);
    select(&mut s, &[p]);
    drag(&mut s, (290.0, 200.0), (290.0, 230.0));
    let a = anchors(&s, p);
    assert!(near(a[0], 200.0, 230.0, 0.01) && near(a[3], 420.0, 270.0, 0.01), "{a:?}");
}

#[test]
fn pointer_shift_drag_on_selected_constrains() {
    let e = expected();
    let (mut s, r1) = r1_session();
    tool(&mut s, ToolId::Pointer);
    drag_m(&mut s, (100.0, 85.0), (160.0, 105.0), |ev| ev.shift());
    assert!(rect_near(bounds(&s, r1), exp_rect(&e["unit08"]["c03_drag_shift_60_20"]), 0.01), "{:?}", bounds(&s, r1));
    assert_eq!(sel(&s), vec![r1], "bleibt gewählt");
    // Umschalt-Klick ohne Ziehen wählt ab
    events(&mut s, &[ToolEvent::down(150.0, 85.0).shift(), ToolEvent::up(150.0, 85.0).shift()]);
    assert!(sel(&s).is_empty());
}

#[test]
fn pointer_alt_drag_is_named_duplicate() {
    let (mut s, r1) = r1_session();
    tool(&mut s, ToolId::Pointer);
    drag_m(&mut s, (100.0, 85.0), (100.0, 135.0), |ev| ev.alt());
    assert_eq!(top_elements(&s).len(), 2);
    assert_eq!(label(&s).as_deref(), Some("Duplizieren"));
    assert!(rect_near(bounds(&s, r1), Rect::new(50.0, 50.0, 150.0, 120.0), 0.01), "Original bleibt");
}

#[test]
fn subselect_point_drag_is_named_move_point() {
    let mut s = session();
    let p = p1(&mut s);
    select(&mut s, &[p]);
    tool(&mut s, ToolId::Subselection);
    click(&mut s, 290.0, 200.0);
    drag(&mut s, (290.0, 200.0), (290.0, 230.0));
    assert!(near(anchors(&s, p)[1], 290.0, 230.0, 0.01));
    assert_eq!(label(&s).as_deref(), Some("Punkt verschieben"));
}

#[test]
fn pointer_drag_holds_inspector_for_fresh_selection() {
    let (mut s, r1) = r1_session();
    select(&mut s, &[]);
    tool(&mut s, ToolId::Pointer);
    events(&mut s, &[ToolEvent::down(100.0, 85.0), ToolEvent::drag(110.0, 90.0), ToolEvent::drag(120.0, 95.0)]);
    assert!(s.tool_inspector_hold(), "Inspektor bleibt beim Dokument");
    events(&mut s, &[ToolEvent::up(120.0, 95.0)]);
    assert!(!s.tool_inspector_hold());
    assert_eq!(sel(&s), vec![r1]);
    // bereits gewählt: kein Halten
    events(&mut s, &[ToolEvent::down(120.0, 95.0), ToolEvent::drag(130.0, 95.0), ToolEvent::drag(140.0, 95.0)]);
    assert!(!s.tool_inspector_hold());
    events(&mut s, &[ToolEvent::up(140.0, 95.0)]);
}

// ------------------------------------------------------------------ 08 Zuschneiden

/// T04-ähnlich: R1, R2, E1, P1, ST ganz rechts außen (654..846).
fn crop_session() -> (Session, ElementId, ElementId) {
    let mut s = session();
    let r1 = plain_rect(&mut s, 50.0, 50.0, 150.0, 120.0, "R1");
    plain_rect(&mut s, 200.0, 50.0, 300.0, 120.0, "R2");
    let st = plain_rect(&mut s, 654.0, 205.0, 846.0, 386.0, "ST");
    select(&mut s, &[]);
    (s, r1, st)
}

#[test]
fn crop_deletes_outside_and_selects_all() {
    let (mut s, r1, st) = crop_session();
    tool(&mut s, ToolId::Crop);
    drag(&mut s, (100.0, 100.0), (500.0, 400.0));
    assert_eq!(s.tool_live_rect().map(|r| r.normalized()), Some(Rect::new(100.0, 100.0, 500.0, 400.0)), "Inspektor live");
    key(&mut s, ToolKey::Enter);
    let d = s.active().expect("d");
    assert_eq!((d.doc.pages[0].canvas.width, d.doc.pages[0].canvas.height), (400.0, 300.0));
    assert!(fp_engine::docops::element_on_page(&d.doc, 0, st).is_none(), "ST ganz außerhalb gelöscht");
    assert!(rect_near(bounds(&s, r1), Rect::new(-50.0, -50.0, 50.0, 20.0), 0.01));
    assert_eq!(sel(&s).len(), 2, "alle Objekte gewählt");
    assert_eq!(label(&s).as_deref(), Some("Dokument zuschneiden"));
    assert_eq!(s.tools.active, ToolId::Crop);
    // §4b: Rückgängig stellt alles wieder her
    s.undo().expect("undo");
    assert!(s.active().map(|d| fp_engine::docops::element_on_page(&d.doc, 0, st).is_some()).unwrap_or(false));
}

#[test]
fn crop_frame_is_limited_to_canvas() {
    let (mut s, r1, _) = crop_session();
    tool(&mut s, ToolId::Crop);
    drag(&mut s, (-50.0, -50.0), (300.0, 200.0));
    key(&mut s, ToolKey::Enter);
    let d = s.active().expect("d");
    assert_eq!((d.doc.pages[0].canvas.width, d.doc.pages[0].canvas.height), (300.0, 200.0));
    assert!(rect_near(bounds(&s, r1), Rect::new(50.0, 50.0, 150.0, 120.0), 0.01), "keine Verschiebung");
}

#[test]
fn crop_trims_bitmaps() {
    let e = expected();
    let mut s = session();
    let b1 = bitmap(&mut s, 40, 40, 480, 320, [10, 120, 200, 255]);
    select(&mut s, &[b1]);
    tool(&mut s, ToolId::Crop);
    drag(&mut s, (100.0, 100.0), (300.0, 300.0));
    key(&mut s, ToolKey::Enter);
    assert!(rect_near(bounds(&s, b1), exp_rect(&e["unit08"]["k19_crop_b1sel"]), 0.01), "{:?}", bounds(&s, b1));
}

#[test]
fn export_area_has_live_rect_without_snapping() {
    let (mut s, _, _) = crop_session();
    tool(&mut s, ToolId::ExportArea);
    drag(&mut s, (50.0, 50.0), (300.0, 200.0));
    assert_eq!(s.tool_live_rect().map(|r| r.normalized()), Some(Rect::new(50.0, 50.0, 300.0, 200.0)));
}

// ------------------------------------------------------------------ 09 Linie, Stift, Vektorpfad

#[test]
fn line_nodes_on_pixel_centers_named_line() {
    let e = expected();
    let mut s = session();
    tool(&mut s, ToolId::Line);
    drag(&mut s, (100.0, 100.0), (300.0, 150.0));
    let id = sel(&s)[0];
    let a = anchors(&s, id);
    let want = &e["unit09"]["l01_line_free"];
    assert!(near(a[0], want[0].as_f64().unwrap_or(0.0) as f32, want[1].as_f64().unwrap_or(0.0) as f32, 0.001), "{a:?}");
    assert!(near(a[1], want[2].as_f64().unwrap_or(0.0) as f32, want[3].as_f64().unwrap_or(0.0) as f32, 0.001), "{a:?}");
    assert_eq!(el(&s, id).common.name.as_deref(), Some("Linie"));
    assert_eq!(label(&s).as_deref(), Some("Linie"));
    drag_m(&mut s, (100.0, 300.0), (200.0, 390.0), |ev| ev.shift());
    let a = anchors(&s, sel(&s)[0]);
    let want = &e["unit09"]["l01_line_shift_45"];
    assert!(near(a[1], want[2].as_f64().unwrap_or(0.0) as f32, want[3].as_f64().unwrap_or(0.0) as f32, 0.001), "{a:?}");
}

#[test]
fn pen_continues_after_esc_and_enter() {
    let mut s = session();
    tool(&mut s, ToolId::Pen);
    for (x, y) in [(450.0, 300.0), (550.0, 300.0), (550.0, 350.0)] {
        click(&mut s, x, y);
    }
    key(&mut s, ToolKey::Escape);
    assert_eq!(label(&s).as_deref(), Some("Stift"));
    let id = sel(&s)[0];
    for (x, y) in [(450.0, 450.0), (550.0, 450.0), (550.0, 500.0)] {
        click(&mut s, x, y);
    }
    key(&mut s, ToolKey::Enter);
    assert_eq!(top_elements(&s).len(), 1, "kein neuer Pfad");
    assert_eq!(anchors(&s, id).len(), 6);
    // 2 × Esc: ebenfalls fortgesetzt
    let mut s = session();
    tool(&mut s, ToolId::Pen);
    click(&mut s, 100.0, 100.0);
    click(&mut s, 200.0, 100.0);
    key(&mut s, ToolKey::Escape);
    key(&mut s, ToolKey::Escape);
    click(&mut s, 400.0, 100.0);
    click(&mut s, 500.0, 100.0);
    key(&mut s, ToolKey::Enter);
    assert_eq!(top_elements(&s).len(), 1);
    assert_eq!(anchors(&s, top_elements(&s)[0].id).len(), 4);
    // Abwählen (Strg+D) → neuer Pfad
    let mut s = session();
    tool(&mut s, ToolId::Pen);
    click(&mut s, 100.0, 100.0);
    click(&mut s, 200.0, 100.0);
    key(&mut s, ToolKey::Escape);
    ex(&mut s, "select.none", json!({}));
    click(&mut s, 400.0, 100.0);
    click(&mut s, 500.0, 100.0);
    key(&mut s, ToolKey::Enter);
    assert_eq!(top_elements(&s).len(), 2);
}

#[test]
fn pen_double_click_ends_for_good() {
    let mut s = session();
    tool(&mut s, ToolId::Pen);
    click(&mut s, 100.0, 100.0);
    events(&mut s, &[ToolEvent::down(200.0, 100.0), ToolEvent::up(200.0, 100.0), ToolEvent::double(200.0, 100.0)]);
    click(&mut s, 400.0, 100.0);
    click(&mut s, 500.0, 100.0);
    key(&mut s, ToolKey::Enter);
    assert_eq!(top_elements(&s).len(), 2);
}

#[test]
fn pen_continues_unselected_path_at_endpoint() {
    let mut s = session();
    let p = p1(&mut s);
    select(&mut s, &[]);
    tool(&mut s, ToolId::Pen);
    click(&mut s, 420.0, 240.0);
    click(&mut s, 480.0, 280.0);
    key(&mut s, ToolKey::Enter);
    assert_eq!(top_elements(&s).len(), 1);
    let a = anchors(&s, p);
    assert_eq!(a.len(), 5);
    assert!(near(a[4], 480.0, 280.0, 0.01));
    assert_eq!(sel(&s), vec![p]);
}

#[test]
fn pen_alt_drag_on_last_point_changes_its_out_handle() {
    let mut s = session();
    tool(&mut s, ToolId::Pen);
    click(&mut s, 100.0, 450.0);
    events(&mut s, &[ToolEvent::down(200.0, 450.0), ToolEvent::drag(200.0, 452.0), ToolEvent::drag(200.0, 455.0), ToolEvent::up(200.0, 455.0)]);
    events(&mut s, &[ToolEvent::down(200.0, 450.0).alt(), ToolEvent::drag(200.0, 480.0).alt(), ToolEvent::drag(200.0, 520.0).alt(), ToolEvent::up(200.0, 520.0).alt()]);
    click(&mut s, 300.0, 450.0);
    key(&mut s, ToolKey::Enter);
    let c = contours(&s, sel(&s)[0]);
    assert_eq!(c[0].nodes.len(), 3, "kein zweiter Knoten");
    let n = &c[0].nodes[1];
    assert!(near(n.pred, 200.0, 445.0, 0.01) && near(n.succ, 200.0, 520.0, 0.01), "{n:?}");
}

#[test]
fn pen_click_on_curve_point_converts_to_corner() {
    let mut s = session();
    let p = p1(&mut s);
    select(&mut s, &[p]);
    tool(&mut s, ToolId::Pen);
    click(&mut s, 370.0, 200.0);
    let c = contours(&s, p);
    assert_eq!(c[0].nodes.len(), 4, "nicht gelöscht");
    let n = &c[0].nodes[2];
    assert!(n.pred == n.anchor && n.succ == n.anchor, "Henkel eingezogen");
    assert_eq!(label(&s).as_deref(), Some("Stift"));
    // Klick auf ein Segment fügt ein, Schritt „Stift“
    let mut s = session();
    let p = p1(&mut s);
    select(&mut s, &[p]);
    tool(&mut s, ToolId::Pen);
    let mid = {
        let c = contours(&s, p);
        let bc = fp_engine::geometry::to_bezier(&c[0]);
        bc.segments().next().map(|seg| seg.eval(0.5).to_point()).expect("Segment")
    };
    click(&mut s, mid.x, mid.y);
    assert_eq!(contours(&s, p)[0].nodes.len(), 5);
    assert_eq!(label(&s).as_deref(), Some("Stift"));
}

#[test]
fn pen_does_not_snap_to_objects() {
    let mut s = session();
    plain_rect(&mut s, 50.0, 50.0, 150.0, 120.0, "R1");
    ex(&mut s, "draw.ellipse", json!({"rect": [50, 170, 150, 240]}));
    select(&mut s, &[]);
    tool(&mut s, ToolId::Pen);
    click(&mut s, 152.0, 122.0);
    click(&mut s, 300.0, 300.0);
    key(&mut s, ToolKey::Enter);
    let a = anchors(&s, sel(&s)[0]);
    assert!(near(a[0], 152.0, 122.0, 0.001), "{a:?}");
    select(&mut s, &[]);
    click(&mut s, 102.0, 207.0);
    click(&mut s, 300.0, 300.0);
    key(&mut s, ToolKey::Enter);
    let a = anchors(&s, sel(&s)[0]);
    assert!(near(a[0], 102.0, 207.0, 0.001), "{a:?}");
}

#[test]
fn pen_path_stays_in_its_document() {
    let mut s = session();
    let a = s.active_id().expect("A");
    tool(&mut s, ToolId::Pen);
    click(&mut s, 400.0, 100.0);
    click(&mut s, 500.0, 100.0);
    ex(&mut s, "file.new", json!({"width": 300, "height": 200}));
    let b = s.active_id().expect("B");
    assert_ne!(a, b);
    click(&mut s, 100.0, 100.0);
    click(&mut s, 200.0, 100.0);
    key(&mut s, ToolKey::Enter);
    let pts = anchors(&s, sel(&s)[0]);
    assert_eq!(pts.len(), 2, "kein Punkt aus Dokument A: {pts:?}");
    assert!(near(pts[0], 100.0, 100.0, 0.01));
    s.activate(a).expect("A");
    assert_eq!(top_elements(&s).len(), 1, "Zug in A beim Wechsel abgeschlossen");
}

/// Zickzack wie die Messung (7 Ecken, je Strecke 10 Zwischenpunkte).
fn zigzag() -> Vec<(f32, f32)> {
    let corners = [(100.0, 300.0), (150.0, 250.0), (200.0, 300.0), (250.0, 250.0), (300.0, 300.0), (350.0, 250.0), (400.0, 300.0)];
    let mut v = vec![corners[0]];
    for w in corners.windows(2) {
        for i in 1..=10 {
            let t = i as f32 / 10.0;
            v.push((w[0].0 + (w[1].0 - w[0].0) * t, w[0].1 + (w[1].1 - w[0].1) * t));
        }
    }
    v
}

#[test]
fn vector_path_keeps_corners_on_pixel_centers() {
    let mut s = session();
    tool(&mut s, ToolId::VectorPath);
    let z = zigzag();
    events(&mut s, &[ToolEvent::down(z[0].0, z[0].1)]);
    for p in &z[1..] {
        events(&mut s, &[ToolEvent::drag(p.0, p.1)]);
    }
    let l = z[z.len() - 1];
    events(&mut s, &[ToolEvent::up(l.0, l.1)]);
    let c = contours(&s, sel(&s)[0]);
    assert!(!c[0].closed);
    assert!(c[0].nodes.len() >= 7, "{}", c[0].nodes.len());
    for n in &c[0].nodes {
        assert!(n.pred == n.anchor && n.succ == n.anchor, "Eckknoten: {n:?}");
        assert!((n.anchor.x.fract() - 0.5).abs() < 1e-4 && (n.anchor.y.fract() - 0.5).abs() < 1e-4, "Pixelmitte: {n:?}");
    }
    assert_eq!(label(&s).as_deref(), Some("Pinsel"));
}

#[test]
fn redraw_path_replaces_only_the_section() {
    let mut s = session();
    let p = p1(&mut s);
    select(&mut s, &[p]);
    tool(&mut s, ToolId::RedrawPath);
    let mut pts = Vec::new();
    for i in 0..=20 {
        let t = i as f32 / 20.0;
        pts.push((290.0 + 40.0 * t, 200.0 + 60.0 * t));
    }
    for i in 1..=20 {
        let t = i as f32 / 20.0;
        pts.push((330.0 + 40.0 * t, 260.0 - 60.0 * t));
    }
    events(&mut s, &[ToolEvent::down(pts[0].0, pts[0].1)]);
    for q in &pts[1..] {
        events(&mut s, &[ToolEvent::drag(q.0, q.1)]);
    }
    events(&mut s, &[ToolEvent::up(370.0, 200.0)]);
    let a = anchors(&s, p);
    assert!(near(a[0], 200.0, 200.0, 0.01), "Anfang bleibt: {a:?}");
    assert!(near(*a.last().expect("Ende"), 420.0, 240.0, 0.01), "Ende bleibt: {a:?}");
    assert!(a.iter().any(|q| near(*q, 330.5, 260.5, 1.0)), "neuer Abschnitt: {a:?}");
    assert!(a.len() >= 5, "{a:?}");
    assert_eq!(label(&s).as_deref(), Some("Pfad nachzeichnen"));
}

// ------------------------------------------------------------------ 10 Formen

#[test]
fn shapes_get_stroke_and_names() {
    let mut s = session();
    for (t, name) in [(ToolId::Rectangle, None), (ToolId::Ellipse, Some("Ellipse")), (ToolId::Polygon, Some("Polygon"))] {
        tool(&mut s, t);
        drag(&mut s, (100.0, 100.0), (250.0, 200.0));
        let e = el(&s, sel(&s)[0]);
        assert_eq!(e.common.name.as_deref(), name, "{t:?}");
        let ElementKind::Path(p) = &e.kind else { panic!("Pfad") };
        assert!(p.attrs.brush.as_ref().is_some_and(|b| b.diameter == 1), "{t:?}: Strich 1 px");
        assert_eq!(p.attrs.stroke_color, fp_color::Argb(0xffc0_4040));
        assert!(p.attrs.fill.is_some(), "{t:?}: Füllung");
    }
}

#[test]
fn polygon_corner_follows_mouse_exactly() {
    let e = expected();
    let mut s = session();
    s.tools.options.polygon_sides = 6;
    tool(&mut s, ToolId::Polygon);
    drag(&mut s, (200.0, 150.0), (260.0, 150.0));
    drag(&mut s, (400.0, 150.0), (400.0, 210.0));
    drag(&mut s, (600.0, 150.0), (650.0, 200.0));
    let a = anchors(&s, sel(&s)[0]);
    let w = &e["unit10"]["r10_poly6_diag"];
    assert!(near(a[0], w[0].as_f64().unwrap_or(0.0) as f32, w[1].as_f64().unwrap_or(0.0) as f32, 0.01), "{a:?}");
}

#[test]
fn rectangle_up_held_raises_roundness() {
    let e = expected();
    let mut s = session();
    tool(&mut s, ToolId::Rectangle);
    events(&mut s, &[ToolEvent::down(400.0, 100.0), ToolEvent::drag(500.0, 170.0), ToolEvent::drag(600.0, 250.0)]);
    // gehalten: Wiederholung je Bild (≈ 30/s) über 1,5 s
    for k in 0..=45u64 {
        let mut ev = ToolEvent::key(ToolKey::Up);
        ev.time_ms = 1000 + k * 33;
        events(&mut s, &[ev]);
    }
    // ← → ohne Wirkung, aber verbraucht (kein Verschieben über das Menükürzel)
    let r = s.tool_event(&ToolEvent::key(ToolKey::Left)).expect("links");
    assert!(r.commands.is_empty() && !r.overlay.is_empty());
    events(&mut s, &[ToolEvent::up(600.0, 250.0)]);
    let ElementKind::Path(p) = &el(&s, sel(&s)[0]).kind else { panic!("Pfad") };
    let round = p.primitive.as_ref().map(|r| r.roundness).unwrap_or(0.0);
    let want = e["unit10"]["r05_rect_uphold_round"].as_f64().unwrap_or(0.0) as f32;
    assert!((round - want).abs() <= 0.06, "Rundung {round} statt {want}");
    assert!(rect_near(bounds(&s, sel(&s)[0]), Rect::new(400.0, 100.0, 600.0, 250.0), 0.01));
    // kurze Tipper: fast keine Rundung
    tool(&mut s, ToolId::Rectangle);
    events(&mut s, &[ToolEvent::down(100.0, 100.0), ToolEvent::drag(200.0, 150.0), ToolEvent::drag(250.0, 200.0)]);
    for k in 0..3u64 {
        let mut ev = ToolEvent::key(ToolKey::Up);
        ev.time_ms = 5000 + k * 600;
        events(&mut s, &[ev]);
    }
    events(&mut s, &[ToolEvent::up(250.0, 200.0)]);
    let ElementKind::Path(p) = &el(&s, sel(&s)[0]).kind else { panic!("Pfad") };
    assert!(p.primitive.as_ref().map(|r| r.roundness).unwrap_or(1.0) <= 0.02);
}

#[test]
fn rectangle_live_rect_while_dragging() {
    let mut s = session();
    tool(&mut s, ToolId::Rectangle);
    events(&mut s, &[ToolEvent::down(100.0, 100.0), ToolEvent::drag(150.0, 130.0), ToolEvent::drag(175.0, 150.0)]);
    assert_eq!(s.tool_live_rect(), Some(Rect::new(100.0, 100.0, 175.0, 150.0)));
    events(&mut s, &[ToolEvent::up(175.0, 150.0)]);
    assert_eq!(s.tool_live_rect(), None);
}

#[test]
fn rect_primitive_corner_pointer_and_subselect() {
    let e = expected();
    let mut s = session();
    let r = rect(&mut s, 100.0, 100.0, 300.0, 250.0);
    select(&mut s, &[r]);
    tool(&mut s, ToolId::Pointer);
    drag(&mut s, (100.0, 100.0), (130.0, 100.0));
    assert!(rect_near(bounds(&s, r), exp_rect(&e["unit10"]["r06_corner_drag_ptr"]), 0.01), "{:?}", bounds(&s, r));
    assert_eq!(label(&s).as_deref(), Some("Rechteck modifizieren"));
    // Teilauswahl: ganzes Rechteck verschieben, Grundform bleibt
    let mut s = session();
    let r = rect(&mut s, 100.0, 100.0, 300.0, 250.0);
    select(&mut s, &[r]);
    tool(&mut s, ToolId::Subselection);
    drag(&mut s, (100.0, 100.0), (80.0, 100.0));
    assert!(rect_near(bounds(&s, r), Rect::new(80.0, 100.0, 280.0, 250.0), 0.01), "{:?}", bounds(&s, r));
    assert!(matches!(&el(&s, r).kind, ElementKind::Path(p) if p.primitive.is_some()), "bleibt Rechteck");
    assert_eq!(label(&s).as_deref(), Some("Verschieben"));
}
