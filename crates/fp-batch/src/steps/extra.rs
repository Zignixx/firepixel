//! Additional steps from the batch spec that were still missing (BA-109, `docs/BATCH.md` §3):
//!
//! - `background.subjectCenter` – find the subject (alpha or "differs from the corner colour"),
//!   crop to its box and centre it on a new canvas with a margin (optionally square, optionally
//!   with a soft shadow below the subject).
//! - `background.alphaCleanup` – snap almost transparent / almost opaque alpha, remove small
//!   islands and defringe semi-transparent edge pixels.
//! - `overlay.roundCorners` – anti-aliased rounded corners (px or % of the shorter side).
//! - `overlay.vignette` – darken (or tint) the edges.
//! - `adjust.posterize`, `adjust.threshold` – tonal reduction.
//! - `filter.pixelate` – block mosaic.
//! - `canvas.pad` – grow the canvas to a multiple of n pixels.
//! - `anim.firstFrame` – still image from an animation (first/middle/last/number/time/sharpest).
//!
//! Animations: every pixel step acts on each frame; `background.subjectCenter` uses the union
//! of the subject boxes of all frames so the subject does not jump.

use super::background::{over, p_bool, p_color, p_f64, p_str, preview_scale, shadow_image, to_f};
use super::overlay::round_rect_cov;
use crate::error::{BatchError, Result};
use crate::item::{Item, ItemKind, Still};
use crate::step::{Issue, ParamDef, Step, StepContext, StepGroup, StepRegistry, validate_params};
use fp_raster::{IRect, RgbaImage};
use rayon::prelude::*;
use serde_json::{Value, json};
use std::sync::Arc;

/// ID "Center subject".
pub const SUBJECT_CENTER: &str = "background.subjectCenter";
/// ID "Clean up alpha".
pub const ALPHA_CLEANUP: &str = "background.alphaCleanup";
/// ID "Round corners".
pub const ROUND_CORNERS: &str = "overlay.roundCorners";
/// ID "Vignette".
pub const VIGNETTE: &str = "overlay.vignette";
/// ID "Posterize".
pub const POSTERIZE: &str = "adjust.posterize";
/// ID "Threshold".
pub const THRESHOLD: &str = "adjust.threshold";
/// ID "Pixelate".
pub const PIXELATE: &str = "filter.pixelate";
/// ID "Pad to multiple".
pub const CANVAS_PAD: &str = "canvas.pad";
/// ID "Still from animation".
pub const FIRST_FRAME: &str = "anim.firstFrame";

/// Registers the steps of this file.
pub fn register(r: &mut StepRegistry) {
    r.register(Arc::new(SubjectCenter));
    r.register(Arc::new(AlphaCleanup));
    r.register(Arc::new(RoundCorners));
    r.register(Arc::new(Vignette));
    r.register(Arc::new(Posterize));
    r.register(Arc::new(Threshold));
    r.register(Arc::new(Pixelate));
    r.register(Arc::new(CanvasPad));
    r.register(Arc::new(FirstFrame));
}

fn p_int(p: &Value, k: &str, d: i64) -> i64 {
    p.get(k)
        .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f.round() as i64)))
        .unwrap_or(d)
}

fn image(w: usize, h: usize, data: Vec<[u8; 4]>) -> Result<RgbaImage> {
    RgbaImage::from_raw(IRect::new(0, 0, w as i32, h as i32), data)
        .map_err(|e| BatchError::Decode(e.to_string()))
}

/// All pixel images of an item (documents must be flattened before).
fn images(item: &Item) -> Vec<&RgbaImage> {
    match item {
        Item::Still(s) => vec![&s.image],
        Item::Animation(a) => a.frames.iter().map(|f| &f.image).collect(),
        _ => Vec::new(),
    }
}

// ------------------------------------------------------------------ Center subject

