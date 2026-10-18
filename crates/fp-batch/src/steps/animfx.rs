//! Animation steps from the batch spec (`docs/BATCH.md` §3.8):
//!
//! - `anim.boomerang` – forward, then backward without repeating the turning frames
//!   (`0 1 2 3 2 1`), optional hold at both ends.
//! - `anim.crossfade` – seamless loop: either the last frames fade into the first ones
//!   (`overlap`, the animation gets shorter) or fade frames are appended (`append`).
//!
//! `anim.stabilize` (calm background) already exists as a step of the GIF optimizer
//! (`steps::gifopt`).

use crate::error::{BatchError, Result};
use crate::item::{Frame, Item, ItemKind};
use crate::step::{ParamDef, Step, StepContext, StepGroup, StepRegistry};
use fp_raster::{IRect, RgbaImage};
use serde_json::Value;
use std::sync::Arc;

/// ID "Boomerang".
pub const BOOMERANG: &str = "anim.boomerang";
/// ID "Cross-fade loop".
pub const CROSSFADE: &str = "anim.crossfade";

/// Registers the steps of this file.
pub fn register(r: &mut StepRegistry) {
    r.register(Arc::new(Boomerang));
    r.register(Arc::new(Crossfade));
}

fn p_int(p: &Value, k: &str, d: i64) -> i64 {
    p.get(k)
        .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f.round() as i64)))
        .unwrap_or(d)
}
fn p_str<'a>(p: &'a Value, k: &str, d: &'a str) -> &'a str {
    p.get(k).and_then(Value::as_str).unwrap_or(d)
}

fn empty() -> BatchError {
    BatchError::Invalid(fp_i18n::t("batch-anim-err-empty"))
}

/// Boomerang order of `n` frames: `0 … n-1 … 1`.
pub fn boomerang_order(n: usize) -> Vec<usize> {
    let mut v: Vec<usize> = (0..n).collect();
    if n > 2 {
        v.extend((1..n - 1).rev());
    }
    v
}

/// Mix of two equally sized images, `t` = share of `b` (premultiplied alpha).
pub fn mix(a: &RgbaImage, b: &RgbaImage, t: f32) -> Result<RgbaImage> {
    if a.width() != b.width() || a.height() != b.height() {
        return Err(BatchError::Invalid(fp_i18n::t("batch-err-frame-size")));
    }
    let px: Vec<[u8; 4]> = a
        .data()
        .iter()
        .zip(b.data())
        .map(|(p, q)| {
            let (pa, qa) = (p[3] as f32 / 255.0, q[3] as f32 / 255.0);
            let al = pa * (1.0 - t) + qa * t;
            let mut o = [0u8; 4];
            for c in 0..3 {
                let v = p[c] as f32 * pa * (1.0 - t) + q[c] as f32 * qa * t;
                o[c] = if al > 0.0 {
                    (v / al).round().clamp(0.0, 255.0) as u8
                } else {
                    0
                };
            }
            o[3] = (al * 255.0).round().clamp(0.0, 255.0) as u8;
            o
        })
        .collect();
    RgbaImage::from_raw(IRect::new(0, 0, a.width() as i32, a.height() as i32), px)
        .map_err(|e| BatchError::Decode(e.to_string()))
}

struct Boomerang;

impl Step for Boomerang {
    fn id(&self) -> &str {
        BOOMERANG
    }
    fn group(&self) -> StepGroup {
        StepGroup::Animation
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![ParamDef::int("hold", 0, 10_000, 0).unit("ms").beginner()]
    }
    fn applies_to(&self) -> &[ItemKind] {
        &[ItemKind::Animation]
    }
    fn apply(&self, item: &mut Item, params: &Value, _ctx: &mut StepContext) -> Result<()> {
        let Item::Animation(a) = item else {
            return Ok(());
        };
        if a.frames.is_empty() {
            return Err(empty());
        }
        let n = a.frames.len();
        let hold = p_int(params, "hold", 0).clamp(0, 10_000) as u32;
        let mut frames: Vec<Frame> = boomerang_order(n)
            .into_iter()
            .map(|i| a.frames[i].clone())
            .collect();
        if hold > 0 && n > 1 {
            frames[0].delay_ms = frames[0].delay_ms.saturating_add(hold);
            frames[n - 1].delay_ms = frames[n - 1].delay_ms.saturating_add(hold);
        }
        a.frames = frames;
        Ok(())
    }
}

/// Cross-fade loop. `overlap`: the last `n` frames fade into the first `n` (frame `N-n+i` gets
/// `(i+1)/(n+1)` of frame `i`), the first `n` frames are dropped, so the end flows into the new
/// start; needs at least `2n + 1` frames. `append`: `n` fade frames from the last to the first
/// frame are appended with `delay` ms each (0 = mean delay).
pub fn crossfade(frames: Vec<Frame>, n: usize, append: bool, delay: u32) -> Result<Vec<Frame>> {
    let total = frames.len();
    if n == 0 || total < 2 {
        return Ok(frames);
    }
    if append {
        let mean = (frames.iter().map(|f| f.delay_ms as u64).sum::<u64>() / total as u64) as u32;
        let d = if delay > 0 { delay } else { mean.max(10) };
        let (first, last) = (&frames[0].image, &frames[total - 1].image);
        let mut extra = Vec::with_capacity(n);
        for i in 0..n {
            let t = (i + 1) as f32 / (n + 1) as f32;
            extra.push(Frame {
                image: mix(last, first, t)?,
                delay_ms: d,
            });
        }
        let mut out = frames;
        out.extend(extra);
        return Ok(out);
    }
    if total < 2 * n + 1 {
        return Err(BatchError::Invalid(fp_i18n::tr!(
            "batch-err-crossfade-short",
            frames = total as i64,
            need = (2 * n + 1) as i64
        )));
    }
    let mut out: Vec<Frame> = frames[n..].to_vec();
    let len = out.len();
    for (i, head) in frames.iter().take(n).enumerate() {
        let t = (i + 1) as f32 / (n + 1) as f32;
        let k = len - n + i;
        out[k].image = mix(&out[k].image, &head.image, t)?;
    }
    Ok(out)
}

struct Crossfade;

impl Step for Crossfade {
    fn id(&self) -> &str {
        CROSSFADE
    }
    fn group(&self) -> StepGroup {
        StepGroup::Animation
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::int("frames", 1, 60, 5).beginner(),
            ParamDef::choice("mode", &["overlap", "append"], "overlap").beginner(),
            ParamDef::int("delay", 0, 10_000, 0)
                .unit("ms")
                .when("mode", &["append"]),
        ]
    }
    fn applies_to(&self) -> &[ItemKind] {
        &[ItemKind::Animation]
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let Item::Animation(a) = item else {
            return Ok(());
        };
        ctx.check_cancel()?;
        let n = p_int(params, "frames", 5).clamp(1, 60) as usize;
        let append = p_str(params, "mode", "overlap") == "append";
        let delay = p_int(params, "delay", 0).clamp(0, 10_000) as u32;
        let frames = std::mem::take(&mut a.frames);
        a.frames = crossfade(frames, n, append, delay)?;
        if a.frames.is_empty() {
            return Err(empty());
        }
        Ok(())
    }
}
