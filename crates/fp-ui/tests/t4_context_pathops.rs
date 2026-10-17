//! T-4 (re\29b): right click into empty canvas clears the selection before the canvas menu (§1c);
//! tool inspector path buttons only with a real path on the page, masks do not count (§2).

use fp_engine::ToolId;
use fp_ui::app_state::{AppState, UiSettings};
use serde_json::json;

fn state() -> AppState {
    fp_engine::shell::disable();
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 500, "height": 400, "background": "#ffffff"})).expect("doc");
    AppState::new(s, UiSettings::default(), None)
}

fn rect(st: &mut AppState, r: [f32; 4]) -> u64 {
    st.session.execute("draw.rectangle", json!({"rect": r})).expect("rect").value["id"].as_u64().unwrap_or(0)
}

#[test]
fn right_click_into_empty_clears_selection() {
    let mut st = state();
    let id = rect(&mut st, [300.0, 50.0, 400.0, 150.0]);
    st.session.execute("select.set", json!({"ids": [id], "mode": "replace"})).expect("select");
    st.session.tools.active = ToolId::Pointer;
    let m = fp_ui::canvas::context::open_at(&mut st, fp_geom::Point::new(250.0, 360.0));
    assert!(st.session.active().expect("d").selection.elements.is_empty(), "Auswahl aufgehoben");
    assert_eq!(m.id, "6914", "Leinwandmenü");
    // right click on the object still selects it
    let m = fp_ui::canvas::context::open_at(&mut st, fp_geom::Point::new(350.0, 100.0));
    assert_eq!(st.session.active().expect("d").selection.elements.len(), 1);
    assert_ne!(m.id, "6914");
}

#[test]
fn path_buttons_need_a_real_path_masks_do_not_count() {
    let mut st = state();
    assert!(!fp_ui::inspector::obj_quads::page_has_paths(&st), "leere Seite");
    let img = fp_raster::RgbaImage::filled(fp_geom::IRect::from_xywh(300, 50, 150, 150), fp_color::Rgba8::new(200, 30, 30, 255)).ok();
    st.session.clipboard.clear();
    st.session.clipboard.pixels = img;
    let b = st.session.execute("edit.paste", json!({})).expect("paste").value["ids"][0].as_u64().unwrap_or(0);
    let m = rect(&mut st, [315.0, 65.0, 435.0, 185.0]);
    st.session.execute("select.set", json!({"ids": [b, m], "mode": "replace"})).expect("select");
    st.session.execute("modify.mask.groupAsMask", json!({})).expect("mask");
    st.session.execute("select.none", json!({})).expect("none");
    assert!(!fp_ui::inspector::obj_quads::page_has_paths(&st), "Bild mit Vektormaske zählt nicht");
    rect(&mut st, [10.0, 10.0, 50.0, 50.0]);
    assert!(fp_ui::inspector::obj_quads::page_has_paths(&st), "echter Pfad");
}
