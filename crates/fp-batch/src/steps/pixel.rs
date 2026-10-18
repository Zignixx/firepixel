//! Geometry, colour and optimize steps from the batch spec (`docs/BATCH.md` §3.3, §3.4, §3.11):
//!
//! - `perspective` – four-point correction: the quad (corners in % of the image) becomes the
//!   whole output rectangle (projective mapping, bilinear sampling).
//! - `tile.split` – collect step: every image becomes N × M tiles (grid or tile size).
//! - `filter.noise` – add noise (uniform/Gaussian, colour/mono, reproducible seed) or remove it
//!   (median filter).
//! - `adjust.lut` – colour lookup table from a `.cube` file (3D, trilinear, strength).
//! - `effect.style` – apply a saved style by name (from the document or a style library) through
//!   the command registry in a headless session.
//! - `optimize.jpeg` – JPEG re-encoding settings (quality, progressive, subsampling, optional
//!   target size); applied when the item is encoded.
//! - `optimize.webSafe` – snap every colour to the 216-colour web palette (optional ordered
//!   dithering).

use crate::error::{BatchError, Result};
use crate::item::{Animation, Frame, Item, ItemKind, Still};
use crate::meta::{EXTRA_OUTPUT_OPTIONS, EXTRA_TARGET};
use crate::output::OutputFormat;
use crate::step::{Issue, ParamDef, Step, StepContext, StepGroup, StepKind, StepRegistry};
use crate::steps::anim::{EXTRA_OUTPUT_STEM, EXTRA_SOURCE_NAME};
use crate::steps::document::web216;
use fp_raster::{IRect, RgbaImage};
use rayon::prelude::*;
use serde_json::{Map, Value, json};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

/// ID "Perspective".
pub const PERSPECTIVE: &str = "perspective";
/// ID "Split into tiles".
pub const TILE_SPLIT: &str = "tile.split";
/// ID "Add/remove noise".
pub const NOISE: &str = "filter.noise";
/// ID "Colour LUT".
pub const LUT: &str = "adjust.lut";
/// ID "Apply style".
pub const STYLE: &str = "effect.style";
/// ID "Optimize JPEG".
pub const OPT_JPEG: &str = "optimize.jpeg";
/// ID "Web-safe colours".
pub const WEB_SAFE: &str = "optimize.webSafe";

/// Registers the steps of this file.
pub fn register(r: &mut StepRegistry) {
    r.register(Arc::new(Perspective));
    r.register(Arc::new(TileSplit));
    r.register(Arc::new(Noise));
    r.register(Arc::new(Lut));
    r.register(Arc::new(StyleStep));
    r.register(Arc::new(OptJpeg));
    r.register(Arc::new(WebSafe));
}

fn p_str<'a>(p: &'a Value, k: &str, d: &'a str) -> &'a str {
    p.get(k).and_then(Value::as_str).unwrap_or(d)
}
fn p_f64(p: &Value, k: &str, d: f64) -> f64 {
    p.get(k)
        .and_then(Value::as_f64)
        .filter(|f| f.is_finite())
        .unwrap_or(d)
}
fn p_int(p: &Value, k: &str, d: i64) -> i64 {
    p.get(k)
        .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f.round() as i64)))
        .unwrap_or(d)
}
fn p_bool(p: &Value, k: &str, d: bool) -> bool {
    p.get(k).and_then(Value::as_bool).unwrap_or(d)
}

fn image(w: usize, h: usize, data: Vec<[u8; 4]>) -> Result<RgbaImage> {
    RgbaImage::from_raw(IRect::new(0, 0, w as i32, h as i32), data)
        .map_err(|e| BatchError::Decode(e.to_string()))
}

fn preview_scale(ctx: &StepContext, w: u32, h: u32) -> f64 {
    ctx.preview_scale().unwrap_or_else(|| {
        let long = ctx.info.width.max(ctx.info.height);
        if long == 0 {
            1.0
        } else {
            (w.max(h) as f64 / long as f64).min(1.0)
        }
    })
}

