//! FIX-F: bekannte Befunde nach CT-2/PAR-X (Bericht `docs\berichte\fixf.md`).

use crate::common;
use common::*;
use fp_engine::Session;
use serde_json::json;

fn undo_count(s: &Session) -> usize {
    s.active().expect("Dokument").history.undo_count()
}

/// PAR-X-INSP-2: `pi.fill` mit derselben Art legt nur beim ersten Mal einen Verlaufsschritt an;
/// eine echte Änderung (andere Art, andere Farbe) danach wieder je einen.
#[test]
fn repeated_fill_same_kind_adds_no_step() {
    let mut s = doc_session(300.0, 200.0);
    let r = rect(&mut s, 10.0, 10.0, 110.0, 90.0);
    select(&mut s, &[r]);
    for cat in ["gradient", "solid", "pattern", "none"] {
        let n0 = undo_count(&s);
        ex(&mut s, "pi.fill", json!({ "category": cat }));
        let n1 = undo_count(&s);
        assert!(n1 <= n0 + 1, "{cat}: höchstens ein Schritt");
        let doc = s.active().unwrap().doc.clone();
        for _ in 0..3 {
            ex(&mut s, "pi.fill", json!({ "category": cat }));
        }
        assert_eq!(undo_count(&s), n1, "{cat}: Wiederholung ohne Schritt");
        assert!(std::sync::Arc::ptr_eq(&doc, &s.active().unwrap().doc), "{cat}: Dokument unverändert");
    }
    // echte Änderung zählt weiter
    ex(&mut s, "pi.fill", json!({ "category": "solid", "color": "#123456" }));
    let n = undo_count(&s);
    ex(&mut s, "pi.fill", json!({ "color": "#654321" }));
    assert_eq!(undo_count(&s), n + 1, "andere Farbe: ein Schritt");
    ex(&mut s, "pi.fill", json!({ "color": "#654321" }));
    assert_eq!(undo_count(&s), n + 1, "gleiche Farbe: kein Schritt");
    ex(&mut s, "pi.fill", json!({ "category": "gradient", "shape": 3 }));
    ex(&mut s, "pi.fill", json!({ "category": "gradient", "shape": 3 }));
    assert_eq!(undo_count(&s), n + 2, "Radial zweimal: ein Schritt");
}
