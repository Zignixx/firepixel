//! FB-146 (TW-297, Firepixel extension): a masked object selected as a whole keeps the measured
//! inspector of the masked element and additionally offers the options of its mask in a column of
//! its own right of Q5/Q6 (mode radios incl. the Firepixel "Pixel Alpha" of FB-151, Show fill and
//! stroke, Invert), bound to `modify.mask.options`. RE spec 81 §6 D5: the mask rows of the Layers
//! group tree show the mask inspector of the row's kind (a child of a group mask by its own kind),
//! the content rows the object's own inspector, never the Document panel. FB-151 (TW-298): the
//! third mode radio "Pixel Alpha" in the vector and the bitmap mask inspector.

use egui::accesskit::Role;
use egui::{Event, Modifiers, PointerButton, Pos2, Rect};
use egui_kittest::Harness;
use egui_kittest::kittest::{By, NodeT, Queryable};
use fp_engine::tools::ToolId;
use fp_model::{ElementKind, GroupKind};
use fp_ui::FirepixelApp;
use fp_ui::app_state::UiSettings;
use serde_json::{Value, json};

type H = Harness<'static, FirepixelApp>;

fn ex(s: &mut fp_engine::Session, c: &str, p: Value) -> Value {
    s.execute(c, p).unwrap_or_else(|e| panic!("{c}: {e}")).value
}

fn id_of(v: &Value) -> u64 {
    v["id"].as_u64().or_else(|| v["ids"][0].as_u64()).unwrap_or(0)
}

fn filled(s: &mut fp_engine::Session, r: [i32; 4], color: &str, name: &str) -> u64 {
    let id = id_of(&ex(
        s,
        "draw.rectangle",
        json!({"rect": r, "fillColor": color, "strokeWidth": 0}),
    ));
    ex(s, "element.setName", json!({"ids": [id], "name": name}));
    id
}

fn bitmap(s: &mut fp_engine::Session, l: i32, t: i32, w: i32, h: i32, name: &str) -> u64 {
    let img = fp_raster::RgbaImage::filled(
        fp_geom::IRect::from_xywh(l, t, w, h),
        fp_color::Rgba8::new(0, 0, 255, 255),
    )
    .expect("image");
    s.clipboard.clear();
    s.clipboard.pixels = Some(img);
    let id = id_of(&ex(s, "edit.paste", json!({})));
    ex(s, "element.setName", json!({"ids": [id], "name": name}));
    id
}

fn build(s: fp_engine::Session) -> H {
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1800.0, 1130.0))
        .with_pixels_per_point(1.0)
        .with_max_steps(20)
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(5);
    h
}

fn layout(h: &H) -> (u16, u16) {
    let l = fp_ui::inspector::layouts::select_layout(&h.state().state.session).expect("layout");
    (l.number, l.quads[0])
}

/// Control with this role and label: (rect, on).
fn control(h: &H, role: Role, label: &str) -> Option<(Rect, bool)> {
    h.query_all(By::new().role(role).label(label)).next().map(|n| {
        let a = n.accesskit_node();
        (
            n.rect(),
            a.toggled() == Some(egui::accesskit::Toggled::True) || a.is_selected() == Some(true),
        )
    })
}

fn click_at(h: &mut H, p: Pos2, modifiers: Modifiers) {
    h.run_steps(30);
    for e in [
        Event::PointerMoved(p),
        Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers },
        Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers },
    ] {
        h.input_mut().events.push(e);
        h.step();
    }
    h.input_mut().events.push(Event::PointerMoved(Pos2::new(5.0, 1120.0)));
    h.run_steps(5);
}

fn click_control(h: &mut H, role: Role, label: &str) {
    let (r, _) = control(h, role, label).unwrap_or_else(|| panic!("{label} missing"));
    click_at(h, r.left_center() + egui::vec2(6.0, 0.0), Modifiers::NONE);
}