/// Subject box `(x0, y0, x1, y1)` (exclusive) of one image: pixels with alpha > 8 (`alpha`)
/// or whose largest channel difference to `key` exceeds `tol` (`color`).
pub fn subject_box(
    img: &RgbaImage,
    by_alpha: bool,
    key: [u8; 4],
    tol: u8,
) -> Option<(usize, usize, usize, usize)> {
    let (w, h) = (img.width(), img.height());
    let data = img.data();
    let (mut x0, mut y0, mut x1, mut y1) = (usize::MAX, usize::MAX, 0usize, 0usize);
    for y in 0..h {
        for x in 0..w {
            let p = data[y * w + x];
            let hit = if by_alpha {
                p[3] > 8
            } else {
                p[3] > 8
                    && (0..3)
                        .map(|i| (p[i] as i16 - key[i] as i16).unsigned_abs())
                        .max()
                        .unwrap_or(0)
                        > tol as u16
            };
            if hit {
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x + 1);
                y1 = y1.max(y + 1);
            }
        }
    }
    (x1 > x0 && y1 > y0).then_some((x0, y0, x1, y1))
}

struct SubjectCenter;

impl Step for SubjectCenter {
    fn id(&self) -> &str {
        SUBJECT_CENTER
    }
    fn group(&self) -> StepGroup {
        StepGroup::Background
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::number("margin", 0.0, 50.0, 10.0)
                .unit("%")
                .beginner(),
            ParamDef::bool("square", false).beginner(),
            ParamDef::bool("shadow", false),
            ParamDef::choice("detect", &["auto", "alpha", "color"], "auto"),
            ParamDef::int("tolerance", 0, 255, 24).when("detect", &["auto", "color"]),
            ParamDef::choice("fill", &["auto", "transparent", "color"], "auto"),
            ParamDef::color("color", "#ffffff").when("fill", &["color"]),
        ]
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        item.flatten_document()?;
        let imgs = images(item);
        let Some(first) = imgs.first() else {
            return Ok(());
        };
        let key = first.get(0, 0).unwrap_or([255, 255, 255, 255]);
        let by_alpha = match p_str(params, "detect", "auto") {
            "alpha" => true,
            "color" => false,
            _ => imgs.iter().any(|i| i.data().iter().any(|p| p[3] < 250)),
        };
        let tol = p_int(params, "tolerance", 24).clamp(0, 255) as u8;
        let mut union: Option<(usize, usize, usize, usize)> = None;
        for img in &imgs {
            if let Some(b) = subject_box(img, by_alpha, key, tol) {
                union = Some(match union {
                    None => b,
                    Some(u) => (u.0.min(b.0), u.1.min(b.1), u.2.max(b.2), u.3.max(b.3)),
                });
            }
        }
        let Some((x0, y0, x1, y1)) = union else {
            ctx.warn(fp_i18n::t("batch-warn-subject-none"));
            return Ok(());
        };
        let (bw, bh) = (x1 - x0, y1 - y0);
        let margin = p_f64(params, "margin", 10.0).clamp(0.0, 50.0) / 100.0;
        let m = (margin * bw.max(bh) as f64).round() as usize;
        let (mut nw, mut nh) = (bw + 2 * m, bh + 2 * m);
        if p_bool(params, "square", false) {
            nw = nw.max(nh);
            nh = nw;
        }
        let fill = match p_str(params, "fill", "auto") {
            "transparent" => [0, 0, 0, 0],
            "color" => p_color(params, "color", [255, 255, 255, 255]),
            _ if by_alpha => [0, 0, 0, 0],
            _ => key,
        };
        let shadow = p_bool(params, "shadow", false);
        let (ox, oy) = ((nw - bw) / 2, (nh - bh) / 2);
        let scale = preview_scale(ctx, nw, nh);
        let cancel = ctx.cancel;
        item.map_images(|img| {
            cancel.check()?;
            let (w, src) = (img.width(), img.data());
            let mut layer = vec![[0u8; 4]; nw * nh];
            for y in 0..bh {
                let s = (y0 + y) * w + x0;
                let d = (oy + y) * nw + ox;
                layer[d..d + bw].copy_from_slice(&src[s..s + bw]);
            }
            let mut layer = image(nw, nh, layer)?;
            if shadow {
                layer = shadow_image(
                    &layer,
                    &json!({"style": "below", "expandCanvas": false}),
                    scale,
                );
            }
            let bg = to_f(fill);
            let out: Vec<[u8; 4]> = layer
                .data()
                .iter()
                .map(|p| super::background::to_u8(over(to_f(*p), bg, false)))
                .collect();
            *img = image(nw, nh, out)?;
            Ok(())
        })?;
        if let Some(meta) = item.meta_mut() {
            meta.has_transparency = fill[3] < 255 || meta.has_transparency;
        }
        Ok(())
    }
}

// ------------------------------------------------------------------ Clean up alpha

