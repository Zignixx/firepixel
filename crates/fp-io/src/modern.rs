//! Moderne Formate (ARCHITEKTUR §5): WebP (image-webp, auch animiert ⇒ Status), JPEG XL (jxl-oxide,
//! Animation ⇒ Status), QOI (eigener Dekoder), ICO/CUR (alle Größen, PNG- oder DIB-Einträge).
//!
//! **AVIF** liest [`crate::avif_in`], **HEIC/HEIF** [`crate::heic_in`].

use std::io::Cursor;
use std::sync::Arc;

use fp_geom::IRect;
use fp_raster::RgbaImage;

use crate::build::{bitmap_element, document_from_raster, ImportResult, ImportedContent};
use crate::detect::FileFormat;
use crate::raster::{Animation, AnimationFrame, RasterImage};
use crate::{check_size, ImportError, ImportOptions, Result};

/// Früher: Begründung für den fehlenden AVIF-Import (bleibt für ältere Aufrufer erhalten).
#[deprecated(note = "AVIF wird jetzt über avif_in gelesen")]
pub const AVIF_NOTE: &str = "AVIF wird über rav1d (reines Rust) gelesen.";
/// Früher: Begründung für den fehlenden HEIC-Import (bleibt für ältere Aufrufer erhalten).
#[deprecated(note = "HEIC wird jetzt über heic_in gelesen")]
pub const HEIF_NOTE: &str = "HEIC/HEIF wird über heic-decoder (reines Rust) gelesen.";

/// QOI lesen.
pub fn decode_qoi(d: &[u8]) -> Result<RasterImage> {
    if d.len() < 22 || &d[..4] != b"qoif" {
        return Err(ImportError::Corrupt("kein QOI".into()));
    }
    let w = u32::from_be_bytes([d[4], d[5], d[6], d[7]]);
    let h = u32::from_be_bytes([d[8], d[9], d[10], d[11]]);
    check_size(w, h)?;
    let n = w as usize * h as usize;
    let mut out = Vec::with_capacity(n * 4);
    let mut index = [[0u8; 4]; 64];
    let mut px = [0u8, 0, 0, 255];
    let mut p = 14usize;
    let end = d.len().saturating_sub(8);
    while out.len() < n * 4 {
        if p >= end {
            out.extend_from_slice(&px);
            continue;
        }
        let b = d[p];
        p += 1;
        let mut run = 1usize;
        match b {
            0xFE => {
                if let Some(s) = d.get(p..p + 3) {
                    px[..3].copy_from_slice(s);
                }
                p += 3;
            }
            0xFF => {
                if let Some(s) = d.get(p..p + 4) {
                    px.copy_from_slice(s);
                }
                p += 4;
            }
            _ => match b >> 6 {
                0 => px = index[(b & 63) as usize],
                1 => {
                    px[0] = px[0].wrapping_add((b >> 4) & 3).wrapping_sub(2);
                    px[1] = px[1].wrapping_add((b >> 2) & 3).wrapping_sub(2);
                    px[2] = px[2].wrapping_add(b & 3).wrapping_sub(2);
                }
                2 => {
                    let b2 = d.get(p).copied().unwrap_or(0);
                    p += 1;
                    let dg = (b & 63).wrapping_sub(32);
                    px[0] = px[0].wrapping_add(dg).wrapping_add(b2 >> 4).wrapping_sub(8);
                    px[1] = px[1].wrapping_add(dg);
                    px[2] = px[2].wrapping_add(dg).wrapping_add(b2 & 15).wrapping_sub(8);
                }
                _ => run = (b & 63) as usize + 1,
            },
        }
        let hash = (px[0] as usize * 3 + px[1] as usize * 5 + px[2] as usize * 7 + px[3] as usize * 11) % 64;
        index[hash] = px;
        for _ in 0..run {
            if out.len() < n * 4 {
                out.extend_from_slice(&px);
            }
        }
    }
    RasterImage::from_rgba(w, h, out)
}

