//! R-32 (run 37 `inspector-text/25`): a fixed-width text that becomes point text
//! (`text.setProperties {frameType: "auto"}`, script `autoExpand = true`) keeps its frame top; the
//! anchor becomes the first baseline with `RIAf = BOAf = 0` (re 09 section 8a file data:
//! Arial 30 pt frame at 50 -> `TOPf` 50, `TOAf` 76).

use crate::common::{doc_session, el, ex, select};
use fp_model::{ElementId, ElementKind};
use serde_json::json;

fn geometry(s: &fp_engine::Session, id: ElementId) -> ([f32; 4], [f32; 4], Option<i32>) {
    let e = el(s, id);
    let ElementKind::Text(t) = &e.kind else {
        panic!("no text");
    };
    let a = t.anchor_rect.unwrap_or(t.bounds);
    let b = t.bounds;
    (
        [a.left, a.top, a.right, a.bottom],
        [b.left, b.top, b.right, b.bottom],
        t.frame_type,
    )
}

#[test]
fn fixed_to_auto_keeps_the_frame_top() {
    if fp_text::FontDb::shared().find_ps("ArialMT").is_none() {
        eprintln!("skipped: ArialMT not installed");
        return;
    }
    let mut s = doc_session(800.0, 600.0);
    let v = ex(
        &mut s,
        "draw.text",
        json!({"text": "Hamburg", "rect": [20, 50, 720, 150], "font": "ArialMT", "size": 30,
               "color": "#000000", "initFromPrefs": false}),
    );
    let id = ElementId(v["id"].as_u64().unwrap_or(0));
    select(&mut s, &[id]);
    let (_, before, ft) = geometry(&s, id);
    assert_eq!((ft, before[1]), (Some(1), 50.0), "fixed frame at 50");
    ex(&mut s, "text.setProperties", json!({"frameType": "auto"}));
    let (anchor, frame, ft) = geometry(&s, id);
    assert_eq!(ft, Some(0));
    assert_eq!(anchor, [20.0, 76.0, 0.0, 0.0], "anchor = first baseline");
    assert_eq!((frame[1], frame[3]), (50.0, 86.0), "frame top kept");
    // a point text stays where it is when set to auto again
    ex(&mut s, "text.setProperties", json!({"frameType": "auto"}));
    assert_eq!(geometry(&s, id).0, anchor);
}

/// TW-214 (re 63 section 2.3): the left edge stays too. A point frame starts at
/// floor(min(anchor x, ink left)); Arial "A" bears left, so the anchor moves 1 px right and the
/// frame keeps its left edge (ink 100 -> 101 in the reference). Myriad Pro and Times do not move.
#[test]
fn fixed_to_auto_keeps_the_left_edge() {
    for (font, text, anchor_x) in [
        ("ArialMT", "Ag", 101.0),
        ("ArialMT", "Zeile", 100.0),
        ("TimesNewRomanPSMT", "Adg", 100.0),
    ] {
        if fp_text::FontDb::shared().find_ps(font).is_none() {
            eprintln!("skipped: {font} not installed");
            continue;
        }
        let mut s = doc_session(600.0, 400.0);
        let v = ex(
            &mut s,
            "draw.text",
            json!({"text": text, "rect": [100, 100, 300, 200], "font": font, "size": 12,
                   "color": "#000000", "initFromPrefs": false}),
        );
        let id = ElementId(v["id"].as_u64().unwrap_or(0));
        select(&mut s, &[id]);
        ex(&mut s, "text.setProperties", json!({"frameType": "auto"}));
        let (anchor, frame, _) = geometry(&s, id);
        assert_eq!(anchor[0], anchor_x, "{font} {text} anchor x");
        assert_eq!(
            (frame[0], frame[1]),
            (100.0, 100.0),
            "{font} {text} frame left/top"
        );
    }
}
