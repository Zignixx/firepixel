//! F28 (run 13, A3-12): TW-104 bridge rect of the symbol properties in the bottom group (whole
//! group from the "Properties" tab, so the shot keeps the "Name | Value" column) and Q-50 bridge
//! protocol of pointer presses (`ui_state.pointer_log`: tool, hit, modifiers).

use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use fp_ui::FirepixelApp;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::panels::PanelId;
use serde_json::{Value, json};

type H = Harness<'static, FirepixelApp>;

fn harness(setup: impl FnOnce(&mut fp_engine::Session)) -> H {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 500, "height": 300, "background": "#ffffff"}))
        .expect("doc");
    setup(&mut s);
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1800.0, 1130.0))
        .with_pixels_per_point(1.0)
        .with_max_steps(20)
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    h
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn ui_state(h: &H) -> Value {
    fp_ui::bridge::ui_state(&h.ctx, st(h))
}

fn screen(h: &H, x: f32, y: f32) -> Pos2 {
    let s = st(h);
    let d = s.session.active().expect("doc");
    let area = s.canvases.get(&d.id).and_then(|c| c.area).expect("canvas");
    let (sx, sy) = d.view.doc_to_screen(fp_geom::Point::new(x, y));
    Pos2::new(area.min.x + sx as f32, area.min.y + sy as f32)
}

fn ev(h: &mut H, e: Event) {
    h.event(e);
    h.run_steps(1);
}

fn texts(shape: &egui::epaint::Shape, out: &mut Vec<(String, egui::Rect)>) {
    match shape {
        egui::epaint::Shape::Text(t) => out.push((
            t.galley.job.text.clone(),
            egui::Rect::from_min_size(t.pos, t.galley.size()),
        )),
        egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| texts(s, out)),
        _ => {}
    }
}

fn rect_texts(h: &H) -> Vec<(String, egui::Rect)> {
    let mut all = Vec::new();
    for cs in &h.output().shapes {
        texts(&cs.shape, &mut all);
    }
    all
}

/// TW-104: instance without parameters selected, symbol properties in front of the bottom group –
/// the bridge rect starts at the group (left of the "Properties" tab) and contains the column head.
#[test]
fn tw104_symbol_properties_rect_is_whole_bottom_group() {
    let mut h = harness(|s| {
        s.execute("draw.rectangle", json!({"rect": [20, 20, 140, 90], "fillColor": "#ff0000", "strokeWidth": 0}))
            .expect("rect");
        s.execute("modify.symbol.convertToSymbol", json!({"name": "Sym"})).expect("symbol");
    });
    {
        let ws = &mut h.state_mut().state.settings.workspace;
        ws.show(PanelId::SymbolProperties);
        ws.bottom_active = 1;
        ws.inspector_collapsed = false;
    }
    h.run_steps(4);
    assert_eq!(st(&h).session.active().expect("doc").selection.elements.len(), 1);
    let v = ui_state(&h);
    let r = v["panel_rects"][PanelId::SymbolProperties.bridge_name()]
        .as_array()
        .unwrap_or_else(|| panic!("symbol properties not reported: {}", v["panel_rects"]))
        .iter()
        .map(|x| x.as_f64().expect("number") as f32)
        .collect::<Vec<_>>();
    let rect = egui::Rect::from_min_max(egui::pos2(r[0], r[1]), egui::pos2(r[2], r[3]));
    let texts = rect_texts(&h);
    let find = |t: &str| {
        texts
            .iter()
            .filter(|(s, _)| s.trim() == t)
            .map(|x| x.1)
            .find(|b| rect.intersects(*b))
            .unwrap_or_else(|| panic!("{t:?} not in {rect:?}"))
    };
    let props_tab = find("Eigenschaften");
    let sym_tab = find("Symboleigenschaften");
    let value_head = find("Wert");
    assert!(rect.left() <= props_tab.left(), "rect {rect:?} starts left of the Properties tab {props_tab:?}");
    assert!(props_tab.left() < sym_tab.left());
    assert!(rect.contains(value_head.center()), "column head inside the rect");
    // the name column (35 px) lies inside the reported rect
    assert!(value_head.left() - rect.left() >= fp_ui::panels::misc::SYMPROPS_NAME_W);
}

/// Q-50: pointer press and release on an object with Alt are logged with tool, hit and modifiers.
#[test]
fn q50_pointer_log_records_tool_hit_and_modifiers() {
    let mut h = harness(|s| {
        s.execute("draw.rectangle", json!({"rect": [50, 50, 150, 120], "fillColor": "#3366cc", "strokeWidth": 0}))
            .expect("rect");
        s.execute("select.none", json!({})).expect("none");
    });
    h.state_mut().state.session.tools.active = fp_engine::tools::ToolId::Pointer;
    h.run_steps(2);
    let alt = Modifiers { alt: true, ..Modifiers::NONE };
    let a = screen(&h, 100.0, 85.0);
    let b = screen(&h, 160.0, 140.0);
    ev(&mut h, Event::PointerMoved(a));
    ev(&mut h, Event::ModifiersChanged(alt));
    ev(&mut h, Event::PointerButton { pos: a, button: PointerButton::Primary, pressed: true, modifiers: alt });
    for i in 1..=6 {
        ev(&mut h, Event::PointerMoved(a + (b - a) * (i as f32 / 6.0)));
    }
    ev(&mut h, Event::PointerButton { pos: b, button: PointerButton::Primary, pressed: false, modifiers: alt });
    ev(&mut h, Event::ModifiersChanged(Modifiers::NONE));
    // a press on empty canvas without modifiers
    let e = screen(&h, 400.0, 250.0);
    ev(&mut h, Event::PointerMoved(e));
    ev(&mut h, Event::PointerButton { pos: e, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    ev(&mut h, Event::PointerButton { pos: e, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
    let v = ui_state(&h);
    let log = v["pointer_log"].as_array().expect("pointer_log").clone();
    assert_eq!(log.len(), 4, "{log:?}");
    assert_eq!(log[0]["event"], "down");
    assert_eq!(log[0]["tool"], log[1]["tool"]);
    assert!(log[0]["tool"].is_string(), "{log:?}");
    assert_eq!(log[0]["hit"], "object", "{log:?}");
    assert_eq!(log[0]["mods"]["alt"], true);
    assert_eq!(log[1]["event"], "up");
    assert_eq!(log[1]["hit"], "object", "release reports what the press hit");
    assert_eq!(log[1]["mods"]["alt"], true);
    assert_eq!(log[2]["hit"], "empty", "{log:?}");
    assert_eq!(log[2]["mods"]["alt"], false);
    let pos = log[0]["pos"].as_array().expect("pos");
    assert!((pos[0].as_f64().unwrap() - 100.0).abs() < 1.0);
}
