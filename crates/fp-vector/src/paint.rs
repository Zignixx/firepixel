//! Füllfarben je Pixel und Fülltextur (Spezifikation 07 §5, Referenz `re\vektor\paint.py`).
//!
//! - Vollton: Farbe `FCLi`; Farb-Alpha < 255 wirkt wie Deckkraft (B Laufzeit T5).
//! - Verläufe: [`crate::gradient`].
//! - **Muster** (`FSHi = 15`): nearest neighbour, texel `(⌊u·W/100⌋ mod W, ⌊−v·H/100⌋ mod H)`
//!   with `(u, v)` relative to handle 1; ARGB as is. Measured (RE spec 61 §1.4): tile pixel (0,0)
//!   sits exactly on `p1`, tile x runs along `p1→p2`, tile rows run opposite to `p1→p3`, no snapping
//!   to the tile size and no seam left of or above `p1` (floor, not truncation). The pattern
//!   therefore travels with the fill handles, i.e. with the object (FB-24, FB-25).
//! - **Web-Dither** (`FSHi = 16`, `@007e4740`): `rx = round_half_away(ix − p1x) & 1`, `ry` analog;
//!   Farbe `FD[1 + 2·ry + rx]`.
//! - **Textur** (`TXFv`, `FTBi`): `w = ⌊FTBi·255/1000⌋`, Kachelung modulo Texturgröße, Ursprung
//!   `(ix − ⌊TOXf⌋, iy − ⌊TOYf⌋)`. `FSMi = 2`: Farbe Richtung Weiß, `f = round255((255 − T)·w)`,
//!   `c′ = c + round255((255 − c)·f)` (B, 33/33). `FSMi = 1`: Deckung moduliert,
//!   `s = T + (255 − w) − round255(T·(255 − w))`, `m′ = round255(s·m)` (B Code `@006f95c0`, ohne
//!   Pixelreferenz; die Python-Referenz nimmt hier eine andere Annahme).
//! - **Transparenz-Dither** (`FDTb`): Deckung × Schachbrett {FF,00 / 00,FF}; Phase hier an
//!   Dokumentkoordinaten (A: im Original an der 256er-Kachel).
//! - **Dither density** (Firepixel extension, FB-38): coverage × the ordered 8 × 8 on/off pattern
//!   of the density on document coordinates ([`crate::feather::density_on`]).

use crate::attrs::{Fill, FillKind, GradientHandles, StampingMode, VectorPath};
use crate::gradient::GradientPaint;
use fp_color::round::mul255;
use fp_geom::Vec2;
use fp_raster::{Mask, RgbaImage};

/// Warum eine Füllung nicht gemalt werden kann (wie die „missing“-Meldungen der Referenz).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PaintIssue {
    /// Verlauf ohne Farbstopps.
    NoStops,
    /// Muster ohne Bild.
    NoPattern,
}

/// `round(a·b/255)` als `u32`.
#[inline]
fn round255(v: u32) -> u32 {
    ((v + 128) + ((v + 128) >> 8)) >> 8
}

/// Rundung „halb weg von der Null“ als ganze Zahl.
#[inline]
fn round_half_away(v: f64) -> i64 {
    let r = if v >= 0.0 {
        (v + 0.5).floor()
    } else {
        -((-v + 0.5).floor())
    };
    if r.is_finite() {
        r.clamp(-9e15, 9e15) as i64
    } else {
        0
    }
}

/// `⌊v⌋ mod m` as a non-negative index; non-finite → 0. A tiny tolerance keeps values such as
/// 36.999999999 (rounding in the handle inversion) on the integer they stand for.
#[inline]
fn floor_mod(v: f64, m: usize) -> usize {
    if !v.is_finite() || m == 0 {
        return 0;
    }
    let i = (v + 1e-7).floor().rem_euclid(m as f64) as usize;
    i.min(m - 1)
}

enum Kind {
    Solid(u32),
    Gradient(Box<GradientPaint>),
    Pattern {
        img: std::sync::Arc<RgbaImage>,
        uv: crate::gradient::UvMap,
    },
    WebDither {
        colors: [u32; 4],
        p1: Vec2,
    },
}

/// Fülltextur-Nachschlag.
struct Tex<'a> {
    mask: &'a Mask,
    w: u32,
    ox: i64,
    oy: i64,
    opaque: bool,
}

impl Tex<'_> {
    #[inline]
    fn value(&self, x: i32, y: i32) -> u32 {
        let (tw, th) = (self.mask.width() as i64, self.mask.height() as i64);
        if tw == 0 || th == 0 {
            return 255;
        }
        let u = (x as i64 - self.ox).rem_euclid(tw);
        let v = (y as i64 - self.oy).rem_euclid(th);
        self.mask
            .data()
            .get((v * tw + u) as usize)
            .copied()
            .unwrap_or(255) as u32
    }
}

