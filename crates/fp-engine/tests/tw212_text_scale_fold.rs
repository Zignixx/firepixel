//! TW-212 (RE spec 66): a pure axis scale of a text folds into its attributes instead of a matrix
//! on the paths of the reference that fold: the numeric scale (`scaleSelection`) for every text of
//! the selection, also inside groups, and the Scale tool / Free Transform / `setQuadrangle` with
//! `rememberQuad` for a single selected text. Size × sy (0.01 pt, at least 4 pt), horizontal scale
//! × sx/sy (0.001); the text is laid out again and a small leftover matrix fits the new set box onto
//! the scaled old one (≈ 1 → no matrix, whole-pixel origin). Rotation, skew, a non-uniform scale of
//! a turned text and the Scale tool on a group keep the matrix.

use crate::common;
use common::*;
use fp_engine::Session;
use fp_geom::{Matrix3, Rect};
use fp_model::{ElementId, ElementKind, TextElem};
use serde_json::json;

fn arial() -> bool {
    let ok = fp_text::FontDb::shared().find_ps("ArialMT").is_some();
    if !ok {
        eprintln!("skipped: ArialMT not installed");
    }
    ok
}

/// "Ag" ArialMT 60 px at the baseline 100/150 (the spec's `PT`), selected.
fn point_text(s: &mut Session) -> ElementId {
    point_text_at(s, 100.0, 150.0, 60.0)
}

fn point_text_at(s: &mut Session, x: f32, y: f32, size: f32) -> ElementId {
    let v = ex(
        s,
        "draw.text",
        json!({"text": "Ag", "anchor": [x, y], "size": size, "font": "ArialMT", "color": "#000000"}),
    );
    let id = ElementId(v["id"].as_u64().unwrap_or(0));
    select(s, &[id]);
    id
}

fn text(s: &Session, id: ElementId) -> TextElem {
    match &el(s, id).kind {
        ElementKind::Text(t) => (**t).clone(),
        _ => panic!("no text"),
    }
}

fn size(t: &TextElem) -> f32 {
    t.initial.size.unwrap_or(0.0)
}

fn hscale(t: &TextElem) -> f32 {
    t.initial.horizontal_scale.unwrap_or(1.0)
}

fn numeric(s: &mut Session, sx: f64, sy: f64) {
    ex(
        s,
        "modify.transform.numericTransform",
        json!({"mode": "scale", "scaleX": sx, "scaleY": sy, "constrain": false}),
    );
}

fn undo_count(s: &Session) -> usize {
    s.active().expect("document").history.undo_count()
}

fn center(r: Rect) -> (f32, f32) {
    ((r.left + r.right) / 2.0, (r.top + r.bottom) / 2.0)
}

#[test]
fn numeric_uniform_150_gives_90pt_without_a_matrix() {
    if !arial() {
        return;
    }
    let mut s = doc_session(600.0, 400.0);
    let id = point_text(&mut s);
    let b0 = visible_bounds(&s, id);
    let n0 = undo_count(&s);
    numeric(&mut s, 150.0, 150.0);
    assert_eq!(undo_count(&s), n0 + 1, "one step");
    let t = text(&s, id);
    assert_eq!(size(&t), 90.0);
    assert_eq!(hscale(&t), 1.0);
    assert_eq!(t.transform, None, "leftover ≈ 1 becomes no matrix");
    // whole-pixel origin, scaled about the centre of the bounds
    let a = t.anchor_rect.expect("anchor");
    assert_eq!((a.left.fract(), a.top.fract()), (0.0, 0.0), "{a:?}");
    let (c0, c1) = (center(b0), center(visible_bounds(&s, id)));
    assert!(
        (c0.0 - c1.0).abs() <= 1.0 && (c0.1 - c1.1).abs() <= 1.0,
        "{c0:?} {c1:?}"
    );
    let b1 = visible_bounds(&s, id);
    assert!(
        (b1.width() - 4.0 - (b0.width() - 4.0) * 1.5).abs() <= 1.5,
        "{b0:?} {b1:?}"
    );
    // a fresh 90 pt text at the same anchor has the same frame
    let fresh = point_text_at(&mut s, a.left, a.top, 90.0);
    assert_eq!(text(&s, fresh).bounds, t.bounds);
    // undo restores size, matrix and place in one step
    ex(&mut s, "edit.undo", json!({}));
    ex(&mut s, "edit.undo", json!({}));
    let t = text(&s, id);
    assert_eq!((size(&t), t.transform), (60.0, None));
    assert_eq!(visible_bounds(&s, id), b0, "undo");
}

