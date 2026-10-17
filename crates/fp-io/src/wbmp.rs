//! WBMP-Leser (Typ 0, 1 Bit; 12 §5.4): Breite/Höhe als Multibyte-Ganzzahlen, Bit 1 = weiß.

use crate::raster::RasterImage;
use crate::{ImportError, Result, check_size};

fn mbi(d: &[u8], p: &mut usize) -> Option<u32> {
    let mut v: u32 = 0;
    for _ in 0..5 {
        let b = *d.get(*p)?;
        *p += 1;
        v = v.checked_mul(128)? | (b & 0x7F) as u32;
        if b & 0x80 == 0 {
            return Some(v);
        }
    }
    None
}

/// WBMP lesen.
pub fn decode(d: &[u8]) -> Result<RasterImage> {
    let bad = |s: &str| ImportError::Corrupt(format!("WBMP: {}", crate::part(s)));
    let mut p = 0usize;
    let typ = mbi(d, &mut p).ok_or_else(|| bad("err-import-part-header"))?;
    if typ != 0 {
        return Err(ImportError::Unsupported(fp_i18n::tr!(
            "err-import-wbmp-type",
            kind = typ.to_string()
        )));
    }
    let fix = *d.get(p).ok_or_else(|| bad("err-import-part-header"))?;
    p += 1;
    if fix & 0x80 != 0 {
        // Erweiterungsköpfe überspringen
        while d.get(p).is_some_and(|b| b & 0x80 != 0) {
            p += 1;
        }
        p += 1;
    }
    let w = mbi(d, &mut p).ok_or_else(|| bad("err-import-part-width"))?;
    let h = mbi(d, &mut p).ok_or_else(|| bad("err-import-part-height"))?;
    check_size(w, h)?;
    let stride = (w as usize).div_ceil(8);
    let mut rgba = Vec::with_capacity(w as usize * h as usize * 4);
    for y in 0..h as usize {
        let row = d
            .get(p + y * stride..p + (y + 1) * stride)
            .ok_or_else(|| bad("err-import-part-pixels-truncated"))?;
        for x in 0..w as usize {
            let v = if row[x / 8] & (0x80 >> (x % 8)) != 0 {
                255
            } else {
                0
            };
            rgba.extend_from_slice(&[v, v, v, 255]);
        }
    }
    let mut img = RasterImage::from_rgba(w, h, rgba)?;
    img.palette = Some(vec![[0, 0, 0], [255, 255, 255]]);
    Ok(img)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small() {
        let r = decode(&[0, 0, 9, 1, 0b1010_1010, 0b1000_0000]).unwrap();
        assert_eq!(
            r.image.pixel(0, 0),
            fp_color::Rgba8::new(255, 255, 255, 255)
        );
        assert_eq!(r.image.pixel(1, 0), fp_color::Rgba8::new(0, 0, 0, 255));
        assert_eq!(
            r.image.pixel(8, 0),
            fp_color::Rgba8::new(255, 255, 255, 255)
        );
        assert!(decode(&[0, 0, 9, 1, 0]).is_err());
    }
}
