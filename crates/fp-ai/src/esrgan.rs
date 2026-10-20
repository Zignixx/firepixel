//! AI upscaling ×2/×4 with Real-ESRGAN (RRDBNet from BasicSR, weights BSD-3-Clause).
//!
//! The network follows `basicsr/archs/rrdbnet_arch.py` exactly:
//!
//! * scale 2 (`RealESRGAN_x2plus`): `pixel_unshuffle(2)` at the input (3 → 12 channels at half
//!   size), scale 1 would be `pixel_unshuffle(4)`; scale 4 takes the image directly,
//! * `conv_first` 3×3 → 64 channels,
//! * 23 RRDB blocks, each three residual dense blocks with five 3×3 convolutions (growth 32,
//!   LeakyReLU 0.2 after conv1…conv4, output `x5 · 0.2 + x`), RRDB output `out · 0.2 + x`,
//! * `conv_body`, added to the `conv_first` features,
//! * twice nearest ×2 + 3×3 conv + LeakyReLU (`conv_up1`, `conv_up2`), then `conv_hr` +
//!   LeakyReLU and `conv_last` → 3 channels.
//!
//! Pre/post-processing as in `RealESRGANer` (inference script of Real-ESRGAN): RGB / 255 without
//! mean/std, odd sizes of the ×2 model reflect-padded to even, output clamped to 0…1 and rounded
//! to 8 bit. Large images run in tiles ([`TILE`] network px with [`TILE_PAD`] px context on every
//! side, only the tile core is kept, no blending – like `RealESRGANer.tile_process`, whose default
//! context is 10 px); the 16-fold high-resolution part runs in row stripes inside each tile.
//!
//! Alpha: images without transparency skip it. Otherwise [`AlphaMode::Network`] (default, as
//! `alpha_upsampler='realesrgan'` in Real-ESRGAN) runs the alpha channel as a grey RGB image
//! through the same network and takes the luma (0.299 R + 0.587 G + 0.114 B) of the result;
//! [`AlphaMode::Bicubic`] scales alpha separately with the bicubic kernel (A = −0.75, half the
//! time). Colours are processed straight (not premultiplied), as Real-ESRGAN does.

use crate::cache::{self, Loaded};
use crate::cancel::{Cancel, Progress, Stage};
use crate::error::{AiError, Result};
use crate::models::ModelId;
use crate::nn::{self, Conv2d};
use crate::weights::Weights;
use fp_geom::IRect;
use fp_raster::RgbaImage;
use rayon::prelude::*;

/// Tile size of the tiled inference in network pixels (×4 model: input pixels; ×2 model: half of
/// them, its input is `pixel_unshuffle`d).
pub const TILE: usize = 384;
/// Context around every tile in network pixels; only the core of each tile is kept. Measured
/// against one pass (synthetic test image, 48 px tiles, ×4 model): 16 → mean 0.76/255,
/// 24 → 0.46/255, 32 → 0.25/255; with 384 px tiles the seams are eight times rarer.
pub const TILE_PAD: usize = 24;
/// Rows above/below a stripe of the high-resolution part (its receptive field is ±2 rows).
const HR_HALO: usize = 2;
/// Slope of the LeakyReLU.
const SLOPE: f32 = 0.2;
/// Residual scaling of the dense blocks.
const RES_SCALE: f32 = 0.2;

fn bad(name: &str) -> AiError {
    AiError::Weights(fp_i18n::tr!("ai-err-d-esrgan-weights", name = name))
}

fn lrelu(x: &mut [f32]) {
    nn::map_inplace(x, |v| if v >= 0.0 { v } else { v * SLOPE });
}

/// `out = out · s + x` (parallel).
fn scaled_residual(out: &mut [f32], x: &[f32], s: f32) {
    out.par_chunks_mut(16 * 1024)
        .zip(x.par_chunks(16 * 1024))
        .for_each(|(o, x)| {
            for (p, q) in o.iter_mut().zip(x) {
                *p = *p * s + *q;
            }
        });
}

/// `pixel_unshuffle` of BasicSR: `[c, h, w]` → `[c·r², h/r, w/r]`, output channel
/// `ci·r² + sy·r + sx` holds the input pixels `(y·r + sy, x·r + sx)`.
pub fn pixel_unshuffle(x: &[f32], c: usize, h: usize, w: usize, r: usize) -> Vec<f32> {
    let (hh, ww) = (h / r, w / r);
    let mut out = nn::take(c * r * r * hh * ww);
    out.par_chunks_mut(ww.max(1))
        .enumerate()
        .for_each(|(row, orow)| {
            let (oc, y) = (row / hh, row % hh);
            let (ci, sy, sx) = (oc / (r * r), (oc / r) % r, oc % r);
            let src = &x[ci * h * w + (y * r + sy) * w..][..w];
            for (xx, v) in orow.iter_mut().enumerate() {
                *v = src[xx * r + sx];
            }
        });
    out
}

/// Residual dense block (five convolutions, dense connections).
#[derive(Clone, Debug)]
struct Rdb {
    conv: Vec<Conv2d>,
}

