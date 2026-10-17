//! Variance-Quantisierer, adaptive Palette, Nächste-Farbe-Suche und Umsetzen/Dithern (08 §2.3–§3).
//!
//! Bitgenau nach der Python-Referenz `re\export\fw_palette.py` (Stand Testrunde 11.10.2026:
//! float32-Mittelwerte, Rundung `floor(x + 0,5)`, Feinabgleich zeilenweise von unten nach oben,
//! Zellen-Kandidatenlisten werden nach dem Feinabgleich **nicht** neu gebaut, Alpha-Test des
//! Histogramms am Originalpixel).
//!
//! * [`colorquant`]: „colorquant“ (Utah Raster Toolkit, Craig Kolb; Variance-Verfahren nach
//!   Wan/Wong/Prusinkiewicz) wie `@0060b9c0`.
//! * [`Quantizer`]: Zustand wie das Original-Quantisiererobjekt (Palette, gesperrte Einträge,
//!   Transparenzmodus, Matte, Raster-/Grau-Schnellsuche, 8×8×8-Zellen) mit
//!   [`Quantizer::build_adaptive`], [`Quantizer::nearest`], [`Quantizer::remap_none`],
//!   [`Quantizer::remap_diffusion`] und [`Quantizer::remap_2x2`].

use std::collections::HashMap;

use crate::px::{a, argb, b, dist_l1, dist2, g, is_web, lum, matte_blend, r, tdiv, websnap};
use crate::tables_data::REDUCE;

/// Transparenzmodus (`paletteTransparency`, 08 §1.4).
pub const TRANSPARENCY_NONE: i32 = 0;
/// Indextransparenz (einzelne Palettenfarben transparent).
pub const TRANSPARENCY_INDEX: i32 = 1;
/// Index-Alpha (Alpha < 16 ⇒ Transparenzeintrag hinter der Palette).
pub const TRANSPARENCY_INDEX_ALPHA: i32 = 2;
/// Alphapalette (PNG8 mit `tRNS` je Eintrag).
pub const TRANSPARENCY_RGBA: i32 = 3;

fn f32r(v: f64) -> f64 {
    v as f32 as f64
}

/// Reduktionstabelle für `bits` (1…8): 1–6 Tabelle, 7 = `k >> 1`, 8 = Identität.
pub fn reduce_table(bits: u32) -> [u8; 256] {
    match bits {
        1..=6 => REDUCE[(bits - 1) as usize],
        7 => std::array::from_fn(|v| (v >> 1) as u8),
        _ => std::array::from_fn(|v| v as u8),
    }
}

#[derive(Clone)]
struct QBox {
    var: f64,
    mean: [f64; 3],
    weight: u64,
    freq: [Vec<u64>; 3],
    low: [usize; 3],
    high: [usize; 3],
}

/// Ergebnis von [`colorquant`]: Palettenfarben (r, g, b) in Box-Reihenfolge und Box-Gewichte.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct QuantResult {
    pub colors: Vec<[u8; 3]>,
    pub weights: Vec<u64>,
}

