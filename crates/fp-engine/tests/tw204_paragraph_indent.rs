//! TW-204 / FB-26 (re 60 section 3): the paragraph indent moves the first line of every paragraph
//! by the indent in px, on point text as on fixed-width text; in a wrapped paragraph only the first
//! line moves; the object keeps its position (left/top). Space before/after act only between
//! paragraphs (not above the first, not below the last), leading only from the second line on;
//! on a one-line text none of the three changes the picture or the bounds.

use crate::common::{doc_session, el, ex, select};
use fp_engine::Session;
use fp_engine::tools::ToolId;
use fp_model::{ElementId, ElementKind};
use serde_json::json;

fn setup() -> Session {
    fp_i18n::set_language("de");
    doc_session(600.0, 400.0)
}

fn point_text(s: &mut Session, text: &str) -> ElementId {
    let v = ex(
        s,
        "draw.text",
        json!({"text": text, "anchor": [100, 60], "size": 15, "font": "Arial"}),
    );
    ElementId(v["id"].as_u64().unwrap_or(0))
}

fn frame_text(s: &mut Session, text: &str, width: f64) -> ElementId {
    let v = ex(
        s,
        "draw.text",
        json!({"text": text, "rect": [100, 60, 100.0 + width, 160], "size": 15, "font": "Arial"}),
    );
    ElementId(v["id"].as_u64().unwrap_or(0))
}

/// Laid-out lines of text `id`: (x of the first glyph, baseline, line width).
fn lines(s: &Session, id: ElementId) -> Vec<(f64, f64, f64)> {
    let d = s.active().expect("doc");
    let e = el(s, id);
    let ElementKind::Text(t) = &e.kind else {
        panic!("not a text")
    };
    let block = fp_render::convert::text_block(&d.doc, t);
    let lay = fp_text::layout(
        &block,
        &fp_text::FontDb::shared(),
        &fp_text::LayoutOptions::default(),
    );
    lay.lines
        .iter()
        .map(|l| {
            let x = lay
                .glyphs
                .get(l.glyphs.start)
                .filter(|_| !l.glyphs.is_empty())
                .map(|g| g.x)
                .unwrap_or(l.start);
            (x, l.baseline, l.width)
        })
        .collect()
}

fn bounds(s: &Session, id: ElementId) -> fp_geom::Rect {
    match &el(s, id).kind {
        ElementKind::Text(t) => t.bounds,
        _ => panic!("no text"),
    }
}

fn last_row(s: &Session) -> String {
    let d = s.active().expect("doc");
    d.history
        .steps()
        .last()
        .map(|st| s.step_label(st))
        .unwrap_or_default()
}

fn apply(s: &mut Session, id: ElementId, p: serde_json::Value) {
    s.set_tool(ToolId::Pointer).expect("pointer");
    select(s, &[id]);
    ex(s, "text.applyAttributes", p);
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.01
}

#[test]
fn indent_moves_the_one_line_point_text_and_keeps_its_position() {
    let mut s = setup();
    let id = point_text(&mut s, "Einzeilig");
    let before = lines(&s, id);
    let b0 = bounds(&s, id);
    let n = s.active().expect("doc").history.len();
    apply(&mut s, id, json!({"indent": 33}));
    let after = lines(&s, id);
    assert_eq!(after.len(), 1);
    assert!(near(after[0].0, before[0].0 + 33.0), "{before:?} -> {after:?}");
    assert!(near(after[0].1, before[0].1), "baseline unchanged");
    let b1 = bounds(&s, id);
    assert_eq!((b1.left, b1.top, b1.bottom), (b0.left, b0.top, b0.bottom), "{b0:?} -> {b1:?}");
    assert!(b1.right >= b0.right + 32.0, "the moved ink stays inside the bounds: {b1:?}");
    assert_eq!(s.active().expect("doc").history.len(), n + 1, "one history step");
    assert_eq!(last_row(&s), "Textänderung");
    // the value is stored on the text (the inspector field reads it back)
    let ElementKind::Text(t) = &el(&s, id).kind else {
        panic!("no text")
    };
    assert!(t.resolved_runs().iter().all(|(a, _)| a.indent == Some(33.0)));
    // and again on a relayout (no drift of the anchor)
    apply(&mut s, id, json!({"spaceBefore": 0}));
    assert!(near(lines(&s, id)[0].0, before[0].0 + 33.0), "stable on relayout");
    assert_eq!(bounds(&s, id).left, b0.left);
}

