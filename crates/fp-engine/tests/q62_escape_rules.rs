//! Q-62 (RE spec 62, measured in the reference with real keys): Escape in transform mode, in the
//! middle of a brush stroke, while dragging a marquee or lasso, with the magic wand, in the text
//! editor and while drawing a rectangle; arrow keys with a pixel selection.

use crate::common;
use common::*;
use fp_engine::tools::ToolId;
use fp_engine::{Overlay, Session, ToolEvent, ToolKey};
use fp_geom::Rect;
use serde_json::json;
use std::sync::Arc;

fn rows(s: &Session) -> usize {
    s.active().expect("doc").history.undo_count()
}

fn redo_rows(s: &Session) -> usize {
    s.active().expect("doc").history.redo_count()
}

fn labels(s: &Session) -> Vec<String> {
    s.active().map(|d| d.history.labels()).unwrap_or_default()
}

fn last_command(s: &Session) -> String {
    let d = s.active().expect("doc");
    d.history.top().map(|st| st.command.clone()).unwrap_or_default()
}

fn esc(s: &mut Session) {
    s.tool_event(&ToolEvent::key(ToolKey::Escape)).expect("Escape");
}

fn px(r: Rect) -> (f32, f32, f32, f32) {
    (r.left.round(), r.top.round(), r.right.round(), r.bottom.round())
}

/// Rectangle R 50,50–150,120 selected, transform mode (Scale tool or Free Transform).
fn transform_setup(free: bool) -> (Session, fp_model::ElementId) {
    let mut s = doc_session(600.0, 400.0);
    let r = rect(&mut s, 50.0, 50.0, 150.0, 120.0);
    select(&mut s, &[r]);
    if free {
        ex(&mut s, "modify.transform.freeTransform", json!({}));
    } else {
        s.set_tool(ToolId::Scale).expect("Scale");
    }
    assert_eq!(s.tools.active, ToolId::Scale);
    (s, r)
}

/// Drags a handle in small steps (press, moves, release).
fn pull(s: &mut Session, from: (f32, f32), to: (f32, f32), release: bool) {
    let mut seq = vec![ToolEvent::drag(from.0, from.1), ToolEvent::down(from.0, from.1)];
    for i in 1..=10 {
        let f = i as f32 / 10.0;
        seq.push(ToolEvent::drag(from.0 + (to.0 - from.0) * f, from.1 + (to.1 - from.1) * f));
    }
    if release {
        seq.push(ToolEvent::up(to.0, to.1));
    }
    events(s, &seq);
}

/// (b) re 62 §1.2: Escape after a drag in transform mode undoes the session: the object is back,
/// the Pointer is active, the row "Transform" stays as a redo step and Ctrl+Y brings it back.
#[test]
fn q62_b_escape_undoes_transform_session_as_redo_step() {
    for lang in ["de", "en"] {
        fp_i18n::set_language(lang);
        for free in [true, false] {
            // reference result of the same drag with Enter
            let (mut s, r) = transform_setup(free);
            pull(&mut s, (151.0, 121.0), (191.0, 151.0), true);
            s.tool_event(&ToolEvent::key(ToolKey::Enter)).expect("Enter");
            let applied = px(bounds(&s, r));
            assert_ne!(applied, (50.0, 50.0, 150.0, 120.0));
            // Escape after one drag
            let (mut s, r) = transform_setup(free);
            let n = rows(&s);
            pull(&mut s, (151.0, 121.0), (191.0, 151.0), true);
            esc(&mut s);
            assert_eq!(px(bounds(&s, r)), (50.0, 50.0, 150.0, 120.0), "{lang} {free}: object back");
            assert_eq!(s.tools.active, ToolId::Pointer, "{lang} {free}: Pointer");
            assert_eq!((rows(&s), redo_rows(&s)), (n, 1), "{lang} {free}: one redo step");
            let want = if lang == "de" { "Transformieren" } else { "Transform" };
            let d = s.active().expect("doc");
            assert_eq!(d.history.redo_label(), Some(want), "{lang} {free}: redo row name");
            assert_eq!(d.selection.elements, vec![r], "selection stays");
            assert_eq!(s.undo_redo_labels().1.as_deref(), Some(want), "Edit menu: Redo: Transform");
            // Ctrl+Y (edit.redo) applies the transformation again
            ex(&mut s, "edit.redo", json!({}));
            assert_eq!(px(bounds(&s, r)), applied, "{lang} {free}: redo");
            assert_eq!((rows(&s), redo_rows(&s)), (n + 1, 0));
        }
    }
    fp_i18n::set_language("de");
}

