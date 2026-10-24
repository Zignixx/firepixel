//! FB-46 (tester feedback): Undo / Redo while a slider pop-up with a live preview is open.
//! Ctrl+Z / Ctrl+Y (and Undo / Redo from the menu, a jump in the History panel) first discard the
//! pending value and close the pop-up like Esc, then move in the history; the pending value never
//! lands on the objects the undo selects. A pending value commits only to the objects it was
//! started for. Covered: the inspector's opacity slider, the Layers panel's opacity slider, the
//! alpha of the inspector's colour pop-up and the History panel; plus the commit guard itself.
//!
//! Realistic input like egui-winit (one event per frame).

use egui::{Event, Id, Key, Modifiers, PointerButton, Pos2, Rect};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fp_ui::FirepixelApp;
use fp_ui::app_state::{Action, AppState, UiSettings};
use fp_ui::inspector::color_picker as cp;
use fp_ui::inspector::fields::{VSLIDER_PERCENT_PAD, VSLIDER_PERCENT_TRACK, VTrack};
use fp_ui::inspector::preview::{self, PreviewTarget, PreviewValue};
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

const W: f32 = 1800.0;
const HH: f32 = 1130.0;

const CTRL: Modifiers = Modifiers { alt: false, ctrl: true, shift: false, mac_cmd: false, command: true };

/// Red rectangle A and blue rectangle B. The last history step changes B's fill (B selected),
/// then A is selected, so an undo selects B again. Returns the harness and the ids (A, B).
fn harness() -> (H, u64, u64) {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 500, "height": 300, "background": "#ffffff"})).expect("doc");
    let id = |v: fp_engine::CommandOutput| v.value.get("id").and_then(|x| x.as_u64()).expect("id");
    let a = id(s.execute("draw.rectangle", json!({"rect": [40, 40, 160, 130], "fillColor": "#ff0000", "strokeWidth": 0})).expect("A"));
    let b = id(s.execute("draw.rectangle", json!({"rect": [260, 40, 380, 130], "fillColor": "#0000ff", "strokeWidth": 0})).expect("B"));
    s.execute("select.set", json!({"ids": [b]})).expect("select B");
    s.execute("pi.fill", json!({"color": "#00ff00"})).expect("fill B");
    s.execute("select.set", json!({"ids": [a]})).expect("select A");
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(W, HH)).with_pixels_per_point(1.0).with_max_steps(20).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    (h, a, b)
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn ev(h: &mut H, e: Event) {
    h.event(e);
    h.run_steps(1);
}

fn press(h: &mut H, p: Pos2, down: bool) {
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: down, modifiers: Modifiers::NONE });
}

fn click(h: &mut H, p: Pos2) {
    ev(h, Event::PointerMoved(p));
    press(h, p, true);
    press(h, p, false);
    h.run_steps(2);
}

fn key(h: &mut H, k: Key, m: Modifiers) {
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: m });
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: false, repeat: false, modifiers: m });
    h.run_steps(1);
}