impl Rdb {
    fn load(wt: &mut Weights, prefix: &str, nf: usize) -> Result<Rdb> {
        let conv = (1..=5)
            .map(|i| wt.conv(&format!("{prefix}.conv{i}"), 1, 1, 1))
            .collect::<Result<Vec<_>>>()?;
        let gc = conv[0].cout;
        for (i, c) in conv.iter().enumerate() {
            let cout = if i == 4 { nf } else { gc };
            if c.k != 3 || c.cin != nf + i * gc || c.cout != cout {
                return Err(bad(&format!("{prefix}.conv{}", i + 1)));
            }
        }
        Ok(Rdb { conv })
    }

    fn forward(&self, x: &[f32], h: usize, w: usize) -> Vec<f32> {
        let hw = h * w;
        let mut cat = nn::take(self.conv[4].cin * hw);
        let mut c = self.conv[0].cin;
        cat[..c * hw]
            .par_chunks_mut(16 * 1024)
            .zip(x.par_chunks(16 * 1024))
            .for_each(|(a, b)| a.copy_from_slice(b));
        for conv in &self.conv[..4] {
            let (mut y, _, _) = conv.forward(&cat[..c * hw], h, w);
            lrelu(&mut y);
            cat[c * hw..(c + conv.cout) * hw]
                .par_chunks_mut(16 * 1024)
                .zip(y.par_chunks(16 * 1024))
                .for_each(|(a, b)| a.copy_from_slice(b));
            nn::give(y);
            c += conv.cout;
        }
        let (mut y, _, _) = self.conv[4].forward(&cat[..c * hw], h, w);
        nn::give(cat);
        scaled_residual(&mut y, x, RES_SCALE);
        y
    }
}

/// Residual-in-residual dense block (three dense blocks).
#[derive(Clone, Debug)]
struct Rrdb {
    rdb: [Rdb; 3],
}

impl Rrdb {
    fn load(wt: &mut Weights, prefix: &str, nf: usize) -> Result<Rrdb> {
        Ok(Rrdb {
            rdb: [
                Rdb::load(wt, &format!("{prefix}.rdb1"), nf)?,
                Rdb::load(wt, &format!("{prefix}.rdb2"), nf)?,
                Rdb::load(wt, &format!("{prefix}.rdb3"), nf)?,
            ],
        })
    }

    fn forward(&self, x: &[f32], h: usize, w: usize) -> Vec<f32> {
        let a = self.rdb[0].forward(x, h, w);
        let b = self.rdb[1].forward(&a, h, w);
        nn::give(a);
        let mut c = self.rdb[2].forward(&b, h, w);
        nn::give(b);
        scaled_residual(&mut c, x, RES_SCALE);
        c
    }
}

/// RRDBNet (Real-ESRGAN generator).
#[derive(Clone, Debug)]
pub struct Esrgan {
    /// `pixel_unshuffle` factor at the input (1 = none, 2 for the ×2 model, 4 for ×1).
    unshuffle: usize,
    conv_first: Conv2d,
    body: Vec<Rrdb>,
    conv_body: Conv2d,
    conv_up1: Conv2d,
    conv_up2: Conv2d,
    conv_hr: Conv2d,
    conv_last: Conv2d,
}

impl Esrgan {
    /// Builds the network from BasicSR weight names (`conv_first`, `body.N.rdbM.convK`,
    /// `conv_body`, `conv_up1`, `conv_up2`, `conv_hr`, `conv_last`). The scale follows from the
    /// input channels of `conv_first` (3 → ×4, 12 → ×2, 48 → ×1), the block count from the names.
    pub fn load(mut wt: Weights) -> Result<Esrgan> {
        let conv_first = wt.conv("conv_first", 1, 1, 1)?;
        let unshuffle = match conv_first.cin {
            3 => 1,
            12 => 2,
            48 => 4,
            _ => return Err(bad("conv_first")),
        };
        let nf = conv_first.cout;
        let mut body = Vec::new();
        while wt.has(&format!("body.{}.rdb1.conv1.weight", body.len())) {
            let p = format!("body.{}", body.len());
            body.push(Rrdb::load(&mut wt, &p, nf)?);
        }
        if body.is_empty() {
            return Err(bad("body.0"));
        }
        let mut conv = |name: &str, cout: usize| -> Result<Conv2d> {
            let c = wt.conv(name, 1, 1, 1)?;
            if c.k != 3 || c.cin != nf || c.cout != cout {
                return Err(bad(name));
            }
            Ok(c)
        };
        Ok(Esrgan {
            unshuffle,
            conv_first,
            conv_body: conv("conv_body", nf)?,
            conv_up1: conv("conv_up1", nf)?,
            conv_up2: conv("conv_up2", nf)?,
            conv_hr: conv("conv_hr", nf)?,
            conv_last: conv("conv_last", 3)?,
            body,
        })
    }

    /// Upscaling factor of the whole network (4 / unshuffle).
    pub fn scale(&self) -> usize {
        4 / self.unshuffle
    }

    /// Input sizes must be multiples of this (the `pixel_unshuffle` factor).
    pub fn unit(&self) -> usize {
        self.unshuffle
    }

    /// Number of RRDB blocks.
    pub fn blocks(&self) -> usize {
        self.body.len()
    }

