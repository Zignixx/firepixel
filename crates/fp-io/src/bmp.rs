//! BMP/DIB-Leser (12 §5.4, Zwischenablage `CF_DIB`/`CF_DIBV5`): Kopf 12/40/52/56/108/124 Byte,
//! 1/4/8/16/24/32 Bit, `BI_RGB`, `BI_RLE8`, `BI_RLE4`, `BI_BITFIELDS`/`BI_ALPHABITFIELDS`, Zeilen von unten
//! oder (negative Höhe) von oben. 32 Bit ohne Alphamaske: Alpha nur, wenn nicht alle Werte 0 sind.

use crate::raster::RasterImage;
use crate::{check_size, ImportError, Result};

fn u16le(d: &[u8], o: usize) -> Option<u16> {
    d.get(o..o + 2).map(|b| u16::from_le_bytes([b[0], b[1]]))
}
fn u32le(d: &[u8], o: usize) -> Option<u32> {
    d.get(o..o + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

fn bad(s: &str) -> ImportError {
    ImportError::Corrupt(format!("BMP: {s}"))
}

/// Datei mit `BITMAPFILEHEADER` lesen.
pub fn decode(data: &[u8]) -> Result<RasterImage> {
    if data.get(..2) != Some(b"BM") {
        return Err(bad("Signatur fehlt"));
    }
    let off = u32le(data, 10).ok_or_else(|| bad("Kopf abgeschnitten"))? as usize;
    decode_dib(data.get(14..).ok_or_else(|| bad("Kopf abgeschnitten"))?, Some(off.saturating_sub(14)), false)
}

fn mask_shift(m: u32) -> (u32, u32) {
    if m == 0 {
        return (0, 0);
    }
    let shift = m.trailing_zeros();
    let bits = (m >> shift).trailing_ones();
    (shift, bits)
}

fn scale(v: u32, bits: u32) -> u8 {
    match bits {
        0 => 0,
        8 => v as u8,
        b if b > 8 => (v >> (b - 8)) as u8,
        b => ((v * 255 + ((1 << b) - 1) / 2) / ((1 << b) - 1)) as u8,
    }
}

/// DIB ohne Dateikopf (`CF_DIB`); `pixel_offset` relativ zum DIB-Anfang (`None` = direkt hinter
/// Kopf und Palette). `alpha32`: bei 32 Bit `BI_RGB` das vierte Byte als Alpha deuten (Symbole,
/// Zwischenablage); sind dann alle Alphawerte 0, gilt das Bild als deckend.
pub fn decode_dib(d: &[u8], pixel_offset: Option<usize>, alpha32: bool) -> Result<RasterImage> {
    let hsize = u32le(d, 0).ok_or_else(|| bad("Kopf abgeschnitten"))? as usize;
    let (w, h, bpp, comp, ncolors) = if hsize == 12 {
        (u16le(d, 4).unwrap_or(0) as i32, u16le(d, 6).unwrap_or(0) as i16 as i32, u16le(d, 10).unwrap_or(0), 0u32, 0u32)
    } else if hsize >= 40 {
        (
            u32le(d, 4).unwrap_or(0) as i32,
            u32le(d, 8).unwrap_or(0) as i32,
            u16le(d, 14).unwrap_or(0),
            u32le(d, 16).unwrap_or(0),
            u32le(d, 32).unwrap_or(0),
        )
    } else {
        return Err(bad("unbekannte Kopfgröße"));
    };
    let top_down = h < 0;
    let (wu, hu) = (w.unsigned_abs(), h.unsigned_abs());
    check_size(wu, hu)?;
    let ppm = if hsize >= 40 { u32le(d, 24).unwrap_or(0) } else { 0 };
    // Farbmasken
    let mut masks = [0u32; 4];
    let mut after_header = hsize;
    if comp == 3 || comp == 6 {
        if hsize >= 52 {
            for (k, m) in masks.iter_mut().enumerate().take(if hsize >= 56 { 4 } else { 3 }) {
                *m = u32le(d, 40 + 4 * k).unwrap_or(0);
            }
        } else {
            for (k, m) in masks.iter_mut().enumerate().take(if comp == 6 { 4 } else { 3 }) {
                *m = u32le(d, hsize + 4 * k).unwrap_or(0);
            }
            after_header += if comp == 6 { 16 } else { 12 };
        }
    } else if bpp == 16 {
        masks = [0x7C00, 0x03E0, 0x001F, 0];
    } else if bpp == 32 {
        masks = [0x00FF_0000, 0x0000_FF00, 0x0000_00FF, 0];
    }
    if hsize >= 108 && comp == 0 && bpp == 32 {
        masks[3] = u32le(d, 52).unwrap_or(0);
    }
    if alpha32 && comp == 0 && bpp == 32 && masks[3] == 0 {
        masks[3] = 0xFF00_0000;
    }
    // Palette
    let entry = if hsize == 12 { 3 } else { 4 };
    let pal_n = if bpp <= 8 { if ncolors == 0 || ncolors > 256 { 1usize << bpp } else { ncolors as usize } } else { 0 };
    let mut palette = Vec::with_capacity(pal_n);
    for i in 0..pal_n {
        let o = after_header + i * entry;
        let Some(c) = d.get(o..o + 3) else { break };
        palette.push([c[2], c[1], c[0]]);
    }
    let pix_off = pixel_offset.unwrap_or(after_header + pal_n * entry);
    let pix = d.get(pix_off..).ok_or_else(|| bad("Pixeldaten fehlen"))?;
    let (w, h) = (wu as usize, hu as usize);
    let mut idx: Vec<u8> = Vec::new();
    let mut rgba = vec![0u8; w * h * 4];
    let row_of = |r: usize| if top_down { r } else { h - 1 - r };
    match (comp, bpp) {
        (1, 8) | (2, 4) => {
            // RLE (Zeilen von unten), unbeschriebene Pixel = Index 0
            idx = vec![0u8; w * h];
            let (mut x, mut y, mut p) = (0usize, 0usize, 0usize);
            let put = |idx: &mut Vec<u8>, x: usize, y: usize, v: u8| {
                if x < w && y < h {
                    idx[row_of(y) * w + x] = v;
                }
            };
            while p + 1 < pix.len() && y < h {
                let (n, v) = (pix[p] as usize, pix[p + 1]);
                p += 2;
                if n > 0 {
                    for k in 0..n {
                        let val = if bpp == 8 { v } else if k % 2 == 0 { v >> 4 } else { v & 15 };
                        put(&mut idx, x, y, val);
                        x += 1;
                    }
                } else {
                    match v {
                        0 => {
                            x = 0;
                            y += 1;
                        }
                        1 => break,
                        2 => {
                            let (dx, dy) = (pix.get(p).copied().unwrap_or(0) as usize, pix.get(p + 1).copied().unwrap_or(0) as usize);
                            p += 2;
                            x += dx;
                            y += dy;
                        }
                        cnt => {
                            let cnt = cnt as usize;
                            let bytes = if bpp == 8 { cnt } else { cnt.div_ceil(2) };
                            for k in 0..cnt {
                                let b = pix.get(p + if bpp == 8 { k } else { k / 2 }).copied().unwrap_or(0);
                                let val = if bpp == 8 { b } else if k % 2 == 0 { b >> 4 } else { b & 15 };
                                put(&mut idx, x, y, val);
                                x += 1;
                            }
                            p += bytes + (bytes & 1);
                        }
                    }
                }
            }
        }
        (0, 1 | 2 | 4 | 8) => {
            let stride = (w * bpp as usize).div_ceil(32) * 4;
            idx = vec![0u8; w * h];
            for r in 0..h {
                let row = pix.get(r * stride..r * stride + stride).ok_or_else(|| bad("Pixeldaten abgeschnitten"))?;
                let y = row_of(r);
                for x in 0..w {
                    let bit = x * bpp as usize;
                    let b = row[bit / 8];
                    idx[y * w + x] = (b >> (8 - bpp as usize - bit % 8)) & ((1u16 << bpp) - 1) as u8;
                }
            }
        }
        (0, 24) => {
            let stride = (w * 3).div_ceil(4) * 4;
            for r in 0..h {
                let row = pix.get(r * stride..r * stride + w * 3).ok_or_else(|| bad("Pixeldaten abgeschnitten"))?;
                let y = row_of(r);
                for x in 0..w {
                    let o = (y * w + x) * 4;
                    rgba[o..o + 4].copy_from_slice(&[row[x * 3 + 2], row[x * 3 + 1], row[x * 3], 255]);
                }
            }
        }
        (0 | 3 | 6, 16 | 32) => {
            let bytes = bpp as usize / 8;
            let stride = (w * bytes).div_ceil(4) * 4;
            let ms: Vec<(u32, u32)> = masks.iter().map(|&m| mask_shift(m)).collect();
            for r in 0..h {
                let row = pix.get(r * stride..r * stride + w * bytes).ok_or_else(|| bad("Pixeldaten abgeschnitten"))?;
                let y = row_of(r);
                for x in 0..w {
                    let v = if bytes == 2 { u16::from_le_bytes([row[x * 2], row[x * 2 + 1]]) as u32 } else { u32::from_le_bytes([row[x * 4], row[x * 4 + 1], row[x * 4 + 2], row[x * 4 + 3]]) };
                    let ch = |k: usize| scale((v & masks[k]) >> ms[k].0, ms[k].1);
                    let a = if masks[3] != 0 { ch(3) } else { 255 };
                    let o = (y * w + x) * 4;
                    rgba[o..o + 4].copy_from_slice(&[ch(0), ch(1), ch(2), a]);
                }
            }
            // 32 Bit mit Alphamaske, aber alle Alphawerte 0 ⇒ deckend (übliche Praxis)
            if masks[3] != 0 && rgba.as_chunks::<4>().0.iter().all(|p| p[3] == 0) {
                for p in rgba.as_chunks_mut::<4>().0 {
                    p[3] = 255;
                }
            }
        }
        _ => return Err(ImportError::Unsupported(format!("BMP-Kompression {comp} mit {bpp} Bit wird nicht unterstützt"))),
    }
    if !idx.is_empty() {
        for (i, &v) in idx.iter().enumerate() {
            let c = palette.get(v as usize).copied().unwrap_or([0, 0, 0]);
            rgba[i * 4..i * 4 + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
        }
    }
    let mut img = RasterImage::from_rgba(wu, hu, rgba)?;
    if ppm > 0 {
        img.dpi = Some((ppm as f64 * 0.0254).round() as f32);
    }
    if !palette.is_empty() {
        img.palette = Some(palette);
    }
    Ok(img)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_bitmaps() {
        // 2×2, 24 Bit, unten → oben
        let mut b = b"BM".to_vec();
        b.extend_from_slice(&(14 + 40 + 16u32).to_le_bytes());
        b.extend_from_slice(&[0, 0, 0, 0]);
        b.extend_from_slice(&54u32.to_le_bytes());
        b.extend_from_slice(&40u32.to_le_bytes());
        b.extend_from_slice(&2i32.to_le_bytes());
        b.extend_from_slice(&2i32.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&24u16.to_le_bytes());
        b.extend_from_slice(&[0u8; 24]);
        b.extend_from_slice(&[0, 0, 255, 0, 255, 0, 0, 0]); // untere Zeile: rot, grün
        b.extend_from_slice(&[255, 0, 0, 255, 255, 255, 0, 0]); // obere Zeile: blau, weiß
        let r = decode(&b).unwrap();
        assert_eq!(r.image.pixel(0, 1), fp_color::Rgba8::new(255, 0, 0, 255));
        assert_eq!(r.image.pixel(0, 0), fp_color::Rgba8::new(0, 0, 255, 255));
        assert_eq!(r.image.pixel(1, 0), fp_color::Rgba8::new(255, 255, 255, 255));
        assert!(decode(&b[..40]).is_err());
    }
}
