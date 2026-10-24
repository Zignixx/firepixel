//! FB-59 to FB-61 (Firepixel extension, tester proposal): expandable group and mask rows in the
//! Layers panel. A group row has a toggle "›"/"⌄" after its name that lists the group's objects
//! indented below (nested groups expand too); a click on such a row selects the object like the
//! Subselection tool (Shift adds or removes it); while an object of a group is subselected on the
//! canvas the group row is pale yellow. A masked object has a circled arrow that lists the
//! objects of its mask; a click on such a row selects the mask for editing like a click on the
//! mask thumbnail (objects of a group mask are subselected). Layers options ▸ Show Group
//! Contents (`panel.layers.groupTree`, default on) switches all of it off.

use egui::{Event, Key, Modifiers, PointerButton, Pos2, Rect};
use egui_kittest::Harness;
use fp_engine::tools::{ToolEvent, ToolId};
use fp_ui::FirepixelApp;
use fp_ui::app_state::UiSettings;
use serde_json::{Value, json};

type H = Harness<'static, FirepixelApp>;

fn ex(s: &mut fp_engine::Session, c: &str, p: Value) -> Value {
    s.execute(c, p).unwrap_or_else(|e| panic!("{c}: {e}")).value
}

/// Ids of the test document.
#[allow(dead_code)]
struct Ids {
    a: u64,
    n1: u64,
    n2: u64,
    inner: u64,
    grp: u64,
    x: u64,
    m: u64,
    mg: u64,
    bm: u64,
    bm_mask: u64,
    y: u64,
    m1: u64,
    m2: u64,
    gm_owner: u64,
}

/// Which test document: `Groups` = group "Grp" of group "Inner" (N1, N2) and "A" above "X"
/// masked by the path "M", without thumbnails (20 px rows, the whole tree fits the list);
/// `Masks` = "Y" masked by a group of "M1", "M2", "BM" with a bitmap mask, "X" masked by "M"
/// (thumbnails 24 px).
#[derive(Clone, Copy, PartialEq)]
enum Doc {
    Groups,
    Masks,
}

fn harness(doc: Doc) -> (H, Ids) {
    fp_i18n::set_language("de");
    let mut s = fp_engine::Session::default();
    ex(&mut s, "file.new", json!({"width": 700, "height": 400, "background": "#ffffff"}));
    let rect = |s: &mut fp_engine::Session, name: &str, x: f64, y: f64| -> u64 {
        let v = ex(s, "draw.rectangle", json!({"rect": [x, y, x + 50.0, y + 50.0]}));
        let id = v["id"].as_u64().expect("id");
        ex(s, "element.setName", json!({"ids": [id], "name": name}));
        id
    };
    let group = |s: &mut fp_engine::Session, ids: &[u64], name: &str| -> u64 {
        ex(s, "select.set", json!({ "ids": ids }));
        let g = ex(s, "modify.group", json!({}))["id"].as_u64().expect("group");
        ex(s, "element.setName", json!({"ids": [g], "name": name}));
        g
    };
    let x = rect(&mut s, "X", 300.0, 20.0);
    let m = rect(&mut s, "M", 310.0, 30.0);
    ex(&mut s, "select.set", json!({"ids": [x, m]}));
    let mg = ex(&mut s, "modify.mask.groupAsMask", json!({}))["id"].as_u64().expect("mask group");
    let mut ids = Ids { a: 0, n1: 0, n2: 0, inner: 0, grp: 0, x, m, mg, bm: 0, bm_mask: 0, y: 0, m1: 0, m2: 0, gm_owner: 0 };
    if doc == Doc::Groups {
        s.prefs.set("LayersList_ThumbnailSize", fp_engine::prefs::PrefValue::Int(0));
        ids.a = rect(&mut s, "A", 20.0, 20.0);
        ids.n1 = rect(&mut s, "N1", 100.0, 20.0);
        ids.n2 = rect(&mut s, "N2", 180.0, 20.0);
        ids.inner = group(&mut s, &[ids.n1, ids.n2], "Inner");
        ids.grp = group(&mut s, &[ids.inner, ids.a], "Grp");
    } else {
        let bm = rect(&mut s, "BM", 400.0, 20.0);
        ex(&mut s, "select.set", json!({"ids": [bm]}));
        let out = ex(&mut s, "panel.layers.addMask", json!({}));
        ids.bm = out["id"].as_u64().expect("owner");
        ids.bm_mask = out["mask"].as_u64().expect("mask");
        ids.y = rect(&mut s, "Y", 500.0, 20.0);
        ids.m1 = rect(&mut s, "M1", 505.0, 25.0);
        ids.m2 = rect(&mut s, "M2", 520.0, 40.0);
        let gm = group(&mut s, &[ids.m1, ids.m2], "GM");
        ex(&mut s, "select.set", json!({"ids": [ids.y, gm]}));
        ids.gm_owner = ex(&mut s, "modify.mask.groupAsMask", json!({}))["id"].as_u64().expect("mask group");
    }
    ex(&mut s, "select.none", json!({}));
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
    let first = if doc == Doc::Groups { "Grp" } else { "Y" };
    for _ in 0..2 {
        if find(&h, first).is_some() {
            break;
        }
        let tab = find(&h, "Ebenen").expect("Layers tab");
        click_at(&mut h, tab.center(), Modifiers::NONE);
        h.run_steps(30);
    }
    let rows: &[&str] = if doc == Doc::Groups { &["Grp", "X"] } else { &["Y", "BM", "X"] };
    for n in rows {
        assert!(find(&h, n).is_some(), "Layers panel shows row {n}: {:?}", panel_texts(&h));
    }
    (h, ids)
}

