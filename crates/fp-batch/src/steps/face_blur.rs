//! `ai.faceBlur` – blur or pixelate faces (BA-109, `docs/BATCH.md` §3.4).
//!
//! Faces are found with the built-in frontal face detector of `fp-ai` ([`fp_ai::face`], Haar
//! cascade embedded in the executable – nothing to download, no model dialog). Every face box is
//! grown by `padding`, then covered with an ellipse or a rectangle with a soft inner edge and
//! blurred (radius = `strength` × half the face edge) or pixelated (block = `strength` × a sixth
//! of the face edge). `region` limits the search to a rectangle of the image. No face found is
//! not an error: the image stays unchanged and the report says so. Animations: every frame is
//! searched on its own.

use super::background::{gauss, p_bool, p_f64, p_str, preview_scale};
use crate::error::{BatchError, Result};
use crate::executor::CancelToken;
use crate::item::{Item, ItemKind};
use crate::step::{ParamDef, Step, StepContext, StepGroup, StepRegistry};
use fp_ai::face::FaceOptions;
use fp_raster::{IRect, RgbaImage};
use serde_json::Value;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// ID "Blur faces".
pub const FACE_BLUR: &str = "ai.faceBlur";

/// Registers the step.
pub fn register(r: &mut StepRegistry) {
    r.register(Arc::new(FaceBlur));
}

/// Search rectangle from `region` (`{"x", "y", "width", "height"}` in source pixels, scaled by
/// `scale`), clipped to the image; `None` = whole image.
fn region_rect(p: &Value, w: usize, h: usize, scale: f64) -> Option<IRect> {
    let r = p.get("region").filter(|v| v.is_object())?;
    let n = |k: &str| r.get(k).and_then(Value::as_f64).filter(|v| v.is_finite());
    let (x, y) = (n("x").unwrap_or(0.0), n("y").unwrap_or(0.0));
    let rw = n("width").or_else(|| n("w"))?;
    let rh = n("height").or_else(|| n("h"))?;
    let s = |v: f64| (v * scale).round() as i32;
    Some(
        IRect::from_xywh(s(x), s(y), s(rw), s(rh)).intersect(&IRect::new(0, 0, w as i32, h as i32)),
    )
}

/// Minimum-neighbour count of the detector for `sensitivity`.
fn neighbours(sensitivity: &str) -> usize {
    match sensitivity {
        "high" => 3,
        "low" => 8,
        _ => fp_ai::face::MIN_NEIGHBORS,
    }
}

