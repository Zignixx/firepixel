//! TW-134: cropping the document with the crop tool ("delete objects when cropping", on by
//! default): objects entirely outside the new canvas are deleted on all layers, bitmaps crossing
//! the edge are clipped to the canvas, vector objects crossing the edge stay unchanged (only moved
//! with the canvas origin).

use crate::common::{bitmap, bounds, doc_session, ellipse, ex, rect, rect_approx, top_elements};
use fp_geom::Rect;
use fp_model::ElementId;
use serde_json::json;

fn ids(s: &fp_engine::Session) -> Vec<ElementId> {
    top_elements(s).iter().map(|e| e.id).collect()
}

fn shifted(r: Rect, dx: f32, dy: f32) -> Rect {
    Rect::new(r.left - dx, r.top - dy, r.right - dx, r.bottom - dy)
}

#[test]
fn crop_tool_deletes_outside_clips_bitmaps_keeps_crossing_vectors() {
    let mut s = doc_session(800.0, 600.0);
    // layer 1
    let inside = rect(&mut s, 50.0, 50.0, 150.0, 120.0);
    let crossing = rect(&mut s, 200.0, 170.0, 421.0, 241.0);
    let outside = rect(&mut s, 50.0, 300.0, 301.0, 361.0);
    // layer 2
    ex(&mut s, "layer.add", json!({}));
    let outside2 = ellipse(&mut s, 465.0, 165.0, 535.0, 245.0);
    let bmp_cross = bitmap(&mut s, 300, 100, 100, 50, [255, 0, 0, 255]);
    let bmp_out = bitmap(&mut s, 600, 400, 20, 20, [0, 0, 255, 255]);
    let before_inside = bounds(&s, inside);
    let before_crossing = bounds(&s, crossing);

    ex(
        &mut s,
        "edit.cropDocument",
        json!({"rect": [40, 40, 320, 260], "tool": true}),
    );

    let d = s.active().expect("doc");
    let c = &d.doc.pages[d.page].canvas;
    assert_eq!((c.width, c.height), (280.0, 220.0), "new canvas");
    let left = ids(&s);
    for gone in [outside, outside2, bmp_out] {
        assert!(!left.contains(&gone), "entirely outside → deleted on every layer");
    }
    for kept in [inside, crossing, bmp_cross] {
        assert!(left.contains(&kept), "touching the canvas → kept");
    }
    assert!(rect_approx(
        bounds(&s, inside),
        shifted(before_inside, 40.0, 40.0)
    ));
    assert!(
        rect_approx(bounds(&s, crossing), shifted(before_crossing, 40.0, 40.0)),
        "vector crossing the edge stays unclipped: {:?}",
        bounds(&s, crossing)
    );
    assert!(
        rect_approx(bounds(&s, bmp_cross), Rect::new(260.0, 60.0, 280.0, 110.0)),
        "bitmap clipped to the canvas: {:?}",
        bounds(&s, bmp_cross)
    );
}
