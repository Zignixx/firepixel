//! Verläufe: Parameter `t` je Form und Farbtabelle (Spezifikation 07 §5.2, §5.3,
//! Referenz `re\vektor\paint.py`, bitgleich).
//!
//! - **Abtastpunkt** ist die linke obere Pixelecke `(ix, iy)` (B).
//! - **Farbtabelle** mit 2001 Einträgen, Eintrag `i` ↔ `t = f32(i·f32(0,0005))`; Eintrag = Alpha aus
//!   dem Deckkraft-Lerp | RGB aus dem Farb-Lerp (`@007e69f0`).
//! - **Lerp zwischen Stopps** (`@007e4da0`): Segment = letztes `i ≤ n−2` mit `pos_i ≤ t`;
//!   Gewicht `w = trunc((t − p_i)/(p_{i+1} − p_i)·255) & 255` (**abgeschnitten**, x87-Rundungsart
//!   0xC00); gleiche Positionen → `w = 0`; je Kanal
//!   `(256·c0 + 127 + w·(c1 − c0) + ⌊w·c1/256⌋ − ⌊w·c0/256⌋) >> 8`.
//! - **Nachschlag** (`@007e4880`): `t ≤ 0` → `[0]`, `t ≥ 1` → `[2000]`, sonst Index aus
//!   `f32(floor(f32(t/0,0005)))`, Gewicht wieder abgeschnitten, derselbe 8-Bit-Lerp.
//! - **t**: linear und radial mit der float32-Rechenfolge aus 5.2; die übrigen Formen wie die
//!   Referenz in `f64` (das Original tastet 7/9/10/13 inkrementell in float ab: T6 dort ±1 an 1–4 Pixeln).
//! - Gespiegelte Wiederholung (`@007e6285`) für satin, pinch, folds, bars, ripple; danach auf
//!   `[0, 1]` begrenzt.

use crate::attrs::{Gradient, GradientHandles, GradientShape, GradientStop};
use fp_color::Argb;
use fp_geom::Vec2;

/// Rundung auf `f32` (wie `struct.pack('<f')` in der Referenz) als `f64`.
#[inline]
pub fn f32r(v: f64) -> f64 {
    v as f32 as f64
}

/// Schrittweite der Farbtabelle: `f32(0,0005)` als `double` (`@007e6a9c`).
pub const TABLE_STEP: f64 = 0.0005f32 as f64;

/// Zahl der Tabelleneinträge.
pub const TABLE_LEN: usize = 2001;

#[inline]
fn sq(v: f64) -> f64 {
    v * v
}

/// Python `int(v) & 255` (Abschneiden zur Null, Zweierkomplement-Maske); nicht-endliche Werte → 0.
#[inline]
pub fn trunc_and_255(v: f64) -> u32 {
    if !v.is_finite() {
        return 0;
    }
    (v.trunc().rem_euclid(256.0)) as u32
}

/// 8-Bit-Lerp aller vier Kanäle (`@007e4da0`, gepackt, exakt).
#[inline]
pub fn lerp8(c0: u32, c1: u32, w: u32) -> u32 {
    let mut out = 0u32;
    for sh in [24u32, 16, 8, 0] {
        let a = ((c0 >> sh) & 255) as i64;
        let b = ((c1 >> sh) & 255) as i64;
        let w = w as i64;
        let v = (256 * a + 127 + w * (b - a) + ((w * b) >> 8) - ((w * a) >> 8)) >> 8;
        out |= ((v & 255) as u32) << sh;
    }
    out
}

/// Farbe an `t` aus einer Stoppliste (Lerp `@007e4da0`). Leere Liste → `None`.
pub fn stop_lerp(st: &[GradientStop], t: f64) -> Option<u32> {
    let first = st.first()?;
    let last = st.last()?;
    if t < first.pos {
        return Some(first.color.0);
    }
    if t > last.pos || st.len() == 1 {
        return Some(last.color.0);
    }
    let mut seg = 0;
    for (i, s) in st.iter().enumerate().take(st.len() - 1) {
        if t < s.pos {
            break;
        }
        seg = i;
    }
    let (a, b) = (st[seg], st[seg + 1]);
    let w = if b.pos != a.pos {
        trunc_and_255((t - a.pos) / (b.pos - a.pos) * 255.0)
    } else {
        0
    };
    Some(lerp8(a.color.0, b.color.0, w))
}

/// Farbtabelle eines Verlaufs (2001 Einträge, ARGB) mit Nachschlag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GradientTable {
    pub entries: Vec<u32>,
}

