//! Farben reduzieren (Schritt `optimize.quantize`, SPEC-BATCH §3.12): Median-Cut über RGBA mit
//! gewichteten Mittelwerten und zwei k-means-Nachläufen, Umsetzen ohne Dithering, geordnet
//! (Bayer 8×8) oder Floyd-Steinberg (Serpentine). Eigene Umsetzung (statt `quantette`, das nicht
//! im Baum ist; keine GPL-Bausteine wie libimagequant).

use std::collections::HashMap;

/// Dithering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dither {
    None,
    Ordered,
    FloydSteinberg,
}

impl Dither {
    pub fn parse(s: &str) -> Dither {
        match s {
            "none" => Dither::None,
            "ordered" | "blueNoise" => Dither::Ordered,
            _ => Dither::FloydSteinberg,
        }
    }
}

type Px = [u8; 4];
/// Box des Median-Cut: Farben mit Anzahl, Teilungskanal, Bewertung.
type ColorBox = (Vec<(Px, u64)>, usize, u64);

fn dist(a: &[f32; 4], b: &Px) -> f32 {
    let d = [
        a[0] - b[0] as f32,
        a[1] - b[1] as f32,
        a[2] - b[2] as f32,
        a[3] - b[3] as f32,
    ];
    // Alpha zählt doppelt (sichtbarer Unterschied an Kanten)
    d[0] * d[0] + d[1] * d[1] + d[2] * d[2] + 2.0 * d[3] * d[3]
}