#[test]
fn numeric_150_by_100_folds_into_the_horizontal_scale() {
    if !arial() {
        return;
    }
    let mut s = doc_session(600.0, 400.0);
    let id = point_text(&mut s);
    numeric(&mut s, 150.0, 100.0);
    let t = text(&s, id);
    assert_eq!((size(&t), hscale(&t)), (60.0, 1.5));
    assert!(
        t.transform.is_none_or(|m| m.is_translation(0.02)),
        "{:?}",
        t.transform
    );
    // 2 / 1.5: 90 pt, horizontal scale 1.333 (stored as f32 like the reference's 1.3329999)
    let mut s = doc_session(600.0, 400.0);
    let id = point_text(&mut s);
    numeric(&mut s, 200.0, 150.0);
    let t = text(&s, id);
    assert_eq!((size(&t), hscale(&t)), (90.0, 1.333));
}

#[test]
fn numeric_y_only_changes_size_and_horizontal_scale() {
    if !arial() {
        return;
    }
    let mut s = doc_session(600.0, 400.0);
    let id = point_text(&mut s);
    numeric(&mut s, 100.0, 200.0);
    let t = text(&s, id);
    assert_eq!((size(&t), hscale(&t)), (120.0, 0.5));
    // 1 / 0.5: 30 pt, horizontal scale 2
    let mut s = doc_session(600.0, 400.0);
    let id = point_text(&mut s);
    numeric(&mut s, 100.0, 50.0);
    let t = text(&s, id);
    assert_eq!((size(&t), hscale(&t)), (30.0, 2.0));
}

#[test]
fn leftover_matrix_fits_the_new_set_box() {
    if !arial() {
        return;
    }
    // 1.25: target 92.5 px wide, the 75 pt set is 93 → [0.995, 1] as measured
    let mut s = doc_session(600.0, 400.0);
    let id = point_text(&mut s);
    numeric(&mut s, 125.0, 125.0);
    let t = text(&s, id);
    assert_eq!(size(&t), 75.0);
    let m = t.transform.expect("leftover matrix");
    assert!((m.m[0][0] - 0.9946).abs() < 0.002, "{m:?}");
    assert!((m.m[1][1] - 1.0).abs() < 0.01, "{m:?}");
    assert_eq!((m.m[0][1], m.m[1][0]), (0.0, 0.0));
    // 1.40625 → 84.38 pt (f32 84.379997)
    let mut s = doc_session(600.0, 400.0);
    let id = point_text(&mut s);
    numeric(&mut s, 140.625, 140.625);
    assert_eq!(size(&text(&s, id)), 84.38);
    // 0.01 → the 4 pt floor, the rest stays as a matrix
    let mut s = doc_session(600.0, 400.0);
    let id = point_text(&mut s);
    numeric(&mut s, 1.0, 1.0);
    let t = text(&s, id);
    assert_eq!(size(&t), 4.0);
    let m = t.transform.expect("leftover matrix");
    assert!(m.m[0][0] < 0.3 && m.m[1][1] < 0.3, "{m:?}");
}

