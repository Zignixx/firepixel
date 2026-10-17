//! F29 (run 14, A3-13): TW-110 look of the symbol properties table (measured in the reference
//! `inspector-group-symbol/04`: light alternating rows, 1 px grey grid at a 19 px pitch, grey-blue
//! box border, gradient header, table edge to edge in the panel).

use egui::epaint::Shape;
use egui::{Color32, Rect};
use egui_kittest::Harness;
use fp_ui::FirepixelApp;
use fp_ui::app_state::UiSettings;
use fp_ui::panels::PanelId;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn harness() -> H {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 500, "height": 300, "background": "#ffffff"})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [20, 20, 140, 90], "fillColor": "#ff0000", "strokeWidth": 0}))
        .expect("rect");
    s.execute("modify.symbol.convertToSymbol", json!({"name": "Sym"})).expect("symbol");
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    {
        let ws = &mut app.state.settings.workspace;
        ws.show(PanelId::SymbolProperties);
        ws.bottom_active = 1;
        ws.inspector_collapsed = false;
    }
    let mut h = Harness::builder()
        .with_size(egui::vec2(1800.0, 1130.0))
        .with_pixels_per_point(1.0)
        .with_max_steps(20)
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    h
}

fn rects(shape: &Shape, out: &mut Vec<(Rect, Color32)>) {
    match shape {
        Shape::Rect(r) => out.push((r.rect, r.fill)),
        Shape::Vec(v) => v.iter().for_each(|s| rects(s, out)),
        _ => {}
    }
}

#[test]
fn tw110_symbol_properties_table_look() {
    let h = harness();
    let mut all = Vec::new();
    for cs in &h.output().shapes {
        rects(&cs.shape, &mut all);
    }
    let border = Color32::from_rgb(0xb7, 0xba, 0xbc);
    let grid = Color32::from_gray(0xaa);
    // full-width horizontal grid lines (1 px) at a 19 px pitch
    let wide = |c: Color32| -> Vec<Rect> {
        all.iter().filter(|(r, f)| *f == c && r.height() == 1.0 && r.width() > 400.0).map(|x| x.0).collect()
    };
    let borders = wide(border);
    assert!(borders.len() >= 3, "box top, header line and box bottom: {borders:?}");
    let top = borders.iter().map(|r| r.top()).fold(f32::MAX, f32::min);
    let mut lines: Vec<f32> = wide(grid).iter().map(|r| r.top()).filter(|y| *y > top).collect();
    lines.sort_by(f32::total_cmp);
    assert!(lines.len() >= 3, "grid lines: {lines:?}");
    assert_eq!(lines[0] - top, 40.0, "first grid line 40 px below the box top (header 19 + row 19)");
    for w in lines.windows(2) {
        assert_eq!(w[1] - w[0], fp_ui::panels::misc::SYMPROPS_ROW_PITCH, "row pitch");
    }
    // alternating light rows and the measured header gradient ends
    for c in [0xeb, 0xe6, 0xe4, 0xd3, 0xe8] {
        assert!(all.iter().any(|(r, f)| *f == Color32::from_gray(c) && r.width() > 400.0), "fill #{c:02x}");
    }
}
