//! Additional steps (BA-109, `docs/BATCH.md` §3): subject centring, alpha cleanup, round
//! corners, vignette, posterize, threshold, pixelate, pad to multiple, still from animation.

use fp_batch::item::{Animation, Frame, Item, ItemInfo, ItemKind, Meta, still_from_rgba};
use fp_batch::{CancelToken, OutputFormat, Severity, StepContext, StepRegistry};
use fp_raster::{IRect, RgbaImage};
use serde_json::{Value, json};
use std::path::PathBuf;

const IDS: [&str; 9] = [
    "background.subjectCenter",
    "background.alphaCleanup",
    "overlay.roundCorners",
    "overlay.vignette",
    "adjust.posterize",
    "adjust.threshold",
    "filter.pixelate",
    "canvas.pad",
    "anim.firstFrame",
];

fn info(w: u32, h: u32) -> ItemInfo {
    ItemInfo {
        path: PathBuf::from("C:/in/a.png"),
        root: None,
        kind: ItemKind::Image,
        format: "png".into(),
        bytes: 0,
        width: w,
        height: h,
        frames: 1,
        modified: None,
        exif: None,
    }
}

fn apply(id: &str, params: Value, item: &mut Item) -> fp_batch::Result<Vec<String>> {
    let reg = StepRegistry::global();
    let step = reg.get(id).expect("step registered");
    let p = fp_batch::step::with_defaults(&step.params(), &params);
    let (w, h) = item.dimensions();
    let inf = info(w, h);
    let cancel = CancelToken::new();
    let mut ctx = StepContext::new(0, 1, &inf, &cancel, OutputFormat::Png);
    step.apply(item, &p, &mut ctx)?;
    Ok(ctx.warnings)
}

fn img(w: u32, h: u32, f: impl Fn(u32, u32) -> [u8; 4]) -> RgbaImage {
    let data = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .map(|(x, y)| f(x, y))
        .collect();
    RgbaImage::from_raw(IRect::new(0, 0, w as i32, h as i32), data).expect("image")
}

fn still(w: u32, h: u32, f: impl Fn(u32, u32) -> [u8; 4]) -> Item {
    let i = img(w, h, f);
    let raw: Vec<u8> = i.data().iter().flatten().copied().collect();
    still_from_rgba(w, h, &raw).expect("item")
}

fn pixels(item: &Item) -> (usize, usize, Vec<[u8; 4]>) {
    let i = item.preview_image().expect("image");
    (i.width(), i.height(), i.data().to_vec())
}

#[test]
fn steps_are_registered_with_texts() {
    let reg = StepRegistry::global();
    for lang in ["de", "en"] {
        fp_i18n::with_language(lang, || {
            for id in IDS {
                let s = reg.get(id).unwrap_or_else(|| panic!("{id} missing"));
                assert!(fp_i18n::has(&fp_batch::step::step_key(id)), "{lang}: {id}");
                for p in s.params() {
                    let k = fp_batch::step::param_key(id, &p.name);
                    assert!(fp_i18n::has(&k), "{lang}: {k}");
                }
                // defaults are valid
                assert!(s.validate(&json!({})).is_empty(), "{id}");
            }
            assert!(fp_i18n::has("batch-warn-subject-none"));
            assert!(fp_i18n::has("batch-err-alpha-range"));
        });
    }
}

#[test]
fn subject_center_by_alpha_centres_with_margin() {
    // 40×20 transparent, opaque 6×4 block at (2, 3)
    let mut it = still(40, 20, |x, y| {
        if (2..8).contains(&x) && (3..7).contains(&y) {
            [200, 0, 0, 255]
        } else {
            [0, 0, 0, 0]
        }
    });
    apply("background.subjectCenter", json!({"margin": 50}), &mut it).expect("apply");
    let (w, h, d) = pixels(&it);
    // margin 50 % of the longer side (6) = 3 px per side
    assert_eq!((w, h), (12, 10));
    assert_eq!(d[3 * w + 3], [200, 0, 0, 255]);
    assert_eq!(d[6 * w + 8], [200, 0, 0, 255]);
    assert_eq!(d[0][3], 0, "margin stays transparent");
    // square
    let mut it = still(40, 20, |x, y| {
        if (2..8).contains(&x) && (3..7).contains(&y) {
            [200, 0, 0, 255]
        } else {
            [0, 0, 0, 0]
        }
    });
    apply(
        "background.subjectCenter",
        json!({"margin": 0, "square": true}),
        &mut it,
    )
    .expect("apply");
    let (w, h, d) = pixels(&it);
    assert_eq!((w, h), (6, 6));
    assert_eq!(d[0][3], 0);
    assert_eq!(d[w], [200, 0, 0, 255]);
}

