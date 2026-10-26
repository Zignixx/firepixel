//! FB-101 and the rests of *RE spec* 73 (TW-273):
//! - the bitmap mask inspector keeps its measured rows (Alpha channel / Grayscale, Firepixel's
//!   Invert) and has no number field; *Alpha channel* carries a tooltip that says where the Paste
//!   Inside underlay comes from and how to mask without it; the Layers panel's *Opacity* field
//!   carries a tooltip, with a mask selected the one about the mask object's own opacity;
//! - Layers thumbnails as measured (*RE spec* 73 §6): 34 × 23 px frame, 1 px #696969, image area
//!   32 × 21 with the page fitted inside, 27 px object rows; the content thumbnail of a masked
//!   bitmap carries the bitmap badge bottom right, a vector mask thumbnail the pen badge, a bitmap
//!   mask thumbnail none; masked objects inside a group tree show the same.

use egui::accesskit::Role;
use egui::epaint::Shape;
use egui::{Color32, Event, Rect};
use egui_kittest::Harness;
use egui_kittest::kittest::{By, Queryable};
use fp_engine::tools::ToolId;
use fp_ui::FirepixelApp;
use fp_ui::app_state::UiSettings;
use fp_ui::panels::layers::{THUMB_FRAME, object_row_height, thumb_frame_size};
use serde_json::{Value, json};

type H = Harness<'static, FirepixelApp>;

fn ex(s: &mut fp_engine::Session, c: &str, p: Value) -> Value {
    s.execute(c, p).unwrap_or_else(|e| panic!("{c}: {e}")).value
}

fn paste_bitmap(s: &mut fp_engine::Session, x: i32, y: i32, w: i32, h: i32, c: [u8; 4]) -> u64 {
    let img = fp_raster::RgbaImage::filled(
        fp_geom::IRect::from_xywh(x, y, w, h),
        fp_color::Rgba8::new(c[0], c[1], c[2], c[3]),
    )
    .expect("image");
    s.clipboard.clear();
    s.clipboard.pixels = Some(img);
    ex(s, "edit.paste", json!({}))["ids"][0]
        .as_u64()
        .expect("bitmap")
}

fn name(s: &mut fp_engine::Session, id: u64, n: &str) {
    ex(s, "element.setName", json!({"ids": [id], "name": n}));
}

/// Paste Inside of a blue bitmap onto a red bitmap mask: returns the masked object.
fn paste_inside(s: &mut fp_engine::Session, x: i32) -> u64 {
    let target = paste_bitmap(s, x, 100, 100, 100, [255, 0, 0, 255]);
    let content = paste_bitmap(s, x - 40, 60, 80, 80, [0, 0, 255, 255]);
    ex(s, "select.set", json!({"ids": [content]}));
    ex(s, "edit.cut", json!({}));
    ex(s, "select.set", json!({"ids": [target]}));
    ex(s, "edit.pasteInside", json!({}));
    let d = s.active().expect("doc");
    fp_engine::docops::mask_owner(&d.doc, d.page, fp_model::ElementId(target))
        .expect("masked object")
        .id
        .0
}

/// "PI": Paste Inside bitmap mask; "VM": bitmap masked by a square path (vector mask); with
/// `grouped` "PI" sits in the group "Grp" (group tree row).
fn session(grouped: bool) -> (fp_engine::Session, u64) {
    let mut s = fp_engine::Session::default();
    ex(
        &mut s,
        "file.new",
        json!({"width": 400, "height": 300, "background": "#ffffff"}),
    );
    let pi = paste_inside(&mut s, 60);
    name(&mut s, pi, "PI");
    let b = paste_bitmap(&mut s, 250, 100, 100, 100, [0, 160, 0, 255]);
    let m = ex(
        &mut s,
        "draw.path",
        json!({"contours": [{"closed": true, "nodes": [[260, 110], [330, 110], [330, 180], [260, 180]]}]}),
    )["id"]
        .as_u64()
        .expect("path");
    ex(&mut s, "select.set", json!({"ids": [m, b]}));
    let vm = ex(&mut s, "modify.mask.groupAsMask", json!({}))["id"]
        .as_u64()
        .expect("group");
    ex(&mut s, "modify.mask.options", json!({"mode": "path"}));
    name(&mut s, vm, "VM");
    if grouped {
        let other = paste_bitmap(&mut s, 10, 10, 20, 20, [0, 0, 0, 255]);
        name(&mut s, other, "O");
        ex(&mut s, "select.set", json!({"ids": [pi, other]}));
        let g = ex(&mut s, "modify.group", json!({}))["id"]
            .as_u64()
            .expect("group");
        name(&mut s, g, "Grp");
    }
    ex(&mut s, "select.none", json!({}));
    s.set_tool(ToolId::Pointer).expect("pointer");
    (s, pi)
}

