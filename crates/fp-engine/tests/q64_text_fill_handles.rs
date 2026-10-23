//! Q-64 (RE spec 62 §2): a text takes gradient and pattern fills with fill handles in document
//! coordinates. New linear gradient: h1 = top centre, h2 = bottom centre of the object bounds,
//! h3 = h1 + (height, 0); new pattern: h1 = centre, h2 = h1 + (tile w, 0), h3 = h1 + (0, −tile h).
//! Every move takes the handles along (image identical after the move), the Scale tool and Free
//! Transform leave them, the numeric transform (`scaleSelection`) scales them about the centre
//! whatever "Scale attributes" says, and editing the text leaves them where they are.

use crate::common;
use common::*;
use fp_engine::Session;
use fp_model::{ElementId, ElementKind, FillHandles};
use serde_json::json;

/// Test pattern 94 × 88 (the size of the reference's "Weave" tile) in the test profile, written
/// once per test program (parallel tests must not read a half-written file).
fn write_pattern() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(write_pattern_file);
}

fn write_pattern_file() {
    let _ = doc_session(10.0, 10.0); // test profile
    let user = fp_i18n::profile::roaming_dir().expect("test profile");
    let path = user.join("Patterns").join("Q64Muster.png");
    let mut s = doc_session(94.0, 88.0);
    let _ = rect(&mut s, 0.0, 0.0, 40.0, 30.0);
    std::fs::create_dir_all(path.parent().expect("folder")).expect("create folder");
    ex(
        &mut s,
        "file.exportArea",
        json!({"rect": [0, 0, 94, 88], "path": path.to_string_lossy(), "settings": {"format": "png32"}}),
    );
    assert!(path.is_file(), "{path:?}");
}

fn handles(s: &Session, id: ElementId) -> FillHandles {
    match &el(s, id).kind {
        ElementKind::Text(t) => t.fill_handles,
        ElementKind::Path(p) => p.fill_handles,
        _ => panic!("text or path"),
    }
}

fn hv(h: FillHandles) -> [(f32, f32); 3] {
    [
        (h.start.x, h.start.y),
        (h.end.x, h.end.y),
        (h.second.x, h.second.y),
    ]
}

fn shifted(h: [(f32, f32); 3], dx: f32, dy: f32) -> [(f32, f32); 3] {
    h.map(|(x, y)| (x + dx, y + dy))
}

fn assert_near(got: [(f32, f32); 3], want: [(f32, f32); 3], what: &str) {
    for (g, w) in got.iter().zip(want) {
        assert!(
            (g.0 - w.0).abs() < 1e-3 && (g.1 - w.1).abs() < 1e-3,
            "{what}: {got:?} != {want:?}"
        );
    }
}

/// "Ag" ArialMT 60 px at the baseline 100/150, selected.
fn text(s: &mut Session) -> ElementId {
    let v = ex(
        s,
        "draw.text",
        json!({"text": "Ag", "anchor": [100, 150], "size": 60, "font": "ArialMT", "color": "#000000"}),
    );
    let id = ElementId(v["id"].as_u64().unwrap_or(0));
    select(s, &[id]);
    id
}

/// Handles of a new linear gradient on the bounds `b` (object bounds as the Inspector shows them).
fn linear_default(b: fp_geom::Rect) -> [(f32, f32); 3] {
    let cx = (b.left + b.right) / 2.0;
    [(cx, b.top), (cx, b.bottom), (cx + b.height(), b.top)]
}

fn gradient_text(s: &mut Session) -> ElementId {
    let t = text(s);
    ex(s, "pi.fill", json!({"category": "gradient"}));
    t
}

#[test]
fn new_gradient_on_text_gets_top_and_bottom_centre_handles() {
    let mut s = doc_session(600.0, 400.0);
    let t = gradient_text(&mut s);
    let b = visible_bounds(&s, t);
    assert_eq!(hv(handles(&s, t)), linear_default(b));
}

#[test]
fn new_linear_gradient_on_a_path_puts_h3_to_the_right() {
    // RE spec 61 §1.2 / 62 §2.2: h3 = h1 + (height, 0); 100,100–260,220 → h3 (300,100)
    let mut s = doc_session(600.0, 400.0);
    let r = rect(&mut s, 100.0, 100.0, 260.0, 220.0);
    select(&mut s, &[r]);
    ex(&mut s, "pi.fill", json!({"category": "gradient"}));
    assert_eq!(
        hv(handles(&s, r)),
        [(180.0, 100.0), (180.0, 220.0), (300.0, 100.0)]
    );
}

