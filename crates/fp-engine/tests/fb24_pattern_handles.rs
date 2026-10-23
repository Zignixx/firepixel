//! FB-24 / FB-25 / TW-207 (RE spec 61 §1): a new pattern fill gets the measured default handles
//! (centre of the bounds, one tile right, one tile up); every kind of move takes all three fill
//! handles along by the same delta and undo puts them back; a scale or rotation with "Scale
//! attributes" off leaves the handles in the document.

use crate::common;
use common::*;
use fp_engine::Session;
use fp_model::{ElementId, ElementKind, FillHandles};
use serde_json::json;

/// Test pattern 16 × 12 in the test profile's pattern folder.
fn write_pattern() {
    let _ = doc_session(10.0, 10.0); // test profile
    let user = fp_i18n::profile::roaming_dir().expect("test profile");
    let path = user.join("Patterns").join("Fb24Muster.png");
    let mut s = doc_session(16.0, 12.0);
    let _ = rect(&mut s, 0.0, 0.0, 8.0, 12.0);
    std::fs::create_dir_all(path.parent().expect("folder")).expect("create folder");
    ex(
        &mut s,
        "file.exportArea",
        json!({"rect": [0, 0, 16, 12], "path": path.to_string_lossy(), "settings": {"format": "png32"}}),
    );
    assert!(path.is_file(), "{path:?}");
}

fn handles(s: &Session, id: ElementId) -> FillHandles {
    let ElementKind::Path(p) = &el(s, id).kind else {
        panic!("path")
    };
    p.fill_handles
}

fn hv(h: FillHandles) -> [(f32, f32); 3] {
    [
        (h.start.x, h.start.y),
        (h.end.x, h.end.y),
        (h.second.x, h.second.y),
    ]
}

fn shifted(h: FillHandles, dx: f32, dy: f32) -> [(f32, f32); 3] {
    hv(h).map(|(x, y)| (x + dx, y + dy))
}

fn pattern_rect(s: &mut Session) -> ElementId {
    let r = rect(s, 100.0, 100.0, 260.0, 220.0);
    select(s, &[r]);
    ex(s, "pi.fill", json!({"pattern": "Fb24Muster"}));
    r
}

#[test]
fn new_pattern_fill_gets_centre_and_tile_sized_handles() {
    write_pattern();
    let mut s = doc_session(600.0, 400.0);
    let r = pattern_rect(&mut s);
    assert_eq!(
        hv(handles(&s, r)),
        [(180.0, 160.0), (196.0, 160.0), (180.0, 148.0)],
        "pi.fill: centre, +tile width, −tile height"
    );
    // choosing the pattern category again keeps the handles
    ex(&mut s, "pi.fill", json!({"category": "pattern"}));
    assert_eq!(hv(handles(&s, r))[0], (180.0, 160.0));
    // element.setPattern on a solid fill: the same default
    let r2 = rect(&mut s, 300.0, 50.0, 400.0, 90.0);
    ex(
        &mut s,
        "element.setPattern",
        json!({"ids": [r2.0], "name": "Fb24Muster"}),
    );
    assert_eq!(
        hv(handles(&s, r2)),
        [(350.0, 70.0), (366.0, 70.0), (350.0, 58.0)]
    );
    // a second element.setPattern on a pattern fill keeps the (moved) handles
    ex(
        &mut s,
        "element.setFillHandles",
        json!({"ids": [r2.0], "start": [340, 60], "end": [356, 60], "second": [340, 48]}),
    );
    ex(
        &mut s,
        "element.setPattern",
        json!({"ids": [r2.0], "name": "Fb24Muster"}),
    );
    assert_eq!(hv(handles(&s, r2))[0], (340.0, 60.0));
}

