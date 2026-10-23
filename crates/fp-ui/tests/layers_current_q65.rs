//! RE spec 64 §2.5 and Q-65 in the Layers panel: an empty circle on every layer row, filled where the
//! layer holds a selected object (grey on the web layer), the circle does not mark the current layer;
//! with nothing selected the current layer's row is highlighted and bold, with a selection the
//! selected object rows are highlighted and bold and the current layer plus the layers of the
//! selection are tinted light; a plain layer-row click writes one "Layer Change" row; on opening the
//! list shows the current layer's row.

use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use fp_engine::tools::ToolId;
use fp_ui::FirepixelApp;
use fp_ui::app_state::UiSettings;
use serde_json::{Value, json};

type H = Harness<'static, FirepixelApp>;

fn ex(s: &mut fp_engine::Session, c: &str, p: Value) -> Value {
    s.execute(c, p).unwrap_or_else(|e| panic!("{c}: {e}")).value
}

/// `mkn` of RE spec 64 §2.1: L1 (bottom) with r1, L2 with r2, L3 with r3; L3 current.
fn mkn() -> (fp_engine::Session, [u64; 3], [u64; 3]) {
    fp_i18n::set_language("de");
    let mut s = fp_engine::Session::default();
    ex(&mut s, "file.new", json!({"width": 600, "height": 400, "background": "#ffffff"}));
    let l1 = s.active().and_then(|d| d.layer).map(|l| l.0).expect("layer");
    ex(&mut s, "layer.rename", json!({"name": "L1"}));
    let mut ls = [l1, 0, 0];
    let mut rs = [0u64; 3];
    for i in 0..3 {
        if i > 0 {
            ls[i] = ex(&mut s, "layer.add", json!({"name": format!("L{}", i + 1)}))["layer"].as_u64().expect("layer");
        }
        let x = 50.0 + 150.0 * i as f64;
        rs[i] = ex(&mut s, "draw.rectangle", json!({"rect": [x, 50.0 + 50.0 * i as f64, x + 80.0, 110.0 + 50.0 * i as f64]}))["id"].as_u64().expect("id");
        ex(&mut s, "element.setName", json!({"ids": [rs[i]], "name": format!("r{}", i + 1)}));
    }
    ex(&mut s, "select.none", json!({}));
    (s, ls, rs)
}

fn harness(mut s: fp_engine::Session, first_row: &str, height: f32) -> H {
    s.set_tool(ToolId::Pointer).expect("pointer");
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.os_clipboard = false;
    app.state.sync_render = true;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1500.0, height))
        .with_pixels_per_point(1.0)
        .with_max_steps(8)
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    for _ in 0..2 {
        if find(&h, first_row).is_some() {
            break;
        }
        let tab = find(&h, "Ebenen").expect("Layers tab");
        click_at(&mut h, tab);
        h.run_steps(30);
    }
    h.run_steps(10);
    h
}

fn ev(h: &mut H, e: Event) {
    h.input_mut().events.push(e);
    h.step();
}

fn click_at(h: &mut H, p: Pos2) {
    h.run_steps(30);
    ev(h, Event::PointerMoved(p));
    let e = |down| Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: down, modifiers: Modifiers::NONE };
    ev(h, e(true));
    ev(h, e(false));
    ev(h, Event::PointerMoved(Pos2::new(5.0, 5.0)));
    h.run_steps(5);
}

fn click(h: &mut H, label: &str) {
    let p = find(h, label).unwrap_or_else(|| panic!("row {label}"));
    click_at(h, p);
}

/// Text shapes with their rectangle, font family and clip rectangle.
fn texts(shape: &egui::epaint::Shape, clip: egui::Rect, out: &mut Vec<(String, egui::Rect, egui::FontFamily, egui::Rect)>) {
    match shape {
        egui::epaint::Shape::Text(t) => {
            let fam = t.galley.job.sections.first().map(|s| s.format.font_id.family.clone()).unwrap_or_default();
            out.push((t.galley.job.text.clone(), egui::Rect::from_min_size(t.pos, t.galley.size()), fam, clip));
        }
        egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| texts(s, clip, out)),
        _ => {}
    }
}

fn all_texts(h: &H) -> Vec<(String, egui::Rect, egui::FontFamily, egui::Rect)> {
    let mut all = Vec::new();
    for cs in &h.output().shapes {
        texts(&cs.shape, cs.clip_rect, &mut all);
    }
    all
}