#[test]
fn subject_center_by_color_fills_with_corner_colour() {
    let mut it = still(30, 30, |x, y| {
        if (20..26).contains(&x) && (1..5).contains(&y) {
            [0, 0, 255, 255]
        } else {
            [255, 255, 255, 255]
        }
    });
    apply("background.subjectCenter", json!({"margin": 0}), &mut it).expect("apply");
    let (w, h, _) = pixels(&it);
    assert_eq!((w, h), (6, 4));
    let mut it = still(30, 30, |x, y| {
        if (20..26).contains(&x) && (1..5).contains(&y) {
            [0, 0, 255, 255]
        } else {
            [255, 255, 255, 255]
        }
    });
    apply(
        "background.subjectCenter",
        json!({"margin": 0, "square": true}),
        &mut it,
    )
    .expect("apply");
    let (w, h, d) = pixels(&it);
    assert_eq!((w, h), (6, 6));
    assert_eq!(d[0], [255, 255, 255, 255], "filled with the corner colour");
    assert_eq!(d[w * 2 + 2], [0, 0, 255, 255]);
    // nothing found → unchanged with a warning
    let mut it = still(8, 8, |_, _| [255, 255, 255, 255]);
    let warn = apply("background.subjectCenter", json!({}), &mut it).expect("apply");
    assert_eq!(it.dimensions(), (8, 8));
    assert_eq!(warn.len(), 1);
}

#[test]
fn subject_center_uses_union_box_for_animations() {
    let frame = |ox: u32| Frame {
        image: img(20, 10, move |x, y| {
            if x == ox && y == 5 {
                [9, 9, 9, 255]
            } else {
                [0, 0, 0, 0]
            }
        }),
        delay_ms: 100,
    };
    let mut it = Item::Animation(Animation {
        width: 20,
        height: 10,
        frames: vec![frame(2), frame(9)],
        loop_count: 0,
        meta: Meta::default(),
    });
    apply("background.subjectCenter", json!({"margin": 0}), &mut it).expect("apply");
    assert_eq!(it.dimensions(), (8, 1));
    let Item::Animation(a) = &it else { panic!() };
    assert_eq!(a.frames[0].image.data()[0][3], 255);
    assert_eq!(a.frames[1].image.data()[7][3], 255);
}

#[test]
fn alpha_cleanup_thresholds_islands_and_defringe() {
    // 12×12: opaque red block 6×6 at (3,3), a 1px semi-transparent green fringe around it,
    // a faint pixel (alpha 4) and a lone speck at (0, 11)
    let mut it = still(12, 12, |x, y| {
        let inside = (3..9).contains(&x) && (3..9).contains(&y);
        let ring = (2..10).contains(&x) && (2..10).contains(&y);
        if inside {
            [255, 0, 0, 250]
        } else if ring {
            [0, 255, 0, 128]
        } else if (x, y) == (0, 11) {
            [0, 0, 255, 255]
        } else if (x, y) == (11, 0) {
            [0, 0, 255, 4]
        } else {
            [0, 0, 0, 0]
        }
    });
    apply(
        "background.alphaCleanup",
        json!({"low": 8, "high": 247, "minIsland": 4, "defringe": 1}),
        &mut it,
    )
    .expect("apply");
    let (w, _, d) = pixels(&it);
    assert_eq!(d[3 * w + 3], [255, 0, 0, 255], "almost opaque snaps to 255");
    assert_eq!(d[11][3], 0, "faint pixel removed");
    assert_eq!(d[11 * w][3], 0, "speck removed");
    let fringe = d[2 * w + 5];
    assert_eq!(fringe[3], 128, "alpha of the fringe stays");
    assert_eq!(
        &fringe[..3],
        &[255, 0, 0],
        "fringe takes the colour of the subject"
    );
    // invalid range
    let reg = StepRegistry::global();
    let s = reg.get("background.alphaCleanup").expect("step");
    let issues = s.validate(&json!({"low": 200, "high": 100}));
    assert!(issues.iter().any(|i| i.severity == Severity::Error));
}

#[test]
fn round_corners_px_and_percent() {
    let mut it = still(20, 10, |_, _| [10, 20, 30, 255]);
    apply("overlay.roundCorners", json!({"radius": 4}), &mut it).expect("apply");
    let (w, h, d) = pixels(&it);
    assert_eq!((w, h), (20, 10));
    assert_eq!(d[0][3], 0, "corner transparent");
    assert_eq!(d[5 * w + 10], [10, 20, 30, 255], "centre untouched");
    assert_eq!(d[5 * w][3], 255, "edge midpoint untouched");
    let partial = d[w + 1][3];
    assert!(partial > 0 && partial < 255, "anti-aliased: {partial}");
    assert!(it.meta().expect("meta").has_transparency);
    // 50 % → a capsule: pixel (1, 0) is outside
    let mut it = still(20, 10, |_, _| [10, 20, 30, 255]);
    apply(
        "overlay.roundCorners",
        json!({"radius": 50, "unit": "percent"}),
        &mut it,
    )
    .expect("apply");
    let (w, _, d) = pixels(&it);
    assert_eq!(d[1][3], 0);
    assert_eq!(d[5 * w + 10][3], 255);
    // radius 0 → unchanged
    let mut it = still(4, 4, |_, _| [1, 2, 3, 255]);
    apply("overlay.roundCorners", json!({"radius": 0}), &mut it).expect("apply");
    assert!(pixels(&it).2.iter().all(|p| *p == [1, 2, 3, 255]));
}

