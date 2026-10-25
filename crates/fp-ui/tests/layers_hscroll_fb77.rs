//! FB-77 (panel part): the Layers panel's list scrolls horizontally when its rows need more width
//! than the panel (a deeply nested group opened in the group tree): a horizontal scroll bar
//! appears and Shift + mouse wheel over the list scrolls it sideways. The wheel over the panel
//! never pans the canvas. A shallow list does not scroll sideways.

use egui::{Event, Modifiers, Pos2, Rect, vec2};
use egui_kittest::Harness;
use fp_engine::tools::ToolId;
use fp_ui::FirepixelApp;
use fp_ui::app_state::UiSettings;
use serde_json::{Value, json};
use std::collections::BTreeSet;

type H = Harness<'static, FirepixelApp>;

const LEVELS: usize = 10;

fn ex(s: &mut fp_engine::Session, c: &str, p: Value) -> Value {
    s.execute(c, p).unwrap_or_else(|e| panic!("{c}: {e}")).value
}

/// Document with "Deep" nested in `LEVELS` groups "L1" (innermost) to "L10" (outermost); returns
/// the harness and the group ids.
fn harness() -> (H, Vec<u64>) {
    fp_i18n::set_language("de");
    let mut s = fp_engine::Session::default();
    ex(
        &mut s,
        "file.new",
        json!({"width": 700, "height": 400, "background": "#ffffff"}),
    );
    let rect = |s: &mut fp_engine::Session, name: &str, x: f64| -> u64 {
        let v = ex(
            s,
            "draw.rectangle",
            json!({"rect": [x, 20.0, x + 20.0, 40.0]}),
        );
        let id = v["id"].as_u64().expect("id");
        ex(s, "element.setName", json!({"ids": [id], "name": name}));
        id
    };
    let mut inner = rect(&mut s, "Deep", 10.0);
    let mut groups = Vec::new();
    for i in 1..=LEVELS {
        let other = rect(&mut s, &format!("R{i}"), 10.0 + 25.0 * i as f64);
        ex(&mut s, "select.set", json!({"ids": [inner, other]}));
        let g = ex(&mut s, "modify.group", json!({}))["id"]
            .as_u64()
            .expect("group");
        ex(
            &mut s,
            "element.setName",
            json!({"ids": [g], "name": format!("L{i}")}),
        );
        groups.push(g);
        inner = g;
    }
    ex(&mut s, "select.none", json!({}));
    // 20 px rows without thumbnails, so more of the tree fits the list
    s.prefs.set(
        "LayersList_ThumbnailSize",
        fp_engine::prefs::PrefValue::Int(0),
    );
    s.set_tool(ToolId::Pointer).expect("pointer");
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.os_clipboard = false;
    app.state.sync_render = true;
    let mut h = Harness::builder()
        .with_size(vec2(1500.0, 950.0))
        .with_pixels_per_point(1.0)
        .with_max_steps(8)
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    for _ in 0..2 {
        if find(&h, "L10").is_some() {
            break;
        }
        let tab = find(&h, "Ebenen").expect("Layers tab");
        click_at(&mut h, tab.center());
        h.run_steps(30);
    }
    assert!(
        find(&h, "L10").is_some(),
        "Layers panel shows L10: {:?}",
        panel_texts(&h)
    );
    (h, groups)
}

fn click_at(h: &mut H, p: Pos2) {
    h.event(Event::PointerMoved(p));
    let e = |down| Event::PointerButton {
        pos: p,
        button: egui::PointerButton::Primary,
        pressed: down,
        modifiers: Modifiers::NONE,
    };
    h.event(e(true));
    h.step();
    h.event(e(false));
    h.step();
    h.event(Event::PointerMoved(Pos2::new(5.0, 940.0)));
    h.run_steps(5);
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

fn origin(h: &H) -> (f64, f64) {
    h.state()
        .state
        .session
        .active()
        .map(|d| d.view.origin)
        .unwrap_or_default()
}

/// Shift + wheel (three notches down) over `p`.
fn shift_wheel(h: &mut H, p: Pos2) {
    h.event(Event::PointerMoved(p));
    h.event(Event::ModifiersChanged(Modifiers::SHIFT));
    h.event(Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: vec2(0.0, -3.0),
        modifiers: Modifiers::SHIFT,
        phase: egui::TouchPhase::Move,
    });
    h.step();
    h.event(Event::ModifiersChanged(Modifiers::NONE));
    h.run_steps(20);
}

/// Opens the group tree of every group.
fn open_all(h: &mut H, groups: &[u64]) {
    let doc = h.state().state.session.active_id().expect("doc").0;
    let set: BTreeSet<(u64, u64)> = groups.iter().map(|g| (doc, *g)).collect();
    h.ctx
        .data_mut(|d| d.insert_temp(egui::Id::new("layers-tree-open"), set));
    h.run_steps(10);
}

#[test]
fn shallow_list_does_not_scroll_sideways() {
    let (mut h, _) = harness();
    let row = find(&h, "L10").expect("L10");
    let o = origin(&h);
    shift_wheel(&mut h, row.center());
    let after = find(&h, "L10").expect("L10");
    assert!(
        (after.left() - row.left()).abs() < 0.5,
        "no sideways scrolling: {row:?} -> {after:?}"
    );
    assert_eq!(origin(&h), o, "the canvas did not pan");
}

#[test]
fn deeply_nested_tree_scrolls_horizontally_with_shift_wheel() {
    let (mut h, groups) = harness();
    open_all(&mut h, &groups);
    // scroll the list down to its end so the innermost row "Deep" is in view
    let l10 = find(&h, "L10").expect("L10");
    h.event(Event::PointerMoved(l10.center()));
    h.event(Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: vec2(0.0, -30.0),
        modifiers: Modifiers::NONE,
        phase: egui::TouchPhase::Move,
    });
    h.run_steps(30);
    let deep = find(&h, "Deep").unwrap_or_else(|| panic!("Deep row listed: {:?}", panel_texts(&h)));
    let l1 = find(&h, "L1").expect("L1");
    assert!(
        deep.left() > l1.left() + 10.0,
        "Deep indented below L1: {l1:?} {deep:?}"
    );
    let o = origin(&h);
    shift_wheel(&mut h, Pos2::new(l1.center().x, deep.center().y));
    let after = find(&h, "Deep").expect("Deep");
    assert!(
        after.left() < deep.left() - 10.0,
        "Shift + wheel scrolled the list sideways: {deep:?} -> {after:?}"
    );
    let l1_after = find(&h, "L1")
        .map(|r| r.left())
        .unwrap_or(f32::NEG_INFINITY);
    assert!(
        l1_after < l1.left() - 10.0,
        "the whole list moved: {l1:?} -> {l1_after}"
    );
    assert_eq!(origin(&h), o, "the canvas did not pan");
}
