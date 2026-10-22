//! TW-195 / Q-63 (RE spec 59 section 1, measured): new auto shapes take the document's tool
//! attributes in full (fill including none, stroke colour, tip, size, placement). Connector Line
//! and Spiral: fill none, the tool stroke; only without a tool stroke their own black 1 px
//! (`bn_1-Pixel` resp. `bn_Soft Line`). Arrow Line: always black `bn_Hard Line` 1 px, not
//! anti-aliased, fill none. Measure Tool: red `bn_Hard Line` 1 px inside plus a black label, not
//! selected afterwards. Only Arrow Line and Measure Tool change the tool (black resp. red
//! `bn_Hard Line` 1 px, fill none, placement kept); the new shape (a group) is not followed. Each
//! shape is one History row "Form" / "Shape"; undo removes it, not the tool change.

use fp_color::Argb;
use fp_engine::Session;
use fp_engine::commands::panel::{preset_index_of, tool_brush_from};
use fp_model::{Element, ElementId, ElementKind};
use serde_json::json;

const RED: Argb = Argb(0xffff_0000);
const BLACK: Argb = Argb(0xff00_0000);
const WHITE: Argb = Argb(0xffff_ffff);
const BLUE: Argb = Argb(0xff00_00ff);

fn session() -> Session {
    fp_i18n::profile::use_test_profile();
    let mut s = Session::default();
    fp_script::install_host(&mut s);
    s.execute("file.new", json!({"width": 600, "height": 400}))
        .unwrap_or_else(|e| panic!("{e}"));
    s
}

/// Tool: fill white solid, stroke red `bn_1-Pixel` 4 px with the given placement.
fn tool_red4(s: &mut Session, placement: &str) {
    let mut st = s.tool_style();
    st.stroke_color = RED;
    st.fill_color = WHITE;
    st.new_fill = None;
    st.new_stroke = fp_engine::tool_follow::stroke_params(
        preset_index_of("bc_Pencil", "bn_1-Pixel"),
        4,
        0,
        placement,
        None,
    );
    st.new_brush = None;
    s.set_tool_style(st);
}

/// Tool: fill white solid, stroke none (colour blue).
fn tool_none(s: &mut Session) {
    let mut st = s.tool_style();
    st.stroke_color = BLUE;
    st.fill_color = WHITE;
    st.new_fill = None;
    st.new_stroke = Some(json!({"none": true}));
    st.new_brush = None;
    s.set_tool_style(st);
}

fn insert(s: &mut Session, shape: &str) -> ElementId {
    s.execute("select.none", json!({})).expect("select.none");
    let v = s
        .execute(
            "autoShape.insert",
            json!({"shape": shape, "rect": [100, 100, 220, 200], "from": [100, 100]}),
        )
        .unwrap_or_else(|e| panic!("{shape}: {e}"))
        .value;
    ElementId(v["id"].as_u64().expect("id"))
}

fn element(s: &Session, id: ElementId) -> Element {
    let d = s.active().expect("doc");
    (*fp_engine::docops::element_on_page(&d.doc, d.page, id).expect("element")).clone()
}

/// (fill?, fill colour, stroke colour, brush (category, name, size, anti-aliased), placement) of
/// every path child.
type Attrs = (bool, Argb, Argb, Option<(String, String, i32, bool)>, i32);

fn paths(s: &Session, id: ElementId) -> Vec<Attrs> {
    let ElementKind::Group(g) = element(s, id).kind else {
        panic!("group")
    };
    g.children
        .iter()
        .filter_map(|c| match &c.kind {
            ElementKind::Path(p) => Some((
                p.attrs.fill.is_some(),
                p.attrs.fill_color,
                p.attrs.stroke_color,
                p.attrs.brush.as_ref().map(|b| {
                    (b.category.clone(), b.name.clone(), b.diameter, b.antialiased)
                }),
                p.stroke_placement,
            )),
            _ => None,
        })
        .collect()
}

fn brush(cat: &str, name: &str, size: i32, aa: bool) -> Option<(String, String, i32, bool)> {
    Some((cat.into(), name.into(), size, aa))
}

