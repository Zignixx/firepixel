//! F159 (FB-143, TW-299; RE spec 82 §1): dragging object rows in the Layers panel with real
//! pointer events. Zones: the upper 6 px of an object row = before it, the rest = after it, a
//! layer row = top of that layer; a drop that changes nothing shows no band and writes no History
//! row; a selected row moves the whole selection as a block; the band is 5 px over the list
//! width, the target layer's folder is highlighted and the drag cursor replaces the system
//! cursor. Firepixel extension (§1.5): rows of an open group take drops into the group (band
//! indented), its objects can be dragged out.

use egui::{Event, Modifiers, PointerButton, Pos2, Rect};
use egui_kittest::Harness;
use fp_engine::tools::ToolId;
use fp_model::{ElementId, ElementKind, LayerId};
use fp_ui::FirepixelApp;
use fp_ui::app_state::UiSettings;
use serde_json::{Value, json};
use std::collections::HashMap;

type H = Harness<'static, FirepixelApp>;

fn ex(s: &mut fp_engine::Session, c: &str, p: Value) -> Value {
    s.execute(c, p).unwrap_or_else(|e| panic!("{c}: {e}")).value
}

struct Scene {
    l1: u64,
    l2: u64,
    ids: HashMap<&'static str, u64>,
}

/// Scene `LD()` of RE spec 82 §1.1: "Ebene 1" top to bottom C, B, G (G2 over G1), A; "Ebene 2"
/// above it with D.
fn harness() -> (H, Scene) {
    fp_i18n::set_language("de");
    let mut s = fp_engine::Session::default();
    s.prefs
        .set("MaxUndos", fp_engine::prefs::PrefValue::Int(100));
    // no thumbnails: short rows, the whole tree fits the list
    s.prefs
        .set("LayersList_ThumbnailSize", fp_engine::prefs::PrefValue::Int(0));
    ex(
        &mut s,
        "file.new",
        json!({"width": 600, "height": 400, "background": "#ffffff"}),
    );
    let l1 = s.active().and_then(|d| d.layer).expect("layer").0;
    let rect = |s: &mut fp_engine::Session, name: &str, x: f64| -> u64 {
        let v = ex(s, "draw.rectangle", json!({"rect": [x, 20.0, x + 40.0, 60.0]}));
        let id = v["id"].as_u64().expect("id");
        ex(s, "element.setName", json!({"ids": [id], "name": name}));
        id
    };
    let mut ids = HashMap::new();
    ids.insert("A", rect(&mut s, "A", 10.0));
    ids.insert("G1", rect(&mut s, "G1", 60.0));
    ids.insert("G2", rect(&mut s, "G2", 110.0));
    ex(&mut s, "select.set", json!({"ids": [ids["G1"], ids["G2"]]}));
    let g = ex(&mut s, "modify.group", json!({}))["id"].as_u64().expect("group");
    ex(&mut s, "element.setName", json!({"ids": [g], "name": "G"}));
    ids.insert("G", g);
    ids.insert("B", rect(&mut s, "B", 160.0));
    ids.insert("C", rect(&mut s, "C", 210.0));
    let l2 = ex(&mut s, "layer.add", json!({}))["layer"].as_u64().expect("layer 2");
    ids.insert("D", rect(&mut s, "D", 260.0));
    ex(&mut s, "select.none", json!({}));
    s.set_tool(ToolId::Pointer).expect("pointer");
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.os_clipboard = false;
    app.state.sync_render = true;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1500.0, 1300.0))
        .with_pixels_per_point(1.0)
        .with_max_steps(8)
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    for _ in 0..2 {
        if find(&h, "C").is_some() {
            break;
        }
        let tab = find(&h, "Ebenen").expect("Layers tab");
        click_at(&mut h, tab.center(), Modifiers::NONE);
        h.run_steps(30);
    }
    for n in ["Ebene 1", "Ebene 2", "A", "B", "C", "D", "G"] {
        assert!(find(&h, n).is_some(), "Layers panel shows row {n}: {:?}", panel_texts(&h));
    }
    (h, Scene { l1, l2, ids })
}