fn ev(h: &mut H, e: Event) {
    h.input_mut().events.push(e);
    h.step();
}

fn click_at(h: &mut H, p: Pos2, modifiers: Modifiers) {
    h.run_steps(30);
    h.input_mut().events.push(Event::ModifiersChanged(modifiers));
    ev(h, Event::PointerMoved(p));
    let e = |down| Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: down, modifiers };
    ev(h, e(true));
    ev(h, e(false));
    h.input_mut().events.push(Event::ModifiersChanged(Modifiers::NONE));
    ev(h, Event::PointerMoved(Pos2::new(5.0, 940.0)));
    h.run_steps(5);
}

fn double_click_at(h: &mut H, p: Pos2) {
    h.run_steps(45);
    ev(h, Event::PointerMoved(p));
    let e = |down| Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: down, modifiers: Modifiers::NONE };
    ev(h, e(true));
    ev(h, e(false));
    ev(h, e(true));
    ev(h, e(false));
    h.run_steps(3);
}

fn texts(shape: &egui::epaint::Shape, out: &mut Vec<(String, Rect)>) {
    match shape {
        egui::epaint::Shape::Text(t) => out.push((t.galley.job.text.clone(), Rect::from_min_size(t.pos, t.galley.size()))),
        egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| texts(s, out)),
        _ => {}
    }
}

fn panel_texts(h: &H) -> Vec<(String, Rect)> {
    let mut all = Vec::new();
    for cs in &h.output().shapes {
        texts(&cs.shape, &mut all);
    }
    all.retain(|(_, r)| r.center().x > 900.0);
    all
}

/// Text rect of a row label in the right part of the window (the Layers panel).
fn find(h: &H, label: &str) -> Option<Rect> {
    panel_texts(h).into_iter().filter(|(t, _)| t == label).map(|(_, r)| r).max_by(|a, b| a.center().x.total_cmp(&b.center().x))
}

fn row(h: &H, label: &str) -> Rect {
    find(h, label).unwrap_or_else(|| panic!("row {label}: {:?}", panel_texts(h)))
}

/// Click on the name of a row.
fn click(h: &mut H, label: &str, m: Modifiers) {
    let p = row(h, label).center();
    click_at(h, p, m);
}

/// Click on the group toggle "›" right after the name of a row.
fn click_group_toggle(h: &mut H, label: &str) {
    let r = row(h, label);
    click_at(h, Pos2::new(r.right() + 9.0, r.center().y), Modifiers::NONE);
}

/// Click on the circled mask arrow after the name of a row (`group`: the row has a "›" too).
fn click_mask_toggle(h: &mut H, label: &str, group: bool) {
    let r = row(h, label);
    let x = r.right() + 4.0 + if group { 14.0 } else { 0.0 } + 7.0;
    click_at(h, Pos2::new(x, r.center().y), Modifiers::NONE);
}

