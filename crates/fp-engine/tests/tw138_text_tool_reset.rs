//! TW-138 rest R3 (re 56): the Properties panel writes the paragraph defaults in every state (pref
//! keys and scaling: indent px, spacing px, leading factor or px with its mode, horizontal scale
//! factor, tracking field value / 100). Activating the text tool resets them (R5) unless
//! `TextEditor_RememberALLSettings` is on; new text gets tracking 0 (R6); every field change is a
//! "Text change" row, also with nothing selected, the open editor ends as one row (R7).

use crate::common::{doc_session, el, ex, select};
use fp_engine::Session;
use fp_engine::prefs::PrefValue;
use fp_engine::tools::{ToolId, ToolInput};
use fp_engine::{ToolEvent, ToolKey};
use fp_model::{ElementId, ElementKind};
use serde_json::json;

fn setup() -> Session {
    fp_i18n::set_language("de");
    doc_session(500.0, 400.0)
}

fn text(s: &mut Session) -> ElementId {
    let v = ex(
        s,
        "draw.text",
        json!({"text": "Abc", "anchor": [50, 80], "size": 30, "color": "#000000"}),
    );
    ElementId(v["id"].as_u64().unwrap_or(0))
}

fn real(s: &Session, k: &str) -> f64 {
    s.prefs.get_real(k, -1.0)
}