fn ev(h: &mut H, e: Event) {
    h.input_mut().events.push(e);
    h.step();
}

fn click_at(h: &mut H, p: Pos2, modifiers: Modifiers) {
    h.run_steps(30);
    h.input_mut().events.push(Event::ModifiersChanged(modifiers));
    ev(h, Event::PointerMoved(p));
    let e = |down| Event::PointerButton {
        pos: p,
        button: PointerButton::Primary,
        pressed: down,
        modifiers,
    };
    ev(h, e(true));
    ev(h, e(false));
    h.input_mut().events.push(Event::ModifiersChanged(Modifiers::NONE));
    ev(h, Event::PointerMoved(Pos2::new(5.0, 1290.0)));
    h.run_steps(5);
}

/// What the panel showed while the button was held at the target.
struct Held {
    /// Rects of the drop band (5 px high, filled with the text colour).
    bands: Vec<Rect>,
    /// The folder highlight was painted.
    folder: bool,
    /// The system cursor was hidden for the painted drag cursor.
    cursor_hidden: bool,
}

/// A real drag: press on `from`, 20 steps to `to`, hold, then release.
fn drag(h: &mut H, from: Pos2, to: Pos2) -> Held {
    h.run_steps(30);
    ev(h, Event::PointerMoved(from));
    let b = |pos, down| Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed: down,
        modifiers: Modifiers::NONE,
    };
    ev(h, b(from, true));
    for i in 1..=20 {
        let t = i as f32 / 20.0;
        ev(h, Event::PointerMoved(from + (to - from) * t));
    }
    h.run_steps(3);
    let text = fp_ui::theme::Theme::current(&h.ctx).colors.text;
    let mut bands = Vec::new();
    let mut folder = Vec::new();
    for cs in &h.output().shapes {
        fills(&cs.shape, text, &mut bands);
        fills(
            &cs.shape,
            egui::Color32::from_rgba_premultiplied(120, 110, 0, 120),
            &mut folder,
        );
    }
    bands.retain(|r| (r.height() - 5.0).abs() < 0.01 && r.center().x > 900.0);
    let cursor_hidden = h.output().platform_output.cursor_icon == egui::CursorIcon::None;
    ev(h, b(to, false));
    ev(h, Event::PointerMoved(Pos2::new(5.0, 1290.0)));
    h.run_steps(5);
    Held {
        bands,
        folder: !folder.is_empty(),
        cursor_hidden,
    }
}

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

fn fills(shape: &egui::epaint::Shape, col: egui::Color32, out: &mut Vec<Rect>) {
    match shape {
        egui::epaint::Shape::Rect(r) if r.fill == col => out.push(r.rect),
        egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| fills(s, col, out)),
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

fn find(h: &H, label: &str) -> Option<Rect> {
    panel_texts(h)
        .into_iter()
        .filter(|(t, _)| t == label)
        .map(|(_, r)| r)
        .max_by(|a, b| a.center().x.total_cmp(&b.center().x))
}

fn label(h: &H, name: &str) -> Rect {
    find(h, name).unwrap_or_else(|| panic!("row {name}: {:?}", panel_texts(h)))
}

fn row_h() -> f32 {
    fp_ui::panels::layers::object_row_height(fp_ui::panels::layers::thumb_frame_size(0))
}

/// y of an object row's top and bottom (the label is centred in the row).
fn row_top(h: &H, name: &str) -> f32 {
    label(h, name).center().y - row_h() / 2.0
}

fn names(h: &H, layer: u64) -> Vec<String> {
    let d = h.state().state.session.active().expect("doc");
    let l = fp_engine::docops::layer_by_id(&d.doc, d.page, LayerId(layer)).expect("layer");
    l.cel(d.frame)
        .map(|c| {
            c.elements
                .iter()
                .map(|e| e.common.name.clone().unwrap_or_default())
                .collect()
        })
        .unwrap_or_default()
}

fn children(h: &H, g: u64) -> Vec<String> {
    let d = h.state().state.session.active().expect("doc");
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, ElementId(g)).expect("group");
    match &e.kind {
        ElementKind::Group(x) => x
            .children
            .iter()
            .map(|c| c.common.name.clone().unwrap_or_default())
            .collect(),
        _ => panic!("not a group"),
    }
}

