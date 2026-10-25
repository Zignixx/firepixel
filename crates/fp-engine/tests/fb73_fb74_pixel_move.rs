//! FB-73 / FB-74 (TW-241): moving the selected pixels of a bitmap. A Pointer / Subselection drag
//! inside the pixel selection and a Ctrl drag of the Marquee show the live preview of the selected
//! pixels only (`Overlay::Ghost` with `pixels`), the release moves the pixels; the arrow keys with
//! a pixel selection move the pixels inside the bitmap (1 px, Shift 10 px), consecutive presses
//! are one History row; without a pixel selection the object nudge stays.

use crate::common;
use common::*;
use fp_engine::tools::{ToolId, ToolKey};
use fp_engine::{Overlay, Session, ToolEvent};
use fp_geom::IRect;
use fp_model::{ElementId, ElementKind};
use serde_json::json;

fn rows(s: &Session) -> usize {
    s.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

fn pixel_bounds(s: &Session) -> Option<IRect> {
    s.active()
        .and_then(|d| d.selection.pixels.as_ref())
        .map(|m| m.tight_bounds())
}

/// The bitmap `id` as an image in document coordinates.
fn image(s: &Session, id: ElementId) -> fp_raster::RgbaImage {
    let d = s.active().expect("document");
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, id).expect("bitmap");
    let ElementKind::Bitmap(b) = &e.kind else {
        panic!("not a bitmap")
    };
    fp_engine::docops::bitmap_image(b).expect("image")
}

fn alpha(s: &Session, id: ElementId, x: i32, y: i32) -> u8 {
    image(s, id).get(x, y).map(|p| p[3]).unwrap_or(0)
}

/// A 400 × 300 bitmap, selected, with the pixel selection 100,100 – 150,150.
fn setup() -> (Session, ElementId) {
    let mut s = doc_session(400.0, 300.0);
    let id = bitmap(&mut s, 0, 0, 400, 300, [200, 40, 40, 255]);
    ex(
        &mut s,
        "select.pixels",
        json!({"shape": "rect", "rect": [100, 100, 150, 150]}),
    );
    assert_eq!(pixel_bounds(&s), Some(IRect::new(100, 100, 150, 150)));
    (s, id)
}

fn pixel_ghost(s: &mut Session) -> Option<(Vec<u64>, fp_geom::Rect, [fp_geom::Point; 4])> {
    s.tool_feedback().into_iter().find_map(|o| match o {
        Overlay::Ghost {
            ids,
            rect,
            quad,
            pixels: true,
            ..
        } => Some((ids, rect, quad)),
        _ => None,
    })
}

/// FB-74: arrow keys with a pixel selection move the selected pixels (not the object), Shift 10
/// px; consecutive presses are one History row; one undo restores pixels and marquee.
#[test]
fn fb74_arrow_keys_move_the_selected_pixels() {
    for t in [ToolId::Pointer, ToolId::Subselection] {
        let (mut s, id) = setup();
        s.set_tool(t).unwrap_or_else(|e| panic!("{e}"));
        let before_rows = rows(&s);
        let bounds = |s: &Session| {
            let d = s.active().expect("d");
            let e = fp_engine::docops::element_on_page(&d.doc, d.page, id).expect("e");
            fp_engine::docops::element_bounds(&d.doc, &e).expect("bounds")
        };
        let b0 = bounds(&s);
        events(
            &mut s,
            &[
                ToolEvent::key(ToolKey::Right),
                ToolEvent::key(ToolKey::Right),
                ToolEvent::key(ToolKey::Down).shift(),
            ],
        );
        assert_eq!(
            pixel_bounds(&s),
            Some(IRect::new(102, 110, 152, 160)),
            "{t:?} marquee moved"
        );
        assert_eq!(bounds(&s), b0, "{t:?} object bounds unchanged");
        // the vacated columns / rows are transparent, the moved pixels opaque
        assert_eq!(alpha(&s, id, 100, 120), 0, "{t:?} hole left");
        assert_eq!(alpha(&s, id, 120, 105), 0, "{t:?} hole top");
        assert_eq!(alpha(&s, id, 151, 159), 255, "{t:?} moved pixels");
        assert_eq!(alpha(&s, id, 20, 20), 255, "{t:?} rest of the bitmap stays");
        assert_eq!(rows(&s), before_rows + 1, "{t:?} one History row");
        ex(&mut s, "edit.undo", json!({}));
        assert_eq!(
            pixel_bounds(&s),
            Some(IRect::new(100, 100, 150, 150)),
            "{t:?} undo"
        );
        assert_eq!(
            alpha(&s, id, 100, 120),
            255,
            "{t:?} undo restores the pixels"
        );
    }
}