impl GradientTable {
    /// Tabelle aus Farb- und Deckkraftstopps (`@007e69f0`).
    pub fn new(color_stops: &[GradientStop], opacity_stops: &[GradientStop]) -> Self {
        let mut entries = Vec::with_capacity(TABLE_LEN);
        for i in 0..TABLE_LEN {
            let t = f32r(i as f64 * TABLE_STEP);
            let c = stop_lerp(color_stops, t).unwrap_or(0x00FF_FFFF);
            let a = stop_lerp(opacity_stops, t).unwrap_or(c | 0xFF00_0000);
            entries.push((a & 0xFF00_0000) | (c & 0x00FF_FFFF));
        }
        Self { entries }
    }

    /// Farbe für `t` (`@007e4880`).
    pub fn lookup(&self, t: f64) -> Argb {
        let e = &self.entries;
        if t.is_nan() || t <= 0.0 {
            return Argb(e[0]);
        }
        if t >= 1.0 {
            return Argb(e[TABLE_LEN - 1]);
        }
        let t = f32r(t);
        let i = f32r(f32r(t / TABLE_STEP).floor());
        if i >= (TABLE_LEN - 1) as f64 {
            return Argb(e[TABLE_LEN - 1]);
        }
        let i = i.max(0.0) as usize;
        let lo = f32r(i as f64 * TABLE_STEP);
        let hi = f32r((i + 1) as f64 * TABLE_STEP);
        let w = trunc_and_255((t - lo) / (hi - lo) * 255.0);
        Argb(lerp8(e[i], e[i + 1], w))
    }
}

/// Gespiegelte Wiederholung (`@007e6285`): `t = |t|`; bei `t > 1` Nachkommateil, bei ungeradem
/// ganzzahligem Teil `1 − t`.
#[inline]
pub fn repeat_mirrored(t: f64) -> f64 {
    let mut t = t.abs();
    if t > 1.0 {
        let n = t.trunc();
        t -= n;
        if n.rem_euclid(2.0) == 1.0 {
            t = 1.0 - t;
        }
    }
    t
}

/// Berechnung des Verlaufsparameters `t(x, y)` für eine Form und ihre Griffe.
#[derive(Debug, Clone, Copy)]
pub enum TParam {
    Linear {
        p1x: f64,
        p1y: f64,
        dx: f64,
        dy: f64,
        l2: f64,
    },
    Radial {
        p1x: f64,
        p1y: f64,
        l: f64,
    },
    Conical {
        p1: Vec2,
        p2: Vec2,
        l2: f64,
        l: f64,
    },
    /// satin (0), waves (1), folds (2) über A/B.
    Ab {
        kind: u8,
        p1: Vec2,
        dx: f64,
        dy: f64,
    },
    /// pinch (0), elliptical/ripple (1), rectangular (2) über u/v.
    Uv {
        kind: u8,
        uv: UvMap,
    },
}

/// Affine Abbildung in den Verlaufsraum: `q − p1 = (u/100)(p2 − p1) + (v/100)(p3 − p1)`.
#[derive(Debug, Clone, Copy)]
pub struct UvMap {
    p1: Vec2,
    ax: f64,
    ay: f64,
    bx: f64,
    by: f64,
    det: f64,
}

impl UvMap {
    /// Aus den drei Griffen (Referenz `_uv`).
    pub fn new(h: &GradientHandles) -> Self {
        let (p1, p2, p3) = (h.p1, h.p2, h.p3);
        let ax = (p2.x - p1.x) / 100.0;
        let ay = (p2.y - p1.y) / 100.0;
        let bx = (p3.x - p1.x) / 100.0;
        let by = (p3.y - p1.y) / 100.0;
        Self {
            p1,
            ax,
            ay,
            bx,
            by,
            det: ax * by - ay * bx,
        }
    }

    /// `(u, v)` des Punkts; bei entarteten Griffen (|det| < 1e−12) `(0, 0)`.
    #[inline]
    pub fn map(&self, x: f64, y: f64) -> (f64, f64) {
        if self.det.abs() < 1e-12 {
            return (0.0, 0.0);
        }
        let qx = x - self.p1.x;
        let qy = y - self.p1.y;
        (
            (qx * self.by - qy * self.bx) / self.det,
            (self.ax * qy - self.ay * qx) / self.det,
        )
    }
}

