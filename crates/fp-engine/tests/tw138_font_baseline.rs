//! TW-138 rest (re 53 section 2): the Properties panel never writes `Text_BaselineShift` (R1); a
//! font change from the Text menu or the panel writes `Text_CoolType_Font` (PostScript name) and
//! puts the family at the front of `RecentFontList` (R2); a style from the Styles panel applied
//! to text writes the text defaults like a text attribute command (R4).

use crate::common::{doc_session, ex, select};
use fp_engine::Session;
use fp_engine::tools::ToolId;
use fp_model::ElementId;
use serde_json::json;

fn setup() -> Session {
    fp_i18n::set_language("de");
    doc_session(500.0, 400.0)
}

fn text(s: &mut Session) -> ElementId {
    let v = ex(
        s,
        "draw.text",
        json!({"text": "Abc", "anchor": [50, 80], "size": 24, "color": "#000000"}),
    );
    ElementId(v["id"].as_u64().unwrap_or(0))
}

/// PostScript name and family the font database gives for `name` (the name itself if unknown).
fn ps_and_family(name: &str) -> (String, String) {
    let db = fp_text::FontDb::shared();
    let id = db
        .find_ps(name)
        .or_else(|| db.find_family(name, false, false).map(|(i, _)| i));
    match id.and_then(|i| db.faces().get(i)) {
        Some(f) => (f.names.ps_name.clone(), f.names.family.clone()),
        None => (name.to_string(), name.to_string()),
    }
}

#[test]
fn text_defaults_panel_never_writes_baseline_shift() {
    let mut s = setup();
    let id = text(&mut s);
    s.set_tool(ToolId::Pointer).expect("pointer");
    select(&mut s, &[id]);
    ex(&mut s, "text.applyAttributes", json!({"baselineShift": 5, "tracking": 20}));
    assert!(!s.prefs.values.contains_key("Text_BaselineShift"), "pointer selection");
    assert_eq!(s.prefs.get_real("Text_RangeKerning", 0.0), 20.0, "tracking still written");
    ex(&mut s, "select.none", json!({}));
    s.set_tool(ToolId::Text).expect("text tool");
    ex(&mut s, "text.applyAttributes", json!({"baselineShift": 7}));
    assert!(!s.prefs.values.contains_key("Text_BaselineShift"), "without text");
    // the explicit default setter (scripts) still may set it
    ex(&mut s, "text.setToolDefaults", json!({"baselineShift": 3}));
    assert_eq!(s.prefs.get_real("Text_BaselineShift", 0.0), 3.0);
}

#[test]
fn text_defaults_font_from_menu_writes_cooltype_font_and_recent_list() {
    let mut s = setup();
    let (ps, fam) = ps_and_family("Arial");
    s.prefs.set("RecentFontList", fp_engine::prefs::PrefValue::Str("(Verdana)(Times New Roman)".into()));
    // Text ▸ Font without text: defaults only
    ex(&mut s, "text.font", json!({"font": "Arial"}));
    assert_eq!(s.prefs.get_str("Text_CoolType_Font", ""), ps, "PostScript name");
    assert_eq!(s.prefs.get_str("Text_Font", ""), fam, "family stays for new text");
    assert_eq!(s.prefs.get_str("RecentFontList", ""), format!("({fam})(Verdana)(Times New Roman)"));
    // again with a pointer-selected text: moves to the front, no duplicate
    let id = text(&mut s);
    s.set_tool(ToolId::Pointer).expect("pointer");
    select(&mut s, &[id]);
    ex(&mut s, "text.font", json!({"font": "Verdana"}));
    let (vps, vfam) = ps_and_family("Verdana");
    assert_eq!(s.prefs.get_str("Text_CoolType_Font", ""), vps);
    assert_eq!(s.prefs.get_str("RecentFontList", ""), format!("({vfam})({fam})(Times New Roman)"));
}

#[test]
fn text_defaults_font_from_panel_writes_cooltype_font_and_recent_list() {
    let mut s = setup();
    let id = text(&mut s);
    s.set_tool(ToolId::Pointer).expect("pointer");
    select(&mut s, &[id]);
    let (ps, fam) = ps_and_family("Arial");
    ex(&mut s, "text.applyAttributes", json!({"font": ps}));
    assert_eq!(s.prefs.get_str("Text_CoolType_Font", ""), ps);
    assert_eq!(s.prefs.get_str("RecentFontList", ""), format!("({fam})"));
    // the recent list keeps at most RecentFontLimit entries
    s.prefs.set("RecentFontLimit", fp_engine::prefs::PrefValue::Int(2));
    s.prefs.set("RecentFontList", fp_engine::prefs::PrefValue::Str("(A)(B)".into()));
    ex(&mut s, "text.applyAttributes", json!({"font": ps}));
    assert_eq!(s.prefs.get_str("RecentFontList", ""), format!("({fam})(A)"));
}

#[test]
fn text_defaults_style_applied_to_text_writes_defaults() {
    let mut s = setup();
    let id = text(&mut s);
    s.set_tool(ToolId::Pointer).expect("pointer");
    select(&mut s, &[id]);
    let (ps, fam) = ps_and_family("Arial");
    ex(&mut s, "text.setAttributes", json!({"font": ps, "bold": true, "italic": true, "leading": 1.5, "size": 40}));
    // style with font, size and font style groups taken from the text
    ex(&mut s, "pi.style.new", json!({"name": "T", "otherFlags": 7}));
    // reset the defaults, then apply the style to a second text
    for k in ["Text_Bold", "Text_Italic", "Text_Font", "Text_CoolType_Font", "Text_Leading", "Text_Size"] {
        s.prefs.values.remove(k);
    }
    let other = text(&mut s);
    ex(&mut s, "pi.style.apply", json!({"name": "T", "ids": [other.0]}));
    assert!(s.prefs.get_bool("Text_Bold", false), "bold");
    assert!(s.prefs.get_bool("Text_Italic", false), "italic");
    assert_eq!(s.prefs.get_str("Text_CoolType_Font", ""), ps, "font");
    assert_eq!(s.prefs.get_str("Text_Font", ""), fam);
    assert_eq!(s.prefs.get_real("Text_Leading", 1.2), 1.5, "leading");
    assert!(!s.prefs.values.contains_key("Text_Size"), "size is not a style default");
    // a style applied to a non-text object writes nothing
    s.prefs.values.remove("Text_Bold");
    let r = ex(&mut s, "draw.rectangle", json!({"rect": [300, 300, 350, 350]}));
    ex(&mut s, "pi.style.apply", json!({"name": "T", "ids": [r["id"]]}));
    assert!(!s.prefs.values.contains_key("Text_Bold"));
}