/// Variance-Quantisierer (`@0060b9c0`). `hist`: (reduzierte Farbe, Anzahl), `ncolors` gewünschte
/// Farben, `bits` Kanaltiefe des reduzierten Würfels (1…8).
pub fn colorquant(hist: &[([u8; 3], u64)], ncolors: usize, bits: u32) -> QuantResult {
    let size = 1usize << bits.clamp(1, 8);
    let total: u64 = hist.iter().map(|h| h.1).sum();
    if total == 0 {
        return QuantResult::default();
    }
    let total_f = total as f64;

    let stats = |bx: &mut QBox| {
        if bx.weight == 0 {
            bx.var = 0.0;
            return;
        }
        let mut v = 0.0f64;
        for c in 0..3 {
            let mut m = 0.0f64;
            let mut s2 = 0.0f64;
            for i in bx.low[c]..bx.high[c] {
                let f = bx.freq[c][i] as f64;
                m += i as f64 * f;
                s2 += (i * i) as f64 * f;
            }
            let m = f32r(m / bx.weight as f64);
            bx.mean[c] = m;
            v += s2 - m * m * bx.weight as f64;
        }
        bx.var = v / total_f;
    };

    let update = |b1: &mut QBox, b2: &mut QBox| {
        for c in 0..3 {
            b2.freq[c].iter_mut().for_each(|x| *x = 0);
        }
        for &(col, n) in hist {
            let (r0, g0, b0) = (col[0] as usize, col[1] as usize, col[2] as usize);
            if b2.low[0] <= r0
                && r0 < b2.high[0]
                && b2.low[1] <= g0
                && g0 < b2.high[1]
                && b2.low[2] <= b0
                && b0 < b2.high[2]
            {
                for (c, i) in [r0, g0, b0].into_iter().enumerate() {
                    b2.freq[c][i] += n;
                    b1.freq[c][i] = b1.freq[c][i].saturating_sub(n);
                }
            }
        }
    };

    let findcut = |bx: &QBox, c: usize| -> Option<(QBox, QBox)> {
        let (lo, hi) = (bx.low[c], bx.high[c]);
        if hi <= lo + 1 {
            return None;
        }
        let mn = ((lo as f64 + bx.mean[c]) * 0.5) as usize;
        let mx = ((bx.mean[c] + hi as f64) * 0.5) as usize;
        let mut cw: u64 = (lo..mn.min(size)).map(|i| bx.freq[c][i]).sum();
        let mut u = 0.0f64;
        let mut best = -1.0f64;
        let mut bestw = bx.weight;
        let mut cut = mn;
        let wf = bx.weight as f64;
        for i in mn..=mx.min(size - 1) {
            cw += bx.freq[c][i];
            if cw == bx.weight {
                break;
            }
            u += i as f64 * bx.freq[c][i] as f64 / wf;
            let d = bx.mean[c] - u;
            let v = (cw as f64 / (bx.weight - cw) as f64) * d * d;
            if v > best {
                best = v;
                bestw = cw;
                cut = i;
            }
        }
        cut += 1;
        let mut b1 = bx.clone();
        let mut b2 = bx.clone();
        b1.weight = bestw;
        b2.weight = bx.weight - bestw;
        if b1.weight == 0 || b2.weight == 0 {
            return None;
        }
        let cut = cut.clamp(lo, hi);
        b1.high[c] = cut;
        b2.low[c] = cut;
        update(&mut b1, &mut b2);
        stats(&mut b1);
        stats(&mut b2);
        Some((b1, b2))
    };

    let cutbox = |bx: &QBox| -> Option<(QBox, QBox)> {
        if bx.var == 0.0 || bx.weight == 0 {
            return None;
        }
        let mut res: Vec<Option<(QBox, QBox)>> = (0..3).map(|c| findcut(bx, c)).collect();
        let tv: Vec<f64> = res
            .iter()
            .map(|r| {
                r.as_ref()
                    .map(|(x, y)| x.var + y.var)
                    .unwrap_or(f64::INFINITY)
            })
            .collect();
        let k = if tv[0] <= tv[1] && tv[0] <= tv[2] {
            0
        } else if tv[1] <= tv[0] && tv[1] <= tv[2] {
            1
        } else {
            2
        };
        res.swap_remove(k)
    };

    let mut b0 = QBox {
        var: 0.0,
        mean: [0.0; 3],
        weight: total,
        freq: [vec![0; size], vec![0; size], vec![0; size]],
        low: [0; 3],
        high: [size; 3],
    };
    for &(col, n) in hist {
        for (c, &v) in col.iter().enumerate().take(3) {
            let i = (v as usize).min(size - 1);
            b0.freq[c][i] += n;
        }
    }
    stats(&mut b0);
    let mut boxes = vec![b0];
    if ncolors >= 2 {
        let mut fails = 0usize;
        let mut rem = ncolors as i64;
        loop {
            let mut rem1 = rem - 1;
            let mut k = 0;
            for i in 1..boxes.len() {
                if boxes[i].var > boxes[k].var {
                    k = i;
                }
            }
            match cutbox(&boxes[k]) {
                None => {
                    if rem1 > 8 || bits == 8 || boxes[k].var == 0.0 {
                        break;
                    }
                    boxes[k].var = 0.0;
                    fails += 1;
                    rem1 = rem;
                    if ncolors < fails {
                        break;
                    }
                }
                Some((x, y)) => {
                    boxes[k] = x;
                    boxes.push(y);
                }
            }
            rem = rem1;
            if boxes.len() >= ncolors {
                break;
            }
        }
    }
    let f = f32r(255.0 / (size - 1) as f64);
    let colors = boxes
        .iter()
        .map(|bx| std::array::from_fn(|c| ((bx.mean[c] * f + 0.5) as i64).clamp(0, 255) as u8))
        .collect();
    let weights = boxes.iter().map(|bx| bx.weight).collect();
    QuantResult { colors, weights }
}

/// Startbits nach der Farbzahl (08 §2.3 Schritt 4).
pub fn initial_bits(n: usize) -> u32 {
    if n < 5 {
        2
    } else if n < 13 {
        3
    } else if n > 40 {
        5
    } else {
        4
    }
}

/// Zustand der Palettenwahl und Nächste-Farbe-Suche (Original-Quantisiererobjekt).
#[derive(Clone, Debug)]
pub struct Quantizer {
    /// Palette (`0xAARRGGBB`, deckend bis auf ausdrücklich transparente Einträge).
    pub palette: Vec<u32>,
    /// Gesperrte Einträge (gleiche Länge wie `palette`).
    pub locked: Vec<bool>,
    /// Box-Gewichte bzw. Pixelzähler je Eintrag (Popularität, A: Das Original zählt hier die Box-Gewichte).
    pub counts: Vec<u64>,
    /// Transparenzmodus (`TRANSPARENCY_*`).
    pub tmode: i32,
    /// Matte-Farbe; Mischung aktiv, wenn ihr Alpha ≠ 0 ist.
    pub matte: u32,
    /// Raster-Schnellsuche (`L` Stufen) bei Einheitlich/Web 216.
    pub grid_levels: u32,
    /// Grau-Schnellsuche.
    pub gray: bool,
    /// Palette „Benutzerdefiniert“.
    pub custom: bool,
    /// Index hinter der Palette für Index-Alpha (Matte-RGB, Alpha 0).
    pub transparent_index: Option<usize>,
    /// Flag `+0x1D58`: nur bei adaptiver Palette gesetzt (Dither-Fehler am Transparenzeintrag).
    pub adaptive: bool,
    cells: Option<Vec<Vec<u16>>>,
    memo: HashMap<u32, usize>,
}

