//! FB-35 / FB-36, RE spec 68 sections 7 and 8 (M-10, M-11): Disable Mask swaps its menu text to
//! "Enable Mask" (no check mark) and the Layers panel draws a red X over the mask thumbnail; a click
//! on the mask thumbnail selects the mask like Edit Mask (bright green frame, mask icon); Delete
//! Mask asks "Discard / Cancel / Apply" (Apply the default) and each button does its part; the
//! inspector shows the Firepixel-own "Invert" checkbox for a selected mask.

use egui::accesskit::Role;
use egui::{Color32, Event, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::{By, Queryable};
use fp_engine::tools::ToolId;
use fp_model::{ElementKind, GroupKind};
use fp_ui::FirepixelApp;
use fp_ui::app_state::UiSettings;
use fp_ui::menu::{ItemAction, Node};
use serde_json::{Value, json};

type H = Harness<'static, FirepixelApp>;

const RED_X: Color32 = Color32::from_rgb(0xE0, 0x20, 0x20);
const GREEN_FRAME: Color32 = Color32::from_rgb(0x33, 0xE0, 0x33);

fn ex(s: &mut fp_engine::Session, c: &str, p: Value) -> Value {
    s.execute(c, p).unwrap_or_else(|e| panic!("{c}: {e}")).value
}

fn id_of(v: &Value) -> u64 {
    v["id"].as_u64().expect("id")
}

/// Red bitmap "B1" (100,100 100 × 100) masked by a square path 120..180 (path outline); the masked
/// element is selected. Returns the harness and the ids (masked element, mask, bitmap).
fn harness() -> (H, u64, u64, u64) {
    fp_i18n::set_language("de");
    let mut s = fp_engine::Session::default();
    ex(&mut s, "file.new", json!({"width": 400, "height": 300, "background": "#ffffff"}));
    let img = fp_raster::RgbaImage::filled(fp_geom::IRect::from_xywh(100, 100, 100, 100), fp_color::Rgba8::new(255, 0, 0, 255)).expect("image");
    s.clipboard.clear();
    s.clipboard.pixels = Some(img);
    let b = ex(&mut s, "edit.paste", json!({}))["ids"][0].as_u64().expect("bitmap");
    ex(&mut s, "element.setName", json!({"ids": [b], "name": "B1"}));
    let m = id_of(&ex(&mut s, "draw.path", json!({"contours": [{"closed": true, "nodes": [[120, 120], [180, 120], [180, 180], [120, 180]]}]})));
    ex(&mut s, "select.set", json!({"ids": [m, b]}));
    let g = id_of(&ex(&mut s, "modify.mask.groupAsMask", json!({})));
    ex(&mut s, "modify.mask.options", json!({"mode": "path", "showAttrs": false}));
    s.set_tool(ToolId::Pointer).expect("pointer");
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.os_clipboard = false;
    app.state.sync_render = true;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1500.0, 950.0))
        .with_pixels_per_point(1.0)
        .with_max_steps(8)
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    for _ in 0..2 {
        if find(&h, "B1").is_some() {
            break;
        }
        if let Some(tab) = find(&h, "Ebenen") {
            click(&mut h, tab);
        }
        h.run_steps(30);
    }
    assert!(find(&h, "B1").is_some(), "Layers panel shows the masked element");
    (h, g, m, b)
}

fn ev(h: &mut H, e: Event) {
    h.input_mut().events.push(e);
    h.step();
}

fn click(h: &mut H, p: Pos2) {
    h.run_steps(30);
    ev(h, Event::PointerMoved(p));
    let e = |down| Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: down, modifiers: Modifiers::NONE };
    ev(h, e(true));
    ev(h, e(false));
    ev(h, Event::PointerMoved(Pos2::new(5.0, 940.0)));
    h.run_steps(5);
}

fn texts(shape: &egui::epaint::Shape, out: &mut Vec<(String, egui::Rect)>) {
    match shape {
        egui::epaint::Shape::Text(t) => out.push((t.galley.job.text.clone(), egui::Rect::from_min_size(t.pos, t.galley.size()))),
        egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| texts(s, out)),
        _ => {}
    }
}

fn text_rect(h: &H, label: &str) -> Option<egui::Rect> {
    let mut all = Vec::new();
    for cs in &h.output().shapes {
        texts(&cs.shape, &mut all);
    }
    all.into_iter().filter(|(t, r)| t == label && r.center().x > 900.0).map(|(_, r)| r).max_by(|a, b| a.center().x.total_cmp(&b.center().x))
}

/// Text in the right part of the window (the Layers panel), rightmost match.
fn find(h: &H, label: &str) -> Option<Pos2> {
    text_rect(h, label).map(|r| r.center())
}

/// Shapes of a colour: line segments (red X) and rectangle frames (green frame).
fn count_colored(shape: &egui::epaint::Shape, col: Color32, n: &mut usize) {
    match shape {
        egui::epaint::Shape::LineSegment { stroke, .. } if stroke.color == col => *n += 1,
        egui::epaint::Shape::Rect(r) if r.stroke.color == col && r.stroke.width > 0.0 => *n += 1,
        egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| count_colored(s, col, n)),
        _ => {}
    }
}

