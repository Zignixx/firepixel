//! TW-219 (RE spec 67 §1.1, §2.10): the transform tools draw the frame line and its handles
//! 1 screen pixel left of and above the frame (for every object, at every zoom); the frame of one
//! text is its set box scaled by 1.3, shown with the corners on whole document pixels.
//! TW-220 (RE spec 67 §1.1, §2.7, §2.10): line and handles are black, the handles solid 5 × 5
//! squares on the pixel left of and above the frame coordinate, the line through their centres;
//! the Pointer keeps its selection-blue handles.

use egui::Pos2;
use egui_kittest::Harness;
use fp_engine::ToolId;
use fp_ui::FirepixelApp;
use fp_ui::app_state::UiSettings;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn harness(s: fp_engine::Session) -> H {
    fp_engine::shell::disable();
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .with_pixels_per_point(1.0)
        .with_max_steps(12)
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    h
}

fn screen_of(h: &H, x: f32, y: f32) -> Pos2 {
    let st = &h.state().state;
    let d = st.session.active().expect("document");
    let area = st.canvases.get(&d.id).and_then(|c| c.area).expect("canvas");
    let (sx, sy) = d.view.doc_to_screen(fp_geom::Point::new(x, y));
    Pos2::new(area.min.x + sx as f32, area.min.y + sy as f32)
}

fn fills(shape: &egui::epaint::Shape, col: egui::Color32, out: &mut Vec<egui::Rect>) {
    match shape {
        egui::epaint::Shape::Rect(r) if r.fill == col => out.push(r.rect),
        egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| fills(s, col, out)),
        _ => {}
    }
}

fn lines(shape: &egui::epaint::Shape, col: egui::Color32, out: &mut Vec<Vec<Pos2>>) {
    match shape {
        egui::epaint::Shape::Path(p)
            if p.closed
                && p.stroke.color == egui::epaint::ColorMode::Solid(col)
                && (p.stroke.width - 1.0).abs() < 0.01 =>
        {
            out.push(p.points.clone())
        }
        egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| lines(s, col, out)),
        _ => {}
    }
}

/// Painted solid squares of the colour `col` on the canvas.
fn squares(h: &H, col: egui::Color32) -> Vec<egui::Rect> {
    let st = &h.state().state;
    let d = st.session.active().expect("document");
    let area = st.canvases.get(&d.id).and_then(|c| c.area).expect("canvas");
    let mut rs = Vec::new();
    for cs in &h.output().shapes {
        fills(&cs.shape, col, &mut rs);
    }
    rs.retain(|r| area.contains_rect(*r));
    rs
}

/// Centres of the painted black 5 × 5 handle squares (every black square must be 5 × 5).
fn handle_centres(h: &H) -> Vec<Pos2> {
    let rs = squares(h, egui::Color32::BLACK);
    for r in &rs {
        assert!(
            (r.width() - 5.0).abs() < 0.01 && (r.height() - 5.0).abs() < 0.01,
            "black square not 5 x 5: {r:?}"
        );
    }
    rs.iter().map(|r| r.center()).collect()
}

fn session() -> fp_engine::Session {
    fp_i18n::profile::use_test_profile();
    let mut s = fp_engine::Session::default();
    s.execute(
        "file.new",
        json!({"width": 600, "height": 400, "background": "#ffffff"}),
    )
    .expect("document");
    s
}

/// The screen pixel left of and above the frame point `x`,`y` (its centre).
fn shifted_pixel(h: &H, x: f32, y: f32) -> Pos2 {
    let p = screen_of(h, x, y);
    Pos2::new(p.x - 0.5, p.y - 0.5)
}

fn assert_handles(h: &H, want: &[(f32, f32)], what: &str) {
    let got = handle_centres(h);
    for &(x, y) in want {
        let shifted = shifted_pixel(h, x, y);
        assert!(
            got.iter().any(|g| g.distance(shifted) < 0.01),
            "{what}: no black handle on the pixel up/left of {x},{y} ({shifted:?}) in {got:?}"
        );
        // the 5 × 5 square covers whole pixels: 2 left/above and 2 right/below the centre pixel
        let r = egui::Rect::from_center_size(shifted, egui::vec2(5.0, 5.0));
        assert_eq!(r.min, r.min.round(), "{what}: handle not on whole pixels");
        let p = screen_of(h, x, y);
        assert!(
            !got.iter().any(|g| g.distance(p) < 0.6),
            "{what}: handle drawn on {x},{y} itself"
        );
    }
    assert_eq!(got.len(), 2 * want.len() - 4, "{what}: 4 corner squares twice, 4 middles");
}