impl Quantizer {
    /// Leerer Quantisierer.
    pub fn new(tmode: i32, matte: u32) -> Quantizer {
        Quantizer {
            palette: Vec::new(),
            locked: Vec::new(),
            counts: Vec::new(),
            tmode,
            matte,
            grid_levels: 0,
            gray: false,
            custom: false,
            transparent_index: None,
            adaptive: false,
            cells: None,
            memo: HashMap::new(),
        }
    }

    /// Feste Palette setzen (Raster-/Grau-Schnellsuche über die Flags).
    pub fn set_palette(&mut self, pal: Vec<u32>, grid_levels: u32, gray: bool, custom: bool) {
        self.locked = vec![false; pal.len()];
        self.counts = vec![0; pal.len()];
        self.palette = pal;
        self.grid_levels = grid_levels;
        self.gray = gray;
        self.custom = custom;
        self.reset_lookup();
        self.append_transparent();
    }

    /// Palette ersetzen, ohne die Schnellsuchen (verwirft Zellen und Cache).
    pub fn replace_palette(&mut self, pal: Vec<u32>) {
        self.locked.resize(pal.len(), false);
        self.counts.resize(pal.len(), 0);
        self.palette = pal;
        self.grid_levels = 0;
        if !self.custom {
            self.gray = false;
        }
        self.reset_lookup();
        self.append_transparent();
    }

    fn reset_lookup(&mut self) {
        self.memo.clear();
        self.cells = None;
    }

    fn append_transparent(&mut self) {
        if self.tmode == TRANSPARENCY_INDEX_ALPHA {
            if self.palette.len() == 256 {
                self.palette.truncate(255);
                self.locked.truncate(255);
                self.counts.truncate(255);
            }
            self.transparent_index = Some(self.palette.len());
        } else {
            self.transparent_index = None;
        }
    }

