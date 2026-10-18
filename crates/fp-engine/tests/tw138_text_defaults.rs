//! TW-138: attributes set while the text editor is open change only the marked text, never the
//! defaults of the text tool. Defaults (app-wide prefs `Text_*`) change only with no editor open:
//! text objects selected with the pointer, or the text tool with nothing selected.

use crate::common::{doc_session, ex, select, top_elements};
use fp_color::Argb;
use fp_engine::tools::{ToolId, ToolInput};
use fp_engine::{Session, ToolEvent, ToolKey};
use fp_model::{ElementId, ElementKind};
use serde_json::json;

const BLACK: Argb = Argb(0xff00_0000);

fn send(s: &mut Session, i: ToolInput) {
    s.tool_event(&ToolEvent {
        input: i,
        mods: Default::default(),
        pressure: 1.0,
        time_ms: 0,
    })
    .expect("event");
}

fn text_ids(s: &Session) -> Vec<ElementId> {
    top_elements(s)
        .iter()
        .filter(|e| matches!(e.kind, ElementKind::Text(_)))
        .map(|e| e.id)
        .collect()
}

/// Size and colour of the first character of text `id`.
fn first_attrs(s: &Session, id: ElementId) -> (f32, Argb) {
    let e = top_elements(s)
        .into_iter()
        .find(|e| e.id == id)
        .expect("text");
    let ElementKind::Text(t) = &e.kind else {
        panic!("no text")
    };
    let runs = t.resolved_runs();
    let a = runs.first().map(|(a, _)| a.clone()).expect("run");
    (
        a.size.unwrap_or(12.0),
        a.color.map(Argb).unwrap_or(t.attrs.fill_color),
    )
}

/// Last run (the end of the text) of text `id`.
fn last_attrs(s: &Session, id: ElementId) -> (f32, Argb) {
    let e = top_elements(s)
        .into_iter()
        .find(|e| e.id == id)
        .expect("text");
    let ElementKind::Text(t) = &e.kind else {
        panic!("no text")
    };
    let runs = t.resolved_runs();
    let a = runs.last().map(|(a, _)| a.clone()).expect("run");
    (
        a.size.unwrap_or(12.0),
        a.color.map(Argb).unwrap_or(t.attrs.fill_color),
    )
}

/// Text tool: click at (x, y), type, Escape. Returns the id of the new text.
fn new_text(s: &mut Session, x: f32, y: f32, text: &str) -> ElementId {
    let before = text_ids(s);
    s.set_tool(ToolId::Text).expect("text tool");
    s.tool_event(&ToolEvent::down(x, y)).expect("down");
    s.tool_event(&ToolEvent::up(x, y)).expect("up");
    assert!(s.tool_is_editing_text(), "editor open");
    send(s, ToolInput::Text(text.into()));
    s.tool_event(&ToolEvent::key(ToolKey::Escape)).expect("Esc");
    *text_ids(s)
        .iter()
        .find(|id| !before.contains(id))
        .expect("new text")
}

fn setup() -> Session {
    fp_i18n::set_language("de");
    doc_session(500.0, 400.0)
}

#[test]
fn factory_defaults() {
    let mut s = setup();
    assert_eq!(s.tools.options.text_size, 12.0);
    assert_eq!(s.tools.options.text_color, BLACK);
    assert_eq!(s.prefs.get_real("Text_Leading", 1.2), 1.2);
    assert_eq!(s.prefs.get_int("Text_Justification", 0), 0);
    let id = new_text(&mut s, 50.0, 80.0, "Abc");
    assert_eq!(first_attrs(&s, id), (12.0, BLACK));
}

