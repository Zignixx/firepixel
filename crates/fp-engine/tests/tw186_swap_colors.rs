//! TW-186 (re 58 §1, measured): Swap Colours (toolbox button and X, `color.swap`). Without a
//! selection only the toolbox colours swap, no History row. With a selection the swapped toolbox
//! colours (taken from the topmost selected object) go to every selected element in ONE row
//! "Stroke Color" ("Strichfarbe"), or "Brush and Fill" ("Pinselstrich und Füllen") when one side is
//! none; an existing stroke keeps tip, width and placement, a new stroke gets the 1-pixel
//! anti-aliased tip; a text loses its fill; a bitmap gets the row without a pixel change. Ctrl+Y
//! repeats the swap, Ctrl+Z restores objects and toolbox colours.
//!
//! TW-04 (re 58 §6, measured): Escape after a marquee on a selected bitmap exits bitmap mode
//! ("Exit Bitmap Mode"), clears the pixel selection, switches to the Pointer and keeps the bitmap
//! selected; a second Escape changes nothing.

use crate::common;
use common::*;
use fp_color::Argb;
use fp_engine::tools::ToolId;
use fp_engine::{Session, ToolEvent, ToolKey};
use fp_model::ElementId;
use serde_json::json;

const RED: Argb = Argb(0xffff_0000);
const BLUE: Argb = Argb(0xff00_00ff);
const GREEN: Argb = Argb(0xff00_ff00);
const YELLOW: Argb = Argb(0xffff_ff00);

