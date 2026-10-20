//! JPEG 2000 for the PDF export (`/JPXDecode`, BA-101).
//!
//! Encoder: [`openjp2`] 0.6.1, the pure-Rust port of OpenJPEG (BSD-2-Clause, no C, no build
//! script). Its safe API only writes to files ([`openjp2::Stream::new_file`]); an in-memory stream
//! would need its `unsafe extern "C"` callback API, which this crate may not use
//! (`unsafe_code = "forbid"`). So the codestream goes through a private temporary file that is
//! read back and removed right away.
//!
//! The output is a JP2 file (signature, `ihdr`, `colr` box, codestream), the container the PDF
//! specification names for `/JPXDecode` from PDF 1.5 on; readers that also take raw codestreams
//! read it just as well.
//!
//! Only the encoder of `openjp2` is used: its decoder dereferences a null pointer when it is
//! dropped (`opj_tcd_free_tile`, caught by the debug UB checks). Tests decode with
//! `hayro-jpeg2000`, the decoder of the PDF import.
//!
//! # Quality mapping
//!
//! The PDF options only know the JPEG quality 1…100 ([`crate::vector_out::PdfOptions::jpeg_quality`];
//! the dialog's levels High…Low set 90/75/60/45/30). JPEG 2000 has no quality scale but a target
//! compression ratio, so [`jpx_rate`] maps:
//!
//! | quality | coding | ratio (uncompressed : compressed) |
//! |---|---|---|
//! | 100 | reversible 5/3 wavelet + RCT, **lossless** | – |
//! | 1…99 | irreversible 9/7 wavelet + ICT | `5 + (100 − q) / 2`, so 99 → 5.5:1, 90 (High) → 10:1, 75 → 17.5:1, 60 → 25:1, 45 → 32.5:1, 30 (Low) → 40:1, 1 → 54.5:1 |
//!
//! The ratios are chosen so that each level lands near the file size of the JPEG of the same
//! level on photographic content (JPEG q90 ≈ 2.4 bit/pixel ≈ 10:1 for 24-bit RGB). The ratio
//! counts against the uncompressed samples, so a grayscale image (one component) gets the same
//! bits per sample as an RGB one.

use crate::{ExportError, Result};
use openjp2::{
    Codec, OPJ_CLRSPC_GRAY, OPJ_CLRSPC_SRGB, OPJ_CODEC_JP2, Stream, opj_cparameters_t, opj_image,
    opj_image_comptparm,
};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

/// Target compression ratio for JPEG quality `quality` (clamped to 1…100); `None` = lossless
/// (quality 100). See the table in the module documentation.
pub fn jpx_rate(quality: i32) -> Option<f32> {
    let q = quality.clamp(1, 100);
    (q < 100).then(|| 5.0 + (100 - q) as f32 / 2.0)
}

fn jpx_error() -> ExportError {
    ExportError::Encode(fp_i18n::t("err-export-d-jpx"))
}

/// Unique path of a private temporary file (process id, counter, clock).
fn temp_path(ext: &str) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("fp-jpx-{}-{n}-{t}.{ext}", std::process::id()))
}

/// Removes the temporary file when dropped (also on errors).
struct TempFile(PathBuf);

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Number of wavelet resolutions: OpenJPEG's default 6, fewer for small images (every
/// resolution level halves the size; the smallest one must keep at least one pixel).
fn resolutions(w: u32, h: u32) -> i32 {
    (w.min(h).max(1).ilog2() as i32 + 1).clamp(1, 6)
}