/// Alpha cleanup on one image: alpha `< low` → 0, `> high` → 255; opaque islands (8-connected,
/// alpha > 0) smaller than `min_island` pixels are removed; `defringe` passes copy the colour of
/// opaque neighbours into semi-transparent edge pixels.
pub fn alpha_cleanup(img: &mut RgbaImage, low: u8, high: u8, min_island: usize, defringe: usize) {
    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 {
        return;
    }
    let mut px = img.data().to_vec();
    for p in &mut px {
        if p[3] < low {
            *p = [0, 0, 0, 0];
        } else if p[3] > high {
            p[3] = 255;
        }
    }
    if min_island > 1 {
        let mut seen = vec![false; w * h];
        let mut stack = Vec::new();
        let mut comp = Vec::new();
        for start in 0..w * h {
            if seen[start] || px[start][3] == 0 {
                continue;
            }
            seen[start] = true;
            stack.push(start);
            comp.clear();
            while let Some(i) = stack.pop() {
                comp.push(i);
                let (x, y) = ((i % w) as i64, (i / w) as i64);
                for dy in -1..=1i64 {
                    for dx in -1..=1i64 {
                        let (nx, ny) = (x + dx, y + dy);
                        if nx < 0 || ny < 0 || nx >= w as i64 || ny >= h as i64 {
                            continue;
                        }
                        let j = ny as usize * w + nx as usize;
                        if !seen[j] && px[j][3] > 0 {
                            seen[j] = true;
                            stack.push(j);
                        }
                    }
                }
            }
            if comp.len() < min_island {
                for &i in &comp {
                    px[i] = [0, 0, 0, 0];
                }
            }
        }
    }
    let mut solid: Vec<bool> = px.iter().map(|p| p[3] == 255).collect();
    for _ in 0..defringe {
        let mut next = solid.clone();
        let mut out = px.clone();
        for i in 0..w * h {
            if solid[i] || px[i][3] == 0 {
                continue;
            }
            let (x, y) = ((i % w) as i64, (i / w) as i64);
            let (mut sum, mut n) = ([0u32; 3], 0u32);
            for dy in -1..=1i64 {
                for dx in -1..=1i64 {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx < 0 || ny < 0 || nx >= w as i64 || ny >= h as i64 {
                        continue;
                    }
                    let j = ny as usize * w + nx as usize;
                    if solid[j] {
                        for c in 0..3 {
                            sum[c] += px[j][c] as u32;
                        }
                        n += 1;
                    }
                }
            }
            if let Some(n) = std::num::NonZeroU32::new(n) {
                for c in 0..3 {
                    out[i][c] = ((sum[c] + n.get() / 2) / n) as u8;
                }
                next[i] = true;
            }
        }
        px = out;
        solid = next;
    }
    if let Ok(new) = RgbaImage::from_raw(IRect::new(0, 0, w as i32, h as i32), px) {
        *img = new;
    }
}

struct AlphaCleanup;

impl Step for AlphaCleanup {
    fn id(&self) -> &str {
        ALPHA_CLEANUP
    }
    fn group(&self) -> StepGroup {
        StepGroup::Background
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::int("low", 0, 254, 8).beginner(),
            ParamDef::int("high", 1, 255, 247).beginner(),
            ParamDef::int("minIsland", 0, 1_000_000, 0).unit("px"),
            ParamDef::int("defringe", 0, 5, 0).unit("px"),
        ]
    }
    fn validate(&self, params: &Value) -> Vec<Issue> {
        let mut v = validate_params(&self.params(), params);
        let (lo, hi) = (p_int(params, "low", 8), p_int(params, "high", 247));
        if lo >= hi {
            v.push(Issue::param("high", fp_i18n::t("batch-err-alpha-range")));
        }
        v
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let low = p_int(params, "low", 8).clamp(0, 254) as u8;
        let high = p_int(params, "high", 247).clamp(1, 255) as u8;
        let (w, h) = item.dimensions();
        let scale = preview_scale(ctx, w as usize, h as usize);
        let island = (p_int(params, "minIsland", 0).clamp(0, 1_000_000) as f64 * scale * scale)
            .round() as usize;
        let defringe = (p_int(params, "defringe", 0).clamp(0, 5) as f64 * scale).ceil() as usize;
        let cancel = ctx.cancel;
        item.map_images(|img| {
            cancel.check()?;
            alpha_cleanup(img, low, high, island, defringe);
            Ok(())
        })
    }
}