    /// Whole network on `x[3, h, w]` (0…1) → `[3, h·s, w·s]` (not clamped). Checks `cancel`
    /// between the blocks. The high-resolution part (`conv_up1` … `conv_last`, receptive field
    /// ±2 network rows) runs in row stripes with a halo of [`HR_HALO`] rows, so its 16-fold
    /// memory does not limit the tile size; the result is the same as in one piece.
    pub fn forward(&self, x: &[f32], h: usize, w: usize, cancel: &Cancel) -> Result<Vec<f32>> {
        self.forward_striped(x, h, w, cancel, 0)
    }

    /// [`Esrgan::forward`] with a fixed stripe height (network rows, 0 = automatic).
    pub fn forward_striped(
        &self,
        x: &[f32],
        h: usize,
        w: usize,
        cancel: &Cancel,
        stripe: usize,
    ) -> Result<Vec<f32>> {
        let u = self.unshuffle;
        if h == 0 || w == 0 || x.len() != 3 * h * w || !h.is_multiple_of(u) || !w.is_multiple_of(u)
        {
            return Err(AiError::Input(fp_i18n::tr!(
                "ai-err-d-esrgan-input",
                width = w,
                height = h,
                unit = u
            )));
        }
        let nf = self.conv_first.cout;
        let (lh, lw) = (h / u, w / u);
        let (feat, _, _) = if u > 1 {
            let un = pixel_unshuffle(x, 3, h, w, u);
            let r = self.conv_first.forward(&un, lh, lw);
            nn::give(un);
            r
        } else {
            self.conv_first.forward(x, lh, lw)
        };
        let mut t = nn::take_copy(&feat);
        for b in &self.body {
            cancel.check()?;
            let n = b.forward(&t, lh, lw);
            nn::give(std::mem::replace(&mut t, n));
        }
        let (body, _, _) = self.conv_body.forward(&t, lh, lw);
        nn::give(t);
        let mut feat = feat;
        nn::add_inplace(&mut feat, &body);
        nn::give(body);
        cancel.check()?;
        // high-resolution part in stripes of `rows` network rows (about 48 MB per HR buffer)
        let rows = if stripe > 0 {
            stripe
        } else {
            ((48usize << 20) / (nf * 4 * 16 * lw).max(1)).max(8)
        }
        .min(lh);
        let (oh, ow) = (4 * lh, 4 * lw);
        let mut out = nn::take(3 * oh * ow);
        for r0 in (0..lh).step_by(rows) {
            cancel.check()?;
            let r1 = (r0 + rows).min(lh);
            let (s0, s1) = (r0.saturating_sub(HR_HALO), (r1 + HR_HALO).min(lh));
            let sh = s1 - s0;
            let mut sub = nn::take(nf * sh * lw);
            sub.par_chunks_mut(sh * lw).enumerate().for_each(|(c, d)| {
                d.copy_from_slice(&feat[c * lh * lw + s0 * lw..][..sh * lw]);
            });
            let hr = self.hr_stage(&sub, sh, lw);
            nn::give(sub);
            let (skip, rows_out) = (4 * (r0 - s0), 4 * (r1 - r0));
            for c in 0..3 {
                let src = &hr[c * 4 * sh * ow..][..4 * sh * ow];
                out[c * oh * ow + 4 * r0 * ow..][..rows_out * ow]
                    .copy_from_slice(&src[skip * ow..(skip + rows_out) * ow]);
            }
            nn::give(hr);
        }
        nn::give(feat);
        Ok(out)
    }

    /// `conv_up1` … `conv_last` on features `[nf, h, w]` → `[3, 4h, 4w]`.
    fn hr_stage(&self, feat: &[f32], lh: usize, lw: usize) -> Vec<f32> {
        let nf = self.conv_first.cout;
        let up = nn::upsample_nearest_2x(feat, nf, lh, lw);
        let (mut f, _, _) = self.conv_up1.forward(&up, 2 * lh, 2 * lw);
        nn::give(up);
        lrelu(&mut f);
        let up = nn::upsample_nearest_2x(&f, nf, 2 * lh, 2 * lw);
        nn::give(f);
        let (mut f, _, _) = self.conv_up2.forward(&up, 4 * lh, 4 * lw);
        nn::give(up);
        lrelu(&mut f);
        let (mut g, _, _) = self.conv_hr.forward(&f, 4 * lh, 4 * lw);
        nn::give(f);
        lrelu(&mut g);
        let (o, _, _) = self.conv_last.forward(&g, 4 * lh, 4 * lw);
        nn::give(g);
        o
    }

