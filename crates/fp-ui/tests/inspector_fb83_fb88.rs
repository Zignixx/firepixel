//! FB-83 (TW-259) and FB-88 (TW-260) in the Properties inspector:
//! - the mask inspector fits the panel: every row of the vector mask (Mask: Path outline /
//!   Grayscale, Show fill and stroke) and of the bitmap mask (Mask: Alpha channel / Grayscale) and
//!   the Firepixel-own "Invert" checkbox lie inside their row of the panel, at full and at half
//!   height; the reference's rows keep their order; a dragged height between half and full
//!   height scrolls instead of cutting rows off;
//! - FB-82: Add Mask (Reveal All) makes a bitmap mask in Grayscale mode, Paste Inside onto a
//!   bitmap one in Alpha channel mode;
//! - the dither density field of the fill ends with a gap before the "Transparent" checkbox, has a pop-up slider arrow with a live canvas preview while the thumb
//!   moves, and the release applies one History row.
//!
//! Realistic input like egui-winit on Windows (one event per frame).

use egui::accesskit::Role;
use egui::{Event, Modifiers, PointerButton, Pos2, Rect};
use egui_kittest::Harness;
use egui_kittest::kittest::{By, Queryable};
use fp_engine::tools::ToolId;
use fp_model::ElementKind;
use fp_ui::FirepixelApp;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::inspector::preview::PreviewValue;
use serde_json::{Value, json};
use std::sync::Arc;

type H = Harness<'static, FirepixelApp>;

const W: f32 = 1800.0;
const HH: f32 = 1130.0;

fn ex(s: &mut fp_engine::Session, c: &str, p: Value) -> Value {
    s.execute(c, p).unwrap_or_else(|e| panic!("{c}: {e}")).value
}

