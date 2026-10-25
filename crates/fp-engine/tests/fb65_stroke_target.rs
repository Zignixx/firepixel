//! FB-65: the bitmap a Brush, Pencil or retouch stroke works on. A Brush/Pencil stroke that starts
//! on a bitmap paints into it, selected or not (also with a vector selected); one that starts on
//! empty space paints into the selected bitmap (near it: the bitmap grows) or creates a new bitmap.
//! Retouch tools: the selected bitmap, else the bitmap under the press point (selected afterwards),
//! else nothing; Red Eye on empty space inserts an empty bitmap. Bitmaps on locked or hidden
//! layers are never painted into.

use crate::common::*;
use fp_engine::tools::ToolId;
use fp_engine::{Session, ToolEvent, docops};
use fp_model::{ElementId, ElementKind, LayerId};
use serde_json::json;

fn image(s: &Session, id: ElementId) -> fp_raster::RgbaImage {
    match &el(s, id).kind {
        ElementKind::Bitmap(b) => docops::bitmap_image(b).expect("image"),
        _ => panic!("not a bitmap"),
    }
}

fn count(s: &Session) -> usize {
    let d = s.active().expect("doc");
    docops::own_layers(&d.doc, d.page)
        .iter()
        .map(|(_, l)| l.cels.iter().map(|c| c.elements.len()).sum::<usize>())
        .sum()
}

fn sel(s: &Session) -> Vec<ElementId> {
    s.active()
        .map(|d| d.selection.elements.clone())
        .unwrap_or_default()
}

/// Two blue bitmaps A (50,50 100×100) and B (300,50 100×100) on layer 1, nothing selected.
fn mk() -> (Session, ElementId, ElementId) {
    fp_i18n::set_language("en");
    let mut s = doc_session(600.0, 400.0);
    let a = bitmap(&mut s, 50, 50, 100, 100, [0, 0, 255, 255]);
    let b = bitmap(&mut s, 300, 50, 100, 100, [0, 0, 255, 255]);
    ex(&mut s, "select.none", json!({}));
    (s, a, b)
}

/// A pencil stroke from `from` to the right, pressed at `from` (as the tool sends it).
fn stroke(s: &mut Session, from: (f32, f32)) -> serde_json::Value {
    let br = serde_json::to_value(fp_model::Brush::basic(4)).expect("brush");
    ex(
        s,
        "bitmap.paint",
        json!({"tool": "pencil", "brush": br, "color": "#C04040", "trimEnd": false,
               "points": [[from.0, from.1], [from.0 + 30.0, from.1]], "at": [from.0, from.1]}),
    )
}

fn red(img: &fp_raster::RgbaImage, x: i32, y: i32) -> bool {
    img.get(x, y).is_some_and(|p| p[0] == 0xC0 && p[2] == 0x40)
}

#[test]
fn a_stroke_on_an_unselected_bitmap_paints_into_it() {
    let (mut s, a, b) = mk();
    let n = count(&s);
    let v = stroke(&mut s, (320.0, 100.0));
    assert_eq!(count(&s), n, "no new bitmap");
    assert_eq!(v["target"].as_u64(), Some(b.0));
    assert!(red(&image(&s, b), 330, 100));
    assert_eq!(sel(&s), vec![b], "the bitmap painted into is selected");
    assert!(!red(&image(&s, a), 70, 100));
}

#[test]
fn a_stroke_on_a_bitmap_wins_over_the_selected_bitmap_and_a_selected_vector() {
    let (mut s, a, b) = mk();
    select(&mut s, &[a]);
    let n = count(&s);
    stroke(&mut s, (320.0, 100.0));
    assert_eq!(count(&s), n);
    assert!(red(&image(&s, b), 330, 100));
    assert!(
        !image(&s, a).data().iter().any(|p| p[0] == 0xC0),
        "A unchanged"
    );
    assert_eq!(sel(&s), vec![b]);
    // a vector selected
    let (mut s, _, b) = mk();
    let r = rect(&mut s, 450.0, 250.0, 550.0, 350.0);
    select(&mut s, &[r]);
    let n = count(&s);
    stroke(&mut s, (320.0, 100.0));
    assert_eq!(count(&s), n);
    assert!(red(&image(&s, b), 330, 100));
}

#[test]
fn a_stroke_with_the_brush_tool_on_an_unselected_bitmap_paints_into_it() {
    let (mut s, _, b) = mk();
    s.set_tool(ToolId::Pencil).expect("pencil");
    let n = count(&s);
    events(
        &mut s,
        &[
            ToolEvent::down(320.0, 100.0),
            ToolEvent::drag(350.0, 100.0),
            ToolEvent::up(350.0, 100.0),
        ],
    );
    assert_eq!(count(&s), n, "no new bitmap");
    assert_eq!(sel(&s), vec![b]);
    assert!(
        image(&s, b)
            .get(335, 100)
            .is_some_and(|p| p[2] != 255 || p[0] != 0),
        "B painted"
    );
}