impl TParam {
    /// Parameterfunktion für die Form (ohne Konturverlauf, der eigene Wege geht).
    pub fn new(shape: GradientShape, h: &GradientHandles) -> Self {
        let (p1, p2) = (h.p1, h.p2);
        match shape {
            GradientShape::Linear | GradientShape::Bars => {
                let (p1x, p1y, p2x, p2y) = (f32r(p1.x), f32r(p1.y), f32r(p2.x), f32r(p2.y));
                let dx = f32r(p2x - p1x);
                let dy = f32r(p2y - p1y);
                let mut l2 = f32r(f32r(dx * dx) + f32r(dy * dy));
                if l2 == 0.0 {
                    l2 = 1.0;
                }
                TParam::Linear {
                    p1x,
                    p1y,
                    dx,
                    dy,
                    l2,
                }
            }
            GradientShape::Radial => {
                let (p1x, p1y, p2x, p2y) = (f32r(p1.x), f32r(p1.y), f32r(p2.x), f32r(p2.y));
                let dx = f32r(p2x - p1x);
                let dy = f32r(p2y - p1y);
                let mut l = f32r(f32r(f32r(dy * dy) + f32r(dx * dx)).sqrt());
                if l == 0.0 {
                    l = 1.0;
                }
                TParam::Radial { p1x, p1y, l }
            }
            GradientShape::Conical => {
                let l2 = sq(p2.x - p1.x) + sq(p2.y - p1.y);
                TParam::Conical {
                    p1,
                    p2,
                    l2,
                    l: l2.sqrt(),
                }
            }
            GradientShape::Satin => TParam::Ab {
                kind: 0,
                p1,
                dx: p2.x - p1.x,
                dy: p2.y - p1.y,
            },
            GradientShape::Waves => TParam::Ab {
                kind: 1,
                p1,
                dx: p2.x - p1.x,
                dy: p2.y - p1.y,
            },
            GradientShape::Folds => TParam::Ab {
                kind: 2,
                p1,
                dx: p2.x - p1.x,
                dy: p2.y - p1.y,
            },
            GradientShape::Pinch => TParam::Uv {
                kind: 0,
                uv: UvMap::new(h),
            },
            GradientShape::Elliptical | GradientShape::Ripple | GradientShape::Contour => {
                TParam::Uv {
                    kind: 1,
                    uv: UvMap::new(h),
                }
            }
            GradientShape::Rectangular => TParam::Uv {
                kind: 2,
                uv: UvMap::new(h),
            },
        }
    }

    /// `t` an der Pixelecke `(x, y)` (vor Wiederholung/Begrenzung). Kegel: `acos(c)/3,14159` mit
    /// genau dieser Konstante (B `@007e5c00`, nicht π).
    #[allow(clippy::approx_constant)]
    pub fn eval(&self, x: f64, y: f64) -> f64 {
        match *self {
            TParam::Linear {
                p1x,
                p1y,
                dx,
                dy,
                l2,
            } => {
                let num = f32r(f32r(x - p1x) * dx + f32r(y - p1y) * dy);
                f32r(num / l2)
            }
            TParam::Radial { p1x, p1y, l } => {
                let a = f32r(x - p1x);
                let b = f32r(y - p1y);
                f32r(f32r(f32r(a * a + b * b).sqrt()) / l)
            }
            TParam::Conical { p1, p2, l2, l } => {
                let a2 = sq(x - p1.x) + sq(y - p1.y);
                let b2 = sq(x - p2.x) + sq(y - p2.y);
                let c = if a2 == 0.0 || l == 0.0 {
                    -1.0
                } else {
                    (a2 + l2 - b2) / (2.0 * a2.sqrt() * l)
                };
                c.clamp(-1.0, 1.0).acos() / 3.14159
            }
            TParam::Ab { kind, p1, dx, dy } => {
                let a = if dy != 0.0 {
                    (y - p1.y) / dy
                } else {
                    9_999_999.9
                };
                let b = if dx != 0.0 {
                    (x - p1.x) / dx
                } else {
                    9_999_999.9
                };
                match kind {
                    0 => 0.5 * (a.sin() * (b - a) - (3.0 * b).cos()).abs(),
                    1 => 0.5 * (a.sin() * sq(b - a) - (b - a) * (3.0 * b).cos()).abs(),
                    _ => (0.5
                        * (a.sin() * b + a)
                        * (24.0 * a * b).sin()
                        * (3.0 * (a + b).cos() * b * (b + 0.25f64.sin()).sin()).cos())
                    .abs(),
                }
            }
            TParam::Uv { kind, uv } => {
                let (u, v) = uv.map(x, y);
                match kind {
                    0 => u.abs() * v.abs() / 10000.0,
                    1 => (u * u / 10000.0 + v * v / 10000.0).sqrt(),
                    _ => u.abs().max(v.abs()) / 100.0,
                }
            }
        }
    }
}

