//! JPEG-Leser (12 §5.2): eigener Dekoder mit dem IJG-Rechenweg ([`crate::jpeg_ijg`]), Rückfall zune-jpeg;
//! keine Farbverwaltung. CMYK/YCCK wird wie im Original ohne Farbtabelle multiplikativ umgerechnet:
//! `R = ⌊(255 − C)·(255 − K)/255⌋` usw. (Laufzeit RF 11.10.2026, T7 4.096/4.096; mit APP14-Segment
//! invertiert gespeichert).
//! Auflösung aus dem JFIF-APP0 (Einheit 1 = dpi, 2 = dpcm, eigene Auswertung) bzw. EXIF `XResolution`.

use zune_jpeg::zune_core::bytestream::ZCursor;
use zune_jpeg::zune_core::colorspace::ColorSpace;
use zune_jpeg::zune_core::options::DecoderOptions;
use zune_jpeg::JpegDecoder;

use crate::raster::RasterImage;
use crate::{check_size, ImportError, Result};

/// Ruft `f(marker, segment)` für jedes Segment vor dem ersten SOS auf, bis `f` `true` liefert.
fn scan_segments(data: &[u8], mut f: impl FnMut(u8, &[u8]) -> bool) {
    let mut i = 2usize;
    while i + 4 <= data.len() {
        if data[i] != 0xFF {
            i += 1;
            continue;
        }
        let m = data[i + 1];
        if m == 0xD8 || (0xD0..=0xD7).contains(&m) || m == 0x01 || m == 0xFF {
            i += if m == 0xFF { 1 } else { 2 };
            continue;
        }
        if m == 0xDA {
            break;
        }
        let len = u16::from_be_bytes([data[i + 2], data[i + 3]]) as usize;
        let seg = data.get(i + 4..(i + 2 + len).min(data.len())).unwrap_or(&[]);
        if f(m, seg) {
            break;
        }
        i += 2 + len.max(2);
    }
}

/// Auflösung aus dem JFIF-APP0 (Einheit 1 = dpi, 2 = dpcm; 0 = nur Seitenverhältnis ⇒ keine).
/// zune-jpeg wertet die Dichte nicht aus.
fn jfif_dpi(data: &[u8]) -> Option<f32> {
    let mut dpi = None;
    scan_segments(data, |m, seg| {
        if m == 0xE0 && seg.len() >= 10 && seg.starts_with(b"JFIF\0") {
            let x = u16::from_be_bytes([seg[8], seg[9]]);
            dpi = match seg[7] {
                1 if x > 0 => Some(f32::from(x)),
                2 if x > 0 => Some(f32::from(x) * 2.54),
                _ => None,
            };
            return true;
        }
        false
    });
    dpi
}

/// Hat die Datei ein APP14-Segment mit Farbtransformations-Angabe (CMYK-Werte invertiert)? Erkannt am
/// Aufbau (Marker `0xEE`, mindestens 12 Byte: 5 Byte Kennung, Version, zwei Flag-Wörter, Transform-Byte).
fn has_app14_transform(data: &[u8]) -> bool {
    let mut found = false;
    scan_segments(data, |m, seg| {
        found = m == 0xEE && seg.len() >= 12;
        found
    });
    found
}

/// EXIF-`XResolution` (Tag 0x011A) und `ResolutionUnit` (0x0128) aus einem APP1-Block (TIFF-Kopf).
pub fn exif_dpi(exif: &[u8]) -> Option<f32> {
    let t = if exif.starts_with(b"Exif\0\0") { &exif[6..] } else { exif };
    let le = match t.get(..2)? {
        b"II" => true,
        b"MM" => false,
        _ => return None,
    };
    let u16at = |o: usize| t.get(o..o + 2).map(|b| if le { u16::from_le_bytes([b[0], b[1]]) } else { u16::from_be_bytes([b[0], b[1]]) });
    let u32at = |o: usize| t.get(o..o + 4).map(|b| if le { u32::from_le_bytes([b[0], b[1], b[2], b[3]]) } else { u32::from_be_bytes([b[0], b[1], b[2], b[3]]) });
    let ifd = u32at(4)? as usize;
    let n = u16at(ifd)? as usize;
    let (mut xres, mut unit) = (None, 2u16);
    for k in 0..n.min(512) {
        let e = ifd + 2 + k * 12;
        let tag = u16at(e)?;
        match tag {
            0x011A => {
                let off = u32at(e + 8)? as usize;
                let (num, den) = (u32at(off)?, u32at(off + 4)?);
                if den != 0 {
                    xres = Some(num as f32 / den as f32);
                }
            }
            0x0128 => unit = u16at(e + 8)?,
            _ => {}
        }
    }
    let x = xres?;
    Some(if unit == 3 { x * 2.54 } else { x })
}

/// CMYK-Anteile nach RGB wie das Original beim JPEG-Import: multiplikativ
/// `R = ⌊(255 − C)·(255 − K)/255⌋` (Laufzeit RF 11.10.2026: T7 4.096/4.096; die frühere Annahme
/// `255 − min(255, C + K)` trifft nur reine Farbfelder).
fn cmyk_naive(c: u8, m: u8, y: u8, k: u8) -> [u8; 4] {
    let ch = |v: u8| ((255 - v as u32) * (255 - k as u32) / 255) as u8;
    [ch(c), ch(m), ch(y), 255]
}