fn build(s: fp_engine::Session, lang: &str) -> H {
    fp_i18n::set_language(lang);
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.os_clipboard = false;
    app.state.sync_render = true;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1800.0, 1130.0))
        .with_pixels_per_point(1.0)
        .with_max_steps(20)
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(6);
    h
}

fn texts(shape: &Shape, out: &mut Vec<(String, Rect)>) {
    match shape {
        Shape::Text(t) => out.push((
            t.galley.job.text.clone(),
            Rect::from_min_size(t.pos, t.galley.size()),
        )),
        Shape::Vec(v) => v.iter().for_each(|s| texts(s, out)),
        _ => {}
    }
}

/// Text rect of a Layers row name (the right part of the window), rightmost match.
fn row(h: &H, label: &str) -> Option<Rect> {
    let mut all = Vec::new();
    for cs in &h.output().shapes {
        texts(&cs.shape, &mut all);
    }
    all.into_iter()
        .filter(|(t, r)| t == label && r.center().x > 1200.0)
        .map(|(_, r)| r)
        .max_by(|a, b| a.center().x.total_cmp(&b.center().x))
}

fn click(h: &mut H, p: egui::Pos2) {
    h.input_mut().events.push(Event::PointerMoved(p));
    h.step();
    for down in [true, false] {
        h.input_mut().events.push(Event::PointerButton {
            pos: p,
            button: egui::PointerButton::Primary,
            pressed: down,
            modifiers: Default::default(),
        });
        h.step();
    }
}

fn ensure_layers(h: &mut H, label: &str) {
    for _ in 0..2 {
        if row(h, label).is_some() {
            return;
        }
        let tab = fp_i18n::t("panel-title-layers");
        if let Some(r) = row(h, &tab) {
            click(h, r.center());
        }
        h.run_steps(30);
    }
    assert!(row(h, label).is_some(), "Layers panel shows row {label}");
}

/// Rect frames (rect, stroke colour, stroke width) and textured meshes of the last frame.
#[derive(Default)]
struct Shapes {
    frames: Vec<(Rect, Color32, f32)>,
    meshes: Vec<Rect>,
}

fn walk(s: &Shape, out: &mut Shapes) {
    match s {
        Shape::Rect(r) if r.stroke.width > 0.0 => {
            out.frames.push((r.rect, r.stroke.color, r.stroke.width))
        }
        Shape::Mesh(m) if m.texture_id != egui::TextureId::default() => {
            let mut b = Rect::NOTHING;
            for v in &m.vertices {
                b.extend_with(v.pos);
            }
            out.meshes.push(b);
        }
        Shape::Vec(v) => v.iter().for_each(|c| walk(c, out)),
        _ => {}
    }
}

fn shapes(h: &H) -> Shapes {
    let mut out = Shapes::default();
    for cs in &h.output().shapes {
        walk(&cs.shape, &mut out);
    }
    out
}