// ------------------------------------------------------------------ Perspective

/// Projective map of the unit square onto the quad `q` (top-left, top-right, bottom-right,
/// bottom-left): returns `(x, y)` for `(u, v)`.
pub fn square_to_quad(q: [(f64, f64); 4]) -> impl Fn(f64, f64) -> (f64, f64) {
    let [(x0, y0), (x1, y1), (x2, y2), (x3, y3)] = q;
    let (dx1, dx2, dx3) = (x1 - x2, x3 - x2, x0 - x1 + x2 - x3);
    let (dy1, dy2, dy3) = (y1 - y2, y3 - y2, y0 - y1 + y2 - y3);
    let det = dx1 * dy2 - dx2 * dy1;
    let (g, h) = if (dx3.abs() < 1e-12 && dy3.abs() < 1e-12) || det.abs() < 1e-12 {
        (0.0, 0.0)
    } else {
        ((dx3 * dy2 - dx2 * dy3) / det, (dx1 * dy3 - dx3 * dy1) / det)
    };
    let (a, b, c) = (x1 - x0 + g * x1, x3 - x0 + h * x3, x0);
    let (d, e, f) = (y1 - y0 + g * y1, y3 - y0 + h * y3, y0);
    move |u: f64, v: f64| {
        let w = g * u + h * v + 1.0;
        let w = if w.abs() < 1e-12 { 1e-12 } else { w };
        ((a * u + b * v + c) / w, (d * u + e * v + f) / w)
    }
}

/// Bilinear sample at pixel-centre coordinates (`x`, `y`) with premultiplied alpha; outside =
/// transparent.
pub fn sample(img: &RgbaImage, x: f64, y: f64) -> [u8; 4] {
    let (w, h) = (img.width() as i64, img.height() as i64);
    let d = img.data();
    let (fx, fy) = (x.floor(), y.floor());
    let (tx, ty) = (x - fx, y - fy);
    let mut acc = [0f64; 4];
    for (dy, wy) in [(0i64, 1.0 - ty), (1, ty)] {
        for (dx, wx) in [(0i64, 1.0 - tx), (1, tx)] {
            let wgt = wx * wy;
            if wgt <= 0.0 {
                continue;
            }
            let (sx, sy) = (fx as i64 + dx, fy as i64 + dy);
            if sx < 0 || sy < 0 || sx >= w || sy >= h {
                continue;
            }
            let p = d[(sy * w + sx) as usize];
            let a = p[3] as f64 / 255.0 * wgt;
            for c in 0..3 {
                acc[c] += p[c] as f64 * a;
            }
            acc[3] += a;
        }
    }
    if acc[3] <= 1e-9 {
        return [0, 0, 0, 0];
    }
    [
        (acc[0] / acc[3]).round().clamp(0.0, 255.0) as u8,
        (acc[1] / acc[3]).round().clamp(0.0, 255.0) as u8,
        (acc[2] / acc[3]).round().clamp(0.0, 255.0) as u8,
        (acc[3] * 255.0).round().clamp(0.0, 255.0) as u8,
    ]
}

/// Four-point correction: `quad` in pixels of `img` (TL, TR, BR, BL) → `out_w × out_h`.
pub fn perspective(
    img: &RgbaImage,
    quad: [(f64, f64); 4],
    out_w: usize,
    out_h: usize,
) -> Result<RgbaImage> {
    let map = square_to_quad(quad);
    let mut px = vec![[0u8; 4]; out_w * out_h];
    px.par_chunks_mut(out_w.max(1))
        .enumerate()
        .for_each(|(oy, row)| {
            for (ox, p) in row.iter_mut().enumerate() {
                let u = (ox as f64 + 0.5) / out_w as f64;
                let v = (oy as f64 + 0.5) / out_h as f64;
                let (x, y) = map(u, v);
                *p = sample(img, x - 0.5, y - 0.5);
            }
        });
    image(out_w, out_h, px)
}

