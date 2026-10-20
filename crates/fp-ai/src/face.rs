//! Face detection for the batch step `ai.faceBlur` (BA-109): a classic Viola-Jones detector
//! (boosted Haar-like features on an integral image) in pure Rust.
//!
//! The classifier is OpenCV's stump-based frontal face cascade
//! `haarcascade_frontalface_default.xml` by Rainer Lienhart (Intel License Agreement for the Open
//! Source Computer Vision Library, Copyright (C) 2000, Intel Corporation, all rights reserved;
//! full notice in `crates/fp-ai/data/haar_frontalface_default.LICENSE.txt` and in the manual). It
//! is converted once into a compact binary form (`examples/haar_convert.rs`, 95 KB) and embedded
//! in the executable, so there is nothing to download.
//!
//! The evaluation follows the documented algorithm of the cascade format: grey value
//! `0.299 R + 0.587 G + 0.114 B`; an image pyramid (scale factor 1.1, bilinear) with a fixed
//! 24 × 24 window; window step 2 px below scale 2, 1 px above; every feature value is divided
//! by the standard deviation of the window interior (inset by 1 px); a stump adds its left leaf
//! when the value is below its threshold, else its right leaf; a stage rejects when its sum is
//! below the stage threshold. Overlapping hits are grouped (similarity 0.2, more than
//! `min_neighbors` hits per group), and groups lying inside a stronger group are dropped.

use crate::cancel::Cancel;
use crate::error::{AiError, Result};
use fp_raster::IRect;
use rayon::prelude::*;
use std::sync::OnceLock;

/// Embedded frontal face cascade (see the module documentation for origin and licence).
static FRONTAL_FACE_BIN: &[u8] = include_bytes!("../data/haar_frontalface_default.bin");

/// Pyramid step between two scales.
pub const SCALE_FACTOR: f64 = 1.1;
/// Hits a group needs beyond this number to count as a face (default; OpenCV's own default is 3,
/// which on the test photos leaves false hits with 4 neighbours on grass and a cup, while real
/// faces of 33 to 760 px collect 14 to 62 hits).
pub const MIN_NEIGHBORS: usize = 5;
/// Similarity of two hits for grouping (fraction of the smaller size).
pub const GROUP_EPS: f64 = 0.2;

/// One rectangle of a Haar-like feature (window coordinates, weight).
#[derive(Clone, Copy, Debug, PartialEq)]
struct HaarRect {
    x: u8,
    y: u8,
    w: u8,
    h: u8,
    weight: f32,
}

/// A weak classifier (decision stump on one feature).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Stump {
    feature: u16,
    threshold: f32,
    left: f32,
    right: f32,
}

/// One boosted stage.
#[derive(Clone, Debug, PartialEq)]
struct Stage {
    threshold: f32,
    stumps: Vec<Stump>,
}

/// A stump-based Haar cascade.
#[derive(Clone, Debug, PartialEq)]
pub struct Cascade {
    win_w: usize,
    win_h: usize,
    features: Vec<Vec<HaarRect>>,
    stages: Vec<Stage>,
}

struct Reader<'a> {
    b: &'a [u8],
    pos: usize,
}

impl Reader<'_> {
    fn take<const N: usize>(&mut self) -> Result<[u8; N]> {
        let s = self
            .b
            .get(self.pos..self.pos + N)
            .ok_or_else(|| AiError::Weights("face cascade truncated".into()))?;
        self.pos += N;
        let mut a = [0u8; N];
        a.copy_from_slice(s);
        Ok(a)
    }
    fn u8(&mut self) -> Result<u8> {
        Ok(self.take::<1>()?[0])
    }
    fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.take()?))
    }
    fn f32(&mut self) -> Result<f32> {
        let v = f32::from_le_bytes(self.take()?);
        if v.is_finite() {
            Ok(v)
        } else {
            Err(AiError::Weights("face cascade: value not finite".into()))
        }
    }
}

