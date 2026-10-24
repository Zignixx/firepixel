//! FB-51 (tester's recordings, smudge tool; RE spec 70 section 1): a smudge stroke behaved
//! differently depending on where the brush centre was at the press. Measured rule of the
//! reference (section 1.6): with the bitmap selected the press position does not matter and the
//! smear is never cut at the bitmap bounds. Without "Use Entire Document" the bitmap grows where
//! opaque colour is carried (dragging transparency in grows nothing); with it the bitmap grows to
//! the stroke rectangle (tip radius included, filled with the page image), also when pressed
//! outside and even for a stroke entirely outside. With nothing selected a stroke pressed beside
//! the bitmap does nothing (no history row); a stroke entirely outside the selected bitmap without
//! "Use Entire Document" changes no pixel but leaves one history row. Firepixel's own option
//! "Confine to boundaries" keeps every stroke inside the bitmap rectangle.
//!
//! Set-up of the measurement: bitmap 80 x 60 at 160/120 (vertical stripes red/green/blue/yellow,
//! 20 px each), white canvas, tip 22 (radius 11), edge 50, pressure 50, strokes at y 150.

use crate::common;
use common::*;
use fp_engine::tools::ToolId;
use fp_engine::{Overlay, PrefValue, Session, ToolEvent};
use fp_geom::IRect;
use fp_model::{ElementId, ElementKind};
use serde_json::json;

const RECT: IRect = IRect {
    left: 160,
    top: 120,
    right: 240,
    bottom: 180,
};

fn image(s: &Session, id: ElementId) -> fp_raster::RgbaImage {
    match &el(s, id).kind {
        ElementKind::Bitmap(b) => fp_engine::docops::bitmap_image(b).expect("image"),
        _ => panic!("not a bitmap"),
    }
}

fn steps(s: &Session) -> usize {
    s.active().expect("document").history.undo_count()
}

/// The striped bitmap of the measurement.
fn stripes(s: &mut Session) -> ElementId {
    let mut img = fp_raster::RgbaImage::new(RECT).expect("image");
    let cols = [
        [255, 0, 0, 255],
        [0, 255, 0, 255],
        [0, 0, 255, 255],
        [255, 255, 0, 255],
    ];
    for y in RECT.top..RECT.bottom {
        for x in RECT.left..RECT.right {
            let c = cols[((x - RECT.left) / 20) as usize];
            img.set_pixel(x, y, fp_color::Rgba8::new(c[0], c[1], c[2], c[3]));
        }
    }
    s.clipboard.clear();
    s.clipboard.pixels = Some(img);
    let v = ex(s, "edit.paste", json!({}));
    ElementId(v["ids"][0].as_u64().expect("id"))
}

#[derive(Clone, Copy)]
struct Opt {
    selected: bool,
    whole: bool,
    confine: bool,
    finger: bool,
}

const PTR: Opt = Opt {
    selected: true,
    whole: false,
    confine: false,
    finger: false,
};

fn setup(o: Opt) -> (Session, ElementId) {
    let mut s = doc_session(400.0, 300.0);
    let id = stripes(&mut s);
    assert_eq!(image(&s, id).bounds(), RECT);
    let ids: Vec<ElementId> = if o.selected { vec![id] } else { Vec::new() };
    select(&mut s, &ids);
    s.prefs.set("SmudgeToolDiameter", PrefValue::Int(22));
    s.prefs.set("SmudgeToolSoftEdge", PrefValue::Int(500));
    s.prefs.set("SmudgeToolPressure", PrefValue::Int(50));
    s.prefs
        .set("SmudgeToolConfineToBounds", PrefValue::Bool(o.confine));
    s.prefs
        .set("SmudgeToolUseEntireDocument", PrefValue::Bool(o.whole));
    s.prefs
        .set("SmudgeToolFingerPaint", PrefValue::Bool(o.finger));
    s.set_tool(ToolId::Smudge).unwrap_or_else(|e| panic!("{e}"));
    (s, id)
}

/// Press, moves (25 steps per piece, as measured), release at the last point; returns the last
/// live preview.
fn stroke(s: &mut Session, xs: &[f32]) -> Option<fp_engine::paint::PaintPreview> {
    let mut pts = vec![(xs[0], 150.0)];
    for w in xs.windows(2) {
        for i in 1..=25 {
            pts.push((w[0] + (w[1] - w[0]) * i as f32 / 25.0, 150.0));
        }
    }
    events(s, &[ToolEvent::down(pts[0].0, pts[0].1)]);
    let mut last = None;
    for p in &pts[1..] {
        events(s, &[ToolEvent::drag(p.0, p.1)]);
        last = s.tool_feedback().into_iter().find_map(|o| match o {
            Overlay::Paint(p) => Some(p),
            _ => None,
        });
    }
    let e = pts[pts.len() - 1];
    events(s, &[ToolEvent::up(e.0, e.1)]);
    last
}