/// (b) two drags of one session are one row; Escape returns to the start state.
#[test]
fn q62_b_two_drags_one_row_escape_restores_start() {
    let (mut s, r) = transform_setup(true);
    let before = Arc::clone(&s.active().expect("doc").doc);
    let n = rows(&s);
    pull(&mut s, (151.0, 121.0), (191.0, 151.0), true);
    let q = s
        .tool_feedback()
        .iter()
        .find_map(|o| match o {
            Overlay::TransformBox { quad, .. } => Some(quad[0]),
            _ => None,
        })
        .expect("transform box");
    pull(&mut s, (q.x, q.y), (q.x - 20.0, q.y - 20.0), true);
    esc(&mut s);
    assert_eq!(px(bounds(&s, r)), (50.0, 50.0, 150.0, 120.0));
    assert_eq!(*s.active().expect("doc").doc, *before, "document as before the session");
    assert_eq!((rows(&s), redo_rows(&s)), (n, 1), "one row for both drags");
    ex(&mut s, "edit.redo", json!({}));
    let b = px(bounds(&s, r));
    assert!(b.0 < 50.0 && b.1 < 50.0 && b.2 > 150.0 && b.3 > 120.0, "both drags redone: {b:?}");
}

/// (b) re 62 §1.2 b9: Escape with the mouse held keeps the preview until the release, then
/// nothing is applied, the mode ends (Pointer) and there is no row; a second Escape does nothing.
#[test]
fn q62_b_escape_while_dragging_ends_mode_after_release() {
    let (mut s, r) = transform_setup(false);
    let before = Arc::clone(&s.active().expect("doc").doc);
    let n = rows(&s);
    pull(&mut s, (151.0, 121.0), (191.0, 151.0), false);
    let ghost = |s: &mut Session| {
        s.tool_feedback().iter().find_map(|o| match o {
            Overlay::Ghost { quad, .. } => Rect::from_points(*quad).map(px),
            _ => None,
        })
    };
    let g0 = ghost(&mut s).expect("preview while dragging");
    esc(&mut s);
    assert_eq!(s.tools.active, ToolId::Scale, "mode stays while the mouse is held");
    assert_eq!(ghost(&mut s), Some(g0), "preview stays");
    events(&mut s, &[ToolEvent::drag(220.0, 180.0)]);
    assert_eq!(ghost(&mut s), Some(g0), "preview does not follow any more");
    events(&mut s, &[ToolEvent::up(220.0, 180.0)]);
    assert_eq!(s.tools.active, ToolId::Pointer, "mode ends on release");
    assert_eq!(px(bounds(&s, r)), (50.0, 50.0, 150.0, 120.0));
    assert!(Arc::ptr_eq(&s.active().expect("doc").doc, &before), "nothing applied");
    assert_eq!((rows(&s), redo_rows(&s)), (n, 0), "no row");
    esc(&mut s);
    assert_eq!((rows(&s), redo_rows(&s), s.tools.active), (n, 0, ToolId::Pointer));
    assert_eq!(s.active().expect("doc").selection.elements, vec![r]);
}

