//! FB-09 / FB-10 (tester feedback): opacity and stroke width in the Properties inspector.
//! Pop-up slider: press on the arrow and drag applies on release; a click on the arrow keeps the
//! pop-up open and applies when it closes (click beside it), Esc discards. Every apply is exactly
//! one history step. While the thumb moves, the canvas shows a live preview of the value without a
//! history step and without dirtying the document (Firepixel enhancement). Number field: Enter,
//! Tab and a click into the document apply; a click on empty inspector space keeps the focus and
//! applies nothing; unchanged values create no step; values are clamped. Realistic input like
//! egui-winit on Windows (one event per frame, typing as `Event::Text`).

use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fp_ui::FirepixelApp;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::inspector::preview::PreviewValue;
use serde_json::json;
use std::sync::Arc;

type H = Harness<'static, FirepixelApp>;

const W: f32 = 1800.0;
const HH: f32 = 1130.0;

fn harness(setup: impl FnOnce(&mut fp_engine::Session)) -> H {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 500, "height": 300, "background": "#ffffff"})).expect("doc");
    setup(&mut s);
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(W, HH)).with_pixels_per_point(1.0).with_max_steps(20).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    h
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn ev(h: &mut H, e: Event) {
    h.event(e);
    h.run_steps(1);
}

fn key(h: &mut H, k: Key, m: Modifiers) {
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: m });
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: false, repeat: false, modifiers: m });
}

