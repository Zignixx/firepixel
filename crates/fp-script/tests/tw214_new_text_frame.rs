//! TW-214 (re 63 section 2): placement of text from `addNewText(rect, bInitFromPrefs)`, its
//! conversion to point text (`autoExpand = true`) and the text tool click. Reference values
//! measured on a 600 x 400 document with the rect `{100,100,300,200}`; script `left/top/width/height`
//! is the layout frame plus 2 px on each side.
//!
//! - The second argument is `bInitFromPrefs`: `true` = the text tool defaults, `false` = the factory
//!   defaults (MyriadPro-Regular 12 pt); both make a fixed-width frame (`autoExpand == false`).
//! - Fixed frame: first baseline `rect.top + round(d·s)` (d = top of the glyph "d"), glyphs on whole
//!   pixels; the first line is `ceil((d + descent)·s)` high (Arial 12/30/60 -> 13/32/63, Myriad Pro
//!   60 -> 58, Times 60 -> 61), each further line adds leading·s.
//! - Point text: top and left edge of the object stay; with Arial ("A" has a negative left
//!   bearing) the ink moves 1 px right; height `ceil(n·leading·s) + 4`.
//! - Text tool click (x, y): y is the first baseline, object top `y - ceil(asc·s) - 2`.

use fp_engine::tools::{ToolId, ToolInput};
use fp_engine::{Session, ToolEvent, ToolKey};
use fp_script::{ScriptOptions, run_source};

fn session() -> Session {
    fp_i18n::profile::use_test_profile();
    Session::default()
}

fn has_font(ps: &str) -> bool {
    let ok = fp_text::FontDb::shared().find_ps(ps).is_some();
    if !ok {
        eprintln!("skipped: {ps} not installed");
    }
    ok
}

fn result(s: &mut Session, src: &str) -> String {
    let o = run_source(s, src, &ScriptOptions::default());
    assert!(
        o.ok,
        "error: {:?}\ndiagnostics: {:?}",
        o.error, o.diagnostics
    );
    o.result.unwrap_or_default()
}

const NEW: &str = "var dom = fw.createFirepixelDocument({x: 600, y: 400}, {pixelsPerUnit: 72, units: 'inch'}, '#ffffff'); dom = fw.getDocumentDOM();\n";

/// `left,top,width,height` and `pixelRect` of the selected text, then `autoExpand`.
const READ: &str = "var t = fw.selection[0], p = t.pixelRect; [t.left, t.top, t.width, t.height, p.left, p.top, p.right, p.bottom, t.autoExpand].join(',')";

/// `addNewText`, then text, face and size, optionally `autoExpand = true`.
fn case(prefs: bool, face: &str, size: u32, text: &str, auto: bool) -> String {
    let mut s = session();
    result(
        &mut s,
        &format!(
            "{NEW} var d = fw.getDocumentDOM(); d.selectNone(); d.addNewText({{left: 100, top: 100, right: 300, bottom: 200}}, {prefs});\n\
             var o = fw.selection[0]; o.textChars = '{text}'; var R = o.textRuns;\n\
             R.initialAttrs.face = '{face}'; R.initialAttrs.size = '{size}pt'; R.initialAttrs.fillColor = '#000000'; R.initialAttrs.alignment = 'left';\n\
             R.textRuns[0].changedAttrs = {{}}; o.textRuns = R; o = fw.selection[0]; if ({auto}) o.autoExpand = true;\n{READ}"
        ),
    )
}

const LINES: &str = "Zeile 1\\rZeile 2\\rZeile 3";

#[test]
fn add_new_text_makes_a_fixed_frame_with_either_flag() {
    if !has_font("ArialMT") || !has_font("MyriadPro-Regular") {
        return;
    }
    let mut s = session();
    // the text tool's defaults of the measurement: Arial 12 pt
    s.tools.options.text_font = "ArialMT".into();
    s.tools.options.text_size = 12.0;
    let probe = |s: &mut Session, flag: bool| {
        result(
            s,
            &format!(
                "{NEW} dom.selectNone(); dom.addNewText({{left: 100, top: 100, right: 300, bottom: 200}}, {flag});\n\
                 var t = fw.selection[0], a = t.textRuns.initialAttrs; [t.left, t.top, t.width, t.height, t.autoExpand, a.face, a.size].join(',')"
            ),
        )
    };
    // true: the text tool's defaults; false: the factory defaults; both fixed width
    assert_eq!(probe(&mut s, true), "98,98,204,17,false,ArialMT,12pt");
    assert_eq!(
        probe(&mut s, false),
        "98,98,204,16,false,MyriadPro-Regular,12pt"
    );
}

