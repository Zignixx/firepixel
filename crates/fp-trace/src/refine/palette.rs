//! Farben einrasten: jedes Pixel bekommt eine Palettennummer (oder [`NONE`] = transparent).
//!
//! Palette (Schalter `snap_colors`): Grundlage sind nur **flache** Pixel (ähnlich zu mindestens 2
//! der 8 Nachbarn, also auch 1 px dünne Linien); Kantenpixel der Kantenglättung und JPEG-Säume tragen nichts bei. Ein
//! Histogramm (5 Bit je Kanal) wird nach Häufigkeit abgearbeitet: Farben nahe einer schon
//! gewählten (Abstand ≤ `color_tolerance`) werden mit ihr gemittelt, neue Farben brauchen eine
//! Mindestmenge und dürfen keine Mischung zweier gewählter Farben sein (außer sie bilden große,
//! ganz flache Flächen). Zwei Ausgleichsrunden (k-Means über die Fächer) liefern die sauberen
//! Mittelwerte der Originalfarben.
//!
//! Zuordnung: flache Pixel nahe einer Palettenfarbe nehmen sie direkt; alle anderen (Kanten)
//! wählen die nächste Farbe **unter den Farben ihrer 5×5-Nachbarschaft** (weiche Kanten: bis 17×17) – eine Mischung aus Blau
//! und Weiß wird also Blau oder Weiß, nie eine dritte, zufällig ähnliche Farbe.

use super::NONE;
use crate::{ColorMode, TraceOptions, luma, over_white, quantize};
use fp_raster::RgbaImage;
use std::collections::HashMap;

pub(crate) struct Labels {
    /// Palettennummer je Pixel oder [`NONE`].
    pub label: Vec<u32>,
    pub palette: Vec<[u8; 3]>,
    /// Deckende Farbe je Pixel (für Mittelwerte, wenn `exact = false`).
    pub rgb: Vec<[u8; 3]>,
    /// Flächenfarbe = Palettenfarbe (sonst Mittelwert der Pixel der Fläche).
    pub exact: bool,
}

pub(crate) fn build(img: &RgbaImage, o: &TraceOptions) -> Labels {
    let (w, h) = (img.width(), img.height());
    let n = w * h;
    let mut rgb = vec![[0u8; 3]; n];
    let mut opaque = vec![false; n];
    for (i, p) in img.as_bytes().as_chunks::<4>().0.iter().enumerate() {
        let mut q = *p;
        if o.ignore_transparent && q[3] < o.alpha_threshold {
            continue;
        }
        if !o.ignore_transparent {
            over_white(&mut q);
        }
        rgb[i] = [q[0], q[1], q[2]];
        opaque[i] = true;
    }
    match o.color_mode {
        ColorMode::BlackWhite => {
            let label = (0..n)
                .map(|i| {
                    if opaque[i] && luma(rgb[i][0], rgb[i][1], rgb[i][2]) < o.threshold as u32 {
                        0
                    } else {
                        NONE
                    }
                })
                .collect();
            Labels {
                label,
                palette: vec![[0, 0, 0]],
                rgb,
                exact: true,
            }
        }
        ColorMode::Color if o.refine.snap_colors => {
            let (label, palette) = snap(&rgb, &opaque, w, h, o);
            Labels {
                label,
                palette,
                rgb,
                exact: true,
            }
        }
        ColorMode::Color => {
            let (label, palette) = plain(&rgb, &opaque, o);
            Labels {
                label,
                palette,
                rgb,
                exact: false,
            }
        }
    }
}

/// Ohne Einrasten: Median-Cut auf `colors` bzw. `color_precision` Bit, gleiche Farben = gleiche Nummer.
fn plain(rgb: &[[u8; 3]], opaque: &[bool], o: &TraceOptions) -> (Vec<u32>, Vec<[u8; 3]>) {
    let mut px: Vec<u8> = rgb.iter().flat_map(|c| [c[0], c[1], c[2], 255]).collect();
    let skip: Vec<bool> = opaque.iter().map(|&b| !b).collect();
    if o.colors >= 2 {
        quantize::reduce(&mut px, &skip, o.colors as usize);
    } else {
        let shift = 8 - o.color_precision.clamp(1, 8) as u32;
        let half = if shift > 0 { 1u8 << (shift - 1) } else { 0 };
        for c in px.as_chunks_mut::<4>().0 {
            for v in &mut c[..3] {
                *v = ((*v >> shift) << shift) | half;
            }
        }
    }
    let mut map: HashMap<[u8; 3], u32> = HashMap::new();
    let mut palette = Vec::new();
    let label = px
        .as_chunks::<4>()
        .0
        .iter()
        .zip(opaque)
        .map(|(c, &op)| {
            if !op {
                return NONE;
            }
            let k = [c[0], c[1], c[2]];
            *map.entry(k).or_insert_with(|| {
                palette.push(k);
                (palette.len() - 1) as u32
            })
        })
        .collect();
    (label, palette)
}

