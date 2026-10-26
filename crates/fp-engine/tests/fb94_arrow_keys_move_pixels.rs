//! FB-94 (preference `Firepixel_ArrowKeysMovePixels`, off by default): with it on and a pixel
//! selection, the arrow keys (1 px, Shift 10 px) move the selected pixels while the Pointer, Select
//! Behind, the Subselection or the Mask Element Selection is active (cut from their place, the hole
//! transparent, the marquee follows; consecutive presses one History row "Move"); Alt+arrow
//! duplicates the pixels (row "Duplicate"). The bitmap selection tools keep moving only the
//! marquee. Off, the reference rule stays: the marquee alone moves with every tool (RE spec 71
//! section 6.3, `fb73_fb74_pixel_move`).

use crate::common;
use common::*;
use fp_engine::prefs::PrefValue;
use fp_engine::tools::{ARROW_KEYS_MOVE_PIXELS_PREF, ToolId, ToolKey};
use fp_engine::{Session, ToolEvent};
use fp_geom::IRect;
use fp_model::{ElementId, ElementKind};
use serde_json::json;

const POINTER_TOOLS: [ToolId; 4] = [
    ToolId::Pointer,
    ToolId::SelectBehind,
    ToolId::Subselection,
    ToolId::MaskElementSelect,
];
const BITMAP_TOOLS: [ToolId; 3] = [ToolId::Marquee, ToolId::Lasso, ToolId::MagicWand];