#[test]
fn turned_text_folds_in_text_space_or_keeps_the_matrix() {
    if !arial() {
        return;
    }
    // 90° then 2/1: the document's x is the text's y → 120 pt, horizontal scale 0.5, the rotation
    // stays with a small leftover
    let mut s = doc_session(600.0, 400.0);
    let id = point_text(&mut s);
    ex(&mut s, "modify.transform.rotate90DegreeCw", json!({}));
    numeric(&mut s, 200.0, 100.0);
    let t = text(&s, id);
    assert_eq!((size(&t), hscale(&t)), (120.0, 0.5));
    let m = t.transform.expect("rotation");
    assert!(
        m.m[0][0].abs() < 1e-4 && (m.m[0][1] - 1.0).abs() < 0.03,
        "{m:?}"
    );
    assert!(
        (m.m[1][0] + 1.0).abs() < 0.03 && m.m[1][1].abs() < 1e-4,
        "{m:?}"
    );
    // 30° then 1.5 uniform: 90 pt, the matrix stays a pure rotation
    let mut s = doc_session(600.0, 400.0);
    let id = point_text(&mut s);
    ex(
        &mut s,
        "modify.transform.numericTransform",
        json!({"mode": "rotate", "angle": 30.0}),
    );
    numeric(&mut s, 150.0, 150.0);
    let t = text(&s, id);
    assert_eq!(size(&t), 90.0);
    let m = t.transform.expect("rotation");
    let (a, b, c, d) = (m.m[0][0], m.m[0][1], m.m[1][0], m.m[1][1]);
    assert!((a * c + b * d).abs() < 1e-3, "no shear: {m:?}");
    assert!((a.hypot(b) - 1.0).abs() < 0.02, "{m:?}");
    // 30° then 2/1: no fold, the matrix shears (B §2.7)
    let mut s = doc_session(600.0, 400.0);
    let id = point_text(&mut s);
    ex(
        &mut s,
        "modify.transform.numericTransform",
        json!({"mode": "rotate", "angle": 30.0}),
    );
    numeric(&mut s, 200.0, 100.0);
    let t = text(&s, id);
    assert_eq!((size(&t), hscale(&t)), (60.0, 1.0));
    let m = t.transform.expect("matrix");
    assert!(
        (m.m[0][0] * m.m[1][0] + m.m[0][1] * m.m[1][1]).abs() > 0.1,
        "sheared: {m:?}"
    );
}

#[test]
fn skewed_text_keeps_the_matrix() {
    if !arial() {
        return;
    }
    let mut s = doc_session(600.0, 400.0);
    let id = point_text(&mut s);
    ex(
        &mut s,
        "transform.matrix",
        json!({"matrix": [1, 0, -0.31, 1, 56, 0]}),
    );
    for (sx, sy) in [(150.0, 150.0), (200.0, 100.0)] {
        let before = text(&s, id).transform.expect("skew");
        numeric(&mut s, sx, sy);
        let t = text(&s, id);
        assert_eq!((size(&t), hscale(&t)), (60.0, 1.0), "{sx}/{sy}");
        assert_ne!(t.transform, Some(before), "matrix × scale");
        ex(&mut s, "edit.undo", json!({}));
    }
    // rotation, reflection and transformSelection never fold
    let mut s = doc_session(600.0, 400.0);
    let id = point_text(&mut s);
    ex(&mut s, "modify.transform.flipHorizontal", json!({}));
    ex(
        &mut s,
        "transform.matrix",
        json!({"matrix": [1.5, 0, 0, 1.5, 0, 0]}),
    );
    let t = text(&s, id);
    assert_eq!(size(&t), 60.0);
    assert!(t.transform.is_some());
}