fn colored(h: &H, col: Color32) -> usize {
    let mut n = 0;
    for cs in &h.output().shapes {
        count_colored(&cs.shape, col, &mut n);
    }
    n
}

fn selection(h: &H) -> Vec<u64> {
    h.state().state.session.active().expect("d").selection.elements.iter().map(|e| e.0).collect()
}

fn find_item<'a>(nodes: &'a [Node], cmd: &str) -> Option<&'a Node> {
    for n in nodes {
        match n {
            Node::Item { action: ItemAction::Command(c), .. } if c == cmd => return Some(n),
            Node::Sub { children, .. } => {
                if let Some(x) = find_item(children, cmd) {
                    return Some(x);
                }
            }
            _ => {}
        }
    }
    None
}

/// (label, checked) of a main-menu entry.
fn main_item(h: &H, cmd: &str) -> (String, Option<bool>) {
    let menus = fp_ui::menu::build_main_menu(&h.state().state);
    for (_, nodes) in &menus {
        if let Some(Node::Item { label, checked, .. }) = find_item(nodes, cmd) {
            return (label.clone(), *checked);
        }
    }
    panic!("{cmd} not in the main menu")
}

fn layers_item(h: &H, cmd: &str) -> String {
    let nodes = fp_ui::panels::common::resource_menu_nodes(&h.state().state, "6923");
    match find_item(&nodes, cmd) {
        Some(Node::Item { label, .. }) => label.clone(),
        _ => panic!("{cmd} not in the Layers context menu"),
    }
}

fn mask_enabled(h: &H, g: u64) -> bool {
    let d = h.state().state.session.active().expect("d");
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, fp_model::ElementId(g)).expect("element");
    matches!(&e.kind, ElementKind::Group(gr) if gr.mask_flags.enabled)
}

fn run(h: &mut H, cmd: &str, p: Value) {
    h.state_mut().state.run(cmd, p);
    h.run_steps(6);
}

#[test]
fn disable_mask_swaps_the_menu_text_and_shows_the_red_x() {
    let (mut h, g, ..) = harness();
    assert_eq!(main_item(&h, "modify.mask.disableMask"), ("Maske deaktivieren".to_string(), None));
    assert_eq!(layers_item(&h, "panel.layers.disableMask"), "Maske deaktivieren");
    assert_eq!(colored(&h, RED_X), 0, "no X while the mask is on");
    run(&mut h, "modify.mask.disableMask", json!({}));
    assert!(!mask_enabled(&h, g));
    assert_eq!(main_item(&h, "modify.mask.disableMask"), ("Maske aktivieren".to_string(), None), "text swaps, no check mark");
    assert_eq!(layers_item(&h, "panel.layers.disableMask"), "Maske aktivieren");
    h.run_steps(10);
    assert_eq!(colored(&h, RED_X), 2, "red X (two strokes) over the mask thumbnail");
    run(&mut h, "modify.mask.disableMask", json!({}));
    assert!(mask_enabled(&h, g));
    assert_eq!(main_item(&h, "modify.mask.disableMask").0, "Maske deaktivieren");
    h.run_steps(10);
    assert_eq!(colored(&h, RED_X), 0);
}

#[test]
fn mask_thumbnail_click_selects_the_mask_with_green_frame() {
    let (mut h, g, m, _) = harness();
    assert_eq!(selection(&h), vec![g]);
    assert_eq!(colored(&h, GREEN_FRAME), 0);
    let rows_before = h.state().state.session.active().expect("d").history.undo_count();
    // the mask thumbnail is the thumbnail directly left of the row name
    let name = text_rect(&h, "B1").expect("row");
    let thumb = Pos2::new(name.left() - 6.0 - 8.0, name.center().y);
    click(&mut h, thumb);
    assert_eq!(selection(&h), vec![m], "the mask thumbnail selects the mask");
    let rows_after = h.state().state.session.active().expect("d").history.undo_count();
    assert_eq!(rows_after, rows_before + 1, "one History row (the mask-select command)");
    h.run_steps(10);
    assert!(colored(&h, GREEN_FRAME) >= 1, "bright green frame around the mask thumbnail");
}

fn button(h: &H, label: &str) -> Option<egui::Rect> {
    h.query_all(By::new().role(Role::Button).label(label)).map(|n| n.rect()).next()
}

fn open_delete_prompt(h: &mut H) {
    run(h, "modify.mask.deleteMask", json!({}));
    let c = h.state().state.dialogs.choice.clone().expect("Delete Mask asks first");
    let labels: Vec<&str> = c.buttons.iter().map(|b| b.0.as_str()).collect();
    assert_eq!(labels, ["Verwerfen", "Abbrechen", "Anwenden"]);
    assert_eq!(c.default, 2, "Apply is the default");
    assert_eq!(c.text, fp_i18n::t("dlg-mask-delete-bitmap"));
    h.run_steps(6);
}

