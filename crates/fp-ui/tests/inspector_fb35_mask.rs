//! FB-35/FB-36 (RE spec 68 §2.1, §4.2): the inspector of a selected mask follows the kind of the
//! mask OBJECT, not the mode. A vector mask keeps "Vector mask", the radios Path outline /
//! Grayscale, "Show fill and stroke" and its fill controls in grayscale mode, and the way back
//! works; a bitmap mask shows "Bitmap mask" with Alpha channel / Grayscale. The painting tools'
//! mask sub-panel shows the real mode and switches it.

use egui::{Event, Modifiers, PointerButton, Pos2};
use egui::epaint::Shape;
use egui_kittest::Harness;
use egui_kittest::kittest::{By, NodeT, Queryable};
use fp_ui::FirepixelApp;
use fp_ui::app_state::UiSettings;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn painted_texts(h: &H) -> Vec<String> {
    fn walk(s: &Shape, out: &mut Vec<String>) {
        match s {
            Shape::Text(t) => out.push(t.galley.text().to_string()),
            Shape::Vec(v) => v.iter().for_each(|x| walk(x, out)),
            _ => {}
        }
    }
    let mut v = Vec::new();
    for cs in &h.output().shapes {
        walk(&cs.shape, &mut v);
    }
    v
}

fn id(v: &serde_json::Value) -> u64 {
    v["id"].as_u64().or_else(|| v["ids"][0].as_u64()).unwrap_or(0)
}

/// White square, mask above (red square or a pasted bitmap), Group as Mask (grayscale), then
/// Edit Mask: the mask is selected.
fn harness(bitmap_mask: bool, brush: bool) -> H {
    fp_engine::shell::disable();
    fp_i18n::set_language("de");
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 300, "height": 250, "background": "#ffffff"})).expect("doc");
    let c = id(&s.execute("draw.rectangle", json!({"rect": [0, 0, 100, 100], "fillColor": "#ffffff", "strokeWidth": 0})).expect("content").value);
    let m = if bitmap_mask {
        let img = fp_raster::RgbaImage::filled(fp_geom::IRect::from_xywh(10, 10, 50, 50), fp_color::Rgba8::new(255, 0, 0, 255)).expect("img");
        s.clipboard.clear();
        s.clipboard.pixels = Some(img);
        id(&s.execute("edit.paste", json!({})).expect("bitmap").value)
    } else {
        id(&s.execute("draw.rectangle", json!({"rect": [10, 10, 90, 90], "fillColor": "#ff0000", "strokeWidth": 0})).expect("mask").value)
    };
    s.execute("select.set", json!({"ids": [c, m]})).expect("select");
    s.execute("modify.mask.groupAsMask", json!({})).expect("group as mask");
    s.execute("panel.layers.editMask", json!({})).expect("edit mask");
    if brush {
        s.set_tool(fp_engine::ToolId::Brush).expect("brush");
    }
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(1800.0, 1130.0)).with_pixels_per_point(1.0).with_max_steps(20).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(5);
    h
}

fn layout(h: &H) -> Option<u16> {
    fp_ui::inspector::layouts::select_layout(&h.state().state.session).map(|l| l.number)
}

fn mask_kind(h: &H) -> fp_model::GroupKind {
    let d = h.state().state.session.active().expect("doc");
    let owner = fp_engine::docops::mask_owner(&d.doc, d.page, d.selection.elements[0]).expect("mask selected");
    match &owner.kind {
        fp_model::ElementKind::Group(g) => g.kind,
        _ => panic!("mask group"),
    }
}

/// Radio button with this label: (rect, selected).
fn radio(h: &H, label: &str) -> Option<(egui::Rect, bool)> {
    h.query_all(By::new().role(egui::accesskit::Role::RadioButton).label(label))
        .next()
        .map(|n| {
            let a = n.accesskit_node();
            (n.rect(), a.toggled() == Some(egui::accesskit::Toggled::True) || a.is_selected() == Some(true))
        })
}

fn click(h: &mut H, p: Pos2) {
    for e in [
        Event::PointerMoved(p),
        Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE },
        Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE },
    ] {
        h.event(e);
        h.run_steps(1);
    }
    h.run_steps(3);
}

