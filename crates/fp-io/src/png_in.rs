//! PNG ohne Original-Daten und APNG (12 §5.3): über das `png`-Crate auf RGBA8 normalisiert
//! (Palette, Graustufen, 16 Bit, `tRNS`), Auflösung aus `pHYs`; Gamma wird wie bei
//! `IgnorePNGGammaOnRead` nicht angewendet. APNG: Einzelbilder mit `dispose_op`/`blend_op`
//! zu vollen Leinwandbildern zusammengesetzt.

use std::io::Cursor;

use fp_geom::IRect;
use fp_raster::RgbaImage;

use crate::raster::{Animation, AnimationFrame, RasterImage};
use crate::{ImportError, Result, check_size};

fn to_rgba(buf: &[u8], info: &png::OutputInfo) -> Vec<u8> {
    let (w, h) = (info.width as usize, info.height as usize);
    let ch = info.color_type.samples();
    let mut out = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        let row = &buf[y * info.line_size..y * info.line_size + w * ch];
        for p in row.chunks_exact(ch) {
            match ch {
                1 => out.extend_from_slice(&[p[0], p[0], p[0], 255]),
                2 => out.extend_from_slice(&[p[0], p[0], p[0], p[1]]),
                3 => out.extend_from_slice(&[p[0], p[1], p[2], 255]),
                _ => out.extend_from_slice(&[p[0], p[1], p[2], p[3]]),
            }
        }
    }
    out
}

fn dpi_of(info: &png::Info) -> Option<f32> {
    let d = info.pixel_dims?;
    match d.unit {
        png::Unit::Meter if d.xppu > 0 => Some((d.xppu as f64 * 0.0254) as f32),
        _ => None,
    }
}

fn reader(data: &[u8]) -> Result<png::Reader<Cursor<&[u8]>>> {
    let mut dec = png::Decoder::new(Cursor::new(data));
    dec.set_transformations(png::Transformations::normalize_to_color8());
    dec.read_info()
        .map_err(|e| ImportError::Decode("PNG", e.to_string()))
}

/// PNG-Einzelbild lesen.
pub fn decode_png(data: &[u8]) -> Result<RasterImage> {
    let mut r = reader(data)?;
    let (w, h) = (r.info().width, r.info().height);
    check_size(w, h)?;
    let dpi = dpi_of(r.info());
    let palette = r.info().palette.as_ref().map(|p| {
        p.as_chunks::<3>()
            .0
            .iter()
            .map(|c| [c[0], c[1], c[2]])
            .collect()
    });
    let size = r.output_buffer_size().ok_or(ImportError::TooLarge(w, h))?;
    let mut buf = vec![0; size];
    let info = r
        .next_frame(&mut buf)
        .map_err(|e| ImportError::Decode("PNG", e.to_string()))?;
    let mut img = RasterImage::from_rgba(info.width, info.height, to_rgba(&buf, &info))?;
    img.dpi = dpi;
    img.palette = palette;
    Ok(img)
}

/// APNG lesen (Einzelbilder als volle Leinwandbilder, Löschmethode 1).
pub fn decode_apng(data: &[u8]) -> Result<Animation> {
    let mut r = reader(data)?;
    let (w, h) = (r.info().width, r.info().height);
    check_size(w, h)?;
    let Some(ac) = r.info().animation_control else {
        let img = decode_png(data)?;
        return Ok(Animation {
            width: w,
            height: h,
            has_transparency: img.has_transparency,
            frames: vec![AnimationFrame {
                image: img.image,
                x: 0,
                y: 0,
                delay_cs: 0,
                disposal: 0,
                replace: true,
            }],
            loop_count: 0,
            palette: None,
        });
    };
    let idat_is_frame = r.info().frame_control.is_some();
    let size = r.output_buffer_size().ok_or(ImportError::TooLarge(w, h))?;
    let mut buf = vec![0; size];
    let full = IRect::new(0, 0, w as i32, h as i32);
    let mut canvas = RgbaImage::new(full).map_err(|e| ImportError::Corrupt(e.to_string()))?;
    let mut frames = Vec::new();
    let mut has_t = false;
    let total = ac.num_frames as usize + usize::from(!idat_is_frame);
    for k in 0..total.min(10_000) {
        let info = match r.next_frame(&mut buf) {
            Ok(i) => i,
            Err(_) if k > 0 => break,
            Err(e) => return Err(ImportError::Decode("APNG", e.to_string())),
        };
        if k == 0 && !idat_is_frame {
            continue;
        }
        let fc = r.info().frame_control;
        let (x, y, dispose, blend, delay) = match fc {
            Some(f) => {
                let den = if f.delay_den == 0 {
                    100
                } else {
                    f.delay_den as u32
                };
                (
                    f.x_offset as i32,
                    f.y_offset as i32,
                    f.dispose_op,
                    f.blend_op,
                    (f.delay_num as u32 * 100).div_ceil(den.max(1)),
                )
            }
            None => (0, 0, png::DisposeOp::None, png::BlendOp::Source, 0),
        };
        let rgba = to_rgba(&buf, &info);
        has_t |= rgba.as_chunks::<4>().0.iter().any(|p| p[3] != 255);
        let r = IRect::new(x, y, x + info.width as i32, y + info.height as i32);
        let sub = RgbaImage::from_rgba_bytes(r, &rgba)
            .map_err(|e| ImportError::Corrupt(e.to_string()))?;
        let before = canvas.clone();
        match blend {
            png::BlendOp::Source => {
                canvas.fill_rect(r, fp_color::Rgba8::TRANSPARENT);
                canvas.paste(&sub);
            }
            png::BlendOp::Over => fp_raster::composite(
                &mut canvas,
                &sub,
                None,
                &fp_raster::CompositeOptions::scalar(),
            ),
        }
        frames.push(AnimationFrame {
            image: canvas.clone(),
            x: 0,
            y: 0,
            delay_cs: delay,
            disposal: 1,
            replace: true,
        });
        match dispose {
            png::DisposeOp::Background => canvas.fill_rect(r, fp_color::Rgba8::TRANSPARENT),
            png::DisposeOp::Previous => canvas = before,
            png::DisposeOp::None => {}
        }
    }
    if frames.is_empty() {
        return Err(ImportError::Corrupt(fp_i18n::t("err-import-apng-no-frame")));
    }
    let loop_count = if ac.num_plays == 0 {
        0
    } else if ac.num_plays == 1 {
        -1
    } else {
        ac.num_plays as i32 - 1
    };
    Ok(Animation {
        width: w,
        height: h,
        frames,
        loop_count,
        has_transparency: has_t,
        palette: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png_bytes(w: u32, h: u32, ct: png::ColorType, data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut e = png::Encoder::new(&mut out, w, h);
            e.set_color(ct);
            e.set_depth(png::BitDepth::Eight);
            let mut wr = e.write_header().unwrap();
            wr.write_image_data(data).unwrap();
        }
        out
    }

    #[test]
    fn gray_and_rgba() {
        let p = png_bytes(2, 1, png::ColorType::Grayscale, &[10, 200]);
        let r = decode_png(&p).unwrap();
        assert_eq!(
            r.image.pixel(1, 0),
            fp_color::Rgba8::new(200, 200, 200, 255)
        );
        assert!(!r.has_transparency);
        let p = png_bytes(1, 1, png::ColorType::Rgba, &[1, 2, 3, 4]);
        let r = decode_png(&p).unwrap();
        assert!(r.has_transparency);
        assert!(decode_png(b"\x89PNG\r\n\x1a\nxx").is_err());
    }
}