const CORNERS: [&str; 8] = ["tlX", "tlY", "trX", "trY", "brX", "brY", "blX", "blY"];
const CORNER_DEFAULTS: [f64; 8] = [0.0, 0.0, 100.0, 0.0, 100.0, 100.0, 0.0, 100.0];

struct Perspective;

impl Step for Perspective {
    fn id(&self) -> &str {
        PERSPECTIVE
    }
    fn group(&self) -> StepGroup {
        StepGroup::Geometry
    }
    fn params(&self) -> Vec<ParamDef> {
        let mut v: Vec<ParamDef> = CORNERS
            .iter()
            .zip(CORNER_DEFAULTS)
            .map(|(n, d)| ParamDef::number(n, -100.0, 200.0, d).unit("%").beginner())
            .collect();
        v.push(ParamDef::choice("size", &["auto", "keep"], "auto").beginner());
        v
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let c: Vec<f64> = CORNERS
            .iter()
            .zip(CORNER_DEFAULTS)
            .map(|(n, d)| p_f64(params, n, d) / 100.0)
            .collect();
        let keep = p_str(params, "size", "auto") == "keep";
        let cancel = ctx.cancel;
        item.map_images(|img| {
            cancel.check()?;
            let (w, h) = (img.width() as f64, img.height() as f64);
            let q = [
                (c[0] * w, c[1] * h),
                (c[2] * w, c[3] * h),
                (c[4] * w, c[5] * h),
                (c[6] * w, c[7] * h),
            ];
            let dist =
                |a: (f64, f64), b: (f64, f64)| ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt();
            let (ow, oh) = if keep {
                (img.width(), img.height())
            } else {
                (
                    ((dist(q[0], q[1]) + dist(q[3], q[2])) / 2.0)
                        .round()
                        .max(1.0) as usize,
                    ((dist(q[0], q[3]) + dist(q[1], q[2])) / 2.0)
                        .round()
                        .max(1.0) as usize,
                )
            };
            *img = perspective(img, q, ow.min(32768), oh.min(32768))?;
            Ok(())
        })?;
        if let Some(m) = item.meta_mut() {
            m.has_transparency = true;
        }
        crate::steps::geometry::refresh_transparency(item);
        Ok(())
    }
}

// ------------------------------------------------------------------ Split into tiles

/// Tile rectangles `(row, col, rect)` for an image `w × h`: a grid of `cols × rows` or tiles of
/// `tw × th` (the last row/column may be smaller).
pub fn tile_rects(
    w: usize,
    h: usize,
    grid: Option<(usize, usize)>,
    size: (usize, usize),
) -> Vec<(usize, usize, IRect)> {
    let (xs, ys): (Vec<usize>, Vec<usize>) = match grid {
        Some((cols, rows)) => {
            let (cols, rows) = (cols.clamp(1, w.max(1)), rows.clamp(1, h.max(1)));
            (
                (0..=cols).map(|i| i * w / cols).collect(),
                (0..=rows).map(|i| i * h / rows).collect(),
            )
        }
        None => {
            let (tw, th) = (size.0.max(1), size.1.max(1));
            let mut xs: Vec<usize> = (0..w).step_by(tw).collect();
            xs.push(w);
            let mut ys: Vec<usize> = (0..h).step_by(th).collect();
            ys.push(h);
            (xs, ys)
        }
    };
    let mut out = Vec::new();
    for r in 0..ys.len().saturating_sub(1) {
        for c in 0..xs.len().saturating_sub(1) {
            let rect = IRect::new(
                xs[c] as i32,
                ys[r] as i32,
                xs[c + 1] as i32,
                ys[r + 1] as i32,
            );
            if !rect.is_empty() {
                out.push((r + 1, c + 1, rect));
            }
        }
    }
    out
}

fn crop(img: &RgbaImage, r: IRect) -> RgbaImage {
    let mut c = img.cropped(r);
    c.set_origin(0, 0);
    c
}

struct TileSplit;