fn undo_label(h: &H) -> Option<String> {
    h.state().state.session.active().and_then(|d| d.history.undo_label().map(str::to_string))
}

#[test]
fn vector_mask_keeps_its_inspector_in_grayscale_and_switches_back() {
    let mut h = harness(false, false);
    assert_eq!(mask_kind(&h), fp_model::GroupKind::BitmapMask, "Group as Mask = grayscale");
    assert_eq!(layout(&h), Some(19), "vector mask layout although the mode is grayscale");
    let texts = painted_texts(&h);
    for key in ["pi-mask-head-vector", "pi-mask-path-outline", "pi-mask-grayscale", "pi-mask-show-attrs"] {
        let t = fp_i18n::t(key);
        assert!(texts.contains(&t), "{key} ({t}) missing: {texts:?}");
    }
    assert!(!texts.iter().any(|x| *x == fp_i18n::t("pi-mask-head-bitmap")));
    let (gray, on) = radio(&h, &fp_i18n::t("pi-mask-grayscale")).expect("grayscale radio");
    assert!(on, "grayscale radio on");
    let _ = gray;
    let (path, on) = radio(&h, &fp_i18n::t("pi-mask-path-outline")).expect("path outline radio");
    assert!(!on);
    click(&mut h, path.left_center() + egui::vec2(6.0, 0.0));
    assert_eq!(mask_kind(&h), fp_model::GroupKind::VectorMask, "the way back");
    assert_eq!(undo_label(&h).as_deref(), Some("Maske modifizieren"));
    assert_eq!(layout(&h), Some(19));
    let (gray, _) = radio(&h, &fp_i18n::t("pi-mask-grayscale")).expect("grayscale radio still there");
    click(&mut h, gray.left_center() + egui::vec2(6.0, 0.0));
    assert_eq!(mask_kind(&h), fp_model::GroupKind::BitmapMask);
    assert_eq!(layout(&h), Some(19), "radios and fill controls stay");
    assert!(radio(&h, &fp_i18n::t("pi-mask-path-outline")).is_some());
}

#[test]
fn bitmap_mask_inspector_has_alpha_and_grayscale() {
    let mut h = harness(true, false);
    assert_eq!(layout(&h), Some(21), "bitmap mask layout");
    let texts = painted_texts(&h);
    assert!(texts.iter().any(|x| *x == fp_i18n::t("pi-mask-head-bitmap")), "{texts:?}");
    let (alpha, on) = radio(&h, &fp_i18n::t("pi-mask-alpha")).expect("alpha channel radio");
    assert!(!on);
    assert!(radio(&h, &fp_i18n::t("pi-mask-gray")).is_some_and(|(_, on)| on), "grayscale on");
    click(&mut h, alpha.left_center() + egui::vec2(6.0, 0.0));
    assert_eq!(mask_kind(&h), fp_model::GroupKind::VectorMask);
    assert_eq!(layout(&h), Some(21), "still the bitmap mask inspector");
    let (gray, _) = radio(&h, &fp_i18n::t("pi-mask-gray")).expect("grayscale radio still there");
    click(&mut h, gray.left_center() + egui::vec2(6.0, 0.0));
    assert_eq!(mask_kind(&h), fp_model::GroupKind::BitmapMask, "the way back");
}

#[test]
fn brush_sub_panel_shows_and_switches_the_mask_mode() {
    let mut h = harness(true, true);
    assert_eq!(h.state().state.session.tools.active, fp_engine::ToolId::Brush);
    let (alpha, on) = radio(&h, &fp_i18n::t("pi-mask-alpha")).expect("sub-panel alpha radio");
    assert!(!on, "the real mode: grayscale");
    assert!(radio(&h, &fp_i18n::t("pi-mask-gray")).is_some_and(|(_, on)| on));
    click(&mut h, alpha.left_center() + egui::vec2(6.0, 0.0));
    assert_eq!(mask_kind(&h), fp_model::GroupKind::VectorMask, "the sub-panel switches the mode");
    assert!(radio(&h, &fp_i18n::t("pi-mask-alpha")).is_some_and(|(_, on)| on));
}
