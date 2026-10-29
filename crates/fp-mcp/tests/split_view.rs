//! FB-97 / TW-272: the split document view (Window ▸ Duplicate Window) leaves `get_document`
//! unchanged: it describes the document with the focused pane's page and selection, and after the
//! other pane is closed the output is the same as before the split.

use fp_mcp::docjson::{DocJsonOptions, document_json};
use serde_json::json;

#[test]
fn get_document_reports_the_focused_pane_and_is_unchanged_by_the_split() {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 200, "height": 100}))
        .expect("doc");
    s.execute("draw.rectangle", json!({"rect": [10, 10, 60, 60]}))
        .expect("rect");
    s.execute("page.add", json!({})).expect("page");
    s.execute("page.setCurrent", json!({"index": 0}))
        .expect("page 0");
    let opts = DocJsonOptions::default();
    let before = document_json(s.active().expect("doc"), &opts);
    s.execute("window.duplicateWindow", json!({}))
        .expect("split");
    assert_eq!(document_json(s.active().expect("doc"), &opts), before);
    // the focused right pane goes to page 1: get_document follows it
    s.execute("page.setCurrent", json!({"index": 1}))
        .expect("page 1");
    let right = document_json(s.active().expect("doc"), &opts);
    assert_eq!(right["currentPage"], 1, "{right}");
    s.execute("window.focusPane", json!({"pane": 0}))
        .expect("focus");
    let left = document_json(s.active().expect("doc"), &opts);
    assert_eq!(left["currentPage"], 0, "{left}");
    // F137: closing the right pane (Close Duplicate Window alone closes the focused one)
    s.execute("window.closeDuplicate", json!({"pane": 1}))
        .expect("close");
    // the page changes counted as edits (History rows): only generation, dirty and the
    // History counts differ
    let strip = |mut v: serde_json::Value| {
        if let Some(m) = v.as_object_mut() {
            m.remove("generation");
            m.remove("dirty");
            m.remove("history");
        }
        v
    };
    assert_eq!(
        strip(document_json(s.active().expect("doc"), &opts)),
        strip(before)
    );
}