fn fills(shape: &egui::epaint::Shape, col: egui::Color32, out: &mut Vec<Rect>) {
    match shape {
        egui::epaint::Shape::Rect(r) if r.fill == col => out.push(r.rect),
        egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| fills(s, col, out)),
        _ => {}
    }
}

fn row_filled(h: &H, label: &str, col: egui::Color32) -> bool {
    let p = row(h, label).center();
    let mut rs = Vec::new();
    for cs in &h.output().shapes {
        fills(&cs.shape, col, &mut rs);
    }
    rs.iter().any(|r| r.contains(p) && r.width() > 100.0)
}

fn blue(h: &H, label: &str) -> bool {
    row_filled(h, label, fp_ui::theme::Theme::current(&h.ctx).colors.list_selected_bg)
}

fn yellow(h: &H, label: &str) -> bool {
    row_filled(h, label, fp_ui::panels::layers::group_child_selected_bg(&fp_ui::theme::Theme::current(&h.ctx)))
}

fn selection(h: &H) -> Vec<u64> {
    let mut v: Vec<u64> = h.state().state.session.active().expect("d").selection.elements.iter().map(|e| e.0).collect();
    v.sort();
    v
}

fn undo_count(h: &H) -> usize {
    h.state().state.session.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

/// A click with the Subselection tool on the canvas at document point (x, y), through the engine
/// like the canvas does.
fn subselect_on_canvas(h: &mut H, x: f32, y: f32, shift: bool) {
    let s = &mut h.state_mut().state.session;
    s.set_tool(ToolId::Subselection).expect("subselection");
    let (mut d, mut u) = (ToolEvent::down(x, y), ToolEvent::up(x, y));
    d.mods.shift = shift;
    u.mods.shift = shift;
    s.tool_event(&d).expect("down");
    s.tool_event(&u).expect("up");
    s.set_tool(ToolId::Pointer).expect("pointer");
    h.run_steps(5);
}

#[test]
fn group_rows_expand_and_collapse_with_nested_groups() {
    let (mut h, _) = harness(Doc::Groups);
    assert!(find(&h, "A").is_none() && find(&h, "Inner").is_none(), "collapsed at first");
    click_group_toggle(&mut h, "Grp");
    assert!(find(&h, "A").is_some() && find(&h, "Inner").is_some(), "children listed: {:?}", panel_texts(&h));
    assert!(find(&h, "N1").is_none(), "the nested group stays collapsed");
    // indented one step, below the group row
    let (g, inner, a) = (row(&h, "Grp"), row(&h, "Inner"), row(&h, "A"));
    assert!(inner.left() >= g.left() + fp_ui::panels::layers::TREE_INDENT - 0.5, "{g:?} {inner:?}");
    assert!(inner.top() > g.top() && a.top() > inner.top(), "Inner above A, both below Grp");
    click_group_toggle(&mut h, "Inner");
    let n1 = row(&h, "N1");
    assert!(n1.left() >= inner.left() + fp_ui::panels::layers::TREE_INDENT - 0.5, "two levels deep");
    assert!(find(&h, "N2").is_some());
    // collapsing the group hides the whole tree; the nested group keeps its own state
    click_group_toggle(&mut h, "Grp");
    for n in ["A", "Inner", "N1", "N2"] {
        assert!(find(&h, n).is_none(), "{n} hidden after collapsing");
    }
    assert!(selection(&h).is_empty(), "the toggle selects nothing");
    let n0 = undo_count(&h);
    click_group_toggle(&mut h, "Grp");
    assert!(find(&h, "N1").is_some(), "Inner still open");
    assert_eq!(undo_count(&h), n0, "expanding is no History step");
}

#[test]
fn child_row_click_selects_like_the_subselection_tool() {
    let (mut h, ids) = harness(Doc::Groups);
    // the reference state: a Subselection click on A on the canvas
    subselect_on_canvas(&mut h, 45.0, 45.0, false);
    let canvas = selection(&h);
    assert_eq!(canvas, vec![ids.a], "the Subselection picks A out of the groups");
    h.state_mut().state.session.execute("select.none", json!({})).expect("none");
    h.run_steps(3);
    click_group_toggle(&mut h, "Grp");
    let n0 = undo_count(&h);
    click(&mut h, "A", Modifiers::NONE);
    assert_eq!(selection(&h), canvas, "row click = Subselection click");
    assert_eq!(undo_count(&h), n0, "like a canvas click: no History row");
    assert!(blue(&h, "A"), "the child row is highlighted");
    assert!(yellow(&h, "Grp"), "the group row turns yellow");
    assert!(!blue(&h, "Grp"));
    // Shift adds and removes children like Shift+click with the Subselection
    click(&mut h, "Inner", Modifiers::SHIFT);
    let mut want = vec![ids.a, ids.inner];
    want.sort();
    assert_eq!(selection(&h), want);
    click_group_toggle(&mut h, "Inner");
    click(&mut h, "N2", Modifiers::SHIFT);
    click(&mut h, "A", Modifiers::SHIFT);
    let mut want = vec![ids.inner, ids.n2];
    want.sort();
    assert_eq!(selection(&h), want, "A removed again, N2 added");
    assert!(blue(&h, "Inner") && yellow(&h, "Grp"), "Inner itself selected, Grp holds selected children");
    // a plain click selects only that child
    click(&mut h, "N1", Modifiers::NONE);
    assert_eq!(selection(&h), vec![ids.n1]);
    assert!(yellow(&h, "Inner") && yellow(&h, "Grp"), "every group above the child is yellow");
    // the inspector follows the selection like after a canvas Subselection click
    let d = h.state().state.session.active().expect("d");
    assert_eq!(d.selection.elements.first().map(|e| e.0), Some(ids.n1));
}

#[test]
fn canvas_subselection_tints_the_collapsed_group_row_yellow() {
    let (mut h, ids) = harness(Doc::Groups);
    assert!(!yellow(&h, "Grp"));
    subselect_on_canvas(&mut h, 45.0, 45.0, false);
    assert_eq!(selection(&h), vec![ids.a]);
    assert!(yellow(&h, "Grp"), "group row pale yellow while its child is subselected");
    assert!(find(&h, "A").is_none(), "the group stays collapsed");
    click_group_toggle(&mut h, "Grp");
    assert!(blue(&h, "A"), "expanded: the child row is blue");
    assert!(yellow(&h, "Grp"));
    // selecting the group itself: blue, not yellow
    h.state_mut().state.session.execute("select.set", json!({"ids": [ids.grp]})).expect("sel");
    h.run_steps(5);
    assert!(blue(&h, "Grp") && !yellow(&h, "Grp"));
}

#[test]
fn mask_member_rows_select_the_mask_for_editing() {
    let (mut h, ids) = harness(Doc::Masks);
    // vector mask of one path: the circled arrow lists it, a click edits the mask
    assert!(find(&h, "M").is_none());
    click_mask_toggle(&mut h, "X", false);
    let (x, m) = (row(&h, "X"), row(&h, "M"));
    assert!(m.top() > x.top() && m.left() >= x.left() - 40.0, "mask row below the object");
    click(&mut h, "M", Modifiers::NONE);
    assert_eq!(selection(&h), vec![ids.m], "the mask is selected for editing");
    let top = h.state().state.session.active().and_then(|d| d.history.undo_label()).unwrap_or_default();
    assert_eq!(top, fp_i18n::t("msg-step-mask-modify"), "one History row like the mask thumbnail");
    click_mask_toggle(&mut h, "X", false);
    assert!(find(&h, "M").is_none(), "collapsed again");
    // bitmap mask: one row "Bitmap-Maske", a click goes into bitmap mask editing
    click_mask_toggle(&mut h, "BM", false);
    click(&mut h, "Bitmap-Maske", Modifiers::NONE);
    assert_eq!(selection(&h), vec![ids.bm_mask]);
    assert!(h.state().state.session.active().expect("d").bitmap_mode, "bitmap mask editing");
    click_mask_toggle(&mut h, "BM", false);
    h.state_mut().state.session.execute("select.none", json!({})).expect("none");
    h.run_steps(3);
    // a group as mask: one row per object; a click edits the mask and subselects the object
    click_mask_toggle(&mut h, "Y", false);
    assert!(find(&h, "M1").is_some() && find(&h, "M2").is_some(), "{:?}", panel_texts(&h));
    click(&mut h, "M2", Modifiers::NONE);
    assert_eq!(selection(&h), vec![ids.m2], "member of the group mask subselected");
    click(&mut h, "M1", Modifiers::SHIFT);
    let mut want = vec![ids.m1, ids.m2];
    want.sort();
    assert_eq!(selection(&h), want, "Shift adds the other member");
    // collapsing again removes the rows
    click_mask_toggle(&mut h, "Y", false);
    assert!(find(&h, "M1").is_none());
}

#[test]
fn switch_off_shows_the_rows_of_the_reference() {
    let (mut h, ids) = harness(Doc::Groups);
    // the panel menu carries the switch, checked
    let nodes = fp_ui::menu::build_panel_menu(&h.state().state, fp_ui::panels::PanelId::Layers);
    let found = nodes.iter().any(|n| matches!(n, fp_ui::menu::Node::Item { action: fp_ui::menu::ItemAction::Command(c), checked: Some(true), .. } if c == "panel.layers.groupTree"));
    assert!(found, "Layers options: Show Group Contents, checked");
    click_group_toggle(&mut h, "Grp");
    assert!(find(&h, "A").is_some());
    h.state_mut().state.run("panel.layers.groupTree", json!({}));
    h.run_steps(5);
    assert!(find(&h, "A").is_none(), "off: no child rows");
    // no toggle: a click where "›" was selects the group like any row click
    click_group_toggle(&mut h, "Grp");
    assert_eq!(selection(&h), vec![ids.grp]);
    assert!(find(&h, "A").is_none());
    // no yellow for a subselected child
    subselect_on_canvas(&mut h, 45.0, 45.0, false);
    assert_eq!(selection(&h), vec![ids.a]);
    assert!(!yellow(&h, "Grp") && !blue(&h, "Grp"), "off: the group row looks as before");
    // RE spec 70 section 4.2: no object row is marked, only the layer row (light tint, filled dot)
    assert!(!blue(&h, "X"));
    let light = fp_ui::theme::Theme::current(&h.ctx).colors.list_selected_light_bg;
    assert!(row_filled(&h, "Ebene 1", light), "layer row tinted light");
    assert!(dot_filled(&h, "Ebene 1"), "layer row shows the selection dot");
    // no mask arrow either
    click_mask_toggle(&mut h, "X", false);
    assert!(find(&h, "M").is_none());
    // on again: the expansion state was kept
    h.state_mut().state.run("panel.layers.groupTree", json!({}));
    h.run_steps(5);
    assert!(find(&h, "A").is_some(), "on again: Grp still expanded");
}

#[test]
fn child_rows_rename_and_hide() {
    let (mut h, ids) = harness(Doc::Groups);
    click_group_toggle(&mut h, "Grp");
    let p = row(&h, "A").center();
    double_click_at(&mut h, p);
    assert!(h.ctx.text_edit_focused(), "rename field open on the child row");
    ev(&mut h, Event::Key { key: Key::A, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::COMMAND });
    ev(&mut h, Event::Text("Alpha".into()));
    ev(&mut h, Event::Key { key: Key::Enter, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE });
    ev(&mut h, Event::Key { key: Key::Enter, physical_key: None, pressed: false, repeat: false, modifiers: Modifiers::NONE });
    h.run_steps(5);
    let d = h.state().state.session.active().expect("d");
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, fp_model::ElementId(ids.a)).expect("A");
    assert_eq!(e.common.name.as_deref(), Some("Alpha"));
    assert!(find(&h, "Alpha").is_some());
    // the eye of a child row hides that child only
    let r = row(&h, "Alpha");
    let eye = Pos2::new(row_left(&h, "Alpha") + 10.0, r.center().y);
    click_at(&mut h, eye, Modifiers::NONE);
    let d = h.state().state.session.active().expect("d");
    let vis = |id: u64| fp_engine::docops::element_on_page(&d.doc, d.page, fp_model::ElementId(id)).expect("el").common.visible;
    assert!(!vis(ids.a), "A hidden");
    assert!(vis(ids.grp) && vis(ids.inner), "the group and the sibling stay visible");
}