#[test]
fn scale_tool_folds_a_single_text_only() {
    if !arial() {
        return;
    }
    // the tool's row carries rememberQuad; a corner drag to 1.5 about the top-left corner of the
    // text's transform frame (its set box scaled by 1.3, RE spec 67)
    let mut s = doc_session(600.0, 400.0);
    let id = point_text(&mut s);
    let b0 = visible_bounds(&s, id);
    let f = fp_engine::docops::text_transform_frame(&text(&s, id), None).rect;
    s.set_tool(fp_engine::ToolId::Scale).expect("Scale tool");
    let (x, y) = (f.right, f.bottom);
    let (x1, y1) = (f.left + 1.5 * f.width(), f.top + 1.5 * f.height());
    events(
        &mut s,
        &[
            fp_engine::ToolEvent::down(x, y),
            fp_engine::ToolEvent::drag((x + x1) / 2.0, (y + y1) / 2.0),
            fp_engine::ToolEvent::drag(x1, y1),
            fp_engine::ToolEvent::up(x1, y1),
            fp_engine::ToolEvent::key(fp_engine::ToolKey::Enter),
        ],
    );
    // (the drop point goes through the snapping, hence about 90 pt)
    let t = text(&s, id);
    assert!(
        (size(&t) - 90.0).abs() < 0.1,
        "{} {:?}",
        size(&t),
        t.transform
    );
    let last = s
        .active()
        .expect("document")
        .history
        .undo_label()
        .map(str::to_string);
    let n = undo_count(&s);
    let row = s
        .execute("panel.history.copySteps", json!({"steps": [n - 1]}))
        .expect("copy steps")
        .value;
    let row = row["script"].as_str().unwrap_or_default().to_string();
    assert!(row.contains("\"rememberQuad\":true"), "{row} ({last:?})");
    // right edge only: horizontal scale
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(size(&text(&s, id)), 60.0);
    ex(
        &mut s,
        "transform.matrix",
        json!({"matrix": [1.5, 0, 0, 1, 0, 0], "center": [b0.left, b0.top], "rememberQuad": true}),
    );
    let t = text(&s, id);
    assert_eq!((size(&t), hscale(&t)), (60.0, 1.5));
    // without rememberQuad (transformSelection, setQuadrangle without the option): matrix
    ex(&mut s, "edit.undo", json!({}));
    ex(
        &mut s,
        "transform.matrix",
        json!({"matrix": [1.5, 0, 0, 1.5, 0, 0], "center": [b0.left, b0.top]}),
    );
    let t = text(&s, id);
    assert_eq!(size(&t), 60.0);
    assert!(t.transform.is_some());
}

#[test]
fn group_keeps_the_matrix_under_the_tool_but_folds_numerically() {
    if !arial() {
        return;
    }
    let mut s = doc_session(600.0, 400.0);
    let id = point_text(&mut s);
    let r = rect(&mut s, 100.0, 200.0, 200.0, 260.0);
    select(&mut s, &[id, r]);
    ex(&mut s, "modify.group", json!({}));
    let g = s.active().expect("document").selection.elements.clone();
    assert_eq!(g.len(), 1, "group selected");
    let gb = visible_bounds(&s, g[0]);
    // Scale tool on the group: matrix at the text, 60 pt (B §2.8)
    s.set_tool(fp_engine::ToolId::Scale).expect("Scale tool");
    let (x, y) = (gb.right, gb.bottom);
    events(
        &mut s,
        &[
            fp_engine::ToolEvent::down(x, y),
            fp_engine::ToolEvent::drag(x + 20.0, y + 20.0),
            fp_engine::ToolEvent::drag(x + 40.0, y + 40.0),
            fp_engine::ToolEvent::up(x + 40.0, y + 40.0),
            fp_engine::ToolEvent::key(fp_engine::ToolKey::Enter),
        ],
    );
    let t = text(&s, id);
    assert_eq!(size(&t), 60.0);
    assert!(t.transform.is_some(), "matrix");
    // the same matrix with rememberQuad on a group: still a matrix (setQuadrangle on a group)
    ex(&mut s, "edit.undo", json!({}));
    ex(
        &mut s,
        "transform.matrix",
        json!({"matrix": [1.5, 0, 0, 1.5, 0, 0], "center": [gb.left, gb.top], "rememberQuad": true}),
    );
    assert_eq!(size(&text(&s, id)), 60.0);
    ex(&mut s, "edit.undo", json!({}));
    // numeric 150 %: the text inside the group folds, the rectangle scales
    numeric(&mut s, 150.0, 150.0);
    let t = text(&s, id);
    assert_eq!((size(&t), t.transform), (90.0, None));
    let rb = visible_bounds(&s, r);
    assert!((rb.width() - 150.0).abs() < 2.0, "{rb:?}");
    // 2/1 inside the group: horizontal scale 2
    ex(&mut s, "edit.undo", json!({}));
    numeric(&mut s, 200.0, 100.0);
    let t = text(&s, id);
    assert_eq!((size(&t), hscale(&t)), (60.0, 2.0));
}