#[test]
fn fixed_frame_first_baseline_and_height_as_measured() {
    for (prefs, face, size, text, want) in [
        (true, "ArialMT", 12, "Ag", "98,98,204,17,100,100,114,112"),
        (true, "ArialMT", 30, "Ag", "98,98,204,36,100,99,135,128"),
        (true, "ArialMT", 60, "Ag", "98,98,204,67,100,100,169,156"),
        (true, "ArialMT", 12, LINES, "98,98,204,46,100,100,137,137"),
        (true, "ArialMT", 30, LINES, "98,98,204,108,100,99,190,194"),
        (true, "ArialMT", 60, LINES, "98,98,204,211,101,99,277,288"),
        (
            true,
            "TimesNewRomanPSMT",
            60,
            "Adg",
            "98,98,204,65,100,100,202,155",
        ),
    ] {
        if !has_font(face) {
            continue;
        }
        let v = case(prefs, face, size, text, false);
        assert_eq!(v, format!("{want},false"), "{face} {size} pt {text}");
    }
    // Myriad Pro 60 pt: frame 62 high, first baseline 143 (ink 102..156); the ink's right edge
    // (166 in the reference) is not part of this rule
    if has_font("MyriadPro-Regular") {
        let v = case(false, "MyriadPro-Regular", 60, "Ag", false);
        let n: Vec<&str> = v.split(',').collect();
        assert_eq!(
            (n[0], n[1], n[2], n[3], n[5], n[7]),
            ("98", "98", "204", "62", "102", "156"),
            "Myriad Pro 60 pt: {v}"
        );
    }
}

#[test]
fn point_text_keeps_top_and_left_edge() {
    for (prefs, face, size, text, want) in [
        // Arial: the ink moves 1 px right (100 -> 101), the object stays at 98
        (true, "ArialMT", 12, "Ag", "98,98,20,19,101,102,115,114"),
        (true, "ArialMT", 30, "Ag", "98,98,42,40,101,104,136,133"),
        (true, "ArialMT", 60, "Ag", "98,98,78,76,101,109,170,165"),
        (true, "ArialMT", 12, LINES, "98,98,41,48,100,102,137,140"),
        (true, "ArialMT", 30, LINES, "98,98,95,112,100,104,190,199"),
        (true, "ArialMT", 60, LINES, "98,98,183,220,101,108,277,297"),
        (
            true,
            "TimesNewRomanPSMT",
            60,
            "Adg",
            "98,98,107,73,100,108,202,163",
        ),
    ] {
        if !has_font(face) {
            continue;
        }
        let v = case(prefs, face, size, text, true);
        assert_eq!(v, format!("{want},true"), "{face} {size} pt {text}");
    }
    if has_font("MyriadPro-Regular") {
        let v = case(false, "MyriadPro-Regular", 60, "Ag", true);
        let n: Vec<&str> = v.split(',').collect();
        assert_eq!(
            (n[0], n[1], n[3], n[4], n[5], n[7]),
            ("98", "98", "70", "101", "110", "164"),
            "Myriad Pro 60 pt: {v}"
        );
    }
}

#[test]
fn zero_width_rect_makes_a_ten_pixel_frame_without_ink() {
    if !has_font("ArialMT") {
        return;
    }
    let mut s = session();
    s.tools.options.text_font = "Arial".into();
    s.tools.options.text_size = 12.0;
    let v = result(
        &mut s,
        &format!(
            "{NEW} dom.selectNone(); dom.addNewText({{left: 100, top: 100, right: 100, bottom: 100}}, true);\n\
             var o = fw.selection[0]; o.textChars = 'Ag'; o = fw.selection[0]; var p = o.pixelRect;\n\
             [o.left, o.top, o.width, o.autoExpand, p.left, p.top, p.right, p.bottom].join(',')"
        ),
    );
    // no ink: `pixelRect` is the empty rect (+-2^31) as in the reference; the height (726 in the
    // reference) is not reproduced
    assert_eq!(
        v,
        "98,98,14,false,-2147483648,-2147483648,2147483648,2147483648"
    );
}

fn text_tool_click(size: f32) -> String {
    let mut s = session();
    result(&mut s, NEW);
    s.tools.options.text_font = "Arial".into();
    s.tools.options.text_size = size;
    s.set_tool(ToolId::Text).expect("text tool");
    for e in [ToolEvent::down(100.0, 100.0), ToolEvent::up(100.0, 100.0)] {
        s.tool_event(&e).expect("click");
    }
    s.tool_event(&ToolEvent {
        input: ToolInput::Text("Ag".into()),
        mods: Default::default(),
        pressure: 1.0,
        time_ms: 0,
    })
    .expect("type");
    s.tool_event(&ToolEvent::key(ToolKey::Escape)).expect("Esc");
    result(
        &mut s,
        &format!("var d = fw.getDocumentDOM(); d.selectAll();\n{READ}"),
    )
}

#[test]
fn text_tool_click_is_the_first_baseline() {
    if !has_font("ArialMT") {
        return;
    }
    // baseline 100; object top 100 - ceil(asc·s) - 2, left 97 (frame 99: "A" bears left)
    assert_eq!(text_tool_click(12.0), "97,87,20,19,100,91,114,103,true");
    assert_eq!(text_tool_click(30.0), "97,72,42,40,100,78,135,107,true");
}