/// Fertiger Verlaufsmaler: Form, Wiederholung und Farbtabelle.
#[derive(Debug, Clone)]
pub struct GradientPaint {
    t: TParam,
    mirrored: bool,
    table: GradientTable,
    dither: bool,
    /// Dither strength in per cent (FB-150); 100 = the measured dither.
    dither_strength: u32,
}

impl GradientPaint {
    /// `None` ohne Farbstopps (Referenz: „no-stops“) oder für den Konturverlauf.
    pub fn new(g: &Gradient, h: &GradientHandles) -> Option<Self> {
        if g.color_stops.is_empty() || g.shape == GradientShape::Contour {
            return None;
        }
        Some(Self {
            t: TParam::new(g.shape, h),
            mirrored: g.shape.mirrored(),
            table: GradientTable::new(&g.color_stops, &g.opacity_stops),
            dither: g.dither,
            dither_strength: u32::from(g.dither_strength).min(DITHER_STRENGTH_MAX),
        })
    }

    /// Farbe an der Pixelecke `(x, y)`.
    #[inline]
    pub fn color(&self, x: f64, y: f64) -> Argb {
        let mut t = self.t.eval(x, y);
        if self.mirrored {
            t = repeat_mirrored(t);
        }
        if !self.dither || self.dither_strength == 0 {
            return self.table.lookup(t);
        }
        if self.dither_strength == 100 {
            let c = self.table.lookup(dither_t(t)).0;
            return Argb(dither_noise(c, x as i64, y as i64));
        }
        // Firepixel extension (FB-150): weaker dither blends the S curve towards the plain
        // position; stronger dither keeps the full S curve and only raises the noise
        let k = (self.dither_strength.min(100)) as f64 / 100.0;
        let tt = if t.is_nan() {
            t
        } else {
            let tc = t.clamp(0.0, 1.0);
            tc + k * (dither_t(tc) - tc)
        };
        let c = self.table.lookup(tt).0;
        Argb(dither_noise_strength(
            c,
            x as i64,
            y as i64,
            self.dither_strength,
        ))
    }

    /// Die Farbtabelle.
    pub fn table(&self) -> &GradientTable {
        &self.table
    }
}

/// Position mapping of a dithered gradient (RE spec 75 §1.5, B): `t' = 0.5·t + 0.5·(3t² − 2t³)`
/// on the clamped position, before the colour table.
#[inline]
pub fn dither_t(t: f64) -> f64 {
    if t.is_nan() {
        return t;
    }
    let t = t.clamp(0.0, 1.0);
    0.5 * t + 0.5 * t * t * (3.0 - 2.0 * t)
}

/// Highest gradient dither strength in per cent (FB-150).
pub const DITHER_STRENGTH_MAX: u32 = 400;

/// Hash of a pixel and a colour channel for the gradient dither noise.
#[inline]
fn noise_hash(x: i64, y: i64, ch: usize) -> u64 {
    let mut h = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
        ^ (ch as u64 + 1).wrapping_mul(0x1656_67B1_9E37_79F9);
    h ^= h >> 33;
    h = h.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    h ^= h >> 33;
    h
}

/// ±1 noise per colour channel of a dithered gradient (RE spec 75 §1.5: about 14 % −1, 13 % +1,
/// independent per channel). The reference's noise source is unknown (A): a deterministic hash of
/// the pixel and the channel stands in, so renders stay reproducible; channels at 0 or 255 keep
/// their value (A, the measured end columns stay exact). Alpha is left alone (A, not measured).
#[inline]
pub fn dither_noise(argb: u32, x: i64, y: i64) -> u32 {
    let mut out = argb & 0xFF00_0000;
    for (ch, shift) in [16u32, 8, 0].into_iter().enumerate() {
        let v = (argb >> shift) & 255;
        let r = noise_hash(x, y, ch) % 100;
        let n = if v == 0 || v == 255 {
            v
        } else if r < 14 {
            v - 1
        } else if r < 27 {
            v + 1
        } else {
            v
        };
        out |= n << shift;
    }
    out
}

