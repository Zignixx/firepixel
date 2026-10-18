//! TW-135: a pointer double-click on a text switches to the text tool and opens the editor in
//! the same event, with the caret at the character boundary under the click point (boundary =
//! glyph middle) and no word selected; a character typed right afterwards is inserted there.

use crate::common::{doc_session, events, ex, top_elements};
use fp_engine::tools::{ToolId, ToolInput};
use fp_engine::{Overlay, Session, ToolEvent, ToolKey};
use fp_model::{ElementId, ElementKind};
use serde_json::json;

/// "Zeile eins" / "Zeile zwei" at 14 px; returns the session, the text id and a click point just
/// behind the middle of the "l" of the first line (expected caret index 4, "Zeil|e").
fn setup() -> (Session, ElementId, (f32, f32)) {
    let mut s = doc_session(400.0, 300.0);
    let v = ex(
        &mut s,
        "draw.text",
        json!({"text": "Zeile eins\rZeile zwei", "anchor": [50, 160], "size": 14, "color": "#000000"}),
    );
    let id = ElementId(v["id"].as_u64().unwrap_or(0));
    ex(&mut s, "select.none", json!({}));
    let d = s.active().expect("doc");
    let e = top_elements(&s)
        .into_iter()
        .find(|e| e.id == id)
        .expect("text");
    let ElementKind::Text(t) = &e.kind else {
        panic!("not a text")
    };
    let block = fp_render::convert::text_block(&d.doc, t);
    let lay = fp_text::layout(
        &block,
        &fp_text::FontDb::shared(),
        &fp_text::LayoutOptions::default(),
    );
    let x = |i: usize| lay.caret(i).expect("caret").top[0];
    let c = lay.caret(0).expect("caret");
    let y = ((c.top[1] + c.bottom[1]) / 2.0) as f32;
    // between the middle of "l" (boundaries 3/4) and boundary 4
    let mid_l = (x(3) + x(4)) / 2.0;
    let px = mid_l + 0.3 * (x(4) - mid_l);
    (s, id, (px as f32, y))
}

fn double_click(s: &mut Session, (x, y): (f32, f32)) {
    events(
        s,
        &[
            ToolEvent::down(x, y),
            ToolEvent::up(x, y),
            ToolEvent::down(x, y),
            ToolEvent::up(x, y),
            ToolEvent::double(x, y),
        ],
    );
}

fn edit_view(s: &mut Session) -> Option<(usize, usize)> {
    s.tool_feedback().iter().find_map(|o| match o {
        Overlay::TextEdit(v) => Some((v.caret, v.sel_anchor)),
        _ => None,
    })
}

#[test]
fn pointer_double_click_puts_caret_at_click_point_without_selection() {
    let (mut s, _, p) = setup();
    s.set_tool(ToolId::Pointer).expect("pointer");
    double_click(&mut s, p);
    assert_eq!(s.tools.active, ToolId::Text);
    assert!(s.tool_is_editing_text(), "editor open in the same event");
    assert_eq!(edit_view(&mut s), Some((4, 4)), "caret at index 4, no word selected");
}

#[test]
fn character_typed_right_after_double_click_is_inserted_at_click_point() {
    let (mut s, id, p) = setup();
    s.set_tool(ToolId::Pointer).expect("pointer");
    double_click(&mut s, p);
    s.tool_event(&ToolEvent {
        input: ToolInput::Text("Q".into()),
        mods: Default::default(),
        pressure: 1.0,
        time_ms: 0,
    })
    .expect("text");
    s.tool_event(&ToolEvent::key(ToolKey::Escape)).expect("Esc");
    let text = top_elements(&s).into_iter().find_map(|e| match &e.kind {
        ElementKind::Text(t) if e.id == id => Some(t.plain_text()),
        _ => None,
    });
    assert_eq!(text.as_deref(), Some("ZeilQe eins\rZeile zwei"));
}