fn group(h: &H, id: u64) -> (GroupKind, bool, bool, bool) {
    let d = h.state().state.session.active().expect("doc");
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, fp_model::ElementId(id)).expect("group");
    match &e.kind {
        ElementKind::Group(g) => (g.kind, g.mask_flags.pixel_alpha, g.mask_flags.hide_attrs, g.mask_flags.invert),
        _ => panic!("not a group"),
    }
}

fn undo_label(h: &H) -> Option<String> {
    h.state().state.session.active().and_then(|d| d.history.undo_label().map(str::to_string))
}

/// Paste Inside: path mask M with the contents C and C2 (underlay on); the mask group selected.
fn paste_inside_scene() -> (fp_engine::Session, u64) {
    fp_engine::shell::disable();
    fp_i18n::set_language("de");
    let mut s = fp_engine::Session::default();
    ex(&mut s, "file.new", json!({"width": 300, "height": 200, "background": "#ffffff"}));
    let m = filled(&mut s, [10, 10, 290, 190], "#ffffff", "M");
    let c = filled(&mut s, [20, 40, 200, 160], "#0080ff", "C");
    let c2 = filled(&mut s, [120, 60, 280, 140], "#00c000", "C2");
    ex(&mut s, "select.set", json!({"ids": [c, c2]}));
    ex(&mut s, "edit.cut", json!({}));
    ex(&mut s, "select.set", json!({"ids": [m]}));
    let g = id_of(&ex(&mut s, "edit.pasteInside", json!({})));
    ex(&mut s, "select.set", json!({"ids": [g]}));
    s.set_tool(ToolId::Pointer).expect("pointer");
    (s, g)
}

#[test]
fn whole_mask_group_shows_its_mask_options_beside_the_measured_inspector() {
    let (s, g) = paste_inside_scene();
    let mut h = build(s);
    let (number, q1) = layout(&h);
    assert_eq!(q1, 14101, "object layout of the masked element (layout {number})");
    let t = fp_i18n::t;
    let (path, on) = control(&h, Role::RadioButton, &t("pi-mask-path-outline")).expect("Path outline");
    assert!(on, "Paste Inside: path outline");
    let (gray, _) = control(&h, Role::RadioButton, &t("pi-mask-grayscale")).expect("Grayscale");
    let (pixel, off) = control(&h, Role::RadioButton, &t("pi-mask-pixel-alpha")).expect("Pixel Alpha");
    assert!(!off);
    let (show, shown) = control(&h, Role::CheckBox, &t("pi-mask-show-attrs")).expect("Show fill and stroke");
    assert!(shown, "Paste Inside: underlay on");
    let (inv, _) = control(&h, Role::CheckBox, &t("pi-mask-invert")).expect("Invert");
    // a column of its own right of Q5/Q6 (2 + 126 + 590 + 180 px from the panel's left edge)
    let min_x = 2.0 + 126.0 + 590.0 + 180.0;
    for (w, r) in [("path", path), ("gray", gray), ("pixel", pixel), ("show", show), ("invert", inv)] {
        assert!(r.left() >= min_x, "{w} {r:?} right of Q5/Q6");
    }
    assert!(path.top() < gray.top() && gray.top() < pixel.top(), "radios in order");
    assert!(pixel.bottom() <= show.top() + 0.5 && show.top() < inv.top(), "second row below");
    // the controls act on the selected mask group: one History row each
    click_control(&mut h, Role::RadioButton, &t("pi-mask-grayscale"));
    assert_eq!(group(&h, g).0, GroupKind::BitmapMask);
    assert_eq!(undo_label(&h).as_deref(), Some("Maske modifizieren"));
    assert_eq!(layout(&h).1, 14101, "the masked element stays selected");
    click_control(&mut h, Role::RadioButton, &t("pi-mask-pixel-alpha"));
    assert_eq!(group(&h, g), (GroupKind::VectorMask, true, false, false), "Pixel Alpha");
    click_control(&mut h, Role::RadioButton, &t("pi-mask-path-outline"));
    assert_eq!(group(&h, g), (GroupKind::VectorMask, false, false, false));
    click_control(&mut h, Role::CheckBox, &t("pi-mask-show-attrs"));
    assert!(group(&h, g).2, "Show fill and stroke off");
    click_control(&mut h, Role::CheckBox, &t("pi-mask-invert"));
    assert!(group(&h, g).3, "inverted");
    let sel: Vec<u64> = h.state().state.session.active().expect("d").selection.elements.iter().map(|e| e.0).collect();
    assert_eq!(sel, vec![g], "selection unchanged");
    // nothing selected or a plain object: no column
    h.state_mut().state.session.execute("select.none", json!({})).expect("none");
    h.run_steps(4);
    assert!(control(&h, Role::RadioButton, &t("pi-mask-pixel-alpha")).is_none());
}