#[test]
fn new_pattern_on_text_gets_centre_and_tile_sized_handles() {
    write_pattern();
    let mut s = doc_session(600.0, 400.0);
    let t = text(&mut s);
    ex(&mut s, "pi.fill", json!({"pattern": "Q64Muster"}));
    let b = visible_bounds(&s, t);
    let (cx, cy) = ((b.left + b.right) / 2.0, (b.top + b.bottom) / 2.0);
    assert_eq!(
        hv(handles(&s, t)),
        [(cx, cy), (cx + 94.0, cy), (cx, cy - 88.0)]
    );
}

/// Page pixels of the text's neighbourhood, `d` = offset of the window.
fn pixels(s: &Session, d: (i32, i32)) -> Vec<[u8; 4]> {
    let doc = &s.active().expect("document").doc;
    let opts = fp_render::RenderOptions {
        cache_policy: fp_render::CachePolicy::None,
        ..fp_render::RenderOptions::default()
    };
    let img = fp_render::Renderer::new(opts).render_page(doc, 0, 0).image;
    (60..200)
        .flat_map(|y| (80..240).map(move |x| (x, y)))
        .map(|(x, y)| {
            let p = img.pixel(x + d.0, y + d.1);
            [p.r, p.g, p.b, p.a]
        })
        .collect()
}

#[test]
fn every_move_takes_the_text_handles_along_and_keeps_the_image() {
    for kind in ["gradient", "pattern"] {
        write_pattern();
        let mut s = doc_session(600.0, 400.0);
        let t = text(&mut s);
        let solid = pixels(&s, (0, 0));
        if kind == "gradient" {
            ex(&mut s, "pi.fill", json!({"category": "gradient"}));
        } else {
            ex(&mut s, "pi.fill", json!({"pattern": "Q64Muster"}));
        }
        let h0 = hv(handles(&s, t));
        let img0 = pixels(&s, (0, 0));
        assert!(solid.iter().any(|p| p[0] < 128), "{kind}: ink");
        assert_ne!(
            img0, solid,
            "{kind}: the fill paints the glyphs, not the run colour"
        );
        // drag / arrows (moveSelectionBy)
        ex(&mut s, "edit.move", json!({"dx": 40, "dy": 30}));
        assert_eq!(hv(handles(&s, t)), shifted(h0, 40.0, 30.0), "{kind}");
        assert_eq!(pixels(&s, (40, 30)), img0, "{kind}: image after the move");
        // X/Y fields: a pure move as well
        let b = visible_bounds(&s, t);
        ex(
            &mut s,
            "element.setBounds",
            json!({"x": b.left + 60.0, "y": b.top + 20.0}),
        );
        assert_eq!(hv(handles(&s, t)), shifted(h0, 100.0, 50.0), "{kind}");
        assert_eq!(pixels(&s, (100, 50)), img0, "{kind}: image after X/Y");
        ex(&mut s, "edit.undo", json!({}));
        ex(&mut s, "edit.undo", json!({}));
        assert_eq!(hv(handles(&s, t)), h0, "{kind}: undo");
    }
}

#[test]
fn numeric_scale_scales_text_handles_about_the_centre_with_and_without_scale_attributes() {
    for attrs in [true, false] {
        let mut s = doc_session(600.0, 400.0);
        let t = gradient_text(&mut s);
        let h0 = hv(handles(&s, t));
        let b = visible_bounds(&s, t);
        let c = b.center();
        ex(
            &mut s,
            "modify.transform.numericTransform",
            json!({"mode": "scale", "scaleX": 150, "scaleY": 150, "scaleAttributes": attrs}),
        );
        let want = h0.map(|(x, y)| (c.x + (x - c.x) * 1.5, c.y + (y - c.y) * 1.5));
        assert_near(
            hv(handles(&s, t)),
            want,
            &format!("scaleAttributes {attrs}"),
        );
        ex(&mut s, "edit.undo", json!({}));
        assert_eq!(hv(handles(&s, t)), h0, "undo");
    }
}

