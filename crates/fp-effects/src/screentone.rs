//! Screentone `{firepixel-screentone}` (Firepixel extension, FB-38; German UI "Rasterfolie"):
//! turns the object's rendered pixels into a halftone screen like the printed tones of black and
//! white illustrations and manga pages.
//!
//! The screen is a grid of square cells of `size` px, rotated by `angle` degrees and fixed to the
//! document grid (cell corners on the document origin, so the tone stays put when the window
//! scrolls). Each pixel takes a tone value `d` in 0..1: the darkness `1 − L/255` of the object
//! pixel (Rec. 601 luminance) when "from luminance" is on, otherwise the constant `density`. A
//! pixel is inked when its threshold inside its cell lies below `d`:
//!
//! - **Dots**: the threshold is the area of the cell that lies closer to the cell centre than the
//!   pixel, so dots grow round from the centre, touch at `d ≈ 0.785` and leave round holes at the
//!   cell corners above that; the inked share of a flat tone is `d`.
//! - **Lines**: the distance to the line through the cell centre (`2·|v|`), lines run along the
//!   rotated x axis; inked share `d`.
//! - **Cross-hatch**: two perpendicular line sets of width `w = 1 − √(1 − d)` each; inked share
//!   `1 − (1 − w)² = d`.
//!
//! Inked pixels take the screentone colour (alpha × the object's alpha) or, with "keep colour",
//! the object's own colour; all other pixels become transparent. The output is binary (no partial
//! coverage), the frame stays (`max_growth` 0).
//!
//! Parameters (all text like the other live filters): `screentone_type` 0 dots / 1 lines /
//! 2 cross-hatch, `screentone_size` 2..32, `screentone_angle` 0..180, `screentone_density`
//! 0..100, `screentone_from_luminance`, `screentone_color` `#rrggbb[aa]`,
//! `screentone_keep_color`.

use fp_color::Rgba8;
use fp_raster::RgbaImage;

use crate::params::Params;

/// Screen pattern of the Screentone filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScreentoneKind {
    /// Round dots (classic halftone).
    #[default]
    Dots,
    /// Parallel lines.
    Lines,
    /// Two perpendicular line sets.
    CrossHatch,
}

impl ScreentoneKind {
    /// From the file value (0 dots, 1 lines, 2 cross-hatch; others: dots).
    pub fn from_i64(v: i64) -> Self {
        match v {
            1 => Self::Lines,
            2 => Self::CrossHatch,
            _ => Self::Dots,
        }
    }

    /// File value.
    pub fn as_i64(self) -> i64 {
        match self {
            Self::Dots => 0,
            Self::Lines => 1,
            Self::CrossHatch => 2,
        }
    }
}

/// Parameters of the Screentone filter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreentoneParams {
    /// `screentone_type`.
    pub kind: ScreentoneKind,
    /// `screentone_size`: cell pitch in px (2..32).
    pub size: f64,
    /// `screentone_angle`: screen angle in degrees (0..180).
    pub angle: f64,
    /// `screentone_density`: constant tone in % when `from_luminance` is off.
    pub density: f64,
    /// `screentone_from_luminance`: tone from the object's luminance (default on).
    pub from_luminance: bool,
    /// `screentone_color`: ink colour (default black).
    pub color: Rgba8,
    /// `screentone_keep_color`: ink with the object's own colour instead of `color`.
    pub keep_color: bool,
}

impl Default for ScreentoneParams {
    fn default() -> Self {
        ScreentoneParams {
            kind: ScreentoneKind::Dots,
            size: Self::DEFAULT_SIZE,
            angle: Self::DEFAULT_ANGLE,
            density: 50.0,
            from_luminance: true,
            color: Rgba8::new(0, 0, 0, 255),
            keep_color: false,
        }
    }
}

impl ScreentoneParams {
    /// Default cell pitch in px.
    pub const DEFAULT_SIZE: f64 = 8.0;
    /// Default screen angle (the classic 45° screen of printed tones).
    pub const DEFAULT_ANGLE: f64 = 45.0;
    /// Range of the cell pitch.
    pub const SIZE_RANGE: (f64, f64) = (2.0, 32.0);
    /// Range of the angle.
    pub const ANGLE_RANGE: (f64, f64) = (0.0, 180.0);

    /// From the file parameters (missing values: defaults; values clamped to their ranges).
    pub fn read(p: Params) -> ScreentoneParams {
        let d = ScreentoneParams::default();
        ScreentoneParams {
            kind: ScreentoneKind::from_i64(p.int("screentone_type", 0)),
            size: p
                .num("screentone_size", d.size)
                .clamp(Self::SIZE_RANGE.0, Self::SIZE_RANGE.1),
            angle: p
                .num("screentone_angle", d.angle)
                .clamp(Self::ANGLE_RANGE.0, Self::ANGLE_RANGE.1),
            density: p.num("screentone_density", d.density).clamp(0.0, 100.0),
            from_luminance: p.flag("screentone_from_luminance", d.from_luminance),
            color: p.color("screentone_color", d.color),
            keep_color: p.flag("screentone_keep_color", d.keep_color),
        }
    }
}

