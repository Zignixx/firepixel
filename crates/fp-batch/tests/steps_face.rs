//! `ai.faceBlur` (BA-109, `docs/BATCH.md` §3.4): blur or pixelate faces found by the built-in
//! detector of `fp-ai`.

use fp_batch::item::{Item, ItemInfo, ItemKind, still_from_rgba};
use fp_batch::{CancelToken, OutputFormat, Severity, StepContext, StepRegistry};
use serde_json::{Value, json};
use std::path::PathBuf;

const ID: &str = "ai.faceBlur";

fn info(w: u32, h: u32) -> ItemInfo {
    ItemInfo {
        path: PathBuf::from("a.jpg"),
        root: None,
        kind: ItemKind::Image,
        format: "jpeg".into(),
        bytes: 0,
        width: w,
        height: h,
        frames: 1,
        modified: None,
        exif: None,
    }
}

fn apply_with(
    params: Value,
    item: &mut Item,
    cancel: &CancelToken,
) -> fp_batch::Result<Vec<String>> {
    let reg = StepRegistry::global();
    let step = reg.get(ID).expect("step registered");
    let p = fp_batch::step::with_defaults(&step.params(), &params);
    let (w, h) = item.dimensions();
    let inf = info(w, h);
    let mut ctx = StepContext::new(0, 1, &inf, cancel, OutputFormat::Png);
    step.apply(item, &p, &mut ctx)?;
    Ok(ctx.warnings)
}

fn apply(params: Value, item: &mut Item) -> fp_batch::Result<Vec<String>> {
    apply_with(params, item, &CancelToken::new())
}

fn portrait() -> Item {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fp-ai/testdata/person_haare.jpg");
    let bytes = std::fs::read(&p).expect("portrait");
    let img = fp_io::jpeg_in::decode(&bytes).expect("decode").image;
    let raw: Vec<u8> = img.data().iter().flatten().copied().collect();
    still_from_rgba(img.width() as u32, img.height() as u32, &raw).expect("item")
}

fn pixels(item: &Item) -> (Vec<[u8; 4]>, usize) {
    match item {
        Item::Still(s) => (s.image.data().to_vec(), s.image.width()),
        _ => panic!("still expected"),
    }
}

/// Mean absolute difference of the RGB channels in a square around (x, y).
fn diff(a: &[[u8; 4]], b: &[[u8; 4]], w: usize, x: usize, y: usize, r: usize) -> f64 {
    let mut s = 0u64;
    let mut n = 0u64;
    for yy in y - r..y + r {
        for xx in x - r..x + r {
            let (p, q) = (a[yy * w + xx], b[yy * w + xx]);
            s += (0..3)
                .map(|c| (p[c] as i32 - q[c] as i32).unsigned_abs() as u64)
                .sum::<u64>();
            n += 3;
        }
    }
    s as f64 / n as f64
}

#[test]
fn step_is_registered_with_texts_in_both_languages() {
    let reg = StepRegistry::global();
    let step = reg.get(ID).expect("registered");
    for lang in ["de", "en"] {
        fp_i18n::with_language(lang, || {
            let k = fp_batch::step::step_key(ID);
            assert!(fp_i18n::has(&k), "{lang}: {k}");
            assert!(
                fp_i18n::has(&format!("{k}.description")),
                "{lang}: description"
            );
            for p in step.params() {
                let pk = fp_batch::step::param_key(ID, &p.name);
                assert!(fp_i18n::has(&pk), "{lang}: {pk}");
            }
            assert!(fp_i18n::has("batch-warn-face-none"));
            assert!(!step.summary(&json!({})).is_empty());
        });
    }
    let en = fp_i18n::with_language("en", || fp_batch::step::step_label(ID));
    assert_eq!(en, "Blur faces");
    // defaults of the spec
    let d = fp_batch::step::with_defaults(&step.params(), &json!({}));
    assert_eq!(d["strength"], json!(60.0));
    assert_eq!(d["shape"], json!("ellipse"));
    assert_eq!(d["padding"], json!(20.0));
    assert_eq!(d["minSize"], json!(24));
    assert_eq!(d["pixelate"], json!(false));
    // out of range = error in the validation
    let issues = step.validate(&json!({"strength": 150}));
    assert!(
        issues.iter().any(|i| i.severity == Severity::Error),
        "{issues:?}"
    );
    // size stays: the dry run knows the output size
    assert_eq!(
        fp_batch::predict::step_size(ID, 300, 200, &d),
        Some((300, 200))
    );
}

