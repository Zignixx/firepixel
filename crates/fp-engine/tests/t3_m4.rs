//! T-3: Soll-Regeln aus der Mess-Sitzung re 29 (M4).

use crate::common;
use common::*;
use fp_engine::tools::ToolId;
use fp_engine::{Session, ToolEvent, ToolKey};
use fp_model::{Contour, ElementId, ElementKind};
use serde_json::json;

fn tool(s: &mut Session, t: ToolId) {
    s.set_tool(t).unwrap_or_else(|e| panic!("{e}"));
}

fn click(s: &mut Session, x: f32, y: f32) {
    events(s, &[ToolEvent::down(x, y), ToolEvent::up(x, y)]);
}

fn key(s: &mut Session, k: ToolKey) {
    events(s, &[ToolEvent::key(k)]);
}

fn contours(s: &Session, id: ElementId) -> Vec<Contour> {
    match &el(s, id).kind {
        ElementKind::Path(p) => p.contours.clone(),
        _ => panic!("kein Pfad"),
    }
}

fn sel(s: &Session) -> Vec<ElementId> {
    s.active().map(|d| d.selection.elements.clone()).unwrap_or_default()
}

/// L31: Klick 320,140, Ziehen 400,140 → 440,180, Esc.
fn l31() -> (Session, ElementId) {
    let mut s = doc_session(600.0, 300.0);
    tool(&mut s, ToolId::Pen);
    click(&mut s, 320.0, 140.0);
    events(&mut s, &[ToolEvent::down(400.0, 140.0), ToolEvent::drag(420.0, 160.0), ToolEvent::drag(440.0, 180.0), ToolEvent::up(440.0, 180.0)]);
    key(&mut s, ToolKey::Escape);
    let id = sel(&s)[0];
    (s, id)
}

fn subselect(s: &mut Session, id: ElementId, node: usize) {
    tool(s, ToolId::Subselection);
    ex(s, "select.points", json!({"points": [{"element": id.0, "contour": 0, "node": node}]}));
    ex(s, "select.set", json!({"ids": [id.0], "mode": "replace"}));
    ex(s, "select.points", json!({"points": [{"element": id.0, "contour": 0, "node": node}]}));
}

#[test]
fn ba001_pen_continues_at_last_subselected_endpoint() {
    // kurz und p2: fortgesetzt [320/140, 400/140, 400/220]
    for p2 in [false, true] {
        let (mut s, id) = l31();
        if p2 {
            subselect(&mut s, id, 1);
        }
        tool(&mut s, ToolId::Pen);
        click(&mut s, 400.0, 140.0);
        click(&mut s, 400.0, 220.0);
        key(&mut s, ToolKey::Escape);
        assert_eq!(top_elements(&s).len(), 1, "p2={p2}");
        let c = contours(&s, id);
        assert_eq!(c[0].nodes.len(), 3);
        assert!(!c[0].closed);
    }
    // p1: Klick auf den anderen Endpunkt schließt, 400,220 ist ein neuer Pfad
    let (mut s, id) = l31();
    subselect(&mut s, id, 0);
    tool(&mut s, ToolId::Pen);
    click(&mut s, 400.0, 140.0);
    let c = contours(&s, id);
    assert!(c[0].closed, "Pfad 1 geschlossen");
    assert_eq!(c[0].nodes.len(), 2);
    click(&mut s, 400.0, 220.0);
    click(&mut s, 450.0, 220.0);
    key(&mut s, ToolKey::Escape);
    assert_eq!(top_elements(&s).len(), 2, "neuer Pfad");
    assert_eq!(contours(&s, id)[0].nodes.len(), 2);
}

#[test]
fn ba002_pen_points_never_snap_to_objects() {
    let mut s = doc_session(400.0, 300.0);
    rect(&mut s, 100.0, 100.0, 200.0, 160.0);
    ex(&mut s, "select.none", json!({}));
    tool(&mut s, ToolId::Pen);
    click(&mut s, 201.0, 200.0);
    events(&mut s, &[ToolEvent::down(152.0, 162.0), ToolEvent::up(152.0, 162.0), ToolEvent::double(152.0, 162.0)]);
    let id = sel(&s)[0];
    let c = contours(&s, id);
    let a: Vec<(f32, f32)> = c[0].nodes.iter().map(|n| (n.anchor.x, n.anchor.y)).collect();
    assert_eq!(a, vec![(201.0, 200.0), (152.0, 162.0)]);
}

