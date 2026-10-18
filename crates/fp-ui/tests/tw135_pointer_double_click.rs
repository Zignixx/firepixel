//! TW-135: a pointer double-click on a text opens the text editor with the caret at the click
//! point and keyboard focus in the same frame; a character typed right after the double-click
//! (even within the same frame) is inserted at the click point ("Zeile eins" → "ZeilQe eins").

use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use fp_ui::FirepixelApp;
use fp_ui::app_state::UiSettings;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn harness(s: fp_engine::Session) -> H {
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
    let d = st.session.active().expect("doc");
    let area = st
        .canvases
        .get(&d.id)
        .and_then(|c| c.area)
        .expect("canvas");
    let (sx, sy) = d.view.doc_to_screen(fp_geom::Point::new(x, y));
    Pos2::new(area.min.x + sx as f32, area.min.y + sy as f32)
}

fn text_of(h: &H, id: u64) -> Option<String> {
    let d = h.state().state.session.active()?;
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, fp_model::ElementId(id))?;
    match &e.kind {
        fp_model::ElementKind::Text(t) => Some(t.plain_text()),
        _ => None,
    }
}

fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

/// Session with "Zeile eins" / "Zeile zwei" (14 px), the pointer tool, and a document point just
/// behind the middle of the "l" of the first line (caret index 4).
fn setup() -> (H, u64, Pos2) {
    fp_engine::shell::disable();
    let mut s = fp_engine::Session::default();
    s.execute(
        "file.new",
        json!({"width": 300, "height": 240, "background": "#ffffff"}),
    )
    .expect("doc");
    let v = s
        .execute(
            "draw.text",
            json!({"text": "Zeile eins\rZeile zwei", "anchor": [50, 160], "size": 14, "color": "#000000"}),
        )
        .expect("text");
    let id = v.value["id"].as_u64().unwrap_or(0);
    s.execute("select.none", json!({})).expect("select");
    s.execute("tool.select", json!({"tool": "tool.pointer"}))
        .expect("tool");
    let (px, py) = {
        let d = s.active().expect("doc");
        let e = fp_engine::docops::element_on_page(&d.doc, d.page, fp_model::ElementId(id))
            .expect("text");
        let fp_model::ElementKind::Text(t) = &e.kind else {
            panic!("not a text")
        };
        let block = fp_render::convert::text_block(&d.doc, t);
        let lay = fp_text::layout(
            &block,
            &fp_text::FontDb::shared(),
            &fp_text::LayoutOptions::default(),
        );
        let x = |i: usize| lay.caret(i).expect("caret").top[0];
        let c = lay.caret(0).expect("caret");
        let mid_l = (x(3) + x(4)) / 2.0;
        (
            (mid_l + 0.3 * (x(4) - mid_l)) as f32,
            ((c.top[1] + c.bottom[1]) / 2.0) as f32,
        )
    };
    let h = harness(s);
    let p = screen_of(&h, px, py);
    (h, id, p)
}

fn escape(h: &mut H) {
    for pressed in [true, false] {
        h.event(Event::Key {
            key: Key::Escape,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: Modifiers::NONE,
        });
    }
    h.run_steps(3);
}

/// Double-click, then "Q" arriving in the same frame as the second release.
#[test]
fn double_click_then_type_in_same_frame_inserts_at_click_point() {
    let (mut h, id, p) = setup();
    h.event(Event::PointerMoved(p));
    h.run_steps(1);
    h.event(button(p, true));
    h.run_steps(1);
    h.event(button(p, false));
    h.run_steps(1);
    h.event(button(p, true));
    h.run_steps(1);
    h.event(button(p, false));
    h.event(Event::Key {
        key: Key::Q,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    });
    h.event(Event::Text("Q".into()));
    h.run_steps(1);
    {
        let st = &h.state().state;
        assert_eq!(st.session.tools.active, fp_engine::ToolId::Text);
        assert!(st.session.tool_is_editing_text(), "editor open");
    }
    // later frames must not move the caret away again
    h.run_steps(3);
    h.event(Event::Text("u".into()));
    h.run_steps(1);
    escape(&mut h);
    assert_eq!(text_of(&h, id).as_deref(), Some("ZeilQue eins\rZeile zwei"));
}

/// Double-click, then "Q", "u", "x" in the following frames.
#[test]
fn double_click_then_type_next_frame_inserts_at_click_point() {
    let (mut h, id, p) = setup();
    h.event(Event::PointerMoved(p));
    h.run_steps(1);
    for _ in 0..2 {
        h.event(button(p, true));
        h.run_steps(1);
        h.event(button(p, false));
        h.run_steps(1);
    }
    // one character per frame, as the parity runner types: no late caret report may move the
    // caret back to the click point in between
    for c in ["Q", "u", "x"] {
        h.event(Event::Text(c.into()));
        h.run_steps(1);
    }
    escape(&mut h);
    assert_eq!(text_of(&h, id).as_deref(), Some("ZeilQuxe eins\rZeile zwei"));
}