#[test]
fn every_move_takes_all_fill_handles_along() {
    write_pattern();
    let mut s = doc_session(600.0, 400.0);
    let r = pattern_rect(&mut s);
    let h0 = handles(&s, r);
    // move by script (37, 23)
    ex(&mut s, "edit.move", json!({"dx": 37, "dy": 23}));
    assert_eq!(hv(handles(&s, r)), shifted(h0, 37.0, 23.0));
    assert_eq!(bounds(&s, r).left, 137.0);
    // undo: handles back
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(hv(handles(&s, r)), hv(h0));
    // X/Y fields (element.setBounds): a pure move also moves the handles
    ex(&mut s, "element.setBounds", json!({"x": 200, "y": 150}));
    assert_eq!(hv(handles(&s, r)), shifted(h0, 100.0, 50.0));
    ex(&mut s, "edit.undo", json!({}));
    // duplicate (+10/+10): the copy's handles moved by the same delta
    ex(&mut s, "edit.duplicate", json!({}));
    let dup = s
        .active()
        .and_then(|d| d.selection.elements.first().copied())
        .expect("copy");
    assert_ne!(dup, r);
    assert_eq!(hv(handles(&s, dup)), shifted(h0, 10.0, 10.0));
    assert_eq!(hv(handles(&s, r)), hv(h0), "original stays");
}

#[test]
fn scale_moves_handles_only_with_scale_attributes() {
    write_pattern();
    let mut s = doc_session(600.0, 400.0);
    let r = pattern_rect(&mut s);
    let h0 = handles(&s, r);
    ex(
        &mut s,
        "modify.transform.numericTransform",
        json!({"mode": "scale", "scaleX": 150, "scaleY": 150, "scaleAttributes": true}),
    );
    // about the centre (180,160): h2 (196,160) → (204,160), h3 (180,148) → (180,142)
    assert_eq!(
        hv(handles(&s, r)),
        [(180.0, 160.0), (204.0, 160.0), (180.0, 142.0)]
    );
    ex(&mut s, "edit.undo", json!({}));
    ex(
        &mut s,
        "modify.transform.numericTransform",
        json!({"mode": "scale", "scaleX": 150, "scaleY": 150, "scaleAttributes": false}),
    );
    assert_eq!(
        hv(handles(&s, r)),
        hv(h0),
        "Scale attributes off: handles stay in the document"
    );
    assert!(bounds(&s, r).width() > 200.0, "the shape itself is scaled");
    ex(&mut s, "edit.undo", json!({}));
    ex(
        &mut s,
        "modify.transform.numericTransform",
        json!({"mode": "rotate", "angle": 30, "scaleAttributes": false}),
    );
    assert_eq!(
        hv(handles(&s, r)),
        hv(h0),
        "rotation without Scale attributes"
    );
    ex(&mut s, "edit.undo", json!({}));
    ex(
        &mut s,
        "modify.transform.numericTransform",
        json!({"mode": "rotate", "angle": 90, "scaleAttributes": true}),
    );
    let h = handles(&s, r);
    // rotated 90° clockwise about the centre: h2 = h1 + (0, 16), h3 = h1 + (12, 0)
    assert!(approx(h.end.x, 180.0) && approx(h.end.y, 176.0), "{h:?}");
    assert!(
        approx(h.second.x, 192.0) && approx(h.second.y, 160.0),
        "{h:?}"
    );
}

fn stroke_diameter(s: &Session, id: ElementId) -> Option<u32> {
    fp_engine::docops::path_attrs_of(&el(s, id))
        .and_then(|a| a.brush.as_ref().map(|b| b.diameter as u32))
}

/// Scale tool: drag the bottom-right corner of the rectangle (100,100)–(260,220) to (420,340)
/// (scale 2 about the top-left corner) and confirm with Enter.
fn scale_by_tool(s: &mut Session) {
    s.set_tool(fp_engine::ToolId::Scale).expect("Scale tool");
    events(
        s,
        &[
            fp_engine::ToolEvent::down(260.0, 220.0),
            fp_engine::ToolEvent::drag(340.0, 280.0),
            fp_engine::ToolEvent::drag(420.0, 340.0),
            fp_engine::ToolEvent::up(420.0, 340.0),
            fp_engine::ToolEvent::key(fp_engine::ToolKey::Enter),
        ],
    );
}