fn undo_count(h: &H) -> usize {
    st(h).session.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

fn redo_count(h: &H) -> usize {
    st(h).session.active().map(|d| d.history.redo_count()).unwrap_or(0)
}

fn undo_label(h: &H) -> String {
    st(h).session.active().and_then(|d| d.history.undo_label().map(|s| s.to_string())).unwrap_or_default()
}

fn redo_label(h: &H) -> String {
    st(h).session.active().and_then(|d| d.history.redo_label().map(|s| s.to_string())).unwrap_or_default()
}

/// Labels of all history rows (done and undone).
fn rows(h: &H) -> Vec<String> {
    let s = st(h);
    let d = s.session.active().expect("doc");
    d.history.steps().iter().map(|x| s.session.step_label(x)).collect()
}

fn no_opacity_row(h: &H) {
    let r = rows(h);
    assert!(!r.iter().any(|l| l.contains("Deckkraft") || l.contains("Opacity") || l.contains("setOpacity")), "no stray History row: {r:?}");
}

fn element(h: &H, id: u64) -> std::sync::Arc<fp_model::Element> {
    let d = st(h).session.active().expect("doc");
    fp_engine::docops::element_on_page(&d.doc, d.page, fp_model::ElementId(id)).expect("element")
}

fn opacity(h: &H, id: u64) -> f64 {
    element(h, id).common.opacity.to_percent().round()
}

fn fill(h: &H, id: u64) -> fp_color::Argb {
    fp_engine::docops::path_attrs_of(&element(h, id)).map(|a| a.fill_color).expect("attrs")
}

fn selected(h: &H) -> Vec<u64> {
    st(h).session.active().map(|d| d.selection.elements.iter().map(|e| e.0).collect()).unwrap_or_default()
}

fn preview(h: &H) -> Option<PreviewValue> {
    st(h).inspector_preview.as_ref().map(|p| p.value.clone())
}

fn rect_of(h: &H, id: Id) -> Option<Rect> {
    h.ctx.read_response(id).map(|r| r.rect)
}

fn sliders(h: &H) -> Vec<Rect> {
    h.query_all(egui_kittest::kittest::By::new().role(egui::accesskit::Role::Slider)).map(|n| n.rect()).collect()
}

/// Click on the inspector's opacity arrow (the pop-up stays open) and drag its thumb to `pct`
/// without applying (released inside the slider).
fn open_opacity_and_drag(h: &mut H, pct: f32) {
    let a = rect_of(h, Id::new("pi-opacity").with("arrow")).expect("opacity arrow drawn").center();
    click(h, a);
    let s = sliders(h);
    assert_eq!(s.len(), 1, "pop-up open after a click on the arrow");
    let sr = s[0];
    let tt = sr.top() + VSLIDER_PERCENT_PAD;
    let knob = Pos2::new(sr.center().x, tt + 1.0);
    let to = Pos2::new(sr.center().x, tt + VSLIDER_PERCENT_TRACK * (1.0 - pct / 100.0));
    ev(h, Event::PointerMoved(knob));
    press(h, knob, true);
    for i in 1..=8 {
        ev(h, Event::PointerMoved(knob + (to - knob) * (i as f32 / 8.0)));
    }
    press(h, to, false);
    h.run_steps(2);
    assert_eq!(sliders(h).len(), 1, "still open after releasing the thumb");
    assert_eq!(preview(h), Some(PreviewValue::Opacity(pct as f64)), "pending value previewed");
}

/// After the undo / redo: the pop-up is closed, nothing pending, and later frames and a click
/// beside the former pop-up apply nothing either.
fn assert_discarded(h: &mut H, a: u64, b: u64, undo: usize) {
    assert!(sliders(h).is_empty(), "the pop-up is closed");
    assert_eq!(preview(h), None, "no preview left");
    assert_eq!(opacity(h, a), 100.0, "A keeps its opacity");
    assert_eq!(opacity(h, b), 100.0, "B gets nothing");
    h.run_steps(4);
    click(h, Pos2::new(W / 2.0, HH - 20.0));
    h.run_steps(2);
    assert_eq!(opacity(h, a), 100.0, "nothing applied later to A");
    assert_eq!(opacity(h, b), 100.0, "nothing applied later to B");
    assert_eq!(undo_count(h), undo, "no step added later");
    no_opacity_row(h);
}

#[test]
fn ctrl_z_with_open_opacity_slider_discards_then_undoes() {
    fp_i18n::set_language("de");
    let (mut h, a, b) = harness();
    let (n0, r0, last) = (undo_count(&h), redo_count(&h), undo_label(&h));
    assert_eq!(selected(&h), vec![a]);
    open_opacity_and_drag(&mut h, 40.0);
    key(&mut h, Key::Z, CTRL);
    h.run_steps(2);
    assert_eq!(undo_count(&h), n0 - 1, "the undo ran");
    assert_eq!(redo_count(&h), r0 + 1, "one redo step");
    assert_eq!(redo_label(&h), last, "the undo applied to the previous step");
    assert_discarded(&mut h, a, b, n0 - 1);
}

#[test]
fn ctrl_y_with_open_opacity_slider_discards_then_redoes() {
    fp_i18n::set_language("de");
    let (mut h, a, b) = harness();
    // one undo makes a redo step; the slider then works on what that undo selected
    h.state_mut().state.session.undo().expect("undo");
    h.run_steps(3);
    assert!(redo_count(&h) >= 1, "a step to redo");
    assert!(!selected(&h).is_empty(), "objects selected for the slider");
    let next = redo_label(&h);
    let (n0, r0) = (undo_count(&h), redo_count(&h));
    open_opacity_and_drag(&mut h, 40.0);
    key(&mut h, Key::Y, CTRL);
    h.run_steps(2);
    assert_eq!(undo_count(&h), n0 + 1, "the redo ran");
    assert_eq!(redo_count(&h), r0.saturating_sub(1), "one redo step used");
    assert_eq!(undo_label(&h), next, "the redo applied the next step");
    assert_discarded(&mut h, a, b, n0 + 1);
}

#[test]
fn menu_undo_action_with_open_opacity_slider_discards_first() {
    // Edit ▸ Undo (also the macOS menu bar, whose click does not reach the pop-up) runs the
    // queued command `edit.undo`
    fp_i18n::set_language("de");
    let (mut h, a, b) = harness();
    let (n0, last) = (undo_count(&h), undo_label(&h));
    open_opacity_and_drag(&mut h, 40.0);
    h.state_mut().state.push(Action::Run("edit.undo".into(), json!({})));
    h.run_steps(3);
    assert_eq!(undo_count(&h), n0 - 1, "the undo ran");
    assert_eq!(redo_label(&h), last);
    assert_discarded(&mut h, a, b, n0 - 1);
}

// ------------------------------------------------------------------ Layers panel opacity

/// Text in the right part of the window (the dock): rects of all matches.
fn find_texts(h: &H, label: &str) -> Vec<Rect> {
    fn texts(shape: &egui::epaint::Shape, out: &mut Vec<(String, Rect)>) {
        match shape {
            egui::epaint::Shape::Text(t) => out.push((t.galley.job.text.clone(), Rect::from_min_size(t.pos, t.galley.size()))),
            egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| texts(s, out)),
            _ => {}
        }
    }
    let mut all = Vec::new();
    for cs in &h.output().shapes {
        texts(&cs.shape, &mut all);
    }
    all.into_iter().filter(|(t, r)| t == label && r.center().x > W / 2.0).map(|(_, r)| r).collect()
}

