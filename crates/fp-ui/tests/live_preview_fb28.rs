//! FB-28 / FB-29 (TW-205 / TW-206) and the remaining sliders: live canvas preview without a history
//! step while a value is pending, exactly one step on commit, nothing on Esc.
//! - Layers panel opacity slider (header row "Normal / Opacity"): like the inspector's opacity.
//! - Alpha of the colour pop-up of the inspector's fill and stroke swatch: the pending alpha shows
//!   on the selection while the pop-up is open; Enter applies it (one step), Esc discards it.
//! - Rectangle roundness slider of the inspector.
//!
//! Realistic input like egui-winit (one event per frame).

use egui::{Event, Id, Key, Modifiers, PointerButton, Pos2, Rect};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fp_color::Argb;
use fp_ui::FirepixelApp;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::inspector::color_picker as cp;
use fp_ui::inspector::fields::VTrack;
use fp_ui::inspector::preview::PreviewValue;
use serde_json::json;
use std::sync::Arc;

type H = Harness<'static, FirepixelApp>;

const W: f32 = 1800.0;
const HH: f32 = 1130.0;

/// Red rectangle 40/40–160/130 with a 7 px black stroke, selected. Returns the harness and its id.
fn harness() -> (H, u64) {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 500, "height": 300, "background": "#ffffff"})).expect("doc");
    let v = s.execute("draw.rectangle", json!({"rect": [40, 40, 160, 130], "fillColor": "#ff0000", "strokeWidth": 0})).expect("rect");
    let id = v.value.get("id").and_then(|x| x.as_u64()).expect("id");
    s.execute("select.set", json!({"ids": [id]})).expect("select");
    s.execute("pi.stroke", json!({"width": 7, "color": "#000000"})).expect("stroke");
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(W, HH)).with_pixels_per_point(1.0).with_max_steps(20).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    (h, id)
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

fn key(h: &mut H, k: Key) {
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: Modifiers::NONE });
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: false, repeat: false, modifiers: Modifiers::NONE });
    h.run_steps(1);
}