/// Liefert ARGB je Pixel und wendet Textur und Transparenz-Dither an.
pub struct FillPainter<'a> {
    kind: Kind,
    tex: Option<Tex<'a>>,
    dither: bool,
    /// Uniform dither density 1..=100 = share of empty pixels (Firepixel extension, FB-38,
    /// FB-109); 0 = off.
    density: u8,
}

impl<'a> FillPainter<'a> {
    /// Maler für die Füllung eines Pfads. Fehler wie die Referenz („no-stops“, Muster ohne Bild);
    /// der Konturverlauf hat einen eigenen Weg ([`crate::contour_gradient`]).
    pub fn new(path: &'a VectorPath, fill: &'a Fill) -> Result<Self, PaintIssue> {
        let h: &GradientHandles = &fill.handles;
        let kind = match &fill.kind {
            FillKind::Solid(c) => Kind::Solid(c.0),
            FillKind::Gradient(g) => Kind::Gradient(Box::new(
                GradientPaint::new(g, h).ok_or(PaintIssue::NoStops)?,
            )),
            FillKind::Pattern(img) => {
                if img.width() == 0 || img.height() == 0 {
                    return Err(PaintIssue::NoPattern);
                }
                Kind::Pattern {
                    img: img.clone(),
                    uv: crate::gradient::UvMap::new(h),
                }
            }
            FillKind::WebDither(c) => Kind::WebDither {
                colors: [c[0].0, c[1].0, c[2].0, c[3].0],
                p1: h.p1,
            },
        };
        let tex = fill
            .texture
            .as_ref()
            .filter(|t| t.amount_permille > 0)
            .map(|t| Tex {
                mask: &t.mask,
                w: t.amount_permille.min(1000) * 255 / 1000,
                ox: sat_floor(path.texture_offset.x),
                oy: sat_floor(path.texture_offset.y),
                opaque: fill.stamping == StampingMode::BlendOpaque,
            });
        Ok(Self {
            kind,
            tex,
            dither: fill.transparency_dither,
            density: if crate::feather::density_active(fill.dither_density) {
                fill.dither_density
            } else {
                0
            },
        })
    }

    /// Grundfarbe (ARGB) an der Pixelecke `(x, y)`.
    pub fn color(&self, x: i32, y: i32) -> u32 {
        match &self.kind {
            Kind::Solid(c) => *c,
            Kind::Gradient(g) => g.color(x as f64, y as f64).0,
            Kind::Pattern { img, uv } => {
                // (u, v) are relative to handle 1 already: tile origin = p1 (RE spec 61 §1.6)
                let (w, h) = (img.width(), img.height());
                let (u, v) = uv.map(x as f64, y as f64);
                let tx = floor_mod(u * w as f64 / 100.0, w);
                let ty = floor_mod(-v * h as f64 / 100.0, h);
                let p = img.data()[ty * w + tx];
                ((p[3] as u32) << 24) | ((p[0] as u32) << 16) | ((p[1] as u32) << 8) | p[2] as u32
            }
            Kind::WebDither { colors, p1 } => {
                let rx = round_half_away(x as f64 - p1.x).rem_euclid(2) as usize;
                let ry = round_half_away(y as f64 - p1.y).rem_euclid(2) as usize;
                colors[ry * 2 + rx]
            }
        }
    }

    /// Farbe und Deckung eines Pixels nach Textur und Transparenz-Dither:
    /// `(argb, deckung)` aus Grundfarbe und Maskenwert `m`.
    pub fn shade(&self, x: i32, y: i32, m: u8) -> (u32, u8) {
        let mut argb = self.color(x, y);
        let mut m = m as u32;
        if self.dither && (x as i64 + y as i64).rem_euclid(2) == 1 {
            m = 0;
        }
        // uniform dither density (Firepixel extension, FB-38): an on/off half tone of the fill
        if self.density != 0 && !crate::feather::density_on(self.density, x, y) {
            m = 0;
        }
        if let Some(t) = &self.tex {
            let tv = t.value(x, y);
            if t.opaque {
                let f = (t.w * (255 - tv) + 127) / 255;
                let ch = |c: u32| c + ((255 - c) * f + 127) / 255;
                let (r, g, b) = (
                    ch((argb >> 16) & 255),
                    ch((argb >> 8) & 255),
                    ch(argb & 255),
                );
                argb = (argb & 0xFF00_0000) | (r << 16) | (g << 8) | b;
            } else {
                let s = tv + (255 - t.w) - round255(tv * (255 - t.w));
                m = round255(s * m);
            }
        }
        (argb, m.min(255) as u8)
    }
}

#[inline]
fn sat_floor(v: f64) -> i64 {
    if v.is_finite() {
        v.floor().clamp(-9e15, 9e15) as i64
    } else {
        0
    }
}

/// Pixel einer Füllung aus Farbe und Deckung (Referenz `fill_layer`): `a = (α·c + 127)/255`.
#[inline]
pub fn fill_pixel(argb: u32, c: u8) -> [u8; 4] {
    let mut a = (argb >> 24) & 255;
    if c != 255 {
        a = (a * c as u32 + 127) / 255;
    }
    if a == 0 {
        return [0, 0, 0, 0];
    }
    [(argb >> 16) as u8, (argb >> 8) as u8, argb as u8, a as u8]
}

