//! Farbfilter: Helligkeit/Kontrast (06 §4.1), Farbton/Sättigung inkl. Kolorieren (§4.2),
//! Farbfüllung (§5, Live-Effekt „Color Fill“) und In Alpha konvertieren (§4.16).
//!
//! Gleitkomma wie das Original mit 53-Bit-Genauigkeit (`f64`), an den markierten Stellen `f32`
//! (Kontrast < 0). Zurückschreiben schneidet ab. Keine FMA (`mul_add` wird nicht benutzt).

use fp_color::blend::blend_rgb;
use fp_color::round::{DIV, div255, round_away};
use fp_color::{BlendMode, Rgba8};
use fp_geom::IRect;

use super::{PixelFilter, start};
use crate::image::{Px, RgbaImage};
use crate::mask::Mask;
use crate::par::for_each_row;

/// Wendet `f` auf jedes Pixel in `region` an (Kopie außerhalb).
pub(crate) fn map_region<F>(src: &RgbaImage, region: IRect, f: F) -> RgbaImage
where
    F: Fn(Px) -> Px + Sync + Send,
{
    let mut out = start(src);
    let b = out.bounds();
    let w = out.width();
    let r = region.intersect(&b);
    if r.is_empty() {
        return out;
    }
    let (x0, x1) = ((r.left - b.left) as usize, (r.right - b.left) as usize);
    let (y0, y1) = ((r.top - b.top) as usize, (r.bottom - b.top) as usize);
    for_each_row(&mut out.data_mut()[y0 * w..y1 * w], w, |_, row| {
        for p in &mut row[x0..x1] {
            *p = f(*p);
        }
    });
    out
}

// ------------------------------------------------------------------ Helligkeit/Kontrast

/// Faktortabelle für Kontrast > 0 (`@00b14120`, 101 Einträge; Index 100 wird gesondert behandelt).
pub const CONTRAST_TABLE: [f64; 101] = [
    0.0, 0.01, 0.02, 0.04, 0.05, 0.06, 0.07, 0.08, 0.1, 0.11, 0.12, 0.14, 0.15, 0.16, 0.17, 0.18,
    0.2, 0.21, 0.22, 0.24, 0.25, 0.27, 0.28, 0.3, 0.32, 0.34, 0.36, 0.38, 0.4, 0.42, 0.44, 0.46,
    0.48, 0.5, 0.53, 0.56, 0.59, 0.62, 0.65, 0.68, 0.71, 0.74, 0.77, 0.8, 0.83, 0.86, 0.89, 0.92,
    0.95, 0.98, 1.0, 1.06, 1.12, 1.18, 1.24, 1.3, 1.36, 1.42, 1.48, 1.54, 1.6, 1.66, 1.72, 1.78,
    1.84, 1.9, 1.96, 2.0, 2.12, 2.25, 2.37, 2.5, 2.62, 2.75, 2.87, 3.0, 3.2, 3.4, 3.6, 3.8, 4.0,
    4.3, 4.7, 4.9, 5.0, 5.5, 6.0, 6.5, 6.8, 7.0, 7.3, 7.5, 7.8, 8.0, 8.4, 8.7, 9.0, 9.4, 9.6, 9.8,
    -1.0,
];

/// Parameter von „Helligkeit/Kontrast…“ (`CBrightnessFilter`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct BrightnessContrastParams {
    /// `brightness_amount`, −100…100.
    pub brightness: i32,
    /// `contrast_amount`, −100…100.
    pub contrast: i32,
}

impl BrightnessContrastParams {
    /// Bereich beider Regler (Dialog und Code).
    pub const RANGE: (i32, i32) = (-100, 100);

    /// Mittlere Helligkeit über die Pixel mit Alpha ≠ 0 in `region`:
    /// `kaufm(0,1·B̄ + 0,3·R̄ + 0,6·Ḡ)`; `None`, wenn es keine solchen Pixel gibt.
    pub fn mean(src: &RgbaImage, region: IRect) -> Option<f64> {
        let r = region.intersect(&src.bounds());
        let (mut n, mut sr, mut sg, mut sb) = (0u64, 0u64, 0u64, 0u64);
        for y in r.top..r.bottom {
            let row = src.row(y).unwrap_or(&[]);
            for p in
                &row[(r.left - src.bounds().left) as usize..(r.right - src.bounds().left) as usize]
            {
                if p[3] != 0 {
                    n += 1;
                    sr += p[0] as u64;
                    sg += p[1] as u64;
                    sb += p[2] as u64;
                }
            }
        }
        if n == 0 {
            return None;
        }
        let n = n as f64;
        let m = (sb as f64 / n) * 0.1 + (sr as f64 / n) * 0.3 + (sg as f64 / n) * 0.6;
        Some(round_away(m))
    }