#[test]
fn ba026_restore_needs_marquee_or_own_saved_delete_any_saved() {
    let mut s = doc_session(300.0, 200.0);
    let a = s.active_id().expect("A");
    let avail = |s: &Session, n: &str| s.is_available(n);
    ex(&mut s, "select.pixels", json!({"shape": "rect", "rect": [20, 20, 120, 100]}));
    ex(&mut s, "select.saveBitmapSelection", json!({"name": "m4sel"}));
    assert!(avail(&s, "select.restoreBitmapSelection") && avail(&s, "select.deleteBitmapSelection"), "A mit Rahmen");
    ex(&mut s, "select.none", json!({}));
    // TW-66 (F19): re 29 §10 maß hier „grau“, Vergleichslauf 4 und 7 (`dialogs-d31-d40/15`) öffnen
    // nach Speichern + Strg+D im selben Dokument aber den Dialog „Auswahl wiederherstellen“. Der
    // jüngere, direkte Beleg gilt: eigene gespeicherte Auswahl im aktiven Dokument genügt.
    assert!(avail(&s, "select.restoreBitmapSelection"), "A ohne Rahmen, eigene Auswahl: aktiv (TW-66)");
    assert!(avail(&s, "select.deleteBitmapSelection"));
    // C andere Größe, B gleiche Größe: ohne Rahmen nur „löschen“
    for (w, h) in [(500, 400), (300, 200)] {
        ex(&mut s, "file.new", json!({"width": w, "height": h, "background": "#ffffff"}));
        assert!(!avail(&s, "select.restoreBitmapSelection"), "{w}×{h}");
        assert!(avail(&s, "select.deleteBitmapSelection"), "{w}×{h}");
    }
    // B mit eigenem Rahmen: beide frei
    ex(&mut s, "select.pixels", json!({"shape": "rect", "rect": [150, 50, 250, 150]}));
    assert!(avail(&s, "select.restoreBitmapSelection") && avail(&s, "select.deleteBitmapSelection"));
    let _ = a;
}

#[test]
fn tw53_animate_selection_two_steps_named_like_original() {
    let mut s = doc_session(400.0, 300.0);
    let r = rect(&mut s, 50.0, 50.0, 150.0, 120.0);
    select(&mut s, &[r]);
    ex(&mut s, "modify.animation.animateSelection", json!({"frames": 5, "addStates": false}));
    let label = |s: &Session| s.active().and_then(|d| d.history.undo_label().map(str::to_string));
    assert_eq!(label(&s).as_deref(), Some("In Symbol konvertieren"));
    // OK auf „Automatisch neue Status hinzufügen?“ (Oberfläche: Schritt „add-states“)
    ex(&mut s, "modify.animation.settings", json!({"frames": 5, "step": "add-states"}));
    assert_eq!(label(&s).as_deref(), Some("Status hinzufügen"));
    assert_eq!(s.active().expect("d").frame_count(), 5);
    fp_i18n::set_language("en");
    let en = fp_i18n::t("msg-step-add-states");
    fp_i18n::set_language("de");
    assert_eq!(en, "Add States");
}

#[test]
fn ba028_export_format_choice_is_step_export_options() {
    let mut s = doc_session(400.0, 300.0);
    rect(&mut s, 50.0, 50.0, 150.0, 120.0);
    ex(&mut s, "select.none", json!({}));
    let before = s.active().expect("d").history.undo_count();
    ex(&mut s, "pi.export.settings", json!({"settings": serde_json::to_value(fp_model::ExportSettings::default()).expect("json"), "step": "export-options"}));
    let d = s.active().expect("d");
    assert_eq!(d.history.undo_count(), before + 1);
    assert_eq!(d.history.undo_label(), Some("Exportoptionen"));
}

#[test]
fn tw57_text_align_reduces_mixed_selection_to_text() {
    let mut s = doc_session(400.0, 300.0);
    let t = ElementId(ex(&mut s, "draw.text", json!({"text": "Abc", "anchor": [40, 60]}))["id"].as_u64().expect("id"));
    let r = rect(&mut s, 200.0, 100.0, 300.0, 200.0);
    select(&mut s, &[t, r]);
    ex(&mut s, "text.align.centeredHorizontally", json!({}));
    assert_eq!(sel(&s), vec![t]);
    select(&mut s, &[t]);
    ex(&mut s, "text.align.right", json!({}));
    assert_eq!(sel(&s), vec![t], "Text allein bleibt gewählt");
}