impl Cascade {
    /// Reads the binary form written by `examples/haar_convert.rs`.
    pub fn parse(bytes: &[u8]) -> Result<Cascade> {
        let bad = |s: &str| AiError::Weights(format!("face cascade: {s}"));
        if bytes.get(..8) != Some(b"FPHAAR01".as_slice()) {
            return Err(bad("wrong header"));
        }
        let mut r = Reader { b: bytes, pos: 8 };
        let (win_w, win_h) = (r.u16()? as usize, r.u16()? as usize);
        let (n_stages, n_feat) = (r.u16()? as usize, r.u16()? as usize);
        if !(4..=255).contains(&win_w) || !(4..=255).contains(&win_h) || n_stages == 0 {
            return Err(bad("bad window or stage count"));
        }
        let mut features = Vec::with_capacity(n_feat);
        for _ in 0..n_feat {
            let n = r.u8()? as usize;
            if !(1..=3).contains(&n) {
                return Err(bad("bad rect count"));
            }
            let mut rects = Vec::with_capacity(n);
            for _ in 0..n {
                let [x, y, w, h] = r.take::<4>()?;
                let weight = r.f32()?;
                if x as usize + w as usize > win_w || y as usize + h as usize > win_h {
                    return Err(bad("rect outside the window"));
                }
                rects.push(HaarRect { x, y, w, h, weight });
            }
            features.push(rects);
        }
        let mut stages = Vec::with_capacity(n_stages);
        for _ in 0..n_stages {
            let n = r.u16()? as usize;
            let threshold = r.f32()?;
            let mut stumps = Vec::with_capacity(n);
            for _ in 0..n {
                let feature = r.u16()?;
                if feature as usize >= features.len() {
                    return Err(bad("feature index out of range"));
                }
                stumps.push(Stump {
                    feature,
                    threshold: r.f32()?,
                    left: r.f32()?,
                    right: r.f32()?,
                });
            }
            stages.push(Stage { threshold, stumps });
        }
        if r.pos != bytes.len() {
            return Err(bad("trailing bytes"));
        }
        Ok(Cascade {
            win_w,
            win_h,
            features,
            stages,
        })
    }

    /// The embedded frontal face cascade (parsed once).
    pub fn frontal_face() -> Result<&'static Cascade> {
        static C: OnceLock<std::result::Result<Cascade, String>> = OnceLock::new();
        C.get_or_init(|| Cascade::parse(FRONTAL_FACE_BIN).map_err(|e| e.to_string()))
            .as_ref()
            .map_err(|e| AiError::Weights(e.clone()))
    }

    /// Window size `(w, h)` in pixels.
    pub fn window(&self) -> (usize, usize) {
        (self.win_w, self.win_h)
    }

    /// Number of stages and of weak classifiers.
    pub fn counts(&self) -> (usize, usize) {
        (
            self.stages.len(),
            self.stages.iter().map(|s| s.stumps.len()).sum(),
        )
    }
}

/// Grey value `0.299 R + 0.587 G + 0.114 B` (14-bit fixed point, rounded).
fn grey(p: [u8; 4]) -> u8 {
    ((p[0] as u32 * 4899 + p[1] as u32 * 9617 + p[2] as u32 * 1868 + 8192) >> 14) as u8
}

/// Bilinear resize of a grey image (pixel centres aligned).
fn resize_grey(src: &[u8], w: usize, h: usize, nw: usize, nh: usize) -> Vec<u8> {
    let (sx, sy) = (w as f64 / nw as f64, h as f64 / nh as f64);
    let xs: Vec<(usize, usize, f32)> = (0..nw)
        .map(|x| {
            let f = ((x as f64 + 0.5) * sx - 0.5).clamp(0.0, (w - 1) as f64);
            let x0 = f.floor() as usize;
            (x0, (x0 + 1).min(w - 1), (f - x0 as f64) as f32)
        })
        .collect();
    let mut out = vec![0u8; nw * nh];
    out.par_chunks_mut(nw).enumerate().for_each(|(y, row)| {
        let f = ((y as f64 + 0.5) * sy - 0.5).clamp(0.0, (h - 1) as f64);
        let y0 = f.floor() as usize;
        let y1 = (y0 + 1).min(h - 1);
        let fy = (f - y0 as f64) as f32;
        let (r0, r1) = (&src[y0 * w..y0 * w + w], &src[y1 * w..y1 * w + w]);
        for (o, &(x0, x1, fx)) in row.iter_mut().zip(&xs) {
            let a = r0[x0] as f32 + (r0[x1] as f32 - r0[x0] as f32) * fx;
            let b = r1[x0] as f32 + (r1[x1] as f32 - r1[x0] as f32) * fx;
            *o = (a + (b - a) * fy).round().clamp(0.0, 255.0) as u8;
        }
    });
    out
}

