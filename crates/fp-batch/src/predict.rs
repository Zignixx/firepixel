//! Output size before processing (dry run with the size token, BA-105): walks the enabled steps
//! of a recipe and computes the size each one produces when that follows from its parameters
//! alone. Steps whose result depends on the pixels (trim, subject centring, free rotation, any
//! registry filter or script …) make the size unknown – the dry run then marks the name as
//! "size after processing".

use crate::item::ItemInfo;
use crate::recipe::Recipe;
use crate::step::{StepRegistry, with_defaults};
use crate::steps::geometry::{canvas_geometry, crop_rect, px_round, resize_target};
use serde_json::Value;

/// Steps that never change the pixel size.
const SIZE_NEUTRAL: &[&str] = &[
    "adjust.auto",
    "adjust.basic",
    "adjust.levels",
    "adjust.curves",
    "adjust.grayscale",
    "adjust.invert",
    "adjust.posterize",
    "adjust.threshold",
    "filter.sharpen",
    "filter.blur",
    "filter.pixelate",
    "filter.screentone",
    "ai.faceBlur",
    "metadata.edit",
    "metadata.set",
    "background.remove",
    "background.removeColor",
    "background.flatten",
    "background.replace",
    "background.alphaCleanup",
    "overlay.text",
    "overlay.image",
    "overlay.vignette",
    "overlay.roundCorners",
    "optimize.png",
    "optimize.quantize",
    "anim.trim",
    "anim.speed",
    "anim.loop",
    "anim.dropFrames",
    "anim.frameDelay",
    "anim.firstFrame",
    "file.copyOnly",
];

fn s<'a>(p: &'a Value, k: &str, d: &'a str) -> &'a str {
    p.get(k).and_then(Value::as_str).unwrap_or(d)
}
fn f(p: &Value, k: &str, d: f64) -> f64 {
    p.get(k)
        .and_then(Value::as_f64)
        .filter(|v| v.is_finite())
        .unwrap_or(d)
}
fn b(p: &Value, k: &str, d: bool) -> bool {
    p.get(k).and_then(Value::as_bool).unwrap_or(d)
}

/// Size after one step (`None` = depends on the pixels / unknown).
pub fn step_size(id: &str, w: u32, h: u32, p: &Value) -> Option<(u32, u32)> {
    if SIZE_NEUTRAL.contains(&id) {
        return Some((w, h));
    }
    let (wi, hi) = (w as i64, h as i64);
    let clamp = |x: i64| x.clamp(1, u32::MAX as i64) as u32;
    match id {
        "resize" => {
            let Some((tw, th, fill)) = resize_target(w as f64, h as f64, p) else {
                return Some((w, h));
            };
            let (nw, nh) = (px_round(tw), px_round(th));
            let (nw, nh) = match fill {
                Some((fw, fh)) => (px_round(fw).min(nw), px_round(fh).min(nh)),
                None => (nw, nh),
            };
            Some((clamp(nw), clamp(nh)))
        }
        "crop" => {
            if b(p, "smart", false) {
                return None;
            }
            match crop_rect(wi, hi, p, 1.0) {
                Some(r) if !r.is_empty() => Some((r.width(), r.height())),
                _ => Some((w, h)),
            }
        }
        "canvas" => match canvas_geometry(wi, hi, p, 1.0) {
            Some((nw, nh, _, _)) => Some((clamp(nw), clamp(nh))),
            None => Some((w, h)),
        },
        "canvas.pad" => {
            let m = (f(p, "multiple", 16.0).round() as u32).clamp(2, 256);
            Some((w.div_ceil(m) * m, h.div_ceil(m) * m))
        }
        "rotate" => {
            let angle = f(p, "angle", 90.0).rem_euclid(360.0);
            let quarter = (angle / 90.0).round();
            if (angle - quarter * 90.0).abs() > 1e-9 {
                return None;
            }
            Some(if quarter as i64 % 2 == 1 {
                (h, w)
            } else {
                (w, h)
            })
        }
        "overlay.border" => {
            let width = f(p, "width", 20.0).max(0.0);
            if s(p, "placement", "outer") == "inner" || width == 0.0 {
                return Some((w, h));
            }
            let bw = if s(p, "unit", "px") == "percent" {
                (width.min(50.0) / 100.0 * w.min(h) as f64).round()
            } else {
                width.round()
            } as u32;
            Some((w + 2 * bw, h + 2 * bw))
        }
        "background.shadow" | "effect.live" if !b(p, "expandCanvas", true) => Some((w, h)),
        "ai.upscale" => {
            let s = crate::steps::upscale::scale_param(p) as i64;
            Some((clamp(wi * s), clamp(hi * s)))
        }
        "optimize.targetSize" if s(p, "strategy", "quality") == "quality" => Some((w, h)),
        _ => None,
    }
}