fn rows(s: &Session) -> usize {
    s.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

fn label(s: &Session) -> String {
    s.active()
        .and_then(|d| d.history.undo_label())
        .unwrap_or_default()
        .to_string()
}

fn pixel_bounds(s: &Session) -> Option<IRect> {
    s.active()
        .and_then(|d| d.selection.pixels.as_ref())
        .map(|m| m.tight_bounds())
}

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

/// A 400 × 300 bitmap, selected, with the pixel selection 100,100 – 150,150; the option set.
fn setup(t: ToolId, on: bool) -> (Session, ElementId) {
    let mut s = doc_session(400.0, 300.0);
    s.prefs
        .set(ARROW_KEYS_MOVE_PIXELS_PREF, PrefValue::Bool(on));
    let id = bitmap(&mut s, 0, 0, 400, 300, [200, 40, 40, 255]);
    ex(
        &mut s,
        "select.pixels",
        json!({"shape": "rect", "rect": [100, 100, 150, 150]}),
    );
    s.set_tool(t).unwrap_or_else(|e| panic!("{e}"));
    // the tool change keeps the pixel selection
    assert_eq!(pixel_bounds(&s), Some(IRect::new(100, 100, 150, 150)));
    (s, id)
}

fn press_right_right_shift_down(s: &mut Session, alt: bool) {
    let k = |k: ToolKey| {
        let e = ToolEvent::key(k);
        if alt { e.alt() } else { e }
    };
    events(
        s,
        &[
            k(ToolKey::Right),
            k(ToolKey::Right),
            k(ToolKey::Down).shift(),
        ],
    );
}

/// On, pointer-type tools: the pixels move with the marquee, the hole is transparent, one row
/// "Move", one undo restores pixels and marquee.
#[test]
fn fb94_on_pointer_tools_move_the_pixels() {
    for t in POINTER_TOOLS {
        let (mut s, id) = setup(t, true);
        let r0 = rows(&s);
        let img0 = image(&s, id);
        press_right_right_shift_down(&mut s, false);
        assert_eq!(
            pixel_bounds(&s),
            Some(IRect::new(102, 110, 152, 160)),
            "{t:?} marquee follows"
        );
        assert_eq!(alpha(&s, id, 101, 101), 0, "{t:?} hole transparent");
        assert_eq!(alpha(&s, id, 151, 159), 255, "{t:?} pixels at the offset");
        assert_eq!(alpha(&s, id, 50, 50), 255, "{t:?} rest unchanged");
        assert_eq!(rows(&s), r0 + 1, "{t:?} one History row");
        assert_eq!(label(&s), fp_i18n::t("msg-step-move"), "{t:?} row name");
        ex(&mut s, "edit.undo", json!({}));
        assert!(image(&s, id) == img0, "{t:?} undo restores the pixels");
        assert_eq!(
            pixel_bounds(&s),
            Some(IRect::new(100, 100, 150, 150)),
            "{t:?} undo restores the marquee"
        );
    }
}

/// On, Alt+arrow with a pointer-type tool duplicates the pixels: the original stays, the copy
/// lands at the offset with the marquee; consecutive presses one row "Duplicate".
#[test]
fn fb94_on_alt_arrow_duplicates_the_pixels() {
    for t in POINTER_TOOLS {
        let (mut s, id) = setup(t, true);
        let r0 = rows(&s);
        let img0 = image(&s, id);
        press_right_right_shift_down(&mut s, true);
        assert_eq!(
            pixel_bounds(&s),
            Some(IRect::new(102, 110, 152, 160)),
            "{t:?} marquee on the copy"
        );
        // the original stays whole under the copy (no hole from re-moving the copy)
        for (x, y) in [(100, 100), (101, 101), (149, 105), (102, 109)] {
            assert_eq!(alpha(&s, id, x, y), 255, "{t:?} original stays at {x},{y}");
        }
        assert!(
            image(&s, id) == img0,
            "{t:?} a same-coloured copy leaves the image equal"
        );
        assert_eq!(rows(&s), r0 + 1, "{t:?} one History row");
        assert_eq!(
            label(&s),
            fp_i18n::t("msg-step-duplicate"),
            "{t:?} row name"
        );
        ex(&mut s, "edit.undo", json!({}));
        assert!(image(&s, id) == img0, "{t:?} undo restores the pixels");
        assert_eq!(
            pixel_bounds(&s),
            Some(IRect::new(100, 100, 150, 150)),
            "{t:?} undo"
        );
    }
}

/// On, the bitmap selection tools keep moving the marquee alone.
#[test]
fn fb94_on_bitmap_tools_move_only_the_marquee() {
    for t in BITMAP_TOOLS {
        let (mut s, id) = setup(t, true);
        let r0 = rows(&s);
        let img0 = image(&s, id);
        press_right_right_shift_down(&mut s, false);
        assert_eq!(
            pixel_bounds(&s),
            Some(IRect::new(102, 110, 152, 160)),
            "{t:?} marquee moved"
        );
        assert!(image(&s, id) == img0, "{t:?} pixels unchanged");
        assert_eq!(rows(&s), r0 + 1, "{t:?} one History row");
        assert_eq!(label(&s), fp_i18n::t("msg-step-move"), "{t:?} row name");
        ex(&mut s, "edit.undo", json!({}));
        assert_eq!(
            pixel_bounds(&s),
            Some(IRect::new(100, 100, 150, 150)),
            "{t:?} undo"
        );
    }
}

/// Off (default), every tool moves the marquee alone (reference, RE spec 71 section 6.3).
#[test]
fn fb94_off_every_tool_moves_only_the_marquee() {
    for t in POINTER_TOOLS.into_iter().chain(BITMAP_TOOLS) {
        let (mut s, id) = setup(t, false);
        let r0 = rows(&s);
        let img0 = image(&s, id);
        press_right_right_shift_down(&mut s, false);
        assert_eq!(
            pixel_bounds(&s),
            Some(IRect::new(102, 110, 152, 160)),
            "{t:?} marquee moved"
        );
        assert!(image(&s, id) == img0, "{t:?} pixels unchanged");
        assert_eq!(rows(&s), r0 + 1, "{t:?} one History row");
    }
}

/// The option is off unless set.
#[test]
fn fb94_default_is_off() {
    let s = doc_session(100.0, 100.0);
    assert!(!s.prefs.get_bool(ARROW_KEYS_MOVE_PIXELS_PREF, false));
}