/// BA-001 Rest (re 33 §6): Stiftklick auf den Anfangspunkt des nach Esc noch gewählten offenen
/// Pfads schließt ihn (Knotenzahl bleibt 3), der nächste Klick beginnt einen neuen Pfad.
#[test]
fn ba001_pen_click_on_start_point_closes_selected_path() {
    let mut s = doc_session(600.0, 400.0);
    tool(&mut s, ToolId::Pen);
    click(&mut s, 100.0, 100.0);
    click(&mut s, 200.0, 100.0);
    click(&mut s, 200.0, 200.0);
    key(&mut s, ToolKey::Escape);
    let id = sel(&s)[0];
    assert!(!contours(&s, id)[0].closed);
    click(&mut s, 100.0, 100.0);
    // Escape did not end the drawing (re 55 §7.2): closing the path of this pen run redraws it as a
    // closed shape (a new element)
    let id = sel(&s)[0];
    let c = contours(&s, id);
    assert!(c[0].closed, "Pfad geschlossen");
    assert_eq!(c[0].nodes.len(), 3, "kein neuer Knoten");
    // Original: schon der Einzelklick 300,250 + Esc ergibt Pfad 2 mit 1 Knoten; Firepixel verwirft
    // Ein-Punkt-Pfade (offener Rest im Bericht s5), daher hier zwei Klicks
    click(&mut s, 300.0, 250.0);
    click(&mut s, 350.0, 250.0);
    key(&mut s, ToolKey::Escape);
    let tops = top_elements(&s);
    assert_eq!(tops.len(), 2, "neuer Pfad");
    let c = contours(&s, id);
    assert!(c[0].closed);
    assert_eq!(c[0].nodes.len(), 3, "Pfad 1 unverändert");
    let other = tops.iter().find(|e| e.id != id).expect("Pfad 2");
    let ElementKind::Path(p) = &other.kind else { panic!("kein Pfad") };
    assert_eq!(p.contours[0].nodes.len(), 2, "Pfad 2 beginnt bei 300/250");
    assert_eq!((p.contours[0].nodes[0].anchor.x, p.contours[0].nodes[0].anchor.y), (300.0, 250.0));
}

/// FP2-02 (re 37 §2): nach Teilauswahl eines Endpunkts und Wechsel zum Stift.
/// Punkt 1 (Anfang): Stiftklick auf denselben Punkt schließt den Pfad (3 Knoten), der Folgeklick
/// beginnt einen neuen Pfad. Punkt 3 (Ende): Klick lässt den Pfad offen, ohne neuen Knoten; der
/// Folgeklick hängt am Ende an (4 Knoten).
#[test]
fn ba001_pen_after_subselection_start_closes_end_continues() {
    let three = |s: &mut Session| -> ElementId {
        tool(s, ToolId::Pen);
        click(s, 100.0, 100.0);
        click(s, 200.0, 100.0);
        click(s, 200.0, 200.0);
        key(s, ToolKey::Escape);
        sel(s)[0]
    };
    let anchors = |s: &Session, id: ElementId| -> Vec<(f32, f32)> { contours(s, id)[0].nodes.iter().map(|n| (n.anchor.x, n.anchor.y)).collect() };
    // Punkt 1
    let mut s = doc_session(600.0, 400.0);
    let id = three(&mut s);
    subselect(&mut s, id, 0);
    tool(&mut s, ToolId::Pen);
    click(&mut s, 100.0, 100.0);
    let c = contours(&s, id);
    assert!(c[0].closed, "P1: geschlossen");
    assert_eq!(c[0].nodes.len(), 3, "P1: Knotenzahl bleibt");
    click(&mut s, 300.0, 250.0);
    click(&mut s, 350.0, 250.0);
    key(&mut s, ToolKey::Escape);
    let tops = top_elements(&s);
    assert_eq!(tops.len(), 2, "P1: Folgeklick = neuer Pfad");
    assert!(contours(&s, id)[0].closed);
    assert_eq!(contours(&s, id)[0].nodes.len(), 3);
    let other = tops.iter().find(|e| e.id != id).expect("Pfad 2");
    let ElementKind::Path(p) = &other.kind else { panic!("kein Pfad") };
    assert_eq!((p.contours[0].nodes[0].anchor.x, p.contours[0].nodes[0].anchor.y), (300.0, 250.0));
    // Punkt 3
    let mut s = doc_session(600.0, 400.0);
    let id = three(&mut s);
    subselect(&mut s, id, 2);
    tool(&mut s, ToolId::Pen);
    click(&mut s, 200.0, 200.0);
    let c = contours(&s, id);
    assert!(!c[0].closed, "P3: bleibt offen");
    assert_eq!(c[0].nodes.len(), 3, "P3: kein neuer Knoten");
    click(&mut s, 300.0, 250.0);
    key(&mut s, ToolKey::Escape);
    assert_eq!(top_elements(&s).len(), 1, "P3: derselbe Pfad");
    assert!(!contours(&s, id)[0].closed);
    assert_eq!(anchors(&s, id), vec![(100.0, 100.0), (200.0, 100.0), (200.0, 200.0), (300.0, 250.0)]);
}