impl Step for TileSplit {
    fn id(&self) -> &str {
        TILE_SPLIT
    }
    fn group(&self) -> StepGroup {
        StepGroup::Geometry
    }
    fn kind(&self) -> StepKind {
        StepKind::Collect
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::choice("mode", &["grid", "size"], "grid").beginner(),
            ParamDef::int("columns", 1, 100, 2)
                .beginner()
                .when("mode", &["grid"]),
            ParamDef::int("rows", 1, 100, 2)
                .beginner()
                .when("mode", &["grid"]),
            ParamDef::int("tileWidth", 1, 32768, 256)
                .unit("px")
                .beginner()
                .when("mode", &["size"]),
            ParamDef::int("tileHeight", 1, 32768, 256)
                .unit("px")
                .beginner()
                .when("mode", &["size"]),
            ParamDef::text("pattern", "{name}_{row}_{col}"),
        ]
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn preview_ok(&self) -> bool {
        false
    }
    fn apply(&self, _item: &mut Item, _params: &Value, _ctx: &mut StepContext) -> Result<()> {
        Ok(())
    }
    fn collect(
        &self,
        items: Vec<Item>,
        params: &Value,
        ctx: &mut StepContext,
    ) -> Result<Vec<Item>> {
        let grid = (p_str(params, "mode", "grid") == "grid").then(|| {
            (
                p_int(params, "columns", 2).clamp(1, 100) as usize,
                p_int(params, "rows", 2).clamp(1, 100) as usize,
            )
        });
        let size = (
            p_int(params, "tileWidth", 256).clamp(1, 32768) as usize,
            p_int(params, "tileHeight", 256).clamp(1, 32768) as usize,
        );
        let pattern = match p_str(params, "pattern", "{name}_{row}_{col}").trim() {
            "" => "{name}_{row}_{col}",
            p => p,
        };
        let fallback = crate::naming::split_extension(&ctx.info.file_name()).0;
        let mut used = HashSet::new();
        let mut out = Vec::new();
        for mut item in items {
            ctx.check_cancel()?;
            item.flatten_document()?;
            let name = item
                .meta()
                .and_then(|m| m.extra.get(EXTRA_SOURCE_NAME))
                .and_then(Value::as_str)
                .map_or_else(|| fallback.clone(), str::to_string);
            let (w, h) = item.dimensions();
            let rects = tile_rects(w as usize, h as usize, grid, size);
            let (rows, cols) = rects
                .iter()
                .fold((0, 0), |(r, c), t| (r.max(t.0), c.max(t.1)));
            let wid = |n: usize| n.to_string().len();
            for (r, c, rect) in rects {
                let stem = pattern
                    .replace("{name}", &name)
                    .replace("{row}", &format!("{r:0w$}", w = wid(rows)))
                    .replace("{col}", &format!("{c:0w$}", w = wid(cols)));
                let mut hints = Vec::new();
                let mut stem = crate::naming::sanitize_stem(&stem, &mut hints);
                if !used.insert(stem.to_lowercase()) {
                    let mut k = 2;
                    while !used.insert(format!("{stem} ({k})").to_lowercase()) {
                        k += 1;
                    }
                    stem = format!("{stem} ({k})");
                }
                let tile = match &item {
                    Item::Still(s) => Item::Still(Still {
                        image: crop(&s.image, rect),
                        meta: s.meta.clone(),
                    }),
                    Item::Animation(a) => Item::Animation(Animation {
                        width: rect.width(),
                        height: rect.height(),
                        frames: a
                            .frames
                            .iter()
                            .map(|f| Frame {
                                image: crop(&f.image, rect),
                                delay_ms: f.delay_ms,
                            })
                            .collect(),
                        loop_count: a.loop_count,
                        meta: a.meta.clone(),
                    }),
                    _ => continue,
                };
                let mut tile = tile;
                crate::steps::geometry::refresh_transparency(&mut tile);
                if let Some(m) = tile.meta_mut() {
                    m.extra.insert(EXTRA_OUTPUT_STEM.into(), json!(stem));
                }
                out.push(tile);
            }
        }
        if out.is_empty() {
            return Err(BatchError::Invalid(fp_i18n::t("batch-anim-err-empty")));
        }
        Ok(out)
    }
}