/// Integral image and integral of squares, `(w + 1) × (h + 1)`, wrapping `u32` arithmetic: the
/// totals may wrap on large images, but every window sum (at most 24 · 24 · 255²) is exact.
pub fn integral(grey: &[u8], w: usize, h: usize) -> (Vec<u32>, Vec<u32>) {
    let stride = w + 1;
    let mut sum = vec![0u32; stride * (h + 1)];
    let mut sq = vec![0u32; stride * (h + 1)];
    for y in 0..h {
        let (mut rs, mut rq) = (0u32, 0u32);
        for x in 0..w {
            let v = grey[y * w + x] as u32;
            rs = rs.wrapping_add(v);
            rq = rq.wrapping_add(v * v);
            let i = (y + 1) * stride + x + 1;
            sum[i] = sum[i - stride].wrapping_add(rs);
            sq[i] = sq[i - stride].wrapping_add(rq);
        }
    }
    (sum, sq)
}

/// Sum of the rectangle with corner offsets `o` (top-left, top-right, bottom-left, bottom-right)
/// at base index `p`.
#[inline]
fn rsum(t: &[u32], p: usize, o: &[usize; 4]) -> u32 {
    t[p + o[0]]
        .wrapping_sub(t[p + o[1]])
        .wrapping_sub(t[p + o[2]])
        .wrapping_add(t[p + o[3]])
}

fn offsets(x: usize, y: usize, w: usize, h: usize, stride: usize) -> [usize; 4] {
    [
        y * stride + x,
        y * stride + x + w,
        (y + h) * stride + x,
        (y + h) * stride + x + w,
    ]
}

/// Cascade prepared for one integral-image stride.
struct Prepared<'a> {
    c: &'a Cascade,
    /// Per feature: up to three rects (offsets, weight); unused rects have weight 0.
    feats: Vec<[([usize; 4], f32); 3]>,
    norm: [usize; 4],
    norm_area: f64,
}