/// Detects faces in `img` (local pixel coordinates), searching only in `region`. The batch
/// cancel token is forwarded to the detector by a small watcher thread.
pub fn find_faces(
    img: &RgbaImage,
    region: Option<IRect>,
    opt: &FaceOptions,
    cancel: &CancelToken,
) -> Result<Vec<IRect>> {
    cancel.check()?;
    let (w, h) = (img.width(), img.height());
    let area = region.unwrap_or(IRect::new(0, 0, w as i32, h as i32));
    if area.is_empty() {
        return Ok(Vec::new());
    }
    let (ax, ay, aw, ah) = (
        area.left as usize,
        area.top as usize,
        area.width() as usize,
        area.height() as usize,
    );
    let src = img.data();
    let px: Vec<[u8; 4]> = if (aw, ah) == (w, h) {
        src.to_vec()
    } else {
        (ay..ay + ah)
            .flat_map(|y| src[y * w + ax..y * w + ax + aw].iter().copied())
            .collect()
    };
    let ai_cancel = fp_ai::Cancel::new();
    let done = AtomicBool::new(false);
    let res = std::thread::scope(|s| {
        s.spawn(|| {
            while !done.load(Ordering::Relaxed) {
                if cancel.is_cancelled() {
                    ai_cancel.cancel();
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        });
        let r = fp_ai::face::detect_faces_with(&px, aw, ah, opt, &ai_cancel);
        done.store(true, Ordering::Relaxed);
        r
    });
    let faces = res.map_err(|e| match e {
        fp_ai::AiError::Cancelled => BatchError::cancelled(),
        e => BatchError::step(FACE_BLUR, e.to_string()),
    })?;
    Ok(faces
        .into_iter()
        .map(|r| {
            IRect::from_xywh(
                r.left + ax as i32,
                r.top + ay as i32,
                r.width() as i32,
                r.height() as i32,
            )
        })
        .collect())
}

/// How a face is covered.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cover {
    /// 0…1.
    pub strength: f64,
    /// Ellipse (else rectangle).
    pub ellipse: bool,
    /// Box growth per side as a fraction of the face edge.
    pub padding: f64,
    /// Mosaic instead of blur.
    pub pixelate: bool,
}

/// Grown face box (clipped to `w × h`).
pub fn padded(face: IRect, padding: f64, w: usize, h: usize) -> IRect {
    let (dx, dy) = (
        (face.width() as f64 * padding).round() as i32,
        (face.height() as f64 * padding).round() as i32,
    );
    face.inflate(dx, dy)
        .intersect(&IRect::new(0, 0, w as i32, h as i32))
}

/// Blurs or pixelates one face in `img` (local coordinates). `face` = detector box (unpadded).
pub fn cover_face(img: &mut RgbaImage, face: IRect, c: &Cover) {
    let (w, h) = (img.width(), img.height());
    if c.strength <= 0.0 || w == 0 || h == 0 {
        return;
    }
    // ellipse/rect geometry of the full padded box (may reach beyond the image)
    let (dx, dy) = (
        (face.width() as f64 * c.padding).round() as i32,
        (face.height() as f64 * c.padding).round() as i32,
    );
    let full = face.inflate(dx, dy);
    let bx = full.intersect(&IRect::new(0, 0, w as i32, h as i32));
    if bx.is_empty() {
        return;
    }
    let edge = face.width().min(face.height()) as f64;
    // patch with context around the box (the blur reads beyond the box edge)
    let radius = (c.strength * edge * 0.5).max(1.0);
    let ctx_px = if c.pixelate { 0 } else { radius.ceil() as i32 };
    let patch = bx
        .inflate(ctx_px, ctx_px)
        .intersect(&IRect::new(0, 0, w as i32, h as i32));
    let (pw, ph) = (patch.width() as usize, patch.height() as usize);
    let (pl, pt) = (patch.left as usize, patch.top as usize);
    let data = img.data().to_vec();
    // premultiplied 0…1 copy of the patch
    let mut buf: Vec<f32> = Vec::with_capacity(pw * ph * 4);
    for y in pt..pt + ph {
        for p in &data[y * w + pl..y * w + pl + pw] {
            let a = p[3] as f32 / 255.0;
            buf.extend_from_slice(&[
                p[0] as f32 / 255.0 * a,
                p[1] as f32 / 255.0 * a,
                p[2] as f32 / 255.0 * a,
                a,
            ]);
        }
    }
    if c.pixelate {
        let block = ((c.strength * edge / 6.0).round() as usize).max(2);
        // blocks aligned to the padded box (patch == box here)
        for by in (0..ph).step_by(block) {
            for bxx in (0..pw).step_by(block) {
                let (ex, ey) = ((bxx + block).min(pw), (by + block).min(ph));
                let mut s = [0f64; 4];
                for y in by..ey {
                    for x in bxx..ex {
                        for (k, v) in s.iter_mut().enumerate() {
                            *v += buf[(y * pw + x) * 4 + k] as f64;
                        }
                    }
                }
                let n = ((ex - bxx) * (ey - by)) as f64;
                for y in by..ey {
                    for x in bxx..ex {
                        for (k, v) in s.iter().enumerate() {
                            buf[(y * pw + x) * 4 + k] = (v / n) as f32;
                        }
                    }
                }
            }
        }
    } else {
        gauss(&mut buf, pw, ph, 4, radius as f32, true);
    }
    // soft inner edge: a few percent of the face edge
    let feather = (edge * 0.08).max(1.5);
    let (cx, cy) = (
        (full.left + full.right) as f64 / 2.0,
        (full.top + full.bottom) as f64 / 2.0,
    );
    let (rx, ry) = (full.width() as f64 / 2.0, full.height() as f64 / 2.0);
    let mut out = data;
    for y in bx.top..bx.bottom {
        for x in bx.left..bx.right {
            let (fx, fy) = (x as f64 + 0.5, y as f64 + 0.5);
            // signed distance to the shape edge (positive inside), approximate for the ellipse
            let sd = if c.ellipse {
                let d = (((fx - cx) / rx).powi(2) + ((fy - cy) / ry).powi(2)).sqrt();
                (1.0 - d) * rx.min(ry)
            } else {
                (fx - full.left as f64)
                    .min(full.right as f64 - fx)
                    .min(fy - full.top as f64)
                    .min(full.bottom as f64 - fy)
            };
            let t = (sd / feather).clamp(0.0, 1.0) as f32;
            if t <= 0.0 {
                continue;
            }
            let (ux, uy) = (x as usize, y as usize);
            let o = out[uy * w + ux];
            let bi = ((uy - pt) * pw + (ux - pl)) * 4;
            let a0 = o[3] as f32 / 255.0;
            let src = [
                o[0] as f32 / 255.0 * a0,
                o[1] as f32 / 255.0 * a0,
                o[2] as f32 / 255.0 * a0,
                a0,
            ];
            let m: [f32; 4] = std::array::from_fn(|k| src[k] + (buf[bi + k] - src[k]) * t);
            out[uy * w + ux] = if m[3] <= 1e-6 {
                [0, 0, 0, 0]
            } else {
                let q = |v: f32| ((v / m[3]).clamp(0.0, 1.0) * 255.0).round() as u8;
                [
                    q(m[0]),
                    q(m[1]),
                    q(m[2]),
                    (m[3].clamp(0.0, 1.0) * 255.0).round() as u8,
                ]
            };
        }
    }
    if let Ok(new) = RgbaImage::from_raw(IRect::new(0, 0, w as i32, h as i32), out) {
        *img = new;
    }
}

struct FaceBlur;

impl Step for FaceBlur {
    fn id(&self) -> &str {
        FACE_BLUR
    }
    fn group(&self) -> StepGroup {
        StepGroup::Color
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::number("strength", 0.0, 100.0, 60.0)
                .unit("%")
                .beginner(),
            ParamDef::choice("shape", &["ellipse", "rect"], "ellipse").beginner(),
            ParamDef::bool("pixelate", false).beginner(),
            ParamDef::number("padding", 0.0, 100.0, 20.0).unit("%"),
            ParamDef::int("minSize", 24, 4096, 24).unit("px"),
            ParamDef::choice("sensitivity", &["low", "normal", "high"], "normal"),
            ParamDef::json("region", Value::Null),
        ]
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let (w, h) = item.dimensions();
        let scale = preview_scale(ctx, w as usize, h as usize);
        let cover = Cover {
            strength: p_f64(params, "strength", 60.0).clamp(0.0, 100.0) / 100.0,
            ellipse: p_str(params, "shape", "ellipse") != "rect",
            padding: p_f64(params, "padding", 20.0).clamp(0.0, 100.0) / 100.0,
            pixelate: p_bool(params, "pixelate", false),
        };
        let min_size = p_f64(params, "minSize", 24.0).clamp(24.0, 4096.0);
        let opt = FaceOptions {
            // the proxy of the preview is smaller: scale the minimum along (the 24 px window
            // stays the floor of the detector)
            min_size: (min_size * scale).round().max(1.0) as u32,
            min_neighbors: neighbours(p_str(params, "sensitivity", "normal")),
        };
        let cancel = ctx.cancel;
        let mut total = 0usize;
        item.map_images(|img| {
            let region = region_rect(params, img.width(), img.height(), scale);
            let faces = find_faces(img, region, &opt, cancel)?;
            total += faces.len();
            for f in faces {
                cancel.check()?;
                cover_face(img, f, &cover);
            }
            Ok(())
        })?;
        if total == 0 {
            ctx.warn(fp_i18n::t("batch-warn-face-none"));
        }
        Ok(())
    }
    fn summary(&self, params: &Value) -> String {
        let mode = if p_bool(params, "pixelate", false) {
            "pixelate"
        } else {
            "blur"
        };
        fp_i18n::tr!(
            "batch-face-summary",
            mode = fp_i18n::t(&format!("batch-face-summary-{mode}")),
            strength = p_f64(params, "strength", 60.0).round() as i64
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn region_is_scaled_and_clipped() {
        let p = json!({"region": {"x": 10, "y": 20, "width": 500, "height": 40}});
        assert_eq!(
            region_rect(&p, 100, 100, 1.0),
            Some(IRect::new(10, 20, 100, 60))
        );
        assert_eq!(
            region_rect(&p, 100, 100, 0.5),
            Some(IRect::new(5, 10, 100, 30))
        );
        assert_eq!(region_rect(&json!({"region": null}), 100, 100, 1.0), None);
        assert_eq!(region_rect(&json!({}), 100, 100, 1.0), None);
    }

    #[test]
    fn padding_grows_and_clips() {
        let f = IRect::from_xywh(10, 10, 50, 50);
        assert_eq!(padded(f, 0.2, 1000, 1000), IRect::new(0, 0, 70, 70));
        assert_eq!(padded(f, 0.0, 1000, 1000), f);
    }

    fn img(w: usize, h: usize, f: impl Fn(usize, usize) -> [u8; 4]) -> RgbaImage {
        let d = (0..h)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .map(|(x, y)| f(x, y))
            .collect();
        RgbaImage::from_raw(IRect::new(0, 0, w as i32, h as i32), d).expect("image")
    }

    #[test]
    fn cover_changes_only_the_face_area() {
        // stripes: blurring visibly changes them
        let mk = || {
            img(120, 100, |x, _| {
                if x % 4 < 2 {
                    [0, 0, 0, 255]
                } else {
                    [255, 255, 255, 255]
                }
            })
        };
        let face = IRect::from_xywh(40, 30, 40, 40);
        for (ellipse, pixelate) in [(true, false), (false, false), (true, true), (false, true)] {
            let mut a = mk();
            let c = Cover {
                strength: 0.6,
                ellipse,
                padding: 0.2,
                pixelate,
            };
            cover_face(&mut a, face, &c);
            let orig = mk();
            let (d0, d1) = (orig.data(), a.data());
            let at = |x: usize, y: usize| (d0[y * 120 + x], d1[y * 120 + x]);
            // centre changed, far corners untouched
            let (o, n) = at(61, 50);
            assert_ne!(o, n, "ellipse {ellipse} pixelate {pixelate}");
            for (x, y) in [(0, 0), (119, 99), (10, 50), (110, 50)] {
                let (o, n) = at(x, y);
                assert_eq!(o, n, "({x}, {y}) ellipse {ellipse} pixelate {pixelate}");
            }
            // box corner: inside the rect, outside the ellipse
            let (o, n) = at(33, 23);
            assert_eq!(
                o == n,
                ellipse,
                "corner, ellipse {ellipse} pixelate {pixelate}"
            );
        }
        // strength 0 = unchanged
        let mut a = mk();
        cover_face(
            &mut a,
            face,
            &Cover {
                strength: 0.0,
                ellipse: true,
                padding: 0.2,
                pixelate: false,
            },
        );
        assert_eq!(a, mk());
    }
}
