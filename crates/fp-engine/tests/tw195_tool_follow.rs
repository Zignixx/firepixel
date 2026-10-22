//! TW-195 / Q-63 (RE spec 59, measured): the tool attributes follow only a single selected path or
//! text - not groups, several objects, bitmaps or hotspots; a path with the fill "none" gives the
//! tool its own stored fill colour; a text gives its colour as the fill, no stroke, placement
//! outside. The fill "none" belongs to the document (a new document starts with a solid fill, the
//! stroke carries over). "Default Colours" sets only the colours and brings back a stroke or fill
//! "none". After the start the tool stroke is `bn_1-Pixel Anti-Aliased` in the size `BrushTipSize`
//! (2 without a stored value), centred. The toolbox wells show "none" as such and the topmost
//! object's colours during a multi-selection.

use crate::common;

use common::*;
use fp_color::Argb;
use fp_engine::Session;
use fp_engine::commands::panel::tool_brush_from;
use fp_model::{ElementId, ElementKind};
use serde_json::json;

const RED: Argb = Argb(0xffff_0000);
const GREEN: Argb = Argb(0xff00_ff00);
const BLUE: Argb = Argb(0xff00_00ff);
const GREY: Argb = Argb(0xff80_8080);

fn id(v: &serde_json::Value) -> ElementId {
    ElementId(v["id"].as_u64().expect("id"))
}

/// The measurement's tool state before each case: fill #00ff00 solid, stroke red 1-Pixel Hard
/// 4 px centred, nothing selected.
fn toolx(s: &mut Session) {
    ex(s, "select.none", json!({}));
    let mut st = s.tool_style();
    st.stroke_color = RED;
    st.fill_color = GREEN;
    st.new_fill = None;
    st.new_stroke = fp_engine::tool_follow::stroke_params(
        fp_engine::commands::panel::preset_index_of("bc_Pencil", "bn_1-Pixel"),
        4,
        0,
        "center",
        None,
    );
    st.new_brush = None;
    s.set_tool_style(st);
}

fn tool_is_toolx(s: &Session) {
    let st = s.tool_style();
    assert_eq!((st.stroke_color, st.fill_color), (RED, GREEN));
    assert_eq!(st.new_fill, None);
    let b = tool_brush_from(st.new_stroke.as_ref(), st.new_brush.as_ref());
    assert_eq!((b.name.as_str(), b.diameter), ("bn_1-Pixel", 4));
}

/// R (fill #ff9900, stroke #c04040 1 px) and B (fill #3366cc, stroke black 2 px) above it.
fn two_paths(s: &mut Session) -> (ElementId, ElementId) {
    let r = id(&ex(
        s,
        "draw.rectangle",
        json!({"rect": [20, 150, 120, 230], "fillColor": "#ff9900", "strokeColor": "#c04040", "strokeWidth": 1}),
    ));
    let b = id(&ex(
        s,
        "draw.rectangle",
        json!({"rect": [200, 180, 300, 260], "fillColor": "#3366cc", "strokeColor": "#000000", "strokeWidth": 2}),
    ));
    (r, b)
}

#[test]
fn a_single_path_is_followed() {
    let mut s = doc_session(600.0, 400.0);
    let (r, _) = two_paths(&mut s);
    toolx(&mut s);
    select(&mut s, &[r]);
    let st = s.tool_style();
    assert_eq!(
        (st.stroke_color, st.fill_color),
        (Argb(0xffc0_4040), Argb(0xffff_9900))
    );
    ex(&mut s, "select.none", json!({}));
    assert_eq!(s.tool_style().fill_color, Argb(0xffff_9900), "kept after deselecting");
}

