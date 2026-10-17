//! Bild auf Bild mischen: Mischmodus, Deckkraft, Maske und Clip-Rechteck.
//!
//! Die Pixelformeln liefert `fp_color::blend` (Original-„over“ skalar bzw. MMX, 46 Mischmodi;
//! 07 §7, 05 §2.5). Hier liegt nur die Flächenlogik: Überlappung bestimmen, zeilenweise mischen,
//! bei großen Flächen parallel (rayon). Jede Zeile ist unabhängig, das Ergebnis also bitgleich
//! zur Einzel-Thread-Rechnung.

use fp_color::blend::{OverVariant, blend_pixel};
use fp_color::{BlendMode, Opacity, Rgba8};
use fp_geom::IRect;

use crate::image::RgbaImage;
use crate::mask::Mask;
use crate::par::for_each_row;

/// Einstellungen für [`composite`] und [`composite_color`]. **Stabil.**
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompositeOptions {
    /// Mischmodus (`BLDi`, 0 = Normal).
    pub mode: BlendMode,
    /// Deckkraft-Byte (255 = voll).
    pub opacity: Opacity,
    /// Rundungsvariante für den Modus Normal (Seitenkomposition: MMX; Pfadpuffer und
    /// Auswahlmasken: skalar).
    pub variant: OverVariant,
    /// Nur innerhalb dieses Rechtecks (Dokumentkoordinaten) schreiben.
    pub clip: Option<IRect>,
}

impl Default for CompositeOptions {
    fn default() -> Self {
        Self {
            mode: BlendMode(0),
            opacity: Opacity(255),
            variant: OverVariant::Mmx,
            clip: None,
        }
    }
}

impl CompositeOptions {
    /// Normal, volle Deckkraft, skalares „over“ (`@006f53c0`).
    pub fn scalar() -> Self {
        Self {
            variant: OverVariant::Scalar,
            ..Self::default()
        }
    }
}

/// Mischt eine konstante Farbe `color` (mit eigenem Alpha) durch die Abdeckungsmaske `coverage`
/// auf `dst` (Füllungen und Striche aus dem Pfad-Rasterizer): je Pixel Deckung
/// `mul(mul(m, op), a)` bzw. die kürzeren Formen (Schritt B), dann Mischmodus und „over“.
///
/// **Stabil** (API für fp-vector/fp-effects/fp-render).
pub fn composite_color(dst: &mut RgbaImage, color: Rgba8, coverage: &Mask, opt: &CompositeOptions) {
    let mut r = dst.bounds().intersect(&coverage.bounds());
    if let Some(c) = opt.clip {
        r = r.intersect(&c);
    }
    if r.is_empty() || opt.opacity.0 == 0 || color.a == 0 {
        return;
    }
    let db = dst.bounds();
    let dw = dst.width();
    let (x0, x1) = ((r.left - db.left) as usize, (r.right - db.left) as usize);
    let (y0, y1) = ((r.top - db.top) as usize, (r.bottom - db.top) as usize);
    let rows = &mut dst.data_mut()[y0 * dw..y1 * dw];
    let (mode, op, variant) = (opt.mode, opt.opacity.0, opt.variant);
    for_each_row(rows, dw, |i, row| {
        let y = r.top + i as i32;
        for (k, d) in row[x0..x1].iter_mut().enumerate() {
            let m = coverage.get(r.left + k as i32, y);
            if m == 0 {
                continue;
            }
            *d = blend_pixel(Rgba8::from_array(*d), color, mode, op, m, variant).to_array();
        }
    });
}