#[test]
fn two_texts_fold_each_numerically_but_not_under_remember_quad() {
    if !arial() {
        return;
    }
    let mut s = doc_session(600.0, 400.0);
    let a = point_text(&mut s);
    let b = point_text_at(&mut s, 300.0, 300.0, 40.0);
    select(&mut s, &[a, b]);
    numeric(&mut s, 150.0, 150.0);
    assert_eq!((size(&text(&s, a)), size(&text(&s, b))), (90.0, 60.0));
    ex(&mut s, "edit.undo", json!({}));
    ex(
        &mut s,
        "transform.matrix",
        json!({"matrix": [1.5, 0, 0, 1.5, 0, 0], "rememberQuad": true}),
    );
    assert_eq!((size(&text(&s, a)), size(&text(&s, b))), (60.0, 40.0));
}

#[test]
fn fixed_width_text_keeps_its_width_rewraps_and_squashes() {
    if !arial() {
        return;
    }
    let mut s = doc_session(600.0, 400.0);
    let v = ex(
        &mut s,
        "draw.text",
        json!({"text": "Alpha Beta Gamma Delta Epsilon Zeta", "rect": [100, 100, 260, 160],
               "font": "ArialMT", "size": 20, "color": "#000000", "initFromPrefs": false}),
    );
    let id = ElementId(v["id"].as_u64().unwrap_or(0));
    select(&mut s, &[id]);
    let t0 = text(&s, id);
    assert_eq!(t0.frame_type, Some(1));
    let w0 = t0.anchor_rect.expect("frame").width();
    numeric(&mut s, 150.0, 150.0);
    let t = text(&s, id);
    assert_eq!(size(&t), 30.0);
    assert_eq!(
        t.anchor_rect.expect("frame").width(),
        w0,
        "frame width stays"
    );
    assert!(
        t.bounds.height() > t0.bounds.height() * 1.6,
        "more lines: {:?} → {:?}",
        t0.bounds,
        t.bounds
    );
    // the leftover matrix stretches the narrower set and squashes the taller one (B §2.5:
    // [1.252, 0.611]; the exact target box of the reference is open)
    let m = t.transform.expect("leftover matrix");
    assert!(m.m[0][0] > 1.1 && m.m[1][1] < 0.8, "{m:?}");
    // 1/2: same wrapping, 40 pt / 0.5, no matrix
    ex(&mut s, "edit.undo", json!({}));
    numeric(&mut s, 100.0, 200.0);
    let t = text(&s, id);
    assert_eq!((size(&t), hscale(&t)), (40.0, 0.5));
}