    /// Ein Kanalwert (nach Helligkeitsverschiebung) mit Kontrast `c` gegen den Mittelwert `mean`.
    fn contrast_channel(v: i64, mean: f64, c: i32) -> i64 {
        if c == 0 {
            return v;
        }
        if c < 0 {
            let d = (v as f64 - mean) as f32;
            let t = ((d as f64 / 100.0) as f32).abs();
            let r = round_away(c as f64 * t as f64) as i64;
            let vf = v as f64;
            return if vf > mean {
                v + r
            } else if vf < mean {
                v - r
            } else {
                v
            };
        }
        let vf = v as f64;
        if c == 100 {
            return if vf >= mean { 255 } else { 0 };
        }
        let k = CONTRAST_TABLE[c as usize];
        if vf >= mean {
            let up = round_away((vf - mean) * k) as i64;
            255.min(v.max(v + up))
        } else {
            let dn = round_away((mean - vf) * k) as i64;
            v.min(0.max(v - dn))
        }
    }
}

impl PixelFilter for BrightnessContrastParams {
    fn process(&self, src: &RgbaImage, region: IRect) -> RgbaImage {
        let b = self.brightness.clamp(-100, 100) as i64;
        let c = self.contrast.clamp(-100, 100);
        // Ohne deckende Pixel gibt es keinen Mittelwert; dann wirkt nur die Helligkeit (A).
        let mean = if c != 0 {
            Self::mean(src, region)
        } else {
            None
        };
        let c = if mean.is_some() { c } else { 0 };
        let m = mean.unwrap_or(0.0);
        map_region(src, region, |p| {
            let mut o = p;
            for ch in 0..3 {
                let v = (p[ch] as i64 + b).clamp(0, 255);
                o[ch] = (Self::contrast_channel(v, m, c) & 0xFF) as u8;
            }
            o
        })
    }
}

/// „Helligkeit/Kontrast…“ anwenden.
pub fn brightness_contrast(
    img: &RgbaImage,
    p: &BrightnessContrastParams,
    mask: Option<&Mask>,
) -> RgbaImage {
    p.apply(img, mask)
}

// ------------------------------------------------------------------ Farbton/Sättigung

const INV255: f64 = 0.00392156862745098;
const INV360: f64 = 0.002777777777777778;
const INV60: f64 = 0.016666666666666666;

/// RGB (0…255) → H, L, S auf der Skala 0…255 (`@006d0930`; 360° = 255).
pub fn rgb_to_hls255(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    let r = r * INV255;
    let g = g * INV255;
    let b = b * INV255;
    let mx = r.max(g).max(b);
    let mn = r.min(g).min(b);
    let l = (mn + mx) * 0.5;
    let d = mx - mn;
    let (h, s) = if d == 0.0 {
        (0.0, 0.0)
    } else {
        let s = if l > 0.5 {
            d / ((2.0 - mx) - mn)
        } else {
            d / (mn + mx)
        };
        let mut h = if mx == r {
            (g - b) / d
        } else if mx == g {
            (b - r) / d + 2.0
        } else {
            (r - g) / d + 4.0
        };
        h *= 60.0;
        if h < 0.0 {
            h += 360.0;
        }
        (h, s)
    };
    (
        (h * INV360 * 255.0).clamp(0.0, 255.0),
        (l * 255.0).clamp(0.0, 255.0),
        (s * 255.0).clamp(0.0, 255.0),
    )
}

#[inline]
fn hls_val(m1: f64, m2: f64, h: f64) -> f64 {
    let h = if h > 360.0 {
        h - 360.0
    } else if h < 0.0 {
        h + 360.0
    } else {
        h
    };
    if h < 60.0 {
        (m2 - m1) * h * INV60 + m1
    } else if h < 180.0 {
        m2
    } else if h >= 240.0 {
        m1
    } else {
        (m2 - m1) * (240.0 - h) * INV60 + m1
    }
}

/// H, L, S (0…255) → RGB (0…255, Gleitkomma, begrenzt) (`@006cf420`, `@006cf380`).
pub fn hls255_to_rgb(hh: f64, ll: f64, ss: f64) -> (f64, f64, f64) {
    let h = hh * INV255 * 360.0;
    let l = ll * INV255;
    let s = ss * INV255;
    let (r, g, b) = if s == 0.0 {
        (l, l, l)
    } else {
        let m2 = if l >= 0.5 {
            (s + l) - s * l
        } else {
            (s + 1.0) * l
        };
        let m1 = (l + l) - m2;
        (
            hls_val(m1, m2, h + 120.0),
            hls_val(m1, m2, h),
            hls_val(m1, m2, h - 120.0),
        )
    };
    (
        (r * 255.0).clamp(0.0, 255.0),
        (g * 255.0).clamp(0.0, 255.0),
        (b * 255.0).clamp(0.0, 255.0),
    )
}

