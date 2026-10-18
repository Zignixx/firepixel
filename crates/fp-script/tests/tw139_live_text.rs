//! TW-139: while the text tool edits, script read-back (element list, `textChars`, `pixelRect`)
//! shows the editor's live state, as the original shows the new object from the first click and
//! the typed text at once. The committed model and the history stay untouched until the edit ends.

use fp_engine::tools::{EditOp, ToolId, ToolInput};
use fp_engine::{Session, ToolEvent, ToolKey};
use fp_model::ElementKind;
use fp_model::element::BitmapNode;
use fp_script::{ScriptOptions, run_source};
use serde_json::json;
use std::sync::Arc;

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

/// TW-143 (rest): the parity runner reads the bounds through
/// `dom.layers[l].frames[f].elements[i]`; while typing into line 2 of a two-line text loaded from a
/// file (which carries the file's stored render cache) that path reports the same live
/// `pixelRect.right` and `width` as `fw.selection[0]`, and both grow with the typed text before
/// Escape. Run 21 (`xs-shortcuts/15_in_text_tx2`): the live element kept the committed text's
/// render cache, so `pixelRect` read the old ink (107) until Escape (120).
#[test]
fn element_list_reads_the_live_bounds_like_the_selection() {
    const BOTH: &str = "var d = fw.getDocumentDOM(), e = null;
for (var l = 0; l < d.layers.length; l++) { var E = d.layers[l].frames[d.currentFrameNum].elements;
  for (var i = 0; i < E.length; i++) if (E[i].textChars != null) e = E[i]; }
var s = fw.selection[0];
[e.pixelRect.right, e.width, e.left, s.pixelRect.right, s.width, s.left, e.textChars.length].join('|')";
    let read = |s: &mut Session| -> Vec<f64> {
        let v = eval(s, BOTH);
        let n: Vec<f64> = v.split('|').filter_map(|x| x.parse().ok()).collect();
        assert_eq!(n.len(), 7, "bounds: {v}");
        n
    };
    let mut s = session();
    s.execute(
        "draw.text",
        json!({"text": "Abcdef\rAb", "anchor": [40, 60], "size": 24}),
    )
    .expect("text");
    // like a text loaded from a file: the element carries a stored render cache of its committed
    // look (here a flattened clone of it)
    let text_id = s.active().expect("doc").selection.elements[0];
    s.execute("edit.clone", json!({})).expect("clone");
    s.execute("modify.flattenSelection", json!({}))
        .expect("flatten");
    let d = s.active().expect("doc");
    let bmp = d.selection.elements[0];
    let path = d.doc.find_element(bmp).expect("bitmap");
    let ElementKind::Bitmap(b) = &d.doc.element_at(&path).expect("bitmap").kind else {
        panic!("flattened clone is a bitmap");
    };
    let cache = Arc::new(BitmapNode {
        common: Default::default(),
        bitmap: b.clone(),
        extra: Default::default(),
    });
    s.execute("edit.clear", json!({})).expect("clear");
    let d = s.active_mut().expect("doc");
    let page = d.page;
    fp_engine::docops::edit_by_id(Arc::make_mut(&mut d.doc), page, text_id, |e| {
        e.cache = Some(cache)
    })
    .expect("text");
    eval(
        &mut s,
        "var E = fw.getDocumentDOM().layers[0].frames[0].elements;
for (var i = 0; i < E.length; i++) if (E[i].textChars != null) fw.selection = [E[i]];",
    );
    let before = read(&mut s);
    assert_eq!(before[..3], before[3..6], "committed: {before:?}");
    s.set_tool(ToolId::Text).expect("tool");
    // a click on line 2 (below the first line's baseline)
    click(&mut s, 45.0, 80.0);
    assert!(s.tool_is_editing_text());
    s.tool_event(&ToolEvent::key(ToolKey::End)).expect("end");
    send(&mut s, ToolInput::Text("VrVrVrVr".into()));
    let mut prev = read(&mut s);
    assert_eq!(prev[..3], prev[3..6], "element list = selection: {prev:?}");
    assert!(prev[0] > before[0] + 10.0, "live {prev:?} vs. {before:?}");
    assert!(prev[1] > before[1] + 10.0, "live {prev:?} vs. {before:?}");
    for _ in 0..2 {
        send(&mut s, ToolInput::Text("WW".into()));
        let now = read(&mut s);
        assert_eq!(now[..3], now[3..6], "element list = selection: {now:?}");
        assert!(
            now[0] > prev[0] && now[1] > prev[1],
            "grows: {now:?} vs. {prev:?}"
        );
        prev = now;
    }
    assert!(s.tool_is_editing_text());
    s.tool_event(&ToolEvent::key(ToolKey::Escape))
        .expect("commit");
    assert_eq!(read(&mut s), prev, "live bounds = committed bounds");
}
