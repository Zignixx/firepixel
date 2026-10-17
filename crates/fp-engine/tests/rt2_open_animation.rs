//! RT-2 (re\27 S4, BA-025): „Als Animation öffnen“ – Leinwand = Vereinigung aller Dateileinwände
//! (oben links bündig), jede Datei liefert alle ihre Status, Reihenfolge = Dateiliste; jeder
//! Status wird ein flaches Bild an 0,0 (re 29b §5, T-4).

use crate::common;
use common::*;
use fp_engine::Session;
use serde_json::json;
use std::path::PathBuf;

fn dir() -> PathBuf {
    let d = std::env::temp_dir().join(format!("fp-rt2-anim-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&d);
    d
}

/// Speichert ein Dokument `w` × `h` mit `states` Status; Status k enthält k + 1 Rechtecke.
fn save(name: &str, w: f32, h: f32, states: usize) -> PathBuf {
    let mut s = doc_session(w, h);
    if states > 1 {
        ex(&mut s, "panel.states.add", json!({"count": states - 1, "where": "end"}));
    }
    for k in 0..states {
        ex(&mut s, "state.setCurrent", json!({"index": k}));
        for i in 0..=k {
            rect(&mut s, 5.0 * i as f32, 5.0, 5.0 * i as f32 + 4.0, 9.0);
        }
    }
    let p = dir().join(name);
    ex(&mut s, "file.saveAs", json!({"path": p.to_string_lossy()}));
    p
}

/// Elemente je Status der ersten Seite (alle eigenen Ebenen).
fn per_state(s: &Session) -> Vec<usize> {
    let d = s.active().expect("Dokument");
    (0..d.frame_count()).map(|f| fp_engine::docops::own_layers(&d.doc, d.page).iter().map(|(_, l)| l.cels.get(f).map(|c| c.elements.len()).unwrap_or(0)).sum()).collect()
}

fn canvas(s: &Session) -> (f32, f32) {
    let d = s.active().expect("Dokument");
    let p = &d.doc.pages[d.page];
    (p.canvas.width, p.canvas.height)
}

#[test]
fn canvas_is_union_and_all_states_are_appended() {
    let a = save("rt2_a.fw.png", 200.0, 100.0, 2);
    let b = save("rt2_b.fw.png", 300.0, 50.0, 2);
    let mut s = doc_session(10.0, 10.0);
    ex(&mut s, "file.open", json!({"path": a.to_string_lossy(), "frames": [b.to_string_lossy()]}));
    assert_eq!(canvas(&s), (300.0, 100.0), "Vereinigung: größte Breite und Höhe");
    // a: Status 1/2, dann b: Status 1/2, je ein flaches Bild
    assert_eq!(per_state(&s), vec![1, 1, 1, 1]);
    // Bild oben links bündig an 0,0
    let d = s.active().expect("Dokument");
    let first = fp_engine::docops::own_layers(&d.doc, d.page).iter().find_map(|(_, l)| l.cels.first().and_then(|c| c.elements.first().cloned())).expect("Rechteck");
    let b = fp_engine::docops::element_bounds(&d.doc, &first).expect("Grenzen");
    assert_eq!((b.left.round(), b.top.round()), (0.0, 0.0));
}

#[test]
fn smaller_follow_up_file_keeps_first_canvas() {
    let a = save("rt2_c.fw.png", 200.0, 100.0, 1);
    let b = save("rt2_d.fw.png", 50.0, 40.0, 1);
    let mut s = doc_session(10.0, 10.0);
    ex(&mut s, "file.open", json!({"path": a.to_string_lossy(), "frames": [b.to_string_lossy()]}));
    assert_eq!(canvas(&s), (200.0, 100.0));
    assert_eq!(per_state(&s), vec![1, 1]);
}
