//! F6 (Q-16, `docs\berichte\m1.md`): Text ▸ Von Pfad lösen wie im Original. Der Text bleibt ein
//! Rahmen mit fester Breite (Breite vor dem Anfügen), wird neu umbrochen und auf die Mitte der
//! geometrischen Pfadgrenzen zentriert; der Pfad kommt als oberstes Objekt der Ebene zurück.
//! Der Fall mit der Testdatei wird übersprungen, wenn sie fehlt (anderer Rechner).

use crate::common::*;
use fp_model::element::ElementKind;
use fp_model::ElementId;
use serde_json::json;

fn find_named(s: &fp_engine::Session, name: &str) -> ElementId {
    let d = s.active().expect("Dokument");
    let mut out = None;
    d.doc.walk_elements(|_, _, e| {
        if e.common.name.as_deref() == Some(name) && out.is_none() {
            out = Some(e.id);
        }
    });
    out.unwrap_or_else(|| panic!("{name} fehlt"))
}

fn layer_names(s: &fp_engine::Session) -> Vec<String> {
    let d = s.active().expect("Dokument");
    let l = &fp_engine::docops::own_layers(&d.doc, 0)[0].1;
    l.cels[0].elements.iter().map(|e| e.common.name.clone().unwrap_or_default()).collect()
}

fn text_of(s: &fp_engine::Session, id: ElementId) -> fp_model::TextElem {
    match &el(s, id).kind {
        ElementKind::Text(t) => (**t).clone(),
        _ => panic!("kein Text"),
    }
}

#[test]
fn detach_like_original_par_text() {
    let src = std::path::Path::new(r"C:\Users\Administrator\Desktop\REA\re\_testdateien\par\par_text.fw.png");
    if !src.is_file() {
        eprintln!("übersprungen: {src:?} fehlt");
        return;
    }
    let p = std::env::temp_dir().join(format!("fp-f6-{}-par_text.fw.png", std::process::id()));
    std::fs::copy(src, &p).expect("Kopie");
    let mut s = doc_session(10.0, 10.0);
    ex(&mut s, "file.open", json!({"path": p.to_string_lossy()}));
    let _ = std::fs::remove_file(&p);

    // TX3 (an Kreis 350,100–550,300) lösen: Original roh 402,183 · 96 × 35, zwei Zeilen, Ellipse oben
    let tx3 = find_named(&s, "TX3");
    select(&mut s, &[tx3]);
    ex(&mut s, "text.detachFromPath", json!({}));
    let t = text_of(&s, tx3);
    assert!(t.on_path.is_none());
    assert_eq!(t.frame_type, Some(1), "Rahmen mit fester Breite");
    let b = t.bounds;
    assert_eq!((b.left, b.top, b.right - b.left), (402.0, 183.0, 96.0), "Rahmen {b:?}");
    assert!((b.bottom - b.top - 35.0).abs() <= 1.0, "zwei Zeilen, Höhe ≈ 35: {b:?}");
    assert_eq!(layer_names(&s)[0], "Ellipse", "Pfad ganz oben: {:?}", layer_names(&s));
    let sel = s.active().map(|d| d.selection.elements.clone()).unwrap_or_default();
    assert_eq!(sel.len(), 2);
    assert!(sel.contains(&tx3));

    // TX2 (Rahmen 50,150 · 200 × 48) an P2 (Linie 50,400–400,450) anfügen und lösen: roh 125,401 · 200 × 48
    let (tx2, p2) = (find_named(&s, "TX2"), find_named(&s, "P2"));
    select(&mut s, &[tx2, p2]);
    ex(&mut s, "text.attachToPath", json!({}));
    ex(&mut s, "text.detachFromPath", json!({}));
    let b = text_of(&s, tx2).bounds;
    assert_eq!((b.left, b.right - b.left), (125.0, 200.0), "Rahmen {b:?}");
    assert!((b.top - 401.0).abs() <= 1.0 && (b.bottom - b.top - 48.0).abs() <= 1.0, "Rahmen {b:?}");
    assert_eq!(layer_names(&s)[0], "P2", "Pfad ganz oben: {:?}", layer_names(&s));
}

/// Nebenbefund: Text mit drei Absätzen an einer offenen Linie zeigt wie im Original nur den ersten
/// Absatz (Original TX2 an P2: Tinte 50,390–104,408 = „Zeile eins“).
#[test]
fn attach_shows_first_paragraph_only() {
    let src = std::path::Path::new(r"C:\Users\Administrator\Desktop\REA\re\_testdateien\par\par_text.fw.png");
    if !src.is_file() {
        eprintln!("übersprungen: {src:?} fehlt");
        return;
    }
    let p = std::env::temp_dir().join(format!("fp-f6b-{}-par_text.fw.png", std::process::id()));
    std::fs::copy(src, &p).expect("Kopie");
    let mut s = doc_session(10.0, 10.0);
    ex(&mut s, "file.open", json!({"path": p.to_string_lossy()}));
    let _ = std::fs::remove_file(&p);
    let (tx2, p2) = (find_named(&s, "TX2"), find_named(&s, "P2"));
    select(&mut s, &[tx2, p2]);
    ex(&mut s, "text.attachToPath", json!({}));
    let b = text_of(&s, tx2).bounds;
    assert!((b.left - 50.0).abs() <= 1.0 && (b.right - 104.0).abs() <= 2.0, "nur „Zeile eins“ sichtbar: {b:?}");
}

#[test]
fn detach_keeps_fixed_width_and_centers_on_path() {
    let mut s = doc_session(600.0, 400.0);
    let r = rect(&mut s, 10.0, 10.0, 20.0, 20.0);
    ex(&mut s, "draw.text", json!({"text": "Text am Pfad entlang", "rect": [50, 50, 146, 100], "size": 16}));
    let t = s.active().and_then(|d| d.selection.elements.first().copied()).expect("Text");
    let p = ellipse(&mut s, 350.0, 100.0, 550.0, 300.0);
    select(&mut s, &[t, p]);
    ex(&mut s, "text.attachToPath", json!({}));
    ex(&mut s, "text.detachFromPath", json!({}));
    let tx = text_of(&s, t);
    assert_eq!(tx.frame_type, Some(1));
    let b = tx.bounds;
    assert_eq!(b.right - b.left, 96.0, "Breite vor dem Anfügen: {b:?}");
    assert!(b.bottom - b.top > 25.0, "neu umbrochen (mehrzeilig): {b:?}");
    assert_eq!(b.left, 402.0);
    assert_eq!(b.top, (200.0 - (b.bottom - b.top) * 0.5 + 0.5).floor());
    // Pfad oberstes Objekt, Rechteck darunter
    let d = s.active().expect("Dokument");
    let l = &fp_engine::docops::own_layers(&d.doc, 0)[0].1;
    let ids: Vec<_> = l.cels[0].elements.iter().map(|e| e.id).collect();
    assert!(matches!(l.cels[0].elements[0].kind, ElementKind::Path(_)));
    assert_eq!(ids.last(), Some(&r));
    // ein Verlaufsschritt
    ex(&mut s, "edit.undo", json!({}));
    assert!(text_of(&s, t).on_path.is_some());
}