// ------------------------------------------------------------------ Noise

/// Small reproducible generator (xorshift64*).
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// Uniform in [0, 1).
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
    /// Standard normal (Box-Muller).
    fn normal(&mut self) -> f64 {
        let u1 = self.unit().max(1e-12);
        let u2 = self.unit();
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }
}

/// Adds noise: `amount` 0…1 (uniform ± amount·127.5, Gaussian σ = amount·64), alpha unchanged.
pub fn add_noise(img: &mut RgbaImage, amount: f64, gaussian: bool, mono: bool, seed: u64) {
    let w = img.width().max(1);
    let d = img.data_mut();
    d.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
        let mut rng = Rng::new(seed ^ (y as u64).wrapping_mul(0xA24B_AED4_963E_E407));
        for p in row.iter_mut() {
            let mut draw = || {
                if gaussian {
                    rng.normal() * amount * 64.0
                } else {
                    (rng.unit() * 2.0 - 1.0) * amount * 127.5
                }
            };
            let one = draw();
            for (c, v) in p.iter_mut().take(3).enumerate() {
                let n = if mono || c == 0 { one } else { draw() };
                *v = (*v as f64 + n).round().clamp(0.0, 255.0) as u8;
            }
        }
    });
}

/// Median filter per channel (alpha included) with radius `r`.
pub fn median(img: &mut RgbaImage, r: usize) {
    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 || r == 0 {
        return;
    }
    let src = img.data().to_vec();
    let mut out = vec![[0u8; 4]; w * h];
    out.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
        let mut win: [Vec<u8>; 4] = Default::default();
        for (x, p) in row.iter_mut().enumerate() {
            for v in &mut win {
                v.clear();
            }
            for yy in y.saturating_sub(r)..(y + r + 1).min(h) {
                for xx in x.saturating_sub(r)..(x + r + 1).min(w) {
                    let s = src[yy * w + xx];
                    for c in 0..4 {
                        win[c].push(s[c]);
                    }
                }
            }
            for c in 0..4 {
                let mid = win[c].len() / 2;
                let (_, m, _) = win[c].select_nth_unstable(mid);
                p[c] = *m;
            }
        }
    });
    if let Ok(new) = RgbaImage::from_raw(IRect::new(0, 0, w as i32, h as i32), out) {
        *img = new;
    }
}

struct Noise;

impl Step for Noise {
    fn id(&self) -> &str {
        NOISE
    }
    fn group(&self) -> StepGroup {
        StepGroup::Color
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::choice("mode", &["add", "remove"], "add").beginner(),
            ParamDef::int("amount", 0, 100, 10)
                .unit("%")
                .beginner()
                .when("mode", &["add"]),
            ParamDef::choice("distribution", &["uniform", "gaussian"], "gaussian")
                .when("mode", &["add"]),
            ParamDef::bool("monochrome", false).when("mode", &["add"]),
            ParamDef::int("seed", 0, 1_000_000, 1).when("mode", &["add"]),
            ParamDef::int("radius", 1, 10, 1)
                .unit("px")
                .beginner()
                .when("mode", &["remove"]),
        ]
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let (w, h) = item.dimensions();
        let scale = preview_scale(ctx, w, h);
        let cancel = ctx.cancel;
        if p_str(params, "mode", "add") == "remove" {
            let r =
                ((p_int(params, "radius", 1).clamp(1, 10) as f64 * scale).round() as usize).max(1);
            return item.map_images(|img| {
                cancel.check()?;
                median(img, r);
                Ok(())
            });
        }
        let amount = p_int(params, "amount", 10).clamp(0, 100) as f64 / 100.0;
        let gaussian = p_str(params, "distribution", "gaussian") == "gaussian";
        let mono = p_bool(params, "monochrome", false);
        let seed = p_int(params, "seed", 1).max(0) as u64;
        let mut k = 0u64;
        item.map_images(|img| {
            cancel.check()?;
            add_noise(img, amount, gaussian, mono, seed.wrapping_add(k));
            k += 1;
            Ok(())
        })
    }
}

