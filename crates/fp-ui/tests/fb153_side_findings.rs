//! FB-153 (side findings in the tester videos of batch 16):
//! - the History panel keeps the current (newest) step visible when steps are added;
//! - clicking "Show fill and stroke" of a vector mask leaves the hover / focus highlight on that
//!   checkbox, it never jumps to the "Invert" checkbox above it.
//!
//! Realistic input like egui-winit on Windows (one event per frame).

use egui::{Event, Id, Modifiers, PointerButton, Pos2, Rect};
use egui_kittest::Harness;
use egui_kittest::kittest::{By, Queryable};
use fp_engine::tools::ToolId;
use fp_ui::FirepixelApp;
use fp_ui::app_state::{AppState, UiSettings};
use serde_json::{Value, json};

type H = Harness<'static, FirepixelApp>;

const W: f32 = 1800.0;
const HH: f32 = 1130.0;

fn ex(s: &mut fp_engine::Session, c: &str, p: Value) -> Value {
    s.execute(c, p).unwrap_or_else(|e| panic!("{c}: {e}")).value
}

fn build(s: fp_engine::Session) -> H {
    fp_i18n::set_language("en");
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.os_clipboard = false;
    app.state.sync_render = true;
    let mut h = Harness::builder()
        .with_size(egui::vec2(W, HH))
        .with_pixels_per_point(1.0)
        .with_max_steps(20)
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(6);
    h
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn ev(h: &mut H, e: Event) {
    h.event(e);
    h.run_steps(1);
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

/// Painted texts equal to `label` in the right half (the panels) with their clip rectangles.
fn texts(h: &H, label: &str) -> Vec<(Rect, Rect)> {
    fn walk(shape: &egui::epaint::Shape, clip: Rect, out: &mut Vec<(String, Rect, Rect)>) {
        match shape {
            egui::epaint::Shape::Text(t) => out.push((
                t.galley.job.text.clone(),
                Rect::from_min_size(t.pos, t.galley.size()),
                clip,
            )),
            egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| walk(s, clip, out)),
            _ => {}
        }
    }
    let mut all = Vec::new();
    for cs in &h.output().shapes {
        walk(&cs.shape, cs.clip_rect, &mut all);
    }
    all.into_iter()
        .filter(|(t, r, _)| t == label && r.center().x > W / 2.0)
        .map(|(_, r, c)| (r, c))
        .collect()
}

fn bring_tab(h: &mut H, label: &str) {
    let tab = texts(h, label)
        .into_iter()
        .map(|(r, _)| r)
        .max_by(|a, b| a.center().x.total_cmp(&b.center().x))
        .unwrap_or_else(|| panic!("{label} tab"));
    click(h, tab.center());
    h.run_steps(10);
}

fn newest_label(h: &H) -> String {
    let s = st(h);
    let d = s.session.active().expect("doc");
    s.session
        .step_label(d.history.steps().last().expect("step"))
}

/// FB-153 (1): after many new steps the History panel shows the newest step (the list scrolls
/// to it); before, the list stayed at its old position and the new rows were hidden below.
#[test]
fn fb153_history_panel_follows_the_newest_step() {
    let mut s = fp_engine::Session::default();
    ex(
        &mut s,
        "file.new",
        json!({"width": 500, "height": 300, "background": "#ffffff"}),
    );
    let mut h = build(s);
    bring_tab(&mut h, "History");
    let mut last = 0;
    for i in 0..40 {
        let v = ex(
            &mut h.state_mut().state.session,
            "draw.rectangle",
            json!({"rect": [10 + i, 10, 60 + i, 60], "fillColor": "#336699", "strokeWidth": 0}),
        );
        last = v["id"].as_u64().expect("id");
    }
    ex(
        &mut h.state_mut().state.session,
        "select.set",
        json!({"ids": [last]}),
    );
    ex(
        &mut h.state_mut().state.session,
        "modify.transform.flipHorizontal",
        json!({}),
    );
    h.run_steps(6);
    let label = newest_label(&h);
    let shown = texts(&h, &label);
    assert!(!shown.is_empty(), "{label:?} drawn");
    assert!(
        shown
            .iter()
            .any(|(r, clip)| clip.contains(r.center()) && clip.height() > 30.0),
        "newest History row {label:?} visible: {shown:?}"
    );
    // Undo moves the current step one up; the undone row next to it stays in view
    h.state_mut().state.run("edit.undo", json!({}));
    h.run_steps(6);
    let shown = texts(&h, &label);
    assert!(
        shown.iter().any(|(r, clip)| clip.contains(r.center())),
        "undone row stays in view: {shown:?}"
    );
}

/// Bitmap masked by a square path in Path outline mode, the mask selected (vector mask inspector).
fn vector_mask_harness() -> H {
    let mut s = fp_engine::Session::default();
    ex(
        &mut s,
        "file.new",
        json!({"width": 400, "height": 300, "background": "#ffffff"}),
    );
    let img = fp_raster::RgbaImage::filled(
        fp_geom::IRect::from_xywh(100, 100, 100, 100),
        fp_color::Rgba8::new(255, 0, 0, 255),
    )
    .expect("image");
    s.clipboard.clear();
    s.clipboard.pixels = Some(img);
    let b = ex(&mut s, "edit.paste", json!({}))["ids"][0]
        .as_u64()
        .expect("bitmap");
    let m = ex(
        &mut s,
        "draw.path",
        json!({"contours": [{"closed": true, "nodes": [[120, 120], [180, 120], [180, 180], [120, 180]]}]}),
    )["id"]
        .as_u64()
        .expect("path");
    ex(&mut s, "select.set", json!({"ids": [m, b]}));
    ex(&mut s, "modify.mask.groupAsMask", json!({}));
    ex(
        &mut s,
        "modify.mask.options",
        json!({"mode": "path", "showAttrs": true}),
    );
    ex(&mut s, "select.set", json!({"ids": [m]}));
    s.set_tool(ToolId::Pointer).expect("pointer");
    build(s)
}

fn checkbox(h: &H, key: &str) -> Rect {
    let label = fp_i18n::t(key);
    let p = fp_ui::inspector::fields::panel_rect(&h.ctx).expect("Properties panel drawn");
    h.query_all(
        By::new()
            .role(egui::accesskit::Role::CheckBox)
            .label(label.as_str()),
    )
    .map(|n| n.rect())
    .find(|r| r.top() >= p.top() && r.left() >= p.left())
    .unwrap_or_else(|| panic!("{key} drawn"))
}

fn hovered(h: &H, id: &str) -> bool {
    // the checkbox interacts with `Id::new(id)` where `id` is itself an `egui::Id`
    h.ctx
        .read_response(Id::new(Id::new(id)))
        .is_some_and(|r| r.hovered())
}

fn show_attrs(h: &H) -> Option<bool> {
    st(h)
        .session
        .active()
        .and_then(|d| {
            let id = d.selection.elements.first()?;
            fp_engine::docops::mask_owner(&d.doc, d.page, *id)
        })
        .and_then(|o| o.element_mask().map(|m| m.show_attrs))
}

/// FB-153 (5): clicking "Show fill and stroke" (off, on, off) keeps the highlight on that box;
/// the "Invert" box next to it (below it since F165) is never highlighted while the pointer rests on "Show".
#[test]
fn fb153_show_fill_click_keeps_highlight_off_invert() {
    let mut h = vector_mask_harness();
    let show = checkbox(&h, "pi-mask-show-attrs");
    let invert = checkbox(&h, "pi-mask-invert");
    assert!(!invert.contains(show.center()));
    for want in [false, true, false] {
        click(&mut h, show.center());
        assert_eq!(show_attrs(&h), Some(want), "Show fill and stroke toggled");
        for _ in 0..3 {
            assert!(
                !hovered(&h, "pi-mask-invert"),
                "Invert highlighted after the click (show = {want})"
            );
            assert!(hovered(&h, "pi-mask-show"), "Show keeps the highlight");
            h.run_steps(1);
        }
        assert_eq!(
            checkbox(&h, "pi-mask-show-attrs"),
            show,
            "Show stays in place"
        );
    }
}