/// (b) Enter applies (row active), Escape afterwards does nothing; Escape without a drag leaves
/// no row.
#[test]
fn q62_b_enter_applies_escape_after_does_nothing() {
    let (mut s, r) = transform_setup(true);
    let n = rows(&s);
    pull(&mut s, (151.0, 121.0), (191.0, 151.0), true);
    s.tool_event(&ToolEvent::key(ToolKey::Enter)).expect("Enter");
    let applied = px(bounds(&s, r));
    assert_eq!((rows(&s), s.tools.active), (n + 1, ToolId::Pointer));
    esc(&mut s);
    assert_eq!((rows(&s), redo_rows(&s), px(bounds(&s, r))), (n + 1, 0, applied));
    // Escape without a drag: mode ends, no row, nothing to redo
    let (mut s, _) = transform_setup(false);
    let n = rows(&s);
    esc(&mut s);
    assert_eq!((rows(&s), redo_rows(&s), s.tools.active), (n, 0, ToolId::Pointer));
}

/// White bitmap 120,80 240 × 200 selected, Bitmap Brush 10 px black.
fn brush_setup() -> (Session, fp_model::ElementId) {
    let mut s = doc_session(600.0, 400.0);
    s.set_colors(Some(fp_color::Argb(0xff00_0000)), None);
    let b = bitmap(&mut s, 120, 80, 240, 200, [255, 255, 255, 255]);
    select(&mut s, &[b]);
    s.set_tool(ToolId::Brush).expect("Brush");
    s.tools.options.brush_size = 10.0;
    s.tools.options.new_stroke = Some(json!({"preset": "Einfach/Präzise abger.", "width": 10, "softness": 0}));
    s.tools.options.new_brush = None;
    (s, b)
}

fn bitmap_px(s: &Session, id: fp_model::ElementId, x: i32, y: i32) -> [u8; 4] {
    match &el(s, id).kind {
        fp_model::ElementKind::Bitmap(b) => fp_engine::docops::bitmap_image(b)
            .and_then(|i| i.get(x, y))
            .unwrap_or([0; 4]),
        _ => [0; 4],
    }
}

fn paint_bounds(s: &mut Session) -> Option<Rect> {
    s.tool_feedback().into_iter().find_map(|o| match o {
        Overlay::Paint(p) => Some(p.bounds),
        _ => None,
    })
}

/// (d) re 62 §1.4: Escape in the middle of a brush stroke switches to the Pointer at once and
/// freezes the preview; the release still paints the whole stroke as one row; bitmap mode stays
/// until the next Escape ("Exit Bitmap Mode", bitmap still selected).
#[test]
fn q62_d_brush_escape_mid_stroke() {
    let (mut s, b) = brush_setup();
    let n = rows(&s);
    events(&mut s, &[ToolEvent::down(140.0, 150.0)]);
    for i in 1..=20 {
        events(&mut s, &[ToolEvent::drag(140.0 + 4.0 * i as f32, 150.0)]);
    }
    esc(&mut s);
    assert_eq!(s.tools.active, ToolId::Pointer, "Pointer at once");
    let frozen = paint_bounds(&mut s).expect("preview stays visible");
    for i in 21..=50 {
        events(&mut s, &[ToolEvent::drag(140.0 + 4.0 * i as f32, 150.0)]);
    }
    assert_eq!(paint_bounds(&mut s), Some(frozen), "preview frozen");
    assert!(frozen.right < 240.0, "preview stopped at the Escape: {frozen:?}");
    assert_eq!(rows(&s), n, "no row while the mouse is held");
    events(&mut s, &[ToolEvent::up(340.0, 150.0)]);
    assert_eq!(rows(&s), n + 1, "one row");
    assert_eq!(last_command(&s), "bitmap.paint");
    assert_eq!(bitmap_px(&s, b, 330, 150)[0], 0, "whole stroke painted");
    assert_eq!(bitmap_px(&s, b, 350, 150), [255, 255, 255, 255], "not beyond the end");
    assert_eq!(s.tools.active, ToolId::Pointer);
    let d = s.active().expect("doc");
    assert!(d.bitmap_mode, "bitmap mode stays on");
    assert_eq!(d.selection.elements, vec![b]);
    assert!(paint_bounds(&mut s).is_none(), "no preview after the release");
    // next Escape: "Exit Bitmap Mode", bitmap stays selected
    esc(&mut s);
    assert_eq!(rows(&s), n + 2);
    assert_eq!(last_command(&s), "modify.exitBitmapMode");
    let d = s.active().expect("doc");
    assert!(!d.bitmap_mode);
    assert_eq!((d.selection.elements.clone(), s.tools.active), (vec![b], ToolId::Pointer));
    // third Escape: nothing
    esc(&mut s);
    assert_eq!(rows(&s), n + 2);
}

