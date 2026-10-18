//! Q-55 (re 46 §4.2): a single text-tool click inside an existing text block (anywhere in its
//! box, also between glyphs) edits that block with the caret at a character boundary and no
//! selection; typing inserts there. A click outside every text block starts a new block.

use crate::common::{bounds, doc_session, ex, events, select, top_elements};
use fp_engine::tools::{ToolId, ToolInput};
use fp_engine::{Session, ToolEvent, ToolKey};
use fp_model::{ElementId, ElementKind};
use serde_json::json;

fn texts(s: &Session) -> Vec<String> {
    top_elements(s)
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Text(t) => Some(t.plain_text()),
            _ => None,
        })
        .collect()
}

fn send(s: &mut Session, i: ToolInput) {
    s.tool_event(&ToolEvent {
        input: i,
        mods: Default::default(),
        pressure: 1.0,
        time_ms: 0,
    })
    .expect("event");
}

fn setup() -> (Session, ElementId) {
    let mut s = doc_session(400.0, 300.0);
    let v = ex(
        &mut s,
        "draw.text",
        json!({"text": "Abc 123", "anchor": [50, 80], "size": 24, "color": "#000000"}),
    );
    let id = ElementId(v["id"].as_u64().unwrap_or(0));
    select(&mut s, &[id]);
    s.set_tool(ToolId::Text).expect("text tool");
    (s, id)
}

#[test]
fn click_between_glyphs_edits_existing_text_without_selection() {
    let (mut s, _) = setup();
    let id = top_elements(&s)[0].id;
    let b = bounds(&s, id);
    let c = ((b.left + b.right) / 2.0, (b.top + b.bottom) / 2.0);
    events(&mut s, &[ToolEvent::down(c.0, c.1), ToolEvent::up(c.0, c.1)]);
    assert!(s.tool_is_editing_text(), "editing");
    // the UI reports the hit character boundary; here: between "c" and " "
    send(&mut s, ToolInput::TextCaret { index: 3, extend: false });
    send(&mut s, ToolInput::Text("V".into()));
    send(&mut s, ToolInput::Text("r".into()));
    s.tool_event(&ToolEvent::key(ToolKey::Escape)).expect("Esc");
    assert_eq!(texts(&s), vec!["AbcVr 123".to_string()]);
}

#[test]
fn click_without_caret_report_inserts_not_replaces() {
    let (mut s, id) = setup();
    let b = bounds(&s, id);
    let p = (b.left + 2.0, (b.top + b.bottom) / 2.0);
    events(&mut s, &[ToolEvent::down(p.0, p.1), ToolEvent::up(p.0, p.1)]);
    send(&mut s, ToolInput::Text("V".into()));
    s.tool_event(&ToolEvent::key(ToolKey::Escape)).expect("Esc");
    let t = texts(&s);
    assert_eq!(t.len(), 1, "no new block: {t:?}");
    assert!(t[0].contains("Abc 123") && t[0].len() == 8, "inserted, nothing replaced: {t:?}");
}

#[test]
fn click_outside_text_starts_new_block() {
    let (mut s, _) = setup();
    events(&mut s, &[ToolEvent::down(300.0, 250.0), ToolEvent::up(300.0, 250.0)]);
    send(&mut s, ToolInput::Text("Vr".into()));
    s.tool_event(&ToolEvent::key(ToolKey::Escape)).expect("Esc");
    let mut t = texts(&s);
    t.sort();
    assert_eq!(t, vec!["Abc 123".to_string(), "Vr".to_string()]);
}