#[test]
fn indent_moves_the_first_line_of_every_paragraph() {
    let mut s = setup();
    for id in [
        point_text(&mut s, "There is something\ror\ranother."),
        frame_text(&mut s, "There is something\ror\ranother.", 200.0),
    ] {
        let before = lines(&s, id);
        assert_eq!(before.len(), 3);
        let b0 = bounds(&s, id);
        apply(&mut s, id, json!({"indent": 33}));
        let after = lines(&s, id);
        assert_eq!(after.len(), 3);
        for (a, b) in after.iter().zip(&before) {
            assert!(near(a.0, b.0 + 33.0), "{before:?} -> {after:?}");
            assert!(near(a.1, b.1), "baselines unchanged");
        }
        let b1 = bounds(&s, id);
        assert_eq!((b1.left, b1.top, b1.bottom), (b0.left, b0.top, b0.bottom));
    }
}

#[test]
fn indent_moves_only_the_first_line_of_a_wrapped_paragraph() {
    let mut s = setup();
    // one paragraph, soft-wrapped into three lines
    let id = frame_text(&mut s, "There is something or another.", 120.0);
    let before = lines(&s, id);
    assert_eq!(before.len(), 3, "{before:?}");
    let b0 = bounds(&s, id);
    apply(&mut s, id, json!({"indent": 33}));
    let after = lines(&s, id);
    assert!(near(after[0].0, before[0].0 + 33.0), "{before:?} -> {after:?}");
    for l in &after[1..] {
        assert!(near(l.0, before[1].0), "following lines stay: {after:?}");
    }
    assert_eq!(bounds(&s, id).left, b0.left);
    assert_eq!(bounds(&s, id).right, b0.right);
}

#[test]
fn spacing_and_leading_on_a_one_line_text_change_nothing_visible() {
    let mut s = setup();
    let id = point_text(&mut s, "Einzeilig");
    let before = lines(&s, id);
    let b0 = bounds(&s, id);
    for p in [
        json!({"leading": 2.0, "leadingMode": 0}),
        json!({"spaceBefore": 20}),
        json!({"spaceAfter": 20}),
    ] {
        apply(&mut s, id, p.clone());
        assert_eq!(lines(&s, id), before, "{p}");
        assert_eq!(bounds(&s, id), b0, "{p}");
    }
}

#[test]
fn spacing_only_between_paragraphs_and_leading_from_the_second_line() {
    let mut s = setup();
    let id = point_text(&mut s, "Absatz eins\rAbsatz zwei");
    let l0 = lines(&s, id);
    let b0 = bounds(&s, id);
    assert_eq!(l0.len(), 2);
    let step0 = l0[1].1 - l0[0].1;
    // leading 200 %: baseline to baseline = 2 x 15
    apply(&mut s, id, json!({"leading": 2.0, "leadingMode": 0}));
    let l1 = lines(&s, id);
    assert!(near(l1[0].1, l0[0].1), "first line stays");
    assert!(near(l1[1].1 - l1[0].1, 30.0), "{l1:?}");
    let b1 = bounds(&s, id);
    assert_eq!(b1.top, b0.top);
    assert!(near((b1.bottom - b0.bottom) as f64, (30.0 - step0).round()), "{b0:?} -> {b1:?}");
    // space before 20: only between the paragraphs, the first stays on top
    apply(&mut s, id, json!({"spaceBefore": 20}));
    let l2 = lines(&s, id);
    assert!(near(l2[0].1, l0[0].1), "nothing above the first paragraph");
    assert!(near(l2[1].1 - l2[0].1, 50.0), "{l2:?}");
    let b2 = bounds(&s, id);
    assert_eq!(b2.top, b0.top);
    assert_eq!(b2.bottom, b1.bottom + 20.0);
    // space after 20: the gap grows again, nothing below the last paragraph
    apply(&mut s, id, json!({"spaceAfter": 20}));
    let l3 = lines(&s, id);
    assert!(near(l3[0].1, l0[0].1));
    assert!(near(l3[1].1 - l3[0].1, 70.0), "{l3:?}");
    let b3 = bounds(&s, id);
    assert_eq!(b3.bottom, b2.bottom + 20.0, "only the gap grows");
}

#[test]
fn horizontal_scale_widens_the_lines() {
    let mut s = setup();
    let id = point_text(&mut s, "Absatz eins\rAbsatz zwei");
    let l0 = lines(&s, id);
    let b0 = bounds(&s, id);
    apply(&mut s, id, json!({"horizontalScale": 2.0}));
    let l1 = lines(&s, id);
    for (a, b) in l1.iter().zip(&l0) {
        assert!(near(a.0, b.0), "lines start at the anchor");
        assert!((a.2 / b.2 - 2.0).abs() < 0.1, "{l0:?} -> {l1:?}");
        assert!(near(a.1, b.1), "baselines unchanged");
    }
    let b1 = bounds(&s, id);
    assert_eq!((b1.left, b1.top, b1.bottom), (b0.left, b0.top, b0.bottom));
    assert!(b1.right > b0.right + 50.0);
}