/// Gradient dither noise at a strength in per cent (Firepixel extension, FB-150). 100 is exactly
/// [`dither_noise`]. Below 100 the share of noisy pixels shrinks in proportion (the −1 and +1
/// thresholds of the same hash scale with the strength), so 0 adds no noise. Above 100 the share
/// stays the measured one (about 14 % −1, 13 % +1) and the step grows instead: on average
/// `strength / 100` levels (the fraction picks the next level by a second part of the hash), at
/// most ±4 at 400 %. Results are clamped to 0..255; channels at 0 or 255 keep their value like
/// in [`dither_noise`], alpha is left alone.
#[inline]
pub fn dither_noise_strength(argb: u32, x: i64, y: i64, strength: u32) -> u32 {
    let strength = strength.min(DITHER_STRENGTH_MAX);
    if strength == 100 {
        return dither_noise(argb, x, y);
    }
    if strength == 0 {
        return argb;
    }
    let share = strength.min(100);
    let mut out = argb & 0xFF00_0000;
    for (ch, shift) in [16u32, 8, 0].into_iter().enumerate() {
        let v = ((argb >> shift) & 255) as i32;
        let h = noise_hash(x, y, ch);
        // compare in 1/100 per cent: r·100 < 14·share ⇔ r < 14·share/100
        let r = (h % 100) as u32 * 100;
        let sign = if r < 14 * share {
            -1
        } else if r < 27 * share {
            1
        } else {
            0
        };
        let n = if v == 0 || v == 255 || sign == 0 {
            v
        } else {
            let step = if strength <= 100 {
                1
            } else {
                let whole = strength / 100;
                let frac = strength % 100;
                let pick = ((h >> 40) % 100) as u32;
                (whole + u32::from(pick < frac)) as i32
            };
            (v + sign * step).clamp(0, 255)
        };
        out |= (n as u32) << shift;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn st(p: f64, c: u32) -> GradientStop {
        GradientStop {
            pos: p,
            color: Argb(c),
        }
    }

    #[test]
    fn dither_s_curve_and_noise() {
        // RE spec 75 §1.5: t' = 0.5·t + 0.5·(3t² − 2t³)
        assert_eq!(dither_t(0.0), 0.0);
        assert_eq!(dither_t(1.0), 1.0);
        assert!((dither_t(0.5) - 0.5).abs() < 1e-12);
        assert!((dither_t(0.1) - 0.064).abs() < 1e-12);
        assert_eq!(dither_t(-2.0), 0.0);
        assert_eq!(dither_t(3.0), 1.0);
        let (mut lo, mut hi, mut n) = (0, 0, 0);
        for y in 0..100 {
            for x in 0..100 {
                let c = dither_noise(0xFF80_8080, x, y);
                assert_eq!(c >> 24, 0xFF, "alpha unchanged");
                for sh in [16, 8, 0] {
                    match ((c >> sh) & 255) as i32 - 128 {
                        -1 => lo += 1,
                        1 => hi += 1,
                        0 => {}
                        d => panic!("noise {d}"),
                    }
                    n += 1;
                }
            }
        }
        let (lo, hi) = (lo as f64 / n as f64, hi as f64 / n as f64);
        assert!(
            (0.11..0.17).contains(&lo) && (0.10..0.16).contains(&hi),
            "{lo} {hi}"
        );
        assert_eq!(
            dither_noise(0xFFFF_0000, 3, 4),
            0xFFFF_0000,
            "ends stay exact"
        );
        assert_eq!(
            dither_noise(0xFF12_3456, 7, 9),
            dither_noise(0xFF12_3456, 7, 9)
        );
    }

    fn ramp(dither: bool, strength: u16) -> GradientPaint {
        let g = Gradient {
            shape: GradientShape::Linear,
            color_stops: vec![st(0.0, 0xFF10_2030), st(1.0, 0xFFE0_C0A0)],
            opacity_stops: Vec::new(),
            dither,
            dither_strength: strength,
        };
        let h = GradientHandles {
            p1: Vec2::new(0.0, 0.0),
            p2: Vec2::new(256.0, 0.0),
            p3: Vec2::new(0.0, -256.0),
        };
        GradientPaint::new(&g, &h).expect("paint")
    }

    fn pixels(p: &GradientPaint) -> Vec<u32> {
        let mut v = Vec::new();
        for y in 0..48 {
            for x in -8..264 {
                v.push(p.color(x as f64 + 0.5, y as f64 + 0.5).0);
            }
        }
        v
    }

    /// Mean absolute channel difference between two renders.
    fn mean_diff(a: &[u32], b: &[u32]) -> f64 {
        let mut sum = 0u64;
        for (x, y) in a.iter().zip(b) {
            for sh in [16, 8, 0] {
                sum += (((x >> sh) & 255) as i64 - ((y >> sh) & 255) as i64).unsigned_abs();
            }
        }
        sum as f64 / (a.len() * 3) as f64
    }

    #[test]
    fn dither_strength_100_is_the_measured_dither_and_0_is_off() {
        // FB-150: 100 % renders exactly the old path (S curve + dither_noise)
        let p = ramp(true, 100);
        for y in 0..20 {
            for x in -4..260 {
                let (fx, fy) = (x as f64 + 0.5, y as f64 + 0.5);
                let t = p.t.eval(fx, fy);
                let want = dither_noise(p.table.lookup(dither_t(t)).0, fx as i64, fy as i64);
                assert_eq!(p.color(fx, fy).0, want, "({x}, {y})");
            }
        }
        // 0 % is identical to the dither switched off
        assert_eq!(pixels(&ramp(true, 0)), pixels(&ramp(false, 100)));
        // the strength has no effect while the dither is off
        assert_eq!(pixels(&ramp(false, 400)), pixels(&ramp(false, 100)));
        // clamped to 400
        assert_eq!(pixels(&ramp(true, 999)), pixels(&ramp(true, 400)));
        assert_eq!(
            dither_noise_strength(0xFF80_8080, 3, 5, 100),
            dither_noise(0xFF80_8080, 3, 5)
        );
        assert_eq!(dither_noise_strength(0xFF80_8080, 3, 5, 0), 0xFF80_8080);
    }

    #[test]
    fn dither_strength_amplitude_is_monotonic() {
        // noise alone on a flat colour: the mean deviation grows with the strength
        let flat = |s: u32| -> f64 {
            let mut sum = 0u64;
            let mut n = 0u64;
            for y in 0..64 {
                for x in 0..64 {
                    let c = dither_noise_strength(0xFF80_8080, x, y, s);
                    assert_eq!(c >> 24, 0xFF, "alpha unchanged");
                    for sh in [16, 8, 0] {
                        let d = ((c >> sh) & 255) as i64 - 128;
                        assert!(d.unsigned_abs() as u32 <= s.div_ceil(100).max(1));
                        sum += d.unsigned_abs();
                        n += 1;
                    }
                }
            }
            sum as f64 / n as f64
        };
        let steps = [0, 10, 25, 50, 75, 100, 150, 200, 300, 400];
        let amp: Vec<f64> = steps.iter().map(|&s| flat(s)).collect();
        assert_eq!(amp[0], 0.0);
        for w in amp.windows(2) {
            assert!(w[1] > w[0], "{amp:?}");
        }
        assert!(amp[9] > 3.0 * amp[5], "400 % clearly stronger: {amp:?}");
        // ends stay exact at every strength
        for s in steps {
            assert_eq!(dither_noise_strength(0xFFFF_0000, 7, 9, s), 0xFFFF_0000);
        }
        // the whole gradient: deviation from the undithered ramp grows with the strength
        let off = pixels(&ramp(false, 100));
        let dev: Vec<f64> = [0u16, 25, 50, 100, 200, 400]
            .iter()
            .map(|&s| mean_diff(&pixels(&ramp(true, s)), &off))
            .collect();
        for w in dev.windows(2) {
            assert!(w[1] > w[0], "{dev:?}");
        }
    }

    #[test]
    fn lerp_and_table() {
        assert_eq!(lerp8(0xFF000000, 0xFFFFFFFF, 0), 0xFF000000);
        assert_eq!(lerp8(0xFF000000, 0xFFFFFFFF, 255) & 0xFF, 255);
        let t = GradientTable::new(&[st(0.0, 0xFF000000), st(1.0, 0xFFFFFFFF)], &[]);
        assert_eq!(t.entries[0], 0xFF000000);
        assert_eq!(t.entries[2000], 0xFFFFFFFF);
        assert_eq!(t.lookup(1.0).0, 0xFFFFFFFF);
        assert_eq!(t.lookup(-1.0).0, 0xFF000000);
        let mid = t.lookup(0.5).0 & 0xFF;
        assert!((126..=128).contains(&mid), "{mid}");
        assert_eq!(trunc_and_255(-3.7), 253);
        assert_eq!(repeat_mirrored(1.25), 0.75);
        assert_eq!(repeat_mirrored(2.25), 0.25);
    }
}
