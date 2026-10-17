//! Vermessung neuer Punkttexte (Befund I18N-QA „B 139 / Y 309 statt B 202 / Y 306“).
//!
//! Ursache der Abweichung (UE-1, 12.10.): keine Übersetzung und keine geänderte Schriftauflösung
//! durch fp-i18n, sondern zwei gewollte Paritätsänderungen von LW:
//! - `ac05b70`: Vorgabeschrift des Textwerkzeugs wie im Original „Myriad Pro“ (wenn installiert,
//!   sonst `Text_Font`/Arial), vorher Arial;
//! - `19536aa`: `TextElem::bounds` neuer Texte = Rahmen des echten Satzes (fp-text, gegen die
//!   gespeicherten Rahmen `LFTf…BOTf` des Korpus geprüft), vorher nur die Schätzung aus
//!   `Element::new_text` (Breite = 0,6 × Größe × Zeichen = 202, oben = Grundlinie − Größe = 306).
//!
//! Die alten Maße 202/306 waren also die Schätzung; die neuen sind die Maße des Satzes. Diese Tests
//! sichern die Satzmaße und dass die Sprache der Oberfläche sie nicht beeinflusst.

use crate::common;
use common::*;
use fp_engine::Session;
use fp_geom::Rect;
use fp_model::ElementId;
use serde_json::json;

fn draw(s: &mut Session, extra: serde_json::Value) -> Rect {
    let mut p = json!({"text": "Firepixel Test", "anchor": [40, 330], "size": 24});
    if let (Some(o), Some(e)) = (p.as_object_mut(), extra.as_object()) {
        o.extend(e.clone());
    }
    let v = ex(s, "draw.text", p);
    bounds(s, ElementId(v["id"].as_u64().expect("id")))
}

fn installed(family: &str) -> bool {
    fp_text::FontDb::shared().find_family(family, false, false).is_some()
}

/// Satzmaße für Arial 24 px (so im Testdokument von I18N-QA: B 139 / Y 309) und für die
/// Vorgabeschrift Myriad Pro (B 130 / Y 309); nicht mehr die Schätzung 202 × 28,8 ab 306.
#[test]
fn point_text_bounds_come_from_layout_not_estimate() {
    let mut s = doc_session(500.0, 500.0);
    // Maße mit Autom. Unterschneidung; neue Texte haben sie wie im Original nicht (Text_AutoKern, EB-1)
    if installed("Arial") {
        assert_eq!(draw(&mut s, json!({"font": "Arial", "autoKern": true})), Rect::new(40.0, 309.0, 179.0, 338.0));
        assert_eq!(draw(&mut s, json!({"font": "Arial"})), Rect::new(40.0, 309.0, 183.0, 338.0), "ohne Unterschneidung breiter");
    }
    if installed("Myriad Pro") {
        assert_eq!(draw(&mut s, json!({"font": "Myriad Pro", "autoKern": true})), Rect::new(40.0, 309.0, 170.0, 336.0));
    }
    let b = draw(&mut s, json!({}));
    assert!(b.width() < 180.0 && b.top > 306.0, "Schätzung statt Satz: {b:?}");
}

/// Die Sprache der Oberfläche ändert die Vermessung nicht (Deutsch, Englisch, Testsprache).
#[test]
fn text_bounds_independent_of_ui_language() {
    let mut s = doc_session(500.0, 500.0);
    let de = fp_i18n::with_language("de", || draw(&mut s, json!({"font": "Arial"})));
    let en = fp_i18n::with_language("en", || draw(&mut s, json!({"font": "Arial"})));
    let qps = fp_i18n::with_language("qps", || draw(&mut s, json!({"font": "Arial"})));
    assert_eq!(de, en);
    assert_eq!(de, qps);
}