/// The 1 px #696969 thumbnail frames on the row of `label`, left to right.
fn thumb_frames(h: &H, label: &str) -> Vec<Rect> {
    let r = row(h, label).expect("row");
    let mut v: Vec<Rect> = shapes(h)
        .frames
        .into_iter()
        .filter(|(f, c, w)| {
            *c == THUMB_FRAME
                && *w == 1.0
                && f.right() <= r.left()
                && f.right() > r.left() - 120.0
                && f.y_range().contains(r.center().y)
        })
        .map(|(f, _, _)| f)
        .collect();
    v.sort_by(|a, b| a.left().total_cmp(&b.left()));
    v
}

/// A 10 × 10 badge (1 px black frame) bottom right inside `frame`?
fn badge_in(h: &H, frame: Rect) -> bool {
    shapes(h).frames.iter().any(|(r, c, w)| {
        *c == Color32::BLACK
            && *w == 1.0
            && (r.width() - 10.0).abs() < 0.01
            && (r.height() - 10.0).abs() < 0.01
            && frame.contains_rect(*r)
            && (r.right() - (frame.right() - 1.0)).abs() < 0.01
            && (r.bottom() - (frame.bottom() - 1.0)).abs() < 0.01
    })
}

#[test]
fn thumbnail_geometry_and_frame_colour_as_measured() {
    assert_eq!(
        thumb_frame_size(32),
        egui::vec2(34.0, 23.0),
        "default size: 34 × 23 frame"
    );
    assert_eq!(
        object_row_height(thumb_frame_size(32)),
        27.0,
        "27 px object rows (RE spec 73 §1)"
    );
    assert_eq!(thumb_frame_size(0), egui::Vec2::ZERO);
    assert_eq!(object_row_height(egui::Vec2::ZERO), 20.0);
    let (s, _) = session(false);
    let mut h = build(s, "en");
    ensure_layers(&mut h, "PI");
    for label in ["PI", "VM"] {
        let f = thumb_frames(&h, label);
        assert_eq!(
            f.len(),
            2,
            "{label}: content and mask thumbnail framed 1 px #696969: {f:?}"
        );
        for r in &f {
            assert_eq!(r.size(), egui::vec2(34.0, 23.0), "{label}: frame 34 × 23");
            // the image of the 400 × 300 page fitted into the 32 × 21 image area: 28 × 21
            // the smallest textured mesh in the frame (the checker fills the whole image area)
            let img = shapes(&h)
                .meshes
                .into_iter()
                .filter(|m| r.contains_rect(*m))
                .min_by(|a, b| a.area().total_cmp(&b.area()))
                .expect("thumbnail image");
            assert!(
                r.shrink(1.0).contains_rect(img),
                "{label}: image inside the frame {img:?}"
            );
            assert!(
                (img.height() - 21.0).abs() < 0.01 && (img.width() - 28.0).abs() < 0.01,
                "{label}: {img:?}"
            );
        }
    }
    // no thumbnail frame in the theme's list border colour any more
    let border = fp_ui::theme::Theme::current(&h.ctx).colors.list_border;
    assert!(
        !shapes(&h)
            .frames
            .iter()
            .any(|(r, c, _)| *c == border && r.size() == egui::vec2(34.0, 23.0)),
        "thumbnail frames are #696969"
    );
    // the object rows are 27 px apart
    let (pi, vm) = (row(&h, "PI").unwrap(), row(&h, "VM").unwrap());
    assert!(
        ((pi.center().y - vm.center().y).abs() - 27.0).abs() < 0.5,
        "{pi:?} {vm:?}"
    );
}

#[test]
fn bitmap_badge_on_bitmap_content_pen_badge_on_vector_masks() {
    let (s, _) = session(false);
    let mut h = build(s, "en");
    ensure_layers(&mut h, "PI");
    let pi = thumb_frames(&h, "PI");
    assert!(
        badge_in(&h, pi[0]),
        "Paste Inside: the bitmap content carries the bitmap badge"
    );
    assert!(!badge_in(&h, pi[1]), "a bitmap mask carries no badge");
    let vm = thumb_frames(&h, "VM");
    assert!(badge_in(&h, vm[0]), "masked bitmap content: bitmap badge");
    assert!(badge_in(&h, vm[1]), "vector mask: pen badge");
}