fn tops(h: &H) -> Vec<std::sync::Arc<fp_model::Element>> {
    let d = h.state().state.session.active().expect("d");
    let mut out = Vec::new();
    for (_, l) in fp_engine::docops::own_layers(&d.doc, d.page).into_iter().rev() {
        if let Some(c) = l.cels.get(d.frame) {
            out.extend(c.elements.iter().cloned());
        }
    }
    out
}

#[test]
fn delete_mask_prompt_cancel_discard_apply() {
    let (mut h, g, _, b) = harness();
    // Cancel: nothing happens
    open_delete_prompt(&mut h);
    let r = button(&h, "Abbrechen").expect("Cancel button");
    click(&mut h, r.center());
    assert!(h.state().state.dialogs.choice.is_none());
    assert_eq!(tops(&h).iter().map(|e| e.id.0).collect::<Vec<_>>(), vec![g]);
    // Discard: the mask goes, the bitmap stays
    open_delete_prompt(&mut h);
    let r = button(&h, "Verwerfen").expect("Discard button");
    click(&mut h, r.center());
    assert!(h.state().state.dialogs.choice.is_none());
    assert_eq!(tops(&h).iter().map(|e| e.id.0).collect::<Vec<_>>(), vec![b]);
    // Apply (after undo): one bitmap with the mask baked in
    h.state_mut().state.session.undo().expect("undo");
    h.run_steps(4);
    assert_eq!(tops(&h)[0].id.0, g);
    open_delete_prompt(&mut h);
    let r = button(&h, "Anwenden").expect("Apply button");
    click(&mut h, r.center());
    assert!(h.state().state.dialogs.choice.is_none());
    let t = tops(&h);
    assert_eq!(t.len(), 1);
    let ElementKind::Bitmap(bm) = &t[0].kind else { panic!("Apply gives a bitmap") };
    let img = fp_engine::docops::bitmap_image(bm).expect("image");
    let r = img.bounds();
    assert_eq!((r.left, r.top, r.right, r.bottom), (120, 120, 180, 180));
    assert_eq!(t[0].common.name.as_deref(), Some("B1"), "keeps the content's name");
}

#[test]
fn inspector_invert_checkbox_for_a_selected_mask() {
    let (mut h, g, m, _) = harness();
    run(&mut h, "select.set", json!({"ids": [m]}));
    h.run_steps(10);
    let label = fp_i18n::t("pi-mask-invert");
    let n = h.query_all(By::new().label(label.as_str())).count();
    assert!(n >= 1, "Invert checkbox in the mask inspector");
    run(&mut h, "modify.mask.options", json!({"invert": true}));
    let d = h.state().state.session.active().expect("d");
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, fp_model::ElementId(g)).expect("element");
    assert!(matches!(&e.kind, ElementKind::Group(gr) if gr.mask_flags.invert && gr.kind == GroupKind::VectorMask));
}

fn screen_of(h: &H, x: f32, y: f32) -> Pos2 {
    let st = &h.state().state;
    let d = st.session.active().expect("doc");
    let area = st.canvases.get(&d.id).and_then(|c| c.area).expect("canvas");
    let (sx, sy) = d.view.doc_to_screen(fp_geom::Point::new(x, y));
    Pos2::new(area.min.x + sx as f32, area.min.y + sy as f32)
}

/// Click on the canvas at document point (x, y) with `mods` held.
fn canvas_click(h: &mut H, x: f32, y: f32, mods: Modifiers) {
    // well past the double-click time of the previous click
    h.run_steps(40);
    let p = screen_of(h, x, y);
    ev(h, Event::ModifiersChanged(mods));
    ev(h, Event::PointerMoved(p));
    let e = |down| Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: down, modifiers: mods };
    ev(h, e(true));
    ev(h, e(false));
    ev(h, Event::ModifiersChanged(Modifiers::NONE));
    h.run_steps(6);
}

fn set_tool(h: &mut H, t: ToolId) {
    h.state_mut().state.session.set_tool(t).expect("tool");
    h.run_steps(3);
}

#[test]
fn canvas_clicks_select_the_mask_with_subselection_and_pointer_alt() {
    let (mut h, g, m, _) = harness();
    run(&mut h, "select.none", json!({}));
    set_tool(&mut h, ToolId::Subselection);
    canvas_click(&mut h, 150.0, 150.0, Modifiers::NONE);
    assert_eq!(selection(&h), vec![m], "Subselection click selects the mask");
    set_tool(&mut h, ToolId::Pointer);
    canvas_click(&mut h, 150.0, 150.0, Modifiers::NONE);
    assert_eq!(selection(&h), vec![g], "a plain Pointer click selects the masked element");
    canvas_click(&mut h, 150.0, 150.0, Modifiers::ALT);
    assert_eq!(selection(&h), vec![m], "Pointer + Alt-click selects the mask");
    set_tool(&mut h, ToolId::SelectBehind);
    for _ in 0..3 {
        canvas_click(&mut h, 150.0, 150.0, Modifiers::NONE);
        assert!(!selection(&h).contains(&m), "Select Behind never picks the mask");
    }
}