    /// Tiled inference: tiles of about `tile` network pixels (input pixels / [`Esrgan::unit`],
    /// evenly distributed) with `pad` network pixels of context. `input(y0, y1, x0, x1)` delivers
    /// the planes `[3, y1 - y0, x1 - x0]` of an input rectangle, `sink` receives every computed
    /// tile; only its core belongs into the result. Progress runs from `a` to `b` over the tiles
    /// (cancel checked between and inside the tiles). An input of at most one tile runs in one
    /// piece.
    #[allow(clippy::too_many_arguments)]
    pub fn run_tiles(
        &self,
        h: usize,
        w: usize,
        tile: usize,
        pad: usize,
        prog: &Progress,
        a: f32,
        b: f32,
        input: impl Fn(usize, usize, usize, usize) -> Vec<f32>,
        mut sink: impl FnMut(&TileBlock),
    ) -> Result<()> {
        let u = self.unshuffle;
        let s = self.scale();
        if h == 0 || w == 0 || !h.is_multiple_of(u) || !w.is_multiple_of(u) {
            return Err(AiError::Input(fp_i18n::tr!(
                "ai-err-d-esrgan-input",
                width = w,
                height = h,
                unit = u
            )));
        }
        let round = |v: usize| v.div_ceil(u) * u;
        let (tile, pad) = (tile.max(1) * u, pad * u);
        prog.step(a, b, 0.0)?;
        // evenly sized tiles (no thin last strip)
        let (nty, ntx) = (h.div_ceil(tile), w.div_ceil(tile));
        let (th, tw) = (round(h.div_ceil(nty)), round(w.div_ceil(ntx)));
        let n = (h.div_ceil(th) * w.div_ceil(tw)).max(1);
        let mut done = 0usize;
        for y0 in (0..h).step_by(th) {
            let y1 = (y0 + th).min(h);
            let (py0, py1) = (y0.saturating_sub(pad), (y1 + pad).min(h));
            for x0 in (0..w).step_by(tw) {
                let x1 = (x0 + tw).min(w);
                let (px0, px1) = (x0.saturating_sub(pad), (x1 + pad).min(w));
                let sub = input(py0, py1, px0, px1);
                let o = self.forward(&sub, py1 - py0, px1 - px0, &prog.cancel)?;
                drop(sub);
                sink(&TileBlock {
                    data: &o,
                    height: (py1 - py0) * s,
                    width: (px1 - px0) * s,
                    top: py0 * s,
                    left: px0 * s,
                    core: (y0 * s, y1 * s, x0 * s, x1 * s),
                });
                nn::give(o);
                done += 1;
                prog.step(a, b, done as f32 / n as f32)?;
            }
        }
        Ok(())
    }

    /// [`Esrgan::run_tiles`] on `x[3, h, w]` → `[3, h·s, w·s]` (not clamped).
    #[allow(clippy::too_many_arguments)]
    pub fn forward_tiled(
        &self,
        x: &[f32],
        h: usize,
        w: usize,
        tile: usize,
        pad: usize,
        prog: &Progress,
        a: f32,
        b: f32,
    ) -> Result<Vec<f32>> {
        if x.len() != 3 * h * w {
            return Err(AiError::Input(fp_i18n::tr!(
                "ai-err-d-esrgan-input",
                width = w,
                height = h,
                unit = self.unshuffle
            )));
        }
        let s = self.scale();
        let (oh, ow) = (h * s, w * s);
        let mut out = nn::take(3 * oh * ow);
        self.run_tiles(
            h,
            w,
            tile,
            pad,
            prog,
            a,
            b,
            |y0, y1, x0, x1| crop_planes(x, h, w, y0, y1, x0, x1),
            |t| {
                let (cy0, cy1, cx0, cx1) = t.core;
                for c in 0..3 {
                    let src = t.plane(c);
                    out[c * oh * ow + cy0 * ow..c * oh * ow + cy1 * ow]
                        .par_chunks_mut(ow)
                        .enumerate()
                        .for_each(|(i, drow)| {
                            let srow = &src[(cy0 + i - t.top) * t.width..][..t.width];
                            drow[cx0..cx1].copy_from_slice(&srow[cx0 - t.left..cx1 - t.left]);
                        });
                }
            },
        )?;
        Ok(out)
    }
}

/// One computed tile of [`Esrgan::run_tiles`] (output coordinates).
#[derive(Debug)]
pub struct TileBlock<'a> {
    /// Network output `[3, height, width]` of the padded tile (not clamped).
    pub data: &'a [f32],
    pub height: usize,
    pub width: usize,
    /// Output position of `data[.., 0, 0]`.
    pub top: usize,
    pub left: usize,
    /// Part of the result this tile is responsible for: `(y0, y1, x0, x1)`, exclusive.
    pub core: (usize, usize, usize, usize),
}

impl TileBlock<'_> {
    /// Channel `c` (`[height, width]`).
    pub fn plane(&self, c: usize) -> &[f32] {
        &self.data[c * self.height * self.width..(c + 1) * self.height * self.width]
    }
}

/// Rectangle `y0..y1 × x0..x1` of `[3, h, w]` planes.
fn crop_planes(
    x: &[f32],
    h: usize,
    w: usize,
    y0: usize,
    y1: usize,
    x0: usize,
    x1: usize,
) -> Vec<f32> {
    let (ph, pw) = (y1 - y0, x1 - x0);
    let mut sub = nn::take(3 * ph * pw);
    sub.par_chunks_mut(pw).enumerate().for_each(|(r, row)| {
        let (c, y) = (r / ph, r % ph);
        row.copy_from_slice(&x[c * h * w + (y0 + y) * w + x0..][..pw]);
    });
    sub
}

/// Upscaling of the alpha channel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AlphaMode {
    /// Alpha as a grey image through the same network (Real-ESRGAN default).
    #[default]
    Network,
    /// Alpha scaled bicubically (faster).
    Bicubic,
}

impl AlphaMode {
    /// From the technical name (`network`, `bicubic`).
    pub fn from_name(s: &str) -> Option<AlphaMode> {
        match s {
            "network" => Some(AlphaMode::Network),
            "bicubic" => Some(AlphaMode::Bicubic),
            _ => None,
        }
    }
}