#[test]
fn a_bitmap_on_a_locked_or_hidden_layer_is_skipped() {
    let (mut s, _, b) = mk();
    let l1 = s.active().and_then(|d| d.layer).expect("layer");
    let l2 = LayerId(
        ex(&mut s, "layer.add", json!({"name": "L2"}))["layer"]
            .as_u64()
            .unwrap_or(0),
    );
    ex(
        &mut s,
        "layer.setLocked",
        json!({"layer": l1.0, "locked": true}),
    );
    let before = image(&s, b);
    let n = count(&s);
    stroke(&mut s, (320.0, 100.0));
    assert_eq!(count(&s), n + 1, "a new bitmap instead");
    assert!(image(&s, b) == before, "the locked bitmap is unchanged");
    let d = s.active().expect("doc");
    let id = d.selection.elements[0];
    let path = docops::find_on_page(&d.doc, d.page, id).expect("path");
    assert_eq!(docops::owning_layer(&d.doc, d.page, &path), Some(l2));
}

#[test]
fn b_a_stroke_on_empty_space_paints_into_the_selected_bitmap_or_a_new_one() {
    // near the selected bitmap: it grows to take the stroke
    let (mut s, a, _) = mk();
    select(&mut s, &[a]);
    let n = count(&s);
    stroke(&mut s, (160.0, 100.0));
    assert_eq!(count(&s), n);
    let img = image(&s, a);
    assert!(img.bounds().right > 150, "A grew: {:?}", img.bounds());
    assert!(red(&img, 170, 100));
    // a stroke on a vector over nothing, with a bitmap selected → still into the selected bitmap
    let (mut s, a, _) = mk();
    rect(&mut s, 155.0, 80.0, 200.0, 120.0);
    select(&mut s, &[a]);
    let n = count(&s);
    stroke(&mut s, (160.0, 100.0));
    assert_eq!(count(&s), n);
    assert!(red(&image(&s, a), 170, 100));
    // nothing selected → a new bitmap
    let (mut s, a, b) = mk();
    let n = count(&s);
    let v = stroke(&mut s, (200.0, 300.0));
    assert_eq!(count(&s), n + 1);
    let id = ElementId(v["target"].as_u64().expect("target"));
    assert!(id != a && id != b);
    assert_eq!(sel(&s), vec![id]);
}

#[test]
fn d_retouch_takes_the_selected_bitmap_else_the_one_under_the_press_point() {
    let blur = |s: &mut Session, x: f32| {
        s.execute(
            "bitmap.retouch",
            json!({"mode": "blur", "points": [[x, 100.0], [x + 20.0, 100.0]], "size": 13, "soft": 1.0, "strength": 50}),
        )
    };
    // unselected bitmap under the press point → it gets selected
    let (mut s, a, b) = mk();
    blur(&mut s, 320.0).expect("blur");
    assert_eq!(sel(&s), vec![b]);
    // the selected bitmap wins
    let (mut s, a2, _) = mk();
    select(&mut s, &[a2]);
    blur(&mut s, 320.0).expect("blur");
    assert_eq!(sel(&s), vec![a2]);
    // empty space, nothing selected → nothing happens
    let (mut s, _, _) = mk();
    let (n, h) = (count(&s), s.active().map(|d| d.history.len()).unwrap_or(0));
    assert!(blur(&mut s, 220.0).is_err());
    assert_eq!(
        (count(&s), s.active().map(|d| d.history.len()).unwrap_or(0)),
        (n, h)
    );
    let _ = a;
}

#[test]
fn d_red_eye_on_empty_space_inserts_an_empty_bitmap() {
    let (mut s, _, _) = mk();
    let n = count(&s);
    ex(
        &mut s,
        "bitmap.retouch",
        json!({"mode": "redEye", "points": [[200.0, 250.0], [260.0, 300.0]]}),
    );
    assert_eq!(count(&s), n + 1, "an empty bitmap");
    let d = s.active().expect("doc");
    assert!(d.bitmap_mode, "bitmap mode starts");
    let id = d.selection.elements[0];
    assert!(matches!(el(&s, id).kind, ElementKind::Bitmap(_)));
    let label = d.history.labels().last().cloned().unwrap_or_default();
    assert_eq!(label, fp_i18n::t("msg-step-insert-empty-bitmap"));
}
