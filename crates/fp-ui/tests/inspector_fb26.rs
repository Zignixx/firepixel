//! FB-26 / TW-204 (re 60 section 3): the paragraph indent typed into the Properties inspector
//! (value + Enter) or set with its pop-up slider moves the first line of every paragraph of the
//! pointer-selected point text on the canvas; the committed value is not swallowed by the live
//! preview (the canvas shows the document again with the new indent).
//!
//! Realistic input like egui-winit on Windows (one event per frame, typing as `Event::Text`).

use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fp_ui::FirepixelApp;
use fp_ui::app_state::{AppState, UiSettings};
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

fn press(h: &mut H, p: Pos2, down: bool) {
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: down, modifiers: Modifiers::NONE });
}

fn click(h: &mut H, p: Pos2) {
    ev(h, Event::PointerMoved(p));
    press(h, p, true);
    press(h, p, false);
    h.run_steps(2);
}

fn undo_count(h: &H) -> usize {
    st(h).session.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

fn doc(h: &H) -> Arc<fp_model::Document> {
    Arc::clone(&st(h).session.active().expect("doc").doc)
}

fn indent_of(h: &H, id: u64) -> f32 {
    let e = fp_engine::docops::element_on_page(&doc(h), 0, fp_model::ElementId(id)).expect("element");
    let fp_model::element::ElementKind::Text(t) = &e.kind else { panic!("not a text") };
    t.runs.first().and_then(|r| r.attrs.indent).or(t.initial.indent).expect("indent")
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

fn field_num(h: &H, f: egui::Rect) -> f64 {
    let s = inputs(h).into_iter().find(|(r, _)| (r.center().y - f.center().y).abs() < 1.0 && (r.left() - f.left()).abs() < 1.0).map(|x| x.1).unwrap_or_default();
    s.parse().unwrap_or_else(|_| panic!("number in the field: {s:?}"))
}

/// Key of the preview in the image shown on the canvas (0 = the document itself).
fn shown_preview_key(h: &H) -> u64 {
    let s = st(h);
    let id = s.session.active().expect("doc").id;
    s.canvases.get(&id).and_then(|c| c.shown).map(|k| k.preview).unwrap_or(0)
}

/// Leftmost dark pixel column of each of the three text lines in the rendered document.
fn ink_lefts(h: &H) -> Vec<i32> {
    let d = st(h).session.active().expect("doc");
    let img = fp_engine::render::default_renderer().expect("renderer").render_page(&d.doc, d.page, d.frame).expect("image");
    // baselines 100, 139.8, 179.6 (24 px, leading 120 %, 5 + 6 px between the paragraphs);
    // x-height band above each
    [100, 140, 180]
        .iter()
        .map(|&b| {
            (0..img.width() as i32)
                .find(|&x| (b - 14..b).any(|y| img.get(x, y).is_some_and(|c| c[0] < 128)))
                .expect("ink in the line")
        })
        .collect()
}

/// Three paragraphs as point text (like the tester's "There is something / or / another."),
/// indent 4 and unique spacing values, selected with the Pointer (editor closed).
fn three_paragraphs() -> (H, u64) {
    let mut id = 0;
    let h = harness(|s| {
        let v = s.execute("draw.text", json!({"text": "There is something\ror\ranother.", "anchor": [40, 100], "size": 24, "color": "#000000"})).expect("text");
        id = v.value.get("id").and_then(|x| x.as_u64()).expect("id");
        s.execute("select.set", json!({"ids": [id]})).expect("select");
        s.execute("text.setAttributes", json!({"tracking": 3, "indent": 4, "spaceBefore": 5, "spaceAfter": 6})).expect("attributes");
    });
    (h, id)
}

#[test]
fn typed_indent_moves_every_first_line_on_the_canvas() {
    let (mut h, id) = three_paragraphs();
    let x0 = ink_lefts(&h);
    let f = input_with(&h, "4");
    let n0 = undo_count(&h);
    click(&mut h, f.center());
    key(&mut h, Key::End, Modifiers::NONE);
    key(&mut h, Key::Home, Modifiers::SHIFT);
    for c in "33".chars() {
        ev(&mut h, Event::Text(c.to_string()));
    }
    key(&mut h, Key::Enter, Modifiers::NONE);
    h.run_steps(2);
    assert_eq!(undo_count(&h), n0 + 1, "Enter applies one row");
    assert_eq!(indent_of(&h, id), 33.0);
    assert_eq!(field_num(&h, f), 33.0, "the field shows the value");
    assert_eq!(shown_preview_key(&h), 0, "the canvas shows the document");
    let x1 = ink_lefts(&h);
    for (a, b) in x1.iter().zip(&x0) {
        assert!((a - b - 29).abs() <= 1, "each paragraph moves by 33 - 4: {x0:?} -> {x1:?}");
    }
}

#[test]
fn slider_indent_moves_every_first_line_on_the_canvas() {
    let (mut h, id) = three_paragraphs();
    let x0 = ink_lefts(&h);
    let f = input_with(&h, "4");
    let n0 = undo_count(&h);
    // press on the arrow right of the field and drag the pop-up thumb up
    let a = Pos2::new(f.right() + 3.0 + 1.5 + 6.75, f.center().y);
    ev(&mut h, Event::PointerMoved(a));
    press(&mut h, a, true);
    let to = Pos2::new(a.x, a.y - 70.0);
    for i in 1..=10 {
        ev(&mut h, Event::PointerMoved(a + (to - a) * (i as f32 / 10.0)));
    }
    h.run_steps(1);
    let v = field_num(&h, f);
    press(&mut h, to, false);
    h.run_steps(2);
    assert_ne!(v, 4.0, "the drag changed the value");
    assert_eq!(undo_count(&h), n0 + 1, "release applies one row");
    assert_eq!(indent_of(&h, id), v as f32);
    assert_eq!(st(&h).inspector_preview.as_ref().map(|p| p.value.clone()), None, "preview gone");
    assert_eq!(shown_preview_key(&h), 0, "the canvas shows the document");
    let x1 = ink_lefts(&h);
    let dx = (v - 4.0).round() as i32;
    for (a, b) in x1.iter().zip(&x0) {
        assert!((a - b - dx).abs() <= 1, "each paragraph moves by {dx}: {x0:?} -> {x1:?}");
    }
}