/// A white bitmap 118,78 244 × 204 selected.
fn bitmap_setup(t: ToolId) -> (Session, fp_model::ElementId) {
    let mut s = doc_session(600.0, 400.0);
    let b = bitmap(&mut s, 118, 78, 244, 204, [255, 0, 0, 255]);
    select(&mut s, &[b]);
    s.set_tool(t).expect("tool");
    (s, b)
}

/// Marquee or lasso gesture from 143,107 to 303,237 (without the release).
fn select_gesture(s: &mut Session, t: ToolId, dx: f32) {
    if t == ToolId::Marquee {
        events(s, &[ToolEvent::down(143.0 + dx, 107.0), ToolEvent::drag(223.0 + dx, 172.0), ToolEvent::drag(303.0 + dx, 237.0)]);
    } else {
        events(s, &[
            ToolEvent::down(143.0 + dx, 107.0),
            ToolEvent::drag(303.0 + dx, 107.0),
            ToolEvent::drag(303.0 + dx, 237.0),
            ToolEvent::drag(143.0 + dx, 237.0),
        ]);
    }
}

/// (e) re 62 §1.5: Escape while dragging the first marquee/lasso of a selected bitmap: drag
/// dropped, Pointer at once, two rows "Exit Bitmap Mode", bitmap stays selected, release does
/// nothing.
#[test]
fn q62_e_escape_while_dragging_first_marquee() {
    for lang in ["de", "en"] {
        fp_i18n::set_language(lang);
        for t in [ToolId::Marquee, ToolId::Lasso] {
            let (mut s, b) = bitmap_setup(t);
            let n = rows(&s);
            select_gesture(&mut s, t, 0.0);
            esc(&mut s);
            assert_eq!(s.tools.active, ToolId::Pointer, "{t:?}: Pointer at once");
            let want = if lang == "de" { "Bitmapmodus beenden" } else { "Exit Bitmap Mode" };
            assert_eq!(labels(&s)[n..], [want, want], "{lang} {t:?}: two rows");
            let doc = Arc::clone(&s.active().expect("doc").doc);
            events(&mut s, &[ToolEvent::drag(330.0, 250.0), ToolEvent::up(330.0, 250.0)]);
            let d = s.active().expect("doc");
            assert!(!d.selection.has_pixels() && !d.bitmap_mode, "{t:?}: no pixel selection, mode ended");
            assert_eq!(d.selection.elements, vec![b], "{t:?}: bitmap stays selected");
            assert!(Arc::ptr_eq(&doc, &d.doc), "{t:?}: release does nothing");
            assert_eq!((rows(&s), s.tools.active), (n + 2, ToolId::Pointer));
            assert!(s.tool_feedback().iter().all(|o| !matches!(o, Overlay::Rect { dashed: true, .. } | Overlay::Polyline { .. })), "{t:?}: drag gone");
        }
    }
    fp_i18n::set_language("de");
}