fn undo_count(h: &H) -> usize {
    h.state()
        .state
        .session
        .active()
        .map(|d| d.history.undo_count())
        .unwrap_or(0)
}

fn undo_label(h: &H) -> Option<String> {
    h.state()
        .state
        .session
        .active()
        .and_then(|d| d.history.undo_label().map(str::to_string))
}

/// A_Ctop of RE spec 82 §1.2: A onto the upper 6 px of C = before C; one row "Ebenenänderung";
/// the band over the whole list width at C's top, the folder highlighted, own cursor.
#[test]
fn drag_into_upper_zone_moves_before_the_row() {
    let (mut h, sc) = harness();
    let n0 = undo_count(&h);
    let a = label(&h, "A");
    let top_c = row_top(&h, "C");
    let held = drag(&mut h, a.center(), Pos2::new(a.center().x, top_c + 3.0));
    assert_eq!(names(&h, sc.l1), ["A", "C", "B", "G"]);
    assert_eq!(undo_count(&h), n0 + 1);
    assert_eq!(undo_label(&h).as_deref(), Some("Ebenenänderung"));
    assert_eq!(held.bands.len(), 1, "one band: {:?}", held.bands);
    let band = held.bands[0];
    assert!(
        (band.top() - (top_c.round() - 3.0)).abs() < 1.01,
        "band {band:?} at the gap {top_c}"
    );
    assert!(band.width() > 200.0, "over the list width: {band:?}");
    assert!(held.folder, "target layer folder highlighted");
    assert!(held.cursor_hidden, "drag cursor painted instead of the system cursor");
    // the moved object stays selected
    let sel = h.state().state.session.active().expect("doc").selection.elements.clone();
    assert_eq!(sel, vec![ElementId(sc.ids["A"])]);
}

/// B_Gmid / A_Bmid / D_Amid: the rest of a row = after it; into the other layer.
#[test]
fn drag_into_lower_part_moves_after_the_row() {
    let (mut h, sc) = harness();
    let b = label(&h, "B");
    let top_g = row_top(&h, "G");
    drag(&mut h, b.center(), Pos2::new(b.center().x, top_g + row_h() / 2.0));
    assert_eq!(names(&h, sc.l1), ["C", "G", "B", "A"]);
    let d = label(&h, "D");
    let top_a = row_top(&h, "A");
    drag(&mut h, d.center(), Pos2::new(d.center().x, top_a + row_h() - 3.0));
    assert_eq!(names(&h, sc.l1), ["C", "G", "B", "A", "D"]);
    assert!(names(&h, sc.l2).is_empty());
    // onto a layer row: top of that layer
    let a = label(&h, "A");
    let e2 = label(&h, "Ebene 2");
    drag(&mut h, a.center(), Pos2::new(a.center().x, e2.center().y));
    assert_eq!(names(&h, sc.l2), ["A"]);
}

/// A_Gmid, A_Gbot, B_Gtop, A_self: the old place shows no band and writes no History row
/// (TW-299: the measured "middle zone" step of the reference is not copied).
#[test]
fn drop_at_the_old_place_does_nothing() {
    let (mut h, sc) = harness();
    let n0 = undo_count(&h);
    let gen0 = h.state().state.session.active().expect("doc").generation;
    let a = label(&h, "A");
    let top_g = row_top(&h, "G");
    for y in [top_g + 10.0, top_g + row_h() / 2.0, top_g + row_h() - 2.0] {
        let held = drag(&mut h, a.center(), Pos2::new(a.center().x, y));
        assert!(held.bands.is_empty(), "no band at {y}: {:?}", held.bands);
        assert!(!held.folder, "no folder highlight");
    }
    let b = label(&h, "B");
    let held = drag(&mut h, b.center(), Pos2::new(b.center().x, top_g + 2.0));
    assert!(held.bands.is_empty());
    assert_eq!(names(&h, sc.l1), ["C", "B", "G", "A"]);
    assert_eq!(undo_count(&h), n0, "no History row");
    assert_eq!(
        h.state().state.session.active().expect("doc").generation,
        gen0
    );
}

