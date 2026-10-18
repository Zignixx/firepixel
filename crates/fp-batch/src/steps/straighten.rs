//! Automatic straightening for `perspective` (B-4, `docs/BATCH.md` §3.3).
//!
//! Detection: gradient orientation histogram. The image is scaled to at most
//! [`DETECT_SIZE`] px, Sobel gradients of the luminance give an edge direction per pixel; strong
//! edges (magnitude ≥ 25 % of the 98th percentile) vote into two magnitude-weighted histograms
//! (0.1° bins): near-horizontal lines (tilt φ = dy/dx angle) and near-vertical lines (tilt
//! ψ = dx/dy angle), both limited to ±`max_angle`. The histograms are smoothed (±0.5°), the peak is
//! refined with the weighted mean of the votes within ±1° of it. A family counts only when it
//! holds at least [`MIN_SHARE`] of the edge weight.
//!
//! Correction (as a source quad for the projective sampler, so the result is one resampling):
//! - `rotate`: one angle (weighted mean of φ and −ψ) → rotation about the centre;
//! - `shear`: φ and ψ separately → shear `x' = x + tan ψ·(y − cy)`, `y' = y + tan φ·(x − cx)`
//!   (keystone-like correction of converging verticals/horizontals is not modelled).
//!
//! With `crop` the output rectangle is shrunk about the centre until no corner falls outside the
//! source (no transparent wedges); the output keeps the source size in pixels either way.

use fp_raster::RgbaImage;

/// Longest side used for the detection.
pub const DETECT_SIZE: u32 = 800;
/// Share of all edge weight a line family needs to count.
pub const MIN_SHARE: f64 = 0.05;

/// Detected tilt of the line families in degrees (`None` = no such lines found).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Skew {
    /// Near-horizontal lines: angle of dy/dx (positive = falling to the right, y down).
    pub horizontal: Option<f64>,
    /// Near-vertical lines: angle of dx/dy (positive = leaning to the right at the bottom).
    pub vertical: Option<f64>,
    /// Edge weight of the families (for the combined rotation).
    pub weights: (f64, f64),
}

impl Skew {
    /// Single rotation angle (degrees) or `None`.
    pub fn rotation(&self) -> Option<f64> {
        match (self.horizontal, self.vertical) {
            (Some(h), Some(v)) => {
                let (wh, wv) = self.weights;
                Some((h * wh - v * wv) / (wh + wv).max(1e-12))
            }
            (Some(h), None) => Some(h),
            (None, Some(v)) => Some(-v),
            (None, None) => None,
        }
    }
}

fn luminance(img: &RgbaImage) -> (Vec<f32>, usize, usize) {
    let (w, h) = (img.width() as u32, img.height() as u32);
    let long = w.max(h).max(1);
    let small;
    let src = if long > DETECT_SIZE {
        let s = DETECT_SIZE as f64 / long as f64;
        small = fp_raster::resample::resize(
            img,
            ((w as f64 * s).round() as u32).max(1),
            ((h as f64 * s).round() as u32).max(1),
            fp_raster::Sampler::Bilinear,
        );
        &small
    } else {
        img
    };
    let (sw, sh) = (src.width(), src.height());
    let l = src
        .data()
        .iter()
        .map(|p| {
            // over white, so transparent areas are flat
            let a = p[3] as f32 / 255.0;
            let y = 0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32;
            y * a + 255.0 * (1.0 - a)
        })
        .collect();
    (l, sw, sh)
}

fn peak(hist: &[(f64, f64)], max: f64) -> Option<(f64, f64)> {
    const BIN: f64 = 0.1;
    let n = (2.0 * max / BIN).ceil() as usize + 1;
    let mut bins = vec![0f64; n];
    for &(a, w) in hist {
        let i = ((a + max) / BIN).round() as isize;
        if i >= 0 && (i as usize) < n {
            bins[i as usize] += w;
        }
    }
    // smooth ±0.5°
    let r = 5isize;
    let sm: Vec<f64> = (0..n as isize)
        .map(|i| {
            (-r..=r)
                .filter_map(|k| bins.get((i + k) as usize).filter(|_| i + k >= 0))
                .sum()
        })
        .collect();
    let (pi, _) = sm.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1))?;
    let p = pi as f64 * BIN - max;
    let (mut s, mut ws) = (0.0, 0.0);
    for &(a, w) in hist {
        if (a - p).abs() <= 1.0 {
            s += a * w;
            ws += w;
        }
    }
    (ws > 0.0).then(|| (s / ws, hist.iter().map(|x| x.1).sum()))
}

