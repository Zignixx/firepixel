//! R-32 (parity run 37, `inspector-text/25_indent_point_text_result`): a new text from
//! `addNewText(rect, true)` and its conversion to point text (`autoExpand = true`).
//!
//! 1. `addNewText(r, true)` reads the paragraph defaults (`Text_ParagraphIndent`, spacing before /
//!    after) like the text tool (re 50 section 2.3, re 56 section 2.1); the Properties panel writes
//!    them also for a text selected with the pointer (re 56 section 2.2), and only activating the
//!    text tool puts them back to 0 (re 56 section 2.4). The probe's "reference started with 0"
//!    came from a reference run whose indent field never applied (its TX2 stayed unindented).
//! 2. `autoExpand = true` on a fixed frame keeps the frame top; the anchor becomes the first
//!    baseline (re 09 section 8a file data, run 37 values).

use fp_engine::Session;
use fp_engine::tools::ToolId;
use fp_model::ElementKind;
use fp_script::{ScriptOptions, run_source};
use serde_json::json;

/// Session on the test profile (the Properties panel writes the `Text_*` preferences).
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

const NEW: &str = "var dom = fw.createFirepixelDocument({x: 800, y: 600}, {pixelsPerUnit: 72, units: 'inch'}, '#ffffff'); dom = fw.getDocumentDOM();\n";

const ADD_AND_READ: &str = "var dom = fw.getDocumentDOM(); dom.selectNone(); dom.addNewText({left: 450, top: 380, right: 690, bottom: 420}, true);\n\
     var a = fw.selection[0].textRuns.initialAttrs;\n\
     [a.paragraphIndent, a.paragraphSpacingBefore, a.paragraphSpacingAfter, a.size].join(',')";

#[test]
fn add_new_text_takes_the_paragraph_defaults_until_the_text_tool_is_activated() {
    let mut s = session();
    result(&mut s, NEW);
    // a text selected with the pointer; the Properties panel sets indent 20 (inspector-text/13)
    s.execute(
        "draw.text",
        json!({"text": "Zeile eins", "rect": [50, 150, 250, 198], "size": 14}),
    )
    .expect("text");
    for p in [
        json!({"indent": 20}),
        json!({"spaceBefore": 10}),
        json!({"spaceAfter": 10}),
    ] {
        s.execute("text.applyAttributes", p).expect("panel");
    }
    assert_eq!(s.prefs.get_real("Text_ParagraphIndent", -1.0), 20.0);
    // addNewText(r, true) reads the defaults like the text tool (re 50 section 2.3)
    assert_eq!(result(&mut s, ADD_AND_READ), "20,10,10,12pt");
    // activating the text tool resets them (re 56 section 2.4); the size stays
    s.set_tool(ToolId::Pointer).expect("pointer");
    s.set_tool(ToolId::Text).expect("text tool");
    assert_eq!(result(&mut s, ADD_AND_READ), "0,0,0,12pt");
}

fn text_geometry(s: &Session) -> Vec<(String, [f32; 4], [f32; 4])> {
    let d = s.active().expect("doc");
    let mut v = Vec::new();
    d.doc.walk_elements(|_, _, e| {
        if let ElementKind::Text(t) = &e.kind {
            let a = t.anchor_rect.unwrap_or(t.bounds);
            let b = t.bounds;
            v.push((
                t.runs.iter().map(|r| r.text.as_str()).collect(),
                [a.left, a.top, a.right, a.bottom],
                [b.left, b.top, b.right, b.bottom],
            ));
        }
    });
    v
}

/// Reference file data (re 09 section 8a; `addNewText` frame at `top`, `setTextRuns`, then
/// `autoExpand = true`): `TOPf` = top, `TOAf` = first baseline, `RIAf = BOAf = 0`.
#[test]
fn auto_expand_keeps_the_frame_top_and_anchors_the_first_baseline() {
    for (face, size, top, txt, toa, bot) in [
        ("ArialMT", 30, 50.0, "Hamburg", 76.0, 86.0),
        ("ArialMT", 13, 430.0, "Tennessee", 442.0, 447.0),
        ("MyriadPro-Regular", 20, 40.0, "Punkttext", 57.0, 62.0),
    ] {
        if !has_font(face) {
            continue;
        }
        let mut s = session();
        result(
            &mut s,
            &format!(
                "{NEW} dom.addNewText({{left: 20, top: {top}, right: 720, bottom: {}}}, false);\n\
                 dom.setTextRuns({{initialAttrs: {{face: '{face}', size: '{size}pt', leading: 1, leadingMode: 'percentage', alignment: 'left'}}, textRuns: [{{changedAttrs: {{}}, characters: '{txt}'}}]}});\n\
                 fw.selection[0].autoExpand = true; 1",
                top + 100.0
            ),
        );
        let g = text_geometry(&s);
        let (_, a, b) = g.iter().find(|(t, _, _)| t == txt).expect("text");
        assert_eq!(*a, [20.0, toa, 0.0, 0.0], "{face} {size} anchor");
        assert_eq!((b[1], b[3]), (top, bot), "{face} {size} frame top/bottom");
    }
}

/// Run 37 `inspector-text/25`: TXP, ArialMT 15 pt, three paragraphs, `addNewText` at
/// 450,380 then `autoExpand = true`. Reference: top 378, height 58, `pixelRect` top 382, bottom
/// 430 (Firepixel before: top 365, `pixelRect` 369-417, the rect top taken as the baseline).
#[test]
fn run37_point_text_top_matches_the_reference() {
    if !has_font("ArialMT") {
        return;
    }
    let mut s = session();
    let v = result(
        &mut s,
        &format!(
            "{NEW} var d = fw.getDocumentDOM(); d.selectNone(); d.addNewText({{left: 450, top: 380, right: 690, bottom: 420}}, true);\n\
             var o = fw.selection[0]; o.textChars = 'Absatz eins\\rAbsatz zwei\\rAbsatz drei'; var R = o.textRuns;\n\
             R.initialAttrs.face = 'ArialMT'; R.initialAttrs.size = '15pt'; R.initialAttrs.fillColor = '#000000'; R.initialAttrs.alignment = 'left';\n\
             R.initialAttrs.bold = false; R.initialAttrs.italic = false; R.initialAttrs.underline = false; R.initialAttrs.paragraphIndent = 0;\n\
             R.textRuns[0].changedAttrs = {{}}; o.textRuns = R; o = fw.selection[0]; o.autoExpand = true;\n\
             var t = fw.selection[0], p = t.pixelRect; [t.left, t.top, t.width, t.height, p.left, p.top, p.right, p.bottom, t.autoExpand].join(',')"
        ),
    );
    let n: Vec<&str> = v.split(',').collect();
    assert_eq!(n.len(), 9, "{v}");
    let num = |i: usize| n[i].parse::<f64>().unwrap_or(f64::NAN);
    assert_eq!((num(1), num(3)), (378.0, 58.0), "top/height: {v}");
    assert_eq!(
        (num(5), num(7)),
        (382.0, 430.0),
        "pixelRect top/bottom: {v}"
    );
    assert!((num(0) - 448.0).abs() <= 1.0, "left: {v}");
    assert_eq!(n[8], "true");
}