/// Left edge of the list row holding the label (the row background spans the list width).
fn row_left(h: &H, label: &str) -> f32 {
    let p = row(h, label).center();
    let mut best = f32::MAX;
    let col = fp_ui::theme::Theme::current(&h.ctx).colors.list_row_line;
    fn lines(shape: &egui::epaint::Shape, col: egui::Color32, out: &mut Vec<Rect>) {
        match shape {
            egui::epaint::Shape::LineSegment { points, stroke } if stroke.color == col => out.push(Rect::from_two_pos(points[0], points[1])),
            egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| lines(s, col, out)),
            _ => {}
        }
    }
    let mut ls = Vec::new();
    for cs in &h.output().shapes {
        lines(&cs.shape, col, &mut ls);
    }
    // the horizontal row line under the label's row
    for r in ls {
        if r.height() < 1.0 && r.width() > 100.0 && r.min.y > p.y && r.min.y - p.y < 40.0 && r.left() < p.x {
            best = best.min(r.left());
        }
    }
    best
}

/// Is there a small filled circle (the selection dot) in the row of the label?
fn dot_filled(h: &H, label: &str) -> bool {
    let r = row(h, label);
    fn circles(shape: &egui::epaint::Shape, out: &mut Vec<(Pos2, f32)>) {
        match shape {
            egui::epaint::Shape::Circle(c) if c.fill.a() > 0 => out.push((c.center, c.radius)),
            egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| circles(s, out)),
            _ => {}
        }
    }
    let mut cs = Vec::new();
    for c in &h.output().shapes {
        circles(&c.shape, &mut cs);
    }
    cs.iter().any(|(p, rad)| *rad < 3.0 && p.x > r.right() && (p.y - r.center().y).abs() < 6.0)
}