#[test]
fn masked_bitmap_column_has_alpha_channel_and_no_show_box() {
    fp_engine::shell::disable();
    fp_i18n::set_language("en");
    let mut s = fp_engine::Session::default();
    ex(&mut s, "file.new", json!({"width": 300, "height": 200, "background": "#ffffff"}));
    let c = filled(&mut s, [20, 20, 120, 120], "#ff0000", "C");
    let b = bitmap(&mut s, 40, 40, 60, 60, "B");
    ex(&mut s, "select.set", json!({"ids": [c]}));
    ex(&mut s, "edit.cut", json!({}));
    ex(&mut s, "select.set", json!({"ids": [b]}));
    let g = id_of(&ex(&mut s, "edit.pasteInside", json!({})));
    ex(&mut s, "select.set", json!({"ids": [g]}));
    s.set_tool(ToolId::Pointer).expect("pointer");
    let mut h = build(s);
    let t = fp_i18n::t;
    assert_eq!(layout(&h).1, 14101);
    let (_, on) = control(&h, Role::RadioButton, &t("pi-mask-alpha")).expect("Alpha channel");
    assert!(on);
    assert!(control(&h, Role::RadioButton, &t("pi-mask-gray")).is_some());
    assert!(control(&h, Role::CheckBox, &t("pi-mask-show-attrs")).is_none(), "like the bitmap mask inspector");
    assert!(control(&h, Role::CheckBox, &t("pi-mask-invert")).is_some());
    click_control(&mut h, Role::RadioButton, &t("pi-mask-pixel-alpha"));
    assert_eq!(group(&h, g).0, GroupKind::VectorMask);
    assert!(group(&h, g).1, "Pixel Alpha on the bitmap mask");
    h.state_mut().state.session.undo().expect("undo");
    h.run_steps(4);
    assert!(!group(&h, g).1, "undo");
}

