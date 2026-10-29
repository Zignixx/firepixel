//! TW-277 (FB-123 to FB-128, *RE spec* 75 part 2): opacity and blend mode of masks in the panels.
//! - With the mask (thumbnail) selected, the Layers panel and the Properties show and change the
//!   masked element's values (the single content, or the mask group), never the mask object's;
//!   with a child of a mask group selected, its own values.
//! - The mask object keeps its own values (also those from before masking), they never count and
//!   come back on Ungroup; a single content keeps its (changed) values; a mask group becomes a
//!   normal group with its values, a second Ungroup dissolves it.
//! - FB-123: the blend mode set on a mask group takes effect against the backdrop.

use egui::accesskit::Role;
use egui::{Event, Id, Modifiers, PointerButton, Pos2, Rect};
use egui_kittest::Harness;
use egui_kittest::kittest::{By, Queryable};
use fp_color::{BlendMode, Opacity};
use fp_engine::Session;
use fp_model::{Element, ElementId, ElementKind, GroupKind};
use fp_ui::FirepixelApp;
use fp_ui::app_state::UiSettings;
use serde_json::{Value, json};
use std::sync::Arc;

type H = Harness<'static, FirepixelApp>;

fn ex(s: &mut Session, c: &str, p: Value) -> Value {
    s.execute(c, p).unwrap_or_else(|e| panic!("{c}: {e}")).value
}

fn id_of(v: &Value) -> u64 {
    v["id"]
        .as_u64()
        .or_else(|| v["ids"][0].as_u64())
        .expect("id")
}

fn rect(s: &mut Session, r: [i32; 4], color: &str, op: f64, blend: &str) -> u64 {
    let id = id_of(&ex(
        s,
        "draw.rectangle",
        json!({"rect": r, "fillColor": color, "strokeWidth": 0}),
    ));
    ex(s, "element.setOpacity", json!({"ids": [id], "opacity": op}));
    ex(s, "element.setBlendMode", json!({"ids": [id], "mode": blend}));
    id
}

fn el(s: &Session, id: u64) -> Arc<Element> {
    let d = s.active().expect("doc");
    fp_engine::docops::element_on_page(&d.doc, d.page, ElementId(id)).expect("element")
}

fn vals(e: &Element) -> (Opacity, BlendMode) {
    (e.common.opacity, e.common.blend)
}

fn bm(name: &str) -> BlendMode {
    BlendMode::from_name(name).expect("blend mode")
}

fn op(p: f64) -> Opacity {
    Opacity::from_percent(p)
}

fn selection(s: &Session) -> Vec<u64> {
    s.active()
        .expect("doc")
        .selection
        .elements
        .iter()
        .map(|i| i.0)
        .collect()
}

/// What the Layers panel and the Properties show for the selection.
fn shown(s: &Session) -> (Option<Opacity>, Option<BlendMode>) {
    let d = s.active().expect("doc");
    fp_engine::docops::shown_opacity_blend(&d.doc, d.page, &d.selection.elements)
        .expect("something selected")
}

fn top_level(s: &Session) -> Vec<Arc<Element>> {
    let d = s.active().expect("doc");
    let page = &d.doc.pages[d.page];
    match &page.root.layers[0] {
        fp_model::LayerSlot::Own(l) => l.cels[0].elements.clone(),
        _ => panic!("own layer"),
    }
}

/// Backdrop #FF8000, content C #0080FF 70 % Multiply (and C2 #00C000), mask object M white 50 %
/// Screen on top, then Group as Mask. Returns (session, masked element, M, C, C2).
fn setup(two: bool) -> (Session, u64, u64, u64, Option<u64>) {
    fp_i18n::set_language("en");
    let mut s = Session::default();
    ex(
        &mut s,
        "file.new",
        json!({"width": 300, "height": 200, "background": "#ff8000"}),
    );
    let c = rect(&mut s, [20, 40, 200, 160], "#0080ff", 70.0, "multiply");
    let c2 = two.then(|| rect(&mut s, [120, 60, 280, 140], "#00c000", 100.0, "normal"));
    let m = rect(&mut s, [60, 30, 240, 170], "#ffffff", 50.0, "screen");
    let mut ids = vec![m];
    ids.extend(c2);
    ids.push(c);
    ex(&mut s, "select.set", json!({ "ids": ids }));
    let g = id_of(&ex(&mut s, "modify.mask.groupAsMask", json!({})));
    let g = {
        let d = s.active().expect("doc");
        fp_engine::docops::mask_owner(&d.doc, d.page, ElementId(g))
            .map(|o| o.id.0)
            .unwrap_or(g)
    };
    assert_eq!(
        el(&s, g).element_mask().map(|k| k.element.id.0),
        Some(m),
        "M is the mask"
    );
    (s, g, m, c, c2)
}

