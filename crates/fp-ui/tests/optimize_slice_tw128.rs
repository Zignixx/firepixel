//! TW-128: choosing a format in the Optimize panel while slices are selected stores it on those
//! slices (their `exportOptions`); the panel then shows the slice's setting. Without a selected
//! slice the page's settings change as before.

use egui::accesskit::Role;
use egui_kittest::kittest::{By, Queryable};
use fp_model::{ElementId, ElementKind};
use fp_ui::app_state::{AppState, UiSettings};
use serde_json::json;

type H = egui_kittest::Harness<'static, AppState>;

fn state() -> (AppState, ElementId, ElementId) {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 200, "height": 120})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [10, 10, 60, 60], "fillColor": "#ff0000"})).expect("rect");
    s.execute("edit.insert.slice", json!({})).expect("slice 1");
    let a = s.doc().expect("doc").selection.elements[0];
    s.execute("draw.rectangle", json!({"rect": [100, 10, 160, 60], "fillColor": "#0000ff"})).expect("rect");
    s.execute("edit.insert.slice", json!({})).expect("slice 2");
    let b = s.doc().expect("doc").selection.elements[0];
    (AppState::new(s, UiSettings::default(), None), a, b)
}

fn harness(st: AppState) -> H {
    let mut h = egui_kittest::Harness::builder()
        .with_size(egui::vec2(260.0, 420.0))
        .with_pixels_per_point(1.0)
        .build_ui_state(
            |ui, st: &mut AppState| {
                fp_ui::panels::optimize::show(ui, st);
                st.process_actions(ui.ctx());
            },
            st,
        );
    h.run_steps(2);
    h
}

fn is_format(v: Option<String>) -> bool {
    v.is_some_and(|v| (0..fp_ui::panels::optimize::FORMATS.len()).any(|i| fp_ui::panels::optimize::format_name(i) == v))
}

/// Opens the format list (the second combo; the first holds the saved settings) and picks `to`.
fn choose_format(h: &mut H, to: &str) {
    h.query_all(By::new().role(Role::ComboBox))
        .nth(1)
        .filter(|n| is_format(n.value()))
        .expect("format list")
        .click();
    h.run_steps(3);
    h.get_by_label(to).click();
    h.run_steps(3);
}

fn select(h: &mut H, ids: &[ElementId]) {
    let ids: Vec<u64> = ids.iter().map(|i| i.0).collect();
    h.state_mut().session.execute("select.set", json!({"ids": ids})).expect("select");
    h.run_steps(2);
}

fn slice_format(h: &H, id: ElementId) -> Option<i32> {
    let d = h.state().session.active().expect("doc");
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, id).expect("slice");
    match &e.kind {
        ElementKind::WebObject(w) => w.slice.as_ref()?.export.as_ref().map(|x| x.format),
        _ => None,
    }
}

fn page_format(h: &H) -> Option<i32> {
    let d = h.state().session.active().expect("doc");
    d.doc.pages[d.page].root.export.as_ref().map(|e| e.format)
}

#[test]
fn tw128_panel_format_goes_to_selected_slice() {
    let (st, a, b) = state();
    let mut h = harness(st);
    let page_before = page_format(&h);
    select(&mut h, &[a]);
    choose_format(&mut h, "JPEG");
    assert_eq!(slice_format(&h, a), Some(1), "slice 1 got JPEG");
    assert_eq!(slice_format(&h, b), None);
    assert_eq!(page_format(&h), page_before, "document default unchanged");
    // the panel shows the slice's own setting
    let shown = h.query_all(By::new().role(Role::ComboBox)).nth(1).and_then(|n| n.value());
    assert_eq!(shown.as_deref(), Some("JPEG"));
    select(&mut h, &[b]);
    choose_format(&mut h, "PNG 8");
    assert_eq!(slice_format(&h, b), Some(2));
    assert_eq!(slice_format(&h, a), Some(1));
}

#[test]
fn tw128_panel_format_without_slice_changes_page() {
    let (st, a, b) = state();
    let mut h = harness(st);
    select(&mut h, &[]);
    choose_format(&mut h, "JPEG");
    assert_eq!(page_format(&h), Some(1));
    assert_eq!(slice_format(&h, a), None);
    assert_eq!(slice_format(&h, b), None);
}