fn id_of(v: &Value) -> u64 {
    v["id"].as_u64().expect("id")
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

fn session_with_bitmap() -> (fp_engine::Session, u64) {
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
    (s, b)
}

fn square(s: &mut fp_engine::Session) -> u64 {
    id_of(&ex(
        s,
        "draw.path",
        json!({"contours": [{"closed": true, "nodes": [[120, 120], [180, 120], [180, 180], [120, 180]]}]}),
    ))
}

/// Bitmap masked by a square path in Path outline mode, the mask selected (vector mask inspector).
fn vector_mask_harness() -> H {
    let (mut s, b) = session_with_bitmap();
    let m = square(&mut s);
    ex(&mut s, "select.set", json!({"ids": [m, b]}));
    ex(&mut s, "modify.mask.groupAsMask", json!({}));
    ex(
        &mut s,
        "modify.mask.options",
        json!({"mode": "path", "showAttrs": false}),
    );
    ex(&mut s, "select.set", json!({"ids": [m]}));
    s.set_tool(ToolId::Pointer).expect("pointer");
    build(s)
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn panel(h: &H) -> Rect {
    fp_ui::inspector::fields::panel_rect(&h.ctx).expect("Properties panel drawn")
}

/// Bottom of row `rows` (1 or 2) of quads: tab row, 1 px line, 2 px margin, rows × row height.
fn rows_bottom(h: &H, rows: f32) -> f32 {
    let t = fp_ui::theme::Theme::current(&h.ctx);
    panel(h).top()
        + t.metrics.inspector_header_height
        + 1.0
        + 2.0
        + rows * t.metrics.inspector_row_height
}

/// Checkbox / radio of the inspector with this label (interact rect).
fn control(h: &H, role: Role, key: &str) -> Option<Rect> {
    let label = fp_i18n::t(key);
    let p = panel(h);
    h.query_all(By::new().role(role).label(label.as_str()))
        .map(|n| n.rect())
        .find(|r| r.top() >= p.top() && r.left() >= p.left())
}

/// The painted box / circle (13 px around the row centre) of a control lies inside the panel and
/// above `bottom`.
fn assert_fits(h: &H, what: &str, r: Rect, bottom: f32) {
    let p = panel(h);
    assert!(
        r.left() >= p.left() && r.right() <= p.right() + 0.5,
        "{what}: {r:?} in {p:?}"
    );
    assert!(r.center().y - 6.5 >= p.top(), "{what}: {r:?}");
    assert!(
        r.center().y + 6.5 <= bottom + 0.01,
        "{what}: {r:?} ends below the row bottom {bottom}"
    );
    assert!(
        r.center().y + 6.5 <= p.bottom(),
        "{what}: {r:?} below the panel {p:?}"
    );
}

#[test]
fn vector_mask_rows_fit_and_keep_their_order() {
    let mut h = vector_mask_harness();
    let full = rows_bottom(&h, 2.0);
    let path = control(&h, Role::RadioButton, "pi-mask-path-outline").expect("Path outline");
    let gray = control(&h, Role::RadioButton, "pi-mask-grayscale").expect("Grayscale");
    let show = control(&h, Role::CheckBox, "pi-mask-show-attrs").expect("Show fill and stroke");
    let inv = control(&h, Role::CheckBox, "pi-mask-invert").expect("Invert");
    for (w, r) in [
        ("path", path),
        ("gray", gray),
        ("show", show),
        ("invert", inv),
    ] {
        assert_fits(&h, w, r, full);
    }
    // the reference's rows keep their order: Mask radios, then Show fill and stroke
    assert!(
        path.top() < gray.top() && gray.top() < show.top(),
        "{path:?} {gray:?} {show:?}"
    );
    // Invert (Firepixel) right above the mask rows in the same column
    assert!(
        inv.bottom() <= path.top() + 0.5 && (inv.left() - show.left()).abs() < 0.5,
        "{inv:?} {show:?}"
    );
    // half height: the first row stays, Invert remains reachable
    h.state_mut().state.settings.workspace.inspector_half = true;
    h.run_steps(4);
    let half = rows_bottom(&h, 1.0);
    let inv = control(&h, Role::CheckBox, "pi-mask-invert").expect("Invert at half height");
    assert_fits(&h, "invert half", inv, half);
    // a dragged height between half and full height scrolls instead of cutting the rows off
    h.state_mut().state.settings.workspace.inspector_half = false;
    h.state_mut().state.settings.workspace.inspector_height = 18.0 + 48.0 + 30.0;
    h.run_steps(4);
    let vp = fp_ui::inspector::hscroll_viewport(&h.ctx).expect("vertical scroll");
    assert!(vp.bottom() <= panel(&h).bottom() + 0.5, "{vp:?}");
    // the default size needs no scroll
    h.state_mut().state.settings.workspace.inspector_height = 0.0;
    h.run_steps(4);
    assert!(
        fp_ui::inspector::hscroll_viewport(&h.ctx).is_none(),
        "full height: no scroll"
    );
}

#[test]
fn bitmap_mask_rows_fit_and_add_mask_is_grayscale() {
    let (mut s, b) = session_with_bitmap();
    ex(&mut s, "select.set", json!({"ids": [b]}));
    ex(&mut s, "modify.mask.revealAll", json!({}));
    s.set_tool(ToolId::Pointer).expect("pointer");
    let mut h = build(s);
    // FB-82: Add Mask (Reveal All) gives Grayscale ("mask to image")
    assert_eq!(
        fp_ui::inspector::obj_quads::selected_mask_mode(st(&h)),
        Some(false)
    );
    for half in [false, true] {
        h.state_mut().state.settings.workspace.inspector_half = half;
        h.run_steps(4);
        let bottom = rows_bottom(&h, 1.0);
        let alpha = control(&h, Role::RadioButton, "pi-mask-alpha").expect("Alpha channel");
        let gray = control(&h, Role::RadioButton, "pi-mask-gray").expect("Grayscale");
        let inv = control(&h, Role::CheckBox, "pi-mask-invert").expect("Invert");
        for (w, r) in [("alpha", alpha), ("gray", gray), ("invert", inv)] {
            assert_fits(&h, w, r, bottom);
        }
        // Invert (Firepixel) right of the mode radios, in the row of Alpha channel
        assert!(alpha.top() < gray.top(), "{alpha:?} {gray:?}");
        assert!(
            inv.left() >= alpha.right() && (inv.center().y - alpha.center().y).abs() < 1.0,
            "{inv:?} {alpha:?}"
        );
    }
}

#[test]
fn paste_inside_onto_a_bitmap_uses_alpha_channel() {
    let (mut s, b) = session_with_bitmap();
    let p = square(&mut s);
    ex(&mut s, "select.set", json!({"ids": [p]}));
    ex(&mut s, "edit.copy", json!({}));
    ex(&mut s, "select.set", json!({"ids": [b]}));
    ex(&mut s, "edit.pasteInside", json!({}));
    let d = s.active().expect("doc");
    let owner = fp_engine::docops::mask_owner(&d.doc, d.page, fp_model::ElementId(b))
        .expect("the bitmap is now a mask");
    let m = owner.element_mask().expect("mask");
    assert!(
        m.to_path,
        "Paste Inside: Alpha channel / Path outline (mask to path)"
    );
    assert!(
        matches!(&m.element.kind, ElementKind::Bitmap(_)),
        "the bitmap is the mask"
    );
}

// ------------------------------------------------------------------ FB-88

fn rect_harness() -> (H, u64) {
    let mut s = fp_engine::Session::default();
    ex(
        &mut s,
        "file.new",
        json!({"width": 500, "height": 300, "background": "#ffffff"}),
    );
    let id = id_of(&ex(
        &mut s,
        "draw.rectangle",
        json!({"rect": [40, 40, 300, 200], "fillColor": "#336699", "strokeWidth": 0}),
    ));
    ex(&mut s, "select.set", json!({"ids": [id]}));
    (build(s), id)
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

fn doc(h: &H) -> Arc<fp_model::Document> {
    Arc::clone(&st(h).session.active().expect("doc").doc)
}

fn density_of(d: &fp_model::Document, id: u64) -> i32 {
    let e = fp_engine::docops::element_on_page(d, 0, fp_model::ElementId(id)).expect("element");
    fp_engine::docops::path_attrs_of(&e)
        .and_then(|a| a.fill.as_ref().map(|f| f.dither_density))
        .expect("fill")
}

fn undo_count(h: &H) -> usize {
    st(h)
        .session
        .active()
        .map(|d| d.history.undo_count())
        .unwrap_or(0)
}

/// The dither density field: the text field in the row of "Transparent", left of it.
fn density_field(h: &H, tr: Rect) -> Rect {
    h.get_all(By::new().role(Role::TextInput))
        .map(|n| n.rect())
        .filter(|r| (r.center().y - tr.center().y).abs() < 3.0 && r.right() <= tr.left())
        .max_by(|a, b| a.left().total_cmp(&b.left()))
        .expect("dither density field")
}

#[test]
fn dither_density_field_has_room_and_a_live_slider() {
    let (mut h, id) = rect_harness();
    let tr = control(&h, Role::CheckBox, "pi-texture-transparent").expect("Transparent");
    let f = density_field(&h, tr);
    assert!(
        !f.expand(2.0).intersects(tr),
        "field {f:?} keeps a gap to Transparent {tr:?}"
    );
    // the arrow (7 DLU) ends 3 DLU (4.5 px) before the checkbox
    assert!(f.right() + 10.5 + 4.0 <= tr.left(), "{f:?} {tr:?}");
    assert!(f.bottom() <= rows_bottom(&h, 2.0), "{f:?}");
    // the pop-up slider arrow right of the field: drag up previews, release applies once
    let rows = undo_count(&h);
    let a = Pos2::new(f.right() + 5.0, f.center().y);
    ev(&mut h, Event::PointerMoved(a));
    press(&mut h, a, true);
    let to = Pos2::new(a.x, a.y - 40.0);
    for i in 1..=10 {
        ev(
            &mut h,
            Event::PointerMoved(a + (to - a) * (i as f32 / 10.0)),
        );
    }
    h.run_steps(1);
    let p = st(&h)
        .inspector_preview
        .clone()
        .expect("live preview while dragging");
    let PreviewValue::DitherDensity(live) = p.value else {
        panic!("dither density preview: {:?}", p.value)
    };
    assert!(live > 0.0, "dragged up: {live}");
    assert_eq!(
        density_of(&p.apply(&doc(&h)), id),
        live.round() as i32,
        "the preview copy carries the value"
    );
    assert_eq!(
        density_of(&doc(&h), id),
        0,
        "document untouched while dragging"
    );
    assert_eq!(undo_count(&h), rows, "no history row while dragging");
    press(&mut h, to, false);
    h.run_steps(3);
    assert_eq!(
        density_of(&doc(&h), id),
        live.round() as i32,
        "release applies"
    );
    assert_eq!(undo_count(&h), rows + 1, "one history row");
}
