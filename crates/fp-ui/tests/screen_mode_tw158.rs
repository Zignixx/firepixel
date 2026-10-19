//! TW-158: three screen modes. `F` cycles Standard -> full screen with menus -> full screen ->
//! Standard and does nothing without a document; full screen shows only the canvas on black (page
//! centred on the window at the unchanged zoom, no menu bar, tabs, status bar, toolbox or panels);
//! the canvas context menu leads back to Standard; full screen with menus keeps the menu bar and
//! the toolbar but has no document tabs.

use egui::{Color32, Event, Key, Modifiers, Rect, pos2};
use egui_kittest::Harness;
use fp_ui::FirepixelApp;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::menu::{ItemAction, Node};
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

const W: f32 = 1400.0;
const HH: f32 = 900.0;

fn harness(with_doc: bool) -> H {
    fp_engine::shell::disable();
    let mut s = fp_engine::Session::default();
    if with_doc {
        s.execute("file.new", json!({"width": 500, "height": 300, "background": "#ffffff"}))
            .expect("doc");
    }
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder()
        .with_size(egui::vec2(W, HH))
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

fn mode(h: &H) -> u8 {
    st(h).session.ui.screen_mode
}

fn press_f(h: &mut H) {
    for pressed in [true, false] {
        h.event(Event::Key {
            key: Key::F,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: Modifiers::NONE,
        });
        h.run_steps(1);
    }
    h.run_steps(3);
}

fn texts(shape: &egui::epaint::Shape, out: &mut Vec<(String, Rect)>) {
    match shape {
        egui::epaint::Shape::Text(t) => out.push((
            t.galley.job.text.clone(),
            Rect::from_min_size(t.pos, t.galley.size()),
        )),
        egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| texts(s, out)),
        _ => {}
    }
}

fn all_texts(h: &H) -> Vec<(String, Rect)> {
    let mut out = Vec::new();
    for cs in &h.output().shapes {
        texts(&cs.shape, &mut out);
    }
    out
}

fn fills(shape: &egui::epaint::Shape, out: &mut Vec<(Rect, Color32)>) {
    match shape {
        egui::epaint::Shape::Rect(r) => out.push((r.rect, r.fill)),
        egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| fills(s, out)),
        _ => {}
    }
}

fn all_fills(h: &H) -> Vec<(Rect, Color32)> {
    let mut out = Vec::new();
    for cs in &h.output().shapes {
        fills(&cs.shape, &mut out);
    }
    out
}

fn doc_title(h: &H) -> String {
    st(h).session.active().expect("doc").title.clone()
}

fn canvas_area(h: &H) -> Rect {
    let id = st(h).session.active().expect("doc").id;
    st(h).canvases.get(&id).and_then(|c| c.area).expect("area")
}

fn page_screen(h: &H) -> Rect {
    let id = st(h).session.active().expect("doc").id;
    st(h).canvases.get(&id).and_then(|c| c.page_screen).expect("page")
}

#[test]
fn f_without_document_does_nothing() {
    let mut h = harness(false);
    press_f(&mut h);
    assert_eq!(mode(&h), 0, "no document: F is a no-op");
    let mut s = fp_engine::Session::default();
    s.execute("view.toggleScreenMode", json!({})).expect("cmd");
    assert_eq!(s.ui.screen_mode, 0, "command without a document");
}

#[test]
fn f_cycles_standard_menus_full_standard() {
    let mut h = harness(true);
    let mut seen = Vec::new();
    for _ in 0..3 {
        press_f(&mut h);
        seen.push(mode(&h));
    }
    assert_eq!(seen, vec![1, 2, 0]);
}