// ------------------------------------------------------------------ Round corners

/// Rounded corners with radius `r` px (anti-aliased, transparent outside).
pub fn round_corners(img: &mut RgbaImage, r: f64) {
    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 || r <= 0.0 {
        return;
    }
    let mut px = img.data().to_vec();
    px.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
        for (x, p) in row.iter_mut().enumerate() {
            let c = round_rect_cov(x as f64, y as f64, 0.0, 0.0, w as f64, h as f64, r);
            if c < 1.0 {
                p[3] = (p[3] as f32 * c).round() as u8;
            }
        }
    });
    if let Ok(new) = RgbaImage::from_raw(IRect::new(0, 0, w as i32, h as i32), px) {
        *img = new;
    }
}

struct RoundCorners;

impl Step for RoundCorners {
    fn id(&self) -> &str {
        ROUND_CORNERS
    }
    fn group(&self) -> StepGroup {
        StepGroup::Overlay
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::number("radius", 0.0, 10000.0, 16.0).beginner(),
            ParamDef::choice("unit", &["px", "percent"], "px").beginner(),
        ]
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        item.flatten_document()?;
        let (w, h) = item.dimensions();
        let scale = preview_scale(ctx, w as usize, h as usize);
        let radius = p_f64(params, "radius", 16.0).max(0.0);
        let percent = p_str(params, "unit", "px") == "percent";
        let cancel = ctx.cancel;
        item.map_images(|img| {
            cancel.check()?;
            let r = if percent {
                radius.min(50.0) / 100.0 * img.width().min(img.height()) as f64
            } else {
                radius * scale
            };
            round_corners(img, r);
            Ok(())
        })?;
        if radius > 0.0
            && let Some(m) = item.meta_mut()
        {
            m.has_transparency = true;
        }
        Ok(())
    }
}

// ------------------------------------------------------------------ Vignette

/// Vignette: towards the corners the colour is mixed with `color` up to `strength` (0…1); the
/// centre up to `size` (0…1 of the half diagonal) stays untouched.
pub fn vignette(img: &mut RgbaImage, strength: f32, size: f32, color: [u8; 4]) {
    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 || strength <= 0.0 {
        return;
    }
    let inner = size.clamp(0.0, 0.99);
    let mut px = img.data().to_vec();
    px.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
        for (x, p) in row.iter_mut().enumerate() {
            let nx = (x as f32 + 0.5) / w as f32 * 2.0 - 1.0;
            let ny = (y as f32 + 0.5) / h as f32 * 2.0 - 1.0;
            let d = ((nx * nx + ny * ny) / 2.0).sqrt();
            let t = ((d - inner) / (1.0 - inner)).clamp(0.0, 1.0);
            let k = strength * t * t * (3.0 - 2.0 * t);
            for c in 0..3 {
                p[c] = (p[c] as f32 * (1.0 - k) + color[c] as f32 * k).round() as u8;
            }
        }
    });
    if let Ok(new) = RgbaImage::from_raw(IRect::new(0, 0, w as i32, h as i32), px) {
        *img = new;
    }
}

struct Vignette;

impl Step for Vignette {
    fn id(&self) -> &str {
        VIGNETTE
    }
    fn group(&self) -> StepGroup {
        StepGroup::Overlay
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::int("strength", 0, 100, 50).unit("%").beginner(),
            ParamDef::int("size", 0, 100, 50).unit("%").beginner(),
            ParamDef::color("color", "#000000"),
        ]
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let strength = p_int(params, "strength", 50).clamp(0, 100) as f32 / 100.0;
        let size = p_int(params, "size", 50).clamp(0, 100) as f32 / 100.0;
        let color = p_color(params, "color", [0, 0, 0, 255]);
        let cancel = ctx.cancel;
        item.map_images(|img| {
            cancel.check()?;
            vignette(img, strength, size, color);
            Ok(())
        })
    }
}

// ------------------------------------------------------------------ Posterize / threshold

/// Per-pixel colour map (alpha unchanged).
fn map_rgb(img: &mut RgbaImage, f: impl Fn([u8; 4]) -> [u8; 3] + Sync) {
    let (w, h) = (img.width(), img.height());
    let px: Vec<[u8; 4]> = img
        .data()
        .par_iter()
        .map(|p| {
            let c = f(*p);
            [c[0], c[1], c[2], p[3]]
        })
        .collect();
    if let Ok(new) = RgbaImage::from_raw(IRect::new(0, 0, w as i32, h as i32), px) {
        *img = new;
    }
}