#[test]
fn several_objects_a_group_a_bitmap_and_a_hotspot_are_not_followed() {
    let mut s = doc_session(600.0, 400.0);
    let (r, b) = two_paths(&mut s);
    // several objects, both orders
    for ids in [[r, b], [b, r]] {
        toolx(&mut s);
        select(&mut s, &ids);
        tool_is_toolx(&s);
        // the wells show the topmost object's colours, the tool attributes stay
        assert_eq!(s.tool_well_colors(), (BLACK, Argb(0xff33_66cc)));
        ex(&mut s, "select.none", json!({}));
        assert_eq!(s.tool_well_colors(), (RED, GREEN));
    }
    // a group
    toolx(&mut s);
    select(&mut s, &[r, b]);
    ex(&mut s, "modify.group", json!({}));
    let g = s.active().expect("doc").selection.elements[0];
    toolx(&mut s);
    select(&mut s, &[g]);
    tool_is_toolx(&s);
    assert_eq!(s.tool_well_colors(), (RED, GREEN), "a group shows the tool");
    // a bitmap
    toolx(&mut s);
    let bm = bitmap(&mut s, 349, 19, 102, 102, [10, 20, 30, 255]);
    select(&mut s, &[bm]);
    tool_is_toolx(&s);
    // a hotspot inserted over a single selected path, then selected
    let p = id(&ex(
        &mut s,
        "draw.rectangle",
        json!({"rect": [450, 200, 550, 300], "fillColor": "#123456", "strokeColor": "#654321", "strokeWidth": 1}),
    ));
    select(&mut s, &[p]);
    assert_eq!(s.tool_style().fill_color, Argb(0xff12_3456));
    ex(&mut s, "edit.insert.hotspot", json!({}));
    assert_eq!(s.tool_style().fill_color, Argb(0xff12_3456), "inserting changes nothing");
    let hs = s.active().expect("doc").selection.elements.clone();
    toolx(&mut s);
    ex(
        &mut s,
        "select.set",
        json!({"ids": hs.iter().map(|i| i.0).collect::<Vec<_>>()}),
    );
    tool_is_toolx(&s);
}

const BLACK: Argb = Argb(0xff00_0000);

/// A path with the fill "none" and the stored fill colour #808080: the tool fill becomes none with
/// that colour; a new document then starts with a solid #808080.
#[test]
fn fill_none_takes_the_paths_stored_fill_colour() {
    let mut s = doc_session(600.0, 400.0);
    let p2 = id(&ex(
        &mut s,
        "draw.rectangle",
        json!({"rect": [450, 200, 550, 300], "fillColor": "#808080", "strokeColor": "#000000", "strokeWidth": 1}),
    ));
    ex(&mut s, "pi.fill", json!({"ids": [p2.0], "category": "none"}));
    ex(&mut s, "pi.stroke", json!({"ids": [p2.0], "placement": "outside"}));
    toolx(&mut s);
    select(&mut s, &[p2]);
    let st = s.tool_style();
    assert!(st.fill_is_none());
    assert_eq!(st.fill_color, GREY);
    assert_eq!(st.placement(), "outside");
    assert_eq!(s.tool_well_colors().1.a(), 0, "the fill well shows none");
    ex(&mut s, "select.none", json!({}));
    ex(&mut s, "file.new", json!({"width": 100, "height": 100}));
    let st = s.tool_style();
    assert!(!st.fill_is_none(), "a new document starts with a solid fill");
    assert_eq!(st.fill_color, GREY);
    assert_eq!(st.placement(), "outside", "the stroke carries over");
}

/// Text: fill = text colour, stroke none, placement outside; kept after deselecting.
#[test]
fn a_text_gives_its_colour_no_stroke_and_outside() {
    let mut s = doc_session(600.0, 400.0);
    let t = id(&ex(
        &mut s,
        "draw.text",
        json!({"text": "Abc", "anchor": [150, 40], "size": 24, "font": "Arial", "color": "#00aa00"}),
    ));
    // the measured text had the placement outside; the tool takes the text's placement
    {
        let d = s.active_mut().expect("doc");
        let page = d.page;
        fp_engine::docops::edit_by_id(d.doc_mut(), page, t, |e| {
            if let ElementKind::Text(tx) = &mut e.kind {
                tx.stroke_placement = 2;
            }
        })
        .expect("text");
    }
    toolx(&mut s);
    select(&mut s, &[t]);
    ex(&mut s, "select.none", json!({}));
    let st = s.tool_style();
    assert_eq!(st.fill_color, Argb(0xff00_aa00));
    assert!(!st.fill_is_none());
    assert!(st.stroke_is_none());
    assert_eq!(st.placement(), "outside");
    assert_eq!(s.tool_well_colors().0.a(), 0, "the stroke well shows none");
    let ElementKind::Text(tx) = &el(&s, t).kind else {
        panic!("text")
    };
    assert_eq!(tx.stroke_placement, 2);
}