#[inline]
fn trunc_u8(x: f64) -> u8 {
    if x.is_finite() {
        ((x.trunc() as i64) & 0xFF) as u8
    } else {
        0
    }
}

/// Parameter von „Farbton/Sättigung…“ (`CHueFilter`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct HueSaturationParams {
    /// `hue_amount` in Grad: −180…180, beim Kolorieren 0…360.
    pub hue: i32,
    /// `saturation_amount` in %: −100…100, beim Kolorieren 0…100.
    pub saturation: i32,
    /// `lightness_amount` in %: −100…100.
    pub lightness: i32,
    /// `hls_colorize`.
    pub colorize: bool,
}

impl HueSaturationParams {
    /// Die vom Code erzwungenen Bereiche (06 §4.2), zugleich die Dialogbereiche.
    pub fn clamped(self) -> Self {
        if self.colorize {
            Self {
                hue: self.hue.clamp(0, 360),
                saturation: self.saturation.clamp(0, 100),
                lightness: self.lightness.clamp(-100, 100),
                colorize: true,
            }
        } else {
            Self {
                hue: self.hue.clamp(-180, 180),
                saturation: self.saturation.clamp(-100, 100),
                lightness: self.lightness.clamp(-100, 100),
                colorize: false,
            }
        }
    }

    /// Ein Pixel umrechnen (Alpha bleibt).
    pub fn apply_px(&self, p: Px) -> Px {
        let q = self.clamped();
        let (mut h, mut l, mut s) = rgb_to_hls255(p[0] as f64, p[1] as f64, p[2] as f64);
        if q.colorize {
            h = (q.hue as f64 / 360.0) * 255.0;
            s = (q.saturation as f64 / 100.0) * 255.0;
        } else {
            h = (((h * 360.0) * 0.00390625 + q.hue as f64) * 256.0 / 360.0) % 255.0;
            let sf = q.saturation as f64 / 100.0;
            s += sf * if sf >= 0.0 { 255.0 - s } else { s };
        }
        let lf = q.lightness as f64 / 100.0;
        l += lf * if lf >= 0.0 { 255.0 - l } else { l };
        let (r, g, b) = hls255_to_rgb(h, l, s);
        [trunc_u8(r), trunc_u8(g), trunc_u8(b), p[3]]
    }
}

impl PixelFilter for HueSaturationParams {
    fn process(&self, src: &RgbaImage, region: IRect) -> RgbaImage {
        let q = self.clamped();
        map_region(src, region, |p| q.apply_px(p))
    }
}

/// „Farbton/Sättigung…“ anwenden.
pub fn hue_saturation(img: &RgbaImage, p: &HueSaturationParams, mask: Option<&Mask>) -> RgbaImage {
    p.apply(img, mask)
}

// ------------------------------------------------------------------ Farbfüllung

/// Parameter des Live-Effekts „Farbfüllung“ (`CDbColorFill`, 06 §5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ColorFillParams {
    /// `Color` (`#RRGGBB`, Alpha wird ignoriert).
    pub color: Rgba8,
    /// `Opacity` in %, 0…100.
    pub opacity: i32,
    /// `Blendmode` (interne Nummer = `BLDi`).
    pub blend_mode: BlendMode,
}

impl Default for ColorFillParams {
    fn default() -> Self {
        Self {
            color: Rgba8::rgb(255, 0, 0),
            opacity: 100,
            blend_mode: BlendMode(0),
        }
    }
}

impl ColorFillParams {
    /// Dialogbereich der Deckkraft.
    pub const OPACITY_RANGE: (i32, i32) = (0, 100);

