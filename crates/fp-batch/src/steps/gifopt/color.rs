//! Farbraum Oklab (Björn Ottosson 2020, öffentlich beschriebene Matrizen) für Abstände,
//! Palettenbildung und Dithering. sRGB ↔ linear über eine Tabelle bzw. die Normformel.

use std::sync::OnceLock;

/// Farbe in Oklab (`L` 0…1, `a`/`b` etwa ±0,4).
pub type Lab = [f32; 3];

fn lin_table() -> &'static [f32; 256] {
    static T: OnceLock<[f32; 256]> = OnceLock::new();
    T.get_or_init(|| {
        let mut t = [0f32; 256];
        for (i, v) in t.iter_mut().enumerate() {
            let c = i as f32 / 255.0;
            *v = if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            };
        }
        t
    })
}

/// sRGB (8 Bit) → linear 0…1.
#[inline]
pub fn to_linear(c: u8) -> f32 {
    lin_table()[c as usize]
}

/// linear → sRGB 0…255 (gerundet, begrenzt).
#[inline]
pub fn from_linear(x: f32) -> u8 {
    let x = x.clamp(0.0, 1.0);
    let s = if x <= 0.003_130_8 {
        x * 12.92
    } else {
        1.055 * x.powf(1.0 / 2.4) - 0.055
    };
    (s * 255.0 + 0.5).clamp(0.0, 255.0) as u8
}

/// sRGB → Oklab.
pub fn rgb_to_lab(rgb: [u8; 3]) -> Lab {
    let (r, g, b) = (to_linear(rgb[0]), to_linear(rgb[1]), to_linear(rgb[2]));
    let l = 0.412_221_46 * r + 0.536_332_55 * g + 0.051_445_995 * b;
    let m = 0.211_903_5 * r + 0.680_699_5 * g + 0.107_396_96 * b;
    let s = 0.088_302_46 * r + 0.281_718_85 * g + 0.629_978_7 * b;
    let (l, m, s) = (l.cbrt(), m.cbrt(), s.cbrt());
    [
        0.210_454_26 * l + 0.793_617_8 * m - 0.004_072_047 * s,
        1.977_998_5 * l - 2.428_592_2 * m + 0.450_593_7 * s,
        0.025_904_037 * l + 0.782_771_77 * m - 0.808_675_77 * s,
    ]
}

/// Oklab → sRGB (außerhalb des Farbraums begrenzt).
pub fn lab_to_rgb(c: Lab) -> [u8; 3] {
    let l = c[0] + 0.396_337_78 * c[1] + 0.215_803_76 * c[2];
    let m = c[0] - 0.105_561_346 * c[1] - 0.063_854_17 * c[2];
    let s = c[0] - 0.089_484_18 * c[1] - 1.291_485_5 * c[2];
    let (l, m, s) = (l * l * l, m * m * m, s * s * s);
    [
        from_linear(4.076_741_7 * l - 3.307_711_6 * m + 0.230_969_94 * s),
        from_linear(-1.268_438 * l + 2.609_757_4 * m - 0.341_319_38 * s),
        from_linear(-0.004_196_086_3 * l - 0.703_418_6 * m + 1.707_614_7 * s),
    ]
}

/// Quadrierter Abstand in Oklab.
#[inline]
pub fn dist2(a: &Lab, b: &Lab) -> f32 {
    let (d0, d1, d2) = (a[0] - b[0], a[1] - b[1], a[2] - b[2]);
    d0 * d0 + d1 * d1 + d2 * d2
}

/// Quadrierter Wahrnehmungsabstand: Oklab mit `wl`-fach gewichteter Helligkeit (BATCH-8b; `wl` = 1
/// ist der gewöhnliche Oklab-Abstand).
#[inline]
pub fn wdist2(a: &Lab, b: &Lab, wl: f32) -> f32 {
    let (d0, d1, d2) = ((a[0] - b[0]) * wl, a[1] - b[1], a[2] - b[2]);
    d0 * d0 + d1 * d1 + d2 * d2
}
/// Abstand in Oklab.
#[inline]
pub fn dist(a: &Lab, b: &Lab) -> f32 {
    dist2(a, b).sqrt()
}

/// RGB als `u32` (0x00RRGGBB).
#[inline]
pub fn pack(rgb: [u8; 3]) -> u32 {
    (rgb[0] as u32) << 16 | (rgb[1] as u32) << 8 | rgb[2] as u32
}

/// `u32` → RGB.
#[inline]
pub fn unpack(c: u32) -> [u8; 3] {
    [(c >> 16) as u8, (c >> 8) as u8, c as u8]
}

/// Schneller Hasher für Farbschlüssel (`u32`), Multiplikations-Hash (Fibonacci).
#[derive(Default, Clone, Copy)]
pub struct ColorHasher(u64);

impl std::hash::Hasher for ColorHasher {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 = (self.0.rotate_left(8) ^ *b as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        }
    }
    fn write_u32(&mut self, i: u32) {
        self.0 = (i as u64 ^ self.0).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    }
    fn write_u64(&mut self, i: u64) {
        self.0 = (i ^ self.0).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    }
}

/// `HashMap` mit [`ColorHasher`].
pub type ColorMap<V> =
    std::collections::HashMap<u32, V, std::hash::BuildHasherDefault<ColorHasher>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_all_greys_and_primaries() {
        for v in [0u8, 1, 17, 128, 200, 254, 255] {
            assert_eq!(lab_to_rgb(rgb_to_lab([v, v, v])), [v, v, v]);
        }
        for c in [[255, 0, 0], [0, 255, 0], [0, 0, 255], [12, 200, 77]] {
            let back = lab_to_rgb(rgb_to_lab(c));
            for k in 0..3 {
                assert!(
                    (back[k] as i32 - c[k] as i32).abs() <= 1,
                    "{c:?} → {back:?}"
                );
            }
        }
        let w = rgb_to_lab([255, 255, 255]);
        assert!((w[0] - 1.0).abs() < 1e-3 && w[1].abs() < 1e-3 && w[2].abs() < 1e-3);
    }
}