fn rows(s: &Session) -> usize {
    s.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

fn last_label(s: &Session) -> String {
    let d = s.active().expect("doc");
    d.history.top().map(|st| s.step_label(st)).unwrap_or_default()
}

fn last_command(s: &Session) -> String {
    let d = s.active().expect("doc");
    d.history.top().map(|st| st.command.clone()).unwrap_or_default()
}

fn tool_colors(s: &Session) -> (Argb, Argb) {
    (s.tools.stroke_color, s.tools.fill_color)
}

/// (fill colour or None, stroke colour or None, brush name, diameter, placement)
fn attrs(s: &Session, id: ElementId) -> (Option<Argb>, Option<Argb>, String, i32, i32) {
    let e = el(s, id);
    let a = fp_engine::docops::path_attrs_of(&e).expect("path attrs");
    let placement = match &e.kind {
        fp_model::ElementKind::Path(p) => p.stroke_placement,
        fp_model::ElementKind::Text(t) => t.stroke_placement,
        _ => -1,
    };
    (
        a.fill.as_ref().map(|_| a.fill_color),
        a.brush.as_ref().map(|_| a.stroke_color),
        a.brush.as_ref().map(|b| b.name.clone()).unwrap_or_default(),
        a.brush.as_ref().map(|b| b.diameter).unwrap_or(0),
        placement,
    )
}

/// Rectangle with fill `fill` and a 1-pixel-hard stroke `stroke` of `width` px, centred.
fn styled_rect(s: &mut Session, r: [f32; 4], fill: Option<&str>, stroke: Option<(&str, f32)>) -> ElementId {
    let id = rect(s, r[0], r[1], r[2], r[3]);
    select(s, &[id]);
    match fill {
        Some(c) => ex(s, "element.setFill", json!({"color": c})),
        None => ex(s, "element.setFill", json!({"kind": "none"})),
    };
    match stroke {
        Some((c, w)) => ex(s, "element.setStroke", json!({"color": c, "width": w, "placement": "center"})),
        None => ex(s, "element.setStroke", json!({"width": 0})),
    };
    id
}

fn swap(s: &mut Session) {
    ex(s, "color.swap", json!({}));
}

/// Nothing selected: only the toolbox colours swap, no row.
#[test]
fn without_selection_only_the_toolbox_swaps() {
    fp_i18n::set_language("de");
    let mut s = doc_session(600.0, 400.0);
    let r = styled_rect(&mut s, [100.0, 100.0, 300.0, 200.0], Some("#ff0000"), Some(("#0000ff", 2.0)));
    select(&mut s, &[]);
    s.set_colors(Some(BLUE), Some(RED));
    let n = rows(&s);
    let before = attrs(&s, r);
    swap(&mut s);
    assert_eq!(tool_colors(&s), (RED, BLUE), "toolbox swapped");
    assert_eq!(rows(&s), n, "no History row");
    assert_eq!(attrs(&s, r), before, "object unchanged");
}

/// One rectangle (fill red, stroke blue 2 px centre): fill blue, stroke red 2 px centre; toolbox
/// swapped; one row "Strichfarbe" `color.swap`; Ctrl+Y repeats (back), Ctrl+Z restores object and
/// toolbox.
///
/// The toolbox shows other colours (green / yellow) while nothing is selected; selecting the
/// rectangle makes it follow (Q-58), so the swap starts from the rectangle's colours as in the
/// reference. Undo, redo and History jumps give the toolbox the colours of the restored
/// selection, which are the colours stored with the swap rows.
#[test]
fn rectangle_swaps_with_one_row_repeat_and_undo() {
    fp_i18n::set_language("de");
    let mut s = doc_session(600.0, 400.0);
    let r = styled_rect(&mut s, [100.0, 100.0, 300.0, 200.0], Some("#ff0000"), Some(("#0000ff", 2.0)));
    select(&mut s, &[]);
    s.set_colors(Some(GREEN), Some(YELLOW));
    select(&mut s, &[r]);
    assert_eq!(tool_colors(&s), (BLUE, RED), "the toolbox follows the selected rectangle");
    let n = rows(&s);
    let (_, _, tip, _, _) = attrs(&s, r);
    swap(&mut s);
    assert_eq!(attrs(&s, r), (Some(BLUE), Some(RED), tip.clone(), 2, 1));
    assert_eq!(tool_colors(&s), (RED, BLUE), "toolbox: stroke red, fill blue");
    assert_eq!(rows(&s), n + 1, "one row");
    assert_eq!(last_label(&s), "Strichfarbe");
    assert_eq!(last_command(&s), "color.swap");
    fp_i18n::set_language("en");
    assert_eq!(last_label(&s), "Stroke Color");
    fp_i18n::set_language("de");
    // X again: back, a second row
    swap(&mut s);
    assert_eq!(attrs(&s, r), (Some(RED), Some(BLUE), tip.clone(), 2, 1));
    assert_eq!(tool_colors(&s), (BLUE, RED));
    assert_eq!(rows(&s), n + 2);
    // Ctrl+Y repeats the swap as a new row
    assert_eq!(s.repeat_label().as_deref(), Some("Strichfarbe"));
    ex(&mut s, "edit.redo", json!({}));
    assert_eq!(attrs(&s, r), (Some(BLUE), Some(RED), tip.clone(), 2, 1));
    assert_eq!(tool_colors(&s), (RED, BLUE));
    assert_eq!(rows(&s), n + 3);
    assert_eq!(last_label(&s), "Strichfarbe");
    // Ctrl+Z restores the object and the toolbox colours, written once (no flicker between the
    // stored colours and the followed object)
    s.drain_events();
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(attrs(&s, r), (Some(RED), Some(BLUE), tip.clone(), 2, 1));
    assert_eq!(tool_colors(&s), (BLUE, RED));
    let writes = s
        .drain_events()
        .iter()
        .filter(|e| matches!(e, fp_engine::SessionEvent::ColorsChanged))
        .count();
    assert_eq!(writes, 1, "the toolbox is written once");
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(tool_colors(&s), (RED, BLUE));
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(attrs(&s, r), (Some(RED), Some(BLUE), tip.clone(), 2, 1));
    assert_eq!(tool_colors(&s), (BLUE, RED), "toolbox as before the first swap");
    assert_eq!(rows(&s), n);
    // redo brings the swap back, a History jump restores the toolbox of the target position
    ex(&mut s, "edit.redo", json!({}));
    assert_eq!(attrs(&s, r), (Some(BLUE), Some(RED), tip.clone(), 2, 1));
    assert_eq!(tool_colors(&s), (RED, BLUE));
    ex(&mut s, "history.goTo", json!({"position": n + 3}));
    assert_eq!(attrs(&s, r), (Some(BLUE), Some(RED), tip.clone(), 2, 1));
    assert_eq!(tool_colors(&s), (RED, BLUE));
    ex(&mut s, "history.goTo", json!({"position": n + 2}));
    assert_eq!(tool_colors(&s), (BLUE, RED));
    ex(&mut s, "history.goTo", json!({"position": n}));
    assert_eq!(attrs(&s, r), (Some(RED), Some(BLUE), tip.clone(), 2, 1));
    assert_eq!(tool_colors(&s), (BLUE, RED));
}

/// Nothing selected: a toolbox-only swap writes no row, so undoing, redoing or jumping over the
/// earlier object rows leaves the swapped toolbox alone as long as those rows end with nothing
/// selected.
#[test]
fn toolbox_only_swap_survives_undo_of_object_rows() {
    fp_i18n::set_language("de");
    let mut s = doc_session(600.0, 400.0);
    let n0 = rows(&s);
    let r = rect(&mut s, 100.0, 100.0, 300.0, 200.0);
    ex(&mut s, "edit.clear", json!({}));
    assert!(s.active().expect("doc").selection.elements.is_empty(), "nothing selected");
    let n = rows(&s);
    assert!(n >= n0 + 2, "drawing and deleting are rows");
    s.set_colors(Some(GREEN), Some(YELLOW));
    swap(&mut s);
    assert_eq!(tool_colors(&s), (YELLOW, GREEN));
    assert_eq!(rows(&s), n, "no row");
    // the delete comes back (the rectangle selected again): the toolbox follows the rectangle
    let drawn = {
        let mut t = doc_session(600.0, 400.0);
        rect(&mut t, 100.0, 100.0, 300.0, 200.0);
        tool_colors(&t)
    };
    ex(&mut s, "edit.undo", json!({}));
    assert!(s.active().expect("doc").selection.elements.contains(&r));
    assert_eq!(tool_colors(&s), drawn, "follows the restored selection");
    // deselect, swap the toolbox only, then a History jump to the empty document (nothing
    // selected, no stored colours on the crossed rows): the toolbox stays
    ex(&mut s, "select.none", json!({}));
    s.set_colors(Some(GREEN), Some(YELLOW));
    let m = rows(&s);
    swap(&mut s);
    assert_eq!(rows(&s), m, "no row");
    ex(&mut s, "history.goTo", json!({"position": n0}));
    assert!(s.active().expect("doc").selection.elements.is_empty());
    assert_eq!(tool_colors(&s), (YELLOW, GREEN), "the toolbox stays");
}

/// A swap row on a selected bitmap (no tool attributes to follow) and nothing selected after the
/// jump: the colours stored with the swap row come back.
#[test]
fn stored_colours_come_back_without_an_object_to_follow() {
    fp_i18n::set_language("de");
    let mut s = doc_session(600.0, 400.0);
    let b = bitmap(&mut s, 120, 80, 100, 80, [200, 0, 0, 255]);
    select(&mut s, &[b]);
    s.set_colors(Some(GREEN), Some(YELLOW));
    let n = rows(&s);
    swap(&mut s);
    swap(&mut s);
    swap(&mut s);
    assert_eq!(tool_colors(&s), (YELLOW, GREEN));
    assert_eq!(rows(&s), n + 3);
    ex(&mut s, "history.goTo", json!({"position": n + 1}));
    assert_eq!(tool_colors(&s), (YELLOW, GREEN));
    ex(&mut s, "history.goTo", json!({"position": n}));
    assert_eq!(tool_colors(&s), (GREEN, YELLOW));
    ex(&mut s, "history.goTo", json!({"position": n + 2}));
    assert_eq!(tool_colors(&s), (GREEN, YELLOW));
    ex(&mut s, "edit.redo", json!({}));
    assert_eq!(tool_colors(&s), (YELLOW, GREEN));
}

/// Two objects: both get the toolbox colours of the topmost one, widths stay.
#[test]
fn two_objects_get_the_colours_of_the_topmost() {
    fp_i18n::set_language("de");
    let mut s = doc_session(600.0, 400.0);
    let low = styled_rect(&mut s, [100.0, 100.0, 300.0, 200.0], Some("#ff0000"), Some(("#0000ff", 2.0)));
    let top = styled_rect(&mut s, [350.0, 100.0, 500.0, 200.0], Some("#00ff00"), Some(("#ffff00", 4.0)));
    select(&mut s, &[low, top]);
    let n = rows(&s);
    swap(&mut s);
    let (lf, ls, _, lw, _) = attrs(&s, low);
    let (tf, ts, _, tw, _) = attrs(&s, top);
    assert_eq!((tf, ts, tw), (Some(YELLOW), Some(GREEN), 4));
    assert_eq!((lf, ls, lw), (Some(YELLOW), Some(GREEN), 2));
    assert_eq!(tool_colors(&s), (GREEN, YELLOW));
    assert_eq!(rows(&s), n + 1, "one row");
    assert_eq!(last_label(&s), "Strichfarbe");
}

/// Stroke none: fill becomes none, the stroke the old fill colour with the 1-pixel anti-aliased
/// tip, 1 px; row "Pinselstrich und Füllen". Fill none: the other way round.
#[test]
fn none_swaps_too_with_brush_and_fill_row() {
    fp_i18n::set_language("de");
    let mut s = doc_session(600.0, 400.0);
    let r = styled_rect(&mut s, [100.0, 100.0, 300.0, 200.0], Some("#ff0000"), None);
    select(&mut s, &[r]);
    let n = rows(&s);
    swap(&mut s);
    let (f, st, tip, w, _) = attrs(&s, r);
    assert_eq!((f, st, tip.as_str(), w), (None, Some(RED), "bn_1-Pixel Anti-Aliased", 1));
    let a = fp_engine::docops::path_attrs_of(&el(&s, r)).expect("attrs");
    assert!(a.brush.as_ref().is_some_and(|b| b.antialiased), "anti-aliased tip");
    assert_eq!(tool_colors(&s).0, RED);
    // toolbox fill none (RE spec 59: the tool follows the path's fill none with its stored colour)
    assert!(s.tool_style().fill_is_none(), "toolbox fill none");
    assert_eq!(s.tool_well_colors().1.a(), 0, "the fill well shows none");
    assert_eq!(rows(&s), n + 1);
    assert_eq!(last_label(&s), "Pinselstrich und Füllen");
    fp_i18n::set_language("en");
    assert_eq!(last_label(&s), "Brush and Fill");
    fp_i18n::set_language("de");

    let f2 = styled_rect(&mut s, [350.0, 100.0, 500.0, 200.0], None, Some(("#0000ff", 2.0)));
    select(&mut s, &[f2]);
    swap(&mut s);
    let (f, st, _, _, _) = attrs(&s, f2);
    assert_eq!((f, st), (Some(BLUE), None), "fill blue (solid), stroke none");
    assert_eq!(last_label(&s), "Pinselstrich und Füllen");
}

/// Text (fill #00aa00, no stroke): fill none, stroke = the old text colour, 1 px.
#[test]
fn text_loses_its_fill_and_gets_a_stroke() {
    fp_i18n::set_language("de");
    let mut s = doc_session(600.0, 400.0);
    let v = ex(&mut s, "draw.text", json!({"text": "Qrs", "anchor": [100, 150], "size": 40, "font": "Arial", "color": "#00aa00"}));
    let t = ElementId(v["id"].as_u64().expect("id"));
    select(&mut s, &[t]);
    let (f0, st0, _, _, pl0) = attrs(&s, t);
    assert_eq!((f0, st0), (Some(Argb(0xff00_aa00)), None), "text colour is the fill");
    let n = rows(&s);
    swap(&mut s);
    let (f, st, tip, w, pl) = attrs(&s, t);
    assert_eq!((f, st, tip.as_str(), w), (None, Some(Argb(0xff00_aa00)), "bn_1-Pixel Anti-Aliased", 1));
    assert_eq!(pl, pl0, "placement of the text stays");
    assert_eq!(rows(&s), n + 1);
    assert_eq!(last_label(&s), "Pinselstrich und Füllen");
}

/// Bitmap: the toolbox swaps, a row "Strichfarbe" is written, the pixels stay.
#[test]
fn bitmap_gets_a_row_without_pixel_change() {
    fp_i18n::set_language("de");
    let mut s = doc_session(600.0, 400.0);
    let b = bitmap(&mut s, 120, 80, 100, 80, [200, 0, 0, 255]);
    select(&mut s, &[b]);
    s.set_colors(Some(Argb(0xff00_aa00)), Some(Argb(0xff00_0000)));
    let doc_before = std::sync::Arc::clone(&s.active().expect("doc").doc);
    let n = rows(&s);
    swap(&mut s);
    assert_eq!(tool_colors(&s), (Argb(0xff00_0000), Argb(0xff00_aa00)));
    assert_eq!(rows(&s), n + 1, "a row");
    assert_eq!(last_label(&s), "Strichfarbe");
    assert!(std::sync::Arc::ptr_eq(&doc_before, &s.active().expect("doc").doc), "bitmap unchanged");
    assert_eq!(s.active().expect("doc").selection.elements, vec![b]);
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(tool_colors(&s), (Argb(0xff00_aa00), Argb(0xff00_0000)), "undo restores the toolbox");
}

/// TW-04: Marquee on a selected bitmap, Escape → "Exit Bitmap Mode" row, no pixel selection,
/// Pointer, the bitmap stays selected; a second Escape changes nothing. Lasso the same.
#[test]
fn marquee_escape_keeps_the_bitmap_selected() {
    fp_i18n::set_language("de");
    for t in [ToolId::Marquee, ToolId::Lasso] {
        let mut s = doc_session(600.0, 400.0);
        let b = bitmap(&mut s, 118, 78, 244, 204, [255, 0, 0, 255]);
        select(&mut s, &[b]);
        s.set_tool(t).expect("tool");
        if t == ToolId::Marquee {
            events(&mut s, &[
                ToolEvent::down(143.0, 107.0),
                ToolEvent::drag(223.0, 172.0),
                ToolEvent::drag(303.0, 237.0),
                ToolEvent::up(303.0, 237.0),
            ]);
        } else {
            events(&mut s, &[
                ToolEvent::down(143.0, 107.0),
                ToolEvent::drag(303.0, 107.0),
                ToolEvent::drag(303.0, 237.0),
                ToolEvent::drag(143.0, 237.0),
                ToolEvent::up(143.0, 237.0),
            ]);
        }
        let d = s.active().expect("doc");
        assert!(d.selection.has_pixels(), "{t:?}: pixel selection");
        let n = rows(&s);
        s.tool_event(&ToolEvent::key(ToolKey::Escape)).expect("Esc 1");
        let d = s.active().expect("doc");
        assert!(!d.selection.has_pixels() && !d.bitmap_mode, "{t:?}: pixel selection and mode gone");
        assert_eq!(d.selection.elements, vec![b], "{t:?}: bitmap stays selected");
        assert_eq!(s.tools.active, ToolId::Pointer, "{t:?}: Pointer");
        assert_eq!(rows(&s), n + 1, "{t:?}: one row");
        assert_eq!(last_command(&s), "modify.exitBitmapMode");
        assert_eq!(last_label(&s), "Bitmapmodus beenden");
        // Escape 2: nothing
        let doc = std::sync::Arc::clone(&s.active().expect("doc").doc);
        s.tool_event(&ToolEvent::key(ToolKey::Escape)).expect("Esc 2");
        let d = s.active().expect("doc");
        assert_eq!(rows(&s), n + 1, "{t:?}: no further row");
        assert_eq!(d.selection.elements, vec![b]);
        assert!(std::sync::Arc::ptr_eq(&doc, &d.doc));
        assert_eq!(s.tools.active, ToolId::Pointer);
    }
}