/// Runs a stroke; returns the bitmap afterwards and the number of new history rows.
fn run(o: Opt, xs: &[f32]) -> (fp_raster::RgbaImage, fp_raster::RgbaImage, usize) {
    let (mut s, id) = setup(o);
    let base = image(&s, id);
    let n0 = steps(&s);
    stroke(&mut s, xs);
    (base, image(&s, id), steps(&s) - n0)
}

// the strokes of RE spec 70 section 1.1 (document x)
const A: &[f32] = &[175.0, 225.0];
const B: &[f32] = &[152.0, 200.0];
const B2: &[f32] = &[140.0, 200.0];
const C: &[f32] = &[220.0, 280.0];
const C2: &[f32] = &[180.0, 120.0];
const F: &[f32] = &[152.0, 200.0, 120.0];
const D: &[f32] = &[300.0, 360.0];

#[test]
fn pressed_outside_with_the_bitmap_selected_works_like_pressed_inside() {
    for (name, xs) in [("b", B), ("b2", B2)] {
        let (base, img, rows) = run(PTR, xs);
        assert_eq!(rows, 1, "{name}: one Smudge row");
        assert_eq!(
            img.bounds(),
            RECT,
            "{name}: transparency dragged in grows nothing"
        );
        assert_ne!(img, base, "{name}: the stroke works");
        let p = img.get(163, 150).unwrap_or([0; 4]);
        assert!(p[3] < 255, "{name}: transparency is smeared in: {p:?}");
    }
    let (base, img, _) = run(PTR, A);
    assert_eq!(img.bounds(), RECT, "a: inside stays inside");
    assert_ne!(img, base);
}

#[test]
fn the_smear_is_never_cut_at_the_bounds_without_entire_document() {
    // c: pressed inside, out to the right (reference 264); c2: out to the left (reference 136);
    // f: pressed outside, in and out again on the left (reference 151, colour diluted)
    let (_, img, _) = run(PTR, C);
    let b = img.bounds();
    assert!(
        b.right > 250 && b.right <= 291,
        "c grows to the right: {b:?}"
    );
    assert_eq!((b.left, b.top, b.bottom), (160, 120, 180), "c: {b:?}");
    let (_, img, _) = run(PTR, C2);
    let b = img.bounds();
    assert!(b.left < 150 && b.left >= 109, "c2 grows to the left: {b:?}");
    assert_eq!(b.right, 240, "c2: {b:?}");
    let (_, img, _) = run(PTR, F);
    let b = img.bounds();
    assert!(
        b.left < 160 && b.left >= 109,
        "f (pressed outside) grows to the left: {b:?}"
    );
    assert_eq!(b.right, 240, "f: {b:?}");
}

#[test]
fn entire_document_grows_to_the_stroke_rectangle_wherever_it_was_pressed() {
    let ed = Opt { whole: true, ..PTR };
    // measured left/right edges (RE spec 70 section 1.3)
    for (name, xs, left, right) in [
        ("a", A, 160, 240),
        ("b", B, 142, 240),
        ("b2", B2, 130, 240),
        ("c", C, 160, 291),
        ("c2", C2, 109, 240),
        ("f", F, 109, 240),
    ] {
        let (_, img, rows) = run(ed, xs);
        assert_eq!(rows, 1, "{name}");
        assert_eq!(
            img.bounds(),
            IRect::new(left, 120, right, 180),
            "{name}: grows to the stroke rectangle"
        );
    }
    // b: the corner of the stroke rectangle the round tip never touched is the page (white)
    let (_, img, _) = run(ed, B);
    assert_eq!(
        img.get(142, 139),
        Some([255, 255, 255, 255]),
        "corner = page image"
    );
    assert_eq!(
        img.get(142, 120),
        Some([0; 4]),
        "above the stroke: transparent"
    );
}