fn text(h: &mut H, t: &str) {
    for c in t.chars() {
        ev(h, Event::Text(c.to_string()));
    }
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

fn screen(h: &H, x: f32, y: f32) -> Pos2 {
    let s = st(h);
    let d = s.session.active().expect("doc");
    let area = s.canvases.get(&d.id).and_then(|c| c.area).expect("canvas");
    let (sx, sy) = d.view.doc_to_screen(fp_geom::Point::new(x, y));
    Pos2::new(area.min.x + sx as f32, area.min.y + sy as f32)
}

fn undo_count(h: &H) -> usize {
    st(h).session.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

fn dirty(h: &H) -> bool {
    st(h).session.active().is_some_and(|d| d.is_dirty())
}

fn doc(h: &H) -> Arc<fp_model::Document> {
    Arc::clone(&st(h).session.active().expect("doc").doc)
}

fn element(h: &H, id: u64) -> Arc<fp_model::Element> {
    let d = st(h).session.active().expect("doc");
    fp_engine::docops::element_on_page(&d.doc, d.page, fp_model::ElementId(id)).expect("element")
}

fn opacity(h: &H, id: u64) -> f64 {
    element(h, id).common.opacity.to_percent().round()
}

fn stroke_width(h: &H, id: u64) -> i32 {
    fp_engine::docops::path_attrs_of(&element(h, id)).and_then(|a| a.brush.as_ref().map(|b| b.diameter)).expect("brush")
}

fn inputs(h: &H) -> Vec<(egui::Rect, String)> {
    let mut v: Vec<(egui::Rect, String)> = h.get_all(egui_kittest::kittest::By::new().role(egui::accesskit::Role::TextInput)).map(|n| (n.rect(), n.value().unwrap_or_default())).filter(|(r, _)| r.top() > HH - 160.0).collect();
    v.sort_by(|a, b| a.0.top().total_cmp(&b.0.top()).then(a.0.left().total_cmp(&b.0.left())));
    v
}

fn input_with(h: &H, value: &str) -> egui::Rect {
    let all = inputs(h);
    let m: Vec<_> = all.iter().filter(|(_, v)| v == value).collect();
    assert_eq!(m.len(), 1, "exactly one field showing \"{value}\": {all:?}");
    m[0].0
}

fn field_at(h: &H, f: egui::Rect) -> String {
    inputs(h).into_iter().find(|(r, _)| (r.center().y - f.center().y).abs() < 1.0 && (r.left() - f.left()).abs() < 1.0).map(|x| x.1).unwrap_or_default()
}

fn sliders(h: &H) -> Vec<egui::Rect> {
    h.query_all(egui_kittest::kittest::By::new().role(egui::accesskit::Role::Slider)).map(|n| n.rect()).collect()
}

/// Arrow ▾ right of a number field (1 DLU gap, 9 DLU wide).
fn arrow_of(field: egui::Rect) -> Pos2 {
    Pos2::new(field.right() + 3.0 + 1.5 + 6.75, field.center().y)
}

/// Replace the field content like a user: click, End, Shift+Home, type.
fn type_into(h: &mut H, field: egui::Rect, s: &str) {
    click(h, field.center());
    key(h, Key::End, Modifiers::NONE);
    key(h, Key::Home, Modifiers::SHIFT);
    text(h, s);
}

fn preview(h: &H) -> Option<PreviewValue> {
    st(h).inspector_preview.as_ref().map(|p| p.value.clone())
}

/// Key of the preview in the image shown on the canvas (0 = the document itself).
fn shown_preview_key(h: &H) -> u64 {
    let s = st(h);
    let id = s.session.active().expect("doc").id;
    s.canvases.get(&id).and_then(|c| c.shown).map(|k| k.preview).unwrap_or(0)
}

/// Red rectangle 40/40–160/130 with a 7 px black stroke, selected (W 120, H 90).
fn rect_with_stroke() -> (H, u64) {
    let mut id = 0;
    let h = harness(|s| {
        let v = s.execute("draw.rectangle", json!({"rect": [40, 40, 160, 130], "fillColor": "#cc3333", "strokeWidth": 0})).expect("rect");
        id = v.value.get("id").and_then(|x| x.as_u64()).expect("id");
        s.execute("select.set", json!({"ids": [id]})).expect("select");
        s.execute("pi.stroke", json!({"width": 7, "color": "#000000"})).expect("stroke");
    });
    (h, id)
}

/// Press on the arrow and drag without releasing: field live, canvas preview, document untouched;
/// release applies exactly one step "Opacity" and closes the pop-up.
#[test]
fn opacity_press_drag_release_applies_once_with_live_preview() {
    let (mut h, r) = rect_with_stroke();
    let f = input_with(&h, "100");
    let a = arrow_of(f);
    let (n0, d0, dirty0) = (undo_count(&h), doc(&h), dirty(&h));
    ev(&mut h, Event::PointerMoved(a));
    press(&mut h, a, true);
    let track = fp_ui::inspector::fields::VSLIDER_PERCENT_TRACK;
    // start the drag, then find the pop-up (moved up at the window edge) and go to 41
    for i in 1..=4 {
        ev(&mut h, Event::PointerMoved(a + egui::vec2(0.0, 4.0 * i as f32)));
    }
    let sr = sliders(&h)[0];
    let tt = sr.top() + fp_ui::inspector::fields::VSLIDER_PERCENT_PAD;
    let from = a + egui::vec2(0.0, 16.0);
    let to = Pos2::new(a.x, tt + track * 0.59);
    for i in 1..=12 {
        ev(&mut h, Event::PointerMoved(from + (to - from) * (i as f32 / 12.0)));
    }
    h.run_steps(1);
    assert_eq!(sliders(&h).len(), 1, "pop-up open while dragging");
    assert_eq!(field_at(&h, f), "41", "field follows live");
    assert_eq!(preview(&h), Some(PreviewValue::Opacity(41.0)), "live preview on the canvas");
    assert_ne!(shown_preview_key(&h), 0, "canvas image rendered from the preview");
    assert!(Arc::ptr_eq(&doc(&h), &d0), "document untouched while dragging");
    assert_eq!(undo_count(&h), n0, "no history step while dragging");
    assert_eq!(dirty(&h), dirty0, "dirty flag untouched while dragging");
    press(&mut h, to, false);
    h.run_steps(2);
    assert_eq!(opacity(&h, r), 41.0, "release applies");
    assert_eq!(undo_count(&h), n0 + 1, "exactly one step");
    assert!(sliders(&h).is_empty(), "release closes the pop-up");
    assert_eq!(preview(&h), None, "preview gone");
    assert_eq!(shown_preview_key(&h), 0, "canvas shows the document again");
    let label = st(&h).session.active().and_then(|d| d.history.undo_label().map(|s| s.to_string())).unwrap_or_default();
    assert!(label.contains("Deckkraft") || label.contains("Opacity") || label.contains("setOpacity"), "step label {label}");
    h.state_mut().state.session.undo().expect("undo");
    h.run_steps(2);
    assert_eq!(opacity(&h, r), 100.0, "one Ctrl+Z restores");
}

/// Click on the arrow: pop-up stays open; dragging the thumb changes only the field (and the
/// preview); releasing the thumb applies nothing; a click beside it applies one step. Esc discards.
#[test]
fn opacity_click_mode_applies_on_close_and_esc_discards() {
    let (mut h, r) = rect_with_stroke();
    let f = input_with(&h, "100");
    let n0 = undo_count(&h);
    click(&mut h, arrow_of(f));
    let s = sliders(&h);
    assert_eq!(s.len(), 1, "pop-up stays open after a click");
    let sr = s[0];
    let tt = sr.top() + fp_ui::inspector::fields::VSLIDER_PERCENT_PAD;
    let track = fp_ui::inspector::fields::VSLIDER_PERCENT_TRACK;
    let knob = Pos2::new(sr.center().x, tt + 1.0);
    let to = Pos2::new(sr.center().x, tt + track * 0.14);
    ev(&mut h, Event::PointerMoved(knob));
    press(&mut h, knob, true);
    for i in 1..=8 {
        ev(&mut h, Event::PointerMoved(knob + (to - knob) * (i as f32 / 8.0)));
    }
    h.run_steps(1);
    assert_eq!(field_at(&h, f), "86");
    assert_eq!(preview(&h), Some(PreviewValue::Opacity(86.0)), "preview while the thumb moves");
    press(&mut h, to, false);
    h.run_steps(2);
    assert_eq!(sliders(&h).len(), 1, "still open after releasing the thumb");
    assert_eq!(opacity(&h, r), 100.0, "not applied yet");
    assert_eq!(undo_count(&h), n0);
    assert_eq!(preview(&h), Some(PreviewValue::Opacity(86.0)), "pending value stays previewed");
    click(&mut h, Pos2::new(sr.right() + 200.0, sr.top() + 20.0));
    assert!(sliders(&h).is_empty());
    assert_eq!(opacity(&h, r), 86.0, "closing applies");
    assert_eq!(undo_count(&h), n0 + 1, "one step");
    assert_eq!(preview(&h), None);
    // Esc: closes without applying, preview discarded, no step, document identical
    let f = input_with(&h, "86");
    click(&mut h, arrow_of(f));
    let sr = sliders(&h)[0];
    let tt = sr.top() + fp_ui::inspector::fields::VSLIDER_PERCENT_PAD;
    let knob = Pos2::new(sr.center().x, tt + track * 0.14);
    let to = Pos2::new(sr.center().x, tt + track * 0.70);
    ev(&mut h, Event::PointerMoved(knob));
    press(&mut h, knob, true);
    for i in 1..=8 {
        ev(&mut h, Event::PointerMoved(knob + (to - knob) * (i as f32 / 8.0)));
    }
    h.run_steps(1);
    press(&mut h, to, false);
    assert!(matches!(preview(&h), Some(PreviewValue::Opacity(v)) if v < 50.0), "preview {:?}", preview(&h));
    let d1 = doc(&h);
    key(&mut h, Key::Escape, Modifiers::NONE);
    h.run_steps(2);
    assert!(sliders(&h).is_empty(), "Esc closes");
    assert_eq!(preview(&h), None, "Esc discards the preview");
    assert_eq!(shown_preview_key(&h), 0, "canvas back to the document");
    assert!(Arc::ptr_eq(&doc(&h), &d1), "Esc applies nothing");
    assert_eq!(undo_count(&h), n0 + 1);
    // Enter in the open pop-up applies the pending value like a click beside it
    let f = input_with(&h, "86");
    click(&mut h, arrow_of(f));
    let sr = sliders(&h)[0];
    let tt = sr.top() + fp_ui::inspector::fields::VSLIDER_PERCENT_PAD;
    let knob = Pos2::new(sr.center().x, tt + track * 0.14);
    let to = Pos2::new(sr.center().x, tt + track * 0.5);
    ev(&mut h, Event::PointerMoved(knob));
    press(&mut h, knob, true);
    for i in 1..=8 {
        ev(&mut h, Event::PointerMoved(knob + (to - knob) * (i as f32 / 8.0)));
    }
    h.run_steps(1);
    press(&mut h, to, false);
    key(&mut h, Key::Enter, Modifiers::NONE);
    h.run_steps(2);
    assert!(sliders(&h).is_empty(), "Enter closes");
    assert_eq!(opacity(&h, r), 50.0, "Enter applies");
    assert_eq!(undo_count(&h), n0 + 2);
    assert_eq!(st(&h).session.active().map(|d| d.selection.elements.len()), Some(1), "selection kept");
}

/// Number field: typing changes nothing; Enter / Tab / click into the document apply (one step
/// each); click on empty inspector space keeps the focus; unchanged = no step; 305 → 100.
#[test]
fn opacity_field_enter_tab_click_and_clamp() {
    let (mut h, r) = rect_with_stroke();
    let n0 = undo_count(&h);
    let f = input_with(&h, "100");
    type_into(&mut h, f, "45");
    assert_eq!(opacity(&h, r), 100.0, "typing changes nothing");
    key(&mut h, Key::Enter, Modifiers::NONE);
    h.run_steps(2);
    assert_eq!(opacity(&h, r), 45.0, "Enter applies");
    assert_eq!(undo_count(&h), n0 + 1);
    type_into(&mut h, f, "60");
    key(&mut h, Key::Tab, Modifiers::NONE);
    h.run_steps(2);
    assert_eq!(opacity(&h, r), 60.0, "Tab applies");
    assert_eq!(undo_count(&h), n0 + 2);
    // click on empty inspector space: focus stays, nothing applied
    type_into(&mut h, f, "30");
    let mut empty = None;
    for p in [Pos2::new(f.center().x, f.bottom() + 2.0), Pos2::new(f.center().x, f.top() - 3.0), Pos2::new(f.left() - 3.0, f.bottom() + 3.0), Pos2::new(f.center().x, HH - 12.0)] {
        ev(&mut h, Event::PointerMoved(p));
        h.run_steps(1);
        if fp_ui::inspector::fields::pointer_on_empty_panel(&h.ctx) {
            empty = Some(p);
            break;
        }
    }
    click(&mut h, empty.expect("an empty inspector point near the field"));
    assert_eq!(opacity(&h, r), 60.0, "click on empty panel space does not apply");
    assert_eq!(field_at(&h, f), "30", "typed value stays in the field");
    // the focus stayed: Enter applies now
    key(&mut h, Key::Enter, Modifiers::NONE);
    h.run_steps(2);
    assert_eq!(opacity(&h, r), 30.0, "focus kept, Enter applies");
    assert_eq!(undo_count(&h), n0 + 3);
    // click into the document applies
    type_into(&mut h, f, "70");
    let p = screen(&h, 100.0, 90.0);
    click(&mut h, p);
    assert_eq!(opacity(&h, r), 70.0, "click into the document applies");
    assert_eq!(undo_count(&h), n0 + 4);
    // unchanged value: no step
    let f = input_with(&h, "70");
    type_into(&mut h, f, "70");
    key(&mut h, Key::Enter, Modifiers::NONE);
    h.run_steps(2);
    assert_eq!(undo_count(&h), n0 + 4, "unchanged value, no step");
    // 305 → 100 (clamped)
    type_into(&mut h, f, "305");
    key(&mut h, Key::Enter, Modifiers::NONE);
    h.run_steps(2);
    assert_eq!(opacity(&h, r), 100.0, "clamped to 100");
    assert_eq!(undo_count(&h), n0 + 5);
    assert_eq!(field_at(&h, f), "100");
}

/// Stroke width: drag from the arrow previews live and applies one step on release (one Ctrl+Z
/// restores); Enter and Tab in the field apply one step each.
#[test]
fn stroke_width_slider_and_field() {
    let (mut h, r) = rect_with_stroke();
    let n0 = undo_count(&h);
    let f = input_with(&h, "7");
    let a = arrow_of(f);
    let d0 = doc(&h);
    ev(&mut h, Event::PointerMoved(a));
    press(&mut h, a, true);
    let to = Pos2::new(a.x, a.y - 60.0);
    for i in 1..=10 {
        ev(&mut h, Event::PointerMoved(a + (to - a) * (i as f32 / 10.0)));
    }
    h.run_steps(1);
    let live: f64 = field_at(&h, f).parse().expect("number");
    assert!(live > 7.0, "field follows: {live}");
    assert_eq!(preview(&h), Some(PreviewValue::StrokeWidth(live)), "live preview");
    assert!(Arc::ptr_eq(&doc(&h), &d0), "document untouched");
    assert_eq!(undo_count(&h), n0);
    press(&mut h, to, false);
    h.run_steps(2);
    assert_eq!(stroke_width(&h, r), live as i32, "release applies");
    assert_eq!(undo_count(&h), n0 + 1, "one step, not two");
    let label = st(&h).session.active().and_then(|d| d.history.undo_label().map(|s| s.to_string())).unwrap_or_default();
    assert!(label.contains("Pinselstrich") || label.contains("Stroke") || label.contains("pi.stroke"), "step label {label}");
    assert_eq!(preview(&h), None);
    h.state_mut().state.session.undo().expect("undo");
    h.run_steps(2);
    assert_eq!(stroke_width(&h, r), 7, "one Ctrl+Z restores the size");
    let f = input_with(&h, "7");
    type_into(&mut h, f, "12");
    assert_eq!(stroke_width(&h, r), 7, "typing changes nothing");
    key(&mut h, Key::Enter, Modifiers::NONE);
    h.run_steps(2);
    assert_eq!(stroke_width(&h, r), 12, "Enter applies");
    let n1 = undo_count(&h);
    type_into(&mut h, f, "3");
    key(&mut h, Key::Tab, Modifiers::NONE);
    h.run_steps(2);
    assert_eq!(stroke_width(&h, r), 3, "Tab applies");
    assert_eq!(undo_count(&h), n1 + 1, "one step");
}

/// The preview copy changes only the preview's elements; the session document stays as it is.
#[test]
fn preview_apply_changes_only_the_copy() {
    let (h, r) = rect_with_stroke();
    let d0 = doc(&h);
    let p = fp_ui::inspector::preview::InspectorPreview::for_selection(st(&h), PreviewValue::Opacity(25.0)).expect("selection");
    let copy = p.apply(&d0);
    let e = fp_engine::docops::element_on_page(&copy, 0, fp_model::ElementId(r)).expect("element");
    assert_eq!(e.common.opacity.to_percent().round(), 25.0);
    assert_eq!(opacity(&h, r), 100.0, "session document unchanged");
    let p = fp_ui::inspector::preview::InspectorPreview::for_selection(st(&h), PreviewValue::StrokeWidth(30.0)).expect("selection");
    let copy = p.apply(&d0);
    let e = fp_engine::docops::element_on_page(&copy, 0, fp_model::ElementId(r)).expect("element");
    assert_eq!(fp_engine::docops::path_attrs_of(&e).and_then(|a| a.brush.as_ref().map(|b| b.diameter)), Some(30));
    assert_eq!(stroke_width(&h, r), 7);
}