#[test]
fn mask_inspectors_offer_pixel_alpha() {
    // vector mask selected (mask thumbnail way): the radio below Grayscale in the column of the
    // mode radios (F165; FB-151 had put it above Path outline)
    let (mut s, _g) = paste_inside_scene();
    ex(&mut s, "panel.layers.editMask", json!({}));
    let mut h = build(s);
    let t = fp_i18n::t;
    assert_eq!(layout(&h).0, 19, "vector mask inspector");
    let (pixel, _) = control(&h, Role::RadioButton, &t("pi-mask-pixel-alpha")).expect("Pixel Alpha");
    let (gray, _) = control(&h, Role::RadioButton, &t("pi-mask-grayscale")).expect("Grayscale");
    assert!(gray.bottom() <= pixel.top() + 0.5 && (pixel.left() - gray.left()).abs() < 0.5, "{pixel:?} {gray:?}");
    click_control(&mut h, Role::RadioButton, &t("pi-mask-pixel-alpha"));
    assert_eq!(
        fp_ui::inspector::obj_quads::selected_mask_mode_ui(&h.state().state),
        Some(fp_ui::inspector::obj_quads::MaskModeUi::PixelAlpha)
    );
    let (_, on) = control(&h, Role::RadioButton, &t("pi-mask-path-outline")).expect("Path outline");
    assert!(!on, "path outline off in Pixel Alpha");
    // the column of FB-146 is not shown while the mask is selected
    assert_eq!(h.query_all(By::new().role(Role::RadioButton).label(t("pi-mask-pixel-alpha").as_str())).count(), 1);
    // bitmap mask (Reveal All on a bitmap): beside Alpha channel
    fp_i18n::set_language("de");
    let mut s = fp_engine::Session::default();
    ex(&mut s, "file.new", json!({"width": 300, "height": 200, "background": "#ffffff"}));
    let b = bitmap(&mut s, 40, 40, 60, 60, "B");
    ex(&mut s, "select.set", json!({"ids": [b]}));
    ex(&mut s, "modify.mask.revealAll", json!({}));
    s.set_tool(ToolId::Pointer).expect("pointer");
    let mut h = build(s);
    assert_eq!(layout(&h).0, 21, "bitmap mask inspector");
    let (alpha, _) = control(&h, Role::RadioButton, &t("pi-mask-alpha")).expect("Alpha channel");
    let (pixel, _) = control(&h, Role::RadioButton, &t("pi-mask-pixel-alpha")).expect("Pixel Alpha");
    let (inv, _) = control(&h, Role::CheckBox, &t("pi-mask-invert")).expect("Invert");
    assert!(pixel.left() >= alpha.right() && (pixel.center().y - alpha.center().y).abs() < 1.0);
    assert!(inv.left() >= pixel.right(), "Invert further right");
    click_control(&mut h, Role::RadioButton, &t("pi-mask-pixel-alpha"));
    assert_eq!(
        fp_ui::inspector::obj_quads::selected_mask_mode_ui(&h.state().state),
        Some(fp_ui::inspector::obj_quads::MaskModeUi::PixelAlpha)
    );
    assert_eq!(undo_label(&h).as_deref(), Some("Maske modifizieren"));
}

// ------------------------------------------------------------ D5: rows of the group tree

fn texts(shape: &egui::epaint::Shape, out: &mut Vec<(String, Rect)>) {
    match shape {
        egui::epaint::Shape::Text(t) => out.push((t.galley.job.text.clone(), Rect::from_min_size(t.pos, t.galley.size()))),
        egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| texts(s, out)),
        _ => {}
    }
}

/// Rows with this label in the Layers panel (right part of the window), top to bottom.
fn rows_of(h: &H, label: &str) -> Vec<Rect> {
    let mut all = Vec::new();
    for cs in &h.output().shapes {
        texts(&cs.shape, &mut all);
    }
    let mut v: Vec<Rect> = all.into_iter().filter(|(t, r)| t == label && r.center().x > 1300.0 && r.center().y < 900.0).map(|(_, r)| r).collect();
    v.sort_by(|a, b| a.top().total_cmp(&b.top()));
    v
}

fn row(h: &H, label: &str) -> Rect {
    rows_of(h, label).first().copied().unwrap_or_else(|| panic!("row {label}"))
}

fn click_mask_toggle(h: &mut H, label: &str) {
    let r = row(h, label);
    click_at(h, Pos2::new(r.right() + 11.0, r.center().y), Modifiers::NONE);
}

fn head_texts(h: &H) -> Vec<String> {
    let mut all = Vec::new();
    for cs in &h.output().shapes {
        texts(&cs.shape, &mut all);
    }
    all.into_iter().map(|(t, _)| t).collect()
}

