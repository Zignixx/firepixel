//! TW-128: the Optimize panel stores the chosen export settings on the selected slices (their
//! `exportOptions`); without a selected slice it keeps changing the page's settings.

use fp_engine::Session;
use fp_model::{ElementId, ElementKind, ExportSettings};
use serde_json::json;

/// Document with two slices; returns their ids.
fn two_slices() -> (Session, ElementId, ElementId) {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 200, "height": 120})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [10, 10, 60, 60], "fillColor": "#ff0000"})).expect("rect");
    s.execute("edit.insert.slice", json!({})).expect("slice 1");
    let a = s.doc().expect("doc").selection.elements[0];
    s.execute("draw.rectangle", json!({"rect": [100, 10, 160, 60], "fillColor": "#0000ff"})).expect("rect");
    s.execute("edit.insert.slice", json!({})).expect("slice 2");
    let b = s.doc().expect("doc").selection.elements[0];
    assert_ne!(a, b);
    (s, a, b)
}

fn slice_export(s: &Session, id: ElementId) -> Option<ExportSettings> {
    let d = s.doc().expect("doc");
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, id).expect("slice");
    match &e.kind {
        ElementKind::WebObject(w) => w.slice.as_ref().expect("slice props").export.as_deref().cloned(),
        _ => panic!("not a slice"),
    }
}

fn page_format(s: &Session) -> Option<i32> {
    let d = s.doc().expect("doc");
    d.doc.pages[d.page].root.export.as_ref().map(|e| e.format)
}

fn set(s: &mut Session, format: i32) {
    let es = ExportSettings { format, ..Default::default() };
    s.execute("panel.optimize.settings", json!({"settings": serde_json::to_value(es).expect("json")})).expect("settings");
}

#[test]
fn tw128_format_goes_to_each_selected_slice() {
    let (mut s, a, b) = two_slices();
    let page_before = page_format(&s);
    s.execute("select.set", json!({"ids": [a.0]})).expect("select a");
    set(&mut s, 1); // JPEG
    s.execute("select.set", json!({"ids": [b.0]})).expect("select b");
    set(&mut s, 0); // GIF
    assert_eq!(slice_export(&s, a).map(|e| e.format), Some(1));
    assert_eq!(slice_export(&s, b).map(|e| e.format), Some(0));
    assert_eq!(page_format(&s), page_before, "document default unchanged");
    assert_eq!(fp_engine::commands::panel::optimize_target_export(&s).map(|e| e.format), Some(0));
}

#[test]
fn tw128_all_selected_slices_get_the_setting() {
    let (mut s, a, b) = two_slices();
    s.execute("select.set", json!({"ids": [a.0, b.0]})).expect("select both");
    set(&mut s, 1);
    assert_eq!(slice_export(&s, a).map(|e| e.format), Some(1));
    assert_eq!(slice_export(&s, b).map(|e| e.format), Some(1));
}

#[test]
fn tw128_without_slice_the_page_default_changes() {
    let (mut s, a, b) = two_slices();
    s.execute("select.set", json!({"ids": []})).expect("deselect");
    set(&mut s, 1);
    assert_eq!(page_format(&s), Some(1));
    assert!(slice_export(&s, a).is_none());
    assert!(slice_export(&s, b).is_none());
}