fn select_mask(s: &mut Session, g: u64, m: u64) {
    ex(s, "select.set", json!({"ids": [g]}));
    ex(s, "panel.layers.editMask", json!({}));
    assert_eq!(selection(s), vec![m], "the mask is selected");
}

fn page_px(s: &Session, x: i32, y: i32) -> [u8; 3] {
    let d = s.active().expect("doc");
    let r = fp_render::Renderer::new(fp_render::RenderOptions::default());
    let p = r.render_page(&d.doc, d.page, d.frame).image.pixel(x, y);
    [p.r, p.g, p.b]
}

#[test]
fn several_contents_get_a_new_group_and_the_mask_keeps_its_values() {
    let (s, g, m, c, c2) = setup(true);
    let e = el(&s, g);
    assert!(e.masked_single().is_none());
    assert_eq!(vals(&e), (op(100.0), BlendMode(0)), "new group 100 / Normal");
    assert_eq!(vals(&el(&s, m)), (op(50.0), bm("screen")), "M keeps its values");
    assert_eq!(vals(&el(&s, c)), (op(70.0), bm("multiply")));
    assert_eq!(vals(&el(&s, c2.expect("C2"))), (op(100.0), BlendMode(0)));
}

#[test]
fn mask_selected_shows_and_changes_the_mask_group() {
    let (mut s, g, m, c, _) = setup(true);
    select_mask(&mut s, g, m);
    // FB-127: the field shows the group's 100, not M's 50
    assert_eq!(shown(&s), (Some(op(100.0)), Some(BlendMode(0))));
    // FB-126 / FB-123: the input goes to the group
    ex(&mut s, "element.setOpacity", json!({"opacity": 60}));
    ex(&mut s, "element.setBlendMode", json!({"mode": "darken"}));
    assert_eq!(vals(&el(&s, g)), (op(60.0), bm("darken")));
    assert_eq!(vals(&el(&s, m)), (op(50.0), bm("screen")), "M unchanged");
    assert_eq!(vals(&el(&s, c)), (op(70.0), bm("multiply")), "children unchanged");
    assert_eq!(shown(&s), (Some(op(60.0)), Some(bm("darken"))));
    assert_eq!(selection(&s), vec![m], "the mask stays selected");
    // FB-123: the group's blend mode acts against the backdrop (measured `gam2b`)
    assert_eq!(page_px(&s, 80, 100), [148, 128, 0]);
    // a child selected: its own values
    ex(&mut s, "select.set", json!({"ids": [c]}));
    assert_eq!(shown(&s), (Some(op(70.0)), Some(bm("multiply"))));
    ex(&mut s, "element.setOpacity", json!({"opacity": 55}));
    assert_eq!(el(&s, c).common.opacity, op(55.0));
    assert_eq!(vals(&el(&s, g)), (op(60.0), bm("darken")), "group unchanged");
}

#[test]
fn single_content_is_the_masked_element() {
    let (mut s, g, m, c, _) = setup(false);
    let e = el(&s, g);
    assert!(e.masked_single().is_some(), "no group of its own");
    assert_eq!(e.placement(), (op(70.0), bm("multiply")), "C's values");
    // FB-125: C's blend mode applies to the masked result against the backdrop (`gam1`)
    assert_eq!(page_px(&s, 80, 100), [76, 83, 0]);
    select_mask(&mut s, g, m);
    assert_eq!(shown(&s), (Some(op(70.0)), Some(bm("multiply"))));
    ex(&mut s, "element.setOpacity", json!({"opacity": 40}));
    ex(&mut s, "element.setBlendMode", json!({"mode": "screen"}));
    assert_eq!(el(&s, g).placement(), (op(40.0), bm("screen")));
    assert_eq!(vals(&el(&s, m)), (op(50.0), bm("screen")), "M unchanged");
    // `set1`: C 40 % Screen with the mask
    assert_eq!(page_px(&s, 80, 100), [255, 154, 102]);
    // the content itself selected shows the same element
    ex(&mut s, "select.set", json!({"ids": [c]}));
    assert_eq!(shown(&s), (Some(op(40.0)), Some(bm("screen"))));
}

