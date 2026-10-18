//! Bild-Export als Ganzes: Optimieren + Kodieren für jedes Format, dazu die Vorschau des Ergebnisses
//! (wie das Register „Vorschau“ und der Dialog „Bildvorschau“, 08 §7).

use fp_raster::{RgbaImage, Sampler};

use crate::formats::{
    IcoImage, IcoOptions, RasterData, TiffCompression, write_bmp, write_bmp_ex, write_ico,
    write_qoi, write_tiff, write_wbmp,
};
use crate::jpeg::{JpegParams, encode_jpeg};
use crate::jpeg_tables::FwJpegTables;
use crate::options::{ExportFormat, OptimizeSettings};
use crate::pipeline::{ExportContext, Optimized, image_pixels, optimize_pixels};
use crate::png_out::{PngOptions, write_png_truecolor, write_png8};
use crate::{ExportError, Result};

/// Zusätzliche Eingaben der Kodierer.
#[derive(Clone, Debug)]
pub struct EncodeExtras {
    /// JPEG-Qualitätstabellen (Original-Anker oder IJG-Ersatz).
    pub jpeg_tables: FwJpegTables,
    /// Maske des selektiven JPEG (`w·h`, ≠ 0 = ausgewählt; JPEG-Maske plus Text-/Schaltflächenumrisse).
    pub jpeg_mask: Option<Vec<u8>>,
    /// Größen für ICO.
    pub ico: IcoOptions,
    /// TIFF-Kompression.
    pub tiff_compression: TiffCompression,
}

impl Default for EncodeExtras {
    fn default() -> Self {
        EncodeExtras {
            jpeg_tables: FwJpegTables::load_default(),
            jpeg_mask: None,
            ico: IcoOptions::default(),
            tiff_compression: TiffCompression::None,
        }
    }
}

impl EncodeExtras {
    /// Ohne Installation (IJG-Tabellen), für Tests und reproduzierbare Ergebnisse.
    pub fn standalone() -> EncodeExtras {
        EncodeExtras {
            jpeg_tables: FwJpegTables::ijg_fallback(),
            jpeg_mask: None,
            ico: IcoOptions::default(),
            tiff_compression: TiffCompression::None,
        }
    }
}

/// Ergebnis eines Bild-Exports.
#[derive(Clone, Debug, PartialEq)]
pub struct ExportedImage {
    /// Dateiinhalt.
    pub bytes: Vec<u8>,
    /// Optimiertes Bild vor der Kodierung.
    pub optimized: Optimized,
    /// Anzahl Farben (indiziert) bzw. 0.
    pub colors: usize,
}

