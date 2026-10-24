//! FB-52 (Firepixel extension): the option "Confine to boundaries" of the smudge also exists for the
//! other bitmap tools that can paint past the bitmap edge: Blur, Rubber Stamp, Brush and Pencil
//! (`<Tool>ConfineToBounds`, default off; sent as `confineToBounds` of `bitmap.retouch` /
//! `bitmap.paint`). With the option on these tools never grow the bitmap and change no pixel
//! outside it: the blur no longer grows it by 2 px nor mixes the transparent white outside into
//! the edge, the Rubber Stamp, Brush and Pencil lay nothing down outside. Sharpen, Dodge and Burn
//! work only inside the bitmap anyway (RE spec 70 section 1.9) and have no such option. The
//! reference has the option for no tool (RE spec 70 section 3).

use crate::common;
use common::*;
use fp_engine::tools::ToolId;
use fp_engine::{PrefValue, Session, ToolEvent};
use fp_geom::IRect;
use fp_model::{ElementId, ElementKind};
use serde_json::json;

const RECT: IRect = IRect {
    left: 0,
    top: 100,
    right: 100,
    bottom: 200,
};

fn image(s: &Session, id: ElementId) -> fp_raster::RgbaImage {
    match &el(s, id).kind {
        ElementKind::Bitmap(b) => fp_engine::docops::bitmap_image(b).expect("image"),
        _ => panic!("not a bitmap"),
    }
}

/// Bitmap 100 x 100 at 0/100: mid grey with a darker stripe x 40..60 (so every tool has something
/// to work on) and a uniform grey edge at the right.
fn bitmap(s: &mut Session) -> ElementId {
    let mut img = fp_raster::RgbaImage::new(RECT).expect("image");
    for y in RECT.top..RECT.bottom {
        for x in RECT.left..RECT.right {
            let v = if (40..60).contains(&x) { 60 } else { 128 };
            img.set_pixel(x, y, fp_color::Rgba8::new(v, v, v, 255));
        }
    }
    s.clipboard.clear();
    s.clipboard.pixels = Some(img);
    let v = ex(s, "edit.paste", json!({}));
    ElementId(v["ids"][0].as_u64().expect("id"))
}

fn confine_key(tool: ToolId) -> &'static str {
    match tool {
        ToolId::Blur => "BlurToolConfineToBounds",
        ToolId::Smudge => "SmudgeToolConfineToBounds",
        ToolId::RubberStamp => "StampToolConfineToBounds",
        ToolId::Brush => "BrushToolConfineToBounds",
        ToolId::Pencil => "PencilToolConfineToBounds",
        ToolId::Sharpen => "SharpenToolConfineToBounds",
        ToolId::Dodge => "DodgeToolConfineToBounds",
        _ => "BurnToolConfineToBounds",
    }
}

/// Session with the bitmap selected and `tool` active (tip 22, the option as given); the stamp
/// source is Alt-clicked 40 px left of `src_for`.
fn setup(tool: ToolId, confine: bool, src_for: (f32, f32)) -> (Session, ElementId) {
    let mut s = doc_session(300.0, 300.0);
    let id = bitmap(&mut s);
    select(&mut s, &[id]);
    match tool {
        ToolId::RubberStamp => s.prefs.set("StampToolSize", PrefValue::Int(22)),
        ToolId::Blur => s.prefs.set("BlurToolDiameter", PrefValue::Int(22)),
        ToolId::Sharpen => s.prefs.set("SharpenToolDiameter", PrefValue::Int(22)),
        ToolId::Dodge => s.prefs.set("DodgeToolDiameter", PrefValue::Int(22)),
        ToolId::Burn => s.prefs.set("BurnToolDiameter", PrefValue::Int(22)),
        _ => s.tools.options.brush_size = 12.0,
    }
    s.prefs.set(confine_key(tool), PrefValue::Bool(confine));
    s.set_tool(tool).unwrap_or_else(|e| panic!("{e}"));
    if tool == ToolId::RubberStamp {
        let (x, y) = (src_for.0 - 40.0, src_for.1);
        events(
            &mut s,
            &[ToolEvent::down(x, y).alt(), ToolEvent::up(x, y).alt()],
        );
    }
    (s, id)
}

fn drag(s: &mut Session, pts: &[(f32, f32)]) {
    events(s, &[ToolEvent::down(pts[0].0, pts[0].1)]);
    for p in &pts[1..] {
        events(s, &[ToolEvent::drag(p.0, p.1)]);
    }
    let e = pts[pts.len() - 1];
    events(s, &[ToolEvent::up(e.0, e.1)]);
}

/// Stroke with `tool` through the points; returns (bitmap before, after).
fn stroke(
    tool: ToolId,
    confine: bool,
    pts: &[(f32, f32)],
) -> (fp_raster::RgbaImage, fp_raster::RgbaImage) {
    let (mut s, id) = setup(tool, confine, pts[0]);
    let base = image(&s, id);
    drag(&mut s, pts);
    (base, image(&s, id))
}