fn bring_tab(h: &mut H, de: &str, en: &str) {
    let tab = find_texts(h, de).into_iter().chain(find_texts(h, en)).max_by(|a, b| a.center().x.total_cmp(&b.center().x)).unwrap_or_else(|| panic!("{de} tab"));
    click(h, tab.center());
    h.run_steps(10);
}

fn layers_arrow(h: &mut H) -> Rect {
    let id = Id::new("layers-opacity").with("arrow");
    if rect_of(h, id).is_none() {
        bring_tab(h, "Ebenen", "Layers");
    }
    rect_of(h, id).expect("Layers panel opacity arrow drawn")
}

/// Press on the arrow and drag the thumb to `frac` of the track without releasing.
fn arrow_drag_to(h: &mut H, arrow: Pos2, frac: f32, track: VTrack) -> Pos2 {
    ev(h, Event::PointerMoved(arrow));
    press(h, arrow, true);
    for i in 1..=4 {
        ev(h, Event::PointerMoved(arrow + egui::vec2(0.0, 4.0 * i as f32)));
    }
    let sr = *sliders(h).first().expect("pop-up slider open");
    let tt = sr.top() + track.pad;
    let from = arrow + egui::vec2(0.0, 16.0);
    let to = Pos2::new(arrow.x, tt + track.len * (1.0 - frac));
    for i in 1..=12 {
        ev(h, Event::PointerMoved(from + (to - from) * (i as f32 / 12.0)));
    }
    h.run_steps(1);
    to
}