/// Predicted output size of `info` after all enabled steps of `recipe` (`None` = only known
/// after processing). Sources with an EXIF rotation by 90° are unknown too (applied on load).
pub fn output_size(recipe: &Recipe, info: &ItemInfo, reg: &StepRegistry) -> Option<(u32, u32)> {
    if info
        .exif
        .as_ref()
        .and_then(|e| e.orientation)
        .is_some_and(|o| (5..=8).contains(&o))
    {
        return None;
    }
    let (mut w, mut h) = (info.width, info.height);
    if w == 0 || h == 0 {
        return None;
    }
    for cfg in recipe.steps.iter().filter(|c| c.enabled) {
        // unknown steps are skipped by the pipeline as well
        let Some(step) = reg.get(&cfg.id) else {
            continue;
        };
        let params = with_defaults(&step.params(), &cfg.params);
        (w, h) = step_size(&cfg.id, w, h, &params)?;
    }
    Some((w, h))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn size(id: &str, w: u32, h: u32, p: Value) -> Option<(u32, u32)> {
        let reg = StepRegistry::global();
        let step = reg.get(id).expect("step");
        step_size(id, w, h, &with_defaults(&step.params(), &p))
    }

    #[test]
    fn sizes_from_parameters() {
        assert_eq!(size("resize", 4000, 3000, json!({})), Some((1600, 1200)));
        assert_eq!(
            size("resize", 800, 600, json!({})),
            Some((800, 600)),
            "only shrink"
        );
        assert_eq!(
            size(
                "resize",
                400,
                300,
                json!({"mode": "percent", "percent": 50})
            ),
            Some((200, 150))
        );
        assert_eq!(
            size(
                "resize",
                400,
                200,
                json!({"mode": "fill", "width": 100, "height": 100})
            ),
            Some((100, 100))
        );
        assert_eq!(
            size("crop", 400, 200, json!({"ratio": "1:1"})),
            Some((200, 200))
        );
        assert_eq!(
            size("canvas", 10, 20, json!({"mode": "square"})),
            Some((20, 20))
        );
        assert_eq!(
            size("canvas.pad", 13, 16, json!({"multiple": 8})),
            Some((16, 16))
        );
        assert_eq!(size("rotate", 10, 20, json!({"angle": 90})), Some((20, 10)));
        assert_eq!(size("rotate", 10, 20, json!({"angle": 45})), None);
        assert_eq!(
            size("overlay.border", 10, 20, json!({"width": 3})),
            Some((16, 26))
        );
        assert_eq!(size("adjust.invert", 10, 20, json!({})), Some((10, 20)));
        assert_eq!(size("trim", 10, 20, json!({})), None);
        assert_eq!(size("background.subjectCenter", 10, 20, json!({})), None);
        assert_eq!(size("ai.upscale", 10, 20, json!({})), Some((40, 80)));
        assert_eq!(
            size("ai.upscale", 10, 20, json!({"scale": "2"})),
            Some((20, 40))
        );
    }
}