fn line(from: (f32, f32), to: (f32, f32), n: usize) -> Vec<(f32, f32)> {
    (0..=n)
        .map(|i| {
            let t = i as f32 / n as f32;
            (from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t)
        })
        .collect()
}

/// Down along the right edge (centre on the bitmap) and a stroke pressed outside, across the edge.
fn edge_strokes() -> Vec<(&'static str, Vec<(f32, f32)>)> {
    vec![
        ("along-edge", line((95.0, 120.0), (95.0, 180.0), 30)),
        ("pressed-outside", line((106.0, 150.0), (80.0, 160.0), 20)),
    ]
}

#[test]
fn blur_with_the_option_neither_grows_nor_mixes_in_the_outside() {
    for (name, pts) in edge_strokes() {
        let (_, free) = stroke(ToolId::Blur, false, &pts);
        assert!(
            free.bounds().right > 100,
            "{name}: without the option the blur grows: {:?}",
            free.bounds()
        );
        assert!(
            free.get(99, 150).is_some_and(|p| p[3] < 255),
            "{name}: without the option the outside is mixed into the edge"
        );
        let (base, conf) = stroke(ToolId::Blur, true, &pts);
        assert_eq!(conf.bounds(), RECT, "{name}: confined");
        for y in RECT.top..RECT.bottom {
            assert_eq!(
                conf.get(99, y),
                base.get(99, y),
                "{name}: the uniform edge stays as it was (y {y})"
            );
        }
    }
}

#[test]
fn stamp_brush_and_pencil_with_the_option_lay_nothing_down_outside() {
    // from the right outside into the bitmap, and a stroke across the right edge
    let strokes = [
        line((106.0, 150.0), (60.0, 150.0), 30),
        line((80.0, 130.0), (130.0, 130.0), 30),
    ];
    for tool in [ToolId::RubberStamp, ToolId::Brush, ToolId::Pencil] {
        for pts in &strokes {
            let (_, free) = stroke(tool, false, pts);
            assert!(
                free.bounds().right > 100,
                "{tool:?}: without the option the bitmap grows: {:?}",
                free.bounds()
            );
            let (base, conf) = stroke(tool, true, pts);
            assert_eq!(conf.bounds(), RECT, "{tool:?}: confined");
            assert_ne!(conf, base, "{tool:?}: the confined stroke works inside");
        }
    }
}

#[test]
fn brush_with_the_option_leaves_the_bitmap_alone_for_a_detached_stroke() {
    // 60 px right of the selected bitmap: without the option the bitmap grows to include the
    // stroke (RE spec 70 section 1.8), with it nothing changes
    let pts = line((160.0, 150.0), (220.0, 150.0), 30);
    for tool in [ToolId::Brush, ToolId::Pencil] {
        let (_, free) = stroke(tool, false, &pts);
        assert!(free.bounds().right > 200, "{tool:?}: {:?}", free.bounds());
        let (base, conf) = stroke(tool, true, &pts);
        assert_eq!(conf, base, "{tool:?}: confined, nothing outside");
    }
}

#[test]
fn sharpen_dodge_and_burn_have_no_option_and_work_only_inside() {
    let pts = line((106.0, 150.0), (30.0, 150.0), 40);
    for tool in [ToolId::Sharpen, ToolId::Dodge, ToolId::Burn] {
        let (base, free) = stroke(tool, false, &pts);
        let (_, set) = stroke(tool, true, &pts);
        assert_eq!(free.bounds(), RECT, "{tool:?}");
        assert_ne!(free, base, "{tool:?}: the stroke works");
        assert_eq!(set, free, "{tool:?}: a stray pref changes nothing");
        let (mut s, _) = setup(tool, true, pts[0]);
        drag(&mut s, &pts);
        let d = s.active().expect("document");
        let params = d.history.top().expect("step").params.clone();
        assert!(
            params.get("confineToBounds").is_none(),
            "{tool:?}: not sent: {params}"
        );
    }
}

#[test]
fn the_option_is_sent_per_tool() {
    for tool in [
        ToolId::Blur,
        ToolId::Smudge,
        ToolId::RubberStamp,
        ToolId::Brush,
        ToolId::Pencil,
    ] {
        for confine in [false, true] {
            let (mut s, _) = setup(tool, confine, (50.0, 150.0));
            drag(&mut s, &[(50.0, 150.0), (60.0, 150.0), (70.0, 150.0)]);
            let d = s.active().expect("document");
            let params = d.history.top().expect("step").params.clone();
            assert_eq!(
                params["confineToBounds"],
                json!(confine),
                "{tool:?} confine {confine}: {params}"
            );
        }
    }
}