/// WebP lesen (animiert ⇒ Animation mit vollen Einzelbildern).
pub fn decode_webp(d: &[u8]) -> Result<Animation> {
    let e = |x: image_webp::DecodingError| ImportError::Decode("WebP", x.to_string());
    let mut dec = image_webp::WebPDecoder::new(Cursor::new(d)).map_err(e)?;
    let (w, h) = dec.dimensions();
    check_size(w, h)?;
    let alpha = dec.has_alpha();
    let size = dec.output_buffer_size().ok_or(ImportError::TooLarge(w, h))?;
    let mut buf = vec![0u8; size];
    let to_rgba = |buf: &[u8]| -> Vec<u8> { if alpha { buf.to_vec() } else { buf.as_chunks::<3>().0.iter().flat_map(|c| [c[0], c[1], c[2], 255]).collect() } };
    let rect = IRect::new(0, 0, w as i32, h as i32);
    if !dec.is_animated() {
        dec.read_image(&mut buf).map_err(e)?;
        let px = to_rgba(&buf);
        let has_t = px.as_chunks::<4>().0.iter().any(|p| p[3] != 255);
        let image = RgbaImage::from_rgba_bytes(rect, &px).map_err(|x| ImportError::Corrupt(x.to_string()))?;
        return Ok(Animation { width: w, height: h, frames: vec![AnimationFrame { image, x: 0, y: 0, delay_cs: 0, disposal: 0, replace: true }], loop_count: 0, has_transparency: has_t, palette: None });
    }
    let loop_count = match dec.loop_count() {
        image_webp::LoopCount::Forever => 0,
        image_webp::LoopCount::Times(n) => n.get() as i32 - 1,
    };
    let mut frames = Vec::new();
    let mut has_t = false;
    for _ in 0..dec.num_frames().min(10_000) {
        let ms = match dec.read_frame(&mut buf) {
            Ok(ms) => ms,
            Err(image_webp::DecodingError::NoMoreFrames) => break,
            Err(x) => return Err(e(x)),
        };
        let px = to_rgba(&buf);
        has_t |= px.as_chunks::<4>().0.iter().any(|p| p[3] != 255);
        let image = RgbaImage::from_rgba_bytes(rect, &px).map_err(|x| ImportError::Corrupt(x.to_string()))?;
        frames.push(AnimationFrame { image, x: 0, y: 0, delay_cs: ms.div_ceil(10), disposal: 1, replace: true });
    }
    if frames.is_empty() {
        return Err(ImportError::Corrupt("WebP ohne Einzelbild".into()));
    }
    Ok(Animation { width: w, height: h, frames, loop_count, has_transparency: has_t, palette: None })
}

/// JPEG XL lesen (sRGB, 8 Bit; Animation ⇒ Einzelbilder).
pub fn decode_jxl(d: &[u8]) -> Result<Animation> {
    let e = |x: Box<dyn std::error::Error + Send + Sync>| ImportError::Decode("JPEG XL", x.to_string());
    let mut img = jxl_oxide::JxlImage::builder().read(Cursor::new(d)).map_err(e)?;
    let (w, h) = (img.width(), img.height());
    check_size(w, h)?;
    img.request_color_encoding(jxl_oxide::EnumColourEncoding::srgb(jxl_oxide::RenderingIntent::Relative));
    let anim = img.image_header().metadata.animation.as_ref().map(|a| (a.tps_numerator, a.tps_denominator, a.num_loops));
    let n = img.num_loaded_keyframes().max(1);
    let mut frames = Vec::with_capacity(n);
    let mut has_t = false;
    for k in 0..n.min(10_000) {
        let r = img.render_frame(k).map_err(e)?;
        let mut stream = r.stream();
        let (fw, fh, ch) = (stream.width(), stream.height(), stream.channels() as usize);
        let mut buf = vec![0u8; fw as usize * fh as usize * ch];
        stream.write_to_buffer::<u8>(&mut buf);
        let px: Vec<u8> = match ch {
            1 => buf.iter().flat_map(|&v| [v, v, v, 255]).collect(),
            2 => buf.as_chunks::<2>().0.iter().flat_map(|c| [c[0], c[0], c[0], c[1]]).collect(),
            3 => buf.as_chunks::<3>().0.iter().flat_map(|c| [c[0], c[1], c[2], 255]).collect(),
            _ => buf.chunks_exact(ch).flat_map(|c| [c[0], c[1], c[2], c[3]]).collect(),
        };
        has_t |= px.as_chunks::<4>().0.iter().any(|p| p[3] != 255);
        let image = RgbaImage::from_rgba_bytes(IRect::new(0, 0, fw as i32, fh as i32), &px).map_err(|x| ImportError::Corrupt(x.to_string()))?;
        let delay_cs = match &anim {
            Some((num, den, _)) if *num > 0 => (r.duration() as u64 * 100 * *den as u64 / *num as u64) as u32,
            _ => 0,
        };
        frames.push(AnimationFrame { image, x: 0, y: 0, delay_cs, disposal: 1, replace: true });
    }
    let loop_count = anim.map(|(_, _, n)| if n == 0 { 0 } else { n as i32 - 1 }).unwrap_or(0);
    Ok(Animation { width: w, height: h, frames, loop_count, has_transparency: has_t, palette: None })
}