/// TW-207 rest (RE spec 61 §1.5): the transform tools transform the fill handles only while
/// `XformToolXformsBFE` ("Scale strokes and effects", default on) is on; off, the handles stay in
/// the document. The stroke width rule of the tools does not depend on the flag.
#[test]
fn transform_tools_honour_xform_tool_pref_for_fill_handles() {
    write_pattern();
    // on (default): the handles scale with the shape about the top-left corner (100,100)
    let mut s = doc_session(600.0, 400.0);
    let r = pattern_rect(&mut s);
    let h0 = handles(&s, r);
    assert!(s.prefs.get_bool("XformToolXformsBFE", false), "default on");
    scale_by_tool(&mut s);
    assert!(
        rect_approx(bounds(&s, r), fp_geom::Rect::new(100.0, 100.0, 420.0, 340.0)),
        "{:?}",
        bounds(&s, r)
    );
    let h = hv(handles(&s, r));
    let want = [(260.0, 220.0), (292.0, 220.0), (260.0, 196.0)];
    for (a, b) in h.iter().zip(want) {
        assert!(approx(a.0, b.0) && approx(a.1, b.1), "{h:?}");
    }
    let stroke_on = stroke_diameter(&s, r);

    // off: the shape is scaled, the handles stay in the document
    let mut s = doc_session(600.0, 400.0);
    s.prefs
        .set("XformToolXformsBFE", fp_engine::PrefValue::Bool(false));
    let r = pattern_rect(&mut s);
    assert_eq!(hv(handles(&s, r)), hv(h0));
    scale_by_tool(&mut s);
    assert!(
        rect_approx(bounds(&s, r), fp_geom::Rect::new(100.0, 100.0, 420.0, 340.0)),
        "the shape itself is scaled: {:?}",
        bounds(&s, r)
    );
    assert_eq!(hv(handles(&s, r)), hv(h0), "flag off: handles unchanged");
    assert_eq!(stroke_diameter(&s, r), stroke_on, "stroke width rule unchanged");
    // undo puts the shape back, the handles were never moved
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(bounds(&s, r).right, 260.0);
    assert_eq!(hv(handles(&s, r)), hv(h0));

    // Distort tool with the flag off: the handles stay too
    s.set_tool(fp_engine::ToolId::Distort).expect("Distort tool");
    events(
        &mut s,
        &[
            fp_engine::ToolEvent::down(260.0, 220.0),
            fp_engine::ToolEvent::drag(270.0, 225.0),
            fp_engine::ToolEvent::drag(280.0, 230.0),
            fp_engine::ToolEvent::up(280.0, 230.0),
            fp_engine::ToolEvent::key(fp_engine::ToolKey::Enter),
        ],
    );
    assert!(bounds(&s, r).right > 260.0, "distorted");
    assert_eq!(hv(handles(&s, r)), hv(h0), "distort, flag off");

    // the numeric transform does not read the tool flag (its own parameter, default on)
    ex(&mut s, "edit.undo", json!({}));
    ex(
        &mut s,
        "modify.transform.numericTransform",
        json!({"mode": "scale", "scaleX": 150, "scaleY": 150}),
    );
    assert_eq!(
        hv(handles(&s, r)),
        [(180.0, 160.0), (204.0, 160.0), (180.0, 142.0)]
    );
}

/// `transform.matrix` keeps taking the fill handles along on a pure move even with
/// `scaleAttributes` off.
#[test]
fn transform_matrix_move_takes_handles_along_without_scale_attributes() {
    write_pattern();
    let mut s = doc_session(600.0, 400.0);
    let r = pattern_rect(&mut s);
    let h0 = handles(&s, r);
    ex(
        &mut s,
        "transform.matrix",
        json!({"matrix": [1, 0, 0, 1, 10, 5], "scaleAttributes": false}),
    );
    assert_eq!(hv(handles(&s, r)), shifted(h0, 10.0, 5.0));
    ex(
        &mut s,
        "transform.matrix",
        json!({"matrix": [2, 0, 0, 2, 0, 0], "scaleAttributes": false}),
    );
    assert_eq!(hv(handles(&s, r)), shifted(h0, 10.0, 5.0), "scale: handles stay");
}