#[test]
fn ctrl_z_while_dragging_the_layers_opacity_slider_discards_then_undoes() {
    fp_i18n::set_language("de");
    let (mut h, a, b) = harness();
    let arrow = layers_arrow(&mut h).center();
    let (n0, last) = (undo_count(&h), undo_label(&h));
    let to = arrow_drag_to(&mut h, arrow, 0.4, VTrack::PERCENT);
    assert_eq!(preview(&h), Some(PreviewValue::Opacity(40.0)), "pending value previewed");
    key(&mut h, Key::Z, CTRL);
    h.run_steps(1);
    assert!(sliders(&h).is_empty(), "Ctrl+Z closes the pop-up");
    // the mouse is released only now: the drag no longer applies anything
    for i in 1..=3 {
        ev(&mut h, Event::PointerMoved(to + egui::vec2(0.0, 3.0 * i as f32)));
    }
    press(&mut h, to + egui::vec2(0.0, 9.0), false);
    h.run_steps(2);
    assert_eq!(undo_count(&h), n0 - 1, "the undo ran");
    assert_eq!(redo_label(&h), last, "the undo applied to the previous step");
    assert_discarded(&mut h, a, b, n0 - 1);
}

// ------------------------------------------------------------- colour pop-up alpha

fn swatch(base: (&str, bool)) -> Id {
    Id::new(Id::new(Id::new(base).with("sw")))
}

#[test]
fn ctrl_z_with_open_colour_popup_alpha_discards_then_undoes() {
    fp_i18n::set_language("de");
    let (mut h, a, b) = harness();
    let sw = swatch(("pi-fill", false));
    let (n0, last) = (undo_count(&h), undo_label(&h));
    let (fa, fb) = (fill(&h, a), fill(&h, b));
    // open the pop-up, open its opacity slider, drag to 50 % (released inside the slider)
    let r = rect_of(&h, sw).expect("fill swatch drawn");
    click(&mut h, r.center());
    h.run_steps(1);
    let p = cp::open_rect(&h.ctx, sw).expect("colour pop-up open");
    click(&mut h, p.min + egui::vec2(54.0, 224.5));
    let sr = cp::slider_rect(p.min, h.ctx.content_rect());
    let (top, height) = cp::alpha_track(sr);
    let at = |v: f32| Pos2::new(sr.center().x, top + height * (1.0 - v / 100.0));
    let (from, to) = (at(90.0), at(50.0));
    ev(&mut h, Event::PointerMoved(from));
    press(&mut h, from, true);
    for i in 1..=10 {
        ev(&mut h, Event::PointerMoved(from + (to - from) * (i as f32 / 10.0)));
    }
    press(&mut h, to, false);
    h.run_steps(2);
    assert!(cp::open_rect(&h.ctx, sw).is_some(), "pop-up open with a pending alpha");
    assert!(matches!(preview(&h), Some(PreviewValue::FillColor(_))), "pending alpha previewed: {:?}", preview(&h));
    key(&mut h, Key::Z, CTRL);
    h.run_steps(2);
    assert!(cp::open_rect(&h.ctx, sw).is_none(), "Ctrl+Z closes the colour pop-up");
    assert!(!cp::any_open(&h.ctx), "no colour pop-up open");
    assert_eq!(preview(&h), None, "no preview left");
    assert_eq!(undo_count(&h), n0 - 1, "the undo ran");
    assert_eq!(redo_label(&h), last, "the undo applied to the previous step");
    assert_eq!(fill(&h, a), fa, "A keeps its fill");
    assert_eq!(fill(&h, b).a(), 0xff, "B gets no pending alpha");
    let _ = fb;
    h.run_steps(4);
    assert_eq!(undo_count(&h), n0 - 1, "no step added later");
    assert_eq!(fill(&h, a), fa);
    assert_eq!(fill(&h, b).a(), 0xff);
}

// ------------------------------------------------------------------ History panel