#[test]
fn vignette_darkens_corners_only() {
    let mut it = still(41, 41, |_, _| [200, 200, 200, 255]);
    apply(
        "overlay.vignette",
        json!({"strength": 100, "size": 50}),
        &mut it,
    )
    .expect("apply");
    let (w, _, d) = pixels(&it);
    assert_eq!(d[20 * w + 20], [200, 200, 200, 255]);
    assert!(d[0][0] < 20, "corner nearly black: {:?}", d[0]);
    assert_eq!(d[0][3], 255);
}

#[test]
fn posterize_and_threshold() {
    let mut it = still(256, 1, |x, _| [x as u8, x as u8, 255 - x as u8, 255]);
    apply("adjust.posterize", json!({"levels": 2}), &mut it).expect("apply");
    let (_, _, d) = pixels(&it);
    assert!(
        d.iter()
            .all(|p| (p[0] == 0 || p[0] == 255) && (p[2] == 0 || p[2] == 255))
    );
    assert_eq!(d[100][0], 0);
    assert_eq!(d[200][0], 255);
    let mut it = still(256, 1, |x, _| [x as u8, x as u8, x as u8, 77]);
    apply("adjust.threshold", json!({"threshold": 128}), &mut it).expect("apply");
    let (_, _, d) = pixels(&it);
    assert_eq!(d[127], [0, 0, 0, 77]);
    assert_eq!(d[128], [255, 255, 255, 77]);
}

#[test]
fn pixelate_averages_blocks() {
    let mut it = still(4, 2, |x, _| {
        if x % 2 == 0 {
            [0, 0, 0, 255]
        } else {
            [200, 100, 50, 255]
        }
    });
    apply("filter.pixelate", json!({"size": 2}), &mut it).expect("apply");
    let (_, _, d) = pixels(&it);
    assert!(d.iter().all(|p| *p == [100, 50, 25, 255]), "{d:?}");
}

#[test]
fn canvas_pad_to_multiple() {
    let mut it = still(13, 16, |_, _| [5, 5, 5, 255]);
    apply(
        "canvas.pad",
        json!({"multiple": 8, "color": "#ff0000"}),
        &mut it,
    )
    .expect("apply");
    let (w, h, d) = pixels(&it);
    assert_eq!((w, h), (16, 16));
    assert_eq!(d[0], [255, 0, 0, 255]);
    assert_eq!(d[1], [5, 5, 5, 255]);
    assert_eq!(d[15], [255, 0, 0, 255]);
    // already a multiple → unchanged size
    let mut it = still(16, 32, |_, _| [5, 5, 5, 255]);
    apply("canvas.pad", json!({}), &mut it).expect("apply");
    assert_eq!(it.dimensions(), (16, 32));
}

fn anim() -> Item {
    // frame 2 is sharp (checkerboard), others flat
    let frames = (0..4u8)
        .map(|k| Frame {
            image: img(8, 8, move |x, y| {
                if k == 2 && (x + y) % 2 == 0 {
                    [255, 255, 255, 255]
                } else {
                    [k * 10, 0, 0, 255]
                }
            }),
            delay_ms: 100,
        })
        .collect();
    Item::Animation(Animation {
        width: 8,
        height: 8,
        frames,
        loop_count: 0,
        meta: Meta::default(),
    })
}

#[test]
fn still_from_animation() {
    let red = |it: &Item| pixels(it).2[1][0];
    for (p, want) in [
        (json!({}), 0),
        (json!({"pick": "middle"}), 10),
        (json!({"pick": "last"}), 30),
        (json!({"pick": "number", "number": 2}), 10),
        (json!({"pick": "number", "number": 99}), 30),
        (json!({"pick": "time", "time": 150}), 10),
        (json!({"pick": "time", "time": 10_000}), 30),
    ] {
        let mut it = anim();
        apply("anim.firstFrame", p.clone(), &mut it).expect("apply");
        assert_eq!(it.kind(), ItemKind::Image, "{p}");
        assert_eq!(red(&it), want, "{p}");
    }
    let mut it = anim();
    apply("anim.firstFrame", json!({"pick": "sharpest"}), &mut it).expect("apply");
    assert_eq!(pixels(&it).2[0], [255, 255, 255, 255]);
}