/// Native scale of an upscaling model (`None` = not an upscaling model).
pub fn model_scale(id: ModelId) -> Option<usize> {
    match id {
        ModelId::RealEsrganX2 => Some(2),
        ModelId::RealEsrganX4 => Some(4),
        _ => None,
    }
}

/// Loads (or reuses) the network of `id`.
pub fn load(id: ModelId, prog: &Progress) -> Result<std::sync::Arc<Loaded>> {
    if !cache::is_loaded(id) {
        prog.set_stage(Stage::LoadModel);
    }
    let m = cache::model(id)?;
    prog.cancel.check()?;
    if !matches!(*m, Loaded::Esrgan(_)) {
        return Err(AiError::Input(fp_i18n::t("ai-err-d-no-esrgan")));
    }
    prog.set_stage(Stage::Analyze);
    Ok(m)
}

/// Planes `[3, h', w']` (0…1) of `r`, `h'`/`w'` rounded up to multiples of `u` by reflecting the
/// last rows/columns (`F.pad(mode='reflect')` of Real-ESRGAN). `pick` selects the value per pixel.
fn planes(
    img: &RgbaImage,
    r: IRect,
    u: usize,
    pick: impl Fn([u8; 4], usize) -> u8 + Sync,
) -> (Vec<f32>, usize, usize) {
    let (w, h) = (r.width() as usize, r.height() as usize);
    let (pw, ph) = (w.div_ceil(u) * u, h.div_ceil(u) * u);
    let refl = |i: usize, n: usize| -> usize {
        if i < n {
            i
        } else {
            // reflect without repeating the edge; tiny images fall back to the edge
            (2 * n).saturating_sub(i + 2).min(n - 1)
        }
    };
    let ib = img.bounds();
    let iw = img.width();
    let data = img.data();
    let mut out = vec![0.0f32; 3 * pw * ph];
    out.par_chunks_mut(pw).enumerate().for_each(|(row, orow)| {
        let (c, y) = (row / ph, row % ph);
        let sy = (r.top - ib.top) as usize + refl(y, h);
        for (x, v) in orow.iter_mut().enumerate() {
            let sx = (r.left - ib.left) as usize + refl(x, w);
            *v = pick(data[sy * iw + sx], c) as f32 / 255.0;
        }
    });
    (out, ph, pw)
}