/// `mul255` als `u8`.
#[inline]
pub fn mul(a: u8, b: u8) -> u8 {
    mul255(a as u32, b as u32) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attrs::FillTexture;
    use fp_color::Argb;
    use fp_geom::IRect;
    use std::sync::Arc;

    /// 3 × 2 tile, every pixel distinct: red = column, green = row.
    fn tile() -> Arc<RgbaImage> {
        let px: Vec<[u8; 4]> = (0..2u8)
            .flat_map(|y| (0..3u8).map(move |x| [x * 10, y * 10, 7, 255]))
            .collect();
        Arc::new(RgbaImage::from_raw(IRect::new(0, 0, 3, 2), px).unwrap())
    }

    fn texel(x: u32, y: u32) -> u32 {
        0xFF00_0000 | (x * 10) << 16 | (y * 10) << 8 | 7
    }

    fn pattern_fill(p1: Vec2, p2: Vec2, p3: Vec2) -> Fill {
        let mut f = Fill::solid(Argb(0xFF00_0000));
        f.kind = FillKind::Pattern(tile());
        f.handles = GradientHandles { p1, p2, p3 };
        f
    }

    /// RE spec 61 §1.4: tile pixel (0,0) on handle 1, x along h1→h2, rows opposite to h1→h3, floor
    /// (no doubled column or row at h1).
    #[test]
    fn pattern_tile_origin_sits_on_handle_1() {
        let path = VectorPath::default();
        let p1 = Vec2::new(10.0, 20.0);
        let f = pattern_fill(p1, Vec2::new(13.0, 20.0), Vec2::new(10.0, 18.0));
        let p = FillPainter::new(&path, &f).unwrap();
        assert_eq!(p.color(10, 20), texel(0, 0));
        assert_eq!(p.color(11, 20), texel(1, 0));
        assert_eq!(p.color(12, 21), texel(2, 1));
        assert_eq!(p.color(13, 22), texel(0, 0));
        // left of and above h1: the previous tile, no seam
        assert_eq!(p.color(9, 20), texel(2, 0));
        assert_eq!(p.color(8, 20), texel(1, 0));
        assert_eq!(p.color(10, 19), texel(0, 1));
        assert_eq!(p.color(7, 17), texel(0, 1));
    }

    /// FB-24/FB-25: moving the handles by (37, 23) moves the pattern by exactly (37, 23), for
    /// tile-sized and for scaled handles.
    #[test]
    fn pattern_moves_with_the_handles() {
        let path = VectorPath::default();
        let d = Vec2::new(37.0, 23.0);
        for (p2, p3) in [
            (Vec2::new(13.0, 20.0), Vec2::new(10.0, 18.0)),
            (Vec2::new(16.0, 20.0), Vec2::new(10.0, 16.0)),
            (Vec2::new(11.5, 20.0), Vec2::new(10.0, 15.0)),
        ] {
            let p1 = Vec2::new(10.0, 20.0);
            let a = pattern_fill(p1, p2, p3);
            let b = pattern_fill(p1 + d, p2 + d, p3 + d);
            let (pa, pb) = (
                FillPainter::new(&path, &a).unwrap(),
                FillPainter::new(&path, &b).unwrap(),
            );
            assert_eq!(pa.color(10, 20), texel(0, 0));
            for y in 0..40 {
                for x in 0..40 {
                    assert_eq!(pa.color(x, y), pb.color(x + 37, y + 23), "({x},{y}) {p2:?}");
                }
            }
        }
    }

    #[test]
    fn web_dither_and_texture() {
        let path = VectorPath::default();
        let mut f = Fill::solid(Argb(0xFF102030));
        f.kind = FillKind::WebDither([Argb(1), Argb(2), Argb(3), Argb(4)]);
        let p = FillPainter::new(&path, &f).unwrap();
        assert_eq!(p.color(0, 0), 1);
        assert_eq!(p.color(1, 0), 2);
        assert_eq!(p.color(0, 1), 3);
        assert_eq!(p.color(-1, -1), 4);
        let mut f = Fill::solid(Argb(0xFF000000));
        let mask = Mask::from_raw(IRect::new(0, 0, 2, 1), vec![0, 255]).unwrap();
        f.texture = Some(FillTexture {
            mask: Arc::new(mask),
            amount_permille: 1000,
        });
        let p = FillPainter::new(&path, &f).unwrap();
        assert_eq!(p.shade(0, 0, 255).0, 0xFFFFFFFF);
        assert_eq!(p.shade(1, 0, 255).0, 0xFF000000);
        assert_eq!(fill_pixel(0x80FF0000, 255), [255, 0, 0, 128]);
        assert_eq!(fill_pixel(0xFFFF0000, 128), [255, 0, 0, 128]);
    }
}