#[test]
fn ungroup_single_gives_the_mask_back_with_its_old_values() {
    let (mut s, g, m, c, _) = setup(false);
    select_mask(&mut s, g, m);
    ex(&mut s, "element.setOpacity", json!({"opacity": 40}));
    ex(&mut s, "select.set", json!({"ids": [g]}));
    ex(&mut s, "modify.ungroup", json!({}));
    let top = top_level(&s);
    assert_eq!(top.len(), 2);
    assert_eq!(top[0].id.0, m, "M on top");
    assert_eq!(vals(&top[0]), (op(50.0), bm("screen")), "M's old values");
    assert_eq!(top[1].id.0, c);
    assert_eq!(vals(&top[1]), (op(40.0), bm("multiply")), "C keeps the edited value");
    let mut sel = selection(&s);
    sel.sort();
    let mut both = vec![m, c];
    both.sort();
    assert_eq!(sel, both, "both selected");
    // M now shows (50 % Screen) over C (40 % Multiply over the backdrop: 153, 102, 0)
    assert_eq!(page_px(&s, 100, 100), [204, 179, 128]);
}

#[test]
fn ungroup_mask_group_leaves_a_normal_group_with_its_values() {
    let (mut s, g, m, c, c2) = setup(true);
    select_mask(&mut s, g, m);
    ex(&mut s, "element.setOpacity", json!({"opacity": 40}));
    ex(&mut s, "element.setBlendMode", json!({"mode": "screen"}));
    ex(&mut s, "select.set", json!({"ids": [g]}));
    ex(&mut s, "modify.ungroup", json!({}));
    let top = top_level(&s);
    assert_eq!(top.len(), 2);
    assert_eq!(top[0].id.0, m);
    assert_eq!(vals(&top[0]), (op(50.0), bm("screen")));
    let ElementKind::Group(gr) = &top[1].kind else {
        panic!("group stays")
    };
    assert_eq!(gr.kind, GroupKind::Normal, "a normal group");
    assert!(gr.mask.is_none());
    assert_eq!(gr.children.len(), 2);
    assert_eq!(vals(&top[1]), (op(40.0), bm("screen")), "group keeps its values");
    // the second Ungroup dissolves it
    ex(&mut s, "select.set", json!({"ids": [top[1].id.0]}));
    ex(&mut s, "modify.ungroup", json!({}));
    let ids: Vec<u64> = top_level(&s).iter().map(|e| e.id.0).collect();
    assert_eq!(ids, vec![m, c2.expect("C2"), c]);
    assert_eq!(vals(&el(&s, c)), (op(70.0), bm("multiply")));
}

#[test]
fn paste_inside_one_content_keeps_its_values_on_the_element() {
    fp_i18n::set_language("en");
    let mut s = Session::default();
    ex(
        &mut s,
        "file.new",
        json!({"width": 300, "height": 200, "background": "#ff8000"}),
    );
    let m = rect(&mut s, [60, 30, 240, 170], "#ffffff", 50.0, "screen");
    let c = rect(&mut s, [20, 40, 200, 160], "#0080ff", 70.0, "multiply");
    ex(&mut s, "select.set", json!({"ids": [c]}));
    ex(&mut s, "edit.cut", json!({}));
    ex(&mut s, "select.set", json!({"ids": [m]}));
    let g = id_of(&ex(&mut s, "edit.pasteInside", json!({})));
    let e = el(&s, g);
    assert!(e.masked_single().is_some());
    assert_eq!(e.placement(), (op(70.0), bm("multiply")));
    assert_eq!(
        e.element_mask().map(|k| vals(k.element)),
        Some((op(50.0), bm("screen"))),
        "the target keeps its values as mask"
    );
    // `pin1`: the white underlay multiplied with the backdrop at 70 % shows the backdrop
    assert_eq!(page_px(&s, 61, 31), [255, 128, 0]);
}