/// Without a pixel selection the arrow keys still move the object.
#[test]
fn fb74_arrow_keys_without_pixel_selection_move_the_object() {
    let (mut s, id) = setup();
    ex(&mut s, "select.pixels", json!({"shape": "none"}));
    s.set_tool(ToolId::Pointer)
        .unwrap_or_else(|e| panic!("{e}"));
    events(&mut s, &[ToolEvent::key(ToolKey::Right).shift()]);
    let d = s.active().expect("d");
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, id).expect("e");
    let b = fp_engine::docops::element_bounds(&d.doc, &e).expect("bounds");
    assert_eq!((b.left, b.top), (10.0, 0.0));
}

/// FB-73 (a): a Pointer drag inside the pixel selection previews only the selected pixels (the
/// ghost covers the selection bounds, moved), the release moves the pixels.
#[test]
fn fb73_pointer_drag_previews_the_selected_pixels() {
    let (mut s, id) = setup();
    s.set_tool(ToolId::Pointer)
        .unwrap_or_else(|e| panic!("{e}"));
    events(
        &mut s,
        &[
            ToolEvent::down(120.0, 120.0),
            ToolEvent::drag(130.0, 125.0),
            ToolEvent::drag(140.0, 130.0),
        ],
    );
    let (ids, rect, quad) = pixel_ghost(&mut s).expect("pixel ghost");
    assert_eq!(ids, vec![id.0]);
    assert_eq!(
        (rect.left, rect.top, rect.right, rect.bottom),
        (100.0, 100.0, 150.0, 150.0)
    );
    assert_eq!(
        (quad[0].x, quad[0].y, quad[2].x, quad[2].y),
        (120.0, 110.0, 170.0, 160.0)
    );
    events(&mut s, &[ToolEvent::up(140.0, 130.0)]);
    assert_eq!(pixel_bounds(&s), Some(IRect::new(120, 110, 170, 160)));
    assert_eq!(alpha(&s, id, 105, 105), 0, "hole");
    assert_eq!(alpha(&s, id, 160, 155), 255, "moved pixels");
}

/// FB-73 (b): Marquee + Ctrl drag inside the selection shows the same live preview, the release
/// moves the pixels (one History row); without Ctrl only the marquee moves (dashed outline).
#[test]
fn fb73_marquee_ctrl_drag_previews_and_moves_the_pixels() {
    let (mut s, id) = setup();
    s.set_tool(ToolId::Marquee)
        .unwrap_or_else(|e| panic!("{e}"));
    let before_rows = rows(&s);
    events(
        &mut s,
        &[
            ToolEvent::down(120.0, 120.0).ctrl(),
            ToolEvent::drag(130.0, 125.0).ctrl(),
            ToolEvent::drag(140.0, 130.0).ctrl(),
        ],
    );
    let (ids, _, quad) = pixel_ghost(&mut s).expect("pixel ghost with Ctrl");
    assert_eq!(ids, vec![id.0]);
    assert_eq!((quad[0].x, quad[0].y), (120.0, 110.0));
    events(&mut s, &[ToolEvent::up(140.0, 130.0).ctrl()]);
    assert_eq!(pixel_bounds(&s), Some(IRect::new(120, 110, 170, 160)));
    assert_eq!(alpha(&s, id, 105, 105), 0, "hole");
    assert_eq!(alpha(&s, id, 160, 155), 255, "moved pixels");
    assert_eq!(rows(&s), before_rows + 1);
    // without Ctrl: the marquee alone, no pixel preview
    events(
        &mut s,
        &[ToolEvent::down(130.0, 130.0), ToolEvent::drag(140.0, 130.0)],
    );
    assert!(
        pixel_ghost(&mut s).is_none(),
        "marquee move: no pixel preview"
    );
    events(&mut s, &[ToolEvent::up(140.0, 130.0)]);
    assert_eq!(pixel_bounds(&s), Some(IRect::new(130, 110, 180, 160)));
    assert_eq!(alpha(&s, id, 175, 120), 255, "pixels stay");
}