const FULL: [&str; 9] = [
    "Arrow",
    "Chamfer Rectangle",
    "Beveled Rectangle",
    "Rounded Rectangle",
    "Doughnut",
    "L-Shape",
    "Smart Polygon",
    "Star",
    "Pie",
];

#[test]
fn nine_shapes_take_the_tool_attributes_in_full() {
    for (placement, id) in [("center", 1), ("inside", 0), ("outside", 2)] {
        let mut s = session();
        tool_red4(&mut s, placement);
        let before = s.tool_style();
        for shape in FULL {
            let id_ = insert(&mut s, shape);
            let ps = paths(&s, id_);
            assert!(!ps.is_empty(), "{shape}");
            for p in ps {
                assert_eq!(
                    p,
                    (true, WHITE, RED, brush("bc_Pencil", "bn_1-Pixel", 4, false), id),
                    "{shape} {placement}"
                );
            }
            s.follow_selection();
            assert_eq!(s.tool_style(), before, "{shape}: the tool stays");
        }
    }
}

#[test]
fn shapes_take_stroke_none_and_fill_none() {
    let mut s = session();
    tool_none(&mut s);
    let star = insert(&mut s, "Star");
    for p in paths(&s, star) {
        assert_eq!((p.0, p.3.clone()), (true, None), "star: no stroke, fill");
    }
    let mut st = s.tool_style();
    st.new_fill = Some(json!({"category": "none"}));
    s.set_tool_style(st);
    let arrow = insert(&mut s, "Arrow");
    for p in paths(&s, arrow) {
        assert_eq!((p.0, p.3.clone()), (false, None), "arrow: no fill, no stroke");
    }
}

#[test]
fn connector_and_spiral_take_the_tool_stroke_without_fill() {
    let mut s = session();
    tool_red4(&mut s, "center");
    let before = s.tool_style();
    for shape in ["Connector Line", "Spiral"] {
        let id = insert(&mut s, shape);
        for p in paths(&s, id) {
            assert_eq!(
                (p.0, p.2, p.3.clone()),
                (false, RED, brush("bc_Pencil", "bn_1-Pixel", 4, false)),
                "{shape}"
            );
        }
        s.follow_selection();
        assert_eq!(s.tool_style(), before, "{shape}: the tool stays");
    }
}

#[test]
fn connector_and_spiral_draw_their_own_stroke_without_a_tool_stroke() {
    let mut s = session();
    tool_none(&mut s);
    let before = s.tool_style();
    let c = insert(&mut s, "Connector Line");
    for p in paths(&s, c) {
        assert_eq!(
            (p.0, p.2, p.3.clone()),
            (false, BLACK, brush("bc_Pencil", "bn_1-Pixel", 1, false))
        );
    }
    let sp = insert(&mut s, "Spiral");
    for p in paths(&s, sp) {
        assert_eq!(
            (p.0, p.2, p.3.clone()),
            (false, BLACK, brush("bc_Basic", "bn_Soft Line", 1, true))
        );
    }
    s.follow_selection();
    assert_eq!(s.tool_style(), before, "the tool stays (stroke none, fill solid)");
}

/// The tool after Arrow Line / Measure Tool: `bn_Hard Line` 1 px in `color`, fill none with the
/// fill colour kept, placement kept.
fn tool_is_hard_line(s: &Session, color: Argb, placement: &str) {
    let st = s.tool_style();
    assert_eq!(st.stroke_color, color);
    let b = tool_brush_from(st.new_stroke.as_ref(), st.new_brush.as_ref());
    assert_eq!(
        (b.category.as_str(), b.name.as_str(), b.diameter, b.antialiased),
        ("bc_Basic", "bn_Hard Line", 1, false)
    );
    assert!(st.fill_is_none());
    assert_eq!(st.fill_color, WHITE, "the fill colour stays");
    assert_eq!(st.placement(), placement);
}