#[test]
fn scale_tool_and_free_transform_leave_text_handles() {
    let mut s = doc_session(600.0, 400.0);
    let t = gradient_text(&mut s);
    let h0 = hv(handles(&s, t));
    let b0 = visible_bounds(&s, t);
    // the transform tools send transform.matrix with the "Scale strokes and effects" flag (on)
    ex(
        &mut s,
        "transform.matrix",
        json!({"matrix": [1.5, 0, 0, 1.4, 0, 0], "center": [b0.left, b0.top], "scaleAttributes": true}),
    );
    assert!(visible_bounds(&s, t).width() > b0.width() * 1.3, "scaled");
    assert_eq!(hv(handles(&s, t)), h0, "transform.matrix");
    ex(&mut s, "edit.undo", json!({}));
    // the Scale tool itself: drag the bottom-right corner of the text's transform frame (RE spec
    // 67) by +39/+38 and confirm
    let f = {
        let d = s.active().expect("document");
        let r = s.renderer().cloned().expect("renderer");
        let fit = |x: &fp_model::TextElem| r.text_fit_boxes(&d.doc, x);
        fp_engine::docops::transform_frame(&d.doc, d.page, &[t], Some(&fit))
            .expect("frame")
            .rect
    };
    s.set_tool(fp_engine::ToolId::Scale).expect("Scale tool");
    let (x, y) = (f.right, f.bottom);
    events(
        &mut s,
        &[
            fp_engine::ToolEvent::down(x, y),
            fp_engine::ToolEvent::drag(x + 20.0, y + 19.0),
            fp_engine::ToolEvent::drag(x + 39.0, y + 38.0),
            fp_engine::ToolEvent::up(x + 39.0, y + 38.0),
            fp_engine::ToolEvent::key(fp_engine::ToolKey::Enter),
        ],
    );
    assert!(
        visible_bounds(&s, t).width() > b0.width() + 20.0,
        "{:?}",
        visible_bounds(&s, t)
    );
    assert_eq!(hv(handles(&s, t)), h0, "Scale tool");
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(visible_bounds(&s, t), b0, "undo");
    // dragging inside the frame in transform mode moves the text, the handles stay (§2.4)
    s.set_tool(fp_engine::ToolId::Scale).expect("Scale tool");
    // inside the frame, away from the pivot in the centre and from the handles
    let c = fp_geom::Point::new(b0.center().x - 15.0, b0.center().y + 12.0);
    events(
        &mut s,
        &[
            fp_engine::ToolEvent::down(c.x, c.y),
            fp_engine::ToolEvent::drag(c.x + 20.0, c.y + 15.0),
            fp_engine::ToolEvent::drag(c.x + 40.0, c.y + 30.0),
            fp_engine::ToolEvent::up(c.x + 40.0, c.y + 30.0),
            fp_engine::ToolEvent::key(fp_engine::ToolKey::Enter),
        ],
    );
    let b = visible_bounds(&s, t);
    assert!(
        (b.left - b0.left - 40.0).abs() < 1e-3 && (b.top - b0.top - 30.0).abs() < 1e-3,
        "moved in transform mode: {b:?}"
    );
    assert_eq!(hv(handles(&s, t)), h0, "move inside the transform frame");
    // the same translation from a script or the arrows takes them along
    ex(
        &mut s,
        "transform.matrix",
        json!({"matrix": [1, 0, 0, 1, 10, 5]}),
    );
    assert_eq!(hv(handles(&s, t)), shifted(h0, 10.0, 5.0), "plain move");
}

#[test]
fn editing_the_text_leaves_its_handles() {
    let mut s = doc_session(600.0, 400.0);
    let t = gradient_text(&mut s);
    let h0 = hv(handles(&s, t));
    let w0 = visible_bounds(&s, t).width();
    ex(&mut s, "text.setText", json!({"text": "Agxyz"}));
    assert!(visible_bounds(&s, t).width() > w0, "longer text");
    assert_eq!(hv(handles(&s, t)), h0, "longer text: no re-centring");
    ex(&mut s, "text.setAttributes", json!({"size": 90}));
    assert_eq!(hv(handles(&s, t)), h0, "larger font size");
}