    /// Adaptive Palette (`@0074a4b0`). `width` = Bildbreite (Reihenfolge des Feinabgleichs: Zeilen von
    /// unten nach oben); `canvas` = Sonderfarbe (Leinwandfarbe, bei gesetzter Matte `None`).
    #[allow(clippy::too_many_arguments)]
    pub fn build_adaptive(
        &mut self,
        pixels: &[u32],
        ncolors: i32,
        websnap_on: bool,
        tolerance: i32,
        locked: &[u32],
        canvas: Option<u32>,
        width: usize,
    ) -> &[u32] {
        self.adaptive = true;
        let (snap, tol) = if websnap_on {
            (true, (tolerance / 2).max(0) as u32)
        } else {
            (true, 3)
        };
        let mut n = ncolors.clamp(2, 256) as usize;
        if (self.tmode == TRANSPARENCY_INDEX || self.tmode == TRANSPARENCY_INDEX_ALPHA) && n == 256
        {
            n = 255;
        }
        let composite = self.tmode != TRANSPARENCY_RGBA && a(self.matte) != 0;
        n = n.max(locked.len());
        let matte = self.matte;
        let prepped: Vec<u32> = pixels
            .iter()
            .map(|&c| {
                let c = if composite && a(c) != 255 {
                    matte_blend(c, matte, false)
                } else {
                    c
                };
                if snap { websnap(c, tol) } else { c }
            })
            .collect();
        let mut bits = initial_bits(n);
        let mut best = 0usize;
        let mut state = 0;
        let mut res;
        loop {
            if state == -1 {
                bits -= 1;
            }
            let tb = reduce_table(bits);
            let mut hm: HashMap<[u8; 3], u64> = HashMap::new();
            for (&c, &c0) in prepped.iter().zip(pixels) {
                if self.tmode == TRANSPARENCY_NONE || a(c0) > 16 {
                    let k = [tb[r(c) as usize], tb[g(c) as usize], tb[b(c) as usize]];
                    *hm.entry(k).or_insert(0) += 1;
                }
            }
            let mut hist: Vec<([u8; 3], u64)> = hm.into_iter().collect();
            hist.sort_unstable_by_key(|h| h.0);
            res = colorquant(&hist, n, bits);
            let count = res.colors.len();
            if state == -1 {
                break;
            }
            if count > best && (count != n || bits != 7) {
                best = count;
                if count >= n {
                    break;
                }
                if bits < 7 {
                    bits += 1;
                    state = 0;
                    continue;
                }
                break;
            }
            state = -1;
        }
        let mut pal: Vec<u32> = res
            .colors
            .iter()
            .map(|c| argb(255, c[0] as u32, c[1] as u32, c[2] as u32))
            .collect();
        let mut counts = res.weights.clone();
        if let Some(cv) = canvas
            && a(cv) != 0
            && !pal.is_empty()
        {
            let mut k = 0;
            for i in 1..pal.len() {
                if dist2(pal[i], cv) < dist2(pal[k], cv) {
                    k = i;
                }
            }
            if dist2(pal[k], cv) <= 0x16B {
                pal[k] = cv | 0xFF00_0000;
            }
        }
        while pal.len() < 2 {
            pal.push(0xFFFF_FFFF);
            counts.push(0);
        }
        if snap {
            for c in &mut pal {
                *c = websnap(*c, tol);
            }
        }
        let mut target = pal.clone();
        let mut bestd = vec![0xFFFFFu32; pal.len()];
        let mut lockf = vec![false; pal.len()];
        for &c in locked {
            let c = c | 0xFF00_0000;
            let key = |i: usize, pal: &[u32], lockf: &[bool]| -> (u32, u32, usize) {
                (
                    if pal[i] == c { 0 } else { 1 },
                    if !lockf[i] { dist2(pal[i], c) } else { 1 << 30 },
                    i,
                )
            };
            let mut k = 0;
            for i in 1..pal.len() {
                if key(i, &pal, &lockf) < key(k, &pal, &lockf) {
                    k = i;
                }
            }
            if pal.len() < n && dist2(pal[k], c) > 0x6C {
                pal.push(c);
                target.push(c);
                bestd.push(0);
                lockf.push(true);
                counts.push(0);
                k = pal.len() - 1;
            }
            pal[k] = c;
            target[k] = c;
            bestd[k] = 0;
            lockf[k] = true;
        }
        self.palette = pal.clone();
        self.locked = lockf.clone();
        self.counts = counts;
        self.grid_levels = 0;
        self.gray = false;
        self.custom = false;
        self.reset_lookup();
        // Feinabgleich (`@007465f0`): Zeilen von unten nach oben, je Zeile links → rechts.
        let w = width.max(1);
        let hgt = prepped.len() / w;
        let mut order: Vec<u32> = Vec::with_capacity(prepped.len());
        if width > 0 {
            for y in (0..hgt).rev() {
                order.extend_from_slice(&prepped[y * w..(y + 1) * w]);
            }
            order.extend_from_slice(&prepped[hgt * w..]);
        } else {
            order.extend_from_slice(&prepped);
        }
        for c in order {
            let i = self.nearest(c);
            if i >= pal.len() || a(pal[i]) < 0x10 {
                continue;
            }
            let d = dist2(pal[i], c);
            if d > 0x6B || lockf[i] {
                continue;
            }
            let sw = is_web(target[i]);
            let pw = is_web(c);
            if pw && !sw {
                // Webfarbe schlägt Nicht-Webfarbe
            } else if (sw && !pw) || bestd[i] <= d {
                continue;
            }
            target[i] = c;
            bestd[i] = d;
        }
        self.palette = target.iter().map(|t| t | 0xFF00_0000).collect();
        // Zellen bleiben (Kandidaten der Palette vor dem Feinabgleich), nur der Ergebnis-Cache wird geleert.
        self.memo.clear();
        self.append_transparent();
        &self.palette
    }

