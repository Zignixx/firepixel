//! „Rauschen hinzufügen“ (`CAddNoiseFilter`, 06 §4.15).
//!
//! Deterministisch: Tabelle aus `N = min(32768, Fläche)` Werten eines Park-Miller-Generators
//! (Startwert `0x1F7DF`), je Original-Pufferkachel ein Startindex aus der **Dokument**-Koordinate
//! der Kachelecke (B Laufzeit T2: 186.000/186.000, 182.100/182.100, 20.000/20.000). Das
//! Kachelmaß ist das des `CPixelBuffer` ([`pixelbuffer_tile`]), Raster ab der linken oberen Ecke
//! des Bildes.

use fp_geom::IRect;

use super::{start, PixelFilter};
use crate::image::RgbaImage;
use crate::mask::Mask;

/// Parameter von „Rauschen hinzufügen…“.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct AddNoiseParams {
    /// `add_noise_amount` in % (Dialog 1…400), Standard 50.
    pub amount: i32,
    /// `add_noise_use_color`: drei Werte je Pixel statt einem.
    pub use_color: bool,
}

impl Default for AddNoiseParams {
    fn default() -> Self {
        Self { amount: 50, use_color: false }
    }
}

impl AddNoiseParams {
    /// Dialogbereich der Menge.
    pub const AMOUNT_RANGE: (i32, i32) = (1, 400);

    /// Menge auf den Dialogbereich begrenzt.
    pub fn dialog_clamped(self) -> Self {
        Self { amount: self.amount.clamp(1, 400), ..self }
    }
}

/// Ein Schritt „minstd“ in 32-Bit-Arithmetik wie `@006e2130`:
/// `x·48271 − (x/44488)·0x7FFFFFFF`, Ergebnis `< 1` → `+ 0x7FFFFFFF`.
#[inline]
pub fn minstd(x: i32) -> i32 {
    let v = x.wrapping_mul(48271).wrapping_sub((x / 44488).wrapping_mul(0x7FFF_FFFF));
    if v < 1 { v.wrapping_add(0x7FFF_FFFF) } else { v }
}

/// Rauschtabelle mit `n` Einträgen (16 Bit, vorzeichenbehaftet) für die Menge `amount`.
pub fn noise_table(amount: i32, n: usize) -> Vec<i16> {
    let a = amount.wrapping_mul(255) / 100;
    let m = (a.wrapping_mul(2).wrapping_add(1)) as u32;
    let mut seed = 0x1F7DFi32;
    (0..n)
        .map(|_| {
            seed = minstd(seed);
            let r = if m == 0 { seed as u32 } else { (seed as u32) % m };
            (r as i32).wrapping_sub(a) as i16
        })
        .collect()
}

/// Kachelmaß eines `CPixelBuffer` (`@00631650`): Breite ≤ 499 → Bildbreite, sonst 500;
/// Höhe `min(H, 250000 / Breite)`; mindestens 1.
pub fn pixelbuffer_tile(width: usize, height: usize) -> (usize, usize) {
    let tw = if width <= 499 { width } else { 500 };
    let tw = tw.max(1);
    let th = height.min(250_000 / tw).max(1);
    (tw, th)
}

impl PixelFilter for AddNoiseParams {
    fn process(&self, src: &RgbaImage, region: IRect) -> RgbaImage {
        let mut out = start(src);
        let b = src.bounds();
        let r = region.intersect(&b);
        if r.is_empty() {
            return out;
        }
        let n = (r.area() as usize).min(0x8000);
        let tab = noise_table(self.amount, n);
        let (tw, th) = pixelbuffer_tile(src.width(), src.height());
        let w = src.width();
        let data = out.data_mut();
        let per_px = if self.use_color { 3 } else { 1 };
        let mut ty = 0usize;
        while ty < src.height() {
            let mut tx = 0usize;
            while tx < w {
                let tile = IRect::new(b.left + tx as i32, b.top + ty as i32, b.left + (tx + tw) as i32, b.top + (ty + th) as i32);
                let part = tile.intersect(&r);
                if !part.is_empty() {
                    let seed = ((part.left as u32) << 16) | (part.top as u32);
                    let mut pos = (minstd(seed as i32) as i64).rem_euclid(n as i64) as usize;
                    for y in part.top..part.bottom {
                        for x in part.left..part.right {
                            let i = (y - b.top) as usize * w + (x - b.left) as usize;
                            let p = &mut data[i];
                            if per_px == 1 {
                                let v = tab[pos] as i32;
                                pos = if pos + 1 == n { 0 } else { pos + 1 };
                                for c in 0..3 {
                                    p[c] = (p[c] as i32 + v).clamp(0, 255) as u8;
                                }
                            } else {
                                for c in 0..3 {
                                    let v = tab[pos] as i32;
                                    pos = if pos + 1 == n { 0 } else { pos + 1 };
                                    p[c] = (p[c] as i32 + v).clamp(0, 255) as u8;
                                }
                            }
                        }
                    }
                }
                tx += tw;
            }
            ty += th;
        }
        out
    }
}

/// „Rauschen hinzufügen…“ anwenden. Das Ergebnis hängt von der Lage des Bildes in
/// Dokumentkoordinaten ab.
pub fn add_noise(img: &RgbaImage, p: &AddNoiseParams, mask: Option<&Mask>) -> RgbaImage {
    p.apply(img, mask)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minstd_and_tiles() {
        // Park-Miller mit 48271: erster Schritt ab 1
        assert_eq!(minstd(1), 48271);
        assert_eq!(pixelbuffer_tile(620, 300), (500, 300));
        assert_eq!(pixelbuffer_tile(200, 100), (200, 100));
        assert_eq!(pixelbuffer_tile(499, 1000), (499, 501));
        let t = noise_table(50, 10);
        assert!(t.iter().all(|&v| (-127..=127).contains(&v)));
    }
}
