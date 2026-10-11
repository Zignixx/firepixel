//! Targa-Leser (12 §5.4, nur Lesen wie das Original): Typen 1/2/3 (unkomprimiert) und 9/10/11 (RLE),
//! 8/15/16/24/32 Bit, Farbtabelle, Ursprung oben/unten bzw. links/rechts, Alphakanal nach Deskriptor.

use crate::raster::RasterImage;
use crate::{check_size, ImportError, Result};

fn bad(s: &str) -> ImportError {
    ImportError::Corrupt(format!("TGA: {s}"))
}

fn px16(v: u16, alpha_bits: u8) -> [u8; 4] {
    let r = ((v >> 10) & 31) as u32;
    let g = ((v >> 5) & 31) as u32;
    let b = (v & 31) as u32;
    let e = |c: u32| ((c * 255 + 15) / 31) as u8;
    [e(r), e(g), e(b), if alpha_bits > 0 && v & 0x8000 == 0 { 0 } else { 255 }]
}

/// TGA lesen.
pub fn decode(d: &[u8]) -> Result<RasterImage> {
    let h = d.get(..18).ok_or_else(|| bad("Kopf abgeschnitten"))?;
    let (id_len, cmap_type, typ) = (h[0] as usize, h[1], h[2]);
    let (cmap_first, cmap_len, cmap_bits) = (u16::from_le_bytes([h[3], h[4]]) as usize, u16::from_le_bytes([h[5], h[6]]) as usize, h[7]);
    let (w, hh) = (u16::from_le_bytes([h[12], h[13]]) as u32, u16::from_le_bytes([h[14], h[15]]) as u32);
    let (bpp, desc) = (h[16], h[17]);
    check_size(w, hh)?;
    if !matches!(typ, 1 | 2 | 3 | 9 | 10 | 11) {
        return Err(ImportError::Unsupported(format!("TGA-Typ {typ} wird nicht unterstützt")));
    }
    let alpha_bits = desc & 15;
    let mut p = 18 + id_len;
    let mut cmap: Vec<[u8; 4]> = Vec::new();
    if cmap_type == 1 {
        let eb = (cmap_bits as usize).div_ceil(8);
        for i in 0..cmap_len {
            let e = d.get(p + i * eb..p + i * eb + eb).ok_or_else(|| bad("Farbtabelle abgeschnitten"))?;
            cmap.push(match eb {
                2 => px16(u16::from_le_bytes([e[0], e[1]]), 0),
                3 => [e[2], e[1], e[0], 255],
                4 => [e[2], e[1], e[0], e[3]],
                _ => [e[0], e[0], e[0], 255],
            });
        }
        p += cmap_len * eb;
    }
    let bytes = (bpp as usize).div_ceil(8);
    if bytes == 0 || bytes > 4 {
        return Err(bad("ungültige Bittiefe"));
    }
    let n = w as usize * hh as usize;
    let mut raw: Vec<u8> = Vec::with_capacity(n * bytes);
    if typ >= 9 {
        while raw.len() < n * bytes {
            let Some(&hdr) = d.get(p) else { break };
            p += 1;
            let count = (hdr & 0x7F) as usize + 1;
            if hdr & 0x80 != 0 {
                let v = d.get(p..p + bytes).ok_or_else(|| bad("RLE abgeschnitten"))?;
                for _ in 0..count {
                    raw.extend_from_slice(v);
                }
                p += bytes;
            } else {
                let v = d.get(p..p + bytes * count).ok_or_else(|| bad("RLE abgeschnitten"))?;
                raw.extend_from_slice(v);
                p += bytes * count;
            }
        }
        raw.truncate(n * bytes);
        raw.resize(n * bytes, 0);
    } else {
        raw.extend_from_slice(d.get(p..p + n * bytes).ok_or_else(|| bad("Pixeldaten abgeschnitten"))?);
    }
    let mut rgba = vec![0u8; n * 4];
    let top = desc & 0x20 != 0;
    let right = desc & 0x10 != 0;
    let (wu, hu) = (w as usize, hh as usize);
    let mut any_alpha = false;
    for i in 0..n {
        let s = &raw[i * bytes..i * bytes + bytes];
        let px = match (typ, bytes) {
            (1 | 9, _) => {
                let idx = if bytes == 2 { u16::from_le_bytes([s[0], s[1]]) as usize } else { s[0] as usize };
                cmap.get(idx.saturating_sub(cmap_first)).copied().unwrap_or([0, 0, 0, 255])
            }
            (3 | 11, 1) => [s[0], s[0], s[0], 255],
            (3 | 11, 2) => [s[0], s[0], s[0], s[1]],
            (_, 2) => px16(u16::from_le_bytes([s[0], s[1]]), alpha_bits),
            (_, 3) => [s[2], s[1], s[0], 255],
            (_, 4) => [s[2], s[1], s[0], if alpha_bits > 0 { s[3] } else { 255 }],
            _ => [s[0], s[0], s[0], 255],
        };
        any_alpha |= px[3] != 0;
        let (x, y) = (i % wu, i / wu);
        let (x, y) = (if right { wu - 1 - x } else { x }, if top { y } else { hu - 1 - y });
        rgba[(y * wu + x) * 4..(y * wu + x) * 4 + 4].copy_from_slice(&px);
    }
    // Alpha durchgehend 0 ⇒ als deckend behandeln (falsch gesetzter Deskriptor)
    if !any_alpha {
        for p in rgba.as_chunks_mut::<4>().0 {
            p[3] = 255;
        }
    }
    RasterImage::from_rgba(w, hh, rgba)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rle_truecolor() {
        let mut t = vec![0u8, 0, 10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 1, 0, 24, 0x20];
        t.extend_from_slice(&[0x81, 0, 0, 255]); // 2 × rot (BGR)
        let r = decode(&t).unwrap();
        assert_eq!(r.image.pixel(1, 0), fp_color::Rgba8::new(255, 0, 0, 255));
        assert!(decode(&t[..10]).is_err());
    }
}