fn maxdiff(a: [u8; 3], b: [u8; 3]) -> i32 {
    (0..3)
        .map(|k| (a[k] as i32 - b[k] as i32).abs())
        .max()
        .unwrap_or(0)
}

fn d2(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|k| (a[k] - b[k]).powi(2)).sum()
}

#[derive(Clone, Copy, Default)]
struct Bin {
    sum: [u64; 3],
    semi: u32,
    strict: u32,
}

impl Bin {
    fn mean(&self) -> [f64; 3] {
        let n = self.semi.max(1) as f64;
        [
            self.sum[0] as f64 / n,
            self.sum[1] as f64 / n,
            self.sum[2] as f64 / n,
        ]
    }
}

#[derive(Clone, Copy, Default)]
struct Center {
    sum: [f64; 3],
    n: f64,
}

impl Center {
    fn add(&mut self, b: &Bin) {
        for k in 0..3 {
            self.sum[k] += b.sum[k] as f64;
        }
        self.n += b.semi as f64;
    }
    fn mean(&self) -> [f64; 3] {
        let n = self.n.max(1.0);
        [self.sum[0] / n, self.sum[1] / n, self.sum[2] / n]
    }
}

fn nearest(centers: &[[f64; 3]], c: [f64; 3]) -> Option<(usize, f64)> {
    centers
        .iter()
        .enumerate()
        .map(|(k, m)| (k, d2(*m, c)))
        .min_by(|a, b| a.1.total_cmp(&b.1))
}

/// Liegt c (fast) auf der Strecke zwischen zwei Palettenfarben (Kantenmischung)?
fn is_blend(c: [f64; 3], centers: &[[f64; 3]], lim: f64) -> bool {
    for i in 0..centers.len() {
        for j in i + 1..centers.len() {
            let (a, b) = (centers[i], centers[j]);
            let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let l2: f64 = ab.iter().map(|v| v * v).sum();
            if l2 < 1.0 {
                continue;
            }
            let t = ((c[0] - a[0]) * ab[0] + (c[1] - a[1]) * ab[1] + (c[2] - a[2]) * ab[2]) / l2;
            if !(0.06..=0.94).contains(&t) {
                continue;
            }
            let q = [a[0] + t * ab[0], a[1] + t * ab[1], a[2] + t * ab[2]];
            if d2(c, q) <= lim * lim {
                return true;
            }
        }
    }
    false
}