impl<'a> Prepared<'a> {
    fn new(c: &'a Cascade, stride: usize) -> Prepared<'a> {
        let feats = c
            .features
            .iter()
            .map(|f| {
                let mut a = [([0usize; 4], 0f32); 3];
                for (slot, r) in a.iter_mut().zip(f) {
                    *slot = (
                        offsets(
                            r.x as usize,
                            r.y as usize,
                            r.w as usize,
                            r.h as usize,
                            stride,
                        ),
                        r.weight,
                    );
                }
                a
            })
            .collect();
        let norm = offsets(1, 1, c.win_w - 2, c.win_h - 2, stride);
        Prepared {
            c,
            feats,
            norm,
            norm_area: ((c.win_w - 2) * (c.win_h - 2)) as f64,
        }
    }

    /// Runs the cascade on the window at base index `p`: `Ok(())` = face, `Err(stage)` = rejected
    /// in that stage.
    fn run(&self, sum: &[u32], sq: &[u32], p: usize) -> std::result::Result<(), usize> {
        let s = rsum(sum, p, &self.norm) as f64;
        let q = rsum(sq, p, &self.norm) as f64;
        let nf = self.norm_area * q - s * s;
        let inv = if nf > 0.0 {
            (1.0 / nf.sqrt()) as f32
        } else {
            1.0
        };
        for (si, st) in self.c.stages.iter().enumerate() {
            let mut acc = 0f64;
            for k in &st.stumps {
                let f = &self.feats[k.feature as usize];
                let mut v = f[0].1 * rsum(sum, p, &f[0].0) as i32 as f32
                    + f[1].1 * rsum(sum, p, &f[1].0) as i32 as f32;
                if f[2].1 != 0.0 {
                    v += f[2].1 * rsum(sum, p, &f[2].0) as i32 as f32;
                }
                acc += if v * inv < k.threshold {
                    k.left
                } else {
                    k.right
                } as f64;
            }
            if acc < st.threshold as f64 {
                return Err(si);
            }
        }
        Ok(())
    }
}

/// Raw hits of `cascade` on a grey image (before grouping), in image pixels. Windows smaller
/// than `min_size` are skipped.
pub fn raw_hits(
    cascade: &Cascade,
    grey_img: &[u8],
    w: usize,
    h: usize,
    min_size: u32,
    cancel: &Cancel,
) -> Result<Vec<IRect>> {
    let (ww, wh) = (cascade.win_w, cascade.win_h);
    let mut hits = Vec::new();
    let mut factor = 1.0f64;
    loop {
        let (sw, sh) = (
            (w as f64 / factor).round() as usize,
            (h as f64 / factor).round() as usize,
        );
        if sw <= ww || sh <= wh {
            break;
        }
        let (win_w, win_h) = (
            (ww as f64 * factor).round() as i32,
            (wh as f64 * factor).round() as i32,
        );
        if win_w.min(win_h) >= min_size as i32 {
            cancel.check()?;
            let small;
            let g = if sw == w && sh == h {
                grey_img
            } else {
                small = resize_grey(grey_img, w, h, sw, sh);
                &small
            };
            let (sum, sq) = integral(g, sw, sh);
            let stride = sw + 1;
            let prep = Prepared::new(cascade, stride);
            let step = if factor > 2.0 { 1 } else { 2 };
            let rows: Vec<usize> = (0..=sh - wh).step_by(step).collect();
            let level: Vec<IRect> = rows
                .par_iter()
                .map(|&y| {
                    let mut out = Vec::new();
                    if cancel.is_cancelled() {
                        return out;
                    }
                    let mut x = 0;
                    while x + ww <= sw {
                        match prep.run(&sum, &sq, y * stride + x) {
                            Ok(()) => out.push(IRect::from_xywh(
                                (x as f64 * factor).round() as i32,
                                (y as f64 * factor).round() as i32,
                                win_w,
                                win_h,
                            )),
                            // rejected in the first stage: the neighbour is unlikely, skip it
                            Err(0) => x += step,
                            Err(_) => {}
                        }
                        x += step;
                    }
                    out
                })
                .flatten()
                .collect();
            hits.extend(level);
        }
        factor *= SCALE_FACTOR;
    }
    cancel.check()?;
    Ok(hits)
}

fn similar(a: &IRect, b: &IRect, eps: f64) -> bool {
    let delta = eps * ((a.width().min(b.width()) + a.height().min(b.height())) as f64) * 0.5;
    (a.left - b.left).abs() as f64 <= delta
        && (a.top - b.top).abs() as f64 <= delta
        && (a.right - b.right).abs() as f64 <= delta
        && (a.bottom - b.bottom).abs() as f64 <= delta
}

fn find(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}

/// Groups overlapping hits: similar hits (see [`GROUP_EPS`]) form a group, the group's rect is
/// the mean; groups with at most `min_neighbors` hits are dropped, as are groups lying inside a
/// group with more hits. Result: `(rect, hits)` sorted from left to right, top to bottom.
pub fn group_rects(rects: &[IRect], min_neighbors: usize, eps: f64) -> Vec<(IRect, usize)> {
    let n = rects.len();
    let mut parent: Vec<usize> = (0..n).collect();
    for i in 0..n {
        for j in i + 1..n {
            if similar(&rects[i], &rects[j], eps) {
                let (a, b) = (find(&mut parent, i), find(&mut parent, j));
                if a != b {
                    parent[b] = a;
                }
            }
        }
    }
    let mut acc: std::collections::BTreeMap<usize, ([i64; 4], usize)> = Default::default();
    for (i, r) in rects.iter().enumerate() {
        let e = acc.entry(find(&mut parent, i)).or_default();
        e.0[0] += r.left as i64;
        e.0[1] += r.top as i64;
        e.0[2] += r.width() as i64;
        e.0[3] += r.height() as i64;
        e.1 += 1;
    }
    let groups: Vec<(IRect, usize)> = acc
        .into_values()
        .map(|(s, k)| {
            let m = |v: i64| (v as f64 / k as f64).round() as i32;
            (IRect::from_xywh(m(s[0]), m(s[1]), m(s[2]), m(s[3])), k)
        })
        .collect();
    let mut out: Vec<(IRect, usize)> = groups
        .iter()
        .enumerate()
        .filter(|(i, (r1, n1))| {
            *n1 > min_neighbors
                && !groups.iter().enumerate().any(|(j, (r2, n2))| {
                    let (dx, dy) = (
                        (r2.width() as f64 * eps).round() as i32,
                        (r2.height() as f64 * eps).round() as i32,
                    );
                    j != *i
                        && *n2 > min_neighbors
                        && r1.left >= r2.left - dx
                        && r1.top >= r2.top - dy
                        && r1.right <= r2.right + dx
                        && r1.bottom <= r2.bottom + dy
                        && (*n2 > (*n1).max(3) || *n1 < 3)
                })
        })
        .map(|(_, g)| *g)
        .collect();
    out.sort_by_key(|(r, _)| (r.top, r.left));
    out
}

/// Options of [`detect_faces_with`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FaceOptions {
    /// Smallest face edge in pixels (below the 24 px window nothing smaller is found anyway).
    pub min_size: u32,
    /// A face needs more than this many overlapping hits (lower = more faces, more false hits).
    pub min_neighbors: usize,
}

