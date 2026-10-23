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
