//! Moderne Ausgabeformate (ARCHITEKTUR §5): WebP verlustbehaftet/verlustfrei, animiertes WebP und AVIF.
//!
//! * WebP über `webp` → libwebp (C-Quelle, von `libwebp-sys` mit MSVC statisch gelinkt; keine DLL).
//! * AVIF über `ravif` → rav1e ohne Assembler (reines Rust); 8 Bit, Alpha mit eigener Qualität.
//!
//! APNG schreibt [`crate::png_out::write_apng`]; die Status-Abfolge liefert [`crate::animation`].

use crate::px::to_rgba;
use crate::{ExportError, Result};

fn rgba_bytes(pixels: &[u32]) -> Vec<u8> {
    pixels.iter().flat_map(|&c| to_rgba(c)).collect()
}

fn check(w: usize, h: usize, n: usize) -> Result<()> {
    if w == 0 || h == 0 || w > 16383 || h > 16383 || n < w * h {
        return Err(ExportError::BadSize(w, h));
    }
    Ok(())
}

/// WebP (verlustbehaftet mit `quality` 0…100 oder verlustfrei).
pub fn write_webp(
    pixels: &[u32],
    w: usize,
    h: usize,
    lossless: bool,
    quality: i32,
) -> Result<Vec<u8>> {
    check(w, h, pixels.len())?;
    let bytes = rgba_bytes(&pixels[..w * h]);
    let enc = webp::Encoder::from_rgba(&bytes, w as u32, h as u32);
    let mem = enc
        .encode_simple(lossless, quality.clamp(0, 100) as f32)
        .map_err(|e| ExportError::Encode(format!("WebP: {e:?}")))?;
    Ok(mem.to_vec())
}

/// Einzelbild eines animierten WebP (volle Leinwand, Pixel `0xAARRGGBB`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebpFrame {
    pub pixels: Vec<u32>,
    /// Anzeigedauer in 1/100 s.
    pub delay_cs: u16,
}

/// Animiertes WebP; `loop_count` 0 = endlos (wie GIF), `None` ⇒ einmal (WebP-Schleife 1).
pub fn write_animated_webp(
    w: usize,
    h: usize,
    frames: &[WebpFrame],
    lossless: bool,
    quality: i32,
    loop_count: Option<u16>,
) -> Result<Vec<u8>> {
    if frames.is_empty() {
        return Err(ExportError::Encode(fp_i18n::t(
            "err-export-d-webp-no-frame",
        )));
    }
    for f in frames {
        check(w, h, f.pixels.len())?;
    }
    let mut cfg = webp::WebPConfig::new()
        .map_err(|_| ExportError::Encode(fp_i18n::t("err-export-d-webp-config")))?;
    cfg.lossless = i32::from(lossless);
    cfg.quality = quality.clamp(0, 100) as f32;
    let data: Vec<Vec<u8>> = frames
        .iter()
        .map(|f| rgba_bytes(&f.pixels[..w * h]))
        .collect();
    let mut enc = webp::AnimEncoder::new(w as u32, h as u32, &cfg);
    enc.set_loop_count(match loop_count {
        Some(n) => n as i32,
        None => 1,
    });
    let mut t = 0i32;
    for (f, d) in frames.iter().zip(&data) {
        enc.add_frame(webp::AnimFrame::from_rgba(d, w as u32, h as u32, t));
        t = t.saturating_add(f.delay_cs.max(1) as i32 * 10);
    }
    let mem = enc.try_encode().map_err(|e| {
        ExportError::Encode(fp_i18n::tr!(
            "err-export-d-webp-anim",
            error = format!("{e:?}")
        ))
    })?;
    Ok(mem.to_vec())
}

/// AVIF mit Qualität 1…100 und Geschwindigkeit 1…10.
pub fn write_avif(pixels: &[u32], w: usize, h: usize, quality: i32, speed: u8) -> Result<Vec<u8>> {
    if w == 0 || h == 0 || pixels.len() < w * h || w > 65535 || h > 65535 {
        return Err(ExportError::BadSize(w, h));
    }
    let px: Vec<ravif::RGBA8> = pixels[..w * h]
        .iter()
        .map(|&c| {
            let p = to_rgba(c);
            ravif::RGBA8::new(p[0], p[1], p[2], p[3])
        })
        .collect();
    let q = quality.clamp(1, 100) as f32;
    let res = ravif::Encoder::new()
        .with_quality(q)
        .with_alpha_quality(q)
        .with_speed(speed.clamp(1, 10))
        .with_num_threads(Some(2))
        .encode_rgba(ravif::Img::new(&px[..], w, h))
        .map_err(|e| ExportError::Encode(format!("AVIF: {e}")))?;
    Ok(res.avif_file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn webp_variants() {
        let px: Vec<u32> = (0..64)
            .map(|i| 0xFF00_0000 | (i * 4) << 16 | 0x80)
            .collect();
        let l = write_webp(&px, 8, 8, true, 75).unwrap();
        assert_eq!(&l[..4], b"RIFF");
        assert_eq!(&l[8..12], b"WEBP");
        let y = write_webp(&px, 8, 8, false, 50).unwrap();
        assert_eq!(&y[8..12], b"WEBP");
        let f = vec![
            WebpFrame {
                pixels: px.clone(),
                delay_cs: 10,
            },
            WebpFrame {
                pixels: vec![0xFFFF_0000; 64],
                delay_cs: 20,
            },
        ];
        let a = write_animated_webp(8, 8, &f, true, 75, Some(0)).unwrap();
        assert!(a.windows(4).any(|w| w == b"ANIM"));
        assert!(write_webp(&[], 0, 0, true, 75).is_err());
    }
}