// ------------------------------------------------------------------ LUT

/// A 3D colour table (`size³` entries, red fastest), values 0…1 after the domain.
#[derive(Clone, Debug, PartialEq)]
pub struct Lut3d {
    pub size: usize,
    pub table: Vec<[f32; 3]>,
    pub domain_min: [f32; 3],
    pub domain_max: [f32; 3],
}

/// Parses a `.cube` file (3D only).
pub fn parse_cube(text: &str) -> std::result::Result<Lut3d, String> {
    let mut size = 0usize;
    let mut table = Vec::new();
    let (mut dmin, mut dmax) = ([0f32; 3], [1f32; 3]);
    let nums = |rest: &str| -> std::result::Result<[f32; 3], String> {
        let v: Vec<f32> = rest
            .split_whitespace()
            .map(|t| t.parse::<f32>().map_err(|e| e.to_string()))
            .collect::<std::result::Result<_, _>>()?;
        match v.as_slice() {
            [a, b, c] => Ok([*a, *b, *c]),
            _ => Err(fp_i18n::t("batch-err-lut-line")),
        }
    };
    for line in text.lines() {
        let l = line.trim();
        if l.is_empty() || l.starts_with('#') {
            continue;
        }
        let (key, rest) = l.split_once(char::is_whitespace).unwrap_or((l, ""));
        match key.to_ascii_uppercase().as_str() {
            "TITLE" => {}
            "LUT_3D_SIZE" => {
                size = rest
                    .trim()
                    .parse()
                    .map_err(|_| fp_i18n::t("batch-err-lut-size"))?;
            }
            "LUT_1D_SIZE" => return Err(fp_i18n::t("batch-err-lut-1d")),
            "DOMAIN_MIN" => dmin = nums(rest)?,
            "DOMAIN_MAX" => dmax = nums(rest)?,
            k if k.starts_with("LUT_") => {}
            _ => table.push(nums(l)?),
        }
    }
    if !(2..=256).contains(&size) {
        return Err(fp_i18n::t("batch-err-lut-size"));
    }
    if table.len() != size * size * size {
        return Err(fp_i18n::t("batch-err-lut-count"));
    }
    Ok(Lut3d {
        size,
        table,
        domain_min: dmin,
        domain_max: dmax,
    })
}

impl Lut3d {
    /// Trilinear lookup of an RGB colour (0…1 each).
    pub fn lookup(&self, rgb: [f32; 3]) -> [f32; 3] {
        let n = self.size;
        let m = (n - 1) as f32;
        let mut i0 = [0usize; 3];
        let mut t = [0f32; 3];
        for c in 0..3 {
            let span = (self.domain_max[c] - self.domain_min[c]).max(1e-6);
            let x = ((rgb[c] - self.domain_min[c]) / span).clamp(0.0, 1.0) * m;
            let f = x.floor().min(m - 1.0).max(0.0);
            i0[c] = f as usize;
            t[c] = x - f;
        }
        let at = |r: usize, g: usize, b: usize| self.table[r + g * n + b * n * n];
        let mut out = [0f32; 3];
        for (dr, wr) in [(0, 1.0 - t[0]), (1, t[0])] {
            for (dg, wg) in [(0, 1.0 - t[1]), (1, t[1])] {
                for (db, wb) in [(0, 1.0 - t[2]), (1, t[2])] {
                    let wgt = wr * wg * wb;
                    if wgt == 0.0 {
                        continue;
                    }
                    let v = at(i0[0] + dr, i0[1] + dg, i0[2] + db);
                    for c in 0..3 {
                        out[c] += v[c] * wgt;
                    }
                }
            }
        }
        out
    }