    /// Global adaptive palette of an animated GIF (spec 44 §3): one histogram over all frames
    /// (`pixels` = frames stacked, `frame_pixels` = W·H of one frame), the same variance quantiser
    /// and bit loop as [`Quantizer::build_adaptive`], but **no** tolerance-3 web snap (only with
    /// web-adaptive), no canvas special colour, no padding to two colours and no fine-tuning.
    /// Locked colours are pre-weighted in the histogram and then replace their nearest entry. The
    /// palette ends sorted by luminance descending with [`crt_qsort`] (unstable, CRT tie order).
    /// `ncolors` is already capped by the caller (255 adaptive, 256 exact fallback).
    pub fn build_adaptive_global(
        &mut self,
        pixels: &[u32],
        frame_pixels: usize,
        ncolors: i32,
        websnap_on: bool,
        tolerance: i32,
        locked: &[u32],
    ) -> &[u32] {
        self.adaptive = true;
        let tol = (tolerance / 2).max(0) as u32;
        let mut n = ncolors.clamp(2, 256) as usize;
        if n == 256 && self.tmode == TRANSPARENCY_INDEX_ALPHA {
            n = 255;
        }
        let matte = self.matte;
        let blend = a(matte) != 0;
        let prepped: Vec<u32> = pixels
            .iter()
            .map(|&p0| {
                let mut p = p0;
                // 0x00010203 is never blended (spec 44 §3.1, kept as specified)
                if blend && a(p) != 255 && p != 0x0001_0203 {
                    p = matte_blend(p, matte, false);
                }
                if websnap_on { websnap(p, tol) } else { p }
            })
            .collect();
        let locked_weight = if locked.is_empty() {
            0
        } else {
            let d = (n as i64 - locked.len() as i64).max(1) as u64;
            frame_pixels as u64 / d
        };
        let mut bits = initial_bits(n);
        let mut best = 0usize;
        let mut state = 0;
        let mut res;
        loop {
            if state == -1 {
                bits -= 1;
            }
            let tb = reduce_table(bits);
            let key = |c: u32| [tb[r(c) as usize], tb[g(c) as usize], tb[b(c) as usize]];
            let mut hm: HashMap<[u8; 3], u64> = HashMap::new();
            if locked_weight > 0 {
                for &c in locked {
                    *hm.entry(key(c)).or_insert(0) += locked_weight;
                }
            }
            for (&c, &c0) in prepped.iter().zip(pixels) {
                if self.tmode == TRANSPARENCY_NONE || a(c0) > 16 {
                    *hm.entry(key(c)).or_insert(0) += 1;
                }
            }
            let mut hist: Vec<([u8; 3], u64)> = hm.into_iter().collect();
            hist.sort_unstable_by_key(|h| h.0);
            res = colorquant(&hist, n, bits);
            let count = res.colors.len();
            if state == -1 {
                break;
            }
            if count > best && (count != n || bits != 7) {
                best = count;
                if count >= n {
                    break;
                }
                if bits < 7 {
                    bits += 1;
                    state = 0;
                    continue;
                }
                break;
            }
            state = -1;
        }
        let mut pal: Vec<u32> = res
            .colors
            .iter()
            .map(|c| argb(255, c[0] as u32, c[1] as u32, c[2] as u32))
            .collect();
        let mut counts = res.weights.clone();
        if pal.is_empty() {
            // nothing counted (all pixels transparent): one opaque matte entry keeps the lookup valid
            pal.push(matte | 0xFF00_0000);
            counts.push(0);
        }
        if websnap_on {
            for c in &mut pal {
                *c = websnap(*c, tol);
            }
        }
        let mut lockf = vec![false; pal.len()];
        for &c in locked {
            let c = c | 0xFF00_0000;
            let k = pal.iter().position(|&p| p == c).unwrap_or_else(|| {
                let mut k = 0;
                for i in 1..pal.len() {
                    if dist2(pal[i], c) < dist2(pal[k], c) {
                        k = i;
                    }
                }
                k
            });
            pal[k] = c;
            lockf[k] = true;
        }
        let mut entries: Vec<(u32, u64, bool)> = pal
            .iter()
            .zip(&counts)
            .zip(&lockf)
            .map(|((&p, &w), &l)| (p, w, l))
            .collect();
        crt_qsort(&mut entries, |x, y| {
            let (lx, ly) = (lum(x.0) & 0xFF, lum(y.0) & 0xFF);
            if ly < lx {
                -1
            } else if lx < ly {
                1
            } else {
                0
            }
        });
        self.palette = entries.iter().map(|e| e.0 | 0xFF00_0000).collect();
        self.counts = entries.iter().map(|e| e.1).collect();
        self.locked = entries.iter().map(|e| e.2).collect();
        self.grid_levels = 0;
        self.gray = false;
        self.custom = false;
        self.reset_lookup();
        self.append_transparent();
        &self.palette
    }

    fn brute(&self, c: u32) -> usize {
        let mut best = 0xFFFFFu32;
        let mut bi = 0;
        for (i, &p) in self.palette.iter().enumerate() {
            let d = dist2(p, c);
            if d < best {
                best = d;
                bi = i;
                if d == 0 {
                    break;
                }
            }
        }
        bi
    }

    fn build_cells(&mut self) {
        let n = self.palette.len();
        let mut cells = Vec::with_capacity(512);
        for ri in 0..8u32 {
            for gi in 0..8u32 {
                for bi in 0..8u32 {
                    let lo = [ri, gi, bi].map(|k| (32 * k as i32 - 7).max(0) as u32);
                    let hi = [ri, gi, bi].map(|k| (32 * k + 39).min(255));
                    let mut cand: Vec<u16> = (0..n)
                        .filter(|&i| {
                            let p = self.palette[i];
                            lo[0] <= r(p)
                                && r(p) <= hi[0]
                                && lo[1] <= g(p)
                                && g(p) <= hi[1]
                                && lo[2] <= b(p)
                                && b(p) <= hi[2]
                        })
                        .map(|i| i as u16)
                        .collect();
                    for dr in [0, 31] {
                        for dg in [0, 31] {
                            for db in [0, 31] {
                                let corner = argb(255, 32 * ri + dr, 32 * gi + dg, 32 * bi + db);
                                let k = self.brute(corner) as u16;
                                if !cand.contains(&k) {
                                    cand.push(k);
                                }
                            }
                        }
                    }
                    cells.push(cand);
                }
            }
        }
        self.cells = Some(cells);
    }