/// Kodiert ein optimiertes Bild im Format der Einstellungen.
pub fn encode(
    opt: &Optimized,
    s: &OptimizeSettings,
    ctx: &ExportContext,
    ex: &EncodeExtras,
) -> Result<Vec<u8>> {
    let matte = s.effective_matte(ctx.canvas);
    let png_opts = PngOptions::from_context(ctx, s.interlaced);
    match (s.format, opt) {
        (ExportFormat::Gif | ExportFormat::AnimatedGif, Optimized::Indexed(ix)) => {
            let pal = ix.palette_rgb(matte);
            let fr = crate::gif::GifFrame {
                indices: ix.indices.clone(),
                w: ix.width.min(65535) as u16,
                h: ix.height.min(65535) as u16,
                delay: 0,
                disposal: 0,
                transparent: ix.transparent_index.map(|t| t.min(255) as u8),
                ..Default::default()
            };
            if ix.width > 65535 || ix.height > 65535 {
                return Err(ExportError::BadSize(ix.width, ix.height));
            }
            Ok(crate::gif::write_gif(
                fr.w,
                fr.h,
                &pal,
                &[fr],
                &crate::gif::GifOptions {
                    interlaced: s.interlaced,
                    ..Default::default()
                },
            ))
        }
        (ExportFormat::Png, Optimized::Indexed(ix)) => write_png8(ix, matte, &png_opts),
        (
            ExportFormat::Png | ExportFormat::Apng,
            Optimized::Truecolor {
                width,
                height,
                pixels,
                alpha,
            },
        ) => write_png_truecolor(pixels, *width, *height, *alpha, &png_opts),
        (
            ExportFormat::Jpeg,
            Optimized::Truecolor {
                width,
                height,
                pixels,
                ..
            },
        ) => {
            let sel = &s.jpeg.selective;
            let p = JpegParams {
                quality: s.jpeg.quality,
                smoothing: s.jpeg.smoothing,
                subsampling: s.jpeg.subsampling,
                progressive: s.jpeg.progressive,
                optimize: s.optimized,
                dpi: ctx.resolution_dpi,
                selective: if sel.enabled {
                    ex.jpeg_mask.clone().map(|m| (sel.quality, m))
                } else {
                    None
                },
                fixed_tables: None,
            };
            encode_jpeg(pixels, *width, *height, &p, &ex.jpeg_tables)
        }
        (ExportFormat::Tiff, Optimized::Indexed(ix)) => write_tiff(
            &RasterData::Indexed(ix),
            ctx.resolution_dpi,
            ex.tiff_compression,
            ctx.software.as_deref(),
        ),
        (
            ExportFormat::Tiff,
            Optimized::Truecolor {
                width,
                height,
                pixels,
                alpha,
            },
        ) => write_tiff(
            &RasterData::Truecolor {
                width: *width,
                height: *height,
                pixels,
                alpha: *alpha,
            },
            ctx.resolution_dpi,
            ex.tiff_compression,
            ctx.software.as_deref(),
        ),
        (ExportFormat::Bmp, Optimized::Indexed(ix)) => {
            write_bmp_ex(&RasterData::Indexed(ix), ctx.resolution_dpi, s.bmp_rle8)
        }
        (
            ExportFormat::Bmp,
            Optimized::Truecolor {
                width,
                height,
                pixels,
                alpha,
            },
        ) => write_bmp(
            &RasterData::Truecolor {
                width: *width,
                height: *height,
                pixels,
                alpha: *alpha,
            },
            ctx.resolution_dpi,
        ),
        (ExportFormat::Wbmp, Optimized::Indexed(ix)) => write_wbmp(ix),
        (
            ExportFormat::WebP | ExportFormat::WebPLossless | ExportFormat::AnimatedWebP,
            Optimized::Truecolor {
                width,
                height,
                pixels,
                ..
            },
        ) => {
            let lossless = s.format == ExportFormat::WebPLossless
                || (s.format == ExportFormat::AnimatedWebP && s.webp.lossless);
            crate::modern::write_webp_with(
                pixels,
                *width,
                *height,
                &crate::options::WebpSettings { lossless, ..s.webp },
            )
        }
        (
            ExportFormat::Avif,
            Optimized::Truecolor {
                width,
                height,
                pixels,
                ..
            },
        ) => crate::modern::write_avif_with(pixels, *width, *height, &s.avif),
        (
            ExportFormat::Qoi,
            Optimized::Truecolor {
                width,
                height,
                pixels,
                alpha,
            },
        ) => write_qoi(pixels, *width, *height, *alpha),
        (
            ExportFormat::Ico,
            Optimized::Truecolor {
                width,
                height,
                pixels,
                ..
            },
        ) => encode_ico(pixels, *width, *height, &ex.ico),
        (f, _) => Err(ExportError::Unsupported(f.display_name().to_string())),
    }
}

/// ICO aus einem Bild: je Zielgröße proportional eingepasst (bikubisch) und zentriert.
pub fn encode_ico(pixels: &[u32], w: usize, h: usize, o: &IcoOptions) -> Result<Vec<u8>> {
    let rect = fp_geom::IRect::new(0, 0, w as i32, h as i32);
    let src = RgbaImage::from_raw(
        rect,
        pixels.iter().map(|&c| crate::px::to_rgba(c)).collect(),
    )
    .map_err(|e| ExportError::Encode(e.to_string()))?;
    let mut imgs: Vec<(usize, Vec<u32>)> = Vec::new();
    let mut sizes = o.sizes.clone();
    sizes.retain(|&s| (1..=256).contains(&s));
    sizes.sort_unstable();
    sizes.dedup();
    if sizes.is_empty() {
        sizes.push(w.clamp(1, 256) as u32);
    }
    for s in sizes {
        let scale = s as f64 / w.max(h).max(1) as f64;
        let (nw, nh) = (
            ((w as f64 * scale).round() as u32).max(1),
            ((h as f64 * scale).round() as u32).max(1),
        );
        let scaled = fp_raster::resample::resize(&src, nw, nh, Sampler::Bicubic);
        let mut canvas = vec![0u32; (s * s) as usize];
        let (ox, oy) = ((s - nw.min(s)) / 2, (s - nh.min(s)) / 2);
        for y in 0..nh.min(s) {
            for x in 0..nw.min(s) {
                let p = scaled
                    .data()
                    .get((y * nw + x) as usize)
                    .copied()
                    .unwrap_or([0; 4]);
                canvas[((oy + y) * s + ox + x) as usize] = crate::px::from_rgba(p);
            }
        }
        imgs.push((s as usize, canvas));
    }
    let list: Vec<IcoImage> = imgs
        .iter()
        .map(|(s, p)| IcoImage {
            width: *s,
            height: *s,
            pixels: p,
        })
        .collect();
    write_ico(&list, o.png_from)
}