/// Share of the unit cell closer to its centre than `rho` (cell units): the dot threshold.
fn dot_area(rho: f64) -> f64 {
    if rho <= 0.5 {
        std::f64::consts::PI * rho * rho
    } else if rho < std::f64::consts::FRAC_1_SQRT_2 {
        let r2 = rho * rho;
        (std::f64::consts::PI * r2 - 4.0 * (r2 * (0.5 / rho).acos() - 0.5 * (r2 - 0.25).sqrt()))
            .clamp(0.0, 1.0)
    } else {
        1.0
    }
}

/// Darkness 0..1 of a pixel from its Rec. 601 luminance.
#[inline]
fn darkness(p: [u8; 4]) -> f64 {
    let l = 0.299 * p[0] as f64 + 0.587 * p[1] as f64 + 0.114 * p[2] as f64;
    (1.0 - l / 255.0).clamp(0.0, 1.0)
}

/// Is the pixel at screen coordinates (u, v) (cell units, rotated document coordinates) inked at
/// tone `d`?
#[inline]
fn inked(kind: ScreentoneKind, u: f64, v: f64, d: f64) -> bool {
    if d <= 0.0 {
        return false;
    }
    if d >= 1.0 {
        return true;
    }
    // position inside the cell, centre = 0, range −0.5..0.5
    let fu = u - u.floor() - 0.5;
    let fv = v - v.floor() - 0.5;
    match kind {
        ScreentoneKind::Dots => dot_area((fu * fu + fv * fv).sqrt()) < d,
        ScreentoneKind::Lines => 2.0 * fv.abs() < d,
        ScreentoneKind::CrossHatch => {
            let w = 1.0 - (1.0 - d).sqrt();
            2.0 * fv.abs() < w || 2.0 * fu.abs() < w
        }
    }
}

/// Applies the Screentone filter to an object image (same frame).
pub fn screentone(obj: &RgbaImage, p: &ScreentoneParams) -> RgbaImage {
    let mut out = obj.clone();
    let b = obj.bounds();
    let w = obj.width();
    if w == 0 {
        return out;
    }
    let size = p.size.clamp(
        ScreentoneParams::SIZE_RANGE.0,
        ScreentoneParams::SIZE_RANGE.1,
    );
    let (sin, cos) = p.angle.to_radians().sin_cos();
    let constant = (p.density / 100.0).clamp(0.0, 1.0);
    for (j, row) in out.data_mut().chunks_mut(w).enumerate() {
        let cy = (b.top + j as i32) as f64 + 0.5;
        for (i, px) in row.iter_mut().enumerate() {
            let src = *px;
            if src[3] == 0 {
                *px = [0, 0, 0, 0];
                continue;
            }
            let cx = (b.left + i as i32) as f64 + 0.5;
            let u = (cx * cos + cy * sin) / size;
            let v = (cy * cos - cx * sin) / size;
            let d = if p.from_luminance {
                darkness(src)
            } else {
                constant
            };
            *px = if !inked(p.kind, u, v, d) {
                [0, 0, 0, 0]
            } else if p.keep_color {
                src
            } else {
                let a = (src[3] as u32 * p.color.a as u32 + 127) / 255;
                if a == 0 {
                    [0, 0, 0, 0]
                } else {
                    [p.color.r, p.color.g, p.color.b, a as u8]
                }
            };
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dot_area_is_monotonic_and_spans_the_cell() {
        assert_eq!(dot_area(0.0), 0.0);
        assert!((dot_area(0.5) - std::f64::consts::FRAC_PI_4).abs() < 1e-12);
        assert_eq!(dot_area(0.71), 1.0);
        let mut last = 0.0;
        for k in 1..=72 {
            let a = dot_area(k as f64 * 0.01);
            assert!(a >= last, "{k}");
            last = a;
        }
        // continuous at the cell edge
        assert!((dot_area(0.5 + 1e-9) - dot_area(0.5)).abs() < 1e-6);
    }

    #[test]
    fn read_defaults_and_ranges() {
        let p = ScreentoneParams::read(Params(&crate::params::params_from_pairs([
            ("screentone_size", "100"),
            ("screentone_angle", "-5"),
            ("screentone_type", "2"),
        ])));
        assert_eq!(p.size, 32.0);
        assert_eq!(p.angle, 0.0);
        assert_eq!(p.kind, ScreentoneKind::CrossHatch);
        assert!(p.from_luminance && !p.keep_color);
        assert_eq!(p.color, Rgba8::new(0, 0, 0, 255));
    }
}
