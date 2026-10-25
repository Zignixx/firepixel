//! FB-67 (TW-251): View ▸ Live Preview While Dragging Points sits right after Snap to Pixels,
//! unchecked by default; with it on, a Subselection point drag on the canvas renders from a
//! preview copy mid-drag (render key with a preview) and the release writes one History step.
//! FB-72 (TW-252): the selection-tool cursor badges follow Shift/Alt pressed or released without
//! any mouse motion.

use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use fp_engine::ToolEvent;
use fp_engine::tools::{Cursor, ToolId};
use fp_ui::FirepixelApp;
use fp_ui::app_state::UiSettings;
use fp_ui::menu::{self, ItemAction, Node};
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn harness(s: fp_engine::Session) -> H {
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
    h
}

fn session() -> fp_engine::Session {
    fp_engine::shell::disable();
    let mut s = fp_engine::Session::default();
    s.execute(
        "file.new",
        json!({"width": 600, "height": 400, "background": "#ffffff"}),
    )
    .expect("doc");
    s
}

fn at(h: &H, x: f32, y: f32) -> Pos2 {
    let st = &h.state().state;
    let d = st.session.active().expect("document");
    let area = st.canvases.get(&d.id).and_then(|c| c.area).expect("canvas");
    let (sx, sy) = d.view.doc_to_screen(fp_geom::Point::new(x, y));
    Pos2::new(area.min.x + sx as f32, area.min.y + sy as f32)
}

type Entry = (Option<bool>, Option<String>);

/// Check mark of `cmd` in the main menu and the command right before it in its list.
fn menu_entry(h: &H, cmd: &str) -> Entry {
    fn walk(ns: &[Node], cmd: &str) -> Option<Entry> {
        let mut prev: Option<String> = None;
        for n in ns {
            match n {
                Node::Item {
                    checked,
                    action: ItemAction::Command(c),
                    ..
                } => {
                    if c == cmd {
                        return Some((*checked, prev));
                    }
                    prev = Some(c.clone());
                }
                Node::Sub { children, .. } => {
                    if let Some(r) = walk(children, cmd) {
                        return Some(r);
                    }
                    prev = None;
                }
                _ => prev = None,
            }
        }
        None
    }
    menu::build_main_menu(&h.state().state)
        .iter()
        .find_map(|(_, ns)| walk(ns, cmd))
        .expect("menu entry")
}

#[test]
fn fb67_menu_entry_after_snap_to_pixels_and_check_mark_follows() {
    let mut h = harness(session());
    let (ck, prev) = menu_entry(&h, "view.livePointPreview");
    assert_eq!(ck, Some(false), "off by default");
    assert_eq!(prev.as_deref(), Some("view.snapToPixels"));
    h.state_mut()
        .state
        .session
        .execute("view.livePointPreview", json!({}))
        .expect("toggle");
    assert_eq!(menu_entry(&h, "view.livePointPreview").0, Some(true));
    assert!(
        h.state()
            .state
            .session
            .prefs
            .get_bool("LivePointPreview", false)
    );
    h.state_mut()
        .state
        .session
        .execute("view.livePointPreview", json!({}))
        .expect("toggle");
    assert_eq!(menu_entry(&h, "view.livePointPreview").0, Some(false));
}

fn rows(h: &H) -> usize {
    h.state()
        .state
        .session
        .active()
        .map(|d| d.history.undo_count())
        .unwrap_or(0)
}

/// Render key preview of the canvas (0 = the document itself).
fn requested_preview(h: &H) -> u64 {
    let st = &h.state().state;
    let id = st.session.active_id().expect("document");
    st.canvases
        .get(&id)
        .and_then(|c| c.requested)
        .map(|k| k.preview)
        .unwrap_or(0)
}

fn press(h: &mut H, p: Pos2, pressed: bool) {
    h.input_mut().events.push(Event::PointerButton {
        pos: p,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    });
    h.step();
}

/// Drags the first anchor of a Pen triangle with the Subselection tool; returns the render key
/// preview mid-drag and the History rows added by the release.
fn drag_anchor(live: bool) -> (u64, usize) {
    let mut s = session();
    s.set_tool(ToolId::Pen).expect("pen");
    for (x, y) in [
        (100.0, 300.0),
        (200.0, 100.0),
        (300.0, 300.0),
        (100.0, 300.0),
    ] {
        s.tool_event(&ToolEvent::down(x, y)).expect("down");
        s.tool_event(&ToolEvent::up(x, y)).expect("up");
    }
    if live {
        s.execute("view.livePointPreview", json!({}))
            .expect("toggle");
    }
    s.set_tool(ToolId::Subselection).expect("subselection");
    let mut h = harness(s);
    let r0 = rows(&h);
    let p0 = at(&h, 100.0, 300.0);
    h.input_mut().events.push(Event::PointerMoved(p0));
    h.step();
    press(&mut h, p0, true);
    let mut last = p0;
    for (x, y) in [(110.0, 310.0), (130.0, 330.0), (150.0, 350.0)] {
        last = at(&h, x, y);
        h.input_mut().events.push(Event::PointerMoved(last));
        h.step();
    }
    h.step();
    let mid = requested_preview(&h);
    assert_eq!(rows(&h), r0, "no History row while dragging");
    press(&mut h, last, false);
    h.run_steps(6);
    assert_eq!(requested_preview(&h), 0, "no preview after the release");
    (mid, rows(&h) - r0)
}