// ------------------------------------------------------------------ the panels themselves

fn build(s: Session) -> H {
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

fn rect_of(h: &H, id: Id) -> Option<Rect> {
    h.ctx.read_response(id).map(|r| r.rect)
}

fn find_text(h: &H, label: &str) -> Option<Pos2> {
    fn texts(shape: &egui::epaint::Shape, out: &mut Vec<(String, Rect)>) {
        match shape {
            egui::epaint::Shape::Text(t) => out.push((
                t.galley.job.text.clone(),
                Rect::from_min_size(t.pos, t.galley.size()),
            )),
            egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| texts(s, out)),
            _ => {}
        }
    }
    let mut all = Vec::new();
    for cs in &h.output().shapes {
        texts(&cs.shape, &mut all);
    }
    all.into_iter()
        .find(|(t, _)| t == label)
        .map(|(_, r)| r.center())
}

fn click(h: &mut H, p: Pos2) {
    h.event(Event::PointerMoved(p));
    h.run_steps(1);
    for pressed in [true, false] {
        h.event(Event::PointerButton {
            pos: p,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
        h.run_steps(1);
    }
    h.run_steps(2);
}

/// Value of the opacity field whose pop-up arrow has the id `base`.with("arrow").
fn opacity_field(h: &mut H, base: &str) -> String {
    let id = Id::new(base).with("arrow");
    if rect_of(h, id).is_none()
        && let Some(tab) = find_text(h, "Layers")
    {
        click(h, tab);
        h.run_steps(10);
    }
    let arrow = rect_of(h, id).unwrap_or_else(|| panic!("{base} arrow drawn"));
    h.get_all(By::new().role(Role::TextInput))
        .map(|n| (n.rect(), n.value().unwrap_or_default()))
        .filter(|(r, _)| {
            (r.center().y - arrow.center().y).abs() < 4.0
                && r.right() <= arrow.left() + 2.0
                && r.right() >= arrow.left() - 8.0
        })
        .map(|x| x.1)
        .next()
        .unwrap_or_else(|| panic!("{base} field"))
}

/// Shown text of the combo box with the id `id`.
fn combo_text(h: &H, id: &str) -> String {
    let r = rect_of(h, Id::new(id)).unwrap_or_else(|| panic!("{id} drawn"));
    h.get_all(By::new().role(Role::ComboBox))
        .find(|n| (n.rect().center() - r.center()).length() < 2.0)
        .and_then(|n| n.value())
        .unwrap_or_default()
}

#[test]
fn layers_panel_and_properties_show_the_masked_element_with_the_mask_selected() {
    let (mut s, g, m, _, _) = setup(true);
    select_mask(&mut s, g, m);
    let mut h = build(s);
    assert_eq!(opacity_field(&mut h, "layers-opacity"), "100", "Layers: the group, not M's 50");
    assert_eq!(opacity_field(&mut h, "pi-opacity"), "100", "Properties: the group");
    assert_eq!(combo_text(&h, "layers-blend"), "Normal");
    assert_eq!(combo_text(&h, "pi-blend"), "Normal", "not M's Screen");
    // a change made like the Layers panel does it: both fields follow (no stale Properties value)
    let s = &mut h.state_mut().state.session;
    ex(s, "element.setOpacity", json!({"opacity": 30}));
    ex(s, "element.setBlendMode", json!({"mode": "darken"}));
    h.run_steps(4);
    assert_eq!(opacity_field(&mut h, "layers-opacity"), "30");
    assert_eq!(opacity_field(&mut h, "pi-opacity"), "30");
    assert_eq!(combo_text(&h, "layers-blend"), "Darken");
    assert_eq!(combo_text(&h, "pi-blend"), "Darken");
    let s = &h.state().state.session;
    assert_eq!(vals(&el(s, m)), (op(50.0), bm("screen")), "M unchanged");
}
