//! FB-32 / FB-33 (RE spec 69 sections 4 and 5) on the real canvas: the cursor of Marquee, Lasso
//! and Magic Wand shows "+" / "-" / "x" only while a pixel selection exists and changes with the
//! next mouse move; inside the selection the move cursors; the Pointer with Alt shows the hollow
//! arrow. The Pointer's Shift-marquee toggles. Firepixel extension: the marquee shape option
//! "Rectangle / Lasso" in the Properties inspector of the Pointer and the Subselection.

use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fp_engine::tools::{Cursor, POINTER_LASSO_PREF, ToolId};
use fp_model::ElementId;
use fp_ui::FirepixelApp;
use fp_ui::app_state::UiSettings;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn session() -> fp_engine::Session {
    fp_engine::shell::disable();
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 600, "height": 400, "background": "#ffffff"})).expect("doc");
    s
}

fn harness(s: fp_engine::Session) -> H {
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).with_pixels_per_point(1.0).with_max_steps(12).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    h
}

fn at(h: &H, x: f32, y: f32) -> Pos2 {
    let st = &h.state().state;
    let d = st.session.active().expect("document");
    let area = st.canvases.get(&d.id).and_then(|c| c.area).expect("canvas");
    let (sx, sy) = d.view.doc_to_screen(fp_geom::Point::new(x, y));
    Pos2::new(area.min.x + sx as f32, area.min.y + sy as f32)
}

fn cursor(h: &H) -> Cursor {
    h.state().state.tool_cursor
}

fn run(h: &mut H, cmd: &str, args: serde_json::Value) -> serde_json::Value {
    let v = h.state_mut().state.session.execute(cmd, args).expect(cmd).value;
    h.run_steps(3);
    v
}

fn tool(h: &mut H, t: ToolId) {
    h.state_mut().state.session.set_tool(t).expect("tool");
    h.run_steps(3);
}

/// Hold the modifiers `m` (as the platform reports them) without moving the mouse.
fn hold(h: &mut H, m: Modifiers) {
    h.input_mut().events.push(Event::ModifiersChanged(m));
    h.step();
    h.run_steps(1);
}

/// Mouse move to the document point with `m` held.
fn hover(h: &mut H, x: f32, y: f32, m: Modifiers) -> Cursor {
    let p = at(h, x, y);
    h.input_mut().events.push(Event::ModifiersChanged(m));
    h.input_mut().events.push(Event::PointerMoved(p));
    h.step();
    h.run_steps(1);
    cursor(h)
}

fn button(h: &mut H, p: Pos2, pressed: bool, m: Modifiers) {
    h.input_mut().events.push(Event::PointerButton { pos: p, button: PointerButton::Primary, pressed, modifiers: m });
    h.step();
}

/// Drag along the document points with `m` held, released at the last one.
fn drag(h: &mut H, pts: &[(f32, f32)], m: Modifiers) {
    let first = at(h, pts[0].0, pts[0].1);
    h.input_mut().events.push(Event::ModifiersChanged(m));
    h.input_mut().events.push(Event::PointerMoved(first));
    h.step();
    button(h, first, true, m);
    let mut last = first;
    for (x, y) in &pts[1..] {
        last = at(h, *x, *y);
        h.input_mut().events.push(Event::PointerMoved(last));
        h.step();
    }
    button(h, last, false, m);
    h.input_mut().events.push(Event::ModifiersChanged(Modifiers::NONE));
    h.run_steps(30);
}

fn sel(h: &H) -> Vec<ElementId> {
    let mut v = h.state().state.session.active().map(|d| d.selection.elements.clone()).unwrap_or_default();
    v.sort_by_key(|i| i.0);
    v
}

fn id_of(v: &serde_json::Value) -> ElementId {
    ElementId(v["id"].as_u64().or_else(|| v["ids"][0].as_u64()).expect("id"))
}

const SHIFT_ALT: Modifiers = Modifiers { alt: true, ctrl: false, shift: true, mac_cmd: false, command: false };