/// Detects the tilt of the dominant near-horizontal and near-vertical lines (±`max_angle`°).
pub fn detect(img: &RgbaImage, max_angle: f64) -> Skew {
    let max = max_angle.clamp(0.1, 45.0);
    let (l, w, h) = luminance(img);
    if w < 3 || h < 3 {
        return Skew::default();
    }
    let at = |x: usize, y: usize| l[y * w + x];
    let mut grads: Vec<(f32, f32, f32)> = Vec::with_capacity((w - 2) * (h - 2));
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let gx = (at(x + 1, y - 1) + 2.0 * at(x + 1, y) + at(x + 1, y + 1))
                - (at(x - 1, y - 1) + 2.0 * at(x - 1, y) + at(x - 1, y + 1));
            let gy = (at(x - 1, y + 1) + 2.0 * at(x, y + 1) + at(x + 1, y + 1))
                - (at(x - 1, y - 1) + 2.0 * at(x, y - 1) + at(x + 1, y - 1));
            let m = (gx * gx + gy * gy).sqrt();
            if m > 0.0 {
                grads.push((gx, gy, m));
            }
        }
    }
    if grads.is_empty() {
        return Skew::default();
    }
    let mut mags: Vec<f32> = grads.iter().map(|g| g.2).collect();
    let k = ((mags.len() as f64 * 0.98) as usize).min(mags.len() - 1);
    let (_, p98, _) = mags.select_nth_unstable_by(k, |a, b| a.total_cmp(b));
    let thr = (*p98 * 0.25).max(8.0);
    let (mut hor, mut ver) = (Vec::new(), Vec::new());
    let mut total = 0.0;
    for &(gx, gy, m) in &grads {
        if m < thr {
            continue;
        }
        let (gx, gy, m) = (gx as f64, gy as f64, m as f64);
        total += m;
        if gy.abs() >= gx.abs() {
            // near-horizontal line: direction (−gy, gx) → tilt atan(dy/dx) = −atan(gx/gy)
            let a = -(gx / gy).atan().to_degrees();
            if a.abs() <= max {
                hor.push((a, m));
            }
        } else {
            // near-vertical line: tilt atan(dx/dy) = atan(−gy/gx)
            let a = (-gy / gx).atan().to_degrees();
            if a.abs() <= max {
                ver.push((a, m));
            }
        }
    }
    let fam = |v: &[(f64, f64)]| -> (Option<f64>, f64) {
        match peak(v, max) {
            Some((a, wsum)) if wsum >= MIN_SHARE * total => (Some(a), wsum),
            _ => (None, 0.0),
        }
    };
    let (horizontal, wh) = fam(&hor);
    let (vertical, wv) = fam(&ver);
    Skew {
        horizontal,
        vertical,
        weights: (wh, wv),
    }
}

/// Source quad (TL, TR, BR, BL in pixels) that straightens an image of `w × h`.
/// `rotate`: one angle; otherwise shear with the two family angles. `crop`: shrink so that no
/// corner lies outside the source.
pub fn quad(w: f64, h: f64, skew: &Skew, rotate: bool, crop: bool) -> [(f64, f64); 4] {
    let (cx, cy) = (w / 2.0, h / 2.0);
    let corners = [(0.0, 0.0), (w, 0.0), (w, h), (0.0, h)];
    if rotate {
        let a = skew.rotation().unwrap_or(0.0).to_radians();
        let (s, c) = a.sin_cos();
        let k = if crop {
            let (sa, ca) = (s.abs(), c.abs());
            // one pixel of margin so the bilinear samples never reach outside
            ((w - 2.0).max(1.0) / (w * ca + h * sa))
                .min((h - 2.0).max(1.0) / (w * sa + h * ca))
                .min(1.0)
        } else {
            1.0
        };
        corners.map(|(x, y)| {
            let (dx, dy) = ((x - cx) * k, (y - cy) * k);
            (cx + dx * c - dy * s, cy + dx * s + dy * c)
        })
    } else {
        let tp = skew.horizontal.unwrap_or(0.0).to_radians().tan();
        let tv = skew.vertical.unwrap_or(0.0).to_radians().tan();
        let k = if crop {
            ((w - 2.0).max(1.0) / (w + tv.abs() * h))
                .min((h - 2.0).max(1.0) / (h + tp.abs() * w))
                .min(1.0)
        } else {
            1.0
        };
        corners.map(|(x, y)| {
            let (dx, dy) = ((x - cx) * k, (y - cy) * k);
            (cx + dx + tv * dy, cy + dy + tp * dx)
        })
    }
}