/// Mehrfach: A and B selected, A dragged before C: B A C G (block, order kept).
#[test]
fn selected_rows_move_as_a_block() {
    let (mut h, sc) = harness();
    let a = label(&h, "A");
    click_at(&mut h, a.center(), Modifiers::NONE);
    let b = label(&h, "B");
    click_at(&mut h, b.center(), Modifiers::COMMAND);
    let mut sel: Vec<u64> = h
        .state()
        .state
        .session
        .active()
        .expect("doc")
        .selection
        .elements
        .iter()
        .map(|e| e.0)
        .collect();
    sel.sort();
    let mut want = vec![sc.ids["A"], sc.ids["B"]];
    want.sort();
    assert_eq!(sel, want);
    let n0 = undo_count(&h);
    let top_c = row_top(&h, "C");
    drag(&mut h, a.center(), Pos2::new(a.center().x, top_c + 2.0));
    assert_eq!(names(&h, sc.l1), ["B", "A", "C", "G"]);
    assert_eq!(undo_count(&h), n0 + 1, "one History row for the block");
}

/// Firepixel extension (RE spec 82 §1.5): with G open, the gap between the group row and its
/// first object leads into the group (band indented); an object of the group is dragged out.
#[test]
fn drops_into_and_out_of_an_open_group() {
    let (mut h, sc) = harness();
    // open the group toggle after the name
    let g = label(&h, "G");
    click_at(&mut h, Pos2::new(g.right() + 9.0, g.center().y), Modifiers::NONE);
    assert!(find(&h, "G2").is_some(), "group open: {:?}", panel_texts(&h));
    let b = label(&h, "B");
    let top_g2 = row_top(&h, "G2");
    let held = drag(&mut h, b.center(), Pos2::new(b.center().x, top_g2 + 2.0));
    assert_eq!(names(&h, sc.l1), ["C", "G", "A"]);
    assert_eq!(children(&h, sc.ids["G"]), ["B", "G2", "G1"]);
    assert_eq!(held.bands.len(), 1);
    // the layer row's name starts 78 px right of the list's left edge
    let list_left = label(&h, "Ebene 1").left() - 78.0;
    assert!(
        held.bands[0].left() > list_left + 60.0,
        "band indented to the group level: {:?} (list {list_left})",
        held.bands[0]
    );
    // the lower part of the open group row: into the group as its first object
    let c = label(&h, "C");
    let top_g = row_top(&h, "G");
    drag(&mut h, c.center(), Pos2::new(c.center().x, top_g + row_h() - 3.0));
    assert_eq!(children(&h, sc.ids["G"]), ["C", "B", "G2", "G1"]);
    assert_eq!(names(&h, sc.l1), ["G", "A"]);
    // an object of the group dragged out below A
    let g1 = label(&h, "G1");
    let top_a = row_top(&h, "A");
    drag(&mut h, g1.center(), Pos2::new(g1.center().x, top_a + row_h() - 3.0));
    assert_eq!(names(&h, sc.l1), ["G", "A", "G1"]);
    assert_eq!(children(&h, sc.ids["G"]), ["C", "B", "G2"]);
    // a group never into itself: G dragged onto its own child row does nothing
    let n0 = undo_count(&h);
    let g = label(&h, "G");
    let top_b = row_top(&h, "B");
    let held = drag(&mut h, g.center(), Pos2::new(g.center().x, top_b + 2.0));
    assert!(held.bands.is_empty());
    assert_eq!(undo_count(&h), n0);
}
