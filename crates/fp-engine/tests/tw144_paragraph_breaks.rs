//! TW-144 (re 50 section 3.4): text from the editor (typed or pasted), `draw.text` and
//! `text.setText` (script, MCP, CLI) stores paragraph ends as `\r`: CRLF and LF become CR.

use crate::common::{doc_session, el, ex, select, top_elements};
use fp_engine::tools::{ToolId, ToolInput};
use fp_engine::{Session, ToolEvent, ToolKey};
use fp_model::{ElementId, ElementKind};
use serde_json::json;

fn text_of(s: &Session, id: ElementId) -> String {
    match &el(s, id).kind {
        ElementKind::Text(t) => t.plain_text(),
        _ => panic!("no text"),
    }
}

#[test]
fn draw_text_and_set_text_convert_crlf_and_lf() {
    let mut s = doc_session(300.0, 200.0);
    let v = ex(
        &mut s,
        "draw.text",
        json!({"text": "a\r\nb\nc\rd", "anchor": [10, 50]}),
    );
    let id = ElementId(v["id"].as_u64().unwrap_or(0));
    assert_eq!(text_of(&s, id), "a\rb\rc\rd");
    select(&mut s, &[id]);
    ex(&mut s, "text.setText", json!({"text": "x\ny\r\nz"}));
    assert_eq!(text_of(&s, id), "x\ry\rz");
    // runs are normalized like the text
    ex(
        &mut s,
        "text.setText",
        json!({"text": "p\nq", "runs": [{"attrs": {}, "text": "p\n"}, {"attrs": {"bold": true}, "text": "q"}]}),
    );
    assert_eq!(text_of(&s, id), "p\rq");
}

#[test]
fn pasted_text_in_the_editor_converts_crlf_and_lf() {
    let mut s = doc_session(300.0, 200.0);
    s.set_tool(ToolId::Text).expect("text tool");
    s.tool_event(&ToolEvent::down(40.0, 60.0)).expect("down");
    s.tool_event(&ToolEvent::up(40.0, 60.0)).expect("up");
    s.tool_event(&ToolEvent {
        input: ToolInput::Text("eins\r\nzwei\ndrei".into()),
        mods: Default::default(),
        pressure: 1.0,
        time_ms: 0,
    })
    .expect("paste");
    s.tool_event(&ToolEvent::key(ToolKey::Escape)).expect("Esc");
    let t = top_elements(&s)
        .into_iter()
        .find_map(|e| match &e.kind {
            ElementKind::Text(t) => Some(t.plain_text()),
            _ => None,
        })
        .expect("text");
    assert_eq!(t, "eins\rzwei\rdrei", "no trailing CR, CR between paragraphs");
}