/// Rect outlines in the colour `col` around the point `p`.
fn framed(h: &H, p: Pos2, col: egui::Color32) -> bool {
    fn strokes(shape: &egui::epaint::Shape, col: egui::Color32, out: &mut Vec<Rect>) {
        match shape {
            egui::epaint::Shape::Rect(r) if r.stroke.color == col && r.stroke.width > 0.0 => out.push(r.rect),
            egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| strokes(s, col, out)),
            _ => {}
        }
    }
    let mut rs = Vec::new();
    for c in &h.output().shapes {
        strokes(&c.shape, col, &mut rs);
    }
    rs.iter().any(|r| r.contains(p) && r.width() < 40.0)
}

/// RE spec 70 section 4.3 with the switch off: a click on the mask thumbnail edits the mask, the
/// row is highlighted, the mask thumbnail framed green, no extra rows; a click on the name
/// selects the object and frames its own thumbnail blue.
#[test]
fn switch_off_mask_thumbnail_edits_like_the_reference() {
    let (mut h, ids) = harness(Doc::Masks);
    h.state_mut().state.run("panel.layers.groupTree", json!({}));
    h.run_steps(5);
    // name click: the object, its thumbnail framed blue
    click(&mut h, "X", Modifiers::NONE);
    assert_eq!(selection(&h), vec![ids.mg]);
    let name = row(&h, "X");
    // thumbnails 24 px: mask thumbnail ends 6 px before the name, the object thumbnail 12 px
    // (chain) before the mask thumbnail
    let mask_thumb = Pos2::new(name.left() - 6.0 - 12.0, name.center().y);
    let obj_thumb = Pos2::new(name.left() - 6.0 - 24.0 - 12.0 - 12.0, name.center().y);
    let focus = fp_ui::theme::Theme::current(&h.ctx).colors.focus;
    assert!(blue(&h, "X") && framed(&h, obj_thumb, focus), "object thumbnail framed blue");
    // mask thumbnail click: mask editing
    click_at(&mut h, mask_thumb, Modifiers::NONE);
    assert_eq!(selection(&h), vec![ids.m], "the mask path is selected");
    assert!(blue(&h, "X"), "the row stays highlighted while its mask is edited");
    assert!(framed(&h, mask_thumb, egui::Color32::from_rgb(0x33, 0xE0, 0x33)), "mask thumbnail framed green");
    assert!(!framed(&h, obj_thumb, focus), "the object thumbnail is not framed blue then");
    assert!(find(&h, "M").is_none(), "no extra rows");
}
