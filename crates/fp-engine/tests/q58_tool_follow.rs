//! Q-58 (TW-192, RE spec 58 section 2.2): the tool attributes (colours of the tools panel, the
//! inspector's tool fill / stroke / brush) take on those of the single selected path or text once
//! the command that changed the selection is done - not inside a command group or a running script -
//! and keep them after deselecting.

use crate::common;

use common::*;
use fp_color::Argb;
use fp_engine::Session;
use fp_model::{ElementId, ElementKind};
use serde_json::json;

/// R1: fill #ff0000, stroke #00ff00 4 px outside; R2 above it: fill #0000ff, no stroke.
fn two_rects(s: &mut Session) -> (ElementId, ElementId) {
    let v = ex(
        s,
        "draw.rectangle",
        json!({"rect": [10, 10, 60, 40], "fillColor": "#ff0000", "strokeColor": "#00ff00", "strokeWidth": 4}),
    );
    let r1 = ElementId(v["id"].as_u64().expect("id"));
    ex(
        s,
        "pi.stroke",
        json!({"ids": [r1.0], "placement": "outside"}),
    );
    let v = ex(
        s,
        "draw.rectangle",
        json!({"rect": [80, 10, 130, 40], "fillColor": "#0000ff"}),
    );
    let r2 = ElementId(v["id"].as_u64().expect("id"));
    (r1, r2)
}

fn assert_r1_style(s: &Session) {
    let st = s.tool_style();
    assert_eq!(st.fill_color, Argb(0xffff_0000));
    assert_eq!(st.stroke_color, Argb(0xff00_ff00));
    assert_eq!(st.new_fill, None, "solid fill = the tool default");
    let ns = st.new_stroke.expect("stroke parameters");
    assert_eq!(ns["width"], json!(4));
    assert_eq!(ns["placement"], json!("outside"));
    let b = fp_engine::commands::panel::tool_brush_from(Some(&ns), st.new_brush.as_ref());
    assert_eq!(b.diameter, 4);
}

#[test]
fn selecting_an_object_sets_the_tool_attributes_and_deselecting_keeps_them() {
    let mut s = doc_session(200.0, 100.0);
    let (r1, _) = two_rects(&mut s);
    select(&mut s, &[r1]);
    assert_r1_style(&s);
    ex(&mut s, "select.none", json!({}));
    assert_r1_style(&s);
    // a shape drawn like the Rectangle tool takes them: fill, stroke colour, tip, size, placement
    let inv = fp_engine::tools::styled(
        fp_engine::tools::Invocation::new(
            "draw.rectangle",
            json!({"rect": [10, 60, 60, 90], "strokeWidth": 1}),
        ),
        &s.tools.options,
    );
    let v = ex(&mut s, &inv.command, inv.params);
    let e = el(&s, ElementId(v["id"].as_u64().expect("id")));
    let ElementKind::Path(p) = &e.kind else {
        panic!("path")
    };
    assert_eq!(p.stroke_placement, 2, "outside");
    assert_eq!(p.attrs.fill_color, Argb(0xffff_0000));
    assert_eq!(p.attrs.stroke_color, Argb(0xff00_ff00));
    assert_eq!(p.attrs.brush.as_ref().map(|b| b.diameter), Some(4));
}

/// Several objects: the tool attributes stay (RE spec 59 section 2.4, formerly the topmost counted);
/// the toolbox wells show the topmost one (R2, no stroke: "none").
#[test]
fn several_selected_keep_the_tool_attributes() {
    let mut s = doc_session(200.0, 100.0);
    let (r1, r2) = two_rects(&mut s);
    select(&mut s, &[r1]);
    select(&mut s, &[r1, r2]);
    assert_r1_style(&s);
    let (stroke, fill) = s.tool_well_colors();
    assert_eq!(fill, Argb(0xff00_00ff));
    assert_eq!(stroke.a(), 0);
}

/// Inside a command group (one script step) nothing follows; after the group it does.
#[test]
fn no_follow_inside_a_group() {
    let mut s = doc_session(200.0, 100.0);
    let (r1, r2) = two_rects(&mut s);
    select(&mut s, &[r2]);
    let before = s.tool_style();
    s.run_group("Script", |s| {
        s.execute("select.set", json!({"ids": [r1.0]}))?;
        assert_eq!(s.tool_style(), before, "unchanged inside the group");
        Ok(())
    })
    .expect("group");
    assert_r1_style(&s);
}

/// A held follow (running script) waits until it is released.
#[test]
fn held_follow_waits_for_release() {
    let mut s = doc_session(200.0, 100.0);
    let (r1, r2) = two_rects(&mut s);
    select(&mut s, &[r2]);
    let before = s.tool_style();
    s.hold_tool_follow();
    select(&mut s, &[r1]);
    assert_eq!(s.tool_style(), before);
    s.release_tool_follow();
    assert!(s.follow_selection());
    assert_r1_style(&s);
    // nothing new to look at: no change, no new generation
    let generation = s.tool_attrs_generation();
    assert!(!s.follow_selection());
    assert_eq!(s.tool_attrs_generation(), generation);
}

/// A selected bitmap has no path attributes: the tool attributes stay.
#[test]
fn bitmap_selection_keeps_the_tool_attributes() {
    let mut s = doc_session(200.0, 100.0);
    let (r1, _) = two_rects(&mut s);
    select(&mut s, &[r1]);
    let b = bitmap(&mut s, 150, 50, 20, 20, [10, 20, 30, 255]);
    select(&mut s, &[b]);
    assert_r1_style(&s);
}