    /// Applies the table with `strength` 0…1 (alpha unchanged).
    pub fn apply(&self, img: &mut RgbaImage, strength: f32) {
        img.data_mut().par_iter_mut().for_each(|p| {
            let src = [
                p[0] as f32 / 255.0,
                p[1] as f32 / 255.0,
                p[2] as f32 / 255.0,
            ];
            let v = self.lookup(src);
            for c in 0..3 {
                let x = src[c] + (v[c] - src[c]) * strength;
                p[c] = (x * 255.0).round().clamp(0.0, 255.0) as u8;
            }
        });
    }
}

/// Reads (and caches by path and modification time) a `.cube` file.
pub fn load_cube(path: &str) -> std::result::Result<Arc<Lut3d>, String> {
    type Cache = Mutex<HashMap<(PathBuf, Option<std::time::SystemTime>), Arc<Lut3d>>>;
    static CACHE: OnceLock<Cache> = OnceLock::new();
    let p = PathBuf::from(path.trim());
    let modified = std::fs::metadata(&p).and_then(|m| m.modified()).ok();
    let key = (p.clone(), modified);
    let cache = CACHE.get_or_init(Default::default);
    if let Some(l) = cache.lock().ok().and_then(|c| c.get(&key).cloned()) {
        return Ok(l);
    }
    let text = std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
    let lut = Arc::new(parse_cube(&text)?);
    if let Ok(mut c) = cache.lock() {
        if c.len() > 16 {
            c.clear();
        }
        c.insert(key, lut.clone());
    }
    Ok(lut)
}

struct Lut;

impl Step for Lut {
    fn id(&self) -> &str {
        LUT
    }
    fn group(&self) -> StepGroup {
        StepGroup::Color
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::path("file").beginner(),
            ParamDef::int("strength", 0, 100, 100).unit("%").beginner(),
        ]
    }
    fn validate(&self, params: &Value) -> Vec<Issue> {
        let mut v = crate::step::validate_params(&self.params(), params);
        let f = p_str(params, "file", "");
        if f.trim().is_empty() {
            v.push(Issue::param("file", fp_i18n::t("batch-err-lut-file")));
        } else if let Err(e) = load_cube(f) {
            v.push(Issue::param("file", e));
        }
        v
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let lut = load_cube(p_str(params, "file", "")).map_err(BatchError::Invalid)?;
        let strength = p_int(params, "strength", 100).clamp(0, 100) as f32 / 100.0;
        let cancel = ctx.cancel;
        item.map_images(|img| {
            cancel.check()?;
            lut.apply(img, strength);
            Ok(())
        })
    }
}

// ------------------------------------------------------------------ Apply style

struct StyleStep;

impl Step for StyleStep {
    fn id(&self) -> &str {
        STYLE
    }
    fn group(&self) -> StepGroup {
        StepGroup::Color
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::text("name", "").beginner(),
            ParamDef::path("library").beginner(),
        ]
    }
    fn validate(&self, params: &Value) -> Vec<Issue> {
        let mut v = crate::step::validate_params(&self.params(), params);
        if p_str(params, "name", "").trim().is_empty() {
            v.push(Issue::param("name", fp_i18n::t("batch-err-style-name")));
        }
        v
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let name = p_str(params, "name", "").trim().to_string();
        let library = p_str(params, "library", "").trim().to_string();
        let engine = |e: fp_engine::EngineError| BatchError::Invalid(e.to_string());
        crate::steps::script::run_on_item(item, ctx, false, &|s| {
            if !library.is_empty() {
                s.execute("panel.styles.importLibrary", json!({"path": library}))
                    .map_err(engine)?;
            }
            let found = s
                .doc()
                .map_err(engine)?
                .doc
                .styles
                .as_ref()
                .is_some_and(|v| v.iter().any(|x| x.name == name));
            if !found {
                return Err(BatchError::Invalid(fp_i18n::tr!(
                    "batch-err-style-missing",
                    name = name.clone()
                )));
            }
            s.execute("select.all", json!({})).map_err(engine)?;
            s.execute("pi.style.apply", json!({"name": name}))
                .map_err(engine)?;
            Ok(())
        })
    }
}