/// Optimiert und kodiert ein Bild.
pub fn export_image(
    img: &RgbaImage,
    s: &OptimizeSettings,
    ctx: &ExportContext,
    ex: &EncodeExtras,
) -> Result<ExportedImage> {
    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 {
        return Err(ExportError::BadSize(w, h));
    }
    let px = image_pixels(img);
    let optimized = optimize_pixels(&px, w, h, s, ctx);
    let bytes = encode(&optimized, s, ctx, ex)?;
    let colors = match &optimized {
        Optimized::Indexed(ix) => ix.palette.len(),
        _ => 0,
    };
    Ok(ExportedImage {
        bytes,
        optimized,
        colors,
    })
}

/// Vorschau des exportierten Bildes (Indexbild aufgelöst, JPEG dekodiert, sonst die Pixel vor der
/// Kodierung; AVIF/WebP verlustbehaftet ohne Dekoder: Pixel vor der Kodierung).
pub fn preview(result: &ExportedImage, s: &OptimizeSettings, origin: (i32, i32)) -> RgbaImage {
    match &result.optimized {
        Optimized::Indexed(ix) => ix.to_rgba(origin),
        Optimized::Truecolor {
            width,
            height,
            pixels,
            ..
        } => {
            let decoded = if s.format == ExportFormat::Jpeg {
                crate::preview::decode_jpeg(&result.bytes)
            } else {
                None
            };
            let (w, h, data) = match decoded {
                Some((dw, dh, d)) if dw == *width && dh == *height => (dw, dh, d),
                _ => (*width, *height, pixels.clone()),
            };
            let rect =
                fp_geom::IRect::new(origin.0, origin.1, origin.0 + w as i32, origin.1 + h as i32);
            RgbaImage::from_raw(rect, data.iter().map(|&c| crate::px::to_rgba(c)).collect())
                .unwrap_or_else(|_| RgbaImage::empty_at(origin.0, origin.1))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::{ColorMode, Transparency};

    fn img() -> RgbaImage {
        let r = fp_geom::IRect::new(0, 0, 16, 12);
        let data: Vec<[u8; 4]> = (0..16 * 12)
            .map(|i| {
                [
                    (i * 3 % 256) as u8,
                    (i * 7 % 256) as u8,
                    90,
                    if i % 5 == 0 { 0 } else { 255 },
                ]
            })
            .collect();
        RgbaImage::from_raw(r, data).unwrap()
    }

    #[test]
    fn all_formats_encode() {
        let ctx = ExportContext::default();
        let ex = EncodeExtras::standalone();
        for f in ExportFormat::ALL {
            if matches!(f, ExportFormat::Pict | ExportFormat::Avif) {
                continue;
            }
            for cm in [ColorMode::Indexed, ColorMode::Rgb24, ColorMode::Rgba32] {
                let mut s = OptimizeSettings {
                    format: f,
                    color_mode: cm,
                    ..OptimizeSettings::default()
                };
                s.transparency = Transparency::IndexAlpha;
                let r = export_image(&img(), &s, &ctx, &ex);
                assert!(r.is_ok(), "{f:?} {cm:?}: {:?}", r.err());
                let r = r.unwrap();
                assert!(!r.bytes.is_empty());
                let p = preview(&r, &s, (0, 0));
                assert_eq!((p.width(), p.height()), (16, 12));
            }
        }
        let s = OptimizeSettings {
            format: ExportFormat::Pict,
            ..OptimizeSettings::default()
        };
        assert!(export_image(&img(), &s, &ctx, &ex).is_err());
    }
}