#[test]
fn partial_selection_in_editor_keeps_the_defaults() {
    let mut s = setup();
    s.set_tool(ToolId::Text).expect("text tool");
    s.tool_event(&ToolEvent::down(50.0, 80.0)).expect("down");
    s.tool_event(&ToolEvent::up(50.0, 80.0)).expect("up");
    send(&mut s, ToolInput::Text("Abc 123".into()));
    // mark "123"
    send(&mut s, ToolInput::TextCaret { index: 4, extend: false });
    send(&mut s, ToolInput::TextCaret { index: 7, extend: true });
    ex(&mut s, "text.applyAttributes", json!({"size": 72}));
    ex(&mut s, "text.applyAttributes", json!({"color": "#a0a0a0"}));
    // the inspector's other entries and the menu behave the same while the editor is open
    ex(&mut s, "text.setAttributes", json!({"size": 60}));
    ex(&mut s, "text.size.other", json!({"size": 60}));
    assert!(s.tool_is_editing_text(), "editor stays open");
    assert_eq!(s.tools.options.text_size, 12.0, "size default unchanged");
    assert_eq!(s.tools.options.text_color, BLACK, "colour default unchanged");
    s.tool_event(&ToolEvent::key(ToolKey::Escape)).expect("Esc");
    let first = text_ids(&s)[0];
    assert_eq!(first_attrs(&s, first), (12.0, BLACK), "\"Abc \" untouched");
    assert_eq!(
        last_attrs(&s, first),
        (60.0, Argb(0xffa0_a0a0)),
        "\"123\" changed"
    );
    // neither the commit nor a tool switch take the attributes over
    s.set_tool(ToolId::Pointer).expect("pointer");
    assert_eq!(s.tools.options.text_size, 12.0);
    assert_eq!(s.prefs.get_real("Text_Size", 12.0), 12.0);
    assert_eq!(s.prefs.get_color("Text_FillColor", BLACK), BLACK);
    let next = new_text(&mut s, 50.0, 250.0, "Abc 123");
    assert_eq!(first_attrs(&s, next), (12.0, BLACK), "next text keeps old defaults");
    assert_eq!(last_attrs(&s, next), (12.0, BLACK));
}

#[test]
fn pointer_selected_text_updates_the_defaults() {
    let mut s = setup();
    let v = ex(
        &mut s,
        "draw.text",
        json!({"text": "Abc", "anchor": [50, 80], "size": 24, "color": "#000000"}),
    );
    let id = ElementId(v["id"].as_u64().unwrap_or(0));
    s.set_tool(ToolId::Pointer).expect("pointer");
    select(&mut s, &[id]);
    assert!(!s.tool_is_editing_text());
    ex(
        &mut s,
        "text.applyAttributes",
        json!({"size": 30, "color": "#ff0000", "bold": true}),
    );
    assert_eq!(first_attrs(&s, id), (30.0, Argb(0xffff_0000)), "text changed");
    assert_eq!(s.tools.options.text_size, 30.0);
    assert_eq!(s.tools.options.text_color, Argb(0xffff_0000));
    assert_eq!(s.prefs.get_real("Text_Size", 12.0), 30.0);
    assert_eq!(s.prefs.get_color("Text_FillColor", BLACK), Argb(0xffff_0000));
    assert!(s.prefs.get_bool("Text_Bold", false));
    ex(&mut s, "select.none", json!({}));
    let next = new_text(&mut s, 50.0, 250.0, "Xyz");
    assert_eq!(first_attrs(&s, next), (30.0, Argb(0xffff_0000)));
}

#[test]
fn text_tool_without_selection_updates_the_defaults() {
    let mut s = setup();
    s.set_tool(ToolId::Text).expect("text tool");
    assert!(!s.tool_is_editing_text());
    let n = s.active().unwrap().history.len();
    ex(
        &mut s,
        "text.applyAttributes",
        json!({"size": 20, "color": "#0000ff", "alignment": 1, "leading": 1.5}),
    );
    assert_eq!(s.active().unwrap().history.len(), n, "no history row");
    assert_eq!(s.tools.options.text_size, 20.0);
    assert_eq!(s.prefs.get_real("Text_Size", 12.0), 20.0);
    assert_eq!(s.prefs.get_int("Text_Justification", 0), 1);
    assert_eq!(s.prefs.get_real("Text_Leading", 1.2), 1.5);
    let next = new_text(&mut s, 50.0, 250.0, "Xyz");
    assert_eq!(first_attrs(&s, next), (20.0, Argb(0xff00_00ff)));
}
