//! TIFF-Leser (12 §5.4): erste Seite über das `tiff`-Crate (unkomprimiert, LZW, Deflate, PackBits,
//! JPEG, CCITT), Graustufen/RGB/RGBA/Palette, 8/16 Bit (oberes Byte). **CMYK wird wie das Original über
//! die eigene CMYK-Tabelle umgerechnet** (Laufzeit 7a T7: 4.096/4.096 Pixel = `cmyk_to_rgb_exakt`);
//! ohne Tabelle `255 − C` (Rückfall von `fp_color::cmyk`). Die Ebene heißt „Hintergrund“.

use std::io::Cursor;

use fp_color::cmyk::{cmyk_to_rgb, CmykTable};
use tiff::decoder::{Decoder, DecodingResult};
use tiff::tags::Tag;
use tiff::ColorType;

use crate::raster::RasterImage;
use crate::{check_size, ImportError, Result};

fn err(e: impl std::fmt::Display) -> ImportError {
    ImportError::Decode("TIFF", e.to_string())
}

/// Palettenbilder (`PhotometricInterpretation` 3) lehnt das `tiff`-Crate ab: In einer Kopie wird die
/// Interpretation der ersten Seite auf 1 (Graustufen) gesetzt, die Indizes werden dann selbst über die
/// `ColorMap` abgebildet. `None`, wenn die erste Seite kein Palettenbild ist.
fn palette_as_gray(data: &[u8]) -> Option<Vec<u8>> {
    let le = match data.get(..2)? {
        b"II" => true,
        b"MM" => false,
        _ => return None,
    };
    let u16_at = |o: usize| -> Option<u16> {
        let b: [u8; 2] = data.get(o..o + 2)?.try_into().ok()?;
        Some(if le { u16::from_le_bytes(b) } else { u16::from_be_bytes(b) })
    };
    let u32_at = |o: usize| -> Option<u32> {
        let b: [u8; 4] = data.get(o..o + 4)?.try_into().ok()?;
        Some(if le { u32::from_le_bytes(b) } else { u32::from_be_bytes(b) })
    };
    let u64_at = |o: usize| -> Option<u64> {
        let b: [u8; 8] = data.get(o..o + 8)?.try_into().ok()?;
        Some(if le { u64::from_le_bytes(b) } else { u64::from_be_bytes(b) })
    };
    // klassisch (42): Zähler u16, Eintrag 12 Byte; BigTIFF (43): Zähler u64, Eintrag 20 Byte
    let (count, first, entry, value_off) = match u16_at(2)? {
        42 => {
            let ifd = u32_at(4)? as usize;
            (u16_at(ifd)? as usize, ifd + 2, 12, 8)
        }
        43 => {
            let ifd = usize::try_from(u64_at(8)?).ok()?;
            (usize::try_from(u64_at(ifd)?).ok()?, ifd + 8, 20, 12)
        }
        _ => return None,
    };
    for k in 0..count.min(4096) {
        let e = first + k * entry;
        if u16_at(e)? == 262 && u16_at(e + 2)? == 3 && u16_at(e + value_off)? == 3 {
            let mut out = data.to_vec();
            let one = if le { 1u16.to_le_bytes() } else { 1u16.to_be_bytes() };
            out.get_mut(e + value_off..e + value_off + 2)?.copy_from_slice(&one);
            return Some(out);
        }
    }
    None
}

