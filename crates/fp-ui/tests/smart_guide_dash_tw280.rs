//! TW-280 (RE spec 77 section 4): the smart guide of a Pointer move is drawn in `SmartGuideColor`
//! (#ff4aff), 1 px, dashed 3 px on / 3 px off from the upper end, widened beyond the reference
//! values by max(40, trunc(0.2 * span)) screen pixels at both ends. Pure run geometry and a pixel
//! check in the rendered window (wgpu).

use egui::{Event, Modifiers, PointerButton, Pos2, pos2};
use egui_kittest::Harness;
use fp_engine::Session;
use fp_engine::tools::{Overlay, ToolId};
use fp_ui::FirepixelApp;
use fp_ui::app_state::UiSettings;
use fp_ui::canvas::smart_guide_dashes;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

/// Runs of a vertical line from y 50 to 160 at x 300.5 (span 110 -> 40 px widening): rows
/// 10..=12, 16..=18, ... up to row 200, 1 px wide in column 300.
#[test]
fn dash_runs_of_a_vertical_line() {
    let runs = smart_guide_dashes(pos2(300.5, 50.0), pos2(300.5, 160.0), 1.0);
    assert_eq!(runs[0].min, pos2(300.0, 10.0));
    assert_eq!(runs[0].max, pos2(301.0, 13.0));
    assert_eq!(runs[1].min.y, 16.0);
    for w in runs.windows(2) {
        assert_eq!(w[1].min.y - w[0].min.y, 6.0, "3 on / 3 off");
    }
    let last = runs.last().expect("runs");
    assert!(last.min.y <= 200.0 && last.max.y <= 201.0, "{last:?}");
    assert!(last.min.y > 194.0, "{last:?}");
    // a long span widens by 0.2 * span: 0..500 -> 100 px
    let runs = smart_guide_dashes(pos2(0.0, 7.0), pos2(500.0, 7.0), 1.0);
    assert_eq!(runs[0].min, pos2(-100.0, 7.0));
    assert_eq!(runs[0].max, pos2(-97.0, 8.0));
    // trunc: span 213 -> 42.6 -> 42
    let runs = smart_guide_dashes(pos2(5.0, 0.0), pos2(5.0, 213.0), 1.0);
    assert_eq!(runs[0].min.y, -42.0);
}

fn harness(s: Session) -> H {
    let ctx = egui::Context::default();
    let mut a = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    a.state.os_clipboard = false;
    a.state.sync_render = true;
    let mut h = Harness::builder()
        .renderer(crate::gpu_common::renderer())
        .with_size(egui::vec2(1400.0, 900.0))
        .with_pixels_per_point(1.0)
        .with_max_steps(8)
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), a);
    h.run_steps(4);
    h
}

/// Screen point of a document point.
fn at(h: &H, x: f32, y: f32) -> Pos2 {
    let s = &h.state().state;
    let d = s.session.active().expect("document");
    let pr = s
        .canvases
        .get(&d.id)
        .and_then(|c| c.page_screen)
        .expect("page position");
    let r = fp_render::Renderer::page_rect(&d.doc, d.page).expect("page");
    let (w, ht) = ((r.right - r.left) as f32, (r.bottom - r.top) as f32);
    pos2(
        pr.left() + (x - r.left as f32) / w * pr.width(),
        pr.top() + (y - r.top as f32) / ht * pr.height(),
    )
}

fn ev(h: &mut H, e: Event) {
    h.input_mut().events.push(e);
    h.step();
}

/// `mkptr.jsf` at 100 %: R 100/100-200/160 dragged by +97 so its right edge snaps to the left
/// edge 300 of Q (300/50-350/120); with the mouse held the guide at x 300 runs dashed from y 10
/// (Q top 50 - 40) to y 200 (R bottom 160 + 40).
#[test]
fn pointer_guide_is_dashed_and_widened() {
    let mut s = Session::default();
    s.execute(
        "file.new",
        json!({"width": 400, "height": 300, "background": "#ffffff"}),
    )
    .expect("document");
    s.execute("draw.rectangle", json!({"rect": [300, 50, 350, 120]}))
        .expect("Q");
    let r = s
        .execute("draw.rectangle", json!({"rect": [100, 100, 200, 160]}))
        .expect("R");
    let rid = r.value["id"].as_u64().expect("id");
    s.execute("select.set", json!({"ids": [rid]})).expect("select");
    s.execute("view.setZoom", json!({"zoom": 1.0})).expect("zoom");
    if let Some(d) = s.active_mut() {
        d.view.snap_to_smart_guides = true;
        d.view.show_smart_guides = true;
    }
    let _ = s.set_tool(ToolId::Pointer);
    let mut h = harness(s);
    h.run_steps(2);
    let start = at(&h, 150.0, 130.0);
    ev(&mut h, Event::PointerMoved(start));
    ev(
        &mut h,
        Event::PointerButton {
            pos: start,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        },
    );
    for i in 1..=10 {
        let p = at(&h, 150.0 + 9.7 * i as f32, 130.0);
        ev(&mut h, Event::PointerMoved(p));
    }
    h.run_steps(3);
    let guides: Vec<(f32, f32, f32, f32)> = {
        let st = &h.state().state;
        let id = st.session.active().expect("doc").id;
        st.canvases
            .get(&id)
            .map(|c| {
                c.feedback
                    .iter()
                    .filter_map(|o| match o {
                        Overlay::SmartGuide { from, to } => Some((from.x, from.y, to.x, to.y)),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default()
    };
    let g = guides
        .iter()
        .find(|g| g.0 == 300.0 && g.2 == 300.0 && g.1 == 50.0)
        .copied()
        .unwrap_or_else(|| panic!("edge guide at x 300 from Q's top: {guides:?}"));
    let shot = match h.render() {
        Ok(i) => i,
        Err(e) => {
            eprintln!("no graphics adapter, pixel check skipped: {e}");
            return;
        }
    };
    let top = at(&h, 300.0, 10.0);
    let (col, row0) = (top.x.floor() as u32, top.y.floor() as u32);
    let magenta = |y: u32| shot.get_pixel(col, y).0[..3] == [255, 74, 255];
    assert!(!magenta(row0 - 1), "nothing above the widened end");
    // the white part above Q: 3 on / 3 off from the upper end
    for k in 0..6u32 {
        for j in 0..3 {
            assert!(magenta(row0 + 6 * k + j), "run {k} row {j} on");
            assert!(!magenta(row0 + 6 * k + 3 + j), "run {k} row {j} off");
        }
    }
    // below R (rows 161..200) the line goes on, and it ends at row 200
    let bottom = at(&h, 300.0, g.3 + 40.0).y.floor() as u32;
    let on_below = (bottom - 30..=bottom).filter(|y| magenta(*y)).count();
    assert!(on_below >= 12, "dashes below R: {on_below}");
    assert!(!(bottom + 1..bottom + 6).any(magenta), "nothing past the widened end");
    // exactly one column: the neighbours are not magenta
    let side = |dx: i32| shot.get_pixel((col as i32 + dx) as u32, row0).0[..3] == [255, 74, 255];
    assert!(!side(-1) && !side(1), "1 px wide");
    let end = at(&h, 247.0, 130.0);
    ev(
        &mut h,
        Event::PointerButton {
            pos: end,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        },
    );
}
