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
        })
    }

    /// Farbe an der Pixelecke `(x, y)`.
    #[inline]
    pub fn color(&self, x: f64, y: f64) -> Argb {
        let mut t = self.t.eval(x, y);
        if self.mirrored {
            t = repeat_mirrored(t);
        }
        self.table.lookup(t)
    }

    /// Die Farbtabelle.
    pub fn table(&self) -> &GradientTable {
        &self.table
    }
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