/// Posterize to `levels` steps per channel.
pub fn posterize(img: &mut RgbaImage, levels: u32) {
    let n = levels.clamp(2, 256) as f32 - 1.0;
    let q = |v: u8| ((v as f32 / 255.0 * n).round() / n * 255.0).round() as u8;
    map_rgb(img, |p| [q(p[0]), q(p[1]), q(p[2])]);
}

/// Black/white by luminance (Rec. 601): `>= t` → white.
pub fn threshold(img: &mut RgbaImage, t: u8) {
    map_rgb(img, |p| {
        let l = 0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32;
        let v = if l.round() >= t as f32 { 255 } else { 0 };
        [v, v, v]
    });
}

struct Posterize;

impl Step for Posterize {
    fn id(&self) -> &str {
        POSTERIZE
    }
    fn group(&self) -> StepGroup {
        StepGroup::Color
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![ParamDef::int("levels", 2, 64, 4).beginner()]
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let levels = p_int(params, "levels", 4).clamp(2, 64) as u32;
        let cancel = ctx.cancel;
        item.map_images(|img| {
            cancel.check()?;
            posterize(img, levels);
            Ok(())
        })
    }
}

struct Threshold;

impl Step for Threshold {
    fn id(&self) -> &str {
        THRESHOLD
    }
    fn group(&self) -> StepGroup {
        StepGroup::Color
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![ParamDef::int("threshold", 0, 255, 128).beginner()]
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let t = p_int(params, "threshold", 128).clamp(0, 255) as u8;
        let cancel = ctx.cancel;
        item.map_images(|img| {
            cancel.check()?;
            threshold(img, t);
            Ok(())
        })
    }
}

// ------------------------------------------------------------------ Pixelate

/// Block mosaic: each `block × block` cell gets its alpha-weighted mean colour.
pub fn pixelate(img: &mut RgbaImage, block: usize) {
    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 || block < 2 {
        return;
    }
    let mut px = img.data().to_vec();
    for by in (0..h).step_by(block) {
        for bx in (0..w).step_by(block) {
            let (ex, ey) = ((bx + block).min(w), (by + block).min(h));
            let mut s = [0u64; 4];
            for y in by..ey {
                for x in bx..ex {
                    let p = px[y * w + x];
                    let a = p[3] as u64;
                    for c in 0..3 {
                        s[c] += p[c] as u64 * a;
                    }
                    s[3] += a;
                }
            }
            let n = ((ex - bx) * (ey - by)) as u64;
            let a = s[3];
            let mean = match std::num::NonZeroU64::new(a) {
                None => [0, 0, 0, 0],
                Some(nz) => [
                    ((s[0] + a / 2) / nz) as u8,
                    ((s[1] + a / 2) / nz) as u8,
                    ((s[2] + a / 2) / nz) as u8,
                    ((a + n / 2) / n.max(1)) as u8,
                ],
            };
            for y in by..ey {
                for x in bx..ex {
                    px[y * w + x] = mean;
                }
            }
        }
    }
    if let Ok(new) = RgbaImage::from_raw(IRect::new(0, 0, w as i32, h as i32), px) {
        *img = new;
    }
}

struct Pixelate;

impl Step for Pixelate {
    fn id(&self) -> &str {
        PIXELATE
    }
    fn group(&self) -> StepGroup {
        StepGroup::Color
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![ParamDef::int("size", 2, 200, 8).unit("px").beginner()]
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let (w, h) = item.dimensions();
        let scale = preview_scale(ctx, w as usize, h as usize);
        let block =
            ((p_int(params, "size", 8).clamp(2, 200) as f64 * scale).round() as usize).max(2);
        let cancel = ctx.cancel;
        item.map_images(|img| {
            cancel.check()?;
            pixelate(img, block);
            Ok(())
        })
    }
}

// ------------------------------------------------------------------ Pad to multiple