    /// Nächste Farbe (`@007462f0`): Transparenzsonderfälle, Grau- und Raster-Schnellsuche, dann
    /// Zellensuche über die Kandidatenlisten. Liefert ggf. den Transparenzindex `palette.len()`.
    pub fn nearest(&mut self, c: u32) -> usize {
        let al = a(c);
        let n = self.palette.len();
        if self.tmode == TRANSPARENCY_INDEX_ALPHA && al < 16 {
            return n;
        }
        let mut c = c;
        if self.gray {
            let l = lum(c);
            if !self.custom {
                if n == 0 {
                    return 0;
                }
                let k = n as i64 - (((n as i64 - 1) * l as i64 + 0x80) / 0xFF) - 1;
                return if k >= 0 && (k as usize) < n {
                    k as usize
                } else {
                    n - 1
                };
            }
            c = argb(255, l, l, l);
        }
        if self.grid_levels > 0 && !self.custom {
            let ll = self.grid_levels as i64;
            let f = |v: u32| ll - ((v as i64 * (ll - 1) + 0x80) / 0xFF) - 1;
            let idx = (f(r(c)) * ll + f(g(c))) * ll + f(b(c));
            return idx.clamp(0, n.saturating_sub(1) as i64) as usize;
        }
        if let Some(&i) = self.memo.get(&c) {
            return i;
        }
        if self.cells.is_none() {
            self.build_cells();
        }
        let cells = self
            .cells
            .as_ref()
            .map(|v| &v[((r(c) >> 5) * 64 + (g(c) >> 5) * 8 + (b(c) >> 5)) as usize]);
        let mut best = 0xFFFFFu32;
        let mut bi = cells.and_then(|v| v.first().copied()).unwrap_or(0) as usize;
        if let Some(cell) = cells {
            for &i in cell {
                let i = i as usize;
                let Some(&p) = self.palette.get(i) else {
                    continue;
                };
                let d = dist2(p, c);
                if d < best {
                    best = d;
                    bi = i;
                    if d == 0 {
                        break;
                    }
                }
            }
        }
        self.memo.insert(c, bi);
        bi
    }

    /// Farbe eines Index; hinter der Palette die Matte-RGB mit Alpha 0.
    pub fn color(&self, i: usize) -> u32 {
        self.palette
            .get(i)
            .copied()
            .unwrap_or(self.matte & 0x00FF_FFFF)
    }

    /// Umsetzen ohne Dithering mit LZW-Glättung (`@0074bcb0`). `smoothing` = `LZWSmoothingAmount`
    /// (Vorgabe 7, nur wirksam mit `optimized` und `smoothing >= lossy`).
    pub fn remap_none(
        &mut self,
        pixels: &[u32],
        w: usize,
        h: usize,
        optimized: bool,
        smoothing: i32,
        lossy: i32,
    ) -> Vec<u8> {
        let mut out = vec![0u8; w * h];
        let comp = a(self.matte) != 0;
        let smooth = optimized && !self.gray && smoothing >= lossy;
        let mut prev = lossy as u32;
        let matte = self.matte;
        let mut src = vec![0u32; w];
        let mut mapped = vec![0usize; w];
        for y in 0..h {
            for x in 0..w {
                let mut c = pixels[y * w + x];
                if comp && a(c) != 255 {
                    c = matte_blend(c, matte, true);
                }
                src[x] = c;
                mapped[x] = self.nearest(c);
            }
            if smooth {
                for x in 0..w {
                    let mut cur = self.color(mapped[x]);
                    let keep =
                        x + 2 < w && mapped[x] == mapped[x + 1] && mapped[x + 1] == mapped[x + 2];
                    if cur == prev || a(src[x]) != a(prev) {
                        // behalten
                    } else if !keep {
                        let d_prev = dist_l1(prev, src[x]) as i64;
                        let d_cur = dist_l1(cur, src[x]) as i64;
                        if d_prev <= d_cur + smoothing as i64 {
                            mapped[x] = self.nearest(prev);
                            cur = prev;
                        }
                    }
                    prev = cur;
                }
            }
            for x in 0..w {
                out[y * w + x] = mapped[x].min(255) as u8;
            }
        }
        out
    }

    /// Umsetzen ohne Glättung (animiertes GIF, `@0074c1d0`).
    pub fn remap_plain(&mut self, pixels: &[u32], w: usize, h: usize) -> Vec<u8> {
        self.remap_none(pixels, w, h, false, 0, 0)
    }