/// Palette mit höchstens `n` Farben (Median-Cut + k-means). Sind es weniger Farben, ist die
/// Palette genau die Menge der Farben.
pub fn palette(px: &[Px], n: usize) -> Vec<Px> {
    let n = n.clamp(2, 256);
    let mut hist: HashMap<Px, u64> = HashMap::new();
    for p in px {
        // voll transparent → eine Farbe
        let k = if p[3] == 0 { [0, 0, 0, 0] } else { *p };
        *hist.entry(k).or_default() += 1;
    }
    let mut colors: Vec<(Px, u64)> = hist.into_iter().collect();
    colors.sort_unstable();
    if colors.len() <= n {
        return colors.into_iter().map(|c| c.0).collect();
    }
    // Median-Cut: Box mit größter gewichteter Spannweite teilen (Kennzahlen je Box beim Anlegen)
    let stats = |b: &Vec<(Px, u64)>| -> (usize, u64) {
        if b.len() < 2 {
            return (0, 0);
        }
        let weight: u64 = b.iter().map(|c| c.1).sum();
        let mut best = (0, 0);
        for ch in 0..4 {
            let (lo, hi) = b.iter().fold((255u8, 0u8), |(lo, hi), c| {
                (lo.min(c.0[ch]), hi.max(c.0[ch]))
            });
            let score = (hi - lo) as u64 * ((weight as f64).sqrt() as u64).max(1);
            if score > best.1 {
                best = (ch, score);
            }
        }
        best
    };
    let first = stats(&colors);
    let mut boxes: Vec<ColorBox> = vec![(colors, first.0, first.1)];
    while boxes.len() < n {
        let Some((bi, _)) = boxes
            .iter()
            .enumerate()
            .filter(|(_, b)| b.2 > 0)
            .max_by_key(|(_, b)| b.2)
        else {
            break;
        };
        let (mut b, ch, _) = boxes.swap_remove(bi);
        b.sort_unstable_by_key(|c| c.0[ch]);
        let total: u64 = b.iter().map(|c| c.1).sum();
        let mut acc = 0;
        let mut cut = 1;
        for (i, c) in b.iter().enumerate() {
            acc += c.1;
            if acc * 2 >= total {
                cut = (i + 1).clamp(1, b.len() - 1);
                break;
            }
        }
        let rest = b.split_off(cut);
        let (s1, s2) = (stats(&b), stats(&rest));
        boxes.push((b, s1.0, s1.1));
        boxes.push((rest, s2.0, s2.1));
    }
    let all: Vec<(Px, u64)> = if boxes.iter().map(|b| b.0.len()).sum::<usize>() <= 50_000 {
        boxes.iter().flat_map(|b| b.0.iter().copied()).collect()
    } else {
        Vec::new()
    };
    let mut pal: Vec<[f32; 4]> = boxes
        .iter()
        .map(|(b, _, _)| {
            let w: f64 = b.iter().map(|c| c.1 as f64).sum();
            let mut m = [0f64; 4];
            for c in b {
                for (mk, ck) in m.iter_mut().zip(c.0) {
                    *mk += ck as f64 * c.1 as f64;
                }
            }
            [
                (m[0] / w) as f32,
                (m[1] / w) as f32,
                (m[2] / w) as f32,
                (m[3] / w) as f32,
            ]
        })
        .collect();
    // k-means (2 Läufe; bei sehr vielen Farben ausgelassen, `all` ist dann leer)
    for _ in 0..if all.is_empty() { 0 } else { 2 } {
        let mut sum = vec![[0f64; 5]; pal.len()];
        for (c, w) in &all {
            let i = nearest_f(&pal, c);
            for k in 0..4 {
                sum[i][k] += c[k] as f64 * *w as f64;
            }
            sum[i][4] += *w as f64;
        }
        for (p, s) in pal.iter_mut().zip(&sum) {
            if s[4] > 0.0 {
                *p = [
                    (s[0] / s[4]) as f32,
                    (s[1] / s[4]) as f32,
                    (s[2] / s[4]) as f32,
                    (s[3] / s[4]) as f32,
                ];
            }
        }
    }
    let mut out: Vec<Px> = pal
        .iter()
        .map(|p| {
            [
                p[0].round() as u8,
                p[1].round() as u8,
                p[2].round() as u8,
                p[3].round() as u8,
            ]
        })
        .collect();
    // fast transparente Einträge als echte Transparenz
    for p in &mut out {
        if p[3] < 4 {
            *p = [0, 0, 0, 0];
        } else if p[3] > 251 {
            p[3] = 255;
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

fn nearest_f(pal: &[[f32; 4]], c: &Px) -> usize {
    let mut best = (0, f32::MAX);
    for (i, p) in pal.iter().enumerate() {
        let d = dist(p, c);
        if d < best.1 {
            best = (i, d);
        }
    }
    best.0
}

fn nearest(pal: &[Px], c: [f32; 4]) -> usize {
    let mut best = (0, f32::MAX);
    for (i, p) in pal.iter().enumerate() {
        let d = dist(&c, p);
        if d < best.1 {
            best = (i, d);
        }
    }
    best.0
}

const BAYER8: [[u8; 8]; 8] = [
    [0, 32, 8, 40, 2, 34, 10, 42],
    [48, 16, 56, 24, 50, 18, 58, 26],
    [12, 44, 4, 36, 14, 46, 6, 38],
    [60, 28, 52, 20, 62, 30, 54, 22],
    [3, 35, 11, 43, 1, 33, 9, 41],
    [51, 19, 59, 27, 49, 17, 57, 25],
    [15, 47, 7, 39, 13, 45, 5, 37],
    [63, 31, 55, 23, 61, 29, 53, 21],
];

/// Setzt Pixel auf die Palette um. `amount` 0…1 (Stärke des Ditherings). Liefert die Indizes.
pub fn remap(px: &mut [Px], w: usize, pal: &[Px], dither: Dither, amount: f32) -> Vec<u8> {
    let h = px.len().checked_div(w).unwrap_or(0);
    let mut idx = vec![0u8; px.len()];
    let mut cache: HashMap<Px, u8> = HashMap::new();
    let mut lookup = |c: [f32; 4]| -> u8 {
        let k = [
            c[0].round().clamp(0.0, 255.0) as u8,
            c[1].round().clamp(0.0, 255.0) as u8,
            c[2].round().clamp(0.0, 255.0) as u8,
            c[3].round().clamp(0.0, 255.0) as u8,
        ];
        *cache.entry(k).or_insert_with(|| {
            nearest(pal, [k[0] as f32, k[1] as f32, k[2] as f32, k[3] as f32]) as u8
        })
    };
    let tof = |p: &Px| [p[0] as f32, p[1] as f32, p[2] as f32, p[3] as f32];
    match dither {
        Dither::None => {
            for (i, p) in px.iter().enumerate() {
                idx[i] = lookup(tof(p));
            }
        }
        Dither::Ordered => {
            // Spreizung nach Palettengröße (wie Abstand der Farben in einem gleichmäßigen Würfel)
            let spread = 255.0 / (pal.len() as f32).cbrt().max(2.0) * amount;
            for (i, p) in px.iter().enumerate() {
                let (x, y) = (i % w, i / w);
                let t = (BAYER8[y % 8][x % 8] as f32 + 0.5) / 64.0 - 0.5;
                let c = tof(p);
                let c = if p[3] == 0 {
                    c
                } else {
                    [
                        c[0] + t * spread,
                        c[1] + t * spread,
                        c[2] + t * spread,
                        c[3],
                    ]
                };
                idx[i] = lookup(c);
            }
        }
        Dither::FloydSteinberg => {
            let mut err = vec![[0f32; 4]; (w + 2) * 2];
            for y in 0..h {
                let ltr = y % 2 == 0;
                let (cur, next) = if y % 2 == 0 { (0, w + 2) } else { (w + 2, 0) };
                for e in &mut err[next..next + w + 2] {
                    *e = [0.0; 4];
                }
                for k in 0..w {
                    let x = if ltr { k } else { w - 1 - k };
                    let i = y * w + x;
                    let p = px[i];
                    let e = err[cur + x + 1];
                    let c = tof(&p);
                    let c = if p[3] == 0 {
                        c
                    } else {
                        [c[0] + e[0], c[1] + e[1], c[2] + e[2], c[3] + e[3]]
                    };
                    let j = lookup(c);
                    idx[i] = j;
                    let q = pal[j as usize];
                    let d = [
                        (c[0] - q[0] as f32) * amount,
                        (c[1] - q[1] as f32) * amount,
                        (c[2] - q[2] as f32) * amount,
                        (c[3] - q[3] as f32) * amount,
                    ];
                    let (fwd, back): (isize, isize) = if ltr { (1, -1) } else { (-1, 1) };
                    let at = |off: isize| (x as isize + 1 + off) as usize;
                    for ch in 0..4 {
                        err[cur + at(fwd)][ch] += d[ch] * 7.0 / 16.0;
                        err[next + at(back)][ch] += d[ch] * 3.0 / 16.0;
                        err[next + at(0)][ch] += d[ch] * 5.0 / 16.0;
                        err[next + at(fwd)][ch] += d[ch] / 16.0;
                    }
                }
            }
        }
    }
    for (p, &i) in px.iter_mut().zip(&idx) {
        *p = pal[i as usize];
    }
    idx
}

/// Reduziert die Farben eines RGBA-Puffers auf höchstens `n` (in place). Liefert die Palette.
pub fn quantize(px: &mut [Px], w: usize, n: usize, dither: Dither, amount: f32) -> Vec<Px> {
    let pal = palette(px, n);
    remap(px, w, &pal, dither, amount.clamp(0.0, 1.0));
    pal
}