/// Alle Bilder einer ICO/CUR-Datei (Größe absteigend).
pub fn decode_ico(d: &[u8]) -> Result<Vec<RasterImage>> {
    let u16le = |o: usize| d.get(o..o + 2).map(|b| u16::from_le_bytes([b[0], b[1]]));
    let u32le = |o: usize| d.get(o..o + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
    let n = u16le(4).ok_or_else(|| ImportError::Corrupt("ICO-Kopf".into()))? as usize;
    let mut out = Vec::new();
    for k in 0..n.min(256) {
        let e = 6 + k * 16;
        let (Some(size), Some(off)) = (u32le(e + 8), u32le(e + 12)) else { break };
        let Some(blob) = d.get(off as usize..off as usize + size as usize) else { continue };
        let img = if blob.starts_with(b"\x89PNG\r\n\x1a\n") {
            crate::png_in::decode_png(blob)?
        } else {
            decode_ico_dib(blob)?
        };
        out.push(img);
    }
    if out.is_empty() {
        return Err(ImportError::Corrupt("ICO ohne Bild".into()));
    }
    out.sort_by_key(|i| std::cmp::Reverse(i.width() * i.height()));
    Ok(out)
}

/// DIB-Eintrag eines Symbols (doppelte Höhe = Bild + AND-Maske).
fn decode_ico_dib(blob: &[u8]) -> Result<RasterImage> {
    let hsize = blob.get(..4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])).unwrap_or(0) as usize;
    if hsize < 40 || blob.len() < hsize {
        return Err(ImportError::Corrupt("ICO-DIB".into()));
    }
    let w = i32::from_le_bytes([blob[4], blob[5], blob[6], blob[7]]);
    let h2 = i32::from_le_bytes([blob[8], blob[9], blob[10], blob[11]]);
    let bpp = u16::from_le_bytes([blob[14], blob[15]]);
    let h = h2 / 2;
    let mut fixed = blob.to_vec();
    fixed[8..12].copy_from_slice(&h.to_le_bytes());
    let mut img = crate::bmp::decode_dib(&fixed, None, true)?;
    // AND-Maske hinter den Farbdaten (bei < 32 Bit bzw. 32 Bit ohne Alphainformation)
    let (wu, hu) = (w.unsigned_abs() as usize, h.unsigned_abs() as usize);
    let ncol = if bpp <= 8 { 1usize << bpp } else { 0 };
    let xor = (wu * bpp as usize).div_ceil(32) * 4 * hu;
    let mo = hsize + ncol * 4 + xor;
    let mstride = wu.div_ceil(32) * 4;
    let no_alpha = bpp < 32 || img.image.data().iter().all(|p| p[3] == 255);
    if no_alpha && blob.len() >= mo + mstride * hu {
        for y in 0..hu {
            let Some(row) = blob.get(mo + y * mstride..mo + (y + 1) * mstride) else { break };
            let ry = (hu - 1 - y) as i32;
            for x in 0..wu {
                if row[x / 8] & (0x80 >> (x % 8)) != 0 {
                    let mut p = img.image.pixel(x as i32, ry);
                    p.a = 0;
                    img.image.set_pixel(x as i32, ry, p);
                }
            }
        }
    }
    img.has_transparency = img.image.data().iter().any(|p| p[3] != 255);
    Ok(img)
}

/// ICO importieren: eine Seite je Größe (Seitenname „B×H“) bzw. nur die größte.
pub fn import_ico(d: &[u8], opts: &ImportOptions) -> Result<ImportResult> {
    let imgs = decode_ico(d)?;
    let mut it = imgs.into_iter();
    let first = it.next().ok_or_else(|| ImportError::Corrupt("ICO ohne Bild".into()))?;
    let name = format!("{}×{}", first.width(), first.height());
    let mut doc = document_from_raster(first, FileFormat::Ico);
    if let Some(p) = doc.page_mut(0) {
        p.name = Some(name);
    }
    if opts.ico_all_sizes {
        for img in it {
            let i = doc.add_page(&format!("{}×{}", img.width(), img.height()));
            let mut ids = std::mem::take(&mut doc.ids);
            let mut tile = 0;
            let e = bitmap_element(&mut ids, &img.image, None, &mut tile);
            doc.ids = ids;
            if let Some(p) = doc.page_mut(i) {
                p.canvas.width = img.width() as f32;
                p.canvas.height = img.height() as f32;
                p.root.background = fp_color::Argb(0x00FF_FFFF);
                if let Some(fp_model::LayerSlot::Own(l)) = p.root.layers.get_mut(0) {
                    let l = Arc::make_mut(l);
                    l.name = "Hintergrund".into();
                    if let Some(c) = l.cels.first_mut() {
                        Arc::make_mut(c).elements.push(Arc::new(e));
                    }
                }
            }
        }
    }
    Ok(ImportResult { document: doc, format: FileFormat::Ico, content: ImportedContent::Bitmap, warnings: vec![] })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qoi_small() {
        // 2×1: QOI_OP_RGB rot, dann Lauf 1
        let mut q = b"qoif".to_vec();
        q.extend_from_slice(&2u32.to_be_bytes());
        q.extend_from_slice(&1u32.to_be_bytes());
        q.extend_from_slice(&[4, 0, 0xFE, 255, 0, 0, 0xC0, 0, 0, 0, 0, 0, 0, 0, 1]);
        let r = decode_qoi(&q).unwrap();
        assert_eq!(r.image.pixel(1, 0), fp_color::Rgba8::new(255, 0, 0, 255));
        assert!(decode_qoi(b"qoif").is_err());
    }
}