fn snap(
    rgb: &[[u8; 3]],
    opaque: &[bool],
    w: usize,
    h: usize,
    o: &TraceOptions,
) -> (Vec<u32>, Vec<[u8; 3]>) {
    let n = w * h;
    let tol = o.refine.color_tolerance;
    let ft = (tol * 0.6).clamp(12.0, 48.0) as i32;
    let mut semi = vec![false; n];
    let mut total = 0usize;
    let mut bins = vec![Bin::default(); 32768];
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if !opaque[i] {
                continue;
            }
            total += 1;
            let c = rgb[i];
            let (mut sim, mut tot) = (0, 0);
            for dy in -1i32..=1 {
                let yy = y as i32 + dy;
                if yy < 0 || yy >= h as i32 {
                    continue;
                }
                for dx in -1i32..=1 {
                    let xx = x as i32 + dx;
                    if (dx == 0 && dy == 0) || xx < 0 || xx >= w as i32 {
                        continue;
                    }
                    let j = yy as usize * w + xx as usize;
                    if opaque[j] {
                        tot += 1;
                        if maxdiff(c, rgb[j]) <= ft {
                            sim += 1;
                        }
                    }
                }
            }
            if sim >= 2 {
                semi[i] = true;
                let b = &mut bins[((c[0] as usize >> 3) << 10)
                    | ((c[1] as usize >> 3) << 5)
                    | (c[2] as usize >> 3)];
                for (s, &v) in b.sum.iter_mut().zip(&c) {
                    *s += v as u64;
                }
                b.semi += 1;
                if tot >= 5 && sim == tot {
                    b.strict += 1;
                }
            }
        }
    }
    let mut cand: Vec<usize> = (0..bins.len()).filter(|&b| bins[b].semi > 0).collect();
    cand.sort_by(|&a, &b| {
        (bins[b].strict, bins[b].semi, a).cmp(&(bins[a].strict, bins[a].semi, b))
    });
    let speck = ((o.filter_speckle * o.filter_speckle) as usize).min((total / 200).max(2));
    let min_semi = (total / 5000).max(speck).max(4) as u32;
    let big = (total / 100).max(4 * speck).max(16) as u32;
    let max_colors = if o.colors >= 2 {
        o.colors as usize
    } else {
        256
    };
    let greedy = |min_semi: u32| -> Vec<Center> {
        let mut centers: Vec<Center> = Vec::new();
        for &bi in &cand {
            let bin = bins[bi];
            let c = bin.mean();
            let means: Vec<[f64; 3]> = centers.iter().map(Center::mean).collect();
            if let Some((k, dd)) = nearest(&means, c)
                && dd <= tol * tol
            {
                centers[k].add(&bin);
                continue;
            }
            if bin.semi < min_semi || centers.len() >= max_colors {
                continue;
            }
            if bin.strict < big && is_blend(c, &means, 0.5 * tol) {
                continue;
            }
            let mut nc = Center::default();
            nc.add(&bin);
            centers.push(nc);
        }
        centers
    };
    let mut centers = greedy(min_semi);
    if centers.is_empty() {
        // winzige Bilder: keine Farbe erreicht die Mindestmenge
        centers = greedy(1);
    }
    // Ausgleich (k-Means über die Fächer)
    for _ in 0..2 {
        let means: Vec<[f64; 3]> = centers.iter().map(Center::mean).collect();
        let mut acc = vec![Center::default(); means.len()];
        for &bi in &cand {
            let bin = bins[bi];
            if let Some((k, dd)) = nearest(&means, bin.mean())
                && dd <= tol * tol
            {
                acc[k].add(&bin);
            }
        }
        let next: Vec<Center> = acc.into_iter().filter(|c| c.n > 0.0).collect();
        if !next.is_empty() {
            centers = next;
        }
    }
    let mut palette: Vec<[u8; 3]> = centers
        .iter()
        .map(|c| c.mean().map(|v| v.round().clamp(0.0, 255.0) as u8))
        .collect();
    let mut seen = std::collections::HashSet::new();
    palette.retain(|c| seen.insert(*c));
    if palette.is_empty() {
        // keine flachen Flächen (winzige Bilder, Rauschen): Median-Cut
        let mut px: Vec<u8> = rgb.iter().flat_map(|c| [c[0], c[1], c[2], 255]).collect();
        let skip: Vec<bool> = opaque.iter().map(|&b| !b).collect();
        quantize::reduce(
            &mut px,
            &skip,
            if o.colors >= 2 { o.colors as usize } else { 8 },
        );
        for (c, &op) in px.as_chunks::<4>().0.iter().zip(opaque) {
            let k = [c[0], c[1], c[2]];
            if op && !palette.contains(&k) {
                palette.push(k);
            }
        }
    }
    let pal: Vec<[i32; 3]> = palette
        .iter()
        .map(|c| [c[0] as i32, c[1] as i32, c[2] as i32])
        .collect();
    let dist =
        |c: [u8; 3], k: usize| -> i32 { (0..3).map(|j| (c[j] as i32 - pal[k][j]).pow(2)).sum() };
    let near_all = |c: [u8; 3]| -> (u32, i32) {
        (0..pal.len())
            .map(|k| (k as u32, dist(c, k)))
            .min_by_key(|v| v.1)
            .unwrap_or((NONE, 0))
    };
    let lim = (tol * tol) as i32;
    let mut core = vec![NONE; n];
    for i in 0..n {
        if opaque[i] && semi[i] {
            let (k, dd) = near_all(rgb[i]);
            if dd <= lim {
                core[i] = k;
            }
        }
    }
    let mut label = vec![NONE; n];
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if !opaque[i] {
                continue;
            }
            if core[i] != NONE {
                label[i] = core[i];
                continue;
            }
            // Farben der Nachbarschaft (5×5, bei breiten weichen Kanten bis 17×17)
            let mut cands = [0u32; 8];
            let mut cnt = 0usize;
            for rad in [2usize, 4, 8] {
                for yy in y.saturating_sub(rad)..(y + rad + 1).min(h) {
                    for xx in x.saturating_sub(rad)..(x + rad + 1).min(w) {
                        let l = core[yy * w + xx];
                        if l != NONE && cnt < 8 && !cands[..cnt].contains(&l) {
                            cands[cnt] = l;
                            cnt += 1;
                        }
                    }
                }
                if cnt >= 2 {
                    break;
                }
            }
            label[i] = if cnt == 0 {
                near_all(rgb[i]).0
            } else {
                *cands[..cnt]
                    .iter()
                    .min_by_key(|&&k| dist(rgb[i], k as usize))
                    .unwrap_or(&0)
            };
        }
    }
    (label, palette)
}