fn to_u8(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// Bicubic weights (PyTorch, A = −0.75) of the four taps around fraction `t`.
fn cubic_weights(t: f32) -> [f32; 4] {
    let a = -0.75f32;
    let c1 = |t: f32| ((a + 2.0) * t - (a + 3.0)) * t * t + 1.0;
    let c2 = |t: f32| ((a * t - 5.0 * a) * t + 8.0 * a) * t - 4.0 * a;
    [c2(t + 1.0), c1(t), c1(1.0 - t), c2(2.0 - t)]
}

/// Bicubic alpha (×`s`) of the output pixels of region `r` into `px[..][3]` (edges clamped to
/// the image).
fn bicubic_alpha(img: &RgbaImage, r: IRect, s: usize, px: &mut [[u8; 4]]) {
    let b = img.bounds();
    let (iw, ih) = (img.width() as isize, img.height() as isize);
    let data = img.data();
    let rw = r.width() as usize * s;
    // weights and first source index (image coordinates) of output pixel `o`
    let taps = |o: usize, origin: i32| -> ([f32; 4], isize) {
        let f = (o as f32 + 0.5) / s as f32 - 0.5;
        let i = f.floor();
        (cubic_weights(f - i), origin as isize + i as isize - 1)
    };
    px.par_chunks_mut(rw.max(1))
        .enumerate()
        .for_each(|(y, row)| {
            let (wy, by) = taps(y, r.top - b.top);
            for (x, p) in row.iter_mut().enumerate() {
                let (wx, bx) = taps(x, r.left - b.left);
                let mut acc = 0.0f32;
                for (m, wyy) in wy.iter().enumerate() {
                    let yy = (by + m as isize).clamp(0, ih - 1) as usize;
                    let mut rsum = 0.0f32;
                    for (k, wxx) in wx.iter().enumerate() {
                        let xx = (bx + k as isize).clamp(0, iw - 1) as usize;
                        rsum += data[yy * iw as usize + xx][3] as f32 * wxx;
                    }
                    acc += rsum * wyy;
                }
                p[3] = acc.clamp(0.0, 255.0).round() as u8;
            }
        });
}

/// Upscales `img` (or only `region` of it) by the native factor of `model`. The region is computed
/// with [`TILE_PAD`] network px of surrounding image as context; the result has the size of the
/// region times the factor, origin (0, 0). Tiles are written straight into the 8-bit result (no
/// full-size float buffer). Stages: load model, then analyze with progress 0…1.
pub fn upscale_image(
    img: &RgbaImage,
    region: Option<IRect>,
    model: ModelId,
    alpha: AlphaMode,
    prog: &Progress,
) -> Result<RgbaImage> {
    let b = img.bounds();
    if region.map(|r| r.intersect(&b)).unwrap_or(b).is_empty() {
        return Err(AiError::Input(fp_i18n::t("ai-err-d-empty-region")));
    }
    let m = load(model, prog)?;
    let Loaded::Esrgan(net) = &*m else {
        return Err(AiError::Input(fp_i18n::t("ai-err-d-no-esrgan")));
    };
    upscale_with(net, img, region, alpha, prog)
}

/// [`upscale_image`] with an already loaded network.
pub fn upscale_with(
    net: &Esrgan,
    img: &RgbaImage,
    region: Option<IRect>,
    alpha: AlphaMode,
    prog: &Progress,
) -> Result<RgbaImage> {
    let b = img.bounds();
    let r = region.map(|r| r.intersect(&b)).unwrap_or(b);
    if r.is_empty() {
        return Err(AiError::Input(fp_i18n::t("ai-err-d-empty-region")));
    }
    prog.set_stage(Stage::Analyze);
    let (s, u) = (net.scale(), net.unit());
    let p = (TILE_PAD * u) as i32;
    let c = r.inflate(p, p).intersect(&b);
    let has_alpha = {
        let iw = img.width();
        let data = img.data();
        // over the context: transparency next to the region bleeds into its edge
        (c.top..c.bottom).any(|y| {
            let row = (y - b.top) as usize * iw;
            (c.left..c.right).any(|x| data[row + (x - b.left) as usize][3] < 255)
        })
    };
    let alpha_net = has_alpha && alpha == AlphaMode::Network;
    let split = if alpha_net { 0.5 } else { 1.0 };
    // result: output pixels of `r`; (oy, ox) = position of `r` in the output grid of `c`
    let (rw, rh) = (r.width() as usize * s, r.height() as usize * s);
    let (ox, oy) = ((r.left - c.left) as usize * s, (r.top - c.top) as usize * s);
    let mut px = vec![[0u8, 0, 0, 255]; rw * rh];
    // copies the core of a tile into `px` (channels via `put`)
    let mut write = |t: &TileBlock, put: &(dyn Fn(&mut [u8; 4], [f32; 3]) + Sync)| {
        let (cy0, cy1, cx0, cx1) = t.core;
        let (y0, y1) = (cy0.max(oy), cy1.min(oy + rh));
        let (x0, x1) = (cx0.max(ox), cx1.min(ox + rw));
        if y0 >= y1 || x0 >= x1 {
            return;
        }
        let (p0, p1, p2) = (t.plane(0), t.plane(1), t.plane(2));
        px[(y0 - oy) * rw..(y1 - oy) * rw]
            .par_chunks_mut(rw)
            .enumerate()
            .for_each(|(i, row)| {
                let sy = (y0 + i - t.top) * t.width;
                for x in x0..x1 {
                    let j = sy + x - t.left;
                    put(&mut row[x - ox], [p0[j], p1[j], p2[j]]);
                }
            });
    };
    let (rgb, ph, pw) = planes(img, c, u, |px, ch| px[ch]);
    nn::pool().install(|| {
        net.run_tiles(
            ph,
            pw,
            TILE,
            TILE_PAD,
            prog,
            0.0,
            split,
            |y0, y1, x0, x1| crop_planes(&rgb, ph, pw, y0, y1, x0, x1),
            |t| {
                write(t, &|o, v| {
                    o[0] = to_u8(v[0]);
                    o[1] = to_u8(v[1]);
                    o[2] = to_u8(v[2]);
                })
            },
        )
    })?;
    drop(rgb);
    if alpha_net {
        let (a, _, _) = planes(img, c, u, |px, _| px[3]);
        nn::pool().install(|| {
            net.run_tiles(
                ph,
                pw,
                TILE,
                TILE_PAD,
                prog,
                split,
                1.0,
                |y0, y1, x0, x1| crop_planes(&a, ph, pw, y0, y1, x0, x1),
                |t| {
                    write(t, &|o, v| {
                        let g = 0.299 * v[0].clamp(0.0, 1.0)
                            + 0.587 * v[1].clamp(0.0, 1.0)
                            + 0.114 * v[2].clamp(0.0, 1.0);
                        o[3] = to_u8(g);
                    })
                },
            )
        })?;
    } else if has_alpha {
        nn::pool().install(|| bicubic_alpha(img, r, s, &mut px));
    }
    prog.set(1.0);
    RgbaImage::from_raw(IRect::new(0, 0, rw as i32, rh as i32), px)
        .map_err(|e| AiError::Input(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Small deterministic pseudo-random numbers (xorshift).
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> f32 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            ((self.0 >> 40) as f32 / (1u64 << 24) as f32) - 0.5
        }
    }

    fn tensor(shape: &[usize], rng: &mut Rng, scale: f32) -> (Vec<usize>, Vec<f32>) {
        let n: usize = shape.iter().product();
        (shape.to_vec(), (0..n).map(|_| rng.next() * scale).collect())
    }

    /// Safetensors bytes of a small random RRDBNet (`nf` features, `gc` growth, `blocks` RRDBs).
    fn tiny_weights(cin: usize, nf: usize, gc: usize, blocks: usize, seed: u64) -> Vec<u8> {
        let mut rng = Rng(seed);
        let mut ts: Vec<(String, Vec<usize>, Vec<f32>)> = Vec::new();
        let mut conv = |name: &str, o: usize, i: usize, rng: &mut Rng| {
            let s = 1.0 / ((i * 9) as f32).sqrt();
            let (sh, d) = tensor(&[o, i, 3, 3], rng, 2.0 * s);
            ts.push((format!("{name}.weight"), sh, d));
            let (sh, d) = tensor(&[o], rng, 0.1);
            ts.push((format!("{name}.bias"), sh, d));
        };
        conv("conv_first", nf, cin, &mut rng);
        for b in 0..blocks {
            for r in 1..=3 {
                for k in 0..5 {
                    let o = if k == 4 { nf } else { gc };
                    conv(
                        &format!("body.{b}.rdb{r}.conv{}", k + 1),
                        o,
                        nf + k * gc,
                        &mut rng,
                    );
                }
            }
        }
        for n in ["conv_body", "conv_up1", "conv_up2", "conv_hr"] {
            conv(n, nf, nf, &mut rng);
        }
        conv("conv_last", 3, nf, &mut rng);
        let mut head = serde_json::Map::new();
        let mut data = Vec::new();
        for (name, shape, d) in &ts {
            let a = data.len();
            for v in d {
                data.extend_from_slice(&v.to_le_bytes());
            }
            head.insert(
                name.clone(),
                serde_json::json!({"dtype": "F32", "shape": shape, "data_offsets": [a, data.len()]}),
            );
        }
        let h = serde_json::to_vec(&serde_json::Value::Object(head)).unwrap_or_default();
        let mut out = (h.len() as u64).to_le_bytes().to_vec();
        out.extend_from_slice(&h);
        out.extend_from_slice(&data);
        out
    }

    fn tiny(cin: usize, blocks: usize) -> Esrgan {
        let w = Weights::parse(&tiny_weights(cin, 8, 4, blocks, 0x5eed + cin as u64))
            .unwrap_or_default();
        Esrgan::load(w).unwrap_or_else(|e| panic!("load: {e}"))
    }

    fn input(h: usize, w: usize, seed: u64) -> Vec<f32> {
        let mut r = Rng(seed);
        (0..3 * h * w).map(|_| r.next() + 0.5).collect()
    }

    #[test]
    fn unshuffle_matches_basicsr_layout() {
        // [1, 4, 4] → [4, 2, 2]: channel sy·2 + sx holds pixel (2y + sy, 2x + sx)
        let x: Vec<f32> = (0..16).map(|v| v as f32).collect();
        let o = pixel_unshuffle(&x, 1, 4, 4, 2);
        assert_eq!(&o[..4], &[0.0, 2.0, 8.0, 10.0]);
        assert_eq!(&o[4..8], &[1.0, 3.0, 9.0, 11.0]);
        assert_eq!(&o[8..12], &[4.0, 6.0, 12.0, 14.0]);
        assert_eq!(&o[12..16], &[5.0, 7.0, 13.0, 15.0]);
    }

    #[test]
    fn shapes_scale_and_determinism() {
        let x4 = tiny(3, 2);
        assert_eq!((x4.scale(), x4.unit(), x4.blocks()), (4, 1, 2));
        let x2 = tiny(12, 1);
        assert_eq!((x2.scale(), x2.unit(), x2.blocks()), (2, 2, 1));
        let c = Cancel::new();
        let (h, w) = (10, 14);
        let x = input(h, w, 7);
        let a = x4.forward(&x, h, w, &c).unwrap_or_default();
        assert_eq!(a.len(), 3 * h * 4 * w * 4);
        let b = x4.forward(&x, h, w, &c).unwrap_or_default();
        assert_eq!(a, b, "deterministic");
        assert!(a.iter().all(|v| v.is_finite()));
        let o2 = x2.forward(&x, h, w, &c).unwrap_or_default();
        assert_eq!(o2.len(), 3 * h * 2 * w * 2);
        // odd size for the ×2 network: clean error
        assert!(x2.forward(&input(9, 14, 1), 9, 14, &c).is_err());
        // wrong weights: clean error
        let wt = Weights::parse(&tiny_weights(5, 8, 4, 1, 3)).unwrap_or_default();
        assert!(Esrgan::load(wt).is_err());
    }

    /// Tiles with enough context give the same result as one pass (up to rounding).
    #[test]
    fn tiled_equals_untiled() {
        // receptive field of the tiny net (one RRDB = 17 convolutions at low resolution plus the
        // high-resolution convolutions): about 19 px for ×4, about 39 input px for ×2
        for (net, h, w, tile, pad) in [
            (tiny(3, 1), 60, 70, 16, 22),
            (tiny(12, 1), 120, 110, 15, 21),
        ] {
            let c = Cancel::new();
            let x = input(h, w, 11);
            let full = net.forward(&x, h, w, &c).unwrap_or_default();
            let prog = Progress::new();
            let tiled = net
                .forward_tiled(&x, h, w, tile, pad, &prog, 0.0, 1.0)
                .unwrap_or_default();
            assert_eq!(full.len(), tiled.len());
            assert_eq!(full.len(), 3 * h * w * net.scale() * net.scale());
            let d = full
                .iter()
                .zip(&tiled)
                .map(|(a, b)| (a - b).abs())
                .fold(0.0f32, f32::max);
            assert!(d < 1e-4, "max diff {d}");
            assert!((prog.get() - 1.0).abs() < 1e-6);
        }
    }

    /// High-resolution stripes (halo 2 rows) give the same result as one piece.
    #[test]
    fn stripes_equal_one_piece() {
        for net in [tiny(3, 1), tiny(12, 1)] {
            let (h, w) = (36, 22);
            let x = input(h, w, 5);
            let c = Cancel::new();
            let one = net
                .forward_striped(&x, h, w, &c, 10_000)
                .unwrap_or_default();
            for stripe in [1, 3, 7] {
                let st = net
                    .forward_striped(&x, h, w, &c, stripe)
                    .unwrap_or_default();
                assert_eq!(one.len(), st.len());
                let d = one
                    .iter()
                    .zip(&st)
                    .map(|(a, b)| (a - b).abs())
                    .fold(0.0f32, f32::max);
                assert!(d < 1e-5, "stripe {stripe}: {d}");
            }
        }
    }

    #[test]
    fn cancel_stops_tiles() {
        let net = tiny(3, 1);
        let prog = Progress::new();
        prog.cancel.cancel();
        let r = net.forward_tiled(&input(20, 20, 1), 20, 20, 8, 4, &prog, 0.0, 1.0);
        assert!(matches!(r, Err(AiError::Cancelled)));
        let r = net.forward(&input(20, 20, 1), 20, 20, &prog.cancel);
        assert!(matches!(r, Err(AiError::Cancelled)));
    }

    fn rgba(w: usize, h: usize, seed: u64, transparent: bool) -> RgbaImage {
        let mut r = Rng(seed);
        let px = (0..w * h)
            .map(|i| {
                let (x, y) = (i % w, i / w);
                let a = if transparent && x < w / 3 {
                    (y * 255 / h) as u8
                } else {
                    255
                };
                let mut v = |o: f32| ((r.next() * 0.3 + 0.5 + o) * 255.0).clamp(0.0, 255.0) as u8;
                [v(0.1), v(0.0), v(-0.1), a]
            })
            .collect();
        RgbaImage::from_raw(IRect::new(0, 0, w as i32, h as i32), px)
            .unwrap_or_else(|_| RgbaImage::empty_at(0, 0))
    }

    /// Image level: sizes, odd sizes (×2), alpha modes, region = crop of the whole result.
    #[test]
    fn image_upscale_sizes_alpha_and_region() {
        for (net, w, h) in [(tiny(3, 1), 30, 21), (tiny(12, 1), 31, 23)] {
            let s = net.scale();
            let img = rgba(w, h, 3, true);
            let prog = Progress::new();
            let full = upscale_with(&net, &img, None, AlphaMode::Network, &prog)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!((full.width(), full.height()), (w * s, h * s));
            assert!((prog.get() - 1.0).abs() < 1e-6);
            // opaque part stays opaque only with an opaque source; the transparent band keeps alpha < 255
            let bic = upscale_with(&net, &img, None, AlphaMode::Bicubic, &Progress::new())
                .unwrap_or_else(|e| panic!("{e}"));
            let at = |im: &RgbaImage, x: usize, y: usize| im.data()[y * im.width() + x];
            assert_eq!(at(&bic, 0, 0)[3], 0);
            assert_eq!(at(&bic, w * s - 1, h * s / 2)[3], 255);
            // RGB is the same in both alpha modes
            assert_eq!(at(&bic, 5, 7)[..3], at(&full, 5, 7)[..3]);
            // opaque source: alpha 255 everywhere
            let opaque = upscale_with(
                &net,
                &rgba(w, h, 4, false),
                None,
                AlphaMode::Network,
                &Progress::new(),
            )
            .unwrap_or_else(|e| panic!("{e}"));
            assert!(opaque.data().iter().all(|p| p[3] == 255));
            // region: same pixels as the crop of the whole result (context covers the receptive field)
            let reg = IRect::new(10, 6, 22, 18);
            let part = upscale_with(&net, &img, Some(reg), AlphaMode::Network, &Progress::new())
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!((part.width(), part.height()), (12 * s, 12 * s));
            let mut dmax = 0u8;
            for y in 0..12 * s {
                for x in 0..12 * s {
                    let a = at(&part, x, y);
                    let b = at(&full, x + 10 * s, y + 6 * s);
                    for c in 0..4 {
                        dmax = dmax.max(a[c].abs_diff(b[c]));
                    }
                }
            }
            assert!(dmax <= 1, "region differs by {dmax}");
        }
    }

    #[test]
    fn reflect_padding_for_odd_sizes() {
        let img = RgbaImage::from_raw(
            IRect::new(0, 0, 3, 1),
            vec![[0, 0, 0, 255], [51, 0, 0, 255], [102, 0, 0, 255]],
        )
        .unwrap_or_else(|_| RgbaImage::empty_at(0, 0));
        let (p, h, w) = planes(&img, img.bounds(), 2, |px, c| px[c]);
        assert_eq!((h, w), (2, 4));
        // row 0: 0, .2, .4, then reflected .2; row 1 reflects row 0 (single row → edge)
        let r: Vec<u8> = p[..4].iter().map(|v| to_u8(*v)).collect();
        assert_eq!(r, vec![0, 51, 102, 51]);
        assert_eq!(&p[..4], &p[4..8]);
    }
}