#[test]
fn mask_rows_of_the_group_tree_show_the_matching_inspector() {
    fp_engine::shell::disable();
    fp_i18n::set_language("de");
    let mut s = fp_engine::Session::default();
    ex(&mut s, "file.new", json!({"width": 700, "height": 400, "background": "#ffffff"}));
    // X masked by the path M (Group as Mask)
    let x = filled(&mut s, [300, 20, 350, 70], "#ff0000", "X");
    let m = filled(&mut s, [310, 30, 360, 80], "#000000", "M");
    ex(&mut s, "select.set", json!({"ids": [x, m]}));
    ex(&mut s, "modify.mask.groupAsMask", json!({}));
    // Y masked by the group GM of the path M1 and the bitmap B1
    let y = filled(&mut s, [500, 20, 600, 120], "#ffd000", "Y");
    let m1 = filled(&mut s, [505, 25, 540, 60], "#ff0000", "M1");
    let b1 = bitmap(&mut s, 550, 30, 40, 40, "B1");
    ex(&mut s, "select.set", json!({"ids": [m1, b1]}));
    let gm = id_of(&ex(&mut s, "modify.group", json!({})));
    ex(&mut s, "element.setName", json!({"ids": [gm], "name": "GM"}));
    ex(&mut s, "select.set", json!({"ids": [y, gm]}));
    ex(&mut s, "modify.mask.groupAsMask", json!({}));
    ex(&mut s, "select.none", json!({}));
    s.set_tool(ToolId::Pointer).expect("pointer");
    let mut h = build(s);
    for _ in 0..2 {
        if !rows_of(&h, "Y").is_empty() {
            break;
        }
        if let Some(tab) = rows_of(&h, "Ebenen").first().copied() {
            click_at(&mut h, tab.center(), Modifiers::NONE);
        }
        h.run_steps(30);
    }
    assert!(!rows_of(&h, "Y").is_empty() && !rows_of(&h, "X").is_empty(), "Layers rows");
    let t = fp_i18n::t;
    // single path mask: the mask row → vector mask inspector with its options
    click_mask_toggle(&mut h, "X");
    let p = row(&h, "M").center();
    click_at(&mut h, p, Modifiers::NONE);
    assert_eq!(layout(&h).0, 19, "M: vector mask inspector");
    assert!(control(&h, Role::RadioButton, &t("pi-mask-path-outline")).is_some());
    // the content row → the object's own inspector, never the Document panel
    let xs = rows_of(&h, "X");
    assert_eq!(xs.len(), 2);
    click_at(&mut h, xs[1].center(), Modifiers::NONE);
    assert_eq!(layout(&h).1, 14101, "content row: object inspector");
    click_mask_toggle(&mut h, "X");
    // group mask: a path member → vector mask inspector, a bitmap member → bitmap mask inspector
    click_mask_toggle(&mut h, "Y");
    let p = row(&h, "M1").center();
    click_at(&mut h, p, Modifiers::NONE);
    assert_eq!(layout(&h).0, 19, "M1: vector mask inspector");
    assert!(head_texts(&h).contains(&t("pi-mask-head-vector")));
    let p = row(&h, "B1").center();
    click_at(&mut h, p, Modifiers::NONE);
    assert_eq!(layout(&h).0, 21, "B1: bitmap mask inspector of its own kind");
    assert!(head_texts(&h).contains(&t("pi-mask-head-bitmap")));
    // its options are those of Y's mask (Group as Mask: grayscale)
    let (_, gray) = control(&h, Role::RadioButton, &t("pi-mask-gray")).expect("Grayscale");
    assert!(gray, "the mode of Y's mask");
    click_control(&mut h, Role::RadioButton, &t("pi-mask-alpha"));
    let d = h.state().state.session.active().expect("d");
    let owner = fp_engine::docops::mask_owner_of(&d.doc, d.page, fp_model::ElementId(b1)).expect("owner");
    assert!(owner.element_mask().is_some_and(|m| m.to_path), "Y's mask switched to alpha channel");
    let ys = rows_of(&h, "Y");
    assert_eq!(ys.len(), 2);
    click_at(&mut h, ys[1].center(), Modifiers::NONE);
    assert_eq!(layout(&h).1, 14101, "content row Y: object inspector");
}

