//! TW-219 (RE spec 67): the transform frame of exactly one text under the Scale tool, Free
//! Transform, Skew and Distort is its set box scaled by 1.3 about its centre (0.15 × width left and
//! right, 0.15 × height top and bottom), unrounded, mapped through the text's matrix. Point text:
//! the set box is the layout frame (`raw`); fixed-width text: a box around the set content. Paths,
//! groups (also with a text) and several objects keep their bounds. Shown with the corners on whole
//! pixels; corner drags project the absolute pointer onto the diagonal of the unrounded frame.

use crate::common;
use common::*;
use fp_engine::docops::{self, TransformFrame};
use fp_engine::{Overlay, Session, ToolEvent, ToolId, ToolKey};
use fp_geom::{Matrix3, Point, Rect};
use fp_model::{ElementId, ElementKind, TextElem};
use serde_json::json;

fn arial() -> bool {
    let ok = fp_text::FontDb::shared().find_ps("ArialMT").is_some();
    if !ok {
        eprintln!("skipped: ArialMT not installed");
    }
    ok
}

/// Point text in ArialMT whose layout frame starts at `raw` 100,100 like the reference's
/// `addNewText({left:100, top:100})` (the anchor sits 1 px right of the frame for "A"/"x", the
/// first baseline at the top + ceil(ascent)).
fn point_text(s: &mut Session, chars: &str, size: f32) -> ElementId {
    let asc = (0.857_986 * size as f64 - 1e-9).ceil() as f32;
    let v = ex(
        s,
        "draw.text",
        json!({"text": chars, "anchor": [101.0, 100.0 + asc], "size": size, "font": "ArialMT", "color": "#000000"}),
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

fn frame_of(s: &Session, ids: &[ElementId]) -> TransformFrame {
    let d = s.active().expect("document");
    let r = s.renderer().cloned().expect("renderer");
    let fit = |t: &TextElem| r.text_fit_boxes(&d.doc, t);
    docops::transform_frame(&d.doc, d.page, ids, Some(&fit)).expect("frame")
}

fn near(a: f32, b: f64, tol: f64) -> bool {
    (a as f64 - b).abs() <= tol
}

fn assert_rect(r: Rect, want: [f64; 4], tol: f64, what: &str) {
    assert!(
        near(r.left, want[0], tol)
            && near(r.top, want[1], tol)
            && near(r.right, want[2], tol)
            && near(r.bottom, want[3], tol),
        "{what}: {r:?} != {want:?}"
    );
}

fn size(t: &TextElem) -> f32 {
    t.initial.size.unwrap_or(0.0)
}

/// Black handles of the Scale tool as shown (8 handles) and the pivot.
fn shown(s: &mut Session) -> (Vec<Point>, Point, [Point; 4]) {
    let fb = s.tool_feedback();
    let hs = fb
        .iter()
        .find_map(|o| match o {
            Overlay::Handles { points } => Some(points.clone()),
            _ => None,
        })
        .expect("handles");
    let piv = fb
        .iter()
        .find_map(|o| match o {
            Overlay::Pivot { at } => Some(*at),
            _ => None,
        })
        .expect("pivot");
    let quad = fb
        .iter()
        .find_map(|o| match o {
            Overlay::TransformBox { quad, .. } => Some(*quad),
            _ => None,
        })
        .expect("frame");
    (hs, piv, quad)
}

fn drag(s: &mut Session, from: (f32, f32), to: (f32, f32)) {
    let mid = ((from.0 + to.0) / 2.0, (from.1 + to.1) / 2.0);
    events(
        s,
        &[
            ToolEvent::down(from.0, from.1),
            ToolEvent::drag(mid.0, mid.1),
            ToolEvent::drag(to.0, to.1),
            ToolEvent::up(to.0, to.1),
            ToolEvent::key(ToolKey::Enter),
        ],
    );
}

#[test]
fn point_text_frame_is_the_raw_rect_scaled_by_1_3() {
    if !arial() {
        return;
    }
    // B §2.1: "Ag" 60 / 30 / 12 pt at raw 100,100
    for (sz, raw, want) in [
        (
            60.0,
            [100.0, 100.0, 174.0, 172.0],
            [88.9, 89.2, 185.1, 182.8],
        ),
        (
            30.0,
            [100.0, 100.0, 138.0, 136.0],
            [94.3, 94.6, 143.7, 141.4],
        ),
        (
            12.0,
            [100.0, 100.0, 116.0, 115.0],
            [97.6, 97.75, 118.4, 117.25],
        ),
    ] {
        let mut s = doc_session(600.0, 400.0);
        let id = point_text(&mut s, "Ag", sz);
        assert_rect(text(&s, id).bounds, raw, 0.0, &format!("raw {sz} pt"));
        let f = frame_of(&s, &[id]);
        assert!(f.text && f.matrix.is_none(), "{f:?}");
        assert_rect(f.rect, want, 1e-3, &format!("frame {sz} pt"));
    }
}

#[test]
fn point_text_three_lines_and_horizontal_scale() {
    if !arial() {
        return;
    }
    // B §2.3: three lines "Ag/xyz/AgAg" 30 pt, leading 100 % → raw 75 × 96, frame
    // 88.75–186.25 × 85.6–210.4
    let mut s = doc_session(600.0, 400.0);
    let id = point_text(&mut s, "Ag\rxyz\rAgAg", 30.0);
    let r = s.renderer().cloned().expect("renderer");
    let doc = s.active().expect("document").doc.clone();
    let mut t = text(&s, id);
    t.initial.leading = Some(1.0);
    t.initial.leading_mode = Some(0);
    for run in &mut t.runs {
        run.attrs.leading = Some(1.0);
        run.attrs.leading_mode = Some(0);
    }
    t.bounds = r.text_frame(&doc, &t).expect("layout");
    assert_rect(
        t.bounds,
        [100.0, 100.0, 175.0, 196.0],
        0.0,
        "raw three lines",
    );
    let fit = |t: &TextElem| r.text_fit_boxes(&doc, t);
    let f = docops::text_transform_frame(&t, Some(&fit));
    assert_rect(f.rect, [88.75, 85.6, 186.25, 210.4], 1e-3, "three lines");
    // horizontal scale 1.5 only widens the set box (raw 111 × 72): 0.15 × 111 left and right
    let mut s = doc_session(600.0, 400.0);
    let id = point_text(&mut s, "Ag", 60.0);
    ex(
        &mut s,
        "modify.transform.numericTransform",
        json!({"mode": "scale", "scaleX": 150.0, "scaleY": 100.0, "constrain": false}),
    );
    let t = text(&s, id);
    assert_eq!(t.initial.horizontal_scale, Some(1.5));
    let b = t.bounds;
    assert_eq!((b.width(), b.height()), (111.0, 72.0));
    let f = frame_of(&s, &[id]);
    let w = [
        b.left as f64 - 16.65,
        b.top as f64 - 10.8,
        b.right as f64 + 16.65,
        b.bottom as f64 + 10.8,
    ];
    assert_rect(f.rect, w, 1e-3, "horizontal scale 1.5");
}

#[test]
fn turned_text_gets_a_turned_frame() {
    if !arial() {
        return;
    }
    // B §2.5: PT turned by 30° about 137,136 (matrix [0.866, 0.5, −0.5, 0.866, 86.355, −50.279])
    let mut s = doc_session(600.0, 400.0);
    let id = point_text(&mut s, "Ag", 60.0);
    let m = docops::about(Point::new(137.0, 136.0), &Matrix3::rotation_degrees(30.0));
    ex(&mut s, "transform.matrix", json!({"matrix": m.to_affine()}));
    let t = text(&s, id);
    let tm = t.transform.expect("matrix").to_affine();
    assert!(
        (tm[4] - 86.355).abs() < 0.01 && (tm[5] + 50.279).abs() < 0.01,
        "{tm:?}"
    );
    let f = frame_of(&s, &[id]);
    assert!(f.matrix.is_some(), "{f:?}");
    let q = f.quad();
    for (p, w) in q.iter().zip([
        (118.74, 71.42),
        (202.05, 119.52),
        (155.25, 200.57),
        (71.94, 152.47),
    ]) {
        assert!(near(p.x, w.0, 0.02) && near(p.y, w.1, 0.02), "{q:?}");
    }
    // shown: rounded corners, middle handles from the rounded corners
    s.set_tool(ToolId::Scale).expect("Scale tool");
    let (hs, _, quad) = shown(&mut s);
    let want = [
        (119.0, 71.0),
        (160.5, 95.5),
        (202.0, 120.0),
        (178.5, 160.5),
        (155.0, 201.0),
        (113.5, 176.5),
        (72.0, 152.0),
        (95.5, 111.5),
    ];
    let got: Vec<(f32, f32)> = hs.iter().map(|p| (p.x, p.y)).collect();
    assert_eq!(got, want.to_vec());
    assert_eq!(quad[0], Point::new(119.0, 71.0));
    // a corner drag on the turned frame scales uniformly about the opposite corner and folds
    let (o, h) = (q[0], q[2]);
    let to = (o.x + 1.25 * (h.x - o.x), o.y + 1.25 * (h.y - o.y));
    drag(&mut s, (h.x, h.y), to);
    let t = text(&s, id);
    assert!(
        (size(&t) - 75.0).abs() <= 0.011,
        "{} {:?}",
        size(&t),
        t.transform
    );
    let tm = t.transform.expect("turned").to_affine();
    assert!(
        (tm[1] as f64 / tm[0] as f64 - 30f64.to_radians().tan()).abs() < 1e-3,
        "{tm:?}"
    );
}

#[test]
fn fixed_width_text_frame_is_the_set_box_scaled() {
    if !arial() {
        return;
    }
    // B §2.4 (ArialMT, left-aligned, leading 100 %, raw left/top 100,100): set box = frame / 1.3
    let cases: [(&str, f32, f32, [f64; 4]); 7] = [
        (
            "Alpha Beta Gamma Delta Epsilon Zeta",
            20.0,
            160.0,
            [99.0, 97.0, 230.0, 161.0],
        ),
        ("Ag", 60.0, 160.0, [99.0, 91.0, 173.0, 163.0]),
        ("Ag\rxyz\rAgAg", 30.0, 200.0, [99.0, 95.0, 174.0, 192.0]),
        ("Ag", 20.0, 160.0, [99.0, 97.0, 124.0, 121.0]),
        ("Ag\rAg", 20.0, 160.0, [99.0, 97.0, 124.0, 141.0]),
        ("Gamma Delta", 20.0, 200.0, [100.0, 97.0, 224.0, 121.0]),
        ("Alpha Beta", 20.0, 200.0, [99.0, 97.0, 197.0, 121.0]),
    ];
    let mut s = doc_session(600.0, 400.0);
    let r = s.renderer().cloned().expect("renderer");
    for (chars, sz, width, want) in cases {
        let v = ex(
            &mut s,
            "draw.text",
            json!({"text": chars, "rect": [100.0, 100.0, 100.0 + width, 140.0], "size": sz, "font": "ArialMT", "color": "#000000"}),
        );
        let id = ElementId(v["id"].as_u64().unwrap_or(0));
        let doc = s.active().expect("document").doc.clone();
        let mut t = text(&s, id);
        assert_eq!(t.frame_type, Some(1));
        t.initial.leading = Some(1.0);
        t.initial.leading_mode = Some(0);
        for run in &mut t.runs {
            run.attrs.leading = Some(1.0);
            run.attrs.leading_mode = Some(0);
        }
        let fit = |t: &TextElem| r.text_fit_boxes(&doc, t);
        let b = docops::text_transform_box(&t, Some(&fit));
        assert_eq!(b, want, "set box of {chars:?} {sz} pt");
        let f = docops::text_transform_frame(&t, Some(&fit));
        let (w, h) = (want[2] - want[0], want[3] - want[1]);
        let exp = [
            want[0] - 0.15 * w,
            want[1] - 0.15 * h,
            want[2] + 0.15 * w,
            want[3] + 0.15 * h,
        ];
        assert_rect(f.rect, exp, 1e-3, chars);
    }
}

#[test]
fn paths_groups_and_several_objects_keep_their_bounds() {
    if !arial() {
        return;
    }
    // B §2.6: rectangle = its bounds; text + rectangle (selected or grouped) = united bounds
    let mut s = doc_session(600.0, 400.0);
    let r = rect(&mut s, 100.0, 100.0, 200.0, 160.0);
    select(&mut s, &[r]);
    let f = frame_of(&s, &[r]);
    assert!(!f.text && f.matrix.is_none());
    assert_rect(f.rect, [100.0, 100.0, 200.0, 160.0], 0.0, "rectangle");
    s.set_tool(ToolId::Scale).expect("Scale tool");
    let (hs, piv, quad) = shown(&mut s);
    assert_eq!(quad, Rect::new(100.0, 100.0, 200.0, 160.0).corners());
    assert_eq!(
        (hs[1], hs[3], piv),
        (
            Point::new(150.0, 100.0),
            Point::new(200.0, 130.0),
            Point::new(150.0, 130.0)
        )
    );
    let mut s = doc_session(600.0, 400.0);
    let t = point_text(&mut s, "Ag", 60.0);
    let r = rect(&mut s, 100.0, 200.0, 200.0, 260.0);
    select(&mut s, &[t, r]);
    assert_rect(
        frame_of(&s, &[t, r]).rect,
        [98.0, 98.0, 200.0, 260.0],
        0.0,
        "multi",
    );
    ex(&mut s, "modify.group", json!({}));
    let g = s.active().expect("document").selection.elements.clone();
    let f = frame_of(&s, &g);
    assert!(!f.text);
    assert_rect(f.rect, [98.0, 98.0, 200.0, 260.0], 0.0, "group with a text");
}

#[test]
fn shown_handles_lie_on_the_rounded_frame() {
    if !arial() {
        return;
    }
    // B §2.10 / §3: PT 60 pt → handles 89/137/185 × 89/136/183, pivot 137/136; the Skew and
    // Distort tools show the same frame
    for tool in [ToolId::Scale, ToolId::Skew, ToolId::Distort] {
        let mut s = doc_session(600.0, 400.0);
        point_text(&mut s, "Ag", 60.0);
        s.set_tool(tool).expect("tool");
        let (hs, piv, quad) = shown(&mut s);
        let got: Vec<(f32, f32)> = hs.iter().map(|p| (p.x, p.y)).collect();
        assert_eq!(
            got,
            vec![
                (89.0, 89.0),
                (137.0, 89.0),
                (185.0, 89.0),
                (185.0, 136.0),
                (185.0, 183.0),
                (137.0, 183.0),
                (89.0, 183.0),
                (89.0, 136.0),
            ],
            "{tool:?}"
        );
        assert_eq!(piv, Point::new(137.0, 136.0));
        assert_eq!(quad, Rect::new(89.0, 89.0, 185.0, 183.0).corners());
    }
}

#[test]
fn corner_drag_projects_onto_the_unrounded_frame() {
    if !arial() {
        return;
    }
    // run 40 / B §2.8: the handle grabbed at 184,182 and dragged +39/+38 → 1.39566 → 83.74 pt,
    // leftover matrix [0.993, 0, 0, 1.005], t 5.085/23.883 (RE spec 66 §3.3)
    let mut s = doc_session(600.0, 400.0);
    let id = point_text(&mut s, "Ag", 60.0);
    s.set_tool(ToolId::Scale).expect("Scale tool");
    drag(&mut s, (184.0, 182.0), (223.0, 220.0));
    let t = text(&s, id);
    assert_eq!(size(&t), 83.74, "{:?}", t.transform);
    let m = t.transform.expect("leftover matrix").to_affine();
    assert!(
        (m[0] - 0.993).abs() < 5e-4 && (m[3] - 1.005).abs() < 5e-4,
        "{m:?}"
    );
    assert_eq!((m[1], m[2]), (0.0, 0.0));
    assert!(
        (m[4] - 5.085).abs() < 0.02 && (m[5] - 23.883).abs() < 0.02,
        "{m:?}"
    );
    let last = s.active().expect("document").history.undo_count();
    let row = s
        .execute("panel.history.copySteps", json!({"steps": [last - 1]}))
        .expect("copy steps")
        .value;
    assert!(
        row["script"]
            .as_str()
            .unwrap_or_default()
            .contains("\"rememberQuad\":true"),
        "{row}"
    );
    // the frame of the new state lands exactly on the dragged target (B §2.9)
    let f = frame_of(&s, &[id]);
    assert_rect(
        f.rect,
        [88.9, 89.2, 223.163, 219.834],
        0.01,
        "frame after the drag",
    );
    s.set_tool(ToolId::Scale).expect("Scale tool");
    let (hs, _, _) = shown(&mut s);
    assert_eq!(
        (hs[0], hs[1], hs[4]),
        (
            Point::new(89.0, 89.0),
            Point::new(156.0, 89.0),
            Point::new(223.0, 220.0)
        )
    );
    // second drag: grabbed at 222,219, +39/+38 → 1.28314 → 107.45 pt
    drag(&mut s, (222.0, 219.0), (261.0, 257.0));
    assert_eq!(size(&text(&s, id)), 107.45);
    // grabbed 2 px beside the handle: the absolute pointer counts → 1.41673 → 85 pt
    let mut s = doc_session(600.0, 400.0);
    let id = point_text(&mut s, "Ag", 60.0);
    s.set_tool(ToolId::Scale).expect("Scale tool");
    drag(&mut s, (186.0, 184.0), (225.0, 222.0));
    assert_eq!(size(&text(&s, id)), 85.0);
}

#[test]
fn edge_drag_scales_one_axis_of_the_frame() {
    if !arial() {
        return;
    }
    // B §2.8: right edge 184,135 → 204,155: f_x = (204 − 88.9) / 96.2 = 1.1965 → horizontal
    // scale 1.196, the size stays
    let mut s = doc_session(600.0, 400.0);
    let id = point_text(&mut s, "Ag", 60.0);
    s.set_tool(ToolId::Scale).expect("Scale tool");
    drag(&mut s, (184.0, 135.0), (204.0, 155.0));
    let t = text(&s, id);
    assert_eq!((size(&t), t.initial.horizontal_scale), (60.0, Some(1.196)));
    // RE spec 66: right edge +39 → hs 1.394; bottom edge +38 → f_y = (220 − 89.2) / 93.6 =
    // 1.3974 → 83.85 pt
    let mut s = doc_session(600.0, 400.0);
    let id = point_text(&mut s, "Ag", 60.0);
    s.set_tool(ToolId::Scale).expect("Scale tool");
    drag(&mut s, (184.0, 135.0), (223.0, 135.0));
    let t = text(&s, id);
    assert_eq!((size(&t), t.initial.horizontal_scale), (60.0, Some(1.394)));
    let mut s = doc_session(600.0, 400.0);
    let id = point_text(&mut s, "Ag", 60.0);
    s.set_tool(ToolId::Scale).expect("Scale tool");
    drag(&mut s, (137.0, 182.0), (137.0, 220.0));
    assert_eq!(size(&text(&s, id)), 83.85);
}

#[test]
fn remove_transformations_puts_the_text_on_the_outline_of_its_frame() {
    if !arial() {
        return;
    }
    // derived (A, RE spec 67 §2.12): the new left/top = the rounded left/top of the axis-aligned
    // outline of the transform frame; PT turned 30° → 72,71; PT after the tool (83.74 pt) → 89,89
    let mut s = doc_session(600.0, 400.0);
    let id = point_text(&mut s, "Ag", 60.0);
    let m = docops::about(Point::new(137.0, 136.0), &Matrix3::rotation_degrees(30.0));
    ex(&mut s, "transform.matrix", json!({"matrix": m.to_affine()}));
    ex(&mut s, "modify.transform.removeTransformations", json!({}));
    let b = visible_bounds(&s, id);
    assert_eq!(
        (b.left, b.top, text(&s, id).transform),
        (72.0, 71.0, None),
        "{b:?}"
    );
    let mut s = doc_session(600.0, 400.0);
    let id = point_text(&mut s, "Ag", 60.0);
    s.set_tool(ToolId::Scale).expect("Scale tool");
    drag(&mut s, (184.0, 182.0), (223.0, 220.0));
    ex(&mut s, "modify.transform.removeTransformations", json!({}));
    let b = visible_bounds(&s, id);
    assert_eq!((b.left, b.top), (89.0, 89.0), "{b:?}");
    assert_eq!(size(&text(&s, id)), 83.74);
}