#[test]
fn entire_document_stroke_entirely_outside_becomes_part_of_the_bitmap() {
    let (_, img, rows) = run(Opt { whole: true, ..PTR }, D);
    assert_eq!(rows, 1);
    // stroke rectangle 290..371 x 139..161 (reference: right edge 371; the press dab at 300
    // picks nothing up, the rectangle starts with the next dab at 301), gap transparent
    assert_eq!(img.bounds(), IRect::new(160, 120, 371, 180));
    assert_eq!(img.get(330, 150), Some([255, 255, 255, 255]), "white strip");
    assert_eq!(
        img.get(290, 139),
        Some([255, 255, 255, 255]),
        "strip corner"
    );
    assert_eq!(
        img.get(370, 160),
        Some([255, 255, 255, 255]),
        "strip corner"
    );
    assert_eq!(
        img.get(289, 150).map(|p| p[3]),
        Some(0),
        "left of the strip"
    );
    assert_eq!(img.get(260, 150).map(|p| p[3]), Some(0), "gap transparent");
    assert_eq!(img.get(330, 135).map(|p| p[3]), Some(0), "above the strip");
}

#[test]
fn stroke_entirely_outside_without_entire_document_changes_nothing_but_leaves_a_row() {
    let (base, img, rows) = run(PTR, D);
    assert_eq!(img, base, "no pixel changed");
    assert_eq!(rows, 1, "one Smudge history row");
}

#[test]
fn smudge_color_pressed_outside_grows_the_bitmap() {
    // reference: b -> left 141 (the swatch colour is laid down at the press point outside)
    let (_, img, _) = run(
        Opt {
            finger: true,
            ..PTR
        },
        B,
    );
    let b = img.bounds();
    assert!(b.left <= 145 && b.left >= 140, "b with Smudge color: {b:?}");
    // d: smear of the swatch colour entirely outside becomes part of the bitmap (reference 328)
    let (_, img, _) = run(
        Opt {
            finger: true,
            ..PTR
        },
        D,
    );
    let b = img.bounds();
    assert!(b.right > 300, "d with Smudge color: {b:?}");
}

#[test]
fn nothing_selected_and_pressed_beside_the_bitmap_does_nothing() {
    for whole in [false, true] {
        for (name, xs) in [("b", B), ("f", F)] {
            let o = Opt {
                selected: false,
                whole,
                ..PTR
            };
            let (base, img, rows) = run(o, xs);
            assert_eq!(img, base, "{name} whole {whole}: no change");
            assert_eq!(rows, 0, "{name} whole {whole}: no history row");
        }
    }
    // a press on the bitmap selects it and works
    let (base, img, rows) = run(
        Opt {
            selected: false,
            ..PTR
        },
        A,
    );
    assert_ne!(img, base);
    assert!(rows >= 1);
}

#[test]
fn confine_to_boundaries_keeps_every_stroke_inside() {
    for whole in [false, true] {
        for (name, xs) in [("a", A), ("b", B), ("c", C), ("c2", C2), ("f", F), ("d", D)] {
            let o = Opt {
                confine: true,
                whole,
                ..PTR
            };
            let (_, img, _) = run(o, xs);
            assert_eq!(img.bounds(), RECT, "{name} whole {whole}: confined");
        }
    }
}

/// The last live preview equals the stored bitmap (pressed outside, with and without "Use Entire
/// Document").
#[test]
fn live_preview_equals_the_result_for_strokes_pressed_outside() {
    for whole in [false, true] {
        for (name, xs) in [("b", B), ("f", F), ("d", D)] {
            let (mut s, id) = setup(Opt { whole, ..PTR });
            let base = image(&s, id);
            let Some(p) = stroke(&mut s, xs) else {
                panic!("{name}: live preview");
            };
            let res = image(&s, id);
            let norm = |p: [u8; 4]| if p[3] == 0 { [0; 4] } else { p };
            let u = res.bounds().union(&base.bounds());
            let mut diff = 0;
            for y in u.top..u.bottom {
                for x in u.left..u.right {
                    let orig = base.get(x, y).unwrap_or([0; 4]);
                    let seen = match p.tiles.iter().find(|t| t.rect.contains(x, y)) {
                        Some(t) => {
                            let v = t.image.get(x, y).unwrap_or([0; 4]);
                            if v[3] == 0 {
                                orig
                            } else {
                                t.exact.as_ref().and_then(|e| e.get(x, y)).unwrap_or(v)
                            }
                        }
                        None => orig,
                    };
                    if norm(seen) != norm(res.get(x, y).unwrap_or([0; 4])) {
                        diff += 1;
                    }
                }
            }
            assert_eq!(diff, 0, "{name} whole {whole}: preview equals the result");
        }
    }
}