/// The badges per tool, modifier and pixel-selection state, as the canvas reports the cursor.
#[test]
fn fb32_cursor_badges_per_tool_modifier_and_selection() {
    let mut s = session();
    let img = fp_raster::RgbaImage::filled(fp_geom::IRect::new(0, 0, 600, 400), fp_color::Rgba8::new(200, 200, 200, 255)).expect("image");
    s.clipboard.clear();
    s.clipboard.pixels = Some(img);
    s.execute("edit.paste", json!({})).expect("bitmap");
    let mut h = harness(s);
    let cases = [
        (ToolId::Marquee, Cursor::Crosshair, [Cursor::CrosshairAdd, Cursor::CrosshairSubtract, Cursor::CrosshairIntersect]),
        (ToolId::Lasso, Cursor::Lasso, [Cursor::LassoAdd, Cursor::LassoSubtract, Cursor::LassoIntersect]),
        (ToolId::MagicWand, Cursor::Wand, [Cursor::WandAdd, Cursor::WandSubtract, Cursor::WandIntersect]),
    ];
    for (t, base, _) in cases {
        tool(&mut h, t);
        run(&mut h, "select.pixels", json!({"shape": "none"}));
        assert_eq!(hover(&mut h, 60.0, 60.0, Modifiers::NONE), base, "{t:?}");
        assert_eq!(hover(&mut h, 62.0, 60.0, Modifiers::SHIFT), base, "{t:?}: no badge without a pixel selection");
        assert_eq!(hover(&mut h, 64.0, 60.0, Modifiers::ALT), base, "{t:?}: no badge without a pixel selection");
        assert_eq!(hover(&mut h, 66.0, 60.0, SHIFT_ALT), base, "{t:?}: no badge without a pixel selection");
    }
    for (t, base, [add, sub, int]) in cases {
        tool(&mut h, t);
        run(&mut h, "select.pixels", json!({"shape": "rect", "rect": [300, 200, 400, 300]}));
        assert_eq!(hover(&mut h, 60.0, 60.0, Modifiers::NONE), base, "{t:?}");
        assert_eq!(hover(&mut h, 62.0, 60.0, Modifiers::SHIFT), add, "{t:?} Shift");
        assert_eq!(hover(&mut h, 64.0, 60.0, Modifiers::ALT), sub, "{t:?} Alt");
        assert_eq!(hover(&mut h, 66.0, 60.0, SHIFT_ALT), int, "{t:?} Shift+Alt");
        assert_eq!(hover(&mut h, 350.0, 250.0, Modifiers::NONE), Cursor::MoveMarquee, "{t:?} inside");
        assert_eq!(hover(&mut h, 352.0, 250.0, Modifiers::CTRL), Cursor::MovePixels, "{t:?} inside + Ctrl");
    }
}

/// The cursor follows a modifier on the next mouse move (RE spec 69 section 4.2).
#[test]
fn fb32_cursor_changes_on_the_next_mouse_move() {
    let mut s = session();
    let img = fp_raster::RgbaImage::filled(fp_geom::IRect::new(0, 0, 600, 400), fp_color::Rgba8::new(200, 200, 200, 255)).expect("image");
    s.clipboard.clear();
    s.clipboard.pixels = Some(img);
    s.execute("edit.paste", json!({})).expect("bitmap");
    let mut h = harness(s);
    tool(&mut h, ToolId::Marquee);
    run(&mut h, "select.pixels", json!({"shape": "rect", "rect": [300, 200, 400, 300]}));
    assert_eq!(hover(&mut h, 60.0, 60.0, Modifiers::NONE), Cursor::Crosshair);
    hold(&mut h, Modifiers::SHIFT);
    assert_eq!(cursor(&h), Cursor::Crosshair, "no change before the mouse moves");
    assert_eq!(hover(&mut h, 61.0, 60.0, Modifiers::SHIFT), Cursor::CrosshairAdd, "after the move");
}

/// Pointer: Alt hover = hollow arrow, Shift and Ctrl no badge; Subselection: no badge on hover.
#[test]
fn fb32_pointer_and_subselection_hover_cursors() {
    let mut s = session();
    s.execute("draw.rectangle", json!({"rect": [100, 80, 200, 160]})).expect("rect");
    s.execute("select.set", json!({"ids": [], "mode": "replace"})).expect("none");
    let mut h = harness(s);
    tool(&mut h, ToolId::Pointer);
    assert_eq!(hover(&mut h, 400.0, 300.0, Modifiers::NONE), Cursor::Arrow);
    assert_eq!(hover(&mut h, 402.0, 300.0, Modifiers::ALT), Cursor::Subselect, "Alt over empty canvas");
    assert_eq!(hover(&mut h, 150.0, 120.0, Modifiers::ALT), Cursor::Subselect, "Alt over an object");
    assert_eq!(hover(&mut h, 152.0, 120.0, Modifiers::SHIFT), Cursor::Arrow, "Shift: no badge");
    assert_eq!(hover(&mut h, 154.0, 120.0, Modifiers::CTRL), Cursor::Arrow, "Ctrl: arrow");
    tool(&mut h, ToolId::Subselection);
    let base = hover(&mut h, 400.0, 300.0, Modifiers::NONE);
    assert_eq!(hover(&mut h, 402.0, 300.0, Modifiers::ALT), base, "Subselection Alt: no badge");
    assert_eq!(hover(&mut h, 404.0, 300.0, Modifiers::SHIFT), base, "Subselection Shift: no badge");
}