/// Encodes 8-bit samples as a JP2 file. `samples` holds `w × h × comps` interleaved bytes; `comps`
/// is 1 (gray) or 3 (sRGB). `quality` 100 = lossless, otherwise see [`jpx_rate`].
pub fn encode_jp2(samples: &[u8], w: u32, h: u32, comps: usize, quality: i32) -> Result<Vec<u8>> {
    if w == 0 || h == 0 {
        return Err(ExportError::BadSize(w as usize, h as usize));
    }
    if !(comps == 1 || comps == 3) || samples.len() != w as usize * h as usize * comps {
        return Err(jpx_error());
    }
    let parm = opj_image_comptparm {
        dx: 1,
        dy: 1,
        w,
        h,
        x0: 0,
        y0: 0,
        prec: 8,
        bpp: 8,
        sgnd: 0,
    };
    let space = if comps == 3 {
        OPJ_CLRSPC_SRGB
    } else {
        OPJ_CLRSPC_GRAY
    };
    let mut image = opj_image::create(&vec![parm; comps], space).ok_or_else(jpx_error)?;
    image.x0 = 0;
    image.y0 = 0;
    image.x1 = w;
    image.y1 = h;
    {
        let planes = image.comps_mut().ok_or_else(jpx_error)?;
        for (c, plane) in planes.iter_mut().enumerate() {
            let data = plane.data_mut().ok_or_else(jpx_error)?;
            for (d, px) in data.iter_mut().zip(samples.chunks_exact(comps)) {
                *d = i32::from(px[c]);
            }
        }
    }
    let mut p = opj_cparameters_t::default();
    p.set_defaults();
    let rate = jpx_rate(quality);
    // one quality layer; rate 0 = all bits (lossless with the reversible transform)
    p.tcp_numlayers = 1;
    p.tcp_rates[0] = rate.unwrap_or(0.0);
    p.cp_disto_alloc = 1;
    p.irreversible = i32::from(rate.is_some());
    // colour transform (RCT/ICT) only across the three RGB components
    p.tcp_mct = if comps == 3 { 1 } else { 0 };
    p.numresolution = resolutions(w, h);

    let mut codec = Codec::new_encoder(OPJ_CODEC_JP2).ok_or_else(jpx_error)?;
    if codec.setup_encoder(&mut p, &mut image) == 0 {
        return Err(jpx_error());
    }
    let tmp = TempFile(temp_path("jp2"));
    {
        let mut s = Stream::new_file(&tmp.0, 1 << 20, false)?;
        if codec.start_compress(&mut image, &mut s) == 0
            || codec.encode(&mut s) == 0
            || codec.end_compress(&mut s) == 0
        {
            return Err(jpx_error());
        }
        s.flush()?;
    }
    let bytes = std::fs::read(&tmp.0)?;
    if bytes.is_empty() {
        return Err(jpx_error());
    }
    Ok(bytes)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Decodes with `hayro-jpeg2000`: `(width, height, components, interleaved samples)`.
    pub(crate) fn decode(jp2: &[u8]) -> (u32, u32, usize, Vec<u8>) {
        let im = hayro_jpeg2000::Image::new(jp2, &hayro_jpeg2000::DecodeSettings::default())
            .expect("JP2 parses");
        let (w, h) = (im.width(), im.height());
        let n = im.color_space().num_channels() as usize + usize::from(im.has_alpha());
        let mut ctx = hayro_jpeg2000::DecoderContext::default();
        let px = im.decode(&mut ctx).expect("JP2 decodes").data_u8();
        (w, h, n, px)
    }

    fn pattern(w: u32, h: u32, comps: usize) -> Vec<u8> {
        let mut v = Vec::new();
        for y in 0..h {
            for x in 0..w {
                for c in 0..comps as u32 {
                    v.push(((x * 37 + y * 11 + c * 90) ^ (x * y)) as u8);
                }
            }
        }
        v
    }

    #[test]
    fn rate_mapping() {
        assert_eq!(jpx_rate(100), None);
        assert_eq!(jpx_rate(150), None);
        assert_eq!(jpx_rate(90), Some(10.0));
        assert_eq!(jpx_rate(30), Some(40.0));
        assert_eq!(jpx_rate(1), Some(54.5));
        assert_eq!(jpx_rate(-5), Some(54.5));
    }

    #[test]
    fn lossless_round_trip_rgb_and_gray_all_sizes() {
        for &(w, h) in &[(1u32, 1u32), (2, 7), (17, 5), (64, 64), (130, 33)] {
            for comps in [1usize, 3] {
                let src = pattern(w, h, comps);
                let jp2 = encode_jp2(&src, w, h, comps, 100).unwrap();
                assert!(jp2.starts_with(openjp2::JP2_RFC3745_MAGIC), "JP2 signature");
                let (dw, dh, dn, px) = decode(&jp2);
                assert_eq!((dw, dh, dn), (w, h, comps));
                assert_eq!(px, src, "{w}x{h}x{comps} lossless");
            }
        }
    }

    #[test]
    fn lossy_is_smaller_and_close() {
        let (w, h) = (96u32, 80u32);
        // smooth gradient: lossy coding stays close
        let mut src = Vec::new();
        for y in 0..h {
            for x in 0..w {
                src.extend_from_slice(&[(x * 2) as u8, (y * 3) as u8, ((x + y) & 255) as u8]);
            }
        }
        let lossless = encode_jp2(&src, w, h, 3, 100).unwrap();
        let high = encode_jp2(&src, w, h, 3, 90).unwrap();
        let low = encode_jp2(&src, w, h, 3, 30).unwrap();
        assert!(high.len() < lossless.len() && low.len() < high.len());
        let (_, _, _, px) = decode(&high);
        let max = px
            .iter()
            .zip(&src)
            .map(|(a, b)| (*a as i32 - *b as i32).abs())
            .max()
            .unwrap();
        assert!(max <= 24, "max deviation {max}");
    }

    #[test]
    fn bad_input_is_an_error_not_a_panic() {
        assert!(encode_jp2(&[], 0, 0, 3, 90).is_err());
        assert!(encode_jp2(&[0; 5], 1, 1, 3, 90).is_err());
        assert!(encode_jp2(&[0; 4], 1, 1, 4, 90).is_err());
    }
}