fn undo_count(h: &H) -> usize {
    st(h).session.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

fn undo_label(h: &H) -> String {
    st(h).session.active().and_then(|d| d.history.undo_label().map(|s| s.to_string())).unwrap_or_default()
}

fn doc(h: &H) -> Arc<fp_model::Document> {
    Arc::clone(&st(h).session.active().expect("doc").doc)
}

fn element(h: &H, id: u64) -> Arc<fp_model::Element> {
    let d = st(h).session.active().expect("doc");
    fp_engine::docops::element_on_page(&d.doc, d.page, fp_model::ElementId(id)).expect("element")
}

fn attrs(h: &H, id: u64) -> fp_model::PathAttrs {
    fp_engine::docops::path_attrs_of(&element(h, id)).map(|a| (*a).clone()).expect("attrs")
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

fn rect_of(h: &H, id: Id) -> Option<Rect> {
    h.ctx.read_response(id).map(|r| r.rect)
}

fn sliders(h: &H) -> Vec<Rect> {
    h.query_all(egui_kittest::kittest::By::new().role(egui::accesskit::Role::Slider)).map(|n| n.rect()).collect()
}

/// Press on the arrow `arrow` of a pop-up slider with the track `track` and drag the thumb to
/// `frac` of the track (0 = bottom, 1 = top) without releasing; returns the pointer position.
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

// ------------------------------------------------------------------ Layers panel opacity (FB-28)

/// Text in the right part of the window (the dock), rightmost match.
fn find_text(h: &H, label: &str) -> Option<Pos2> {
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
    all.iter().filter(|(t, r)| t == label && r.center().x > W / 2.0).map(|(_, r)| r.center()).max_by(|a, b| a.x.total_cmp(&b.x))
}

/// Arrow of the Layers panel's opacity field (the Layers tab is brought to the front if needed).
fn layers_arrow(h: &mut H) -> Rect {
    let id = Id::new("layers-opacity").with("arrow");
    for _ in 0..2 {
        if let Some(r) = rect_of(h, id) {
            return r;
        }
        let tab = find_text(h, "Ebenen").or_else(|| find_text(h, "Layers")).expect("Layers tab");
        click(h, tab);
        h.run_steps(10);
    }
    rect_of(h, id).expect("Layers panel opacity arrow drawn")
}

#[test]
fn layers_opacity_slider_previews_live_and_applies_once() {
    fp_i18n::set_language("de");
    let (mut h, r) = harness();
    let a = layers_arrow(&mut h).center();
    let (n0, d0) = (undo_count(&h), doc(&h));
    let to = arrow_drag_to(&mut h, a, 0.41, VTrack::PERCENT);
    assert_eq!(preview(&h), Some(PreviewValue::Opacity(41.0)), "live preview while dragging");
    assert_ne!(shown_preview_key(&h), 0, "canvas image rendered from the preview");
    assert!(Arc::ptr_eq(&doc(&h), &d0), "document untouched while dragging");
    assert_eq!(undo_count(&h), n0, "no history step while dragging");
    press(&mut h, to, false);
    h.run_steps(2);
    assert_eq!(element(&h, r).common.opacity.to_percent().round(), 41.0, "release applies");
    assert_eq!(undo_count(&h), n0 + 1, "exactly one step");
    assert_eq!(preview(&h), None, "preview gone");
    assert_eq!(shown_preview_key(&h), 0, "canvas shows the document again");
    let label = undo_label(&h);
    assert!(label.contains("Deckkraft") || label.contains("Opacity") || label.contains("setOpacity"), "step label {label}");
}

#[test]
fn layers_opacity_slider_esc_discards() {
    fp_i18n::set_language("de");
    let (mut h, r) = harness();
    let a = layers_arrow(&mut h).center();
    let (n0, d0) = (undo_count(&h), doc(&h));
    let to = arrow_drag_to(&mut h, a, 0.3, VTrack::PERCENT);
    assert_eq!(preview(&h), Some(PreviewValue::Opacity(30.0)));
    key(&mut h, Key::Escape);
    press(&mut h, to, false);
    h.run_steps(2);
    assert_eq!(preview(&h), None, "Esc ends the preview");
    assert!(Arc::ptr_eq(&doc(&h), &d0), "document unchanged");
    assert_eq!(undo_count(&h), n0, "no step");
    assert_eq!(element(&h, r).common.opacity.to_percent().round(), 100.0);
}

// ------------------------------------------------------------- colour pop-up alpha (FB-29)

fn swatch(base: (&str, bool)) -> Id {
    Id::new(Id::new(Id::new(base).with("sw")))
}

/// Opens the colour pop-up of `sw`, opens its opacity slider and drags the thumb to `pct` percent
/// (pointer still held at the end, then released inside the slider). Returns the pop-up origin.
fn drag_alpha(h: &mut H, sw: Id, pct: f32, mut each: impl FnMut(&H)) -> Pos2 {
    let r = rect_of(h, sw).expect("swatch drawn");
    click(h, r.center());
    h.run_steps(1);
    let p = cp::open_rect(&h.ctx, sw).expect("colour pop-up open");
    click(h, p.min + egui::vec2(54.0, 224.5));
    let sr = cp::slider_rect(p.min, h.ctx.content_rect());
    let (top, height) = cp::alpha_track(sr);
    let at = |v: f32| Pos2::new(sr.center().x, top + height * (1.0 - v / 100.0));
    let (a, b) = (at(90.0), at(pct));
    ev(h, Event::PointerMoved(a));
    press(h, a, true);
    for i in 1..=10 {
        ev(h, Event::PointerMoved(a + (b - a) * (i as f32 / 10.0)));
        each(h);
    }
    press(h, b, false);
    h.run_steps(2);
    p.min
}

fn alpha_case(stroke: bool, commit: bool) {
    fp_i18n::set_language("de");
    let (mut h, r) = harness();
    let sw = swatch(if stroke { ("pi-stroke", false) } else { ("pi-fill", false) });
    let before = attrs(&h, r);
    let (n0, d0) = (undo_count(&h), doc(&h));
    let want = if stroke { Argb(0x8000_0000) } else { Argb(0x80ff_0000) };
    let want_preview = if stroke { PreviewValue::StrokeColor(want) } else { PreviewValue::FillColor(want) };
    let mut seen = false;
    drag_alpha(&mut h, sw, 50.0, |h| {
        assert_eq!(undo_count(h), n0, "no history step while dragging");
        assert!(Arc::ptr_eq(&doc(h), &d0), "document untouched while dragging");
        seen |= preview(h).is_some();
    });
    assert!(seen, "live preview while dragging");
    assert!(cp::open_rect(&h.ctx, sw).is_some(), "pop-up still open after the drag");
    assert_eq!(preview(&h), Some(want_preview), "pending alpha 50 % previewed on the canvas");
    assert_ne!(shown_preview_key(&h), 0, "canvas image rendered from the preview");
    assert_eq!(undo_count(&h), n0, "release in the slider applies nothing yet");
    if commit {
        key(&mut h, Key::Enter);
        assert!(cp::open_rect(&h.ctx, sw).is_none(), "Enter closes the pop-up");
        assert_eq!(undo_count(&h), n0 + 1, "exactly one step");
        let a = attrs(&h, r);
        if stroke {
            assert_eq!(a.stroke_color, want, "stroke alpha applied");
            assert_eq!(a.fill_color, before.fill_color, "fill untouched");
        } else {
            assert_eq!(a.fill_color, want, "fill alpha applied");
            assert_eq!(a.stroke_color, before.stroke_color, "stroke untouched");
        }
    } else {
        // first Esc closes the opacity slider, the second the pop-up: nothing applied
        key(&mut h, Key::Escape);
        key(&mut h, Key::Escape);
        assert!(cp::open_rect(&h.ctx, sw).is_none(), "Esc closes the pop-up");
        assert_eq!(undo_count(&h), n0, "no step");
        assert!(Arc::ptr_eq(&doc(&h), &d0), "document unchanged");
    }
    h.run_steps(2);
    assert_eq!(preview(&h), None, "preview gone");
    assert_eq!(shown_preview_key(&h), 0, "canvas shows the document again");
}

#[test]
fn fill_popup_alpha_previews_live_and_enter_applies_once() {
    alpha_case(false, true);
}

#[test]
fn fill_popup_alpha_esc_discards() {
    alpha_case(false, false);
}

#[test]
fn stroke_popup_alpha_previews_live_and_enter_applies_once() {
    alpha_case(true, true);
}

#[test]
fn stroke_popup_alpha_esc_discards() {
    alpha_case(true, false);
}

// ------------------------------------------------------------------ rectangle roundness

#[test]
fn rectangle_roundness_slider_previews_live_and_applies_once() {
    fp_i18n::set_language("de");
    let (mut h, r) = harness();
    let a = rect_of(&h, Id::new("pi-round").with("arrow")).expect("roundness arrow drawn").center();
    let (n0, d0) = (undo_count(&h), doc(&h));
    let to = arrow_drag_to(&mut h, a, 0.41, VTrack::DEFAULT);
    assert_eq!(preview(&h), Some(PreviewValue::Roundness { value: 41.0, pixels: false }), "live preview while dragging");
    assert_ne!(shown_preview_key(&h), 0, "canvas image rendered from the preview");
    assert!(Arc::ptr_eq(&doc(&h), &d0), "document untouched while dragging");
    assert_eq!(undo_count(&h), n0, "no history step while dragging");
    // the preview copy carries the rounded contour
    let p = st(&h).inspector_preview.clone().expect("preview");
    let copy = p.apply(&doc(&h));
    let d = st(&h).session.active().expect("doc");
    let e = fp_engine::docops::element_on_page(&copy, d.page, fp_model::ElementId(r)).expect("element");
    let fp_model::element::ElementKind::Path(pe) = &e.kind else { panic!("path") };
    assert!((pe.primitive.as_ref().expect("primitive").roundness - 0.41).abs() < 1e-6, "copy rounded");
    press(&mut h, to, false);
    h.run_steps(2);
    let e = element(&h, r);
    let fp_model::element::ElementKind::Path(pe) = &e.kind else { panic!("path") };
    assert!((pe.primitive.as_ref().expect("primitive").roundness - 0.41).abs() < 1e-6, "release applies");
    assert_eq!(undo_count(&h), n0 + 1, "exactly one step");
    assert_eq!(preview(&h), None, "preview gone");
}

/// The Tools panel's fill well acts on the selected vectors as well: its pending alpha is previewed
/// on them, Enter applies it in one step.
#[test]
fn tools_panel_fill_well_alpha_previews_live() {
    fp_i18n::set_language("de");
    let (mut h, r) = harness();
    let sw = Id::new(("well", false));
    let well = rect_of(&h, sw).expect("fill well drawn");
    let n0 = undo_count(&h);
    // the well opens its pop-up on its right part (the left part is the stroke well's corner)
    let open_at = Pos2::new(well.right() - well.width() / 4.0, well.center().y);
    let p = {
        click(&mut h, open_at);
        h.run_steps(1);
        cp::open_rect(&h.ctx, sw).expect("pop-up open")
    };
    click(&mut h, p.min + egui::vec2(54.0, 224.5));
    let sr = cp::slider_rect(p.min, h.ctx.content_rect());
    let (top, height) = cp::alpha_track(sr);
    let b = Pos2::new(sr.center().x, top + height * 0.5);
    click(&mut h, b);
    assert_eq!(preview(&h), Some(PreviewValue::FillColor(Argb(0x80ff_0000))), "pending alpha previewed");
    assert_eq!(undo_count(&h), n0, "nothing applied yet");
    key(&mut h, Key::Enter);
    h.run_steps(2);
    assert_eq!(attrs(&h, r).fill_color, Argb(0x80ff_0000), "Enter applies the alpha");
    assert_eq!(undo_count(&h), n0 + 1, "exactly one step");
    assert_eq!(preview(&h), None, "preview gone");
}

// ------------------------------------------- RE-29 (re 61 §2, §4): measured rules

/// Values of the text fields inside the open colour pop-up `popup`, top to bottom, left to right
/// (HEX, opacity, R, G, B).
fn popup_inputs(h: &H, popup: Rect) -> Vec<String> {
    let mut v: Vec<(Rect, String)> = h
        .get_all(egui_kittest::kittest::By::new().role(egui::accesskit::Role::TextInput))
        .map(|n| (n.rect(), n.value().unwrap_or_default()))
        .filter(|(r, _)| popup.contains(r.center()))
        .collect();
    v.sort_by(|a, b| a.0.top().total_cmp(&b.0.top()).then(a.0.left().total_cmp(&b.0.left())));
    v.into_iter().map(|x| x.1).collect()
}

/// HEX and RGB always show the base colour, never blended with the alpha: on opening a
/// semi-transparent fill (deliberate deviation: the reference shows a blended value there) and
/// while the alpha is edited (as measured).
#[test]
fn hex_and_rgb_show_the_base_colour_not_blended() {
    fp_i18n::set_language("de");
    let (mut h, _r) = harness();
    h.state_mut().state.session.execute("pi.fill", json!({"color": "#ff000080"})).expect("alpha fill");
    h.run_steps(3);
    let sw = swatch(("pi-fill", false));
    let r = rect_of(&h, sw).expect("swatch drawn");
    click(&mut h, r.center());
    h.run_steps(1);
    let p = cp::open_rect(&h.ctx, sw).expect("pop-up open");
    assert_eq!(popup_inputs(&h, p), ["#FF0000", "50", "255", "0", "0"], "base colour on opening");
    key(&mut h, Key::Escape);
    let o = drag_alpha(&mut h, sw, 30.0, |_| {});
    let p = Rect::from_min_size(o, egui::vec2(cp::POPUP_W, cp::POPUP_H));
    assert_eq!(popup_inputs(&h, p), ["#FF0000", "30", "255", "0", "0"], "base colour while the alpha is edited");
}

/// Opens the colour pop-up of `sw` and presses on its opacity arrow and drags to `pct` percent
/// (pointer still held). Returns the pointer position.
fn arrow_alpha_drag(h: &mut H, sw: Id, pct: f32) -> Pos2 {
    let r = rect_of(h, sw).expect("swatch drawn");
    click(h, r.center());
    h.run_steps(1);
    let p = cp::open_rect(&h.ctx, sw).expect("colour pop-up open");
    let a = p.min + egui::vec2(54.0, 224.5);
    ev(h, Event::PointerMoved(a));
    press(h, a, true);
    for i in 1..=3 {
        ev(h, Event::PointerMoved(a + egui::vec2(0.0, 4.0 * i as f32)));
    }
    let sr = cp::slider_rect(p.min, h.ctx.content_rect());
    let (top, height) = cp::alpha_track(sr);
    let b = Pos2::new(sr.center().x, top + height * (1.0 - pct / 100.0));
    let from = a + egui::vec2(0.0, 12.0);
    for i in 1..=8 {
        ev(h, Event::PointerMoved(from + (b - from) * (i as f32 / 8.0)));
    }
    h.run_steps(1);
    b
}

/// Press on the opacity arrow and drag: the canvas previews the alpha, release applies it in one
/// step and closes the pop-up (as measured: fill `#ff000080`, stroke `#0000ff80`; the reference
/// records two rows for the stroke, Firepixel one).
#[test]
fn alpha_arrow_press_drag_release_applies_once_and_closes() {
    fp_i18n::set_language("de");
    for stroke in [false, true] {
        let (mut h, r) = harness();
        if stroke {
            h.state_mut().state.session.execute("pi.stroke", json!({"color": "#0000ff"})).expect("blue stroke");
            h.run_steps(2);
        }
        let sw = swatch(if stroke { ("pi-stroke", false) } else { ("pi-fill", false) });
        let (n0, d0) = (undo_count(&h), doc(&h));
        let want = if stroke { Argb(0x8000_00ff) } else { Argb(0x80ff_0000) };
        let to = arrow_alpha_drag(&mut h, sw, 50.0);
        let wp = if stroke { PreviewValue::StrokeColor(want) } else { PreviewValue::FillColor(want) };
        assert_eq!(preview(&h), Some(wp), "live preview while dragging (stroke {stroke})");
        assert!(Arc::ptr_eq(&doc(&h), &d0), "document untouched while dragging");
        assert_eq!(undo_count(&h), n0, "no step while dragging");
        press(&mut h, to, false);
        h.run_steps(2);
        assert!(cp::open_rect(&h.ctx, sw).is_none(), "release closes the pop-up");
        assert_eq!(undo_count(&h), n0 + 1, "exactly one step (stroke {stroke})");
        let a = attrs(&h, r);
        assert_eq!(if stroke { a.stroke_color } else { a.fill_color }, want, "alpha applied");
        assert_eq!(preview(&h), None, "preview gone");
    }
}

/// Esc while dragging from the opacity arrow restores the opacity: nothing previewed, nothing
/// applied.
#[test]
fn alpha_arrow_drag_esc_cancels() {
    fp_i18n::set_language("de");
    let (mut h, r) = harness();
    let sw = swatch(("pi-fill", false));
    let (n0, d0) = (undo_count(&h), doc(&h));
    let to = arrow_alpha_drag(&mut h, sw, 40.0);
    assert!(preview(&h).is_some(), "previewed while dragging");
    key(&mut h, Key::Escape);
    press(&mut h, to, false);
    h.run_steps(2);
    assert_eq!(preview(&h), None, "Esc restores the opacity: no preview");
    assert_eq!(undo_count(&h), n0, "no step");
    assert!(Arc::ptr_eq(&doc(&h), &d0), "document unchanged");
    assert_eq!(attrs(&h, r).fill_color, Argb(0xffff_0000));
}

/// Layers panel: Opacity and Blend mode show blank when the selected objects have mixed values
/// (RE-29, re 61 §4); one object shows its value; nothing selected greys them out.
#[test]
fn layers_opacity_and_blend_blank_on_mixed_selection() {
    fp_i18n::set_language("de");
    let (mut h, r1) = harness();
    let s = &mut h.state_mut().state.session;
    let v = s.execute("draw.rectangle", json!({"rect": [200, 40, 300, 130], "fillColor": "#00ff00", "strokeWidth": 0})).expect("rect 2");
    let r2 = v.value.get("id").and_then(|x| x.as_u64()).expect("id");
    s.execute("select.set", json!({"ids": [r2]})).expect("select");
    s.execute("element.setOpacity", json!({"opacity": 50})).expect("opacity");
    s.execute("select.set", json!({"ids": [r1, r2]})).expect("select both");
    h.run_steps(3);
    let arrow = layers_arrow(&mut h);
    let field = |h: &H| -> String {
        h.get_all(egui_kittest::kittest::By::new().role(egui::accesskit::Role::TextInput))
            .map(|n| (n.rect(), n.value().unwrap_or_default()))
            .filter(|(r, _)| (r.center().y - arrow.center().y).abs() < 4.0 && r.right() <= arrow.left() + 2.0 && r.right() >= arrow.left() - 8.0)
            .map(|x| x.1)
            .next()
            .expect("Layers opacity field")
    };
    assert_eq!(field(&h), "", "mixed opacity: blank");
    h.state_mut().state.session.execute("select.set", json!({"ids": [r2]})).expect("select one");
    h.run_steps(3);
    assert_eq!(field(&h), "50", "one object: its value");
    h.state_mut().state.session.execute("element.setOpacity", json!({"ids": [r1], "opacity": 50})).expect("same opacity");
    h.state_mut().state.session.execute("select.set", json!({"ids": [r1, r2]})).expect("select both");
    h.run_steps(3);
    assert_eq!(field(&h), "50", "equal values: shown");
}

// ------------------------------------------- R-32: opacity pop-ups give whole per cents, 2 px each

/// Press on the opacity arrow `arrow`, start the drag and move to `dy` px below the thumb of 100 %
/// of the opened pop-up (pointer still held). Returns the pointer position and the slider rect.
fn opacity_drag_below_100(h: &mut H, arrow: Pos2, dy: f32) -> (Pos2, Rect) {
    ev(h, Event::PointerMoved(arrow));
    press(h, arrow, true);
    for i in 1..=4 {
        ev(h, Event::PointerMoved(arrow + egui::vec2(0.0, 4.0 * i as f32)));
    }
    let sr = *sliders(h).first().expect("pop-up slider open");
    let from = arrow + egui::vec2(0.0, 16.0);
    let to = Pos2::new(arrow.x, sr.top() + VTrack::PERCENT.pad + dy);
    for i in 1..=8 {
        ev(h, Event::PointerMoved(from + (to - from) * (i as f32 / 8.0)));
    }
    h.run_steps(1);
    (to, sr)
}

/// R-32 (RE spec 61 §4.1, RE spec 60 §2.3): the opacity pop-ups of the Layers panel and the
/// Properties inspector run 2 px per per cent (the reference: a 41 px drag from 100 gives 80;
/// Firepixel's 212 px track gave 81, stored as 207/255 = 81.18). A drag that ends between two steps
/// shows and commits a whole per cent; the History step carries it.
#[test]
fn opacity_sliders_two_px_per_percent_commit_whole_percent_r32() {
    fp_i18n::set_language("de");
    for layers in [true, false] {
        let (mut h, r) = harness();
        let a = if layers {
            layers_arrow(&mut h).center()
        } else {
            rect_of(&h, Id::new("pi-opacity").with("arrow")).expect("Properties opacity arrow drawn").center()
        };
        let n0 = undo_count(&h);
        // 40.6 px below the thumb of 100 %: 100 − 40.6 / 2 = 79.7 → 80
        let (to, sr) = opacity_drag_below_100(&mut h, a, 40.6);
        assert!((sr.height() - (fp_ui::inspector::fields::VSLIDER_TRACK + 2.0 * fp_ui::inspector::fields::VSLIDER_PAD)).abs() < 0.5, "pop-up size unchanged");
        if layers {
            let thumb100 = sr.top() + VTrack::PERCENT.pad;
            assert!((thumb100 - a.y).abs() <= 1.0, "thumb of 100 % under the press point: {thumb100} / {}", a.y);
        }
        assert_eq!(preview(&h), Some(PreviewValue::Opacity(80.0)), "live preview shows the whole per cent (layers {layers})");
        press(&mut h, to, false);
        h.run_steps(2);
        assert_eq!(undo_count(&h), n0 + 1, "one step (layers {layers})");
        assert_eq!(element(&h, r).common.opacity.0, 204, "stored 80 % = 204/255 (layers {layers})");
        assert_eq!(element(&h, r).common.opacity.to_percent(), 80.0, "read back 80, not 81.18");
        let d = st(&h).session.active().expect("doc");
        let step = d.history.steps().last().copied().expect("history step");
        assert_eq!(step.command, "element.setOpacity");
        assert_eq!(step.params["opacity"].as_f64(), Some(80.0), "History step carries 80: {}", step.params);
        assert!(step.label.contains("Deckkraft"), "row text {}", step.label);
    }
}

/// R-32: the colour pop-up's alpha slider (same 24 × 222 pop-up) also runs 2 px per per cent with
/// 100 % 10 px below its top edge; a drag that ends between two steps applies a whole per cent
/// (52 % → alpha 0x85 like the reference's `#ff000085`).
#[test]
fn alpha_slider_two_px_per_percent_commits_whole_percent_r32() {
    fp_i18n::set_language("de");
    let (mut h, r) = harness();
    let sw = swatch(("pi-fill", false));
    let n0 = undo_count(&h);
    // 51.65 % lies between two steps (0.7 px below the step of 52)
    let to = arrow_alpha_drag(&mut h, sw, 51.65);
    let p = cp::open_rect(&h.ctx, sw).expect("pop-up open");
    let sr = cp::slider_rect(p.min, h.ctx.content_rect());
    let (top, len) = cp::alpha_track(sr);
    assert_eq!((top - sr.top(), len), (10.0, 200.0), "track: 100 % 10 px below the top, 2 px per per cent");
    assert_eq!(preview(&h), Some(PreviewValue::FillColor(Argb(0x85ff_0000))), "live preview with 52 %");
    press(&mut h, to, false);
    h.run_steps(2);
    assert_eq!(undo_count(&h), n0 + 1, "one step");
    assert_eq!(attrs(&h, r).fill_color, Argb(0x85ff_0000), "52 % applied");
}