/// Mischt `src` an seiner Lage auf `dst`. Mit `mask` wird die Deckung je Pixel zusätzlich mit dem
/// Maskenwert (Dokumentkoordinaten, außerhalb 0) skaliert (Schritt B von `fp_color::blend`).
/// Quellpixel mit Alpha 0 lassen das Ziel unverändert.
///
/// **Stabil** (API für fp-vector/fp-effects/fp-render).
pub fn composite(
    dst: &mut RgbaImage,
    src: &RgbaImage,
    mask: Option<&Mask>,
    opt: &CompositeOptions,
) {
    let mut r = dst.bounds().intersect(&src.bounds());
    if let Some(c) = opt.clip {
        r = r.intersect(&c);
    }
    if let Some(m) = mask {
        r = r.intersect(&m.bounds());
    }
    if r.is_empty() || opt.opacity.0 == 0 {
        return;
    }
    let db = dst.bounds();
    let dw = dst.width();
    let (x0, x1) = ((r.left - db.left) as usize, (r.right - db.left) as usize);
    let (y0, y1) = ((r.top - db.top) as usize, (r.bottom - db.top) as usize);
    let rows = &mut dst.data_mut()[y0 * dw..y1 * dw];
    let (mode, op, variant) = (opt.mode, opt.opacity.0, opt.variant);
    for_each_row(rows, dw, |i, row| {
        let y = r.top + i as i32;
        let srow = match src.row(y) {
            Some(s) => s,
            None => return,
        };
        let sx0 = (r.left - src.bounds().left) as usize;
        for (k, d) in row[x0..x1].iter_mut().enumerate() {
            let s = srow[sx0 + k];
            if s[3] == 0 {
                continue;
            }
            let m = match mask {
                Some(m) => {
                    let v = m.get(r.left + k as i32, y);
                    if v == 0 {
                        continue;
                    }
                    v
                }
                None => 255,
            };
            *d = blend_pixel(
                Rgba8::from_array(*d),
                Rgba8::from_array(s),
                mode,
                op,
                m,
                variant,
            )
            .to_array();
        }
    });
}

/// Mischt `src` auf eine Kopie von `dst`, deren Grenzen bei Bedarf auf die Vereinigung beider
/// Bilder erweitert werden (transparent). Praktisch für Effektebenen.
pub fn composite_extend(
    dst: &RgbaImage,
    src: &RgbaImage,
    mask: Option<&Mask>,
    opt: &CompositeOptions,
) -> RgbaImage {
    let mut out = dst
        .extended_to(src.bounds())
        .unwrap_or_else(|_| dst.clone());
    composite(&mut out, src, mask, opt);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use fp_color::blend::over_scalar;

    #[test]
    fn normal_over_with_mask_and_clip() {
        let mut dst = RgbaImage::filled(IRect::new(0, 0, 10, 10), Rgba8::rgb(0, 0, 255)).unwrap();
        let src = RgbaImage::filled(IRect::new(5, 5, 15, 15), Rgba8::new(255, 0, 0, 128)).unwrap();
        let mut mask = Mask::filled(IRect::new(0, 0, 20, 20), 255).unwrap();
        mask.set(6, 6, 0);
        let opt = CompositeOptions {
            clip: Some(IRect::new(0, 0, 9, 9)),
            ..CompositeOptions::scalar()
        };
        composite(&mut dst, &src, Some(&mask), &opt);
        let expect = over_scalar(Rgba8::rgb(0, 0, 255), [255, 0, 0], 128);
        assert_eq!(dst.pixel(5, 5), expect);
        assert_eq!(dst.pixel(6, 6), Rgba8::rgb(0, 0, 255));
        assert_eq!(dst.pixel(9, 9), Rgba8::rgb(0, 0, 255));
        assert_eq!(dst.pixel(4, 4), Rgba8::rgb(0, 0, 255));
        let ext = composite_extend(&dst, &src, None, &CompositeOptions::default());
        assert_eq!(ext.bounds(), IRect::new(0, 0, 15, 15));
        assert_eq!(ext.pixel(14, 14), Rgba8::new(255, 0, 0, 128));
    }

    #[test]
    fn parallel_equals_sequential() {
        let b = IRect::new(0, 0, 300, 200);
        let mut dst = RgbaImage::new(b).unwrap();
        let mut src = RgbaImage::new(b).unwrap();
        for (i, p) in src.data_mut().iter_mut().enumerate() {
            *p = [(i * 7) as u8, (i * 13) as u8, (i * 3) as u8, (i * 11) as u8];
        }
        for (i, p) in dst.data_mut().iter_mut().enumerate() {
            *p = [(i * 5) as u8, (i * 2) as u8, (i * 17) as u8, (i * 19) as u8];
        }
        let opt = CompositeOptions {
            mode: BlendMode(3),
            opacity: Opacity(200),
            ..Default::default()
        };
        let mut a = dst.clone();
        composite(&mut a, &src, None, &opt);
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .unwrap();
        let mut s = dst.clone();
        pool.install(|| composite(&mut s, &src, None, &opt));
        assert_eq!(a, s);
    }
}