    /// Ein Pixel füllen (`StBlendConstSrcPreserveAlpha`). Nur Pixel mit Alpha ≠ 0, Alpha bleibt.
    /// Andere Mischmodi als Normal: erst die Mischfarbe `B(Farbe, Pixel)` (fp-color), dann dieselbe
    /// Überblendung (A).
    pub fn apply_px(&self, p: Px) -> Px {
        let a = p[3] as i64;
        if a == 0 {
            return p;
        }
        let o = ((self.opacity.wrapping_mul(255)) / 100) & 0xFF;
        if o == 0 {
            return p;
        }
        let col = [self.color.r, self.color.g, self.color.b];
        let col = if self.blend_mode.0 == 0 {
            col
        } else {
            blend_rgb(self.blend_mode, col, [p[0], p[1], p[2]])
        };
        if o == 255 && self.blend_mode.0 == 0 {
            return [col[0], col[1], col[2], p[3]];
        }
        let o = o as i64;
        let t = div255((a * o) as u32) as i64;
        let ra = (o - t + a) & 0xFF;
        let w = if a == 255 {
            o
        } else {
            DIV[o as usize][ra as usize] as i64
        };
        let ch = |c: u8, f: u8| {
            let (c, f) = (c as i64, f as i64);
            let x = (f - c) * w + 0x80 + c * 255;
            (((x + (x >> 8)) >> 8) & 0xFF) as u8
        };
        [ch(p[0], col[0]), ch(p[1], col[1]), ch(p[2], col[2]), p[3]]
    }
}

impl PixelFilter for ColorFillParams {
    fn process(&self, src: &RgbaImage, region: IRect) -> RgbaImage {
        let q = *self;
        map_region(src, region, move |p| q.apply_px(p))
    }
}

/// Farbfüllung anwenden.
pub fn color_fill(img: &RgbaImage, p: &ColorFillParams, mask: Option<&Mask>) -> RgbaImage {
    p.apply(img, mask)
}

// ------------------------------------------------------------------ In Alpha konvertieren

/// „In Alphakanal konvertieren“ (`CCreateAlphaFilter`, `@00b15d90`):
/// `L = (109·R + 366·G + 37·B) >> 9`, `A' = div255(A·(255−L))`, `RGB' = 0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct ConvertToAlpha;

impl PixelFilter for ConvertToAlpha {
    fn process(&self, src: &RgbaImage, region: IRect) -> RgbaImage {
        map_region(src, region, |p| {
            let l = (p[0] as u32 * 109 + p[1] as u32 * 366 + p[2] as u32 * 37) >> 9;
            let x = p[3] as u32 * ((255 - l) & 0xFF) + 0x80;
            [0, 0, 0, ((((x >> 8) + x) >> 8) & 0xFF) as u8]
        })
    }
}

/// „In Alphakanal konvertieren“ anwenden.
pub fn convert_to_alpha(img: &RgbaImage, mask: Option<&Mask>) -> RgbaImage {
    ConvertToAlpha.apply(img, mask)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hls_roundtrip_gray_and_colors() {
        let q = HueSaturationParams::default();
        for p in [
            [0, 0, 0, 255],
            [255, 255, 255, 9],
            [200, 10, 30, 255],
            [12, 250, 99, 0],
        ] {
            let o = q.apply_px(p);
            for c in 0..3 {
                assert!(o[c].abs_diff(p[c]) <= 1, "{p:?} → {o:?}");
            }
            assert_eq!(o[3], p[3]);
        }
        let gray = HueSaturationParams {
            saturation: -100,
            ..Default::default()
        }
        .apply_px([200, 10, 30, 255]);
        assert_eq!(gray[0], gray[1]);
    }

    #[test]
    fn contrast_extremes() {
        let img = RgbaImage::from_raw(
            IRect::new(0, 0, 2, 1),
            vec![[10, 10, 10, 255], [250, 250, 250, 255]],
        )
        .unwrap();
        let p = BrightnessContrastParams {
            brightness: 0,
            contrast: 100,
        };
        let o = p.apply(&img, None);
        assert_eq!(o.data(), &[[0, 0, 0, 255], [255, 255, 255, 255]]);
        let p = BrightnessContrastParams {
            brightness: 0,
            contrast: -100,
        };
        let o = p.apply(&img, None);
        assert_eq!(o.data()[0], o.data()[1]);
    }

    #[test]
    fn color_fill_and_alpha() {
        let f = ColorFillParams {
            color: Rgba8::rgb(0, 0, 0),
            opacity: 100,
            blend_mode: BlendMode(0),
        };
        assert_eq!(f.apply_px([9, 9, 9, 7]), [0, 0, 0, 7]);
        assert_eq!(f.apply_px([9, 9, 9, 0]), [9, 9, 9, 0]);
        let g = ColorFillParams { opacity: 0, ..f };
        assert_eq!(g.apply_px([9, 9, 9, 7]), [9, 9, 9, 7]);
        let img = RgbaImage::from_raw(
            IRect::new(0, 0, 2, 1),
            vec![[255, 255, 255, 255], [0, 0, 0, 255]],
        )
        .unwrap();
        let a = ConvertToAlpha.apply(&img, None);
        assert_eq!(a.data(), &[[0, 0, 0, 0], [0, 0, 0, 255]]);
    }
}