// ------------------------------------------------------------------ Optimize JPEG

struct OptJpeg;

impl Step for OptJpeg {
    fn id(&self) -> &str {
        OPT_JPEG
    }
    fn group(&self) -> StepGroup {
        StepGroup::Optimize
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::int("quality", 1, 100, 82).beginner(),
            ParamDef::number("maxKB", 0.0, 100_000.0, 0.0)
                .unit("KB")
                .beginner(),
            ParamDef::int("minQuality", 10, 95, 40),
            ParamDef::bool("progressive", true),
            ParamDef::choice("subsampling", &["auto", "444", "422", "420"], "auto"),
        ]
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        if ctx.output_format != OutputFormat::Jpeg {
            ctx.warn(fp_i18n::tr!(
                "batch-warn-jpeg-only",
                format = ctx.output_format.display_name()
            ));
            return Ok(());
        }
        let Some(m) = item.meta_mut() else {
            return Ok(());
        };
        let mut o: Map<String, Value> = m
            .extra
            .get(EXTRA_OUTPUT_OPTIONS)
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        o.insert(
            "quality".into(),
            json!(p_int(params, "quality", 82).clamp(1, 100)),
        );
        o.insert(
            "progressive".into(),
            json!(p_bool(params, "progressive", true)),
        );
        o.insert(
            "subsampling".into(),
            json!(match p_str(params, "subsampling", "auto") {
                "444" => "4:4:4",
                "422" => "4:2:2",
                "420" => "4:2:0",
                _ => "auto",
            }),
        );
        m.extra
            .insert(EXTRA_OUTPUT_OPTIONS.into(), Value::Object(o));
        let kb = p_f64(params, "maxKB", 0.0);
        if kb > 0.0 {
            m.extra.insert(
                EXTRA_TARGET.into(),
                json!({"maxKB": kb, "strategy": "quality", "minQuality": p_int(params, "minQuality", 40)}),
            );
        } else {
            m.extra.remove(EXTRA_TARGET);
        }
        Ok(())
    }
}

// ------------------------------------------------------------------ Web-safe colours

/// Snaps every pixel to the 216-colour web palette; `ordered` adds a 4 × 4 Bayer offset of
/// ±25 first (ordered dithering). Alpha unchanged.
pub fn web_safe(img: &mut RgbaImage, ordered: bool) {
    const BAYER: [[f32; 4]; 4] = [
        [0.0, 8.0, 2.0, 10.0],
        [12.0, 4.0, 14.0, 6.0],
        [3.0, 11.0, 1.0, 9.0],
        [15.0, 7.0, 13.0, 5.0],
    ];
    let w = img.width().max(1);
    img.data_mut()
        .par_chunks_mut(w)
        .enumerate()
        .for_each(|(y, row)| {
            for (x, p) in row.iter_mut().enumerate() {
                let off = if ordered {
                    ((BAYER[y % 4][x % 4] + 0.5) / 16.0 - 0.5) * 51.0
                } else {
                    0.0
                };
                let rgb = [0, 1, 2].map(|c| (p[c] as f32 + off).round().clamp(0.0, 255.0) as u8);
                let s = web216(rgb);
                p[..3].copy_from_slice(&s);
            }
        });
}

struct WebSafe;

impl Step for WebSafe {
    fn id(&self) -> &str {
        WEB_SAFE
    }
    fn group(&self) -> StepGroup {
        StepGroup::Optimize
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![ParamDef::choice("dither", &["none", "ordered"], "none").beginner()]
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let ordered = p_str(params, "dither", "none") == "ordered";
        let cancel = ctx.cancel;
        item.map_images(|img| {
            cancel.check()?;
            web_safe(img, ordered);
            Ok(())
        })
    }
}