#[test]
fn full_screen_shows_only_the_canvas_on_black() {
    let mut h = harness(true);
    let title = doc_title(&h);
    assert!(
        all_texts(&h).iter().any(|(t, _)| t.contains(&title)),
        "Standard shows the document tab"
    );
    let zoom = st(&h).session.active().expect("d").view.zoom;
    press_f(&mut h);
    press_f(&mut h);
    assert_eq!(mode(&h), 2);
    h.run_steps(2);
    // the canvas covers the whole window: no menu bar, toolbar, tabs, preview bar, status bar,
    // toolbox, dock or properties around it
    // the harness wraps the app in a panel with an 8 px margin: the root ui is the "window"
    let screen = Rect::from_min_size(pos2(0.0, 0.0), egui::vec2(W, HH)).shrink(8.0);
    assert_eq!(canvas_area(&h), screen, "canvas = whole window");
    let page = page_screen(&h);
    let txt: Vec<_> = all_texts(&h)
        .into_iter()
        .filter(|(_, r)| !page.contains_rect(*r))
        .collect();
    assert!(txt.is_empty(), "no text outside the page in full screen: {txt:?}");
    // black pasteboard over the whole window, no page shadow
    let fl = all_fills(&h);
    assert!(
        fl.iter().any(|(r, c)| *c == Color32::BLACK && r.contains_rect(screen)),
        "black background"
    );
    assert!(
        !fl.iter().any(|(r, c)| *c != Color32::BLACK
            && c.r() == 0
            && c.g() == 0
            && c.b() == 0
            && c.a() > 0
            && *r == page.translate(egui::vec2(2.0, 2.0))),
        "no shadow"
    );
    // page centred on the window at the unchanged zoom
    let z = st(&h).session.active().expect("d").view.zoom;
    assert!((z - zoom).abs() < 1e-9, "zoom unchanged");
    assert!((page.center().x - W / 2.0).abs() <= 1.0, "{page:?}");
    assert!((page.center().y - HH / 2.0).abs() <= 1.0, "{page:?}");
    assert!((page.width() - 500.0 * z as f32).abs() <= 1.0);
    // Esc does not leave full screen
    for pressed in [true, false] {
        h.event(Event::Key {
            key: Key::Escape,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: Modifiers::NONE,
        });
        h.run_steps(1);
    }
    assert_eq!(mode(&h), 2, "Esc keeps full screen");
}

#[test]
fn context_menu_exit_full_screen_returns_to_standard() {
    let mut h = harness(true);
    press_f(&mut h);
    press_f(&mut h);
    assert_eq!(mode(&h), 2);
    let label = fp_i18n::t("ctx-exitFullScreen");
    let m = fp_ui::canvas::context::open_at(&mut h.state_mut().state, fp_geom::Point::new(250.0, 150.0));
    let nodes = fp_ui::canvas::context::build(&h.state().state, fp_ui::canvas::context::entries(m.id));
    let cmd = nodes
        .iter()
        .find_map(|n| match n {
            Node::Item { label: l, action: ItemAction::Command(c), enabled: true, .. } if *l == label => {
                Some(c.clone())
            }
            _ => None,
        })
        .expect("enabled entry 'Exit Full Screen Mode' in the canvas menu");
    h.state_mut().state.session.execute(&cmd, json!({})).expect("exit");
    h.run_steps(3);
    assert_eq!(mode(&h), 0);
    // in Standard the entry stays but is disabled
    let nodes = fp_ui::canvas::context::build(&h.state().state, fp_ui::canvas::context::entries(m.id));
    assert!(nodes.iter().any(|n| matches!(n, Node::Item { label: l, enabled: false, .. } if *l == label)));
}

#[test]
fn full_screen_with_menus_keeps_menu_bar_and_toolbar_without_tabs() {
    let mut h = harness(true);
    let title = doc_title(&h);
    let m = fp_ui::theme::Theme::current(&h.ctx).metrics.clone();
    let std_area = canvas_area(&h);
    press_f(&mut h);
    assert_eq!(mode(&h), 1);
    h.run_steps(2);
    let txt = all_texts(&h);
    assert!(
        txt.iter().any(|(_, r)| r.center().y < m.app_bar_height),
        "menu bar still drawn"
    );
    assert!(!txt.iter().any(|(t, _)| t.contains(&title)), "no document tabs");
    let area = canvas_area(&h);
    // document area spans the full width below menu bar and toolbar (tabs gone)
    assert!(area.top() >= m.app_bar_height + m.toolbar_height - 1.0, "{area:?}");
    assert!(area.top() < std_area.top(), "no tab row: {area:?} vs {std_area:?}");
    assert!(area.width() > std_area.width(), "full width: {area:?} vs {std_area:?}");
    // the next F goes on to full screen
    h.state_mut().state.session.execute("view.toggleScreenMode", json!({})).expect("F");
    assert_eq!(mode(&h), 2);
}

#[test]
fn showing_a_panel_returns_to_standard() {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 100, "height": 100, "background": "#ffffff"}))
        .expect("doc");
    s.execute("view.toggleScreenMode", json!({})).expect("F");
    s.execute("view.toggleScreenMode", json!({})).expect("F");
    assert_eq!(s.ui.screen_mode, 2);
    let shown = s.ui.panels.get("window.history").copied().unwrap_or(false);
    assert!(!shown, "history hidden by default");
    s.execute("window.history", json!({})).expect("show");
    assert_eq!(s.ui.screen_mode, 0);
}
