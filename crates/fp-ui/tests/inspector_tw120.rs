//! TW-120: property inspector with several objects selected – fill, stroke and effects come from
//! the topmost selected object in stacking order; mixed opacity is a blank field; header
//! "n objects"; W/H/X/Y = union of path bounds without stroke.

use egui_kittest::kittest::{By, Queryable};
use egui_kittest::Harness;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

const W: f32 = 1800.0;
const HH: f32 = 1130.0;

fn harness(setup: impl FnOnce(&mut fp_engine::Session)) -> H {
    fp_i18n::set_language("de");
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 500, "height": 300, "background": "#ffffff"})).expect("doc");
    setup(&mut s);
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(W, HH)).with_pixels_per_point(1.0).with_max_steps(20).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    h
}

fn id_of(v: fp_engine::CommandOutput) -> u64 {
    v.value.get("id").and_then(|x| x.as_u64()).expect("id")
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn inputs(h: &H) -> Vec<String> {
    h.get_all(By::new().role(egui::accesskit::Role::TextInput)).filter(|n| n.rect().top() > HH - 160.0).map(|n| n.value().unwrap_or_default()).collect()
}

/// Rectangle R (#3366CC) below a black text TX, R selected first: the fill shown is black.
#[test]
fn rect_and_text_shows_text_fill() {
    let mut ids = (0, 0);
    let mut h = harness(|s| {
        ids.0 = id_of(s.execute("draw.rectangle", json!({"rect": [40, 40, 200, 120], "fillColor": "#3366cc", "strokeWidth": 0})).expect("R"));
        ids.1 = id_of(s.execute("draw.text", json!({"text": "Hallo", "anchor": [60, 60], "size": 24})).expect("TX"));
        s.execute("element.setFill", json!({"ids": [ids.1], "color": "#000000"})).ok();
        s.execute("select.set", json!({"ids": [ids.0, ids.1]})).expect("select");
    });
    h.run_steps(3);
    let tx = fp_engine::docops::path_attrs_of(&st(&h).session.active().and_then(|d| fp_engine::docops::elements_on_page(&d.doc, d.page, &[fp_model::ElementId(ids.1)]).pop()).expect("TX")).expect("TX attrs");
    let a = fp_ui::inspector::obj_quads::shown_attrs(st(&h)).expect("attrs");
    assert_eq!(a.fill_color.0 & 0x00ff_ffff, tx.fill_color.0 & 0x00ff_ffff, "fill of the topmost object (text)");
    assert_ne!(a.fill_color.0 & 0x00ff_ffff, 0x3366cc, "not the first selected rectangle");
}

/// E1 (1 px red stroke) above R1 (no stroke), selection order R1, E1 or E1, R1: the stroke shown is
/// E1's red 1 px; W/H/X/Y = union of the path bounds without the stroke.
#[test]
fn ellipse_over_rect_shows_ellipse_stroke() {
    for order in [false, true] {
        let mut ids = (0, 0);
        let mut h = harness(|s| {
            ids.0 = id_of(s.execute("draw.rectangle", json!({"rect": [50, 50, 150, 120], "fillColor": "#808080", "strokeWidth": 0})).expect("R1"));
            ids.1 = id_of(s.execute("draw.ellipse", json!({"rect": [50, 50, 150, 240], "fillColor": "#808080", "strokeWidth": 1, "strokeColor": "#c04040"})).expect("E1"));
            let sel = if order { json!([ids.1, ids.0]) } else { json!([ids.0, ids.1]) };
            s.execute("select.set", json!({ "ids": sel })).expect("select");
        });
        h.run_steps(3);
        let a = fp_ui::inspector::obj_quads::shown_attrs(st(&h)).expect("attrs");
        let brush = a.brush.as_ref().expect("stroke of E1");
        assert_eq!(brush.diameter, 1, "{brush:?}");
        assert_eq!(a.stroke_color.0 & 0x00ff_ffff, 0xc04040);
        let v = inputs(&h);
        for want in ["100", "190", "50"] {
            assert!(v.iter().any(|x| x == want), "W/H/X/Y {want} in {v:?}");
        }
    }
}

/// Two rectangles with different opacity: the opacity field is blank.
#[test]
fn mixed_opacity_is_blank() {
    let mut h = harness(|s| {
        let r1 = id_of(s.execute("draw.rectangle", json!({"rect": [50, 50, 150, 120], "fillColor": "#808080", "strokeWidth": 0})).expect("R1"));
        let r2 = id_of(s.execute("draw.rectangle", json!({"rect": [150, 50, 300, 120], "fillColor": "#808080", "strokeWidth": 0})).expect("R2"));
        s.execute("select.set", json!({"ids": [r2]})).expect("select");
        s.execute("element.setOpacity", json!({"opacity": 50})).expect("opacity");
        s.execute("select.set", json!({"ids": [r1, r2]})).expect("select");
    });
    h.run_steps(3);
    let v = inputs(&h);
    // "50" only as X and Y, no opacity value
    assert!(v.iter().filter(|x| *x == "50").count() == 2 && v.iter().all(|x| x != "100"), "mixed opacity = blank: {v:?}");
    assert!(v.iter().any(|x| x == "250") && v.iter().any(|x| x == "70"), "W 250 H 70: {v:?}");
}