    /// Original-Fehlerdiffusion (`@0074c530`): Serpentine, Übertrag 170/86, 4×4-Mustermatrix.
    pub fn remap_diffusion(
        &mut self,
        pixels: &[u32],
        w: usize,
        h: usize,
        percent: i32,
        websnap_on: bool,
        tolerance: i32,
    ) -> Vec<u8> {
        const PATTERN: [i32; 16] = [6, 0, -6, 0, 0, 0, 0, 0, -6, 0, 6, 0, 0, 0, 0, 0];
        let mut pct = percent;
        let mut thr = 5;
        if pct < 10 {
            pct = 10;
        } else if pct >= 100 {
            pct = 100;
            thr = 2;
        }
        let tol = (tolerance / 2).max(0) as u32;
        let mut out = vec![0u8; w * h];
        let mut up = [vec![0i32; w + 1], vec![0i32; w + 1], vec![0i32; w + 1]];
        let comp = a(self.matte) != 0;
        let matte = self.matte;
        for y in 0..h {
            let mut carry = [0i32; 3];
            for k in 0..w {
                let x = if y % 2 == 0 { k } else { w - 1 - k };
                let orig = pixels[y * w + x];
                let mut v: [i32; 3] = if self.gray {
                    let l = lum(orig) as i32;
                    [l, l, l]
                } else {
                    let mut c = orig;
                    if websnap_on {
                        c = websnap(c, tol);
                    }
                    if comp && a(c) != 255 {
                        c = matte_blend(c, matte, false);
                    }
                    [r(c) as i32, g(c) as i32, b(c) as i32]
                };
                let pat = PATTERN[(x & 3) + (y & 3) * 4];
                for ch in 0..3 {
                    let mut e = up[ch][x] * 0x56 + carry[ch] * 0xAA;
                    e = tdiv(e, 256);
                    e = tdiv(e * pct, 100);
                    v[ch] += tdiv(e * pct, 100) + tdiv(e * pat, 10);
                }
                let cl = v.map(|t| t.clamp(0, 255) as u32);
                let idx = self.nearest(argb(a(orig), cl[0], cl[1], cl[2]));
                out[y * w + x] = idx.min(255) as u8;
                let pc = self.color(idx);
                let mut diffuse = true;
                if a(pc) == 0 || idx >= self.palette.len() {
                    diffuse =
                        a(orig) != 0 && self.tmode != TRANSPARENCY_INDEX_ALPHA && !self.adaptive;
                }
                if diffuse {
                    for (ch, pv) in [r(pc), g(pc), b(pc)].into_iter().enumerate() {
                        let diff = v[ch] - pv as i32;
                        let old = carry[ch];
                        let mut ne = diff + (tdiv(old, 2) - old);
                        if diff.abs() < ne.abs() {
                            ne = diff;
                        }
                        if ne.abs() < thr {
                            ne = 0;
                        }
                        ne = ne.clamp(-255, 255);
                        carry[ch] = ne;
                        up[ch][x] = ne;
                    }
                } else {
                    carry = [0; 3];
                    for u in up.iter_mut() {
                        u[x] = 0;
                    }
                }
            }
        }
        out
    }

    /// 2×2-Musterdither (`@0074cee0`).
    pub fn remap_2x2(&mut self, pixels: &[u32], w: usize, h: usize) -> Vec<u8> {
        let mut out = vec![0u8; w * h];
        let comp = a(self.matte) != 0;
        let matte = self.matte;
        for y in 0..h {
            for x in 0..w {
                let mut c = pixels[y * w + x];
                if comp && a(c) != 255 {
                    c = matte_blend(c, matte, true);
                }
                let p = [r(c) as i32, g(c) as i32, b(c) as i32];
                let mut acc = p;
                let mut idx = [self.nearest(c), 0, 0, 0];
                for k in 0..3 {
                    let pc = self.color(idx[k]);
                    acc = [
                        acc[0] + p[0] - r(pc) as i32,
                        acc[1] + p[1] - g(pc) as i32,
                        acc[2] + p[2] - b(pc) as i32,
                    ];
                    let cl = acc.map(|t| t.clamp(0, 255) as u32);
                    idx[k + 1] = self.nearest(argb(a(c), cl[0], cl[1], cl[2]));
                }
                if idx[0] == idx[2] {
                    idx.swap(2, 3);
                }
                out[y * w + x] = idx[(x & 1) + 2 * (y & 1)].min(255) as u8;
            }
        }
        out
    }
}