#[test]
fn blurs_the_face_of_the_portrait_and_nothing_far_away() {
    let orig = portrait();
    let (o, w) = pixels(&orig);
    for pixelate in [false, true] {
        let mut it = orig.clone();
        let warn = apply(json!({"pixelate": pixelate}), &mut it).expect("apply");
        assert!(warn.is_empty(), "{warn:?}");
        let (n, _) = pixels(&it);
        // eyes, nose and mouth clearly changed
        for (x, y) in [(470, 760), (750, 835), (560, 980), (520, 1110)] {
            let d = diff(&o, &n, w, x, y, 12);
            assert!(
                d > 3.0,
                "pixelate {pixelate}: ({x}, {y}) changed only {d:.2}"
            );
        }
        // top corners (background, hair) and bottom left (shoulder) untouched
        for (x, y) in [(20, 20), (1250, 20), (40, 1440)] {
            assert_eq!(
                diff(&o, &n, w, x, y, 12),
                0.0,
                "pixelate {pixelate}: ({x}, {y})"
            );
        }
    }
}

#[test]
fn no_face_leaves_the_image_unchanged_and_says_so() {
    // flat grey picture
    let raw = vec![128u8; 200 * 150 * 4];
    let flat = still_from_rgba(200, 150, &raw).expect("item");
    let mut it = flat.clone();
    let warn = apply(json!({}), &mut it).expect("no error");
    assert_eq!(pixels(&it).0, pixels(&flat).0);
    assert_eq!(warn.len(), 1, "{warn:?}");
    fp_i18n::with_language("en", || {
        let mut it = flat.clone();
        let warn = apply(json!({}), &mut it).expect("no error");
        assert!(warn[0].contains("0 faces"), "{warn:?}");
    });
    // the portrait with a search area away from the face, or a minimum above the face size
    let orig = portrait();
    for p in [
        json!({"region": {"x": 0, "y": 0, "width": 1280, "height": 300}}),
        json!({"minSize": 1000}),
    ] {
        let mut it = orig.clone();
        let warn = apply(p.clone(), &mut it).expect("no error");
        assert_eq!(warn.len(), 1, "{p}: {warn:?}");
        assert!(pixels(&it).0 == pixels(&orig).0, "{p}: changed");
    }
}

#[test]
fn strength_zero_keeps_pixels_and_region_with_face_works() {
    let orig = portrait();
    let mut it = orig.clone();
    let warn = apply(json!({"strength": 0}), &mut it).expect("apply");
    assert!(warn.is_empty(), "face is still found: {warn:?}");
    assert!(pixels(&it).0 == pixels(&orig).0);
    // a region around the face finds it as well
    let mut it = orig.clone();
    let warn = apply(
        json!({"region": {"x": 100, "y": 400, "width": 1000, "height": 950}, "shape": "rect"}),
        &mut it,
    )
    .expect("apply");
    assert!(warn.is_empty(), "{warn:?}");
    let ((o, w), (n, _)) = (pixels(&orig), pixels(&it));
    assert!(diff(&o, &n, w, 560, 980, 12) > 3.0);
}

#[test]
fn cancel_is_reported_as_cancelled() {
    let cancel = CancelToken::new();
    cancel.cancel();
    let mut it = portrait();
    let e = apply_with(json!({}), &mut it, &cancel).expect_err("cancelled");
    assert!(matches!(e, fp_batch::BatchError::Cancelled(_)), "{e:?}");
}