/// (e) with an existing pixel selection: Escape while dragging a new one ends bitmap mode, the old
/// selection is gone too, one row; Escape without a drag is unchanged (one row, Pointer, bitmap
/// selected).
#[test]
fn q62_e_escape_while_dragging_with_existing_selection() {
    for t in [ToolId::Marquee, ToolId::Lasso] {
        let (mut s, b) = bitmap_setup(t);
        select_gesture(&mut s, t, 0.0);
        events(&mut s, &[ToolEvent::up(if t == ToolId::Marquee { 303.0 } else { 143.0 }, 237.0)]);
        assert!(s.active().expect("doc").selection.has_pixels(), "{t:?}: selection");
        let n = rows(&s);
        select_gesture(&mut s, t, 20.0);
        esc(&mut s);
        assert_eq!(s.tools.active, ToolId::Pointer, "{t:?}");
        events(&mut s, &[ToolEvent::up(320.0, 240.0)]);
        let d = s.active().expect("doc");
        assert!(!d.selection.has_pixels() && !d.bitmap_mode, "{t:?}: old selection gone too");
        assert_eq!(d.selection.elements, vec![b]);
        assert_eq!(rows(&s), n + 1, "{t:?}: one row");
        assert_eq!(last_command(&s), "modify.exitBitmapMode");
        // without a drag: unchanged rule
        let (mut s, b) = bitmap_setup(t);
        select_gesture(&mut s, t, 0.0);
        events(&mut s, &[ToolEvent::up(if t == ToolId::Marquee { 303.0 } else { 143.0 }, 237.0)]);
        let n = rows(&s);
        esc(&mut s);
        let d = s.active().expect("doc");
        assert!(!d.selection.has_pixels() && !d.bitmap_mode);
        assert_eq!((d.selection.elements.clone(), s.tools.active, rows(&s)), (vec![b], ToolId::Pointer, n + 1));
    }
}

/// (f) re 62 §1.6 (keep): magic wand, Escape → "Exit Bitmap Mode", pixel selection gone, Pointer,
/// bitmap stays selected; a second Escape does nothing.
#[test]
fn q62_f_magic_wand_escape_pinned() {
    let (mut s, b) = bitmap_setup(ToolId::MagicWand);
    events(&mut s, &[ToolEvent::down(163.0, 107.0), ToolEvent::up(163.0, 107.0)]);
    assert!(s.active().expect("doc").selection.has_pixels());
    let n = rows(&s);
    esc(&mut s);
    let d = s.active().expect("doc");
    assert!(!d.selection.has_pixels() && !d.bitmap_mode);
    assert_eq!((d.selection.elements.clone(), s.tools.active, rows(&s)), (vec![b], ToolId::Pointer, n + 1));
    assert_eq!(last_command(&s), "modify.exitBitmapMode");
    let doc = Arc::clone(&s.active().expect("doc").doc);
    esc(&mut s);
    let d = s.active().expect("doc");
    assert!(Arc::ptr_eq(&doc, &d.doc));
    assert_eq!((d.selection.elements.clone(), s.tools.active, rows(&s)), (vec![b], ToolId::Pointer, n + 1));
}

/// (h) re 62 §1.8: arrows with Marquee/Lasso and a pixel selection move only the marquee (1 px,
/// Shift 10 px); consecutive presses are ONE row "Move", one undo restores the marquee; tool and
/// bitmap mode stay.
#[test]
fn q62_h_marquee_arrow_nudges_merge_into_one_row() {
    for lang in ["de", "en"] {
        fp_i18n::set_language(lang);
        for t in [ToolId::Marquee, ToolId::Lasso] {
            let (mut s, b) = bitmap_setup(t);
            select_gesture(&mut s, t, 0.0);
            events(&mut s, &[ToolEvent::up(if t == ToolId::Marquee { 303.0 } else { 143.0 }, 237.0)]);
            let mask0 = s.active().expect("doc").selection.pixels.as_ref().map(|m| m.bounds()).expect("mask");
            let doc = Arc::clone(&s.active().expect("doc").doc);
            let n = rows(&s);
            for e in [ToolEvent::key(ToolKey::Right), ToolEvent::key(ToolKey::Right), ToolEvent::key(ToolKey::Down).shift()] {
                s.tool_event(&e).expect("arrow");
            }
            let d = s.active().expect("doc");
            let mask = d.selection.pixels.as_ref().map(|m| m.bounds()).expect("mask");
            assert_eq!((mask.left - mask0.left, mask.top - mask0.top), (2, 10), "{t:?}: marquee +2/+10");
            assert!(Arc::ptr_eq(&doc, &d.doc), "{t:?}: pixels stay");
            let want = if lang == "de" { "Verschieben" } else { "Move" };
            assert_eq!(labels(&s)[n..], [want], "{lang} {t:?}: one row");
            assert_eq!(d.selection.elements, vec![b]);
            // a marquee keeps Firepixel in bitmap mode through its pixel selection
            assert!(d.selection.has_pixels(), "{t:?}: bitmap mode stays");
            assert_eq!(s.tools.active, t, "{t:?}: tool stays");
            s.undo().expect("undo");
            let d = s.active().expect("doc");
            assert_eq!(d.selection.pixels.as_ref().map(|m| m.bounds()), Some(mask0), "{t:?}: one undo restores the marquee");
            assert_eq!(rows(&s), n);
        }
    }
    fp_i18n::set_language("de");
}