/// Label in the right part of the window (the Layers panel), rightmost match.
fn find(h: &H, label: &str) -> Option<Pos2> {
    all_texts(h).into_iter().filter(|(t, r, _, _)| t == label && r.center().x > 900.0).map(|(_, r, _, _)| r.center()).max_by(|a, b| a.x.total_cmp(&b.x))
}

fn bold(h: &H, label: &str) -> bool {
    let fam = fp_ui::theme::Theme::current(&h.ctx).bold(12.0).family;
    all_texts(h).into_iter().filter(|(t, r, _, _)| t == label && r.center().x > 900.0).any(|(_, _, f, _)| f == fam)
}

fn fills(shape: &egui::epaint::Shape, col: egui::Color32, out: &mut Vec<egui::Rect>) {
    match shape {
        egui::epaint::Shape::Rect(r) if r.fill == col => out.push(r.rect),
        egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| fills(s, col, out)),
        _ => {}
    }
}

fn on_fill(h: &H, label: &str, col: egui::Color32) -> bool {
    let p = find(h, label).unwrap_or_else(|| panic!("{label}"));
    let mut rs = Vec::new();
    for cs in &h.output().shapes {
        fills(&cs.shape, col, &mut rs);
    }
    rs.iter().any(|r| r.contains(p))
}

/// Row background: "dark" (selected colour #a8c6ed), "light" (#d3e2f5) or "-".
fn tint(h: &H, label: &str) -> &'static str {
    let c = fp_ui::theme::Theme::current(&h.ctx).colors.clone();
    if on_fill(h, label, c.list_selected_bg) {
        "dark"
    } else if on_fill(h, label, c.list_selected_light_bg) {
        "light"
    } else {
        "-"
    }
}

fn circles(shape: &egui::epaint::Shape, out: &mut Vec<egui::epaint::CircleShape>) {
    match shape {
        egui::epaint::Shape::Circle(c) => out.push(*c),
        egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| circles(s, out)),
        _ => {}
    }
}

/// The circle at the right of the row `label`: `None` without one, else (filled, stroke colour).
fn dot(h: &H, label: &str) -> Option<(bool, egui::Color32)> {
    let p = find(h, label).unwrap_or_else(|| panic!("{label}"));
    let mut cs = Vec::new();
    for s in &h.output().shapes {
        circles(&s.shape, &mut cs);
    }
    let row: Vec<_> = cs.iter().filter(|c| (c.center.y - p.y).abs() < 6.0 && c.center.x > p.x).collect();
    let ring = row.iter().find(|c| (c.radius - 4.5).abs() < 0.1)?;
    let filled = row.iter().any(|c| (c.radius - 2.0).abs() < 0.1 && c.fill != egui::Color32::TRANSPARENT);
    Some((filled, ring.stroke.color))
}

const WEB: &str = "Webebene";