fn rows(s: &Session) -> Vec<String> {
    let d = s.active().expect("doc");
    d.history.steps().iter().map(|st| s.step_label(st)).collect()
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

/// The panel values of case b (re 56 section 2.2), as the Properties panel sends them.
fn panel_values(s: &mut Session) {
    for p in [
        json!({"indent": 41}),
        json!({"spaceBefore": 24}),
        json!({"spaceAfter": 26}),
        json!({"leading": 1.7, "leadingMode": 0}),
        json!({"tracking": 45}),
        json!({"horizontalScale": 1.3}),
    ] {
        ex(s, "text.applyAttributes", p);
    }
}

fn assert_panel_values(s: &Session) {
    assert_eq!(real(s, "Text_ParagraphIndent"), 41.0);
    assert_eq!(real(s, "Text_ParagraphSpacingBefore"), 24.0);
    assert_eq!(real(s, "Text_ParagraphSpacingAfter"), 26.0);
    assert!((real(s, "Text_Leading") - 1.7).abs() < 1e-6);
    assert_eq!(s.prefs.get_int("Text_LeadingMode", -1), 0);
    assert!((real(s, "Text_RangeKerning") - 0.45).abs() < 1e-9, "field / 100");
    assert!((real(s, "Text_HorizScale") - 1.3).abs() < 1e-6);
}

fn assert_reset(s: &Session) {
    assert_eq!(real(s, "Text_ParagraphIndent"), 0.0);
    assert_eq!(real(s, "Text_ParagraphSpacingBefore"), 0.0);
    assert_eq!(real(s, "Text_ParagraphSpacingAfter"), 0.0);
    assert!((real(s, "Text_Leading") - 1.2).abs() < 1e-9);
    assert_eq!(s.prefs.get_int("Text_LeadingMode", -1), 0);
    assert_eq!(real(s, "Text_RangeKerning"), 0.0);
    assert_eq!(real(s, "Text_HorizScale"), 1.0);
    assert_eq!(real(s, "Text_BaselineShift"), 0.0);
}

#[test]
fn panel_writes_paragraph_defaults_in_all_states() {
    // case a: nothing selected, text tool active
    let mut s = setup();
    s.set_tool(ToolId::Text).expect("text tool");
    panel_values(&mut s);
    assert_panel_values(&s);
    // case b: text selected with the pointer
    let mut s = setup();
    let id = text(&mut s);
    select(&mut s, &[id]);
    panel_values(&mut s);
    assert_panel_values(&s);
    ex(&mut s, "text.applyAttributes", json!({"leading": 1.4}));
    assert!((real(&s, "Text_Leading") - 1.4).abs() < 1e-6, "140 % -> 1.4");
    ex(&mut s, "text.applyAttributes", json!({"horizontalScale": 1.1}));
    assert!((real(&s, "Text_HorizScale") - 1.1).abs() < 1e-6, "110 % -> 1.1");
    ex(&mut s, "text.applyAttributes", json!({"tracking": 15}));
    assert!((real(&s, "Text_RangeKerning") - 0.15).abs() < 1e-9, "15 -> 0.15");
}

/// re 56 section 2.5: % -> px at 30 pt and 120 % gives 36 and `Text_LeadingMode` 1; px -> %
/// divides by the size.
#[test]
fn leading_unit_switch_converts_and_writes_mode() {
    use fp_engine::commands::convert_leading;
    assert_eq!(convert_leading(1.2, 30.0, false, true), 36.0);
    assert_eq!(convert_leading(1.234, 30.0, false, true), 37.0, "rounded");
    assert!((convert_leading(36.0, 30.0, true, false) - 1.2).abs() < 1e-6);
    assert_eq!(convert_leading(40.0, 30.0, true, true), 40.0);
    let mut s = setup();
    s.set_tool(ToolId::Text).expect("text tool");
    let v = convert_leading(1.2, 30.0, false, true);
    ex(&mut s, "text.applyAttributes", json!({"leading": v, "leadingMode": 1}));
    assert_eq!(real(&s, "Text_Leading"), 36.0);
    assert_eq!(s.prefs.get_int("Text_LeadingMode", -1), 1);
    ex(&mut s, "text.applyAttributes", json!({"leading": 40, "leadingMode": 1}));
    // new text "Px": leading 40, exact mode
    let id = text(&mut s);
    let ElementKind::Text(t) = &el(&s, id).kind else {
        panic!("no text");
    };
    let a = t.resolved_runs()[0].0.clone();
    assert_eq!((a.leading, a.leading_mode), (Some(40.0), Some(1)));
}

#[test]
fn activating_text_tool_resets_paragraph_defaults() {
    let mut s = setup();
    // case b with the pointer, then the text tool: everything back, the size stays
    let id = text(&mut s);
    select(&mut s, &[id]);
    ex(&mut s, "text.applyAttributes", json!({"size": 30}));
    panel_values(&mut s);
    s.prefs.set("Text_BaselineShift", PrefValue::Real(3.0));
    s.set_tool(ToolId::Text).expect("text tool");
    assert_reset(&s);
    assert_eq!(real(&s, "Text_Size"), 30.0, "size kept");
    // DefaultLeading is the reset value
    s.prefs.set("DefaultLeading", PrefValue::Real(1.5));
    s.set_tool(ToolId::Pointer).expect("pointer");
    s.set_tool(ToolId::Text).expect("text tool");
    assert!((real(&s, "Text_Leading") - 1.5).abs() < 1e-9);
}

#[test]
fn values_set_with_text_tool_active_survive() {
    let mut s = setup();
    s.set_tool(ToolId::Text).expect("text tool");
    // case a
    panel_values(&mut s);
    // re-selecting the active tool is no activation
    s.set_tool(ToolId::Text).expect("text tool");
    assert_panel_values(&s);
    // case b': text selected while the text tool is active
    let id = text(&mut s);
    select(&mut s, &[id]);
    ex(&mut s, "text.applyAttributes", json!({"indent": 51}));
    assert_eq!(real(&s, "Text_ParagraphIndent"), 51.0);
    // new text takes them (except the tracking, R6)
    let n = text(&mut s);
    let ElementKind::Text(t) = &el(&s, n).kind else {
        panic!("no text");
    };
    let a = t.resolved_runs()[0].0.clone();
    assert_eq!(a.indent, Some(51.0));
    assert_eq!(a.space_before, Some(24.0));
    assert_eq!(a.space_after, Some(26.0));
    assert!((a.horizontal_scale.unwrap_or(1.0) - 1.3).abs() < 1e-6);
    assert!((a.leading.unwrap_or(0.0) - 1.7).abs() < 1e-6);
    assert_eq!(a.tracking.unwrap_or(0.0), 0.0, "R6: tracking 0");
    // switching to the pointer keeps the values, back to the text tool resets them
    s.set_tool(ToolId::Pointer).expect("pointer");
    assert_eq!(real(&s, "Text_ParagraphIndent"), 51.0);
    s.set_tool(ToolId::Text).expect("text tool");
    assert_reset(&s);
}

#[test]
fn remember_all_settings_keeps_everything() {
    let mut s = setup();
    s.prefs
        .set("TextEditor_RememberALLSettings", PrefValue::Bool(true));
    let id = text(&mut s);
    select(&mut s, &[id]);
    panel_values(&mut s);
    s.set_tool(ToolId::Text).expect("text tool");
    assert_panel_values(&s);
}

#[test]
fn new_text_gets_tracking_zero() {
    let mut s = setup();
    s.set_tool(ToolId::Text).expect("text tool");
    ex(&mut s, "text.applyAttributes", json!({"tracking": 15}));
    // script/agent path
    let id = text(&mut s);
    let ElementKind::Text(t) = &el(&s, id).kind else {
        panic!("no text");
    };
    assert!(t.resolved_runs().iter().all(|(a, _)| a.tracking.unwrap_or(0.0) == 0.0));
    assert!(t.resolved_runs().iter().all(|(a, _)| a.kerning.unwrap_or(0.0) == 0.0));
    // text tool path
    s.tool_event(&ToolEvent::down(300.0, 300.0)).expect("down");
    s.tool_event(&ToolEvent::up(300.0, 300.0)).expect("up");
    send(&mut s, ToolInput::Text("Px".into()));
    s.tool_event(&ToolEvent::key(ToolKey::Escape)).expect("Esc");
    let d = s.active().expect("doc");
    let texts: Vec<_> = fp_engine::docops::selectable_ids(&d.doc, d.page, d.frame)
        .into_iter()
        .filter_map(|i| fp_engine::docops::element_on_page(&d.doc, d.page, i))
        .filter_map(|e| match &e.kind {
            ElementKind::Text(t) if t.plain_text() == "Px" => Some(t.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(texts.len(), 1);
    assert!(
        texts[0]
            .resolved_runs()
            .iter()
            .all(|(a, _)| a.tracking.unwrap_or(0.0) == 0.0)
    );
    assert!((real(&s, "Text_RangeKerning") - 0.15).abs() < 1e-9, "pref kept");
}

#[test]
fn every_field_change_is_a_text_change_row() {
    // case a: nothing selected
    let mut s = setup();
    s.set_tool(ToolId::Text).expect("text tool");
    let n = rows(&s).len();
    panel_values(&mut s);
    let r = rows(&s);
    assert_eq!(r.len(), n + 6, "one row per field: {r:?}");
    assert!(r[n..].iter().all(|l| l == "Textänderung"), "{r:?}");
    // case b: text selected with the pointer
    let mut s = setup();
    let id = text(&mut s);
    select(&mut s, &[id]);
    s.set_tool(ToolId::Pointer).expect("pointer");
    let n = rows(&s).len();
    panel_values(&mut s);
    let r = rows(&s);
    assert_eq!(r.len(), n + 6, "{r:?}");
    assert!(r[n..].iter().all(|l| l == "Textänderung"), "{r:?}");
    fp_i18n::set_language("en");
    assert_eq!(rows(&s).last().map(String::as_str), Some("Text Change"));
    fp_i18n::set_language("de");
}

#[test]
fn open_editor_session_is_one_row() {
    let mut s = setup();
    let id = text(&mut s);
    select(&mut s, &[id]);
    s.set_tool(ToolId::Text).expect("text tool");
    let b = crate::common::bounds(&s, id);
    let c = ((b.left + b.right) / 2.0, (b.top + b.bottom) / 2.0);
    s.tool_event(&ToolEvent::down(c.0, c.1)).expect("down");
    s.tool_event(&ToolEvent::up(c.0, c.1)).expect("up");
    assert!(s.tool_is_editing_text(), "editing");
    let n = rows(&s).len();
    send(&mut s, ToolInput::Edit(fp_engine::tools::EditOp::SelectAll));
    panel_values(&mut s);
    assert_eq!(rows(&s).len(), n, "no row while the editor is open");
    assert_panel_values(&s);
    s.tool_event(&ToolEvent::key(ToolKey::Escape)).expect("Esc");
    let r = rows(&s);
    assert_eq!(r.len(), n + 1, "{r:?}");
    assert_eq!(r.last().map(String::as_str), Some("Textänderung"));
}