/// (c) re 62 §1.3 (keep): Escape while drawing a rectangle is ignored, the release makes the
/// rectangle (one row); Escape afterwards changes nothing.
#[test]
fn q62_c_rectangle_ignores_escape_pinned() {
    let mut s = doc_session(600.0, 400.0);
    s.set_tool(ToolId::Rectangle).expect("Rectangle");
    let n = rows(&s);
    events(&mut s, &[ToolEvent::down(300.0, 117.0), ToolEvent::drag(450.0, 237.0), ToolEvent::key(ToolKey::Escape), ToolEvent::drag(473.0, 257.0), ToolEvent::up(473.0, 257.0)]);
    assert_eq!(rows(&s), n + 1, "one row");
    let r = top_elements(&s)[0].id;
    assert_eq!(px(bounds(&s, r)), (300.0, 117.0, 473.0, 257.0));
    assert_eq!((s.tools.active, s.active().expect("doc").selection.elements.clone()), (ToolId::Rectangle, vec![r]));
    let doc = Arc::clone(&s.active().expect("doc").doc);
    esc(&mut s);
    let d = s.active().expect("doc");
    assert!(Arc::ptr_eq(&doc, &d.doc));
    assert_eq!((rows(&s), s.tools.active, d.selection.elements.clone()), (n + 1, ToolId::Rectangle, vec![r]));
}

/// (g) re 62 §1.7 (keep): Escape in the text editor keeps the text and closes the editor; the tool
/// stays Text, also when the editor was opened by a Pointer double-click; a second Escape does
/// nothing.
#[test]
fn q62_g_text_escape_keeps_text_tool_pinned() {
    let mut s = doc_session(600.0, 400.0);
    s.set_tool(ToolId::Text).expect("Text");
    let n = rows(&s);
    events(&mut s, &[ToolEvent::down(100.0, 120.0), ToolEvent::up(100.0, 120.0), ToolEvent::text("Hallo")]);
    esc(&mut s);
    assert!(!s.tool_is_editing_text());
    assert_eq!((rows(&s), s.tools.active), (n + 1, ToolId::Text));
    let t = top_elements(&s)[0].id;
    assert_eq!(s.active().expect("doc").selection.elements, vec![t]);
    // Pointer double-click opens the editor (tool Text), Escape keeps the tool
    s.set_tool(ToolId::Pointer).expect("Pointer");
    let b = bounds(&s, t);
    let (x, y) = (b.left + 5.0, (b.top + b.bottom) / 2.0);
    events(&mut s, &[ToolEvent::down(x, y), ToolEvent::up(x, y), ToolEvent::down(x, y), ToolEvent::up(x, y), ToolEvent::double(x, y)]);
    assert!(s.tool_is_editing_text(), "editor open");
    s.tool_event(&ToolEvent::key(ToolKey::End)).expect("End");
    s.tool_event(&ToolEvent::text(" Welt")).expect("text");
    esc(&mut s);
    assert!(!s.tool_is_editing_text());
    assert_eq!(s.tools.active, ToolId::Text, "tool stays Text");
    assert_eq!(rows(&s), n + 2, "one row for the edit");
    assert!(matches!(&el(&s, t).kind, fp_model::ElementKind::Text(x) if x.plain_text() == "Hallo Welt"));
    esc(&mut s);
    assert_eq!((rows(&s), s.tools.active), (n + 2, ToolId::Text));
}