#[test]
fn history_panel_jump_with_open_opacity_slider_discards_first() {
    fp_i18n::set_language("de");
    let (mut h, a, b) = harness();
    bring_tab(&mut h, "Verlauf", "History");
    // the row of the step before the last: its slider column jumps there (one step back)
    let labels = rows(&h);
    let n0 = undo_count(&h);
    assert!(n0 >= 2 && labels.len() >= n0);
    let i = n0 - 2;
    let nth = labels[..i].iter().filter(|l| **l == labels[i]).count();
    let mut found = find_texts(&h, &labels[i]);
    found.sort_by(|x, y| x.top().total_cmp(&y.top()));
    let row = *found.get(nth).expect("history row drawn");
    let last = undo_label(&h);
    open_opacity_and_drag(&mut h, 40.0);
    click(&mut h, Pos2::new(row.left() - 34.0, row.center().y));
    h.run_steps(2);
    assert_eq!(undo_count(&h), n0 - 1, "jumped one step back");
    assert_eq!(redo_label(&h), last, "the last step is the one undone");
    assert_discarded(&mut h, a, b, n0 - 1);
}

// ------------------------------------------------------------------ commit guard

#[test]
fn commit_runs_only_for_the_objects_it_was_started_for() {
    fp_i18n::set_language("de");
    let (mut h, a, b) = harness();
    let started = PreviewTarget::of(st(&h));
    assert_eq!(started.as_ref().map(|t| t.ids.iter().map(|i| i.0).collect::<Vec<_>>()), Some(vec![a]));
    // a pop-up commit queues its command for A; the selection changes to B before it runs
    preview::stamp_commit(started.clone());
    h.state_mut().state.run("element.setOpacity", json!({"opacity": 40}));
    h.state_mut().state.session.execute("select.set", json!({"ids": [b]})).expect("select B");
    let ctx = h.ctx.clone();
    h.state_mut().state.process_actions(&ctx);
    assert_eq!(opacity(&h, b), 100.0, "not applied to B");
    assert_eq!(opacity(&h, a), 100.0, "nor to A");
    // with the same selection it applies
    h.state_mut().state.session.execute("select.set", json!({"ids": [a]})).expect("select A");
    preview::stamp_commit(started);
    h.state_mut().state.run("element.setOpacity", json!({"opacity": 40}));
    h.state_mut().state.process_actions(&ctx);
    assert_eq!(opacity(&h, a), 40.0, "applied to A");
    assert_eq!(opacity(&h, b), 100.0);
    // the stamp goes to one command only: the next one runs whatever is selected
    h.state_mut().state.session.execute("select.set", json!({"ids": [b]})).expect("select B");
    h.state_mut().state.run("element.setOpacity", json!({"opacity": 30}));
    h.state_mut().state.process_actions(&ctx);
    assert_eq!(opacity(&h, b), 30.0, "an ordinary command is not guarded");
}

#[test]
fn undo_in_the_same_batch_drops_a_pop_up_commit() {
    fp_i18n::set_language("de");
    let (mut h, a, b) = harness();
    let n0 = undo_count(&h);
    preview::stamp_commit(PreviewTarget::of(st(&h)));
    h.state_mut().state.run("element.setOpacity", json!({"opacity": 40}));
    h.state_mut().state.run("edit.undo", json!({}));
    let ctx = h.ctx.clone();
    h.state_mut().state.process_actions(&ctx);
    assert_eq!(undo_count(&h), n0 - 1, "only the undo ran");
    assert_eq!(opacity(&h, a), 100.0);
    assert_eq!(opacity(&h, b), 100.0);
    no_opacity_row(&h);
}

// ------------------------------------------------------- click outside the open pop-up

fn canvas_at(h: &H, x: f32, y: f32) -> Pos2 {
    let s = st(h);
    let d = s.session.active().expect("doc");
    let area = s.canvases.get(&d.id).and_then(|c| c.area).expect("canvas");
    let (sx, sy) = d.view.doc_to_screen(fp_geom::Point::new(x, y));
    Pos2::new(area.min.x + sx as f32, area.min.y + sy as f32)
}

fn opacity_rows(h: &H) -> usize {
    rows(h).iter().filter(|l| l.contains("Deckkraft") || l.contains("Opacity")).count()
}