#[test]
fn masked_objects_inside_a_group_show_the_same_thumbnails() {
    let (s, pi) = session(true);
    let mut h = build(s, "en");
    ensure_layers(&mut h, "Grp");
    // open the group's tree (the "›" after its name); the nested masked object has both
    // thumbnails
    let g = row(&h, "Grp").expect("group row");
    click(&mut h, egui::pos2(g.right() + 9.0, g.center().y));
    h.run_steps(10);
    ensure_layers(&mut h, "PI");
    let f = thumb_frames(&h, "PI");
    assert_eq!(
        f.len(),
        2,
        "nested masked object: content and mask thumbnail {f:?}"
    );
    assert!(f.iter().all(|r| r.size() == egui::vec2(34.0, 23.0)));
    assert!(badge_in(&h, f[0]) && !badge_in(&h, f[1]));
    // the nested content thumbnail uses the subject of a top-level row (`thumbs::thumb_subject`):
    // underlay on in alpha channel mode → the masked result with the underlay
    let d = h.state().state.session.active().expect("doc");
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, fp_model::ElementId(pi))
        .expect("nested row");
    let sub = fp_ui::panels::thumbs::thumb_subject(&e);
    assert!(
        sub.element_mask().is_some_and(|m| m.enabled),
        "masked result shown, as on top level"
    );
}

fn inspector_tips(h: &mut H) -> Vec<String> {
    fp_ui::inspector::fields::set_audit(&h.ctx, true);
    let _ = fp_ui::inspector::fields::take_tips(&h.ctx);
    h.run_steps(1);
    let v = fp_ui::inspector::fields::take_tips(&h.ctx);
    fp_ui::inspector::fields::set_audit(&h.ctx, false);
    v.into_iter().map(|(t, _)| t).collect()
}

#[test]
fn bitmap_mask_quad_and_layers_opacity_say_what_they_do() {
    for lang in ["en", "de"] {
        let (mut s, pi) = session(false);
        ex(&mut s, "select.set", json!({"ids": [pi]}));
        ex(&mut s, "panel.layers.editMask", json!({}));
        let mut h = build(s, lang);
        ensure_layers(&mut h, "PI");
        // the measured rows, no number field in the bitmap mask inspector
        let panel = fp_ui::inspector::fields::panel_rect(&h.ctx).expect("Properties panel");
        let alpha_label = fp_i18n::t("pi-mask-alpha");
        let alpha = h
            .query_all(
                By::new()
                    .role(Role::RadioButton)
                    .label(alpha_label.as_str()),
            )
            .map(|n| n.rect())
            .find(|r| panel.contains(r.center()))
            .expect("Alpha channel");
        assert!(
            !h.query_all(By::new().role(Role::TextInput)).any(|n| {
                let r = n.rect();
                panel.contains(r.center())
                    && r.left() > alpha.left()
                    && (r.center().y - alpha.center().y).abs() < 30.0
            }),
            "{lang}: no number field next to the mask rows"
        );
        let tips = inspector_tips(&mut h);
        let tip = fp_i18n::t("pi-tip-bmask-alpha");
        assert!(
            tips.contains(&tip),
            "{lang}: Alpha channel tooltip in {tips:?}"
        );
        assert!(
            tip.contains(&fp_i18n::t("pi-mask-show-attrs")),
            "{lang}: the tooltip names the underlay option"
        );
        // the Layers panel's Opacity field: tooltip about the mask object's own opacity
        let field = h
            .query_all(By::new().role(Role::TextInput))
            .map(|n| n.rect())
            .find(|r| r.center().x > 1200.0 && r.center().y < 700.0 && r.width() < 40.0)
            .expect("Layers opacity field");
        h.input_mut()
            .events
            .push(Event::PointerMoved(field.center()));
        h.step();
        for _ in 0..60 {
            h.step();
        }
        let want = fp_i18n::t("panel-layers-opacity-tip-mask");
        assert!(
            h.query_by_label(&want).is_some(),
            "{lang}: Layers opacity tooltip shown"
        );
    }
}
