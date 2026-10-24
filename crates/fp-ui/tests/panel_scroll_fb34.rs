//! FB-34: panels scroll vertically when their content is taller than the panel. The Path panel
//! at its docked height showed only "Combine Paths", "Alter Path" and "Edit Points"; the
//! "Compound Shape" group was cut off below, with neither a scroll bar nor mouse-wheel scrolling.
//! Now every docked or floating panel sits in a vertical scroll area whose bar appears only when
//! the content does not fit, so panels that fit are drawn as before.

use egui::accesskit::Role;
use egui::{Event, Modifiers, Rect, pos2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{By, Queryable};
use fp_ui::FirepixelApp;
use fp_ui::app_state::UiSettings;
use fp_ui::docking::panel_vscroll_shown;
use fp_ui::panels::PanelId;
use fp_ui::workspace::{FloatingGroup, PanelGroup};
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn app() -> FirepixelApp {
    fp_engine::shell::disable();
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 200, "height": 150})).expect("doc");
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    app
}

fn harness(app: FirepixelApp, size: egui::Vec2) -> H {
    let mut h = Harness::builder()
        .with_size(size)
        .with_pixels_per_point(1.0)
        .with_max_steps(12)
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    h
}

/// Path panel alone in a floating group of the given height (tab strip included).
fn path_floating(height: f32) -> H {
    let mut app = app();
    let ws = &mut app.state.settings.workspace;
    for g in &mut ws.dock {
        g.panels.retain(|p| *p != PanelId::Path);
        g.active = g.active.min(g.panels.len().saturating_sub(1));
    }
    ws.floating.push(FloatingGroup {
        group: PanelGroup::new(&[PanelId::Path], false),
        pos: [300.0, 120.0],
        size: [268.0, height],
    });
    harness(app, vec2(1280.0, 900.0))
}

fn panel_rect(h: &H, p: PanelId) -> Rect {
    h.state()
        .state
        .dock
        .last_panel_rects
        .iter()
        .find(|(q, _)| *q == p)
        .map(|(_, r)| *r)
        .unwrap_or_else(|| panic!("{p:?} not drawn"))
}

/// The panel's last button ("Combine" of the Compound Shape group) inside the panel's group.
fn combine_button(h: &H) -> Rect {
    let group = panel_rect(h, PanelId::Path);
    let label = fp_ui::panels::common::t("panel-path-combine-compound");
    h.query_all(By::new().role(Role::Button).label(label.as_str()))
        .map(|n| n.rect())
        .find(|r| group.x_range().contains(r.center().x))
        .expect("Combine button of the Path panel")
}

fn wheel(h: &mut H, at: egui::Pos2, lines: f32) {
    h.event(Event::PointerMoved(at));
    h.event(Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: vec2(0.0, lines),
        modifiers: Modifiers::NONE,
        phase: egui::TouchPhase::Move,
    });
    h.run_steps(6);
}

#[test]
fn path_panel_at_250_px_shows_scroll_bar_and_wheel_reveals_compound_shape() {
    let mut h = path_floating(250.0);
    let group = panel_rect(&h, PanelId::Path);
    assert!(panel_vscroll_shown(&h.ctx, PanelId::Path), "content taller than the panel: scroll bar");
    let before = combine_button(&h);
    assert!(before.bottom() > group.bottom(), "Compound Shape cut off at first: {before:?} in {group:?}");
    // mouse wheel down over the panel content (several notches) reveals the group
    wheel(&mut h, pos2(group.center().x, group.bottom() - 40.0), -10.0);
    let after = combine_button(&h);
    assert!(after.top() < before.top(), "scrolled: {before:?} -> {after:?}");
    assert!(after.bottom() <= group.bottom() + 0.5, "Combine fully visible after scrolling: {after:?} in {group:?}");
    let union = fp_ui::panels::common::t("panel-path-union");
    let visible = h
        .query_all(By::new().role(Role::Button).label(union.as_str()))
        .filter(|n| group.contains_rect(n.rect()))
        .count();
    assert!(visible >= 1, "Union of the Compound Shape group visible");
    // and back up
    wheel(&mut h, pos2(group.center().x, group.center().y), 10.0);
    let back = combine_button(&h);
    assert!((back.top() - before.top()).abs() < 0.5, "scrolled back: {before:?} -> {back:?}");
}

#[test]
fn path_panel_at_500_px_has_no_scroll_bar() {
    let h = path_floating(500.0);
    let group = panel_rect(&h, PanelId::Path);
    assert!(!panel_vscroll_shown(&h.ctx, PanelId::Path), "content fits: no scroll bar");
    let combine = combine_button(&h);
    assert!(combine.bottom() <= group.bottom(), "everything visible: {combine:?} in {group:?}");
}

/// The default workspace at the size of the parity runs: every panel of the three open dock
/// groups fits its height and shows no scroll bar (drawn as before).
#[test]
fn default_dock_panels_show_no_scroll_bar() {
    let mut h = harness(app(), vec2(1600.0, 1000.0));
    for gi in 0..3 {
        let n = h.state().state.settings.workspace.dock[gi].panels.len();
        for i in 0..n {
            h.state_mut().state.settings.workspace.dock[gi].active = i;
            h.run_steps(4);
            let p = h.state().state.settings.workspace.dock[gi].panels[i];
            assert!(!panel_vscroll_shown(&h.ctx, p), "{p:?} at its default height: no scroll bar");
        }
    }
}