#[test]
fn panel_dots_tints_and_bold_as_measured() {
    let (s, ls, rs) = mkn();
    let mut h = harness(s, "L1", 950.0);
    let col = fp_ui::theme::Theme::current(&h.ctx).colors.clone();
    // s0: nothing selected – current L3 dark and bold, every circle empty, the web one grey
    assert_eq!([tint(&h, "L1"), tint(&h, "L2"), tint(&h, "L3")], ["-", "-", "dark"]);
    assert!(bold(&h, "L3") && !bold(&h, "L1"));
    for l in ["L1", "L2", "L3"] {
        assert_eq!(dot(&h, l), Some((false, col.icon_fg)), "{l}: empty circle");
    }
    assert_eq!(dot(&h, WEB), Some((false, col.icon_disabled)), "web layer: grey circle");
    // s1: object row r1 → [r1], L1 current: L1 light, r1 dark and bold, filled circle at L1 only
    click(&mut h, "r1");
    let cur = h.state().state.session.active().and_then(|d| d.layer).map(|l| l.0);
    assert_eq!(cur, Some(ls[0]), "the object row makes its layer current");
    assert_eq!([tint(&h, "L1"), tint(&h, "L2"), tint(&h, "L3"), tint(&h, "r1")], ["light", "-", "-", "dark"]);
    assert!(bold(&h, "r1") && !bold(&h, "L1"));
    assert_eq!(dot(&h, "L1"), Some((true, col.icon_fg)));
    assert_eq!(dot(&h, "L2"), Some((false, col.icon_fg)));
    // s17: r1 and r2 selected with L3 current – L3 light (current, empty circle), L1 and L2 light
    // with filled circles
    h.state_mut().state.session.execute("layer.setActive", json!({"layer": ls[2]})).expect("L3");
    h.state_mut().state.session.execute("select.set", json!({"ids": [rs[0], rs[1]]})).expect("sel");
    h.run_steps(5);
    assert_eq!([tint(&h, "L1"), tint(&h, "L2"), tint(&h, "L3")], ["light", "light", "light"]);
    assert_eq!([dot(&h, "L1"), dot(&h, "L2"), dot(&h, "L3")].map(|d| d.map(|x| x.0)), [Some(true), Some(true), Some(false)]);
    assert!(!bold(&h, "L3"));
    assert_eq!([tint(&h, "r1"), tint(&h, "r2"), tint(&h, "r3")], ["dark", "dark", "-"]);
    // a selected slice fills the web layer's circle (grey)
    let sl = h.state_mut().state.session.execute("web.addSlice", json!({"shape": "rect", "rect": [500, 300, 560, 360]})).expect("slice").value["id"].as_u64().expect("id");
    h.state_mut().state.session.execute("select.set", json!({"ids": [sl]})).expect("sel");
    h.run_steps(5);
    assert_eq!(dot(&h, WEB), Some((true, col.icon_disabled)));
    // deselecting: back to the current layer highlighted
    h.state_mut().state.session.execute("select.none", json!({})).expect("none");
    h.run_steps(5);
    assert_eq!(tint(&h, "L3"), "dark");
}

#[test]
fn layer_row_click_writes_a_layer_change_row() {
    let (s, ls, rs) = mkn();
    let mut h = harness(s, "L1", 950.0);
    let rows = h.state().state.session.active().map(|d| d.history.len()).unwrap_or(0);
    click(&mut h, "L2");
    let d = h.state().state.session.active().expect("doc");
    assert_eq!(d.layer.map(|l| l.0), Some(ls[1]));
    assert_eq!(d.selection.elements.iter().map(|e| e.0).collect::<Vec<_>>(), [rs[1]]);
    assert_eq!(d.history.len(), rows + 1);
    assert_eq!(d.history.labels().last().map(String::as_str), Some("Ebenenänderung"));
    // the object row afterwards writes nothing
    click(&mut h, "r1");
    assert_eq!(h.state().state.session.active().map(|d| d.history.len()), Some(rows + 1));
    h.state_mut().state.session.undo().expect("undo");
    let d = h.state().state.session.active().expect("doc");
    assert_eq!(d.layer.map(|l| l.0), Some(ls[2]), "undo restores the current layer");
    assert!(d.selection.elements.is_empty(), "and the selection");
}

#[test]
fn opening_scrolls_to_the_current_layer() {
    // 24 layers with one object each; L1 (bottom) current as after opening: its row lies far below
    // the top of the list and must be visible
    fp_i18n::set_language("de");
    let mut s = fp_engine::Session::default();
    ex(&mut s, "file.new", json!({"width": 600, "height": 400, "background": "#ffffff"}));
    ex(&mut s, "layer.rename", json!({"name": "L1"}));
    let l1 = s.active().and_then(|d| d.layer).map(|l| l.0).expect("layer");
    ex(&mut s, "draw.rectangle", json!({"rect": [10, 10, 30, 30]}));
    for i in 2..=24 {
        ex(&mut s, "layer.add", json!({"name": format!("L{i}")}));
        ex(&mut s, "draw.rectangle", json!({"rect": [10, 10, 30, 30]}));
    }
    ex(&mut s, "select.none", json!({}));
    ex(&mut s, "layer.setActive", json!({"layer": l1}));
    let h = harness(s, "L24", 700.0);
    let row = all_texts(&h).into_iter().find(|(t, r, _, _)| t == "L1" && r.center().x > 900.0);
    let (_, r, _, clip) = row.expect("L1 row painted");
    assert!(clip.contains(r.center()), "the current layer's row is scrolled into view: {r:?} in {clip:?}");
    // control: the list is longer than the panel, the top row is out of view now
    let top = all_texts(&h).into_iter().find(|(t, r, _, _)| t == "L24" && r.center().x > 900.0);
    let (_, r, _, clip) = top.expect("L24 row painted");
    assert!(!clip.contains(r.center()), "the list did scroll");
}