fn any_menu_open(h: &H) -> bool {
    (0..=fp_ui::menu::main_menu().len()).any(|i| egui::Popup::is_id_open(&h.ctx, Id::new(("fp-main-menu", i))))
}

/// RE spec 69 §1.2 f1: a click on object B closes the pop-up, applies the pending value to A
/// (the object it was opened for, one row) and is swallowed: B is not selected. The next click
/// on B selects it as usual.
#[test]
fn click_on_another_object_commits_to_the_original_selection_and_is_swallowed() {
    fp_i18n::set_language("de");
    let (mut h, a, b) = harness();
    let (n0, rows0) = (undo_count(&h), opacity_rows(&h));
    open_opacity_and_drag(&mut h, 40.0);
    let on_b = canvas_at(&h, 320.0, 85.0);
    click(&mut h, on_b);
    h.run_steps(2);
    assert!(sliders(&h).is_empty(), "the pop-up is closed");
    assert_eq!(preview(&h), None, "no preview left");
    assert_eq!(opacity(&h, a), 40.0, "applied to A");
    assert_eq!(opacity(&h, b), 100.0, "not to B");
    assert_eq!(selected(&h), vec![a], "the click was swallowed: B not selected");
    assert_eq!(undo_count(&h), n0 + 1, "one step");
    assert_eq!(opacity_rows(&h), rows0 + 1, "one opacity row: {:?}", rows(&h));
    // the next click works again
    click(&mut h, on_b);
    h.run_steps(2);
    assert_eq!(selected(&h), vec![b], "a second click selects B");
    assert_eq!(opacity(&h, b), 100.0);
    assert_eq!(undo_count(&h), n0 + 1, "no further step");
}

/// RE spec 69 §1.2 d1: a click on a menu title closes the pop-up and applies the pending value;
/// the menu opens only on the second click.
#[test]
fn click_on_the_menu_bar_commits_and_the_menu_opens_only_on_the_second_click() {
    fp_i18n::set_language("de");
    let (mut h, a, b) = harness();
    let n0 = undo_count(&h);
    open_opacity_and_drag(&mut h, 40.0);
    let titles: Vec<Rect> = h.ctx.data(|d| d.get_temp(Id::new(fp_ui::menu::MENU_TITLES_ID))).expect("menu titles");
    let edit = titles[1].center();
    click(&mut h, edit);
    h.run_steps(2);
    assert!(sliders(&h).is_empty(), "the pop-up is closed");
    assert_eq!(opacity(&h, a), 40.0, "applied to A");
    assert_eq!(opacity(&h, b), 100.0);
    assert_eq!(undo_count(&h), n0 + 1, "one step");
    assert!(!any_menu_open(&h), "the first click opens no menu");
    click(&mut h, edit);
    h.run_steps(2);
    assert!(any_menu_open(&h), "the second click opens the Edit menu");
    key(&mut h, Key::Escape, Modifiers::NONE);
    key(&mut h, Key::Escape, Modifiers::NONE);
}

/// A click beside the open pop-up on empty canvas still applies the value (TW-52) and keeps the
/// selection; Esc instead discards it.
#[test]
fn click_on_empty_canvas_commits_and_esc_discards() {
    fp_i18n::set_language("de");
    let (mut h, a, _b) = harness();
    let n0 = undo_count(&h);
    open_opacity_and_drag(&mut h, 40.0);
    let empty = canvas_at(&h, 220.0, 250.0);
    click(&mut h, empty);
    h.run_steps(2);
    assert_eq!(opacity(&h, a), 40.0);
    assert_eq!(selected(&h), vec![a], "selection kept");
    assert_eq!(undo_count(&h), n0 + 1);
    open_opacity_and_drag(&mut h, 70.0);
    key(&mut h, Key::Escape, Modifiers::NONE);
    h.run_steps(2);
    assert!(sliders(&h).is_empty());
    assert_eq!(opacity(&h, a), 40.0, "Esc discards");
    assert_eq!(undo_count(&h), n0 + 1);
}