#[test]
fn fb67_live_point_drag_renders_a_preview_and_commits_one_step() {
    let (mid, added) = drag_anchor(false);
    assert_eq!(
        mid, 0,
        "switch off: outline only, the page renders the document"
    );
    assert_eq!(added, 1);
    let (mid, added) = drag_anchor(true);
    assert_ne!(mid, 0, "switch on: the page renders a live preview copy");
    assert_eq!(added, 1, "the same single step");
}

fn cursor(h: &H) -> Cursor {
    h.state().state.tool_cursor
}

/// Changes only the modifiers (no mouse event).
fn hold(h: &mut H, m: Modifiers) {
    h.input_mut().events.push(Event::ModifiersChanged(m));
    h.step();
    h.run_steps(1);
}

const SHIFT_ALT: Modifiers = Modifiers {
    alt: true,
    ctrl: false,
    shift: true,
    mac_cmd: false,
    command: false,
};

#[test]
fn fb72_selection_badges_follow_modifiers_without_mouse_motion() {
    let mut s = session();
    let img = fp_raster::RgbaImage::filled(
        fp_geom::IRect::new(0, 0, 600, 400),
        fp_color::Rgba8::new(200, 200, 200, 255),
    )
    .expect("image");
    s.clipboard.clear();
    s.clipboard.pixels = Some(img);
    s.execute("edit.paste", json!({})).expect("bitmap");
    let mut h = harness(s);
    let cases = [
        (
            ToolId::Marquee,
            [
                Cursor::Crosshair,
                Cursor::CrosshairAdd,
                Cursor::CrosshairSubtract,
                Cursor::CrosshairIntersect,
            ],
        ),
        (
            ToolId::Lasso,
            [
                Cursor::Lasso,
                Cursor::LassoAdd,
                Cursor::LassoSubtract,
                Cursor::LassoIntersect,
            ],
        ),
        (
            ToolId::MagicWand,
            [
                Cursor::Wand,
                Cursor::WandAdd,
                Cursor::WandSubtract,
                Cursor::WandIntersect,
            ],
        ),
    ];
    for (i, (t, [base, add, sub, int])) in cases.into_iter().enumerate() {
        h.state_mut().state.session.set_tool(t).expect("tool");
        h.state_mut()
            .state
            .session
            .execute(
                "select.pixels",
                json!({"shape": "rect", "rect": [300, 200, 400, 300]}),
            )
            .expect("selection");
        h.run_steps(3);
        // hover outside the selection (a new place per tool), then only modifier changes
        let p = at(&h, 60.0 + 4.0 * i as f32, 60.0);
        h.input_mut()
            .events
            .push(Event::ModifiersChanged(Modifiers::NONE));
        h.input_mut().events.push(Event::PointerMoved(p));
        h.step();
        h.run_steps(1);
        assert_eq!(cursor(&h), base, "{t:?}");
        hold(&mut h, Modifiers::SHIFT);
        assert_eq!(cursor(&h), add, "{t:?}: Shift pressed, no motion");
        hold(&mut h, SHIFT_ALT);
        assert_eq!(cursor(&h), int, "{t:?}: Shift+Alt, no motion");
        hold(&mut h, Modifiers::ALT);
        assert_eq!(cursor(&h), sub, "{t:?}: Shift released, no motion");
        hold(&mut h, Modifiers::NONE);
        assert_eq!(cursor(&h), base, "{t:?}: all released, no motion");
        // inside the pixel selection: Shift pressed without motion shows "+" at once
        let q = at(&h, 350.0 + 4.0 * i as f32, 250.0);
        h.input_mut().events.push(Event::PointerMoved(q));
        h.step();
        h.run_steps(1);
        let inside = cursor(&h);
        hold(&mut h, Modifiers::SHIFT);
        assert_eq!(cursor(&h), add, "{t:?}: inside, Shift pressed, no motion");
        hold(&mut h, Modifiers::NONE);
        assert_eq!(
            cursor(&h),
            inside,
            "{t:?}: inside, Shift released, no motion"
        );
    }
}

/// Pointer: Alt shows the hollow arrow as soon as it is pressed, without motion.
#[test]
fn fb72_pointer_alt_cursor_without_mouse_motion() {
    let mut s = session();
    s.execute("draw.rectangle", json!({"rect": [100, 80, 200, 160]}))
        .expect("rect");
    s.execute("select.set", json!({"ids": [], "mode": "replace"}))
        .expect("none");
    s.set_tool(ToolId::Pointer).expect("pointer");
    let mut h = harness(s);
    let p = at(&h, 400.0, 300.0);
    h.input_mut().events.push(Event::PointerMoved(p));
    h.step();
    h.run_steps(1);
    assert_eq!(cursor(&h), Cursor::Arrow);
    hold(&mut h, Modifiers::ALT);
    assert_eq!(cursor(&h), Cursor::Subselect, "Alt pressed, no motion");
    hold(&mut h, Modifiers::NONE);
    assert_eq!(cursor(&h), Cursor::Arrow, "Alt released, no motion");
}