/// The fill "none" belongs to the document: a new document starts solid, the first one keeps its
/// none; the stroke "none" carries over.
#[test]
fn fill_none_is_per_document_stroke_is_session_wide() {
    let mut s = doc_session(300.0, 200.0);
    let a = s.active_id().expect("doc");
    let mut st = s.tool_style();
    st.fill_color = BLUE;
    st.new_fill = Some(json!({"category": "none"}));
    st.new_stroke = Some(json!({"none": true}));
    s.set_tool_style(st);
    ex(&mut s, "file.new", json!({"width": 100, "height": 100}));
    let b = s.active_id().expect("doc");
    let st = s.tool_style();
    assert!(!st.fill_is_none());
    assert_eq!(st.fill_color, BLUE);
    assert!(st.stroke_is_none(), "stroke none carries over");
    s.activate(a).expect("activate");
    assert!(s.tool_style().fill_is_none(), "the first document keeps its none");
    s.activate(b).expect("activate");
    assert!(!s.tool_style().fill_is_none());
    // the transparent toolbox colour ("None" button) also comes back solid in its colour
    s.set_colors(None, Some(Argb(0x0012_3456)));
    ex(&mut s, "file.new", json!({"width": 100, "height": 100}));
    assert_eq!(s.tools.fill_color, Argb(0xff12_3456));
}

/// "Default Colours": colours only, tip, size and placement stay; stroke none becomes the 1-pixel
/// anti-aliased tip 1 px, fill none a solid white.
#[test]
fn default_colours_keep_the_tip_and_bring_back_none() {
    let mut s = doc_session(300.0, 200.0);
    let mut st = s.tool_style();
    st.stroke_color = RED;
    st.new_stroke = fp_engine::tool_follow::stroke_params(
        fp_engine::commands::panel::preset_index_of("bc_Pencil", "bn_1-Pixel"),
        4,
        0,
        "inside",
        None,
    );
    s.set_tool_style(st);
    ex(&mut s, "color.defaults", json!({}));
    let st = s.tool_style();
    assert_eq!((st.stroke_color, st.fill_color), (BLACK, Argb(0xffff_ffff)));
    let b = tool_brush_from(st.new_stroke.as_ref(), st.new_brush.as_ref());
    assert_eq!((b.name.as_str(), b.diameter), ("bn_1-Pixel", 4));
    assert_eq!(st.placement(), "inside");

    let mut st = s.tool_style();
    st.new_stroke = Some(json!({"none": true}));
    st.new_fill = Some(json!({"category": "none"}));
    s.set_tool_style(st);
    ex(&mut s, "color.defaults", json!({}));
    let st = s.tool_style();
    assert!(!st.stroke_is_none() && !st.fill_is_none());
    let b = tool_brush_from(st.new_stroke.as_ref(), st.new_brush.as_ref());
    assert_eq!((b.name.as_str(), b.diameter, b.antialiased), ("bn_1-Pixel Anti-Aliased", 1, true));
    assert_eq!(st.new_fill, None, "solid");
    assert_eq!(st.fill_color, Argb(0xffff_ffff));
    // the transparent "none" colours too
    s.set_colors(Some(Argb(0x00ff_0000)), Some(Argb(0x0000_ff00)));
    ex(&mut s, "color.defaults", json!({}));
    assert_eq!(s.tool_well_colors(), (BLACK, Argb(0xffff_ffff)));
}

/// After the start: `bn_1-Pixel Anti-Aliased` 2 px centred (no stored `BrushTipSize`), else the
/// stored size; a rectangle drawn like the Rectangle tool gets it.
#[test]
fn start_stroke_is_one_pixel_anti_aliased_in_the_tip_size() {
    let s = Session::default();
    let st = s.tool_style();
    let b = tool_brush_from(st.new_stroke.as_ref(), st.new_brush.as_ref());
    assert_eq!(
        (b.category.as_str(), b.name.as_str(), b.diameter, b.antialiased),
        ("bc_Pencil", "bn_1-Pixel Anti-Aliased", 2, true)
    );
    assert_eq!(st.placement(), "center");
    assert_eq!(st.new_fill, None, "solid fill");
    let mut p = fp_engine::prefs::Preferences::new();
    p.set("BrushTipSize", fp_engine::prefs::PrefValue::Int(5));
    let o = fp_engine::tools::ToolOptions::from_prefs(&p);
    let b = tool_brush_from(o.new_stroke.as_ref(), None);
    assert_eq!((b.name.as_str(), b.diameter), ("bn_1-Pixel Anti-Aliased", 5));
}
