//! FB-21 / FB-22 / FB-17 (tester feedback, TW-200, TW-201): live canvas preview for more
//! Properties inspector pop-up sliders, the slider pop-up anchored to its arrow, View > Tooltips
//! off by default.
//! - Fill edge feather, stroke edge softness, the text fields font size, tracking, leading,
//!   indent, space before/after and horizontal scale: while the thumb moves the canvas shows the
//!   value (a preview copy of the document, no History row, document untouched); release applies
//!   exactly one row; Enter in the field applies one row; Esc discards (as for opacity, FB-09).
//! - The vertical pop-up slider opens at a fixed place: centred on the arrow, the thumb level with
//!   the arrow, moved (never flipped) into the window; the same rect on its first and second frame.
//!
//! Realistic input like egui-winit on Windows (one event per frame, typing as `Event::Text`).

use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fp_ui::FirepixelApp;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::inspector::fields::{VSLIDER_PERCENT_PAD, VSLIDER_PERCENT_TRACK, VSLIDER_POPUP, VTrack};
use fp_ui::inspector::preview::PreviewValue;
use serde_json::{Value, json};
use std::sync::Arc;

type H = Harness<'static, FirepixelApp>;

const W: f32 = 1800.0;
const HH: f32 = 1130.0;

fn harness(setup: impl FnOnce(&mut fp_engine::Session)) -> H {
    let mut s = fp_engine::Session::default();
    s.execute(
        "file.new",
        json!({"width": 500, "height": 300, "background": "#ffffff"}),
    )
    .expect("doc");
    setup(&mut s);
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

fn ev(h: &mut H, e: Event) {
    h.event(e);
    h.run_steps(1);
}

fn key(h: &mut H, k: Key, m: Modifiers) {
    ev(
        h,
        Event::Key {
            key: k,
            physical_key: Some(k),
            pressed: true,
            repeat: false,
            modifiers: m,
        },
    );
    ev(
        h,
        Event::Key {
            key: k,
            physical_key: Some(k),
            pressed: false,
            repeat: false,
            modifiers: m,
        },
    );
}

fn text(h: &mut H, t: &str) {
    for c in t.chars() {
        ev(h, Event::Text(c.to_string()));
    }
}

fn press(h: &mut H, p: Pos2, down: bool) {
    ev(
        h,
        Event::PointerButton {
            pos: p,
            button: PointerButton::Primary,
            pressed: down,
            modifiers: Modifiers::NONE,
        },
    );
}

fn click(h: &mut H, p: Pos2) {
    ev(h, Event::PointerMoved(p));
    press(h, p, true);
    press(h, p, false);
    h.run_steps(2);
}

fn undo_count(h: &H) -> usize {
    st(h)
        .session
        .active()
        .map(|d| d.history.undo_count())
        .unwrap_or(0)
}

fn dirty(h: &H) -> bool {
    st(h).session.active().is_some_and(|d| d.is_dirty())
}

fn doc(h: &H) -> Arc<fp_model::Document> {
    Arc::clone(&st(h).session.active().expect("doc").doc)
}

fn element_in(d: &fp_model::Document, id: u64) -> Arc<fp_model::Element> {
    fp_engine::docops::element_on_page(d, 0, fp_model::ElementId(id)).expect("element")
}

fn element(h: &H, id: u64) -> Arc<fp_model::Element> {
    element_in(&doc(h), id)
}

fn feather_of(e: &fp_model::Element) -> (i32, i32) {
    fp_engine::docops::path_attrs_of(e)
        .and_then(|a| a.fill.as_ref().map(|f| (f.edge, f.feather)))
        .expect("fill")
}

fn softness_of(e: &fp_model::Element) -> i32 {
    fp_engine::docops::path_attrs_of(e)
        .and_then(|a| a.brush.as_ref().map(|b| b.softness))
        .expect("brush")
}

/// A text attribute of the first run (or the initial attributes).
fn text_attr(e: &fp_model::Element, f: impl Fn(&fp_model::text::TextAttrs) -> Option<f32>) -> f32 {
    let fp_model::element::ElementKind::Text(t) = &e.kind else {
        panic!("not a text")
    };
    t.runs
        .first()
        .and_then(|r| f(&r.attrs))
        .or_else(|| f(&t.initial))
        .expect("attribute")
}

fn inputs(h: &H) -> Vec<(egui::Rect, String)> {
    let mut v: Vec<(egui::Rect, String)> = h
        .get_all(egui_kittest::kittest::By::new().role(egui::accesskit::Role::TextInput))
        .map(|n| (n.rect(), n.value().unwrap_or_default()))
        .filter(|(r, _)| r.top() > HH - 160.0)
        .collect();
    v.sort_by(|a, b| {
        a.0.top()
            .total_cmp(&b.0.top())
            .then(a.0.left().total_cmp(&b.0.left()))
    });
    v
}

fn input_with(h: &H, value: &str) -> egui::Rect {
    let all = inputs(h);
    let m: Vec<_> = all.iter().filter(|(_, v)| v == value).collect();
    assert_eq!(m.len(), 1, "exactly one field showing \"{value}\": {all:?}");
    m[0].0
}

fn field_at(h: &H, f: egui::Rect) -> String {
    inputs(h)
        .into_iter()
        .find(|(r, _)| {
            (r.center().y - f.center().y).abs() < 1.0 && (r.left() - f.left()).abs() < 1.0
        })
        .map(|x| x.1)
        .unwrap_or_default()
}

fn field_num(h: &H, f: egui::Rect) -> f64 {
    let s = field_at(h, f);
    s.trim_end_matches('%')
        .parse()
        .unwrap_or_else(|_| panic!("number in the field: {s:?}"))
}

fn sliders(h: &H) -> Vec<egui::Rect> {
    h.query_all(egui_kittest::kittest::By::new().role(egui::accesskit::Role::Slider))
        .map(|n| n.rect())
        .collect()
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
    s.canvases
        .get(&id)
        .and_then(|c| c.shown)
        .map(|k| k.preview)
        .unwrap_or(0)
}

/// The preview copy the canvas renders right now.
fn preview_doc(h: &H) -> fp_model::Document {
    let p = st(h).inspector_preview.clone().expect("preview");
    p.apply(&doc(h))
}

/// Press on the arrow of `field` and drag `dy` px (negative = up) without releasing.
fn drag_arrow(h: &mut H, field: egui::Rect, dy: f32) -> Pos2 {
    let a = arrow_of(field);
    ev(h, Event::PointerMoved(a));
    press(h, a, true);
    let to = Pos2::new(a.x, a.y + dy);
    for i in 1..=10 {
        ev(h, Event::PointerMoved(a + (to - a) * (i as f32 / 10.0)));
    }
    h.run_steps(1);
    to
}

/// Drag from the arrow: the field follows, the canvas previews, nothing is applied; release
/// applies exactly one History row. Returns the dragged value.
fn drag_previews_then_applies(
    h: &mut H,
    field: egui::Rect,
    dy: f32,
    expect: impl Fn(f64) -> PreviewValue,
) -> f64 {
    let (n0, d0, dirty0) = (undo_count(h), doc(h), dirty(h));
    let to = drag_arrow(h, field, dy);
    assert_eq!(sliders(h).len(), 1, "pop-up open while dragging");
    let live = field_num(h, field);
    assert_eq!(preview(h), Some(expect(live)), "live preview of {live}");
    assert_ne!(
        shown_preview_key(h),
        0,
        "canvas image rendered from the preview"
    );
    assert!(
        Arc::ptr_eq(&doc(h), &d0),
        "document untouched while dragging"
    );
    assert_eq!(undo_count(h), n0, "no History row while dragging");
    assert_eq!(dirty(h), dirty0, "dirty flag untouched while dragging");
    press(h, to, false);
    h.run_steps(2);
    assert!(sliders(h).is_empty(), "release closes the pop-up");
    assert_eq!(undo_count(h), n0 + 1, "exactly one History row");
    assert_eq!(preview(h), None, "preview gone");
    assert_eq!(shown_preview_key(h), 0, "canvas shows the document again");
    live
}

/// Typed value + Enter applies one row.
fn enter_applies(h: &mut H, field: egui::Rect, typed: &str) {
    let n0 = undo_count(h);
    type_into(h, field, typed);
    key(h, Key::Enter, Modifiers::NONE);
    h.run_steps(2);
    assert_eq!(undo_count(h), n0 + 1, "Enter applies one row");
}

/// Red rectangle with a 7 px stroke of 33 % softness and a feathered fill edge (23), selected.
fn rect_feathered() -> (H, u64) {
    let mut id = 0;
    let h = harness(|s| {
        let v = s
            .execute(
                "draw.rectangle",
                json!({"rect": [40, 40, 160, 130], "fillColor": "#cc3333", "strokeWidth": 0}),
            )
            .expect("rect");
        id = v.value.get("id").and_then(|x| x.as_u64()).expect("id");
        s.execute("select.set", json!({"ids": [id]}))
            .expect("select");
        s.execute(
            "pi.stroke",
            json!({"width": 7, "color": "#000000", "softness": 33}),
        )
        .expect("stroke");
        s.execute("pi.fill", json!({"edge": 2, "feather": 23}))
            .expect("feather");
    });
    (h, id)
}

/// Text "Hallo" with unique field values, selected (editor closed).
fn text_object() -> (H, u64) {
    let mut id = 0;
    let h = harness(|s| {
        let v = s
            .execute(
                "draw.text",
                json!({"text": "Hallo", "anchor": [40, 150], "size": 24}),
            )
            .expect("text");
        id = v.value.get("id").and_then(|x| x.as_u64()).expect("id");
        s.execute("select.set", json!({"ids": [id]}))
            .expect("select");
        s.execute("text.setAttributes", json!({"tracking": 3, "leading": 1.17, "leadingMode": 0, "indent": 4, "spaceBefore": 5, "spaceAfter": 6, "horizontalScale": 1.1})).expect("attributes");
    });
    (h, id)
}

#[test]
fn feather_slider_previews_live_and_applies_once() {
    let (mut h, r) = rect_feathered();
    let f = input_with(&h, "23");
    let v = drag_previews_then_applies(&mut h, f, -40.0, PreviewValue::Feather);
    assert!(v > 23.0, "dragged up: {v}");
    assert_eq!(
        feather_of(&element(&h, r)),
        (2, v as i32),
        "release applies"
    );
    // the preview copy carries the value while the thumb moves (checked on a second drag)
    let f = input_with(&h, &format!("{v}"));
    let to = drag_arrow(&mut h, f, 30.0);
    let live = field_num(&h, f);
    assert_eq!(
        feather_of(&element_in(&preview_doc(&h), r)),
        (2, live as i32),
        "canvas copy"
    );
    assert_eq!(
        feather_of(&element(&h, r)),
        (2, v as i32),
        "document still at the old value"
    );
    // Esc discards the dragged value
    let n0 = undo_count(&h);
    // Esc while the thumb is still dragged from the arrow (egui ends the drag on Esc: that must not
    // count as a release)
    key(&mut h, Key::Escape, Modifiers::NONE);
    assert!(sliders(&h).is_empty(), "Esc closes the pop-up");
    press(&mut h, to, false);
    h.run_steps(2);
    assert_eq!(preview(&h), None, "Esc discards the preview");
    assert_eq!(undo_count(&h), n0, "Esc applies nothing");
    assert_eq!(feather_of(&element(&h, r)), (2, v as i32));
    let f = input_with(&h, &format!("{v}"));
    enter_applies(&mut h, f, "12");
    assert_eq!(feather_of(&element(&h, r)), (2, 12), "Enter applies");
}

#[test]
fn edge_softness_slider_previews_live_and_applies_once() {
    let (mut h, r) = rect_feathered();
    let f = input_with(&h, "33");
    let v = drag_previews_then_applies(&mut h, f, -40.0, PreviewValue::StrokeSoftness);
    assert!(v > 33.0, "dragged up: {v}");
    assert_eq!(
        softness_of(&element(&h, r)),
        v as i32 * 10,
        "release applies"
    );
    let f = input_with(&h, &format!("{v}"));
    enter_applies(&mut h, f, "8");
    assert_eq!(softness_of(&element(&h, r)), 80, "Enter applies");
}

#[test]
fn font_size_slider_previews_live_and_applies_once() {
    let (mut h, r) = text_object();
    let f = input_with(&h, "24");
    let to = drag_arrow(&mut h, f, -50.0);
    let live = field_num(&h, f);
    assert!(live > 24.0, "dragged up: {live}");
    assert_eq!(preview(&h), Some(PreviewValue::Text(json!({"size": live}))));
    assert_eq!(
        text_attr(&element_in(&preview_doc(&h), r), |a| a.size),
        live as f32,
        "canvas copy"
    );
    assert_eq!(
        text_attr(&element(&h, r), |a| a.size),
        24.0,
        "document untouched"
    );
    let n0 = undo_count(&h);
    press(&mut h, to, false);
    h.run_steps(2);
    assert_eq!(undo_count(&h), n0 + 1, "one row");
    assert_eq!(
        text_attr(&element(&h, r), |a| a.size),
        live as f32,
        "release applies"
    );
    assert_eq!(preview(&h), None);
    let f = input_with(&h, &format!("{live}"));
    enter_applies(&mut h, f, "30");
    assert_eq!(
        text_attr(&element(&h, r), |a| a.size),
        30.0,
        "Enter applies"
    );
}

#[test]
fn tracking_slider_previews_live_and_applies_once() {
    let (mut h, r) = text_object();
    let f = input_with(&h, "3");
    let v = drag_previews_then_applies(&mut h, f, -40.0, |x| {
        PreviewValue::Text(json!({"tracking": x}))
    });
    assert!(v > 3.0, "dragged up: {v}");
    assert_eq!(
        text_attr(&element(&h, r), |a| a.tracking),
        v as f32,
        "release applies"
    );
    let f = input_with(&h, &format!("{v}"));
    enter_applies(&mut h, f, "-7");
    assert_eq!(
        text_attr(&element(&h, r), |a| a.tracking),
        -7.0,
        "Enter applies"
    );
}

#[test]
fn horizontal_scale_slider_previews_live_and_applies_once() {
    let (mut h, r) = text_object();
    let f = input_with(&h, "110%");
    let v = drag_previews_then_applies(&mut h, f, -40.0, |x| {
        PreviewValue::Text(json!({"horizontalScale": x / 100.0}))
    });
    assert!(v > 110.0, "dragged up: {v}");
    assert!(
        (text_attr(&element(&h, r), |a| a.horizontal_scale) - (v / 100.0) as f32).abs() < 1e-6,
        "release applies"
    );
    let f = input_with(&h, &format!("{v}%"));
    enter_applies(&mut h, f, "150");
    assert!(
        (text_attr(&element(&h, r), |a| a.horizontal_scale) - 1.5).abs() < 1e-6,
        "Enter applies"
    );
}

/// Reads one optional paragraph attribute.
type ParagraphAttr = fn(&fp_model::text::TextAttrs) -> Option<f32>;

#[test]
fn paragraph_sliders_preview_live() {
    let (mut h, r) = text_object();
    let cases: [(&str, &str, ParagraphAttr, f64); 4] = [
        ("117", "leading", |a| a.leading, 100.0),
        ("4", "indent", |a| a.indent, 1.0),
        ("5", "spaceBefore", |a| a.space_before, 1.0),
        ("6", "spaceAfter", |a| a.space_after, 1.0),
    ];
    for (shown, name, get, div) in cases {
        let f = input_with(&h, shown);
        let v = drag_previews_then_applies(&mut h, f, -70.0, |x| {
            let mut p = json!({ name: x / div });
            if name == "leading" {
                p["leadingMode"] = Value::from(0);
            }
            PreviewValue::Text(p)
        });
        // (near the window bottom the pop-up is moved up, so the value need not grow)
        assert_ne!(
            v,
            shown.parse::<f64>().expect("number"),
            "{name}: the drag changed the value"
        );
        assert!(
            (text_attr(&element(&h, r), get) - (v / div) as f32).abs() < 1e-5,
            "{name}: release applies"
        );
    }
}

/// FB-21c (RE spec 60 §2e): the pop-up opens flush left with its arrow, the thumb of the current
/// value under the press point (pressing lower opens it lower), moved into the window when it would
/// overflow (never flipped above or beside the arrow), and keeps that rect from its first visible
/// frame on. (egui shows a new pop-up first in an invisible sizing pass at a provisional place; the
/// first painted frame is the next one.)
#[test]
fn slider_popup_is_anchored_to_its_arrow() {
    for opacity in [100.0, 30.0] {
        let (mut h, _) = rect_feathered();
        h.state_mut()
            .state
            .session
            .execute("element.setOpacity", json!({"opacity": opacity}))
            .expect("opacity");
        h.run_steps(3);
        let f = input_with(&h, &format!("{opacity}"));
        let a = arrow_of(f);
        let id = fp_ui::inspector::fields::slider_popup_id("pi-opacity");
        let area = |h: &H| h.ctx.memory(|m| m.area_rect(id));
        ev(&mut h, Event::PointerMoved(a));
        press(&mut h, a, true);
        press(&mut h, a, false);
        assert!(
            area(&h).is_some(),
            "{opacity}: pop-up opened by the click (sizing pass)"
        );
        h.run_steps(1);
        let first = area(&h).expect("first painted frame");
        h.run_steps(1);
        let second = area(&h).expect("pop-up still open");
        h.run_steps(3);
        let later = area(&h).expect("pop-up still open");
        assert_eq!(
            first, second,
            "{opacity}: same rect on the first and second painted frame"
        );
        assert_eq!(second, later, "{opacity}: and later");
        assert_eq!(sliders(&h).len(), 1, "{opacity}: one pop-up slider");
        // the place computed for the arrow (the arrow rect is 9 DLU = 13.5 px wide)
        let arrow_rect = egui::Rect::from_center_size(a, egui::vec2(13.5, 19.5));
        let expected = fp_ui::inspector::fields::vslider_popup_pos_on(
            &h.ctx,
            arrow_rect,
            a.y,
            opacity as f32 / 100.0,
            VTrack::PERCENT,
        );
        assert!(
            (first.min - expected).length() <= 1.0,
            "{opacity}: at the computed place {first:?} / {expected:?}"
        );
        assert!(
            (first.size() - VSLIDER_POPUP).length() < 0.5,
            "{opacity}: size {:?}",
            first.size()
        );
        let screen = h.ctx.content_rect();
        assert!(
            screen.contains_rect(first),
            "{opacity}: inside the window {first:?}"
        );
        assert!(
            (first.left() - arrow_rect.left()).abs() <= 1.0,
            "{opacity}: flush left with the arrow {first:?} / {arrow_rect:?}"
        );
        let thumb = first.top()
            + 2.0
            + VSLIDER_PERCENT_PAD
            + (1.0 - opacity as f32 / 100.0) * VSLIDER_PERCENT_TRACK;
        if opacity == 100.0 {
            // the thumb would be level with the arrow, the pop-up would leave the window at the
            // bottom: it is moved up just enough (before: flipped entirely above the arrow)
            assert!(
                (first.bottom() - screen.bottom()).abs() <= 1.0,
                "moved up to the window edge: {first:?}"
            );
            assert!(
                first.top() < a.y && first.bottom() > a.y,
                "the arrow stays beside the pop-up: {first:?}"
            );
        } else {
            assert!(
                (thumb - a.y).abs() <= 1.0,
                "{opacity}: thumb {thumb} level with the arrow {}",
                a.y
            );
        }
        key(&mut h, Key::Escape, Modifiers::NONE);
        h.run_steps(2);
        assert!(sliders(&h).is_empty());
        if opacity == 30.0 {
            // pressing 6 px lower on the arrow opens the pop-up 6 px lower (thumb under the press)
            let low = a + egui::vec2(0.0, 6.0);
            ev(&mut h, Event::PointerMoved(low));
            press(&mut h, low, true);
            press(&mut h, low, false);
            h.run_steps(2);
            let lower = area(&h).expect("pop-up open");
            assert!(
                (lower.top() - first.top() - 6.0).abs() <= 1.0,
                "follows the press point: {lower:?} / {first:?}"
            );
            key(&mut h, Key::Escape, Modifiers::NONE);
            h.run_steps(2);
            assert!(sliders(&h).is_empty());
            // press on the arrow and drag sideways: the thumb sits at the pointer, so the value
            // stays 30 in every frame (no flash of a value from the provisional sizing-pass place)
            ev(&mut h, Event::PointerMoved(a));
            press(&mut h, a, true);
            let mut seen = Vec::new();
            for i in 1..=8 {
                ev(
                    &mut h,
                    Event::PointerMoved(a + egui::vec2(2.0 * i as f32, 0.0)),
                );
                seen.push((field_at(&h, f), preview(&h)));
            }
            assert_eq!(sliders(&h).len(), 1, "drag opened the pop-up");
            assert!(
                seen.iter().all(|(t, p)| t == "30"
                    && p.as_ref().is_none_or(|p| *p == PreviewValue::Opacity(30.0))),
                "{seen:?}"
            );
            press(&mut h, a + egui::vec2(16.0, 0.0), false);
            h.run_steps(2);
        }
    }
}