// ------------------------------------------------------------ F165: layout of the mask rows

/// Audit entries of one frame (label, check box and radio texts of the inspector).
fn audit(h: &mut H) -> Vec<fp_ui::inspector::fields::TextAudit> {
    fp_ui::inspector::fields::set_audit(&h.ctx, true);
    h.run_steps(2);
    let v = fp_ui::inspector::fields::take_audit(&h.ctx);
    fp_ui::inspector::fields::set_audit(&h.ctx, false);
    v
}

fn text_entry<'a>(a: &'a [fp_ui::inspector::fields::TextAudit], text: &str) -> &'a fp_ui::inspector::fields::TextAudit {
    a.iter().rfind(|e| e.text == text).unwrap_or_else(|| panic!("{text:?} not drawn"))
}

/// The text is drawn whole (not shrunk or cut) and inside its quad / column.
fn assert_whole(a: &[fp_ui::inspector::fields::TextAudit], text: &str) {
    let e = text_entry(a, text);
    assert_eq!(e.fit, 0, "{text:?} shown whole");
    assert!(e.drawn.right() <= e.clip.right() + 0.5 && e.drawn.left() >= e.clip.left() - 0.5, "{text:?}: {:?} inside {:?}", e.drawn, e.clip);
    assert!(e.drawn.bottom() <= e.clip.bottom() + 0.5 && e.drawn.top() >= e.clip.top() - 0.5, "{text:?}: {:?} inside {:?}", e.drawn, e.clip);
}

/// "Mask:" first, then Path outline, Grayscale, Pixel Alpha, Show fill and stroke, Invert, one
/// below the other in one column, without overlap; the label ends with a gap before the radios.
fn assert_mask_rows(h: &mut H, what: &str) {
    let t = fp_i18n::t;
    let a = audit(h);
    let label = text_entry(&a, &t("pi-mask")).drawn;
    let (path, _) = control(h, Role::RadioButton, &t("pi-mask-path-outline")).expect("Path outline");
    let (gray, _) = control(h, Role::RadioButton, &t("pi-mask-grayscale")).expect("Grayscale");
    let (pixel, _) = control(h, Role::RadioButton, &t("pi-mask-pixel-alpha")).expect("Pixel Alpha");
    let (show, _) = control(h, Role::CheckBox, &t("pi-mask-show-attrs")).expect("Show fill and stroke");
    let (inv, _) = control(h, Role::CheckBox, &t("pi-mask-invert")).expect("Invert");
    assert!((label.center().y - path.center().y).abs() < 2.0, "{what}: Mask: in the row of Path outline");
    assert!(label.right() + 4.0 <= path.left(), "{what}: gap between {label:?} and {path:?}");
    let rows = [path, gray, pixel, show, inv];
    for w in rows.windows(2) {
        assert!(w[0].bottom() <= w[1].top() + 0.5, "{what}: order and no overlap {:?} {:?}", w[0], w[1]);
    }
    assert!((path.left() - gray.left()).abs() < 0.5 && (gray.left() - pixel.left()).abs() < 0.5, "{what}: radios in one column");
    assert!((show.left() - inv.left()).abs() < 0.5 && show.left() < path.left(), "{what}: check boxes below");
    for key in ["pi-mask", "pi-mask-path-outline", "pi-mask-grayscale", "pi-mask-pixel-alpha", "pi-mask-show-attrs", "pi-mask-invert"] {
        assert_whole(&a, &t(key));
    }
}