#[test]
fn arrow_line_draws_black_hard_line_and_sets_the_tool() {
    let mut s = session();
    tool_red4(&mut s, "inside");
    let id = insert(&mut s, "Arrow Line");
    let ps = paths(&s, id);
    assert_eq!(ps.len(), 3);
    for p in &ps {
        assert_eq!(p.2, BLACK);
        assert_eq!(p.3, brush("bc_Basic", "bn_Hard Line", 1, false));
    }
    assert!(!ps[0].0, "the line has no fill");
    tool_is_hard_line(&s, BLACK, "inside");
    assert_eq!(
        s.active().expect("doc").selection.elements,
        vec![id],
        "selected"
    );
    // undo removes the shape, not the tool change
    s.execute("edit.undo", json!({})).expect("undo");
    let d = s.active().expect("doc");
    assert!(fp_engine::docops::element_on_page(&d.doc, d.page, id).is_none());
    tool_is_hard_line(&s, BLACK, "inside");
    s.execute("edit.redo", json!({})).expect("redo");
    tool_is_hard_line(&s, BLACK, "inside");
}

#[test]
fn measure_tool_is_red_inside_not_selected_and_sets_the_tool() {
    let mut s = session();
    tool_red4(&mut s, "center");
    let id = insert(&mut s, "Measure Tool");
    let ps = paths(&s, id);
    assert_eq!(ps.len(), 1);
    assert_eq!(
        ps[0],
        (false, ps[0].1, RED, brush("bc_Basic", "bn_Hard Line", 1, false), 0)
    );
    let ElementKind::Group(g) = element(&s, id).kind else {
        panic!("group")
    };
    let text = g
        .children
        .iter()
        .find_map(|c| match &c.kind {
            ElementKind::Text(t) => Some(t.clone()),
            _ => None,
        })
        .expect("label");
    assert_eq!(text.plain_text(), "156 px");
    let color = text
        .runs
        .first()
        .and_then(|r| r.attrs.color)
        .or(text.initial.color)
        .map(Argb);
    assert_eq!(color, Some(BLACK), "black label");
    assert!(
        s.active().expect("doc").selection.elements.is_empty(),
        "not selected"
    );
    tool_is_hard_line(&s, RED, "center");
    s.execute("edit.undo", json!({})).expect("undo");
    tool_is_hard_line(&s, RED, "center");
}

#[test]
fn each_shape_is_one_history_row_shape() {
    for (lang, label) in [("de", "Form"), ("en", "Shape")] {
        fp_i18n::set_language(lang);
        let mut s = session();
        for shape in ["Star", "Arrow Line", "Measure Tool"] {
            s.execute("select.none", json!({})).expect("select.none");
            let n = s.active().expect("doc").history.undo_count();
            insert(&mut s, shape);
            let d = s.active().expect("doc");
            assert_eq!(d.history.undo_count(), n + 1, "{lang} {shape}: one row");
            assert_eq!(d.history.labels().last().map(String::as_str), Some(label), "{lang} {shape}");
        }
    }
    fp_i18n::set_language("de");
}

/// A selected auto shape is not followed; the wells show its path's colours meanwhile.
#[test]
fn a_selected_auto_shape_is_not_followed_but_shown() {
    let mut s = session();
    tool_none(&mut s);
    let mut st = s.tool_style();
    st.stroke_color = BLACK;
    st.new_stroke = None;
    s.set_tool_style(st);
    let star = insert(&mut s, "Star");
    tool_red4(&mut s, "center");
    s.execute("select.none", json!({})).expect("none");
    s.execute("select.set", json!({"ids": [star.0]})).expect("select");
    s.follow_selection();
    let st = s.tool_style();
    assert_eq!((st.stroke_color, st.fill_color), (RED, WHITE));
    let b = tool_brush_from(st.new_stroke.as_ref(), st.new_brush.as_ref());
    assert_eq!(b.diameter, 4, "the tool stays");
    assert_eq!(s.tool_well_colors(), (BLACK, WHITE), "the wells show the star");
    s.execute("select.none", json!({})).expect("none");
    assert_eq!(s.tool_well_colors(), (RED, WHITE));
}