/// Pointer Shift-marquee toggles the objects inside (selected ones become deselected).
#[test]
fn fb33_pointer_shift_marquee_toggles_on_the_canvas() {
    let mut s = session();
    let a = id_of(&s.execute("draw.rectangle", json!({"rect": [100, 80, 200, 160]})).expect("a").value);
    let b = id_of(&s.execute("draw.rectangle", json!({"rect": [250, 80, 350, 160]})).expect("b").value);
    s.execute("select.set", json!({"ids": [a.0], "mode": "replace"})).expect("a");
    let mut h = harness(s);
    tool(&mut h, ToolId::Pointer);
    let rows = |h: &H| h.state().state.session.active().map(|d| d.history.undo_count()).unwrap_or(0);
    let r0 = rows(&h);
    drag(&mut h, &[(80.0, 60.0), (200.0, 120.0), (370.0, 180.0)], Modifiers::SHIFT);
    assert_eq!(sel(&h), vec![b], "A toggled off, B on");
    assert_eq!(rows(&h), r0, "no History row");
}

/// Inspector of the Pointer and the Subselection: the marquee shape "Rectangle / Lasso" reads and
/// writes `Firepixel_PointerLasso`; with Lasso a free-form drag selects the enclosed objects.
#[test]
fn lasso_marquee_option_in_the_inspector() {
    let mut s = session();
    let a = id_of(&s.execute("draw.rectangle", json!({"rect": [100, 80, 200, 160]})).expect("a").value);
    let _b = id_of(&s.execute("draw.rectangle", json!({"rect": [250, 80, 350, 160]})).expect("b").value);
    let c = id_of(&s.execute("draw.rectangle", json!({"rect": [400, 200, 500, 300]})).expect("c").value);
    s.execute("select.set", json!({"ids": [], "mode": "replace"})).expect("none");
    let mut h = harness(s);
    tool(&mut h, ToolId::Pointer);
    let rect_t = fp_i18n::t("pi-pointer-marquee-rect");
    let lasso_t = fp_i18n::t("pi-pointer-marquee-lasso");
    let combo = |h: &H, v: &str| {
        h.query_all(egui_kittest::kittest::By::new().role(egui::accesskit::Role::ComboBox))
            .find(|n| n.value().as_deref() == Some(v))
            .map(|n| n.rect())
    };
    let r = combo(&h, &rect_t).expect("marquee shape list shows Rectangle");
    let pref = |h: &H| h.state().state.session.prefs.get_bool(POINTER_LASSO_PREF, false);
    assert!(!pref(&h), "default: rectangle");
    // open the list and pick "Lasso"
    let c0 = r.center();
    h.event(Event::PointerMoved(c0));
    button(&mut h, c0, true, Modifiers::NONE);
    button(&mut h, c0, false, Modifiers::NONE);
    h.run_steps(3);
    let item = h.query_all_by_label(&lasso_t).map(|n| n.rect()).find(|q| (q.center().y - c0.y).abs() > 2.0).expect("list item Lasso");
    h.event(Event::PointerMoved(item.center()));
    button(&mut h, item.center(), true, Modifiers::NONE);
    button(&mut h, item.center(), false, Modifiers::NONE);
    h.run_steps(4);
    assert!(pref(&h), "the list writes Firepixel_PointerLasso");
    assert!(combo(&h, &lasso_t).is_some(), "the list shows Lasso");
    // the same option on the Subselection
    tool(&mut h, ToolId::Subselection);
    assert!(combo(&h, &lasso_t).is_some(), "Subselection shows the option too");
    tool(&mut h, ToolId::Pointer);
    // a free-form outline around A and C, leaving out B
    drag(&mut h, &[(70.0, 50.0), (220.0, 60.0), (220.0, 180.0), (520.0, 180.0), (520.0, 320.0), (80.0, 320.0), (70.0, 330.0)], Modifiers::NONE);
    let mut want = vec![a, c];
    want.sort_by_key(|i| i.0);
    assert_eq!(sel(&h), want, "the lasso selects A and C");
    // set elsewhere (MCP): the list follows (the tool row shows without a selection)
    run(&mut h, "select.set", json!({"ids": [], "mode": "replace"}));
    run(&mut h, "tool.pointerLasso", json!({"on": false}));
    assert!(combo(&h, &rect_t).is_some());
}
