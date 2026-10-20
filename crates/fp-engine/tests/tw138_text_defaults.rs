//! TW-138 (re 50 section 2): every text attribute command writes its `Text_*` default, with the
//! editor open (also for a partial selection) and with text selected by the pointer; only
//! Larger/Smaller write nothing. New text reads every `Text_*` default except `Text_FillColor`:
//! its colour is the current fill colour of the text tool.

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

/// Marks "123" of a new "Abc 123" in the open editor.
fn editor_with_partial_selection(s: &mut Session) {
    s.set_tool(ToolId::Text).expect("text tool");
    s.tool_event(&ToolEvent::down(50.0, 80.0)).expect("down");
    s.tool_event(&ToolEvent::up(50.0, 80.0)).expect("up");
    send(s, ToolInput::Text("Abc 123".into()));
    send(s, ToolInput::TextCaret { index: 4, extend: false });
    send(s, ToolInput::TextCaret { index: 7, extend: true });
}

#[test]
fn partial_selection_size_becomes_the_default() {
    let mut s = setup();
    editor_with_partial_selection(&mut s);
    ex(&mut s, "text.applyAttributes", json!({"size": 72}));
    assert!(s.tool_is_editing_text(), "editor stays open");
    assert_eq!(s.prefs.get_real("Text_Size", 12.0), 72.0, "Text_Size written");
    assert_eq!(s.tools.options.text_size, 72.0);
    s.tool_event(&ToolEvent::key(ToolKey::Escape)).expect("Esc");
    let first = text_ids(&s)[0];
    assert_eq!(first_attrs(&s, first).0, 12.0, "\"Abc \" untouched");
    assert_eq!(last_attrs(&s, first).0, 72.0, "\"123\" changed");
    s.set_tool(ToolId::Pointer).expect("pointer");
    let next = new_text(&mut s, 50.0, 250.0, "Xyz");
    assert_eq!(first_attrs(&s, next).0, 72.0, "next text is 72 pt");
}

#[test]
fn partial_selection_colour_does_not_colour_new_text() {
    let mut s = setup();
    editor_with_partial_selection(&mut s);
    ex(&mut s, "text.applyAttributes", json!({"color": "#a0a0a0"}));
    assert_eq!(
        s.prefs.get_color("Text_FillColor", BLACK),
        Argb(0xffa0_a0a0),
        "Text_FillColor written"
    );
    assert_eq!(s.tools.options.text_color, BLACK, "fill colour of the tool stays");
    s.tool_event(&ToolEvent::key(ToolKey::Escape)).expect("Esc");
    let first = text_ids(&s)[0];
    assert_eq!(first_attrs(&s, first).1, BLACK);
    assert_eq!(last_attrs(&s, first).1, Argb(0xffa0_a0a0), "\"123\" grey");
    s.set_tool(ToolId::Pointer).expect("pointer");
    let next = new_text(&mut s, 50.0, 250.0, "Xyz");
    assert_eq!(first_attrs(&s, next).1, BLACK, "next text keeps the fill colour");
    // a restart does not read `Text_FillColor` either
    let opts = fp_engine::tools::ToolOptions::from_prefs(&s.prefs);
    assert_eq!(opts.text_color, BLACK);
}

#[test]
fn editor_menu_commands_write_the_defaults() {
    let mut s = setup();
    editor_with_partial_selection(&mut s);
    ex(&mut s, "text.size.other", json!({"size": 60}));
    ex(&mut s, "text.style.fauxBold", json!({}));
    ex(&mut s, "text.align.right", json!({}));
    assert!(s.tool_is_editing_text());
    assert_eq!(s.prefs.get_real("Text_Size", 12.0), 60.0);
    assert!(s.prefs.get_bool("Text_Bold", false));
    assert_eq!(s.prefs.get_int("Text_Justification", 0), 2);
    // the script setter writes nothing
    ex(&mut s, "text.setAttributes", json!({"size": 30}));
    assert_eq!(s.prefs.get_real("Text_Size", 12.0), 60.0);
    s.tool_event(&ToolEvent::key(ToolKey::Escape)).expect("Esc");
    s.set_tool(ToolId::Pointer).expect("pointer");
    let next = new_text(&mut s, 50.0, 250.0, "Xyz");
    let e = top_elements(&s).into_iter().find(|e| e.id == next).expect("text");
    let ElementKind::Text(t) = &e.kind else { panic!("no text") };
    let a = t.resolved_runs()[0].0.clone();
    assert_eq!((a.size, a.bold, a.alignment), (Some(60.0), Some(true), Some(2)));
}

#[test]
fn larger_smaller_write_no_default() {
    let mut s = setup();
    let v = ex(
        &mut s,
        "draw.text",
        json!({"text": "Abc", "anchor": [50, 80], "size": 24}),
    );
    let id = ElementId(v["id"].as_u64().unwrap_or(0));
    s.set_tool(ToolId::Pointer).expect("pointer");
    select(&mut s, &[id]);
    ex(&mut s, "text.size.larger", json!({}));
    assert_eq!(first_attrs(&s, id).0, 25.0);
    assert_eq!(s.prefs.get_real("Text_Size", 12.0), 12.0, "pointer: unchanged");
    assert_eq!(s.tools.options.text_size, 12.0);
    editor_with_partial_selection(&mut s);
    ex(&mut s, "text.size.larger", json!({}));
    ex(&mut s, "text.size.smaller", json!({}));
    ex(&mut s, "text.size.smaller", json!({}));
    assert_eq!(s.prefs.get_real("Text_Size", 12.0), 12.0, "editor: unchanged");
    assert_eq!(s.tools.options.text_size, 12.0);
}

#[test]
fn text_menu_bold_with_pointer_selection_writes_the_default() {
    let mut s = setup();
    let v = ex(
        &mut s,
        "draw.text",
        json!({"text": "Abc", "anchor": [50, 80], "size": 24}),
    );
    let id = ElementId(v["id"].as_u64().unwrap_or(0));
    s.set_tool(ToolId::Pointer).expect("pointer");
    select(&mut s, &[id]);
    ex(&mut s, "text.style.fauxBold", json!({}));
    assert!(s.prefs.get_bool("Text_Bold", false), "Text_Bold written");
    ex(&mut s, "text.style.fauxBold", json!({}));
    assert!(!s.prefs.get_bool("Text_Bold", true), "toggled off again");
    ex(&mut s, "text.size.36", json!({}));
    assert_eq!(s.prefs.get_real("Text_Size", 12.0), 36.0);
    ex(&mut s, "text.align.centeredHorizontally", json!({}));
    assert_eq!(s.prefs.get_int("Text_Justification", 0), 1);
    ex(&mut s, "text.style.underline", json!({}));
    ex(&mut s, "text.style.plain", json!({}));
    assert!(!s.prefs.get_bool("Text_Underline", true));
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
    // R7 (re 56 section 3): one "Text change" row also with nothing selected
    assert_eq!(s.active().unwrap().history.len(), n + 1, "one history row");
    assert_eq!(s.tools.options.text_size, 20.0);
    assert_eq!(s.prefs.get_real("Text_Size", 12.0), 20.0);
    assert_eq!(s.prefs.get_int("Text_Justification", 0), 1);
    assert_eq!(s.prefs.get_real("Text_Leading", 1.2), 1.5);
    let next = new_text(&mut s, 50.0, 250.0, "Xyz");
    assert_eq!(first_attrs(&s, next), (20.0, Argb(0xff00_00ff)));
}