/// Unstable quicksort with the tie behaviour of the classic C runtime `qsort` (spec 44 §3.2):
/// median-of-three pivot, partition loops as specified, an explicit stack and a selection
/// "short sort" for partitions of at most 8 elements. `cmp` returns <0, 0 or >0. A stable sort
/// orders equal keys differently, which would change palette indices.
pub fn crt_qsort<T, F: FnMut(&T, &T) -> i32>(a: &mut [T], mut cmp: F) {
    const CUTOFF: isize = 8;
    let num = a.len() as isize;
    if num < 2 {
        return;
    }
    let at = |i: isize| i as usize;
    let mut stack: Vec<(isize, isize)> = Vec::new();
    let (mut lo, mut hi): (isize, isize) = (0, num - 1);
    loop {
        let size = hi - lo + 1;
        if size <= CUTOFF {
            // short sort: move the largest element to the end, then shrink
            let mut h = hi;
            while h > lo {
                let mut mx = lo;
                for p in lo + 1..=h {
                    if cmp(&a[at(p)], &a[at(mx)]) > 0 {
                        mx = p;
                    }
                }
                a.swap(at(mx), at(h));
                h -= 1;
            }
        } else {
            let mut mid = lo + size / 2;
            if cmp(&a[at(lo)], &a[at(mid)]) > 0 {
                a.swap(at(lo), at(mid));
            }
            if cmp(&a[at(lo)], &a[at(hi)]) > 0 {
                a.swap(at(lo), at(hi));
            }
            if cmp(&a[at(mid)], &a[at(hi)]) > 0 {
                a.swap(at(mid), at(hi));
            }
            let mut loguy = lo;
            let mut higuy = hi;
            loop {
                if mid > loguy {
                    loop {
                        loguy += 1;
                        if !(loguy < mid && cmp(&a[at(loguy)], &a[at(mid)]) <= 0) {
                            break;
                        }
                    }
                }
                if mid <= loguy {
                    loop {
                        loguy += 1;
                        if !(loguy <= hi && cmp(&a[at(loguy)], &a[at(mid)]) <= 0) {
                            break;
                        }
                    }
                }
                loop {
                    higuy -= 1;
                    if !(higuy > mid && cmp(&a[at(higuy)], &a[at(mid)]) > 0) {
                        break;
                    }
                }
                if higuy < loguy {
                    break;
                }
                a.swap(at(loguy), at(higuy));
                if mid == higuy {
                    mid = loguy;
                }
            }
            higuy += 1;
            if mid < higuy {
                loop {
                    higuy -= 1;
                    if !(higuy > mid && cmp(&a[at(higuy)], &a[at(mid)]) == 0) {
                        break;
                    }
                }
            }
            if mid >= higuy {
                loop {
                    higuy -= 1;
                    if !(higuy > lo && cmp(&a[at(higuy)], &a[at(mid)]) == 0) {
                        break;
                    }
                }
            }
            if higuy - lo >= hi - loguy {
                if lo < higuy {
                    stack.push((lo, higuy));
                }
                if loguy < hi {
                    lo = loguy;
                    continue;
                }
            } else {
                if loguy < hi {
                    stack.push((loguy, hi));
                }
                if lo < higuy {
                    hi = higuy;
                    continue;
                }
            }
        }
        match stack.pop() {
            Some((l, h)) => {
                lo = l;
                hi = h;
            }
            None => return,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crt_qsort_sorts_and_keeps_crt_tie_order() {
        // sorted result for distinct keys
        let mut v: Vec<i32> = (0..300).map(|i| (i * 7919) % 1009).collect();
        let mut e = v.clone();
        e.sort();
        crt_qsort(&mut v, |x, y| x.cmp(y) as i32);
        assert_eq!(v, e);
        // many ties: still ordered by key, every element kept (tie order is pinned by the
        // animated-GIF golden test)
        let mut t: Vec<(i32, i32)> = (0..40).map(|i| ((i * 13) % 4, i)).collect();
        crt_qsort(&mut t, |x, y| x.0.cmp(&y.0) as i32);
        assert!(t.windows(2).all(|w| w[0].0 <= w[1].0));
        let mut ids: Vec<i32> = t.iter().map(|x| x.1).collect();
        ids.sort();
        assert_eq!(ids, (0..40).collect::<Vec<_>>());
        // short sort only (≤ 8 elements)
        let mut s = vec![3, 1, 2];
        crt_qsort(&mut s, |x, y| x.cmp(y) as i32);
        assert_eq!(s, vec![1, 2, 3]);
    }

    #[test]
    fn colorquant_two_colors() {
        let hist = vec![([0u8, 0, 0], 10u64), ([31, 31, 31], 10)];
        let r = colorquant(&hist, 2, 5);
        assert_eq!(r.colors.len(), 2);
        assert!(r.colors.contains(&[0, 0, 0]) && r.colors.contains(&[255, 255, 255]));
        assert!(colorquant(&[], 4, 5).colors.is_empty());
    }

    #[test]
    fn grid_and_gray_lookup() {
        let mut q = Quantizer::new(0, 0);
        q.set_palette(crate::palettes::palette_web216(), 6, false, false);
        assert_eq!(q.nearest(0xFFFFFFFF), 0);
        assert_eq!(q.nearest(0xFF000000), 215);
        let mut g = Quantizer::new(0, 0);
        g.set_palette(crate::palettes::palette_gray(4), 0, true, false);
        assert_eq!(g.nearest(0xFFFFFFFF), 0);
        assert_eq!(g.nearest(0xFF000000), 3);
    }

    #[test]
    fn adaptive_small_image() {
        let px: Vec<u32> = (0..64)
            .map(|i| if i < 32 { 0xFFFF0000 } else { 0xFF0000FF })
            .collect();
        let mut q = Quantizer::new(0, 0xFFFFFFFF);
        q.build_adaptive(&px, 16, false, 14, &[], Some(0xFFFFFFFF), 8);
        assert!(q.palette.contains(&0xFFFF0000) && q.palette.contains(&0xFF0000FF));
        let idx = q.remap_none(&px, 8, 8, true, 7, 0);
        assert_eq!(q.palette[idx[0] as usize], 0xFFFF0000);
        assert_eq!(q.palette[idx[63] as usize], 0xFF0000FF);
    }
}