#[test]
fn f165_mask_rows_read_in_order_and_german_texts_fit() {
    // whole mask group: the column right of Q5/Q6
    let (s, _g) = paste_inside_scene();
    let mut h = build(s);
    assert_mask_rows(&mut h, "column");
    // vector mask selected: Q5 / Q6 of layout 19, same arrangement
    let (mut s, _g) = paste_inside_scene();
    ex(&mut s, "panel.layers.editMask", json!({}));
    let mut h = build(s);
    assert_eq!(layout(&h).0, 19, "vector mask inspector");
    assert_mask_rows(&mut h, "layout 19");
    // the measured quads Q3/Q4 stay where they are: Q5 begins 2 + 126 + 590 px right of the panel
    let (path, _) = control(&h, Role::RadioButton, &fp_i18n::t("pi-mask-path-outline")).expect("Path outline");
    let panel = fp_ui::inspector::fields::panel_rect(&h.ctx).expect("panel");
    assert!(path.left() >= panel.left() + 2.0 + 126.0 + 590.0, "{path:?}");
}

#[test]
fn f165_bitmap_mask_label_keeps_a_gap() {
    fp_engine::shell::disable();
    fp_i18n::set_language("de");
    let t = fp_i18n::t;
    let mut s = fp_engine::Session::default();
    ex(&mut s, "file.new", json!({"width": 300, "height": 200, "background": "#ffffff"}));
    let b = bitmap(&mut s, 40, 40, 60, 60, "B");
    ex(&mut s, "select.set", json!({"ids": [b]}));
    ex(&mut s, "modify.mask.revealAll", json!({}));
    s.set_tool(ToolId::Pointer).expect("pointer");
    let mut h = build(s);
    assert_eq!(layout(&h).0, 21, "bitmap mask inspector");
    let a = audit(&mut h);
    let label = text_entry(&a, &t("pi-mask")).drawn;
    let (alpha, _) = control(&h, Role::RadioButton, &t("pi-mask-alpha")).expect("Alpha channel");
    assert!(label.right() + 4.0 <= alpha.left(), "gap between {label:?} and {alpha:?}");
    assert_whole(&a, &t("pi-mask"));
    // the column of a masked bitmap selected as a whole (Paste Inside onto a bitmap): Alpha
    // channel, Grayscale, Pixel Alpha below "Maske:" with a gap, then Invert
    let mut s = fp_engine::Session::default();
    ex(&mut s, "file.new", json!({"width": 300, "height": 200, "background": "#ffffff"}));
    let c = filled(&mut s, [20, 20, 120, 120], "#ff0000", "C");
    let b = bitmap(&mut s, 40, 40, 60, 60, "B");
    ex(&mut s, "select.set", json!({"ids": [c]}));
    ex(&mut s, "edit.cut", json!({}));
    ex(&mut s, "select.set", json!({"ids": [b]}));
    let g = id_of(&ex(&mut s, "edit.pasteInside", json!({})));
    ex(&mut s, "select.set", json!({"ids": [g]}));
    s.set_tool(ToolId::Pointer).expect("pointer");
    let mut h = build(s);
    assert_eq!(layout(&h).1, 14101);
    let a = audit(&mut h);
    let label = text_entry(&a, &t("pi-mask")).drawn;
    let (alpha, _) = control(&h, Role::RadioButton, &t("pi-mask-alpha")).expect("Alpha channel");
    let (gray, _) = control(&h, Role::RadioButton, &t("pi-mask-gray")).expect("Grayscale");
    let (pixel, _) = control(&h, Role::RadioButton, &t("pi-mask-pixel-alpha")).expect("Pixel Alpha");
    let (inv, _) = control(&h, Role::CheckBox, &t("pi-mask-invert")).expect("Invert");
    assert!(label.right() + 4.0 <= alpha.left(), "column: gap between {label:?} and {alpha:?}");
    for w in [alpha, gray, pixel, inv].windows(2) {
        assert!(w[0].bottom() <= w[1].top() + 0.5, "column: order {:?} {:?}", w[0], w[1]);
    }
    for key in ["pi-mask", "pi-mask-alpha", "pi-mask-gray", "pi-mask-pixel-alpha", "pi-mask-invert"] {
        assert_whole(&a, &t(key));
    }
}
