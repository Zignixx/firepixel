//! 3×3-Faltung (`@00b2fd60`/`@00b2df50`, 06 §3.2) und die Filter darauf: Weichzeichnen,
//! Stark weichzeichnen, Einfach weichzeichnen, Schärfen, Stark schärfen, Kanten finden.
//!
//! Je Kanal getrennt, nicht vormultipliziert, ganzzahlig: `v = (Σ kᵢ·pᵢ) / teiler − versatz`
//! (C-Division zur Null hin), auf 0…255 begrenzt. Außerhalb der Bildgrenzen werden die Randpixel
//! wiederholt. Mit Flag „Alpha“ wird Alpha mitgefaltet, sonst bleibt das Alpha des Mittelpixels.
//! Die Farbe transparenter Pixel geht ein.

use fp_geom::IRect;

use super::{PixelFilter, start};
use crate::image::RgbaImage;
use crate::mask::Mask;
use crate::par::for_each_row;

/// Faltet `region` von `src` mit `kernel` (Zeilen oben→unten).
pub fn convolve3x3(
    src: &RgbaImage,
    region: IRect,
    kernel: [i32; 9],
    divisor: i32,
    offset: i32,
    with_alpha: bool,
) -> RgbaImage {
    let mut out = start(src);
    let b = src.bounds();
    let r = region.intersect(&b);
    if r.is_empty() || divisor == 0 {
        return out;
    }
    let (w, h) = (src.width() as i64, src.height() as i64);
    let w_us = src.width();
    let sdata = src.data();
    let (x0, x1) = ((r.left - b.left) as usize, (r.right - b.left) as usize);
    let (y0, y1) = ((r.top - b.top) as usize, (r.bottom - b.top) as usize);
    for_each_row(&mut out.data_mut()[y0 * w_us..y1 * w_us], w_us, |i, row| {
        let y = (y0 + i) as i64;
        let rows = [(y - 1).clamp(0, h - 1), y, (y + 1).clamp(0, h - 1)].map(|v| v as usize * w_us);
        for x in x0..x1 {
            let xs = [
                (x as i64 - 1).clamp(0, w - 1) as usize,
                x,
                (x as i64 + 1).clamp(0, w - 1) as usize,
            ];
            let mut acc = [0i32; 4];
            for (ky, ro) in rows.iter().enumerate() {
                for (kx, &xx) in xs.iter().enumerate() {
                    let k = kernel[ky * 3 + kx];
                    let p = sdata[ro + xx];
                    for c in 0..4 {
                        acc[c] += k * p[c] as i32;
                    }
                }
            }
            let mut o = [0u8; 4];
            for c in 0..4 {
                o[c] = (acc[c] / divisor - offset).clamp(0, 255) as u8;
            }
            if !with_alpha {
                o[3] = sdata[y as usize * w_us + x][3];
            }
            row[x] = o;
        }
    });
    out
}

/// „Einfach weichzeichnen“ (`CSimpleBlurFilter`, `@00b26fa0`, intern): Nachbarn 1, Mitte
/// `11 − s`, Teiler `19 − s`, mit Alpha; `s = clamp(amount, 1, 10)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SimpleBlurParams {
    /// `simple_blur_amount` (1…10, fehlend 0 → 1).
    pub amount: i32,
}

impl PixelFilter for SimpleBlurParams {
    fn process(&self, src: &RgbaImage, region: IRect) -> RgbaImage {
        let a = self.amount.clamp(1, 10);
        convolve3x3(
            src,
            region,
            [1, 1, 1, 1, 11 - a, 1, 1, 1, 1],
            19 - a,
            0,
            true,
        )
    }
}

/// „Weichzeichnen“ (Stärke 3: Mitte 8, Teiler 16).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct Blur;

impl PixelFilter for Blur {
    fn process(&self, src: &RgbaImage, region: IRect) -> RgbaImage {
        SimpleBlurParams { amount: 3 }.process(src, region)
    }
}

/// „Stärker weichzeichnen“ (Stärke 7: Mitte 4, Teiler 12).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct BlurMore;

impl PixelFilter for BlurMore {
    fn process(&self, src: &RgbaImage, region: IRect) -> RgbaImage {
        SimpleBlurParams { amount: 7 }.process(src, region)
    }
}

/// Schärfen (`CSharpenFilter`/`CSharpenMoreFilter`, `@00b26390`): Nachbarn −1, Mitte `19 − s`,
/// Teiler `11 − s`, mit Alpha, `s = clamp(amount, 1, 10)`.
///
/// Als **Live-Filter** fehlt `sharpen_amount` in der Datei → 0 → `s = 1` (Mitte 18, Teiler 10)
/// für beide Varianten (06 §4.9, B Laufzeit T3). Der **Menübefehl** nutzt den Prototyp mit 4
/// bzw. 8 ([`SharpenParams::MENU`], [`SharpenParams::MENU_MORE`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SharpenParams {
    /// `sharpen_amount` (0 = fehlt → 1).
    pub amount: i32,
}

impl SharpenParams {
    /// Live-Filter (beide Varianten): Stärke aus der Datei, normalerweise fehlend.
    pub const LIVE: SharpenParams = SharpenParams { amount: 0 };
    /// Menü „Scharfzeichnen“.
    pub const MENU: SharpenParams = SharpenParams { amount: 4 };
    /// Menü „Stark scharfzeichnen“.
    pub const MENU_MORE: SharpenParams = SharpenParams { amount: 8 };
}

impl PixelFilter for SharpenParams {
    fn process(&self, src: &RgbaImage, region: IRect) -> RgbaImage {
        let a = self.amount.clamp(1, 10);
        convolve3x3(
            src,
            region,
            [-1, -1, -1, -1, 19 - a, -1, -1, -1, -1],
            11 - a,
            0,
            true,
        )
    }
}

/// „Ränder suchen“ (`CFindEdgeFilter`, `@00b1b7c0`): Laplace (Nachbarn 1, Mitte −8), Teiler 1,
/// ohne Alpha.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct FindEdges;

impl PixelFilter for FindEdges {
    fn process(&self, src: &RgbaImage, region: IRect) -> RgbaImage {
        convolve3x3(src, region, [1, 1, 1, 1, -8, 1, 1, 1, 1], 1, 0, false)
    }
}

/// „Weichzeichnen“ anwenden.
pub fn blur(img: &RgbaImage, mask: Option<&Mask>) -> RgbaImage {
    Blur.apply(img, mask)
}

/// „Stärker weichzeichnen“ anwenden.
pub fn blur_more(img: &RgbaImage, mask: Option<&Mask>) -> RgbaImage {
    BlurMore.apply(img, mask)
}

/// Schärfen mit Parametern anwenden.
pub fn sharpen(img: &RgbaImage, p: &SharpenParams, mask: Option<&Mask>) -> RgbaImage {
    p.apply(img, mask)
}

/// „Ränder suchen“ anwenden.
pub fn find_edges(img: &RgbaImage, mask: Option<&Mask>) -> RgbaImage {
    FindEdges.apply(img, mask)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fp_color::Rgba8;

    #[test]
    fn flat_image_unchanged_by_blur_and_sharpen() {
        let img = RgbaImage::filled(IRect::new(3, 4, 9, 8), Rgba8::new(40, 80, 120, 200)).unwrap();
        assert_eq!(Blur.apply(&img, None), img);
        assert_eq!(SharpenParams::LIVE.apply(&img, None), img);
        let e = FindEdges.apply(&img, None);
        assert!(e.data().iter().all(|p| *p == [0, 0, 0, 200]));
    }
}