impl Default for FaceOptions {
    fn default() -> Self {
        FaceOptions {
            min_size: 24,
            min_neighbors: MIN_NEIGHBORS,
        }
    }
}

/// Detects frontal faces in an RGBA image (`w × h`, rows top to bottom) with the default
/// grouping. `min_size` = smallest face edge in pixels. Returns the face rectangles in image
/// pixels (clipped to the image), top to bottom; an image without faces gives an empty list.
pub fn detect_faces(
    rgba: &[[u8; 4]],
    w: usize,
    h: usize,
    min_size: u32,
    cancel: &Cancel,
) -> Result<Vec<IRect>> {
    detect_faces_with(
        rgba,
        w,
        h,
        &FaceOptions {
            min_size,
            ..FaceOptions::default()
        },
        cancel,
    )
}

/// [`detect_faces`] with all options.
pub fn detect_faces_with(
    rgba: &[[u8; 4]],
    w: usize,
    h: usize,
    opt: &FaceOptions,
    cancel: &Cancel,
) -> Result<Vec<IRect>> {
    if w == 0 || h == 0 || rgba.len() != w * h {
        return Err(AiError::Input(format!(
            "face detection: {w} x {h} does not match {} pixels",
            rgba.len()
        )));
    }
    let cascade = Cascade::frontal_face()?;
    let g: Vec<u8> = rgba.par_iter().map(|&p| grey(p)).collect();
    let hits = raw_hits(cascade, &g, w, h, opt.min_size, cancel)?;
    let full = IRect::new(0, 0, w as i32, h as i32);
    Ok(group_rects(&hits, opt.min_neighbors, GROUP_EPS)
        .into_iter()
        .map(|(r, _)| r.intersect(&full))
        .filter(|r| !r.is_empty())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_cascade_parses() {
        let c = Cascade::frontal_face().expect("cascade");
        assert_eq!(c.window(), (24, 24));
        assert_eq!(c.counts(), (25, 2913));
        // first stump of the first stage (values of the published cascade)
        let s = c.stages[0].stumps[0];
        assert_eq!(s.feature, 0);
        assert_eq!(s.threshold as f64, -3.151_199_966_669_082_6e-2);
        assert_eq!(c.features[0].len(), 2);
        assert_eq!(
            c.features[0][0],
            HaarRect {
                x: 6,
                y: 4,
                w: 12,
                h: 9,
                weight: -1.0
            }
        );
        assert_eq!(c.stages[0].threshold as f64, -5.042_550_086_975_098);
    }

    #[test]
    fn damaged_cascade_is_an_error() {
        assert!(Cascade::parse(b"nope").is_err());
        let mut b = FRONTAL_FACE_BIN.to_vec();
        b.truncate(b.len() - 3);
        assert!(Cascade::parse(&b).is_err());
        b = FRONTAL_FACE_BIN.to_vec();
        b.push(0);
        assert!(Cascade::parse(&b).is_err());
    }

    #[test]
    fn integral_sums_match_direct_sums() {
        let (w, h) = (7usize, 5usize);
        let g: Vec<u8> = (0..w * h).map(|i| ((i * 37 + 11) % 256) as u8).collect();
        let (sum, sq) = integral(&g, w, h);
        let stride = w + 1;
        for (x, y, rw, rh) in [(0, 0, 7, 5), (1, 1, 3, 2), (6, 4, 1, 1), (2, 0, 4, 5)] {
            let o = offsets(x, y, rw, rh, stride);
            let (mut s, mut q) = (0u32, 0u32);
            for yy in y..y + rh {
                for xx in x..x + rw {
                    let v = g[yy * w + xx] as u32;
                    s += v;
                    q += v * v;
                }
            }
            assert_eq!(rsum(&sum, 0, &o), s);
            assert_eq!(rsum(&sq, 0, &o), q);
        }
    }

    #[test]
    fn integral_window_sums_survive_wrapping() {
        // 300 × 300 white: the total of squares (300² · 255² ≈ 5.9e9) wraps u32
        let (w, h) = (300usize, 300usize);
        let g = vec![255u8; w * h];
        let (sum, sq) = integral(&g, w, h);
        let o = offsets(0, 0, 24, 24, w + 1);
        let p = 270 * (w + 1) + 270;
        assert_eq!(rsum(&sum, p, &o), 576 * 255);
        assert_eq!(rsum(&sq, p, &o), 576 * 255 * 255);
    }

    #[test]
    fn grouping_needs_neighbours_and_drops_inner_groups() {
        let r = |x, y, s| IRect::from_xywh(x, y, s, s);
        // four similar hits around (100, 100), size 50 → one face, mean rect
        let mut v = vec![
            r(100, 100, 50),
            r(102, 101, 50),
            r(99, 98, 52),
            r(101, 100, 48),
        ];
        // two lonely hits elsewhere → dropped (≤ 3 neighbours)
        v.push(r(300, 300, 40));
        v.push(r(302, 301, 40));
        let g = group_rects(&v, 3, 0.2);
        assert_eq!(g.len(), 1, "{g:?}");
        assert_eq!(g[0].1, 4);
        assert_eq!(g[0].0, IRect::from_xywh(101, 100, 50, 50));
        // a weaker group inside a strong one is dropped
        let mut v2 = v.clone();
        v2.extend([
            r(110, 110, 20),
            r(111, 110, 20),
            r(110, 111, 20),
            r(111, 111, 20),
        ]);
        v2.extend([r(100, 100, 50), r(101, 101, 50)]);
        let g2 = group_rects(&v2, 3, 0.2);
        assert_eq!(g2.len(), 1, "{g2:?}");
        // without neighbours required every distinct hit survives
        assert_eq!(group_rects(&[r(0, 0, 30), r(200, 0, 30)], 0, 0.2).len(), 2);
    }

    #[test]
    fn flat_and_noise_images_have_no_faces() {
        let c = Cancel::new();
        let flat = vec![[128u8, 128, 128, 255]; 200 * 150];
        assert!(detect_faces(&flat, 200, 150, 24, &c).unwrap().is_empty());
        let mut s = 12345u32;
        let noise: Vec<[u8; 4]> = (0..160 * 120)
            .map(|_| {
                s ^= s << 13;
                s ^= s >> 17;
                s ^= s << 5;
                let v = (s & 255) as u8;
                [v, v, v, 255]
            })
            .collect();
        assert!(detect_faces(&noise, 160, 120, 24, &c).unwrap().is_empty());
        // too small for one window
        assert!(
            detect_faces(&flat[..20 * 20], 20, 20, 24, &c)
                .unwrap()
                .is_empty()
        );
        assert!(detect_faces(&flat[..10], 20, 20, 24, &c).is_err());
    }

    #[test]
    fn cancel_stops_detection() {
        let c = Cancel::new();
        c.cancel();
        let flat = vec![[128u8, 128, 128, 255]; 100 * 100];
        assert!(matches!(
            detect_faces(&flat, 100, 100, 24, &c),
            Err(AiError::Cancelled)
        ));
    }

    fn load_jpeg(name: &str) -> (Vec<[u8; 4]>, usize, usize) {
        let p = std::path::Path::new(name);
        let p = if p.is_absolute() {
            p.to_path_buf()
        } else {
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("testdata")
                .join(name)
        };
        let bytes = std::fs::read(&p).expect("test photo");
        let img = fp_io::jpeg_in::decode(&bytes).expect("decode").image;
        (img.data().to_vec(), img.width(), img.height())
    }

    fn inside(f: &IRect, x: i32, y: i32) -> bool {
        f.left <= x && x < f.right && f.top <= y && y < f.bottom
    }

    /// The portrait in the test data: exactly one face, covering eyes, nose and mouth.
    #[test]
    fn finds_the_face_in_the_portrait() {
        let (px, w, h) = load_jpeg("person_haare.jpg");
        let t = std::time::Instant::now();
        let faces = detect_faces(&px, w, h, 24, &Cancel::new()).expect("detect");
        eprintln!("{w} x {h}: {faces:?} in {:?}", t.elapsed());
        assert_eq!(faces.len(), 1, "{faces:?}");
        // eyes at about (470, 760) and (750, 835), nose tip (560, 980), mouth (520, 1110)
        for (x, y) in [(470, 760), (750, 835), (560, 980), (520, 1110)] {
            assert!(inside(&faces[0], x, y), "{:?} misses ({x}, {y})", faces[0]);
        }
        assert!(faces[0].width() < 900, "{faces:?}");
        // a minimum size above the face finds nothing
        let none = detect_faces(&px, w, h, 1000, &Cancel::new()).expect("detect");
        assert!(none.is_empty(), "{none:?}");
    }

    /// The same portrait scaled down to 1/4, 1/12 and 1/24 (face ≈ 190, 65 and 33 px).
    #[test]
    fn finds_small_faces() {
        let (px, w, h) = load_jpeg("person_haare.jpg");
        let g: Vec<u8> = px.iter().map(|&p| grey(p)).collect();
        let c = Cascade::frontal_face().expect("cascade");
        for d in [4usize, 12, 24] {
            let (nw, nh) = (w / d, h / d);
            let small = resize_grey(&g, w, h, nw, nh);
            let hits = raw_hits(c, &small, nw, nh, 24, &Cancel::new()).expect("hits");
            let faces = group_rects(&hits, MIN_NEIGHBORS, GROUP_EPS);
            assert_eq!(faces.len(), 1, "1/{d}: {faces:?}");
            let (cx, cy) = ((520 / d) as i32, (900 / d) as i32);
            assert!(inside(&faces[0].0, cx, cy), "1/{d}: {faces:?}");
        }
    }

    /// The other test photos (dog, cat, cup) contain no human face.
    #[test]
    fn no_faces_in_animal_and_product_photos() {
        for f in ["hund_gras.jpg", "katze.jpg", "produkt_tasse.jpg"] {
            let (px, w, h) = load_jpeg(f);
            let faces = detect_faces(&px, w, h, 24, &Cancel::new()).expect("detect");
            assert!(faces.is_empty(), "{f}: {faces:?}");
        }
    }

    /// Own photos: `FP_FACE_PHOTO=<jpeg> cargo test -p fp-ai face -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn faces_in_photo_from_env() {
        let Ok(p) = std::env::var("FP_FACE_PHOTO") else {
            return;
        };
        let (px, w, h) = load_jpeg(&p);
        let t = std::time::Instant::now();
        let faces = detect_faces(&px, w, h, 24, &Cancel::new()).expect("detect");
        eprintln!(
            "{p}: {w} x {h}, {} faces in {:?}: {faces:?}",
            faces.len(),
            t.elapsed()
        );
    }
}
