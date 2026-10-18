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

// ------------------------------------------------------------------ TW-130 (re 47 §2)

fn type_and_close(s: &mut Session, t: &str) {
    send(s, ToolInput::Text(t.into()));
    s.tool_event(&ToolEvent::key(ToolKey::Escape)).expect("Esc");
}

/// The engine itself puts the caret at the boundary under the click point (no UI caret report):
/// runner step xs-shortcuts/15, click at x + 12 into "Abc 123" at 24 pt, "Vr" ⇒ "AVrbc 123".
#[test]
fn click_places_caret_at_click_point() {
    let (mut s, id) = setup();
    let b = bounds(&s, id);
    let p = (b.left + 12.0, (b.top + b.bottom) / 2.0);
    events(&mut s, &[ToolEvent::down(p.0, p.1), ToolEvent::up(p.0, p.1)]);
    assert!(s.tool_is_editing_text(), "editing");
    type_and_close(&mut s, "Vr");
    assert_eq!(texts(&s), vec!["AVrbc 123".to_string()]);
}

#[test]
fn click_at_left_and_right_edge_puts_caret_at_start_and_end() {
    let (mut s, id) = setup();
    let b = bounds(&s, id);
    let y = (b.top + b.bottom) / 2.0;
    events(&mut s, &[ToolEvent::down(b.left + 1.0, y), ToolEvent::up(b.left + 1.0, y)]);
    type_and_close(&mut s, "Vr");
    assert_eq!(texts(&s), vec!["VrAbc 123".to_string()]);

    s.set_tool(ToolId::Text).expect("text tool");
    let b = bounds(&s, id);
    events(&mut s, &[ToolEvent::down(b.right - 1.0, y), ToolEvent::up(b.right - 1.0, y)]);
    type_and_close(&mut s, "X");
    assert_eq!(texts(&s), vec!["VrAbc 123X".to_string()]);
}

/// Selection and modifiers do not matter: an unselected text is hit the same way, also with Shift.
#[test]
fn unselected_text_with_modifier_is_edited_too() {
    let (mut s, id) = setup();
    select(&mut s, &[]);
    s.set_tool(ToolId::Text).expect("text tool");
    let b = bounds(&s, id);
    let p = (b.left + 12.0, (b.top + b.bottom) / 2.0);
    let mods = fp_engine::tools::Modifiers {
        shift: true,
        ..Default::default()
    };
    for kind in [
        fp_engine::tools::PointerKind::Down,
        fp_engine::tools::PointerKind::Up,
    ] {
        s.tool_event(&ToolEvent {
            input: ToolInput::Pointer {
                kind,
                pos: fp_geom::Point::new(p.0, p.1),
            },
            mods,
            pressure: 1.0,
            time_ms: 0,
        })
        .expect("event");
    }
    assert!(s.tool_is_editing_text(), "editing");
    type_and_close(&mut s, "Vr");
    assert_eq!(texts(&s), vec!["AVrbc 123".to_string()]);
}

/// A drag below 20 px is a click (auto width); from 20 px on the block gets a fixed-width frame.
#[test]
fn short_drag_is_a_click_long_drag_a_frame() {
    let (mut s, _) = setup();
    events(&mut s, &[ToolEvent::down(200.0, 200.0), ToolEvent::up(212.0, 205.0)]);
    type_and_close(&mut s, "WWWWWW");
    let short = top_elements(&s)
        .iter()
        .find(|e| matches!(&e.kind, ElementKind::Text(t) if t.plain_text() == "WWWWWW"))
        .map(|e| e.id)
        .expect("short drag text");
    let b = bounds(&s, short);
    assert!(b.width() > 40.0, "auto width, one line: {b:?}");
    assert!(b.height() < 40.0, "one line: {b:?}");

    s.set_tool(ToolId::Text).expect("text tool");
    events(&mut s, &[ToolEvent::down(100.0, 230.0), ToolEvent::up(180.0, 235.0)]);
    type_and_close(&mut s, "a");
    let long = top_elements(&s)
        .iter()
        .find(|e| matches!(&e.kind, ElementKind::Text(t) if t.plain_text() == "a"))
        .map(|e| e.id)
        .expect("frame text");
    let b = bounds(&s, long);
    assert!((b.width() - 80.0).abs() <= 4.0, "fixed width 80: {b:?}");
}

/// Third click in the open editor: the paragraph around the caret is selected.
#[test]
fn select_paragraph_in_open_editor() {
    let mut s = doc_session(400.0, 300.0);
    s.set_tool(ToolId::Text).expect("text tool");
    events(&mut s, &[ToolEvent::down(50.0, 80.0), ToolEvent::up(50.0, 80.0)]);
    send(&mut s, ToolInput::Text("one two".into()));
    send(&mut s, ToolInput::Key(ToolKey::Enter));
    send(&mut s, ToolInput::Text("three four".into()));
    send(&mut s, ToolInput::TextCaret { index: 10, extend: false });
    send(&mut s, ToolInput::Edit(fp_engine::tools::EditOp::SelectParagraph));
    send(&mut s, ToolInput::Text("X".into()));
    s.tool_event(&ToolEvent::key(ToolKey::Escape)).expect("Esc");
    let t = texts(&s);
    assert_eq!(t.len(), 1, "{t:?}");
    assert_eq!(t[0].replace('\r', "\n").replace("\n\n", "\n"), "one two\nX", "{t:?}");
}
