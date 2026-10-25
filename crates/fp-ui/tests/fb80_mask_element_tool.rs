//! FB-80 (Firepixel extension, TW-266): the Subselection flyout holds the new Mask Element
//! Selection ("Mask Element Selection (A, 1)"); picking it from the flyout makes it the active
//! tool and the slot shows it; over the canvas it shows its own cursor (Subselection arrow with a
//! mask badge); a click on a masked object selects the mask, the Subselection its content.

use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fp_engine::tools::{Cursor, ToolId};
use fp_model::ElementId;
use fp_ui::FirepixelApp;
use fp_ui::app_state::UiSettings;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn ex(s: &mut fp_engine::Session, cmd: &str, args: serde_json::Value) -> serde_json::Value {
    s.execute(cmd, args).expect(cmd).value
}

fn id_of(v: &serde_json::Value) -> u64 {
    v["id"].as_u64().expect("id")
}

/// A red path 100..200 masked by the path square 120..180: (harness, mask, content).
fn harness() -> (H, u64, u64) {
    fp_i18n::set_language("en");
    fp_engine::shell::disable();
    let mut s = fp_engine::Session::default();
    ex(
        &mut s,
        "file.new",
        json!({"width": 400, "height": 300, "background": "#ffffff"}),
    );
    let c = id_of(&ex(
        &mut s,
        "draw.path",
        json!({"contours": [{"closed": true, "nodes": [[100, 100], [200, 100], [200, 200], [100, 200]]}]}),
    ));
    ex(
        &mut s,
        "element.setFill",
        json!({"ids": [c], "color": "#ff0000"}),
    );
    let m = id_of(&ex(
        &mut s,
        "draw.path",
        json!({"contours": [{"closed": true, "nodes": [[120, 120], [180, 120], [180, 180], [120, 180]]}]}),
    ));
    ex(&mut s, "select.set", json!({"ids": [m, c]}));
    ex(&mut s, "modify.mask.groupAsMask", json!({}));
    ex(
        &mut s,
        "modify.mask.options",
        json!({"mode": "path", "showAttrs": false}),
    );
    ex(&mut s, "select.none", json!({}));
    ex(
        &mut s,
        "edit.keyboardShortcuts",
        json!({"set": "Firepixel"}),
    );
    s.set_tool(ToolId::Pointer).expect("pointer");
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .with_pixels_per_point(1.0)
        .with_max_steps(12)
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    (h, m, c)
}

fn at(h: &H, x: f32, y: f32) -> Pos2 {
    let st = &h.state().state;
    let d = st.session.active().expect("document");
    let area = st.canvases.get(&d.id).and_then(|c| c.area).expect("canvas");
    let (sx, sy) = d.view.doc_to_screen(fp_geom::Point::new(x, y));
    Pos2::new(area.min.x + sx as f32, area.min.y + sy as f32)
}

fn hover(h: &mut H, x: f32, y: f32) -> Cursor {
    let p = at(h, x, y);
    h.input_mut().events.push(Event::PointerMoved(p));
    h.step();
    h.run_steps(1);
    h.state().state.tool_cursor
}

fn canvas_click(h: &mut H, x: f32, y: f32) {
    let p = at(h, x, y);
    h.input_mut().events.push(Event::PointerMoved(p));
    h.step();
    for pressed in [true, false] {
        h.input_mut().events.push(Event::PointerButton {
            pos: p,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
        h.step();
    }
    h.run_steps(4);
}

fn sel(h: &H) -> Vec<ElementId> {
    h.state()
        .state
        .session
        .active()
        .map(|d| d.selection.elements.clone())
        .unwrap_or_default()
}

#[test]
fn flyout_entry_cursor_and_clicks() {
    let (mut h, m, c) = harness();
    // the Subselection slot has a flyout now; open it with a click on its corner triangle
    let slot = h.get_by_label("Subselection (A, 1)").rect();
    let corner = Pos2::new(slot.max.x - 3.0, slot.max.y - 3.0);
    h.input_mut().events.push(Event::PointerMoved(corner));
    h.step();
    for pressed in [true, false] {
        h.input_mut().events.push(Event::PointerButton {
            pos: corner,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
        h.step();
    }
    h.run_steps(3);
    assert!(h.state().state.flyout.is_some(), "flyout open");
    h.get_by_label("Mask Element Selection (A, 1)").click();
    h.run_steps(4);
    assert_eq!(
        h.state().state.session.tools.active,
        ToolId::MaskElementSelect
    );
    assert!(h.state().state.flyout.is_none(), "flyout closed");
    // the slot now shows the new tool
    assert!(h.query_by_label("Mask Element Selection (A, 1)").is_some());
    // own cursor over empty canvas and over objects
    assert_eq!(hover(&mut h, 40.0, 40.0), Cursor::MaskSelect);
    assert_eq!(hover(&mut h, 150.0, 150.0), Cursor::MaskSelect);
    canvas_click(&mut h, 150.0, 150.0);
    assert_eq!(sel(&h), vec![ElementId(m)], "the mask");
    h.state_mut()
        .state
        .session
        .set_tool(ToolId::Subselection)
        .expect("tool");
    h.run_steps(3);
    canvas_click(&mut h, 150.0, 150.0);
    assert_eq!(sel(&h), vec![ElementId(c)], "the content");
    assert_eq!(
        hover(&mut h, 40.0, 40.0),
        Cursor::Arrow,
        "Subselection keeps its plain arrow"
    );
}
