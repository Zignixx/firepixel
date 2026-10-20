//! Qualitätsmaße für Messung und Tests (SPEC-BATCH §5.6 Punkt 7): SSIM auf Oklab-L (eigene Umsetzung
//! nach Wang et al. 2004, 8×8-Fenster mit Schrittweite 4) und PSNR auf RGB. Transparente Pixel werden
//! vorher auf Mittelgrau gelegt (Alpha ≥ 128 gilt als deckend, wie im GIF).

use super::color::rgb_to_lab;

fn flat(p: [u8; 4]) -> [u8; 3] {
    if p[3] >= 128 {
        [p[0], p[1], p[2]]
    } else {
        [128, 128, 128]
    }
}

/// SSIM zweier gleich großer Bilder (Oklab-L, 0…1; 1 = gleich).
pub fn ssim(a: &[[u8; 4]], b: &[[u8; 4]], w: usize, h: usize) -> f64 {
    if a.len() != b.len() || a.len() < w * h || w == 0 || h == 0 {
        return 0.0;
    }
    let la: Vec<f64> = a.iter().map(|p| rgb_to_lab(flat(*p))[0] as f64).collect();
    let lb: Vec<f64> = b.iter().map(|p| rgb_to_lab(flat(*p))[0] as f64).collect();
    ssim_l(&la, &lb, w, h)
}

/// Oklab L of a transparent pixel for [`ssim_l`] (mid grey, as in [`ssim`]).
pub fn transparent_l() -> f64 {
    rgb_to_lab([128, 128, 128])[0] as f64
}

/// SSIM on precomputed Oklab-L planes (the core of [`ssim`]; BA-110: the encoder's lossy LZW with
/// an SSIM bound measures each frame with it).
pub fn ssim_l(la: &[f64], lb: &[f64], w: usize, h: usize) -> f64 {
    ssim_l_in(la, lb, w, h, None)
}

/// Like [`ssim_l`], but only over the windows that overlap `region` (`[x, y, w, h]`; `None` = all).
/// No overlapping window (empty region) counts as 1.
pub fn ssim_l_in(la: &[f64], lb: &[f64], w: usize, h: usize, region: Option<[usize; 4]>) -> f64 {
    if la.len() != lb.len() || la.len() < w * h || w == 0 || h == 0 {
        return 0.0;
    }
    let (c1, c2) = (0.01f64 * 0.01, 0.03f64 * 0.03);
    let win = 8.min(w).min(h);
    let step = (win / 2).max(1);
    let (mut sum, mut cnt) = (0.0, 0.0);
    let mut y = 0;
    while y + win <= h {
        let mut x = 0;
        while x + win <= w {
            if let Some([rx, ry, rw, rh]) = region
                && !(x < rx + rw && x + win > rx && y < ry + rh && y + win > ry)
            {
                x += step;
                continue;
            }
            let (mut ma, mut mb, mut va, mut vb, mut cov) = (0.0, 0.0, 0.0, 0.0, 0.0);
            let n = (win * win) as f64;
            for yy in y..y + win {
                for xx in x..x + win {
                    ma += la[yy * w + xx];
                    mb += lb[yy * w + xx];
                }
            }
            ma /= n;
            mb /= n;
            for yy in y..y + win {
                for xx in x..x + win {
                    let (da, db) = (la[yy * w + xx] - ma, lb[yy * w + xx] - mb);
                    va += da * da;
                    vb += db * db;
                    cov += da * db;
                }
            }
            va /= n - 1.0;
            vb /= n - 1.0;
            cov /= n - 1.0;
            sum += ((2.0 * ma * mb + c1) * (2.0 * cov + c2))
                / ((ma * ma + mb * mb + c1) * (va + vb + c2));
            cnt += 1.0;
            x += step;
        }
        y += step;
    }
    if cnt > 0.0 { sum / cnt } else { 1.0 }
}

/// Farbtreue in der Fläche (BATCH-8b): mittlerer Oklab-Abstand der Mittelwerte von 8×8-Fenstern
/// (Schrittweite 4), ×100. Unempfindlich für feines Korn und Dithering, empfindlich für Farbflecken
/// und Farbverschiebungen, die SSIM auf L nicht sieht. 0 = gleich.
pub fn color_error(a: &[[u8; 4]], b: &[[u8; 4]], w: usize, h: usize) -> f64 {
    if a.len() != b.len() || a.len() < w * h || w == 0 || h == 0 {
        return 100.0;
    }
    let la: Vec<[f32; 3]> = a.iter().map(|p| rgb_to_lab(flat(*p))).collect();
    let lb: Vec<[f32; 3]> = b.iter().map(|p| rgb_to_lab(flat(*p))).collect();
    let win = 8.min(w).min(h);
    let step = (win / 2).max(1);
    let (mut sum, mut cnt) = (0.0f64, 0.0f64);
    let mut y = 0;
    while y + win <= h {
        let mut x = 0;
        while x + win <= w {
            let (mut ma, mut mb) = ([0f64; 3], [0f64; 3]);
            for yy in y..y + win {
                for xx in x..x + win {
                    for k in 0..3 {
                        ma[k] += la[yy * w + xx][k] as f64;
                        mb[k] += lb[yy * w + xx][k] as f64;
                    }
                }
            }
            let n = (win * win) as f64;
            sum += ((0..3).map(|k| ((ma[k] - mb[k]) / n).powi(2)).sum::<f64>()).sqrt();
            cnt += 1.0;
            x += step;
        }
        y += step;
    }
    if cnt > 0.0 { sum / cnt * 100.0 } else { 0.0 }
}

/// PSNR in dB auf RGB (∞ bei Gleichheit → 99).
pub fn psnr(a: &[[u8; 4]], b: &[[u8; 4]]) -> f64 {
    let mut se = 0f64;
    let mut n = 0f64;
    for (p, q) in a.iter().zip(b) {
        let (p, q) = (flat(*p), flat(*q));
        for k in 0..3 {
            let d = p[k] as f64 - q[k] as f64;
            se += d * d;
        }
        n += 3.0;
    }
    if se == 0.0 || n == 0.0 {
        return 99.0;
    }
    10.0 * (255.0f64 * 255.0 / (se / n)).log10()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssim_and_psnr_order_by_damage() {
        let (w, h) = (32, 32);
        let a: Vec<[u8; 4]> = (0..w * h)
            .map(|i| [(i % w * 8) as u8, (i / w * 8) as u8, 90, 255])
            .collect();
        let small: Vec<[u8; 4]> = a
            .iter()
            .map(|p| [p[0].saturating_add(2), p[1], p[2], 255])
            .collect();
        let big: Vec<[u8; 4]> = a
            .iter()
            .enumerate()
            .map(|(i, p)| if i % 3 == 0 { [255, 255, 255, 255] } else { *p })
            .collect();
        assert!((ssim(&a, &a, w, h) - 1.0).abs() < 1e-9);
        assert!(ssim(&a, &small, w, h) > ssim(&a, &big, w, h));
        assert!(psnr(&a, &small) > psnr(&a, &big));
        assert_eq!(psnr(&a, &a), 99.0);
    }
}