#[allow(clippy::needless_range_loop)]
/// JPEG lesen: zuerst der IJG-Rechenweg ([`crate::jpeg_ijg`]), bei nicht unterstützten Varianten zune-jpeg.
pub fn decode(data: &[u8]) -> Result<RasterImage> {
    if let Ok(p) = crate::jpeg_ijg::decode_planes(data) {
        let (w, h) = (p.width, p.height);
        check_size(w, h)?;
        let n = w as usize * h as usize;
        let t = crate::jpeg_ijg::YccTables::default();
        let mut rgba = Vec::with_capacity(n * 4);
        let pl = &p.planes;
        use crate::jpeg_ijg::JpegColor as C;
        for i in 0..n {
            let px = match p.color {
                C::Gray => [pl[0][i], pl[0][i], pl[0][i], 255],
                C::Rgb => [pl[0][i], pl[1][i], pl[2][i], 255],
                C::YCbCr => {
                    let [r, g, b] = t.rgb(pl[0][i], pl[1][i], pl[2][i]);
                    [r, g, b, 255]
                }
                C::Ycck => {
                    let [r, g, b] = t.rgb(pl[0][i], pl[1][i], pl[2][i]);
                    cmyk_naive(255 - r, 255 - g, 255 - b, 255 - pl[3][i])
                }
                C::Cmyk => {
                    if p.app14 {
                        cmyk_naive(255 - pl[0][i], 255 - pl[1][i], 255 - pl[2][i], 255 - pl[3][i])
                    } else {
                        cmyk_naive(pl[0][i], pl[1][i], pl[2][i], pl[3][i])
                    }
                }
            };
            rgba.extend_from_slice(&px);
        }
        let mut img = RasterImage::from_rgba(w, h, rgba)?;
        let mut exif = None;
        scan_segments(data, |m, seg| {
            if m == 0xE1 && seg.starts_with(b"Exif\0\0") {
                exif = Some(seg.to_vec());
                return true;
            }
            false
        });
        img.dpi = jfif_dpi(data).or_else(|| exif.as_deref().and_then(exif_dpi));
        return Ok(img);
    }
    decode_zune(data)
}

/// JPEG über zune-jpeg (Rückfall für arithmetische/12-Bit/verlustfreie Dateien).
pub fn decode_zune(data: &[u8]) -> Result<RasterImage> {
    let mut probe = JpegDecoder::new(ZCursor::new(data));
    probe.decode_headers().map_err(|e| ImportError::Decode("JPEG", format!("{e:?}")))?;
    let input = probe.input_colorspace().unwrap_or(ColorSpace::YCbCr);
    let info = probe.info().ok_or_else(|| ImportError::Corrupt("JPEG ohne Kopf".into()))?;
    let (w, h) = (info.width as u32, info.height as u32);
    check_size(w, h)?;
    let four = matches!(input, ColorSpace::CMYK | ColorSpace::YCCK);
    let out_cs = if four { input } else { ColorSpace::RGB };
    let opts = DecoderOptions::default().jpeg_set_out_colorspace(out_cs).set_max_width(1 << 16).set_max_height(1 << 16);
    let mut d = JpegDecoder::new_with_options(ZCursor::new(data), opts);
    let px = d.decode().map_err(|e| ImportError::Decode("JPEG", format!("{e:?}")))?;
    let n = w as usize * h as usize;
    let mut rgba = Vec::with_capacity(n * 4);
    if four {
        if px.len() < n * 4 {
            return Err(ImportError::Corrupt("JPEG-Daten zu kurz".into()));
        }
        let app14 = has_app14_transform(data);
        for p in px.as_chunks::<4>().0.iter().take(n) {
            let (c, m, y, k) = if input == ColorSpace::YCCK {
                // YCbCr → (255−C, 255−M, 255−Y); K invertiert gespeichert
                let (yy, cb, cr) = (p[0] as f32, p[1] as f32 - 128.0, p[2] as f32 - 128.0);
                let r = (yy + 1.402 * cr).round().clamp(0.0, 255.0) as u8;
                let g = (yy - 0.344_136 * cb - 0.714_136 * cr).round().clamp(0.0, 255.0) as u8;
                let b = (yy + 1.772 * cb).round().clamp(0.0, 255.0) as u8;
                (255 - r, 255 - g, 255 - b, 255 - p[3])
            } else if app14 {
                (255 - p[0], 255 - p[1], 255 - p[2], 255 - p[3])
            } else {
                (p[0], p[1], p[2], p[3])
            };
            rgba.extend_from_slice(&cmyk_naive(c, m, y, k));
        }
    } else {
        if px.len() < n * 3 {
            return Err(ImportError::Corrupt("JPEG-Daten zu kurz".into()));
        }
        for p in px.as_chunks::<3>().0.iter().take(n) {
            rgba.extend_from_slice(&[p[0], p[1], p[2], 255]);
        }
    }
    let mut img = RasterImage::from_rgba(w, h, rgba)?;
    img.dpi = jfif_dpi(data).or_else(|| d.exif().and_then(|e| exif_dpi(e)));
    Ok(img)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exif_resolution() {
        // Minimaler TIFF-Kopf (II), eine IFD mit XResolution = 300/1 und ResolutionUnit = 2
        let mut t = b"II*\0\x08\0\0\0".to_vec();
        t.extend_from_slice(&2u16.to_le_bytes());
        t.extend_from_slice(&[0x1A, 0x01, 5, 0, 1, 0, 0, 0, 38, 0, 0, 0]);
        t.extend_from_slice(&[0x28, 0x01, 3, 0, 1, 0, 0, 0, 2, 0, 0, 0]);
        t.extend_from_slice(&0u32.to_le_bytes());
        t.extend_from_slice(&300u32.to_le_bytes());
        t.extend_from_slice(&1u32.to_le_bytes());
        assert_eq!(exif_dpi(&t), Some(300.0));
        assert!(decode(b"\xff\xd8\xff\xe0\0\x10JFIF").is_err());
    }
}