/// The black frame line runs through the pixel centres up/left of the corners.
fn assert_line(h: &H, corners: &[(f32, f32)], what: &str) {
    let mut ls = Vec::new();
    for cs in &h.output().shapes {
        lines(&cs.shape, egui::Color32::BLACK, &mut ls);
    }
    let frame = ls
        .iter()
        .find(|l| l.len() == 4)
        .unwrap_or_else(|| panic!("{what}: no black frame line in {ls:?}"));
    for &(x, y) in corners {
        let want = shifted_pixel(h, x, y);
        assert!(
            frame.iter().any(|p| p.distance(want) < 0.01),
            "{what}: frame line misses {want:?}: {frame:?}"
        );
    }
}

#[test]
fn scale_tool_draws_the_frame_one_screen_pixel_up_and_left() {
    rect_case(ToolId::Scale);
}

#[test]
fn skew_and_distort_draw_the_same_black_frame() {
    // RE spec 67 §2.7: the same frame pixel for pixel under all transform tools
    rect_case(ToolId::Skew);
    rect_case(ToolId::Distort);
}

fn rect_case(tool: ToolId) {
    let mut s = session();
    let v = s
        .execute("draw.rectangle", json!({"rect": [100, 100, 200, 160]}))
        .expect("rectangle");
    let id = v.value["id"].as_u64().expect("id");
    s.execute("select.set", json!({"ids": [id]}))
        .expect("select");
    s.set_tool(tool).expect("transform tool");
    let mut h = harness(s);
    h.run_steps(2);
    assert_handles(
        &h,
        &[
            (100.0, 100.0),
            (150.0, 100.0),
            (200.0, 100.0),
            (200.0, 130.0),
            (200.0, 160.0),
            (150.0, 160.0),
            (100.0, 160.0),
            (100.0, 130.0),
        ],
        "rectangle",
    );
    assert_line(
        &h,
        &[(100.0, 100.0), (200.0, 100.0), (200.0, 160.0), (100.0, 160.0)],
        "rectangle",
    );
}

#[test]
fn pointer_keeps_blue_handles_without_a_black_frame() {
    let mut s = session();
    let v = s
        .execute("draw.rectangle", json!({"rect": [100, 100, 200, 160]}))
        .expect("rectangle");
    let id = v.value["id"].as_u64().expect("id");
    s.execute("select.set", json!({"ids": [id]}))
        .expect("select");
    s.set_tool(ToolId::Pointer).expect("Pointer");
    let mut h = harness(s);
    h.run_steps(2);
    assert!(
        squares(&h, egui::Color32::BLACK).is_empty(),
        "Pointer draws black handles"
    );
}

#[test]
fn text_frame_handles_lie_on_the_rounded_scaled_set_box() {
    if fp_text::FontDb::shared().find_ps("ArialMT").is_none() {
        eprintln!("skipped: ArialMT not installed");
        return;
    }
    // "Ag" ArialMT 60 pt at raw 100,100 (74 × 72): frame 88.9–185.1 × 89.2–182.8, shown at
    // 89/137/185 × 89/136/183 (B: on screen at 100 % 88/136/184 × 88/135/182 from the origin)
    let mut s = session();
    let v = s
        .execute(
            "draw.text",
            json!({"text": "Ag", "anchor": [101.0, 152.0], "size": 60.0, "font": "ArialMT", "color": "#000000"}),
        )
        .expect("text");
    let id = v.value["id"].as_u64().expect("id");
    s.execute("select.set", json!({"ids": [id]}))
        .expect("select");
    s.set_tool(ToolId::Scale).expect("Scale tool");
    let mut h = harness(s);
    h.run_steps(2);
    assert_handles(
        &h,
        &[
            (89.0, 89.0),
            (137.0, 89.0),
            (185.0, 89.0),
            (185.0, 136.0),
            (185.0, 183.0),
            (137.0, 183.0),
            (89.0, 183.0),
            (89.0, 136.0),
        ],
        "text",
    );
    assert_line(
        &h,
        &[(89.0, 89.0), (185.0, 89.0), (185.0, 183.0), (89.0, 183.0)],
        "text",
    );
}