/// RE spec 66 §2.6: the other attributes, through the public fold with the session renderer.
#[test]
fn fold_scales_leading_kerning_indent_and_spacing() {
    if !arial() {
        return;
    }
    let mut s = doc_session(600.0, 400.0);
    let id = point_text(&mut s);
    let r = s.renderer().cloned().expect("renderer");
    let doc = s.active().expect("document").doc.clone();
    let fit = |t: &TextElem| r.text_fit_boxes(&doc, t);
    let base = {
        let mut t = text(&s, id);
        t.initial.size = Some(20.0);
        t.initial.horizontal_scale = Some(0.8);
        t.initial.leading = Some(30.0);
        t.initial.leading_mode = Some(1);
        t.initial.tracking = Some(0.1);
        t.initial.baseline_shift = Some(5.0);
        t.initial.indent = Some(10.0);
        t.initial.space_before = Some(6.0);
        t.initial.space_after = Some(8.0);
        t
    };
    for ((sx, sy), want) in [
        ((1.5, 1.5), [30.0, 0.8, 45.0, 0.1, 5.0, 15.0, 9.0, 12.0]),
        ((2.0, 1.0), [20.0, 1.6, 30.0, 0.2, 5.0, 20.0, 6.0, 8.0]),
        ((1.0, 2.0), [40.0, 0.4, 60.0, 0.05, 5.0, 10.0, 12.0, 16.0]),
    ] {
        let mut t = base.clone();
        assert!(docops_fold(&mut t, Matrix3::scaling(sx, sy), &fit));
        let a = &t.initial;
        let got = [
            a.size,
            a.horizontal_scale,
            a.leading,
            a.tracking,
            a.baseline_shift,
            a.indent,
            a.space_before,
            a.space_after,
        ]
        .map(|v| v.unwrap_or(f32::NAN));
        for (g, w) in got.iter().zip(want) {
            assert!((g - w).abs() < 1e-4, "{sx}/{sy}: {got:?} != {want:?}");
        }
    }
    // percentage leading stays
    let mut t = base.clone();
    t.initial.leading = Some(1.2);
    t.initial.leading_mode = Some(0);
    assert!(docops_fold(&mut t, Matrix3::scaling(1.5, 1.5), &fit));
    assert_eq!(t.initial.leading, Some(1.2));
    // a reflection, a rotation or a pure move does not fold
    for m in [
        Matrix3::scaling(-1.0, 1.0),
        Matrix3::rotation_degrees(30.0),
        Matrix3::translation(10.0, 5.0),
    ] {
        let mut t = base.clone();
        assert!(!docops_fold(&mut t, m, &fit), "{m:?}");
        assert_eq!(t, base);
    }
}

fn docops_fold(t: &mut TextElem, m: Matrix3, fit: fp_engine::docops::TextFitFn) -> bool {
    fp_engine::docops::fold_text_scale(t, &m, fit)
}

/// RE spec 66 §2.11: the folded text is set again with the new size, pixel for pixel the same as a
/// fresh text of that size (0 of 240 000 pixels differ in the reference).
#[test]
fn folded_text_renders_like_a_fresh_text_of_the_new_size() {
    if !arial() {
        return;
    }
    let render = |s: &Session| {
        let doc = &s.active().expect("document").doc;
        let opts = fp_render::RenderOptions {
            cache_policy: fp_render::CachePolicy::None,
            ..fp_render::RenderOptions::default()
        };
        fp_render::Renderer::new(opts).render_page(doc, 0, 0).image
    };
    let mut s = doc_session(600.0, 400.0);
    let id = point_text(&mut s);
    numeric(&mut s, 150.0, 150.0);
    let a = text(&s, id).anchor_rect.expect("anchor");
    let scaled = render(&s);
    let mut f = doc_session(600.0, 400.0);
    point_text_at(&mut f, a.left, a.top, 90.0);
    let fresh = render(&f);
    assert_eq!(scaled.width(), fresh.width());
    let mut diff = 0;
    let mut ink = 0;
    for y in 0..scaled.height() as i32 {
        for x in 0..scaled.width() as i32 {
            let (p, q) = (scaled.pixel(x, y), fresh.pixel(x, y));
            if p != q {
                diff += 1;
            }
            if p.r < 128 {
                ink += 1;
            }
        }
    }
    assert!(ink > 1000, "ink {ink}");
    assert_eq!(diff, 0, "pixels differing");
}
