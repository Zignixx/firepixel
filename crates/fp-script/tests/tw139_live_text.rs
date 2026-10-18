//! TW-139: while the text tool edits, script read-back (element list, `textChars`, `pixelRect`)
//! shows the editor's live state, as the original shows the new object from the first click and
//! the typed text at once. The committed model and the history stay untouched until the edit ends.

use fp_engine::tools::{EditOp, ToolId, ToolInput};
use fp_engine::{Session, ToolEvent, ToolKey};
use fp_model::ElementKind;
use fp_script::{ScriptOptions, run_source};
use serde_json::json;

/// Text objects of the first frame as "textChars|pixelRect has ink", joined by ",".
const TEXTS: &str = "var d = fw.getDocumentDOM(), a = [], L = d.frames[0].layers;\n\
for (var i = 0; i < L.length; i++) { var E = L[i].elements;\n\
  for (var k = 0; k < E.length; k++) if (E[k].textChars !== undefined) {\n\
    var r = E[k].pixelRect; a.push(E[k].textChars + '|' + (r.right > r.left && r.bottom > r.top)); } }\n\
a.join(',')";

fn session() -> Session {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200}))
        .expect("doc");
    s
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

fn click(s: &mut Session, x: f32, y: f32) {
    s.tool_event(&ToolEvent::down(x, y)).expect("down");
    s.tool_event(&ToolEvent::up(x, y)).expect("up");
}

fn eval(s: &mut Session, src: &str) -> String {
    let o = run_source(s, src, &ScriptOptions::default());
    assert!(o.ok, "{:?}", o.error);
    o.result.unwrap_or_default()
}

/// Plain texts in the committed model of the active document.
fn model_texts(s: &Session) -> Vec<String> {
    let d = s.active().expect("doc");
    let mut out = Vec::new();
    d.doc.walk_elements(|_, _, e| {
        if let ElementKind::Text(t) = &e.kind {
            out.push(t.plain_text());
        }
    });
    out
}

#[test]
fn new_text_reads_back_live_while_editing() {
    let mut s = session();
    s.set_tool(ToolId::Text).expect("tool");
    click(&mut s, 40.0, 60.0);
    send(&mut s, ToolInput::Text("Q".into()));
    assert!(s.tool_is_editing_text());
    assert_eq!(eval(&mut s, TEXTS), "Q|true");
    // the editor stays open, the model and history are unchanged by the read
    assert!(s.tool_is_editing_text());
    assert!(model_texts(&s).is_empty(), "{:?}", model_texts(&s));
    send(&mut s, ToolInput::Text("R".into()));
    assert_eq!(eval(&mut s, TEXTS), "QR|true");
    s.tool_event(&ToolEvent::key(ToolKey::Escape))
        .expect("commit");
    assert!(!s.tool_is_editing_text());
    assert_eq!(model_texts(&s), ["QR"]);
    assert_eq!(eval(&mut s, TEXTS), "QR|true");
}

#[test]
fn edited_text_reads_back_live_and_a_command_still_works() {
    let mut s = session();
    s.execute(
        "draw.text",
        json!({"text": "Alt", "anchor": [40, 60], "size": 24}),
    )
    .expect("text");
    s.set_tool(ToolId::Text).expect("tool");
    click(&mut s, 45.0, 52.0);
    assert!(s.tool_is_editing_text());
    send(&mut s, ToolInput::Edit(EditOp::SelectAll));
    send(&mut s, ToolInput::Text("Neu".into()));
    assert_eq!(eval(&mut s, TEXTS), "Neu|true");
    assert_eq!(model_texts(&s), ["Alt"]);
    // a changing command inside the script runs on the committed model, the read after it shows
    // the live text again
    let r = eval(
        &mut s,
        &format!(
            "fw.getDocumentDOM().addNewRectangle({{left: 200, top: 120, right: 260, bottom: 180}}, 0);\n{TEXTS}"
        ),
    );
    assert_eq!(r, "Neu|true");
    assert!(s.tool_is_editing_text());
    s.tool_event(&ToolEvent::key(ToolKey::Escape))
        .expect("commit");
    assert_eq!(model_texts(&s), ["Neu"]);
}

/// TW-143: typing into an auto-width text grows its read-back bounds at once (the reference reads
/// 107 → 120 while typing): `left + width` and `pixelRect.right` both grow before Escape, and the
/// live values equal the committed ones afterwards.
#[test]
fn edited_text_bounds_grow_live_while_typing() {
    const RIGHT: &str = "var L = fw.getDocumentDOM().frames[0].layers, e = null;
for (var i = 0; i < L.length; i++) for (var k = 0; k < L[i].elements.length; k++)
  if (L[i].elements[k].textChars !== undefined) e = L[i].elements[k];
(e.left + e.width) + '|' + e.pixelRect.right";
    let right = |s: &mut Session| -> (f64, f64) {
        let v = eval(s, RIGHT);
        let n: Vec<f64> = v.split('|').filter_map(|x| x.parse().ok()).collect();
        assert_eq!(n.len(), 2, "bounds: {v}");
        (n[0], n[1])
    };
    let mut s = session();
    s.execute(
        "draw.text",
        json!({"text": "Alt", "anchor": [40, 60], "size": 24}),
    )
    .expect("text");
    let before = right(&mut s);
    s.set_tool(ToolId::Text).expect("tool");
    click(&mut s, 45.0, 52.0);
    assert!(s.tool_is_editing_text());
    s.tool_event(&ToolEvent::key(ToolKey::End)).expect("end");
    send(&mut s, ToolInput::Text("Vrrrr".into()));
    let live = right(&mut s);
    assert!(
        live.0 > before.0 + 10.0,
        "live left + width {live:?} vs. committed {before:?}"
    );
    assert!(
        live.1 > before.1 + 10.0,
        "live pixelRect.right {live:?} vs. committed {before:?}"
    );
    assert!(s.tool_is_editing_text());
    s.tool_event(&ToolEvent::key(ToolKey::Escape))
        .expect("commit");
    assert_eq!(model_texts(&s), ["AltVrrrr"]);
    assert_eq!(right(&mut s), live, "live bounds = committed bounds");
}