/// Grows the canvas (centred) to the next multiple of `m` in both directions.
pub fn pad_to_multiple(img: &RgbaImage, m: usize, fill: [u8; 4]) -> Result<RgbaImage> {
    let (w, h) = (img.width(), img.height());
    let m = m.max(1);
    let (nw, nh) = (w.div_ceil(m) * m, h.div_ceil(m) * m);
    if (nw, nh) == (w, h) {
        return Ok(img.clone());
    }
    let (ox, oy) = ((nw - w) / 2, (nh - h) / 2);
    let mut out = vec![fill; nw * nh];
    let src = img.data();
    for y in 0..h {
        let d = (oy + y) * nw + ox;
        out[d..d + w].copy_from_slice(&src[y * w..y * w + w]);
    }
    image(nw, nh, out)
}

struct CanvasPad;

impl Step for CanvasPad {
    fn id(&self) -> &str {
        CANVAS_PAD
    }
    fn group(&self) -> StepGroup {
        StepGroup::Geometry
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::int("multiple", 2, 256, 16).unit("px").beginner(),
            ParamDef::color("color", "transparent").beginner(),
        ]
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let m = p_int(params, "multiple", 16).clamp(2, 256) as usize;
        let fill = p_color(params, "color", [0, 0, 0, 0]);
        let cancel = ctx.cancel;
        item.map_images(|img| {
            cancel.check()?;
            *img = pad_to_multiple(img, m, fill)?;
            Ok(())
        })?;
        if fill[3] < 255
            && let Some(meta) = item.meta_mut()
        {
            meta.has_transparency = true;
        }
        Ok(())
    }
}

// ------------------------------------------------------------------ Still from animation

/// Sharpness (variance of the 4-neighbour Laplacian on the luminance).
fn sharpness(img: &RgbaImage) -> f64 {
    let (w, h) = (img.width(), img.height());
    if w < 3 || h < 3 {
        return 0.0;
    }
    let d = img.data();
    let l = |x: usize, y: usize| {
        let p = d[y * w + x];
        (0.299 * p[0] as f64 + 0.587 * p[1] as f64 + 0.114 * p[2] as f64) * p[3] as f64 / 255.0
    };
    let (mut s, mut s2, mut n) = (0.0, 0.0, 0.0);
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let v = 4.0 * l(x, y) - l(x - 1, y) - l(x + 1, y) - l(x, y - 1) - l(x, y + 1);
            s += v;
            s2 += v * v;
            n += 1.0;
        }
    }
    s2 / n - (s / n).powi(2)
}

struct FirstFrame;

impl Step for FirstFrame {
    fn id(&self) -> &str {
        FIRST_FRAME
    }
    fn group(&self) -> StepGroup {
        StepGroup::Animation
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::choice(
                "pick",
                &["first", "middle", "last", "number", "time", "sharpest"],
                "first",
            )
            .beginner(),
            ParamDef::int("number", 1, 1_000_000, 1)
                .beginner()
                .when("pick", &["number"]),
            ParamDef::int("time", 0, 86_400_000, 0)
                .unit("ms")
                .beginner()
                .when("pick", &["time"]),
        ]
    }
    fn applies_to(&self) -> &[ItemKind] {
        &[ItemKind::Animation]
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let Item::Animation(a) = item else {
            return Ok(());
        };
        let n = a.frames.len();
        if n == 0 {
            return Err(BatchError::Decode(fp_i18n::t("batch-err-no-frames")));
        }
        let idx = match p_str(params, "pick", "first") {
            "middle" => (n - 1) / 2,
            "last" => n - 1,
            "number" => (p_int(params, "number", 1).max(1) as usize - 1).min(n - 1),
            "time" => {
                let t = p_int(params, "time", 0).max(0) as u64;
                let mut acc = 0u64;
                let mut pick = n - 1;
                for (i, f) in a.frames.iter().enumerate() {
                    acc += f.delay_ms as u64;
                    if t < acc {
                        pick = i;
                        break;
                    }
                }
                pick
            }
            "sharpest" => {
                ctx.check_cancel()?;
                a.frames
                    .par_iter()
                    .map(|f| sharpness(&f.image))
                    .enumerate()
                    .reduce(
                        || (0, f64::MIN),
                        |x, y| {
                            if y.1 > x.1 || (y.1 == x.1 && y.0 < x.0) {
                                y
                            } else {
                                x
                            }
                        },
                    )
                    .0
            }
            _ => 0,
        };
        let image = a.frames.swap_remove(idx).image;
        let mut meta = a.meta.clone();
        meta.has_transparency = image.data().iter().any(|p| p[3] < 255);
        *item = Item::Still(Still { image, meta });
        Ok(())
    }
}