/// TIFF lesen (erste Seite).
pub fn decode(data: &[u8], cmyk: Option<&CmykTable>) -> Result<RasterImage> {
    let patched = palette_as_gray(data);
    let is_palette = patched.is_some();
    let mut d = Decoder::new(Cursor::new(patched.as_deref().unwrap_or(data))).map_err(err)?;
    let (w, h) = d.dimensions().map_err(err)?;
    check_size(w, h)?;
    let ct = match d.colortype().map_err(err)? {
        ColorType::Gray(bits) if is_palette => ColorType::Palette(bits),
        other => other,
    };
    let photometric = d.find_tag_unsigned::<u16>(Tag::PhotometricInterpretation).ok().flatten().unwrap_or(1);
    let colormap = d.get_tag_u16_vec(Tag::ColorMap).ok();
    let dpi = match (d.find_tag(Tag::XResolution).ok().flatten(), d.find_tag_unsigned::<u16>(Tag::ResolutionUnit).ok().flatten().unwrap_or(2)) {
        (Some(tiff::decoder::ifd::Value::Rational(n, dd)), unit) if dd != 0 => Some(n as f32 / dd as f32 * if unit == 3 { 2.54 } else { 1.0 }),
        _ => None,
    };
    let img = d.read_image().map_err(err)?;
    // Auf 8 Bit (oberes Byte bei 16 Bit wie das Plug-in)
    let samples: Vec<u8> = match img {
        DecodingResult::U8(v) => v,
        DecodingResult::U16(v) => v.iter().map(|x| (x >> 8) as u8).collect(),
        DecodingResult::U32(v) => v.iter().map(|x| (x >> 24) as u8).collect(),
        DecodingResult::F32(v) => v.iter().map(|x| (x.clamp(0.0, 1.0) * 255.0).round() as u8).collect(),
        _ => return Err(ImportError::Unsupported("TIFF-Sampleformat wird nicht unterstützt".into())),
    };
    let n = w as usize * h as usize;
    let mut rgba = Vec::with_capacity(n * 4);
    let need = |k: usize| -> Result<()> {
        if samples.len() < n * k {
            Err(ImportError::Corrupt("TIFF-Daten zu kurz".into()))
        } else {
            Ok(())
        }
    };
    match ct {
        ColorType::Gray(_) => {
            need(1)?;
            for &v in &samples[..n] {
                let v = if photometric == 0 { 255 - v } else { v };
                rgba.extend_from_slice(&[v, v, v, 255]);
            }
        }
        ColorType::GrayA(_) => {
            need(2)?;
            for p in samples.as_chunks::<2>().0.iter().take(n) {
                let v = if photometric == 0 { 255 - p[0] } else { p[0] };
                rgba.extend_from_slice(&[v, v, v, p[1]]);
            }
        }
        ColorType::RGB(_) => {
            need(3)?;
            for p in samples.as_chunks::<3>().0.iter().take(n) {
                rgba.extend_from_slice(&[p[0], p[1], p[2], 255]);
            }
        }
        ColorType::RGBA(_) => {
            need(4)?;
            for p in samples.as_chunks::<4>().0.iter().take(n) {
                rgba.extend_from_slice(p);
            }
        }
        ColorType::CMYK(_) | ColorType::CMYKA(_) => {
            let k = if matches!(ct, ColorType::CMYK(_)) { 4 } else { 5 };
            need(k)?;
            for p in samples.chunks_exact(k).take(n) {
                let (r, g, b) = cmyk_to_rgb(p[0], p[1], p[2], p[3], cmyk);
                rgba.extend_from_slice(&[r, g, b, if k == 5 { p[4] } else { 255 }]);
            }
        }
        ColorType::Palette(bits) => {
            let map = colormap.ok_or_else(|| ImportError::Corrupt("TIFF-Palette ohne ColorMap".into()))?;
            let nc = map.len() / 3;
            let idx: Vec<usize> = if bits == 8 {
                samples.iter().take(n).map(|&v| v as usize).collect()
            } else {
                // gepackte Indizes je Zeile
                let stride = (w as usize * bits as usize).div_ceil(8);
                let mut v = Vec::with_capacity(n);
                for y in 0..h as usize {
                    for x in 0..w as usize {
                        let bit = x * bits as usize;
                        let byte = samples.get(y * stride + bit / 8).copied().unwrap_or(0);
                        v.push(((byte >> (8 - bits as usize - bit % 8)) & ((1u16 << bits) - 1) as u8) as usize);
                    }
                }
                v
            };
            for i in idx {
                let c = |k: usize| map.get(k * nc + i).map(|&x| (x >> 8) as u8).unwrap_or(0);
                rgba.extend_from_slice(&[c(0), c(1), c(2), 255]);
            }
        }
        other => return Err(ImportError::Unsupported(format!("TIFF-Farbtyp {other:?} wird nicht unterstützt"))),
    }
    let mut img = RasterImage::from_rgba(w, h, rgba)?;
    img.dpi = dpi;
    img.layer_name = Some("Hintergrund".into());
    Ok(img)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmyk_without_table_is_inverse() {
        // Minimal-TIFF (II), 1×1 CMYK 8 Bit, unkomprimiert
        let mut t = b"II*\0".to_vec();
        t.extend_from_slice(&8u32.to_le_bytes());
        let entries: Vec<(u16, u16, u32, u32)> = vec![(256, 3, 1, 1), (257, 3, 1, 1), (258, 3, 4, 0), (259, 3, 1, 1), (262, 3, 1, 5), (273, 4, 1, 0), (277, 3, 1, 4), (278, 3, 1, 1), (279, 4, 1, 4)];
        let ifd_len = 2 + entries.len() * 12 + 4;
        let bps_off = 8 + ifd_len as u32;
        let data_off = bps_off + 8;
        t.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        for (tag, typ, cnt, val) in entries {
            let v = match tag {
                258 => bps_off,
                273 => data_off,
                _ => val,
            };
            t.extend_from_slice(&tag.to_le_bytes());
            t.extend_from_slice(&typ.to_le_bytes());
            t.extend_from_slice(&cnt.to_le_bytes());
            if typ == 3 && cnt == 1 {
                t.extend_from_slice(&(v as u16).to_le_bytes());
                t.extend_from_slice(&[0, 0]);
            } else {
                t.extend_from_slice(&v.to_le_bytes());
            }
        }
        t.extend_from_slice(&0u32.to_le_bytes());
        for _ in 0..4 {
            t.extend_from_slice(&8u16.to_le_bytes());
        }
        t.extend_from_slice(&[255, 0, 10, 200]);
        let r = decode(&t, None).unwrap();
        assert_eq!(r.image.pixel(0, 0), fp_color::Rgba8::new(0, 255, 245, 255));
        assert_eq!(r.layer_name.as_deref(), Some("Hintergrund"));
    }
}
